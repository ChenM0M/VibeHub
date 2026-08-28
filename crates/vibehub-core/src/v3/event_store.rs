use super::domain::{AppendResult, EventDraft, V3Error, V3ErrorCategory, V3EventEnvelope};
use super::indexed_store::{
    IndexedProjectStore, IndexedStoreStatus, STORE_FORMAT, STORE_MODEL_VERSION,
};
use super::lifecycle::TaskLifecycleProjection;
use super::orchestration::OrchestrationProjection;
use super::project_memory::MemoryProjection;
use super::projection::V3Projection;
use chrono::{SecondsFormat, Utc};
use serde_json::{json, Value};
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime};
use uuid::Uuid;

// A retryable safety valve: under heavy multi-writer contention on slow
// filesystems (notably throttled CI runners), waiting out the handover is
// preferable to failing an append of an event that is the workflow's
// source of truth.
const LOCK_TIMEOUT: Duration = Duration::from_secs(10);
const STALE_LOCK_AGE: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct V3EventStore {
    project_root: PathBuf,
    projection_runtime: Arc<Mutex<ProjectionRuntimeState>>,
}

#[derive(Debug, Clone)]
enum ProjectionRuntimeState {
    Idle,
    Rebuilding,
    Failed(V3Error),
}

impl V3EventStore {
    pub fn open(project_root: impl AsRef<Path>) -> Result<Self, V3Error> {
        let project_root = project_root
            .as_ref()
            .canonicalize()
            .map_err(io_error("V3_ROOT_NOT_FOUND"))?;
        if !project_root.join(".vibehub").is_dir() {
            return Err(V3Error::new(
                "V3_NOT_INITIALIZED",
                V3ErrorCategory::NotFound,
                false,
                "project does not contain .vibehub",
            ));
        }
        Ok(Self {
            project_root,
            projection_runtime: Arc::new(Mutex::new(ProjectionRuntimeState::Idle)),
        })
    }

    pub fn append(&self, draft: EventDraft) -> Result<AppendResult, V3Error> {
        self.append_inner(draft, false)
    }

    /// Append an event and rebuild the projection while the event-store lock
    /// is synchronized. Store format 2 retains the method name for API
    /// compatibility, but updates the SQLite WAL index and affected shards
    /// after releasing the short JSONL sequencing lock; it never rewrites the
    /// monolithic projection.json during steady-state appends.
    pub fn append_with_rebuild(&self, draft: EventDraft) -> Result<AppendResult, V3Error> {
        self.set_projection_runtime(ProjectionRuntimeState::Rebuilding);
        let result = self.append_inner(draft, true);
        match &result {
            Err(error)
                if matches!(
                    error.code.as_str(),
                    "V3_PROJECTION_REBUILD_FAILED" | "V3_PROJECTION_UPDATE_FAILED"
                ) =>
            {
                self.set_projection_runtime(ProjectionRuntimeState::Failed(error.clone()));
            }
            _ => self.set_projection_runtime(ProjectionRuntimeState::Idle),
        }
        result
    }

    fn append_inner(&self, draft: EventDraft, _rebuild: bool) -> Result<AppendResult, V3Error> {
        validate_draft(&draft)?;
        let paths = self.paths(&draft.project_id.0);
        let project_id = draft.project_id.0.clone();
        self.ensure_store_root(&paths, &project_id)?;
        let index = self.open_index(&paths, &project_id)?;
        index.sync_from_jsonl(&paths.events, &project_id)?;
        let sync_started = Instant::now();
        let result = loop {
            let _lease = LockLease::acquire(&paths.lock)?;
            recover_partial_tail(&paths.events, &paths.quarantine)?;
            let status = index.status(&paths.events)?;
            if status.state != "synced" {
                drop(_lease);
                if sync_started.elapsed() >= LOCK_TIMEOUT {
                    return Err(index_sync_timeout(&project_id, &status));
                }
                index.sync_from_jsonl(&paths.events, &project_id)?;
                continue;
            }
            if let Some(event) = index.event_by_idempotency_key(&draft.idempotency_key)? {
                break duplicate_result(event, &draft)?;
            } else {
                let current_version = index.aggregate_version(&draft.aggregate_id)?;
                if current_version != draft.expected_version {
                    return Err(V3Error::new(
                        "V3_VERSION_CONFLICT",
                        V3ErrorCategory::VersionConflict,
                        true,
                        "expected aggregate version does not match current version",
                    )
                    .with_detail("expected_version", draft.expected_version)
                    .with_detail("current_version", current_version));
                }
                let event = event_from_draft(draft, current_version);
                append_line(&paths.events, &event)?;
                break AppendResult::Appended { event };
            }
        };
        if let AppendResult::Appended { event } = &result {
            index
                .sync_from_jsonl(&paths.events, &project_id)
                .map_err(|cause| projection_update_error(&project_id, &paths.index, cause))?;
            if index
                .event_by_idempotency_key(&event.idempotency_key)?
                .is_none()
            {
                return Err(projection_update_error(
                    &project_id,
                    &paths.index,
                    V3Error::new(
                        "V3_INDEX_BEHIND_SOURCE",
                        V3ErrorCategory::StaleResource,
                        true,
                        "the appended event is durable but not yet indexed",
                    ),
                ));
            }
        }
        Ok(result)
    }

    /// Rebuild the projection.json from the event log under the same lock
    /// used by append. The returned projection is the exact folded input that
    /// was written.
    pub fn rebuild_projection(&self, project_id: &str) -> Result<V3Projection, V3Error> {
        self.set_projection_runtime(ProjectionRuntimeState::Rebuilding);
        let result = self.rebuild_projection_inner(project_id).map_err(|error| {
            if error.code == "V3_PROJECTION_REBUILD_FAILED" {
                error
            } else {
                projection_rebuild_error(project_id, &self.projection_path(project_id), 0, error)
            }
        });
        match &result {
            Ok(_) => self.set_projection_runtime(ProjectionRuntimeState::Idle),
            Err(error) => {
                self.set_projection_runtime(ProjectionRuntimeState::Failed(error.clone()))
            }
        }
        result
    }

    fn rebuild_projection_inner(&self, project_id: &str) -> Result<V3Projection, V3Error> {
        let paths = self.paths(project_id);
        self.ensure_store_root(&paths, project_id)?;
        let index = self.open_index(&paths, project_id)?;
        let _lease = LockLease::acquire(&paths.lock)?;
        recover_partial_tail(&paths.events, &paths.quarantine)?;
        index.reset_and_sync(&paths.events, project_id)?;
        let projection = index.full_projection(project_id)?;
        super::projection::write_atomic(&paths.projection, &projection).map_err(|error| {
            projection_rebuild_error(
                project_id,
                &paths.projection,
                projection.total_event_count as usize,
                error,
            )
        })?;
        Ok(projection)
    }

    /// Check whether the projection is stale relative to the event log.
    ///
    /// Returns `Ok(true)` if the projection is missing or its event count
    /// doesn't match the event log length. Uses `total_event_count` when
    /// available (new projections), falls back to `source_event_ids.len()`
    /// for older projections that predate the field. Returns `Ok(false)` if
    /// the projection is current.
    pub fn projection_is_stale(&self, project_id: &str) -> Result<bool, V3Error> {
        match self.projection_runtime_state() {
            ProjectionRuntimeState::Rebuilding => {
                return Err(projection_rebuild_in_progress(project_id))
            }
            ProjectionRuntimeState::Failed(error) => return Err(error),
            ProjectionRuntimeState::Idle => {}
        }
        let (_, status) = self.synced_index(project_id)?;
        Ok(status.state != "synced")
    }

    /// Return projection staleness info as a JSON value.
    ///
    /// Useful for MCP tools and diagnostics. Returns an object with:
    /// - `stale`: bool
    /// - `event_count`: number of events in the log
    /// - `projection_event_count`: number of events in the projection
    ///   (null if projection doesn't exist or can't be parsed)
    /// - `last_event_timestamp`: timestamp of the last event in the projection
    /// - `projection_path`: path to the projection file
    pub fn projection_status(&self, project_id: &str) -> Result<Value, V3Error> {
        if matches!(
            self.projection_runtime_state(),
            ProjectionRuntimeState::Rebuilding
        ) {
            return Ok(json!({
                "state": "rebuilding",
                "rebuild_state": "rebuilding",
                "stale": true,
                "event_count": Value::Null,
                "projection_event_count": Value::Null,
                "last_event_timestamp": Value::Null,
                "projection_metadata_consistent": Value::Null,
                "projection_path": self.projection_path(project_id).to_string_lossy(),
                "repair_action": "wait for the active projection rebuild to finish, then retry"
            }));
        }
        let (_, status) = self.synced_index(project_id)?;
        Ok(indexed_projection_status_value(
            project_id,
            &self.projection_path(project_id),
            &status,
        ))
    }

    /// Return projection freshness after the caller has already loaded and
    /// validated the event stream. View assembly uses this to avoid parsing
    /// the same Project log a second time in one read request.
    pub fn projection_status_for_events(
        &self,
        project_id: &str,
        events: &[V3EventEnvelope],
    ) -> Result<Value, V3Error> {
        self.projection_status_for_event_count(project_id, events.len())
    }

    /// Load one projection snapshot and derive its freshness status in the
    /// same parse. This is the bounded-read bridge for format 1 projects;
    /// store format 2 replaces the file count with its indexed watermark.
    pub fn projection_snapshot_for_events(
        &self,
        project_id: &str,
        events: &[V3EventEnvelope],
    ) -> Result<(Option<V3Projection>, Value), V3Error> {
        self.projection_snapshot_for_event_count(project_id, events.len())
    }

    fn projection_status_for_event_count(
        &self,
        project_id: &str,
        event_count: usize,
    ) -> Result<Value, V3Error> {
        self.projection_snapshot_for_event_count(project_id, event_count)
            .map(|(_, status)| status)
    }

    fn projection_snapshot_for_event_count(
        &self,
        project_id: &str,
        event_count: usize,
    ) -> Result<(Option<V3Projection>, Value), V3Error> {
        let proj_path = self.projection_path(project_id);
        if matches!(
            self.projection_runtime_state(),
            ProjectionRuntimeState::Rebuilding
        ) {
            return Ok((
                None,
                json!({
                    "state": "rebuilding",
                    "rebuild_state": "rebuilding",
                    "stale": true,
                    "event_count": event_count,
                    "projection_event_count": Value::Null,
                    "last_event_timestamp": Value::Null,
                    "projection_metadata_consistent": Value::Null,
                    "projection_path": proj_path.to_string_lossy(),
                    "repair_action": "wait for the active projection rebuild to finish, then retry"
                }),
            ));
        }
        match self.synced_index(project_id) {
            Ok((index, status)) => {
                let projection = index.full_projection(project_id)?;
                self.set_projection_runtime(ProjectionRuntimeState::Idle);
                Ok((
                    Some(projection),
                    indexed_projection_status_value(project_id, &proj_path, &status),
                ))
            }
            Err(error) => Ok((
                None,
                projection_status_value(
                    project_id,
                    &proj_path,
                    "rebuild_failed",
                    true,
                    event_count,
                    None,
                    None,
                    false,
                    Some(error),
                ),
            )),
        }
    }

    /// Load the atomically written compatibility projection without replaying
    /// the event log. Callers must first verify freshness against a validated
    /// event count (or, in store format 2, the indexed source watermark).
    pub fn load_projection(&self, project_id: &str) -> Result<V3Projection, V3Error> {
        let (index, _) = self.synced_index(project_id)?;
        index.full_projection(project_id)
    }

    fn set_projection_runtime(&self, state: ProjectionRuntimeState) {
        if let Ok(mut current) = self.projection_runtime.lock() {
            *current = state;
        }
    }

    fn projection_runtime_state(&self) -> ProjectionRuntimeState {
        self.projection_runtime
            .lock()
            .map(|state| state.clone())
            .unwrap_or(ProjectionRuntimeState::Idle)
    }

    pub fn load_project(&self, project_id: &str) -> Result<Vec<V3EventEnvelope>, V3Error> {
        let (index, _) = self.synced_index(project_id)?;
        index.all_events()
    }

    pub fn load_task_events(
        &self,
        project_id: &str,
        task_id: &str,
    ) -> Result<Vec<V3EventEnvelope>, V3Error> {
        let (index, _) = self.synced_index(project_id)?;
        index.task_events(task_id)
    }

    pub fn load_aggregate_events(
        &self,
        project_id: &str,
        aggregate_id: &str,
    ) -> Result<Vec<V3EventEnvelope>, V3Error> {
        let (index, _) = self.synced_index(project_id)?;
        index.aggregate_events(aggregate_id)
    }

    pub fn aggregate_version(&self, project_id: &str, aggregate_id: &str) -> Result<u64, V3Error> {
        let (index, _) = self.synced_index(project_id)?;
        index.aggregate_version(aggregate_id)
    }

    pub fn event_by_idempotency_key(
        &self,
        project_id: &str,
        idempotency_key: &str,
    ) -> Result<Option<V3EventEnvelope>, V3Error> {
        let (index, _) = self.synced_index(project_id)?;
        index.event_by_idempotency_key(idempotency_key)
    }

    pub fn task_projection(
        &self,
        project_id: &str,
        task_id: &str,
    ) -> Result<TaskLifecycleProjection, V3Error> {
        let (index, _) = self.synced_index(project_id)?;
        Ok(index
            .task_projection(task_id)?
            .unwrap_or_else(|| TaskLifecycleProjection::empty(task_id)))
    }

    pub fn project_memory_projection(&self, project_id: &str) -> Result<MemoryProjection, V3Error> {
        let (index, _) = self.synced_index(project_id)?;
        Ok(index
            .project_memory(project_id)?
            .unwrap_or_else(|| super::project_memory::fold(project_id, &[])))
    }

    pub fn orchestration_projection(
        &self,
        project_id: &str,
        task_id: &str,
    ) -> Result<OrchestrationProjection, V3Error> {
        let (index, _) = self.synced_index(project_id)?;
        index.orchestration_projection(task_id)
    }

    pub fn indexed_projection(&self, project_id: &str) -> Result<V3Projection, V3Error> {
        let (index, _) = self.synced_index(project_id)?;
        index.full_projection(project_id)
    }

    fn synced_index(
        &self,
        project_id: &str,
    ) -> Result<(IndexedProjectStore, IndexedStoreStatus), V3Error> {
        let paths = self.paths(project_id);
        self.ensure_store_root(&paths, project_id)?;
        let index = self.open_index(&paths, project_id)?;
        index.sync_from_jsonl(&paths.events, project_id)?;
        let sync_started = Instant::now();
        loop {
            let lease = LockLease::acquire(&paths.lock)?;
            recover_partial_tail(&paths.events, &paths.quarantine)?;
            let status = index.status(&paths.events)?;
            if status.state == "synced" {
                return Ok((index, status));
            }
            drop(lease);
            if sync_started.elapsed() >= LOCK_TIMEOUT {
                return Err(index_sync_timeout(project_id, &status));
            }
            index.sync_from_jsonl(&paths.events, project_id)?;
        }
    }

    pub fn projection_path(&self, project_id: &str) -> PathBuf {
        self.paths(project_id).projection
    }

    pub(crate) fn project_root(&self) -> &Path {
        &self.project_root
    }

    fn paths(&self, project_id: &str) -> StorePaths {
        let safe_id = project_id.replace(['/', '\\'], "_");
        let root = self.project_root.join(".vibehub/v3/projects").join(safe_id);
        StorePaths {
            events: root.join("events.jsonl"),
            lock: root.join("events.lock"),
            quarantine: root.join("quarantine"),
            projection: root.join("projection.json"),
            index: root.join("store-v2.sqlite3"),
            migration_lock: root.join("store-v2.migration.lock"),
            migration_marker: root.join("store-v2-migration.json"),
            backups: root.join("backups"),
            root,
        }
    }

    fn ensure_store_root(&self, paths: &StorePaths, project_id: &str) -> Result<(), V3Error> {
        validate_project_storage_identity(project_id)?;
        let vibehub = self.project_root.join(".vibehub");
        let v3 = vibehub.join("v3");
        let projects = v3.join("projects");
        for path in [&vibehub, &v3, &projects, &paths.root] {
            match fs::symlink_metadata(path) {
                Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                    return Err(V3Error::new(
                        "V3_STORE_PATH_UNSAFE",
                        V3ErrorCategory::PermissionDenied,
                        false,
                        "V3 event-store directories must not be symlinks or non-directories",
                    )
                    .with_detail("path", path.to_string_lossy().to_string()));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(io_error("V3_STORE_PATH_METADATA_FAILED")(error)),
            }
        }
        fs::create_dir_all(&paths.root).map_err(io_error("V3_STORE_CREATE_FAILED"))?;
        let metadata =
            fs::symlink_metadata(&paths.root).map_err(io_error("V3_STORE_PATH_METADATA_FAILED"))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(V3Error::new(
                "V3_STORE_PATH_UNSAFE",
                V3ErrorCategory::PermissionDenied,
                false,
                "V3 project event-store root must be a real directory",
            ));
        }
        Ok(())
    }

    fn open_index(
        &self,
        paths: &StorePaths,
        project_id: &str,
    ) -> Result<IndexedProjectStore, V3Error> {
        if paths.index.exists() {
            match IndexedProjectStore::open(&paths.index) {
                Ok(index) => return Ok(index),
                Err(error) if recoverable_index_open_error(&error) => {}
                Err(error) => return Err(error),
            }
        }
        let _migration = LockLease::acquire(&paths.migration_lock)?;
        if paths.index.exists() {
            match IndexedProjectStore::open(&paths.index) {
                Ok(index) => return Ok(index),
                Err(error) if recoverable_index_open_error(&error) => {
                    let _events = LockLease::acquire(&paths.lock)?;
                    recover_partial_tail(&paths.events, &paths.quarantine)?;
                    IndexedProjectStore::quarantine_corrupt(&paths.index)?;
                    return IndexedProjectStore::migrate_from_jsonl(
                        &paths.index,
                        &paths.events,
                        &paths.projection,
                        &paths.backups,
                        &paths.migration_marker,
                        project_id,
                        None,
                    );
                }
                Err(error) => return Err(error),
            }
        }
        let _events = LockLease::acquire(&paths.lock)?;
        recover_partial_tail(&paths.events, &paths.quarantine)?;
        IndexedProjectStore::migrate_from_jsonl(
            &paths.index,
            &paths.events,
            &paths.projection,
            &paths.backups,
            &paths.migration_marker,
            project_id,
            None,
        )
    }
}

fn recoverable_index_open_error(error: &V3Error) -> bool {
    matches!(
        error.code.as_str(),
        "V3_INDEX_CONFIG_FAILED"
            | "V3_INDEX_SCHEMA_FAILED"
            | "V3_INDEX_SCHEMA_INCOMPLETE"
            | "V3_INDEX_MODEL_OUTDATED"
    )
}

struct StorePaths {
    root: PathBuf,
    events: PathBuf,
    lock: PathBuf,
    quarantine: PathBuf,
    projection: PathBuf,
    index: PathBuf,
    migration_lock: PathBuf,
    migration_marker: PathBuf,
    backups: PathBuf,
}

struct LockLease {
    path: PathBuf,
}

impl LockLease {
    fn acquire(path: &Path) -> Result<Self, V3Error> {
        let started = Instant::now();
        loop {
            match OpenOptions::new().write(true).create_new(true).open(path) {
                Ok(mut file) => {
                    writeln!(
                        file,
                        "pid={} acquired_at={}",
                        std::process::id(),
                        Utc::now().to_rfc3339()
                    )
                    .map_err(io_error("V3_LOCK_WRITE_FAILED"))?;
                    file.sync_all().map_err(io_error("V3_LOCK_SYNC_FAILED"))?;
                    return Ok(Self {
                        path: path.to_owned(),
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    let owner_is_dead = lock_owner_pid(path).is_some_and(|pid| !process_alive(pid));
                    let stale = fs::metadata(path)
                        .and_then(|metadata| metadata.modified())
                        .ok()
                        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
                        .is_some_and(|age| age > STALE_LOCK_AGE);
                    if owner_is_dead || stale {
                        let _ = fs::remove_file(path);
                        continue;
                    }
                    if started.elapsed() >= LOCK_TIMEOUT {
                        return Err(V3Error::new(
                            "V3_LOCK_TIMEOUT",
                            V3ErrorCategory::VersionConflict,
                            true,
                            "timed out waiting for the project event-store lock",
                        ));
                    }
                    thread::sleep(Duration::from_millis(15));
                }
                Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                    // Windows: the previous owner's remove_file can still be
                    // settling (delete-pending), which surfaces here as
                    // ACCESS_DENIED instead of AlreadyExists. Treat it as
                    // contention and retry until the name disappears.
                    if started.elapsed() >= LOCK_TIMEOUT {
                        return Err(V3Error::new(
                            "V3_LOCK_TIMEOUT",
                            V3ErrorCategory::VersionConflict,
                            true,
                            "timed out waiting for the project event-store lock",
                        ));
                    }
                    thread::sleep(Duration::from_millis(15));
                }
                Err(error) => return Err(io_error("V3_LOCK_CREATE_FAILED")(error)),
            }
        }
    }
}

fn lock_owner_pid(path: &Path) -> Option<u32> {
    let content = fs::read_to_string(path).ok()?;
    content
        .split_whitespace()
        .find_map(|field| field.strip_prefix("pid="))?
        .parse()
        .ok()
}

#[cfg(unix)]
fn process_alive(pid: u32) -> bool {
    // Signal 0 performs existence/permission checking without sending a signal.
    let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
    result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(windows)]
fn process_alive(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return false;
    }
    unsafe { CloseHandle(handle) };
    true
}

#[cfg(not(any(unix, windows)))]
fn process_alive(_pid: u32) -> bool {
    true
}

impl Drop for LockLease {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn validate_draft(draft: &EventDraft) -> Result<(), V3Error> {
    validate_project_storage_identity(&draft.project_id.0)?;
    for (name, value) in [
        ("event_type", draft.event_type.as_str()),
        ("aggregate_id", draft.aggregate_id.as_str()),
        ("idempotency_key", draft.idempotency_key.as_str()),
        ("project_id", draft.project_id.0.as_str()),
        ("task_id", draft.task_id.0.as_str()),
        ("actor", draft.actor.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(V3Error::new(
                "V3_VALIDATION_ERROR",
                V3ErrorCategory::Validation,
                false,
                format!("{name} must not be empty"),
            ));
        }
    }
    if draft.task_id.0 == "." || draft.task_id.0 == ".." || draft.task_id.0.contains(['/', '\\']) {
        return Err(V3Error::new(
            "V3_TASK_ID_PATH_UNSAFE",
            V3ErrorCategory::Validation,
            false,
            "task_id must not contain path separators or traversal segments",
        ));
    }
    Ok(())
}

fn validate_project_storage_identity(project_id: &str) -> Result<(), V3Error> {
    let valid = project_id.starts_with("project.")
        && project_id.len() <= 200
        && project_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
        && !matches!(project_id, "." | "..");
    if !valid {
        return Err(V3Error::new(
            "V3_PROJECT_ID_PATH_UNSAFE",
            V3ErrorCategory::Validation,
            false,
            "project_id must be a bounded project.* identifier without path separators",
        )
        .with_detail("project_id", project_id.to_owned()));
    }
    Ok(())
}

fn duplicate_result(event: V3EventEnvelope, draft: &EventDraft) -> Result<AppendResult, V3Error> {
    let same_request = event.event_type == draft.event_type
        && event.aggregate_id == draft.aggregate_id
        && event.expected_version == draft.expected_version
        && event.project_id == draft.project_id
        && event.task_id == draft.task_id
        && event.node_id == draft.node_id
        && event.session_id == draft.session_id
        && event.worktree_id == draft.worktree_id
        && event.lease_id == draft.lease_id
        && event.operation_id == draft.operation_id
        && event.actor == draft.actor
        && event.evidence_grade == draft.evidence_grade
        && event.commit_sha == draft.commit_sha
        && event.payload == draft.payload
        && draft
            .occurred_at
            .as_ref()
            .is_none_or(|occurred_at| event.occurred_at == *occurred_at);
    if !same_request {
        return Err(V3Error::new(
            "V3_IDEMPOTENCY_SCOPE_MISMATCH",
            V3ErrorCategory::ScopeMismatch,
            false,
            "idempotency key was already used for a different request scope",
        )
        .with_detail("existing_event_id", event.event_id));
    }
    Ok(AppendResult::Duplicate { event })
}

fn event_from_draft(draft: EventDraft, current_version: u64) -> V3EventEnvelope {
    let recorded_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    V3EventEnvelope {
        event_id: format!("evt.{}", Uuid::new_v4()),
        event_type: draft.event_type,
        event_version: "1.0".to_owned(),
        aggregate_id: draft.aggregate_id,
        aggregate_version: current_version + 1,
        expected_version: draft.expected_version,
        idempotency_key: draft.idempotency_key,
        project_id: draft.project_id,
        task_id: draft.task_id,
        node_id: draft.node_id,
        session_id: draft.session_id,
        worktree_id: draft.worktree_id,
        lease_id: draft.lease_id,
        operation_id: draft.operation_id,
        actor: draft.actor,
        evidence_grade: draft.evidence_grade,
        occurred_at: draft.occurred_at.unwrap_or_else(|| recorded_at.clone()),
        recorded_at,
        commit_sha: draft.commit_sha,
        payload: draft.payload,
    }
}

fn projection_rebuild_in_progress(project_id: &str) -> V3Error {
    V3Error::new(
        "V3_PROJECTION_REBUILD_IN_PROGRESS",
        V3ErrorCategory::StaleResource,
        true,
        "projection rebuild is in progress; views must wait for a synchronized projection",
    )
    .with_detail("project_id", project_id)
    .with_detail("projection_state", "rebuilding")
    .with_detail(
        "repair_action",
        "retry the read or write after the active projection rebuild finishes",
    )
}

fn projection_rebuild_error(
    project_id: &str,
    projection_path: &Path,
    event_count: usize,
    cause: V3Error,
) -> V3Error {
    V3Error::new(
        "V3_PROJECTION_REBUILD_FAILED",
        V3ErrorCategory::Internal,
        true,
        "projection rebuild failed; the event log is durable but the projection is unavailable",
    )
    .with_detail("project_id", project_id)
    .with_detail("event_count", event_count as u64)
    .with_detail("projection_path", projection_path.to_string_lossy().to_string())
    .with_detail("projection_state", "rebuild_failed")
    .with_detail("cause_code", cause.code)
    .with_detail(
        "cause_category",
        serde_json::to_value(cause.category).unwrap_or(Value::String("internal".to_owned())),
    )
    .with_detail("cause_message", cause.message)
    .with_detail(
        "repair_action",
        "fix the projection path or permissions, then retry projection_rebuild; task_view and task_list remain blocked until it is synced",
    )
}

fn projection_update_error(project_id: &str, index_path: &Path, cause: V3Error) -> V3Error {
    V3Error::new(
        "V3_PROJECTION_UPDATE_FAILED",
        V3ErrorCategory::Internal,
        true,
        "event is durable in JSONL but the derived index/projection update failed",
    )
    .with_detail("project_id", project_id.to_owned())
    .with_detail("index_path", index_path.to_string_lossy().to_string())
    .with_detail("cause_code", cause.code)
    .with_detail("cause_message", cause.message)
    .with_detail(
        "repair_action",
        "retry any typed read or run projection_rebuild to catch the SQLite index up from events.jsonl",
    )
}

fn index_sync_timeout(project_id: &str, status: &IndexedStoreStatus) -> V3Error {
    V3Error::new(
        "V3_INDEX_SYNC_TIMEOUT",
        V3ErrorCategory::StaleResource,
        true,
        "timed out waiting for the derived index to reach the JSONL source watermark",
    )
    .with_detail("project_id", project_id.to_owned())
    .with_detail("index_state", status.state.clone())
    .with_detail("source_byte_offset", status.source_byte_offset)
    .with_detail("source_file_bytes", status.source_file_bytes)
    .with_detail(
        "index_path",
        status.database_path.to_string_lossy().to_string(),
    )
    .with_detail(
        "repair_action",
        "retry after the active writer commits; run projection_rebuild if the watermark remains behind",
    )
}

fn indexed_projection_status_value(
    project_id: &str,
    compatibility_projection_path: &Path,
    status: &IndexedStoreStatus,
) -> Value {
    let store_root = compatibility_projection_path.parent();
    let migration_marker = store_root.map(|root| root.join("store-v2-migration.json"));
    let backups_root = store_root.map(|root| root.join("backups"));
    let backup_count = backups_root
        .as_deref()
        .and_then(|root| fs::read_dir(root).ok())
        .map(|entries| entries.filter_map(Result::ok).count())
        .unwrap_or(0);
    json!({
        "state": status.state,
        "rebuild_state": if status.state == "synced" { "idle" } else { "pending" },
        "stale": status.state != "synced",
        "store_format": STORE_FORMAT,
        "store_model_version": STORE_MODEL_VERSION,
        "index_state": status.state,
        "index_path": status.database_path.to_string_lossy(),
        "event_count": status.event_count,
        "projection_event_count": status.event_count,
        "last_global_seq": status.last_global_seq,
        "last_incremental_source_offset": status.source_byte_offset,
        "source_file_bytes": status.source_file_bytes,
        "last_event_timestamp": status.last_event_timestamp,
        "projection_metadata_consistent": status.state == "synced",
        "projection_path": compatibility_projection_path.to_string_lossy(),
        "compatibility_projection_role": "explicit_rebuild_only",
        "migration_state": if migration_marker.as_ref().is_some_and(|path| path.is_file()) { "complete" } else { "unrecorded" },
        "migration_marker_path": migration_marker.map(|path| path.to_string_lossy().to_string()),
        "source_backup_root": backups_root.map(|path| path.to_string_lossy().to_string()),
        "source_backup_count": backup_count,
        "error": Value::Null,
        "repair_action": if status.state == "synced" {
            Value::Null
        } else {
            Value::String("retry a typed read or run projection_rebuild to catch the index up from events.jsonl".to_owned())
        },
        "project_id": project_id,
    })
}

fn projection_status_value(
    project_id: &str,
    projection_path: &Path,
    state: &str,
    stale: bool,
    event_count: usize,
    projection_event_count: Option<usize>,
    last_event_timestamp: Option<String>,
    metadata_consistent: bool,
    error: Option<V3Error>,
) -> Value {
    let error_value = error.map(|error| {
        json!({
            "code": error.code,
            "category": serde_json::to_value(error.category).unwrap_or(Value::String("internal".to_owned())),
            "retryable": error.retryable,
            "message": error.message,
            "details": error.details,
        })
    });
    let rebuild_state = match state {
        "synced" => "idle",
        "stale" => "pending",
        "rebuilding" => "rebuilding",
        "rebuild_failed" => "rebuild_failed",
        _ => "unknown",
    };
    json!({
        "state": state,
        "rebuild_state": rebuild_state,
        "stale": stale,
        "event_count": event_count,
        "projection_event_count": projection_event_count,
        "last_event_timestamp": last_event_timestamp,
        "projection_metadata_consistent": metadata_consistent,
        "projection_path": projection_path.to_string_lossy(),
        "error": error_value,
        "repair_action": if state == "synced" { Value::Null } else { Value::String("retry projection_rebuild after resolving the reported projection state".to_owned()) },
        "project_id": project_id,
    })
}

fn append_line(path: &Path, event: &V3EventEnvelope) -> Result<(), V3Error> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(io_error("V3_APPEND_OPEN_FAILED"))?;
    serde_json::to_writer(&mut file, event).map_err(json_error("V3_EVENT_SERIALIZE_FAILED"))?;
    file.write_all(b"\n")
        .map_err(io_error("V3_APPEND_WRITE_FAILED"))?;
    file.sync_all().map_err(io_error("V3_APPEND_SYNC_FAILED"))
}

#[cfg(test)]
fn read_events(path: &Path) -> Result<Vec<V3EventEnvelope>, V3Error> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(io_error("V3_EVENT_READ_FAILED")(error)),
    };
    content
        .lines()
        .enumerate()
        .map(|(index, line)| {
            serde_json::from_str(line).map_err(|error| {
                V3Error::new(
                    "V3_CORRUPT_LOG",
                    V3ErrorCategory::CorruptLog,
                    false,
                    error.to_string(),
                )
                .with_detail("line", (index + 1) as u64)
            })
        })
        .collect()
}

fn recover_partial_tail(path: &Path, quarantine_dir: &Path) -> Result<(), V3Error> {
    let mut file = match OpenOptions::new().read(true).write(true).open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(io_error("V3_RECOVERY_OPEN_FAILED")(error)),
    };
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(io_error("V3_RECOVERY_READ_FAILED"))?;
    if bytes.is_empty() || bytes.ends_with(b"\n") {
        return Ok(());
    }
    let valid_len = bytes
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |index| index + 1);
    fs::create_dir_all(quarantine_dir).map_err(io_error("V3_QUARANTINE_CREATE_FAILED"))?;
    let quarantine_path = quarantine_dir.join(format!("partial-{}.bin", Uuid::new_v4()));
    fs::write(&quarantine_path, &bytes[valid_len..])
        .map_err(io_error("V3_QUARANTINE_WRITE_FAILED"))?;
    file.set_len(valid_len as u64)
        .map_err(io_error("V3_RECOVERY_TRUNCATE_FAILED"))?;
    file.seek(SeekFrom::Start(valid_len as u64))
        .map_err(io_error("V3_RECOVERY_SEEK_FAILED"))?;
    file.sync_all().map_err(io_error("V3_RECOVERY_SYNC_FAILED"))
}

fn io_error(code: &'static str) -> impl FnOnce(std::io::Error) -> V3Error {
    move |error| V3Error::new(code, V3ErrorCategory::Internal, true, error.to_string())
}

fn json_error(code: &'static str) -> impl FnOnce(serde_json::Error) -> V3Error {
    move |error| V3Error::new(code, V3ErrorCategory::Internal, false, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v3::domain::{EvidenceGrade, NodeId, ProjectId, SessionId, TaskId, WorktreeId};
    use serde_json::json;
    use std::collections::BTreeSet;
    use std::sync::Barrier;

    fn root() -> PathBuf {
        let root = std::env::temp_dir().join(format!("vibehub-v3-store-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub")).unwrap();
        root
    }

    fn draft(expected_version: u64, key: &str) -> EventDraft {
        EventDraft {
            event_type: "session.opened".to_owned(),
            aggregate_id: "session.main".to_owned(),
            expected_version,
            idempotency_key: key.to_owned(),
            project_id: ProjectId::from("project.test"),
            task_id: TaskId::from("task.test"),
            node_id: None,
            session_id: None,
            worktree_id: None,
            lease_id: None,
            operation_id: None,
            actor: "test".to_owned(),
            evidence_grade: EvidenceGrade::HardObserved,
            occurred_at: None,
            commit_sha: None,
            payload: json!({}),
        }
    }

    fn legacy_store_fixture() -> (PathBuf, V3EventStore) {
        let root = root();
        let store = V3EventStore::open(&root).unwrap();
        let paths = store.paths("project.test");
        fs::create_dir_all(&paths.root).unwrap();
        let first = event_from_draft(draft(0, "key.legacy.one"), 0);
        let second = event_from_draft(draft(1, "key.legacy.two"), 1);
        append_line(&paths.events, &first).unwrap();
        append_line(&paths.events, &second).unwrap();
        let projection = super::super::projection::fold("project.test", &[first, second]);
        super::super::projection::write_atomic(&paths.projection, &projection).unwrap();
        (root, store)
    }

    #[test]
    fn duplicate_is_a_no_op_and_stale_version_conflicts() {
        let root = root();
        let store = V3EventStore::open(&root).unwrap();
        let first = store.append(draft(0, "key.one")).unwrap();
        let duplicate = store.append(draft(0, "key.one")).unwrap();
        assert!(matches!(first, AppendResult::Appended { .. }));
        assert!(matches!(duplicate, AppendResult::Duplicate { .. }));
        let error = store.append(draft(0, "key.two")).unwrap_err();
        assert_eq!(error.category, V3ErrorCategory::VersionConflict);
        assert_eq!(store.load_project("project.test").unwrap().len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn partial_tail_is_quarantined_before_next_append() {
        let root = root();
        let store = V3EventStore::open(&root).unwrap();
        store.append(draft(0, "key.one")).unwrap();
        let paths = store.paths("project.test");
        OpenOptions::new()
            .append(true)
            .open(&paths.events)
            .unwrap()
            .write_all(b"{partial")
            .unwrap();
        store.append(draft(1, "key.two")).unwrap();
        assert_eq!(store.load_project("project.test").unwrap().len(), 2);
        assert_eq!(fs::read_dir(paths.quarantine).unwrap().count(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn idempotency_key_cannot_be_reused_for_another_scope() {
        let root = root();
        let store = V3EventStore::open(&root).unwrap();
        store.append(draft(0, "key.shared")).unwrap();
        let mut different = draft(0, "key.shared");
        different.aggregate_id = "session.other".to_owned();
        let error = store.append(different).unwrap_err();
        assert_eq!(error.category, V3ErrorCategory::ScopeMismatch);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn idempotency_key_cannot_hide_changed_request_semantics() {
        let root = root();
        let store = V3EventStore::open(&root).unwrap();
        store.append(draft(0, "key.shared")).unwrap();

        let mut changed_payload = draft(0, "key.shared");
        changed_payload.payload = json!({"read_only": true});
        assert_eq!(
            store.append(changed_payload).unwrap_err().code,
            "V3_IDEMPOTENCY_SCOPE_MISMATCH"
        );

        let mut changed_actor = draft(0, "key.shared");
        changed_actor.actor = "other-agent".to_owned();
        assert_eq!(
            store.append(changed_actor).unwrap_err().code,
            "V3_IDEMPOTENCY_SCOPE_MISMATCH"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dead_process_lock_is_recovered_without_stale_timeout() {
        let root = root();
        let store = V3EventStore::open(&root).unwrap();
        let paths = store.paths("project.test");
        fs::create_dir_all(&paths.root).unwrap();
        fs::write(
            &paths.lock,
            "pid=4294967294 acquired_at=2026-01-01T00:00:00Z\n",
        )
        .unwrap();
        let started = Instant::now();
        store.append(draft(0, "key.recovered")).unwrap();
        // The bound proves recovery did not wait for STALE_LOCK_AGE; the
        // budget is generous because a cold first append also initializes
        // the derived index, which is slow on throttled CI runners.
        assert!(started.elapsed() < Duration::from_secs(5));
        assert!(!paths.lock.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn live_process_lock_is_not_stolen() {
        let root = root();
        let store = V3EventStore::open(&root).unwrap();
        let paths = store.paths("project.test");
        fs::create_dir_all(&paths.root).unwrap();
        fs::write(
            &paths.lock,
            format!(
                "pid={} acquired_at={}\n",
                std::process::id(),
                Utc::now().to_rfc3339()
            ),
        )
        .unwrap();
        let error = store.append(draft(0, "key.blocked")).unwrap_err();
        assert_eq!(error.code, "V3_LOCK_TIMEOUT");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn steady_state_append_updates_index_without_rewriting_compatibility_projection() {
        let root = root();
        let store = V3EventStore::open(&root).unwrap();
        let result = store
            .append_with_rebuild(draft(0, "key.rebuild.one"))
            .unwrap();
        assert!(matches!(result, AppendResult::Appended { .. }));

        let proj_path = store.projection_path("project.test");
        assert!(!proj_path.exists());
        assert!(store.paths("project.test").index.exists());

        // Append a second event with rebuild
        store
            .append_with_rebuild(draft(1, "key.rebuild.two"))
            .unwrap();
        let projection2 = store.load_projection("project.test").unwrap();
        assert_eq!(
            projection2.source_event_ids.len(),
            2,
            "indexed projection should reflect both events"
        );
        let status = store.projection_status("project.test").unwrap();
        assert_eq!(status["store_format"], "2");
        assert_eq!(status["index_state"], "synced");

        store.rebuild_projection("project.test").unwrap();
        assert!(
            proj_path.exists(),
            "explicit rebuild writes compatibility projection"
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn append_returns_structured_error_when_incremental_projection_transaction_fails() {
        let root = root();
        let store = V3EventStore::open(&root).unwrap();
        // First write a valid event
        store.append(draft(0, "key.noblock")).unwrap();

        let paths = store.paths("project.test");
        let connection = rusqlite::Connection::open(&paths.index).unwrap();
        connection
            .execute_batch(
                "CREATE TRIGGER fail_task_projection
                 BEFORE INSERT ON task_projections
                 BEGIN SELECT RAISE(FAIL, 'injected projection failure'); END;",
            )
            .unwrap();
        drop(connection);

        let error = store
            .append_with_rebuild(draft(1, "key.noblock.two"))
            .unwrap_err();
        assert_eq!(error.code, "V3_PROJECTION_UPDATE_FAILED");
        assert_eq!(error.details["project_id"], "project.test");
        assert!(error.details["cause_code"].as_str().is_some());
        assert!(error.details["repair_action"].as_str().is_some());
        let connection = rusqlite::Connection::open(&paths.index).unwrap();
        connection
            .execute_batch("DROP TRIGGER fail_task_projection;")
            .unwrap();
        drop(connection);
        assert_eq!(store.load_project("project.test").unwrap().len(), 2);
        let status = store.projection_status("project.test").unwrap();
        assert_eq!(status["state"], "synced");
        assert_eq!(status["rebuild_state"], "idle");
        assert_eq!(status["stale"], false);
        assert_eq!(status["event_count"], 2);

        // Clean up
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn append_with_rebuild_keeps_counts_consistent_across_restart() {
        let root = root();
        {
            let store = V3EventStore::open(&root).unwrap();
            for (version, key) in [
                (0, "key.continuous.one"),
                (1, "key.continuous.two"),
                (2, "key.continuous.three"),
            ] {
                assert!(matches!(
                    store.append_with_rebuild(draft(version, key)).unwrap(),
                    AppendResult::Appended { .. }
                ));
                let status = store.projection_status("project.test").unwrap();
                assert_eq!(status["state"], "synced");
                assert_eq!(status["stale"], false);
                assert_eq!(status["event_count"], version + 1);
                assert_eq!(status["projection_event_count"], version + 1);
            }
        }

        let reopened = V3EventStore::open(&root).unwrap();
        let status = reopened.projection_status("project.test").unwrap();
        assert_eq!(status["state"], "synced");
        assert_eq!(status["stale"], false);
        assert_eq!(status["event_count"], 3);
        assert_eq!(status["projection_event_count"], 3);
        assert_eq!(reopened.load_project("project.test").unwrap().len(), 3);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rebuild_projection_is_idempotent() {
        let root = root();
        let store = V3EventStore::open(&root).unwrap();
        store.append(draft(0, "key.idem")).unwrap();
        store.append(draft(1, "key.idem.two")).unwrap();

        store.rebuild_projection("project.test").unwrap();
        let proj1 = fs::read_to_string(store.projection_path("project.test")).unwrap();

        store.rebuild_projection("project.test").unwrap();
        let proj2 = fs::read_to_string(store.projection_path("project.test")).unwrap();

        assert_eq!(proj1, proj2, "rebuild should be deterministic");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn indexed_projection_is_fresh_without_compatibility_file() {
        let root = root();
        let store = V3EventStore::open(&root).unwrap();
        store.append(draft(0, "key.stale.missing")).unwrap();

        assert!(!store.projection_path("project.test").exists());
        assert!(!store.projection_is_stale("project.test").unwrap());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn indexed_projection_stays_fresh_after_plain_append() {
        let root = root();
        let store = V3EventStore::open(&root).unwrap();
        store
            .append_with_rebuild(draft(0, "key.stale.new"))
            .unwrap();

        // Projection is fresh after rebuild
        assert!(!store.projection_is_stale("project.test").unwrap());

        // Store format 2 indexes every append; projection.json remains an
        // explicit-rebuild compatibility artifact.
        store.append(draft(1, "key.stale.newer")).unwrap();
        assert!(!store.projection_is_stale("project.test").unwrap());
        assert!(!store.projection_path("project.test").exists());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn projection_status_reports_correct_counts() {
        let root = root();
        let store = V3EventStore::open(&root).unwrap();
        store.append_with_rebuild(draft(0, "key.status")).unwrap();

        let status = store.projection_status("project.test").unwrap();
        assert_eq!(status["stale"], false);
        assert_eq!(status["store_format"], "2");
        assert_eq!(status["store_model_version"], STORE_MODEL_VERSION);
        assert_eq!(status["index_state"], "synced");
        assert_eq!(status["event_count"], 1);
        assert_eq!(status["projection_event_count"], 1);
        assert_eq!(status["last_global_seq"], 1);
        assert!(status["last_incremental_source_offset"].as_u64().unwrap() > 0);
        assert_eq!(
            status["last_incremental_source_offset"],
            status["source_file_bytes"]
        );
        assert!(status["last_event_timestamp"].as_str().is_some());
        assert!(status["repair_action"].is_null());

        // Plain append updates the indexed projection in the same logical
        // write, so its watermark remains synchronized.
        store.append(draft(1, "key.status.two")).unwrap();
        let status2 = store.projection_status("project.test").unwrap();
        assert_eq!(status2["stale"], false);
        assert_eq!(status2["event_count"], 2);
        assert_eq!(status2["projection_event_count"], 2);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn projection_status_when_no_projection_exists() {
        let root = root();
        let store = V3EventStore::open(&root).unwrap();
        // An empty store format 2 index is a valid synchronized projection.
        let status = store.projection_status("project.test").unwrap();
        assert_eq!(status["stale"], false);
        assert_eq!(status["event_count"], 0);
        assert_eq!(status["projection_event_count"], 0);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn projection_status_reports_rebuilding_state() {
        let root = root();
        let store = V3EventStore::open(&root).unwrap();
        store.set_projection_runtime(ProjectionRuntimeState::Rebuilding);

        let status = store.projection_status("project.test").unwrap();
        assert_eq!(status["state"], "rebuilding");
        assert_eq!(status["rebuild_state"], "rebuilding");
        assert_eq!(status["stale"], true);
        assert!(status["repair_action"].as_str().is_some());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn indexed_shards_are_field_equivalent_to_full_jsonl_replay() {
        let root = root();
        let store = V3EventStore::open(&root).unwrap();

        let mut plan = draft(0, "key.equivalent.plan");
        plan.event_type = "plan.node_added".to_owned();
        plan.aggregate_id = "task.test".to_owned();
        plan.node_id = Some(NodeId("node.test".to_owned()));
        plan.payload = json!({
            "node_id": "node.test",
            "title": "Indexed projection",
            "goal": "Stay equivalent",
            "scope": ["event_store.rs"],
            "dependencies": []
        });
        store.append_with_rebuild(plan).unwrap();

        let mut opened = draft(0, "key.equivalent.open");
        opened.aggregate_id = "session.test".to_owned();
        opened.session_id = Some(SessionId("session.test".to_owned()));
        opened.node_id = Some(NodeId("node.test".to_owned()));
        store.append_with_rebuild(opened).unwrap();

        let mut progress = draft(1, "key.equivalent.progress");
        progress.event_type = "progress.logged".to_owned();
        progress.aggregate_id = "session.test".to_owned();
        progress.session_id = Some(SessionId("session.test".to_owned()));
        progress.node_id = Some(NodeId("node.test".to_owned()));
        progress.payload = json!({"summary": "indexed"});
        store.append_with_rebuild(progress).unwrap();

        let mut worktree = draft(0, "key.equivalent.worktree");
        worktree.event_type = "worktree.planned".to_owned();
        worktree.aggregate_id = "worktree.test".to_owned();
        worktree.node_id = Some(NodeId("node.test".to_owned()));
        worktree.worktree_id = Some(WorktreeId("worktree.test".to_owned()));
        worktree.payload = json!({"eligibility_digest": "a".repeat(64), "read_only": false});
        store.append_with_rebuild(worktree).unwrap();

        let mut memory = draft(0, "key.equivalent.memory");
        memory.event_type = "memory.created".to_owned();
        memory.aggregate_id = "memory.test".to_owned();
        memory.task_id = TaskId("task.memory".to_owned());
        memory.payload = json!({"entry_id": "memory.test"});
        store.append_with_rebuild(memory).unwrap();

        let indexed = store.indexed_projection("project.test").unwrap();
        let jsonl_events = read_events(&store.paths("project.test").events).unwrap();
        let replayed = super::super::projection::fold("project.test", &jsonl_events);
        assert_eq!(indexed, replayed);
        assert_eq!(
            store
                .load_task_events("project.test", "task.test")
                .unwrap()
                .len(),
            4
        );
        assert_eq!(
            store
                .aggregate_version("project.test", "session.test")
                .unwrap(),
            2
        );
        assert!(store
            .event_by_idempotency_key("project.test", "key.equivalent.progress")
            .unwrap()
            .is_some());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn independent_task_writes_preserve_global_order_and_projection_equivalence() {
        const WRITERS: usize = 24;
        let root = root();
        let store = Arc::new(V3EventStore::open(&root).unwrap());
        for index in 0..WRITERS {
            let task_id = format!("task.concurrent.{index:02}");
            let mut created = draft(0, &format!("key.concurrent.seed.{index:02}"));
            created.event_type = "task.created".to_owned();
            created.aggregate_id = task_id.clone();
            created.task_id = TaskId(task_id);
            created.payload = json!({"workflow_profile": "standard"});
            store.append_with_rebuild(created).unwrap();
        }

        let barrier = Arc::new(Barrier::new(WRITERS));
        let handles = (0..WRITERS)
            .map(|index| {
                let store = Arc::clone(&store);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    let task_id = format!("task.concurrent.{index:02}");
                    let mut event = draft(
                        if index % 3 == 0 { 1 } else { 0 },
                        &format!("key.concurrent.write.{index:02}"),
                    );
                    event.task_id = TaskId(task_id.clone());
                    match index % 3 {
                        0 => {
                            event.event_type = "plan.node_added".to_owned();
                            event.aggregate_id = task_id;
                            event.node_id = Some(NodeId(format!("node.concurrent.{index:02}")));
                            event.payload = json!({
                                "node_id": format!("node.concurrent.{index:02}"),
                                "title": "Concurrent plan",
                                "goal": "Preserve order",
                                "scope": ["event_store.rs"],
                                "dependencies": []
                            });
                        }
                        1 => {
                            let session_id = format!("session.concurrent.{index:02}");
                            event.aggregate_id = session_id.clone();
                            event.session_id = Some(SessionId(session_id));
                            event.event_type = "session.opened".to_owned();
                        }
                        _ => {
                            let session_id = format!("session.concurrent.{index:02}");
                            event.aggregate_id = session_id.clone();
                            event.session_id = Some(SessionId(session_id));
                            event.event_type = "progress.logged".to_owned();
                            event.payload = json!({"summary": "concurrent progress"});
                        }
                    }
                    barrier.wait();
                    store.append_with_rebuild(event)
                })
            })
            .collect::<Vec<_>>();

        for handle in handles {
            assert!(matches!(
                handle.join().unwrap().unwrap(),
                AppendResult::Appended { .. }
            ));
        }
        let indexed = store.indexed_projection("project.test").unwrap();
        let jsonl_events = read_events(&store.paths("project.test").events).unwrap();
        assert_eq!(jsonl_events.len(), WRITERS * 2);
        assert_eq!(
            indexed,
            super::super::projection::fold("project.test", &jsonl_events)
        );
        assert_eq!(
            indexed
                .source_event_ids
                .iter()
                .collect::<BTreeSet<_>>()
                .len(),
            WRITERS * 2
        );
        assert!(!store.projection_path("project.test").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn hot_aggregate_conflicts_and_duplicate_retries_are_exact_under_concurrency() {
        const CONTENDERS: usize = 16;
        let root = root();
        let store = Arc::new(V3EventStore::open(&root).unwrap());
        let barrier = Arc::new(Barrier::new(CONTENDERS));
        let handles = (0..CONTENDERS)
            .map(|index| {
                let store = Arc::clone(&store);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    let mut event = draft(0, &format!("key.hot.{index:02}"));
                    event.aggregate_id = "session.hot".to_owned();
                    event.session_id = Some(SessionId("session.hot".to_owned()));
                    barrier.wait();
                    store.append_with_rebuild(event)
                })
            })
            .collect::<Vec<_>>();
        let mut appended = 0;
        let mut conflicts = 0;
        for handle in handles {
            match handle.join().unwrap() {
                Ok(AppendResult::Appended { .. }) => appended += 1,
                Err(error) if error.code == "V3_VERSION_CONFLICT" => conflicts += 1,
                other => panic!("unexpected hotspot result: {other:?}"),
            }
        }
        assert_eq!(appended, 1);
        assert_eq!(conflicts, CONTENDERS - 1);

        let barrier = Arc::new(Barrier::new(CONTENDERS));
        let handles = (0..CONTENDERS)
            .map(|_| {
                let store = Arc::clone(&store);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    let mut event = draft(1, "key.hot.duplicate");
                    event.event_type = "progress.logged".to_owned();
                    event.aggregate_id = "session.hot".to_owned();
                    event.session_id = Some(SessionId("session.hot".to_owned()));
                    event.payload = json!({"summary": "same request"});
                    barrier.wait();
                    store.append_with_rebuild(event)
                })
            })
            .collect::<Vec<_>>();
        let mut appended = 0;
        let mut duplicates = 0;
        for handle in handles {
            match handle.join().unwrap().unwrap() {
                AppendResult::Appended { .. } => appended += 1,
                AppendResult::Duplicate { .. } => duplicates += 1,
            }
        }
        assert_eq!(appended, 1);
        assert_eq!(duplicates, CONTENDERS - 1);
        assert_eq!(store.load_project("project.test").unwrap().len(), 2);
        assert_eq!(
            store
                .aggregate_version("project.test", "session.hot")
                .unwrap(),
            2
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn index_lag_is_caught_up_without_serving_a_silent_old_projection() {
        let root = root();
        let store = V3EventStore::open(&root).unwrap();
        store.append_with_rebuild(draft(0, "key.lag.one")).unwrap();
        let paths = store.paths("project.test");
        let lagged_event = event_from_draft(draft(1, "key.lag.two"), 1);
        append_line(&paths.events, &lagged_event).unwrap();

        let index = IndexedProjectStore::open(&paths.index).unwrap();
        let behind = index.status(&paths.events).unwrap();
        assert_eq!(behind.state, "behind");
        assert!(behind.source_byte_offset < behind.source_file_bytes);

        let projection = store.indexed_projection("project.test").unwrap();
        assert_eq!(projection.total_event_count, 2);
        assert!(projection.source_event_ids.contains(&lagged_event.event_id));
        let synced = store.projection_status("project.test").unwrap();
        assert_eq!(synced["index_state"], "synced");
        assert_eq!(synced["event_count"], 2);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sqlite_writer_wait_never_holds_the_jsonl_sequencing_lock() {
        let root = root();
        let store = Arc::new(V3EventStore::open(&root).unwrap());
        store
            .append_with_rebuild(draft(0, "key.wait.seed"))
            .unwrap();
        let paths = store.paths("project.test");
        let blocker = rusqlite::Connection::open(&paths.index).unwrap();
        blocker.execute_batch("BEGIN IMMEDIATE").unwrap();

        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let writer_store = Arc::clone(&store);
        let writer = thread::spawn(move || {
            started_tx.send(()).unwrap();
            writer_store.append_with_rebuild(draft(1, "key.wait.write"))
        });
        started_rx.recv().unwrap();
        thread::sleep(Duration::from_millis(100));

        let lock_started = Instant::now();
        let lease = LockLease::acquire(&paths.lock).unwrap();
        assert!(
            lock_started.elapsed() < Duration::from_secs(1),
            "a writer waiting on SQLite must not retain the JSONL lock"
        );
        drop(lease);
        blocker.execute_batch("ROLLBACK").unwrap();
        assert!(matches!(
            writer.join().unwrap().unwrap(),
            AppendResult::Appended { .. }
        ));
        assert_eq!(store.load_project("project.test").unwrap().len(), 2);
        // Windows cannot remove a directory tree under an open SQLite file.
        drop(blocker);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn legacy_store_migration_is_atomic_backed_up_and_idempotent() {
        let (root, store) = legacy_store_fixture();
        let paths = store.paths("project.test");
        let events_before = fs::read(&paths.events).unwrap();
        let projection_before = fs::read(&paths.projection).unwrap();
        let task_dir = root.join(".vibehub/tasks/task.test");
        fs::create_dir_all(&task_dir).unwrap();
        fs::write(
            task_dir.join("task.yaml"),
            "task_id: task.test\ntitle: Keep me\n",
        )
        .unwrap();
        fs::write(root.join(".vibehub/tasks/current"), "current-sentinel").unwrap();
        fs::write(root.join(".vibehub/legacy-v2.keep"), "legacy-sentinel").unwrap();
        fs::write(root.join("user.keep"), "user-sentinel").unwrap();

        assert_eq!(store.load_project("project.test").unwrap().len(), 2);
        assert!(paths.index.is_file());
        assert!(paths.migration_marker.is_file());
        let status = store.projection_status("project.test").unwrap();
        assert_eq!(status["migration_state"], "complete");
        assert_eq!(status["source_backup_count"], 1);
        let backups = fs::read_dir(&paths.backups)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.path().is_dir())
            .collect::<Vec<_>>();
        assert_eq!(backups.len(), 1);
        assert_eq!(
            fs::read(backups[0].path().join("events.jsonl")).unwrap(),
            events_before
        );
        assert_eq!(
            fs::read(backups[0].path().join("projection.json")).unwrap(),
            projection_before
        );
        let manifest: Value = serde_json::from_str(
            &fs::read_to_string(backups[0].path().join("manifest.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest["project_id"], "project.test");
        assert_eq!(manifest["source_store_format"], "1");
        assert_eq!(manifest["target_store_format"], "2");

        assert_eq!(store.load_project("project.test").unwrap().len(), 2);
        assert_eq!(fs::read_dir(&paths.backups).unwrap().count(), 1);
        assert_eq!(fs::read(&paths.events).unwrap(), events_before);
        assert_eq!(fs::read(&paths.projection).unwrap(), projection_before);
        assert_eq!(
            fs::read_to_string(task_dir.join("task.yaml")).unwrap(),
            "task_id: task.test\ntitle: Keep me\n"
        );
        assert_eq!(
            fs::read_to_string(root.join(".vibehub/tasks/current")).unwrap(),
            "current-sentinel"
        );
        assert_eq!(
            fs::read_to_string(root.join(".vibehub/legacy-v2.keep")).unwrap(),
            "legacy-sentinel"
        );
        assert_eq!(
            fs::read_to_string(root.join("user.keep")).unwrap(),
            "user-sentinel"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn migration_preserves_historical_t_prefixed_task_identity() {
        let root = root();
        let store = V3EventStore::open(&root).unwrap();
        let paths = store.paths("project.test");
        fs::create_dir_all(&paths.root).unwrap();
        let mut historical = draft(0, "key.historical-task");
        historical.task_id = TaskId("T-20260713051939-3465e637".to_owned());
        historical.aggregate_id = historical.task_id.0.clone();
        let historical = event_from_draft(historical, 0);
        append_line(&paths.events, &historical).unwrap();

        let migrated = store.load_project("project.test").unwrap();
        assert_eq!(migrated, vec![historical.clone()]);
        assert_eq!(
            store
                .load_task_events("project.test", &historical.task_id.0)
                .unwrap(),
            vec![historical]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn migration_rebuilds_cross_task_session_and_binding_shards_globally() {
        let root = root();
        let store = V3EventStore::open(&root).unwrap();
        let paths = store.paths("project.test");
        fs::create_dir_all(&paths.root).unwrap();

        let mut opened = draft(0, "key.cross.open");
        opened.task_id = TaskId("task.first".to_owned());
        opened.session_id = Some(SessionId("session.cross".to_owned()));
        opened.aggregate_id = "session.cross".to_owned();
        let opened = event_from_draft(opened, 0);

        let mut bound = draft(1, "key.cross.bind");
        bound.event_type = "session.task_bound".to_owned();
        bound.task_id = TaskId("task.second".to_owned());
        bound.session_id = Some(SessionId("session.cross".to_owned()));
        bound.aggregate_id = "session.cross".to_owned();
        bound.payload = json!({
            "binding": {
                "schema_version": "1.0",
                "project_id": "project.test",
                "interaction_id": "interaction.cross",
                "session_id": "session.cross",
                "agent_id": "codex",
                "host": "codex",
                "bound_task_id": "task.second",
                "binding_revision": 1,
                "freshness": "fresh",
                "source": "explicit_task_id",
                "status": "bound",
                "target_task_revision": 1,
                "bound_at": null
            }
        });
        let bound = event_from_draft(bound, 1);

        let mut progress = draft(2, "key.cross.progress");
        progress.event_type = "progress.logged".to_owned();
        progress.task_id = TaskId("task.second".to_owned());
        progress.session_id = Some(SessionId("session.cross".to_owned()));
        progress.aggregate_id = "session.cross".to_owned();
        let progress = event_from_draft(progress, 2);
        let events = vec![opened, bound, progress];
        for event in &events {
            append_line(&paths.events, event).unwrap();
        }

        let indexed = store.indexed_projection("project.test").unwrap();
        let replayed = super::super::projection::fold("project.test", &events);
        assert_eq!(indexed, replayed);
        assert_eq!(indexed.sessions["session.cross"].task_id, "task.first");
        assert_eq!(
            indexed.session_bindings["session.cross"]
                .bound_task_id
                .as_deref(),
            Some("task.second")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn migration_disk_full_and_rename_faults_roll_back_then_retry() {
        let (root, store) = legacy_store_fixture();
        let paths = store.paths("project.test");
        let events_before = fs::read(&paths.events).unwrap();
        let projection_before = fs::read(&paths.projection).unwrap();

        let error = IndexedProjectStore::migrate_from_jsonl(
            &paths.index,
            &paths.events,
            &paths.projection,
            &paths.backups,
            &paths.migration_marker,
            "project.test",
            Some(crate::v3::indexed_store::MigrationFault::DiskFull),
        )
        .unwrap_err();
        assert_eq!(error.code, "V3_INDEX_MIGRATION_DISK_FULL");
        assert!(!paths.index.exists());
        assert_eq!(fs::read(&paths.events).unwrap(), events_before);

        let error = IndexedProjectStore::migrate_from_jsonl(
            &paths.index,
            &paths.events,
            &paths.projection,
            &paths.backups,
            &paths.migration_marker,
            "project.test",
            Some(crate::v3::indexed_store::MigrationFault::RenameFailed),
        )
        .unwrap_err();
        assert_eq!(error.code, "V3_INDEX_MIGRATION_RENAME_FAILED");
        assert!(!paths.index.exists());
        assert!(paths.index.with_extension("sqlite3.migrating").is_file());
        assert_eq!(fs::read(&paths.projection).unwrap(), projection_before);

        let migrated = IndexedProjectStore::migrate_from_jsonl(
            &paths.index,
            &paths.events,
            &paths.projection,
            &paths.backups,
            &paths.migration_marker,
            "project.test",
            None,
        )
        .unwrap();
        assert_eq!(migrated.all_events().unwrap().len(), 2);
        assert!(paths.index.is_file());
        assert!(!paths.index.with_extension("sqlite3.migrating").exists());
        assert_eq!(fs::read(&paths.events).unwrap(), events_before);
        assert_eq!(fs::read(&paths.projection).unwrap(), projection_before);
        assert_eq!(fs::read_dir(&paths.backups).unwrap().count(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn corrupt_interrupted_temp_is_quarantined_and_rebuilt() {
        let (root, store) = legacy_store_fixture();
        let paths = store.paths("project.test");
        let temp = paths.index.with_extension("sqlite3.migrating");
        fs::write(&temp, b"not a sqlite database").unwrap();
        let migrated = IndexedProjectStore::migrate_from_jsonl(
            &paths.index,
            &paths.events,
            &paths.projection,
            &paths.backups,
            &paths.migration_marker,
            "project.test",
            None,
        )
        .unwrap();
        assert_eq!(migrated.all_events().unwrap().len(), 2);
        assert!(fs::read_dir(&paths.root)
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .contains("store-v2.sqlite3.failed.")
            }));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn corrupt_final_index_is_quarantined_and_rebuilt_from_jsonl() {
        let (root, store) = legacy_store_fixture();
        let paths = store.paths("project.test");
        let events_before = fs::read(&paths.events).unwrap();

        assert_eq!(store.load_project("project.test").unwrap().len(), 2);
        fs::write(&paths.index, b"not a sqlite database").unwrap();

        let recovered = store.load_project("project.test").unwrap();
        assert_eq!(recovered.len(), 2);
        assert_eq!(fs::read(&paths.events).unwrap(), events_before);
        assert!(fs::read_dir(&paths.root)
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .contains("store-v2.failed.")));
        assert_eq!(
            store.projection_status("project.test").unwrap()["index_state"],
            "synced"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn incomplete_format_two_schema_is_rebuilt_instead_of_patched_in_place() {
        let (root, store) = legacy_store_fixture();
        let paths = store.paths("project.test");
        assert_eq!(store.load_project("project.test").unwrap().len(), 2);
        let connection = rusqlite::Connection::open(&paths.index).unwrap();
        connection
            .execute_batch("DROP TABLE task_projections")
            .unwrap();
        drop(connection);

        let recovered = store.indexed_projection("project.test").unwrap();
        assert_eq!(recovered.total_event_count, 2);
        let connection = rusqlite::Connection::open(&paths.index).unwrap();
        let task_table_exists = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='task_projections')",
                [],
                |row| row.get::<_, bool>(0),
            )
            .unwrap();
        assert!(task_table_exists);
        // Windows cannot remove a directory tree under an open SQLite file;
        // close the test connection before cleanup.
        drop(connection);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unsupported_old_index_and_unsafe_project_identity_fail_closed() {
        let (root, store) = legacy_store_fixture();
        let paths = store.paths("project.test");
        let connection = rusqlite::Connection::open(&paths.index).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 INSERT INTO meta(key, value) VALUES('store_format', '1');",
            )
            .unwrap();
        drop(connection);
        let events_before = fs::read(&paths.events).unwrap();
        let error = store.load_project("project.test").unwrap_err();
        assert_eq!(error.code, "V3_INDEX_FORMAT_UNSUPPORTED");
        assert_eq!(fs::read(&paths.events).unwrap(), events_before);

        let unsafe_error = store.load_project("../escape").unwrap_err();
        assert_eq!(unsafe_error.code, "V3_PROJECT_ID_PATH_UNSAFE");
        assert!(!root.join(".vibehub/v3/escape").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn migration_rejects_symlinked_sources_and_read_only_targets() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let root = root();
        let store = V3EventStore::open(&root).unwrap();
        let paths = store.paths("project.test");
        fs::create_dir_all(&paths.root).unwrap();
        let external = root.join("external-events.jsonl");
        fs::write(&external, b"external-sentinel").unwrap();
        symlink(&external, &paths.events).unwrap();
        let error = IndexedProjectStore::migrate_from_jsonl(
            &paths.index,
            &paths.events,
            &paths.projection,
            &paths.backups,
            &paths.migration_marker,
            "project.test",
            None,
        )
        .unwrap_err();
        assert_eq!(error.code, "V3_INDEX_MIGRATION_SOURCE_SYMLINK");
        assert_eq!(fs::read_to_string(&external).unwrap(), "external-sentinel");
        fs::remove_file(&paths.events).unwrap();

        let event = event_from_draft(draft(0, "key.readonly"), 0);
        append_line(&paths.events, &event).unwrap();
        let original_mode = fs::metadata(&paths.root).unwrap().permissions().mode();
        fs::set_permissions(&paths.root, fs::Permissions::from_mode(0o555)).unwrap();
        let result = IndexedProjectStore::migrate_from_jsonl(
            &paths.index,
            &paths.events,
            &paths.projection,
            &paths.backups,
            &paths.migration_marker,
            "project.test",
            None,
        );
        fs::set_permissions(&paths.root, fs::Permissions::from_mode(original_mode)).unwrap();
        let error = result.unwrap_err();
        assert!(matches!(
            error.code.as_str(),
            "V3_INDEX_MIGRATION_BACKUP_CREATE_FAILED" | "V3_INDEX_OPEN_FAILED"
        ));
        assert!(!paths.index.exists());

        let external_index = root.join("external-index");
        fs::write(&external_index, b"index-sentinel").unwrap();
        symlink(&external_index, &paths.index).unwrap();
        let error = IndexedProjectStore::open(&paths.index).unwrap_err();
        assert_eq!(error.code, "V3_INDEX_PATH_SYMLINK");
        assert_eq!(
            fs::read_to_string(&external_index).unwrap(),
            "index-sentinel"
        );
        fs::remove_dir_all(root).unwrap();
    }
}

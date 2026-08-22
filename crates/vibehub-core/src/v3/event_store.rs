use super::domain::{AppendResult, EventDraft, V3Error, V3ErrorCategory, V3EventEnvelope};
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

const LOCK_TIMEOUT: Duration = Duration::from_secs(3);
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
    /// is still held. A successful return therefore means the event log and
    /// projection describe the same event batch. If rebuilding fails, the
    /// event remains durable but this method returns a structured error and
    /// records the projection as unavailable to this store instance.
    pub fn append_with_rebuild(&self, draft: EventDraft) -> Result<AppendResult, V3Error> {
        self.set_projection_runtime(ProjectionRuntimeState::Rebuilding);
        let result = self.append_inner(draft, true);
        match &result {
            Err(error) if error.code == "V3_PROJECTION_REBUILD_FAILED" => {
                self.set_projection_runtime(ProjectionRuntimeState::Failed(error.clone()));
            }
            _ => self.set_projection_runtime(ProjectionRuntimeState::Idle),
        }
        result
    }

    fn append_inner(&self, draft: EventDraft, rebuild: bool) -> Result<AppendResult, V3Error> {
        validate_draft(&draft)?;
        let paths = self.paths(&draft.project_id.0);
        fs::create_dir_all(&paths.root).map_err(io_error("V3_STORE_CREATE_FAILED"))?;
        let _lease = LockLease::acquire(&paths.lock)?;
        recover_partial_tail(&paths.events, &paths.quarantine)?;
        let events = read_events(&paths.events)?;
        let project_id = draft.project_id.0.clone();
        let result = append_loaded(&paths.events, &events, draft)?;
        if rebuild {
            let mut events_after = events;
            if let AppendResult::Appended { event } = &result {
                events_after.push(event.clone());
            }
            self.write_projection_locked(&project_id, &events_after)?;
        }
        Ok(result)
    }

    /// Rebuild the projection.json from the event log under the same lock
    /// used by append. The returned projection is the exact folded input that
    /// was written.
    pub fn rebuild_projection(&self, project_id: &str) -> Result<V3Projection, V3Error> {
        self.set_projection_runtime(ProjectionRuntimeState::Rebuilding);
        let result = self
            .rebuild_projection_inner(project_id)
            .map_err(|error| {
                if error.code == "V3_PROJECTION_REBUILD_FAILED" {
                    error
                } else {
                    projection_rebuild_error(
                        project_id,
                        &self.projection_path(project_id),
                        0,
                        error,
                    )
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
        fs::create_dir_all(&paths.root).map_err(io_error("V3_STORE_CREATE_FAILED"))?;
        let _lease = LockLease::acquire(&paths.lock)?;
        recover_partial_tail(&paths.events, &paths.quarantine)?;
        let events = read_events(&paths.events)?;
        self.write_projection_locked(project_id, &events)
    }

    fn write_projection_locked(
        &self,
        project_id: &str,
        events: &[V3EventEnvelope],
    ) -> Result<V3Projection, V3Error> {
        let projection = super::projection::fold(project_id, events);
        super::projection::write_atomic(&self.projection_path(project_id), &projection).map_err(
            |error| {
                projection_rebuild_error(
                    project_id,
                    &self.projection_path(project_id),
                    events.len(),
                    error,
                )
            },
        )?;
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
        let events = self.load_project(project_id)?;
        let proj_path = self.projection_path(project_id);
        if !proj_path.exists() {
            return Ok(true);
        }
        let content = fs::read_to_string(&proj_path).map_err(io_error("V3_PROJECTION_READ_FAILED"))?;
        let projection: V3Projection =
            serde_json::from_str(&content).map_err(|error| {
                V3Error::new(
                    "V3_PROJECTION_PARSE_FAILED",
                    V3ErrorCategory::CorruptLog,
                    false,
                    error.to_string(),
                )
            })?;
        // Use total_event_count if set (new projections), otherwise fall back
        // to source_event_ids.len() for backward compatibility with projections
        // written before this field existed.
        let (projection_count, metadata_consistent) = projection_event_count(&projection);
        Ok(!metadata_consistent || projection_count != events.len())
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
        let events = self.load_project(project_id)?;
        let event_count = events.len();
        let proj_path = self.projection_path(project_id);
        let runtime_error = match self.projection_runtime_state() {
            ProjectionRuntimeState::Failed(error) => Some(error),
            _ => None,
        };
        if let Some(error) = runtime_error {
            return Ok(projection_status_value(
                project_id,
                &proj_path,
                "rebuild_failed",
                true,
                event_count,
                None,
                None,
                false,
                Some(error),
            ));
        }
        if !proj_path.exists() {
            return Ok(projection_status_value(
                project_id,
                &proj_path,
                "stale",
                true,
                event_count,
                None,
                None,
                true,
                None,
            ));
        }
        let content = match fs::read_to_string(&proj_path) {
            Ok(content) => content,
            Err(error) => {
                let error = io_error("V3_PROJECTION_READ_FAILED")(error);
                return Ok(projection_status_value(
                    project_id,
                    &proj_path,
                    "rebuild_failed",
                    true,
                    event_count,
                    None,
                    None,
                    false,
                    Some(error),
                ));
            }
        };
        let projection = match serde_json::from_str::<V3Projection>(&content) {
            Ok(projection) => projection,
            Err(error) => {
                let error = V3Error::new(
                    "V3_PROJECTION_PARSE_FAILED",
                    V3ErrorCategory::CorruptLog,
                    false,
                    error.to_string(),
                );
                return Ok(projection_status_value(
                    project_id,
                    &proj_path,
                    "rebuild_failed",
                    true,
                    event_count,
                    None,
                    None,
                    false,
                    Some(error),
                ));
            }
        };
        let (count, metadata_consistent) = projection_event_count(&projection);
        let stale = !metadata_consistent || count != event_count;
        Ok(projection_status_value(
            project_id,
            &proj_path,
            if stale { "stale" } else { "synced" },
            stale,
            event_count,
            Some(count),
            projection.last_event_timestamp,
            metadata_consistent,
            None,
        ))
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
        let paths = self.paths(project_id);
        fs::create_dir_all(&paths.root).map_err(io_error("V3_STORE_CREATE_FAILED"))?;
        let _lease = LockLease::acquire(&paths.lock)?;
        recover_partial_tail(&paths.events, &paths.quarantine)?;
        read_events(&paths.events)
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
            root,
        }
    }
}

struct StorePaths {
    root: PathBuf,
    events: PathBuf,
    lock: PathBuf,
    quarantine: PathBuf,
    projection: PathBuf,
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
    Ok(())
}

fn append_loaded(
    path: &Path,
    events: &[V3EventEnvelope],
    draft: EventDraft,
) -> Result<AppendResult, V3Error> {
    if let Some(event) = events
        .iter()
        .find(|event| event.idempotency_key == draft.idempotency_key)
    {
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
            .with_detail("existing_event_id", event.event_id.clone()));
        }
        return Ok(AppendResult::Duplicate {
            event: event.clone(),
        });
    }

    let current_version = events
        .iter()
        .filter(|event| event.aggregate_id == draft.aggregate_id)
        .map(|event| event.aggregate_version)
        .max()
        .unwrap_or(0);
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

    let recorded_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let event = V3EventEnvelope {
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
    };
    append_line(path, &event)?;
    Ok(AppendResult::Appended { event })
}

fn projection_event_count(projection: &V3Projection) -> (usize, bool) {
    let source_count = projection.source_event_ids.len();
    if projection.total_event_count == 0 {
        // Projections written before total_event_count was introduced are
        // still valid when their source event ids provide the count.
        return (source_count, true);
    }
    let total_count = projection.total_event_count as usize;
    (total_count, total_count == source_count)
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
    use crate::v3::domain::{EvidenceGrade, ProjectId, TaskId};
    use serde_json::json;

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
        assert!(started.elapsed() < Duration::from_secs(1));
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
    fn append_with_rebuild_writes_projection() {
        let root = root();
        let store = V3EventStore::open(&root).unwrap();
        let result = store.append_with_rebuild(draft(0, "key.rebuild.one")).unwrap();
        assert!(matches!(result, AppendResult::Appended { .. }));

        // The projection file should exist after append_with_rebuild
        let proj_path = store.projection_path("project.test");
        assert!(proj_path.exists(), "projection.json should be written after append_with_rebuild");

        // Load and verify the projection contains our event
        let content = fs::read_to_string(&proj_path).unwrap();
        let projection: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(projection["project_id"], "project.test");
        assert!(
            projection["source_event_ids"]
                .as_array()
                .is_some_and(|ids| !ids.is_empty()),
            "projection should reference the appended event"
        );

        // Append a second event with rebuild
        store.append_with_rebuild(draft(1, "key.rebuild.two")).unwrap();
        let content2 = fs::read_to_string(&proj_path).unwrap();
        let projection2: serde_json::Value = serde_json::from_str(&content2).unwrap();
        assert_eq!(
            projection2["source_event_ids"].as_array().unwrap().len(),
            2,
            "projection should reflect both events"
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn append_with_rebuild_returns_structured_error_on_rebuild_failure() {
        let root = root();
        let store = V3EventStore::open(&root).unwrap();
        // First write a valid event
        store.append(draft(0, "key.noblock")).unwrap();

        // Corrupt the projection directory to force rebuild failure
        let paths = store.paths("project.test");
        let proj_dir = paths.projection.parent().unwrap();
        fs::create_dir_all(proj_dir).unwrap();
        // Create a directory where projection.json should be — this will cause write_atomic to fail
        fs::create_dir_all(&paths.projection).unwrap();

        let error = store
            .append_with_rebuild(draft(1, "key.noblock.two"))
            .unwrap_err();
        assert_eq!(error.code, "V3_PROJECTION_REBUILD_FAILED");
        assert_eq!(error.details["project_id"], "project.test");
        assert_eq!(error.details["event_count"], 2);
        assert_eq!(error.details["projection_state"], "rebuild_failed");
        assert!(error.details["cause_code"].as_str().is_some());
        assert!(error.details["repair_action"].as_str().is_some());
        assert_eq!(store.load_project("project.test").unwrap().len(), 2);
        let status = store.projection_status("project.test").unwrap();
        assert_eq!(status["state"], "rebuild_failed");
        assert_eq!(status["rebuild_state"], "rebuild_failed");
        assert_eq!(status["stale"], true);
        assert_eq!(status["event_count"], 2);
        assert!(status["error"].is_object());

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
    fn projection_is_stale_when_missing() {
        let root = root();
        let store = V3EventStore::open(&root).unwrap();
        store.append(draft(0, "key.stale.missing")).unwrap();

        // No projection written yet → stale
        assert!(store.projection_is_stale("project.test").unwrap());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn projection_is_stale_after_new_events() {
        let root = root();
        let store = V3EventStore::open(&root).unwrap();
        store.append_with_rebuild(draft(0, "key.stale.new")).unwrap();

        // Projection is fresh after rebuild
        assert!(!store.projection_is_stale("project.test").unwrap());

        // Append another event without rebuild → stale
        store.append(draft(1, "key.stale.newer")).unwrap();
        assert!(store.projection_is_stale("project.test").unwrap());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn projection_status_reports_correct_counts() {
        let root = root();
        let store = V3EventStore::open(&root).unwrap();
        store.append_with_rebuild(draft(0, "key.status")).unwrap();

        let status = store.projection_status("project.test").unwrap();
        assert_eq!(status["stale"], false);
        assert_eq!(status["event_count"], 1);
        assert_eq!(status["projection_event_count"], 1);

        // Add another event → stale
        store.append(draft(1, "key.status.two")).unwrap();
        let status2 = store.projection_status("project.test").unwrap();
        assert_eq!(status2["stale"], true);
        assert_eq!(status2["event_count"], 2);
        assert_eq!(status2["projection_event_count"], 1);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn projection_status_when_no_projection_exists() {
        let root = root();
        let store = V3EventStore::open(&root).unwrap();
        // No events, no projection
        let status = store.projection_status("project.test").unwrap();
        assert_eq!(status["stale"], true);
        assert_eq!(status["event_count"], 0);
        assert!(status["projection_event_count"].is_null());

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
}

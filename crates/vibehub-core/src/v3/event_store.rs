use super::domain::{AppendResult, EventDraft, V3Error, V3ErrorCategory, V3EventEnvelope};
use chrono::{SecondsFormat, Utc};
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime};
use uuid::Uuid;

const LOCK_TIMEOUT: Duration = Duration::from_secs(3);
const STALE_LOCK_AGE: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct V3EventStore {
    project_root: PathBuf,
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
        Ok(Self { project_root })
    }

    pub fn append(&self, draft: EventDraft) -> Result<AppendResult, V3Error> {
        validate_draft(&draft)?;
        let paths = self.paths(&draft.project_id.0);
        fs::create_dir_all(&paths.root).map_err(io_error("V3_STORE_CREATE_FAILED"))?;
        let _lease = LockLease::acquire(&paths.lock)?;
        recover_partial_tail(&paths.events, &paths.quarantine)?;
        let events = read_events(&paths.events)?;

        if let Some(event) = events
            .iter()
            .find(|event| event.idempotency_key == draft.idempotency_key)
        {
            let same_request = event.event_type == draft.event_type
                && event.aggregate_id == draft.aggregate_id
                && event.project_id == draft.project_id
                && event.task_id == draft.task_id
                && event.node_id == draft.node_id
                && event.session_id == draft.session_id;
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
            actor: draft.actor,
            evidence_grade: draft.evidence_grade,
            occurred_at: draft.occurred_at.unwrap_or_else(|| recorded_at.clone()),
            recorded_at,
            commit_sha: draft.commit_sha,
            payload: draft.payload,
        };
        append_line(&paths.events, &event)?;
        Ok(AppendResult::Appended { event })
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
}

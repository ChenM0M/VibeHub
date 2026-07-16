use super::{fold_task_lifecycle, V3Error, V3ErrorCategory, V3EventStore};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const V3_SCHEMA_VERSION: u32 = 3;
const VIBEHUB_DIR: &str = ".vibehub";
const STAGING_DIR: &str = ".vibehub.v2-migration";
const V3_DIRS: &[&str] = &["events", "projections", "indexes", "runtime"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectLayoutState {
    Absent,
    V2,
    V3,
    MigrationInterrupted,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectLayoutStatus {
    pub state: ProjectLayoutState,
    pub schema_version: Option<u32>,
    pub message: String,
    pub recommended_action: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V3BootstrapResult {
    pub status: String,
    pub archived_legacy_v2: bool,
    pub created_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V3RepairResult {
    pub status: String,
    pub task_id: String,
    pub created_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V3RepairCandidate {
    pub task_id: String,
    pub title: String,
    pub state: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProjectMarker {
    schema_version: u32,
    name: String,
}

#[derive(Debug, Deserialize)]
struct RepairTaskDocument {
    task_id: String,
    #[serde(default)]
    title: String,
}

#[derive(Debug, Serialize)]
struct RepairCurrentTaskPointer<'a> {
    schema_version: u32,
    kind: &'a str,
    task_id: &'a str,
    path: String,
    updated_at: String,
    updated_by: &'a str,
}

pub fn inspect_project_layout(
    project_root: impl AsRef<Path>,
) -> Result<ProjectLayoutStatus, V3Error> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    let root = project_root.join(VIBEHUB_DIR);
    let staging = project_root.join(STAGING_DIR);

    if staging.exists() {
        return Ok(status(
            ProjectLayoutState::MigrationInterrupted,
            read_schema_version(&root),
            "a legacy-v2 migration staging directory is present",
            Some("run `vibehub v3 <project> migrate-recover` before retrying migration"),
        ));
    }
    if !root.exists() {
        return Ok(status(
            ProjectLayoutState::Absent,
            None,
            "project does not contain .vibehub",
            Some("run `vibehub v3 <project> init` to initialize v3"),
        ));
    }
    reject_symlink(&root, "V3_ROOT_SYMLINK")?;
    if !root.is_dir() {
        return Ok(status(
            ProjectLayoutState::Conflict,
            None,
            ".vibehub exists but is not a directory",
            None,
        ));
    }

    let schema_version = read_schema_version(&root);
    if schema_version == Some(V3_SCHEMA_VERSION) {
        return Ok(status(
            ProjectLayoutState::V3,
            schema_version,
            "project uses the v3 disk layout",
            None,
        ));
    }
    if root.join("legacy-v2").exists() {
        return Ok(status(
            ProjectLayoutState::Conflict,
            schema_version,
            "legacy-v2 exists without a valid v3 schema marker",
            Some("do not overwrite the archive; inspect the project and recover manually"),
        ));
    }
    Ok(status(
        ProjectLayoutState::V2,
        schema_version,
        "this is a v2 project",
        Some("run `vibehub v3 <project> migrate` to archive v2 and initialize v3"),
    ))
}

pub fn initialize_v3(project_root: impl AsRef<Path>) -> Result<V3BootstrapResult, V3Error> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    match inspect_project_layout(&project_root)?.state {
        ProjectLayoutState::V3 => Ok(V3BootstrapResult {
            status: "already_initialized".to_owned(),
            archived_legacy_v2: false,
            created_paths: Vec::new(),
        }),
        ProjectLayoutState::Absent => create_v3_root(&project_root),
        state => Err(layout_error(
            "V3_INIT_REQUIRES_EMPTY_PROJECT",
            format!("cannot initialize v3 from layout state {state:?}"),
        )),
    }
}

pub fn migrate_v2_to_v3(project_root: impl AsRef<Path>) -> Result<V3BootstrapResult, V3Error> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    let current = inspect_project_layout(&project_root)?;
    if current.state == ProjectLayoutState::V3 {
        return Ok(V3BootstrapResult {
            status: "already_migrated".to_owned(),
            archived_legacy_v2: project_root.join(VIBEHUB_DIR).join("legacy-v2").is_dir(),
            created_paths: Vec::new(),
        });
    }
    if current.state != ProjectLayoutState::V2 {
        return Err(layout_error(
            "V3_MIGRATION_STATE_INVALID",
            format!("cannot migrate layout state {:?}", current.state),
        ));
    }

    let root = project_root.join(VIBEHUB_DIR);
    let staging = project_root.join(STAGING_DIR);
    reject_symlink(&root, "V3_MIGRATION_ROOT_SYMLINK")?;
    fs::rename(&root, &staging).map_err(|error| io_error("V3_MIGRATION_STAGE_FAILED", error))?;

    let bootstrap = match create_v3_root(&project_root) {
        Ok(result) => result,
        Err(error) => {
            rollback_staged_root(&root, &staging)?;
            return Err(error);
        }
    };
    let archive = root.join("legacy-v2");
    if let Err(error) = fs::rename(&staging, &archive) {
        rollback_staged_root(&root, &staging)?;
        return Err(io_error("V3_MIGRATION_ARCHIVE_FAILED", error));
    }

    let mut created_paths = bootstrap.created_paths;
    created_paths.push(".vibehub/legacy-v2".to_owned());
    Ok(V3BootstrapResult {
        status: "migrated".to_owned(),
        archived_legacy_v2: true,
        created_paths,
    })
}

pub fn recover_interrupted_migration(
    project_root: impl AsRef<Path>,
) -> Result<V3BootstrapResult, V3Error> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    let root = project_root.join(VIBEHUB_DIR);
    let staging = project_root.join(STAGING_DIR);
    if !staging.exists() {
        return Err(layout_error(
            "V3_MIGRATION_NOT_INTERRUPTED",
            "migration staging directory is absent",
        ));
    }
    reject_symlink(&staging, "V3_MIGRATION_STAGE_SYMLINK")?;
    if root.exists() && read_schema_version(&root) == Some(V3_SCHEMA_VERSION) {
        let archive = root.join("legacy-v2");
        if archive.exists() {
            return Err(layout_error(
                "V3_MIGRATION_ARCHIVE_CONFLICT",
                "legacy-v2 already exists; refusing to overwrite it",
            ));
        }
        fs::rename(&staging, &archive)
            .map_err(|error| io_error("V3_MIGRATION_RECOVER_FAILED", error))?;
        return Ok(V3BootstrapResult {
            status: "recovered_forward".to_owned(),
            archived_legacy_v2: true,
            created_paths: vec![".vibehub/legacy-v2".to_owned()],
        });
    }
    if root.exists() {
        return Err(layout_error(
            "V3_MIGRATION_RECOVERY_CONFLICT",
            ".vibehub exists without a valid v3 marker; refusing automatic recovery",
        ));
    }
    fs::rename(&staging, &root).map_err(|error| io_error("V3_MIGRATION_RECOVER_FAILED", error))?;
    Ok(V3BootstrapResult {
        status: "rolled_back_to_v2".to_owned(),
        archived_legacy_v2: false,
        created_paths: Vec::new(),
    })
}

pub fn repair_v3_layout(
    project_root: impl AsRef<Path>,
    task_id: &str,
) -> Result<V3RepairResult, V3Error> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    validate_repair_task_id(task_id)?;
    let root = project_root.join(VIBEHUB_DIR);
    let status = inspect_project_layout(&project_root)?;
    let marker_is_valid = status.state == ProjectLayoutState::V3;
    let recoverable_conflict = status.state == ProjectLayoutState::Conflict
        && status.schema_version.is_none()
        && root.join("legacy-v2").is_dir();
    if !marker_is_valid && !recoverable_conflict {
        return Err(layout_error(
            "V3_REPAIR_STATE_INVALID",
            format!("cannot repair layout state {:?}", status.state),
        ));
    }

    reject_symlink(&root, "V3_REPAIR_ROOT_SYMLINK")?;
    reject_regular_directory(&root.join("legacy-v2"), "V3_REPAIR_LEGACY_INVALID")?;
    let task_path = root.join("tasks").join(task_id).join("task.yaml");
    let task = read_repair_task(&task_path)?;
    if task.task_id != task_id {
        return Err(layout_error(
            "V3_REPAIR_TASK_MISMATCH",
            "task.yaml identity does not match the requested current task",
        ));
    }

    let project_id = project_id(&project_root);
    reject_regular_file(
        &repair_events_path(&root, &project_id),
        "V3_REPAIR_EVENTS_INVALID",
    )?;
    let store = V3EventStore::open(&project_root)?;
    let events = store.load_project(&project_id)?;
    if !events.iter().any(|event| event.task_id.0 == task_id) {
        return Err(layout_error(
            "V3_REPAIR_TASK_EVENTS_MISSING",
            "requested current task has no V3 lifecycle events",
        ));
    }

    let current = root.join("tasks/current");
    if current.exists() {
        return Err(layout_error(
            "V3_REPAIR_CURRENT_EXISTS",
            "current task pointer already exists; refusing to overwrite it",
        ));
    }

    let mut created_paths = Vec::new();
    let marker = root.join("project.yaml");
    let created_marker = !marker_is_valid;
    if created_marker {
        let marker_content = serde_yaml::to_string(&ProjectMarker {
            schema_version: V3_SCHEMA_VERSION,
            name: project_root
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("project")
                .to_owned(),
        })
        .map_err(|error| layout_error("V3_REPAIR_MARKER_FAILED", error.to_string()))?;
        write_new_synced(
            &marker,
            marker_content.as_bytes(),
            "V3_REPAIR_MARKER_FAILED",
        )?;
        created_paths.push(".vibehub/project.yaml".to_owned());
    }

    let pointer = RepairCurrentTaskPointer {
        schema_version: 1,
        kind: "current_task_pointer",
        task_id,
        path: format!(".vibehub/tasks/{task_id}"),
        updated_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        updated_by: "vibehub",
    };
    let pointer_content = serde_yaml::to_string(&pointer)
        .map_err(|error| layout_error("V3_REPAIR_CURRENT_FAILED", error.to_string()))?;
    if let Err(error) = write_new_synced(
        &current,
        pointer_content.as_bytes(),
        "V3_REPAIR_CURRENT_FAILED",
    ) {
        if created_marker {
            let _ = fs::remove_file(&marker);
        }
        return Err(error);
    }
    created_paths.push(".vibehub/tasks/current".to_owned());

    Ok(V3RepairResult {
        status: "repaired".to_owned(),
        task_id: task_id.to_owned(),
        created_paths,
    })
}

pub fn inspect_v3_repair_candidates(
    project_root: impl AsRef<Path>,
) -> Result<Vec<V3RepairCandidate>, V3Error> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    let root = project_root.join(VIBEHUB_DIR);
    let status = inspect_project_layout(&project_root)?;
    let repairable = status.state == ProjectLayoutState::V3
        || (status.state == ProjectLayoutState::Conflict
            && status.schema_version.is_none()
            && root.join("legacy-v2").is_dir());
    if !repairable {
        return Ok(Vec::new());
    }

    let tasks_root = root.join("tasks");
    reject_regular_directory(&tasks_root, "V3_REPAIR_TASKS_INVALID")?;
    let project_id = project_id(&project_root);
    let events_path = repair_events_path(&root, &project_id);
    if !events_path.exists() {
        return Ok(Vec::new());
    }
    reject_regular_file(&events_path, "V3_REPAIR_EVENTS_INVALID")?;
    let events = V3EventStore::open(&project_root)?.load_project(&project_id)?;
    let mut candidates = Vec::new();
    let entries =
        fs::read_dir(&tasks_root).map_err(|error| io_error("V3_REPAIR_TASKS_INVALID", error))?;
    for entry in entries {
        let entry = entry.map_err(|error| io_error("V3_REPAIR_TASKS_INVALID", error))?;
        let metadata = entry
            .file_type()
            .map_err(|error| io_error("V3_REPAIR_TASKS_INVALID", error))?;
        if !metadata.is_dir() {
            continue;
        }
        let task = match read_repair_task(&entry.path().join("task.yaml")) {
            Ok(task) => task,
            Err(_) => continue,
        };
        if !events.iter().any(|event| event.task_id.0 == task.task_id) {
            continue;
        }
        let lifecycle = fold_task_lifecycle(&task.task_id, &events);
        candidates.push(V3RepairCandidate {
            task_id: task.task_id,
            title: task.title,
            state: lifecycle.state,
        });
    }
    candidates.sort_by(|left, right| {
        let left_terminal = matches!(left.state.as_str(), "completed" | "cancelled" | "failed");
        let right_terminal = matches!(right.state.as_str(), "completed" | "cancelled" | "failed");
        left_terminal
            .cmp(&right_terminal)
            .then_with(|| left.task_id.cmp(&right.task_id))
    });
    Ok(candidates)
}

fn create_v3_root(project_root: &Path) -> Result<V3BootstrapResult, V3Error> {
    let root = project_root.join(VIBEHUB_DIR);
    fs::create_dir(&root).map_err(|error| io_error("V3_INIT_CREATE_FAILED", error))?;
    let result = (|| {
        for directory in V3_DIRS {
            fs::create_dir(root.join(directory))
                .map_err(|error| io_error("V3_INIT_CREATE_FAILED", error))?;
        }
        let marker = ProjectMarker {
            schema_version: V3_SCHEMA_VERSION,
            name: project_root
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("project")
                .to_owned(),
        };
        let content = serde_yaml::to_string(&marker).map_err(|error| {
            layout_error(
                "V3_INIT_MARKER_FAILED",
                format!("failed to encode marker: {error}"),
            )
        })?;
        fs::write(root.join("project.yaml"), content)
            .map_err(|error| io_error("V3_INIT_MARKER_FAILED", error))?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_dir_all(&root);
        return Err(error);
    }

    let mut created_paths = vec![".vibehub/project.yaml".to_owned()];
    created_paths.extend(V3_DIRS.iter().map(|path| format!(".vibehub/{path}")));
    Ok(V3BootstrapResult {
        status: "initialized".to_owned(),
        archived_legacy_v2: false,
        created_paths,
    })
}

fn validate_repair_task_id(task_id: &str) -> Result<(), V3Error> {
    if task_id.is_empty()
        || task_id.len() > 128
        || task_id.contains("..")
        || !task_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-'))
    {
        return Err(layout_error(
            "V3_REPAIR_TASK_ID_INVALID",
            "repair task identifier is unsafe",
        ));
    }
    Ok(())
}

fn project_id(project_root: &Path) -> String {
    let name = project_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("project")
        .to_ascii_lowercase()
        .replace(|character: char| !character.is_ascii_alphanumeric(), "-");
    format!("project.{name}")
}

fn repair_events_path(root: &Path, project_id: &str) -> PathBuf {
    root.join("v3/projects")
        .join(project_id.replace(['/', '\\'], "_"))
        .join("events.jsonl")
}

fn read_repair_task(path: &Path) -> Result<RepairTaskDocument, V3Error> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| io_error("V3_REPAIR_TASK_INVALID", error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > 1024 * 1024 {
        return Err(layout_error(
            "V3_REPAIR_TASK_INVALID",
            "repair task document must be a regular bounded file",
        ));
    }
    let content =
        fs::read_to_string(path).map_err(|error| io_error("V3_REPAIR_TASK_INVALID", error))?;
    serde_yaml::from_str(&content)
        .map_err(|error| layout_error("V3_REPAIR_TASK_INVALID", error.to_string()))
}

fn reject_regular_directory(path: &Path, code: &'static str) -> Result<(), V3Error> {
    let metadata = fs::symlink_metadata(path).map_err(|error| io_error(code, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(layout_error(code, "path must be a regular directory"));
    }
    Ok(())
}

fn reject_regular_file(path: &Path, code: &'static str) -> Result<(), V3Error> {
    let metadata = fs::symlink_metadata(path).map_err(|error| io_error(code, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(layout_error(code, "path must be a regular file"));
    }
    Ok(())
}

fn write_new_synced(path: &Path, content: &[u8], code: &'static str) -> Result<(), V3Error> {
    let parent = path
        .parent()
        .ok_or_else(|| layout_error(code, "repair path does not have a parent"))?;
    let temporary = parent.join(format!(".repair.{}.tmp", Uuid::new_v4()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| io_error(code, error))?;
        file.write_all(content)
            .and_then(|_| file.sync_all())
            .map_err(|error| io_error(code, error))?;
        if path.exists() {
            return Err(layout_error(code, "repair target already exists"));
        }
        fs::rename(&temporary, path).map_err(|error| io_error(code, error))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn rollback_staged_root(root: &Path, staging: &Path) -> Result<(), V3Error> {
    if root.exists() {
        fs::remove_dir_all(root)
            .map_err(|error| io_error("V3_MIGRATION_ROLLBACK_FAILED", error))?;
    }
    fs::rename(staging, root).map_err(|error| io_error("V3_MIGRATION_ROLLBACK_FAILED", error))
}

fn read_schema_version(root: &Path) -> Option<u32> {
    let marker = root.join("project.yaml");
    let metadata = fs::symlink_metadata(&marker).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > 64 * 1024 {
        return None;
    }
    let content = fs::read_to_string(marker).ok()?;
    serde_yaml::from_str::<serde_yaml::Value>(&content)
        .ok()?
        .get("schema_version")?
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
}

fn canonical_project_root(project_root: &Path) -> Result<PathBuf, V3Error> {
    project_root
        .canonicalize()
        .map_err(|error| io_error("V3_PROJECT_ROOT_NOT_FOUND", error))
}

fn reject_symlink(path: &Path, code: &'static str) -> Result<(), V3Error> {
    let metadata = fs::symlink_metadata(path).map_err(|error| io_error(code, error))?;
    if metadata.file_type().is_symlink() {
        return Err(layout_error(code, "symbolic links are not allowed"));
    }
    Ok(())
}

fn status(
    state: ProjectLayoutState,
    schema_version: Option<u32>,
    message: &str,
    recommended_action: Option<&str>,
) -> ProjectLayoutStatus {
    ProjectLayoutStatus {
        state,
        schema_version,
        message: message.to_owned(),
        recommended_action: recommended_action.map(str::to_owned),
    }
}

fn layout_error(code: &str, message: impl Into<String>) -> V3Error {
    V3Error::new(code, V3ErrorCategory::Validation, false, message)
}

fn io_error(code: &'static str, error: std::io::Error) -> V3Error {
    V3Error::new(code, V3ErrorCategory::Internal, false, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_project() -> PathBuf {
        let root = std::env::temp_dir().join(format!("vibehub-v3-bootstrap-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        root
    }

    #[test]
    fn initializes_absent_project_and_is_idempotent() {
        let project = temp_project();
        let first = initialize_v3(&project).unwrap();
        assert_eq!(first.status, "initialized");
        assert_eq!(
            inspect_project_layout(&project).unwrap().state,
            ProjectLayoutState::V3
        );
        let second = initialize_v3(&project).unwrap();
        assert_eq!(second.status, "already_initialized");
        for directory in V3_DIRS {
            assert!(project.join(VIBEHUB_DIR).join(directory).is_dir());
        }
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn migrates_v2_root_without_converting_legacy_files() {
        let project = temp_project();
        let root = project.join(VIBEHUB_DIR);
        fs::create_dir(&root).unwrap();
        fs::write(root.join("project.yaml"), "schema_version: 1\nname: old\n").unwrap();
        fs::write(root.join("history.txt"), "unchanged").unwrap();

        let result = migrate_v2_to_v3(&project).unwrap();
        assert_eq!(result.status, "migrated");
        assert_eq!(
            inspect_project_layout(&project).unwrap().state,
            ProjectLayoutState::V3
        );
        assert_eq!(
            fs::read_to_string(project.join(".vibehub/legacy-v2/history.txt")).unwrap(),
            "unchanged"
        );
        assert_eq!(
            migrate_v2_to_v3(&project).unwrap().status,
            "already_migrated"
        );
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn refuses_archive_conflict_and_reports_v2_upgrade() {
        let project = temp_project();
        let root = project.join(VIBEHUB_DIR);
        fs::create_dir_all(root.join("legacy-v2")).unwrap();
        let status = inspect_project_layout(&project).unwrap();
        assert_eq!(status.state, ProjectLayoutState::Conflict);
        assert!(migrate_v2_to_v3(&project).is_err());
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn repairs_release_baseline_marker_and_current_pointer_loss() {
        use crate::v3::{create_v3_task, V3TaskCreateRequest, V3ViewRepository};

        let project = temp_project();
        initialize_v3(&project).unwrap();
        fs::create_dir(project.join(".vibehub/legacy-v2")).unwrap();
        let task = create_v3_task(
            &project,
            V3TaskCreateRequest {
                title: "Repair V3 bootstrap".to_owned(),
                intent: "Recover valid V3 runtime state".to_owned(),
                acceptance_criteria: vec!["MCP can resolve current task".to_owned()],
            },
        )
        .unwrap();
        fs::remove_file(project.join(".vibehub/project.yaml")).unwrap();
        fs::remove_file(project.join(".vibehub/tasks/current")).unwrap();

        assert_eq!(
            inspect_project_layout(&project).unwrap().state,
            ProjectLayoutState::Conflict
        );
        let repaired = repair_v3_layout(&project, &task.task_id).unwrap();
        assert_eq!(repaired.status, "repaired");
        assert_eq!(
            inspect_project_layout(&project).unwrap().state,
            ProjectLayoutState::V3
        );
        assert_eq!(
            V3ViewRepository::open(&project)
                .unwrap()
                .current_task_id()
                .unwrap(),
            task.task_id
        );
        assert!(project.join(".vibehub/legacy-v2").is_dir());
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn repair_refuses_conflict_without_v3_event_store() {
        let project = temp_project();
        let root = project.join(".vibehub");
        fs::create_dir_all(root.join("legacy-v2")).unwrap();
        fs::create_dir_all(root.join("tasks/task.fake")).unwrap();
        fs::write(
            root.join("tasks/task.fake/task.yaml"),
            "task_id: task.fake\n",
        )
        .unwrap();

        let error = repair_v3_layout(&project, "task.fake").unwrap_err();
        assert_eq!(error.code, "V3_REPAIR_EVENTS_INVALID");
        assert!(!root.join("project.yaml").exists());
        assert!(!root.join("tasks/current").exists());
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn repair_candidates_only_include_tasks_with_v3_events() {
        use crate::v3::{create_v3_task, V3TaskCreateRequest};

        let project = temp_project();
        initialize_v3(&project).unwrap();
        fs::create_dir(project.join(".vibehub/legacy-v2")).unwrap();
        let task = create_v3_task(
            &project,
            V3TaskCreateRequest {
                title: "Repair candidate".to_owned(),
                intent: "Expose a safe recovery choice".to_owned(),
                acceptance_criteria: vec!["Candidate is visible".to_owned()],
            },
        )
        .unwrap();
        let eventless = project.join(".vibehub/tasks/task.eventless");
        fs::create_dir(&eventless).unwrap();
        fs::write(
            eventless.join("task.yaml"),
            "task_id: task.eventless\ntitle: Eventless\n",
        )
        .unwrap();
        fs::remove_file(project.join(".vibehub/project.yaml")).unwrap();
        fs::remove_file(project.join(".vibehub/tasks/current")).unwrap();

        let candidates = inspect_v3_repair_candidates(&project).unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].task_id, task.task_id);
        assert_eq!(candidates[0].title, "Repair candidate");
        let error = repair_v3_layout(&project, "task.eventless").unwrap_err();
        assert_eq!(error.code, "V3_REPAIR_TASK_EVENTS_MISSING");
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn recovers_interrupted_migration_forward_or_backward() {
        let project = temp_project();
        let staging = project.join(STAGING_DIR);
        fs::create_dir(&staging).unwrap();
        fs::write(staging.join("old.txt"), "legacy").unwrap();
        assert_eq!(
            recover_interrupted_migration(&project).unwrap().status,
            "rolled_back_to_v2"
        );

        fs::rename(project.join(VIBEHUB_DIR), &staging).unwrap();
        create_v3_root(&project).unwrap();
        assert_eq!(
            recover_interrupted_migration(&project).unwrap().status,
            "recovered_forward"
        );
        assert!(project.join(".vibehub/legacy-v2/old.txt").is_file());
        fs::remove_dir_all(project).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn refuses_symlink_vibehub_root() {
        use std::os::unix::fs::symlink;
        let project = temp_project();
        let outside = temp_project();
        symlink(&outside, project.join(VIBEHUB_DIR)).unwrap();
        let error = inspect_project_layout(&project).unwrap_err();
        assert_eq!(error.code, "V3_ROOT_SYMLINK");
        fs::remove_file(project.join(VIBEHUB_DIR)).unwrap();
        fs::remove_dir_all(project).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }
}

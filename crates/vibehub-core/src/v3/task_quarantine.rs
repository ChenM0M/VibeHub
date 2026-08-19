use super::{inspect_project_layout, ProjectLayoutState, V3Error, V3ErrorCategory};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const MAX_TASK_METADATA_BYTES: u64 = 1024 * 1024;
const QUARANTINE_DIRECTORY: &str = "quarantine";
const QUARANTINE_TASKS_DIRECTORY: &str = "tasks";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V3TaskQuarantineResult {
    pub status: String,
    pub task_id: String,
    pub source_path: String,
    pub archive_path: String,
    pub audit_path: String,
}

#[derive(Debug, Serialize)]
struct QuarantineAudit<'a> {
    schema_version: u32,
    kind: &'a str,
    task_id: &'a str,
    source_path: String,
    archive_path: String,
    quarantined_at: String,
    reason_code: &'a str,
    reason: &'a str,
    action: &'a str,
}

#[derive(Debug, Deserialize)]
struct V3TaskMetadata {
    task_id: String,
    title: String,
    intent: String,
    phase: String,
    phase_status: String,
    #[serde(default)]
    acceptance_criteria: Vec<String>,
    #[serde(default)]
    dependencies: Vec<String>,
    #[serde(default)]
    workflow_profile: String,
}

struct TaskMetadataIssue {
    code: &'static str,
    message: String,
}

/// Moves an invalid task directory out of `.vibehub/tasks` without deleting any
/// of its files. The staged audit and task directory are committed together by
/// renaming the containing directory inside `.vibehub/quarantine/tasks`.
pub fn quarantine_v3_task(
    project_root: impl AsRef<Path>,
    task_id: &str,
) -> Result<V3TaskQuarantineResult, V3Error> {
    let project_root = project_root
        .as_ref()
        .canonicalize()
        .map_err(|error| io_error("V3_TASK_QUARANTINE_PROJECT_NOT_FOUND", error))?;
    if inspect_project_layout(&project_root)?.state != ProjectLayoutState::V3 {
        return Err(validation(
            "V3_TASK_QUARANTINE_REQUIRES_V3",
            "task quarantine is only available in a schema_version: 3 project",
        ));
    }
    validate_task_id(task_id)?;

    let root = project_root.join(".vibehub");
    let source = root.join("tasks").join(task_id);
    require_regular_directory(&source, "V3_TASK_QUARANTINE_TASK_NOT_FOUND")?;
    let task_yaml = source.join("task.yaml");
    let issue = task_metadata_issue(&task_yaml, task_id)?;
    let Some(issue) = issue else {
        return Err(validation(
            "V3_TASK_QUARANTINE_REQUIRES_INVALID_METADATA",
            "refusing to quarantine a task whose V3 metadata is valid",
        ));
    };

    let quarantine_root = ensure_quarantine_root(&root)?;
    let archive_name = format!("{}-{}", task_id, Uuid::new_v4().simple());
    let staging = quarantine_root.join(format!(".staging-{archive_name}"));
    let archive = quarantine_root.join(&archive_name);
    fs::create_dir(&staging)
        .map_err(|error| io_error("V3_TASK_QUARANTINE_STAGE_CREATE_FAILED", error))?;

    let source_path = project_relative(&project_root, &source);
    let archive_path = project_relative(&project_root, &archive.join("task"));
    let audit_path = project_relative(&project_root, &archive.join("audit.json"));
    let audit = QuarantineAudit {
        schema_version: 1,
        kind: "v3_task_quarantine",
        task_id,
        source_path: source_path.clone(),
        archive_path: archive_path.clone(),
        quarantined_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        reason_code: issue.code,
        reason: &issue.message,
        action: "quarantined",
    };
    let audit_content = serde_json::to_vec_pretty(&audit)
        .map_err(|error| validation("V3_TASK_QUARANTINE_AUDIT_ENCODE_FAILED", error.to_string()))?;
    if let Err(error) = write_new_synced(
        &staging.join("audit.json"),
        &audit_content,
        "V3_TASK_QUARANTINE_AUDIT_WRITE_FAILED",
    ) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }

    if let Err(error) = fs::rename(&source, staging.join("task")) {
        let _ = fs::remove_dir_all(&staging);
        return Err(io_error("V3_TASK_QUARANTINE_MOVE_FAILED", error));
    }
    if let Err(error) = fs::rename(&staging, &archive) {
        return Err(io_error("V3_TASK_QUARANTINE_FINALIZE_FAILED", error)
            .with_detail("recovery_path", project_relative(&project_root, &staging)));
    }

    Ok(V3TaskQuarantineResult {
        status: "quarantined".to_owned(),
        task_id: task_id.to_owned(),
        source_path,
        archive_path,
        audit_path,
    })
}

fn task_metadata_issue(
    task_yaml: &Path,
    expected_task_id: &str,
) -> Result<Option<TaskMetadataIssue>, V3Error> {
    let metadata = match fs::symlink_metadata(task_yaml) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Some(TaskMetadataIssue {
                code: "V3_TASK_METADATA_INVALID",
                message: "task.yaml is missing".to_owned(),
            }));
        }
        Err(error) => return Err(io_error("V3_TASK_QUARANTINE_TASK_READ_FAILED", error)),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Ok(Some(TaskMetadataIssue {
            code: "V3_TASK_METADATA_INVALID",
            message: "task.yaml must be a regular file".to_owned(),
        }));
    }
    if metadata.len() > MAX_TASK_METADATA_BYTES {
        return Ok(Some(TaskMetadataIssue {
            code: "V3_TASK_METADATA_INVALID",
            message: format!("task.yaml exceeds {MAX_TASK_METADATA_BYTES} bytes"),
        }));
    }
    let content = fs::read_to_string(task_yaml)
        .map_err(|error| io_error("V3_TASK_QUARANTINE_TASK_READ_FAILED", error))?;
    let task: V3TaskMetadata = match serde_yaml::from_str(&content) {
        Ok(task) => task,
        Err(error) => {
            return Ok(Some(TaskMetadataIssue {
                code: "V3_TASK_METADATA_INVALID",
                message: limited_message(error.to_string()),
            }));
        }
    };
    if task.task_id != expected_task_id {
        return Ok(Some(TaskMetadataIssue {
            code: "V3_TASK_METADATA_INVALID",
            message: "task.yaml task_id does not match its task directory".to_owned(),
        }));
    }
    if [
        task.title.as_str(),
        task.intent.as_str(),
        task.phase.as_str(),
        task.phase_status.as_str(),
    ]
    .iter()
    .any(|field| field.trim().is_empty())
    {
        return Ok(Some(TaskMetadataIssue {
            code: "V3_TASK_METADATA_INVALID",
            message: "a required V3 task metadata field is empty".to_owned(),
        }));
    }

    // The V3 reader intentionally defaults these fields for historical V3
    // documents, so their absence alone is not a quarantine condition.
    let _ = (
        task.acceptance_criteria,
        task.dependencies,
        task.workflow_profile,
    );
    Ok(None)
}

fn ensure_quarantine_root(root: &Path) -> Result<PathBuf, V3Error> {
    require_regular_directory(root, "V3_TASK_QUARANTINE_ROOT_INVALID")?;
    let quarantine = root.join(QUARANTINE_DIRECTORY);
    ensure_regular_directory(&quarantine, "V3_TASK_QUARANTINE_ROOT_INVALID")?;
    let tasks = quarantine.join(QUARANTINE_TASKS_DIRECTORY);
    ensure_regular_directory(&tasks, "V3_TASK_QUARANTINE_ROOT_INVALID")?;
    Ok(tasks)
}

fn require_regular_directory(path: &Path, code: &'static str) -> Result<(), V3Error> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.file_type().is_symlink() && metadata.is_dir() => Ok(()),
        Ok(_) => Err(validation(code, "path must be a regular directory")),
        Err(error) => Err(io_error(code, error)),
    }
}

fn ensure_regular_directory(path: &Path, code: &'static str) -> Result<(), V3Error> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.file_type().is_symlink() && metadata.is_dir() => Ok(()),
        Ok(_) => Err(validation(code, "path must be a regular directory")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(path).map_err(|error| io_error(code, error))
        }
        Err(error) => Err(io_error(code, error)),
    }
}

fn write_new_synced(path: &Path, content: &[u8], code: &'static str) -> Result<(), V3Error> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| io_error(code, error))?;
    file.write_all(content)
        .and_then(|_| file.write_all(b"\n"))
        .and_then(|_| file.sync_all())
        .map_err(|error| io_error(code, error))
}

fn project_relative(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn validate_task_id(task_id: &str) -> Result<(), V3Error> {
    if task_id.is_empty()
        || task_id.len() > 128
        || task_id.contains("..")
        || !task_id.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_')
        })
    {
        return Err(validation(
            "V3_TASK_QUARANTINE_TASK_ID_INVALID",
            "task identifier is unsafe",
        ));
    }
    Ok(())
}

fn limited_message(message: String) -> String {
    const MAX_CHARS: usize = 512;
    if message.chars().count() <= MAX_CHARS {
        message
    } else {
        format!("{}…", message.chars().take(MAX_CHARS).collect::<String>())
    }
}

fn validation(code: &str, message: impl Into<String>) -> V3Error {
    V3Error::new(code, V3ErrorCategory::Validation, false, message)
}

fn io_error(code: &'static str, error: std::io::Error) -> V3Error {
    let category = if error.kind() == std::io::ErrorKind::NotFound {
        V3ErrorCategory::NotFound
    } else {
        V3ErrorCategory::Internal
    };
    V3Error::new(code, category, false, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v3::{create_v3_task, initialize_v3, V3TaskCreateRequest, V3ViewRepository};
    use serde_json::Value;

    fn temp_project() -> PathBuf {
        let project =
            std::env::temp_dir().join(format!("vibehub-v3-quarantine-{}", Uuid::new_v4()));
        fs::create_dir(&project).unwrap();
        project
    }

    #[test]
    fn quarantines_invalid_task_without_deleting_artifacts_and_restores_views() {
        let project = temp_project();
        initialize_v3(&project).unwrap();
        let valid = create_v3_task(
            &project,
            V3TaskCreateRequest {
                title: "Valid V3 task".to_owned(),
                intent: "Remain visible after quarantine".to_owned(),
                acceptance_criteria: vec!["Views remain usable".to_owned()],
                workflow_profile: "standard".to_owned(),
                trigger_context: Default::default(),
                profile_override: None,
                initial_plan: Vec::new(),
                preflight: false,
            },
        )
        .unwrap();
        let legacy = project.join(".vibehub/tasks/T-20260718191720-ffb8c89a");
        fs::create_dir(&legacy).unwrap();
        let legacy_yaml = "schema_version: 1\nkind: vibehub_task\ntask_id: T-20260718191720-ffb8c89a\ntitle: Legacy task\nmode: evidence_drive\nphase: align\nphase_status: active\n";
        fs::write(legacy.join("task.yaml"), legacy_yaml).unwrap();
        fs::write(legacy.join("artifact.md"), "preserve me").unwrap();

        let result = quarantine_v3_task(&project, "T-20260718191720-ffb8c89a").unwrap();

        assert_eq!(result.status, "quarantined");
        assert!(!legacy.exists());
        assert_eq!(
            fs::read_to_string(project.join(&result.archive_path).join("task.yaml")).unwrap(),
            legacy_yaml
        );
        assert_eq!(
            fs::read_to_string(project.join(&result.archive_path).join("artifact.md")).unwrap(),
            "preserve me"
        );
        let audit: Value =
            serde_json::from_str(&fs::read_to_string(project.join(&result.audit_path)).unwrap())
                .unwrap();
        assert_eq!(audit["task_id"], "T-20260718191720-ffb8c89a");
        assert_eq!(audit["action"], "quarantined");
        assert_eq!(
            V3ViewRepository::open(&project)
                .unwrap()
                .load_bundle(&valid.task_id)
                .unwrap()
                .project_overview["active_tasks"]
                .as_array()
                .unwrap()
                .len(),
            1
        );

        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn refuses_to_quarantine_valid_v3_task_metadata() {
        let project = temp_project();
        initialize_v3(&project).unwrap();
        let valid = create_v3_task(
            &project,
            V3TaskCreateRequest {
                title: "Valid V3 task".to_owned(),
                intent: "Must not be quarantined".to_owned(),
                acceptance_criteria: vec!["Metadata is valid".to_owned()],
                workflow_profile: "standard".to_owned(),
                trigger_context: Default::default(),
                profile_override: None,
                initial_plan: Vec::new(),
                preflight: false,
            },
        )
        .unwrap();

        let error = quarantine_v3_task(&project, &valid.task_id).unwrap_err();
        assert_eq!(error.code, "V3_TASK_QUARANTINE_REQUIRES_INVALID_METADATA");
        assert!(project.join(".vibehub/tasks").join(valid.task_id).is_dir());

        fs::remove_dir_all(project).unwrap();
    }
}

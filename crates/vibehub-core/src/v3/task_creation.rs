use super::application::canonical_criterion_id;
use super::lifecycle::command;
use super::{
    inspect_project_layout, resolve_policy, EffectiveExecutionPolicy, EvidenceGrade,
    ProfileOverride, ProjectLayoutState, TriggerContext, V3ApplicationService, V3Error,
    V3ErrorCategory,
};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use uuid::Uuid;

const MAX_FIELD_LENGTH: usize = 4096;
const MAX_CRITERIA: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V3TaskCreateRequest {
    pub title: String,
    pub intent: String,
    pub acceptance_criteria: Vec<String>,
    #[serde(default = "default_workflow_profile")]
    pub workflow_profile: String,
    #[serde(default)]
    pub trigger_context: TriggerContext,
    #[serde(default)]
    pub profile_override: Option<ProfileOverride>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V3TaskCreateResult {
    pub status: String,
    pub task_id: String,
    pub task_path: String,
    pub current_pointer_path: String,
    pub initial_node_id: Option<String>,
    pub lifecycle_version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct TaskDocument {
    task_id: String,
    title: String,
    intent: String,
    phase: String,
    phase_status: String,
    acceptance_criteria: Vec<String>,
    dependencies: Vec<String>,
    #[serde(default = "default_workflow_profile")]
    workflow_profile: String,
    #[serde(default)]
    recommended_profile: String,
    #[serde(default)]
    effective_profile: String,
    #[serde(default)]
    execution_policy: Option<EffectiveExecutionPolicy>,
    #[serde(default)]
    policy_version: u32,
    #[serde(default)]
    enforcement_epoch: String,
    #[serde(default)]
    trigger_reasons: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CurrentTaskPointer<'a> {
    schema_version: u32,
    kind: &'a str,
    task_id: &'a str,
    path: String,
    updated_at: String,
    updated_by: &'a str,
}

pub fn create_v3_task(
    project_root: impl AsRef<Path>,
    request: V3TaskCreateRequest,
) -> Result<V3TaskCreateResult, V3Error> {
    let project_root = project_root
        .as_ref()
        .canonicalize()
        .map_err(|error| task_error("V3_PROJECT_ROOT_NOT_FOUND", error.to_string()))?;
    if inspect_project_layout(&project_root)?.state != ProjectLayoutState::V3 {
        return Err(task_error(
            "V3_TASK_CREATE_REQUIRES_V3",
            "tasks can only be created in an initialized V3 project",
        ));
    }

    let request = validate_request(request)?;
    let task_id = stable_task_id(&request);
    let initial_node_id = stable_initial_node_id(&task_id);
    validate_task_id(&task_id)?;
    let tasks_root = project_root.join(".vibehub/tasks");
    ensure_safe_tasks_root(&tasks_root)?;

    let task_dir = tasks_root.join(&task_id);
    let policy = resolve_policy(
        &request.title,
        &request.intent,
        request.acceptance_criteria.len(),
        &request.workflow_profile,
        &request.trigger_context,
        request.profile_override.clone(),
    )?;
    let document = TaskDocument {
        task_id: task_id.clone(),
        title: request.title,
        intent: request.intent,
        phase: if request.workflow_profile == "lightweight" {
            "execute"
        } else {
            "plan"
        }
        .to_owned(),
        phase_status: "active".to_owned(),
        acceptance_criteria: request.acceptance_criteria,
        dependencies: Vec::new(),
        workflow_profile: policy.effective_profile.clone(),
        recommended_profile: policy.recommended_profile.clone(),
        effective_profile: policy.effective_profile.clone(),
        policy_version: policy.policy_version,
        enforcement_epoch: policy.enforcement_epoch.clone(),
        trigger_reasons: policy.trigger_reasons.clone(),
        execution_policy: Some(policy.clone()),
    };
    let task_yaml = serde_yaml::to_string(&document)
        .map_err(|error| task_error("V3_TASK_ENCODE_FAILED", error.to_string()))?;

    let created = match fs::create_dir(&task_dir) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let metadata = fs::symlink_metadata(&task_dir)
                .map_err(|error| io_error("V3_TASK_PATH_INVALID", error))?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(task_error(
                    "V3_TASK_PATH_INVALID",
                    "existing task path is not a regular directory",
                ));
            }
            let existing = fs::read_to_string(task_dir.join("task.yaml"))
                .map_err(|error| io_error("V3_TASK_ALREADY_EXISTS", error))?;
            let existing: TaskDocument = serde_yaml::from_str(&existing)
                .map_err(|error| task_error("V3_TASK_ALREADY_EXISTS", error.to_string()))?;
            if existing != document {
                return Err(task_error(
                    "V3_TASK_ALREADY_EXISTS",
                    "the generated task identifier is already used by different task metadata",
                ));
            }
            false
        }
        Err(error) => return Err(io_error("V3_TASK_CREATE_FAILED", error)),
    };

    let pending_marker = task_dir.join(".creation.pending");
    if created {
        if let Err(error) = write_new_atomic(&task_dir.join("task.yaml"), task_yaml.as_bytes()) {
            let _ = fs::remove_dir(&task_dir);
            return Err(error);
        }
        if let Err(error) = write_new_atomic(&pending_marker, task_id.as_bytes()) {
            let _ = fs::remove_file(task_dir.join("task.yaml"));
            let _ = fs::remove_dir(&task_dir);
            return Err(error);
        }
    }

    let application = V3ApplicationService::open(&project_root)?;
    let project_id = project_id(&project_root);
    let existing_version = application.task_lifecycle(&project_id, &task_id)?.version;
    let lifecycle_version = if pending_marker.is_file() || existing_version > 0 {
        append_creation_events(
            &application,
            &project_id,
            &task_id,
            &initial_node_id,
            &document.title,
            &document.intent,
            &document.acceptance_criteria,
            &document.workflow_profile,
            &policy,
        )?
    } else {
        // Existing eventless tasks predate lifecycle recording. They remain read-only;
        // only a durable pending marker authorizes interrupted-create recovery.
        0
    };

    if let Err(error) = write_current_pointer(&tasks_root, &task_id) {
        return Err(error);
    }
    if pending_marker.exists() {
        fs::remove_file(&pending_marker)
            .map_err(|error| io_error("V3_TASK_RECOVERY_MARKER_REMOVE_FAILED", error))?;
    }

    Ok(V3TaskCreateResult {
        status: if created { "created" } else { "already_exists" }.to_owned(),
        task_id: task_id.clone(),
        task_path: format!(".vibehub/tasks/{task_id}/task.yaml"),
        current_pointer_path: ".vibehub/tasks/current".to_owned(),
        initial_node_id: (document.workflow_profile != "lightweight").then_some(initial_node_id),
        lifecycle_version,
    })
}

fn append_creation_events(
    application: &V3ApplicationService,
    project_id: &str,
    task_id: &str,
    initial_node_id: &str,
    title: &str,
    intent: &str,
    acceptance_criteria: &[String],
    workflow_profile: &str,
    policy: &EffectiveExecutionPolicy,
) -> Result<u64, V3Error> {
    let mut created_command = command(
        "task.created",
        &project_id,
        task_id,
        0,
        &format!("create.{task_id}.task"),
        json!({"title": title, "intent": intent, "workflow_profile": workflow_profile, "recommended_profile": policy.recommended_profile, "effective_profile": policy.effective_profile, "execution_policy": policy, "policy_version": policy.policy_version, "enforcement_epoch": policy.enforcement_epoch, "trigger_reasons": policy.trigger_reasons}),
    );
    created_command.actor = "vibehub".to_owned();
    created_command.evidence_grade = Some(EvidenceGrade::HardObserved);
    application.lifecycle_command(created_command)?;

    if workflow_profile != "lightweight" {
        let mut node_command = command(
            "plan.node_added",
            &project_id,
            task_id,
            1,
            &format!("create.{task_id}.initial-node"),
            json!({
                "node_id": initial_node_id,
                "title": title,
                "goal": intent,
                "scope": [],
                "dependencies": []
            }),
        );
        node_command.actor = "vibehub".to_owned();
        node_command.evidence_grade = Some(EvidenceGrade::HardObserved);
        application.lifecycle_command(node_command)?;
    }

    for (index, title) in acceptance_criteria.iter().enumerate() {
        let criterion_id = canonical_criterion_id(task_id, index);
        let lifecycle = application.task_lifecycle(project_id, task_id)?;
        if lifecycle.criteria.contains_key(&criterion_id) {
            continue;
        }
        let expected_version = lifecycle.version;
        let mut criterion_command = command(
            "criterion.accepted",
            project_id,
            task_id,
            expected_version,
            &format!("create.{task_id}.criterion.c{:02}", index + 1),
            json!({
                "criterion_id": criterion_id,
                "title": title,
                "required": true
            }),
        );
        criterion_command.actor = "vibehub".to_owned();
        criterion_command.evidence_grade = Some(EvidenceGrade::HardObserved);
        application.lifecycle_command(criterion_command)?;
    }
    Ok(application.task_lifecycle(&project_id, task_id)?.version)
}

fn default_workflow_profile() -> String {
    "standard".to_owned()
}

fn validate_request(request: V3TaskCreateRequest) -> Result<V3TaskCreateRequest, V3Error> {
    let title = clean_field("title", request.title)?;
    let intent = clean_field("intent", request.intent)?;
    if request.acceptance_criteria.is_empty() || request.acceptance_criteria.len() > MAX_CRITERIA {
        return Err(task_error(
            "V3_TASK_VALIDATION_ERROR",
            format!("acceptance_criteria must contain 1 to {MAX_CRITERIA} items"),
        ));
    }
    let acceptance_criteria = request
        .acceptance_criteria
        .into_iter()
        .enumerate()
        .map(|(index, value)| clean_field(&format!("acceptance_criteria[{index}]"), value))
        .collect::<Result<Vec<_>, _>>()?;
    if !matches!(
        request.workflow_profile.as_str(),
        "lightweight" | "standard" | "full"
    ) {
        return Err(task_error(
            "V3_TASK_VALIDATION_ERROR",
            "workflow_profile must be lightweight, standard, or full",
        ));
    }
    Ok(V3TaskCreateRequest {
        title,
        intent,
        acceptance_criteria,
        workflow_profile: request.workflow_profile,
        trigger_context: request.trigger_context,
        profile_override: request.profile_override,
    })
}

fn clean_field(name: &str, value: String) -> Result<String, V3Error> {
    let value = value.trim().to_owned();
    if value.is_empty() || value.len() > MAX_FIELD_LENGTH || value.contains('\0') {
        return Err(task_error(
            "V3_TASK_VALIDATION_ERROR",
            format!("{name} must contain 1 to {MAX_FIELD_LENGTH} valid UTF-8 bytes"),
        ));
    }
    Ok(value)
}

fn stable_task_id(request: &V3TaskCreateRequest) -> String {
    let slug: String = request
        .title
        .chars()
        .flat_map(char::to_lowercase)
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(40)
        .collect();
    let slug = if slug.is_empty() { "task" } else { &slug };
    let mut hasher = Sha256::new();
    hasher.update(request.title.as_bytes());
    hasher.update([0]);
    hasher.update(request.intent.as_bytes());
    for criterion in &request.acceptance_criteria {
        hasher.update([0]);
        hasher.update(criterion.as_bytes());
    }
    let digest = format!("{:x}", hasher.finalize());
    format!("task.{slug}.{}", &digest[..12])
}

fn stable_initial_node_id(task_id: &str) -> String {
    format!("node.{task_id}.initial")
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

fn validate_task_id(task_id: &str) -> Result<(), V3Error> {
    if task_id.len() > 96
        || !task_id.starts_with("task.")
        || !task_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-'))
        || task_id.contains("..")
    {
        return Err(task_error(
            "V3_TASK_ID_INVALID",
            "generated task identifier is unsafe",
        ));
    }
    Ok(())
}

fn ensure_safe_tasks_root(tasks_root: &Path) -> Result<(), V3Error> {
    if tasks_root.exists() {
        let metadata = fs::symlink_metadata(tasks_root)
            .map_err(|error| io_error("V3_TASKS_ROOT_INVALID", error))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(task_error(
                "V3_TASKS_ROOT_INVALID",
                ".vibehub/tasks must be a regular directory",
            ));
        }
    } else {
        fs::create_dir(tasks_root)
            .map_err(|error| io_error("V3_TASKS_ROOT_CREATE_FAILED", error))?;
    }
    Ok(())
}

fn write_new_atomic(path: &Path, content: &[u8]) -> Result<(), V3Error> {
    let parent = path
        .parent()
        .ok_or_else(|| task_error("V3_TASK_WRITE_FAILED", "task path does not have a parent"))?;
    let temporary = parent.join(format!(".task.yaml.{}.tmp", Uuid::new_v4()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| io_error("V3_TASK_WRITE_FAILED", error))?;
        file.write_all(content)
            .and_then(|_| file.sync_all())
            .map_err(|error| io_error("V3_TASK_WRITE_FAILED", error))?;
        fs::rename(&temporary, path).map_err(|error| io_error("V3_TASK_WRITE_FAILED", error))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn write_current_pointer(tasks_root: &Path, task_id: &str) -> Result<(), V3Error> {
    let path = tasks_root.join("current");
    if path.exists() {
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| io_error("V3_CURRENT_TASK_INVALID", error))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(task_error(
                "V3_CURRENT_TASK_INVALID",
                "current task pointer must be a regular file",
            ));
        }
    }
    let pointer = CurrentTaskPointer {
        schema_version: 1,
        kind: "current_task_pointer",
        task_id,
        path: format!(".vibehub/tasks/{task_id}"),
        updated_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        updated_by: "vibehub",
    };
    let content = serde_yaml::to_string(&pointer)
        .map_err(|error| task_error("V3_CURRENT_TASK_WRITE_FAILED", error.to_string()))?;
    let temporary = tasks_root.join(format!(".current.{}.tmp", Uuid::new_v4()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| io_error("V3_CURRENT_TASK_WRITE_FAILED", error))?;
        file.write_all(content.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|error| io_error("V3_CURRENT_TASK_WRITE_FAILED", error))?;
        fs::rename(&temporary, &path)
            .map_err(|error| io_error("V3_CURRENT_TASK_WRITE_FAILED", error))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn task_error(code: &str, message: impl Into<String>) -> V3Error {
    V3Error::new(code, V3ErrorCategory::Validation, false, message)
}

fn io_error(code: &str, error: std::io::Error) -> V3Error {
    V3Error::new(code, V3ErrorCategory::Internal, false, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v3::{initialize_v3, V3EventStore, V3ViewRepository};
    use std::path::PathBuf;

    fn temp_project() -> PathBuf {
        let root = std::env::temp_dir().join(format!("vibehub-v3-task-create-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        root
    }

    fn request() -> V3TaskCreateRequest {
        V3TaskCreateRequest {
            title: "Ship safe task creation".to_owned(),
            intent: "Create a bounded V3 task without protocol state".to_owned(),
            acceptance_criteria: vec!["Task is readable from production views".to_owned()],
            workflow_profile: "standard".to_owned(),
            trigger_context: Default::default(),
            profile_override: None,
        }
    }

    #[test]
    fn creates_only_v3_metadata_and_current_pointer() {
        let project = temp_project();
        initialize_v3(&project).unwrap();
        let result = create_v3_task(&project, request()).unwrap();
        assert_eq!(result.status, "created");
        assert!(project.join(&result.task_path).is_file());
        assert_eq!(result.lifecycle_version, 3);
        assert_eq!(
            result.initial_node_id.as_deref(),
            Some(format!("node.{}.initial", result.task_id).as_str())
        );
        let repository = V3ViewRepository::open(&project).unwrap();
        let bundle = repository.load_bundle(&result.task_id).unwrap();
        let nodes = bundle.plan_graph["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0]["node_id"], result.initial_node_id.unwrap());
        assert_eq!(nodes[0]["title"], request().title);
        assert_eq!(nodes[0]["goal"], request().intent);
        assert_eq!(nodes[0]["scope"], json!([]));
        assert_eq!(bundle.plan_graph["plan_version"], 3);
        assert_eq!(bundle.plan_graph["scheduling_edges"], json!([]));
        let lifecycle = V3ApplicationService::open(&project)
            .unwrap()
            .task_lifecycle(&repository.project_id(), &result.task_id)
            .unwrap();
        let criterion_id = canonical_criterion_id(&result.task_id, 0);
        assert_eq!(
            lifecycle.criteria[&criterion_id].title,
            request().acceptance_criteria[0]
        );
        assert_eq!(
            lifecycle.criteria[&criterion_id].state,
            super::super::lifecycle::CriterionState::Accepted
        );
        assert_eq!(repository.current_task_id().unwrap(), result.task_id);
        assert!(!project.join(".vibehub/agent-view").exists());
        assert!(!project.join(".vibehub/runs").exists());
        assert!(!project.join(".vibehub/context").exists());
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn identical_request_is_idempotent() {
        let project = temp_project();
        initialize_v3(&project).unwrap();
        let first = create_v3_task(&project, request()).unwrap();
        let second = create_v3_task(&project, request()).unwrap();
        assert_eq!(first.task_id, second.task_id);
        assert_eq!(first.initial_node_id, second.initial_node_id);
        assert_eq!(second.lifecycle_version, 3);
        assert_eq!(second.status, "already_exists");
        let repository = V3ViewRepository::open(&project).unwrap();
        let events = V3EventStore::open(&project)
            .unwrap()
            .load_project(&repository.project_id())
            .unwrap();
        assert_eq!(events.len(), 3);
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn lightweight_creation_skips_task_graph_and_keeps_minimal_lifecycle() {
        let project = temp_project();
        initialize_v3(&project).unwrap();
        let mut lightweight = request();
        lightweight.workflow_profile = "lightweight".to_owned();
        let result = create_v3_task(&project, lightweight).unwrap();
        assert_eq!(result.lifecycle_version, 2);
        assert!(result.initial_node_id.is_none());
        let task_yaml = fs::read_to_string(project.join(&result.task_path)).unwrap();
        assert!(task_yaml.contains("phase: execute"));
        let repository = V3ViewRepository::open(&project).unwrap();
        let bundle = repository.load_bundle(&result.task_id).unwrap();
        assert_eq!(bundle.plan_graph["nodes"].as_array().unwrap().len(), 0);
        assert_eq!(bundle.plan_graph["workflow_profile"], "lightweight");
        assert_eq!(bundle.plan_graph["planning_required"], false);
        assert!(bundle.plan_graph["warnings"].as_array().unwrap().is_empty());
        assert_eq!(bundle.node_brief["workflow_profile"], "lightweight");
        assert_eq!(
            bundle.node_brief["execution_policy"]["milestone_policy"],
            "minimal"
        );
        assert_eq!(bundle.node_brief["budget"]["max_tokens"], 2000);
        let repository_id = repository.project_id();
        let error = V3ApplicationService::open(&project)
            .unwrap()
            .plan_add_node(crate::v3::PlanAddNodeCommand {
                identity: crate::v3::PlanCommandIdentity {
                    project_id: repository_id,
                    task_id: result.task_id.clone(),
                    actor: "test".to_owned(),
                    expected_version: result.lifecycle_version,
                    idempotency_key: "lightweight-plan-forbidden".to_owned(),
                },
                node_id: "node.forbidden".to_owned(),
                title: "Forbidden plan".to_owned(),
                goal: "Must not be added".to_owned(),
                scope: Vec::new(),
                dependencies: Vec::new(),
                criterion_ids: Vec::new(),
            })
            .unwrap_err();
        assert_eq!(error.code, "V3_LIGHTWEIGHT_PLAN_FORBIDDEN");
        assert_eq!(
            bundle.project_overview["active_tasks"][0]["workflow_profile"],
            "lightweight"
        );
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn historical_eventless_task_is_not_backfilled() {
        let project = temp_project();
        initialize_v3(&project).unwrap();
        let validated_request = validate_request(request()).unwrap();
        let task_id = stable_task_id(&validated_request);
        let task_dir = project.join(".vibehub/tasks").join(&task_id);
        fs::create_dir_all(&task_dir).unwrap();
        let policy = resolve_policy(
            &validated_request.title,
            &validated_request.intent,
            validated_request.acceptance_criteria.len(),
            &validated_request.workflow_profile,
            &validated_request.trigger_context,
            validated_request.profile_override.clone(),
        )
        .unwrap();
        let document = TaskDocument {
            task_id: task_id.clone(),
            title: validated_request.title,
            intent: validated_request.intent,
            phase: "plan".to_owned(),
            phase_status: "active".to_owned(),
            acceptance_criteria: validated_request.acceptance_criteria,
            dependencies: Vec::new(),
            workflow_profile: validated_request.workflow_profile,
            recommended_profile: policy.recommended_profile.clone(),
            effective_profile: policy.effective_profile.clone(),
            execution_policy: Some(policy.clone()),
            policy_version: policy.policy_version,
            enforcement_epoch: policy.enforcement_epoch.clone(),
            trigger_reasons: policy.trigger_reasons.clone(),
        };
        fs::write(
            task_dir.join("task.yaml"),
            serde_yaml::to_string(&document).unwrap(),
        )
        .unwrap();

        let result = create_v3_task(&project, request()).unwrap();
        assert_eq!(result.status, "already_exists");
        assert_eq!(result.lifecycle_version, 0);
        let repository = V3ViewRepository::open(&project).unwrap();
        assert!(V3EventStore::open(&project)
            .unwrap()
            .load_project(&repository.project_id())
            .unwrap()
            .is_empty());
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn rejects_invalid_input_and_non_v3_projects() {
        let project = temp_project();
        let error = create_v3_task(&project, request()).unwrap_err();
        assert_eq!(error.code, "V3_TASK_CREATE_REQUIRES_V3");
        initialize_v3(&project).unwrap();
        let mut invalid = request();
        invalid.acceptance_criteria = vec!["  ".to_owned()];
        let error = create_v3_task(&project, invalid).unwrap_err();
        assert_eq!(error.code, "V3_TASK_VALIDATION_ERROR");
        fs::remove_dir_all(project).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_tasks_root_without_writing_outside() {
        use std::os::unix::fs::symlink;
        let project = temp_project();
        let outside = temp_project();
        initialize_v3(&project).unwrap();
        symlink(&outside, project.join(".vibehub/tasks")).unwrap();
        let error = create_v3_task(&project, request()).unwrap_err();
        assert_eq!(error.code, "V3_TASKS_ROOT_INVALID");
        assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
        fs::remove_file(project.join(".vibehub/tasks")).unwrap();
        fs::remove_dir_all(project).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }
}

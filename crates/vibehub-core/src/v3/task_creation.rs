use super::application::canonical_criterion_id;
use super::lifecycle::command;
use super::{
    inspect_project_layout, project_id, resolve_policy, EffectiveExecutionPolicy, EvidenceGrade,
    PlanNodeOrigin, PlanNodeRole, ProfileOverride, ProjectLayoutState, TriggerContext,
    V3ApplicationService, V3Error, V3ErrorCategory, PLAN_NODE_ORIGIN_CONTRACT_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use uuid::Uuid;

const MAX_FIELD_LENGTH: usize = 4096;
const MAX_CRITERIA: usize = 100;
const MAX_INITIAL_PLAN_NODES: usize = 100;

/// A plan confirmed together with task creation. Dependencies and criteria use 1-based
/// positions so callers do not need to know the generated task id or criterion ids.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V3TaskCreateInitialPlanNode {
    #[serde(default)]
    pub node_id: Option<String>,
    pub title: String,
    pub goal: String,
    #[serde(default)]
    pub scope: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<usize>,
    #[serde(default)]
    pub criteria: Vec<usize>,
    #[serde(default)]
    pub role: Option<String>,
}

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
    /// When present, this confirmed plan replaces the administrative bootstrap node.
    #[serde(default)]
    pub initial_plan: Vec<V3TaskCreateInitialPlanNode>,
    /// When true, compute the deterministic task id and report whether an equivalent
    /// task already exists, WITHOUT writing task.yaml or appending lifecycle events.
    /// Agents should preflight before committing a create to avoid probe-generated junk.
    #[serde(default)]
    pub preflight: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V3TaskCreateResult {
    pub status: String,
    pub task_id: String,
    pub task_path: String,
    pub current_pointer_path: String,
    pub current_pointer_updated: bool,
    pub initial_node_id: Option<String>,
    pub lifecycle_version: u64,
    #[serde(default)]
    pub initial_plan_node_ids: Vec<String>,
    /// True when this result came from a preflight (no writes, no events).
    #[serde(default)]
    pub preflight: bool,
    /// For preflight: true if an equivalent task does not already exist (a create would write).
    #[serde(default)]
    pub would_create: bool,
    /// For preflight: the existing task_id when an equivalent task is already present.
    #[serde(default)]
    pub existing_task_id: Option<String>,
    /// For preflight: a short state summary of the existing task, e.g. "plan/active".
    #[serde(default)]
    pub existing_state: Option<String>,
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
    validate_task_id(&task_id)?;
    let tasks_root = project_root.join(".vibehub/tasks");
    ensure_safe_tasks_root(&tasks_root)?;

    let task_dir = tasks_root.join(&task_id);
    let initial_plan = request.initial_plan.clone();
    let plan_node_ids = initial_plan_node_ids(&task_id, &initial_plan)?;

    // Preflight: report what a create would do without writing files or events.
    // This lets an agent confirm an equivalent task does not already exist before
    // committing, so probing never leaves an invalid duplicate behind.
    if request.preflight {
        let (would_create, existing_task_id, existing_state) =
            match read_existing_task_state(&task_dir) {
                Some((existing_id, state)) => (false, Some(existing_id), Some(state)),
                None => (true, None, None),
            };
        return Ok(V3TaskCreateResult {
            status: "preflight".to_owned(),
            task_id: task_id.clone(),
            task_path: format!(".vibehub/tasks/{task_id}/task.yaml"),
            current_pointer_path: ".vibehub/tasks/current".to_owned(),
            current_pointer_updated: false,
            initial_node_id: None,
            lifecycle_version: 0,
            initial_plan_node_ids: plan_node_ids,
            preflight: true,
            would_create,
            existing_task_id,
            existing_state,
        });
    }

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
            &document.title,
            &document.intent,
            &document.acceptance_criteria,
            &document.workflow_profile,
            &policy,
            &initial_plan,
            &plan_node_ids,
        )?
    } else {
        // Existing eventless tasks predate lifecycle recording. They remain read-only;
        // only a durable pending marker authorizes interrupted-create recovery.
        0
    };

    if pending_marker.exists() {
        fs::remove_file(&pending_marker)
            .map_err(|error| io_error("V3_TASK_RECOVERY_MARKER_REMOVE_FAILED", error))?;
    }

    Ok(V3TaskCreateResult {
        status: if created { "created" } else { "already_exists" }.to_owned(),
        task_id: task_id.clone(),
        task_path: format!(".vibehub/tasks/{task_id}/task.yaml"),
        current_pointer_path: ".vibehub/tasks/current".to_owned(),
        current_pointer_updated: false,
        // New standard/full tasks without a confirmed plan deliberately have
        // no administrative placeholder node. The plan remains empty until a
        // real authored node is added through the typed plan command.
        initial_node_id: None,
        lifecycle_version,
        initial_plan_node_ids: plan_node_ids,
        preflight: false,
        would_create: created,
        existing_task_id: if created { None } else { Some(task_id.clone()) },
        existing_state: if created {
            None
        } else {
            read_existing_task_state(&task_dir).map(|(_, state)| state)
        },
    })
}

fn append_creation_events(
    application: &V3ApplicationService,
    project_id: &str,
    task_id: &str,
    title: &str,
    intent: &str,
    acceptance_criteria: &[String],
    workflow_profile: &str,
    policy: &EffectiveExecutionPolicy,
    initial_plan: &[V3TaskCreateInitialPlanNode],
    plan_node_ids: &[String],
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
    for (index, node) in initial_plan.iter().enumerate() {
        let node_id = &plan_node_ids[index];
        let lifecycle = application.task_lifecycle(project_id, task_id)?;
        if let Some(existing) = lifecycle.nodes.get(node_id) {
            if existing.is_historical_bootstrap() {
                return Err(task_error(
                    "V3_TASK_PLAN_INVALID",
                    format!(
                        "initial_plan node_id collides with historical bootstrap node: {node_id}"
                    ),
                ));
            }
            continue;
        }
        let dependencies: Vec<&str> = node
            .depends_on
            .iter()
            .map(|position| plan_node_ids[position - 1].as_str())
            .collect();
        let criterion_ids: Vec<String> = node
            .criteria
            .iter()
            .map(|position| canonical_criterion_id(task_id, position - 1))
            .collect();
        let mut node_command = command(
            "plan.node_added",
            project_id,
            task_id,
            lifecycle.version,
            &format!("create.{task_id}.plan.n{:02}", index + 1),
            json!({
                "node_id": node_id,
                "title": node.title,
                "goal": node.goal,
                "scope": node.scope,
                "dependencies": dependencies,
                "criterion_ids": criterion_ids,
                "origin": PlanNodeOrigin::Authored.as_str(),
                "role": node.role.clone().unwrap_or_else(|| PlanNodeRole::Execution.as_str().to_owned()),
                "origin_contract_version": PLAN_NODE_ORIGIN_CONTRACT_VERSION,
            }),
        );
        node_command.actor = "vibehub".to_owned();
        node_command.evidence_grade = Some(EvidenceGrade::HardObserved);
        application.lifecycle_command(node_command)?;
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
    let initial_plan = validate_initial_plan(
        request.initial_plan,
        acceptance_criteria.len(),
        &request.workflow_profile,
    )?;
    Ok(V3TaskCreateRequest {
        title,
        intent,
        acceptance_criteria,
        workflow_profile: request.workflow_profile,
        trigger_context: request.trigger_context,
        profile_override: request.profile_override,
        initial_plan,
        preflight: request.preflight,
    })
}

fn validate_initial_plan(
    plan: Vec<V3TaskCreateInitialPlanNode>,
    criteria_count: usize,
    workflow_profile: &str,
) -> Result<Vec<V3TaskCreateInitialPlanNode>, V3Error> {
    if plan.is_empty() {
        return Ok(plan);
    }
    if workflow_profile == "lightweight" {
        return Err(task_error(
            "V3_TASK_PLAN_FORBIDDEN",
            "lightweight tasks do not carry a plan",
        ));
    }
    if plan.len() > MAX_INITIAL_PLAN_NODES {
        return Err(task_error(
            "V3_TASK_PLAN_INVALID",
            format!("initial_plan must contain at most {MAX_INITIAL_PLAN_NODES} nodes"),
        ));
    }
    let mut covered = vec![false; criteria_count];
    let mut ids = std::collections::BTreeSet::new();
    let validated = plan
        .into_iter()
        .enumerate()
        .map(|(index, node)| {
            let position = index + 1;
            let node_id = node
                .node_id
                .map(|value| clean_plan_node_id(position, value))
                .transpose()?;
            if let Some(id) = &node_id {
                if !ids.insert(id.clone()) {
                    return Err(task_error(
                        "V3_TASK_PLAN_INVALID",
                        "initial_plan repeats a node_id",
                    ));
                }
            }
            if node
                .depends_on
                .iter()
                .any(|dependency| *dependency == 0 || *dependency >= position)
            {
                return Err(task_error(
                    "V3_TASK_PLAN_INVALID",
                    "initial_plan dependencies must point to earlier nodes",
                ));
            }
            let mut dependency_ids = std::collections::BTreeSet::new();
            if node
                .depends_on
                .iter()
                .any(|dependency| !dependency_ids.insert(*dependency))
            {
                return Err(task_error(
                    "V3_TASK_PLAN_INVALID",
                    "initial_plan repeats a dependency",
                ));
            }
            let mut criterion_ids = std::collections::BTreeSet::new();
            for criterion in &node.criteria {
                if *criterion == 0 || *criterion > criteria_count {
                    return Err(task_error(
                        "V3_TASK_PLAN_INVALID",
                        "initial_plan references a missing criterion",
                    ));
                }
                if !criterion_ids.insert(*criterion) {
                    return Err(task_error(
                        "V3_TASK_PLAN_INVALID",
                        "initial_plan repeats a criterion link",
                    ));
                }
                covered[*criterion - 1] = true;
            }
            if node
                .role
                .as_deref()
                .is_some_and(|role| !matches!(role, "execution" | "validation"))
            {
                return Err(task_error(
                    "V3_TASK_PLAN_INVALID",
                    "initial_plan role must be execution or validation",
                ));
            }
            Ok(V3TaskCreateInitialPlanNode {
                node_id,
                title: clean_field(&format!("initial_plan[{index}].title"), node.title)?,
                goal: clean_field(&format!("initial_plan[{index}].goal"), node.goal)?,
                scope: node
                    .scope
                    .into_iter()
                    .enumerate()
                    .map(|(entry, value)| {
                        clean_field(&format!("initial_plan[{index}].scope[{entry}]"), value)
                    })
                    .collect::<Result<_, _>>()?,
                depends_on: node.depends_on,
                criteria: node.criteria,
                role: node.role,
            })
        })
        .collect::<Result<Vec<_>, V3Error>>()?;
    if covered.iter().any(|covered| !covered) {
        return Err(task_error(
            "V3_TASK_PLAN_COVERAGE_INCOMPLETE",
            "initial_plan must cover every acceptance criterion",
        ));
    }
    Ok(validated)
}

fn clean_plan_node_id(position: usize, node_id: String) -> Result<String, V3Error> {
    let node_id = node_id.trim().to_owned();
    if node_id.len() < 3
        || node_id.len() > 127
        || !node_id.starts_with("node.")
        || !node_id.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-' | ':')
        })
    {
        return Err(task_error(
            "V3_TASK_PLAN_INVALID",
            format!("initial_plan[{position}] node_id is invalid"),
        ));
    }
    Ok(node_id)
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
    hasher.update([0]);
    hasher.update(serde_json::to_vec(&request.initial_plan).unwrap_or_default());
    let digest = format!("{:x}", hasher.finalize());
    format!("task.{slug}.{}", &digest[..12])
}

fn initial_plan_node_ids(
    task_id: &str,
    plan: &[V3TaskCreateInitialPlanNode],
) -> Result<Vec<String>, V3Error> {
    let ids = plan
        .iter()
        .enumerate()
        .map(|(index, node)| {
            node.node_id
                .clone()
                .unwrap_or_else(|| format!("node.{task_id}.n{:02}", index + 1))
        })
        .collect::<Vec<_>>();
    let unique = ids.iter().collect::<std::collections::BTreeSet<_>>();
    if unique.len() != ids.len() {
        return Err(task_error(
            "V3_TASK_PLAN_INVALID",
            "initial_plan generated node ids are not unique",
        ));
    }
    Ok(ids)
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

fn read_existing_task_state(task_dir: &Path) -> Option<(String, String)> {
    let yaml = fs::read_to_string(task_dir.join("task.yaml")).ok()?;
    let document: TaskDocument = serde_yaml::from_str(&yaml).ok()?;
    Some((
        document.task_id,
        format!("{}/{}", document.phase, document.phase_status),
    ))
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

fn task_error(code: &str, message: impl Into<String>) -> V3Error {
    V3Error::new(code, V3ErrorCategory::Validation, false, message)
}

fn io_error(code: &str, error: std::io::Error) -> V3Error {
    V3Error::new(code, V3ErrorCategory::Internal, false, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v3::{initialize_v3, LifecycleCommand, V3EventStore, V3ViewRepository};
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
            initial_plan: Vec::new(),
            preflight: false,
        }
    }

    #[test]
    fn creation_without_initial_plan_stays_in_planning_with_an_empty_graph() {
        let project = temp_project();
        initialize_v3(&project).unwrap();
        let result = create_v3_task(&project, request()).unwrap();
        assert_eq!(result.status, "created");
        assert!(project.join(&result.task_path).is_file());
        assert_eq!(result.lifecycle_version, 2);
        assert!(result.initial_node_id.is_none());
        let repository = V3ViewRepository::open(&project).unwrap();
        let bundle = repository.load_bundle(&result.task_id).unwrap();
        let nodes = bundle.plan_graph["nodes"].as_array().unwrap();
        assert!(nodes.is_empty());
        assert_eq!(bundle.plan_graph["plan_version"], 2);
        assert_eq!(bundle.plan_graph["scheduling_edges"], json!([]));
        assert!(bundle.plan_graph["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning["code"] == "V3_PLAN_NOT_RECORDED"));
        assert_eq!(bundle.node_brief["next_intent"], "plan");
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
    fn create_only_preserves_current_pointer_until_a_later_explicit_bind() {
        let project = temp_project();
        initialize_v3(&project).unwrap();
        let first = create_v3_task(&project, request()).unwrap();
        let current_pointer = project.join(".vibehub/tasks/current");
        let pointer_content = format!(
            "schema_version: 1\nkind: current_task_pointer\ntask_id: {}\npath: .vibehub/tasks/{}\nupdated_at: 2026-08-15T00:00:00Z\nupdated_by: test\n",
            first.task_id, first.task_id
        );
        fs::write(&current_pointer, &pointer_content).unwrap();

        let mut second_request = request();
        second_request.title = "Start an independent task later".to_owned();
        second_request.intent = "Create only, then bind explicitly".to_owned();
        second_request.workflow_profile = "lightweight".to_owned();
        let second = create_v3_task(&project, second_request).unwrap();
        assert!(!second.current_pointer_updated);
        assert_eq!(
            fs::read_to_string(&current_pointer).unwrap(),
            pointer_content
        );

        let repository = V3ViewRepository::open(&project).unwrap();
        let project_id = repository.project_id();
        let app = V3ApplicationService::open(&project).unwrap();
        app.session_task_bind(
            &project_id,
            &second.task_id,
            "session.create-and-start",
            "interaction.create-and-start",
            "codex",
            super::super::routing::BindingSource::CreatedAndStart,
            0,
            "bind.create-and-start",
            None,
            None,
            Some("codex".to_owned()),
            None,
        )
        .unwrap();
        app.session_open_with_context(
            &project_id,
            &second.task_id,
            "session.create-and-start",
            "codex",
            1,
            "open.create-and-start",
            None,
            None,
            None,
        )
        .unwrap();

        assert_eq!(repository.current_task_id().unwrap(), first.task_id);
        let binding = app
            .session_task_binding(&project_id, "session.create-and-start")
            .unwrap();
        assert_eq!(
            binding.bound_task_id.as_deref(),
            Some(second.task_id.as_str())
        );
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
        assert_eq!(second.lifecycle_version, 2);
        assert_eq!(second.status, "already_exists");
        let repository = V3ViewRepository::open(&project).unwrap();
        let events = V3EventStore::open(&project)
            .unwrap()
            .load_project(&repository.project_id())
            .unwrap();
        assert_eq!(events.len(), 2);
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn preflight_reports_existing_task_without_writing_or_emitting_events() {
        let project = temp_project();
        initialize_v3(&project).unwrap();
        let created = create_v3_task(&project, request()).unwrap();

        // An equivalent request preflights to "already present" and writes nothing.
        let mut probe = request();
        probe.preflight = true;
        let preflight = create_v3_task(&project, probe).unwrap();
        assert_eq!(preflight.status, "preflight");
        assert!(preflight.preflight);
        assert!(!preflight.would_create);
        assert_eq!(
            preflight.existing_task_id.as_deref(),
            Some(created.task_id.as_str())
        );
        assert!(preflight.existing_state.is_some());

        // A different request preflights to "would create".
        let mut fresh = request();
        fresh.preflight = true;
        fresh.title = "A clearly different task".to_owned();
        let fresh_preflight = create_v3_task(&project, fresh).unwrap();
        assert_eq!(fresh_preflight.status, "preflight");
        assert!(fresh_preflight.would_create);
        assert!(fresh_preflight.existing_task_id.is_none());

        // Preflight must not append any lifecycle events.
        let repository = V3ViewRepository::open(&project).unwrap();
        let events = V3EventStore::open(&project)
            .unwrap()
            .load_project(&repository.project_id())
            .unwrap();
        assert_eq!(events.len(), 2);
        fs::remove_dir_all(project).unwrap();
    }

    fn request_with_initial_plan() -> V3TaskCreateRequest {
        V3TaskCreateRequest {
            title: "Ship a planned task".to_owned(),
            intent: "Create real authored nodes at task creation time".to_owned(),
            acceptance_criteria: vec![
                "The first node is covered".to_owned(),
                "The second node is covered".to_owned(),
                "The final node is covered".to_owned(),
            ],
            workflow_profile: "full".to_owned(),
            trigger_context: Default::default(),
            profile_override: None,
            initial_plan: vec![
                V3TaskCreateInitialPlanNode {
                    node_id: Some("node.plan.first".to_owned()),
                    title: "First node".to_owned(),
                    goal: "Prepare the first stage".to_owned(),
                    scope: vec!["src/first".to_owned()],
                    depends_on: Vec::new(),
                    criteria: vec![1],
                    role: None,
                },
                V3TaskCreateInitialPlanNode {
                    node_id: Some("node.plan.second".to_owned()),
                    title: "Second node".to_owned(),
                    goal: "Prepare the second stage".to_owned(),
                    scope: vec!["src/second".to_owned()],
                    depends_on: Vec::new(),
                    criteria: vec![2],
                    role: Some("validation".to_owned()),
                },
                V3TaskCreateInitialPlanNode {
                    node_id: Some("node.plan.final".to_owned()),
                    title: "Final node".to_owned(),
                    goal: "Join both independent stages".to_owned(),
                    scope: vec!["src/final".to_owned()],
                    depends_on: vec![1, 2],
                    criteria: vec![3],
                    role: Some("execution".to_owned()),
                },
            ],
            preflight: false,
        }
    }

    #[test]
    fn initial_plan_is_written_as_authored_nodes_without_bootstrap() {
        let project = temp_project();
        initialize_v3(&project).unwrap();
        let request = request_with_initial_plan();
        let result = create_v3_task(&project, request.clone()).unwrap();
        assert!(result.initial_node_id.is_none());
        assert_eq!(
            result.initial_plan_node_ids,
            vec!["node.plan.first", "node.plan.second", "node.plan.final"]
        );
        assert_eq!(result.lifecycle_version, 7);

        let repository = V3ViewRepository::open(&project).unwrap();
        let bundle = repository.load_bundle(&result.task_id).unwrap();
        let nodes = bundle.plan_graph["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 3);
        assert!(nodes.iter().all(|node| node["node_id"].is_string()));
        let first = nodes
            .iter()
            .find(|node| node["node_id"] == "node.plan.first")
            .unwrap();
        let second = nodes
            .iter()
            .find(|node| node["node_id"] == "node.plan.second")
            .unwrap();
        let final_node = nodes
            .iter()
            .find(|node| node["node_id"] == "node.plan.final")
            .unwrap();
        assert_eq!(first["parallel_candidate"], true);
        assert_eq!(second["parallel_candidate"], true);
        assert_eq!(final_node["parallel_candidate"], false);
        assert_eq!(final_node["parallel_layer"], 1);
        assert_eq!(first["execution_state"], "not_started");
        assert_eq!(
            bundle.plan_graph["scheduling_edges"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        let app = V3ApplicationService::open(&project).unwrap();
        let project_id = repository.project_id();
        app.plan_set_state(crate::v3::PlanSetStateCommand {
            identity: crate::v3::PlanCommandIdentity {
                project_id: project_id.clone(),
                task_id: result.task_id.clone(),
                actor: "codex".to_owned(),
                expected_version: result.lifecycle_version,
                idempotency_key: "activate-first".to_owned(),
            },
            node_id: "node.plan.first".to_owned(),
            state: "active".to_owned(),
        })
        .unwrap();
        app.session_open_with_context(
            &project_id,
            &result.task_id,
            "session.plan.first",
            "codex",
            0,
            "open-first",
            Some(project.to_string_lossy().into_owned()),
            Some("node.plan.first".to_owned()),
            None,
        )
        .unwrap();
        let active_bundle = repository.load_bundle(&result.task_id).unwrap();
        let active_first = active_bundle.plan_graph["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["node_id"] == "node.plan.first")
            .unwrap();
        assert_eq!(active_first["execution_state"], "session_active");
        assert_eq!(
            active_first["agent_result_ids"].as_array().unwrap().len(),
            0
        );
        let lifecycle = V3ApplicationService::open(&project)
            .unwrap()
            .task_lifecycle(&repository.project_id(), &result.task_id)
            .unwrap();
        assert!(lifecycle
            .effective_nodes()
            .all(|(_, node)| !node.is_historical_bootstrap()));
        assert!(lifecycle
            .nodes
            .values()
            .all(|node| node.origin == super::super::lifecycle::PlanNodeOrigin::Authored));
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn historical_bootstrap_event_remains_in_timeline_but_is_excluded_from_current_plan() {
        let project = temp_project();
        initialize_v3(&project).unwrap();
        let result = create_v3_task(&project, request()).unwrap();
        let repository = V3ViewRepository::open(&project).unwrap();
        let project_id = repository.project_id();
        let app = V3ApplicationService::open(&project).unwrap();
        app.lifecycle_command(LifecycleCommand {
            event_type: "plan.node_added".to_owned(),
            project_id: project_id.clone(),
            task_id: result.task_id.clone(),
            node_id: Some("node.task.legacy.initial".to_owned().into()),
            session_id: None,
            actor: "vibehub".to_owned(),
            expected_version: result.lifecycle_version,
            idempotency_key: "legacy-bootstrap-event".to_owned(),
            evidence_grade: Some(crate::v3::EvidenceGrade::HardObserved),
            payload: json!({"node_id":"node.task.legacy.initial","title":"Legacy placeholder","goal":"Administrative bootstrap","scope":[],"dependencies":[]}),
        }).unwrap();
        let lifecycle = app.task_lifecycle(&project_id, &result.task_id).unwrap();
        assert!(lifecycle.nodes["node.task.legacy.initial"].is_historical_bootstrap());
        app.plan_add_node(crate::v3::PlanAddNodeCommand {
            identity: crate::v3::PlanCommandIdentity {
                project_id: project_id.clone(),
                task_id: result.task_id.clone(),
                actor: "codex".to_owned(),
                expected_version: result.lifecycle_version + 1,
                idempotency_key: "authored-after-legacy".to_owned(),
            },
            node_id: "node.authored.real".to_owned(),
            title: "Real node".to_owned(),
            goal: "Actual implementation".to_owned(),
            scope: vec!["src".to_owned()],
            dependencies: Vec::new(),
            criterion_ids: vec![canonical_criterion_id(&result.task_id, 0)],
        })
        .unwrap();
        let bundle = repository.load_bundle(&result.task_id).unwrap();
        assert_eq!(bundle.plan_graph["nodes"].as_array().unwrap().len(), 1);
        assert_eq!(
            bundle.plan_graph["nodes"][0]["node_id"],
            "node.authored.real"
        );
        assert!(bundle.task_timeline["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["node_id"] == "node.task.legacy.initial"));
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

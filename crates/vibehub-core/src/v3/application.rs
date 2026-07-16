use super::domain::{
    AppendResult, EventDraft, EvidenceGrade, NodeId, ProjectId, SessionId, TaskId, V3Error,
    WorktreeId,
};
use super::event_store::V3EventStore;
use super::lifecycle::{
    apply_command_with_required_criteria, LifecycleCommand, PlanAddNodeCommand,
    PlanSetDependenciesCommand, PlanSetStateCommand, TaskLifecycleProjection,
};
use super::orchestration::{self, OrchestrationCommand, OrchestrationProjection};
use super::projection::{self, V3Projection};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct V3ApplicationService {
    store: V3EventStore,
}

impl V3ApplicationService {
    pub fn open(project_root: impl AsRef<Path>) -> Result<Self, V3Error> {
        Ok(Self {
            store: V3EventStore::open(project_root)?,
        })
    }

    pub fn session_open(
        &self,
        project_id: &str,
        task_id: &str,
        session_id: &str,
        actor: &str,
        expected_version: u64,
        idempotency_key: &str,
    ) -> Result<AppendResult, V3Error> {
        self.session_open_with_context(
            project_id,
            task_id,
            session_id,
            actor,
            expected_version,
            idempotency_key,
            None,
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn session_open_with_context(
        &self,
        project_id: &str,
        task_id: &str,
        session_id: &str,
        actor: &str,
        expected_version: u64,
        idempotency_key: &str,
        working_directory: Option<String>,
        node_id: Option<String>,
        worktree_id: Option<String>,
    ) -> Result<AppendResult, V3Error> {
        self.session_open_with_context_and_provider(
            project_id,
            task_id,
            session_id,
            actor,
            expected_version,
            idempotency_key,
            working_directory,
            node_id,
            worktree_id,
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn session_open_with_context_and_provider(
        &self,
        project_id: &str,
        task_id: &str,
        session_id: &str,
        actor: &str,
        expected_version: u64,
        idempotency_key: &str,
        working_directory: Option<String>,
        node_id: Option<String>,
        worktree_id: Option<String>,
        provider: Option<String>,
        provider_session_id: Option<String>,
    ) -> Result<AppendResult, V3Error> {
        let mut payload = json!({});
        if let Some(path) = working_directory {
            payload["working_directory"] = Value::String(path);
        }
        if let Some(provider) = provider.filter(|value| !value.trim().is_empty()) {
            payload["provider"] = Value::String(provider);
        }
        if let Some(provider_session_id) =
            provider_session_id.filter(|value| !value.trim().is_empty())
        {
            payload["provider_session_id"] = Value::String(provider_session_id);
        }
        self.append_session_event_with_context(
            "session.opened",
            project_id,
            task_id,
            session_id,
            actor,
            expected_version,
            idempotency_key,
            payload,
            node_id,
            worktree_id,
        )
    }

    pub fn event_log(
        &self,
        kind: &str,
        project_id: &str,
        task_id: &str,
        session_id: &str,
        actor: &str,
        expected_version: u64,
        idempotency_key: &str,
        details: Value,
    ) -> Result<AppendResult, V3Error> {
        let event_type = match kind {
            "progress" => "progress.logged",
            "risk" => "risk.logged",
            other => other,
        };
        self.append_session_event(
            event_type,
            project_id,
            task_id,
            session_id,
            actor,
            expected_version,
            idempotency_key,
            details,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn agent_result_record(
        &self,
        project_id: &str,
        task_id: &str,
        session_id: &str,
        actor: &str,
        expected_version: u64,
        idempotency_key: &str,
        result_id: &str,
        node_id: Option<String>,
        details: Value,
    ) -> Result<AppendResult, V3Error> {
        for field in ["kind", "request_source", "instruction", "status", "summary"] {
            if details.get(field).and_then(Value::as_str).is_none() {
                return Err(V3Error::new(
                    "V3_AGENT_RESULT_INVALID",
                    super::domain::V3ErrorCategory::Validation,
                    false,
                    format!("agent result requires string field {field}"),
                ));
            }
        }
        validate_agent_result_value(&details, "kind", &["execution", "evaluation"])?;
        validate_agent_result_value(
            &details,
            "request_source",
            &["user_request", "evaluation_instruction"],
        )?;
        validate_agent_result_value(
            &details,
            "status",
            &["pending", "running", "succeeded", "failed"],
        )?;
        validate_agent_result_details(&details)?;
        let mut payload = details;
        payload["result_id"] = Value::String(result_id.to_owned());
        self.append_session_event_with_context(
            "agent.result_recorded",
            project_id,
            task_id,
            session_id,
            actor,
            expected_version,
            idempotency_key,
            payload,
            node_id,
            None,
        )
    }

    pub fn session_close(
        &self,
        project_id: &str,
        task_id: &str,
        session_id: &str,
        actor: &str,
        expected_version: u64,
        idempotency_key: &str,
    ) -> Result<AppendResult, V3Error> {
        self.append_session_event(
            "session.closed",
            project_id,
            task_id,
            session_id,
            actor,
            expected_version,
            idempotency_key,
            json!({}),
        )
    }

    pub fn rebuild(&self, project_id: &str) -> Result<V3Projection, V3Error> {
        let events = self.store.load_project(project_id)?;
        let projection = projection::fold(project_id, &events);
        projection::write_atomic(&self.store.projection_path(project_id), &projection)?;
        Ok(projection)
    }

    pub fn aggregate_version(&self, project_id: &str, aggregate_id: &str) -> Result<u64, V3Error> {
        Ok(self
            .store
            .load_project(project_id)?
            .into_iter()
            .filter(|event| event.aggregate_id == aggregate_id)
            .map(|event| event.aggregate_version)
            .max()
            .unwrap_or(0))
    }

    pub fn lifecycle_command(&self, command: LifecycleCommand) -> Result<AppendResult, V3Error> {
        let required_criterion_ids = self.required_criterion_ids(&command.task_id)?;
        apply_command_with_required_criteria(&self.store, command, &required_criterion_ids)
    }

    pub fn plan_add_node(&self, command: PlanAddNodeCommand) -> Result<AppendResult, V3Error> {
        self.lifecycle_command(command.into())
    }

    pub fn plan_set_dependencies(
        &self,
        command: PlanSetDependenciesCommand,
    ) -> Result<AppendResult, V3Error> {
        self.lifecycle_command(command.into())
    }

    pub fn plan_set_state(&self, command: PlanSetStateCommand) -> Result<AppendResult, V3Error> {
        self.lifecycle_command(command.into())
    }

    pub fn task_lifecycle(
        &self,
        project_id: &str,
        task_id: &str,
    ) -> Result<TaskLifecycleProjection, V3Error> {
        let events = self.store.load_project(project_id)?;
        Ok(super::lifecycle::fold_task(task_id, &events))
    }

    pub fn orchestration_command(
        &self,
        command: OrchestrationCommand,
    ) -> Result<AppendResult, V3Error> {
        orchestration::apply_command(&self.store, command)
    }

    pub fn worktree_orchestration(
        &self,
        project_id: &str,
        task_id: &str,
    ) -> Result<OrchestrationProjection, V3Error> {
        let events = self.store.load_project(project_id)?;
        Ok(orchestration::fold_task(task_id, &events))
    }

    fn required_criterion_ids(&self, task_id: &str) -> Result<BTreeSet<String>, V3Error> {
        #[derive(Deserialize)]
        struct TaskCriteria {
            #[serde(default)]
            acceptance_criteria: Vec<String>,
        }

        let path = self
            .store
            .project_root()
            .join(".vibehub/tasks")
            .join(task_id)
            .join("task.yaml");
        let content = fs::read_to_string(&path).map_err(|error| {
            V3Error::new(
                "V3_TASK_READ_FAILED",
                super::domain::V3ErrorCategory::NotFound,
                false,
                error.to_string(),
            )
        })?;
        let task: TaskCriteria = serde_yaml::from_str(&content).map_err(|error| {
            V3Error::new(
                "V3_TASK_INVALID",
                super::domain::V3ErrorCategory::CorruptLog,
                false,
                error.to_string(),
            )
        })?;
        Ok(task
            .acceptance_criteria
            .iter()
            .enumerate()
            .map(|(index, _)| canonical_criterion_id(task_id, index))
            .collect())
    }

    #[allow(clippy::too_many_arguments)]
    fn append_session_event_with_context(
        &self,
        event_type: &str,
        project_id: &str,
        task_id: &str,
        session_id: &str,
        actor: &str,
        expected_version: u64,
        idempotency_key: &str,
        payload: Value,
        node_id: Option<String>,
        worktree_id: Option<String>,
    ) -> Result<AppendResult, V3Error> {
        self.store.append(EventDraft {
            event_type: event_type.to_owned(),
            aggregate_id: session_id.to_owned(),
            expected_version,
            idempotency_key: idempotency_key.to_owned(),
            project_id: ProjectId::from(project_id),
            task_id: TaskId::from(task_id),
            node_id: node_id.map(NodeId),
            session_id: Some(SessionId::from(session_id)),
            worktree_id: worktree_id.map(WorktreeId),
            lease_id: None,
            operation_id: None,
            actor: actor.to_owned(),
            evidence_grade: EvidenceGrade::AgentReported,
            occurred_at: None,
            commit_sha: None,
            payload,
        })
    }

    fn append_session_event(
        &self,
        event_type: &str,
        project_id: &str,
        task_id: &str,
        session_id: &str,
        actor: &str,
        expected_version: u64,
        idempotency_key: &str,
        payload: Value,
    ) -> Result<AppendResult, V3Error> {
        self.append_session_event_with_context(
            event_type,
            project_id,
            task_id,
            session_id,
            actor,
            expected_version,
            idempotency_key,
            payload,
            None,
            None,
        )
    }
}

fn validate_agent_result_value(
    details: &Value,
    field: &str,
    allowed: &[&str],
) -> Result<(), V3Error> {
    let value = details
        .get(field)
        .and_then(Value::as_str)
        .expect("required agent result string fields are checked before enum validation");
    if allowed.contains(&value) {
        return Ok(());
    }
    Err(V3Error::new(
        "V3_AGENT_RESULT_INVALID",
        super::domain::V3ErrorCategory::Validation,
        false,
        format!(
            "agent result field {field} must be one of: {}",
            allowed.join(", ")
        ),
    ))
}

fn invalid_agent_result(message: impl Into<String>) -> V3Error {
    V3Error::new(
        "V3_AGENT_RESULT_INVALID",
        super::domain::V3ErrorCategory::Validation,
        false,
        message,
    )
}

fn validate_agent_result_details(details: &Value) -> Result<(), V3Error> {
    if let Some(evaluation) = details.get("evaluation").filter(|value| !value.is_null()) {
        let object = evaluation.as_object().ok_or_else(|| {
            invalid_agent_result("agent result evaluation must be an object or null")
        })?;
        if object.get("target").and_then(Value::as_str).is_none() {
            return Err(invalid_agent_result(
                "agent result evaluation requires string field target",
            ));
        }
        let rubric = object
            .get("rubric")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                invalid_agent_result("agent result evaluation requires string array field rubric")
            })?;
        if rubric
            .iter()
            .any(|item| item.as_str().is_none_or(str::is_empty))
        {
            return Err(invalid_agent_result(
                "agent result evaluation rubric entries must be non-empty strings",
            ));
        }
        let verdict = object
            .get("verdict")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                invalid_agent_result("agent result evaluation requires string field verdict")
            })?;
        if !["passed", "needs_revision", "failed", "inconclusive"].contains(&verdict) {
            return Err(invalid_agent_result(
                "agent result evaluation verdict is outside the V3 contract",
            ));
        }
        let findings = object
            .get("findings")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                invalid_agent_result("agent result evaluation requires array field findings")
            })?;
        for finding in findings {
            let finding = finding.as_object().ok_or_else(|| {
                invalid_agent_result("agent result evaluation findings must be objects")
            })?;
            for field in ["title", "detail", "severity"] {
                if finding.get(field).and_then(Value::as_str).is_none() {
                    return Err(invalid_agent_result(format!(
                        "agent result evaluation finding requires string field {field}"
                    )));
                }
            }
            let severity = finding
                .get("severity")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if !["info", "low", "medium", "high", "critical"].contains(&severity) {
                return Err(invalid_agent_result(
                    "agent result evaluation finding severity is outside the V3 contract",
                ));
            }
            if finding
                .get("evidence_refs")
                .and_then(Value::as_array)
                .is_none()
            {
                return Err(invalid_agent_result(
                    "agent result evaluation finding requires array field evidence_refs",
                ));
            }
        }
    }
    for field in ["artifacts", "evidence_refs"] {
        if details.get(field).is_some_and(|value| !value.is_array()) {
            return Err(invalid_agent_result(format!(
                "agent result field {field} must be an array"
            )));
        }
    }
    Ok(())
}

pub(crate) fn canonical_criterion_id(task_id: &str, index: usize) -> String {
    format!(
        "criterion.{}.c{:02}",
        task_id.to_ascii_lowercase(),
        index + 1
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v3::lifecycle::command;
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn rebuild_is_deterministic_after_projection_deletion() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-app-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub")).unwrap();
        let app = V3ApplicationService::open(&root).unwrap();
        app.session_open(
            "project.test",
            "task.test",
            "session.main",
            "codex",
            0,
            "open.1",
        )
        .unwrap();
        app.event_log(
            "progress",
            "project.test",
            "task.test",
            "session.main",
            "codex",
            1,
            "log.1",
            json!({"summary": "working"}),
        )
        .unwrap();
        app.session_close(
            "project.test",
            "task.test",
            "session.main",
            "codex",
            2,
            "close.1",
        )
        .unwrap();
        let first = app.rebuild("project.test").unwrap();
        fs::remove_file(app.store.projection_path("project.test")).unwrap();
        let second = app.rebuild("project.test").unwrap();
        assert_eq!(first, second);
        assert_eq!(second.sessions["session.main"].state, "closed");
        assert_eq!(second.sessions["session.main"].progress_entries, 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn agent_result_rejects_status_outside_the_contract() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-result-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub")).unwrap();
        let app = V3ApplicationService::open(&root).unwrap();
        app.session_open(
            "project.test",
            "task.test",
            "session.test",
            "codex",
            0,
            "open.1",
        )
        .unwrap();

        let error = app
            .agent_result_record(
                "project.test",
                "task.test",
                "session.test",
                "codex",
                1,
                "result.blocked",
                "result.test",
                None,
                json!({
                    "kind": "execution",
                    "request_source": "user_request",
                    "instruction": "Run the task",
                    "status": "blocked",
                    "summary": "Blocked"
                }),
            )
            .unwrap_err();
        assert_eq!(error.code, "V3_AGENT_RESULT_INVALID");

        app.agent_result_record(
            "project.test",
            "task.test",
            "session.test",
            "codex",
            1,
            "result.failed",
            "result.test",
            None,
            json!({
                "kind": "execution",
                "request_source": "user_request",
                "instruction": "Run the task",
                "status": "failed",
                "summary": "Blocked by an external prerequisite"
            }),
        )
        .unwrap();

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn agent_result_rejects_malformed_evaluation_details() {
        let root =
            std::env::temp_dir().join(format!("vibehub-v3-result-evaluation-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub")).unwrap();
        let app = V3ApplicationService::open(&root).unwrap();
        app.session_open(
            "project.test",
            "task.test",
            "session.test",
            "codex",
            0,
            "open.1",
        )
        .unwrap();

        let error = app
            .agent_result_record(
                "project.test",
                "task.test",
                "session.test",
                "codex",
                1,
                "result.invalid-evaluation",
                "result.test",
                None,
                json!({
                    "kind": "evaluation",
                    "request_source": "evaluation_instruction",
                    "instruction": "Review the task",
                    "status": "succeeded",
                    "summary": "Review finished",
                    "evaluation": {
                        "target": "workspace",
                        "verdict": "needs_action",
                        "findings": ["missing rubric"]
                    }
                }),
            )
            .unwrap_err();
        assert_eq!(error.code, "V3_AGENT_RESULT_INVALID");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn completion_requires_every_canonical_task_criterion() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-app-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub/tasks/task.test")).unwrap();
        fs::write(
            root.join(".vibehub/tasks/task.test/task.yaml"),
            "task_id: task.test\ntitle: Test task\nintent: Verify completion\nphase: implement\nphase_status: active\nacceptance_criteria:\n- First criterion\n- Second criterion\n",
        )
        .unwrap();
        let app = V3ApplicationService::open(&root).unwrap();
        app.lifecycle_command(command(
            "criterion.passed",
            "project.test",
            "task.test",
            0,
            "criterion.1",
            json!({"criterion_id":"criterion.task.test.c01","reviewer":"reviewer","evidence_refs":["evidence.1"]}),
        ))
        .unwrap();

        let partial_digest = app
            .task_lifecycle("project.test", "task.test")
            .unwrap()
            .completion_digest();
        let incomplete = app
            .lifecycle_command(command(
                "task.completion_proposed",
                "project.test",
                "task.test",
                1,
                "proposal.partial",
                json!({"digest":partial_digest}),
            ))
            .unwrap_err();
        assert_eq!(incomplete.code, "V3_TASK_NOT_COMPLETABLE");

        app.lifecycle_command(command(
            "criterion.passed",
            "project.test",
            "task.test",
            1,
            "criterion.2",
            json!({"criterion_id":"criterion.task.test.c02","reviewer":"reviewer","evidence_refs":["evidence.2"]}),
        ))
        .unwrap();
        let complete_digest = app
            .task_lifecycle("project.test", "task.test")
            .unwrap()
            .completion_digest();
        app.lifecycle_command(command(
            "task.completion_proposed",
            "project.test",
            "task.test",
            2,
            "proposal.complete",
            json!({"digest":complete_digest}),
        ))
        .unwrap();

        let projection = app.task_lifecycle("project.test", "task.test").unwrap();
        assert_eq!(projection.state, "completion_pending");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_rebuild_includes_worktree_orchestration() {
        use crate::v3::orchestration::OrchestrationCommand;

        let root = std::env::temp_dir().join(format!("vibehub-v3-app-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub")).unwrap();
        let app = V3ApplicationService::open(&root).unwrap();
        app.orchestration_command(OrchestrationCommand {
            event_type: "worktree.planned".to_owned(),
            project_id: "project.test".to_owned(),
            task_id: "task.test".to_owned(),
            node_id: "node.test".to_owned(),
            worktree_id: "worktree.test".to_owned(),
            session_id: Some("session.test".to_owned()),
            lease_id: None,
            operation_id: Some("operation.plan".to_owned()),
            eligibility_digest: "a".repeat(64),
            lease_generation: None,
            actor: "codex".to_owned(),
            expected_version: 0,
            idempotency_key: "worktree.plan.1".to_owned(),
            evidence_grade: Some(EvidenceGrade::HardObserved),
            payload: json!({}),
        })
        .unwrap();

        let first = app.rebuild("project.test").unwrap();
        fs::remove_file(app.store.projection_path("project.test")).unwrap();
        let second = app.rebuild("project.test").unwrap();
        assert_eq!(first, second);
        assert_eq!(
            second.worktrees["worktree.test"].state,
            crate::v3::worktree::WorktreeState::Planned
        );
        assert!(second.unknown_event_types.is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}

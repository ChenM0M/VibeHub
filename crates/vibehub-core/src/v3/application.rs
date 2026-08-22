use super::domain::{
    AppendResult, EventDraft, EvidenceGrade, NodeId, ProjectId, SessionId, TaskId, V3Error,
    WorktreeId,
};
use super::event_store::V3EventStore;
use super::lifecycle::{
    apply_command_with_required_criteria, is_terminal_plan_node_state,
    plan_node_state_covers_criteria, LifecycleCommand, PlanAddNodeCommand,
    PlanSetDependenciesCommand, PlanSetStateCommand, TaskLifecycleProjection,
};
use super::orchestration::{self, OrchestrationCommand, OrchestrationProjection};
use super::projection::{self, V3Projection};
use super::routing::{
    BindingFreshness, BindingSource, BindingStatus, SessionTaskBinding, SessionTaskIdentity,
    SESSION_TASK_ROUTING_SCHEMA_VERSION,
};
use chrono::{SecondsFormat, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

fn default_standard_profile() -> String {
    "standard".to_owned()
}

#[derive(Default)]
struct SessionFacts {
    exists: bool,
    open: bool,
    closed: bool,
    gapped: bool,
    terminal_result: bool,
    task_id: Option<String>,
    node_id: Option<String>,
    binding: Option<SessionTaskBinding>,
}
fn session_error(code: &str, message: &str) -> V3Error {
    V3Error::new(
        code,
        super::domain::V3ErrorCategory::Validation,
        false,
        message,
    )
}

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

    /// Get the current git HEAD SHA, if available.
    ///
    /// Returns None if git is not available or the project root is not a
    /// git repository.
    pub(crate) fn git_head_sha(&self) -> Option<String> {
        let root = self.store.project_root();
        let output = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(root)
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let sha = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if sha.is_empty() {
            None
        } else {
            Some(sha)
        }
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
        let facts_before_open = self.session_facts(project_id, session_id)?;
        let mut open_expected_version = expected_version;
        if facts_before_open.binding.is_none() && !facts_before_open.exists {
            // A direct session_open carries an explicit task_id, so this is a
            // compatibility create-and-start path rather than a fuzzy route.
            // Persist the binding first so every following task-scoped write is
            // still checked against a typed binding fact.
            self.session_task_bind(
                project_id,
                task_id,
                session_id,
                session_id,
                actor,
                BindingSource::LegacySessionOpen,
                expected_version,
                &format!("{idempotency_key}.binding"),
                None,
                None,
                provider.clone(),
                None,
            )?;
            open_expected_version = self.aggregate_version(project_id, session_id)?;
        }
        self.require_session_binding(project_id, task_id, session_id, None)?;
        if let Some(policy) = self.enforced_task_policy(task_id)? {
            if policy.planning_required {
                let node_id = node_id.as_deref().ok_or_else(|| {
                    session_error(
                        "V3_SESSION_NODE_REQUIRED",
                        "standard/full session_open requires an active plan node",
                    )
                })?;
                let lifecycle = self.task_lifecycle(project_id, task_id)?;
                let node = lifecycle.effective_node(node_id).ok_or_else(|| {
                    session_error(
                        "V3_SESSION_NODE_NOT_FOUND",
                        "session node does not exist in the task plan",
                    )
                })?;
                if node.state != "active"
                    || !node.dependencies.iter().all(|dependency| {
                        lifecycle.effective_node(dependency).is_some_and(|value| {
                            matches!(value.state.as_str(), "completed" | "waived")
                        })
                    })
                {
                    return Err(session_error(
                        "V3_SESSION_NODE_NOT_ACTIVE_READY",
                        "session node must exist, have satisfied dependencies, and be active",
                    ));
                }
            } else if node_id.is_some() {
                return Err(session_error(
                    "V3_LIGHTWEIGHT_NODE_FORBIDDEN",
                    "lightweight sessions use no plan-node binding",
                ));
            }
        }
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
        // Record the git HEAD at session open for commit-to-Task traceability
        if let Some(git_head) = self.git_head_sha() {
            payload["git_head_sha"] = Value::String(git_head);
        }
        let facts_after_binding = self.session_facts(project_id, session_id)?;
        if facts_after_binding.open || facts_after_binding.closed || facts_after_binding.gapped {
            if let Some(existing) = self
                .store
                .load_project(project_id)?
                .into_iter()
                .find(|event| {
                    event.aggregate_id == session_id
                        && event.task_id.0 == task_id
                        && event.event_type == "session.opened"
                        && event.idempotency_key == idempotency_key
                })
            {
                // Binding events are part of the session aggregate now. Return
                // the original session.opened event for an exact retry instead
                // of reusing a pre-binding expected-version calculation.
                return Ok(AppendResult::Duplicate { event: existing });
            }
            if self.store.load_project(project_id)?.iter().any(|event| {
                event.aggregate_id == session_id
                    && event.event_type == "session.opened"
                    && event.idempotency_key == idempotency_key
            }) {
                return self.append_session_event_with_context(
                    "session.opened",
                    project_id,
                    task_id,
                    session_id,
                    actor,
                    open_expected_version,
                    idempotency_key,
                    payload,
                    node_id,
                    worktree_id,
                );
            }
            return Err(session_error(
                "V3_SESSION_ALREADY_EXISTS",
                "session_id is already recorded",
            ));
        }
        self.append_session_event_with_context(
            "session.opened",
            project_id,
            task_id,
            session_id,
            actor,
            open_expected_version,
            idempotency_key,
            payload,
            node_id,
            worktree_id,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn session_task_bind(
        &self,
        project_id: &str,
        task_id: &str,
        session_id: &str,
        interaction_id: &str,
        actor: &str,
        source: BindingSource,
        expected_version: u64,
        idempotency_key: &str,
        expected_binding_revision: Option<u64>,
        agent_id: Option<String>,
        host: Option<String>,
        _provider_session_id: Option<String>,
    ) -> Result<AppendResult, V3Error> {
        if interaction_id.trim().is_empty() {
            return Err(session_error(
                "V3_TASK_BINDING_IDENTITY_REQUIRED",
                "interaction_id is required for Session–Task binding",
            ));
        }
        self.validate_binding_target(project_id, task_id)?;
        let facts = self.session_facts(project_id, session_id)?;
        let current_revision = facts
            .binding
            .as_ref()
            .map(|binding| binding.binding_revision)
            .unwrap_or(0);
        if expected_binding_revision.is_some_and(|expected| expected != current_revision) {
            return Err(V3Error::new(
                "V3_TASK_BINDING_REVISION_CONFLICT",
                super::domain::V3ErrorCategory::VersionConflict,
                true,
                "expected Session–Task binding revision does not match the current binding",
            )
            .with_detail(
                "expected_binding_revision",
                expected_binding_revision.unwrap_or(0),
            )
            .with_detail("current_binding_revision", current_revision));
        }
        let target_task_revision = self.task_lifecycle(project_id, task_id)?.version;
        let identity = SessionTaskIdentity {
            project_id: project_id.to_owned(),
            interaction_id: interaction_id.to_owned(),
            session_id: session_id.to_owned(),
            agent_id,
            host,
        };
        let binding = SessionTaskBinding {
            schema_version: SESSION_TASK_ROUTING_SCHEMA_VERSION.to_owned(),
            project_id: project_id.to_owned(),
            interaction_id: identity.interaction_id.clone(),
            session_id: identity.session_id.clone(),
            agent_id: identity.agent_id.clone(),
            host: identity.host.clone(),
            bound_task_id: Some(task_id.to_owned()),
            binding_revision: current_revision.saturating_add(1),
            freshness: BindingFreshness::Fresh,
            source,
            status: BindingStatus::Bound,
            target_task_revision: Some(target_task_revision),
            bound_at: Some(Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)),
        };
        self.store.append_with_rebuild(EventDraft {
            event_type: "session.task_bound".to_owned(),
            aggregate_id: session_id.to_owned(),
            expected_version,
            idempotency_key: idempotency_key.to_owned(),
            project_id: ProjectId(project_id.to_owned()),
            task_id: TaskId(task_id.to_owned()),
            node_id: None,
            session_id: Some(SessionId(session_id.to_owned())),
            worktree_id: None,
            lease_id: None,
            operation_id: None,
            actor: actor.to_owned(),
            evidence_grade: EvidenceGrade::HardObserved,
            occurred_at: None,
            commit_sha: None,
            payload: json!({
                "schema_version": SESSION_TASK_ROUTING_SCHEMA_VERSION,
                "binding": binding,
                "binding_revision": current_revision.saturating_add(1),
            }),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn session_task_unbind(
        &self,
        project_id: &str,
        task_id: &str,
        session_id: &str,
        actor: &str,
        expected_version: u64,
        idempotency_key: &str,
        expected_binding_revision: Option<u64>,
    ) -> Result<AppendResult, V3Error> {
        let facts = self.session_facts(project_id, session_id)?;
        let current = facts.binding.ok_or_else(|| {
            session_error(
                "V3_TASK_BINDING_REQUIRED",
                "session has no recorded Task binding",
            )
        })?;
        if current.bound_task_id.as_deref() != Some(task_id) {
            return Err(session_error(
                "V3_TASK_BINDING_MISMATCH",
                "session binding does not target the requested Task",
            ));
        }
        if expected_binding_revision.is_some_and(|expected| expected != current.binding_revision) {
            return Err(V3Error::new(
                "V3_TASK_BINDING_REVISION_CONFLICT",
                super::domain::V3ErrorCategory::VersionConflict,
                true,
                "expected Session–Task binding revision does not match the current binding",
            ));
        }
        let binding = SessionTaskBinding {
            schema_version: SESSION_TASK_ROUTING_SCHEMA_VERSION.to_owned(),
            project_id: project_id.to_owned(),
            interaction_id: current.interaction_id,
            session_id: session_id.to_owned(),
            agent_id: current.agent_id,
            host: current.host,
            bound_task_id: None,
            binding_revision: current.binding_revision.saturating_add(1),
            freshness: BindingFreshness::Unknown,
            source: BindingSource::UserConfirmed,
            status: BindingStatus::Unbound,
            target_task_revision: None,
            bound_at: None,
        };
        self.store.append_with_rebuild(EventDraft {
            event_type: "session.task_unbound".to_owned(),
            aggregate_id: session_id.to_owned(),
            expected_version,
            idempotency_key: idempotency_key.to_owned(),
            project_id: ProjectId(project_id.to_owned()),
            task_id: TaskId(task_id.to_owned()),
            node_id: None,
            session_id: Some(SessionId(session_id.to_owned())),
            worktree_id: None,
            lease_id: None,
            operation_id: None,
            actor: actor.to_owned(),
            evidence_grade: EvidenceGrade::UserConfirmed,
            occurred_at: None,
            commit_sha: None,
            payload: json!({"schema_version": SESSION_TASK_ROUTING_SCHEMA_VERSION, "binding": binding}),
        })
    }

    pub fn session_task_binding(
        &self,
        project_id: &str,
        session_id: &str,
    ) -> Result<SessionTaskBinding, V3Error> {
        let facts = self.session_facts(project_id, session_id)?;
        Ok(facts.binding.unwrap_or_else(|| {
            SessionTaskBinding::unbound(&SessionTaskIdentity {
                project_id: project_id.to_owned(),
                interaction_id: session_id.to_owned(),
                session_id: session_id.to_owned(),
                agent_id: None,
                host: None,
            })
        }))
    }

    /// Validate the binding precondition for any Task-scoped write whose
    /// operation is not itself a session bind/open action. The caller may use
    /// this before constructing a plan, criterion, finding, memory, or
    /// orchestration command.
    pub fn validate_task_binding(
        &self,
        project_id: &str,
        task_id: &str,
        session_id: &str,
        expected_binding_revision: Option<u64>,
    ) -> Result<(), V3Error> {
        self.require_session_binding(project_id, task_id, session_id, expected_binding_revision)
            .map(|_| ())
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
        self.require_open_session(project_id, task_id, session_id)?;
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
        let facts = self.require_open_session(project_id, task_id, session_id)?;
        if self.enforced_task_policy(task_id)?.is_some() && facts.node_id != node_id {
            return Err(session_error(
                "V3_AGENT_RESULT_NODE_MISMATCH",
                "agent_result node_id must match the open session node",
            ));
        }
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
        let facts = self.require_open_session(project_id, task_id, session_id)?;
        if self.enforced_task_policy(task_id)?.is_some() && !facts.terminal_result {
            return Err(session_error(
                "V3_SESSION_TERMINAL_RESULT_REQUIRED",
                "session_close requires a real succeeded/failed agent result",
            ));
        }
        // Record the git HEAD at session close for commit-to-Task traceability
        let mut close_payload = json!({});
        if let Some(git_head) = self.git_head_sha() {
            close_payload["git_head_sha"] = Value::String(git_head);
        }
        self.append_session_event(
            "session.closed",
            project_id,
            task_id,
            session_id,
            actor,
            expected_version,
            idempotency_key,
            close_payload,
        )
    }

    pub fn session_gap(
        &self,
        project_id: &str,
        task_id: &str,
        session_id: &str,
        actor: &str,
        expected_version: u64,
        idempotency_key: &str,
        reason: &str,
    ) -> Result<AppendResult, V3Error> {
        self.require_open_session(project_id, task_id, session_id)?;
        if reason.trim().is_empty() {
            return Err(session_error(
                "V3_SESSION_GAP_REASON_REQUIRED",
                "session gap requires a reason",
            ));
        }
        self.append_session_event(
            "session.gap_detected",
            project_id,
            task_id,
            session_id,
            actor,
            expected_version,
            idempotency_key,
            json!({"reason":reason}),
        )
    }

    pub fn session_recover(
        &self,
        project_id: &str,
        task_id: &str,
        session_id: &str,
        actor: &str,
        expected_version: u64,
        idempotency_key: &str,
        recovery_evidence: Vec<String>,
    ) -> Result<AppendResult, V3Error> {
        let facts = self.session_facts(project_id, session_id)?;
        if facts.task_id.as_deref() != Some(task_id) || !facts.gapped {
            return Err(session_error(
                "V3_SESSION_NOT_GAPPED",
                "only a gapped session may be recovered",
            ));
        }
        if recovery_evidence.is_empty() {
            return Err(session_error(
                "V3_SESSION_RECOVERY_EVIDENCE_REQUIRED",
                "session recovery requires evidence",
            ));
        }
        self.append_session_event(
            "session.recovered",
            project_id,
            task_id,
            session_id,
            actor,
            expected_version,
            idempotency_key,
            json!({"evidence_refs":recovery_evidence}),
        )
    }

    pub fn rebuild(&self, project_id: &str) -> Result<V3Projection, V3Error> {
        self.store.rebuild_projection(project_id)
    }

    /// Check projection staleness and rebuild if stale.
    ///
    /// Returns Ok(true) if a rebuild was performed, Ok(false) if the projection
    /// was already fresh. Errors from the staleness check or rebuild are
    /// propagated.
    pub fn sync_projection_if_stale(&self, project_id: &str) -> Result<bool, V3Error> {
        match self.store.projection_is_stale(project_id) {
            Ok(false) => Ok(false),
            Ok(true) => self
                .rebuild(project_id)
                .map(|_| true)
                .map_err(|error| {
                    if error.code == "V3_PROJECTION_REBUILD_FAILED" {
                        error
                    } else {
                        projection_sync_error(project_id, error)
                    }
                }),
            Err(error) => Err(projection_sync_error(project_id, error)),
        }
    }

    /// Return projection staleness status as JSON.
    pub fn projection_status(&self, project_id: &str) -> Result<Value, V3Error> {
        self.store.projection_status(project_id)
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
        if self.task_workflow_profile(&command.identity.task_id)? == "lightweight" {
            return Err(V3Error::new(
                "V3_LIGHTWEIGHT_PLAN_FORBIDDEN",
                super::domain::V3ErrorCategory::Validation,
                false,
                "lightweight tasks do not create plan nodes; record the minimal session/result flow instead",
            ));
        }
        self.lifecycle_command(command.into())
    }

    pub fn plan_set_dependencies(
        &self,
        command: PlanSetDependenciesCommand,
    ) -> Result<AppendResult, V3Error> {
        self.lifecycle_command(command.into())
    }

    pub fn plan_set_criteria(
        &self,
        command: super::PlanSetCriteriaCommand,
    ) -> Result<AppendResult, V3Error> {
        self.lifecycle_command(command.into())
    }

    pub fn plan_set_state(&self, command: PlanSetStateCommand) -> Result<AppendResult, V3Error> {
        self.lifecycle_command(command.into())
    }

    pub fn upgrade_task_policy(
        &self,
        project_id: &str,
        task_id: &str,
        actor: &str,
        target_profile: &str,
        reason: &str,
        expected_version: u64,
        idempotency_key: &str,
    ) -> Result<AppendResult, V3Error> {
        let lifecycle = self.task_lifecycle(project_id, task_id)?;
        let current = lifecycle.execution_policy.or_else(|| self.task_execution_policy(task_id).ok()).ok_or_else(|| V3Error::new("V3_EXECUTION_POLICY_MISSING", super::domain::V3ErrorCategory::Validation, false, "task has no effective execution policy; repair legacy coverage before runtime upgrade"))?;
        let upgraded = super::upgrade_policy(&current, target_profile, reason, actor)?;
        self.lifecycle_command(LifecycleCommand {
            event_type: "task.policy_upgraded".to_owned(), project_id: project_id.to_owned(), task_id: task_id.to_owned(), node_id: None, session_id: None,
            actor: actor.to_owned(), expected_version, idempotency_key: idempotency_key.to_owned(), evidence_grade: Some(EvidenceGrade::AgentReported),
            payload: json!({"execution_policy": upgraded, "reason": reason, "target_profile": target_profile}),
        })
    }

    pub fn task_lifecycle(
        &self,
        project_id: &str,
        task_id: &str,
    ) -> Result<TaskLifecycleProjection, V3Error> {
        let events = self.store.load_project(project_id)?;
        Ok(super::lifecycle::fold_task(task_id, &events))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn review_criterion(
        &self,
        project_id: &str,
        task_id: &str,
        actor: &str,
        expected_version: u64,
        idempotency_key: &str,
        criterion_id: &str,
        outcome: &str,
        reviewer: &str,
        evidence_refs: Vec<String>,
        details: Value,
    ) -> Result<AppendResult, V3Error> {
        if !matches!(outcome, "passed" | "failed" | "blocked") {
            return Err(V3Error::new(
                "V3_CRITERION_OUTCOME_INVALID",
                super::domain::V3ErrorCategory::Validation,
                false,
                "criterion outcome must be passed, failed, or blocked",
            ));
        }
        let mut payload = json!({
            "criterion_id": criterion_id,
            "reviewer": reviewer,
            "evidence_refs": evidence_refs,
        });
        if !details.is_null() {
            payload["details"] = details;
        }
        self.lifecycle_command(LifecycleCommand {
            event_type: format!("criterion.{outcome}"),
            project_id: project_id.to_owned(),
            task_id: task_id.to_owned(),
            node_id: None,
            session_id: None,
            actor: actor.to_owned(),
            expected_version,
            idempotency_key: idempotency_key.to_owned(),
            evidence_grade: Some(EvidenceGrade::HardObserved),
            payload,
        })
    }

    pub fn propose_task_completion(
        &self,
        project_id: &str,
        task_id: &str,
        actor: &str,
        expected_version: u64,
        idempotency_key: &str,
    ) -> Result<AppendResult, V3Error> {
        let lifecycle = self.task_lifecycle(project_id, task_id)?;
        let digest = self.validate_completion_gate(project_id, task_id, &lifecycle)?;
        self.lifecycle_command(LifecycleCommand {
            event_type: "task.completion_proposed".to_owned(),
            project_id: project_id.to_owned(),
            task_id: task_id.to_owned(),
            node_id: None,
            session_id: None,
            actor: actor.to_owned(),
            expected_version,
            idempotency_key: idempotency_key.to_owned(),
            evidence_grade: Some(EvidenceGrade::HardObserved),
            payload: json!({"digest": digest, "lifecycle_digest": lifecycle.completion_digest(), "gate_digest_version": 1}),
        })
    }

    pub fn complete_task(
        &self,
        project_id: &str,
        task_id: &str,
        actor: &str,
        confirmed_by: &str,
        channel: &str,
        idempotency_key: &str,
    ) -> Result<AppendResult, V3Error> {
        if !matches!(channel, "desktop_ui" | "cli") {
            return Err(V3Error::new(
                "V3_CONFIRMATION_AUTHENTICITY_REQUIRED",
                super::domain::V3ErrorCategory::Validation,
                false,
                "completion confirmation requires a trusted channel",
            ));
        }
        let events = self.store.load_project(project_id)?;
        let proposal_key = format!("{idempotency_key}.proposal");
        let confirmation_key = format!("{idempotency_key}.confirmation");
        let existing_confirmation = events.iter().find(|event| {
            event.task_id.0 == task_id
                && event.event_type == "task.completion_confirmed"
                && event.idempotency_key == confirmation_key
        });
        if let Some(existing_confirmation) = existing_confirmation {
            let digest = existing_confirmation
                .payload
                .get("digest")
                .and_then(Value::as_str)
                .unwrap_or_default();
            return self.lifecycle_command(LifecycleCommand {
                event_type: "task.completion_confirmed".to_owned(),
                project_id: project_id.to_owned(),
                task_id: task_id.to_owned(),
                node_id: None,
                session_id: None,
                actor: actor.to_owned(),
                expected_version: existing_confirmation.expected_version,
                idempotency_key: confirmation_key,
                evidence_grade: Some(EvidenceGrade::UserConfirmed),
                payload: json!({"digest": digest, "confirmed_by": confirmed_by, "channel": channel}),
            });
        }
        let existing_proposal = events.iter().find(|event| {
            event.task_id.0 == task_id
                && event.event_type == "task.completion_proposed"
                && event.idempotency_key == proposal_key
        });
        let lifecycle = super::lifecycle::fold_task(task_id, &events);
        let pending = lifecycle.confirmation.as_ref().filter(|confirmation| {
            confirmation.valid && confirmation.confirmed_at_version.is_none()
        });
        let (digest, confirmation_expected_version) = if let Some(pending) = pending {
            (pending.digest.clone(), lifecycle.version)
        } else {
            let digest = existing_proposal
                .and_then(|event| event.payload.get("digest"))
                .and_then(Value::as_str)
                .map(str::to_owned)
                .unwrap_or_else(|| lifecycle.completion_digest());
            self.lifecycle_command(LifecycleCommand {
                event_type: "task.completion_proposed".to_owned(),
                project_id: project_id.to_owned(),
                task_id: task_id.to_owned(),
                node_id: None,
                session_id: None,
                actor: actor.to_owned(),
                expected_version: existing_proposal
                    .map(|event| event.expected_version)
                    .unwrap_or(lifecycle.version),
                idempotency_key: proposal_key,
                evidence_grade: Some(EvidenceGrade::HardObserved),
                payload: json!({"digest": digest}),
            })?;
            let proposed = self.task_lifecycle(project_id, task_id)?;
            (digest, proposed.version)
        };
        self.lifecycle_command(LifecycleCommand {
            event_type: "task.completion_confirmed".to_owned(),
            project_id: project_id.to_owned(),
            task_id: task_id.to_owned(),
            node_id: None,
            session_id: None,
            actor: actor.to_owned(),
            expected_version: confirmation_expected_version,
            idempotency_key: confirmation_key,
            evidence_grade: Some(EvidenceGrade::UserConfirmed),
            payload: json!({"digest": digest, "confirmed_by": confirmed_by, "channel": channel}),
        })
    }

    pub fn close_task_with_exceptions(
        &self,
        project_id: &str,
        task_id: &str,
        actor: &str,
        confirmed_by: &str,
        channel: &str,
        reason: &str,
        idempotency_key: &str,
    ) -> Result<AppendResult, V3Error> {
        let lifecycle = self.task_lifecycle(project_id, task_id)?;
        self.lifecycle_command(LifecycleCommand {
            event_type: "task.closed_with_exceptions".to_owned(), project_id: project_id.to_owned(), task_id: task_id.to_owned(),
            node_id: None, session_id: None, actor: actor.to_owned(), expected_version: lifecycle.version,
            idempotency_key: idempotency_key.to_owned(), evidence_grade: Some(EvidenceGrade::UserConfirmed),
            payload: json!({"reason": reason, "confirmed_by": confirmed_by, "channel": channel, "criteria_snapshot": lifecycle.criteria}),
        })
    }

    pub fn orchestration_command(
        &self,
        command: OrchestrationCommand,
    ) -> Result<AppendResult, V3Error> {
        orchestration::apply_command(&self.store, command)
    }

    pub fn memory_command(&self, command: super::MemoryCommand) -> Result<AppendResult, V3Error> {
        super::project_memory::apply_command(&self.store, command)
    }
    pub fn project_memory(&self, project_id: &str) -> Result<super::MemoryProjection, V3Error> {
        Ok(super::project_memory::fold(
            project_id,
            &self.store.load_project(project_id)?,
        ))
    }
    pub fn query_project_memory(
        &self,
        project_id: &str,
        query: &super::MemoryQuery,
    ) -> Result<Vec<super::MemoryEntry>, V3Error> {
        Ok(super::project_memory::query(
            &self.project_memory(project_id)?,
            query,
        ))
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

    fn task_workflow_profile(&self, task_id: &str) -> Result<String, V3Error> {
        #[derive(Deserialize)]
        struct TaskProfile {
            #[serde(default = "default_standard_profile")]
            workflow_profile: String,
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
        let task: TaskProfile = serde_yaml::from_str(&content).map_err(|error| {
            V3Error::new(
                "V3_TASK_INVALID",
                super::domain::V3ErrorCategory::CorruptLog,
                false,
                error.to_string(),
            )
        })?;
        Ok(task.workflow_profile)
    }

    fn task_execution_policy(
        &self,
        task_id: &str,
    ) -> Result<super::EffectiveExecutionPolicy, V3Error> {
        #[derive(Deserialize)]
        struct TaskPolicy {
            execution_policy: super::EffectiveExecutionPolicy,
        }
        let path = self
            .store
            .project_root()
            .join(".vibehub/tasks")
            .join(task_id)
            .join("task.yaml");
        let content = fs::read_to_string(path).map_err(|error| {
            V3Error::new(
                "V3_TASK_READ_FAILED",
                super::domain::V3ErrorCategory::NotFound,
                false,
                error.to_string(),
            )
        })?;
        serde_yaml::from_str::<TaskPolicy>(&content)
            .map(|task| task.execution_policy)
            .map_err(|error| {
                V3Error::new(
                    "V3_EXECUTION_POLICY_MISSING",
                    super::domain::V3ErrorCategory::CorruptLog,
                    false,
                    error.to_string(),
                )
            })
    }

    fn enforced_task_policy(
        &self,
        task_id: &str,
    ) -> Result<Option<super::EffectiveExecutionPolicy>, V3Error> {
        match self.task_execution_policy(task_id) {
            Ok(policy) if policy.policy_version > 0 && !policy.enforcement_epoch.is_empty() => {
                Ok(Some(policy))
            }
            Ok(_) => Ok(None),
            Err(error)
                if matches!(
                    error.code.as_str(),
                    "V3_EXECUTION_POLICY_MISSING" | "V3_TASK_READ_FAILED"
                ) =>
            {
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    fn session_facts(&self, project_id: &str, session_id: &str) -> Result<SessionFacts, V3Error> {
        let mut facts = SessionFacts::default();
        for event in self
            .store
            .load_project(project_id)?
            .into_iter()
            .filter(|event| event.aggregate_id == session_id)
        {
            facts.exists = true;
            facts.task_id = Some(event.task_id.0);
            if event.event_type == "session.opened" {
                facts.open = true;
                facts.node_id = event.node_id.map(|id| id.0);
            }
            if matches!(
                event.event_type.as_str(),
                "session.task_bound" | "session.task_unbound"
            ) {
                facts.binding = event
                    .payload
                    .get("binding")
                    .cloned()
                    .and_then(|value| serde_json::from_value::<SessionTaskBinding>(value).ok());
            }
            if event.event_type == "session.closed" {
                facts.open = false;
                facts.closed = true;
            }
            if event.event_type == "session.gap_detected" {
                facts.open = false;
                facts.gapped = true;
            }
            if event.event_type == "session.recovered" {
                facts.open = true;
                facts.gapped = false;
            }
            if event.event_type == "agent.result_recorded" {
                facts.terminal_result = matches!(
                    event.payload.get("status").and_then(Value::as_str),
                    Some("succeeded" | "failed")
                );
            }
        }
        Ok(facts)
    }

    fn validate_binding_target(&self, project_id: &str, task_id: &str) -> Result<(), V3Error> {
        let lifecycle = self.task_lifecycle(project_id, task_id)?;
        if matches!(
            lifecycle.state.as_str(),
            "completed" | "cancelled" | "closed_with_exceptions" | "superseded"
        ) {
            return Err(V3Error::new(
                "V3_TASK_BINDING_MISMATCH",
                super::domain::V3ErrorCategory::StaleResource,
                false,
                "cannot bind a session to a terminal Task",
            )
            .with_detail("task_id", task_id.to_owned())
            .with_detail("task_state", lifecycle.state));
        }
        Ok(())
    }

    fn require_session_binding(
        &self,
        project_id: &str,
        task_id: &str,
        session_id: &str,
        expected_binding_revision: Option<u64>,
    ) -> Result<SessionTaskBinding, V3Error> {
        let facts = self.session_facts(project_id, session_id)?;
        let binding = facts.binding.ok_or_else(|| {
            V3Error::new(
                "V3_TASK_BINDING_REQUIRED",
                super::domain::V3ErrorCategory::PermissionDenied,
                false,
                "task-scoped write requires an explicit Session–Task binding",
            )
            .with_detail("session_id", session_id.to_owned())
            .with_detail("requested_task_id", task_id.to_owned())
            .with_detail(
                "repair_action",
                "call session_task_bind after confirming the target Task",
            )
        })?;
        if binding.project_id != project_id
            || binding.session_id != session_id
            || binding.bound_task_id.as_deref() != Some(task_id)
        {
            return Err(V3Error::new(
                "V3_TASK_BINDING_MISMATCH",
                super::domain::V3ErrorCategory::ScopeMismatch,
                false,
                "Session–Task binding does not match the requested project, session, or Task",
            )
            .with_detail(
                "bound_task_id",
                binding.bound_task_id.clone().unwrap_or_default(),
            )
            .with_detail("requested_task_id", task_id.to_owned())
            .with_detail("binding_revision", binding.binding_revision));
        }
        if binding.status != BindingStatus::Bound || binding.freshness != BindingFreshness::Fresh {
            return Err(V3Error::new(
                "V3_TASK_BINDING_STALE",
                super::domain::V3ErrorCategory::StaleResource,
                false,
                "Session–Task binding is stale or invalid; rebind before writing",
            )
            .with_detail(
                "binding_status",
                serde_json::to_value(binding.status).unwrap_or(Value::Null),
            )
            .with_detail(
                "binding_freshness",
                serde_json::to_value(binding.freshness).unwrap_or(Value::Null),
            )
            .with_detail(
                "repair_action",
                "call session_task_bind with the current Task",
            ));
        }
        if expected_binding_revision.is_some_and(|expected| expected != binding.binding_revision) {
            return Err(V3Error::new(
                "V3_TASK_BINDING_REVISION_CONFLICT",
                super::domain::V3ErrorCategory::VersionConflict,
                true,
                "Session–Task binding revision is stale",
            )
            .with_detail(
                "expected_binding_revision",
                expected_binding_revision.unwrap_or(0),
            )
            .with_detail("current_binding_revision", binding.binding_revision));
        }
        self.validate_binding_target(project_id, task_id)?;
        Ok(binding)
    }
    fn require_open_session(
        &self,
        project_id: &str,
        task_id: &str,
        session_id: &str,
    ) -> Result<SessionFacts, V3Error> {
        let facts = self.session_facts(project_id, session_id)?;
        self.require_session_binding(project_id, task_id, session_id, None)?;
        if !facts.open || facts.closed || facts.gapped {
            return Err(session_error(
                "V3_SESSION_NOT_OPEN",
                "session event requires an open non-gapped session",
            ));
        }
        Ok(facts)
    }

    fn validate_completion_gate(
        &self,
        project_id: &str,
        task_id: &str,
        lifecycle: &TaskLifecycleProjection,
    ) -> Result<String, V3Error> {
        let policy = lifecycle
            .execution_policy
            .clone()
            .or_else(|| self.enforced_task_policy(task_id).ok().flatten());
        let required = self.required_criterion_ids(task_id)?;
        if !lifecycle.required_criteria_passed(&required)
            || lifecycle.has_open_findings()
            || lifecycle
                .attempts
                .values()
                .any(|attempt| attempt.state == "started")
        {
            return Err(session_error(
                "V3_COMPLETION_REVIEW_GATE_FAILED",
                "criteria, findings, or attempts are not terminal and evidenced",
            ));
        }
        if policy
            .as_ref()
            .is_some_and(|policy| policy.planning_required)
        {
            if lifecycle.effective_nodes().next().is_none()
                || lifecycle
                    .effective_nodes()
                    .any(|(_, node)| !is_terminal_plan_node_state(node.state.as_str()))
            {
                return Err(session_error(
                    "V3_COMPLETION_PLAN_GATE_FAILED",
                    "all required plan nodes must be terminal",
                ));
            }
            let covered = lifecycle
                .effective_nodes()
                .filter(|(_, node)| plan_node_state_covers_criteria(node.state.as_str()))
                .flat_map(|(_, node)| node.criterion_ids.iter().cloned())
                .collect::<BTreeSet<_>>();
            if !required.is_subset(&covered) {
                return Err(session_error(
                    "V3_COMPLETION_CRITERION_COVERAGE_MISSING",
                    "every required criterion must be linked to a plan node",
                ));
            }
        }
        let projection = self.rebuild_in_memory(project_id)?;
        let task_sessions = projection
            .sessions
            .iter()
            .filter(|(_, session)| session.task_id == task_id)
            .collect::<Vec<_>>();
        if task_sessions.iter().any(|(_, session)| {
            session.state != "closed"
                || !session
                    .agent_results
                    .iter()
                    .any(|result| matches!(result.status.as_str(), "succeeded" | "failed"))
        }) {
            return Err(session_error(
                "V3_COMPLETION_SESSION_GATE_FAILED",
                "all sessions must be closed with terminal results and no gaps",
            ));
        }
        if policy.as_ref().is_some_and(|policy| {
            policy
                .required_records
                .iter()
                .any(|record| record == "progress")
        }) && task_sessions
            .iter()
            .any(|(_, session)| !projection::session_has_milestone_evidence(session))
        {
            return Err(session_error(
                "V3_COMPLETION_PROGRESS_GATE_FAILED",
                "policy requires progress evidence for each session, or a recorded gap recovery for interrupted sessions",
            ));
        }
        let orchestration = self.worktree_orchestration(project_id, task_id)?;
        if policy
            .as_ref()
            .is_some_and(|policy| policy.effective_profile == "full")
            && orchestration.worktrees.values().any(|worktree| {
                !matches!(
                    worktree.state,
                    super::worktree::WorktreeState::Integrated
                        | super::worktree::WorktreeState::Abandoned
                        | super::worktree::WorktreeState::Cleaned
                ) || worktree
                    .lease
                    .as_ref()
                    .is_some_and(|lease| lease.state == super::LeaseState::Active)
            })
        {
            return Err(session_error(
                "V3_COMPLETION_INTEGRATION_GATE_FAILED",
                "full-profile worktrees, leases, and integrations must be settled",
            ));
        }
        let effective_nodes = lifecycle
            .effective_nodes()
            .map(|(node_id, node)| (node_id.clone(), node.clone()))
            .collect::<BTreeMap<_, _>>();
        let facts = json!({"lifecycle":lifecycle.completion_digest(),"nodes":effective_nodes,"sessions":task_sessions,"worktrees":orchestration.worktrees,"policy":policy,"model":"completion-gate-1"});
        let mut hasher = Sha256::new();
        hasher.update(serde_json::to_vec(&facts).unwrap_or_default());
        Ok(format!("sha256:{:x}", hasher.finalize()))
    }
    fn rebuild_in_memory(&self, project_id: &str) -> Result<V3Projection, V3Error> {
        Ok(projection::fold(
            project_id,
            &self.store.load_project(project_id)?,
        ))
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
        // Binding facts are a new aggregate concern. Historical callers used
        // session versions that counted only opened/log/result events, so a
        // request whose version is exactly behind the number of binding facts
        // is accepted as an auditable compatibility read and normalized to
        // the real event-store version. New MCP/CLI callers resolve the real
        // aggregate version and take the direct branch.
        let events = self.store.load_project(project_id)?;
        let current_version = events
            .iter()
            .filter(|event| event.aggregate_id == session_id)
            .map(|event| event.aggregate_version)
            .max()
            .unwrap_or(0);
        let binding_events = events
            .iter()
            .filter(|event| {
                event.aggregate_id == session_id
                    && matches!(
                        event.event_type.as_str(),
                        "session.task_bound" | "session.task_unbound"
                    )
            })
            .count() as u64;
        let effective_expected_version =
            if expected_version.saturating_add(binding_events) == current_version {
                current_version
            } else {
                expected_version
            };
        self.store.append_with_rebuild(EventDraft {
            event_type: event_type.to_owned(),
            aggregate_id: session_id.to_owned(),
            expected_version: effective_expected_version,
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

fn projection_sync_error(project_id: &str, cause: V3Error) -> V3Error {
    V3Error::new(
        "V3_PROJECTION_SYNC_FAILED",
        super::domain::V3ErrorCategory::Internal,
        true,
        "projection synchronization failed; callers must not continue using the old projection",
    )
    .with_detail("project_id", project_id)
    .with_detail("projection_state", "rebuild_failed")
    .with_detail("cause_code", cause.code)
    .with_detail(
        "cause_category",
        serde_json::to_value(cause.category)
            .unwrap_or(Value::String("internal".to_owned())),
    )
    .with_detail("cause_message", cause.message)
    .with_detail("cause_details", cause.details)
    .with_detail(
        "repair_action",
        "fix the projection path or permissions, then retry projection_rebuild or reopen MCP",
    )
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
    use crate::v3::lifecycle::{command, PlanCommandIdentity};
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
    fn projection_sync_failure_is_structured_and_blocks_old_projection() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-sync-failure-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub")).unwrap();
        let app = V3ApplicationService::open(&root).unwrap();
        app.session_open(
            "project.test",
            "task.test",
            "session.main",
            "codex",
            0,
            "open.sync-failure",
        )
        .unwrap();

        let projection_path = app.store.projection_path("project.test");
        fs::remove_file(&projection_path).unwrap();
        fs::create_dir(&projection_path).unwrap();

        let error = app
            .sync_projection_if_stale("project.test")
            .unwrap_err();
        assert_eq!(error.code, "V3_PROJECTION_SYNC_FAILED");
        assert_eq!(error.details["project_id"], "project.test");
        assert_eq!(error.details["projection_state"], "rebuild_failed");
        assert_eq!(error.details["cause_code"], "V3_PROJECTION_READ_FAILED");
        assert!(error.details["repair_action"].as_str().is_some());

        let status = app.projection_status("project.test").unwrap();
        assert_eq!(status["state"], "rebuild_failed");
        assert_eq!(status["stale"], true);
        assert!(status["error"].is_object());

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

    fn write_planning_task(root: &Path, criteria: &[&str]) {
        fs::create_dir_all(root.join(".vibehub/tasks/task.test")).unwrap();
        let acceptance = criteria
            .iter()
            .map(|criterion| format!("- {criterion}\n"))
            .collect::<String>();
        fs::write(
            root.join(".vibehub/tasks/task.test/task.yaml"),
            format!(
                "task_id: task.test\ntitle: Planning task\nintent: Verify plan completion gates\nphase: implement\nphase_status: active\nacceptance_criteria:\n{acceptance}workflow_profile: standard\nexecution_policy:\n  recommended_profile: standard\n  effective_profile: standard\n  policy_version: 1\n  enforcement_epoch: v3.1-hard-closure\n  trigger_reasons:\n  - multiple_verifiable_milestones\n  milestone_policy: standard\n  planning_required: true\n  review_required: true\n  required_records:\n  - plan\n  - session\n  - progress\n  - result\n  - review\n  upgrade_history: []\n"
            ),
        )
        .unwrap();
    }

    fn write_basic_task(root: &Path) {
        fs::create_dir_all(root.join(".vibehub/tasks/task.test")).unwrap();
        fs::write(
            root.join(".vibehub/tasks/task.test/task.yaml"),
            "task_id: task.test\ntitle: Binding task\nintent: Verify Session binding\nphase: implement\nphase_status: active\nacceptance_criteria:\n- Session binding is enforced\n",
        )
        .unwrap();
    }

    #[test]
    fn task_scoped_writes_fail_closed_without_binding_and_after_unbind() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-binding-gate-{}", Uuid::new_v4()));
        write_basic_task(&root);
        let app = V3ApplicationService::open(&root).unwrap();

        let unbound_error = app
            .event_log(
                "progress",
                "project.test",
                "task.test",
                "session.unbound",
                "codex",
                0,
                "progress.unbound",
                json!({"summary": "must be rejected"}),
            )
            .unwrap_err();
        assert_eq!(unbound_error.code, "V3_TASK_BINDING_REQUIRED");

        app.session_task_bind(
            "project.test",
            "task.test",
            "session.bound",
            "interaction.bound",
            "codex",
            BindingSource::UserConfirmed,
            0,
            "bind.1",
            None,
            Some("agent.codex".to_owned()),
            Some("codex".to_owned()),
            None,
        )
        .unwrap();
        let binding = app
            .session_task_binding("project.test", "session.bound")
            .unwrap();
        assert_eq!(binding.status, BindingStatus::Bound);
        assert_eq!(binding.binding_revision, 1);

        let revision_error = app
            .validate_task_binding("project.test", "task.test", "session.bound", Some(0))
            .unwrap_err();
        assert_eq!(revision_error.code, "V3_TASK_BINDING_REVISION_CONFLICT");

        app.session_open_with_context(
            "project.test",
            "task.test",
            "session.bound",
            "codex",
            1,
            "open.1",
            None,
            None,
            None,
        )
        .unwrap();
        app.event_log(
            "progress",
            "project.test",
            "task.test",
            "session.bound",
            "codex",
            2,
            "progress.bound",
            json!({"summary": "binding verified"}),
        )
        .unwrap();

        app.session_task_unbind(
            "project.test",
            "task.test",
            "session.bound",
            "codex",
            3,
            "unbind.1",
            Some(1),
        )
        .unwrap();
        let unbound = app
            .session_task_binding("project.test", "session.bound")
            .unwrap();
        assert_eq!(unbound.status, BindingStatus::Unbound);
        assert_eq!(unbound.binding_revision, 2);
        let after_unbind_error = app
            .event_log(
                "progress",
                "project.test",
                "task.test",
                "session.bound",
                "codex",
                4,
                "progress.after-unbind",
                json!({"summary": "must be rejected"}),
            )
            .unwrap_err();
        assert_eq!(after_unbind_error.code, "V3_TASK_BINDING_MISMATCH");

        let projection = app.rebuild("project.test").unwrap();
        assert_eq!(
            projection.session_bindings["session.bound"].status,
            BindingStatus::Unbound
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn task_scoped_binding_rejects_a_different_task() {
        let root =
            std::env::temp_dir().join(format!("vibehub-v3-binding-mismatch-{}", Uuid::new_v4()));
        write_basic_task(&root);
        let app = V3ApplicationService::open(&root).unwrap();
        app.session_task_bind(
            "project.test",
            "task.test",
            "session.bound",
            "interaction.bound",
            "codex",
            BindingSource::ExplicitTaskId,
            0,
            "bind.1",
            None,
            None,
            Some("codex".to_owned()),
            None,
        )
        .unwrap();
        let error = app
            .validate_task_binding("project.test", "task.other", "session.bound", Some(1))
            .unwrap_err();
        assert_eq!(error.code, "V3_TASK_BINDING_MISMATCH");
        fs::remove_dir_all(root).unwrap();
    }

    fn add_planning_node(
        app: &V3ApplicationService,
        version: u64,
        node_id: &str,
        criterion_ids: Vec<String>,
    ) {
        app.plan_add_node(PlanAddNodeCommand {
            identity: PlanCommandIdentity {
                project_id: "project.test".to_owned(),
                task_id: "task.test".to_owned(),
                actor: "codex".to_owned(),
                expected_version: version,
                idempotency_key: format!("plan.add.{node_id}"),
            },
            node_id: node_id.to_owned(),
            title: node_id.to_owned(),
            goal: "verify plan terminal gate".to_owned(),
            scope: Vec::new(),
            dependencies: Vec::new(),
            criterion_ids,
        })
        .unwrap();
    }

    fn set_planning_node_state(
        app: &V3ApplicationService,
        version: u64,
        node_id: &str,
        state: &str,
    ) {
        app.plan_set_state(PlanSetStateCommand {
            identity: PlanCommandIdentity {
                project_id: "project.test".to_owned(),
                task_id: "task.test".to_owned(),
                actor: "codex".to_owned(),
                expected_version: version,
                idempotency_key: format!("plan.state.{node_id}.{state}"),
            },
            node_id: node_id.to_owned(),
            state: state.to_owned(),
        })
        .unwrap();
    }

    #[test]
    fn completion_plan_gate_accepts_cancelled_nodes_as_terminal() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-cancelled-{}", Uuid::new_v4()));
        write_planning_task(&root, &["Only criterion"]);
        let app = V3ApplicationService::open(&root).unwrap();

        add_planning_node(
            &app,
            0,
            "node.done",
            vec!["criterion.task.test.c01".to_owned()],
        );
        add_planning_node(&app, 1, "node.dropped", Vec::new());
        set_planning_node_state(&app, 2, "node.done", "active");
        set_planning_node_state(&app, 3, "node.done", "completed");
        set_planning_node_state(&app, 4, "node.dropped", "cancelled");
        app.review_criterion(
            "project.test",
            "task.test",
            "codex",
            5,
            "criterion.pass.1",
            "criterion.task.test.c01",
            "passed",
            "reviewer",
            vec!["test:review".to_owned()],
            Value::Null,
        )
        .unwrap();

        let lifecycle = app.task_lifecycle("project.test", "task.test").unwrap();
        assert_eq!(lifecycle.nodes["node.dropped"].state, "cancelled");
        app.propose_task_completion("project.test", "task.test", "codex", 6, "proposal.1")
            .unwrap();
        let proposed = app.task_lifecycle("project.test", "task.test").unwrap();
        assert_eq!(proposed.state, "completion_pending");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cancelled_nodes_do_not_cover_required_criteria() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-coverage-{}", Uuid::new_v4()));
        write_planning_task(&root, &["Only criterion"]);
        let app = V3ApplicationService::open(&root).unwrap();

        add_planning_node(
            &app,
            0,
            "node.dropped",
            vec!["criterion.task.test.c01".to_owned()],
        );
        set_planning_node_state(&app, 1, "node.dropped", "cancelled");
        app.review_criterion(
            "project.test",
            "task.test",
            "codex",
            2,
            "criterion.pass.1",
            "criterion.task.test.c01",
            "passed",
            "reviewer",
            vec!["test:review".to_owned()],
            Value::Null,
        )
        .unwrap();

        let error = app
            .propose_task_completion("project.test", "task.test", "codex", 3, "proposal.1")
            .unwrap_err();
        assert_eq!(error.code, "V3_COMPLETION_CRITERION_COVERAGE_MISSING");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn blocked_session_with_risk_and_failed_result_satisfies_progress_gate() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-risk-{}", Uuid::new_v4()));
        write_planning_task(&root, &["Only criterion"]);
        let app = V3ApplicationService::open(&root).unwrap();

        add_planning_node(
            &app,
            0,
            "node.done",
            vec!["criterion.task.test.c01".to_owned()],
        );
        set_planning_node_state(&app, 1, "node.done", "active");
        app.session_open_with_context(
            "project.test",
            "task.test",
            "session.blocked",
            "codex",
            0,
            "open.blocked",
            Some("/tmp/vibehub-test".to_owned()),
            Some("node.done".to_owned()),
            None,
        )
        .unwrap();
        app.event_log(
            "risk",
            "project.test",
            "task.test",
            "session.blocked",
            "codex",
            1,
            "risk.blocked",
            json!({"summary": "blocked by upstream defect"}),
        )
        .unwrap();
        app.agent_result_record(
            "project.test",
            "task.test",
            "session.blocked",
            "codex",
            2,
            "result.blocked",
            "result.test",
            Some("node.done".to_owned()),
            json!({
                "kind": "execution",
                "request_source": "user_request",
                "instruction": "Fix the defect",
                "status": "failed",
                "summary": "Blocked by upstream defect"
            }),
        )
        .unwrap();
        app.session_close(
            "project.test",
            "task.test",
            "session.blocked",
            "codex",
            3,
            "close.blocked",
        )
        .unwrap();
        set_planning_node_state(&app, 2, "node.done", "completed");
        app.review_criterion(
            "project.test",
            "task.test",
            "codex",
            3,
            "criterion.pass.1",
            "criterion.task.test.c01",
            "passed",
            "reviewer",
            vec!["test:review".to_owned()],
            Value::Null,
        )
        .unwrap();

        app.propose_task_completion("project.test", "task.test", "codex", 4, "proposal.1")
            .unwrap();
        let proposed = app.task_lifecycle("project.test", "task.test").unwrap();
        assert_eq!(proposed.state, "completion_pending");
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
    fn complete_task_is_all_green_and_idempotent() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-complete-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub/tasks/task.test")).unwrap();
        fs::write(
            root.join(".vibehub/tasks/task.test/task.yaml"),
            "task_id: task.test\ntitle: Complete task\nintent: Finish without another agent run\nphase: execute\nphase_status: active\nacceptance_criteria:\n- Review passes\n",
        )
        .unwrap();
        let app = V3ApplicationService::open(&root).unwrap();
        app.lifecycle_command(command(
            "criterion.passed",
            "project.test",
            "task.test",
            0,
            "criterion.pass.1",
            json!({"criterion_id":"criterion.task.test.c01","reviewer":"reviewer","evidence_refs":["test:review"]}),
        ))
        .unwrap();

        let first = app
            .complete_task(
                "project.test",
                "task.test",
                "desktop-user",
                "ChenM0M",
                "desktop_ui",
                "complete.1",
            )
            .unwrap();
        let second = app
            .complete_task(
                "project.test",
                "task.test",
                "desktop-user",
                "ChenM0M",
                "desktop_ui",
                "complete.1",
            )
            .unwrap();
        assert!(matches!(first, AppendResult::Appended { .. }));
        assert!(matches!(second, AppendResult::Duplicate { .. }));
        let lifecycle = app.task_lifecycle("project.test", "task.test").unwrap();
        assert_eq!(lifecycle.state, "completed");
        assert_eq!(lifecycle.version, 3);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn complete_task_confirms_the_existing_pending_proposal() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-confirm-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub/tasks/task.test")).unwrap();
        fs::write(
            root.join(".vibehub/tasks/task.test/task.yaml"),
            "task_id: task.test\ntitle: Confirm task\nintent: Confirm the reviewed truth\nphase: execute\nphase_status: active\nacceptance_criteria:\n- Review passes\n",
        )
        .unwrap();
        let app = V3ApplicationService::open(&root).unwrap();
        app.review_criterion(
            "project.test",
            "task.test",
            "codex",
            0,
            "criterion.pass.1",
            "criterion.task.test.c01",
            "passed",
            "reviewer",
            vec!["test:review".to_owned()],
            Value::Null,
        )
        .unwrap();
        app.propose_task_completion("project.test", "task.test", "codex", 1, "proposal.1")
            .unwrap();
        let proposed = app.task_lifecycle("project.test", "task.test").unwrap();
        let proposed_digest = proposed.confirmation.as_ref().unwrap().digest.clone();

        app.complete_task(
            "project.test",
            "task.test",
            "codex",
            "ChenM0M",
            "cli",
            "complete.1",
        )
        .unwrap();

        let completed = app.task_lifecycle("project.test", "task.test").unwrap();
        assert_eq!(completed.state, "completed");
        assert_eq!(completed.version, 3);
        assert_eq!(completed.confirmation.unwrap().digest, proposed_digest);
        let events = app.store.load_project("project.test").unwrap();
        assert_eq!(
            events
                .iter()
                .filter(|event| event.event_type == "task.completion_proposed")
                .count(),
            1
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn forced_closure_rejects_blank_reason() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-force-close-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub/tasks/task.test")).unwrap();
        fs::write(
            root.join(".vibehub/tasks/task.test/task.yaml"),
            "task_id: task.test\ntitle: Force close\nintent: Require a reason\nphase: execute\nphase_status: active\n",
        )
        .unwrap();
        let error = V3ApplicationService::open(&root)
            .unwrap()
            .close_task_with_exceptions(
                "project.test",
                "task.test",
                "desktop-user",
                "ChenM0M",
                "desktop_ui",
                "   ",
                "closure.blank",
            )
            .unwrap_err();
        assert_eq!(error.code, "V3_FORCE_CLOSE_CONFIRMATION_REQUIRED");
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

use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Implementation, ListResourcesResult, PaginatedRequestParams,
        ReadResourceRequestParams, ReadResourceResult, Resource, ResourceContents,
        ServerCapabilities, ServerInfo,
    },
    schemars, tool, tool_handler, tool_router, ServerHandler, ServiceExt,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use vibehub_core::v3::{
    assess_root_alignment, inspect_mcp_host_configs, read_project_settings, resolve_project_scopes,
    AgentSpecTarget, LifecycleCommand, MemoryCommand, MemoryEntry, MemoryQuery,
    OrchestrationCommand, PlanAddNodeCommand, PlanCommandIdentity, PlanSetCriteriaCommand,
    PlanSetDependenciesCommand, PlanSetStateCommand, ProjectScopeInspection, ResolvedProjectScopes,
    V3ApplicationService, V3Error, V3ErrorCategory, V3ViewRepository,
};

const RESOURCE_PREFIX: &str = "vibehub://v3/1.0";

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct SessionWrite {
    project_id: String,
    task_id: String,
    session_id: String,
    actor: String,
    #[serde(default)]
    expected_version: Option<u64>,
    #[serde(default)]
    idempotency_key: Option<String>,
    #[serde(default)]
    working_directory: Option<String>,
    #[serde(default)]
    node_id: Option<String>,
    #[serde(default)]
    worktree_id: Option<String>,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    provider_session_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct EventLogWrite {
    project_id: String,
    task_id: String,
    session_id: String,
    actor: String,
    #[serde(default)]
    expected_version: Option<u64>,
    #[serde(default)]
    idempotency_key: Option<String>,
    #[schemars(description = "Supported values are progress and risk")]
    kind: String,
    #[serde(default)]
    details: Value,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct AgentResultWrite {
    project_id: String,
    task_id: String,
    session_id: String,
    actor: String,
    #[serde(default)]
    expected_version: Option<u64>,
    #[serde(default)]
    idempotency_key: Option<String>,
    result_id: String,
    #[serde(default)]
    node_id: Option<String>,
    #[serde(default)]
    details: Value,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct PlanWriteScope {
    project_id: String,
    task_id: String,
    actor: String,
    #[serde(default)]
    expected_version: Option<u64>,
    #[serde(default)]
    idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct TaskCandidatesRead {
    project_id: String,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct TaskViewRead {
    task_id: String,
    #[serde(default)]
    node_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct TaskPolicyUpgradeWrite {
    #[serde(flatten)]
    scope: PlanWriteScope,
    target_profile: String,
    reason: String,
}
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct PlanCriteriaSetWrite {
    #[serde(flatten)]
    scope: PlanWriteScope,
    node_id: String,
    criterion_ids: Vec<String>,
}
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct FindingWrite {
    #[serde(flatten)]
    scope: PlanWriteScope,
    action: String,
    finding_id: String,
    #[serde(default)]
    node_id: Option<String>,
    #[serde(default)]
    severity: Option<String>,
    #[serde(default)]
    evidence_refs: Vec<String>,
    #[serde(default)]
    details: Value,
}
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct AttemptWrite {
    #[serde(flatten)]
    scope: PlanWriteScope,
    action: String,
    attempt_id: String,
    finding_id: String,
    #[serde(default)]
    node_id: Option<String>,
    #[serde(default)]
    evidence_refs: Vec<String>,
    #[serde(default)]
    details: Value,
}
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct SessionRecoveryWrite {
    #[serde(flatten)]
    session: SessionWrite,
    action: String,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    evidence_refs: Vec<String>,
}
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct MemoryWrite {
    project_id: String,
    task_id: String,
    actor: String,
    action: String,
    entry_id: String,
    expected_revision: u64,
    #[serde(default)]
    idempotency_key: Option<String>,
    #[serde(default)]
    entry: Option<Value>,
    #[serde(default)]
    details: Value,
}
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct MemoryQueryRead {
    project_id: String,
    #[serde(default)]
    kinds: Vec<String>,
    #[serde(default)]
    scope: Vec<String>,
    #[serde(default)]
    principal_scope: Option<String>,
    #[serde(default)]
    include_stale: bool,
    #[serde(default)]
    include_disputed: bool,
    #[serde(default = "default_memory_budget")]
    token_budget: usize,
}
fn default_memory_budget() -> usize {
    2000
}
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct OrchestrationWrite {
    #[serde(flatten)]
    scope: PlanWriteScope,
    event_type: String,
    node_id: String,
    worktree_id: String,
    eligibility_digest: String,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    lease_id: Option<String>,
    #[serde(default)]
    operation_id: Option<String>,
    #[serde(default)]
    lease_generation: Option<u64>,
    #[serde(default)]
    details: Value,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct CriterionReviewWrite {
    #[serde(flatten)]
    scope: PlanWriteScope,
    criterion_id: String,
    #[schemars(description = "Review outcome: passed, failed, or blocked")]
    outcome: String,
    reviewer: String,
    evidence_refs: Vec<String>,
    #[serde(default)]
    details: Value,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct TaskCompletionWrite {
    #[serde(flatten)]
    scope: PlanWriteScope,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct TaskCompleteWrite {
    project_id: String,
    task_id: String,
    actor: String,
    confirmed_by: String,
    #[schemars(
        description = "Trusted confirmation channel; MCP Agents must use cli only after explicit current-user confirmation"
    )]
    channel: String,
    #[serde(default)]
    expected_version: Option<u64>,
    #[serde(default)]
    idempotency_key: Option<String>,
}

fn resolve_and_append<T, F>(
    app: &V3ApplicationService,
    project_id: &str,
    aggregate_id: &str,
    expected_version: Option<u64>,
    idempotency_key: Option<String>,
    mut append: F,
) -> Result<T, V3Error>
where
    F: FnMut(u64, &str) -> Result<T, V3Error>,
{
    let auto_version = expected_version.is_none();
    let key = idempotency_key.unwrap_or_else(|| format!("auto.{}", Uuid::new_v4()));
    let version = expected_version.unwrap_or(app.aggregate_version(project_id, aggregate_id)?);
    match append(version, &key) {
        Err(error) if auto_version && error.category == V3ErrorCategory::VersionConflict => {
            let current_version = app.aggregate_version(project_id, aggregate_id)?;
            append(current_version, &key)
        }
        result => result,
    }
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct PlanNodeAddWrite {
    #[serde(flatten)]
    identity: PlanWriteScope,
    node_id: String,
    title: String,
    goal: String,
    #[serde(default)]
    scope: Vec<String>,
    #[serde(default)]
    dependencies: Vec<String>,
    #[serde(default)]
    criterion_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct PlanDependenciesSetWrite {
    #[serde(flatten)]
    scope: PlanWriteScope,
    node_id: String,
    dependencies: Vec<String>,
    #[serde(default)]
    change_mode: Option<String>,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct PlanNodeStateSetWrite {
    #[serde(flatten)]
    scope: PlanWriteScope,
    node_id: String,
    #[schemars(
        description = "Target state: ready, active, blocked, completed, failed, or cancelled"
    )]
    state: String,
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
struct ToolResponse {
    ok: bool,
    result: Value,
}

#[derive(Debug, Clone)]
pub struct V3McpServer {
    project_root: PathBuf,
    app: V3ApplicationService,
    views: V3ViewRepository,
    project_id: String,
    scopes: ProjectScopeInspection,
    resolved_scopes: ResolvedProjectScopes,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl V3McpServer {
    fn open(project_root: impl AsRef<Path>) -> Result<Self, V3Error> {
        let resolved_scopes = resolve_project_scopes(project_root.as_ref(), None)?;
        let project_root = resolved_scopes.control_root.clone();
        let scopes = resolved_scopes.inspection();
        let views = V3ViewRepository::open(&project_root)?;
        let project_id = views.project_id();
        // Validate that a current V3 task exists at startup, but resolve it again
        // for every resource request so a long-lived MCP process cannot advertise
        // or read a stale task after the current pointer changes.
        let _ = views.current_task_id()?;
        Ok(Self {
            app: V3ApplicationService::open(&project_root)?,
            views,
            project_id,
            project_root,
            scopes,
            resolved_scopes,
            tool_router: Self::tool_router(),
        })
    }

    #[tool(
        description = "Open a VibeHub V3 agent session to start recording execution facts. When: right before you begin implementing, after task_view and (for standard/full) after plan_node_state_set moves the target node to active. Prerequisite: know the task_id and the real working_directory; pass node_id for standard/full. Typical params: project_id, task_id, session_id (a stable id you choose), actor, working_directory, node_id. expected_version and idempotency_key are optional and auto-resolved when omitted; explicitly provided values are strictly validated"
    )]
    fn session_open(&self, Parameters(input): Parameters<SessionWrite>) -> CallToolResult {
        let SessionWrite {
            project_id,
            task_id,
            session_id,
            actor,
            expected_version,
            idempotency_key,
            working_directory,
            node_id,
            worktree_id,
            provider,
            provider_session_id,
        } = input;
        self.tool_result(resolve_and_append(
            &self.app,
            &project_id,
            &session_id,
            expected_version,
            idempotency_key,
            |version, key| {
                self.app.session_open_with_context_and_provider(
                    &project_id,
                    &task_id,
                    &session_id,
                    &actor,
                    version,
                    key,
                    working_directory.clone(),
                    node_id.clone(),
                    worktree_id.clone(),
                    provider.clone(),
                    provider_session_id.clone(),
                )
            },
        ))
    }

    #[tool(
        description = "Discover active V3 task candidates and their workflow, risk, criteria, session, and relation summaries. When: call this first for every task, before task_view or any write tool. Prerequisite: a connected V3 workspace and its project_id. Typical params: project_id. Use the returned task_id to call task_view; do not infer the current task from files or chat"
    )]
    fn task_candidates(&self, Parameters(input): Parameters<TaskCandidatesRead>) -> CallToolResult {
        if input.project_id != self.project_id {
            return tool_error(
                json!({"code":"V3_PROJECT_MISMATCH","message":"project_id does not match this MCP workspace"}),
            );
        }
        let result = self
            .views
            .current_task_id()
            .and_then(|task_id| self.views.load_bundle(&task_id))
            .map(|bundle| {
                bundle
                    .project_overview
                    .get("active_tasks")
                    .cloned()
                    .unwrap_or_else(|| json!([]))
            });
        self.tool_result(result)
    }

    #[tool(
        description = "Read a complete V3 view bundle for a specified task candidate. When: immediately after task_candidates and before planning, opening a session, or editing files. Prerequisite: a task_id returned by task_candidates. Typical params: task_id. Read and echo node_brief.workflow_profile plus node_brief.execution_policy (planning_required, milestone_policy, review_required, required_records), then obey them"
    )]
    fn task_view(&self, Parameters(input): Parameters<TaskViewRead>) -> CallToolResult {
        self.tool_result(
            self.views
                .load_bundle_for_node(&input.task_id, input.node_id.as_deref()),
        )
    }

    #[tool(
        description = "Record the evidence-backed review outcome for one accepted criterion. When: after implementation, run the real validation for each required criterion, then call this once per criterion to move it from accepted to passed/failed/blocked. Prerequisite: real validation already executed and its output captured as evidence. Typical params: project_id, task_id, actor, criterion_id, outcome (passed|failed|blocked), reviewer, evidence_refs (paths/commands/logs). Run the real validation first; accepted only means registered, not passed. expected_version and idempotency_key are optional and auto-resolved when omitted"
    )]
    fn criterion_review(
        &self,
        Parameters(input): Parameters<CriterionReviewWrite>,
    ) -> CallToolResult {
        let CriterionReviewWrite {
            scope,
            criterion_id,
            outcome,
            reviewer,
            evidence_refs,
            details,
        } = input;
        let PlanWriteScope {
            project_id,
            task_id,
            actor,
            expected_version,
            idempotency_key,
        } = scope;
        self.tool_result(resolve_and_append(
            &self.app,
            &project_id,
            &task_id,
            expected_version,
            idempotency_key,
            |version, key| {
                self.app.review_criterion(
                    &project_id,
                    &task_id,
                    &actor,
                    version,
                    key,
                    &criterion_id,
                    &outcome,
                    &reviewer,
                    evidence_refs.clone(),
                    details.clone(),
                )
            },
        ))
    }

    #[tool(
        description = "Move an all-green task to completion_pending and ask the user for explicit confirmation. When: only after every required criterion_review is passed with evidence, findings are closed, any planning_required node is completed, agent_result_record succeeded, and session_close finished. Prerequisite: all review and execution gates declared by execution_policy are already green; never use this to bypass them. Typical params: project_id, task_id, actor. Do not leave an all-green task in review"
    )]
    fn task_completion_propose(
        &self,
        Parameters(input): Parameters<TaskCompletionWrite>,
    ) -> CallToolResult {
        let PlanWriteScope {
            project_id,
            task_id,
            actor,
            expected_version,
            idempotency_key,
        } = input.scope;
        self.tool_result(resolve_and_append(
            &self.app,
            &project_id,
            &task_id,
            expected_version,
            idempotency_key,
            |version, key| {
                self.app
                    .propose_task_completion(&project_id, &task_id, &actor, version, key)
            },
        ))
    }

    #[tool(
        description = "Confirm and archive an all-green task. When: immediately after the user explicitly agrees in the current trusted interaction to a valid task_completion_propose. Prerequisite: the task is completion_pending and the current user has explicitly confirmed; never infer or fabricate confirmation. Typical params: project_id, task_id, actor, confirmed_by, channel=cli. Do not ask the user to close it manually"
    )]
    fn task_complete(&self, Parameters(input): Parameters<TaskCompleteWrite>) -> CallToolResult {
        let TaskCompleteWrite {
            project_id,
            task_id,
            actor,
            confirmed_by,
            channel,
            expected_version,
            idempotency_key,
        } = input;
        if channel != "cli" {
            return tool_error(json!({
                "code": "V3_CONFIRMATION_AUTHENTICITY_REQUIRED",
                "category": "validation",
                "retryable": false,
                "message": "the MCP task_complete tool accepts cli confirmation only"
            }));
        }
        if confirmed_by.trim().is_empty() {
            return tool_error(json!({
                "code": "V3_CONFIRMATION_AUTHENTICITY_REQUIRED",
                "category": "validation",
                "retryable": false,
                "message": "confirmed_by must identify the user who explicitly confirmed completion"
            }));
        }
        if let Some(expected_version) = expected_version {
            match self.app.aggregate_version(&project_id, &task_id) {
                Ok(current_version) if current_version != expected_version => {
                    return tool_error(json!({
                        "code": "V3_VERSION_CONFLICT",
                        "category": "version_conflict",
                        "retryable": true,
                        "message": "expected task version does not match current lifecycle version",
                        "details": {"expected_version": expected_version, "current_version": current_version}
                    }));
                }
                Err(error) => return self.tool_result::<Value>(Err(error)),
                _ => {}
            }
        }
        let key = idempotency_key.unwrap_or_else(|| format!("auto.{}", Uuid::new_v4()));
        self.tool_result(self.app.complete_task(
            &project_id,
            &task_id,
            &actor,
            &confirmed_by,
            &channel,
            &key,
        ))
    }

    #[tool(
        description = "Record progress or risk evidence for the current session. When: call with kind=progress after every verifiable milestone; call with kind=risk immediately when you hit a blocker, scope drift, version conflict, or evidence gap (do not report these only in chat). Prerequisite: an open session (session_open). Typical params: project_id, task_id, session_id, actor, kind (progress|risk), details ({summary, evidence, node_id}). expected_version and idempotency_key are optional and auto-resolved when omitted; explicitly provided values are strictly validated"
    )]
    fn event_log(&self, Parameters(input): Parameters<EventLogWrite>) -> CallToolResult {
        let EventLogWrite {
            project_id,
            task_id,
            session_id,
            actor,
            expected_version,
            idempotency_key,
            kind,
            details,
        } = input;
        if !matches!(kind.as_str(), "progress" | "risk") {
            return tool_error(json!({
                "code": "V3_VALIDATION_ERROR",
                "category": "validation",
                "retryable": false,
                "message": "kind must be progress or risk"
            }));
        }
        self.tool_result(resolve_and_append(
            &self.app,
            &project_id,
            &session_id,
            expected_version,
            idempotency_key,
            |version, key| {
                self.app.event_log(
                    &kind,
                    &project_id,
                    &task_id,
                    &session_id,
                    &actor,
                    version,
                    key,
                    details.clone(),
                )
            },
        ))
    }

    #[tool(
        description = "Record an Agent execution or evaluation result. When: before you stop or hand off, write the truthful pending/running/succeeded/failed outcome with evidence; call this just before session_close. Prerequisite: an open session and the work's real outcome. Typical params: project_id, task_id, session_id, actor, result_id, node_id, details ({kind: execution|evaluation, request_source: user_request|evaluation_instruction, instruction, status: pending|running|succeeded|failed, summary, evidence}). expected_version and idempotency_key are optional and auto-resolved when omitted; explicitly provided values are strictly validated"
    )]
    fn agent_result_record(
        &self,
        Parameters(input): Parameters<AgentResultWrite>,
    ) -> CallToolResult {
        let AgentResultWrite {
            project_id,
            task_id,
            session_id,
            actor,
            expected_version,
            idempotency_key,
            result_id,
            node_id,
            details,
        } = input;
        self.tool_result(resolve_and_append(
            &self.app,
            &project_id,
            &session_id,
            expected_version,
            idempotency_key,
            |version, key| {
                self.app.agent_result_record(
                    &project_id,
                    &task_id,
                    &session_id,
                    &actor,
                    version,
                    key,
                    &result_id,
                    node_id.clone(),
                    details.clone(),
                )
            },
        ))
    }

    #[tool(
        description = "Close a VibeHub V3 agent session. When: after agent_result_record, as the last step of a work batch or handoff. Prerequisite: agent_result already recorded for this session. Typical params: project_id, task_id, session_id, actor. expected_version and idempotency_key are optional and auto-resolved when omitted; explicitly provided values are strictly validated"
    )]
    fn session_close(&self, Parameters(input): Parameters<SessionWrite>) -> CallToolResult {
        let SessionWrite {
            project_id,
            task_id,
            session_id,
            actor,
            expected_version,
            idempotency_key,
            ..
        } = input;
        self.tool_result(resolve_and_append(
            &self.app,
            &project_id,
            &session_id,
            expected_version,
            idempotency_key,
            |version, key| {
                self.app
                    .session_close(&project_id, &task_id, &session_id, &actor, version, key)
            },
        ))
    }

    #[tool(
        description = "Add a node to the VibeHub V3 task plan. When: at the start of a standard/full task (execution_policy.planning_required is true), before implementing, to break work into verifiable nodes. Prerequisite: you have read task_view. Typical params: project_id, task_id, actor, node_id (a stable id you choose), title, goal, scope (files/areas), dependencies (ids of already-added nodes; leave empty and set later via plan_dependencies_set if the dependency does not exist yet). expected_version and idempotency_key are optional and auto-resolved when omitted; explicitly provided values are strictly validated"
    )]
    fn plan_node_add(&self, Parameters(input): Parameters<PlanNodeAddWrite>) -> CallToolResult {
        let PlanNodeAddWrite {
            identity,
            node_id,
            title,
            goal,
            scope,
            dependencies,
            criterion_ids,
        } = input;
        let PlanWriteScope {
            project_id,
            task_id,
            actor,
            expected_version,
            idempotency_key,
        } = identity;
        self.tool_result(resolve_and_append(
            &self.app,
            &project_id,
            &task_id,
            expected_version,
            idempotency_key,
            |version, key| {
                self.app.plan_add_node(PlanAddNodeCommand {
                    identity: PlanCommandIdentity {
                        project_id: project_id.clone(),
                        task_id: task_id.clone(),
                        actor: actor.clone(),
                        expected_version: version,
                        idempotency_key: key.to_owned(),
                    },
                    node_id: node_id.clone(),
                    title: title.clone(),
                    goal: goal.clone(),
                    scope: scope.clone(),
                    dependencies: dependencies.clone(),
                    criterion_ids: criterion_ids.clone(),
                })
            },
        ))
    }

    #[tool(
        description = "Replace dependencies for a VibeHub V3 plan node. When: after all referenced nodes exist, to wire scheduling order between plan nodes. Prerequisite: both the node and every dependency node were already added via plan_node_add. Typical params: project_id, task_id, actor, node_id, dependencies (list of existing node_ids). expected_version and idempotency_key are optional and auto-resolved when omitted; explicitly provided values are strictly validated"
    )]
    fn plan_dependencies_set(
        &self,
        Parameters(input): Parameters<PlanDependenciesSetWrite>,
    ) -> CallToolResult {
        let PlanDependenciesSetWrite {
            scope,
            node_id,
            dependencies,
            change_mode,
            reason,
        } = input;
        let PlanWriteScope {
            project_id,
            task_id,
            actor,
            expected_version,
            idempotency_key,
        } = scope;
        self.tool_result(resolve_and_append(
            &self.app,
            &project_id,
            &task_id,
            expected_version,
            idempotency_key,
            |version, key| {
                self.app.plan_set_dependencies(PlanSetDependenciesCommand {
                    identity: PlanCommandIdentity {
                        project_id: project_id.clone(),
                        task_id: task_id.clone(),
                        actor: actor.clone(),
                        expected_version: version,
                        idempotency_key: key.to_owned(),
                    },
                    node_id: node_id.clone(),
                    dependencies: dependencies.clone(),
                    change_mode: change_mode.clone(),
                    reason: reason.clone(),
                })
            },
        ))
    }

    #[tool(
        description = "Transition a VibeHub V3 plan node state. When: set active right before you start working a node (before session_open); set completed after its work and evidence are done; set blocked/failed when stuck (also log a risk event). Prerequisite: the node exists. Typical params: project_id, task_id, actor, node_id, state (ready|active|blocked|completed|failed|cancelled). expected_version and idempotency_key are optional and auto-resolved when omitted; explicitly provided values are strictly validated"
    )]
    fn plan_node_state_set(
        &self,
        Parameters(input): Parameters<PlanNodeStateSetWrite>,
    ) -> CallToolResult {
        let PlanNodeStateSetWrite {
            scope,
            node_id,
            state,
        } = input;
        let PlanWriteScope {
            project_id,
            task_id,
            actor,
            expected_version,
            idempotency_key,
        } = scope;
        self.tool_result(resolve_and_append(
            &self.app,
            &project_id,
            &task_id,
            expected_version,
            idempotency_key,
            |version, key| {
                self.app.plan_set_state(PlanSetStateCommand {
                    identity: PlanCommandIdentity {
                        project_id: project_id.clone(),
                        task_id: task_id.clone(),
                        actor: actor.clone(),
                        expected_version: version,
                        idempotency_key: key.to_owned(),
                    },
                    node_id: node_id.clone(),
                    state: state.clone(),
                })
            },
        ))
    }

    #[tool(
        description = "Upgrade a task execution policy at runtime. When: a new trigger requires stricter execution. Prerequisite: task_view policy. Typical params: project_id, task_id, target_profile, reason."
    )]
    fn task_policy_upgrade(
        &self,
        Parameters(input): Parameters<TaskPolicyUpgradeWrite>,
    ) -> CallToolResult {
        let PlanWriteScope {
            project_id,
            task_id,
            actor,
            expected_version,
            idempotency_key,
        } = input.scope;
        self.tool_result(resolve_and_append(
            &self.app,
            &project_id,
            &task_id,
            expected_version,
            idempotency_key,
            |version, key| {
                self.app.upgrade_task_policy(
                    &project_id,
                    &task_id,
                    &actor,
                    &input.target_profile,
                    &input.reason,
                    version,
                    key,
                )
            },
        ))
    }

    #[tool(
        description = "Link registered acceptance criterion ids to a planned node. When: planning standard/full work. Prerequisite: registered criteria and planned node. Typical params: project_id, task_id, node_id, criterion_ids."
    )]
    fn plan_criteria_set(
        &self,
        Parameters(input): Parameters<PlanCriteriaSetWrite>,
    ) -> CallToolResult {
        let PlanWriteScope {
            project_id,
            task_id,
            actor,
            expected_version,
            idempotency_key,
        } = input.scope;
        self.tool_result(resolve_and_append(
            &self.app,
            &project_id,
            &task_id,
            expected_version,
            idempotency_key,
            |version, key| {
                self.app.plan_set_criteria(PlanSetCriteriaCommand {
                    identity: PlanCommandIdentity {
                        project_id: project_id.clone(),
                        task_id: task_id.clone(),
                        actor: actor.clone(),
                        expected_version: version,
                        idempotency_key: key.to_owned(),
                    },
                    node_id: input.node_id.clone(),
                    criterion_ids: input.criterion_ids.clone(),
                })
            },
        ))
    }

    #[tool(
        description = "Manage a finding through the shared validator. When: review finds or closes a defect. Prerequisite: task and target node exist. Typical params: action, finding_id, evidence_refs."
    )]
    fn finding_manage(&self, Parameters(input): Parameters<FindingWrite>) -> CallToolResult {
        let PlanWriteScope {
            project_id,
            task_id,
            actor,
            expected_version,
            idempotency_key,
        } = input.scope;
        let event_type = match input.action.as_str() {
            "open" => "finding.opened",
            "regress" => "finding.regressed",
            "close" => "finding.closed",
            _ => {
                return tool_error(
                    json!({"code":"V3_FINDING_ACTION_INVALID","message":"action must be open, regress, or close"}),
                )
            }
        };
        self.tool_result(resolve_and_append(&self.app,&project_id,&task_id,expected_version,idempotency_key,|version,key|self.app.lifecycle_command(LifecycleCommand{event_type:event_type.to_owned(),project_id:project_id.clone(),task_id:task_id.clone(),node_id:input.node_id.clone(),session_id:None,actor:actor.clone(),expected_version:version,idempotency_key:key.to_owned(),evidence_grade:None,payload:json!({"finding_id":input.finding_id,"target_node_id":input.node_id,"severity":input.severity,"evidence_refs":input.evidence_refs,"details":input.details})})))
    }

    #[tool(
        description = "Manage a remediation attempt. When: addressing an open finding. Prerequisite: finding exists. Typical params: action, attempt_id, finding_id, evidence_refs."
    )]
    fn attempt_manage(&self, Parameters(input): Parameters<AttemptWrite>) -> CallToolResult {
        let PlanWriteScope {
            project_id,
            task_id,
            actor,
            expected_version,
            idempotency_key,
        } = input.scope;
        let event_type = match input.action.as_str() {
            "start" => "attempt.started",
            "complete" => "attempt.completed",
            "fail" => "attempt.failed",
            _ => {
                return tool_error(
                    json!({"code":"V3_ATTEMPT_ACTION_INVALID","message":"action must be start, complete, or fail"}),
                )
            }
        };
        self.tool_result(resolve_and_append(&self.app,&project_id,&task_id,expected_version,idempotency_key,|version,key|self.app.lifecycle_command(LifecycleCommand{event_type:event_type.to_owned(),project_id:project_id.clone(),task_id:task_id.clone(),node_id:input.node_id.clone(),session_id:None,actor:actor.clone(),expected_version:version,idempotency_key:key.to_owned(),evidence_grade:None,payload:json!({"attempt_id":input.attempt_id,"finding_id":input.finding_id,"evidence_refs":input.evidence_refs,"details":input.details})})))
    }

    #[tool(
        description = "Record or recover an abnormal session gap. When: interruption or recovery occurs. Prerequisite: open session for gap or gapped session for recover. Typical params: action, session scope, reason/evidence_refs."
    )]
    fn session_recovery(
        &self,
        Parameters(input): Parameters<SessionRecoveryWrite>,
    ) -> CallToolResult {
        let session = input.session;
        self.tool_result(resolve_and_append(
            &self.app,
            &session.project_id,
            &session.session_id,
            session.expected_version,
            session.idempotency_key,
            |version, key| match input.action.as_str() {
                "gap" => self.app.session_gap(
                    &session.project_id,
                    &session.task_id,
                    &session.session_id,
                    &session.actor,
                    version,
                    key,
                    input.reason.as_deref().unwrap_or(""),
                ),
                "recover" => self.app.session_recover(
                    &session.project_id,
                    &session.task_id,
                    &session.session_id,
                    &session.actor,
                    version,
                    key,
                    input.evidence_refs.clone(),
                ),
                _ => Err(V3Error::new(
                    "V3_SESSION_RECOVERY_ACTION_INVALID",
                    V3ErrorCategory::Validation,
                    false,
                    "action must be gap or recover",
                )),
            },
        ))
    }

    #[tool(
        description = "Write Project Memory with revision preconditions. When: curating or promoting durable context. Prerequisite: correct entry revision and evidence. Typical params: action, entry_id, expected_revision, entry."
    )]
    fn memory_write(&self, Parameters(input): Parameters<MemoryWrite>) -> CallToolResult {
        let entry = match input.entry {
            Some(value) => match serde_json::from_value::<MemoryEntry>(value) {
                Ok(entry) => Some(entry),
                Err(error) => {
                    return tool_error(
                        json!({"code":"V3_MEMORY_ENTRY_INVALID","message":error.to_string()}),
                    )
                }
            },
            None => None,
        };
        let key = input
            .idempotency_key
            .unwrap_or_else(|| format!("auto.{}", Uuid::new_v4()));
        self.tool_result(self.app.memory_command(MemoryCommand {
            action: input.action,
            project_id: input.project_id,
            task_id: input.task_id,
            actor: input.actor,
            entry_id: input.entry_id,
            expected_revision: input.expected_revision,
            idempotency_key: key,
            entry,
            details: input.details,
        }))
    }

    #[tool(
        description = "Query Project Memory. When: preparing task/node context. Prerequisite: project_id and actor scope. Typical params: kinds, scope, principal_scope, token_budget."
    )]
    fn memory_query(&self, Parameters(input): Parameters<MemoryQueryRead>) -> CallToolResult {
        if input.project_id != self.project_id {
            return tool_error(
                json!({"code":"V3_PROJECT_MISMATCH","message":"project_id does not match this MCP workspace"}),
            );
        }
        self.tool_result(self.app.query_project_memory(
            &input.project_id,
            &MemoryQuery {
                kinds: input.kinds,
                scope: input.scope,
                principal_scope: input.principal_scope,
                include_stale: input.include_stale,
                include_disputed: input.include_disputed,
                token_budget: input.token_budget,
            },
        ))
    }

    #[tool(
        description = "Apply typed worktree, lease, or integration transitions. When: orchestrating parallel work. Prerequisite: valid prior state and eligibility digest. Typical params: event_type, worktree_id, operation_id, eligibility_digest."
    )]
    fn orchestration_write(
        &self,
        Parameters(input): Parameters<OrchestrationWrite>,
    ) -> CallToolResult {
        let PlanWriteScope {
            project_id,
            task_id,
            actor,
            expected_version,
            idempotency_key,
        } = input.scope;
        self.tool_result(resolve_and_append(
            &self.app,
            &project_id,
            &input.worktree_id,
            expected_version,
            idempotency_key,
            |version, key| {
                self.app.orchestration_command(OrchestrationCommand {
                    event_type: input.event_type.clone(),
                    project_id: project_id.clone(),
                    task_id: task_id.clone(),
                    node_id: input.node_id.clone(),
                    worktree_id: input.worktree_id.clone(),
                    session_id: input.session_id.clone(),
                    lease_id: input.lease_id.clone(),
                    operation_id: input.operation_id.clone(),
                    eligibility_digest: input.eligibility_digest.clone(),
                    lease_generation: input.lease_generation,
                    actor: actor.clone(),
                    expected_version: version,
                    idempotency_key: key.to_owned(),
                    evidence_grade: None,
                    payload: input.details.clone(),
                })
            },
        ))
    }

    fn tool_result<T: Serialize>(&self, result: Result<T, V3Error>) -> CallToolResult {
        match result.and_then(|value| {
            serde_json::to_value(value).map_err(|error| {
                V3Error::new(
                    "V3_SERIALIZE_FAILED",
                    vibehub_core::v3::V3ErrorCategory::Internal,
                    false,
                    error.to_string(),
                )
            })
        }) {
            Ok(result) => tool_success(json!(ToolResponse { ok: true, result })),
            Err(error) => tool_error(serde_json::to_value(error).unwrap_or_else(
                |_| json!({"code": "V3_INTERNAL", "message": "failed to serialize error"}),
            )),
        }
    }

    fn resource_catalog(&self) -> Result<Vec<Resource>, V3Error> {
        let task_id = self.views.current_task_id()?;
        let resources = [
            (
                format!("projects/{}/overview", self.project_id),
                "project-overview",
                "M0-compatible project overview",
            ),
            (
                format!("projects/{}/structure", self.project_id),
                "project-structure",
                "M0-compatible project structure",
            ),
            (
                format!("tasks/{}/timeline", task_id),
                "task-timeline",
                "M0-compatible task timeline",
            ),
            (
                format!("tasks/{}/plan", task_id),
                "plan-graph",
                "M0-compatible task plan graph",
            ),
            (
                format!("tasks/{}/nodes/current/brief", task_id),
                "node-brief",
                "M0-compatible current node brief",
            ),
            (
                format!("projects/{}/projection", self.project_id),
                "projection",
                "Current event-derived recovery projection",
            ),
            (
                "diagnostics".to_owned(),
                "diagnostics",
                "Project-scoped V3 diagnostics",
            ),
        ]
        .into_iter()
        .map(|(path, name, description)| {
            Resource::new(format!("{RESOURCE_PREFIX}/{path}"), name)
                .with_description(description)
                .with_mime_type("application/json")
        })
        .collect::<Vec<_>>();
        Ok(resources)
    }

    fn read_resource_text(&self, uri: &str) -> Result<String, V3Error> {
        let suffix = uri
            .strip_prefix(&format!("{RESOURCE_PREFIX}/"))
            .ok_or_else(|| {
                V3Error::new(
                    "V3_RESOURCE_NOT_FOUND",
                    vibehub_core::v3::V3ErrorCategory::NotFound,
                    false,
                    format!("unsupported resource URI: {uri}"),
                )
            })?;
        let task_id = self.views.current_task_id()?;
        let bundle = self.views.load_bundle(&task_id)?;
        let value = match suffix {
            path if path == format!("projects/{}/overview", self.project_id) => {
                bundle.project_overview
            }
            path if path == format!("projects/{}/structure", self.project_id) => {
                bundle.project_structure
            }
            path if path == format!("tasks/{}/timeline", task_id) => bundle.task_timeline,
            path if path == format!("tasks/{}/plan", task_id) => bundle.plan_graph,
            path if path == format!("tasks/{}/nodes/current/brief", task_id) => bundle.node_brief,
            path if path == format!("projects/{}/projection", self.project_id) => {
                serde_json::to_value(self.app.rebuild(&self.project_id)?).map_err(|error| {
                    V3Error::new(
                        "V3_SERIALIZE_FAILED",
                        vibehub_core::v3::V3ErrorCategory::Internal,
                        false,
                        error.to_string(),
                    )
                })?
            }
            "diagnostics" => {
                let targets = read_project_settings(&self.project_root)
                    .ok()
                    .and_then(|inspection| inspection.settings)
                    .map(|settings| settings.agent_spec_targets)
                    .filter(|targets| !targets.is_empty())
                    .unwrap_or_else(|| {
                        vec![AgentSpecTarget::ClaudeCode, AgentSpecTarget::Opencode]
                    });
                let host_configs = inspect_mcp_host_configs(&self.resolved_scopes, &targets);
                let alignment = assess_root_alignment(&self.resolved_scopes, &host_configs);
                let restart_required = alignment.restart_required;
                json!({
                    "schema_version": "1.0",
                    "server_version": env!("CARGO_PKG_VERSION"),
                    "transport": "stdio",
                    "project_id": self.project_id,
                    "project_root": self.project_root,
                    "scopes": self.scopes,
                    "resource_namespace": RESOURCE_PREFIX,
                    "tool_catalog": [
                        "session_open", "task_candidates", "task_view", "criterion_review",
                        "task_completion_propose", "task_complete", "event_log", "agent_result_record",
                        "session_close", "plan_node_add", "plan_dependencies_set", "plan_node_state_set"
                        , "task_policy_upgrade", "plan_criteria_set", "finding_manage", "attempt_manage",
                        "session_recovery", "memory_write", "memory_query", "orchestration_write"
                    ],
                    "mcp_hosts": host_configs,
                    "root_alignment": alignment,
                    "restart_required": restart_required,
                    "last_error": Value::Null
                })
            }
            _ => {
                return Err(V3Error::new(
                    "V3_RESOURCE_NOT_FOUND",
                    vibehub_core::v3::V3ErrorCategory::NotFound,
                    false,
                    format!("unsupported resource URI: {uri}"),
                ))
            }
        };
        serde_json::to_string_pretty(&value).map_err(|error| {
            V3Error::new(
                "V3_SERIALIZE_FAILED",
                vibehub_core::v3::V3ErrorCategory::Internal,
                false,
                error.to_string(),
            )
        })
    }
}

#[tool_handler]
impl ServerHandler for V3McpServer {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.capabilities = ServerCapabilities::builder()
            .enable_resources()
            .enable_tools()
            .build();
        info.server_info = Implementation::new("vibehub-v3", env!("CARGO_PKG_VERSION"))
            .with_title("VibeHub V3 MCP");
        info.instructions = Some(
            "First call task_candidates, then task_view; echo node_brief.workflow_profile and node_brief.execution_policy before acting. Lightweight uses the minimal session/result/risk-if-any flow without a plan graph. When planning_required is true (standard/full), create or refine the plan before file changes, activate the target node, then call session_open. Record progress at each milestone and risks immediately; run real validation before criterion_review; complete the node, record agent_result with the required detail fields, then session_close. Call task_completion_propose only when all gates are green, and task_complete only after explicit current-user confirmation. Accepted criteria are not passed. expected_version and idempotency_key may be omitted and are resolved by the server; explicitly provided values are checked strictly."
                .to_owned(),
        );
        info
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourcesResult, rmcp::ErrorData>> + Send + '_
    {
        let result = self
            .resource_catalog()
            .map(ListResourcesResult::with_all_items)
            .map_err(|error| {
                rmcp::ErrorData::invalid_params(error.message, Some(Value::Object(error.details)))
            });
        std::future::ready(result)
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl std::future::Future<Output = Result<ReadResourceResult, rmcp::ErrorData>> + Send + '_
    {
        let result = self.read_resource_text(&request.uri).map(|text| {
            ReadResourceResult::new(vec![
                ResourceContents::text(text, request.uri).with_mime_type("application/json")
            ])
        });
        std::future::ready(result.map_err(|error| {
            rmcp::ErrorData::invalid_params(error.message, Some(Value::Object(error.details)))
        }))
    }
}

pub fn run_stdio(project_root: &str) {
    let server = V3McpServer::open(project_root).unwrap_or_else(|error| {
        eprintln!(
            "{}",
            serde_json::to_string(&error).unwrap_or_else(|_| error.to_string())
        );
        std::process::exit(1);
    });
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|error| {
        eprintln!("failed to start MCP runtime: {error}");
        std::process::exit(1);
    });
    runtime.block_on(async move {
        match server
            .serve((tokio::io::stdin(), tokio::io::stdout()))
            .await
        {
            Ok(service) => {
                if let Err(error) = service.waiting().await {
                    eprintln!("MCP server stopped with error: {error}");
                    std::process::exit(1);
                }
            }
            Err(error) => {
                eprintln!("failed to start MCP stdio transport: {error}");
                std::process::exit(1);
            }
        }
    });
}

fn tool_success(value: Value) -> CallToolResult {
    CallToolResult::structured(value)
}

fn tool_error(value: Value) -> CallToolResult {
    CallToolResult::structured_error(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use uuid::Uuid;

    fn server() -> (PathBuf, V3McpServer) {
        let root = std::env::temp_dir().join(format!("vibehub-v3-mcp-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub/tasks/current")).unwrap();
        fs::create_dir_all(root.join(".vibehub/tasks/task.test")).unwrap();
        let task = "task_id: task.test\ntitle: MCP test\nintent: Verify MCP\nphase: implement\nphase_status: active\nacceptance_criteria: []\ndependencies: []\n";
        fs::write(root.join(".vibehub/tasks/current/task.yaml"), task).unwrap();
        fs::write(root.join(".vibehub/tasks/task.test/task.yaml"), task).unwrap();
        let server = V3McpServer::open(&root).unwrap();
        (root, server)
    }

    fn assert_in_order(text: &str, terms: &[&str]) {
        let mut offset = 0;
        for term in terms {
            let relative = text[offset..]
                .find(term)
                .unwrap_or_else(|| panic!("missing '{term}' in: {text}"));
            offset += relative + term.len();
        }
    }

    #[test]
    fn catalog_is_versioned_and_readable() {
        let (root, server) = server();
        let resources = server.resource_catalog().unwrap();
        assert_eq!(resources.len(), 7);
        assert!(resources
            .iter()
            .all(|resource| resource.uri.starts_with(RESOURCE_PREFIX)));
        let diagnostics = server
            .read_resource_text("vibehub://v3/1.0/diagnostics")
            .unwrap();
        let diagnostics: Value = serde_json::from_str(&diagnostics).unwrap();
        assert_eq!(diagnostics["transport"], "stdio");
        assert_eq!(diagnostics["project_id"], server.project_id);
        assert_eq!(
            diagnostics["scopes"]["control_root"],
            server.scopes.control_root
        );
        assert_eq!(diagnostics["tool_catalog"].as_array().unwrap().len(), 20);
        assert_eq!(diagnostics["restart_required"], false);
        assert_eq!(diagnostics["root_alignment"]["status"], "aligned");
        assert_eq!(
            diagnostics["root_alignment"]["control_root"],
            server.scopes.control_root
        );
        assert!(diagnostics["mcp_hosts"].is_array());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn tool_descriptions_expose_when_prerequisite_and_typical_params() {
        let (root, server) = server();
        let tools = server.tool_router.list_all();
        assert_eq!(tools.len(), 20);
        for tool in &tools {
            let description = tool
                .description
                .as_deref()
                .unwrap_or_else(|| panic!("{} must have a description", tool.name));
            for section in ["When:", "Prerequisite:", "Typical params:"] {
                assert!(
                    description.contains(section),
                    "{} description is missing {section}: {description}",
                    tool.name
                );
            }
        }

        let description = |name: &str| {
            tools
                .iter()
                .find(|tool| tool.name.as_ref() == name)
                .and_then(|tool| tool.description.as_deref())
                .unwrap()
        };
        let task_view = description("task_view");
        for field in [
            "workflow_profile",
            "planning_required",
            "milestone_policy",
            "review_required",
            "required_records",
        ] {
            assert!(
                task_view.contains(field),
                "task_view must advertise {field}"
            );
        }

        let result = description("agent_result_record");
        for field in ["kind", "request_source", "instruction", "status", "summary"] {
            assert!(
                result.contains(field),
                "agent_result_record must advertise required field {field}"
            );
        }

        let completion = description("task_completion_propose");
        for prerequisite in ["criterion_review", "agent_result_record", "session_close"] {
            assert!(
                completion.contains(prerequisite),
                "task_completion_propose must name {prerequisite}"
            );
        }

        let instructions = server.get_info().instructions.unwrap();
        assert_in_order(&instructions, &["task_candidates", "task_view"]);
        assert!(instructions.contains("echo"));
        assert!(instructions.contains("plan before file changes"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn catalog_tracks_current_task_after_pointer_changes() {
        let (root, server) = server();
        fs::remove_dir_all(root.join(".vibehub/tasks/current")).unwrap();
        fs::create_dir_all(root.join(".vibehub/tasks/task.next")).unwrap();
        fs::write(
            root.join(".vibehub/tasks/task.next/task.yaml"),
            "task_id: task.next\ntitle: Next task\nintent: Verify fresh MCP resources\nphase: implement\nphase_status: active\nacceptance_criteria: []\ndependencies: []\n",
        )
        .unwrap();
        fs::write(
            root.join(".vibehub/tasks/current"),
            "schema_version: 1\nkind: current_task_pointer\ntask_id: task.next\npath: .vibehub/tasks/task.next\nupdated_at: 2026-07-14T00:00:00Z\nupdated_by: vibehub\n",
        )
        .unwrap();

        let resources = server.resource_catalog().unwrap();
        let timeline_uri = format!("{RESOURCE_PREFIX}/tasks/task.next/timeline");
        assert!(resources
            .iter()
            .any(|resource| resource.uri == timeline_uri));
        let timeline = server.read_resource_text(&timeline_uri).unwrap();
        assert!(timeline.contains("task.next"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn omitted_write_scope_is_resolved_and_versions_advance() {
        let (root, server) = server();
        let opened = server.session_open(Parameters(SessionWrite {
            project_id: "project.test".into(),
            task_id: "task.test".into(),
            session_id: "session.auto".into(),
            actor: "codex".into(),
            expected_version: None,
            idempotency_key: None,
            working_directory: None,
            node_id: None,
            worktree_id: None,
            provider: None,
            provider_session_id: None,
        }));
        assert!(!opened.is_error.unwrap_or(false));
        assert_eq!(
            opened.structured_content.unwrap()["result"]["status"],
            "appended"
        );
        assert_eq!(
            server
                .app
                .aggregate_version("project.test", "session.auto")
                .unwrap(),
            1
        );

        let first_log = server.event_log(Parameters(EventLogWrite {
            project_id: "project.test".into(),
            task_id: "task.test".into(),
            session_id: "session.auto".into(),
            actor: "codex".into(),
            expected_version: None,
            idempotency_key: None,
            kind: "progress".into(),
            details: json!({"message": "first"}),
        }));
        let second_log = server.event_log(Parameters(EventLogWrite {
            project_id: "project.test".into(),
            task_id: "task.test".into(),
            session_id: "session.auto".into(),
            actor: "codex".into(),
            expected_version: None,
            idempotency_key: None,
            kind: "progress".into(),
            details: json!({"message": "second"}),
        }));
        assert!(!first_log.is_error.unwrap_or(false));
        assert!(!second_log.is_error.unwrap_or(false));
        assert_eq!(
            first_log.structured_content.unwrap()["result"]["status"],
            "appended"
        );
        assert_eq!(
            second_log.structured_content.unwrap()["result"]["status"],
            "appended"
        );
        assert_eq!(
            server
                .app
                .aggregate_version("project.test", "session.auto")
                .unwrap(),
            3
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn tools_share_application_service_semantics() {
        let (root, server) = server();
        let input = SessionWrite {
            project_id: "project.test".into(),
            task_id: "task.test".into(),
            session_id: "session.test".into(),
            actor: "codex".into(),
            expected_version: Some(0),
            idempotency_key: Some("open.1".into()),
            working_directory: None,
            node_id: None,
            worktree_id: None,
            provider: None,
            provider_session_id: None,
        };
        let first = server.session_open(Parameters(input.clone()));
        let duplicate = server.session_open(Parameters(input));
        assert!(!first.is_error.unwrap_or(false));
        assert!(!duplicate.is_error.unwrap_or(false));
        assert_eq!(
            first.structured_content.unwrap()["result"]["status"],
            "appended"
        );
        assert_eq!(
            duplicate.structured_content.unwrap()["result"]["status"],
            "duplicate"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn session_open_preserves_provider_session_link() {
        let (root, server) = server();
        let project_id = server.project_id.clone();
        let opened = server.session_open(Parameters(SessionWrite {
            project_id,
            task_id: "task.test".into(),
            session_id: "session.workflow".into(),
            actor: "codex".into(),
            expected_version: Some(0),
            idempotency_key: Some("open.provider.1".into()),
            working_directory: Some("/repo/app".into()),
            node_id: None,
            worktree_id: None,
            provider: Some("codex".into()),
            provider_session_id: Some("provider-123".into()),
        }));
        assert!(!opened.is_error.unwrap_or(false));

        let bundle = server.views.load_bundle("task.test").unwrap();
        let session_event = bundle.task_timeline["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["session_id"] == "session.workflow")
            .expect("session event");
        assert_eq!(
            session_event["details"]["provider_session_id"],
            "provider-123"
        );
        assert_eq!(session_event["details"]["provider"], "codex");
        fs::remove_dir_all(root).unwrap();
    }
}

use super::application::canonical_criterion_id;
use super::blockers::{
    BlockerDetail, BlockerKind, BlockerProvenance, BlockerProvenanceStatus, RepairAction,
    RepairActionKind, BLOCKER_MODEL_VERSION,
};
use super::lifecycle::{
    fold_task, is_terminal_plan_node_state, task_truth_state, valid_evidence_ref, CriterionState,
    PlanNodeProjection, TaskLifecycleProjection,
};
use super::orchestration::fold_task as fold_orchestration;
use super::orchestration::LeaseState;
use super::project_intelligence::{
    AnalyzerFinding, GitState, NodeKind, ProjectIndexService, ProjectModelSnapshot, ProjectPage,
};
use super::projection;
use super::routing::TaskRouteCandidate;
use super::worktree::WorktreeState;
use super::{
    project_id, resolve_project_scopes, EffectiveExecutionPolicy, EvidenceGrade,
    ProjectScopeSource, V3Error, V3ErrorCategory, V3EventEnvelope, V3EventStore,
};
use crate::process_util::silent_command;
use crate::vibehub::current::resolve_current_task;
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const MODEL_VERSION: &str = "v3-core-2";
const WORKSPACE_IGNORED_DIRS: &[&str] = &[
    ".git",
    ".vibehub",
    ".next",
    ".turbo",
    ".vite",
    ".cache",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "target",
    "vendor",
    "__pycache__",
    ".venv",
    "venv",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct V3ViewBundle {
    pub project_overview: Value,
    pub project_structure: Value,
    pub agent_results: Value,
    pub task_timeline: Value,
    pub plan_graph: Value,
    pub node_brief: Value,
}

#[derive(Debug, Clone)]
pub struct V3ViewRepository {
    root: PathBuf,
    store: V3EventStore,
}

#[derive(Debug, Clone, Deserialize)]
struct TaskDocument {
    task_id: String,
    title: String,
    intent: String,
    phase: String,
    phase_status: String,
    #[serde(default)]
    acceptance_criteria: Vec<String>,
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

#[derive(Debug, Clone)]
struct TaskReadResult {
    tasks: Vec<TaskDocument>,
    warnings: Vec<Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SessionIntegrity {
    observed: BTreeSet<String>,
    opened: BTreeSet<String>,
    closed: BTreeSet<String>,
    terminal_results: BTreeSet<String>,
    unknown: Vec<String>,
    gapped: Vec<String>,
    closed_without_result: Vec<String>,
}

impl SessionIntegrity {
    fn has_blocking_gap(&self) -> bool {
        !self.unknown.is_empty()
            || !self.gapped.is_empty()
            || !self.closed_without_result.is_empty()
    }

    fn blocking_session_ids(&self) -> Vec<String> {
        self.unknown
            .iter()
            .chain(self.gapped.iter())
            .chain(self.closed_without_result.iter())
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

fn event_session_id(event: &V3EventEnvelope) -> Option<String> {
    event
        .session_id
        .as_ref()
        .map(|id| id.0.clone())
        .or_else(|| {
            event
                .payload
                .get("session_id")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
}

/// Reconstruct Session coverage from the event log without inventing a
/// missing `session.opened` or terminal result. A Session that only has a
/// historical binding, or one that closed without a terminal result, is an
/// explicit workflow gap rather than successful work.
fn session_integrity(
    events: &[V3EventEnvelope],
    task_id: &str,
    lifecycle: &TaskLifecycleProjection,
) -> SessionIntegrity {
    let mut integrity = SessionIntegrity::default();
    for event in events.iter().filter(|event| event.task_id.0 == task_id) {
        let Some(session_id) = event_session_id(event) else {
            continue;
        };
        integrity.observed.insert(session_id.clone());
        match event.event_type.as_str() {
            "session.opened" => {
                integrity.opened.insert(session_id);
            }
            "session.closed" => {
                integrity.closed.insert(session_id);
            }
            "agent.result_recorded"
                if matches!(
                    event.payload.get("status").and_then(Value::as_str),
                    Some("succeeded" | "failed")
                ) =>
            {
                integrity.terminal_results.insert(session_id);
            }
            _ => {}
        }
    }

    for session_id in &integrity.observed {
        let state = lifecycle
            .sessions
            .get(session_id)
            .map(|session| session.state.as_str())
            .unwrap_or("unknown");
        if !integrity.opened.contains(session_id) || state == "unknown" {
            integrity.unknown.push(session_id.clone());
        }
        if state == "gapped" {
            integrity.gapped.push(session_id.clone());
        }
        if integrity.closed.contains(session_id) && !integrity.terminal_results.contains(session_id)
        {
            integrity.closed_without_result.push(session_id.clone());
        }
    }
    integrity
}

fn projection_is_stale(status: &Value) -> bool {
    status.get("stale").and_then(Value::as_bool).unwrap_or(true)
}

fn projection_warning(status: &Value) -> Option<Value> {
    if !projection_is_stale(status) {
        return None;
    }
    Some(json!({
        "code": "V3_PROJECTION_STALE",
        "severity": "warning",
        "message_key": "v3.warning.projection_stale",
        "details": {
            "event_count": status.get("event_count").cloned().unwrap_or(Value::Null),
            "projection_event_count": status.get("projection_event_count").cloned().unwrap_or(Value::Null),
            "last_event_timestamp": status.get("last_event_timestamp").cloned().unwrap_or(Value::Null),
            "repair_action": "通过受支持的 projection rebuild 命令追平事件与 projection；在追平前不得宣称 completed"
        },
        "evidence_refs": []
    }))
}

#[derive(Debug, Clone, Default)]
struct SessionGitTrace {
    session_id: String,
    open_git_head: Option<String>,
    close_git_head: Option<String>,
    opened_at: Option<String>,
    closed_at: Option<String>,
    event_commit_shas: BTreeSet<String>,
}

fn default_workflow_profile() -> String {
    "standard".to_owned()
}

fn select_node_id(
    lifecycle: &TaskLifecycleProjection,
    requested: Option<&str>,
) -> Result<String, V3Error> {
    if let Some(id) = requested {
        if lifecycle.effective_node(id).is_some() {
            return Ok(id.to_owned());
        }
        return Err(V3Error::new(
            "V3_NODE_NOT_FOUND",
            V3ErrorCategory::NotFound,
            false,
            "requested NodeBrief node_id does not exist",
        ));
    }
    let choose = |states: &[&str]| {
        lifecycle
            .effective_nodes()
            .map(|(_, node)| node)
            .find(|node| states.contains(&node.state.as_str()))
            .map(|node| node.node_id.clone())
    };
    Ok(choose(&["active"])
        .or_else(|| {
            lifecycle
                .effective_nodes()
                .map(|(_, node)| node)
                .find(|node| {
                    node.state == "ready"
                        || node.state == "planned"
                            && node.dependencies.iter().all(|dependency| {
                                lifecycle.effective_node(dependency).is_some_and(|value| {
                                    matches!(value.state.as_str(), "completed" | "waived")
                                })
                            })
                })
                .map(|node| node.node_id.clone())
        })
        .or_else(|| choose(&["blocked", "failed"]))
        .or_else(|| lifecycle.effective_nodes().next().map(|(id, _)| id.clone()))
        .unwrap_or_default())
}

fn plan_node_layer(
    node_id: &str,
    nodes: &BTreeMap<String, PlanNodeProjection>,
    memo: &mut BTreeMap<String, usize>,
    visiting: &mut BTreeSet<String>,
) -> usize {
    if let Some(layer) = memo.get(node_id) {
        return *layer;
    }
    if !visiting.insert(node_id.to_owned()) {
        // The lifecycle validator normally prevents cycles. A malformed
        // historical graph must still render deterministically and must not
        // turn a dependency layer into an execution claim.
        return 0;
    }
    let layer = nodes
        .get(node_id)
        .map(|node| {
            node.dependencies
                .iter()
                .filter(|dependency| nodes.get(*dependency).is_some())
                .map(|dependency| plan_node_layer(dependency, nodes, memo, visiting) + 1)
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0);
    visiting.remove(node_id);
    memo.insert(node_id.to_owned(), layer);
    layer
}

fn effective_plan_layers(
    lifecycle: &TaskLifecycleProjection,
) -> (BTreeMap<String, usize>, BTreeMap<usize, usize>) {
    let effective_nodes = lifecycle
        .effective_nodes()
        .map(|(id, node)| (id.clone(), node.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut layers = BTreeMap::new();
    for node_id in effective_nodes.keys() {
        let mut visiting = BTreeSet::new();
        let layer = plan_node_layer(node_id, &effective_nodes, &mut layers, &mut visiting);
        layers.insert(node_id.clone(), layer);
    }
    let mut counts = BTreeMap::new();
    for layer in layers.values() {
        *counts.entry(*layer).or_insert(0) += 1;
    }
    (layers, counts)
}

impl V3ViewRepository {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, V3Error> {
        let store = V3EventStore::open(&root)?;
        let root = root
            .as_ref()
            .canonicalize()
            .map_err(internal("V3_ROOT_NOT_FOUND"))?;
        Ok(Self { root, store })
    }

    pub fn current_task_id(&self) -> Result<String, V3Error> {
        let pointer_path = self.root.join(".vibehub/tasks/current");
        let pointed_task_id = if pointer_path.is_dir() {
            fs::read_to_string(pointer_path.join("task.yaml"))
                .map_err(internal("V3_CURRENT_TASK_READ_FAILED"))
                .and_then(|content| {
                    serde_yaml::from_str::<TaskDocument>(&content).map_err(|error| {
                        V3Error::new(
                            "V3_CURRENT_TASK_INVALID",
                            V3ErrorCategory::CorruptLog,
                            false,
                            error.to_string(),
                        )
                    })
                })
                .map(|task| task.task_id)
        } else {
            resolve_current_task(&self.root)
                .map(|pointer| pointer.task_id)
                .map_err(|error| {
                    V3Error::new(
                        "V3_CURRENT_TASK_INVALID",
                        V3ErrorCategory::CorruptLog,
                        false,
                        error.to_string(),
                    )
                })
        };
        match pointed_task_id.and_then(|task_id| self.effective_current_task_id(task_id)) {
            Ok(task_id) => Ok(task_id),
            Err(error) if current_task_fallback_allowed(&error) => {
                self.fallback_current_task_id().or(Err(error))
            }
            Err(error) => Err(error),
        }
    }

    fn effective_current_task_id(&self, pointed_task_id: String) -> Result<String, V3Error> {
        let events = self.store.load_project(&self.project_id())?;
        let projection_status = self.store.projection_status(&self.project_id())?;
        let projection_stale = projection_is_stale(&projection_status);
        let tasks = self.read_tasks()?.tasks;
        let Some(pointed_task) = tasks.iter().find(|task| task.task_id == pointed_task_id) else {
            return self
                .select_fallback_task_id(&tasks, &events, projection_stale)
                .ok_or_else(|| {
                    V3Error::new(
                        "V3_CURRENT_TASK_INVALID",
                        V3ErrorCategory::CorruptLog,
                        false,
                        "current task is missing or has invalid V3 metadata",
                    )
                });
        };
        let pointed_lifecycle = fold_task(&pointed_task_id, &events);
        let pointed_integrity = session_integrity(&events, &pointed_task_id, &pointed_lifecycle);
        let pointed_state = task_state_for(
            &pointed_lifecycle,
            Some(pointed_task.phase_status.as_str()),
            &pointed_integrity,
            projection_stale,
        );
        if !is_terminal_task_state(pointed_state) {
            return Ok(pointed_task_id);
        }

        Ok(self
            .select_fallback_task_id(&tasks, &events, projection_stale)
            .unwrap_or(pointed_task_id))
    }

    fn fallback_current_task_id(&self) -> Result<String, V3Error> {
        let events = self.store.load_project(&self.project_id())?;
        let projection_status = self.store.projection_status(&self.project_id())?;
        let projection_stale = projection_is_stale(&projection_status);
        let tasks = self.read_tasks()?.tasks;
        self.select_fallback_task_id(&tasks, &events, projection_stale)
            .ok_or_else(|| {
                V3Error::new(
                    "V3_CURRENT_TASK_INVALID",
                    V3ErrorCategory::CorruptLog,
                    false,
                    "current task is invalid and no valid V3 task is available",
                )
            })
    }

    fn select_fallback_task_id(
        &self,
        tasks: &[TaskDocument],
        events: &[V3EventEnvelope],
        projection_stale: bool,
    ) -> Option<String> {
        tasks
            .iter()
            .find(|task| {
                let lifecycle = fold_task(&task.task_id, events);
                let integrity = session_integrity(events, &task.task_id, &lifecycle);
                let state = task_state_for(
                    &lifecycle,
                    Some(task.phase_status.as_str()),
                    &integrity,
                    projection_stale,
                );
                !is_terminal_task_state(state)
            })
            .or_else(|| tasks.first())
            .map(|task| task.task_id.clone())
    }

    pub fn project_id(&self) -> String {
        project_id(&self.root)
    }

    /// Return every non-terminal project Task without consulting the legacy
    /// current pointer as a write target. The pointer is exposed only as
    /// `is_current_default` metadata for compatibility and UI display.
    pub fn task_candidates(&self) -> Result<Value, V3Error> {
        let project_id = self.project_id();
        let events = self.store.load_project(&project_id)?;
        let projection_status = self.store.projection_status(&project_id)?;
        let projection_stale = projection_is_stale(&projection_status);
        let tasks = self.read_tasks()?;
        let current_default = self.current_task_id().ok();
        let candidates = tasks
            .tasks
            .iter()
            .filter_map(|task| {
                let lifecycle = fold_task(&task.task_id, &events);
                let integrity = session_integrity(&events, &task.task_id, &lifecycle);
                let state = task_state_for(
                    &lifecycle,
                    Some(task.phase_status.as_str()),
                    &integrity,
                    projection_stale,
                );
                if is_terminal_task_state(state) {
                    return None;
                }
                let active_session_count = sessions_for_task(&events, &task.task_id)
                    .into_iter()
                    .filter(|(_, open, closed)| *open && !*closed)
                    .count() as u64;
                Some(TaskRouteCandidate {
                    task_id: task.task_id.clone(),
                    title: task.title.clone(),
                    intent: task.intent.clone(),
                    state: state.to_owned(),
                    project_id: Some(project_id.clone()),
                    is_current_default: current_default.as_deref() == Some(task.task_id.as_str()),
                    is_ui_selected: false,
                    active_session_count,
                })
            })
            .collect::<Vec<_>>();
        serde_json::to_value(candidates).map_err(|error| {
            V3Error::new(
                "V3_SERIALIZE_FAILED",
                V3ErrorCategory::Internal,
                false,
                error.to_string(),
            )
        })
    }

    /// Return a stable project task index.
    ///
    /// `task_candidates` is intentionally the active-only routing surface.
    /// This index is the explicit query surface for archived tasks and keeps
    /// its filtering and ordering independent from the current/default task
    /// pointer. When `include_archived` is false, terminal tasks are omitted
    /// from `tasks` but counted in `omitted_archived_count`.
    pub fn task_list(&self, include_archived: bool) -> Result<Value, V3Error> {
        let project_id = self.project_id();
        let events = self.store.load_project(&project_id)?;
        let projection_status = self.store.projection_status(&project_id)?;
        let projection_stale = projection_is_stale(&projection_status);
        let tasks = self.read_tasks()?;
        let generated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let mut all: Vec<Value> = tasks
            .tasks
            .iter()
            .filter_map(|task| {
                let lifecycle = fold_task(&task.task_id, &events);
                let integrity = session_integrity(&events, &task.task_id, &lifecycle);
                let state = task_state_for(
                    &lifecycle,
                    Some(task.phase_status.as_str()),
                    &integrity,
                    projection_stale,
                );
                let is_terminal = is_terminal_task_state(state);
                if !include_archived && is_terminal {
                    return None;
                }
                Some(task_list_summary(
                    task,
                    &events,
                    &lifecycle,
                    state,
                    &generated_at,
                ))
            })
            .collect();
        sort_task_list(&mut all);
        let total = all.len();
        let archived_count = all
            .iter()
            .filter(|task| task.get("is_terminal").and_then(Value::as_bool) == Some(true))
            .count();
        let active_count = total.saturating_sub(archived_count);
        let project_task_count = tasks.tasks.len();
        let omitted_archived_count = if include_archived {
            0
        } else {
            project_task_count
                .saturating_sub(active_count)
                .saturating_sub(archived_count)
        };
        Ok(json!({
            "schema_version": "1.0",
            "tasks": all,
            "total_count": total,
            "returned_count": total,
            "active_count": active_count,
            "archived_count": archived_count,
            "project_task_count": project_task_count,
            "omitted_archived_count": omitted_archived_count,
            "include_archived": include_archived,
            "project_id": project_id,
            "sort": {
                "order": "non_terminal_first_then_terminal_at_desc",
                "terminal_at_missing": "last",
                "state_tie_breaker": "state_rank_ascending",
                "tie_breaker": "task_id_ascending"
            },
        }))
    }

    /// Return Task -> Session Git traceability for a task.
    pub fn task_commits(&self, task_id: &str) -> Result<Value, V3Error> {
        validate_id("task_id", task_id)?;
        let task = self.read_task(task_id)?;
        let project_id = self.project_id();
        let events = self.store.load_project(&project_id)?;
        let traces = session_git_traces(&events, task_id);
        let sessions = traces
            .iter()
            .map(session_git_trace_value)
            .collect::<Vec<_>>();
        let missing_heads = traces
            .iter()
            .filter_map(|trace| {
                let mut missing = Vec::new();
                if trace.open_git_head.is_none() {
                    missing.push("open_git_head");
                }
                if trace.close_git_head.is_none() {
                    missing.push("close_git_head");
                }
                (!missing.is_empty()).then(|| {
                    json!({
                        "task_id": task.task_id,
                        "session_id": trace.session_id,
                        "missing": missing,
                        "reason": "historical session event did not record a Git HEAD; no commit binding is inferred"
                    })
                })
            })
            .collect::<Vec<_>>();
        let complete_sessions = traces
            .iter()
            .filter(|trace| trace.open_git_head.is_some() && trace.close_git_head.is_some())
            .count();
        let partial_sessions = traces
            .iter()
            .filter(|trace| trace.open_git_head.is_some() ^ trace.close_git_head.is_some())
            .count();
        let missing_sessions = traces
            .iter()
            .filter(|trace| trace.open_git_head.is_none() && trace.close_git_head.is_none())
            .count();
        Ok(json!({
            "schema_version": "1.0",
            "task_id": task_id,
            "project_id": project_id,
            "session_count": traces.len(),
            "sessions": sessions,
            "git_evidence": {
                "status": if missing_sessions == traces.len() && !traces.is_empty() { "missing" } else if partial_sessions > 0 || missing_sessions > 0 { "partial" } else if complete_sessions > 0 { "complete" } else { "unavailable" },
                "complete_sessions": complete_sessions,
                "partial_sessions": partial_sessions,
                "missing_sessions": missing_sessions,
                "historical_gaps": missing_heads,
            },
            "source": "v3_session_git_head_events",
        }))
    }

    /// Return all Tasks associated with a Git commit hash.
    ///
    /// A commit is associated from explicit event commit evidence or from a
    /// recorded session open/close range. Missing historical HEADs never
    /// become an inferred association. Unknown but syntactically valid hashes
    /// return an empty `tasks` array; ambiguous short hashes fail explicitly.
    pub fn commit_tasks(&self, commit_hash: &str) -> Result<Value, V3Error> {
        let query_hash = normalize_commit_hash(commit_hash)?;
        let project_id = self.project_id();
        let events = self.store.load_project(&project_id)?;
        let tasks = self.read_tasks()?;
        let projection_status = self.store.projection_status(&project_id)?;
        let projection_stale = projection_is_stale(&projection_status);
        let stored_hashes = collect_stored_commit_hashes(&events);
        let resolved_hash = resolve_commit_hash(&self.root, &query_hash, &stored_hashes)?;
        let comparison_hash = resolved_hash.as_deref().unwrap_or(query_hash.as_str());
        let mut associations = BTreeMap::<String, Value>::new();
        let mut historical_gaps = Vec::new();

        for task in &tasks.tasks {
            let task_events = events
                .iter()
                .filter(|event| event.task_id.0 == task.task_id)
                .collect::<Vec<_>>();
            let lifecycle = fold_task(&task.task_id, &events);
            let integrity = session_integrity(&events, &task.task_id, &lifecycle);
            let state = task_state_for(
                &lifecycle,
                Some(task.phase_status.as_str()),
                &integrity,
                projection_stale,
            );
            let traces = session_git_traces(&events, &task.task_id);
            for trace in &traces {
                let mut missing = Vec::new();
                if trace.open_git_head.is_none() {
                    missing.push("open_git_head");
                }
                if trace.close_git_head.is_none() {
                    missing.push("close_git_head");
                }
                if !missing.is_empty() {
                    historical_gaps.push(json!({
                        "task_id": task.task_id,
                        "session_id": trace.session_id,
                        "missing": missing,
                        "reason": "historical session event did not record a Git HEAD; range matching is unavailable"
                    }));
                }
                let mut match_kind = None;
                if trace
                    .open_git_head
                    .as_deref()
                    .is_some_and(|head| stored_hash_matches(comparison_hash, head))
                {
                    match_kind = Some("open_head");
                } else if trace
                    .close_git_head
                    .as_deref()
                    .is_some_and(|head| stored_hash_matches(comparison_hash, head))
                {
                    match_kind = Some("close_head");
                } else if let (Some(open), Some(close), Some(resolved)) = (
                    trace.open_git_head.as_deref(),
                    trace.close_git_head.as_deref(),
                    resolved_hash.as_deref(),
                ) {
                    if git_commit_in_session_range(&self.root, resolved, open, close) {
                        match_kind = Some("session_range");
                    }
                }
                if let Some(match_kind) = match_kind {
                    add_commit_task_association(
                        &mut associations,
                        task,
                        state,
                        &trace.session_id,
                        match_kind,
                        trace,
                    );
                }
            }
            for event in task_events {
                if event
                    .commit_sha
                    .as_deref()
                    .is_some_and(|sha| stored_hash_matches(comparison_hash, sha))
                {
                    let session_id = event
                        .session_id
                        .as_ref()
                        .map(|id| id.0.as_str())
                        .unwrap_or("session.unknown");
                    add_commit_task_association(
                        &mut associations,
                        task,
                        state,
                        session_id,
                        "event_commit",
                        &SessionGitTrace::default(),
                    );
                }
            }
        }

        let task_values = associations.into_values().collect::<Vec<_>>();
        Ok(json!({
            "schema_version": "1.0",
            "project_id": project_id,
            "query_commit_hash": query_hash,
            "resolved_commit_hash": resolved_hash,
            "match": if task_values.is_empty() { "none" } else { "associated" },
            "task_count": task_values.len(),
            "tasks": task_values,
            "historical_gaps": historical_gaps,
            "source": "v3_git_traceability",
        }))
    }

    pub fn load_bundle(&self, task_id: &str) -> Result<V3ViewBundle, V3Error> {
        self.load_bundle_for_node(task_id, None)
    }

    pub fn load_bundle_for_node(
        &self,
        task_id: &str,
        requested_node_id: Option<&str>,
    ) -> Result<V3ViewBundle, V3Error> {
        validate_id("task_id", task_id)?;
        let task = self.read_task(task_id)?;
        let project_id = self.project_id();
        let events = self.store.load_project(&project_id)?;
        let projection_status = self.store.projection_status(&project_id)?;
        let projection_stale = projection_is_stale(&projection_status);
        let projection_warning = projection_warning(&projection_status);
        let lifecycle = fold_task(task_id, &events);
        let task_integrity = session_integrity(&events, task_id, &lifecycle);
        let session_binding_projection = projection::fold(&project_id, &events).session_bindings;
        let orchestration = fold_orchestration(task_id, &events);
        let generated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let current_criteria = criteria(&task, &events, &lifecycle, &generated_at);
        let mut current_blocker_details =
            blocker_details(&task, &events, &lifecycle, None, &generated_at);
        if projection_stale {
            current_blocker_details.push(projection_stale_blocker_detail(
                &project_id,
                &projection_status,
                &generated_at,
            ));
        }
        let evidence_refs = evidence_refs(&task, &events, &generated_at);
        let native_root = native_path(&self.root);
        let sessions = sessions_for_task(&events, task_id);
        let opened_sessions = sessions.iter().filter(|session| session.1).count();
        let closed_sessions = sessions.iter().filter(|session| session.2).count();
        let explicit_session_gaps = task_integrity.blocking_session_ids().len();
        let current_task_state = task_state_for(
            &lifecycle,
            Some(task.phase_status.as_str()),
            &task_integrity,
            projection_stale,
        );
        let task_read = self.read_tasks()?;
        let task_metadata_warnings = task_read.warnings;
        let project_tasks = task_read.tasks;
        let effective_current_task_id = self.current_task_id().ok();
        let active_tasks: Vec<Value> = project_tasks
            .iter()
            .filter_map(|project_task| {
                let task_lifecycle = fold_task(&project_task.task_id, &events);
                let task_integrity = session_integrity(&events, &project_task.task_id, &task_lifecycle);
                let state = task_state_for(
                    &task_lifecycle,
                    Some(project_task.phase_status.as_str()),
                    &task_integrity,
                    projection_stale,
                );
                if is_terminal_task_state(state) {
                    return None;
                }
                let task_sessions = sessions_for_task(&events, &project_task.task_id);
                let task_opened_sessions = task_sessions.iter().filter(|session| session.1).count();
                let task_closed_sessions = task_sessions.iter().filter(|session| session.2).count();
                let task_criteria = criteria(project_task, &events, &task_lifecycle, &generated_at);
                let mut task_blocker_details =
                    blocker_details(project_task, &events, &task_lifecycle, None, &generated_at);
                if projection_stale {
                    task_blocker_details.push(projection_stale_blocker_detail(
                        &project_id,
                        &projection_status,
                        &generated_at,
                    ));
                }
                Some(json!({
                    "task_id": project_task.task_id,
                    "title": project_task.title,
                    "intent": project_task.intent,
                    "workflow_profile": project_task.workflow_profile,
                    "state": state,
                    "risk_level": risk_level(&events, &project_task.task_id),
                    "criteria": task_criteria,
                    "blocker_details": task_blocker_details,
                    "relations": task_lifecycle.relations,
                    "active_sessions": task_opened_sessions.saturating_sub(task_closed_sessions),
                    "is_current_default": effective_current_task_id.as_deref() == Some(project_task.task_id.as_str()),
                    "is_ui_selected": false
                }))
            })
            .collect();
        let mut archived_tasks: Vec<Value> = project_tasks
            .iter()
            .filter_map(|project_task| {
                let task_lifecycle = fold_task(&project_task.task_id, &events);
                let task_integrity =
                    session_integrity(&events, &project_task.task_id, &task_lifecycle);
                let state = task_state_for(
                    &task_lifecycle,
                    Some(project_task.phase_status.as_str()),
                    &task_integrity,
                    projection_stale,
                );
                is_terminal_task_state(state).then(|| {
                    archived_task_summary(
                        project_task,
                        &events,
                        &task_lifecycle,
                        state,
                        &generated_at,
                    )
                })
            })
            .collect();
        sort_archived_tasks_newest_first(&mut archived_tasks);
        let workspace = resolve_workspace(&self.root, task_id, &events);
        let workspace_native_root = native_path(&workspace.root);
        let resolved_scopes = resolve_project_scopes(&self.root, Some(&workspace.root))?;
        let scope_inspection = resolved_scopes.inspection();
        let index = ProjectIndexService::open(&workspace.root).and_then(|service| {
            let snapshot = service.current_snapshot()?;
            let page = service.first_page()?;
            Ok((page, snapshot))
        });
        let (index_page, index_snapshot) = match index {
            Ok((page, snapshot)) => (Some(page), Some(snapshot)),
            Err(_) => (None, None),
        };
        let indexed_files = index_snapshot
            .as_ref()
            .map(|snapshot| snapshot.indexed_files)
            .unwrap_or(0);
        let project_structure = project_structure_view(
            &workspace.root,
            &project_id,
            &generated_at,
            &evidence_refs,
            index_page,
            index_snapshot.as_ref(),
            &workspace,
        );
        let architecture_nodes = project_structure["architecture_nodes"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let architecture_edges = project_structure["architecture_edges"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let architecture_modules = architecture_nodes
            .iter()
            .filter(|node| node["kind"] != "workspace")
            .count();
        let declared_docs = architecture_nodes
            .iter()
            .filter(|node| node["source_kind"] == "documentation")
            .count();
        let architecture_claims = architecture_nodes
            .iter()
            .filter(|node| node["kind"] != "workspace")
            .chain(architecture_edges.iter())
            .filter_map(|claim| claim["confidence"].as_f64())
            .collect::<Vec<_>>();
        let architecture_confidence = if architecture_claims.is_empty() {
            0.0
        } else {
            architecture_claims.iter().sum::<f64>() / architecture_claims.len() as f64
        };
        let architecture_evidence_refs = architecture_view_evidence_refs(&project_structure);
        let architecture_warnings = project_structure["warnings"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let architecture_errors = project_structure["errors"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let mut overview_warnings = task_metadata_warnings.clone();
        overview_warnings.extend(architecture_warnings.clone());
        if let Some(warning) = projection_warning.clone() {
            overview_warnings.push(warning);
        }
        overview_warnings.extend(scope_inspection.warnings.iter().map(|warning| {
            json!({
                "code": warning.split(':').next().unwrap_or("V3_PROJECT_SCOPE_WARNING"),
                "severity": "warning",
                "message_key": "v3.warning.project_scope",
                "details": {"scope_warning": warning},
                "evidence_refs": []
            })
        }));
        let index_state = project_structure["index_state"].as_str().unwrap_or("error");
        let model_state = match index_state {
            "ready" => "ready",
            "indexing" => "rebuilding",
            "uninitialized" => "uninitialized",
            _ => "error",
        };
        let structure_freshness = project_structure["freshness"]
            .as_str()
            .unwrap_or("unavailable");
        let view_freshness = if projection_stale {
            "stale"
        } else {
            structure_freshness
        };
        let structure_completeness = project_structure["completeness"]
            .as_str()
            .unwrap_or("unknown");
        let overview_completeness = if projection_stale || !task_metadata_warnings.is_empty() {
            "partial"
        } else {
            structure_completeness
        };
        let structure_model_version = project_structure["model_version"]
            .as_str()
            .unwrap_or(MODEL_VERSION);

        let project_overview = json!({
            "schema_version": "1.0", "project_id": project_id, "name": self.root.file_name().and_then(|v| v.to_str()).unwrap_or("Project"),
            "root": native_root, "generated_at": generated_at, "model_version": MODEL_VERSION,
            "scopes": scope_inspection,
            "freshness": view_freshness, "completeness": overview_completeness,
            "repository": {"state": if resolved_scopes.git_root.is_some() {"available"} else {"not_repository"}, "branch": Value::Null, "head": Value::Null, "dirty": Value::Null, "worktree_count": if resolved_scopes.git_root.is_some() {1} else {0}},
            "model": {"state": model_state, "last_evidence_at": generated_at, "generator_version": structure_model_version, "indexed_files": indexed_files},
            "architecture": {"declared_docs": declared_docs, "modules": architecture_modules, "relationships": architecture_edges.len(), "confidence": architecture_confidence, "evidence_refs": architecture_evidence_refs},
            "current_task_id": effective_current_task_id,
            "current_default_task_id": effective_current_task_id,
            "ui_selected_task_id": Value::Null,
            "session_bindings": session_binding_projection.clone(),
            "active_tasks": active_tasks,
            "archived_tasks": archived_tasks,
            "protocol_coverage": {"state": protocol_coverage(opened_sessions, closed_sessions, explicit_session_gaps), "opened_sessions": opened_sessions, "closed_sessions": closed_sessions, "gaps": explicit_session_gaps},
            "evidence_refs": evidence_refs, "warnings": overview_warnings, "errors": architecture_errors
        });
        let agent_results =
            agent_results_view(&project_id, task_id, &generated_at, &events, &evidence_refs);

        let lanes: Vec<Value> = lifecycle
            .sessions
            .values()
            .map(|session| {
                json!({
                    "lane_id": session.session_id, "kind": "session", "label": session.session_id,
                    "state": match session.state.as_str() {
                        "active" => "active", "closed" => "closed", "gapped" => "gapped",
                        "repaired" => "repaired", _ => "idle"
                    }
                })
            })
            .collect();
        let timeline_events: Vec<Value> = events
            .iter()
            .filter(|event| event.task_id.0 == task_id)
            .map(timeline_event)
            .collect();
        let task_bindings = session_binding_projection
            .iter()
            .filter(|(_, binding)| binding.bound_task_id.as_deref() == Some(task_id))
            .collect::<BTreeMap<_, _>>();
        let mut timeline_warnings = task_metadata_warnings.clone();
        if let Some(warning) = projection_warning.clone() {
            timeline_warnings.push(warning);
        }
        let task_timeline = json!({
            "schema_version": "1.0", "project_id": project_id, "task_id": task.task_id, "title": task.title, "state": current_task_state,
            "generated_at": generated_at, "model_version": MODEL_VERSION, "freshness": view_freshness, "completeness": if projection_stale {"partial"} else {"complete"},
            "criteria": current_criteria, "blocker_details": current_blocker_details.clone(), "completion": completion_view(&lifecycle), "lanes": lanes, "events": timeline_events, "session_bindings": task_bindings, "window": page(events.len()),
            "evidence_refs": evidence_refs, "warnings": timeline_warnings, "errors": []
        });

        let (parallel_layers, parallel_layer_counts) = effective_plan_layers(&lifecycle);
        let graph_nodes: Vec<Value> = lifecycle
            .effective_nodes()
            .map(|(_, node)| {
                let session_ids = lifecycle
                    .sessions
                    .values()
                    .filter(|session| session.node_id.as_deref() == Some(node.node_id.as_str()))
                    .map(|session| session.session_id.clone())
                    .collect::<Vec<_>>();
                let session_id_set = session_ids.iter().cloned().collect::<BTreeSet<_>>();
                let result_events = events
                    .iter()
                    .filter(|event| {
                        event.task_id.0 == task_id
                            && event.event_type == "agent.result_recorded"
                            && (event
                                .node_id
                                .as_ref()
                                .is_some_and(|id| id.0 == node.node_id)
                                || event
                                    .session_id
                                    .as_ref()
                                    .is_some_and(|id| session_id_set.contains(&id.0)))
                    })
                    .collect::<Vec<_>>();
                let agent_result_ids = result_events
                    .iter()
                    .map(|event| {
                        event
                            .payload
                            .get("result_id")
                            .and_then(Value::as_str)
                            .filter(|id| !id.is_empty())
                            .unwrap_or(event.event_id.as_str())
                            .to_owned()
                    })
                    .collect::<Vec<_>>();
                let execution_state = if result_events.iter().any(|event| {
                    event.payload.get("status").and_then(Value::as_str) == Some("failed")
                }) {
                    "failed"
                } else if result_events.iter().any(|event| {
                    event.payload.get("status").and_then(Value::as_str) == Some("succeeded")
                }) {
                    "succeeded"
                } else if session_ids.iter().any(|session_id| {
                    lifecycle
                        .sessions
                        .get(session_id)
                        .is_some_and(|session| session.state == "active")
                }) {
                    "session_active"
                } else if session_ids.is_empty() {
                    "not_started"
                } else {
                    "observed"
                };
                let layer = parallel_layers.get(&node.node_id).copied().unwrap_or(0);
                let node_blocker_details = blocker_details(
                    &task,
                    &events,
                    &lifecycle,
                    Some(node.node_id.as_str()),
                    &generated_at,
                );
                json!({
                    "node_id": node.node_id, "title": node.title, "goal": node.goal, "state": view_node_state(&node.state),
                    "readiness": if node.state == "blocked" || node.state == "failed" {"blocked"} else if node.dependencies.iter().all(|dependency| lifecycle.nodes.get(dependency).is_some_and(|value| value.is_historical_bootstrap() || value.state == "completed")) {"ready"} else {"blocked"},
                    "block_reasons": if node.state == "blocked" {vec!["lifecycle.blocked"]} else {Vec::<&str>::new()},
                    "blocker_details": node_blocker_details,
                    "scope": node.scope, "criterion_ids": node.criterion_ids, "session_ids": session_ids,
                    "agent_result_ids": agent_result_ids, "parallel_layer": layer,
                    "parallel_candidate": parallel_layer_counts.get(&layer).copied().unwrap_or(0) > 1,
                    "execution_state": execution_state
                })
            })
            .collect();
        let scheduling_edges: Vec<Value> = lifecycle
            .effective_nodes()
            .flat_map(|(_, node)| {
                node.dependencies
                    .iter()
                    .filter(|dependency| lifecycle.effective_node(dependency).is_some())
                    .map(|dependency| json!({
                        "edge_id": format!("schedule.{}.{}", dependency, node.node_id), "from_node_id": dependency, "to_node_id": node.node_id, "kind": "depends_on"
                    }))
                    .collect::<Vec<_>>()
            })
            .collect();
        let trace_relations: Vec<Value> = lifecycle.findings.values().flat_map(|finding| finding.attempt_ids.iter().map(|attempt_id| json!({
            "relation_id": format!("trace.{}.{}", attempt_id, finding.finding_id), "from_id": attempt_id, "to_id": finding.finding_id, "kind": "addresses", "evidence_refs": []
        }))).collect();
        let node_id = select_node_id(&lifecycle, requested_node_id)?;
        let mut plan_warnings = task_metadata_warnings.clone();
        if let Some(warning) = projection_warning.clone() {
            plan_warnings.push(warning);
        }
        let effective_plan_is_empty = lifecycle.effective_nodes().next().is_none();
        if effective_plan_is_empty && task.workflow_profile != "lightweight" {
            plan_warnings.push(json!({
                "code": "V3_PLAN_NOT_RECORDED",
                "severity": "warning",
                "message_key": "v3.warning.plan_not_recorded",
                "details": {},
                "evidence_refs": evidence_refs
            }));
        }
        let planned_worktrees = orchestration.worktrees.len();
        let observed_worktrees = orchestration
            .worktrees
            .values()
            .filter(|worktree| worktree.event_ids.len() > 1)
            .count();
        let plan_graph = json!({
            "schema_version": "1.0", "project_id": project_id, "task_id": task.task_id, "plan_version": lifecycle.version,
            "workflow_profile": task.workflow_profile, "planning_required": task.workflow_profile != "lightweight",
            "generated_at": generated_at, "model_version": MODEL_VERSION, "freshness": view_freshness, "completeness": if projection_stale {"partial"} else if effective_plan_is_empty && task.workflow_profile == "lightweight" {"complete"} else if effective_plan_is_empty {"unknown"} else {"partial"}, "graph_state": "valid",
            "nodes": graph_nodes,
            "scheduling_edges": scheduling_edges, "trace_relations": trace_relations,
            "execution": {"planned_sessions": 0, "observed_sessions": sessions.len(), "planned_worktrees": planned_worktrees, "observed_worktrees": observed_worktrees},
            "evidence_refs": evidence_refs, "warnings": plan_warnings, "errors": []
        });

        let mut node_warnings = task_metadata_warnings;
        node_warnings.extend(architecture_warnings.clone());
        if let Some(warning) = projection_warning.clone() {
            node_warnings.push(warning);
        }
        let (
            max_tokens,
            estimated_tokens,
            legacy_milestone_policy,
            legacy_planning_required,
            legacy_review_required,
        ) = match task.workflow_profile.as_str() {
            "lightweight" => (2000, 400, "minimal", false, false),
            "full" => (16000, 2000, "full", true, true),
            _ => (8000, 1000, "standard", true, true),
        };
        let effective_profile = if task.effective_profile.is_empty() {
            task.workflow_profile.clone()
        } else {
            task.effective_profile.clone()
        };
        let recommended_profile = if task.recommended_profile.is_empty() {
            task.workflow_profile.clone()
        } else {
            task.recommended_profile.clone()
        };
        let effective_policy = lifecycle
            .execution_policy
            .as_ref()
            .or(task.execution_policy.as_ref());
        let policy_version = effective_policy
            .map(|policy| policy.policy_version)
            .unwrap_or(task.policy_version);
        let enforcement_epoch = effective_policy
            .map(|policy| policy.enforcement_epoch.clone())
            .unwrap_or_else(|| task.enforcement_epoch.clone());
        let milestone_policy = effective_policy
            .map(|policy| policy.milestone_policy.as_str())
            .unwrap_or(legacy_milestone_policy);
        let planning_required = effective_policy
            .map(|policy| policy.planning_required)
            .unwrap_or(legacy_planning_required);
        let review_required = effective_policy
            .map(|policy| policy.review_required)
            .unwrap_or(legacy_review_required);
        let required_records = effective_policy
            .map(|policy| policy.required_records.clone())
            .unwrap_or_else(|| {
                if task.workflow_profile == "lightweight" {
                    vec![
                        "session".to_owned(),
                        "result".to_owned(),
                        "risk_if_any".to_owned(),
                    ]
                } else {
                    vec![
                        "plan".to_owned(),
                        "session".to_owned(),
                        "progress".to_owned(),
                        "result".to_owned(),
                        "review".to_owned(),
                    ]
                }
            });
        let task_events = events
            .iter()
            .filter(|event| event.task_id.0 == task.task_id)
            .collect::<Vec<_>>();
        let record_present = |record: &str| match record {
            "plan" => lifecycle.effective_nodes().next().is_some(),
            "session" => task_events
                .iter()
                .any(|event| event.event_type == "session.opened"),
            "progress" => task_events
                .iter()
                .any(|event| event.event_type == "progress.logged"),
            "result" => task_events
                .iter()
                .any(|event| event.event_type == "agent.result_recorded"),
            "review" => lifecycle.criteria.values().any(|criterion| {
                matches!(
                    criterion.state,
                    CriterionState::Passed | CriterionState::Failed | CriterionState::Blocked
                )
            }),
            "risk_if_any" => true,
            _ => false,
        };
        let protocol_records=required_records.iter().map(|record|{let present=record_present(record);json!({"record":record,"status":if present{"complete"}else{"missing"},"repair_action":if present{Value::Null}else{Value::String(format!("record_{record}_via_typed_command"))}})}).collect::<Vec<_>>();
        let protocol_state = if protocol_records
            .iter()
            .all(|record| record["status"] == "complete")
        {
            "complete"
        } else {
            "partial"
        };
        let incomplete_nodes = lifecycle
            .effective_nodes()
            .filter(|(_, node)| !is_terminal_plan_node_state(node.state.as_str()))
            .map(|(_, node)| format!("{}={}", node.node_id, node.state))
            .collect::<Vec<_>>();
        let unsettled_sessions = lifecycle
            .sessions
            .values()
            .filter(|session| session.state != "closed")
            .map(|session| format!("{}={}", session.session_id, session.state))
            .chain(
                task_integrity
                    .blocking_session_ids()
                    .into_iter()
                    .filter(|session_id| !lifecycle.sessions.contains_key(session_id))
                    .map(|session_id| format!("{session_id}=unknown_or_incomplete")),
            )
            .collect::<Vec<_>>();
        let opened_session_ids = task_events
            .iter()
            .filter(|event| event.event_type == "session.opened")
            .filter_map(|event| event.session_id.as_ref().map(|id| id.0.clone()))
            .collect::<BTreeSet<_>>();
        let terminal_result_session_ids = task_events
            .iter()
            .filter(|event| {
                event.event_type == "agent.result_recorded"
                    && matches!(
                        event.payload.get("status").and_then(Value::as_str),
                        Some("succeeded" | "failed")
                    )
            })
            .filter_map(|event| event.session_id.as_ref().map(|id| id.0.clone()))
            .collect::<BTreeSet<_>>();
        let sessions_without_result = opened_session_ids
            .difference(&terminal_result_session_ids)
            .cloned()
            .collect::<Vec<_>>();
        let progress_session_ids = task_events
            .iter()
            .filter(|event| event.event_type == "progress.logged")
            .filter_map(|event| event.session_id.as_ref().map(|id| id.0.clone()))
            .collect::<BTreeSet<_>>();
        let recovered_session_ids = task_events
            .iter()
            .filter(|event| event.event_type == "session.recovered")
            .filter_map(|event| event.session_id.as_ref().map(|id| id.0.clone()))
            .collect::<BTreeSet<_>>();
        let risk_session_ids = task_events
            .iter()
            .filter(|event| event.event_type == "risk.logged")
            .filter_map(|event| event.session_id.as_ref().map(|id| id.0.clone()))
            .collect::<BTreeSet<_>>();
        let failed_result_session_ids = task_events
            .iter()
            .filter(|event| {
                event.event_type == "agent.result_recorded"
                    && event.payload.get("status").and_then(Value::as_str) == Some("failed")
            })
            .filter_map(|event| event.session_id.as_ref().map(|id| id.0.clone()))
            .collect::<BTreeSet<_>>();
        let sessions_without_progress = if required_records.contains(&"progress".to_owned()) {
            opened_session_ids
                .iter()
                .filter(|session_id| {
                    !progress_session_ids.contains(*session_id)
                        && !recovered_session_ids.contains(*session_id)
                        && !(risk_session_ids.contains(*session_id)
                            && failed_result_session_ids.contains(*session_id))
                })
                .cloned()
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let criterion_issues = lifecycle
            .criteria
            .values()
            .filter_map(|criterion| {
                if !matches!(
                    criterion.state,
                    CriterionState::Passed | CriterionState::NotApplicable
                ) {
                    Some(format!("{}={:?}", criterion.criterion_id, criterion.state))
                } else if criterion.state != CriterionState::NotApplicable
                    && !criterion
                        .evidence_refs
                        .iter()
                        .any(|reference| valid_evidence_ref(reference))
                {
                    Some(format!(
                        "{}=invalid_or_stale_evidence",
                        criterion.criterion_id
                    ))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let open_findings = lifecycle
            .findings
            .values()
            .filter(|finding| finding.state != "closed")
            .map(|finding| format!("{}={}", finding.finding_id, finding.state))
            .collect::<Vec<_>>();
        let unsettled_worktrees = orchestration
            .worktrees
            .values()
            .filter_map(|worktree| {
                let lease_active = worktree
                    .lease
                    .as_ref()
                    .is_some_and(|lease| lease.state == LeaseState::Active);
                (!matches!(
                    worktree.state,
                    WorktreeState::Integrated | WorktreeState::Cleaned | WorktreeState::Abandoned
                ) || lease_active)
                    .then(|| {
                        format!(
                            "{}={:?},lease_active={lease_active}",
                            worktree.worktree_id, worktree.state
                        )
                    })
            })
            .collect::<Vec<_>>();
        let external_blockers = current_blocker_details
            .iter()
            .filter(|blocker| {
                matches!(
                    blocker.get("kind").and_then(Value::as_str),
                    Some("external_precondition" | "permission")
                )
            })
            .filter_map(|blocker| {
                blocker
                    .get("blocker_id")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .collect::<Vec<_>>();
        let missing_records = protocol_records
            .iter()
            .filter(|record| record["status"] == "missing")
            .filter_map(|record| record["record"].as_str().map(str::to_owned))
            .collect::<Vec<_>>();
        let items = vec![
            completion_gate_item("required_records", "completion.records_missing", missing_records.is_empty(), "所有 effective policy required_records 均存在", format!("缺少记录：{}", missing_records.join(", ")), missing_records.clone(), Vec::new(), missing_records.iter().map(|record| format!("通过 typed command 记录 {record}" )).collect()),
            completion_gate_item("plan_terminal", "completion.plan_non_terminal", !planning_required || (lifecycle.effective_nodes().next().is_some() && incomplete_nodes.is_empty()), "所有必需 plan node 均为 completed/waived/superseded/cancelled", format!("未终结节点：{}", incomplete_nodes.join(", ")), incomplete_nodes.clone(), Vec::new(), vec!["按 DAG 顺序完成、waive、supersede 或 cancel 未终结节点".to_owned()]),
            completion_gate_item("sessions_settled", "completion.session_unsettled", unsettled_sessions.is_empty(), "所有 session 均 closed，gap 已 recover", format!("未结 session：{}", unsettled_sessions.join(", ")), unsettled_sessions.clone(), Vec::new(), vec!["对 gapped session 先 session_recovery(recover)，记录 terminal result 后 session_close".to_owned()]),
            completion_gate_item("results_terminal", "completion.result_missing_or_non_terminal", sessions_without_result.is_empty(), "每个 opened session 都有 succeeded/failed terminal AgentResult", format!("缺少 terminal result 的 session：{}", sessions_without_result.join(", ")), sessions_without_result.clone(), Vec::new(), vec!["为列出的 session 调用 agent_result_record(status=succeeded|failed)".to_owned()]),
            completion_gate_item("progress_evidence", "completion.progress_missing", sessions_without_progress.is_empty(), "policy 要求 progress 时，每个 session 都有 progress 事件，或中断 session 有 session.recovered 证据，或阻塞 session 同时有 risk 事件与 failed terminal result", format!("缺少 progress/recovery 证据的 session：{}", sessions_without_progress.join(", ")), sessions_without_progress.clone(), Vec::new(), vec!["session 仍开启时调用 event_log(kind=progress)；已中断则先 session_recovery(action=recover, evidence_refs=[...]) 再结束；阻塞收尾则记录 event_log(kind=risk) 与 agent_result_record(status=failed)".to_owned()]),
            completion_gate_item("criteria_green", "completion.review_or_evidence_not_green", criterion_issues.is_empty(), "所有 required criterion 为 passed/not_applicable，且 passed evidence 有效且未标记 stale", format!("未通过项：{}", criterion_issues.join("；")), criterion_issues.clone(), current_blocker_details.iter().filter(|blocker| blocker.get("source_type").and_then(Value::as_str) == Some("criterion")).filter_map(|blocker| blocker.get("blocker_id").and_then(Value::as_str).map(str::to_owned)).collect(), vec!["执行每项真实验证；缺记录则 review，blocked 则解除环境条件，invalid/stale evidence 则重新采集".to_owned()]),
            completion_gate_item("findings_closed", "completion.finding_open", open_findings.is_empty(), "所有 finding 均有 remediation attempt 且 closed", format!("未闭环 finding：{}", open_findings.join(", ")), open_findings.clone(), current_blocker_details.iter().filter(|blocker| blocker.get("source_type").and_then(Value::as_str) == Some("finding")).filter_map(|blocker| blocker.get("blocker_id").and_then(Value::as_str).map(str::to_owned)).collect(), vec!["记录 remediation attempt 与验证 evidence，然后关闭 finding".to_owned()]),
            completion_gate_item("orchestration_settled", "completion.worktree_or_lease_unsettled", unsettled_worktrees.is_empty(), "所有 worktree 已 integrated/cleaned/abandoned，且无 active lease", format!("未结 orchestration：{}", unsettled_worktrees.join("；")), unsettled_worktrees.clone(), current_blocker_details.iter().filter(|blocker| matches!(blocker.get("source_type").and_then(Value::as_str), Some("worktree" | "lease" | "integration"))).filter_map(|blocker| blocker.get("blocker_id").and_then(Value::as_str).map(str::to_owned)).collect(), vec!["完成或安全放弃 integration，并 release/reclaim lease".to_owned()]),
            completion_gate_item("external_preconditions", "completion.external_or_permission_blocked", external_blockers.is_empty(), "无等待外部环境或权限的 blocker", format!("外部/权限 blocker：{}", external_blockers.join(", ")), external_blockers.clone(), external_blockers.clone(), vec!["按 blocker repair_actions 在指定环境或受信人工渠道解除前置条件".to_owned()]),
        ];
        let completion_gate = json!({
            "all_passed": items.iter().all(|item| item["passed"] == true),
            "blocked_chain": items.iter().filter(|item| item["passed"] == false).map(|item| item["gate"].clone()).collect::<Vec<_>>(),
            "items": items
        });
        let selected_node = lifecycle.effective_node(&node_id);
        let memory_scope = selected_node
            .map(|node| node.scope.clone())
            .unwrap_or_default();
        let actor = events
            .iter()
            .rev()
            .find(|event| {
                event.task_id.0 == task.task_id
                    && event.node_id.as_ref().is_some_and(|id| id.0 == node_id)
            })
            .map(|event| format!("user:{}", event.actor))
            .unwrap_or_else(|| "team".to_owned());
        let memory_projection = super::project_memory::fold(&project_id, &events);
        let injected_memory = super::project_memory::query(&memory_projection, &super::MemoryQuery { kinds: Vec::new(), scope: memory_scope.clone(), principal_scope: Some(actor), include_stale: false, include_disputed: false, token_budget: max_tokens / 4 }).into_iter().filter(|entry| !super::prompt_injection_suspected(&entry.content)).map(|entry| json!({
            "entry_id": entry.entry_id, "kind": entry.kind, "source": "project_memory", "revision": entry.revision, "status": entry.status,
            "evidence_refs": entry.evidence_refs, "content": entry.content, "why_injected": format!("scope/freshness/precedence match; precedence={}", super::memory_precedence(&entry.kind)),
            "instructional": false, "security_boundary": "untrusted_data_only"
        })).collect::<Vec<_>>();
        let node_brief = json!({
            "schema_version": "1.0", "project_id": project_id, "task_id": task.task_id, "node_id": node_id,
            "workflow_profile": task.workflow_profile,
            "execution_policy": {
                "recommended_profile": recommended_profile,
                "effective_profile": effective_profile,
                "policy_version": policy_version,
                "enforcement_epoch": enforcement_epoch,
                "trigger_reasons": task.trigger_reasons,
                "override_record": effective_policy.and_then(|policy| policy.override_record.clone()),
                "upgrade_history": effective_policy.map(|policy| policy.upgrade_history.clone()).unwrap_or_default(),
                "milestone_policy": milestone_policy,
                "planning_required": planning_required,
                "review_required": review_required,
                "required_records": required_records
            },
            "generated_at": generated_at, "model_version": MODEL_VERSION, "freshness": view_freshness, "completeness": if projection_stale {"partial"} else {structure_completeness},
            "goal": selected_node.map(|node| node.goal.as_str()).unwrap_or(&task.intent),
            "scope": selected_node.map(|node| node.scope.clone()).unwrap_or_default(), "non_scope": [],
            "dependencies": selected_node.map(|node| node.dependencies.iter().cloned().collect::<Vec<_>>()).unwrap_or_default(),
            "accepted_decisions": [],
            "project_memory": injected_memory,
            "protocol_records": protocol_records,
            "coverage_mode": if effective_policy.is_some() {"enforced"} else {"legacy_degraded"},
            "completion_gate": completion_gate,
            "research_summary": [], "criteria": current_criteria, "files": [workspace_native_root],
            // `state` remains the selected plan-node state for the plan drawer.
            // `task_state` is the canonical task conclusion shared with the
            // timeline, overview and candidates; keeping both named avoids
            // turning a historical node/task mismatch into a silent rewrite.
            "validation_commands": [], "state": selected_node.map(|node| view_node_state(&node.state)).unwrap_or_else(|| node_state(&task)), "task_state": current_task_state, "next_intent": task.phase,
            "blocker_details": current_blocker_details,
            "budget": {"max_tokens": max_tokens, "estimated_tokens": estimated_tokens, "truncated_sections": []},
            "source_versions": {"view_model": MODEL_VERSION, "project_model": structure_model_version}, "protocol_coverage": if explicit_session_gaps>0{"gapped"}else{protocol_state},
            "evidence_refs": evidence_refs, "warnings": node_warnings, "errors": architecture_errors
        });

        Ok(V3ViewBundle {
            project_overview,
            project_structure,
            agent_results,
            task_timeline,
            plan_graph,
            node_brief,
        })
    }

    /// Load only the NodeBrief of one explicitly requested plan node.
    ///
    /// The cockpit plan view needs the brief of whatever node the user clicked,
    /// including `completed` ones that `select_node_id` never prefers. Returning
    /// the whole bundle for that would ship megabytes of unrelated project
    /// structure on every click, so this keeps the payload to the brief itself.
    pub fn load_node_brief(&self, task_id: &str, node_id: &str) -> Result<Value, V3Error> {
        validate_id("node_id", node_id)?;
        Ok(self
            .load_bundle_for_node(task_id, Some(node_id))?
            .node_brief)
    }

    pub fn workspace_root(&self, task_id: &str) -> Result<PathBuf, V3Error> {
        let events = self.store.load_project(&self.project_id())?;
        Ok(resolve_workspace(&self.root, task_id, &events).root)
    }

    pub fn load_project_structure_page(
        &self,
        task_id: &str,
        relative_dir: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<Value, V3Error> {
        let events = self.store.load_project(&self.project_id())?;
        let workspace = resolve_workspace(&self.root, task_id, &events);
        let service = ProjectIndexService::open(&workspace.root).map_err(|error| {
            V3Error::new(
                "V3_PROJECT_INDEX_QUERY_FAILED",
                V3ErrorCategory::Validation,
                true,
                error.to_string(),
            )
        })?;
        let snapshot = service.current_snapshot().map_err(|error| {
            V3Error::new(
                "V3_PROJECT_INDEX_QUERY_FAILED",
                V3ErrorCategory::Validation,
                true,
                error.to_string(),
            )
        })?;
        let page = service.page(relative_dir, cursor, limit).map_err(|error| {
            V3Error::new(
                "V3_PROJECT_INDEX_QUERY_FAILED",
                V3ErrorCategory::Validation,
                true,
                error.to_string(),
            )
        })?;
        let generated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        Ok(project_structure_view(
            &workspace.root,
            &self.project_id(),
            &generated_at,
            &[],
            Some(page),
            Some(&snapshot),
            &workspace,
        ))
    }

    pub fn search_project_structure(
        &self,
        task_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Value, V3Error> {
        let events = self.store.load_project(&self.project_id())?;
        let workspace = resolve_workspace(&self.root, task_id, &events);
        let service = ProjectIndexService::open(&workspace.root).map_err(|error| {
            V3Error::new(
                "V3_PROJECT_INDEX_SEARCH_FAILED",
                V3ErrorCategory::Validation,
                true,
                error.to_string(),
            )
        })?;
        let snapshot = service.current_snapshot().map_err(|error| {
            V3Error::new(
                "V3_PROJECT_INDEX_SEARCH_FAILED",
                V3ErrorCategory::Validation,
                true,
                error.to_string(),
            )
        })?;
        let page = service.search(query, limit).map_err(|error| {
            V3Error::new(
                "V3_PROJECT_INDEX_SEARCH_FAILED",
                V3ErrorCategory::Validation,
                true,
                error.to_string(),
            )
        })?;
        let generated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        Ok(project_structure_view(
            &workspace.root,
            &self.project_id(),
            &generated_at,
            &[],
            Some(page),
            Some(&snapshot),
            &workspace,
        ))
    }

    fn read_task(&self, task_id: &str) -> Result<TaskDocument, V3Error> {
        let path = self
            .root
            .join(".vibehub/tasks")
            .join(task_id)
            .join("task.yaml");
        let content = fs::read_to_string(&path).map_err(internal("V3_TASK_NOT_FOUND"))?;
        let task = serde_yaml::from_str(&content).map_err(|error| {
            V3Error::new(
                "V3_TASK_INVALID",
                V3ErrorCategory::CorruptLog,
                false,
                format!("{}: {error}", path.display()),
            )
        })?;
        validate_task_identity(&task, task_id, &path)?;
        Ok(task)
    }

    fn read_tasks(&self) -> Result<TaskReadResult, V3Error> {
        let tasks_root = self.root.join(".vibehub/tasks");
        let mut task_paths = fs::read_dir(&tasks_root)
            .map_err(internal("V3_TASKS_READ_FAILED"))?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name() != "current")
            .filter_map(|entry| {
                entry
                    .file_type()
                    .ok()
                    .filter(|file_type| file_type.is_dir() && !file_type.is_symlink())
                    .map(|_| entry.path().join("task.yaml"))
            })
            .collect::<Vec<_>>();
        task_paths.sort();
        let mut tasks = Vec::new();
        let mut warnings = Vec::new();
        for path in task_paths {
            let expected_task_id = path
                .parent()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            match self.read_task_document(&path, expected_task_id) {
                Ok(task) => tasks.push(task),
                Err(error) => warnings.push(task_metadata_warning(&self.root, &path, &error)),
            }
        }
        Ok(TaskReadResult { tasks, warnings })
    }

    fn read_task_document(
        &self,
        path: &Path,
        expected_task_id: &str,
    ) -> Result<TaskDocument, V3Error> {
        let content = fs::read_to_string(path).map_err(internal("V3_TASK_READ_FAILED"))?;
        let task = serde_yaml::from_str(&content).map_err(|error| {
            V3Error::new(
                "V3_TASK_INVALID",
                V3ErrorCategory::CorruptLog,
                false,
                format!("{}: {error}", path.display()),
            )
        })?;
        validate_task_identity(&task, expected_task_id, path)?;
        Ok(task)
    }
}

fn current_task_fallback_allowed(error: &V3Error) -> bool {
    matches!(
        error.code.as_str(),
        "V3_CURRENT_TASK_INVALID"
            | "V3_CURRENT_TASK_READ_FAILED"
            | "V3_TASK_INVALID"
            | "V3_TASK_NOT_FOUND"
            | "V3_TASK_READ_FAILED"
    )
}

fn validate_task_identity(
    task: &TaskDocument,
    expected_task_id: &str,
    path: &Path,
) -> Result<(), V3Error> {
    if task.task_id != expected_task_id {
        return Err(V3Error::new(
            "V3_TASK_INVALID",
            V3ErrorCategory::CorruptLog,
            false,
            format!(
                "{}: task_id '{}' does not match task directory '{}'",
                path.display(),
                task.task_id,
                expected_task_id
            ),
        ));
    }
    Ok(())
}

fn task_metadata_warning(root: &Path, path: &Path, error: &V3Error) -> Value {
    let task_id = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .unwrap_or("unknown-task");
    let task_path = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    json!({
        "code": "V3_TASK_METADATA_INVALID",
        "severity": "warning",
        "message_key": "v3.warning.task_metadata_invalid",
        "details": {
            "task_id": task_id,
            "task_path": task_path,
            "reason_code": error.code,
            "reason": limited_metadata_error(&error.message),
            "recommended_action": "Review and preserve the task, then run `vibehub v3 <project> quarantine-task <task_id>`."
        },
        "evidence_refs": [{
            "evidence_id": format!("task-metadata:{task_id}"),
            "kind": "file",
            "grade": "hard_observed",
            "label_key": "v3.evidence.task_metadata",
            "locator": format!("file:{task_path}")
        }]
    })
}

fn limited_metadata_error(message: &str) -> String {
    const MAX_CHARS: usize = 512;
    if message.chars().count() <= MAX_CHARS {
        message.to_owned()
    } else {
        format!("{}…", message.chars().take(MAX_CHARS).collect::<String>())
    }
}

#[derive(Debug, Clone)]
struct WorkspaceSelection {
    root: PathBuf,
    source: &'static str,
    session_id: Option<String>,
    worktree_id: Option<String>,
    fallback_reason: Option<String>,
}

#[derive(Debug, Clone)]
struct ArchitectureModule {
    node_id: String,
    name: String,
    kind: &'static str,
    relative_path: String,
    analyzer: String,
    package_name: Option<String>,
    manifest_path: Option<String>,
    source_kind: &'static str,
    confidence: f64,
    file_count: usize,
    evidence_refs: Vec<Value>,
}

#[derive(Debug, Clone)]
struct ArchitectureRelation {
    edge_id: String,
    from_node_id: String,
    to_node_id: String,
    kind: &'static str,
    source_kind: &'static str,
    confidence: f64,
    evidence_refs: Vec<Value>,
}

#[derive(Debug)]
struct ArchitectureView {
    modules: Vec<ArchitectureModule>,
    relations: Vec<ArchitectureRelation>,
    unsupported_analyzers: Vec<String>,
    warnings: Vec<Value>,
    completeness: &'static str,
    model_version: String,
}

impl ArchitectureView {
    fn nodes(&self, root: &Path) -> Vec<Value> {
        self.modules
            .iter()
            .map(|module| {
                let module_path = if module.relative_path == "." {
                    root.to_path_buf()
                } else {
                    root.join(&module.relative_path)
                };
                json!({
                    "node_id": module.node_id,
                    "name": module.name,
                    "kind": module.kind,
                    "path": native_path(&module_path),
                    "file_count": module.file_count,
                    "source_kind": module.source_kind,
                    "confidence": module.confidence,
                    "generator_version": self.model_version,
                    "evidence_refs": module.evidence_refs
                })
            })
            .collect()
    }

    fn edges(&self) -> Vec<Value> {
        self.relations
            .iter()
            .map(|relation| {
                json!({
                    "edge_id": relation.edge_id,
                    "from_node_id": relation.from_node_id,
                    "to_node_id": relation.to_node_id,
                    "kind": relation.kind,
                    "source_kind": relation.source_kind,
                    "confidence": relation.confidence,
                    "generator_version": self.model_version,
                    "evidence_refs": relation.evidence_refs
                })
            })
            .collect()
    }

    fn module_id_for_path(&self, path: &str) -> Option<&str> {
        nearest_architecture_module(&self.modules, path, &file_analyzer(path))
            .map(|module| module.node_id.as_str())
    }
}

fn project_structure_view(
    root: &Path,
    project_id: &str,
    generated_at: &str,
    evidence_refs: &[Value],
    page_result: Option<ProjectPage>,
    index_snapshot: Option<&ProjectModelSnapshot>,
    workspace: &WorkspaceSelection,
) -> Value {
    let (Some(result), Some(index_snapshot)) = (page_result, index_snapshot) else {
        return json!({
            "schema_version": "1.0", "project_id": project_id, "generated_at": generated_at, "model_version": MODEL_VERSION,
            "freshness": "unavailable", "completeness": "unknown", "index_state": "error",
            "workspace": workspace_view(workspace),
            "nodes": [], "edges": [], "architecture_nodes": [], "architecture_edges": [], "unsupported_analyzers": ["cargo", "npm", "python", "imports"],
            "page": {"cursor": Value::Null, "next_cursor": Value::Null, "limit": 200, "returned": 0, "total_estimate": Value::Null, "truncated": false, "truncation_reason": "none", "model_version": MODEL_VERSION},
            "evidence_refs": evidence_refs, "warnings": [],
            "errors": [{"code": "PI_INDEX_UNAVAILABLE", "category": "internal", "recoverable": true, "message_key": "v3.error.project_index_unavailable", "details": {}, "evidence_refs": evidence_refs}]
        });
    };
    let model_version = index_snapshot.model_version.clone();
    let mut structure_evidence_refs = evidence_refs.to_vec();
    structure_evidence_refs.push(json!({
        "evidence_id": format!("project-model:{}", model_version),
        "kind": "file",
        "grade": "hard_observed",
        "label_key": "v3.evidence.project_model_index",
        "locator": format!("file:{}", super::project_intelligence::PROJECT_MODEL_INDEX_PATH),
        "captured_at": index_snapshot.generated_at
    }));
    let architecture = architecture_view(
        root,
        &index_snapshot.analyzer_findings,
        &model_version,
        &index_snapshot.generated_at,
        &structure_evidence_refs,
    );
    let nodes: Vec<Value> = result.snapshot.nodes.iter().map(|node| {
        let full_path = if node.relative_path == "." { root.to_path_buf() } else { root.join(&node.relative_path) };
        let module_id = if node.kind == NodeKind::Root {
            Some("architecture.workspace")
        } else {
            architecture.module_id_for_path(&node.relative_path)
        };
        json!({
            "node_id": node.node_id, "parent_id": node.parent_id, "name": node.name,
            "kind": match node.kind { NodeKind::Root => "root", NodeKind::Directory => "directory", NodeKind::File => "file" },
            "path": native_path(&full_path),
            "git_state": match node.git_state { GitState::Clean => "clean", GitState::Modified => "modified", GitState::Added => "added", GitState::Deleted => "deleted", GitState::Ignored => "ignored", GitState::Unknown => "unknown" },
            "module_id": module_id, "ide_target": if node.kind == NodeKind::File { Some(node.relative_path.clone()) } else { None }, "evidence_refs": structure_evidence_refs
        })
    }).collect();
    let edges: Vec<Value> = result.snapshot.nodes.iter().filter_map(|node| node.parent_id.as_ref().map(|parent| json!({
        "edge_id": format!("contains.{}.{}", parent, node.node_id), "from_node_id": parent, "to_node_id": node.node_id,
        "kind": "contains", "source_kind": "filesystem", "confidence": 1.0, "evidence_refs": structure_evidence_refs
    }))).collect();
    let mut index_warnings: Vec<Value> = result.snapshot.warnings.iter().map(|warning| json!({
        "code": warning.code, "severity": "warning", "message_key": "v3.warning.project_index_gap",
        "details": {"path": warning.path, "message": warning.message}, "evidence_refs": structure_evidence_refs
    })).collect();
    index_warnings.extend(architecture.warnings.iter().cloned());
    let returned = nodes.len().saturating_sub(1);
    let truncated = result.next_cursor.is_some();
    let architecture_nodes = architecture.nodes(root);
    let architecture_edges = architecture.edges();
    json!({
        "schema_version": "1.0", "project_id": project_id, "generated_at": index_snapshot.generated_at, "model_version": model_version,
        "freshness": "fresh", "completeness": architecture.completeness, "index_state": "ready",
        "workspace": workspace_view(workspace),
        "nodes": nodes, "edges": edges, "architecture_nodes": architecture_nodes, "architecture_edges": architecture_edges, "unsupported_analyzers": architecture.unsupported_analyzers,
        "page": {"cursor": result.cursor, "next_cursor": result.next_cursor, "limit": result.limit, "returned": returned, "total_estimate": result.total_estimate, "truncated": truncated, "truncation_reason": if truncated {"page_limit"} else {"none"}, "model_version": model_version},
        "evidence_refs": structure_evidence_refs, "warnings": index_warnings, "errors": []
    })
}

fn workspace_view(workspace: &WorkspaceSelection) -> Value {
    json!({
        "root": native_path(&workspace.root),
        "source": workspace.source,
        "session_id": workspace.session_id,
        "worktree_id": workspace.worktree_id,
        "fallback_reason": workspace.fallback_reason,
        "ignored_directories": WORKSPACE_IGNORED_DIRS
    })
}

fn resolve_workspace(
    project_root: &Path,
    task_id: &str,
    events: &[V3EventEnvelope],
) -> WorkspaceSelection {
    let active_sessions: std::collections::BTreeSet<String> = sessions_for_task(events, task_id)
        .into_iter()
        .filter(|(_, opened, closed)| *opened && !*closed)
        .map(|(session_id, _, _)| session_id)
        .collect();
    if active_sessions.is_empty() {
        return fallback_workspace(
            project_root,
            None,
            "no active session or worktree context was recorded",
        );
    }
    let candidate_sessions = |event: &&V3EventEnvelope| {
        event.task_id.0 == task_id
            && event
                .session_id
                .as_ref()
                .is_some_and(|session| active_sessions.contains(&session.0))
    };
    for event in events.iter().rev().filter(candidate_sessions) {
        if event.event_type.starts_with("worktree.") {
            if let Some(path) = event
                .payload
                .get("native_path")
                .and_then(Value::as_str)
                .and_then(accessible_directory)
            {
                return WorkspaceSelection {
                    root: path,
                    source: "session_worktree",
                    session_id: event.session_id.as_ref().map(|value| value.0.clone()),
                    worktree_id: event.worktree_id.as_ref().map(|value| value.0.clone()),
                    fallback_reason: None,
                };
            }
        }
    }
    for event in events.iter().rev().filter(candidate_sessions) {
        if event.event_type == "session.opened" {
            if let Some(path) = event
                .payload
                .get("working_directory")
                .and_then(Value::as_str)
                .and_then(accessible_directory)
            {
                return WorkspaceSelection {
                    root: path,
                    source: "session_working_directory",
                    session_id: event.session_id.as_ref().map(|value| value.0.clone()),
                    worktree_id: event.worktree_id.as_ref().map(|value| value.0.clone()),
                    fallback_reason: None,
                };
            }
        }
    }
    fallback_workspace(
        project_root,
        active_sessions.iter().next().cloned(),
        "active session has no accessible working directory or worktree path",
    )
}

fn fallback_workspace(
    project_root: &Path,
    session_id: Option<String>,
    reason: &str,
) -> WorkspaceSelection {
    let resolved = resolve_project_scopes(project_root, None).ok();
    let detected = resolved
        .as_ref()
        .filter(|scopes| scopes.source == ProjectScopeSource::DetectedGitRoot);
    WorkspaceSelection {
        root: detected
            .map(|scopes| scopes.execution_root.clone())
            .unwrap_or_else(|| project_root.to_path_buf()),
        source: if detected.is_some() {
            "detected_git_root"
        } else {
            "project_root_fallback"
        },
        session_id,
        worktree_id: None,
        fallback_reason: Some(reason.to_owned()),
    }
}

fn accessible_directory(value: &str) -> Option<PathBuf> {
    let path = PathBuf::from(value);
    path.is_dir().then(|| path.canonicalize().ok()).flatten()
}

fn architecture_view_evidence_refs(project_structure: &Value) -> Vec<Value> {
    let mut seen = BTreeSet::new();
    let mut output = Vec::new();
    let mut add = |reference: &Value| {
        if output.len() >= 32 {
            return;
        }
        if reference
            .get("evidence_id")
            .and_then(Value::as_str)
            .is_some_and(|id| seen.insert(id.to_owned()))
        {
            output.push(reference.clone());
        }
    };
    for reference in project_structure["evidence_refs"]
        .as_array()
        .into_iter()
        .flatten()
    {
        add(reference);
    }
    for claim in project_structure["architecture_nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .chain(
            project_structure["architecture_edges"]
                .as_array()
                .into_iter()
                .flatten(),
        )
    {
        for reference in claim["evidence_refs"].as_array().into_iter().flatten() {
            add(reference);
        }
    }
    output
}

fn architecture_view(
    root: &Path,
    findings: &[AnalyzerFinding],
    model_version: &str,
    generated_at: &str,
    fallback_evidence_refs: &[Value],
) -> ArchitectureView {
    let workspace_evidence = findings
        .iter()
        .filter(|finding| finding.kind == "workspace_manifest")
        .map(|finding| architecture_evidence(finding, generated_at))
        .collect::<Vec<_>>();
    let mut modules = vec![ArchitectureModule {
        node_id: "architecture.workspace".to_owned(),
        name: root
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Workspace")
            .to_owned(),
        kind: "workspace",
        relative_path: ".".to_owned(),
        analyzer: "workspace".to_owned(),
        package_name: None,
        manifest_path: None,
        source_kind: "manifest",
        confidence: 1.0,
        file_count: 0,
        evidence_refs: if workspace_evidence.is_empty() {
            fallback_evidence_refs.to_vec()
        } else {
            workspace_evidence
        },
    }];

    for finding in findings
        .iter()
        .filter(|finding| matches!(finding.kind.as_str(), "package_manifest" | "manifest"))
    {
        let relative_path = manifest_parent(&finding.path);
        let identity = format!("{}:{}", finding.analyzer, finding.path);
        modules.push(ArchitectureModule {
            node_id: architecture_stable_id("package", &identity),
            name: finding.label.clone(),
            kind: "package",
            relative_path,
            analyzer: finding.analyzer.clone(),
            package_name: Some(
                finding
                    .target
                    .clone()
                    .unwrap_or_else(|| finding.label.clone()),
            ),
            manifest_path: Some(finding.path.clone()),
            source_kind: "manifest",
            confidence: 1.0,
            file_count: 0,
            evidence_refs: vec![architecture_evidence(finding, generated_at)],
        });
    }

    disambiguate_package_names(&mut modules);
    add_source_modules(findings, generated_at, &mut modules);
    add_documentation_modules(findings, generated_at, &mut modules);
    if modules.len() == 1 {
        add_inferred_modules(root, fallback_evidence_refs, &mut modules);
    }
    populate_architecture_file_counts(root, &mut modules);
    modules.sort_by(|left, right| {
        (
            left.kind != "workspace",
            &left.relative_path,
            left.kind,
            &left.name,
        )
            .cmp(&(
                right.kind != "workspace",
                &right.relative_path,
                right.kind,
                &right.name,
            ))
    });

    let mut relations = BTreeMap::<(String, String, &'static str), ArchitectureRelation>::new();
    for module in modules.iter().filter(|module| module.kind == "package") {
        let evidence = findings
            .iter()
            .find(|finding| {
                finding.kind == "workspace_member"
                    && finding
                        .target_path
                        .as_deref()
                        .is_some_and(|target| same_relative_path(target, &module.relative_path))
            })
            .map(|finding| vec![architecture_evidence(finding, generated_at)])
            .unwrap_or_else(|| module.evidence_refs.clone());
        insert_architecture_relation(
            &mut relations,
            "architecture.workspace",
            &module.node_id,
            "contains",
            "manifest",
            1.0,
            evidence,
        );
    }
    for module in modules.iter().filter(|module| module.kind == "module") {
        let parent = nearest_package(&modules, &module.relative_path, &module.analyzer)
            .map(|parent| parent.node_id.as_str())
            .unwrap_or("architecture.workspace");
        insert_architecture_relation(
            &mut relations,
            parent,
            &module.node_id,
            "contains",
            module.source_kind,
            module.confidence,
            module.evidence_refs.clone(),
        );
    }

    for finding in findings
        .iter()
        .filter(|finding| finding.kind == "dependency")
    {
        let Some(source) = package_for_manifest(&modules, &finding.path) else {
            continue;
        };
        let target = finding
            .target_path
            .as_deref()
            .and_then(|path| package_for_path(&modules, path, &finding.analyzer))
            .or_else(|| {
                finding
                    .target
                    .as_deref()
                    .and_then(|name| package_for_name(&modules, name, &finding.analyzer))
            });
        if let Some(target) = target {
            insert_architecture_relation(
                &mut relations,
                &source.node_id,
                &target.node_id,
                "depends_on",
                "manifest",
                1.0,
                vec![architecture_evidence(finding, generated_at)],
            );
        }
    }

    for finding in findings
        .iter()
        .filter(|finding| finding.kind == "static_import")
    {
        let analyzer = analyzer_family(&finding.analyzer);
        let Some(source) = nearest_architecture_module(&modules, &finding.path, analyzer) else {
            continue;
        };
        let target_path = finding
            .target_path
            .clone()
            .or_else(|| resolve_import_path(&finding.path, finding.target.as_deref()));
        let target = target_path
            .as_deref()
            .and_then(|path| nearest_architecture_module(&modules, path, analyzer))
            .or_else(|| {
                finding.target.as_deref().and_then(|name| {
                    package_for_name(&modules, import_package_name(name), analyzer)
                })
            });
        if let Some(target) = target {
            insert_architecture_relation(
                &mut relations,
                &source.node_id,
                &target.node_id,
                "imports",
                "parser",
                0.9,
                vec![architecture_evidence(finding, generated_at)],
            );
        }
    }

    let mut warnings = findings
        .iter()
        .filter(|finding| finding.kind == "analyzer_error")
        .map(|finding| {
            json!({
                "code": "PI_ANALYZER_DEGRADED",
                "severity": "warning",
                "message_key": "v3.warning.analyzer_degraded",
                "details": {"analyzer": finding.analyzer, "path": finding.path, "message": finding.label},
                "evidence_refs": [architecture_evidence(finding, generated_at)]
            })
        })
        .collect::<Vec<_>>();
    let unsupported_analyzers = findings
        .iter()
        .filter(|finding| {
            finding.kind == "capability"
                && finding.label.to_ascii_lowercase().contains("unsupported")
        })
        .map(|finding| finding.analyzer.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if modules.len() == 1 {
        warnings.push(json!({
            "code": "PI_ARCHITECTURE_UNSUPPORTED",
            "severity": "warning",
            "message_key": "v3.warning.architecture_unsupported",
            "details": {"message": "No supported manifest, source module, or declared architecture document was observed"},
            "evidence_refs": fallback_evidence_refs
        }));
    }
    if !unsupported_analyzers.is_empty() {
        warnings.push(json!({
            "code": "PI_ANALYZER_UNSUPPORTED",
            "severity": "warning",
            "message_key": "v3.warning.analyzer_unsupported",
            "details": {"analyzers": unsupported_analyzers},
            "evidence_refs": fallback_evidence_refs
        }));
    }
    let completeness = if modules.len() == 1 {
        "unsupported"
    } else if warnings.is_empty() {
        "complete"
    } else {
        "partial"
    };
    ArchitectureView {
        modules,
        relations: relations.into_values().collect(),
        unsupported_analyzers,
        warnings,
        completeness,
        model_version: model_version.to_owned(),
    }
}

fn add_source_modules(
    findings: &[AnalyzerFinding],
    generated_at: &str,
    modules: &mut Vec<ArchitectureModule>,
) {
    let mut grouped = BTreeMap::<(String, String), Vec<&AnalyzerFinding>>::new();
    for finding in findings
        .iter()
        .filter(|finding| finding.kind == "static_import")
    {
        let analyzer = analyzer_family(&finding.analyzer).to_owned();
        let path = manifest_parent(&finding.path);
        if modules.iter().any(|module| {
            module.kind == "package"
                && module.analyzer == analyzer
                && same_relative_path(&module.relative_path, &path)
        }) {
            continue;
        }
        grouped.entry((analyzer, path)).or_default().push(finding);
    }
    for ((analyzer, path), evidence) in grouped {
        modules.push(ArchitectureModule {
            node_id: architecture_stable_id("module", &format!("{analyzer}:{path}")),
            name: path.clone(),
            kind: "module",
            relative_path: path,
            analyzer,
            package_name: None,
            manifest_path: None,
            source_kind: "parser",
            confidence: 0.9,
            file_count: 0,
            evidence_refs: unique_architecture_evidence(evidence, generated_at, 16),
        });
    }
}

fn add_documentation_modules(
    findings: &[AnalyzerFinding],
    generated_at: &str,
    modules: &mut Vec<ArchitectureModule>,
) {
    let mut grouped = BTreeMap::<String, Vec<&AnalyzerFinding>>::new();
    for finding in findings
        .iter()
        .filter(|finding| matches!(finding.kind.as_str(), "readme" | "architecture_document"))
    {
        grouped
            .entry(manifest_parent(&finding.path))
            .or_default()
            .push(finding);
    }
    for (path, evidence) in grouped {
        modules.push(ArchitectureModule {
            node_id: architecture_stable_id("documentation", &path),
            name: if path == "." {
                "Declared architecture".to_owned()
            } else {
                path.clone()
            },
            kind: "module",
            relative_path: path,
            analyzer: "documentation".to_owned(),
            package_name: None,
            manifest_path: None,
            source_kind: "documentation",
            confidence: 1.0,
            file_count: evidence.len(),
            evidence_refs: unique_architecture_evidence(evidence, generated_at, 16),
        });
    }
}

fn add_inferred_modules(
    root: &Path,
    evidence_refs: &[Value],
    modules: &mut Vec<ArchitectureModule>,
) {
    for candidate in ["src", "src-tauri", "crates", "packages", "apps"] {
        if root.join(candidate).is_dir() {
            modules.push(ArchitectureModule {
                node_id: architecture_stable_id("module", candidate),
                name: candidate.to_owned(),
                kind: "module",
                relative_path: candidate.to_owned(),
                analyzer: "inference".to_owned(),
                package_name: None,
                manifest_path: None,
                source_kind: "inference",
                confidence: 0.6,
                file_count: 0,
                evidence_refs: evidence_refs.to_vec(),
            });
        }
    }
}

fn insert_architecture_relation(
    relations: &mut BTreeMap<(String, String, &'static str), ArchitectureRelation>,
    from: &str,
    to: &str,
    kind: &'static str,
    source_kind: &'static str,
    confidence: f64,
    evidence_refs: Vec<Value>,
) {
    if from == to {
        return;
    }
    let key = (from.to_owned(), to.to_owned(), kind);
    if let Some(existing) = relations.get_mut(&key) {
        merge_evidence(&mut existing.evidence_refs, evidence_refs, 32);
        existing.confidence = existing.confidence.max(confidence);
        return;
    }
    relations.insert(
        key,
        ArchitectureRelation {
            edge_id: architecture_stable_id("edge", &format!("{kind}:{from}:{to}")),
            from_node_id: from.to_owned(),
            to_node_id: to.to_owned(),
            kind,
            source_kind,
            confidence,
            evidence_refs,
        },
    );
}

fn architecture_stable_id(kind: &str, identity: &str) -> String {
    let digest = format!("{:x}", Sha256::digest(identity.as_bytes()));
    format!("architecture.{kind}.{}", &digest[..16])
}

fn architecture_evidence(finding: &AnalyzerFinding, generated_at: &str) -> Value {
    let identity = format!(
        "{}:{}:{}:{}",
        finding.analyzer, finding.kind, finding.path, finding.label
    );
    json!({
        "evidence_id": architecture_stable_id("evidence", &identity),
        "kind": "file",
        "grade": "hard_observed",
        "label_key": format!("v3.evidence.architecture.{}", finding.kind),
        "locator": format!("file:{}", finding.path),
        "captured_at": generated_at,
        "excerpt": finding.label
    })
}

fn unique_architecture_evidence(
    findings: Vec<&AnalyzerFinding>,
    generated_at: &str,
    limit: usize,
) -> Vec<Value> {
    let mut seen = BTreeSet::new();
    findings
        .into_iter()
        .filter(|finding| seen.insert((finding.path.clone(), finding.label.clone())))
        .take(limit)
        .map(|finding| architecture_evidence(finding, generated_at))
        .collect()
}

fn merge_evidence(target: &mut Vec<Value>, incoming: Vec<Value>, limit: usize) {
    let mut seen = target
        .iter()
        .filter_map(|reference| reference.get("evidence_id").and_then(Value::as_str))
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    for reference in incoming {
        if target.len() >= limit {
            break;
        }
        if reference
            .get("evidence_id")
            .and_then(Value::as_str)
            .is_some_and(|id| seen.insert(id.to_owned()))
        {
            target.push(reference);
        }
    }
}

fn disambiguate_package_names(modules: &mut [ArchitectureModule]) {
    let counts = modules
        .iter()
        .filter_map(|module| module.package_name.as_deref())
        .fold(BTreeMap::<String, usize>::new(), |mut counts, name| {
            *counts.entry(name.to_owned()).or_default() += 1;
            counts
        });
    for module in modules.iter_mut().filter(|module| module.kind == "package") {
        let Some(package_name) = module.package_name.as_deref() else {
            continue;
        };
        if counts.get(package_name).copied().unwrap_or_default() > 1 {
            let location = if module.relative_path == "." {
                "root"
            } else {
                module.relative_path.as_str()
            };
            module.name = format!("{package_name} ({} · {location})", module.analyzer);
        }
    }
}

fn populate_architecture_file_counts(root: &Path, modules: &mut [ArchitectureModule]) {
    let files = visible_project_files(root);
    let package_snapshot = modules.to_vec();
    for module in modules {
        module.file_count = if module.kind == "workspace" {
            files.len()
        } else if module.kind == "package" {
            files
                .iter()
                .filter(|file| {
                    package_for_path(&package_snapshot, file, &file_analyzer(file))
                        .is_some_and(|owner| owner.node_id == module.node_id)
                })
                .count()
        } else {
            files
                .iter()
                .filter(|file| {
                    path_is_within(file, &module.relative_path)
                        && (module.analyzer == "inference"
                            || analyzer_matches(&module.analyzer, &file_analyzer(file)))
                })
                .count()
        };
    }
}

fn visible_project_files(root: &Path) -> Vec<String> {
    let output = silent_command("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--cached", "--others", "--exclude-standard"])
        .output();
    match output {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|path| !path.is_empty())
            .map(|path| path.replace('\\', "/"))
            .filter(|path| {
                !Path::new(path).components().any(|component| {
                    matches!(component, std::path::Component::Normal(value) if WORKSPACE_IGNORED_DIRS.contains(&value.to_string_lossy().as_ref()))
                })
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn nearest_architecture_module<'a>(
    modules: &'a [ArchitectureModule],
    path: &str,
    analyzer: &str,
) -> Option<&'a ArchitectureModule> {
    modules
        .iter()
        .filter(|module| {
            module.kind != "workspace"
                && path_is_within(path, &module.relative_path)
                && analyzer_matches(&module.analyzer, analyzer)
        })
        .max_by_key(|module| {
            (
                module.relative_path.matches('/').count(),
                usize::from(module.kind == "module"),
            )
        })
        .or_else(|| package_for_path(modules, path, analyzer))
}

fn nearest_package<'a>(
    modules: &'a [ArchitectureModule],
    path: &str,
    analyzer: &str,
) -> Option<&'a ArchitectureModule> {
    package_for_path(modules, path, analyzer)
}

fn package_for_path<'a>(
    modules: &'a [ArchitectureModule],
    path: &str,
    analyzer: &str,
) -> Option<&'a ArchitectureModule> {
    modules
        .iter()
        .filter(|module| {
            module.kind == "package"
                && path_is_within(path, &module.relative_path)
                && analyzer_matches(&module.analyzer, analyzer)
        })
        .max_by_key(|module| module.relative_path.matches('/').count())
}

fn package_for_manifest<'a>(
    modules: &'a [ArchitectureModule],
    manifest: &str,
) -> Option<&'a ArchitectureModule> {
    modules
        .iter()
        .find(|module| module.manifest_path.as_deref() == Some(manifest))
}

fn package_for_name<'a>(
    modules: &'a [ArchitectureModule],
    name: &str,
    analyzer: &str,
) -> Option<&'a ArchitectureModule> {
    let name = normalized_package_name(name);
    modules.iter().find(|module| {
        module.kind == "package"
            && analyzer_matches(&module.analyzer, analyzer)
            && module
                .package_name
                .as_deref()
                .is_some_and(|candidate| normalized_package_name(candidate) == name)
    })
}

fn analyzer_family(analyzer: &str) -> &str {
    match analyzer {
        "rust_imports" => "cargo",
        "javascript_imports" => "npm",
        "python_imports" => "python",
        other => other,
    }
}

fn analyzer_matches(left: &str, right: &str) -> bool {
    analyzer_family(left) == analyzer_family(right) || left == "inference" || right == "inference"
}

fn file_analyzer(path: &str) -> String {
    let name = Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if name == "Cargo.toml" {
        return "cargo".to_owned();
    }
    if matches!(name, "package.json" | "tsconfig.json" | "vite.config.ts") {
        return "npm".to_owned();
    }
    match Path::new(path).extension().and_then(|value| value.to_str()) {
        Some("rs") => "cargo",
        Some("ts" | "tsx" | "js" | "jsx" | "json" | "css" | "html") => "npm",
        Some("py") => "python",
        Some("md" | "mdx") => "documentation",
        _ => "inference",
    }
    .to_owned()
}

fn manifest_parent(path: &str) -> String {
    Path::new(path)
        .parent()
        .and_then(Path::to_str)
        .filter(|path| !path.is_empty())
        .unwrap_or(".")
        .replace('\\', "/")
}

fn path_is_within(path: &str, root: &str) -> bool {
    root == "." || path == root || path.starts_with(&format!("{root}/"))
}

fn same_relative_path(left: &str, right: &str) -> bool {
    left.trim_end_matches('/').trim_start_matches("./")
        == right.trim_end_matches('/').trim_start_matches("./")
}

fn normalized_package_name(name: &str) -> String {
    name.trim()
        .trim_matches(['\'', '"'])
        .replace('-', "_")
        .to_ascii_lowercase()
}

fn import_package_name(target: &str) -> &str {
    if target.starts_with('@') {
        target
            .splitn(3, '/')
            .take(2)
            .last()
            .and_then(|name| target.find(name).map(|index| &target[..index + name.len()]))
            .unwrap_or(target)
    } else {
        target.split(['/', ':', '.']).next().unwrap_or(target)
    }
}

fn resolve_import_path(source: &str, target: Option<&str>) -> Option<String> {
    let target = target?;
    let candidate = if let Some(target) = target.strip_prefix("@/") {
        PathBuf::from("src").join(target)
    } else if target.starts_with('.') {
        Path::new(source)
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(target)
    } else {
        return None;
    };
    normalize_relative_path(&candidate)
}

fn normalize_relative_path(path: &Path) -> Option<String> {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::Normal(value) => {
                components.push(value.to_string_lossy().into_owned())
            }
            std::path::Component::ParentDir => {
                components.pop()?;
            }
            _ => return None,
        }
    }
    Some(if components.is_empty() {
        ".".to_owned()
    } else {
        components.join("/")
    })
}

fn normalize_agent_evidence_ref(value: &Value, event: &V3EventEnvelope) -> Option<Value> {
    if let Some(locator) = value.as_str() {
        let kind = if locator.starts_with("file:") {
            "file"
        } else if locator.starts_with("evt.") || locator.starts_with("vibehub://v3/events/") {
            "event"
        } else {
            "external"
        };
        return Some(json!({
            "evidence_id": locator,
            "kind": kind,
            "grade": event.evidence_grade,
            "label_key": "v3.evidence.agent_result",
            "locator": locator,
            "captured_at": event.recorded_at
        }));
    }
    let object = value.as_object()?;
    let locator = object.get("locator").and_then(Value::as_str)?;
    let evidence_id = object
        .get("evidence_id")
        .and_then(Value::as_str)
        .unwrap_or(locator);
    let kind = object
        .get("kind")
        .and_then(Value::as_str)
        .filter(|kind| {
            [
                "event", "file", "git", "command", "test", "user", "external",
            ]
            .contains(kind)
        })
        .unwrap_or("external");
    let event_grade = match event.evidence_grade {
        EvidenceGrade::HardObserved => "hard_observed",
        EvidenceGrade::AgentReported => "agent_reported",
        EvidenceGrade::Inferred => "inferred",
        EvidenceGrade::UserConfirmed => "user_confirmed",
    };
    let grade = object
        .get("grade")
        .and_then(Value::as_str)
        .filter(|grade| {
            [
                "hard_observed",
                "agent_reported",
                "inferred",
                "user_confirmed",
            ]
            .contains(grade)
        })
        .unwrap_or(event_grade);
    Some(json!({
        "evidence_id": evidence_id,
        "kind": kind,
        "grade": grade,
        "label_key": object.get("label_key").and_then(Value::as_str).unwrap_or("v3.evidence.agent_result"),
        "locator": locator,
        "captured_at": object.get("captured_at").cloned().unwrap_or_else(|| Value::String(event.recorded_at.clone())),
        "excerpt": object.get("excerpt").cloned().unwrap_or(Value::Null)
    }))
}

fn normalize_agent_evaluation(value: Option<&Value>) -> (Value, bool) {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return (Value::Null, false);
    };
    let Some(object) = value.as_object() else {
        return (Value::Null, true);
    };
    let target = object
        .get("target")
        .and_then(Value::as_str)
        .unwrap_or("Evaluation target unavailable");
    let rubric: Vec<Value> = object
        .get("rubric")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|item| json!(item))
                .collect()
        })
        .unwrap_or_default();
    let raw_verdict = object
        .get("verdict")
        .and_then(Value::as_str)
        .unwrap_or("inconclusive");
    let verdict = if ["passed", "needs_revision", "failed", "inconclusive"].contains(&raw_verdict) {
        raw_verdict
    } else {
        "inconclusive"
    };
    let findings: Vec<Value> = object
        .get("findings")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|finding| {
                    if let Some(detail) = finding.as_str() {
                        return Some(json!({"title": detail, "detail": detail, "severity": "medium", "evidence_refs": []}));
                    }
                    let finding = finding.as_object()?;
                    let title = finding.get("title").and_then(Value::as_str).unwrap_or("Untitled finding");
                    let detail = finding.get("detail").and_then(Value::as_str).unwrap_or("Finding details unavailable");
                    let raw_severity = finding.get("severity").and_then(Value::as_str).unwrap_or("medium");
                    let severity = if ["info", "low", "medium", "high", "critical"].contains(&raw_severity) { raw_severity } else { "medium" };
                    let evidence_refs = finding.get("evidence_refs").and_then(Value::as_array).cloned().unwrap_or_default();
                    Some(json!({"title": title, "detail": detail, "severity": severity, "evidence_refs": evidence_refs}))
                })
                .collect()
        })
        .unwrap_or_default();
    let normalized =
        json!({"target": target, "rubric": rubric, "verdict": verdict, "findings": findings});
    (normalized.clone(), &normalized != value)
}

fn normalize_agent_artifacts(value: Option<&Value>) -> (Vec<Value>, bool) {
    let Some(items) = value.and_then(Value::as_array) else {
        return (Vec::new(), value.is_some_and(|value| !value.is_null()));
    };
    let normalized: Vec<Value> = items
        .iter()
        .filter_map(|item| {
            if let Some(label) = item.as_str() {
                return Some(json!({"label": label, "path": null, "uri": null}));
            }
            let object = item.as_object()?;
            let label = object
                .get("label")
                .or_else(|| object.get("kind"))
                .or_else(|| object.get("locator"))
                .and_then(Value::as_str)
                .unwrap_or("Unnamed artifact");
            let path = object
                .get("path")
                .filter(|path| {
                    path.get("platform").and_then(Value::as_str).is_some()
                        && path.get("native").and_then(Value::as_str).is_some()
                        && path.get("display").and_then(Value::as_str).is_some()
                        && path.get("identity_key").and_then(Value::as_str).is_some()
                })
                .cloned()
                .unwrap_or(Value::Null);
            let uri = object
                .get("uri")
                .cloned()
                .or_else(|| object.get("locator").cloned())
                .filter(|uri| uri.is_string())
                .unwrap_or(Value::Null);
            Some(json!({"label": label, "path": path, "uri": uri}))
        })
        .collect();
    let changed = normalized != *items;
    (normalized, changed)
}

fn agent_results_view(
    project_id: &str,
    task_id: &str,
    generated_at: &str,
    events: &[V3EventEnvelope],
    evidence_refs: &[Value],
) -> Value {
    let session_events: Vec<&V3EventEnvelope> = events
        .iter()
        .filter(|event| event.task_id.0 == task_id && event.event_type == "session.opened")
        .collect();
    let closed_session_ids: std::collections::BTreeSet<&str> = events
        .iter()
        .filter(|event| event.task_id.0 == task_id && event.event_type == "session.closed")
        .filter_map(|event| event.session_id.as_ref().map(|value| value.0.as_str()))
        .collect();
    let result_events: Vec<&V3EventEnvelope> = events
        .iter()
        .filter(|event| event.task_id.0 == task_id && event.event_type == "agent.result_recorded")
        .collect();
    let mut results = Vec::new();
    let mut result_positions = std::collections::BTreeMap::new();
    let mut warnings = Vec::new();
    for event in result_events {
        let payload = &event.payload;
        let raw_result_evidence = payload.get("evidence_refs").and_then(Value::as_array);
        let result_evidence: Vec<Value> = raw_result_evidence.map(|items| items.iter().filter_map(|item| normalize_agent_evidence_ref(item, event)).collect()).filter(|items: &Vec<Value>| !items.is_empty()).unwrap_or_else(|| vec![json!({
            "evidence_id": event.event_id, "kind": "event", "grade": event.evidence_grade, "label_key": "v3.evidence.agent_result", "locator": format!("vibehub://v3/events/{}", event.event_id), "captured_at": event.recorded_at
        })]);
        let result_id = payload
            .get("result_id")
            .and_then(Value::as_str)
            .unwrap_or(&event.event_id);
        let raw_status = payload
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("failed");
        let session_id = event
            .session_id
            .as_ref()
            .map(|value| value.0.as_str())
            .unwrap_or("session.unknown");
        let status = match raw_status {
            "running" if closed_session_ids.contains(session_id) => {
                warnings.push(json!({
                    "code": "V3_AGENT_RESULT_SESSION_CLOSED",
                    "severity": "warning",
                    "message_key": "v3.warning.agent_result_session_closed",
                    "details": {"result_id": result_id, "projected_status": "pending"},
                    "evidence_refs": result_evidence
                }));
                "pending"
            }
            "pending" | "running" | "succeeded" | "failed" => raw_status,
            invalid => {
                warnings.push(json!({
                    "code": "V3_AGENT_RESULT_STATUS_INVALID",
                    "severity": "warning",
                    "message_key": "v3.warning.agent_result_status_invalid",
                    "details": {"result_id": result_id, "original_status": invalid, "projected_status": "failed"},
                    "evidence_refs": result_evidence
                }));
                "failed"
            }
        };
        let (evaluation, evaluation_normalized) =
            normalize_agent_evaluation(payload.get("evaluation"));
        let (artifacts, artifacts_normalized) = normalize_agent_artifacts(payload.get("artifacts"));
        if evaluation_normalized
            || artifacts_normalized
            || raw_result_evidence.is_some_and(|items| items.iter().any(|item| !item.is_object()))
        {
            warnings.push(json!({
                "code": "V3_AGENT_RESULT_DETAILS_NORMALIZED",
                "severity": "warning",
                "message_key": "v3.warning.agent_result_details_normalized",
                "details": {"result_id": result_id},
                "evidence_refs": result_evidence
            }));
        }
        let result = json!({
            "result_id": result_id,
            "kind": payload.get("kind").and_then(Value::as_str).unwrap_or("execution"),
            "session_id": session_id,
            "node_id": event.node_id.as_ref().map(|value| value.0.clone()),
            "request": {"source": payload.get("request_source").and_then(Value::as_str).unwrap_or("user_request"), "instruction": payload.get("instruction").and_then(Value::as_str).unwrap_or("Result instruction unavailable")},
            "status": status,
            "summary": payload.get("summary").and_then(Value::as_str).unwrap_or(""),
            "body": payload.get("body").cloned().unwrap_or(Value::Null),
            "evaluation": evaluation,
            "artifacts": artifacts,
            "started_at": payload.get("started_at").cloned().unwrap_or(Value::Null),
            "completed_at": payload.get("completed_at").cloned().unwrap_or(Value::Null),
            "evidence_refs": result_evidence
        });
        if let Some(position) = result_positions.get(result_id).copied() {
            results[position] = result;
        } else {
            result_positions.insert(result_id.to_owned(), results.len());
            results.push(result);
        }
    }
    let state = if results.iter().any(|result| result["status"] == "failed") {
        "failed"
    } else if results
        .iter()
        .any(|result| matches!(result["status"].as_str(), Some("pending" | "running")))
    {
        "awaiting_result"
    } else if !results.is_empty() {
        "available"
    } else if session_events.is_empty() {
        "not_executed"
    } else {
        "awaiting_result"
    };
    let review_required = results.iter().any(|result| result["status"] == "succeeded")
        && !events.iter().any(|event| {
            event.task_id.0 == task_id
                && matches!(
                    event.event_type.as_str(),
                    "criterion.passed"
                        | "criterion.failed"
                        | "criterion.blocked"
                        | "criterion.not_applicable"
                )
        });
    let state = if review_required {
        "review_required"
    } else {
        state
    };
    json!({
        "schema_version": "1.0", "project_id": project_id, "task_id": task_id, "generated_at": generated_at, "model_version": MODEL_VERSION,
        "freshness": "fresh", "completeness": if results.is_empty() {"unknown"} else {"complete"}, "state": state, "review_required": review_required, "next_action": if review_required { Value::String("run_review".to_owned()) } else { Value::Null }, "results": results,
        "evidence_refs": evidence_refs, "warnings": warnings, "errors": []
    })
}

fn sort_archived_tasks_newest_first(archived_tasks: &mut [Value]) {
    archived_tasks.sort_by(|left, right| {
        let left_terminal_at = left.get("terminal_at").and_then(Value::as_str);
        let right_terminal_at = right.get("terminal_at").and_then(Value::as_str);
        match (left_terminal_at, right_terminal_at) {
            (Some(left_at), Some(right_at)) => right_at.cmp(left_at),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
        .then_with(|| {
            let left_task_id = left
                .get("task_id")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let right_task_id = right
                .get("task_id")
                .and_then(Value::as_str)
                .unwrap_or_default();
            left_task_id.cmp(right_task_id)
        })
    });
}

fn task_list_summary(
    task: &TaskDocument,
    events: &[V3EventEnvelope],
    lifecycle: &TaskLifecycleProjection,
    state: &str,
    generated_at: &str,
) -> Value {
    let criteria = criteria(task, events, lifecycle, generated_at);
    let criterion_pass_rate = criterion_pass_rate(&criteria);
    let required_node_completion_rate = required_node_completion_rate(lifecycle);
    let active_session_count = sessions_for_task(events, &task.task_id)
        .into_iter()
        .filter(|(_, open, closed)| *open && !*closed)
        .count();
    let terminal_at = task_terminal_at(&task.task_id, events);
    json!({
        "task_id": task.task_id,
        "title": task.title,
        "intent": task.intent,
        "workflow_profile": task.workflow_profile,
        "state": state,
        "is_terminal": is_terminal_task_state(state),
        "terminal_at": terminal_at,
        "criterion_pass_rate": criterion_pass_rate,
        "required_node_completion_rate": required_node_completion_rate,
        "active_session_count": active_session_count,
        "source": "v3_event_log_projection"
    })
}

fn criterion_pass_rate(criteria: &[Value]) -> Value {
    let required = criteria
        .iter()
        .filter(|criterion| criterion["required"].as_bool().unwrap_or(true))
        .collect::<Vec<_>>();
    let passed = required
        .iter()
        .filter(|criterion| criterion["status"] == "passed")
        .count();
    let not_applicable = required
        .iter()
        .filter(|criterion| criterion["status"] == "not_applicable")
        .count();
    let effective_passed = passed + not_applicable;
    let total = required.len();
    json!({
        "passed": passed,
        "not_applicable": not_applicable,
        "effective_passed": effective_passed,
        "total": total,
        "ratio": if total == 0 { Value::Null } else { json!(effective_passed as f64 / total as f64) }
    })
}

fn required_node_completion_rate(lifecycle: &TaskLifecycleProjection) -> Value {
    let necessary = lifecycle
        .effective_nodes()
        .filter(|(_, node)| !node.is_historical_bootstrap())
        .map(|(_, node)| node)
        .collect::<Vec<_>>();
    let total = necessary.len();
    let completed = necessary
        .iter()
        .filter(|node| node.state == "completed")
        .count();
    let waived = necessary
        .iter()
        .filter(|node| node.state == "waived")
        .count();
    let credited = completed + waived;
    json!({
        "completed": completed,
        "waived": waived,
        "credited": credited,
        "total": total,
        "ratio": if total == 0 { Value::Null } else { json!(credited as f64 / total as f64) }
    })
}

fn task_terminal_at(task_id: &str, events: &[V3EventEnvelope]) -> Option<String> {
    events
        .iter()
        .filter(|event| {
            event.task_id.0 == task_id
                && matches!(
                    event.event_type.as_str(),
                    "task.completion_confirmed" | "task.closed_with_exceptions"
                )
        })
        .max_by(|left, right| {
            left.occurred_at
                .cmp(&right.occurred_at)
                .then_with(|| left.event_id.cmp(&right.event_id))
        })
        .map(|event| event.occurred_at.clone())
}

fn sort_task_list(tasks: &mut [Value]) {
    tasks.sort_by(|left, right| {
        let left_terminal = left["is_terminal"].as_bool().unwrap_or(false);
        let right_terminal = right["is_terminal"].as_bool().unwrap_or(false);
        match (left_terminal, right_terminal) {
            (false, true) => Ordering::Less,
            (true, false) => Ordering::Greater,
            (true, true) => compare_terminal_at(left, right),
            (false, false) => task_state_rank(left["state"].as_str().unwrap_or_default()).cmp(
                &task_state_rank(right["state"].as_str().unwrap_or_default()),
            ),
        }
        .then_with(|| {
            left["task_id"]
                .as_str()
                .unwrap_or_default()
                .cmp(right["task_id"].as_str().unwrap_or_default())
        })
    });
}

fn compare_terminal_at(left: &Value, right: &Value) -> Ordering {
    let left_at = left["terminal_at"].as_str();
    let right_at = right["terminal_at"].as_str();
    match (left_at, right_at) {
        (Some(left_at), Some(right_at)) => right_at.cmp(left_at),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn task_state_rank(state: &str) -> u8 {
    match state {
        "active" => 0,
        "planned" => 1,
        "blocked" => 2,
        "review" => 3,
        _ => 4,
    }
}

fn session_git_traces(events: &[V3EventEnvelope], task_id: &str) -> Vec<SessionGitTrace> {
    let mut traces = BTreeMap::<String, SessionGitTrace>::new();
    for event in events.iter().filter(|event| {
        event.task_id.0 == task_id
            && matches!(
                event.event_type.as_str(),
                "session.opened" | "session.closed"
            )
    }) {
        let Some(session_id) = event.session_id.as_ref().map(|id| id.0.clone()) else {
            continue;
        };
        let trace = traces
            .entry(session_id.clone())
            .or_insert_with(|| SessionGitTrace {
                session_id,
                ..SessionGitTrace::default()
            });
        if let Some(commit_sha) = event.commit_sha.as_deref().and_then(normalize_stored_hash) {
            trace.event_commit_shas.insert(commit_sha);
        }
        match event.event_type.as_str() {
            "session.opened" => {
                trace.open_git_head = git_head_from_event(event);
                trace.opened_at = Some(event.occurred_at.clone());
            }
            "session.closed" => {
                trace.close_git_head = git_head_from_event(event);
                trace.closed_at = Some(event.occurred_at.clone());
            }
            _ => {}
        }
    }
    traces.into_values().collect()
}

fn git_head_from_event(event: &V3EventEnvelope) -> Option<String> {
    event
        .payload
        .get("git_head_sha")
        .and_then(Value::as_str)
        .and_then(normalize_stored_hash)
}

fn session_git_trace_value(trace: &SessionGitTrace) -> Value {
    let status = git_trace_status(
        trace.open_git_head.as_deref(),
        trace.close_git_head.as_deref(),
    );
    json!({
        "session_id": trace.session_id,
        "open_git_head": trace.open_git_head,
        "close_git_head": trace.close_git_head,
        "opened_at": trace.opened_at,
        "closed_at": trace.closed_at,
        "git_trace_status": status,
        "has_git_evidence": status != "missing" && status != "unavailable",
        "event_commit_shas": trace.event_commit_shas.iter().cloned().collect::<Vec<_>>()
    })
}

fn git_trace_status(open_git_head: Option<&str>, close_git_head: Option<&str>) -> &'static str {
    match (open_git_head, close_git_head) {
        (Some(_), Some(_)) => "complete",
        (Some(_), None) | (None, Some(_)) => "partial",
        (None, None) => "missing",
    }
}

fn collect_stored_commit_hashes(events: &[V3EventEnvelope]) -> BTreeSet<String> {
    let mut hashes = BTreeSet::new();
    for event in events {
        if let Some(hash) = event.commit_sha.as_deref().and_then(normalize_stored_hash) {
            hashes.insert(hash);
        }
        if let Some(hash) = git_head_from_event(event) {
            hashes.insert(hash);
        }
    }
    hashes
}

fn normalize_stored_hash(value: &str) -> Option<String> {
    let normalized = value.trim().to_ascii_lowercase();
    (normalized.len() >= 4
        && normalized.len() <= 64
        && normalized
            .chars()
            .all(|character| character.is_ascii_hexdigit()))
    .then_some(normalized)
}

fn normalize_commit_hash(value: &str) -> Result<String, V3Error> {
    normalize_stored_hash(value).ok_or_else(|| {
        V3Error::new(
            "V3_COMMIT_HASH_INVALID",
            V3ErrorCategory::Validation,
            false,
            "commit hash must be 4-64 hexadecimal characters",
        )
    })
}

fn resolve_commit_hash(
    root: &Path,
    query_hash: &str,
    stored_hashes: &BTreeSet<String>,
) -> Result<Option<String>, V3Error> {
    let matching_stored = stored_hashes
        .iter()
        .filter(|hash| hash.starts_with(query_hash))
        .cloned()
        .collect::<Vec<_>>();
    if matching_stored.len() > 1 {
        return Err(V3Error::new(
            "V3_COMMIT_HASH_AMBIGUOUS",
            V3ErrorCategory::Validation,
            false,
            "short commit hash matches multiple recorded Git commits",
        )
        .with_detail("query_commit_hash", query_hash.to_owned())
        .with_detail("matching_commits", matching_stored));
    }
    if let Some(hash) = matching_stored.into_iter().next() {
        return Ok(Some(hash));
    }
    let revision = format!("{query_hash}^{{commit}}");
    let output = silent_command("git")
        .arg("-C")
        .arg(root)
        .arg("rev-parse")
        .arg("--verify")
        .arg(revision)
        .output();
    if let Ok(output) = output {
        if output.status.success() {
            let resolved = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            if let Some(hash) = normalize_stored_hash(&resolved) {
                return Ok(Some(hash));
            }
        }
    }
    if query_hash.len() >= 40 {
        Ok(Some(query_hash.to_owned()))
    } else {
        Ok(None)
    }
}

fn stored_hash_matches(query_hash: &str, stored_hash: &str) -> bool {
    let query_hash = query_hash.trim().to_ascii_lowercase();
    let stored_hash = stored_hash.trim().to_ascii_lowercase();
    query_hash == stored_hash
        || (query_hash.len() < stored_hash.len() && stored_hash.starts_with(&query_hash))
        || (stored_hash.len() < query_hash.len() && query_hash.starts_with(&stored_hash))
}

fn git_commit_in_session_range(root: &Path, commit: &str, open: &str, close: &str) -> bool {
    if commit == open || commit == close || open == close {
        return false;
    }
    git_is_ancestor(root, open, commit) && git_is_ancestor(root, commit, close)
}

fn git_is_ancestor(root: &Path, ancestor: &str, descendant: &str) -> bool {
    silent_command("git")
        .arg("-C")
        .arg(root)
        .args(["merge-base", "--is-ancestor", ancestor, descendant])
        .status()
        .is_ok_and(|status| status.success())
}

fn add_commit_task_association(
    associations: &mut BTreeMap<String, Value>,
    task: &TaskDocument,
    state: &str,
    session_id: &str,
    match_kind: &str,
    trace: &SessionGitTrace,
) {
    let entry = associations.entry(task.task_id.clone()).or_insert_with(|| {
        json!({
            "task_id": task.task_id,
            "title": task.title,
            "intent": task.intent,
            "state": state,
            "sessions": []
        })
    });
    let sessions = entry
        .get_mut("sessions")
        .and_then(Value::as_array_mut)
        .expect("commit association sessions must be an array");
    if let Some(existing) = sessions
        .iter_mut()
        .find(|session| session["session_id"].as_str() == Some(session_id))
    {
        let match_kinds = existing
            .get_mut("match_kinds")
            .and_then(Value::as_array_mut)
            .expect("commit association match_kinds must be an array");
        if !match_kinds
            .iter()
            .any(|kind| kind.as_str() == Some(match_kind))
        {
            match_kinds.push(Value::String(match_kind.to_owned()));
        }
        if existing["open_git_head"].is_null() {
            existing["open_git_head"] = trace
                .open_git_head
                .clone()
                .map(Value::String)
                .unwrap_or(Value::Null);
        }
        if existing["close_git_head"].is_null() {
            existing["close_git_head"] = trace
                .close_git_head
                .clone()
                .map(Value::String)
                .unwrap_or(Value::Null);
        }
        return;
    }
    sessions.push(json!({
        "session_id": session_id,
        "match_kinds": [match_kind],
        "open_git_head": trace.open_git_head,
        "close_git_head": trace.close_git_head,
        "git_trace_status": git_trace_status(trace.open_git_head.as_deref(), trace.close_git_head.as_deref())
    }));
}

fn archived_task_summary(
    task: &TaskDocument,
    events: &[V3EventEnvelope],
    lifecycle: &TaskLifecycleProjection,
    state: &str,
    generated_at: &str,
) -> Value {
    let task_events: Vec<&V3EventEnvelope> = events
        .iter()
        .filter(|event| event.task_id.0 == task.task_id)
        .collect();
    let criteria = criteria(task, events, lifecycle, generated_at);
    let completion = completion_view(lifecycle);
    let confirmed = completion
        .get("confirmed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let terminal_at = task_events
        .iter()
        .filter(|event| {
            matches!(
                event.event_type.as_str(),
                "task.completion_confirmed"
                    | "task.completion_proposed"
                    | "task.completion_rejected"
                    | "task.closed_with_exceptions"
            )
        })
        .map(|event| event.occurred_at.clone())
        .max()
        .or_else(|| {
            task_events
                .iter()
                .map(|event| event.occurred_at.clone())
                .max()
        });
    let latest_result = task_events
        .iter()
        .filter(|event| event.event_type == "agent.result_recorded")
        .max_by(|left, right| left.occurred_at.cmp(&right.occurred_at));
    let result_status = latest_result
        .and_then(|event| event.payload.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("not_executed");
    let result_summary = latest_result
        .and_then(|event| event.payload.get("summary"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|summary| !summary.is_empty())
        .map(ToOwned::to_owned);
    let artifact_count = latest_result
        .and_then(|event| event.payload.get("artifacts"))
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let evidence_refs = evidence_refs(task, events, generated_at);
    let session_events = sessions_for_task(events, &task.task_id);
    let plan_total = lifecycle.effective_nodes().count();
    let plan_completed = lifecycle
        .effective_nodes()
        .filter(|(_, node)| node.state == "completed")
        .count();
    let plan_cancelled = lifecycle
        .effective_nodes()
        .filter(|(_, node)| node.state == "cancelled")
        .count();
    let plan_active = lifecycle
        .effective_nodes()
        .filter(|(_, node)| node.state == "active")
        .count();
    let plan_blocked = lifecycle
        .effective_nodes()
        .filter(|(_, node)| matches!(node.state.as_str(), "blocked" | "failed"))
        .count();
    let plan_planned = lifecycle
        .effective_nodes()
        .filter(|(_, node)| matches!(node.state.as_str(), "planned" | "ready"))
        .count();
    let finding_total = lifecycle.findings.len();
    let finding_closed = lifecycle
        .findings
        .values()
        .filter(|finding| finding.state == "closed")
        .count();
    let finding_open = finding_total.saturating_sub(finding_closed);
    let closure_event = task_events
        .iter()
        .filter(|event| {
            matches!(
                event.event_type.as_str(),
                "task.completion_confirmed" | "task.closed_with_exceptions"
            )
        })
        .max_by(|left, right| left.occurred_at.cmp(&right.occurred_at));
    let closure_method = closure_event.map(|event| {
        if event.event_type == "task.completion_confirmed" {
            "all_green"
        } else {
            "with_exceptions"
        }
    });
    let unresolved_items = criteria
        .iter()
        .filter(|criterion| {
            !matches!(
                criterion["status"].as_str(),
                Some("passed" | "not_applicable")
            )
        })
        .filter_map(|criterion| criterion["criterion_id"].as_str().map(str::to_owned))
        .collect::<Vec<_>>();
    let next_action = if state == "completed" && confirmed {
        "无待处理动作；可从验收与时间线回顾本次任务".to_owned()
    } else if state == "completed" {
        "等待用户或受信渠道确认完成；当前不能视为最终绿灯".to_owned()
    } else {
        "确认是否需要重新开始，或基于本次结果创建后续任务".to_owned()
    };

    json!({
        "task_id": task.task_id,
        "title": task.title,
        "intent": task.intent,
        "state": state,
        "risk_level": risk_level(events, &task.task_id),
        "terminal_at": terminal_at,
        "completion": completion,
        "closure": {
            "method": closure_method,
            "actor": closure_event.map(|event| event.actor.clone()),
            "confirmed_by": closure_event.and_then(|event| event.payload.get("confirmed_by")).cloned().unwrap_or(Value::Null),
            "channel": closure_event.and_then(|event| event.payload.get("channel")).cloned().unwrap_or(Value::Null),
            "confirmed_at": closure_event.map(|event| event.occurred_at.clone()),
            "reason": closure_event.and_then(|event| event.payload.get("reason")).cloned().unwrap_or(Value::Null),
            "criteria_snapshot": criteria.clone(),
            "unresolved_items": unresolved_items
        },
        "criteria": criteria,
        "blocker_details": blocker_details(task, events, lifecycle, None, generated_at),
        "plan": {
            "total": plan_total,
            "completed": plan_completed,
            "cancelled": plan_cancelled,
            "active": plan_active,
            "blocked": plan_blocked,
            "planned": plan_planned
        },
        "sessions": {
            "total": lifecycle.sessions.len().max(session_events.len()),
            "opened": session_events.iter().filter(|(_, opened, _)| *opened).count(),
            "closed": session_events.iter().filter(|(_, _, closed)| *closed).count(),
            "gapped": lifecycle.sessions.values().filter(|session| session.state == "gapped").count()
        },
        "findings": {"total": finding_total, "open": finding_open, "closed": finding_closed},
        "result": {
            "status": result_status,
            "summary": result_summary,
            "artifact_count": artifact_count,
            "recorded_at": latest_result.map(|event| event.recorded_at.clone())
        },
        "evidence_count": evidence_refs.len(),
        "evidence_refs": evidence_refs,
        "next_action": next_action,
        "source": "v3_projection"
    })
}

fn is_terminal_task_state(state: &str) -> bool {
    matches!(state, "completed" | "cancelled" | "closed_with_exceptions")
}

fn validate_id(name: &str, value: &str) -> Result<(), V3Error> {
    if value.len() < 3
        || value.len() > 128
        || !value.starts_with(|character: char| character.is_ascii_alphabetic())
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "._:-".contains(character))
    {
        return Err(V3Error::new(
            "V3_VALIDATION_ERROR",
            V3ErrorCategory::Validation,
            false,
            format!("{name} is not a stable identifier"),
        ));
    }
    Ok(())
}

fn criteria(
    task: &TaskDocument,
    events: &[V3EventEnvelope],
    lifecycle: &TaskLifecycleProjection,
    generated_at: &str,
) -> Vec<Value> {
    task.acceptance_criteria
        .iter()
        .enumerate()
        .map(|(index, title)| {
            let id = canonical_criterion_id(&task.task_id, index);
            let lifecycle_criterion = lifecycle.criteria.get(&id);
            let passed = events.iter().any(|event| {
                event.task_id.0 == task.task_id
                    && event.event_type == "criterion.passed"
                    && event.payload.get("criterion_id").and_then(Value::as_str) == Some(&id)
            });
            let status = lifecycle_criterion.map(|criterion| match criterion.state {
                CriterionState::Proposed => "proposed", CriterionState::Accepted => "accepted", CriterionState::Passed => "passed",
                CriterionState::Failed => "failed", CriterionState::Blocked => "blocked", CriterionState::NotApplicable => "not_applicable",
            }).unwrap_or(if passed {"passed"} else {"accepted"});
            let evidence_refs = lifecycle_criterion.map(|criterion| criterion.evidence_refs.iter().map(|reference| json!({
                "evidence_id": reference, "kind": "event", "grade": "hard_observed", "label_key": "v3.evidence.criterion", "locator": reference, "captured_at": generated_at
            })).collect::<Vec<_>>()).unwrap_or_default();
            json!({"criterion_id": id, "title": title, "status": status, "required": lifecycle_criterion.map(|criterion| criterion.required).unwrap_or(true), "evidence_refs": evidence_refs})
        })
        .collect()
}

fn blocker_details(
    task: &TaskDocument,
    events: &[V3EventEnvelope],
    lifecycle: &TaskLifecycleProjection,
    node_id: Option<&str>,
    generated_at: &str,
) -> Vec<Value> {
    let orchestration = fold_orchestration(&task.task_id, events);
    let session_integrity = session_integrity(events, &task.task_id, lifecycle);
    let task_has_blocked_state = lifecycle
        .nodes
        .values()
        .any(|node| matches!(node.state.as_str(), "blocked" | "failed"))
        || lifecycle
            .criteria
            .values()
            .any(|criterion| criterion.state == CriterionState::Blocked)
        || lifecycle
            .sessions
            .values()
            .any(|session| session.state == "gapped")
        || session_integrity.has_blocking_gap()
        || lifecycle.has_open_findings()
        || orchestration.worktrees.values().any(|worktree| {
            matches!(
                worktree.state,
                WorktreeState::Conflicted | WorktreeState::Repairing
            )
        });
    let node_has_blocked_state = node_id
        .and_then(|id| lifecycle.effective_node(id))
        .is_some_and(|node| matches!(node.state.as_str(), "blocked" | "failed"));

    // Blocker details describe current workflow truth. Historical risk and blocked
    // events remain available in the timeline, but must not keep a recovered task
    // or completed node visually blocked forever.
    if (node_id.is_some() && !node_has_blocked_state)
        || (node_id.is_none() && !task_has_blocked_state)
    {
        return Vec::new();
    }

    let mut candidates: Vec<&V3EventEnvelope> = events
        .iter()
        .filter(|event| event.task_id.0 == task.task_id)
        .filter(|event| {
            node_id.map_or(true, |wanted| {
                event.node_id.as_ref().is_some_and(|id| id.0 == wanted)
                    || event.payload.get("node_id").and_then(Value::as_str) == Some(wanted)
            })
        })
        .filter(|event| is_blocker_event(event))
        .collect();
    candidates.reverse();

    let mut seen_reasons = BTreeSet::new();
    let mut details = Vec::new();
    for event in candidates {
        let (reason_code, kind) = blocker_classification(event);
        if !seen_reasons.insert(reason_code.clone()) {
            continue;
        }
        details.push(blocker_detail_from_event(
            event,
            &reason_code,
            kind,
            task,
            lifecycle,
            node_id,
            generated_at,
        ));
        if details.len() >= 4 {
            break;
        }
    }

    // A node-level event often only contains the technical lifecycle code. Merge the
    // task-level evidence so a blocked plan node shows the human-readable cause too.
    if node_has_blocked_state {
        for detail in blocker_details(task, events, lifecycle, None, generated_at) {
            let reason = detail
                .get("reason_code")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            if seen_reasons.insert(reason.to_owned()) {
                details.push(detail);
            }
            if details.len() >= 6 {
                break;
            }
        }
    }

    if node_id.is_none() && task_has_blocked_state {
        let mut blocking_session_ids = session_integrity.blocking_session_ids();
        blocking_session_ids.extend(
            lifecycle
                .sessions
                .values()
                .filter(|session| matches!(session.state.as_str(), "gapped" | "unknown"))
                .map(|session| session.session_id.clone()),
        );
        blocking_session_ids.sort();
        blocking_session_ids.dedup();
        for session_id in blocking_session_ids {
            let session = lifecycle.sessions.get(&session_id);
            let state = session
                .map(|value| value.state.as_str())
                .unwrap_or("unknown");
            let missing_terminal_result = !session_integrity.terminal_results.contains(&session_id);
            let legacy_unknown = !session_integrity.opened.contains(&session_id);
            let reason = if state == "gapped" {
                format!("session.gap:{session_id}")
            } else if legacy_unknown {
                format!("session.unknown:{session_id}")
            } else if missing_terminal_result {
                format!("session.terminal_result_missing:{session_id}")
            } else {
                format!("session.unknown:{session_id}")
            };
            if !seen_reasons.insert(reason.clone()) {
                continue;
            }
            let (summary, why_blocked, expected_state, observed_state, resume_action) = if state
                == "gapped"
            {
                (
                        format!("Session {session_id} 存在未恢复的执行 gap"),
                        "session.gap_detected 已记录，但尚无匹配的 session.recovered 证据".to_owned(),
                        "session=recovered/closed，gap evidence 已核对".to_owned(),
                        format!("session={state}, terminal_result={missing_terminal_result}"),
                        format!("调用 session_recovery(action=recover, session_id={session_id}, evidence_refs=[...])，再记录 terminal agent_result 并关闭 session"),
                    )
            } else if legacy_unknown {
                (
                        format!("Legacy/unknown Session {session_id} 没有 session.opened 和 terminal agent result"),
                        "历史事件带有 Session 关联，但没有可确认的 Session open 事实；V3 不补写成功或 terminal result".to_owned(),
                        "session.opened 与 succeeded/failed terminal agent.result_recorded 均存在".to_owned(),
                        format!("session={state}, opened=false, closed={}, terminal_result={missing_terminal_result}", session_integrity.closed.contains(&session_id)),
                        format!("核对历史 Session {session_id} 的真实执行事实；无法恢复时保持 blocked，并用新的受支持 Session 重新执行和验证"),
                    )
            } else {
                (
                        format!("Session {session_id} 没有可核验的 terminal agent result"),
                        "历史 Session 只有绑定/打开/关闭等事实，缺少 succeeded 或 failed 的 terminal result；不能把历史 criterion 或 archive 摘要当作执行成功".to_owned(),
                        "session 有 terminal agent.result_recorded(status=succeeded|failed)，且必要时完成 gap recovery".to_owned(),
                        format!("session={state}, opened={}, closed={}, terminal_result=false", session_integrity.opened.contains(&session_id), session_integrity.closed.contains(&session_id)),
                        format!("核对 Session {session_id} 的真实工作、工作树和验证输出；如为中断先调用 session_recovery(recover)，然后记录 succeeded/failed terminal agent_result，再 session_close"),
                    )
            };
            details.push(typed_blocker_detail(
                format!("blocker.{}.session-gap", session_id),
                BlockerKind::Workflow,
                reason,
                "session".to_owned(),
                session_id.clone(),
                summary,
                why_blocked,
                expected_state,
                observed_state,
                vec![
                    "terminal agent.result_recorded 的真实状态".to_owned(),
                    "Session gap/recovery evidence（如适用）".to_owned(),
                ],
                "该 Session 的执行事实不完整，结果与完成门禁不可依赖".to_owned(),
                session
                    .map(|value| value.host.clone())
                    .unwrap_or_else(|| "Session owner".to_owned()),
                vec!["定位中断点并核对工作树、节点和已执行验证".to_owned()],
                resume_action,
                None,
                None,
                session.and_then(|value| value.node_id.clone()),
                Vec::new(),
                BlockerProvenanceStatus::Native,
                Vec::new(),
                Vec::new(),
            ));
            if details.len() >= 6 {
                break;
            }
        }

        for finding in lifecycle
            .findings
            .values()
            .filter(|finding| finding.state != "closed")
        {
            let reason = format!("finding.open:{}", finding.finding_id);
            if !seen_reasons.insert(reason.clone()) {
                continue;
            }
            let missing = if finding.attempt_ids.is_empty() {
                vec![
                    "至少一个 typed remediation attempt".to_owned(),
                    "finding closure review".to_owned(),
                ]
            } else {
                vec!["finding.closed 结论与关闭证据".to_owned()]
            };
            details.push(typed_blocker_detail(
                format!("blocker.{}.open", finding.finding_id),
                BlockerKind::Workflow,
                reason,
                "finding".to_owned(),
                finding.finding_id.clone(),
                format!("Finding {} 尚未闭环", finding.finding_id),
                format!("finding state={}；完成门禁要求所有 finding=closed", finding.state),
                "finding=closed，且至少一个 remediation attempt 有 evidence".to_owned(),
                format!("finding={}, attempts={}", finding.state, finding.attempt_ids.len()),
                missing,
                "相关 criterion、节点和任务完成门禁保持阻塞".to_owned(),
                "finding owner/reviewer".to_owned(),
                vec!["复现 finding 并确认修复范围".to_owned()],
                format!("调用 attempt_manage 记录针对 {} 的修复与 evidence，验证后调用 finding_manage(action=close)", finding.finding_id),
                None,
                None,
                finding.target_node_id.clone(),
                string_evidence_refs(&finding.evidence_refs, generated_at, "v3.evidence.finding"),
                BlockerProvenanceStatus::Native,
                Vec::new(),
                Vec::new(),
            ));
        }

        for (_, node) in lifecycle
            .effective_nodes()
            .filter(|(_, node)| matches!(node.state.as_str(), "blocked" | "failed"))
        {
            let incomplete_dependencies = node
                .dependencies
                .iter()
                .filter(|dependency| {
                    lifecycle.nodes.get(*dependency).is_none_or(|dependency| {
                        !dependency.is_historical_bootstrap()
                            && !matches!(dependency.state.as_str(), "completed" | "waived")
                    })
                })
                .cloned()
                .collect::<Vec<_>>();
            if incomplete_dependencies.is_empty() {
                continue;
            }
            let reason = format!("plan.dependency_not_ready:{}", node.node_id);
            if !seen_reasons.insert(reason.clone()) {
                continue;
            }
            details.push(typed_blocker_detail(
                format!("blocker.{}.dependency", node.node_id),
                BlockerKind::Dependency,
                reason,
                "plan_node".to_owned(),
                node.node_id.clone(),
                format!("节点 {} 的依赖尚未完成", node.title),
                "DAG validator 禁止在依赖未 completed/waived 时恢复或完成下游节点".to_owned(),
                "所有 scheduling dependencies=completed/waived".to_owned(),
                format!("未完成依赖：{}", incomplete_dependencies.join(", ")),
                incomplete_dependencies
                    .iter()
                    .map(|dependency| format!("依赖 {dependency} 的 terminal state 与 evidence"))
                    .collect(),
                "下游节点不能进入 ready/active，关联 criteria 无法验收".to_owned(),
                "依赖节点 owner".to_owned(),
                vec!["不得绕过 DAG；先处理列出的依赖节点".to_owned()],
                format!(
                    "依次完成或经授权 waive：{}；随后重新读取 task_view 并激活 {}",
                    incomplete_dependencies.join(", "),
                    node.node_id
                ),
                None,
                None,
                Some(node.node_id.clone()),
                Vec::new(),
                BlockerProvenanceStatus::Native,
                Vec::new(),
                Vec::new(),
            ));
        }

        for worktree in orchestration.worktrees.values().filter(|worktree| {
            matches!(
                worktree.state,
                WorktreeState::Conflicted | WorktreeState::Repairing
            )
        }) {
            let reason = format!("worktree.integration:{:?}", worktree.state).to_ascii_lowercase();
            if !seen_reasons.insert(reason.clone()) {
                continue;
            }
            let lease_fact = worktree
                .lease
                .as_ref()
                .map(|lease| {
                    format!(
                        "lease {} state={:?} owner={}",
                        lease.lease_id, lease.state, lease.owner_session_id
                    )
                })
                .unwrap_or_else(|| "lease=none".to_owned());
            let owner = worktree
                .lease
                .as_ref()
                .map(|lease| lease.owner_session_id.clone())
                .unwrap_or_else(|| "integration owner".to_owned());
            let missing_facts = if worktree.state == WorktreeState::Conflicted {
                vec![
                    "冲突文件的 resolution evidence".to_owned(),
                    "新的 eligibility_digest".to_owned(),
                    "integrated event".to_owned(),
                ]
            } else {
                vec!["repair 结果与 terminal worktree state".to_owned()]
            };
            details.push(typed_blocker_detail(
                format!("blocker.{}.integration", worktree.worktree_id),
                if worktree.state == WorktreeState::Conflicted { BlockerKind::Conflict } else { BlockerKind::Workflow },
                reason,
                "integration".to_owned(),
                worktree.worktree_id.clone(),
                format!("Worktree {} 集成状态为 {:?}", worktree.worktree_id, worktree.state),
                "worktree 尚未进入 integrated/cleaned，当前 eligibility/lease 事实不能证明可安全集成".to_owned(),
                "worktree=integrated/cleaned，lease=released/reclaimed，eligibility digest 有效".to_owned(),
                format!("worktree={:?}；{lease_fact}", worktree.state),
                missing_facts,
                "变更不能成为任务集成真值，完成提议必须被拒绝".to_owned(),
                owner,
                vec!["保留冲突与当前 worktree 证据，不覆盖其他 owner 的变更".to_owned()],
                format!("通过 orchestration_write 修复 {}，重新计算 eligibility_digest，完成 integration 并 release/reclaim lease", worktree.worktree_id),
                None,
                None,
                Some(worktree.node_id.clone()),
                string_evidence_refs(&worktree.event_ids, generated_at, "v3.evidence.worktree"),
                BlockerProvenanceStatus::Native,
                worktree.event_ids.clone(),
                Vec::new(),
            ));
        }
    }

    // Accepted criteria without evidence are an actionable evidence gap, not an
    // implementation step. Surface them only when the task/node is blocked so a
    // normal in-flight task does not look blocked merely because it is unverified.
    if node_id.is_none() && task_has_blocked_state {
        for (index, title) in task.acceptance_criteria.iter().enumerate() {
            let criterion_id = canonical_criterion_id(&task.task_id, index);
            let Some(criterion) = lifecycle.criteria.get(&criterion_id) else {
                continue;
            };
            if criterion.state != CriterionState::Accepted || !criterion.evidence_refs.is_empty() {
                continue;
            }
            let reason = format!("acceptance.evidence_gap:{criterion_id}");
            if !seen_reasons.insert(reason.clone()) {
                continue;
            }
            let precondition = "在标准指定的真实执行环境中完成检查并保留可核验输出".to_owned();
            let resume_action =
                format!("执行并记录验收标准：{title}；随后由受信 reviewer 运行 criterion_review");
            details.push(typed_blocker_detail(
                format!("blocker.{}.evidence-gap", criterion_id),
                BlockerKind::EvidenceGap,
                reason,
                "criterion".to_owned(),
                criterion_id.clone(),
                format!("验收标准尚无可核验证据：{title}"),
                "该必需 criterion 仍为 accepted 且没有 evidence，完成门禁不能证明标准已满足"
                    .to_owned(),
                "criterion 由真实验证推进到 passed，并包含可解析的 evidence_refs".to_owned(),
                "criterion=accepted，evidence_refs=[]".to_owned(),
                vec![format!("该标准的验证输出与 reviewer 结论：{title}")],
                "任务不能进入 completion_pending".to_owned(),
                "验收执行者/审查者".to_owned(),
                vec![precondition.clone()],
                resume_action.clone(),
                None,
                Some(criterion_id),
                None,
                Vec::new(),
                BlockerProvenanceStatus::Native,
                Vec::new(),
                Vec::new(),
            ));
        }
    }

    if details.is_empty() && (task_has_blocked_state || node_has_blocked_state) {
        details.push(generic_blocker_detail(node_id));
    }
    details
}

fn is_blocker_event(event: &V3EventEnvelope) -> bool {
    let payload = &event.payload;
    match event.event_type.as_str() {
        "criterion.blocked" => true,
        "plan.node_state_changed" => payload
            .get("state")
            .and_then(Value::as_str)
            .is_some_and(|state| matches!(state, "blocked" | "failed")),
        "risk.logged" => {
            let text = blocker_signal_text(payload);
            payload
                .get("external_action_required")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || payload.get("blocked_on").is_some()
                || payload.get("macos_accessibility").is_some()
                || payload.get("resume_condition").is_some()
                || text.contains("blocked")
                || text.contains("阻塞")
                || text.contains("accessibility")
                || text.contains("tcc")
                || text.contains("权限")
                || text.contains("证据缺口")
                || text.contains("evidence gap")
        }
        "agent.result_recorded" => {
            payload.get("status").and_then(Value::as_str) == Some("failed")
                || payload
                    .get("external_action_required")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                || payload.get("blocked_on").is_some()
                || payload.get("macos_accessibility").is_some()
                || payload.get("resume_condition").is_some()
        }
        _ => false,
    }
}

fn blocker_classification(event: &V3EventEnvelope) -> (String, &'static str) {
    let payload = &event.payload;
    let text = blocker_signal_text(payload);
    if text.contains("accessibility")
        || text.contains("system events")
        || text.contains("tcc")
        || text.contains("-25211")
        || text.contains("辅助访问")
        || text.contains("权限")
    {
        return ("macos.accessibility.tcc".to_owned(), "permission");
    }
    if payload
        .get("external_action_required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return ("external.precondition".to_owned(), "external_precondition");
    }
    if event.event_type == "criterion.blocked" {
        return ("criterion.blocked".to_owned(), "evidence_gap");
    }
    if let Some(reason) = payload
        .get("block_reasons")
        .and_then(Value::as_array)
        .and_then(|items| items.iter().find_map(Value::as_str))
    {
        return (reason.to_owned(), "workflow");
    }
    if event.event_type == "plan.node_state_changed" {
        return ("lifecycle.blocked".to_owned(), "workflow");
    }
    if text.contains("证据")
        || text.contains("evidence")
        || text.contains("gap")
        || text.contains("缺口")
    {
        return ("acceptance.evidence_gap".to_owned(), "evidence_gap");
    }
    ("workflow.blocked".to_owned(), "workflow")
}

fn blocker_signal_text(payload: &Value) -> String {
    [
        "summary",
        "reason",
        "message",
        "body",
        "error",
        "blocked_on",
        "resume_condition",
        "required_next",
        "next",
        "owner",
        "owner_party",
        "blocked_by",
        "responsible",
        "risk_code",
    ]
    .iter()
    .filter_map(|key| payload.get(*key).and_then(Value::as_str))
    .map(str::to_lowercase)
    .collect::<Vec<_>>()
    .join(" ")
}

fn blocker_detail_from_event(
    event: &V3EventEnvelope,
    reason_code: &str,
    kind: &str,
    task: &TaskDocument,
    lifecycle: &TaskLifecycleProjection,
    node_id: Option<&str>,
    generated_at: &str,
) -> Value {
    let payload = &event.payload;
    let criterion_id = payload
        .get("criterion_id")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let criterion_title = criterion_id.as_deref().and_then(|wanted| {
        task.acceptance_criteria
            .iter()
            .enumerate()
            .find(|(index, _)| canonical_criterion_id(&task.task_id, *index) == wanted)
            .map(|(_, title)| title.as_str())
    });
    let mut summary = payload_text(payload, &["summary", "reason", "message", "body", "error"])
        .unwrap_or_else(|| match kind {
            "permission" => "外部权限或原生交互前置条件未满足".to_owned(),
            "external_precondition" => "外部环境前置条件未满足".to_owned(),
            "evidence_gap" => criterion_title.map_or_else(
                || "验收证据缺口尚未闭环".to_owned(),
                |title| format!("验收标准阻塞：{title}"),
            ),
            _ => "工作流节点处于阻塞状态".to_owned(),
        });
    if let Some(title) = criterion_title.filter(|title| !summary.contains(*title)) {
        summary = format!("{title}：{summary}");
    }
    let release_handoff = release_handoff_context(payload);
    let precondition = payload_text(
        payload,
        &["resume_condition", "required_next", "blocked_on"],
    )
    .or_else(|| {
        release_handoff
            .as_ref()
            .map(|context| context.precondition.clone())
    })
    .unwrap_or_else(|| match kind {
        "permission" => {
            "为当前执行宿主授予 macOS Accessibility/System Events 权限，并完成一次复检".to_owned()
        }
        "external_precondition" => "完成外部环境前置后重新运行受影响验证".to_owned(),
        "evidence_gap" => criterion_title.map_or_else(
            || "补充真实 evidence，并由受信 reviewer 确认".to_owned(),
            |title| format!("在该标准要求的真实环境执行“{title}”并保留原始输出"),
        ),
        _ => "补充具体阻塞原因、依赖或恢复条件".to_owned(),
    });
    let mut resume_action = payload_text(payload, &["next", "required_next", "resume_condition"])
        .or_else(|| release_handoff.as_ref().map(|context| context.repair_action.clone()))
        .unwrap_or_else(|| match kind {
            "permission" => "解除权限阻塞后恢复对应 plan node，只复验未覆盖的原生链路".to_owned(),
            "external_precondition" => {
                "解除外部前置后恢复节点并记录新的 progress/evidence".to_owned()
            }
            "evidence_gap" => criterion_title.map_or_else(
                || "执行验收标准并记录 evidence，再进入审查".to_owned(),
                |title| format!("执行“{title}”，记录 evidence_refs，并调用 criterion_review(outcome=passed|failed|blocked)"),
            ),
            _ => "在计划图中补充 blocker details 后将节点恢复为 ready/active".to_owned(),
        });
    if criterion_id.is_some() && !resume_action.contains("criterion_review") {
        resume_action.push_str(
            "；完成后调用 criterion_review(outcome=passed|failed|blocked) 并附 evidence_refs",
        );
    }
    let owner = payload_text(
        payload,
        &["owner", "owner_party", "blocked_by", "responsible"],
    )
    .unwrap_or_else(|| match kind {
        "permission" => "用户/当前 macOS 执行宿主".to_owned(),
        "external_precondition" => "外部环境/用户".to_owned(),
        "evidence_gap" => "验收执行者/审查者".to_owned(),
        _ => "V3 执行者".to_owned(),
    });
    let event_node_id = event.node_id.as_ref().map(|id| id.0.clone()).or_else(|| {
        payload
            .get("node_id")
            .and_then(Value::as_str)
            .map(str::to_owned)
    });
    let evidence_refs = blocker_evidence_refs(event, generated_at);
    let source_type = if criterion_id.is_some() {
        "criterion"
    } else if event.event_type.starts_with("session.") {
        "session"
    } else if event_node_id.is_some() || node_id.is_some() {
        "plan_node"
    } else {
        "event"
    };
    let source_id = criterion_id
        .clone()
        .or_else(|| event_node_id.clone())
        .unwrap_or_else(|| event.event_id.clone());
    let expected_state = payload_text(payload, &["expected_state", "expected"])
        .or_else(|| {
            release_handoff
                .as_ref()
                .map(|context| context.expected_state.clone())
        })
        .unwrap_or_else(|| expected_state_for_kind(kind).to_owned());
    let observed_state = payload_text(payload, &["observed_state", "observed", "actual"])
        .or_else(|| {
            release_handoff
                .as_ref()
                .map(|context| context.observed_state.clone())
        })
        .unwrap_or_else(|| observed_state_for_event(event, kind));
    let existing_criterion_evidence = criterion_id
        .as_deref()
        .and_then(|id| lifecycle.criteria.get(id))
        .map(|criterion| criterion.evidence_refs.len())
        .unwrap_or_default();
    let missing_facts = payload_string_list(payload, "missing_facts")
        .or_else(|| release_handoff.as_ref().map(|context| context.missing_facts.clone()))
        .unwrap_or_else(|| {
        if kind == "evidence_gap" {
            vec![format!(
                "reviewer 尚未确认的验证结果（当前已有 {existing_criterion_evidence} 条 evidence；blocked 表示这些证据仍不足）"
            )]
        } else {
            default_missing_facts(kind, criterion_id.as_deref(), event_node_id.as_deref())
        }
        });
    let why_blocked = payload_text(payload, &["why_blocked", "reason", "summary"])
        .unwrap_or_else(|| {
            if kind == "evidence_gap" {
                format!("criterion review 为 blocked；现有 {existing_criterion_evidence} 条 evidence 未证明“{expected_state}”")
            } else {
                format!("期望状态“{expected_state}”与当前状态“{observed_state}”不一致")
            }
        });
    let impact = payload_text(payload, &["impact"])
        .unwrap_or_else(|| "受影响节点及任务完成门禁保持阻塞，不能被声明为完成".to_owned());
    let legacy_has_context = release_handoff.is_some()
        || payload_text(payload, &["summary", "reason", "message", "body", "error"]).is_some()
            && payload_text(
                payload,
                &["next", "required_next", "resume_condition", "blocked_on"],
            )
            .is_some();
    let provenance_status = if legacy_has_context {
        BlockerProvenanceStatus::Legacy
    } else {
        BlockerProvenanceStatus::Degraded
    };
    let unknown_fields = if provenance_status == BlockerProvenanceStatus::Degraded {
        vec![
            "why_blocked".to_owned(),
            "missing_facts".to_owned(),
            "repair_actions".to_owned(),
        ]
    } else {
        Vec::new()
    };
    typed_blocker_detail(
        format!("blocker.{}", event.event_id),
        blocker_kind(kind),
        reason_code.to_owned(),
        source_type.to_owned(),
        source_id,
        summary,
        why_blocked,
        expected_state,
        observed_state,
        missing_facts,
        impact,
        owner,
        vec![precondition],
        resume_action,
        None,
        criterion_id,
        node_id.or(event_node_id.as_deref()).map(str::to_owned),
        evidence_refs,
        provenance_status,
        vec![event.event_id.clone()],
        unknown_fields,
    )
}

struct ReleaseHandoffContext {
    precondition: String,
    repair_action: String,
    expected_state: String,
    observed_state: String,
    missing_facts: Vec<String>,
}

fn release_handoff_context(payload: &Value) -> Option<ReleaseHandoffContext> {
    let evidence = payload
        .get("evidence_refs")?
        .as_array()?
        .iter()
        .filter_map(|reference| {
            reference.as_str().map(str::to_owned).or_else(|| {
                reference
                    .get("locator")
                    .or_else(|| reference.get("evidence_id"))
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
        })
        .collect::<Vec<_>>();
    let blocked_checks = evidence
        .iter()
        .find(|reference| reference.contains("A07") && reference.contains("G05"))?;
    let artifact = evidence
        .iter()
        .find(|reference| reference.contains(".exe") && reference.contains("sha256="))?;
    let signing = evidence
        .iter()
        .find(|reference| reference.contains("WINDOWS_SIGNING_STATUS="))?;
    let checks = blocked_checks
        .strip_prefix("BLOCKED: ")
        .and_then(|value| value.strip_suffix(" await Windows host"))
        .unwrap_or("A07, E07, F07-F10, G05");

    Some(ReleaseHandoffContext {
        precondition: format!(
            "在 Windows 主机取得并校验发布产物：{artifact}；确认签名状态：{signing}"
        ),
        repair_action: format!(
            "在 Windows 主机使用精确发布产物（{artifact}，{signing}）执行 {checks}；记录 Windows/IDE 版本和每项原始结果，再调用 criterion_review(outcome=passed|failed|blocked)"
        ),
        expected_state: format!("Windows 原生验收 {checks} 均有真实主机 evidence"),
        observed_state: format!("发布产物已定位：{artifact}；{signing}；等待 Windows 主机执行 {checks}"),
        missing_facts: checks
            .split(',')
            .map(str::trim)
            .filter(|check| !check.is_empty())
            .map(|check| format!("Windows 原生验收 {check} 的真实结果"))
            .collect(),
    })
}

fn generic_blocker_detail(node_id: Option<&str>) -> Value {
    let source_id = node_id.unwrap_or("task").to_owned();
    typed_blocker_detail(
        format!("blocker.{source_id}"),
        BlockerKind::Workflow,
        "lifecycle.blocked.details_missing".to_owned(),
        if node_id.is_some() { "plan_node" } else { "task" }.to_owned(),
        source_id,
        "工作流已标记为阻塞，但旧事件没有保存可恢复的具体原因".to_owned(),
        "投影只能确认 blocked/failed 状态，不能从现有事件证明触发条件；缺失事实不会被伪造成已知事实".to_owned(),
        "存在包含原因、责任方、影响、缺失事实和解除条件的 risk/finding 记录".to_owned(),
        "仅观察到 lifecycle state=blocked/failed".to_owned(),
        vec!["原始阻塞原因".to_owned(), "责任主体".to_owned(), "解除条件".to_owned(), "验证证据".to_owned()],
        "当前节点和任务完成门禁保持阻塞".to_owned(),
        "V3 执行者/原阻塞责任方".to_owned(),
        vec!["从原执行环境、finding 或人工责任方取得真实阻塞事实".to_owned()],
        "通过 typed risk/finding 命令补录可核验事实与 evidence_refs，再按 validator 允许的 reopen/recover 流程恢复节点".to_owned(),
        None,
        None,
        node_id.map(str::to_owned),
        Vec::new(),
        BlockerProvenanceStatus::Degraded,
        Vec::new(),
        vec!["source_event_id".to_owned(), "why_blocked".to_owned(), "repair command".to_owned()],
    )
}

fn projection_stale_blocker_detail(project_id: &str, status: &Value, _generated_at: &str) -> Value {
    let event_count = status
        .get("event_count")
        .and_then(Value::as_u64)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_owned());
    let projection_event_count = status
        .get("projection_event_count")
        .and_then(Value::as_u64)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "missing_or_unreadable".to_owned());
    typed_blocker_detail(
        format!("blocker.projection.stale.{project_id}"),
        BlockerKind::Workflow,
        "projection.stale".to_owned(),
        "projection".to_owned(),
        project_id.to_owned(),
        "V3 projection 落后于事件日志，当前完成结论不可直接依赖".to_owned(),
        "事件日志与 projection 的计数不一致；视图保留事件事实，但不能把旧 projection 的 terminal 摘要当作当前完成真值".to_owned(),
        "projection_event_count=event_count 且 projection 可解析".to_owned(),
        format!("event_count={event_count}, projection_event_count={projection_event_count}"),
        vec![
            "projection rebuild 的结构化结果".to_owned(),
            "重新读取 task_view、task_candidates 和 archive 的一致状态".to_owned(),
        ],
        "所有受影响 Task 在 projection 追平前保持 stale/partial/blocked/review，不得宣称 completed".to_owned(),
        "V3 控制面维护者".to_owned(),
        vec!["保留当前事件日志和 projection 文件，不直接编辑 projection".to_owned()],
        format!("通过受支持的 `v3 rebuild {project_id}` 追平 projection，失败时保留结构化错误并重新验证"),
        None,
        None,
        None,
        Vec::new(),
        BlockerProvenanceStatus::Native,
        Vec::new(),
        Vec::new(),
    )
}

#[allow(clippy::too_many_arguments)]
fn typed_blocker_detail(
    blocker_id: String,
    kind: BlockerKind,
    reason_code: String,
    source_type: String,
    source_id: String,
    summary: String,
    why_blocked: String,
    expected_state: String,
    observed_state: String,
    missing_facts: Vec<String>,
    impact: String,
    owner: String,
    preconditions: Vec<String>,
    resume_action: String,
    command: Option<String>,
    criterion_id: Option<String>,
    node_id: Option<String>,
    evidence_refs: Vec<Value>,
    provenance_status: BlockerProvenanceStatus,
    source_event_ids: Vec<String>,
    unknown_fields: Vec<String>,
) -> Value {
    let precondition = preconditions.join("；");
    let action_id = format!("repair.{blocker_id}");
    let action_kind = if command.is_some() {
        RepairActionKind::Command
    } else if kind == BlockerKind::EvidenceGap {
        RepairActionKind::CollectEvidence
    } else {
        RepairActionKind::Manual
    };
    BlockerDetail {
        model_version: BLOCKER_MODEL_VERSION.to_owned(),
        blocker_id,
        kind,
        reason_code,
        source_type,
        source_id,
        summary,
        why_blocked,
        expected_state,
        observed_state,
        missing_facts,
        impact,
        owner: owner.clone(),
        preconditions: preconditions.clone(),
        repair_actions: vec![RepairAction {
            action_id,
            kind: action_kind,
            label: "解除此阻塞".to_owned(),
            instructions: resume_action.clone(),
            owner,
            preconditions,
            verification: "重新读取 task_view，并确认该 blocker_id 消失或其 observed_state 已满足 expected_state".to_owned(),
            command,
            target: criterion_id.clone().or_else(|| node_id.clone()),
            copy_text: resume_action.clone(),
        }],
        evidence_refs,
        freshness: "fresh".to_owned(),
        provenance: BlockerProvenance {
            status: provenance_status,
            source_event_ids,
            reconstructed_fields: vec![
                "source_type".to_owned(),
                "expected_state".to_owned(),
                "observed_state".to_owned(),
                "repair_actions".to_owned(),
            ],
            unknown_fields,
        },
        precondition,
        resume_action,
        criterion_id,
        node_id,
    }
    .into_value()
}

fn blocker_kind(kind: &str) -> BlockerKind {
    match kind {
        "external_precondition" => BlockerKind::ExternalPrecondition,
        "permission" => BlockerKind::Permission,
        "evidence_gap" => BlockerKind::EvidenceGap,
        "dependency" => BlockerKind::Dependency,
        "conflict" => BlockerKind::Conflict,
        "workflow" => BlockerKind::Workflow,
        _ => BlockerKind::Unknown,
    }
}

fn expected_state_for_kind(kind: &str) -> &'static str {
    match kind {
        "permission" => "required permission granted and native verification passed",
        "external_precondition" => "external precondition satisfied with evidence",
        "evidence_gap" => "required criterion terminal with valid evidence",
        _ => "workflow prerequisite satisfied and source entity terminal",
    }
}

fn observed_state_for_event(event: &V3EventEnvelope, kind: &str) -> String {
    event
        .payload
        .get("status")
        .or_else(|| event.payload.get("state"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{} emitted {}", event.event_type, kind))
}

fn default_missing_facts(
    kind: &str,
    criterion_id: Option<&str>,
    node_id: Option<&str>,
) -> Vec<String> {
    match kind {
        "permission" => vec!["权限授予状态".to_owned(), "原生复验输出".to_owned()],
        "evidence_gap" => vec![format!(
            "{} 的真实验证输出与 terminal review",
            criterion_id.unwrap_or("相关 criterion")
        )],
        _ => vec![format!(
            "{} 的解除条件与验证 evidence",
            node_id.unwrap_or("阻塞源")
        )],
    }
}

fn payload_string_list(payload: &Value, key: &str) -> Option<Vec<String>> {
    let values = payload
        .get(key)?
        .as_array()?
        .iter()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    (!values.is_empty()).then_some(values)
}

fn string_evidence_refs(references: &[String], generated_at: &str, label_key: &str) -> Vec<Value> {
    references
        .iter()
        .map(|reference| {
            json!({
                "evidence_id": reference,
                "kind": "event",
                "grade": "hard_observed",
                "label_key": label_key,
                "locator": reference,
                "captured_at": generated_at
            })
        })
        .collect()
}

fn payload_text(payload: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        payload
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    })
}

fn blocker_evidence_refs(event: &V3EventEnvelope, generated_at: &str) -> Vec<Value> {
    let timestamp = if event.recorded_at.is_empty() {
        generated_at.to_owned()
    } else {
        event.recorded_at.clone()
    };
    let mut refs = event
        .payload
        .get("evidence_refs")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    item.as_str()
                        .map(|reference| {
                            json!({
                                "evidence_id": reference,
                                "kind": "event",
                                "grade": event.evidence_grade,
                                "label_key": "v3.evidence.blocker",
                                "locator": reference,
                                "captured_at": timestamp
                            })
                        })
                        .or_else(|| item.as_object().map(|object| Value::Object(object.clone())))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    refs.push(json!({
        "evidence_id": event.event_id,
        "kind": "event",
        "grade": event.evidence_grade,
        "label_key": "v3.evidence.blocker",
        "locator": format!("vibehub://v3/events/{}", event.event_id),
        "captured_at": timestamp
    }));
    refs
}

fn completion_view(lifecycle: &TaskLifecycleProjection) -> Value {
    lifecycle.confirmation.as_ref().map_or_else(
        || json!({"proposal_event_id": Value::Null, "proposed_at_version": Value::Null, "digest": Value::Null, "valid": false, "confirmed": false, "confirmed_at_version": Value::Null, "confirmed_by": Value::Null, "channel": Value::Null}),
        |confirmation| json!({
            "proposal_event_id": confirmation.proposal_event_id,
            "proposed_at_version": confirmation.proposed_at_version,
            "digest": confirmation.digest,
            "valid": confirmation.valid,
            "confirmed": confirmation.valid && confirmation.confirmed_at_version.is_some(),
            "confirmed_at_version": confirmation.confirmed_at_version,
            "confirmed_by": confirmation.confirmed_by,
            "channel": confirmation.channel
        }),
    )
}

fn completion_gate_item(
    gate: &str,
    reason_code: &str,
    passed: bool,
    expected_state: &str,
    observed_state: String,
    missing_facts: Vec<String>,
    blocker_ids: Vec<String>,
    repair_actions: Vec<String>,
) -> Value {
    json!({
        "gate": gate,
        "status": if passed { "satisfied" } else { "blocked" },
        "passed": passed,
        "reason_code": reason_code,
        "summary": if passed { format!("{gate} 已满足") } else { format!("{gate} 尚未满足") },
        "expected_state": expected_state,
        "observed_state": observed_state,
        "satisfied_facts": if passed { vec![expected_state.to_owned()] } else { Vec::<String>::new() },
        "missing_facts": if passed { Vec::<String>::new() } else { missing_facts },
        "blocker_ids": blocker_ids,
        "repair_actions": if passed { Vec::<String>::new() } else { repair_actions }
    })
}

fn evidence_refs(
    task: &TaskDocument,
    events: &[V3EventEnvelope],
    generated_at: &str,
) -> Vec<Value> {
    let mut refs = vec![json!({
        "evidence_id": format!("evidence.{}.task", task.task_id.to_ascii_lowercase()), "kind": "file", "grade": "hard_observed",
        "label_key": "v3.evidence.task_metadata", "locator": format!(".vibehub/tasks/{}/task.yaml", task.task_id), "captured_at": generated_at
    })];
    refs.extend(events.iter().filter(|event| event.task_id.0 == task.task_id).map(|event| json!({
        "evidence_id": event.event_id, "kind": "event", "grade": event.evidence_grade,
        "label_key": "v3.evidence.event", "locator": format!("vibehub://v3/events/{}", event.event_id), "captured_at": event.recorded_at
    })));
    refs
}

fn sessions_for_task(events: &[V3EventEnvelope], task_id: &str) -> Vec<(String, bool, bool)> {
    let mut sessions = std::collections::BTreeMap::<String, (bool, bool)>::new();
    for event in events.iter().filter(|event| event.task_id.0 == task_id) {
        if let Some(session_id) = &event.session_id {
            let entry = sessions.entry(session_id.0.clone()).or_default();
            entry.0 |= event.event_type == "session.opened";
            entry.1 |= event.event_type == "session.closed";
        }
    }
    sessions
        .into_iter()
        .map(|(id, (open, closed))| (id, open, closed))
        .collect()
}

fn timeline_event(event: &V3EventEnvelope) -> Value {
    let kind = match event.event_type.as_str() {
        "risk.logged" | "finding.opened" | "finding.closed" | "finding.regressed" => "finding",
        "session.opened" | "session.closed" | "session.task_bound" | "session.task_unbound" => {
            "session"
        }
        event_type if event_type.starts_with("criterion.") => "validation",
        event_type if event_type.starts_with("attempt.") => "attempt",
        event_type if event_type.starts_with("plan.") => "plan",
        event_type if event_type.starts_with("task.completion_") => "confirmation",
        _ => "evidence",
    };
    json!({
        "timeline_event_id": event.event_id, "kind": kind, "occurred_at": event.occurred_at,
        "recorded_at": event.recorded_at, "order_state": "ordered",
        "lane_id": event.session_id.as_ref().map(|id| id.0.clone()).unwrap_or_else(|| "lane.task".to_owned()),
        "actor": event.actor, "tool": Value::Null, "node_id": event.node_id.as_ref().map(|id| id.0.clone()),
        "session_id": event.session_id.as_ref().map(|id| id.0.clone()), "commit_sha": event.commit_sha,
        "summary_key": event.event_type, "details": event.payload, "evidence_refs": []
    })
}

fn native_path(path: &Path) -> Value {
    let native = path.to_string_lossy().to_string();
    let platform = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unknown"
    };
    let (display, path_kind) = if platform == "windows" {
        (windows_display_path(&native), windows_path_kind(&native))
    } else {
        (native.clone(), "absolute")
    };
    json!({"platform": platform, "native": native, "display": display, "identity_key": format!("{platform}:{native}"), "path_kind": path_kind, "accessible": true})
}

fn windows_display_path(native: &str) -> String {
    let display = native.replace('\\', "/");
    if let Some(rest) = display.strip_prefix("//?/UNC/") {
        format!("//{rest}")
    } else if let Some(rest) = display.strip_prefix("//?/") {
        rest.to_owned()
    } else {
        display
    }
}

fn windows_path_kind(native: &str) -> &'static str {
    if native.starts_with(r"\\?\") {
        "extended"
    } else if native.starts_with(r"\\") {
        "unc"
    } else if native.as_bytes().get(1) == Some(&b':') {
        "drive"
    } else {
        "absolute"
    }
}

fn page(returned: usize) -> Value {
    json!({"cursor": Value::Null, "next_cursor": Value::Null, "limit": 200, "returned": returned, "total_estimate": returned, "truncated": false, "truncation_reason": "none", "model_version": MODEL_VERSION})
}

fn node_state(task: &TaskDocument) -> &'static str {
    if task.phase_status == "completed" {
        "completed"
    } else if task.phase_status == "blocked" {
        "blocked"
    } else {
        "active"
    }
}

fn projected_task_state(lifecycle: &TaskLifecycleProjection) -> &str {
    task_truth_state(lifecycle, None, false, false).as_str()
}

fn task_state_for(
    lifecycle: &TaskLifecycleProjection,
    fallback_state: Option<&str>,
    session_integrity: &SessionIntegrity,
    projection_stale: bool,
) -> &'static str {
    task_truth_state(
        lifecycle,
        fallback_state,
        session_integrity.has_blocking_gap(),
        projection_stale,
    )
    .as_str()
}

fn view_node_state(state: &str) -> &str {
    match state {
        "failed" => "blocked",
        "planned"
        | "ready"
        | "active"
        | "blocked"
        | "review"
        | "completed"
        | "cancelled"
        | "closed_with_exceptions"
        | "superseded" => state,
        _ => "planned",
    }
}

fn risk_level(events: &[V3EventEnvelope], task_id: &str) -> &'static str {
    if events
        .iter()
        .any(|event| event.task_id.0 == task_id && event.event_type == "risk.logged")
    {
        "medium"
    } else {
        "none"
    }
}

fn protocol_coverage(opened: usize, closed: usize, explicit_gaps: usize) -> &'static str {
    if explicit_gaps > 0 {
        "gapped"
    } else if opened == 0 {
        "unknown"
    } else if opened == closed {
        "complete"
    } else {
        "partial"
    }
}

fn internal(code: &'static str) -> impl FnOnce(std::io::Error) -> V3Error {
    move |error| V3Error::new(code, V3ErrorCategory::Internal, true, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v3::lifecycle::PlanNodeProjection;
    use crate::v3::{
        create_v3_task, EventDraft, EvidenceGrade, OrchestrationCommand, PlanAddNodeCommand,
        PlanCommandIdentity, PlanSetStateCommand, ProjectId, SessionId, TaskId,
        V3ApplicationService, V3TaskCreateInitialPlanNode, V3TaskCreateRequest,
    };
    use std::collections::BTreeSet;
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn windows_paths_preserve_native_identity_and_normalize_display() {
        let cases = [
            (r"C:\Users\Alex\VibeHub", "C:/Users/Alex/VibeHub", "drive"),
            (r"\\server\share\VibeHub", "//server/share/VibeHub", "unc"),
            (
                r"\\?\C:\Users\Alex\VibeHub",
                "C:/Users/Alex/VibeHub",
                "extended",
            ),
            (
                r"\\?\UNC\server\share\VibeHub",
                "//server/share/VibeHub",
                "extended",
            ),
        ];

        for (native, expected_display, expected_kind) in cases {
            assert_eq!(windows_display_path(native), expected_display);
            assert_eq!(windows_path_kind(native), expected_kind);
        }
    }

    #[test]
    fn task_state_reflects_operational_plan_state_before_completion_confirmation() {
        let mut lifecycle = TaskLifecycleProjection::empty("task.test");
        lifecycle.state = "active".to_owned();
        lifecycle.nodes.insert(
            "node.test".to_owned(),
            PlanNodeProjection {
                node_id: "node.test".to_owned(),
                title: "Test".to_owned(),
                goal: "Test state projection".to_owned(),
                state: "blocked".to_owned(),
                dependencies: BTreeSet::new(),
                scope: Vec::new(),
                criterion_ids: BTreeSet::new(),
                origin: super::super::lifecycle::PlanNodeOrigin::Authored,
                role: super::super::lifecycle::PlanNodeRole::Execution,
                origin_contract_version: 1,
            },
        );
        assert_eq!(projected_task_state(&lifecycle), "blocked");
        lifecycle.nodes.get_mut("node.test").unwrap().state = "active".to_owned();
        assert_eq!(projected_task_state(&lifecycle), "active");
        lifecycle.nodes.get_mut("node.test").unwrap().state = "completed".to_owned();
        assert_eq!(projected_task_state(&lifecycle), "review");
        lifecycle.state = "completed".to_owned();
        assert_eq!(projected_task_state(&lifecycle), "completed");
        lifecycle.nodes.get_mut("node.test").unwrap().state = "blocked".to_owned();
        lifecycle.state = "closed_with_exceptions".to_owned();
        assert_eq!(projected_task_state(&lifecycle), "closed_with_exceptions");
    }

    #[test]
    fn active_session_is_partial_coverage_not_a_protocol_gap() {
        assert_eq!(protocol_coverage(1, 0, 0), "partial");
        assert_eq!(protocol_coverage(1, 1, 0), "complete");
        assert_eq!(protocol_coverage(0, 0, 0), "unknown");
        assert_eq!(protocol_coverage(1, 0, 1), "gapped");
    }

    #[test]
    fn real_bundle_preserves_identity_across_all_views() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-views-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub/tasks/task.test")).unwrap();
        fs::create_dir_all(root.join(".vibehub/tasks/current")).unwrap();
        let task = "task_id: task.test\ntitle: Test task\nintent: Verify real views\nphase: implement\nphase_status: active\nacceptance_criteria:\n- Views validate\ndependencies: []\n";
        fs::write(root.join(".vibehub/tasks/task.test/task.yaml"), task).unwrap();
        fs::write(root.join(".vibehub/tasks/current/task.yaml"), task).unwrap();
        let repo = V3ViewRepository::open(&root).unwrap();
        let project_id = repo.project_id();
        V3ApplicationService::open(&root)
            .unwrap()
            .session_open(
                &project_id,
                "task.test",
                "session.test",
                "codex",
                0,
                "open.1",
            )
            .unwrap();
        let bundle = repo.load_bundle("task.test").unwrap();
        for view in [
            &bundle.project_overview,
            &bundle.project_structure,
            &bundle.agent_results,
            &bundle.task_timeline,
            &bundle.plan_graph,
            &bundle.node_brief,
        ] {
            assert_eq!(view["project_id"], project_id);
            assert_eq!(view["schema_version"], "1.0");
        }
        let timeline_events = bundle.task_timeline["events"].as_array().unwrap();
        assert_eq!(timeline_events.len(), 2);
        assert!(timeline_events
            .iter()
            .any(|event| event["summary_key"] == "session.task_bound"));
        assert!(timeline_events
            .iter()
            .any(|event| event["summary_key"] == "session.opened"));
        assert_eq!(bundle.task_timeline["completion"]["valid"], false);
        assert_eq!(bundle.task_timeline["completion"]["confirmed"], false);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn malformed_task_metadata_does_not_block_valid_v3_task_views() {
        let root =
            std::env::temp_dir().join(format!("vibehub-v3-malformed-task-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        super::super::initialize_v3(&root).unwrap();
        let valid = create_v3_task(
            &root,
            V3TaskCreateRequest {
                title: "Valid task".to_owned(),
                intent: "Keep valid V3 tasks visible".to_owned(),
                acceptance_criteria: vec!["View bundle remains available".to_owned()],
                workflow_profile: "standard".to_owned(),
                trigger_context: Default::default(),
                profile_override: None,
                initial_plan: Vec::new(),
                preflight: false,
            },
        )
        .unwrap();
        let legacy_task_id = "T-20260718191720-ffb8c89a";
        let legacy_dir = root.join(".vibehub/tasks").join(legacy_task_id);
        fs::create_dir(&legacy_dir).unwrap();
        fs::write(
            legacy_dir.join("task.yaml"),
            format!(
                "schema_version: 1\nkind: vibehub_task\ntask_id: {legacy_task_id}\ntitle: Legacy task\nmode: evidence_drive\nphase: align\nphase_status: active\n"
            ),
        )
        .unwrap();
        fs::write(
            root.join(".vibehub/tasks/current"),
            format!(
                "schema_version: 1\nkind: current_task_pointer\ntask_id: {legacy_task_id}\npath: .vibehub/tasks/{legacy_task_id}\nupdated_at: 2026-07-18T00:00:00Z\nupdated_by: vibehub\n"
            ),
        )
        .unwrap();

        let repository = V3ViewRepository::open(&root).unwrap();
        let bundle = repository.load_bundle(&valid.task_id).unwrap();

        assert_eq!(repository.current_task_id().unwrap(), valid.task_id);
        assert_eq!(
            bundle.project_overview["active_tasks"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let warning = bundle.project_overview["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|warning| warning["code"] == "V3_TASK_METADATA_INVALID")
            .expect("invalid legacy task warning");
        assert_eq!(warning["details"]["task_id"], legacy_task_id);
        assert!(warning["details"]["recommended_action"]
            .as_str()
            .unwrap()
            .contains("quarantine-task"));
        assert!(bundle.task_timeline["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning["code"] == "V3_TASK_METADATA_INVALID"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn plan_nodes_expose_sessions_by_node_id() {
        let project =
            std::env::temp_dir().join(format!("vibehub-v3-plan-session-{}", Uuid::new_v4()));
        fs::create_dir_all(project.join(".vibehub/tasks/task.test")).unwrap();
        fs::write(project.join(".vibehub/tasks/task.test/task.yaml"), "task_id: task.test\ntitle: Plan session\nintent: Map the active session\nphase: plan\nphase_status: active\nacceptance_criteria:\n- Session is visible\n").unwrap();
        let repository = V3ViewRepository::open(&project).unwrap();
        let project_id = repository.project_id();
        let app = V3ApplicationService::open(&project).unwrap();
        app.plan_add_node(PlanAddNodeCommand {
            identity: PlanCommandIdentity {
                project_id: project_id.clone(),
                task_id: "task.test".to_owned(),
                actor: "codex".to_owned(),
                expected_version: 0,
                idempotency_key: "plan.node.test".to_owned(),
            },
            node_id: "node.test".to_owned(),
            title: "Plan session".to_owned(),
            goal: "Map the active session".to_owned(),
            scope: Vec::new(),
            dependencies: Vec::new(),
            criterion_ids: Vec::new(),
        })
        .unwrap();
        app.session_open_with_context(
            &project_id,
            "task.test",
            "session.test",
            "codex",
            0,
            "session.node.test",
            None,
            Some("node.test".to_owned()),
            None,
        )
        .unwrap();

        let bundle = repository.load_bundle("task.test").unwrap();
        assert_eq!(
            bundle.plan_graph["nodes"][0]["session_ids"],
            json!(["session.test"])
        );
        assert_eq!(bundle.task_timeline["lanes"][0]["state"], "active");
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn session_working_directory_and_agent_results_are_projected_from_events() {
        let project = std::env::temp_dir().join(format!("vibehub-v3-project-{}", Uuid::new_v4()));
        let workspace =
            std::env::temp_dir().join(format!("vibehub-v3-workspace-{}", Uuid::new_v4()));
        fs::create_dir_all(project.join(".vibehub/tasks/task.test")).unwrap();
        fs::create_dir_all(&workspace).unwrap();
        fs::write(workspace.join("package.json"), r#"{"name":"workspace"}"#).unwrap();
        fs::write(project.join(".vibehub/tasks/task.test/task.yaml"), "task_id: task.test\ntitle: Agent result\nintent: Project result\nphase: implement\nphase_status: active\n").unwrap();
        let repository = V3ViewRepository::open(&project).unwrap();
        let project_id = repository.project_id();
        let app = V3ApplicationService::open(&project).unwrap();
        app.session_open_with_context(
            &project_id,
            "task.test",
            "session.test",
            "agent",
            0,
            "session.context",
            Some(workspace.to_string_lossy().into_owned()),
            None,
            None,
        )
        .unwrap();
        app.agent_result_record(&project_id, "task.test", "session.test", "agent", 1, "result.running", "result.test", None, json!({
            "kind":"evaluation", "request_source":"evaluation_instruction", "instruction":"Evaluate workspace", "status":"running", "summary":"Running", "body":null, "evaluation":null, "artifacts":[], "started_at":null, "completed_at":null
        })).unwrap();
        app.agent_result_record(&project_id, "task.test", "session.test", "agent", 2, "result.succeeded", "result.test", None, json!({
            "kind":"evaluation", "request_source":"evaluation_instruction", "instruction":"Evaluate workspace", "status":"succeeded", "summary":"Passed", "body":"All checks passed", "evaluation":{"target":"workspace","rubric":["exists"],"verdict":"passed","findings":[]}, "artifacts":[], "started_at":null, "completed_at":null
        })).unwrap();

        let bundle = repository.load_bundle("task.test").unwrap();
        assert_eq!(
            bundle.project_structure["workspace"]["source"],
            "session_working_directory"
        );
        assert_eq!(
            bundle.project_structure["workspace"]["root"]["native"],
            workspace.canonicalize().unwrap().to_string_lossy().as_ref()
        );
        assert_eq!(bundle.agent_results["state"], "review_required");
        assert_eq!(bundle.agent_results["review_required"], true);
        assert_eq!(bundle.agent_results["next_action"], "run_review");
        assert_eq!(bundle.agent_results["results"].as_array().unwrap().len(), 1);
        assert_eq!(bundle.agent_results["results"][0]["status"], "succeeded");
        assert_eq!(bundle.agent_results["results"][0]["summary"], "Passed");
        assert_eq!(
            bundle.agent_results["results"][0]["evaluation"]["verdict"],
            "passed"
        );
        fs::remove_dir_all(project).unwrap();
        fs::remove_dir_all(workspace).unwrap();
    }

    #[test]
    fn workspace_falls_back_and_exposes_reason_when_working_directory_missing() {
        let project = std::env::temp_dir().join(format!("vibehub-v3-fallback-{}", Uuid::new_v4()));
        fs::create_dir_all(project.join(".vibehub/tasks/task.test")).unwrap();
        fs::write(
            project.join(".vibehub/tasks/task.test/task.yaml"),
            "task_id: task.test\ntitle: Fallback\nintent: Workspace fallback\nphase: implement\nphase_status: active\n",
        )
        .unwrap();
        let repository = V3ViewRepository::open(&project).unwrap();
        let project_id = repository.project_id();
        let app = V3ApplicationService::open(&project).unwrap();
        let missing = project.join("does-not-exist");
        app.session_open_with_context(
            &project_id,
            "task.test",
            "session.test",
            "agent",
            0,
            "session.context",
            Some(missing.to_string_lossy().into_owned()),
            None,
            None,
        )
        .unwrap();

        let bundle = repository.load_bundle("task.test").unwrap();
        let workspace = &bundle.project_structure["workspace"];
        assert_eq!(workspace["source"], "project_root_fallback");
        assert!(workspace["fallback_reason"]
            .as_str()
            .expect("fallback reason present")
            .contains("no accessible working directory"));
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn forced_closure_archive_preserves_reason_and_unresolved_snapshot() {
        let project = std::env::temp_dir().join(format!("vibehub-v3-closure-{}", Uuid::new_v4()));
        fs::create_dir_all(project.join(".vibehub/tasks/task.test")).unwrap();
        fs::create_dir_all(project.join(".vibehub/tasks/current")).unwrap();
        let task = "task_id: task.test\ntitle: Forced closure\nintent: Preserve closure audit\nphase: execute\nphase_status: active\nacceptance_criteria:\n- Review passes\n";
        fs::write(project.join(".vibehub/tasks/task.test/task.yaml"), task).unwrap();
        fs::write(project.join(".vibehub/tasks/current/task.yaml"), task).unwrap();
        let repository = V3ViewRepository::open(&project).unwrap();
        let project_id = repository.project_id();
        let app = V3ApplicationService::open(&project).unwrap();
        app.plan_add_node(PlanAddNodeCommand {
            identity: PlanCommandIdentity {
                project_id: project_id.clone(),
                task_id: "task.test".to_owned(),
                actor: "codex".to_owned(),
                expected_version: 0,
                idempotency_key: "plan.node.blocked".to_owned(),
            },
            node_id: "node.blocked".to_owned(),
            title: "Blocked validation".to_owned(),
            goal: "Preserve the unresolved node in the archive".to_owned(),
            scope: Vec::new(),
            dependencies: Vec::new(),
            criterion_ids: Vec::new(),
        })
        .unwrap();
        app.plan_set_state(PlanSetStateCommand {
            identity: PlanCommandIdentity {
                project_id: project_id.clone(),
                task_id: "task.test".to_owned(),
                actor: "codex".to_owned(),
                expected_version: 1,
                idempotency_key: "plan.node.blocked.state".to_owned(),
            },
            node_id: "node.blocked".to_owned(),
            state: "blocked".to_owned(),
        })
        .unwrap();

        let before_closure = repository.load_bundle("task.test").unwrap();
        assert_eq!(
            before_closure.project_overview["active_tasks"][0]["state"],
            "blocked"
        );

        app.close_task_with_exceptions(
            &project_id,
            "task.test",
            "desktop-user",
            "ChenM0M",
            "desktop_ui",
            "User chose to archive before review",
            "closure.force.1",
        )
        .unwrap();

        let bundle = repository.load_bundle("task.test").unwrap();
        assert!(bundle.project_overview["active_tasks"]
            .as_array()
            .unwrap()
            .is_empty());
        let archived = &bundle.project_overview["archived_tasks"][0];
        assert_eq!(archived["state"], "closed_with_exceptions");
        assert_eq!(archived["closure"]["method"], "with_exceptions");
        assert_eq!(archived["closure"]["confirmed_by"], "ChenM0M");
        assert_eq!(archived["closure"]["channel"], "desktop_ui");
        assert_eq!(
            archived["closure"]["reason"],
            "User chose to archive before review"
        );
        assert_eq!(
            archived["closure"]["unresolved_items"],
            json!(["criterion.task.test.c01"])
        );
        assert_eq!(
            archived["closure"]["criteria_snapshot"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn closed_session_workspace_falls_back_to_project_root_with_reason() {
        let project = std::env::temp_dir().join(format!("vibehub-v3-project-{}", Uuid::new_v4()));
        let workspace =
            std::env::temp_dir().join(format!("vibehub-v3-workspace-{}", Uuid::new_v4()));
        fs::create_dir_all(project.join(".vibehub/tasks/task.test")).unwrap();
        fs::create_dir_all(&workspace).unwrap();
        fs::write(project.join(".vibehub/tasks/task.test/task.yaml"), "task_id: task.test\ntitle: Closed session\nintent: Fall back after close\nphase: implement\nphase_status: active\n").unwrap();
        let repository = V3ViewRepository::open(&project).unwrap();
        let project_id = repository.project_id();
        let app = V3ApplicationService::open(&project).unwrap();
        app.session_open_with_context(
            &project_id,
            "task.test",
            "session.test",
            "agent",
            0,
            "session.open",
            Some(workspace.to_string_lossy().into_owned()),
            None,
            None,
        )
        .unwrap();
        app.session_close(
            &project_id,
            "task.test",
            "session.test",
            "agent",
            1,
            "session.close",
        )
        .unwrap();

        let bundle = repository.load_bundle("task.test").unwrap();
        assert_eq!(
            bundle.project_structure["workspace"]["source"],
            "project_root_fallback"
        );
        assert_eq!(
            bundle.project_structure["workspace"]["session_id"],
            Value::Null
        );
        assert_eq!(
            bundle.project_structure["workspace"]["root"]["native"],
            project.canonicalize().unwrap().to_string_lossy().as_ref()
        );
        assert_eq!(
            bundle.project_structure["workspace"]["fallback_reason"],
            "no active session or worktree context was recorded"
        );

        fs::remove_dir_all(project).unwrap();
        fs::remove_dir_all(workspace).unwrap();
    }

    #[test]
    fn architecture_file_counts_use_git_visible_files() {
        let project = std::env::temp_dir().join(format!("vibehub-v3-counts-{}", Uuid::new_v4()));
        fs::create_dir_all(project.join(".vibehub/tasks/task.test")).unwrap();
        fs::create_dir_all(project.join("src")).unwrap();
        fs::create_dir_all(project.join("node_modules/dependency")).unwrap();
        fs::write(project.join(".vibehub/tasks/task.test/task.yaml"), "task_id: task.test\ntitle: Count files\nintent: Ignore heavy directories\nphase: implement\nphase_status: active\n").unwrap();
        fs::write(project.join("package.json"), r#"{"name":"count-files"}"#).unwrap();
        fs::write(project.join(".gitignore"), "/node_modules/\n").unwrap();
        fs::write(project.join("src/index.ts"), "export const value = 1;").unwrap();
        fs::write(
            project.join("node_modules/dependency/index.js"),
            "module.exports = 1;",
        )
        .unwrap();
        silent_command("git")
            .arg("-C")
            .arg(&project)
            .arg("init")
            .output()
            .unwrap();

        let visible = visible_project_files(&project);
        assert!(visible.iter().any(|path| path == "src/index.ts"));
        assert!(visible
            .iter()
            .all(|path| !path.starts_with("node_modules/")));

        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn persisted_json_index_drives_stable_evidenced_architecture_views() {
        let project =
            std::env::temp_dir().join(format!("vibehub-v3-json-index-{}", Uuid::new_v4()));
        fs::create_dir_all(project.join(".vibehub/tasks/task.test")).unwrap();
        fs::create_dir_all(project.join("src/feature")).unwrap();
        fs::create_dir_all(project.join("crates/vibehub-core/src")).unwrap();
        fs::create_dir_all(project.join("crates/vibehub-cli/src")).unwrap();
        fs::create_dir_all(project.join("src-tauri/src")).unwrap();
        fs::write(
            project.join(".vibehub/tasks/task.test/task.yaml"),
            "task_id: task.test\ntitle: JSON architecture index\nintent: Parse a persisted project model\nphase: implement\nphase_status: active\n",
        )
        .unwrap();
        fs::write(
            project.join("Cargo.toml"),
            "[workspace]\nmembers=[\"crates/vibehub-core\",\"crates/vibehub-cli\",\"src-tauri\"]\nresolver=\"2\"\n",
        )
        .unwrap();
        fs::write(project.join("package.json"), r#"{"name":"frontend-app"}"#).unwrap();
        fs::write(
            project.join("crates/vibehub-core/Cargo.toml"),
            "[package]\nname=\"vibehub-core\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        fs::write(
            project.join("crates/vibehub-cli/Cargo.toml"),
            "[package]\nname=\"vibehub-cli\"\nversion=\"0.1.0\"\n[dependencies]\nvibehub-core={path=\"../vibehub-core\"}\n",
        )
        .unwrap();
        fs::write(
            project.join("src-tauri/Cargo.toml"),
            "[package]\nname=\"desktop-host\"\nversion=\"0.1.0\"\n[dependencies]\ncore={package=\"vibehub-core\",path=\"../crates/vibehub-core\"}\nadapters={package=\"vibehub-cli\",path=\"../crates/vibehub-cli\"}\n",
        )
        .unwrap();
        fs::write(
            project.join("src/app.ts"),
            "import { tool } from './feature/tool';\nexport { tool };\n",
        )
        .unwrap();
        fs::write(
            project.join("src/feature/tool.ts"),
            "export const tool = true;\n",
        )
        .unwrap();
        fs::write(
            project.join("crates/vibehub-core/src/lib.rs"),
            "pub struct Core;\n",
        )
        .unwrap();
        fs::write(
            project.join("crates/vibehub-cli/src/lib.rs"),
            "use vibehub_core::Core;\npub fn run(_: Core) {}\n",
        )
        .unwrap();
        fs::write(
            project.join("src-tauri/src/main.rs"),
            "use vibehub_core::Core;\nfn main() { let _ = Core; }\n",
        )
        .unwrap();
        silent_command("git")
            .arg("-C")
            .arg(&project)
            .arg("init")
            .output()
            .unwrap();

        let repository = V3ViewRepository::open(&project).unwrap();
        let first = repository.load_bundle("task.test").unwrap();
        let snapshot_path = project.join(crate::v3::project_intelligence::PROJECT_MODEL_INDEX_PATH);
        let snapshot: ProjectModelSnapshot =
            serde_json::from_slice(&fs::read(&snapshot_path).unwrap()).unwrap();
        assert!(snapshot
            .analyzer_findings
            .iter()
            .any(|finding| finding.kind == "package_manifest"));
        assert!(snapshot
            .analyzer_findings
            .iter()
            .any(|finding| finding.kind == "dependency"));

        let nodes = first.project_structure["architecture_nodes"]
            .as_array()
            .unwrap();
        let edges = first.project_structure["architecture_edges"]
            .as_array()
            .unwrap();
        let ids = nodes
            .iter()
            .filter_map(|node| node["node_id"].as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), nodes.len());
        assert!(edges.iter().all(|edge| {
            let from = edge["from_node_id"].as_str().unwrap();
            let to = edge["to_node_id"].as_str().unwrap();
            from != to && ids.contains(from) && ids.contains(to)
        }));
        assert_eq!(
            first.project_overview["architecture"]["modules"],
            nodes
                .iter()
                .filter(|node| node["kind"] != "workspace")
                .count()
        );
        assert_eq!(
            first.project_overview["architecture"]["relationships"],
            edges.len()
        );
        assert!(first.project_overview["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|warning| warning["code"] != "V3_ARCHITECTURE_INDEX_PENDING"));
        assert!(first.project_structure["evidence_refs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reference| reference["locator"]
                == format!(
                    "file:{}",
                    crate::v3::project_intelligence::PROJECT_MODEL_INDEX_PATH
                )));
        assert!(nodes
            .iter()
            .filter(|node| node["kind"] == "package")
            .all(
                |node| node["evidence_refs"][0]["locator"]
                    .as_str()
                    .is_some_and(|locator| locator.ends_with("Cargo.toml")
                        || locator.ends_with("package.json"))
            ));

        let stable_ids = nodes
            .iter()
            .filter(|node| node["kind"] == "package")
            .filter_map(|node| {
                Some((
                    node["name"].as_str()?.to_owned(),
                    node["node_id"].as_str()?.to_owned(),
                ))
            })
            .collect::<BTreeMap<_, _>>();
        fs::create_dir_all(project.join("aaa/src")).unwrap();
        fs::write(
            project.join("aaa/Cargo.toml"),
            "[package]\nname=\"unrelated\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        fs::write(project.join("aaa/src/lib.rs"), "pub struct Unrelated;\n").unwrap();
        let second = repository.load_bundle("task.test").unwrap();
        let second_ids = second.project_structure["architecture_nodes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|node| node["kind"] == "package")
            .filter_map(|node| {
                Some((
                    node["name"].as_str()?.to_owned(),
                    node["node_id"].as_str()?.to_owned(),
                ))
            })
            .collect::<BTreeMap<_, _>>();
        for (name, id) in stable_ids {
            assert_eq!(second_ids.get(&name), Some(&id));
        }
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn project_overview_lists_every_v3_task_with_its_own_session_count() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-tasks-{}", Uuid::new_v4()));
        for (task_id, title, phase_status) in [
            ("task.completed", "Completed task", "active"),
            ("task.one", "First task", "active"),
            ("task.two", "Second task", "active"),
        ] {
            let task_dir = root.join(".vibehub/tasks").join(task_id);
            fs::create_dir_all(&task_dir).unwrap();
            fs::write(
                task_dir.join("task.yaml"),
                format!("task_id: {task_id}\ntitle: {title}\nintent: Verify task enumeration\nphase: implement\nphase_status: {phase_status}\nacceptance_criteria:\n- Visible in overview\ndependencies: []\n"),
            )
            .unwrap();
        }
        let repository = V3ViewRepository::open(&root).unwrap();
        let project_id = repository.project_id();
        V3ApplicationService::open(&root)
            .unwrap()
            .session_open(
                &project_id,
                "task.two",
                "session.two",
                "agent",
                0,
                "session.two.open",
            )
            .unwrap();
        let store = V3EventStore::open(&root).unwrap();
        store
            .append(EventDraft {
                event_type: "task.closed_with_exceptions".to_owned(),
                aggregate_id: "task.completed".to_owned(),
                expected_version: 0,
                idempotency_key: "overview.task.completed.close".to_owned(),
                project_id: ProjectId(project_id.clone()),
                task_id: TaskId("task.completed".to_owned()),
                node_id: None,
                session_id: None,
                worktree_id: None,
                lease_id: None,
                operation_id: None,
                actor: "fixture".to_owned(),
                evidence_grade: EvidenceGrade::HardObserved,
                occurred_at: Some("2026-07-14T00:00:00.000Z".to_owned()),
                commit_sha: None,
                payload: json!({"reason": "fixture terminal task"}),
            })
            .unwrap();
        store.rebuild_projection(&project_id).unwrap();

        let bundle = repository.load_bundle("task.one").unwrap();
        let tasks = bundle.project_overview["active_tasks"].as_array().unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0]["task_id"], "task.one");
        assert_eq!(tasks[0]["active_sessions"], 0);
        assert_eq!(tasks[1]["task_id"], "task.two");
        assert_eq!(tasks[1]["active_sessions"], 1);
        let archived = bundle.project_overview["archived_tasks"]
            .as_array()
            .unwrap();
        assert_eq!(archived.len(), 1);
        assert_eq!(archived[0]["task_id"], "task.completed");
        assert_eq!(archived[0]["state"], "closed_with_exceptions");
        assert_eq!(archived[0]["criteria"][0]["status"], "accepted");
        assert_eq!(archived[0]["plan"]["total"], 0);
        assert_eq!(archived[0]["result"]["status"], "not_executed");
        assert_eq!(archived[0]["completion"]["confirmed"], false);
        assert_eq!(archived[0]["source"], "v3_projection");
        assert_eq!(
            bundle.project_structure["page"]["returned"],
            bundle.project_structure["page"]["total_estimate"]
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn archived_tasks_are_ordered_by_terminal_at_newest_first() {
        let root =
            std::env::temp_dir().join(format!("vibehub-v3-archive-order-{}", Uuid::new_v4()));
        for task_id in [
            "task.alpha",
            "task.beta",
            "task.gamma",
            "task.delta",
            "task.active",
        ] {
            let task_dir = root.join(".vibehub/tasks").join(task_id);
            fs::create_dir_all(&task_dir).unwrap();
            let phase_status = match task_id {
                "task.active" => "active",
                // An eventless legacy cancellation remains terminal, but has
                // no terminal timestamp; it exercises the missing-value sort
                // bucket without treating legacy completion as success.
                "task.delta" => "cancelled",
                _ => "active",
            };
            fs::write(
                task_dir.join("task.yaml"),
                format!("task_id: {task_id}\ntitle: {task_id}\nintent: Verify archive ordering\nphase: implement\nphase_status: {phase_status}\nacceptance_criteria:\n- Visible in archive\ndependencies: []\n"),
            )
            .unwrap();
        }
        let repository = V3ViewRepository::open(&root).unwrap();
        let project_id = repository.project_id();
        let store = V3EventStore::open(&root).unwrap();
        for (task_id, occurred_at) in [
            ("task.alpha", "2026-01-01T00:00:00.000Z"),
            ("task.beta", "2026-03-03T00:00:00.000Z"),
            ("task.gamma", "2026-02-02T00:00:00.000Z"),
        ] {
            store
                .append(EventDraft {
                    event_type: "task.closed_with_exceptions".to_owned(),
                    aggregate_id: task_id.to_owned(),
                    expected_version: 0,
                    idempotency_key: format!("archive.order.close.{task_id}"),
                    project_id: ProjectId(project_id.clone()),
                    task_id: TaskId(task_id.to_owned()),
                    node_id: None,
                    session_id: None,
                    worktree_id: None,
                    lease_id: None,
                    operation_id: None,
                    actor: "agent".to_owned(),
                    evidence_grade: EvidenceGrade::AgentReported,
                    occurred_at: Some(occurred_at.to_owned()),
                    commit_sha: None,
                    payload: json!({"reason": "seed archive ordering"}),
                })
                .unwrap();
        }
        store.rebuild_projection(&project_id).unwrap();

        let bundle = repository.load_bundle("task.active").unwrap();
        let archived = bundle.project_overview["archived_tasks"]
            .as_array()
            .unwrap();
        let order: Vec<&str> = archived
            .iter()
            .map(|task| task["task_id"].as_str().unwrap())
            .collect();
        assert_eq!(
            order,
            vec!["task.beta", "task.gamma", "task.alpha", "task.delta"],
            "archived tasks must be ordered by terminal_at descending with missing values last"
        );
        assert_eq!(archived[0]["terminal_at"], "2026-03-03T00:00:00.000Z");
        assert_eq!(archived[2]["terminal_at"], "2026-01-01T00:00:00.000Z");
        assert!(archived[3]["terminal_at"].is_null());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn archive_ordering_breaks_terminal_at_ties_by_task_id() {
        let mut archived = vec![
            json!({"task_id": "task.zeta", "terminal_at": "2026-05-05T00:00:00.000Z"}),
            json!({"task_id": "task.alpha", "terminal_at": "2026-05-05T00:00:00.000Z"}),
            json!({"task_id": "task.no-terminal"}),
            json!({"task_id": "task.newest", "terminal_at": "2026-06-06T00:00:00.000Z"}),
        ];
        sort_archived_tasks_newest_first(&mut archived);
        let order: Vec<&str> = archived
            .iter()
            .map(|task| task["task_id"].as_str().unwrap())
            .collect();
        assert_eq!(
            order,
            vec!["task.newest", "task.alpha", "task.zeta", "task.no-terminal"]
        );
    }

    #[test]
    fn task_list_is_pointer_independent_and_exposes_rates_and_terminal_sorting() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-task-list-{}", Uuid::new_v4()));
        for (task_id, phase_status, criteria) in [
            ("task.active", "active", "- First\n- Second"),
            ("task.alpha", "completed", "- Archived"),
            ("task.beta", "completed", "- Archived"),
            ("task.gamma", "completed", "- Archived"),
            // A legacy task file may carry an explicit terminal cancellation
            // without a V3 terminal event. Keep it terminal while exercising
            // the documented missing-terminal_at sort bucket.
            ("task.delta", "cancelled", "- Archived"),
        ] {
            let task_dir = root.join(".vibehub/tasks").join(task_id);
            fs::create_dir_all(&task_dir).unwrap();
            fs::write(
                task_dir.join("task.yaml"),
                format!(
                    "task_id: {task_id}\ntitle: {task_id}\nintent: Verify task list\nphase: implement\nphase_status: {phase_status}\nacceptance_criteria:\n{criteria}\n"
                ),
            )
            .unwrap();
        }
        let repository = V3ViewRepository::open(&root).unwrap();
        let project_id = repository.project_id();
        let store = V3EventStore::open(&root).unwrap();
        store.rebuild_projection(&project_id).unwrap();

        for (task_id, occurred_at) in [
            ("task.alpha", "2026-01-01T00:00:00.000Z"),
            ("task.beta", "2026-03-03T00:00:00.000Z"),
            ("task.gamma", "2026-02-02T00:00:00.000Z"),
        ] {
            store
                .append(EventDraft {
                    event_type: "task.closed_with_exceptions".to_owned(),
                    aggregate_id: task_id.to_owned(),
                    expected_version: 0,
                    idempotency_key: format!("task-list.close.{task_id}"),
                    project_id: ProjectId(project_id.clone()),
                    task_id: TaskId(task_id.to_owned()),
                    node_id: None,
                    session_id: None,
                    worktree_id: None,
                    lease_id: None,
                    operation_id: None,
                    actor: "fixture".to_owned(),
                    evidence_grade: EvidenceGrade::HardObserved,
                    occurred_at: Some(occurred_at.to_owned()),
                    commit_sha: None,
                    payload: json!({"reason": "fixture"}),
                })
                .unwrap();
        }
        let criterion_id = canonical_criterion_id("task.active", 0);
        store
            .append(EventDraft {
                event_type: "criterion.passed".to_owned(),
                aggregate_id: "task.active".to_owned(),
                expected_version: 0,
                idempotency_key: "task-list.criterion.passed".to_owned(),
                project_id: ProjectId(project_id.clone()),
                task_id: TaskId("task.active".to_owned()),
                node_id: None,
                session_id: None,
                worktree_id: None,
                lease_id: None,
                operation_id: None,
                actor: "fixture".to_owned(),
                evidence_grade: EvidenceGrade::HardObserved,
                occurred_at: None,
                commit_sha: None,
                payload: json!({
                    "criterion_id": criterion_id,
                    "title": "First",
                    "required": true,
                    "evidence_refs": ["test:task-list"]
                }),
            })
            .unwrap();
        store.rebuild_projection(&project_id).unwrap();

        // An invalid current pointer must not prevent the explicit index query.
        fs::write(
            root.join(".vibehub/tasks/current"),
            "schema_version: 1\nkind: current_task_pointer\ntask_id: task.missing\npath: .vibehub/tasks/task.missing\n",
        )
        .unwrap();

        let active_only = repository.task_list(false).unwrap();
        assert_eq!(active_only["tasks"].as_array().unwrap().len(), 1);
        assert_eq!(active_only["tasks"][0]["task_id"], "task.active");
        assert_eq!(active_only["omitted_archived_count"], 4);
        let active = &active_only["tasks"][0];
        assert_eq!(active["terminal_at"], Value::Null);
        assert_eq!(active["criterion_pass_rate"]["passed"], 1);
        assert_eq!(active["criterion_pass_rate"]["total"], 2);
        assert_eq!(active["criterion_pass_rate"]["ratio"], 0.5);
        assert!(active["required_node_completion_rate"]["ratio"].is_null());

        let all = repository.task_list(true).unwrap();
        let order = all["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|task| task["task_id"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            order,
            vec![
                "task.active",
                "task.beta",
                "task.gamma",
                "task.alpha",
                "task.delta"
            ]
        );
        assert_eq!(all["archived_count"], 4);
        assert_eq!(all["tasks"][1]["terminal_at"], "2026-03-03T00:00:00.000Z");
        assert!(all["tasks"][4]["terminal_at"].is_null());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn commit_tasks_supports_short_full_unknown_and_historical_missing_head_facts() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-commit-tasks-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        for task_id in ["task.git.a", "task.git.b", "task.git.history"] {
            let task_dir = root.join(".vibehub/tasks").join(task_id);
            fs::create_dir_all(&task_dir).unwrap();
            fs::write(
                task_dir.join("task.yaml"),
                format!(
                    "task_id: {task_id}\ntitle: {task_id}\nintent: Verify Git traceability\nphase: implement\nphase_status: active\n"
                ),
            )
            .unwrap();
        }
        let init = silent_command("git")
            .arg("-C")
            .arg(&root)
            .args(["init", "-q"])
            .output()
            .unwrap();
        assert!(init.status.success());
        for (key, value) in [
            ("user.name", "VibeHub fixture"),
            ("user.email", "fixture@example.invalid"),
        ] {
            assert!(silent_command("git")
                .arg("-C")
                .arg(&root)
                .args(["config", key, value])
                .status()
                .unwrap()
                .success());
        }
        fs::write(root.join("trace.txt"), "base\n").unwrap();
        assert!(silent_command("git")
            .arg("-C")
            .arg(&root)
            .args(["add", "."])
            .status()
            .unwrap()
            .success());
        assert!(silent_command("git")
            .arg("-C")
            .arg(&root)
            .args(["commit", "-qm", "base"])
            .status()
            .unwrap()
            .success());
        let base = String::from_utf8(
            silent_command("git")
                .arg("-C")
                .arg(&root)
                .args(["rev-parse", "HEAD"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_owned();
        fs::write(root.join("trace.txt"), "middle\n").unwrap();
        assert!(silent_command("git")
            .arg("-C")
            .arg(&root)
            .args(["add", "."])
            .status()
            .unwrap()
            .success());
        assert!(silent_command("git")
            .arg("-C")
            .arg(&root)
            .args(["commit", "-qm", "middle"])
            .status()
            .unwrap()
            .success());
        let middle = String::from_utf8(
            silent_command("git")
                .arg("-C")
                .arg(&root)
                .args(["rev-parse", "HEAD"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_owned();
        fs::write(root.join("trace.txt"), "close\n").unwrap();
        assert!(silent_command("git")
            .arg("-C")
            .arg(&root)
            .args(["add", "."])
            .status()
            .unwrap()
            .success());
        assert!(silent_command("git")
            .arg("-C")
            .arg(&root)
            .args(["commit", "-qm", "close"])
            .status()
            .unwrap()
            .success());
        let close = String::from_utf8(
            silent_command("git")
                .arg("-C")
                .arg(&root)
                .args(["rev-parse", "HEAD"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_owned();

        let repository = V3ViewRepository::open(&root).unwrap();
        let project_id = repository.project_id();
        let store = V3EventStore::open(&root).unwrap();
        for (task_id, session_id, with_heads) in [
            ("task.git.a", "session.git.a", true),
            ("task.git.b", "session.git.b", true),
            ("task.git.history", "session.git.history", false),
        ] {
            for (event_type, expected_version, key, head) in [
                (
                    "session.opened",
                    0,
                    "open",
                    with_heads.then_some(base.as_str()),
                ),
                (
                    "session.closed",
                    1,
                    "close",
                    with_heads.then_some(close.as_str()),
                ),
            ] {
                store
                    .append(EventDraft {
                        event_type: event_type.to_owned(),
                        aggregate_id: session_id.to_owned(),
                        expected_version,
                        idempotency_key: format!("git-fixture.{task_id}.{key}"),
                        project_id: ProjectId(project_id.clone()),
                        task_id: TaskId(task_id.to_owned()),
                        node_id: None,
                        session_id: Some(SessionId(session_id.to_owned())),
                        worktree_id: None,
                        lease_id: None,
                        operation_id: None,
                        actor: "fixture".to_owned(),
                        evidence_grade: EvidenceGrade::HardObserved,
                        occurred_at: None,
                        commit_sha: None,
                        payload: head
                            .map(|value| json!({"git_head_sha": value}))
                            .unwrap_or_else(|| json!({})),
                    })
                    .unwrap();
            }
        }
        store.rebuild_projection(&project_id).unwrap();

        let short = &middle[..7];
        let reverse = repository.commit_tasks(short).unwrap();
        assert_eq!(reverse["resolved_commit_hash"], middle);
        assert_eq!(reverse["match"], "associated");
        assert_eq!(reverse["task_count"], 2);
        assert_eq!(
            reverse["tasks"]
                .as_array()
                .unwrap()
                .iter()
                .map(|task| task["task_id"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["task.git.a", "task.git.b"]
        );
        let full = repository.commit_tasks(&middle).unwrap();
        assert_eq!(full["task_count"], 2);
        let unknown = repository.commit_tasks("deadbeef").unwrap();
        assert_eq!(unknown["match"], "none");
        assert!(unknown["resolved_commit_hash"].is_null());
        assert_eq!(unknown["tasks"], json!([]));

        let complete = repository.task_commits("task.git.a").unwrap();
        assert_eq!(complete["git_evidence"]["status"], "complete");
        assert_eq!(complete["sessions"][0]["open_git_head"], base);
        assert_eq!(complete["sessions"][0]["close_git_head"], close);
        assert_eq!(complete["sessions"][0]["git_trace_status"], "complete");
        let historical = repository.task_commits("task.git.history").unwrap();
        assert_eq!(historical["git_evidence"]["status"], "missing");
        assert_eq!(historical["sessions"][0]["open_git_head"], Value::Null);
        assert_eq!(historical["sessions"][0]["has_git_evidence"], false);
        assert_eq!(
            historical["git_evidence"]["historical_gaps"]
                .as_array()
                .unwrap()
                .len(),
            1
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn completed_pointer_falls_back_to_a_non_terminal_task_and_is_not_active() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-current-{}", Uuid::new_v4()));
        let tasks_root = root.join(".vibehub/tasks");
        for (task_id, phase_status) in [("task.completed", "completed"), ("task.active", "active")]
        {
            let task_dir = tasks_root.join(task_id);
            fs::create_dir_all(&task_dir).unwrap();
            fs::write(
                task_dir.join("task.yaml"),
                format!("task_id: {task_id}\ntitle: {task_id}\nintent: Test current selection\nphase: implement\nphase_status: {phase_status}\n"),
            )
            .unwrap();
        }
        fs::write(
            tasks_root.join("current"),
            "schema_version: 1\nkind: current_task_pointer\ntask_id: task.completed\npath: .vibehub/tasks/task.completed\nupdated_at: 2026-07-14T00:00:00Z\nupdated_by: vibehub\n",
        )
        .unwrap();

        let repository = V3ViewRepository::open(&root).unwrap();
        let project_id = repository.project_id();
        let store = V3EventStore::open(&root).unwrap();
        store
            .append(EventDraft {
                event_type: "task.closed_with_exceptions".to_owned(),
                aggregate_id: "task.completed".to_owned(),
                expected_version: 0,
                idempotency_key: "current.task.completed.close".to_owned(),
                project_id: ProjectId(project_id.clone()),
                task_id: TaskId("task.completed".to_owned()),
                node_id: None,
                session_id: None,
                worktree_id: None,
                lease_id: None,
                operation_id: None,
                actor: "fixture".to_owned(),
                evidence_grade: EvidenceGrade::HardObserved,
                occurred_at: Some("2026-07-14T00:00:00.000Z".to_owned()),
                commit_sha: None,
                payload: json!({"reason": "fixture terminal task"}),
            })
            .unwrap();
        store.rebuild_projection(&project_id).unwrap();
        assert_eq!(repository.current_task_id().unwrap(), "task.active");
        let bundle = repository.load_bundle("task.active").unwrap();
        assert_eq!(bundle.project_overview["current_task_id"], "task.active");
        let active_tasks = bundle.project_overview["active_tasks"].as_array().unwrap();
        assert_eq!(active_tasks.len(), 1);
        assert_eq!(active_tasks[0]["task_id"], "task.active");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_historical_agent_result_status_is_projected_as_failed() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-result-view-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub/tasks/task.test")).unwrap();
        fs::write(
            root.join(".vibehub/tasks/task.test/task.yaml"),
            "task_id: task.test\ntitle: Result compatibility\nintent: Render historical events\nphase: implement\nphase_status: active\n",
        )
        .unwrap();
        let repository = V3ViewRepository::open(&root).unwrap();
        let project_id = repository.project_id();
        let store = V3EventStore::open(&root).unwrap();
        store
            .append(EventDraft {
                event_type: "agent.result_recorded".to_owned(),
                aggregate_id: "session.test".to_owned(),
                expected_version: 0,
                idempotency_key: "result.blocked".to_owned(),
                project_id: ProjectId(project_id),
                task_id: TaskId("task.test".to_owned()),
                node_id: None,
                session_id: Some(SessionId("session.test".to_owned())),
                worktree_id: None,
                lease_id: None,
                operation_id: None,
                actor: "legacy-agent".to_owned(),
                evidence_grade: EvidenceGrade::AgentReported,
                occurred_at: None,
                commit_sha: None,
                payload: json!({
                    "result_id": "result.test",
                    "kind": "execution",
                    "request_source": "user_request",
                    "instruction": "Run",
                    "status": "blocked",
                    "summary": "Blocked"
                }),
            })
            .unwrap();

        let bundle = repository.load_bundle("task.test").unwrap();
        assert_eq!(bundle.agent_results["state"], "failed");
        assert_eq!(bundle.agent_results["results"][0]["status"], "failed");
        assert_eq!(
            bundle.agent_results["warnings"][0]["code"],
            "V3_AGENT_RESULT_STATUS_INVALID"
        );
        assert_eq!(
            bundle.agent_results["warnings"][0]["details"]["original_status"],
            "blocked"
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn malformed_historical_agent_result_details_are_normalized() {
        let root =
            std::env::temp_dir().join(format!("vibehub-v3-result-details-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub/tasks/task.test")).unwrap();
        fs::write(
            root.join(".vibehub/tasks/task.test/task.yaml"),
            "task_id: task.test\ntitle: Result compatibility\nintent: Render historical events\nphase: implement\nphase_status: active\n",
        )
        .unwrap();
        let repository = V3ViewRepository::open(&root).unwrap();
        let project_id = repository.project_id();
        V3EventStore::open(&root)
            .unwrap()
            .append(EventDraft {
                event_type: "agent.result_recorded".to_owned(),
                aggregate_id: "session.test".to_owned(),
                expected_version: 0,
                idempotency_key: "result.malformed".to_owned(),
                project_id: ProjectId(project_id),
                task_id: TaskId("task.test".to_owned()),
                node_id: None,
                session_id: Some(SessionId("session.test".to_owned())),
                worktree_id: None,
                lease_id: None,
                operation_id: None,
                actor: "legacy-agent".to_owned(),
                evidence_grade: EvidenceGrade::AgentReported,
                occurred_at: None,
                commit_sha: None,
                payload: json!({
                    "result_id": "result.test",
                    "kind": "evaluation",
                    "request_source": "evaluation_instruction",
                    "instruction": "Review",
                    "status": "succeeded",
                    "summary": "Reviewed",
                    "evaluation": {"target": "workspace", "verdict": "needs_action", "findings": ["missing rubric"]},
                    "artifacts": ["src/example.rs"],
                    "evidence_refs": ["evt.legacy"]
                }),
            })
            .unwrap();

        let bundle = repository.load_bundle("task.test").unwrap();
        let result = &bundle.agent_results["results"][0];
        assert_eq!(result["evaluation"]["rubric"], json!([]));
        assert_eq!(result["evaluation"]["verdict"], "inconclusive");
        assert_eq!(
            result["evaluation"]["findings"][0]["title"],
            "missing rubric"
        );
        assert_eq!(result["artifacts"][0]["label"], "src/example.rs");
        assert_eq!(result["evidence_refs"][0]["evidence_id"], "evt.legacy");
        assert!(bundle.agent_results["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning["code"] == "V3_AGENT_RESULT_DETAILS_NORMALIZED"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn successful_agent_result_evidence_does_not_create_a_false_blocker() {
        let root =
            std::env::temp_dir().join(format!("vibehub-v3-blocker-signal-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub/tasks/task.test")).unwrap();
        fs::write(
            root.join(".vibehub/tasks/task.test/task.yaml"),
            "task_id: task.test\ntitle: Blocker signal\nintent: Keep evidence semantic\nphase: implement\nphase_status: active\nacceptance_criteria:\n- Evidence remains factual\n",
        )
        .unwrap();
        let repository = V3ViewRepository::open(&root).unwrap();
        let project_id = repository.project_id();
        let app = V3ApplicationService::open(&root).unwrap();
        app.session_open(
            &project_id,
            "task.test",
            "session.test",
            "codex",
            0,
            "session.open",
        )
        .unwrap();
        app.agent_result_record(
            &project_id,
            "task.test",
            "session.test",
            "codex",
            1,
            "result.success",
            "result.success",
            None,
            json!({
                "kind": "execution",
                "request_source": "user_request",
                "instruction": "Read evidence",
                "status": "succeeded",
                "summary": "M9 projection was inspected",
                "body": "The M9 accessibility evidence belongs to another task.",
                "evidence_refs": [{"evidence_id":"file:m9-accessibility","kind":"file","grade":"hard_observed","label_key":"test","locator":"file:m9-accessibility"}]
            }),
        )
        .unwrap();

        let bundle = repository.load_bundle("task.test").unwrap();
        assert_eq!(bundle.task_timeline["blocker_details"], json!([]));
        assert_eq!(
            bundle.project_overview["active_tasks"][0]["blocker_details"],
            json!([])
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resolved_criterion_does_not_project_historical_blockers() {
        let root =
            std::env::temp_dir().join(format!("vibehub-v3-resolved-blocker-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub/tasks/task.test")).unwrap();
        fs::write(
            root.join(".vibehub/tasks/task.test/task.yaml"),
            "task_id: task.test\ntitle: Resolved blocker\nintent: Keep history without stale status\nphase: implement\nphase_status: active\nacceptance_criteria:\n- Native flow passes\n",
        )
        .unwrap();
        let repository = V3ViewRepository::open(&root).unwrap();
        let project_id = repository.project_id();
        let app = V3ApplicationService::open(&root).unwrap();
        app.lifecycle_command(super::super::lifecycle::command(
            "criterion.blocked",
            &project_id,
            "task.test",
            0,
            "criterion.blocked",
            json!({"criterion_id":"criterion.task.test.c01","reviewer":"reviewer","evidence_refs":["evt.blocked"],"reason":"macOS Accessibility permission was unavailable"}),
        ))
        .unwrap();
        let blocked_bundle = repository.load_bundle("task.test").unwrap();
        let blocked = &blocked_bundle.task_timeline["blocker_details"][0];
        assert_eq!(blocked["source_type"], "criterion");
        assert!(blocked["summary"]
            .as_str()
            .is_some_and(|summary| summary.contains("Native flow passes")));
        assert!(blocked["why_blocked"]
            .as_str()
            .is_some_and(|reason| reason.contains("permission")));
        assert!(blocked["repair_actions"][0]["instructions"]
            .as_str()
            .is_some_and(|action| action.contains("criterion_review")));
        assert_eq!(blocked["provenance"]["status"], "degraded");
        assert!(blocked["provenance"]["unknown_fields"]
            .as_array()
            .is_some_and(|fields| fields.iter().any(|field| field == "missing_facts")));
        app.lifecycle_command(super::super::lifecycle::command(
            "criterion.passed",
            &project_id,
            "task.test",
            1,
            "criterion.passed",
            json!({"criterion_id":"criterion.task.test.c01","reviewer":"reviewer","evidence_refs":["test:native-flow"]}),
        ))
        .unwrap();

        let bundle = repository.load_bundle("task.test").unwrap();
        assert_eq!(bundle.task_timeline["blocker_details"], json!([]));
        assert_eq!(
            bundle.project_overview["active_tasks"][0]["blocker_details"],
            json!([])
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn eventless_tasks_have_no_synthetic_plan_or_execution_counts() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-views-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub/tasks/task.test")).unwrap();
        fs::write(
            root.join(".vibehub/tasks/task.test/task.yaml"),
            "task_id: task.test\ntitle: Historical task\nintent: Remain read only\nphase: implement\nphase_status: active\n",
        )
        .unwrap();
        let bundle = V3ViewRepository::open(&root)
            .unwrap()
            .load_bundle("task.test")
            .unwrap();
        assert_eq!(bundle.plan_graph["plan_version"], 0);
        assert_eq!(bundle.plan_graph["nodes"], json!([]));
        assert_eq!(bundle.plan_graph["scheduling_edges"], json!([]));
        assert_eq!(bundle.plan_graph["execution"]["planned_sessions"], 0);
        assert_eq!(bundle.plan_graph["execution"]["observed_sessions"], 0);
        assert_eq!(bundle.plan_graph["execution"]["planned_worktrees"], 0);
        assert_eq!(bundle.plan_graph["execution"]["observed_worktrees"], 0);
        assert_eq!(bundle.node_brief["coverage_mode"], "legacy_degraded");
        assert!(bundle.node_brief["protocol_records"]
            .as_array()
            .unwrap()
            .iter()
            .any(|record| record["status"] == "missing"
                && record["repair_action"].as_str().is_some()));
        assert!(bundle.plan_graph["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning["code"] == "V3_PLAN_NOT_RECORDED"));
        assert!(bundle.plan_graph["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning["code"] == "V3_PROJECTION_STALE"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn plan_edges_and_execution_counts_are_derived_from_events() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-views-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        super::super::initialize_v3(&root).unwrap();
        let created = create_v3_task(
            &root,
            V3TaskCreateRequest {
                title: "Build projection".to_owned(),
                intent: "Project real plan state".to_owned(),
                acceptance_criteria: vec!["Projection is factual".to_owned()],
                workflow_profile: "standard".to_owned(),
                trigger_context: Default::default(),
                profile_override: None,
                initial_plan: vec![V3TaskCreateInitialPlanNode {
                    node_id: Some("node.initial".to_owned()),
                    title: "Initial".to_owned(),
                    goal: "Start the projection work".to_owned(),
                    scope: Vec::new(),
                    depends_on: Vec::new(),
                    criteria: vec![1],
                    role: None,
                }],
                preflight: false,
            },
        )
        .unwrap();
        let repo = V3ViewRepository::open(&root).unwrap();
        let project_id = repo.project_id();
        let app = V3ApplicationService::open(&root).unwrap();
        let initial_node_id = created.initial_plan_node_ids[0].clone();
        app.lifecycle_command(super::super::lifecycle::command(
            "plan.node_added",
            &project_id,
            &created.task_id,
            created.lifecycle_version,
            "plan.second",
            json!({"node_id":"node.second","title":"Second","goal":"Continue","scope":[],"dependencies":[initial_node_id.clone()]}),
        ))
        .unwrap();
        app.plan_set_state(PlanSetStateCommand {
            identity: PlanCommandIdentity {
                project_id: project_id.clone(),
                task_id: created.task_id.clone(),
                actor: "test".to_owned(),
                expected_version: created.lifecycle_version + 1,
                idempotency_key: "plan.initial.active".to_owned(),
            },
            node_id: initial_node_id.clone(),
            state: "active".to_owned(),
        })
        .unwrap();
        app.session_open_with_context(
            &project_id,
            &created.task_id,
            "session.one",
            "test",
            0,
            "session.open",
            None,
            Some(initial_node_id),
            None,
        )
        .unwrap();
        app.orchestration_command(OrchestrationCommand {
            event_type: "worktree.planned".to_owned(),
            project_id: project_id.clone(),
            task_id: created.task_id.clone(),
            node_id: "node.second".to_owned(),
            worktree_id: "worktree.one".to_owned(),
            session_id: None,
            lease_id: None,
            operation_id: Some("operation.plan".to_owned()),
            eligibility_digest: "a".repeat(64),
            lease_generation: None,
            actor: "test".to_owned(),
            expected_version: 0,
            idempotency_key: "worktree.plan".to_owned(),
            evidence_grade: Some(EvidenceGrade::HardObserved),
            payload: json!({}),
        })
        .unwrap();
        let bundle = repo.load_bundle(&created.task_id).unwrap();
        assert_eq!(bundle.plan_graph["plan_version"], 5);
        assert_eq!(
            bundle.plan_graph["scheduling_edges"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(bundle.plan_graph["execution"]["planned_sessions"], 0);
        assert_eq!(bundle.plan_graph["execution"]["observed_sessions"], 1);
        assert_eq!(bundle.plan_graph["execution"]["planned_worktrees"], 1);
        assert_eq!(bundle.plan_graph["execution"]["observed_worktrees"], 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn node_brief_can_be_loaded_for_any_plan_node_including_completed_ones() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-brief-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        super::super::initialize_v3(&root).unwrap();
        let created = create_v3_task(
            &root,
            V3TaskCreateRequest {
                title: "Inspect any node".to_owned(),
                intent: "Open the brief of a finished node".to_owned(),
                acceptance_criteria: vec!["Every node is inspectable".to_owned()],
                workflow_profile: "standard".to_owned(),
                trigger_context: Default::default(),
                profile_override: None,
                initial_plan: vec![V3TaskCreateInitialPlanNode {
                    node_id: Some("node.initial".to_owned()),
                    title: "Initial".to_owned(),
                    goal: "Open the brief of a finished node".to_owned(),
                    scope: Vec::new(),
                    depends_on: Vec::new(),
                    criteria: vec![1],
                    role: None,
                }],
                preflight: false,
            },
        )
        .unwrap();
        let repo = V3ViewRepository::open(&root).unwrap();
        let project_id = repo.project_id();
        let app = V3ApplicationService::open(&root).unwrap();
        let initial_node_id = created.initial_plan_node_ids[0].clone();
        app.lifecycle_command(super::super::lifecycle::command(
            "plan.node_added",
            &project_id,
            &created.task_id,
            created.lifecycle_version,
            "plan.second",
            json!({"node_id":"node.second","title":"Second","goal":"Continue the work","scope":["src/second.rs"],"dependencies":[initial_node_id.clone()]}),
        ))
        .unwrap();
        for (offset, state) in [(1, "active"), (2, "completed")] {
            app.plan_set_state(PlanSetStateCommand {
                identity: PlanCommandIdentity {
                    project_id: project_id.clone(),
                    task_id: created.task_id.clone(),
                    actor: "test".to_owned(),
                    expected_version: created.lifecycle_version + offset,
                    idempotency_key: format!("plan.initial.{state}"),
                },
                node_id: initial_node_id.clone(),
                state: state.to_owned(),
            })
            .unwrap();
        }

        // The default projection never prefers a completed node, so the cockpit
        // could only ever open the brief of whatever node it happened to pick.
        let default_brief = repo.load_bundle(&created.task_id).unwrap().node_brief;
        assert_eq!(default_brief["node_id"], "node.second");

        let completed_brief = repo
            .load_node_brief(&created.task_id, &initial_node_id)
            .unwrap();
        assert_eq!(completed_brief["node_id"], initial_node_id.as_str());
        assert_eq!(completed_brief["state"], "completed");
        assert_eq!(completed_brief["goal"], "Open the brief of a finished node");
        assert_eq!(completed_brief["task_id"], created.task_id.as_str());

        let second_brief = repo
            .load_node_brief(&created.task_id, "node.second")
            .unwrap();
        assert_eq!(second_brief["node_id"], "node.second");
        assert_eq!(second_brief["goal"], "Continue the work");
        assert_eq!(second_brief["scope"], json!(["src/second.rs"]));
        assert_eq!(second_brief["dependencies"], json!([initial_node_id]));

        let missing = repo
            .load_node_brief(&created.task_id, "node.absent")
            .unwrap_err();
        assert_eq!(missing.code, "V3_NODE_NOT_FOUND");
        let invalid = repo.load_node_brief(&created.task_id, "!!").unwrap_err();
        assert_eq!(invalid.code, "V3_VALIDATION_ERROR");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resolves_the_canonical_current_task_pointer_file() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-pointer-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub/tasks/task.test")).unwrap();
        fs::write(
            root.join(".vibehub/tasks/task.test/task.yaml"),
            "task_id: task.test\ntitle: Test task\nintent: Verify pointer\nphase: implement\nphase_status: active\n",
        )
        .unwrap();
        fs::write(
            root.join(".vibehub/tasks/current"),
            "schema_version: 1\nkind: current_task_pointer\ntask_id: task.test\npath: .vibehub/tasks/task.test\nupdated_at: 2026-07-12T00:00:00Z\nupdated_by: vibehub\n",
        )
        .unwrap();

        let repository = V3ViewRepository::open(&root).unwrap();
        assert_eq!(repository.current_task_id().unwrap(), "task.test");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn derives_source_specific_gap_and_finding_blockers() {
        let task: TaskDocument = serde_yaml::from_str(
            "task_id: task.test\ntitle: Typed blockers\nintent: Explain blockers\nphase: implement\nphase_status: active\n",
        )
        .unwrap();
        let mut lifecycle = TaskLifecycleProjection::empty("task.test");
        lifecycle.sessions.insert(
            "session.test".to_owned(),
            super::super::lifecycle::SessionLifecycleProjection {
                session_id: "session.test".to_owned(),
                host: "Codex".to_owned(),
                node_id: Some("node.test".to_owned()),
                state: "gapped".to_owned(),
                coverage: "degraded".to_owned(),
            },
        );
        lifecycle.findings.insert(
            "finding.test".to_owned(),
            super::super::lifecycle::FindingProjection {
                finding_id: "finding.test".to_owned(),
                severity: "high".to_owned(),
                state: "open".to_owned(),
                target_node_id: Some("node.test".to_owned()),
                evidence_refs: vec!["evt.finding".to_owned()],
                attempt_ids: Vec::new(),
            },
        );

        let details = blocker_details(&task, &[], &lifecycle, None, "2026-07-28T00:00:00Z");
        assert!(details.iter().any(|detail| {
            detail["source_type"] == "session"
                && detail["missing_facts"]
                    .as_array()
                    .is_some_and(|facts| !facts.is_empty())
        }));
        assert!(details.iter().any(|detail| {
            detail["source_type"] == "finding"
                && detail["repair_actions"][0]["instructions"]
                    .as_str()
                    .is_some_and(|text| text.contains("attempt_manage"))
        }));
    }

    #[test]
    fn completion_checklist_item_distinguishes_satisfied_and_blocked_facts() {
        let item = completion_gate_item(
            "criteria_green",
            "completion.review_or_evidence_not_green",
            false,
            "all required criteria passed with fresh evidence",
            "criterion.test=blocked".to_owned(),
            vec!["Windows A07 evidence".to_owned()],
            vec!["blocker.criterion.test".to_owned()],
            vec!["run A07 on the Windows release artifact".to_owned()],
        );
        assert_eq!(item["status"], "blocked");
        assert_eq!(
            item["reason_code"],
            "completion.review_or_evidence_not_green"
        );
        assert_eq!(item["missing_facts"][0], "Windows A07 evidence");
        assert_eq!(item["satisfied_facts"], json!([]));
    }

    #[test]
    fn eventless_blocked_state_is_explicitly_degraded_without_invented_facts() {
        let detail = generic_blocker_detail(Some("node.legacy"));
        assert_eq!(detail["reason_code"], "lifecycle.blocked.details_missing");
        assert_eq!(detail["provenance"]["status"], "degraded");
        assert_eq!(detail["provenance"]["source_event_ids"], json!([]));
        assert!(detail["missing_facts"]
            .as_array()
            .is_some_and(|facts| facts.iter().any(|fact| fact == "原始阻塞原因")));
        assert!(detail["repair_actions"][0]["instructions"]
            .as_str()
            .is_some_and(|action| action.contains("补录可核验事实")));
    }

    #[test]
    fn release_handoff_evidence_becomes_an_exact_windows_repair_action() {
        let context = release_handoff_context(&json!({
            "evidence_refs": [
                "VibeHub_3.1.0_x64-setup.exe sha256=4fc847a802c944de5096d9dacf9f55977168bc8204ab9a4b256e578f30be1a8c",
                "WINDOWS_SIGNING_STATUS=unsigned",
                "BLOCKED: A07, E07, F07-F10, G05 await Windows host"
            ]
        }))
        .expect("release handoff context");

        assert!(context.observed_state.contains("4fc847a802c944de"));
        assert!(context.observed_state.contains("unsigned"));
        assert_eq!(
            context.missing_facts,
            vec![
                "Windows 原生验收 A07 的真实结果",
                "Windows 原生验收 E07 的真实结果",
                "Windows 原生验收 F07-F10 的真实结果",
                "Windows 原生验收 G05 的真实结果"
            ]
        );
        assert!(context.repair_action.contains("Windows/IDE 版本"));
        assert!(context.repair_action.contains("criterion_review"));
    }

    fn blocked_task_document() -> TaskDocument {
        serde_yaml::from_str(
            "task_id: task.test\ntitle: Blocker coverage\nintent: Cover every blocker source\nphase: implement\nphase_status: active\nacceptance_criteria:\n- Windows 原生验收在真实主机完成\n",
        )
        .unwrap()
    }

    fn blocked_lifecycle() -> TaskLifecycleProjection {
        let mut lifecycle = TaskLifecycleProjection::empty("task.test");
        lifecycle.state = "active".to_owned();
        lifecycle.nodes.insert(
            "node.upstream".to_owned(),
            PlanNodeProjection {
                node_id: "node.upstream".to_owned(),
                title: "Upstream".to_owned(),
                goal: "Provide dependency".to_owned(),
                state: "active".to_owned(),
                dependencies: BTreeSet::new(),
                scope: Vec::new(),
                criterion_ids: BTreeSet::new(),
                origin: super::super::lifecycle::PlanNodeOrigin::Authored,
                role: super::super::lifecycle::PlanNodeRole::Execution,
                origin_contract_version: 1,
            },
        );
        lifecycle.nodes.insert(
            "node.downstream".to_owned(),
            PlanNodeProjection {
                node_id: "node.downstream".to_owned(),
                title: "Downstream".to_owned(),
                goal: "Consume dependency".to_owned(),
                state: "blocked".to_owned(),
                dependencies: BTreeSet::from(["node.upstream".to_owned()]),
                scope: Vec::new(),
                criterion_ids: BTreeSet::new(),
                origin: super::super::lifecycle::PlanNodeOrigin::Authored,
                role: super::super::lifecycle::PlanNodeRole::Execution,
                origin_contract_version: 1,
            },
        );
        lifecycle.sessions.insert(
            "session.gapped".to_owned(),
            super::super::lifecycle::SessionLifecycleProjection {
                session_id: "session.gapped".to_owned(),
                host: "Codex".to_owned(),
                node_id: Some("node.downstream".to_owned()),
                state: "gapped".to_owned(),
                coverage: "degraded".to_owned(),
            },
        );
        lifecycle.findings.insert(
            "finding.open".to_owned(),
            super::super::lifecycle::FindingProjection {
                finding_id: "finding.open".to_owned(),
                severity: "high".to_owned(),
                state: "open".to_owned(),
                target_node_id: Some("node.downstream".to_owned()),
                evidence_refs: vec!["evt.finding".to_owned()],
                attempt_ids: Vec::new(),
            },
        );
        lifecycle.criteria.insert(
            "criterion.task.test.c01".to_owned(),
            super::super::lifecycle::CriterionProjection {
                criterion_id: "criterion.task.test.c01".to_owned(),
                title: "Windows 原生验收在真实主机完成".to_owned(),
                required: true,
                state: CriterionState::Accepted,
                evidence_refs: Vec::new(),
                reviewer: None,
                version: 1,
            },
        );
        lifecycle
    }

    fn typed_blocker(detail: &Value) -> super::super::blockers::BlockerDetail {
        let parsed: super::super::blockers::BlockerDetail =
            serde_json::from_value(detail.clone()).expect("blocker detail matches typed contract");
        parsed.validate().expect("blocker detail is actionable");
        parsed
    }

    #[test]
    fn every_workflow_blocker_source_is_typed_and_actionable() {
        let details = blocker_details(
            &blocked_task_document(),
            &[],
            &blocked_lifecycle(),
            None,
            "2026-07-28T00:00:00Z",
        );

        let sources = details
            .iter()
            .map(|detail| typed_blocker(detail).source_type)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            sources,
            BTreeSet::from([
                "criterion".to_owned(),
                "finding".to_owned(),
                "plan_node".to_owned(),
                "session".to_owned(),
            ])
        );
        assert!(details.iter().all(|detail| {
            let blocker = typed_blocker(detail);
            blocker.reason_code != "lifecycle.blocked.details_missing"
                && !blocker.missing_facts.is_empty()
                && blocker.provenance.status
                    == super::super::blockers::BlockerProvenanceStatus::Native
        }));
        assert!(details.iter().any(|detail| {
            typed_blocker(detail)
                .observed_state
                .contains("未完成依赖：node.upstream")
        }));
    }

    #[test]
    fn settled_workflow_truth_leaves_no_blocker_or_generic_message() {
        let mut lifecycle = blocked_lifecycle();
        lifecycle.nodes.get_mut("node.upstream").unwrap().state = "completed".to_owned();
        lifecycle.nodes.get_mut("node.downstream").unwrap().state = "completed".to_owned();
        lifecycle.sessions.get_mut("session.gapped").unwrap().state = "closed".to_owned();
        lifecycle.findings.get_mut("finding.open").unwrap().state = "closed".to_owned();
        let criterion = lifecycle
            .criteria
            .get_mut("criterion.task.test.c01")
            .unwrap();
        criterion.state = CriterionState::Passed;
        criterion.evidence_refs = vec!["cargo test -p vibehub-core v3::views::tests".to_owned()];

        assert_eq!(
            blocker_details(
                &blocked_task_document(),
                &[],
                &lifecycle,
                None,
                "2026-07-28T00:00:00Z"
            ),
            Vec::<Value>::new()
        );
        assert_eq!(
            blocker_details(
                &blocked_task_document(),
                &[],
                &lifecycle,
                Some("node.downstream"),
                "2026-07-28T00:00:00Z"
            ),
            Vec::<Value>::new()
        );
    }

    #[test]
    fn blocker_derivation_is_stable_across_event_order_permutations() {
        let task = blocked_task_document();
        let lifecycle = blocked_lifecycle();
        let events = [
            json!({
                "event_id":"evt.risk","event_type":"risk.logged","event_version":"1.0",
                "aggregate_id":"task.test","aggregate_version":1,"expected_version":0,
                "idempotency_key":"risk.1","project_id":"project.test","task_id":"task.test",
                "node_id":"node.downstream","actor":"test","evidence_grade":"agent_reported",
                "occurred_at":"2026-07-28T00:00:00Z","recorded_at":"2026-07-28T00:00:00Z",
                "payload":{"summary":"权限缺失导致阻塞","external_action_required":true}
            }),
            json!({
                "event_id":"evt.node","event_type":"plan.node_state_changed","event_version":"1.0",
                "aggregate_id":"task.test","aggregate_version":2,"expected_version":1,
                "idempotency_key":"node.1","project_id":"project.test","task_id":"task.test",
                "node_id":"node.downstream","actor":"test","evidence_grade":"agent_reported",
                "occurred_at":"2026-07-28T00:00:01Z","recorded_at":"2026-07-28T00:00:01Z",
                "payload":{"node_id":"node.downstream","state":"blocked"}
            }),
        ]
        .into_iter()
        .map(|event| serde_json::from_value::<V3EventEnvelope>(event).unwrap())
        .collect::<Vec<_>>();

        let forward = blocker_details(&task, &events, &lifecycle, None, "2026-07-28T00:00:00Z");
        let mut permuted = events.clone();
        permuted.reverse();
        let reversed = blocker_details(&task, &permuted, &lifecycle, None, "2026-07-28T00:00:00Z");

        let reasons = |details: &[Value]| {
            details
                .iter()
                .map(|detail| typed_blocker(detail).reason_code)
                .collect::<BTreeSet<_>>()
        };
        assert_eq!(reasons(&forward), reasons(&reversed));
        assert!(reasons(&forward).contains("macos.accessibility.tcc"));
        assert!(reasons(&forward).contains("lifecycle.blocked"));
        assert!(forward.iter().any(|detail| {
            let blocker = typed_blocker(detail);
            blocker.kind == super::super::blockers::BlockerKind::Permission
                && blocker
                    .provenance
                    .source_event_ids
                    .contains(&"evt.risk".to_owned())
        }));
    }

    #[test]
    fn stale_criterion_evidence_keeps_the_completion_gate_blocked() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-stale-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub/tasks/task.test")).unwrap();
        fs::write(
            root.join(".vibehub/tasks/task.test/task.yaml"),
            "task_id: task.test\ntitle: Stale evidence\nintent: Reject stale completion evidence\nphase: implement\nphase_status: active\nworkflow_profile: lightweight\nacceptance_criteria:\n- Windows 原生验收在真实主机完成\n",
        )
        .unwrap();
        let app = V3ApplicationService::open(&root).unwrap();
        let repo = V3ViewRepository::open(&root).unwrap();
        app.lifecycle_command(super::super::lifecycle::command(
            "criterion.passed",
            &repo.project_id(),
            "task.test",
            0,
            "criterion.stale",
            json!({"criterion_id":"criterion.task.test.c01","reviewer":"reviewer","evidence_refs":["stale:2026-07-01-windows-run"]}),
        ))
        .unwrap();

        let bundle = repo.load_bundle("task.test").unwrap();
        let criteria_gate = bundle.node_brief["completion_gate"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["gate"] == "criteria_green")
            .unwrap()
            .clone();
        assert_eq!(criteria_gate["passed"], false);
        assert_eq!(
            criteria_gate["missing_facts"][0],
            "criterion.task.test.c01=invalid_or_stale_evidence"
        );
        assert_eq!(bundle.node_brief["completion_gate"]["all_passed"], false);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_session_progress_is_a_visible_gate_not_a_hidden_completion_failure() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-progress-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        super::super::initialize_v3(&root).unwrap();
        let created = create_v3_task(
            &root,
            V3TaskCreateRequest {
                title: "Progress gate".to_owned(),
                intent: "Expose the progress requirement in the view".to_owned(),
                acceptance_criteria: vec!["Gate is visible".to_owned()],
                workflow_profile: "standard".to_owned(),
                trigger_context: Default::default(),
                profile_override: None,
                initial_plan: vec![V3TaskCreateInitialPlanNode {
                    node_id: Some("node.initial".to_owned()),
                    title: "Initial".to_owned(),
                    goal: "Start the progress-gated work".to_owned(),
                    scope: Vec::new(),
                    depends_on: Vec::new(),
                    criteria: vec![1],
                    role: None,
                }],
                preflight: false,
            },
        )
        .unwrap();
        let repo = V3ViewRepository::open(&root).unwrap();
        let project_id = repo.project_id();
        let app = V3ApplicationService::open(&root).unwrap();
        let initial_node_id = created.initial_plan_node_ids[0].clone();
        app.plan_set_state(PlanSetStateCommand {
            identity: PlanCommandIdentity {
                project_id: project_id.clone(),
                task_id: created.task_id.clone(),
                actor: "test".to_owned(),
                expected_version: created.lifecycle_version,
                idempotency_key: "plan.progress.active".to_owned(),
            },
            node_id: initial_node_id.clone(),
            state: "active".to_owned(),
        })
        .unwrap();
        app.session_open_with_context(
            &project_id,
            &created.task_id,
            "session.silent",
            "test",
            0,
            "session.silent.open",
            None,
            Some(initial_node_id),
            None,
        )
        .unwrap();

        let gate = |bundle: &V3ViewBundle| {
            bundle.node_brief["completion_gate"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["gate"] == "progress_evidence")
                .unwrap()
                .clone()
        };
        let blocked = gate(&repo.load_bundle(&created.task_id).unwrap());
        assert_eq!(blocked["passed"], false);
        assert_eq!(blocked["missing_facts"][0], "session.silent");
        assert!(blocked["repair_actions"][0]
            .as_str()
            .is_some_and(|action| action.contains("session_recovery")));

        app.session_gap(
            &project_id,
            &created.task_id,
            "session.silent",
            "test",
            1,
            "session.silent.gap",
            "interrupted without progress",
        )
        .unwrap();
        app.session_recover(
            &project_id,
            &created.task_id,
            "session.silent",
            "test",
            2,
            "session.silent.recover",
            vec!["evt.gap".to_owned()],
        )
        .unwrap();

        let recovered = gate(&repo.load_bundle(&created.task_id).unwrap());
        assert_eq!(recovered["passed"], true);
        assert_eq!(recovered["missing_facts"], json!([]));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unknown_legacy_session_demotes_completion_and_exposes_recovery_action() {
        let event = serde_json::from_value::<V3EventEnvelope>(json!({
            "event_id": "evt.legacy.session",
            "event_type": "session.task_bound",
            "event_version": "1.0",
            "aggregate_id": "session.legacy",
            "aggregate_version": 1,
            "expected_version": 0,
            "idempotency_key": "legacy.session.bound",
            "project_id": "project.test",
            "task_id": "task.test",
            "session_id": "session.legacy",
            "actor": "legacy-agent",
            "evidence_grade": "agent_reported",
            "occurred_at": "2026-07-28T00:00:00Z",
            "recorded_at": "2026-07-28T00:00:00Z",
            "payload": {"session_id": "session.legacy"}
        }))
        .unwrap();
        let events = vec![event];
        let mut lifecycle = fold_task("task.test", &events);
        lifecycle.state = "completed".to_owned();
        let integrity = session_integrity(&events, "task.test", &lifecycle);
        assert_eq!(integrity.unknown, vec!["session.legacy"]);
        assert_eq!(
            task_state_for(&lifecycle, None, &integrity, false),
            "blocked"
        );

        let details = blocker_details(
            &blocked_task_document(),
            &events,
            &lifecycle,
            None,
            "2026-07-28T00:00:00Z",
        );
        let legacy = details
            .iter()
            .map(typed_blocker)
            .find(|detail| detail.reason_code == "session.unknown:session.legacy")
            .expect("legacy Session must remain an actionable blocker");
        assert!(legacy.summary.contains("Legacy/unknown"));
        assert!(legacy.repair_actions[0]
            .instructions
            .contains("保持 blocked"));
    }

    #[test]
    fn projection_stale_warning_is_actionable_and_not_a_completion_claim() {
        let warning = projection_warning(&json!({
            "stale": true,
            "event_count": 12,
            "projection_event_count": 11,
            "last_event_timestamp": "2026-07-28T00:00:00Z"
        }))
        .expect("stale projection warning");
        assert_eq!(warning["code"], "V3_PROJECTION_STALE");
        assert_eq!(warning["details"]["event_count"], 12);
        assert!(warning["details"]["repair_action"]
            .as_str()
            .is_some_and(|action| action.contains("rebuild")));

        let blocker = projection_stale_blocker_detail(
            "project.test",
            &json!({"stale": true, "event_count": 12, "projection_event_count": 11}),
            "2026-07-28T00:00:00Z",
        );
        let blocker = typed_blocker(&blocker);
        assert_eq!(blocker.reason_code, "projection.stale");
        assert!(blocker.why_blocked.contains("旧 projection"));
    }
}

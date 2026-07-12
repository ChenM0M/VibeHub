use super::domain::{
    AppendResult, EventDraft, EvidenceGrade, NodeId, ProjectId, SessionId, TaskId, V3Error,
    V3ErrorCategory, V3EventEnvelope,
};
use super::event_store::V3EventStore;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const LIFECYCLE_EVENT_TYPES: &[&str] = &[
    "task.created",
    "plan.node_added",
    "plan.node_state_changed",
    "plan.dependency_changed",
    "criterion.accepted",
    "criterion.passed",
    "criterion.failed",
    "criterion.blocked",
    "criterion.not_applicable",
    "finding.opened",
    "finding.closed",
    "finding.regressed",
    "attempt.started",
    "attempt.completed",
    "attempt.failed",
    "session.opened",
    "session.heartbeat",
    "session.closed",
    "session.gap_detected",
    "session.recovered",
    "task.completion_proposed",
    "task.completion_confirmed",
    "task.completion_rejected",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CriterionState {
    Proposed,
    Accepted,
    Passed,
    Failed,
    Blocked,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriterionProjection {
    pub criterion_id: String,
    pub title: String,
    pub required: bool,
    pub state: CriterionState,
    pub evidence_refs: Vec<String>,
    pub reviewer: Option<String>,
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanNodeProjection {
    pub node_id: String,
    pub title: String,
    pub goal: String,
    pub state: String,
    pub dependencies: BTreeSet<String>,
    pub scope: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindingProjection {
    pub finding_id: String,
    pub severity: String,
    pub state: String,
    pub target_node_id: Option<String>,
    pub evidence_refs: Vec<String>,
    pub attempt_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttemptProjection {
    pub attempt_id: String,
    pub finding_id: String,
    pub node_id: Option<String>,
    pub state: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionLifecycleProjection {
    pub session_id: String,
    pub host: String,
    pub node_id: Option<String>,
    pub state: String,
    pub coverage: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionConfirmation {
    pub proposal_event_id: String,
    pub digest: String,
    pub proposed_at_version: u64,
    pub confirmed_at_version: Option<u64>,
    pub confirmed_by: Option<String>,
    pub channel: Option<String>,
    pub valid: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskLifecycleProjection {
    pub task_id: String,
    pub version: u64,
    pub state: String,
    pub nodes: BTreeMap<String, PlanNodeProjection>,
    pub criteria: BTreeMap<String, CriterionProjection>,
    pub findings: BTreeMap<String, FindingProjection>,
    pub attempts: BTreeMap<String, AttemptProjection>,
    pub sessions: BTreeMap<String, SessionLifecycleProjection>,
    pub confirmation: Option<CompletionConfirmation>,
    pub event_ids: Vec<String>,
}

impl TaskLifecycleProjection {
    pub fn empty(task_id: &str) -> Self {
        Self {
            task_id: task_id.to_owned(),
            version: 0,
            state: "planned".to_owned(),
            nodes: BTreeMap::new(),
            criteria: BTreeMap::new(),
            findings: BTreeMap::new(),
            attempts: BTreeMap::new(),
            sessions: BTreeMap::new(),
            confirmation: None,
            event_ids: Vec::new(),
        }
    }

    pub fn required_criteria_passed(&self, required_criterion_ids: &BTreeSet<String>) -> bool {
        !required_criterion_ids.is_empty()
            && required_criterion_ids.iter().all(|criterion_id| {
                self.criteria.get(criterion_id).is_some_and(|criterion| {
                    matches!(
                        criterion.state,
                        CriterionState::Passed | CriterionState::NotApplicable
                    ) && (!criterion.evidence_refs.is_empty()
                        || criterion.state == CriterionState::NotApplicable)
                })
            })
    }

    pub fn has_open_findings(&self) -> bool {
        self.findings
            .values()
            .any(|finding| finding.state != "closed")
    }

    pub fn completion_digest(&self) -> String {
        let criteria = self
            .criteria
            .values()
            .map(|criterion| {
                format!(
                    "{}:{:?}:{}:{}",
                    criterion.criterion_id,
                    criterion.state,
                    criterion.version,
                    criterion.evidence_refs.join(",")
                )
            })
            .collect::<Vec<_>>()
            .join("|");
        let mut hasher = Sha256::new();
        hasher.update(format!("{}:{}:{}", self.task_id, self.version, criteria));
        format!("sha256:{:x}", hasher.finalize())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LifecycleCommand {
    pub event_type: String,
    pub project_id: String,
    pub task_id: String,
    #[serde(default)]
    pub node_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    pub actor: String,
    pub expected_version: u64,
    pub idempotency_key: String,
    #[serde(default)]
    pub evidence_grade: Option<EvidenceGrade>,
    #[serde(default)]
    pub payload: Value,
}

#[cfg(test)]
pub(crate) fn apply_command(
    store: &V3EventStore,
    command: LifecycleCommand,
) -> Result<AppendResult, V3Error> {
    let required_criterion_ids =
        fold_task(&command.task_id, &store.load_project(&command.project_id)?)
            .criteria
            .into_iter()
            .filter_map(|(id, criterion)| criterion.required.then_some(id))
            .collect();
    apply_command_with_required_criteria(store, command, &required_criterion_ids)
}

pub(crate) fn apply_command_with_required_criteria(
    store: &V3EventStore,
    mut command: LifecycleCommand,
    required_criterion_ids: &BTreeSet<String>,
) -> Result<AppendResult, V3Error> {
    if !LIFECYCLE_EVENT_TYPES.contains(&command.event_type.as_str()) {
        return Err(validation(
            "V3_LIFECYCLE_EVENT_UNSUPPORTED",
            "event type is not part of the M4 lifecycle catalog",
        ));
    }
    let events = store.load_project(&command.project_id)?;
    if events.iter().any(|event| {
        event.idempotency_key == command.idempotency_key
            && event.event_type == command.event_type
            && event.task_id.0 == command.task_id
    }) {
        return store.append(command_draft(command));
    }
    let projection = fold_task(&command.task_id, &events);
    if command.event_type == "task.completion_proposed"
        && command
            .payload
            .get("digest")
            .and_then(Value::as_str)
            .is_none()
    {
        command.payload["digest"] = Value::String(projection.completion_digest());
    }
    validate_command(&projection, &command, required_criterion_ids)?;
    store.append(command_draft(command))
}

fn command_draft(command: LifecycleCommand) -> EventDraft {
    EventDraft {
        event_type: command.event_type,
        aggregate_id: command.task_id.clone(),
        expected_version: command.expected_version,
        idempotency_key: command.idempotency_key,
        project_id: ProjectId(command.project_id),
        task_id: TaskId(command.task_id),
        node_id: command.node_id.map(NodeId),
        session_id: command.session_id.map(SessionId),
        actor: command.actor,
        evidence_grade: command
            .evidence_grade
            .unwrap_or(EvidenceGrade::AgentReported),
        occurred_at: None,
        commit_sha: None,
        payload: command.payload,
    }
}

pub fn fold_task(task_id: &str, events: &[V3EventEnvelope]) -> TaskLifecycleProjection {
    let mut projection = TaskLifecycleProjection::empty(task_id);
    for event in events.iter().filter(|event| event.task_id.0 == task_id) {
        if event.aggregate_id == task_id {
            projection.version = projection.version.max(event.aggregate_version);
        }
        projection.event_ids.push(event.event_id.clone());
        if let Some(confirmation) = projection.confirmation.as_mut() {
            if event.event_type != "task.completion_confirmed"
                && event.aggregate_version > confirmation.proposed_at_version
            {
                confirmation.valid = false;
                projection.state = "active".to_owned();
            }
        }
        apply_event(&mut projection, event);
    }
    projection
}

fn apply_event(projection: &mut TaskLifecycleProjection, event: &V3EventEnvelope) {
    let payload = &event.payload;
    match event.event_type.as_str() {
        "task.created" => projection.state = "active".to_owned(),
        "plan.node_added" => {
            let Some(node_id) = value_id(payload, "node_id")
                .or_else(|| event.node_id.as_ref().map(|id| id.0.clone()))
            else {
                return;
            };
            projection.nodes.insert(
                node_id.clone(),
                PlanNodeProjection {
                    node_id,
                    title: value_string(payload, "title", "Untitled node"),
                    goal: value_string(payload, "goal", ""),
                    state: "planned".to_owned(),
                    dependencies: string_set(payload, "dependencies"),
                    scope: string_vec(payload, "scope"),
                },
            );
        }
        "plan.node_state_changed" => {
            if let Some(node) = find_node_mut(projection, event, payload) {
                node.state = value_string(payload, "state", &node.state);
            }
        }
        "plan.dependency_changed" => {
            if let Some(node) = find_node_mut(projection, event, payload) {
                node.dependencies = string_set(payload, "dependencies");
            }
        }
        event_type if event_type.starts_with("criterion.") => {
            let Some(criterion_id) = value_id(payload, "criterion_id") else {
                return;
            };
            let state = criterion_state(event_type);
            let entry = projection
                .criteria
                .entry(criterion_id.clone())
                .or_insert_with(|| CriterionProjection {
                    criterion_id,
                    title: value_string(payload, "title", "Acceptance criterion"),
                    required: payload
                        .get("required")
                        .and_then(Value::as_bool)
                        .unwrap_or(true),
                    state: CriterionState::Proposed,
                    evidence_refs: Vec::new(),
                    reviewer: None,
                    version: 0,
                });
            entry.state = state;
            entry.version += 1;
            entry.evidence_refs = string_vec(payload, "evidence_refs");
            entry.reviewer = payload
                .get("reviewer")
                .and_then(Value::as_str)
                .map(str::to_owned);
        }
        "finding.opened" | "finding.regressed" => {
            let Some(finding_id) = value_id(payload, "finding_id") else {
                return;
            };
            let entry = projection
                .findings
                .entry(finding_id.clone())
                .or_insert_with(|| FindingProjection {
                    finding_id,
                    severity: value_string(payload, "severity", "medium"),
                    state: "open".to_owned(),
                    target_node_id: payload
                        .get("target_node_id")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    evidence_refs: Vec::new(),
                    attempt_ids: Vec::new(),
                });
            entry.state = if event.event_type == "finding.regressed" {
                "regressed"
            } else {
                "open"
            }
            .to_owned();
            entry
                .evidence_refs
                .extend(string_vec(payload, "evidence_refs"));
        }
        "finding.closed" => {
            if let Some(finding) =
                value_id(payload, "finding_id").and_then(|id| projection.findings.get_mut(&id))
            {
                finding.state = "closed".to_owned();
            }
        }
        event_type if event_type.starts_with("attempt.") => {
            let Some(attempt_id) = value_id(payload, "attempt_id") else {
                return;
            };
            let finding_id = value_string(payload, "finding_id", "");
            let state = event_type.trim_start_matches("attempt.").to_owned();
            let entry = projection
                .attempts
                .entry(attempt_id.clone())
                .or_insert_with(|| AttemptProjection {
                    attempt_id: attempt_id.clone(),
                    finding_id: finding_id.clone(),
                    node_id: event.node_id.as_ref().map(|id| id.0.clone()),
                    state: state.clone(),
                    evidence_refs: Vec::new(),
                });
            entry.state = state;
            entry
                .evidence_refs
                .extend(string_vec(payload, "evidence_refs"));
            if let Some(finding) = projection.findings.get_mut(&finding_id) {
                if !finding.attempt_ids.contains(&attempt_id) {
                    finding.attempt_ids.push(attempt_id);
                }
            }
        }
        event_type if event_type.starts_with("session.") => {
            let Some(session_id) = event.session_id.as_ref().map(|id| id.0.clone()) else {
                return;
            };
            let state = match event_type {
                "session.opened" => "active",
                "session.heartbeat" => "active",
                "session.closed" => "closed",
                "session.gap_detected" => "gapped",
                "session.recovered" => "repaired",
                _ => "unknown",
            };
            let coverage = match state {
                "closed" => "complete",
                "gapped" => "degraded",
                "repaired" => "recoverable",
                _ => "unknown",
            };
            let entry = projection
                .sessions
                .entry(session_id.clone())
                .or_insert_with(|| SessionLifecycleProjection {
                    session_id,
                    host: value_string(payload, "host", &event.actor),
                    node_id: event.node_id.as_ref().map(|id| id.0.clone()),
                    state: state.to_owned(),
                    coverage: coverage.to_owned(),
                });
            entry.state = state.to_owned();
            entry.coverage = coverage.to_owned();
        }
        "task.completion_proposed" => {
            projection.confirmation = Some(CompletionConfirmation {
                proposal_event_id: event.event_id.clone(),
                digest: value_string(payload, "digest", ""),
                proposed_at_version: event.aggregate_version,
                confirmed_at_version: None,
                confirmed_by: None,
                channel: None,
                valid: true,
            });
            projection.state = "completion_pending".to_owned();
        }
        "task.completion_confirmed" => {
            if let Some(confirmation) = projection.confirmation.as_mut() {
                confirmation.confirmed_at_version = Some(event.aggregate_version);
                confirmation.confirmed_by = payload
                    .get("confirmed_by")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                confirmation.channel = payload
                    .get("channel")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                confirmation.valid = payload.get("digest").and_then(Value::as_str)
                    == Some(confirmation.digest.as_str());
                projection.state = if confirmation.valid {
                    "completed"
                } else {
                    "active"
                }
                .to_owned();
            }
        }
        "task.completion_rejected" => {
            if let Some(confirmation) = projection.confirmation.as_mut() {
                confirmation.valid = false;
            }
            projection.state = "active".to_owned();
        }
        _ => {}
    }
}

fn validate_command(
    projection: &TaskLifecycleProjection,
    command: &LifecycleCommand,
    required_criterion_ids: &BTreeSet<String>,
) -> Result<(), V3Error> {
    if command.expected_version != projection.version {
        return Err(V3Error::new(
            "V3_VERSION_CONFLICT",
            V3ErrorCategory::VersionConflict,
            true,
            "expected task version does not match current lifecycle version",
        )
        .with_detail("expected_version", command.expected_version)
        .with_detail("current_version", projection.version));
    }
    let payload = &command.payload;
    match command.event_type.as_str() {
        "plan.node_added" => {
            let node_id = required_id(payload, "node_id")?;
            if projection.nodes.contains_key(node_id) {
                return Err(validation("V3_NODE_EXISTS", "plan node already exists"));
            }
            ensure_dependencies_exist(projection, payload)?;
            let mut candidate = projection.clone();
            let synthetic = synthetic_event(command, projection.version + 1);
            apply_event(&mut candidate, &synthetic);
            ensure_acyclic(&candidate)?;
        }
        "plan.dependency_changed" => {
            require_node(projection, command, payload)?;
            ensure_dependencies_exist(projection, payload)?;
            let mut candidate = projection.clone();
            let synthetic = synthetic_event(command, projection.version + 1);
            apply_event(&mut candidate, &synthetic);
            ensure_acyclic(&candidate)?;
        }
        "plan.node_state_changed" => {
            let node = require_node(projection, command, payload)?;
            let next = required_id(payload, "state")?;
            let legal = matches!(
                (node.state.as_str(), next),
                ("planned", "ready" | "active" | "cancelled" | "blocked")
                    | ("ready", "active" | "cancelled" | "blocked")
                    | ("active", "completed" | "failed" | "blocked" | "cancelled")
                    | ("blocked", "ready" | "active" | "cancelled")
                    | ("failed", "active" | "cancelled")
            );
            if !legal {
                return Err(validation(
                    "V3_ILLEGAL_NODE_TRANSITION",
                    "plan node transition is not legal",
                ));
            }
        }
        event_type if event_type.starts_with("criterion.") => {
            required_id(payload, "criterion_id")?;
            if matches!(
                event_type,
                "criterion.passed" | "criterion.failed" | "criterion.blocked"
            ) {
                if string_vec(payload, "evidence_refs").is_empty()
                    || payload.get("reviewer").and_then(Value::as_str).is_none()
                {
                    return Err(validation(
                        "V3_CRITERION_EVIDENCE_REQUIRED",
                        "criterion review requires evidence_refs and reviewer",
                    ));
                }
            }
        }
        "attempt.started" => {
            let finding_id = required_id(payload, "finding_id")?;
            required_id(payload, "attempt_id")?;
            if !projection.findings.contains_key(finding_id) {
                return Err(validation(
                    "V3_FINDING_NOT_FOUND",
                    "attempt must address an existing finding",
                ));
            }
        }
        "finding.closed" => {
            let finding_id = required_id(payload, "finding_id")?;
            let finding = projection
                .findings
                .get(finding_id)
                .ok_or_else(|| validation("V3_FINDING_NOT_FOUND", "finding does not exist"))?;
            if finding.attempt_ids.is_empty() {
                return Err(validation(
                    "V3_FINDING_REVIEW_REQUIRED",
                    "finding cannot close without a remediation attempt",
                ));
            }
        }
        "task.completion_proposed" => {
            if !projection.required_criteria_passed(required_criterion_ids)
                || projection.has_open_findings()
            {
                return Err(validation(
                    "V3_TASK_NOT_COMPLETABLE",
                    "all required criteria need evidence and all findings must be closed",
                ));
            }
            let digest = required_id(payload, "digest")?;
            if digest != projection.completion_digest() {
                return Err(validation(
                    "V3_CONFIRMATION_DIGEST_STALE",
                    "completion proposal digest does not match current task truth",
                ));
            }
        }
        "task.completion_confirmed" => {
            let confirmation = projection
                .confirmation
                .as_ref()
                .filter(|value| value.valid)
                .ok_or_else(|| {
                    validation(
                        "V3_CONFIRMATION_MISSING",
                        "no valid completion proposal exists",
                    )
                })?;
            if payload.get("digest").and_then(Value::as_str) != Some(confirmation.digest.as_str())
                || payload
                    .get("confirmed_by")
                    .and_then(Value::as_str)
                    .is_none()
                || !matches!(
                    payload.get("channel").and_then(Value::as_str),
                    Some("desktop_ui" | "cli")
                )
            {
                return Err(validation("V3_CONFIRMATION_AUTHENTICITY_REQUIRED", "completion confirmation requires matching digest, human identity, and trusted channel"));
            }
        }
        _ => {}
    }
    Ok(())
}

fn ensure_dependencies_exist(
    projection: &TaskLifecycleProjection,
    payload: &Value,
) -> Result<(), V3Error> {
    for dependency in string_set(payload, "dependencies") {
        if !projection.nodes.contains_key(&dependency) {
            return Err(validation(
                "V3_DEPENDENCY_NOT_FOUND",
                "scheduling dependency does not exist",
            ));
        }
    }
    Ok(())
}

fn ensure_acyclic(projection: &TaskLifecycleProjection) -> Result<(), V3Error> {
    fn visit(
        id: &str,
        projection: &TaskLifecycleProjection,
        visiting: &mut BTreeSet<String>,
        visited: &mut BTreeSet<String>,
    ) -> bool {
        if visiting.contains(id) {
            return false;
        }
        if visited.contains(id) {
            return true;
        }
        visiting.insert(id.to_owned());
        if let Some(node) = projection.nodes.get(id) {
            for dependency in &node.dependencies {
                if !visit(dependency, projection, visiting, visited) {
                    return false;
                }
            }
        }
        visiting.remove(id);
        visited.insert(id.to_owned());
        true
    }
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for id in projection.nodes.keys() {
        if !visit(id, projection, &mut visiting, &mut visited) {
            return Err(validation(
                "V3_PLAN_CYCLE",
                "scheduling dependencies must remain acyclic",
            ));
        }
    }
    Ok(())
}

fn synthetic_event(command: &LifecycleCommand, aggregate_version: u64) -> V3EventEnvelope {
    V3EventEnvelope {
        event_id: "evt.validation".to_owned(),
        event_type: command.event_type.clone(),
        event_version: "1.0".to_owned(),
        aggregate_id: command.task_id.clone(),
        aggregate_version,
        expected_version: command.expected_version,
        idempotency_key: command.idempotency_key.clone(),
        project_id: ProjectId(command.project_id.clone()),
        task_id: TaskId(command.task_id.clone()),
        node_id: command.node_id.clone().map(NodeId),
        session_id: command.session_id.clone().map(SessionId),
        actor: command.actor.clone(),
        evidence_grade: command
            .evidence_grade
            .unwrap_or(EvidenceGrade::AgentReported),
        occurred_at: String::new(),
        recorded_at: String::new(),
        commit_sha: None,
        payload: command.payload.clone(),
    }
}

fn require_node<'a>(
    projection: &'a TaskLifecycleProjection,
    command: &LifecycleCommand,
    payload: &Value,
) -> Result<&'a PlanNodeProjection, V3Error> {
    let id = command
        .node_id
        .as_deref()
        .or_else(|| payload.get("node_id").and_then(Value::as_str))
        .ok_or_else(|| validation("V3_NODE_ID_REQUIRED", "node_id is required"))?;
    projection
        .nodes
        .get(id)
        .ok_or_else(|| validation("V3_NODE_NOT_FOUND", "plan node does not exist"))
}

fn find_node_mut<'a>(
    projection: &'a mut TaskLifecycleProjection,
    event: &V3EventEnvelope,
    payload: &Value,
) -> Option<&'a mut PlanNodeProjection> {
    let id = event
        .node_id
        .as_ref()
        .map(|id| id.0.as_str())
        .or_else(|| payload.get("node_id").and_then(Value::as_str))?;
    projection.nodes.get_mut(id)
}

fn criterion_state(event_type: &str) -> CriterionState {
    match event_type {
        "criterion.accepted" => CriterionState::Accepted,
        "criterion.passed" => CriterionState::Passed,
        "criterion.failed" => CriterionState::Failed,
        "criterion.blocked" => CriterionState::Blocked,
        "criterion.not_applicable" => CriterionState::NotApplicable,
        _ => CriterionState::Proposed,
    }
}

fn required_id<'a>(payload: &'a Value, field: &str) -> Result<&'a str, V3Error> {
    payload
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            validation(
                "V3_LIFECYCLE_FIELD_REQUIRED",
                format!("{field} is required"),
            )
        })
}
fn value_id(payload: &Value, field: &str) -> Option<String> {
    payload
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
}
fn value_string(payload: &Value, field: &str, fallback: &str) -> String {
    payload
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_owned()
}
fn string_vec(payload: &Value, field: &str) -> Vec<String> {
    payload
        .get(field)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}
fn string_set(payload: &Value, field: &str) -> BTreeSet<String> {
    string_vec(payload, field).into_iter().collect()
}
fn validation(code: &str, message: impl Into<String>) -> V3Error {
    V3Error::new(code, V3ErrorCategory::Validation, false, message)
}

pub fn command(
    event_type: &str,
    project_id: &str,
    task_id: &str,
    version: u64,
    key: &str,
    payload: Value,
) -> LifecycleCommand {
    LifecycleCommand {
        event_type: event_type.to_owned(),
        project_id: project_id.to_owned(),
        task_id: task_id.to_owned(),
        node_id: payload
            .get("node_id")
            .and_then(Value::as_str)
            .map(str::to_owned),
        session_id: payload
            .get("session_id")
            .and_then(Value::as_str)
            .map(str::to_owned),
        actor: "test".to_owned(),
        expected_version: version,
        idempotency_key: key.to_owned(),
        evidence_grade: Some(EvidenceGrade::HardObserved),
        payload,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use uuid::Uuid;

    fn store() -> (std::path::PathBuf, V3EventStore) {
        let root = std::env::temp_dir().join(format!("vibehub-m4-lifecycle-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub")).unwrap();
        let store = V3EventStore::open(&root).unwrap();
        (root, store)
    }
    fn apply(
        store: &V3EventStore,
        version: &mut u64,
        event_type: &str,
        key: &str,
        payload: Value,
    ) -> AppendResult {
        let result = apply_command(
            store,
            command(
                event_type,
                "project.test",
                "task.test",
                *version,
                key,
                payload,
            ),
        )
        .unwrap();
        if matches!(result, AppendResult::Appended { .. }) {
            *version += 1;
        }
        result
    }

    #[test]
    fn cycle_and_illegal_transition_are_rejected() {
        let (root, store) = store();
        let mut version = 0;
        apply(
            &store,
            &mut version,
            "plan.node_added",
            "n1",
            json!({"node_id":"node.a","title":"A","dependencies":[]}),
        );
        apply(
            &store,
            &mut version,
            "plan.node_added",
            "n2",
            json!({"node_id":"node.b","title":"B","dependencies":["node.a"]}),
        );
        let cycle = apply_command(
            &store,
            command(
                "plan.dependency_changed",
                "project.test",
                "task.test",
                version,
                "cycle",
                json!({"node_id":"node.a","dependencies":["node.b"]}),
            ),
        )
        .unwrap_err();
        assert_eq!(cycle.code, "V3_PLAN_CYCLE");
        let illegal = apply_command(
            &store,
            command(
                "plan.node_state_changed",
                "project.test",
                "task.test",
                version,
                "illegal",
                json!({"node_id":"node.a","state":"completed"}),
            ),
        )
        .unwrap_err();
        assert_eq!(illegal.code, "V3_ILLEGAL_NODE_TRANSITION");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn two_remediation_cycles_and_completion_truth_survive_rebuild() {
        let (root, store) = store();
        let mut version = 0;
        apply(
            &store,
            &mut version,
            "criterion.accepted",
            "c0",
            json!({"criterion_id":"criterion.one","title":"It works","required":true}),
        );
        apply(
            &store,
            &mut version,
            "finding.opened",
            "f0",
            json!({"finding_id":"finding.one","severity":"high","evidence_refs":["evidence.fail.1"]}),
        );
        apply(
            &store,
            &mut version,
            "attempt.started",
            "a1s",
            json!({"attempt_id":"attempt.one","finding_id":"finding.one"}),
        );
        apply(
            &store,
            &mut version,
            "attempt.failed",
            "a1f",
            json!({"attempt_id":"attempt.one","finding_id":"finding.one","evidence_refs":["evidence.attempt.1"]}),
        );
        apply(
            &store,
            &mut version,
            "finding.regressed",
            "f1",
            json!({"finding_id":"finding.one","evidence_refs":["evidence.fail.2"]}),
        );
        apply(
            &store,
            &mut version,
            "attempt.started",
            "a2s",
            json!({"attempt_id":"attempt.two","finding_id":"finding.one"}),
        );
        apply(
            &store,
            &mut version,
            "attempt.completed",
            "a2c",
            json!({"attempt_id":"attempt.two","finding_id":"finding.one","evidence_refs":["evidence.attempt.2"]}),
        );
        apply(
            &store,
            &mut version,
            "finding.closed",
            "fc",
            json!({"finding_id":"finding.one"}),
        );
        let incomplete = apply_command(
            &store,
            command(
                "task.completion_proposed",
                "project.test",
                "task.test",
                version,
                "p0",
                json!({"digest":"bad"}),
            ),
        )
        .unwrap_err();
        assert_eq!(incomplete.code, "V3_TASK_NOT_COMPLETABLE");
        apply(
            &store,
            &mut version,
            "criterion.passed",
            "cp",
            json!({"criterion_id":"criterion.one","reviewer":"reviewer.one","evidence_refs":["evidence.pass"]}),
        );
        let projection = fold_task("task.test", &store.load_project("project.test").unwrap());
        let digest = projection.completion_digest();
        apply(
            &store,
            &mut version,
            "task.completion_proposed",
            "p1",
            json!({"digest":digest}),
        );
        apply(
            &store,
            &mut version,
            "task.completion_confirmed",
            "p2",
            json!({"digest":digest,"confirmed_by":"owner","channel":"desktop_ui"}),
        );
        let rebuilt = fold_task("task.test", &store.load_project("project.test").unwrap());
        assert_eq!(rebuilt.state, "completed");
        assert_eq!(
            rebuilt.findings["finding.one"].attempt_ids,
            vec!["attempt.one", "attempt.two"]
        );
        assert_eq!(rebuilt.attempts.len(), 2);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn duplicate_retry_is_noop_and_new_truth_invalidates_confirmation() {
        let (root, store) = store();
        let mut version = 0;
        apply(
            &store,
            &mut version,
            "criterion.accepted",
            "c0",
            json!({"criterion_id":"criterion.one","required":true}),
        );
        apply(
            &store,
            &mut version,
            "criterion.passed",
            "c1",
            json!({"criterion_id":"criterion.one","reviewer":"reviewer","evidence_refs":["evidence.pass"]}),
        );
        let digest = fold_task("task.test", &store.load_project("project.test").unwrap())
            .completion_digest();
        apply(
            &store,
            &mut version,
            "task.completion_proposed",
            "p1",
            json!({"digest":digest}),
        );
        let confirmed = apply(
            &store,
            &mut version,
            "task.completion_confirmed",
            "p2",
            json!({"digest":digest,"confirmed_by":"owner","channel":"cli"}),
        );
        assert!(matches!(confirmed, AppendResult::Appended { .. }));
        let duplicate = apply_command(
            &store,
            command(
                "task.completion_confirmed",
                "project.test",
                "task.test",
                version - 1,
                "p2",
                json!({"digest":digest,"confirmed_by":"owner","channel":"cli"}),
            ),
        )
        .unwrap();
        assert!(matches!(duplicate, AppendResult::Duplicate { .. }));
        apply(
            &store,
            &mut version,
            "finding.opened",
            "new-finding",
            json!({"finding_id":"finding.late","severity":"high","evidence_refs":["evidence.late"]}),
        );
        let rebuilt = fold_task("task.test", &store.load_project("project.test").unwrap());
        assert_eq!(rebuilt.state, "active");
        assert!(!rebuilt.confirmation.unwrap().valid);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn proposal_identity_wins_when_completion_events_share_recorded_at() {
        let (root, store) = store();
        let mut version = 0;
        apply(
            &store,
            &mut version,
            "criterion.accepted",
            "c0",
            json!({"criterion_id":"criterion.one","required":true}),
        );
        apply(
            &store,
            &mut version,
            "criterion.passed",
            "c1",
            json!({"criterion_id":"criterion.one","reviewer":"reviewer","evidence_refs":["evidence.pass.1"]}),
        );
        let first_digest = fold_task("task.test", &store.load_project("project.test").unwrap())
            .completion_digest();
        apply(
            &store,
            &mut version,
            "task.completion_proposed",
            "proposal.1",
            json!({"digest":first_digest}),
        );
        apply(
            &store,
            &mut version,
            "task.completion_confirmed",
            "confirmation.1",
            json!({"digest":first_digest,"confirmed_by":"owner","channel":"desktop_ui"}),
        );
        apply(
            &store,
            &mut version,
            "criterion.passed",
            "c2",
            json!({"criterion_id":"criterion.one","reviewer":"reviewer","evidence_refs":["evidence.pass.2"]}),
        );
        let second_digest = fold_task("task.test", &store.load_project("project.test").unwrap())
            .completion_digest();
        apply(
            &store,
            &mut version,
            "task.completion_proposed",
            "proposal.2",
            json!({"digest":second_digest}),
        );

        let mut events = store.load_project("project.test").unwrap();
        for event in events.iter_mut().filter(|event| {
            matches!(
                event.event_type.as_str(),
                "task.completion_proposed" | "task.completion_confirmed"
            )
        }) {
            event.recorded_at = "2026-07-12T00:00:00.000Z".to_owned();
        }
        let latest_proposal_id = events.last().unwrap().event_id.clone();
        let rebuilt = fold_task("task.test", &events);
        let confirmation = rebuilt.confirmation.unwrap();
        assert_eq!(confirmation.proposal_event_id, latest_proposal_id);
        assert_eq!(confirmation.proposed_at_version, version);
        assert_eq!(confirmation.confirmed_at_version, None);
        assert!(confirmation.valid);
        assert_eq!(rebuilt.state, "completion_pending");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn fixed_soak_rebuilds_100_times_and_recovers_three_hosts() {
        let (root, store) = store();
        let mut version = 0;
        for host in ["codex", "opencode", "claude-code"] {
            for iteration in 0..10 {
                let session_id = format!("session.{host}.{iteration}");
                for (suffix, event_type) in [
                    ("open", "session.opened"),
                    ("gap", "session.gap_detected"),
                    ("recover", "session.recovered"),
                    ("close", "session.closed"),
                ] {
                    let key = format!("{host}.{iteration}.{suffix}");
                    apply(
                        &store,
                        &mut version,
                        event_type,
                        &key,
                        json!({"session_id": session_id, "host": host}),
                    );
                }
            }
        }
        let events = store.load_project("project.test").unwrap();
        let expected = fold_task("task.test", &events);
        assert_eq!(expected.sessions.len(), 30);
        assert!(expected
            .sessions
            .values()
            .all(|session| session.state == "closed" && session.coverage == "complete"));
        for _ in 0..100 {
            assert_eq!(fold_task("task.test", &events), expected);
        }
        let duplicate = apply_command(
            &store,
            command(
                "session.closed",
                "project.test",
                "task.test",
                version - 1,
                "codex.0.close",
                json!({"session_id":"session.codex.0","host":"codex"}),
            ),
        )
        .unwrap();
        assert!(matches!(duplicate, AppendResult::Duplicate { .. }));
        assert_eq!(store.load_project("project.test").unwrap().len(), 120);
        fs::remove_dir_all(root).unwrap();
    }
}

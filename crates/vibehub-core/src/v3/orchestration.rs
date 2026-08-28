use super::domain::{
    AppendResult, EventDraft, EvidenceGrade, LeaseId, NodeId, OperationId, ProjectId, SessionId,
    TaskId, V3Error, V3ErrorCategory, V3EventEnvelope, WorktreeId,
};
use super::event_store::V3EventStore;
use super::worktree::WorktreeState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const ORCHESTRATION_EVENT_TYPES: &[&str] = &[
    "worktree.planned",
    "worktree.create_prepared",
    "worktree.ready",
    "worktree.activated",
    "worktree.dirty",
    "worktree.submitted",
    "worktree.integration_prepared",
    "worktree.integrated",
    "worktree.conflicted",
    "worktree.repairing",
    "worktree.abandoned",
    "worktree.cleaned",
    "lease.acquired",
    "lease.heartbeat",
    "lease.released",
    "lease.reclaimed",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaseState {
    Active,
    Released,
    Reclaimed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaseProjection {
    pub lease_id: String,
    pub owner_session_id: String,
    pub generation: u64,
    pub state: LeaseState,
    pub reclaim_challenge: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeProjection {
    pub worktree_id: String,
    pub node_id: String,
    pub version: u64,
    pub state: WorktreeState,
    pub read_only: bool,
    pub eligibility_digest: String,
    pub lease: Option<LeaseProjection>,
    pub last_operation_id: Option<String>,
    pub event_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrchestrationProjection {
    pub task_id: String,
    pub worktrees: BTreeMap<String, WorktreeProjection>,
    pub unknown_event_types: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationCommand {
    pub event_type: String,
    pub project_id: String,
    pub task_id: String,
    pub node_id: String,
    pub worktree_id: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub lease_id: Option<String>,
    #[serde(default)]
    pub operation_id: Option<String>,
    pub eligibility_digest: String,
    #[serde(default)]
    pub lease_generation: Option<u64>,
    pub actor: String,
    pub expected_version: u64,
    pub idempotency_key: String,
    #[serde(default)]
    pub evidence_grade: Option<EvidenceGrade>,
    #[serde(default)]
    pub payload: Value,
}

pub(crate) fn apply_command(
    store: &V3EventStore,
    command: OrchestrationCommand,
) -> Result<AppendResult, V3Error> {
    if !ORCHESTRATION_EVENT_TYPES.contains(&command.event_type.as_str()) {
        return Err(validation(
            "V3_ORCHESTRATION_EVENT_UNSUPPORTED",
            "event type is not part of the M5 orchestration catalog",
        ));
    }
    validate_identity(&command)?;
    if store
        .event_by_idempotency_key(&command.project_id, &command.idempotency_key)?
        .is_some_and(|event| {
            event.event_type == command.event_type
                && event.worktree_id.as_ref().map(|id| id.0.as_str())
                    == Some(command.worktree_id.as_str())
        })
    {
        return store.append_with_rebuild(command_draft(command));
    }
    let projection = store.orchestration_projection(&command.project_id, &command.task_id)?;
    validate_transition(projection.worktrees.get(&command.worktree_id), &command)?;
    store.append_with_rebuild(command_draft(command))
}

pub fn fold_task(task_id: &str, events: &[V3EventEnvelope]) -> OrchestrationProjection {
    let mut projection = OrchestrationProjection {
        task_id: task_id.to_owned(),
        worktrees: BTreeMap::new(),
        unknown_event_types: Vec::new(),
    };
    for event in events.iter().filter(|event| event.task_id.0 == task_id) {
        if !ORCHESTRATION_EVENT_TYPES.contains(&event.event_type.as_str()) {
            continue;
        }
        let (Some(worktree_id), Some(node_id)) = (&event.worktree_id, &event.node_id) else {
            projection
                .unknown_event_types
                .push(event.event_type.clone());
            continue;
        };
        if event.event_type == "worktree.planned" {
            projection.worktrees.insert(
                worktree_id.0.clone(),
                WorktreeProjection {
                    worktree_id: worktree_id.0.clone(),
                    node_id: node_id.0.clone(),
                    version: event.aggregate_version,
                    state: WorktreeState::Planned,
                    read_only: event
                        .payload
                        .get("read_only")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    eligibility_digest: event
                        .payload
                        .get("eligibility_digest")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                    lease: None,
                    last_operation_id: event.operation_id.as_ref().map(|id| id.0.clone()),
                    event_ids: vec![event.event_id.clone()],
                },
            );
            continue;
        }
        let Some(worktree) = projection.worktrees.get_mut(&worktree_id.0) else {
            projection
                .unknown_event_types
                .push(event.event_type.clone());
            continue;
        };
        worktree.version = event.aggregate_version;
        worktree.event_ids.push(event.event_id.clone());
        if let Some(operation_id) = &event.operation_id {
            worktree.last_operation_id = Some(operation_id.0.clone());
        }
        if let Some(state) = target_state(&event.event_type) {
            worktree.state = state;
        }
        apply_lease_event(worktree, event);
    }
    projection
}

fn validate_identity(command: &OrchestrationCommand) -> Result<(), V3Error> {
    for (name, value) in [
        ("project_id", command.project_id.as_str()),
        ("task_id", command.task_id.as_str()),
        ("node_id", command.node_id.as_str()),
        ("worktree_id", command.worktree_id.as_str()),
        ("actor", command.actor.as_str()),
        ("idempotency_key", command.idempotency_key.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(validation(
                "V3_ORCHESTRATION_IDENTITY_INVALID",
                format!("{name} must not be empty"),
            ));
        }
    }
    for (name, value) in [
        ("session_id", command.session_id.as_deref()),
        ("lease_id", command.lease_id.as_deref()),
        ("operation_id", command.operation_id.as_deref()),
    ] {
        if value.is_some_and(|value| value.trim().is_empty()) {
            return Err(validation(
                "V3_ORCHESTRATION_IDENTITY_INVALID",
                format!("{name} must not be blank when provided"),
            ));
        }
    }
    if command.eligibility_digest.len() != 64
        || !command
            .eligibility_digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(validation(
            "V3_ELIGIBILITY_DIGEST_INVALID",
            "eligibility digest must be 64 lowercase hexadecimal characters",
        ));
    }
    if command.operation_id.is_none() {
        return Err(validation(
            "V3_OPERATION_ID_REQUIRED",
            "orchestration events require operation_id",
        ));
    }
    Ok(())
}

fn validate_transition(
    current: Option<&WorktreeProjection>,
    command: &OrchestrationCommand,
) -> Result<(), V3Error> {
    if command.event_type == "worktree.planned" {
        return current.is_none().then_some(()).ok_or_else(|| {
            conflict(
                "V3_WORKTREE_ALREADY_PLANNED",
                "worktree identity already exists",
            )
        });
    }
    let current = current.ok_or_else(|| {
        validation(
            "V3_WORKTREE_NOT_PLANNED",
            "worktree must be planned before lifecycle or lease events",
        )
    })?;
    if current.node_id != command.node_id {
        return Err(validation(
            "V3_WORKTREE_NODE_MISMATCH",
            "worktree identity cannot move between plan nodes",
        ));
    }
    if command.eligibility_digest != current.eligibility_digest {
        return Err(conflict(
            "V3_ELIGIBILITY_DIGEST_STALE",
            "command eligibility digest does not match the planned worktree",
        ));
    }
    if command.event_type.starts_with("lease.") {
        return validate_lease(current, command);
    }
    let target = target_state(&command.event_type).ok_or_else(|| {
        validation(
            "V3_WORKTREE_TRANSITION_UNKNOWN",
            "worktree event does not have a target state",
        )
    })?;
    if !current.state.can_transition_to(target) {
        return Err(conflict(
            "V3_WORKTREE_TRANSITION_INVALID",
            format!("cannot transition from {:?} to {target:?}", current.state),
        ));
    }
    if command.event_type == "worktree.dirty" && current.read_only {
        return Err(conflict(
            "V3_READ_ONLY_WORKTREE_DIRTY",
            "read-only worktrees cannot record writable changes",
        ));
    }
    if command.event_type == "worktree.activated" && !current.read_only {
        let lease_id = command.lease_id.as_deref().ok_or_else(|| {
            validation(
                "V3_LEASE_ID_REQUIRED",
                "writable worktree activation requires lease_id",
            )
        })?;
        let session_id = command.session_id.as_deref().ok_or_else(|| {
            validation(
                "V3_LEASE_OWNER_REQUIRED",
                "writable worktree activation requires session_id",
            )
        })?;
        let lease = active_lease(current, lease_id, session_id)?;
        if command.lease_generation != Some(lease.generation) {
            return Err(conflict(
                "V3_LEASE_GENERATION_MISMATCH",
                "writable worktree activation requires the current lease generation",
            ));
        }
    }
    Ok(())
}

fn validate_lease(
    current: &WorktreeProjection,
    command: &OrchestrationCommand,
) -> Result<(), V3Error> {
    let lease_id = command
        .lease_id
        .as_deref()
        .ok_or_else(|| validation("V3_LEASE_ID_REQUIRED", "lease event requires lease_id"))?;
    let session_id = command.session_id.as_deref().ok_or_else(|| {
        validation(
            "V3_LEASE_OWNER_REQUIRED",
            "lease event requires owner session_id",
        )
    })?;
    let generation = command.lease_generation.ok_or_else(|| {
        validation(
            "V3_LEASE_GENERATION_REQUIRED",
            "lease event requires generation",
        )
    })?;
    match command.event_type.as_str() {
        "lease.acquired" => {
            if current.state != WorktreeState::Ready {
                return Err(conflict(
                    "V3_LEASE_ACQUIRE_STATE_INVALID",
                    "a new lease can only be acquired for a ready worktree",
                ));
            }
            if current.read_only {
                return Err(conflict(
                    "V3_READ_ONLY_LEASE_REFUSED",
                    "read-only worktrees do not receive writable leases",
                ));
            }
            require_next_reclaim_challenge(command)?;
            if current.lease.as_ref().is_some_and(|lease| {
                matches!(lease.state, LeaseState::Active | LeaseState::Reclaimed)
            }) {
                return Err(conflict(
                    "V3_LEASE_ACTIVE",
                    "worktree already has an active lease",
                ));
            }
            let expected = current
                .lease
                .as_ref()
                .map_or(1, |lease| lease.generation + 1);
            if generation != expected {
                return Err(conflict(
                    "V3_LEASE_GENERATION_MISMATCH",
                    format!("expected lease generation {expected}"),
                ));
            }
        }
        "lease.heartbeat" | "lease.released" => {
            let lease = active_lease(current, lease_id, session_id)?;
            if generation != lease.generation {
                return Err(conflict(
                    "V3_LEASE_GENERATION_MISMATCH",
                    "lease generation is stale",
                ));
            }
            if command.event_type == "lease.heartbeat"
                && command
                    .payload
                    .get("next_reclaim_challenge")
                    .is_some_and(|value| value.as_str().is_none_or(|value| value.trim().is_empty()))
            {
                return Err(validation(
                    "V3_LEASE_RECLAIM_CHALLENGE_INVALID",
                    "next reclaim challenge must be a non-empty string",
                ));
            }
        }
        "lease.reclaimed" => {
            let lease = current
                .lease
                .as_ref()
                .ok_or_else(|| conflict("V3_LEASE_NOT_FOUND", "cannot reclaim a missing lease"))?;
            if !matches!(lease.state, LeaseState::Active | LeaseState::Reclaimed) {
                return Err(conflict(
                    "V3_LEASE_RECLAIM_STATE_INVALID",
                    "released leases cannot be reclaimed",
                ));
            }
            if lease.lease_id != lease_id {
                return Err(conflict(
                    "V3_LEASE_ID_MISMATCH",
                    "reclaim must preserve the active lease identity",
                ));
            }
            if generation != lease.generation + 1 {
                return Err(conflict(
                    "V3_LEASE_GENERATION_MISMATCH",
                    "reclaim must increment lease generation",
                ));
            }
            let owner_dead = command
                .payload
                .get("owner_process_confirmed_dead")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let challenge = command
                .payload
                .get("reclaim_challenge")
                .and_then(Value::as_str);
            if !owner_dead
                || challenge.is_none_or(|challenge| challenge.trim().is_empty())
                || challenge != lease.reclaim_challenge.as_deref()
            {
                return Err(conflict(
                    "V3_LEASE_RECLAIM_EVIDENCE_REQUIRED",
                    "reclaim requires dead-process evidence and the active challenge",
                ));
            }
            require_next_reclaim_challenge(command)?;
        }
        _ => {}
    }
    Ok(())
}

fn require_next_reclaim_challenge(command: &OrchestrationCommand) -> Result<(), V3Error> {
    if command
        .payload
        .get("next_reclaim_challenge")
        .and_then(Value::as_str)
        .is_none_or(|challenge| challenge.trim().is_empty())
    {
        return Err(validation(
            "V3_LEASE_RECLAIM_CHALLENGE_INVALID",
            "lease acquisition and reclaim require a non-empty next challenge",
        ));
    }
    Ok(())
}

fn active_lease<'a>(
    current: &'a WorktreeProjection,
    lease_id: &str,
    session_id: &str,
) -> Result<&'a LeaseProjection, V3Error> {
    let lease = current
        .lease
        .as_ref()
        .filter(|lease| matches!(lease.state, LeaseState::Active | LeaseState::Reclaimed))
        .ok_or_else(|| conflict("V3_LEASE_NOT_ACTIVE", "worktree lease is not active"))?;
    if lease.lease_id != lease_id || lease.owner_session_id != session_id {
        return Err(conflict(
            "V3_LEASE_OWNER_MISMATCH",
            "lease identity or owner session does not match",
        ));
    }
    Ok(lease)
}

fn apply_lease_event(worktree: &mut WorktreeProjection, event: &V3EventEnvelope) {
    let Some(lease_id) = event.lease_id.as_ref() else {
        return;
    };
    let generation = event
        .payload
        .get("lease_generation")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    match event.event_type.as_str() {
        "lease.acquired" | "lease.reclaimed" => {
            worktree.lease = Some(LeaseProjection {
                lease_id: lease_id.0.clone(),
                owner_session_id: event
                    .session_id
                    .as_ref()
                    .map(|id| id.0.clone())
                    .unwrap_or_default(),
                generation,
                state: if event.event_type == "lease.reclaimed" {
                    LeaseState::Reclaimed
                } else {
                    LeaseState::Active
                },
                reclaim_challenge: event
                    .payload
                    .get("next_reclaim_challenge")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            });
        }
        "lease.heartbeat" => {
            if let Some(lease) = &mut worktree.lease {
                lease.reclaim_challenge = event
                    .payload
                    .get("next_reclaim_challenge")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .or_else(|| lease.reclaim_challenge.clone());
            }
        }
        "lease.released" => {
            if let Some(lease) = &mut worktree.lease {
                lease.state = LeaseState::Released;
                lease.reclaim_challenge = None;
            }
        }
        _ => {}
    }
}

fn command_draft(command: OrchestrationCommand) -> EventDraft {
    let mut payload = command.payload;
    payload["eligibility_digest"] = Value::String(command.eligibility_digest);
    if let Some(generation) = command.lease_generation {
        payload["lease_generation"] = Value::from(generation);
    }
    EventDraft {
        event_type: command.event_type,
        aggregate_id: command.worktree_id.clone(),
        expected_version: command.expected_version,
        idempotency_key: command.idempotency_key,
        project_id: ProjectId(command.project_id),
        task_id: TaskId(command.task_id),
        node_id: Some(NodeId(command.node_id)),
        session_id: command.session_id.map(SessionId),
        worktree_id: Some(WorktreeId(command.worktree_id)),
        lease_id: command.lease_id.map(LeaseId),
        operation_id: command.operation_id.map(OperationId),
        actor: command.actor,
        evidence_grade: command
            .evidence_grade
            .unwrap_or(EvidenceGrade::AgentReported),
        occurred_at: None,
        commit_sha: None,
        payload,
    }
}

fn target_state(event_type: &str) -> Option<WorktreeState> {
    Some(match event_type {
        "worktree.planned" => WorktreeState::Planned,
        "worktree.create_prepared" => WorktreeState::Creating,
        "worktree.ready" => WorktreeState::Ready,
        "worktree.activated" => WorktreeState::Active,
        "worktree.dirty" => WorktreeState::Dirty,
        "worktree.submitted" => WorktreeState::Submitted,
        "worktree.integration_prepared" => WorktreeState::Integrating,
        "worktree.integrated" => WorktreeState::Integrated,
        "worktree.conflicted" => WorktreeState::Conflicted,
        "worktree.repairing" => WorktreeState::Repairing,
        "worktree.abandoned" => WorktreeState::Abandoned,
        "worktree.cleaned" => WorktreeState::Cleaned,
        _ => return None,
    })
}

fn validation(code: &str, message: impl Into<String>) -> V3Error {
    V3Error::new(code, V3ErrorCategory::Validation, false, message)
}

fn conflict(code: &str, message: impl Into<String>) -> V3Error {
    V3Error::new(code, V3ErrorCategory::VersionConflict, true, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use uuid::Uuid;

    const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn command(event_type: &str, version: u64, key: &str) -> OrchestrationCommand {
        OrchestrationCommand {
            event_type: event_type.to_owned(),
            project_id: "project.test".to_owned(),
            task_id: "task.test".to_owned(),
            node_id: "node.test".to_owned(),
            worktree_id: "worktree.test".to_owned(),
            session_id: Some("session.test".to_owned()),
            lease_id: None,
            operation_id: Some(format!("operation.{key}")),
            eligibility_digest: DIGEST.to_owned(),
            lease_generation: None,
            actor: "codex".to_owned(),
            expected_version: version,
            idempotency_key: key.to_owned(),
            evidence_grade: Some(EvidenceGrade::HardObserved),
            payload: json!({}),
        }
    }

    fn store() -> (std::path::PathBuf, V3EventStore) {
        let root = std::env::temp_dir().join(format!("vibehub-v3-orch-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub")).unwrap();
        let store = V3EventStore::open(&root).unwrap();
        (root, store)
    }

    #[test]
    fn lifecycle_and_rebuild_are_deterministic() {
        let (root, store) = store();
        for (version, event_type) in [
            "worktree.planned",
            "worktree.create_prepared",
            "worktree.ready",
            "lease.acquired",
            "worktree.activated",
            "worktree.dirty",
            "worktree.submitted",
            "worktree.integration_prepared",
            "worktree.integrated",
            "worktree.cleaned",
        ]
        .into_iter()
        .enumerate()
        {
            let mut next = command(event_type, version as u64, &format!("key.{version}"));
            if event_type == "lease.acquired" {
                next.lease_id = Some("lease.test".to_owned());
                next.lease_generation = Some(1);
                next.payload = json!({"next_reclaim_challenge":"challenge.1"});
            } else if event_type == "worktree.activated" {
                next.lease_id = Some("lease.test".to_owned());
                next.lease_generation = Some(1);
            }
            apply_command(&store, next).unwrap();
        }
        let events = store.load_project("project.test").unwrap();
        let first = fold_task("task.test", &events);
        let second = fold_task("task.test", &events);
        assert_eq!(first, second);
        let worktree = &first.worktrees["worktree.test"];
        assert_eq!(worktree.state, WorktreeState::Cleaned);
        assert_eq!(worktree.version, 10);
        assert_eq!(worktree.event_ids.len(), 10);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stale_digest_and_illegal_transition_are_rejected() {
        let (root, store) = store();
        apply_command(&store, command("worktree.planned", 0, "plan")).unwrap();
        let error =
            apply_command(&store, command("worktree.activated", 1, "activate")).unwrap_err();
        assert_eq!(error.code, "V3_WORKTREE_TRANSITION_INVALID");
        let mut stale = command("worktree.create_prepared", 1, "create");
        stale.eligibility_digest = "b".repeat(64);
        let error = apply_command(&store, stale).unwrap_err();
        assert_eq!(error.code, "V3_ELIGIBILITY_DIGEST_STALE");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn lease_generation_and_reclaim_require_process_evidence() {
        let (root, store) = store();
        apply_command(&store, command("worktree.planned", 0, "plan")).unwrap();
        apply_command(
            &store,
            command("worktree.create_prepared", 1, "create.prepare"),
        )
        .unwrap();
        apply_command(&store, command("worktree.ready", 2, "create.ready")).unwrap();
        let mut acquire = command("lease.acquired", 3, "lease.acquire");
        acquire.lease_id = Some("lease.test".to_owned());
        acquire.lease_generation = Some(1);
        acquire.payload = json!({"next_reclaim_challenge":"challenge.1"});
        apply_command(&store, acquire).unwrap();

        let mut reclaim = command("lease.reclaimed", 4, "lease.reclaim");
        reclaim.lease_id = Some("lease.test".to_owned());
        reclaim.lease_generation = Some(2);
        reclaim.payload = json!({"reclaim_challenge":"challenge.1"});
        let error = apply_command(&store, reclaim.clone()).unwrap_err();
        assert_eq!(error.code, "V3_LEASE_RECLAIM_EVIDENCE_REQUIRED");
        reclaim.payload["owner_process_confirmed_dead"] = Value::Bool(true);
        reclaim.payload["next_reclaim_challenge"] = Value::String("challenge.2".to_owned());
        apply_command(&store, reclaim).unwrap();

        let projection = fold_task("task.test", &store.load_project("project.test").unwrap());
        let lease = projection.worktrees["worktree.test"]
            .lease
            .as_ref()
            .unwrap();
        assert_eq!(lease.generation, 2);
        assert_eq!(lease.state, LeaseState::Reclaimed);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn writable_activation_requires_lease_but_read_only_is_explicit_exception() {
        let (root, store) = store();
        apply_command(&store, command("worktree.planned", 0, "write.plan")).unwrap();
        apply_command(
            &store,
            command("worktree.create_prepared", 1, "write.create"),
        )
        .unwrap();
        apply_command(&store, command("worktree.ready", 2, "write.ready")).unwrap();
        let error =
            apply_command(&store, command("worktree.activated", 3, "write.activate")).unwrap_err();
        assert_eq!(error.code, "V3_LEASE_ID_REQUIRED");

        let mut read_only_plan = command("worktree.planned", 0, "read.plan");
        read_only_plan.worktree_id = "worktree.read".to_owned();
        read_only_plan.payload = json!({"read_only":true});
        apply_command(&store, read_only_plan).unwrap();
        let mut read_only_create = command("worktree.create_prepared", 1, "read.create");
        read_only_create.worktree_id = "worktree.read".to_owned();
        apply_command(&store, read_only_create).unwrap();
        let mut read_only_ready = command("worktree.ready", 2, "read.ready");
        read_only_ready.worktree_id = "worktree.read".to_owned();
        apply_command(&store, read_only_ready).unwrap();
        let mut read_only_activate = command("worktree.activated", 3, "read.activate");
        read_only_activate.worktree_id = "worktree.read".to_owned();
        apply_command(&store, read_only_activate).unwrap();
        let mut dirty = command("worktree.dirty", 4, "read.dirty");
        dirty.worktree_id = "worktree.read".to_owned();
        let error = apply_command(&store, dirty).unwrap_err();
        assert_eq!(error.code, "V3_READ_ONLY_WORKTREE_DIRTY");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn lease_transfer_requires_reclaim_and_released_lease_cannot_be_revived() {
        let (root, store) = store();
        apply_command(&store, command("worktree.planned", 0, "plan")).unwrap();
        apply_command(
            &store,
            command("worktree.create_prepared", 1, "create.prepare"),
        )
        .unwrap();
        apply_command(&store, command("worktree.ready", 2, "create.ready")).unwrap();
        let mut acquire = command("lease.acquired", 3, "lease.acquire");
        acquire.lease_id = Some("lease.test".to_owned());
        acquire.lease_generation = Some(1);
        acquire.payload = json!({"next_reclaim_challenge":"challenge.1"});
        apply_command(&store, acquire).unwrap();
        let mut activate = command("worktree.activated", 4, "activate");
        activate.lease_id = Some("lease.test".to_owned());
        activate.lease_generation = Some(1);
        apply_command(&store, activate).unwrap();

        let mut transfer = command("lease.acquired", 5, "lease.transfer");
        transfer.session_id = Some("session.other".to_owned());
        transfer.lease_id = Some("lease.other".to_owned());
        transfer.lease_generation = Some(2);
        assert_eq!(
            apply_command(&store, transfer).unwrap_err().code,
            "V3_LEASE_ACQUIRE_STATE_INVALID"
        );

        let mut release = command("lease.released", 5, "lease.release");
        release.lease_id = Some("lease.test".to_owned());
        release.lease_generation = Some(1);
        apply_command(&store, release).unwrap();
        let projection = fold_task("task.test", &store.load_project("project.test").unwrap());
        assert_eq!(
            projection.worktrees["worktree.test"]
                .lease
                .as_ref()
                .unwrap()
                .reclaim_challenge,
            None
        );

        let mut reclaim = command("lease.reclaimed", 6, "lease.reclaim.released");
        reclaim.lease_id = Some("lease.test".to_owned());
        reclaim.lease_generation = Some(2);
        reclaim.payload = json!({
            "owner_process_confirmed_dead": true,
            "reclaim_challenge": "challenge.1",
            "next_reclaim_challenge": "challenge.2"
        });
        assert_eq!(
            apply_command(&store, reclaim).unwrap_err().code,
            "V3_LEASE_RECLAIM_STATE_INVALID"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn blank_lease_identity_and_reclaim_challenges_are_rejected() {
        let (root, store) = store();
        let mut blank_operation = command("worktree.planned", 0, "blank.operation");
        blank_operation.operation_id = Some("   ".to_owned());
        assert_eq!(
            apply_command(&store, blank_operation).unwrap_err().code,
            "V3_ORCHESTRATION_IDENTITY_INVALID"
        );

        apply_command(&store, command("worktree.planned", 0, "plan")).unwrap();
        apply_command(
            &store,
            command("worktree.create_prepared", 1, "create.prepare"),
        )
        .unwrap();
        apply_command(&store, command("worktree.ready", 2, "create.ready")).unwrap();
        let mut acquire = command("lease.acquired", 3, "lease.acquire.blank");
        acquire.lease_id = Some("lease.test".to_owned());
        acquire.lease_generation = Some(1);
        acquire.payload = json!({"next_reclaim_challenge":""});
        assert_eq!(
            apply_command(&store, acquire).unwrap_err().code,
            "V3_LEASE_RECLAIM_CHALLENGE_INVALID"
        );

        let mut blank_lease = command("lease.acquired", 3, "lease.acquire.identity");
        blank_lease.lease_id = Some(" ".to_owned());
        blank_lease.lease_generation = Some(1);
        blank_lease.payload = json!({"next_reclaim_challenge":"challenge.1"});
        assert_eq!(
            apply_command(&store, blank_lease).unwrap_err().code,
            "V3_ORCHESTRATION_IDENTITY_INVALID"
        );
        fs::remove_dir_all(root).unwrap();
    }
}

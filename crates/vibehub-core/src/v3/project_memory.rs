use super::{
    AppendResult, EventDraft, EvidenceGrade, ProjectId, TaskId, V3Error, V3ErrorCategory,
    V3EventEnvelope, V3EventStore,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const MEMORY_MODEL_VERSION: &str = "v3-memory-1";
pub const MEMORY_EVENT_TYPES: &[&str] = &[
    "memory.created",
    "memory.updated",
    "memory.superseded",
    "memory.verified",
    "memory.archived",
    "memory.promotion_candidate_created",
    "memory.promoted",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub entry_id: String,
    pub kind: String,
    pub scope: Vec<String>,
    pub principal_scope: String,
    pub revision: u64,
    pub status: String,
    pub owner: String,
    pub content: String,
    pub evidence_refs: Vec<String>,
    pub verified_against: Vec<String>,
    pub confidence: u8,
    pub freshness: String,
    pub invalidation: Option<String>,
    pub supersedes: Vec<String>,
    pub injection_policy: String,
    pub sensitivity: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryCommand {
    pub action: String,
    pub project_id: String,
    pub task_id: String,
    pub actor: String,
    pub entry_id: String,
    pub expected_revision: u64,
    pub idempotency_key: String,
    #[serde(default)]
    pub entry: Option<MemoryEntry>,
    #[serde(default)]
    pub details: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryProjection {
    pub model_version: String,
    pub project_id: String,
    pub entries: BTreeMap<String, MemoryEntry>,
    pub event_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryQuery {
    #[serde(default)]
    pub kinds: Vec<String>,
    #[serde(default)]
    pub scope: Vec<String>,
    pub principal_scope: Option<String>,
    #[serde(default)]
    pub include_stale: bool,
    #[serde(default)]
    pub include_disputed: bool,
    #[serde(default = "default_budget")]
    pub token_budget: usize,
}
fn default_budget() -> usize {
    2000
}

pub fn apply_command(
    store: &V3EventStore,
    command: MemoryCommand,
) -> Result<AppendResult, V3Error> {
    let event_type = event_type(&command.action)?;
    if store
        .event_by_idempotency_key(&command.project_id, &command.idempotency_key)?
        .is_some()
    {
        return store.append_with_rebuild(draft(command, event_type));
    }
    let projection = store.project_memory_projection(&command.project_id)?;
    validate(&projection, &command)?;
    store.append_with_rebuild(draft(command, event_type))
}

pub fn fold(project_id: &str, events: &[V3EventEnvelope]) -> MemoryProjection {
    let mut projection = MemoryProjection {
        model_version: MEMORY_MODEL_VERSION.to_owned(),
        project_id: project_id.to_owned(),
        entries: BTreeMap::new(),
        event_ids: Vec::new(),
    };
    for event in events
        .iter()
        .filter(|event| event.project_id.0 == project_id && event.event_type.starts_with("memory."))
    {
        projection.event_ids.push(event.event_id.clone());
        let id = event
            .payload
            .get("entry_id")
            .and_then(Value::as_str)
            .unwrap_or(&event.aggregate_id)
            .to_owned();
        match event.event_type.as_str() {
            "memory.created" | "memory.promotion_candidate_created" => {
                if let Some(mut entry) = decode_entry(&event.payload) {
                    entry.revision = event.aggregate_version;
                    if event.event_type == "memory.promotion_candidate_created" {
                        entry.status = "candidate".to_owned();
                        entry.injection_policy = "never".to_owned();
                    }
                    projection.entries.insert(id, entry);
                }
            }
            "memory.updated" | "memory.verified" | "memory.promoted" => {
                if let Some(mut entry) = decode_entry(&event.payload) {
                    entry.revision = event.aggregate_version;
                    if event.event_type == "memory.verified" {
                        entry.freshness = "fresh".to_owned();
                        entry.status = "active".to_owned();
                    }
                    if event.event_type == "memory.promoted" {
                        entry.status = "active".to_owned();
                    }
                    projection.entries.insert(id, entry);
                }
            }
            "memory.archived" => {
                if let Some(entry) = projection.entries.get_mut(&id) {
                    entry.revision = event.aggregate_version;
                    entry.status = "archived".to_owned();
                    entry.injection_policy = "never".to_owned();
                }
            }
            "memory.superseded" => {
                if let Some(entry) = projection.entries.get_mut(&id) {
                    entry.revision = event.aggregate_version;
                    entry.status = "superseded".to_owned();
                    entry.injection_policy = "never".to_owned();
                }
                if let Some(mut replacement) = event
                    .payload
                    .get("replacement")
                    .cloned()
                    .and_then(|v| serde_json::from_value::<MemoryEntry>(v).ok())
                {
                    replacement.revision = 1;
                    projection
                        .entries
                        .insert(replacement.entry_id.clone(), replacement);
                }
            }
            _ => {}
        }
    }
    mark_stale_and_disputed(&mut projection);
    projection
}

pub fn query(projection: &MemoryProjection, query: &MemoryQuery) -> Vec<MemoryEntry> {
    let mut remaining = query.token_budget;
    let mut entries = projection
        .entries
        .values()
        .filter(|entry| {
            (query.kinds.is_empty() || query.kinds.contains(&entry.kind))
                && query.principal_scope.as_ref().is_none_or(|scope| {
                    entry.principal_scope == *scope
                        || entry.principal_scope == "team"
                        || entry.principal_scope == "project"
                })
                && (query.scope.is_empty()
                    || entry.scope.iter().any(|item| {
                        query.scope.iter().any(|scope| {
                            item == scope || item.starts_with(scope) || scope.starts_with(item)
                        })
                    }))
                && entry.status != "archived"
                && entry.status != "superseded"
                && entry.status != "candidate"
                && (query.include_stale || entry.freshness != "stale")
                && (query.include_disputed || entry.status != "disputed")
                && entry.sensitivity != "secret"
                && entry.injection_policy != "never"
        })
        .cloned()
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| {
        (
            precedence(&entry.kind),
            std::cmp::Reverse(entry.confidence),
            std::cmp::Reverse(entry.revision),
        )
    });
    entries
        .into_iter()
        .take_while(|entry| {
            let cost = entry.content.split_whitespace().count().max(1);
            if cost > remaining {
                false
            } else {
                remaining -= cost;
                true
            }
        })
        .collect()
}

fn validate(projection: &MemoryProjection, command: &MemoryCommand) -> Result<(), V3Error> {
    if command.entry_id.trim().is_empty() || !command.entry_id.starts_with("memory.") {
        return Err(error(
            "V3_MEMORY_ID_INVALID",
            "entry_id must be a stable memory.* id",
        ));
    }
    let current = projection.entries.get(&command.entry_id);
    let revision = current.map(|entry| entry.revision).unwrap_or(0);
    if revision != command.expected_revision {
        return Err(V3Error::new(
            "V3_MEMORY_REVISION_CONFLICT",
            V3ErrorCategory::VersionConflict,
            true,
            "expected memory revision does not match current revision",
        )
        .with_detail("current_revision", revision));
    }
    match command.action.as_str() {
        "create" | "promotion_candidate" if current.is_some() => {
            return Err(error("V3_MEMORY_EXISTS", "memory entry already exists"))
        }
        "create" | "promotion_candidate" | "update" | "verify" | "promote" => validate_entry(
            command
                .entry
                .as_ref()
                .ok_or_else(|| error("V3_MEMORY_ENTRY_REQUIRED", "action requires entry"))?,
        )?,
        "supersede" => {
            if current.is_none() {
                return Err(error("V3_MEMORY_NOT_FOUND", "memory entry does not exist"));
            }
            let replacement: MemoryEntry = command
                .details
                .get("replacement")
                .cloned()
                .and_then(|v| serde_json::from_value(v).ok())
                .ok_or_else(|| {
                    error(
                        "V3_MEMORY_REPLACEMENT_REQUIRED",
                        "supersede requires replacement",
                    )
                })?;
            validate_entry(&replacement)?;
        }
        "archive" if current.is_none() => {
            return Err(error("V3_MEMORY_NOT_FOUND", "memory entry does not exist"))
        }
        _ => {}
    }
    if matches!(command.action.as_str(), "update" | "verify" | "promote") && current.is_none() {
        return Err(error("V3_MEMORY_NOT_FOUND", "memory entry does not exist"));
    }
    if command.action == "promote" && current.is_some_and(|entry| entry.status != "candidate") {
        return Err(error(
            "V3_MEMORY_PROMOTION_CANDIDATE_REQUIRED",
            "only a promotion candidate may be promoted",
        ));
    }
    Ok(())
}

fn validate_entry(entry: &MemoryEntry) -> Result<(), V3Error> {
    if !matches!(
        entry.kind.as_str(),
        "project_profile"
            | "team_profile"
            | "user_preference"
            | "accepted_decision"
            | "curated_knowledge"
            | "implementation_route"
    ) {
        return Err(error(
            "V3_MEMORY_KIND_INVALID",
            "unsupported project memory kind",
        ));
    }
    if !matches!(
        entry.status.as_str(),
        "active" | "candidate" | "disputed" | "stale"
    ) || entry.owner.trim().is_empty()
        || entry.content.trim().is_empty()
        || entry.confidence > 100
    {
        return Err(error(
            "V3_MEMORY_ENTRY_INVALID",
            "memory entry status, owner, content, or confidence is invalid",
        ));
    }
    if entry.kind == "user_preference" && !entry.principal_scope.starts_with("user:") {
        return Err(error(
            "V3_MEMORY_PRINCIPAL_SCOPE_REQUIRED",
            "user preference must be isolated to a user:* principal_scope",
        ));
    }
    if entry.sensitivity == "secret" && entry.injection_policy != "never" {
        return Err(error(
            "V3_MEMORY_SECRET_INJECTION_FORBIDDEN",
            "secret memory must use injection_policy=never",
        ));
    }
    if entry.evidence_refs.is_empty()
        && !matches!(
            entry.kind.as_str(),
            "user_preference" | "project_profile" | "team_profile"
        )
    {
        return Err(error(
            "V3_MEMORY_EVIDENCE_REQUIRED",
            "durable decisions, knowledge, and routes require evidence_refs",
        ));
    }
    Ok(())
}

fn mark_stale_and_disputed(projection: &mut MemoryProjection) {
    let now = Utc::now();
    for entry in projection.entries.values_mut() {
        if entry.status == "active"
            && entry.invalidation.as_deref().is_some_and(|value| {
                DateTime::parse_from_rfc3339(value)
                    .is_ok_and(|time| time.with_timezone(&Utc) <= now)
            })
        {
            entry.freshness = "stale".to_owned();
        }
    }
    let ids = projection.entries.keys().cloned().collect::<Vec<_>>();
    for (i, left_id) in ids.iter().enumerate() {
        for right_id in ids.iter().skip(i + 1) {
            let conflict = {
                let left = &projection.entries[left_id];
                let right = &projection.entries[right_id];
                left.status == "active"
                    && right.status == "active"
                    && left.kind == right.kind
                    && left.principal_scope == right.principal_scope
                    && left.scope == right.scope
                    && left.content != right.content
                    && !left.supersedes.contains(right_id)
                    && !right.supersedes.contains(left_id)
            };
            if conflict {
                projection.entries.get_mut(left_id).unwrap().status = "disputed".to_owned();
                projection.entries.get_mut(right_id).unwrap().status = "disputed".to_owned();
            }
        }
    }
}
fn decode_entry(payload: &Value) -> Option<MemoryEntry> {
    payload
        .get("entry")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
}
fn draft(command: MemoryCommand, event_type: &str) -> EventDraft {
    let payload = serde_json::json!({"entry_id":command.entry_id,"entry":command.entry,"replacement":command.details.get("replacement")});
    EventDraft {
        event_type: event_type.to_owned(),
        aggregate_id: command.entry_id.clone(),
        expected_version: command.expected_revision,
        idempotency_key: command.idempotency_key,
        project_id: ProjectId(command.project_id),
        task_id: TaskId(command.task_id),
        node_id: None,
        session_id: None,
        worktree_id: None,
        lease_id: None,
        operation_id: None,
        actor: command.actor,
        evidence_grade: EvidenceGrade::AgentReported,
        occurred_at: None,
        commit_sha: None,
        payload,
    }
}
fn event_type(action: &str) -> Result<&'static str, V3Error> {
    match action {
        "create" => Ok("memory.created"),
        "update" => Ok("memory.updated"),
        "supersede" => Ok("memory.superseded"),
        "verify" => Ok("memory.verified"),
        "archive" => Ok("memory.archived"),
        "promotion_candidate" => Ok("memory.promotion_candidate_created"),
        "promote" => Ok("memory.promoted"),
        _ => Err(error(
            "V3_MEMORY_ACTION_UNSUPPORTED",
            "unsupported memory action",
        )),
    }
}
fn precedence(kind: &str) -> u8 {
    match kind {
        "accepted_decision" => 1,
        "project_profile" => 2,
        "team_profile" => 3,
        "user_preference" => 4,
        "implementation_route" => 5,
        "curated_knowledge" => 6,
        _ => 9,
    }
}
pub fn memory_precedence(kind: &str) -> u8 {
    precedence(kind)
}
pub fn prompt_injection_suspected(content: &str) -> bool {
    let value = content.to_lowercase();
    [
        "ignore previous",
        "ignore all",
        "system prompt",
        "developer message",
        "execute this",
        "忽略之前",
        "系统提示",
        "开发者消息",
    ]
    .iter()
    .any(|needle| value.contains(needle))
}
fn error(code: &str, message: &str) -> V3Error {
    V3Error::new(code, V3ErrorCategory::Validation, false, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use uuid::Uuid;
    fn entry(id: &str, content: &str) -> MemoryEntry {
        MemoryEntry {
            entry_id: id.to_owned(),
            kind: "accepted_decision".to_owned(),
            scope: vec!["crates/core".to_owned()],
            principal_scope: "team".to_owned(),
            revision: 0,
            status: "active".to_owned(),
            owner: "team".to_owned(),
            content: content.to_owned(),
            evidence_refs: vec!["test:evidence".to_owned()],
            verified_against: vec![],
            confidence: 90,
            freshness: "fresh".to_owned(),
            invalidation: None,
            supersedes: vec![],
            injection_policy: "task_relevant".to_owned(),
            sensitivity: "internal".to_owned(),
            updated_at: "2026-07-28T00:00:00Z".to_owned(),
        }
    }
    #[test]
    fn rebuild_conflict_privacy_and_promotion_are_closed() {
        let root = std::env::temp_dir().join(format!("v3-memory-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub")).unwrap();
        let store = V3EventStore::open(&root).unwrap();
        for (id, content) in [("memory.a", "A"), ("memory.b", "B")] {
            apply_command(
                &store,
                MemoryCommand {
                    action: "create".to_owned(),
                    project_id: "project.test".to_owned(),
                    task_id: "task.test".to_owned(),
                    actor: "test".to_owned(),
                    entry_id: id.to_owned(),
                    expected_revision: 0,
                    idempotency_key: id.to_owned(),
                    entry: Some(entry(id, content)),
                    details: Value::Null,
                },
            )
            .unwrap();
        }
        let projection = fold("project.test", &store.load_project("project.test").unwrap());
        assert!(projection
            .entries
            .values()
            .all(|entry| entry.status == "disputed"));
        assert!(query(&projection, &MemoryQuery::default()).is_empty());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn secret_and_user_scope_fail_closed() {
        let mut e = entry("memory.secret", "token");
        e.sensitivity = "secret".to_owned();
        assert_eq!(
            validate_entry(&e).unwrap_err().code,
            "V3_MEMORY_SECRET_INJECTION_FORBIDDEN"
        );
        e.kind = "user_preference".to_owned();
        e.sensitivity = "internal".to_owned();
        assert_eq!(
            validate_entry(&e).unwrap_err().code,
            "V3_MEMORY_PRINCIPAL_SCOPE_REQUIRED"
        );
    }
}

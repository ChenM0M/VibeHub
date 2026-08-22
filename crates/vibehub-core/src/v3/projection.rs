use super::domain::{V3Error, V3ErrorCategory, V3EventEnvelope};
use super::lifecycle::{self, TaskLifecycleProjection, LIFECYCLE_EVENT_TYPES};
use super::orchestration::{self, WorktreeProjection, ORCHESTRATION_EVENT_TYPES};
use super::project_memory::{self, MemoryProjection, MEMORY_EVENT_TYPES};
use super::routing::SessionTaskBinding;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct V3Projection {
    pub schema_version: String,
    pub model_version: String,
    pub project_id: String,
    pub source_event_ids: Vec<String>,
    pub aggregate_versions: BTreeMap<String, u64>,
    #[serde(default)]
    pub tasks: BTreeMap<String, TaskLifecycleProjection>,
    pub sessions: BTreeMap<String, SessionProjection>,
    #[serde(default)]
    pub session_bindings: BTreeMap<String, SessionTaskBinding>,
    pub worktrees: BTreeMap<String, WorktreeProjection>,
    #[serde(default)]
    pub project_memory: Option<MemoryProjection>,
    pub unknown_event_types: Vec<String>,
    #[serde(default)]
    pub total_event_count: u64,
    #[serde(default)]
    pub last_event_timestamp: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionLogEntry {
    pub kind: String,
    pub actor: String,
    pub occurred_at: String,
    pub details: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentResultEntry {
    pub result_id: String,
    pub status: String,
    pub summary: String,
    pub node_id: Option<String>,
    pub actor: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionProjection {
    pub task_id: String,
    pub state: String,
    pub progress_entries: u64,
    pub risk_entries: u64,
    #[serde(default)]
    pub recovered_gaps: u64,
    #[serde(default)]
    pub entries: Vec<SessionLogEntry>,
    #[serde(default)]
    pub agent_results: Vec<AgentResultEntry>,
}

pub fn session_has_milestone_evidence(session: &SessionProjection) -> bool {
    session.progress_entries > 0
        || session.recovered_gaps > 0
        || (session.risk_entries > 0
            && session
                .agent_results
                .iter()
                .any(|result| result.status == "failed"))
}

pub fn fold(project_id: &str, events: &[V3EventEnvelope]) -> V3Projection {
    let total_event_count = events.len() as u64;
    let last_event_timestamp = events.last().map(|event| event.recorded_at.clone());
    let mut projection = V3Projection {
        schema_version: "1.0".to_owned(),
        model_version: "v3-core-2".to_owned(),
        project_id: project_id.to_owned(),
        source_event_ids: Vec::new(),
        aggregate_versions: BTreeMap::new(),
        tasks: BTreeMap::new(),
        sessions: BTreeMap::new(),
        session_bindings: BTreeMap::new(),
        worktrees: BTreeMap::new(),
        project_memory: None,
        unknown_event_types: Vec::new(),
        total_event_count,
        last_event_timestamp,
    };
    let mut lifecycle_task_ids = BTreeSet::new();
    let mut orchestration_task_ids = BTreeSet::new();
    for event in events {
        projection.source_event_ids.push(event.event_id.clone());
        projection
            .aggregate_versions
            .insert(event.aggregate_id.clone(), event.aggregate_version);
        let is_lifecycle_event = LIFECYCLE_EVENT_TYPES.contains(&event.event_type.as_str());
        let is_known_event = is_lifecycle_event
            || ORCHESTRATION_EVENT_TYPES.contains(&event.event_type.as_str())
            || MEMORY_EVENT_TYPES.contains(&event.event_type.as_str())
            || matches!(
                event.event_type.as_str(),
                "progress.logged" | "risk.logged" | "agent.result_recorded"
            );
        if is_lifecycle_event {
            lifecycle_task_ids.insert(event.task_id.0.clone());
        }
        if ORCHESTRATION_EVENT_TYPES.contains(&event.event_type.as_str()) {
            orchestration_task_ids.insert(event.task_id.0.clone());
            continue;
        }
        let Some(session_id) = event.session_id.as_ref().map(|id| id.0.clone()) else {
            if !is_known_event {
                projection
                    .unknown_event_types
                    .push(event.event_type.clone());
            }
            continue;
        };
        if event.event_type == "session.task_bound" {
            if let Some(binding) = event
                .payload
                .get("binding")
                .cloned()
                .and_then(|value| serde_json::from_value::<SessionTaskBinding>(value).ok())
            {
                projection
                    .session_bindings
                    .insert(session_id.clone(), binding);
            }
        } else if event.event_type == "session.task_unbound" {
            if let Some(binding) = event
                .payload
                .get("binding")
                .cloned()
                .and_then(|value| serde_json::from_value::<SessionTaskBinding>(value).ok())
            {
                projection
                    .session_bindings
                    .insert(session_id.clone(), binding);
            }
        }
        let session = projection
            .sessions
            .entry(session_id)
            .or_insert_with(|| SessionProjection {
                task_id: event.task_id.0.clone(),
                state: "unknown".to_owned(),
                progress_entries: 0,
                risk_entries: 0,
                recovered_gaps: 0,
                entries: Vec::new(),
                agent_results: Vec::new(),
            });
        match event.event_type.as_str() {
            "session.opened" => session.state = "open".to_owned(),
            "session.gap_detected" => session.state = "gapped".to_owned(),
            "session.recovered" => {
                session.state = "open".to_owned();
                session.recovered_gaps += 1;
            }
            "session.closed" => session.state = "closed".to_owned(),
            "progress.logged" | "risk.logged" => {
                let kind = if event.event_type == "progress.logged" {
                    session.progress_entries += 1;
                    "progress"
                } else {
                    session.risk_entries += 1;
                    "risk"
                };
                session.entries.push(SessionLogEntry {
                    kind: kind.to_owned(),
                    actor: event.actor.clone(),
                    occurred_at: event.occurred_at.clone(),
                    details: event.payload.clone(),
                });
            }
            "agent.result_recorded" => session.agent_results.push(AgentResultEntry {
                result_id: event
                    .payload
                    .get("result_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                status: event
                    .payload
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                summary: event
                    .payload
                    .get("summary")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                node_id: event.node_id.as_ref().map(|id| id.0.clone()),
                actor: event.actor.clone(),
                occurred_at: event.occurred_at.clone(),
            }),
            _ if !is_known_event => projection
                .unknown_event_types
                .push(event.event_type.clone()),
            _ => {}
        }
    }
    for task_id in lifecycle_task_ids {
        projection
            .tasks
            .insert(task_id.clone(), lifecycle::fold_task(&task_id, events));
    }
    for task_id in orchestration_task_ids {
        projection
            .worktrees
            .extend(orchestration::fold_task(&task_id, events).worktrees);
    }
    projection.project_memory = Some(project_memory::fold(project_id, events));
    projection
}

pub fn write_atomic(path: &Path, projection: &V3Projection) -> Result<(), V3Error> {
    let parent = path.parent().ok_or_else(|| {
        V3Error::new(
            "V3_PROJECTION_PATH_INVALID",
            V3ErrorCategory::Internal,
            false,
            "projection path has no parent",
        )
    })?;
    fs::create_dir_all(parent).map_err(internal("V3_PROJECTION_CREATE_FAILED"))?;
    let temp = path.with_extension("json.tmp");
    let content = serde_json::to_vec_pretty(projection).map_err(|error| {
        V3Error::new(
            "V3_PROJECTION_SERIALIZE_FAILED",
            V3ErrorCategory::Internal,
            false,
            error.to_string(),
        )
    })?;
    fs::write(&temp, content).map_err(internal("V3_PROJECTION_WRITE_FAILED"))?;
    fs::rename(&temp, path).map_err(internal("V3_PROJECTION_RENAME_FAILED"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v3::domain::{EvidenceGrade, NodeId, SessionId};

    fn event(
        event_id: &str,
        event_type: &str,
        aggregate_id: &str,
        aggregate_version: u64,
        node_id: Option<&str>,
        session_id: Option<&str>,
        payload: Value,
    ) -> V3EventEnvelope {
        V3EventEnvelope {
            event_id: event_id.to_owned(),
            event_type: event_type.to_owned(),
            event_version: "1.0".to_owned(),
            aggregate_id: aggregate_id.to_owned(),
            aggregate_version,
            expected_version: aggregate_version.saturating_sub(1),
            idempotency_key: format!("key.{event_id}"),
            project_id: "project.test".into(),
            task_id: "task.test".into(),
            node_id: node_id.map(|id| NodeId(id.to_owned())),
            session_id: session_id.map(|id| SessionId(id.to_owned())),
            worktree_id: None,
            lease_id: None,
            operation_id: None,
            actor: "agent".to_owned(),
            evidence_grade: EvidenceGrade::AgentReported,
            occurred_at: format!("2026-07-14T00:00:0{aggregate_version}Z"),
            recorded_at: format!("2026-07-14T00:00:0{aggregate_version}Z"),
            commit_sha: None,
            payload,
        }
    }

    #[test]
    fn folds_lifecycle_events_and_session_entries() {
        let events = vec![
            event(
                "added",
                "plan.node_added",
                "task.test",
                1,
                Some("node.test"),
                None,
                serde_json::json!({
                    "node_id": "node.test",
                    "title": "Implement projection",
                    "goal": "Keep lifecycle state visible",
                    "scope": ["projection.rs"],
                    "dependencies": []
                }),
            ),
            event(
                "activated",
                "plan.node_state_changed",
                "task.test",
                2,
                Some("node.test"),
                None,
                serde_json::json!({"node_id": "node.test", "state": "active"}),
            ),
            event(
                "opened",
                "session.opened",
                "session.test",
                1,
                Some("node.test"),
                Some("session.test"),
                serde_json::json!({}),
            ),
            event(
                "progress",
                "progress.logged",
                "session.test",
                2,
                Some("node.test"),
                Some("session.test"),
                serde_json::json!({"details": {"phase": "build"}, "message": "working"}),
            ),
            event(
                "result",
                "agent.result_recorded",
                "session.test",
                3,
                Some("node.test"),
                Some("session.test"),
                serde_json::json!({
                    "result_id": "result.test",
                    "status": "succeeded",
                    "summary": "Projection is visible"
                }),
            ),
        ];

        let projection = fold("project.test", &events);
        let task = &projection.tasks["task.test"];
        assert_eq!(task.nodes["node.test"].state, "active");
        assert_eq!(projection.sessions["session.test"].progress_entries, 1);
        assert_eq!(
            projection.sessions["session.test"].entries[0].kind,
            "progress"
        );
        assert_eq!(
            projection.sessions["session.test"].entries[0].details["details"]["phase"],
            "build"
        );
        assert_eq!(
            projection.sessions["session.test"].agent_results[0].result_id,
            "result.test"
        );
        assert_eq!(
            projection.sessions["session.test"].agent_results[0].summary,
            "Projection is visible"
        );
        assert!(projection.unknown_event_types.is_empty());
    }
}

fn internal(code: &'static str) -> impl FnOnce(std::io::Error) -> V3Error {
    move |error| V3Error::new(code, V3ErrorCategory::Internal, true, error.to_string())
}

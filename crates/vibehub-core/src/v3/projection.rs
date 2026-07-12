use super::domain::{V3Error, V3ErrorCategory, V3EventEnvelope};
use super::orchestration::{self, WorktreeProjection, ORCHESTRATION_EVENT_TYPES};
use serde::{Deserialize, Serialize};
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
    pub sessions: BTreeMap<String, SessionProjection>,
    pub worktrees: BTreeMap<String, WorktreeProjection>,
    pub unknown_event_types: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionProjection {
    pub task_id: String,
    pub state: String,
    pub progress_entries: u64,
    pub risk_entries: u64,
}

pub fn fold(project_id: &str, events: &[V3EventEnvelope]) -> V3Projection {
    let mut projection = V3Projection {
        schema_version: "1.0".to_owned(),
        model_version: "v3-core-1".to_owned(),
        project_id: project_id.to_owned(),
        source_event_ids: Vec::new(),
        aggregate_versions: BTreeMap::new(),
        sessions: BTreeMap::new(),
        worktrees: BTreeMap::new(),
        unknown_event_types: Vec::new(),
    };
    let mut orchestration_task_ids = BTreeSet::new();
    for event in events {
        projection.source_event_ids.push(event.event_id.clone());
        projection
            .aggregate_versions
            .insert(event.aggregate_id.clone(), event.aggregate_version);
        if ORCHESTRATION_EVENT_TYPES.contains(&event.event_type.as_str()) {
            orchestration_task_ids.insert(event.task_id.0.clone());
            continue;
        }
        let Some(session_id) = event.session_id.as_ref().map(|id| id.0.clone()) else {
            projection
                .unknown_event_types
                .push(event.event_type.clone());
            continue;
        };
        let session = projection
            .sessions
            .entry(session_id)
            .or_insert_with(|| SessionProjection {
                task_id: event.task_id.0.clone(),
                state: "unknown".to_owned(),
                progress_entries: 0,
                risk_entries: 0,
            });
        match event.event_type.as_str() {
            "session.opened" => session.state = "open".to_owned(),
            "session.closed" => session.state = "closed".to_owned(),
            "progress.logged" => session.progress_entries += 1,
            "risk.logged" => session.risk_entries += 1,
            _ => projection
                .unknown_event_types
                .push(event.event_type.clone()),
        }
    }
    for task_id in orchestration_task_ids {
        projection
            .worktrees
            .extend(orchestration::fold_task(&task_id, events).worktrees);
    }
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

fn internal(code: &'static str) -> impl FnOnce(std::io::Error) -> V3Error {
    move |error| V3Error::new(code, V3ErrorCategory::Internal, true, error.to_string())
}

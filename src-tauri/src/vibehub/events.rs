use crate::vibehub::current;
use crate::vibehub::util::{
    canonical_initialized_project_root, normalize_path, relative_to_project,
};
use anyhow::{Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RunEventAppendResult {
    pub events_path: String,
}

pub fn append_run_event(
    project_root: impl AsRef<Path>,
    task_id: &str,
    run_id: &str,
    event_type: &str,
    summary: &str,
    details: Value,
) -> Result<RunEventAppendResult> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let events_path = project_root
        .join(".vibehub")
        .join("tasks")
        .join(task_id)
        .join("runs")
        .join(run_id)
        .join("events.jsonl");
    if let Some(parent) = events_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }

    let event = json!({
        "schema_version": 1,
        "generated_at": Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        "generated_by": "vibehub",
        "task_id": task_id,
        "run_id": run_id,
        "event_type": event_type,
        "summary": summary,
        "evidence_grade": "hard_observed",
        "details": details,
    });
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&events_path)
        .with_context(|| format!("Failed to open {}", events_path.display()))?;
    writeln!(
        file,
        "{}",
        serde_json::to_string(&event).context("Failed to serialize VibeHub event")?
    )
    .with_context(|| format!("Failed to write {}", events_path.display()))?;

    Ok(RunEventAppendResult {
        events_path: normalize_path(&relative_to_project(&project_root, &events_path)?),
    })
}

pub fn append_current_run_event(
    project_root: impl AsRef<Path>,
    event_type: &str,
    summary: &str,
    details: Value,
) -> Result<Option<RunEventAppendResult>> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let task = match current::resolve_current_task(&project_root) {
        Ok(task) => task,
        Err(_) => return Ok(None),
    };
    let run = match current::resolve_current_run(&project_root, &task.task_id) {
        Ok(run) => run,
        Err(_) => return Ok(None),
    };
    append_run_event(
        &project_root,
        &task.task_id,
        &run.run_id,
        event_type,
        summary,
        details,
    )
    .map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_project() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("vibehub-events-test-{}", Uuid::new_v4()));
        fs::create_dir_all(path.join(".vibehub/tasks/T-001/runs/R-001")).expect("create");
        path
    }

    #[test]
    fn appends_run_event_jsonl() {
        let project = temp_project();

        let result = append_run_event(
            &project,
            "T-001",
            "R-001",
            "review_evidence_generated",
            "Review evidence generated.",
            json!({"review_path": "phases/review.md"}),
        )
        .expect("append event");

        assert_eq!(
            result.events_path,
            ".vibehub/tasks/T-001/runs/R-001/events.jsonl"
        );
        let content = fs::read_to_string(project.join(&result.events_path)).expect("read");
        assert!(content.contains("\"event_type\":\"review_evidence_generated\""));
        assert!(content.contains("\"evidence_grade\":\"hard_observed\""));

        fs::remove_dir_all(project).expect("cleanup");
    }
}

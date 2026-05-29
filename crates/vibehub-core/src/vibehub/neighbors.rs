use crate::vibehub::util::canonical_initialized_project_root;
use crate::vibehub::{current, events, ownership};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskNeighbor {
    pub task_id: String,
    pub title: Option<String>,
    pub active_capabilities: Vec<String>,
    pub shared_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskNeighborReport {
    pub task_id: String,
    pub neighbors: Vec<TaskNeighbor>,
    pub warnings: Vec<String>,
    pub event_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CurrentRunPointer {
    run_id: String,
}

#[derive(Debug, Deserialize)]
struct RunMetadata {
    #[serde(default)]
    phase: Option<String>,
    #[serde(default)]
    phase_status: Option<String>,
}

pub fn query_current_task_neighbors(project_root: impl AsRef<Path>) -> Result<TaskNeighborReport> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let task = current::resolve_current_task(&project_root)?;
    let run = current::resolve_current_run(&project_root, &task.task_id)?;
    let mut report = query_task_neighbors_for(&project_root, &task.task_id)?;
    report.event_id =
        record_neighbor_conflicts(&project_root, &task.task_id, &run.run_id, &report.neighbors)?;
    Ok(report)
}

pub fn query_task_neighbors_for(
    project_root: impl AsRef<Path>,
    task_id: &str,
) -> Result<TaskNeighborReport> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let active_tasks = active_task_ids_from_state(&project_root)?;
    let active_files = ownership::active_files_by_task(&project_root).unwrap_or_default();
    let current_files = active_files.get(task_id).cloned().unwrap_or_default();
    let mut neighbors = Vec::new();

    for neighbor_task_id in active_tasks.into_iter().filter(|id| id != task_id) {
        let neighbor_files = active_files
            .get(&neighbor_task_id)
            .cloned()
            .unwrap_or_default();
        let shared_files = shared_files(&current_files, &neighbor_files);
        neighbors.push(TaskNeighbor {
            task_id: neighbor_task_id.clone(),
            title: read_task_title(&project_root, &neighbor_task_id),
            active_capabilities: read_active_capabilities(&project_root, &neighbor_task_id),
            shared_files,
        });
    }

    let warnings = neighbor_warnings(&neighbors);
    Ok(TaskNeighborReport {
        task_id: task_id.to_string(),
        neighbors,
        warnings,
        event_id: None,
    })
}

pub fn record_neighbor_conflicts(
    project_root: impl AsRef<Path>,
    task_id: &str,
    run_id: &str,
    neighbors: &[TaskNeighbor],
) -> Result<Option<String>> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let shared_files = neighbors
        .iter()
        .flat_map(|neighbor| neighbor.shared_files.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if shared_files.is_empty() {
        return Ok(None);
    }

    let mut tasks = neighbors
        .iter()
        .filter(|neighbor| !neighbor.shared_files.is_empty())
        .map(|neighbor| neighbor.task_id.clone())
        .collect::<BTreeSet<_>>();
    tasks.insert(task_id.to_string());

    let event = events::append_structured_run_event(
        &project_root,
        task_id,
        run_id,
        events::VibehubEvent::NeighborConflictDetected {
            tasks: tasks.into_iter().collect(),
            shared_files,
        },
    )?;
    Ok(Some(event.event_id))
}

fn active_task_ids_from_state(project_root: &Path) -> Result<Vec<String>> {
    let state_path = project_root.join(".vibehub/state.yaml");
    let content = match fs::read_to_string(&state_path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(error).with_context(|| format!("Failed to read {}", state_path.display()))
        }
    };
    let state: serde_yaml::Value = serde_yaml::from_str(&content)
        .with_context(|| format!("Invalid YAML in {}", state_path.display()))?;
    let mut active = state
        .get("tasks")
        .and_then(|tasks| tasks.get("active"))
        .and_then(serde_yaml::Value::as_sequence)
        .map(|sequence| {
            sequence
                .iter()
                .filter_map(serde_yaml::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    active.sort();
    active.dedup();
    Ok(active)
}

fn shared_files(current: &BTreeSet<String>, neighbor: &BTreeSet<String>) -> Vec<String> {
    current.intersection(neighbor).cloned().collect::<Vec<_>>()
}

fn neighbor_warnings(neighbors: &[TaskNeighbor]) -> Vec<String> {
    neighbors
        .iter()
        .filter(|neighbor| !neighbor.shared_files.is_empty())
        .map(|neighbor| {
            format!(
                "Neighbor task {} shares {} file(s); coordinate before changing shared files.",
                neighbor.task_id,
                neighbor.shared_files.len()
            )
        })
        .collect()
}

fn read_task_title(project_root: &Path, task_id: &str) -> Option<String> {
    let path = project_root
        .join(".vibehub/tasks")
        .join(task_id)
        .join("task.yaml");
    let content = fs::read_to_string(path).ok()?;
    let value: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;
    for key in ["title", "name", "summary"] {
        if let Some(title) = value.get(key).and_then(serde_yaml::Value::as_str) {
            let title = title.trim();
            if !title.is_empty() {
                return Some(title.to_string());
            }
        }
    }
    None
}

fn read_active_capabilities(project_root: &Path, task_id: &str) -> Vec<String> {
    let Some(run_id) = read_current_run_id(project_root, task_id) else {
        return Vec::new();
    };
    let mut active = BTreeSet::<String>::new();
    if let Some((phase, status)) = read_run_phase(project_root, task_id, &run_id) {
        if matches!(status.as_str(), "active" | "running") {
            active.insert(phase);
        }
    }
    if let Ok(stored_events) = events::list_events(project_root, task_id, &run_id, None) {
        for stored in stored_events {
            let Some(event_type) = stored.event.get("event_type").and_then(JsonValue::as_str)
            else {
                continue;
            };
            let payload = stored.event.get("payload").unwrap_or(&JsonValue::Null);
            match event_type {
                "CapabilityClaimed" => {
                    if let Some(capability) = payload.get("capability").and_then(JsonValue::as_str)
                    {
                        active.insert(capability.to_string());
                    }
                }
                "CapabilityReleased" => {
                    if let Some(capability) = payload.get("capability").and_then(JsonValue::as_str)
                    {
                        active.remove(capability);
                    }
                }
                _ => {}
            }
        }
    }
    active.into_iter().collect()
}

fn read_current_run_id(project_root: &Path, task_id: &str) -> Option<String> {
    let path = project_root
        .join(".vibehub/tasks")
        .join(task_id)
        .join("runs/current");
    let content = fs::read_to_string(path).ok()?;
    serde_yaml::from_str::<CurrentRunPointer>(&content)
        .ok()
        .map(|pointer| pointer.run_id)
}

fn read_run_phase(project_root: &Path, task_id: &str, run_id: &str) -> Option<(String, String)> {
    let path = project_root
        .join(".vibehub/tasks")
        .join(task_id)
        .join("runs")
        .join(run_id)
        .join("run.yaml");
    let content = fs::read_to_string(path).ok()?;
    let metadata: RunMetadata = serde_yaml::from_str(&content).ok()?;
    Some((
        metadata.phase.unwrap_or_else(|| "align".to_string()),
        metadata
            .phase_status
            .unwrap_or_else(|| "active".to_string()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vibehub::current::{write_current_run_pointer, write_current_task_pointer};
    use crate::vibehub::events::VibehubEvent;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn temp_project() -> PathBuf {
        let path = std::env::temp_dir().join(format!("vibehub-neighbors-test-{}", Uuid::new_v4()));
        fs::create_dir_all(path.join(".vibehub/tasks/T-1/runs/R-1")).expect("create T-1");
        fs::create_dir_all(path.join(".vibehub/tasks/T-2/runs/R-2")).expect("create T-2");
        fs::write(
            path.join(".vibehub/tasks/T-1/task.yaml"),
            "title: Current\n",
        )
        .expect("write T-1 task");
        fs::write(
            path.join(".vibehub/tasks/T-2/task.yaml"),
            "title: Neighbor\n",
        )
        .expect("write T-2 task");
        fs::write(
            path.join(".vibehub/tasks/T-1/runs/R-1/run.yaml"),
            "phase: implement\nphase_status: active\n",
        )
        .expect("write T-1 run");
        fs::write(
            path.join(".vibehub/tasks/T-2/runs/R-2/run.yaml"),
            "phase: review\nphase_status: active\n",
        )
        .expect("write T-2 run");
        fs::write(
            path.join(".vibehub/state.yaml"),
            "tasks:\n  active:\n    - T-1\n    - T-2\n",
        )
        .expect("write state");
        write_current_task_pointer(&path, "T-1").expect("write task pointer");
        write_current_run_pointer(&path, "T-1", "R-1").expect("write T-1 run pointer");
        write_current_run_pointer(&path, "T-2", "R-2").expect("write T-2 run pointer");
        fs::create_dir_all(path.join(".vibehub/index")).expect("create index");
        fs::write(
            path.join(".vibehub/index/file-ownership.yaml"),
            r#"schema_version: 1
file_ownership:
  - file_path: src/shared.rs
    task_id: T-1
    run_id: R-1
    capability: implement
    source: user_declared
    confidence: 1.0
    active_from_event: evt-1
    active_to_event: null
    first_seen_at: "2026-05-28T00:00:00Z"
    last_seen_at: "2026-05-28T00:00:00Z"
  - file_path: src/shared.rs
    task_id: T-2
    run_id: R-2
    capability: review
    source: user_declared
    confidence: 1.0
    active_from_event: evt-2
    active_to_event: null
    first_seen_at: "2026-05-28T00:00:00Z"
    last_seen_at: "2026-05-28T00:00:00Z"
"#,
        )
        .expect("write ownership");
        events::append_structured_run_event(
            &path,
            "T-2",
            "R-2",
            VibehubEvent::CapabilityClaimed {
                capability: "review".to_string(),
            },
        )
        .expect("append claim");
        path
    }

    #[test]
    fn reports_active_neighbor_with_shared_files() {
        let project = temp_project();

        let report = query_task_neighbors_for(&project, "T-1").expect("query neighbors");

        assert_eq!(report.neighbors.len(), 1);
        assert_eq!(report.neighbors[0].task_id, "T-2");
        assert_eq!(report.neighbors[0].title.as_deref(), Some("Neighbor"));
        assert_eq!(report.neighbors[0].active_capabilities, vec!["review"]);
        assert_eq!(report.neighbors[0].shared_files, vec!["src/shared.rs"]);
        assert!(!report.warnings.is_empty());
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn records_neighbor_conflict_event_when_shared_files_exist() {
        let project = temp_project();
        let report = query_task_neighbors_for(&project, "T-1").expect("query neighbors");

        let event_id = record_neighbor_conflicts(&project, "T-1", "R-1", &report.neighbors)
            .expect("record conflict");

        assert!(event_id.is_some());
        let events = events::list_events(&project, "T-1", "R-1", None).expect("list events");
        assert!(events.iter().any(|event| {
            event.event.get("event_type").and_then(JsonValue::as_str)
                == Some("NeighborConflictDetected")
        }));
        fs::remove_dir_all(project).ok();
    }
}

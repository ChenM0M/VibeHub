use crate::vibehub::util::{canonical_project_root, normalize_path, relative_to_project};
use crate::vibehub::{agent_view, context, current, start_task};
use anyhow::{Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ArchiveViewData {
    pub cards: Vec<ArchivedTaskCard>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ArchivedTaskCard {
    pub task_id: String,
    pub title: Option<String>,
    pub label: String,
    pub summary: String,
    pub status: String,
    pub status_reason: Option<String>,
    pub task_path: String,
    pub run_id: Option<String>,
    pub run_path: Option<String>,
    pub mode: Option<String>,
    pub phase: Option<String>,
    pub phase_status: Option<String>,
    pub updated_at: Option<String>,
    pub event_count: usize,
    pub events: Vec<ArchivedTaskEvent>,
    pub process: Vec<ArchivedProcessStep>,
    pub handoff_artifacts: Vec<ArchiveArtifact>,
    pub output_artifacts: Vec<ArchiveArtifact>,
    pub event_artifacts: Vec<ArchiveArtifact>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ArchivedProcessStep {
    pub name: String,
    pub status: String,
    pub event_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ArchivedTaskEvent {
    pub event_id: Option<String>,
    pub timestamp: Option<String>,
    pub event_type: String,
    pub summary: String,
    pub artifact_path: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ArchiveArtifact {
    pub label: String,
    pub path: String,
    pub exists: bool,
}

#[derive(Debug, Clone)]
struct RunSnapshot {
    run_id: String,
    run_path: PathBuf,
    mode: Option<String>,
    phase: Option<String>,
    phase_status: Option<String>,
    events: Vec<ArchivedTaskEvent>,
    event_artifacts: Vec<ArchiveArtifact>,
    handoff_artifacts: Vec<ArchiveArtifact>,
    output_artifacts: Vec<ArchiveArtifact>,
    event_count: usize,
    updated_at: Option<String>,
    terminal_status: Option<String>,
    status_reason: Option<String>,
    process: Vec<ArchivedProcessStep>,
}

#[derive(Debug, Clone)]
struct CurrentRunSelection {
    run_id: String,
    run_path: PathBuf,
    mode: String,
    phase: String,
    phase_status: String,
}

pub fn read_archive(project_root: impl AsRef<Path>) -> Result<ArchiveViewData> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    let tasks_dir = project_root.join(".vibehub/tasks");
    if !tasks_dir.is_dir() {
        return Ok(ArchiveViewData {
            cards: Vec::new(),
            warnings: Vec::new(),
        });
    }

    let active_tasks = read_active_task_ids(&project_root);
    let mut cards = Vec::new();
    let mut warnings = Vec::new();

    let entries = fs::read_dir(&tasks_dir)
        .with_context(|| format!("Failed to read {}", tasks_dir.display()))?;
    for entry in entries.flatten() {
        let task_dir = entry.path();
        if !task_dir.is_dir() {
            continue;
        }
        let Some(task_id) = task_dir.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let task_yaml_path = task_dir.join("task.yaml");
        if !task_yaml_path.is_file() {
            continue;
        }

        let task_yaml = match read_yaml_value(&task_yaml_path) {
            Ok(value) => value,
            Err(error) => {
                warnings.push(format!(
                    "Failed to read archived task metadata for {task_id}: {error}"
                ));
                continue;
            }
        };

        let title = yaml_string(&task_yaml, &["title"])
            .or_else(|| yaml_string(&task_yaml, &["summary"]))
            .or_else(|| Some(task_id.to_string()));
        let task_status = yaml_string(&task_yaml, &["status"])
            .or_else(|| yaml_string(&task_yaml, &["phase_status"]));
        let task_phase = yaml_string(&task_yaml, &["phase"]);
        let task_mode = yaml_string(&task_yaml, &["mode"]);
        let task_terminal = terminal_status(task_status.as_deref());

        let mut runs = read_run_snapshots(&project_root, task_id, &task_dir, &mut warnings);
        runs.sort_by(|a, b| {
            b.updated_at
                .cmp(&a.updated_at)
                .then_with(|| b.run_id.cmp(&a.run_id))
        });
        let preferred_run = runs
            .iter()
            .find(|run| run.terminal_status.is_some())
            .or_else(|| runs.first());

        let run_terminal = preferred_run.and_then(|run| run.terminal_status.clone());
        let terminal = task_terminal.or(run_terminal);
        let is_active = active_tasks.contains(task_id);
        if terminal.is_none()
            || (is_active && !matches!(terminal.as_deref(), Some("completed" | "cancelled")))
        {
            continue;
        }
        let status = terminal.unwrap_or_else(|| "completed".to_string());

        let run = preferred_run;
        let summary = summary_text(title.as_deref(), task_id);
        let label = short_label(title.as_deref().unwrap_or(task_id));
        let task_path = normalize_path(&relative_to_project(&project_root, &task_yaml_path)?);
        let run_path = run.map(|run| {
            normalize_path(
                &relative_to_project(&project_root, &run.run_path)
                    .unwrap_or_else(|_| run.run_path.clone()),
            )
        });

        cards.push(ArchivedTaskCard {
            task_id: task_id.to_string(),
            title,
            label,
            summary,
            status,
            status_reason: run
                .and_then(|run| run.status_reason.clone())
                .or_else(|| yaml_string(&task_yaml, &["cancel_reason"]))
                .or_else(|| yaml_string(&task_yaml, &["completion_reason"])),
            task_path,
            run_id: run.map(|run| run.run_id.clone()),
            run_path,
            mode: run.and_then(|run| run.mode.clone()).or(task_mode),
            phase: run.and_then(|run| run.phase.clone()).or(task_phase),
            phase_status: run.and_then(|run| run.phase_status.clone()).or(task_status),
            updated_at: run.and_then(|run| run.updated_at.clone()),
            event_count: run.map(|run| run.event_count).unwrap_or(0),
            events: run.map(|run| run.events.clone()).unwrap_or_default(),
            process: run.map(|run| run.process.clone()).unwrap_or_default(),
            handoff_artifacts: run
                .map(|run| run.handoff_artifacts.clone())
                .unwrap_or_default(),
            output_artifacts: run
                .map(|run| run.output_artifacts.clone())
                .unwrap_or_default(),
            event_artifacts: run
                .map(|run| run.event_artifacts.clone())
                .unwrap_or_default(),
        });
    }

    cards.sort_by(|a, b| {
        b.updated_at
            .cmp(&a.updated_at)
            .then_with(|| a.task_id.cmp(&b.task_id))
    });

    Ok(ArchiveViewData { cards, warnings })
}

fn read_run_snapshots(
    project_root: &Path,
    task_id: &str,
    task_dir: &Path,
    warnings: &mut Vec<String>,
) -> Vec<RunSnapshot> {
    let runs_dir = task_dir.join("runs");
    let Ok(entries) = fs::read_dir(runs_dir) else {
        return Vec::new();
    };

    let mut runs = Vec::new();
    for entry in entries.flatten() {
        let run_path = entry.path();
        if !run_path.is_dir() {
            continue;
        }
        let Some(run_id) = run_path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let run_yaml = read_yaml_value(&run_path.join("run.yaml")).unwrap_or(YamlValue::Null);
        let mode = yaml_string(&run_yaml, &["mode"]);
        let phase = yaml_string(&run_yaml, &["phase"]);
        let phase_status = yaml_string(&run_yaml, &["phase_status"]);
        let mut events = Vec::new();
        let mut event_artifacts = Vec::new();
        let mut terminal = terminal_status(phase_status.as_deref());
        let mut reason = None;

        let event_log = run_path.join("events.jsonl");
        if event_log.is_file() {
            let artifact_path = normalize_path(
                &relative_to_project(project_root, &event_log)
                    .unwrap_or_else(|_| event_log.clone()),
            );
            event_artifacts.push(ArchiveArtifact {
                label: "Events".to_string(),
                path: artifact_path.clone(),
                exists: true,
            });
            match read_events(&event_log, &artifact_path) {
                Ok(read) => {
                    for event in &read {
                        if is_cancel_event(&event.event_type) {
                            terminal = Some("cancelled".to_string());
                            reason = Some(event.summary.clone());
                        }
                        if is_completion_event(&event.event_type) {
                            terminal.get_or_insert_with(|| "completed".to_string());
                        }
                    }
                    events = read;
                }
                Err(error) => warnings.push(format!(
                    "Failed to read archived task events for {task_id}/{run_id}: {error}"
                )),
            }
        }

        let handoff_artifacts =
            collect_artifacts(project_root, &run_path.join("handoffs"), "Handoff");
        let mut output_artifacts =
            collect_artifacts(project_root, &run_path.join("outputs"), "Output");
        output_artifacts.extend(collect_artifacts(
            project_root,
            &run_path.join("phases"),
            "Phase output",
        ));

        let updated_at = latest_timestamp(&events).or_else(|| file_modified_iso(&run_path));
        let process = build_process(phase.as_deref(), phase_status.as_deref(), &events);
        let event_count = events.len();

        runs.push(RunSnapshot {
            run_id: run_id.to_string(),
            run_path,
            mode,
            phase,
            phase_status,
            events,
            event_artifacts,
            handoff_artifacts,
            output_artifacts,
            event_count,
            updated_at,
            terminal_status: terminal,
            status_reason: reason,
            process,
        });
    }
    runs
}

fn read_events(path: &Path, artifact_path: &str) -> Result<Vec<ArchivedTaskEvent>> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let mut events = Vec::new();
    for (index, line) in content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .enumerate()
    {
        let raw: JsonValue = serde_json::from_str(line)
            .with_context(|| format!("Invalid JSONL event {} in {}", index + 1, path.display()))?;
        let event_type = json_string(&raw, &["event_type"])
            .or_else(|| json_string(&raw, &["event", "event_type"]))
            .unwrap_or_else(|| "Unknown".to_string());
        let payload = raw.get("payload").or_else(|| raw.get("details"));
        let summary = json_string(&raw, &["summary"])
            .or_else(|| payload.and_then(|payload| json_string(payload, &["summary"])))
            .or_else(|| payload.and_then(|payload| json_string(payload, &["reason"])))
            .or_else(|| capability_event_summary(&event_type, payload))
            .unwrap_or_else(|| event_type.clone());
        events.push(ArchivedTaskEvent {
            event_id: json_string(&raw, &["event_id"]),
            timestamp: json_string(&raw, &["timestamp"])
                .or_else(|| json_string(&raw, &["generated_at"])),
            event_type,
            summary,
            artifact_path: artifact_path.to_string(),
        });
    }
    Ok(events)
}

fn build_process(
    phase: Option<&str>,
    phase_status: Option<&str>,
    events: &[ArchivedTaskEvent],
) -> Vec<ArchivedProcessStep> {
    let mut statuses: BTreeMap<String, String> = BTreeMap::new();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();

    for event in events {
        let name = capability_name_from_summary(&event.summary).unwrap_or_else(|| {
            event
                .summary
                .split_whitespace()
                .last()
                .filter(|value| !value.is_empty())
                .unwrap_or(&event.event_type)
                .trim_matches(|ch: char| ch == '(' || ch == ')' || ch == ':')
                .to_string()
        });
        if event.event_type.contains("CapabilityClaimed") {
            statuses.insert(name.clone(), "active".to_string());
            *counts.entry(name).or_default() += 1;
        } else if event.event_type.contains("CapabilityReleased") {
            let status = if event.summary.contains("completed") {
                "completed"
            } else {
                "released"
            };
            statuses.insert(name.clone(), status.to_string());
            *counts.entry(name).or_default() += 1;
        } else if event.event_type.contains("Phase") {
            let key = phase.unwrap_or("phase").to_string();
            statuses.insert(key.clone(), phase_status.unwrap_or("observed").to_string());
            *counts.entry(key).or_default() += 1;
        }
    }

    if statuses.is_empty() {
        if let Some(phase) = phase {
            statuses.insert(
                phase.to_string(),
                phase_status.unwrap_or("observed").to_string(),
            );
            counts.insert(phase.to_string(), 1);
        }
    }

    statuses
        .into_iter()
        .map(|(name, status)| ArchivedProcessStep {
            event_count: counts.get(&name).copied().unwrap_or(0),
            name,
            status,
        })
        .collect()
}

fn capability_event_summary(event_type: &str, payload: Option<&JsonValue>) -> Option<String> {
    let payload = payload?;
    let capability = json_string(payload, &["capability"])?;
    if event_type.contains("CapabilityClaimed") {
        return Some(format!("Claimed {capability}"));
    }
    if event_type.contains("CapabilityReleased") {
        let outcome = json_string(payload, &["outcome"]).unwrap_or_else(|| "released".to_string());
        return Some(format!("Released {capability} ({outcome})"));
    }
    Some(format!("{event_type}: {capability}"))
}

fn capability_name_from_summary(summary: &str) -> Option<String> {
    summary
        .strip_prefix("Claimed ")
        .or_else(|| summary.strip_prefix("Released "))
        .and_then(|rest| rest.split_whitespace().next())
        .map(|value| value.trim_matches(|ch: char| ch == '(' || ch == ')' || ch == ':'))
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn collect_artifacts(project_root: &Path, dir: &Path, label: &str) -> Vec<ArchiveArtifact> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut artifacts = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let rel = normalize_path(&relative_to_project(project_root, &path).unwrap_or(path.clone()));
        artifacts.push(ArchiveArtifact {
            label: format!("{label}: {file_name}"),
            path: rel,
            exists: true,
        });
    }
    artifacts.sort_by(|a, b| a.path.cmp(&b.path));
    artifacts
}

fn read_active_task_ids(project_root: &Path) -> BTreeSet<String> {
    let state =
        read_yaml_value(&project_root.join(".vibehub/state.yaml")).unwrap_or(YamlValue::Null);
    state
        .get("tasks")
        .and_then(|tasks| tasks.get("active"))
        .and_then(YamlValue::as_sequence)
        .map(|items| {
            items
                .iter()
                .filter_map(YamlValue::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn read_yaml_value(path: &Path) -> Result<YamlValue> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    serde_yaml::from_str(&content).with_context(|| format!("Invalid YAML in {}", path.display()))
}

fn yaml_string(value: &YamlValue, keys: &[&str]) -> Option<String> {
    let mut current = value;
    for key in keys {
        current = current.get(*key)?;
    }
    current
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn json_string(value: &JsonValue, keys: &[&str]) -> Option<String> {
    let mut current = value;
    for key in keys {
        current = current.get(*key)?;
    }
    current
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn terminal_status(value: Option<&str>) -> Option<String> {
    match value
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "completed" | "complete" | "done" => Some("completed".to_string()),
        "cancelled" | "canceled" | "cancelled_task" | "canceled_task" => {
            Some("cancelled".to_string())
        }
        _ => None,
    }
}

fn is_cancel_event(event_type: &str) -> bool {
    event_type.eq_ignore_ascii_case("TaskCancelled")
        || event_type.eq_ignore_ascii_case("task_cancelled")
        || event_type.to_ascii_lowercase().contains("cancel")
}

fn is_completion_event(event_type: &str) -> bool {
    event_type.eq_ignore_ascii_case("TaskCompleted")
        || event_type.eq_ignore_ascii_case("task_completed")
        || event_type.eq_ignore_ascii_case("PhaseCompleted")
        || event_type.eq_ignore_ascii_case("phase_completed")
}

fn latest_timestamp(events: &[ArchivedTaskEvent]) -> Option<String> {
    events
        .iter()
        .filter_map(|event| event.timestamp.clone())
        .max()
}

fn file_modified_iso(path: &Path) -> Option<String> {
    let modified = path.metadata().ok()?.modified().ok()?;
    let datetime: chrono::DateTime<chrono::Utc> = modified.into();
    Some(datetime.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
}

fn short_label(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "Task".to_string();
    }
    let first_word = trimmed.split_whitespace().next().unwrap_or(trimmed);
    let label: String = first_word.chars().take(4).collect();
    if label.chars().count() >= 2 {
        label
    } else {
        trimmed.chars().take(4).collect()
    }
}

fn summary_text(title: Option<&str>, task_id: &str) -> String {
    title
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(task_id)
        .chars()
        .take(120)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vibehub::{current, init};
    use std::fs;
    use uuid::Uuid;

    fn temp_project() -> PathBuf {
        let p = std::env::temp_dir().join(format!("vibehub-archive-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&p).expect("create");
        p
    }

    #[test]
    fn reads_completed_and_cancelled_tasks_only() {
        let project = temp_project();
        fs::create_dir_all(project.join(".vibehub/tasks/T-done/runs/R-1/outputs")).expect("done");
        fs::create_dir_all(project.join(".vibehub/tasks/T-cancel/runs/R-1/handoffs"))
            .expect("cancel");
        fs::create_dir_all(project.join(".vibehub/tasks/T-active/runs/R-1")).expect("active");
        fs::write(
            project.join(".vibehub/state.yaml"),
            "tasks:\n  active:\n    - T-active\n",
        )
        .expect("state");
        fs::write(
            project.join(".vibehub/tasks/T-done/task.yaml"),
            "task_id: T-done\ntitle: Done task\nphase_status: completed\n",
        )
        .expect("done task");
        fs::write(
            project.join(".vibehub/tasks/T-done/runs/R-1/run.yaml"),
            "run_id: R-1\nmode: evidence_drive\nphase: review\nphase_status: completed\n",
        )
        .expect("done run");
        fs::write(
            project.join(".vibehub/tasks/T-done/runs/R-1/events.jsonl"),
            r#"{"event_id":"evt-1","timestamp":"2026-05-28T00:00:00Z","event_type":"PhaseCompleted","payload":{"phase":"review"}}"#,
        )
        .expect("done events");
        fs::write(
            project.join(".vibehub/tasks/T-done/runs/R-1/outputs/output.md"),
            "# Done\n",
        )
        .expect("done output");
        fs::write(
            project.join(".vibehub/tasks/T-cancel/task.yaml"),
            "task_id: T-cancel\ntitle: Cancelled task\nphase_status: active\n",
        )
        .expect("cancel task");
        fs::write(
            project.join(".vibehub/tasks/T-cancel/runs/R-1/run.yaml"),
            "run_id: R-1\nmode: evidence_drive\nphase: implement\nphase_status: active\n",
        )
        .expect("cancel run");
        fs::write(
            project.join(".vibehub/tasks/T-cancel/runs/R-1/events.jsonl"),
            r#"{"event_id":"evt-2","timestamp":"2026-05-28T00:01:00Z","event_type":"TaskCancelled","payload":{"reason":"No longer needed"}}"#,
        )
        .expect("cancel events");
        fs::write(
            project.join(".vibehub/tasks/T-active/task.yaml"),
            "task_id: T-active\ntitle: Active task\nphase_status: active\n",
        )
        .expect("active task");
        fs::write(
            project.join(".vibehub/tasks/T-active/runs/R-1/run.yaml"),
            "run_id: R-1\nmode: evidence_drive\nphase: plan\nphase_status: active\n",
        )
        .expect("active run");

        let archive = read_archive(&project).expect("archive");
        assert_eq!(archive.cards.len(), 2);
        assert!(archive
            .cards
            .iter()
            .any(|card| card.task_id == "T-done" && card.status == "completed"));
        assert!(archive
            .cards
            .iter()
            .any(|card| card.task_id == "T-cancel" && card.status == "cancelled"));
        assert!(!archive.cards.iter().any(|card| card.task_id == "T-active"));
        assert!(archive
            .cards
            .iter()
            .find(|card| card.task_id == "T-done")
            .unwrap()
            .output_artifacts
            .iter()
            .any(|artifact| artifact.path.ends_with("outputs/output.md")));

        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn archive_current_task_switches_pointers_to_remaining_active_task() {
        let project = temp_project();
        init::init_project(&project).expect("init");
        fs::create_dir_all(project.join(".vibehub/tasks/T-done/runs/R-done")).expect("done");
        fs::create_dir_all(project.join(".vibehub/tasks/T-active/runs/R-active")).expect("active");
        fs::write(
            project.join(".vibehub/state.yaml"),
            r#"current:
  mode: guided_drive
  task_id: T-done
  run_id: R-done
  phase: review
  phase_status: completed
tasks:
  active:
    - T-done
    - T-active
"#,
        )
        .expect("state");
        fs::write(
            project.join(".vibehub/tasks/T-done/task.yaml"),
            "task_id: T-done\ntitle: Done task\nmode: guided_drive\nphase: review\nphase_status: completed\n",
        )
        .expect("done task");
        fs::write(
            project.join(".vibehub/tasks/T-done/runs/R-done/run.yaml"),
            "run_id: R-done\nmode: guided_drive\nphase: review\nphase_status: completed\n",
        )
        .expect("done run");
        fs::write(
            project.join(".vibehub/tasks/T-active/task.yaml"),
            "task_id: T-active\ntitle: Active task\nmode: evidence_drive\nphase: plan\nphase_status: active\n",
        )
        .expect("active task");
        fs::write(
            project.join(".vibehub/tasks/T-active/runs/R-active/run.yaml"),
            "run_id: R-active\nmode: evidence_drive\nphase: plan\nphase_status: active\n",
        )
        .expect("active run");
        current::write_current_task_pointer(&project, "T-done").expect("current task");
        current::write_current_run_pointer(&project, "T-done", "R-done").expect("done run ptr");
        current::write_current_run_pointer(&project, "T-active", "R-active")
            .expect("active run ptr");

        let archived = archive_completed_tasks(&project, None).expect("archive");

        assert_eq!(archived.archived_task_ids, vec!["T-done"]);
        assert_eq!(archived.remaining_active, vec!["T-active"]);
        let current_task = current::resolve_current_task(&project).expect("current task");
        let current_run = current::resolve_current_run(&project, "T-active").expect("current run");
        assert_eq!(current_task.task_id, "T-active");
        assert_eq!(current_run.run_id, "R-active");
        assert!(!project.join(".vibehub/tasks/T-done/runs/current").exists());

        let state = read_yaml_value(&project.join(".vibehub/state.yaml")).expect("state");
        assert_eq!(
            yaml_string(&state, &["current", "task_id"]).as_deref(),
            Some("T-active")
        );
        assert_eq!(
            yaml_string(&state, &["current", "run_id"]).as_deref(),
            Some("R-active")
        );
        assert_eq!(
            yaml_string(&state, &["current", "phase_status"]).as_deref(),
            Some("active")
        );
        assert_eq!(
            yaml_string(&state, &["context", "current_pack"]).as_deref(),
            Some(".vibehub/tasks/T-active/runs/R-active/context-packs/plan.md")
        );

        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn archive_last_current_task_clears_current_state_and_pointers() {
        let project = temp_project();
        init::init_project(&project).expect("init");
        fs::create_dir_all(project.join(".vibehub/tasks/T-done/runs/R-done")).expect("done");
        fs::write(
            project.join(".vibehub/state.yaml"),
            r#"current:
  mode: guided_drive
  task_id: T-done
  run_id: R-done
  phase: review
  phase_status: completed
tasks:
  active:
    - T-done
"#,
        )
        .expect("state");
        fs::write(
            project.join(".vibehub/tasks/T-done/task.yaml"),
            "task_id: T-done\ntitle: Done task\nmode: guided_drive\nphase: review\nphase_status: completed\n",
        )
        .expect("done task");
        fs::write(
            project.join(".vibehub/tasks/T-done/runs/R-done/run.yaml"),
            "run_id: R-done\nmode: guided_drive\nphase: review\nphase_status: completed\n",
        )
        .expect("done run");
        current::write_current_task_pointer(&project, "T-done").expect("current task");
        current::write_current_run_pointer(&project, "T-done", "R-done").expect("run ptr");

        let archived = archive_completed_tasks(&project, None).expect("archive");

        assert_eq!(archived.archived_task_ids, vec!["T-done"]);
        assert!(archived.remaining_active.is_empty());
        assert!(!project.join(".vibehub/tasks/current").exists());
        assert!(!project.join(".vibehub/tasks/T-done/runs/current").exists());
        let state = read_yaml_value(&project.join(".vibehub/state.yaml")).expect("state");
        assert!(yaml_string(&state, &["current", "task_id"]).is_none());
        assert_eq!(
            state
                .get("context")
                .and_then(|context| context.get("stale"))
                .and_then(YamlValue::as_bool),
            Some(true)
        );

        fs::remove_dir_all(project).ok();
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ArchiveTaskResult {
    pub archived_count: usize,
    pub archived_task_ids: Vec<String>,
    pub remaining_active: Vec<String>,
}

pub fn archive_completed_tasks(
    project_root: impl AsRef<Path>,
    target_task_id: Option<&str>,
) -> Result<ArchiveTaskResult> {
    use crate::vibehub::util::canonical_project_root;
    let project_root = canonical_project_root(project_root.as_ref())?;
    let state_path = project_root.join(".vibehub/state.yaml");
    if !state_path.is_file() {
        return Ok(ArchiveTaskResult {
            archived_count: 0,
            archived_task_ids: Vec::new(),
            remaining_active: Vec::new(),
        });
    }

    let content = fs::read_to_string(&state_path)
        .with_context(|| format!("Failed to read {}", state_path.display()))?;
    let mut state: YamlValue = serde_yaml::from_str(&content)
        .with_context(|| format!("Invalid YAML in {}", state_path.display()))?;

    let active: Vec<String> = state
        .get("tasks")
        .and_then(|t| t.get("active"))
        .and_then(|a| a.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(serde_yaml::Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default();

    let mut archived = Vec::new();
    let mut remaining = Vec::new();

    for task_id in &active {
        if let Some(target) = target_task_id {
            if task_id != target {
                remaining.push(task_id.clone());
                continue;
            }
        }

        let task_yaml_path = project_root
            .join(".vibehub/tasks")
            .join(task_id)
            .join("task.yaml");
        if !task_yaml_path.is_file() {
            remaining.push(task_id.clone());
            continue;
        }

        let is_terminal = match fs::read_to_string(&task_yaml_path) {
            Ok(content) => match serde_yaml::from_str::<YamlValue>(&content) {
                Ok(value) => is_archivable_task_status(&value),
                Err(_) => false,
            },
            Err(_) => false,
        };

        if is_terminal {
            archived.push(task_id.clone());
        } else {
            remaining.push(task_id.clone());
        }
    }

    let active_value = YamlValue::Sequence(
        remaining
            .iter()
            .map(|id| YamlValue::String(id.clone()))
            .collect(),
    );
    if let Some(tasks) = state.get_mut("tasks") {
        if let Some(mapping) = tasks.as_mapping_mut() {
            mapping.insert(YamlValue::String("active".to_string()), active_value);
        }
    }

    let current_task_id = state
        .get("current")
        .and_then(|c| c.get("task_id"))
        .and_then(YamlValue::as_str)
        .map(ToString::to_string);
    let was_current_archived = current_task_id
        .as_ref()
        .map(|id| archived.contains(id))
        .unwrap_or(false);

    if was_current_archived || active.is_empty() {
        if let Some(next) = remaining.first() {
            set_current_task_state(&project_root, &mut state, next)?;
        } else {
            clear_current_state(&project_root, &mut state, current_task_id.as_deref());
        }
    }

    for task_id in &archived {
        let _ = fs::remove_file(
            project_root
                .join(".vibehub/tasks")
                .join(task_id)
                .join("runs")
                .join("current"),
        );
    }

    let content = serde_yaml::to_string(&state).context("Failed to serialize state.yaml")?;
    fs::write(&state_path, content)
        .with_context(|| format!("Failed to write {}", state_path.display()))?;

    let _ = agent_view::generate_agent_view(&project_root);

    Ok(ArchiveTaskResult {
        archived_count: archived.len(),
        archived_task_ids: archived,
        remaining_active: remaining,
    })
}

fn is_archivable_task_status(task_yaml: &YamlValue) -> bool {
    let status = task_yaml
        .get("status")
        .and_then(YamlValue::as_str)
        .or_else(|| task_yaml.get("phase_status").and_then(YamlValue::as_str))
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    matches!(
        status.as_str(),
        "completed" | "complete" | "done" | "cancelled" | "canceled"
    )
}

fn set_current_task_state(project_root: &Path, state: &mut YamlValue, task_id: &str) -> Result<()> {
    let selection = select_current_run(project_root, task_id)?;
    current::write_current_task_pointer(project_root, task_id)?;
    current::write_current_run_pointer(project_root, task_id, &selection.run_id)?;

    let run_path = normalize_path(&relative_to_project(project_root, &selection.run_path)?);
    let _ = start_task::ensure_context_spec(
        project_root,
        task_id,
        &selection.run_id,
        &selection.phase,
    )?;
    let pack =
        context::build_context_pack(project_root, task_id, &selection.run_id, &selection.phase)
            .with_context(|| {
                format!("Failed to build context pack for remaining active task {task_id}")
            })?;

    set_yaml_string(state, &["current", "mode"], &selection.mode);
    set_yaml_string(state, &["current", "task_id"], task_id);
    set_yaml_string(state, &["current", "run_id"], &selection.run_id);
    set_yaml_string(state, &["current", "phase"], &selection.phase);
    set_yaml_string(state, &["current", "phase_status"], &selection.phase_status);
    set_yaml_null(state, &["current", "session_id"]);
    set_yaml_string(
        state,
        &["current", "event_log_path"],
        &format!("{run_path}/events.jsonl"),
    );
    set_yaml_string(
        state,
        &["pointers", "task_pointer"],
        ".vibehub/tasks/current",
    );
    set_yaml_string(
        state,
        &["pointers", "run_pointer"],
        &format!(".vibehub/tasks/{task_id}/runs/current"),
    );
    set_yaml_string(state, &["context", "current_pack"], &pack.pack_path);
    set_yaml_string(state, &["context", "current_manifest"], &pack.manifest_path);
    set_yaml_bool(state, &["context", "stale"], false);
    set_yaml_string(state, &["context", "generated_by"], "vibehub_backend");
    let (research_required, research_status) = if selection.mode == "evidence_drive" {
        (true, "required")
    } else {
        (false, "skipped")
    };
    set_yaml_bool(state, &["research", "required"], research_required);
    set_yaml_string(state, &["research", "status"], research_status);
    set_yaml_string(
        state,
        &["last_updated"],
        &Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
    );
    set_yaml_string(state, &["resume_hint"], ".vibehub/agent-view/current.md");

    Ok(())
}

fn select_current_run(project_root: &Path, task_id: &str) -> Result<CurrentRunSelection> {
    let run_id = current::resolve_current_run(project_root, task_id)
        .map(|pointer| pointer.run_id)
        .or_else(|_| infer_first_run_id(project_root, task_id))?;
    let run_path = project_root
        .join(".vibehub/tasks")
        .join(task_id)
        .join("runs")
        .join(&run_id);
    let run_yaml = read_yaml_value(&run_path.join("run.yaml"))?;
    Ok(CurrentRunSelection {
        run_id,
        run_path,
        mode: yaml_string(&run_yaml, &["mode"]).unwrap_or_else(|| "guided_drive".to_string()),
        phase: yaml_string(&run_yaml, &["phase"]).unwrap_or_else(|| "align".to_string()),
        phase_status: yaml_string(&run_yaml, &["phase_status"])
            .unwrap_or_else(|| "active".to_string()),
    })
}

fn infer_first_run_id(project_root: &Path, task_id: &str) -> Result<String> {
    let runs_dir = project_root
        .join(".vibehub/tasks")
        .join(task_id)
        .join("runs");
    let mut run_ids = fs::read_dir(&runs_dir)
        .with_context(|| format!("Failed to read {}", runs_dir.display()))?
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.is_dir() && path.join("run.yaml").is_file() {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(ToString::to_string)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    run_ids.sort();
    run_ids.into_iter().next().with_context(|| {
        format!("Cannot select current run for remaining active task {task_id}: no run.yaml found")
    })
}

fn clear_current_state(project_root: &Path, state: &mut YamlValue, current_task_id: Option<&str>) {
    remove_yaml_path(state, &["current", "mode"]);
    remove_yaml_path(state, &["current", "task_id"]);
    remove_yaml_path(state, &["current", "run_id"]);
    remove_yaml_path(state, &["current", "phase"]);
    remove_yaml_path(state, &["current", "phase_status"]);
    remove_yaml_path(state, &["current", "session_id"]);
    remove_yaml_path(state, &["current", "event_log_path"]);
    remove_yaml_path(state, &["pointers", "task_pointer"]);
    remove_yaml_path(state, &["pointers", "run_pointer"]);
    remove_yaml_path(state, &["context", "current_pack"]);
    remove_yaml_path(state, &["context", "current_manifest"]);
    set_yaml_bool(state, &["context", "stale"], true);
    set_yaml_string(
        state,
        &["last_updated"],
        &Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
    );
    let _ = fs::remove_file(project_root.join(".vibehub/tasks/current"));
    if let Some(task_id) = current_task_id {
        let _ = fs::remove_file(
            project_root
                .join(".vibehub/tasks")
                .join(task_id)
                .join("runs")
                .join("current"),
        );
    }
}

fn set_yaml_string(value: &mut YamlValue, path: &[&str], next: &str) {
    set_yaml_value(value, path, YamlValue::String(next.to_string()));
}

fn set_yaml_bool(value: &mut YamlValue, path: &[&str], next: bool) {
    set_yaml_value(value, path, YamlValue::Bool(next));
}

fn set_yaml_null(value: &mut YamlValue, path: &[&str]) {
    set_yaml_value(value, path, YamlValue::Null);
}

fn set_yaml_value(value: &mut YamlValue, path: &[&str], next: YamlValue) {
    if path.is_empty() {
        *value = next;
        return;
    }
    if !matches!(value, YamlValue::Mapping(_)) {
        *value = YamlValue::Mapping(Default::default());
    }
    let mut current = value;
    for key in &path[..path.len() - 1] {
        let mapping = current.as_mapping_mut().expect("mapping value");
        current = mapping
            .entry(YamlValue::String((*key).to_string()))
            .or_insert_with(|| YamlValue::Mapping(Default::default()));
        if !matches!(current, YamlValue::Mapping(_)) {
            *current = YamlValue::Mapping(Default::default());
        }
    }
    let mapping = current.as_mapping_mut().expect("mapping value");
    mapping.insert(YamlValue::String(path[path.len() - 1].to_string()), next);
}

fn remove_yaml_path(value: &mut YamlValue, path: &[&str]) {
    if path.is_empty() {
        return;
    }
    let mut current = value;
    for key in &path[..path.len() - 1] {
        let Some(next) = current.get_mut(*key) else {
            return;
        };
        current = next;
    }
    if let Some(mapping) = current.as_mapping_mut() {
        mapping.remove(YamlValue::String(path[path.len() - 1].to_string()));
    }
}

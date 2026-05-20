use crate::process_util::silent_command;
use crate::vibehub::util::{canonical_project_root, normalize_path};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VibehubCockpitStatus {
    pub project_root: String,
    pub initialized: bool,
    pub current_task_id: Option<String>,
    pub current_task_title: Option<String>,
    pub current_run_id: Option<String>,
    pub current_mode: Option<String>,
    pub current_phase: Option<String>,
    pub phase_status: Option<String>,
    pub git_available: bool,
    pub git_dirty: Option<bool>,
    pub git_changed_files_count: Option<usize>,
    pub context_pack_status: FileStatus,
    pub agent_output_status: FileStatus,
    pub handoff_status: FileStatus,
    pub flow: Vec<FlowPhaseStatus>,
    pub observability_level: Option<String>,
    pub locale: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FlowPhaseStatus {
    pub phase: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FileStatus {
    pub configured: bool,
    pub exists: bool,
    pub stale: Option<bool>,
    pub path: Option<String>,
    pub status: String,
}

#[derive(Debug, Deserialize)]
struct CurrentTaskPointer {
    task_id: String,
}

#[derive(Debug, Deserialize)]
struct CurrentRunPointer {
    run_id: String,
}

pub fn read_cockpit_status(project_root: impl AsRef<Path>) -> Result<VibehubCockpitStatus> {
    let project_root = canonical_project_root(project_root.as_ref())?;

    let vibehub_root = project_root.join(".vibehub");
    let state_path = vibehub_root.join("state.yaml");
    if !vibehub_root.is_dir() {
        return Ok(VibehubCockpitStatus {
            project_root: normalize_path(&project_root),
            initialized: false,
            current_task_id: None,
            current_task_title: None,
            current_run_id: None,
            current_mode: None,
            current_phase: None,
            phase_status: None,
            git_available: is_git_repo(&project_root),
            git_dirty: git_dirty(&project_root),
            git_changed_files_count: git_changed_files_count(&project_root),
            context_pack_status: not_configured_status(),
            agent_output_status: not_configured_status(),
            handoff_status: not_configured_status(),
            flow: Vec::new(),
            observability_level: Some("best_effort".to_string()),
            locale: None,
            warnings: vec![".vibehub directory is missing; run VibeHub init first.".to_string()],
        });
    }

    let state = read_yaml_value(&state_path).ok();
    let mut warnings = Vec::new();
    if state.is_none() {
        warnings.push("Missing or invalid .vibehub/state.yaml.".to_string());
    }
    if let Some(loop_status) = yaml_string(&state, &["loop_detection", "status"]) {
        if loop_status != "normal" {
            warnings.push(format!("Loop detection status: {loop_status}"));
        }
    }
    warnings.extend(yaml_string_sequence(
        &state,
        &["loop_detection", "warnings"],
    ));

    let current_task_id = yaml_string(&state, &["current", "task_id"])
        .or_else(|| read_current_task_pointer(&project_root));
    let current_run_id = yaml_string(&state, &["current", "run_id"])
        .or_else(|| read_current_run_pointer(&project_root, current_task_id.as_deref()));
    let current_mode = yaml_string(&state, &["current", "mode"]);
    let current_phase = yaml_string(&state, &["current", "phase"]);
    let phase_status = yaml_string(&state, &["current", "phase_status"]);
    let flow = read_flow_statuses(
        &state,
        current_mode.as_deref(),
        current_phase.as_deref(),
        phase_status.as_deref(),
    );

    let context_path = yaml_string(&state, &["context", "current_pack"]).or_else(|| {
        match (
            current_task_id.as_deref(),
            current_run_id.as_deref(),
            current_phase.as_deref(),
        ) {
            (Some(task_id), Some(run_id), Some(phase)) => Some(format!(
                ".vibehub/tasks/{task_id}/runs/{run_id}/context-packs/{phase}.md"
            )),
            _ => None,
        }
    });
    let context_stale = yaml_bool(&state, &["context", "stale"]);
    let handoff_path = yaml_string(&state, &["handoff", "current"])
        .or_else(|| Some(".vibehub/agent-view/handoff.md".to_string()));
    let handoff_state = yaml_string(&state, &["handoff", "status"]);
    let agent_output_path = latest_agent_output_path(
        &project_root,
        current_task_id.as_deref(),
        current_run_id.as_deref(),
    );

    Ok(VibehubCockpitStatus {
        project_root: normalize_path(&project_root),
        initialized: true,
        current_task_title: current_task_id
            .as_deref()
            .and_then(|task_id| read_task_title(&project_root, task_id)),
        current_task_id,
        current_run_id,
        current_mode,
        current_phase,
        phase_status,
        git_available: is_git_repo(&project_root),
        git_dirty: git_dirty(&project_root),
        git_changed_files_count: git_changed_files_count(&project_root),
        context_pack_status: file_status(&project_root, context_path, context_stale, None),
        agent_output_status: file_status(
            &project_root,
            agent_output_path,
            None,
            Some("available".to_string()),
        ),
        handoff_status: file_status(&project_root, handoff_path, None, handoff_state),
        flow,
        observability_level: yaml_string(&state, &["observability", "level"])
            .or_else(|| Some("best_effort".to_string())),
        locale: yaml_string(&state, &["preferences", "locale"])
            .or_else(|| yaml_string(&state, &["settings", "locale"]))
            .or_else(|| yaml_string(&state, &["locale"])),
        warnings,
    })
}

fn read_flow_statuses(
    state: &Option<serde_yaml::Value>,
    mode: Option<&str>,
    current_phase: Option<&str>,
    phase_status: Option<&str>,
) -> Vec<FlowPhaseStatus> {
    let phases = match mode.unwrap_or("guided_drive") {
        "yolo_drive" => &["align_lite", "implement", "review_lite"][..],
        "evidence_drive" => &["align", "research", "plan", "implement", "review"][..],
        _ => &["align", "plan", "implement", "review"][..],
    };

    phases
        .iter()
        .map(|phase| {
            let status = yaml_string(state, &["flow", phase])
                .or_else(|| {
                    if current_phase == Some(*phase) {
                        phase_status.map(ToString::to_string)
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| "pending".to_string());
            FlowPhaseStatus {
                phase: (*phase).to_string(),
                status,
            }
        })
        .collect()
}

fn read_yaml_value(path: &Path) -> Result<serde_yaml::Value> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    serde_yaml::from_str(&content).with_context(|| format!("Invalid YAML: {}", path.display()))
}

fn latest_agent_output_path(
    project_root: &Path,
    task_id: Option<&str>,
    run_id: Option<&str>,
) -> Option<String> {
    let task_id = task_id?;
    let run_id = run_id?;
    let run_dir = project_root
        .join(".vibehub")
        .join("tasks")
        .join(task_id)
        .join("runs")
        .join(run_id);
    let mut candidates: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
    let run_output = run_dir.join("outputs").join("output.md");
    if run_output.is_file() {
        candidates.push((
            run_output,
            file_modified(&run_dir.join("outputs").join("output.md")),
        ));
    }

    let sessions_dir = run_dir.join("sessions");
    if let Ok(entries) = fs::read_dir(sessions_dir) {
        for entry in entries.flatten() {
            let output = entry.path().join("output.md");
            if output.is_file() {
                candidates.push((output.clone(), file_modified(&output)));
            }
        }
    }

    candidates.sort_by(|a, b| a.1.cmp(&b.1));
    let latest = candidates.pop()?.0;
    latest
        .strip_prefix(project_root)
        .ok()
        .map(|path| normalize_path(path))
}

fn file_modified(path: &Path) -> std::time::SystemTime {
    path.metadata()
        .and_then(|metadata| metadata.modified())
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
}

fn read_current_task_pointer(project_root: &Path) -> Option<String> {
    let pointer: CurrentTaskPointer = read_yaml_value(&project_root.join(".vibehub/tasks/current"))
        .ok()
        .and_then(|value| serde_yaml::from_value(value).ok())?;
    Some(pointer.task_id)
}

fn read_current_run_pointer(project_root: &Path, task_id: Option<&str>) -> Option<String> {
    let task_id = task_id?;
    let pointer: CurrentRunPointer = read_yaml_value(
        &project_root
            .join(".vibehub")
            .join("tasks")
            .join(task_id)
            .join("runs")
            .join("current"),
    )
    .ok()
    .and_then(|value| serde_yaml::from_value(value).ok())?;
    Some(pointer.run_id)
}

fn read_task_title(project_root: &Path, task_id: &str) -> Option<String> {
    let task_yaml = read_yaml_value(
        &project_root
            .join(".vibehub")
            .join("tasks")
            .join(task_id)
            .join("task.yaml"),
    )
    .ok()?;
    for key in ["title", "name", "summary"] {
        if let Some(value) = task_yaml.get(key).and_then(|value| value.as_str()) {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn yaml_string(state: &Option<serde_yaml::Value>, keys: &[&str]) -> Option<String> {
    let mut current = state.as_ref()?;
    for key in keys {
        current = current.get(*key)?;
    }
    current
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn yaml_bool(state: &Option<serde_yaml::Value>, keys: &[&str]) -> Option<bool> {
    let mut current = state.as_ref()?;
    for key in keys {
        current = current.get(*key)?;
    }
    current.as_bool()
}

fn yaml_string_sequence(state: &Option<serde_yaml::Value>, keys: &[&str]) -> Vec<String> {
    let Some(mut current) = state.as_ref() else {
        return Vec::new();
    };
    for key in keys {
        let Some(next) = current.get(*key) else {
            return Vec::new();
        };
        current = next;
    }
    current
        .as_sequence()
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn file_status(
    project_root: &Path,
    path: Option<String>,
    stale: Option<bool>,
    explicit_status: Option<String>,
) -> FileStatus {
    let Some(path) = path.filter(|value| !value.trim().is_empty()) else {
        return not_configured_status();
    };
    let exists = project_relative_path(project_root, &path)
        .map(|path| path.is_file())
        .unwrap_or(false);
    let status = if !exists {
        "missing".to_string()
    } else if stale == Some(true) {
        "stale".to_string()
    } else {
        explicit_status.unwrap_or_else(|| "available".to_string())
    };

    FileStatus {
        configured: true,
        exists,
        stale,
        path: Some(path),
        status,
    }
}

fn not_configured_status() -> FileStatus {
    FileStatus {
        configured: false,
        exists: false,
        stale: None,
        path: None,
        status: "not_configured".to_string(),
    }
}

fn project_relative_path(project_root: &Path, raw_path: &str) -> Option<PathBuf> {
    let raw = Path::new(raw_path);
    if raw.is_absolute()
        || raw.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return None;
    }
    Some(project_root.join(raw))
}

fn is_git_repo(project_root: &Path) -> bool {
    silent_command("git")
        .arg("-C")
        .arg(project_root)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn git_dirty(project_root: &Path) -> Option<bool> {
    Some(git_changed_files_count(project_root)? > 0)
}

fn git_changed_files_count(project_root: &Path) -> Option<usize> {
    let output = silent_command("git")
        .arg("-C")
        .arg(project_root)
        .args(["status", "--porcelain"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_project() -> PathBuf {
        let path = std::env::temp_dir().join(format!("vibehub-status-test-{}", Uuid::new_v4()));
        fs::create_dir_all(path.join(".vibehub/tasks/T-001/runs/R-001/context-packs"))
            .expect("create temp project");
        fs::create_dir_all(path.join(".vibehub/agent-view")).expect("create agent-view");
        path
    }

    #[test]
    fn reports_uninitialized_project_without_error() {
        let path = std::env::temp_dir().join(format!("vibehub-status-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).expect("create temp project");

        let status = read_cockpit_status(&path).expect("read status");

        assert!(!status.initialized);
        assert_eq!(status.context_pack_status.status, "not_configured");
        assert!(status.flow.is_empty());
        assert!(!status.warnings.is_empty());

        fs::remove_dir_all(path).expect("cleanup");
    }

    #[test]
    fn reads_state_and_file_presence() {
        let project = temp_project();
        fs::write(
            project.join(".vibehub/state.yaml"),
            r#"current:
  mode: guided_drive
  task_id: T-001
  run_id: R-001
  phase: implement
  phase_status: running
context:
  current_pack: ".vibehub/tasks/T-001/runs/R-001/context-packs/implement.md"
  stale: false
handoff:
  current: ".vibehub/agent-view/handoff.md"
  status: available
observability:
  level: best_effort
"#,
        )
        .expect("write state");
        fs::write(
            project.join(".vibehub/tasks/T-001/task.yaml"),
            "title: Build cockpit\n",
        )
        .expect("write task");
        fs::write(
            project.join(".vibehub/tasks/T-001/runs/R-001/context-packs/implement.md"),
            "# Context\n",
        )
        .expect("write context");
        fs::write(
            project.join(".vibehub/agent-view/handoff.md"),
            "# Handoff\n",
        )
        .expect("write handoff");
        fs::create_dir_all(project.join(".vibehub/tasks/T-001/runs/R-001/outputs"))
            .expect("create outputs");
        fs::write(
            project.join(".vibehub/tasks/T-001/runs/R-001/outputs/output.md"),
            "# Output\n",
        )
        .expect("write output");

        let status = read_cockpit_status(&project).expect("read status");

        assert!(status.initialized);
        assert_eq!(status.current_task_id.as_deref(), Some("T-001"));
        assert_eq!(status.current_task_title.as_deref(), Some("Build cockpit"));
        assert_eq!(status.current_phase.as_deref(), Some("implement"));
        assert!(status.context_pack_status.exists);
        assert!(status.agent_output_status.exists);
        assert!(status.handoff_status.exists);
        assert_eq!(
            status.flow,
            vec![
                FlowPhaseStatus {
                    phase: "align".to_string(),
                    status: "pending".to_string()
                },
                FlowPhaseStatus {
                    phase: "plan".to_string(),
                    status: "pending".to_string()
                },
                FlowPhaseStatus {
                    phase: "implement".to_string(),
                    status: "running".to_string()
                },
                FlowPhaseStatus {
                    phase: "review".to_string(),
                    status: "pending".to_string()
                },
            ]
        );

        fs::remove_dir_all(project).expect("cleanup");
    }
}

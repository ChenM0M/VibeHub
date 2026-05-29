use crate::vibehub::util::{
    canonical_initialized_project_root, normalize_path, relative_to_project,
};
use crate::vibehub::{
    agent_view, context, current, events, projection, start_task, state_migration,
};
use anyhow::{anyhow, Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TaskSwitchResult {
    pub from_task_id: Option<String>,
    pub from_run_id: Option<String>,
    pub to_task_id: String,
    pub to_run_id: String,
    pub mode: String,
    pub phase: String,
    pub phase_status: String,
    pub context_pack_path: String,
    pub context_manifest_path: String,
    pub active_tasks: Vec<String>,
    pub event_id: Option<String>,
    pub index_path: String,
}

#[derive(Debug, Deserialize)]
struct RunMetadata {
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    phase: Option<String>,
    #[serde(default)]
    phase_status: Option<String>,
}

pub fn switch_task(
    project_root: impl AsRef<Path>,
    task_id: impl AsRef<str>,
) -> Result<TaskSwitchResult> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let to_task_id = validate_id("task_id", task_id.as_ref())?.to_string();
    let task_path = project_root.join(".vibehub/tasks").join(&to_task_id);
    if !task_path.is_dir() {
        return Err(anyhow!(
            "Cannot switch task: task directory does not exist: {}",
            task_path.display()
        ));
    }

    let from_task = current::resolve_current_task(&project_root).ok();
    let from_run = from_task
        .as_ref()
        .and_then(|task| current::resolve_current_run(&project_root, &task.task_id).ok());
    let _ = projection::project_current_run_state(&project_root);

    let to_run = current::resolve_current_run(&project_root, &to_task_id)
        .with_context(|| format!("Cannot switch task: no current run for {to_task_id}"))?;
    let run_path = project_root
        .join(".vibehub/tasks")
        .join(&to_task_id)
        .join("runs")
        .join(&to_run.run_id);
    let run_meta = read_run_metadata(&run_path.join("run.yaml"))?;
    let mode = run_meta.mode.unwrap_or_else(|| "guided_drive".to_string());
    let phase = run_meta.phase.unwrap_or_else(|| "align".to_string());
    let phase_status = run_meta
        .phase_status
        .unwrap_or_else(|| "active".to_string());
    let run_relative = normalize_path(&relative_to_project(&project_root, &run_path)?);

    current::write_current_task_pointer(&project_root, &to_task_id)?;
    current::write_current_run_pointer(&project_root, &to_task_id, &to_run.run_id)?;

    let context_spec =
        start_task::ensure_context_spec(&project_root, &to_task_id, &to_run.run_id, &phase)?;
    let pack = context::build_context_pack(&project_root, &to_task_id, &to_run.run_id, &phase)
        .with_context(|| format!("Failed to load context pack for task {to_task_id}"))?;
    let _ = context_spec;

    let active_tasks = update_state(
        &project_root,
        &to_task_id,
        &to_run.run_id,
        &mode,
        &phase,
        &phase_status,
        &run_relative,
        from_task.as_ref().map(|task| task.task_id.as_str()),
        &pack.pack_path,
        &pack.manifest_path,
    )?;

    let event = events::append_structured_run_event(
        &project_root,
        &to_task_id,
        &to_run.run_id,
        events::VibehubEvent::TaskSwitched {
            from_task_id: from_task.as_ref().map(|task| task.task_id.clone()),
            from_run_id: from_run.as_ref().map(|run| run.run_id.clone()),
            to_task_id: to_task_id.clone(),
            to_run_id: to_run.run_id.clone(),
        },
    )
    .ok();
    let _ = projection::project_current_run_state(&project_root);
    let _ = agent_view::generate_agent_view(&project_root);

    Ok(TaskSwitchResult {
        from_task_id: from_task.map(|task| task.task_id),
        from_run_id: from_run.map(|run| run.run_id),
        to_task_id,
        to_run_id: to_run.run_id,
        mode,
        phase,
        phase_status,
        context_pack_path: pack.pack_path,
        context_manifest_path: pack.manifest_path,
        active_tasks,
        event_id: event.map(|event| event.event_id),
        index_path: ".vibehub/index/task-events.idx".to_string(),
    })
}

fn read_run_metadata(path: &Path) -> Result<RunMetadata> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    serde_yaml::from_str(&content).with_context(|| format!("Invalid YAML in {}", path.display()))
}

fn update_state(
    project_root: &Path,
    task_id: &str,
    run_id: &str,
    mode: &str,
    phase: &str,
    phase_status: &str,
    run_path: &str,
    from_task_id: Option<&str>,
    context_pack_path: &str,
    context_manifest_path: &str,
) -> Result<Vec<String>> {
    let state_path = project_root.join(".vibehub/state.yaml");
    let mut state = if state_path.is_file() {
        let content = fs::read_to_string(&state_path)
            .with_context(|| format!("Failed to read {}", state_path.display()))?;
        serde_yaml::from_str::<Value>(&content)
            .with_context(|| format!("Invalid YAML in {}", state_path.display()))?
    } else {
        Value::Mapping(Mapping::new())
    };

    set_value(
        &mut state,
        &["schema_version"],
        Value::Number(state_migration::CURRENT_SCHEMA_VERSION.into()),
    );
    set_string(&mut state, &["current", "mode"], mode);
    set_string(&mut state, &["current", "task_id"], task_id);
    set_string(&mut state, &["current", "run_id"], run_id);
    set_string(&mut state, &["current", "phase"], phase);
    set_string(&mut state, &["current", "phase_status"], phase_status);
    set_null(&mut state, &["current", "session_id"]);
    set_string(
        &mut state,
        &["current", "event_log_path"],
        &format!("{run_path}/events.jsonl"),
    );
    set_string(
        &mut state,
        &["pointers", "task_pointer"],
        ".vibehub/tasks/current",
    );
    set_string(
        &mut state,
        &["pointers", "run_pointer"],
        &format!(".vibehub/tasks/{task_id}/runs/current"),
    );
    set_string(&mut state, &["context", "current_pack"], context_pack_path);
    set_string(
        &mut state,
        &["context", "current_manifest"],
        context_manifest_path,
    );
    set_bool(&mut state, &["context", "stale"], false);
    set_string(&mut state, &["context", "generated_by"], "vibehub_backend");
    let (research_required, research_status) = if mode == "evidence_drive" {
        (true, "required")
    } else {
        (false, "skipped")
    };
    set_bool(&mut state, &["research", "required"], research_required);
    set_string(&mut state, &["research", "status"], research_status);
    set_string(
        &mut state,
        &["last_updated"],
        &Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
    );
    set_string(
        &mut state,
        &["resume_hint"],
        ".vibehub/agent-view/current.md",
    );

    let active_tasks = active_tasks_with(&state, from_task_id, task_id);
    set_value(
        &mut state,
        &["tasks", "active"],
        Value::Sequence(
            active_tasks
                .iter()
                .map(|task| Value::String(task.clone()))
                .collect(),
        ),
    );

    let content = serde_yaml::to_string(&state).context("Failed to serialize state.yaml")?;
    fs::write(&state_path, content)
        .with_context(|| format!("Failed to write {}", state_path.display()))?;
    Ok(active_tasks)
}

fn active_tasks_with(state: &Value, from_task_id: Option<&str>, to_task_id: &str) -> Vec<String> {
    let mut active = state
        .get("tasks")
        .and_then(|tasks| tasks.get("active"))
        .and_then(Value::as_sequence)
        .map(|sequence| {
            sequence
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for task_id in [from_task_id, Some(to_task_id)].into_iter().flatten() {
        if !active.iter().any(|existing| existing == task_id) {
            active.push(task_id.to_string());
        }
    }
    active
}

fn validate_id<'a>(label: &str, value: &'a str) -> Result<&'a str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("Invalid {label}: value is empty"));
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err(anyhow!(
            "Invalid {label} '{}': only ASCII letters, numbers, '-' and '_' are allowed",
            value
        ));
    }
    Ok(trimmed)
}

fn set_string(value: &mut Value, path: &[&str], next: &str) {
    set_value(value, path, Value::String(next.to_string()));
}

fn set_bool(value: &mut Value, path: &[&str], next: bool) {
    set_value(value, path, Value::Bool(next));
}

fn set_null(value: &mut Value, path: &[&str]) {
    set_value(value, path, Value::Null);
}

fn set_value(value: &mut Value, path: &[&str], next: Value) {
    if path.is_empty() {
        *value = next;
        return;
    }
    if !matches!(value, Value::Mapping(_)) {
        *value = Value::Mapping(Mapping::new());
    }
    let mut current = value;
    for key in &path[..path.len() - 1] {
        let mapping = current.as_mapping_mut().expect("mapping value");
        current = mapping
            .entry(Value::String((*key).to_string()))
            .or_insert_with(|| Value::Mapping(Mapping::new()));
        if !matches!(current, Value::Mapping(_)) {
            *current = Value::Mapping(Mapping::new());
        }
    }
    let mapping = current.as_mapping_mut().expect("mapping value");
    mapping.insert(Value::String(path[path.len() - 1].to_string()), next);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vibehub::{current, init, start_task};
    use uuid::Uuid;

    fn temp_project() -> std::path::PathBuf {
        let project = std::env::temp_dir().join(format!("vibehub-switch-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&project).expect("create temp project");
        init::init_project(&project).expect("init project");
        project
    }

    #[test]
    fn switch_task_updates_current_pointers_and_active_list() {
        let project = temp_project();
        let first = start_task::start_task(&project, Some("First".to_string()), None, None)
            .expect("first task");
        let second = start_task::start_task(&project, Some("Second".to_string()), None, None)
            .expect("second task");

        let result = switch_task(&project, &first.task_id).expect("switch to first");

        assert_eq!(
            result.from_task_id.as_deref(),
            Some(second.task_id.as_str())
        );
        assert_eq!(result.to_task_id, first.task_id);
        assert!(result.active_tasks.contains(&first.task_id));
        assert!(result.active_tasks.contains(&second.task_id));
        assert!(project.join(".vibehub/index/task-events.idx").is_file());
        let current_task = current::resolve_current_task(&project).expect("current task");
        assert_eq!(current_task.task_id, first.task_id);
        fs::remove_dir_all(project).ok();
    }
}

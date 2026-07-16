use crate::vibehub::phase::{STATUS_ACTIVE, STATUS_NEEDS_ACTION, STATUS_PENDING};
use crate::vibehub::util::{
    canonical_initialized_project_root, normalize_path, relative_to_project,
};
use crate::vibehub::{current, events, fitness, workflow};
use anyhow::{anyhow, Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serde_yaml::{Mapping, Value as YamlValue};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const TRACE_REL_PATH: &str = ".vibehub/derivation_trace.yaml";
const STATUS_IDLE: &str = "idle";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DerivedState {
    pub task_id: String,
    pub run_id: String,
    pub mode: String,
    pub event_log_path: String,
    pub current_phase: Option<String>,
    pub current_phase_status: Option<String>,
    pub flow: BTreeMap<String, String>,
    #[serde(default)]
    pub active_capabilities: Vec<String>,
    pub source_event_ids: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectionApplyResult {
    pub state_path: String,
    pub trace_path: String,
    pub derived: DerivedState,
}

pub fn derive_current_run_state(project_root: impl AsRef<Path>) -> Result<DerivedState> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let task = current::resolve_current_task(&project_root)?;
    let run = current::resolve_current_run(&project_root, &task.task_id)?;
    let state = read_state(&project_root)?;
    let mode = yaml_string(&state, &["current", "mode"])
        .ok_or_else(|| anyhow!("No current mode in .vibehub/state.yaml"))?;
    let workflow = workflow::read_workflow_file(&project_root)?;
    let capabilities = workflow::resolve_phase_list(&workflow, &mode)?;
    let events = events::list_events(&project_root, &task.task_id, &run.run_id, None)?;
    let event_log_path = format!(
        ".vibehub/tasks/{}/runs/{}/events.jsonl",
        task.task_id, run.run_id
    );
    Ok(fold_events(
        &state,
        &task.task_id,
        &run.run_id,
        &mode,
        &event_log_path,
        &capabilities,
        events,
    ))
}

pub fn project_current_run_state(project_root: impl AsRef<Path>) -> Result<ProjectionApplyResult> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let derived = derive_current_run_state(&project_root)?;
    apply_derived_state(&project_root, derived)
}

fn fold_events(
    existing_state: &YamlValue,
    task_id: &str,
    run_id: &str,
    mode: &str,
    event_log_path: &str,
    capabilities: &[String],
    stored_events: Vec<events::StoredEvent>,
) -> DerivedState {
    let mut flow = BTreeMap::new();
    for capability in capabilities {
        flow.insert(capability.clone(), STATUS_PENDING.to_string());
    }
    if let Some(existing_flow) = existing_state.get("flow").and_then(YamlValue::as_mapping) {
        for (key, value) in existing_flow {
            if let (Some(key), Some(value)) = (key.as_str(), value.as_str()) {
                flow.entry(key.to_string())
                    .or_insert_with(|| value.to_string());
            }
        }
    }

    let mut current_phase = None;
    let mut active_capabilities = BTreeSet::new();
    let mut source_event_ids = Vec::new();
    let mut saw_projection_event = false;

    for stored in stored_events {
        let event_id = stored.event_id.clone();
        let Some(event_type) = stored.event.get("event_type").and_then(JsonValue::as_str) else {
            continue;
        };
        let payload = stored.event.get("payload").unwrap_or(&JsonValue::Null);
        match event_type {
            "CapabilityClaimed" => {
                if let Some(capability) = payload.get("capability").and_then(JsonValue::as_str) {
                    flow.insert(capability.to_string(), STATUS_ACTIVE.to_string());
                    active_capabilities.insert(capability.to_string());
                    source_event_ids.extend(event_id);
                }
            }
            "CapabilityReleased" => {
                if let (Some(capability), Some(outcome)) = (
                    payload.get("capability").and_then(JsonValue::as_str),
                    payload.get("outcome").and_then(JsonValue::as_str),
                ) {
                    flow.insert(capability.to_string(), outcome.to_string());
                    active_capabilities.remove(capability);
                    source_event_ids.extend(event_id);
                }
            }
            "PhaseProjected" => {
                if let Some(phase) = payload.get("phase").and_then(JsonValue::as_str) {
                    current_phase = Some(phase.to_string());
                    saw_projection_event = true;
                    source_event_ids.extend(event_id);
                }
            }
            "PlanInvalidated" => {
                flow.insert("plan".to_string(), STATUS_NEEDS_ACTION.to_string());
                active_capabilities.remove("plan");
                source_event_ids.extend(event_id);
            }
            "DiffReverted" => {
                flow.insert("implement".to_string(), STATUS_NEEDS_ACTION.to_string());
                active_capabilities.remove("implement");
                source_event_ids.extend(event_id);
            }
            "Legacy" => fold_legacy_phase_event(
                payload,
                &mut flow,
                &mut current_phase,
                &mut active_capabilities,
            ),
            _ => {}
        }
    }

    let mut warnings = Vec::new();
    if !saw_projection_event {
        current_phase = yaml_string(existing_state, &["current", "phase"]);
        if source_event_ids.is_empty() {
            overlay_existing_flow(existing_state, &mut flow);
        }
        if current_phase.is_some() {
            warnings.push("No PhaseProjected event found; projected current.phase falls back to existing state.yaml.".to_string());
        }
    }
    let current_phase_status = current_phase
        .as_deref()
        .and_then(|phase| flow.get(phase).cloned())
        .or_else(|| yaml_string(existing_state, &["current", "phase_status"]))
        .or_else(|| Some(STATUS_IDLE.to_string()));

    DerivedState {
        task_id: task_id.to_string(),
        run_id: run_id.to_string(),
        mode: mode.to_string(),
        event_log_path: event_log_path.to_string(),
        current_phase,
        current_phase_status,
        flow,
        active_capabilities: active_capabilities.into_iter().collect(),
        source_event_ids,
        warnings,
    }
}

fn overlay_existing_flow(existing_state: &YamlValue, flow: &mut BTreeMap<String, String>) {
    if let Some(existing_flow) = existing_state.get("flow").and_then(YamlValue::as_mapping) {
        for (key, value) in existing_flow {
            if let (Some(key), Some(value)) = (key.as_str(), value.as_str()) {
                flow.insert(key.to_string(), value.to_string());
            }
        }
    }
}

fn fold_legacy_phase_event(
    payload: &JsonValue,
    flow: &mut BTreeMap<String, String>,
    current_phase: &mut Option<String>,
    active_capabilities: &mut BTreeSet<String>,
) {
    let legacy_type = payload
        .get("legacy_event_type")
        .and_then(JsonValue::as_str)
        .unwrap_or_default();
    let details = payload.get("details").unwrap_or(&JsonValue::Null);
    match legacy_type {
        "phase_status_set" | "phase_completed" | "phase_advance_blocked" => {
            if let (Some(phase), Some(status)) = (
                details.get("phase").and_then(JsonValue::as_str),
                details.get("status").and_then(JsonValue::as_str),
            ) {
                flow.insert(phase.to_string(), status.to_string());
                if status == STATUS_ACTIVE {
                    active_capabilities.insert(phase.to_string());
                } else {
                    active_capabilities.remove(phase);
                }
            }
        }
        "phase_advanced" => {
            if let (Some(previous_phase), Some(previous_status)) = (
                details.get("previous_phase").and_then(JsonValue::as_str),
                details.get("previous_status").and_then(JsonValue::as_str),
            ) {
                flow.insert(previous_phase.to_string(), previous_status.to_string());
                if previous_status == STATUS_ACTIVE {
                    active_capabilities.insert(previous_phase.to_string());
                } else {
                    active_capabilities.remove(previous_phase);
                }
            }
            if let Some(current) = details.get("current_phase").and_then(JsonValue::as_str) {
                *current_phase = Some(current.to_string());
                let status = details
                    .get("current_status")
                    .and_then(JsonValue::as_str)
                    .unwrap_or(STATUS_ACTIVE);
                flow.insert(current.to_string(), status.to_string());
                if status == STATUS_ACTIVE {
                    active_capabilities.insert(current.to_string());
                } else {
                    active_capabilities.remove(current);
                }
            }
        }
        _ => {}
    }
}

fn apply_derived_state(
    project_root: &Path,
    derived: DerivedState,
) -> Result<ProjectionApplyResult> {
    let state_path = project_root.join(".vibehub/state.yaml");
    let mut state = read_state(project_root)?;
    let stored_events = events::list_events(project_root, &derived.task_id, &derived.run_id, None)?;
    let metrics = fitness::compute_metrics(&state, &stored_events);
    set_string(
        &mut state,
        &["current", "event_log_path"],
        &derived.event_log_path,
    );
    if let Some(phase) = derived.current_phase.as_deref() {
        set_string(&mut state, &["current", "phase"], phase);
    }
    if let Some(status) = derived.current_phase_status.as_deref() {
        set_string(&mut state, &["current", "phase_status"], status);
    }
    for (phase, status) in &derived.flow {
        set_string(&mut state, &["flow", phase], status);
    }
    set_bool(&mut state, &["derived", "current", "phase"], true);
    set_bool(&mut state, &["derived", "current", "phase_status"], true);
    set_bool(&mut state, &["derived", "flow"], true);
    set_string(&mut state, &["derived", "trace_path"], TRACE_REL_PATH);
    set_value(&mut state, &["metrics"], fitness::metrics_to_yaml(&metrics));
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    set_string(&mut state, &["last_updated"], &now);
    let serialized = serde_yaml::to_string(&state).context("Failed to serialize state.yaml")?;
    fs::write(&state_path, serialized)
        .with_context(|| format!("Failed to write {}", state_path.display()))?;

    let trace_path = project_root.join(TRACE_REL_PATH);
    write_derivation_trace(&trace_path, &derived, &now)?;

    Ok(ProjectionApplyResult {
        state_path: relative_string(project_root, &state_path),
        trace_path: relative_string(project_root, &trace_path),
        derived,
    })
}

fn write_derivation_trace(path: &Path, derived: &DerivedState, generated_at: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    let mut root = Mapping::new();
    root.insert(
        YamlValue::String("schema_version".to_string()),
        YamlValue::Number(1.into()),
    );
    root.insert(
        YamlValue::String("kind".to_string()),
        YamlValue::String("vibehub_derivation_trace".to_string()),
    );
    root.insert(
        YamlValue::String("generated_at".to_string()),
        YamlValue::String(generated_at.to_string()),
    );
    root.insert(
        YamlValue::String("task_id".to_string()),
        YamlValue::String(derived.task_id.clone()),
    );
    root.insert(
        YamlValue::String("run_id".to_string()),
        YamlValue::String(derived.run_id.clone()),
    );
    root.insert(
        YamlValue::String("event_log_path".to_string()),
        YamlValue::String(derived.event_log_path.clone()),
    );
    root.insert(
        YamlValue::String("source_event_ids".to_string()),
        YamlValue::Sequence(
            derived
                .source_event_ids
                .iter()
                .cloned()
                .map(YamlValue::String)
                .collect(),
        ),
    );
    let derived_yaml =
        serde_yaml::to_value(derived).context("Failed to serialize derived state for trace")?;
    root.insert(YamlValue::String("derived".to_string()), derived_yaml);
    let content = serde_yaml::to_string(&YamlValue::Mapping(root))
        .context("Failed to serialize derivation trace")?;
    fs::write(path, content).with_context(|| format!("Failed to write {}", path.display()))
}

fn read_state(project_root: &Path) -> Result<YamlValue> {
    let state_path = project_root.join(".vibehub/state.yaml");
    let content = fs::read_to_string(&state_path)
        .with_context(|| format!("Failed to read {}", state_path.display()))?;
    serde_yaml::from_str(&content)
        .with_context(|| format!("Invalid YAML in {}", state_path.display()))
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

fn set_string(value: &mut YamlValue, path: &[&str], next: &str) {
    set_value(value, path, YamlValue::String(next.to_string()));
}

fn set_bool(value: &mut YamlValue, path: &[&str], next: bool) {
    set_value(value, path, YamlValue::Bool(next));
}

fn set_value(value: &mut YamlValue, path: &[&str], next: YamlValue) {
    if path.is_empty() {
        *value = next;
        return;
    }
    if !matches!(value, YamlValue::Mapping(_)) {
        *value = YamlValue::Mapping(Mapping::new());
    }
    let mut current = value;
    for key in &path[..path.len() - 1] {
        let mapping = current.as_mapping_mut().expect("mapping value");
        current = mapping
            .entry(YamlValue::String((*key).to_string()))
            .or_insert_with(|| YamlValue::Mapping(Mapping::new()));
        if !matches!(current, YamlValue::Mapping(_)) {
            *current = YamlValue::Mapping(Mapping::new());
        }
    }
    let mapping = current.as_mapping_mut().expect("mapping value");
    mapping.insert(YamlValue::String(path[path.len() - 1].to_string()), next);
}

fn relative_string(project_root: &Path, target: &Path) -> String {
    relative_to_project(project_root, target)
        .map(|path| normalize_path(&path))
        .unwrap_or_else(|_| {
            target
                .strip_prefix(project_root)
                .map(PathBuf::from)
                .map(|path| normalize_path(&path))
                .unwrap_or_else(|_| normalize_path(target))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vibehub::{current, events::VibehubEvent};
    use std::fs;
    use uuid::Uuid;

    fn temp_project() -> PathBuf {
        let path = std::env::temp_dir().join(format!("vibehub-projection-test-{}", Uuid::new_v4()));
        fs::create_dir_all(path.join(".vibehub/tasks/T-001/runs/R-001")).expect("create run");
        fs::write(
            path.join(".vibehub/tasks/T-001/task.yaml"),
            "schema_version: 1\nkind: vibehub_task\ntask_id: T-001\n",
        )
        .expect("write task");
        fs::write(
            path.join(".vibehub/tasks/T-001/runs/R-001/run.yaml"),
            "schema_version: 1\nkind: vibehub_run\ntask_id: T-001\nrun_id: R-001\n",
        )
        .expect("write run");
        current::write_current_task_pointer(&path, "T-001").expect("write task pointer");
        current::write_current_run_pointer(&path, "T-001", "R-001").expect("write run pointer");
        fs::write(
            path.join(".vibehub/workflow.yaml"),
            r#"schema_version: 2
modes:
  evidence_drive:
    phases: [align, research, plan, implement, review]
    capabilities: [align, research, plan, implement, review]
phase_order: [align, research, plan, implement, review]
"#,
        )
        .expect("write workflow");
        fs::write(
            path.join(".vibehub/state.yaml"),
            r#"schema_version: 4
current:
  mode: evidence_drive
  task_id: T-001
  run_id: R-001
  event_log_path: .vibehub/tasks/T-001/runs/R-001/events.jsonl
  phase: align
  phase_status: active
flow:
  align: active
"#,
        )
        .expect("write state");
        path
    }

    #[test]
    fn projects_current_phase_and_flow_from_events() {
        let project = temp_project();
        events::append_structured_run_event(
            &project,
            "T-001",
            "R-001",
            VibehubEvent::CapabilityReleased {
                capability: "align".to_string(),
                outcome: "completed".to_string(),
                task_pack_dirty: false,
            },
        )
        .expect("append release");
        events::append_structured_run_event(
            &project,
            "T-001",
            "R-001",
            VibehubEvent::CapabilityClaimed {
                capability: "research".to_string(),
            },
        )
        .expect("append claim");
        events::append_structured_run_event(
            &project,
            "T-001",
            "R-001",
            VibehubEvent::PhaseProjected {
                phase: "research".to_string(),
                derived_from: vec!["evt-test".to_string()],
            },
        )
        .expect("append projection");

        let result = project_current_run_state(&project).expect("project");

        assert_eq!(result.derived.current_phase.as_deref(), Some("research"));
        assert_eq!(
            result.derived.current_phase_status.as_deref(),
            Some("active")
        );
        assert_eq!(
            result.derived.flow.get("align").map(String::as_str),
            Some("completed")
        );
        assert_eq!(
            result.derived.flow.get("research").map(String::as_str),
            Some("active")
        );
        assert_eq!(result.derived.active_capabilities, vec!["research"]);
        let state = fs::read_to_string(project.join(".vibehub/state.yaml")).expect("read state");
        assert!(state.contains("trace_path: .vibehub/derivation_trace.yaml"));
        assert!(project.join(".vibehub/derivation_trace.yaml").is_file());

        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn compensation_events_remove_active_projection_without_rewriting_history() {
        let project = temp_project();
        events::append_structured_run_event(
            &project,
            "T-001",
            "R-001",
            VibehubEvent::CapabilityClaimed {
                capability: "plan".to_string(),
            },
        )
        .expect("claim plan");
        events::append_structured_run_event(
            &project,
            "T-001",
            "R-001",
            VibehubEvent::PlanInvalidated {
                reason: "requirements changed".to_string(),
            },
        )
        .expect("invalidate plan");

        let before =
            fs::read_to_string(project.join(".vibehub/tasks/T-001/runs/R-001/events.jsonl"))
                .expect("read events before projection");
        let result = project_current_run_state(&project).expect("project");
        let after =
            fs::read_to_string(project.join(".vibehub/tasks/T-001/runs/R-001/events.jsonl"))
                .expect("read events after projection");

        assert_eq!(before, after);
        assert_eq!(
            result.derived.flow.get("plan").map(String::as_str),
            Some(STATUS_NEEDS_ACTION)
        );
        assert!(!result
            .derived
            .active_capabilities
            .contains(&"plan".to_string()));

        fs::remove_dir_all(project).ok();
    }
}

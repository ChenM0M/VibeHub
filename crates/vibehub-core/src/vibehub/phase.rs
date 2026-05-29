use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::vibehub::util::{
    canonical_initialized_project_root, normalize_path, relative_to_project,
};
use crate::vibehub::{agent_view, context, events, handoff, projection, start_task, workflow};

pub const STATUS_PENDING: &str = "pending";
pub const STATUS_ACTIVE: &str = "active";
pub const STATUS_REPORTED: &str = "reported";
pub const STATUS_VALIDATING: &str = "validating";
pub const STATUS_COMPLETED: &str = "completed";
pub const STATUS_NEEDS_ACTION: &str = "needs_action";
pub const STATUS_FAILED: &str = "failed";
pub const STATUS_BLOCKED: &str = "blocked";

const VALID_STATUSES: &[&str] = &[
    STATUS_PENDING,
    STATUS_ACTIVE,
    STATUS_REPORTED,
    STATUS_VALIDATING,
    STATUS_COMPLETED,
    STATUS_NEEDS_ACTION,
    STATUS_FAILED,
    STATUS_BLOCKED,
];

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PhaseValidationResult {
    pub phase: String,
    pub status: String,
    pub required_outputs: Vec<String>,
    pub found_outputs: Vec<String>,
    pub missing_outputs: Vec<String>,
    pub source_output_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PhaseAdvanceResult {
    pub mode: String,
    pub previous_phase: String,
    pub previous_status: String,
    pub current_phase: String,
    pub current_status: String,
    pub next_phase: Option<String>,
    pub validation: PhaseValidationResult,
    /// True when the current phase handoff was complete at the time advance
    /// was attempted. Surfaced for the SOFT handoff gate (Phase B step 8):
    /// `advance_phase` defaults to `force=false`, in which case an incomplete
    /// handoff blocks advance with `current_status="needs_action"` and the
    /// missing section names returned in `missing_handoff_sections`. Pass
    /// `force=true` (via `advance_phase_with_force`) to override.
    #[serde(default)]
    pub handoff_complete: bool,
    /// Sections that were missing in the handoff at the time advance was
    /// attempted. Empty when the handoff was complete OR when no handoff has
    /// been generated yet (in which case `handoff_complete` is `false`).
    #[serde(default)]
    pub missing_handoff_sections: Vec<String>,
    /// True when the advance proceeded only because `force=true` was passed
    /// while the handoff gate would otherwise have blocked it. Frontends can
    /// surface this as an audit hint.
    #[serde(default)]
    pub forced: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HandoffGate {
    complete: bool,
    missing_sections: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PhaseSetResult {
    pub phase: String,
    pub status: String,
    pub mode: String,
    pub current_phase: String,
    pub current_phase_status: String,
}

#[derive(Debug, Deserialize)]
struct PhaseRules {
    #[serde(flatten)]
    phases: BTreeMap<String, PhaseRuleEntry>,
}

#[derive(Debug, Deserialize)]
struct PhaseRuleEntry {
    required_outputs: Option<Vec<String>>,
}

fn read_workflow(project_root: &Path) -> Result<workflow::WorkflowConfig> {
    workflow::read_workflow_file(project_root)
}

fn read_phase_rules(project_root: &Path) -> Result<PhaseRules> {
    let path = project_root.join(".vibehub/rules/phase-rules.yaml");
    if !path.is_file() {
        return Ok(PhaseRules {
            phases: BTreeMap::new(),
        });
    }
    let content =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    serde_yaml::from_str(&content).with_context(|| format!("Invalid YAML in {}", path.display()))
}

fn read_state(project_root: &Path) -> Result<(Value, PathBuf)> {
    let state_path = project_root.join(".vibehub/state.yaml");
    if !state_path.is_file() {
        return Err(anyhow!(
            "VibeHub state.yaml not found at {}",
            state_path.display()
        ));
    }
    let content = fs::read_to_string(&state_path)
        .with_context(|| format!("Failed to read {}", state_path.display()))?;
    let value = serde_yaml::from_str::<Value>(&content)
        .with_context(|| format!("Invalid YAML in {}", state_path.display()))?;
    Ok((value, state_path))
}

fn write_state(state_path: &Path, state: &Value) -> Result<()> {
    let content = serde_yaml::to_string(state).context("Failed to serialize state.yaml")?;
    fs::write(state_path, content)
        .with_context(|| format!("Failed to write {}", state_path.display()))
}

fn yaml_string(value: &Value, keys: &[&str]) -> Option<String> {
    let mut current = value;
    for key in keys {
        current = current.get(*key)?;
    }
    current
        .as_str()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
}

fn get_current_phase(state: &Value) -> Option<String> {
    yaml_string(state, &["current", "phase"])
}

fn get_current_mode(state: &Value) -> Option<String> {
    yaml_string(state, &["current", "mode"])
}

fn get_current_task_id(state: &Value) -> Option<String> {
    yaml_string(state, &["current", "task_id"])
}

fn get_current_run_id(state: &Value) -> Option<String> {
    yaml_string(state, &["current", "run_id"])
}

fn set_yaml_string(value: &mut Value, path: &[&str], next: &str) {
    set_yaml_value(value, path, Value::String(next.to_string()));
}

fn set_yaml_bool(value: &mut Value, path: &[&str], next: bool) {
    set_yaml_value(value, path, Value::Bool(next));
}

fn set_yaml_value(value: &mut Value, path: &[&str], next: Value) {
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

fn find_latest_output(project_root: &Path, task_id: &str, run_id: &str) -> Option<PathBuf> {
    let run_dir = project_root
        .join(".vibehub")
        .join("tasks")
        .join(task_id)
        .join("runs")
        .join(run_id);

    let mut candidates: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();

    let run_output = run_dir.join("outputs").join("output.md");
    if run_output.is_file() {
        if let Ok(metadata) = run_output.metadata() {
            if let Ok(modified) = metadata.modified() {
                candidates.push((run_output, modified));
            }
        }
    }

    let sessions_dir = run_dir.join("sessions");
    if let Ok(entries) = fs::read_dir(&sessions_dir) {
        for entry in entries.flatten() {
            let session_dir = entry.path();
            if !session_dir.is_dir() {
                continue;
            }
            let output = session_dir.join("output.md");
            if output.is_file() {
                if let Ok(metadata) = output.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        candidates.push((output, modified));
                    }
                }
            }
        }
    }

    candidates.sort_by_key(|(_, m)| *m);
    candidates.last().map(|(p, _)| p.clone())
}

fn parse_output_sections(path: &Path) -> Result<BTreeMap<String, String>> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let mut sections = BTreeMap::new();
    let mut current_key: Option<String> = None;
    let mut current_lines = Vec::new();

    for line in content.lines() {
        if let Some(title) = markdown_heading_title(line) {
            if let Some(key) = current_key.take() {
                let body = current_lines.join("\n").trim().to_string();
                if !body.is_empty() {
                    sections.insert(key, body);
                }
                current_lines.clear();
            }
            current_key = Some(normalize_heading(title));
        } else if current_key.is_some() {
            current_lines.push(line.to_string());
        }
    }

    if let Some(key) = current_key {
        let body = current_lines.join("\n").trim().to_string();
        if !body.is_empty() {
            sections.insert(key, body);
        }
    }

    Ok(sections)
}

fn markdown_heading_title(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let hashes = trimmed.chars().take_while(|ch| *ch == '#').count();
    if !(2..=6).contains(&hashes) {
        return None;
    }
    let rest = &trimmed[hashes..];
    if !rest.starts_with(' ') {
        return None;
    }
    Some(rest.trim())
}

fn normalize_heading(value: &str) -> String {
    let mut normalized = String::new();
    let mut previous_space = false;
    for ch in value.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch);
            previous_space = false;
        } else if !previous_space {
            normalized.push(' ');
            previous_space = true;
        }
    }
    normalized.trim().to_string()
}

fn required_output_to_section_keys(output_name: &str) -> &'static [&'static str] {
    match output_name {
        "changed_files" => &["files changed"],
        "implementation_summary" => &["completed"],
        "unresolved_questions" => &["not yet done"],
        "files_reportedly_read" => &["files reportedly read"],
        "commands_reportedly_run" => &["commands run"],
        "handoff_notes" => &["next session should"],
        "test_results_or_reason" => &["tests run", "tests run or reason not run"],
        "diff_summary" => &["diff summary"],
        "verdict" => &["verdict"],
        "risks" => &["warnings", "unresolved risks"],
        "evidence_grades" => &["evidence grades"],
        "intent" => &["completed", "key decisions made"],
        "acceptance_criteria" => &["completed", "key decisions made"],
        "autonomy_level" => &["completed", "key decisions made"],
        "source_log" => &["files reportedly read"],
        "findings" => &["completed", "key decisions made"],
        "research_pack" => &["context still needed"],
        "implementation_plan" => &["completed", "key decisions made"],
        "validation_plan" => &["tests run", "tests run or reason not run"],
        "context_plan" => &["context still needed"],
        _ => &["completed"],
    }
}

fn section_has_content(sections: &BTreeMap<String, String>, keys: &[&str]) -> bool {
    for key in keys {
        if let Some(value) = sections.get(*key) {
            if !value.trim().is_empty() {
                return true;
            }
        }
    }
    false
}

fn resolve_phase_list(workflow: &workflow::WorkflowConfig, mode: &str) -> Result<Vec<String>> {
    workflow::resolve_phase_list(workflow, mode)
}

pub fn validate_phase(project_root: impl AsRef<Path>) -> Result<PhaseValidationResult> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let (state, _) = read_state(&project_root)?;

    let phase =
        get_current_phase(&state).ok_or_else(|| anyhow!("No current phase in state.yaml"))?;
    let task_id =
        get_current_task_id(&state).ok_or_else(|| anyhow!("No current task_id in state.yaml"))?;
    let run_id =
        get_current_run_id(&state).ok_or_else(|| anyhow!("No current run_id in state.yaml"))?;

    let phase_rules = read_phase_rules(&project_root)?;
    let required_outputs = phase_rules
        .phases
        .get(&phase)
        .and_then(|entry| entry.required_outputs.clone())
        .unwrap_or_default();

    let output_path = find_latest_output(&project_root, &task_id, &run_id);
    let sections = match &output_path {
        Some(path) => parse_output_sections(path).unwrap_or_default(),
        None => BTreeMap::new(),
    };

    let mut found_outputs = Vec::new();
    let mut missing_outputs = Vec::new();

    for req in &required_outputs {
        let keys = required_output_to_section_keys(req);
        if section_has_content(&sections, keys) {
            found_outputs.push(req.clone());
        } else {
            missing_outputs.push(req.clone());
        }
    }

    let status = if required_outputs.is_empty() {
        STATUS_COMPLETED.to_string()
    } else if missing_outputs.is_empty() {
        STATUS_COMPLETED.to_string()
    } else {
        STATUS_NEEDS_ACTION.to_string()
    };

    let source_output_path = output_path
        .as_ref()
        .and_then(|p| relative_to_project(&project_root, p).ok())
        .map(|p| normalize_path(&p));

    Ok(PhaseValidationResult {
        phase,
        status,
        required_outputs,
        found_outputs,
        missing_outputs,
        source_output_path,
    })
}

pub fn set_phase_result(
    project_root: impl AsRef<Path>,
    target_phase: &str,
    status: &str,
) -> Result<PhaseSetResult> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let target_phase = validate_id("phase", target_phase)?;
    let status = validate_status(status)?;

    let (state, _) = read_state(&project_root)?;
    let mode = get_current_mode(&state).ok_or_else(|| anyhow!("No current mode in state.yaml"))?;

    let workflow = read_workflow(&project_root)?;
    let phases = resolve_phase_list(&workflow, &mode)?;
    if !phases.contains(&target_phase.to_string()) {
        return Err(anyhow!(
            "Phase '{}' is not defined for mode '{}' in workflow.yaml",
            target_phase,
            mode
        ));
    }

    let current_phase = get_current_phase(&state);
    let current_phase_status = get_current_phase_status(&state);
    if current_phase.as_deref() == Some(target_phase) {
        if let (Some(task_id), Some(run_id)) =
            (get_current_task_id(&state), get_current_run_id(&state))
        {
            update_task_run_phase(&project_root, &task_id, &run_id, target_phase, status)?;
        }
    }

    let target_is_current = current_phase.as_deref() == Some(target_phase);
    if let (Some(task_id), Some(run_id)) = (get_current_task_id(&state), get_current_run_id(&state))
    {
        let legacy_event = events::append_run_event(
            &project_root,
            &task_id,
            &run_id,
            "phase_status_set",
            "Phase status updated.",
            serde_json::json!({
                "phase": target_phase,
                "status": status,
                "current_phase": current_phase.clone(),
                "target_is_current": target_is_current,
            }),
        )
        .ok();
        append_phase_projection_events(
            &project_root,
            &task_id,
            &run_id,
            target_phase,
            status,
            legacy_event,
        );
        let _ = projection::project_current_run_state(&project_root);
    }

    Ok(PhaseSetResult {
        phase: target_phase.to_string(),
        status: status.to_string(),
        mode,
        current_phase: current_phase.unwrap_or_default(),
        current_phase_status: if target_is_current {
            status.to_string()
        } else {
            current_phase_status.unwrap_or_default()
        },
    })
}

/// Mark the current phase as `blocked` so VibeHub records that the agent
/// (or the user) is stopping work intentionally rather than completing.
/// Used by the CLI `pause` verb.
pub fn pause_current_phase(project_root: impl AsRef<Path>) -> Result<PhaseSetResult> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let (state, _) = read_state(&project_root)?;
    let phase = get_current_phase(&state)
        .ok_or_else(|| anyhow!("No current phase in state.yaml; cannot pause."))?;
    set_phase_result(&project_root, &phase, STATUS_BLOCKED)
}

fn append_phase_projection_events(
    project_root: &Path,
    task_id: &str,
    run_id: &str,
    phase: &str,
    status: &str,
    legacy_event: Option<events::RunEventAppendResult>,
) {
    let mut derived_from = event_ids([legacy_event]);
    if status == STATUS_ACTIVE {
        if let Ok(event) = events::append_structured_run_event(
            project_root,
            task_id,
            run_id,
            events::VibehubEvent::CapabilityClaimed {
                capability: phase.to_string(),
            },
        ) {
            derived_from.push(event.event_id);
        }
    } else if matches!(
        status,
        STATUS_COMPLETED | STATUS_FAILED | STATUS_BLOCKED | STATUS_NEEDS_ACTION
    ) {
        if let Ok(event) = events::append_structured_run_event(
            project_root,
            task_id,
            run_id,
            events::VibehubEvent::CapabilityReleased {
                capability: phase.to_string(),
                outcome: status.to_string(),
                task_pack_dirty: status != STATUS_COMPLETED,
            },
        ) {
            derived_from.push(event.event_id);
        }
    }
    let _ = events::append_structured_run_event(
        project_root,
        task_id,
        run_id,
        events::VibehubEvent::PhaseProjected {
            phase: phase.to_string(),
            derived_from,
        },
    );
}

fn append_phase_advanced_events(
    project_root: &Path,
    task_id: &str,
    run_id: &str,
    previous_phase: &str,
    previous_status: &str,
    current_phase: &str,
    current_status: &str,
    legacy_event: Option<events::RunEventAppendResult>,
) {
    let mut derived_from = event_ids([legacy_event]);
    if matches!(
        previous_status,
        STATUS_COMPLETED | STATUS_FAILED | STATUS_BLOCKED | STATUS_NEEDS_ACTION
    ) {
        if let Ok(event) = events::append_structured_run_event(
            project_root,
            task_id,
            run_id,
            events::VibehubEvent::CapabilityReleased {
                capability: previous_phase.to_string(),
                outcome: previous_status.to_string(),
                task_pack_dirty: previous_status != STATUS_COMPLETED,
            },
        ) {
            derived_from.push(event.event_id);
        }
    }
    if current_status == STATUS_ACTIVE && current_phase != previous_phase {
        if let Ok(event) = events::append_structured_run_event(
            project_root,
            task_id,
            run_id,
            events::VibehubEvent::CapabilityClaimed {
                capability: current_phase.to_string(),
            },
        ) {
            derived_from.push(event.event_id);
        }
    }
    let _ = events::append_structured_run_event(
        project_root,
        task_id,
        run_id,
        events::VibehubEvent::PhaseProjected {
            phase: current_phase.to_string(),
            derived_from,
        },
    );
}

fn append_handoff_bypass_risk(
    project_root: &Path,
    task_id: &str,
    run_id: &str,
    previous_phase: &str,
    current_phase: &str,
    missing_sections: &[String],
) {
    let _ = events::append_structured_run_event(
        project_root,
        task_id,
        run_id,
        events::VibehubEvent::RiskRaised {
            id: format!("handoff_force_advance_{previous_phase}_to_{current_phase}"),
            severity: "medium".to_string(),
            note: format!(
                "Phase advanced with incomplete handoff; missing sections: {}",
                missing_sections.join(", ")
            ),
        },
    );
}

fn event_ids<const N: usize>(events: [Option<events::RunEventAppendResult>; N]) -> Vec<String> {
    events
        .into_iter()
        .filter_map(|event| event.map(|event| event.event_id))
        .collect()
}

pub fn complete_phase(project_root: impl AsRef<Path>) -> Result<PhaseAdvanceResult> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let (state, _) = read_state(&project_root)?;

    let phase =
        get_current_phase(&state).ok_or_else(|| anyhow!("No current phase in state.yaml"))?;
    let mode = get_current_mode(&state).ok_or_else(|| anyhow!("No current mode in state.yaml"))?;

    let validation = validate_phase(&project_root)?;

    let target_status = if !validation.missing_outputs.is_empty() {
        STATUS_NEEDS_ACTION.to_string()
    } else {
        STATUS_COMPLETED.to_string()
    };

    if let (Some(task_id), Some(run_id)) = (get_current_task_id(&state), get_current_run_id(&state))
    {
        update_task_run_phase(&project_root, &task_id, &run_id, &phase, &target_status)?;
    }

    let workflow = read_workflow(&project_root)?;
    let phases = resolve_phase_list(&workflow, &mode)?;

    let next_phase = next_phase_in_list(&phases, &phase);

    if let (Some(task_id), Some(run_id)) = (get_current_task_id(&state), get_current_run_id(&state))
    {
        let legacy_event = events::append_run_event(
            &project_root,
            &task_id,
            &run_id,
            "phase_completed",
            "Current phase completion validated.",
            serde_json::json!({
                "phase": phase.clone(),
                "status": target_status.clone(),
                "missing_outputs": validation.missing_outputs.clone(),
                "next_phase": next_phase.clone(),
            }),
        )
        .ok();
        append_phase_projection_events(
            &project_root,
            &task_id,
            &run_id,
            &phase,
            &target_status,
            legacy_event,
        );
        let _ = projection::project_current_run_state(&project_root);
    }

    Ok(PhaseAdvanceResult {
        mode,
        previous_phase: phase.clone(),
        previous_status: target_status.clone(),
        current_phase: phase,
        current_status: target_status,
        next_phase,
        validation,
        handoff_complete: false,
        missing_handoff_sections: vec![],
        forced: false,
    })
}

pub fn advance_phase(project_root: impl AsRef<Path>) -> Result<PhaseAdvanceResult> {
    advance_phase_with_force(project_root, false)
}

pub fn advance_phase_with_force(
    project_root: impl AsRef<Path>,
    force: bool,
) -> Result<PhaseAdvanceResult> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let (mut state, state_path) = read_state(&project_root)?;

    let phase =
        get_current_phase(&state).ok_or_else(|| anyhow!("No current phase in state.yaml"))?;
    let mode = get_current_mode(&state).ok_or_else(|| anyhow!("No current mode in state.yaml"))?;

    let validation = validate_phase(&project_root)?;

    if !validation.missing_outputs.is_empty() {
        if let (Some(task_id), Some(run_id)) =
            (get_current_task_id(&state), get_current_run_id(&state))
        {
            update_task_run_phase(
                &project_root,
                &task_id,
                &run_id,
                &phase,
                STATUS_NEEDS_ACTION,
            )?;
        }

        if let (Some(task_id), Some(run_id)) =
            (get_current_task_id(&state), get_current_run_id(&state))
        {
            let legacy_event = events::append_run_event(
                &project_root,
                &task_id,
                &run_id,
                "phase_advance_blocked",
                "Phase advance blocked by missing required outputs.",
                serde_json::json!({
                    "phase": phase.clone(),
                    "status": STATUS_NEEDS_ACTION,
                    "missing_outputs": validation.missing_outputs.clone(),
                }),
            )
            .ok();
            append_phase_projection_events(
                &project_root,
                &task_id,
                &run_id,
                &phase,
                STATUS_NEEDS_ACTION,
                legacy_event,
            );
            let _ = projection::project_current_run_state(&project_root);
        }

        return Ok(PhaseAdvanceResult {
            mode,
            previous_phase: phase.clone(),
            previous_status: STATUS_ACTIVE.to_string(),
            current_phase: phase.clone(),
            current_status: STATUS_NEEDS_ACTION.to_string(),
            next_phase: None,
            validation,
            handoff_complete: false,
            missing_handoff_sections: vec![],
            forced: false,
        });
    }

    let handoff_gate = read_handoff_gate(&project_root)?;
    if !handoff_gate.complete && !force {
        if let (Some(task_id), Some(run_id)) =
            (get_current_task_id(&state), get_current_run_id(&state))
        {
            update_task_run_phase(
                &project_root,
                &task_id,
                &run_id,
                &phase,
                STATUS_NEEDS_ACTION,
            )?;
        }

        if let (Some(task_id), Some(run_id)) =
            (get_current_task_id(&state), get_current_run_id(&state))
        {
            let legacy_event = events::append_run_event(
                &project_root,
                &task_id,
                &run_id,
                "phase_advance_blocked",
                "Phase advance blocked by incomplete handoff.",
                serde_json::json!({
                    "phase": phase.clone(),
                    "status": STATUS_NEEDS_ACTION,
                    "missing_handoff_sections": handoff_gate.missing_sections.clone(),
                }),
            )
            .ok();
            append_phase_projection_events(
                &project_root,
                &task_id,
                &run_id,
                &phase,
                STATUS_NEEDS_ACTION,
                legacy_event,
            );
            let _ = projection::project_current_run_state(&project_root);
        }

        return Ok(PhaseAdvanceResult {
            mode,
            previous_phase: phase.clone(),
            previous_status: STATUS_ACTIVE.to_string(),
            current_phase: phase,
            current_status: STATUS_NEEDS_ACTION.to_string(),
            next_phase: None,
            validation,
            handoff_complete: false,
            missing_handoff_sections: handoff_gate.missing_sections,
            forced: false,
        });
    }

    let workflow = read_workflow(&project_root)?;
    let phases = resolve_phase_list(&workflow, &mode)?;

    let next_phase = match next_phase_in_list(&phases, &phase) {
        Some(next) => next,
        None => {
            if let (Some(task_id), Some(run_id)) =
                (get_current_task_id(&state), get_current_run_id(&state))
            {
                update_task_run_phase(&project_root, &task_id, &run_id, &phase, STATUS_COMPLETED)?;
            }

            let _ = agent_view::generate_agent_view(&project_root);
            if let (Some(task_id), Some(run_id)) =
                (get_current_task_id(&state), get_current_run_id(&state))
            {
                let legacy_event = events::append_run_event(
                    &project_root,
                    &task_id,
                    &run_id,
                    "phase_advanced",
                    "Final phase completed.",
                    serde_json::json!({
                        "previous_phase": phase.clone(),
                        "current_phase": phase.clone(),
                        "current_status": STATUS_COMPLETED,
                        "next_phase": Option::<String>::None,
                    }),
                )
                .ok();
                append_phase_advanced_events(
                    &project_root,
                    &task_id,
                    &run_id,
                    &phase,
                    STATUS_COMPLETED,
                    &phase,
                    STATUS_COMPLETED,
                    legacy_event,
                );
                let _ = projection::project_current_run_state(&project_root);
                if force && !handoff_gate.complete {
                    let _ = events::append_run_event(
                        &project_root,
                        &task_id,
                        &run_id,
                        "handoff_force_advance",
                        "Final phase completed despite incomplete handoff.",
                        serde_json::json!({
                            "previous_phase": phase.clone(),
                            "current_phase": phase.clone(),
                            "missing_handoff_sections": handoff_gate.missing_sections.clone(),
                        }),
                    );
                    append_handoff_bypass_risk(
                        &project_root,
                        &task_id,
                        &run_id,
                        &phase,
                        &phase,
                        &handoff_gate.missing_sections,
                    );
                }
            }

            return Ok(PhaseAdvanceResult {
                mode,
                previous_phase: phase.clone(),
                previous_status: STATUS_ACTIVE.to_string(),
                current_phase: phase,
                current_status: STATUS_COMPLETED.to_string(),
                next_phase: None,
                validation,
                handoff_complete: handoff_gate.complete,
                missing_handoff_sections: handoff_gate.missing_sections,
                forced: force && !handoff_gate.complete,
            });
        }
    };

    let run_id = get_current_run_id(&state).unwrap_or_default();
    let task_id = get_current_task_id(&state).unwrap_or_default();
    let run_path = resolve_run_path(&project_root, &task_id, &run_id).unwrap_or_default();
    start_task::ensure_context_spec(&project_root, &task_id, &run_id, &next_phase)?;
    context::build_context_pack(&project_root, &task_id, &run_id, &next_phase)
        .with_context(|| format!("Failed to build context pack for phase '{}'", next_phase))?;

    set_yaml_string(
        &mut state,
        &["context", "current_pack"],
        &format!("{run_path}/context-packs/{next_phase}.md"),
    );
    set_yaml_string(
        &mut state,
        &["context", "current_manifest"],
        &format!("{run_path}/context-packs/{next_phase}.manifest.yaml"),
    );
    set_yaml_bool(&mut state, &["context", "stale"], false);
    set_yaml_string(&mut state, &["handoff", "status"], "empty");
    set_yaml_string(&mut state, &["agent_report", "status"], "not_started");
    set_yaml_string(
        &mut state,
        &["agent_report", "validation_status"],
        "pending",
    );

    set_yaml_string(
        &mut state,
        &["resume_hint"],
        ".vibehub/agent-view/current.md",
    );

    let next_next = next_phase_in_list(&phases, &next_phase);

    update_task_run_phase(&project_root, &task_id, &run_id, &next_phase, STATUS_ACTIVE)?;
    write_state(&state_path, &state)?;
    let _ = agent_view::generate_agent_view(&project_root);
    let legacy_event = events::append_run_event(
        &project_root,
        &task_id,
        &run_id,
        "phase_advanced",
        "Phase advanced to next phase.",
        serde_json::json!({
            "previous_phase": phase.clone(),
            "previous_status": STATUS_COMPLETED,
            "current_phase": next_phase.clone(),
            "current_status": STATUS_ACTIVE,
            "next_phase": next_next.clone(),
            "context_pack": format!("{run_path}/context-packs/{next_phase}.md"),
            "context_manifest": format!("{run_path}/context-packs/{next_phase}.manifest.yaml"),
        }),
    )
    .ok();
    append_phase_advanced_events(
        &project_root,
        &task_id,
        &run_id,
        &phase,
        STATUS_COMPLETED,
        &next_phase,
        STATUS_ACTIVE,
        legacy_event,
    );
    let _ = projection::project_current_run_state(&project_root);
    if force && !handoff_gate.complete {
        let _ = events::append_run_event(
            &project_root,
            &task_id,
            &run_id,
            "handoff_force_advance",
            "Phase advanced despite incomplete handoff.",
            serde_json::json!({
                "previous_phase": phase.clone(),
                "current_phase": next_phase.clone(),
                "missing_handoff_sections": handoff_gate.missing_sections.clone(),
            }),
        );
        append_handoff_bypass_risk(
            &project_root,
            &task_id,
            &run_id,
            &phase,
            &next_phase,
            &handoff_gate.missing_sections,
        );
    }

    Ok(PhaseAdvanceResult {
        mode,
        previous_phase: phase,
        previous_status: STATUS_COMPLETED.to_string(),
        current_phase: next_phase,
        current_status: STATUS_ACTIVE.to_string(),
        next_phase: next_next,
        validation,
        handoff_complete: handoff_gate.complete,
        missing_handoff_sections: handoff_gate.missing_sections,
        forced: force && !handoff_gate.complete,
    })
}

fn read_handoff_gate(project_root: &Path) -> Result<HandoffGate> {
    let result = handoff::build_handoff(project_root)?;
    Ok(HandoffGate {
        complete: result.complete,
        missing_sections: result.missing_required_sections,
    })
}

fn next_phase_in_list(phases: &[String], current: &str) -> Option<String> {
    let pos = phases.iter().position(|p| p == current)?;
    if pos + 1 < phases.len() {
        Some(phases[pos + 1].clone())
    } else {
        None
    }
}

fn get_current_phase_status(state: &Value) -> Option<String> {
    yaml_string(state, &["current", "phase_status"])
}

fn resolve_run_path(project_root: &Path, task_id: &str, run_id: &str) -> Option<String> {
    if task_id.is_empty() || run_id.is_empty() {
        return None;
    }
    let path = format!(".vibehub/tasks/{task_id}/runs/{run_id}");
    if project_root.join(&path).is_dir() {
        Some(path)
    } else {
        None
    }
}

fn update_task_run_phase(
    project_root: &Path,
    task_id: &str,
    run_id: &str,
    phase: &str,
    status: &str,
) -> Result<()> {
    update_yaml_phase(
        &project_root
            .join(".vibehub")
            .join("tasks")
            .join(task_id)
            .join("task.yaml"),
        phase,
        status,
    )?;
    update_yaml_phase(
        &project_root
            .join(".vibehub")
            .join("tasks")
            .join(task_id)
            .join("runs")
            .join(run_id)
            .join("run.yaml"),
        phase,
        status,
    )
}

fn update_yaml_phase(path: &Path, phase: &str, status: &str) -> Result<()> {
    if !path.is_file() {
        return Ok(());
    }
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let mut value = serde_yaml::from_str::<Value>(&content)
        .with_context(|| format!("Invalid YAML in {}", path.display()))?;
    set_yaml_string(&mut value, &["phase"], phase);
    set_yaml_string(&mut value, &["phase_status"], status);
    let content = serde_yaml::to_string(&value).context("Failed to serialize phase metadata")?;
    fs::write(path, content).with_context(|| format!("Failed to write {}", path.display()))
}

fn validate_id<'a>(name: &str, value: &'a str) -> Result<&'a str> {
    let value = value.trim();
    if value.is_empty() {
        return Err(anyhow!("Invalid {}: value is empty", name));
    }
    if value.contains('/') || value.contains('\\') {
        return Err(anyhow!(
            "Invalid {} '{}': path separators are not allowed",
            name,
            value
        ));
    }
    Ok(value)
}

fn validate_status<'a>(value: &'a str) -> Result<&'a str> {
    let value = value.trim();
    if value.is_empty() {
        return Err(anyhow!("Invalid status: value is empty"));
    }
    if !VALID_STATUSES.contains(&value) {
        return Err(anyhow!(
            "Invalid status '{}': must be one of {:?}",
            value,
            VALID_STATUSES
        ));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vibehub::init;
    use crate::vibehub::start_task;
    use uuid::Uuid;

    fn temp_project() -> PathBuf {
        let path = std::env::temp_dir().join(format!("vibehub-phase-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).expect("create temp project");
        path
    }

    fn setup_guided_drive(project: &Path) -> start_task::VibehubStartTaskResult {
        init::init_project(project).expect("init");
        start_task::start_task(
            project,
            Some("Phase test task".to_string()),
            Some("guided_drive".to_string()),
            Some("implement".to_string()),
        )
        .expect("start task")
    }

    fn setup_yolo_drive(project: &Path) -> start_task::VibehubStartTaskResult {
        init::init_project(project).expect("init");
        start_task::start_task(
            project,
            Some("Phase test task".to_string()),
            Some("yolo_drive".to_string()),
            Some("align_lite".to_string()),
        )
        .expect("start task")
    }

    fn write_output_with_sections(
        project: &Path,
        task_id: &str,
        run_id: &str,
        sections: &[(&str, &str)],
    ) {
        let outputs_dir = project
            .join(".vibehub")
            .join("tasks")
            .join(task_id)
            .join("runs")
            .join(run_id)
            .join("outputs");
        fs::create_dir_all(&outputs_dir).expect("create outputs dir");

        let mut content = String::from("# Session Output\n\n");
        for (heading, body) in sections {
            content.push_str(&format!("## {}\n{}\n\n", heading, body));
        }
        let has_heading = |candidate: &str| {
            sections
                .iter()
                .any(|(heading, _)| heading.eq_ignore_ascii_case(candidate))
        };
        let has_what_changed = [
            "Completed",
            "Not Yet Done",
            "Key Decisions Made",
            "Files Changed",
        ]
        .iter()
        .any(|heading| has_heading(heading));
        if !has_what_changed {
            content.push_str("## Completed\n- Test fixture phase output.\n\n");
        }
        if !has_heading("Commands Run") {
            content.push_str("## Commands Run\n- cargo test vibehub::phase\n\n");
        }
        if !has_heading("Tests Run") {
            content.push_str("## Tests Run\n- cargo test vibehub::phase\n\n");
        }
        if !has_heading("Context Still Needed") {
            content.push_str("## Context Still Needed\n- None.\n\n");
        }
        if !has_heading("Warnings") {
            content.push_str("## Warnings\n- None.\n\n");
        }
        if !has_heading("Next Session Should") {
            content.push_str("## Next Session Should\n1. Continue.\n\n");
        }
        fs::write(outputs_dir.join("output.md"), content).expect("write output");
    }

    fn write_raw_output_with_sections(
        project: &Path,
        task_id: &str,
        run_id: &str,
        sections: &[(&str, &str)],
    ) {
        let outputs_dir = project
            .join(".vibehub")
            .join("tasks")
            .join(task_id)
            .join("runs")
            .join(run_id)
            .join("outputs");
        fs::create_dir_all(&outputs_dir).expect("create outputs dir");

        let mut content = String::from("# Session Output\n\n");
        for (heading, body) in sections {
            content.push_str(&format!("## {}\n{}\n\n", heading, body));
        }
        fs::write(outputs_dir.join("output.md"), content).expect("write raw output");
    }

    fn write_session_output_with_sections(
        project: &Path,
        task_id: &str,
        run_id: &str,
        session_id: &str,
        sections: &[(&str, &str)],
    ) {
        let session_dir = project
            .join(".vibehub")
            .join("tasks")
            .join(task_id)
            .join("runs")
            .join(run_id)
            .join("sessions")
            .join(session_id);
        fs::create_dir_all(&session_dir).expect("create session dir");

        let mut content = String::from("# Session Output\n\n");
        for (heading, body) in sections {
            content.push_str(&format!("## {}\n{}\n\n", heading, body));
        }
        fs::write(session_dir.join("output.md"), content).expect("write session output");
    }

    fn read_flow_status(project: &Path, phase_name: &str) -> Option<String> {
        let (state, _) = read_state(project).ok()?;
        yaml_string(&state, &["flow", phase_name])
    }

    fn event_types(project: &Path, task_id: &str, run_id: &str) -> Vec<String> {
        events::list_events(project, task_id, run_id, None)
            .expect("list events")
            .into_iter()
            .filter_map(|stored| {
                stored
                    .event
                    .get("event_type")
                    .and_then(serde_json::Value::as_str)
                    .map(ToString::to_string)
            })
            .collect()
    }

    #[test]
    fn validate_detects_missing_required_outputs() {
        let project = temp_project();
        let _started = setup_guided_drive(&project);

        let result = validate_phase(&project).expect("validate");

        assert_eq!(result.phase, "implement");
        assert_eq!(result.status, STATUS_NEEDS_ACTION);
        assert!(!result.required_outputs.is_empty());
        assert!(!result.missing_outputs.is_empty());
        assert!(result
            .missing_outputs
            .contains(&"changed_files".to_string()));
        assert!(result
            .missing_outputs
            .contains(&"implementation_summary".to_string()));
        assert!(result
            .missing_outputs
            .contains(&"unresolved_questions".to_string()));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn validate_passes_when_all_required_outputs_present() {
        let project = temp_project();
        let started = setup_guided_drive(&project);

        write_output_with_sections(
            &project,
            &started.task_id,
            &started.run_id,
            &[
                ("Files Changed", "- src/main.rs\n- src/lib.rs"),
                ("Completed", "- Implemented phase system\n- Added tests"),
                ("Not Yet Done", "- Need more documentation"),
            ],
        );

        let result = validate_phase(&project).expect("validate");

        assert_eq!(result.phase, "implement");
        assert_eq!(result.status, STATUS_COMPLETED);
        assert!(result.missing_outputs.is_empty());
        assert_eq!(result.found_outputs.len(), result.required_outputs.len());
        assert!(result.source_output_path.is_some());

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn validate_uses_session_output_when_outputs_absent() {
        let project = temp_project();
        let started = setup_guided_drive(&project);

        write_session_output_with_sections(
            &project,
            &started.task_id,
            &started.run_id,
            "S-001",
            &[
                ("Files Changed", "- src/main.rs"),
                ("Completed", "- Done"),
                ("Not Yet Done", "- More work"),
            ],
        );

        let result = validate_phase(&project).expect("validate");

        assert_eq!(result.status, STATUS_COMPLETED);
        assert!(result.source_output_path.is_some());
        assert!(result
            .source_output_path
            .as_ref()
            .unwrap()
            .contains("S-001"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn guided_drive_implement_to_review_full_advance() {
        let project = temp_project();
        let started = setup_guided_drive(&project);

        write_output_with_sections(
            &project,
            &started.task_id,
            &started.run_id,
            &[
                ("Files Changed", "- src/main.rs"),
                ("Completed", "- Implemented phase system"),
                ("Not Yet Done", "- Need more docs"),
            ],
        );

        let result = advance_phase(&project).expect("advance");

        assert_eq!(result.mode, "guided_drive");
        assert_eq!(result.previous_phase, "implement");
        assert_eq!(result.previous_status, STATUS_COMPLETED);
        assert_eq!(result.current_phase, "review");
        assert_eq!(result.current_status, STATUS_ACTIVE);
        assert_eq!(result.next_phase, None);

        let flow_impl = read_flow_status(&project, "implement");
        let flow_review = read_flow_status(&project, "review");
        assert_eq!(flow_impl.as_deref(), Some(STATUS_COMPLETED));
        assert_eq!(flow_review.as_deref(), Some(STATUS_ACTIVE));

        let (state, _) = read_state(&project).expect("read state");
        assert_eq!(
            yaml_string(&state, &["current", "phase"]).as_deref(),
            Some("review")
        );
        assert_eq!(
            yaml_string(&state, &["current", "phase_status"]).as_deref(),
            Some(STATUS_ACTIVE)
        );
        let events = event_types(&project, &started.task_id, &started.run_id);
        assert!(events.contains(&"Legacy".to_string()));
        assert!(events.contains(&"CapabilityReleased".to_string()));
        assert!(events.contains(&"CapabilityClaimed".to_string()));
        assert!(events.contains(&"PhaseProjected".to_string()));
        assert!(events::ensure_consistency(&project)
            .expect("consistency")
            .is_empty());

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn yolo_drive_align_lite_to_implement_to_review_lite() {
        let project = temp_project();
        let started = setup_yolo_drive(&project);

        write_output_with_sections(
            &project,
            &started.task_id,
            &started.run_id,
            &[
                ("Completed", "- Intent defined"),
                ("Key Decisions Made", "- High autonomy"),
            ],
        );

        let result1 = advance_phase(&project).expect("advance align_lite -> implement");

        assert_eq!(result1.mode, "yolo_drive");
        assert_eq!(result1.previous_phase, "align_lite");
        assert_eq!(result1.previous_status, STATUS_COMPLETED);
        assert_eq!(result1.current_phase, "implement");
        assert_eq!(result1.current_status, STATUS_ACTIVE);
        assert_eq!(result1.next_phase, Some("review_lite".to_string()));

        let flow_align_lite = read_flow_status(&project, "align_lite");
        let flow_impl = read_flow_status(&project, "implement");
        assert_eq!(flow_align_lite.as_deref(), Some(STATUS_COMPLETED));
        assert_eq!(flow_impl.as_deref(), Some(STATUS_ACTIVE));

        write_output_with_sections(
            &project,
            &started.task_id,
            &started.run_id,
            &[
                ("Files Changed", "- src/main.rs"),
                ("Completed", "- Implemented stuff"),
                ("Not Yet Done", "- More tests"),
            ],
        );

        let result2 = advance_phase(&project).expect("advance implement -> review_lite");

        assert_eq!(result2.previous_phase, "implement");
        assert_eq!(result2.previous_status, STATUS_COMPLETED);
        assert_eq!(result2.current_phase, "review_lite");
        assert_eq!(result2.current_status, STATUS_ACTIVE);

        let flow_impl2 = read_flow_status(&project, "implement");
        let flow_review_lite = read_flow_status(&project, "review_lite");
        assert_eq!(flow_impl2.as_deref(), Some(STATUS_COMPLETED));
        assert_eq!(flow_review_lite.as_deref(), Some(STATUS_ACTIVE));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn missing_outputs_blocks_advance_and_sets_needs_action() {
        let project = temp_project();
        setup_guided_drive(&project);

        let result = advance_phase(&project).expect("advance");

        assert_eq!(result.previous_phase, "implement");
        assert_eq!(result.current_phase, "implement");
        assert_eq!(result.current_status, STATUS_NEEDS_ACTION);
        assert_eq!(result.next_phase, None);
        assert_eq!(result.validation.status, STATUS_NEEDS_ACTION);
        assert!(!result.validation.missing_outputs.is_empty());

        let flow_impl = read_flow_status(&project, "implement");
        assert_eq!(flow_impl.as_deref(), Some(STATUS_NEEDS_ACTION));

        let (state, _) = read_state(&project).expect("read state");
        assert_eq!(
            yaml_string(&state, &["current", "phase"]).as_deref(),
            Some("implement")
        );
        assert_eq!(
            yaml_string(&state, &["current", "phase_status"]).as_deref(),
            Some(STATUS_NEEDS_ACTION)
        );

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn incomplete_handoff_blocks_advance_after_outputs_validate() {
        let project = temp_project();
        let started = setup_guided_drive(&project);

        write_raw_output_with_sections(
            &project,
            &started.task_id,
            &started.run_id,
            &[
                ("Files Changed", "- src/main.rs"),
                ("Completed", "- Implemented phase system"),
                ("Not Yet Done", "- Need more docs"),
            ],
        );

        let result = advance_phase(&project).expect("advance");

        assert_eq!(result.current_phase, "implement");
        assert_eq!(result.current_status, STATUS_NEEDS_ACTION);
        assert!(result.validation.missing_outputs.is_empty());
        assert!(!result.handoff_complete);
        assert!(result
            .missing_handoff_sections
            .contains(&"Commands Run".to_string()));
        assert!(!result.forced);

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn force_allows_advance_with_incomplete_handoff() {
        let project = temp_project();
        let started = setup_guided_drive(&project);

        write_raw_output_with_sections(
            &project,
            &started.task_id,
            &started.run_id,
            &[
                ("Files Changed", "- src/main.rs"),
                ("Completed", "- Implemented phase system"),
                ("Not Yet Done", "- Need more docs"),
            ],
        );

        let result = advance_phase_with_force(&project, true).expect("advance");

        assert_eq!(result.previous_phase, "implement");
        assert_eq!(result.current_phase, "review");
        assert_eq!(result.current_status, STATUS_ACTIVE);
        assert!(result.validation.missing_outputs.is_empty());
        assert!(!result.handoff_complete);
        assert!(result.forced);
        assert!(result
            .missing_handoff_sections
            .contains(&"Commands Run".to_string()));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn set_phase_result_explicitly() {
        let project = temp_project();
        let _started = setup_guided_drive(&project);

        let result =
            set_phase_result(&project, "implement", STATUS_COMPLETED).expect("set phase result");

        assert_eq!(result.phase, "implement");
        assert_eq!(result.status, STATUS_COMPLETED);
        assert_eq!(result.mode, "guided_drive");

        let flow_impl = read_flow_status(&project, "implement");
        assert_eq!(flow_impl.as_deref(), Some(STATUS_COMPLETED));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn set_phase_result_rejects_invalid_status() {
        let project = temp_project();
        setup_guided_drive(&project);

        let err = set_phase_result(&project, "implement", "bogus").expect_err("invalid status");
        assert!(err.to_string().contains("Invalid status"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn set_phase_result_rejects_unknown_phase() {
        let project = temp_project();
        setup_guided_drive(&project);

        let err =
            set_phase_result(&project, "nonexistent", STATUS_COMPLETED).expect_err("unknown phase");
        assert!(err.to_string().contains("not defined for mode"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn complete_phase_validates_then_completes() {
        let project = temp_project();
        let started = setup_guided_drive(&project);

        write_output_with_sections(
            &project,
            &started.task_id,
            &started.run_id,
            &[
                ("Files Changed", "- src/main.rs"),
                ("Completed", "- Done"),
                ("Not Yet Done", "- More"),
            ],
        );

        let result = complete_phase(&project).expect("complete");

        assert_eq!(result.previous_phase, "implement");
        assert_eq!(result.current_status, STATUS_COMPLETED);
        assert!(result.validation.missing_outputs.is_empty());

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn complete_phase_sets_needs_action_without_output() {
        let project = temp_project();
        setup_guided_drive(&project);

        let result = complete_phase(&project).expect("complete");

        assert_eq!(result.current_status, STATUS_NEEDS_ACTION);
        assert!(!result.validation.missing_outputs.is_empty());

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn advance_last_phase_marks_completed_no_next() {
        let project = temp_project();
        init::init_project(&project).expect("init");
        let started = start_task::start_task(
            &project,
            Some("Last phase test".to_string()),
            Some("guided_drive".to_string()),
            Some("review".to_string()),
        )
        .expect("start task");

        write_output_with_sections(
            &project,
            &started.task_id,
            &started.run_id,
            &[
                ("Diff Summary", "- 3 files changed"),
                ("Tests Run Or Reason Not Run", "- cargo test"),
                ("Verdict", "- passed"),
                ("Warnings", "- none"),
                ("Evidence Grades", "- hard_observed"),
            ],
        );

        let result = advance_phase(&project).expect("advance");

        assert_eq!(result.previous_phase, "review");
        assert_eq!(result.current_phase, "review");
        assert_eq!(result.current_status, STATUS_COMPLETED);
        assert_eq!(result.next_phase, None);

        let flow_review = read_flow_status(&project, "review");
        assert_eq!(flow_review.as_deref(), Some(STATUS_COMPLETED));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn context_pack_path_updates_when_advancing() {
        let project = temp_project();
        let started = setup_yolo_drive(&project);

        write_output_with_sections(
            &project,
            &started.task_id,
            &started.run_id,
            &[
                ("Completed", "- Intent defined"),
                ("Key Decisions Made", "- High autonomy"),
            ],
        );

        advance_phase(&project).expect("advance");

        let (state, _) = read_state(&project).expect("read state");
        let pack_path = yaml_string(&state, &["context", "current_pack"]);
        let manifest_path = yaml_string(&state, &["context", "current_manifest"]);

        assert!(pack_path.unwrap().contains("implement.md"));
        assert!(manifest_path.unwrap().contains("implement.manifest.yaml"));

        let state_stale = state
            .get("context")
            .and_then(|c| c.get("stale"))
            .and_then(|s| s.as_bool());
        assert_eq!(state_stale, Some(false));

        fs::remove_dir_all(project).expect("cleanup");
    }
}

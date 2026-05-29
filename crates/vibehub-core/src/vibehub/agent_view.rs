use crate::vibehub::util::{
    canonical_initialized_project_root, normalize_path, relative_to_project,
};
use crate::vibehub::{current, handoff, neighbors, projection, research};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AgentViewGenerateResult {
    pub current_path: String,
    pub current_context_path: String,
    pub handoff_path: String,
    pub handoff_created: bool,
    pub handoff_complete: bool,
    pub missing_handoff_sections: Vec<String>,
    pub task_id: String,
    pub run_id: String,
    pub phase: String,
}

#[derive(Debug, Deserialize)]
struct ContextManifest {
    #[serde(default)]
    included: Vec<ManifestIncluded>,
    #[serde(default)]
    missing: Vec<ManifestMissing>,
    #[serde(default)]
    excluded: Vec<ManifestExcluded>,
    #[serde(default)]
    quality: Option<ManifestQuality>,
}

#[derive(Debug, Deserialize)]
struct ManifestIncluded {
    path: String,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    required: bool,
}

#[derive(Debug, Deserialize)]
struct ManifestMissing {
    path: String,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ManifestExcluded {
    path: String,
    #[serde(default)]
    policy: Option<String>,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ManifestQuality {
    #[serde(default)]
    flags: Vec<String>,
}

struct AgentViewInput {
    task_id: String,
    task_path: String,
    run_id: String,
    run_path: String,
    mode: String,
    phase: String,
    phase_status: String,
    active_task_ids: Vec<String>,
    active_capabilities: Vec<String>,
    neighbor_tasks: Vec<neighbors::TaskNeighbor>,
    intake_queue: Option<IntakeQueueView>,
    context_pack_path: Option<String>,
    manifest_path: Option<String>,
    research_required: String,
    research_status: String,
    research_pack_exists: bool,
    manifest: Option<ContextManifest>,
}

struct IntakeQueueView {
    batch_id: Option<String>,
    source_message: Option<String>,
    split_confidence: Option<String>,
    current_task_id: Option<String>,
    remaining_task_ids: Vec<String>,
    items: Vec<IntakeQueueItemView>,
}

struct IntakeQueueItemView {
    order: Option<u64>,
    task_id: Option<String>,
    title: Option<String>,
    status: Option<String>,
    dependencies: Vec<String>,
}

pub fn generate_agent_view(project_root: impl AsRef<Path>) -> Result<AgentViewGenerateResult> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let task_pointer = current::resolve_current_task(&project_root)?;
    let run_pointer = current::resolve_current_run(&project_root, &task_pointer.task_id)?;
    let state_path = project_root.join(".vibehub/state.yaml");
    let state = read_yaml_value(&state_path, "state.yaml")?;
    let projected = projection::derive_current_run_state(&project_root).ok();

    let phase = projected
        .as_ref()
        .and_then(|state| state.current_phase.as_deref())
        .or_else(|| get_str(&state, &["current", "phase"]))
        .unwrap_or("unknown");
    let phase_status = projected
        .as_ref()
        .and_then(|state| state.current_phase_status.as_deref())
        .or_else(|| get_str(&state, &["current", "phase_status"]))
        .unwrap_or("unknown");
    let active_capabilities = projected
        .as_ref()
        .map(|state| state.active_capabilities.clone())
        .filter(|capabilities| !capabilities.is_empty())
        .unwrap_or_else(|| active_capabilities_from_state(&state));
    let input = AgentViewInput {
        task_id: task_pointer.task_id.clone(),
        task_path: task_pointer.path.clone(),
        run_id: run_pointer.run_id.clone(),
        run_path: run_pointer.path.clone(),
        mode: get_str(&state, &["current", "mode"])
            .unwrap_or("unknown")
            .to_string(),
        phase: phase.to_string(),
        phase_status: phase_status.to_string(),
        active_task_ids: active_task_ids_from_state(&state),
        active_capabilities,
        neighbor_tasks: neighbors::query_task_neighbors_for(&project_root, &task_pointer.task_id)
            .map(|report| report.neighbors)
            .unwrap_or_default(),
        intake_queue: read_intake_queue(&state),
        context_pack_path: state_path_value(&state, &["context", "current_pack"])
            .or_else(|| infer_context_pack_path(&run_pointer.path, phase, &project_root)),
        manifest_path: state_path_value(&state, &["context", "current_manifest"])
            .or_else(|| infer_manifest_path(&run_pointer.path, phase, &project_root)),
        research_required: get_bool(&state, &["research", "required"])
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        research_status: get_str(&state, &["research", "status"])
            .unwrap_or("unknown")
            .to_string(),
        research_pack_exists: research::check_research_pack_exists(&project_root),
        manifest: None,
    };

    let manifest = input
        .manifest_path
        .as_deref()
        .and_then(|path| read_manifest_if_available(&project_root, path).transpose())
        .transpose()?;
    let input = AgentViewInput { manifest, ..input };

    let agent_view_dir = project_root.join(".vibehub/agent-view");
    fs::create_dir_all(&agent_view_dir)
        .with_context(|| format!("Failed to create {}", agent_view_dir.display()))?;

    let current_path = agent_view_dir.join("current.md");
    let current_context_path = agent_view_dir.join("current-context.md");
    let handoff_path = agent_view_dir.join("handoff.md");

    fs::write(&current_path, build_current_md(&input))
        .with_context(|| format!("Failed to write {}", current_path.display()))?;
    fs::write(&current_context_path, build_current_context_md(&input))
        .with_context(|| format!("Failed to write {}", current_context_path.display()))?;

    let handoff_existed = handoff_path.exists();
    let handoff_result = handoff::build_handoff(&project_root)?;

    Ok(AgentViewGenerateResult {
        current_path: normalize_path(&relative_to_project(&project_root, &current_path)?),
        current_context_path: normalize_path(&relative_to_project(
            &project_root,
            &current_context_path,
        )?),
        handoff_path: normalize_path(&relative_to_project(&project_root, &handoff_path)?),
        handoff_created: !handoff_existed,
        handoff_complete: handoff_result.complete,
        missing_handoff_sections: handoff_result.missing_required_sections,
        task_id: input.task_id,
        run_id: input.run_id,
        phase: input.phase,
    })
}

fn build_current_md(input: &AgentViewInput) -> String {
    let mut output = String::new();
    output.push_str("# VibeHub Current\n\n");
    output.push_str("## Task\n\n");
    output.push_str(&format!("- Task ID: {}\n", input.task_id));
    output.push_str(&format!("- Task path: {}\n", input.task_path));
    output.push_str(&format!("- Run ID: {}\n", input.run_id));
    output.push_str(&format!("- Run path: {}\n\n", input.run_path));

    output.push_str("## Mode\n\n");
    output.push_str(&format!("- Mode: {}\n\n", input.mode));

    output.push_str("## Current Phase\n\n");
    output.push_str(&format!("- Phase: {}\n", input.phase));
    output.push_str(&format!("- Status: {}\n\n", input.phase_status));

    output.push_str("## Active Tasks\n\n");
    if input.active_task_ids.is_empty() {
        output.push_str("- None observed.\n\n");
    } else {
        for task_id in &input.active_task_ids {
            let marker = if task_id == &input.task_id {
                " (current)"
            } else {
                ""
            };
            output.push_str(&format!("- {}{}\n", task_id, marker));
        }
        output.push('\n');
    }

    output.push_str("## Intake Queue\n\n");
    if let Some(queue) = &input.intake_queue {
        if let Some(batch_id) = &queue.batch_id {
            output.push_str(&format!("- Batch: {batch_id}\n"));
        }
        if let Some(confidence) = &queue.split_confidence {
            output.push_str(&format!("- Split confidence: {confidence}\n"));
        }
        if let Some(source_message) = &queue.source_message {
            output.push_str(&format!("- Source message: {source_message}\n"));
        }
        if let Some(current_task_id) = &queue.current_task_id {
            output.push_str(&format!("- Current queue task: {current_task_id}\n"));
        }
        if !queue.remaining_task_ids.is_empty() {
            output.push_str(&format!(
                "- Remaining order: {}\n",
                queue.remaining_task_ids.join(" -> ")
            ));
        }
        if queue.items.is_empty() {
            output.push_str("- No intake items recorded.\n");
        } else {
            for item in &queue.items {
                output.push_str(&format!(
                    "- {}. {}{}{}{}\n",
                    item.order.unwrap_or(0),
                    item.task_id.as_deref().unwrap_or("uncreated"),
                    item.title
                        .as_deref()
                        .map(|title| format!(" ({title})"))
                        .unwrap_or_default(),
                    item.status
                        .as_deref()
                        .map(|status| format!(": {status}"))
                        .unwrap_or_default(),
                    if item.dependencies.is_empty() {
                        String::new()
                    } else {
                        format!("; depends_on=[{}]", item.dependencies.join(", "))
                    }
                ));
            }
        }
        output.push('\n');
    } else {
        output.push_str("- None observed.\n\n");
    }

    output.push_str("## Active Capabilities\n\n");
    if input.active_capabilities.is_empty() {
        output.push_str("- None observed.\n\n");
    } else {
        for capability in &input.active_capabilities {
            output.push_str(&format!("- {}\n", capability));
        }
        output.push('\n');
    }

    output.push_str("## Neighbor Tasks\n\n");
    if input.neighbor_tasks.is_empty() {
        output.push_str("- None observed.\n\n");
    } else {
        for neighbor in &input.neighbor_tasks {
            output.push_str(&format!(
                "- {}{}: active_capabilities=[{}], shared_files=[{}]\n",
                neighbor.task_id,
                neighbor
                    .title
                    .as_deref()
                    .map(|title| format!(" ({title})"))
                    .unwrap_or_default(),
                neighbor.active_capabilities.join(", "),
                neighbor.shared_files.join(", ")
            ));
        }
        output.push('\n');
        for neighbor in input
            .neighbor_tasks
            .iter()
            .filter(|neighbor| !neighbor.shared_files.is_empty())
        {
            output.push_str(&format!(
                "- Warning: neighbor {} shares files; coordinate before changing shared files.\n",
                neighbor.task_id
            ));
        }
        output.push('\n');
    }

    output.push_str("## Observability Note\n\n");
    output.push_str("- P0/P1 observability is best-effort.\n");
    output.push_str("- Hard observed: Git diff, filesystem state, and VibeHub-generated files.\n");
    output.push_str("- Agent reported: files read, commands run, summaries, and handoff notes.\n");
    output.push_str("- Inferred: task mapping, likely risk, and context completeness.\n");
    output.push_str("- Runtime observation is not enabled in P0.\n\n");

    output.push_str("## What To Read\n\n");
    output.push_str("- **Next session: read `.vibehub/agent-view/handoff.md` first.** It captures the prior session handoff (completed, remaining, commands run, tests run, context used, and warnings).\n");
    output.push_str("- .vibehub/agent-view/current-context.md\n");
    output.push_str("- .vibehub/agent-view/handoff.md\n");
    output.push_str("- .vibehub/rules/hard-rules.md\n");
    if let Some(path) = &input.context_pack_path {
        output.push_str(&format!("- {}\n", path));
    }
    output.push('\n');

    output.push_str("## What To Write\n\n");
    output.push_str(&format!(
        "- Suggested phase output under {}/outputs/ if needed.\n",
        input.run_path
    ));
    output.push_str("- Changed files only within the active task scope.\n");
    output.push_str("- Final response or agent output must include changed files, commands run, tests run or reason not run, unresolved risks, and handoff notes.\n\n");

    output.push_str("## Stop Condition\n\n");
    output.push_str("- Do not edit .vibehub/state.yaml directly.\n");
    output.push_str("- Do not mark canonical task, run, or phase state completed.\n");
    output.push_str("- Stop and return to VibeHub validation when the phase output is ready or when required context is missing.\n");
    output
}

fn build_current_context_md(input: &AgentViewInput) -> String {
    let mut output = String::new();
    output.push_str("# VibeHub Current Context\n\n");
    output.push_str("## Context Pack\n\n");
    output.push_str(&format!(
        "- Path: {}\n\n",
        input
            .context_pack_path
            .as_deref()
            .unwrap_or("not available")
    ));

    output.push_str("## Manifest\n\n");
    output.push_str(&format!(
        "- Path: {}\n",
        input.manifest_path.as_deref().unwrap_or("not available")
    ));
    output.push_str(&format!(
        "- Status: {}\n\n",
        if input.manifest.is_some() {
            "available"
        } else {
            "not available"
        }
    ));

    output.push_str("## Important Project Files\n\n");
    match &input.manifest {
        Some(manifest) => {
            let mut included = manifest
                .included
                .iter()
                .filter(|entry| !entry.path.starts_with(".vibehub/"))
                .collect::<Vec<_>>();
            included.sort_by(|a, b| a.path.cmp(&b.path));
            if included.is_empty() {
                output.push_str("- None listed in manifest.\n\n");
            } else {
                for entry in included {
                    output.push_str(&format!(
                        "- {}{}{}\n",
                        entry.path,
                        if entry.required { " (required)" } else { "" },
                        entry
                            .reason
                            .as_deref()
                            .map(|reason| format!(": {reason}"))
                            .unwrap_or_default()
                    ));
                }
                output.push('\n');
            }
        }
        None => output.push_str("- No manifest available.\n\n"),
    }

    output.push_str("## Research Pack\n\n");
    output.push_str(&format!("- Required: {}\n", input.research_required));
    output.push_str(&format!("- Status: {}\n", input.research_status));
    output.push_str(&format!(
        "- Present on disk: {}\n",
        if input.research_pack_exists {
            "yes"
        } else {
            "no"
        }
    ));
    output.push_str("- Current research path: .vibehub/research/current/research-pack.md\n\n");

    output.push_str("## Known Missing Context\n\n");
    match &input.manifest {
        Some(manifest) => {
            let mut missing = manifest.missing.iter().collect::<Vec<_>>();
            missing.sort_by(|a, b| a.path.cmp(&b.path));
            if missing.is_empty() && manifest.excluded.is_empty() {
                output.push_str("- None recorded in manifest.\n");
            } else {
                for entry in missing {
                    output.push_str(&format!(
                        "- Missing {}{}{}\n",
                        entry.path,
                        if entry.required {
                            " (required)"
                        } else {
                            " (optional)"
                        },
                        entry
                            .reason
                            .as_deref()
                            .map(|reason| format!(": {reason}"))
                            .unwrap_or_default()
                    ));
                }
                let mut excluded = manifest.excluded.iter().collect::<Vec<_>>();
                excluded.sort_by(|a, b| a.path.cmp(&b.path));
                for entry in excluded {
                    output.push_str(&format!(
                        "- Excluded {}{}{}\n",
                        entry.path,
                        entry
                            .policy
                            .as_deref()
                            .map(|policy| format!(" ({policy})"))
                            .unwrap_or_default(),
                        entry
                            .reason
                            .as_deref()
                            .map(|reason| format!(": {reason}"))
                            .unwrap_or_default()
                    ));
                }
                if let Some(quality) = &manifest.quality {
                    for flag in &quality.flags {
                        output.push_str(&format!("- Quality flag: {flag}\n"));
                    }
                }
            }
        }
        None => output.push_str("- Context manifest is not available.\n"),
    }
    output
}

fn read_manifest_if_available(project_root: &Path, path: &str) -> Result<Option<ContextManifest>> {
    let manifest_path = project_root.join(path);
    if !manifest_path.is_file() {
        return Ok(None);
    }
    let content = fs::read_to_string(&manifest_path)
        .with_context(|| format!("Failed to read {}", manifest_path.display()))?;
    let manifest = serde_yaml::from_str(&content)
        .with_context(|| format!("Invalid YAML in manifest {}", manifest_path.display()))?;
    Ok(Some(manifest))
}

fn read_yaml_value(path: &Path, label: &str) -> Result<Value> {
    if !path.is_file() {
        return Err(anyhow!("Missing {} file: {}", label, path.display()));
    }
    let content = fs::read_to_string(path).with_context(|| format!("Failed to read {label}"))?;
    serde_yaml::from_str(&content).with_context(|| format!("Invalid YAML in {}", path.display()))
}

fn get_str<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))
        .and_then(Value::as_str)
}

fn get_bool(value: &Value, path: &[&str]) -> Option<bool> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))
        .and_then(Value::as_bool)
}

fn state_path_value(value: &Value, path: &[&str]) -> Option<String> {
    get_str(value, path).and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.replace('\\', "/"))
        }
    })
}

fn active_capabilities_from_state(state: &Value) -> Vec<String> {
    let mut active = state
        .get("flow")
        .and_then(Value::as_mapping)
        .map(|flow| {
            flow.iter()
                .filter_map(|(key, value)| match (key.as_str(), value.as_str()) {
                    (Some(capability), Some("active")) => Some(capability.to_string()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if active.is_empty() {
        if let (Some(phase), Some(status)) = (
            get_str(state, &["current", "phase"]),
            get_str(state, &["current", "phase_status"]),
        ) {
            if matches!(status, "active" | "running") {
                active.push(phase.to_string());
            }
        }
    }
    active.sort();
    active
}

fn read_intake_queue(state: &Value) -> Option<IntakeQueueView> {
    let queue = state.get("tasks")?.get("intake_queue")?;
    let items = queue
        .get("items")
        .and_then(Value::as_sequence)
        .map(|items| {
            items
                .iter()
                .map(|item| IntakeQueueItemView {
                    order: item.get("order").and_then(Value::as_u64),
                    task_id: item
                        .get("task_id")
                        .and_then(Value::as_str)
                        .map(ToString::to_string),
                    title: item
                        .get("title")
                        .and_then(Value::as_str)
                        .map(ToString::to_string),
                    status: item
                        .get("status")
                        .and_then(Value::as_str)
                        .map(ToString::to_string),
                    dependencies: item
                        .get("dependencies")
                        .and_then(Value::as_sequence)
                        .map(|dependencies| {
                            dependencies
                                .iter()
                                .filter_map(Value::as_str)
                                .map(ToString::to_string)
                                .collect()
                        })
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default();
    Some(IntakeQueueView {
        batch_id: queue
            .get("batch_id")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        source_message: queue
            .get("source_message")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        split_confidence: queue
            .get("split_confidence")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        current_task_id: queue
            .get("current_task_id")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        remaining_task_ids: queue
            .get("remaining_task_ids")
            .and_then(Value::as_sequence)
            .map(|tasks| {
                tasks
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        items,
    })
}

fn active_task_ids_from_state(state: &Value) -> Vec<String> {
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
    if active.is_empty() {
        if let Some(task_id) = get_str(state, &["current", "task_id"]) {
            active.push(task_id.to_string());
        }
    }
    active.sort();
    active.dedup();
    active
}

fn infer_context_pack_path(run_path: &str, phase: &str, project_root: &Path) -> Option<String> {
    infer_existing_run_file(
        run_path,
        phase,
        &format!("context-packs/{phase}.md"),
        project_root,
    )
}

fn infer_manifest_path(run_path: &str, phase: &str, project_root: &Path) -> Option<String> {
    infer_existing_run_file(
        run_path,
        phase,
        &format!("context-packs/{phase}.manifest.yaml"),
        project_root,
    )
}

fn infer_existing_run_file(
    run_path: &str,
    phase: &str,
    suffix: &str,
    project_root: &Path,
) -> Option<String> {
    if phase_unknown(phase) {
        return None;
    }
    let candidate = format!("{}/{}", run_path.trim_end_matches('/'), suffix);
    if project_root.join(&candidate).is_file() {
        Some(candidate)
    } else {
        None
    }
}

fn phase_unknown(value: &str) -> bool {
    value.trim().is_empty() || value == "unknown"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vibehub::current::{write_current_run_pointer, write_current_task_pointer};
    use crate::vibehub::events::{self, VibehubEvent};
    use std::path::PathBuf;
    use uuid::Uuid;

    fn temp_project() -> PathBuf {
        let path = std::env::temp_dir().join(format!("vibehub-agent-view-test-{}", Uuid::new_v4()));
        fs::create_dir_all(path.join(".vibehub/tasks/T-001/runs/R-001/context-packs"))
            .expect("create run dirs");
        fs::create_dir_all(path.join(".vibehub/agent-view")).expect("create agent-view");
        path
    }

    fn write_state(project: &Path) {
        fs::write(
            project.join(".vibehub/state.yaml"),
            r#"schema_version: 1
current:
  mode: guided_drive
  task_id: T-001
  run_id: R-001
  phase: implement
  phase_status: active
context:
  current_pack: ".vibehub/tasks/T-001/runs/R-001/context-packs/implement.md"
  current_manifest: ".vibehub/tasks/T-001/runs/R-001/context-packs/implement.manifest.yaml"
research:
  required: false
  status: skipped
"#,
        )
        .expect("write state");
    }

    fn write_manifest(project: &Path) {
        fs::write(
            project.join(".vibehub/tasks/T-001/runs/R-001/context-packs/implement.manifest.yaml"),
            r#"included:
  - path: src/main.rs
    reason: implementation entrypoint
    required: true
  - path: .vibehub/rules/hard-rules.md
    reason: protocol rules
    required: true
missing:
  - path: tests/main.rs
    required: false
    reason: no test file found
excluded:
  - path: .env.local
    policy: deny_secret_path
    reason: secret-like path denied
quality:
  flags:
    - missing_optional_context
"#,
        )
        .expect("write manifest");
        fs::write(
            project.join(".vibehub/tasks/T-001/runs/R-001/context-packs/implement.md"),
            "# Context\n",
        )
        .expect("write pack");
    }

    fn write_workflow(project: &Path) {
        fs::write(
            project.join(".vibehub/workflow.yaml"),
            r#"modes:
  guided_drive:
    phases: [align, research, implement]
    capabilities: [align, research, implement]
phase_order: [align, research, implement]
"#,
        )
        .expect("write workflow");
    }

    #[test]
    fn generates_agent_view_from_current_pointers_state_and_manifest() {
        let project = temp_project();
        write_state(&project);
        write_manifest(&project);
        write_current_task_pointer(&project, "T-001").expect("task pointer");
        write_current_run_pointer(&project, "T-001", "R-001").expect("run pointer");

        let result = generate_agent_view(&project).expect("generate agent view");

        assert_eq!(result.current_path, ".vibehub/agent-view/current.md");
        assert!(result.handoff_created);

        let current = fs::read_to_string(project.join(".vibehub/agent-view/current.md"))
            .expect("read current");
        let current_context =
            fs::read_to_string(project.join(".vibehub/agent-view/current-context.md"))
                .expect("read current context");
        let handoff = fs::read_to_string(project.join(".vibehub/agent-view/handoff.md"))
            .expect("read handoff");

        assert!(current.contains("- Task ID: T-001"));
        assert!(current.contains("- Mode: guided_drive"));
        assert!(current.contains("- Phase: implement"));
        assert!(current.contains("## Active Capabilities"));
        assert!(current.contains("- implement"));
        assert!(current.contains(".vibehub/tasks/T-001/runs/R-001/context-packs/implement.md"));
        assert!(current_context.contains("- src/main.rs (required): implementation entrypoint"));
        assert!(current_context.contains("- Missing tests/main.rs (optional): no test file found"));
        assert!(current_context
            .contains("- Excluded .env.local (deny_secret_path): secret-like path denied"));
        assert!(handoff.contains("# Handoff from Session unknown"));
        assert!(handoff.contains("Handoff complete: no"));
        assert!(handoff.contains("## Missing Required Sections"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn current_view_lists_multiple_active_capabilities_from_projection() {
        let project = temp_project();
        write_state(&project);
        write_manifest(&project);
        write_workflow(&project);
        write_current_task_pointer(&project, "T-001").expect("task pointer");
        write_current_run_pointer(&project, "T-001", "R-001").expect("run pointer");
        events::append_structured_run_event(
            &project,
            "T-001",
            "R-001",
            VibehubEvent::CapabilityClaimed {
                capability: "align".to_string(),
            },
        )
        .expect("claim align");
        events::append_structured_run_event(
            &project,
            "T-001",
            "R-001",
            VibehubEvent::CapabilityClaimed {
                capability: "research".to_string(),
            },
        )
        .expect("claim research");

        generate_agent_view(&project).expect("generate agent view");
        let current = fs::read_to_string(project.join(".vibehub/agent-view/current.md"))
            .expect("read current");

        assert!(current.contains("## Active Capabilities"));
        assert!(current.contains("- align"));
        assert!(current.contains("- research"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn refreshes_existing_handoff() {
        let project = temp_project();
        write_state(&project);
        write_manifest(&project);
        fs::write(
            project.join(".vibehub/agent-view/handoff.md"),
            "existing handoff\n",
        )
        .expect("write handoff");
        write_current_task_pointer(&project, "T-001").expect("task pointer");
        write_current_run_pointer(&project, "T-001", "R-001").expect("run pointer");

        let result = generate_agent_view(&project).expect("generate agent view");
        let handoff = fs::read_to_string(project.join(".vibehub/agent-view/handoff.md"))
            .expect("read handoff");

        assert!(!result.handoff_created);
        assert_ne!(handoff, "existing handoff\n");
        assert!(handoff.contains("Task: T-001"));

        fs::remove_dir_all(project).expect("cleanup");
    }
}

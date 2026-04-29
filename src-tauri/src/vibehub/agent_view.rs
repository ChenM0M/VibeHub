use crate::vibehub::current;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AgentViewGenerateResult {
    pub current_path: String,
    pub current_context_path: String,
    pub handoff_path: String,
    pub handoff_created: bool,
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
    context_pack_path: Option<String>,
    manifest_path: Option<String>,
    research_required: String,
    research_status: String,
    manifest: Option<ContextManifest>,
}

pub fn generate_agent_view(project_root: impl AsRef<Path>) -> Result<AgentViewGenerateResult> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    let task_pointer = current::resolve_current_task(&project_root)?;
    let run_pointer = current::resolve_current_run(&project_root, &task_pointer.task_id)?;
    let state_path = project_root.join(".vibehub/state.yaml");
    let state = read_yaml_value(&state_path, "state.yaml")?;

    let phase = get_str(&state, &["current", "phase"]).unwrap_or("unknown");
    let input = AgentViewInput {
        task_id: task_pointer.task_id.clone(),
        task_path: task_pointer.path.clone(),
        run_id: run_pointer.run_id.clone(),
        run_path: run_pointer.path.clone(),
        mode: get_str(&state, &["current", "mode"])
            .unwrap_or("unknown")
            .to_string(),
        phase: phase.to_string(),
        phase_status: get_str(&state, &["current", "phase_status"])
            .unwrap_or("unknown")
            .to_string(),
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

    let handoff_created = if handoff_path.exists() {
        false
    } else {
        fs::write(&handoff_path, build_placeholder_handoff_md(&input))
            .with_context(|| format!("Failed to write {}", handoff_path.display()))?;
        true
    };

    Ok(AgentViewGenerateResult {
        current_path: normalize_path(&relative_to_project(&project_root, &current_path)?),
        current_context_path: normalize_path(&relative_to_project(
            &project_root,
            &current_context_path,
        )?),
        handoff_path: normalize_path(&relative_to_project(&project_root, &handoff_path)?),
        handoff_created,
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

    output.push_str("## Observability Note\n\n");
    output.push_str("- P0/P1 observability is best-effort.\n");
    output.push_str("- Hard observed: Git diff, filesystem state, and VibeHub-generated files.\n");
    output.push_str("- Agent reported: files read, commands run, summaries, and handoff notes.\n");
    output.push_str("- Inferred: task mapping, likely risk, and context completeness.\n");
    output.push_str("- Runtime observation is not enabled in P0.\n\n");

    output.push_str("## What To Read\n\n");
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

fn build_placeholder_handoff_md(input: &AgentViewInput) -> String {
    format!(
        r#"# VibeHub Handoff

## Completed
No previous handoff exists for task {task_id} run {run_id}.

## Not Yet Done
- Continue the current phase: {phase}.
- Report changed files, commands run, tests run or reason not run, unresolved risks, and handoff notes.

## Key Decisions
None recorded yet.

## Context Still Needed
Read .vibehub/agent-view/current-context.md and the current context pack before implementing.

## Warnings
P0/P1 observability is best-effort and does not include runtime observation.

## Next Session Should
Start from .vibehub/agent-view/current.md.
"#,
        task_id = input.task_id,
        run_id = input.run_id,
        phase = input.phase
    )
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

fn canonical_project_root(project_root: &Path) -> Result<PathBuf> {
    let project_root = fs::canonicalize(project_root)
        .with_context(|| format!("Project path does not exist: {}", project_root.display()))?;
    if !project_root.is_dir() {
        return Err(anyhow!(
            "Project path is not a directory: {}",
            project_root.display()
        ));
    }
    if !project_root.join(".vibehub").is_dir() {
        return Err(anyhow!(
            "VibeHub directory does not exist: {}",
            project_root.join(".vibehub").display()
        ));
    }
    Ok(project_root)
}

fn relative_to_project(project_root: &Path, target: &Path) -> Result<PathBuf> {
    target
        .strip_prefix(project_root)
        .map(PathBuf::from)
        .with_context(|| format!("Path escapes project root: {}", target.display()))
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vibehub::current::{write_current_run_pointer, write_current_task_pointer};
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
        assert!(current.contains(".vibehub/tasks/T-001/runs/R-001/context-packs/implement.md"));
        assert!(current_context.contains("- src/main.rs (required): implementation entrypoint"));
        assert!(current_context.contains("- Missing tests/main.rs (optional): no test file found"));
        assert!(current_context
            .contains("- Excluded .env.local (deny_secret_path): secret-like path denied"));
        assert!(handoff.contains("No previous handoff exists for task T-001 run R-001."));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn does_not_overwrite_existing_handoff() {
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
        assert_eq!(handoff, "existing handoff\n");

        fs::remove_dir_all(project).expect("cleanup");
    }
}

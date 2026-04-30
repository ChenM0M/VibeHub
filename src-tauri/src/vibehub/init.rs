use crate::vibehub::agent_adapter::{self, AgentTool};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VibehubInitResult {
    pub project_root: String,
    pub vibehub_root: String,
    pub created_files: Vec<String>,
    pub skipped_existing_files: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VibehubInitOptions {
    #[serde(default)]
    pub agent_tools: Option<Vec<AgentTool>>,
}

struct InitFile {
    path: &'static str,
    content: String,
}

pub fn init_project(project_path: impl AsRef<Path>) -> Result<VibehubInitResult> {
    init_project_with_options(project_path, None)
}

pub fn init_project_with_options(
    project_path: impl AsRef<Path>,
    options: Option<VibehubInitOptions>,
) -> Result<VibehubInitResult> {
    let project_root = fs::canonicalize(project_path.as_ref()).with_context(|| {
        format!(
            "Project path does not exist: {}",
            project_path.as_ref().display()
        )
    })?;

    if !project_root.is_dir() {
        return Err(anyhow!(
            "Project path is not a directory: {}",
            project_root.display()
        ));
    }

    let project_name = project_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("project");
    let vibehub_root = project_root.join(".vibehub");

    if vibehub_root.exists() && !vibehub_root.is_dir() {
        return Err(anyhow!(
            ".vibehub exists but is not a directory: {}",
            vibehub_root.display()
        ));
    }

    for dir in init_dirs() {
        fs::create_dir_all(vibehub_root.join(dir))
            .with_context(|| format!("Failed to create .vibehub directory: {}", dir))?;
    }

    let mut created_files = Vec::new();
    let mut skipped_existing_files = Vec::new();

    for file in init_files(project_name) {
        let target = vibehub_root.join(file.path);
        let relative = format_vibehub_path(&project_root, &target);

        if target.exists() {
            skipped_existing_files.push(relative);
            continue;
        }

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create parent directory for {}", relative))?;
        }

        fs::write(&target, file.content)
            .with_context(|| format!("Failed to write {}", relative))?;
        created_files.push(relative);
    }

    let config_path = vibehub_root.join("adapters/config.yaml");
    let config_relative = format_vibehub_path(&project_root, &config_path);
    let config_existed = config_path.exists();
    let agent_tools = options
        .and_then(|options| options.agent_tools)
        .unwrap_or_else(agent_adapter::default_tools);
    agent_adapter::ensure_adapter_config(&project_root, agent_tools.clone())?;
    if config_existed {
        skipped_existing_files.push(config_relative);
    } else {
        created_files.push(config_relative);
    }
    let adapter_sync = agent_adapter::sync_agent_adapters(&project_root, Some(agent_tools), false)?;
    created_files.extend(adapter_sync.created_files);
    created_files.extend(adapter_sync.updated_files);
    skipped_existing_files.extend(adapter_sync.skipped_files);
    let errors = adapter_sync
        .conflict_files
        .into_iter()
        .map(|conflict| format!("{}: {}", conflict.path, conflict.reason))
        .collect();

    Ok(VibehubInitResult {
        project_root: normalize_path(&project_root),
        vibehub_root: normalize_path(&vibehub_root),
        created_files,
        skipped_existing_files,
        errors,
    })
}

fn init_dirs() -> &'static [&'static str] {
    &[
        ".",
        "agent-view",
        "rules",
        "tasks",
        "research/current",
        "research/archive",
        "journal",
        "adapters/templates",
        "adapters/generated",
    ]
}

fn init_files(project_name: &str) -> Vec<InitFile> {
    vec![
        InitFile {
            path: "project.yaml",
            content: project_yaml(project_name),
        },
        InitFile {
            path: "state.yaml",
            content: state_yaml(project_name),
        },
        InitFile {
            path: "workflow.yaml",
            content: WORKFLOW_YAML.to_string(),
        },
        InitFile {
            path: "housekeeping.yaml",
            content: HOUSEKEEPING_YAML.to_string(),
        },
        InitFile {
            path: "agent-view/current.md",
            content: AGENT_CURRENT_MD.to_string(),
        },
        InitFile {
            path: "agent-view/current-context.md",
            content: AGENT_CURRENT_CONTEXT_MD.to_string(),
        },
        InitFile {
            path: "agent-view/handoff.md",
            content: AGENT_HANDOFF_MD.to_string(),
        },
        InitFile {
            path: "rules/phase-rules.yaml",
            content: PHASE_RULES_YAML.to_string(),
        },
        InitFile {
            path: "rules/research-triggers.yaml",
            content: RESEARCH_TRIGGERS_YAML.to_string(),
        },
        InitFile {
            path: "rules/autonomy.yaml",
            content: AUTONOMY_YAML.to_string(),
        },
        InitFile {
            path: "rules/review.yaml",
            content: REVIEW_YAML.to_string(),
        },
        InitFile {
            path: "rules/loop-detection.yaml",
            content: LOOP_DETECTION_YAML.to_string(),
        },
        InitFile {
            path: "rules/hard-rules.md",
            content: HARD_RULES_MD.to_string(),
        },
        InitFile {
            path: "rules/preferences.yaml",
            content: PREFERENCES_YAML.to_string(),
        },
        InitFile {
            path: "journal/index.md",
            content: JOURNAL_INDEX_MD.to_string(),
        },
        InitFile {
            path: "adapters/sync-state.yaml",
            content: ADAPTER_SYNC_STATE_YAML.to_string(),
        },
    ]
}

fn project_yaml(project_name: &str) -> String {
    let project_name = yaml_string(project_name);
    format!(
        r#"schema_version: 1
kind: vibehub_project
name: {project_name}
root: "."
protocol_version: "2.0-r10"
initialized_by: vibehub
"#
    )
}

fn state_yaml(project_name: &str) -> String {
    let project_name = yaml_string(project_name);
    format!(
        r#"schema_version: 1

project:
  id: {project_name}
  name: {project_name}
  root: "."

current:
  mode: guided_drive
  task_id: null
  run_id: null
  session_id: null
  phase: null
  phase_status: idle

pointers:
  task_pointer: ".vibehub/tasks/current"
  run_pointer: null

flow:
  align: pending
  research: pending
  plan: pending
  implement: pending
  review: pending

observability:
  level: best_effort
  runtime_adapter: none
  current_observation_sources:
    - vibehub_generated

autonomy:
  level: high

research:
  required: false
  status: not_started
  scope: active_task
  skipped_reason: null

context:
  current_pack: null
  current_manifest: null
  stale: false
  generated_by: vibehub_backend

agent_report:
  status: not_started
  validation_status: pending

handoff:
  current: ".vibehub/agent-view/handoff.md"
  status: empty

git:
  baseline_commit: null
  last_seen_head: null
  dirty: null
  changed_files_count: 0

loop_detection:
  status: normal
  warnings: []

last_updated: null
resume_hint: "No active task. Create or select a task before starting a run."
"#
    )
}

const WORKFLOW_YAML: &str = r#"schema_version: 1
name: vibehub-default-4plus1
philosophy: observable_yolo

modes:
  yolo_drive:
    phases:
      - align_lite
      - implement
      - review_lite

  guided_drive:
    phases:
      - align
      - plan
      - implement
      - review

  evidence_drive:
    phases:
      - align
      - research
      - plan
      - implement
      - review

default_mode: guided_drive

phase_order:
  - align_lite
  - align
  - research
  - plan
  - implement
  - review_lite
  - review
"#;

const HOUSEKEEPING_YAML: &str = r#"retention:
  max_active_tasks: 1
  max_runs_kept_per_task: 5
  max_sessions_kept_per_run: 10
  max_context_pack_versions_per_phase: 3
  max_research_archives_kept: 10

archive:
  strategy: compress_and_gitignore
  archive_dir: ".vibehub/archive"
  compress_after_days: 30
  archive_completed_runs_after_days: 14
  archive_old_sessions_after_days: 14

agent_visible_state:
  include:
    - ".vibehub/agent-view/"
    - ".vibehub/state.yaml"
    - ".vibehub/workflow.yaml"
    - ".vibehub/rules/hard-rules.md"
    - ".vibehub/research/current/research-pack.md"
  exclude:
    - ".vibehub/archive/"
    - ".vibehub/tasks/*/runs/*/sessions/*/transcript.md"
    - ".vibehub/tasks/*/runs/*/events.jsonl"
    - ".vibehub/adapters/generated/"
"#;

const PHASE_RULES_YAML: &str = r#"align_lite:
  required_outputs:
    - intent
  optional_outputs:
    - acceptance_criteria
    - affected_area
    - autonomy_level

align:
  required_outputs:
    - intent
    - acceptance_criteria
    - autonomy_level
  optional_outputs:
    - non_goals
    - research_decision
    - risk_level

research:
  optional: true
  required_outputs:
    - source_log
    - findings
    - research_pack

plan:
  required_outputs:
    - implementation_plan
    - validation_plan
    - context_plan

implement:
  required_outputs:
    - changed_files
    - implementation_summary
    - unresolved_questions
  optional_outputs:
    - files_reportedly_read
    - commands_reportedly_run
    - handoff_notes

review_lite:
  required_outputs:
    - diff_summary
    - verdict
  optional_outputs:
    - test_results_or_reason
    - changed_files
    - risk_note

review:
  required_outputs:
    - diff_summary
    - test_results_or_reason
    - verdict
    - risks
    - evidence_grades
"#;

const RESEARCH_TRIGGERS_YAML: &str = r#"strong:
  - external_framework_or_plugin
  - unfamiliar_sdk_or_api
  - migration
  - security_sensitive
  - performance_sensitive
  - architecture_refactor
  - compatibility_work
  - repeated_failure
  - user_requests_research

recommended:
  - ambiguous_requirement
  - unknown_best_practice
  - mature_reference_available
  - model_context_confidence_low

skip_by_default:
  - typo
  - simple_copy_change
  - small_css_change
  - mechanical_rename
  - obvious_local_fix
"#;

const AUTONOMY_YAML: &str = r#"default_level: high

agent_may:
  - inspect_relevant_source_files
  - propose_plan
  - implement_within_task_scope
  - self_review
  - request_more_context
  - suggest_state_changes

agent_must_not:
  - update_canonical_state_directly
  - mark_state_yaml_completed
  - claim_runtime_observation_without_adapter
  - bypass_review_evidence

vibehub_owns:
  - canonical_state_transitions
  - validation
  - context_pack_generation
  - handoff_generation
"#;

const REVIEW_YAML: &str = r#"evidence_grades:
  - hard_observed
  - agent_reported
  - inferred
  - user_confirmed

required_sections:
  - diff_summary
  - test_results_or_reason
  - verdict
  - risks
  - evidence_grades

review_lite:
  required_sections:
    - diff_summary
    - verdict
    - test_results_or_reason
"#;

const LOOP_DETECTION_YAML: &str = r#"status: enabled

signals:
  repeated_file_edits:
    threshold: 8
    evidence_grade: inferred
  repeated_review_failures:
    threshold: 2
    evidence_grade: inferred
  diff_scope_growth:
    ratio_threshold: 2.0
    evidence_grade: inferred

actions:
  - warn_user
  - recommend_review
  - recommend_rebuild_context
"#;

const PREFERENCES_YAML: &str = r#"schema_version: 1
preferences: {}
"#;

const ADAPTER_SYNC_STATE_YAML: &str = r#"schema_version: 1
status: placeholder
generated:
  files: []
conflicts: []
last_sync: null
"#;

const AGENT_CURRENT_MD: &str = r#"# VibeHub Current State

No active task has been created yet.

## Observability Note
P0/P1 observability is best-effort.
Report files read, commands run, decisions made, and unresolved risks.

## What you should read
1. .vibehub/agent-view/current-context.md
2. .vibehub/agent-view/handoff.md
3. .vibehub/rules/hard-rules.md

## Stop condition
Do not mark state.yaml completed.
Return to VibeHub for validation.
"#;

const AGENT_CURRENT_CONTEXT_MD: &str = r#"# Current Context

## Context Pack
Not generated yet.

## Manifest
Not generated yet.

## Important Project Files
No task-specific files selected yet.

## Research Pack
Not required yet.

## Known Missing Context
No active task has been created yet.
"#;

const AGENT_HANDOFF_MD: &str = r#"# Handoff

## Completed
No active session yet.

## Not Yet Done
- Create or select a task.
- Build a context pack for the first active phase.

## Key Decisions
None yet.

## Context Still Needed
Task-specific context has not been selected yet.

## Warnings
P0/P1 observability is best-effort and does not include runtime interception.

## Next Session Should
Start from .vibehub/agent-view/current.md after VibeHub creates an active task.
"#;

const HARD_RULES_MD: &str = r#"# VibeHub Hard Rules

- Agent output is reported state only.
- Only VibeHub code updates canonical state transitions.
- Do not mark state.yaml completed from agent output.
- Distinguish hard_observed, agent_reported, inferred, and user_confirmed evidence.
- P0/P1 observability is best-effort and must not claim full runtime observation.
- Agents should read agent-view files and the current context pack, not the whole .vibehub directory.
- Keep changes scoped to the active task.
"#;

const JOURNAL_INDEX_MD: &str = r#"# VibeHub Journal

No journal entries yet.
"#;

fn format_vibehub_path(project_root: &Path, target: &Path) -> String {
    target
        .strip_prefix(project_root)
        .map(normalize_path)
        .unwrap_or_else(|_| normalize_path(target))
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn yaml_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn temp_project() -> PathBuf {
        let path = std::env::temp_dir().join(format!("vibehub-init-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).expect("create temp project");
        path
    }

    #[test]
    fn creates_minimal_vibehub_structure() {
        let project = temp_project();
        let result = init_project(&project).expect("init project");

        assert!(project.join(".vibehub/project.yaml").exists());
        assert!(project.join(".vibehub/state.yaml").exists());
        assert!(project.join(".vibehub/workflow.yaml").exists());
        assert!(project.join(".vibehub/housekeeping.yaml").exists());
        assert!(project.join(".vibehub/agent-view/current.md").exists());
        assert!(project.join(".vibehub/rules/hard-rules.md").exists());
        assert!(project.join(".vibehub/tasks").is_dir());
        assert!(project.join(".vibehub/research/current").is_dir());
        assert!(project.join(".vibehub/research/archive").is_dir());
        assert!(project.join(".vibehub/journal/index.md").exists());
        assert!(project.join(".vibehub/adapters/sync-state.yaml").exists());
        assert!(project.join(".vibehub/adapters/templates").is_dir());
        assert!(project.join(".vibehub/adapters/generated").is_dir());
        assert!(result.errors.is_empty());
        assert!(result
            .created_files
            .contains(&".vibehub/project.yaml".to_string()));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn skips_existing_files_without_overwriting() {
        let project = temp_project();
        let vibehub = project.join(".vibehub");
        fs::create_dir_all(&vibehub).expect("create .vibehub");
        fs::write(vibehub.join("state.yaml"), "custom: true\n").expect("write custom state");

        let result = init_project(&project).expect("init project");
        let state = fs::read_to_string(vibehub.join("state.yaml")).expect("read state");

        assert_eq!(state, "custom: true\n");
        assert!(result
            .skipped_existing_files
            .contains(&".vibehub/state.yaml".to_string()));
        assert!(!result
            .created_files
            .contains(&".vibehub/state.yaml".to_string()));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn second_run_is_idempotent() {
        let project = temp_project();
        let first = init_project(&project).expect("first init");
        let second = init_project(&project).expect("second init");

        assert!(!first.created_files.is_empty());
        assert!(second.created_files.is_empty());
        assert_eq!(
            second.skipped_existing_files.len(),
            first.created_files.len()
        );

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn rejects_file_project_path() {
        let project = temp_project();
        let file = project.join("not-a-directory.txt");
        fs::write(&file, "x").expect("write file");

        let err = init_project(&file).expect_err("file path should fail");
        assert!(err.to_string().contains("not a directory"));

        fs::remove_dir_all(project).expect("cleanup");
    }
}

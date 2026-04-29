use crate::vibehub::{agent_view, context, current, handoff, init, review, status};
use anyhow::{anyhow, Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use serde_yaml::{Mapping, Value};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const DEFAULT_MODE: &str = "guided_drive";
const DEFAULT_PHASE: &str = "implement";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VibehubStartTaskResult {
    pub task_id: String,
    pub run_id: String,
    pub mode: String,
    pub phase: String,
    pub phase_status: String,
    pub task_path: String,
    pub run_path: String,
    pub task_pointer_path: String,
    pub run_pointer_path: String,
    pub context_spec_path: String,
}

pub fn start_task(
    project_root: impl AsRef<Path>,
    title: Option<String>,
    mode: Option<String>,
    phase: Option<String>,
) -> Result<VibehubStartTaskResult> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    ensure_initialized(&project_root)?;

    let mode = validate_id("mode", mode.as_deref().unwrap_or(DEFAULT_MODE))?.to_string();
    let phase = validate_id("phase", phase.as_deref().unwrap_or(DEFAULT_PHASE))?.to_string();
    let phase_status = "active".to_string();
    let task_id = next_id("T");
    let run_id = next_id("R");

    let task_dir = project_root.join(".vibehub").join("tasks").join(&task_id);
    let context_dir = task_dir.join("context");
    let run_dir = task_dir.join("runs").join(&run_id);
    fs::create_dir_all(&context_dir)
        .with_context(|| format!("Failed to create {}", context_dir.display()))?;
    fs::create_dir_all(run_dir.join("sessions"))
        .with_context(|| format!("Failed to create {}", run_dir.join("sessions").display()))?;
    fs::create_dir_all(run_dir.join("outputs"))
        .with_context(|| format!("Failed to create {}", run_dir.join("outputs").display()))?;

    let task_title = title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("VibeHub task");
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let task_path = normalize_path(&relative_to_project(&project_root, &task_dir)?);
    let run_path = normalize_path(&relative_to_project(&project_root, &run_dir)?);

    fs::write(
        task_dir.join("task.yaml"),
        task_yaml(&task_id, task_title, &mode, &phase, &phase_status, &now),
    )
    .with_context(|| format!("Failed to write {}", task_dir.join("task.yaml").display()))?;
    fs::write(
        run_dir.join("run.yaml"),
        run_yaml(&task_id, &run_id, &mode, &phase, &phase_status, &now),
    )
    .with_context(|| format!("Failed to write {}", run_dir.join("run.yaml").display()))?;

    let context_spec_path = context_dir.join(format!("{phase}.yaml"));
    fs::write(
        &context_spec_path,
        context_spec_yaml(&task_id, &run_id, &phase),
    )
    .with_context(|| format!("Failed to write {}", context_spec_path.display()))?;

    current::write_current_task_pointer(&project_root, &task_id)?;
    current::write_current_run_pointer(&project_root, &task_id, &run_id)?;
    update_state(
        &project_root,
        &task_id,
        &run_id,
        &mode,
        &phase,
        &phase_status,
        &run_path,
    )?;

    Ok(VibehubStartTaskResult {
        task_id,
        run_id,
        mode,
        phase,
        phase_status,
        task_path,
        run_path,
        task_pointer_path: ".vibehub/tasks/current".to_string(),
        run_pointer_path: format!(
            "{}/runs/current",
            normalize_path(&relative_to_project(&project_root, &task_dir)?)
        ),
        context_spec_path: normalize_path(&relative_to_project(&project_root, &context_spec_path)?),
    })
}

fn ensure_initialized(project_root: &Path) -> Result<()> {
    init::init_project(project_root)?;
    Ok(())
}

fn update_state(
    project_root: &Path,
    task_id: &str,
    run_id: &str,
    mode: &str,
    phase: &str,
    phase_status: &str,
    run_path: &str,
) -> Result<()> {
    let state_path = project_root.join(".vibehub/state.yaml");
    let mut state = if state_path.is_file() {
        let content = fs::read_to_string(&state_path)
            .with_context(|| format!("Failed to read {}", state_path.display()))?;
        serde_yaml::from_str::<Value>(&content)
            .with_context(|| format!("Invalid YAML in {}", state_path.display()))?
    } else {
        Value::Mapping(Mapping::new())
    };

    set_string(&mut state, &["current", "mode"], mode);
    set_string(&mut state, &["current", "task_id"], task_id);
    set_string(&mut state, &["current", "run_id"], run_id);
    set_null(&mut state, &["current", "session_id"]);
    set_string(&mut state, &["current", "phase"], phase);
    set_string(&mut state, &["current", "phase_status"], phase_status);
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
    set_string(&mut state, &["flow", phase], phase_status);
    set_string(
        &mut state,
        &["context", "current_pack"],
        &format!("{run_path}/context-packs/{phase}.md"),
    );
    set_string(
        &mut state,
        &["context", "current_manifest"],
        &format!("{run_path}/context-packs/{phase}.manifest.yaml"),
    );
    set_bool(&mut state, &["context", "stale"], false);
    set_string(&mut state, &["context", "generated_by"], "vibehub_backend");
    set_bool(&mut state, &["research", "required"], false);
    set_string(&mut state, &["research", "status"], "skipped");
    set_string(
        &mut state,
        &["handoff", "current"],
        ".vibehub/agent-view/handoff.md",
    );
    set_string(&mut state, &["handoff", "status"], "empty");
    set_string(&mut state, &["observability", "level"], "best_effort");
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

    let content = serde_yaml::to_string(&state).context("Failed to serialize state.yaml")?;
    fs::write(&state_path, content)
        .with_context(|| format!("Failed to write {}", state_path.display()))
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

fn task_yaml(
    task_id: &str,
    title: &str,
    mode: &str,
    phase: &str,
    phase_status: &str,
    created_at: &str,
) -> String {
    format!(
        r#"schema_version: 1
kind: vibehub_task
task_id: {}
title: {}
mode: {}
phase: {}
phase_status: {}
created_at: {}
created_by: vibehub
"#,
        yaml_string(task_id),
        yaml_string(title),
        yaml_string(mode),
        yaml_string(phase),
        yaml_string(phase_status),
        yaml_string(created_at)
    )
}

fn run_yaml(
    task_id: &str,
    run_id: &str,
    mode: &str,
    phase: &str,
    phase_status: &str,
    created_at: &str,
) -> String {
    format!(
        r#"schema_version: 1
kind: vibehub_run
task_id: {}
run_id: {}
mode: {}
phase: {}
phase_status: {}
created_at: {}
created_by: vibehub
baseline_commit: null
"#,
        yaml_string(task_id),
        yaml_string(run_id),
        yaml_string(mode),
        yaml_string(phase),
        yaml_string(phase_status),
        yaml_string(created_at)
    )
}

fn context_spec_yaml(task_id: &str, run_id: &str, phase: &str) -> String {
    format!(
        r#"phase: {}
task_id: {}
run_id: {}
known_missing_context: []
stop_condition: Write output.md and return to VibeHub for validation.
entries:
  - path: .vibehub/tasks/{}/task.yaml
    type: file
    reason: active task metadata and goal
    required: true
  - path: .vibehub/tasks/{}/runs/{}/run.yaml
    type: file
    reason: active run metadata
    required: true
  - path: .vibehub/rules/hard-rules.md
    type: file
    reason: protocol hard rules
    required: true
"#,
        yaml_string(phase),
        yaml_string(task_id),
        yaml_string(run_id),
        task_id,
        task_id,
        run_id
    )
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
    Ok(project_root)
}

fn next_id(prefix: &str) -> String {
    let timestamp = Utc::now().format("%Y%m%d%H%M%S");
    let suffix = Uuid::new_v4().simple().to_string();
    format!("{prefix}-{timestamp}-{}", &suffix[..8])
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

fn relative_to_project(project_root: &Path, target: &Path) -> Result<PathBuf> {
    target
        .strip_prefix(project_root)
        .map(PathBuf::from)
        .with_context(|| format!("Path escapes project root: {}", target.display()))
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

    fn temp_project() -> PathBuf {
        let path = std::env::temp_dir().join(format!("vibehub-start-task-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).expect("create temp project");
        path
    }

    #[test]
    fn golden_path_init_start_context_agent_view_review_and_handoff() {
        let project = temp_project();

        init::init_project(&project).expect("init");
        let started = start_task(
            &project,
            Some("Backend P0 file protocol bootstrap".to_string()),
            None,
            None,
        )
        .expect("start task");

        assert!(project.join(&started.task_path).is_dir());
        assert!(project.join(&started.run_path).is_dir());
        assert!(project.join(&started.context_spec_path).is_file());

        let status_after_start = status::read_cockpit_status(&project).expect("status");
        assert_eq!(
            status_after_start.current_task_id.as_deref(),
            Some(started.task_id.as_str())
        );
        assert_eq!(
            status_after_start.current_run_id.as_deref(),
            Some(started.run_id.as_str())
        );
        assert_eq!(
            status_after_start.current_mode.as_deref(),
            Some("guided_drive")
        );
        assert_eq!(
            status_after_start.current_phase.as_deref(),
            Some("implement")
        );
        assert_eq!(status_after_start.phase_status.as_deref(), Some("active"));

        let pack =
            context::build_context_pack(&project, &started.task_id, &started.run_id, "implement")
                .expect("build context");
        assert_eq!(pack.missing_count, 0);
        assert!(project.join(&pack.pack_path).is_file());
        assert!(project.join(&pack.manifest_path).is_file());

        let agent_view = agent_view::generate_agent_view(&project).expect("agent view");
        assert_eq!(agent_view.task_id, started.task_id);
        assert_eq!(agent_view.run_id, started.run_id);
        assert_eq!(agent_view.phase, "implement");
        assert!(project.join(&agent_view.current_path).is_file());
        assert!(project.join(&agent_view.current_context_path).is_file());

        let session_dir = project
            .join(&started.run_path)
            .join("sessions")
            .join("S-001");
        fs::create_dir_all(&session_dir).expect("create session");
        fs::write(
            session_dir.join("output.md"),
            r#"# Session Output

## Completed
- Implemented backend P0 start task.

## Not Yet Done
- Frontend cockpit UI is out of scope.

## Key Decisions Made
- Use VibeHub-generated YAML pointers as canonical current state.

## Files Changed
- src-tauri/src/vibehub/start_task.rs

## Files Reportedly Read
- .vibehub/agent-view/current.md

## Context Still Needed
- None.

## Warnings
- Runtime observation is not enabled in P0.

## Next Session Should
- Run final verification.

## Tests Run
- cargo test vibehub::start_task
"#,
        )
        .expect("write output");

        let review = review::generate_review_evidence(&project).expect("review");
        assert!(project.join(&review.review_path).is_file());
        assert_eq!(review.task_id, started.task_id);
        assert_eq!(review.run_id, started.run_id);

        let handoff = handoff::build_handoff(&project).expect("handoff");
        assert!(project.join(&handoff.handoff_path).is_file());
        assert_eq!(handoff.task_id, started.task_id);
        assert_eq!(handoff.run_id, started.run_id);
        assert!(handoff.complete);

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn completes_partial_vibehub_init_before_starting_task() {
        let project = temp_project();
        fs::create_dir_all(project.join(".vibehub/tasks")).expect("create partial vibehub");

        let started = start_task(
            &project,
            Some("Partial init recovery".to_string()),
            None,
            None,
        )
        .expect("start task");
        let pack =
            context::build_context_pack(&project, &started.task_id, &started.run_id, "implement")
                .expect("build context");

        assert!(project.join(".vibehub/rules/hard-rules.md").is_file());
        assert_eq!(pack.missing_count, 0);

        fs::remove_dir_all(project).expect("cleanup");
    }
}

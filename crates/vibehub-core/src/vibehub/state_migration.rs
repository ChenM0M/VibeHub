// Schema-version detection and forward migration for `.vibehub/state.yaml`.
//
// r9 -> r10 introduced `align_lite` and `review_lite` to the flow map. Older
// state files written by r9 only contain `align`, `research`, `plan`,
// `implement`, `review`. Schema v3 adds a run-scoped `current.event_log_path`
// so readers can discover the append-only event stream without recomputing it.
// Schema v4 marks phase/status/flow as projection-derived and records the
// trace path used to audit event-to-state derivation. Schema v5 adds the
// metrics bucket written by the M5 fitness projection.

use crate::vibehub::{fitness, util::canonical_initialized_project_root};
use anyhow::{Context, Result};
use serde::Serialize;
use serde_yaml::{Mapping, Value};
use std::fs;
use std::path::{Path, PathBuf};

pub const CURRENT_SCHEMA_VERSION: u32 = 5;

const R10_FLOW_PHASES: &[&str] = &[
    "align_lite",
    "align",
    "research",
    "plan",
    "implement",
    "review_lite",
    "review",
];

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Default)]
pub struct StateMigrationReport {
    pub state_path: String,
    pub previous_schema_version: Option<u32>,
    pub current_schema_version: u32,
    pub migrated: bool,
    pub backup_path: Option<String>,
    pub added_keys: Vec<String>,
    pub notes: Vec<String>,
}

/// Inspect the on-disk `state.yaml` and return what a forward migration would
/// change, without writing anything. Use this when surfacing a "preview" to
/// the user.
pub fn dry_run(project_root: impl AsRef<Path>) -> Result<StateMigrationReport> {
    migrate_state(project_root.as_ref(), false)
}

/// Run the actual migration: rewrite `.vibehub/state.yaml` in place and store
/// the original at `.vibehub/state.yaml.bak.<schema>` so the user can roll back.
pub fn migrate(project_root: impl AsRef<Path>) -> Result<StateMigrationReport> {
    migrate_state(project_root.as_ref(), true)
}

fn migrate_state(project_root: &Path, write: bool) -> Result<StateMigrationReport> {
    let project_root = canonical_initialized_project_root(project_root)?;
    let state_path = project_root.join(".vibehub").join("state.yaml");
    let mut report = StateMigrationReport {
        state_path: relative_path(&project_root, &state_path),
        previous_schema_version: None,
        current_schema_version: CURRENT_SCHEMA_VERSION,
        ..Default::default()
    };

    if !state_path.is_file() {
        report
            .notes
            .push("state.yaml is missing; nothing to migrate.".to_string());
        return Ok(report);
    }

    let original = fs::read_to_string(&state_path)
        .with_context(|| format!("Failed to read {}", state_path.display()))?;

    let parsed = match serde_yaml::from_str::<Value>(&original) {
        Ok(v) => v,
        Err(err) => {
            report.notes.push(format!(
                "state.yaml could not be parsed as YAML: {err}. Migration skipped."
            ));
            return Ok(report);
        }
    };

    let mut state = parsed.clone();
    let previous = read_schema_version(&state);
    report.previous_schema_version = previous;

    let mut changed = false;

    if previous != Some(CURRENT_SCHEMA_VERSION) {
        set_value(
            &mut state,
            &["schema_version"],
            Value::Number(CURRENT_SCHEMA_VERSION.into()),
        );
        changed = true;
        report.notes.push(format!(
            "schema_version: {} -> {}",
            previous
                .map(|v| v.to_string())
                .unwrap_or_else(|| "none".to_string()),
            CURRENT_SCHEMA_VERSION
        ));
    }

    // Ensure r10 flow keys exist.
    for phase in R10_FLOW_PHASES {
        let path = ["flow", *phase];
        if state.get("flow").and_then(|f| f.get(*phase)).is_none() {
            set_value(&mut state, &path, Value::String("pending".to_string()));
            report.added_keys.push(format!("flow.{phase}"));
            changed = true;
        }
    }

    // Ensure loop_detection bucket exists for the loop-warning UI.
    if state.get("loop_detection").is_none() {
        let mut bucket = Mapping::new();
        bucket.insert(
            Value::String("status".to_string()),
            Value::String("normal".to_string()),
        );
        bucket.insert(
            Value::String("warnings".to_string()),
            Value::Sequence(Vec::new()),
        );
        set_value(&mut state, &["loop_detection"], Value::Mapping(bucket));
        report.added_keys.push("loop_detection".to_string());
        changed = true;
    }

    // Ensure observability bucket exists with best_effort default.
    if state.get("observability").is_none() {
        let mut bucket = Mapping::new();
        bucket.insert(
            Value::String("level".to_string()),
            Value::String("best_effort".to_string()),
        );
        bucket.insert(
            Value::String("runtime_adapter".to_string()),
            Value::String("none".to_string()),
        );
        set_value(&mut state, &["observability"], Value::Mapping(bucket));
        report.added_keys.push("observability".to_string());
        changed = true;
    }

    if state
        .get("tasks")
        .and_then(|tasks| tasks.get("active"))
        .is_none()
    {
        let active = current_task_id(&state)
            .map(|task_id| Value::Sequence(vec![Value::String(task_id)]))
            .unwrap_or_else(|| Value::Sequence(Vec::new()));
        set_value(&mut state, &["tasks", "active"], active);
        report.added_keys.push("tasks.active".to_string());
        changed = true;
    }

    if state.get("metrics").is_none() {
        set_value(&mut state, &["metrics"], fitness::default_metrics_yaml());
        report.added_keys.push("metrics".to_string());
        changed = true;
    }

    if state
        .get("current")
        .and_then(|current| current.get("event_log_path"))
        .is_none()
    {
        let event_log_path = current_event_log_path(&state);
        set_value(&mut state, &["current", "event_log_path"], event_log_path);
        report.added_keys.push("current.event_log_path".to_string());
        changed = true;
    }

    if state
        .get("derived")
        .and_then(|derived| derived.get("current"))
        .and_then(|current| current.get("phase"))
        .is_none()
    {
        set_value(
            &mut state,
            &["derived", "current", "phase"],
            Value::Bool(true),
        );
        report.added_keys.push("derived.current.phase".to_string());
        changed = true;
    }

    if state
        .get("derived")
        .and_then(|derived| derived.get("current"))
        .and_then(|current| current.get("phase_status"))
        .is_none()
    {
        set_value(
            &mut state,
            &["derived", "current", "phase_status"],
            Value::Bool(true),
        );
        report
            .added_keys
            .push("derived.current.phase_status".to_string());
        changed = true;
    }

    if state
        .get("derived")
        .and_then(|derived| derived.get("flow"))
        .is_none()
    {
        set_value(&mut state, &["derived", "flow"], Value::Bool(true));
        report.added_keys.push("derived.flow".to_string());
        changed = true;
    }

    if state
        .get("derived")
        .and_then(|derived| derived.get("trace_path"))
        .is_none()
    {
        set_value(
            &mut state,
            &["derived", "trace_path"],
            Value::String(".vibehub/derivation_trace.yaml".to_string()),
        );
        report.added_keys.push("derived.trace_path".to_string());
        changed = true;
    }

    if !changed {
        return Ok(report);
    }

    if write {
        let backup_name = format!(
            "state.yaml.bak.r{}",
            previous
                .map(|v| v.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        );
        let backup_path = state_path.with_file_name(&backup_name);
        fs::write(&backup_path, &original)
            .with_context(|| format!("Failed to write backup {}", backup_path.display()))?;
        let serialized =
            serde_yaml::to_string(&state).context("Failed to serialize migrated state.yaml")?;
        fs::write(&state_path, serialized)
            .with_context(|| format!("Failed to write {}", state_path.display()))?;
        report.backup_path = Some(relative_path(&project_root, &backup_path));
        report.migrated = true;
    } else {
        report
            .notes
            .push("Dry run: migration not applied. Call migrate() to write changes.".to_string());
    }

    Ok(report)
}

fn read_schema_version(value: &Value) -> Option<u32> {
    value
        .get("schema_version")
        .and_then(Value::as_u64)
        .map(|v| v as u32)
}

fn current_event_log_path(state: &Value) -> Value {
    match (
        yaml_string(state, &["current", "task_id"]),
        yaml_string(state, &["current", "run_id"]),
    ) {
        (Some(task_id), Some(run_id)) => Value::String(format!(
            ".vibehub/tasks/{task_id}/runs/{run_id}/events.jsonl"
        )),
        _ => Value::Null,
    }
}

fn current_task_id(state: &Value) -> Option<String> {
    state
        .get("current")
        .and_then(|current| current.get("task_id"))
        .and_then(Value::as_str)
        .filter(|task_id| !task_id.trim().is_empty())
        .map(ToString::to_string)
}

fn yaml_string(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str().map(ToString::to_string)
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
        let mapping = current.as_mapping_mut().expect("mapping");
        current = mapping
            .entry(Value::String((*key).to_string()))
            .or_insert_with(|| Value::Mapping(Mapping::new()));
        if !matches!(current, Value::Mapping(_)) {
            *current = Value::Mapping(Mapping::new());
        }
    }
    let mapping = current.as_mapping_mut().expect("mapping");
    mapping.insert(Value::String(path[path.len() - 1].to_string()), next);
}

fn relative_path(project_root: &Path, target: &Path) -> String {
    target
        .strip_prefix(project_root)
        .map(PathBuf::from)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| target.to_string_lossy().replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_project() -> PathBuf {
        let p = std::env::temp_dir().join(format!("vibehub-migration-test-{}", Uuid::new_v4()));
        fs::create_dir_all(p.join(".vibehub")).expect("create");
        p
    }

    #[test]
    fn dry_run_reports_added_keys_without_writing() {
        let project = temp_project();
        let state_path = project.join(".vibehub/state.yaml");
        fs::write(
            &state_path,
            "schema_version: 1\nflow:\n  align: pending\n  implement: pending\n",
        )
        .expect("write old state");

        let report = dry_run(&project).expect("dry run");

        assert_eq!(report.previous_schema_version, Some(1));
        assert_eq!(report.current_schema_version, CURRENT_SCHEMA_VERSION);
        assert!(!report.migrated);
        assert!(report.added_keys.iter().any(|k| k == "flow.align_lite"));
        assert!(report.added_keys.iter().any(|k| k == "flow.review_lite"));
        let on_disk = fs::read_to_string(&state_path).unwrap();
        assert!(on_disk.contains("schema_version: 1")); // unchanged

        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn migrate_writes_backup_and_bumps_schema() {
        let project = temp_project();
        let state_path = project.join(".vibehub/state.yaml");
        fs::write(
            &state_path,
            "schema_version: 1\nflow:\n  align: pending\n  implement: pending\n  review: pending\n",
        )
        .expect("write old state");

        let report = migrate(&project).expect("migrate");

        assert!(report.migrated);
        assert!(report.backup_path.is_some());
        let new_content = fs::read_to_string(&state_path).unwrap();
        assert!(new_content.contains("schema_version: 5"));
        assert!(new_content.contains("align_lite"));
        assert!(new_content.contains("review_lite"));
        assert!(new_content.contains("event_log_path: null"));
        assert!(new_content.contains("trace_path: .vibehub/derivation_trace.yaml"));
        assert!(project.join(".vibehub/state.yaml.bak.r1").is_file());

        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn migration_is_idempotent_on_current_state() {
        let project = temp_project();
        let state_path = project.join(".vibehub/state.yaml");
        fs::write(
            &state_path,
            "schema_version: 5\ncurrent:\n  event_log_path: null\ntasks:\n  active: []\nflow:\n  align_lite: pending\n  align: pending\n  research: pending\n  plan: pending\n  implement: pending\n  review_lite: pending\n  review: pending\nderived:\n  current:\n    phase: true\n    phase_status: true\n  flow: true\n  trace_path: .vibehub/derivation_trace.yaml\nloop_detection:\n  status: normal\n  warnings: []\nobservability:\n  level: best_effort\n  runtime_adapter: none\nmetrics:\n  tasks:\n    active_count: 0\n",
        )
        .expect("write current state");

        let report = migrate(&project).expect("migrate");

        assert!(!report.migrated);
        assert!(report.added_keys.is_empty());

        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn v2_to_v4_adds_event_log_path_and_derivation_markers() {
        let project = temp_project();
        let state_path = project.join(".vibehub/state.yaml");
        fs::write(
            &state_path,
            concat!(
                "schema_version: 2\n",
                "project:\n  id: legacy\n  name: Legacy Project\n",
                "current:\n",
                "  mode: evidence_drive\n",
                "  task_id: T-legacy\n",
                "  run_id: R-legacy\n",
                "  phase: research\n",
                "  phase_status: active\n",
                "flow:\n",
                "  align_lite: pending\n",
                "  align: completed\n",
                "  research: active\n",
                "  plan: pending\n",
                "  implement: pending\n",
                "  review_lite: pending\n",
                "  review: pending\n",
                "custom_bucket:\n",
                "  keep_me: yes\n",
                "loop_detection:\n",
                "  status: normal\n",
                "  warnings: []\n",
                "observability:\n",
                "  level: best_effort\n",
                "  runtime_adapter: none\n",
            ),
        )
        .expect("write v2 state");

        let report = migrate(&project).expect("migrate v2");

        assert!(report.migrated);
        assert_eq!(report.previous_schema_version, Some(2));
        assert_eq!(report.current_schema_version, 5);
        assert!(report
            .added_keys
            .iter()
            .any(|key| key == "current.event_log_path"));
        let migrated = fs::read_to_string(&state_path).expect("read migrated");
        assert!(migrated.contains("schema_version: 5"));
        assert!(migrated.contains("metrics:"));
        assert!(migrated.contains("name: Legacy Project"));
        assert!(migrated.contains("keep_me: yes"));
        assert!(
            migrated.contains("event_log_path: .vibehub/tasks/T-legacy/runs/R-legacy/events.jsonl")
        );
        assert!(migrated.contains("trace_path: .vibehub/derivation_trace.yaml"));
        assert!(migrated.contains("phase_status: true"));
        assert!(project.join(".vibehub/state.yaml.bak.r2").is_file());

        fs::remove_dir_all(project).ok();
    }
}

use anyhow::{anyhow, Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const SCHEMA_VERSION: u32 = 1;
const UPDATED_BY: &str = "vibehub";
const CURRENT_TASK_KIND: &str = "current_task_pointer";
const CURRENT_RUN_KIND: &str = "current_run_pointer";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurrentTaskPointer {
    pub schema_version: u32,
    pub kind: String,
    pub task_id: String,
    pub path: String,
    pub updated_at: String,
    pub updated_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurrentRunPointer {
    pub schema_version: u32,
    pub kind: String,
    pub task_id: String,
    pub run_id: String,
    pub path: String,
    pub updated_at: String,
    pub updated_by: String,
}

pub fn write_current_task_pointer(
    project_root: impl AsRef<Path>,
    task_id: impl AsRef<str>,
) -> Result<CurrentTaskPointer> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    let task_id = validate_id("task_id", task_id.as_ref())?;
    let task_path = project_root.join(".vibehub").join("tasks").join(task_id);

    if !task_path.is_dir() {
        return Err(anyhow!(
            "Cannot create current task pointer: task directory does not exist: {}",
            task_path.display()
        ));
    }

    let pointer = CurrentTaskPointer {
        schema_version: SCHEMA_VERSION,
        kind: CURRENT_TASK_KIND.to_string(),
        task_id: task_id.to_string(),
        path: normalize_path(&relative_to_project(&project_root, &task_path)?),
        updated_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        updated_by: UPDATED_BY.to_string(),
    };

    write_yaml_file(&project_root.join(".vibehub/tasks/current"), &pointer)?;
    Ok(pointer)
}

pub fn write_current_run_pointer(
    project_root: impl AsRef<Path>,
    task_id: impl AsRef<str>,
    run_id: impl AsRef<str>,
) -> Result<CurrentRunPointer> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    let task_id = validate_id("task_id", task_id.as_ref())?;
    let run_id = validate_id("run_id", run_id.as_ref())?;
    let run_path = project_root
        .join(".vibehub")
        .join("tasks")
        .join(task_id)
        .join("runs")
        .join(run_id);

    if !run_path.is_dir() {
        return Err(anyhow!(
            "Cannot create current run pointer: run directory does not exist: {}",
            run_path.display()
        ));
    }

    let pointer = CurrentRunPointer {
        schema_version: SCHEMA_VERSION,
        kind: CURRENT_RUN_KIND.to_string(),
        task_id: task_id.to_string(),
        run_id: run_id.to_string(),
        path: normalize_path(&relative_to_project(&project_root, &run_path)?),
        updated_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        updated_by: UPDATED_BY.to_string(),
    };

    write_yaml_file(
        &project_root
            .join(".vibehub")
            .join("tasks")
            .join(task_id)
            .join("runs")
            .join("current"),
        &pointer,
    )?;
    Ok(pointer)
}

pub fn resolve_current_task(project_root: impl AsRef<Path>) -> Result<CurrentTaskPointer> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    let pointer_path = project_root.join(".vibehub/tasks/current");
    let pointer: CurrentTaskPointer = read_yaml_file(&pointer_path, "current task pointer")
        .with_context(|| {
            format!(
                "Failed to resolve current task from {}",
                pointer_path.display()
            )
        })?;

    validate_common_pointer(
        "current task pointer",
        pointer.schema_version,
        &pointer.kind,
        CURRENT_TASK_KIND,
        &pointer.path,
        &pointer.updated_by,
    )?;
    validate_id("task_id", &pointer.task_id)?;

    let target = resolve_pointer_target(&project_root, &pointer.path, "current task pointer")?;
    if !target.is_dir() {
        return Err(anyhow!(
            "Broken current task pointer: target path is not a directory: {}",
            pointer.path
        ));
    }

    Ok(pointer)
}

pub fn resolve_current_run(
    project_root: impl AsRef<Path>,
    task_id: impl AsRef<str>,
) -> Result<CurrentRunPointer> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    let expected_task_id = validate_id("task_id", task_id.as_ref())?;
    let pointer_path = project_root
        .join(".vibehub")
        .join("tasks")
        .join(expected_task_id)
        .join("runs")
        .join("current");
    let pointer: CurrentRunPointer = read_yaml_file(&pointer_path, "current run pointer")
        .with_context(|| {
            format!(
                "Failed to resolve current run from {}",
                pointer_path.display()
            )
        })?;

    validate_common_pointer(
        "current run pointer",
        pointer.schema_version,
        &pointer.kind,
        CURRENT_RUN_KIND,
        &pointer.path,
        &pointer.updated_by,
    )?;
    validate_id("task_id", &pointer.task_id)?;
    validate_id("run_id", &pointer.run_id)?;

    if pointer.task_id != expected_task_id {
        return Err(anyhow!(
            "Invalid current run pointer: task_id '{}' does not match requested task_id '{}'",
            pointer.task_id,
            expected_task_id
        ));
    }

    let target = resolve_pointer_target(&project_root, &pointer.path, "current run pointer")?;
    if !target.is_dir() {
        return Err(anyhow!(
            "Broken current run pointer: target path is not a directory: {}",
            pointer.path
        ));
    }

    Ok(pointer)
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

    let vibehub_root = project_root.join(".vibehub");
    if !vibehub_root.is_dir() {
        return Err(anyhow!(
            "VibeHub directory does not exist: {}",
            vibehub_root.display()
        ));
    }

    Ok(project_root)
}

fn read_yaml_file<T>(path: &Path, label: &str) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    if !path.exists() {
        return Err(anyhow!("Missing {} file: {}", label, path.display()));
    }
    if !path.is_file() {
        return Err(anyhow!(
            "Invalid {} file: not a file: {}",
            label,
            path.display()
        ));
    }

    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    serde_yaml::from_str(&content)
        .with_context(|| format!("Invalid YAML in {}: {}", label, path.display()))
}

fn write_yaml_file<T>(path: &Path, value: &T) -> Result<()>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create pointer parent {}", parent.display()))?;
    }

    let content = serde_yaml::to_string(value)
        .with_context(|| format!("Failed to serialize pointer {}", path.display()))?;
    fs::write(path, content).with_context(|| format!("Failed to write {}", path.display()))
}

fn validate_common_pointer(
    label: &str,
    schema_version: u32,
    kind: &str,
    expected_kind: &str,
    path: &str,
    updated_by: &str,
) -> Result<()> {
    if schema_version != SCHEMA_VERSION {
        return Err(anyhow!(
            "Invalid {}: schema_version {} is not supported; expected {}",
            label,
            schema_version,
            SCHEMA_VERSION
        ));
    }
    if kind != expected_kind {
        return Err(anyhow!(
            "Invalid {}: kind '{}' does not match expected kind '{}'",
            label,
            kind,
            expected_kind
        ));
    }
    if path.trim().is_empty() {
        return Err(anyhow!("Invalid {}: path is empty", label));
    }
    if updated_by != UPDATED_BY {
        return Err(anyhow!(
            "Invalid {}: updated_by '{}' does not match expected writer '{}'",
            label,
            updated_by,
            UPDATED_BY
        ));
    }

    Ok(())
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

fn resolve_pointer_target(project_root: &Path, pointer_path: &str, label: &str) -> Result<PathBuf> {
    let raw = Path::new(pointer_path);
    if raw.is_absolute() {
        return Err(anyhow!(
            "Invalid {}: path must be project-relative: {}",
            label,
            pointer_path
        ));
    }

    let target = fs::canonicalize(project_root.join(raw)).with_context(|| {
        format!(
            "Broken {}: target path does not exist: {}",
            label, pointer_path
        )
    })?;
    if !target.starts_with(project_root) {
        return Err(anyhow!(
            "Invalid {}: target path escapes project root: {}",
            label,
            pointer_path
        ));
    }

    Ok(target)
}

fn relative_to_project(project_root: &Path, target: &Path) -> Result<PathBuf> {
    target
        .strip_prefix(project_root)
        .map(PathBuf::from)
        .with_context(|| format!("Pointer target escapes project root: {}", target.display()))
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_project() -> PathBuf {
        let path = std::env::temp_dir().join(format!("vibehub-current-test-{}", Uuid::new_v4()));
        fs::create_dir_all(path.join(".vibehub/tasks")).expect("create temp project");
        path
    }

    fn create_task(project: &Path, task_id: &str) -> PathBuf {
        let path = project.join(".vibehub").join("tasks").join(task_id);
        fs::create_dir_all(&path).expect("create task");
        path
    }

    fn create_run(project: &Path, task_id: &str, run_id: &str) -> PathBuf {
        let path = project
            .join(".vibehub")
            .join("tasks")
            .join(task_id)
            .join("runs")
            .join(run_id);
        fs::create_dir_all(&path).expect("create run");
        path
    }

    #[test]
    fn writes_and_resolves_current_task_pointer() {
        let project = temp_project();
        create_task(&project, "T-001");

        let written = write_current_task_pointer(&project, "T-001").expect("write pointer");
        let resolved = resolve_current_task(&project).expect("resolve pointer");

        assert_eq!(written.kind, CURRENT_TASK_KIND);
        assert_eq!(resolved.task_id, "T-001");
        assert_eq!(resolved.path, ".vibehub/tasks/T-001");
        assert_eq!(resolved.updated_by, UPDATED_BY);

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn writes_and_resolves_current_run_pointer() {
        let project = temp_project();
        create_task(&project, "T-001");
        create_run(&project, "T-001", "R-001");

        let written = write_current_run_pointer(&project, "T-001", "R-001").expect("write pointer");
        let resolved = resolve_current_run(&project, "T-001").expect("resolve pointer");

        assert_eq!(written.kind, CURRENT_RUN_KIND);
        assert_eq!(resolved.task_id, "T-001");
        assert_eq!(resolved.run_id, "R-001");
        assert_eq!(resolved.path, ".vibehub/tasks/T-001/runs/R-001");
        assert_eq!(resolved.updated_by, UPDATED_BY);

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn missing_task_pointer_returns_clear_error() {
        let project = temp_project();
        let err = resolve_current_task(&project).expect_err("missing pointer should fail");

        assert!(err.to_string().contains("Failed to resolve current task"));
        assert!(format!("{err:#}").contains("Missing current task pointer file"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn invalid_yaml_returns_clear_error() {
        let project = temp_project();
        fs::write(project.join(".vibehub/tasks/current"), "schema_version: [")
            .expect("write invalid yaml");

        let err = resolve_current_task(&project).expect_err("invalid yaml should fail");

        assert!(format!("{err:#}").contains("Invalid YAML in current task pointer"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn wrong_task_pointer_kind_returns_clear_error() {
        let project = temp_project();
        fs::write(
            project.join(".vibehub/tasks/current"),
            r#"schema_version: 1
kind: current_run_pointer
task_id: T-001
path: .vibehub/tasks/T-001
updated_at: "2026-04-29T00:00:00Z"
updated_by: vibehub
"#,
        )
        .expect("write pointer");

        let err = resolve_current_task(&project).expect_err("wrong kind should fail");

        assert!(err.to_string().contains("kind 'current_run_pointer'"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn broken_task_pointer_does_not_auto_repair() {
        let project = temp_project();
        fs::write(
            project.join(".vibehub/tasks/current"),
            r#"schema_version: 1
kind: current_task_pointer
task_id: T-missing
path: .vibehub/tasks/T-missing
updated_at: "2026-04-29T00:00:00Z"
updated_by: vibehub
"#,
        )
        .expect("write pointer");

        let err = resolve_current_task(&project).expect_err("broken pointer should fail");

        assert!(format!("{err:#}").contains("target path does not exist"));
        assert!(!project.join(".vibehub/tasks/T-missing").exists());

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn run_pointer_for_different_task_returns_clear_error() {
        let project = temp_project();
        create_task(&project, "T-001");
        fs::create_dir_all(project.join(".vibehub/tasks/T-001/runs")).expect("create runs dir");
        fs::write(
            project.join(".vibehub/tasks/T-001/runs/current"),
            r#"schema_version: 1
kind: current_run_pointer
task_id: T-other
run_id: R-001
path: .vibehub/tasks/T-other/runs/R-001
updated_at: "2026-04-29T00:00:00Z"
updated_by: vibehub
"#,
        )
        .expect("write pointer");

        let err = resolve_current_run(&project, "T-001").expect_err("mismatched task should fail");

        assert!(err
            .to_string()
            .contains("does not match requested task_id 'T-001'"));

        fs::remove_dir_all(project).expect("cleanup");
    }
}

use crate::vibehub::current;
use crate::vibehub::util::{
    canonical_initialized_project_root, normalize_path, relative_to_project,
};
use anyhow::{Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use serde_yaml::{Mapping, Value};
use std::fs;
use std::path::Path;

const RESEARCH_CURRENT_DIR: &str = ".vibehub/research/current";
const RESEARCH_ARCHIVE_DIR: &str = ".vibehub/research/archive";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResearchPackBuildResult {
    pub research_pack_path: String,
    pub source_log_path: String,
    pub findings_path: String,
    pub status: String,
    pub files_created: Vec<String>,
    pub files_skipped: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResearchPackArchiveResult {
    pub archived_to: String,
    pub archive_path: String,
    pub status: String,
}

pub fn build_research_pack(
    project_root: impl AsRef<Path>,
    title: Option<String>,
) -> Result<ResearchPackBuildResult> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let task_pointer = current::resolve_current_task(&project_root)?;
    let current_dir = project_root.join(RESEARCH_CURRENT_DIR);
    fs::create_dir_all(&current_dir)
        .with_context(|| format!("Failed to create {}", current_dir.display()))?;

    let research_pack_path = current_dir.join("research-pack.md");
    let source_log_path = current_dir.join("source-log.yaml");
    let findings_path = current_dir.join("findings.yaml");

    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let task_title = title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("Untitled research");

    let mut files_created = Vec::new();
    let mut files_skipped = Vec::new();

    let rp_rel = normalize_path(&relative_to_project(&project_root, &research_pack_path)?);
    let sl_rel = normalize_path(&relative_to_project(&project_root, &source_log_path)?);
    let fn_rel = normalize_path(&relative_to_project(&project_root, &findings_path)?);

    if research_pack_path.is_file() {
        files_skipped.push(rp_rel.clone());
    } else {
        fs::write(
            &research_pack_path,
            research_pack_template(&task_pointer.task_id, task_title, &now),
        )
        .with_context(|| format!("Failed to write {}", research_pack_path.display()))?;
        files_created.push(rp_rel.clone());
    }

    if source_log_path.is_file() {
        files_skipped.push(sl_rel.clone());
    } else {
        fs::write(
            &source_log_path,
            source_log_template(&task_pointer.task_id, &now),
        )
        .with_context(|| format!("Failed to write {}", source_log_path.display()))?;
        files_created.push(sl_rel.clone());
    }

    if findings_path.is_file() {
        files_skipped.push(fn_rel.clone());
    } else {
        fs::write(
            &findings_path,
            findings_template(&task_pointer.task_id, &now),
        )
        .with_context(|| format!("Failed to write {}", findings_path.display()))?;
        files_created.push(fn_rel.clone());
    }

    update_research_status_in_state(&project_root, "active")?;

    Ok(ResearchPackBuildResult {
        research_pack_path: rp_rel,
        source_log_path: sl_rel,
        findings_path: fn_rel,
        status: "active".to_string(),
        files_created,
        files_skipped,
    })
}

pub fn archive_current_research(
    project_root: impl AsRef<Path>,
) -> Result<Option<ResearchPackArchiveResult>> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let current_dir = project_root.join(RESEARCH_CURRENT_DIR);
    let research_pack = current_dir.join("research-pack.md");

    if !research_pack.is_file() {
        return Ok(None);
    }

    let task_pointer = current::resolve_current_task(&project_root)?;
    let archive_dir = project_root
        .join(RESEARCH_ARCHIVE_DIR)
        .join(&task_pointer.task_id);
    fs::create_dir_all(&archive_dir)
        .with_context(|| format!("Failed to create {}", archive_dir.display()))?;

    for filename in &["research-pack.md", "source-log.yaml", "findings.yaml"] {
        let src = current_dir.join(filename);
        let dst = archive_dir.join(filename);
        if src.is_file() {
            fs::rename(&src, &dst).with_context(|| {
                format!("Failed to move {} to {}", src.display(), dst.display())
            })?;
        }
    }

    update_research_status_in_state(&project_root, "completed")?;

    let archive_path = normalize_path(&relative_to_project(&project_root, &archive_dir)?);
    Ok(Some(ResearchPackArchiveResult {
        archived_to: task_pointer.task_id,
        archive_path,
        status: "completed".to_string(),
    }))
}

pub fn read_research_pack_content(
    project_root: impl AsRef<Path>,
) -> Result<Option<ResearchPackContent>> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let research_pack = project_root
        .join(RESEARCH_CURRENT_DIR)
        .join("research-pack.md");
    if !research_pack.is_file() {
        return Ok(None);
    }
    let content = fs::read_to_string(&research_pack)
        .with_context(|| format!("Failed to read {}", research_pack.display()))?;
    let source_log_path = project_root
        .join(RESEARCH_CURRENT_DIR)
        .join("source-log.yaml");
    let findings_path = project_root
        .join(RESEARCH_CURRENT_DIR)
        .join("findings.yaml");
    Ok(Some(ResearchPackContent {
        research_pack_path: normalize_path(&relative_to_project(&project_root, &research_pack)?),
        research_pack_md: content,
        source_log_exists: source_log_path.is_file(),
        findings_exists: findings_path.is_file(),
    }))
}

pub fn check_research_pack_exists(project_root: &Path) -> bool {
    project_root
        .join(RESEARCH_CURRENT_DIR)
        .join("research-pack.md")
        .is_file()
}

pub fn read_research_status(project_root: &Path) -> ResearchStatus {
    let state_path = project_root.join(".vibehub/state.yaml");
    let required = read_yaml_bool_path(&state_path, &["research", "required"]).unwrap_or(false);
    let status = read_yaml_str_path(&state_path, &["research", "status"])
        .unwrap_or_else(|| "unknown".to_string());
    let exists = check_research_pack_exists(project_root);
    ResearchStatus {
        required,
        status,
        research_pack_exists: exists,
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResearchStatus {
    pub required: bool,
    pub status: String,
    pub research_pack_exists: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResearchPackContent {
    pub research_pack_path: String,
    pub research_pack_md: String,
    pub source_log_exists: bool,
    pub findings_exists: bool,
}

fn update_research_status_in_state(project_root: &Path, status: &str) -> Result<()> {
    let state_path = project_root.join(".vibehub/state.yaml");
    let content = fs::read_to_string(&state_path)
        .with_context(|| format!("Failed to read {}", state_path.display()))?;
    let mut state: Value = serde_yaml::from_str(&content)
        .with_context(|| format!("Invalid YAML in {}", state_path.display()))?;
    set_string(&mut state, &["research", "status"], status);
    set_string(
        &mut state,
        &["last_updated"],
        &Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
    );
    let content = serde_yaml::to_string(&state).context("Failed to serialize state.yaml")?;
    fs::write(&state_path, content)
        .with_context(|| format!("Failed to write {}", state_path.display()))
}

fn set_string(value: &mut Value, path: &[&str], next: &str) {
    set_value(value, path, Value::String(next.to_string()));
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

fn read_yaml_str_path(path: &Path, keys: &[&str]) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let value: Value = serde_yaml::from_str(&content).ok()?;
    keys.iter()
        .try_fold(&value, |current, key| current.get(*key))
        .and_then(Value::as_str)
        .map(|s| s.to_string())
}

fn read_yaml_bool_path(path: &Path, keys: &[&str]) -> Option<bool> {
    let content = fs::read_to_string(path).ok()?;
    let value: Value = serde_yaml::from_str(&content).ok()?;
    keys.iter()
        .try_fold(&value, |current, key| current.get(*key))
        .and_then(Value::as_bool)
}

fn research_pack_template(task_id: &str, title: &str, generated_at: &str) -> String {
    let title_escaped = title.replace('"', "\\\"");
    format!(
        r#"# Research Pack

Task: {}
Title: "{}"
Generated: {}
Status: active

## Research Questions

<!-- Add research questions here. Each question should be specific and actionable. -->

## Sources

<!-- Reference source-log.yaml entries here. List key sources consulted. -->

## Findings

<!-- Reference findings.yaml entries here. Summarize evidence-backed findings. -->

## Recommendation

<!-- Based on research: proceed / more research needed / defer / block -->
- **Recommendation**: [to be filled]
- **Confidence**: [low / medium / high]
- **Rationale**: [brief reasoning]

---
*This research pack was generated by VibeHub. Fill in sections above with evidence-backed research.*
"#,
        task_id, title_escaped, generated_at
    )
}

fn source_log_template(task_id: &str, generated_at: &str) -> String {
    format!(
        r#"schema_version: 1
kind: research_source_log
research_id: "{}"
generated_at: "{}"
sources: []
# Add sources in the format:
# - kind: url | file | agent_observed | model_inferred
#   ref: "the reference or URL"
#   description: "what was consulted"
#   relevance: high | medium | low
#   notes: "additional context"
"#,
        task_id, generated_at
    )
}

fn findings_template(task_id: &str, generated_at: &str) -> String {
    format!(
        r#"schema_version: 1
kind: research_findings
research_id: "{}"
generated_at: "{}"
findings: []
# Add findings in the format:
# - id: F001
#   question: "the research question"
#   answer: "evidence-backed answer"
#   evidence: hard_observed | agent_reported | inferred | user_confirmed
#   confidence: low | medium | high
#   sources: [0, 1]  # indices into source-log.yaml sources list
#   recommendation: proceed | more_research | defer | block
"#,
        task_id, generated_at
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vibehub::current::{write_current_run_pointer, write_current_task_pointer};
    use crate::vibehub::init;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn temp_project() -> PathBuf {
        let path = std::env::temp_dir().join(format!("vibehub-research-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).expect("create temp project");
        path
    }

    fn init_minimal(project: &Path) {
        init::init_project(project).expect("init vibehub");
        fs::create_dir_all(project.join(".vibehub/tasks/T-001/runs/R-001/sessions"))
            .expect("create run dirs");
        fs::create_dir_all(project.join(".vibehub/tasks/T-001/runs/R-001/outputs"))
            .expect("create outputs dir");
        fs::write(
            project.join(".vibehub/tasks/T-001/task.yaml"),
            "task_id: T-001\nmode: evidence_drive\nphase: research\nphase_status: active\n",
        )
        .expect("write task");
        fs::write(
            project.join(".vibehub/tasks/T-001/runs/R-001/run.yaml"),
            "task_id: T-001\nrun_id: R-001\nmode: evidence_drive\n",
        )
        .expect("write run");
        fs::create_dir_all(project.join(RESEARCH_CURRENT_DIR)).expect("create research dir");
        fs::create_dir_all(project.join(RESEARCH_ARCHIVE_DIR)).expect("create archive dir");
        write_current_task_pointer(project, "T-001").expect("task pointer");
        write_current_run_pointer(project, "T-001", "R-001").expect("run pointer");
        fs::write(
            project.join(".vibehub/state.yaml"),
            r#"current:
  mode: evidence_drive
  task_id: T-001
  run_id: R-001
  phase: research
  phase_status: active
research:
  required: true
  status: required
"#,
        )
        .expect("write state");
    }

    fn set_task_pointer(project: &Path, task_id: &str, run_id: &str) {
        write_current_task_pointer(project, task_id).expect("task pointer");
        write_current_run_pointer(project, task_id, run_id).expect("run pointer");
        let state = format!(
            r#"current:
  mode: evidence_drive
  task_id: {}
  run_id: {}
  phase: research
  phase_status: active
research:
  required: true
  status: required
"#,
            task_id, run_id
        );
        fs::write(project.join(".vibehub/state.yaml"), state).expect("write state");
        fs::create_dir_all(
            project
                .join(".vibehub/tasks")
                .join(task_id)
                .join("runs")
                .join(run_id),
        )
        .expect("create run dirs");
    }

    #[test]
    fn build_research_pack_creates_all_three_files() {
        let project = temp_project();
        init_minimal(&project);

        let result = build_research_pack(&project, Some("Test research".to_string()))
            .expect("build research pack");

        assert_eq!(result.status, "active");
        assert_eq!(result.files_created.len(), 3);
        assert_eq!(result.files_skipped.len(), 0);

        let rp = project.join(&result.research_pack_path);
        let sl = project.join(&result.source_log_path);
        let fn_ = project.join(&result.findings_path);

        assert!(rp.is_file(), "research-pack.md should exist");
        assert!(sl.is_file(), "source-log.yaml should exist");
        assert!(fn_.is_file(), "findings.yaml should exist");

        let rp_content = fs::read_to_string(&rp).expect("read research-pack");
        assert!(rp_content.contains("# Research Pack"));
        assert!(rp_content.contains("Test research"));
        assert!(rp_content.contains("Status: active"));

        let sl_content = fs::read_to_string(&sl).expect("read source-log");
        assert!(sl_content.contains("kind: research_source_log"));

        let fn_content = fs::read_to_string(&fn_).expect("read findings");
        assert!(fn_content.contains("kind: research_findings"));

        let state = fs::read_to_string(project.join(".vibehub/state.yaml")).expect("read state");
        assert!(state.contains("status: active"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn build_research_pack_skips_existing_files() {
        let project = temp_project();
        init_minimal(&project);

        let first =
            build_research_pack(&project, Some("First build".to_string())).expect("first build");
        assert_eq!(first.files_created.len(), 3);

        let second =
            build_research_pack(&project, Some("Second build".to_string())).expect("second build");
        assert_eq!(second.files_created.len(), 0);
        assert_eq!(second.files_skipped.len(), 3);

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn archive_current_research_moves_files_to_archive() {
        let project = temp_project();
        init_minimal(&project);

        build_research_pack(&project, Some("To archive".to_string())).expect("build");

        assert!(check_research_pack_exists(&project));

        let result = archive_current_research(&project)
            .expect("archive")
            .expect("should archive");

        assert_eq!(result.archived_to, "T-001");
        assert_eq!(result.status, "completed");
        assert!(result.archive_path.contains("archive/T-001"));

        assert!(!check_research_pack_exists(&project));

        let archived_rp = project.join(&result.archive_path).join("research-pack.md");
        assert!(archived_rp.is_file(), "archived research-pack should exist");

        let state = fs::read_to_string(project.join(".vibehub/state.yaml")).expect("read state");
        assert!(state.contains("status: completed"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn archive_returns_none_when_no_research_pack() {
        let project = temp_project();
        init_minimal(&project);

        let result = archive_current_research(&project).expect("archive");
        assert!(result.is_none());

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn read_research_pack_content_returns_content() {
        let project = temp_project();
        init_minimal(&project);

        build_research_pack(&project, Some("Read test".to_string())).expect("build");

        let content = read_research_pack_content(&project)
            .expect("read content")
            .expect("should have content");

        assert!(content.research_pack_md.contains("# Research Pack"));
        assert!(content.research_pack_md.contains("Read test"));
        assert!(content.source_log_exists);
        assert!(content.findings_exists);

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn read_research_pack_content_returns_none_when_no_file() {
        let project = temp_project();
        init_minimal(&project);

        let content = read_research_pack_content(&project).expect("read content");
        assert!(content.is_none());

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn read_research_status_reflects_all_states() {
        let project = temp_project();
        init_minimal(&project);

        let status = read_research_status(&project);
        assert!(status.required);
        assert_eq!(status.status, "required");
        assert!(!status.research_pack_exists);

        build_research_pack(&project, Some("Status test".to_string())).expect("build");

        let status = read_research_status(&project);
        assert!(status.required);
        assert_eq!(status.status, "active");
        assert!(status.research_pack_exists);

        archive_current_research(&project).expect("archive");

        let status = read_research_status(&project);
        assert!(status.required);
        assert_eq!(status.status, "completed");
        assert!(!status.research_pack_exists);

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn start_new_task_archives_old_current_research() {
        let project = temp_project();
        init_minimal(&project);

        build_research_pack(&project, Some("Old task research".to_string())).expect("build");
        assert!(check_research_pack_exists(&project));

        fs::create_dir_all(project.join(".vibehub/tasks/T-002/runs/R-002/sessions"))
            .expect("create new task dirs");
        fs::write(
            project.join(".vibehub/tasks/T-002/task.yaml"),
            "task_id: T-002\nmode: evidence_drive\nphase: research\nphase_status: active\n",
        )
        .expect("write task2");
        fs::write(
            project.join(".vibehub/tasks/T-002/runs/R-002/run.yaml"),
            "task_id: T-002\nrun_id: R-002\nmode: evidence_drive\n",
        )
        .expect("write run2");
        set_task_pointer(&project, "T-002", "R-002");

        let archived = archive_current_research(&project)
            .expect("archive")
            .expect("should archive");

        assert_eq!(archived.archived_to, "T-002");
        assert!(archived.archive_path.contains("archive/T-002"));
        assert!(!check_research_pack_exists(&project));

        let archived_rp = project
            .join(&archived.archive_path)
            .join("research-pack.md");
        assert!(archived_rp.is_file());

        let state = fs::read_to_string(project.join(".vibehub/state.yaml")).expect("read state");
        assert!(state.contains("status: completed"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn research_pack_template_has_all_sections() {
        let tmpl = research_pack_template("T-001", "Test task", "2026-01-01T00:00:00Z");
        assert!(tmpl.contains("## Research Questions"));
        assert!(tmpl.contains("## Sources"));
        assert!(tmpl.contains("## Findings"));
        assert!(tmpl.contains("## Recommendation"));
        assert!(tmpl.contains("T-001"));
        assert!(tmpl.contains("Test task"));
        assert!(tmpl.contains("Status: active"));
    }

    #[test]
    fn source_log_template_has_correct_structure() {
        let tmpl = source_log_template("T-001", "2026-01-01T00:00:00Z");
        assert!(tmpl.contains("schema_version: 1"));
        assert!(tmpl.contains("kind: research_source_log"));
        assert!(tmpl.contains("sources: []"));
        assert!(tmpl.contains("T-001"));
    }

    #[test]
    fn findings_template_has_correct_structure() {
        let tmpl = findings_template("T-001", "2026-01-01T00:00:00Z");
        assert!(tmpl.contains("schema_version: 1"));
        assert!(tmpl.contains("kind: research_findings"));
        assert!(tmpl.contains("findings: []"));
        assert!(tmpl.contains("T-001"));
        assert!(tmpl.contains("evidence: hard_observed"));
    }
}

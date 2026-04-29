use anyhow::{anyhow, Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

const SCHEMA_VERSION: u32 = 1;
const MAX_FILE_SIZE_BYTES: u64 = 256 * 1024;
const MAX_ESTIMATED_TOKENS: usize = 12_000;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ContextPackBuildResult {
    pub task_id: String,
    pub run_id: String,
    pub phase: String,
    pub pack_path: String,
    pub manifest_path: String,
    pub included_count: usize,
    pub missing_count: usize,
    pub excluded_count: usize,
    pub estimated_tokens: usize,
}

#[derive(Debug, Deserialize)]
struct ContextSpec {
    #[serde(default)]
    phase: Option<String>,
    #[serde(default)]
    task_id: Option<String>,
    #[serde(default)]
    entries: Vec<ContextEntry>,
    #[serde(default)]
    known_missing_context: Vec<String>,
    #[serde(default)]
    stop_condition: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ContextEntry {
    path: String,
    #[serde(rename = "type")]
    #[serde(default = "default_entry_type")]
    entry_type: String,
    reason: String,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    confidence: Option<String>,
    #[serde(default)]
    include_when: Option<String>,
}

#[derive(Debug, Serialize)]
struct ContextManifest {
    id: String,
    schema_version: u32,
    phase: String,
    task_id: String,
    run_id: String,
    generated_at: String,
    source_commit: Option<String>,
    budget: BudgetManifest,
    included: Vec<IncludedManifestEntry>,
    missing: Vec<MissingManifestEntry>,
    excluded: Vec<ExcludedManifestEntry>,
    quality: QualityManifest,
    observation: ObservationManifest,
}

#[derive(Debug, Serialize)]
struct BudgetManifest {
    max_file_size_bytes: u64,
    max_tokens: usize,
    estimated_tokens: usize,
}

#[derive(Debug, Serialize)]
struct IncludedManifestEntry {
    path: String,
    reason: String,
    required: bool,
    confidence: String,
    bytes: u64,
    estimated_tokens: usize,
}

#[derive(Debug, Serialize)]
struct MissingManifestEntry {
    path: String,
    required: bool,
    reason: String,
}

#[derive(Debug, Serialize)]
struct ExcludedManifestEntry {
    path: String,
    required: bool,
    reason: String,
    policy: String,
}

#[derive(Debug, Serialize)]
struct QualityManifest {
    has_goal: bool,
    has_plan: bool,
    has_relevant_code: bool,
    has_test_hint: bool,
    stale_sources: Vec<String>,
    flags: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ObservationManifest {
    generated_by: String,
    evidence_grade: String,
}

struct IncludedFile {
    path: String,
    reason: String,
    required: bool,
    confidence: String,
    bytes: u64,
    estimated_tokens: usize,
    content: String,
}

pub fn build_context_pack(
    project_root: impl AsRef<Path>,
    task_id: impl AsRef<str>,
    run_id: impl AsRef<str>,
    phase: impl AsRef<str>,
) -> Result<ContextPackBuildResult> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    let task_id = validate_id("task_id", task_id.as_ref())?;
    let run_id = validate_id("run_id", run_id.as_ref())?;
    let phase = validate_id("phase", phase.as_ref())?;

    let input_path = project_root
        .join(".vibehub")
        .join("tasks")
        .join(task_id)
        .join("context")
        .join(format!("{phase}.yaml"));
    let input = fs::read_to_string(&input_path)
        .with_context(|| format!("Failed to read context spec {}", input_path.display()))?;
    let spec: ContextSpec = serde_yaml::from_str(&input)
        .with_context(|| format!("Invalid YAML in context spec {}", input_path.display()))?;

    if let Some(spec_task_id) = spec.task_id.as_deref() {
        if spec_task_id != task_id {
            return Err(anyhow!(
                "Context spec task_id '{}' does not match requested task_id '{}'",
                spec_task_id,
                task_id
            ));
        }
    }
    if let Some(spec_phase) = spec.phase.as_deref() {
        if spec_phase != phase {
            return Err(anyhow!(
                "Context spec phase '{}' does not match requested phase '{}'",
                spec_phase,
                phase
            ));
        }
    }

    let run_dir = project_root
        .join(".vibehub")
        .join("tasks")
        .join(task_id)
        .join("runs")
        .join(run_id);
    if !run_dir.is_dir() {
        return Err(anyhow!(
            "Run directory does not exist: {}",
            run_dir.display()
        ));
    }

    let mut included_files = Vec::new();
    let mut missing = Vec::new();
    let mut excluded = Vec::new();
    let mut fatal_errors = Vec::new();

    for entry in &spec.entries {
        if entry.entry_type != "file" {
            let reason = format!("unsupported entry type '{}'", entry.entry_type);
            excluded.push(excluded_entry(entry, reason.clone(), "unsupported_type"));
            if entry.required {
                fatal_errors.push(format!(
                    "Required context entry is unsupported: {}",
                    entry.path
                ));
            }
            continue;
        }

        if let Some(condition) = entry.include_when.as_deref() {
            let reason = format!("include_when not satisfied in P0: {condition}");
            excluded.push(excluded_entry(entry, reason, "include_when"));
            continue;
        }

        if is_secret_like_path(&entry.path) {
            excluded.push(excluded_entry(
                entry,
                "secret-like path denied".to_string(),
                "deny_secret_path",
            ));
            if entry.required {
                fatal_errors.push(format!("Required context path is denied: {}", entry.path));
            }
            continue;
        }

        let Some(candidate) = repo_relative_candidate(&project_root, &entry.path)? else {
            excluded.push(excluded_entry(
                entry,
                "path escapes repository root".to_string(),
                "path_outside_repo",
            ));
            if entry.required {
                fatal_errors.push(format!(
                    "Required context path escapes repo: {}",
                    entry.path
                ));
            }
            continue;
        };

        if !candidate.exists() {
            missing.push(MissingManifestEntry {
                path: normalize_entry_path(&entry.path),
                required: entry.required,
                reason: entry.reason.clone(),
            });
            if entry.required {
                fatal_errors.push(format!("Required context file is missing: {}", entry.path));
            }
            continue;
        }

        let canonical = fs::canonicalize(&candidate)
            .with_context(|| format!("Failed to canonicalize {}", candidate.display()))?;
        if !canonical.starts_with(&project_root) {
            excluded.push(excluded_entry(
                entry,
                "path resolves outside repository root".to_string(),
                "path_outside_repo",
            ));
            if entry.required {
                fatal_errors.push(format!(
                    "Required context path escapes repo: {}",
                    entry.path
                ));
            }
            continue;
        }
        if !canonical.is_file() {
            excluded.push(excluded_entry(
                entry,
                "only file entries are supported".to_string(),
                "not_a_file",
            ));
            if entry.required {
                fatal_errors.push(format!(
                    "Required context path is not a file: {}",
                    entry.path
                ));
            }
            continue;
        }

        let metadata = fs::metadata(&canonical)
            .with_context(|| format!("Failed to inspect {}", canonical.display()))?;
        if metadata.len() > MAX_FILE_SIZE_BYTES {
            excluded.push(excluded_entry(
                entry,
                format!("file exceeds {} byte limit", MAX_FILE_SIZE_BYTES),
                "file_too_large",
            ));
            if entry.required {
                fatal_errors.push(format!(
                    "Required context file is too large: {}",
                    entry.path
                ));
            }
            continue;
        }

        let content = fs::read_to_string(&canonical)
            .with_context(|| format!("Failed to read context file {}", canonical.display()))?;
        let estimated_tokens = estimate_tokens(&content);
        included_files.push(IncludedFile {
            path: normalize_entry_path(&entry.path),
            reason: entry.reason.clone(),
            required: entry.required,
            confidence: entry
                .confidence
                .clone()
                .unwrap_or_else(|| default_confidence(entry.required).to_string()),
            bytes: metadata.len(),
            estimated_tokens,
            content,
        });
    }

    if !fatal_errors.is_empty() {
        return Err(anyhow!(
            "Context pack build failed: {}",
            fatal_errors.join("; ")
        ));
    }

    let generated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let source_commit = source_commit(&project_root);
    let estimated_tokens = included_files
        .iter()
        .map(|entry| entry.estimated_tokens)
        .sum::<usize>();
    let quality = build_quality(
        &spec,
        &included_files,
        &missing,
        &excluded,
        estimated_tokens,
    );

    let manifest = ContextManifest {
        id: format!("{phase}-context-{run_id}-v1"),
        schema_version: SCHEMA_VERSION,
        phase: phase.to_string(),
        task_id: task_id.to_string(),
        run_id: run_id.to_string(),
        generated_at: generated_at.clone(),
        source_commit: source_commit.clone(),
        budget: BudgetManifest {
            max_file_size_bytes: MAX_FILE_SIZE_BYTES,
            max_tokens: MAX_ESTIMATED_TOKENS,
            estimated_tokens,
        },
        included: included_files
            .iter()
            .map(|entry| IncludedManifestEntry {
                path: entry.path.clone(),
                reason: entry.reason.clone(),
                required: entry.required,
                confidence: entry.confidence.clone(),
                bytes: entry.bytes,
                estimated_tokens: entry.estimated_tokens,
            })
            .collect(),
        missing,
        excluded,
        quality,
        observation: ObservationManifest {
            generated_by: "vibehub_backend".to_string(),
            evidence_grade: "hard_observed".to_string(),
        },
    };

    let pack_dir = run_dir.join("context-packs");
    fs::create_dir_all(&pack_dir)
        .with_context(|| format!("Failed to create {}", pack_dir.display()))?;
    let pack_path = pack_dir.join(format!("{phase}.md"));
    let manifest_path = pack_dir.join(format!("{phase}.manifest.yaml"));
    let markdown = build_markdown(
        task_id,
        run_id,
        phase,
        &generated_at,
        source_commit.as_deref(),
        &included_files,
        &spec.known_missing_context,
        spec.stop_condition.as_deref(),
    );
    let manifest_yaml =
        serde_yaml::to_string(&manifest).context("Failed to serialize context manifest")?;

    fs::write(&pack_path, markdown)
        .with_context(|| format!("Failed to write {}", pack_path.display()))?;
    fs::write(&manifest_path, manifest_yaml)
        .with_context(|| format!("Failed to write {}", manifest_path.display()))?;

    Ok(ContextPackBuildResult {
        task_id: task_id.to_string(),
        run_id: run_id.to_string(),
        phase: phase.to_string(),
        pack_path: normalize_path(&relative_to_project(&project_root, &pack_path)?),
        manifest_path: normalize_path(&relative_to_project(&project_root, &manifest_path)?),
        included_count: manifest.included.len(),
        missing_count: manifest.missing.len(),
        excluded_count: manifest.excluded.len(),
        estimated_tokens,
    })
}

fn build_markdown(
    task_id: &str,
    run_id: &str,
    phase: &str,
    generated_at: &str,
    source_commit: Option<&str>,
    included_files: &[IncludedFile],
    known_missing_context: &[String],
    stop_condition: Option<&str>,
) -> String {
    let mut output = String::new();
    output.push_str(&format!("# Context Pack: {}\n\n", title_case(phase)));
    output.push_str(&format!("Task: {task_id}  \n"));
    output.push_str(&format!("Run: {run_id}  \n"));
    output.push_str(&format!("Phase: {}  \n", title_case(phase)));
    output.push_str(&format!("Generated at: {generated_at}  \n"));
    output.push_str(&format!(
        "Source commit: {}\n\n",
        source_commit.unwrap_or("unavailable")
    ));
    output.push_str("## Instructions\n\n");
    output.push_str("Use this context only for the current phase.\n");
    output.push_str("Do not mark state.yaml completed.\n");
    output.push_str("Report files read, commands run, decisions made, and unresolved risks.\n\n");

    for file in included_files {
        output.push_str(&format!("## File: {}\n\n", file.path));
        output.push_str(&format!("Reason: {}\n\n", file.reason));
        output.push_str("```text\n");
        output.push_str(&file.content);
        if !file.content.ends_with('\n') {
            output.push('\n');
        }
        output.push_str("```\n\n");
    }

    output.push_str("## Known Missing Context\n\n");
    if known_missing_context.is_empty() {
        output.push_str("- None declared.\n\n");
    } else {
        for item in known_missing_context {
            output.push_str(&format!("- {item}\n"));
        }
        output.push('\n');
    }

    output.push_str("## Stop Condition\n\n");
    output.push_str(
        stop_condition.unwrap_or("Write output.md and return to VibeHub for validation."),
    );
    output.push('\n');

    output
}

fn build_quality(
    spec: &ContextSpec,
    included: &[IncludedFile],
    missing: &[MissingManifestEntry],
    excluded: &[ExcludedManifestEntry],
    estimated_tokens: usize,
) -> QualityManifest {
    let combined = included
        .iter()
        .map(|entry| format!("{} {}", entry.path, entry.reason))
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();
    let mut flags = Vec::new();

    if !missing.is_empty() {
        flags.push("missing_optional_context".to_string());
    }
    if !excluded.is_empty() {
        flags.push("excluded_context_entries".to_string());
    }
    if estimated_tokens > MAX_ESTIMATED_TOKENS {
        flags.push("token_budget_exceeded".to_string());
    }
    if spec.entries.is_empty() {
        flags.push("empty_context_spec".to_string());
    }

    QualityManifest {
        has_goal: contains_any(&combined, &["goal", "acceptance", "align", "task"]),
        has_plan: contains_any(&combined, &["plan", "implementation"]),
        has_relevant_code: included
            .iter()
            .any(|entry| !entry.path.starts_with(".vibehub/")),
        has_test_hint: contains_any(&combined, &["test", "validation"]),
        stale_sources: Vec::new(),
        flags,
    }
}

fn source_commit(project_root: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(project_root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if commit.is_empty() {
        None
    } else {
        Some(commit)
    }
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

fn repo_relative_candidate(project_root: &Path, raw_path: &str) -> Result<Option<PathBuf>> {
    let raw = Path::new(raw_path);
    if raw.is_absolute() {
        return Ok(None);
    }

    let mut relative = PathBuf::new();
    for component in raw.components() {
        match component {
            Component::Normal(part) => relative.push(part),
            Component::CurDir => {}
            Component::ParentDir => return Ok(None),
            Component::RootDir | Component::Prefix(_) => return Ok(None),
        }
    }

    Ok(Some(project_root.join(relative)))
}

fn is_secret_like_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/").to_lowercase();
    normalized.split('/').any(|part| {
        part.starts_with(".env") || part == ".git" || part == "secrets" || part == "credentials"
    })
}

fn excluded_entry(entry: &ContextEntry, reason: String, policy: &str) -> ExcludedManifestEntry {
    ExcludedManifestEntry {
        path: normalize_entry_path(&entry.path),
        required: entry.required,
        reason,
        policy: policy.to_string(),
    }
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

fn normalize_entry_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn estimate_tokens(content: &str) -> usize {
    content.chars().count().div_ceil(4).max(1)
}

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn default_confidence(required: bool) -> &'static str {
    if required {
        "high"
    } else {
        "medium"
    }
}

fn default_entry_type() -> String {
    "file".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_project() -> PathBuf {
        let path = std::env::temp_dir().join(format!("vibehub-context-test-{}", Uuid::new_v4()));
        fs::create_dir_all(path.join(".vibehub/tasks/T-001/context")).expect("create context dir");
        fs::create_dir_all(path.join(".vibehub/tasks/T-001/runs/R-001")).expect("create run dir");
        path
    }

    fn write_spec(project: &Path, content: &str) {
        fs::write(
            project.join(".vibehub/tasks/T-001/context/implement.yaml"),
            content,
        )
        .expect("write spec");
    }

    #[test]
    fn generates_markdown_and_manifest_from_context_yaml() {
        let project = temp_project();
        fs::write(project.join("README.md"), "# Goal\nBuild it.\n").expect("write readme");
        fs::write(project.join("src.txt"), "fn main() {}\n").expect("write source");
        write_spec(
            &project,
            r#"phase: implement
task_id: T-001
known_missing_context:
  - No explicit test command found.
stop_condition: Write output.md and return to VibeHub.
entries:
  - path: README.md
    type: file
    reason: task goal and plan
    required: true
  - path: src.txt
    type: file
    reason: relevant code
    required: false
"#,
        );

        let result =
            build_context_pack(&project, "T-001", "R-001", "implement").expect("build context");

        assert_eq!(result.included_count, 2);
        assert_eq!(result.missing_count, 0);
        assert_eq!(
            result.pack_path,
            ".vibehub/tasks/T-001/runs/R-001/context-packs/implement.md"
        );

        let pack = fs::read_to_string(project.join(&result.pack_path)).expect("read pack");
        let manifest =
            fs::read_to_string(project.join(&result.manifest_path)).expect("read manifest");

        assert!(pack.contains("# Context Pack: Implement"));
        assert!(pack.contains("Task: T-001"));
        assert!(pack.contains("## File: README.md"));
        assert!(pack.contains("Reason: task goal and plan"));
        assert!(pack.contains("- No explicit test command found."));
        assert!(manifest.contains("evidence_grade: hard_observed"));
        assert!(manifest.contains("included:"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn required_missing_file_fails_build() {
        let project = temp_project();
        write_spec(
            &project,
            r#"phase: implement
task_id: T-001
entries:
  - path: missing.md
    reason: required plan
    required: true
"#,
        );

        let err = build_context_pack(&project, "T-001", "R-001", "implement")
            .expect_err("required missing should fail");

        assert!(err.to_string().contains("Required context file is missing"));
        assert!(!project
            .join(".vibehub/tasks/T-001/runs/R-001/context-packs/implement.md")
            .exists());

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn optional_missing_file_is_recorded_in_manifest() {
        let project = temp_project();
        fs::write(project.join("README.md"), "# Goal\n").expect("write readme");
        write_spec(
            &project,
            r#"phase: implement
task_id: T-001
entries:
  - path: README.md
    reason: task goal
    required: true
  - path: tests/missing.test.ts
    reason: expected test file
    required: false
"#,
        );

        let result =
            build_context_pack(&project, "T-001", "R-001", "implement").expect("build context");
        let manifest =
            fs::read_to_string(project.join(&result.manifest_path)).expect("read manifest");

        assert_eq!(result.missing_count, 1);
        assert!(manifest.contains("missing:"));
        assert!(manifest.contains("path: tests/missing.test.ts"));
        assert!(manifest.contains("required: false"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn optional_secret_path_is_excluded() {
        let project = temp_project();
        fs::write(project.join("README.md"), "# Goal\n").expect("write readme");
        write_spec(
            &project,
            r#"phase: implement
task_id: T-001
entries:
  - path: README.md
    reason: task goal
    required: true
  - path: .env.local
    reason: should never be included
    required: false
"#,
        );

        let result =
            build_context_pack(&project, "T-001", "R-001", "implement").expect("build context");
        let pack = fs::read_to_string(project.join(&result.pack_path)).expect("read pack");
        let manifest =
            fs::read_to_string(project.join(&result.manifest_path)).expect("read manifest");

        assert_eq!(result.excluded_count, 1);
        assert!(!pack.contains(".env.local"));
        assert!(manifest.contains("policy: deny_secret_path"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn required_secret_path_fails_build() {
        let project = temp_project();
        write_spec(
            &project,
            r#"phase: implement
task_id: T-001
entries:
  - path: secrets/api-key.txt
    reason: unsafe
    required: true
"#,
        );

        let err = build_context_pack(&project, "T-001", "R-001", "implement")
            .expect_err("required secret should fail");

        assert!(err.to_string().contains("Required context path is denied"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn required_path_escape_fails_build() {
        let project = temp_project();
        write_spec(
            &project,
            r#"phase: implement
task_id: T-001
entries:
  - path: ../outside.md
    reason: unsafe
    required: true
"#,
        );

        let err = build_context_pack(&project, "T-001", "R-001", "implement")
            .expect_err("path escape should fail");

        assert!(err
            .to_string()
            .contains("Required context path escapes repo"));

        fs::remove_dir_all(project).expect("cleanup");
    }
}

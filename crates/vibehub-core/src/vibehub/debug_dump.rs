use crate::vibehub::current;
use crate::vibehub::util::{
    canonical_initialized_project_root, normalize_path, relative_to_project,
};
use anyhow::{Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DebugDumpOptions {
    pub include_events: Option<bool>,
    pub include_packs: Option<bool>,
    pub redact_secrets: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DebugDumpManifest {
    pub schema_version: u32,
    pub kind: String,
    pub created_at: String,
    pub source_project: String,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub redaction_enabled: bool,
    pub copied_files: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DebugDumpResult {
    pub dump_path: String,
    pub manifest_path: String,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub files_copied: usize,
    pub redacted_files: usize,
    pub events_copied: usize,
    pub context_packs_copied: usize,
    pub outputs_copied: usize,
    pub handoffs_copied: usize,
    pub sync_reports_copied: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Default)]
struct DumpBuilder {
    dump_root: PathBuf,
    redact_secrets: bool,
    files_copied: usize,
    redacted_files: usize,
    events_copied: usize,
    context_packs_copied: usize,
    outputs_copied: usize,
    handoffs_copied: usize,
    sync_reports_copied: usize,
    copied_files: Vec<String>,
    warnings: Vec<String>,
}

pub fn create_debug_dump(
    project_root: impl AsRef<Path>,
    options: Option<DebugDumpOptions>,
) -> Result<DebugDumpResult> {
    let options = options.unwrap_or_default();
    let include_events = options.include_events.unwrap_or(true);
    let include_packs = options.include_packs.unwrap_or(true);
    let redact_secrets = options.redact_secrets.unwrap_or(true);
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let vibehub_root = project_root.join(".vibehub");
    let dump_root = next_dump_root(&vibehub_root.join("debug-dumps"))?;
    fs::create_dir_all(&dump_root)
        .with_context(|| format!("Failed to create debug dump {}", dump_root.display()))?;

    let mut builder = DumpBuilder {
        dump_root: dump_root.clone(),
        redact_secrets,
        ..Default::default()
    };

    copy_optional_file(
        &mut builder,
        &vibehub_root.join("state.yaml"),
        Path::new("state/state.yaml"),
    )?;
    copy_optional_file(
        &mut builder,
        &vibehub_root.join("policy.yaml"),
        Path::new("project/policy.yaml"),
    )?;
    copy_optional_file(
        &mut builder,
        &vibehub_root.join("skills.registry.yaml"),
        Path::new("project/skills.registry.yaml"),
    )?;
    copy_optional_file(
        &mut builder,
        &vibehub_root.join("adapters/config.yaml"),
        Path::new("project/adapters-config.yaml"),
    )?;
    copy_optional_dir(
        &mut builder,
        &vibehub_root.join("agent-view"),
        Path::new("agent-view"),
    )?;
    copy_optional_dir(
        &mut builder,
        &vibehub_root.join("rules"),
        Path::new("rules"),
    )?;

    let current_task = match current::resolve_current_task(&project_root) {
        Ok(pointer) => Some(pointer),
        Err(error) => {
            builder
                .warnings
                .push(format!("Could not resolve current task pointer: {error}"));
            None
        }
    };
    let current_run = match current_task.as_ref() {
        Some(task) => match current::resolve_current_run(&project_root, &task.task_id) {
            Ok(pointer) => Some(pointer),
            Err(error) => {
                builder
                    .warnings
                    .push(format!("Could not resolve current run pointer: {error}"));
                None
            }
        },
        None => None,
    };

    if let Some(task) = current_task.as_ref() {
        let task_root = vibehub_root.join("tasks").join(&task.task_id);
        copy_optional_file(
            &mut builder,
            &task_root.join("task.yaml"),
            &Path::new("current-task").join("task.yaml"),
        )?;
        copy_run_artifacts(
            &mut builder,
            &task.task_id,
            &task_root,
            include_events,
            include_packs,
        )?;
    }

    let manifest = DebugDumpManifest {
        schema_version: 1,
        kind: "vibehub_debug_dump".to_string(),
        created_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        source_project: normalize_path(&project_root),
        task_id: current_task.as_ref().map(|task| task.task_id.clone()),
        run_id: current_run.as_ref().map(|run| run.run_id.clone()),
        redaction_enabled: redact_secrets,
        copied_files: builder.copied_files.clone(),
        warnings: builder.warnings.clone(),
    };
    let manifest_path = dump_root.join("manifest.json");
    let manifest_content = serde_json::to_string_pretty(&manifest)
        .context("Failed to serialize debug dump manifest")?;
    fs::write(&manifest_path, manifest_content)
        .with_context(|| format!("Failed to write {}", manifest_path.display()))?;

    Ok(DebugDumpResult {
        dump_path: normalize_path(&relative_to_project(&project_root, &dump_root)?),
        manifest_path: normalize_path(&relative_to_project(&project_root, &manifest_path)?),
        task_id: manifest.task_id,
        run_id: manifest.run_id,
        files_copied: builder.files_copied,
        redacted_files: builder.redacted_files,
        events_copied: builder.events_copied,
        context_packs_copied: builder.context_packs_copied,
        outputs_copied: builder.outputs_copied,
        handoffs_copied: builder.handoffs_copied,
        sync_reports_copied: builder.sync_reports_copied,
        warnings: builder.warnings,
    })
}

fn copy_run_artifacts(
    builder: &mut DumpBuilder,
    task_id: &str,
    task_root: &Path,
    include_events: bool,
    include_packs: bool,
) -> Result<()> {
    let runs_root = task_root.join("runs");
    if !runs_root.is_dir() {
        builder
            .warnings
            .push(format!("Current task has no runs directory: {task_id}"));
        return Ok(());
    }

    for entry in read_dir_sorted(&runs_root)? {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let run_id = entry.file_name().to_string_lossy().to_string();
        if run_id == "current" {
            continue;
        }
        let target_base = Path::new("current-task").join("runs").join(&run_id);

        copy_optional_file(
            builder,
            &path.join("run.yaml"),
            &target_base.join("run.yaml"),
        )?;
        if include_events {
            let before = builder.files_copied;
            copy_optional_file(
                builder,
                &path.join("events.jsonl"),
                &target_base.join("events/events.jsonl"),
            )?;
            if builder.files_copied > before {
                builder.events_copied += 1;
            }
        }
        if include_packs {
            let before = builder.files_copied;
            copy_optional_dir(
                builder,
                &path.join("context-packs"),
                &target_base.join("context-packs"),
            )?;
            builder.context_packs_copied += builder.files_copied - before;
        }
        let before = builder.files_copied;
        copy_optional_dir(builder, &path.join("outputs"), &target_base.join("outputs"))?;
        builder.outputs_copied += builder.files_copied - before;

        let before = builder.files_copied;
        copy_optional_dir(
            builder,
            &path.join("handoffs"),
            &target_base.join("handoffs"),
        )?;
        builder.handoffs_copied += builder.files_copied - before;

        let before = builder.files_copied;
        copy_optional_dir(
            builder,
            &path.join("sync"),
            &target_base.join("sync-reports"),
        )?;
        builder.sync_reports_copied += builder.files_copied - before;
    }

    Ok(())
}

fn copy_optional_dir(
    builder: &mut DumpBuilder,
    source: &Path,
    target_relative: &Path,
) -> Result<()> {
    if !source.exists() {
        return Ok(());
    }
    if !source.is_dir() {
        builder.warnings.push(format!(
            "Expected directory but found file: {}",
            source.display()
        ));
        return Ok(());
    }

    for entry in read_dir_sorted(source)? {
        let path = entry.path();
        let child_target = target_relative.join(entry.file_name());
        if path.is_dir() {
            copy_optional_dir(builder, &path, &child_target)?;
        } else if path.is_file() {
            copy_optional_file(builder, &path, &child_target)?;
        }
    }
    Ok(())
}

fn copy_optional_file(
    builder: &mut DumpBuilder,
    source: &Path,
    target_relative: &Path,
) -> Result<()> {
    if !source.exists() {
        return Ok(());
    }
    if !source.is_file() {
        builder.warnings.push(format!(
            "Expected file but found directory: {}",
            source.display()
        ));
        return Ok(());
    }

    let target = builder.dump_root.join(target_relative);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create debug dump dir {}", parent.display()))?;
    }

    let mut redacted = false;
    if builder.redact_secrets && is_text_like(source) {
        let content = fs::read_to_string(source)
            .with_context(|| format!("Failed to read {}", source.display()))?;
        let sanitized = redact_sensitive_text(&content, &mut redacted);
        fs::write(&target, sanitized)
            .with_context(|| format!("Failed to write {}", target.display()))?;
    } else {
        fs::copy(source, &target).with_context(|| {
            format!(
                "Failed to copy {} to {}",
                source.display(),
                target.display()
            )
        })?;
    }

    builder.files_copied += 1;
    if redacted {
        builder.redacted_files += 1;
    }
    builder.copied_files.push(normalize_path(target_relative));
    Ok(())
}

fn read_dir_sorted(path: &Path) -> Result<Vec<fs::DirEntry>> {
    let mut entries = fs::read_dir(path)
        .with_context(|| format!("Failed to read directory {}", path.display()))?
        .collect::<std::io::Result<Vec<_>>>()
        .with_context(|| {
            format!(
                "Failed to collect directory entries from {}",
                path.display()
            )
        })?;
    entries.sort_by_key(|entry| entry.file_name());
    Ok(entries)
}

fn next_dump_root(parent: &Path) -> Result<PathBuf> {
    fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create debug dump root {}", parent.display()))?;
    let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let mut candidate = parent.join(&timestamp);
    let mut suffix = 1usize;
    while candidate.exists() {
        suffix += 1;
        candidate = parent.join(format!("{timestamp}-{suffix}"));
    }
    Ok(candidate)
}

fn is_text_like(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some(
            "json"
                | "jsonl"
                | "md"
                | "txt"
                | "yaml"
                | "yml"
                | "toml"
                | "lock"
                | "rs"
                | "ts"
                | "tsx"
                | "js"
                | "jsx"
        )
    )
}

fn redact_sensitive_text(content: &str, redacted: &mut bool) -> String {
    content
        .lines()
        .map(|line| redact_sensitive_line(line, redacted))
        .collect::<Vec<_>>()
        .join("\n")
}

fn redact_sensitive_line(line: &str, redacted: &mut bool) -> String {
    let lowered = line.to_ascii_lowercase();
    let sensitive = [
        "api_key",
        "apikey",
        "access_key",
        "auth_token",
        "authorization",
        "bearer ",
        "client_secret",
        "credential",
        "password",
        "private_key",
        "secret",
        "session_cookie",
        "token",
    ];
    if !sensitive.iter().any(|needle| lowered.contains(needle)) {
        return line.to_string();
    }

    if let Some(index) = line.find(':') {
        *redacted = true;
        return format!("{}: [REDACTED]", &line[..index]);
    }
    if let Some(index) = line.find('=') {
        *redacted = true;
        return format!("{}=[REDACTED]", &line[..index]);
    }

    *redacted = true;
    "[REDACTED]".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_project() -> PathBuf {
        std::env::temp_dir().join(format!("vibehub-debug-dump-test-{}", Uuid::new_v4()))
    }

    fn write(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
        fs::write(path, content).expect("write file");
    }

    #[test]
    fn creates_redacted_debug_dump_for_current_task() {
        let project = temp_project();
        let run_root = project.join(".vibehub/tasks/T-001/runs/R-001");
        fs::create_dir_all(&run_root).expect("create run");
        write(
            &project.join(".vibehub/tasks/current"),
            r#"schema_version: 1
kind: current_task_pointer
task_id: T-001
path: .vibehub/tasks/T-001
updated_at: "2026-05-29T00:00:00Z"
updated_by: vibehub
"#,
        );
        write(
            &project.join(".vibehub/tasks/T-001/runs/current"),
            r#"schema_version: 1
kind: current_run_pointer
task_id: T-001
run_id: R-001
path: .vibehub/tasks/T-001/runs/R-001
updated_at: "2026-05-29T00:00:00Z"
updated_by: vibehub
"#,
        );
        write(
            &project.join(".vibehub/state.yaml"),
            "token: should-not-leak\n",
        );
        write(
            &project.join(".vibehub/tasks/T-001/task.yaml"),
            "task_id: T-001\n",
        );
        write(&run_root.join("run.yaml"), "run_id: R-001\n");
        write(&run_root.join("events.jsonl"), "{\"event\":\"ok\"}\n");
        write(&run_root.join("context-packs/research.md"), "# pack\n");
        write(&run_root.join("outputs/output.md"), "# output\n");
        write(&run_root.join("handoffs/X4.json"), "{}\n");
        write(&run_root.join("sync/sync.md"), "# sync\n");

        let result = create_debug_dump(&project, None).expect("debug dump");
        assert_eq!(result.task_id.as_deref(), Some("T-001"));
        assert_eq!(result.run_id.as_deref(), Some("R-001"));
        assert_eq!(result.events_copied, 1);
        assert_eq!(result.context_packs_copied, 1);
        assert_eq!(result.outputs_copied, 1);
        assert_eq!(result.handoffs_copied, 1);
        assert_eq!(result.sync_reports_copied, 1);
        assert_eq!(result.redacted_files, 1);

        let dump_root = project.join(&result.dump_path);
        assert!(dump_root.join("manifest.json").is_file());
        assert!(dump_root
            .join("current-task/runs/R-001/events/events.jsonl")
            .is_file());
        let state = fs::read_to_string(dump_root.join("state/state.yaml")).expect("state");
        assert!(state.contains("token: [REDACTED]"));

        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn can_skip_events_and_packs() {
        let project = temp_project();
        let run_root = project.join(".vibehub/tasks/T-001/runs/R-001");
        fs::create_dir_all(&run_root).expect("create run");
        write(&project.join(".vibehub/state.yaml"), "schema_version: 1\n");
        write(
            &project.join(".vibehub/tasks/current"),
            r#"schema_version: 1
kind: current_task_pointer
task_id: T-001
path: .vibehub/tasks/T-001
updated_at: "2026-05-29T00:00:00Z"
updated_by: vibehub
"#,
        );
        write(
            &project.join(".vibehub/tasks/T-001/runs/current"),
            r#"schema_version: 1
kind: current_run_pointer
task_id: T-001
run_id: R-001
path: .vibehub/tasks/T-001/runs/R-001
updated_at: "2026-05-29T00:00:00Z"
updated_by: vibehub
"#,
        );
        write(&run_root.join("events.jsonl"), "{}\n");
        write(&run_root.join("context-packs/align.md"), "# pack\n");

        let result = create_debug_dump(
            &project,
            Some(DebugDumpOptions {
                include_events: Some(false),
                include_packs: Some(false),
                redact_secrets: Some(true),
            }),
        )
        .expect("debug dump");

        let dump_root = project.join(&result.dump_path);
        assert!(!dump_root
            .join("current-task/runs/R-001/events/events.jsonl")
            .exists());
        assert!(!dump_root
            .join("current-task/runs/R-001/context-packs/align.md")
            .exists());
        assert_eq!(result.events_copied, 0);
        assert_eq!(result.context_packs_copied, 0);

        fs::remove_dir_all(project).ok();
    }
}

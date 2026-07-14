//! Read-only projection of an archived VibeHub V2 tree.
//!
//! This module is intentionally independent from the V3 domain. Its public
//! entry point accepts only a project root and always reads the fixed
//! `.vibehub/legacy-v2` location below it.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

const ARCHIVE_RELATIVE_ROOT: &str = ".vibehub/legacy-v2";
const MAX_YAML_BYTES: u64 = 256 * 1024;
const MAX_OUTPUT_BYTES: u64 = 512 * 1024;
const MAX_TASKS: usize = 10_000;
const MAX_SUMMARY_CHARS: usize = 2_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LegacyV2SourceState {
    Absent,
    Available,
    Degraded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct LegacyV2Archive {
    pub source_state: LegacyV2SourceState,
    pub cards: Vec<LegacyV2Card>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct LegacyV2Card {
    pub task_id: String,
    pub title: String,
    pub state: Option<String>,
    pub completed_at: Option<String>,
    pub phase: Option<String>,
    pub final_summary: Option<String>,
    pub file_links: Vec<String>,
    pub warnings: Vec<String>,
}

impl LegacyV2Archive {
    fn absent() -> Self {
        Self {
            source_state: LegacyV2SourceState::Absent,
            cards: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

/// Load the fixed `<project>/.vibehub/legacy-v2` archive without writing to it.
pub fn load_archive(project_root: impl AsRef<Path>) -> Result<LegacyV2Archive> {
    let project_root = fs::canonicalize(project_root.as_ref()).context("invalid project root")?;
    if !project_root.is_dir() {
        anyhow::bail!("project root is not a directory");
    }

    let archive_path = project_root.join(ARCHIVE_RELATIVE_ROOT);
    let archive_metadata = match fs::symlink_metadata(&archive_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(LegacyV2Archive::absent())
        }
        Err(_) => {
            return Ok(degraded_archive("archive root cannot be inspected"));
        }
    };
    if archive_metadata.file_type().is_symlink() {
        return Ok(degraded_archive("archive root symlink was rejected"));
    }
    if !archive_metadata.is_dir() {
        return Ok(degraded_archive("archive root is not a directory"));
    }

    let canonical_archive = match fs::canonicalize(&archive_path) {
        Ok(path) if path.starts_with(&project_root) => path,
        _ => return Ok(degraded_archive("archive root escapes the project")),
    };
    let tasks_path = canonical_archive.join("tasks");
    let tasks_metadata = match fs::symlink_metadata(&tasks_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(LegacyV2Archive {
                source_state: LegacyV2SourceState::Available,
                cards: Vec::new(),
                warnings: Vec::new(),
            });
        }
        Err(_) => return Ok(degraded_archive("tasks directory cannot be inspected")),
    };
    if tasks_metadata.file_type().is_symlink() || !tasks_metadata.is_dir() {
        return Ok(degraded_archive("tasks directory is not a safe directory"));
    }

    let mut warnings = Vec::new();
    let mut entries = match fs::read_dir(&tasks_path) {
        Ok(entries) => entries.filter_map(|entry| entry.ok()).collect::<Vec<_>>(),
        Err(_) => return Ok(degraded_archive("tasks directory cannot be read")),
    };
    entries.sort_by_key(|entry| entry.file_name());
    if entries.len() > MAX_TASKS {
        entries.truncate(MAX_TASKS);
        warnings.push(format!("task scan limited to {MAX_TASKS} entries"));
    }

    let mut cards = Vec::new();
    for entry in entries {
        let task_id = entry.file_name().to_string_lossy().into_owned();
        if !valid_identifier(&task_id, 'T') {
            warnings.push(format!(
                "rejected invalid task entry: {}",
                safe_label(&task_id)
            ));
            continue;
        }
        match load_task(&project_root, &canonical_archive, &entry.path(), &task_id) {
            Ok(card) => cards.push(card),
            Err(reason) => warnings.push(format!("task {task_id} skipped: {reason}")),
        }
    }

    cards.sort_by(|left, right| {
        right
            .completed_at
            .cmp(&left.completed_at)
            .then_with(|| right.task_id.cmp(&left.task_id))
    });
    let degraded = !warnings.is_empty() || cards.iter().any(|card| !card.warnings.is_empty());
    Ok(LegacyV2Archive {
        source_state: if degraded {
            LegacyV2SourceState::Degraded
        } else {
            LegacyV2SourceState::Available
        },
        cards,
        warnings,
    })
}

fn load_task(
    project_root: &Path,
    archive_root: &Path,
    task_path: &Path,
    task_id: &str,
) -> std::result::Result<LegacyV2Card, &'static str> {
    let metadata = fs::symlink_metadata(task_path).map_err(|_| "entry cannot be inspected")?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("entry is not a safe directory");
    }
    let canonical_task = fs::canonicalize(task_path).map_err(|_| "entry cannot be resolved")?;
    if !canonical_task.starts_with(archive_root) {
        return Err("entry escapes the archive");
    }

    let task_yaml_path = canonical_task.join("task.yaml");
    let task_yaml = read_safe_text(
        &task_yaml_path,
        archive_root,
        MAX_YAML_BYTES,
        "task metadata",
    )
    .map_err(|_| "task.yaml is missing, unsafe, unreadable, or too large")?;
    let task: Value = serde_yaml::from_str(&task_yaml).map_err(|_| "task.yaml is corrupt")?;
    if string_field(&task, "task_id").as_deref() != Some(task_id) {
        return Err("task.yaml task_id does not match its directory");
    }

    let mut card = LegacyV2Card {
        task_id: task_id.to_string(),
        title: string_field(&task, "title").unwrap_or_else(|| task_id.to_string()),
        state: string_field(&task, "state").or_else(|| string_field(&task, "phase_status")),
        completed_at: string_field(&task, "completed_at")
            .or_else(|| string_field(&task, "created_at")),
        phase: string_field(&task, "phase"),
        final_summary: string_field(&task, "final_summary")
            .or_else(|| string_field(&task, "summary"))
            .and_then(|text| limited_text(&text)),
        file_links: vec![relative_link(project_root, &task_yaml_path)],
        warnings: Vec::new(),
    };

    if let Some((run_path, run_value)) =
        latest_stable_run(&canonical_task, archive_root, &mut card.warnings)
    {
        if card.phase.is_none() {
            card.phase = string_field(&run_value, "phase");
        }
        if card.completed_at.is_none() {
            card.completed_at = string_field(&run_value, "completed_at")
                .or_else(|| string_field(&run_value, "created_at"));
        }
        let run_yaml_path = run_path.join("run.yaml");
        card.file_links
            .push(relative_link(project_root, &run_yaml_path));

        let output_path = run_path.join("outputs/output.md");
        match read_safe_text(&output_path, archive_root, MAX_OUTPUT_BYTES, "run output") {
            Ok(output) => {
                card.file_links
                    .push(relative_link(project_root, &output_path));
                if card.final_summary.is_none() {
                    card.final_summary = summary_from_output(&output);
                }
            }
            Err(ReadFailure::Missing) => {}
            Err(_) => card
                .warnings
                .push("latest run output is unsafe, unreadable, or too large".to_string()),
        }
    }

    card.file_links.sort();
    card.file_links.dedup();
    Ok(card)
}

fn latest_stable_run(
    task_path: &Path,
    archive_root: &Path,
    warnings: &mut Vec<String>,
) -> Option<(PathBuf, Value)> {
    let runs_path = task_path.join("runs");
    let metadata = match fs::symlink_metadata(&runs_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return None,
        Err(_) => {
            warnings.push("runs directory cannot be inspected".to_string());
            return None;
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        warnings.push("runs directory is not a safe directory".to_string());
        return None;
    }

    let mut entries = match fs::read_dir(&runs_path) {
        Ok(entries) => entries.filter_map(|entry| entry.ok()).collect::<Vec<_>>(),
        Err(_) => {
            warnings.push("runs directory cannot be read".to_string());
            return None;
        }
    };
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.file_name()));
    for entry in entries {
        let run_id = entry.file_name().to_string_lossy().into_owned();
        if !valid_identifier(&run_id, 'R') {
            warnings.push(format!(
                "rejected invalid run entry: {}",
                safe_label(&run_id)
            ));
            continue;
        }
        let Ok(metadata) = fs::symlink_metadata(entry.path()) else {
            warnings.push(format!("run {run_id} cannot be inspected"));
            continue;
        };
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            warnings.push(format!("run {run_id} is not a safe directory"));
            continue;
        }
        let Ok(canonical_run) = fs::canonicalize(entry.path()) else {
            warnings.push(format!("run {run_id} cannot be resolved"));
            continue;
        };
        if !canonical_run.starts_with(task_path) || !canonical_run.starts_with(archive_root) {
            warnings.push(format!("run {run_id} escapes the archive"));
            continue;
        }
        let run_yaml_path = canonical_run.join("run.yaml");
        let Ok(text) = read_safe_text(&run_yaml_path, archive_root, MAX_YAML_BYTES, "run metadata")
        else {
            warnings.push(format!("run {run_id} metadata was skipped"));
            continue;
        };
        match serde_yaml::from_str::<Value>(&text) {
            Ok(value) if string_field(&value, "run_id").as_deref() == Some(run_id.as_str()) => {
                return Some((canonical_run, value));
            }
            _ => warnings.push(format!("run {run_id} metadata is corrupt or mismatched")),
        }
    }
    None
}

#[derive(Debug)]
enum ReadFailure {
    Missing,
    Unsafe,
    TooLarge,
    Unreadable,
}

fn read_safe_text(
    path: &Path,
    archive_root: &Path,
    limit: u64,
    _label: &str,
) -> std::result::Result<String, ReadFailure> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            ReadFailure::Missing
        } else {
            ReadFailure::Unreadable
        }
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(ReadFailure::Unsafe);
    }
    if metadata.len() > limit {
        return Err(ReadFailure::TooLarge);
    }
    let canonical = fs::canonicalize(path).map_err(|_| ReadFailure::Unreadable)?;
    if !canonical.starts_with(archive_root) {
        return Err(ReadFailure::Unsafe);
    }

    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(&canonical)
        .map_err(|_| ReadFailure::Unreadable)?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ReadFailure::Unreadable)?;
    if bytes.len() as u64 > limit {
        return Err(ReadFailure::TooLarge);
    }
    String::from_utf8(bytes).map_err(|_| ReadFailure::Unreadable)
}

fn summary_from_output(output: &str) -> Option<String> {
    let mut in_completed = false;
    for raw_line in output.lines() {
        let line = raw_line.trim();
        if line.starts_with("## ") {
            if in_completed {
                break;
            }
            let heading = line.trim_start_matches('#').trim().to_ascii_lowercase();
            in_completed = heading == "completed" || heading.starts_with("completed /");
            continue;
        }
        if in_completed {
            if let Some(candidate) = line
                .strip_prefix("- ")
                .or_else(|| line.strip_prefix("* "))
                .map(strip_evidence_label)
                .and_then(limited_text)
            {
                return Some(candidate);
            }
        }
    }

    output
        .lines()
        .map(str::trim)
        .filter(|line| {
            !line.is_empty()
                && !line.starts_with('#')
                && !line.starts_with("<!--")
                && !line.starts_with('_')
        })
        .map(strip_evidence_label)
        .find_map(limited_text)
}

fn strip_evidence_label(text: &str) -> &str {
    let trimmed = text.trim();
    for label in [
        "`hard_observed`:",
        "`agent_reported`:",
        "`user_confirmed`:",
        "`inferred`:",
    ] {
        if let Some(rest) = trimmed.strip_prefix(label) {
            return rest.trim();
        }
    }
    trimmed
}

fn limited_text(text: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let mut result: String = text.chars().take(MAX_SUMMARY_CHARS).collect();
    if text.chars().count() > MAX_SUMMARY_CHARS {
        result.push('…');
    }
    Some(result)
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    let value = value.as_mapping()?.get(Value::String(key.to_string()))?;
    match value {
        Value::String(text) => limited_text(text),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(boolean) => Some(boolean.to_string()),
        _ => None,
    }
}

fn valid_identifier(value: &str, prefix: char) -> bool {
    value.len() >= 3
        && value.starts_with(prefix)
        && value.as_bytes().get(1) == Some(&b'-')
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
        && !value.contains("--")
        && Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn relative_link(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .expect("validated archive path must be project-relative")
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn safe_label(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
        .take(80)
        .collect()
}

fn degraded_archive(warning: &str) -> LegacyV2Archive {
    LegacyV2Archive {
        source_state: LegacyV2SourceState::Degraded,
        cards: Vec::new(),
        warnings: vec![warning.to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::time::SystemTime;
    use uuid::Uuid;

    fn temp_project() -> PathBuf {
        let project =
            std::env::temp_dir().join(format!("vibehub-legacy-v2-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&project).expect("create temp project");
        project
    }

    fn write_task(project: &Path, task_id: &str, title: &str, created_at: &str) -> PathBuf {
        let task = project
            .join(ARCHIVE_RELATIVE_ROOT)
            .join("tasks")
            .join(task_id);
        fs::create_dir_all(&task).unwrap();
        fs::write(
            task.join("task.yaml"),
            format!(
                "schema_version: 1\ntask_id: {task_id}\ntitle: {title}\nphase: review\nphase_status: completed\ncreated_at: {created_at}\n"
            ),
        )
        .unwrap();
        task
    }

    fn snapshot_tree(root: &Path) -> BTreeMap<String, (u64, SystemTime)> {
        fn walk(root: &Path, current: &Path, out: &mut BTreeMap<String, (u64, SystemTime)>) {
            for entry in fs::read_dir(current).unwrap() {
                let entry = entry.unwrap();
                let metadata = fs::symlink_metadata(entry.path()).unwrap();
                let relative = entry
                    .path()
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                out.insert(relative, (metadata.len(), metadata.modified().unwrap()));
                if metadata.is_dir() && !metadata.file_type().is_symlink() {
                    walk(root, &entry.path(), out);
                }
            }
        }
        let mut output = BTreeMap::new();
        walk(root, root, &mut output);
        output
    }

    #[test]
    fn absent_root_returns_empty_without_creating_it() {
        let project = temp_project();
        let before = snapshot_tree(&project);
        let archive = load_archive(&project).unwrap();
        assert_eq!(archive.source_state, LegacyV2SourceState::Absent);
        assert!(archive.cards.is_empty());
        assert_eq!(before, snapshot_tree(&project));
        assert!(!project.join(ARCHIVE_RELATIVE_ROOT).exists());
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn loads_valid_corpus_with_stable_order_and_completed_summary() {
        let project = temp_project();
        let older = write_task(
            &project,
            "T-20260101000000-aaaa1111",
            "Older",
            "2026-01-01T00:00:00Z",
        );
        let newer = write_task(
            &project,
            "T-20260201000000-bbbb2222",
            "Newer",
            "2026-02-01T00:00:00Z",
        );
        let run = newer.join("runs/R-20260201000000-cccc3333");
        fs::create_dir_all(run.join("outputs")).unwrap();
        fs::write(
            run.join("run.yaml"),
            "run_id: R-20260201000000-cccc3333\nphase: review\nphase_status: completed\ncreated_at: 2026-02-02T00:00:00Z\n",
        )
        .unwrap();
        fs::write(
            run.join("outputs/output.md"),
            "# Result\n\n## Completed\n- `hard_observed`: Shipped the archive reader.\n",
        )
        .unwrap();
        let before = snapshot_tree(&project);

        let archive = load_archive(&project).unwrap();
        assert_eq!(archive.source_state, LegacyV2SourceState::Available);
        assert_eq!(archive.cards.len(), 2);
        assert_eq!(archive.cards[0].task_id, "T-20260201000000-bbbb2222");
        assert_eq!(
            archive.cards[0].final_summary.as_deref(),
            Some("Shipped the archive reader.")
        );
        assert!(archive.cards[0]
            .file_links
            .iter()
            .all(|path| path.starts_with(ARCHIVE_RELATIVE_ROOT)));
        assert!(archive.cards[0]
            .file_links
            .iter()
            .all(|path| !Path::new(path).is_absolute()));
        assert_eq!(before, snapshot_tree(&project));
        drop(older);
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn corrupt_sibling_degrades_without_hiding_valid_card() {
        let project = temp_project();
        write_task(
            &project,
            "T-20260101000000-aaaa1111",
            "Valid",
            "2026-01-01T00:00:00Z",
        );
        let corrupt = project
            .join(ARCHIVE_RELATIVE_ROOT)
            .join("tasks/T-20260102000000-bbbb2222");
        fs::create_dir_all(&corrupt).unwrap();
        fs::write(corrupt.join("task.yaml"), "not: [valid").unwrap();

        let archive = load_archive(&project).unwrap();
        assert_eq!(archive.source_state, LegacyV2SourceState::Degraded);
        assert_eq!(archive.cards.len(), 1);
        assert!(archive
            .warnings
            .iter()
            .any(|warning| warning.contains("bbbb2222")));
        assert!(archive
            .warnings
            .iter()
            .all(|warning| !warning.contains(project.to_string_lossy().as_ref())));
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn rejects_traversal_shaped_task_id() {
        assert!(!valid_identifier("../T-evil", 'T'));
        assert!(!valid_identifier("T-..", 'T'));
        assert!(!valid_identifier("T-a/b", 'T'));
        assert!(!valid_identifier("/T-evil", 'T'));

        let project = temp_project();
        fs::create_dir_all(project.join(ARCHIVE_RELATIVE_ROOT).join("tasks/bad_task")).unwrap();
        let archive = load_archive(&project).unwrap();
        assert_eq!(archive.source_state, LegacyV2SourceState::Degraded);
        assert!(archive.cards.is_empty());
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn tolerates_missing_optional_fields_and_legacy_field_names() {
        struct Case {
            task_id: &'static str,
            yaml: &'static str,
            expected_title: &'static str,
            expected_state: Option<&'static str>,
            expected_completed_at: Option<&'static str>,
            expected_summary: Option<&'static str>,
        }

        let cases = [
            Case {
                task_id: "T-20260101000000-aaaa1111",
                yaml: "task_id: T-20260101000000-aaaa1111\n",
                expected_title: "T-20260101000000-aaaa1111",
                expected_state: None,
                expected_completed_at: None,
                expected_summary: None,
            },
            Case {
                task_id: "T-20260102000000-bbbb2222",
                yaml: "task_id: T-20260102000000-bbbb2222\ntitle: Legacy\nphase_status: completed\ncreated_at: 2025-12-31T23:59:59Z\nsummary: Legacy summary\n",
                expected_title: "Legacy",
                expected_state: Some("completed"),
                expected_completed_at: Some("2025-12-31T23:59:59Z"),
                expected_summary: Some("Legacy summary"),
            },
        ];

        for case in cases {
            let project = temp_project();
            let task = project
                .join(ARCHIVE_RELATIVE_ROOT)
                .join("tasks")
                .join(case.task_id);
            fs::create_dir_all(&task).unwrap();
            fs::write(task.join("task.yaml"), case.yaml).unwrap();

            let archive = load_archive(&project).unwrap();
            assert_eq!(archive.source_state, LegacyV2SourceState::Available);
            assert_eq!(archive.cards.len(), 1);
            let card = &archive.cards[0];
            assert_eq!(card.title, case.expected_title);
            assert_eq!(card.state.as_deref(), case.expected_state);
            assert_eq!(card.completed_at.as_deref(), case.expected_completed_at);
            assert_eq!(card.final_summary.as_deref(), case.expected_summary);
            assert_eq!(
                card.file_links,
                vec![format!(
                    "{ARCHIVE_RELATIVE_ROOT}/tasks/{}/task.yaml",
                    case.task_id
                )]
            );
            assert!(card.warnings.is_empty());
            fs::remove_dir_all(project).ok();
        }
    }

    #[test]
    fn corrupt_latest_run_falls_back_and_missing_output_adds_no_link() {
        let project = temp_project();
        let task = write_task(
            &project,
            "T-20260101000000-aaaa1111",
            "Run fallback",
            "2026-01-01T00:00:00Z",
        );
        let older_run = task.join("runs/R-20260101000000-bbbb2222");
        fs::create_dir_all(&older_run).unwrap();
        fs::write(
            older_run.join("run.yaml"),
            "run_id: R-20260101000000-bbbb2222\nphase: implement\n",
        )
        .unwrap();
        let corrupt_run = task.join("runs/R-20260102000000-cccc3333");
        fs::create_dir_all(&corrupt_run).unwrap();
        fs::write(corrupt_run.join("run.yaml"), "not: [valid").unwrap();

        let archive = load_archive(&project).unwrap();
        assert_eq!(archive.source_state, LegacyV2SourceState::Degraded);
        assert!(archive.warnings.is_empty());
        assert_eq!(archive.cards.len(), 1);
        let card = &archive.cards[0];
        assert_eq!(card.phase.as_deref(), Some("review"));
        assert!(card.final_summary.is_none());
        assert!(card
            .warnings
            .iter()
            .any(|warning| warning.contains("cccc3333") && warning.contains("corrupt")));
        assert!(card
            .file_links
            .iter()
            .any(|link| link.ends_with("R-20260101000000-bbbb2222/run.yaml")));
        assert!(!card
            .file_links
            .iter()
            .any(|link| link.ends_with("output.md") || link.contains("cccc3333")));
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn rejects_traversal_shaped_run_entry_without_hiding_valid_task() {
        let project = temp_project();
        let task = write_task(
            &project,
            "T-20260101000000-aaaa1111",
            "Valid task",
            "2026-01-01T00:00:00Z",
        );
        fs::create_dir_all(task.join("runs/bad_run")).unwrap();

        let archive = load_archive(&project).unwrap();
        assert_eq!(archive.source_state, LegacyV2SourceState::Degraded);
        assert_eq!(archive.cards.len(), 1);
        assert!(archive.cards[0]
            .warnings
            .iter()
            .any(|warning| warning.contains("invalid run entry")));
        assert!(archive.cards[0]
            .file_links
            .iter()
            .all(|link| link.starts_with(ARCHIVE_RELATIVE_ROOT)
                && !link.contains("..")
                && !Path::new(link).is_absolute()));
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn large_history_has_deterministic_descending_order() {
        let project = temp_project();
        let task_count = 256;
        for index in 0..task_count {
            let task_id = format!("T-20260101000000-{index:08x}");
            write_task(
                &project,
                &task_id,
                "Historical task",
                "2026-01-01T00:00:00Z",
            );
        }

        let first = load_archive(&project).unwrap();
        let second = load_archive(&project).unwrap();
        assert_eq!(first.source_state, LegacyV2SourceState::Available);
        assert_eq!(first.cards.len(), task_count);
        assert_eq!(first.cards, second.cards);
        assert!(first
            .cards
            .windows(2)
            .all(|cards| cards[0].task_id > cards[1].task_id));
        assert!(first.warnings.is_empty());
        fs::remove_dir_all(project).ok();
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_root_task_and_artifact_escapes() {
        use std::os::unix::fs::symlink;

        let outside = temp_project();

        let root_project = temp_project();
        fs::create_dir_all(root_project.join(".vibehub")).unwrap();
        symlink(&outside, root_project.join(ARCHIVE_RELATIVE_ROOT)).unwrap();
        let root_result = load_archive(&root_project).unwrap();
        assert_eq!(root_result.source_state, LegacyV2SourceState::Degraded);
        assert!(root_result.cards.is_empty());

        let project = temp_project();
        let tasks = project.join(ARCHIVE_RELATIVE_ROOT).join("tasks");
        fs::create_dir_all(&tasks).unwrap();
        symlink(&outside, tasks.join("T-20260101000000-aaaa1111")).unwrap();
        let valid = write_task(
            &project,
            "T-20260102000000-bbbb2222",
            "Valid",
            "2026-01-02T00:00:00Z",
        );
        let run = valid.join("runs/R-20260102000000-cccc3333");
        fs::create_dir_all(run.join("outputs")).unwrap();
        fs::write(
            run.join("run.yaml"),
            "run_id: R-20260102000000-cccc3333\nphase: review\n",
        )
        .unwrap();
        let outside_output = outside.join("output.md");
        fs::write(&outside_output, "## Completed\n- Secret outside text\n").unwrap();
        symlink(&outside_output, run.join("outputs/output.md")).unwrap();

        let archive = load_archive(&project).unwrap();
        assert_eq!(archive.source_state, LegacyV2SourceState::Degraded);
        assert_eq!(archive.cards.len(), 1);
        assert!(archive.cards[0].final_summary.is_none());
        assert!(!archive.cards[0]
            .file_links
            .iter()
            .any(|link| link.ends_with("output.md")));
        assert!(archive.cards[0]
            .warnings
            .iter()
            .any(|warning| warning.contains("unsafe")));

        fs::remove_dir_all(root_project).ok();
        fs::remove_dir_all(project).ok();
        fs::remove_dir_all(outside).ok();
    }
}

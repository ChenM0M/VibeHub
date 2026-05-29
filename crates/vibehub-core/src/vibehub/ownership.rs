use crate::vibehub::util::{
    canonical_initialized_project_root, normalize_path, relative_to_project,
};
use crate::vibehub::{current, drift, events};
use anyhow::{anyhow, bail, Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

const OWNERSHIP_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileOwnershipIndex {
    pub schema_version: u32,
    pub file_ownership: Vec<FileOwnershipRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileOwnershipRecord {
    pub file_path: String,
    pub task_id: String,
    pub run_id: Option<String>,
    pub capability: Option<String>,
    pub source: FileOwnershipSource,
    pub confidence: f64,
    pub active_from_event: Option<String>,
    pub active_to_event: Option<String>,
    pub first_seen_at: String,
    pub last_seen_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FileOwnershipSource {
    EventProjection,
    GitHistory,
    UserDeclared,
    AgentInferred,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileOwnershipRecordRequest {
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub capability: Option<String>,
    pub scope_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FileOwnershipRecordResult {
    pub index_path: String,
    pub task_id: String,
    pub run_id: Option<String>,
    pub files_added: Vec<String>,
    pub files_removed: Vec<String>,
    pub event_id: Option<String>,
    pub active_records: Vec<FileOwnershipRecord>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FileOwnershipClassificationReport {
    pub index_path: String,
    pub active_task_id: Option<String>,
    pub files: Vec<FileOwnershipClassification>,
    pub aggregate_action: OwnershipAggregateAction,
    pub prompt: Option<OwnershipPrompt>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FileOwnershipClassification {
    pub file_path: String,
    pub status: FileOwnershipStatus,
    pub owner_task_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FileOwnershipStatus {
    Unowned,
    OwnedBy,
    MultiOwned,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OwnershipAggregateAction {
    NoFiles,
    AssignToActiveTask,
    AssignToOwnerTask,
    AskUserUnowned,
    AskUserPartialOverlap,
    AskUserMultiOwner,
    AskUserAllDrift,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct OwnershipPrompt {
    pub kind: OwnershipPromptKind,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OwnershipPromptKind {
    NoIntersection,
    PartialOverlap,
    MultiOwner,
    AllDrift,
}

pub fn record_file_ownership(
    project_root: impl AsRef<Path>,
    request: FileOwnershipRecordRequest,
) -> Result<FileOwnershipRecordResult> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let current_task = current::resolve_current_task(&project_root).ok();
    let task_id = request
        .task_id
        .or_else(|| current_task.as_ref().map(|task| task.task_id.clone()))
        .ok_or_else(|| anyhow!("Missing task_id and no current VibeHub task is selected"))?;
    let run_id = request.run_id.or_else(|| {
        current_task.as_ref().and_then(|task| {
            current::resolve_current_run(&project_root, &task.task_id)
                .ok()
                .map(|run| run.run_id)
        })
    });
    let scope_files = normalize_scope_files(&request.scope_files)?;
    if scope_files.is_empty() {
        bail!("scope_files must include at least one file path");
    }

    let mut index = read_index(&project_root)?;
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let files_added = scope_files
        .iter()
        .filter(|file_path| {
            !index.file_ownership.iter().any(|record| {
                record.file_path == file_path.as_str()
                    && record.task_id == task_id.as_str()
                    && record.active_to_event.is_none()
            })
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut files_removed = BTreeSet::new();
    for file_path in &scope_files {
        if index.file_ownership.iter().any(|record| {
            record.file_path == *file_path
                && record.active_to_event.is_none()
                && record.task_id != task_id.as_str()
        }) {
            files_removed.insert(file_path.clone());
        }
    }

    let ownership_event = events::VibehubEvent::FileOwnershipUpdated {
        task_id: task_id.clone(),
        files_added: files_added.clone(),
        files_removed: files_removed.iter().cloned().collect(),
    };
    let event = if let Some(run_id) = run_id.as_deref() {
        events::append_structured_run_event(&project_root, &task_id, run_id, ownership_event).ok()
    } else {
        events::append_current_structured_event(&project_root, ownership_event)
            .ok()
            .flatten()
    };
    let event_id = event.map(|event| event.event_id);
    let event_marker = event_id
        .clone()
        .unwrap_or_else(|| format!("manual-record-{}", Utc::now().timestamp_millis()));

    for file_path in &scope_files {
        for record in index.file_ownership.iter_mut().filter(|record| {
            record.file_path == *file_path
                && record.active_to_event.is_none()
                && record.task_id != task_id.as_str()
        }) {
            record.active_to_event = Some(event_marker.clone());
            record.last_seen_at = now.clone();
        }

        if let Some(record) = index.file_ownership.iter_mut().find(|record| {
            record.file_path == *file_path
                && record.task_id == task_id.as_str()
                && record.active_to_event.is_none()
        }) {
            record.run_id = run_id.clone().or_else(|| record.run_id.clone());
            record.capability = request
                .capability
                .clone()
                .or_else(|| record.capability.clone());
            record.source = FileOwnershipSource::UserDeclared;
            record.confidence = 1.0;
            record.last_seen_at = now.clone();
        } else {
            index.file_ownership.push(FileOwnershipRecord {
                file_path: file_path.clone(),
                task_id: task_id.clone(),
                run_id: run_id.clone(),
                capability: request.capability.clone(),
                source: FileOwnershipSource::UserDeclared,
                confidence: 1.0,
                active_from_event: Some(event_marker.clone()),
                active_to_event: None,
                first_seen_at: now.clone(),
                last_seen_at: now.clone(),
            });
        }
    }

    write_index(&project_root, &index)?;
    Ok(FileOwnershipRecordResult {
        index_path: index_path_relative(&project_root)?,
        task_id: task_id.clone(),
        run_id,
        files_added,
        files_removed: files_removed.into_iter().collect(),
        event_id,
        active_records: active_records_for_task(&index, &task_id),
    })
}

pub fn classify_workspace_ownership(
    project_root: impl AsRef<Path>,
    changed_files: Option<Vec<String>>,
) -> Result<FileOwnershipClassificationReport> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let active_task_id = current::resolve_current_task(&project_root)
        .ok()
        .map(|task| task.task_id);
    let files = match changed_files {
        Some(files) => normalize_scope_files(&files)?,
        None => drift::check_workspace_drift(&project_root)?.changed_files,
    };
    classify_files(&project_root, files, active_task_id.as_deref())
}

pub fn classify_files(
    project_root: impl AsRef<Path>,
    changed_files: Vec<String>,
    active_task_id: Option<&str>,
) -> Result<FileOwnershipClassificationReport> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let index = read_index(&project_root)?;
    let normalized_files = normalize_scope_files(&changed_files)?;
    let files: Vec<FileOwnershipClassification> = normalized_files
        .iter()
        .map(|file_path| classify_one(file_path, &index))
        .collect();
    let aggregate_action = aggregate_action(&files, active_task_id);
    let prompt = prompt_for_action(&files, active_task_id, &aggregate_action);

    Ok(FileOwnershipClassificationReport {
        index_path: index_path_relative(&project_root)?,
        active_task_id: active_task_id.map(ToString::to_string),
        files,
        aggregate_action,
        prompt,
    })
}

pub fn active_files_for_task(
    project_root: impl AsRef<Path>,
    task_id: &str,
) -> Result<BTreeSet<String>> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let index = read_index(&project_root)?;
    Ok(active_records_for_task(&index, task_id)
        .into_iter()
        .map(|record| record.file_path)
        .collect())
}

pub fn active_files_by_task(
    project_root: impl AsRef<Path>,
) -> Result<BTreeMap<String, BTreeSet<String>>> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let index = read_index(&project_root)?;
    let mut by_task = BTreeMap::<String, BTreeSet<String>>::new();
    for record in index
        .file_ownership
        .into_iter()
        .filter(|record| record.active_to_event.is_none())
    {
        by_task
            .entry(record.task_id)
            .or_default()
            .insert(record.file_path);
    }
    Ok(by_task)
}

fn classify_one(file_path: &str, index: &FileOwnershipIndex) -> FileOwnershipClassification {
    let owner_task_ids: Vec<String> = index
        .file_ownership
        .iter()
        .filter(|record| record.active_to_event.is_none())
        .filter(|record| ownership_matches(&record.file_path, file_path))
        .map(|record| record.task_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let status = match owner_task_ids.len() {
        0 => FileOwnershipStatus::Unowned,
        1 => FileOwnershipStatus::OwnedBy,
        _ => FileOwnershipStatus::MultiOwned,
    };
    FileOwnershipClassification {
        file_path: file_path.to_string(),
        status,
        owner_task_ids,
    }
}

fn aggregate_action(
    files: &[FileOwnershipClassification],
    active_task_id: Option<&str>,
) -> OwnershipAggregateAction {
    if files.is_empty() {
        return OwnershipAggregateAction::NoFiles;
    }
    if files
        .iter()
        .any(|file| file.status == FileOwnershipStatus::MultiOwned)
    {
        return OwnershipAggregateAction::AskUserMultiOwner;
    }
    if files
        .iter()
        .all(|file| file.status == FileOwnershipStatus::Unowned)
    {
        return OwnershipAggregateAction::AskUserUnowned;
    }

    let owners: BTreeSet<String> = files
        .iter()
        .flat_map(|file| file.owner_task_ids.iter().cloned())
        .collect();
    if owners.len() == 1 && files.iter().all(|file| !file.owner_task_ids.is_empty()) {
        let owner = owners.iter().next().expect("owner exists");
        if active_task_id == Some(owner.as_str()) {
            OwnershipAggregateAction::AssignToActiveTask
        } else {
            OwnershipAggregateAction::AssignToOwnerTask
        }
    } else if active_task_id
        .map(|active| {
            files
                .iter()
                .any(|file| file.owner_task_ids.iter().any(|owner| owner == active))
        })
        .unwrap_or(false)
    {
        OwnershipAggregateAction::AskUserPartialOverlap
    } else {
        OwnershipAggregateAction::AskUserAllDrift
    }
}

fn prompt_for_action(
    files: &[FileOwnershipClassification],
    active_task_id: Option<&str>,
    action: &OwnershipAggregateAction,
) -> Option<OwnershipPrompt> {
    match action {
        OwnershipAggregateAction::AskUserUnowned => Some(OwnershipPrompt {
            kind: OwnershipPromptKind::NoIntersection,
            message: format!(
                "我看到这些改动目前不属于任何已知 VibeHub task：\n\n{}\n\n请选择处理方式：\n1. 开新 task，并把这些文件作为 implement 产出\n2. 归入当前 task {}\n3. 部分归入 / 部分开新 task\n4. 暂时只记录为 drift，不推进状态",
                render_file_list(files),
                active_task_id.unwrap_or("<none>")
            ),
        }),
        OwnershipAggregateAction::AskUserPartialOverlap => {
            let active_files = files
                .iter()
                .filter(|file| {
                    active_task_id
                        .map(|active| file.owner_task_ids.iter().any(|owner| owner == active))
                        .unwrap_or(false)
                })
                .map(|file| file.file_path.clone())
                .collect::<Vec<_>>();
            let drift_files = files
                .iter()
                .filter(|file| {
                    !active_task_id
                        .map(|active| file.owner_task_ids.iter().any(|owner| owner == active))
                        .unwrap_or(false)
                })
                .map(|file| file.file_path.clone())
                .collect::<Vec<_>>();
            Some(OwnershipPrompt {
                kind: OwnershipPromptKind::PartialOverlap,
                message: format!(
                    "这些文件看起来分属不同范围：\n\n当前 task 文件：\n{}\n\n疑似 drift / 其它 task 文件：\n{}\n\n请说明哪些文件归入当前 task，哪些应另开或切换 task。",
                    render_plain_files(&active_files),
                    render_plain_files(&drift_files)
                ),
            })
        }
        OwnershipAggregateAction::AskUserMultiOwner => Some(OwnershipPrompt {
            kind: OwnershipPromptKind::MultiOwner,
            message: format!(
                "以下文件同时命中多个 task 的 ownership：\n\n{}\n\n请为每个文件指定主要归属 task。VibeHub 只记录 ownership，不会修改 git。",
                render_ambiguous_table(files)
            ),
        }),
        OwnershipAggregateAction::AskUserAllDrift => Some(OwnershipPrompt {
            kind: OwnershipPromptKind::AllDrift,
            message: format!(
                "当前改动没有命中 active task {}，但命中了其它 task：\n\n{}\n\n是否切换到对应 task 继续，还是把这些改动登记为当前 task 的新增范围？",
                active_task_id.unwrap_or("<none>"),
                render_task_file_table(files)
            ),
        }),
        _ => None,
    }
}

fn render_file_list(files: &[FileOwnershipClassification]) -> String {
    render_plain_files(
        &files
            .iter()
            .map(|file| file.file_path.clone())
            .collect::<Vec<_>>(),
    )
}

fn render_plain_files(files: &[String]) -> String {
    if files.is_empty() {
        return "- (none)".to_string();
    }
    files
        .iter()
        .map(|file| format!("- {file}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_ambiguous_table(files: &[FileOwnershipClassification]) -> String {
    files
        .iter()
        .filter(|file| file.status == FileOwnershipStatus::MultiOwned)
        .map(|file| format!("- {}: {}", file.file_path, file.owner_task_ids.join(", ")))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_task_file_table(files: &[FileOwnershipClassification]) -> String {
    let mut by_task = BTreeMap::<String, Vec<String>>::new();
    for file in files {
        for owner in &file.owner_task_ids {
            by_task
                .entry(owner.clone())
                .or_default()
                .push(file.file_path.clone());
        }
    }
    by_task
        .into_iter()
        .map(|(task, files)| format!("- {task}: {}", files.join(", ")))
        .collect::<Vec<_>>()
        .join("\n")
}

fn ownership_matches(owned_path: &str, changed_path: &str) -> bool {
    owned_path == changed_path
        || (owned_path.ends_with('/') && changed_path.starts_with(owned_path))
}

fn active_records_for_task(index: &FileOwnershipIndex, task_id: &str) -> Vec<FileOwnershipRecord> {
    index
        .file_ownership
        .iter()
        .filter(|record| record.task_id == task_id && record.active_to_event.is_none())
        .cloned()
        .collect()
}

fn read_index(project_root: &Path) -> Result<FileOwnershipIndex> {
    let path = index_path(project_root);
    if !path.is_file() {
        return Ok(FileOwnershipIndex {
            schema_version: OWNERSHIP_SCHEMA_VERSION,
            file_ownership: Vec::new(),
        });
    }
    let content =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    let index: FileOwnershipIndex = serde_yaml::from_str(&content)
        .with_context(|| format!("Invalid YAML in {}", path.display()))?;
    if index.schema_version != OWNERSHIP_SCHEMA_VERSION {
        bail!(
            "Unsupported file ownership schema_version {} in {}",
            index.schema_version,
            path.display()
        );
    }
    Ok(index)
}

fn write_index(project_root: &Path, index: &FileOwnershipIndex) -> Result<()> {
    let path = index_path(project_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    let content = serde_yaml::to_string(index).context("Failed to serialize ownership index")?;
    fs::write(&path, content).with_context(|| format!("Failed to write {}", path.display()))
}

fn index_path(project_root: &Path) -> PathBuf {
    project_root.join(".vibehub/index/file-ownership.yaml")
}

fn index_path_relative(project_root: &Path) -> Result<String> {
    Ok(normalize_path(&relative_to_project(
        project_root,
        &index_path(project_root),
    )?))
}

fn normalize_scope_files(files: &[String]) -> Result<Vec<String>> {
    let mut normalized = BTreeSet::new();
    for file in files {
        let file = normalize_scope_file(file)?;
        normalized.insert(file);
    }
    Ok(normalized.into_iter().collect())
}

fn normalize_scope_file(file: &str) -> Result<String> {
    let trimmed = file.trim();
    if trimmed.is_empty() {
        bail!("scope file path cannot be empty");
    }
    let path = Path::new(trimmed);
    if path.is_absolute() {
        bail!("scope file path must be project-relative: {trimmed}");
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        bail!("scope file path must stay inside the project: {trimmed}");
    }
    let had_trailing_slash = trimmed.ends_with('/') || trimmed.ends_with('\\');
    let mut normalized = normalize_path(path);
    if had_trailing_slash && !normalized.ends_with('/') {
        normalized.push('/');
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_project() -> PathBuf {
        let path = std::env::temp_dir().join(format!("vibehub-ownership-test-{}", Uuid::new_v4()));
        fs::create_dir_all(path.join(".vibehub")).expect("create .vibehub");
        path
    }

    fn write_records(project: &Path, records: Vec<FileOwnershipRecord>) {
        write_index(
            project,
            &FileOwnershipIndex {
                schema_version: OWNERSHIP_SCHEMA_VERSION,
                file_ownership: records,
            },
        )
        .expect("write index");
    }

    fn record(file_path: &str, task_id: &str) -> FileOwnershipRecord {
        FileOwnershipRecord {
            file_path: file_path.to_string(),
            task_id: task_id.to_string(),
            run_id: Some(format!("R-{task_id}")),
            capability: Some("implement".to_string()),
            source: FileOwnershipSource::EventProjection,
            confidence: 0.9,
            active_from_event: Some("evt-1-0001".to_string()),
            active_to_event: None,
            first_seen_at: "2026-05-28T00:00:00Z".to_string(),
            last_seen_at: "2026-05-28T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn classifies_empty_file_set_as_no_files() {
        let project = temp_project();
        let report = classify_files(&project, vec![], Some("T-1")).expect("classify");
        assert_eq!(report.aggregate_action, OwnershipAggregateAction::NoFiles);
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn classifies_unowned_files_with_prompt() {
        let project = temp_project();
        let report = classify_files(&project, vec!["src/main.rs".to_string()], Some("T-1"))
            .expect("classify");
        assert_eq!(
            report.aggregate_action,
            OwnershipAggregateAction::AskUserUnowned
        );
        assert_eq!(
            report.prompt.as_ref().map(|prompt| &prompt.kind),
            Some(&OwnershipPromptKind::NoIntersection)
        );
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn assigns_single_intersection_to_active_task() {
        let project = temp_project();
        write_records(&project, vec![record("src/main.rs", "T-1")]);
        let report = classify_files(&project, vec!["src/main.rs".to_string()], Some("T-1"))
            .expect("classify");
        assert_eq!(
            report.aggregate_action,
            OwnershipAggregateAction::AssignToActiveTask
        );
        assert!(report.prompt.is_none());
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn assigns_single_intersection_to_owner_task() {
        let project = temp_project();
        write_records(&project, vec![record("src/main.rs", "T-2")]);
        let report = classify_files(&project, vec!["src/main.rs".to_string()], Some("T-1"))
            .expect("classify");
        assert_eq!(
            report.aggregate_action,
            OwnershipAggregateAction::AssignToOwnerTask
        );
        assert!(report.prompt.is_none());
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn asks_for_partial_overlap() {
        let project = temp_project();
        write_records(&project, vec![record("src/owned.rs", "T-1")]);
        let report = classify_files(
            &project,
            vec!["src/owned.rs".to_string(), "src/unowned.rs".to_string()],
            Some("T-1"),
        )
        .expect("classify");
        assert_eq!(
            report.aggregate_action,
            OwnershipAggregateAction::AskUserPartialOverlap
        );
        assert_eq!(
            report.prompt.as_ref().map(|prompt| &prompt.kind),
            Some(&OwnershipPromptKind::PartialOverlap)
        );
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn asks_for_multi_owner_conflict() {
        let project = temp_project();
        write_records(
            &project,
            vec![
                record("src/shared.rs", "T-1"),
                record("src/shared.rs", "T-2"),
            ],
        );
        let report = classify_files(&project, vec!["src/shared.rs".to_string()], Some("T-1"))
            .expect("classify");
        assert_eq!(
            report.aggregate_action,
            OwnershipAggregateAction::AskUserMultiOwner
        );
        assert_eq!(
            report.prompt.as_ref().map(|prompt| &prompt.kind),
            Some(&OwnershipPromptKind::MultiOwner)
        );
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn asks_for_all_drift_when_files_split_across_other_tasks() {
        let project = temp_project();
        write_records(&project, vec![record("a.rs", "T-2"), record("b.rs", "T-3")]);
        let report = classify_files(
            &project,
            vec!["a.rs".to_string(), "b.rs".to_string()],
            Some("T-1"),
        )
        .expect("classify");
        assert_eq!(
            report.aggregate_action,
            OwnershipAggregateAction::AskUserAllDrift
        );
        assert_eq!(
            report.prompt.as_ref().map(|prompt| &prompt.kind),
            Some(&OwnershipPromptKind::AllDrift)
        );
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn matches_directory_prefix_records() {
        let project = temp_project();
        write_records(&project, vec![record("src/vibehub/", "T-1")]);
        let report = classify_files(
            &project,
            vec!["src/vibehub/events.rs".to_string()],
            Some("T-1"),
        )
        .expect("classify");
        assert_eq!(
            report.aggregate_action,
            OwnershipAggregateAction::AssignToActiveTask
        );
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn manual_record_expires_other_active_owner() {
        let project = temp_project();
        write_records(&project, vec![record("src/shared.rs", "T-2")]);
        let result = record_file_ownership(
            &project,
            FileOwnershipRecordRequest {
                task_id: Some("T-1".to_string()),
                run_id: Some("R-1".to_string()),
                capability: Some("implement".to_string()),
                scope_files: vec!["src/shared.rs".to_string()],
            },
        )
        .expect("record");
        assert_eq!(result.files_added, vec!["src/shared.rs"]);
        assert_eq!(result.files_removed, vec!["src/shared.rs"]);

        let report = classify_files(&project, vec!["src/shared.rs".to_string()], Some("T-1"))
            .expect("classify");
        assert_eq!(
            report.aggregate_action,
            OwnershipAggregateAction::AssignToActiveTask
        );
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn rejects_paths_that_escape_project() {
        let project = temp_project();
        let err = classify_files(&project, vec!["../secret".to_string()], Some("T-1"))
            .expect_err("reject path");
        assert!(err.to_string().contains("must stay inside"));
        fs::remove_dir_all(project).ok();
    }
}

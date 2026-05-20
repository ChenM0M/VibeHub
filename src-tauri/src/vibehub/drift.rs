use crate::process_util::silent_command;
use crate::vibehub::locale::VibehubLocale;
use crate::vibehub::util::{canonical_project_root, normalize_path};
use crate::vibehub::{agent_adapter, agent_view, current, events};
use anyhow::{Context, Result};
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WorkspaceDriftReport {
    pub project_root: String,
    pub git_available: bool,
    pub head: Option<String>,
    pub last_seen_head: Option<String>,
    pub head_changed: bool,
    pub dirty: bool,
    pub changed_files: Vec<String>,
    pub context_stale: bool,
    pub adapter_conflicts: Vec<String>,
    pub warnings: Vec<String>,
    pub recommended_actions: Vec<String>,
    pub recover_report_path: Option<String>,
}

pub fn check_workspace_drift(project_path: impl AsRef<Path>) -> Result<WorkspaceDriftReport> {
    check_workspace_drift_with_locale(project_path, None)
}

pub fn check_workspace_drift_with_locale(
    project_path: impl AsRef<Path>,
    locale_override: Option<&str>,
) -> Result<WorkspaceDriftReport> {
    build_report(project_path.as_ref(), false, locale_override)
}

pub fn sync_workspace_state(project_path: impl AsRef<Path>) -> Result<WorkspaceDriftReport> {
    sync_workspace_state_with_locale(project_path, None)
}

pub fn sync_workspace_state_with_locale(
    project_path: impl AsRef<Path>,
    locale_override: Option<&str>,
) -> Result<WorkspaceDriftReport> {
    let project_root = canonical_project_root(project_path.as_ref())?;
    let _ = agent_view::generate_agent_view(&project_root);
    // Do NOT auto-run agent_adapter::sync_agent_adapters here. Per spec §18.5
    // (manual_by_default), adapter sync must be triggered explicitly by the
    // user, not as a side effect of every drift recovery.
    build_report(&project_root, true, locale_override)
}

fn build_report(
    project_path: &Path,
    write_recover: bool,
    locale_override: Option<&str>,
) -> Result<WorkspaceDriftReport> {
    let project_root = canonical_project_root(project_path)?;
    let locale = VibehubLocale::detect_with_override(&project_root, locale_override);
    let head = git_stdout(&project_root, &["rev-parse", "HEAD"]);
    let git_available = is_git_repo(&project_root);
    let changed_files = git_porcelain_lines(&project_root)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|line| porcelain_path(&line))
        .map(|line| line.replace('\\', "/"))
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    let dirty = !changed_files.is_empty();
    let last_seen_head = read_last_seen_head(&project_root);
    let head_changed = matches!((&head, &last_seen_head), (Some(head), Some(last)) if head != last);
    let context_stale = read_context_stale(&project_root).unwrap_or(false);
    let adapter_status = agent_adapter::get_agent_adapter_status(&project_root)?;
    let adapter_conflicts = adapter_status
        .files
        .into_iter()
        .filter(|file| file.status == "modified_outside_vibehub")
        .map(|file| file.path)
        .collect::<Vec<_>>();

    let mut warnings = Vec::new();
    let mut recommended_actions = Vec::new();
    if dirty {
        warnings.push(drift_warning_dirty(locale).to_string());
        recommended_actions.push(drift_action_review_diff(locale).to_string());
    }
    if head_changed {
        warnings.push(drift_warning_head_changed(locale).to_string());
        recommended_actions.push(drift_action_recover(locale).to_string());
    }
    if context_stale {
        warnings.push(drift_warning_context_stale(locale).to_string());
        recommended_actions.push(drift_action_rebuild_context(locale).to_string());
    }
    if !adapter_conflicts.is_empty() {
        warnings.push(drift_warning_adapter_conflict(locale).to_string());
        recommended_actions.push(drift_action_adapter_conflict(locale).to_string());
    }
    let repeated_edit_threshold = read_repeated_file_edit_threshold(&project_root).unwrap_or(8);
    if dirty && changed_files.len() >= repeated_edit_threshold {
        warnings.push(drift_warning_loop_file_edits(
            locale,
            changed_files.len(),
            repeated_edit_threshold,
        ));
        recommended_actions.push(drift_action_loop_review(locale).to_string());
    }
    if warnings.is_empty() {
        recommended_actions.push(drift_action_continue(locale).to_string());
    }

    let recover_report_path = if write_recover && !warnings.is_empty() {
        Some(write_recover_report(
            &project_root,
            locale,
            &warnings,
            &recommended_actions,
            &changed_files,
        )?)
    } else {
        None
    };

    Ok(WorkspaceDriftReport {
        project_root: normalize_path(&project_root),
        git_available,
        head,
        last_seen_head,
        head_changed,
        dirty,
        changed_files,
        context_stale,
        adapter_conflicts,
        warnings,
        recommended_actions,
        recover_report_path,
    })
}

fn write_recover_report(
    project_root: &Path,
    locale: VibehubLocale,
    warnings: &[String],
    actions: &[String],
    changed_files: &[String],
) -> Result<String> {
    let active = current::resolve_current_task(project_root)
        .ok()
        .and_then(|task| {
            current::resolve_current_run(project_root, &task.task_id)
                .ok()
                .map(|run| (task.task_id, run.run_id))
        });
    let path = if let Some((task_id, run_id)) = &active {
        project_root
            .join(".vibehub")
            .join("tasks")
            .join(task_id)
            .join("runs")
            .join(run_id)
            .join("recover.md")
    } else {
        project_root.join(".vibehub/recover.md")
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    let mut content = format!("# {}\n\n", recover_title(locale));
    content.push_str(&format!("## {}\n\n", recover_warnings(locale)));
    for warning in warnings {
        content.push_str(&format!("- {warning}\n"));
    }
    content.push_str(&format!("\n## {}\n\n", recover_actions(locale)));
    for action in actions {
        content.push_str(&format!("- {action}\n"));
    }
    content.push_str(&format!("\n## {}\n\n", recover_changed_files(locale)));
    if changed_files.is_empty() {
        content.push_str(&format!("- {}\n", locale.none_observed()));
    } else {
        for file in changed_files {
            content.push_str(&format!("- {file}\n"));
        }
    }
    fs::write(&path, content).with_context(|| format!("Failed to write {}", path.display()))?;
    let relative = normalize_path(path.strip_prefix(project_root).unwrap_or(path.as_path()));
    if let Some((task_id, run_id)) = active {
        let _ = events::append_run_event(
            project_root,
            &task_id,
            &run_id,
            "recover_report_written",
            "Recover report generated.",
            serde_json::json!({
                "recover_report_path": relative.clone(),
                "warnings_count": warnings.len(),
                "changed_files_count": changed_files.len(),
            }),
        );
    }
    Ok(relative)
}

fn drift_warning_dirty(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Git worktree has uncommitted changes observed outside VibeHub state.",
        VibehubLocale::ZhCn => "Git 工作区存在 VibeHub 状态之外观察到的未提交变更。",
        VibehubLocale::ZhTw => "Git 工作區存在 VibeHub 狀態之外觀察到的未提交變更。",
    }
}

fn drift_warning_head_changed(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Git HEAD differs from .vibehub/state.yaml last_seen_head.",
        VibehubLocale::ZhCn => "Git HEAD 与 .vibehub/state.yaml 中的 last_seen_head 不一致。",
        VibehubLocale::ZhTw => "Git HEAD 與 .vibehub/state.yaml 中的 last_seen_head 不一致。",
    }
}

fn drift_warning_context_stale(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Current context pack is marked stale.",
        VibehubLocale::ZhCn => "当前上下文包被标记为过期。",
        VibehubLocale::ZhTw => "目前上下文包被標記為過期。",
    }
}

fn drift_warning_adapter_conflict(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Generated AI instruction files were modified outside VibeHub.",
        VibehubLocale::ZhCn => "生成的 AI 指令文件在 VibeHub 之外被修改。",
        VibehubLocale::ZhTw => "產生的 AI 指令檔案在 VibeHub 之外被修改。",
    }
}

fn drift_action_review_diff(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Review the diff and run VibeHub Review Evidence.",
        VibehubLocale::ZhCn => "检查 diff，并运行 VibeHub Review Evidence。",
        VibehubLocale::ZhTw => "檢查 diff，並執行 VibeHub Review Evidence。",
    }
}

fn drift_action_recover(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Run Recover Drift before advancing task state.",
        VibehubLocale::ZhCn => "推进任务状态前先运行 Recover Drift。",
        VibehubLocale::ZhTw => "推進任務狀態前先執行 Recover Drift。",
    }
}

fn drift_action_rebuild_context(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Rebuild the current context pack.",
        VibehubLocale::ZhCn => "重建当前上下文包。",
        VibehubLocale::ZhTw => "重建目前上下文包。",
    }
}

fn drift_action_adapter_conflict(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => {
            "Resolve AI instruction conflicts in the VibeHub AI Instructions panel."
        }
        VibehubLocale::ZhCn => "在 VibeHub AI Instructions 面板中解决 AI 指令冲突。",
        VibehubLocale::ZhTw => "在 VibeHub AI Instructions 面板中解決 AI 指令衝突。",
    }
}

fn drift_warning_loop_file_edits(
    locale: VibehubLocale,
    changed_count: usize,
    threshold: usize,
) -> String {
    match locale {
        VibehubLocale::En => format!(
            "Loop warning: {} changed file(s) meets or exceeds repeated_file_edits threshold {}.",
            changed_count, threshold
        ),
        VibehubLocale::ZhCn => format!(
            "循环警告：{} 个变更文件已达到或超过 repeated_file_edits 阈值 {}。",
            changed_count, threshold
        ),
        VibehubLocale::ZhTw => format!(
            "循環警告：{} 個變更檔案已達到或超過 repeated_file_edits 閾值 {}。",
            changed_count, threshold
        ),
    }
}

fn drift_action_loop_review(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => {
            "Run review evidence or rebuild context before continuing a broad edit loop."
        }
        VibehubLocale::ZhCn => "大范围编辑循环继续前，先运行审查证据或重建上下文。",
        VibehubLocale::ZhTw => "大範圍編輯循環繼續前，先執行審查證據或重建上下文。",
    }
}

fn drift_action_continue(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "No drift detected; continue from .vibehub/agent-view/current.md.",
        VibehubLocale::ZhCn => "未检测到漂移；从 .vibehub/agent-view/current.md 继续。",
        VibehubLocale::ZhTw => "未偵測到漂移；從 .vibehub/agent-view/current.md 繼續。",
    }
}

fn recover_title(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "VibeHub Recover Report",
        VibehubLocale::ZhCn => "VibeHub 恢复报告",
        VibehubLocale::ZhTw => "VibeHub 恢復報告",
    }
}

fn recover_warnings(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Warnings",
        VibehubLocale::ZhCn => "警告",
        VibehubLocale::ZhTw => "警告",
    }
}

fn recover_actions(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Recommended Actions",
        VibehubLocale::ZhCn => "建议动作",
        VibehubLocale::ZhTw => "建議動作",
    }
}

fn recover_changed_files(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Changed Files",
        VibehubLocale::ZhCn => "变更文件",
        VibehubLocale::ZhTw => "變更檔案",
    }
}

fn is_git_repo(project_root: &Path) -> bool {
    silent_command("git")
        .arg("-C")
        .arg(project_root)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn git_stdout(project_root: &Path, args: &[&str]) -> Option<String> {
    let output = silent_command("git")
        .arg("-C")
        .arg(project_root)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn git_porcelain_lines(project_root: &Path) -> Option<Vec<String>> {
    let output = silent_command("git")
        .arg("-C")
        .arg(project_root)
        .args(["status", "--porcelain"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| !line.is_empty())
            .map(ToString::to_string)
            .collect(),
    )
}

fn porcelain_path(line: &str) -> Option<String> {
    if line.len() < 4 {
        return None;
    }
    let path = &line[3..];
    Some(path.split(" -> ").last().unwrap_or(path).trim().to_string())
}

fn read_last_seen_head(project_root: &Path) -> Option<String> {
    let state = read_state(project_root)?;
    state
        .get("git")?
        .get("last_seen_head")?
        .as_str()
        .map(ToString::to_string)
}

fn read_context_stale(project_root: &Path) -> Option<bool> {
    let state = read_state(project_root)?;
    state.get("context")?.get("stale")?.as_bool()
}

fn read_state(project_root: &Path) -> Option<serde_yaml::Value> {
    fs::read_to_string(project_root.join(".vibehub/state.yaml"))
        .ok()
        .and_then(|content| serde_yaml::from_str(&content).ok())
}

fn read_repeated_file_edit_threshold(project_root: &Path) -> Option<usize> {
    let content =
        fs::read_to_string(project_root.join(".vibehub/rules/loop-detection.yaml")).ok()?;
    let value: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;
    value
        .get("signals")?
        .get("repeated_file_edits")?
        .get("threshold")?
        .as_u64()
        .map(|value| value as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vibehub::current;
    use std::path::PathBuf;
    use std::process::Command;
    use uuid::Uuid;

    fn temp_project() -> PathBuf {
        let path = std::env::temp_dir().join(format!("vibehub-drift-test-{}", Uuid::new_v4()));
        fs::create_dir_all(path.join(".vibehub/adapters")).expect("create temp project");
        fs::write(
            path.join(".vibehub/state.yaml"),
            "git:\n  last_seen_head: old\ncontext:\n  stale: true\n",
        )
        .expect("state");
        path
    }

    #[test]
    fn detects_dirty_worktree_and_stale_context() {
        let project = temp_project();
        Command::new("git")
            .arg("-C")
            .arg(&project)
            .arg("init")
            .output()
            .expect("git init");
        fs::write(project.join("changed.txt"), "changed").expect("write");

        let report = check_workspace_drift(&project).expect("drift");

        assert!(report.dirty);
        assert!(report.context_stale);
        assert!(!report.warnings.is_empty());

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn parses_porcelain_paths_without_dropping_first_character() {
        assert_eq!(porcelain_path(" M AGENTS.md").as_deref(), Some("AGENTS.md"));
        assert_eq!(
            porcelain_path("?? src-tauri/src/main.rs").as_deref(),
            Some("src-tauri/src/main.rs")
        );
        assert_eq!(
            porcelain_path("R  old.rs -> src/new.rs").as_deref(),
            Some("src/new.rs")
        );
    }

    #[test]
    fn recover_report_prefers_active_run_path() {
        let project = temp_project();
        let task_dir = project.join(".vibehub/tasks/T-001");
        let run_dir = task_dir.join("runs/R-001");
        fs::create_dir_all(&run_dir).expect("create run");
        current::write_current_task_pointer(&project, "T-001").expect("task pointer");
        current::write_current_run_pointer(&project, "T-001", "R-001").expect("run pointer");
        Command::new("git")
            .arg("-C")
            .arg(&project)
            .arg("init")
            .output()
            .expect("git init");
        fs::write(project.join("changed.txt"), "changed").expect("write changed");

        let report = sync_workspace_state(&project).expect("recover");

        assert_eq!(
            report.recover_report_path.as_deref(),
            Some(".vibehub/tasks/T-001/runs/R-001/recover.md")
        );
        assert!(project
            .join(".vibehub/tasks/T-001/runs/R-001/events.jsonl")
            .is_file());

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn loop_detection_threshold_adds_warning() {
        let project = temp_project();
        fs::create_dir_all(project.join(".vibehub/rules")).expect("rules");
        fs::write(
            project.join(".vibehub/rules/loop-detection.yaml"),
            "signals:\n  repeated_file_edits:\n    threshold: 1\n",
        )
        .expect("loop rule");
        Command::new("git")
            .arg("-C")
            .arg(&project)
            .arg("init")
            .output()
            .expect("git init");
        fs::write(project.join("changed.txt"), "changed").expect("write changed");

        let report = check_workspace_drift(&project).expect("drift");

        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("Loop warning")));

        fs::remove_dir_all(project).expect("cleanup");
    }
}

use crate::process_util::silent_command;
use crate::vibehub::{agent_adapter, agent_view};
use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

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
    build_report(project_path.as_ref(), false)
}

pub fn sync_workspace_state(project_path: impl AsRef<Path>) -> Result<WorkspaceDriftReport> {
    let project_root = canonical_project_root(project_path.as_ref())?;
    let _ = agent_view::generate_agent_view(&project_root);
    let _ = agent_adapter::sync_agent_adapters(&project_root, None, false);
    build_report(&project_root, true)
}

fn build_report(project_path: &Path, write_recover: bool) -> Result<WorkspaceDriftReport> {
    let project_root = canonical_project_root(project_path)?;
    let head = git_stdout(&project_root, &["rev-parse", "HEAD"]);
    let git_available = is_git_repo(&project_root);
    let changed_files = git_lines(&project_root, &["status", "--porcelain"])
        .unwrap_or_default()
        .into_iter()
        .map(|line| line.chars().skip(3).collect::<String>())
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
        warnings.push(
            "Git worktree has uncommitted changes observed outside VibeHub state.".to_string(),
        );
        recommended_actions.push("Review the diff and run VibeHub Review Evidence.".to_string());
    }
    if head_changed {
        warnings.push("Git HEAD differs from .vibehub/state.yaml last_seen_head.".to_string());
        recommended_actions.push("Run Recover Drift before advancing task state.".to_string());
    }
    if context_stale {
        warnings.push("Current context pack is marked stale.".to_string());
        recommended_actions.push("Rebuild the current context pack.".to_string());
    }
    if !adapter_conflicts.is_empty() {
        warnings.push("Generated AI instruction files were modified outside VibeHub.".to_string());
        recommended_actions.push(
            "Resolve AI instruction conflicts in the VibeHub AI Instructions panel.".to_string(),
        );
    }
    if warnings.is_empty() {
        recommended_actions
            .push("No drift detected; continue from .vibehub/agent-view/current.md.".to_string());
    }

    let recover_report_path = if write_recover && !warnings.is_empty() {
        Some(write_recover_report(
            &project_root,
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
    warnings: &[String],
    actions: &[String],
    changed_files: &[String],
) -> Result<String> {
    let path = project_root.join(".vibehub/recover.md");
    let mut content = String::from("# VibeHub Recover Report\n\n");
    content.push_str("## Warnings\n\n");
    for warning in warnings {
        content.push_str(&format!("- {warning}\n"));
    }
    content.push_str("\n## Recommended Actions\n\n");
    for action in actions {
        content.push_str(&format!("- {action}\n"));
    }
    content.push_str("\n## Changed Files\n\n");
    if changed_files.is_empty() {
        content.push_str("- None observed.\n");
    } else {
        for file in changed_files {
            content.push_str(&format!("- {file}\n"));
        }
    }
    fs::write(&path, content).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(normalize_path(
        path.strip_prefix(project_root).unwrap_or(path.as_path()),
    ))
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

fn git_lines(project_root: &Path, args: &[&str]) -> Option<Vec<String>> {
    let output = silent_command("git")
        .arg("-C")
        .arg(project_root)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToString::to_string)
            .collect(),
    )
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

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
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
}

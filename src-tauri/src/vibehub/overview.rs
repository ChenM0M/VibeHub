// Aggregated cockpit overview.
//
// Replaces the per-tab read commands so the frontend can pull every panel's
// data in a single Tauri IPC round-trip and benefit from one cached `git`
// invocation per call. The shape combines the previous outputs of:
//
//   * vibehub_read_cockpit_status
//   * vibehub_read_context_view
//   * vibehub_read_review_view
//   * vibehub_read_handoff_view
//   * vibehub_read_diff_view
//   * vibehub_read_research_status

use crate::process_util::silent_command;
use crate::vibehub::status::VibehubCockpitStatus;
use crate::vibehub::util::canonical_project_root;
use crate::vibehub::{
    cockpit::{ContextViewData, HandoffViewData, ReviewViewData},
    research::ResearchStatus,
    status,
};
use anyhow::Result;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DiffViewData {
    pub dirty: bool,
    pub changed_files: Vec<String>,
    pub changed_files_count: usize,
    pub diff_stat: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CockpitOverview {
    pub status: VibehubCockpitStatus,
    pub context: Option<ContextViewData>,
    pub review: Option<ReviewViewData>,
    pub handoff: Option<HandoffViewData>,
    pub diff: DiffViewData,
    pub research: ResearchStatus,
    /// True when the per-tab data was skipped because `.vibehub/` is missing.
    pub initialized: bool,
}

pub fn read_overview(project_root: impl AsRef<Path>) -> Result<CockpitOverview> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    let status = status::read_cockpit_status(&project_root)?;

    // Compute git data ONCE per overview call. The per-tab commands each used
    // to spawn their own `git status --porcelain` / `git diff --stat`.
    let diff = read_diff_cached(&project_root);

    if !status.initialized {
        return Ok(CockpitOverview {
            status,
            context: None,
            review: None,
            handoff: None,
            diff,
            research: crate::vibehub::research::read_research_status(&project_root),
            initialized: false,
        });
    }

    let context = crate::vibehub::cockpit::read_context_view(&project_root).ok();
    let review = crate::vibehub::cockpit::read_review_view(&project_root).ok();
    let handoff = crate::vibehub::cockpit::read_handoff_view(&project_root).ok();
    let research = crate::vibehub::research::read_research_status(&project_root);

    Ok(CockpitOverview {
        status,
        context,
        review,
        handoff,
        diff,
        research,
        initialized: true,
    })
}

fn read_diff_cached(project_root: &Path) -> DiffViewData {
    if !is_git_repo(project_root) {
        return DiffViewData {
            dirty: false,
            changed_files: Vec::new(),
            changed_files_count: 0,
            diff_stat: Vec::new(),
        };
    }

    let mut tracked = git_lines(project_root, &["diff", "--name-only", "HEAD", "--"]);
    let untracked = git_lines(
        project_root,
        &["ls-files", "--others", "--exclude-standard"],
    );
    tracked.extend(untracked);
    tracked.sort();
    tracked.dedup();
    let dirty = !tracked.is_empty();
    let diff_stat = git_lines(project_root, &["diff", "--stat", "HEAD", "--"]);

    DiffViewData {
        dirty,
        changed_files_count: tracked.len(),
        changed_files: tracked,
        diff_stat,
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

fn git_lines(project_root: &Path, args: &[&str]) -> Vec<String> {
    let Ok(output) = silent_command("git")
        .arg("-C")
        .arg(project_root)
        .args(args)
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.replace('\\', "/"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn temp_project() -> PathBuf {
        let p = std::env::temp_dir().join(format!("vibehub-overview-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&p).expect("create");
        p
    }

    #[test]
    fn returns_uninitialized_overview_without_vibehub() {
        let project = temp_project();
        let overview = read_overview(&project).expect("overview");
        assert!(!overview.initialized);
        assert!(overview.context.is_none());
        assert!(overview.review.is_none());
        assert!(overview.handoff.is_none());
        fs::remove_dir_all(project).ok();
    }
}

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
use crate::vibehub::branches::{self, GitBranchesView};
use crate::vibehub::notes::{self, ProjectDigest};
use crate::vibehub::status::VibehubCockpitStatus;
use crate::vibehub::util::canonical_project_root;
use crate::vibehub::{
    cockpit::{ContextViewData, HandoffViewData, ReviewViewData},
    research::ResearchStatus,
    status,
};
use anyhow::Result;
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DiffViewData {
    pub dirty: bool,
    pub changed_files: Vec<String>,
    pub changed_files_count: usize,
    pub diff_stat: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FlowArtifact {
    pub label: String,
    pub path: String,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FlowDetail {
    pub phase: String,
    pub status: String,
    pub context_spec_path: String,
    pub context_spec_exists: bool,
    pub context_pack_path: String,
    pub context_pack_exists: bool,
    pub manifest_path: String,
    pub manifest_exists: bool,
    pub phase_output_path: String,
    pub phase_output_exists: bool,
    pub review_path: Option<String>,
    pub review_exists: bool,
    pub read_inputs: Vec<FlowArtifact>,
    pub written_outputs: Vec<FlowArtifact>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CockpitOverview {
    pub status: VibehubCockpitStatus,
    pub context: Option<ContextViewData>,
    pub review: Option<ReviewViewData>,
    pub handoff: Option<HandoffViewData>,
    pub diff: DiffViewData,
    pub research: ResearchStatus,
    /// Agent-written project-level one-liner digest (summary + status). The
    /// fields fall back to `None` when the seeded templates were never
    /// replaced. VibeHub never writes these files after init.
    pub project_digest: ProjectDigest,
    /// Multi-branch git read-model. Pure read; no fetch, no checkout. Empty
    /// when the workspace is not a git repo.
    pub git_branches: GitBranchesView,
    /// Per-phase read/write artifacts for the 5-stage flow drawer. Pure read:
    /// missing files are reported as placeholders instead of being created.
    pub flow_details: Vec<FlowDetail>,
    /// True when the per-tab data was skipped because `.vibehub/` is missing.
    pub initialized: bool,
}

pub fn read_overview(project_root: impl AsRef<Path>) -> Result<CockpitOverview> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    let status = status::read_cockpit_status(&project_root)?;

    // Compute git data ONCE per overview call. The per-tab commands each used
    // to spawn their own `git status --porcelain` / `git diff --stat`.
    let diff = read_diff_cached(&project_root);
    // Always compute the multi-branch read-model: it's useful in the
    // uninitialized view too (so the dashboard can still show "you're on
    // branch X with N other local branches" before init runs).
    let git_branches = branches::read_git_branches(&project_root);

    if !status.initialized {
        return Ok(CockpitOverview {
            status,
            context: None,
            review: None,
            handoff: None,
            diff,
            research: crate::vibehub::research::read_research_status(&project_root),
            project_digest: ProjectDigest::default(),
            git_branches,
            flow_details: Vec::new(),
            initialized: false,
        });
    }

    let context = crate::vibehub::cockpit::read_context_view(&project_root).ok();
    let review = crate::vibehub::cockpit::read_review_view(&project_root).ok();
    let handoff = crate::vibehub::cockpit::read_handoff_view(&project_root).ok();
    let research = crate::vibehub::research::read_research_status(&project_root);
    let project_digest = with_digest_fallbacks(
        notes::read_project_digest(&project_root).unwrap_or_default(),
        &status,
    );
    let flow_details = read_flow_details(&project_root, &status);

    Ok(CockpitOverview {
        status,
        context,
        review,
        handoff,
        diff,
        research,
        project_digest,
        git_branches,
        flow_details,
        initialized: true,
    })
}

fn read_flow_details(project_root: &Path, status: &VibehubCockpitStatus) -> Vec<FlowDetail> {
    let (Some(task_id), Some(run_id)) = (
        status.current_task_id.as_deref(),
        status.current_run_id.as_deref(),
    ) else {
        return Vec::new();
    };

    status
        .flow
        .iter()
        .map(|item| {
            let context_spec_path = format!(".vibehub/tasks/{task_id}/context/{}.yaml", item.phase);
            let context_pack_path = format!(
                ".vibehub/tasks/{task_id}/runs/{run_id}/context-packs/{}.md",
                item.phase
            );
            let manifest_path = format!(
                ".vibehub/tasks/{task_id}/runs/{run_id}/context-packs/{}.manifest.yaml",
                item.phase
            );
            let phase_output_path = format!(
                ".vibehub/tasks/{task_id}/runs/{run_id}/phases/{}.output.md",
                item.phase
            );
            let review_path = if item.phase.contains("review") {
                Some(format!(
                    ".vibehub/tasks/{task_id}/runs/{run_id}/phases/review.md"
                ))
            } else {
                None
            };

            let context_spec_exists = rel_exists(project_root, &context_spec_path);
            let context_pack_exists = rel_exists(project_root, &context_pack_path);
            let manifest_exists = rel_exists(project_root, &manifest_path);
            let phase_output_exists = rel_exists(project_root, &phase_output_path);
            let review_exists = review_path
                .as_deref()
                .map(|path| rel_exists(project_root, path))
                .unwrap_or(false);

            let read_inputs = vec![
                FlowArtifact {
                    label: "Context spec".to_string(),
                    path: context_spec_path.clone(),
                    exists: context_spec_exists,
                },
                FlowArtifact {
                    label: "Context pack".to_string(),
                    path: context_pack_path.clone(),
                    exists: context_pack_exists,
                },
                FlowArtifact {
                    label: "Context manifest".to_string(),
                    path: manifest_path.clone(),
                    exists: manifest_exists,
                },
            ];

            let mut written_outputs = vec![FlowArtifact {
                label: "Phase output".to_string(),
                path: phase_output_path.clone(),
                exists: phase_output_exists,
            }];
            if let Some(path) = &review_path {
                written_outputs.push(FlowArtifact {
                    label: "Review output".to_string(),
                    path: path.clone(),
                    exists: review_exists,
                });
            }

            FlowDetail {
                phase: item.phase.clone(),
                status: item.status.clone(),
                context_spec_path,
                context_spec_exists,
                context_pack_path,
                context_pack_exists,
                manifest_path,
                manifest_exists,
                phase_output_path,
                phase_output_exists,
                review_path,
                review_exists,
                read_inputs,
                written_outputs,
            }
        })
        .collect()
}

fn with_digest_fallbacks(
    mut digest: ProjectDigest,
    status: &VibehubCockpitStatus,
) -> ProjectDigest {
    if digest.summary_line.is_none() {
        digest.summary_line = status.current_task_title.clone();
    }
    if digest.status_line.is_none() {
        digest.status_line = match (
            status.current_phase.as_deref(),
            status.phase_status.as_deref(),
        ) {
            (Some(phase), Some(phase_status)) => {
                Some(format!("Current phase: {phase} ({phase_status})."))
            }
            (Some(phase), None) => Some(format!("Current phase: {phase}.")),
            _ => None,
        };
    }
    digest
}

fn rel_exists(project_root: &Path, rel_path: &str) -> bool {
    fs::metadata(project_root.join(rel_path))
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
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
    use crate::vibehub::{init, start_task};
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
        assert!(overview.flow_details.is_empty());
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn reports_flow_details_without_creating_outputs() {
        let project = temp_project();
        init::init_project(&project).expect("init");
        let started = start_task::start_task(
            &project,
            Some("Overview flow detail test".to_string()),
            Some("guided_drive".to_string()),
            Some("implement".to_string()),
        )
        .expect("start task");

        let output_path = project
            .join(".vibehub")
            .join("tasks")
            .join(&started.task_id)
            .join("runs")
            .join(&started.run_id)
            .join("phases")
            .join("implement.output.md");
        assert!(!output_path.exists());

        let overview = read_overview(&project).expect("overview");
        assert_eq!(
            overview.project_digest.summary_line.as_deref(),
            Some("Overview flow detail test")
        );
        assert_eq!(
            overview.project_digest.status_line.as_deref(),
            Some("Current phase: implement (active).")
        );
        let implement = overview
            .flow_details
            .iter()
            .find(|detail| detail.phase == "implement")
            .expect("implement detail");

        assert!(implement.context_spec_exists);
        assert!(implement.context_pack_exists);
        assert!(implement.manifest_exists);
        assert!(!implement.phase_output_exists);
        assert!(implement
            .phase_output_path
            .ends_with("/phases/implement.output.md"));
        assert!(implement
            .read_inputs
            .iter()
            .any(|artifact| artifact.label == "Context pack" && artifact.exists));
        assert!(implement
            .written_outputs
            .iter()
            .any(|artifact| artifact.label == "Phase output" && !artifact.exists));
        assert!(!output_path.exists());

        fs::remove_dir_all(project).ok();
    }
}

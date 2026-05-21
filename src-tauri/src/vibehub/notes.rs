// Project-level digest reader (`.vibehub/notes/summary.md` + `status.md`).
//
// These two files are AGENT-WRITABLE one-liners surfaced in the cockpit
// dashboard. VibeHub seeds them at init via `init.rs` and NEVER overwrites
// them afterwards — agents update them through `vibehub-checkpoint` /
// `vibehub-finish` adapter commands. This reader stays read-only.

use crate::vibehub::util::canonical_project_root;
use anyhow::Result;
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Default)]
pub struct ProjectDigest {
    /// One-sentence project summary, or `None` if the file is missing or
    /// still contains only the seeded placeholder.
    pub summary_line: Option<String>,
    /// One-sentence project status, or `None` if the file is missing or still
    /// contains only the seeded placeholder.
    pub status_line: Option<String>,
    /// Workspace-relative path to the summary file (always reported so the
    /// frontend can deep-link even when the file does not exist yet).
    pub summary_path: String,
    /// Workspace-relative path to the status file.
    pub status_path: String,
    /// True when `.vibehub/notes/summary.md` exists on disk.
    pub summary_exists: bool,
    /// True when `.vibehub/notes/status.md` exists on disk.
    pub status_exists: bool,
}

pub fn read_project_digest(project_path: impl AsRef<Path>) -> Result<ProjectDigest> {
    let project_root = canonical_project_root(project_path.as_ref())?;
    let summary_rel = ".vibehub/notes/summary.md";
    let status_rel = ".vibehub/notes/status.md";
    let summary_abs = project_root.join(summary_rel);
    let status_abs = project_root.join(status_rel);

    Ok(ProjectDigest {
        summary_line: extract_first_meaningful_line(&summary_abs),
        status_line: extract_first_meaningful_line(&status_abs),
        summary_path: summary_rel.to_string(),
        status_path: status_rel.to_string(),
        summary_exists: summary_abs.exists(),
        status_exists: status_abs.exists(),
    })
}

/// Return the first non-empty, non-heading, non-comment, non-placeholder line
/// from `path`, or `None` when the file does not exist / has no real content.
///
/// Filters applied:
///   * `# Heading` lines (any level)
///   * blank lines
///   * HTML comments `<!-- ... -->` (single-line; multi-line comments are also
///     skipped until a closing `-->`)
///   * lines that consist entirely of an italic placeholder (`_..._`) — these
///     are the seeded "no summary yet" hint emitted by `init.rs`.
fn extract_first_meaningful_line(path: &Path) -> Option<String> {
    let contents = fs::read_to_string(path).ok()?;
    let mut in_html_comment = false;
    let mut in_italic_placeholder = false;
    for raw in contents.lines() {
        let line = raw.trim();
        if in_italic_placeholder {
            if line.ends_with('_') {
                in_italic_placeholder = false;
            }
            continue;
        }
        if in_html_comment {
            if line.contains("-->") {
                in_html_comment = false;
            }
            continue;
        }
        if line.is_empty() {
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        if line.starts_with("<!--") {
            if !line.contains("-->") {
                in_html_comment = true;
            }
            continue;
        }
        // Seeded placeholder italic line (single-line `_..._`)
        if line.starts_with('_') && line.ends_with('_') && line.len() > 2 {
            continue;
        }
        // Seeded placeholders may wrap across lines in the init template.
        if line.starts_with('_') {
            in_italic_placeholder = true;
            continue;
        }
        return Some(line.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn temp_project() -> PathBuf {
        let p = std::env::temp_dir().join(format!("vibehub-notes-test-{}", Uuid::new_v4()));
        fs::create_dir_all(p.join(".vibehub/notes")).expect("mkdir");
        p
    }

    #[test]
    fn returns_paths_even_when_files_missing() {
        let project = temp_project();
        // Don't create files — make sure dir exists is the only requirement.
        let digest = read_project_digest(&project).expect("digest");
        assert!(digest.summary_line.is_none());
        assert!(digest.status_line.is_none());
        assert!(!digest.summary_exists);
        assert!(!digest.status_exists);
        assert_eq!(digest.summary_path, ".vibehub/notes/summary.md");
        assert_eq!(digest.status_path, ".vibehub/notes/status.md");
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn skips_heading_comment_and_placeholder() {
        let project = temp_project();
        fs::write(
            project.join(".vibehub/notes/summary.md"),
            "# Project Summary\n\n<!-- hint -->\n\n_No summary yet._\n",
        )
        .unwrap();
        let digest = read_project_digest(&project).expect("digest");
        assert!(digest.summary_exists);
        assert!(digest.summary_line.is_none());
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn skips_wrapped_italic_placeholder() {
        let project = temp_project();
        fs::write(
            project.join(".vibehub/notes/summary.md"),
            "# Project Summary\n\n_No project summary yet. Run `vibehub-finish` (or have an agent write here) to\nfill in one sentence about the project goal._\n",
        )
        .unwrap();
        let digest = read_project_digest(&project).expect("digest");
        assert!(digest.summary_line.is_none());
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn returns_first_real_line() {
        let project = temp_project();
        fs::write(
            project.join(".vibehub/notes/summary.md"),
            "# Project Summary\n\n<!-- ignored -->\nVibeHub is a project orchestration desktop app.\n\nMore details below.\n",
        )
        .unwrap();
        fs::write(
            project.join(".vibehub/notes/status.md"),
            "# Project Status\n\nFixing lifecycle gaps; next step is the cockpit overview extension.\n",
        )
        .unwrap();
        let digest = read_project_digest(&project).expect("digest");
        assert_eq!(
            digest.summary_line.as_deref(),
            Some("VibeHub is a project orchestration desktop app.")
        );
        assert_eq!(
            digest.status_line.as_deref(),
            Some("Fixing lifecycle gaps; next step is the cockpit overview extension.")
        );
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn skips_multiline_html_comment() {
        let project = temp_project();
        fs::write(
            project.join(".vibehub/notes/summary.md"),
            "# Project Summary\n\n<!-- multi\n line comment -->\nReal summary line.\n",
        )
        .unwrap();
        let digest = read_project_digest(&project).expect("digest");
        assert_eq!(digest.summary_line.as_deref(), Some("Real summary line."));
        fs::remove_dir_all(project).ok();
    }
}

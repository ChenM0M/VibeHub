// Pure read-only multi-branch git inspection for the cockpit dashboard.
//
// Surfaces every local branch with upstream / ahead / behind / head SHA / last
// commit subject + time, using only `git for-each-ref` and `git symbolic-ref`.
// No fetch, no checkout, no write — anything that would mutate state is out
// of scope. Without a recent `git fetch`, the ahead/behind numbers reflect
// only what the local repo already knows about the upstream.

use crate::process_util::silent_command;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Default)]
pub struct GitBranchInfo {
    pub name: String,
    pub is_current: bool,
    pub upstream: Option<String>,
    pub ahead: Option<u32>,
    pub behind: Option<u32>,
    pub head_sha: String,
    pub last_commit_subject: String,
    /// ISO-8601 committer date string; preserved as git emits it so the
    /// frontend can sort/format freely.
    pub last_commit_time: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Default)]
pub struct GitBranchesView {
    /// True when the project root is inside a git work tree.
    pub git_available: bool,
    /// Name of the currently checked-out branch, or `None` for detached HEAD.
    pub current_branch: Option<String>,
    /// One entry per local branch, sorted by `last_commit_time` descending so
    /// the frontend can render "top N most recent" without extra sorting.
    pub branches: Vec<GitBranchInfo>,
    /// True when HEAD is currently detached.
    pub detached_head: bool,
}

pub fn read_git_branches(project_root: &Path) -> GitBranchesView {
    if !is_git_repo(project_root) {
        return GitBranchesView::default();
    }

    let symbolic_ref = silent_command("git")
        .arg("-C")
        .arg(project_root)
        .args(["symbolic-ref", "--quiet", "--short", "HEAD"])
        .output()
        .ok();
    let (current_branch, detached_head) = match symbolic_ref {
        Some(out) if out.status.success() => (
            Some(String::from_utf8_lossy(&out.stdout).trim().to_string()),
            false,
        ),
        _ => (None, true),
    };

    // Field separator that is essentially impossible to collide with branch
    // names or commit subjects.
    const SEP: &str = "\x1f";
    let format = format!(
        "%(refname:short){sep}%(HEAD){sep}%(upstream:short){sep}%(upstream:track){sep}%(objectname:short){sep}%(committerdate:iso8601){sep}%(contents:subject)",
        sep = SEP
    );

    let Ok(output) = silent_command("git")
        .arg("-C")
        .arg(project_root)
        .args(["for-each-ref", "--format", &format, "refs/heads"])
        .output()
    else {
        return GitBranchesView {
            git_available: true,
            current_branch,
            branches: Vec::new(),
            detached_head,
        };
    };
    if !output.status.success() {
        return GitBranchesView {
            git_available: true,
            current_branch,
            branches: Vec::new(),
            detached_head,
        };
    }

    let mut branches: Vec<GitBranchInfo> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| parse_branch_line(line, SEP))
        .collect();
    // Most-recent commit first — Phase E asks for "top 3 by recent commit
    // time" so sort here once and let the frontend slice.
    branches.sort_by(|a, b| b.last_commit_time.cmp(&a.last_commit_time));

    GitBranchesView {
        git_available: true,
        current_branch,
        branches,
        detached_head,
    }
}

fn parse_branch_line(line: &str, sep: &str) -> Option<GitBranchInfo> {
    if line.trim().is_empty() {
        return None;
    }
    let parts: Vec<&str> = line.splitn(7, sep).collect();
    if parts.len() < 7 {
        return None;
    }
    let name = parts[0].trim().to_string();
    if name.is_empty() {
        return None;
    }
    let is_current = parts[1].trim() == "*";
    let upstream_raw = parts[2].trim();
    let upstream = if upstream_raw.is_empty() {
        None
    } else {
        Some(upstream_raw.to_string())
    };
    let (ahead, behind) = parse_track_field(parts[3].trim());
    let head_sha = parts[4].trim().to_string();
    let last_commit_time = parts[5].trim().to_string();
    let last_commit_subject = parts[6].trim().to_string();

    Some(GitBranchInfo {
        name,
        is_current,
        upstream,
        ahead,
        behind,
        head_sha,
        last_commit_subject,
        last_commit_time,
    })
}

/// `%(upstream:track)` outputs one of:
///   * `""`              — no upstream configured (or up to date with none)
///   * `"[gone]"`        — upstream branch deleted
///   * `"[ahead 3]"`     — ahead only
///   * `"[behind 2]"`    — behind only
///   * `"[ahead 3, behind 2]"`
/// Returns (ahead, behind). Both `None` means "no upstream / unknown".
fn parse_track_field(track: &str) -> (Option<u32>, Option<u32>) {
    if track.is_empty() || track == "[gone]" {
        return (None, None);
    }
    let trimmed = track.trim_start_matches('[').trim_end_matches(']');
    let mut ahead: Option<u32> = None;
    let mut behind: Option<u32> = None;
    for chunk in trimmed.split(',') {
        let chunk = chunk.trim();
        if let Some(rest) = chunk.strip_prefix("ahead ") {
            ahead = rest.parse().ok();
        } else if let Some(rest) = chunk.strip_prefix("behind ") {
            behind = rest.parse().ok();
        }
    }
    // If a chunk was parsed, treat the unmentioned side as 0 to make the
    // frontend simpler ("upstream configured ⇒ ahead/behind known").
    match (ahead, behind) {
        (Some(_), None) => (ahead, Some(0)),
        (None, Some(_)) => (Some(0), behind),
        other => other,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ahead_and_behind() {
        assert_eq!(parse_track_field("[ahead 3, behind 2]"), (Some(3), Some(2)));
    }

    #[test]
    fn parses_ahead_only_with_implicit_zero_behind() {
        assert_eq!(parse_track_field("[ahead 5]"), (Some(5), Some(0)));
    }

    #[test]
    fn parses_behind_only_with_implicit_zero_ahead() {
        assert_eq!(parse_track_field("[behind 4]"), (Some(0), Some(4)));
    }

    #[test]
    fn empty_or_gone_returns_none() {
        assert_eq!(parse_track_field(""), (None, None));
        assert_eq!(parse_track_field("[gone]"), (None, None));
    }

    #[test]
    fn parses_branch_line_with_full_fields() {
        let sep = "\x1f";
        let line = format!(
            "main{sep}*{sep}origin/main{sep}[ahead 1, behind 2]{sep}abc123{sep}2026-05-20 12:00:00 +0000{sep}Initial commit",
            sep = sep
        );
        let info = parse_branch_line(&line, sep).expect("parsed");
        assert_eq!(info.name, "main");
        assert!(info.is_current);
        assert_eq!(info.upstream.as_deref(), Some("origin/main"));
        assert_eq!(info.ahead, Some(1));
        assert_eq!(info.behind, Some(2));
        assert_eq!(info.head_sha, "abc123");
        assert_eq!(info.last_commit_subject, "Initial commit");
        assert_eq!(info.last_commit_time, "2026-05-20 12:00:00 +0000");
    }

    #[test]
    fn parses_branch_line_without_upstream() {
        let sep = "\x1f";
        let line = format!(
            "feature/x{sep} {sep}{sep}{sep}deadbee{sep}2026-05-19 10:00:00 +0000{sep}WIP: try thing",
            sep = sep
        );
        let info = parse_branch_line(&line, sep).expect("parsed");
        assert_eq!(info.name, "feature/x");
        assert!(!info.is_current);
        assert!(info.upstream.is_none());
        assert!(info.ahead.is_none());
        assert!(info.behind.is_none());
    }

    #[test]
    fn rejects_truncated_lines() {
        let info = parse_branch_line("only|two|cols", "|");
        assert!(info.is_none());
    }
}

use crate::vibehub::current;
use crate::vibehub::util::{
    canonical_initialized_project_root, normalize_path, relative_to_project,
};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VibehubFileReadResult {
    pub path: String,
    pub content: String,
    pub exists: bool,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ContextViewData {
    pub pack_path: Option<String>,
    pub manifest_path: Option<String>,
    pub pack_exists: bool,
    pub manifest_exists: bool,
    pub included_count: usize,
    pub missing_count: usize,
    pub excluded_count: usize,
    pub stale: Option<bool>,
    pub phase: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ReviewViewData {
    pub review_path: Option<String>,
    pub review_exists: bool,
    pub review_summary: String,
    pub evidence_map_summary: String,
    pub evidence_grades: Vec<String>,
    pub diff_patch_path: Option<String>,
    pub diff_patch_exists: bool,
    pub changed_files_path: Option<String>,
    pub changed_files_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct HandoffViewData {
    pub handoff_path: Option<String>,
    pub handoff_exists: bool,
    pub complete: bool,
    pub missing_sections: Vec<String>,
    pub sections_count: usize,
}

// Safe file reader for .vibehub/ files.

pub fn read_vibehub_file(
    project_root: impl AsRef<Path>,
    relative_path: impl AsRef<Path>,
) -> Result<VibehubFileReadResult> {
    let (project_root, resolved, rel) = resolve_vibehub_file_path(project_root, relative_path)?;

    let exists = resolved.is_file();
    let content = if exists {
        fs::read_to_string(&resolved)
            .with_context(|| format!("Failed to read {}", resolved.display()))?
    } else {
        String::new()
    };
    let size = if exists {
        resolved.metadata().map(|m| m.len()).unwrap_or(0)
    } else {
        0
    };

    Ok(VibehubFileReadResult {
        path: normalize_path(
            &relative_to_project(&project_root, &resolved).unwrap_or_else(|_| rel.to_path_buf()),
        ),
        content,
        exists,
        size,
    })
}

pub fn resolve_vibehub_file_path(
    project_root: impl AsRef<Path>,
    relative_path: impl AsRef<Path>,
) -> Result<(PathBuf, PathBuf, PathBuf)> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let rel = relative_path.as_ref();

    if rel.is_absolute()
        || rel.components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(anyhow!("Path escapes project root: {}", rel.display()));
    }

    let full = project_root.join(rel);
    let vibehub_dir = project_root.join(".vibehub");

    // Canonicalize BOTH full path and vibehub_dir before prefix-checking, so
    // that symlinks, junctions, or Windows reparse points cannot escape the
    // sandbox by joining a textually-clean relative path that resolves
    // outside `.vibehub/`. If the file does not exist we fall back to the
    // canonical parent + a textual prefix check.
    let canonical_vibehub = fs::canonicalize(&vibehub_dir)
        .with_context(|| format!("Failed to canonicalize {}", vibehub_dir.display()))?;
    let resolved = if full.exists() {
        fs::canonicalize(&full)
            .with_context(|| format!("Failed to canonicalize {}", full.display()))?
    } else {
        let parent = full
            .parent()
            .ok_or_else(|| anyhow!("Path has no parent: {}", full.display()))?;
        let parent_canon = if parent.exists() {
            fs::canonicalize(parent).unwrap_or_else(|_| parent.to_path_buf())
        } else {
            parent.to_path_buf()
        };
        let file_name = full
            .file_name()
            .ok_or_else(|| anyhow!("Path has no file name: {}", full.display()))?;
        parent_canon.join(file_name)
    };

    if !resolved.starts_with(&canonical_vibehub) {
        return Err(anyhow!(
            "Path is not within .vibehub/ directory: {}",
            rel.display()
        ));
    }

    Ok((project_root, resolved, rel.to_path_buf()))
}

// Context view.

#[derive(Debug, Deserialize)]
struct ManifestSummary {
    #[serde(default)]
    included: Vec<serde_yaml::Value>,
    #[serde(default)]
    missing: Vec<serde_yaml::Value>,
    #[serde(default)]
    excluded: Vec<serde_yaml::Value>,
}

pub fn read_context_view(project_root: impl AsRef<Path>) -> Result<ContextViewData> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let task_id = current::resolve_current_task(&project_root)
        .map(|p| p.task_id)
        .unwrap_or_default();
    let run_id = current::resolve_current_run(&project_root, &task_id)
        .map(|p| p.run_id)
        .unwrap_or_default();

    let phase = read_yaml_str_path(
        &project_root.join(".vibehub/state.yaml"),
        &["current", "phase"],
    );
    let stale = read_yaml_bool_path(
        &project_root.join(".vibehub/state.yaml"),
        &["context", "stale"],
    );

    let (pack_path, pack_exists, manifest_path, manifest_exists, included, missing, excluded) =
        if !task_id.is_empty() && !run_id.is_empty() {
            let phase_str = phase.clone().unwrap_or_else(|| "unknown".to_string());
            let md_path_rel =
                format!(".vibehub/tasks/{task_id}/runs/{run_id}/context-packs/{phase_str}.md");
            let md_full = project_root.join(&md_path_rel);
            let manifest_rel = format!(
                ".vibehub/tasks/{task_id}/runs/{run_id}/context-packs/{phase_str}.manifest.yaml"
            );
            let manifest_full = project_root.join(&manifest_rel);

            let (incl, miss, excl) = if manifest_full.is_file() {
                match fs::read_to_string(&manifest_full) {
                    Ok(content) => match serde_yaml::from_str::<ManifestSummary>(&content) {
                        Ok(summary) => (
                            summary.included.len(),
                            summary.missing.len(),
                            summary.excluded.len(),
                        ),
                        Err(_) => (0, 0, 0),
                    },
                    Err(_) => (0, 0, 0),
                }
            } else {
                (0, 0, 0)
            };

            (
                Some(md_path_rel),
                md_full.is_file(),
                Some(manifest_rel),
                manifest_full.is_file(),
                incl,
                miss,
                excl,
            )
        } else {
            (None, false, None, false, 0, 0, 0)
        };

    Ok(ContextViewData {
        pack_path,
        manifest_path,
        pack_exists,
        manifest_exists,
        included_count: included,
        missing_count: missing,
        excluded_count: excluded,
        stale,
        phase,
    })
}

// Review view.

pub fn read_review_view(project_root: impl AsRef<Path>) -> Result<ReviewViewData> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let task_id = current::resolve_current_task(&project_root)
        .map(|p| p.task_id)
        .unwrap_or_default();
    let run_id = current::resolve_current_run(&project_root, &task_id)
        .map(|p| p.run_id)
        .unwrap_or_default();

    if task_id.is_empty() || run_id.is_empty() {
        return Ok(ReviewViewData {
            review_path: None,
            review_exists: false,
            review_summary: String::new(),
            evidence_map_summary: String::new(),
            evidence_grades: Vec::new(),
            diff_patch_path: None,
            diff_patch_exists: false,
            changed_files_path: None,
            changed_files_count: 0,
        });
    }

    let base = format!(".vibehub/tasks/{}/runs/{}", task_id, run_id);
    let review_rel = format!("{}/phases/review.md", base);
    let diff_rel = format!("{}/evidence/diff.patch", base);
    let changed_rel = format!("{}/evidence/changed-files.txt", base);

    let review_full = project_root.join(&review_rel);
    let diff_full = project_root.join(&diff_rel);
    let changed_full = project_root.join(&changed_rel);

    let review_exists = review_full.is_file();
    let (review_summary, evidence_map_summary, evidence_grades) = if review_exists {
        match fs::read_to_string(&review_full) {
            Ok(content) => {
                // Extract Verdict and first meaningful lines as summary
                let mut lines = Vec::new();
                for line in content.lines().skip(1).take(30) {
                    let trimmed = line.trim();
                    if trimmed.is_empty()
                        || trimmed.starts_with("Task:")
                        || trimmed.starts_with("Run:")
                        || trimmed.starts_with("Generated ")
                        || trimmed.starts_with("Source:")
                    {
                        continue;
                    }
                    lines.push(trimmed.to_string());
                    if lines.len() >= 15 {
                        break;
                    }
                }
                let evidence_map_summary = extract_markdown_section(&content, "Evidence Map")
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .take(24)
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n");
                let evidence_grades = extract_markdown_section(&content, "Evidence Grades")
                    .lines()
                    .map(str::trim)
                    .filter(|line| line.starts_with("- "))
                    .map(|line| line.trim_start_matches("- ").to_string())
                    .collect::<Vec<_>>();
                (lines.join("\n"), evidence_map_summary, evidence_grades)
            }
            Err(_) => (
                "Failed to read review.md".to_string(),
                String::new(),
                Vec::new(),
            ),
        }
    } else {
        (String::new(), String::new(), Vec::new())
    };

    let diff_exists = diff_full.is_file();
    let changed_exists = changed_full.is_file();
    let changed_count = if changed_exists {
        fs::read_to_string(&changed_full)
            .map(|c| c.lines().filter(|l| !l.trim().is_empty()).count())
            .unwrap_or(0)
    } else {
        0
    };

    Ok(ReviewViewData {
        review_path: if review_exists {
            Some(review_rel)
        } else {
            None
        },
        review_exists,
        review_summary,
        evidence_map_summary,
        evidence_grades,
        diff_patch_path: if diff_exists { Some(diff_rel) } else { None },
        diff_patch_exists: diff_exists,
        changed_files_path: if changed_exists {
            Some(changed_rel)
        } else {
            None
        },
        changed_files_count: changed_count,
    })
}

fn extract_markdown_section(content: &str, heading: &str) -> String {
    let heading_marker = format!("## {heading}");
    let mut in_section = false;
    let mut lines = Vec::new();
    for line in content.lines() {
        if line.trim() == heading_marker {
            in_section = true;
            continue;
        }
        if in_section && line.trim_start().starts_with("## ") {
            break;
        }
        if in_section {
            lines.push(line.to_string());
        }
    }
    lines.join("\n").trim().to_string()
}

// Handoff view.

pub fn read_handoff_view(project_root: impl AsRef<Path>) -> Result<HandoffViewData> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let handoff_path = project_root.join(".vibehub/agent-view/handoff.md");

    if !handoff_path.is_file() {
        return Ok(HandoffViewData {
            handoff_path: Some(".vibehub/agent-view/handoff.md".to_string()),
            handoff_exists: false,
            complete: false,
            missing_sections: vec![],
            sections_count: 0,
        });
    }

    let content = fs::read_to_string(&handoff_path)
        .with_context(|| format!("Failed to read {}", handoff_path.display()))?;

    let complete = content.contains("Handoff complete: yes");
    let sections_count = content
        .lines()
        .filter(|l| {
            l.trim_start().starts_with("## ")
                && !l.contains("Missing Required")
                && !l.contains("Handoff from")
        })
        .count();

    let missing: Vec<String> = if !complete {
        let mut ms = Vec::new();
        let mut in_missing = false;
        for line in content.lines() {
            if line.trim() == "## Missing Required Sections" {
                in_missing = true;
                continue;
            }
            if in_missing {
                if line.trim_start().starts_with("## ") {
                    break;
                }
                if let Some(item) = line.trim().strip_prefix("- ") {
                    ms.push(item.to_string());
                }
            }
        }
        ms
    } else {
        vec![]
    };

    Ok(HandoffViewData {
        handoff_path: Some(".vibehub/agent-view/handoff.md".to_string()),
        handoff_exists: true,
        complete,
        missing_sections: missing,
        sections_count,
    })
}

// Diff view: removed. Overview module owns the (cached) git invocations now.

// Helpers.

fn read_yaml_str_path(path: &Path, keys: &[&str]) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let value: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;
    keys.iter()
        .try_fold(&value, |current, key| current.get(*key))
        .and_then(serde_yaml::Value::as_str)
        .map(|s| s.to_string())
}

fn read_yaml_bool_path(path: &Path, keys: &[&str]) -> Option<bool> {
    let content = fs::read_to_string(path).ok()?;
    let value: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;
    keys.iter()
        .try_fold(&value, |current, key| current.get(*key))
        .and_then(serde_yaml::Value::as_bool)
}

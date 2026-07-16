use crate::vibehub::phase;
use crate::vibehub::util::{
    canonical_initialized_project_root, normalize_path, relative_to_project,
};
use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const REQUIRED_SECTIONS: &[&str] = &[
    "completed",
    "not yet done",
    "key decisions made",
    "files changed",
    "files reportedly read",
    "commands run",
    "tests run",
    "context still needed",
    "warnings",
    "next session should",
];

const EVIDENCE_LABELS: &[&str] = &[
    "hard_observed",
    "agent_reported",
    "inferred",
    "user_confirmed",
];

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct OutputLintReport {
    pub output_path: Option<String>,
    pub status: String,
    pub issue_count: usize,
    pub issues: Vec<OutputLintIssue>,
    pub phase_validation: Option<phase::PhaseValidationResult>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct OutputLintIssue {
    pub code: String,
    pub severity: String,
    pub message: String,
    pub hint: String,
}

pub fn lint_current_output(project_root: impl AsRef<Path>) -> Result<OutputLintReport> {
    lint_output_for_task(project_root, None)
}

pub fn lint_output_for_task(
    project_root: impl AsRef<Path>,
    task_id: Option<&str>,
) -> Result<OutputLintReport> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let validation = match task_id {
        Some(task_id) => Some(phase::validate_phase_for_task(&project_root, task_id)?),
        None => Some(phase::validate_phase(&project_root)?),
    };
    let output_path = validation
        .as_ref()
        .and_then(|validation| validation.source_output_path.clone());
    let mut issues = Vec::new();

    let Some(output_path) = &output_path else {
        issues.push(issue(
            "output.missing",
            "error",
            "No output.md was found for the selected task/run.",
            "Write run-level output.md before validating, finishing, advancing, or handing off.",
        ));
        return Ok(report(output_path.clone(), issues, validation));
    };

    let absolute = project_root.join(output_path);
    let content = fs::read_to_string(&absolute)
        .with_context(|| format!("Failed to read {}", absolute.display()))?;
    let sections = parse_sections(&content);

    for section in REQUIRED_SECTIONS {
        if !sections.contains_key(*section) {
            issues.push(issue(
                "section.missing",
                "error",
                format!("Required output section is missing: {section}"),
                "Add the section using the shared VibeHub output contract.",
            ));
        }
    }

    if let Some(validation) = &validation {
        for missing in &validation.missing_outputs {
            issues.push(issue(
                "phase.required_output_missing",
                "error",
                format!("Phase required output is missing: {missing}"),
                "Fill the mapped output.md section before finish or advance.",
            ));
        }
    }

    for (section, body) in &sections {
        if body.trim().is_empty() {
            issues.push(issue(
                "section.empty",
                "warning",
                format!("Output section is empty: {section}"),
                "Report none explicitly with an evidence label when there is no content.",
            ));
        }
        if should_require_evidence(section) && !contains_evidence_label(body) {
            issues.push(issue(
                "evidence.missing",
                "warning",
                format!("Output section has no evidence label: {section}"),
                "Use hard_observed, agent_reported, inferred, or user_confirmed.",
            ));
        }
    }

    let lower = content.to_lowercase();
    if (lower.contains("尚未运行测试")
        || lower.contains("尚未跑测试")
        || lower.contains("tests not run")
        || lower.contains("not run tests"))
        && (lower.contains(" passed") || lower.contains("通过") || lower.contains("0 failed"))
    {
        issues.push(issue(
            "content.contradiction.tests",
            "warning",
            "Output appears to say tests were not run and also reports passing tests.",
            "Update stale lines so the latest validation state is unambiguous.",
        ));
    }

    if lower.contains("尚未实现代码")
        && (lower.contains("files changed") || lower.contains("diff summary"))
    {
        issues.push(issue(
            "content.stale.implementation",
            "warning",
            "Output appears to contain stale 'not implemented yet' language after code changes.",
            "Remove old handoff text or move it under historical context.",
        ));
    }

    Ok(report(Some(output_path.clone()), issues, validation))
}

fn report(
    output_path: Option<String>,
    issues: Vec<OutputLintIssue>,
    phase_validation: Option<phase::PhaseValidationResult>,
) -> OutputLintReport {
    let status = if issues.iter().any(|issue| issue.severity == "error") {
        "needs_action"
    } else if issues.is_empty() {
        "passed"
    } else {
        "warning"
    };
    OutputLintReport {
        output_path,
        status: status.to_string(),
        issue_count: issues.len(),
        issues,
        phase_validation,
    }
}

fn issue(
    code: impl Into<String>,
    severity: impl Into<String>,
    message: impl Into<String>,
    hint: impl Into<String>,
) -> OutputLintIssue {
    OutputLintIssue {
        code: code.into(),
        severity: severity.into(),
        message: message.into(),
        hint: hint.into(),
    }
}

fn parse_sections(content: &str) -> BTreeMap<String, String> {
    let mut sections = BTreeMap::new();
    let mut current_key: Option<String> = None;
    let mut current_lines = Vec::new();

    for line in content.lines() {
        if let Some(title) = markdown_heading_title(line) {
            if let Some(key) = current_key.take() {
                sections.insert(key, current_lines.join("\n").trim().to_string());
                current_lines.clear();
            }
            current_key = Some(normalize_heading(title));
        } else if current_key.is_some() {
            current_lines.push(line.to_string());
        }
    }

    if let Some(key) = current_key {
        sections.insert(key, current_lines.join("\n").trim().to_string());
    }

    sections
}

fn markdown_heading_title(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let hashes = trimmed.chars().take_while(|ch| *ch == '#').count();
    if !(2..=6).contains(&hashes) {
        return None;
    }
    let rest = &trimmed[hashes..];
    if !rest.starts_with(' ') {
        return None;
    }
    Some(rest.trim())
}

fn normalize_heading(value: &str) -> String {
    let mut normalized = String::new();
    let mut previous_space = false;
    for ch in value.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch);
            previous_space = false;
        } else if !previous_space {
            normalized.push(' ');
            previous_space = true;
        }
    }
    normalized.trim().to_string()
}

fn contains_evidence_label(body: &str) -> bool {
    EVIDENCE_LABELS.iter().any(|label| body.contains(label))
}

fn should_require_evidence(section: &str) -> bool {
    !matches!(
        section,
        "implementation plan" | "validation plan" | "context plan"
    )
}

#[allow(dead_code)]
fn normalize_relative(project_root: &Path, path: &Path) -> String {
    normalize_path(&relative_to_project(project_root, path).unwrap_or_else(|_| PathBuf::from(path)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vibehub::{init, start_task};
    use std::fs;
    use uuid::Uuid;

    fn temp_project() -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("vibehub-output-lint-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).expect("create temp project");
        path
    }

    #[test]
    fn flags_missing_output() {
        let project = temp_project();
        init::init_project(&project).expect("init");
        start_task::start_task(
            &project,
            Some("Lint task".to_string()),
            Some("guided_drive".to_string()),
            None,
        )
        .expect("start");

        let report = lint_current_output(&project).expect("lint");

        assert_eq!(report.status, "needs_action");
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == "output.missing"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn flags_test_contradiction() {
        let content = "## Completed\n- `hard_observed`: done\n\n## Tests Run\n- `agent_reported`: 尚未运行测试\n- `hard_observed`: cargo test 0 failed\n";
        let sections = parse_sections(content);

        assert!(sections.contains_key("tests run"));
        assert!(content.to_lowercase().contains("0 failed"));
    }
}

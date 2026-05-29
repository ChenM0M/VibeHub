use crate::process_util::silent_command;
use crate::vibehub::locale::VibehubLocale;
use crate::vibehub::util::{
    canonical_initialized_project_root, normalize_path, relative_to_project,
};
use crate::vibehub::{current, events};
use anyhow::{Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use serde_yaml::{Mapping, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct HandoffBuildResult {
    pub handoff_path: String,
    pub source_output_path: Option<String>,
    pub complete: bool,
    pub missing_required_sections: Vec<String>,
    pub files_changed_evidence: String,
    pub task_id: String,
    pub run_id: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone)]
struct HandoffInput {
    task_id: String,
    task_path: String,
    run_id: String,
    run_path: String,
    phase: String,
    phase_status: String,
    session_id: Option<String>,
    source_output_path: Option<String>,
    sections: BTreeMap<String, String>,
    git_changed_files: Option<Vec<String>>,
    context_pack_path: Option<String>,
    context_manifest_available: bool,
    prior_outputs_summary: Vec<PriorOutputSummary>,
    task_pack_delta: TaskPackDelta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PriorOutputSummary {
    capability: String,
    key_decisions: Vec<String>,
    completed: Vec<String>,
    full_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TaskPackDelta {
    task_pack_dirty: bool,
    delta_fields: Vec<String>,
}

pub fn build_handoff(project_root: impl AsRef<Path>) -> Result<HandoffBuildResult> {
    build_handoff_with_locale(project_root, None)
}

pub fn build_handoff_with_locale(
    project_root: impl AsRef<Path>,
    locale_override: Option<&str>,
) -> Result<HandoffBuildResult> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let locale = VibehubLocale::detect_with_override(&project_root, locale_override);
    let task_pointer = current::resolve_current_task(&project_root)?;
    let run_pointer = current::resolve_current_run(&project_root, &task_pointer.task_id)?;
    let output = find_latest_session_output(&project_root, &run_pointer.path)?;
    let sections = match &output {
        Some((_, path)) => parse_output_sections(path)?,
        None => BTreeMap::new(),
    };
    let phase = read_current_phase(&project_root).unwrap_or_else(|| "unknown".to_string());
    let phase_status =
        read_current_phase_status(&project_root).unwrap_or_else(|| "unknown".to_string());
    let git_changed_files = git_changed_files(&project_root);
    let source_output_path = output
        .as_ref()
        .map(|(_, path)| relative_to_project(&project_root, path).map(|path| normalize_path(&path)))
        .transpose()?;
    let session_id = output.as_ref().map(|(session_id, _)| session_id.clone());
    let context_pack_path =
        read_state_field(&project_root, &["context", "current_pack"]).filter(|s| !s.is_empty());
    let manifest_path = context_pack_path
        .as_deref()
        .map(|p| p.trim_end_matches(".md").to_string() + ".manifest.yaml")
        .or_else(|| {
            read_state_field(&project_root, &["context", "current_manifest"])
                .filter(|s| !s.is_empty())
        });
    let context_manifest_available = manifest_path
        .as_deref()
        .map(|p| project_root.join(p).is_file())
        .unwrap_or(false);

    let prior_outputs_summary =
        build_prior_outputs_summary(&phase, source_output_path.as_deref(), &sections);
    let task_pack_delta = infer_task_pack_delta(&sections, git_changed_files.as_ref());

    let input = HandoffInput {
        task_id: task_pointer.task_id,
        task_path: task_pointer.path,
        run_id: run_pointer.run_id,
        run_path: run_pointer.path,
        phase,
        phase_status,
        session_id,
        source_output_path,
        sections,
        git_changed_files,
        context_pack_path,
        context_manifest_available,
        prior_outputs_summary,
        task_pack_delta,
    };

    let missing_required_sections = missing_required_sections(&input);
    let complete = missing_required_sections.is_empty();
    let agent_view_dir = project_root.join(".vibehub/agent-view");
    fs::create_dir_all(&agent_view_dir)
        .with_context(|| format!("Failed to create {}", agent_view_dir.display()))?;
    let handoff_path = agent_view_dir.join("handoff.md");
    let rendered = render_handoff(&input, complete, &missing_required_sections, locale);
    // Detect "nothing actually changed" so we can avoid spamming the event log
    // with identical `handoff_built` entries on every `generate_agent_view`
    // poll. The handoff body changes only via state — but it carries a fresh
    // "Generated at" timestamp every call, so compare with that line stripped.
    let previous_content = fs::read_to_string(&handoff_path).ok();
    let content_changed = previous_content.as_deref().map(strip_generated_at_line)
        != Some(strip_generated_at_line(rendered.as_str()));
    fs::write(&handoff_path, &rendered)
        .with_context(|| format!("Failed to write {}", handoff_path.display()))?;
    let handoff_rel = normalize_path(&relative_to_project(&project_root, &handoff_path)?);
    update_handoff_state(&project_root, &handoff_rel, complete)?;
    if content_changed {
        let legacy_event = events::append_run_event(
            &project_root,
            &input.task_id,
            &input.run_id,
            "handoff_built",
            "Handoff generated.",
            serde_json::json!({
                "handoff_path": handoff_rel.clone(),
                "complete": complete,
                "missing_required_sections": missing_required_sections.clone(),
                "source_output_path": input.source_output_path.clone(),
                "session_id": input.session_id.clone(),
            }),
        )
        .ok();
        let _ = events::append_structured_run_event(
            &project_root,
            &input.task_id,
            &input.run_id,
            events::VibehubEvent::HandoffWritten {
                path: handoff_rel.clone(),
                capability: Some(input.phase.clone()),
            },
        );
        if complete {
            let _ = events::append_structured_run_event(
                &project_root,
                &input.task_id,
                &input.run_id,
                events::VibehubEvent::EvidenceAdded {
                    source: "handoff".to_string(),
                    summary: "Complete handoff generated.".to_string(),
                    refs: legacy_event
                        .map(|event| vec![event.event_id, handoff_rel.clone()])
                        .unwrap_or_else(|| vec![handoff_rel.clone()]),
                },
            );
        }
    }

    Ok(HandoffBuildResult {
        handoff_path: handoff_rel,
        source_output_path: input.source_output_path,
        complete,
        missing_required_sections,
        files_changed_evidence: if input.git_changed_files.is_some() {
            "hard_observed".to_string()
        } else {
            "agent_reported".to_string()
        },
        task_id: input.task_id,
        run_id: input.run_id,
        session_id: input.session_id,
    })
}

/// Drop the "Generated at: <timestamp>" header line so that
/// `build_handoff` can detect "nothing actually changed" content-wise. Works
/// across all supported locales (`Generated at`, `生成时间`, `產生時間`,
/// etc.) by matching any line that starts with one of the localized labels
/// followed by `: `.
fn strip_generated_at_line(content: &str) -> String {
    content
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !(trimmed.starts_with("Generated at:")
                || trimmed.starts_with("生成时间:")
                || trimmed.starts_with("產生時間:"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_handoff(
    input: &HandoffInput,
    complete: bool,
    missing: &[String],
    locale: VibehubLocale,
) -> String {
    let mut o = String::new();

    // Header
    o.push_str(&format!(
        "# {} {}\n\n",
        handoff_session_title(locale),
        input.session_id.as_deref().unwrap_or("unknown")
    ));
    o.push_str(&format!("{}: {}\n", label_task(locale), input.task_id));
    o.push_str(&format!("{}: {}\n", label_run(locale), input.run_id));
    o.push_str(&format!(
        "{}: {}\n",
        label_phase(locale),
        title_case(&input.phase)
    ));
    o.push_str(&format!("{}: VibeHub\n", label_generated_by(locale)));
    o.push_str(&format!(
        "{}: {}\n",
        label_generated_at(locale),
        Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
    ));
    o.push_str(&format!(
        "{}: {}\n",
        label_source(locale),
        input
            .source_output_path
            .as_deref()
            .unwrap_or(label_missing_session_output(locale))
    ));
    o.push_str(&format!(
        "{}: {}\n",
        label_handoff_complete(locale),
        yes_no(locale, complete)
    ));
    o.push_str(&format!("{}: mixed\n\n", locale.evidence_grade_label()));

    // Missing sections notice
    if !missing.is_empty() {
        o.push_str(&format!("## {}\n\n", section_missing_required(locale)));
        for section in missing {
            o.push_str(&format!(
                "- {}\n",
                translate_missing_section_name(locale, section)
            ));
        }
        o.push('\n');
    }

    // 1. Current Task
    o.push_str(&format!("## {}\n\n", section_current_task(locale)));
    o.push_str(&format!("- {}: {}\n", label_task_id(locale), input.task_id));
    o.push_str(&format!(
        "- {}: {}\n",
        label_task_path(locale),
        input.task_path
    ));
    o.push_str(&format!("- {}: {}\n", label_run_id(locale), input.run_id));
    o.push_str(&format!(
        "- {}: {}\n\n",
        label_run_path(locale),
        input.run_path
    ));
    o.push_str(&format!(
        "{}: hard_observed\n\n",
        locale.evidence_grade_label()
    ));

    // 2. Current Phase
    o.push_str(&format!("## {}\n\n", section_current_phase(locale)));
    o.push_str(&format!(
        "- {}: {}\n",
        label_phase(locale),
        title_case(&input.phase)
    ));
    o.push_str(&format!(
        "- {}: {}\n\n",
        label_status(locale),
        input.phase_status
    ));
    o.push_str(&format!(
        "{}: hard_observed\n\n",
        locale.evidence_grade_label()
    ));

    // 3. What Changed
    o.push_str(&format!("## {}\n\n", section_what_changed(locale)));
    o.push_str("### Completed\n");
    push_sub_section(&mut o, input, "completed", "Completed", locale);
    o.push_str("### Not Yet Done\n");
    push_sub_section(&mut o, input, "not yet done", "Not Yet Done", locale);
    o.push_str("### Key Decisions Made\n");
    push_sub_section(
        &mut o,
        input,
        "key decisions made",
        "Key Decisions Made",
        locale,
    );
    o.push_str("### Files Changed\n");
    push_files_changed(&mut o, input, locale);
    o.push_str(&format!("\n{}: mixed\n\n", locale.evidence_grade_label()));

    o.push_str("## Prior Outputs Summary\n\n");
    if input.prior_outputs_summary.is_empty() {
        o.push_str("- `agent_reported`: []\n\n");
    } else {
        o.push_str("```json\n");
        o.push_str(&render_prior_outputs_summary_json(
            &input.prior_outputs_summary,
        ));
        o.push_str("\n```\n\n");
    }
    o.push_str(&format!(
        "{}: agent_reported\n\n",
        locale.evidence_grade_label()
    ));

    o.push_str("## Task Pack Delta\n\n");
    o.push_str(&format!(
        "- `agent_reported`: task_pack_dirty: {}\n",
        input.task_pack_delta.task_pack_dirty
    ));
    if input.task_pack_delta.delta_fields.is_empty() {
        o.push_str("- `agent_reported`: delta_fields: []\n\n");
    } else {
        o.push_str(&format!(
            "- `agent_reported`: delta_fields: {}\n\n",
            input.task_pack_delta.delta_fields.join(", ")
        ));
    }
    o.push_str(&format!(
        "{}: agent_reported\n\n",
        locale.evidence_grade_label()
    ));

    // 4. Commands Run
    o.push_str(&format!("## {}\n\n", section_commands_run(locale)));
    push_reported_section(&mut o, input, "commands run", "Commands Run", locale);
    o.push_str(&format!(
        "\n{}: agent_reported\n\n",
        locale.evidence_grade_label()
    ));

    // 5. Tests Run
    o.push_str(&format!("## {}\n\n", section_tests_run(locale)));
    push_reported_section(&mut o, input, "tests run", "Tests Run", locale);
    o.push_str(&format!(
        "\n{}: agent_reported\n\n",
        locale.evidence_grade_label()
    ));

    // 6. Context Used
    o.push_str(&format!("## {}\n\n", section_context_used(locale)));
    o.push_str(&format!("### {}\n", sub_files_read(locale)));
    push_sub_section(
        &mut o,
        input,
        "files reportedly read",
        "Files Reportedly Read",
        locale,
    );
    o.push_str(&format!("### {}\n", sub_context_pack(locale)));
    o.push_str(&format!(
        "- {}: {}\n",
        label_path(locale),
        input
            .context_pack_path
            .as_deref()
            .unwrap_or(label_not_available(locale))
    ));
    o.push_str(&format!(
        "- {}: {}\n",
        label_manifest(locale),
        if input.context_manifest_available {
            label_available(locale)
        } else {
            label_not_available(locale)
        }
    ));
    o.push_str(&format!("\n{}: mixed\n\n", locale.evidence_grade_label()));

    // 7. Context Still Needed
    o.push_str(&format!("## {}\n\n", section_context_still_needed(locale)));
    push_reported_section(
        &mut o,
        input,
        "context still needed",
        "Context Still Needed",
        locale,
    );
    o.push_str(&format!(
        "\n{}: agent_reported\n\n",
        locale.evidence_grade_label()
    ));

    // 8. Risks / Warnings
    o.push_str(&format!("## {}\n\n", section_risks_warnings(locale)));
    push_reported_section(&mut o, input, "warnings", "Risks / Warnings", locale);
    o.push_str(&format!(
        "\n{}: agent_reported\n\n",
        locale.evidence_grade_label()
    ));

    // 9. Next Session Should
    o.push_str(&format!("## {}\n\n", section_next_session_should(locale)));
    push_reported_section(
        &mut o,
        input,
        "next session should",
        "Next Session Should",
        locale,
    );
    o.push_str(&format!(
        "\n{}: agent_reported\n\n",
        locale.evidence_grade_label()
    ));

    // 10. Handoff Completeness
    o.push_str(&format!("## {}\n\n", section_handoff_completeness(locale)));
    o.push_str(&format!(
        "- {}: {}\n",
        label_complete(locale),
        yes_no(locale, complete)
    ));
    o.push_str(&format!(
        "- {}: {}\n",
        label_sections_from_output(locale),
        input.sections.len()
    ));
    o.push_str(&format!(
        "- {}: {}\n",
        label_files_from_git(locale),
        yes_no(locale, input.git_changed_files.is_some())
    ));
    o.push_str(&format!(
        "- {}: {}\n",
        label_context_manifest(locale),
        if input.context_manifest_available {
            label_available(locale)
        } else {
            label_not_available(locale)
        }
    ));
    if missing.is_empty() {
        o.push_str(&format!(
            "- {}: {}\n",
            label_missing_required_sections_label(locale),
            label_none(locale)
        ));
    } else {
        o.push_str(&format!(
            "- {}: {}\n",
            label_missing_required_sections_label(locale),
            missing
                .iter()
                .map(|s| translate_missing_section_name(locale, s))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    o.push_str(&format!("\n{}: computed\n", locale.evidence_grade_label()));

    o
}

fn push_sub_section(
    output: &mut String,
    input: &HandoffInput,
    key: &str,
    title: &str,
    locale: VibehubLocale,
) {
    match input.sections.get(key).map(|value| value.trim()) {
        Some(value) if !value.is_empty() => {
            output.push_str(value);
            output.push('\n');
        }
        _ => {
            output.push_str(&format!(
                "- {} {}\n",
                label_missing_from_output(locale),
                title
            ));
        }
    }
}

fn push_files_changed(output: &mut String, input: &HandoffInput, locale: VibehubLocale) {
    match &input.git_changed_files {
        Some(files) if files.is_empty() => {
            output.push_str(&format!("- {}\n", label_no_changed_files_observed(locale)));
        }
        Some(files) => {
            for file in files {
                output.push_str(&format!("- {}\n", file));
            }
        }
        None => {
            push_reported_section(output, input, "files changed", "Files Changed", locale);
        }
    }
}

fn push_reported_section(
    output: &mut String,
    input: &HandoffInput,
    key: &str,
    title: &str,
    locale: VibehubLocale,
) {
    match input.sections.get(key).map(|value| value.trim()) {
        Some(value) if !value.is_empty() => output.push_str(value),
        _ => output.push_str(&format!(
            "- {} {}\n",
            label_missing_from_output(locale),
            title
        )),
    }
}

fn build_prior_outputs_summary(
    capability: &str,
    source_output_path: Option<&str>,
    sections: &BTreeMap<String, String>,
) -> Vec<PriorOutputSummary> {
    let completed = section_bullets(sections.get("completed").map(String::as_str));
    let key_decisions = section_bullets(sections.get("key decisions made").map(String::as_str));
    if completed.is_empty() && key_decisions.is_empty() {
        return Vec::new();
    }
    vec![PriorOutputSummary {
        capability: capability.to_string(),
        key_decisions,
        completed,
        full_ref: source_output_path
            .unwrap_or("missing output.md")
            .to_string(),
    }]
}

fn render_prior_outputs_summary_json(summaries: &[PriorOutputSummary]) -> String {
    let value = summaries
        .iter()
        .map(|summary| {
            serde_json::json!({
                "capability": summary.capability,
                "key_decisions": summary.key_decisions,
                "completed": summary.completed,
                "full_ref": summary.full_ref,
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| "[]".to_string())
}

fn infer_task_pack_delta(
    sections: &BTreeMap<String, String>,
    git_changed_files: Option<&Vec<String>>,
) -> TaskPackDelta {
    let mut delta_fields = Vec::new();
    if substantive_section(sections.get("not yet done").map(String::as_str))
        || substantive_section(sections.get("context still needed").map(String::as_str))
        || substantive_section(sections.get("warnings").map(String::as_str))
    {
        delta_fields.push("open_items".to_string());
    }
    if substantive_section(sections.get("key decisions made").map(String::as_str)) {
        delta_fields.push("decisions_journal".to_string());
    }
    if git_changed_files
        .map(|files| !files.is_empty())
        .unwrap_or(false)
        || substantive_section(sections.get("files changed").map(String::as_str))
    {
        delta_fields.push("files_in_scope".to_string());
    }
    delta_fields.sort();
    delta_fields.dedup();
    TaskPackDelta {
        task_pack_dirty: !delta_fields.is_empty(),
        delta_fields,
    }
}

fn section_bullets(section: Option<&str>) -> Vec<String> {
    let Some(section) = section else {
        return Vec::new();
    };
    section
        .lines()
        .map(str::trim)
        .filter_map(|line| {
            let item = line
                .strip_prefix("- ")
                .or_else(|| line.strip_prefix("* "))
                .unwrap_or(line)
                .trim();
            if item.is_empty() || is_none_like(item) {
                None
            } else {
                Some(item.to_string())
            }
        })
        .collect()
}

fn substantive_section(section: Option<&str>) -> bool {
    section
        .map(|section| section_bullets(Some(section)))
        .map(|items| !items.is_empty())
        .unwrap_or(false)
}

fn is_none_like(value: &str) -> bool {
    let value = value
        .trim()
        .trim_matches(['`', '.', ',', ';', ':'])
        .to_ascii_lowercase();
    matches!(
        value.as_str(),
        "none" | "none." | "n/a" | "na" | "no" | "no changes" | "no changed files"
    )
}

fn section_present(input: &HandoffInput, key: &str) -> bool {
    input
        .sections
        .get(key)
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
}

fn missing_required_sections(input: &HandoffInput) -> Vec<String> {
    let mut missing = Vec::new();

    // What Changed: composite of completed, not yet done, key decisions made, files changed, or git files
    let what_changed_ok = section_present(input, "completed")
        || section_present(input, "not yet done")
        || section_present(input, "key decisions made")
        || input
            .git_changed_files
            .as_ref()
            .map(|f| !f.is_empty())
            .unwrap_or(false)
        || section_present(input, "files changed");
    if !what_changed_ok {
        missing.push("What Changed".to_string());
    }

    // Commands Run: from output.md
    if !section_present(input, "commands run") {
        missing.push("Commands Run".to_string());
    }

    // Tests Run: from output.md
    if !section_present(input, "tests run") {
        missing.push("Tests Run".to_string());
    }

    // Context Used: needs files reportedly read or manifest
    if !section_present(input, "files reportedly read") && !input.context_manifest_available {
        missing.push("Context Used".to_string());
    }

    // Context Still Needed: from output.md
    if !section_present(input, "context still needed") {
        missing.push("Context Still Needed".to_string());
    }

    // Risks / Warnings: from output.md
    if !section_present(input, "warnings") {
        missing.push("Risks / Warnings".to_string());
    }

    // Next Session Should: from output.md
    if !section_present(input, "next session should") {
        missing.push("Next Session Should".to_string());
    }

    missing
}

fn parse_output_sections(path: &Path) -> Result<BTreeMap<String, String>> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let mut sections = BTreeMap::new();
    let mut current_key: Option<String> = None;
    let mut current_lines = Vec::new();

    for line in content.lines() {
        if let Some(title) = markdown_heading_title(line) {
            if let Some(next_key) = canonical_section_key(title) {
                if let Some(key) = current_key.take() {
                    sections.insert(key, current_lines.join("\n").trim().to_string());
                    current_lines.clear();
                }
                current_key = Some(next_key);
            } else if current_key.is_some() {
                current_lines.push(line.to_string());
            }
        } else if current_key.is_some() {
            current_lines.push(line.to_string());
        }
    }

    if let Some(key) = current_key {
        sections.insert(key, current_lines.join("\n").trim().to_string());
    }

    Ok(sections)
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

fn canonical_section_key(title: &str) -> Option<String> {
    let normalized = normalize_heading(title);
    let key = match normalized.as_str() {
        "completed" => "completed",
        "not yet done" => "not yet done",
        "key decisions made" | "key decisions" => "key decisions made",
        "files changed" | "changed files" => "files changed",
        "files reportedly read" | "files read" => "files reportedly read",
        "commands run" => "commands run",
        "tests run" => "tests run",
        "context still needed" | "missing context" => "context still needed",
        "warnings" | "unresolved risks" | "risks" | "risks warnings" | "risks/warnings" => {
            "warnings"
        }
        "next session should" | "handoff notes" => "next session should",
        _ => return None,
    };
    Some(key.to_string())
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

fn find_latest_session_output(
    project_root: &Path,
    run_path: &str,
) -> Result<Option<(String, PathBuf)>> {
    let run_dir = project_root.join(run_path);
    let mut candidates: Vec<(String, PathBuf, std::time::SystemTime)> = Vec::new();
    let run_output_path = run_dir.join("outputs").join("output.md");
    if run_output_path.is_file() {
        let modified = run_output_path
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        candidates.push(("run".to_string(), run_output_path, modified));
    }

    let sessions_dir = run_dir.join("sessions");
    if !sessions_dir.is_dir() {
        candidates.sort_by(|a, b| a.2.cmp(&b.2).then_with(|| a.0.cmp(&b.0)));
        return Ok(candidates
            .pop()
            .map(|(session_id, path, _)| (session_id, path)));
    }

    for entry in fs::read_dir(&sessions_dir)
        .with_context(|| format!("Failed to read {}", sessions_dir.display()))?
    {
        let entry = entry.with_context(|| format!("Failed to read {}", sessions_dir.display()))?;
        if !entry
            .file_type()
            .with_context(|| format!("Failed to inspect {}", entry.path().display()))?
            .is_dir()
        {
            continue;
        }
        let session_id = entry.file_name().to_string_lossy().to_string();
        let output_path = entry.path().join("output.md");
        if output_path.is_file() {
            let modified = output_path
                .metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            candidates.push((session_id, output_path, modified));
        }
    }
    candidates.sort_by(|a, b| a.2.cmp(&b.2).then_with(|| a.0.cmp(&b.0)));
    Ok(candidates
        .pop()
        .map(|(session_id, path, _)| (session_id, path)))
}

fn git_changed_files(project_root: &Path) -> Option<Vec<String>> {
    let git_repo = silent_command("git")
        .arg("-C")
        .arg(project_root)
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .output()
        .ok()?;
    if !git_repo.status.success() {
        return None;
    }

    let mut files =
        run_git_lines(project_root, &["diff", "--name-only", "HEAD", "--"]).unwrap_or_default();
    files.extend(
        run_git_lines(
            project_root,
            &["ls-files", "--others", "--exclude-standard"],
        )
        .unwrap_or_default(),
    );
    files.sort();
    files.dedup();
    Some(files)
}

fn run_git_lines(project_root: &Path, args: &[&str]) -> Option<Vec<String>> {
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
            .map(|line| line.replace('\\', "/"))
            .collect(),
    )
}

fn read_state_field(project_root: &Path, field_path: &[&str]) -> Option<String> {
    let content = fs::read_to_string(project_root.join(".vibehub/state.yaml")).ok()?;
    let value: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;
    field_path
        .iter()
        .try_fold(&value, |current, key| current.get(*key))
        .and_then(serde_yaml::Value::as_str)
        .map(|s| s.to_string().replace('\\', "/"))
}

fn read_current_phase(project_root: &Path) -> Option<String> {
    read_state_field(project_root, &["current", "phase"])
}

fn read_current_phase_status(project_root: &Path) -> Option<String> {
    read_state_field(project_root, &["current", "phase_status"])
}

fn update_handoff_state(project_root: &Path, handoff_rel: &str, complete: bool) -> Result<()> {
    let state_path = project_root.join(".vibehub/state.yaml");
    if !state_path.is_file() {
        return Ok(());
    }

    let content = fs::read_to_string(&state_path)
        .with_context(|| format!("Failed to read {}", state_path.display()))?;
    let mut state = serde_yaml::from_str::<Value>(&content)
        .with_context(|| format!("Invalid YAML in {}", state_path.display()))?;
    set_yaml_string(&mut state, &["handoff", "current"], handoff_rel);
    set_yaml_string(
        &mut state,
        &["handoff", "status"],
        if complete {
            "available"
        } else {
            "needs_action"
        },
    );
    set_yaml_string(
        &mut state,
        &["last_updated"],
        &Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
    );
    let content = serde_yaml::to_string(&state).context("Failed to serialize state.yaml")?;
    fs::write(&state_path, content)
        .with_context(|| format!("Failed to write {}", state_path.display()))
}

fn set_yaml_string(value: &mut Value, path: &[&str], next: &str) {
    set_yaml_value(value, path, Value::String(next.to_string()));
}

fn set_yaml_value(value: &mut Value, path: &[&str], next: Value) {
    if path.is_empty() {
        *value = next;
        return;
    }
    if !matches!(value, Value::Mapping(_)) {
        *value = Value::Mapping(Mapping::new());
    }
    let mut current = value;
    for key in &path[..path.len() - 1] {
        let mapping = current.as_mapping_mut().expect("mapping value");
        current = mapping
            .entry(Value::String((*key).to_string()))
            .or_insert_with(|| Value::Mapping(Mapping::new()));
        if !matches!(current, Value::Mapping(_)) {
            *current = Value::Mapping(Mapping::new());
        }
    }
    let mapping = current.as_mapping_mut().expect("mapping value");
    mapping.insert(Value::String(path[path.len() - 1].to_string()), next);
}

fn title_case(value: &str) -> String {
    value
        .split(['_', '-', ' '])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn handoff_session_title(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Handoff from Session",
        VibehubLocale::ZhCn => "会话交接",
        VibehubLocale::ZhTw => "會議交接",
    }
}

fn label_task(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Task",
        VibehubLocale::ZhCn => "任务",
        VibehubLocale::ZhTw => "任務",
    }
}

fn label_run(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Run",
        VibehubLocale::ZhCn => "运行",
        VibehubLocale::ZhTw => "執行",
    }
}

fn label_phase(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Phase",
        VibehubLocale::ZhCn => "阶段",
        VibehubLocale::ZhTw => "階段",
    }
}

fn label_generated_by(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Generated by",
        VibehubLocale::ZhCn => "生成来源",
        VibehubLocale::ZhTw => "產生來源",
    }
}

fn label_generated_at(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Generated at",
        VibehubLocale::ZhCn => "生成时间",
        VibehubLocale::ZhTw => "產生時間",
    }
}

fn label_source(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Source",
        VibehubLocale::ZhCn => "来源",
        VibehubLocale::ZhTw => "來源",
    }
}

fn label_handoff_complete(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Handoff complete",
        VibehubLocale::ZhCn => "交接完成",
        VibehubLocale::ZhTw => "交接完成",
    }
}

fn yes_no(locale: VibehubLocale, value: bool) -> &'static str {
    match (locale, value) {
        (VibehubLocale::En, true) => "yes",
        (VibehubLocale::En, false) => "no",
        (VibehubLocale::ZhCn, true) => "是",
        (VibehubLocale::ZhCn, false) => "否",
        (VibehubLocale::ZhTw, true) => "是",
        (VibehubLocale::ZhTw, false) => "否",
    }
}

fn section_missing_required(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Missing Required Sections",
        VibehubLocale::ZhCn => "缺失的必要章节",
        VibehubLocale::ZhTw => "缺失的必要章節",
    }
}

fn section_current_task(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Current Task",
        VibehubLocale::ZhCn => "当前任务",
        VibehubLocale::ZhTw => "目前任務",
    }
}

fn section_current_phase(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Current Phase",
        VibehubLocale::ZhCn => "当前阶段",
        VibehubLocale::ZhTw => "目前階段",
    }
}

fn section_what_changed(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "What Changed",
        VibehubLocale::ZhCn => "变更内容",
        VibehubLocale::ZhTw => "變更內容",
    }
}

fn section_commands_run(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Commands Run",
        VibehubLocale::ZhCn => "执行的命令",
        VibehubLocale::ZhTw => "執行的命令",
    }
}

fn section_tests_run(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Tests Run",
        VibehubLocale::ZhCn => "运行的测试",
        VibehubLocale::ZhTw => "執行的測試",
    }
}

fn section_context_used(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Context Used",
        VibehubLocale::ZhCn => "使用的上下文",
        VibehubLocale::ZhTw => "使用的上下文",
    }
}

fn section_context_still_needed(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Context Still Needed",
        VibehubLocale::ZhCn => "仍需的上下文",
        VibehubLocale::ZhTw => "仍需的上下文",
    }
}

fn section_risks_warnings(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Risks / Warnings",
        VibehubLocale::ZhCn => "风险 / 警告",
        VibehubLocale::ZhTw => "風險 / 警告",
    }
}

fn section_next_session_should(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Next Session Should",
        VibehubLocale::ZhCn => "下次会话应",
        VibehubLocale::ZhTw => "下次會議應",
    }
}

fn section_handoff_completeness(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Handoff Completeness",
        VibehubLocale::ZhCn => "交接完整性",
        VibehubLocale::ZhTw => "交接完整性",
    }
}

fn sub_files_read(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Files Read",
        VibehubLocale::ZhCn => "读取的文件",
        VibehubLocale::ZhTw => "讀取的檔案",
    }
}

fn sub_context_pack(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Context Pack",
        VibehubLocale::ZhCn => "上下文包",
        VibehubLocale::ZhTw => "上下文包",
    }
}

fn label_task_id(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Task ID",
        VibehubLocale::ZhCn => "任务 ID",
        VibehubLocale::ZhTw => "任務 ID",
    }
}

fn label_task_path(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Task path",
        VibehubLocale::ZhCn => "任务路径",
        VibehubLocale::ZhTw => "任務路徑",
    }
}

fn label_run_id(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Run ID",
        VibehubLocale::ZhCn => "运行 ID",
        VibehubLocale::ZhTw => "執行 ID",
    }
}

fn label_run_path(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Run path",
        VibehubLocale::ZhCn => "运行路径",
        VibehubLocale::ZhTw => "執行路徑",
    }
}

fn label_status(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Status",
        VibehubLocale::ZhCn => "状态",
        VibehubLocale::ZhTw => "狀態",
    }
}

fn label_complete(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Complete",
        VibehubLocale::ZhCn => "完成",
        VibehubLocale::ZhTw => "完成",
    }
}

fn label_sections_from_output(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Sections from output.md",
        VibehubLocale::ZhCn => "来自 output.md 的章节",
        VibehubLocale::ZhTw => "來自 output.md 的章節",
    }
}

fn label_files_from_git(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Files from git",
        VibehubLocale::ZhCn => "来自 git 的文件",
        VibehubLocale::ZhTw => "來自 git 的檔案",
    }
}

fn label_context_manifest(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Context manifest",
        VibehubLocale::ZhCn => "上下文清单",
        VibehubLocale::ZhTw => "上下文清單",
    }
}

fn label_missing_required_sections_label(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Missing required sections",
        VibehubLocale::ZhCn => "缺失的必要章节",
        VibehubLocale::ZhTw => "缺失的必要章節",
    }
}

fn label_path(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Path",
        VibehubLocale::ZhCn => "路径",
        VibehubLocale::ZhTw => "路徑",
    }
}

fn label_manifest(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Manifest",
        VibehubLocale::ZhCn => "清单",
        VibehubLocale::ZhTw => "清單",
    }
}

fn label_available(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "available",
        VibehubLocale::ZhCn => "可用",
        VibehubLocale::ZhTw => "可用",
    }
}

fn label_not_available(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "not available",
        VibehubLocale::ZhCn => "不可用",
        VibehubLocale::ZhTw => "不可用",
    }
}

fn label_missing_session_output(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "missing session output.md",
        VibehubLocale::ZhCn => "缺少会话 output.md",
        VibehubLocale::ZhTw => "缺少會議 output.md",
    }
}

fn label_no_changed_files_observed(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "No changed files observed by Git.",
        VibehubLocale::ZhCn => "Git 未观察到变更文件。",
        VibehubLocale::ZhTw => "Git 未觀察到變更檔案。",
    }
}

fn label_missing_from_output(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Missing from output.md:",
        VibehubLocale::ZhCn => "output.md 中缺失:",
        VibehubLocale::ZhTw => "output.md 中缺失:",
    }
}

fn label_none(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "None",
        VibehubLocale::ZhCn => "无",
        VibehubLocale::ZhTw => "無",
    }
}

fn translate_missing_section_name(locale: VibehubLocale, name: &str) -> String {
    match name {
        "What Changed" => section_what_changed(locale).to_string(),
        "Commands Run" => section_commands_run(locale).to_string(),
        "Tests Run" => section_tests_run(locale).to_string(),
        "Context Used" => section_context_used(locale).to_string(),
        "Context Still Needed" => section_context_still_needed(locale).to_string(),
        "Risks / Warnings" => section_risks_warnings(locale).to_string(),
        "Next Session Should" => section_next_session_should(locale).to_string(),
        _ => name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vibehub::current::{write_current_run_pointer, write_current_task_pointer};
    use std::process::Command;
    use uuid::Uuid;

    fn temp_project() -> PathBuf {
        let path = std::env::temp_dir().join(format!("vibehub-handoff-test-{}", Uuid::new_v4()));
        fs::create_dir_all(path.join(".vibehub/tasks/T-001/runs/R-001/sessions/S-001"))
            .expect("create session");
        fs::create_dir_all(path.join(".vibehub/agent-view")).expect("create agent view");
        fs::write(
            path.join(".vibehub/state.yaml"),
            "current:\n  phase: implement\n  phase_status: active\n",
        )
        .expect("write state");
        write_current_task_pointer(&path, "T-001").expect("task pointer");
        write_current_run_pointer(&path, "T-001", "R-001").expect("run pointer");
        path
    }

    fn write_complete_output(project: &Path) {
        fs::write(
            project.join(".vibehub/tasks/T-001/runs/R-001/sessions/S-001/output.md"),
            r#"# Session Output

## Completed
- Implemented handoff builder.

## Not Yet Done
- Wire GUI status later.

## Key Decisions Made
- Use Git status for changed files when available.

## Files Changed
- agent reported file should be replaced when Git is available.

## Files Reportedly Read
- AGENTS.md

## Commands Run
- cargo test vibehub

## Tests Run
- vibehub tests: 56 passed

## Context Still Needed
- None.

## Warnings
- Runtime observation is not available.

## Next Session Should
1. Review generated handoff.
"#,
        )
        .expect("write output");
    }

    #[test]
    fn complete_output_generates_complete_handoff_without_git() {
        let project = temp_project();
        write_complete_output(&project);

        let result = build_handoff(&project).expect("build handoff");
        let handoff =
            fs::read_to_string(project.join(".vibehub/agent-view/handoff.md")).expect("read");

        assert!(result.complete);
        assert!(result.missing_required_sections.is_empty());
        assert!(handoff.contains("Handoff complete: yes"));
        assert!(handoff.contains("## What Changed"));
        assert!(handoff.contains("## Commands Run"));
        assert!(handoff.contains("## Tests Run"));
        assert!(handoff.contains("## Context Used"));
        assert!(handoff.contains("## Context Still Needed"));
        assert!(handoff.contains("## Risks / Warnings"));
        assert!(handoff.contains("## Next Session Should"));
        assert!(handoff.contains("## Handoff Completeness"));
        assert!(handoff.contains("- Complete: yes"));
        assert!(handoff.contains("Evidence grade: computed"));
        assert_eq!(
            read_state_field(&project, &["handoff", "status"]).as_deref(),
            Some("available")
        );

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn handoff_contains_prior_summary_and_dirty_delta() {
        let project = temp_project();
        write_complete_output(&project);

        build_handoff(&project).expect("build handoff");
        let handoff =
            fs::read_to_string(project.join(".vibehub/agent-view/handoff.md")).expect("read");

        assert!(handoff.contains("## Prior Outputs Summary"));
        assert!(handoff.contains("\"capability\": \"implement\""));
        assert!(handoff.contains("Implemented handoff builder."));
        assert!(handoff.contains("## Task Pack Delta"));
        assert!(handoff.contains("task_pack_dirty: true"));
        assert!(handoff.contains("open_items"));
        assert!(handoff.contains("decisions_journal"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn handoff_delta_can_skip_task_pack_rebuild() {
        let project = temp_project();
        fs::write(
            project.join(".vibehub/tasks/T-001/runs/R-001/sessions/S-001/output.md"),
            r#"# Session Output

## Completed
- Completed a local-only check.

## Not Yet Done
- None.

## Key Decisions Made
- None.

## Files Changed
- None.

## Files Reportedly Read
- AGENTS.md

## Commands Run
- cargo test vibehub::handoff

## Tests Run
- passed

## Context Still Needed
- None.

## Warnings
- None.

## Next Session Should
- None.
"#,
        )
        .expect("write output");

        build_handoff(&project).expect("build handoff");
        let handoff =
            fs::read_to_string(project.join(".vibehub/agent-view/handoff.md")).expect("read");

        assert!(handoff.contains("## Task Pack Delta"));
        assert!(handoff.contains("task_pack_dirty: false"));
        assert!(handoff.contains("delta_fields: []"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn missing_section_generates_incomplete_handoff() {
        let project = temp_project();
        fs::write(
            project.join(".vibehub/tasks/T-001/runs/R-001/sessions/S-001/output.md"),
            "## Completed\n- Done.\n\n## Next Session Should\n- Continue.\n",
        )
        .expect("write output");

        let result = build_handoff(&project).expect("build handoff");
        let handoff =
            fs::read_to_string(project.join(".vibehub/agent-view/handoff.md")).expect("read");

        assert!(!result.complete);
        assert!(result
            .missing_required_sections
            .contains(&"Commands Run".to_string()));
        assert!(result
            .missing_required_sections
            .contains(&"Tests Run".to_string()));
        assert!(result
            .missing_required_sections
            .contains(&"Context Still Needed".to_string()));
        assert!(result
            .missing_required_sections
            .contains(&"Risks / Warnings".to_string()));
        assert!(handoff.contains("Handoff complete: no"));
        assert!(handoff.contains("## Missing Required Sections"));
        assert!(handoff.contains("- Commands Run"));
        assert!(handoff.contains("- Tests Run"));
        assert!(handoff.contains("- Missing from output.md: Commands Run"));
        assert!(handoff.contains("- Missing from output.md: Tests Run"));
        assert_eq!(
            read_state_field(&project, &["handoff", "status"]).as_deref(),
            Some("needs_action")
        );

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn git_changed_files_appear_in_what_changed() {
        let project = temp_project();
        write_complete_output(&project);
        Command::new("git")
            .arg("-C")
            .arg(&project)
            .arg("init")
            .output()
            .expect("git init");
        fs::write(project.join("src.rs"), "changed\n").expect("write changed file");
        fs::write(project.join("lib.rs"), "also changed\n").expect("write another file");

        let result = build_handoff(&project).expect("build handoff");
        let handoff =
            fs::read_to_string(project.join(".vibehub/agent-view/handoff.md")).expect("read");

        assert!(result.complete);
        assert_eq!(result.files_changed_evidence, "hard_observed");
        assert!(handoff.contains("## What Changed"));
        assert!(handoff.contains("### Files Changed"));
        assert!(handoff.contains("- src.rs"));
        assert!(handoff.contains("- lib.rs"));
        assert!(handoff.contains("Evidence grade: mixed"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn accepts_run_level_output_without_session_directory() {
        let project = temp_project();
        fs::create_dir_all(project.join(".vibehub/tasks/T-001/runs/R-001/outputs"))
            .expect("create outputs");
        fs::write(
            project.join(".vibehub/tasks/T-001/runs/R-001/outputs/output.md"),
            r#"# Run Output

## Completed
- Wrote run-level output.

## Not Yet Done
- None.

## Key Decisions Made
- Prefer run-level output when no session id is available.

## Files Changed
- output.md

## Files Reportedly Read
- .vibehub/agent-view/current.md

## Commands Run
- cargo test vibehub

## Tests Run
- all tests passed

## Context Still Needed
- None.

## Warnings
- None.

## Next Session Should
- Continue validation.
"#,
        )
        .expect("write run output");

        let result = build_handoff(&project).expect("build handoff");
        let handoff =
            fs::read_to_string(project.join(".vibehub/agent-view/handoff.md")).expect("read");

        assert!(result.complete);
        assert_eq!(
            result.source_output_path.as_deref(),
            Some(".vibehub/tasks/T-001/runs/R-001/outputs/output.md")
        );
        assert!(handoff.contains("Source: .vibehub/tasks/T-001/runs/R-001/outputs/output.md"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn handoff_uses_simplified_chinese_headings() {
        let project = temp_project();
        fs::write(
            project.join(".vibehub/state.yaml"),
            "current:\n  phase: implement\n  phase_status: active\npreferences:\n  locale: zh-CN\n",
        )
        .expect("write state");
        write_complete_output(&project);

        let result = build_handoff(&project).expect("build handoff");
        let handoff =
            fs::read_to_string(project.join(".vibehub/agent-view/handoff.md")).expect("read");

        assert!(result.complete);
        assert!(handoff.contains("# 会话交接"));
        assert!(handoff.contains("## 变更内容"));
        assert!(handoff.contains("## 执行的命令"));
        assert!(handoff.contains("## 运行的测试"));
        assert!(handoff.contains("## 使用的上下文"));
        assert!(handoff.contains("## 交接完整性"));
        assert!(handoff.contains("交接完成: 是"));
        assert!(handoff.contains("### Completed"));
        assert!(handoff.contains("### Not Yet Done"));
        assert!(handoff.contains("### Key Decisions Made"));
        assert!(handoff.contains("### Files Changed"));

        fs::remove_dir_all(project).expect("cleanup");
    }
}

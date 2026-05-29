use crate::process_util::silent_command;
use crate::vibehub::locale::VibehubLocale;
use crate::vibehub::util::{
    canonical_initialized_project_root, normalize_path, relative_to_project,
};
use crate::vibehub::{current, events};
use anyhow::{anyhow, Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ReviewEvidenceGenerateResult {
    pub task_id: String,
    pub run_id: String,
    pub review_path: String,
    pub changed_files_path: String,
    pub diff_path: String,
    pub changed_files_count: usize,
    pub baseline_ref: Option<String>,
    pub source_output_path: Option<String>,
}

#[derive(Debug, Clone)]
struct ReviewInput {
    task_id: String,
    run_id: String,
    run_path: String,
    baseline_ref: Option<String>,
    head_ref: Option<String>,
    generated_at: String,
    changed_files: Vec<String>,
    diff_stat: Vec<String>,
    source_output_path: Option<String>,
    sections: BTreeMap<String, String>,
    git_available: bool,
    context_manifest: Option<ContextManifestRaw>,
    context_manifest_path: Option<String>,
    research_available: bool,
    research_path: Option<String>,
    research_source_log_exists: bool,
    research_findings_exists: bool,
    locale: VibehubLocale,
}

#[derive(Debug, Clone, Deserialize)]
struct ContextManifestRaw {
    id: String,
    #[serde(default)]
    phase: String,
    #[serde(default, rename = "source_commit")]
    source_commit: Option<String>,
    budget: BudgetRaw,
    #[serde(default)]
    included: Vec<ManifestEntryRaw>,
    #[serde(default)]
    missing: Vec<ManifestEntryRaw>,
    #[serde(default)]
    excluded: Vec<ManifestEntryRaw>,
    quality: QualityRaw,
    observation: ObservationRaw,
}

#[derive(Debug, Clone, Deserialize)]
struct BudgetRaw {
    #[serde(default, rename = "max_file_size_bytes")]
    max_file_size_bytes: u64,
    #[serde(default, rename = "max_tokens")]
    max_tokens: usize,
    #[serde(default, rename = "estimated_tokens")]
    estimated_tokens: usize,
}

#[derive(Debug, Clone, Deserialize)]
struct ManifestEntryRaw {
    path: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    confidence: String,
    #[serde(default)]
    bytes: u64,
    #[serde(default, rename = "estimated_tokens")]
    estimated_tokens: usize,
    #[serde(default)]
    policy: String,
}

#[derive(Debug, Clone, Deserialize)]
struct QualityRaw {
    #[serde(default, rename = "has_goal")]
    has_goal: bool,
    #[serde(default, rename = "has_plan")]
    has_plan: bool,
    #[serde(default, rename = "has_relevant_code")]
    has_relevant_code: bool,
    #[serde(default, rename = "has_test_hint")]
    has_test_hint: bool,
    #[serde(default, rename = "stale_sources")]
    #[allow(dead_code)]
    stale_sources: Vec<String>,
    #[serde(default)]
    flags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ObservationRaw {
    #[serde(default, rename = "evidence_grade")]
    evidence_grade: String,
}

#[allow(dead_code)]
pub fn generate_review_evidence(
    project_root: impl AsRef<Path>,
) -> Result<ReviewEvidenceGenerateResult> {
    generate_review_evidence_with_locale(project_root, None)
}

pub fn generate_review_evidence_with_locale(
    project_root: impl AsRef<Path>,
    locale_override: Option<&str>,
) -> Result<ReviewEvidenceGenerateResult> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let locale = VibehubLocale::detect_with_override(&project_root, locale_override);
    let task_pointer = current::resolve_current_task(&project_root)?;
    let run_pointer = current::resolve_current_run(&project_root, &task_pointer.task_id)?;
    let run_dir = project_root.join(&run_pointer.path);
    if !run_dir.is_dir() {
        return Err(anyhow!(
            "Run directory does not exist: {}",
            run_dir.display()
        ));
    }

    let output = find_latest_session_output(&project_root, &run_pointer.path)?;
    let sections = match &output {
        Some((_, path)) => parse_output_sections(path)?,
        None => BTreeMap::new(),
    };
    let source_output_path = output
        .as_ref()
        .map(|(_, path)| relative_to_project(&project_root, path).map(|path| normalize_path(&path)))
        .transpose()?;

    let git_available = is_git_repo(&project_root);
    let baseline_ref = if git_available {
        discover_baseline_ref(&project_root, &run_dir)
    } else {
        None
    };
    let head_ref = if git_available {
        git_stdout(&project_root, &["rev-parse", "--short", "HEAD"])
    } else {
        None
    };
    let changed_files = if git_available {
        git_changed_files(&project_root, baseline_ref.as_deref())?
    } else {
        Vec::new()
    };
    let diff_patch = if git_available {
        build_diff_patch(&project_root, baseline_ref.as_deref())?
    } else {
        "Git repository not available; diff.patch could not be generated.\n".to_string()
    };
    let diff_stat = if git_available {
        git_diff_stat(&project_root, baseline_ref.as_deref()).unwrap_or_default()
    } else {
        Vec::new()
    };

    let (context_manifest, context_manifest_path) =
        read_context_manifest(&project_root, &run_pointer.path);
    let (research_available, research_path, source_log_exists, findings_exists) =
        read_research_pack_info(&project_root);

    let evidence_dir = run_dir.join("evidence");
    let phases_dir = run_dir.join("phases");
    fs::create_dir_all(&evidence_dir)
        .with_context(|| format!("Failed to create {}", evidence_dir.display()))?;
    fs::create_dir_all(&phases_dir)
        .with_context(|| format!("Failed to create {}", phases_dir.display()))?;

    let changed_files_path = evidence_dir.join("changed-files.txt");
    let diff_path = evidence_dir.join("diff.patch");
    let review_path = phases_dir.join("review.md");

    fs::write(&changed_files_path, render_changed_files(&changed_files))
        .with_context(|| format!("Failed to write {}", changed_files_path.display()))?;
    fs::write(&diff_path, diff_patch)
        .with_context(|| format!("Failed to write {}", diff_path.display()))?;

    let input = ReviewInput {
        task_id: task_pointer.task_id,
        run_id: run_pointer.run_id,
        run_path: run_pointer.path,
        baseline_ref,
        head_ref,
        generated_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        changed_files,
        diff_stat,
        source_output_path,
        sections,
        git_available,
        context_manifest,
        context_manifest_path,
        research_available,
        research_path,
        research_source_log_exists: source_log_exists,
        research_findings_exists: findings_exists,
        locale,
    };
    let (verdict, _) = determine_verdict(&input);
    fs::write(&review_path, render_review(&input))
        .with_context(|| format!("Failed to write {}", review_path.display()))?;
    let review_rel = normalize_path(&relative_to_project(&project_root, &review_path)?);
    let changed_rel = normalize_path(&relative_to_project(&project_root, &changed_files_path)?);
    let diff_rel = normalize_path(&relative_to_project(&project_root, &diff_path)?);
    let legacy_event = events::append_run_event(
        &project_root,
        &input.task_id,
        &input.run_id,
        "review_evidence_generated",
        "Review evidence artifacts generated.",
        serde_json::json!({
            "review_path": review_rel.clone(),
            "changed_files_path": changed_rel.clone(),
            "diff_path": diff_rel.clone(),
            "changed_files_count": input.changed_files.len(),
            "baseline_ref": input.baseline_ref.clone(),
            "source_output_path": input.source_output_path.clone(),
        }),
    )
    .ok();
    let mut refs = vec![review_rel.clone(), changed_rel.clone(), diff_rel.clone()];
    if let Some(event) = legacy_event {
        refs.insert(0, event.event_id);
    }
    let _ = events::append_structured_run_event(
        &project_root,
        &input.task_id,
        &input.run_id,
        events::VibehubEvent::EvidenceAdded {
            source: "review".to_string(),
            summary: "Review evidence artifacts generated.".to_string(),
            refs,
        },
    );
    let _ = events::append_structured_run_event(
        &project_root,
        &input.task_id,
        &input.run_id,
        events::VibehubEvent::DiffObserved {
            commit_range: input
                .baseline_ref
                .as_deref()
                .map(|baseline| format!("{baseline}..HEAD"))
                .unwrap_or_else(|| "HEAD".to_string()),
            files: input.changed_files.clone(),
        },
    );
    let _ = append_review_failure_release_if_needed(&project_root, &input, verdict);

    Ok(ReviewEvidenceGenerateResult {
        task_id: input.task_id,
        run_id: input.run_id,
        review_path: review_rel,
        changed_files_path: changed_rel,
        diff_path: diff_rel,
        changed_files_count: input.changed_files.len(),
        baseline_ref: input.baseline_ref,
        source_output_path: input.source_output_path,
    })
}

fn append_review_failure_release_if_needed(
    project_root: &Path,
    input: &ReviewInput,
    verdict: &str,
) -> Result<Option<events::RunEventAppendResult>> {
    if verdict == "ready_for_human_review" {
        return Ok(None);
    }
    if !review_capability_is_active(project_root, input)? {
        return Ok(None);
    }
    events::append_structured_run_event(
        project_root,
        &input.task_id,
        &input.run_id,
        events::VibehubEvent::CapabilityReleased {
            capability: "review".to_string(),
            outcome: "failed".to_string(),
            task_pack_dirty: true,
        },
    )
    .map(Some)
}

fn review_capability_is_active(project_root: &Path, input: &ReviewInput) -> Result<bool> {
    let events = events::list_events(project_root, &input.task_id, &input.run_id, None)?;
    let mut active_from_events = None;
    for stored in events {
        let Some(event_type) = stored.event.get("event_type").and_then(JsonValue::as_str) else {
            continue;
        };
        let payload = stored.event.get("payload").unwrap_or(&JsonValue::Null);
        let capability = payload.get("capability").and_then(JsonValue::as_str);
        if capability != Some("review") {
            continue;
        }
        match event_type {
            "CapabilityClaimed" => active_from_events = Some(true),
            "CapabilityReleased" => active_from_events = Some(false),
            _ => {}
        }
    }
    if let Some(active) = active_from_events {
        return Ok(active);
    }
    Ok(
        read_current_phase(project_root).as_deref() == Some("review")
            && read_current_phase_status(project_root)
                .as_deref()
                .map(|status| status == "active")
                .unwrap_or(true),
    )
}

fn render_review(input: &ReviewInput) -> String {
    let mut out = String::new();
    let locale = input.locale;

    let (verdict, verdict_reason) = determine_verdict(input);

    out.push_str(&format!("# {}\n\n", review_title(locale)));
    out.push_str(&format!("{}: {}\n", label_task(locale), input.task_id));
    out.push_str(&format!("{}: {}\n", label_run(locale), input.run_id));
    out.push_str(&format!("{}: {}\n", label_run_path(locale), input.run_path));
    out.push_str(&format!("{}: VibeHub\n", label_generated_by(locale)));
    out.push_str(&format!(
        "{}: {}\n",
        label_generated_at(locale),
        input.generated_at
    ));
    out.push_str(&format!(
        "{}: {}\n\n",
        label_source(locale),
        input
            .source_output_path
            .as_deref()
            .unwrap_or("missing session output.md")
    ));

    out.push_str(&format!("## {}\n\n", section_verdict(locale)));
    out.push_str(&format!("{}\n\n", verdict));
    out.push_str(&format!("{}\n\n", verdict_reason));

    out.push_str(&format!("## {}\n\n", section_evidence_map(locale)));

    out.push_str("### hard_observed\n\n");
    if input.git_available {
        out.push_str(&format!(
            "- {}: {} file(s) changed (source: `git diff --stat`)\n",
            label_git_diff_summary(locale),
            input.diff_stat.len()
        ));
        out.push_str(&format!(
            "- {}: {} file(s) (source: `git diff --name-only` + status)\n",
            label_changed_files_list(locale),
            input.changed_files.len()
        ));
    } else {
        out.push_str(&format!(
            "- {}: {}\n",
            label_git_diff_summary(locale),
            locale.unavailable_git()
        ));
        out.push_str(&format!(
            "- {}: {}\n",
            label_changed_files_list(locale),
            locale.unavailable_git()
        ));
    }
    out.push_str(&format!(
        "- {}: evidence/changed-files.txt, evidence/diff.patch, phases/review.md\n",
        label_generated_artifacts(locale)
    ));
    if let Some(ref manifest_path) = input.context_manifest_path {
        out.push_str(&format!(
            "- {}: available ({})\n",
            label_context_manifest_label(locale),
            manifest_path
        ));
    } else {
        out.push_str(&format!(
            "- {}: {}\n",
            label_context_manifest_label(locale),
            locale.not_available()
        ));
    }
    out.push('\n');

    out.push_str("### agent_reported\n\n");
    push_reported_section_map(
        &mut out,
        input,
        &[
            ("tests run", &label_tests_run(locale)),
            (
                "tests run or reason not run",
                &label_tests_run_or_reason(locale),
            ),
            ("test results", &label_test_results(locale)),
        ],
        &format!("- {}: {}\n", label_tests_run(locale), locale.not_reported()),
    );
    push_reported_section_map(
        &mut out,
        input,
        &[("commands run", &label_commands_run(locale))],
        &format!(
            "- {}: {}\n",
            label_commands_run(locale),
            locale.not_reported()
        ),
    );
    push_reported_section_map(
        &mut out,
        input,
        &[("files changed", &label_files_changed(locale))],
        &format!(
            "- {} ({}): {}\n",
            label_files_changed(locale),
            locale.reported_suffix(),
            locale.not_reported()
        ),
    );
    push_reported_section_map(
        &mut out,
        input,
        &[
            ("completed", &label_completed_summary(locale)),
            ("key decisions made", &label_key_decisions(locale)),
            ("rationale", &label_rationale(locale)),
        ],
        &format!("- {}: {}\n", label_summary(locale), locale.not_reported()),
    );
    push_reported_section_map(
        &mut out,
        input,
        &[
            ("unresolved risks", &label_unresolved_risks(locale)),
            ("warnings", &label_warnings(locale)),
        ],
        &format!("- {}: {}\n", label_risks(locale), locale.not_reported()),
    );
    if input.sections.contains_key("next session should") {
        out.push_str(&format!("- {}\n", locale.handoff_reported()));
    } else {
        out.push_str(&format!("- {}\n", locale.handoff_not_reported()));
    }
    out.push('\n');

    out.push_str("### inferred\n\n");
    out.push_str(&format!(
        "- {}: task {}, run {} (from VibeHub current pointers)\n",
        label_task_mapping(locale),
        input.task_id,
        input.run_id
    ));
    if input.context_manifest.is_some() {
        out.push_str(&format!("- {}\n", locale.context_complete_manifest()));
    } else {
        out.push_str(&format!("- {}\n", locale.context_complete_unknown()));
    }
    if input.research_available {
        out.push_str(&format!("- {}\n", locale.research_available()));
    } else {
        out.push_str(&format!("- {}\n", locale.research_not_available()));
    }
    if input.source_output_path.is_none() {
        out.push_str(&format!("- {}\n", locale.risk_no_output()));
    }
    if !input.git_available {
        out.push_str(&format!("- {}\n", locale.risk_git_unavailable()));
    }
    out.push('\n');

    if input.context_manifest.is_some() || input.context_manifest_path.is_some() {
        out.push_str(&format!("## {}\n\n", section_context_manifest(locale)));
        if let Some(ref manifest) = input.context_manifest {
            out.push_str(&format!(
                "- {}: {}\n",
                label_manifest_id(locale),
                manifest.id
            ));
            out.push_str(&format!(
                "- {}: {}\n",
                label_phase(locale),
                if manifest.phase.is_empty() {
                    "unspecified"
                } else {
                    &manifest.phase
                }
            ));
            out.push_str(&format!(
                "- {}: {}\n",
                label_source_commit(locale),
                manifest.source_commit.as_deref().unwrap_or("unavailable")
            ));
            out.push_str(&format!(
                "- {} {} / {} {} ({}: {}, {}: {})\n\n",
                label_budget(locale),
                manifest.budget.estimated_tokens,
                manifest.budget.max_tokens,
                locale.tokens_used(),
                locale.limit_label(),
                manifest.budget.max_tokens,
                locale.max_file_label(),
                manifest.budget.max_file_size_bytes,
            ));

            out.push_str(&format!("### {}\n\n", section_included_files(locale)));
            if manifest.included.is_empty() {
                out.push_str(&format!("- {}\n\n", locale.no_files_included()));
            } else {
                for entry in &manifest.included {
                    let req = if entry.required {
                        locale.required()
                    } else {
                        locale.optional()
                    };
                    out.push_str(&format!(
                        "- `{}` ({}, {}, {}, {}): {}\n",
                        entry.path,
                        entry.reason,
                        req,
                        manifest_entry_bytes(entry),
                        manifest_entry_tokens(entry),
                        entry.confidence
                    ));
                }
                out.push('\n');
            }

            out.push_str(&format!(
                "### {}\n\n",
                section_missing_required_context(locale)
            ));
            let missing_req: Vec<_> = manifest.missing.iter().filter(|m| m.required).collect();
            if missing_req.is_empty() && manifest.missing.is_empty() {
                out.push_str(&format!("- {}\n\n", locale.none()));
            } else {
                for entry in &manifest.missing {
                    let req = if entry.required {
                        locale.required()
                    } else {
                        locale.optional()
                    };
                    out.push_str(&format!(
                        "- `{}` ({}, {}, {}): {}\n",
                        entry.path,
                        req,
                        manifest_entry_bytes(entry),
                        manifest_entry_tokens(entry),
                        entry.reason
                    ));
                }
                out.push('\n');
            }

            out.push_str(&format!(
                "### {}\n\n",
                section_excluded_secret_files(locale)
            ));
            let secret_entries: Vec<_> = manifest
                .excluded
                .iter()
                .filter(|e| e.policy == "deny_secret_path")
                .collect();
            if secret_entries.is_empty() {
                out.push_str(&format!("- {}\n\n", locale.none()));
            } else {
                for entry in &secret_entries {
                    out.push_str(&format!(
                        "- `{}` ({}, {}): {}\n",
                        entry.path,
                        manifest_entry_bytes(entry),
                        manifest_entry_tokens(entry),
                        entry.reason
                    ));
                }
                out.push('\n');
            }

            out.push_str(&format!("### {}\n\n", section_quality(locale)));
            out.push_str(&format!(
                "- {}: {}\n",
                label_has_goal(locale),
                manifest.quality.has_goal
            ));
            out.push_str(&format!(
                "- {}: {}\n",
                label_has_plan(locale),
                manifest.quality.has_plan
            ));
            out.push_str(&format!(
                "- {}: {}\n",
                label_has_relevant_code(locale),
                manifest.quality.has_relevant_code
            ));
            out.push_str(&format!(
                "- {}: {}\n",
                label_has_test_hint(locale),
                manifest.quality.has_test_hint
            ));
            if manifest.quality.flags.is_empty() {
                out.push_str(&format!(
                    "- {}: {}\n",
                    label_flags(locale),
                    locale.none_lowercase()
                ));
            } else {
                out.push_str(&format!(
                    "- {}: {}\n",
                    label_flags(locale),
                    manifest.quality.flags.join(", ")
                ));
            }
            out.push_str(&format!(
                "- {}: {}\n",
                label_observation_grade(locale),
                manifest.observation.evidence_grade
            ));
            out.push('\n');
        } else {
            out.push_str(&format!(
                "{}\n\n",
                locale.manifest_parse_error(
                    input.context_manifest_path.as_deref().unwrap_or("unknown")
                )
            ));
        }
    }

    out.push_str(&format!("## {}\n\n", section_research_pack(locale)));
    if input.research_available {
        out.push_str(&format!(
            "- {}: {} `{}`\n",
            label_status(locale),
            locale.available_at(),
            input.research_path.as_deref().unwrap_or("unknown")
        ));
        out.push_str(&format!(
            "- {}: {}\n",
            label_research_evidence(locale),
            locale.yes_file_exists()
        ));
        out.push_str(&format!(
            "- {} (source-log.yaml): {}\n",
            label_source_log(locale),
            if input.research_source_log_exists {
                locale.present()
            } else {
                locale.missing_label()
            }
        ));
        out.push_str(&format!(
            "- {} (findings.yaml): {}\n",
            label_findings(locale),
            if input.research_findings_exists {
                locale.present()
            } else {
                locale.missing_label()
            }
        ));
        out.push_str(&format!("- {}\n", locale.research_not_analyzed()));
    } else {
        out.push_str(&format!(
            "- {}: {}\n",
            label_status(locale),
            locale.not_available()
        ));
        out.push_str(&format!(
            "- {}: {}\n",
            label_research_evidence(locale),
            locale.no_file_missing()
        ));
        out.push_str(&format!("- {}\n", locale.research_missing_review()));
    }
    out.push('\n');

    out.push_str(&format!("## {}\n\n", section_diff_summary(locale)));
    if input.diff_stat.is_empty() {
        out.push_str(&format!("- {}\n", locale.no_diff_summary()));
    } else {
        for line in &input.diff_stat {
            out.push_str(&format!("- {}\n", line));
        }
    }
    out.push_str(&format!(
        "\n{}: hard_observed\n\n",
        locale.evidence_grade_label()
    ));

    out.push_str(&format!("## {}\n\n", section_changed_files(locale)));
    if input.changed_files.is_empty() {
        out.push_str(&format!("- {}\n", locale.no_changed_files()));
    } else {
        for file in &input.changed_files {
            out.push_str(&format!("- {}\n", file));
        }
    }
    out.push_str(&format!(
        "\n{}: hard_observed\n\n",
        locale.evidence_grade_label()
    ));

    out.push_str(&format!("## {}\n\n", section_tests_run_or_reason(locale)));
    push_reported_section(
        &mut out,
        input,
        &["tests run", "test results", "tests run or reason not run"],
        &format!("- {}\n", locale.not_reported_by_session()),
    );
    out.push_str(&format!(
        "\n{}: agent_reported\n\n",
        locale.evidence_grade_label()
    ));

    out.push_str(&format!("## {}\n\n", section_unresolved_risks(locale)));
    push_reported_section(
        &mut out,
        input,
        &["unresolved risks", "warnings", "risks"],
        &format!("- {}\n", locale.not_reported_by_session()),
    );
    out.push_str(&format!(
        "\n{}: agent_reported\n\n",
        locale.evidence_grade_label()
    ));

    out.push_str(&format!("## {}\n\n", section_rationale(locale)));
    push_reported_section(
        &mut out,
        input,
        &[
            "rationale",
            "key decisions made",
            "key decisions",
            "completed",
        ],
        &format!("- {}\n", locale.not_reported_by_session()),
    );
    out.push_str(&format!(
        "\n{}: agent_reported\n\n",
        locale.evidence_grade_label()
    ));

    out.push_str(&format!(
        "## {}\n\n",
        section_observation_limitations(locale)
    ));
    out.push_str(&format!("- {}\n", locale.limitation_p0()));
    out.push_str(&format!("- {}\n", locale.limitation_runtime()));
    out.push_str(&format!("- {}\n", locale.limitation_tests()));
    out.push_str(&format!("- {}\n", locale.limitation_task_mapping()));
    if !input.git_available {
        out.push_str(&format!("- {}\n", locale.limitation_git()));
    }
    out.push('\n');

    out.push_str(&format!("## {}\n\n", section_evidence_grades(locale)));
    out.push_str(&format!(
        "- {}: hard_observed\n",
        label_evidence_changed_files(locale)
    ));
    out.push_str(&format!(
        "- {}: hard_observed\n",
        label_evidence_diff_summary(locale)
    ));
    out.push_str(&format!(
        "- {}: agent_reported\n",
        label_evidence_tests_run(locale)
    ));
    out.push_str(&format!(
        "- {}: agent_reported\n",
        label_evidence_commands_run(locale)
    ));
    out.push_str(&format!(
        "- {}: agent_reported\n",
        label_evidence_rationale(locale)
    ));
    out.push_str(&format!(
        "- {}: hard_observed\n",
        label_evidence_context_manifest(locale)
    ));
    out.push_str(&format!(
        "- {}: hard_observed ({})\n",
        label_evidence_research_pack(locale),
        locale.filesystem_presence()
    ));
    out.push_str(&format!(
        "- {}: inferred\n\n",
        label_evidence_task_mapping(locale)
    ));

    out.push_str(&format!("## {}\n\n", section_git_scope(locale)));
    out.push_str(&format!(
        "- {}: {}\n",
        label_baseline_or_checkpoint(locale),
        input.baseline_ref.as_deref().unwrap_or("HEAD fallback")
    ));
    out.push_str(&format!(
        "- {}: {}\n",
        label_current_head(locale),
        input.head_ref.as_deref().unwrap_or("unavailable")
    ));
    out.push_str(&format!("- {}:\n", label_generated_artifacts_list(locale)));
    out.push_str("  - evidence/changed-files.txt\n");
    out.push_str("  - evidence/diff.patch\n");
    out.push_str("  - phases/review.md\n");

    out
}

fn determine_verdict(input: &ReviewInput) -> (&'static str, String) {
    let locale = input.locale;
    let has_output = input.source_output_path.is_some();

    if !has_output && !input.git_available {
        return ("blocked", locale.verdict_blocked_no_evidence().to_string());
    }

    if !has_output {
        return ("needs_action", locale.verdict_needs_output().to_string());
    }

    let mut reasons: Vec<String> = Vec::new();

    let required_sections: &[(&str, &str)] = &[
        ("tests run", &label_tests_run(locale)),
        ("files changed", &label_files_changed(locale)),
    ];
    let mut missing: Vec<&str> = Vec::new();
    for (key, display) in required_sections {
        if !input.sections.contains_key(*key) {
            missing.push(display);
        }
    }
    if !missing.is_empty() {
        reasons.push(format!(
            "{}: {}",
            locale.missing_required_sections(),
            missing.join(", ")
        ));
    }

    if input.context_manifest.is_none() {
        reasons.push(locale.verdict_no_manifest().to_string());
    } else if let Some(ref cm) = input.context_manifest {
        let missing_required: Vec<_> = cm.missing.iter().filter(|m| m.required).collect();
        if !missing_required.is_empty() {
            reasons.push(format!(
                "{}",
                locale.verdict_missing_required_files(missing_required.len())
            ));
        }
        if !cm.quality.flags.is_empty() {
            reasons.push(format!(
                "{}: {}",
                locale.context_quality_flags(),
                cm.quality.flags.join(", ")
            ));
        }
    }

    if !input.git_available {
        reasons.push(locale.verdict_git_unavailable().to_string());
    }

    if reasons.is_empty() {
        ("ready_for_human_review", locale.verdict_ready().to_string())
    } else {
        let joined = reasons
            .iter()
            .enumerate()
            .map(|(i, r)| format!("{}. {}", i + 1, r))
            .collect::<Vec<_>>()
            .join("\n");
        ("needs_action", joined)
    }
}

fn manifest_entry_bytes(entry: &ManifestEntryRaw) -> String {
    format!("{} bytes", entry.bytes)
}

fn manifest_entry_tokens(entry: &ManifestEntryRaw) -> String {
    format!("{} estimated tokens", entry.estimated_tokens)
}

fn read_state_field(project_root: &Path, field_path: &[&str]) -> Option<String> {
    let content = fs::read_to_string(project_root.join(".vibehub/state.yaml")).ok()?;
    let value: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;
    field_path
        .iter()
        .try_fold(&value, |current, key| current.get(*key))
        .and_then(serde_yaml::Value::as_str)
        .map(ToString::to_string)
}

fn read_current_phase_status(project_root: &Path) -> Option<String> {
    read_state_field(project_root, &["current", "phase_status"])
}

fn push_reported_section(output: &mut String, input: &ReviewInput, keys: &[&str], fallback: &str) {
    for key in keys {
        if let Some(value) = input.sections.get(*key).map(|value| value.trim()) {
            if !value.is_empty() {
                output.push_str(value);
                output.push('\n');
                return;
            }
        }
    }
    output.push_str(fallback);
}

fn push_reported_section_map(
    output: &mut String,
    input: &ReviewInput,
    mapping: &[(&str, &str)],
    fallback: &str,
) {
    for (key, _display) in mapping {
        if let Some(value) = input.sections.get(*key).map(|v| v.trim()) {
            if !value.is_empty() {
                let body = truncate_for_inline(value, 180);
                output.push_str(&format!("- {}: {}\n", _display, body));
                return;
            }
        }
    }
    output.push_str(fallback);
}

fn truncate_for_inline(value: &str, max_len: usize) -> String {
    let single = value.replace('\n', " / ");
    if single.len() <= max_len {
        single
    } else {
        let mut truncated = single.chars().take(max_len).collect::<String>();
        truncated.push_str("...");
        truncated
    }
}

fn render_changed_files(files: &[String]) -> String {
    if files.is_empty() {
        "No changed files observed by Git.\n".to_string()
    } else {
        let mut output = files.join("\n");
        output.push('\n');
        output
    }
}

fn discover_baseline_ref(project_root: &Path, run_dir: &Path) -> Option<String> {
    for (path, keys) in [
        (
            project_root.join(".vibehub/state.yaml"),
            &[
                &["git", "last_checkpoint_commit"][..],
                &["git", "baseline_commit"][..],
            ][..],
        ),
        (
            run_dir.join("run.yaml"),
            &[
                &["git", "last_checkpoint_commit"][..],
                &["git", "baseline_commit"][..],
                &["last_checkpoint_commit"][..],
                &["baseline_commit"][..],
            ][..],
        ),
    ] {
        if let Some(value) = read_first_yaml_string(&path, keys) {
            if git_ref_exists(project_root, &value) {
                return Some(value);
            }
        }
    }
    git_stdout(project_root, &["rev-parse", "--verify", "HEAD"]).filter(|value| {
        let trimmed = value.trim();
        !trimmed.is_empty() && git_ref_exists(project_root, trimmed)
    })
}

fn read_first_yaml_string(path: &Path, keys: &[&[&str]]) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let value: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;
    for key_path in keys {
        let mut current = &value;
        let mut found = true;
        for key in *key_path {
            match current.get(*key) {
                Some(next) => current = next,
                None => {
                    found = false;
                    break;
                }
            }
        }
        if !found {
            continue;
        }
        if let Some(value) = current.as_str() {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn git_changed_files(project_root: &Path, baseline_ref: Option<&str>) -> Result<Vec<String>> {
    let mut files = BTreeSet::new();
    if let Some(base) = baseline_ref {
        for file in git_lines(project_root, &["diff", "--name-only", base, "HEAD", "--"])? {
            files.insert(file);
        }
    }
    for args in [
        &["diff", "--name-only", "--cached", "--"][..],
        &["diff", "--name-only", "--"][..],
        &["ls-files", "--others", "--exclude-standard"][..],
    ] {
        for file in git_lines(project_root, args)? {
            files.insert(file);
        }
    }
    Ok(files.into_iter().collect())
}

fn build_diff_patch(project_root: &Path, baseline_ref: Option<&str>) -> Result<String> {
    let mut output = String::new();
    if let Some(base) = baseline_ref {
        output.push_str(&run_git_patch(project_root, &["diff", base, "HEAD", "--"])?);
    }
    output.push_str(&run_git_patch(project_root, &["diff", "--cached", "--"])?);
    output.push_str(&run_git_patch(project_root, &["diff", "--"])?);

    let untracked = git_lines(
        project_root,
        &["ls-files", "--others", "--exclude-standard"],
    )?;
    for file in untracked {
        if is_binary_like_path(&file) {
            output.push_str(&format!(
                "\n# Untracked binary-like file omitted from patch: {file}\n"
            ));
            continue;
        }
        let null_path = if cfg!(windows) { "NUL" } else { "/dev/null" };
        output.push_str(&run_git_patch(
            project_root,
            &["diff", "--no-index", "--", null_path, &file],
        )?);
    }
    if output.trim().is_empty() {
        output.push_str("No Git diff observed.\n");
    }
    Ok(output)
}

fn git_diff_stat(project_root: &Path, baseline_ref: Option<&str>) -> Result<Vec<String>> {
    let mut lines = Vec::new();
    if let Some(base) = baseline_ref {
        lines.extend(git_lines(
            project_root,
            &["diff", "--stat", base, "HEAD", "--"],
        )?);
    }
    lines.extend(git_lines(
        project_root,
        &["diff", "--stat", "--cached", "--"],
    )?);
    lines.extend(git_lines(project_root, &["diff", "--stat", "--"])?);
    Ok(lines)
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

fn git_ref_exists(project_root: &Path, value: &str) -> bool {
    silent_command("git")
        .arg("-C")
        .arg(project_root)
        .args(["rev-parse", "--verify", "--quiet", value])
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

fn git_lines(project_root: &Path, args: &[&str]) -> Result<Vec<String>> {
    let output = silent_command("git")
        .arg("-C")
        .arg(project_root)
        .args(args)
        .output()
        .with_context(|| format!("Failed to run git {}", args.join(" ")))?;
    if !output.status.success() {
        return Err(anyhow!(
            "Git command failed: git {}\n{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.replace('\\', "/"))
        .collect())
}

fn run_git_patch(project_root: &Path, args: &[&str]) -> Result<String> {
    let output = silent_command("git")
        .arg("-C")
        .arg(project_root)
        .args(args)
        .output()
        .with_context(|| format!("Failed to run git {}", args.join(" ")))?;
    if !output.status.success() && !args.contains(&"--no-index") {
        return Err(anyhow!(
            "Git command failed: git {}\n{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn is_binary_like_path(path: &str) -> bool {
    let lower = path.to_lowercase();
    [
        ".png", ".jpg", ".jpeg", ".gif", ".webp", ".ico", ".icns", ".pdf", ".zip", ".exe", ".dll",
    ]
    .iter()
    .any(|extension| lower.ends_with(extension))
}

fn parse_output_sections(path: &Path) -> Result<BTreeMap<String, String>> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
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
    let sessions_dir = project_root.join(run_path).join("sessions");
    if !sessions_dir.is_dir() {
        return Ok(None);
    }

    let mut candidates = Vec::new();
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
            candidates.push((session_id, output_path));
        }
    }
    candidates.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(candidates.pop())
}

fn read_context_manifest(
    project_root: &Path,
    run_path: &str,
) -> (Option<ContextManifestRaw>, Option<String>) {
    let run_dir = project_root.join(run_path);
    let context_packs_dir = run_dir.join("context-packs");
    if !context_packs_dir.is_dir() {
        return (None, None);
    }

    let read_manifest = |path: &Path| -> Option<(ContextManifestRaw, String)> {
        let content = fs::read_to_string(path).ok()?;
        let manifest: ContextManifestRaw = serde_yaml::from_str(&content).ok()?;
        let rel = relative_to_project(project_root, path)
            .map(|p| normalize_path(&p))
            .unwrap_or_else(|_| path.to_string_lossy().to_string());
        Some((manifest, rel))
    };

    let current_phase = read_current_phase(project_root);
    if let Some(ref phase) = current_phase {
        let named_path = context_packs_dir.join(format!("{}.manifest.yaml", phase));
        if named_path.is_file() {
            if let Some((manifest, rel)) = read_manifest(&named_path) {
                return (Some(manifest), Some(rel));
            }
        }
    }

    if let Ok(entries) = fs::read_dir(&context_packs_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .map_or(false, |n| {
                    n.ends_with(".manifest.yaml") || n.ends_with(".manifest.yml")
                })
            {
                if let Some((manifest, rel)) = read_manifest(&path) {
                    return (Some(manifest), Some(rel));
                }
            }
        }
    }

    (None, None)
}

fn read_current_phase(project_root: &Path) -> Option<String> {
    let content = fs::read_to_string(project_root.join(".vibehub/state.yaml")).ok()?;
    let value: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;
    value
        .get("current")
        .and_then(|c| c.get("phase"))
        .and_then(|p| p.as_str())
        .map(str::to_string)
}

fn read_research_pack_info(project_root: &Path) -> (bool, Option<String>, bool, bool) {
    let path = project_root
        .join(".vibehub")
        .join("research")
        .join("current")
        .join("research-pack.md");
    let source_log = project_root
        .join(".vibehub")
        .join("research")
        .join("current")
        .join("source-log.yaml");
    let findings = project_root
        .join(".vibehub")
        .join("research")
        .join("current")
        .join("findings.yaml");
    if path.is_file() {
        let rel = relative_to_project(project_root, &path)
            .map(|p| normalize_path(&p))
            .unwrap_or_else(|_| path.to_string_lossy().to_string());
        (true, Some(rel), source_log.is_file(), findings.is_file())
    } else {
        (false, None, false, false)
    }
}

fn review_title(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Review Report",
        VibehubLocale::ZhCn => "审查报告",
        VibehubLocale::ZhTw => "審查報告",
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

fn label_run_path(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Run path",
        VibehubLocale::ZhCn => "运行路径",
        VibehubLocale::ZhTw => "執行路徑",
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

fn section_verdict(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Verdict",
        VibehubLocale::ZhCn => "判定",
        VibehubLocale::ZhTw => "判定",
    }
}

fn section_evidence_map(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Evidence Map",
        VibehubLocale::ZhCn => "证据地图",
        VibehubLocale::ZhTw => "證據地圖",
    }
}

fn label_git_diff_summary(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Git diff summary",
        VibehubLocale::ZhCn => "Git diff 摘要",
        VibehubLocale::ZhTw => "Git diff 摘要",
    }
}

fn label_changed_files_list(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Changed files list",
        VibehubLocale::ZhCn => "变更文件列表",
        VibehubLocale::ZhTw => "變更檔案列表",
    }
}

fn label_generated_artifacts(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Generated artifacts",
        VibehubLocale::ZhCn => "生成产物",
        VibehubLocale::ZhTw => "產生產物",
    }
}

fn label_context_manifest_label(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Context manifest",
        VibehubLocale::ZhCn => "上下文清单",
        VibehubLocale::ZhTw => "上下文清單",
    }
}

fn label_tests_run(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Tests run",
        VibehubLocale::ZhCn => "测试执行",
        VibehubLocale::ZhTw => "測試執行",
    }
}

fn label_tests_run_or_reason(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Tests run or reason not run",
        VibehubLocale::ZhCn => "测试执行或未执行原因",
        VibehubLocale::ZhTw => "測試執行或未執行原因",
    }
}

fn label_test_results(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Test results",
        VibehubLocale::ZhCn => "测试结果",
        VibehubLocale::ZhTw => "測試結果",
    }
}

fn label_commands_run(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Commands run",
        VibehubLocale::ZhCn => "命令执行",
        VibehubLocale::ZhTw => "命令執行",
    }
}

fn label_files_changed(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Files changed",
        VibehubLocale::ZhCn => "变更文件",
        VibehubLocale::ZhTw => "變更檔案",
    }
}

fn label_completed_summary(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Completed / summary",
        VibehubLocale::ZhCn => "完成 / 摘要",
        VibehubLocale::ZhTw => "完成 / 摘要",
    }
}

fn label_key_decisions(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Key decisions",
        VibehubLocale::ZhCn => "关键决策",
        VibehubLocale::ZhTw => "關鍵決策",
    }
}

fn label_rationale(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Rationale",
        VibehubLocale::ZhCn => "理由",
        VibehubLocale::ZhTw => "理由",
    }
}

fn label_summary(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Summary",
        VibehubLocale::ZhCn => "摘要",
        VibehubLocale::ZhTw => "摘要",
    }
}

fn label_unresolved_risks(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Unresolved risks",
        VibehubLocale::ZhCn => "未解决风险",
        VibehubLocale::ZhTw => "未解決風險",
    }
}

fn label_warnings(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Warnings",
        VibehubLocale::ZhCn => "警告",
        VibehubLocale::ZhTw => "警告",
    }
}

fn label_risks(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Risks",
        VibehubLocale::ZhCn => "风险",
        VibehubLocale::ZhTw => "風險",
    }
}

fn label_task_mapping(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Task mapping",
        VibehubLocale::ZhCn => "任务映射",
        VibehubLocale::ZhTw => "任務映射",
    }
}

fn section_context_manifest(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Context Manifest",
        VibehubLocale::ZhCn => "上下文清单",
        VibehubLocale::ZhTw => "上下文清單",
    }
}

fn label_manifest_id(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Manifest ID",
        VibehubLocale::ZhCn => "清单 ID",
        VibehubLocale::ZhTw => "清單 ID",
    }
}

fn label_phase(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Phase",
        VibehubLocale::ZhCn => "阶段",
        VibehubLocale::ZhTw => "階段",
    }
}

fn label_source_commit(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Source commit",
        VibehubLocale::ZhCn => "源提交",
        VibehubLocale::ZhTw => "源提交",
    }
}

fn label_budget(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Budget:",
        VibehubLocale::ZhCn => "预算：",
        VibehubLocale::ZhTw => "預算：",
    }
}

fn section_included_files(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Included Files",
        VibehubLocale::ZhCn => "包含文件",
        VibehubLocale::ZhTw => "包含檔案",
    }
}

fn section_missing_required_context(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Missing Required Context",
        VibehubLocale::ZhCn => "缺失必要上下文",
        VibehubLocale::ZhTw => "缺失必要上下文",
    }
}

fn section_excluded_secret_files(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Excluded Secret-like Files",
        VibehubLocale::ZhCn => "排除的类密钥文件",
        VibehubLocale::ZhTw => "排除的類密鑰檔案",
    }
}

fn section_quality(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Quality",
        VibehubLocale::ZhCn => "质量",
        VibehubLocale::ZhTw => "品質",
    }
}

fn label_has_goal(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Has goal",
        VibehubLocale::ZhCn => "有目标",
        VibehubLocale::ZhTw => "有目標",
    }
}

fn label_has_plan(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Has plan",
        VibehubLocale::ZhCn => "有计划",
        VibehubLocale::ZhTw => "有計畫",
    }
}

fn label_has_relevant_code(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Has relevant code",
        VibehubLocale::ZhCn => "有相关代码",
        VibehubLocale::ZhTw => "有相關程式碼",
    }
}

fn label_has_test_hint(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Has test hint",
        VibehubLocale::ZhCn => "有测试提示",
        VibehubLocale::ZhTw => "有測試提示",
    }
}

fn label_flags(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Flags",
        VibehubLocale::ZhCn => "标记",
        VibehubLocale::ZhTw => "標記",
    }
}

fn label_observation_grade(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Observation grade",
        VibehubLocale::ZhCn => "观察等级",
        VibehubLocale::ZhTw => "觀察等級",
    }
}

fn section_research_pack(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Research Pack",
        VibehubLocale::ZhCn => "研究包",
        VibehubLocale::ZhTw => "研究包",
    }
}

fn label_status(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Status",
        VibehubLocale::ZhCn => "状态",
        VibehubLocale::ZhTw => "狀態",
    }
}

fn label_research_evidence(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Research evidence available",
        VibehubLocale::ZhCn => "研究证据可用",
        VibehubLocale::ZhTw => "研究證據可用",
    }
}

fn label_source_log(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Source log",
        VibehubLocale::ZhCn => "来源日志",
        VibehubLocale::ZhTw => "來源日誌",
    }
}

fn label_findings(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Findings",
        VibehubLocale::ZhCn => "发现",
        VibehubLocale::ZhTw => "發現",
    }
}

fn section_diff_summary(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Diff Summary",
        VibehubLocale::ZhCn => "差异摘要",
        VibehubLocale::ZhTw => "差異摘要",
    }
}

fn section_changed_files(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Changed Files",
        VibehubLocale::ZhCn => "变更文件",
        VibehubLocale::ZhTw => "變更檔案",
    }
}

fn section_tests_run_or_reason(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Tests Run Or Reason Not Run",
        VibehubLocale::ZhCn => "测试执行或未执行原因",
        VibehubLocale::ZhTw => "測試執行或未執行原因",
    }
}

fn section_unresolved_risks(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Unresolved Risks",
        VibehubLocale::ZhCn => "未解决风险",
        VibehubLocale::ZhTw => "未解決風險",
    }
}

fn section_rationale(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Rationale",
        VibehubLocale::ZhCn => "理由",
        VibehubLocale::ZhTw => "理由",
    }
}

fn section_observation_limitations(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Observation Limitations",
        VibehubLocale::ZhCn => "观察局限性",
        VibehubLocale::ZhTw => "觀察局限性",
    }
}

fn section_evidence_grades(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Evidence Grades",
        VibehubLocale::ZhCn => "证据等级",
        VibehubLocale::ZhTw => "證據等級",
    }
}

fn label_evidence_changed_files(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "changed files",
        VibehubLocale::ZhCn => "变更文件",
        VibehubLocale::ZhTw => "變更檔案",
    }
}

fn label_evidence_diff_summary(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "diff summary",
        VibehubLocale::ZhCn => "差异摘要",
        VibehubLocale::ZhTw => "差異摘要",
    }
}

fn label_evidence_tests_run(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "tests run",
        VibehubLocale::ZhCn => "测试执行",
        VibehubLocale::ZhTw => "測試執行",
    }
}

fn label_evidence_commands_run(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "commands run",
        VibehubLocale::ZhCn => "命令执行",
        VibehubLocale::ZhTw => "命令執行",
    }
}

fn label_evidence_rationale(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "rationale",
        VibehubLocale::ZhCn => "理由",
        VibehubLocale::ZhTw => "理由",
    }
}

fn label_evidence_context_manifest(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "context manifest",
        VibehubLocale::ZhCn => "上下文清单",
        VibehubLocale::ZhTw => "上下文清單",
    }
}

fn label_evidence_research_pack(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "research pack",
        VibehubLocale::ZhCn => "研究包",
        VibehubLocale::ZhTw => "研究包",
    }
}

fn label_evidence_task_mapping(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "task mapping",
        VibehubLocale::ZhCn => "任务映射",
        VibehubLocale::ZhTw => "任務映射",
    }
}

fn section_git_scope(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Git Scope",
        VibehubLocale::ZhCn => "Git 范围",
        VibehubLocale::ZhTw => "Git 範圍",
    }
}

fn label_baseline_or_checkpoint(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "baseline or checkpoint",
        VibehubLocale::ZhCn => "基线或检查点",
        VibehubLocale::ZhTw => "基線或檢查點",
    }
}

fn label_current_head(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "current HEAD",
        VibehubLocale::ZhCn => "当前 HEAD",
        VibehubLocale::ZhTw => "目前 HEAD",
    }
}

fn label_generated_artifacts_list(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "generated artifacts",
        VibehubLocale::ZhCn => "生成产物",
        VibehubLocale::ZhTw => "產生產物",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vibehub::current::{write_current_run_pointer, write_current_task_pointer};
    use std::process::Command;
    use uuid::Uuid;

    fn temp_project() -> PathBuf {
        let path = std::env::temp_dir().join(format!("vibehub-review-test-{}", Uuid::new_v4()));
        fs::create_dir_all(path.join(".vibehub/tasks/T-001/runs/R-001/sessions/S-001"))
            .expect("create session");
        fs::write(
            path.join(".vibehub/state.yaml"),
            "current:\n  phase: review\ngit:\n  baseline_commit: null\n",
        )
        .expect("write state");
        write_current_task_pointer(&path, "T-001").expect("task pointer");
        write_current_run_pointer(&path, "T-001", "R-001").expect("run pointer");
        path
    }

    fn init_git(project: &Path) {
        Command::new("git")
            .arg("-C")
            .arg(project)
            .arg("init")
            .output()
            .expect("git init");
        Command::new("git")
            .arg("-C")
            .arg(project)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .expect("git config email");
        Command::new("git")
            .arg("-C")
            .arg(project)
            .args(["config", "user.name", "Test User"])
            .output()
            .expect("git config name");
    }

    fn commit_all(project: &Path, message: &str) -> String {
        Command::new("git")
            .arg("-C")
            .arg(project)
            .args(["add", "."])
            .output()
            .expect("git add");
        Command::new("git")
            .arg("-C")
            .arg(project)
            .args(["commit", "-m", message])
            .output()
            .expect("git commit");
        git_stdout(project, &["rev-parse", "HEAD"]).expect("head")
    }

    fn write_output(project: &Path) {
        fs::write(
            project.join(".vibehub/tasks/T-001/runs/R-001/sessions/S-001/output.md"),
            r#"# Session Output

## Tests Run
- cargo test vibehub::review

## Unresolved Risks
- Review is not a full AI review.

## Rationale
- Minimal P0 evidence only.
"#,
        )
        .expect("write output");
    }

    fn write_output_with_sections(project: &Path, sections: &[(&str, &str)]) {
        let mut content = String::from("# Session Output\n\n");
        for (heading, body) in sections {
            content.push_str(&format!("## {}\n{}\n\n", heading, body));
        }
        fs::write(
            project.join(".vibehub/tasks/T-001/runs/R-001/sessions/S-001/output.md"),
            content,
        )
        .expect("write output");
    }

    fn write_context_manifest(project: &Path, yaml_content: &str) {
        let manifest_dir = project.join(".vibehub/tasks/T-001/runs/R-001/context-packs");
        fs::create_dir_all(&manifest_dir).expect("create context-packs dir");
        fs::write(manifest_dir.join("review.manifest.yaml"), yaml_content).expect("write manifest");
    }

    #[test]
    fn generates_review_evidence_for_dirty_changes() {
        let project = temp_project();
        init_git(&project);
        write_output(&project);
        fs::write(project.join("src.rs"), "initial\n").expect("write src");
        commit_all(&project, "initial");
        fs::write(project.join("src.rs"), "changed\n").expect("change src");
        fs::write(project.join("new.txt"), "untracked\n").expect("write untracked");

        let result = generate_review_evidence(&project).expect("generate review");
        let changed =
            fs::read_to_string(project.join(&result.changed_files_path)).expect("read changed");
        let patch = fs::read_to_string(project.join(&result.diff_path)).expect("read patch");
        let review = fs::read_to_string(project.join(&result.review_path)).expect("read review");

        assert_eq!(result.changed_files_count, 2);
        assert!(changed.contains("src.rs"));
        assert!(changed.contains("new.txt"));
        assert!(patch.contains("-initial"));
        assert!(patch.contains("+changed"));
        assert!(patch.contains("+untracked"));
        assert!(review.contains("## Verdict"));
        assert!(review.contains("needs_action"));
        assert!(review.contains("tests run: agent_reported"));
        assert!(review.contains("cargo test vibehub::review"));
        assert!(review.contains("## Evidence Map"));
        assert!(review.contains("### hard_observed"));
        assert!(review.contains("### agent_reported"));
        assert!(review.contains("### inferred"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn uses_baseline_for_committed_changes() {
        let project = temp_project();
        init_git(&project);
        fs::write(project.join("src.rs"), "initial\n").expect("write src");
        let baseline = commit_all(&project, "initial");
        fs::write(
            project.join(".vibehub/state.yaml"),
            format!("current:\n  phase: review\ngit:\n  baseline_commit: {baseline}\n"),
        )
        .expect("write state baseline");
        fs::write(project.join("src.rs"), "committed\n").expect("change src");
        commit_all(&project, "change");

        let result = generate_review_evidence(&project).expect("generate review");
        let changed =
            fs::read_to_string(project.join(&result.changed_files_path)).expect("read changed");
        let patch = fs::read_to_string(project.join(&result.diff_path)).expect("read patch");

        assert_eq!(result.baseline_ref.as_deref(), Some(baseline.as_str()));
        assert!(changed.contains("src.rs"));
        assert!(patch.contains("+committed"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn generates_placeholders_without_git_repo() {
        let project = temp_project();

        let result = generate_review_evidence(&project).expect("generate review");
        let changed =
            fs::read_to_string(project.join(&result.changed_files_path)).expect("read changed");
        let patch = fs::read_to_string(project.join(&result.diff_path)).expect("read patch");
        let review = fs::read_to_string(project.join(&result.review_path)).expect("read review");

        assert_eq!(result.changed_files_count, 0);
        assert!(changed.contains("No changed files observed"));
        assert!(patch.contains("Git repository not available"));
        assert!(review.contains("Git was not available"));
        assert!(review.contains("## Verdict"));
        assert!(review.contains("blocked"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn review_includes_context_evidence_when_manifest_present() {
        let project = temp_project();
        init_git(&project);
        write_output_with_sections(
            &project,
            &[
                ("Files Changed", "- src/main.rs\n- src/lib.rs"),
                ("Tests Run", "- cargo test vibehub: 55 passed"),
                ("Commands Run", "- cargo test vibehub\n- npm run build"),
            ],
        );
        write_context_manifest(
            &project,
            r#"id: review-context-R-001-v1
phase: review
task_id: T-001
run_id: R-001
generated_at: "2026-05-04T00:00:00Z"
source_commit: "abc1234"
budget:
  max_file_size_bytes: 262144
  max_tokens: 12000
  estimated_tokens: 300
included:
  - path: .vibehub/tasks/T-001/task.yaml
    reason: active task metadata
    required: true
    confidence: high
    bytes: 173
    estimated_tokens: 44
  - path: src/main.rs
    reason: relevant source code
    required: false
    confidence: medium
    bytes: 500
    estimated_tokens: 125
missing: []
excluded: []
quality:
  has_goal: true
  has_plan: true
  has_relevant_code: true
  has_test_hint: false
  flags: []
observation:
  evidence_grade: hard_observed
"#,
        );

        fs::write(project.join("src.rs"), "fn main() {}\n").expect("write src");
        commit_all(&project, "initial");

        let result = generate_review_evidence(&project).expect("generate review");
        let review = fs::read_to_string(project.join(&result.review_path)).expect("read review");

        assert!(review.contains("## Context Manifest"));
        assert!(review.contains("review-context-R-001-v1"));
        assert!(review.contains("### Included Files"));
        assert!(review.contains("src/main.rs"));
        assert!(review.contains("active task metadata"));
        assert!(review.contains("### Quality"));
        assert!(review.contains("Has goal: true"));
        assert!(review.contains("Has test hint: false"));
        assert!(review.contains("context manifest: hard_observed"));
        assert!(review.contains("ready_for_human_review"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn verdict_is_needs_action_when_required_output_missing() {
        let project = temp_project();
        init_git(&project);
        write_output_with_sections(&project, &[("Completed", "- Did some work")]);

        fs::write(project.join("src.rs"), "fn main() {}\n").expect("write src");
        commit_all(&project, "initial");

        let result = generate_review_evidence(&project).expect("generate review");
        let review = fs::read_to_string(project.join(&result.review_path)).expect("read review");

        assert!(review.contains("## Verdict"));
        assert!(review.contains("needs_action"));
        assert!(review.contains("Missing required output sections"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn failed_review_auto_releases_review_capability_with_dirty_delta() {
        let project = temp_project();
        init_git(&project);
        write_output_with_sections(&project, &[("Completed", "- Did some work")]);
        events::append_structured_run_event(
            &project,
            "T-001",
            "R-001",
            events::VibehubEvent::CapabilityClaimed {
                capability: "review".to_string(),
            },
        )
        .expect("claim review");

        fs::write(project.join("src.rs"), "fn main() {}\n").expect("write src");
        commit_all(&project, "initial");

        generate_review_evidence(&project).expect("generate review");
        let events = events::list_events(&project, "T-001", "R-001", None).expect("events");
        let release = events
            .iter()
            .rev()
            .find(|event| {
                event.event.get("event_type").and_then(JsonValue::as_str)
                    == Some("CapabilityReleased")
            })
            .expect("release event");
        let payload = release.event.get("payload").expect("payload");
        assert_eq!(
            payload.get("capability").and_then(JsonValue::as_str),
            Some("review")
        );
        assert_eq!(
            payload.get("outcome").and_then(JsonValue::as_str),
            Some("failed")
        );
        assert_eq!(
            payload.get("task_pack_dirty").and_then(JsonValue::as_bool),
            Some(true)
        );

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn ready_review_does_not_emit_failure_release() {
        let project = temp_project();
        init_git(&project);
        write_output_with_sections(
            &project,
            &[
                ("Files Changed", "- src/main.rs"),
                ("Tests Run", "- cargo test vibehub"),
            ],
        );
        write_context_manifest(
            &project,
            r#"id: review-context-R-001-v1
phase: review
budget:
  max_file_size_bytes: 262144
  max_tokens: 12000
  estimated_tokens: 100
included:
  - path: src/main.rs
    reason: relevant source
    required: true
missing: []
excluded: []
quality:
  has_goal: true
  has_plan: true
  has_relevant_code: true
  has_test_hint: true
  flags: []
observation:
  evidence_grade: hard_observed
"#,
        );
        events::append_structured_run_event(
            &project,
            "T-001",
            "R-001",
            events::VibehubEvent::CapabilityClaimed {
                capability: "review".to_string(),
            },
        )
        .expect("claim review");

        fs::write(project.join("src.rs"), "fn main() {}\n").expect("write src");
        commit_all(&project, "initial");

        generate_review_evidence(&project).expect("generate review");
        let events = events::list_events(&project, "T-001", "R-001", None).expect("events");
        assert!(!events.iter().any(|event| {
            event.event.get("event_type").and_then(JsonValue::as_str) == Some("CapabilityReleased")
        }));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn generates_readable_report_when_git_unavailable() {
        let project = temp_project();
        write_output_with_sections(
            &project,
            &[
                ("Files Changed", "- src/main.rs"),
                ("Tests Run", "- cargo test vibehub"),
                ("Commands Run", "- cargo test vibehub"),
            ],
        );

        let result = generate_review_evidence(&project).expect("generate review");
        let review = fs::read_to_string(project.join(&result.review_path)).expect("read review");

        assert!(review.contains("# Review Report"));
        assert!(review.contains("## Verdict"));
        assert!(review.contains("needs_action"));
        assert!(review.contains("Git was not available"));
        assert!(review.contains("## Evidence Map"));
        assert!(review.contains("### hard_observed"));
        assert!(review.contains("Git diff summary: unavailable"));
        assert!(review.contains("### agent_reported"));
        assert!(review.contains("### inferred"));
        assert!(review.contains("## Research Pack"));
        assert!(review.contains("## Observation Limitations"));
        assert!(review.contains("## Evidence Grades"));
        assert!(review.contains("## Git Scope"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn review_with_manifest_and_missing_context_shows_flags() {
        let project = temp_project();
        init_git(&project);
        write_output_with_sections(
            &project,
            &[
                ("Files Changed", "- src/main.rs"),
                ("Tests Run", "- cargo test vibehub"),
            ],
        );
        write_context_manifest(
            &project,
            r#"id: review-context-R-001-v1
phase: review
budget:
  max_file_size_bytes: 262144
  max_tokens: 12000
  estimated_tokens: 100
included:
  - path: .vibehub/tasks/T-001/task.yaml
    reason: task metadata
    required: true
missing:
  - path: src/missing.rs
    reason: expected source file
    required: true
  - path: tests/optional.rs
    reason: optional test
    required: false
excluded:
  - path: .env.local
    reason: secret-like path denied
    required: false
    policy: deny_secret_path
quality:
  has_goal: true
  has_plan: false
  has_relevant_code: true
  has_test_hint: false
  flags: ["missing_optional_context", "excluded_context_entries"]
observation:
  evidence_grade: hard_observed
"#,
        );

        fs::write(project.join("src.rs"), "fn main() {}\n").expect("write src");
        commit_all(&project, "initial");

        let result = generate_review_evidence(&project).expect("generate review");
        let review = fs::read_to_string(project.join(&result.review_path)).expect("read review");

        assert!(review.contains("needs_action"));
        assert!(review.contains("required context file(s) missing per manifest"));
        assert!(review.contains("missing_optional_context"));
        assert!(review.contains("excluded_context_entries"));
        assert!(review.contains("### Missing Required Context"));
        assert!(review.contains("src/missing.rs"));
        assert!(review.contains("### Excluded Secret-like Files"));
        assert!(review.contains(".env.local"));
        assert!(review.contains("secret-like path denied"));

        fs::remove_dir_all(project).expect("cleanup");
    }
}

use crate::vibehub::locale::VibehubLocale;
use crate::vibehub::util::{canonical_project_root, normalize_path, relative_to_project};
use crate::vibehub::{
    agent_adapter, agent_view, context, current, drift, events, init, phase, start_task, status,
};
use anyhow::{Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use serde_yaml::{Mapping, Value};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SyncReport {
    pub project_root: String,
    pub status: String,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub phase: Option<String>,
    pub report_path: Option<String>,
    pub agent_view_sync_path: String,
    pub agent_view_status: String,
    pub context_status: String,
    pub adapter_status: String,
    pub drift_warnings: Vec<String>,
    pub changed_files: Vec<String>,
    pub phase_validation_status: Option<String>,
    pub missing_phase_outputs: Vec<String>,
    pub questions_for_user: Vec<String>,
    pub recommended_actions: Vec<String>,
}

pub fn sync_workspace(project_root: impl AsRef<Path>) -> Result<SyncReport> {
    sync_workspace_with_locale(project_root, None)
}

pub fn sync_workspace_with_locale(
    project_root: impl AsRef<Path>,
    locale_override: Option<&str>,
) -> Result<SyncReport> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    init::init_project(&project_root)?;
    let locale = VibehubLocale::detect_with_override(&project_root, locale_override);

    let current_task = current::resolve_current_task(&project_root).ok();
    let current_run = current_task
        .as_ref()
        .and_then(|task| current::resolve_current_run(&project_root, &task.task_id).ok());
    let cockpit_status = status::read_cockpit_status(&project_root).ok();
    let phase_name = cockpit_status
        .as_ref()
        .and_then(|s| s.current_phase.clone());

    let mut context_status = "not_applicable".to_string();
    if let (Some(task), Some(run), Some(phase)) = (&current_task, &current_run, &phase_name) {
        match start_task::ensure_context_spec(&project_root, &task.task_id, &run.run_id, phase)
            .and_then(|_| {
                context::build_context_pack(&project_root, &task.task_id, &run.run_id, phase)
            }) {
            Ok(pack) => {
                context_status = format!(
                    "rebuilt:{} included={} missing={}",
                    pack.pack_path, pack.included_count, pack.missing_count
                );
            }
            Err(error) => {
                context_status = format!("needs_action:{error:#}");
            }
        }
    }

    let agent_view_status = match agent_view::generate_agent_view(&project_root) {
        Ok(result) => format!("updated:{}", result.current_path),
        Err(error) => format!("skipped:{error:#}"),
    };

    let adapter_status = match agent_adapter::sync_agent_adapters(&project_root, None, false) {
        Ok(result) => result.summary,
        Err(error) => format!("needs_action:{error:#}"),
    };

    let drift = drift::check_workspace_drift(&project_root)?;
    let phase_validation = phase::validate_phase(&project_root).ok();

    let questions_for_user = build_questions(
        locale,
        current_task.as_ref().map(|task| task.task_id.as_str()),
        current_run.as_ref().map(|run| run.run_id.as_str()),
        &drift,
        phase_validation.as_ref(),
        &agent_view_status,
        &context_status,
    );
    let recommended_actions = build_recommended_actions(
        locale,
        current_task.as_ref().map(|task| task.task_id.as_str()),
        &drift,
        phase_validation.as_ref(),
        &questions_for_user,
    );

    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let report_body = render_report(
        locale,
        &now,
        current_task.as_ref().map(|task| task.task_id.as_str()),
        current_run.as_ref().map(|run| run.run_id.as_str()),
        phase_name.as_deref(),
        &agent_view_status,
        &context_status,
        &adapter_status,
        &drift,
        phase_validation.as_ref(),
        &questions_for_user,
        &recommended_actions,
    );

    let report_path = if let (Some(task), Some(run)) = (&current_task, &current_run) {
        let sync_dir = project_root
            .join(".vibehub")
            .join("tasks")
            .join(&task.task_id)
            .join("runs")
            .join(&run.run_id)
            .join("sync");
        fs::create_dir_all(&sync_dir)
            .with_context(|| format!("Failed to create {}", sync_dir.display()))?;
        let report = sync_dir.join(format!("sync-{}.md", Utc::now().format("%Y%m%d-%H%M%S")));
        fs::write(&report, &report_body)
            .with_context(|| format!("Failed to write {}", report.display()))?;
        Some(normalize_path(&relative_to_project(
            &project_root,
            &report,
        )?))
    } else {
        let report = project_root.join(".vibehub").join("sync.md");
        fs::write(&report, &report_body)
            .with_context(|| format!("Failed to write {}", report.display()))?;
        Some(normalize_path(&relative_to_project(
            &project_root,
            &report,
        )?))
    };

    let agent_view_sync = project_root.join(".vibehub/agent-view/sync.md");
    if let Some(parent) = agent_view_sync.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    fs::write(&agent_view_sync, &report_body)
        .with_context(|| format!("Failed to write {}", agent_view_sync.display()))?;
    let agent_view_sync_path =
        normalize_path(&relative_to_project(&project_root, &agent_view_sync)?);

    update_sync_state(
        &project_root,
        &now,
        report_path.as_deref(),
        &agent_view_sync_path,
        &drift,
        &questions_for_user,
    )?;
    let _ = events::append_current_run_event(
        &project_root,
        "sync_report_written",
        "Workspace sync report generated.",
        serde_json::json!({
            "report_path": report_path.clone(),
            "agent_view_sync_path": agent_view_sync_path.clone(),
            "status": if questions_for_user.is_empty() && drift.warnings.is_empty() {
                "synced"
            } else {
                "needs_attention"
            },
            "warnings_count": drift.warnings.len(),
            "changed_files_count": drift.changed_files.len(),
            "questions_count": questions_for_user.len(),
        }),
    );

    let status = if questions_for_user.is_empty() && drift.warnings.is_empty() {
        "synced"
    } else {
        "needs_attention"
    }
    .to_string();

    Ok(SyncReport {
        project_root: normalize_path(&project_root),
        status,
        task_id: current_task.map(|task| task.task_id),
        run_id: current_run.map(|run| run.run_id),
        phase: phase_name,
        report_path,
        agent_view_sync_path,
        agent_view_status,
        context_status,
        adapter_status,
        drift_warnings: drift.warnings,
        changed_files: drift.changed_files,
        phase_validation_status: phase_validation.as_ref().map(|v| v.status.clone()),
        missing_phase_outputs: phase_validation
            .map(|v| v.missing_outputs)
            .unwrap_or_default(),
        questions_for_user,
        recommended_actions,
    })
}

fn build_questions(
    locale: VibehubLocale,
    task_id: Option<&str>,
    run_id: Option<&str>,
    drift: &drift::WorkspaceDriftReport,
    validation: Option<&phase::PhaseValidationResult>,
    agent_view_status: &str,
    context_status: &str,
) -> Vec<String> {
    let mut questions = Vec::new();
    if task_id.is_none() || run_id.is_none() {
        questions.push(match locale {
            VibehubLocale::En => "No active VibeHub task/run is currently resolved. Should the current engineering state be attached to a new task? Please provide the goal, current progress, and next plan.",
            VibehubLocale::ZhCn => "当前没有解析到 active VibeHub task/run。是否要把当前工程状态归入一个新任务？请补充目标、当前进度和下一步计划。",
            VibehubLocale::ZhTw => "目前沒有解析到 active VibeHub task/run。是否要把目前工程狀態歸入一個新任務？請補充目標、目前進度和下一步計畫。",
        }.to_string());
    }
    if drift.dirty {
        questions.push(match locale {
            VibehubLocale::En => format!(
                "Git currently reports {} changed file(s). Do all of these belong to the active VibeHub task? If not, identify which files should be excluded or moved to a separate task.",
                drift.changed_files.len()
            ),
            VibehubLocale::ZhCn => format!(
                "Git 当前有 {} 个变更文件。这些变更是否都属于当前 VibeHub 任务？如不是，请说明哪些需要排除或另开任务。",
                drift.changed_files.len()
            ),
            VibehubLocale::ZhTw => format!(
                "Git 目前有 {} 個變更檔案。這些變更是否都屬於目前 VibeHub 任務？若不是，請說明哪些需要排除或另開任務。",
                drift.changed_files.len()
            ),
        });
    }
    if let Some(validation) = validation {
        if !validation.missing_outputs.is_empty() {
            questions.push(match locale {
                VibehubLocale::En => format!(
                    "The current phase is missing required output(s): {}. Please add current progress, unfinished work, validation results or why validation was not run, and the next plan.",
                    validation.missing_outputs.join(", ")
                ),
                VibehubLocale::ZhCn => format!(
                    "当前阶段缺少必要输出：{}。请补充当前进度、未完成项、验证结果或未验证原因，以及后续计划。",
                    validation.missing_outputs.join(", ")
                ),
                VibehubLocale::ZhTw => format!(
                    "目前階段缺少必要輸出：{}。請補充目前進度、未完成項、驗證結果或未驗證原因，以及後續計畫。",
                    validation.missing_outputs.join(", ")
                ),
            });
        }
    }
    if agent_view_status.starts_with("skipped:") {
        questions.push(match locale {
            VibehubLocale::En => "Agent view could not be refreshed. Please confirm whether the current task pointers are correct or whether a new task should be created from the current project state.",
            VibehubLocale::ZhCn => "agent-view 未能刷新。请确认当前任务指针是否正确，或是否需要从当前工程状态创建新任务。",
            VibehubLocale::ZhTw => "agent-view 未能刷新。請確認目前任務指標是否正確，或是否需要從目前工程狀態建立新任務。",
        }.to_string());
    }
    if context_status.starts_with("needs_action:") {
        questions.push(match locale {
            VibehubLocale::En => "The current context pack could not be rebuilt completely. Please confirm whether missing files still belong to the required task context.",
            VibehubLocale::ZhCn => "当前上下文包无法完整重建。请确认缺失文件是否仍属于任务必要上下文。",
            VibehubLocale::ZhTw => "目前上下文包無法完整重建。請確認缺失檔案是否仍屬於任務必要上下文。",
        }.to_string());
    }
    questions
}

fn build_recommended_actions(
    locale: VibehubLocale,
    task_id: Option<&str>,
    drift: &drift::WorkspaceDriftReport,
    validation: Option<&phase::PhaseValidationResult>,
    questions: &[String],
) -> Vec<String> {
    let mut actions = Vec::new();
    if task_id.is_none() {
        actions.push(
            match locale {
                VibehubLocale::En => {
                    "Run `vibehub start <project> <mode> <title>` or start a task from the cockpit."
                }
                VibehubLocale::ZhCn => {
                    "运行 `vibehub start <project> <mode> <title>`，或在 cockpit 中启动任务。"
                }
                VibehubLocale::ZhTw => {
                    "執行 `vibehub start <project> <mode> <title>`，或在 cockpit 中啟動任務。"
                }
            }
            .to_string(),
        );
    }
    if drift.dirty {
        actions.push(match locale {
            VibehubLocale::En => "Review diff, then ask the agent to summarize changed files and intent in the active phase output.",
            VibehubLocale::ZhCn => "检查 diff，然后让 agent 在当前阶段输出中总结变更文件和意图。",
            VibehubLocale::ZhTw => "檢查 diff，然後讓 agent 在目前階段輸出中總結變更檔案和意圖。",
        }.to_string());
    }
    if validation
        .map(|v| !v.missing_outputs.is_empty())
        .unwrap_or(false)
    {
        actions.push(
            match locale {
                VibehubLocale::En => {
                    "Complete the missing phase output sections before advancing phase."
                }
                VibehubLocale::ZhCn => "推进阶段前，先补齐缺失的阶段输出章节。",
                VibehubLocale::ZhTw => "推進階段前，先補齊缺失的階段輸出章節。",
            }
            .to_string(),
        );
    }
    if !questions.is_empty() {
        actions.push(match locale {
            VibehubLocale::En => "Ask the listed sync questions. If the user does not answer, record them as unresolved risks and continue from hard evidence.",
            VibehubLocale::ZhCn => "询问列出的同步问题；如果用户未回答，将其记录为未解决风险，并基于硬证据继续。",
            VibehubLocale::ZhTw => "詢問列出的同步問題；如果使用者未回答，將其記錄為未解決風險，並基於硬證據繼續。",
        }.to_string());
    }
    if actions.is_empty() {
        actions.push(
            match locale {
                VibehubLocale::En => "Continue from `.vibehub/agent-view/current.md`.",
                VibehubLocale::ZhCn => "从 `.vibehub/agent-view/current.md` 继续。",
                VibehubLocale::ZhTw => "從 `.vibehub/agent-view/current.md` 繼續。",
            }
            .to_string(),
        );
    }
    actions
}

fn render_report(
    locale: VibehubLocale,
    generated_at: &str,
    task_id: Option<&str>,
    run_id: Option<&str>,
    phase: Option<&str>,
    agent_view_status: &str,
    context_status: &str,
    adapter_status: &str,
    drift: &drift::WorkspaceDriftReport,
    validation: Option<&phase::PhaseValidationResult>,
    questions: &[String],
    actions: &[String],
) -> String {
    let mut output = String::new();
    output.push_str(&format!("# {}\n\n", sync_title(locale)));
    output.push_str(&format!("{}: {generated_at}\n", label_generated_at(locale)));
    output.push_str(&format!(
        "{}: VibeHub backend\n",
        label_generated_by(locale)
    ));
    output.push_str(&format!("{}: mixed\n\n", locale.evidence_grade_label()));

    output.push_str(&format!("## {}\n\n", section_current_state(locale)));
    output.push_str(&format!(
        "- {}: {}\n",
        label_task(locale),
        task_id.unwrap_or("none")
    ));
    output.push_str(&format!(
        "- {}: {}\n",
        label_run(locale),
        run_id.unwrap_or("none")
    ));
    output.push_str(&format!(
        "- {}: {}\n",
        label_phase(locale),
        phase.unwrap_or("none")
    ));
    output.push_str(&format!("- {}: {}\n", label_git_dirty(locale), drift.dirty));
    output.push_str(&format!(
        "- {}: {}\n\n",
        label_changed_files(locale),
        drift.changed_files.len()
    ));
    output.push_str(&format!(
        "{}: hard_observed\n\n",
        locale.evidence_grade_label()
    ));

    output.push_str(&format!("## {}\n\n", section_sync_actions(locale)));
    output.push_str(&format!(
        "- {}: {agent_view_status}\n",
        label_agent_view(locale)
    ));
    output.push_str(&format!("- {}: {context_status}\n", label_context(locale)));
    output.push_str(&format!(
        "- {}: {adapter_status}\n",
        label_adapter_sync(locale)
    ));
    if let Some(validation) = validation {
        output.push_str(&format!(
            "- {}: {} ({}: {})\n",
            label_phase_validation(locale),
            validation.status,
            label_missing(locale),
            validation.missing_outputs.join(", ")
        ));
    } else {
        output.push_str(&format!("- {}: skipped\n", label_phase_validation(locale)));
    }
    output.push_str(&format!("\n{}: mixed\n\n", locale.evidence_grade_label()));

    output.push_str(&format!("## {}\n\n", section_changed_files(locale)));
    if drift.changed_files.is_empty() {
        output.push_str(&format!("- {}\n", locale.none_observed()));
    } else {
        for file in &drift.changed_files {
            output.push_str(&format!("- {file}\n"));
        }
    }
    output.push_str(&format!(
        "\n{}: hard_observed\n\n",
        locale.evidence_grade_label()
    ));

    output.push_str(&format!("## {}\n\n", section_questions(locale)));
    if questions.is_empty() {
        output.push_str(&format!("- {}\n", locale.none()));
    } else {
        for question in questions {
            output.push_str(&format!("- {question}\n"));
        }
    }
    output.push_str(&format!(
        "\n{}: inferred\n\n",
        locale.evidence_grade_label()
    ));

    output.push_str(&format!("## {}\n\n", section_recommended_actions(locale)));
    for action in actions {
        output.push_str(&format!("- {action}\n"));
    }
    output.push_str(&format!(
        "\n{}: inferred\n\n",
        locale.evidence_grade_label()
    ));

    output.push_str(&format!("## {}\n\n", section_warnings(locale)));
    if drift.warnings.is_empty() {
        output.push_str(&format!("- {}\n", locale.none()));
    } else {
        for warning in &drift.warnings {
            output.push_str(&format!("- {warning}\n"));
        }
    }
    output.push_str(&format!("\n{}: mixed\n", locale.evidence_grade_label()));
    output
}

fn sync_title(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "VibeHub Sync Report",
        VibehubLocale::ZhCn => "VibeHub 同步报告",
        VibehubLocale::ZhTw => "VibeHub 同步報告",
    }
}

fn label_generated_at(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Generated at",
        VibehubLocale::ZhCn => "生成时间",
        VibehubLocale::ZhTw => "產生時間",
    }
}

fn label_generated_by(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Generated by",
        VibehubLocale::ZhCn => "生成来源",
        VibehubLocale::ZhTw => "產生來源",
    }
}

fn section_current_state(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Current State",
        VibehubLocale::ZhCn => "当前状态",
        VibehubLocale::ZhTw => "目前狀態",
    }
}

fn section_sync_actions(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Sync Actions",
        VibehubLocale::ZhCn => "同步动作",
        VibehubLocale::ZhTw => "同步動作",
    }
}

fn section_changed_files(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Changed Files",
        VibehubLocale::ZhCn => "变更文件",
        VibehubLocale::ZhTw => "變更檔案",
    }
}

fn section_questions(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Questions For User",
        VibehubLocale::ZhCn => "需要用户确认的问题",
        VibehubLocale::ZhTw => "需要使用者確認的問題",
    }
}

fn section_recommended_actions(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Recommended Actions",
        VibehubLocale::ZhCn => "建议动作",
        VibehubLocale::ZhTw => "建議動作",
    }
}

fn section_warnings(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Warnings",
        VibehubLocale::ZhCn => "警告",
        VibehubLocale::ZhTw => "警告",
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

fn label_git_dirty(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Git dirty",
        VibehubLocale::ZhCn => "Git 是否有未提交变更",
        VibehubLocale::ZhTw => "Git 是否有未提交變更",
    }
}

fn label_changed_files(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Changed files",
        VibehubLocale::ZhCn => "变更文件数",
        VibehubLocale::ZhTw => "變更檔案數",
    }
}

fn label_agent_view(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Agent view",
        VibehubLocale::ZhCn => "Agent 视图",
        VibehubLocale::ZhTw => "Agent 視圖",
    }
}

fn label_context(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Context",
        VibehubLocale::ZhCn => "上下文",
        VibehubLocale::ZhTw => "上下文",
    }
}

fn label_adapter_sync(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Adapter sync",
        VibehubLocale::ZhCn => "适配器同步",
        VibehubLocale::ZhTw => "適配器同步",
    }
}

fn label_phase_validation(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "Phase validation",
        VibehubLocale::ZhCn => "阶段验证",
        VibehubLocale::ZhTw => "階段驗證",
    }
}

fn label_missing(locale: VibehubLocale) -> &'static str {
    match locale {
        VibehubLocale::En => "missing",
        VibehubLocale::ZhCn => "缺失",
        VibehubLocale::ZhTw => "缺失",
    }
}

fn update_sync_state(
    project_root: &Path,
    synced_at: &str,
    report_path: Option<&str>,
    agent_view_sync_path: &str,
    drift: &drift::WorkspaceDriftReport,
    questions: &[String],
) -> Result<()> {
    let state_path = project_root.join(".vibehub/state.yaml");
    if !state_path.is_file() {
        return Ok(());
    }
    let content = fs::read_to_string(&state_path)
        .with_context(|| format!("Failed to read {}", state_path.display()))?;
    let mut state = serde_yaml::from_str::<Value>(&content)
        .with_context(|| format!("Invalid YAML in {}", state_path.display()))?;
    set_string(&mut state, &["sync", "last_synced_at"], synced_at);
    if let Some(report_path) = report_path {
        set_string(&mut state, &["sync", "last_report"], report_path);
        set_string(
            &mut state,
            &["agent_report", "last_sync_report"],
            report_path,
        );
    }
    set_string(
        &mut state,
        &["sync", "agent_view_sync"],
        agent_view_sync_path,
    );
    set_bool(&mut state, &["git", "dirty"], drift.dirty);
    set_usize(
        &mut state,
        &["git", "changed_files_count"],
        drift.changed_files.len(),
    );
    if let Some(head) = drift.head.as_deref() {
        set_string(&mut state, &["git", "last_seen_head"], head);
    }
    set_yaml_sequence(&mut state, &["sync", "questions"], questions);
    let loop_warnings = drift
        .warnings
        .iter()
        .filter(|warning| is_loop_warning(warning))
        .cloned()
        .collect::<Vec<_>>();
    set_string(
        &mut state,
        &["loop_detection", "status"],
        if loop_warnings.is_empty() {
            "normal"
        } else {
            "warning"
        },
    );
    set_yaml_sequence(&mut state, &["loop_detection", "warnings"], &loop_warnings);
    set_string(
        &mut state,
        &["last_updated"],
        &Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
    );
    let content = serde_yaml::to_string(&state).context("Failed to serialize state.yaml")?;
    fs::write(&state_path, content)
        .with_context(|| format!("Failed to write {}", state_path.display()))
}

fn set_string(value: &mut Value, path: &[&str], next: &str) {
    set_value(value, path, Value::String(next.to_string()));
}

fn set_bool(value: &mut Value, path: &[&str], next: bool) {
    set_value(value, path, Value::Bool(next));
}

fn set_usize(value: &mut Value, path: &[&str], next: usize) {
    set_value(value, path, Value::Number((next as u64).into()));
}

fn set_yaml_sequence(value: &mut Value, path: &[&str], next: &[String]) {
    let values = next.iter().cloned().map(Value::String).collect();
    set_value(value, path, Value::Sequence(values));
}

fn set_value(value: &mut Value, path: &[&str], next: Value) {
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

fn is_loop_warning(warning: &str) -> bool {
    warning.starts_with("Loop warning:")
        || warning.starts_with("循环警告：")
        || warning.starts_with("循環警告：")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dirty_drift() -> drift::WorkspaceDriftReport {
        drift::WorkspaceDriftReport {
            project_root: "project".to_string(),
            git_available: true,
            head: Some("head".to_string()),
            last_seen_head: Some("head".to_string()),
            head_changed: false,
            dirty: true,
            changed_files: vec!["src/main.rs".to_string(), "README.md".to_string()],
            context_stale: false,
            adapter_conflicts: Vec::new(),
            warnings: vec!["warning".to_string()],
            recommended_actions: Vec::new(),
            recover_report_path: None,
        }
    }

    #[test]
    fn sync_questions_use_simplified_chinese() {
        let questions = build_questions(
            VibehubLocale::ZhCn,
            Some("T-1"),
            Some("R-1"),
            &dirty_drift(),
            None,
            "updated:.vibehub/agent-view/current.md",
            "rebuilt:pack included=1 missing=0",
        );

        assert!(questions[0].contains("Git 当前有 2 个变更文件"));
        assert!(questions[0].contains("当前 VibeHub 任务"));
    }

    #[test]
    fn sync_report_uses_traditional_chinese_headings() {
        let drift = dirty_drift();
        let questions = build_questions(
            VibehubLocale::ZhTw,
            Some("T-1"),
            Some("R-1"),
            &drift,
            None,
            "updated:.vibehub/agent-view/current.md",
            "rebuilt:pack included=1 missing=0",
        );
        let actions =
            build_recommended_actions(VibehubLocale::ZhTw, Some("T-1"), &drift, None, &questions);

        let report = render_report(
            VibehubLocale::ZhTw,
            "2026-05-05T00:00:00Z",
            Some("T-1"),
            Some("R-1"),
            Some("implement"),
            "updated:.vibehub/agent-view/current.md",
            "rebuilt:pack included=1 missing=0",
            "ok",
            &drift,
            None,
            &questions,
            &actions,
        );

        assert!(report.contains("# VibeHub 同步報告"));
        assert!(report.contains("## 目前狀態"));
        assert!(report.contains("Git 目前有 2 個變更檔案"));
    }
}

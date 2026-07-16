use crate::vibehub::status::VibehubCockpitStatus;
use crate::vibehub::{status, util::canonical_project_root};
use anyhow::Result;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NextActionReport {
    pub project_root: String,
    pub action: String,
    pub skill: String,
    pub cli: String,
    pub reason: String,
    pub confidence: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_intent: Option<String>,
    pub operating_loop: Vec<&'static str>,
    pub routing_table: Vec<RouteRule>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RouteRule {
    pub when: &'static str,
    pub action: &'static str,
    pub skill: &'static str,
    pub cli: &'static str,
}

pub fn operating_loop() -> Vec<&'static str> {
    vec![
        "Read state first; do not trust memory.",
        "If request has independent deliverables, split tasks.",
        "If the user asks for meta-review, architecture critique, or protocol changes, treat it as new work unless already scoped to the current task.",
        "If workspace or phase is unclear, sync before edits.",
        "Work only inside the current task and phase.",
        "Use CLI for canonical state changes; never edit pointers/state directly.",
        "Write evidence-labeled output.md before stopping.",
        "Validate and lint output before asking to finish or advance.",
        "Report risks and next actions instead of hiding uncertainty.",
    ]
}

pub fn routing_table() -> Vec<RouteRule> {
    vec![
        RouteRule {
            when: "No .vibehub project state exists",
            action: "initialize_project",
            skill: "vibehub-init",
            cli: "vibehub init <project>",
        },
        RouteRule {
            when: "User asks to evaluate, redesign, or harden VibeHub/Agent protocol behavior",
            action: "start_task",
            skill: "vibehub-start",
            cli: "vibehub start <project> <mode> <title>",
        },
        RouteRule {
            when: "User request has multiple independently deliverable goals",
            action: "split_task_intake",
            skill: "vibehub-start-intake",
            cli: "vibehub start-intake <project> --stdin",
        },
        RouteRule {
            when: "No active task exists for the requested work",
            action: "start_task",
            skill: "vibehub-start",
            cli: "vibehub start <project> <mode> <title>",
        },
        RouteRule {
            when: "Workspace drift, unclear phase, continue, refresh, sync, 继续, or 同步",
            action: "sync_workspace",
            skill: "vibehub-sync",
            cli: "vibehub sync <project>",
        },
        RouteRule {
            when: "Active phase has missing context or missing output",
            action: "continue_phase",
            skill: "vibehub-continue",
            cli: "vibehub status <project>",
        },
        RouteRule {
            when: "Phase output appears ready",
            action: "validate_phase",
            skill: "vibehub-validate",
            cli: "vibehub validate <project>",
        },
        RouteRule {
            when: "Output exists but may contain stale, contradictory, or unlabeled evidence",
            action: "lint_output",
            skill: "vibehub-output-lint",
            cli: "vibehub output-lint <project> [task_id]",
        },
        RouteRule {
            when: "Phase is completed and user confirmed transition",
            action: "advance_phase",
            skill: "vibehub-advance",
            cli: "vibehub advance <project> --confirmed-by-user",
        },
        RouteRule {
            when: "State pointers or event projection are inconsistent",
            action: "recover_state",
            skill: "vibehub-recover",
            cli: "vibehub recover <project>",
        },
    ]
}

pub fn recommend_next_action(project_root: impl AsRef<Path>) -> Result<NextActionReport> {
    recommend_next_action_with_intent(project_root, None)
}

pub fn recommend_next_action_with_intent(
    project_root: impl AsRef<Path>,
    intent: Option<&str>,
) -> Result<NextActionReport> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    let cockpit = status::read_cockpit_status(&project_root)?;
    let (action, skill, cli, reason, confidence, matched_intent) = decide(&cockpit, intent);

    Ok(NextActionReport {
        project_root: cockpit.project_root,
        action: action.to_string(),
        skill: skill.to_string(),
        cli: cli.to_string(),
        reason,
        confidence: confidence.to_string(),
        matched_intent,
        operating_loop: operating_loop(),
        routing_table: routing_table(),
        warnings: cockpit.warnings,
    })
}

fn decide(
    cockpit: &VibehubCockpitStatus,
    intent: Option<&str>,
) -> (
    &'static str,
    &'static str,
    &'static str,
    String,
    &'static str,
    Option<String>,
) {
    if !cockpit.initialized {
        return (
            "initialize_project",
            "vibehub-init",
            "vibehub init <project>",
            ".vibehub project state is missing.".to_string(),
            "high",
            intent.map(ToString::to_string),
        );
    }

    if let Some(route) = route_from_intent(intent) {
        return route;
    }

    if cockpit.current_task_id.is_none() {
        return (
            "start_task",
            "vibehub-start",
            "vibehub start <project> <mode> <title>",
            "No active VibeHub task is configured for this work.".to_string(),
            "high",
            intent.map(ToString::to_string),
        );
    }

    if has_state_warning(cockpit) {
        return (
            "recover_state",
            "vibehub-recover",
            "vibehub recover <project>",
            "State or event consistency warnings are visible.".to_string(),
            "medium",
            intent.map(ToString::to_string),
        );
    }

    if cockpit.context_pack_status.configured
        && (!cockpit.context_pack_status.exists || cockpit.context_pack_status.stale == Some(true))
    {
        return (
            "sync_workspace",
            "vibehub-sync",
            "vibehub sync <project>",
            "Current context pack is missing or stale.".to_string(),
            "high",
            intent.map(ToString::to_string),
        );
    }

    match cockpit.phase_status.as_deref() {
        Some("active" | "running") => {
            if cockpit.agent_output_status.configured && cockpit.agent_output_status.exists {
                (
                    "validate_phase",
                    "vibehub-validate",
                    "vibehub validate <project>",
                    "Current phase is active and output.md exists; validate before finish/advance."
                        .to_string(),
                    "medium",
                    intent.map(ToString::to_string),
                )
            } else {
                (
                    "continue_phase",
                    "vibehub-continue",
                    "vibehub status <project>",
                    "Current phase is active and still needs phase output.".to_string(),
                    "high",
                    intent.map(ToString::to_string),
                )
            }
        }
        Some("needs_action" | "blocked" | "paused") => (
            "continue_phase",
            "vibehub-continue",
            "vibehub status <project>",
            "Current phase requires agent action before transition.".to_string(),
            "high",
            intent.map(ToString::to_string),
        ),
        Some("completed") => (
            "ask_to_advance",
            "vibehub-advance",
            "vibehub advance <project> --confirmed-by-user",
            "Current phase is completed; advance only after user confirmation.".to_string(),
            "high",
            intent.map(ToString::to_string),
        ),
        _ => (
            "sync_workspace",
            "vibehub-sync",
            "vibehub sync <project>",
            "Current phase status is unclear.".to_string(),
            "medium",
            intent.map(ToString::to_string),
        ),
    }
}

fn route_from_intent(
    intent: Option<&str>,
) -> Option<(
    &'static str,
    &'static str,
    &'static str,
    String,
    &'static str,
    Option<String>,
)> {
    let intent = intent?.trim();
    if intent.is_empty() {
        return None;
    }
    let lower = intent.to_lowercase();
    let matched = Some(intent.to_string());
    if contains_any(
        &lower,
        &[
            "agent",
            "代理",
            "智能体",
            "协议",
            "protocol",
            "adapter",
            "适配器",
            "暴露",
            "能力边界",
            "工具调用",
            "流程约束",
            "定位",
            "反思",
            "评价",
            "审视",
            "review the workflow",
            "operating loop",
        ],
    ) && contains_any(
        &lower,
        &[
            "vibehub",
            "vipp",
            "agent",
            "代理",
            "智能体",
            "工具",
            "workflow",
            "流程",
        ],
    ) {
        return Some((
            "start_task",
            "vibehub-start",
            "vibehub start <project> <mode> <title>",
            "User intent asks for meta-level evaluation or improvement of the VibeHub/Agent operating contract; treat it as new scoped work rather than validating the current phase.".to_string(),
            "high",
            matched,
        ));
    }
    if contains_any(
        &lower,
        &[
            "同步",
            "sync",
            "sycn",
            "刷新",
            "refresh",
            "继续",
            "continue",
            "update vibehub",
            "更新 vibehub",
        ],
    ) {
        return Some((
            "sync_workspace",
            "vibehub-sync",
            "vibehub sync <project>",
            "User intent asks to continue, refresh, or reconcile workspace state.".to_string(),
            "high",
            matched,
        ));
    }
    if contains_any(
        &lower,
        &["拆分", "多任务", "多个任务", "multi-intent", "split"],
    ) && !contains_any(
        &lower,
        &["validate-task", "错验", "multiple active", "多 active"],
    ) {
        return Some((
            "split_task_intake",
            "vibehub-start-intake",
            "vibehub start-intake <project> --stdin",
            "User intent suggests independently deliverable work should be split.".to_string(),
            "medium",
            matched,
        ));
    }
    if contains_any(
        &lower,
        &["多 active", "multiple active", "错验", "validate-task"],
    ) || (contains_any(&lower, &["多任务"])
        && contains_any(&lower, &["验证", "validate", "校验", "错"]))
    {
        return Some((
            "validate_task",
            "vibehub-validate",
            "vibehub validate-task <project> <task_id>",
            "User intent points at multi-task validation; prefer task-scoped validation to avoid the wrong current pointer.".to_string(),
            "high",
            matched,
        ));
    }
    if contains_any(
        &lower,
        &[
            "新任务",
            "新需求",
            "帮我做",
            "优化",
            "改进",
            "实现",
            "一次性",
            "全部优化",
            "build",
            "implement",
            "improve",
        ],
    ) && !contains_any(
        &lower,
        &["继续", "continue", "收口", "finish", "推进", "advance"],
    ) {
        return Some((
            "start_task",
            "vibehub-start",
            "vibehub start <project> <mode> <title>",
            "User intent appears to introduce new scoped work; create or switch to an appropriate VibeHub task before editing.".to_string(),
            "medium",
            matched,
        ));
    }
    if contains_any(
        &lower,
        &["切换", "switch", "current pointer", "当前任务", "错任务"],
    ) {
        return Some((
            "switch_or_validate_task",
            "vibehub-switch",
            "vibehub switch <project> <task_id>",
            "User intent mentions task selection or wrong-current-task risk; inspect status and switch or use task-scoped validation.".to_string(),
            "medium",
            matched,
        ));
    }
    if contains_any(&lower, &["lint", "质量", "自相矛盾", "证据标签", "output"]) {
        return Some((
            "lint_output",
            "vibehub-output-lint",
            "vibehub output-lint <project> [task_id]",
            "User intent asks to check output quality or evidence hygiene.".to_string(),
            "high",
            matched,
        ));
    }
    if contains_any(&lower, &["验证", "validate", "检查输出"]) {
        return Some((
            "validate_phase",
            "vibehub-validate",
            "vibehub validate <project>",
            "User intent asks to validate current phase output.".to_string(),
            "high",
            matched,
        ));
    }
    if contains_any(&lower, &["完成阶段", "finish", "收口"]) {
        return Some((
            "finish_phase",
            "vibehub-finish",
            "vibehub finish <project> --confirmed-by-user",
            "User intent asks to finish the current phase; CLI still requires explicit confirmation flag.".to_string(),
            "medium",
            matched,
        ));
    }
    if contains_any(&lower, &["推进", "advance", "下一阶段"]) {
        return Some((
            "advance_phase",
            "vibehub-advance",
            "vibehub advance <project> --confirmed-by-user",
            "User intent asks to move to the next phase; CLI still requires explicit confirmation flag.".to_string(),
            "medium",
            matched,
        ));
    }
    if contains_any(&lower, &["归档", "archive"]) {
        return Some((
            "archive_tasks",
            "vibehub-archive",
            "vibehub archive <project> --confirmed-by-user [task_id]",
            "User intent asks to archive terminal tasks; CLI still requires explicit confirmation flag.".to_string(),
            "medium",
            matched,
        ));
    }
    None
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn has_state_warning(cockpit: &VibehubCockpitStatus) -> bool {
    cockpit.warnings.iter().any(|warning| {
        let warning = warning.to_lowercase();
        warning.contains("consistency")
            || warning.contains("pointer")
            || warning.contains("invalid .vibehub/state.yaml")
            || warning.contains("missing or invalid .vibehub/state.yaml")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vibehub::{init, start_task};
    use std::fs;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn temp_project() -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("vibehub-next-action-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).expect("create temp project");
        path
    }

    #[test]
    fn recommends_init_for_uninitialized_project() {
        let project = temp_project();

        let report = recommend_next_action(&project).expect("next action");

        assert_eq!(report.action, "initialize_project");
        assert_eq!(report.skill, "vibehub-init");
        assert!(!report.operating_loop.is_empty());
        assert!(!report.routing_table.is_empty());

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn recommends_start_when_no_active_task_exists() {
        let project = temp_project();
        init::init_project(&project).expect("init");

        let report = recommend_next_action(&project).expect("next action");

        assert_eq!(report.action, "start_task");
        assert_eq!(report.skill, "vibehub-start");

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn recommends_continue_for_active_phase_without_output() {
        let project = temp_project();
        init::init_project(&project).expect("init");
        start_task::start_task(
            &project,
            Some("Next action task".to_string()),
            Some("guided_drive".to_string()),
            None,
        )
        .expect("start task");

        let report = recommend_next_action(&project).expect("next action");

        assert_eq!(report.action, "continue_phase");
        assert_eq!(report.skill, "vibehub-continue");

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn routes_explicit_sync_intent() {
        let project = temp_project();
        init::init_project(&project).expect("init");
        start_task::start_task(
            &project,
            Some("Intent task".to_string()),
            Some("guided_drive".to_string()),
            None,
        )
        .expect("start task");

        let report =
            recommend_next_action_with_intent(&project, Some("请同步当前状态")).expect("route");

        assert_eq!(report.action, "sync_workspace");
        assert_eq!(report.skill, "vibehub-sync");
        assert!(report.cli.starts_with("vibehub "));
        assert_eq!(report.matched_intent.as_deref(), Some("请同步当前状态"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn routes_continue_intent_to_sync_first() {
        let project = temp_project();
        init::init_project(&project).expect("init");
        start_task::start_task(
            &project,
            Some("Continue intent task".to_string()),
            Some("guided_drive".to_string()),
            None,
        )
        .expect("start task");

        let report =
            recommend_next_action_with_intent(&project, Some("继续当前状态")).expect("route");

        assert_eq!(report.action, "sync_workspace");
        assert_eq!(report.skill, "vibehub-sync");
        assert_eq!(report.cli, "vibehub sync <project>");

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn routes_meta_agent_protocol_intent_as_new_work() {
        let project = temp_project();
        init::init_project(&project).expect("init");
        start_task::start_task(
            &project,
            Some("Existing task".to_string()),
            Some("guided_drive".to_string()),
            None,
        )
        .expect("start task");

        let report = recommend_next_action_with_intent(
            &project,
            Some("深度评价当前项目面向 Agent 暴露的能力边界和工具调用流程"),
        )
        .expect("route");

        assert_eq!(report.action, "start_task");
        assert_eq!(report.skill, "vibehub-start");
        assert_eq!(report.cli, "vibehub start <project> <mode> <title>");

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn routes_continue_meta_agent_protocol_intent_as_new_work() {
        let project = temp_project();
        init::init_project(&project).expect("init");
        start_task::start_task(
            &project,
            Some("Existing task".to_string()),
            Some("guided_drive".to_string()),
            None,
        )
        .expect("start task");

        let report =
            recommend_next_action_with_intent(&project, Some("继续优化 Agent 协议和工具调用流程"))
                .expect("route");

        assert_eq!(report.action, "start_task");
        assert_eq!(report.skill, "vibehub-start");
        assert_eq!(report.cli, "vibehub start <project> <mode> <title>");

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn routes_multi_task_validation_to_validate_task() {
        let project = temp_project();
        init::init_project(&project).expect("init");

        let report = recommend_next_action_with_intent(
            &project,
            Some("多 active 任务时不要错验，请用 validate-task"),
        )
        .expect("route");

        assert_eq!(report.action, "validate_task");
        assert_eq!(report.skill, "vibehub-validate");
        assert_eq!(report.cli, "vibehub validate-task <project> <task_id>");

        fs::remove_dir_all(project).expect("cleanup");
    }
}

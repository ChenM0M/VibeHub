---
name: vibehub-sync
description: "Reconcile external workspace state with VibeHub. Use for VibeHub workflow step: vibehub-sync."
---

# vibehub-sync

中文: 将外部工作区变更与 VibeHub 状态对齐。
English: Reconcile external workspace state with VibeHub.

Invocation input: <project_root> [mode]

Read first / 先读:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

CLI:
  vibehub sync <project_path>   (accepts `sycn` typo)
  Returns: {status, sync_level, task_id, run_id, phase, drift_warnings, changed_files, questions_for_user, recommended_actions}
  After: sync report at .vibehub/agent-view/sync.md; follow recommended_actions.

Pre-flight / 前置检查:
  1. Check Git is available: `git status --porcelain`
  2. Read `.vibehub/agent-view/current.md` to find current task/run.
  3. Collect hard evidence BEFORE asking user questions.

Task / 任务:Run a best-effort sync via `vibehub sync <project>`. Inspect Git diff/status, VibeHub pointers, current context, latest output, and handoff. Accept `sycn` as typo alias.

Sync behavior:
- Treat "sync", "sycn", "同步", "刷新状态", "update VibeHub", or plain requests to continue from current engineering reality as this command.
- Autonomously collect hard evidence first: Git status/diff, current task/run/phase, context pack state, latest output, handoff, visible warnings.
- Ask user only for missing intent/progress/future-plan details not inferrable from hard evidence.
- If user does not answer, record questions as unresolved risks; continue from hard_observed evidence.

Stop when:
  1. Sync report at `.vibehub/agent-view/sync.md` is generated.
  2. All hard-evidence questions answered or recorded as unresolved risks.
  3. If state is broken, recommend vibehub-recover instead of silently advancing.

Output / 输出:
Write output to `.vibehub/tasks/<task_id>/runs/<run_id>/outputs/output.md` following `.vibehub/adapters/protocol.md`. Required sections (bilingual supported): Completed / 已完成, Not Yet Done / 未完成, Key Decisions Made / 关键决策, Files Changed / 变更文件, Files Reportedly Read / 已读文件, Commands Run / 执行命令, Tests Run / 测试, Context Still Needed / 仍需上下文, Warnings / 警告, Next Session Should / 后续应做。Use evidence labels.

**IMPORTANT: Output in Chinese (中文) unless user requests otherwise.**

Constraints / 约束:
- Do not edit `.vibehub/state.yaml` or canonical task/run pointers directly. / 不要直接编辑。
- Never manually create `.vibehub/tasks/` directories. / 不要手动创建目录，使用 CLI。
- NEVER run `vibehub finish` or `vibehub advance` without user confirmation. / 未经确认绝不运行 finish/advance。
- If CLI unavailable, ask user to run command. / 如 CLI 不可用请用户执行。
- Run vibehub-sync first if state is stale or drifted. / 先运行 vibehub-sync。

Registry contract:
- name: `vibehub-sync`
- args: `<project_root> [mode]`
- returns: `skill_response_schema_v1`
- callable_by: main-agent, sub-agent
- side_effects: writes_events, rebuilds_context, may_write_adapter_files
- idempotent: true

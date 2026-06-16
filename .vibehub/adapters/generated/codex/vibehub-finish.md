---
name: vibehub-finish
description: "Close a task after task_finishable gate passes. Use for VibeHub workflow step: vibehub-finish."
---

# vibehub-finish

中文: 完成当前工作并建议状态流转。
English: Close a task after task_finishable gate passes.

Invocation input: <project_root> [task_id]

Read first / 先读:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

CLI:
  vibehub finish <project_path> --confirmed-by-user
  Returns: {previous_phase, previous_status, current_phase, current_status, next_phase, validation}
  After: phase validated (required outputs checked); next run `vibehub advance --confirmed-by-user`.

Pre-flight / 前置检查:
  1. Verify output.md exists at `.vibehub/tasks/<task_id>/runs/<run_id>/outputs/output.md`.
  2. Run `vibehub validate <project>` and `vibehub output-lint <project>` before finishing.
  3. Run `vibehub finish <project> --confirmed-by-user` only after the user has confirmed.

Task / 任务:Complete the current phase after explicit user confirmation. The CLI validates required outputs against the phase contract. On success, the phase is marked completed; next step is `vibehub advance --confirmed-by-user` to move to the next phase.

Stop when:
  1. `vibehub validate` returns status=completed with no missing_outputs.
  2. `vibehub output-lint` has no severity=error issues.
  3. Handoff is complete or missing handoff context is documented in output.md.

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
- name: `vibehub-finish`
- args: `<project_root> [task_id]`
- returns: `skill_response_schema_v1`
- callable_by: main-agent
- side_effects: writes_events, updates_projection
- idempotent: false

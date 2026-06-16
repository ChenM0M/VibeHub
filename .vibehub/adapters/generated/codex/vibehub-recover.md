---
name: vibehub-recover
description: "Re-align after interruption or inconsistent state. Use for VibeHub workflow step: vibehub-recover."
---

# vibehub-recover

中文: 在漂移、HEAD 变化或中断后生成恢复报告。
English: Re-align after interruption or inconsistent state.

Invocation input: <project_root> [reason]

Read first / 先读:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

CLI:
  vibehub recover <project_path>
  Returns: {project_root, drift_warnings, changed_files, recommended_actions}

Pre-flight / 前置检查:
  1. Check if VibeHub state.yaml exists and has valid schema_version.
  2. Check Git HEAD matches VibeHub recorded HEAD.
  3. Verify task/run pointer files (`.vibehub/tasks/current`, `.vibehub/tasks/<id>/runs/current`) are valid.

Task / 任务:Analyze interrupted or drifted work. Check: (1) Git HEAD vs VibeHub pointers, (2) task/run pointer files integrity, (3) state.yaml vs event log consistency, (4) context pack freshness. Recommend vibehub-sync for minor drift or vibehub-start for broken pointers.

Stop when:
  1. Drift analysis is complete.
  2. Recommended actions are clear (sync, recover, or start new task).
  3. If pointers are broken, do NOT manually fix — recommend vibehub-start.

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
- name: `vibehub-recover`
- args: `<project_root> [reason]`
- returns: `skill_response_schema_v1`
- callable_by: main-agent
- side_effects: writes_events, rebuilds_context
- idempotent: true

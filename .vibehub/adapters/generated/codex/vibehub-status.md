---
name: vibehub-status
description: "Read projected task, run, and capability status. Use for VibeHub workflow step: vibehub-status."
---

# vibehub-status

中文: 读取当前任务、阶段、上下文、Git 和 handoff 状态。
English: Read projected task, run, and capability status.

Invocation input: <project_root>

Read first / 先读:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

CLI:
  vibehub status <project_path>
  Returns: {current_task_id, current_task_title, current_phase, phase_status, active_tasks, active_capabilities, flow, gate_statuses, warnings}

Pre-flight / 前置检查:
  (none)

Task / 任务:Read current state from `vibehub status <project>`. Summarize: active task/run/phase, flow (all phases and their statuses), context pack availability, handoff, Git status, active capabilities, claimable capabilities, gate statuses, and warnings.

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
- name: `vibehub-status`
- args: `<project_root>`
- returns: `skill_response_schema_v1`
- callable_by: main-agent
- side_effects: none
- idempotent: true

---
name: vibehub-archive
description: "Archive completed tasks. Use for VibeHub workflow step: vibehub-archive."
---

# vibehub-archive

中文: 归档已完成的任务。
English: Archive completed tasks.

Invocation input: [task_id]

Read first / 先读:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

CLI:
  vibehub archive <project_path> --confirmed-by-user [task_id]
  Moves completed tasks out of the active list so they appear in archive.

Pre-flight / 前置检查:
  1. Run `vibehub status <project>` to see all active tasks and their phase_status.
  2. Identify tasks with phase_status=completed or cancelled.
  3. Run `vibehub archive <project> --confirmed-by-user` only after the user has confirmed.

Task / 任务:Archive completed or cancelled tasks to clean up the active task list after explicit user confirmation. Without a task_id, archives all tasks that are in completed or cancelled status.

Output / 输出:
Write output to `.vibehub/tasks/<task_id>/runs/<run_id>/outputs/output.md` following `.vibehub/adapters/protocol.md`. Required sections (bilingual supported): Completed / 已完成, Not Yet Done / 未完成, Key Decisions Made / 关键决策, Files Changed / 变更文件, Files Reportedly Read / 已读文件, Commands Run / 执行命令, Tests Run / 测试, Context Still Needed / 仍需上下文, Warnings / 警告, Next Session Should / 后续应做。Use evidence labels.

**IMPORTANT: Output in Chinese (中文) unless user requests otherwise.**

Constraints / 约束:
- Do not edit `.vibehub/state.yaml` or canonical task/run pointers directly. / 不要直接编辑。
- Never manually create `.vibehub/tasks/` directories. / 不要手动创建目录，使用 CLI。
- NEVER run `vibehub finish` or `vibehub advance` without user confirmation. / 未经确认绝不运行 finish/advance。
- If CLI unavailable, ask user to run command. / 如 CLI 不可用请用户执行。
- Run vibehub-sync first if state is stale or drifted. / 先运行 vibehub-sync。

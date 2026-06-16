---
name: vibehub-start
description: "Create one or more VibeHub tasks; split multi-intent or independently deliverable requests before starting work. Use for VibeHub workflow step: vibehub-start."
---

# vibehub-start

中文: 根据用户请求创建任务；复杂需求先拆成独立 Task。
English: Create one or more VibeHub tasks; split multi-intent or independently deliverable requests before starting work.

Invocation input: <project_root> [title] [intent] [mode] [intake]

Read first / 先读:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

CLI:
  vibehub start <project_path> <mode> <title>
  Returns: {task_id, run_id, mode, phase, phase_status, task_path, run_path}
  After: task is auto-registered as active; proceed to vibehub-continue or vibehub-sync. For multi-intent requests, use vibehub-start-intake first.

Pre-flight / 前置检查:
  1. The VibeHub CLI is the same `vibehub` binary as the desktop app. Verify it runs headless with `vibehub status <project>` (it prints JSON and exits; it does not open a window). If the binary is missing, the user must install VibeHub; do not fall back to editing `.vibehub/` files by hand.
  2. Check for existing active task: `vibehub status <project>`. If one exists, consider vibehub-sync first.
  3. NEVER manually create `.vibehub/tasks/` directories or edit `state.yaml` directly.

Task / 任务:Create one or more VibeHub tasks. Split multi-intent requests into separate task drafts when requirements are independently deliverable; use vibehub-start-intake first when the request contains multiple deliverables.

Modes: yolo_drive (align_lite → implement → review_lite), guided_drive (align → plan → implement → review), evidence_drive (align → research → plan → implement → review).

After creation, the task is auto-registered as active. Proceed to vibehub-continue to start work on the align phase.

Stop when:
  1. CLI returns task_id and run_id successfully.
  2. Task appears in `vibehub status` output as active.
  3. Context pack is available for the first phase (usually align).

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
- name: `vibehub-start`
- args: `<project_root> [title] [intent] [mode] [intake]`
- returns: `skill_response_schema_v1`
- callable_by: main-agent
- side_effects: writes_events, writes_task_state, builds_context_pack
- idempotent: false

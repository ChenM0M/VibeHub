---
name: vibehub-handoff
description: "Write a durable handoff. Use for VibeHub workflow step: vibehub-handoff."
---

# vibehub-handoff

中文: 生成 session handoff notes。
English: Write a durable handoff.

Invocation input: <project_root> [capability] <summary> [next_steps]

Read first / 先读:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

CLI:
  vibehub handoff <project_path>
  Returns: {handoff_path, complete, missing_required_sections}

Pre-flight / 前置检查:
  (none)

Task / 任务:Build a session handoff that lets the next session resume without chat history. Reads the latest output.md and summarizes what was done, what remains, and next steps.

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
- name: `vibehub-handoff`
- args: `<project_root> [capability] <summary> [next_steps]`
- returns: `skill_response_schema_v1`
- callable_by: main-agent, sub-agent
- side_effects: writes_events, writes_handoff
- idempotent: false

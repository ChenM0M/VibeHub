---
name: vibehub-release
description: "Release an active capability and produce handoff state. Use for VibeHub workflow step: vibehub-release."
---

# vibehub-release

中文: 执行 vibehub-release skill。
English: Release an active capability and produce handoff state.

Invocation input: <project_root> <capability> <outcome> [task_pack_dirty]

Read first / 先读:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

Skill contract:
- name: `vibehub-release`
- description: Release an active capability and produce handoff state.
- returns: `skill_response_schema_v1`
- callable_by: main-agent
- side_effects: writes_events, writes_handoff, may_rebuild_task_pack
- idempotent: false

Arguments:
- `project_root`: required, type=string
- `capability`: required, type=string
- `outcome`: required, type=enum[completed,paused,failed,cancelled]
- `task_pack_dirty`: optional, type=boolean, default=false

Task / 任务:
Use the VibeHub CLI or app command surface for `vibehub-release` when available. Keep changes scoped to the active task and follow the shared output contract.

Output / 输出:
Write the phase output following the contract in `.vibehub/adapters/protocol.md`. Required sections (bilingual): Completed / 已完成, Not Yet Done / 未完成, Key Decisions Made / 关键决策, Files Changed / 变更文件, Files Reportedly Read / 已读文件, Commands Run / 执行命令, Tests Run / 测试, Context Still Needed / 仍需上下文, Warnings / 警告, Next Session Should / 后续应做。Use evidence labels. **Output in Chinese (中文) unless user requests otherwise.**

Constraints / 约束:
- Do not edit `.vibehub/state.yaml`. / 不要编辑。
- Do not claim runtime observation unless a runtime adapter captured it.
- Adapter writes are the only INV-6 UI-write exception.

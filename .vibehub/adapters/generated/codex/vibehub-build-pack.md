---
name: vibehub-build-pack
description: "Build task or capability Context Pack. Use for VibeHub workflow step: vibehub-build-pack."
---

# vibehub-build-pack

中文: 执行 vibehub-build-pack skill。
English: Build task or capability Context Pack.

Invocation input: <project_root> <kind> [capability]

Read first / 先读:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

Skill contract:
- name: `vibehub-build-pack`
- description: Build task or capability Context Pack.
- returns: `skill_response_schema_v1`
- callable_by: sub-agent
- side_effects: writes_context_pack, writes_events
- idempotent: true

Arguments:
- `project_root`: required, type=string
- `kind`: required, type=enum[task,capability]
- `capability`: optional, type=string

Task / 任务:
Use the VibeHub CLI or app command surface for `vibehub-build-pack` when available. Keep changes scoped to the active task and follow the shared output contract.

Output / 输出:
Write the phase output following the contract in `.vibehub/adapters/protocol.md`. Required sections (bilingual): Completed / 已完成, Not Yet Done / 未完成, Key Decisions Made / 关键决策, Files Changed / 变更文件, Files Reportedly Read / 已读文件, Commands Run / 执行命令, Tests Run / 测试, Context Still Needed / 仍需上下文, Warnings / 警告, Next Session Should / 后续应做。Use evidence labels. **Output in Chinese (中文) unless user requests otherwise.**

Constraints / 约束:
- Do not edit `.vibehub/state.yaml`. / 不要编辑。
- Do not claim runtime observation unless a runtime adapter captured it.
- Adapter writes are the only INV-6 UI-write exception.

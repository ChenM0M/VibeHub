---
name: vibehub-next-action
description: "Recommend the next agent action, skill, and CLI command from current VibeHub state. Use for VibeHub workflow step: vibehub-next-action."
---

# vibehub-next-action

中文: 根据当前状态推荐下一步 Agent 动作。
English: Recommend the next agent action, skill, and CLI command from current VibeHub state.

Invocation input: <project_root> [intent]

Read first / 先读:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

CLI:
  vibehub next-action <project_path> [intent...]
  Returns: {action, skill, cli, reason, confidence, matched_intent, operating_loop, routing_table, warnings}

Pre-flight / 前置检查:
  1. Run `vibehub next-action <project> [intent...]` before choosing a workflow command when state or intent is unclear.
  2. Follow the returned `skill` and `cli` unless the user explicitly overrides it.

Task / 任务:Ask VibeHub to recommend the next agent action. Returns a machine-readable action, skill name, CLI command, reason, confidence, matched intent, operating loop, routing table, and warnings. Use this when the agent is unsure whether to start, split, sync, continue, validate, lint output, advance, archive, or recover.

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
- name: `vibehub-next-action`
- args: `<project_root> [intent]`
- returns: `skill_response_schema_v1`
- callable_by: main-agent, sub-agent
- side_effects: none
- idempotent: true

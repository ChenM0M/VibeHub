---
name: vibehub-advance
description: "Advance to the next workflow phase. Use for VibeHub workflow step: vibehub-advance."
---

# vibehub-advance

中文: 推进到下一个工作流阶段。
English: Advance to the next workflow phase.

Invocation input: --confirmed-by-user [--force]

Read first / 先读:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

CLI:
  vibehub-cli advance <project_path> --confirmed-by-user [--force]
  Returns: {previous_phase, current_phase, next_phase, validation, handoff_complete}
  After: new phase context pack auto-built; resume with vibehub-continue.

Pre-flight / 前置检查:
  1. Run `vibehub-cli validate <project>` and `vibehub-cli output-lint <project>`.
  2. Run `vibehub-cli handoff <project>` to check handoff is complete.
  3. Run `vibehub-cli advance <project> --confirmed-by-user`; use --force only with user confirmation.

Task / 任务:Advance to the next phase in the workflow after explicit user confirmation. Requires all current phase outputs to be valid. If handoff is incomplete, advance is blocked unless --force is used.

Output / 输出:
Write output to `.vibehub/tasks/<task_id>/runs/<run_id>/outputs/output.md` following `.vibehub/adapters/protocol.md`. Required sections (bilingual supported): Completed / 已完成, Not Yet Done / 未完成, Key Decisions Made / 关键决策, Files Changed / 变更文件, Files Reportedly Read / 已读文件, Commands Run / 执行命令, Tests Run / 测试, Context Still Needed / 仍需上下文, Warnings / 警告, Next Session Should / 后续应做。Use evidence labels.

**IMPORTANT: Output in Chinese (中文) unless user requests otherwise.**

Constraints / 约束:
- Do not edit `.vibehub/state.yaml` or canonical task/run pointers directly. / 不要直接编辑。
- Never manually create `.vibehub/tasks/` directories. / 不要手动创建目录，使用 CLI。
- NEVER run `vibehub-cli finish` or `vibehub-cli advance` without user confirmation. / 未经确认绝不运行 finish/advance。
- If CLI unavailable, ask user to run command. / 如 CLI 不可用请用户执行。
- Run vibehub-sync first if state is stale or drifted. / 先运行 vibehub-sync。

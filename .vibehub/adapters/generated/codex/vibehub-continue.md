---
name: vibehub-continue
description: "Continue the active phase from VibeHub state and complete output.md. Use for VibeHub workflow step: vibehub-continue."
---

# vibehub-continue

中文: 从 VibeHub 当前状态继续阶段工作，并补齐 output.md。
English: Continue the active phase from VibeHub state and complete output.md.

Invocation input: [instruction]

Read first / 先读:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

CLI:
  * No direct CLI action. Read vibehub status for current state. Run vibehub sync first if needed.
  * Use vibehub validate to check outputs, vibehub finish to complete phase, vibehub advance to move forward.

Pre-flight / 前置检查:
  1. Run `vibehub sync <project>` first to align workspace state.
  2. Read `.vibehub/agent-view/current.md` for current phase.
  3. Check `.vibehub/workflow.yaml` capabilities.<phase>.required_fields for expected outputs.
  4. Read the context pack at the path in current-context.md.

Task / 任务:Continue the active phase from VibeHub state, not memory. Run `vibehub status` if phase/status is unclear; run vibehub-sync first when the user says continue/继续/refresh or when drift is visible. Check workflow.yaml required outputs, read the context pack, keep changes scoped, and write output.md before stopping.

Stop when:
  1. All phase required outputs are written to output.md (check workflow.yaml).
  2. Phase acceptance criteria are met.
  3. If blocked, report the blocker; write partial output with what's done.

Output / 输出:
Write output to `.vibehub/tasks/<task_id>/runs/<run_id>/outputs/output.md` following `.vibehub/adapters/protocol.md`. Required sections (bilingual supported): Completed / 已完成, Not Yet Done / 未完成, Key Decisions Made / 关键决策, Files Changed / 变更文件, Files Reportedly Read / 已读文件, Commands Run / 执行命令, Tests Run / 测试, Context Still Needed / 仍需上下文, Warnings / 警告, Next Session Should / 后续应做。Use evidence labels.

**IMPORTANT: Output in Chinese (中文) unless user requests otherwise.**

Constraints / 约束:
- Do not edit `.vibehub/state.yaml` or canonical task/run pointers directly. / 不要直接编辑。
- Never manually create `.vibehub/tasks/` directories. / 不要手动创建目录，使用 CLI。
- NEVER run `vibehub finish` or `vibehub advance` without user confirmation. / 未经确认绝不运行 finish/advance。
- If CLI unavailable, ask user to run command. / 如 CLI 不可用请用户执行。
- Run vibehub-sync first if state is stale or drifted. / 先运行 vibehub-sync。

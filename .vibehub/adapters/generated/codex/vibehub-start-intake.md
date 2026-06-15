---
name: vibehub-start-intake
description: "Analyze complex requests and split into independent tasks. Use for VibeHub workflow step: vibehub-start-intake."
---

# vibehub-start-intake

中文: 分析复杂需求并拆分为多个独立 Task。
English: Analyze complex requests and split into independent tasks.

Invocation input: <source_message> [mode]

Read first / 先读:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

CLI:
  Prepare JSON request following the intake schema, then: vibehub-cli start-intake <project_path> --stdin
  Alternative: vibehub-cli start-intake <project_path> <json_path>
  JSON schema: {title, intent, source_message, split_confidence, split_reason, intake: [{title, intent, acceptance_criteria, dependencies, suggested_order}]}
  Returns: {created_tasks, proposed_tasks, active_tasks, current_task_id}

Pre-flight / 前置检查:
  1. Identify independently deliverable units in the user request.
  2. For each unit, draft: title, intent, acceptance_criteria.
  3. Determine split_confidence: high if clearly independent, medium if user should confirm.
  4. Pipe the JSON to `vibehub-cli start-intake <project> --stdin` (or use a temp JSON file if stdin is unavailable).

Task / 任务:Analyze a complex user request and split it into independent, deliverable tasks. Use when the user asks for multiple things in one message.

Process: (1) Identify independently deliverable units. (2) Draft each as a task with title, intent, acceptance_criteria. (3) Assign suggested_order and dependencies. (4) Run `vibehub-cli start-intake <project> --stdin` with the JSON request, or pass a JSON file path when stdin is unavailable.

Split confidence: high=clearly independent, medium=user should confirm, low=collapse into one task.

Output / 输出:
Write output to `.vibehub/tasks/<task_id>/runs/<run_id>/outputs/output.md` following `.vibehub/adapters/protocol.md`. Required sections (bilingual supported): Completed / 已完成, Not Yet Done / 未完成, Key Decisions Made / 关键决策, Files Changed / 变更文件, Files Reportedly Read / 已读文件, Commands Run / 执行命令, Tests Run / 测试, Context Still Needed / 仍需上下文, Warnings / 警告, Next Session Should / 后续应做。Use evidence labels.

**IMPORTANT: Output in Chinese (中文) unless user requests otherwise.**

Constraints / 约束:
- Do not edit `.vibehub/state.yaml` or canonical task/run pointers directly. / 不要直接编辑。
- Never manually create `.vibehub/tasks/` directories. / 不要手动创建目录，使用 CLI。
- NEVER run `vibehub-cli finish` or `vibehub-cli advance` without user confirmation. / 未经确认绝不运行 finish/advance。
- If CLI unavailable, ask user to run command. / 如 CLI 不可用请用户执行。
- Run vibehub-sync first if state is stale or drifted. / 先运行 vibehub-sync。

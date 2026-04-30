---
name: vibehub-plan
description: "Create or repair implementation and validation plans. Use for VibeHub workflow step: vibehub-plan."
---

# vibehub-plan

中文: 创建或修复实现与验证计划。
English: Create or repair implementation and validation plans.

Invocation input: [goal]

中文: 创建或修复实现与验证计划。
English: Create or repair implementation and validation plans.

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`

Task:
Create or repair the implementation plan, validation plan, risk list, and context plan.

Output requirements:
- changed files, if any
- files read
- commands run
- tests run, or reason not run
- evidence labels: `hard_observed`, `agent_reported`, `inferred`, `user_confirmed`
- unresolved risks
- handoff notes or recommended VibeHub action

Constraints:
- Do not edit `.vibehub/state.yaml` or canonical task/run pointers directly.
- Do not claim runtime observation unless a runtime adapter captured it.
- If state is stale or drifted, report it and recommend VibeHub sync/recover instead of silently advancing state.


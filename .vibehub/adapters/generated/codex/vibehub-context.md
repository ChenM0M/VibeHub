---
name: vibehub-context
description: "Inspect or propose context for the current phase. Use for VibeHub workflow step: vibehub-context."
---

# vibehub-context

中文: 检查或建议当前阶段上下文。
English: Inspect or propose context for the current phase.

Invocation input: [phase]

中文: 检查或建议当前阶段上下文。
English: Inspect or propose context for the current phase.

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`

Task:
Inspect current context quality and propose missing files or stale context rebuilds.

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


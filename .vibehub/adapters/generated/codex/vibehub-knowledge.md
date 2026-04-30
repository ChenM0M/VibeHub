---
name: vibehub-knowledge
description: "Promote repeated learnings into rules or knowledge. Use for VibeHub workflow step: vibehub-knowledge."
---

# vibehub-knowledge

中文: 将重复经验沉淀为规则或知识。
English: Promote repeated learnings into rules or knowledge.

Invocation input: [lesson]

中文: 将重复经验沉淀为规则或知识。
English: Promote repeated learnings into rules or knowledge.

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`

Task:
Promote repeated lessons into reusable rules, preferences, or knowledge notes.

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


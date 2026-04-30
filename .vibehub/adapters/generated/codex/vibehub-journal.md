---
name: vibehub-journal
description: "Draft durable session journal notes. Use for VibeHub workflow step: vibehub-journal."
---

# vibehub-journal

中文: 草拟可沉淀的 session journal notes。
English: Draft durable session journal notes.

Invocation input: [note]

中文: 草拟可沉淀的 session journal notes。
English: Draft durable session journal notes.

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`

Task:
Draft durable session notes suitable for VibeHub journal promotion.

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


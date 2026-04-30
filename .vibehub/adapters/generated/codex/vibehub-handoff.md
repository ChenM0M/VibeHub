---
name: vibehub-handoff
description: "Build session handoff notes. Use for VibeHub workflow step: vibehub-handoff."
---

# vibehub-handoff

中文: 生成 session handoff notes。
English: Build session handoff notes.

Invocation input: [note]

中文: 生成 session handoff notes。
English: Build session handoff notes.

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`

Task:
Create handoff notes that let the next session resume without chat history.

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


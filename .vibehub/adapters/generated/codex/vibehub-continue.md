---
name: vibehub-continue
description: "Continue the active VibeHub phase. Use for VibeHub workflow step: vibehub-continue."
---

# vibehub-continue

中文: 继续当前 VibeHub 阶段。
English: Continue the active VibeHub phase.

Invocation input: [instruction]

中文: 继续当前 VibeHub 阶段。
English: Continue the active VibeHub phase.

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`

Task:
Continue the active phase using current.md and the context pack. Keep changes scoped to the active task.

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


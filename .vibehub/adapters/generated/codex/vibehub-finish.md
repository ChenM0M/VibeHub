---
name: vibehub-finish
description: "Finish current work and propose state transition. Use for VibeHub workflow step: vibehub-finish."
---

# vibehub-finish

中文: 完成当前工作并建议状态流转。
English: Finish current work and propose state transition.

Invocation input: [summary]

中文: 完成当前工作并建议状态流转。
English: Finish current work and propose state transition.

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`

Task:
Summarize completion evidence and recommend whether VibeHub should advance state, request review, or recover.

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


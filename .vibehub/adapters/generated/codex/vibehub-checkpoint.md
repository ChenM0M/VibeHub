---
name: vibehub-checkpoint
description: "Record progress, commands, risks, and next steps. Use for VibeHub workflow step: vibehub-checkpoint."
---

# vibehub-checkpoint

中文: 记录进展、命令、风险和下一步。
English: Record progress, commands, risks, and next steps.

Invocation input: [note]

中文: 记录进展、命令、风险和下一步。
English: Record progress, commands, risks, and next steps.

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`

Task:
Capture progress, decisions, commands, tests, changed files, risks, and next steps without marking state complete.

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


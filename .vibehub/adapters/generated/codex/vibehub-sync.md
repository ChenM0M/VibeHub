---
name: vibehub-sync
description: "Reconcile external workspace changes with VibeHub state. Use for VibeHub workflow step: vibehub-sync."
---

# vibehub-sync

中文: 将外部工作区变更与 VibeHub 状态对齐。
English: Reconcile external workspace changes with VibeHub state.

Invocation input: [scope]

中文: 将外部工作区变更与 VibeHub 状态对齐。
English: Reconcile external workspace changes with VibeHub state.

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`

Task:
Inspect Git diff/status and VibeHub pointers. Produce a sync report with observed drift, suspected cause, and recommended VibeHub action.

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


---
name: vibehub-diff
description: "Summarize current Git diff and task-scope drift. Use for VibeHub workflow step: vibehub-diff."
---

# vibehub-diff

中文: 汇总当前 Git diff 和任务范围漂移。
English: Summarize current Git diff and task-scope drift.

Invocation input: [focus]

中文: 汇总当前 Git diff 和任务范围漂移。
English: Summarize current Git diff and task-scope drift.

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`

Task:
Summarize changed files and scope drift against the current task and context pack.

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


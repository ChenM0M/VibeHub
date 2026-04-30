---
name: vibehub-help
description: "Show VibeHub command index. Use for VibeHub workflow step: vibehub-help."
---

# vibehub-help

中文: 显示 VibeHub 命令索引。
English: Show VibeHub command index.

Invocation input: 

中文: 显示 VibeHub 命令索引。
English: Show VibeHub command index.

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`

Task:
List available VibeHub commands and explain when to use each one.

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


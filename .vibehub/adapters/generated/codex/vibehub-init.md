---
name: vibehub-init
description: "Check whether this project is connected to VibeHub. Use for VibeHub workflow step: vibehub-init."
---

# vibehub-init

中文: 检查项目是否已连接 VibeHub。
English: Check whether this project is connected to VibeHub.

Invocation input: 

中文: 检查项目是否已连接 VibeHub。
English: Check whether this project is connected to VibeHub.

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`

Task:
Check whether `.vibehub/` exists. If it is missing, ask the user to initialize from the VibeHub app; do not create canonical state yourself.

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


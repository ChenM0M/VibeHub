---
name: vibehub-start
description: "Draft a VibeHub task from the user request. Use for VibeHub workflow step: vibehub-start."
---

# vibehub-start

中文: 根据用户请求草拟 VibeHub 任务。
English: Draft a VibeHub task from the user request.

Invocation input: <request>

中文: 根据用户请求草拟 VibeHub 任务。
English: Draft a VibeHub task from the user request.

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`

Task:
Convert the user's request into a VibeHub task draft with goal, acceptance criteria, mode suggestion, and context candidates.

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


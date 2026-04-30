---
name: vibehub-status
description: "Read current task, phase, context, Git, and handoff status. Use for VibeHub workflow step: vibehub-status."
---

# vibehub-status

中文: 读取当前任务、阶段、上下文、Git 和 handoff 状态。
English: Read current task, phase, context, Git, and handoff status.

Invocation input: 

中文: 读取当前任务、阶段、上下文、Git 和 handoff 状态。
English: Read current task, phase, context, Git, and handoff status.

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`

Task:
Summarize current task, run, phase, context pack, handoff, Git status, and visible warnings.

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


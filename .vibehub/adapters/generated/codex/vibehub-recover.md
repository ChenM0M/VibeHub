---
name: vibehub-recover
description: "Build a recovery report after drift, HEAD changes, or interruption. Use for VibeHub workflow step: vibehub-recover."
---

# vibehub-recover

中文: 在漂移、HEAD 变化或中断后生成恢复报告。
English: Build a recovery report after drift, HEAD changes, or interruption.

Invocation input: [symptom]

中文: 在漂移、HEAD 变化或中断后生成恢复报告。
English: Build a recovery report after drift, HEAD changes, or interruption.

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`

Task:
Analyze interrupted or drifted work and produce a recover report with safe next actions.

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


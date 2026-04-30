---
name: vibehub-review
description: "Review using diff, context, and research evidence. Use for VibeHub workflow step: vibehub-review."
---

# vibehub-review

中文: 基于 diff、context 和 research evidence 进行 review。
English: Review using diff, context, and research evidence.

Invocation input: [focus]

中文: 基于 diff、context 和 research evidence 进行 review。
English: Review using diff, context, and research evidence.

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`

Task:
Review the current diff against context, plan, research, tests, and VibeHub hard rules.

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


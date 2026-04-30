---
name: vibehub-research
description: "Run evidence-backed research and produce research notes. Use for VibeHub workflow step: vibehub-research."
---

# vibehub-research

中文: 执行有证据支撑的研究并产出 research notes。
English: Run evidence-backed research and produce research notes.

Invocation input: <question>

中文: 执行有证据支撑的研究并产出 research notes。
English: Run evidence-backed research and produce research notes.

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`

Task:
Run evidence-backed research, cite sources when external facts are used, and write research notes for VibeHub review.

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


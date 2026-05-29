---
name: vibehub-journal
description: "Record durable decisions and rationale. Use for VibeHub workflow step: vibehub-journal."
---

# vibehub-journal

中文: 草拟可沉淀的 session journal notes。
English: Record durable decisions and rationale.

Invocation input: <project_root> <decision> [rationale] [refs]

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

Task:
Draft durable session notes suitable for VibeHub journal promotion.



Output requirements:
- write the active run phase output before ending work:
  `.vibehub/tasks/<task_id>/runs/<run_id>/outputs/output.md`
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

Registry contract:
- name: `vibehub-journal`
- args: `<project_root> <decision> [rationale] [refs]`
- returns: `skill_response_schema_v1`
- callable_by: main-agent, sub-agent
- side_effects: writes_events, writes_notes
- idempotent: false

---
name: vibehub-events
description: "Read event stream slices. Use for VibeHub workflow step: vibehub-events."
---

# vibehub-events

中文: 执行 vibehub-events skill。
English: Read event stream slices.

Invocation input: <project_root> [run_id] [since]

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

Skill contract:
- name: `vibehub-events`
- description: Read event stream slices.
- returns: `skill_response_schema_v1`
- callable_by: sub-agent
- side_effects: none
- idempotent: true

Arguments:
- `project_root`: required, type=string
- `run_id`: optional, type=string
- `since`: optional, type=string

Task:
Use the VibeHub CLI or app command surface for `vibehub-events` when available. Keep changes scoped to the active task and follow the shared output contract.

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
- Adapter writes are the only INV-6 UI-write exception and are limited to generated adapter configuration files.

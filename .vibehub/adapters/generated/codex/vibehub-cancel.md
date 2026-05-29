---
name: vibehub-cancel
description: "Cancel a task while preserving history. Use for VibeHub workflow step: vibehub-cancel."
---

# vibehub-cancel

中文: 执行 vibehub-cancel skill。
English: Cancel a task while preserving history.

Invocation input: <project_root> <task_id> <reason>

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

Skill contract:
- name: `vibehub-cancel`
- description: Cancel a task while preserving history.
- returns: `skill_response_schema_v1`
- callable_by: main-agent
- side_effects: writes_events, updates_projection
- idempotent: false

Arguments:
- `project_root`: required, type=string
- `task_id`: required, type=string
- `reason`: required, type=string

Task:
Use the VibeHub CLI or app command surface for `vibehub-cancel` when available. Keep changes scoped to the active task and follow the shared output contract.

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

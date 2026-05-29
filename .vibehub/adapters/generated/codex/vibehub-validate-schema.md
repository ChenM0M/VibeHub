---
name: vibehub-validate-schema
description: "Validate a payload against a built-in or project custom capability schema. Use for VibeHub workflow step: vibehub-validate-schema."
---

# vibehub-validate-schema

中文: 执行 vibehub-validate-schema skill。
English: Validate a payload against a built-in or project custom capability schema.

Invocation input: <schema_ref> [project_root] [capability] <payload>

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

Skill contract:
- name: `vibehub-validate-schema`
- description: Validate a payload against a built-in or project custom capability schema.
- returns: `skill_response_schema_v1`
- callable_by: sub-agent
- side_effects: none
- idempotent: true

Arguments:
- `schema_ref`: required, type=string
- `project_root`: optional, type=string
- `capability`: optional, type=string
- `payload`: required, type=object

Task:
Use the VibeHub CLI or app command surface for `vibehub-validate-schema` when available. Keep changes scoped to the active task and follow the shared output contract.

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

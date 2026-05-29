---
name: vibehub-configure-custom-capability
description: "Add or update a project custom_capabilities workflow entry and validate that it does not conflict with built-ins. Use for VibeHub workflow step: vibehub-configure-custom-capability."
---

# vibehub-configure-custom-capability

中文: 执行 vibehub-configure-custom-capability skill。
English: Add or update a project custom_capabilities workflow entry and validate that it does not conflict with built-ins.

Invocation input: <project_root> <capability> <required_fields> [optional_fields] [produces] [consumes] [gates] [parallel_safe]

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

Skill contract:
- name: `vibehub-configure-custom-capability`
- description: Add or update a project custom_capabilities workflow entry and validate that it does not conflict with built-ins.
- returns: `skill_response_schema_v1`
- callable_by: main-agent
- side_effects: writes_workflow, validates_schema
- idempotent: false

Arguments:
- `project_root`: required, type=string
- `capability`: required, type=string
- `required_fields`: required, type=array[string]
- `optional_fields`: optional, type=array[string], default=[]
- `produces`: optional, type=array[string], default=[]
- `consumes`: optional, type=array[string], default=[]
- `gates`: optional, type=array[string], default=[]
- `parallel_safe`: optional, type=boolean, default=false

Task:
Use the VibeHub CLI or app command surface for `vibehub-configure-custom-capability` when available. Keep changes scoped to the active task and follow the shared output contract.

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

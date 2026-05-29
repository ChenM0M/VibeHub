---
name: vibehub-build-pack
description: "Build task or capability Context Pack. Use for VibeHub workflow step: vibehub-build-pack."
---

# vibehub-build-pack

中文: 执行 vibehub-build-pack skill。
English: Build task or capability Context Pack.

Invocation input: <project_root> <kind> [capability]

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

Skill contract:
- name: `vibehub-build-pack`
- description: Build task or capability Context Pack.
- returns: `skill_response_schema_v1`
- callable_by: sub-agent
- side_effects: writes_context_pack, writes_events
- idempotent: true

Arguments:
- `project_root`: required, type=string
- `kind`: required, type=enum[task,capability]
- `capability`: optional, type=string

Task:
Use the VibeHub CLI or app command surface for `vibehub-build-pack` when available. Keep changes scoped to the active task and follow the shared output contract.

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

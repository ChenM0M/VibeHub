---
name: vibehub-release
description: "Release an active capability and produce handoff state. Use for VibeHub workflow step: vibehub-release."
---

# vibehub-release

中文: 执行 vibehub-release skill。
English: Release an active capability and produce handoff state.

Invocation input: <project_root> <capability> <outcome> [task_pack_dirty]

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

Skill contract:
- name: `vibehub-release`
- description: Release an active capability and produce handoff state.
- returns: `skill_response_schema_v1`
- callable_by: main-agent
- side_effects: writes_events, writes_handoff, may_rebuild_task_pack
- idempotent: false

Arguments:
- `project_root`: required, type=string
- `capability`: required, type=string
- `outcome`: required, type=enum[completed,paused,failed,cancelled]
- `task_pack_dirty`: optional, type=boolean, default=false

Task:
Use the VibeHub CLI or app command surface for `vibehub-release` when available. Keep changes scoped to the active task and follow the shared output contract.

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

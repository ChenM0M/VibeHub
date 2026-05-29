---
name: vibehub-claim
description: "Enter a capability if gates allow it. Use for VibeHub workflow step: vibehub-claim."
---

# vibehub-claim

中文: 在 gate 允许时 claim 指定 capability。
English: Enter a capability if gates allow it.

Invocation input: <project_root> <capability>

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

Task:
Claim a named capability only after VibeHub gates allow it. Prefer `claim <project_path> <capability>` or `vibehub claim <project_path> <capability>` when the CLI is available, then read the rebuilt capability context pack and continue the work.



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
- name: `vibehub-claim`
- args: `<project_root> <capability>`
- returns: `skill_response_schema_v1`
- callable_by: main-agent
- side_effects: writes_events, builds_context_pack
- idempotent: false

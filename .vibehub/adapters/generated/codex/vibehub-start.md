---
name: vibehub-start
description: "Create one or more VibeHub tasks; split multi-intent user requests into separate task drafts when requirements are independently deliverable. Use for VibeHub workflow step: vibehub-start."
---

# vibehub-start

中文: 根据用户请求草拟 VibeHub 任务。
English: Create one or more VibeHub tasks; split multi-intent user requests into separate task drafts when requirements are independently deliverable.

Invocation input: <project_root> [title] [intent] [mode] [intake]

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

Task:
Convert the user's request into a VibeHub task draft with goal, acceptance criteria, mode suggestion, and context candidates.



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
- name: `vibehub-start`
- args: `<project_root> [title] [intent] [mode] [intake]`
- returns: `skill_response_schema_v1`
- callable_by: main-agent
- side_effects: writes_events, writes_task_state, builds_context_pack
- idempotent: false

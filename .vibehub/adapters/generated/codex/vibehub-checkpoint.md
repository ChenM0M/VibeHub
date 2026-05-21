---
name: vibehub-checkpoint
description: "Record progress, commands, risks, and next steps. Use for VibeHub workflow step: vibehub-checkpoint."
---

# vibehub-checkpoint

中文: 记录进展、命令、风险和下一步。
English: Record progress, commands, risks, and next steps.

Invocation input: [note]

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

Task:
Capture progress, decisions, commands, tests, changed files, risks, and next steps without marking state complete. Also write the current phase snapshot to `runs/<run_id>/phases/<phase>.output.md`, update `.vibehub/notes/status.md` with a one-sentence current status, and update `.vibehub/notes/summary.md` only if project scope changed.

Agent-written lifecycle artifacts:
- Mirror the phase output into `.vibehub/tasks/<task_id>/runs/<run_id>/phases/<phase>.output.md`.
- Update `.vibehub/notes/status.md` with exactly one current-status sentence at the end of the session.
- Update `.vibehub/notes/summary.md` only when the project scope or goal changes.
- These files are agent-owned business artifacts; VibeHub should read them, not generate them.


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

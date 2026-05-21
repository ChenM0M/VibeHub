---
name: vibehub-sync
description: "Reconcile external workspace changes with VibeHub state. Use for VibeHub workflow step: vibehub-sync."
---

# vibehub-sync

中文: 将外部工作区变更与 VibeHub 状态对齐。
English: Reconcile external workspace changes with VibeHub state.

Invocation input: [scope]

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

Task:
Run a best-effort sync of actual project state into VibeHub. Inspect Git diff/status, VibeHub pointers, current context, latest output, and handoff. If a VibeHub CLI is available, prefer `vibehub sync <project_path>` or `sync <project_path>`; accept `sycn` as a typo alias. Then summarize the generated `.vibehub/agent-view/sync.md` report. Ask the user only for missing intent/progress/future-plan details that cannot be inferred from hard evidence. If the user does not answer, record the questions as unresolved risks and continue from hard_observed evidence.

Sync behavior:
- Treat "sync", "sycn", "同步", "刷新状态", "update VibeHub", or plain requests to continue from current engineering reality as this command.
- Autonomously collect hard evidence first: Git status/diff, current task/run/phase, context pack state, latest output, handoff, and visible warnings.
- Ask concise follow-up questions when needed: current progress, whether dirty files belong to this task, validation/test status, unresolved risks, and next plan.
- Do not block the sync when the user gives no answer; write the open questions and inferred risk into the output.



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


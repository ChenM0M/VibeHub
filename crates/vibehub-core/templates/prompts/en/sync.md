# VibeHub Sync Prompt

Project: `{{project_root}}`
Current task: `{{task_id}}` - {{task_title}}
Current run: `{{run_id}}`
Current phase: `{{phase}}` (`{{phase_status}}`)
Changed files visible to cockpit: {{changed_files_count}}

Please run VibeHub sync for this project. Treat continue, refresh status, visible drift, or unclear current phase as sync first.

## Steps
1. Inspect hard evidence: Git status/diff, current task/run/phase pointers, context pack state, latest output.md, handoff.
2. Check for workspace drift: Git HEAD vs VibeHub recorded HEAD, stale context, dirty files not owned by current task.
3. Ask the user only for missing intent/progress/future-plan details that cannot be inferred from evidence.
4. If the user does not answer, record questions as unresolved risks.
5. Regenerate agent-view files so `current.md` and `current-context.md` point to the same phase.

## Stop
Generate sync report at `.vibehub/agent-view/sync.md`. Do not silently advance canonical state. If state is broken, recommend `vibehub-recover`.

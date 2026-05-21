# VibeHub Write Surface Audit - 2026-05-21

Evidence grade: `hard_observed` from `rg -n "fs::write|tokio::fs::write|OpenOptions::|create_dir_all" src-tauri/src/vibehub`.

## Classification

Allowed VibeHub-owned writes:

- `init.rs`: init seed files under `.vibehub/`, including `.vibehub/notes/{summary,status}.md`. VibeHub only seeds these files when missing.
- `start_task.rs`, `phase.rs`, `current.rs`, `locale.rs`, `research.rs`, `state_migration.rs`: canonical state/task/run metadata transitions owned by VibeHub.
- `agent_view.rs`, `handoff.rs`: generated agent-view cache and handoff cache under `.vibehub/agent-view/`.
- `events.rs`: append-only `events.jsonl` audit log.
- `sync.rs`, `drift.rs`: sync/recover reports and state reconciliation artifacts.
- `context.rs`: generated context packs and manifests.
- `agent_adapter.rs`: generated adapter instruction files and adapter config/hash tracking.
- `review.rs`: generated review evidence, diff patch, changed-files report, and review summary.
- `journal.rs`, `knowledge.rs`, `research.rs`: explicit user/agent-requested durable notes.

Agent-owned business artifacts:

- `.vibehub/tasks/<task_id>/runs/<run_id>/outputs/output.md`
- `.vibehub/tasks/<task_id>/runs/<run_id>/phases/<phase>.output.md`
- `.vibehub/notes/summary.md` after init seed
- `.vibehub/notes/status.md` after init seed

## Changes From This Pass

- `agent_reported`: `vibehub-checkpoint` and `vibehub-finish` command bodies now instruct agents to write `phases/<phase>.output.md` and maintain `.vibehub/notes/status.md`, with `.vibehub/notes/summary.md` updated only when scope changes.
- `hard_observed`: `overview.flow_details[]` reads phase output paths but does not create them.

## Residual Risk

- `inferred`: Some existing VibeHub generators still produce business-adjacent artifacts (`review.rs`, `research.rs`, `journal.rs`, `knowledge.rs`) because they are explicit commands. This pass did not migrate those workflows to agent-only writes.

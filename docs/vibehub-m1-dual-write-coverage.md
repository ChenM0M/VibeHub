# VibeHub M1 Dual-Write Coverage

Status: Draft v1
Date: 2026-05-28
Baseline: `docs/vibehub-capability-redesign-2026-05-27.md` section 12 M1 and section 20
Implementation manual: `docs/vibehub-capability-implementation-steps-2026-05-27.md` M1b

## Scope

M1b keeps the existing `state.yaml`, task YAML, run YAML, handoff, sync, review, and recover write paths unchanged. Each legacy `append_run_event` write point now also appends one or more structured `VibehubEvent` records to the active run `events.jsonl`.

This is a compatibility period. Legacy events remain present as `Legacy` payloads so existing readers are not forced to migrate during M1.

## Coverage Table

| Existing trigger | Legacy event | Structured event(s) | File |
|---|---|---|---|
| Task/run creation | `task_started` | `TaskCreated`, `CapabilityClaimed`, `PhaseProjected` | `src-tauri/src/vibehub/start_task.rs` |
| Explicit phase status set | `phase_status_set` | `CapabilityClaimed` when status becomes `active`; `CapabilityReleased` when status becomes `completed`, `failed`, `blocked`, or `needs_action`; always `PhaseProjected` | `src-tauri/src/vibehub/phase.rs` |
| Current phase completed | `phase_completed` | `CapabilityReleased`, `PhaseProjected` | `src-tauri/src/vibehub/phase.rs` |
| Phase advance blocked by missing outputs | `phase_advance_blocked` | `CapabilityReleased { outcome: needs_action }`, `PhaseProjected` | `src-tauri/src/vibehub/phase.rs` |
| Phase advance blocked by incomplete handoff | `phase_advance_blocked` | `CapabilityReleased { outcome: needs_action }`, `PhaseProjected` | `src-tauri/src/vibehub/phase.rs` |
| Final phase completed | `phase_advanced` | `CapabilityReleased`, `PhaseProjected` | `src-tauri/src/vibehub/phase.rs` |
| Phase advanced to next phase | `phase_advanced` | `CapabilityReleased`, `CapabilityClaimed`, `PhaseProjected` | `src-tauri/src/vibehub/phase.rs` |
| Forced advance with incomplete handoff | `handoff_force_advance` | `RiskRaised` | `src-tauri/src/vibehub/phase.rs` |
| Handoff generated | `handoff_built` | `HandoffWritten`; `EvidenceAdded` when the generated handoff is complete | `src-tauri/src/vibehub/handoff.rs` |
| Workspace sync starts | none | `SyncStarted` | `src-tauri/src/vibehub/sync.rs` |
| Workspace sync report generated | `sync_report_written` | `SyncCompleted`, `SyncReport` | `src-tauri/src/vibehub/sync.rs` |
| Recover report generated | `recover_report_written` | `EvidenceAdded`, `DiffObserved` | `src-tauri/src/vibehub/drift.rs` |
| Review evidence generated | `review_evidence_generated` | `EvidenceAdded`, `DiffObserved` | `src-tauri/src/vibehub/review.rs` |

## Checkpoint Note

`vibehub-checkpoint` is currently an agent instruction path, not a dedicated Rust backend command or module. No existing Rust checkpoint write point calls `append_run_event`. The backend paths that persist checkpoint-like evidence today are covered through handoff, review evidence, recover evidence, and sync report writes.

If a future backend `checkpoint.rs` or CLI checkpoint command is added, it should append `EvidenceAdded`, `DiffObserved`, and `ValidationRun` as appropriate and update this table in the same change.

## Consistency Self-Check

`events::ensure_consistency(project_root)` folds the latest structured `PhaseProjected` event and compares it to the current `state.yaml` `current.phase`.

The check is warning-only:

- It returns an empty warning list when no active task/run exists.
- It returns no warning for old event streams that do not yet contain `PhaseProjected`.
- It returns a warning when `state.yaml` and the latest `PhaseProjected` disagree.
- `sync_workspace` calls the check and merges warnings into the sync warnings without blocking sync.

## DoD Evidence

| DoD | Evidence |
|---|---|
| Every old write point has a corresponding event | `rg -n "append_run_event|append_current_run_event" src-tauri/src/vibehub` mapped in the coverage table above |
| Startup/self-check exists and is called | `events::ensure_consistency` exists and `sync_workspace` calls it |
| Real task event stream remains ordered and non-empty | Existing writer validation enforces event_id order; targeted phase test asserts `CapabilityReleased`, `CapabilityClaimed`, and `PhaseProjected` |
| Coverage table has no omitted backend legacy write point | This document maps all current backend `append_run_event` call sites |

## Known Limits

- `ensure_consistency` only compares phase projection in M1b; full event fold versus all derived state fields is deferred to M1c.
- Existing old event lines without `event_id` are tolerated during the compatibility period.
- Agent-only checkpoint behavior still depends on agent output files until a backend checkpoint API exists.

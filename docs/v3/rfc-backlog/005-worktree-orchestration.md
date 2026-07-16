# RFC Backlog 005: Worktree Orchestration

- Status: M5 implementation in progress; D1-D7 frozen
- Owner: Worktree Orchestration owner
- Decision milestone: M4 planning; M5 implementation
- Implementation milestone: M5
- Related task: `T-20260711080144-de5ecb84`
- Entry gate: automated and macOS M4 evidence passed; Windows native evidence and owner approval pending
- Evidence: `.vibehub/research/current/research-pack.md` F005, F006, F007

## Problem

Git worktree isolates `HEAD`, index, and working files, but it does not decide whether PlanNodes are independent, own a session, integrate branches, resolve conflicts, or recover abandoned/locked trees. V3 needs an observable orchestration lifecycle across multiple agent tools and native platforms.

## Invariants

1. One active parallel PlanNode owns one branch/worktree lease unless explicitly read-only.
2. Scope overlap and shared migration/generated files are evaluated before launch.
3. No dirty worktree is removed or reset automatically.
4. Integration and conflict resolution have explicit ownership and evidence.
5. Every Git command uses argument arrays and stable porcelain output where available.

## Decisions To Resolve

### D1. Eligibility and scope overlap

Define path/module ownership, declared and observed changes, shared-file denylist, unknown scope, and pre-launch severity.

### D2. Branch/worktree identity

Define branch naming, base commit, worktree root, task/node/session IDs, detached/unborn states, and already-checked-out behavior.

### D3. Lease and heartbeat

Define owner, acquired/expires/heartbeat, lock reason, reclaim challenge, stale lease, and crash recovery.

### D4. Lifecycle state machine

Cover `planned`, `creating`, `ready`, `active`, `dirty`, `submitted`, `integrating`, `conflicted`, `integrated`, `abandoned`, `repairing`, and `cleaned`, including legal transitions and evidence.

### D5. Integration policy

Choose merge/rebase/cherry-pick policy per project, queue ordering, pre-integration tests, base drift, conflict assignment, retry, and final retention.

### D6. Native path/process policy

Measure Windows root placement and path budget; cover drive/UNC restrictions, antivirus/file occupation, IDE terminals, agent cwd, application exit, and orphan processes.

### D7. Self-host observability

During M5, V3 records its own Task/PlanNode/Session/Worktree events while V2 retains a minimal audit pointer and rollback control. Define comparison and rollback triggers before launch.

## Required Spikes

1. Three worktrees, at least two agent tools, three non-overlapping nodes, with event/workspace contamination checks.
2. Intentional same-file conflict through integration and ownership flow.
3. Agent crash, desktop exit, missing directory, locked file, and moved worktree repair scenarios.
4. Windows long-path budget test from the proposed default root.

## Deliverables

- Eligibility, lease, worktree, and integration state machines.
- Git command/error mapping and stable porcelain parser contract.
- Conflict ownership and recovery UX contract.
- Native path budget and cleanup policy.
- V3 self-host shadow/rollback runbook.

## Exit Criteria

- M4 gate and owner approval are recorded before the first V3-managed M5 event.
- Parallel sessions have isolated cwd/index/events and explainable ownership.
- Conflict, crash, locked-file, and missing-worktree scenarios recover without silent loss.
- UI explains base, branch, owner, dirty state, integration state, and next action for every worktree.

## M0 Freeze Record

- `decided`: M0 and M1 do not implement or simulate worktree orchestration.
  `PlanGraphView.execution` may show planned versus observed counts only.
- `decided`: native/display path identity, inaccessible paths, case collisions,
  scope overlap warnings, and explicit degraded states are prerequisites for
  later orchestration.
- `deferred`: eligibility, leases, branch naming, integration policy, cleanup,
  path budgets, and self-host shadow comparison remain gated by M4/M5 evidence.
- Alternatives retained: merge/rebase/cherry-pick remain project policy choices;
  no dirty worktree cleanup may be inferred from a fixture.
- Evidence/exit: FX-PARALLEL and FX-WIN-PATHS provide stress inputs only; no
  host/platform execution claim is made before the required native spikes.

## M5 Implementation Checkpoint (2026-07-12)

- `implemented`: Added the versioned `WorktreeOrchestrationView` contract for
  entry-gate, policy, eligibility, lease, Git observation, integration queue,
  conflict ownership, recovery, orphan candidates, and next action.
- `implemented`: Extended application commands and event/view references with
  worktree, lease, operation, and integration identities. Generated TypeScript
  types and all 12 deterministic fixture bundles now include the sixth view.
- `implemented`: FX-PARALLEL blocks overlapping/generated scope without a
  lease; FX-REWORK preserves conflict owner and retry operation identity;
  FX-WIN-PATHS requires process inspection and refuses dirty cleanup.
- `implemented`: Added pure core domain rules for lifecycle transitions, scope
  normalization, case-fold and observed-file collision, denylist matching,
  evidence-backed owner override, stable eligibility digest, branch identity,
  and challenged lease reclaim with generation increments.
- `implemented`: Added an injectable Git runner using argv plus explicit cwd,
  stable worktree/status porcelain parsers, prepare/execute/inspect/result
  operation records, and clean/remove refusal when dirty or ownership is not
  confirmed.
- `implemented`: Extended the Rust event envelope and idempotency scope with
  worktree, lease, and operation identities. Added event-backed worktree and
  lease handlers with optimistic versions, idempotent retry, eligibility
  digest checks, explicit transitions, challenged reclaim generations,
  writable activation lease enforcement, and an explicit read-only exception.
- `implemented`: Project rebuild now folds orchestration events into a stable
  worktree projection; deleting and rebuilding the projection preserves state
  and no longer reports recognized orchestration events as unknown.
- `validated`: Contract check passes 423 assertions across 12 scenarios and 6
  views. Core passes 251 tests. The production TypeScript build passes.
- `not_yet_done`: Git side-effect reconciliation, complete contract-view
  projection, launcher ownership, MCP/Tauri control plane, production UI
  mapping, native harnesses, and self-host shadow remain. No real M5 self-host
  event has been written.

# RFC Backlog 004: Task, PlanNode, and Acceptance Lifecycle

- Status: M0 view boundary frozen; M4 state-machine decision backlog
- Owner: Task / Plan Lifecycle owner
- Decision milestone: M0 boundary; M4 state-machine freeze
- Implementation milestone: M4
- Related task: `T-20260711080143-fbe94685`
- Evidence: `.vibehub/research/current/research-pack.md` F002, F003, F007

## Problem

V3 replaces the hard-coded phase pipeline with an editable plan graph while preserving align, research, plan, implement, and review semantics. It must retain failed attempts and findings, prevent false completion, and recover sessions without flattening schedule, causality, and actual event order into one graph.

## Invariants

1. Plan DAG contains scheduling dependencies only and remains acyclic.
2. Trace Graph contains `repairs`, `addresses`, `validates`, `supersedes`, and other causal relations.
3. Event Timeline records what happened; history is append-only.
4. Criterion status is independent and cannot be inferred from node completion alone.
5. Failed findings/attempts remain visible after remediation.

## Decisions To Resolve

### D1. Entity identity and ownership

Freeze Project/Task/PlanNode/Session/Criterion/Finding/Attempt IDs, aggregate ownership, ordering, and cross-task references.

### D2. Plan graph editing

Define add/split/replace/retry/cancel/block/unblock transitions, dependency edits, cycle rejection, scope change, and plan-version conflicts.

### D3. Criterion lifecycle

Define proposed/accepted/pass/fail/blocked/not-applicable states, required evidence, reviewer identity, supersession, and completion rules.

### D4. Finding and remediation loop

Define severity, evidence, target, remediation node/attempt creation, re-review, closure, regression, and UI folding without overwriting history.

### D5. Session and coverage lifecycle

Define open/active/idle/closed/gapped/abandoned/repaired, heartbeat, node scope, commit association, and coverage grades.

### D6. Completion confirmation

Define proposal digest, state version, criterion summary, confirmation channel, expiry, rejection, and invalidation after new events.

## M4 Stability Gate Before Self-Hosting M5

All items are mandatory:

1. Deterministic rebuild matches live projections for the M4 gold corpus.
2. Duplicate/reordered retry tests create no duplicate semantic events.
3. Criterion, Finding, and Attempt history survives two remediation cycles.
4. Kill-resume scenarios pass the four semantic hard gates in the main plan.
5. Codex, Claude Code, and OpenCode complete the minimal read/open/log/close/recover loop.
6. Native macOS and Windows smoke suites pass for paths, stdio, lock, and process cleanup.
7. No open P0/P1 data-loss, wrong-task, false-completion, or confirmation-authenticity defect.
8. A predeclared soak count/duration passes; the threshold cannot be weakened after seeing failures.
9. The project owner explicitly approves the V3-managed M5 shadow run.

If any item fails, M5 remains managed by V2. No partial self-hosting is inferred from feature completeness.

## Deliverables

- Entity/state/transition tables and event catalog.
- Plan DAG and Trace Graph validation rules.
- Criterion/Finding/Attempt projection rules.
- Session/coverage/recovery contract.
- M4 stability evidence report and M5 rollback procedure.

## Exit Criteria

- UI explains current state and full history from the same events.
- A failed required Criterion cannot project Task `completed`.
- Two repair cycles preserve all attempts and causality.
- The M4 stability gate is machine-checkable where possible and human-approved where required.

## M0 Freeze Record

- `decided`: Plan DAG scheduling edges are distinct from trace relations;
  timeline history keeps decisions, evidence, findings, attempts, validation,
  confirmation, session, plan, and gap entries.
- `decided`: criterion state is independent from node/task state, and NodeBrief
  carries scoped current intent rather than a repository or history dump.
- `deferred`: legal transition tables, optimistic plan edits, criterion
  confirmation digest, heartbeat, and exact self-host stability thresholds are
  M4 decisions.
- Alternatives rejected for the view boundary: one graph for both scheduling
  and causality; overwriting failed attempts after remediation.
- Evidence/exit: FX-PARALLEL, FX-REWORK, FX-LARGE, and FX-COVERAGE-GAP cover
  consumer semantics while preserving the full M4 gate above.

# VibeHub Skills Registry v1

> Status: Draft v1
> Date: 2026-05-28
> Baseline refs: §10, §22, §27
> Registry sample: [`.vibehub/skills.registry.yaml`](../.vibehub/skills.registry.yaml)

## 1. Contract

VibeHub is the project memory, task router, and workflow gatekeeper for coding agents. Agents do the engineering work; VibeHub tracks state, context, gates, output, and handoff.

Agent-facing command examples use the headless CLI binary `vibehub-cli`. The desktop application may also be named VibeHub, so docs and generated adapter instructions should avoid bare `vibehub ...` command examples.

Every skill returns the baseline §22.1 shape:

```json
{
  "status": "ok",
  "data": {},
  "error": null,
  "events_emitted": [],
  "context_header": {
    "task_id": "T-...",
    "active_capabilities": [],
    "freshness": { "git_head": "...", "last_event_id": "evt-..." }
  }
}
```

On failure, `status` is `"error"`, `data` is empty, and `error.code` uses the baseline §25 three-part form.

## 2. Caller Matrix

| Skill | main-agent | sub-agent | Notes |
|---|---:|---:|---|
| `vibehub-init` | yes | no | Project bootstrap |
| `vibehub-status` | yes | no | Human-facing summary |
| `vibehub-next-action` | yes | yes | Intent-aware router for start/sync/continue/validate/lint/transition/recover |
| `vibehub-output-lint` | yes | yes | Output quality, evidence-label, and stale-contradiction lint |
| `vibehub-sync` | yes | yes | Sub-agent candidate for deep sync |
| `vibehub-start` | yes | no | Task creation; multi-intent intake and task splitting |
| `vibehub-claim` | yes | no | Capability entry |
| `vibehub-record` | yes | no | Schema-checked output write |
| `vibehub-release` | yes | no | Capability exit |
| `vibehub-handoff` | yes | yes | Sub-agent candidate for writing summaries |
| `vibehub-checkpoint` | yes | no | Progress snapshot |
| `vibehub-journal` | yes | yes | Sub-agent candidate for durable notes |
| `vibehub-finish` | yes | no | Task finish gate |
| `vibehub-recover` | yes | no | Re-entry and repair |
| `vibehub-cancel` | yes | no | User-driven cancellation |
| `vibehub-events` | no | yes | Read-only event access |
| `vibehub-build-pack` | no | yes | Context Pack builder |
| `vibehub-validate-schema` | no | yes | Deterministic schema check helper |
| `vibehub-debug-dump` | yes | no | Diagnostics |

Sub-agent candidates align with baseline §10.2: deep sync, pack building, schema assistance, journal, and handoff work.

In multi-active-task workspaces, agents should run `vibehub-cli status` and either switch with `vibehub-cli switch <project> <task_id>` or use task-scoped validation such as `vibehub-cli validate-task <project> <task_id>` before relying on validation results.

## 3. Skill Details

### `vibehub-init`

- Fields: `name=vibehub-init`; `args=[project_root, mode?, agents?, output_language?]`; `returns=skill_response_schema_v1`; `side_effects=[writes_vibehub_state, may_write_adapter_files]`; `callable_by=[main-agent]`; `idempotent=true`; `description=Initialize VibeHub for a git project`.
- Main use case: run once when a git project has no `.vibehub` state or when an explicit repair of managed adapter instructions is requested.
- Error example: `{ "code": "sync.git.head_unreachable", "message": "Project is not a git repository", "hint": "Initialize git first or choose another project_root" }`.
- Call example: `vibehub-init { project_root: "/repo", mode: "evidence_drive", agents: ["codex"] }`.

### `vibehub-status`

- Fields: `name=vibehub-status`; `args=[project_root]`; `returns=skill_response_schema_v1`; `side_effects=[]`; `callable_by=[main-agent]`; `idempotent=true`; `description=Read projected task/run/capability status`.
- Main use case: let an agent confirm current task, phase/capability state, dirty git count, context pack availability, and warnings before taking action.
- Error example: `{ "code": "pack.build.failed", "message": "Agent view is missing", "hint": "Run vibehub-sync or vibehub-recover" }`.
- Call example: `vibehub-status { project_root: "/repo" }`.

### `vibehub-next-action`

- Fields: `name=vibehub-next-action`; `args=[project_root, intent?]`; `returns=skill_response_schema_v1`; `side_effects=[]`; `callable_by=[main-agent, sub-agent]`; `idempotent=true`; `description=Recommend the next agent action, skill, and CLI command from current VibeHub state`.
- Main use case: ask VibeHub for a machine-readable route before choosing whether to start, split, sync, continue, validate, lint output, advance, archive, or recover. The optional `intent` lets an agent pass the user's latest wording for lightweight routing without loading more prompt text.
- Error example: `{ "code": "state.read.failed", "message": "Cannot inspect current VibeHub state", "hint": "Run vibehub-recover" }`.
- Call example: `vibehub-next-action { project_root: "/repo", intent: "同步当前状态" }`.

### `vibehub-output-lint`

- Fields: `name=vibehub-output-lint`; `args=[project_root, task_id?]`; `returns=skill_response_schema_v1`; `side_effects=[]`; `callable_by=[main-agent, sub-agent]`; `idempotent=true`; `description=Lint VibeHub output.md for missing sections, stale contradictions, and evidence-label hygiene`.
- Main use case: run before finish/advance or final user reporting to catch output that technically has sections but is stale, contradictory, or missing evidence labels.
- Error example: `{ "code": "output.missing", "message": "No output.md was found", "hint": "Write run-level output.md before advancing" }`.
- Call example: `vibehub-output-lint { project_root: "/repo", task_id: "T-123" }`.

### `vibehub-sync`

- Fields: `name=vibehub-sync`; `args=[project_root, mode?]`; `returns=skill_response_schema_v1`; `side_effects=[writes_events, rebuilds_context, may_write_adapter_files]`; `callable_by=[main-agent, sub-agent]`; `idempotent=true`; `description=Reconcile external workspace state with VibeHub; use for continue/refresh/sync requests before edits`.
- Main use case: run when git head, dirty files, adapter instructions, or agent context freshness has drifted; deep mode is a sub-agent candidate.
- Error example: `{ "code": "sync.git.head_unreachable", "message": "Cannot read git HEAD", "hint": "Check repository availability and permissions" }`.
- Call example: `vibehub-sync { project_root: "/repo", mode: "auto" }`.

### `vibehub-start`

- Fields: `name=vibehub-start`; `args=[project_root, title?, intent?, mode?, intake?]`; `returns=skill_response_schema_v1`; `side_effects=[writes_events, writes_task_state, builds_context_pack]`; `callable_by=[main-agent]`; `idempotent=false`; `description=Create one or more VibeHub tasks; split multi-intent or independently deliverable requests before starting work`.
- Main use case: convert a new user request or externally discovered drift into tracked task metadata and context. If one user message contains multiple independent requirements, the main agent should split them into multiple task drafts or task creation calls instead of forcing everything into one task.
- Multi-intent intake rule: split when requirements have independent deliverables, acceptance criteria, pause/cancel semantics, or file/module scope. Keep one task when items are merely implementation steps of the same user goal. Ask one concise confirmation question only when the split is genuinely ambiguous.
- Error example: `{ "code": "schema.required.missing", "message": "Task intent is missing", "hint": "Provide a short goal statement" }`.
- Call example: `vibehub-start { title: "Capability refactor", intent: "Replace phase flow with event-backed capabilities" }`.
- Multi-task call example: `vibehub-start { intake: [{ title: "Fix login error", intent: "Resolve login failure" }, { title: "Remote link setting", intent: "Add project remote URL setting" }] }`.

### `vibehub-claim`

- Fields: `name=vibehub-claim`; `args=[project_root, capability]`; `returns=skill_response_schema_v1`; `side_effects=[writes_events, builds_context_pack]`; `callable_by=[main-agent]`; `idempotent=false`; `description=Enter a capability if gates allow it`.
- Main use case: replace old `vibehub-continue` and phase commands with explicit capability entry and fresh capability pack generation.
- Error example: `{ "code": "gate.precondition.unmet", "message": "Capability is not currently claimable", "hint": "Run vibehub-status to see unmet gates" }`.
- Call example: `vibehub-claim { capability: "research" }`.

### `vibehub-record`

- Fields: `name=vibehub-record`; `args=[project_root, capability, output]`; `returns=skill_response_schema_v1`; `side_effects=[writes_events, writes_output]`; `callable_by=[main-agent]`; `idempotent=false`; `description=Persist schema-checked capability output`.
- Main use case: write the structured artifact for a completed or partially completed capability after synchronous schema validation.
- Error example: `{ "code": "schema.required.missing", "message": "research output missing sources", "hint": "Add at least one source object or explicit placeholder where allowed" }`.
- Call example: `vibehub-record { capability: "research", output: { schema_version: "1.0", data: {} } }`.

### `vibehub-release`

- Fields: `name=vibehub-release`; `args=[project_root, capability, outcome, task_pack_dirty?]`; `returns=skill_response_schema_v1`; `side_effects=[writes_events, writes_handoff, may_rebuild_task_pack]`; `callable_by=[main-agent]`; `idempotent=false`; `description=Release an active capability and produce handoff state`.
- Main use case: finish, pause, or fail a capability while preserving the next agent's continuation context.
- Error example: `{ "code": "gate.precondition.unmet", "message": "Capability is not active", "hint": "Claim the capability before release or run recover" }`.
- Call example: `vibehub-release { capability: "plan", outcome: "completed", task_pack_dirty: true }`.

### `vibehub-handoff`

- Fields: `name=vibehub-handoff`; `args=[project_root, capability?, summary, next_steps?]`; `returns=skill_response_schema_v1`; `side_effects=[writes_events, writes_handoff]`; `callable_by=[main-agent, sub-agent]`; `idempotent=false`; `description=Write a durable handoff without necessarily releasing a capability`.
- Main use case: capture context before interruption, tool handover, or long-running step transition; writing can be delegated when the summary is self-contained.
- Error example: `{ "code": "schema.required.missing", "message": "Handoff summary is empty", "hint": "Summarize completed work, risks, and next steps" }`.
- Call example: `vibehub-handoff { capability: "implement", summary: "M1a writer added", next_steps: ["Run cargo test"] }`.

### `vibehub-checkpoint`

- Fields: `name=vibehub-checkpoint`; `args=[project_root, note?, changed_files?]`; `returns=skill_response_schema_v1`; `side_effects=[writes_events, writes_output]`; `callable_by=[main-agent]`; `idempotent=false`; `description=Record progress, commands, risks, and next actions`.
- Main use case: leave an auditable progress snapshot at the end of a step or before risky changes.
- Error example: `{ "code": "schema.required.missing", "message": "Checkpoint has no note or changed files", "hint": "Provide at least one progress note" }`.
- Call example: `vibehub-checkpoint { note: "S2 schema doc completed", changed_files: ["docs/vibehub-capability-schema-v1.md"] }`.

### `vibehub-journal`

- Fields: `name=vibehub-journal`; `args=[project_root, decision, rationale?, refs?]`; `returns=skill_response_schema_v1`; `side_effects=[writes_events, writes_notes]`; `callable_by=[main-agent, sub-agent]`; `idempotent=false`; `description=Record durable decisions and rationale`.
- Main use case: promote repeated or important decisions into a durable journal that future context packs can summarize.
- Error example: `{ "code": "schema.required.missing", "message": "Decision text is missing", "hint": "Write the decision in one sentence" }`.
- Call example: `vibehub-journal { decision: "Capability v1 uses preset capabilities only", refs: ["baseline §3.3"] }`.

### `vibehub-finish`

- Fields: `name=vibehub-finish`; `args=[project_root, task_id?]`; `returns=skill_response_schema_v1`; `side_effects=[writes_events, updates_projection]`; `callable_by=[main-agent]`; `idempotent=false`; `description=Close a task after task_finishable gate passes`.
- Main use case: end a task only after required artifacts, validation, risks, and handoffs satisfy gates.
- Error example: `{ "code": "gate.precondition.unmet", "message": "Open risks remain", "hint": "Resolve risks or record accepted risk before finishing" }`.
- Call example: `vibehub-finish { task_id: "T-..." }`.

### `vibehub-recover`

- Fields: `name=vibehub-recover`; `args=[project_root, reason?]`; `returns=skill_response_schema_v1`; `side_effects=[writes_events, rebuilds_context]`; `callable_by=[main-agent]`; `idempotent=true`; `description=Re-align after interruption, stale context, or inconsistent pointers`.
- Main use case: resume after aborted turns, HEAD changes, damaged context packs, or contradictory agent views.
- Error example: `{ "code": "pack.build.failed", "message": "Could not rebuild context pack", "hint": "Inspect sub-agent failure and retry" }`.
- Call example: `vibehub-recover { reason: "interrupted session" }`.

### `vibehub-cancel`

- Fields: `name=vibehub-cancel`; `args=[project_root, task_id, reason]`; `returns=skill_response_schema_v1`; `side_effects=[writes_events, updates_projection]`; `callable_by=[main-agent]`; `idempotent=false`; `description=Cancel a task while preserving history`.
- Main use case: record that a task should no longer proceed without deleting events or artifacts.
- Error example: `{ "code": "schema.required.missing", "message": "Cancel reason is missing", "hint": "Provide the user-visible reason for cancellation" }`.
- Call example: `vibehub-cancel { task_id: "T-...", reason: "Superseded by new design" }`.

### `vibehub-events`

- Fields: `name=vibehub-events`; `args=[project_root, run_id?, since?]`; `returns=skill_response_schema_v1`; `side_effects=[]`; `callable_by=[sub-agent]`; `idempotent=true`; `description=Read event stream slices`.
- Main use case: give sub-agents read-only access to facts for pack building, consistency checks, and summaries without write permission.
- Error example: `{ "code": "event.append.out_of_order", "message": "Event stream order is invalid", "hint": "Run recover before trusting projection" }`.
- Call example: `vibehub-events { run_id: "R-...", since: "evt-..." }`.

### `vibehub-build-pack`

- Fields: `name=vibehub-build-pack`; `args=[project_root, kind, capability?]`; `returns=skill_response_schema_v1`; `side_effects=[writes_context_pack, writes_events]`; `callable_by=[sub-agent]`; `idempotent=true`; `description=Build task or capability Context Pack`.
- Main use case: perform heavy context synthesis outside the main agent while preserving strict failure semantics from INV-7.
- Error example: `{ "code": "pack.build.failed", "message": "Required source event is missing", "hint": "Run sync or recover, then rebuild pack" }`.
- Call example: `vibehub-build-pack { kind: "capability", capability: "implement" }`.

### `vibehub-validate-schema`

- Fields: `name=vibehub-validate-schema`; `args=[schema_ref, payload]`; `returns=skill_response_schema_v1`; `side_effects=[]`; `callable_by=[sub-agent]`; `idempotent=true`; `description=Validate a payload against a capability schema`.
- Main use case: let a sub-agent or preflight flow check structured outputs before `vibehub-record` attempts a write.
- Error example: `{ "code": "schema.type.mismatch", "message": "status must be passed, failed, or blocked", "hint": "Use one of the enum values from the schema" }`.
- Call example: `vibehub-validate-schema { schema_ref: "capability.validate.v1", payload: {} }`.

### `vibehub-debug-dump`

- Fields: `name=vibehub-debug-dump`; `args=[project_root, include_events?=true, include_packs?=true, redact_secrets?=true]`; `returns=skill_response_schema_v1`; `side_effects=[writes_debug_artifact]`; `callable_by=[main-agent]`; `idempotent=false`; `description=Export a redacted diagnostic bundle for cross-tool and cross-machine debugging`.
- Main use case: collect a redacted support bundle when sync, projection, or adapter generation behaves unexpectedly.
- Error example: `{ "code": "concurrency.lock.timeout", "message": "Could not read a stable snapshot", "hint": "Retry after active writes finish" }`.
- Call example: `vibehub-debug-dump { include_events: true, include_packs: true }`.

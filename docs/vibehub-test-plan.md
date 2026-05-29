# VibeHub Capability Refactor Test Plan

> Status: Draft v1
> Date: 2026-05-28
> Baseline refs: §18, §20, §28

## 1. Goals

This plan covers M1-M5 for the capability/gate/event refactor. It defines unit tests, integration tests, manual UI checks, fixtures, coverage targets, and CI integration before implementation begins.

## 2. Invariant Coverage Matrix

| Invariant | Required tests |
|---|---|
| INV-1 append-only events | writer appends only; mutation/delete attempts fail; compensation events preserve history |
| INV-2 replayable derived fields | fold event fixtures into derived state; compare with projected state |
| INV-3 single writer | concurrent append requests serialize through writer queue |
| INV-4 required schema fields | invalid capability outputs are rejected with hints |
| INV-5 Context Pack freshness | pack write fails without `freshness.git_head` and `freshness.last_event_id` |
| INV-6 UI does not write state | UI e2e verifies actions generate prompts only, except adapter config update |
| INV-7 sub-agent failure creates no pack | failed pack build leaves no empty pack file |
| INV-8 VibeHub does not write git | command audit denies write-like git invocations in VibeHub backend |
| INV-9 git repository required | init/status fail cleanly outside git repo |
| INV-10 cross-tool protocol consistency | registry projects identical return schema into adapters |

## 3. Unit Test Matrix

| Area | Test | Reproduce |
|---|---|---|
| event_id | unique IDs in same millisecond | freeze clock, append 100 events, assert sequence `0001..0100` |
| event_id | monotonic IDs across milliseconds | advance clock and assert lexical/time order |
| writer queue | FIFO append | enqueue mixed event types concurrently, read jsonl order |
| writer queue | reject out-of-order append | inject older event id and expect `event.append.out_of_order` |
| corruption | damaged jsonl recovery | fixture with truncated line, expect `EventLogCorrupted` new event location |
| projection | fold task lifecycle | replay `TaskCreated -> CapabilityClaimed -> CapabilityReleased` |
| projection | compensation events | replay `PlanDrafted -> PlanInvalidated`, assert plan is not active |
| schema | required fields | validate every S2 invalid example, expect `schema.required.missing` |
| schema | type/enum mismatch | invalid `validate.status=green`, expect `schema.type.mismatch` |
| policy | WIP limit | claim beyond `max_concurrent_claims`, expect `gate.precondition.unmet` |
| fitness | metrics aggregation | feed synthetic events, assert `events.write_per_minute` and failure rate |

Coverage target: core Rust VibeHub modules at or above 80% line coverage once M5 lands.

## 4. Integration Test Matrix

| Flow | Test | Reproduce |
|---|---|---|
| sync quick | fresh head, high overlap | fixture repo with same HEAD and in-scope doc change |
| sync deep | stale sync | set last sync > 24h and assert pack rebuild |
| sync rebuild | schema major mismatch | fixture state schema v1 vs runtime v2 |
| claim + release | happy path | claim `research`, build pack, record output, release |
| claim blocked | unmet gate | claim `review` before required validation, expect `gate.precondition.unmet` |
| pack rebuild | task dirty delta | release with `task_pack_dirty=true`, assert task pack rebuilt |
| pack no rebuild | no task delta | release with `task_pack_dirty=false`, assert only capability handoff changes |
| migration | v2 to v3 | fixture old task upgrades with `event_log_path` and no lost fields |
| recovery | ten restart loop | crash after each write point, restart, fold events, assert consistency |
| ownership | partial overlap | dirty files split active/unowned, expect user-question prompt data |

## 5. Manual UI E2E Checklist

- Open Kanban with no tasks; verify empty state only offers prompt generation.
- Open Kanban with multiple active capabilities; verify no direct state-changing buttons.
- Click claim/continue affordance; verify modal displays command/prompt text, not backend mutation.
- Open JSON renderer for Context Pack; verify freshness is visible.
- Trigger adapter update flow; verify it is clearly scoped to generated adapter files.
- Verify mobile and desktop layouts do not overlap text in task/capability cards.

## 6. Fixtures

Recommended fixture directory:

```text
src-tauri/tests/fixtures/vibehub/
├── events/
│   ├── minimal-task.jsonl
│   ├── capability-happy-path.jsonl
│   ├── compensation-plan-invalidated.jsonl
│   ├── corrupted-truncated-line.jsonl
│   └── large-10000-events.jsonl
├── state/
│   ├── schema-v2-task.yaml
│   ├── schema-v3-projected.yaml
│   └── schema-major-mismatch.yaml
├── outputs/
│   ├── research-valid.json
│   ├── research-missing-sources.json
│   ├── validate-invalid-status.json
│   └── review-clean.json
└── repos/
    ├── clean-fresh/
    ├── stale-head/
    └── partial-overlap/
```

Fixture rules:

- Event fixtures are append-only snapshots and must never be rewritten in tests.
- Corrupted fixtures are copied to a temp dir before recovery tests.
- Git repos are generated in temp dirs from small scripts rather than committed `.git` directories.

## 7. CI Workflow Draft

This workflow can be enabled once M1a introduces test modules:

```yaml
name: VibeHub Capability Tests

on:
  pull_request:
    paths:
      - "src-tauri/src/vibehub/**"
      - "src-tauri/tests/**"
      - "docs/vibehub-*.md"
      - ".vibehub/skills.registry.yaml"
  push:
    branches: [feature/vibehub-v2-p0]

jobs:
  capability-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: npm
      - run: npm ci
      - run: npm run build
      - run: cargo test --locked --manifest-path src-tauri/Cargo.toml vibehub
```

## 8. Per-DoD Reproduction Checklist

| DoD | Reproduce |
|---|---|
| M1a writer append-only | `cargo test vibehub::events::tests::append_only` |
| M1b double-write coverage | run demo phase commands, inspect events and state consistency |
| M1c migration | `cargo test vibehub::state_migration::tests::v2_to_v3` |
| M2a workflow parser | parse old and new workflow fixtures |
| M2b gate engine | claim valid/invalid capabilities in demo project |
| M3 projection | grep direct writes to derived fields and run projection tests |
| M4 parallel capabilities | claim two capabilities in same task and inspect agent-view |
| M5 schema policy fitness | submit invalid output, assert rejection and metrics update |

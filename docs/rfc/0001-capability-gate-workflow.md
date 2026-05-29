# RFC-0001: Capability Gate Workflow

> Status: Draft
> Date: 2026-05-28
> Baseline: [`docs/vibehub-capability-redesign-2026-05-27.md`](../vibehub-capability-redesign-2026-05-27.md)
> Implementation steps: [`docs/vibehub-capability-implementation-steps-2026-05-27.md`](../vibehub-capability-implementation-steps-2026-05-27.md)

## Motivation

VibeHub's current lifecycle is driven by a fixed `align -> research -> plan -> implement -> review` phase order. The baseline records the core problem: real work is nonlinear, cross-tool handoffs are expensive, manual edits need to be reconciled later, and interruptions should resume rather than roll back. This RFC turns that baseline into an implementation contract for replacing phase-driven state with event-backed capabilities, declarative gates, and replayable projections. See baseline §1.

## Design Principles

1. **Sync is tiered and decided by VibeHub.** VibeHub chooses quick, deep, or rebuild sync from freshness signals instead of relying on the agent to guess. See baseline §2 and §6.
2. **Flow is soft, outputs are hard.** Agents may choose capability order flexibly, but persisted capability outputs must satisfy schema. See baseline §2 and §7.
3. **Checkpoints are continuation points, not rollback points.** Crash, interruption, and tool switching must lead to recovery and continuation. See baseline §2 and §9.

## Concepts

**Task** is the user's request-sized work unit and the Kanban card boundary. It contains intent, ownership hints, run history, and projected status. It is not a git branch or commit. See baseline §3.2 and §19.

**Capability** is a deliverable unit inside a task, such as `research`, `plan`, `implement`, `validate`, or `review`. Capability replaces phase as the work driver while preserving old phase names as UI and compatibility labels. See baseline §3.3 and §19.

**Custom capability** is a project-defined capability declared in `workflow.yaml` under `custom_capabilities`. It uses the same claim, gate, context pack, schema validation, release, and handoff lifecycle as built-in capabilities, but its output fields and produced artifacts are project-defined. See baseline §3.3 v2+ and the [custom capability guide](../vibehub-custom-capability-guide.md).

**Event** is an append-only fact written to `runs/<run_id>/events.jsonl`. Events are the source of truth for task and capability state, while logs and UI views are derived. See baseline §3.4, §18, and §20.

**Gate** is a declarative predicate that answers whether a capability or task transition is currently allowed. Gates replace the old hardcoded phase order with rules such as `all`, `any`, and `not`. See baseline §3.5 and §12.

**Projection** is a replayable view derived from events. `state.yaml`, `agent-view`, context packs, and UI cards consume projected fields rather than mutating flow fields directly. See baseline §3.6, §18, and §20.

## Schema Overview

S2 will define the detailed capability output schema in `docs/vibehub-capability-schema-v1.md`. Until S2 lands, implementation must follow these rules from the baseline:

- All capability outputs carry `schema_version`.
- Required fields are enforced at write time, not only during review.
- Placeholder values such as `"no_risk"` and `"none"` must be centralized.
- Errors use the `<domain>.<type>.<subclass>` form described in baseline §25.
- `required_output_schema` names and field types must align with baseline §17.5.

For custom capabilities, `workflow.yaml` registers a lightweight project schema:

```yaml
custom_capabilities:
  security_audit:
    required_fields: [audit_scope, "findings[]", severity_summary]
    optional_fields: [recommendations]
    produces: [audit_report]
    gates: [security_audit_ready]
```

The schema checker enforces required custom fields under `data`. Required field names ending in `[]` validate the base field as a non-empty array, for example `"findings[]"` validates `data.findings`. Rich per-field JSON Schema typing remains a future extension; projects should document expected field shapes in their custom capability guide or task context.

## Event Contract

M1 introduces a per-run event log with common fields `event_id`, `timestamp`, `task_id`, `actor`, and `schema_version`. The initial event set must cover baseline §20.2, including task lifecycle, capability claim/release, pack generation, evidence, planning, diff observation, validation, risks, handoff, gates, sync, schema failure, event-log corruption, and compensation events.

`event_id` format is `evt-<unix_ms>-<seq_within_ms>`. Writers must be single-owner, FIFO, append-only, and able to detect corrupted historical lines without mutating them. See baseline §18, §20, and §21.

## Migration

| Milestone | Summary | Compatibility |
|---|---|---|
| M1 | Add `events.jsonl`, event writer, read API, and double-write existing phase/handoff/checkpoint paths. | Non-breaking: `state.yaml` remains authoritative for existing UI and commands during double-write. |
| M2 | Add `capabilities:` and `gates:` to `workflow.yaml`; introduce `vibehub-claim`. | Non-breaking: old `phases:` remains parseable and old commands become aliases. |
| M3 | Move derived state fields to projection from event logs. | Potentially breaking internally: direct writes to derived fields are rejected, but CLI/UI behavior must remain compatible. |
| M4 | Allow multiple active capabilities per task and per-capability context packs/handoffs. | Behavior-changing: agents may see multiple active units; old single-phase display remains a projection for compatibility. |
| M5 | Enforce schema, `policy.yaml`, WIP limits, and fitness metrics. | Potentially breaking for invalid outputs: old tasks load with default policy, but new invalid writes are rejected. |
| M8 | Add project-defined `custom_capabilities`, custom output schema checks, custom context pack schema hints, and end-to-end security audit fixture coverage. | Non-breaking: projects without `custom_capabilities` keep preset-only behavior. Custom names that conflict with built-ins are rejected. |

## Backward Compatibility

Old command names remain valid during M1-M5. Their behavior is defined as aliases over capability operations:

| Old command / phase | Alias behavior |
|---|---|
| `align` | `vibehub-claim align` or `align_lite` when the workflow marks the lighter capability as sufficient. |
| `research` | `vibehub-claim research`. |
| `plan` | `vibehub-claim plan`. |
| `implement` | `vibehub-claim implement`. |
| `validate` | `vibehub-claim validate`. |
| `review` | `vibehub-claim review` or `review_lite` when gates allow a lightweight review. |
| `continue` / `vibehub-continue` | Resolve the next recommended claim from gates; if multiple claims are valid, report choices instead of silently picking a risky one. |
| `finish` | Check `task_finishable` gate, then release/close through events. |

Adapters may keep exposing old instruction names, but generated prompts should describe capability semantics to avoid reintroducing linear phase assumptions. See baseline §22 and §27.

## Open Questions

The following remain intentionally unresolved and are tracked in baseline §14:

- Sync thresholds for `Delta t`, git head drift, and file-overlap drift.
- Whether capability identifiers are global or task-local.
- Whether soft dependencies should exist in addition to hard gates.
- Exact placeholder constants and whether draft-mode schema writes are allowed.
- Sub-agent transport, timeout behavior, and multi-agent event writer contention.
- Whether old `schema_version = 2` tasks need historical event backfill.

## Reviewer Checklist

- [ ] The design preserves INV-1 through INV-10 from baseline §18.
- [ ] Every new write path has an event representation or an explicit reason it remains outside VibeHub state.
- [ ] Old phase commands still work as aliases and are covered by tests.
- [ ] Schema validation failures return structured errors with repair hints.
- [ ] Custom capabilities can be claimed, schema-checked, released, and used by downstream gates in an end-to-end fixture.
- [ ] Event projection can explain derived fields through a trace.
- [ ] UI changes preserve the "display and prompt generator, not direct executor" boundary.
- [ ] Tests include old project fixtures and at least one recovery/interruption path.

# VibeHub V3 View Contracts

These JSON Schema 2020-12 documents are the canonical V3 `1.0` wire
contracts consumed by M1. They describe disposable read models, not V2 YAML,
domain events, Rust structs, or React props.

## Version Policy

- `schema_version` is `1.0` for this frozen surface.
- Adding an optional field is backward-compatible within `1.x` after a
  fixture and consumer proof are added.
- Removing, renaming, changing meaning, or tightening an accepted value
  requires a new major version and parallel fixture corpus.
- Unknown enum values must produce an explicit validation or degraded-state
  result. Consumers must never silently map them to a known value.

## Shared Semantics

Every top-level view carries generation metadata, freshness, completeness,
evidence references, warnings, and structured errors from
`common.schema.json`. Native paths preserve filesystem identity in `native`
and provide a separate render-safe `display` value. Labels are stable codes;
localization belongs to the consumer.

| Field family | UX purpose | Provenance |
| --- | --- | --- |
| identity and scope | stable navigation and selection | domain identity projection |
| generated/model versions | explain which projection is shown | projection checkpoint |
| freshness/completeness | loading, stale, partial, unsupported states | projection/index lifecycle |
| evidence refs | navigate from claims to supporting material | event, Git, file, command, or user record |
| warnings/errors | non-blank degraded and failure states | validator, analyzer, adapter, or projection |
| page/window metadata | stable large-data navigation | model-version-scoped cursor |
| native/display paths | preserve machine identity while rendering safely | filesystem observation |

## Top-Level Field Map

The shared fields `schema_version`, `generated_at`, `model_version`,
`freshness`, `completeness`, `evidence_refs`, `warnings`, and `errors` render
contract compatibility, projection age/quality, evidence navigation, degraded
state, and failure recovery. Their provenance is the schema registry,
projection checkpoint, evidence index, and structured validator/adapter result.

| View | Fields | UX and evidence provenance |
| --- | --- | --- |
| ProjectOverviewView | `project_id`, `name`, `root` | project switcher/header; registered identity and observed filesystem root |
| ProjectOverviewView | `repository`, `model`, `architecture` | repository/model health and architecture summary; Git, index lifecycle, docs/manifests/parser evidence |
| ProjectOverviewView | `active_tasks`, `protocol_coverage` | active work, risk, acceptance, and recovery gaps; task/session event projection |
| ProjectStructureView | `project_id`, `index_state` | selected project and index progress/degradation; project registry and index lifecycle |
| ProjectStructureView | `nodes`, `edges`, `unsupported_analyzers` | structure/map browser and evidence drill-down; filesystem, Git, manifests, parsers, docs, explicit inference |
| ProjectStructureView | `page` | stable large-tree paging/truncation; model-version-scoped index cursor |
| TaskTimelineView | `project_id`, `task_id`, `title`, `state` | task identity/header; task event projection |
| TaskTimelineView | `criteria`, `lanes`, `events`, `window` | acceptance rollup, session/node swimlanes, ordered history, paging; criterion/session/domain events and evidence refs |
| PlanGraphView | `project_id`, `task_id`, `plan_version`, `graph_state` | selected version and invalid/stale graph state; plan event projection and validator |
| PlanGraphView | `nodes`, `scheduling_edges`, `trace_relations` | executable plan and causal repair overlay; plan/finding/attempt/validation events |
| PlanGraphView | `execution` | planned versus observed sessions/worktrees; plan plus session/worktree observations, never orchestration claims |
| NodeBrief | `project_id`, `task_id`, `node_id`, `state`, `next_intent` | recoverable scoped work header; current plan/session projection |
| NodeBrief | `goal`, `scope`, `non_scope`, `dependencies` | bounded agent assignment; accepted plan version |
| NodeBrief | `accepted_decisions`, `research_summary`, `criteria` | decision/research/acceptance context; journal, research, and criterion evidence |
| NodeBrief | `files`, `validation_commands` | focused navigation and verification actions; scoped plan plus observed evidence |
| NodeBrief | `budget`, `source_versions`, `protocol_coverage` | truncation disclosure, staleness comparison, and recovery quality; brief builder and session coverage projection |

Run `npm run v3:contracts:check` to validate schemas, references, fixtures,
coverage, deterministic generation, generated TypeScript drift, and the M1
fixture-only dependency boundary.

# M0 Task Pack: Freeze V3 Contracts, Fixtures, and Baseline

## Freeze Status (2026-07-11)

- Contract version: `1.0`
- Canonical source: JSON Schema 2020-12 under `contracts/v3/`
- TypeScript adapter: generated under `src/v3/contracts/generated/`
- Consumer boundary: `createV3FixtureRepository` in
  `src/v3/contracts/fixtureRepository.ts`
- Fixture corpus: 12 named scenarios, 60 positive view instances, three negative
  sentinels, fixed seed `20260711`
- Stable validation command: `npm run v3:contracts:check`
- Boundary result: M1 can consume every view without V2 YAML, Tauri commands,
  `.vibehub` state, or `vibehub-core` imports.

Schema SHA-256 values recorded by the validation command:

| Schema | SHA-256 |
| --- | --- |
| common | `f09e50df9615797ab45ba34f414bee849a6ff99e27e943f4ec05ec0784022d13` |
| ProjectOverviewView | `4064a58d70a6d9e7aafb0867604e7198fd7692b5bbacc4c3c7c9c04f7b42aa8d` |
| ProjectStructureView | `18bbc2ae96c2673ef86465e55d23e10fb3818759c4f255ff27c0a93a227d722b` |
| TaskTimelineView | `6ab7213b5d6c309f760a0da094c72ce86e45902b8b9b88215670871f6b64606b` |
| PlanGraphView | `bacb43de2a9015991d59d85b23892ba8adab3148df86e5c80f64311554dfaab3` |
| NodeBrief | `1b375f71fb3d7fe22c049937116c18c4ba60f7978e940e6a45e48318067197a3` |

The pre-M0 baseline remains the measured 653 startup lines / 46,680 bytes,
30,209 Rust core lines / 39 modules / 208 tests, 153 managed adapter files,
11,943 frontend lines, and no prior frontend test script, as recorded in the
V3 redesign plan. M0 adds an isolated contract check rather than modifying
those V2 production surfaces.

### Acceptance Evidence

| Criterion | Status | Evidence |
| --- | --- | --- |
| M0-C01 | passed | Ajv meta-validates all schemas, resolves every `$ref`, and checks stable versioned IDs. |
| M0-C02 | passed | 60 positive instances validate; three negative sentinels fail at the expected keyword and instance path, including mixed Windows separators. |
| M0-C03 | passed | `fixtures/v3/manifest.json` lists all 12 named scenarios and maps them to Criteria. |
| M0-C04 | passed | the check rebuilds the complete fixture tree from seed `20260711` and requires byte identity plus per-view SHA-256. |
| M0-C05 | passed (contract) | Windows drive, UNC, extended, long, case-collision, and inaccessible paths preserve native/display identity; native file operations remain `not_tested` on macOS. |
| M0-C06 | passed | common schema owns freshness, completeness, evidence, warning, page, path, and error semantics. |
| M0-C07 | passed | the check transpiles and executes `V3ViewRepository`, loading all five views for HAPPY, WIN-PATHS, REWORK, and LARGE with forbidden-dependency scans. |
| M0-C08 | passed | JSON Schema is canonical; generated TypeScript is regenerated in a temporary directory and byte-compared for drift. Rust/event round-trip is explicitly deferred to M2 in RFC-001. |
| M0-C09 | passed | all five RFC backlogs name an owner, milestone, alternatives, evidence, exit criteria, M0 decision/defer status, and non-scope. |
| M0-C10 | passed | `npm run v3:contracts:check`, `npm run build`, `cargo fmt --all -- --check`, and `cargo test --workspace` pass. |
| M0-C11 | passed | `contracts/v3/README.md` maps every top-level field family to UX purpose and evidence provenance. |
| M0-C12 | passed | `user_confirmed`: the project owner explicitly requested starting and completing M0; the executable four-scenario repository walkthrough is the M1-ready review artifact. Contract deltas discovered by visual M1 work must return as versioned M0 changes. |

Validation notes: `npm run v3:contracts:check` reports 285 assertions,
12 scenarios, and five views. `cargo test --workspace` reports 17 desktop and
208 core tests passed. The frontend build retains its pre-existing bundle-size
and stale browser-data warnings; no M0 code is included in a production runtime
path unless a future M1 consumer imports the repository.

- Task ID: `T-20260711080143-c3669d9d`
- Current V2 mode/phase: `guided_drive / align`
- Milestone: M0
- Depends on: confirmed V3 redesign plan and completed formal Research Pack
- Blocks: M1 high-fidelity fixture frontend and all later implementation milestones
- Estimated engineering window: 0.5-1 focused day, excluding native platform lab availability

## Mission

Create the smallest versioned contract and fixture boundary that lets M1 build the complete V3 Project/Task experience without V2 YAML, real V3 core, or MCP. M0 also turns five architectural unknowns into bounded RFC backlogs so later milestones can decide them with evidence instead of reopening product direction.

## Authoritative Inputs

1. `docs/vibehub-v3-redesign-plan.md`
2. `.vibehub/research/current/research-pack.md`
3. `docs/v3/rfc-backlog/001-domain-event-contract.md`
4. `docs/v3/rfc-backlog/002-mcp-control-plane-contract.md`
5. `docs/v3/rfc-backlog/003-project-intelligence-lifecycle.md`
6. `docs/v3/rfc-backlog/004-task-node-acceptance-lifecycle.md`
7. `docs/v3/rfc-backlog/005-worktree-orchestration.md`

If these conflict with V2 capability/YAML documents, the V3 redesign plan and this Task Pack govern M0.

## Definition Of Ready

- V3 direction is owner-confirmed and the parent align/research phases are complete.
- M0-M6 exist as separate VibeHub tasks.
- M0 has no code dependency on M1-M6.
- Open RFC questions are assigned to a milestone and do not block the first fixture slice.
- The active pointer is explicitly switched to `T-20260711080143-c3669d9d` before M0 output or validation.

## Scope

### In Scope

1. Contract conventions: schema version, IDs, timestamps, freshness/completeness, evidence refs, pagination, and structured errors.
2. Five JSON Schema 2020-12 read models: `ProjectOverviewView`, `ProjectStructureView`, `TaskTimelineView`, `PlanGraphView`, and `NodeBrief`.
3. Deterministic schema-valid fixtures for happy and non-happy states.
4. A fixture manifest mapping scenarios to contracts and M0/M1 Criteria.
5. One validation command for schemas, refs, fixtures, versions, coverage, and deterministic generation.
6. One TypeScript/Rust generation or round-trip spike, with the decision recorded in RFC-001.
7. Baseline measurements and forbidden-dependency checks that keep M1 fixture-only.
8. Triage/freeze of five RFC backlog boundaries and exit criteria.

### Out Of Scope

- V3 production event store, Application Service, MCP server, CLI adapter, cross-process locking, or real projections.
- M1 visual components beyond a minimal fixture-loader proof.
- M3 analyzers/indexes, M4 workflow implementation, M5 worktree commands, or M6 migration.
- Rewriting or deleting V2 workflow code.
- Selecting every future event payload, optional MCP feature, language analyzer, or Windows worktree root without a spike.

## Contract Conventions To Freeze

Every top-level view must include:

- `schema_version`: semver-like contract version, initially `1.0`.
- Stable entity ID and project/task scope as applicable.
- `generated_at` and source/model version.
- `freshness`: `fresh | stale | rebuilding | unavailable`.
- `completeness`: `complete | partial | unsupported | unknown`.
- `evidence_refs`: typed, navigable references with evidence grade.
- `warnings`: structured code, severity, message key, and evidence refs.
- Optional `page` metadata where collections can exceed one response.

Rules:

1. Missing, empty, unavailable, stale, partial, unsupported, and error are different states.
2. User-facing labels are localized from stable codes, not stored as canonical enum values.
3. Paths preserve native identity and carry a separate display representation.
4. Unknown enum/event values fail visibly or degrade explicitly according to policy.
5. M1 component props may narrow a view but cannot redefine the wire contract.

## View Contract Minimums

### ProjectOverviewView

- Project identity/root/platform/repository summary.
- Project model lifecycle and last evidence update.
- Module/architecture summary with evidence and confidence.
- Active Task summary, risk/criterion rollup, usage provenance, and protocol coverage.
- Explicit no-repo/no-docs/unindexed/stale/partial states.

### ProjectStructureView

- Versioned tree/graph page, stable node IDs, native/display paths, kind, and Git overlay.
- Module/dependency edges with source kind, confidence, and evidence.
- Pagination/search cursor, truncation reason, index lifecycle, and unsupported analyzers.
- IDE target metadata without assuming one IDE or shell.

### TaskTimelineView

- Task/criterion summary and ordered event/session lanes.
- Decision, evidence, finding, attempt, validation, confirmation, and gap event kinds.
- Actor/tool/session/node/commit association and expandable evidence.
- Pagination/window metadata and late/reordered display behavior.

### PlanGraphView

- Plan version, nodes, scheduling edges, readiness/block reasons, scope, and Criteria links.
- Separate trace relations for repairs/addresses/validates/supersedes.
- Planned versus observed session/worktree summary without implementing orchestration.
- Cycle/unknown-node/changed-plan/stale states.

### NodeBrief

- Goal, scope/non-scope, dependencies, accepted decisions, research summary, Criteria, files/evidence, validation commands, current state, and next intent.
- Token/size budget metadata, truncation sections, source versions, and protocol coverage.
- No embedded repository dump or complete historical timeline.

## Fixture Matrix

| Scenario ID | Required coverage | Platform dimension | Size |
|---|---|---|---|
| FX-EMPTY | Empty/uninitialized project, no tasks, no architecture docs | macOS + Windows roots | small |
| FX-HAPPY | One active task, complete evidence, fresh structure, passing criteria | neutral | small |
| FX-NO-DOCS | Hard file/manifest facts without declared architecture | neutral | small |
| FX-PARALLEL | Multiple tasks/sessions/nodes with non-overlap and one overlap warning | neutral | medium |
| FX-REWORK | Finding -> remediation -> re-review with two attempts | neutral | medium |
| FX-STALE | Stale project model plus newer Git/task events | neutral | small |
| FX-PARTIAL | Unsupported analyzer, inaccessible subtree, truncated/paged results | neutral | medium |
| FX-ERROR | Structured recoverable and terminal errors without blank UI | macOS + Windows | small |
| FX-WIN-PATHS | Drive, UNC, extended-length, long component, case collision, locked path | Windows | medium |
| FX-MAC-PATHS | Spaces, Unicode, symlink, package/bundle path, case-variant display | macOS | medium |
| FX-LARGE | Deterministically generated large tree/timeline/graph with stable seed | both viewports | generated |
| FX-COVERAGE-GAP | Missing session open/close, recovered gap, unknown host capability | all hosts | small |

Each scenario directory must contain a manifest naming its seed, contract files, expected warnings/states, and covered Criteria. FX-LARGE must be generated and hash-checked, not stored as an opaque giant snapshot.

## Planned File Surface

Create or update only this surface unless the task records a justified delta:

```text
contracts/v3/
  README.md
  common.schema.json
  project-overview-view.schema.json
  project-structure-view.schema.json
  task-timeline-view.schema.json
  plan-graph-view.schema.json
  node-brief.schema.json
fixtures/v3/
  manifest.json
  <scenario-id>/...
scripts/v3-contracts/
  validate.*
  generate-large-fixture.*
src/v3/contracts/          # generated or verified TS surface, based on spike
crates/vibehub-core/tests/ # Rust contract tests only if selected
package.json               # stable validation command if Node tooling is selected
docs/v3/rfc-backlog/*      # decision updates only
docs/v3/m0-task-pack.md    # evidence/status updates only
```

Do not edit V2 event/projection/capability/UI production modules in M0.

## Work Plan

### WP0 - Baseline and Guardrails

Record file count, protocol-tax measurements, shallow scanner limits, frontend test gap, and the M0 allowlist. Add a check that the V3 fixture/UI boundary does not import V2 YAML or invoke V2 Tauri workflow commands.

### WP1 - Common Schema Vocabulary

Choose JSON Schema IDs/ref layout, version policy, evidence/path/freshness/error/page definitions, and fixture manifest schema. Complete the RFC-001 source-of-truth spike.

### WP2 - Five Read Models

Implement the five schemas from the minimums above. Keep domain/event internals opaque; include only information the Project/Task experiences need.

### WP3 - Fixture Corpus

Build FX-EMPTY through FX-COVERAGE-GAP and validate every instance. Add stable seed/hash behavior for FX-LARGE; repeated generation must be byte-identical.

### WP4 - Consumer Proof

Load at least FX-HAPPY, FX-WIN-PATHS, FX-REWORK, and FX-LARGE through a minimal TypeScript repository interface; round-trip representative views through Rust if RFC-001 selects/generated Rust types. This is a contract proof, not M1 UI.

### WP5 - RFC Triage

For each RFC, mark every M0 decision as decided/deferred/blocked, attach spike evidence, and preserve later-milestone questions. No RFC may claim host/platform support without a test.

### WP6 - Freeze and Handoff

Run validation, record contract hashes/versions, document extension points, and prepare an M1 handoff that points only to schemas, fixtures, the fixture repository interface, and visual Criteria.

## Acceptance Criteria

- **M0-C01**: All five schemas declare JSON Schema 2020-12, stable `$id`, version, and no unresolved `$ref`.
- **M0-C02**: Every fixture validates against its declared schema; invalid sentinels fail with expected code/path.
- **M0-C03**: All 12 named fixture scenarios exist and map to Criteria.
- **M0-C04**: Repeated FX-LARGE generation with the same seed is byte/hash identical.
- **M0-C05**: Windows drive/UNC/extended/long/case/locked examples preserve identity and render-safe display values.
- **M0-C06**: Freshness, completeness, evidence, warning, pagination, and error semantics are shared, not redefined per view.
- **M0-C07**: M1 loads all views through one fixture repository without V2 YAML/Tauri/core dependencies.
- **M0-C08**: Source-of-truth/codegen is recorded with a round-trip spike and drift test.
- **M0-C09**: Five RFC backlogs each have owner, milestone, alternatives, evidence, exit criteria, and explicit non-scope.
- **M0-C10**: `npm run build` and the new stable contract command pass.
- **M0-C11**: Contract docs map every top-level field to UX and evidence provenance; no unexplained metric remains.
- **M0-C12**: The project owner reviews the M1-ready fixture walkthrough and approves contract `1.0` or records changes.

## Validation Plan

M0 must add a stable repository command, proposed name:

```bash
npm run v3:contracts:check
```

It must perform schema meta-validation, ref resolution, positive fixtures, negative sentinels, manifest coverage, deterministic generator hash, and generated-type drift checks.

Also run:

```bash
npm run build
cargo fmt --all -- --check
cargo test --workspace
git diff --check
```

Native/visual checks before M0 exit:

1. Load FX-WIN-PATHS on Windows and verify JSON parsing/path display; no file-operation claim yet.
2. Load FX-MAC-PATHS on macOS and verify symlink/Unicode/display behavior.
3. Render longest labels/paths and largest graph/timeline at the M1 desktop minimum and compact viewport; M0 verifies stress inputs, not final polish.
4. Confirm no M0 executable path starts MCP or writes canonical V3 state.

## Risks And Stop Conditions

- Stop if schemas mirror V2 YAML or current component props; rewrite around user-facing views.
- Stop if a fixture needs a real backend call; represent the state or defer interaction to M2.
- Stop if codegen requires broad build churn; keep JSON Schema canonical and record a narrower verified adapter.
- Record `not_tested` if native evidence is unavailable; never infer platform success.
- Do not expand M0 into event store, MCP, Project Intelligence, PlanGraph execution, or worktree implementation.

## M1 Handoff Contract

M1 receives five frozen `1.0` schemas, the fixture manifest/generator, one typed fixture repository, field-to-evidence/state documentation, and visual/user-flow Criteria.

M1 does not receive V2 state paths, V2 commands, a mock event store, or permission to reinterpret missing fields. Contract gaps return to M0 as explicit versioned deltas before M2 implements real views.

## Completion And Next Action

After M0 validation and owner approval, complete M0 and switch to M1 task `T-20260711080143-49b5012c`. Do not start M2 until M1 validates the information architecture and accepted contract deltas are frozen.

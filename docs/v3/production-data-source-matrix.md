# V3 production data-source matrix

Date: 2026-07-13
Scope: final production route, fixture/debug route, and retained M1 surfaces

This matrix is the final source contract for the V3 product surfaces. It
classifies what users see as **real**, **derived**, **fixture**, **mock**,
**placeholder**, or **unsupported**. A classification describes the source and
truthfulness boundary; it does not turn an unavailable capability into a zero
or an estimate.

## Classification rules

| Classification | Meaning in V3 |
| --- | --- |
| **Real** | Read from a project-local V3 repository, legacy archive, or supported local usage source through an explicit production loader. |
| **Derived** | Computed from real project records, filesystem/index facts, or supported source records. The UI labels warnings, freshness, partiality, and gaps rather than treating a derivation as a new primary fact. |
| **Fixture** | Deterministic contract data loaded only by the explicit debug playground or by a debug scenario selection. It is not production data. |
| **Mock** | Synthetic runtime data presented as if it were production data. **There is no V3 production mock data path.** Existing deterministic test scenarios are fixtures, not mocks. |
| **Placeholder** | A static loading, empty, error, or unavailable presentation before/without an authoritative value. It must not be interpreted as a measurement or project fact. |
| **Unsupported** | A capability intentionally has no stable, validated, project-attributable source. It is shown as unavailable/unsupported and excluded from aggregates. |

## Route and command boundary

The production route in [`src/pages/Home.tsx`](../../src/pages/Home.tsx) is the
only normal project entry that injects the following functions into
`V3Cockpit`:

| Production dependency | Native boundary | Classification |
| --- | --- | --- |
| `loadV3ProductionViews` | `v3_load_view_bundle` | Real/derived V3 project view bundle |
| `loadLegacyV2Archive` | `legacy_v2_load_archive` | Real legacy archive read model |
| `vibehubReadLocalAgentUsage` | `vibehub_read_local_agent_usage` | Real supported local usage plus explicit unsupported-source summaries |
| V3 lifecycle API | `v3_inspect_project_layout`, `v3_initialize_project`, `v3_migrate_project`, `v3_recover_project_migration` | Real project layout and bounded lifecycle operations |
| V3 task creation | `v3_create_task` | Bounded production write |
| Legacy/project-file actions | `vibehub_open_vibehub_file`, `vibehub_reveal_project_file`, `vibehub_open_project_file` | Bounded production external-open actions |

`loadV3ProductionViews` in [`src/services/v3ProductionViews.ts`](../../src/services/v3ProductionViews.ts)
loads the production bundle through `v3_load_view_bundle`. The independent
legacy loader in [`src/services/legacyV2.ts`](../../src/services/legacyV2.ts)
loads only `legacy_v2_load_archive`.

The fixture playground in `Home` is reachable only when
`isV3PlaygroundRequested()` permits it. It passes only `initialSourceMode="fixture"`,
`debugMode`, and `onBack`; it receives none of the production loaders,
lifecycle APIs, task-create callback, legacy-file callback, or project-file
actions. In [`src/v3/stores/v3Store.ts`](../../src/v3/stores/v3Store.ts),
selecting a fixture clears production loaders/API references and invalidates all
read-channel request IDs before the fixture bundle is loaded.

A debug scenario selected after opening a production cockpit intentionally
switches the cockpit to fixture source mode. That preserves the M1 scenario
control while isolating fixture data and commands from production state.

## User-visible surface matrix

| Surface or field family | Classification in normal production | Authoritative source / production behavior | Fixture/debug behavior | Truthfulness rule |
| --- | --- | --- | --- | --- |
| Project name, project ID, root, repository branch and freshness | Real + derived | V3 view bundle loaded by `v3_load_view_bundle`; presentation/freshness comes from the returned versioned view. | Deterministic scenario bundle only. | A missing or stale value stays missing/stale; a fixture label never identifies a real project. |
| Active task list, titles, task state, acceptance criteria, risk, session counts | Real + derived | Project overview and task timeline views from the V3 repository. | Scenario-supplied active tasks. | Empty state means no supplied active task; it is not generated work. |
| Acceptance progress | Derived | Calculated/displayed from V3 task criteria and completion state. | Scenario-supplied criteria state. | No criterion is marked complete from a fabricated percentage. |
| Timeline events, actors, timestamps, commit references, event details | Real + derived | Ordered V3 timeline view from event/projection data. | Deterministic synthetic test events. | Unordered/gap/partial conditions are displayed as warnings rather than silently repaired. |
| Plan graph, dependencies, readiness, warnings, and trace relations | Derived | V3 plan view derives product-facing graph data from project records; source evidence/warnings remain in the view contract. | Scenario graph data. | A missing or unsupported analyzer does not become a fabricated plan edge or agent metric. |
| Node brief, goal, scope, decisions, research summary, validations, next intent | Real + derived | V3 node-brief view supplied by the production repository. | Scenario node brief. | The UI presents supplied fields only; omitted fields remain absent. |
| Project structure, file/directory identity, paths, Git overlay, architecture/module facts | Real + derived | V3 project-structure view and bounded project index/file actions. | Scenario structure data, including platform-path test cases. | Native/display paths stay distinct; inaccessible/unsupported facts remain explicit. |
| Project file reveal/open controls | Real action | Production route injects bounded Tauri actions, restricted by backend path validation. | No callback exists. | Any failure is surfaced locally; fixture mode cannot open a real file. |
| V3 bootstrap/lifecycle status, error, confirmation, result | Real + placeholder | Production lifecycle API inspects before load and maps only compatible states to initialize/migrate/recover. | No lifecycle API exists. | Loading/error/confirmation UI is placeholder state, not a claim that migration occurred. Conflict fails closed. |
| V3-native create-task action | Real action + placeholder | Production route injects `v3CreateTask`; task state is refreshed after successful creation. | No callback exists. | Form/pending/error content is a placeholder interaction state; fixture mode cannot create a project task. |
| Legacy archive list, task title/state/phase/time/summary/warnings | Real | Independent `legacy_v2_load_archive` reader reads the project-local `legacy-v2` archive. | No legacy loader exists. | Archive values remain source values; V3 does not fabricate archive usage or token history. |
| Legacy archive file links | Real action + placeholder | Production-only bounded legacy-file open callback. | No callback exists. | Missing/traversal/shell failure remains an error; fixture mode cannot open an archive file. |
| Supported local usage: Claude Code | Real + derived | Read-only project-attributed local records; deduplicated token/session summaries. | No production reader in fixture route. | Prompt/message bodies are not exposed; malformed/ambiguous records become partial or unavailable. |
| Supported local usage: Codex | Real + derived | Read-only SQLite/rollout records with exact path or explicitly warned unique same-name fallback. | No production reader in fixture route. | Ambiguous attribution fails closed and does not aggregate. |
| Supported local usage: OpenCode | Real + derived | Read-only SQLite session/project records with exact or explicitly warned unique same-name fallback. | No production reader in fixture route. | Source-recorded cost can remain source metadata; it is not an actual cross-provider bill. |
| Usage total tokens, cache split, source count, freshness and completeness | Derived from real supported sources | `vibehub_read_local_agent_usage` aggregates only Claude Code, Codex, and OpenCode records. | Scenario UI may show fixture values only in explicit fixture mode. | Unsupported sources never enter totals or source count. Stale/partial/empty/error are distinct states. |
| M1 token summary and project usage drawer | Derived + placeholder | Uses the production usage overview when available. | Fixture data is local to the selected scenario. | No result renders “暂无”; it is not numeric zero usage. |
| Actual cross-provider subscription bill / aggregate cost card | Unsupported | No authoritative billing source exists; the M1 surface says `费用不可用`. | Fixture may exercise display layout but does not establish a bill. | Static model-price estimates and zero-valued transport fields are never presented as an actual bill. |
| Claude App ordinary chat usage | Unsupported | No stable project-scoped source is read; reader emits explicit `claude_app` unsupported summary. | No production reader in fixture route. | Never aggregated, never represented as zero usage. |
| Cursor usage | Unsupported | No stable project-scoped source is read; reader emits explicit `cursor` unsupported summary. | No production reader in fixture route. | Never aggregated, never represented as zero usage. |
| External-agent phase distribution and quota | Unsupported | Current external records have no stable mapping to V3 phase or quota contract. | Scenario values only exercise fixture states. | The M1 surface remains, with unsupported wording rather than an inferred value. |
| Model/tool activity and session metadata | Real where recorded; otherwise unsupported/placeholder | Supported reader returns safe model/tool/session metadata only when present. | Scenario data only. | Missing metadata remains unknown; no message or prompt content is displayed. |
| Refresh control | Real command orchestration + placeholder | Reloads the current production bundle, legacy archive, and usage readers through their injected loaders. | Reloads fixture bundle only after source-mode isolation. | Spinner/loading state indicates request status, not a new authoritative observation. |
| Debug indicator | Real configuration state | `V3_DEBUG_ENABLED` reflects the build/runtime debug flag. | Enabled in explicit playground. | It is a control-state label, not product/project data. |
| Scenario selector | Fixture control | Visible only under `debugMode`; choosing it clears production dependencies before fixture load. | Selects deterministic `FX-*` data. | It never changes a project or invokes production native commands. |
| Settings/setup control | Real UI control + placeholder | Retained debug-mode settings/setup presentation; it does not create mock data. | Same visual control is retained for fixture/debug exercise. | Presence of the control is not evidence of a production project mutation. |
| Loading, empty, retry, error, unavailable labels | Placeholder | Rendered from real request/status outcomes when production loader calls are pending, empty, or rejected. | Rendered from fixture loader state. | They are explicit UI states, not facts about absent work or zero metrics. |

## Production invariants

1. **No production mock path.** The normal project route has explicit production
   loaders and commands. Deterministic `FX-*` records live behind fixture/debug
   selection and are never passed as a normal project bundle.
2. **No fixture command escalation.** Fixture mode has no lifecycle, task-create,
   legacy-file, project-file, production bundle, legacy archive, or usage
   callback. Selecting a fixture nulls previously retained production callbacks.
3. **Unsupported is not zero.** Claude App, Cursor, quota, external-agent phase
   mapping, and actual cross-provider billing are explicit unavailable states;
   unsupported counters do not participate in aggregates.
4. **Read-only source boundaries remain read-only.** Local usage readers do not
   modify source files/databases or expose prompt/message bodies. `legacy-v2`
   remains an archive reader rather than a new V3 write path.
5. **M1 is preserved.** Usage/token/cost, scenario, fixture, debug, settings,
   refresh, task, timeline, plan, structure, and archive surfaces remain in the
   product. Isolation is enforced by data and command injection rather than by
   deleting those controls.

The contract check in [`scripts/v3-contracts/check.mjs`](../../scripts/v3-contracts/check.mjs)
contains static assertions for these boundaries. The usage-capability rationale
is detailed in [usage source capabilities](usage-source-capabilities.md).

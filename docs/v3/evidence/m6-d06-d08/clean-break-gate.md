# Batch M6 — D06, D07, D08: old Cockpit removal and clean-break gate

Date: 2026-07-13
Platform: macOS arm64
Branch: `feature/vibehub-v2-p0`

## Result

D06, D07, and D08 are VERIFIED. The unreachable legacy Cockpit components, their exclusive TS wrappers, their exclusive Tauri command definitions and registrations, and their exclusive shared types have been removed. A static production gate now proves no retired canonical write, V2 schema import, duplicate adapter/dispatcher, or old Cockpit reachability remains.

## D06 — Deleted unreachable old Cockpit components

### Deleted files

| File | Reason |
| --- | --- |
| `src/components/VibehubCockpitDialog.tsx` | Root old Cockpit dialog. No production route imported it after D05. Only `VibehubProjectCenter` (also deleted) and the deleted `ProjectDetailBoard`/`ProjectStructureExplorer` imported its exports. |
| `src/components/VibehubProjectCenter.tsx` | Second legacy-content host wrapping `VibehubCockpitContent`. No production route rendered it. |
| `src/components/ProjectDetailBoard.tsx` | Exclusive child of `VibehubCockpitDialog`. Imported helpers only from `VibehubCockpitDialog`. No external consumer. |
| `src/components/ProjectStructureExplorer.tsx` | Exclusive child of `VibehubCockpitDialog`. Imported `labelOrFallback` only from `VibehubCockpitDialog`. No external consumer. |

### Consumer scan evidence

- `grep -rn "VibehubCockpitDialog|VibehubProjectCenter|ProjectDetailBoard|ProjectStructureExplorer" src/` → no matches (after deletion).
- `grep -rn "VibehubCockpitDialog|VibehubProjectCenter|ProjectDetailBoard|ProjectStructureExplorer" src/v3/` → no matches (v3 never imported old Cockpit).
- No dangling imports remain in any `src/` file.

### Deleted shared types (old-Cockpit-exclusive)

Removed from `src/types/index.ts`:
`ContextPackBuildResult`, `PendingReplayResult`, `DebugDumpOptions`, `DebugDumpResult`, `VibehubStartTaskResult`, `VibehubIntakeConfidence`, `VibehubTaskDraft`, `VibehubStartTaskIntakeRequest`, `VibehubTaskIntakeItem`, `VibehubTaskIntakeFailure`, `VibehubStartTaskIntakeResult`, `AgentTool`, `AgentCommandSpec`, `AgentAdapterConflict`, `AgentAdapterFileStatus`, `AgentAdapterStatus`, `AgentAdapterConfig`, `AgentAdapterConfigPatch`, `AgentAdapterSyncResult`, `WorkspaceDriftReport`, `VibehubFileStatus`, `VibehubFlowPhaseStatus`, `VibehubTaskNeighbor`, `VibehubActiveTask`, `CapabilityGateStatus`, `VibehubCockpitStatus`, `VibehubJournalAppendResult`, `VibehubKnowledgeAppendResult`, `PhaseValidationResult`, `PhaseAdvanceResult`, `PhaseSetResult`, `ResearchPackBuildResult`, `ResearchPackArchiveResult`, `ResearchStatus`, `VibehubFileReadResult`, `VibehubContextViewData`, `VibehubReviewViewData`, `VibehubHandoffViewData`, `VibehubDiffViewData`, `VibehubSyncReport`, `VibehubProjectDigest`, `VibehubGitBranchInfo`, `VibehubGitBranchesView`, `VibehubFlowArtifact`, `VibehubFlowDetail`, `VibehubEventTimelineItem`, `VibehubArchiveArtifact`, `VibehubArchivedTaskEvent`, `VibehubArchivedProcessStep`, `VibehubArchivedTaskCard`, `VibehubArchiveViewData`, `VibehubProjectStructureKind`, `VibehubProjectStructureGraphNode`, `VibehubProjectStructureGraphEdge`, `VibehubProjectStructureTreeNode`, `VibehubProjectStructureViewData`, `VibehubPromptTemplateId`, `VibehubPromptTemplateOption`, `VibehubPromptRenderResult`, `VibehubCockpitOverview`, `VibehubStateMigrationReport`.

### Retained shared types (production-consumed)

Kept in `src/types/index.ts`:
V3 types (`V3ProjectLayoutState`, `V3ProjectLayoutStatus`, `V3BootstrapResult`, `V3TaskCreateRequest`, `V3TaskCreateResult`), app-level types (`ProjectType`, `TagCategory`, `TagConfig`, `ToolType`, `Theme`, `ProjectMetadata`, `Project`, `Workspace`, `Tag`, `AppConfig`, `SettingsImportAdjustment`, `SettingsImportResult`), production usage types (`LocalAgentUsageOverview`, `AgentUsagePrimaryMetricKind`, `AgentUsagePrimaryMetric`, `AgentUsageSourceSummary`, `AgentUsageTokenBreakdown`, `AgentUsageRecentItem`), and storage types (`StorageSource`, `StorageInfo`).

## D07 — Removed V3-replaced aliases, commands, and wrappers

### Deleted TS wrappers (22)

Removed from `src/services/tauri.ts`:
`vibehubInit`, `vibehubStartTask`, `vibehubStartTaskIntake`, `vibehubGenerateAgentView`, `vibehubGetAgentAdapterStatus`, `vibehubUpdateAgentAdapterConfig`, `vibehubSyncAgentAdapters`, `vibehubCheckWorkspaceDrift`, `vibehubSyncWorkspace`, `vibehubBuildContextPack`, `vibehubBuildHandoff`, `vibehubDebugDump`, `vibehubGenerateReviewEvidence`, `vibehubReadOverview`, `vibehubReadProjectDigest`, `vibehubListPromptTemplates`, `vibehubRenderPrompt`, `vibehubValidatePhase`, `vibehubPausePhase`, `vibehubReadVibehubFile`, `vibehubRevealVibehubFile`, `vibehubSetProjectLocale`.

### Retained TS wrappers (production-consumed)

Kept in `src/services/tauri.ts`:
`vibehubReadLocalAgentUsage` (usage reader), `vibehubOpenVibehubFile` (legacy archive file open), `vibehubRevealProjectFile` (bounded project file reveal), `vibehubOpenProjectFile` (bounded project file open), plus all V3 wrappers (`v3InspectProjectLayout`, `v3InitializeProject`, `v3MigrateProject`, `v3RecoverProjectMigration`, `v3CreateTask`) and all app-level wrappers.

### Deleted Tauri command definitions (26)

Removed from `src-tauri/src/commands.rs`:
`vibehub_init`, `vibehub_start_task`, `vibehub_start_task_intake`, `vibehub_build_context_pack`, `vibehub_generate_agent_view`, `vibehub_get_agent_adapter_status`, `vibehub_update_agent_adapter_config`, `vibehub_sync_agent_adapters`, `vibehub_check_workspace_drift`, `vibehub_sync_workspace`, `vibehub_workflow_explain`, `vibehub_switch_task`, `vibehub_classify_file_ownership`, `vibehub_query_task_neighbors`, `vibehub_debug_dump`, `vibehub_build_handoff`, `vibehub_generate_review_evidence`, `vibehub_read_overview`, `vibehub_read_project_digest`, `vibehub_list_prompt_templates`, `vibehub_render_prompt`, `vibehub_validate_phase`, `vibehub_pause_phase`, `vibehub_read_vibehub_file`, `vibehub_reveal_vibehub_file`, `vibehub_set_project_locale`.

Also removed: `VibehubStatusChangedEvent` struct, `emit_vibehub_status_changed` helper, `emit_on_success` helper (all only used by removed commands).

### Deleted Tauri command registrations (26)

Removed from `src-tauri/src/main.rs`: same 26 commands removed from the `invoke_handler` registration list.

### Retained Tauri commands (4)

Kept in both `commands.rs` and `main.rs`:
`vibehub_read_local_agent_usage` (production usage reader), `vibehub_open_vibehub_file` (legacy archive open), `vibehub_reveal_project_file` (bounded project file reveal), `vibehub_open_project_file` (bounded project file open).

### Cleaned-up imports

`src-tauri/src/commands.rs` import block reduced from 20 `vibehub::*` module imports to 2: `vibehub::cockpit` (for `resolve_vibehub_file_path` used by retained `vibehub_open_vibehub_file`) and `vibehub::project_structure` (for `resolve_project_file_path` used by retained file-action commands). Also removed `tauri::Emitter` (only used by removed emit helpers), keeping `tauri::State`.

### Independently retained backend paths (CLI consumers)

The following backend modules are NOT removed because the CLI dispatcher (`crates/vibehub-cli/src/dispatcher.rs`) independently consumes them via `vibehub_core::vibehub::*` (or `vibehub_core::v3::*` for V3). The desktop binary shares this dispatcher via `vibehub_adapters::dispatcher::dispatch` in `main.rs`.

| CLI action | Backend function | Module |
| --- | --- | --- |
| `start` | `start_task::start_task` | `vibehub::start_task` |
| `start-intake` | `start_task::start_task_intake` | `vibehub::start_task` |
| `sync` / `continue` | `sync::sync_workspace` | `vibehub::sync` |
| `status` | `status::read_cockpit_status` | `vibehub::status` |
| `next-action` | `next_action::recommend_next_action_with_intent` | `vibehub::next_action` |
| `adapter-status` | `agent_adapter::get_agent_adapter_status` | `vibehub::agent_adapter` |
| `sync-adapters` | `agent_adapter::sync_agent_adapters` | `vibehub::agent_adapter` |
| `review` | `review::generate_review_evidence_with_locale` | `vibehub::review` |
| `recover` | `drift::sync_workspace_state` | `vibehub::drift` |
| `handoff` | `handoff::build_handoff` | `vibehub::handoff` |
| `pause` | `phase::pause_current_phase` | `vibehub::phase` |
| `validate` | `phase::validate_phase` | `vibehub::phase` |
| `validate-task` | `phase::validate_phase_for_task` | `vibehub::phase` |
| `output-lint` | `output_lint::lint_output_for_task` | `vibehub::output_lint` |
| `advance` | `phase::advance_phase_with_force` | `vibehub::phase` |
| `finish` | `phase::complete_phase` | `vibehub::phase` |
| `workflow-explain` | `workflow::explain_workflow` | `vibehub::workflow` |
| `switch` | `task_switch::switch_task` | `vibehub::task_switch` |
| `ownership` | `ownership::classify_workspace_ownership` | `vibehub::ownership` |
| `record` | `ownership::record_file_ownership` | `vibehub::ownership` |
| `neighbors` | `neighbors::query_current_task_neighbors` | `vibehub::neighbors` |
| `archive` | `archive::archive_completed_tasks` | `vibehub::archive` |
| `claim` | `capability::claim_capability` | `vibehub::capability` |
| `gates` | `capability::evaluate_capability_gates` | `vibehub::capability` |
| `schema-check` | `schema_check::validate_current_capability_output_file` | `vibehub::schema_check` |
| `migrate` | `state_migration::migrate` / `dry_run` | `vibehub::state_migration` |
| `debug-dump` | `debug_dump::create_debug_dump` | `vibehub::debug_dump` |
| `locale` | `locale::persist_project_locale` | `vibehub::locale` |
| `replay-pending` | `events::replay_pending_events` | `vibehub::events` |
| `v3 doctor/init/migrate/migrate-recover` | `v3::inspect_project_layout` etc. | `vibehub_core::v3` |
| `mcp-stdio` | `V3McpServer` | `vibehub_core::v3` |

The MCP server (`crates/vibehub-cli/src/mcp.rs`) is purely V3 and does not use any old `vibehub_*` Tauri commands.

## D08 — Static production gate

Added 81 new assertions to `scripts/v3-contracts/check.mjs` (total now 539, up from 458). The gate proves:

1. Production route (`Home.tsx`) does not import old Cockpit components (`VibehubCockpitDialog`, `VibehubProjectCenter`, `ProjectDetailBoard`, `ProjectStructureExplorer`).
2. No retired canonical-write or protocol-era commands are registered in `main.rs` (26 commands checked individually).
3. No retired TS wrappers have residual in `tauri.ts` (22 wrappers checked individually).
4. Retained vibehub commands are still registered in `main.rs` (4 commands checked).
5. Desktop binary shares the CLI dispatcher for headless mode (`dispatcher::dispatch` in `main.rs`).
6. No retired old-Cockpit-exclusive types have residual in `types/index.ts` (24 types checked individually).
7. Legacy-v2 reader does not import V3 modules or the shared types module (stays independent).
8. Production `LocalAgentUsageOverview` type is retained.

Pre-existing assertions continue to prove:
- `ProjectCard` cannot reach or mount the legacy Cockpit.
- Fixture playground receives no production lifecycle, project-file, or task-mutation actions.
- Production V3 route injects bounded task-create, project-file, and usage actions.
- M1 usage/token/cost/scenario/fixture/debug/settings surfaces are preserved.
- V3 components do not directly depend on Tauri, V2 YAML, or `.vibehub/` state.

## Automated verification

Commands and results:

```text
npm run v3:contracts:check
passed: 539 assertions, 12 scenarios, 6 views, 2 write contracts

npm run build
TypeScript and Vite production build passed (3706 modules transformed)

cargo test -p vibehub-core
275 passed; 0 failed; 0 ignored

cargo test -p vibehub-cli
2 passed; 0 failed; 0 ignored

cargo test --manifest-path src-tauri/Cargo.toml
23 passed; 0 failed; 1 ignored; 0 measured

cargo build -p vibehub-cli
passed

cargo check --manifest-path src-tauri/Cargo.toml
passed (7 pre-existing dead-code warnings, no errors)

git diff --check
passed with no output
```

Residual scans:

```text
grep -rn "VibehubCockpitDialog|VibehubProjectCenter|ProjectDetailBoard|ProjectStructureExplorer" src/
no matches

grep -rn retired TS wrappers in src/ (excluding types/index.ts)
no matches

grep "commands::vibehub" src-tauri/src/main.rs
4 matches (only the retained commands)

grep retired types in src/types/index.ts
0 matches
```

## Browser evidence

No browser-visible UI changes were made. The deleted components were unreachable from both the production route (`Home.tsx` → `V3Cockpit`) and the fixture playground (`V3Cockpit` in fixture mode). The production-injected TS wrappers (`vibehubReadLocalAgentUsage`, `vibehubOpenVibehubFile`, `vibehubRevealProjectFile`, `vibehubOpenProjectFile`, `v3CreateTask`, V3 lifecycle) are all retained. The `npm run build` and `npx tsc --noEmit` both pass, confirming no broken imports or type errors.

## Remaining blockers

None for D06, D07, or D08. The pre-existing macOS Accessibility/System Events permission blocker for A08, B05–B07, C05–C06, F04, and E06 remains unchanged and is not affected by this batch.

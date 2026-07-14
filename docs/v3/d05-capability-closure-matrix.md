# D05 legacy Cockpit capability closure matrix

Date: 2026-07-12

## Decision boundary

The project-card body opens the production `V3Cockpit`, while the project-card context menu still opens `VibehubCockpitDialog`. Removing that context-menu entry would remove user-reachable capabilities that production V3 does not currently replace.

This matrix treats a capability as closed only when an equivalent user-reachable surface exists. A registered Tauri command or frontend wrapper by itself is not an equivalent user entry.

## Matrix

| Capability | Current user entry | Wrapper / command | Access | Equivalent production V3 or other stable entry | Gap | Recommendation |
| --- | --- | --- | --- | --- | --- | --- |
| Inspect and initialize an absent V3 layout | Project-card body → production lifecycle modal | `v3InspectProjectLayout` / `v3_inspect_project_layout`; `v3InitializeProject` / `v3_initialize_project` | Read + write | Yes. Production V3 inspects first and offers initialize only for `absent`. | Native Tauri UI evidence is still missing because Accessibility automation is unavailable in the current environment. | Keep production V3 entry; do not use the legacy Cockpit as the V3 bootstrap replacement. |
| Migrate V2 to V3 | Project-card body → production lifecycle modal | `v3MigrateProject` / `v3_migrate_project` | Write | Yes. Production V3 requires explicit legacy-archive confirmation. | Native Tauri UI evidence is still missing. | Keep production V3 entry. |
| Recover interrupted V3 migration | Project-card body → production lifecycle modal | `v3RecoverProjectMigration` / `v3_recover_project_migration` | Write | Yes, for `migration_interrupted`; conflict remains fail-closed. | Native Tauri UI evidence is still missing. | Keep production V3 entry. |
| Initialize the retired protocol workspace and select agent adapters | Project-card context menu → VibeHub Cockpit → uninitialized setup | `vibehubInit` / `vibehub_init` | Write | No. Production V3 bootstrap intentionally does not configure adapters or create protocol files beyond the V3 layout. | This is protocol-era behavior and is not semantically equivalent to V3 initialization. | Do not silently port. Keep while the legacy Cockpit remains, or require an explicit product retirement decision. |
| Create the first protocol task | Project-card context menu → VibeHub Cockpit → first-task form | `vibehubStartTask` / `vibehub_start_task` | Write | No. Production V3 displays active tasks but has no task-creation entry. | Removing the old entry removes the only in-app first-task creation flow. | Add a V3-native task creation design before removal, or obtain explicit product approval to retire it. |
| Read active task, phase, context, review, handoff, diff, evidence, research, activity, Git and archive details | Project-card context menu → VibeHub Cockpit dashboard and detail drawers | `vibehubReadOverview` / `vibehub_read_overview` | Read-only | Partial. Production V3 provides overview, main-task timeline, main-task plan graph, node brief, structure and legacy archive, but its current bundle only provides full detail for the main task and does not expose all protocol-era context/review/handoff/diff/evidence fields. | Detail depth and secondary-task coverage are not equivalent. | Keep until each retained product field has a V3 read-model and user entry, or explicitly retire individual protocol-era details. |
| Validate the current phase | VibeHub Cockpit load and phase/task detail | `vibehubValidatePhase` / `vibehub_validate_phase` | Read-only | No equivalent production V3 action or status. The command remains registered, but that is not user reachability. | Schema/output validation diagnostics would disappear. | Keep, or add a V3-native validation read model and surface before removal. |
| Inspect workspace drift | VibeHub Cockpit load → Git detail | `vibehubCheckWorkspaceDrift` / `vibehub_check_workspace_drift` | Read-only | No equivalent production V3 surface. | HEAD change, stale context and drift warnings would disappear. | Keep, or replace with a V3-native project freshness/drift surface. |
| Synchronize protocol workspace | Legacy command/wrapper remains available; the Cockpit primarily renders recommended prompt actions rather than directly invoking sync | `vibehubSyncWorkspace` / `vibehub_sync_workspace` | Write | No production V3 entry. | Backend existence alone does not close the user-entry gap; the protocol itself is retired. | Require an explicit retirement decision; do not expose this write in production V3 by default. |
| Inspect and synchronize agent adapters | VibeHub Cockpit → Settings / adapters drawer | `vibehubGetAgentAdapterStatus`, `vibehubSyncAgentAdapters` / corresponding Tauri commands | Read + write | No production V3 or other inspected stable entry. | Adapter status, file conflicts and repair/sync controls would disappear. | Preserve via a stable settings surface before removing the legacy Cockpit, or explicitly retire adapter management. |
| Set project locale | VibeHub Cockpit → Settings / adapters drawer, and legacy initialization form | `vibehubSetProjectLocale` / `vibehub_set_project_locale` | Write | No production V3 project-locale entry. Application locale is not equivalent to persisted project locale. | Per-project generated-content locale control would disappear. | Move to stable project settings before removal, or explicitly retire it. |
| List, render, confirm and copy prompt templates | VibeHub Cockpit header/settings/recommended actions → prompt generator modal | `vibehubListPromptTemplates`, `vibehubRenderPrompt` / corresponding Tauri commands; clipboard copy in UI | Read-only apart from clipboard | No production V3 entry. | New-task and maintenance prompt generation, including dangerous-template confirmation, would disappear. | Preserve in a stable prompt/settings surface before removal, or explicitly retire it. |
| Preview `.vibehub` artifacts and open/reveal them | VibeHub Cockpit → preview, archive and activity details | `vibehubReadVibehubFile`, `vibehubOpenVibehubFile`, `vibehubRevealVibehubFile` / corresponding Tauri commands | Read + external open/reveal | Partial. Production V3 can open legacy archive file links, but it does not provide the old structured preview/reveal surface for all protocol artifacts. | Production V3 has no generic artifact preview and no reveal action. | Keep until retained artifacts have a V3-native viewer; legacy archive file-open coverage alone is insufficient. |
| Open or reveal non-legacy project files from the structure view | Project-card body → production V3 → Structure Architecture → selected node actions; legacy Cockpit retains its equivalent actions | `vibehubOpenProjectFile`, `vibehubRevealProjectFile` / corresponding Tauri commands | External open/reveal | Yes at the code level. Production V3 receives bounded callbacks only from the production route, exposes reveal for selected nodes, and exposes default-app open only for file nodes. Fixture playground receives neither callback. | Browser preview proves fixture isolation and preserves the structure surface, but native reveal/open success and shell failure still require Tauri-window evidence. | Keep the production V3 actions; retain the legacy entry until the remaining capability matrix is closed. |
| Read local AI usage | Project-card body → production V3 overview panel and top Token/fee entry | `vibehubReadLocalAgentUsage` / `vibehub_read_local_agent_usage` | Read-only | Yes. Production V3 preserves task usage, project usage sidebar, refresh, source/model/tool/stage/session states according to the existing M1 surface. | Native Tauri loading/refresh/stale/partial/session evidence remains missing. | Keep production V3 entry; do not remove or redesign the M1 usage surface. |
| Read legacy archive list, details, summaries and warnings | Project-card body → production V3 archived-task section and detail drawer | `loadLegacyV2Archive` → `legacy_v2_load_archive` | Read-only | Yes at the code level, including source state, loading, empty, error, warnings and details. | Native Tauri evidence for B05–B06 remains missing. | Keep production V3 entry. |
| Open legacy archive file links | Production V3 archived-task detail | `vibehubOpenVibehubFile` / `vibehub_open_vibehub_file` | External open | Yes at the code level with surfaced errors. | Native success, missing, traversal and shell-open-failure evidence remains missing for B07. | Keep production V3 entry. |

## Entry and registration findings

- `ProjectCard` keeps the legacy context-menu item and mounts `VibehubCockpitDialog`; ordinary card selection is routed through `Home` to production `V3Cockpit`.
- `SortableProjectCard` only forwards selection and does not provide a separate Cockpit capability.
- `VibehubProjectCenter` is a second legacy-content host, but no inspected production route currently renders it.
- `main.rs` still registers both V3 lifecycle/read commands and protocol-era commands. Registration must not be used as evidence that users retain an entry after UI removal.

## Closure decision

D05 removal is not currently safe. The legacy context-menu entry must remain until the unresolved capabilities above are either replaced by stable user-reachable entries or explicitly retired by product decision. Consequently, `VibehubCockpitDialog`, `VibehubProjectCenter`, and their exclusive wrappers/commands must not be deleted yet. D06–D08 should not proceed on the assumption that D05 is closed.

## Native acceptance gate observed on 2026-07-12

The current macOS host reports:

- Screen Recording preflight: granted.
- Accessibility trust for the current process: not granted.
- System Events UI automation: disabled.
- Rust, Node, `osascript`, and `screencapture`: available.

Because trusted UI automation is unavailable, this environment cannot currently produce reliable native interaction evidence for A08, B05–B07, or C05–C06. No real project or transcript should be used to work around this gate.

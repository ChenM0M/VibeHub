# M6 completion execution plan

Date: 2026-07-12
Status source: `docs/v3/m6-checklist.md`, maintained as an ordinary software-engineering checklist. Historical `.vibehub` protocol state is not an execution dependency.

## Objective

Finish every checklist item that is reachable on the current macOS host in no more than nine completion batches, then finish the Windows-only acceptance matrix in no more than two Windows batches.

A batch is a deliverable boundary, not a chat turn. Work does not stop or report success halfway through a batch. A batch ends only when all named IDs are VERIFIED, or when a newly demonstrated external blocker is recorded with exact evidence.

## Current strict state

- VERIFIED: 27
- IN_PROGRESS: 7
- NOT_STARTED: 18
- BLOCKED: 6
- Total: 58
- Strict completion: 27/58 = 46.6%

The 27 VERIFIED items are:

- A01–A06
- B01–B04, B08, B09
- C01, C03, C04, C07–C10
- D01–D04
- E01–E04

The seven IN_PROGRESS items are:

- A08
- B05–B07
- C02
- C05–C06

The eighteen NOT_STARTED items are:

- D05–D08
- E05, E06, E08
- F01–F06
- G01–G05

The six Windows-blocked items are:

- A07
- E07
- F07–F10

## Non-negotiable execution rules

1. Do not run VibeHub Agent Protocol workflow commands, do not read protocol state as an execution source, and do not update `.vibehub` state, checklist, context packs, handoff, or output files.
2. Preserve all existing uncommitted changes. Do not restore deleted `CLAUDE.md` or `AGENTS.md`.
3. M1 frontend content, layout, metrics, scenario controls, fixture selector, debug controls, usage panels, and interactions remain product authority.
4. Every implementation batch names its checklist IDs before editing code.
5. No standalone audit batch is allowed. Inspection is only permitted as the first step of a batch that implements or verifies named IDs.
6. No partial work is reported as checklist progress. If an ID remains IN_PROGRESS or NOT_STARTED, say so plainly.
7. Do not revisit an already passing subsystem unless the current batch depends on it or a directly related test fails.
8. Run targeted checks during implementation. Run the complete relevant gate only at the end of the batch.
9. Browser preview is evidence only for browser-observable behavior. Native Tauri, shell, Finder, IDE, installer, upgrade, and uninstall behavior require native evidence.
10. Use only isolated temporary/copied projects for migration, legacy, worktree, upgrade, and destructive-path tests. Never migrate the real VibeHub repository or alter real Claude transcripts.
11. Do not modify CI before local platform gates pass.
12. Do not commit or push without explicit user instruction.
13. Do not produce another replacement plan. Newly discovered defects are fixed inside the active batch. Only a genuine external blocker may change the schedule.

## Definition of VERIFIED

An ID becomes VERIFIED only when all of the following are true:

- The complete behavior described by the checklist item exists.
- Direct automated checks pass when the behavior is automatable.
- Browser evidence exists for browser-visible behavior.
- Native evidence exists for native-only behavior.
- Failure, empty, partial, stale, unsupported, or recovery states required by the item are exercised.
- The evidence names the exact command, fixture or isolated project, platform, result, and relevant artifact.
- No M1 surface was removed or redesigned.

## macOS completion batches

### Batch M1 — Close data-source capability and migration documentation

Target IDs: C02, G01
Expected strict state: 29/58

Deliverables:

- Finish the Claude App ordinary-chat capability conclusion without modifying active databases or transcripts. If exact project-scoped usage is not available through a stable, validated source, record it as unsupported rather than estimating it.
- Finalize the Cursor conclusion under the same rule: no stable validated source means unsupported.
- Ensure production UI/read model represents unsupported sources truthfully.
- Write executable V3 inspect, initialize, migrate, interrupted-recovery, conflict, and isolated-test-project documentation using final CLI names.

Exit gate:

- Focused source/fixture tests and contract checks pass.
- Documentation commands match current CLI help and behavior.
- C02 and G01 are independently reproducible without private transcript mutation.

### Batch M2 — Produce and verify the packaged macOS application foundation

Target IDs: F01, F03, E05
Expected strict state: 32/58

Deliverables:

- Build the aarch64 macOS `.app` locally.
- Apply and verify prerelease ad-hoc signing.
- Record artifact path and SHA-256.
- Run a real `mcp-stdio` session against the packaged desktop binary through absolute path and, when available, installed PATH resolution.

Exit gate:

- Tauri app build succeeds.
- `codesign --verify` and signature inspection succeed.
- Packaged binary completes the MCP request matrix rather than merely starting or printing help.

### Batch M3 — Close all shared native-window evidence in one run

Target IDs: A08, B05, B06, B07, C05, C06
Expected strict state: 38/58

External prerequisite:

- Accessibility and System Events UI automation must be granted to the actual host process used for native automation. Screen Recording is already available.

Deliverables:

- Use isolated absent, V2, interrupted, conflict, legacy-empty, legacy-corrupt, usage-fresh, usage-stale, usage-partial, and usage-error projects.
- Capture inspect → initialize/migrate/recover behavior.
- Capture legacy list, detail, warning, missing link, traversal rejection, shell-open success, and shell-open failure.
- Capture usage loading, refresh, stale, partial, error, project drawer, and session expansion while preserving all M1 UI.

Exit gate:

- Native screenshots/logs show every required state.
- No real project is migrated.
- All six IDs close together; do not perform six separate repeated app launches unless isolation requires it.

If Accessibility remains unavailable, freeze these six items and continue with M4–M9. Do not spend another batch retrying the same permission gate.

### Batch M4 — Build, mount, copy, and smoke the macOS distributable

Target IDs: F02, F04, E06
Expected strict state: 41/58 after M3, or 35/58 if M3 remains externally blocked

Deliverables:

- Create the APFS DMG, mount it, copy the app, launch the copied app, and record hashes.
- On the clean copied app, smoke GUI, headless CLI, MCP, legacy browsing, and project lifecycle using isolated data.
- Verify V3 structure open/reveal with the preferred installed IDE and verify understandable failure behavior.

Exit gate:

- DMG lifecycle succeeds without using the build-tree app as a substitute.
- Native Finder/default-app/IDE actions have success and failure evidence.

### Batch M5 — Close the old Cockpit capability boundary and remove its user entry

Target ID: D05
Expected strict state: 42/58 after M3, or 36/58 if M3 remains blocked

Policy:

- Preserve user value, not retired protocol mechanics.
- Retain or replace capabilities that remain meaningful in V3: V3 task creation, project settings/locale where still consumed, bounded project file actions, and truthful project diagnostics.
- Do not expose retired Agent Protocol sync, phase mutation, adapter-file generation, or prompt-template mechanics in production V3 merely to preserve old implementation details.
- Any retired behavior must have a code-level reason tied to the stopped protocol; no capability is removed merely because it is old.

Deliverables:

- Complete the remaining required V3 replacements in one continuous batch.
- Remove the `ProjectCard → VibehubCockpitDialog` user-reachable entry only after the retained-capability gate passes.
- Keep M1 production and playground surfaces unchanged.

Exit gate:

- Consumer matrix proves no required stable capability loses its only entry.
- Production V3 and project settings cover all retained capabilities.
- The old context-menu entry is gone.

### Batch M6 — Delete the unreachable legacy Cockpit implementation and enforce clean break

Target IDs: D06, D07, D08
Expected strict state: 45/58 after M3, or 39/58 if M3 remains blocked

Deliverables:

- Delete unreachable legacy Cockpit components, exclusive state, styles, wrappers, IPC, aliases, and adapters.
- Preserve independently consumed diagnostics and platform functions.
- Add a static production gate for canonical writes, V2 schema imports, duplicate adapters/dispatchers, and forbidden old Cockpit reachability.

Exit gate:

- Full consumer scan has no dangling registration/wrapper/component.
- Contracts, frontend build, relevant Rust packages, CLI/MCP checks, and static clean-break gate pass.

### Batch M7 — Complete packaged parallel-node and upgrade/rollback behavior

Target IDs: E08, F05, F06
Expected strict state: 48/58 after M3, or 42/58 if M3 remains blocked

Deliverables:

- Run packaged-app worktree smoke on an isolated project: create, observe, interrupted recovery, integration state, and cleanup.
- Upgrade from an isolated installed V2 fixture to V3 and verify archive fidelity, recovery, and rollback.
- Uninstall and verify the documented application/helper/runtime/worktree/temp residue list without deleting user project archives.

Exit gate:

- Native packaged evidence exists for every transition and cleanup invariant.

### Batch M8 — Finish release documentation, evidence index, and production gate

Target IDs: G02, G03, G04
Expected strict state: 51/58 after M3, or 45/58 if M3 remains blocked

Deliverables:

- Write install, upgrade, uninstall, prerelease signing, formal-signing decision boundary, rollback, and archive documentation.
- Create an evidence index mapping every checklist ID to concrete logs, screenshots, protocol output, platform, binary version, and artifact hash.
- Run the final production gate for declared data sources, M1 preservation, debug/fixture data isolation, old Cockpit removal, and old write-path removal.

Exit gate:

- Documentation is executable against the produced artifacts.
- Evidence index contains no unsupported “tested” claims.
- G04 follows the user-confirmed M1 rule: debug/scenario/settings surfaces are preserved; the gate verifies data and command isolation rather than deleting those controls.

### Batch M9 — macOS dry run and blocker reconciliation

Target: no new scope and no speculative features
Expected strict state: 51/58 maximum on macOS, or 45/58 if the six native-window IDs remain permission-blocked

Deliverables:

- Execute the complete macOS path once from clean install through lifecycle, legacy, MCP, IDE, parallel worktree, upgrade/rollback, and uninstall.
- Fix any discovered defect inside this batch.
- Reconstruct all 58 statuses from evidence and prepare the Windows artifact/input package.

Exit gate:

- All non-Windows IDs except G05 are VERIFIED, or each remaining item has one demonstrated external blocker.

## Windows completion batches

### Batch W1 — Windows platform, installer, IDE, and packaged runtime

Target IDs: A07, E07, F07, F09
Expected strict state: 55/58

Prerequisite:

- Windows 10/11 native host or trusted VM with an installed supported IDE.

Deliverables:

- Verify reparse/junction, cross-volume rename, file lock, process interruption, recovery, long paths, spaces, and missing files.
- Clean-install the Windows package and run GUI, headless CLI, MCP, IDE, and parallel-node smoke.

### Batch W2 — Windows upgrade/uninstall and final cross-platform acceptance

Target IDs: F08, F10, G05
Expected strict state: 58/58

Deliverables:

- Verify V2→V3 upgrade, clean break, file-lock recovery, rollback, uninstall, and residue behavior.
- Execute the final cross-platform chain: clean install → legacy → MCP → IDE → parallel node → upgrade/rollback → uninstall.
- Reconcile every ID against the evidence index.

## Time and report discipline

- Maximum planned completion batches: 11 total — nine macOS, two Windows.
- Maximum macOS strict result with native permissions: 51/58.
- Maximum macOS strict result without native permissions: 45/58.
- Full 58/58 requires the Windows host and two Windows batches.
- Each batch report contains only: IDs closed, files changed, commands/tests, native/browser evidence, remaining blockers, and updated strict count.
- No additional audit-only, handoff-only, context-pack, or replanning batch is permitted.

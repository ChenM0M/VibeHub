# M6 engineering checklist

Date: 2026-07-12
Authority: ordinary software-engineering acceptance checklist

This document replaces the historical protocol-managed checklist as the active M6 status source. Checklist IDs are retained only as stable requirement identifiers. They do not imply any dependency on the retired VibeHub Agent Protocol.

## Status rules

- `VERIFIED`: complete implementation plus the evidence required by the item.
- `IN_PROGRESS`: implementation exists, but a required path or evidence remains incomplete.
- `NOT_STARTED`: insufficient implementation evidence.
- `BLOCKED`: completion requires an unavailable external platform or permission.
- Only `VERIFIED` counts toward strict completion.

Current totals:

- VERIFIED: 43
- IN_PROGRESS: 0
- NOT_STARTED: 1
- BLOCKED: 14
- Total: 58
- Strict completion: 43/58 = 74.1%

## A. V3 clean break and recovery

| ID | Status | Acceptance requirement | Current evidence or remaining gate |
| --- | --- | --- | --- |
| A01 | VERIFIED | V3 root layout defines `project.yaml` schema version 3, events, projections, indexes, runtime, and read-only `legacy-v2`. | V3 bootstrap implementation and tests. |
| A02 | VERIFIED | Layout inspection distinguishes absent, V2, V3, interrupted, and conflict; unknown/conflict states fail closed. | V3 bootstrap tests. |
| A03 | VERIFIED | Absent project initializes V3 idempotently. | Bootstrap tests and isolated CLI smoke. |
| A04 | VERIFIED | V2 migration archives source data unchanged under `legacy-v2` without overwriting an existing archive. | Bootstrap tests and fidelity smoke. |
| A05 | VERIFIED | Interrupted migration can safely recover; ambiguous states refuse automatic mutation. | Recovery tests. |
| A06 | VERIFIED | Migration tests use only temporary or copied projects. | Existing test fixtures and command evidence. |
| A07 | BLOCKED | Windows junction/reparse, cross-volume rename, file locking, process interruption, and recovery pass. | Requires Windows 10/11 native host or trusted VM. |
| A08 | BLOCKED | Desktop UI provides understandable inspect → initialize/migrate/recover flows, confirmation, errors, and recovery guidance. | Code and automated tests exist; native interaction evidence is externally blocked because macOS Accessibility/System Events reports UI automation disabled. |

## B. Legacy V2 read-only product entry

| ID | Status | Acceptance requirement | Current evidence or remaining gate |
| --- | --- | --- | --- |
| B01 | VERIFIED | Rust legacy read model is independent from V3 domain and reads archives only. | Legacy reader tests. |
| B02 | VERIFIED | Independent Tauri command and TypeScript loader remain outside V3 generated contracts. | Loader, command, contract checks. |
| B03 | VERIFIED | DTO contains only source state, task identity, title, status, time, phase, summary, file links, and warnings. | Legacy TypeScript contracts. |
| B04 | VERIFIED | Legacy loading/error/request generation is independent from the main bundle and stale responses are invalidated. | Store implementation and checks. |
| B05 | BLOCKED | Production V3 displays real archive list plus loading, empty, error, and source state. | UI is connected; required native interaction evidence is externally blocked by unavailable macOS Accessibility/System Events automation. |
| B06 | BLOCKED | Archive details truthfully display phase, completion time, summary, and warnings without fabricated fields. | UI and corpus code exist; required native detail evidence is externally blocked by unavailable macOS Accessibility/System Events automation. |
| B07 | BLOCKED | File links handle success, missing targets, traversal rejection, and shell-open failure as local errors. | Bounded backend and UI exist; required native success/failure interaction evidence is externally blocked by unavailable macOS Accessibility/System Events automation. |
| B08 | VERIFIED | Removed fabricated archive token totals, token history, and archive AI usage. | Frontend source and build checks. |
| B09 | VERIFIED | Parameterized corpus covers missing fields, old fields, corrupt latest run fallback, no summary/link, invalid entries, traversal, large history, and valid siblings beside corruption. | Nine legacy reader tests and full core package pass. |

## C. Product data, usage, fixtures, and debug boundary

| ID | Status | Acceptance requirement | Current evidence or remaining gate |
| --- | --- | --- | --- |
| C01 | VERIFIED | Every user-visible V3 field/control is classified as real, derived, fixture, mock, placeholder, or unsupported. | Engineering source matrix exists; M1 remains authoritative. |
| C02 | VERIFIED | M1 token, cost, usage, model/tool activity, phase distribution, and sessions have final authoritative-source capability conclusions. | `usage-source-capabilities.md`; Claude App/Cursor are explicit unsupported sources, excluded from aggregation; usage tests and production contract gates pass. |
| C03 | VERIFIED | Provider-independent usage read model defines units, windows, freshness, stale, completeness, source state, sessions, and deduplication. | Read-model design and implementation evidence. |
| C04 | VERIFIED | Core/Tauri usage service reads real supported local sources and distinguishes unavailable, permission, partial provider, parse, and read failures. | Usage reader fixture tests and real read-only smoke. |
| C05 | BLOCKED | Production M1 usage surface uses real data and covers loading, empty, partial, stale, error, and refresh without redesign. | Code/tests/build exist; required native state/refresh evidence is externally blocked by unavailable macOS Accessibility/System Events automation. |
| C06 | BLOCKED | All M1 usage/token/cost/model/tool/phase/session surfaces remain and progressively replace mock sources truthfully. | Code/browser evidence exists; required native drawer/session-expansion evidence is externally blocked by unavailable macOS Accessibility/System Events automation. |
| C07 | VERIFIED | M1 scenario, fixture selector, debug toggles, settings, and interactions remain while production/fixture data and command boundaries are technically isolated. | Explicit source mode, route isolation, and contract gates. |
| C08 | VERIFIED | Fixture playground is reachable only through explicit test/debug entry and cannot invoke production lifecycle, legacy, or project-file operations. | Debug route and contract gates. |
| C09 | VERIFIED | Automated gates prevent removal of M1 surfaces and direct Tauri dependencies inside V3 components. | V3 contract checks. |
| C10 | VERIFIED | All user-visible V3 screens were audited for fabricated metrics and unsupported states are shown explicitly. | PlanGraph agent fabrication removed; capability wording corrected; browser/build checks passed. |

## D. Old Cockpit and duplicate path removal

| ID | Status | Acceptance requirement | Current evidence or remaining gate |
| --- | --- | --- | --- |
| D01 | VERIFIED | Command definition/registration, wrapper, UI consumer, and CLI/MCP consumer matrix exists. | Consumer scan evidence. |
| D02 | VERIFIED | First group of unconsumed duplicate/canonical-write Tauri commands is removed. | Rust source diff and checks. |
| D03 | VERIFIED | Corresponding TypeScript wrappers/imports are removed with no residual consumers. | Wrapper diff and residual scan. |
| D04 | VERIFIED | CLI dispatcher is shared; standalone CLI and desktop headless mode do not duplicate action implementations. | Shared dispatcher and binary smoke. |
| D05 | VERIFIED | Remove the project-card legacy Cockpit entry after every retained user capability has a stable replacement or an explicit retirement rule. | V3-native bounded task creation is available only in production V3; retained locale, diagnostics, and project-file actions remain; retired protocol mechanics were not ported; ProjectCard entry removal, contracts, builds, Rust tests, scans, and browser evidence are recorded in `evidence/m5-d05/task-create-and-entry-removal.md`. |
| D06 | VERIFIED | Delete unreachable old Cockpit components and code used exclusively by them. | Deleted `VibehubCockpitDialog`, `VibehubProjectCenter`, `ProjectDetailBoard`, `ProjectStructureExplorer` and all old-Cockpit-exclusive shared types. Evidence in `evidence/m6-d06-d08/clean-break-gate.md`. |
| D07 | VERIFIED | Delete V3-replaced aliases, commands, and adapter files; document independently retained paths. | Removed 22 old-Cockpit-exclusive TS wrappers, 26 Tauri command definitions and registrations, emit helpers, and unused imports. Independently retained CLI-consumed backend paths documented in `evidence/m6-d06-d08/clean-break-gate.md`. |
| D08 | VERIFIED | Static production gate proves no retired canonical write, V2 schema import, duplicate adapter/dispatcher, or old Cockpit reachability. | 81 new static-gate assertions added to `check.mjs` (539 total). Gate proves no retired command registration, no retired TS wrapper, no retired type, no old Cockpit reachability, shared dispatcher, and legacy-v2 independence. |

## E. Packaged CLI, MCP, IDE, and parallel nodes

| ID | Status | Acceptance requirement | Current evidence or remaining gate |
| --- | --- | --- | --- |
| E01 | VERIFIED | Standalone CLI help/version/V3 doctor work and unknown commands fail safely. | Isolated binary smoke. |
| E02 | VERIFIED | Desktop binary launches GUI without arguments and runs headless actions without opening a window. | Dispatcher behavior and binary smoke. |
| E03 | VERIFIED | MCP server unit tests cover versioned resources and idempotent writes. | CLI package tests. |
| E04 | VERIFIED | MCP protocol check covers seven resources, seven typed tools, and request matrix. | Final M9 packaged MCP check passes 7 resources, 7 tools, and 21 requests. |
| E05 | VERIFIED | Installed/packaged desktop binary completes a real `mcp-stdio` session through absolute path and PATH where available. | Signed packaged binary passed the full 7-resource/3-tool request matrix by absolute path and isolated PATH resolution; the host's stale installed PATH copy is recorded separately in `evidence/m2-f01-f03-e05/native-build-sign-mcp.md`. |
| E06 | BLOCKED | macOS V3 UI opens/reveals through the preferred IDE and surfaces understandable failures. | Visual Studio Code is available and backend file bounds exist; native UI success/failure evidence is externally blocked by unavailable Accessibility/System Events automation. |
| E07 | BLOCKED | Windows IDE open/reveal handles spaces, long paths, and missing files. | Requires Windows host and supported IDE. |
| E08 | VERIFIED | Packaged app completes parallel-node/worktree create, observe, recover, integration, and cleanup smoke. | Two worktree lifecycles (happy path + conflict recovery) completed on packaged binary v2.0.0-pre.22. CLI worktree commands wired into dispatcher. Evidence in `evidence/m7-e08-f05-f06/worktree-upgrade-uninstall.md`. |

## F. macOS and Windows distribution

| ID | Status | Acceptance requirement | Current evidence or remaining gate |
| --- | --- | --- | --- |
| F01 | VERIFIED | Local aarch64 macOS `.app` build succeeds. | Current-source Tauri build produced the arm64 `VibeHub.app`; version, executable, and hashes are recorded in `evidence/m2-f01-f03-e05/native-build-sign-mcp.md`. |
| F02 | VERIFIED | APFS DMG creation, mount, copy, and first launch succeed with artifact hash. | APFS UDZO DMG was created, mounted read-only, copied to isolated storage, strict-signature verified, and the copied GUI/runtime launched; artifact hash is recorded in `evidence/m4-f02-f04-e06/dmg-native-smoke.md`. |
| F03 | VERIFIED | Prerelease ad-hoc signature verification succeeds without enabling notarization by default. | Bundle and main executable pass strict codesign verification with explicit ad-hoc identity, no team identity, timestamp, Developer ID claim, or notarization. |
| F04 | BLOCKED | Clean macOS install passes GUI, headless CLI, MCP, IDE open, and legacy browsing against isolated projects. | Isolated copied app launch, CLI, and full MCP pass; required GUI/legacy/IDE interaction evidence is externally blocked by unavailable Accessibility/System Events automation. |
| F05 | VERIFIED | Isolated V2→V3 macOS upgrade preserves archive, supports recovery, and supports rollback. | Isolated V2 fixture created, migrated to V3 with archive fidelity (5 files, identical SHA-256), interrupted recovery tested, rollback verified. Evidence in `evidence/m7-e08-f05-f06/worktree-upgrade-uninstall.md`. |
| F06 | VERIFIED | macOS uninstall removes documented app/helper/runtime/worktree/temp residue without deleting project archives. | Isolated app copy installed to `/tmp`, uninstalled, verified: app bundle removed, project archives preserved, no LaunchAgents, no worktree residue. Evidence in `evidence/m7-e08-f05-f06/worktree-upgrade-uninstall.md`. |
| F07 | BLOCKED | Windows installer clean-install and first launch pass on Windows 10/11. | Requires Windows host/VM. |
| F08 | BLOCKED | Windows V2→V3 upgrade, file-lock handling, recovery, and rollback pass. | Requires Windows host/VM and old-version fixture. |
| F09 | BLOCKED | Installed Windows app passes headless CLI, MCP, IDE, and parallel-node flows. | Requires Windows host/VM. |
| F10 | BLOCKED | Windows uninstall/residue cleanup passes without deleting user projects or legacy archives. | Requires Windows host/VM. |

## G. Documentation, evidence, and final gates

| ID | Status | Acceptance requirement | Current evidence or remaining gate |
| --- | --- | --- | --- |
| G01 | VERIFIED | Executable migration guide covers inspect, initialize/migrate, interrupted recovery, conflict handling, and isolated first-test requirements using final CLI names. | `migration-guide.md` and `evidence/m1-c02-g01/cli-smoke.md`; current CLI help plus isolated absent/init/migrate/recover/conflict matrix pass. |
| G02 | VERIFIED | Executable install/upgrade/uninstall, prerelease/formal signing, rollback, and archive documentation references platform evidence. | `release-guide.md` provides executable macOS DMG/direct-copy install, isolated V2→V3 upgrade, rollback from `legacy-v2`, app-only uninstall, archive handling, verified prerelease ad-hoc signing, and an explicitly unperformed formal Developer ID/notarization boundary. Platform provenance is linked to M2, M4, and M7 evidence. |
| G03 | VERIFIED | Evidence index links every checklist ID to concrete logs, screenshots/protocol output, platform, binary version, and artifact hash. | `evidence-index.md` maps all 58 IDs to concrete evidence or exact blocker, records platform/artifact version/hash where an artifact exists, and contains no unsupported tested claims. M8 coverage scan reports 43 VERIFIED, 14 BLOCKED, and 1 NOT_STARTED rows. |
| G04 | VERIFIED | Final production gate proves declared data sources, production/fixture command isolation, M1 preservation, old Cockpit removal, and retired write-path removal. | `production-data-source-matrix.md` declares real/derived/fixture/mock/placeholder/unsupported classifications. Current `v3:contracts:check` passes 630 assertions, 12 scenarios, 6 views, and 5 write contracts, preserving M1 debug/scenario/settings surfaces while proving data/command isolation, explicit unsupported Claude App/Cursor sources, old Cockpit removal, and retired production-write removal. Evidence in `evidence/m8-g02-g04/final-production-gate.md` and `evidence/m9-macos-dry-run/macos-dry-run.md`. |
| G05 | NOT_STARTED | Final dual-platform chain passes: clean install → legacy → MCP → IDE → parallel node → upgrade/rollback → uninstall. | Requires all A–F platform work. |

## Active execution batch

Batch M3 demonstrated that A08, B05, B06, B07, C05, and C06 are externally blocked by unavailable macOS Accessibility/System Events UI automation. Batch M4 closed F02; F04 and E06 passed their headless/runtime prerequisites but are blocked on the same native interaction permission. Batch M5 closed D05 with bounded V3-native task creation and removal of the project-card legacy Cockpit entry. Batch M6 closed D06, D07, and D08 by deleting unreachable old Cockpit components, exclusive wrappers, Tauri commands, and shared types, and by adding a static production gate. Batch M7 closed E08, F05, and F06 with packaged-app worktree smoke, isolated V2→V3 upgrade with archive fidelity and rollback, and uninstall residue cleanup. Batch M8 closed G02, G03, and G04 with the executable release guide, complete evidence index, and declared source matrix. Batch M9 revalidated the complete automatable macOS path with the final dirty-worktree artifact, 630 contract assertions, 7 MCP tools, rollback, uninstall, and blocker reconciliation. Strict completion remains 43/58; Windows W1/W2 plus the existing macOS Accessibility prerequisite are the remaining external paths.

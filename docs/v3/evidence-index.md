# M6 evidence index

Date: 2026-07-13
Checklist authority: [M6 engineering checklist](m6-checklist.md)
Index scope: all 58 stable acceptance IDs; a row is not a replacement status
source.

## Reading this index

- **Evidence** is the concrete log, protocol transcript, test source, static
gate, or recorded native result supporting the row. Links without a line suffix
identify the complete reproducible record.
- **Platform / artifact** identifies the host and exact binary/hash only where
an artifact was built or run. `N/A — source-level/host-independent` means that
an app artifact was not the relevant evidence; it is not a missing test claim.
- **Result / blocker** preserves the exact evidence level. `BLOCKED` and
`NOT_STARTED` rows are deliberately not described as passing or tested.
- Current M8 command outputs are recorded in
  [M8 final production gate evidence](evidence/m8-g02-g04/final-production-gate.md).
  The current final static gate is `npm run v3:contracts:check` with 630
  assertions, 12 scenarios, 6 views, and 5 write contracts.
- [M9 macOS packaged dry run](evidence/m9-macos-dry-run/macos-dry-run.md)
  revalidated the complete automatable clean-copy path on a freshly signed APFS
  DMG artifact, reconciled all statuses, and prepares Windows W1/W2 inputs.

## Artifact ledger

| Artifact | Platform / version | SHA-256 | Evidence |
| --- | --- | --- | --- |
| Signed packaged main executable | macOS arm64, `2.0.0-pre.22` | `c1fc7dc0898dbcadee49d3faf67b625df922aa06c3255a7293e294ed7164d2f4` | [M2 native build/sign/MCP](evidence/m2-f01-f03-e05/native-build-sign-mcp.md) |
| Signed build-tree bundle content | macOS arm64, `2.0.0-pre.22` | `94c1498fa9ec33d3597935b91dbfc12920126edc31a64c1ee5cce7392bf6c425` | [M2 native build/sign/MCP](evidence/m2-f01-f03-e05/native-build-sign-mcp.md) |
| APFS UDZO DMG | macOS arm64, `2.0.0-pre.22` | `e6b1105a5a3eed1eb058f1b5b6b1480f82cd007a994182b990d9307b6a95b230` | [M4 DMG native smoke](evidence/m4-f02-f04-e06/dmg-native-smoke.md) |
| Copied-bundle regular-file manifest | macOS arm64 copied install | `d5e3dd0f94d8232c618af8dcb45b6007ce68313fb6c28286f419a8082f89d90a` | [M4 DMG native smoke](evidence/m4-f02-f04-e06/dmg-native-smoke.md) |
| Packaged worktree/upgrade/uninstall binary | macOS arm64, `2.0.0-pre.22` | Same signed executable as ledger row 1 | [M7 worktree/upgrade/uninstall](evidence/m7-e08-f05-f06/worktree-upgrade-uninstall.md) |
| M9 final packaged main executable | macOS arm64, `2.0.0-pre.22`; post-protocol-fix dirty-worktree build at HEAD `ed3196588d8cdc153025aff9c593dbee3164e918` | `24a7601f1f0f7fc1e23a3d239ab75713ea24aa99be1973264db59189cb048f52` | [M9 macOS dry run](evidence/m9-macos-dry-run/macos-dry-run.md) |
| M9 final APFS UDZO DMG | macOS arm64, `2.0.0-pre.22`; post-protocol-fix dirty-worktree build | `1fe83ed96673f1c2eeb09459c5de3b121bd4a3fbf3d168297057aa1361ab8a35` | [M9 macOS dry run](evidence/m9-macos-dry-run/macos-dry-run.md) |

## A. V3 clean break and recovery

| ID | Status | Evidence | Platform / artifact | Result / blocker |
| --- | --- | --- | --- | --- |
| A01 | VERIFIED | [`crates/vibehub-core/src/v3/bootstrap.rs`](../../crates/vibehub-core/src/v3/bootstrap.rs) layout implementation and its tests; [G01 CLI smoke](evidence/m1-c02-g01/cli-smoke.md) | N/A — source-level/host-independent | V3 root marker and events/projections/indexes/runtime layout are created; legacy archive is separate. |
| A02 | VERIFIED | [G01 CLI smoke](evidence/m1-c02-g01/cli-smoke.md) absent/V2/V3/interrupted/conflict outputs; bootstrap tests | macOS isolated `/tmp`; debug CLI | Layout inspection distinguishes required states and conflict fails closed. |
| A03 | VERIFIED | [G01 CLI smoke](evidence/m1-c02-g01/cli-smoke.md) `absent-init` and repeat results; bootstrap idempotency test | macOS isolated `/tmp`; debug CLI | Initial creation returns `initialized`; repeat returns `already_initialized`. |
| A04 | VERIFIED | [G01 CLI smoke](evidence/m1-c02-g01/cli-smoke.md); [M7 F05 archive-fidelity table](evidence/m7-e08-f05-f06/worktree-upgrade-uninstall.md) | macOS arm64, packaged `2.0.0-pre.22` for F05 | V2 tree archives to `legacy-v2` and five recorded files retain SHA-256 values. |
| A05 | VERIFIED | [G01 CLI smoke](evidence/m1-c02-g01/cli-smoke.md) backward/forward recovery and conflict refusal; bootstrap recovery test | macOS isolated `/tmp`; debug CLI | Recovery selects safe direction; ambiguous layout refuses automatic mutation. |
| A06 | VERIFIED | [G01 CLI smoke](evidence/m1-c02-g01/cli-smoke.md); [M7 F05](evidence/m7-e08-f05-f06/worktree-upgrade-uninstall.md) | macOS isolated `/tmp` | All recorded migration tests use temporary/copied projects, not the real repository. |
| A07 | BLOCKED | [Checklist requirement](m6-checklist.md); Windows blocker below | Windows 10/11 native host/VM required | No Windows host/VM available for junction/reparse, cross-volume, lock, interruption, and recovery evidence. |
| A08 | VERIFIED | [M9 native Accessibility closure](evidence/m9-macos-dry-run/native-accessibility-closure.md) | macOS arm64; isolated `VibeHubM9Isolated.app` SHA-256 `736ced67…f7f8a` | Native absent init, V2 confirmation/migration, interrupted recovery, and resulting states pass through PID-bound AX interaction. |

## B. Legacy V2 read-only product entry

| ID | Status | Evidence | Platform / artifact | Result / blocker |
| --- | --- | --- | --- | --- |
| B01 | VERIFIED | [`crates/vibehub-core/src/legacy_v2.rs`](../../crates/vibehub-core/src/legacy_v2.rs) and core tests; final static gate | N/A — source-level/host-independent | Independent archive-only Rust reader remains outside the V3 domain. |
| B02 | VERIFIED | [`src/services/legacyV2.ts`](../../src/services/legacyV2.ts); [`src-tauri/src/commands.rs`](../../src-tauri/src/commands.rs); final contract gate | N/A — source-level/host-independent | Tauri command and TypeScript loader remain independent of generated V3 contracts. |
| B03 | VERIFIED | [`src/legacy-v2/contracts.ts`](../../src/legacy-v2/contracts.ts); [`crates/vibehub-core/src/legacy_v2.rs`](../../crates/vibehub-core/src/legacy_v2.rs) | N/A — source-level/host-independent | Archive DTO scope is limited to source fields, identity, summaries, links, and warnings. |
| B04 | VERIFIED | [`src/v3/stores/v3Store.ts`](../../src/v3/stores/v3Store.ts); final contract gate stale-request assertions | N/A — source-level/host-independent | Archive request channel is independent and stale responses are invalidated. |
| B05 | VERIFIED | [M9 native Accessibility closure](evidence/m9-macos-dry-run/native-accessibility-closure.md); production implementation in [`V3Cockpit.tsx`](../../src/v3/app/V3Cockpit.tsx) | macOS arm64 isolated app | Native degraded/source warning, empty, refresh, and one-card list states pass. |
| B06 | VERIFIED | [M9 native Accessibility closure](evidence/m9-macos-dry-run/native-accessibility-closure.md) | macOS arm64 isolated app | Native detail truthfully displays completion, time, phase, summary, warnings, and bounded files. |
| B07 | VERIFIED | [M9 native Accessibility closure](evidence/m9-macos-dry-run/native-accessibility-closure.md); bounded actions in [`commands.rs`](../../src-tauri/src/commands.rs) | macOS arm64 isolated app | Existing-file success, visible missing-target and shell failures pass; traversal remains resolver-test verified. |
| B08 | VERIFIED | [M6 clean-break gate](evidence/m6-d06-d08/clean-break-gate.md); final static gate | N/A — source-level/host-independent | Fabricated archive token totals/history/archive AI usage were removed. |
| B09 | VERIFIED | `legacy_v2` parameterized reader tests in [`crates/vibehub-core/src/legacy_v2.rs`](../../crates/vibehub-core/src/legacy_v2.rs); M8 core test output | N/A — source-level/host-independent | Corpus covers missing/old/corrupt/link/traversal/history/sibling cases. |

## C. Product data, usage, fixtures, and debug boundary

| ID | Status | Evidence | Platform / artifact | Result / blocker |
| --- | --- | --- | --- | --- |
| C01 | VERIFIED | [Production data-source matrix](production-data-source-matrix.md); [usage capabilities](usage-source-capabilities.md) | N/A — source-level/host-independent | Every production/fixture/debug field/control classification is declared. |
| C02 | VERIFIED | [Usage source capabilities](usage-source-capabilities.md); usage reader tests in [`local_agent_usage.rs`](../../src-tauri/src/local_agent_usage.rs); final gate | N/A — source-level/host-independent | Claude App/Cursor are explicit unsupported sources and never aggregate. |
| C03 | VERIFIED | [Usage source capabilities](usage-source-capabilities.md); [`local_agent_usage.rs`](../../src-tauri/src/local_agent_usage.rs) | N/A — source-level/host-independent | Units, cache split, freshness, completeness, source state, safe sessions, and deduplication are defined. |
| C04 | VERIFIED | [`src-tauri/src/local_agent_usage.rs`](../../src-tauri/src/local_agent_usage.rs) tests; M8 Tauri test output | N/A — source-level/host-independent | Read-only supported readers distinguish unavailable, permission, partial, parse, and read failures. |
| C05 | VERIFIED | [M9 native Accessibility closure](evidence/m9-macos-dry-run/native-accessibility-closure.md); usage UI in [`AIUsagePanel.tsx`](../../src/v3/components/task/AIUsagePanel.tsx) | macOS arm64 isolated app | Native refresh plus partial/empty/unavailable/unsupported state matrix passes. |
| C06 | VERIFIED | [M9 native Accessibility closure](evidence/m9-macos-dry-run/native-accessibility-closure.md); M1 preservation gate | macOS arm64 isolated app | Native Token drawer and zero-session detail preserve truthful provider and cost boundaries. |
| C07 | VERIFIED | [`src/pages/Home.tsx`](../../src/pages/Home.tsx), [`src/v3/debug.ts`](../../src/v3/debug.ts), final static gate | N/A — source-level/host-independent | M1 scenario, fixture, debug, settings, and interaction controls remain while modes isolate data/commands. |
| C08 | VERIFIED | [`Home.tsx`](../../src/pages/Home.tsx), [`v3Store.ts`](../../src/v3/stores/v3Store.ts), final static gate | N/A — source-level/host-independent | Explicit debug playground gets no lifecycle, legacy, project-file, or task mutation callbacks. |
| C09 | VERIFIED | `npm run v3:contracts:check` current 751-assertion gate; [M8 baseline gate evidence](evidence/m8-g02-g04/final-production-gate.md); [M9 rerun](evidence/m9-macos-dry-run/macos-dry-run.md) | macOS host; source/build gate | Static checks guard M1 surface presence and V3 direct-Tauri/V2-state boundary. |
| C10 | VERIFIED | [Usage source capabilities](usage-source-capabilities.md); [`PlanGraph.tsx`](../../src/v3/components/task/PlanGraph.tsx); final gate | N/A — source-level/host-independent | Unsupported/fabricated metrics are corrected and shown explicitly. |

## D. Old Cockpit and duplicate-path removal

| ID | Status | Evidence | Platform / artifact | Result / blocker |
| --- | --- | --- | --- | --- |
| D01 | VERIFIED | [M6 retained-consumer matrix](evidence/m6-d06-d08/clean-break-gate.md); [`dispatcher.rs`](../../crates/vibehub-cli/src/dispatcher.rs) | N/A — source-level/host-independent | Command/registration/wrapper/UI/CLI/MCP consumption matrix exists. |
| D02 | VERIFIED | [M6 clean-break gate](evidence/m6-d06-d08/clean-break-gate.md); final static gate | N/A — source-level/host-independent | First unconsumed duplicate/canonical-write command group is removed. |
| D03 | VERIFIED | [M6 clean-break gate](evidence/m6-d06-d08/clean-break-gate.md); final static gate | N/A — source-level/host-independent | Corresponding TypeScript wrappers/imports have no residual consumers. |
| D04 | VERIFIED | [`src-tauri/src/main.rs`](../../src-tauri/src/main.rs); [`dispatcher.rs`](../../crates/vibehub-cli/src/dispatcher.rs); final static gate | N/A — source-level/host-independent | Desktop headless mode uses shared CLI dispatcher. |
| D05 | VERIFIED | [M5 task-create and entry removal](evidence/m5-d05/task-create-and-entry-removal.md) | Browser dev evidence + source/build; no native IPC claim | Retained V3 task creation, settings, diagnostics, and file actions exist; legacy ProjectCard entry is absent. |
| D06 | VERIFIED | [M6 clean-break gate](evidence/m6-d06-d08/clean-break-gate.md) | N/A — source-level/host-independent | Four unreachable legacy Cockpit components and exclusive types are deleted. |
| D07 | VERIFIED | [M6 clean-break gate](evidence/m6-d06-d08/clean-break-gate.md) | N/A — source-level/host-independent | 22 wrappers, 26 commands/registrations, emit helpers, and unused imports removed; independent CLI paths recorded. |
| D08 | VERIFIED | [`scripts/v3-contracts/check.mjs`](../../scripts/v3-contracts/check.mjs); [M6 gate](evidence/m6-d06-d08/clean-break-gate.md); current 751-assertion output | macOS host; source/build gate | No retired registration/wrapper/type/old Cockpit reachability; shared dispatcher and legacy independence remain verified. |

## E. Packaged CLI, MCP, IDE, and parallel nodes

| ID | Status | Evidence | Platform / artifact | Result / blocker |
| --- | --- | --- | --- | --- |
| E01 | VERIFIED | [`dispatcher.rs`](../../crates/vibehub-cli/src/dispatcher.rs); [M8 final-gate command log](evidence/m8-g02-g04/final-production-gate.md) | CLI build on macOS; source binary | Help/version/V3 doctor and safe unknown-command behavior are covered by CLI evidence. |
| E02 | VERIFIED | [`src-tauri/src/main.rs`](../../src-tauri/src/main.rs); [M4 copied runtime smoke](evidence/m4-f02-f04-e06/dmg-native-smoke.md) | macOS arm64 packaged `2.0.0-pre.22`; ledger hash | GUI no-argument startup and shared headless dispatch behavior are recorded. |
| E03 | VERIFIED | [`crates/vibehub-cli/src/mcp.rs`](../../crates/vibehub-cli/src/mcp.rs); [M8 CLI test log](evidence/m8-g02-g04/final-production-gate.md) | macOS CLI tests | MCP unit tests cover versioned resources and idempotent writes. |
| E04 | VERIFIED | [`scripts/v3-mcp/contract-test.mjs`](../../scripts/v3-mcp/contract-test.mjs); [M9 final MCP matrix](evidence/m9-macos-dry-run/macos-dry-run.md) | macOS arm64 packaged `2.0.0-pre.22`; M9 final ledger hash | Protocol matrix: 7 resources, 7 tools, 21 requests. |
| E05 | VERIFIED | [M2 packaged MCP absolute/PATH matrix](evidence/m2-f01-f03-e05/native-build-sign-mcp.md) | macOS arm64 `2.0.0-pre.22`; ledger hash | Exact signed binary passes absolute path and isolated PATH MCP matrices. |
| E06 | VERIFIED | [M9 native Accessibility closure](evidence/m9-macos-dry-run/native-accessibility-closure.md) | macOS arm64 isolated app | `code .` succeeds; nonexistent IDE remains in-dialog with visible exit-status error. |
| E07 | BLOCKED | [Checklist requirement](m6-checklist.md) | Windows native host + supported IDE required | No Windows host/IDE evidence for spaces/long paths/missing files. |
| E08 | VERIFIED | [M7 packaged worktree smoke](evidence/m7-e08-f05-f06/worktree-upgrade-uninstall.md); [M9 copied-artifact worktree rerun](evidence/m9-macos-dry-run/macos-dry-run.md) | macOS arm64 packaged `2.0.0-pre.22`; M9 rebuilt executable hash in ledger | Happy (10 events) and conflict-recovery (12 events) worktrees reach `cleaned` on the fresh copied M9 artifact. |

## F. macOS and Windows distribution

| ID | Status | Evidence | Platform / artifact | Result / blocker |
| --- | --- | --- | --- | --- |
| F01 | VERIFIED | [M2 native build/sign/MCP](evidence/m2-f01-f03-e05/native-build-sign-mcp.md); [M9 rebuilt artifact](evidence/m9-macos-dry-run/macos-dry-run.md) | macOS arm64 `2.0.0-pre.22`; M9 executable ledger hash | Current-source Tauri build produced and M9 revalidated the arm64 app bundle. |
| F02 | VERIFIED | [M4 DMG native smoke](evidence/m4-f02-f04-e06/dmg-native-smoke.md); [M9 APFS clean copy](evidence/m9-macos-dry-run/macos-dry-run.md) | macOS arm64 `2.0.0-pre.22`; M4 and M9 DMG ledger hashes | APFS UDZO DMG create, read-only mount, copy, and strict signature verification pass. |
| F03 | VERIFIED | [M2 signing evidence](evidence/m2-f01-f03-e05/native-build-sign-mcp.md); [M9 signing rerun](evidence/m9-macos-dry-run/macos-dry-run.md) | macOS arm64 `2.0.0-pre.22`; M9 signed ledger hashes | Explicit prerelease ad-hoc signing passes strict verification; notarization is not claimed. |
| F04 | VERIFIED | [M4 copied CLI/MCP evidence](evidence/m4-f02-f04-e06/dmg-native-smoke.md); [M9 native Accessibility closure](evidence/m9-macos-dry-run/native-accessibility-closure.md) | macOS arm64 copied/isolated app | Copied GUI, legacy, usage, IDE, CLI, and MCP portions now have direct evidence. |
| F05 | VERIFIED | [M7 isolated V2→V3 upgrade](evidence/m7-e08-f05-f06/worktree-upgrade-uninstall.md); [M9 copied-artifact upgrade](evidence/m9-macos-dry-run/macos-dry-run.md) | macOS arm64 packaged `2.0.0-pre.22`; M9 executable ledger hash | Archive fidelity, idempotency, interrupted recovery, and manual rollback pass. |
| F06 | VERIFIED | [M7 isolated uninstall](evidence/m7-e08-f05-f06/worktree-upgrade-uninstall.md); [M9 copied-app uninstall](evidence/m9-macos-dry-run/macos-dry-run.md) | macOS arm64 copied app; M9 executable ledger hash | The `/tmp` app bundle is removed while archive persists and no isolated LaunchAgent/worktree residue remains. |
| F07 | BLOCKED | [Checklist requirement](m6-checklist.md) | Windows 10/11 native host/VM required | No Windows installer clean-install/first-launch evidence. |
| F08 | BLOCKED | [Checklist requirement](m6-checklist.md) | Windows host/VM + old-version fixture required | No Windows V2→V3 lock/recovery/rollback evidence. |
| F09 | BLOCKED | [Checklist requirement](m6-checklist.md) | Windows host/VM required | No installed Windows CLI/MCP/IDE/parallel-node evidence. |
| F10 | BLOCKED | [Checklist requirement](m6-checklist.md) | Windows host/VM required | No Windows uninstall/residue evidence. |

## G. Documentation, evidence, and final gates

| ID | Status | Evidence | Platform / artifact | Result / blocker |
| --- | --- | --- | --- | --- |
| G01 | VERIFIED | [Migration guide](migration-guide.md); [G01 isolated CLI smoke](evidence/m1-c02-g01/cli-smoke.md) | macOS isolated `/tmp`; debug CLI | Final V3 inspect/init/migrate/recovery/conflict guidance matches executable outcomes. |
| G02 | VERIFIED | [Release operations guide](release-guide.md); [M2](evidence/m2-f01-f03-e05/native-build-sign-mcp.md), [M4](evidence/m4-f02-f04-e06/dmg-native-smoke.md), [M7](evidence/m7-e08-f05-f06/worktree-upgrade-uninstall.md), and [M9](evidence/m9-macos-dry-run/macos-dry-run.md) | macOS arm64 `2.0.0-pre.22`; artifact ledger | Executable install/upgrade/uninstall/ad-hoc-signing/rollback/archive guide states formal-signing/notarization as unperformed boundary. |
| G03 | VERIFIED | This [evidence index](evidence-index.md); [M8 final gate log](evidence/m8-g02-g04/final-production-gate.md); [M9 reconciliation](evidence/m9-macos-dry-run/checklist-reconciliation.md) | Mixed host/source/native evidence; ledger above | All 58 IDs map to concrete evidence or an exact external blocker; M9 retains 43 VERIFIED, 14 BLOCKED, and 1 NOT_STARTED without unsupported tested claims. |
| G04 | VERIFIED | [Production data-source matrix](production-data-source-matrix.md); [`check.mjs`](../../scripts/v3-contracts/check.mjs); [M8 baseline gate log](evidence/m8-g02-g04/final-production-gate.md); [M9 native closure](evidence/m9-macos-dry-run/native-accessibility-closure.md) | macOS host; final static gate 751 assertions | Declared sources, production/fixture isolation, M1 surface preservation, old Cockpit removal, and retired write-path removal are proven. |
| G05 | NOT_STARTED | [Checklist requirement](m6-checklist.md); Windows batches W1/W2 in [fixed execution plan](m6-execution-plan.md); [M9 Windows input package](evidence/m9-macos-dry-run/windows-w1-w2-input-package.md) | Dual-platform; no final chain artifact | Requires clean install → legacy → MCP → IDE → parallel node → upgrade/rollback → uninstall across supported macOS and Windows hosts. |

## External blocker ledger

| Blocker | Affected IDs | Concrete evidence | Status implication |
| --- | --- | --- | --- |
| Historical macOS Accessibility/System Events blocker | A08, B05–B07, C05–C06 | [M3 historical blocker](evidence/m3-native-window/accessibility-blocker.md); [M9 closure](evidence/m9-macos-dry-run/native-accessibility-closure.md) | Permission became available on 2026-07-14; PID-bound native evidence supersedes this blocker. |
| Historical copied-GUI native prerequisite | E06, F04 | [M4 historical blocker](evidence/m4-f02-f04-e06/dmg-native-smoke.md); [M9 closure](evidence/m9-macos-dry-run/native-accessibility-closure.md) | Native IDE success/failure and copied GUI/legacy interaction now pass. |
| Windows host/VM and platform setup unavailable | A07, E07, F07–F10 | [M6 checklist](m6-checklist.md) and fixed Windows W1/W2 plan | Items remain BLOCKED until Windows 10/11 native host or trusted VM evidence exists. |

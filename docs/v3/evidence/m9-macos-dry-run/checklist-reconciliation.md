# Batch M9 — 58-item checklist reconciliation

Date: 2026-07-13
Authority: [`docs/v3/m6-checklist.md`](../../m6-checklist.md)
Method: cross-check each current checklist row with its cited evidence-index
record, M8 final gate, and the new clean-copy M9 evidence. A status was not
promoted from source/build/headless evidence when the acceptance requirement
requires native macOS interaction or Windows-native execution.

## Result

| Status | Count |
| --- | ---: |
| VERIFIED | 43 |
| IN_PROGRESS | 0 |
| NOT_STARTED | 1 |
| BLOCKED | 14 |
| Total | 58 |
| Strict completion | **43/58 = 74.1%** |

## Reconciled stable IDs

| ID | Status | M9 reconciliation basis |
| --- | --- | --- |
| A01 | VERIFIED | Existing bootstrap tests and V3-root evidence remain valid. |
| A02 | VERIFIED | M9 repeated absent/V2/V3/interrupted/conflict packaged results match the layout contract. |
| A03 | VERIFIED | M9 copied artifact initialized absent project and repeated init idempotently. |
| A04 | VERIFIED | M9 archive manifests were byte-identical pre/post migration. |
| A05 | VERIFIED | M9 forward recovery completed and ambiguous conflict refused mutation. |
| A06 | VERIFIED | All final M9 migration fixtures were descendants of `/private/tmp/vibehub-m9-final-20260713-1637`. |
| A07 | BLOCKED | Windows reparse/cross-volume/lock/interruption/recovery require Windows native host/VM. |
| A08 | BLOCKED | Accessibility/System Events native desktop interaction remains unavailable. |
| B01 | VERIFIED | Independent archive reader/test evidence remains valid. |
| B02 | VERIFIED | Independent Tauri loader/contract evidence remains valid. |
| B03 | VERIFIED | DTO scope evidence remains valid. |
| B04 | VERIFIED | Independent request/stale invalidation evidence remains valid. |
| B05 | BLOCKED | Native legacy list/loading/empty/error/source state requires unavailable UI automation. |
| B06 | BLOCKED | Native truthful archive-detail interaction requires unavailable UI automation. |
| B07 | BLOCKED | Native file-link success/missing/traversal/shell-failure interaction requires unavailable UI automation. |
| B08 | VERIFIED | Fabricated archive usage removal remains covered by final production gate. |
| B09 | VERIFIED | Legacy reader corpus/tests remain valid. |
| C01 | VERIFIED | Source matrix remains authoritative. |
| C02 | VERIFIED | Supported/unsupported usage capability conclusions remain covered by gate/tests. |
| C03 | VERIFIED | Read-model design/implementation evidence remains valid. |
| C04 | VERIFIED | Read-only usage service tests remain valid. |
| C05 | BLOCKED | Native loading/empty/partial/stale/error/refresh evidence requires unavailable UI automation. |
| C06 | BLOCKED | Native M1 usage drawer/session expansion evidence requires unavailable UI automation. |
| C07 | VERIFIED | Fixture/production command isolation gate still passes. |
| C08 | VERIFIED | Fixture command boundary gate still passes. |
| C09 | VERIFIED | M9 contract gate passed all M1-preservation and Tauri-boundary assertions. |
| C10 | VERIFIED | Fabrication/unsupported-state evidence remains valid. |
| D01 | VERIFIED | Consumer matrix remains valid. |
| D02 | VERIFIED | Removed duplicate command evidence remains valid. |
| D03 | VERIFIED | Removed wrapper/import evidence remains valid. |
| D04 | VERIFIED | Shared dispatcher remains covered by source and final gate. |
| D05 | VERIFIED | Retained-capability and old-entry-removal evidence remains valid. |
| D06 | VERIFIED | Old Cockpit components remain deleted per clean-break gate. |
| D07 | VERIFIED | Replaced aliases/commands/adapters remain absent per clean-break gate. |
| D08 | VERIFIED | M9 630-assertion gate preserves the clean-break static checks. |
| E01 | VERIFIED | Packaged M9 executable reported its version and V3 operations behaved safely. |
| E02 | VERIFIED | Existing packaged GUI/headless dispatcher evidence remains valid; M9 used copied headless path. |
| E03 | VERIFIED | CLI MCP tests passed (2 tests). |
| E04 | VERIFIED | M9 copied packaged MCP matrix passed: 7 resources, 7 tools, 21 operations. |
| E05 | VERIFIED | M9 copied packaged executable passed real MCP stdio matrix. |
| E06 | BLOCKED | Native preferred-IDE success/failure interaction needs unavailable UI automation. |
| E07 | BLOCKED | Windows IDE spaces/long-path/missing-file handling needs Windows host and supported IDE. |
| E08 | VERIFIED | M9 copied app completed happy worktree path and conflict-recovery path to `cleaned`. |
| F01 | VERIFIED | M9 rebuilt current arm64 app bundle successfully. |
| F02 | VERIFIED | M9 APFS DMG was mounted read-only, strict-verified, and copied fresh. |
| F03 | VERIFIED | M9 explicit ad-hoc signing and strict bundle/executable verification passed. |
| F04 | BLOCKED | Clean copied CLI/MCP passed, but GUI/legacy/IDE interactions require unavailable UI automation. |
| F05 | VERIFIED | M9 copied artifact upgrade retained archive hashes, was idempotent, recovered, and manually rolled back. |
| F06 | VERIFIED | M9 only removed `/tmp` app copy and confirmed archive/no-residue invariants. |
| F07 | BLOCKED | Windows clean install/first launch needs Windows host/VM. |
| F08 | BLOCKED | Windows upgrade/file-lock/recovery/rollback needs Windows host/VM plus old fixture. |
| F09 | BLOCKED | Windows installed CLI/MCP/IDE/parallel-node needs Windows host/VM. |
| F10 | BLOCKED | Windows uninstall/residue evidence needs Windows host/VM. |
| G01 | VERIFIED | Migration guide and isolated CLI behavior remain consistent. |
| G02 | VERIFIED | Release guide remains executable; M9 supplies refreshed prerelease artifact evidence. |
| G03 | VERIFIED | Evidence index maps all 58 IDs; this reconciliation preserves its stated counts. |
| G04 | VERIFIED | M9 contract check passed 630 assertions, including 5 write contracts. |
| G05 | NOT_STARTED | Requires final dual-platform chain, including unavailable Windows acceptance. |

## Blocker integrity

The 14 blocked IDs are exactly `A07`, `A08`, `B05–B07`, `C05–C06`, `E06`,
`E07`, and `F04`, `F07–F10`. They reduce to two demonstrated external causes:

1. **macOS native UI automation unavailable:** A08, B05–B07, C05–C06, E06,
   F04. The pre-existing System Events result is `false`; no M9 action changed
   this environment, and no non-native substitute is considered completion.
2. **No Windows 10/11 native host or trusted VM:** A07, E07, F07–F10. M9 only
   prepares the handoff package and claims no Windows execution.

`G05` is the sole `NOT_STARTED` item because it is the final dual-platform
chain, not merely an aggregation of existing macOS checks.

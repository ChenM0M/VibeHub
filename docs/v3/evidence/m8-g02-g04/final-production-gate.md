# Batch M8 — G02, G03, G04 final production gate

Date: 2026-07-13
Platform: macOS arm64, Darwin 25.5.0 (Build 25F80)
Branch: `feature/vibehub-v2-p0`

## Scope and result

G02, G03, and G04 are VERIFIED by this batch.

- **G02:** [Release operations guide](../../release-guide.md) provides executable
  macOS DMG/direct-copy installation, isolated V2→V3 upgrade, interrupted
  recovery, manual archive rollback, app-only uninstall, legacy archive
  handling, prerelease ad-hoc signing, and the explicitly unperformed formal
  Developer ID/notarization decision boundary.
- **G03:** [M6 evidence index](../../evidence-index.md) contains all 58
  checklist IDs. Its coverage scan produced 43 VERIFIED, 14 BLOCKED, and 1
  NOT_STARTED row; it records concrete evidence, platform, artifacts/hashes
  where applicable, and exact external blockers without calling blocked work
  tested.
- **G04:** [Production data-source matrix](../../production-data-source-matrix.md)
  declares real/derived/fixture/mock/placeholder/unsupported semantics. The
  expanded static gate proves production/fixture isolation, explicit
  unsupported-source handling, M1 surface preservation, old Cockpit removal,
  and retired write-path removal.

No VibeHub Agent Protocol workflow was run. This batch did not read or modify
repository `.vibehub` state, real Claude transcripts, CI configuration, or
`/Applications/VibeHub.app`. Automated test fixtures and MCP test projects are
isolated temporary data.

## Artifact provenance carried forward

| Artifact | Version/platform | SHA-256 | Source evidence |
| --- | --- | --- | --- |
| Signed packaged main executable | `2.0.0-pre.22`, macOS arm64 | `c1fc7dc0898dbcadee49d3faf67b625df922aa06c3255a7293e294ed7164d2f4` | [M2 F01/F03/E05](../m2-f01-f03-e05/native-build-sign-mcp.md) |
| Signed build-tree bundle content | `2.0.0-pre.22`, macOS arm64 | `94c1498fa9ec33d3597935b91dbfc12920126edc31a64c1ee5cce7392bf6c425` | [M2 F01/F03/E05](../m2-f01-f03-e05/native-build-sign-mcp.md) |
| APFS UDZO DMG | `2.0.0-pre.22`, macOS arm64 | `e6b1105a5a3eed1eb058f1b5b6b1480f82cd007a994182b990d9307b6a95b230` | [M4 F02/F04/E06](../m4-f02-f04-e06/dmg-native-smoke.md) |

The only signing claim remains prerelease ad-hoc signing. Developer ID signing,
notarization, stapling, and a formal-release artifact were not performed and
are not claimed.

## G04 static assertions

The expanded `scripts/v3-contracts/check.mjs` adds 26 final-production
assertions. Together with the existing D08/M1 assertions, the gate verifies:

1. the production route injects the dedicated V3 bundle loader, independent
   legacy reader, and production usage reader;
2. the production loader invokes only `v3_load_view_bundle` and maps the
   complete versioned bundle;
3. fixture route/scenario selection receives and retains no production loader,
   lifecycle, task-create, legacy-file, or project-file callback;
4. Claude App and Cursor remain explicit unsupported summaries and are excluded
   from token/freshness/completeness aggregation;
5. the capability contract documents those unsupported summaries;
6. M1 usage/token/cost, scenario, debug, settings, and setup surfaces remain;
7. existing D08 assertions still reject old Cockpit imports/reachability,
   retired Tauri registrations/wrappers/types, duplicated desktop dispatch,
   and legacy/V3 reader coupling.

The gate validates isolation rather than deleting debug/scenario/settings
controls.

## Final commands and raw results

```text
$ npm run v3:contracts:check
V3 contract check passed: 565 assertions, 12 scenarios, 6 views, 2 write contracts.

$ npm run build
TypeScript and Vite production build passed.
3706 modules transformed; built in 4.47s.
Warnings only: stale baseline-browser-mapping, stale caniuse-lite, and a
pre-existing large frontend chunk advisory.

$ cargo test -p vibehub-core
275 passed; 0 failed; 0 ignored.
Doc-tests: 0 passed; 0 failed.
Warning only: existing duplicate CLI `main.rs` build-target declaration.

$ cargo test --manifest-path src-tauri/Cargo.toml
23 passed; 0 failed; 1 ignored.
Ignored test: deliberately does not read the developer's local Claude Code
transcript directory.
Warnings only: existing duplicate CLI target declaration and three unused
Gateway helper methods.

$ cargo build -p vibehub-cli
passed.
Warning only: existing duplicate CLI `main.rs` build-target declaration.

$ cargo test -p vibehub-cli
2 passed; 0 failed; 0 ignored.

$ npm run v3:mcp:check
status: passed
protocol: 2025-11-25
resources: 7
tools: 3
request/response operations: 12
total: 1144 ms

$ git diff --check
passed with no output.

$ rg <all 58 stable checklist IDs> docs/v3/evidence-index.md | wc -l
58

$ rg <all 58 stable checklist IDs> docs/v3/evidence-index.md | status column summary
43 VERIFIED
14 BLOCKED
1 NOT_STARTED
```

The MCP contract script uses a temporary V3 project generated under `/tmp`; it
reported a clean passing protocol session. No real project was migrated or
modified.

## Documentation/evidence review

The M8 review compared each row of [the evidence index](../../evidence-index.md)
with the checklist requirement and current outcome:

- all 43 VERIFIED rows cite a concrete code/test/static/native/browser evidence
  record as appropriate;
- all 14 BLOCKED rows identify either the recorded macOS
  Accessibility/System Events prerequisite or unavailable Windows host/VM;
- G05 is the single NOT_STARTED row and is not presented as tested;
- artifact rows retain exact `2.0.0-pre.22` and SHA-256 provenance where an
  executable, bundle, copied bundle, or DMG was actually produced;
- source-level items use `N/A — source-level/host-independent` instead of
  inventing a binary version/hash.

## Remaining external blockers

These are unchanged and outside M8 scope:

| Blocker | Checklist IDs | Evidence |
| --- | --- | --- |
| macOS Accessibility/System Events UI automation disabled | A08, B05–B07, C05–C06 | [M3 blocker](../m3-native-window/accessibility-blocker.md) (`osascript` reported `false`) |
| Same native UI prerequisite for copied GUI/IDE interaction | E06, F04 | [M4 blocker](../m4-f02-f04-e06/dmg-native-smoke.md) |
| Windows 10/11 native host or trusted VM unavailable | A07, E07, F07–F10 | [M6 checklist](../../m6-checklist.md) and fixed Windows plan |

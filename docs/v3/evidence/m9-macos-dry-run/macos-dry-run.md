# Batch M9 — macOS packaged dry run and blocker reconciliation

Date: 2026-07-13
Platform: macOS 26.5.1 (Build 25F80), Darwin 25.5.0, Apple arm64
Branch: `feature/vibehub-v2-p0`
Scope: one clean, isolated packaged-artifact path; no real project, repository
`.vibehub`, Claude transcript, `/Applications/VibeHub.app`, CI configuration, or
installed application was read, migrated, stopped, overwritten, or modified.

## Result

The full automatable macOS release path passed using a freshly built, explicitly
ad-hoc-signed, APFS-DMG-packaged app copied into `/private/tmp`. The first recorded M4
DMG copy was also verified and used for lifecycle/MCP checks; it exposed that
its older packaged executable lacked the subsequently added worktree CLI
commands. The M9 artifact was rebuilt from the current existing source,
re-signed, and re-packaged before the final clean-copy run. No application code
was changed to address this artifact drift.

The native interaction blockers remain unchanged. This batch did **not** retry
the unavailable Accessibility/System Events permission gate and does not claim
GUI, IDE, legacy-list/detail/link, or usage-panel interaction from process
launch, source inspection, browser preview, CLI, or MCP evidence.

## Artifact provenance and clean copy

### Recorded M4 artifact observation

The recorded APFS DMG retained its documented SHA-256:

```text
e6b1105a5a3eed1eb058f1b5b6b1480f82cd007a994182b990d9307b6a95b230
```

It was mounted read-only and copied only into
`/tmp/vibehub-m9-artifact.3QijlJ/install/VibeHub.app`. The mounted and copied
bundles both passed:

```sh
codesign --verify --deep --strict --verbose=4 "$APP"
```

with `valid on disk` and `satisfies its Designated Requirement`; its executable
reported `2.0.0-pre.22`. Its copied executable completed lifecycle and MCP
checks, but rejected `v3 ... worktree-event` as an unknown command. This is a
real stale-artifact observation, not an E08 success claim for that old DMG.

### Final M9 artifact

The current app was rebuilt from the existing **dirty** worktree, then
explicitly signed for prerelease testing and wrapped in a new APFS UDZO image
under `/private/tmp`. A late criterion-title lifecycle defect found during M9
reconciliation was fixed, fully tested, and followed by this final rebuild:

```sh
npm run tauri -- build --bundles app
codesign --force --deep --sign - --timestamp=none \
  target/release/bundle/macos/VibeHub.app
codesign --verify --deep --strict --verbose=4 \
  target/release/bundle/macos/VibeHub.app
codesign --verify --strict --verbose=4 \
  target/release/bundle/macos/VibeHub.app/Contents/MacOS/vibehub
hdiutil create -volname 'VibeHub 2.0.0-pre.22 M9' -srcfolder "$STAGE" \
  -fs APFS -format UDZO -imagekey zlib-level=9 \
  /private/tmp/vibehub-m9-final-protocol-20260713/VibeHub-2.0.0-pre.22-arm64-m9-protocol-final.dmg
```

| Field | Result |
| --- | --- |
| Version | `2.0.0-pre.22` |
| Architecture | arm64 |
| Git HEAD at build | `ed3196588d8cdc153025aff9c593dbee3164e918` |
| Worktree provenance | Dirty worktree; this is not a clean-commit/reproducible-release claim |
| Signing | Explicit ad-hoc prerelease signing; no Developer ID/notarization claim |
| Main executable SHA-256 | `24a7601f1f0f7fc1e23a3d239ab75713ea24aa99be1973264db59189cb048f52` |
| M9 APFS UDZO DMG SHA-256 | `1fe83ed96673f1c2eeb09459c5de3b121bd4a3fbf3d168297057aa1361ab8a35` |
| DMG source | `/private/tmp/vibehub-m9-final-protocol-20260713/VibeHub-2.0.0-pre.22-arm64-m9-protocol-final.dmg` |
| Clean installed copy | `/private/tmp/vibehub-m9-final-protocol-20260713/install/VibeHub.app` |

`hdiutil verify` reported the final DMG `VALID`. The M9 DMG was attached with
`hdiutil attach -readonly -nobrowse`, its mounted
app passed deep strict verification, `ditto` copied it to the fresh install
path, the copied app passed the same strict verification, and its executable
reported `2.0.0-pre.22`. The image was detached before lifecycle testing.

The earlier full-chain artifact (`b123cc44...` DMG, `ee8f8221...` executable)
ran the complete lifecycle/worktree matrix. The post-fix artifact above then
repeated DMG verification, read-only clean copy, packaged MCP, packaged
task-create criterion initialization, rollback, and uninstall. This preserves
the complete-chain evidence while proving the late protocol fix is present in
the final package.

## Isolated packaged V3 lifecycle and archive boundary

All paths below are descendants of
`/private/tmp/vibehub-m9-final-20260713-1637/projects/`.

| Fixture | Packaged-binary command/result |
| --- | --- |
| Absent project | `doctor` → `state: absent`; `init` → `status: initialized`; repeat `init` → `already_initialized`; final `doctor` → `state: v3`, `schema_version: 3`. |
| V2 fixture | `doctor` → `v2`; `migrate` → `migrated`, `archived_legacy_v2: true`; final `doctor` → V3; repeated `migrate` → `already_migrated`. |
| Interrupted forward recovery | A V3 root and `.vibehub.v2-migration` staging tree were constructed in the disposable fixture. `doctor` → `migration_interrupted`; `migrate-recover` → `recovered_forward`; final `doctor` → V3 and `legacy-v2/old-data.txt` exists. |
| Archive conflict | A V2-marked root containing `legacy-v2` produced `state: conflict`; `migrate` exited non-zero with `V3_MIGRATION_STATE_INVALID`, leaving the fixture unmodified by migration. |

The final V2 archive fixture contained four files: `project.yaml`,
`history.txt`, `tasks/current`, and `tasks/legacy/task.yaml`. Before migration
and after migration, this command produced equal sorted manifests (empty `diff
-u`):

```sh
find . -type f -print0 | LC_ALL=C sort -z | xargs -0 shasum -a 256
```

After a packaged `session-open` V3 write, the same archive manifest was again
byte-identical. This verifies archive hash fidelity and that the V3 write did
not mutate the legacy archive. The archive was only inspected by its hashes in
the `/tmp` fixture; no real project archive was read or changed.

## Packaged MCP stdio matrix

The copied M9 executable completed:

```sh
node scripts/v3-mcp/contract-test.mjs \
  /private/tmp/vibehub-m9-final-protocol-20260713/install/VibeHub.app/Contents/MacOS/vibehub
```

Result:

```text
status: passed
protocol: 2025-11-25
resources: 7
tools: 7
request/response operations: 21
total: 427 ms
```

The test uses a generated temporary project and validates `initialize`,
`resources/list`, `tools/list`, all seven schema-validated resource reads, and
all seven advertised typed tools with a clean server exit and no stderr.

The final packaged binary also created a fresh two-criterion V3 task. The
result reported `lifecycle_version: 4` (task + initial node + two
`criterion.accepted` events), and both task lifecycle and project overview
preserved the two real criterion titles with state `accepted`. This directly
verifies the late workflow-protocol fix in the installed clean copy.

## Packaged worktree lifecycle

A fresh git-initialized V3 fixture ran the current copied artifact's dispatcher
commands exclusively. The happy path appended ten events:

```text
planned → create_prepared → ready → lease.acquired → activated → dirty
→ submitted → integration_prepared → integrated → cleaned
```

The conflict-recovery path appended twelve events:

```text
planned → create_prepared → ready → lease.acquired → activated → dirty
→ submitted → integration_prepared → conflicted → repairing → integrated → cleaned
```

The projection command:

```sh
vibehub v3 <tmp-worktree> worktree-orchestration \
  project.m9-worktree task.m9-worktree
```

reported `worktree.alpha: cleaned, version 10` and
`worktree.beta: cleaned, version 12`, with no unknown event types. This is
packaged/copy evidence for E08's happy and conflict-recovery paths.

## Isolated upgrade, idempotency, recovery, and rollback

A second disposable V2 fixture was upgraded with the copied M9 artifact.
`doctor` reported V2; `migrate` returned `migrated`; repeat `migrate` returned
`already_migrated`; and final `doctor` before rollback reported V3. The complete
pre-upgrade archive manifest exactly matched
`.vibehub/legacy-v2` afterward.

Manual rollback followed the documented copy-only procedure:

```sh
ditto "$PROJECT/.vibehub" "$PROJECT/.vibehub.pre-v2-rollback"
ditto "$PROJECT/.vibehub/legacy-v2" "$PROJECT/.vibehub.v2-restore"
rm -rf "$PROJECT/.vibehub"
mv "$PROJECT/.vibehub.v2-restore" "$PROJECT/.vibehub"
vibehub v3 "$PROJECT" doctor
```

The final doctor output was `state: v2`, confirming that the original V2 tree
was restored from the preserved archive. All four restored files retained the
archive SHA-256 values, and the pre-rollback V3 backup remained present. This
destructive operation was scoped only to
`/private/tmp/vibehub-m9-final-20260713-1637/projects/rollback-final`.
The same rollback was repeated with the post-fix final artifact under
`/private/tmp/vibehub-m9-final-protocol-20260713/projects/rollback-final`.

## Uninstall and residue

Only the app copy under
`/private/tmp/vibehub-m9-final-protocol-20260713/uninstall-final/Applications/VibeHub.app`
was removed. The test then
verified:

- the isolated `/private/tmp` app copy no longer existed;
- the migrated fixture's `.vibehub/legacy-v2` archive remained present;
- the isolated `Library` tree contained no helper/runtime/cache file;
- no `com.vibehub*.plist` existed under the isolated `/private/tmp` tree; and
- no Git worktree administration directory existed in the fixture.

No application in `/Applications`, no user Application Support directory, and
no real project were touched.

## Native interaction blocker reconciliation

The external macOS native-interaction blocker was reconfirmed after the user
stated that permission was already enabled:

```sh
osascript -e 'tell application "System Events" to return UI elements enabled'
# false

# One real AX window query against the running final clean-copy process:
# “osascript”不允许辅助访问。 (-25211)
```

The final app process was running, so this is specifically an effective TCC
authorization failure for the current automation client, not an app-launch
failure. Per the fixed execution plan, M9 stopped after this user-requested
single recheck and actual AX attempt. The following remain `BLOCKED` because
packaged CLI/MCP/process evidence cannot prove the required visible native
interaction:

| IDs | Exact missing evidence |
| --- | --- |
| A08 | Desktop inspect → initialize/migrate/recover confirmation, errors, and guidance. |
| B05–B07 | Native legacy list/loading/empty/error states, truthful details, and local file-open success/missing/traversal/shell-failure interactions. |
| C05–C06 | Native production usage loading/empty/partial/stale/error/refresh and M1 drawer/session expansion. |
| E06 | Native preferred-IDE open/reveal success and understandable failure. |
| F04 | Clean copied-app GUI/legacy/IDE interaction matrix. |

No Browser preview, running GUI process, source inspection, headless CLI, or MCP
result has been substituted for these requirements.

## Final gate results

```text
npm run v3:contracts:check
  PASS — 630 assertions, 12 scenarios, 6 views, 5 write contracts.

npm run build
  PASS — TypeScript and Vite production build; 3708 modules transformed.

cargo test -p vibehub-core
  PASS — 299 passed, 0 failed, 0 ignored.

cargo test --manifest-path src-tauri/Cargo.toml
  PASS — 23 passed, 0 failed, 1 intentionally ignored.

cargo build -p vibehub-cli
  PASS.

cargo test -p vibehub-cli
  PASS — 2 passed, 0 failed, 0 ignored.

npm run v3:mcp:check
  PASS — packaged executable, protocol 2025-11-25; 7 resources; 7 tools;
  21 operations; 427 ms.

git diff --check
  PASS — no output.
```

Warnings were limited to existing duplicate CLI build-target metadata, unused
helpers, stale browser-data notices, and the Vite large-chunk advisory.

## M9 reconciliation result

The original dry-run evidence review recorded **43 VERIFIED, 0 IN_PROGRESS,
1 NOT_STARTED, 14 BLOCKED; 43/58 = 74.1%**. On 2026-07-14, direct PID-bound
Accessibility evidence superseded the eight macOS native blockers without using
a browser/headless substitute. The current reconciliation is **51 VERIFIED,
0 IN_PROGRESS, 1 NOT_STARTED, 6 BLOCKED; 51/58 = 87.9%**. See
[native Accessibility closure](native-accessibility-closure.md), the updated
[M9 checklist reconciliation](checklist-reconciliation.md), and the remaining
[W1/W2 input package](windows-w1-w2-input-package.md).

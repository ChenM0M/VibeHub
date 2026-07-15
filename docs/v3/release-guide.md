# V3 macOS release operations guide

Date: 2026-07-15
Scope: VibeHub `3.0.0` release workflow and historical macOS validation evidence

This guide is executable only against a **copied or disposable project** until a
release owner has completed its own backup and approval process. It does not
use the retired VibeHub Agent Protocol, does not modify Claude transcripts, and
does not require the real VibeHub repository to be migrated.

The V3 bootstrap commands in this guide are the current commands implemented by
the product:

```text
vibehub v3 <project> doctor
vibehub v3 <project> init
vibehub v3 <project> migrate
vibehub v3 <project> migrate-recover
```

`doctor` is read-only. It must be run before and after every bootstrap or
rollback operation. See [the migration guide](migration-guide.md) for all
layout states and error codes.

## Release artifact identity and evidence

The `3.0.0` GitHub Actions workflow validates version equality and the complete
test gate, then creates one draft Release containing macOS, Windows, and Linux
assets plus per-platform SHA-256 manifests. Final artifact filenames and hashes
do not exist until that tagged workflow succeeds; never copy historical hashes
into a `3.0.0` release record.

### Historical native evidence

The following earlier artifact retains native macOS behavioral evidence:

| Item | Recorded value |
| --- | --- |
| App bundle | `target/release/bundle/macos/VibeHub.app` |
| Main executable | `VibeHub.app/Contents/MacOS/vibehub` |
| Version | `2.0.0-pre.22` |
| Architecture | Mach-O arm64 |
| Signed executable SHA-256 | `c1fc7dc0898dbcadee49d3faf67b625df922aa06c3255a7293e294ed7164d2f4` |
| Signed build-tree bundle content SHA-256 | `94c1498fa9ec33d3597935b91dbfc12920126edc31a64c1ee5cce7392bf6c425` |
| APFS DMG | `target/release/bundle/macos/VibeHub-2.0.0-pre.22-arm64-apfs.dmg` |
| DMG SHA-256 | `e6b1105a5a3eed1eb058f1b5b6b1480f82cd007a994182b990d9307b6a95b230` |

The artifact was explicitly ad-hoc signed, verified with strict `codesign`, and
exercised through the packaged MCP stdio matrix. The signed-artifact and MCP
evidence is in [M2 native build, signing, and MCP evidence](evidence/m2-f01-f03-e05/native-build-sign-mcp.md).
The APFS DMG creation, read-only mount, copied-bundle verification, and copied
runtime smoke are in [M4 DMG evidence](evidence/m4-f02-f04-e06/dmg-native-smoke.md).

A copied bundle's regular-file manifest hash was
`d5e3dd0f94d8232c618af8dcb45b6007ce68313fb6c28286f419a8082f89d90a`.
That differs from the build-tree aggregate because filesystem copy metadata
varies; the executable hash above and strict sealed-resource verification are
the payload checks.

## 1. Verify the downloaded or supplied artifact

Set the paths explicitly. Do not substitute an unrelated installed copy from
`/Applications` for the release artifact.

```bash
DMG="/absolute/path/to/VibeHub_3.0.0_aarch64.dmg"
APP="/absolute/path/to/VibeHub.app"

shasum -a 256 "$DMG"
codesign --verify --deep --strict --verbose=4 "$APP"
codesign --verify --strict --verbose=4 "$APP/Contents/MacOS/vibehub"
"$APP/Contents/MacOS/vibehub" --version
```

For `3.0.0`, compare the DMG with `SHA256SUMS-macos-aarch64.txt` from the same
draft Release. `codesign` must report that the bundle is valid on disk and
satisfies its designated requirement. A hash or signature mismatch is a stop
condition: do not install, launch, or migrate a project with that artifact.

## 2. Install on macOS from the APFS DMG

Mount the image read-only, verify the mounted bundle, and copy it to a location
owned by the intended user. `~/Applications` avoids requiring administrator
rights; use `/Applications` only when the release owner intentionally wants a
system-wide install.

```bash
DMG="/absolute/path/to/VibeHub_3.0.0_aarch64.dmg"
MOUNT="$(mktemp -d /tmp/vibehub-mount.XXXXXX)"
DESTINATION="$HOME/Applications/VibeHub.app"

hdiutil attach -readonly -nobrowse -mountpoint "$MOUNT" "$DMG"
codesign --verify --deep --strict --verbose=4 "$MOUNT/VibeHub.app"
mkdir -p "$(dirname "$DESTINATION")"
rm -rf "$DESTINATION"
ditto "$MOUNT/VibeHub.app" "$DESTINATION"
codesign --verify --deep --strict --verbose=4 "$DESTINATION"
"$DESTINATION/Contents/MacOS/vibehub" --version
hdiutil detach "$MOUNT"
rmdir "$MOUNT"
```

The `rm -rf` line is intentionally limited to the explicit `DESTINATION` value.
Before running it, print and inspect the value:

```bash
printf 'Replacing only: %s\n' "$DESTINATION"
```

The recorded M4 smoke used this same attach → verify → `ditto` copy shape in an
isolated `/tmp` destination. It also ran the copied executable's headless CLI
and full MCP request matrix. The later M9 PID-bound Accessibility run closed
the native GUI/IDE interaction gap; see
[`native-accessibility-closure.md`](evidence/m9-macos-dry-run/native-accessibility-closure.md).

## 3. Install by direct app-bundle copy

If the signed `VibeHub.app` bundle was delivered directly rather than in a DMG,
verify it first and copy it with `ditto`:

```bash
SOURCE_APP="/absolute/path/to/VibeHub.app"
DESTINATION="$HOME/Applications/VibeHub.app"

codesign --verify --deep --strict --verbose=4 "$SOURCE_APP"
mkdir -p "$(dirname "$DESTINATION")"
rm -rf "$DESTINATION"
ditto "$SOURCE_APP" "$DESTINATION"
codesign --verify --deep --strict --verbose=4 "$DESTINATION"
"$DESTINATION/Contents/MacOS/vibehub" --version
```

This operation replaces only the selected application bundle. It does not
migrate projects and does not alter project-local V3 or `legacy-v2` archives.

## 4. Upgrade an isolated V2 project to V3

Migration is safe only after a complete backup and a first run on a copied
project. Never use the real VibeHub repository as the first migration or
acceptance target.

```bash
SOURCE_PROJECT="/absolute/path/to/original-v2-project"
WORK_ROOT="$(mktemp -d /tmp/vibehub-v3-upgrade.XXXXXX)"
PROJECT="$WORK_ROOT/project-copy"

# Copy the whole project before making any V3 change.
ditto "$SOURCE_PROJECT" "$PROJECT"

# Capture byte-level V2 archive provenance before migration.
(
  cd "$PROJECT/.vibehub"
  find . -type f -print0 | LC_ALL=C sort -z | xargs -0 shasum -a 256
) > "$WORK_ROOT/v2-before.sha256"

vibehub v3 "$PROJECT" doctor
vibehub v3 "$PROJECT" migrate
vibehub v3 "$PROJECT" doctor

# The post-migration archive must match the pre-migration V2 tree.
(
  cd "$PROJECT/.vibehub/legacy-v2"
  find . -type f -print0 | LC_ALL=C sort -z | xargs -0 shasum -a 256
) > "$WORK_ROOT/v2-archive-after.sha256"
diff -u "$WORK_ROOT/v2-before.sha256" "$WORK_ROOT/v2-archive-after.sha256"
```

Proceed only when the initial `doctor` reports `state: v2`, `migrate` reports
`status: migrated`, the final `doctor` reports `state: v3` and
`schema_version: 3`, and the checksum diff is empty. A repeated migration of a
healthy migrated project reports `already_migrated`; it must not replace the
existing archive.

The macOS upgrade evidence used an isolated V2 fixture, verified five archived
files with identical SHA-256 values, exercised idempotent migration and
interrupted forward recovery, then performed a rollback. See [M7 upgrade,
worktree, and uninstall evidence](evidence/m7-e08-f05-f06/worktree-upgrade-uninstall.md).

### Interrupted upgrade recovery

If `doctor` reports `migration_interrupted`, do not rerun `migrate`, manually
rename paths, or remove the staging directory. Run only:

```bash
vibehub v3 "$PROJECT" migrate-recover
vibehub v3 "$PROJECT" doctor
```

The result is either `rolled_back_to_v2` or `recovered_forward`, determined
from the on-disk state. An ambiguous root or existing archive fails closed.

## 5. Roll back a copied V3 project to its V2 archive

There is no automatic reverse-migration command. The observed rollback is a
manual restore of the original V2 tree from `legacy-v2`. Use it only on a
copied/backup project and preserve the current V3 root first.

```bash
PROJECT="/absolute/path/to/copied-v3-project"

vibehub v3 "$PROJECT" doctor
# Confirm the output is state: v3 and legacy-v2 exists before continuing.
test -d "$PROJECT/.vibehub/legacy-v2"

# Preserve the complete V3 root, including its legacy archive, outside the
# root that will be replaced.
rm -rf "$PROJECT/.vibehub.pre-v2-rollback" "$PROJECT/.vibehub.v2-restore"
ditto "$PROJECT/.vibehub" "$PROJECT/.vibehub.pre-v2-rollback"
ditto "$PROJECT/.vibehub/legacy-v2" "$PROJECT/.vibehub.v2-restore"

# Replace only the copied project's V3 root with the preserved V2 payload.
rm -rf "$PROJECT/.vibehub"
mv "$PROJECT/.vibehub.v2-restore" "$PROJECT/.vibehub"

vibehub v3 "$PROJECT" doctor
```

The final `doctor` must report `state: v2`. Keep
`.vibehub.pre-v2-rollback` until the restored V2 project has been independently
checked. Do not delete it merely to make a rollback appear successful. The
recorded F05 rollback used this exact semantic outcome: remove the V3 root,
restore the V2 archive, then confirm V2 layout detection.

## 6. Archive layout and read-only use

A migrated V3 project has this relevant layout:

```text
<project>/.vibehub/
  project.yaml                 # V3 schema marker (schema_version: 3)
  events/
  projections/
  indexes/
  runtime/
  legacy-v2/                   # byte-preserved former V2 .vibehub tree
```

`legacy-v2/` is an archive of the original V2 `.vibehub` contents, not a
converted V3 domain model. V3 reads it through the independent legacy read
model. Treat the archive as immutable operational data:

- do not edit, overwrite, or delete it during normal V3 operation;
- do not use it as a place for new V3 events, projections, tasks, or runtime
  files;
- preserve it when copying, backing up, uninstalling, or diagnosing a project;
- use it as the source for the manual rollback procedure above.

V3 migration refuses to overwrite an existing archive and recovery fails closed
when automatic preservation would be ambiguous. The isolated F05 evidence
proves five archive files retain their original SHA-256 values.

## 7. Uninstall macOS application files without deleting projects

Quit the application, then remove the **specific installed app bundle**. This
is an app uninstall, not a request to remove project data.

```bash
APP="$HOME/Applications/VibeHub.app"
PROJECT="/absolute/path/to/project-that-must-be-preserved"

printf 'Removing app bundle only: %s\n' "$APP"
printf 'Preserving project archive: %s\n' "$PROJECT/.vibehub"
test "$(basename "$APP")" = "VibeHub.app"
rm -rf "$APP"

test ! -e "$APP"
test -d "$PROJECT/.vibehub"
find "$HOME/Library/LaunchAgents" -maxdepth 1 -name 'com.vibehub*.plist' -print
```

If the app was installed in `/Applications`, set `APP=/Applications/VibeHub.app`
and inspect that value before deletion. The app does not install LaunchAgents,
helpers, or background daemons. Project archives always reside inside their
project directory, so app-bundle removal must not remove `.vibehub/` or
`legacy-v2/`.

`~/Library/Application Support/VibeHub/` may contain user configuration such as
`config.json` and `gateway_config.json`. It is not a project archive. Leave it
in place for a normal uninstall; remove it only as a separately approved
preference reset after backing it up. Do not delete arbitrary project folders
or their `.vibehub` data as part of app removal.

The recorded F06 test installed an isolated app copy under `/tmp`, uninstalled
only that bundle, and verified app removal, archive preservation, V3 doctor
continuity, no LaunchAgents, and no worktree residue. See [M7 uninstall
evidence](evidence/m7-e08-f05-f06/worktree-upgrade-uninstall.md).

## 8. Signing boundary

### Tested prerelease path

The recorded prerelease build was signed explicitly with an ad-hoc identity:

```bash
codesign --force --deep --sign - --timestamp=none VibeHub.app
codesign --verify --deep --strict --verbose=4 VibeHub.app
```

It verified successfully, but it has `Signature=adhoc`, no TeamIdentifier, no
Developer ID identity, and no timestamp. It is suitable only for the recorded
prerelease validation boundary. It is **not** a notarized or publicly
Developer-ID-signed distribution claim.

### Formal-release workflow (configured; evidence requires a successful run)

The Release workflow now requires all Apple Developer ID and notarization
Secrets for a stable tag. It imports the certificate, builds both macOS
architectures, verifies the app, notarizes and staples each DMG, and records
post-notarization hashes. A stable workflow fails instead of silently falling
back to ad-hoc signing. A tag containing `-` may still generate an explicitly
non-formal prerelease artifact when credentials are absent.

The workflow implements these gates:

1. build the final, versioned bundle from reviewed source;
2. sign every required code object with the intended `Developer ID Application`
   identity and a secure timestamp;
3. verify the signed bundle with strict `codesign` and inspect its identity;
4. submit the exact signed distribution artifact to Apple's notarization
   service, wait for an accepted result, then staple and reassess the artifact;
5. produce and record final hashes **after** signing/notarization; and
6. repeat the clean-copy, CLI, and MCP release checks against that exact final
   artifact.

Do not reuse the historical ad-hoc SHA-256 values as formal-release hashes.
Developer ID signature, notarization, staple, Gatekeeper assessment, and final
hashes may be claimed only from a successful tagged GitHub Actions run and its
uploaded draft-Release assets.

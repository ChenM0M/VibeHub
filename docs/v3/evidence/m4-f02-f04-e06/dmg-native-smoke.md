# Batch M4 — APFS DMG and packaged runtime evidence

Date: 2026-07-12
Platform: macOS 26.5.1 (Build 25F80), Apple arm64

## F02 APFS DMG

Source app:

```text
/Users/chenm0m/LocalRepo/VibeHub/target/release/bundle/macos/VibeHub.app
```

DMG creation used an isolated `/tmp` staging directory containing `VibeHub.app` and an `Applications` symlink:

```sh
hdiutil create \
  -volname 'VibeHub 2.0.0-pre.22' \
  -srcfolder "$STAGE" \
  -fs APFS \
  -format UDZO \
  -imagekey zlib-level=9 \
  /Users/chenm0m/LocalRepo/VibeHub/target/release/bundle/macos/VibeHub-2.0.0-pre.22-arm64-apfs.dmg
```

Artifact:

```text
/Users/chenm0m/LocalRepo/VibeHub/target/release/bundle/macos/VibeHub-2.0.0-pre.22-arm64-apfs.dmg
SHA-256: e6b1105a5a3eed1eb058f1b5b6b1480f82cd007a994182b990d9307b6a95b230
Image format: UDZO
Volume name: VibeHub 2.0.0-pre.22
File System Personality: APFS
Type (Bundle): apfs
```

The DMG was attached read-only with `hdiutil attach`, and `diskutil info` confirmed APFS. The mounted app passed:

```sh
codesign --verify --deep --strict --verbose=4 "$MOUNT/VibeHub.app"
```

Result:

```text
VibeHub.app: valid on disk
VibeHub.app: satisfies its Designated Requirement
```

The app was copied from the mounted image into a fresh `/tmp/vibehub-m4-install.*` directory using `ditto`. The copied executable reported:

```text
2.0.0-pre.22
Mach-O 64-bit executable arm64
SHA-256: c1fc7dc0898dbcadee49d3faf67b625df922aa06c3255a7293e294ed7164d2f4
```

This executable hash exactly matches the signed Batch M2 executable. The copied app also passed deep strict codesign verification.

The copied bundle's regular-file manifest hash was `d5e3dd0f94d8232c618af8dcb45b6007ce68313fb6c28286f419a8082f89d90a`; filesystem copy metadata differs from the build-tree aggregate, while the executable hash and strict sealed-resource verification prove the signed payload remained valid.

## Copied packaged runtime smoke

### GUI first launch

A fresh second mount/copy was launched directly from `/tmp/vibehub-m4-launch-install.*`. `VIBEHUB_PORTABLE=1` and a temporary app-local `data/config.json` isolated application storage from the user's installed app and default Application Support directory.

Observed after four seconds:

```text
process state: running
command: /tmp/vibehub-m4-launch-install.*/VibeHub.app/Contents/MacOS/vibehub
portable config: present
OpenAI Chat Gateway listening on 127.0.0.1:12347
OpenAI Responses Gateway listening on 127.0.0.1:12346
Anthropic Gateway listening on 127.0.0.1:12345
```

The copied process was terminated after the smoke. The pre-existing `/Applications/VibeHub.app` process was not used, terminated, or modified.

### Headless CLI and MCP

The copied executable completed `--version` and the complete MCP contract matrix:

```text
protocol: 2025-11-25
resources: 7
tools: 3
request/response operations: 12
total: 208 ms
clean exit: yes
stderr: empty
```

The matrix included initialize, resources/list, tools/list, five schema-validated resources/read calls, idempotent session_open, event_log, and session_close.

## F04 and E06 remaining external gate

The required clean-copy GUI, legacy browsing, project lifecycle, IDE open/reveal interaction, and understandable UI failure evidence cannot be completed reproducibly because the actual host process lacks macOS Accessibility/System Events UI automation permission:

```sh
osascript -e 'tell application "System Events" to return UI elements enabled'
# false
```

Launching a process and exercising headless commands do not prove those UI interactions, so F04 and E06 are not claimed as VERIFIED.

Visual Studio Code is available at `/Applications/Visual Studio Code.app` and `/opt/homebrew/bin/code`, but the current backend project-file actions call the system default `open` command. Proving the user-visible success/failure surface requires native UI interaction. No IDE process was launched merely to manufacture success evidence.

## Safety

- All staging, mount, install, application data, and MCP projects were isolated under `/tmp`.
- The DMG mount was read-only and detached after each check.
- No real project was migrated or mutated.
- No real transcript was read or modified.
- No installed application, CI file, or repository `.vibehub` file was modified.

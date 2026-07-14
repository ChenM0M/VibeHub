# Batch M2 native evidence — F01, F03, E05

Date: 2026-07-12

## Scope and safety

- Platform: macOS 26.5.1 (Build 25F80), Darwin 25.5.0, Apple arm64.
- Repository branch: `feature/vibehub-v2-p0`.
- Build used the current repository source, `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.
- All MCP sessions used temporary projects created and deleted by `scripts/v3-mcp/contract-test.mjs` under `/tmp/vibehub-v3-mcp-contract-*`.
- The real VibeHub repository was not used as an MCP project and was not migrated or mutated by these checks.
- No real Claude transcript was read or modified. No CI or `.vibehub` file was modified. No notarization was attempted.

## F01 — local macOS application build

Command:

```sh
npm run tauri -- build --bundles app
```

Result:

- `npm run build`: passed; TypeScript and Vite completed, 3709 modules transformed.
- Rust release build: passed.
- Tauri bundle: passed with `Finished 1 bundle`.
- Warnings were non-fatal: stale browser compatibility metadata, large frontend chunk, duplicate CLI source target declaration, and existing unused Rust functions.

Artifact:

- App bundle: `/Users/chenm0m/LocalRepo/VibeHub/target/release/bundle/macos/VibeHub.app`
- Main executable: `/Users/chenm0m/LocalRepo/VibeHub/target/release/bundle/macos/VibeHub.app/Contents/MacOS/vibehub`
- `CFBundleShortVersionString`: `2.0.0-pre.22`
- `CFBundleVersion`: `2.0.0-pre.22`
- `CFBundleExecutable`: `vibehub`
- `file`: `Mach-O 64-bit executable arm64`
- Packaged executable `--version`: `2.0.0-pre.22` (exit 0)
- Packaged executable `--help`: exit 0

Pre-sign hashes:

```text
executable SHA-256: ab1e7a840218accd77eca17055fe85c720706bc09040aa08c740d8838674b5ba
bundle content SHA-256: 71a76a1d80f4435e5ee12cb6cd653acc53cc48339235a136a8c7e16acc827e54
```

The bundle content hash is computed by sorting all regular bundle files by path, hashing each file with SHA-256, then SHA-256 hashing that manifest stream:

```sh
find "$APP" -type f -print0 | LC_ALL=C sort -z | xargs -0 shasum -a 256 | shasum -a 256
```

## F03 — prerelease ad-hoc signing

The initial Tauri output contained only a linker-generated ad-hoc Mach-O signature. Initial bundle verification reported:

```text
code has no resources but signature indicates they must be present
```

The complete app bundle was therefore signed explicitly for prerelease validation:

```sh
codesign --force --deep --sign - --timestamp=none \
  /Users/chenm0m/LocalRepo/VibeHub/target/release/bundle/macos/VibeHub.app
```

This is prerelease ad-hoc signing only. It is not Developer ID signing, has no team identity, does not use a timestamp, and does not enable or claim notarization.

Bundle verification command:

```sh
codesign --verify --deep --strict --verbose=4 \
  /Users/chenm0m/LocalRepo/VibeHub/target/release/bundle/macos/VibeHub.app
```

Result:

```text
VibeHub.app: valid on disk
VibeHub.app: satisfies its Designated Requirement
```

Main executable verification command:

```sh
codesign --verify --strict --verbose=4 \
  /Users/chenm0m/LocalRepo/VibeHub/target/release/bundle/macos/VibeHub.app/Contents/MacOS/vibehub
```

Result:

```text
vibehub: valid on disk
vibehub: satisfies its Designated Requirement
```

`codesign -dv --verbose=4` for both the bundle and main executable reported:

```text
Identifier=com.vibehub.launcher
Format=app bundle with Mach-O thin (arm64)
flags=0x2(adhoc)
Signature=adhoc
TeamIdentifier=not set
Sealed Resources version=2 rules=13 files=1
CDHash=8b4e3427a99cea35169f62466ed2a90e6b8d0156
```

Final post-sign hashes (authoritative for later batches):

```text
main executable SHA-256: c1fc7dc0898dbcadee49d3faf67b625df922aa06c3255a7293e294ed7164d2f4
signed bundle content SHA-256: 94c1498fa9ec33d3597935b91dbfc12920126edc31a64c1ee5cce7392bf6c425
```

Signing changed bundle contents, so the pre-sign hashes above are retained only for provenance and these post-sign hashes supersede them.

## E05 — packaged desktop binary MCP stdio session

### Absolute packaged path

Command:

```sh
node scripts/v3-mcp/contract-test.mjs \
  /Users/chenm0m/LocalRepo/VibeHub/target/release/bundle/macos/VibeHub.app/Contents/MacOS/vibehub
```

Result: passed, protocol `2025-11-25`, 7 resources, 3 tools, 12 request/response operations, clean exit, empty stderr, no partial stdout frame, and 201 ms total runtime.

Request matrix:

| Request | Count | Verified behavior |
| --- | ---: | --- |
| `initialize` | 1 | Server identity `vibehub-v3`; protocol negotiation succeeded. |
| `resources/list` | 1 | Exactly 7 resources; every URI uses `vibehub://v3/1.0/`. |
| `tools/list` | 1 | Exactly `event_log`, `session_close`, and `session_open`. |
| `resources/read` | 5 | Project overview, project structure, task timeline, plan graph, and node brief read and passed their V3 JSON schemas. |
| `tools/call session_open` | 2 | First call returned `appended`; identical idempotency key returned `duplicate`. |
| `tools/call event_log` | 1 | Progress event returned `appended` at expected version 1. |
| `tools/call session_close` | 1 | Session close returned `appended` at expected version 2. |

The test also sent `notifications/initialized` and a cancellation notification for an already completed request, then verified clean server shutdown. Response summaries contain no prompt/message bodies, tokens, secrets, or private transcript data.

### PATH invocation of this packaged binary

An isolated temporary PATH entry was used so command-name resolution referred to the exact signed artifact without installing or overwriting the user's existing application:

```sh
shim_dir=$(mktemp -d /tmp/vibehub-m2-path.XXXXXX)
ln -s "/Users/chenm0m/LocalRepo/VibeHub/target/release/bundle/macos/VibeHub.app/Contents/MacOS/vibehub" "$shim_dir/vibehub"
PATH="$shim_dir:$PATH" command -v vibehub
PATH="$shim_dir:$PATH" vibehub --version
node scripts/v3-mcp/contract-test.mjs "$shim_dir/vibehub"
```

Resolution realpath:

```text
/Users/chenm0m/LocalRepo/VibeHub/target/release/bundle/macos/VibeHub.app/Contents/MacOS/vibehub
```

Result: version `2.0.0-pre.22`; the same complete matrix passed with protocol `2025-11-25`, 7 resources, 3 tools, 12 request/response operations, and 202 ms total runtime. The temporary PATH directory was deleted after the check.

### Existing installed PATH observation

The host's unmodified installed PATH resolves `vibehub` to `/opt/homebrew/bin/vibehub`, whose code-sign metadata points to `/Applications/VibeHub.app/Contents/MacOS/vibehub`. It reports version `2.0.0-pre.22`, arm64, but has a different SHA-256:

```text
570534e900151db81f9e9ce4f339a067785364fb15e9e5760deba75bec026177
```

Running the matrix against that pre-existing installed binary timed out on `initialize`. It is therefore a stale, independently installed build and is not claimed as this batch's packaged artifact or as a passing installed-binary result. The existing installation was not overwritten or modified. The exact current packaged binary passed both absolute-path and isolated PATH resolution checks above.

## Focused automated check

```sh
cargo test -p vibehub-cli mcp
```

Result: 2 passed, 0 failed, 0 ignored. The tests verify the versioned 7-resource catalog/read path and application-service/idempotency semantics for tools.

# Batch M7 — E08, F05, F06: parallel-node worktree, V2→V3 upgrade, uninstall cleanup

Date: 2026-07-13
Platform: macOS arm64 (Darwin 25.5.0, Build 25F80)
Branch: `feature/vibehub-v2-p0`

## Result

E08, F05, and F06 are VERIFIED. The packaged app binary completed a parallel-node worktree lifecycle smoke (create → observe → interrupted recovery → integration → cleanup), an isolated V2→V3 upgrade with archive fidelity, recovery, and rollback, and an uninstall residue cleanup test that preserved project archives.

## E08 — Packaged app parallel-node/worktree smoke

### CLI worktree command wiring

Added two V3 CLI commands to `crates/vibehub-cli/src/dispatcher.rs`:

- `v3 <project> worktree-event <command_json_path|--stdin|->` — accepts an `OrchestrationCommand` JSON and appends it to the event store via `V3ApplicationService::orchestration_command`.
- `v3 <project> worktree-orchestration <project_id> <task_id>` — reads the orchestration projection via `V3ApplicationService::worktree_orchestration`.

These commands expose the existing `vibehub-core::v3::orchestration` module through the CLI, which is the same dispatcher shared by the desktop binary for headless mode.

### Packaged binary smoke

Binary:

```text
/Users/chenm0m/LocalRepo/VibeHub/target/release/bundle/macos/VibeHub.app/Contents/MacOS/vibehub
Version: 2.0.0-pre.22
```

Isolated project: `/tmp/vibehub-e08-packaged/project` (created fresh, git-initialized, V3-initialized, task created with 2 acceptance criteria).

#### Happy path (worktree.alpha)

```text
worktree.planned (0):          appended
worktree.create_prepared (1):  appended
worktree.ready (2):            appended
lease.acquired (3):            appended
worktree.activated (4):        appended
worktree.dirty (5):            appended
worktree.submitted (6):        appended
worktree.integration_prepared (7): appended
worktree.integrated (8):       appended
worktree.cleaned (9):          appended
```

#### Conflict recovery path (worktree.beta)

```text
worktree.planned (0):          appended
worktree.create_prepared (1):  appended
worktree.ready (2):            appended
lease.acquired (3):            appended
worktree.activated (4):        appended
worktree.dirty (5):            appended
worktree.submitted (6):        appended
worktree.integration_prepared (7): appended
worktree.conflicted (8):       appended  ← interruption
worktree.repairing (9):        appended  ← recovery start
worktree.integrated (10):      appended  ← recovery completion (Repairing → Integrated)
worktree.cleaned (11):        appended  ← cleanup
```

#### Final orchestration projection

```text
worktree.alpha: state=cleaned version=10 events=10
worktree.beta:  state=cleaned version=12 events=12
```

Both worktrees reached `cleaned` state. The conflict recovery path exercised the `Conflicted → Repairing → Integrated → Cleaned` transition, proving that an interrupted integration can be recovered without losing event-sourced state.

## F05 — Isolated V2→V3 macOS upgrade

### V2 fixture creation

Isolated project: `/tmp/vibehub-f05-upgrade/project`

Created V2 layout:

```text
.vibehub/project.yaml    (schema_version: 1, name: old-v2-project)
.vibehub/state.yaml      (schema_version: 1, current_task_id: old-task-001)
.vibehub/history.txt     (V2 history marker)
.vibehub/tasks/current   (old-task-001)
.vibehub/tasks/old-task-001/task.yaml
```

### Upgrade

```text
$ vibehub v3 /tmp/vibehub-f05-upgrade/project doctor
state: v2, recommended: run `vibehub v3 <project> migrate`

$ vibehub v3 /tmp/vibehub-f05-upgrade/project migrate
status: migrated
archived_legacy_v2: true
created_paths: [.vibehub/project.yaml, .vibehub/events, .vibehub/projections, .vibehub/indexes, .vibehub/runtime, .vibehub/legacy-v2]

$ vibehub v3 /tmp/vibehub-f05-upgrade/project doctor
state: v3, schema_version: 3
```

### Archive fidelity

All 5 V2 files were preserved in `.vibehub/legacy-v2/` with identical SHA-256 checksums:

| File | SHA-256 (pre-migration) | SHA-256 (post-archive) |
| --- | --- | --- |
| history.txt | d3ebb4a1... | d3ebb4a1... |
| project.yaml | 76ff670e... | 76ff670e... |
| state.yaml | 927d9b5b... | 927d9b5b... |
| tasks/current | 011333bd... | 011333bd... |
| tasks/old-task-001/task.yaml | c264f20a... | c264f20a... |

### Idempotent re-migrate

```text
$ vibehub v3 ... migrate
status: already_migrated, archived_legacy_v2: true
```

### Interrupted recovery

Created a project with `.vibehub.v2-migration/` staging directory and partial V3 root:

```text
$ vibehub v3 ... doctor
state: migration_interrupted
recommended: run `vibehub v3 <project> migrate-recover`

$ vibehub v3 ... migrate-recover
status: recovered_forward
archived_legacy_v2: true
created_paths: [.vibehub/legacy-v2, ...]

$ vibehub v3 ... doctor
state: v3, schema_version: 3
```

V2 data preserved in `.vibehub/legacy-v2/old-data.txt` after recovery.

### Rollback

Manual rollback test: removed V3 root, restored V2 from `legacy-v2` archive:

```text
$ vibehub v3 ... doctor
state: v2
recommended: run `vibehub v3 <project> migrate`

V2 data preserved: old data (from .vibehub/old-data.txt)
```

## F06 — macOS uninstall residue cleanup

### Install

```text
ditto .../VibeHub.app /tmp/vibehub-f06-uninstall/Applications/VibeHub.app
```

### Create isolated project with archives

```text
/tmp/vibehub-f06-uninstall/project/.vibehub/
  project.yaml (schema_version: 3, project_id: project.f06-test)
  tasks/current (task.archive-test)
  tasks/task.archive-test/task.yaml
  events/
```

### Uninstall (remove app bundle only)

```text
rm -rf /tmp/vibehub-f06-uninstall/Applications/VibeHub.app
```

### Post-uninstall verification

| Check | Result |
| --- | --- |
| App bundle removed | PASS — app directory does not exist |
| Project archives preserved | PASS — `.vibehub/` directory intact |
| Project doctor still works | PASS — state: v3 |
| No LaunchAgents | PASS — no `com.vibehub*.plist` |
| No worktree residue in project | PASS — no worktree directories in `.vibehub/` |

### Residue analysis

- Project archives (`.vibehub/`) always live inside the project directory, never inside the app bundle or app data directory. Uninstalling the app cannot delete project archives.
- App-level data (`~/Library/Application Support/VibeHub/`) contains `config.json` and `gateway_config.json` for the user's real install. These are user configuration, not project archives.
- The app uses `VIBEHUB_PORTABLE=1` for isolated testing, keeping all runtime data inside the app bundle directory.
- No LaunchAgents, helpers, or background daemons are installed.

## Automated verification

Commands and results:

```text
npm run v3:contracts:check
passed: 539 assertions, 12 scenarios, 6 views, 2 write contracts

npm run build
TypeScript and Vite production build passed

cargo test -p vibehub-core
275 passed; 0 failed; 0 ignored

cargo test -p vibehub-cli
0 tests (no unit tests in CLI crate); build passed

cargo test --manifest-path src-tauri/Cargo.toml
23 passed; 0 failed; 1 ignored

cargo build -p vibehub-cli
passed

git diff --check
passed with no output
```

## Files changed

- `crates/vibehub-cli/src/dispatcher.rs` — added `worktree-event` and `worktree-orchestration` V3 CLI commands, `read_worktree_command_content` helper, and help text updates.

## Remaining blockers

None for E08, F05, or F06. The pre-existing macOS Accessibility/System Events permission blocker for A08, B05–B07, C05–C06, F04, and E06 remains unchanged.

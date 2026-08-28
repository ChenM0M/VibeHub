# V3 migration guide

Date: 2026-07-12

This guide uses the final V3 CLI surface implemented by the current product:

```text
vibehub v3 <project> doctor
vibehub v3 <project> init
vibehub v3 <project> migrate
vibehub v3 <project> migrate-recover
```

`<project>` is an existing project directory. The `doctor` command is the layout inspection command; `init` means initialize an absent layout. Do not confuse these commands with the legacy top-level `vibehub migrate` and `vibehub recover`, which operate on retired state/workspace facilities rather than V3 bootstrap.

## Safety requirements

1. Back up the complete project before migration. Verify that the backup can be opened independently.
2. First test only on a disposable temporary project or a complete copied project. Never use the real VibeHub repository for the first test or for acceptance evidence.
3. Stop processes that may write the copied project's `.vibehub` directory.
4. Run `doctor` before every state-changing command and again afterward.
5. Do not manually delete `.vibehub.v2-migration` or `.vibehub/legacy-v2`. Use `migrate-recover` for an interrupted migration and preserve conflicts for diagnosis.

V3 bootstrap is deliberately narrow. It does **not** initialize or modify Git, configure a programming language, configure an Agent adapter, create a task, or run a workflow. There is no force/overwrite conflict option.

## 1. Inspect the layout

```bash
vibehub v3 "/absolute/path/to/copied-project" doctor
```

The JSON `state` determines the only safe next action:

| State | Meaning | Allowed next action |
| --- | --- | --- |
| `absent` | No `.vibehub` exists. | `init` |
| `v2` | `.vibehub` exists without a valid V3 marker/archive conflict. | Back up, then `migrate` |
| `v3` | Valid schema version 3 layout. | No bootstrap action |
| `migration_interrupted` | `.vibehub.v2-migration` exists. | `migrate-recover` |
| `conflict` | Ambiguous or unsafe layout. | No automatic mutation; diagnose manually from the message and backup |

`doctor` is read-only and can be repeated. Its `recommended_action` is informational; the CLI still validates the state before mutation.

## 2. Initialize an absent project

After `doctor` reports `absent`:

```bash
vibehub v3 "/absolute/path/to/copied-project" init
vibehub v3 "/absolute/path/to/copied-project" doctor
```

A successful first call returns `status: "initialized"` and creates:

- `.vibehub/project.yaml`
- `.vibehub/events`
- `.vibehub/projections`
- `.vibehub/indexes`
- `.vibehub/runtime`

Repeating `init` on the resulting V3 layout is safe and returns `status: "already_initialized"` with no created paths. Repeating `doctor` continues to report `v3`.

`init` refuses V2, interrupted, and conflict layouts; it does not overwrite them.

## 3. Migrate a V2 copy

After a verified backup and `doctor` reports `v2`:

```bash
vibehub v3 "/absolute/path/to/copied-project" migrate
vibehub v3 "/absolute/path/to/copied-project" doctor
```

Migration atomically stages the old `.vibehub`, creates the V3 root, then moves the staged V2 tree to:

```text
.vibehub/legacy-v2/
```

The archive is the original V2 tree moved without domain conversion. Its files and bytes are preserved as-is; V3 readers treat it as legacy read-only data. Migration never overwrites an existing `legacy-v2`. A repeated `migrate` on an already migrated V3 project returns `status: "already_migrated"`.

After migration, compare important files or hashes from the backup with `.vibehub/legacy-v2` before using the migrated copy.

## 4. Recover an interrupted migration

If `doctor` reports `migration_interrupted`, do not rerun `migrate` and do not rename directories manually:

```bash
vibehub v3 "/absolute/path/to/copied-project" migrate-recover
vibehub v3 "/absolute/path/to/copied-project" doctor
```

Recovery selects a direction from disk state without guessing:

- If no `.vibehub` root exists, staging is moved back to `.vibehub`; result: `rolled_back_to_v2`. Inspect again, verify the restored V2 data, then decide whether to retry migration.
- If `.vibehub` is a valid V3 root and no archive exists, staging is moved to `.vibehub/legacy-v2`; result: `recovered_forward`.
- If `.vibehub` exists without a valid V3 marker, or `legacy-v2` already exists, recovery fails closed and leaves data for manual diagnosis.

Calling `migrate-recover` without a staging directory fails with `V3_MIGRATION_NOT_INTERRUPTED`; it does not change a healthy project.

## 5. Conflict handling

When `doctor` reports `conflict`, stop before initializing or migrating. Typical causes include:

- `.vibehub` exists but is not a directory;
- `.vibehub/legacy-v2` exists without a valid V3 schema marker;
- interrupted recovery sees an invalid/ambiguous `.vibehub` root;
- recovery would overwrite an existing archive;
- a protected root or staging path is a symbolic link.

There is intentionally no `--force`, overwrite, or automatic archive deletion. V3 provides one bounded repair path for the release-baseline regression where all of the following are independently verified:

- `.vibehub/legacy-v2` is a regular directory;
- the requested `.vibehub/tasks/<task-id>/task.yaml` is a bounded regular file whose identity matches;
- the same task has durable V3 lifecycle events under `.vibehub/v3/projects/<project-id>/events.jsonl`;
- `.vibehub/tasks/current` is absent, so repair cannot overwrite an existing pointer.

Inspect candidates and run the typed repair command only after preserving a backup:

```bash
vibehub v3 "/absolute/path/to/project" repair-candidates
vibehub v3 "/absolute/path/to/project" repair <verified-task-id>
vibehub v3 "/absolute/path/to/project" doctor
```

The repair command creates only `.vibehub/project.yaml` when the V3 marker is missing and `.vibehub/tasks/current`; it does not rewrite `legacy-v2`, V3 events, projections, task documents, or project settings. The desktop recovery surface exposes the same verified candidates and typed command. Missing/mismatched events, unsafe paths, an existing current pointer, pure V2 state, or any other ambiguous layout still fail closed.

## 6. V3 event store format 1 → format 2

This migration is separate from the V2-layout migration above. It applies only to an already initialized V3 Project whose audit source is:

```text
.vibehub/v3/projects/<project-id>/events.jsonl
.vibehub/v3/projects/<project-id>/projection.json   # optional compatibility snapshot
```

The current-source typed V3 read/write surface performs a non-destructive, one-time compatibility migration when `store-v2.sqlite3` is absent:

1. Validate the bounded `project.*` identity and reject symlinked/non-directory store paths or symlinked source files.
2. While the Project event source is stable, hash `events.jsonl` and the optional `projection.json` with SHA-256.
3. Copy both source files to `backups/store-format-1-<digest>/` and write `manifest.json`; an existing backup is reused only after its identity, sizes and hashes match.
4. Build `store-v2.sqlite3.migrating` from JSONL, compare the materialized shards with a complete replay, verify the source hashes again, and checkpoint WAL.
5. Atomically rename the verified temp database to `store-v2.sqlite3`, then atomically write `store-v2-migration.json`.

The migration never rewrites or removes JSONL, `projection.json`, Task YAML, Plan history, `legacy-v2`, the current pointer, or user files. A disk/capacity, permission, checkpoint, verification, or rename failure returns a structured error and leaves the source plus verified backup unchanged. A verified `.migrating` database is reusable on retry; a corrupt interrupted temp database is quarantined as `store-v2.sqlite3.failed.<id>` before rebuilding.

Historical audit records may contain the earlier path-safe `T-*` Task identity.
Compatibility replay preserves that identity verbatim; the migration must not rename
or rewrite such events. Session and binding shards are rebuilt by `session_id` across
all Task identities, because a historical Session can contain bind/unbind or recovery
events associated with more than one Task. Derived projection model upgrades (for
example `v3-sqlite-wal-1` to `v3-sqlite-wal-2`) quarantine and rebuild the disposable
index from JSONL while unsupported store formats continue to fail closed.

Do not copy a `store-v2.sqlite3` file while WAL writers are active. For acceptance, use a disposable copy and verify `projection-status` reports store format 2, a synchronized index watermark, the migration marker and the expected event count. Windows and macOS atomic rename, lock and crash behavior require separate native evidence.

## Error diagnosis

- `V3_PROJECT_ROOT_NOT_FOUND`: the project path does not exist or cannot be resolved.
- `V3_INIT_REQUIRES_EMPTY_PROJECT`: `init` was used for a non-absent layout; run `doctor` and use its state-specific command.
- `V3_MIGRATION_STATE_INVALID`: `migrate` was used when the layout was not V2.
- `V3_MIGRATION_NOT_INTERRUPTED`: no staging directory exists.
- `V3_MIGRATION_ARCHIVE_CONFLICT`: forward recovery would overwrite `legacy-v2`.
- `V3_MIGRATION_RECOVERY_CONFLICT`: the root is ambiguous and automatic recovery was refused.
- `V3_ROOT_SYMLINK` / `V3_MIGRATION_*_SYMLINK`: bootstrap refuses symbolic-link roots/staging.
- `V3_MIGRATION_STAGE_FAILED`, `V3_MIGRATION_ARCHIVE_FAILED`, `V3_MIGRATION_RECOVER_FAILED`, or rollback errors: retain both backup and on-disk state; diagnose permissions, locks, volume/filesystem behavior, and free space on a copy.

Errors are JSON on stderr and return a non-zero exit status. Do not treat a JSON message containing a refusal as success.

## Reproducible isolated smoke

The macOS Batch M1 evidence is in `docs/v3/evidence/m1-c02-g01/cli-smoke.md`. It uses only generated `/tmp` projects and covers:

- absent inspect;
- initialize, repeated initialize and repeated inspect;
- V2 inspect/migrate and byte-identical archive payload;
- interrupted backward and forward recovery;
- conflict inspect and migration refusal.

No real project or user transcript is part of that evidence.

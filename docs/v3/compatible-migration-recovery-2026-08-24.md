# N05 compatible migration and recovery evidence

- Task: `task.v3-task-scoped.cb5f33fa90c0`
- PlanNode: `node.v3-store.n05-compatible-migration-recovery`
- Reference host: macOS, debug build, current-source `target/debug/vibehub`
- Scope: store format 1 JSONL/projection compatibility, atomic format 2 migration,
  rollback/retry, corruption recovery, and a disposable copy of the current real
  `project.vibehub` data. No real control-plane source file was manually edited or
  migrated.

## Migration contract

`events.jsonl` remains the audit source. When `store-v2.sqlite3` is absent, the
current-source reader:

1. validates the Project identity and source/target paths without following
   symlinks;
2. writes one content-addressed backup containing the original JSONL,
   compatibility projection, SHA-256 manifest, and store/model versions;
3. builds `store-v2.sqlite3.migrating` under SQLite WAL, then compares every
   materialized projection field with a complete JSONL replay;
4. checkpoints and atomically renames the verified database, then writes
   `store-v2-migration.json`;
5. leaves source JSONL, `projection.json`, Task YAML, Plan history, current pointer,
   `legacy-v2`, and unrelated user files unchanged.

The migration is retryable after injected disk-full or rename failures. Corrupt
interrupted temp files and recoverably corrupt final indexes are quarantined before
rebuild. Unsupported store formats, symlink/directory targets, unsafe Project IDs,
and permission failures remain fail closed. Format 2 projection model
`v3-sqlite-wal-2` refreshes Session and binding shards by `session_id`, so historical
Sessions that span Task identities remain equivalent to global replay; an older
derived model is rebuilt from JSONL rather than served silently.

## Automated validation

Command:

```text
cargo test --locked -p vibehub-core v3::event_store -- --nocapture
```

Result: `29 passed; 0 failed`. Coverage includes:

- content-addressed backup, marker, idempotent retry, and byte-preserving source;
- disk-full and atomic-rename fault injection;
- partial tail, lagging index, projection transaction failure, and final-index
  corruption/missing-table recovery;
- symlink sources/targets, read-only directory, unsafe Project ID, unsupported old
  store format, and interrupted corrupt temp quarantine;
- historical `T-*` Task identities without event rewriting;
- cross-Task Session/binding events assembled by global Session identity;
- Task YAML, current pointer, `legacy-v2`, and unrelated user-file sentinels.

The V3 regression with the three sandbox-only sidecar port tests explicitly skipped
passed `251/251`. `cargo check --locked -p vibehub-core`,
`cargo fmt --all -- --check`, and `git diff --check` also passed at this N05
checkpoint. The three network tests are not counted as passed here; the full
workspace gate must rerun them with loopback permission in N06.

## Current real Project disposable-copy exercise

Fixture root:

```text
/private/tmp/vibehub-n05-real-copy-20260823-7f484efb/VibeHub
```

The fixture was copied from the current real `.vibehub` tree. Only the copy's
derived format 2 index/marker/backup was removed before the current-source CLI
triggered migration. Source snapshots were kept outside the copied Project store
for byte comparison.

Observed result:

- JSONL lines: `5,972`; SQLite indexed events: `5,972`;
- store format: `2`; model version: `v3-sqlite-wal-2`;
- compact task-view response: `333,218` bytes after migration;
- backup directory count before and after another read: `1` and `1`;
- migration marker and backup manifest SHA-256 remained stable;
- stable Task/criterion/timeline event facts remained identical across reads;
- source events SHA-256:
  `2bf3ee123d4dbdfae2edd9688ea3446bd6272bb904a0250b7eed097300137b46`;
- source projection SHA-256:
  `e334e3c8c966d94397311b543f6ac0d2594b9bfd5e97457aa1ec1549a193c0d2`;
- `cmp` passed for source JSONL, compatibility projection, target Task YAML,
  current pointer, and both backup copies; recursive `diff` passed for
  `legacy-v2`.

The first deliberately misnamed fixture root resolved to `project.root` and failed
closed without migration. After correcting the disposable root name, real history
exposed two compatibility defects—historical `T-*` identities and cross-Task
Session/binding shards. Both were fixed and retained as regression tests. A failed
verification never replaced the final index or modified the JSONL source.

## Platform boundary

These results validate the implementation and native macOS filesystem path used by
the fixture. They do not validate Windows file locks, replace semantics, crash
recovery, or path/reparse-point behavior. That evidence remains an explicit N06
native-platform gate.

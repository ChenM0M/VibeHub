# N06 performance, native filesystem, and final-gate evidence

- Task: `task.v3-task-scoped.cb5f33fa90c0`
- PlanNode: `node.v3-store.n06-performance-native-final-gates`
- Host: macOS 26.6.2 build 25G83, Darwin 25.6.0, arm64
- Toolchain: rustc 1.98.0 (`aarch64-apple-darwin`), VibeHub 3.3.5
- Build mode for performance samples: debug

This report separates automated/source evidence, macOS-native evidence, and the
missing Windows-native evidence. It is not a release, tag, push, or completion
claim.

## 1k Tasks / 5k and 50k events

Command:

```text
cargo run --locked -p vibehub-core --example v3_store_benchmark -- 5000 50000
```

Raw output: `/private/tmp/vibehub-n06-store-benchmark-20260824.json`.

| Metric | 5k events | 50k events |
| --- | ---: | ---: |
| Tasks | 1,000 | 1,000 |
| JSONL bytes | 2,649,000 | 27,203,000 |
| one-time index build | 868.395 ms | 7,342.013 ms |
| indexed reload | 55.537 ms | 359.994 ms |
| complete replay fold | 41.966 ms | 487.983 ms |
| steady append p50 | 14.500 ms | 18.344 ms |
| steady append p95 | 16.517 ms | 20.470 ms |
| steady append max | 19.282 ms | 36.639 ms |
| compatibility projection rewritten | false | false |

The one-time initial build is recorded separately and is not represented as a
steady-state latency. The 50k steady append p95 is below the required 250 ms.
Increasing the fixture from 5k to 50k does not cause steady writes to reparse the
full JSONL or rewrite `projection.json`; each append refreshes the touched indexed
aggregate/Task/Session shards.

## Current real Project disposable-copy task-view

Fixture:

```text
/private/tmp/vibehub-n05-real-copy-20260823-7f484efb/VibeHub
```

Twenty current-source CLI processes loaded the selected Task and checked both Task
and Project identity on every run:

- min: 133.314 ms;
- p50: 137.838 ms;
- p95/max: 249.148 ms;
- compact response: 333,218 bytes on all runs;
- identity: `task.v3-task-scoped.cb5f33fa90c0` / `project.vibehub`.

This passes the N06 p95 <= 500 ms and response < 1 MiB budgets. The fixture held
5,972 real source events and 5,972 indexed events. It was a disposable copy; the
real `.vibehub` source was not manually migrated.

## Concurrency, failure, and macOS-native filesystem behavior

Environment and command:

```text
sw_vers
uname -a
rustc -Vv
cargo test --locked -p vibehub-core v3::event_store::tests:: -- --nocapture --test-threads=1
```

Result: `29 passed; 0 failed` on native macOS. The tests internally create
disposable Project roots and exercise:

- 24 independent Task writers with a single continuous Project order and replay
  equivalence;
- a 16-way hotspot aggregate: exactly one append plus 15 version conflicts;
- a 16-way identical idempotency request: exactly one append plus 15 duplicates;
- live lock refusal, dead-process lock recovery, and proof that a SQLite writer wait
  does not retain the JSONL sequencing lock;
- partial-tail quarantine, index lag catch-up, and incremental projection transaction
  failure without lost JSONL events;
- disk-full and rename fault rollback, corrupt interrupted temp recovery, corrupt
  final-index recovery, and incomplete/outdated model rebuild;
- symlink, read-only directory, unsafe Project identity, and historical path-safe
  Task identity behavior;
- format 1 backup/migration idempotence and cross-Task Session/binding equivalence.

These executions are macOS-native evidence for the code paths under test. They are
not Windows evidence.

## Diagnostics and protocol surfaces

`projection_status` reports and the MCP contract now assert:

- `store_format` and `store_model_version`;
- `index_state`, `event_count`, and `projection_event_count`;
- `last_global_seq`, `last_incremental_source_offset`, and `source_file_bytes`;
- `last_event_timestamp`, migration/backup status, and executable-or-null
  `repair_action`.

CLI and MCP call the same application service; Tauri production loaders and the
bounded Task/Project contracts were compiled by the workspace/frontend gates. New
projection/index error codes map to the existing `projectionUnavailable` diagnostic
in Simplified Chinese, Traditional Chinese, and English. ADR-006 and the migration
guide document the format, global Session identity, recovery, and platform boundary.

## Fixed final gates

| Gate | Observed result |
| --- | --- |
| `npm run release:check` | passed, version 3.3.5 |
| `npm run v3:contracts:check` | passed, 818 assertions / 12 scenarios / 6 views / 5 write contracts |
| `npm run v3:mcp:check` | passed, protocol 2025-11-25 / 31 tools / 7 resources |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo build --locked -p vibehub-cli --bin vibehub` | exit 0 |
| `cargo test --locked --workspace` | exit 0 outside sandbox; Tauri 94 passed/3 ignored, adapters 10 passed, core 461 passed, doc tests 0 failed |
| `npm run build` | exit 0, production build completed |
| `git diff --check` | exit 0 |

The three ignored Tauri tests are explicitly marked real-user-data smoke tests that
require opt-in environment variables. The previously sandbox-blocked protocol
sidecar tests were not ignored: all three passed in the permitted loopback workspace
run.

## Windows native blocker and criterion disposition

No trusted Windows 10/11 native GUI/runtime is available in the current execution
environment. Therefore Windows file locking, atomic replacement, crash recovery,
path/reparse-point behavior, and exact current-source/artifact operation remain
unverified. macOS and CI/source tests cannot substitute for those facts.

- C06: `blocked` despite all performance, concurrency, automation, and macOS-native
  portions passing. Repair requires running the disposable fixtures on a trusted
  Windows 10/11 host and recording the exact binary/artifact, hash, Windows version,
  filesystem/path setup, and lock/rename/crash results.
- C07: eligible for `passed` based on the synchronized diagnostics/contracts/MCP/
  CLI/Tauri/i18n/docs and the fixed gate results above.
- Task completion proposal: not allowed while C06 remains blocked.

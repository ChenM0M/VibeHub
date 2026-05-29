# VibeHub Legacy Structure Archive

Date: 2026-05-29

## Purpose

This archive records the VibeHub v2.0 structural migration from the old Tauri-local backend layout to the new shared core crate layout.

The archived legacy structure is informational. The active implementation now lives in `crates/vibehub-core`, and the Tauri app consumes it through a narrow re-export shim.

## Current Boundary

- Active core implementation: `crates/vibehub-core/src/vibehub/`
- Active CLI implementation: `crates/vibehub-cli/src/main.rs`
- Active Tauri bridge: `src-tauri/src/commands.rs`
- Tauri compatibility shim: `src-tauri/src/vibehub/mod.rs`
- Retired implementation location: `src-tauri/src/vibehub/*.rs`

`src-tauri/src/vibehub/mod.rs` intentionally contains only:

```rust
pub use vibehub_core::vibehub::*;
```

That line is the compatibility boundary for older Tauri imports. New VibeHub backend logic should not be added under `src-tauri/src/vibehub/`.

## Legacy Module Mapping

| Legacy Tauri module | Active core module | Status |
| --- | --- | --- |
| `src-tauri/src/vibehub/agent_adapter.rs` | `crates/vibehub-core/src/vibehub/agent_adapter.rs` | migrated |
| `src-tauri/src/vibehub/agent_view.rs` | `crates/vibehub-core/src/vibehub/agent_view.rs` | migrated |
| `src-tauri/src/vibehub/branches.rs` | `crates/vibehub-core/src/vibehub/branches.rs` | migrated |
| `src-tauri/src/vibehub/cockpit.rs` | `crates/vibehub-core/src/vibehub/cockpit.rs` | migrated |
| `src-tauri/src/vibehub/context.rs` | `crates/vibehub-core/src/vibehub/context.rs` | migrated |
| `src-tauri/src/vibehub/current.rs` | `crates/vibehub-core/src/vibehub/current.rs` | migrated |
| `src-tauri/src/vibehub/drift.rs` | `crates/vibehub-core/src/vibehub/drift.rs` | migrated |
| `src-tauri/src/vibehub/events.rs` | `crates/vibehub-core/src/vibehub/events.rs` | migrated |
| `src-tauri/src/vibehub/handoff.rs` | `crates/vibehub-core/src/vibehub/handoff.rs` | migrated |
| `src-tauri/src/vibehub/init.rs` | `crates/vibehub-core/src/vibehub/init.rs` | migrated |
| `src-tauri/src/vibehub/journal.rs` | `crates/vibehub-core/src/vibehub/journal.rs` | migrated |
| `src-tauri/src/vibehub/knowledge.rs` | `crates/vibehub-core/src/vibehub/knowledge.rs` | migrated |
| `src-tauri/src/vibehub/locale.rs` | `crates/vibehub-core/src/vibehub/locale.rs` | migrated |
| `src-tauri/src/vibehub/notes.rs` | `crates/vibehub-core/src/vibehub/notes.rs` | migrated |
| `src-tauri/src/vibehub/overview.rs` | `crates/vibehub-core/src/vibehub/overview.rs` | migrated |
| `src-tauri/src/vibehub/phase.rs` | `crates/vibehub-core/src/vibehub/phase.rs` | migrated |
| `src-tauri/src/vibehub/research.rs` | `crates/vibehub-core/src/vibehub/research.rs` | migrated |
| `src-tauri/src/vibehub/review.rs` | `crates/vibehub-core/src/vibehub/review.rs` | migrated |
| `src-tauri/src/vibehub/start_task.rs` | `crates/vibehub-core/src/vibehub/start_task.rs` | migrated |
| `src-tauri/src/vibehub/state_migration.rs` | `crates/vibehub-core/src/vibehub/state_migration.rs` | migrated |
| `src-tauri/src/vibehub/status.rs` | `crates/vibehub-core/src/vibehub/status.rs` | migrated |
| `src-tauri/src/vibehub/sync.rs` | `crates/vibehub-core/src/vibehub/sync.rs` | migrated |
| `src-tauri/src/vibehub/util.rs` | `crates/vibehub-core/src/vibehub/util.rs` | migrated |

## New Core-Only Modules

These modules were added during the v2.0 capability work and do not have a legacy Tauri-local equivalent:

- `archive.rs`
- `capability.rs`
- `debug_dump.rs`
- `fitness.rs`
- `neighbors.rs`
- `ownership.rs`
- `policy.rs`
- `project_structure.rs`
- `projection.rs`
- `prompts.rs`
- `schema_check.rs`
- `task_switch.rs`
- `workflow.rs`

## Why The Legacy Files Stay Deleted

Keeping duplicate Rust implementations under both `src-tauri/src/vibehub/` and `crates/vibehub-core/src/vibehub/` would create two backend surfaces with the same names but different ownership. The v2.0 boundary is:

- shared logic in `vibehub-core`
- command-line access in `vibehub-cli`
- desktop app access through Tauri commands
- no feature logic under the old Tauri-local module tree

## Historical Source Retrieval

Before the migration commit is finalized, the old implementation is still available from the pre-migration Git tree. After the migration is committed, retrieve a legacy file from the parent of the migration commit:

```bash
git show <migration_parent>:src-tauri/src/vibehub/<module>.rs
```

For the current working branch before commit, the old tree is visible from `HEAD`:

```bash
git show HEAD:src-tauri/src/vibehub/status.rs
```

## Release Checklist Impact

- `cargo test --all --offline` must pass from the workspace root.
- `cargo check --locked` must pass from `src-tauri`.
- `npm run build` must pass.
- `npm run tauri -- build --target aarch64-apple-darwin --bundles app` must pass before macOS release workflow changes or tags.
- Adapter projection warnings should be resolved before final `v2.0.0`.

## Do Not Reopen

Do not add new implementation files under `src-tauri/src/vibehub/` unless the migration is intentionally reverted. If a Tauri command needs new VibeHub behavior, add the implementation to `crates/vibehub-core/src/vibehub/` and expose it through `src-tauri/src/commands.rs`.

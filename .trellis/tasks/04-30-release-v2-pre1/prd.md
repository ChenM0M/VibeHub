# Release 2.0.0-pre.1

## Goal

Prepare a coherent pre-release from the currently completed VibeHub v2 work by aligning version metadata to `2.0.0-pre.1`, preserving the already-finished product changes in the working tree, and validating that the app still builds cleanly.

## What I already know

* The user confirmed that all release-facing version strings should use `2.0.0-pre.1`.
* Current version metadata is still `1.3.5` in `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml`.
* `release.ps1` currently assumes the tag is `v$Version`, stages everything with `git add .`, commits, tags, and pushes.
* Product-facing working tree changes currently include:
* `src/components/ProjectCard.tsx` click-handling fix for cockpit navigation.
* `src-tauri/src/updater.rs` formatting-only cleanup.
* There are many unrelated `.trellis/**` untracked or modified files that should not be swept into a product release accidentally.

## Requirements

* Update repo version metadata from `1.3.5` to `2.0.0-pre.1`.
* Ensure release tooling accepts and produces a `2.0.0-pre.1` release/tag flow without requiring a separate `v2-pre1` alias.
* Keep the current product code changes that are already completed and suitable for this pre-release.
* Avoid broad release automation that blindly stages unrelated `.trellis/**` files.
* Verify the repo still builds after the version updates and release-prep changes.

## Acceptance Criteria

* [ ] `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml` all report `2.0.0-pre.1`.
* [ ] Release script behavior is consistent with the new version/tag naming and does not silently include unrelated files by default.
* [ ] The current product code changes remain intact.
* [ ] Frontend build passes.
* [ ] Relevant Rust validation passes, or any blocker is documented clearly.

## Definition of Done

* Required files are updated for `2.0.0-pre.1`.
* Build/type-check validation has been run.
* Risks or manual follow-up for the actual git tag/push release step are explicit.

## Technical Approach

Adjust version metadata directly in the tracked release files, harden `release.ps1` so it is safer around dirty working trees and still tags the chosen semver version, then run the existing build checks that are feasible in this workspace.

## Decision (ADR-lite)

**Context**: The original user request used the human label `v2-pre1`, but Tauri and Cargo require standard semver for the actual app version.

**Decision**: Use `2.0.0-pre.1` uniformly for repo version metadata and release tagging in this pre-release.

**Consequences**: The release is standards-compliant across npm/Tauri/Cargo and update/version parsing remains simpler. Any marketing-style alias such as `v2-pre1` is intentionally dropped from the technical release flow.

## Out of Scope

* Pushing tags or publishing to a remote release target without an explicit commit plan/confirmation.
* Sweeping unrelated `.trellis/**` bootstrap/spec files into the product release by default.
* New feature work beyond version alignment and release preparation.

## Technical Notes

* Relevant version files: `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `release.ps1`
* Relevant product files already changed: `src/components/ProjectCard.tsx`, `src-tauri/src/updater.rs`
* Release preparation must respect the dirty worktree and keep unrelated files out unless explicitly included later.

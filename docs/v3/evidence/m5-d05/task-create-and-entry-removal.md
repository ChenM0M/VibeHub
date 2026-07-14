# Batch M5 — D05 capability closure and legacy entry removal

Date: 2026-07-13
Platform: macOS arm64
Branch: `feature/vibehub-v2-p0`

## Result

D05 is VERIFIED. Production V3 now has a bounded, user-reachable V3-native task creation flow, and the project-card context menu no longer imports, mounts, or opens `VibehubCockpitDialog`.

## Retained capability closure

- V3 task creation: implemented in `vibehub-core` as `create_v3_task`, exposed by the independent `v3_create_task` Tauri command and TypeScript wrapper, and injected only into the production `V3Cockpit` route.
- Project locale/settings: existing backend/CLI path remains independently retained; no retired adapter or prompt-template behavior was moved into production V3.
- Project file open/reveal: existing bounded production V3 structure callbacks remain unchanged.
- Diagnostics: V3 lifecycle, views, doctor/MCP diagnostics, usage, legacy archive, structure, timeline, plan, and node brief remain available.
- Retired protocol mechanics (sync, phase mutation, adapter generation, prompt templates, run/context/agent-view/output mechanics) were not added to production V3.

## Task-create behavior

The V3 core service:

- requires an inspected, valid schema-version-3 project;
- trims and validates title, intent, and 1–100 acceptance criteria;
- derives a stable safe task ID from normalized title plus a SHA-256 request digest;
- rejects unsafe/symlinked task roots and conflicting existing task paths;
- atomically writes only `.vibehub/tasks/<task_id>/task.yaml`;
- atomically updates the canonical `.vibehub/tasks/current` pointer;
- treats an identical repeat request as `already_exists` and rejects conflicting metadata;
- creates no protocol run, context, agent-view, handoff, review, or output state.

All Rust tests use isolated directories under the operating-system temporary directory.

## User-entry boundary

- Production `Home` injects `tauriApi.v3CreateTask` into `V3Cockpit`.
- The fixture playground does not receive a task-create callback.
- Production V3 presents a create-task button in the normal header and an empty-task recovery surface when no current task exists.
- The form covers validation, cancel, backend error, pending, and successful bundle refresh behavior.
- `ProjectCard` still preserves launch, custom launch, Explorer, Terminal, copy path, star, edit, delete, and ordinary card-body navigation to production V3.
- `ProjectCard` contains no `VibehubCockpitDialog`, `isCockpitOpen`, `LayoutDashboard`, or `VibeHub Cockpit` reference.

## Automated verification

Commands and results:

```text
cargo test -p vibehub-core task_creation
4 passed; 0 failed

cargo test --manifest-path src-tauri/Cargo.toml commands::tests
4 passed; 0 failed

cargo build -p vibehub-cli
passed

npm run v3:contracts:check
passed: 458 assertions, 12 scenarios, 6 views, 2 write contracts

npm run build
TypeScript and Vite production build passed

git diff --check
passed with no output
```

Residual scans:

```text
rg -n "VibehubCockpitDialog|isCockpitOpen|VibeHub Cockpit|LayoutDashboard" src/components/ProjectCard.tsx
no matches

rg -n "v3CreateTask|createTask|创建任务" src/pages/Home.tsx src/v3/app/V3Cockpit.tsx src/services/tauri.ts
production wrapper, injection, guarded UI entry, validation form, and refresh path found
```

The V3 contract gate now additionally proves:

- fixture playground receives no production file or task mutation callbacks;
- production route injects the bounded task-create callback;
- production cockpit exposes the task-create entry;
- project cards cannot regress to the old Cockpit entry;
- all pre-existing M1 usage/token/cost/scenario/debug surfaces remain present.

## Browser evidence

Browser preview ran against the frontend development server on 2026-07-13.

- Fixture playground loaded `FX-HAPPY` with the existing M1 layout, active-task list, overview/timeline/plan/structure tabs, token/fee surface, usage panel, fixture selector, debug indicator, and settings control.
- Fixture playground displayed no production “创建任务” action.
- Browser console: no errors.
- Development server log: no errors.
- A screenshot was captured in the Browser preview result for this batch.

The browser preview has no real Tauri project configuration and cannot execute native IPC. Production callback injection, empty-task behavior, old-entry removal, and fixture isolation are therefore established by TypeScript build, contract assertions, and source scans rather than misrepresented as native evidence. D05 does not require native shell/IDE interaction evidence.

# VibeHub Core Contracts

## Scenario: P0/P1/P2 Core Cockpit Commands

### 1. Scope / Trigger

- Trigger: VibeHub Core adds cross-layer Tauri commands consumed by the React cockpit center.
- Scope: commands exposed through `src-tauri/src/commands.rs`, TypeScript wrappers in `src/services/tauri.ts`, DTOs in `src/types/index.ts`, and UI actions in `VibehubCockpitDialog`.
- Boundary: These are P0/P1/P2 best-effort file protocol commands. They must not claim runtime observation, automatic rollback, auto-commit, or multi-agent orchestration.

### 2. Signatures

- `vibehub_init(project_path: String) -> VibehubInitResult`
- `vibehub_start_task(project_path: String, title: Option<String>, mode: Option<String>, phase: Option<String>) -> VibehubStartTaskResult`
- `vibehub_build_context_pack(project_path: String, task_id: String, run_id: String, phase: String) -> ContextPackBuildResult`
- `vibehub_generate_agent_view(project_path: String) -> AgentViewGenerateResult`
- `vibehub_sync_agent_adapter(project_path: String, dry_run: Option<bool>) -> AgentAdapterSyncResult`
- `vibehub_build_handoff(project_path: String) -> HandoffBuildResult`
- `vibehub_generate_review_evidence(project_path: String) -> ReviewEvidenceGenerateResult`
- `vibehub_read_cockpit_status(project_path: String) -> VibehubCockpitStatus`
- `vibehub_append_journal_entry(project_path: String, title: Option<String>, body: Option<String>) -> JournalAppendResult`
- `vibehub_append_knowledge_note(project_path: String, note: Option<String>) -> KnowledgeAppendResult`

### 3. Contracts

- `project_path` must point to a local project directory.
- `start_task` must create a concrete task/run, write YAML current pointers, update `.vibehub/state.yaml`, and seed a context spec for the selected phase.
- `build_context_pack` must use active IDs from cockpit status or a known task/run/phase. UI must not invent IDs.
- `sync_agent_adapter` writes only a managed VibeHub section in root `AGENTS.md`; it must preserve user content outside markers.
- `append_journal_entry` writes append-only manual notes to `.vibehub/journal/index.md`.
- `append_knowledge_note` writes append-only manual reusable notes to `.vibehub/journal/knowledge.md`.
- Cockpit UI labels must map honestly to behavior:
  - `Continue` may generate/update agent view.
  - `Build Handoff/Recover Report` must not imply destructive rollback.
  - `Update AGENTS.md` may update only the managed adapter section.

### 4. Validation & Error Matrix

| Condition | Expected behavior |
|---|---|
| `project_path` does not exist | Command returns an error string to UI. |
| Project is not initialized | Journal/knowledge actions reject; UI disables these actions until initialized. |
| Partial `.vibehub` exists | `start_task` must run idempotent init before seeding task/run files. |
| Missing active task/run/phase | Build Context, Continue, Review, and Handoff are disabled with a clear reason. |
| `AGENTS.md` has no VibeHub markers | Adapter sync appends a managed section and preserves existing content. |
| `AGENTS.md` has malformed/duplicate VibeHub markers | Adapter sync reports conflict and does not write. |
| `dry_run` adapter sync | Report says `would create/update`; no files are written. |
| Empty knowledge note | Reject in backend and disable UI submit. |
| Empty journal note | Allowed as a manual session note with safe defaults. |

### 5. Good/Base/Bad Cases

- Good: user opens project center, initializes, starts task, builds context, continues, reviews evidence, builds handoff, updates `AGENTS.md`, appends manual journal and knowledge notes.
- Base: uninitialized project shows Initialize and disables downstream actions with a concrete reason.
- Bad: UI calls Build Context without a task/run/phase; command may fail, but the UI should prevent this path.
- Bad: adapter sync silently overwrites user content outside managed markers. This is forbidden.

### 6. Tests Required

- Rust golden path: `init -> start_task -> build_context_pack -> generate_agent_view -> generate_review_evidence -> build_handoff`.
- Rust adapter tests: create, append to unmanaged `AGENTS.md`, update managed region, malformed marker conflict, duplicate marker conflict, dry-run create/update.
- Rust journal tests: append preserves existing content, missing journal file is created, uninitialized project rejects.
- Rust knowledge tests: append preserves existing content, uninitialized project rejects, empty note rejects.
- Frontend build/type-check: `npm.cmd run -s build`.
- Cross-layer check: every new Tauri command must have a TypeScript wrapper and DTO type when consumed by UI.

### 7. Wrong vs Correct

#### Wrong

```tsx
<Button onClick={() => tauriApi.vibehubBuildContextPack(project.path, '', '', '')}>
  Build Context
</Button>
```

This lets the UI invent invalid IDs and shifts validation to a late backend failure.

#### Correct

```tsx
const hasActiveContextTarget = Boolean(
  status?.initialized && status.current_task_id && status.current_run_id && status.current_phase
);

<Button disabled={!hasActiveContextTarget}>
  Build Context
</Button>
```

The cockpit uses the status contract as the source of truth and disables invalid paths before calling the backend.


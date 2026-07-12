# Research Pack

Task: T-20260711062223-6f07315c
Title: "VibeHub V3 contracts, frontend-first delivery, MCP core, and delayed self-hosting"
Generated: 2026-07-11
Status: complete

## Research Questions

1. Does the current repository contain reusable foundations for V3, and which parts must not leak into the V3 contracts?
2. Is the sequence M0 contracts/fixtures -> M1 high-fidelity frontend -> M2 real core/MCP technically sound?
3. Which MCP primitives and host differences must the M0/M2 contracts account for?
4. Which Windows/macOS and worktree constraints must become executable fixtures or tests?
5. What evidence gate should delay V3 self-hosting until M4 is stable?

## Executive Finding

The requested sequence should proceed unchanged. The repository has useful V2 foundations, but no stable V3 read-model boundary: event append/projection, research artifacts, project structure scanning, Tauri commands, and IDE/file reveal are reusable inputs; V2 YAML projections, capability gates, the monolithic cockpit component, and UI-side canonical writes are not V3 contracts.

M0 must therefore freeze a small read-model surface and executable fixtures before either UI or core is rewritten. M1 should consume only those fixtures and exercise the complete Project/Task information architecture. M2 should then implement the Application Service, event store, derived views, stdio MCP adapter, and CLI fallback behind the already-reviewed contracts. V3 self-hosting should begin only after M4 passes a repeatable stability gate; M5 is the first milestone managed by V3 itself.

## Findings

### F001 - The repository has foundations, not a V3 domain

`hard_observed`: `crates/vibehub-core/src/vibehub/events.rs` already provides append-only JSONL envelopes, recovery, event IDs, and an index; `projection.rs` folds events into current workflow state. The envelope lacks the V3 session/node/commit/idempotency/version fields, and the event enum encodes the V2 capability workflow. This is a migration foundation, not a contract to expose to M1.

`hard_observed`: `project_structure.rs` and `ProjectStructureExplorer.tsx` provide shallow filesystem facts and reveal/open behavior. The current scanner truncates depth/count and does not model architecture evidence, lifecycle, pagination, confidence, or versioning. M3 should extend this behind `ProjectStructureView`, not make the existing Rust shape public.

`hard_observed`: the current frontend has no test script in `package.json`; `VibehubCockpitDialog.tsx` is 4,268 lines. High-fidelity V3 work needs a separate fixture adapter and isolated V3 component boundary to avoid coupling the new IA to this component.

### F002 - Contract-first plus fixture-first is the lowest-risk boundary

`hard_observed`: there is currently no repository-owned V3 JSON Schema, shared view-model package, or fixture suite. JSON Schema 2020-12 formally describes JSON instance structure and is also the schema dialect used by the official Rust MCP SDK's schema-generation stack.

`inferred`: five versioned read models (`ProjectOverviewView`, `ProjectStructureView`, `TaskTimelineView`, `PlanGraphView`, `NodeBrief`) are sufficient to let M1 validate information architecture without inventing V2-shaped backend behavior. Common metadata must include `schema_version`, `generated_at`, freshness/completeness, evidence references, and explicit unavailable/error states.

`inferred`: fixtures are acceptance inputs, not screenshots or informal examples. Each fixture must validate against the schema, be deterministic, and identify the criterion it exercises. Large-repository data should be generated deterministically to avoid checking in an opaque giant JSON blob.

### F003 - M1 should be a complete experience but an intentionally fake transport

`user_confirmed`: M1 is high-fidelity frontend and M2 is the first real core/MCP connection.

`hard_observed`: the current Tauri service layer and cockpit invoke many V2 commands directly. A V3 fixture repository interface prevents M1 from importing V2 YAML/Tauri response shapes while preserving a later swap to a real repository adapter.

`inferred`: M1 must cover the complete primary workflow and all data states, but its mutations are scripted prototype transitions. It must not append canonical V3 events, start a real MCP server, or claim backend completeness.

### F004 - MCP requires an adapter and compatibility matrix, not a second domain

`hard_observed`: MCP defines a JSON-RPC data layer with lifecycle/capability negotiation and distinct tools, resources, prompts, notifications, elicitation, and logging primitives. Local stdio servers usually serve one client; Streamable HTTP commonly serves many.

`hard_observed`: Codex supports local stdio and Streamable HTTP, shared MCP configuration across desktop/CLI/IDE, server instructions, tool allow/deny lists, and approval modes. Claude Code supports stdio and HTTP but adds host-specific project roots, trust/approval behavior, and other extensions. OpenCode supports local/remote MCP configuration and per-agent tool enablement. Host parity cannot be assumed.

`hard_observed`: the official Rust MCP SDK is Tier 2 as of 2026-07-11, while its current README documents Tokio, stdio server operation, tools/resources/prompts, notifications, and JSON Schema 2020-12 generation.

`inferred`: M2 should expose read-heavy state as resources and state transitions as narrow typed tools. The MCP adapter and CLI adapter must call one Application Service. Elicitation, dynamic discovery, annotations, and roots are compatibility-matrix capabilities, not unconditional requirements for the minimal loop.

### F005 - Cross-platform constraints belong in M0 fixtures and M2 tests

`hard_observed`: Windows paths remain subject to `MAX_PATH` in many contexts unless both OS policy and the application manifest opt into long paths. Extended paths and UNC have distinct forms; relative paths remain bounded. A path valid to a low-level API may still fail in shell/UI tooling.

`hard_observed`: the repository CI builds on macOS, Ubuntu, and Windows, but the current tests do not establish V3 stdio MCP, cross-process locking, long-path fixture rendering, or worktree recovery.

`inferred`: contract fields must carry display path and stable machine identity separately. Fixtures must include drive-letter paths, UNC paths, mixed separators as invalid input, very long components, case collisions, inaccessible paths, and stale/missing worktrees. Platform claims remain unverified until native tests run.

### F006 - Worktree isolation is necessary but insufficient

`hard_observed`: Git worktree creates linked working trees with separate per-worktree `HEAD` and index while sharing repository objects/refs. Git also exposes lock, prune, repair, remove, porcelain output, and explicit failure behavior for dirty or already checked-out branches.

`inferred`: M5 requires an orchestration state machine around Git: scope overlap, base/branch identity, lease, agent/session owner, dirty state, integration queue, conflict ownership, repair, and cleanup. A raw `git worktree add` action cannot meet the milestone.

### F007 - Self-hosting before M4 would invalidate the evidence

`user_confirmed`: V3 should manage M5 only after M4 is stable.

`inferred`: M4 stability must be an evidence gate, not a date or subjective declaration. Required evidence is: deterministic rebuild; no duplicate events under idempotent retry; criterion/finding/attempt projections preserve history; kill-resume meets semantic hard gates; the three target MCP hosts complete the minimal recovery loop; macOS and Windows native smoke tests pass; and no unresolved P0/P1 data-loss or false-completion defects remain across a soak window.

`inferred`: M0-M4 remain tracked by V2. After the M4 gate, create the M5 plan/task/session records in V3, mirror only minimal audit references in V2, and compare V3 projection against the known M5 plan. Failure rolls management back to V2 without discarding M5 code work.

## Risks

| ID | Risk | Consequence | Required response |
|---|---|---|---|
| R1 | Contracts mirror V2 storage or component props | M1 bakes migration debt into the UI | Schemas describe user-facing views; add forbidden-dependency checks |
| R2 | M1 prototype silently calls real V2 commands | Prototype behavior is mistaken for V3 capability | One fixture repository interface; no V2 imports/invokes in V3 feature root |
| R3 | Rust MCP Tier 2 changes or host behavior diverges | M2 integration churn | Pin SDK, isolate adapter, Inspector plus three-host contract tests |
| R4 | In-process mutex is mistaken for cross-process locking | Event corruption with CLI/MCP/desktop concurrency | Cross-process lock spike and native crash/recovery tests before M2 exit |
| R5 | Windows paths are normalized destructively | Wrong file identity or failed launch/worktree | Preserve native path identity; explicit path fixtures and native tests |
| R6 | Large fixtures become unreviewable snapshots | Schema drift and false confidence | Small named fixtures plus deterministic large-data generator |
| R7 | V3 self-hosting starts on feature completeness alone | Lost task state masks product defects | M4 evidence gate, shadow run, explicit rollback |
| R8 | Eight active tasks create scope confusion in V2 | Validation or output is written to the wrong task | Always status/switch; use task-scoped validation |

## Open Questions

These questions are intentionally deferred to the named RFC or milestone; none blocks starting M0.

1. Which JSON Schema-to-Rust/TypeScript generation strategy becomes canonical? Resolve in RFC-001 with a small round-trip spike.
2. Does M2 need an external locking crate or a platform-specific lock adapter? Resolve with macOS/Windows cross-process tests in RFC-001.
3. Which MCP optional capabilities are reliable in each host version? Populate the compatibility matrix in RFC-002 during M2.
4. Which language/build-system analyzers enter M3 first? Decide from VibeHub plus two sample repositories in RFC-003.
5. What numeric budgets define `large-data` and Project Intelligence freshness? Benchmark in M0/M3 before freezing thresholds.
6. Where should Windows worktrees live by default, and what path budget is safe? Test on a native Windows machine in RFC-005.
7. What soak duration and pass count qualify M4 as stable? Set before M4 implementation, then freeze before self-host approval.

## Recommendation

- **Recommendation**: proceed to planning and start M0 only after the M0 Task Pack is validated.
- **Confidence**: high for sequencing and boundaries; medium for platform and host-specific implementation details pending native prototypes.
- **Rationale**: local code evidence and official protocol/platform sources converge on a small stable contract boundary, explicit fixtures, adapter isolation, and delayed self-hosting. No evidence supports merging M0-M6 or connecting M1 directly to V2/core.

## Source Index

See `source-log.yaml` for the complete source register and `findings.yaml` for machine-readable findings.

# Context Pack: Align

Task: T-20260529145443-52bb4f48
Run: R-20260529145443-224267fe
Phase: Align
Generated at: 2026-05-29T14:54:43Z
Source commit: 3252e1f

## Instructions

Use this context only for the current phase.
Do not mark state.yaml completed.
Report files read, commands run, decisions made, and unresolved risks.

## Capability Output Schema

```json
{
  "required_fields": [
    "intent",
    "scope",
    "success_criteria",
    "non_goals"
  ],
  "optional_fields": [
    "stakeholders",
    "references"
  ],
  "produces": [
    "alignment_summary"
  ],
  "consumes": [],
  "parallel_safe": false,
  "custom": false
}
```

## Prior Outputs Summary

```json
[
  {
    "capability": "review",
    "completed": [
      "`user_confirmed`: User requested closing out current VibeHub state, clearing the VibeHub phase, archiving the old structure, and treating this as a new task.",
      "`hard_observed`: Prior task `T-20260513154224-58d8a685` research phase validation passed with required outputs `source_log`, `findings`, and `research_pack`.",
      "`hard_observed`: Ran VibeHub CLI `finish`, rebuilt handoff, and ran `advance`; old task moved out of `research active` and VibeHub status later reported `current_phase=plan`, `phase_status=active`.",
      "`hard_observed`: Created new VibeHub task `T-20260529090147-9df98d71` / run `R-20260529090147-65de009c` with title `收口 v2.0 发布前状态并归档旧 VibeHub 结构`.",
      "`hard_observed`: Added `docs/archive/vibehub-legacy-structure-archive-2026-05-29.md`.",
      "`hard_observed`: Archive document maps each retired `src-tauri/src/vibehub/*.rs` module to its active `crates/vibehub-core/src/vibehub/*.rs` replacement and records the Tauri shim boundary.",
      "`hard_observed`: New task align output validated successfully, was finished, and was advanced through the optional research checkpoint.",
      "`hard_observed`: Current VibeHub status reports new task `T-20260529090147-9df98d71` at `plan active`, with `align` and `research` completed.",
      "`hard_observed`: New task was advanced through `plan` and `implement`; current status reached `review active`.",
      "`hard_observed`: Ran VibeHub adapter sync after cleanup; adapter sync reported `created 0, updated 0, skipped 117, conflicts 0`.",
      "`hard_observed`: Ran VibeHub adapter status after sync; `warnings` was empty and all generated adapter files were in sync.",
      "`hard_observed`: Removed stale local build/debug artifacts from the workspace: `dist`, `target`, `src-tauri/target`, `.vibehub/debug-dumps`, and `.vibehub/state.yaml.bak.r2`.",
      "`hard_observed`: Current VibeHub sync rebuilt `.vibehub/agent-view/current.md` and review context pack for task `T-20260529090147-9df98d71`."
    ],
    "full_ref": ".vibehub/tasks/T-20260529090147-9df98d71/runs/R-20260529090147-65de009c/outputs/output.md",
    "key_decisions": [
      "`inferred`: The old structure should be archived as a documented migration map, not as duplicated live Rust files under `src-tauri/src/vibehub/`, because duplicate implementations would create two backend ownership surfaces.",
      "`inferred`: Adapter projection should now be treated as current; the remaining release gate is disciplined review/staging of the broad v2.0 dirty worktree."
    ]
  }
]
```

## Neighbors

```json
[
  {
    "task_id": "T-20260529090147-9df98d71",
    "title": "收口 v2.0 发布前状态并归档旧 VibeHub 结构",
    "active_capabilities": [],
    "shared_files": []
  }
]
```

## File: .vibehub/tasks/T-20260529145443-52bb4f48/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: "T-20260529145443-52bb4f48"
title: "Fix AI Agent Skill/Prompt Quality Issues"
mode: "evidence_drive"
phase: "align"
phase_status: "active"
created_at: "2026-05-29T14:54:43Z"
created_by: vibehub
```

## File: .vibehub/tasks/T-20260529145443-52bb4f48/runs/R-20260529145443-224267fe/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: "T-20260529145443-52bb4f48"
run_id: "R-20260529145443-224267fe"
mode: "evidence_drive"
phase: "align"
phase_status: "active"
created_at: "2026-05-29T14:54:43Z"
created_by: vibehub
baseline_commit: null
```

## File: .vibehub/rules/hard-rules.md

Reason: protocol hard rules

```text
# VibeHub Hard Rules

- Agent output is reported state only.
- Only VibeHub code updates canonical state transitions.
- Do not mark state.yaml completed from agent output.
- Distinguish hard_observed, agent_reported, inferred, and user_confirmed evidence.
- P0/P1 observability is best-effort and must not claim full runtime observation.
- Agents should read agent-view files and the current context pack, not the whole .vibehub directory.
- Keep changes scoped to the active task.

## CI/CD 改动纪律 (2026-05-21 从 8 轮返工中总结)

### 1. 本地先跑通再改 CI
- 任何 macOS CI 构建改动，**先在本地 macOS 验证**：
  `npm run tauri -- build --target aarch64-apple-darwin --bundles app`
- 本地能成功 `hdiutil create -fs APFS`，再改 GitHub Actions。
- CI 不是调试器，不要拿它当测试环境用。

### 2. 每次只改一个变量
- CI workflow 单次改动只改一项：构建方式 / 签名方式 / DMG 方式 分开验证。
- 改多个变量时无法定位失败原因，导致穷举试错。

### 3. 签名方案先问"要不要"
- **prerelease / 预发布**: 用 ad-hoc 签名 (`APPLE_SIGNING_IDENTITY="-"`)，不走 notarization。
- **正式发布**: 才需要 Developer ID 证书 + 公证流程。
- 不要默认启用全套 Apple 签名，先确认是否必要。

### 4. 优先用直接命令，少用 Action 封装
- `npm run tauri -- build` 直接 shell 命令 > `tauri-action` GitHub Action。
- 直接命令可以在本地完美复现，action 的传参行为是黑盒。
- 必须用 action 时，先查源码理解其内部命令拼接逻辑。
```

## Known Missing Context

- None declared.

## Stop Condition

Write output.md and return to VibeHub for validation.

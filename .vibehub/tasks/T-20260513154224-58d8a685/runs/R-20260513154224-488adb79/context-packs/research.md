# Context Pack: Research

Task: T-20260513154224-58d8a685
Run: R-20260513154224-488adb79
Phase: Research
Generated at: 2026-05-28T06:43:07Z
Source commit: 678a5e4

## Instructions

Use this context only for the current phase.
Do not mark state.yaml completed.
Report files read, commands run, decisions made, and unresolved risks.

## Prior Outputs Summary

```json
[
  {
    "capability": "research",
    "completed": [
      "`user_confirmed`: User asked to continue M5.",
      "`hard_observed`: Added `src-tauri/src/vibehub/schema_check.rs` with v1 envelope validation and capability-specific required field/type/min-length/enum/pattern checks for `align_lite`, `align`, `research`, `plan`, `implement`, `validate`, `review_lite`, and `review`.",
      "`hard_observed`: Added structured schema errors with `code`, `message`, `hint`, and `details.schema_ref`; invalid capability output writes append `SchemaValidationFailed { target, errors[] }`.",
      "`hard_observed`: Added `src-tauri/src/vibehub/policy.rs` to the module graph, created `.vibehub/policy.yaml`, added policy to init, and consumed policy in capability WIP/open-risk gates, pack oversize events, schema strict mode, loop detection threshold, and subagent timeout metrics.",
      "`hard_observed`: Added `src-tauri/src/vibehub/fitness.rs` and projection integration so `state.yaml.metrics` can contain the nine M5 baseline metrics.",
      "`hard_observed`: Bumped state schema to v5 and migration defaults now add `metrics`.",
      "`hard_observed`: Reworked event log corruption handling to truncate malformed suffixes and append `EventLogCorrupted` to `events.jsonl`.",
      "`hard_observed`: Added event-frequency loop detection that emits `LoopWarning` for rapid claim/release cycles.",
      "`hard_observed`: Added Tauri and CLI schema-check/write surfaces.",
      "`hard_observed`: Updated `docs/vibehub-capability-implementation-steps-2026-05-27.md` to mark M5 complete and record Draft v2.1.",
      "`hard_observed`: Wrote M5 handoff to `.vibehub/tasks/T-20260513154224-58d8a685/runs/R-20260513154224-488adb79/handoffs/M5.json`."
    ],
    "full_ref": ".vibehub/tasks/T-20260513154224-58d8a685/runs/R-20260513154224-488adb79/outputs/output.md",
    "key_decisions": [
      "`agent_reported`: Kept schema validation in Rust instead of adding a JSON Schema dependency, matching existing local parser/validator style.",
      "`agent_reported`: Used v5 for the state metrics schema addition.",
      "`agent_reported`: Event recovery now repairs the canonical log in place instead of writing a `.pending` stub, because M5 requires recovery before/while writing events.",
      "`agent_reported`: `policy.yaml` defaults preserve old behavior: 5 active capabilities, strict schema enabled for new capability JSON writes, pack warning at 12000 tokens, sub-agent timeout 60s."
    ]
  }
]
```

## File: .vibehub/tasks/T-20260513154224-58d8a685/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: "T-20260513154224-58d8a685"
title: "Close r10 implementation gaps"
mode: "evidence_drive"
phase: "research"
phase_status: "active"
created_at: "2026-05-13T15:42:24Z"
created_by: vibehub
```

## File: .vibehub/tasks/T-20260513154224-58d8a685/runs/R-20260513154224-488adb79/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: "T-20260513154224-58d8a685"
run_id: "R-20260513154224-488adb79"
mode: "evidence_drive"
phase: "research"
phase_status: "active"
created_at: "2026-05-13T15:42:24Z"
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

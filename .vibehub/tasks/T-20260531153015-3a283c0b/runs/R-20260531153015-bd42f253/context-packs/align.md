# Context Pack: Align

Task: T-20260531153015-3a283c0b
Run: R-20260531153015-bd42f253
Phase: Align
Generated at: 2026-05-31T15:57:35Z
Source commit: b6abdf3

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
    "capability": "plan",
    "completed": [
      "`user_confirmed`: 用户确认创建新任务，继续做 Agent Operating Loop 和工具编排重构；允许一次性推进到结束前，但不要 archive，也不要静默归档。",
      "`hard_observed`: 已创建任务 `T-20260531155016-81ab5ec4`，运行 `R-20260531155016-74716caf`，模式 `guided_drive`，当前 phase 为 `align` / `active`。",
      "`hard_observed`: 当前另有 UI task `T-20260531153015-3a283c0b` active；本任务范围不包含 UI 重做。",
      "`inferred`: 本任务目标是把 VibeHub Agent 协议从“多处提示和工具说明”收敛为可执行的 Operating Loop、机器可读 routing/next-action 决策，以及更轻的 skill/adapter 入口。"
    ],
    "full_ref": ".vibehub/tasks/T-20260531155016-81ab5ec4/runs/R-20260531155016-74716caf/outputs/output.md",
    "key_decisions": [
      "`inferred`: 本轮不再扩大长文档，而是补一层小而硬的决策模型：Operating Loop + routing table + CLI next-action。",
      "`inferred`: `finish` / `advance` 可按用户本轮授权推进，但最终不 archive；最终响应需明确完成状态、体验改善、覆盖范围和剩余风险。"
    ]
  }
]
```

## Neighbors

```json
[
  {
    "task_id": "T-20260531155016-81ab5ec4",
    "title": "Refine VibeHub agent operating loop and tool orchestration",
    "active_capabilities": [
      "plan"
    ],
    "shared_files": []
  }
]
```

## File: .vibehub/tasks/T-20260531153015-3a283c0b/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: "T-20260531153015-3a283c0b"
title: "Redesign VibeHub project detail UI and project structure explorer"
mode: "guided_drive"
phase: "align"
phase_status: "active"
created_at: "2026-05-31T15:30:15Z"
created_by: vibehub
```

## File: .vibehub/tasks/T-20260531153015-3a283c0b/runs/R-20260531153015-bd42f253/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: "T-20260531153015-3a283c0b"
run_id: "R-20260531153015-bd42f253"
mode: "guided_drive"
phase: "align"
phase_status: "active"
created_at: "2026-05-31T15:30:15Z"
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

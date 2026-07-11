# Context Pack: Align

Task: T-20260711080143-fbe94685
Run: R-20260711080143-1866a605
Phase: Align
Generated at: 2026-07-11T08:01:43Z
Source commit: d7ece57

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

- None available.

## Neighbors

```json
[
  {
    "task_id": "T-20260711062223-6f07315c",
    "title": "完善 VibeHub V3 重设计方案与 Agent 集成架构",
    "active_capabilities": [
      "research"
    ],
    "shared_files": []
  },
  {
    "task_id": "T-20260711080143-49b5012c",
    "title": "M1 基于 fixtures 构建 V3 高保真前端体验",
    "active_capabilities": [
      "align"
    ],
    "shared_files": []
  },
  {
    "task_id": "T-20260711080143-b1ff21ea",
    "title": "M2 实现 V3 事件核心与 MCP 控制面",
    "active_capabilities": [
      "align"
    ],
    "shared_files": []
  },
  {
    "task_id": "T-20260711080143-c3669d9d",
    "title": "M0 冻结 V3 契约、fixtures 与实施基线",
    "active_capabilities": [
      "align"
    ],
    "shared_files": []
  },
  {
    "task_id": "T-20260711080143-e975f3c8",
    "title": "M3 构建 Project Intelligence 与架构地图",
    "active_capabilities": [
      "align"
    ],
    "shared_files": []
  }
]
```

## File: .vibehub/tasks/T-20260711080143-fbe94685/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: "T-20260711080143-fbe94685"
title: "M4 完成 Task 计划图、时间线与验收闭环"
mode: "guided_drive"
phase: "align"
phase_status: "active"
created_at: "2026-07-11T08:01:43Z"
created_by: vibehub
```

## File: .vibehub/tasks/T-20260711080143-fbe94685/runs/R-20260711080143-1866a605/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: "T-20260711080143-fbe94685"
run_id: "R-20260711080143-1866a605"
mode: "guided_drive"
phase: "align"
phase_status: "active"
created_at: "2026-07-11T08:01:43Z"
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

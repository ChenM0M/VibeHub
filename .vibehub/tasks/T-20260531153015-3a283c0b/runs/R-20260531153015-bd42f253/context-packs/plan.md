# Context Pack: Plan

Task: T-20260531153015-3a283c0b
Run: R-20260531153015-bd42f253
Phase: Plan
Generated at: 2026-05-31T15:57:53Z
Source commit: b6abdf3

## Instructions

Use this context only for the current phase.
Do not mark state.yaml completed.
Report files read, commands run, decisions made, and unresolved risks.

## Capability Output Schema

```json
{
  "required_fields": [
    "steps",
    "validation_plan",
    "affected_files"
  ],
  "optional_fields": [
    "risks",
    "references"
  ],
  "produces": [
    "implementation_plan"
  ],
  "consumes": [
    "alignment_summary",
    "research_output"
  ],
  "parallel_safe": true,
  "custom": false
}
```

## Prior Outputs Summary

```json
[
  {
    "capability": "align",
    "completed": [
      "`user_confirmed`: 用户确认了设计意图：",
      "美学采用现代简约（类似 Linear 风格），无需过多边框/区块，避免过度 AI 感的布局。",
      "将庞大的 `VibehubCockpitDialog` 拆分成多个子组件以提高可维护性。",
      "项目资源管理器类似于树状图/节点图脑图，支持鼠标拖拽浏览，主要作为美观的阅读器。"
    ],
    "full_ref": ".vibehub/tasks/T-20260531153015-3a283c0b/runs/R-20260531153015-bd42f253/outputs/output.md",
    "key_decisions": [
      "`user_confirmed`: 对老旧的大组件进行彻底拆分。",
      "`user_confirmed`: 使用 Tailwind 实现轻量化、无过多边框的简约视觉风格。"
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
task_id: T-20260531153015-3a283c0b
title: Redesign VibeHub project detail UI and project structure explorer
mode: guided_drive
phase: align
phase_status: completed
created_at: 2026-05-31T15:30:15Z
created_by: vibehub
```

## File: .vibehub/tasks/T-20260531153015-3a283c0b/runs/R-20260531153015-bd42f253/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: T-20260531153015-3a283c0b
run_id: R-20260531153015-bd42f253
mode: guided_drive
phase: align
phase_status: completed
created_at: 2026-05-31T15:30:15Z
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

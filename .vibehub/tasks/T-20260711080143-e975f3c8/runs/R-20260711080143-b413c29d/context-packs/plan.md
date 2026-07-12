# Context Pack: Plan

Task: T-20260711080143-e975f3c8
Run: R-20260711080143-b413c29d
Phase: Plan
Generated at: 2026-07-12T03:44:04Z
Source commit: 4359ef6

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
      "`hard_observed`: 对照 M1 完成输出、现有 `StructureArchitecture` 产品入口、M3 task metadata、主计划 M3 与 RFC-003，完成 M3 任务范围和验收细化。",
      "`hard_observed`: 明确 M3 所有新增能力映射回 M1 现有结构架构 Tab，并加入真实性、降级、增量、性能和视觉回归 gate。"
    ],
    "full_ref": ".vibehub/tasks/T-20260711080143-e975f3c8/runs/R-20260711080143-b413c29d/outputs/output.md",
    "key_decisions": [
      "`user_confirmed`: M1 当前产品体验保持不变；M3 的价值来自真实数据质量和可解释下钻，而非增加页面数量。",
      "`agent_reported`: `ProjectStructureView` 是 UI 边界，scanner/indexer internal shapes 不直接泄漏给组件。",
      "`agent_reported`: freshness/completeness 是数据事实，不是装饰 badge；任何缓存或 partial service 都必须携带 model/source versions。",
      "`agent_reported`: M3 依赖 M2 的 Application Service/repository/event primitives，但 Project Intelligence index 可独立 rebuild，不能成为 event stream 的第二权威状态源。"
    ]
  }
]
```

## Neighbors

```json
[
  {
    "task_id": "T-20260711080143-fbe94685",
    "title": "M4 完成 Task 计划图、时间线与验收闭环",
    "active_capabilities": [
      "align"
    ],
    "shared_files": []
  },
  {
    "task_id": "T-20260711080144-5db160f2",
    "title": "M6 完成 legacy-v2 只读迁移与发布硬化",
    "active_capabilities": [
      "align"
    ],
    "shared_files": []
  },
  {
    "task_id": "T-20260711080144-de5ecb84",
    "title": "M5 用稳定 V3 自管理多会话与 worktree 编排",
    "active_capabilities": [
      "align"
    ],
    "shared_files": []
  }
]
```

## File: .vibehub/tasks/T-20260711080143-e975f3c8/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: T-20260711080143-e975f3c8
title: M3 构建 Project Intelligence 与架构地图
mode: guided_drive
phase: align
phase_status: active
created_at: 2026-07-11T08:01:43Z
created_by: vibehub
intent: 把现有文件结构浏览升级为证据化、增量化、可降级的项目架构与知识视图。
acceptance_criteria:
- 文件树支持按需分页、搜索、Git 叠层与 ignore
- manifest/import/symbol/README/ADR 证据进入版本化项目模型
- fresh/stale/partial/unsupported/rebuilding 生命周期可观察
- VibeHub 与至少两个异构样本项目达到性能和真实性预算
dependencies:
- M2
intake:
  batch_id: intake-20260711080143
  split_confidence: high
  suggested_order: 3
  total_tasks: 3
  source_message: align阶段已经结束，现在需要进行调查和计划阶段，产出正式 research pack、五份 RFC backlog 和 M0 Task Pack。M0–M6 分别创建独立任务，不能做成一个超级大任务。M0 固化契约和 fixtures，M1 做高保真前端，M2 再接真实 core/MCP。等 M4 稳定后再让 v3 自己管理 M5，避免过早 self-host。
  split_reason: 用户明确要求 M0-M6 分别创建独立任务，且每个里程碑都有独立交付面、依赖与验收边界。
```

## File: .vibehub/tasks/T-20260711080143-e975f3c8/runs/R-20260711080143-b413c29d/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: "T-20260711080143-e975f3c8"
run_id: "R-20260711080143-b413c29d"
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

# Context Pack: Align

Task: T-20260711080144-de5ecb84
Run: R-20260711080144-924b5971
Phase: Align
Generated at: 2026-07-12T05:49:18Z
Source commit: 4359ef6

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
    "capability": "align",
    "completed": [
      "`hard_observed`: 对照 M1 产品基线、M4 stability gate、M5 task metadata、主计划 M5 与 RFC-005，完成 M5 entry/self-host、orchestration、recovery 和 UI 映射验收重写。"
    ],
    "full_ref": ".vibehub/tasks/T-20260711080144-de5ecb84/runs/R-20260711080144-924b5971/outputs/output.md",
    "key_decisions": [
      "`user_confirmed`: M5 是第一个由稳定 V3 管理的里程碑，M0-M4 不提前 self-host。",
      "`agent_reported`: worktree 不是独立功能按钮，而是 PlanNode/session execution 的受控生命周期；产品状态落在现有 plan/timeline/brief。",
      "`agent_reported`: 一个 active parallel PlanNode 默认拥有一个 branch/worktree lease；只读 session 需显式例外。",
      "`agent_reported`: V2 在 shadow 期间只有 audit/rollback 权，不双写 V3 domain，也不成为第二投影源。"
    ]
  }
]
```

## Neighbors

```json
[
  {
    "task_id": "T-20260711080144-5db160f2",
    "title": "M6 完成 legacy-v2 只读迁移与发布硬化",
    "active_capabilities": [
      "align"
    ],
    "shared_files": []
  }
]
```

## File: .vibehub/tasks/T-20260711080144-de5ecb84/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: T-20260711080144-de5ecb84
title: M5 用稳定 V3 自管理多会话与 worktree 编排
mode: guided_drive
phase: align
phase_status: active
created_at: 2026-07-11T08:01:44Z
created_by: vibehub
intent: 仅在 M4 稳定后，让 V3 自己管理 M5 的任务状态，并实现 scope、lease、worktree、集成与冲突生命周期。
acceptance_criteria:
- 存在 M4 稳定性进入门：核心投影、验收闭环、恢复基准与双平台 smoke 连续通过
- M5 的任务、节点、session 与验收由 V3 自己记录并可恢复
- 三个并行 session 跨至少两种工具且工作区/事件无污染
- 冲突、崩溃、桌面退出与 Windows 文件占用均可解释和恢复
dependencies:
- M4
intake:
  batch_id: intake-20260711080143
  split_confidence: high
  suggested_order: 5
  total_tasks: 3
  source_message: align阶段已经结束，现在需要进行调查和计划阶段，产出正式 research pack、五份 RFC backlog 和 M0 Task Pack。M0–M6 分别创建独立任务，不能做成一个超级大任务。M0 固化契约和 fixtures，M1 做高保真前端，M2 再接真实 core/MCP。等 M4 稳定后再让 v3 自己管理 M5，避免过早 self-host。
  split_reason: 用户明确要求 M0-M6 分别创建独立任务，且每个里程碑都有独立交付面、依赖与验收边界。
```

## File: .vibehub/tasks/T-20260711080144-de5ecb84/runs/R-20260711080144-924b5971/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: "T-20260711080144-de5ecb84"
run_id: "R-20260711080144-924b5971"
mode: "guided_drive"
phase: "align"
phase_status: "active"
created_at: "2026-07-11T08:01:44Z"
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

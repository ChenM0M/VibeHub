# Context Pack: Align

Task: T-20260711080143-fbe94685
Run: R-20260711080143-1866a605
Phase: Align
Generated at: 2026-07-12T05:16:25Z
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
      "`hard_observed`: 对照 M1 Review/current components、M4 task metadata、主计划 M4、RFC-004 和 M5 entry requirements，完成 M4 范围、页面映射、真实性与稳定性验收重写。"
    ],
    "full_ref": ".vibehub/tasks/T-20260711080143-fbe94685/runs/R-20260711080143-1866a605/outputs/output.md",
    "key_decisions": [
      "`user_confirmed`: M1 是产品交互基线；M4 通过接真实状态与动作增强它，不另建 Task 产品。",
      "`agent_reported`: 概要、时间线、计划图和结构架构各有单一职责；五段方法论映射到 graph/template，不映射到页面数量。",
      "`agent_reported`: 节点完成、Criterion pass 和 Task completion 是三个独立状态，不能互相隐式推断。",
      "`agent_reported`: M5 entry gate 是 M4 的独立交付物；阈值必须预先声明且失败后不可下调。"
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

## File: .vibehub/tasks/T-20260711080143-fbe94685/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: T-20260711080143-fbe94685
title: M4 完成 Task 计划图、时间线与验收闭环
mode: guided_drive
phase: align
phase_status: active
created_at: 2026-07-11T08:01:43Z
created_by: vibehub
intent: 把 PlanGraph、Criterion、Finding、Attempt 与 session timeline 接入真实 core，形成可回放的五段工作闭环。
acceptance_criteria:
- 真实任务完成 align 到 review 的事件化闭环，图变更可回放
- Criterion 逐项验收，失败项不能投影为 completed
- Finding-remediation-re-review 保留全部 attempts
- 三类 agent 会话启动与 brief 归属正确，M4 达到稳定门槛
dependencies:
- M2
- M3
intake:
  batch_id: intake-20260711080143
  split_confidence: high
  suggested_order: 4
  total_tasks: 3
  source_message: align阶段已经结束，现在需要进行调查和计划阶段，产出正式 research pack、五份 RFC backlog 和 M0 Task Pack。M0–M6 分别创建独立任务，不能做成一个超级大任务。M0 固化契约和 fixtures，M1 做高保真前端，M2 再接真实 core/MCP。等 M4 稳定后再让 v3 自己管理 M5，避免过早 self-host。
  split_reason: 用户明确要求 M0-M6 分别创建独立任务，且每个里程碑都有独立交付面、依赖与验收边界。
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

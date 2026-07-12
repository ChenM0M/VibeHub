# Context Pack: Review

Task: T-20260711080143-e975f3c8
Run: R-20260711080143-b413c29d
Phase: Review
Generated at: 2026-07-12T04:57:42Z
Source commit: 4359ef6

## Instructions

Use this context only for the current phase.
Do not mark state.yaml completed.
Report files read, commands run, decisions made, and unresolved risks.

## Capability Output Schema

```json
{
  "required_fields": [
    "summary",
    "concerns",
    "gate_pass",
    "risk_review"
  ],
  "optional_fields": [
    "references",
    "related_threads"
  ],
  "produces": [
    "review_summary"
  ],
  "consumes": [
    "diff",
    "validation_result"
  ],
  "parallel_safe": true,
  "custom": false
}
```

## Prior Outputs Summary

```json
[
  {
    "capability": "review",
    "completed": [
      "`hard_observed`: 2026-07-12 状态复核确认 M3 仍处于 `review: active`；align/plan/implement 已完成，review 尚未通过，context pack 与 handoff 均可用且未标记 stale。",
      "`hard_observed`: `page()` canonicalize 目标目录并验证仍位于项目根；新增 Unix symlink escape 回归测试。",
      "`hard_observed`: `git_output` 改为 `trim_end()`，保留 porcelain 第一行状态空格；dirty fingerprint 纳入变更路径、状态、长度和纳秒 mtime，并新增同状态内容改写改变 model version 的测试。",
      "`hard_observed`: `ProjectPage` 携带实际 effective limit，structure projection 不再固定报告 200。",
      "`hard_observed`: 远程搜索以 request id 忽略过期响应，并正确清理 loading/error。",
      "`hard_observed`: 子目录页合并保留已有 parent_id；新增加载更多控制以消费当前目录 `next_cursor`。",
      "`hard_observed`: VibeHub 五次 release 样本修复后 first page 61.0-86.2ms、full index 63.2-64.0ms、search 68.0-71.1ms，均通过冻结预算。"
    ],
    "full_ref": ".vibehub/tasks/T-20260711080143-e975f3c8/runs/R-20260711080143-b413c29d/outputs/output.md",
    "key_decisions": [
      "`agent_reported`: cursor model version 采用 path/status/size/纳秒 mtime 的 dirty fingerprint，避免每次查询读取大批 dirty 文件导致 search 超过 100ms gate。",
      "`agent_reported`: Review 不因 macOS 本地通过而替代 Windows/kill/RSS 发布证据。"
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
phase: review
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
task_id: T-20260711080143-e975f3c8
run_id: R-20260711080143-b413c29d
mode: guided_drive
phase: review
phase_status: active
created_at: 2026-07-11T08:01:43Z
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

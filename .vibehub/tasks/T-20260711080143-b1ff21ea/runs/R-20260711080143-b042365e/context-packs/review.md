# Context Pack: Review

Task: T-20260711080143-b1ff21ea
Run: R-20260711080143-b042365e
Phase: Review
Generated at: 2026-07-12T03:23:03Z
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
      "`hard_observed`: 已审查 M2 diff、Implement context/handoff、任务 acceptance criteria、RFC-001/002、核心与 MCP 测试证据。",
      "`hard_observed`: 本机实现验证无失败：Rust workspace 234 tests passed，V3 contracts 366 assertions passed，TypeScript/Vite production build passed，MCP 2025-11-25 的 7 resources/3 tools 合约测试 passed。",
      "`hard_observed`: 事件 append/rebuild/idempotency/optimistic conflict 与五类 M0/M1 view schema compatibility 有自动化测试证据。",
      "`hard_observed`: Review gate verdict 为 `gate_pass: false`，原因是四条任务 acceptance criteria 中两条尚无完整硬证据，而不是本机代码测试失败。"
    ],
    "full_ref": ".vibehub/tasks/T-20260711080143-b1ff21ea/runs/R-20260711080143-b042365e/outputs/output.md",
    "key_decisions": [
      "`hard_observed`: 本机功能与契约实现可进入正式审查，但不能据此宣称 M2 完整验收。",
      "`hard_observed`: `gate_pass: false`；三宿主与 Windows 两项 acceptance gaps 属于 release blockers，Review 结论为 needs_action。",
      "`user_confirmed`: 用户要求结束 Implement 并进入 Review；未要求忽略或豁免既有 acceptance criteria。"
    ]
  }
]
```

## Neighbors

```json
[
  {
    "task_id": "T-20260711080143-e975f3c8",
    "title": "M3 构建 Project Intelligence 与架构地图",
    "active_capabilities": [
      "align"
    ],
    "shared_files": []
  },
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

## File: .vibehub/tasks/T-20260711080143-b1ff21ea/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: T-20260711080143-b1ff21ea
title: M2 实现 V3 事件核心与 MCP 控制面
mode: guided_drive
phase: review
phase_status: active
created_at: 2026-07-11T08:01:43Z
created_by: vibehub
intent: 在 M1 契约稳定后实现事件源 core、Application Service、真实 views、MCP stdio 与 CLI fallback。
acceptance_criteria:
- 追加事件到派生视图闭环支持 rebuild、idempotency 与 optimistic version conflict
- M0/M1 view contracts 接入真实数据且保持兼容
- Codex、OpenCode、Claude Code 完成 MCP 最小恢复闭环
- macOS/Windows 的 stdio、路径、锁、崩溃清理与协议税预算通过
dependencies:
- M0
- M1
intake:
  batch_id: intake-20260711080143
  split_confidence: high
  suggested_order: 2
  total_tasks: 3
  source_message: align阶段已经结束，现在需要进行调查和计划阶段，产出正式 research pack、五份 RFC backlog 和 M0 Task Pack。M0–M6 分别创建独立任务，不能做成一个超级大任务。M0 固化契约和 fixtures，M1 做高保真前端，M2 再接真实 core/MCP。等 M4 稳定后再让 v3 自己管理 M5，避免过早 self-host。
  split_reason: 用户明确要求 M0-M6 分别创建独立任务，且每个里程碑都有独立交付面、依赖与验收边界。
```

## File: .vibehub/tasks/T-20260711080143-b1ff21ea/runs/R-20260711080143-b042365e/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: T-20260711080143-b1ff21ea
run_id: R-20260711080143-b042365e
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

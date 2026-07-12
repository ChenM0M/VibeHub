# Context Pack: Align

Task: T-20260711080143-b1ff21ea
Run: R-20260711080143-b042365e
Phase: Align
Generated at: 2026-07-12T02:27:21Z
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
      "`hard_observed`: M1 Review 已 validate/output-lint/finish，用户确认当前 UI、功能与交互完全符合预期。",
      "`hard_observed`: 对照 M2 task metadata、M1 Review output、当前 cockpit/store、主计划 M2、RFC-001、RFC-002 与 M0 contracts，完成 M2 范围和验收重写。"
    ],
    "full_ref": ".vibehub/tasks/T-20260711080143-b1ff21ea/runs/R-20260711080143-b042365e/outputs/output.md",
    "key_decisions": [
      "`user_confirmed`: M1 是产品 golden baseline；M2 是 data/core integration milestone，不是第二轮前端设计。",
      "`agent_reported`: fixture repository 是回归 oracle，production repository 是真实运行入口，两者共享 consumer contract 而非两套 UI。",
      "`agent_reported`: UI 不直接写 canonical state；有限的人类确认、导航和 agent 启动通过受限 adapter 进入 Application Service。",
      "`agent_reported`: V2 只作为 M6 前的未迁移项目 fallback，不得进入 V3 domain/read model。"
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
phase: align
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
task_id: "T-20260711080143-b1ff21ea"
run_id: "R-20260711080143-b042365e"
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

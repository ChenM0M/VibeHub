# Context Pack: Implement

Task: T-20260711080143-b1ff21ea
Run: R-20260711080143-b042365e
Phase: Implement
Generated at: 2026-07-12T03:20:35Z
Source commit: 4359ef6

## Instructions

Use this context only for the current phase.
Do not mark state.yaml completed.
Report files read, commands run, decisions made, and unresolved risks.

## Capability Output Schema

```json
{
  "required_fields": [
    "diff_summary",
    "changed_files",
    "commands_run"
  ],
  "optional_fields": [
    "rollback_plan",
    "references"
  ],
  "produces": [
    "diff"
  ],
  "consumes": [
    "implementation_plan"
  ],
  "parallel_safe": false,
  "custom": false
}
```

## Prior Outputs Summary

```json
[
  {
    "capability": "implement",
    "completed": [
      "`hard_observed`: 新增 JSON Schema 2020-12 event envelope/application command contracts、generated TypeScript 与 valid/invalid contract assertions；冻结的五类 M0 view schemas 未改变。",
      "`hard_observed`: 独立 Rust V3 domain/Application Service/event store/projection 已完成 append、request-scoped idempotency、per-aggregate optimistic version、`sync_all` JSONL、partial-tail quarantine、atomic projection replacement 与 deterministic rebuild。",
      "`hard_observed`: lock lease 记录 PID；dead owner 可立即回收，live owner 不会被偷锁，30 秒 age fallback 只用于不可探测/PID 复用兜底。",
      "`hard_observed`: production `V3ViewRepository` 从 task metadata 与 V3 events 生成 project overview/structure、task timeline、plan graph、node brief；五类真实输出均通过冻结 schema，M3-owned 架构索引明确返回 `partial/uninitialized`。",
      "`hard_observed`: typed Tauri `v3_load_view_bundle` 已接入 M1 cockpit；native loader 位于 `src/services`，V3 view/store 保持 transport-neutral。刷新同时校验 project path 与 `project_id`，fixture playground 继续作为 regression oracle。",
      "`hard_observed`: 官方 Rust SDK `rmcp 2.2.0` 实现 MCP `2025-11-25` stdio server；七个 versioned resources 位于 `vibehub://v3/1.0/`，三个 tools 映射到同一 Application Service。",
      "`hard_observed`: MCP structured tool results 保留 append/duplicate/version conflict/scope mismatch/retryable semantics；stdout 仅 JSON-RPC、stderr 仅诊断、stdin EOF clean shutdown，cancel notification 不破坏服务。",
      "`hard_observed`: MCP adapter 作为 shared library 同时进入 CLI fallback 与 Tauri native binary，消除了 workspace 两个 `vibehub` binary 的构建顺序差异。",
      "`hard_observed`: release `VibeHub.app` 内最终 executable 完整通过 initialize、7 resources、五 view schema reads、3 tools、open/duplicate/log/close、cancel 与 shutdown contract，总耗时 100 ms。",
      "`hard_observed`: Codex 0.132.0 在 isolated HOME 完成 `mcp add/get` 参数数组验证；Claude Code 2.1.197 完成 config add/get，但 isolated HOME 因未登录无法执行 health check；host config 与 capability matrix 已文档化。",
      "`hard_observed`: RFC-001/002 已更新实现证据、accepted/provisional/not_tested 边界；全部 M2 文件已用 `vibehub record` 登记归属。"
    ],
    "full_ref": ".vibehub/tasks/T-20260711080143-b1ff21ea/runs/R-20260711080143-b042365e/outputs/output.md",
    "key_decisions": [
      "`user_confirmed`: M1 保持冻结 golden baseline；M2 仅通过 repository/loader 接线，不重做 cockpit 体验。",
      "`hard_observed`: JSON Schema 2020-12 是 read/write wire source of truth；Rust/TypeScript/MCP outputs 均服从该边界。",
      "`hard_observed`: MCP、CLI、Tauri 共用 typed Application Service/View Repository；adapter 不直接写 store，V3 view layer 不依赖 Tauri。",
      "`agent_reported`: versioned stdio MCP 是 primary，CLI 是 mandatory fallback；HTTP daemon、remote MCP、elicitation/list-changed 不进入最小恢复闭环。",
      "`agent_reported`: production structure 在 M3 前返回诚实的 partial view，不用猜测数据伪造 complete architecture intelligence。",
      "`inferred`: PID liveness + stale age fallback 足以完成 macOS crash cleanup，但 Windows runner 通过前不把跨平台锁决策升级为最终 accepted。"
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
phase: implement
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
phase: implement
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

# Context Pack: Review

Task: T-20260711080143-fbe94685
Run: R-20260711080143-1866a605
Phase: Review
Generated at: 2026-07-12T05:46:41Z
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
    "capability": "implement",
    "completed": [
      "`hard_observed`: lifecycle core、application bridge、production projection、M1 四 Tab 验收入口、RFC freeze 和 stability runner 已实现。",
      "`hard_observed`: 首轮稳定性报告为 `automated_gate=passed`、`macos_local=passed`、`windows_native=not_run`、`owner_approval=pending`、`m5_entry_gate=closed`。",
      "`hard_observed`: 首轮完整 `cargo test --workspace` 通过：Tauri 17、adapters 2、core 231，零失败；V3 contracts 366 assertions/12 scenarios、MCP contract 和 TypeScript production build 通过。",
      "`hard_observed`: Review 窗口重跑 `npm run v3:m4:stability` 与 `cargo test --workspace` 均通过；稳定性报告仍为 automated passed、M5 entry gate closed。",
      "`hard_observed`: 两个 P1 correctness finding 已修复：completion 校验绑定 canonical `task.yaml` 的完整验收项集合；UI 改为消费 projection 暴露的 proposal identity/version 与权威 valid/confirmed 状态。",
      "`hard_observed`: 新增回归测试覆盖“多 criterion 只通过子集必须拒绝”和“旧 confirmation 与新 proposal 同毫秒仍按 identity/version 正确关联”；M4 stability 与完整 workspace tests 均通过。"
    ],
    "full_ref": ".vibehub/tasks/T-20260711080143-fbe94685/runs/R-20260711080143-1866a605/outputs/output.md",
    "key_decisions": [
      "`agent_reported`: lifecycle optimistic version 归 Task aggregate；node/session ID 作为 envelope scope identity，不各自产生互相不可比较的 Task truth。",
      "`agent_reported`: completion proposal digest 由 core 生成；UI 只提交人工确认，任何后续 Task event 使旧 proposal 失效。",
      "`agent_reported`: Plan DAG 只含 `depends_on`，Finding/Attempt 使用 Trace Graph `addresses`，实际顺序只在 Event Timeline。",
      "`agent_reported`: canonical acceptance criterion ID 由 task ID 与 task.yaml 顺序确定；application service 读取完整集合后交给 lifecycle validator，未出现的 criterion 不再被视作通过。",
      "`agent_reported`: task-timeline `completion` 成为必填 view-contract 字段，包含 proposal event ID、proposal/confirmation aggregate version、digest、valid、confirmed、identity/channel；前端不再从时间戳推断归属。",
      "`user_confirmed`: 记录本次修复后结束并归档 M4，再切换到下一个任务。",
      "`agent_reported`: 测试影响限定为两条 correctness 回归、task-timeline contract/fixture 字段覆盖及既有 gate 重跑；未修改 soak/recovery 阈值、M5 gate 条件或生产事件数据。",
      "`user_confirmed`: 当前窗口不跨阶段；Review、最终复验和 M4 关闭留到下一窗口。"
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
phase: implement
phase_status: completed
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
task_id: T-20260711080143-fbe94685
run_id: R-20260711080143-1866a605
mode: guided_drive
phase: implement
phase_status: completed
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

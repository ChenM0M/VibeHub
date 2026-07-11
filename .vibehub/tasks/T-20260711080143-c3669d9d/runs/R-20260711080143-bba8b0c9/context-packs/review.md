# Context Pack: Review

Task: T-20260711080143-c3669d9d
Run: R-20260711080143-bba8b0c9
Phase: Review
Generated at: 2026-07-11T08:48:34Z
Source commit: d7ece57

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
      "`agent_reported`: Diff summary：新增五个 JSON Schema 2020-12 view contract 与共享 vocabulary；新增 12 场景/60 正例/2 反例的确定性 fixture corpus；新增 TypeScript 生成类型与统一 fixture repository；新增 279 断言的稳定契约检查；冻结五份 RFC 的 M0 边界并记录 M0-C01 至 M0-C12 证据。",
      "`hard_observed`: `npm run v3:contracts:check` 通过，报告 279 assertions、12 scenarios、5 views，并输出六个 schema SHA-256。",
      "`hard_observed`: repository consumer proof 实际转译并执行 TypeScript repository，加载 FX-HAPPY、FX-WIN-PATHS、FX-REWORK、FX-LARGE 的全部五个 view。",
      "`hard_observed`: V3 source boundary scan未发现 V2 YAML、Tauri invoke/service、`.vibehub` state 或 `vibehub-core` 依赖。",
      "`agent_reported`: Rollback plan：删除新增 `contracts/v3`、`fixtures/v3`、`scripts/v3-contracts`、`src/v3`，移除两个 npm scripts 与三个 devDependencies，并回退 RFC/Task Pack freeze 记录；无需迁移或回退任何 V2 runtime/canonical state。"
    ],
    "full_ref": ".vibehub/tasks/T-20260711080143-c3669d9d/runs/R-20260711080143-bba8b0c9/outputs/output.md",
    "key_decisions": [
      "`agent_reported`: JSON Schema 2020-12 是 canonical wire source；TypeScript 类型生成并做临时目录 byte drift comparison，Rust/event round-trip 留给 M2。",
      "`agent_reported`: M1 通过依赖注入的 `JsonLoader` 使用 `V3ViewRepository`；repository 不知道 transport、Tauri 或 core。",
      "`agent_reported`: fixture 全量由固定 seed 生成，FX-LARGE 不作为人工维护的 opaque snapshot；每个实例哈希写入 scenario manifest。",
      "`agent_reported`: RFC 只冻结 M0 消费边界，event/MCP/analyzer/task state machine/worktree implementation 仍按 M2-M5 的 exit criteria 决策。"
    ]
  }
]
```

## Neighbors

```json
[
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

## File: .vibehub/tasks/T-20260711080143-c3669d9d/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: T-20260711080143-c3669d9d
title: M0 冻结 V3 契约、fixtures 与实施基线
mode: guided_drive
phase: review
phase_status: active
created_at: 2026-07-11T08:01:43Z
created_by: vibehub
intent: 冻结五类 view contract、事件与证据术语、双平台 fixtures，并形成能独立驱动 M1 的 M0 Task Pack。
acceptance_criteria:
- ProjectOverviewView、ProjectStructureView、TaskTimelineView、PlanGraphView、NodeBrief 的 JSON Schema 与版本策略完成
- fixtures 覆盖空项目、大仓库、无架构文档、并行 Task、Windows 路径、stale/partial/error 状态
- 五份 RFC backlog 的决策问题、依赖与退出条件完成
- M1 可仅依赖 contracts/fixtures 开发，不读取 v2 YAML 或调用真实 v3 core
dependencies: []
intake:
  batch_id: intake-20260711080143
  split_confidence: high
  suggested_order: 0
  total_tasks: 3
  source_message: align阶段已经结束，现在需要进行调查和计划阶段，产出正式 research pack、五份 RFC backlog 和 M0 Task Pack。M0–M6 分别创建独立任务，不能做成一个超级大任务。M0 固化契约和 fixtures，M1 做高保真前端，M2 再接真实 core/MCP。等 M4 稳定后再让 v3 自己管理 M5，避免过早 self-host。
  split_reason: 用户明确要求 M0-M6 分别创建独立任务，且每个里程碑都有独立交付面、依赖与验收边界。
```

## File: .vibehub/tasks/T-20260711080143-c3669d9d/runs/R-20260711080143-bba8b0c9/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: T-20260711080143-c3669d9d
run_id: R-20260711080143-bba8b0c9
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

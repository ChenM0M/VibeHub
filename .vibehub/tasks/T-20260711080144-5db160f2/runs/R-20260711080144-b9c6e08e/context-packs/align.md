# Context Pack: Align

Task: T-20260711080144-5db160f2
Run: R-20260711080144-b9c6e08e
Phase: Align
Generated at: 2026-07-11T08:24:07Z
Source commit: d7ece57

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
    "capability": "plan",
    "completed": [
      "`hard_observed` 已产出五份正式 RFC backlog，分别覆盖 domain/event、MCP control plane、Project Intelligence、Task/PlanNode/acceptance、worktree orchestration。",
      "`hard_observed` 已产出 `docs/v3/m0-task-pack.md`，包含 scope/non-scope、五类 view contract 最小字段、12 组 fixture、planned file surface、7 个 work packages、12 条 Criterion、验证计划、stop conditions 与 M1 handoff。",
      "`hard_observed` 已将 M0-M6 七个独立 VibeHub Task ID、依赖和正式工件入口回写到 V3 总计划，并将状态从 draft 更新为 aligned。",
      "`user_confirmed` 计划顺序保持为 M0 contract/fixtures -> M1 fixture-only 高保真前端 -> M2 真实 core/MCP。",
      "`user_confirmed` RFC-004/005 固定 M4 stability gate；只有全部证据通过且项目所有者明确批准后，M5 才由 V3 self-host。"
    ],
    "full_ref": ".vibehub/tasks/T-20260711062223-6f07315c/runs/R-20260711062223-10a76733/outputs/output.md",
    "key_decisions": [
      "`inferred` M0 的 canonical boundary 是五份 JSON Schema 2020-12 read models 和 deterministic fixtures，不是 V2 storage/event/Tauri response。",
      "`inferred` M1 只能经 fixture repository 消费契约；不得读 V2 YAML、调用 V2 workflow commands 或启动真实 MCP。",
      "`inferred` M0 生产文件范围限制为 `contracts/v3`、`fixtures/v3`、contract scripts/types/tests 与 RFC/Task Pack 决策更新；不得改 V2 production workflow modules。",
      "`inferred` M0 native fixture 可验证 parsing/display/identity，但不得把未执行的 Windows/macOS file operations 报成通过。",
      "`inferred` M4 gate 包括 deterministic rebuild、idempotent retry、两轮 remediation history、kill-resume、三 host MCP loop、双平台 native smoke、零未解决 P0/P1、预声明 soak 和 owner approval。"
    ]
  }
]
```

## Neighbors

```json
[
  {
    "task_id": "T-20260711062223-6f07315c",
    "title": "完善 VibeHub V3 重设计方案与 Agent 集成架构",
    "active_capabilities": [],
    "shared_files": []
  },
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
    "task_id": "T-20260711080143-c3669d9d",
    "title": "M0 冻结 V3 契约、fixtures 与实施基线",
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
    "task_id": "T-20260711080144-de5ecb84",
    "title": "M5 用稳定 V3 自管理多会话与 worktree 编排",
    "active_capabilities": [
      "align"
    ],
    "shared_files": []
  }
]
```

## File: .vibehub/tasks/T-20260711080144-5db160f2/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: T-20260711080144-5db160f2
title: M6 完成 legacy-v2 只读迁移与发布硬化
mode: guided_drive
phase: align
phase_status: active
created_at: 2026-07-11T08:01:44Z
created_by: vibehub
intent: 隔离 legacy-v2 只读浏览，删除旧写路径，并完成 Windows/macOS 安装升级卸载与发布验证。
acceptance_criteria:
- legacy-v2 仅经独立只读 adapter 提供最小历史入口，不进入 v3 domain
- 旧命令、重复适配与 UI canonical 写入被清理
- Windows/macOS 从干净安装到 MCP、IDE、并行节点与卸载清理全流程通过
- 发布、回滚与归档文档可执行
dependencies:
- M5
intake:
  batch_id: intake-20260711080144
  split_confidence: high
  suggested_order: 6
  total_tasks: 1
  source_message: align阶段已经结束，现在需要进行调查和计划阶段，产出正式 research pack、五份 RFC backlog 和 M0 Task Pack。M0–M6 分别创建独立任务，不能做成一个超级大任务。M0 固化契约和 fixtures，M1 做高保真前端，M2 再接真实 core/MCP。等 M4 稳定后再让 v3 自己管理 M5，避免过早 self-host。
  split_reason: 用户明确要求 M0-M6 分别创建独立任务，且每个里程碑都有独立交付面、依赖与验收边界。
```

## File: .vibehub/tasks/T-20260711080144-5db160f2/runs/R-20260711080144-b9c6e08e/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: "T-20260711080144-5db160f2"
run_id: "R-20260711080144-b9c6e08e"
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

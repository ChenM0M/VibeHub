# Context Pack: Plan

Task: T-20260711062223-6f07315c
Run: R-20260711062223-10a76733
Phase: Plan
Generated at: 2026-07-11T08:07:41Z
Source commit: d7ece57

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
    "capability": "research",
    "completed": [
      "`hard_observed` 已产出正式 Research Pack 三件套：`.vibehub/research/current/research-pack.md`、`source-log.yaml`、`findings.yaml`。",
      "`hard_observed` 已调查现有 events/projection/research/project_structure/Tauri/UI/CI 基线，并记录可复用资产与 V2 泄漏边界。",
      "`hard_observed` 已核验 JSON Schema、MCP architecture/SDK、Rust SDK、Codex、Claude Code、OpenCode、Windows path、Git worktree 与 event sourcing 官方资料。",
      "`user_confirmed` 已固定顺序：M0 contracts/fixtures；M1 fixture-only 高保真前端；M2 真实 core/MCP；M4 稳定后才由 V3 自管理 M5。",
      "`hard_observed` 已通过三批 high-confidence intake 创建 M0-M6 七个独立 Task；总任务未吸收里程碑实现。"
    ],
    "full_ref": ".vibehub/tasks/T-20260711062223-6f07315c/runs/R-20260711062223-10a76733/outputs/output.md",
    "key_decisions": [
      "`inferred` 五个 read model 是 M0 的最小跨前后端边界；V2 YAML、V2 event enum 和 Tauri command response 不得成为 M1 contract。",
      "`inferred` M1 通过单一 fixture repository 接口运行，所有 canonical 写入和真实 MCP 调用推迟到 M2。",
      "`inferred` M2 的 MCP 与 CLI 都是 Application Service adapters；resources 承载读模型，typed tools 承载状态转换。",
      "`inferred` Windows 路径、fresh/stale/partial/error、并行 Task 与大仓库必须变成 fixture/测试输入。",
      "`inferred` M4 stability gate 必须以 rebuild/idempotency/recovery/criteria-history/cross-host/native smoke/P0-P1 缺陷证据判定；M5 是第一个 self-host 里程碑。"
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

## File: .vibehub/tasks/T-20260711062223-6f07315c/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: T-20260711062223-6f07315c
title: 完善 VibeHub V3 重设计方案与 Agent 集成架构
mode: evidence_drive
phase: research
phase_status: completed
created_at: 2026-07-11T06:22:23Z
created_by: vibehub
```

## File: .vibehub/tasks/T-20260711062223-6f07315c/runs/R-20260711062223-10a76733/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: T-20260711062223-6f07315c
run_id: R-20260711062223-10a76733
mode: evidence_drive
phase: research
phase_status: completed
created_at: 2026-07-11T06:22:23Z
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

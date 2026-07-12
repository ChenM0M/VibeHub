# Context Pack: Research

Task: T-20260711091709-5a15c972
Run: R-20260711091709-5cfcade1
Phase: Research
Generated at: 2026-07-11T09:17:23Z
Source commit: 4359ef6

## Instructions

Use this context only for the current phase.
Do not mark state.yaml completed.
Report files read, commands run, decisions made, and unresolved risks.

## Capability Output Schema

```json
{
  "required_fields": [
    "sources",
    "risks",
    "open_questions"
  ],
  "optional_fields": [
    "hypothesis",
    "references"
  ],
  "produces": [
    "research_output"
  ],
  "consumes": [],
  "parallel_safe": true,
  "custom": false
}
```

## Prior Outputs Summary

```json
[
  {
    "capability": "align",
    "completed": [
      "`hard_observed`: 读取 M0 全部交付物并确认 M0 已完成（phase: review,",
      "status: completed）：5 个 JSON Schema 2020-12 contracts、12 场景",
      "fixtures、generated TypeScript types、`createV3FixtureRepository`",
      "接口、`npm run v3:contracts:check`（285 assertions / 12 scenarios /",
      "5 views 通过）。",
      "`hard_observed`: 确认 M1 task 为当前指针",
      "(`T-20260711080143-49b5012c`)，phase=align (active)，M0",
      "(`T-20260711080143-c3669d9d`) 已 completed。",
      "`hard_observed`: 读取 v3 改版计划 §3.6 UI 信息架构、§4 M1 里程碑",
      "定义、M0 Task Pack M1 Handoff Contract，确认 M1 scope 与验收边界。",
      "`hard_observed`: 读取现有前端结构（src/，11,943 行 TS/TSX，",
      "VibehubCockpitDialog 4,268 行等），确认 M1 需新建 V3 UI 而非",
      "扩展 V2 组件。",
      "`hard_observed`: 确认 fixtureRepository.ts 的",
      "`V3ViewRepository` 接口（`listScenarios()` + `loadScenario()`）",
      "是 M1 的唯一数据入口；`V3FixtureBundle` 包含五类 view。",
      "`hard_observed`: 读取 ProjectOverviewView generated type 和",
      "FX-HAPPY fixture，确认 contract → TypeScript → fixture 链路完整。",
      "`agent_reported`: 产出 align 阶段 output.md，包含 intent、scope、",
      "success_criteria、non_goals、autonomy_level、risk_level。"
    ],
    "full_ref": ".vibehub/tasks/T-20260711091709-5a15c972/runs/R-20260711091709-5cfcade1/outputs/output.md",
    "key_decisions": [
      "`agent_reported`: M1 UI 代码放在 `src/v3/` 下，与 V2 UI（`src/components/`、",
      "`src/pages/`）隔离。M1 是新路由/新页面，不修改 V2 组件。理由：v3 改版",
      "计划明确 M4 才删除旧 UI 写入路径；M1 只加不减。",
      "`agent_reported`: M1 数据入口是 `createV3FixtureRepository`，通过",
      "`V3ViewRepository.loadScenario()` 加载 fixture JSON。fixture 路径",
      "默认 `/fixtures/v3`，在 Vite dev/build 中通过静态资源或 import",
      "加载（plan 阶段确定具体方式）。",
      "`agent_reported`: Contract gap 不在 M1 内补。发现 gap 时记录为",
      "M0 delta 候选，在 output.md 的 Warnings 或 Key Decisions 中列出，",
      "等用户确认后回传 M0 作为 versioned change。理由：M0 Task Pack",
      "M1 Handoff Contract 明确要求。",
      "`agent_reported`: M1 验收用 fixture 场景映射：FX-HAPPY 做主流程，",
      "FX-EMPTY 做空状态，FX-STALE 做过期，FX-PARTIAL 做部分支持，",
      "FX-ERROR 做错误，FX-LARGE 做大数据，FX-REWORK 做 review finding，",
      "FX-PARALLEL 做多 session，FX-WIN-PATHS/FX-MAC-PATHS 做跨平台路径，",
      "FX-NO-DOCS 做无架构文档，FX-COVERAGE-GAP 做 protocol coverage。"
    ]
  }
]
```

## Neighbors

```json
[
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

## File: .vibehub/tasks/T-20260711091709-5a15c972/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: T-20260711091709-5a15c972
title: M1 基于 fixtures 构建 V3 高保真前端体验
mode: evidence_drive
phase: align
phase_status: completed
created_at: 2026-07-11T09:17:09Z
created_by: vibehub
```

## File: .vibehub/tasks/T-20260711091709-5a15c972/runs/R-20260711091709-5cfcade1/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: T-20260711091709-5a15c972
run_id: R-20260711091709-5cfcade1
mode: evidence_drive
phase: align
phase_status: completed
created_at: 2026-07-11T09:17:09Z
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

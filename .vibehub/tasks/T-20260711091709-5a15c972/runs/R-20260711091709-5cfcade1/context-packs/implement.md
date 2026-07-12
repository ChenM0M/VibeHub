# Context Pack: Implement

Task: T-20260711091709-5a15c972
Run: R-20260711091709-5cfcade1
Phase: Implement
Generated at: 2026-07-11T15:48:34Z
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
      "`hard_observed`: 读取 plan context pack，确认 required_fields: steps, validation_plan, affected_files。",
      "`hard_observed`: 读取 tauri.conf.json，确认 devUrl=localhost:1420, frontendDist=../dist。",
      "`hard_observed`: 检查 public/ 目录和 fixtures/v3/ 路径，确认需要配置静态资源复制（R2 验证）。",
      "`hard_observed`: 检查 FX-LARGE 数据量（250 nodes, 300 events, 80 plan nodes），确认虚拟滚动必要。",
      "`hard_observed`: 检查 FX-EMPTY/FX-ERROR 降级状态字段，确认状态映射策略。",
      "`agent_reported`: 产出 5 阶段实现计划（阶段 0 基础设施 → 阶段 1 Project 视图 → 阶段 2 Task 视图 → 阶段 3 整合 → 阶段 4 状态覆盖 → 阶段 5 走查收尾）。",
      "`agent_reported`: 产出验证计划（M1-C01~C05 对应验证矩阵）。",
      "`agent_reported`: 产出影响文件清单（16 个新建文件 + 5 个修改文件 + V2 隔离边界）。",
      "`agent_reported`: 产出上下文计划（implement 阶段必读和参考文件）。",
      "`agent_reported`: 解决 Q1-Q5 开放问题。"
    ],
    "full_ref": ".vibehub/tasks/T-20260711091709-5a15c972/runs/R-20260711091709-5cfcade1/outputs/output.md",
    "key_decisions": [
      "`agent_reported`: 实现按 5 阶段垂直薄切片推进，每个薄切片可独立验证。先 FX-HAPPY 主流程，再逐步覆盖非正常状态。",
      "`agent_reported`: Fixture 路径用 Vite 静态资源复制（viteStaticCopy 或 public/ symlink），不打入 JS bundle。",
      "`agent_reported`: V3 入口在 `main.tsx` 新增 `'v3'` PageType，侧边栏新增导航项，不修改 V2 路由逻辑。",
      "`agent_reported`: 12 场景验证用手动 ScenarioSelector 切换走查，不强制自动化快照测试（R6 缓解）。",
      "`agent_reported`: trace_relations 用 toggle 切换显示（Q1 决策）。",
      "`agent_reported`: 时间轴比例尺从 events 自动计算（Q2 决策）。",
      "`agent_reported`: 搜索时扁平展示匹配项 + 父路径 breadcrumb（Q3 决策）。",
      "`agent_reported`: 单元测试是 nice-to-have，优先完成 UI 实现（Q4 决策）。",
      "`agent_reported`: i18n label_key 翻译放 `src/v3/locales/{en,zh}/v3.json`（Q5 决策）。"
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
phase: implement
phase_status: active
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
phase: implement
phase_status: active
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

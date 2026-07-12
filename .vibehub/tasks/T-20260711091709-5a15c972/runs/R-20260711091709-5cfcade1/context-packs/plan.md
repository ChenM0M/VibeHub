# Context Pack: Plan

Task: T-20260711091709-5a15c972
Run: R-20260711091709-5cfcade1
Phase: Plan
Generated at: 2026-07-11T09:22:19Z
Source commit: 4359ef6

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
      "`hard_observed`: 调查 React Flow（v12, MIT, 37.6K stars）作为 DAG/计划图可视化方案，确认支持自定义节点/边、Dagre 自动布局、Tailwind 兼容、暗色模式、TypeScript。",
      "`hard_observed`: 调查 TanStack Virtual（MIT, 819M downloads）作为虚拟滚动方案，确认 headless 设计、支持动态行高/水平泳道/网格。",
      "`hard_observed`: 调查 D3.js（v7.9, ISC, 113K stars）作为备选底层可视化库，确认 d3-hierarchy 和 d3-scale 可用但需手动 React 集成。",
      "`hard_observed`: 调查 Linear Method 设计原则，提取 8 条与 VibeHub V3 直接适用的设计哲学。",
      "`hard_observed`: 调查 Tauri v2 开发文档，确认 Vite + Tauri 集成模式和前端架构自由度。",
      "`hard_observed`: 阅读现有项目代码库（main.tsx, vite.config.ts, appStore.ts, VibehubProjectCenter.tsx, package.json），确认技术栈和可复用组件。",
      "`hard_observed`: 阅读全部 5 个 generated TypeScript types，确认 M1 UI 需要渲染的数据结构和字段。",
      "`agent_reported`: 产出技术选型推荐表（8 项需求 → 推荐方案 → 理由 → 替代方案）。",
      "`agent_reported`: 产出信息架构设计原则（5 条核心原则：两层控制台、渐进式披露、字段有来源、状态优先、密度可调）。",
      "`agent_reported`: 产出组件架构推荐（src/v3/ 目录结构，4 个 common 组件 + 4 个 project 组件 + 4 个 task 组件）。",
      "`agent_reported`: 产出 fixture 加载策略分析（方案 A import.meta.glob vs 方案 B fetch from public/，推荐方案 B）。",
      "`agent_reported`: 产出降级状态渲染策略矩阵（freshness × completeness → UI 行为）。",
      "`agent_reported`: 产出跨平台视口策略（路径显示、视口尺寸、大数据处理）。",
      "`agent_reported`: 识别 5 项风险（R1-R5）和 5 个开放问题（Q1-Q5）。"
    ],
    "full_ref": ".vibehub/tasks/T-20260711091709-5a15c972/runs/R-20260711091709-5cfcade1/outputs/output.md",
    "key_decisions": [
      "`agent_reported`: DAG/计划图用 **React Flow** (`@xyflow/react`)，不用裸 D3。理由：React Flow 已封装交互层（拖拽/缩放/平移/选择），与 Tailwind 兼容，TypeScript 一等支持，减少 M1 实现量。",
      "`agent_reported`: 虚拟滚动用 **TanStack Virtual** (`@tanstack/react-virtual`)，不用 react-window。理由：headless 设计不强制视觉组件，支持动态行高和水平泳道，与现有 Zustand 同生态。",
      "`agent_reported`: 时间线泳道**自建**而非用 Gantt 库。理由：Contract 的 lanes[]/events[] 模型是事件驱动而非任务条，Gantt 库太僵化。用 TanStack Virtual 水平模式 + CSS Grid 自建。",
      "`agent_reported`: 结构浏览器树**自建**而非用通用树组件。理由：需要 evidence_refs 展示、Git 叠层、路径 native/display 分离，通用树组件无法满足。",
      "`agent_reported`: 状态管理继续用 **Zustand**，不引入新依赖。新增 `v3Store` 管理场景选择/视图切换/选中项。",
      "`agent_reported`: 路由用 **Zustand state 模拟**（与现有 V2 一致），不引入 react-router。M1 是 fixture 原型，不需要 URL 路由。",
      "`agent_reported`: Fixture 加载用 **fetch from public/** 方案，不用 import.meta.glob。理由：不打入 bundle（FX-LARGE 可能大），更接近 M2 真实异步加载模式，`createV3FixtureRepository` 默认 basePath 已匹配。",
      "`agent_reported`: M1 UI 代码放 `src/v3/` 下，复用现有 `src/components/ui/*` shadcn 组件但不修改 V2 组件。在 `main.tsx` 新增 V3 入口。"
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
phase: research
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
phase: research
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

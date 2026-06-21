# Context Pack: Implement

Task: T-20260609092907-7c439d16
Run: R-20260609092907-c9e04b56
Phase: Implement
Generated at: 2026-06-21T05:51:49Z
Source commit: d08b65e

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
      "`hard_observed`: 已按 `continue/继续` 路由先执行 VibeHub sync；`cargo run -p vibehub-cli --offline -- sync /Users/chenm0m/LocalRepo/VibeHub` 生成 `.vibehub/agent-view/sync.md` 和本 run 下 `sync-20260615-065327.md`，状态为 `needs_attention`，原因是 dirty worktree ownership unavailable。",
      "`hard_observed`: 在 `src/components/VibehubCockpitDialog.tsx` 中新增共享 `TaskLifecycleCanvas`，用 React + SVG/HTML 实现只读任务生命周期 canvas；未新增 React Flow / Konva / ELK 等依赖。",
      "`hard_observed`: `TaskDetailContent` 改为复用 `TaskLifecycleCanvas`，具体任务详情不再显示全项目任务列表或左侧阶段列表 + 右侧堆叠信息的旧结构。",
      "`hard_observed`: `StatusTabContent` / phase detail 改为复用同一个 `TaskLifecycleCanvas`，从 phase pill 打开时继续传入 `selectedTaskId` / `selectedPhase` 并自动聚焦对应节点。",
      "`hard_observed`: Canvas 节点按任务 mode 的完整 phase flow 展示，节点包含 phase/status、read inputs、written outputs、events 和 package path 摘要；节点之间用 SVG 连接线表达阶段流转。",
      "`hard_observed`: 右侧 inspector 展示选中 phase 的 task/run metadata、阶段传递包体、phase files、当前 phase validation、节点事件、task relations 和 warnings。",
      "`hard_observed`: task / phase detail drawer 宽度调整为 `max-w-5xl`，给 canvas + inspector 留出稳定空间；activity/archive/git 仍保持 `max-w-4xl`。",
      "`hard_observed`: `npm run build` 已通过，包含 TypeScript 严格检查和 Vite production build。",
      "`hard_observed`: 本地 Vite dev server 已启动于 `http://127.0.0.1:1420/`；Codex in-app Browser 可加载首页且无 console error，但当前浏览器 profile 没有 workspace/project，无法通过正常 UI 进入 cockpit 做截图级 canvas 验收。",
      "`user_confirmed`: 用户确认不用继续等待视觉验收，直接提交、推送、发版。",
      "`hard_observed`: 发布版本已统一 bump 到 `2.0.0-pre.16`，覆盖 `package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`、`crates/vibehub-cli/Cargo.toml`、`crates/vibehub-core/Cargo.toml` 和 `Cargo.lock`。",
      "`hard_observed`: `npm run build`、`cargo test`、`npm run tauri -- build --target aarch64-apple-darwin --bundles app` 均已在 `2.0.0-pre.16` 下通过。"
    ],
    "full_ref": ".vibehub/tasks/T-20260531153015-3a283c0b/runs/R-20260531153015-bd42f253/outputs/output.md",
    "key_decisions": [
      "`user_confirmed`: 项目页定位仍是“项目状态中台 / project command map”，不推翻主视图；本轮只改“点击具体任务/phase 后的内部详情形态”。",
      "`agent_reported`: 任务详情采用轻量自研 canvas，不新增流程图库；核心是只读展示任务内阶段流转、上下文传递、输出、验证和事件历史。",
      "`agent_reported`: phase detail 不再是一套孤立状态页；它复用任务生命周期 canvas，并用传入 phase 作为默认选中节点。",
      "`agent_reported`: Canvas 只读，不新增任何 workflow mutation button；保留现有安全的 artifact/file/preview 模式。"
    ]
  }
]
```

## Neighbors

```json
[
  {
    "task_id": "T-20260531153015-3a283c0b",
    "title": "Redesign VibeHub project detail UI and project structure explorer",
    "active_capabilities": [],
    "shared_files": []
  },
  {
    "task_id": "T-20260616062024-a8e8bdc2",
    "title": "test cli dispatch",
    "active_capabilities": [
      "align"
    ],
    "shared_files": []
  },
  {
    "task_id": "T-20260619044922-98d818ee",
    "title": "Display local Codex and OpenCode workspace usage insights",
    "active_capabilities": [
      "implement"
    ],
    "shared_files": []
  }
]
```

## File: .vibehub/tasks/T-20260609092907-7c439d16/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: T-20260609092907-7c439d16
title: Harden VibeHub agent protocol and CLI routing
mode: guided_drive
phase: implement
phase_status: active
created_at: 2026-06-09T09:29:07Z
created_by: vibehub
```

## File: .vibehub/tasks/T-20260609092907-7c439d16/runs/R-20260609092907-c9e04b56/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: T-20260609092907-7c439d16
run_id: R-20260609092907-c9e04b56
mode: guided_drive
phase: implement
phase_status: active
created_at: 2026-06-09T09:29:07Z
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

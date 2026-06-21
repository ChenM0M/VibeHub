# Context Pack: Implement

Task: T-20260531153015-3a283c0b
Run: R-20260531153015-bd42f253
Phase: Implement
Generated at: 2026-06-21T03:10:51Z
Source commit: 751d88c

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
    "capability": "review",
    "completed": [
      "`hard_observed`: 已进入 `review` 阶段并读取 review context pack。",
      "`hard_observed`: 已运行 `git diff --check -- src/stores/appStore.ts src/main.tsx src/components/Header.tsx src-tauri/src/main.rs`，未发现空白错误。",
      "`hard_observed`: 已运行 `vibehub review /Users/chenm0m/LocalRepo/VibeHub` 生成 review evidence。",
      "`agent_reported`: 已人工复查主题监听、入口冲突移除、Header effective theme、macOS titlebar overlay 和平台隔离路径。",
      "### summary",
      "`agent_reported`: 本次改动满足 align/plan 的核心目标：`auto` 模式会监听系统深浅色变化，手动 `light` / `dark` 不被系统变化覆盖，入口层不再覆盖 store 的 auto 结果。",
      "`agent_reported`: macOS 路径保留原生 traffic lights，并设置 overlay titlebar style；Windows/Linux 的自定义窗口按钮仍受 `!isMac` 条件保护。",
      "`agent_reported`: 验证覆盖 TypeScript/Vite 构建、Rust fmt、Rust check、主题 resolver 断言和 diff whitespace check。",
      "### concerns",
      "`agent_reported`: 未发现阻塞性代码问题。",
      "`inferred`: Header 左侧 `ml-20` 是保守预留值，真实 macOS 窗口里仍可能需要视觉微调。",
      "`hard_observed`: `vibehub review` 的 evidence 统计包含大量 VibeHub 状态文件，不全是本任务应用源码改动；本次人工 review 重点限定在应用代码四个文件和当前任务输出。",
      "### gate_pass",
      "`agent_reported`: pass。",
      "### verdict",
      "`agent_reported`: pass。当前主题/标题栏 diff 可以进入收口；未发现阻塞问题。",
      "### risk_review",
      "`agent_reported`: auto 主题监听风险低：listener 只在配置为 `auto` 时响应系统变化，且初始化有单例 guard，避免 React StrictMode 下重复注册。",
      "`agent_reported`: 手动主题回归风险低：`resolveEffectiveTheme('light', true)` 保持 `light`，`resolveEffectiveTheme('dark', false)` 保持 `dark`，并已用 Node 断言验证。",
      "`agent_reported`: 平台影响风险中低：macOS titlebar 改动在 Rust `#[cfg(target_os = \"macos\")]` 内，Header 的窗口按钮仍只在非 macOS 显示。",
      "`inferred`: macOS 视觉风险未完全消除：当前未打开真实 GUI 窗口确认 traffic lights 与搜索框的精确距离。",
      "### evidence_grades",
      "`hard_observed`: 源码 diff、`npm run build`、`cargo fmt --check`、`cargo check`、Node resolver assertions、`git diff --check` 均来自本地命令输出。",
      "`agent_reported`: 代码审查结论、风险评级和 gate verdict 来自本轮人工审查。",
      "`inferred`: macOS 真实视觉微调风险来自 titlebar overlay 行为和未打开 GUI 的验证边界。"
    ],
    "full_ref": ".vibehub/tasks/T-20260620165245-165f9e8f/runs/R-20260620165245-c3b6eb90/outputs/output.md",
    "key_decisions": [
      "`agent_reported`: 不再为本次 review 回改代码；当前 diff 进入通过状态。",
      "`agent_reported`: 将 macOS 真实视觉确认记录为非阻塞风险，而非阻塞本次功能修复。"
    ]
  }
]
```

## Neighbors

```json
[
  {
    "task_id": "T-20260609092907-7c439d16",
    "title": "Harden VibeHub agent protocol and CLI routing",
    "active_capabilities": [
      "implement"
    ],
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
  },
  {
    "task_id": "T-20260620165245-165f9e8f",
    "title": "Add desktop auto dark mode and macOS integrated titlebar",
    "active_capabilities": [],
    "shared_files": []
  },
  {
    "task_id": "T-20260620165245-f6d23db9",
    "title": "Align usage display with remote billing statistics",
    "active_capabilities": [],
    "shared_files": []
  }
]
```

## File: .vibehub/tasks/T-20260531153015-3a283c0b/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: T-20260531153015-3a283c0b
title: Redesign VibeHub project detail UI and project structure explorer
mode: guided_drive
phase: implement
phase_status: active
created_at: 2026-05-31T15:30:15Z
created_by: vibehub
```

## File: .vibehub/tasks/T-20260531153015-3a283c0b/runs/R-20260531153015-bd42f253/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: T-20260531153015-3a283c0b
run_id: R-20260531153015-bd42f253
mode: guided_drive
phase: implement
phase_status: active
created_at: 2026-05-31T15:30:15Z
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

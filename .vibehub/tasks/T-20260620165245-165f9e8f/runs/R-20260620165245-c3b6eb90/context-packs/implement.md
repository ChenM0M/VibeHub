# Context Pack: Implement

Task: T-20260620165245-165f9e8f
Run: R-20260620165245-c3b6eb90
Phase: Implement
Generated at: 2026-06-21T02:36:39Z
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
    "capability": "plan",
    "completed": [
      "`hard_observed`: 已完成 align 阶段验证与推进：`vibehub finish` 成功后 `vibehub advance` 将当前任务推进到 `plan`。",
      "`hard_observed`: 已读取 plan context pack，当前 plan 阶段要求输出 `steps`、`validation_plan`、`affected_files`。",
      "`hard_observed`: 已补充检查 `src-tauri/src/main.rs`，确认 macOS 启动路径中存在 `window.set_decorations(true)`，这是重新启用原生标题栏、造成顶部额外框感的关键代码路径。",
      "`hard_observed`: 已确认 Tauri 2 本地依赖支持 `set_title_bar_style` / `TitleBarStyle`，可用于 macOS 标题栏 overlay/transparent 方案。",
      "### steps",
      "`agent_reported`: 1. 在 `src/stores/appStore.ts` 增加集中主题解析逻辑：根据配置主题和 `prefers-color-scheme` 计算 effective theme，并统一应用 `document.documentElement.classList.toggle('dark', isDark)`。",
      "`agent_reported`: 2. 在 store 中保存 `effectiveTheme`，让 UI 能知道 `auto` 当前实际是浅色还是深色。",
      "`agent_reported`: 3. 在初始化时建立 `matchMedia('(prefers-color-scheme: dark)')` listener；只有配置为 `auto` 时，系统变化才会更新 effective theme 和 DOM class。",
      "`agent_reported`: 4. 在 `setTheme` 和 `refreshConfig` 后重新应用集中主题逻辑，确保配置变更、加载配置、系统变化共用同一入口。",
      "`agent_reported`: 5. 移除 `src/main.tsx` 中只识别 `dark` 的独立 dark mode effect，避免它在 `auto` 下把 store 已设置的 `dark` class 清掉。",
      "`agent_reported`: 6. 更新 `src/components/Header.tsx`：主题图标基于 `effectiveTheme`，点击仍在手动浅/深之间切换，不破坏设置页的 `auto` 选择。",
      "`agent_reported`: 7. 修改 macOS 窗口配置/启动逻辑：停止在 macOS 上调用 `set_decorations(true)` 打开完整原生标题栏，改用 Tauri 的 macOS title bar overlay/transparent 样式保留原生 traffic lights。",
      "`agent_reported`: 8. 为前端 header 增加 macOS 平台 class 或 data attribute，在 macOS 上给左侧内容预留 traffic-light 区域；非 macOS 保持现有搜索栏、按钮和窗口控制布局。",
      "`agent_reported`: 9. 如 Tauri Rust API 在当前版本下编译不接受预期的 `TitleBarStyle` 路径，则退回到配置级 `titleBarStyle`，并用构建结果验证。",
      "### validation_plan",
      "`agent_reported`: 运行 `npm run build`，验证 TypeScript 和 Vite 构建通过。",
      "`agent_reported`: 运行 `cargo fmt --manifest-path src-tauri/Cargo.toml --check`，验证 Rust 格式。",
      "`agent_reported`: 运行 `cargo check --manifest-path src-tauri/Cargo.toml`，验证 Tauri/Rust API 改动可编译。",
      "`agent_reported`: 使用 Node/JSDOM 或轻量脚本验证主题 resolver：`auto` 会随 mocked `matchMedia` change 改变 `dark` class，手动 `light` / `dark` 不被系统变化覆盖。",
      "`agent_reported`: 如本机允许 GUI/Tauri 运行，启动应用或截图检查 macOS header 左侧预留区；如无法实测 GUI，则在 implementation 输出中记录该视觉验证风险。",
      "### affected_files",
      "`agent_reported`: `src/stores/appStore.ts`：集中主题 resolver、effective theme 状态、系统主题监听。",
      "`agent_reported`: `src/main.tsx`：移除冲突的 dark mode effect。",
      "`agent_reported`: `src/components/Header.tsx`：使用 effective theme，并为 macOS 标题栏/traffic lights 留出空间。",
      "`agent_reported`: `src/styles/globals.css`：必要时增加 macOS titlebar/header 安全区域样式。",
      "`agent_reported`: `src-tauri/src/main.rs`：macOS 窗口标题栏配置，避免重新启用违和原生标题栏。",
      "`agent_reported`: `src-tauri/tauri.conf.json`：必要时增加/调整 macOS title bar style 配置。"
    ],
    "full_ref": ".vibehub/tasks/T-20260620165245-165f9e8f/runs/R-20260620165245-c3b6eb90/outputs/output.md",
    "key_decisions": [
      "`agent_reported`: 主题状态采用 `Theme` 配置值 + `effectiveTheme` 派生值模型，避免把 `auto` 改写成手动主题。",
      "`agent_reported`: `src/main.tsx` 不再直接操作 `document.documentElement.classList`，由 store 统一负责。",
      "`agent_reported`: macOS 标题栏优先通过 Tauri title bar style 和前端平台留白解决，不改变 Windows/Linux 现有窗口按钮。"
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
    "active_capabilities": [
      "implement"
    ],
    "shared_files": []
  },
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
    "task_id": "T-20260620165245-f6d23db9",
    "title": "Align usage display with remote billing statistics",
    "active_capabilities": [
      "implement"
    ],
    "shared_files": []
  }
]
```

## File: .vibehub/tasks/T-20260620165245-165f9e8f/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: T-20260620165245-165f9e8f
title: Add desktop auto dark mode and macOS integrated titlebar
mode: guided_drive
phase: plan
phase_status: completed
created_at: 2026-06-20T16:52:45Z
created_by: vibehub
intent: Add system-following dark mode on Windows and macOS, and improve macOS window chrome so VibeHub uses an integrated custom app titlebar with native red/yellow/green controls while preserving behavior on other platforms.
acceptance_criteria:
- VibeHub can follow system light/dark appearance automatically on Windows and macOS.
- Manual theme behavior, if already present, remains compatible and predictable.
- macOS uses an integrated hidden-titlebar/overlay style that keeps native traffic-light controls visually within the app surface.
- Windows/Linux window behavior is unchanged except for system theme following where intended.
- Visual or build verification covers macOS-specific config and cross-platform guarded code paths.
dependencies: []
intake:
  batch_id: intake-20260620165245
  split_confidence: high
  suggested_order: 2
  total_tasks: 2
  source_message: 目前这个用量显示总感觉还是不太对劲。我是希望它能够跟我的远端保持一致的，就是远端主要用来计价的，一般来说的那个统计呃统计数字。我的远端要么是Sub to API，要么是new API这种。然后我希望你这一次做了丰富且全面的调查之后再实行。其次就是需要给Vibehub添加一个就是自动变深色模式的功能。无论是Windows电脑还是Mac电脑。其次就是目前在Mac电脑上面，我觉得还是不够美观，因为它上面它有一个那个框。它不是就是它最顶上它有个框一样的，而不是直接在应用上我们自己，然后包含它Mac的三个操作，就是红黄绿的三个点这样子。而上面一个框这样，好奇怪哦，很违和。不过修改的时候记得不要影响到其他平台、其他系统。
  split_reason: The usage accounting model and desktop window/theme behavior are independently deliverable, testable, and likely touch different subsystems. The usage task also requires dedicated research before implementation.
```

## File: .vibehub/tasks/T-20260620165245-165f9e8f/runs/R-20260620165245-c3b6eb90/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: T-20260620165245-165f9e8f
run_id: R-20260620165245-c3b6eb90
mode: guided_drive
phase: plan
phase_status: completed
created_at: 2026-06-20T16:52:45Z
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

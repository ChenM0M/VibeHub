# Context Pack: Plan

Task: T-20260620165245-165f9e8f
Run: R-20260620165245-c3b6eb90
Phase: Plan
Generated at: 2026-06-21T02:34:22Z
Source commit: 751d88c

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
    "capability": "align",
    "completed": [
      "`hard_observed`: 已读取当前任务、运行与阶段状态：任务 `T-20260620165245-165f9e8f`，运行 `R-20260620165245-c3b6eb90`，阶段 `align`。",
      "`hard_observed`: 已读取当前 align context pack，任务原始意图包含“Windows/macOS 跟随系统深浅色”和“macOS 标题栏一体化且不影响其他平台”。",
      "`hard_observed`: 已检查主题相关实现，发现 `src/stores/appStore.ts` 只在初始化和 `setTheme` 时读取一次 `prefers-color-scheme`，没有监听系统主题变化。",
      "`hard_observed`: 已检查 `src/main.tsx`，发现其 dark mode effect 只处理 `config?.theme === 'dark'`，在 `auto` 模式下可能移除 `dark` class，与 store 中的 auto 逻辑冲突。",
      "`hard_observed`: 已检查 `src/components/Header.tsx`，发现头部主题按钮只用 `config?.theme === 'dark'` 判断图标和切换目标，不能表达 auto 模式的当前有效主题。",
      "`hard_observed`: 已检查 `src-tauri/tauri.conf.json`，当前主窗口 `decorations` 为 `false`，应用已经使用自定义标题栏；macOS 侧仍需确保原生 traffic lights 能自然融入应用表面。",
      "### intent",
      "`user_confirmed`: 修复“跟随系统”模式不真正跟随 OS 深浅色变化的问题；用户观察到其他软件已经随系统切换时，VibeHub 仍可能停留在深色。",
      "`user_confirmed`: 将该问题写入计划，并在计划完成后直接进入 implementation，一直推进到完成。",
      "`hard_observed`: 任务原始意图还包含 macOS 顶部窗口框/标题栏违和，需要让 macOS 的红黄绿窗口控制更像位于应用自身界面中，同时不影响其他平台。",
      "### scope",
      "`agent_reported`: 前端主题状态需要集中到一个 resolver：`light` 强制浅色、`dark` 强制深色、`auto` 使用 `window.matchMedia('(prefers-color-scheme: dark)')` 的当前值。",
      "`agent_reported`: `auto` 模式需要注册 `matchMedia(...).change` 监听，OS 主题改变时同步更新 `document.documentElement.classList`。",
      "`agent_reported`: 移除或改造 `src/main.tsx` 中与 store 冲突的单独 dark mode effect，让全局只有一个主题应用入口。",
      "`agent_reported`: 头部主题按钮应基于“当前有效主题”显示 sun/moon，而不是只看存储配置是否为 `dark`。",
      "`agent_reported`: macOS 标题栏一体化只做平台相关配置/样式调整；Windows/Linux 保持现有自定义窗口按钮与拖拽行为。",
      "### success_criteria",
      "`agent_reported`: 当设置为“跟随系统/auto”时，系统从浅色切到深色或从深色切到浅色，VibeHub 的根节点 `dark` class 会实时变化，无需重启应用。",
      "`agent_reported`: 当设置为手动 `light` 或 `dark` 时，系统主题变化不会覆盖用户手动选择。",
      "`agent_reported`: 初始化加载配置后，`auto` 模式能立刻按当前系统偏好应用正确主题。",
      "`agent_reported`: `src/main.tsx` 不再有与 store 冲突的 auto 忽略逻辑。",
      "`agent_reported`: macOS 标题栏一体化仅影响 macOS 视觉/配置路径；非 macOS 平台的窗口按钮、拖拽和装饰行为不回退。",
      "### acceptance_criteria",
      "`agent_reported`: 设置页选择“跟随系统”后，运行时 OS 主题变化会更新应用主题。",
      "`agent_reported`: 设置页选择“浅色”或“深色”后，应用保持用户指定主题。",
      "`agent_reported`: Header 主题图标与当前有效外观一致，不因 `auto` 配置而固定显示错误状态。",
      "`agent_reported`: macOS 应用顶部视觉不再出现违和的额外框感；原生窗口控制在 macOS 上保留并与应用表面协调。",
      "`agent_reported`: 验证至少包含前端构建；如平台限制无法实测 macOS 窗口外观，需要在 warnings 中记录。",
      "### autonomy_level",
      "`user_confirmed`: 用户已授权“写完计划之后进入 implementation 阶段并一直到完成”，允许在完成阶段输出和验证后运行 `vibehub finish` / `vibehub advance`。",
      "### non_goals",
      "`agent_reported`: 本任务不重做完整视觉设计、不调整主题色体系、不改变设置项文案结构。",
      "`agent_reported`: 本任务不改变 Windows/Linux 的窗口控制布局，除跟随系统主题修复外不扩大平台行为变更。",
      "`agent_reported`: 本任务不接入新的原生 OS 主题 API；优先使用 WebView 已支持的 `prefers-color-scheme`，保持跨平台实现简洁。",
      "`agent_reported`: 用量显示与远端计费一致性属于任务 `T-20260620165245-f6d23db9`，不在本任务的主题/窗口范围内实现。"
    ],
    "full_ref": ".vibehub/tasks/T-20260620165245-165f9e8f/runs/R-20260620165245-c3b6eb90/outputs/output.md",
    "key_decisions": [
      "`agent_reported`: 主题修复的核心是把主题应用逻辑集中，并增加 `matchMedia` change listener。",
      "`agent_reported`: 手动 `light` / `dark` 优先级高于系统变化；只有 `auto` 会响应 OS 外观变化。",
      "`agent_reported`: macOS 标题栏工作必须平台隔离，避免影响 Windows/Linux。"
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
phase: align
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
phase: align
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

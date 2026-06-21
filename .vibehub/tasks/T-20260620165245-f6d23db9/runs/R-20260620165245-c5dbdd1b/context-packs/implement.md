# Context Pack: Implement

Task: T-20260620165245-f6d23db9
Run: R-20260620165245-c5dbdd1b
Phase: Implement
Generated at: 2026-06-21T02:46:40Z
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
      "`hard_observed`: 当前任务已进入 `implement` 阶段，并读取了 implement context pack。",
      "`hard_observed`: 已在 `src/stores/appStore.ts` 增加集中主题解析：新增 `effectiveTheme`、`resolveEffectiveTheme`、`applyTheme`、`watchSystemTheme`。",
      "`hard_observed`: `auto` 模式现在注册 `matchMedia('(prefers-color-scheme: dark)')` 的 `change` listener；系统主题变化时只在当前配置仍为 `auto` 时更新 DOM `dark` class。",
      "`hard_observed`: `watchSystemTheme` 同时兼容现代 `addEventListener('change', ...)` 和旧 WebKit `addListener(...)`。",
      "`hard_observed`: 已删除 `src/main.tsx` 中只识别 `config?.theme === 'dark'` 的独立 dark mode effect，避免覆盖 auto 模式。",
      "`hard_observed`: 已更新 `src/components/Header.tsx`，主题按钮基于 `effectiveTheme` 显示 sun/moon，并在 macOS user agent 下为左侧 traffic lights 预留空间。",
      "`hard_observed`: 已更新 `src-tauri/src/main.rs`，macOS 平台在保留原生 traffic lights 的同时设置 `tauri::TitleBarStyle::Overlay`，减少独立原生标题栏带来的顶部框感。",
      "### diff_summary",
      "`agent_reported`: 主题系统从“初始化/切换时一次性读系统偏好”改为“集中 resolver + effective theme + 系统变化监听”。",
      "`agent_reported`: 应用入口不再直接操作根节点 `dark` class，避免和 store 中的 auto 逻辑互相覆盖。",
      "`agent_reported`: Header 不再把配置值 `auto` 当成浅色/非暗色，而是显示当前实际生效主题。",
      "`agent_reported`: macOS 启动路径增加 overlay titlebar style，配合前端 header 左侧留白，让原生红黄绿控件更像嵌入应用表面。"
    ],
    "full_ref": ".vibehub/tasks/T-20260620165245-165f9e8f/runs/R-20260620165245-c3b6eb90/outputs/output.md",
    "key_decisions": [
      "`agent_reported`: 保持用户配置 `theme` 为 `light | dark | auto`，另用 `effectiveTheme` 表达当前实际外观，避免把 `auto` 破坏成手动值。",
      "`agent_reported`: Header 的快速切换仍在手动 light/dark 之间切换；设置页继续负责选择 auto。",
      "`agent_reported`: macOS 使用 `TitleBarStyle::Overlay` 而不是关闭所有原生控件，保留红黄绿窗口按钮。"
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
    "task_id": "T-20260620165245-165f9e8f",
    "title": "Add desktop auto dark mode and macOS integrated titlebar",
    "active_capabilities": [],
    "shared_files": []
  }
]
```

## File: .vibehub/tasks/T-20260620165245-f6d23db9/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: T-20260620165245-f6d23db9
title: Align usage display with remote billing statistics
mode: guided_drive
phase: implement
phase_status: active
created_at: 2026-06-20T16:52:45Z
created_by: vibehub
intent: Investigate Sub to API/new API usage and pricing-stat semantics, compare them with local Codex/OpenCode token sources, then update the VibeHub usage display to use a remote-aligned metric with clear labeling and fallback behavior.
acceptance_criteria:
- Research notes identify available remote usage/stat fields for Sub to API and new API style providers and document assumptions with evidence labels.
- Usage display primary number matches the remote billing/statistical unit as closely as possible instead of only local raw or non-cached tokens.
- UI labels clearly distinguish remote-aligned estimate, local observed tokens, cache effects, and unavailable remote data.
- Existing local Codex/OpenCode usage details remain available for diagnostics.
- Relevant Rust/TypeScript tests or focused build checks pass.
dependencies: []
intake:
  batch_id: intake-20260620165245
  split_confidence: high
  suggested_order: 1
  total_tasks: 2
  source_message: 目前这个用量显示总感觉还是不太对劲。我是希望它能够跟我的远端保持一致的，就是远端主要用来计价的，一般来说的那个统计呃统计数字。我的远端要么是Sub to API，要么是new API这种。然后我希望你这一次做了丰富且全面的调查之后再实行。其次就是需要给Vibehub添加一个就是自动变深色模式的功能。无论是Windows电脑还是Mac电脑。其次就是目前在Mac电脑上面，我觉得还是不够美观，因为它上面它有一个那个框。它不是就是它最顶上它有个框一样的，而不是直接在应用上我们自己，然后包含它Mac的三个操作，就是红黄绿的三个点这样子。而上面一个框这样，好奇怪哦，很违和。不过修改的时候记得不要影响到其他平台、其他系统。
  split_reason: The usage accounting model and desktop window/theme behavior are independently deliverable, testable, and likely touch different subsystems. The usage task also requires dedicated research before implementation.
```

## File: .vibehub/tasks/T-20260620165245-f6d23db9/runs/R-20260620165245-c5dbdd1b/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: T-20260620165245-f6d23db9
run_id: R-20260620165245-c5dbdd1b
mode: guided_drive
phase: implement
phase_status: active
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

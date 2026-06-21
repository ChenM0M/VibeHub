# Context Pack: Align

Task: T-20260620165245-165f9e8f
Run: R-20260620165245-c3b6eb90
Phase: Align
Generated at: 2026-06-21T02:32:19Z
Source commit: 751d88c

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

- None available.

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
phase_status: active
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
task_id: "T-20260620165245-165f9e8f"
run_id: "R-20260620165245-c3b6eb90"
mode: "guided_drive"
phase: "align"
phase_status: "active"
created_at: "2026-06-20T16:52:45Z"
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

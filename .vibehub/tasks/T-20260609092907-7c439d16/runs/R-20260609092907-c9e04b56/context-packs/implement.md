# Context Pack: Implement

Task: T-20260609092907-7c439d16
Run: R-20260609092907-c9e04b56
Phase: Implement
Generated at: 2026-06-09T09:44:50Z
Source commit: b6abdf3

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
      "`user_confirmed`: 用户重新确认项目页定位是“项目状态中台 / project command map”，不是删减成极简详情页；目标是保留中台能力，但重排信息层级、阅读路径和视觉语法。",
      "`hard_observed`: 重建 `src/components/ProjectDetailBoard.tsx`，主视图改为“项目身份 + 最近提交/动态 + active task 工作地图 + 右侧健康/下一步建议 + 下方结构/归档”的信息架构。",
      "`hard_observed`: 重建 `src/components/ProjectStructureExplorer.tsx`，从伪脑图/拖拽缩放改为模块索引、目录树、选中对象详情三栏联动阅读器。",
      "`hard_observed`: 更新 `src/components/VibehubCockpitDialog.tsx`，接入新的主视图和结构浏览器，并将全局推荐命令/prompt grid 限制到 settings 详情内，避免每个详情抽屉都重复显示全局工具区。",
      "`hard_observed`: 更新 `src/locales/en.json`、`src/locales/zh.json`、`src/locales/zh-TW.json`，补齐新主视图文案。"
    ],
    "full_ref": ".vibehub/tasks/T-20260531153015-3a283c0b/runs/R-20260531153015-bd42f253/outputs/output.md",
    "key_decisions": [
      "`user_confirmed`: 中台能力不砍掉；问题不是信息多，而是所有信息同等重量堆叠导致不可读。",
      "`agent_reported`: 主页面以 active task 为主语，旧 dashboard 型信息改为摘要、入口或对象详情，不再平铺成多组大卡片。",
      "`agent_reported`: 视觉语法改用列表、分隔线、状态点、紧凑指标和右侧 rail，减少圆角玻璃卡片、框中框和重复 prompt 区域。",
      "`agent_reported`: 项目结构能力保留，但首版以可信的文件系统扫描阅读器为主，不默认展示伪交互脑图。"
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

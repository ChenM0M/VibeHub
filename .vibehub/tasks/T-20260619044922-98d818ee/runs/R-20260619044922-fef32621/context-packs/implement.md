# Context Pack: Implement

Task: T-20260619044922-98d818ee
Run: R-20260619044922-fef32621
Phase: Implement
Generated at: 2026-06-21T05:52:15Z
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
    "capability": "align",
    "completed": [
      "`user_confirmed`: 本轮目标不是让项目详情页顶部“刷新/同步检测”按钮执行更新，而是让项目详情页设置面板里的“修复/更新 Agent 工具”按钮确实可用，至少能弥补本次 `vibehub-cli` -> `vibehub` CLI 调用问题。",
      "`hard_observed`: 保持刷新按钮为只读 reload；设置页更新按钮仍走 `vibehubSyncAgentAdapters` / `sync_agent_adapters`。",
      "`hard_observed`: `sync_agent_adapters` 现在会识别旧版 VibeHub 生成的 adapter/skill/command 文件，即使旧项目没有 `generated_hashes` 记录，也会安全覆盖明显由 VibeHub 生成的旧文件。",
      "`hard_observed`: 新增回归测试覆盖旧 `.agents/skills/vibehub-sync/SKILL.md` 中 `vibehub-cli sync <project_path>` 会被设置页更新按钮背后的 sync 逻辑改为 `vibehub sync <project_path>`。",
      "`hard_observed`: app 主二进制 headless CLI 补齐 `next-action`、`output-lint`、`validate-task`、`sync-adapters`、`adapter-status`、`archive`、`start-intake --stdin` 等生成指令会引用的动作，并对 `finish` / `advance` / `archive` 加回显式用户确认检查。",
      "`hard_observed`: 已实际运行 `cargo run -p vibehub -- sync-adapters /Users/chenm0m/LocalRepo/VibeHub`，结果为 created=0、updated=3、skipped=150、conflicts=0，验证设置页更新按钮背后的后端入口可写入本项目 adapter 更新。",
      "`hard_observed`: `crates/vibehub-cli` 现在同时产出 `vibehub` 和 legacy `vibehub-cli` 两个 bin，同一份 `main.rs` 支持 `vibehub ...`、`vibehub-cli ...` 以及 `vibehub vibehub-cli ...` 兼容入口。",
      "`hard_observed`: Tauri app bundle 内 `Contents/MacOS/vibehub` 可执行 `--help`、`--version`、`output-lint`、`vibehub-cli status` 和 `sync-adapters --dry-run`，覆盖 Homebrew cask `binary \"#{appdir}/VibeHub.app/Contents/MacOS/vibehub\", target: \"vibehub\"` 路径。",
      "`hard_observed`: `target/aarch64-apple-darwin/release/vibehub` standalone release 二进制可执行 `--version`、`output-lint` 和 `sync-adapters --dry-run`，覆盖 macOS portable-style binary path；Windows/Linux portable release 使用同一 Tauri app binary headless dispatch。",
      "`hard_observed`: 发布版本已 bump 到 `2.0.0-pre.17`，覆盖 npm、Tauri、Rust workspace crate versions 和 lockfiles。"
    ],
    "full_ref": ".vibehub/tasks/T-20260616062024-a8e8bdc2/runs/R-20260616062024-6b02149c/outputs/output.md",
    "key_decisions": [
      "`user_confirmed`: 更新行为应由项目详情页设置面板的更新按钮触发，不由刷新/同步检测按钮触发。",
      "`agent_reported`: 对旧项目采用“识别 VibeHub 生成物并覆盖”的兼容策略，只覆盖 `.agents/skills/vibehub-*`、`.claude/commands/vibehub-*`、`.opencode/commands/vibehub-*`、`.vibehub/adapters/generated/`、平台约束/索引等明确受管路径；普通用户文件仍按冲突处理。",
      "`agent_reported`: 继续保留对外部手工改动的保护：不符合旧 VibeHub 生成特征的文件仍返回 conflict。",
      "`agent_reported`: 对新安装路径统一推荐 `vibehub`；对旧脚本、旧 agent 文档、旧便携式二进制调用保留 `vibehub-cli` 兼容别名。"
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
    "task_id": "T-20260609092907-7c439d16",
    "title": "Harden VibeHub agent protocol and CLI routing",
    "active_capabilities": [],
    "shared_files": []
  },
  {
    "task_id": "T-20260616062024-a8e8bdc2",
    "title": "test cli dispatch",
    "active_capabilities": [],
    "shared_files": []
  }
]
```

## File: .vibehub/tasks/T-20260619044922-98d818ee/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: T-20260619044922-98d818ee
title: Display local Codex and OpenCode workspace usage insights
mode: evidence_drive
phase: implement
phase_status: active
created_at: 2026-06-19T04:49:22Z
created_by: vibehub
```

## File: .vibehub/tasks/T-20260619044922-98d818ee/runs/R-20260619044922-fef32621/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: T-20260619044922-98d818ee
run_id: R-20260619044922-fef32621
mode: evidence_drive
phase: implement
phase_status: active
created_at: 2026-06-19T04:49:22Z
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

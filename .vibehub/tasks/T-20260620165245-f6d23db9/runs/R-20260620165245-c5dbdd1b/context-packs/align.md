# Context Pack: Align

Task: T-20260620165245-f6d23db9
Run: R-20260620165245-c5dbdd1b
Phase: Align
Generated at: 2026-06-20T16:52:45Z
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

```json
[
  {
    "capability": "implement",
    "completed": [
      "`user_confirmed`: 用户反馈上一版更新后项目详情页检测不到任何 AI 用量，要求继续修复并补足测试。",
      "`hard_observed`: 已按 VibeHub 规则读取 current/current-context/handoff/hard-rules/protocol，并在用户说“继续”后运行 `vibehub sync /Users/chenm0m/LocalRepo/VibeHub`。",
      "`hard_observed`: `vibehub sync` 后当前任务仍为 `T-20260619044922-98d818ee`，run 为 `R-20260619044922-fef32621`，phase 为 `implement`。",
      "`hard_observed`: 真实本机数据对照显示 `/Users/chenm0m/LocalRepo/VibeHub` 在 Codex `~/.codex/state_5.sqlite` 中有 56 条记录、`487305514` total tokens；在 Codex `~/.codex/sqlite/state_5.sqlite` 中有 52 条记录、`460226943` total tokens。",
      "`hard_observed`: 真实本机数据对照显示 OpenCode `~/.local/share/opencode/opencode.db` 中有 24 条匹配 session，`tokens_input=2868011`、`tokens_output=206679`、`tokens_reasoning=87055`、`tokens_cache_read=48391296`。",
      "`hard_observed`: 已修复 `src-tauri/src/local_agent_usage.rs`：Codex/OpenCode 不再只使用第一个存在的候选数据库；现在会扫描所有候选库，选择匹配记录最多、最近更新时间更新的最佳库。",
      "`hard_observed`: 已修复路径匹配：不再只做 `cwd = project_path` / `worktree = project_path` 精确匹配；现在支持 normalized path、canonical path、尾斜杠规整、大小写兼容和项目子路径匹配，同时避免把 `/repo/app` 误匹配到 `/repo/application`。",
      "`hard_observed`: 用户指定样本 `mind2realistic` 的 VibeHub 配置路径为 `/Users/chenm0m/ArchiveRepo/mind2realistic`，但 Codex/OpenCode 本地用量记录路径为 `/Users/chenm0m/LocalRepo/mind2realistic`；这是项目搬家后历史用量仍按旧 cwd/worktree 保存导致的无记录。",
      "`hard_observed`: 已新增受控同名项目 fallback：当配置路径无精确/子路径命中，且本地用量库里项目目录名唯一匹配时，使用该同名历史路径并在来源 warnings 中说明 fallback 路径；若同名路径不唯一则拒绝 fallback，避免误归因。",
      "`hard_observed`: 已保留上一轮非缓存主口径：顶部 `AI 用量` 使用 `non_cached_total_tokens`，详情页同时展示非缓存与含缓存总量。",
      "`hard_observed`: 已新增回归测试覆盖“第一个候选 Codex DB 为空但后续 DB 有记录”、“OpenCode/Codex 子路径能匹配但同前缀兄弟目录不匹配”、“配置路径搬家后可回落到唯一同名历史路径”。",
      "`hard_observed`: `cargo fmt --manifest-path src-tauri/Cargo.toml` 已运行。",
      "`hard_observed`: `cargo test --manifest-path src-tauri/Cargo.toml` 通过，14 个测试全部通过，其中 `local_agent_usage` 6 个测试全部通过。",
      "`hard_observed`: `npm run build` 通过，TypeScript 与 Vite production build 成功。",
      "`hard_observed`: `vibehub validate /Users/chenm0m/LocalRepo/VibeHub` 通过，implement required outputs 均已找到。",
      "`hard_observed`: `vibehub output-lint /Users/chenm0m/LocalRepo/VibeHub T-20260619044922-98d818ee` 通过，issue_count 为 0。",
      "`user_confirmed`: 用户确认非缓存口径解释符合预期，并要求提交、推送、发版。",
      "`hard_observed`: 已将 prerelease 版本从 `2.0.0-pre.19` bump 到 `2.0.0-pre.20`，准备创建 `v2.0.0-pre.20` tag。",
      "`hard_observed`: 版本 bump 后重新运行 `cargo test --manifest-path src-tauri/Cargo.toml`、`npm run build`、`vibehub output-lint /Users/chenm0m/LocalRepo/VibeHub T-20260619044922-98d818ee`，均通过。"
    ],
    "full_ref": ".vibehub/tasks/T-20260619044922-98d818ee/runs/R-20260619044922-fef32621/outputs/output.md",
    "key_decisions": [
      "`agent_reported`: 对多个 Codex/OpenCode 候选数据库采用“选择最佳匹配库”而不是“全部合并”，避免把旧库和当前库重复计入。",
      "`agent_reported`: 最佳库选择规则为：优先匹配记录数更多；记录数相同时优先最近更新时间更晚。",
      "`agent_reported`: 路径匹配放到 Rust 侧过滤，避免新增 `rusqlite` 的 `functions` feature，也让匹配逻辑更容易测试。",
      "`agent_reported`: 对 moved project 的历史用量只做唯一 basename fallback，不做自由模糊搜索；这样能恢复 `mind2realistic` 这类搬家项目，同时避免多个同名 repo 时错误合并。",
      "`inferred`: 用户遇到“任何用量都检测不到”时，最危险的回归点是候选库顺序、路径别名/子路径和精确匹配过窄；因此这些场景必须进入单元测试。"
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
  }
]
```

## File: .vibehub/tasks/T-20260620165245-f6d23db9/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: "T-20260620165245-f6d23db9"
title: "Align usage display with remote billing statistics"
mode: "guided_drive"
phase: "align"
phase_status: "active"
created_at: "2026-06-20T16:52:45Z"
created_by: vibehub
```

## File: .vibehub/tasks/T-20260620165245-f6d23db9/runs/R-20260620165245-c5dbdd1b/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: "T-20260620165245-f6d23db9"
run_id: "R-20260620165245-c5dbdd1b"
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

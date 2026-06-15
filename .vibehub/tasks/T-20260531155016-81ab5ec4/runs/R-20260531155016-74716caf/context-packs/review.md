# Context Pack: Review

Task: T-20260531155016-81ab5ec4
Run: R-20260531155016-74716caf
Phase: Review
Generated at: 2026-05-31T16:30:45Z
Source commit: b6abdf3

## Instructions

Use this context only for the current phase.
Do not mark state.yaml completed.
Report files read, commands run, decisions made, and unresolved risks.

## Capability Output Schema

```json
{
  "required_fields": [
    "summary",
    "concerns",
    "gate_pass",
    "risk_review"
  ],
  "optional_fields": [
    "references",
    "related_threads"
  ],
  "produces": [
    "review_summary"
  ],
  "consumes": [
    "diff",
    "validation_result"
  ],
  "parallel_safe": true,
  "custom": false
}
```

## Prior Outputs Summary

```json
[
  {
    "capability": "review",
    "completed": [
      "`hard_observed`: Review phase 已生成 context pack，并运行 `vibehub review /Users/chenm0m/LocalRepo/VibeHub` 产出 `phases/review.md`、`evidence/changed-files.txt`、`evidence/diff.patch`。",
      "`hard_observed`: Core/CLI 实现已完成：`next-action [intent...]`、`output-lint [task_id]`、`validate-task <task_id>`、CLI finish/advance/archive 确认门。",
      "`hard_observed`: Adapter/skill 暴露已完成：新增 `vibehub-output-lint`，更新 `vibehub-next-action`、`vibehub-finish`、`vibehub-advance`、`vibehub-archive`、`vibehub-validate`，并生成 Codex/Claude/OpenCode 入口。",
      "`hard_observed`: Command index 已分层为 Core Loop、State Transitions、Diagnostics、Capabilities。",
      "`hard_observed`: 当前 implementation output 曾通过 `vibehub validate` 和 `vibehub output-lint`；随后已 finish implement 并 advance 到 review。",
      "## Diff Summary",
      "`hard_observed`: 新增 `crates/vibehub-core/src/vibehub/output_lint.rs`，提供 output.md 质量 lint，覆盖 missing output、missing section、missing evidence labels、测试矛盾和过期实现状态。",
      "`hard_observed`: 扩展 `crates/vibehub-core/src/vibehub/next_action.rs`，加入 intent routing、`matched_intent` 字段、output-lint 路由和确认旗标后的 transition CLI 建议。",
      "`hard_observed`: 扩展 `crates/vibehub-core/src/vibehub/phase.rs`，新增 `validate_phase_for_task`，支持不切换 current pointer 的 task-scoped validation，并增加回归测试。",
      "`hard_observed`: 扩展 `crates/vibehub-cli/src/main.rs`，新增 `validate-task`、`output-lint`，并让 `finish`、`advance`、`archive` 缺少 `--confirmed-by-user` 时拒绝执行。",
      "`hard_observed`: 扩展 `crates/vibehub-core/src/vibehub/agent_adapter.rs`，生成新 skill/commands、确认门说明、validate+lint 收口要求和分层 command index。",
      "`hard_observed`: 更新 `.vibehub/skills.registry.yaml`、`docs/vibehub-skills-registry-v1.md`、`AGENTS.md`、`CLAUDE.md`、`.vibehub/adapters/protocol.md` 及平台生成文件。",
      "## Verdict",
      "`hard_observed`: gate_pass。Core 单元测试全量通过，CLI 编译/测试通过，关键 CLI smoke 均通过。",
      "`inferred`: 本轮改造满足用户提出的主要方向：减少长提示依赖、把 Agent 操作循环封装为可查询/可组合工具、强化多任务安全、强化状态流转确认、提升 output 质量约束。"
    ],
    "full_ref": ".vibehub/tasks/T-20260531155016-81ab5ec4/runs/R-20260531155016-74716caf/outputs/output.md",
    "key_decisions": [
      "`inferred`: 使用 CLI 确认旗标做硬保护，比仅在 skill 文案中提醒更可靠。",
      "`inferred`: 使用 `validate-task` 解决多 active task 的 current pointer 误验证风险，避免必须先 switch。",
      "`inferred`: 使用 output-lint 补足 validate 的弱点：validate 只判断 required output 是否存在，lint 检查内容质量和证据卫生。",
      "`inferred`: 保留完整 skill 集，但用 command index 分层让 Agent 首屏只关注 Core Loop。"
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

## File: .vibehub/tasks/T-20260531155016-81ab5ec4/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: T-20260531155016-81ab5ec4
title: Refine VibeHub agent operating loop and tool orchestration
mode: guided_drive
phase: review
phase_status: completed
created_at: 2026-05-31T15:50:16Z
created_by: vibehub
```

## File: .vibehub/tasks/T-20260531155016-81ab5ec4/runs/R-20260531155016-74716caf/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: T-20260531155016-81ab5ec4
run_id: R-20260531155016-74716caf
mode: guided_drive
phase: review
phase_status: completed
created_at: 2026-05-31T15:50:16Z
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

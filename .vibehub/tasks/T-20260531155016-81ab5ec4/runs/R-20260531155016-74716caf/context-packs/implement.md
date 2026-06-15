# Context Pack: Implement

Task: T-20260531155016-81ab5ec4
Run: R-20260531155016-74716caf
Phase: Implement
Generated at: 2026-05-31T16:05:47Z
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
    "capability": "plan",
    "completed": [
      "`user_confirmed`: 用户要求开新 Task 做 Agent Operator Loop 重构；允许一次性推进实现，但结束前需要先说明，不要直接 complete/archive。",
      "`hard_observed`: 当前 Task 为 `T-20260531155016-81ab5ec4`，Run 为 `R-20260531155016-74716caf`，当前 phase 为 `plan` / `active`。",
      "`hard_observed`: 已新增 `crates/vibehub-core/src/vibehub/next_action.rs`，提供机器可读的 `NextActionReport`，包含 `action`、`skill`、`cli`、`reason`、`confidence`、`operating_loop`、`routing_table`、`warnings`。",
      "`hard_observed`: 已在 `crates/vibehub-cli/src/main.rs` 增加 `vibehub next-action <project>` / `next_action` / `route` 路由，并加入 CLI help。",
      "`hard_observed`: 已在 `crates/vibehub-core/src/vibehub/agent_adapter.rs` 中把 VibeHub Operating Loop、routing shortcut、`next-action` skill 生成逻辑暴露给 AGENTS、CLAUDE、共享 protocol、Codex skills、Claude commands、OpenCode commands。",
      "`hard_observed`: 已在 `.vibehub/skills.registry.yaml` 和 `docs/vibehub-skills-registry-v1.md` 注册并记录 `vibehub-next-action`。",
      "`hard_observed`: 已运行 `sync-adapters`，生成 `.agents/skills/vibehub-next-action/SKILL.md`、`.vibehub/adapters/generated/codex/vibehub-next-action.md`、`.claude/commands/vibehub-next-action.md`、`.opencode/commands/vibehub-next-action.md`，并更新共享协议与命令索引。",
      "`hard_observed`: `vibehub next-action /Users/chenm0m/LocalRepo/VibeHub` 已返回可用 JSON；当前建议为 `validate_phase` / `vibehub-validate`，原因是当前 phase active 且 output.md 已存在。",
      "`agent_reported`: 本轮没有运行 `vibehub finish`、`vibehub advance` 或 `vibehub archive`；按用户要求停在可说明、可验证、未归档的状态。"
    ],
    "full_ref": ".vibehub/tasks/T-20260531155016-81ab5ec4/runs/R-20260531155016-74716caf/outputs/output.md",
    "key_decisions": [
      "`inferred`: 这次不继续增加长篇规则，而是新增一个轻量 operator router：Agent 不确定下一步时先查 `vibehub next-action`，由状态投影返回推荐 skill 和 CLI。",
      "`inferred`: Operating Loop 保持短规则，承担“约束”；routing table 和 next-action JSON 承担“自由组合工具的选择建议”，避免把所有判断都塞进 prompt。",
      "`inferred`: `next-action` 只读状态，不写 canonical state；状态变化仍必须通过 `start`、`sync`、`validate`、`finish`、`advance`、`recover` 等 CLI 完成。",
      "`inferred`: `finish` / `advance` / `archive` 继续保持用户确认边界，避免再次出现用户未确认就自动 complete/archive 的体验问题。",
      "## Implementation Plan",
      "`agent_reported`: Step 1: 在 core 层实现 Operating Loop 与 routing table 的结构化输出。",
      "`agent_reported`: Step 2: 根据 `status::read_cockpit_status` 做推荐决策：未初始化、无 active task、状态 warning、context stale/missing、active phase missing output、active phase ready for validation、completed phase awaiting user transition、unclear state。",
      "`agent_reported`: Step 3: 把 `next-action` 暴露到 CLI 和 help，使 Agent 可以用一个低 token 命令获得下一步建议。",
      "`agent_reported`: Step 4: 把 `vibehub-next-action` 加入 skills registry 和 adapter generator，让 Codex/Claude/OpenCode 都有同一套入口。",
      "`agent_reported`: Step 5: 运行 adapter 同步，检查生成 skill/command/protocol/command-index 是否包含新入口。",
      "`agent_reported`: Step 6: 运行格式化、单元测试、CLI 实测和当前 phase validate。",
      "## Validation Plan",
      "`hard_observed`: 已运行 `cargo fmt`。",
      "`hard_observed`: 已运行 `cargo test --target-dir /private/tmp/vibehub-target -p vibehub-core --offline next_action`，结果 3 passed。",
      "`hard_observed`: 已运行 `cargo test --target-dir /private/tmp/vibehub-target -p vibehub-core --offline`，结果 199 passed。",
      "`hard_observed`: 已运行 `cargo test --target-dir /private/tmp/vibehub-target -p vibehub-cli --offline`，结果 0 tests, ok。",
      "`hard_observed`: 已运行 `cargo run --target-dir /private/tmp/vibehub-target -p vibehub-cli --offline -- next-action /Users/chenm0m/LocalRepo/VibeHub`，确认 JSON 输出包含推荐 action/skill/cli/reason/confidence/operating_loop/routing_table/warnings。",
      "`hard_observed`: 已运行 `cargo run --target-dir /private/tmp/vibehub-target -p vibehub-cli --offline -- sync-adapters /Users/chenm0m/LocalRepo/VibeHub`，结果 created 4, updated 9, skipped 135, conflicts 1。",
      "`hard_observed`: 首次运行 `vibehub validate` 时发现 current pointer 指向另一个 UI task；随后运行 `vibehub switch ... T-20260531155016-81ab5ec4` 切回本任务。",
      "`hard_observed`: 切回本任务后已运行 `vibehub validate /Users/chenm0m/LocalRepo/VibeHub`，当前 plan 输出通过：required outputs `implementation_plan`、`validation_plan`、`context_plan` 均 found，missing_outputs 为空。",
      "`hard_observed`: 已运行 `vibehub handoff /Users/chenm0m/LocalRepo/VibeHub`，handoff complete，missing_required_sections 为空，source_output_path 指向本任务 output.md。",
      "## Affected Files",
      "`hard_observed`: `crates/vibehub-core/src/vibehub/next_action.rs`",
      "`hard_observed`: `crates/vibehub-core/src/vibehub/mod.rs`",
      "`hard_observed`: `crates/vibehub-cli/src/main.rs`",
      "`hard_observed`: `crates/vibehub-core/src/vibehub/agent_adapter.rs`",
      "`hard_observed`: `.vibehub/skills.registry.yaml`",
      "`hard_observed`: `docs/vibehub-skills-registry-v1.md`",
      "`hard_observed`: `.agents/skills/vibehub-next-action/SKILL.md`",
      "`hard_observed`: `.vibehub/adapters/generated/codex/vibehub-next-action.md`",
      "`hard_observed`: `.claude/commands/vibehub-next-action.md`",
      "`hard_observed`: `.opencode/commands/vibehub-next-action.md`",
      "`hard_observed`: `.vibehub/adapters/protocol.md`",
      "`hard_observed`: `AGENTS.md`",
      "`hard_observed`: `CLAUDE.md`",
      "`hard_observed`: `.codex/vibehub/command-index.md`",
      "`hard_observed`: `.codex/vibehub/constraints.md`",
      "`hard_observed`: `.claude/vibehub/command-index.md`",
      "`hard_observed`: `.claude/vibehub/constraints.md`",
      "`hard_observed`: `.opencode/vibehub/command-index.md`",
      "`hard_observed`: `.opencode/vibehub/constraints.md`",
      "`hard_observed`: `.vibehub/tasks/T-20260531155016-81ab5ec4/runs/R-20260531155016-74716caf/outputs/output.md`"
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
phase: plan
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
phase: plan
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

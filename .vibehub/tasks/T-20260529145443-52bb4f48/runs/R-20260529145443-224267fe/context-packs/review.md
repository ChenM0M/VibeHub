# Context Pack: Review

Task: T-20260529145443-52bb4f48
Run: R-20260529145443-224267fe
Phase: Review
Generated at: 2026-05-29T15:02:26Z
Source commit: 3252e1f

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
    "capability": "implement",
    "completed": [
      "`hard_observed`: 修改 `crates/vibehub-core/src/vibehub/agent_adapter.rs` 中 `default_command_body()` 函数 (行 1262-1337)，覆盖以下高优问题：",
      "### #1 vip-enabled start: CLI-first 指令",
      "`vibehub-start` 的 task 描述从 \"Convert the user's request into a VibeHub task draft...\" 改为明确的 CLI 调用指令：",
      "`Create one or more VibeHub tasks by running the CLI: vibehub start <project_path> <mode> <title>`",
      "新增: `DO NOT manually create .vibehub/tasks/ directories or edit state.yaml directly — the CLI handles task registration, pointer files, and state updates automatically.`",
      "新增 fallback: `If CLI is unavailable, ask the user to start the task from the VibeHub cockpit UI`",
      "### #3 + #5 vip-enabled continue/recover: 更详细的 phase 指引和 Stop Condition",
      "`vibehub-continue` task 内容扩展为包含 phase-aware 指引：",
      "`Read current.md for the current phase and run. Check workflow.yaml capabilities.<phase>.required_fields for phase-specific outputs. Read the context pack.`",
      "新增 Stop Condition: `Stop when: (1) all required phase outputs are written to output.md, (2) phase acceptance criteria are met, or (3) if blocked, report the blocker and write partial output.`",
      "`vibehub-sync` task 新增 Stop Condition",
      "`vibehub-recover` task 新增分析步骤清单",
      "`vibehub-review` task 新增协议合规检查",
      "### #6 Output requirements 样板代码精简",
      "所有 skill 中的原始 `Output requirements` block（15 行变更文件+文件读取+命令+测试+证据标签+风险+交接注释）替换为 2 行简洁引用：",
      "`Output: Write the phase output following the contract in .vibehub/adapters/protocol.md to .vibehub/tasks/<task_id>/runs/<run_id>/outputs/output.md. Include sections: Completed, Not Yet Done, Key Decisions Made, Files Changed, Files Reportedly Read, Commands Run, Tests Run, Context Still Needed, Warnings, Next Session Should. Use evidence labels.`",
      "### #12 AGENTS.md 命令发现机制澄清",
      "`build_static_protocol()` 更新 `## Command Namespace` section 新增 `## Key Rules for VibeHub Commands` subsection，明确每个核心命令的正确用法",
      "### #4 protocol.md 补充 phase validation 说明",
      "`build_adapter_protocol()` 新增说明：phase-specific required fields 定义在 workflow.yaml，advance 时的 needs_action 含义",
      "### GUI prompt templates (#10, #11, #14)",
      "`templates/prompts/en/new-task.md`: 13 行 → 含 CLI 指令和输出合约引用",
      "`templates/prompts/en/sync.md`: 9 行 → 含 4 步工作流和停止条件",
      "同步更新 `zh-CN` 和 `zh-TW` 版本",
      "### 代码质量",
      "`cargo test -p vibehub-core` 全 192 测试通过",
      "`vibehub sync-adapters` 成功生成 106 个文件更新到 6 个 agent tools"
    ],
    "full_ref": ".vibehub/tasks/T-20260529145443-52bb4f48/runs/R-20260529145443-224267fe/outputs/output.md",
    "key_decisions": [
      "`inferred`: 将 `Output requirements` 从每个 skill 中完全移除不可行，因为 agent 可能不读 protocol.md。折中方案：保留 2 行引用（列出 10 个 section 名称 + 证据标签），比原来的 15 行大幅精简。",
      "`inferred`: `registry_command_body()` 也做了同样的精简，保持一致性。",
      "`inferred`: Phase 映射表存在于 `phase.rs:294-317`（`required_output_to_section_keys`），但将其逐字包含在每个 skill 中不实际。更好的方式是在 Stop Condition 中提示 agent 去检查 workflow.yaml。"
    ]
  }
]
```

## Neighbors

```json
[
  {
    "task_id": "T-20260529090147-9df98d71",
    "title": "收口 v2.0 发布前状态并归档旧 VibeHub 结构",
    "active_capabilities": [],
    "shared_files": []
  }
]
```

## File: .vibehub/tasks/T-20260529145443-52bb4f48/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: T-20260529145443-52bb4f48
title: Fix AI Agent Skill/Prompt Quality Issues
mode: evidence_drive
phase: implement
phase_status: completed
created_at: 2026-05-29T14:54:43Z
created_by: vibehub
```

## File: .vibehub/tasks/T-20260529145443-52bb4f48/runs/R-20260529145443-224267fe/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: T-20260529145443-52bb4f48
run_id: R-20260529145443-224267fe
mode: evidence_drive
phase: implement
phase_status: completed
created_at: 2026-05-29T14:54:43Z
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

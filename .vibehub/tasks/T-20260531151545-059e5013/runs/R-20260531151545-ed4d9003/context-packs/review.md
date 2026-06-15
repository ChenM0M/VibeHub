# Context Pack: Review

Task: T-20260531151545-059e5013
Run: R-20260531151545-ed4d9003
Phase: Review
Generated at: 2026-05-31T15:27:56Z
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
    "capability": "implement",
    "completed": [
      "`user_confirmed`: 用户反馈当前项目的 Agent 约束不稳定：任务拆分、状态更新、流程控制、前端交互层和存储信息层经常没有被主动执行。",
      "`hard_observed`: 已创建任务 `T-20260531151545-059e5013`，运行 `R-20260531151545-ed4d9003`，使用 `guided_drive` 以控制 token 成本。",
      "`hard_observed`: 已完成 align 和 plan 阶段，并推进到 implement 阶段。",
      "`hard_observed`: 发现并修复 `advance` 后 `.vibehub/agent-view/current.md` 仍显示旧 phase 的问题；现在 `current.md` 与 `vibehub status` 均显示 `implement active`。",
      "`hard_observed`: 修改 `crates/vibehub-core/src/vibehub/phase.rs`，确保 phase advance 写入 event/projection 后再生成 agent-view。",
      "`hard_observed`: 新增 `phase::tests::agent_view_current_matches_advanced_phase` 回归测试。",
      "`hard_observed`: 修改 `crates/vibehub-core/src/vibehub/agent_adapter.rs`，在共享协议中加入短 Routing Shortcuts，并强化 `vibehub-start` / `vibehub-continue` / `vibehub-sync` 的触发描述。",
      "`hard_observed`: 修改前端 prompt 模板：复杂需求先 `vibehub start-intake`，单一交付再 `vibehub start`；继续、刷新、漂移、阶段不清楚时先 sync。",
      "`hard_observed`: 更新 `.vibehub/skills.registry.yaml` 与 `docs/vibehub-skills-registry-v1.md`，让 registry 描述与生成 skill 保持一致。",
      "`hard_observed`: 已运行 `sync-adapters`，更新 AGENTS/CLAUDE、protocol、Codex skills、Claude/OpenCode commands 和命令索引。"
    ],
    "full_ref": ".vibehub/tasks/T-20260531151545-059e5013/runs/R-20260531151545-ed4d9003/outputs/output.md",
    "key_decisions": [
      "`inferred`: 选择短路由表和 skill 描述，而不是继续加长 AGENTS.md，降低 token 成本并提高触发率。",
      "`inferred`: 状态一致性问题优先在 CLI/core 层修复，避免前端或 Agent 读取 stale agent-view。",
      "`inferred`: start/sync/continue 是 Agent 最容易漏掉的入口，因此本轮优先强化这三个高频入口。"
    ]
  }
]
```

## Neighbors

```json
[]
```

## File: .vibehub/tasks/T-20260531151545-059e5013/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: T-20260531151545-059e5013
title: Improve agent workflow adherence and low-token skill orchestration
mode: guided_drive
phase: implement
phase_status: completed
created_at: 2026-05-31T15:15:45Z
created_by: vibehub
```

## File: .vibehub/tasks/T-20260531151545-059e5013/runs/R-20260531151545-ed4d9003/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: T-20260531151545-059e5013
run_id: R-20260531151545-ed4d9003
mode: guided_drive
phase: implement
phase_status: completed
created_at: 2026-05-31T15:15:45Z
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

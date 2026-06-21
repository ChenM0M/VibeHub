# Context Pack: Review

Task: T-20260620165245-f6d23db9
Run: R-20260620165245-c5dbdd1b
Phase: Review
Generated at: 2026-06-21T03:05:02Z
Source commit: 751d88c

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
      "`hard_observed`: 已读取当前 implement context pack，当前任务为 `T-20260620165245-f6d23db9`，阶段为 `implement`。",
      "`hard_observed`: 已在 `src-tauri/src/local_agent_usage.rs` 新增 `AgentUsagePrimaryMetric`，并在 `LocalAgentUsageOverview` 上新增 `primary_metric` 字段。",
      "`hard_observed`: 已实现 cost/quota-first 的主指标选择逻辑：OpenCode 有本地 `cost` 字段时主指标为 `cost`；没有 cost/quota 时回退到 `non_cached_total_tokens`；无匹配记录时为 `unavailable`。",
      "`hard_observed`: 已补充 Rust 单元测试，覆盖 OpenCode cost 主指标、Codex-only token fallback、无记录 unavailable。",
      "`hard_observed`: 已同步 `src/types/index.ts` 的 TypeScript 类型。",
      "`hard_observed`: 已更新项目详情页 `src/components/ProjectDetailBoard.tsx`，顶部 AI 用量卡片改为显示账单口径主指标。",
      "`hard_observed`: 已更新 `src/components/VibehubCockpitDialog.tsx`，Agent Usage 详情顶部新增账单口径区块，展示主指标、类型、来源、可信度、是否估算和说明；原 token 分项与来源诊断保留。",
      "`hard_observed`: 已更新 `src/locales/zh.json`、`src/locales/zh-TW.json`、`src/locales/en.json` 的账单口径相关文案。",
      "### diff_summary",
      "`agent_reported`: 本地 Agent 用量从“主显示非缓存 token 数”改成“主显示远端计费语义更接近的 cost/quota-first 指标”。",
      "`agent_reported`: 当前没有接入真实远端账单 API；OpenCode cost 被标记为 local recorded / estimated，Codex-only 明确显示 token fallback，避免把 token 冒充账单金额。",
      "`agent_reported`: 诊断信息仍完整保留：非缓存 token、含缓存 token、source_count、各来源 cost/token/recent/warnings 都还在详情页。"
    ],
    "full_ref": ".vibehub/tasks/T-20260620165245-f6d23db9/runs/R-20260620165245-c5dbdd1b/outputs/output.md",
    "key_decisions": [
      "`agent_reported`: `primary_metric.kind` 预留 `cost`、`quota`、`token_fallback`、`unavailable`，但当前本地数据只有 OpenCode cost 和 token fallback 两类真实来源。",
      "`agent_reported`: OpenCode cost 使用 `USD` 显示，并标记为 `local_recorded` / `estimated=true`，避免误导为远端 invoice。",
      "`agent_reported`: Codex 本地记录没有 cost/quota 时不再把 token 当成“账单金额”，而是明确显示 `Token fallback`。"
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
phase_status: completed
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

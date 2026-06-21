# Context Pack: Plan

Task: T-20260620165245-f6d23db9
Run: R-20260620165245-c5dbdd1b
Phase: Plan
Generated at: 2026-06-20T17:02:00Z
Source commit: 751d88c

## Instructions

Use this context only for the current phase.
Do not mark state.yaml completed.
Report files read, commands run, decisions made, and unresolved risks.

## Capability Output Schema

```json
{
  "required_fields": [
    "steps",
    "validation_plan",
    "affected_files"
  ],
  "optional_fields": [
    "risks",
    "references"
  ],
  "produces": [
    "implementation_plan"
  ],
  "consumes": [
    "alignment_summary",
    "research_output"
  ],
  "parallel_safe": true,
  "custom": false
}
```

## Prior Outputs Summary

```json
[
  {
    "capability": "align",
    "completed": [
      "`user_confirmed`: 用户澄清目标不是专门贴合 Sub to API 或 New API 私有实现，而是按最上游 GPT/OpenAI 这类官方账单/用量规范来显示，Sub to API/New API 只是可能的远端路径。",
      "`user_confirmed`: 用户确认自动深色模式无需再做；现有问题是设置被固定为浅色而不是跟随系统。",
      "`hard_observed`: OpenAI 官方 Cookbook 示例说明默认 usage/cost dashboards 对多数用户足够；自定义监控可使用 Completions Usage API 和 Costs API。",
      "`hard_observed`: OpenAI Completions Usage API 示例字段包含 `input_tokens`、`output_tokens`、`input_cached_tokens`；Costs API 示例字段包含 `amount.value`、`amount.currency`、`line_item`，并按日期汇总 `amount_value`。",
      "`hard_observed`: OpenAI OpenAPI 规格包含 `/organization/costs` 和 `/organization/usage/completions`；cost result 的 `amount` 表示关联货币的 monetary value，`currency` 为 ISO-4217 小写代码。",
      "`hard_observed`: OpenAI prompt caching 文档说明 `usage.prompt_tokens_details.cached_tokens` 会展示缓存命中 token；缓存降低输入 token 成本，但输出仍正常计算。",
      "`hard_observed`: New API 源码 `model/log.go` 的消费日志包含 `Quota`、`PromptTokens`、`CompletionTokens`；`GetLogsStat`/`GetLogsSelfStat` 返回 `quota`、`rpm`、`tpm`，其中 `quota` 是计价/扣费主统计。",
      "`hard_observed`: 当前 VibeHub 项目详情页主指标使用 `localAgentUsage.non_cached_total_tokens`，详情页再展示非缓存 token 与含缓存 token。",
      "`hard_observed`: 当前 VibeHub Gateway stats 有 `total_cost`，但 proxy 侧现实现按请求体估算输入 token、输出 token 置 0 后用本地配置费率计算；这不是远端真实账单 amount。",
      "`inferred`: 规范主口径应优先显示远端/官方可计费金额或额度消耗，token 作为解释和 fallback，而不是把 local total tokens 或 non-cached tokens 作为顶部主数字。"
    ],
    "full_ref": ".vibehub/tasks/T-20260620165245-f6d23db9/runs/R-20260620165245-c5dbdd1b/outputs/output.md",
    "key_decisions": [
      "`user_confirmed`: 需求主轴从“跟某个中转平台一致”调整为“跟最上游官方账单/用量规范一致”。",
      "`agent_reported`: 主指标采用 cost/quota-first，而不是上一轮的 non-cached-token-first。",
      "`agent_reported`: New API 的 `quota` 类字段应被视为可计费额度消耗；OpenAI 官方成本 API 的 `amount.value + currency` 是更标准的金额口径。",
      "`agent_reported`: Token breakdown 保留在详情中，且 cached token 只解释成本差异，不再构成顶部主指标。"
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
    "active_capabilities": [
      "align"
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
task_id: T-20260620165245-f6d23db9
title: Align usage display with remote billing statistics
mode: guided_drive
phase: align
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
phase: align
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

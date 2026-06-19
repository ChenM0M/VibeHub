# Context Pack: Plan

Task: T-20260619044922-98d818ee
Run: R-20260619044922-fef32621
Phase: Plan
Generated at: 2026-06-19T09:43:26Z
Source commit: 7a60edf

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
    "capability": "research",
    "completed": [
      "`user_confirmed`: 新建任务，目标是在项目详细页展示每个工作区/项目的本地 Codex 与 OpenCode token 用量等 AI Agent 使用信息，并支持点击进入详情查看。",
      "`hard_observed`: 已创建 VibeHub task `T-20260619044922-98d818ee` / run `R-20260619044922-fef32621`，模式为 `evidence_drive`，当前 canonical phase 为 `align`。",
      "`hard_observed`: 已通过 `vibehub gates` 确认 `research` 可 claim，并通过 `vibehub claim ... research` 进入 align + research 并行能力；`vibehub status` 显示当前任务 active_capabilities 为 `align, research`。",
      "`hard_observed`: 已读取 align 与 research context pack；research 输出要求为 `sources`, `risks`, `open_questions`。",
      "`hard_observed`: 已调查本机 Codex 存储。Codex 官方 manual 说明 `CODEX_HOME` 默认是 `~/.codex`，包含 config、auth、logs、sessions、skills、standalone package metadata；本机实际存在 `~/.codex/state_5.sqlite`、`~/.codex/logs_2.sqlite`、`~/.codex/session_index.jsonl`、`~/.codex/sessions/.../*.jsonl`。",
      "`hard_observed`: Codex `state_5.sqlite` 的 `threads` 表包含 `cwd`, `rollout_path`, `tokens_used`, `model_provider`, `model`, `created_at_ms`, `updated_at_ms` 等字段，可按项目路径聚合 token 总量。",
      "`hard_observed`: Codex rollout JSONL 中存在 `payload.info.total_token_usage` 和 `payload.info.last_token_usage`，字段包含 `input_tokens`, `cached_input_tokens`, `output_tokens`, `reasoning_output_tokens`, `total_tokens`，可用于拆分 token 类型。",
      "`hard_observed`: 本机 Codex 对 `/Users/chenm0m/LocalRepo/VibeHub` 的聚合验证结果为 53 个 threads、`tokens_used=460711508`；最近样本 rollout 的 `total_token_usage.total_tokens` 与 `threads.tokens_used` 对齐。",
      "`hard_observed`: 已调查本机 OpenCode 存储。项目内存在 `opencode.json` 和 `.opencode/commands`, `.opencode/vibehub`；全局配置在 `~/.config/opencode/opencode.jsonc`；主数据在 `~/.local/share/opencode/opencode.db`；桌面端另有 `~/Library/Application Support/ai.opencode.desktop/*`。",
      "`hard_observed`: OpenCode `opencode.db` 的 `session` 表包含 `project_id`, `directory`, `workspace_id`, `path`, `agent`, `model`, `cost`, `tokens_input`, `tokens_output`, `tokens_reasoning`, `tokens_cache_read`, `tokens_cache_write`, `time_created`, `time_updated`；`project` 表包含 `worktree`；可按 project/worktree 聚合。",
      "`hard_observed`: 本机 OpenCode 对 `/Users/chenm0m/LocalRepo/VibeHub` 的聚合验证结果为 22 个 sessions、`cost=6.4124`、`tokens_input=2716308`、`tokens_output=198539`、`tokens_reasoning=85235`、`tokens_cache_read=48174080`。",
      "`hard_observed`: 项目详情页顶部指标区由 `src/components/ProjectDetailBoard.tsx` 渲染，当前有 Active tasks、Changed files、Warnings 三个 MetricCell；点击详细面板由 `DashboardDetail` / `DetailDrawer` 管理。",
      "`inferred`: 用户截图蓝框位置最适合新增一个可点击的 “AI 用量 / Agent 用量” 指标块；点击后打开新的 detail panel，如 `agentUsage`，显示 Codex 与 OpenCode 分项、最近会话、来源路径与可用性状态。",
      "### Intent",
      "`user_confirmed`: 让 VibeHub 在项目详细页基于本机 Codex/OpenCode 数据展示每个工作区或项目的 AI Agent 用量信息，重点是 token 用量、会话数量、最近使用、模型/agent 来源、成本或缓存 token 等可观测指标。",
      "`inferred`: 首版应优先做只读本地聚合，不修改 Codex/OpenCode 数据，不依赖网络，不上传会话内容。",
      "### Scope",
      "`hard_observed`: 数据源范围包括本机 Codex `~/.codex` / `CODEX_HOME` / `CODEX_SQLITE_HOME` 及 OpenCode `~/.local/share/opencode`, `~/.config/opencode`, 项目内 `.opencode` / `opencode.json`。",
      "`inferred`: 后端新增只读读取器，前端新增项目详情页指标卡和详情抽屉；适配当前 Tauri invoke 模式。",
      "`inferred`: 读取器需要按当前 project path 匹配 Codex `threads.cwd` 和 OpenCode `project.worktree` / `session.directory` / `session.path`。",
      "### Success Criteria / Acceptance Criteria",
      "`inferred`: 项目详情页顶部指标区出现一个清晰的 AI Agent 用量入口，展示合计 token 或最近周期 token，并能点击进入详情。",
      "`inferred`: 详情页至少展示 Codex 与 OpenCode 两块：可用/不可用状态、数据源路径、session/thread 数、总 token、input/output/reasoning/cache token、最近更新时间、最近若干会话摘要。",
      "`inferred`: 若本机没有安装或没有数据，应显示 “未发现数据源” 或 “无本项目记录”，而不是报错或阻塞项目详情页。",
      "`inferred`: 不读取或展示 auth token、access_token、refresh_token、消息正文、用户 prompt 正文；只读 schema/聚合字段和安全摘要。",
      "`inferred`: 后端读取 SQLite/JSONL 失败时返回 warnings，并允许 UI 降级展示部分来源。",
      "`inferred`: 验证需要至少覆盖后端聚合单元测试，以及前端详情页基本渲染/无数据状态。",
      "### Non-goals",
      "`inferred`: 不在本任务首版实现云端 OpenAI/ChatGPT 企业用量统计或账单 API 对账。",
      "`inferred`: 不修改 Codex/OpenCode 的本地数据库或配置文件。",
      "`inferred`: 不展示完整会话正文、工具输出正文、diff 内容或密钥。",
      "`inferred`: 不把 Codex 与 OpenCode 的 token 口径强行合并成单一精确账单数字；不同工具字段定义不同，应保留来源维度。",
      "### Autonomy Level",
      "`user_confirmed`: 当前阶段先做对齐和调研，不进入实现。",
      "`agent_reported`: 后续进入 plan/implement 前，需要用户确认展示指标口径和 UI 入口位置；若用户不确认，建议采用 “顶部指标卡 + 详情抽屉” 作为默认方案。"
    ],
    "full_ref": ".vibehub/tasks/T-20260619044922-98d818ee/runs/R-20260619044922-fef32621/outputs/output.md",
    "key_decisions": [
      "`agent_reported`: 推荐将功能命名为 `AgentUsage` 或 `LocalAgentUsage`，语义覆盖 Codex 与 OpenCode，避免绑定某一个工具。",
      "`agent_reported`: 推荐后端在 `src-tauri` 或 `crates/vibehub-core` 中实现只读聚合逻辑；当前 Cargo 依赖尚未包含 SQLite 读取库，计划阶段需决定添加 `rusqlite` 或通过现有轻量方式读取。",
      "`agent_reported`: 推荐 UI 放在 `ProjectDetailBoard` 顶部三指标右侧或改成四指标栅格；卡片点击打开 detail drawer，不放到 Settings。",
      "`agent_reported`: 推荐详情抽屉新增 `DashboardDetail` 值，如 `agentUsage`，展示两个 provider section，而不是复用 `evidence` 或 `settings`。",
      "`agent_reported`: 推荐数据读取只输出聚合和安全摘要；任何读取 session JSONL 时只解析 `payload.info.total_token_usage`，不输出消息内容。",
      "## Research Notes",
      "`agent_reported`: 本节汇总本轮 Codex/OpenCode 本地存储调研、可实现性判断、技术方案、风险与开放问题；所有敏感凭据和消息正文均不纳入输出。",
      "### Sources",
      "`hard_observed`: OpenAI Codex manual fetched to `/private/tmp/openai-docs-cache/codex-manual.md`; relevant sections: config/state locations, auth/session caching, app settings/profile usage insights, environment variables.",
      "`hard_observed`: Local Codex SQLite schemas: `~/.codex/state_5.sqlite` tables `threads`, `agent_jobs`, `agent_job_items`; `~/.codex/logs_2.sqlite` table `logs`.",
      "`hard_observed`: Local Codex JSONL session files under `~/.codex/sessions/2026/.../rollout-*.jsonl`.",
      "`hard_observed`: Local OpenCode SQLite schema: `~/.local/share/opencode/opencode.db` tables `session`, `message`, `part`, `project`, `workspace`.",
      "`hard_observed`: Local OpenCode config/data locations: project `opencode.json`, project `.opencode`, global `~/.config/opencode`, data `~/.local/share/opencode`, desktop support `~/Library/Application Support/ai.opencode.desktop`.",
      "`hard_observed`: VibeHub UI/source files: `src/components/ProjectDetailBoard.tsx`, `src/components/VibehubCockpitDialog.tsx`, `src/types/index.ts`, `src/services/tauri.ts`, `src-tauri/src/main.rs`.",
      "### Feasibility",
      "`hard_observed`: Codex total token aggregation is feasible from `threads.tokens_used` grouped by `cwd`.",
      "`hard_observed`: Codex token breakdown is feasible for sessions whose `rollout_path` JSONL contains `total_token_usage`; some observed Codex rollout files may have no `total_token_usage`, so parser must tolerate null.",
      "`hard_observed`: OpenCode token/cost aggregation is directly feasible from `session` columns and `project` join.",
      "`inferred`: Cross-workspace display is feasible by iterating VibeHub configured projects/workspaces and running the same local readers per project path.",
      "`inferred`: Accuracy should be described as local observed usage, not authoritative billing. Codex app profile/lifetime insights exist in product UI, but local DB schemas are not a public stable API.",
      "### Proposed Technical Implementation",
      "`inferred`: Add backend data model:",
      "`LocalAgentUsageOverview { project_path, generated_at, codex, opencode, warnings }`",
      "`CodexUsage { available, source, threads, total_tokens, input_tokens?, output_tokens?, reasoning_tokens?, cached_input_tokens?, recent_threads }`",
      "`OpenCodeUsage { available, source, sessions, cost, tokens_input, tokens_output, tokens_reasoning, tokens_cache_read, tokens_cache_write, recent_sessions }`",
      "`inferred`: Codex reader:",
      "Resolve `CODEX_HOME` default `~/.codex`; resolve `CODEX_SQLITE_HOME` default `CODEX_HOME` unless config says otherwise.",
      "Open `state_5.sqlite` read-only and query `threads where cwd = project_path`.",
      "Sum `tokens_used`; collect recent thread metadata without prompt/message body.",
      "Optionally parse each `rollout_path` JSONL and keep the last `payload.info.total_token_usage` to split token types.",
      "`inferred`: OpenCode reader:",
      "Resolve XDG-style data dir `~/.local/share/opencode/opencode.db`; on macOS also treat desktop support files as secondary UI state, not primary token source.",
      "Query `session` joined to `project` by `project_id`, matching `project.worktree`, `session.directory`, or `session.path` to project path.",
      "Sum cost and token columns; collect recent session title/model/agent/timestamps.",
      "`inferred`: Tauri/API:",
      "Add command `read_local_agent_usage(project_path)` or include it in cockpit overview loading.",
      "Add `tauriApi.vibehubReadLocalAgentUsage(projectPath)` and TypeScript interfaces in `src/types/index.ts`.",
      "Load in `VibehubCockpitDialog` alongside existing dashboard data; pass into `ProjectDetailBoard`.",
      "`inferred`: UI:",
      "Add fourth MetricCell in header metric grid with label `AI 用量` and value like compact total tokens (`461M` / `54M`) or source count.",
      "Make metric clickable, `onOpenDetail('agentUsage')`.",
      "In drawer: top summary, Codex/OpenCode source cards, token breakdown table, recent sessions list, warnings/data-source section."
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
phase: research
phase_status: completed
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
phase: research
phase_status: completed
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

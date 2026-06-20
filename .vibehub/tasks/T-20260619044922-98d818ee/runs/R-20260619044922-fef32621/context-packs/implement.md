# Context Pack: Implement

Task: T-20260619044922-98d818ee
Run: R-20260619044922-fef32621
Phase: Implement
Generated at: 2026-06-20T16:03:32Z
Source commit: 7f4def4

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
      "`user_confirmed`: 用户要求完成计划制定与 output 内容，结束计划阶段并进入执行阶段推进。",
      "`agent_reported`: 已形成实现计划，默认采用“AI 用量”作为顶部卡片文案，顶部显示本地观察到的 lifetime total token，详情页展示 Codex/OpenCode 分项 token、可用性、最近记录、数据源与 warnings。",
      "`agent_reported`: 已把多系统支持纳入计划：至少覆盖 macOS、Windows、Linux；Codex 通过 `CODEX_HOME` / `CODEX_SQLITE_HOME` 与 home fallback 解析，OpenCode 通过 XDG/macOS Application Support/Windows AppData 候选路径和可降级 warnings 解析。",
      "`hard_observed`: Implement 阶段已新增只读本地 Agent 用量读取器 `src-tauri/src/local_agent_usage.rs`，支持 Codex 与 OpenCode 聚合、跨 macOS/Linux/Windows 路径候选、warnings 降级和 fixture 单元测试。",
      "`hard_observed`: 已新增 Tauri command `vibehub_read_local_agent_usage`，并在前端 `tauriApi.vibehubReadLocalAgentUsage(projectPath)` 中暴露。",
      "`hard_observed`: 已在项目详细页顶部 metrics 区新增可点击 `AI 用量` 卡片；点击会打开 `agentUsage` 详情抽屉。",
      "`hard_observed`: 已新增 `AgentUsageTabContent`，展示总 token、来源数、Codex/OpenCode 分项、成本、最近记录、数据源路径和 warnings。",
      "`hard_observed`: 已补充 `zh` / `zh-TW` / `en` 文案。",
      "`hard_observed`: 发布前版本已从 `2.0.0-pre.17` 推进到 `2.0.0-pre.18`，覆盖 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` 和 `Cargo.lock`。",
      "`hard_observed`: 已确认远端已有 `v2.0.0-pre.17` tag；本轮当前最新内容将通过新 tag `v2.0.0-pre.18` 触发 release workflow。",
      "### Implement Diff Summary",
      "`hard_observed`: 后端新增 `rusqlite` bundled 依赖并更新 `Cargo.lock`，通过 read-only SQLite 查询 Codex `threads` 和 OpenCode `session`/`project` 表；Codex JSONL 仅解析 `payload.info.total_token_usage`，不读取或返回消息正文。",
      "`hard_observed`: 前端随 dashboard 初始化并行读取本地用量，失败时只把 usage 置空，不阻塞项目详情页。",
      "`hard_observed`: UI 首屏 metrics 从 3 个扩展为响应式 4 个；新增卡片使用 compact token 值，详情抽屉宽度设为 `max-w-4xl`。",
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
      "`agent_reported`: 计划阶段决定使用 `rusqlite` read-only 查询 SQLite；若实现时新增依赖需要联网下载，则按 Codex sandbox 规则请求批准。为跨平台稳定性优先考虑 bundled SQLite feature。",
      "`agent_reported`: 计划阶段决定首版只做 Project Detail 入口；Workspace 层聚合留作后续扩展。",
      "`agent_reported`: 计划阶段决定不要求 OpenCode CLI 在 PATH；只读本地数据文件。",
      "`hard_observed`: 实现阶段采用 `AI 用量` 作为中文卡片和抽屉标题；英文为 `AI usage`。",
      "`hard_observed`: 顶部卡片显示 Codex + OpenCode 本地观测总 token；详情保留 cache/read/write、input/output/reasoning/cost 等来源分项。",
      "`hard_observed`: 单个来源不可用或无匹配记录时以 warnings/empty 状态展示，不让整个 dashboard 失败。",
      "`hard_observed`: 由于 `v2.0.0-pre.17` 已指向旧 HEAD，本轮发版采用 `v2.0.0-pre.18`，避免移动既有 release tag。",
      "## Implementation Plan",
      "`agent_reported`: 本节是 plan 阶段的 implementation_plan，覆盖 required fields: `steps`, `validation_plan`, `affected_files`。",
      "### steps",
      "`agent_reported`: Step 1 - 后端数据模型与路径解析：新增 `src-tauri/src/local_agent_usage.rs`，定义 `LocalAgentUsageOverview`、`AgentUsageSourceSummary`、`TokenBreakdown`、`AgentUsageRecentItem`、`AgentUsageSourceStatus` 等 serde 类型；实现 `resolve_codex_paths()` 和 `resolve_opencode_paths()`，按 macOS / Linux / Windows 候选路径查找本地数据。",
      "`agent_reported`: Step 2 - Codex 只读聚合：读取 `CODEX_SQLITE_HOME` 或 `CODEX_HOME` 下 `state_5.sqlite`，按 `threads.cwd = project_path` 聚合 `tokens_used`、thread count、最近更新时间和最近 thread 摘要；安全解析 `rollout_path` 指向的 JSONL，只读取 `payload.info.total_token_usage` 作为可选 token breakdown，不输出消息正文。",
      "`agent_reported`: Step 3 - OpenCode 只读聚合：读取 `~/.local/share/opencode/opencode.db`、XDG data dir、macOS Application Support 旁路候选和 Windows AppData 候选中的 SQLite DB；通过 `session` left join `project`，匹配 `project.worktree` / `session.directory` / `session.path`，聚合 `cost`、`tokens_input`、`tokens_output`、`tokens_reasoning`、`tokens_cache_read`、`tokens_cache_write`、sessions 和最近 session 摘要。",
      "`agent_reported`: Step 4 - Tauri command：在 `src-tauri/src/commands.rs` 增加 `read_local_agent_usage(project_path)` 或 `vibehub_read_local_agent_usage(project_path)`；在 `src-tauri/src/main.rs` 注册 invoke handler。命令必须只读、失败返回 warnings，不因单个来源失败阻塞整个响应。",
      "`agent_reported`: Step 5 - 前端类型和 API：在 `src/types/index.ts` 添加 `LocalAgentUsageOverview` 等接口；在 `src/services/tauri.ts` 增加 `vibehubReadLocalAgentUsage(projectPath)`。",
      "`agent_reported`: Step 6 - 数据加载：在 `src/components/VibehubCockpitDialog.tsx` 中随 dashboard 加载项目用量；维护 loading/error/warnings 状态，传入 `ProjectDetailBoard` 和 `DetailDrawer`。",
      "`agent_reported`: Step 7 - 顶部指标卡：在 `src/components/ProjectDetailBoard.tsx` 顶部 metrics 区加入第四个可点击 `MetricCell`，label 使用 `AI 用量` / fallback `AI usage`，value 使用 compact total tokens；无数据时显示 `--` 或 `0`，tone 可在 warnings 时轻量提示。",
      "`agent_reported`: Step 8 - 详情抽屉：扩展 `DashboardDetail` 为 `agentUsage`；新增 `AgentUsageTabContent`，展示总览、Codex/OpenCode 两个来源、token breakdown、最近记录、数据源路径、warnings。保留紧凑工具型布局，不做营销式说明。",
      "`agent_reported`: Step 9 - i18n 与格式化：在 `src/locales/zh.json`、`src/locales/zh-TW.json`、`src/locales/en.json` 添加必要文案；实现 token/cost/time compact formatter 或复用现有格式工具，避免按钮/卡片文本溢出。",
      "`agent_reported`: Step 10 - 测试夹具：添加 Rust 单元测试，使用临时目录创建最小 Codex/OpenCode SQLite/JSONL fixture，覆盖有数据、无数据、缺字段、路径不存在、多系统路径候选、敏感表不读取等场景。",
      "`agent_reported`: Step 11 - 构建验证与 UI 验证：运行 Rust/TS/Vite 构建；启动 dev server 或 app 页面后用浏览器/截图检查项目详情页顶部指标和详情抽屉无重叠、无文本溢出、无数据状态可读。",
      "### validation_plan",
      "`agent_reported`: Rust unit tests: 针对 `local_agent_usage` 的 Codex/OpenCode parser/aggregator/path resolver fixture 测试；目标覆盖聚合字段、recent items、warnings 和 privacy guard。",
      "`agent_reported`: TypeScript/build: 运行 `npm run build`，确保新增类型、Tauri API 调用和 React 组件编译通过。",
      "`agent_reported`: Rust compile: 运行 `cargo test -p vibehub --bin vibehub --no-run` 或更窄的 `cargo test -p vibehub local_agent_usage`；若新增依赖下载受限，按 sandbox escalation 流程请求批准。",
      "`agent_reported`: Visual QA: 启动 `npm run dev` 后用浏览器检查项目详情页；桌面宽度确认四指标卡对齐，移动宽度确认 metrics 不溢出；点击 “AI 用量” 能打开详情抽屉。",
      "`agent_reported`: Privacy QA: 用 `rg -n \"access_token|refresh_token|auth.json|message.data|part.data\"` 检查实现没有读取或渲染敏感字段；fixture 中加入敏感列也不输出。",
      "`agent_reported`: Cross-platform reasoning: 单元测试以参数化路径覆盖 macOS/Linux/Windows 候选路径；真实 Windows/Linux 路径仍列为 residual risk，首版以候选解析和手动路径 warnings 降级。",
      "### affected_files",
      "`agent_reported`: `src-tauri/Cargo.toml` - 新增 SQLite 读取依赖，如 `rusqlite`。",
      "`agent_reported`: `src-tauri/src/local_agent_usage.rs` - 新增本地 Codex/OpenCode usage 聚合模块。",
      "`agent_reported`: `src-tauri/src/commands.rs` - 新增 Tauri command wrapper。",
      "`agent_reported`: `src-tauri/src/main.rs` - 注册 Tauri command。",
      "`agent_reported`: `src/types/index.ts` - 新增前端数据类型。",
      "`agent_reported`: `src/services/tauri.ts` - 新增 invoke API。",
      "`agent_reported`: `src/components/VibehubCockpitDialog.tsx` - 加载/传递 local agent usage，新增 detail drawer 分支。",
      "`agent_reported`: `src/components/ProjectDetailBoard.tsx` - 新增顶部 “AI 用量” 指标卡。",
      "`agent_reported`: `src/locales/zh.json`, `src/locales/zh-TW.json`, `src/locales/en.json` - 新增 UI 文案。",
      "`agent_reported`: `src-tauri/Cargo.lock` / workspace lockfiles - 若新增依赖会更新。",
      "`agent_reported`: `.vibehub/tasks/T-20260619044922-98d818ee/runs/R-20260619044922-fef32621/outputs/output.md` - 记录计划、验证和阶段推进结果。",
      "### Plan Risks",
      "`inferred`: `rusqlite` 新依赖可能需要网络下载；如果 sandbox 阻止，需要请求升级权限。",
      "`inferred`: Codex/OpenCode local DB schema 可能随版本变化，必须用字段探测和 warnings 降级，不能 panic。",
      "`inferred`: 当前仓库有多个 active tasks 和未归属 docs 文件，实施时只触碰本任务范围内文件。",
      "`inferred`: UI 顶部 metrics 从 3 个变 4 个，需检查小屏布局不拥挤。",
      "`inferred`: Windows/Linux OpenCode 默认路径需要通过候选路径和 fixture 覆盖，真实机器验证仍可能发现差异。",
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

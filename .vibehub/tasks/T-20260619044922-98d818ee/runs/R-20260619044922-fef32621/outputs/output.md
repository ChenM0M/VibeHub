# Display local Codex and OpenCode workspace usage insights - implement output

## Completed

- `user_confirmed`: 用户确认调整 AI 用量口径，避免当前显示比直觉中的实际用量偏大。
- `hard_observed`: 已运行 `vibehub sync /Users/chenm0m/LocalRepo/VibeHub`；当前任务仍为 `T-20260619044922-98d818ee`，当前阶段为 `implement`，active capability 为 `implement`。
- `hard_observed`: 已确认旧实现顶部卡片使用 `localAgentUsage.total_tokens`，后端总量为 Codex total + OpenCode total，且 OpenCode total 包含 `tokens_cache_read` / `tokens_cache_write`。
- `hard_observed`: 本机数据抽样显示 Codex 对 `/Users/chenm0m/LocalRepo/VibeHub` 有 55 个 thread、约 480.3M total tokens；可解析 rollout 中 cached input 约 455.5M。
- `hard_observed`: 本机 OpenCode 对 `/Users/chenm0m/LocalRepo/VibeHub` 有 24 个 session、约 51.6M total tokens，其中 cache read 约 48.4M。
- `hard_observed`: 已在后端 `LocalAgentUsageOverview`、`AgentUsageSourceSummary`、`AgentUsageRecentItem` 增加 `non_cached_total_tokens` 字段。
- `hard_observed`: 非缓存口径采用 `total - cached_input - cache_read - cache_write`，保留原始含缓存 `total_tokens` 供详情页展示。
- `hard_observed`: 项目详情页顶部 `AI 用量` 卡片已改为显示 `non_cached_total_tokens`，不再把缓存 token 作为主数字。
- `hard_observed`: `agentUsage` 详情抽屉已改为同时展示 `非缓存 Token` 与 `含缓存总量`；最近记录徽标也改为非缓存值。
- `hard_observed`: 已更新 `zh` / `zh-TW` / `en` 三套文案，补充非缓存与含缓存总量标签。
- `hard_observed`: 已补充 Rust fixture 单元测试断言，覆盖 Codex cached input 和 OpenCode cache read/write 被排除出非缓存主口径。
- `hard_observed`: 由于 `v2.0.0-pre.18` 已存在并指向旧提交，已将本次发版版本推进到 `2.0.0-pre.19`。
- `hard_observed`: `cargo test --manifest-path src-tauri/Cargo.toml local_agent_usage` 通过，2 个相关测试通过。
- `hard_observed`: `npm run build` 通过，TypeScript 与 Vite production build 成功。
- `hard_observed`: `vibehub validate /Users/chenm0m/LocalRepo/VibeHub` 通过，implement required outputs 均已找到。
- `hard_observed`: `vibehub output-lint /Users/chenm0m/LocalRepo/VibeHub T-20260619044922-98d818ee` 通过，issue_count 为 0。

## Not Yet Done

- `agent_reported`: 尚未在真实 Tauri 桌面窗口中点击项目详情页做视觉验证；本轮已完成类型、构建和后端聚合测试。
- `agent_reported`: 尚未新增最近 7 天 / 30 天筛选；当前仍是本地 observed lifetime 口径，只是主数字排除了缓存。
- `agent_reported`: 尚未在真实 Windows/Linux 机器验证默认路径；当前依赖候选路径与 warnings 降级。

## Key Decisions Made

- `user_confirmed`: 接受“顶部显示非缓存口径，详情保留含缓存总量”的方案。
- `agent_reported`: 不把当前功能命名成账单或成本统计，仍保持为本地 Agent 用量观察，避免暗示精确计费。
- `agent_reported`: 不删除含缓存总量，因为它仍是 Codex/OpenCode 本地记录的硬证据；只是不作为顶部主指标。
- `inferred`: 对 Codex，`cached_input_tokens` 已包含在 total/input 口径内；对 OpenCode，cache read/write 是独立字段。因此统一用 `total - cached_input - cache_read - cache_write` 作为非缓存主口径。

## Files Changed

- `hard_observed`: `src-tauri/src/local_agent_usage.rs` - 新增非缓存用量字段、计算逻辑和单元测试断言。
- `hard_observed`: `src/types/index.ts` - 同步新增 `non_cached_total_tokens` TypeScript 字段。
- `hard_observed`: `src/components/ProjectDetailBoard.tsx` - 顶部 `AI 用量` 卡片改用非缓存用量。
- `hard_observed`: `src/components/VibehubCockpitDialog.tsx` - 详情抽屉展示非缓存与含缓存总量，最近记录徽标改为非缓存值。
- `hard_observed`: `src/locales/zh.json`, `src/locales/zh-TW.json`, `src/locales/en.json` - 新增非缓存/含缓存文案。
- `hard_observed`: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `Cargo.lock` - 版本推进到 `2.0.0-pre.19`。
- `hard_observed`: `.vibehub/tasks/T-20260619044922-98d818ee/runs/R-20260619044922-fef32621/outputs/output.md` - 更新本阶段输出。
- `hard_observed`: VibeHub sync 自动更新了 `.vibehub/agent-view/*`、`.vibehub/state.yaml`、当前 run context pack、events/index 和新增 sync report；这些由 VibeHub CLI 生成，未手工编辑 canonical state。

## Files Reportedly Read

- `hard_observed`: `.vibehub/agent-view/current.md`
- `hard_observed`: `.vibehub/agent-view/current-context.md`
- `hard_observed`: `.vibehub/agent-view/handoff.md`
- `hard_observed`: `.vibehub/rules/hard-rules.md`
- `hard_observed`: `.vibehub/adapters/protocol.md`
- `hard_observed`: `.vibehub/workflow.yaml`
- `hard_observed`: `.agents/skills/vibehub-continue/SKILL.md`
- `hard_observed`: `.vibehub/tasks/T-20260619044922-98d818ee/runs/R-20260619044922-fef32621/context-packs/implement.md`
- `hard_observed`: `.vibehub/tasks/T-20260619044922-98d818ee/runs/R-20260619044922-fef32621/outputs/output.md`
- `hard_observed`: `src-tauri/src/local_agent_usage.rs`
- `hard_observed`: `src/types/index.ts`
- `hard_observed`: `src/components/ProjectDetailBoard.tsx`
- `hard_observed`: `src/components/VibehubCockpitDialog.tsx`
- `hard_observed`: `src/locales/zh.json`
- `hard_observed`: `src/locales/zh-TW.json`
- `hard_observed`: `src/locales/en.json`
- `hard_observed`: local Codex SQLite/JSONL metadata under `~/.codex`
- `hard_observed`: local OpenCode SQLite metadata under `~/.local/share/opencode/opencode.db`

## Commands Run

- `hard_observed`: `vibehub sync /Users/chenm0m/LocalRepo/VibeHub`
- `hard_observed`: `vibehub status /Users/chenm0m/LocalRepo/VibeHub`
- `hard_observed`: `sed -n ...` / `nl -ba ...` reads for VibeHub protocol files and relevant source files.
- `hard_observed`: `rg -n "agentUsage|local_agent_usage|LocalAgentUsage|AI 用量|tokens_used|cache_read|cached" src src-tauri -S`
- `hard_observed`: `rg -n "2\\.0\\.0-pre\\.18|2\\.0\\.0-pre\\.19" package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json Cargo.lock`
- `hard_observed`: `sqlite3 /Users/chenm0m/.codex/state_5.sqlite ...`
- `hard_observed`: `sqlite3 /Users/chenm0m/.local/share/opencode/opencode.db ...`
- `hard_observed`: `sqlite3 ... | node -e ...` to aggregate Codex rollout token breakdown without reading message text into output.
- `hard_observed`: `cargo fmt --manifest-path src-tauri/Cargo.toml`
- `hard_observed`: `cargo test --manifest-path src-tauri/Cargo.toml local_agent_usage`
- `hard_observed`: `npm run build`
- `hard_observed`: `vibehub validate /Users/chenm0m/LocalRepo/VibeHub`
- `hard_observed`: `vibehub output-lint /Users/chenm0m/LocalRepo/VibeHub T-20260619044922-98d818ee`
- `hard_observed`: `git status --short`
- `hard_observed`: `git diff --stat`

## Tests Run

- `hard_observed`: `cargo test --manifest-path src-tauri/Cargo.toml local_agent_usage` passed: 2 passed, 0 failed.
- `hard_observed`: `npm run build` passed: `tsc && vite build` completed successfully.
- `hard_observed`: `vibehub validate /Users/chenm0m/LocalRepo/VibeHub` passed: implement output status completed, missing_outputs empty.
- `hard_observed`: `vibehub output-lint /Users/chenm0m/LocalRepo/VibeHub T-20260619044922-98d818ee` passed: issue_count 0.
- `agent_reported`: No browser/Tauri visual QA was run in this turn.

## Context Still Needed

- `user_confirmed_needed`: 是否要继续增加时间窗口筛选，例如 lifetime / 30 天 / 7 天。
- `user_confirmed_needed`: 是否要把详情页最近记录也同时显示含缓存值，而不是只在来源汇总里显示。
- `agent_reported`: 真实 Windows/Linux 默认路径仍需后续机器验证。

## Warnings

- `hard_observed`: `vibehub sync` 报告 `needs_attention`，原因包括 `ownership_unavailable`、Git 工作区存在 VibeHub 状态之外观察到的未提交变更、Git HEAD 与 `last_seen_head` 不一致。
- `hard_observed`: 当前仓库仍有多个 active tasks；本轮只改当前任务相关的 AI 用量文件。
- `hard_observed`: `cargo test` 输出已有 warning：`vibehub-cli/src/main.rs` 被多个 bin target 使用，以及若干 gateway dead_code warning。
- `hard_observed`: `npm run build` 输出已有 warning：Baseline/Browserslist 数据过旧，以及 chunk size 超过 500 kB。
- `inferred`: Codex/OpenCode 本地 schema 不是稳定公共 API，未来版本变化仍可能需要字段探测或兼容。

## Next Session Should

- `agent_reported`: 如用户要继续完善，优先做 Tauri 桌面视觉验证，确认项目详情页顶部数值与详情抽屉文案无溢出。
- `agent_reported`: 如果要进一步贴近“实际消耗”，新增 lifetime / 30 天 / 7 天筛选，并在详情页标明当前时间窗口。
- `agent_reported`: 在进入 review/finish 前，由用户确认是否要运行 `vibehub finish` / `vibehub advance`；本轮未执行这些状态转换命令。

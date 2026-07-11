# Display local Codex and OpenCode workspace usage insights - implement output

## Completed

- `user_confirmed`: 用户反馈上一版更新后项目详情页检测不到任何 AI 用量，要求继续修复并补足测试。
- `hard_observed`: 已按 VibeHub 规则读取 current/current-context/handoff/hard-rules/protocol，并在用户说“继续”后运行 `vibehub sync /Users/chenm0m/LocalRepo/VibeHub`。
- `hard_observed`: `vibehub sync` 后当前任务仍为 `T-20260619044922-98d818ee`，run 为 `R-20260619044922-fef32621`，phase 为 `implement`。
- `hard_observed`: 真实本机数据对照显示 `/Users/chenm0m/LocalRepo/VibeHub` 在 Codex `~/.codex/state_5.sqlite` 中有 56 条记录、`487305514` total tokens；在 Codex `~/.codex/sqlite/state_5.sqlite` 中有 52 条记录、`460226943` total tokens。
- `hard_observed`: 真实本机数据对照显示 OpenCode `~/.local/share/opencode/opencode.db` 中有 24 条匹配 session，`tokens_input=2868011`、`tokens_output=206679`、`tokens_reasoning=87055`、`tokens_cache_read=48391296`。
- `hard_observed`: 已修复 `src-tauri/src/local_agent_usage.rs`：Codex/OpenCode 不再只使用第一个存在的候选数据库；现在会扫描所有候选库，选择匹配记录最多、最近更新时间更新的最佳库。
- `hard_observed`: 已修复路径匹配：不再只做 `cwd = project_path` / `worktree = project_path` 精确匹配；现在支持 normalized path、canonical path、尾斜杠规整、大小写兼容和项目子路径匹配，同时避免把 `/repo/app` 误匹配到 `/repo/application`。
- `hard_observed`: 用户指定样本 `mind2realistic` 的 VibeHub 配置路径为 `/Users/chenm0m/ArchiveRepo/mind2realistic`，但 Codex/OpenCode 本地用量记录路径为 `/Users/chenm0m/LocalRepo/mind2realistic`；这是项目搬家后历史用量仍按旧 cwd/worktree 保存导致的无记录。
- `hard_observed`: 已新增受控同名项目 fallback：当配置路径无精确/子路径命中，且本地用量库里项目目录名唯一匹配时，使用该同名历史路径并在来源 warnings 中说明 fallback 路径；若同名路径不唯一则拒绝 fallback，避免误归因。
- `hard_observed`: 已将顶部主口径调整为总用量：`primary_metric.tokens` 使用 `total_tokens`，详情页同时展示“总用量（含缓存）”和“非缓存部分”。
- `hard_observed`: 已新增回归测试覆盖“第一个候选 Codex DB 为空但后续 DB 有记录”、“OpenCode/Codex 子路径能匹配但同前缀兄弟目录不匹配”、“配置路径搬家后可回落到唯一同名历史路径”。
- `hard_observed`: `cargo fmt --manifest-path src-tauri/Cargo.toml` 已运行。
- `hard_observed`: `cargo test --manifest-path src-tauri/Cargo.toml` 通过，14 个测试全部通过，其中 `local_agent_usage` 6 个测试全部通过。
- `hard_observed`: `npm run build` 通过，TypeScript 与 Vite production build 成功。
- `hard_observed`: `vibehub validate /Users/chenm0m/LocalRepo/VibeHub` 通过，implement required outputs 均已找到。
- `hard_observed`: `vibehub output-lint /Users/chenm0m/LocalRepo/VibeHub T-20260619044922-98d818ee` 通过，issue_count 为 0。
- `user_confirmed`: 用户确认非缓存口径解释符合预期，并要求提交、推送、发版。
- `hard_observed`: 已将 prerelease 版本从 `2.0.0-pre.19` bump 到 `2.0.0-pre.20`，准备创建 `v2.0.0-pre.20` tag。
- `hard_observed`: 版本 bump 后重新运行 `cargo test --manifest-path src-tauri/Cargo.toml`、`npm run build`、`vibehub output-lint /Users/chenm0m/LocalRepo/VibeHub T-20260619044922-98d818ee`，均通过。
- `user_confirmed`: 用户反馈不希望主显示 USD/cost，因为本地 cost 不可靠；希望看到最直观的 `M tokens` 口径，并理解缓存/非缓存计算方式。
- `hard_observed`: 已更新 `src-tauri/src/local_agent_usage.rs`，`primary_metric` 改为总 token-first：即使 OpenCode 有 `cost` 字段，主指标仍为 `total_tokens`，cost 只留在来源诊断明细。
- `hard_observed`: 已更新 `src/components/ProjectDetailBoard.tsx`，项目详情指标从“账单口径”改为“Token 用量”，显示 `xx.xM tokens` / `K tokens` 等紧凑 token 文案。
- `hard_observed`: 已更新 `src/components/VibehubCockpitDialog.tsx`，Agent Usage 详情顶部从“账单口径”改为“Token 口径”，并展示 token 指标来源、可信度、是否估算和计算说明。
- `hard_observed`: 已更新 `src/types/index.ts` 以及 `src/locales/zh.json`、`src/locales/zh-TW.json`、`src/locales/en.json`，新增 `tokens` 主指标类型和 Token 口径文案。
- `hard_observed`: 本轮 `cargo test --manifest-path src-tauri/Cargo.toml local_agent_usage` 通过：9 passed, 0 failed。
- `hard_observed`: 本轮 `npm run build` 通过。
- `hard_observed`: 本轮 `cargo fmt --manifest-path src-tauri/Cargo.toml --check` 和 `git diff --check -- ...` 均通过。
- `hard_observed`: 用户进一步确认“实际用量总量”应为非缓存加缓存后，已将主指标从 `non_cached_total_tokens` 改为 `total_tokens`，并将来源卡片/最近记录的主展示同步为总 token。
- `hard_observed`: 总量口径调整后重新运行 `cargo test --manifest-path src-tauri/Cargo.toml local_agent_usage`、`npm run build`、`git diff --check -- ...`，均通过。
- `hard_observed`: 因 `v2.0.0-pre.20` 已存在且已推送到远端，本次待发布版本已推进到 `2.0.0-pre.21`。
- `hard_observed`: `2.0.0-pre.21` 发布前验证已重新运行：`cargo test --manifest-path src-tauri/Cargo.toml` 通过 17 个测试，`npm run build` 通过，`git diff --check` 通过。
- `hard_observed`: 诊断用户反馈的 Homebrew upgrade 卡在 pre21：本地 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`、lockfiles 和本地/远端 `v2.0.0-pre.22` tag 均已存在，GitHub release `v2.0.0-pre.22` 已发布且包含 arm64/x64 DMG。
- `hard_observed`: 远端 Homebrew tap `ChenM0M/homebrew-vibehub` 的 `Casks/vibehub.rb` 已是 `2.0.0-pre.22`，但本机 tap checkout 仍停在 commit `0b8da64` / `2.0.0-pre.21`，且 `brew config` 显示 `HOMEBREW_NO_AUTO_UPDATE: set`；因此 `brew upgrade` 使用旧 tap 误判已是最新。
- `hard_observed`: 已运行 `brew update` 更新 `chenm0m/vibehub` tap 到 commit `e46b3f9`，随后 `brew upgrade --cask vibehub` 成功将本机安装从 `2.0.0-pre.21` 升级到 `2.0.0-pre.22`；`/opt/homebrew/bin/vibehub --version` 现输出 `2.0.0-pre.22`。

## Not Yet Done

- `agent_reported`: 尚未在真实 Tauri 桌面窗口点击项目详情页做视觉验证；本轮重点修复后端读取链路和回归测试。
- `agent_reported`: 尚未新增最近 7 天 / 30 天筛选；当前仍是本地 observed lifetime 口径，顶部主数字排除缓存。
- `agent_reported`: 尚未在真实 Windows/Linux 机器验证默认路径；当前通过候选路径、路径规整和 warnings 降级降低风险。
- `agent_reported`: 尚未接入 Sub to API / New API 的远端用量 API；当前显示是本地 Codex/OpenCode 记录推导出的 observed token 口径。

## Key Decisions Made

- `agent_reported`: 对多个 Codex/OpenCode 候选数据库采用“选择最佳匹配库”而不是“全部合并”，避免把旧库和当前库重复计入。
- `agent_reported`: 最佳库选择规则为：优先匹配记录数更多；记录数相同时优先最近更新时间更晚。
- `agent_reported`: 路径匹配放到 Rust 侧过滤，避免新增 `rusqlite` 的 `functions` feature，也让匹配逻辑更容易测试。
- `agent_reported`: 对 moved project 的历史用量只做唯一 basename fallback，不做自由模糊搜索；这样能恢复 `mind2realistic` 这类搬家项目，同时避免多个同名 repo 时错误合并。
- `inferred`: 用户遇到“任何用量都检测不到”时，最危险的回归点是候选库顺序、路径别名/子路径和精确匹配过窄；因此这些场景必须进入单元测试。
- `user_confirmed`: 主显示不使用 USD/cost；主显示使用总用量 `total_tokens`，也就是非缓存部分加缓存部分。
- `agent_reported`: Codex 主总量直接使用本地 `total_tokens`；OpenCode 主总量为 `tokens_input + tokens_output + tokens_reasoning + tokens_cache_read + tokens_cache_write`。`non_cached_total_tokens` 作为详情拆分项保留；OpenCode 的 `cost` 仍保留在来源诊断卡片，但不再参与主指标。

## Files Changed

- `hard_observed`: `src-tauri/src/local_agent_usage.rs` - 修复候选数据库选择、项目路径匹配、最佳库选择、唯一同名历史路径 fallback，并新增回归测试。
- `hard_observed`: `src-tauri/src/local_agent_usage.rs` - 将主指标从 cost/quota-first 调整为 token-first，并新增/更新主指标回归测试。
- `hard_observed`: `src/components/ProjectDetailBoard.tsx` - 将项目详情 AI 用量小格子改为 `Token 用量` 和 `xx.xM tokens`。
- `hard_observed`: `src/components/VibehubCockpitDialog.tsx` - 将 Agent Usage 详情顶部改为 Token 口径说明，保留 cost 诊断明细。
- `hard_observed`: `src/types/index.ts` - 增加 `tokens` 主指标类型。
- `hard_observed`: `src/locales/zh.json`, `src/locales/zh-TW.json`, `src/locales/en.json` - 更新 Token 口径文案。
- `hard_observed`: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `Cargo.lock` - 版本推进到 `2.0.0-pre.21`。
- `hard_observed`: `.vibehub/tasks/T-20260619044922-98d818ee/runs/R-20260619044922-fef32621/outputs/output.md` - 更新本轮修复、验证和风险记录。
- `hard_observed`: `.vibehub/tasks/T-20260619044922-98d818ee/runs/R-20260619044922-fef32621/outputs/output.md` - 追加 pre22 Homebrew tap/本机安装诊断与升级结果。
- `hard_observed`: VibeHub CLI sync 自动更新 `.vibehub/agent-view/handoff.md`、`.vibehub/agent-view/sync.md`、`.vibehub/index/task-events.idx`、`.vibehub/state.yaml`、当前 run context pack、events 和新增 sync report；未手工编辑 canonical state。

## Files Reportedly Read

- `hard_observed`: `.agents/skills/vibehub-sync/SKILL.md`
- `hard_observed`: `.vibehub/agent-view/current.md`
- `hard_observed`: `.vibehub/agent-view/current-context.md`
- `hard_observed`: `.vibehub/agent-view/handoff.md`
- `hard_observed`: `.vibehub/agent-view/sync.md`
- `hard_observed`: `.vibehub/rules/hard-rules.md`
- `hard_observed`: `.vibehub/adapters/protocol.md`
- `hard_observed`: `.vibehub/tasks/T-20260619044922-98d818ee/runs/R-20260619044922-fef32621/context-packs/implement.md`
- `hard_observed`: `.vibehub/tasks/T-20260619044922-98d818ee/runs/R-20260619044922-fef32621/outputs/output.md`
- `hard_observed`: `src-tauri/src/local_agent_usage.rs`
- `hard_observed`: `src/components/VibehubCockpitDialog.tsx`
- `hard_observed`: `src/components/ProjectDetailBoard.tsx`
- `hard_observed`: `src/services/tauri.ts`
- `hard_observed`: `src/types/index.ts`
- `hard_observed`: `src-tauri/src/commands.rs`
- `hard_observed`: `src-tauri/tauri.conf.json`
- `hard_observed`: `.github/workflows/homebrew.yml`
- `hard_observed`: `/opt/homebrew/Library/Taps/chenm0m/homebrew-vibehub/Casks/vibehub.rb`
- `hard_observed`: local Codex SQLite metadata under `~/.codex`
- `hard_observed`: local OpenCode SQLite metadata under `~/.local/share/opencode/opencode.db`

## Commands Run

- `hard_observed`: `sed -n ...` reads for VibeHub protocol files, skill files, context, output, and relevant source files.
- `hard_observed`: `git status --porcelain`
- `hard_observed`: `git status --short`
- `hard_observed`: `git diff -- src-tauri/src/local_agent_usage.rs`
- `hard_observed`: `vibehub sync /Users/chenm0m/LocalRepo/VibeHub`
- `hard_observed`: `rg -n "local_agent_usage|AgentUsage|vibehub_read_local_agent_usage|agentUsage|AI 用量|tokens_used|opencode" src src-tauri -S`
- `hard_observed`: `sqlite3 /Users/chenm0m/.codex/state_5.sqlite ...`
- `hard_observed`: `sqlite3 /Users/chenm0m/.codex/sqlite/state_5.sqlite ...`
- `hard_observed`: `sqlite3 /Users/chenm0m/.local/share/opencode/opencode.db ...`
- `hard_observed`: `jq '.projects[] | select(...)' "/Users/chenm0m/Library/Application Support/VibeHub/config.json"`
- `hard_observed`: `ls -ld /Users/chenm0m/ArchiveRepo /Users/chenm0m/ArchiveRepo/mind2realistic /Users/chenm0m/LocalRepo /Users/chenm0m/LocalRepo/mind2realistic`
- `hard_observed`: `find /Users/chenm0m -maxdepth 3 -type d -iname 'mind2realistic'`
- `hard_observed`: `find /Users/chenm0m/.codex -name 'state_5.sqlite' -type f -maxdepth 4`
- `hard_observed`: `find /Users/chenm0m/.local/share -path '*opencode*opencode.db' -type f -maxdepth 4`
- `hard_observed`: `cargo test local_agent_usage --manifest-path src-tauri/Cargo.toml`
- `hard_observed`: `cargo fmt --manifest-path src-tauri/Cargo.toml`
- `hard_observed`: `cargo test --manifest-path src-tauri/Cargo.toml`
- `hard_observed`: `npm run build`
- `hard_observed`: `vibehub validate /Users/chenm0m/LocalRepo/VibeHub`
- `hard_observed`: `vibehub output-lint /Users/chenm0m/LocalRepo/VibeHub T-20260619044922-98d818ee`
- `hard_observed`: `rg -n "2\\.0\\.0-pre\\.19|2\\.0\\.0-pre\\.20" package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json Cargo.lock`
- `hard_observed`: `vibehub status /Users/chenm0m/LocalRepo/VibeHub`
- `hard_observed`: `vibehub switch /Users/chenm0m/LocalRepo/VibeHub T-20260619044922-98d818ee`
- `hard_observed`: `rg -n "primary_metric|non_cached|cached|total_tokens|tokens|cost|quota|billing|账单|用量|usage" src-tauri/src/local_agent_usage.rs src/components/ProjectDetailBoard.tsx src/components/VibehubCockpitDialog.tsx src/types/index.ts src/locales/zh.json src/locales/en.json`
- `hard_observed`: `cargo test --manifest-path src-tauri/Cargo.toml local_agent_usage`
- `hard_observed`: `npm run build`
- `hard_observed`: `cargo fmt --manifest-path src-tauri/Cargo.toml --check`
- `hard_observed`: `git diff --check -- src-tauri/src/local_agent_usage.rs src/components/ProjectDetailBoard.tsx src/components/VibehubCockpitDialog.tsx src/types/index.ts src/locales/zh.json src/locales/zh-TW.json src/locales/en.json`
- `hard_observed`: `rg -n "Local recorded cost|billing-style|token_fallback|Billing usage|Billing basis|账单口径|帳單口徑" src-tauri/src/local_agent_usage.rs src/components/ProjectDetailBoard.tsx src/components/VibehubCockpitDialog.tsx src/locales/zh.json src/locales/zh-TW.json src/locales/en.json src/types/index.ts`
- `hard_observed`: `rg -n "\"version\"|2\\.0\\.0-pre\\." package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json Cargo.lock`
- `hard_observed`: `git ls-remote --tags origin v2.0.0-pre.20`
- `hard_observed`: `cargo test --manifest-path src-tauri/Cargo.toml`
- `hard_observed`: `npm run build`
- `hard_observed`: `git diff --check`
- `hard_observed`: `git tag --list 'v2.0.0-pre.*' --sort=-version:refname | head -20`
- `hard_observed`: `git show-ref --tags v2.0.0-pre.22 v2.0.0-pre.21`
- `hard_observed`: `git ls-remote --tags origin 'refs/tags/v2.0.0-pre.22' 'refs/tags/v2.0.0-pre.21'`
- `hard_observed`: `curl -fsSL https://raw.githubusercontent.com/ChenM0M/homebrew-vibehub/main/Casks/vibehub.rb | sed -n '1,120p'`
- `hard_observed`: `curl -fsSL https://api.github.com/repos/ChenM0M/VibeHub/releases/tags/v2.0.0-pre.22 | rg '"tag_name"|"name"|"browser_download_url"|"published_at"'`
- `hard_observed`: `brew config | rg 'HOMEBREW_NO_AUTO_UPDATE|HOMEBREW_AUTO_UPDATE|API|TAP|HOMEBREW_VERSION|Core tap|HOMEBREW_BREW_GIT_REMOTE|HOMEBREW_NO_INSTALL_FROM_API'`
- `hard_observed`: `brew update`
- `hard_observed`: `brew info --cask vibehub`
- `hard_observed`: `brew upgrade --cask vibehub`
- `hard_observed`: `which vibehub && vibehub --version`
- `hard_observed`: `brew list --cask --versions vibehub`
- `hard_observed`: `brew outdated --cask vibehub || true`

## Tests Run

- `hard_observed`: `cargo test local_agent_usage --manifest-path src-tauri/Cargo.toml` passed: 6 passed, 0 failed.
- `hard_observed`: `cargo test --manifest-path src-tauri/Cargo.toml` passed: 14 passed, 0 failed.
- `hard_observed`: `npm run build` passed: `tsc && vite build` completed successfully.
- `hard_observed`: `vibehub validate /Users/chenm0m/LocalRepo/VibeHub` passed: status completed, missing_outputs empty.
- `hard_observed`: `vibehub output-lint /Users/chenm0m/LocalRepo/VibeHub T-20260619044922-98d818ee` passed: issue_count 0.
- `hard_observed`: Release-bump validation passed after `2.0.0-pre.20`: Rust 14 passed, frontend build passed, output-lint passed.
- `hard_observed`: 本轮 `cargo test --manifest-path src-tauri/Cargo.toml local_agent_usage` passed: 9 passed, 0 failed.
- `hard_observed`: 本轮 `npm run build` passed: `tsc && vite build` completed successfully.
- `hard_observed`: 本轮 `cargo fmt --manifest-path src-tauri/Cargo.toml --check` passed.
- `hard_observed`: 本轮 `git diff --check -- src-tauri/src/local_agent_usage.rs src/components/ProjectDetailBoard.tsx src/components/VibehubCockpitDialog.tsx src/types/index.ts src/locales/zh.json src/locales/zh-TW.json src/locales/en.json` passed.
- `hard_observed`: 总量口径调整后再次运行 `cargo test --manifest-path src-tauri/Cargo.toml local_agent_usage` passed: 9 passed, 0 failed.
- `hard_observed`: 总量口径调整后再次运行 `npm run build` passed: `tsc && vite build` completed successfully.
- `hard_observed`: 总量口径调整后再次运行 `git diff --check -- src-tauri/src/local_agent_usage.rs src/components/ProjectDetailBoard.tsx src/components/VibehubCockpitDialog.tsx src/types/index.ts src/locales/zh.json src/locales/zh-TW.json src/locales/en.json` passed.
- `hard_observed`: `2.0.0-pre.21` 发布前 `cargo test --manifest-path src-tauri/Cargo.toml` passed: 17 passed, 0 failed.
- `hard_observed`: `2.0.0-pre.21` 发布前 `npm run build` passed: `tsc && vite build` completed successfully.
- `hard_observed`: `2.0.0-pre.21` 发布前 `git diff --check` passed.
- `agent_reported`: No browser/Tauri visual QA was run in this turn.
- `hard_observed`: Homebrew upgrade verification passed: `vibehub --version` outputs `2.0.0-pre.22`, `brew list --cask --versions vibehub` outputs `vibehub 2.0.0-pre.22`, and `brew outdated --cask vibehub` reports no outdated cask.

## Context Still Needed

- `user_confirmed_needed`: `vibehub sync` 询问当前 Git 变更是否都属于当前任务；本轮基于用户“请继续”和硬证据继续推进，仍建议用户最终确认 VibeHub sync 自动生成文件的归属。
- `user_confirmed_needed`: 是否要继续增加时间窗口筛选，例如 lifetime / 30 天 / 7 天。
- `agent_reported`: 真实 Windows/Linux 默认路径仍需后续机器验证。

## Warnings

- `hard_observed`: `vibehub sync` 报告 `needs_attention`，原因包括 `ownership_unavailable`、Git 工作区存在 VibeHub 状态之外观察到的未提交变更、Git HEAD 与 `last_seen_head` 不一致。
- `hard_observed`: 当前仓库仍有多个 active tasks；本轮只改当前任务相关的 AI 用量读取器文件和当前 run output。
- `hard_observed`: `cargo test` 输出已有 warning：`vibehub-cli/src/main.rs` 被多个 bin target 使用，以及若干 gateway dead_code warning。
- `hard_observed`: `npm run build` 输出已有 warning：Baseline/Browserslist 数据过旧，以及 chunk size 超过 500 kB。
- `inferred`: Codex/OpenCode 本地 schema 不是稳定公共 API，未来版本变化仍可能需要字段探测或兼容。
- `hard_observed`: 本机 Homebrew 环境设置了 `HOMEBREW_NO_AUTO_UPDATE`；未来如果直接运行 `brew upgrade vibehub` 而不先 `brew update`，仍可能再次看到 tap 未刷新导致的“已是最新”假象。

## Next Session Should

- `agent_reported`: 优先在真实 Tauri 桌面窗口打开项目详情页，确认 `AI 用量` 卡片能显示本地记录且详情抽屉能打开。
- `agent_reported`: 如果用户确认实现已可接受，可进入 review/finish 流程；未经用户确认不要运行 `vibehub finish` / `vibehub advance`。
- `agent_reported`: 当前没有执行 `vibehub finish` / `vibehub advance` / `vibehub archive`；后续若进入 review/finish 仍需用户明确确认。

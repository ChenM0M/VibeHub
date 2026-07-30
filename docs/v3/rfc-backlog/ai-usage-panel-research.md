# AI 用量面板调研：成熟产品的呈现共识 + 各 Agent 官方数据目录核对

- 任务：`task.ai-anomaly.db4b93b16a17` / 节点 `node.research`
- 日期：2026-07-29
- 状态：调研完成，**本节点未修改任何实现代码**
- 目标：在动手改 `src-tauri/src/local_agent_usage.rs` / `src/v3/components/task/AIUsagePanel.tsx` 之前，先确认（A）成熟产品怎么呈现用量与额度，（B）我们读取的目录/字段是否真的正确，（C）给出可执行结论。

---

## 0. 问题基线（真实数据，修复前）

在本机对项目根跑 `read_local_agent_usage`（`cargo test -p vibehub --bin vibehub` 临时诊断，已还原）：

```
status cc=stale codex=stale oc=available
completeness=partial freshness=stale refresh_state=partial cache=disabled
total_tokens=2902772081  cost_state=partial
missing=["claude_code.model_or_price_unknown","codex.model_or_price_unknown"]
WARNING_COUNT=3121
```

即：主指标 29 亿 token（恒踩 `>=1e9` 的 anomaly 阈值）、3121 条 warnings、cache 恒 disabled、cost 恒 partial、freshness 恒 stale。面板必然满屏红黄。

---

## 1. 成熟产品怎么做（呈现与交互）

| 产品 | 主指标口径 | 次要指标/展开 | 缺数据·陈旧文案 | 红色/触顶条件 | 诊断细节位置 |
|---|---|---|---|---|---|
| Claude Code `/usage` `/cost`<br>https://code.claude.com/docs/en/costs | 订阅：**plan usage bar（额度 %）**；API：当前会话 $ | Session 区块展开 per-model token 明细（input/output/**cache read**/cache write 并列） | 「computes the dollar figure locally from token counts priced at standard list rates … may differ from your actual bill」；额度接口失败时用 60 分钟内本机快照并标注 “Showing last-known usage” | 无「统计异常」红条概念，只有额度接近/耗尽 | 权威账单外链 Claude Console |
| Codex CLI `/status`<br>`codex-rs/tui/src/status/rate_limits.rs` | **多窗口剩余 % + 20 段进度条 + 重置时间** | `Token usage: 1.9K total (1K input + 900 output)`、`Context window: 100% left` 作副行 | `Warning: limits may be stale - start new turn to refresh.` / `Limits: data not available yet`（三态：available/stale/missing 是一等公民） | 无异常红条 | 置顶一行外链 `chatgpt.com/codex/settings/usage` |
| GitHub Copilot<br>https://docs.github.com/en/copilot/managing-copilot/monitoring-usage-and-entitlements/monitoring-your-copilot-usage-and-entitlements | **premium request 已用/上限 + 进度条 + reset date** | IDE 小面板 → billing 概览 → analytics → 可下载 CSV（三级） | 用 reset date 消歧，不写免责长文 | 仅「额度耗尽」时在各 Copilot 界面通知 | web billing/analytics，不进 IDE |
| Cursor<br>https://cursor.com/docs/account/pricing · https://cursor.com/help/models-and-usage/usage-limits.md | **两个 pool 的 remaining allowance** | 定价表才拆 Input / Cache write / **Cache read** / Output——cache 属定价参考层，不进用量主面 | 按账单周期重置、不结转 | 触顶/spend limit 达成才通知并停用 | web dashboard |
| ccusage<br>https://ccusage.com/guide/daily-reports · /blocks-reports · /cost-modes · /statusline | 报表主指标 **Cost (USD)**；statusline 主指标 = 当前会话成本 + 今日总额 + 当前 5h block | `Cache Create`/`Cache Read` 是**独立列且仅宽终端（≥100 字符）显示**；`--breakdown` 展开模型子行 | 三种成本模式 auto/calculate/display（display 无数据显示 `$0.00`）；默认离线价目 | `--token-limit` 后：绿 <70% / 黄 70–90% / 红 ≥90%；⚠️ 接近、🚨 超限 | **`--debug` / `--debug-samples` 才显示 pricing mismatch、missing cost data** |
| Warp<br>https://docs.warp.dev/support-and-community/plans-and-billing/credits/ | **credits 单一余额** | 回复底部一个 chip，**hover 才展开** credits/tool calls/context window/files changed/diffs | 明说 credit usage 是 non-deterministic，cache hit/miss 是原因之一 | 余额耗尽走 add-on；超月度上限 “it won't process” | 「为什么贵」写在文档里，不塞 UI |
| Zed<br>https://zed.dev/docs/ai/plans-and-usage | **美元 token credit 余额** | 编辑器内控件形态未验证 | — | — | — |

跨领域 UX 依据：NN/g indicator vs validation vs notification 分工（https://www.nngroup.com/articles/indicators-validations-notifications/）、渐进披露（https://www.nngroup.com/articles/progressive-disclosure/）、错误信息准则（https://www.nngroup.com/articles/error-message-guidelines/）、IBM Carbon notification 变体分工（https://carbondesignsystem.com/components/notification/usage/）。

### 共识（7 条）

1. **主指标几乎从不是 token 总量**；token 是解释项。
2. **「剩余 + 何时重置」永远同框**。
3. **成本口径必须自我声明**（list rates / 估算 / non-deterministic）。
4. **权威真值一律外链**，位置固定。
5. **available / stale / missing 是三态一等公民**，各有专属短句，不是错误。
6. **紧凑面只放 1 行**，明细靠 hover / 宽度 / 子命令 / debug 展开。
7. **红色只留给「已被阻断或将被阻断」**；统计波动最高到 ⚠️。ccusage 阈值 70/90 是可抄的事实标准。

### 未验证（不据此决策）

Claude `/usage` 的 24h↔7d 键位（d/w/r）、Zed 编辑器内控件、JetBrains AI 配额 UI、OpenAI Platform usage dashboard 页面结构。

---

## 2. 各 Agent 的官方数据目录/格式核对

### 2.1 Claude Code

| 事实 | 来源 |
|---|---|
| 转录明文在 `~/.claude/projects/`，默认留 30 天（`cleanupPeriodDays`） | https://code.claude.com/docs/en/data-usage |
| 层级 `projects/<project>/<session>.jsonl`，**另有 `projects/<project>/<session>/subagents/*.jsonl`** 与 `tool-results/` | https://code.claude.com/docs/en/claude-directory#application-data |
| `~/.claude/stats-cache.json` = `/usage` 的聚合缓存，不受清理 | 同上（本机不存在，schema 未验证） |
| `CLAUDE_CONFIG_DIR` 可整体重定位；ccusage 还探测 `$XDG_CONFIG_HOME/claude` | https://code.claude.com/docs/en/env-vars ；ccusage `rust/adapters/claude/src/paths.rs:12-51` |
| `message.usage` 字段：`input_tokens`、`output_tokens`、`cache_read_input_tokens`、`cache_creation_input_tokens`、`cache_creation.{ephemeral_5m_input_tokens,ephemeral_1h_input_tokens}`、`service_tier`、`server_tool_use`、`inference_geo` | 本机 13,107 条记录全字段普查 |
| **计价**：cache read = 0.1x input；5m write = 1.25x；1h write = 2x；`inference_geo=US` 全类别 ×1.1 | https://docs.anthropic.com/en/docs/about-claude/pricing |
| OTel `claude_code.token.usage` 用 `type=input\|output\|cacheRead\|cacheCreation` **四类并列**，官方从不给「total tokens」合计 | https://code.claude.com/docs/en/monitoring-usage |
| API 限额只在响应头 `anthropic-ratelimit-*`（不是 `x-ratelimit-*`），本地 JSONL 无任何额度字段 → **本地 reader 无法观测周/月配额** | https://docs.anthropic.com/en/api/rate-limits |
| ccusage 的 `Total Tokens` 列 = input+output+cacheCreate+cacheRead（**含 cache read**），但同屏并列 4 列，决策数字是 Cost | `crates/ccusage-core/src/types.rs:86-90`、`output.rs:173-177` |
| ccusage 去重键 `(message.id, requestId)`，`requestId` 缺失退化为纯 `message.id`；冲突时**非 sidechain 优先 → token 总和更大者优先** | `adapters/claude/src/lib.rs:126-206` |

**本机核对**：`~/.claude/projects` 存在，7 个项目目录，325 个 jsonl（212 主 + **113 subagents**），13,107 条记录，覆盖 CC 版本 2.0.76 → 2.1.217。`requestId` 本机 0 次出现（本机走第三方代理，无法判定官方直连是否也无）。

**关键数量级事实（本机主转录）**：

```
                   input        output   cacheCreate      cacheRead        TOTAL
不去重          121,336,390   3,836,793    15,958,294    533,151,395   674,282,872
按 message.id   26,801,264     855,166     8,765,657    221,269,532   257,691,619   ← 2.62x 差
主+subagents    46,183,415   1,227,425     9,737,909    252,953,891   310,102,640
```

cache_read 占 TOTAL 的 **85.9%** —— 「total 是否含 cache_read」直接决定数量级；subagents 与主转录 message.id 重叠 0，漏读即少算。

### 2.2 Codex CLI（以上游 `openai/codex` main 源码为准）

| 事实 | 来源 |
|---|---|
| `CODEX_HOME` 覆盖，默认 `~/.codex` | `codex-rs/core/src/config/mod.rs:4573-4583` |
| rollout = `$CODEX_HOME/sessions/YYYY/MM/DD/rollout-<ts>-<thread_id>.jsonl` | `codex-rs/rollout/src/recorder.rs:1550-1575` |
| `archived_sessions/` 仍存在，**扁平**（rename 过去，无 YYYY/MM/DD） | `codex-rs/rollout/src/lib.rs:25-26`、`thread-store/src/local/archive_thread.rs:95-108` |
| ⚠️ **冷 rollout（>7 天）会被后台压成 `.jsonl.zst`** | `codex-rs/rollout/src/compression.rs:18,258,378-390` |
| 状态库 `state_5.sqlite`，目录优先 config `sqlite_home` → env `CODEX_SQLITE_HOME` → `CODEX_HOME` | `codex-rs/state/src/sqlite.rs:29-33`、`config/mod.rs:276-286` |
| token 用量 = `event_msg` 且 `payload.type=="token_count"`：`TokenCountEvent{info,rate_limits}`；`TokenUsageInfo{total_token_usage,last_token_usage,model_context_window}`；`TokenUsage{input_tokens,cached_input_tokens,cache_write_input_tokens,output_tokens,reasoning_output_tokens,total_tokens}` | `codex-rs/protocol/src/protocol.rs:2057-2081,2142-2146` |
| `token_count` 一定会被写入 rollout（policy 显式 true） | `codex-rs/rollout/src/policy.rs:98-104` |
| 旧/被中断 rollout 合法地没有 `token_count` ⇒ 「No total_token_usage found」是**预期噪声** | `info: Option<…>` + 本机 515 个 rollout 中 15 个无该事件 |
| `session_meta` 无 `model`，模型在 `turn_context.payload.model` | `protocol.rs:3059-3155,3265-3290` |
| `/status` 的 headline 用 `percent_of_context_window_remaining()`（BASELINE_TOKENS=12000），单值展示用 `blended_total() = 非缓存 input + output` | `protocol.rs:2215-2260` |
| **ChatGPT 套餐额度的唯一本地面 = `payload.rate_limits`**：`primary`/`secondary` 各含 `used_percent`、`window_minutes`、`resets_at`(unix 秒)，另有 `credits{has_credits,unlimited,balance}`、`plan_type`、`spend_control_reached` | `protocol.rs:2148-2205`（注意不是旧命名 `resets_in_seconds`） |
| `threads.tokens_used` = 最后一次 `token_count` 的 `total_token_usage.total_tokens`，与 rollout 同口径，可作 fallback | `codex-rs/state/src/extract.rs:101-103` |

**本机核对**：`sessions/2026/{05,06,07}/DD/rollout-*.jsonl` 515 个；`archived_sessions/` 扁平 8 个；`.jsonl.zst` 0 个（本机版本尚未压缩过）；`state_5.sqlite` 在 `~/.codex/` 根（另有历史 `~/.codex/sqlite/state_5.sqlite`）；`threads` 523 行、`tokens_used>0` 500 行；抽查 3 个 rollout 的 `token_count` 行数 89/36/17，`rate_limits` 一一对应，字段名与上游一致。

### 2.3 OpenCode

| 事实 | 来源 |
|---|---|
| 数据目录 `$XDG_DATA_HOME/opencode`，默认 `~/.local/share/opencode`（**macOS/Windows 同样走该路径**，不用 Application Support/AppData） | `packages/core/src/global.ts:10-30` |
| 现版本用 SQLite：`$OPENCODE_DB` → channel ∈ {latest,beta,prod} 或 `OPENCODE_DISABLE_CHANNEL_DB=1` 时 `<data>/opencode.db` → **否则 `<data>/opencode-<channel>.db`（源码/自建版默认 channel=`local` ⇒ `opencode-local.db`）** | `packages/core/src/database/database.ts:44-56`、`installation/version.ts:7` |
| 旧 JSON 树 `storage/session/info|message|part/...` 只作一次性迁移源 | `packages/opencode/src/storage/storage.ts:97-183` |
| 用量：message `data` JSON 的 `$.tokens.{input,output,reasoning,cache.read,cache.write}`、`$.cost`；session 行有去规范化列 `cost, tokens_input/output/reasoning/cache_read/cache_write` | `database/migration/20260510033149_session_usage.ts`、`core/src/session/sql.ts:23-64` |
| 项目关联 `session.project_id → project.id`，`project.worktree` 是绝对路径（非哈希） | `packages/core/src/project/sql.ts` |
| `opencode stats` 的总数就用 session 行的 `cost`/`tokens.*` 之和 | https://opencode.ai/docs/cli/#stats ；`cli/cmd/stats.ts:171-224` |

**本机核对**：`~/.local/share/opencode/opencode.db`（1.6GB）+ wal/shm，无 `opencode-*.db` 变体；`storage/` 仅剩 `migration/`、`session_diff/`；session 270 行（全部 `ses_` 前缀），project 10 行，`worktree LIKE %VibeHub%` 命中 1 个项目、61 个 session。

### 2.4 Claude App / Cursor

保持现状「unsupported」判断是正确的：两者都没有稳定、公开、可按项目归因的本地用量契约（Cursor 的用量真值在 web dashboard，Claude App 只有内部 Electron/SQLite/IndexedDB）。**但 unsupported 不应当作 warning**（见结论 C3）。

---

## 3. 我们的实现对不对（逐条）

### 3.1 正确的部分

- `codex_state_db_candidates()`：`CODEX_SQLITE_HOME`/`CODEX_HOME`/`~/.codex` + `state_5.sqlite` ✅
- `codex_rollout_root_candidates()`：`sessions` + `archived_sessions`，`WalkDir::max_depth(4)` 刚好覆盖 `YYYY/MM/DD/file` 与扁平 archived ✅
- rollout 解析字段名（`parse_codex_usage_value`）与上游 `TokenUsage` 完全一致 ✅
- DB fallback 用 `threads.tokens_used` 与 `total_token_usage.total_tokens` 同口径 ✅
- opencode SQL（`session LEFT JOIN project ON project.id=session.project_id`，取 `worktree/directory/path` + 6 个 tokens 列 + cost）与上游 schema 一致，且与 `opencode stats` 同口径 ✅
- 「No total_token_usage found in Codex rollout X」在事实层面正确（本机 15/515），只是**不该以 warning 形式逐条进 UI** ✅
- claude_app / cursor 判为 unsupported ✅

### 3.2 已确认的真实缺陷（本次调研新增，超出任务原 5 项）

| # | 缺陷 | 证据 | 后果 |
|---|---|---|---|
| R1 | `encode_claude_project_path`（`local_agent_usage.rs:1495-1507`）只把 `/ \ :` 换成 `-`，而 Claude Code 实际是**所有非字母数字字符 → `-`** | 本机 `cwd=/Users/chenm0m/codepilot_assistant` 的真实目录是 `-Users-chenm0m-codepilot-assistant` | 路径含 `_`/`.`/空格的项目**完全读不到** Claude Code 用量，只报「session directory was not found」 |
| R2 | 只 `read_dir(sessions_dir)` 顶层（`:1208`），不递归 `\<session\>/subagents/*.jsonl` | 本机 113 个 subagent jsonl，message.id 与主转录重叠 0 | subagent（Task 工具）用量全部漏算 |
| R3 | 去重是**先到先得**（`:1436` `seen_messages.insert` 命中即 return） | 旧版 CC（2.0.76）会写多条同 `message.id` 的流式中间行（0/0 token）+ 一条终值行；ccusage 的策略是「token 总和更大者胜」 | 在含该模式的转录上**严重低估** |
| R4 | 去重域仅限单个 session accumulator，跨文件/跨 session 不去重 | 本机 150 例跨文件重复 message.id | 少量高估 |
| R5 | cache write 不区分 5m/1h（1h 是 **2x** 而非 1.25x）；`cache_read_per_1m: None` 的模型静默丢失该部分成本（`:201/208/215`），未按官方 0.1x 兜底 | 官方 pricing 页 + ccusage `pricing.rs:321` | 成本低估且 `cost_state` 恒 partial |
| R6 | 未处理 codex `.jsonl.zst`；`File::open` 失败时静默 `None` 且不告警（`:2367-2370`） | `compression.rs`（>7 天压缩） | 随 codex 升级必然触发：细分 token 静默变 0，只剩 DB total |
| R7 | opencode 只找 `opencode.db`，缺 `opencode-<channel>.db` 与 `$OPENCODE_DB` | `database.ts:44-56` | 源码/自建版（channel=local）读不到 |
| R8 | 候选路径里的 `OPENCODE_DATA_HOME`（`:2443`）不是 opencode 真实环境变量；`Library/Application Support/ai.opencode.desktop`、`AppData/*` 对 CLI 版不成立 | 上游只认 XDG | 无害但给出错误预期 |
| R9 | codex `rate_limits` 完全未读取 | `protocol.rs:2148-2205` | 我们本可以拿到**真额度百分比 + 重置时间**（正是共识里的主指标），却在展示自造的 token 合计 |

---

## 4. 可执行结论（供后续节点实现）

### C1 主指标（对应 criterion c01）

- **面板 headline 不用 `total_tokens`**（含 cache_read，本机 85.9% 是 cache read，数量级失真）。
- 有 codex `rate_limits` 时，headline 用**额度剩余 % + 重置时间**（Codex/Copilot/Cursor 共识）；没有额度数据时退化为「本周期 non-cached tokens + 估算成本」。
- `total_tokens` 保留在数据层（与 ccusage 可交叉验证），展开层按 `input / output / cache 写 / cache 读` 四列并列，并标注 cache read 按 0.1x 计费。
- **anomaly 不再用「总量 >= 1e9」**。合理触发只剩三类：① 额度耗尽/将被拒绝；② 支出上限达成；③ 数据源写入/解析失败导致计量不可信。趋势类（突增、burn rate）最高 ⚠️，用中性指示器。可选：采用 ccusage 的 70/90 阈值分级。

### C2 warnings 分级（对应 c02）

- 三层：`ui_status`（≤1 条最高优先级，卡片内可见）→ `notices`（折叠「还有 N 项提示 ›」）→ `diagnostics`（默认不进 UI，仅审计/日志，对齐 ccusage `--debug`）。
- 归入 diagnostics：逐行 lineage/compaction 提示、缺 `total_token_usage` 的 rollout、逐文件解析细节。UI 只显示**按类聚合的计数**（如「12 个 rollout 缺 token 记录」）。
- `refresh.state` 与 `completeness` 不再因 diagnostics 非空而降级；UI 也不再「warnings 非空即 partial」（`AIUsagePanel.tsx:89`）。

### C3 unsupported 源（对应 c03）

claude_app / cursor 的 unsupported 是设计事实：不产生 warning、不降级 completeness/refresh.state；在展开层用中性一行说明「不支持本地读取，见官方 dashboard」，并给外链（共识 4）。

### C4 缓存（对应 c04）

commands 改走 `*_with_cache`，`cache_state` 反映真实命中；消除 `shared_usage_cache` / `*_with_cache` 的 dead_code 警告；留连续两次刷新 `duration_ms` 证据。

### C5 stale（对应 c05）

抄 Codex 的三态与文案：
- available；
- stale → `额度/用量数据可能已过期 · 开始新一轮对话后自动刷新`（原文 `limits may be stale - start new turn to refresh.`）；
- missing → `用量数据暂不可用`（原文 `data not available yet`）。

阈值不用固定 900s：短窗口（5h 额度）>5 分钟未更新算 stale；日/周期聚合 >1 小时才算 stale；「最近没有 Agent 活动」≠ 数据陈旧，前者不显示告警。

### C6 UI 精简

移出卡片：cost-missing-reasons 列表、audit breakdown、provider 饼图（1 列宽不可读）、完整 session 列表。改为一句成本口径声明 + 展开层的排序条形列表 + 「当前会话」一行 + 固定底部权威外链。

### C7 建议纳入本任务或后续任务的路径修复

R1、R2、R3 直接决定「数字对不对」，建议在本任务内随 c01 一并修（会显著改变 total）；R5–R9 建议排后续任务，其中 R6（`.jsonl.zst`）与 R9（`rate_limits`）优先级最高——R9 是把 headline 换成真额度的前置条件。

---

## 5. 明确未验证项

- Claude Code project 目录 slug 的官方算法（文档只写 `<project>` 占位符；本结论由本机 9 条 cwd 反推，未覆盖含 `.`、空格、中文、Windows `C:\`）。
- `requestId` 是否为官方直连必有字段（本机全经第三方代理，0 命中）。
- `~/.claude/stats-cache.json` 的 schema（本机不存在）。
- Codex 旧版本是否曾用 `rate_limits.*.resets_in_seconds`。
- OpenCode「JSON 树 → SQLite」的确切 npm 版本边界（只定位到 drizzle 迁移 `20260127222353`）。
- 本机 codex 为改装 fork（含 `cockpit-*`、`world_state`），Part 2.2 结论以上游源码为准。
- Claude `/usage` 的 d/w/r 键位、Zed 编辑器内控件、JetBrains AI 配额 UI、OpenAI Platform usage dashboard 页面结构。

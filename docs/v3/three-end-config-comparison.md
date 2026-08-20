# 三端配置能力与回落行为对照

本文档对照 VibeHub 管理的三个 Agent 端的配置能力、受管字段与第三方端点回落行为。
代码侧的契约说明见 `crates/vibehub-core/src/v3/agent_specs.rs` 模块文档。

## 1. 三端配置面

| 维度 | Claude Code | OpenCode | Codex |
| --- | --- | --- | --- |
| 主配置文件 | `~/.claude/settings.json`（Profile 位于 `~/.claude/vibehub-profiles/<name>.settings.json`） | `~/.config/opencode/opencode.jsonc` 或 `opencode.json` | `~/.codex/config.toml`（Profile 为 `~/.codex/<name>.config.toml`） |
| 模型选择字段 | `env` 变量：`ANTHROPIC_BASE_URL`、模型别名等 | Provider `models[]` + 默认模型 | `model_provider` + `model` |
| VibeHub 受管 env | `CLAUDE_CODE_SUBAGENT_MODEL`、`ANTHROPIC_DEFAULT_HAIKU_MODEL`、`ANTHROPIC_DEFAULT_SONNET_MODEL`、`ANTHROPIC_DEFAULT_OPUS_MODEL`、`ANTHROPIC_DEFAULT_FABLE_MODEL`、`DISABLE_PROMPT_CACHING` | Provider / Base URL / credential reference / 模型 / 默认模型 / 小模型 / variants | `model_provider` / `model` / reasoning effort / Provider Base URL / wire API / env key |
| 保留字段 | `permissions`、`hooks`、`mcpServers`、`sandbox`、插件、未知字段原样保留 | 未知字段由 Adapter 尽力保留 | 项目级配置可覆盖用户级默认；未知字段保留 |
| 第三方端点风险 | **高** — 原生 `claude-*` ID 可能无法解析 | 低 — 模型 ID 由用户按 Provider 自填 | 低 — model 是用户自填字符串 |

只有 Claude Code 通过环境变量解析模型，且这些变量可能被第三方端点拒绝，因此子 Agent / 小模型 / 别名的回落逻辑只发生在 Claude Code。

## 2. Claude Code 回落规则

`patch_for_claude`（`src-tauri/src/agent_profiles.rs`）在投影前解析高级配置；`apply_settings_patch`
（`crates/vibehub-core/src/v3/claude_code_adapter.rs`）写入 env。回落规则：

- 任一高级项（子 Agent 模型、小/快模型、`sonnet`/`opus`/`haiku`/`fable` 别名）未配置时，
  回落为受管默认模型（主模型）。
- 目的：子 Agent（含内建子 Agent 如 `statusline-setup`）与后台任务不会漂移到第三方端点
  无法提供服务的原生 `claude-*` ID。
- `haiku_model` 与 `small_fast_model` 同属 `ANTHROPIC_DEFAULT_HAIKU_MODEL` 层；解析时 `haiku_model`
  优先，写回时 `small_fast_model` 镜像 `haiku_model` 的规范值。
- 官方 `api.anthropic.com` 端点提供全部原生 ID，回落对其无副作用。

未在“高级配置”区填写任何项 = 采用回落；填写了某项 = 使用该项（空字符串视为未配置，同样回落）。

## 3. 第三方端点诊断

当 Claude Code Profile 指向非 `anthropic.com` 端点，`read_claude_profile_with_index` 会在
`ClaudeCodeProfileView.warnings` 中产出诊断（经 `agent_profiles::profile_warnings` 映射为
`result.warnings`）：

| code | 含义 |
| --- | --- |
| `CLAUDE_NATIVE_MODEL_ON_THIRD_PARTY_ENDPOINT` | 主 `model` 仍是原生 `claude-*` ID |
| `CLAUDE_THIRD_PARTY_ENDPOINT_NATIVE_TIER` | 某个高级别名仍是原生 `claude-*` ID |
| `CLAUDE_THIRD_PARTY_ENDPOINT_ADVANCED_UNSET` | 所有高级项均未配置（回落生效，属预期但予以提示） |

官方 `api.anthropic.com` 端点不产生上述任何诊断。

## 4. 故障场景（已修复方向）

- 子 Agent 默认漂移到 `claude-sonnet-5` 等原生 ID → 第三方端点不提供 → 报错。
  修复：回落主模型；诊断提示未配置的高级项。
- 小/快模型层未配置 → 历史 `ANTHROPIC_SMALL_FAST_MODEL`（已弃用）漂移。
  修复：统一走 `ANTHROPIC_DEFAULT_HAIKU_MODEL` 并回落。

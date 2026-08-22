# Agent Profiles 配置中心交付说明

VibeHub 的 Agent Profiles 配置中心只管理 OpenCode、Claude Code 和 Codex。
Cursor、账号/OAuth 登录、配额、账号池和重型网关控制台不在范围内。

## 支持矩阵

| Agent | macOS / Linux host | Windows host | WSL runtime | Profile 启动语义 |
| --- | --- | --- | --- | --- |
| OpenCode | `~/.config/opencode/opencode.jsonc` 或 `opencode.json` | `%APPDATA%\\opencode\\opencode.jsonc` 或 `opencode.json` | 目标发行版 home 下的 `~/.config/opencode/opencode.jsonc/json` | 原生单配置；“仅此次启动”使用当前配置 |
| Claude Code | `~/.claude/settings.json`；受管 Profile 位于 `~/.claude/vibehub-profiles/<name>.settings.json` | `%USERPROFILE%\\.claude\\settings.json`；Profile 位于同目录 `vibehub-profiles` | 发行版 home 下的同名路径 | 临时启动使用 `claude --settings <Profile 路径>`；默认启动使用用户 settings 投影 |
| Codex | `~/.codex/config.toml`；原生 Profile 为 `~/.codex/<name>.config.toml` | `%USERPROFILE%\\.codex\\config.toml`；Profile 为同目录 `<name>.config.toml` | 发行版 home 下的同名路径 | 临时启动使用当前 CLI 支持的 `codex --profile-v2 <name>`；默认通过受管字段投影到 `config.toml` |

Windows host、WSL 每个发行版和 macOS/Linux host 都是独立 runtime target。VibeHub
不会把 host 名称当作能力证明，也不会把 Windows 路径直接当成 WSL Linux 路径。
WSL 的发现、保存和启动必须使用该发行版观察到的 home；Windows 上启动 WSL
Agent 时由 Rust 侧转换成 `wsl.exe -d <distribution> --cd <linux-home> -- ...`。

## 用户操作

1. 从左侧导航打开独立的“Agent Profiles”单页面（也支持 `#agent-profiles` 深链接）。
2. 选择 Agent 和 runtime target；列表会展示来源路径、revision、默认标记和兼容性。
3. 选择 Profile 后，通过图形化表单新增/编辑/删除 Provider、模型、Base URL、credential reference、思考档位和 variants。
4. 点击“保存”写回当前文件；“设为默认”只更新该 Agent 允许投影的字段；“仅此次启动”不修改默认配置。
5. Claude Code 和 Codex 可以通过图形化 Dialog 新建、复制、重命名和删除 Profile。删除默认 Profile 必须先选择替代 Profile；Claude Code 的原生 settings 仅支持单一 Anthropic Provider，因此只允许编辑而不伪造多个 Provider。
6. “高级配置”降为兜底入口，显示脱敏的受管结构化投影、原生来源、协议和保留范围；应用草稿后仍走相同的 Schema、revision、备份和原子写回门禁。

Claude Code 的兼容区还会显示 `schema_capability.capability_declaration` 和
`custom_model_options`：前者说明声明来源、支持字段和回落顺序，后者说明模型候选
来自哪一层、当前可选值以及是否允许自定义。`__auto__` 只是受控 select 的界面
占位值，保存 payload 使用 `null`；因此 null、空白、缺失和旧的 `__auto__` 值都会
回到“自动沿用主模型”，不会在控件中留下空白值。已有但不在候选列表中的模型 ID
会作为未知值保留并可见，避免读取旧配置时静默丢值。适配器未声明能力时，界面明确
显示 unavailable，并保留自动回落，而不是渲染一个没有选项的空控件。

空状态表示目标可观察但尚无配置，不代表可以猜测路径。权限错误、语法错误、未知 Schema、协议能力未知和 revision 冲突都显示为可恢复或明确不可用状态，不能静默覆盖文件。

## 受管字段与保留边界

- OpenCode：Provider、Base URL、环境变量形式的 credential reference、模型、默认模型、小模型、variants/思考档位。
- Claude Code：model、受管 Base URL 和思考开关/档位；`permissions`、`hooks`、`mcpServers`、`sandbox`、插件和未知字段保留在原文件中。
- Codex：`model_provider`、`model`、reasoning effort、Provider Base URL、wire API 和环境变量 key；项目级配置可能覆盖用户级默认。
- 未知字段、注释、排序/格式语义和未受管字段由 Adapter 尽可能保留；无法可靠迁移的字段只读展示并说明原因。

## 协议与静默适配

协议路线由 Agent 原生协议、Provider upstream 协议和模型能力共同决定：

| 条件 | 路线 |
| --- | --- |
| native 与 upstream 相同且能力完整 | `direct` |
| native/upstream 不同但存在已声明转换器 | `adapter`，后台准备受管运行时 |
| 协议或能力未知、不匹配或转换器缺失 | `unavailable`，显示诊断并停止启动 |

用户不需要在默认视图里选择“代理/网关”。协议诊断默认折叠，只用于解释为什么
某个模型是直连、适配、部分支持或不可用。转换器必须保留文本、多轮消息、system/
developer、tool、图片、流式增量、reasoning、usage 和结构化错误语义；不支持的
能力返回明确错误，不伪装成成功。

## Secret 模型

VibeHub 从不读取或修改 OpenCode/Codex auth 文件、Claude OAuth/credentials 文件，
也不会把 API Key 写入普通 Agent 配置、VibeHub 项目文件、日志、诊断或导出。
配置中心只接受以下 reference：

- 环境变量名，例如 `DEEPSEEK_API_KEY`；
- macOS Keychain、Windows Credential Manager 或等价 Secret Store 的 reference；
- `none`/`unknown` 状态。

“已配置”只表示 reference 存在，不表示 VibeHub 读取了 secret 值。literal key 会被
标记为隐藏/不受支持，错误文本也会做防御性脱敏。

## 保存、备份与恢复

保存顺序是：读取 revision → 校验 runtime target、来源路径、Schema、协议和 Secret
边界 → 同目录备份 → 临时文件写入并同步 → 原子替换 → 重新读取确认。写入失败会
清理临时文件，不能留下截断配置。

“恢复本次备份”只接受所选 Profile 同目录、由 VibeHub 生成且与当前文件匹配的备份。
如果外部修改使 revision 过期，恢复和保存都会拒绝覆盖，并要求重新读取；不会删除
外部新内容。

## 常见故障

- **没有 runtime target**：确认目标 home 可观察；Windows WSL 发行版需要可用的 `wsl.exe`。
- **Profile 只读/未知**：先检查 Schema/协议版本；未知能力不会通过手动输入被强行升级。
- **revision 冲突**：点击刷新，重新读取后再编辑；不要复制旧草稿覆盖新文件。
- **默认切换失败**：检查 Agent 的默认投影路径和权限；Codex 项目级配置可能覆盖用户级默认。
- **启动失败**：确认对应 `claude`、`opencode` 或 `codex` 已安装并在目标 runtime 的 PATH 中；错误不会回显 Secret。
- **协议不可用**：展开协议诊断检查 native/upstream、adapter 版本和 limitation；未知协议会 fail closed。

## 非范围与验收状态

本功能不负责账号登录、配额计费、多实例负载均衡、自动唤醒、Cursor、各 Agent
凭据迁移或完整网关控制台。配置 Adapter、协议 IR、Sidecar 生命周期和跨平台路径
规则已有离线 fixture 与 Rust 测试；macOS/本地 UI/核心测试可在仓库内复现。

Windows 原生和至少一个 WSL 发行版的最终验收必须使用发布后的同一 artifact/hash，
并记录安装包、版本、IDE/CLI 版本、路径、无损保存、默认、临时启动、转换和恢复
结果。CI 或 macOS 证据不能替代该手测；在完成前，相关 Full criterion 保持
`blocked`。

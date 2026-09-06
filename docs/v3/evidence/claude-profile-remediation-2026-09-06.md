# Claude Profile 修复与验收记录

- Task：`task.claude-agent-profile-auto.fdad6ac0200e`
- 验证基线 HEAD：`449e51a83588fbc35c1509dcf888ff0ef3ffb7d9`；以下验证在提交前的修复工作树上执行，随后与代码一并提交。实际提交标识以 Git 和 V3 Session 记录为准。
- 控制面真值在 V3 events/criterion reviews；本文是可复核的代码、命令和行为证据摘要。

## 修复

1. C02：Haiku/background 共用单个 UI 入口和 `haiku_model` 读回字段，不再镜像到旧 `small_fast_model`。UI 选 auto 时清空两个字段；显式覆盖之外不复制主模型。旧别名仅作为兼容输入。
2. C03：原生用户 settings 的启动参数为空；独立 Profile 默认和临时启动均显式使用 `--setting-sources "" --settings <path>`。激活只写 VibeHub 索引，不再改写用户 settings。
3. C04：三语 auto 标签、说明、能力 fallback_priority 和默认投影说明与实际行为一致。auto 为 Claude 原生选择，不承诺跟随主模型；子 Agent 环境变量为显式强制覆盖。
4. 同时修复关闭禁用缓存时旧 `DISABLE_PROMPT_CACHING=1` 不被清除的问题。

## 本地验证

| 验证 | 实际结果 |
| --- | --- |
| `cargo test --locked -p vibehub-core v3::claude_code_adapter --lib` | 16 passed，包括 JSONC 只读拒写且注释字节不变 |
| `cargo test --locked --manifest-path src-tauri/Cargo.toml agent_profiles::tests` | 20 passed（可监听回环端口的执行环境） |
| `npm run v3:contracts:check` | 819 assertions passed |
| `npm run v3:i18n:check` | 三语 parity / TSX 扫描通过 |
| `npm run v3:agent-profiles:check` | auto、未知模型、单一 Haiku 编辑器、实际三语文案与后端能力声明通过 |
| `npm run build` | 通过；仅现有 bundle 大小/浏览器数据警告 |
| `git diff --check` | 通过 |
| `cargo test --locked --workspace` | 通过（2026-09-06，可监听回环端口的执行环境；core 466 passed） |

完整读写回归 `claude_read_auto_save_read_removes_all_overrides` 修复前实际失败：
`override survived auto: ANTHROPIC_DEFAULT_HAIKU_MODEL`；修复后通过。测试从真实文件读取、
清空高级字段、更换主模型、经 Tauri save 保存、重新读取，检查模型覆盖不存在、未知字段和权限保留。
其他新增测试验证 A/B/A 激活后用户/Profile 字节不变、默认/临时启动参数一致、旧 revision 和
外部损坏 JSON 拒绝覆盖、凭据引用不被更换。

## 页面证据（2026-09-05）

通过 ego-browser 打开实际 React 页面：
`http://127.0.0.1:5193/?fixture=release&fixtureAgent=claude` → Agent 配置 → 兼容区。
这是 DEV fixture，无真实用户配置或桌面文件写回。

- 未知 `legacy-model` 可见，未渲染成空选项。
- Haiku 改 auto 后可见标签为“自动（Claude 原生选择）”。
- 通过真实 change 事件触发 React，再从高级草稿 textarea 读回：
  `{"haiku_model":null,"small_fast_model":null}`。
- `#claude-small-model` 不存在；页面无“沿用主模型”旧文案。
- 能力声明显示 `explicit_override → claude_native_resolution`。

## Claude CLI 证据与边界

`node scripts/v3-agent-profiles/claude-runtime-smoke.mjs` 使用真实本机 Claude Code 2.1.236、
独立 `CLAUDE_CONFIG_DIR`、临时 cwd、假 API Key 和回环模拟 API。
不改写 HOME，不读取用户凭据，不调用真实第三方模型服务。

2026-09-05 的执行实际观察到：A 请求 `claude-sonnet-4-6`，B 请求 `claude-haiku-4-5`，
重启 A 又请求 `claude-sonnet-4-6`；未使用用户/项目 fixture 的 `claude-opus-4-6`。
带空格的 Profile 路径可用；每次运行后四份配置文件字节不变。

该测试附加 `--bare`、禁用工具与 MCP，验证的是明确 settings 和请求模型解析路径，
不是交互式终端完整验收、账号隔离或 Windows GUI 验收。
Windows/WSL 同一 Release artifact/hash 的原生验收未执行，C06 必须保留 blocked。
本次没有发布新 artifact，不能用本地测试替代发布后跨平台验收。

2026-09-06 再次执行同一脚本通过，A/B/A 模型和四份文件不变结果一致。
测试夹具位于本机临时目录 `vibehub-claude-runtime-FkHQSH`，无真实凭据。

## 使用与升级注意

- 外部终端直接执行 `claude` 仍使用原生配置，不跟随 VibeHub 独立默认索引。
- 旧自动投影产生的值无法可靠地区别于用户显式指定的值，因此不自动迁移或删除；需明确切回 auto 后保存。
- settings 文件隔离并不隔离进程环境变量、管理员策略或账号。第三方模型须明确填写端点支持的 ID。
- [官方模型配置](https://code.claude.com/docs/en/model-config)：Haiku/background 共用键；子 Agent 变量覆盖所有子 Agent 的选择。
- [官方 CLI](https://code.claude.com/docs/en/cli-usage)：settings 与 setting-sources 参数。

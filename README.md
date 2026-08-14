# VibeHub

[English](README_EN.md) | [简体中文](README.md) | [繁體中文](README_TC.md)

![alt text](image.png)

> VibeHub 是一个围绕 Agent 工作流体验打造的 All-in-One 工具平台。它源于个人 Vibe
> 开发过程中的痛点：Agent 干活的计划、进度、验收与阻塞缺少可见、可追溯的载体。
> VibeHub 用任务 / 计划 / 会话 / 事件驱动的工作流，把这些"看不见的过程"变成看板与
> 计划图。它不绑定任何特定的 Agent Harness —— 只要能读取仓库里的工作流编排说明、
> 并能调用标准 MCP 工具，你的 Agent 就能接入。桌面端用可视化 Cockpit 降低使用门槛，
> 而内核本质是一个独立的 CLI，可以拆出去单独使用或 Fork 改造。

![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)
![GitHub Release](https://img.shields.io/github/v/release/ChenM0M/VibeHub)

[更新日志](CHANGELOG.md) · [Releases](https://github.com/ChenM0M/VibeHub/releases)

## 主要特色：V3 工作流

- **Harness 无关** — 不绑定 OpenCode / Codex / Claude Code 等特定 Harness；Agent 只要能读取仓库里的工作流编排说明、并能调用 V3 MCP 工具，就能接入同一套工作流
- **CLI 内核 + 可视化前端** — 本质是一个独立 CLI（`vibehub v3 ...`），可单独拆出使用或 Fork 改造；桌面端的 V3 Cockpit 只是它的可视化入口
- **全程留痕、可溯源** — 任务、计划 DAG、会话、事件全记录；progress / risk 随手记，过程、结果与已做事项都可回溯
- **逐项验收 + 阻塞修复引导** — 验收 criterion 逐项核验（passed / failed / blocked）；blocker 带原因、影响与可执行的修复动作
- **完成门禁闭环** — plan → session → progress → result → review 缺一不可，无法凭"口头完成"关任务
- **Project Memory** — 长期记忆只经 typed command 写入；disputed / stale / secret 默认不注入
- **Cockpit 可视化** — 计划图、节点抽屉、验收进度、blocker 面板、AI 用量面板一站式呈现

## 围绕工作流的配套

### 项目管理与启动 —— 工作流的入口

- 指定工作区目录，自动扫描并识别 Node.js / Rust / Python / Java / Go / .NET 等项目
- 标签分类（IDE、CLI、环境等）+ 拖拽排序，点一下就能用对应工具打开项目；也可用"自定义启动"填入任意命令
- 打开的项目以标签页常驻，顺序持久化保存
- 卡片上直接显示当前分支和变更状态
- 深色模式跟随系统或手动切换；界面支持简体中文 / 繁體中文 / English

### Agent 配置与用量 —— 工作流的资源侧

- **Agent Profiles** — 统一管理 OpenCode / Claude Code / Codex 的模型、Provider、凭据引用与思考档位；协议能力自动诊断（直连 / 适配 / 不可用），凭据只接受环境变量或系统 Secret Store 引用，从不读取 auth 文件
- **AI 用量统计** — 本地只读汇总 Claude Code / Codex / OpenCode 的 token 记录，费用由真实 token 数推导；异常数据 fail-closed，绝不把"0"当成没用量，也绝不虚构账单

### 基础设施 —— 支撑体验的底座

- **AI 网关** — 内置代理服务，支持多供应商负载均衡、模型映射、Claude Code 协议转换；默认只监听本机回环地址 `127.0.0.1`
- **Portable** — 绿色免安装，配置文件放在程序旁边的 `data` 目录
- **多语言** — 界面内置简 / 繁 / 英三语

## 安装与下载

### macOS：Homebrew

推荐用 Homebrew 安装 macOS 版本：

```bash
brew install --cask chenm0m/vibehub/vibehub
```

更新：

```bash
brew update
brew upgrade --cask vibehub
```

Homebrew tap 仓库为 `ChenM0M/homebrew-vibehub`。每次 GitHub Release（含预览版）发布后，CI 会根据 Apple Silicon 和 Intel 两个 DMG 产物自动更新 cask。

### 手动下载

[→ Releases 页面](https://github.com/ChenM0M/VibeHub/releases)

| 平台 | 格式 |
|------|------|
| Windows | `.exe` 安装包 / `Portable.zip` 便携版 |
| macOS | `.dmg` (Apple Silicon & Intel) |
| Linux | `.deb` / `.AppImage` |

Windows / Linux Portable 版解压即用。macOS 安装版的配置和 AI 网关数据会写入系统应用数据目录：

```text
~/Library/Application Support/VibeHub
```

如果确实需要便携模式，可以用 `VIBEHUB_PORTABLE=1` 启动，此时配置会写到可执行文件旁边的 `data/`。普通 macOS `.app` / DMG / Homebrew 安装不建议使用便携模式，因为应用包内部通常不可写。

## V3 工作流快速上手

V3 内核是一个独立 CLI（也提供同名 MCP server），可以在任意项目目录直接使用：

```bash
# 查看 / 初始化 / 迁移一个项目的 V3 布局
vibehub v3 <project> doctor
vibehub v3 <project> init
vibehub v3 <project> migrate

# 创建任务并查看任务与计划
vibehub v3 <project> task-create <request.json>
vibehub v3 <project> task-view <task_id>
vibehub v3 <project> task-lifecycle . <task_id>

# 会话与验收
vibehub v3 <project> session-open ...   # 开工前打开会话
vibehub v3 <project> event-log ...      # 里程碑 progress / 风险 risk
vibehub v3 <project> criterion-review ...  # 逐项验收
vibehub v3 <project> task-completion-propose ...  # 请求用户确认完成
```

接入 Agent 只需两步：让 Agent 读取仓库里的工作流编排说明（`AGENTS.md`），并给它配置 V3 MCP server：

```json
{
  "mcpServers": {
    "vibehub": {
      "command": "vibehub",
      "args": ["mcp-stdio", "/path/to/your/project"]
    }
  }
}
```

所有命令输出 JSON。更多细节见：

- [`docs/v3/agent-release-process.md`](docs/v3/agent-release-process.md) — Agent 工作与发布验收的强制流程
- [`docs/v3/migration-guide.md`](docs/v3/migration-guide.md) — V2 → V3 迁移与布局状态
- [`docs/v3/agent-profiles.md`](docs/v3/agent-profiles.md) — Agent Profiles 支持矩阵与 Secret 模型
- [`docs/v3/usage-source-capabilities.md`](docs/v3/usage-source-capabilities.md) — AI 用量数据源能力边界
- [`contracts/v3/`](contracts/v3/README.md) — 视图与命令 JSON Schema 契约

## 从源码运行

需要 Node.js 20+ 和 Rust 1.77.2+。

```bash
git clone https://github.com/ChenM0M/VibeHub.git
cd VibeHub
npm install
npm run tauri dev
```

macOS 可以直接跑环境检查和开发启动脚本：

```bash
./start-dev.sh --check
./start-dev.sh
```

构建发行版：

```bash
npm run tauri build
```

只构建 V3 CLI（可独立分发）：

```bash
cargo build --locked -p vibehub-cli --bin vibehub
```

常用质量门禁：

```bash
npm run release:check        # 版本一致性（package / lockfile / 两个 V3 crate / Tauri / tauri.conf.json）
npm run build                # 前端类型检查与构建
npm run v3:contracts:check   # V3 契约、fixtures 与生成类型校验
npm run v3:mcp:check         # MCP 契约测试
```

平台依赖：

- Windows → Visual Studio Build Tools
- macOS → Xcode Command Line Tools
- Linux → `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev`

## 发布流程

1. 版本修改后运行 `npm run release:check`，确保 `package.json`、lockfile、两个 V3 crate、Tauri Cargo 与 `tauri.conf.json` 版本一致；校验 tag 可用 `RELEASE_TAG=v3.2.0 npm run release:check`。
2. 创建版本标签（例如 `v3.2.0`；含 `-` 的标签为预览版）。
3. `Release` workflow 构建 Windows、Linux、macOS Apple Silicon、macOS Intel 产物，创建草稿 Release 并附各平台 SHA-256 清单；只有全部平台成功且必需产物校验通过后，草稿才会被发布。
4. 发布后直接调用可复用的 `Update Homebrew Cask` workflow，用实际发布资产的 DMG 与 SHA-256 更新 `ChenM0M/homebrew-vibehub`（需要 `HOMEBREW_TAP_TOKEN`；Release 链路上缺失会明确失败，避免 cask 静默滞后）。
5. Apple / Windows 代码签名是可选增强：凭据完整时启用正式签名，缺失时产物为 macOS ad-hoc / Windows unsigned，不等同于正式签名。
6. Windows 原生手测项（`A07`、`E07`、`F07–F10`、`G05`）必须在发布后，使用同一版本正式 Release 的 Windows artifact 与真实 SHA-256 在 Windows 主机完成。

详见 [`docs/v3/agent-release-process.md`](docs/v3/agent-release-process.md)。

## 项目结构

```
VibeHub/
├── src/                     # React + TypeScript 前端
│   ├── pages/               # Home / AgentProfiles / Gateway / Settings / About
│   ├── v3/                  # V3 Cockpit（计划图、验收进度、blocker、AI 用量面板）
│   └── legacy-v2/           # 旧协议入口（只读归档与迁移）
├── src-tauri/               # Tauri 桌面壳与 Rust 命令（扫描器、启动器、网关、Agent Profiles、用量读取）
├── crates/
│   ├── vibehub-core/        # V3 领域核心（事件、投影、验证器）
│   └── vibehub-cli/         # vibehub CLI 与 MCP server
├── contracts/v3/            # V3 视图与命令 JSON Schema 契约
├── docs/v3/                 # V3 交付、流程与迁移文档
├── scripts/                 # 契约生成、门禁与发布检查脚本
└── .vibehub/                # 项目内 V3 工作流状态目录（事件、投影、任务）
```

## 标签和启动是怎么工作的

VibeHub 的核心概念是**标签**。每个标签可以绑定一个启动配置（可执行文件 + 参数 + 环境变量），分类为 IDE、CLI、环境等。

给项目关联标签后，点击启动会按标签类型执行对应操作 —— IDE 类会把项目路径作为参数传递，CLI 类会在项目目录下打开新窗口。

也可以跳过标签，直接用"自定义启动"填入任意命令。

### CLI 标签示例

CLI 标签可以选择终端应用。不同系统会展示适合当前平台的选项；不认识的终端值会安全回退到系统默认启动方式。

| 想要的效果 | 分类 | Terminal | Executable | Args |
| --- | --- | --- | --- | --- |
| 用 Warp 打开项目并运行 OpenCode | CLI | `Warp` | `opencode` | 留空或填写参数 |
| 用 Warp 打开项目并运行 Claude Code | CLI | `Warp` | `claude` | 留空或填写参数 |
| 用 Warp 打开项目并运行 AMP | CLI | `Warp` | `amp` | 留空或填写参数 |
| 用 iTerm 打开项目并运行 OpenCode | CLI | `iTerm` | `opencode` | 留空或填写参数 |
| 用系统 Terminal 运行 npm dev | CLI | `Terminal` | `npm` | `run dev` |
| 用 Windows Terminal 运行 OpenCode | CLI | `WindowsTerminal` | `opencode` | 留空或填写参数 |
| 用 PowerShell 运行 Claude Code | CLI | `PowerShell` | `claude` | 留空或填写参数 |
| 用 cmd 运行 npm dev | CLI | `CommandPrompt` | `npm` | `run dev` |

如果只需要打开 Warp 到项目目录而不自动运行命令，Warp 官方也支持 `warp://action/new_tab?path=<项目路径>` 这类 URI；VibeHub 的 CLI 标签则更适合"进入项目目录并启动某个 CLI agent"。

Agent 的模型、Provider 与凭据引用由 **Agent Profiles** 统一管理，与标签启动配置相互独立。

## 贡献

PR 和 Issue 都欢迎。

## 许可证

[Apache License 2.0](LICENSE)

## 致谢

- [Tauri](https://tauri.app/) — 跨平台桌面应用框架
- [React](https://react.dev/) + [TailwindCSS](https://tailwindcss.com/) — 前端
- [b4u2cc](https://github.com/CassiopeiaCode/b4u2cc) — Claude Code 协议转换参考

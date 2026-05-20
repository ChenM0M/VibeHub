# VibeHub

[English](README_EN.md) | [简体中文](README.md) | [繁體中文](README_TC.md)

![alt text](image.png)

> 管理散落在各处的项目，用标签分类，一键启动你常用的 IDE 和 CLI 工具。
> 还内置了 AI 网关，帮你代理和分发 AI 请求。

![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)

## 它能干什么

- **项目管理** — 指定工作区目录，自动扫描并识别 Node.js / Rust / Python / Java / Go / .NET 等项目
- **标签 + 启动** — 给项目打标签（IDE、CLI、环境等），点一下就能用对应工具打开项目
- **AI 网关** — 内置代理服务，支持多供应商负载均衡、模型映射、Claude Code 协议转换
- **拖拽排序** — 项目卡片支持拖拽排列，顺序持久化保存
- **Portable** — 绿色免安装，配置文件就放在程序旁边的 `data` 目录
- **Git 信息** — 卡片上直接显示当前分支和变更状态
- **深色模式** — 跟随系统或手动切换

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

Homebrew tap 仓库名约定为 `ChenM0M/homebrew-vibehub`。每次 GitHub Release 从草稿发布后，CI 会根据 Apple Silicon 和 Intel 两个 DMG 产物自动更新 cask。预览版 / prerelease 也走同一套 cask 更新流程，指向当前发布的预览版 DMG。

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

AI 网关默认只监听本机回环地址 `127.0.0.1`，不会暴露到局域网。

## 从源码运行

需要 Node.js 18+ 和 Rust 1.70+。

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

平台依赖：
- Windows → Visual Studio Build Tools
- macOS → Xcode Command Line Tools
- Linux → `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev`

## 发布流程

1. 创建版本标签，例如 `v2.0.0`。
2. `Release` workflow 会构建 Windows、Linux、macOS Apple Silicon、macOS Intel 产物，并创建草稿 Release。
3. 按草稿 Release 里的检查清单测试两个 macOS DMG，确认能首次启动、打开项目、启动 CLI/IDE、AI 网关可保存配置。
4. 发布草稿 Release。
5. `Update Homebrew Cask` workflow 会下载公开的 DMG、计算 SHA256，并用实际发布资产文件名更新 `ChenM0M/homebrew-vibehub` 里的 `Casks/vibehub.rb`，适配正式版和预览版。

Homebrew 自动更新需要先创建 `ChenM0M/homebrew-vibehub` 仓库，并在本仓库 Secrets 里配置 `HOMEBREW_TAP_TOKEN`。如果要让 macOS 用户双击即正常打开，Release workflow 还需要配置 Apple 签名/公证相关 Secrets：`APPLE_CERTIFICATE`、`APPLE_CERTIFICATE_PASSWORD`、`APPLE_SIGNING_IDENTITY`、`APPLE_ID`、`APPLE_PASSWORD`、`APPLE_TEAM_ID`。

## 项目结构

```
VibeHub/
├── src/                 # React + TypeScript 前端
├── src-tauri/           # Rust 后端
│   └── src/
│       ├── main.rs      # 入口
│       ├── commands.rs  # Tauri 命令
│       ├── scanner.rs   # 项目扫描器
│       ├── launcher.rs  # 启动器
│       ├── storage.rs   # 配置读写
│       └── models.rs    # 数据结构
└── package.json
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

如果只需要打开 Warp 到项目目录而不自动运行命令，Warp 官方也支持 `warp://action/new_tab?path=<项目路径>` 这类 URI；VibeHub 的 CLI 标签则更适合“进入项目目录并启动某个 CLI agent”。

## 贡献

PR 和 Issue 都欢迎。

## 许可证

[Apache License 2.0](LICENSE)

## 致谢

- [Tauri](https://tauri.app/) — 跨平台桌面应用框架
- [React](https://react.dev/) + [TailwindCSS](https://tailwindcss.com/) — 前端
- [b4u2cc](https://github.com/CassiopeiaCode/b4u2cc) — Claude Code 协议转换参考

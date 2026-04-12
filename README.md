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

## 下载

[→ Releases 页面](https://github.com/ChenM0M/VibeHub/releases)

| 平台 | 格式 |
|------|------|
| Windows | `.exe` 安装包 / `Portable.zip` 便携版 |
| macOS | `.dmg` (Intel & Apple Silicon) |
| Linux | `.deb` / `.AppImage` |

Portable 版解压即用，配置自动存在 `data/` 下，删掉文件夹就是干净卸载。

## 从源码运行

需要 Node.js 18+ 和 Rust 1.70+。

```bash
git clone https://github.com/ChenM0M/VibeHub.git
cd VibeHub
npm install
npm run tauri dev
```

构建发行版：

```bash
npm run tauri build
```

平台依赖：
- Windows → Visual Studio Build Tools
- macOS → Xcode Command Line Tools
- Linux → `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev`

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

## 贡献

PR 和 Issue 都欢迎。

## 许可证

[Apache License 2.0](LICENSE)

## 致谢

- [Tauri](https://tauri.app/) — 跨平台桌面应用框架
- [React](https://react.dev/) + [TailwindCSS](https://tailwindcss.com/) — 前端
- [b4u2cc](https://github.com/CassiopeiaCode/b4u2cc) — Claude Code 协议转换参考

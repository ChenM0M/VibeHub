# VibeHub

[English](README_EN.md) | [简体中文](README.md) | [繁體中文](README_TC.md)
![alt text](image.png)
你的本地指揮中心。解鎖無限可能。靈活標籤管理項目，一鍵啟動 IDE、腳本或 AI 網關。專為 VibeCoding 打造。

![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)
![Version](https://img.shields.io/badge/version-v1.2.0-green)

## ✨ 特性

- 🤖 **AI 網關集成**: 內置高性能 AI 網關，無縫連接未來開發體驗
- 🚀 **快速啟動**: 一鍵啟動 VSCode、CLI 工具等，VibeCoding 友好
- 📁 **本地多項目管理**: 自動掃描和管理本地項目，支持多種語言和框架
- 🏷️ **標籤系統**: 靈活的項目分類和過濾
- 💾 **Portable 模式**: 綠色便攜，配置隨行
- 🎨 **現代 UI**: Notion 風格的簡約設計，支持深色模式
- 🔄 **Git 集成**: 顯示分支和更改狀態
- ⚡ **性能優化**: 基於 Rust 和 Tauri，快速且輕量

## 📦 下載

前往 [Releases]() 頁面下載最新版本 (v1.2.0)：

- **Windows**: `VibeHub-Windows-Portable.zip` (推薦) 或 `.msi` 安裝包
- **macOS**: `.dmg` 或 `.app.tar.gz`
- **Linux**: `.deb` 或 `.AppImage`

## 🚀 快速開始

### Portable 版本（Windows）

1. 下載 `VibeHub-Windows-Portable.zip`
2. 解壓到任意目錄
3. 運行 `vibehub.exe`
4. 所有配置自動保存在 `data` 文件夾

### 安裝版本

1. 下載對應平台的安裝包
2. 按照提示安裝
3. 啟動應用

## 🛠️ 開發

### 前置要求

- Node.js 18+
- Rust 1.70+
- 平台特定依賴：
  - Windows: Visual Studio Build Tools
  - macOS: Xcode Command Line Tools
  - Linux: `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev`

### 本地運行

```bash
# 克隆倉庫
git clone https://github.com/VibeCoding/VibeHub.git
cd VibeHub

# 安裝依賴
npm install

# 開發模式運行
npm run tauri dev

# 構建
npm run tauri build
```

### 項目結構

```
VibeHub/
├── src/                    # 前端代碼 (React + TypeScript)
├── src-tauri/              # 後端代碼 (Rust)
│   ├── src/
│   │   ├── main.rs        # 主入口
│   │   ├── commands.rs    # Tauri 命令
│   │   ├── scanner.rs     # 項目掃描
│   │   ├── launcher.rs    # 啟動器
│   │   ├── storage.rs     # 數據存儲
│   │   ├── models.rs      # 數據模型
│   └── Cargo.toml
└── package.json
```

## 📝 功能說明

### 工作區管理

- 添加工作區目錄
- 自動掃描識別項目類型
- 支持項目類型：Node.js、Rust、Python、Java、Go、.NET 等

### 項目配置

- 名稱和描述
- 自定義標籤
- 收藏/星標
- 自定義圖標

### 啟動配置

支持配置各種工具：
- IDE（VSCode、IntelliJ IDEA 等）
- CLI 工具（Claude Code、Gemini CLI、AntiGravity 等）
- 終端
- 自定義程序

### 標籤系統

內置標籤分類：
- 工作區分組
- IDE 工具
- CLI 工具
- 環境配置
- 自定義標籤

## 🤝 貢獻

歡迎貢獻！請查看 [CONTRIBUTING.md](CONTRIBUTING.md)

## 📄 許可證

Apache License 2.0 - 詳見 [LICENSE](LICENSE)

## 🙏 致謝

- [Tauri](https://tauri.app/) - 跨平台應用框架
- [React](https://react.dev/) - UI 框架
- [TailwindCSS](https://tailwindcss.com/) - CSS 框架

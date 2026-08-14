# VibeHub

[English](README_EN.md) | [简体中文](README.md) | [繁體中文](README_TC.md)

![alt text](image.png)

> VibeHub 是一個圍繞 Agent 工作流體驗打造的 All-in-One 工具平台。它源於個人 Vibe
> 開發過程中的痛點：Agent 工作的計劃、進度、驗收與阻塞缺少可見、可追溯的載體。
> VibeHub 用任務 / 計劃 / 會話 / 事件驅動的工作流，把這些「看不見的過程」變成看板與
> 計劃圖。它不綁定任何特定的 Agent Harness —— 只要能讀取倉庫裡的工作流編排說明、
> 並能呼叫標準 MCP 工具，你的 Agent 就能接入。桌面端用可視化 Cockpit 降低使用門檻，
> 而核心本質是一個獨立的 CLI，可以拆出去單獨使用或 Fork 改造。

![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)
![GitHub Release](https://img.shields.io/github/v/release/ChenM0M/VibeHub)

[更新日誌](CHANGELOG.md) · [Releases](https://github.com/ChenM0M/VibeHub/releases)

## 主要特色：V3 工作流

- **Harness 無關** — 不綁定 OpenCode / Codex / Claude Code 等特定 Harness；Agent 只要能讀取倉庫裡的工作流編排說明、並能呼叫 V3 MCP 工具，就能接入同一套工作流
- **CLI 核心 + 可視化前端** — 本質是一個獨立 CLI（`vibehub v3 ...`），可單獨拆出使用或 Fork 改造；桌面端的 V3 Cockpit 只是它的可視化入口
- **全程留痕、可溯源** — 任務、計劃 DAG、會話、事件全記錄；progress / risk 隨手記，過程、結果與已做事項都可回溯
- **逐項驗收 + 阻塞修復引導** — 驗收 criterion 逐項核驗（passed / failed / blocked）；blocker 帶原因、影響與可執行的修復動作
- **完成門禁閉環** — plan → session → progress → result → review 缺一不可，無法憑「口頭完成」關任務
- **Project Memory** — 長期記憶只經 typed command 寫入；disputed / stale / secret 預設不注入
- **Cockpit 可視化** — 計劃圖、節點抽屜、驗收進度、blocker 面板、AI 用量面板一站式呈現

## 圍繞工作流的配套

### 專案管理與啟動 —— 工作流的入口

- 指定工作區目錄，自動掃描並識別 Node.js / Rust / Python / Java / Go / .NET 等專案
- 標籤分類（IDE、CLI、環境等）+ 拖曳排序，點一下就能用對應工具開啟專案；也可用「自訂啟動」填入任意指令
- 開啟的專案以分頁常駐，順序持久化儲存
- 卡片上直接顯示目前分支和變更狀態
- 深色模式跟隨系統或手動切換；介面支援简体中文 / 繁體中文 / English

### Agent 設定與用量 —— 工作流的資源側

- **Agent Profiles** — 統一管理 OpenCode / Claude Code / Codex 的模型、Provider、憑證引用與思考檔位；協定能力自動診斷（直連 / 適配 / 不可用），憑證只接受環境變數或系統 Secret Store 引用，從不讀取 auth 檔案
- **AI 用量統計** — 本機唯讀彙總 Claude Code / Codex / OpenCode 的 token 記錄，費用由真實 token 數推導；異常資料 fail-closed，絕不把「0」當成沒用量，也絕不虛構帳單

### 基礎設施 —— 支撐體驗的底座

- **AI 閘道** — 內建代理服務，支援多供應商負載均衡、模型映射、Claude Code 協定轉換；預設只監聽本機回環位址 `127.0.0.1`
- **Portable** — 綠色免安裝，設定檔放在程式旁邊的 `data` 目錄
- **多語言** — 介面內建簡 / 繁 / 英三語

## 安裝與下載

### macOS：Homebrew

推薦用 Homebrew 安裝 macOS 版本：

```bash
brew install --cask chenm0m/vibehub/vibehub
```

更新：

```bash
brew update
brew upgrade --cask vibehub
```

Homebrew tap 倉庫為 `ChenM0M/homebrew-vibehub`。每次 GitHub Release（含預覽版）發布後，CI 會根據 Apple Silicon 和 Intel 兩個 DMG 產物自動更新 cask。

### 手動下載

[→ Releases 頁面](https://github.com/ChenM0M/VibeHub/releases)

| 平台 | 格式 |
|------|------|
| Windows | `.exe` 安裝包 / `Portable.zip` 便攜版 |
| macOS | `.dmg` (Apple Silicon & Intel) |
| Linux | `.deb` / `.AppImage` |

Windows / Linux Portable 版解壓即用。macOS 安裝版的設定和 AI 閘道資料會寫入系統應用資料目錄：

```text
~/Library/Application Support/VibeHub
```

如果確實需要便攜模式，可以用 `VIBEHUB_PORTABLE=1` 啟動，此時設定會寫到可執行檔旁邊的 `data/`。一般 macOS `.app` / DMG / Homebrew 安裝不建議使用便攜模式，因為應用套件內部通常不可寫。

## V3 工作流快速上手

V3 核心是一個獨立 CLI（也提供同名 MCP server），可以在任意專案目錄直接使用：

```bash
# 查看 / 初始化 / 遷移一個專案的 V3 佈局
vibehub v3 <project> doctor
vibehub v3 <project> init
vibehub v3 <project> migrate

# 建立任務並查看任務與計劃
vibehub v3 <project> task-create <request.json>
vibehub v3 <project> task-view <task_id>
vibehub v3 <project> task-lifecycle . <task_id>

# 會話與驗收
vibehub v3 <project> session-open ...   # 開工前開啟會話
vibehub v3 <project> event-log ...      # 里程碑 progress / 風險 risk
vibehub v3 <project> criterion-review ...  # 逐項驗收
vibehub v3 <project> task-completion-propose ...  # 請求使用者確認完成
```

接入 Agent 只需兩步：讓 Agent 讀取倉庫裡的工作流編排說明（`AGENTS.md`），並給它設定 V3 MCP server：

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

所有指令輸出 JSON。更多細節見：

- [`docs/v3/agent-release-process.md`](docs/v3/agent-release-process.md) — Agent 工作與發布驗收的強制流程
- [`docs/v3/migration-guide.md`](docs/v3/migration-guide.md) — V2 → V3 遷移與佈局狀態
- [`docs/v3/agent-profiles.md`](docs/v3/agent-profiles.md) — Agent Profiles 支援矩陣與 Secret 模型
- [`docs/v3/usage-source-capabilities.md`](docs/v3/usage-source-capabilities.md) — AI 用量資料來源能力邊界
- [`contracts/v3/`](contracts/v3/README.md) — 視圖與指令 JSON Schema 契約

## 從原始碼執行

需要 Node.js 20+ 和 Rust 1.77.2+。

```bash
git clone https://github.com/ChenM0M/VibeHub.git
cd VibeHub
npm install
npm run tauri dev
```

macOS 可以直接跑環境檢查和開發啟動腳本：

```bash
./start-dev.sh --check
./start-dev.sh
```

建置發行版：

```bash
npm run tauri build
```

只建置 V3 CLI（可獨立分發）：

```bash
cargo build --locked -p vibehub-cli --bin vibehub
```

常用品質門禁：

```bash
npm run release:check        # 版本一致性（package / lockfile / 兩個 V3 crate / Tauri / tauri.conf.json）
npm run build                # 前端型別檢查與建置
npm run v3:contracts:check   # V3 契約、fixtures 與生成型別校驗
npm run v3:mcp:check         # MCP 契約測試
```

平台依賴：

- Windows → Visual Studio Build Tools
- macOS → Xcode Command Line Tools
- Linux → `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev`

## 發布流程

1. 版本修改後執行 `npm run release:check`，確保 `package.json`、lockfile、兩個 V3 crate、Tauri Cargo 與 `tauri.conf.json` 版本一致；校驗 tag 可用 `RELEASE_TAG=v3.2.0 npm run release:check`。
2. 建立版本標籤（例如 `v3.2.0`；含 `-` 的標籤為預覽版）。
3. `Release` workflow 建置 Windows、Linux、macOS Apple Silicon、macOS Intel 產物，建立草稿 Release 並附各平台 SHA-256 清單；只有全部平台成功且必需產物校驗通過後，草稿才會被發布。
4. 發布後直接呼叫可重用的 `Update Homebrew Cask` workflow，用實際發布資產的 DMG 與 SHA-256 更新 `ChenM0M/homebrew-vibehub`（需要 `HOMEBREW_TAP_TOKEN`；Release 鏈路上缺失會明確失敗，避免 cask 靜默滯後）。
5. Apple / Windows 程式碼簽章是選用增強：憑證完整時啟用正式簽章，缺失時產物為 macOS ad-hoc / Windows unsigned，不等同於正式簽章。
6. Windows 原生手測項（`A07`、`E07`、`F07–F10`、`G05`）必須在發布後，使用同一版本正式 Release 的 Windows artifact 與真實 SHA-256 在 Windows 主機完成。

詳見 [`docs/v3/agent-release-process.md`](docs/v3/agent-release-process.md)。

## 專案結構

```
VibeHub/
├── src/                     # React + TypeScript 前端
│   ├── pages/               # Home / AgentProfiles / Gateway / Settings / About
│   ├── v3/                  # V3 Cockpit（計劃圖、驗收進度、blocker、AI 用量面板）
│   └── legacy-v2/           # 舊協定入口（唯讀歸檔與遷移）
├── src-tauri/               # Tauri 桌面殼與 Rust 指令（掃描器、啟動器、閘道、Agent Profiles、用量讀取）
├── crates/
│   ├── vibehub-core/        # V3 領域核心（事件、投影、驗證器）
│   └── vibehub-cli/         # vibehub CLI 與 MCP server
├── contracts/v3/            # V3 視圖與指令 JSON Schema 契約
├── docs/v3/                 # V3 交付、流程與遷移文件
├── scripts/                 # 契約生成、門禁與發布檢查腳本
└── .vibehub/                # 專案內 V3 工作流狀態目錄（事件、投影、任務）
```

## 標籤和啟動是怎麼運作的

VibeHub 的核心概念是**標籤**。每個標籤可以綁定一個啟動設定（可執行檔 + 參數 + 環境變數），分類為 IDE、CLI、環境等。

給專案關聯標籤後，點擊啟動會按標籤型別執行對應操作 —— IDE 類會把專案路徑作為參數傳遞，CLI 類會在專案目錄下開啟新視窗。

也可以跳過標籤，直接用「自訂啟動」填入任意指令。

### CLI 標籤範例

CLI 標籤可以選擇終端應用。不同系統會展示適合目前平台的選項；不認識的終端值會安全回退到系統預設啟動方式。

| 想要的效果 | 分類 | Terminal | Executable | Args |
| --- | --- | --- | --- | --- |
| 用 Warp 開啟專案並執行 OpenCode | CLI | `Warp` | `opencode` | 留空或填寫參數 |
| 用 Warp 開啟專案並執行 Claude Code | CLI | `Warp` | `claude` | 留空或填寫參數 |
| 用 Warp 開啟專案並執行 AMP | CLI | `Warp` | `amp` | 留空或填寫參數 |
| 用 iTerm 開啟專案並執行 OpenCode | CLI | `iTerm` | `opencode` | 留空或填寫參數 |
| 用系統 Terminal 執行 npm dev | CLI | `Terminal` | `npm` | `run dev` |
| 用 Windows Terminal 執行 OpenCode | CLI | `WindowsTerminal` | `opencode` | 留空或填寫參數 |
| 用 PowerShell 執行 Claude Code | CLI | `PowerShell` | `claude` | 留空或填寫參數 |
| 用 cmd 執行 npm dev | CLI | `CommandPrompt` | `npm` | `run dev` |

如果只需要開啟 Warp 到專案目錄而不自動執行指令，Warp 官方也支援 `warp://action/new_tab?path=<專案路徑>` 這類 URI；VibeHub 的 CLI 標籤則更適合「進入專案目錄並啟動某個 CLI agent」。

Agent 的模型、Provider 與憑證引用由 **Agent Profiles** 統一管理，與標籤啟動設定相互獨立。

## 貢獻

PR 和 Issue 都歡迎。

## 授權

[Apache License 2.0](LICENSE)

## 致謝

- [Tauri](https://tauri.app/) — 跨平台桌面應用框架
- [React](https://react.dev/) + [TailwindCSS](https://tailwindcss.com/) — 前端
- [b4u2cc](https://github.com/CassiopeiaCode/b4u2cc) — Claude Code 協定轉換參考

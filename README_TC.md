# VibeHub

[English](README_EN.md) | [简体中文](README.md) | [繁體中文](README_TC.md)

![alt text](image.png)

> 把散落各處的專案集中管理，用標籤分類，一鍵啟動常用的 IDE 和 CLI 工具。
> 還內建了 AI 閘道，幫你代理和分發 AI 請求。

![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)

## 它能做什麼

- **專案管理** — 指定工作區目錄，自動掃描並識別 Node.js / Rust / Python / Java / Go / .NET 等專案
- **標籤 + 啟動** — 給專案打標籤（IDE、CLI、環境等），點一下就能用對應工具開啟專案
- **AI 閘道** — 內建代理服務，支援多供應商負載均衡、模型映射、Claude Code 協議轉換
- **拖曳排序** — 專案卡片支援拖曳排列，順序會持久化儲存
- **Portable** — 綠色免安裝，設定檔就放在程式旁邊的 `data` 目錄
- **Git 資訊** — 卡片上直接顯示目前分支和變更狀態
- **深色模式** — 跟隨系統或手動切換

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

Homebrew tap 倉庫名約定為 `ChenM0M/homebrew-vibehub`。每次 GitHub Release 從草稿發布後，CI 會根據 Apple Silicon 和 Intel 兩個 DMG 產物自動更新 cask。預覽版 / prerelease 也走同一套 cask 更新流程，指向目前發布的預覽版 DMG。

### 手動下載

[→ Releases 頁面](https://github.com/ChenM0M/VibeHub/releases)

| 平台 | 格式 |
|------|------|
| Windows | `.exe` 安裝包 / 便攜執行檔 |
| macOS | `.dmg` (Intel & Apple Silicon) |
| Linux | `.deb` / `.AppImage` |

Windows / Linux Portable 版解壓即用。macOS 安裝版的設定和 AI 閘道資料會寫入系統應用資料目錄：

```text
~/Library/Application Support/VibeHub
```

如果確實需要便攜模式，可以用 `VIBEHUB_PORTABLE=1` 啟動，此時設定會寫到可執行檔旁邊的 `data/`。普通 macOS `.app` / DMG / Homebrew 安裝不建議使用便攜模式，因為應用程式包內部通常不可寫。

AI 閘道預設只監聽本機回環位址 `127.0.0.1`，不會暴露到區域網路。

## 從原始碼執行

需要 Node.js 18+ 和 Rust 1.70+。

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

平台相依套件：
- Windows → Visual Studio Build Tools
- macOS → Xcode Command Line Tools
- Linux → `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev`

## 發布流程

1. 建立版本標籤，例如 `v2.0.0`。
2. `Release` workflow 會建置 Windows、Linux、macOS Apple Silicon、macOS Intel 產物，並建立草稿 Release。
3. 按草稿 Release 裡的檢查清單測試兩個 macOS DMG，確認能首次啟動、開啟專案、啟動 CLI/IDE、AI 閘道可儲存設定。
4. 發布草稿 Release。
5. `Update Homebrew Cask` workflow 會下載公開的 DMG、計算 SHA256，並用實際發布資產檔名更新 `ChenM0M/homebrew-vibehub` 裡的 `Casks/vibehub.rb`，適配正式版和預覽版。

Homebrew 自動更新需要先建立 `ChenM0M/homebrew-vibehub` 倉庫，並在本倉庫 Secrets 裡設定 `HOMEBREW_TAP_TOKEN`。如果要讓 macOS 使用者雙擊即正常開啟，Release workflow 還需要設定 Apple 簽名/公證相關 Secrets：`APPLE_CERTIFICATE`、`APPLE_CERTIFICATE_PASSWORD`、`APPLE_SIGNING_IDENTITY`、`APPLE_ID`、`APPLE_PASSWORD`、`APPLE_TEAM_ID`。

## 專案結構

```
VibeHub/
├── src/                 # React + TypeScript 前端
├── src-tauri/           # Rust 後端
│   └── src/
│       ├── main.rs      # 入口
│       ├── commands.rs  # Tauri 命令
│       ├── scanner.rs   # 專案掃描器
│       ├── launcher.rs  # 啟動器
│       ├── storage.rs   # 設定讀寫
│       └── models.rs    # 資料結構
└── package.json
```

## 標籤和啟動是怎麼運作的

VibeHub 的核心概念是**標籤**。每個標籤可以綁定一組啟動設定（可執行檔 + 參數 + 環境變數），分類為 IDE、CLI、環境等。

給專案關聯標籤後，點選啟動會依標籤類型執行對應操作 —— IDE 類會把專案路徑作為參數傳入，CLI 類會在專案目錄下開啟新視窗。

也可以跳過標籤，直接用「自訂啟動」填入任意命令。

## 貢獻

PR 和 Issue 都歡迎。

## 授權

[Apache License 2.0](LICENSE)

## 致謝

- [Tauri](https://tauri.app/) — 跨平台桌面應用框架
- [React](https://react.dev/) + [TailwindCSS](https://tailwindcss.com/) — 前端
- [b4u2cc](https://github.com/CassiopeiaCode/b4u2cc) — Claude Code 協議轉換參考

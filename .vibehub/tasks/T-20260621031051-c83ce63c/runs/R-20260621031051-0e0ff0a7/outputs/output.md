## Intent
- `user_confirmed`: 修复 macOS 顶部仍像原生白色标题栏的问题，同时保留标题栏应有的窗口交互：顶部空白区域可拖拽，双击可窗口化最大化/还原，不触发绿色按钮的全屏模式。
- `user_confirmed`: 调整 macOS 左上角 VibeHub 图标/标题位置，避免挤在红黄绿窗口按钮旁边显得突兀。
- `user_confirmed`: 继续修复左侧 VibeHub 图标视觉不够美观/略偏下的问题，并修复进入项目详情页后点击 VibeHub 品牌不能返回首页的问题。

## Scope
- `agent_reported`: 仅调整主窗口 macOS 标题栏配置、顶部 Header/Sidebar 布局与 drag region；不改项目卡片、用量统计、VibeHub workflow 状态机或其他平台窗口按钮行为。
- `agent_reported`: Windows/Linux 继续显示现有自绘最小化/最大化/关闭按钮；本轮只让 Header 使用 Tauri 官方 drag region 以保留拖拽并补上双击最大化语义。
- `agent_reported`: 本次增量只调整 React 前端状态流和 Sidebar 品牌位：不改 VibeHub 后端数据读取、不改项目详情内容布局、不改 Tauri native 窗口配置。

## Completed
- `hard_observed`: 将 `src-tauri/tauri.conf.json` 主窗口从无装饰改为 `decorations: true` + `titleBarStyle: Overlay` + `hiddenTitle: true`，并设置 macOS `trafficLightPosition`，让红黄绿按钮保留但标题栏并入应用内容。
- `hard_observed`: 移除 `src-tauri/src/main.rs` 中启动后再调用 `set_decorations(true)` / `set_title_bar_style(Overlay)` 的补丁式窗口配置，避免创建期配置与运行期配置互相覆盖。
- `hard_observed`: `src/components/Header.tsx` 改用 `data-tauri-drag-region="deep"`；Tauri 内置脚本会在非交互区域拖拽窗口，并在双击时调用内部窗口化 maximize toggle。
- `hard_observed`: `src/components/Sidebar.tsx` 在 macOS 上给红黄绿按钮下方留出独立拖拽空白区，并将 VibeHub logo/title 放回拖拽区下方的侧栏品牌位，避免与窗口按钮挤在同一行；非 macOS 仍保留原品牌区。
- `hard_observed`: `src/styles/globals.css` 补齐 `html/body/#root` 高度，避免 overlay/圆角窗口边缘露出非应用背景。
- `hard_observed`: `src/components/Sidebar.tsx` 将品牌位改为语义化按钮，使用固定高度、图标容器、轻微上移和缩放，让 `/app-icon.png` 在侧栏品牌区内视觉更居中；点击品牌仍会清空 selected workspace 并请求回首页。
- `hard_observed`: `src/main.tsx` 新增 `homeResetKey`，每次外层导航到 `home` 都递增，作为首页内部状态的重置信号。
- `hard_observed`: `src/pages/Home.tsx` 接收 `resetKey`，并在 `resetKey` 或 `selectedWorkspaceId` 变化时清空 `selectedProjectId`，确保从项目详情页点击左侧 VibeHub 品牌或工作区入口会退出详情页回到项目列表。
- `hard_observed`: `src/components/Sidebar.tsx` 的工作区列表项现在始终调用 `onNavigate('home')`，即使当前外层页面已经是 home，也能触发 `Home` 内部详情状态重置。
- `hard_observed`: 发版版本号从 `2.0.0-pre.21` bump 到 `2.0.0-pre.22`，因为远端已存在 `v2.0.0-pre.21` 标签；已同步 `package.json`、Tauri 配置、Rust crate version 和 lockfile 主包版本。

## Not Yet Done
- `agent_reported`: 未由 Codex 自动打开真实 GUI 做拖拽和双击手感验证；用户中断/拒绝了上一轮启动 `.app` 的 GUI 操作请求。
- `agent_reported`: 未在 Windows/Linux 真机上手动验证；本轮保持原窗口装饰配置不变，并仅使用 Tauri 官方 drag region 语义降低跨平台风险。

## Key Decisions Made
- `agent_reported`: 不再使用 `decorations: false`，因为它会把红黄绿按钮和系统窗口语义一起拿掉；改用 Tauri 支持的 macOS overlay titlebar。
- `agent_reported`: 不再手写 `onMouseDown -> startDragging`；改用 Tauri 内置 `data-tauri-drag-region`，因为它同时处理拖拽、双击 maximize，以及 macOS 双击后 mouseup 取消逻辑。
- `agent_reported`: macOS 侧栏顶部保留 VibeHub logo/title，但将其下移到红黄绿按钮下方，避免与窗口按钮竞争同一行视觉空间。
- `agent_reported`: 项目详情页返回首页的状态由外层 `homeResetKey` 协调，而不是把项目详情选中状态提升到全局 store；这样改动范围小，避免把仅属于 Home 的 UI 状态泄漏到应用级状态。
- `agent_reported`: 品牌图标继续复用现有 `/app-icon.png`，仅在 Sidebar 呈现层调整容器、阴影、ring、缩放和 `translate-y`，不引入新图标资产。

## Files Changed
- `hard_observed`: `src-tauri/tauri.conf.json`
- `hard_observed`: `src-tauri/src/main.rs`
- `hard_observed`: `src/components/Header.tsx`
- `hard_observed`: `src/components/Sidebar.tsx`
- `hard_observed`: `src/main.tsx`
- `hard_observed`: `src/pages/Home.tsx`
- `hard_observed`: `src/styles/globals.css`
- `hard_observed`: `package.json`
- `hard_observed`: `src-tauri/Cargo.toml`
- `hard_observed`: `Cargo.lock`
- `hard_observed`: `src-tauri/Cargo.lock`
- `hard_observed`: `.vibehub/tasks/T-20260621031051-c83ce63c/runs/R-20260621031051-0e0ff0a7/outputs/output.md`
- `hard_observed`: VibeHub generated state/view files changed after `vibehub switch`.

## Files Reportedly Read
- `hard_observed`: `.vibehub/agent-view/current.md`
- `hard_observed`: `.vibehub/agent-view/current-context.md`
- `hard_observed`: `.vibehub/agent-view/handoff.md`
- `hard_observed`: `.vibehub/rules/hard-rules.md`
- `hard_observed`: `.vibehub/adapters/protocol.md`
- `hard_observed`: `.vibehub/tasks/T-20260621031051-c83ce63c/runs/R-20260621031051-0e0ff0a7/context-packs/align_lite.md`
- `hard_observed`: `.vibehub/workflow.yaml`
- `hard_observed`: `src-tauri/tauri.conf.json`
- `hard_observed`: `src-tauri/src/main.rs`
- `hard_observed`: `src/components/Header.tsx`
- `hard_observed`: `src/components/Sidebar.tsx`
- `hard_observed`: `src/components/Layout.tsx`
- `hard_observed`: `src/pages/Home.tsx`
- `hard_observed`: `src/components/ProjectCard.tsx`
- `hard_observed`: `src/components/VibehubProjectCenter.tsx`
- `hard_observed`: `src/stores/appStore.ts`
- `hard_observed`: `src/services/tauri.ts`
- `hard_observed`: `src/main.tsx`
- `hard_observed`: `src/styles/globals.css`
- `hard_observed`: `package.json`
- `hard_observed`: `node_modules/@tauri-apps/api/core.js`
- `hard_observed`: `node_modules/@tauri-apps/api/app.js`
- `hard_observed`: `node_modules/@tauri-apps/api/window.js`
- `hard_observed`: `node_modules/@tauri-apps/api/window.d.ts`
- `hard_observed`: local Tauri `drag.js` under Cargo registry

## Commands Run
- `hard_observed`: `vibehub status /Users/chenm0m/LocalRepo/VibeHub`
- `hard_observed`: `vibehub switch /Users/chenm0m/LocalRepo/VibeHub T-20260621031051-c83ce63c`
- `hard_observed`: `rg -n "data-tauri-drag-region|startDragging|toggleMaximize|titleBarStyle|trafficLightPosition" ...`
- `hard_observed`: `npm run build`
- `hard_observed`: `cargo check --manifest-path src-tauri/Cargo.toml`
- `hard_observed`: `npm run tauri -- build --target aarch64-apple-darwin --bundles app`
- `hard_observed`: `git status --short`
- `hard_observed`: `git diff --stat`
- `hard_observed`: `git diff -- src-tauri/tauri.conf.json src-tauri/src/main.rs src/components/Header.tsx src/components/Sidebar.tsx src/styles/globals.css`
- `hard_observed`: `vibehub status .`
- `hard_observed`: `vibehub sync /Users/chenm0m/LocalRepo/VibeHub`
- `hard_observed`: `rg --files src`
- `hard_observed`: `rg -n "<Sidebar|currentPage|setCurrentPage|selectedProject|ProjectDetail|onNavigate|onSelect" ...`
- `hard_observed`: `npm run dev -- --host 127.0.0.1`
- `hard_observed`: `node -e "const { chromium } = require('playwright'); ..."` failed because project dependencies do not include Playwright.
- `hard_observed`: `python3 -m http.server 1421 --bind 127.0.0.1` started a temporary local static server for the mock UI verification after sandbox approval.
- `hard_observed`: Browser automation loaded `http://127.0.0.1:1421/vibehub-ui-test.html`, clicked a mock project detail, clicked the Sidebar VibeHub brand, and captured `/private/tmp/vibehub-sidebar-brand-after.png`.
- `hard_observed`: `cp public/app-icon.png /private/tmp/app-icon.png`
- `hard_observed`: `git ls-remote --tags origin 'refs/tags/v2.0.0-pre.21'`
- `hard_observed`: `git ls-remote --tags origin 'refs/tags/v2.0.0-pre.22'`
- `hard_observed`: `npm run build`
- `hard_observed`: `cargo check --manifest-path src-tauri/Cargo.toml`
- `hard_observed`: `npm run tauri -- build --target aarch64-apple-darwin --bundles app`

## Tests Run
- `hard_observed`: `npm run build` passed.
- `hard_observed`: `cargo check --manifest-path src-tauri/Cargo.toml` passed with existing dead-code warnings.
- `hard_observed`: `npm run tauri -- build --target aarch64-apple-darwin --bundles app` passed and produced `/Users/chenm0m/LocalRepo/VibeHub/target/aarch64-apple-darwin/release/bundle/macos/VibeHub.app`.
- `hard_observed`: Re-ran `npm run build` after the Sidebar/Home navigation fix; it passed.
- `hard_observed`: Re-ran `npm run build` after the `2.0.0-pre.22` version bump; it passed.
- `hard_observed`: Re-ran `cargo check --manifest-path src-tauri/Cargo.toml` after the `2.0.0-pre.22` version bump; it passed with existing dead-code warnings.
- `hard_observed`: Re-ran `npm run tauri -- build --target aarch64-apple-darwin --bundles app` after the `2.0.0-pre.22` version bump; it passed and produced `/Users/chenm0m/LocalRepo/VibeHub/target/aarch64-apple-darwin/release/bundle/macos/VibeHub.app`.
- `hard_observed`: Browser mock verification passed: starting list had 2 project cards, clicking `WorldQuantGenius` entered detail with 0 project cards and one Sidebar brand button, then clicking the brand returned to list with 2 project cards.
- `hard_observed`: Browser visual check confirmed the Sidebar brand image loaded (`naturalWidth=2048`) and rendered in the adjusted brand button; screenshot saved to `/private/tmp/vibehub-sidebar-brand-after.png`.

## Context Still Needed
- `agent_reported`: User or a permitted GUI session should confirm actual macOS drag and double-click maximize behavior in the running `.app`.
- `agent_reported`: Windows/Linux manual smoke test remains useful before release, though no platform-specific window config was changed for those systems.

## Warnings
- `agent_reported`: Tauri build emitted existing warnings about unused gateway/local usage helper functions and duplicate bin target metadata; they are not introduced by this change.
- `agent_reported`: Browser/Playwright cannot fully validate native titlebar drag behavior because that behavior is provided by Tauri's injected desktop window script.
- `hard_observed`: `vibehub sync` returned `needs_attention` with `ownership_unavailable`; it asked whether all 17 changed files belong to the current VibeHub task. No user answer was available during this run, so this remains an unresolved ownership risk.
- `hard_observed`: Local browser verification used a temporary mock Tauri environment because plain Vite lacks the real Tauri backend. It validates the React state path and Sidebar rendering, not native desktop shell behavior.

## Next Session Should
- `agent_reported`: Open the packaged macOS app, verify top blank regions drag the window, double-clicking the blank title area toggles window maximize/restore, search/buttons do not start dragging, and red/yellow/green buttons still work.
- `agent_reported`: If the top spacer feels too tall or too short in the real app, tune only the macOS `Sidebar` spacer height and `trafficLightPosition`.

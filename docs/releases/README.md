# GitHub Release 说明

每个发布 tag 都要有一份用户能直接阅读的说明，而不是只生成 compare 链接。

## 每个 tag 必交

1. `docs/releases/<tag>.md`（含 `v` 前缀，例如 `v3.3.3.md`）
2. `CHANGELOG.md` 对应版本的条目（给仓库读者；GitHub Release 正文以本目录的 markdown 为准）

界面截图（`assets/releases/<tag>/*.png`）为可选项：提供时有助于展示新功能或可见修复，但不是发版前置条件。

## 说明文案

- 先写用户能感知到的变化，再列细节。
- 截图使用相对仓库路径，例如：

  `![检测模型](../../assets/releases/v3.3.3/agent-import-models.png)`

- `scripts/release/notes.mjs` 会在创建 GitHub Release 时把这些路径改成指向该 tag 的 raw URL。
- 不要只贴 `Full Changelog` 链接；compare 链接可以放在文末。

## 截图

- 统一 **1600×1000 PNG**，与 README 视觉导览一致。
- 使用脱敏演示数据：不要出现真实家目录、API Key、私人项目名或真实邮箱。
- 必须来自当前源码的真实界面，不要生成假 UI。优先顺序：
  1. `scripts/release/capture-macos.sh`：demo HOME + `npm run tauri dev`，需要辅助功能和屏幕录制权限。
  2. 权限不可用时：`npm run dev` 打开 `http://localhost:1420/?fixture=release`，用同一套组件和 `src/lib/releaseFixture.ts` 脱敏数据截图，再 `sips -z 1000 1600`。生产构建不会带 `?fixture=release`。
- 新版本主界面图同时复制到 `assets/readme/`，保持视觉导览不过期。

## 流水线

Release workflow 用 `--notes-file` 创建或更新 draft，不再使用 `--generate-notes`。  
`RELEASE_TAG=vX.Y.Z npm run release:check` 会确认说明文件存在；若说明中引用了 `assets/releases/` 截图，则校验其为 1600×1000 的 PNG，但不要求必须提供截图。

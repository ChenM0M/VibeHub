# V3 Agent 与发布验收流程

本文件是 VibeHub V3 的项目级执行补充，供 Codex、Claude Code 和 OpenCode
在仓库内工作时遵守。它补充根目录 `AGENTS.md` / `CLAUDE.md` 中的 V3
Agent Specification；任务、计划、事件和 session 的事实来源仍然是 V3
控制面，不是本文件。

## 1. 每次 Agent 工作的最小流程

开始前必须：

1. 通过 V3 MCP 读取 current task、task lifecycle、plan 和 session 投影；MCP
   不可用时使用当前源码构建的 `target/debug/vibehub` CLI JSON fallback。
2. 明确目标 plan node；先将 node 置为 `active`，再执行 `session_open`，记录
   task、node、Agent 和真实 working directory。
3. 不直接编辑 `.vibehub` 的事件日志、投影或 current pointer；所有状态改变使用
   typed command 或 V3 MCP tool。
4. 每个可核验里程碑写 `progress`；发现阻塞、外部依赖、证据缺口或范围漂移，
   立即写 `risk`。
5. 结束时写真实的 `agent_result`，然后 `session_close`。中断后必须新开
   recovery session 并记录 gap/recover，不得复用“看起来仍在运行”的旧 session。

## 2. 版本与发布前门禁

版本修改后、本地准备 tag 前、以及 GitHub Actions 发布前都必须运行：

```bash
npm run release:check
```

如果要校验某个发布 tag：

```bash
RELEASE_TAG=v3.0.0 npm run release:check
```

该检查确保以下版本全部一致：

- `package.json`、`package-lock.json` 及 lockfile 根 package；
- `crates/vibehub-core/Cargo.toml`；
- `crates/vibehub-cli/Cargo.toml`；
- `src-tauri/Cargo.toml`；
- `src-tauri/tauri.conf.json`。

设置 `RELEASE_TAG` 时，tag 去掉可选的 `v` 前缀后必须等于项目版本。它只做
版本/标签一致性检查，不代替编译、测试、签名或原生验收。

## 3. 本地发布候选门槛

在声称“可以发版”前，至少要有以下可核验证据：

```bash
npm run release:check
npm run build
npm run v3:contracts:check
cargo fmt --all -- --check
cargo test --locked --workspace
cargo build --locked -p vibehub-cli --bin vibehub
npm run v3:mcp:check
git diff --check
```

macOS 还要对当前版本 bundle 做一次构建和 strict codesign 检查。任何旧版本
artifact/hash 都不能冒充当前版本证据。

## 4. GitHub Actions 发布边界

- `Build and Test` 在 feature、main/dev push 或 PR 上运行前端、contracts、Rust、
  MCP 和 Tauri 构建门槛。
- `Release` 先 checkout 精确 tag，运行 preflight，再创建一个 draft Release，
  最后上传 macOS、Windows、Linux 产物和平台 SHA-256 清单。
- 稳定 tag 缺少 Apple Developer ID/notarization Secrets 或 Windows
  Authenticode PFX Secrets 时必须失败；不得把 ad-hoc/unsigned 产物称为正式签名。
- 带 `-` 的预览 tag 可以生成明确标注的非正式测试产物，但仍要保留 exact tag、
  artifact 路径和 SHA-256。
- Release workflow 的成功只证明构建、包装和签名边界；它不证明 Windows 原生
  UI、IDE、文件锁、reparse point、升级/回滚或卸载行为。

当前稳定版所需仓库 Secrets：

- Apple：`APPLE_CERTIFICATE`、`APPLE_CERTIFICATE_PASSWORD`、
  `APPLE_SIGNING_IDENTITY`、`APPLE_ID`、`APPLE_PASSWORD`、`APPLE_TEAM_ID`；
- Windows：`WINDOWS_CERTIFICATE`、`WINDOWS_CERTIFICATE_PASSWORD`；
- 可选 Homebrew：`HOMEBREW_TAP_TOKEN`。

## 5. Windows 发布后手测交接

Windows 原生验收必须使用 draft/published Release 中的精确 Windows artifact，
并把版本、源 commit、安装包路径、`SHA256SUMS-windows-x64.txt`、签名状态、
Windows 版本和 IDE 版本写入证据。不得从 macOS evidence、CI build success 或
本地重编译推断 Windows 通过。

测试范围见
[`windows-w1-w2-input-package.md`](evidence/m9-macos-dry-run/windows-w1-w2-input-package.md)：
`A07`、`E07`、`F07–F10` 和最终双平台链 `G05`。所有 fixture 必须是 disposable
副本；不得迁移、删除或锁定真实 VibeHub 仓库、用户项目或 `legacy-v2` archive。

## 6. 结束条件

只有在必需 criterion 都有 evidence、findings 已关闭、阻塞已解决或有准确外部
阻塞记录，并且用户通过受信渠道确认后，才可以提出或确认任务完成。若 Windows
尚未手测，必须保持对应 ID 为 `BLOCKED`，并明确写“等待发布后 Windows 主机验收”。

# v3.3.11 发布与 Windows 原生验收交接

- 日期：2026-09-07
- 版本：3.3.11（tag `v3.3.11`）
- 源 commit：`d61192e`（tag 指向）；携带修复提交 `fd23e2d`（Claude Profile 隔离与 native auto 语义）、`5863353`（OpenCode provider 上游协议持久化）、`f8240ce`（版本提升）
- Release URL：https://github.com/ChenM0M/VibeHub/releases/tag/v3.3.11（published 2026-09-07T09:32:32Z）
- Release workflow：run `34104992355`，preflight / create-draft / 4 个平台 build-and-upload / publish-release / update-homebrew-cask 全部 success

## 待 Windows 主机验收（保持 BLOCKED）

以下 ID 必须使用本 Release 的精确 Windows artifact 与真实 SHA-256 在
Windows 10/11 主机或可信 VM 手测；macOS/CI 证据不得替代：

- `A07`、`E07`、`F07–F10`（见
  `docs/v3/evidence/m9-macos-dry-run/windows-w1-w2-input-package.md`）
- 最终双平台链 `G05`

涉及任务：`task.windows-opencode.f3a08096abf5`、
`task.windows-opencode-launcher.a92f9d632909`、
`task.claude-agent-profile-auto.fdad6ac0200e`（C06/N04）。

## Windows artifact 与 SHA-256（来自 `SHA256SUMS-windows-x64.txt`）

```
79ba0cb78bffe4e29b84adeed0ddf500bb4d062ea379c08487773ef7a2e6b8ec  VibeHub_3.3.11_x64-setup.exe
476b8485cd625f082bf784df779d1acc0cd2be33a5982e9e87007dc18f044acf  VibeHub_3.3.11_x64_en-US.msi
add6d8fc22c9f3782d5ee58bc3e0445332ee8661aedc175a9ca159b2d53e6178  VibeHub_3.3.11_portable_x64.exe
```

- 签名状态：本仓库未配置 Windows Authenticode 凭据（`WINDOWS_CERTIFICATE`
  缺失），产物为 CI 无凭据 fallback 的 unsigned 构建，不得称为正式签名。
- 手测时必须先核对上述 SHA-256 与 `SHA256SUMS-windows-x64.txt` 完全一致，
  并把 Windows 版本、IDE 版本与测试结果写入证据。
- 所有 fixture 使用 disposable 副本；不得迁移、删除或锁定真实 VibeHub
  仓库、用户项目或 `legacy-v2` archive。

## macOS 本地验证（已记录，证据先行后清理）

- `npm run tauri build` 产出 `VibeHub.app` 与
  `VibeHub_3.3.11_aarch64.dmg`（sha256
  `2bf7830eee99a72a66067f6ab862f860dab39f018a3908119bb6bf7b4744d062`）。
- 按 CI 无凭据分支 `codesign --force --deep --sign - --timestamp=none`
  签名后 `codesign --verify --deep --strict` 通过（valid on disk /
  satisfies its Designated Requirement）。
- 验证完成后，`target/release/`、`dist/`、
  `target/debug/{deps,incremental,build,examples}`（共 9.0G）已按根
  `AGENTS.md` 构建产物清理规则移入
  `~/.Trash/vibehub-build-cleanup-20260907T171428/`（可恢复）；
  `target/debug/vibehub` 运行时保留。

## 交接清单

1. 在 Windows 主机下载 `VibeHub_3.3.11_x64-setup.exe`（或 portable），核对 SHA-256。
2. 按 W1–W2 清单执行 `A07`、`E07`、`F07–F10`，完成后执行 `G05` 双平台链。
3. 将结果（通过/失败）写入对应任务的 criterion_review 与 evidence；未完成前对应 criterion 保持 `blocked`。

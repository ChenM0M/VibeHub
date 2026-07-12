# M2 Review - V3 事件核心与 MCP 控制面

Task: T-20260711080143-b1ff21ea
Run: R-20260711080143-b042365e
Phase: review (active)

## Diff Summary

- `hard_observed`: M2 新增 V3 JSON Schema write contracts、Rust event core/application/projection/views、共享 MCP/CLI adapter、Tauri production loader、host compatibility evidence 与 RFC 状态更新；VibeHub 生成的全工作区 evidence 同时混入 M0/M1 和其他任务变更，不能整体归因于 M2。

## Verdict

- `user_confirmed`: `accepted_with_residual_risks`，`gate_pass: true`。用户确认 M2 可以结束并归档；本机实现与自动化验证通过，三宿主真实 recovery tools/call 和 Windows runtime 转为后续发布验证项。
- `user_confirmed`: 当前审查范围内未发现技术实现、功能特性、真实数据视图、前后端接入或 MCP/CLI 共用链路的具体问题；保留的外部宿主/平台缺口不表示已发现实现缺陷。

## Evidence Grades

- `hard_observed`: Git diff、任务元数据、Rust/Node 命令退出状态、测试计数、schema assertions、MCP protocol/resources/tools 结果。
- `agent_reported`: 先前 packaged app 与 host CLI 探测记录；本轮未独立复跑全部历史探测。
- `inferred`: 未验证平台可能存在 stdio、路径、锁、崩溃清理或 packaging 回归。
- `user_confirmed`: 用户明确要求进入审查阶段，但未豁免 acceptance criteria。

## Completed

- `hard_observed`: 已审查 M2 diff、Implement context/handoff、任务 acceptance criteria、RFC-001/002、核心与 MCP 测试证据。
- `hard_observed`: 本机实现验证无失败：Rust workspace 234 tests passed，V3 contracts 366 assertions passed，TypeScript/Vite production build passed，MCP 2025-11-25 的 7 resources/3 tools 合约测试 passed。
- `hard_observed`: 事件 append/rebuild/idempotency/optimistic conflict 与五类 M0/M1 view schema compatibility 有自动化测试证据。
- `hard_observed`: production repository 到 Tauri `v3_load_view_bundle`、M1 cockpit/store 的前后端真实数据接入已编译并通过现有契约/构建验证；未发现接线断裂或 schema 不兼容。
- `hard_observed`: MCP、CLI、Tauri 共用 Application Service/View Repository 的实现边界符合计划，现有审查未发现功能语义分叉。
- `user_confirmed`: Review gate verdict 为 `gate_pass: true with accepted residual risks`；用户接受两项外部验证缺口不阻止 M2 结束归档。

## Not Yet Done

- `hard_observed`: Codex、OpenCode、Claude Code 尚未全部完成真实 MCP recovery tools/call 最小闭环；OpenCode 未安装，Claude isolated HOME 未登录。
- `hard_observed`: Windows stdio、路径、锁、崩溃清理、fsync/rename、native packaging 与协议预算尚无 Windows runner 证据。
- `hard_observed`: 官方 MCP Inspector 未获执行新下载 npm 代码的显式授权，未纳入证据。

## Key Decisions Made

- `hard_observed`: 本机功能与契约实现可进入正式审查，但不能据此宣称 M2 完整验收。
- `user_confirmed`: 将“现有证据内技术实现、功能特性及前后端接入未发现问题”记录为审查结论；与“Windows 和三宿主矩阵尚未验证”并列保留。
- `user_confirmed`: 三宿主与 Windows 两项 acceptance gaps 作为后续发布验证风险保留，不再阻止 M2 Review 通过与归档。
- `user_confirmed`: 用户要求结束 Implement 并进入 Review；未要求忽略或豁免既有 acceptance criteria。

## Files Changed

- `hard_observed`: Review 仅新增/刷新 VibeHub review context、diff evidence、review report、events、agent views 与本输出。
- `hard_observed`: 本轮未修改 M2 业务实现文件。

## Files Reportedly Read

- `hard_observed`: `.vibehub/agent-view/{current.md,current-context.md,handoff.md,sync.md}`、hard rules、adapter protocol、Implement/Review context packs、task metadata、Implement output、generated review report。
- `hard_observed`: `package.json` scripts、M2 Rust/TypeScript test locations、scoped Git diff/stat、RFC-001/002 与 host compatibility evidence。

## Commands Run

- `hard_observed`: `vibehub status`、`vibehub sync`、`vibehub validate`、`vibehub output-lint`、`vibehub handoff`、`vibehub finish --confirmed-by-user`、`vibehub advance --confirmed-by-user`、`vibehub review`、`vibehub claim review`。
- `hard_observed`: `cargo fmt --all -- --check`、`cargo test --workspace`、`npm run v3:contracts:check`、`npm run build`、`npm run v3:mcp:check`。

## Tests Run

- `hard_observed`: `cargo test --workspace` passed：desktop 17、MCP adapter 2、core 215，0 failed。
- `hard_observed`: `npm run v3:contracts:check` passed：366 assertions、12 scenarios、5 views、2 write contracts。
- `hard_observed`: `npm run build` passed；仅有 stale browser data 与 >500 kB chunk warnings。
- `hard_observed`: `npm run v3:mcp:check` passed：protocol 2025-11-25、7 resources、3 tools、12 requests、173 ms。
- `hard_observed`: Windows runner 与三宿主真实 recovery tools/call acceptance matrix 尚未执行。

## Context Still Needed

- `agent_reported`: Windows runner/toolchain、已登录 Claude/Codex host、OpenCode binary。
- `agent_reported`: 若要求 Inspector 证据，需要用户显式授权执行第三方 npm package。

## Warnings

- `hard_observed`: 工作区有 172 个左右变更文件，包含 M0/M1、其他 VibeHub 任务与生成状态；generated review diff 的 223 文件不能全部归因于 M2。
- `hard_observed`: Cargo dual-bin、Tauri dead-code、Vite stale data/chunk size warnings 未造成测试失败。
- `inferred`: 未完成的 host/platform matrix 可能隐藏 stdio、path、locking 或 packaging 的平台特定回归。

## Next Session Should

1. `agent_reported`: 在 Windows runner 执行 workspace/contracts/MCP tests、kill injection、long/UNC paths、stdio shutdown 与 native packaging。
2. `agent_reported`: 在 Codex、Claude Code、OpenCode 中用同一 packaged binary 完成 resources/read 与 recovery tools/call 闭环。
3. `user_confirmed`: M2 结束归档后切换到 M3；后续发布硬化时继续补齐 Windows 与三宿主矩阵。

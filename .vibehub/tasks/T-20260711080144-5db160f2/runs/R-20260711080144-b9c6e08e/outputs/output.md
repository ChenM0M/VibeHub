# M6 Align - 保留 M1 V3 主体验，完成 legacy-v2 只读归档与发布硬化

Task: T-20260711080144-5db160f2
Run: R-20260711080144-b9c6e08e
Phase: align (active)

## Intent

- `user_confirmed`: M1 当前 V3 前端、功能和交互逻辑是最终产品基线，后续里程碑不得改回 V2 体验。
- `agent_reported`: M6 完成 V2 -> V3 clean break，把旧数据作为独立 legacy-v2 只读归档接入 M1 已有“归档任务”交互，删除旧写路径与重复 adapters，并以 Windows/macOS 安装、升级、MCP、IDE、并行任务、卸载、回滚全流程证明 V3 可发布。

## Scope

- `agent_reported`: 实现 v2 detection、preflight、显式用户确认、原子归档/失败恢复：旧 `.vibehub/` 整体进入 `.vibehub/legacy-v2/`，新 V3 state 从零初始化，不做 domain 数据转换。
- `agent_reported`: 实现独立 read-only legacy adapter，只投影 task list/title/time/phase/final summary/file links；禁止写、禁止复用 V3 domain、禁止把 V2 YAML shape 传入 V3 repository/components。
- `agent_reported`: 将 legacy task list/detail 接到 M1 左侧底部现有“归档任务”展开与详情交互；V3 项目/active tasks/四 Tab 始终是主体验。
- `agent_reported`: 删除 V2 canonical write commands、旧 capability/gate/phase 仪式、重复 adapters/skills、UI arbitrary writes、fixture production assets 和不再引用的旧 UI 路径；保留经 M2-M5 证明需要的 launcher/gateway/scanner/doctor assets。
- `agent_reported`: `doctor` 检测 V2、partial migration、legacy permissions、adapter drift、MCP packaging、orphan processes/worktrees，并提供非破坏性 repair guidance。
- `agent_reported`: 完成 Windows/macOS clean install、V2 upgrade、V3 upgrade、MCP stdio/config、IDE reveal/open、M5 parallel node、application exit/restart、uninstall cleanup、user data retention 与 rollback。
- `agent_reported`: 建立 prerelease/release packaging、签名/公证选择、发布清单、rollback/runbook、artifact/SBOM/checksum 和可复现本地/CI 验证。

## Success Criteria

- `agent_reported`: 以下 M6-C01 至 M6-C10 共同定义迁移真实性、产品守恒和可发布性。

1. `M6-C01 V3 主体验守恒`: normal production 默认进入 M1 已确认的 V3 cockpit；项目选择、四 Tab、task selection、timeline/plan/structure/brief 与 M2-M5 新能力保持一致，无 V2 页面或旧八视图回流。
2. `M6-C02 Clean break 可恢复`: 对真实 V2 corpus 执行 preflight -> digest/backup plan -> 用户确认 -> atomic archive -> V3 init；中断/权限/磁盘不足/目标已存在不会留下半迁移，重试幂等，rollback runbook 能恢复原 V2 目录。
3. `M6-C03 Legacy 绝对只读`: legacy adapter 仅暴露最小字段和 file links；写尝试、path escape、symlink escape、损坏 YAML/JSON、缺失文件均结构化拒绝/降级。V2 schema/types 不进入 V3 core、events、Application Service 或 production repository。
4. `M6-C04 归档交互一致`: legacy 历史使用 M1 现有左侧“归档任务”展开和详情交互；可查看标题、时间、阶段、摘要、相关文件并返回 active task，不新增与 V3 竞争的主导航或复杂迁移 UI。
5. `M6-C05 旧写路径清零`: source/dependency/binary scans 证明 V2 canonical commands、重复 adapter generation、UI arbitrary writes、旧 skills/commands 和 production fixtures 已移除；保留项有明确 owner、调用图和非 V2 理由。
6. `M6-C06 macOS 全流程`: 从 clean install/V2 upgrade 到项目初始化、三 host 至少目标范围内的 MCP、session recovery、IDE open、并行 node、restart、upgrade、uninstall/cleanup 全流程通过；prerelease 先本地 ad-hoc 验证，正式 Developer ID/notarization 仅在用户决定需要时启用。
7. `M6-C07 Windows 全流程`: 覆盖 installer/upgrade/uninstall、stdio config/argument arrays、drive/UNC/long path/case、file lock/antivirus、IDE、orphan process/worktree、restart 和 cleanup；无仅 macOS 可用的隐含路径或 shell 假设。
8. `M6-C08 数据与卸载安全`: uninstall 默认不删除项目仓库、V3 state、legacy-v2 archive 或 dirty worktrees；删除用户数据必须独立明确确认。日志/diagnostic bundle 进行路径、token、secret redaction。
9. `M6-C09 发布与回滚可执行`: 本地与 CI 使用同一直接构建命令和冻结 toolchain；artifacts/checksums/version/SBOM/release notes/known issues 可追溯，失败 release 可回滚 binary/config 而不回滚或破坏用户项目数据。
10. `M6-C10 最终回归`: M1 12 scenarios、M2 core/MCP recovery、M3 Project Intelligence、M4 task truth/stability、M5 parallel/conflict/recovery 的 gold suites 在 production packaging 下通过；1440x900、1024x600 和 Windows/macOS 路径视觉/交互无回归。

## Acceptance Criteria

- `agent_reported`: M6-C01 至 M6-C10 全部 required；V2 写入残留、legacy schema 泄漏、半迁移、默认删除用户数据、仅单平台通过或破坏 M1 产品体验均为 release blocker。
- `agent_reported`: clean break 必须用真实 V2 corpus、corrupt/partial cases 和 kill injection 验证；“旧历史能打开一次”不足以证明迁移安全。
- `user_confirmed`: 任何真实项目归档迁移、正式签名/公证启用、发布或会删除数据的 cleanup 均需用户明确确认。

## Non-Goals

- `user_confirmed`: 不迁移 V2 domain state 到 V3，不保留旧 UI/命令兼容层，不改变 M1 V3 主体验。
- `agent_reported`: 不为 legacy 增加编辑、恢复执行、重新开启 session、PlanGraph 重建或 V3 search/index；legacy 只供最小历史阅读。
- `agent_reported`: 不默认删除用户仓库数据、legacy archives、dirty worktrees 或自定义 host config。
- `agent_reported`: 不在未本地复现的情况下用 CI 穷举调试 macOS packaging/signing。

## Autonomy Level

- `agent_reported`: `guided_drive / critical release risk`。只读审计、实现和非破坏性测试可自主推进；真实迁移、删除旧数据/commands 的不可逆步骤、签名/公证选择、发布/回滚和卸载数据清理必须用户确认。

## Key Decisions Made

- `user_confirmed`: M1 V3 cockpit 是最终主产品，不因 legacy 或发布工作回退交互。
- `agent_reported`: clean break 不做数据转换；legacy-v2 是独立目录与独立只读 adapter，不是 V3 aggregate。
- `agent_reported`: M1 已有“归档任务”区域是 legacy 最小入口，避免另建主导航和维护第二套产品。
- `agent_reported`: prerelease 使用 ad-hoc signing 并先在本地 macOS 验证；正式 Developer ID/notarization 是用户级发布选择。

## Completed

- `hard_observed`: 对照 M1 产品基线、M6 task metadata、主计划 migration/deletion/legacy/release sections、hard-rules CI discipline 与 M2-M5 refined Align，完成 M6 任务范围和最终验收重写。

## Not Yet Done

- `agent_reported`: M6 Research/Plan/Implement/Review、真实 migration、双平台 installers、signing/release 均未执行。

## Files Changed

- `hard_observed`: `.vibehub/tasks/T-20260711080144-5db160f2/runs/R-20260711080144-b9c6e08e/outputs/output.md`。
- `hard_observed`: `vibehub switch/sync` 管理的 agent-view、context pack、events、index 与 projection 文件。

## Files Reportedly Read

- `hard_observed`: VibeHub current/current-context/handoff/hard-rules/protocol 与当前 Align context pack。
- `hard_observed`: M1 Review/current `V3Cockpit` 归档交互、M2-M5 refined Align outputs。
- `hard_observed`: `docs/vibehub-v3-redesign-plan.md` M6/迁移/删除/legacy/release sections、research pack 与平台硬规则。

## Commands Run

- `hard_observed`: `vibehub switch . T-20260711080144-5db160f2`, `vibehub sync .`。
- `hard_observed`: `rg`, `sed`, `find` 等任务、源码和文档读取命令。

## Tests Run

- `not_tested`: Align 只完善任务定义，未执行 migration、installer、native smoke、signing 或 release tests。
- `hard_observed`: M1 baseline 与 M2-M5 refined gold gates 已被纳入 M6 最终回归要求，但尚未在 production package 下执行。

## Context Still Needed

- `agent_reported`: Research/Plan 需盘点完整 V2 write/delete/adapters 调用图、真实 legacy corpus、installer technology、Windows host、release channels、data retention policy 和用户对正式签名/公证的选择。

## Warnings

- `hard_observed`: 当前工作区有大量未提交 M0/M1/VibeHub 变更，不是 M6 migration baseline；真实迁移和删除清单必须在 M5 验收后从可追溯 commit 开始。
- `inferred`: 当前归档任务数据仍是 M1 mock；M6 应替换其 data source 为独立 legacy adapter，而不是改变已认可的展开/详情交互。

## Next Session Should

1. `agent_reported`: validate/output-lint 本 Align output；未经用户确认不 finish/advance。
2. `agent_reported`: Research 先完成 V2 write/delete inventory、legacy threat model、migration failure matrix、platform packaging matrix 和 release/signing decision record。
3. `agent_reported`: Plan 按 read-only legacy slice -> migration preflight/atomicity -> old-path removal -> macOS local package -> Windows package -> install/upgrade/uninstall -> full gold regression -> release/rollback 推进。

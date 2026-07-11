# Plan 阶段输出

## Completed

- `hard_observed` 已产出五份正式 RFC backlog，分别覆盖 domain/event、MCP control plane、Project Intelligence、Task/PlanNode/acceptance、worktree orchestration。
- `hard_observed` 已产出 `docs/v3/m0-task-pack.md`，包含 scope/non-scope、五类 view contract 最小字段、12 组 fixture、planned file surface、7 个 work packages、12 条 Criterion、验证计划、stop conditions 与 M1 handoff。
- `hard_observed` 已将 M0-M6 七个独立 VibeHub Task ID、依赖和正式工件入口回写到 V3 总计划，并将状态从 draft 更新为 aligned。
- `user_confirmed` 计划顺序保持为 M0 contract/fixtures -> M1 fixture-only 高保真前端 -> M2 真实 core/MCP。
- `user_confirmed` RFC-004/005 固定 M4 stability gate；只有全部证据通过且项目所有者明确批准后，M5 才由 V3 self-host。

## Not Yet Done

- `agent_reported` 尚未进入或实现 M0；schemas、fixtures、validator 与 consumer proof 均是 M0 Task 的工作。
- `agent_reported` 尚未对 M0-M6 各自的 align 阶段执行 finish/advance；这些 Task 保持独立 active 状态等待按顺序处理。
- `agent_reported` 尚未决定 JSON Schema codegen、cross-process lock、optional MCP host capabilities、M3 analyzer 优先级、Windows worktree root 与 M4 soak 数值；均已路由到相应 RFC/spike。

## Key Decisions Made

- `inferred` M0 的 canonical boundary 是五份 JSON Schema 2020-12 read models 和 deterministic fixtures，不是 V2 storage/event/Tauri response。
- `inferred` M1 只能经 fixture repository 消费契约；不得读 V2 YAML、调用 V2 workflow commands 或启动真实 MCP。
- `inferred` M0 生产文件范围限制为 `contracts/v3`、`fixtures/v3`、contract scripts/types/tests 与 RFC/Task Pack 决策更新；不得改 V2 production workflow modules。
- `inferred` M0 native fixture 可验证 parsing/display/identity，但不得把未执行的 Windows/macOS file operations 报成通过。
- `inferred` M4 gate 包括 deterministic rebuild、idempotent retry、两轮 remediation history、kill-resume、三 host MCP loop、双平台 native smoke、零未解决 P0/P1、预声明 soak 和 owner approval。

## Files Changed

- `hard_observed` `docs/v3/rfc-backlog/001-domain-event-contract.md`
- `hard_observed` `docs/v3/rfc-backlog/002-mcp-control-plane-contract.md`
- `hard_observed` `docs/v3/rfc-backlog/003-project-intelligence-lifecycle.md`
- `hard_observed` `docs/v3/rfc-backlog/004-task-node-acceptance-lifecycle.md`
- `hard_observed` `docs/v3/rfc-backlog/005-worktree-orchestration.md`
- `hard_observed` `docs/v3/m0-task-pack.md`
- `hard_observed` `docs/vibehub-v3-redesign-plan.md`
- `hard_observed` `.vibehub/research/current/research-pack.md`
- `hard_observed` `.vibehub/research/current/source-log.yaml`
- `hard_observed` `.vibehub/research/current/findings.yaml`
- `hard_observed` `.vibehub/tasks/T-20260711062223-6f07315c/runs/R-20260711062223-10a76733/outputs/output.md`
- `hard_observed` VibeHub CLI 生成/更新的 M0-M6 task/run/context、agent-view、state、index、handoff、sync 与 event 文件。

## Files Reportedly Read

- `hard_observed` VibeHub current/current-context/handoff/hard-rules/protocol/workflow 与 align/research/plan context packs。
- `hard_observed` V3 总计划、Research Pack、M0-M6 task metadata。
- `hard_observed` events/projection/research/project_structure、Tauri commands/config/services、ProjectStructure/Cockpit/ProjectCenter UI、Cargo/package/CI 文件。
- `hard_observed` Research Pack source log 登记的 JSON Schema、MCP/Rust SDK、Codex/Claude/OpenCode、Windows path、Git worktree、event sourcing 官方资料。

## Commands Run

- `hard_observed` `vibehub sync/status/validate/output-lint/handoff/finish/advance/switch/start-intake`。
- `hard_observed` `sed`、`rg`、`find`、`wc`、`du`、`git status/log/diff/diff --check`。
- `hard_observed` `curl` 官方资料；`ruby` 安全解析 Research Pack YAML。

## Tests Run

- `hard_observed` research phase 的 `vibehub validate` completed、`output-lint` passed。
- `hard_observed` Research Pack `source-log.yaml` 与 `findings.yaml` 已通过 Ruby `YAML.safe_load`。
- `hard_observed` `git diff --check` 对 Research Pack、V3 总计划、五份 RFC backlog 和 M0 Task Pack无错误。
- `agent_reported` 未运行 Rust/TypeScript 测试：本轮只修改 Markdown/YAML 规划资产和 VibeHub workflow metadata，未修改可执行代码。

## Context Still Needed

- `inferred` M0 开始时需要显式 switch 到 `T-20260711080143-c3669d9d`，读取其 align context，并由项目所有者在 contract `1.0` freeze 前完成 fixture walkthrough。
- `inferred` Windows native lab、三 MCP host pinned versions 与两个 M3 sample repos 在对应 milestone 前仍需准备。

## Warnings

- `hard_observed` 共有八个 active tasks；所有验证必须 status/switch 或使用 task-scoped command，不能依赖 current pointer 记忆。
- `hard_observed` start-intake 每批最多三个 high-confidence drafts，七个 Task 分三批创建；task metadata 中 dependency 使用 M0-M5 milestone labels，完整 Task ID 映射已写入总计划。
- `hard_observed` CLI 将 M0-M6 创建为 guided_drive；是否需要为 M0/M2/M5 另行采用 evidence discipline 需在各自 align 中通过 CLI 支持的正规流程处理，禁止手改 task metadata。
- `hard_observed` 工作区已有旧任务 output 未提交变更，本轮未覆盖或归入 V3。

## Next Session Should

- `agent_reported` 运行 plan phase `vibehub validate`、`output-lint` 和 `handoff`。
- `agent_reported` 等项目所有者确认是否结束/推进父计划 Task；不要擅自进入 implement。
- `agent_reported` 获得开始 M0 的明确指令后，switch 到 M0 Task，先完成其 align，再按 `docs/v3/m0-task-pack.md` 实施 contracts/fixtures。

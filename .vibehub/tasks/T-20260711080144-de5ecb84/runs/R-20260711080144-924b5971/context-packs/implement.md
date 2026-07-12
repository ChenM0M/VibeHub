# Context Pack: Implement

Task: T-20260711080144-de5ecb84
Run: R-20260711080144-924b5971
Phase: Implement
Generated at: 2026-07-12T06:33:13Z
Source commit: b24a9e9

## Instructions

Use this context only for the current phase.
Do not mark state.yaml completed.
Report files read, commands run, decisions made, and unresolved risks.

## Capability Output Schema

```json
{
  "required_fields": [
    "diff_summary",
    "changed_files",
    "commands_run"
  ],
  "optional_fields": [
    "rollback_plan",
    "references"
  ],
  "produces": [
    "diff"
  ],
  "consumes": [
    "implementation_plan"
  ],
  "parallel_safe": false,
  "custom": false
}
```

## Prior Outputs Summary

```json
[
  {
    "capability": "implement",
    "completed": [
      "`user_confirmed`: 用户明确要求 M5 进入计划阶段；canonical status 已确认是 M5 `plan / active`，`plan_ready` gate 已通过，无需重复执行 `advance`。",
      "`hard_observed`: 对照 RFC-004 M4 stability gate、RFC-005 D1-D7、V3 主计划 M5、当前 V3 event/session lifecycle、MCP/Tauri bridge、production view bundle、contracts/fixtures 与 launcher cwd 边界，完成正式 implementation plan。",
      "`hard_observed`: `target/m4-stability-report.json` 与 `docs/v3/m4-stability-gate.md` 表明自动化和 macOS local gate 已通过，但 Windows native smoke 未运行、owner shadow approval 未记录，因此 `m5_entry_gate` 仍关闭。",
      "`hard_observed`: 当前 V3 已有 append-only event store、跨进程 append lock、乐观版本、幂等键、Task/PlanNode/Session lifecycle、deterministic rebuild 与五视图 production bundle；尚无 Worktree aggregate、长期 lease、scope overlap engine、Git integration queue 或 recovery/cleanup policy。",
      "`hard_observed`: production PlanGraph 的 agent 分布仍由节点 sequence 派生 mock；M5 必须用真实 session/worktree projection 替换，同时保留现有 toggle、节点选择、timeline 和 brief 交互。",
      "### Frozen Decisions (D1-D7)",
      "1. `agent_reported` **D1 eligibility/scope**：launch 前对 declared scope、dependency readiness、`parallel_safe`、observed delta、case-fold collision、shared/generated/migration denylist 做纯函数评估，输出 `allow | warn | block`、稳定 reason code 和 evidence refs。unknown scope 默认 `warn`；共享迁移、生成入口、canonical event store 路径和已观察到的同文件写入默认 `block`，只能由带证据的 owner override 解锁。",
      "2. `agent_reported` **D2 identity/path**：一个 active parallel PlanNode 默认绑定一个 `WorktreeId + LeaseId + branch + base_sha + native_path`；read-only session 必须显式声明且不获可写 lease。分支采用 `vibehub/<task-slug>/<node-slug>-<short-id>`，display name 与 native path 分离；默认 root 必须由 native path policy 解析，Windows root/path budget 在真机 spike 后冻结。",
      "3. `agent_reported` **D3 lease**：worktree lease 是长期 domain lease，不复用 event-store 短时 append lock。lease 记录 owner session、host/tool、acquired/heartbeat/expires、generation 和 reclaim challenge；过期仅变为 `stale`，不得仅凭 wall-clock 删除目录或夺取活进程资源。reclaim 必须经过 inspect + owner/process challenge + 新 generation 事件。",
      "4. `agent_reported` **D4 lifecycle**：状态机固定为 `planned -> creating -> ready -> active -> dirty -> submitted -> integrating -> integrated`，异常分支为 `conflicted | abandoned | repairing`，终态为 `cleaned`。所有转换先验证 expected version 和 Git observation，再 append domain event；失败 side effect 记录可重试 operation/evidence，禁止伪造成功事件。",
      "5. `agent_reported` **D5 integration**：v1 默认 `merge --no-ff`，允许 project policy 显式选择 `rebase` 或 `cherry-pick`；集成队列按 Plan DAG topology、ready time、stable node id 排序。base drift 在 preflight 阶段重新评估；冲突必须指派原 node/session owner 并提供 files/base/head/next action，VibeHub 不自动覆盖或解决内容冲突。",
      "6. `agent_reported` **D6 native/process**：所有 Git/launcher 调用使用参数数组与显式 cwd；解析稳定 porcelain，不解析本地化人类输出。missing/moved/pruned/locked/dirty worktree 分开建模；dirty worktree 永不自动 reset/remove。desktop exit 只停止新调度并记录 orphan candidates，后续由 recovery inspect 判定，不假定子进程已退出。",
      "7. `agent_reported` **D7 self-host**：M5 实现和隔离 spike 可在 entry gate 关闭时推进，但首条真实 V3-managed M5 shadow event 之前必须固定 M4 gate digest、Windows native evidence 和 `user_confirmed` owner approval。shadow 期间 V2 仅保存 audit pointer/rollback control，不双写 V3 domain、不成为第二 projection source。",
      "### Implementation Steps",
      "1. `agent_reported` **契约先行**：扩展 event/application command contract；为 worktree orchestration view 定义 identity、eligibility、lease、Git observation、integration、conflict、recovery、next action 和 evidence。扩展 PlanGraph/Timeline/NodeBrief 的引用字段，生成 TypeScript types，并用 FX-PARALLEL/FX-WIN-PATHS/FX-REWORK 增加确定性样本与 invalid sentinels。",
      "2. `agent_reported` **纯 domain 与 eligibility**：新增 Worktree/Lease/Integration IDs、enums、transition table、scope normalizer/collision evaluator、branch/path identity policy 和稳定错误码。此切片不执行 Git，不写真实 M5 self-host event，可在 gate 关闭时完整单测。",
      "3. `agent_reported` **Git adapter 与 operation journal**：以可注入 `GitRunner` 封装 `worktree list/add/repair/remove`、status/diff、merge/rebase/cherry-pick；命令只接收 argv，记录 cwd/stdout/stderr/exit/evidence。每个 side effect 采用 prepare -> execute -> inspect -> append result，支持幂等 retry 和 crash 后 reconciliation。",
      "4. `agent_reported` **lease/worktree lifecycle**：在单一项目根 V3 event store 中实现 acquire/heartbeat/release/reclaim，以及 create/activate/submit/integrate/conflict/repair/clean command handlers 和 deterministic projection。read-only exception、already-checked-out、detached/unborn、base drift、missing/moved/pruned、dirty refusal 都有显式结果。",
      "5. `agent_reported` **launcher 与 session ownership**：将 node/session/lease/worktree native cwd 传给既有 Tauri launcher；启动 Codex/OpenCode/Claude 前复核 lease generation 和 eligibility digest。session open/close/gap/recover 与 worktree lifecycle 通过 IDs 关联，事件始终写回主项目 V3 store，worktree 内不得生成 canonical 分叉。",
      "6. `agent_reported` **MCP/Tauri control plane**：MCP 新增版本化只读 orchestration resource 和最小 command tools；Tauri 新增 load/command bridge。两端复用 core application service、expected_version/idempotency/error envelope，UI 不直接写 `.vibehub` 或调用 Git。",
      "7. `agent_reported` **production UX 映射**：在既有 PlanGraph session toggle 中展示真实 tool/session/worktree owner；timeline 增加 lease/create/submit/integrate/conflict/repair 事件；NodeBrief/detail 展示 base、branch、native path、dirty/integration state、conflict owner 和 next action。移除 sequence-derived agent mock，不新增独立 worktree 管理首页。",
      "8. `agent_reported` **stability、native 与 self-host shadow**：先运行 contracts/core/Git adapter/process/product 回归，再做 macOS 与 Windows native harness。全部 M4 gate 和 owner approval 记录完成后，执行三个非重叠节点、至少两种 agent tool 的 shadow；命中 wrong-task、event divergence、workspace contamination、silent conflict/data loss、unrecoverable lease 任一 trigger 即停止 V3 写入并保留现场，由 V2 audit pointer 指向 rollback evidence。",
      "### Validation Plan",
      "`agent_reported` **contract**：`npm run v3:contracts:generate` 后 `npm run v3:contracts:check`；验证 schema 版本、deterministic fixture/hash、generated type drift、invalid sentinels、MCP tool/resource envelopes 和五视图兼容。",
      "`agent_reported` **core unit/property**：eligibility decision table、POSIX/Windows case/path collision、denylist、unknown scope、合法/非法 transition、lease generation/expiry/reclaim、idempotent retry、base drift、conflict owner、dirty cleanup refusal、event replay/rebuild 等价。",
      "`agent_reported` **Git integration**：临时 bare + working repo 创建三个 worktree，验证独立 HEAD/index/cwd；覆盖 already checked out、detached/unborn、missing/moved/pruned、intentional same-file conflict，以及 merge/rebase/cherry-pick policy。测试 teardown 仅删除自己创建且确认 clean 的临时目录。",
      "`agent_reported` **process/recovery**：agent kill、desktop exit、orphan child、并发 event-store writers、stale/dead lease、locked-file retry；活进程所有权不得仅由 timeout 推断。",
      "`agent_reported` **MCP/Tauri/product**：`npm run v3:mcp:check`、Rust bridge tests、`npm run build`；验证真实 session owner/base/branch/dirty/integration/next-action 可解释，PlanGraph mock 完全移除，timeline/brief 保持 M1-M4 交互回归。",
      "`agent_reported` **native gates**：macOS 本地执行 cwd/process/lock/cleanup smoke；Windows 真机覆盖默认 root、路径预算、drive/UNC/extended path、long-path 开关、占用删除与 process cleanup。FX-WIN-PATHS 只验证契约，不代替 native evidence。",
      "`agent_reported` **self-host acceptance**：固定 gate digest 后运行 3 worktrees / 3 nodes / >=2 tools；比较 V3 events、live projection、deterministic rebuild、Git refs/index/status 与 V2 audit pointer，验证无 workspace/event contamination。故意冲突与 crash/restart 必须保留 ownership 和恢复证据。",
      "### Affected Files",
      "`agent_reported` **contracts/generation**：`contracts/v3/{common,event-envelope,application-command,plan-graph-view,task-timeline-view,node-brief}.schema.json`；新增 `contracts/v3/worktree-orchestration-view.schema.json`；`scripts/v3-contracts/{fixture-data,generate-fixtures,generate-types,check}.mjs`；`src/v3/contracts/generated/*`、`src/v3/contracts/{index,fixtureRepository}.ts`、`fixtures/v3/{manifest.json,FX-PARALLEL,FX-WIN-PATHS,FX-REWORK,invalid}/*`。",
      "`agent_reported` **core**：新增 `crates/vibehub-core/src/v3/{worktree,orchestration,git_runner}.rs`；更新 `crates/vibehub-core/src/v3/{mod,domain,application,projection,lifecycle,views}.rs`。`event_store.rs` 只在需要公开 reconciliation 所需读取边界时最小扩展，不改变单一 canonical store 不变量。",
      "`agent_reported` **control plane/launcher**：`crates/vibehub-cli/src/mcp.rs`；`src-tauri/src/{commands,main,launcher}.rs`；`src/services/{v3ProductionViews,tauri}.ts`。具体 launcher command surface 在实现前先用 adapter test 固定，避免把 shell command string 带回 core。",
      "`agent_reported` **product**：`src/v3/stores/v3Store.ts`、`src/v3/app/V3Cockpit.tsx`、`src/v3/components/task/{PlanGraph,TaskTimeline,NodeBriefPanel}.tsx`，必要时新增同目录下 unframed worktree detail/next-action 子组件。",
      "`agent_reported` **validation/docs**：新增 `scripts/v3-worktree-orchestration/*` 与 package scripts；更新 `docs/v3/rfc-backlog/005-worktree-orchestration.md`；新增 M5 native matrix 和 self-host shadow/rollback runbook。`docs/v3/m4-stability-gate.md` 仅追加实际证据引用，不降低冻结阈值。"
    ],
    "full_ref": ".vibehub/tasks/T-20260711080144-de5ecb84/runs/R-20260711080144-924b5971/outputs/output.md",
    "key_decisions": [
      "`user_confirmed`: M5 进入计划阶段并继续当前 Plan 工作。",
      "`agent_reported`: worktree 是 PlanNode/session execution 的受控资源，不是独立功能按钮，也不建立第二套任务状态。",
      "`agent_reported`: 一个 active parallel PlanNode 默认一个可写 branch/worktree lease；read-only session 是显式例外。",
      "`agent_reported`: worktree lease 与 event-store append lock 分层；前者是长期、可恢复 domain resource，后者是短时写入互斥。",
      "`agent_reported`: Git side effect 使用 prepare/execute/inspect/append，避免“事件已成功但 Git 未发生”或 crash 后盲目重试。",
      "`agent_reported`: 默认 integration policy 为 merge --no-ff；冲突回到原 node/session owner，不自动解决、不静默覆盖。",
      "`agent_reported`: V2 shadow 只有 audit/rollback 权，不双写 V3 domain，不成为第二 projection source。"
    ]
  }
]
```

## Neighbors

```json
[
  {
    "task_id": "T-20260711080144-5db160f2",
    "title": "M6 完成 legacy-v2 只读迁移与发布硬化",
    "active_capabilities": [
      "align"
    ],
    "shared_files": []
  }
]
```

## File: .vibehub/tasks/T-20260711080144-de5ecb84/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: T-20260711080144-de5ecb84
title: M5 用稳定 V3 自管理多会话与 worktree 编排
mode: guided_drive
phase: implement
phase_status: active
created_at: 2026-07-11T08:01:44Z
created_by: vibehub
intent: 仅在 M4 稳定后，让 V3 自己管理 M5 的任务状态，并实现 scope、lease、worktree、集成与冲突生命周期。
acceptance_criteria:
- 存在 M4 稳定性进入门：核心投影、验收闭环、恢复基准与双平台 smoke 连续通过
- M5 的任务、节点、session 与验收由 V3 自己记录并可恢复
- 三个并行 session 跨至少两种工具且工作区/事件无污染
- 冲突、崩溃、桌面退出与 Windows 文件占用均可解释和恢复
dependencies:
- M4
intake:
  batch_id: intake-20260711080143
  split_confidence: high
  suggested_order: 5
  total_tasks: 3
  source_message: align阶段已经结束，现在需要进行调查和计划阶段，产出正式 research pack、五份 RFC backlog 和 M0 Task Pack。M0–M6 分别创建独立任务，不能做成一个超级大任务。M0 固化契约和 fixtures，M1 做高保真前端，M2 再接真实 core/MCP。等 M4 稳定后再让 v3 自己管理 M5，避免过早 self-host。
  split_reason: 用户明确要求 M0-M6 分别创建独立任务，且每个里程碑都有独立交付面、依赖与验收边界。
```

## File: .vibehub/tasks/T-20260711080144-de5ecb84/runs/R-20260711080144-924b5971/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: T-20260711080144-de5ecb84
run_id: R-20260711080144-924b5971
mode: guided_drive
phase: implement
phase_status: active
created_at: 2026-07-11T08:01:44Z
created_by: vibehub
baseline_commit: null
```

## File: .vibehub/rules/hard-rules.md

Reason: protocol hard rules

```text
# VibeHub Hard Rules

- Agent output is reported state only.
- Only VibeHub code updates canonical state transitions.
- Do not mark state.yaml completed from agent output.
- Distinguish hard_observed, agent_reported, inferred, and user_confirmed evidence.
- P0/P1 observability is best-effort and must not claim full runtime observation.
- Agents should read agent-view files and the current context pack, not the whole .vibehub directory.
- Keep changes scoped to the active task.

## CI/CD 改动纪律 (2026-05-21 从 8 轮返工中总结)

### 1. 本地先跑通再改 CI
- 任何 macOS CI 构建改动，**先在本地 macOS 验证**：
  `npm run tauri -- build --target aarch64-apple-darwin --bundles app`
- 本地能成功 `hdiutil create -fs APFS`，再改 GitHub Actions。
- CI 不是调试器，不要拿它当测试环境用。

### 2. 每次只改一个变量
- CI workflow 单次改动只改一项：构建方式 / 签名方式 / DMG 方式 分开验证。
- 改多个变量时无法定位失败原因，导致穷举试错。

### 3. 签名方案先问"要不要"
- **prerelease / 预发布**: 用 ad-hoc 签名 (`APPLE_SIGNING_IDENTITY="-"`)，不走 notarization。
- **正式发布**: 才需要 Developer ID 证书 + 公证流程。
- 不要默认启用全套 Apple 签名，先确认是否必要。

### 4. 优先用直接命令，少用 Action 封装
- `npm run tauri -- build` 直接 shell 命令 > `tauri-action` GitHub Action。
- 直接命令可以在本地完美复现，action 的传参行为是黑盒。
- 必须用 action 时，先查源码理解其内部命令拼接逻辑。
```

## Known Missing Context

- None declared.

## Stop Condition

Write output.md and return to VibeHub for validation.

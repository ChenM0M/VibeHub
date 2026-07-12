# Context Pack: Align

Task: T-20260711080144-5db160f2
Run: R-20260711080144-b9c6e08e
Phase: Align
Generated at: 2026-07-12T08:42:56Z
Source commit: b24a9e9

## Instructions

Use this context only for the current phase.
Do not mark state.yaml completed.
Report files read, commands run, decisions made, and unresolved risks.

## Capability Output Schema

```json
{
  "required_fields": [
    "intent",
    "scope",
    "success_criteria",
    "non_goals"
  ],
  "optional_fields": [
    "stakeholders",
    "references"
  ],
  "produces": [
    "alignment_summary"
  ],
  "consumes": [],
  "parallel_safe": false,
  "custom": false
}
```

## Prior Outputs Summary

```json
[
  {
    "capability": "review",
    "completed": [
      "`user_confirmed`: 用户要求根据上一轮审查问题完成修复，并在修复后重新进行完整自行审查。",
      "`hard_observed`: 上一轮 6 个 findings 已全部修复并有负向回归测试：operation journal 跳阶段、owner override 绕过 dependency/parallel gate、非 Ready worktree 普通 acquire、`..`/absolute/root escape scope、released lease reclaim、idempotency payload/actor/version 漂移。",
      "`hard_observed`: 完整复审额外发现并修复 Rust command API 可接受空白 session/lease/operation identity 与空 reclaim challenge 的边界；acquire/reclaim 现在要求非空 next challenge。",
      "`hard_observed`: 以基线 `b24a9e9a15db755a5663f9dd9686fa74cc90e177` 重新审查 contracts/fixtures、eligibility/path、Git adapter/journal、event store/orchestration、projection/application 五条链路。",
      "`hard_observed`: 最终代码审查未发现剩余可操作问题；`verdict = approved_with_external_validation_pending`，`gate_pass = true`（仅针对当前实现 diff 的 Review gate）。",
      "### Diff Summary",
      "`agent_reported`: M5 contracts 与基础 core slice 的安全边界现已与 RFC-005 一致：dependency/parallel gate 不可 override，路径先做平台无关 lexical normalization，普通 lease acquire 只允许 Ready，所有 ownership transfer 走带 challenge 和 dead-process evidence 的 reclaim，released lease 不可复活，Git operation 和 idempotent request 都保持真实顺序与语义。",
      "`hard_observed`: 复审证据已重新生成到 `evidence/diff.patch`、`evidence/changed-files.txt` 与 `phases/review.md`。",
      "### Concerns",
      "`hard_observed`: 无剩余 P0/P1/P2 代码 finding。",
      "`agent_reported`: 真实临时仓库三 worktree/integration、agent kill/desktop exit、Windows locked-file native smoke 与 self-host shadow 尚未执行；这些是 M5 环境验收项，不是当前 diff 中已观察到的代码缺陷。",
      "### Gate Pass",
      "`hard_observed`: `gate_pass = true`，原 6 项 review blockers 及复审新增的 identity/challenge blocker 均已修复并通过回归测试。",
      "`agent_reported`: 此 gate 不等同于打开 `entry_gate.self_host_writes_allowed`；M4 gate digest、Windows native evidence 与 owner approval 未满足前，真实 self-host 写入仍须保持关闭。",
      "### Verdict",
      "`hard_observed`: `verdict = approved_with_external_validation_pending`。",
      "`hard_observed`: 当前实现 diff 无剩余可操作代码 finding；Review gate 通过，native/self-host 环境验收继续保持 pending。",
      "### Risk Review",
      "`hard_observed`: ownership、scope、side-effect truthfulness 与 idempotency 的已知绕过路径均有拒绝测试覆盖。",
      "`inferred`: 本地单元/契约测试无法替代跨进程 crash、Windows 文件锁和真实 Git 冲突恢复，残余风险集中在尚未运行的 native integration harness。",
      "`inferred`: 工作区包含 115 个 VibeHub/任务相关变更文件，未获得逐文件用户归属确认；本轮仅增量修改当前 M5 core/review 输出，未回退或改写其他已有变更。",
      "### Evidence Grades",
      "`hard_observed`: Git diff/source、VibeHub sync/review evidence、257 个 Rust tests、423 项 contract assertions、TypeScript/Vite build、format 与 whitespace checks。",
      "`agent_reported`: 修复意图、Review gate 与 self-host entry gate 的边界、后续 native harness 建议。",
      "`inferred`: 未运行的跨进程/Windows/self-host 场景残余风险与未确认 dirty-file ownership 风险。",
      "`user_confirmed`: 按既有审查问题修复并完成一次完整复审的用户指令。"
    ],
    "full_ref": ".vibehub/tasks/T-20260711080144-de5ecb84/runs/R-20260711080144-924b5971/outputs/output.md",
    "key_decisions": [
      "`agent_reported`: 只有 scope overlap、case collision、denylist/generated/canonical 与 observed-file collision 可被带证据 owner override 降级；dependency、parallel safety 与 invalid path 永久不可 override。",
      "`agent_reported`: lease 普通 acquire 只允许 Ready；Active/Reclaimed ownership transfer 必须使用同 lease identity、generation+1、非空 challenge 与 dead-process evidence；release 清空 challenge。",
      "`agent_reported`: event-store duplicate 必须比较 expected version、完整 identity、actor、evidence grade、commit、payload 与显式 occurred_at；真正相同的 retry 才返回 Duplicate。",
      "`agent_reported`: Review gate 与 self-host entry gate 分离，代码复审通过不自动声明 native/self-host 验收完成。"
    ]
  }
]
```

## Neighbors

```json
[
  {
    "task_id": "T-20260711080144-de5ecb84",
    "title": "M5 用稳定 V3 自管理多会话与 worktree 编排",
    "active_capabilities": [],
    "shared_files": []
  }
]
```

## File: .vibehub/tasks/T-20260711080144-5db160f2/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: T-20260711080144-5db160f2
title: M6 完成 legacy-v2 只读迁移与发布硬化
mode: guided_drive
phase: align
phase_status: active
created_at: 2026-07-11T08:01:44Z
created_by: vibehub
intent: 隔离 legacy-v2 只读浏览，删除旧写路径，并完成 Windows/macOS 安装升级卸载与发布验证。
acceptance_criteria:
- legacy-v2 仅经独立只读 adapter 提供最小历史入口，不进入 v3 domain
- 旧命令、重复适配与 UI canonical 写入被清理
- Windows/macOS 从干净安装到 MCP、IDE、并行节点与卸载清理全流程通过
- 发布、回滚与归档文档可执行
dependencies:
- M5
intake:
  batch_id: intake-20260711080144
  split_confidence: high
  suggested_order: 6
  total_tasks: 1
  source_message: align阶段已经结束，现在需要进行调查和计划阶段，产出正式 research pack、五份 RFC backlog 和 M0 Task Pack。M0–M6 分别创建独立任务，不能做成一个超级大任务。M0 固化契约和 fixtures，M1 做高保真前端，M2 再接真实 core/MCP。等 M4 稳定后再让 v3 自己管理 M5，避免过早 self-host。
  split_reason: 用户明确要求 M0-M6 分别创建独立任务，且每个里程碑都有独立交付面、依赖与验收边界。
```

## File: .vibehub/tasks/T-20260711080144-5db160f2/runs/R-20260711080144-b9c6e08e/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: "T-20260711080144-5db160f2"
run_id: "R-20260711080144-b9c6e08e"
mode: "guided_drive"
phase: "align"
phase_status: "active"
created_at: "2026-07-11T08:01:44Z"
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

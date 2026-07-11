# Context Pack: Align

Task: T-20260711080143-49b5012c
Run: R-20260711080143-b14d3ac8
Phase: Align
Generated at: 2026-07-11T09:03:36Z
Source commit: d7ece57

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
      "`agent_reported`: Review summary：M0 contract/fixture/type/repository/RFC surface 与 task intent 一致，未发现未修复的 P0/P1/P2 缺陷；M1 可在不读取 V2 YAML、调用 Tauri 或连接 core/MCP 的情况下加载五类 view。",
      "`hard_observed`: Review 发现并修复四项 fixture/contract 语义缺口：FX-EMPTY 的 synthetic active work、FX-PARALLEL session lane 不足、FX-REWORK 缺第二次 attempt、Windows mixed-separator 未被拒绝。",
      "`hard_observed`: 修复后 `npm run v3:contracts:check` 通过 285 assertions、12 scenarios、5 views；新增 semantic assertions 防止四项问题回归。",
      "`agent_reported`: Gate pass：是。任务 task.yaml 的四项 acceptance criteria 和 Task Pack M0-C01 至 M0-C12 均有实现/测试证据，只有明确标注的原生平台实验留给后续里程碑。",
      "`hard_observed`: VibeHub review evidence 已生成到当前 run 的 `phases/review.md`、`evidence/changed-files.txt` 和 `evidence/diff.patch`。",
      "### Diff Summary",
      "`agent_reported`: 新增并冻结五个 V3 read-model schema、共享 contract vocabulary、12 场景确定性 fixtures、generated TypeScript、transport-neutral fixture repository、单命令 validator，以及五份 RFC/M0 Task Pack 的 freeze evidence；未修改 V2 production modules。",
      "### Verdict",
      "`agent_reported`: `ready_for_human_review / gate_pass`。四项 review finding 已修复并新增回归断言；没有未修复的阻塞或高风险 finding。",
      "### Evidence Grades",
      "`hard_observed`: filesystem/Git diff、schema/fixture validator、TypeScript/Vite build、Cargo format/test、VibeHub-generated review evidence。",
      "`agent_reported`: 变更意图、M0/M1 边界、review verdict、rollback 与 handoff 建议。",
      "`inferred`: JSON contract evidence不等于 Windows/macOS native file-operation evidence。",
      "`user_confirmed`: 用户明确要求开始并完成 M0。"
    ],
    "full_ref": ".vibehub/tasks/T-20260711080143-c3669d9d/runs/R-20260711080143-bba8b0c9/outputs/output.md",
    "key_decisions": [
      "`agent_reported`: Review verdict 为 `ready/pass`；原生平台缺口不阻塞 wire/fixture freeze，因为 contract 未宣称执行能力，且后续 RFC exit criteria 保留强制 native evidence。",
      "`agent_reported`: Review 修复直接落入 canonical generator/schema，并由语义断言锁定，不手改生成 fixtures。",
      "`agent_reported`: VibeHub `review` 首次在 review output 生成前执行导致 workflow 标记 failed/needs_action；这是可恢复的 output ordering 问题，需写完本文件后重新 claim/validate review，不能把它误报为产品测试失败。"
    ]
  }
]
```

## Neighbors

```json
[
  {
    "task_id": "T-20260711080143-b1ff21ea",
    "title": "M2 实现 V3 事件核心与 MCP 控制面",
    "active_capabilities": [
      "align"
    ],
    "shared_files": []
  },
  {
    "task_id": "T-20260711080143-c3669d9d",
    "title": "M0 冻结 V3 契约、fixtures 与实施基线",
    "active_capabilities": [],
    "shared_files": []
  },
  {
    "task_id": "T-20260711080143-e975f3c8",
    "title": "M3 构建 Project Intelligence 与架构地图",
    "active_capabilities": [
      "align"
    ],
    "shared_files": []
  },
  {
    "task_id": "T-20260711080143-fbe94685",
    "title": "M4 完成 Task 计划图、时间线与验收闭环",
    "active_capabilities": [
      "align"
    ],
    "shared_files": []
  },
  {
    "task_id": "T-20260711080144-5db160f2",
    "title": "M6 完成 legacy-v2 只读迁移与发布硬化",
    "active_capabilities": [
      "align"
    ],
    "shared_files": []
  },
  {
    "task_id": "T-20260711080144-de5ecb84",
    "title": "M5 用稳定 V3 自管理多会话与 worktree 编排",
    "active_capabilities": [
      "align"
    ],
    "shared_files": []
  }
]
```

## File: .vibehub/tasks/T-20260711080143-49b5012c/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: T-20260711080143-49b5012c
title: M1 基于 fixtures 构建 V3 高保真前端体验
mode: guided_drive
phase: align
phase_status: active
created_at: 2026-07-11T08:01:43Z
created_by: vibehub
intent: 用 M0 contracts/fixtures 实现 Project 与 Task 双层高保真体验，不接真实 v3 core/MCP。
acceptance_criteria:
- Project Center 与 Task 体验覆盖主流程和 loading/empty/stale/error/partial/large-data 状态
- 字段均映射到 M0 contract 和 evidence source，无神秘指标
- Windows/macOS viewport、长路径与大数据 fixture 无溢出重叠
- 完成理解项目、发现风险、进入 Task、查看依据、IDE 深入的可用性走查
dependencies:
- M0
intake:
  batch_id: intake-20260711080143
  split_confidence: high
  suggested_order: 1
  total_tasks: 3
  source_message: align阶段已经结束，现在需要进行调查和计划阶段，产出正式 research pack、五份 RFC backlog 和 M0 Task Pack。M0–M6 分别创建独立任务，不能做成一个超级大任务。M0 固化契约和 fixtures，M1 做高保真前端，M2 再接真实 core/MCP。等 M4 稳定后再让 v3 自己管理 M5，避免过早 self-host。
  split_reason: 用户明确要求 M0-M6 分别创建独立任务，且每个里程碑都有独立交付面、依赖与验收边界。
```

## File: .vibehub/tasks/T-20260711080143-49b5012c/runs/R-20260711080143-b14d3ac8/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: "T-20260711080143-49b5012c"
run_id: "R-20260711080143-b14d3ac8"
mode: "guided_drive"
phase: "align"
phase_status: "active"
created_at: "2026-07-11T08:01:43Z"
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

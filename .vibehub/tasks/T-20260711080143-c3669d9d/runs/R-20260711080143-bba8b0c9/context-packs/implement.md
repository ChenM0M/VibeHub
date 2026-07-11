# Context Pack: Implement

Task: T-20260711080143-c3669d9d
Run: R-20260711080143-bba8b0c9
Phase: Implement
Generated at: 2026-07-11T08:31:01Z
Source commit: d7ece57

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
    "capability": "plan",
    "completed": [
      "`agent_reported`: Steps：",
      "1. 新建 `contracts/v3/common.schema.json`，冻结 contract version、ID、timestamp、freshness/completeness、evidence、warning、path、page、structured error 等共享词汇。",
      "2. 新建五个 JSON Schema 2020-12 read model，所有共享语义仅通过 `$ref` 引用 common schema；补充字段到 UX/evidence provenance 文档。",
      "3. 新建确定性 fixture generator，产出 FX-EMPTY、FX-HAPPY、FX-NO-DOCS、FX-PARALLEL、FX-REWORK、FX-STALE、FX-PARTIAL、FX-ERROR、FX-WIN-PATHS、FX-MAC-PATHS、FX-LARGE、FX-COVERAGE-GAP；每个场景带 scenario manifest。",
      "4. 新建 TypeScript fixture repository 与由 JSON Schema 生成的类型 surface；添加 schema/type drift check 和禁止 V2 YAML/Tauri/core 依赖的静态边界检查。",
      "5. 新建 `npm run v3:contracts:check`，串联 schema meta-validation、ref resolution、正反 fixture 校验、coverage、determinism、type drift 与 boundary checks。",
      "6. 更新 RFC-001 的 M0 source-of-truth 决策，给五份 RFC 补齐 owner、M0 triage、alternatives/evidence/exit/non-scope 状态；更新 M0 Task Pack 的 freeze evidence 与 M1 handoff。",
      "`agent_reported`: Validation plan：先运行 fixture/type generation，再运行 `npm run v3:contracts:check` 两次确认稳定；随后运行 `npm run build`、`cargo fmt --all -- --check`、`cargo test --workspace`、`git diff --check`；检查所有 M0-C01 至 M0-C11 证据，M0-C12 明确区分本次 owner 授权与尚未进行的视觉 walkthrough。",
      "`agent_reported`: Affected files：`contracts/v3/**`、`fixtures/v3/**`、`scripts/v3-contracts/**`、`src/v3/contracts/**`、`package.json`、`package-lock.json`、`docs/v3/rfc-backlog/*.md`、`docs/v3/m0-task-pack.md`，以及 VibeHub CLI 生成的当前 task/run 状态与 output。",
      "`agent_reported`: Rollback boundary：删除新增 V3 surface 并回退 package scripts/dependencies 即可；M0 不修改 V2 production modules、canonical data format 或 runtime command surface。"
    ],
    "full_ref": ".vibehub/tasks/T-20260711080143-c3669d9d/runs/R-20260711080143-bba8b0c9/outputs/output.md",
    "key_decisions": [
      "`agent_reported`: 使用 Node ESM 脚本和 Ajv 2020 执行契约校验；选择成熟 validator 以覆盖 meta-schema 与 `$ref`，避免自制 schema validator。",
      "`agent_reported`: JSON Schema 是 wire source of truth；TypeScript 类型由 schema 生成并纳入 drift check。Rust round-trip 延后到 M2，因为 M0 不需要修改 core，RFC-001 记录该明确边界。",
      "`agent_reported`: fixtures 按场景生成，每个 view 文件独立校验；FX-LARGE 以固定 seed、稳定排序和 SHA-256 保证可重复。",
      "`agent_reported`: Windows/macOS path fixture 只声明 contract identity/display parsing 证据；不在单一主机上声称原生 file-operation 支持。"
    ]
  }
]
```

## Neighbors

```json
[
  {
    "task_id": "T-20260711080143-49b5012c",
    "title": "M1 基于 fixtures 构建 V3 高保真前端体验",
    "active_capabilities": [
      "align"
    ],
    "shared_files": []
  },
  {
    "task_id": "T-20260711080143-b1ff21ea",
    "title": "M2 实现 V3 事件核心与 MCP 控制面",
    "active_capabilities": [
      "align"
    ],
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

## File: .vibehub/tasks/T-20260711080143-c3669d9d/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: T-20260711080143-c3669d9d
title: M0 冻结 V3 契约、fixtures 与实施基线
mode: guided_drive
phase: plan
phase_status: completed
created_at: 2026-07-11T08:01:43Z
created_by: vibehub
intent: 冻结五类 view contract、事件与证据术语、双平台 fixtures，并形成能独立驱动 M1 的 M0 Task Pack。
acceptance_criteria:
- ProjectOverviewView、ProjectStructureView、TaskTimelineView、PlanGraphView、NodeBrief 的 JSON Schema 与版本策略完成
- fixtures 覆盖空项目、大仓库、无架构文档、并行 Task、Windows 路径、stale/partial/error 状态
- 五份 RFC backlog 的决策问题、依赖与退出条件完成
- M1 可仅依赖 contracts/fixtures 开发，不读取 v2 YAML 或调用真实 v3 core
dependencies: []
intake:
  batch_id: intake-20260711080143
  split_confidence: high
  suggested_order: 0
  total_tasks: 3
  source_message: align阶段已经结束，现在需要进行调查和计划阶段，产出正式 research pack、五份 RFC backlog 和 M0 Task Pack。M0–M6 分别创建独立任务，不能做成一个超级大任务。M0 固化契约和 fixtures，M1 做高保真前端，M2 再接真实 core/MCP。等 M4 稳定后再让 v3 自己管理 M5，避免过早 self-host。
  split_reason: 用户明确要求 M0-M6 分别创建独立任务，且每个里程碑都有独立交付面、依赖与验收边界。
```

## File: .vibehub/tasks/T-20260711080143-c3669d9d/runs/R-20260711080143-bba8b0c9/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: T-20260711080143-c3669d9d
run_id: R-20260711080143-bba8b0c9
mode: guided_drive
phase: plan
phase_status: completed
created_at: 2026-07-11T08:01:43Z
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

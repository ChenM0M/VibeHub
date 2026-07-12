# Context Pack: Implement

Task: T-20260711080143-e975f3c8
Run: R-20260711080143-b413c29d
Phase: Implement
Generated at: 2026-07-12T04:18:28Z
Source commit: 4359ef6

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
      "`hard_observed`: 完成 dirty-worktree/HEAD/ignore/analyzer registry model fingerprint、全量 Git tracked+untracked-exclude-standard snapshot、snapshot identity/schema 恢复校验与损坏降级。",
      "`hard_observed`: 完成 Cargo/npm/Python manifest、README/ADR 与 Rust/TS/JS/Python 静态 import evidence analyzers；动态 import、超 1 MiB 或不可读源文件不产生猜测关系。",
      "`hard_observed`: 完成 tree/Git/manifest/import/documentation 五类失效单元分类，普通无关文件不会触发 semantic invalidation。",
      "`hard_observed`: StructureArchitecture 已接生产目录懒加载、model-scoped pagination merge、180ms 去抖服务端搜索、loading/error 状态；fixture 模式保持本地查询。",
      "`hard_observed`: 新增真实 core release benchmark harness 与结果文档；VibeHub、LLMplayground、graduation_song_vote 各 5 次样本全部通过 first page/full index/search/snapshot size 冻结预算。",
      "`hard_observed`: 1440x900 与 1024x600 浏览器 QA 均为 document width 等于 viewport、0 个控件越界，结构搜索框可见。",
      "`hard_observed`: 新增 `v3::project_intelligence`，实现 model-version 绑定的稳定目录分页、1-500 page limit、安全相对路径校验、固定重目录过滤、符号链接/元数据 structured gap 与 Git changed overlay。",
      "`hard_observed`: 实现不可变 `ProjectModelSnapshot` 与同目录 staging + fsync + rename 原子 publish；测试覆盖分页无重复、跨 model cursor 拒绝和 snapshot 可读性。",
      "`hard_observed`: `V3ViewRepository` 的 production structure view 已从 M3 pending root 占位切换到真实 filesystem/Git 投影，overview 同步报告 tracked files、首层 modules、generator/model version；索引失败诚实降级为 recoverable unavailable/error。",
      "`hard_observed`: 新增 `v3_query_project_structure` Tauri command 与 `queryV3ProjectStructure` TypeScript service，支持按目录、cursor 和 limit 查询；保留现有四 Tab/StructureArchitecture 产品入口。",
      "`hard_observed`: 完整 core 218 tests、Tauri cargo check、V3 contract 366 assertions/12 scenarios 与前端 production build 全部通过。",
      "`hard_observed`: Align 已通过 validate/output-lint/handoff，VibeHub CLI 将当前任务推进到 Plan。",
      "`hard_observed`: 完成旧扫描、V3 contracts、M2 production projection、M1 StructureArchitecture、依赖现状与三个本地 corpus 的只读调查。",
      "`hard_observed`: 形成实现步骤、验证矩阵、影响文件、失效规则、首批 analyzer、数值预算和回滚边界。"
    ],
    "full_ref": ".vibehub/tasks/T-20260711080143-e975f3c8/runs/R-20260711080143-b413c29d/outputs/output.md",
    "key_decisions": [
      "`user_confirmed`: 按 VibeHub 路由从 M3 Align 正式进入 Plan 并开始推进 M3。",
      "`agent_reported`: 采用 immutable snapshot + atomic publish，不引入 Project Intelligence 第二事件权威源。",
      "`agent_reported`: 先交真实 tree/Git vertical slice，再按 Cargo/npm/Python 顺序增加 analyzer；inference 不进入首批交付。",
      "`agent_reported`: first interactive 与 full index 分离，超大仓库不能阻塞现有结构架构 Tab。",
      "`agent_reported`: 冻结上述 numeric budgets；失败应优化或降级能力，不在实现后放宽 gate。"
    ]
  }
]
```

## Neighbors

```json
[
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

## File: .vibehub/tasks/T-20260711080143-e975f3c8/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: T-20260711080143-e975f3c8
title: M3 构建 Project Intelligence 与架构地图
mode: guided_drive
phase: implement
phase_status: active
created_at: 2026-07-11T08:01:43Z
created_by: vibehub
intent: 把现有文件结构浏览升级为证据化、增量化、可降级的项目架构与知识视图。
acceptance_criteria:
- 文件树支持按需分页、搜索、Git 叠层与 ignore
- manifest/import/symbol/README/ADR 证据进入版本化项目模型
- fresh/stale/partial/unsupported/rebuilding 生命周期可观察
- VibeHub 与至少两个异构样本项目达到性能和真实性预算
dependencies:
- M2
intake:
  batch_id: intake-20260711080143
  split_confidence: high
  suggested_order: 3
  total_tasks: 3
  source_message: align阶段已经结束，现在需要进行调查和计划阶段，产出正式 research pack、五份 RFC backlog 和 M0 Task Pack。M0–M6 分别创建独立任务，不能做成一个超级大任务。M0 固化契约和 fixtures，M1 做高保真前端，M2 再接真实 core/MCP。等 M4 稳定后再让 v3 自己管理 M5，避免过早 self-host。
  split_reason: 用户明确要求 M0-M6 分别创建独立任务，且每个里程碑都有独立交付面、依赖与验收边界。
```

## File: .vibehub/tasks/T-20260711080143-e975f3c8/runs/R-20260711080143-b413c29d/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: T-20260711080143-e975f3c8
run_id: R-20260711080143-b413c29d
mode: guided_drive
phase: implement
phase_status: active
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

# Task / Session / Projection 真值门禁
本文定义 V3 任务视图的读取语义。它只描述由事件日志重建出的事实和诊断，
不回写历史事件、不把旧的绿色摘要升级成当前成功。

## 事实层次

任务级视图按以下顺序解释事实：

1. `lifecycle.fold_task` 重建 Task、PlanNode、Criterion、Finding、Session 和
   completion proposal/confirmation。事件日志是事实来源；`task.yaml` 的
   `phase_status` 只作为没有 V3 生命周期事件时的降级输入。
2. Session 完整性从该 Task 的全部带 `session_id` 事件重建。只有历史绑定、没有
   `session.opened`，或者 Session 已关闭/发生 gap 但没有 `succeeded`/`failed`
   terminal `agent.result_recorded`，都标记为 `unknown` / `blocked` 风险。
3. projection freshness 是独立事实。事件数与 `projection_event_count` 不一致、
   projection 缺失或不可解析时，视图 `freshness=stale`，并给出
   `V3_PROJECTION_STALE` 与 `v3 rebuild <project_id>` 修复动作。
4. `node_brief.state` 是所选 PlanNode 的状态；`node_brief.task_state` 才是与
   timeline、project overview、task candidates 对齐的 Task 级结论。这样既能显示
   历史节点事实，也不会用节点状态覆盖 Task 的完成门禁。

## Task 级结论

`TaskTruthState` 是唯一的任务级状态投影。Criterion 的 `accepted`、`passed`、
`failed`、`blocked` 和原始 lifecycle 的 `completion_pending` 不是 Task terminal
状态，不能单独表示完成。

| 优先级 | 条件 | Task 结论 | 允许的下一步 |
|---|---|---|---|
| 1 | 显式 `closed_with_exceptions` | `closed_with_exceptions` | 保留未解决项，创建后续任务或按授权恢复 |
| 2 | 已记录 `completed`，但存在 unknown/gapped/无 terminal result Session 或 stale projection | `blocked` | 取得真实 Session/投影证据；不得显示为 completed |
| 3 | PlanNode/criterion blocked 或 failed，或 finding 未关闭 | `blocked` | 修复、reopen/replan 或记录准确风险 |
| 4 | `task.completion_proposed` 尚未被当前 digest/用户确认闭环 | `review` | 复核 evidence 与 confirmation |
| 5 | 生命周期为 active，或仍有 active PlanNode | `active` | 继续执行并记录 progress/result |
| 6 | 依赖已 ready、尚未执行 | `planned` | 激活 ready PlanNode |
| 7 | 有有效 `task.completion_confirmed`，无更高优先级阻塞事实 | `completed` | 只读回顾；不再当作可写执行目标 |

`cancelled` 和 `closed_with_exceptions` 仍是 terminal，但两者都不等价于
all-green `completed`。没有 typed completion confirmation 的旧 `phase_status:
completed` 只能进入 `review`，不能单独生成 archive completed。

## 视图一致性规则

- `task_candidates` 只返回非 terminal Task；它的 `state` 使用同一 TaskTruthState。
- `task_timeline.state`、`project_overview.active_tasks[].state`、
  `project_overview.archived_tasks[].state` 与 `node_brief.task_state` 必须相同。
- `node_brief.state` 只用于 PlanNode drawer；检查 Task 完成时必须读取
  `node_brief.task_state`，不能把它与节点状态混用。
- `review`、`blocked`、`completion_pending` 和 `completed` 的原始事件/criteria
  仍保留在 `completion`、`criteria`、timeline events 和 blocker details 中；视图
  不能用摘要覆盖这些事实。
- stale projection 会同时降低 overview/timeline/node brief 的 freshness 和
  completeness，并产生结构化 warning/blocker。freshness 不是成功证据。

## Evidence 归属

| 证据 | 能证明什么 | 不能证明什么 |
|---|---|---|
| 源码、单元/契约测试 | 模型实现和控制面规则在当前 checkout 的行为 | 真实外部平台运行、发布 artifact 或历史 Session 缺失事实 |
| V3 lifecycle/session/projection 事件与 typed review | workflow 控制面记录了什么、哪些门禁已满足 | 没有记录的执行事实；unknown Session 不能被补写成成功 |
| Git HEAD、commit range、发布 artifact/hash | 代码范围与发布产物溯源 | Windows/macOS 原生行为，除非在对应平台真实执行 |
| 外部平台手测、OAuth、Windows 主机证据 | 对应外部环境的运行事实 | 不能替代本地代码/Task/Session 的 V3 closure |

只要 required criterion、Session terminal result、projection freshness、plan
terminal 或外部平台证据缺失，Task 必须保持 `partial`、`blocked` 或 `review`，
不能调用 `task_completion_propose` 伪装成已完成。

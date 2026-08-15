# Session–Task 绑定与轻量路由

本文件冻结 V3 的 Session–Task 绑定语义。它解决一个交互项目同时存在多个
active Task 时，Agent 把连续请求、计划、进度或结果写入错误 Task 的问题。
事件、Task、Session 和投影仍以 V3 typed command/MCP 结果为事实来源；本文是
契约说明，不是状态存储。

## 范围与不变式

- 绑定身份由 `project_id`、`interaction_id`、`session_id` 组成，可附带
  `agent_id` 与 `host`。不同 Session 的绑定永远相互独立。
- `SessionTaskBinding` 使用 schema version `1.0`，记录 `bound_task_id`、
  `binding_revision`、`target_task_revision`、`freshness`、`source`、`status`
  与时间。`status` 至少覆盖 `unbound`、`bound`、`stale`、`invalid`。
- 候选只来自项目内非终态 Task。项目 `current/default`、UI 的
  `selected_task_id` 和当前 Session 的 `bound_task_id` 是三个不同概念；前两者
  只能提供展示或兼容 fallback，不能替代 Session binding。
- 只有 `HostCapabilities.state=known` 且宿主同时声明 Session binding 与 binding
  precondition 能力时，路由结果才可进入写入路径；能力未知或未声明完整时不得猜测
  写入目标，必须 fail closed 并返回结构化诊断。

契约文件为
[`contracts/v3/session-task-routing.schema.json`](../../contracts/v3/session-task-routing.schema.json)，
TypeScript 类型由 `npm run v3:contracts:generate` 生成到
`src/v3/contracts/generated/session-task-routing.ts`。

## RouteDecision

`RouteRequest` 和 `TaskRouteDecision` 共享 schema version。决策动作只有四种：

| action | 含义 | 是否可直接作为写入目标 |
| --- | --- | --- |
| `continue` | 复用当前 Session 的 fresh binding | 是，需同时提交 binding revision |
| `bind` | 根据明确指令或唯一高置信候选建立/切换绑定 | 是，先完成 bind 事件 |
| `ask` | 证据不足、候选重复、绑定失效或宿主能力未知 | 否；先询问用户 |
| `new` | 明确是独立执行需求且没有可安全复用的 Task | 否；先执行显式 `task_create` intake，再按需要 bind |

`ask` 的 `options` 最多三个。中文或跨语言语义不足、同名/近似重复、最高两个
候选接近、指定 Task 不存在或已终态时，不能把“最像”的 Task 当作写入目标。
候选必须属于当前 project；隐式候选绑定还需要高置信的多词证据或完整标题匹配，
单个泛化词不能单独成为写入依据。

## 确定性路由优先级

轻量路由只在触发器出现时运行一次。普通连续请求直接复用当前 fresh binding，
不重新扫描全部 Task、不调用额外模型，也不向用户输出路由解释。

发生明确切换、新独立执行需求、绑定 Task 终态/失效或范围冲突时，按以下顺序
判断：

1. 用户显式提供 `task_id` 或唯一标题指令；
2. 当前 Session 的有效 binding；
3. 同一交互刚创建且用户明确要求 `create-and-start` 的 Task；
4. 唯一且高置信的已有候选；
5. 证据不足返回 `ask`，明确独立执行且无候选时才返回 `new`。

`project_current_default_task_id` 和 `ui_selected_task_id` 不参与上述写入优先级。
单纯查看其它 Task 也不得改变绑定。

## 写入门禁与失败恢复

未绑定 Session 可以读取 `task_candidates`、`task_view`、代码和文档并讨论；不能
执行源码、配置或 V3 工作流的增删改。`task_create` 是明确用户意图下的 intake
写操作，但 create-only 不改变 current/default，也不抢占既有 Session。

正式 `session_open` 以及 plan、criterion、finding、memory、orchestration、
progress/result 等 Task-scoped 写入必须携带 `session_id`，由服务端校验：

- binding 的 `project_id`、`bound_task_id` 与请求目标一致；
- `binding_revision` 与服务端当前 revision 一致；
- binding 和目标 Task 仍 fresh、active；
- 宿主声明了可执行的 binding precondition 能力。

校验失败返回 `V3_TASK_BINDING_REQUIRED`、`V3_TASK_BINDING_MISMATCH`、
`V3_TASK_BINDING_REVISION_CONFLICT` 或对应的结构化诊断，并带可执行的重新选择/
重新绑定动作。失败不能通过读取 current pointer 或 Agent prompt 静默修复。

`create-and-start` 是两个可审计动作：先记录 `task_create`，成功后再记录
`session_task_bind`；任一步失败都保留真实的“已创建/未绑定”状态，允许安全重试。
`session_task_unbind` 会使后续 Task-scoped 写入重新进入 required gate。

## 执行复杂度

绑定状态与 effective execution policy 正交：

- 未绑定只读讨论不创建 Task；
- 单一原子低风险修改在创建/绑定后走 `lightweight` 最小 session/result/验证；
- 普通多里程碑工作走 `standard`；
- 发布、迁移、安全、跨平台或多 Agent 工作走 `full`。

路由不能静默降低 policy；`lightweight` 也不会被扩展成完整 DAG 或每轮路由。

## 兼容边界

历史 current pointer 只作为明确标注的 default/fallback 读取来源。新 Task 不再
更新 pointer，旧客户端可继续读取它但不能把它解释为 Session binding。绑定事实
通过 `session.task_bound` / `session.task_unbound` 事件投影到 overview/timeline；
恢复和迁移只能使用受支持的 typed command，不能直接编辑事件日志、projection 或
current pointer。

本范围不新增从 Task 卡片启动 Agent 的入口，不做模型能力档位路由，也不重构无关的
Plan/DAG、Agent Profile 编辑器或 Workspace 项目标签。

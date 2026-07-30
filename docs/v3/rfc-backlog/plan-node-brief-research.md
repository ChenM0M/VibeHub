# 计划视图节点详情调研：为什么已完成/非当前节点点击没有任何反应

- 任务：`task.task.419944a3160a` / 节点 `node.task.task.419944a3160a.initial`
- 会话：`ses.419944a3160a.research.01`
- 日期：2026-07-30
- 状态：调研完成，**本节点未修改任何实现代码**
- 目标：在动手改 `src/v3/app/V3Cockpit.tsx` / `crates/vibehub-core/src/v3/views.rs` 之前，确认（A）现象与根因链路，（B）后端到底支不支持任意节点简报，（C）几种修法的真实代价与取舍。

---

## 0. 现象基线

V3 Cockpit →「计划」标签的 React Flow 图里：

- 每个节点都被渲染成 `cursor-pointer`（`src/v3/components/task/PlanGraph.tsx:76`），侧边提示文案 `v3.plan.nodeHint` = 「点击节点查看详细简报」（`src/locales/zh.json`）。
- 实际只有**一个**节点点得开右侧简报抽屉；点其它节点（尤其是 `completed`）**完全没有反应**：不弹框、不报错、不 loading，只有一个选中描边。

---

## 1. 根因链路（三处，逐处有硬证据）

### 1.1 UI 门禁是身份相等判断，不是状态判断

`src/v3/app/V3Cockpit.tsx:580`（plan 标签渲染 `PlanGraph` 的那一行）：

```tsx
onNodeClick={(node) => { if (node.node_id === nodeBrief.node_id) setNodeBriefPanel(nodeBrief); }}
```

- 抽屉状态只有一个 `nodeBriefPanel`（`V3Cockpit.tsx:148`），渲染点在 `V3Cockpit.tsx:881-889`。
- 也就是说：**唯一能打开的节点是 bundle 里那份 `node_brief` 对应的节点**，且打开后展示的永远是那份 brief，而不是被点节点的数据。条件不成立时是**静默 return**，无任何反馈。
- 另一个入口「查看完整简报」（`V3Cockpit.tsx:489`）同样硬绑这份唯一 brief。
- `PlanGraph.tsx:237-241` 的 `handleNodeClick` 本身是正确的：它按 `node_id` 找到完整节点并回调，锅不在 PlanGraph。

### 1.2 一个 bundle 只投影一份 node_brief，且选谁不由用户决定

`crates/vibehub-core/src/v3/views.rs:101-142` `select_node_id`：`active` → `ready`/依赖已完成的 `planned` → `blocked`/`failed` → `nodes.keys().next()`。`completed` 永远不被优先。

`views.rs:527` 只算出一个 `node_id`，`views.rs:822-853` 只产出一份 `node_brief`。

真实数据（本机 `.vibehub`，`task.ai-anomaly.db4b93b16a17`，7 个节点全部 completed/cancelled）：

```
$ ./target/debug/vibehub v3 . task-view task.ai-anomaly.db4b93b16a17
node.cache-stale completed
node.gates completed
node.metric completed
node.reader-fix completed
node.research completed
node.task.ai-anomaly.db4b93b16a17.initial cancelled
node.warnings completed
default brief -> node.cache-stale completed
```

即：7 个节点里只有恰好排第一的 `node.cache-stale` 能点开，其余 6 个全是死点击。这与用户报的「已完成的任务点不开详情」完全吻合。

### 1.3 前端从来没把 node_id 传下去（后端其实早就支持）

- Tauri 命令签名本来就有 `node_id`：`src-tauri/src/commands.rs:352-382` → `repository.load_bundle_for_node(&task_id, node_id.as_deref())`（`views.rs:276`）。
- 但前端 `src/services/v3ProductionViews.ts:20-24` 只传 `projectPath / taskId / expectedProjectId`，**永不传 node_id**；`V3ProductionLoader` 类型（`src/v3/stores/v3Store.ts:44-48`）里也没有这个参数。
- `v3Store` 里 `selectedNodeId` / `selectNode`（`v3Store.ts:100,141,409-423,616`）是死代码：`V3Cockpit` 与 `PlanGraph` 都不读它，`PlanGraph` 自己维护本地 `selectedNodeId`（`PlanGraph.tsx:150`）。

**后端能力验证（硬证据）**：显式指定一个 completed 节点，简报正常返回该节点自身数据；指定不存在的节点会明确报错，不会静默回退。

```
$ ./target/debug/vibehub v3 . task-view task.ai-anomaly.db4b93b16a17 node.gates
brief node_id: node.gates | state: completed
goal: 跑真实门禁并逐条 criterion_review 留证：cargo test -p vibehub、cargo fmt -p vibehub -- --che
scope: ['src-tauri/src', 'src', 'scripts']
deps: ['node.cache-stale', 'node.metric', 'node.warnings']
criteria count: 6

$ ./target/debug/vibehub v3 . task-view task.ai-anomaly.db4b93b16a17 node.does-not-exist
{ "code": "V3_NODE_NOT_FOUND", "category": "not_found", "retryable": false, ... }
```

结论：**这不是后端缺能力，而是前端缺通路 + UI 门禁写错**。

---

## 2. 关键量化：payload 与耗时

对同一 bundle 实测（debug 构建，含进程启动；`v3 . task-view <task> node.gates`）：

| 分片 | JSON 字节 |
|---|---|
| project_structure | 1,816,469 |
| project_overview | 1,224,525 |
| task_timeline | 95,354 |
| **node_brief** | **60,080** |
| plan_graph | 24,207 |
| plan_graph.nodes | 5,356 |
| agent_results | 23,063 |
| **整包合计** | **4,547,150（约 4.5 MB）** |

耗时：`real 0.47 / 0.37 / 0.36` 秒（三次）。座舱本身已按 4 秒轮询这个整包（`V3Cockpit.tsx:223-234`）。

另一个结构性事实：`node_brief` 里**只有 5 个字段真正随节点变化**——`node_id`、`goal`、`scope`、`dependencies`、`state`（`views.rs:839-848`）。其余（`execution_policy`、`project_memory`、`protocol_records`、`completion_gate`、`budget`、`criteria`、`blocker_details`、`warnings`…）全是任务级，逐节点复制就是纯冗余。而 `plan_graph.nodes` 里**已经**带了 `title / goal / state / readiness / block_reasons / blocker_details / scope / criterion_ids / session_ids`（`views.rs:513-519`），依赖关系也已在 `scheduling_edges` 里。

---

## 3. 方案取舍

| 方案 | 做法 | 优点 | 代价 / 风险 |
|---|---|---|---|
| A. 点击时重取整包 bundle（带 node_id） | 复用现有 `v3_load_view_bundle` | 改动最小 | 每次点击拉 **4.5 MB / ~370ms**，还会顶掉当前 bundle 状态、与 4s 轮询打架；不可接受 |
| B. 新增瘦命令 `v3_load_node_brief(project_path, task_id, node_id)` | 只回 `node_brief` | 单节点 60KB，可按需取；后端 `select_node_id` 已支持 node_id，改动可控 | 多一个 IPC 往返（点击有可感知延迟，需 loading 态）；fixture/演示模式没有这个通路，需要兜底 |
| C. bundle 内投影全部节点简报 | `node_briefs: []` | 点击零往返，fixture 模式天然可用 | 7 节点 × 60KB ≈ **+420KB/次轮询**，且 95% 是重复的任务级字段；契约（`contracts/v3/node-brief.schema.json`、生成类型、fixtures、`v3:contracts:check`）全线改动 |
| **D（建议）. C-lite + B 按需** | 抽屉的**节点级区块**直接用 `plan_graph.nodes[i]` + `scheduling_edges` 渲染（零往返、fixture 模式可用）；**任务级区块**沿用已在内存里的当前 brief；仅当需要该节点的完整 brief 时用 B 的瘦命令按需取 | 点击即时有内容、任何状态节点都能开、无 payload 膨胀、fixture 模式不残废；同时满足 c03「真实的按 node_id 通路」 | 需要把抽屉内容按「节点级 / 任务级」明确分区，避免把任务级数字冒充成节点级；两条数据来源需保证 node_id 一致性校验 |

**建议采用 D。** 实现顺序：先做 B 的数据通路（`node.419944a3160a.brief-data`），再改 UI（`node.419944a3160a.ui`）。

---

## 4. 顺带发现（同任务内应一并处理或明确记为不做）

1. **`node_brief.criteria` 未按节点过滤**：请求 `node.gates` 时仍返回全部 6 条任务级 criteria（见 §1.3 输出），节点已有 `criterion_ids`（`views.rs:518`）可用。若抽屉要显示「本节点验收标准」，必须过滤，否则数字会误导。→ 建议在 `brief-data` 节点修，属 c03 范围。
2. **`blocker_details` 在 brief 里是任务级**（`views.rs:849` 用 `current_blocker_details`），而 `plan_graph` 里是**节点级**（`views.rs:506-512`）。抽屉应优先用节点级那份。
3. **状态编辑对 completed 是空下拉**：`PlanGraph.tsx:34-40` 的 `stateTransitions` 没有 `completed / review / cancelled` 键，点「更新状态」得到空 select + `noTransitions`；依赖编辑对 `active/completed` 直接 return（`PlanGraph.tsx:256`）。这是**另一个问题**（编辑而非查看），本任务只保证「查看」不再静默失败，编辑侧仅需保证有提示，不扩大范围。
4. **已归档任务完全没有计划图**：终态任务被 `views.rs:322` 从 `active_tasks` 排除，归档详情（`V3Cockpit.tsx:826-878`）只有 `plan.completed/plan.total` 计数。若用户说的「已完成的任务」也包含这一层，需要另开任务，**本任务不覆盖**。
5. **`selectedTaskHasDetail` 门禁**（`V3Cockpit.tsx:268-271`）要求 timeline/planGraph/nodeBrief 三个 task_id 与选中任务一致，否则整个计划标签退化为提示文案——修 UI 时不要绕过它，否则会在多任务场景渲染错任务的图。

---

## 5. 验证口径（供 `node.419944a3160a.verify` 使用）

- 仓库没有 `npm run lint` / `npm run typecheck` 脚本（`package.json.scripts` 实测）。c05 的「或等价」按以下执行：
  - `npx tsc --noEmit`
  - `npm run v3:contracts:check`
  - `npm run v3:i18n:check`
  - `cargo test -p vibehub-core`（涉及 `views.rs`）+ `cargo fmt -- --check`
- UI 行为回归建议复用现成的 esbuild headless 断言范式：`scripts/v3-cockpit/background-refresh.mjs`（直接 import `v3Store.ts` 跑真实 store 断言），新增一支 `scripts/v3-cockpit/plan-node-brief.mjs` 断言「任意状态节点点击都产出抽屉数据 + 不可用时有明确反馈」，这样 c02/c04 才有可执行证据而不是截图口述。

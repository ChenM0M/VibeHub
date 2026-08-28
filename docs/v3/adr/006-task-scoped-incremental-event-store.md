# ADR-006：Task-scoped 增量事件索引与投影

- 状态：Accepted
- 冻结日期：2026-08-23
- 适用版本：VibeHub V3 store format 2
- 对应 Task：`task.v3-task-scoped.cb5f33fa90c0`

## 决策摘要

保留 `events.jsonl` 作为不可变、可审计、可从零重放的事件事实来源；新增一个由
SQLite WAL 承载、可删除后重建的 `store-v2.sqlite3` 作为事件索引和增量读模型。
`task.yaml` 继续只保存受限 Task 元数据，Plan DAG、Criterion、Session、Finding、
Worktree 和 Project Memory 的可变历史仍只由事件表达。

写入保留一个很短的 Project 级全序临界区，用于 partial-tail 检查、分配连续
`global_seq`、核对 aggregate version / idempotency key、追加并 `fsync` 一条 JSONL，
以及提交对应 SQLite 事务。临界区内不得读取完整日志、对所有 Task fold 或重写整份
Project projection。不同 Task 仍共享可解释的 Project 全序，但不再共享全量 rebuild
的长临界区。

SQLite 是派生状态而不是第二事实来源。索引落后、事务失败或读模型更新失败时，已
`fsync` 的 JSONL 事件保持有效，索引进入 `lagging`/`rebuild_required`；所有依赖新鲜
投影的读取 fail closed，并给出确定的 rebuild/recovery 动作，不能静默返回旧投影。

## 2026-08-23 真实基线

测量对象为 `/Users/chenm0m/LocalRepo/VibeHub`，使用当前源码构建的 debug CLI
`target/debug/vibehub`。测量发生在 N01 Session 打开后、首个 progress 写入前；因此
后续控制面事件会使计数自然增长。

| 指标 | 实测值 |
| --- | ---: |
| Project events | 5,922 |
| Task | 141（active 2 / archived 139） |
| PlanNode | 541 |
| Session / binding / aggregate | 417 / 70 / 574 |
| `events.jsonl` | 6,406,452 B |
| `projection.json` | 3,881,937 B |
| `task-candidates` compact response | 1,590 B |
| `task-candidates` 20 次 median / p95 | 532.767 / 549.180 ms |
| `task-view` compact response | 7,034,615 B |
| `task-view` 20 次 median / p95 | 1,014.154 / 1,036.786 ms |
| `projection-status` 20 次 p95 | 118.574 ms |

首个真实 progress 写入的 MCP 端到端调用耗时约 5.0 s；这个数字包含 MCP/进程边界，
不是纯磁盘 append 延迟，但它准确反映当前 Agent 可见的写入上界。规模 fixture 必须
另外记录纯 store append 延迟。

确定性 fixture `deterministic-v3-store-scale-v1` 使用 1,000 Tasks；debug 基线为：

| events | JSONL | 全量解析 | `projection::fold` | compact projection |
| ---: | ---: | ---: | ---: | ---: |
| 10,000 | 5,364,000 B | 88.226 ms | 112.640 ms | 2,322,392 B |
| 50,000 | 27,203,000 B | 414.154 ms | 762.710 ms | 10,644,392 B |

该 fixture 位于 `crates/vibehub-core/examples/v3_store_benchmark.rs`，固定 Task/Event/
aggregate 分布并只使用唯一临时目录；每次运行结束删除自身 disposable 数据。

## 已确认的放大路径

1. `event_store::append_inner` 在 `events.lock` 下执行 partial-tail recovery 后调用
   `read_events`，每次写入都解析完整 Project 日志。
2. lifecycle/project-memory/orchestration 写入在验证前先 `load_project`，随后
   `append_with_rebuild` 再次加载，形成一次命令内的重复全量读取。
3. `append_with_rebuild` 在同一锁内调用 `projection::fold` 并原子替换完整
   `projection.json`。
4. `projection::fold` 先收集 Task ID，随后对每个 Task 调用 `fold_task(task_id,
   events)`，形成 O(Task x Event) 扫描。
5. `views::load_bundle_for_node` 重新加载全部事件、再次执行 Project fold，并为每个
   Task 重复构造 lifecycle/session/criteria/blocker；随后把全部 archived Task、完整
   Project Structure、所有 Task evidence 和未分页 timeline 塞进一个 bundle。
6. 所有写入共享 `.vibehub/v3/projects/<project_id>/events.lock`，上述全量工作把本应
   很短的全序 append 临界区扩展为 Project 级长锁。

## 方案比较

| 方案 | 优点 | 主要风险 | 结论 |
| --- | --- | --- | --- |
| JSONL + JSON offset index + Task 分片 JSON | 依赖少；文件可直接检查 | 跨多个索引/分片的原子提交、自愈与 Windows rename/锁语义复杂；容易形成自制事务日志 | 不选 |
| JSONL + SQLite WAL 派生索引/读模型 | 原生复合索引、事务、WAL 并发读、可重建；仓库已有受支持的 bundled SQLite 依赖链 | 必须严格处理 JSONL 已 durable 而 DB 未提交的窗口；要测试 WAL/锁/路径平台差异 | **采用** |
| SQLite 作为唯一事件事实来源 | 单事务最简、查询最快 | 改变现有审计介质和兼容边界，迁移与回滚风险最大；旧工具无法读取 | 暂不采用 |

## Store format 2

`store-v2.sqlite3` 至少包含以下逻辑表/索引（物理 schema 可兼容扩展）：

- `store_meta(project_id, format_version, schema_version, status,
  indexed_event_count, indexed_source_bytes, last_global_seq, last_event_id,
  last_event_timestamp)`；
- `events(global_seq, event_id, task_id, aggregate_id, aggregate_version,
  idempotency_key, byte_offset, byte_length, recorded_at, envelope_json)`；
- 唯一索引：`event_id`、`(aggregate_id, aggregate_version)`、
  `(task_id, idempotency_key, event_type)`；
- 查询索引：`task_id + global_seq`、`aggregate_id + global_seq`、
  `idempotency_key`、`global_seq`；
- 增量投影行：Task lifecycle/Worktree 按 `task_id` 分片，Session/binding 按
  `session_id` 汇聚跨 Task 历史，Project Memory 独立；一次事件只更新受影响的行和
  Project 摘要，不重写其他 Task。

`projection.json` 降为格式 1 兼容 checkpoint：完整 rebuild、显式 export 或迁移时可
原子生成，但 steady-state append 不再依赖或重写它。新读路径以 SQLite 的连续
watermark 为 freshness 判据，并以 JSONL 当前安全尾部作交叉校验。

## 读取边界

- `task-view(task_id)` 只加载指定 Task 的事件/投影、该 Task 的 Session/binding、
  PlanGraph、Criterion、Finding/Attempt 和有界 timeline/evidence；
- `project_overview` 只包含 active Task 的有界摘要和归档计数，不包含 archived Task
  详情；
- archived Task 使用 `task-list(include_archived, cursor, limit)`；
- Project Structure 使用既有独立分页/search surface；task-view 只给摘要和查询
  locator，不内嵌完整图；
- 大 evidence/timeline 使用 cursor + limit，有 `returned/total_estimate/next_cursor`，
  不以“未分页但声称有 window”冒充有界读取；
- MCP、CLI、Tauri/UI 始终显式携带 `task_id`，`selected_task_id` 仅用于展示，不能改写
  Session binding。

## 一致性与故障域

1. aggregate optimistic concurrency 与幂等判定来自已追平的唯一索引；索引不新鲜时
   禁止写入并先恢复。
2. JSONL append 成功但 SQLite 事务失败：返回结构化 projection/index failure，记录
   durable source cursor，后续读写 fail closed；rebuild 从最后连续 watermark 或从零
   追平，不能回删已 durable 事件。
3. SQLite 事务成功前不得发布新的 freshness watermark；读者只观察已提交事务。
4. partial tail 只允许隔离最后一个不完整 JSON 片段；中间损坏、序号/身份不一致必须
   quarantine 并停止。
5. `projection.json` 写失败不再扩大到 steady-state append；显式 checkpoint 失败保留
   上一个完整文件并报告失败。
6. SQLite WAL/SHM 锁只保护数据库事务；JSONL Project sequencer 仅保护全序 append，
   不包围 Task fold、全项目 view 组装或 checkpoint 写出。

## 兼容迁移与回滚

- 首次打开格式 1 项目时只读校验 canonical root、project id、`events.jsonl` 安全尾部
  与可选 `projection.json` 身份；不得跟随逃逸 control root 的 symlink。
- 在同目录创建唯一临时数据库，完整导入并逐字段对比从零 fold；`fsync` 文件和父目录
  后原子 rename 为 `store-v2.sqlite3`。原始 JSONL、projection、current pointer、
  task.yaml 和 legacy-v2 均不改写。
- 临时构建失败直接删除/保留 quarantine 临时物，不会改变读指针；重复启动可重试。
- 已安装数据库损坏或 watermark 落后时保留带 hash 的诊断信息，重建到新的临时数据库
  并原子替换；旧数据库在成功替换前保持可恢复。Windows 与 macOS 分别验证 open-file
  rename、WAL checkpoint、锁超时和路径语义。

## 验证承诺

- 确定性 fixture：10k/50k events 与 1k Tasks；固定 ID、event type 分布和 payload；
- rebuild 等价：格式 1 全量 fold 与格式 2 分片/索引重建逐字段一致；
- 性能：真实项目 task-view 20 次 compact JSON < 1 MiB 且 p95 <= 300 ms；规模门禁
  p95 <= 500 ms，steady append p95 <= 250 ms；
- 正确性：独立 Task 并发、热点 aggregate 版本冲突、重复请求、partial tail、索引落后、
  projection/SQLite 写失败、锁超时、迁移中断与回滚；
- 平台：macOS 与 Windows 原生文件锁、原子替换、崩溃恢复和路径语义分别留证。没有
  Windows 原生环境时 criterion 必须保持 blocked，不能用 CI/macOS 代替。

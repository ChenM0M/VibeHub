# V3 Task-scoped 索引与增量投影验证（2026-08-23）

## 结论

N03 将 `events.jsonl` 保留为不可变审计事实源，并新增可从零重建的 `store-v2.sqlite3`（SQLite WAL）派生层。steady-state Task/Session/Plan 写入不再反序列化完整 Project JSONL，不再调用 Project 全量 `projection::fold`，也不再重写 `projection.json`。后者仅由显式 `projection_rebuild` 生成，作为兼容与人工检查快照。

## 索引与分片

- `events.global_seq`：JSONL 行顺序对应的 Project 全序。
- `events(task_id, global_seq)`：Task-scoped 事件读取。
- `events(aggregate_id, aggregate_version)`：乐观并发与 aggregate 历史。
- `events.idempotency_key`：Project 内唯一幂等键。
- `events.event_id`：审计事件唯一身份。
- `meta.source_byte_offset`：已索引 JSONL 字节水位；同时报告源文件字节数和最近全局序号。
- `task_projections`：Task lifecycle、criteria、finding、Plan DAG 与关系历史。
- `session_projections`、`session_bindings`：Session 状态、progress/risk/result 与显式 Task binding。
- `worktree_projections`：Task-scoped worktree/lease 投影。
- `project_memory`：Project Memory 独立分片。

每次追加先在短 Project JSONL 临界区内完成 partial-tail 恢复、索引水位追平、幂等/expected-version 判定和单行 durable append；随后释放 JSONL 锁，在一个 SQLite `IMMEDIATE` 事务中插入事件并只刷新受影响 Task/Session/binding/Worktree/Memory 分片。若派生事务失败，JSONL 事件仍 durable，返回 `V3_PROJECTION_UPDATE_FAILED`，下一次 typed read/write 从 `source_byte_offset` 追平，不读取或改写历史语义。

## 验证

参考环境：macOS 26.6.2（Darwin 25.6.0）、arm64 MacBook Air、Rust 1.98.0；CPU 型号读取被当前沙箱拒绝。基准使用 debug build，因此数字不冒充 release 性能。

### 字段等价与故障追平

`cargo test --locked -p vibehub-core v3::event_store -- --nocapture`

- 16/16 passed。
- `indexed_shards_are_field_equivalent_to_full_jsonl_replay` 将 Task lifecycle、Session progress、Worktree 和 Memory 事件从 JSONL 完整 replay，并与 SQLite 分片组装结果做 `V3Projection` 全字段相等断言。
- `append_returns_structured_error_when_incremental_projection_transaction_fails` 用 SQLite trigger 注入分片事务失败：JSONL 保留事件；移除 fault 后 typed read 自动从尾部追平到 2/2 events。
- `steady_state_append_updates_index_without_rewriting_compatibility_projection` 证明普通 append 后 `projection.json` 不存在，显式 rebuild 才生成兼容快照。

### V3 回归

`cargo test --locked -p vibehub-core v3:: -- --skip protocol_runtime::tests::healthy_sidecar_is_reused_and_unhealthy_registry_is_cleaned --skip protocol_runtime::tests::sidecar_failed_spawn_releases_lock_and_unknown_ids_fail_closed --skip protocol_runtime::tests::sidecar_port_conflict_lock_lifecycle_and_crash_recovery_are_deterministic`

- 238/238 passed。
- 三条跳过项在受限沙箱内均因 loopback 端口分配返回 `Operation not permitted`；它们不属于事件存储逻辑，必须在最终 workspace gate 的可绑定环境重跑，不能据此声称通过。
- 首轮发现的 store-format 旧断言和 current-pointer fallback 回归已修复并逐条单线程重跑通过。

### 1k Tasks / 50k events

`cargo run --locked -p vibehub-core --example v3_store_benchmark -- 50000`

debug build、1,000 Tasks、50,000 初始 events：

- JSONL fixture：27,203,000 bytes。
- 首次建索引与分片：3,034.038 ms（一次性追平成本）。
- 已索引的 50k event reload：370.214 ms。
- 旧完整 `projection::fold` 对照：476.668 ms。
- 200 次 steady-state append：p50 17.704 ms，p95 20.799 ms，max 28.862 ms。
- steady-state 后 `compatibility_projection_rewritten=false`。

该结果证明 steady-state append 未随 50k Project 规模回退到全日志反序列化或整份 Project 投影重写。并发独立 Task、热点 aggregate、崩溃/锁超时和平台文件语义属于 N04/N06 的后续门禁，不能由本报告替代。

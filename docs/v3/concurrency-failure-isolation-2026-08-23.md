# V3 并发与失败隔离验证（2026-08-23）

## 临界区

store format 2 的写入分为两段：

1. 锁外打开/初始化 SQLite 并追平 JSONL 水位；首次建库、SQLite busy 等待和受影响分片 fold 均不持有 `events.lock`。
2. 获取 `events.lock` 后只执行 partial-tail 恢复、只读比较 SQLite watermark 与 JSONL 长度、幂等/aggregate version 查询和单行 durable append。若 watermark 落后，立即释放锁，在锁外追平后重试。

append 完成后，受影响 Task/Session/binding/Worktree/Memory 分片在 SQLite `IMMEDIATE` transaction 内提交。JSONL 是 durable 事实源；派生事务失败不会回写或删除 JSONL，下次 typed operation 从 `source_byte_offset` 追平。

## 发现并修复的竞态

首次并发套件真实发现并记录了两类缺陷：

- 每个并发 `open` 都重复执行 schema 和 `PRAGMA journal_mode=WAL`，热点 aggregate 出现 `V3_INDEX_SCHEMA_FAILED: database is locked`。修复为 schema-ready 快速路径，并对首次初始化的 busy/locked 做 3 秒有界、锁外重试。
- `sync_from_jsonl` 在等待 SQLite transaction 前缓存源文件长度；等待期间另一写者可能提交新 watermark，导致旧长度与新 watermark 比较并误报 `V3_INDEX_AHEAD_OF_SOURCE`。修复为 transaction snapshot 建立后再采样 JSONL 长度；status 最后采样源长度。

这两项都由失败测试先暴露，再修复并重跑；没有把首次失败改写成通过。

## 并发与故障矩阵

`cargo test --locked -p vibehub-core v3::event_store -- --nocapture`

- 20/20 passed。
- 24 个独立 Task 同时执行 plan/session/progress 类追加：48 条 seed+concurrent events 全部保留，event ID 唯一，SQLite 分片组装与 JSONL replay 全字段相等，steady-state 未生成 `projection.json`。
- 16 个写者竞争同一 aggregate、相同 `expected_version=0`：严格得到 1 个 appended、15 个 `V3_VERSION_CONFLICT`。
- 16 个写者重试同一 aggregate、同一幂等键和相同 payload：严格得到 1 个 appended、15 个 duplicate no-op，最终 aggregate version 为 2、事件总数为 2。
- 热点用例并行独立运行 4 次，4/4 passed。
- 手工持有 SQLite `BEGIN IMMEDIATE` 时，等待中的 typed writer 不持有 JSONL 锁；测试线程在 1 秒内成功获取 `events.lock`。
- partial tail 被 quarantine 后再 append；dead-process lock 自动恢复；live-process lock 在 3 秒后返回 `V3_LOCK_TIMEOUT`，不偷锁。
- 手工把合法事件只追加到 JSONL，index status 先报告 `behind`；typed projection read 追平后返回 2/2 events，不静默返回旧投影。
- SQLite trigger 注入分片事务失败时返回 `V3_PROJECTION_UPDATE_FAILED`；JSONL 事件保留，去除 fault 后 typed read 自动追平。

扩大回归命令（排除当前沙箱禁止端口绑定的 3 条 protocol sidecar 测试）为 242/242 passed。跨进程崩溃恢复、迁移回滚和原生 Windows 文件语义仍分别属于 N05/N06，不能由本报告替代。

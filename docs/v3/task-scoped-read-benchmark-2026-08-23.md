# Task-scoped 读取基准（2026-08-23）

对应 Task：`task.v3-task-scoped.cb5f33fa90c0`，节点：
`node.v3-store.n02-bounded-read-surfaces`。

## 环境与方法

- 主机：当前 macOS 参考主机；
- 构建：当前源码 `target/debug/vibehub`；
- 数据：真实 `project.vibehub`，基线 5,922 events / 141 Tasks / 541 PlanNodes，
  验证时因 V3 控制面事件自然增长；
- 方法：同一命令连续执行 20 次，以 nearest-rank 计算 p95；compact JSON stdout 的
  byte length 作为响应体积；
- 命令：`target/debug/vibehub v3 . task-view
  task.v3-task-scoped.cb5f33fa90c0`。

## 结果

| 版本 | response | median | p95 | max |
| --- | ---: | ---: | ---: | ---: |
| N01 基线 | 7,034,615 B | 1,014.154 ms | 1,036.786 ms | 1,048.811 ms |
| N02 bounded bundle | 184,042 B | 281.123 ms | 291.388 ms | 1,656.300 ms |

N02 结果满足 response < 1 MiB、连续 20 次 p95 <= 300 ms。单个 1.656 s
调度离群值仍真实保留；nearest-rank p95 是排序后的第 19 个样本，不删除样本。

同期 `task-candidates` 的 20 次结果为 1,590 B、median 243.069 ms、p95
247.689 ms；其进一步消除全日志解析由 N03 索引节点负责。

## 契约边界

- `task-view` 只内嵌目标 Task 的 Plan/Criteria/Session/binding 和最近 200 条 timeline；
- Project overview 只含 active Task 摘要、`archived_task_count` 和
  `task_archive_page` locator；`archived_tasks` 在 bundle 中为空；
- archived Task 详情通过 MCP `task_archive_page`、CLI `task-archive-page` 和 Tauri
  `v3_query_archived_tasks` 获取，默认 50、最大 100；
- Project Structure bundle 只含 workspace/index locator；首个页面和搜索继续使用
  `v3_query_project_structure`；前端结构页在 locator bundle 上自动请求第一页；
- session bindings 只返回绑定到目标 Task 的记录；`ui_selected_task_id` 仍不参与写入
  绑定或路由；
- 格式 1 兼容 projection 在一次请求中只解析一次；projection 缺失/stale 的测试 fixture
  仍从已验证 events 显式回退并携带 stale warning，不静默信任旧状态。

## 验证

- `cargo test --locked -p vibehub-core v3::views`：37 passed；
- `cargo test --locked -p vibehub-core v3::event_store`：15 passed；
- `cargo check --locked -p vibehub`：passed；
- `npm run v3:contracts:check`：818 assertions passed；
- `npm run v3:mcp:check`：7 resources / 31 tools / 1,279 ms total，passed；
- `npm run build`：passed；
- in-app browser：`?v3-playground=1` 可见烟测通过，结构架构 tab 正常展示模块、关系和
  文件树，浏览器 console 无 warn/error。生产 Tauri on-demand surface 由 Rust/Tauri/MCP
  contract 覆盖；原生桌面交互留到 N06 平台门禁。

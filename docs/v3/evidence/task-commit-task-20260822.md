# V3 Task 索引与 Git 双向溯源证据

日期：2026-08-22
项目：`project.vibehub`
Task：`task.v3-task-commit-task.606ff931a359`
采集根目录：`/Users/chenm0m/LocalRepo/VibeHub`
采集二进制：`/Users/chenm0m/LocalRepo/VibeHub/target/debug/vibehub`（当前源码构建）
采集时 `git rev-parse HEAD`：`9427e281d7ed586a85a019c038b96c8cb9337dcf`

本报告记录查询命令和真实输出摘要。工作树当时包含其他 Task 的 dirty 改动；当前 HEAD 只作为环境事实记录，不能作为本 Task 的 Session Git 绑定证据。

## 查询命令

```sh
./target/debug/vibehub v3 . projection-status project.vibehub
./target/debug/vibehub v3 . task-list
./target/debug/vibehub v3 . task-list --include-archived
./target/debug/vibehub v3 . task-candidates
./target/debug/vibehub v3 . task-commits task.v3-task-commit-task.606ff931a359
./target/debug/vibehub v3 . commit-tasks 9427e281d7ed586a85a019c038b96c8cb9337dcf
./target/debug/vibehub v3 . commit-tasks 9427e28
./target/debug/vibehub v3 . commit-tasks deadbeef
npm run v3:contracts:check
npm run v3:mcp:check
```

下面的 `task-list` 与 `commit-tasks` 摘要使用 `jq` 只保留本 Task 和结构字段；未过滤的命令仍是上面列出的 CLI 查询。`commit-tasks` 未过滤输出包含全项目历史缺口，因此摘要保留总数和本 Task 的全部缺口条目。

## projection-status

```json
{
  "error": null,
  "event_count": 5738,
  "last_event_timestamp": "2026-08-22T10:27:47.112Z",
  "project_id": "project.vibehub",
  "projection_event_count": 5738,
  "projection_metadata_consistent": true,
  "projection_path": "/Users/chenm0m/LocalRepo/VibeHub/.vibehub/v3/projects/project.vibehub/projection.json",
  "rebuild_state": "idle",
  "repair_action": null,
  "stale": false,
  "state": "synced"
}
```

## task-list 与 task_candidates

`task-list` 默认只返回非终态 Task，且不读取 current/default pointer 作为筛选条件：

```json
{
  "schema_version": "1.0",
  "project_id": "project.vibehub",
  "include_archived": false,
  "total_count": 23,
  "returned_count": 23,
  "active_count": 23,
  "archived_count": 0,
  "omitted_archived_count": 114,
  "sort": {
    "order": "non_terminal_first_then_terminal_at_desc",
    "state_tie_breaker": "state_rank_ascending",
    "terminal_at_missing": "last",
    "tie_breaker": "task_id_ascending"
  },
  "selected_task": [
    {
      "active_session_count": 1,
      "criterion_pass_rate": {
        "effective_passed": 0,
        "not_applicable": 0,
        "passed": 0,
        "ratio": 0.0,
        "total": 5
      },
      "intent": "补齐 V3 Task 的可发现性和 Git 双向溯源。当前 task_candidates 过滤 terminal task，task_list 只有基础字段，task_commits 只能 Task→Session HEAD，不能由任意 commit hash 反查 Task；历史 Session 的 open_git_head/close_git_head 也常为 null。实现必须把这些缺口拆成可验证的查询契约，不能把历史缺失的 Git 事实伪造成已存在。只修改 views、CLI/MCP 查询、Git 读取和对应 contract/test，不直接改事件日志或手工补历史事实。",
      "is_terminal": false,
      "required_node_completion_rate": {
        "completed": 2,
        "credited": 2,
        "ratio": 0.6666666666666666,
        "total": 3,
        "waived": 0
      },
      "source": "v3_event_log_projection",
      "state": "active",
      "task_id": "task.v3-task-commit-task.606ff931a359",
      "terminal_at": null,
      "title": "[V3溯源] Task 索引、归档查询与 commit→Task 反查",
      "workflow_profile": "full"
    }
  ]
}
```

`task-list --include-archived` 的相同 Task 摘要字段保持不变，项目计数变为 `total_count=137`、`active_count=23`、`archived_count=114`、`omitted_archived_count=0`，排序声明仍为 `non_terminal_first_then_terminal_at_desc`，缺失 `terminal_at` 仍排最后。

`task_candidates` 对该 Task 的完整候选摘要为：

```json
[
  {
    "active_session_count": 1,
    "intent": "补齐 V3 Task 的可发现性和 Git 双向溯源。当前 task_candidates 过滤 terminal task，task_list 只有基础字段，task_commits 只能 Task→Session HEAD，不能由任意 commit hash 反查 Task；历史 Session 的 open_git_head/close_git_head 也常为 null。实现必须把这些缺口拆成可验证的查询契约，不能把历史缺失的 Git 事实伪造成已存在。只修改 views、CLI/MCP 查询、Git 读取和对应 contract/test，不直接改事件日志或手工补历史事实。",
    "is_current_default": false,
    "is_ui_selected": false,
    "project_id": "project.vibehub",
    "state": "active",
    "task_id": "task.v3-task-commit-task.606ff931a359",
    "title": "[V3溯源] Task 索引、归档查询与 commit→Task 反查"
  }
]
```

## task-commits：Task → Session Git 事实

输入 `task_id=task.v3-task-commit-task.606ff931a359` 的完整 JSON 摘要：

```json
{
  "git_evidence": {
    "complete_sessions": 0,
    "historical_gaps": [
      {
        "missing": ["open_git_head", "close_git_head"],
        "reason": "historical session event did not record a Git HEAD; no commit binding is inferred",
        "session_id": "session.codex.v3-task-commit-task.606ff931a359.20260822",
        "task_id": "task.v3-task-commit-task.606ff931a359"
      },
      {
        "missing": ["open_git_head", "close_git_head"],
        "reason": "historical session event did not record a Git HEAD; no commit binding is inferred",
        "session_id": "session.codex.v3-task-commit-task.606ff931a359.n02.20260822",
        "task_id": "task.v3-task-commit-task.606ff931a359"
      },
      {
        "missing": ["open_git_head", "close_git_head"],
        "reason": "historical session event did not record a Git HEAD; no commit binding is inferred",
        "session_id": "session.codex.v3-task-commit-task.606ff931a359.n03.20260822",
        "task_id": "task.v3-task-commit-task.606ff931a359"
      }
    ],
    "missing_sessions": 3,
    "partial_sessions": 0,
    "status": "missing"
  },
  "project_id": "project.vibehub",
  "schema_version": "1.0",
  "session_count": 3,
  "sessions": [
    {
      "close_git_head": null,
      "closed_at": "2026-08-22T10:17:29.777Z",
      "event_commit_shas": [],
      "git_trace_status": "missing",
      "has_git_evidence": false,
      "open_git_head": null,
      "opened_at": "2026-08-22T09:42:55.665Z",
      "session_id": "session.codex.v3-task-commit-task.606ff931a359.20260822"
    },
    {
      "close_git_head": null,
      "closed_at": "2026-08-22T10:26:29.691Z",
      "event_commit_shas": [],
      "git_trace_status": "missing",
      "has_git_evidence": false,
      "open_git_head": null,
      "opened_at": "2026-08-22T10:18:36.225Z",
      "session_id": "session.codex.v3-task-commit-task.606ff931a359.n02.20260822"
    },
    {
      "close_git_head": null,
      "closed_at": null,
      "event_commit_shas": [],
      "git_trace_status": "missing",
      "has_git_evidence": false,
      "open_git_head": null,
      "opened_at": "2026-08-22T10:27:30.181Z",
      "session_id": "session.codex.v3-task-commit-task.606ff931a359.n03.20260822"
    }
  ],
  "source": "v3_session_git_head_events",
  "task_id": "task.v3-task-commit-task.606ff931a359"
}
```

结论：三个 Session 的 Git HEAD 都是历史缺口；`9427e281d7ed586a85a019c038b96c8cb9337dcf` 是当前仓库 HEAD，但没有被显示成上述 Session 的绑定。

## commit-tasks：commit → Task 反查

### 完整 hash

输入 `9427e281d7ed586a85a019c038b96c8cb9337dcf` 的结果（历史缺口按本 Task 保留）：

```json
{
  "schema_version": "1.0",
  "project_id": "project.vibehub",
  "query_commit_hash": "9427e281d7ed586a85a019c038b96c8cb9337dcf",
  "resolved_commit_hash": "9427e281d7ed586a85a019c038b96c8cb9337dcf",
  "match": "associated",
  "task_count": 4,
  "tasks": [
    {"task_id": "task.claude-ui-capability.19ff4ab9cd52", "title": "[Claude适配] 自动回落 UI 值与 capability 配置契约", "state": "active"},
    {"task_id": "task.v3-mcp.e793fdbe80f8", "title": "[V3修复] 投影写入一致性与 MCP 错误传播", "state": "blocked"},
    {"task_id": "task.v3-task-session-projection.5b0fe53a8469", "title": "[V3工作流] Task/Session/Projection 完成真值统一", "state": "active"},
    {"task_id": "task.windows-opencode-launcher.a92f9d632909", "title": "[Windows验收] OpenCode launcher 原生无黑框与配置路径验证", "state": "blocked"}
  ],
  "historical_gap_count": 396,
  "historical_gaps_for_task": [
    {"missing": ["open_git_head", "close_git_head"], "reason": "historical session event did not record a Git HEAD; range matching is unavailable", "session_id": "session.codex.v3-task-commit-task.606ff931a359.20260822", "task_id": "task.v3-task-commit-task.606ff931a359"},
    {"missing": ["open_git_head", "close_git_head"], "reason": "historical session event did not record a Git HEAD; range matching is unavailable", "session_id": "session.codex.v3-task-commit-task.606ff931a359.n02.20260822", "task_id": "task.v3-task-commit-task.606ff931a359"},
    {"missing": ["open_git_head", "close_git_head"], "reason": "historical session event did not record a Git HEAD; range matching is unavailable", "session_id": "session.codex.v3-task-commit-task.606ff931a359.n03.20260822", "task_id": "task.v3-task-commit-task.606ff931a359"}
  ],
  "source": "v3_git_traceability"
}
```

### 7 位短 hash

输入 `9427e28` 返回同一解析结果：`resolved_commit_hash` 为 `9427e281d7ed586a85a019c038b96c8cb9337dcf`，`match=associated`，`task_count=4`，Task 集合与完整 hash 相同，`historical_gap_count=396`，本 Task 仍列出上述三个缺失 HEAD。

### 未知 hash

输入 `deadbeef` 的完整结果摘要为：

```json
{
  "schema_version": "1.0",
  "project_id": "project.vibehub",
  "query_commit_hash": "deadbeef",
  "resolved_commit_hash": null,
  "match": "none",
  "task_count": 0,
  "tasks": [],
  "historical_gap_count": 396,
  "historical_gaps_for_task": [
    {"missing": ["open_git_head", "close_git_head"], "reason": "historical session event did not record a Git HEAD; range matching is unavailable", "session_id": "session.codex.v3-task-commit-task.606ff931a359.20260822", "task_id": "task.v3-task-commit-task.606ff931a359"},
    {"missing": ["open_git_head", "close_git_head"], "reason": "historical session event did not record a Git HEAD; range matching is unavailable", "session_id": "session.codex.v3-task-commit-task.606ff931a359.n02.20260822", "task_id": "task.v3-task-commit-task.606ff931a359"},
    {"missing": ["open_git_head", "close_git_head"], "reason": "historical session event did not record a Git HEAD; range matching is unavailable", "session_id": "session.codex.v3-task-commit-task.606ff931a359.n03.20260822", "task_id": "task.v3-task-commit-task.606ff931a359"}
  ],
  "source": "v3_git_traceability"
}
```

## 验证结果

通过：

- `cargo test --locked -p vibehub-core task_list_is_pointer_independent_and_exposes_rates_and_terminal_sorting -- --nocapture`
- `cargo test --locked -p vibehub-core archive_ordering_breaks_terminal_at_ties_by_task_id -- --nocapture`
- `cargo test --locked -p vibehub-core commit_tasks_supports_short_full_unknown_and_historical_missing_head_facts -- --nocapture`
- `cargo check --locked -p vibehub-core`
- `cargo check --locked -p vibehub-cli`
- `cargo build --locked -p vibehub-cli --bin vibehub`
- `npm run v3:contracts:check`：818 assertions、12 scenarios、6 views、5 write contracts
- `npm run v3:mcp:check`：真实 stdio session，7 resources、30 tools、37 requests，status `passed`

Git fixture 明确覆盖：一个 commit 关联多个 Task、完整/短 hash、未知 hash、归档任务、历史 Session 无 HEAD，以及 Task→Session 查询的 `missing/partial/complete` 字段。真实工作区的历史缺口则按上面的 `null` 与 `historical_gaps` 原样报告。

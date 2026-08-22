# V3 Task / Session / Projection 只读回归矩阵（2026-08-22）

本矩阵服务于 `task.v3-task-session-projection.5b0fe53a8469` 的 n03。它只读取
V3 事件、Task view、Task lifecycle、projection status、Git trace 和当前工作树，
不为历史 Task 补写 Session、result、criterion 或 completion 事件。

## 读取方法与基线

串行执行了以下命令；第一次并行读取因 projection 同步锁竞争返回的
`V3_LOCK_TIMEOUT` 已排除，不作为业务状态证据：

```text
target/debug/vibehub v3 <project> task-view <task_id>
target/debug/vibehub v3 <project> task-lifecycle project.vibehub <task_id>
target/debug/vibehub v3 <project> projection-status project.vibehub
target/debug/vibehub v3 <project> task-commits <task_id>
target/debug/vibehub v3 <project> task-list --include-archived
git status --short --branch
git rev-parse HEAD
git diff --name-only
```

串行 rebuild 后的 projection 证据为：`event_count=5729`、
`projection_event_count=5729`、`stale=false`、`rebuild_state=idle`。
代码 checkout 为 `fix/codex-model-catalog-json-provider`，HEAD 为
`9427e281d7ed586a85a019c038b96c8cb9337dcf`；工作树仍包含多个其他 Task 的
dirty 修改，因此文件名不能单独证明某个历史 Task 的提交归属。

n03 按 DAG 正式置为 `active`、Session 完成 bind/open 后，重新执行了同一组串行
查询。最终权威 projection 证据为：`event_count=5776`、
`projection_event_count=5776`、`stale=false`、`rebuild_state=idle`；以下状态均以
这次 n03 active 批次为准。

## 矩阵

| 计划中的 Task | 代码/工作树状态 | canonical Task view | lifecycle / Session 事实 | 证据缺口与建议动作 |
|---|---|---|---|---|
| `task.tags-launch-claude-code.8b0aab9fcb0a` | 当前工作树 dirty；`task-commits` 未记录 Git HEAD | `completed`，fresh；5 个 criterion 全部通过 | lifecycle v34=`completed`；2 个 Session 均 `closed/complete`；确认有效 | `open_git_head`/`close_git_head` 全缺失。保持 terminal，不补历史 HEAD；若需要溯源，使用新的带 Git 记录 Session。 |
| `task.mcp-agent-result-details.06c7e9f6fade` | 精确 ID 在当前 Task 文件和 V3 view 中不存在 | `V3_TASK_NOT_FOUND` | `task-lifecycle` 只返回空的降级 planned projection，不能视为真实 Task | 控制面实际存在的是 `task.mcp-agent-result-record-details.06c7e9f6fade`；按实际 ID 重读后为 `completed`、lifecycle v12、4 个 criterion passed、2 个 Session closed/complete，但 Git HEAD 全缺失。不得为拼写错误 ID 补事件。 |
| `task.workspace-state-revision.75c2cce41fc5` | 当前工作树 dirty；`task-commits` 未记录 Git HEAD | `completed`，fresh；3 个 criterion passed | lifecycle v9=`completed`；Session `revision-conflict-session` 为 `closed/complete`；确认有效 | 历史 Git provenance 缺失。保持 terminal，后续仅补受支持的真实 provenance，不把 dirty 文件归因给该 Task。 |
| `task.codex-model-catalog-json-provider.8c19c1157488` | 当前 checkout 有 Codex 相关 dirty 文件；Git trace 缺失 | `blocked`，fresh；阻塞 `session.codex-catalog-fix` 无 terminal result | lifecycle v50 原始状态=`completed`、criteria 全 passed、confirmation 有效，但 view/timeline canonical state=`blocked` | 未知/不完整 Session 使完成结论失效。不得重写历史或强行完成；核对真实 Session，必要时用新的 bound Session 重做验证。 |
| `task.claude-code-agent.9389b00ab06f` | 当前 checkout 有 Claude 相关 dirty 文件；Git trace 缺失 | `blocked`，fresh；`cli-cc-main`、`session.n02-advanced-ui`、`session.n04-validation` 为 unknown/incomplete | lifecycle v31 原始状态=`completed`、6 个 criterion passed；view/timeline 因 3 个 Session gap 统一为 blocked | 不得把绿色 criterion/archive 摘要升级为完成；不能伪造 recovery。需要真实执行结果，或以新 Session 重新验证。 |
| `task.mcp-agent-harness-vibehub-mcp.eca87960b032` | 当前 checkout 有 MCP 相关 dirty 文件；Git trace 缺失 | `blocked`，fresh；4 个 legacy/unknown Session gap | lifecycle v38 原始状态=`completed`、6 个 criterion passed、finding closed、attempt completed；view/timeline canonical state=`blocked` | `session.cleanup.1`、`session.mcp-stable-fix*` 没有可确认的 open/terminal 事实。保持 blocked，不能补写成功历史；按实际控制面重新执行或关闭缺口。 |
| `task.windows-opencode-agent.c03366fc710` | 精确 ID 不存在 | `V3_TASK_NOT_FOUND` | 该 ID 无真实 lifecycle | 当前控制面实际 ID 为 `task.windows-opencode-agent.c03366fc7109`；它是 `closed_with_exceptions`，c02 为 blocked，canonical c01/c02 仍 accepted 且无证据。后续外部验收 Task 为 `task.windows-opencode-launcher.a92f9d632909`，当前 `blocked`，5 个 criterion blocked；该 follow-up 的两个 Session 有完整 Git trace（open/close 均为当前 HEAD），但缺 Windows artifact/主机证据。 |
| `task.v3-git-task.2ff28709cbc1` | 当前工作树包含 projection/MCP 相关 dirty 文件；Git trace 缺失 | `completed`，fresh；criteria 全 passed | lifecycle v54=`completed`；Session `session-fix-projection-001` 为 `closed/complete`；确认有效 | 历史 Session 没有 open/close Git HEAD。保留 completed 业务结论，但把 Git provenance 作为独立缺口；不能据此证明当前 dirty diff 属于该 Task。 |

## 结论

1. lifecycle 的原始 `completed`、有效 confirmation 或全部绿色 criterion 不能覆盖
   unknown/legacy Session gap；Codex、Claude、MCP harness 三个 Task 已由统一 view
   降为 `blocked`。
2. `closed_with_exceptions` 是可见的 terminal 结论，不等价于 all-green
   `completed`；Windows 旧 Task 和后续 Windows 验收 Task 的平台证据仍保持 blocked。
3. 两个计划 ID 已不再是当前控制面中的实体。必须以 `task-list`/精确
   `task-view` 重新发现实际 ID，不能用 current/default pointer 或手工事件补齐。
4. 历史 Git HEAD 缺失是独立的 provenance gap；它不改变已经由 V3 closure 证明的
   业务状态，也不允许把当前 dirty checkout 归因给历史 Task。

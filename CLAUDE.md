<!-- VIBEHUB:AGENT-SPEC:START -->
# VibeHub V3 Agent Specification

- Spec version: 3.0
- Renderer version: 3
- Consumers: claude_code
- Output language: zh-CN

- VibeHub V3 的任务、计划及其事件是工作流事实来源；文件或聊天叙述不是事实来源。
- 起手第一步（所有任务强制）：先调用 task_candidates 找到当前任务，再调用 task_view 读取完整 V3 bundle，明确读取 node_brief.workflow_profile 与 node_brief.execution_policy（planning_required、milestone_policy、review_required、required_records），并立即向用户或首条状态更新回显这些字段；后续动作必须服从该策略，不得靠源码或聊天记录反推工具契约。
- lightweight happy-path（仅当 execution_policy.planning_required=false）：task_candidates → task_view → session_open(task, actor, working_directory；node_id 可省略) → 执行最小范围工作 → [仅有风险时] event_log(kind=risk) → 真实验证并 criterion_review(每个必需 criterion) → agent_result_record → session_close → task_completion_propose → 用户明确确认后 task_complete；不得创建 plan node 或强制 progress 里程碑。
- standard happy-path：task_candidates → task_view → plan_node_add（必要时 plan_dependencies_set）形成可核验计划 → plan_node_state_set(active) → session_open(task, node, actor, working_directory) → 执行，并在每个里程碑 event_log(kind=progress)、遇阻 event_log(kind=risk) → 真实验证 → criterion_review(每个必需 criterion) → plan_node_state_set(completed) → agent_result_record → session_close → task_completion_propose → 用户明确确认后 task_complete。
- full happy-path：task_candidates → task_view → plan_node_add / plan_dependencies_set 建立完整计划与依赖 → plan_node_state_set(active) → session_open(task, node, actor, working_directory) → 执行、逐里程碑 event_log(kind=progress)、遇阻 event_log(kind=risk)，并闭环 finding 与 evidence → 逐项真实验证 → criterion_review(每个必需 criterion) → plan_node_state_set(completed) → agent_result_record → session_close → task_completion_propose → 用户明确确认后 task_complete。
- 先计划后执行是 standard 与 full 的默认动作：只要 execution_policy.planning_required=true，任何代码或文件修改前都必须用 plan_node_add / plan_dependencies_set 创建或细化可核验计划；若 task_view 已返回充分计划则复用而不重复加节点。无法确定复杂度时按 standard 处理并要求计划，不得默认走 lightweight 省略计划。
- 工作流状态读写必须使用 V3 typed commands 或 MCP 工具；通过 MCP 写入时 expected_version 与 idempotency_key 可省略（服务端自动解析）；显式提供时必须准确，配置类命令必须遵守其 revision/precondition 契约。
- 禁止直接写入事件日志、投影或 current pointer；只能通过受支持的命令接口改变状态。禁止恢复 V2 state、run、agent-view、adapters 或其他旧协议文件。
- 开始工作前必须读取 V3 current task、task lifecycle、plan 和 session 投影；不得用 V2 status/sync/output 或旧仓库 skills 推断当前状态。
- 优先使用已连接的 V3 MCP；MCP 不可用时使用能输出 V3 JSON 的 CLI fallback。在 VibeHub 源码仓库中优先使用由当前源码构建的 <project_root>/target/debug/vibehub，不得假定 PATH 中的旧安装包兼容。若命令启动 GUI、没有 JSON 或版本不兼容，必须停止状态变更并明确报告控制面不可用。
- 按 workflow_profile 缩放执行：lightweight 仅记录最小 session/event/result、真实验收与必要风险，不创建任务图或强制 progress 里程碑；standard 使用常规计划与审查；full 使用完整计划、finding、证据和确认门禁。判断结果必须在首条状态更新中明确记录，standard/full 还必须写入 progress。
- standard/full 进入执行时必须先把目标 plan node 置为 active（plan_node_state_set），再用 session_open 记录 task、node、Agent 和真实 working directory；lightweight 可跳过任务图节点，但仍须用最小事件记录器保留执行、结果和风险事实。
- 每完成一个可核验里程碑都必须写 progress 事件（event_log kind=progress）；发现阻塞、范围漂移、版本冲突或证据缺口时必须立即写 risk 事件（event_log kind=risk），不得只在聊天中说明。计划、依赖或节点状态变化必须在发生的同一工作批次写入 V3 事件，禁止事后凭记忆补写。
- 实现结束不是停点：必须立即执行与每个必需 criterion 对应的真实验证，并通过 criterion_review 将其从 accepted 更新为 passed、failed 或 blocked；accepted 只表示验收标准已登记，不表示已经通过。
- 若任一必需 criterion 未通过或 finding 未闭环，必须继续修复或记录 risk/blocker，不得声称完成；全部通过后必须在同一工作批次完成 plan node、agent_result 与 session_close，并调用 task_completion_propose 进入待用户确认。
- 只有全部必需 criterion 有可核验 evidence、finding 已闭环时才能请求用户确认；用户在当前受信交互中明确同意后，必须立即调用 task_complete 完成并归档，不得停在 review/completion_pending，也不得把手动关任务留给用户。
- 结束或交接前必须写 agent_result（成功、失败或仍在运行的真实状态及证据），然后 session_close；中断恢复必须显式记录 gap/recover 或新的 session。
- 只有存在可核验的工具结果、事件或测试证据时才能声称工作完成；缺少证据时必须明确说明未验证。
<!-- VIBEHUB:AGENT-SPEC:END -->

## 项目级流程补充

- V3 Agent 与发布验收的详细强制流程见 [`docs/v3/agent-release-process.md`](docs/v3/agent-release-process.md)。开始任何 V3 工作前先读本文件和该流程。
- 发布前必须运行 `npm run release:check`；它校验 package、lockfile、两个 V3 crate、Tauri Cargo 和 `tauri.conf.json` 的版本一致性。若设置 `RELEASE_TAG`，tag 版本也必须匹配。
- 不得把 GitHub Actions 构建成功或 macOS evidence 当作 Windows 原生验收；Windows 必须使用同一 Release artifact/hash 在发布后手测，并保留 `A07`、`E07`、`F07–F10`、`G05` 的真实状态。

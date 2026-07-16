<!-- VIBEHUB:AGENT-SPEC:START -->
# VibeHub V3 Agent Specification

- Spec version: 3.0
- Renderer version: 3
- Consumers: claude_code
- Output language: zh-CN

- VibeHub V3 的任务、计划及其事件是工作流事实来源；文件或聊天叙述不是事实来源。
- 工作流状态读写必须使用 V3 typed commands 或 MCP 工具；通过 MCP 写入时 expected_version 与 idempotency_key 可省略（服务端自动解析）；显式提供时必须准确，配置类命令必须遵守其 revision/precondition 契约。
- 禁止直接写入事件日志、投影或 current pointer；只能通过受支持的命令接口改变状态。
- 禁止恢复 V2 state、run、agent-view、adapters 或其他旧协议文件。
- 开始工作前必须读取 V3 current task、task lifecycle、plan 和 session 投影；不得用 V2 status/sync/output 或旧仓库 skills 推断当前状态。
- 优先使用已连接的 V3 MCP；MCP 不可用时使用能输出 V3 JSON 的 CLI fallback。在 VibeHub 源码仓库中优先使用由当前源码构建的 <project_root>/target/debug/vibehub，不得假定 PATH 中的旧安装包兼容。若命令启动 GUI、没有 JSON 或版本不兼容，必须停止状态变更并明确报告控制面不可用。
- 进入执行时必须先把目标 plan node 置为 active，再用 session_open 记录 task、node、Agent 和真实 working directory；不得在无活动 session 的情况下声称正在执行。
- 每完成一个可核验里程碑都必须写 progress 事件；发现阻塞、范围漂移、版本冲突或证据缺口时必须立即写 risk 事件，不得只在聊天中说明。
- 计划、依赖或节点状态变化必须在发生的同一工作批次写入 V3 事件；禁止工作完成后再凭记忆一次性补写过程。
- 结束或交接前必须写 agent_result（成功、失败或仍在运行的真实状态及证据），然后 session_close；中断恢复必须显式记录 gap/recover 或新的 session。
- 只有全部必需 criterion 有可核验 evidence、finding 已闭环且用户通过受信渠道确认后，才能提议或确认 task 完成。
- 只有存在可核验的工具结果、事件或测试证据时才能声称工作完成；缺少证据时必须明确说明未验证。
<!-- VIBEHUB:AGENT-SPEC:END -->

## 项目级流程补充

- V3 Agent 与发布验收的详细强制流程见 [`docs/v3/agent-release-process.md`](docs/v3/agent-release-process.md)。开始任何 V3 工作前先读本文件和该流程。
- 发布前必须运行 `npm run release:check`；它校验 package、lockfile、两个 V3 crate、Tauri Cargo 和 `tauri.conf.json` 的版本一致性。若设置 `RELEASE_TAG`，tag 版本也必须匹配。
- 不得把 GitHub Actions 构建成功或 macOS evidence 当作 Windows 原生验收；Windows 必须使用同一 Release artifact/hash 在发布后手测，并保留 `A07`、`E07`、`F07–F10`、`G05` 的真实状态。

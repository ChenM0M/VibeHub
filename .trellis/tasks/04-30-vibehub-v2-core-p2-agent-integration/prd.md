# VibeHub v2 Core P2 Low-friction Agent Integration

## 目标

让常见 AI coding 工具能低摩擦读取 VibeHub v2 的静态协议、当前状态和上下文，不要求深度 runtime 集成，但要形成可自用的 Agent 入口体验。

## 需求

* 生成或维护 Agent 可读入口：
  * AGENTS/CLAUDE/OpenCode 等静态说明入口。
  * 指向 `.vibehub/agent-view/current.md`、`current-context.md`、`handoff.md`。
* 提供 adapter sync 预览能力或占位能力：
  * 展示会生成/更新哪些文件。
  * 不静默覆盖用户改动。
  * 有 managed region / conflict detection 的最小策略。
* Journal / knowledge promotion 手动版：
  * 至少能记录一次 session/work item 的结论。
  * 不要求自动归档或自动提交。
* 保持与 P0/P1 的文件协议一致。

## 验收标准

* [ ] 至少一种 Agent 入口文件能被生成/更新，并指向 VibeHub 当前状态。
* [ ] Agent 默认读取 agent-view 和 context pack，而不是整个 `.vibehub`。
* [ ] Adapter sync 不会静默覆盖用户改动。
* [ ] 有最小 journal/knowledge promotion 手动路径。
* [ ] 相关 build/type-check/tests 通过。

## 非目标

* Runtime interception。
* 多 Agent 调度。
* Worktree 自动隔离。
* CI gate。
* Marketplace。

## 技术备注

实现前需要先研究现有：

* `src-tauri/src/vibehub/agent_view.rs`
* `.vibehub/adapters/` 初始化结构
* 当前项目中 AGENTS/CLAUDE/OpenCode 相关生成逻辑（用 `rg` 定位）

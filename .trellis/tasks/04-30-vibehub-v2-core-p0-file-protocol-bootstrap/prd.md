# VibeHub v2 Core P0 File Protocol Bootstrap

## 目标

补齐 VibeHub v2 Core 的 P0 文件协议闭环，让一个本地项目能从 UI/后端真实完成 init -> start task -> build context -> agent-view -> review/handoff。

## 需求

* 初始化 `.vibehub` 可从 UI 触发，保持幂等，不覆盖已有文件。
* 新增或补齐 Start Task 后端能力：
  * 创建最小 task/run 目录。
  * 写入 `.vibehub/tasks/current`。
  * 写入 `.vibehub/tasks/{task_id}/runs/current`。
  * 更新 `.vibehub/state.yaml` 的 current/mode/phase/status/context/handoff 基础字段。
  * seed context spec，让 Build Context 不需要用户手工造文件。
* Build Context 使用现有 `context.rs`，从 active task/run/phase 构建 pack 和 manifest。
* Continue 生成 agent-view 时能读取 active pointer/state/context。
* Review/Handoff 能在 Golden Path 后工作。
* Recover 文案在 P0 内不得暗示 rollback/revert。

## 验收标准

* [ ] 后端 Golden Path 测试覆盖 init -> start task -> build context -> agent-view -> review evidence -> handoff。
* [ ] Start Task 后 current task/run pointer 是合法 YAML，且能被 `current.rs` resolve。
* [ ] Build Context 后生成 context pack 和 manifest。
* [ ] agent-view/current.md 和 current-context.md 包含当前 task/run/phase/context 信息。
* [ ] review evidence 和 handoff 文件能生成。
* [ ] 相关 Rust tests 通过。

## 非目标

* 完整 P1 中台布局。
* Runtime observation。
* 多 Agent 调度。
* 破坏性 recovery。

## 技术备注

重点文件：

* `src-tauri/src/vibehub/init.rs`
* `src-tauri/src/vibehub/current.rs`
* `src-tauri/src/vibehub/context.rs`
* `src-tauri/src/vibehub/agent_view.rs`
* `src-tauri/src/vibehub/review.rs`
* `src-tauri/src/vibehub/handoff.rs`
* `src-tauri/src/vibehub/status.rs`
* `src-tauri/src/commands.rs`
* `src/services/tauri.ts`

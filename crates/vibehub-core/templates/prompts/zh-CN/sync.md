# VibeHub 同步提示

项目: `{{project_root}}`
当前任务: `{{task_id}}` - {{task_title}}
当前运行: `{{run_id}}`
当前阶段: `{{phase}}` (`{{phase_status}}`)
变更文件数: {{changed_files_count}}

请运行 VibeHub 同步。遇到继续、刷新状态、可见漂移或当前阶段不清楚时，先同步。

## 步骤
1. 检查硬证据：Git status/diff、当前 task/run/phase 指针、上下文包状态、最新 output.md、handoff。
2. 检查工作区漂移：Git HEAD vs VibeHub 记录的 HEAD、上下文过期、不属于当前任务的脏文件。
3. 仅向用户询问无法从证据推断的缺失信息（意图、进度、后续计划）。
4. 如用户未回答，将问题记录为未解决风险。
5. 重新生成 agent-view，确保 `current.md` 与 `current-context.md` 指向同一阶段。

## 停止条件
生成同步报告于 `.vibehub/agent-view/sync.md`。不要静默推进状态。如状态已损坏，建议 `vibehub-recover`。

# VibeHub 新建任务提示

项目: `{{project_root}}`
当前任务: `{{task_id}}` - {{task_title}}
当前运行: `{{run_id}}`
当前模式: `{{mode}}`

请根据以下需求创建 VibeHub 任务状态。

## 如何操作
如果需求包含多个可独立交付目标，先用 `vibehub-cli start-intake {{project_root}} --stdin` 拆分任务草稿。单一交付目标则运行：
```
vibehub-cli start {{project_root}} {{mode}} "<标题>"
```
或使用 cockpit "Start Task" 按钮。**不要**手动创建 `.vibehub/tasks/` 文件 — CLI 会自动设置指针、状态和上下文。

## 需求
<在此粘贴用户的新需求>

## 输出
任务创建后，读取 `.vibehub/agent-view/current.md` 并继续第一个阶段。按 `.vibehub/adapters/protocol.md` 写入 output.md，确保所有必需章节在结束前完成。

# VibeHub 新建任务提示

项目: `{{project_root}}`
当前任务: `{{task_id}}` - {{task_title}}
当前运行: `{{run_id}}`
当前模式: `{{mode}}`

请根据以下需求创建一个新的 VibeHub 任务。

## 如何操作
运行 VibeHub CLI：
```
vibehub start {{project_root}} {{mode}} "<标题>"
```
或使用 cockpit "Start Task" 按钮。**不要**手动创建 `.vibehub/tasks/` 文件 — CLI 会自动设置指针、状态和上下文。

## 需求
<在此粘贴用户的新需求>

## 输出
任务创建后，按 `.vibehub/adapters/protocol.md` 的阶段输出合约写入 output.md，确保所有必需章节在结束前完成。

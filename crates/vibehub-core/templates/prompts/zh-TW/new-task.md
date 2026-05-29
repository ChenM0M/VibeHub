# VibeHub 新建任務提示

專案: `{{project_root}}`
目前任務: `{{task_id}}` - {{task_title}}
目前執行: `{{run_id}}`
目前模式: `{{mode}}`

請根據以下需求建立一個新的 VibeHub 任務。

## 如何操作
執行 VibeHub CLI：
```
vibehub start {{project_root}} {{mode}} "<標題>"
```
或使用 cockpit "Start Task" 按鈕。**不要**手動建立 `.vibehub/tasks/` 檔案 — CLI 會自動設定指標、狀態和上下文。

## 需求
<在此貼上使用者的新需求>

## 輸出
任務建立後，按照 `.vibehub/adapters/protocol.md` 的階段輸出合約寫入 output.md，確保所有必要章節在結束前完成。

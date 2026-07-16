# VibeHub 同步提示

專案: `{{project_root}}`
目前任務: `{{task_id}}` - {{task_title}}
目前執行: `{{run_id}}`
目前階段: `{{phase}}` (`{{phase_status}}`)
變更檔案數: {{changed_files_count}}

請執行 VibeHub 同步。遇到繼續、刷新狀態、可見漂移或目前階段不清楚時，先同步。

## 步驟
1. 檢查硬證據：Git status/diff、目前 task/run/phase 指標、上下文包狀態、最新 output.md、handoff。
2. 檢查工作區漂移：Git HEAD vs VibeHub 記錄的 HEAD、上下文過期、不屬於目前任務的髒檔案。
3. 僅向使用者詢問無法從證據推斷的缺失資訊（意圖、進度、後續計畫）。
4. 如使用者未回答，將問題記錄為未解決風險。
5. 重新產生 agent-view，確保 `current.md` 與 `current-context.md` 指向同一階段。

## 停止條件
產生同步報告於 `.vibehub/agent-view/sync.md`。不要靜默推進狀態。如狀態已損壞，建議 `vibehub-recover`。

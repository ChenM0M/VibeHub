# VibeHub v2 Core P1 Project Cockpit Center

## 目标

把 VibeHub v2 从“右键菜单里的简单弹窗”升级为“点击项目后的详情页式中台”。项目列表负责选择项目，中台负责展示和驱动 VibeHub v2 的核心工作流。

## 需求

* 点击项目后进入/打开项目详情页式 VibeHub 中台。
* 中台至少包含：
  * 项目概览。
  * 当前 task/run。
  * 4+1 flow 概览。
  * 当前 phase/status。
  * Git 状态。
  * Context Pack 状态。
  * Review Evidence 状态。
  * Handoff 状态。
  * Observability 边界说明。
  * 动作区：Initialize、Start Task、Build Context、Continue、Review、Build Handoff/Recover Report。
* 右键菜单可以保留为快捷入口，但不能是主要路径。
* UI 文案必须区分已实现能力和未来能力。
* 不使用营销页式 hero；这是工作台界面，应密集、清晰、适合反复操作。

## 验收标准

* [ ] 点击项目进入 VibeHub 中台。
* [ ] 未初始化项目能在中台看到 Initialize。
* [ ] 已初始化项目能在中台看到当前 VibeHub 状态。
* [ ] P0 actions 全部能从中台触发或显示明确不可用原因。
* [ ] Recover 相关 UI 不暗示未实现的 rollback/revert。
* [ ] 前端 build/type-check 通过。

## 非目标

* 完整 P3 runtime observation。
* 高级 loop warning 算法。
* 完整 Research Viewer。
* 多 Agent work planner。

## 技术备注

重点文件预计包括：

* `src/components/VibehubCockpitDialog.tsx`
* `src/services/tauri.ts`
* 项目列表/项目卡片相关组件（实现前需用 `rg` 定位）
* `src/types` 相关类型定义

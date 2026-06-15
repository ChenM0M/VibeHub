# Context Pack: Implement

Task: T-20260531153015-3a283c0b
Run: R-20260531153015-bd42f253
Phase: Implement
Generated at: 2026-06-15T06:53:26Z
Source commit: b6abdf3

## Instructions

Use this context only for the current phase.
Do not mark state.yaml completed.
Report files read, commands run, decisions made, and unresolved risks.

## Capability Output Schema

```json
{
  "required_fields": [
    "diff_summary",
    "changed_files",
    "commands_run"
  ],
  "optional_fields": [
    "rollback_plan",
    "references"
  ],
  "produces": [
    "diff"
  ],
  "consumes": [
    "implementation_plan"
  ],
  "parallel_safe": false,
  "custom": false
}
```

## Prior Outputs Summary

```json
[
  {
    "capability": "implement",
    "completed": [
      "`user_confirmed`: 用户重新确认项目页定位是“项目状态中台 / project command map”，不是删减成极简详情页；目标是保留中台能力，但重排信息层级、阅读路径和视觉语法。",
      "`hard_observed`: 重建 `src/components/ProjectDetailBoard.tsx`，主视图改为“项目身份 + 最近提交/动态 + active task 工作地图 + 右侧健康/下一步建议 + 下方结构/归档”的信息架构。",
      "`hard_observed`: 重建 `src/components/ProjectStructureExplorer.tsx`，从伪脑图/拖拽缩放改为模块索引、目录树、选中对象详情三栏联动阅读器。",
      "`hard_observed`: 更新 `src/components/VibehubCockpitDialog.tsx` 接入新的主视图和结构浏览器，并将全局推荐命令/prompt grid 限制到 settings 详情内，避免每个详情抽屉重复显示全局工具区。",
      "`hard_observed`: 第二轮重构 `TaskDetailContent`：从“当前 run 小卡 + task 卡片堆叠”改为左侧 active task 索引、右侧选中任务标题/状态/过程/元数据/关联/警告的对象详情页。",
      "`hard_observed`: 第二轮重构 `StatusTabContent` / `FlowDetailPanel`：phase/process 详情改为顶部状态结论、输入/输出 artifact 行表、phase 文件关系和验证状态，移除阶段页内重复的 recommended command/prompt 区。",
      "`hard_observed`: 第二轮重构 `ActivityDetailContent`：活动页改为左侧筛选时间线、右侧选中事件目标和 payload；移除默认嵌入的 package/file preview，使 artifact preview 回到显式预览入口。",
      "`hard_observed`: 第二轮重构 `ArchiveDetailContent`：归档页改为归档索引 + 历史详情，保留流程、产物、事件时间线，但用分隔线/行列表替代多层卡片。",
      "`hard_observed`: 调整 detail drawer 宽度策略：task / phase / activity / archive / git 使用更宽的对象详情宽度，structure 保持宽视图，其余维护面板保持窄视图。",
      "`user_confirmed`: 用户指出“从主视图点击具体任务后，详情左栏不应再次显示所有任务，而应显示当前任务内部已经经过的环节、流程、阶段，并能查看阶段传递数据”。",
      "`hard_observed`: 修正任务详情交互模型：`ProjectDetailBoard` 打开 task/phase detail 时携带 `taskId` 和 `phase` 上下文，`DetailDrawer` 保存 `selectedDetailTaskId`，不再丢失被点击任务。",
      "`hard_observed`: 修正 `TaskDetailContent`：左栏从全项目 active task 索引改为当前任务内部流程/阶段列表，右侧显示当前任务标题、过程条、选中阶段输入/输出包体、阶段文件关系、任务元数据和关联信息。",
      "`hard_observed`: 修正 `StatusTabContent`：phase detail 同样带入当前 task 上下文，左栏显示该任务内部流程，右侧显示选中 phase/capability 的传递数据和验证状态。",
      "`hard_observed`: 更新 `src/locales/en.json`、`src/locales/zh.json`、`src/locales/zh-TW.json`，补齐新主视图和第二轮详情抽屉文案。",
      "`user_confirmed`: 用户进一步判断当前“任务详情左侧阶段列表 + 右侧信息堆叠”仍然怪异，并提出可参考 Datadog-like service map / canvas 效果，用一个 canvas 展示单个任务的全流程历史，其它项目中台主体保持现状。",
      "`agent_reported`: 已把下一轮方向收敛为“Task lifecycle canvas”：项目状态中台仍负责多任务总览；点击具体任务后进入该任务自己的流程/数据流画布；右侧 inspector 展示选中节点的阶段、包体、文件、事件、验证和风险。",
      "`hard_observed`: `package.json` 未直接声明 React Flow / Konva / ELK 等流程图库；`recharts` 是直接依赖，`d3-*` 出现在 lockfile 的间接依赖中。下一轮若不新增依赖，优先用 React + SVG/HTML absolute layout 做轻量只读 canvas。",
      "`hard_observed`: 当前 `TaskDetailContent` 和 `StatusTabContent` 仍在 `src/components/VibehubCockpitDialog.tsx` 内部实现，二者都有“左侧流程列表 + 右侧详情”的重复结构，可由新的共享 `TaskLifecycleCanvas` 取代。"
    ],
    "full_ref": ".vibehub/tasks/T-20260531153015-3a283c0b/runs/R-20260531153015-bd42f253/outputs/output.md",
    "key_decisions": [
      "`user_confirmed`: 中台能力不砍掉；问题不是信息多，而是所有信息同等重量堆叠导致不可读。",
      "`agent_reported`: 主页面以 active task 为主语，旧 dashboard 型信息改为摘要、入口或对象详情，不再平铺成多组大卡片。",
      "`agent_reported`: detail surface 应按被点击对象组织：task 看任务，phase/process 看流转与包体，activity 看事件，archive 看历史，不再把 prompt、preview、状态卡全部混在同一个抽屉里。",
      "`user_confirmed`: task detail 的左侧导航属于“当前任务内部”，不是另一个全局任务菜单；全局多任务选择只应发生在项目状态中台主视图。",
      "`user_confirmed`: 可以把具体任务详情改成类似 service map 的 canvas：核心不是装饰性连线，而是用一张图展示任务内阶段流转、上下文传递、输出/handoff/review/validation 的历史关系。",
      "`agent_reported`: Project detail 主视图不应被再次推翻；这次改造聚焦“从项目中台点进一个任务后的内部详情形态”。",
      "`agent_reported`: phase/detail 不应成为另一套孤立详情页；从 phase pill 打开时应复用同一个 task lifecycle canvas，并自动聚焦对应 phase 节点。",
      "`agent_reported`: Canvas 必须只读，不直接触发 workflow 状态变更；允许沿用既有的预览、打开、定位文件等安全动作入口。",
      "`agent_reported`: 下一轮首选轻量自研 canvas，不新增流程图库；只有当自研布局明显不足时，再请求用户批准引入 React Flow / @xyflow/react 等依赖。",
      "`agent_reported`: 视觉语法改用列表、分隔线、状态点、紧凑指标和右侧 rail，减少圆角玻璃卡片、框中框和重复 prompt 区域。",
      "`agent_reported`: 项目结构能力保留，但首版以可信的文件系统扫描阅读器为主，不默认展示伪交互脑图。"
    ]
  }
]
```

## Neighbors

```json
[
  {
    "task_id": "T-20260609092907-7c439d16",
    "title": "Harden VibeHub agent protocol and CLI routing",
    "active_capabilities": [
      "implement"
    ],
    "shared_files": []
  }
]
```

## File: .vibehub/tasks/T-20260531153015-3a283c0b/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: T-20260531153015-3a283c0b
title: Redesign VibeHub project detail UI and project structure explorer
mode: guided_drive
phase: implement
phase_status: active
created_at: 2026-05-31T15:30:15Z
created_by: vibehub
```

## File: .vibehub/tasks/T-20260531153015-3a283c0b/runs/R-20260531153015-bd42f253/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: T-20260531153015-3a283c0b
run_id: R-20260531153015-bd42f253
mode: guided_drive
phase: implement
phase_status: active
created_at: 2026-05-31T15:30:15Z
created_by: vibehub
baseline_commit: null
```

## File: .vibehub/rules/hard-rules.md

Reason: protocol hard rules

```text
# VibeHub Hard Rules

- Agent output is reported state only.
- Only VibeHub code updates canonical state transitions.
- Do not mark state.yaml completed from agent output.
- Distinguish hard_observed, agent_reported, inferred, and user_confirmed evidence.
- P0/P1 observability is best-effort and must not claim full runtime observation.
- Agents should read agent-view files and the current context pack, not the whole .vibehub directory.
- Keep changes scoped to the active task.

## CI/CD 改动纪律 (2026-05-21 从 8 轮返工中总结)

### 1. 本地先跑通再改 CI
- 任何 macOS CI 构建改动，**先在本地 macOS 验证**：
  `npm run tauri -- build --target aarch64-apple-darwin --bundles app`
- 本地能成功 `hdiutil create -fs APFS`，再改 GitHub Actions。
- CI 不是调试器，不要拿它当测试环境用。

### 2. 每次只改一个变量
- CI workflow 单次改动只改一项：构建方式 / 签名方式 / DMG 方式 分开验证。
- 改多个变量时无法定位失败原因，导致穷举试错。

### 3. 签名方案先问"要不要"
- **prerelease / 预发布**: 用 ad-hoc 签名 (`APPLE_SIGNING_IDENTITY="-"`)，不走 notarization。
- **正式发布**: 才需要 Developer ID 证书 + 公证流程。
- 不要默认启用全套 Apple 签名，先确认是否必要。

### 4. 优先用直接命令，少用 Action 封装
- `npm run tauri -- build` 直接 shell 命令 > `tauri-action` GitHub Action。
- 直接命令可以在本地完美复现，action 的传参行为是黑盒。
- 必须用 action 时，先查源码理解其内部命令拼接逻辑。
```

## Known Missing Context

- None declared.

## Stop Condition

Write output.md and return to VibeHub for validation.

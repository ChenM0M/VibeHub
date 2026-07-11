# M1 Align Output — 基于 fixtures 构建 V3 高保真前端体验

Task: T-20260711080143-49b5012c
Run: R-20260711080143-b14d3ac8
Phase: align (guided_drive)
Generated: 2026-07-11

---

## Intent

`user_confirmed` + `hard_observed`

用 M0 已冻结的 `1.0` view contracts 和 12 场景确定性 fixtures，构建 V3
Project 与 Task 双层高保真前端体验原型。M1 是 Experience-first 里程碑：
让产品可见、信息架构可验证、契约字段可走查，**但不接真实 v3 core/MCP**。

UI 层级固定为 `Project → Task → PlanNode → Session → Event`（来自 v3 改版
计划 §3.6）。Project 视图回答"这个系统是什么、怎么组成、现在整体怎样"；
Task 视图回答"这次为什么改、谁做了什么、何时做、是否验收"。

---

## Scope

`hard_observed`：scope 依据 contracts/v3/README.md 字段映射、docs/vibehub-v3-redesign-plan.md §3.6 UI 信息架构、§4 M1 里程碑定义、M0 Task Pack M1 Handoff Contract。M1 只做前端体验原型，不接真实 core/MCP。

### In Scope

`hard_observed`（依据 contracts/v3/README.md、docs/vibehub-v3-redesign-plan.md §3.6、M0 Task Pack M1 Handoff Contract）

**A. Project 全局视图（跨 Task、长期容器）**

1. **项目总览 (ProjectOverviewView)**：技术栈、入口、构建/测试方式、主要
   模块、健康度、活跃 Task 与风险、protocol coverage。
2. **架构地图 (ProjectStructureView)**：模块/包/服务/数据流关系；区分
   "声明事实、静态分析、agent 推断"并显示证据与置信度。
3. **结构浏览器 (ProjectStructureView)**：完整可按需展开的目录树、搜索、
   Git 变更叠层、生成/第三方文件过滤；点击模块/文件显示职责、依赖、
   被依赖、关键 symbol、相关 Task/决策/commit、最近变化。
4. **全局时间图 (TaskTimelineView 聚合)**：把所有 Task、release、重要架构
   决策按时间排列，可筛选模块/agent/分支。

**B. Task 视图（单次需求与追溯单位）**

5. **时间线 (TaskTimelineView)**：用户需求原话 → 决策 → commit/diff →
   测试 → 风险 → 恢复点；事件按 session/node lane 排列。
6. **计划图 (PlanGraphView)**：DAG、计划 vs 实际编排、节点状态、scope 与
   依赖；待执行节点展示 goal/scope/non_scope/dependencies。
7. **验收进度 (TaskTimelineView criteria + NodeBrief criteria)**：
   Criterion checklist，逐项 pass/fail/证据。
8. **会话泳道 (TaskTimelineView lanes)**：Codex/OpenCode/Claude 等
   session 在时间轴上的并行区间、占用范围与交接关系。
9. **节点简报 (NodeBrief)**：进入 Task/Node 后的聚焦上下文：goal、scope、
   accepted decisions、research summary、files、validation commands、
   budget、source versions、protocol coverage。

**C. 状态覆盖（全部 12 fixture 场景）**

10. loading / empty / stale / error / partial / large-data 状态的完整
    渲染——每个屏幕都能从 fixture 的 `freshness`、`completeness`、
    `warnings`、`errors` 字段驱动正确状态，不出现空白或假数据。
11. review finding / rework attempt 的展示（FX-REWORK 两轮 attempt）。
12. protocol coverage 状态展示（FX-COVERAGE-GAP 的 gap 与 recovery）。

**D. 深入动作（mock 交互）**

13. 点击模块/文件显示证据化详情；打通现有 reveal/open。
14. 新增"首选 IDE 打开/定位"的 mock 交互（不实际启动 IDE）。
15. 新增"询问项目"启动流程的 mock 交互（不实际拉起 agent 会话）。

**E. 契约反推**

16. 通过交互原型反推缺失字段；任何契约 gap 回到 M0 作为 versioned delta，
    不在 M1 内重新定义 wire contract。

### Out Of Scope (non_goals)

`hard_observed`（依据 M0 Task Pack、RFC backlog、v3 改版计划 §4 里程碑边界）

- **不接真实 v3 core/MCP**：不实现事件存储、Application Service、MCP
  server、CLI adapter、跨进程锁或真实 projection。
- **不读取 V2 YAML**：UI 只依赖 M0 view contracts 的形状，不直接读取
  `state.yaml` / `task.yaml` / `run.yaml` 等 V2 快照。
- **不写 canonical 项目状态**：UI 不触发 `vibehub_start_task`、
  `vibehub_complete_phase`、`vibehub_advance_phase` 等 Tauri command
  （这是 v3 改版计划 §1.1 不变量，M4 才删除旧 UI 写入路径）。
- **不实现 Project Intelligence 扫描器**（M3）：M1 用 fixture 数据展示
  架构地图，不实现文件树扫描、manifest 解析、增量索引。
- **不实现 PlanGraph 执行/编排**（M4）：M1 展示计划图状态和节点，
  不实现图变更事件、Finding→remediation 循环、worktree 创建。
- **不实现 worktree 编排**（M5）：fixture 中的 worktree 数据只做展示。
- **不重新定义 wire contract**：M1 component props 可以 narrow 一个 view
  但不能 redefine wire contract（M0 Task Pack 规则 5）。
- **不做 native file-operation 验证**：M0 已标记 `not_tested`，M1 在
  fixture 层验证路径显示，不声称 native 文件操作能力。
- **不做发布硬化**（M6）：M1 不处理安装/升级/卸载。

---

## Success Criteria (acceptance_criteria)

`hard_observed`（依据 task.yaml + v3 改版计划 §4 M1 验收 + M0 Task Pack M1 Handoff Contract）

### M1-C01 状态全覆盖

`hard_observed`（依据 task.yaml acceptance_criteria 第 1 条 + v3 改版计划 §4 M1 验收）

Project Center 与 Task 体验覆盖主流程和 loading/empty/stale/error/partial/
large-data 状态。12 个 fixture 场景（FX-EMPTY through FX-COVERAGE-GAP）
每个都能加载并正确渲染五类 view，不出现空白 UI 或假数据。

验证方式：为每个 fixture 场景渲染快照或交互走查；状态由 contract 的
`freshness` / `completeness` / `warnings` / `errors` 字段驱动。

### M1-C02 字段可溯源

`hard_observed`（依据 task.yaml acceptance_criteria 第 2 条 + contracts/v3/README.md Field Map）

每个屏幕字段均映射到 M0 contract 和 evidence source，无神秘指标。
contracts/v3/README.md 的 Field Map 是权威映射；UI 不展示 contract 中
不存在的字段，也不隐藏 contract 中存在的字段。

验证方式：逐屏幕核对字段 ↔ contract 字段 ↔ evidence provenance；
contracts/v3/README.md §Top-Level Field Map 作为 checklist。

### M1-C03 跨平台无溢出

`hard_observed`（依据 task.yaml acceptance_criteria 第 3 条 + M0 Task Pack FX-WIN-PATHS/FX-MAC-PATHS/FX-LARGE）

Windows/macOS viewport、长路径与大数据 fixture 无溢出重叠。
FX-WIN-PATHS（drive/UNC/extended/long/case-collision）和
FX-MAC-PATHS（spaces/Unicode/symlink/bundle）在紧凑视口下不截断、
不溢出、不重叠。FX-LARGE 的大树/时间线/图在桌面最小视口可滚动/分页。

验证方式：至少在 macOS 上测试紧凑视口（如 1024×600 或更小）；
Windows viewport 用 fixture 数据模拟（M1 不要求 Windows 物理机验证，
但布局必须为 Windows 路径长度和分隔符留余量）。

### M1-C04 可用性走查

`hard_observed`（依据 task.yaml acceptance_criteria 第 4 条 + v3 改版计划 §4 M1 验收第 1 条）

完成"理解项目 → 发现活跃风险 → 进入 Task → 查看依据 → IDE 深入"
的可用性走查。项目所有者仅看原型即可完成这条任务链。

验证方式：用 FX-HAPPY 场景做主流程走查；用 FX-STALE/FX-PARTIAL/
FX-ERROR 做降级状态走查；用 FX-REWORK 做 review finding 走查；
用 FX-PARALLEL 做多 session 泳道走查。

### M1-C05 契约边界守恒

`hard_observed`（依据 M0 Task Pack M1 Handoff Contract + M0-C07 consumer proof + v3 改版计划 §1.1 不变量）

UI 只依赖 M0 view contracts（`src/v3/contracts/generated/*` types +
`createV3FixtureRepository`），不 import V2 YAML 类型、不调用 V2 Tauri
command、不 import `vibehub-core`。Contract gap 回到 M0 作为 versioned
delta，不在 M1 内补。

验证方式：forbidden-dependency scan（扩展 M0 的
`npm run v3:contracts:check` 已有的边界检查到 M1 前端代码）；
`npm run build` 通过。

---

## Autonomy Level

`agent_reported`

**Autonomy: guided_drive — agent 在 plan 批准后自主 implement，但关键
信息架构决策需用户确认。**

- Agent 可以自主选择组件拆分、状态管理方式、样式方案。
- Agent 可以自主决定 fixture loader 的实现方式（fetch / import / Vite plugin）。
- Agent **不可**自主重新定义 wire contract；发现 gap 时记录为 M0 delta
  候选，等用户确认后回传 M0。
- Agent **不可**自主决定删除或替换 V2 production UI 模块（M4 scope）。
- Agent **不可**自主运行 `vibehub finish` 或 `vibehub advance`。

---

## Risk Level

`inferred`

**Medium** — M1 是体验原型，不触及 canonical 状态，技术风险可控。主要风险：

1. **契约 gap 发现循环**：M1 交互原型可能发现 M0 contract 字段不足，
   需要 versioned delta 回 M0。这是预期行为（M0-C12 明确预留），但
   可能导致 M1 plan 阶段需要并行处理 M0 delta。缓解：gap 先记录，
   不阻塞 M1 实现；用 fixture 内现有字段展示降级状态。
2. **V2 共存边界**：M1 新 UI 与 V2 现有 UI（VibehubCockpitDialog 等）
   共存于同一 React app。需要明确 M1 是新路由/新页面，不修改 V2
   组件。缓解：M1 代码放在 `src/v3/` 下，独立路由。
3. **大 fixture 性能**：FX-LARGE 的树/图/时间线在浏览器中可能卡顿。
   缓解：虚拟滚动、按需加载、分页（contract 的 `page` 字段已预留）。
4. **跨平台视口验证不完整**：M1 在 macOS 上开发，Windows 视口只能
   用 fixture 模拟。缓解：用 CSS 媒体查询和 fixture 路径数据覆盖
   紧凑视口；M6 做真实 Windows 验证。

---

## Research Decision

`inferred`

M1 是 guided_drive 模式，research 阶段被跳过（`align → plan → implement
→ review`）。M0 已完成的 Research Pack 和五份 RFC backlog 提供
足够上下文。如 plan 阶段发现需要额外调查（如 UI 框架选择、虚拟滚动
方案），在 plan 阶段内做局部 spike，不升级为 evidence_drive。

---

## Completed

- `hard_observed`: 读取 M0 全部交付物并确认 M0 已完成（phase: review,
  status: completed）：5 个 JSON Schema 2020-12 contracts、12 场景
  fixtures、generated TypeScript types、`createV3FixtureRepository`
  接口、`npm run v3:contracts:check`（285 assertions / 12 scenarios /
  5 views 通过）。
- `hard_observed`: 确认 M1 task 为当前指针
  (`T-20260711080143-49b5012c`)，phase=align (active)，M0
  (`T-20260711080143-c3669d9d`) 已 completed。
- `hard_observed`: 读取 v3 改版计划 §3.6 UI 信息架构、§4 M1 里程碑
  定义、M0 Task Pack M1 Handoff Contract，确认 M1 scope 与验收边界。
- `hard_observed`: 读取现有前端结构（src/，11,943 行 TS/TSX，
  VibehubCockpitDialog 4,268 行等），确认 M1 需新建 V3 UI 而非
  扩展 V2 组件。
- `hard_observed`: 确认 fixtureRepository.ts 的
  `V3ViewRepository` 接口（`listScenarios()` + `loadScenario()`）
  是 M1 的唯一数据入口；`V3FixtureBundle` 包含五类 view。
- `hard_observed`: 读取 ProjectOverviewView generated type 和
  FX-HAPPY fixture，确认 contract → TypeScript → fixture 链路完整。
- `agent_reported`: 产出 align 阶段 output.md，包含 intent、scope、
  success_criteria、non_goals、autonomy_level、risk_level。

## Not Yet Done

- `inferred`: M1 plan 阶段（implementation_plan、validation_plan、
  context_plan）——需要用户确认 align 后 advance。
- `inferred`: M1 implement 阶段（实际 UI 组件、路由、fixture loader、
  状态管理、样式）。
- `inferred`: M1 review 阶段（可用性走查、跨平台视口验证、字段溯源
  checklist、forbidden-dependency scan）。
- `inferred`: M0 contract delta 候选清单（M1 交互原型中发现 gap 后
  回传 M0）。

## Key Decisions Made

- `agent_reported`: M1 UI 代码放在 `src/v3/` 下，与 V2 UI（`src/components/`、
  `src/pages/`）隔离。M1 是新路由/新页面，不修改 V2 组件。理由：v3 改版
  计划明确 M4 才删除旧 UI 写入路径；M1 只加不减。
- `agent_reported`: M1 数据入口是 `createV3FixtureRepository`，通过
  `V3ViewRepository.loadScenario()` 加载 fixture JSON。fixture 路径
  默认 `/fixtures/v3`，在 Vite dev/build 中通过静态资源或 import
  加载（plan 阶段确定具体方式）。
- `agent_reported`: Contract gap 不在 M1 内补。发现 gap 时记录为
  M0 delta 候选，在 output.md 的 Warnings 或 Key Decisions 中列出，
  等用户确认后回传 M0 作为 versioned change。理由：M0 Task Pack
  M1 Handoff Contract 明确要求。
- `agent_reported`: M1 验收用 fixture 场景映射：FX-HAPPY 做主流程，
  FX-EMPTY 做空状态，FX-STALE 做过期，FX-PARTIAL 做部分支持，
  FX-ERROR 做错误，FX-LARGE 做大数据，FX-REWORK 做 review finding，
  FX-PARALLEL 做多 session，FX-WIN-PATHS/FX-MAC-PATHS 做跨平台路径，
  FX-NO-DOCS 做无架构文档，FX-COVERAGE-GAP 做 protocol coverage。

## Files Changed

- `agent_reported`: `.vibehub/tasks/T-20260711080143-49b5012c/runs/R-20260711080143-b14d3ac8/outputs/output.md`（新建，本文件）

## Files Reportedly Read

- `hard_observed`: `.vibehub/agent-view/current.md`
- `hard_observed`: `.vibehub/agent-view/current-context.md`
- `hard_observed`: `.vibehub/agent-view/handoff.md`
- `hard_observed`: `.vibehub/rules/hard-rules.md`
- `hard_observed`: `.vibehub/tasks/T-20260711080143-49b5012c/task.yaml`
- `hard_observed`: `.vibehub/tasks/T-20260711080143-49b5012c/runs/R-20260711080143-b14d3ac8/context-packs/align.md`
- `hard_observed`: `.vibehub/tasks/T-20260711080143-c3669d9d/task.yaml`（M0 任务元数据）
- `hard_observed`: `contracts/v3/README.md`（字段 ↔ UX ↔ evidence 映射）
- `hard_observed`: `docs/v3/m0-task-pack.md`（M0 Task Pack + M1 Handoff Contract）
- `hard_observed`: `docs/vibehub-v3-redesign-plan.md`（§3.6 UI 信息架构、§4 M1 里程碑）
- `hard_observed`: `fixtures/v3/manifest.json`（12 场景 → Criteria 映射）
- `hard_observed`: `fixtures/v3/FX-HAPPY/project-overview.json`（fixture 样例）
- `hard_observed`: `src/v3/contracts/fixtureRepository.ts`（V3ViewRepository 接口）
- `hard_observed`: `src/v3/contracts/index.ts`（导出入口）
- `hard_observed`: `src/v3/contracts/generated/project-overview-view.ts`（TypeScript 类型样例）
- `hard_observed`: `package.json`（脚本与依赖）
- `hard_observed`: `.vibehub/workflow.yaml`（align capability required_fields）
- `hard_observed`: `.vibehub/rules/phase-rules.yaml`（align phase required_outputs）

## Commands Run

- `hard_observed`: `git status --short` — 确认工作区状态（M0 交付物未提交，27 个变更文件）
- `hard_observed`: `git log --oneline -5` — 确认 HEAD 在 d7ece57
- `hard_observed`: `vibehub status .` — 确认当前 task=M1, phase=align(active), context pack available
- `hard_observed`: `vibehub validate .` — 确认 missing outputs: intent, acceptance_criteria, autonomy_level

## Tests Run

- `hard_observed`: 未运行测试。align 阶段不产生代码变更，无需测试。
  M0 的 `npm run v3:contracts:check` 已在 M0 review 中通过（285 assertions）。

## Context Still Needed

- `inferred`: M1 plan 阶段需要确定 fixture 加载方式（Vite import / fetch /
  static copy）。需要检查 Vite 配置和 `fixtures/v3/` 的公共路径可用性。
- `inferred`: M1 plan 阶段需要确定路由方案（当前前端无路由，用本地 state）。
  需要决定是否引入 react-router 或用 zustand state 模拟路由。
- `inferred`: M1 plan 阶段需要确定 UI 组件库使用范围（现有 Radix UI +
  Tailwind + shadcn/ui 组件在 `src/components/ui/` 下，M1 可复用）。
- `inferred`: 需要用户确认 align 产出后，才能 advance 到 plan 阶段。

## Warnings

- `hard_observed`: 工作区有 27 个未提交变更（M0 交付物：
  contracts/、fixtures/、src/v3/、docs/v3/ 等）。M0 已 completed 但
  文件未 commit。建议在 M1 plan 前先提交 M0 交付物，避免 M1 变更与
  M0 混淆。`user_confirmed` 需要用户决定是否现在提交。
- `inferred`: M1 是 guided_drive，跳过 research 阶段。如 plan 中发现
  需要 UI 技术调查（虚拟滚动、DAG 渲染、时间线库），可能需要局部 spike。
- `inferred`: V2 现有 UI（VibehubCockpitDialog 4,268 行）有 canonical
  写入按钮，违反 §1.1 不变量。M1 不修改它，但 M1 新 UI 必须不含
  canonical 写入。M4 才删除旧 UI 写入路径。

## Next Session Should

1. `user_confirmed`: 等用户确认 align 产出后，运行
   `vibehub advance . --confirmed-by-user` 进入 plan 阶段。
2. `agent_reported`: plan 阶段产出 implementation_plan（组件拆分、
   路由方案、fixture loader 方式、状态管理）、validation_plan
   （12 场景走查 checklist、跨平台视口验证、字段溯源 checklist、
   forbidden-dependency scan）、context_plan（M1 需要读取的文件）。
3. `agent_reported`: 考虑是否先提交 M0 交付物（27 个未提交文件），
   以保持 M1 变更的 git 历史清晰。
4. `agent_reported`: plan 阶段应按 Project 视图（4 个屏幕）和 Task 视图
  （4 个屏幕）拆分垂直薄切片，每个薄切片用 1-2 个 fixture 场景驱动。

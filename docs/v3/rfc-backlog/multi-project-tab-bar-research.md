# 多任务并行的顶部标签页（Tab Bar）调研与方案对齐

- 任务：`task.tab-bar.6fb15dcfa460`
- 状态：调研中，等待用户对齐；本文件不含实现代码改动
- 目标问题：用户通常多项目/多任务并行，现在每次切换都要「座舱 → 返回列表 → 点另一张项目卡 → 进入座舱」，上下文与已加载数据全部丢失

## 1. 现状盘点（阻塞点与硬约束，均带文件:行号）

### 1.1 导航层：没有路由，只有单槽 state
- `src/main.tsx:13,17,37-42,57-60`：页面级切换是 `PageType = 'home' | 'settings' | 'gateway' | 'about'` 的 `useState`，回 home 靠 `homeResetKey` 强制重置；**无 react-router、无 URL/history**。
- `src/pages/Home.tsx:65`：`const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null)` —— 「列表 vs 座舱」的唯一开关。
- `src/pages/Home.tsx:167-215`：`selectedProject` 命中就 `return <V3Cockpit .../>` 早返回，因此**全局只可能挂载一个座舱实例**。
- 返回列表三条路径都在重置同一份 state：`V3Cockpit.tsx:593`（返回箭头）→ `Home.tsx:209-212` `leaveV3Project(); setSelectedProjectId(null)`；`Sidebar.tsx:47-63,81-91` → `main.tsx:38-40` bump resetKey → `Home.tsx:69-72`；切换工作区（同一 effect 依赖 `selectedWorkspaceId`）。
- 项目卡入口：`src/components/ProjectCard.tsx:151-161,246-254`（内容区 `onClick → onSelect`，banner 是拖拽把手）经 `SortableProjectCard.tsx:48` 上报，`Home.tsx:277,309` `onSelect={() => setSelectedProjectId(project.id)}`。

### 1.2 状态层：v3Store 是「单槽 + 模块级单例 + 销毁式切换」
- `src/v3/stores/v3Store.ts:93,96,99,100`：`projectPath` / `bundle` / `selectedTaskId` / `selectedNodeId` 各只有一份。
- `v3Store.ts:50-62`：`productionLoader / legacyLoader / usageLoader / lifecycleApi / projectSettingsApi` 是**模块级单例**，另有 8 个单调 requestId 计数器用于失效旧响应。
- `v3Store.ts:215-246 selectProject`：bump 全部 8 个 requestId → 覆写 5 个 loader 单例 → 清空 `bundle/selectedTaskId/selectedNodeId/legacyArchive/usage/taskUsage/layoutStatus/lifecycle*/projectSettings/agentSpecs`，`currentView` 复位。`248-292 leaveProject` 同样。→ **打开项目 B 会彻底销毁项目 A 的已加载状态，没有任何缓存。**
- `v3Store.ts:355-398 loadCurrentBundle`：`364` 用上一次 `project_id` 作为 `expectedProjectId`，`370-372` 一旦返回不同项目直接 `throw V3_IDENTITY_MISMATCH`；后端同样守卫 `src-tauri/src/commands.rs:352-382`（不匹配报错在 `363-369`）。→ 多标签并存必须把这条「全局唯一项目」假设改成「按标签作用域」。
- `v3Store.ts:76-84,557-596`：已有 `currentView` + `navStack`（drillIn/goBack/goHome）但**座舱实际没用**，座舱用自己的本地 `activeTab`。这套死代码可以作为 tab 内部导航栈复用。

### 1.3 刷新与轮询绑定单实例
- `V3Cockpit.tsx:223-234`：`window.setInterval(4000)` 在座舱内部，条件 `sourceMode==='production' && projectPath && productionLoader && layoutStatus?.state==='v3'`，`document.visibilityState === 'hidden'` 时跳过，`refreshInFlight` 去重，卸载即清除。
- `V3Cockpit.tsx:242-248`：`selectedTaskId` 变化触发 `loadTaskUsage`；`655-660`：切换任务会再整包 `loadCurrentBundle()`。
- → 多标签必须回答：后台标签是否继续 4s 轮询（N 个项目 × 4s 的 IPC/CPU 成本），还是只前台轮询 + 切回时立即刷新。

### 1.4 窗口 chrome / 标题栏（决定标签栏能放哪）
- `src/components/Layout.tsx:16-33`：唯一的全窗口布局 = `Sidebar`（`w-64`）+ 右列 `Header`（`h-12`）+ `main`。`Layout` 目前只接收 `children/onSearch/currentPage/onNavigate/onCheckUpdate/isCheckingUpdate:7-14`，**完全不知道项目**。
- `src/components/Header.tsx:65,66,116-146`：`data-tauri-drag-region="deep"`、`data-platform`、**Windows 专属最小化/最大化/关闭按钮**。
- `src/components/Sidebar.tsx:76`：`{isMac && <div className="h-12 shrink-0" data-tauri-drag-region="deep" />}` —— macOS 红绿灯占位。
- `src-tauri/tauri.conf.json:23-25`：`decorations: true` + `titleBarStyle: "Overlay"` + `trafficLightPosition {x:16,y:18}` + `hiddenTitle`；`src-tauri/tauri.windows.conf.json:13`：`decorations: false`（所以 Windows 自绘按钮）。
- → 标签栏若上移到窗口顶端，必须同时处理 macOS 红绿灯让位与 Windows caption 按钮共存，且保留可拖拽区域（标签之间的空白必须是 drag region，否则窗口拖不动）。

### 1.5 契约与 i18n 硬门禁（重构必须满足）
- `scripts/v3-contracts/check.mjs:144-146` 读取 `Home.tsx` / `V3Cockpit.tsx` / `v3Store.ts` **源码文本**并做正则断言：例如要求 `initialSourceMode="production"` … `projectPath={selectedProject.path}` … `debugMode={V3_DEBUG_ENABLED}` 出现在同一 1200 字符窗口内、要求 `if (showV3Playground)` 分支存在、要求 `if (sourceMode === "production") … selectProject(`、并锁定 store 的 requestId 失效顺序。→ 任何多标签重构要么保持这些文本，要么**同批次更新契约脚本并说明理由**。
- `scripts/v3-i18n/check.mjs:14-30,42-47`：`v3.*` 键在 `zh/zh-TW/en` 必须叶子对齐，且 `src/v3/**/*.tsx` **出现任何汉字即失败**。→ 新标签 UI 必须全量走 i18n key。
- 技术栈事实：React 18.3.1、zustand 4.5.7（无 persist）、Tailwind 3.4、framer-motion 12（已用 `layoutId` 做 tab 下划线 `V3Cockpit.tsx:759-764`）、`@dnd-kit/*` 已在用（项目卡排序，可直接复用于标签拖拽）、Radix `Tabs` 已存在但只在 `Settings.tsx:178-192` 用。

### 1.6 已存在的「类标签」层级（避免概念叠加）
1. 侧边栏页面导航（`Sidebar.tsx:81-107`）。
2. 座舱内左侧任务列表（`V3Cockpit.tsx:636-747`，含归档抽屉）——**已经是「任务切换器」**。
3. 座舱内容区 4 个 tab：概要/时间线/实现计划/结构架构（`V3Cockpit.tsx:52-60,751-768`）。
→ 再加一层顶部标签，必须明确它与 2、3 的分工，否则会出现「三层 tab」的认知负担。

## 2. 主流做法对标

| 产品 | 标签粒度 | 溢出策略 | 拖拽/固定 | 会话恢复 | 后台标签成本 | 对 VibeHub 的启示 |
| --- | --- | --- | --- | --- | --- | --- |
| Chrome | 一个标签 = 一个独立文档；可拖出成窗口 | 等比压缩到最小宽度 + 溢出菜单（Chrome 已改为水平滚动+分组） | 拖拽排序、Pin（固定标签缩为图标并置左）、Tab Group | 崩溃后恢复；标签冻结（frozen/discarded） | 冻结/丢弃后台标签，回到前台再恢复 | 「压缩到最小宽度 + 固定标签置左」最贴近桌面直觉；后台冻结正好对应「后台不轮询」 |
| VS Code | 编辑器 tab（文件） + 顶部无项目 tab，项目=窗口 | 水平滚动 + `…` 溢出下拉 + 「预览 tab（斜体）」 | 拖拽、Pin、拖到另一个编辑器组 | `restoreWindows`、workspace 级恢复 | 后台编辑器保留内存模型，语言服务按活跃文件 | Pin + 预览 tab 的「临时打开不占位」机制很适合「点一下看看某个项目」 |
| Notion | 顶部标签 = 页面；`⌘+click` 新标签 | 等分收缩 + 滚动 | 拖拽排序、拖出为新窗口 | 重启恢复上次标签集 | 后台页面只保留缓存，不订阅实时更新 | 标签 = 「工作对象」而非「视图」；标签内保留各自的滚动/子 tab 状态 |
| Zed / Warp | Zed：项目=窗口，文件=tab；Warp：tab=终端会话，可分屏 | Zed 滚动；Warp 收缩+溢出 | 均支持拖拽、快捷键 `⌘1..9`、`⌘⇧[ / ]` | Warp 会话恢复 | Warp 后台 tab 仍在跑（进程语义） | Agent 在跑的任务天然是「进程语义」，后台仍需要状态变化提示（角标/动效），而不是完全冻结 |

**共识做法（几乎所有产品都有）**
1. `⌘/Ctrl+W` 关闭、`⌘/Ctrl+T`（或点 `+`）新建、`⌘1..9` 直达、`⌘⇧[ / ]` 或 `Ctrl+Tab` 前后切换。
2. 中键点击关闭；hover 才显示关闭按钮；活动标签视觉抬升 + 指示条（我们已有 framer-motion `layoutId` 方案）。
3. 溢出：先等比收缩到 min-width（约 120–180px），再横向滚动，最后给溢出下拉列表（含搜索）。
4. 标签自带「脏/活跃」状态徽标（VS Code 圆点、Chrome 播放图标）——对 VibeHub 就是「有 Agent 在跑 / 有 blocker / 待确认」的角标。
5. 最右侧留一块可拖拽空白（窗口拖动区），macOS 左侧留红绿灯位。

## 3. 候选方案

先要回答的三个正交问题：**标签粒度**、**标签栏位置**、**状态模型**。

### 3.1 标签粒度
- **A. 仅项目级（推荐）**：一个标签 = 一个项目座舱；任务切换仍用座舱左侧任务列表。层级最清晰（标签=项目、左列=任务、内容 tab=视图），改动面最小。
- **B. 项目 + 任务级（Chrome 式扁平）**：标签可以是「项目」或「项目内某任务」。更贴合「多任务并行」的字面需求，但会与座舱左列任务列表功能重叠，且同一项目多个任务标签共享同一个 bundle，需要「标签 → (projectPath, taskId)」双键作用域，复杂度显著上升。
- **B'（折中）**：标签仍是项目级，但标签标题显示「项目 · 当前任务名」，并支持「在新标签打开任务」→ 实际是同项目第二个标签，仅 `selectedTaskId` 不同（需要 per-tab task 选择而非 per-project）。

### 3.2 标签栏位置
- **P1. `Layout.tsx` 的 Header 下方独立一行（推荐，低风险）**：`Layout.tsx:26-27` 之间插入 `h-9~h-10` 的 tab 行，不动 macOS 红绿灯占位与 Windows caption 按钮，跨平台风险最低；代价是多占 ~36px 且和 Header 视觉上两条横线。
- **P2. 内嵌进 `Header.tsx`（h-12 那一行）**：最省空间、最像 Chrome/Notion，但要和搜索框、主题、通知、Windows 三按钮抢位；搜索框需收成图标，标签可用宽度在 900px 最小窗口下紧张。
- **P3. 融入窗口标题栏（真正的 Chrome 式）**：视觉最佳，但要同时改 `tauri.conf.json:23-25`（macOS overlay + trafficLightPosition）与 `tauri.windows.conf.json:13`（decorations:false 自绘），并重排 `Sidebar.tsx:76` 的红绿灯占位与 drag region；风险与验收成本（含 Windows 原生手测，见 `docs/v3/agent-release-process.md`）最高。
- 另可选 **P0. 侧边栏「已打开项目」区**（最低成本，但不是用户要的顶部标签）。

### 3.3 状态模型
- **S1. v3Store 改为 per-project map（推荐）**：`projects: Record<string, ProjectSlice>` + `activeProjectPath`，把 5 个 loader 单例与 8 个 requestId 一并搬进 slice；`V3_IDENTITY_MISMATCH`（`v3Store.ts:370-372`）由「全局唯一」改成「slice 内校验」。一次性改动大（`v3Store.ts:50-62,91-142,215-292` 全域），但之后行为最正确，且 `scripts/v3-contracts/check.mjs:174-183` 的断言需同批更新。
- **S2. 单槽 + LRU 快照缓存（渐进）**：保留现有单槽结构，`leaveProject/selectProject` 前把当前 slice 快照存入 `Map`（上限 3–5，LRU 淘汰），切回时先渲染快照再后台刷新。改动集中在 `selectProject/leaveProject` 两个函数，契约脚本大概率不需要改；代价是后台标签不做实时刷新（切回可能有短暂旧数据 + 一次刷新闪烁），且仍只有一个「活的」项目。
- **S3. 多实例挂载**：同时挂载多个 `V3Cockpit`（非活动的 `display:none` 保活）。DOM/内存和 N×4s 轮询成本最高，但保留滚动位置等隐式 UI 状态最好；建议只作为 S1 之上的可选优化。

### 3.4 组合推荐（分三阶段）
- **阶段 1（1 个迭代，风险低）**：粒度 A + 位置 P1 + 状态 S2。产出：`openTabs` 状态（提到 `appStore` 或新建 `tabsStore`，因为 `selectedProjectId` 现在困在 `Home.tsx:65`）、tab 行 UI（dnd-kit 拖拽、hover 关闭、中键关闭、`⌘1..9`/`⌘W`/`Ctrl+Tab`、溢出滚动+下拉）、快照缓存让切回不再从零加载。**只有前台标签轮询**，后台标签在切回时立即刷新一次。
- **阶段 2（中风险）**：状态升级到 S1 per-project slice，让后台标签也能低频（例如 20–30s）刷新并在标签上显示角标（Agent 运行中 / blocker / 待确认）；同批更新 `scripts/v3-contracts/check.mjs`。
- **阶段 3（可选）**：粒度 B'（同项目多任务标签）、标签持久化到 `config.json`（`src-tauri/src/storage.rs:20-48`）实现重启恢复、以及位置升级到 P3 的原生标题栏融合（需 Windows 原生验收）。

### 3.5 已与用户对齐的结论（2026-07-29）
- **粒度：仅项目级（A）**。用户原话：「就是像是浏览器打开的一个浏览页面一样子，只不过我们打开的就是具体的这个项目详细页的内容」→ 一个标签 = 一个项目详情页（座舱），任务切换仍在座舱左列。
- **位置：P1**，Header（h-12）下方独立一行，不动 macOS 红绿灯与 Windows caption 按钮。
- **后台刷新：S2 语义**，后台标签不轮询，切回时先渲染 LRU 快照再立即刷新一次；只有前台标签跑 4s 轮询。
- **持久化/关闭：会话内**（不写 config.json），关闭最后一个标签回项目列表；含 hover 关闭、中键关闭、`⌘W`/`⌘1..9`/`Ctrl+Tab`、dnd-kit 拖拽排序、溢出三级降级；暂不做 Pin 与重启恢复（留阶段 3）。
- → 落地范围即「阶段 1 = A + P1 + S2」；per-project slice（S1）、角标、Pin、重启恢复、标题栏融合均为后续独立任务。

### 3.6 待后续任务再定的开放问题
1. 标签粒度：A（仅项目）/ B（项目+任务）/ B'（项目级但可同项目开多标签）。
2. 标签栏位置：P1 Header 下独立一行 / P2 内嵌 Header / P3 融入标题栏。
3. 最大标签数与溢出：上限（如 8）？超限提示还是 LRU 自动关闭最旧？溢出用滚动还是下拉。
4. 持久化范围：仅会话内，还是写入 config.json 重启恢复（含「上次打开的标签集」）。
5. 关闭/固定语义：是否要 Pin；关闭最后一个标签是回项目列表还是保留空标签页；是否要 VS Code 式「预览标签」（单击临时、双击/编辑才固化）。
6. 后台标签刷新策略：完全不刷（切回再刷）/ 低频刷新以点亮角标 / 与前台同频。

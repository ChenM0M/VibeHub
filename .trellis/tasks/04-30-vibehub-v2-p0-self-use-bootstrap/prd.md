# VibeHub v2 Core

## 目标

把当前 VibeHub v2 spike 推进到“核心功能可正常自用”的状态：用户从项目列表点击一个项目后，进入项目详情页式的 VibeHub 中台，在这个中台里完成初始化、任务启动、上下文构建、Agent 入口、Review evidence、Handoff、基础恢复判断和核心可视化。

这个任务不是只补 P0，也不是一次性实现 r10 的所有远期能力。它的范围是 **VibeHub v2 Core**：

* **P0：File Protocol Bootstrap** - 跑通真实 task/run、context pack、agent-view、review/handoff 的文件协议闭环。
* **P1：Project Cockpit Center** - 把 VibeHub v2 做成点击项目后的中台式主体验，而不是右键菜单里的简单弹窗。
* **P2：Low-friction Agent Integration** - 让常见 AI coding 工具能低摩擦读取 VibeHub 状态、规则和上下文，形成可自用的集成体验。
* **P3/P4 暂缓** - runtime observation、多 Agent 协同、worktree 隔离、CI gate、enterprise policy 等高级能力只保留路线图，不进入本轮核心交付。

完成本父任务后，可以说 VibeHub v2 的核心功能可用于自用和继续迭代；不能宣称已经完成 P3/P4 的高级自动化。

## 背景

用户最初要求修复 VibeHub v2 当前 P0 spike，让它达到 self-use bootstrap。随后明确补充：

* 希望目标接近完整 V2 Core，而不是只补一个 P0 弹窗。
* P3/P4 的多 agents 协同等高级能力可以后置。
* UI 不应只是右键菜单打开的简单对话框，而应是点击项目后的“详细页 / 中台”。
* 开始实现后应拆分任务、使用子 agent/子窗口，避免主上下文无限增长。

现有需求来源：

* `dev_docs/vibehub-v2-r10.md`：产品协议与路线图。
* `docs/vibehub-v2-p0-reality-audit.md`：用户提到的 audit，但当前工作树中缺失。
* 当前代码中的 VibeHub 后端模块与 Cockpit 弹窗 spike。

## 总体需求

* 项目点击后进入 VibeHub 中台。
  * 项目列表仍是选择入口。
  * 点击项目后显示项目详情页式中台，承载 VibeHub v2 的主要状态和操作。
  * 右键菜单可保留为快捷入口，但不能是唯一或主要体验。
* 中台要服务 P0/P1/P2 核心闭环。
  * 初始化 `.vibehub`。
  * 创建/启动真实 task/run。
  * 构建 Context Pack 和 Manifest。
  * 生成 Agent View。
  * 生成 Review Evidence。
  * 生成/查看 Handoff。
  * 显示 Git、Context、Handoff、Review、Observability 的核心状态。
* 文案必须诚实。
  * Recover 如果实际只是 build handoff 或 recovery report，就不能暗示 rollback/revert。
  * P0/P1 观测能力必须标注 best-effort，不能声称 runtime interception。
* 测试必须覆盖 Golden Path。
  * 至少后端 Golden Path 覆盖 init -> start task -> build context -> agent-view -> review/handoff。
  * 前端至少保证中台入口和关键 action wiring 可构建、可类型检查。
* 实施时拆分上下文。
  * P0/P1/P2 作为子任务推进。
  * 每个子任务独立 PRD/context，适合后续派发 `trellis-implement` 和 `trellis-check`。
  * 主线程负责决策、集成和最终确认，避免把所有代码阅读堆进一个上下文。

## 子任务

* `04-30-vibehub-v2-core-p0-file-protocol-bootstrap`
  * 目标：补齐文件协议内核和 self-use bootstrap。
  * 重点：Start Task、Init、Build Context、current pointers、Golden Path backend tests。
* `04-30-vibehub-v2-core-p1-project-cockpit-center`
  * 目标：把 VibeHub v2 变成项目详情页式中台。
  * 重点：点击项目进入中台、核心状态区、4+1 flow 概览、Context/Review/Handoff/Observability 面板骨架、动作区。
* `04-30-vibehub-v2-core-p2-agent-integration`
  * 目标：低摩擦 Agent 集成。
  * 重点：AGENTS/CLAUDE/OpenCode 等静态入口、agent-view 消费路径、adapter sync 预览、managed regions、冲突检测、journal/knowledge promotion 手动版。

## 非目标

* 本轮不做 P3 runtime observation adapter。
* 本轮不做多 Agent 协同调度。
* 本轮不做 Git worktree 自动隔离。
* 本轮不做 CI gate / enterprise policy。
* 本轮不做 marketplace、cross-repo librarian、团队协同高级能力。

## 验收标准

* [ ] P0 子任务完成，能跑通真实文件协议 Golden Path。
* [ ] P1 子任务完成，点击项目后进入 VibeHub 中台，而不是只能打开右键弹窗。
* [ ] P2 子任务完成，至少一种常见 Agent 消费 VibeHub 状态/上下文的静态路径可用。
* [ ] UI 对 P0/P1 能力边界表达诚实，不夸大 Recover 或 runtime observation。
* [ ] 所有子任务各自通过 lint/type-check/tests。
* [ ] 最终父任务有一条端到端手工或自动验证记录：选择项目 -> 中台 -> init -> start task -> build context -> continue/agent-view -> review/handoff。

## 执行策略

先做 P0，再做 P1，最后做 P2。P1 中台设计可以在 P0 后端接口稳定前先研究和草图化，但真正实现时要以 P0 文件协议为数据源。P2 依赖 P0/P1 的协议和 UI 主入口稳定后再落地。

后续开始实现时，优先按 Trellis workflow 派发子 agent：

* P0：一个实现 agent + 一个检查 agent。
* P1：必要时拆前端布局/状态 wiring 两个实现分支，但避免写同一文件冲突。
* P2：先研究现有 agent-view/adapter 约定，再实现。

## 技术备注

现有相关文件：

* `dev_docs/vibehub-v2-r10.md`
* `src/components/VibehubCockpitDialog.tsx`
* `src/services/tauri.ts`
* `src-tauri/src/commands.rs`
* `src-tauri/src/vibehub/init.rs`
* `src-tauri/src/vibehub/current.rs`
* `src-tauri/src/vibehub/context.rs`
* `src-tauri/src/vibehub/agent_view.rs`
* `src-tauri/src/vibehub/status.rs`
* `src-tauri/src/vibehub/review.rs`
* `src-tauri/src/vibehub/handoff.rs`

缺失但被引用的文件：

* `docs/vibehub-v2-p0-reality-audit.md`

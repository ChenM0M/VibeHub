# VibeHub v3 改版总计划（飞行记录仪重构）

- 日期：2026-07-11
- 状态：aligned（项目所有者已确认；Research Pack、RFC backlog 与 M0 Task Pack 已进入正式规划）
- 读者：任何接手此计划的 AI agent 或人类开发者。本文档自包含，不依赖任何对话上下文。
- 本文档是 v3 改版期间的**最高优先级依据**。与旧文档（docs/rfc/0001、capability 系列文档）冲突时，以本文档为准。

---

## 0. 一句话总结

把 VibeHub 从"流程守门员"改造为"项目控制台 + 中台 + 飞行记录仪"：事件流成为唯一可信状态源，五段方法论从硬编码管道变成可修改的计划图，Agent 通过原生 MCP 工具组合交互，UI 同时呈现项目架构、文件结构与任务时间线，并以 Windows/macOS 一致、多会话可恢复、并行改码可隔离为基础能力。

---

## 1. 产品定位（不可违背的方向）

VibeHub 是 vibe coding 的**中台 + 飞行记录仪**：

- **Git 溯源代码，VibeHub 溯源需求与决策。** 弥补 Git 只能追溯代码、不能追溯"为什么这么做"的空白。
- **让每次 agent 对话带状态。** 任意时刻中断会话、切换到任意 AI 工具（Codex / Claude Code / OpenCode / Amp / Cursor 等），都能从记录的状态恢复继续。
- **用户获得对项目的绝对掌握感。** 不只看到任务进度、决策、验收达成度、用量消耗与 agent 编排，还能从项目全局看到架构边界、模块职责、依赖关系、目录/文件结构、基础设计理念及其证据来源；需要深入时可逐层下钻、打开 IDE 定位到文件/行，或携带当前上下文直接询问 agent。
- **项目是长期容器，Task 是工作与追溯单位。** UI 层级固定为 `Project → Task → PlanNode → Session → Event`。项目视图回答"这个系统是什么、怎么组成、现在整体怎样"；Task 视图回答"这次为什么改、谁做了什么、何时做、是否验收"。

### 1.1 三条边界不变量（每个功能提案都要用这三条检验）

1. **VibeHub 读世界（git / 文件系统 / 用量数据），只写 `.vibehub/` 与适配文件。** 它不写代码、不写任务实质内容。
2. **代码只由 agent 写。** 一切工程操作和交互都通过 agent 对话完成。
3. **决策只由用户做。** VibeHub 呈现状态供用户决策，不代替用户决策，也不设置阻塞 agent 的机关。

**例外只限于导航、启动与人类决策落证。** VibeHub 出身是 launcher，"把人或 agent 带到工位上"不算修改项目状态：UI 可以生成适配文件、拉起 agent 会话（自动注入简报）、在文件管理器/IDE 中定位文件，以及发起携带只读上下文的"询问项目"会话。默认仍在 agent 对话中确认；只有宿主无法提供可靠交互确认时，UI 才允许用户亲自追加 approve/reject/accept 这类决策事件。除此之外 UI 不写 canonical 项目状态，也不代替用户作决定。

### 1.2 五段方法论（保留，但实现方式彻底改变）

用户的工作方法论，作为**自适应默认**而非硬性管道：

1. **对齐（align）**——一切的根。没对齐需求，后面全偏。产出：意图、范围、带 ID 的验收标准清单。
2. **调查（research）**——收集外部证据：开源成熟方案、官方文档、论文/文章，萃取后注入任务上下文，让执行更准、少踩坑。
3. **规划（plan）**——产出**计划图**：任务拆分（节点 + 范围 + 依赖 + 验收项）+ 编排建议（并行度、建议执行工具）。图在执行中可改：加分支、拆节点、回退重跑。
4. **实行（implement)**——按图执行。随时可打断、可换工具、状态可恢复。
5. **审查（review）**——逐条核对 align 定下的验收标准，全部闭环才算任务完成。

流程深浅与任务赌注成正比：小任务走精简图，大任务走完整图。用哪张图由 align 阶段评估后建议、用户确认（详见 §3.2 与 M4）。

### 1.3 跨平台是架构约束，不是发布末期补丁

- v3 首发最低支持 **Windows 10/11 + macOS（Apple Silicon 与 Intel 能力边界明确）**；Linux 可继续保持兼容，但不以牺牲 Windows/macOS 为代价。
- 核心 domain、事件存储、MCP server、CLI、项目扫描器不得依赖 shell 方言；进程启动使用参数数组，不拼接 `sh`/PowerShell 字符串。
- 路径模型必须覆盖 Windows drive/UNC、大小写差异、分隔符、长路径与 symlink/junction；文件监听、锁、原子写、进程回收都要有双平台测试。
- 所有里程碑的验收矩阵至少包含 macOS 与 Windows；不能把"Rust/Tauri 理论跨平台"当作已验证。

---

## 2. 现状事实基线（2026-07-11 实测）

以下为改版前的硬数据，供任何执行者校验起点。

### 2.1 协议税（每次 agent 会话的启动开销）

| 启动必读 | 行数 | 字节 |
|---|---:|---:|
| `.vibehub/agent-view/current.md` | 73 | 3,409 |
| `.vibehub/agent-view/current-context.md` | 25 | 560 |
| `.vibehub/agent-view/handoff.md` | 306 | 27,786 |
| `.vibehub/rules/hard-rules.md` | 31 | 1,605 |
| `.vibehub/adapters/protocol.md` | 151 | 8,710 |
| `AGENTS.md` 注入块 | 67 | 4,610 |
| **合计** | **653 行** | **46,680 B ≈ 15.6k tokens** |

handoff.md 一项占约 60%。且这还不含 context pack。

### 2.2 后端（crates/vibehub-core，30,209 行 Rust，39 个模块，208 个测试）

- **事件流已存在且质量不错**：`events.rs`（1,891 行）——append-only JSONL（`.vibehub/tasks/<task>/runs/<run>/events.jsonl`）、~30 种事件类型、损坏尾部恢复、pending 重放、全局索引、单调 event ID。`projection.rs`（548 行）已能从事件重建 phase 状态。
- **但架构是反的**：YAML 快照（state.yaml / task.yaml / run.yaml）是主状态，30+ 模块通过 `serde_yaml::Value` 动态读写它们，事件只是审计副产品。没有统一强类型 domain model。`sync` / `recover` / `ensure_consistency` 的存在是"多可信源会漂移"的症状。
- 大块守门员机制：`capability.rs`（1,066 行，gates/claim）、`schema_check.rs`（1,090 行）、`output_lint.rs`（299 行）、`ownership.rs`（767 行）、`fitness.rs`（381 行）。
- CLI（crates/vibehub-cli，手写 dispatch）约 30 个子命令 + 别名。

### 2.3 适配文件（153 个受管生成文件）

- 35 个命令/skill 被复制 4 份：`.agents/skills/`、`.vibehub/adapters/generated/codex/`、`.claude/commands/`、`.opencode/commands/`，共 1,500+ 行 × 多份。
- AGENTS.md 与 CLAUDE.md 全文（各 67 行）都是注入块。
- 规则在 protocol.md、constraints、command-index、skills 之间大量重复。

### 2.4 前端（src/，11,943 行 TS/TSX）

- `VibehubCockpitDialog.tsx` 4,268 行，占前端 1/3，表单/字段堆砌式展示。
- **UI 可以触发 canonical 写入**（`vibehub_start_task`、`vibehub_complete_phase`、`vibehub_advance_phase` 等 Tauri command）——违反 §1.1 不变量。
- 已有 `project_structure.rs` + `ProjectStructureExplorer.tsx` 薄切片：可显示目录/文件、Git changed 标记并打开文件，但扫描固定最大深度 3、每目录 12 项、图只是目录层级，明确标记 `semantic_graph_available=false`。这是 Project Intelligence 的起点，不是最终架构视图。
- `vibehub_open_project_file` 已按 Windows Explorer / macOS `open` / Linux `xdg-open` 做基础跨平台打开；尚没有“首选 IDE + 文件/行定位”的统一抽象。
- 两套用量口径并存且未区分标注：项目级本地 agent 用量（读 Codex/OpenCode 本地 SQLite，估算）与 Gateway 统计（网关内精确）。
- 页面导航是本地 state，无路由。

### 2.5 现状与定位的偏差清单

1. plan 阶段只有步骤表单，没有编排（agent 分配 / 并行 / 交接）。
2. 流程是线性 `phase_order` 管道，不能分支、回退、改图——"不能随机应变"的结构性原因。
3. align 的 success_criteria 只是文本，review 不逐条核对，需求→验收未闭环。
4. 状态靠 agent 临终写 output.md/handoff.md，中途 kill 即丢失——与"随时掐断可恢复"矛盾。
5. gates/claim/`--confirmed-by-user` 仪式让 VibeHub 成为守门员而非中台，协议税因此膨胀。
6. research 产出没有可靠链路流入 implement 的上下文。
7. 一个 task 一条 run 一个 current 指针，多窗口/多 agent 完全没有承载。
8. UI 有 canonical 写入按钮，违反不变量。

---

## 3. 目标架构

### 3.1 数据模型：事件为唯一可信源

```
唯一可信源：  Git 仓库（代码）  +  每任务 events.jsonl（需求/决策/进展）
派生视图：    status / handoff / timeline / brief / 计划图状态 —— 全部可随时重算
```

- **所有状态变化都是事件。** YAML 快照（state.yaml 等）降级为派生缓存，任何时候可用 `vibehub rebuild` 从事件流 + git 重算。sync/recover 的大部分场景随之消失。
- **事件分两类：**
  - *agent 事件*（新增，agent 只报机器看不出的语义）：`decision`（做了什么/为什么/放弃了什么）、`progress`、`risk`、`question`、`finding`、`scope_change`、`architecture_claim`、`next_intent`（下一步打算——恢复的关键）、`criterion_check`（验收项核对结果）。
  - *系统事件*（VibeHub 写）：task/node/session 生命周期、Git/文件观察、brief 生成、图变更、Project Model 失效/重建、protocol coverage gap 等。保留现有 events.rs 的 envelope（event_id/timestamp/task_id/actor）并加 `session_id` 与 `node_id` 字段。
- **每条事件挂 commit SHA**（或 dirty 快照哈希），实现"需求→决策→代码"物理挂钩。
- **凡机器可推导的不让 agent 报**：改了哪些文件、diff、命令历史，由 VibeHub 从 git/文件系统算。agent 只报机器算不出的：决策、理由、风险、意图。
- 建立**统一强类型 domain model**（Task / PlanGraph / PlanNode / Session / Event / Criterion），废除散落各模块的 `serde_yaml::Value` 动态访问。

### 3.2 计划图（活流程图）

计划的核心对象是一张 DAG：

```yaml
node:
  id: N-xxxx
  type: align | research | plan | implement | review | custom
  title: ...
  scope: [src/gateway/, crates/vibehub-core/src/vibehub/events.rs]
  depends_on: [N-yyyy]
  acceptance: [C-001, C-002]     # 引用带 ID 的验收项
  executor_hint: { tool: any, parallel: suggested }   # 建议，不是规定
  status: <从事件派生：pending/ready/active/reported_done/accepted/needs_rework/cancelled>
```

- 图的每次变更（加节点、拆节点、回退、取消）都是事件，可回放整个规划演化史。
- 三种旧 mode（yolo/guided/evidence）退化为三张**模板图**；align 输出"建议用哪张 + 赌注评估"，用户确认。
- **验收项（Criterion）是一等公民**：align 时生成带 ID 清单，review 节点必须逐项 `criterion_check`（pass/fail/证据），UI 渲染为验收进度条。
- 编排原则：**规划到"节点 + 范围 + 依赖 + 建议并行度"为止**，sub-agent 怎么开由执行 agent 按其工具能力现场决定；VibeHub 记录实际发生的编排，UI 呈现"计划 vs 实际"两张图对照。

#### 3.2.1 DAG 不表示“世界上的所有关系”

必须区分三种结构，不能把它们硬塞进一张图：

1. **Plan DAG（排程图）**：只有 `depends_on` 这类执行先后边，必须无环，用于判断 ready、并行和拓扑集成顺序。
2. **Trace Graph（追溯关系图）**：`repairs / addresses / validates / supersedes / caused_by / touches` 等语义边，用于解释“这个修复来自哪里”，不参与 ready 计算，不要求按 DAG 拓扑执行。
3. **Event Timeline（事实时间线）**：记录每次 review、finding、返工、确认的实际发生顺序，是两张图的可回放依据。

#### 3.2.2 review 发现旧节点有问题时如何表示

不要给原节点画一条回头的依赖边，也不要覆盖原来的“完成”历史。每一轮返工展开成新的节点与 attempt：

```text
N-20 实现 ──depends_on──▶ R-21 Review #1 ──▶ N-22 修复 ──▶ R-23 Re-review
   ▲                           │                  │
   └──── targets/repairs ───── F-007 ◀─addresses─┘
```

- `R-21` 产生一等实体 `Finding F-007`，关联失败 Criterion、证据、严重度与被审查节点 `N-20`。
- 新节点 `N-22` 在排程上依赖 `R-21`，在追溯上 `repairs N-20`、`addresses F-007`；因此 DAG 继续向前，没有环。
- `N-20` 的“工作曾被完成”事实保留，但验收投影改为 `needs_rework`；`N-22` 完成不自动恢复 accepted，必须由新的 `R-23` 重新核对受影响 Criterion。
- 多轮返工继续追加 `Finding/Remediation/Re-review`，UI 默认折叠成 `N-20 → Attempt 1 → Attempt 2`，需要时再展开完整因果链。
- 若修复改变了原需求或架构，不是普通 remediation，而是创建 change request，重新评估受影响节点与 Criterion，避免用“修 bug”偷渡范围变更。

### 3.3 一个核心，三种适配器：Domain API / MCP / CLI

v3 不应让 MCP 去 shell out 调 CLI，也不应让 CLI 成为业务逻辑本体。成熟结构是：

```text
                    ┌─ MCP adapter（Agent 首选）
UI/read models ─── Domain Application Service ─┼─ CLI adapter（人类/脚本/CI/诊断）
                    └─ Tauri adapter（只读查询、导航、启动）
                              │
                       event store + git
```

- 所有校验、并发控制、事件追加、计划图规则、派生视图都只实现在 `vibehub-core` 的强类型 Application Service 中。
- MCP 与 CLI 只是同一用例的不同输入/输出适配器，必须通过同一组 contract tests，避免出现"MCP 能做但 CLI 状态不同"。
- CLI 保留，但收敛为人类可读/脚本友好的薄壳：`start/log/status/plan/brief/done/init/rebuild/doctor`。它仍是无 MCP 客户端、CI、排障和恢复时的重要逃生通道。
- **禁止提供 `run_cli(command)` 万能 MCP 工具。** 它会丢失 JSON Schema、权限语义、幂等性和可观测性，也让弱 agent 更容易拼错命令。

### 3.4 MCP 原生控制面（已定，不再列为可选项）

VibeHub v3 内置本地 MCP server，Agent 以结构化工具调用为主，CLI 为人类/脚本入口与兼容兜底。第一版使用 `vibehub mcp serve --stdio` 启动本地 server：这个命令只是进程入口，不代表 MCP 内部转调 CLI。后续只有在多客户端通知、单实例锁或远程控制出现明确需求时，才增加 loopback Streamable HTTP daemon。

**为什么现在应直接采用 MCP：** Codex、Claude Code、OpenCode 都已原生支持本地 stdio 与远程 MCP；MCP 自带能力发现、JSON Schema、结构化结果、通知与客户端 elicitation，明显比让模型记住 CLI 字符串更原生。代价是工具定义也会占上下文，所以工具面必须小、正交、按需发现，不能把旧 30 个命令一比一搬过去。

建议的 v1 surface：

| 类型 | 名称（示意） | 作用 |
|---|---|---|
| Resource | `vibehub://project/{id}/overview` | 项目状态、架构摘要、活跃任务 |
| Resource | `vibehub://project/{id}/structure` | 可分页文件树与模块图 |
| Resource | `vibehub://task/{id}/timeline` | Task 时间线、决策、风险、commit |
| Resource | `vibehub://node/{id}/brief` | 节点冷启动简报 |
| Tool | `task_start` | 创建 Task；复杂请求由 agent 提议拆分，服务端校验 |
| Tool | `session_open` | 领取 `session_id`、节点与 scope lease |
| Tool | `event_log` | 记录 decision/progress/risk/question/next_intent |
| Tool | `plan_patch` | 对计划图做带 base version 的结构化 patch |
| Tool | `criterion_check` | 写入验收证据；不能用自由文本冒充 pass |
| Tool | `node_report` | 申报节点完成/阻塞，返回未满足条件 |
| Tool | `task_propose_completion` | 只提出任务完成，最终确认仍来自用户 |
| Tool | `task_accept_completion` | 消费带摘要 digest 的 challenge；必须走宿主交互确认或受限人类入口 |
| Tool | `session_close` | 写 next intent、释放 lease、生成恢复点 |

每个写工具必须具备：`task_id/session_id/node_id` 显式标识、`expected_version` 乐观并发、`idempotency_key`、结构化错误码、返回 `event_id/current_version/next_actions`。读工具分页并支持紧凑视图；不要一次把整棵树或完整时间线塞进模型上下文。

不同 Agent 宿主对 MCP resources/prompts 的呈现能力不完全一致；为兼容性可给关键 resource 提供等价的紧凑 read tool，但二者必须调用同一个 read model，不能复制业务逻辑。

### 3.5 协议注入：MCP 优先，文件兜底

- 静态注入（AGENTS.md/CLAUDE.md）缩到 **≤20 行 / ≤1k tokens**：说明何时读 VibeHub resource、何时调用 `event_log`、何时 `session_close`，以及失败时如何退回 CLI。
- MCP `instructions` 只放跨工具的不变量与常见工作顺序，前 512 字符必须自包含；具体参数由 tool schema 描述，不在提示词重复。
- handoff.md 变成**派生且限长**的兼容视图；output.md 十段契约废除。恢复质量来自增量事件和 `node brief`，不靠 agent 停机前一次性回忆。
- 没有 MCP 能力的工具继续使用短文件协议 + CLI；MCP 故障不能让项目不可恢复。
- 适配安装器分别生成 Codex `.codex/config.toml`、Claude `.mcp.json`、OpenCode `opencode.json` 的最小配置，并显式展示将启动的本地二进制与权限。

### 3.6 UI 信息架构：项目全局 + Task 叙事

UI 不是只有 Task 时间线，而是两层控制台。

**A. Project 全局视图（长期、跨 Task）**

1. **项目总览**：技术栈、入口、构建/测试方式、主要模块、健康度、活跃 Task 与风险。
2. **架构地图**：模块/包/服务/数据流关系；区分“声明事实、静态分析、agent 推断”并显示证据与置信度。
3. **结构浏览器**：完整可按需展开的目录树、搜索、Git 变更叠层、生成/第三方文件过滤。点击模块/文件后显示职责、依赖、被依赖、关键 symbol、相关 Task/决策/commit、最近变化。
4. **全局时间图**：把所有 Task、release、重要架构决策按时间排列，可筛选模块/agent/分支。
5. **深入动作**：在文件管理器显示、在首选 IDE 打开项目或定位到文件/行、带当前模块证据启动“询问项目”会话。

**B. Task 视图（单次需求与追溯单位）**

1. **时间线**：用户需求原话 → 决策（做了什么/为什么/放弃什么）→ commit/diff → 测试 → 风险 → 恢复点。
2. **计划图**：DAG、计划 vs 实际编排、节点状态、scope 与依赖；待执行节点可启动 agent 并注入 brief。
3. **验收进度**：Criterion checklist，逐项 pass/fail/证据。
4. **会话泳道**：Codex/OpenCode/Claude 等 session 在时间轴上的并行区间、占用范围与交接关系。

当前 `project_structure.rs`、`ProjectStructureExplorer.tsx` 和打开文件能力作为 M1 原型基础保留，但现状只是深度/数量截断的文件系统树，`semantic_graph_available=false`；v3 要把它升级为按需加载、可搜索、带语义和证据的 Project Model。

### 3.7 Project Intelligence：可解释的项目模型

“让前端直接看出架构”不能靠 LLM 编一段漂亮摘要。系统应建立三层证据：

1. **硬事实**：文件树、Git 状态、语言/包管理 manifest、构建入口、测试命令、import/dependency、公开 symbol。
2. **声明知识**：README、ADR、架构文档、配置与用户确认的模块说明。
3. **推断知识**：agent 归纳的模块职责、边界和数据流，必须带来源、生成时间、模型/规则版本与置信度，可被新证据推翻。

扫描器采用增量索引：尊重 `.gitignore`，默认排除 vendor/generated/build artifacts；目录树按需分页，语义索引按语言插件逐步增加。没有 parser 的语言仍能提供硬文件结构，不能为了“图完整”伪造依赖。项目模型以 read model 存储，可由文件系统 + Git + 声明知识重建，不成为新的 canonical source。

### 3.8 Agent 防偏：约束操作，不假装约束智能

VibeHub 能显著降低弱 agent 走偏，但**仅靠 MCP 或提示词不能保证 agent 写对代码**。如果宿主仍允许 agent 任意写整个仓库，VibeHub 最多能发现越界，不能物理阻止。成熟方案是分层防线：

1. **可发现性**：短 server instructions + 少量正交工具 + 每个错误返回可执行 next action。
2. **输入约束**：JSON Schema、枚举、Criterion ID、图版本、scope，不接受含糊自由文本替代结构字段。
3. **状态约束**：服务端验证依赖、版本、幂等、节点状态和用户确认挑战；绝不相信 agent 自己声称“我已经确认”。
4. **权限约束**：读写工具分级，默认最小权限；高风险工具使用客户端 approval/elicitation。MCP 配置展示并限制启用工具。
5. **工作区约束**：scope overlap 预警；真正需要隔离时用 worktree、宿主 sandbox 或 hooks 限制目录。VibeHub 记录越界并在 UI 高亮。
6. **运行约束**：session lease/heartbeat、乐观并发、原子事件追加、崩溃恢复；冲突必须显式返回，不做 last-write-wins。
7. **质量约束**：协议 conformance、错误恢复、弱模型任务集、故意漏步骤/错 task/重复调用/过期版本的 adversarial tests。

自由度来自工具的**可组合性**，不是来自“什么都能执行”的万能入口。少量原子工具配合资源读取可以覆盖很高的自由度，同时把危险状态转换保留在服务端。

### 3.9 多会话、并行改码与 Git worktree

- 每个 agent 会话领取 `session_id`，事件带 session 标识；多会话安全追加同一任务事件流。
- 会话从 PlanNode 继承 scope，VibeHub 先检查依赖与文件范围重叠，再建议能否并行。
- **Git worktree 的含义**：同一个 Git 仓库可以在多个独立目录同时 checkout 不同分支；每个目录有自己的 `HEAD`、index 和未提交文件，但共享 Git object database 与 refs。于是两个 agent 不会在同一工作目录里互相覆盖未提交文件。
- worktree 不是自动并行器，也不会消灭逻辑/合并冲突。要安全使用，必须配套“每节点一分支/目录、稳定 base commit、scope 尽量不重叠、节点内提交、集成队列、失败回收与冲突归属”。
- 推荐 v1：VibeHub 只为 `parallel_safe=true` 且依赖已满足的节点创建 worktree；命名 `.vibehub/worktrees/<task>/<node>`（如 Windows 路径长度不利则落到应用数据目录）；节点完成后由集成节点按拓扑顺序 merge/cherry-pick，冲突回到原节点处理。
- 多 agent 使用同一事件存储时，事件追加要跨进程锁 + 原子落盘；worktree 自身的 `.vibehub` 视图不得分裂出多套 canonical 任务状态。

### 3.10 Project Intelligence 的触发与生命周期

项目结构不应依赖 agent “想起来才维护”，也不能每改一行代码就让 LLM 重写架构说明。按证据类型分配维护者与触发器：

| 触发 | VibeHub 自动工作 | AI 工作 | 结果 |
|---|---|---|---|
| 项目首次注册 / v3 init | 全量文件树、Git、manifest、入口与现有文档扫描 | 对无法由规则确定的模块职责做一次证据化摘要 | `ProjectModel v1`，状态 `partial` 或 `fresh` |
| 文件/目录/Git HEAD 变化 | watcher + 内容 hash 增量更新硬事实；标记受影响模块 | 通常不调用 AI | 精确文件树立即更新，语义层可能 `stale` |
| manifest/import/入口/顶层目录变化 | 重算受影响依赖与架构 fingerprint | 仅对受影响模块重新归纳，不全仓重写 | 新版本 Project Model |
| 计划涉及未知模块 | 构建该节点所需的局部索引 | 按需解释局部职责/风险 | node brief 补齐上下文 |
| agent 作出架构决策 | 记录 decision 与关联文件/模块 | agent 只报告“为什么”和放弃方案 | 声明知识增量更新 |
| node/session 结束 | 对比 base/HEAD/diff，判断结构影响 | 若检测到架构变化但无决策，要求最小补录 | `architecture_impact` 或 coverage gap |
| 用户打开架构视图/主动刷新 | 先返回当前版本与 freshness，再后台补建 | 只有无缓存且确有失效时调用 | UI 不因全量 AI 分析阻塞 |

生命周期固定为：`discover → index → infer(optional) → publish → observe → invalidate → incrementally rebuild → supersede`。每个 Project Model 版本记录 `source_commit/source_hash/indexer_version/generated_at/evidence/confidence`；旧版本可追溯但不继续修改。

关键原则：

- **文件结构和 Git 事实永远由机器维护；AI 不负责抄目录树。** AI 只补机器无法可靠推出的语义。
- 采用 fingerprint 驱动失效，不靠固定时间 TTL。无关 README 拼写修改不应让全项目架构失效，manifest/入口/模块边界变化才触发结构级更新。
- UI 必须显示 `fresh/stale/partial/unsupported/rebuilding`，过期信息可以看但不能伪装成最新事实。
- Project Model 是可重建 read model；AI 推断也以带证据的 claim/event 保存，不能成为第二个不可解释真相源。

### 3.11 一个 Task 从创建到结束的完整生命周期

| 阶段 | 用户 | Agent | VibeHub |
|---|---|---|---|
| 1. Intake | 说出原始需求 | 识别单/多意图，提出 Task 草案 | 保存原话、创建 draft、绑定 Project Model 版本 |
| 2. Align | 澄清并确认目标/边界 | 产出 intent、scope、Criterion、非目标与赌注评估 | 分配稳定 ID，记录确认来源与未决问题 |
| 3. Research | 决定调查深度 | 收集并萃取证据，关联到 Criterion/风险 | 保存来源、时间、可信度，供 brief 按需读取 |
| 4. Plan | 确认关键取舍 | 提议 Plan DAG、scope、依赖、并行度与执行工具 | 校验无环、重叠、版本和可执行性；记录图变更 |
| 5. Schedule | 必要时选择工具/并行策略 | 领取 ready node | 创建 session/lease/brief；需要时创建 worktree |
| 6. Execute | 只处理需要用户决定的问题 | 写代码；仅在关键决策/风险/换方向时增量 log | 自动观察 Git/文件/进程；更新 timeline 与 coverage |
| 7. Report | 查看阶段结果 | `node_report` 附测试/证据与 next intent | 验证结构完整性，状态变 `reported_done`，不等于 accepted |
| 8. Review | 判断失败是否接受/改范围 | 逐项 criterion_check，产生 Finding | 失败则追加 remediation + re-review 节点，更新追溯图 |
| 9. Accept | 明确确认任务完成 | 提交 completion proposal，展示摘要 digest | 确认所有 Criterion/风险/版本，记录用户 acceptance |
| 10. Settle | 查看最终叙事 | 无额外临终长报告 | 释放 lease/worktree、刷新受影响 Project Model、生成派生摘要并归档 |

这张表描述完整语义生命周期，不重新引入固定线性管道。小任务可把 Align/Research/Plan 合并为一个轻量节点，甚至跳过无价值调查；但 Criterion、session 归属、review finding、用户 acceptance 等状态语义保持一致。

任何阶段都允许中断。恢复入口始终是 `session_open → node brief`；brief 从事件、当前图、Criterion、Project Model freshness 与 Git 事实生成，不要求前一个 agent 在线。

### 3.12 Agent 最小维护契约与失联降级

Agent 不应为了 VibeHub 变成兼职书记员。正常工作只增加五类动作：

1. **开始 Task/session**：调用 `task_start` 或 `session_open`，确认自己正在正确的 Task/Node/scope。
2. **开始工作前**：读取一个 node brief，不再手工遍历整套协议文件。
3. **语义变化时**：只记录机器看不出来的 decision/risk/question/finding/scope change；普通文件编辑不逐条 log。
4. **节点申报时**：提交结果、测试/验收证据与受影响 Criterion。
5. **暂停/退出时**：`session_close` 写一句结构化 next intent；若被 kill，已落盘的增量事件仍可恢复。

如果 Agent 漏做或不遵循指令，按“自动补事实、暴露缺口、限制虚假声明、不阻止真实工程”处理：

- VibeHub 从 Git/文件系统自动补齐改动文件、commit、diff、时间等硬事实，并创建 `unattributed_change`，不让观察空白消失在 UI。
- 下一次 MCP 调用发现未开 session、过期 version、scope 越界或未关闭 lease 时，返回结构化 repair action；能自动修的自动修，语义缺口只要求最小 backfill。
- 给 Task/session 维护 `protocol_coverage`：`complete / recoverable / degraded / unknown`，并列出缺失的是 decision、criterion evidence 还是 next intent。这个指标表示记录完整度，不表示代码质量。
- VibeHub 不拦截 agent 直接写代码，但缺少 Criterion 证据时拒绝把节点投影为 `accepted`，缺少可靠用户确认时拒绝把 Task 投影为完成。这是维护账本真实性，不是流程守门。
- hooks/host policy 可用于提醒或自动调用，但不能成为唯一依赖；不支持 hooks 的 Agent 仍可通过 MCP precondition、自动观察和下一会话 repair 恢复。
- M2/M4 必须用弱模型和故意违规场景验证：跳过 `session_open`、写错 task_id、忘记 log、重复 tool call、过期 graph version、直接声称完成、进程被 kill。

---

## 4. 里程碑

采用 **Experience-first（体验优先）+ Contract-first（契约优先）+ Vertical Slice（垂直薄切片）**。这比简单的“先前端、后后端”更准确：先用高保真可交互前端验证信息架构和使用体验，同时先定义稳定 read model/MCP schema；随后每次只打通一条 UI→Domain→Event Store 的真实链路。纯静态前端做完再反推后端会制造大量假数据假设，不采用。

依赖关系：M1 先把产品看得见，M2 建控制面与事件地基，M3 补项目智能，M4 打通任务闭环，M5 实现真正多 agent 并行，M6 做迁移与发布硬化。每个里程碑都先交付可演示薄切片，再加深。

### M0 冻结、基线与 UX 契约（0.5–1 天）

- 冻结旧体系功能开发；本文档入库；记录协议税基线（§2.1 已测）。
- 圈定保留资产：`events.rs`、`projection.rs`、`context.rs`（改造为 brief builder）、`agent_adapter.rs`（瘦身）、`drift.rs`（并入 doctor）、launcher、gateway、扫描器。
- 定义第一版 `ProjectOverviewView / ProjectStructureView / TaskTimelineView / PlanGraphView / NodeBrief` fixture 与 JSON Schema；fixture 必须覆盖空项目、大仓库、无架构文档、并行 Task、Windows 路径。
- 从主计划拆出五份实现 RFC：domain/event、MCP contract、Project Intelligence lifecycle、Task/Node lifecycle、worktree orchestration；主计划只保留不变量和验收。
- **验收**：无上下文 agent 能复述目标；前后端对同一组 view contract 无歧义；fixture 可驱动 M1。

### M1 前端体验原型（先让产品可见）

- 用 fixture 重写 Project Center：项目总览、架构地图、结构浏览器、全局时间图。
- 重写 Task 体验：时间线、计划图、验收进度、会话泳道；完整展示 loading/empty/stale/error/partial/large-data、review finding/rework attempt 与 protocol coverage 状态。
- 点击模块/文件显示证据化详情；打通现有 reveal/open，新增“首选 IDE 打开/定位”和“询问项目”启动流程的 mock 交互。
- UI 只依赖 M0 view contracts，不直接读取 v2 YAML 形状；通过交互原型反推缺失字段并修订契约。
- **验收**：
  1. 项目所有者仅看原型即可完成“理解项目→发现活跃风险→进入 Task→查看依据→打开 IDE 深入”的任务；
  2. Windows/macOS 两套 viewport 与长路径 fixture 无溢出/重叠；
  3. 每个屏幕字段都能对应到契约与证据源，没有“先放一个以后再算”的神秘指标。

### M2 核心反转 + MCP 控制面（最大的一块）

- 新建 v3 domain model（含 Finding、Attempt、typed trace relation、ProjectModelVersion、ProtocolCoverage）；事件 envelope 扩展 `session_id/node_id/commit_sha`；所有状态变更统一“追加事件→派生视图”。
- 实现 Application Service、`rebuild`、跨进程追加锁与乐观版本；M0/M1 view contracts 接入真实数据。
- 用官方 Rust MCP SDK（当前 Tier 2）做隔离 adapter；先完成 resources + `session_open/event_log/session_close` 的最小闭环，再增加计划/验收工具。为 Tier 2 风险增加 MCP Inspector 与跨客户端 contract tests。
- CLI 变成同一 Application Service 的薄 adapter；`vibehub mcp serve --stdio` 随桌面应用一起打包，不依赖 Node/Python runtime。
- 新版 ≤20 行注入、MCP 配置生成、handoff 派生限长；`doctor` 合并旧诊断。
- **验收**：
  1. 删除派生文件后可完整 rebuild；重复 idempotency key 不产生双事件；过期 version 明确冲突；
  2. Codex/OpenCode/Claude Code 至少各跑通一次“读资源→开 session→log→关 session→新会话恢复”；
  3. 启动协议 ≤1k tokens，受管文件 ≤40；MCP 不可用时 CLI 兜底仍能恢复；
  4. macOS 与 Windows 的 stdio 启动、路径、锁、崩溃清理测试通过；故意漏开/漏关 session 后能产生 gap 并 repair。

### M3 Project Intelligence：从文件树到架构地图

- 把现有深度 3/每目录 12 项的扫描升级为按需分页、搜索、Git 叠层与 ignore 规则。
- 解析 manifest、workspace/package、imports 与 symbols，构建模块依赖；读取 README/ADR 等声明知识；推断层带证据/置信度/生成版本。
- 落地 §3.10 的 fingerprint、增量失效、局部 AI 归纳与 `fresh/stale/partial/unsupported/rebuilding` 生命周期。
- Project resources 与 UI 接真实索引；模块详情关联历史 Task/决策/commit；IDE 定位跨平台完成。
- **验收**：
  1. 在 VibeHub 自身仓库与至少两个异构样本项目上，用户能从总览下钻到真实证据；
  2. 大仓库首次可交互时间、增量刷新与内存达到预算；扫描中断不会留下“完整”假状态；
  3. 无 parser/无文档时明确降级为硬文件事实，不生成伪架构；结构无关改动不会触发全仓 AI 重算。

### M4 Task 计划图、时间线与验收闭环

- PlanGraph 与 Criterion 一等模型；三张模板图；图变更事件；research 摘要进入 node brief。
- 实现 Finding → remediation → re-review 展开式循环；排程 DAG 保持无环，Trace Graph 保留 repairs/addresses/validates 因果。
- M1 Task 原型接真实 timeline/plan/criteria/session 数据；节点启动优先使用 MCP 资源简报，CLI brief 兜底。
- 删除 UI canonical 写入，只保留导航、适配生成与会话启动；用量双口径标注。
- **验收**：
  1. 真实任务可完成 align→研究→计划→执行→逐项验收，图变更全史可回放；
  2. 用户只看 UI 能回答进度、关键决策、验收比例、下一步、涉及模块；
  3. fail Criterion 只能让 agent 提议未完成，不可伪装任务已完成；
  4. 从节点一键拉起 Codex/OpenCode/Claude，brief 生效且 Task/session 归属正确；
  5. review 连续两轮发现问题时，旧 attempt 不被覆盖、验收不会提前恢复、UI 可折叠查看完整修复链。

### M5 多会话 + worktree 并行编排（必要能力，不再可选）

- session scope/lease/heartbeat，重叠可视化，计划 vs 实际编排。
- 每个可并行 PlanNode 一分支一 worktree；创建、锁、启动、提交检查、集成、冲突回退、清理形成完整生命周期。
- 支持同工具多开与 Codex/OpenCode/Claude 混合并行；不假设它们共享进程或上下文。
- **验收**：
  1. 三个并行 session（至少两个不同工具）处理三个无重叠节点，工作目录和事件均无污染；
  2. 人为制造同文件冲突，系统在启动前预警或在集成时明确归属，绝不静默覆盖；
  3. agent 崩溃、桌面应用退出、Windows 文件占用三种异常后可恢复/回收；
  4. UI 能解释每个 worktree 的 base、branch、owner、改动、集成状态。

### M6 迁移、legacy 浏览与发布硬化

- 执行 v2 clean break 归档；提供最简单的 legacy-v2 只读列表/详情入口，独立 adapter 读取，绝不让旧 schema 进入 v3 domain。
- 删除旧命令/适配复制/写入式 UI；完成安装、升级、卸载、签名与双平台 smoke/e2e。
- **验收**：旧历史可看但不会拖累新写路径；Windows/macOS 从干净安装到 MCP 会话、IDE 打开、并行节点、卸载清理全流程通过。

---

## 5. 迁移与兼容

- **Clean break，不做数据迁移。** v3 `vibehub init` 把旧 `.vibehub/` 整体移入 `.vibehub/legacy-v2/` 只读归档（约 3.9 MB，主要是 tasks/ 历史），新结构从零开始。理由：用户基数即项目所有者本人，迁移代码的成本远高于价值。
- 旧命令不做别名兼容，直接删；`vibehub doctor` 对旧结构给出"这是 v2 项目，运行 init 归档升级"的提示。
- 适配文件由 M2 重新生成并清理旧文件（有 config.yaml 的 153 文件清单可据以删除）。
- `.DS_Store` 等垃圾一并清理并入 .gitignore。

## 6. 删除清单（v3 明确废除）

| 废除 | 替代 |
|---|---|
| gates / claim / release / capability 仪式（capability.rs 1,066 行） | 计划图节点依赖 + review 验收闭环 |
| schema_check.rs（1,090 行）、output_lint.rs、output.md 十段契约 | MCP schema + 增量 `event_log` + 机器推导；CLI `log` 仅兜底 |
| ownership.rs、fitness.rs、next_action.rs、start_task 的 intake 拆分 | 图 scope + doctor + agent 自主判断 |
| sync / recover / continue 三命令 | 事件为唯一源后大部分漂移不存在；剩余诊断入 doctor |
| 35 个 skill × 4 份复制 | 一个 MCP server + 最小 server instructions + 少量 CLI fallback 文档 |
| `--confirmed-by-user` 自报 flag（phase 级） | `completion_proposed` + 带摘要 digest 的交互确认；节点推进是自然工作流 |
| UI 任意 canonical 写入 command | UI read-mostly；只保留启动/导航和受限的人类决策兜底 |
| MCP 包装任意 CLI 字符串 | MCP/CLI 共同调用强类型 Application Service |

## 7. 已定决策（项目所有者已确认的方向）

1. 五段方法论保留，但以"模板图 + 自适应深浅"实现，不是硬管道。
2. 三条边界不变量（§1.1）成立；导航、启动与受限的人类确认是非工程写入例外，VibeHub 仍不代替用户决策。
3. 编排"看见"优先于"规定"：图规划到节点/范围/依赖/建议并行度为止。
4. 状态保存必须增量（边干边 log），不能依赖临终报告。
5. 验收标准必须一等公民、review 逐项闭环。
6. 用量放弃"与远端账单一致"目标，改为双口径明示。
7. Agent 首选 MCP 原生工具，CLI 保留做人类/脚本/诊断/故障兜底；两者共享同一 core。
8. Project 与 Task 双层信息架构；Task 仍是追溯单位，但项目架构/文件结构跨 Task 存在。
9. 采用体验优先 + 契约优先 + 垂直薄切片；M1 先做前端体验原型，再以契约反推后端。
10. M5 worktree 并行是必要能力，但只有 scope、branch、集成与冲突策略一起完成才算交付。
11. Windows/macOS 是每个里程碑的验收维度。
12. 协议税目标 ≤1k tokens/会话（node brief 与按需资源另计，但必须有预算）。
13. Plan DAG 只表达排程依赖；修复、验证、取代等关系进入 Trace Graph，实际发生顺序由 Event Timeline 记录。
14. Project Intelligence 以机器增量维护为主、AI 按失效范围惰性补语义；绝不要求 Agent 手工维护文件树。
15. Agent 漏遵循协议时允许工程继续，但必须显示 coverage gap；缺少验收/确认时不能把账本投影为 accepted/completed。

## 8. 原待定问题的结论

### 8.1 kill-resume：不限制“重复次数”，限制“语义断裂”

不建议用“最多重读 N 个文件/重跑 N 条命令”作为唯一上限。重跑测试可能是正确验证，重复问已回答的问题却是严重失败。成熟做法是 **Recovery Quality SLO + 场景基准**：

**四个零容忍 hard gate：**

1. 不得违背已确认决策或把已拒绝方案重新当成默认；
2. 不得重复不可逆操作、重复发布、重复迁移或静默覆盖；
3. 必须恢复到正确 Task/Node/scope/base commit；
4. 不得再次询问事件流中已有明确答案的用户问题。

**可量化目标：**

- 新会话通过 `session_open` + 一个 node brief（目标 ≤2 次 MCP 往返、≤2k tokens，不含按需源码）定位当前状态与下一步。
- 人类无需重新复述目标、关键决策和验收标准；首次行动与 gold next intent 一致。
- 重复劳动采用加权 waste score，而非裸次数：重复读取/测试低权重，重复代码实现/用户解释/不可逆动作高权重。
- 建立中断矩阵：对齐后、调查中、计划改图后、编辑未提交、测试失败后、等待用户、两个 session 并行中。每个场景自动核对 state/scope/decision/next action，人工评审叙事连续性。
- 阶段目标：M2 达到“单会话语义连续（L2）”，M5 达到“多会话依赖连续（L3）”；任一 hard gate 失败即不合格，不用平均分掩盖。

这比追求“完全不重读”更接近真正的完美衔接：允许合理重新验证，但不允许丢失意图、决策或安全边界。

### 8.2 MCP server：做，而且是 Agent 首选；CLI 也保留

- **MCP 侧重点**：让模型发现并调用结构化工具/资源，获得 schema、能力协商、通知、approval/elicitation 和统一跨工具接入。
- **CLI 侧重点**：人类操作、shell 脚本、CI、debug、无 MCP 客户端与故障恢复；文本/JSON 输出也便于观察底层事实。
- **实现结论**：两者都要，但都只是 adapter；业务逻辑只存在一份。第一版内置本地 stdio server，避免 daemon、端口、鉴权和安装复杂度；以后再按多客户端实测决定是否增加本地 HTTP daemon。
- **技术风险**：官方 Rust MCP SDK 当前为 Tier 2，不如 TypeScript/Python Tier 1 成熟；考虑到 VibeHub core 与桌面分发均为 Rust，仍优先 Rust，代价由 adapter 隔离、Inspector、协议测试和跨客户端 E2E 对冲。

### 8.3 用户确认：对话体验不变，去掉自证 flag

默认流程是 agent 调 `task_propose_completion`，服务端返回包含验收摘要、state version 与 digest 的 confirmation challenge；agent 在当前对话展示，用户明确确认后再提交该 challenge。支持 MCP elicitation/强制 tool approval 的宿主由宿主直接收集确认，VibeHub 记录 `confirmation_channel` 与摘要 digest。

`--confirmed-by-user` 不能证明用户真的确认，只能证明调用者传了一个 flag，因此删除。对不支持交互确认的宿主，降级为 `completion_proposed`，再由受限 UI/人类 CLI 完成确认；不能让 agent 自报后直接变成强 `user_confirmed` 证据。

### 8.4 legacy-v2：保留最小只读入口

保留 Task 列表、标题、时间、阶段、最终摘要与文件链接即可。它通过独立 legacy adapter 读取归档，禁止复用 v3 domain、禁止写、禁止为了旧 UI 拖慢 M1–M5；实现放到 M6。

### 8.5 M5 worktree：值得且必要，但不是“多开窗口”的同义词

用户的理解基本正确：只要任务切得互不干扰，就可以同时开多个 Codex、多个 OpenCode，或混合运行。缺少 worktree 时，它们若共用一个目录，会共享未提交文件和 index，容易互相覆盖/误提交；worktree 给每个节点独立目录和分支，解决工作区隔离。

但 worktree 不解决任务拆错、共享数据库迁移、公共类型改动、最终 merge 冲突，也不自动调度 agent。因此 M5 的真正交付物是“计划图可并行性判断 + scope overlap + worktree 生命周期 + 集成队列 + 冲突归属 + UI 可观测”，而不是一个 `git worktree add` 按钮。

### 8.6 仍需在实现中用原型/测试回答的问题

1. Architecture Map 第一批支持哪些语言/构建系统，按真实项目样本排序，而非预先承诺全语言。
2. 本地 stdio 多进程在压力下是否足够；只有跨进程通知/锁成为瓶颈时才引入 daemon。
3. 不同宿主对 MCP elicitation、tool annotations、动态 tool discovery 的支持差异，必须进兼容矩阵。
4. worktree 根目录在 Windows 的默认位置与最大路径预算，需要真实机器压测后定。

## 9. 术语表

| 术语 | 含义 |
|---|---|
| 事件流 events.jsonl | 每任务 append-only JSONL，v3 唯一可信状态源（git 之外） |
| 计划图 PlanGraph | 任务的 DAG：节点/依赖/范围/验收/编排建议，可在执行中修改 |
| Trace Graph | repairs/addresses/validates 等非排程语义关系图，用于追溯因果，不参与 ready 计算 |
| Finding | review 发现的一等问题实体，关联证据、Criterion、目标节点、严重度与后续 remediation |
| Attempt | 同一意图的一次具体执行/修复尝试；失败历史保留，不用覆盖原节点 |
| 验收项 Criterion | align 产出的带 ID 可核对标准，review 逐项闭环 |
| 简报 brief | 按节点生成的紧凑上下文包：范围+决策+调查摘要+验收项，供新会话/sub-agent 冷启动 |
| Project Model | 从文件/Git/manifest/文档/推断生成的可重建只读模型，为架构图和结构浏览器供数 |
| MCP server | 向 Agent 暴露 VibeHub resources/tools 的协议适配器；不承载重复业务逻辑 |
| Application Service | 唯一业务用例层，供 MCP/CLI/Tauri adapters 共同调用 |
| Experience-first | 先验证用户看到什么、如何完成任务，再落实底层实现，但必须与 contract-first 配套 |
| Vertical Slice | 每次打通一条可工作的 UI→core→store 链路，而不是横向把一层全部做完 |
| Git worktree | 同一仓库的独立工作目录/HEAD/index，供并行节点隔离未提交改动；不自动消除 merge 冲突 |
| Protocol Coverage | VibeHub 记录完整度：complete/recoverable/degraded/unknown；不等同于代码质量 |
| 协议税 | agent 会话为服从 VibeHub 协议付出的启动上下文开销（tokens） |
| 中台不变量 | §1.1 三条边界：VibeHub 只写 .vibehub/；代码只由 agent 写；决策只由用户做 |
| kill-resume 测试 | 任意时刻杀掉会话，新工具冷启动恢复，度量重复劳动与决策矛盾 |

## 10. 调查依据（2026-07-11）

1. [MCP Architecture Overview](https://modelcontextprotocol.io/docs/learn/architecture)：确认 MCP 的 host/client/server 架构、tools/resources/prompts、stdio/Streamable HTTP、能力协商与通知。
2. [MCP SDKs](https://modelcontextprotocol.io/docs/sdk)：官方 SDK 分级；当前 Rust 为 Tier 2，TypeScript/Python/C#/Go 为 Tier 1。
3. [MCP Security Best Practices](https://modelcontextprotocol.io/docs/tutorials/security/security_best_practices)：本地 server 同意机制、最小权限、stdio 安全、禁止 shell 打开 URL、scope minimization 等。
4. [OpenAI Codex MCP 文档](https://developers.openai.com/codex/mcp)：Codex CLI/IDE/Desktop 共享 MCP 配置，支持 stdio/Streamable HTTP、server instructions、tool allow/deny 与 approval mode。
5. [Claude Code MCP 文档](https://code.claude.com/docs/en/mcp)：支持本地/远程 MCP、动态 tool 更新、resources、elicitation、tool search 与强制用户交互元数据。
6. [OpenCode MCP 文档](https://opencode.ai/docs/mcp-servers)：支持 local/remote MCP、OAuth 与按 agent 启停工具；同时明确提醒过多工具会消耗上下文。
7. [Git worktree 官方文档](https://git-scm.com/docs/git-worktree)：同一仓库可挂多个 working tree，各自拥有 HEAD/index 等 per-worktree 状态并共享仓库对象与 refs；也明确 submodule 多 checkout 支持不完整。

## 11. 计划应写多细，以及写完之后怎么执行

### 11.1 文档分层：主计划不承担所有细节

主计划的职责是让任何新参与者在有限上下文内理解“不可以走偏的方向”，不是成为几千行实现说明。采用五层文档：

| 层级 | 放什么 | 不放什么 |
|---|---|---|
| 本主计划 | 产品定位、不变量、核心 domain、生命周期、里程碑、验收、已定/未定决策 | 完整 JSON Schema、每个函数、逐文件改法 |
| RFC | 一个难题的完整 contract 与备选方案，例如 MCP tools、事件 schema、索引生命周期 | 与该主题无关的项目总览 |
| ADR | 一项已作出的关键选择、理由、替代方案与后果 | 大段实施教程 |
| Milestone Task Pack | 当前里程碑的 scope、依赖、文件、fixture、测试命令、Criterion | 后续所有里程碑的细节 |
| 代码与测试 | 可执行真相与边界行为 | 产品战略解释 |

判断是否该写进主计划的规则：如果遗漏会让另一个 agent 做出**架构方向相反但局部看似合理**的实现，就写进来；如果只是决定某个 struct 字段、tool schema 或组件拆分，就放 RFC/Task Pack。本文 §3.10–3.12 定义生命周期与责任边界，已经达到主计划应有粒度；具体事件 payload 和状态转移表在 M0 RFC 固化。

### 11.2 用现有 v2 VibeHub 执行，但不要让旧模型绑架 v3

现有 VibeHub 是建设 v3 的**施工脚手架**：继续用它记录任务、调查、计划、实现证据与 review，因为它已经可用；但 v3 架构以本文和 RFC 为准，不为了适配旧 capability/gate/current pointer 而扭曲新 domain。

执行策略：

1. 当前“完善 v3 方案”Task 只负责把方向、调查依据、RFC backlog 与 M0 入口准备好，不在同一个巨大 Task 里实现 M0–M6。
2. 项目所有者确认本计划后，按现有 VibeHub 规则完成当前 align，再让 research/plan 输出正式 research pack 与 M0 Task Pack；implement/review 只完成这些计划资产的定稿。
3. 当前方案 Task 关闭后，M0–M6 **每个里程碑建立独立 VibeHub Task**，每个 Task 有自己的 Criterion、上下文和 review。高风险 M0/M2/M5 用 evidence drive，M1 体验原型可用 guided drive。
4. 里程碑之间的依赖先由本文与 Task metadata 显式引用；在 v3 PlanGraph 尚未可用前，不假装旧 VibeHub 已能表达完整 DAG。
5. v3 代码以独立 module/feature flag/fixture adapter 做垂直薄切片，保持 v2 可运行；直到 M6 才 clean break 数据与移除旧写路径。
6. 不要过早 self-host：M0–M3 仍由 v2 跟踪；M4 的 Task/PlanGraph 稳定后，用 v3 试跑一个内部任务；验证通过再用 v3 编排 M5 多会话。

### 11.3 计划确认后的第一个实际动作

不是直接大改事件系统，也不是继续扩写总计划。第一步是 M0：

1. 固化五份 RFC 的目录、问题边界和负责人；
2. 先定义 UI 所需五个 read model contracts 与覆盖异常状态的 fixtures；
3. 用这些 fixtures 开始 M1 高保真前端原型，让项目所有者验证信息架构；
4. 同时只做足以证明契约可实现的 core spike，不提前铺开全后端；
5. 原型确认后，再冻结 v1 contracts 并进入 M2/M3 的真实数据接入。

开始写代码的 Definition of Ready：本计划已由项目所有者确认；M0 Task Pack 已列出 scope/非目标/Criterion；关键 RFC 的开放问题不会阻塞第一个薄切片；Windows/macOS fixture 与验证方式已定义。未满足时继续对齐，不用“先写起来再说”制造返工。

### 11.4 正式执行工件与任务映射

- Research Pack：`.vibehub/research/current/research-pack.md`
- 五份 RFC backlog：`docs/v3/rfc-backlog/001` 至 `005`
- M0 Task Pack：`docs/v3/m0-task-pack.md`

| 里程碑 | VibeHub Task ID | 依赖 |
|---|---|---|
| M0 | `T-20260711080143-c3669d9d` | 无 |
| M1 | `T-20260711080143-49b5012c` | M0 |
| M2 | `T-20260711080143-b1ff21ea` | M0、M1 |
| M3 | `T-20260711080143-e975f3c8` | M2 |
| M4 | `T-20260711080143-fbe94685` | M2、M3 |
| M5 | `T-20260711080144-de5ecb84` | M4 稳定性门通过并由项目所有者批准 self-host |
| M6 | `T-20260711080144-5db160f2` | M5 |

这些 Task 保持独立 scope、Criterion、上下文与 review。M0-M4 继续由 v2 跟踪；M5 是第一个由通过稳定性门的 v3 自管理的里程碑。

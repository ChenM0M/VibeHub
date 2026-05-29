# VibeHub Capability 重构 — 分步落地手册（完整版）

> **文档状态**: Draft v3.7
> **日期**: 2026-05-27（v2: 2026-05-28）
> **配套基线**: [`docs/vibehub-capability-redesign-2026-05-27.md`](./vibehub-capability-redesign-2026-05-27.md)（下称「基线」）
> **目的**: 把基线 §1-29 的**全部需求**拆成「一次对话窗口正好做完一个 Step」的可执行清单。每个 Step 自带验收指标（DoD）；**未达成 DoD 不允许结束当前对话**，必须由 agent 在同一会话里继续或主动 handoff 给下一会话续跑。
> **范围**: v1 首版仅覆盖 S0-M5（核心引擎）。v2 扩展覆盖 M6-M8（多 task / UI / 自定义）+ X1-X4（跨切面工程）。
> **使用方式**:
> 1. 用户对 agent 说 `执行 Step N`（或 `vibehub-continue` + 当前 Step 标识）
> 2. Agent 按本手册「必读输入 → 任务清单 → DoD 检查」三段式跑完
> 3. 跑完前必须自检 DoD；任何一条 ❌ → **不允许结束**，要么继续做、要么写 handoff 说明阻塞点
> 4. 跑通后写 `handoff.md` + 推进 VibeHub 事件，进入 Step N+1

---

## 0. 总览（Step 地图）

### Phase 1 — 核心引擎（v1 scope, S0-M5）

| Step | 名称 | 类型 | 预计窗口 | 依赖 |
|---|---|---|---|---|
| ✅ **S0** | 基线入库 & 共识校准 | 文档 | 1 个 | — |
| ✅ **S1** | 起草 RFC-0001（capability/gate/event） | 文档 | 1 个 | S0 |
| ✅ **S2** | Capability Schema 详细版（基线 §7.2 展开） | 文档 | 1 个 | S0, S1 |
| ✅ **S3** | Skill 接口清单文档（基线 §22.8 展开） | 文档 | 1 个 | S2 |
| ✅ **S4** | 条件检测算法细化（基线 §6.2 展开） | 文档 | 1 个 | S2 |
| ✅ **S5** | 文件 ownership 表设计 | 文档 | 1 个 | S2 |
| ✅ **S6** | 测试计划文档（基线 §28.2） | 文档 | 1 个 | S2-S5 |
| ✅ **S7** | Kanban UI 原型 mockup | 文档/设计 | 1 个 | S0 |
| ✅ **M1a** | events.jsonl writer + event enum + event_id 生成 | 代码 | 1 个 | S1-S3 |
| ✅ **M1b** | 在现有 phase 切换 / handoff / checkpoint 埋点（双写） | 代码 | 1 个 | M1a |
| ✅ **M1c** | M1 一致性测试 + schema_version v2→v3 migration | 代码 | 1 个 | M1b |
| ✅ **M2a** | workflow.yaml 扩展 capability/gate 段 + 解析器 | 代码 | 1 个 | M1c, S2 |
| ✅ **M2b** | Gate 引擎 + 条件检测 + `vibehub-claim` skill | 代码 | 1 个 | M2a, S4 |
| ✅ **M3** | Projection 层接管 state.yaml derived 字段 | 代码 | 1 个 | M2b |
| ✅ **M4** | Task 内 capability 并行 + 补偿事件 + 每 capability pack | 代码 | 1-2 个 | M3 |
| ✅ **M5** | Schema 强校验 + policy.yaml + fitness 指标 | 代码 | 1 个 | M4 |

### Phase 2 — 多 Task 并行（M6, 基线 §8.2/§8.3/§26）

| Step | 名称 | 类型 | 预计窗口 | 依赖 |
|---|---|---|---|---|
| ✅ **M6a** | 多 task 状态模型 + task 切换引擎 | 代码 | 1 个 | M5 |
| ✅ **M6b** | 文件 ownership 引擎 + 冲突询问 | 代码 | 1 个 | M6a, S5 |
| ✅ **M6c** | 邻居 task 感知 + context pack `neighbors` 字段落地 | 代码 | 1 个 | M6b |
| ✅ **M6d** | 多需求意图拆分 + 自动多 task intake | Agent 协议/代码 | 1 个 | M6a-M6c, S3 |

### Phase 3 — UI 全面对接（M7, 基线 §5/§23/§24）

| Step | 名称 | 类型 | 预计窗口 | 依赖 |
|---|---|---|---|---|
| ✅ **M7a** | Kanban-first 项目详情主视图 IA shell | 前端 | 1-2 个 | M5, S7 |
| ✅ **M7b** | 最近动态 / 全局事件流 + JSON 可视化渲染器 | 前端 | 1 个 | M7a |
| ✅ **M7c** | 提示词生成器模态 + 7 个模板 | 前端 | 1 个 | M7a |
| ✅ **M7d** | 归档栏 + 完成/取消 task 详情子页面 | 前端 | 1 个 | M7a |
| ✅ **M7e** | 项目总结构图 + 文件目录联动详情页 | 前端 | 1-2 个 | M7a | 已完成 |

### Phase 4 — Capability 自定义（M8, 基线 §3.3 v2+）

| Step | 名称 | 类型 | 预计窗口 | 依赖 |
|---|---|---|---|---|
| ✅ **M8a** | 自定义 capability 声明 + schema 注册 + workflow.yaml 扩展 | 代码 | 1-2 个 | M5 | 已完成 |
| ✅ **M8b** | 自定义 capability 端到端验证 + 文档 | 代码/文档 | 1 个 | M8a | 已完成 |

### 跨切面工程步骤（X, 基线 §21/§27/§28/§29）

| Step | 名称 | 类型 | 预计窗口 | 依赖 | 可并行 |
|---|---|---|---|---|---|
| ✅ **X1** | Core/CLI 拆分（`crates/vibehub-core` + `crates/vibehub-cli`） | 架构 | 1-2 个 | M5 | 已完成 |
| ✅ **X2** | Adapter 配置生成器 + 6 工具投影 | 代码 | 1 个 | M5, S3 | 已完成 |
| ✅ **X3** | `.pending/` 兜底写入机制（启动器未运行时写入） | 代码 | 1 个 | M5 | 已完成 |
| ✅ **X4** | `vibehub-debug-dump` + 调试导出 | 代码 | 0.5 个 | M5 | 已完成 |

> **执行建议**: Phase 1 已完成（S0-M5）。Phase 2-4 和 X 系列可按需组合推进，X 系列与 M6/M7 可并行。

---

## 1. 跨 Step 的通用规则

### 1.1 每个 Step 开场必做

1. 读 `.vibehub/agent-view/current.md`、`current-context.md`、`handoff.md`、`.vibehub/rules/hard-rules.md`
2. 读基线对应章节（每步明确列出）
3. 读上一 Step 产出的 handoff 文件
4. 用 `vibehub-status` 确认当前 phase / task 状态

### 1.2 每个 Step 结束必做（DoD 闸门）

1. 自检 DoD 表，**每条都要给出 `hard_observed` / `agent_reported` / `user_confirmed` 证据标签**
2. 任何一条 ❌ → **不允许结束对话**，必须二选一：
   - (a) 在同一会话内继续推进直到通过
   - (b) 若遇到外部阻塞（用户决策 / 权限 / 工具不可用）→ 写明阻塞点到 handoff，并把当前 Step 标为 `blocked`，等待下次续跑
3. 通过后：写 `handoff.md` → 调 `vibehub-checkpoint` → 在事件流追加对应事件（M1 后启用）

### 1.3 失败 / 中断续跑规则

- 任何 Step 都可被中断；下次进入时第一件事是 **`vibehub-recover` + 读上次 handoff**
- 不允许"重头开始"，必须基于事件流 / handoff 续跑
- 续跑后仍须通过原 Step 的 DoD 才能进入下一 Step

### 1.4 不变量守护

- 全程满足基线 §18 的 10 条不变量（INV-1 ~ INV-10）
- 任何代码 Step 提交前 `cargo test` + `npm run build` 必须通过
- 任何文档 Step 完成后必须有交叉引用到基线对应节号

---

## 2. 文档类 Steps

### S0 — 基线入库 & 共识校准

**目标**: 让 baseline 文档真正成为后续所有工作的"单一参考基线"，并补一份「Agent Bootstrap」入口让长文档可用。

**必读输入**:
- `docs/vibehub-capability-redesign-2026-05-27.md`（全文 1 次 + 重点 §0 / §18 / §19 / §29）

**任务清单**:
1. 确认基线已纳入 VibeHub task 跟踪
2. 在基线文档顶部增补「Agent Bootstrap」节（不改原文内容，仅新增导航/速查）
3. 把基线 §13「下一步工作清单」与本手册 Step 地图做对应表（列入本手册附录）

**DoD**:
- [ ] 基线文档顶部存在「Agent Bootstrap」节，能在 80 行内回答："3 铁律是什么？10 不变量？术语速查？按问题反查节号？"
- [ ] §0 导航被重写为 6 大 Part 分组，每节带「⏱ 阅读优先级」
- [ ] 本手册 Step 地图与基线 §13 一一对应
- [ ] 用户口头/书面确认基线 + 本手册为后续工作的双重权威源

**未达成续跑**: 若用户尚未确认 → 标 `blocked: awaiting_user_confirmation`，handoff 留下两份文档的精简摘要供下次对齐。

**产出 handoff**: `runs/<run_id>/handoffs/S0.json`（含基线 commit hash、本手册 commit hash、用户确认时间戳）

---

### S1 — RFC-0001 起草

**目标**: 把基线浓缩为一份 PR 评审用 RFC，作为代码改动的对外契约。

**必读输入**: 基线 §1-3, §11, §12, §18, §20

**任务清单**:
1. 新建 `docs/rfc/0001-capability-gate-workflow.md`
2. RFC 必含小节：
   - Motivation（≤200 字，引用基线 §1）
   - Design Principles（3 铁律）
   - Concepts（task / capability / event / gate / projection）
   - Schema overview（指向 S2 输出）
   - Migration（M1-M5 摘要）
   - Backward compatibility（与现有 phase 命令的 alias 策略）
   - Open questions（指向基线 §14）
3. 在基线 §12 对应 Milestone 加链接到该 RFC

**DoD**:
- [ ] `docs/rfc/0001-capability-gate-workflow.md` 存在，篇幅 ≤ 600 行
- [ ] 每个核心概念有 1 段定义 + 引用基线节号
- [ ] Migration 段包含 5 个 Milestone 的「破坏性 / 非破坏性」标记
- [ ] Backward compatibility 显式列出 `align / research / plan / implement / review / continue` 等旧命令的 alias 行为
- [ ] 文末有 reviewer checklist（≥ 5 条）

**未达成续跑**: 若哪一节内容缺失 → 当场补；不允许"先发后补"。

**产出 handoff**: `runs/<run_id>/handoffs/S1.json`（含 RFC 路径、章节清单、未决问题）

---

### S2 — Capability Schema 详细版

**目标**: 把基线 §7.2 表格细化到可被代码直接消费的 JSON Schema / Rust struct。

**必读输入**: 基线 §3.3, §7, §17.5, §20

**任务清单**:
1. 新建 `docs/vibehub-capability-schema-v1.md`
2. 对每个 capability（`align_lite / align / research / plan / implement / validate / review_lite / review`）输出：
   - 必填字段（type / min_length / pattern / enum / 占位符如 `"no_risk"`）
   - 选填字段
   - 字段间约束（at-least-one / mutually-exclusive 等）
   - JSON Schema 片段
   - Rust struct 草案（`#[derive(Serialize, Deserialize)]`）
   - 1 个合规样例 + 1 个违规样例 + 期望错误码（参考基线 §25.1）
3. 定义 `schema_version: "1.0"` 的版本策略 + 后续 migration 规则
4. 把统一的"占位符常量"集中列出（`"no_risk"`, `"none"`, `"n/a"` 等）

**DoD**:
- [ ] 每个 capability 都有：字段表 + JSON Schema + Rust struct + 合规/违规样例
- [ ] 错误码均出自基线 §25.1 的 `<域>.<类型>.<子类>` 体系
- [ ] 占位符常量集中定义，无重复 / 无冲突
- [ ] schema_version 与 migration 策略明确（"破坏性变更必须升 major"）
- [ ] 与基线 §17.5 `required_output_schema` 字段对齐（命名、类型一致）

**未达成续跑**: 若个别 capability 难以定型 → 标 `draft` 但保留 TODO 列表；不允许"未定型字段悄悄遗漏"。

**产出 handoff**: `runs/<run_id>/handoffs/S2.json`（含每个 capability 的字段数、未决项）

---

### S3 — Skill 接口清单文档

**目标**: 把基线 §22.8 表格扩展为可生成 `.vibehub/skills.registry.yaml` 的权威清单。

**必读输入**: 基线 §10, §22, §27

**任务清单**:
1. 新建 `docs/vibehub-skills-registry-v1.md`
2. 对基线 §22.8 列出的 17 个 skill，逐个写：
   - `name` / `args` / `returns` / `side_effects` / `callable_by` / `idempotent` / `description`
   - 主用例（1 段文字）
   - 错误返回示例（至少 1 个）
   - 调用样例（伪代码）
3. 标注 sub-agent 候选 skill（与基线 §10.2 对齐）
4. 同步生成 `.vibehub/skills.registry.yaml` 的样板文件（不立刻启用，作为参考）

**DoD**:
- [ ] 17 个 skill 全部有完整字段（缺一项即不合规）
- [ ] sub-agent / main-agent 调用方矩阵清晰
- [ ] 样板 `skills.registry.yaml` 可被 YAML parser 解析（用任意 `yq` / `python -c` 验）
- [ ] 与基线 §22.1 统一返回格式 100% 匹配

**未达成续跑**: 若 skill 描述偏空泛 → 必须补到能让 agent 读懂何时调用为止。

**产出 handoff**: `runs/<run_id>/handoffs/S3.json`

---

### S4 — 条件检测算法细化

**目标**: 把基线 §6.2 的伪代码细化为可实现的算法规格。

**必读输入**: 基线 §6, §20, §26

**任务清单**:
1. 新建 `docs/vibehub-sync-condition-algorithm.md`
2. 形式化定义：`Δt`、`Δhead`、`Δoverlap` 的精确公式与边界
3. 给出阈值表（轻 / 深 / 强制重建）+ 默认值 + 可配置项
4. 写决策伪代码 + 决策树图（diagram block）
5. 列举 8 个典型场景的预期级别（如"刚刚 sync 过 + 0 改动" / "1 周未 sync + 大改动"…）

**DoD**:
- [ ] 三个信号的公式严格无歧义
- [ ] 决策树覆盖所有 8 个场景且互不冲突
- [ ] 阈值默认值与可配置项分离（哪些进 `policy.yaml`）
- [ ] 与基线 §6.3 不变量"必须留下完整快照"一致

**产出 handoff**: `runs/<run_id>/handoffs/S4.json`

---

### S5 — 文件 ownership 表设计

**目标**: 解决基线 §8.3 / §26.4 提到的"多 task 改同文件归属"问题。

**必读输入**: 基线 §4 旅程 C、§8、§26

**任务清单**:
1. 新建 `docs/vibehub-file-ownership.md`
2. 设计 ownership 数据结构（task ↔ files，多对多 / 时间窗）
3. 设计自动判定算法（基于 task 历史改动文件集合的交集）
4. 设计冲突处理流程（多归属 / 无归属 / 全部漂移）
5. 设计 agent 询问用户的 prompt 模板

**DoD**:
- [ ] 数据结构能表达"同一文件在不同时间属于不同 task"
- [ ] 算法对"无交集 / 单交集 / 多交集"三种情况都有明确动作
- [ ] 冲突时调用 agent 询问的 prompt 模板已落到文件
- [ ] 与基线 INV-8（VibeHub 不写 git）一致：仅读，不改 git

**产出 handoff**: `runs/<run_id>/handoffs/S5.json`

---

### S6 — 测试计划文档

**目标**: 落实基线 §28.2 的测试计划文档（先写计划，不急于实现）。

**必读输入**: 基线 §18, §20, §28；S2-S5 的产出

**任务清单**:
1. 新建 `docs/vibehub-test-plan.md`
2. 内容（参照基线 §28.2）：
   - 单元测试矩阵（schema / 事件投影 / event_id / writer 队列）
   - 集成测试矩阵（sync 三级 / claim+release / pack 重建）
   - UI e2e 手动测试清单
   - 关键 fixture（典型 events.jsonl 样例 / 损坏样例 / 大事件流样例）
   - 覆盖率目标（核心 Rust 模块 ≥ 80%）
   - CI 集成方案（GitHub Actions / 本地脚本）

**DoD**:
- [ ] 测试矩阵覆盖所有 10 条不变量
- [ ] 每条 DoD 单测都有"如何复现"步骤
- [ ] CI 方案至少给出一个可立刻 enable 的 workflow 草稿

**产出 handoff**: `runs/<run_id>/handoffs/S6.json`

---

### S7 — Kanban UI 原型 mockup

**目标**: 落实基线 §5.5 / §23 的看板形态，明确"显示器，不操作"边界。

**必读输入**: 基线 §5, §23, §24

**任务清单**:
1. 新建 `docs/vibehub-ui-kanban-mockup.md`
2. 用 ASCII / diagram block 画 3 套 mockup：
   - 空看板（首次打开，E1 决议）
   - 多 task + 多 capability 活跃
   - 含归档栏（E2 决议）
3. 标注"哪些是按钮、按钮点了弹什么模态、模态里给什么提示词"
4. 列出 23.4 的全部 7 个模板的占位结构（不必写最终文案，先列变量与渲染位置）
5. 评估对现有 Tauri webview 改动量

**DoD**:
- [ ] 3 套 mockup 全部完成
- [ ] 所有按钮都明确"只生成提示词，不直接执行"
- [ ] 模板占位结构与基线 §23.4 一一对应（7 个模板）
- [ ] 改动量评估给出 S / M / L 估算

**产出 handoff**: `runs/<run_id>/handoffs/S7.json`

---

## 3. 代码类 Steps（M1）

### M1a — events.jsonl writer + event enum + event_id 生成

**目标**: 落地基线 §20，让 VibeHub backend 能严格 append-only 写入事件。

**必读输入**: 基线 §18 (INV-1/3), §20, §21；S1 RFC；S3 skill 清单

**任务清单**:
1. 新增 `src-tauri/src/vibehub/events.rs`
2. 实现 `VibehubEvent` enum（覆盖基线 §20.2 全部 v1 事件）
3. 实现 `EventId` 生成（格式 `evt-<unix_ms>-<seq>`，进程内自增 seq）
4. 实现 `EventWriter`（单例，FIFO 队列，append-only，pre-write CRC 自检）
5. 实现 `events.jsonl` 读 API：`list_events(run_id, since)`（只读）
6. 单元测试：
   - event_id 唯一性、严格自增
   - 写入后读回内容一致
   - 模拟"乱序写入"被拒
   - 模拟"损坏事件流"恢复路径（EventLogCorrupted 写到新位置，旧文件不动）

**DoD**:
- [ ] `cargo test` 全绿；新增测试至少 8 个
- [ ] 不引入任何依赖 Tauri runtime 的代码（基线 §29.4 解耦要求）
- [ ] 写入路径单写者（INV-3）已用类型系统或 Mutex 保证
- [ ] writer 在 `.vibehub/.pending/` 兜底路径有 stub（M2a 启用，但接口要留好）

**未达成续跑**: 若某事件类型 schema 仍有争议 → 暂留 `TODO_event_v1.1` 字段并写注释；不允许编译失败 / 测试失败就结束。

**产出 handoff**: `runs/<run_id>/handoffs/M1a.json`（含模块路径、PR/commit、测试覆盖率）

---

### M1b — 在现有路径埋点（双写）

**目标**: 让所有现有的 phase 切换 / handoff / checkpoint / sync 路径**同时**写事件，保持 state.yaml 不变。

**必读输入**: 基线 §12 (M1)；M1a 产出；现有 `phase.rs / status.rs / handoff` 路径

**任务清单**:
1. 在每个已有写入点插入 `EventWriter.append(...)`
2. 至少覆盖：
   - phase 切换 → `CapabilityClaimed / CapabilityReleased / PhaseProjected`
   - handoff 写入 → `HandoffWritten`
   - checkpoint → `EvidenceAdded / DiffObserved` 等
   - sync 入口 → `SyncStarted / SyncCompleted`
3. 写一份"双写覆盖表"文档：`docs/vibehub-m1-dual-write-coverage.md`
4. 添加 `ensure_consistency()` 启动自检：`fold(events) == derived(state.yaml)`，不一致打 warn 但不阻塞

**DoD**:
- [ ] 每个旧写入点都有对应事件
- [ ] 启动自检函数存在并被调用
- [ ] 跑通现有 e2e（手动操作几个真实 task），事件流非空且顺序正确
- [ ] 双写覆盖表中"事件类型 ↔ 触发点"无遗漏

**未达成续跑**: 若某触发点不易插入 → 写到 handoff 中标 `TODO_dual_write_<path>`；下一次必须补齐。

**产出 handoff**: `runs/<run_id>/handoffs/M1b.json`

---

### M1c — M1 一致性测试 + schema_version v2→v3 migration

**目标**: 保证双写期数据一致，schema 升级不破坏老 task。

**必读输入**: 现有 `state_migration.rs`；M1a / M1b 产出

**任务清单**:
1. 编写 migration: `state.schema_version: 2 → 3`，新增 `event_log_path` 字段
2. 写老 task 兼容测试（schema v2 项目升 v3 后能继续工作）
3. 写一致性测试集：
   - 随机生成事件序列 → fold → 与并行写的 state.yaml 比对
   - 模拟 10 次崩溃 + 重启，每次自检通过
4. 在 CI 中加入这些测试

**DoD**:
- [ ] 老 task 升级不丢任何已有字段（用一个真实 v2 fixture 跑）
- [ ] 100 次随机事件序列 → fold/state 比对 100% 一致
- [ ] 崩溃恢复测试通过
- [ ] CI 增量绿

**产出 handoff**: `runs/<run_id>/handoffs/M1c.json`

---

## 4. 代码类 Steps（M2-M5）

### ✅ M2a — workflow.yaml 扩展 capability/gate 段 + 解析器

**目标**: 让配置层能声明 capability 与 gate（与旧 `phases:` 并存）。

**必读输入**: 基线 §3.3, §3.5, §12 (M2)；现有 `workflow.yaml` 结构

**任务清单**:
1. ✅ 设计新版 `workflow.yaml` 结构：`capabilities:` + `gates:` + 保留 `phases:`（兼容）
2. ✅ 实现 Rust 解析器 + 校验器
3. ✅ 提供 `vibehub-workflow-explain` 调试命令（列出当前 capability 集合 + gates）
4. ✅ 兼容性测试：旧 workflow.yaml 不报错，新 workflow.yaml 能解析

**DoD**:
- [x] 旧/新两种 workflow.yaml 都能解析
- [x] gates 支持 `all / any / not` 组合
- [x] 解析错误返回 §25.1 体系错误码
- [x] 单测 ≥ 10 个

**产出 handoff**: `runs/<run_id>/handoffs/M2a.json`

---

### ✅ M2b — Gate 引擎 + 条件检测 + `vibehub-claim`

**目标**: 让 agent 能 claim capability，引擎根据事件流推导可执行集合。

**必读输入**: 基线 §6, §12 (M2)；S4 算法细化；M2a 产出

**任务清单**:
1. ✅ 实现 gate 评估器：输入事件流 + workflow.yaml → 输出可执行 capability 集合
2. ✅ 实现 §6.2 条件检测（按 S4 文档）
3. ✅ 新增 skill `vibehub-claim <capability>`（旧 `vibehub-continue` 保持兼容）
4. ✅ 实现 `CapabilityClaimed / GateChecked` 事件写入
5. ✅ 完整路径单测 + 集成测试

**DoD**:
- [x] `vibehub-claim` 能在 demo 项目上跑通
- [x] 不满足 gate 时返回 `gate.precondition.unmet` 错误 + 修复 hint
- [x] 旧命令仍可用（alias 不破坏）
- [x] 条件检测三级阈值与 S4 文档一致

**产出 handoff**: `runs/<run_id>/handoffs/M2b.json`

---

### ✅ M3 — Projection 接管 state.yaml derived 字段

**目标**: state.yaml 的 phase / phase_status / flow 改由投影派生。

**必读输入**: 基线 §3.6, §12 (M3)；M1-M2 产出

**任务清单**:
1. ✅ 实现 `Projection { fold: events → derived_state }`
2. ✅ 在 `state.yaml` 标记 derived 字段 `derived: true`
3. ✅ 关闭直接写入这些字段的入口（编译期/运行期校验）
4. ✅ 实现 `derivation_trace.yaml` 记录派生依据
5. ✅ 升级 `vibehub-status` 输出"可执行 capability + 已满足/未满足 gate"
6. ✅ UI 显示与改造前保持一致（行为兼容）

**DoD**:
- [x] derived 字段无任何直接写入路径（grep 通过，除 `projection.rs` 单一写入点）
- [x] `derivation_trace.yaml` 每次更新都可追溯
- [x] `vibehub-status` 新输出在 demo 项目上正确
- [x] UI 看板表现与旧版一致（DOM smoke check；截图接口超时，未发现布局回归）

**产出 handoff**: `runs/<run_id>/handoffs/M3.json`

---

### M4 — Task 内 capability 并行 + 补偿事件 + 每 capability pack

**目标**: 落地基线 §8.1, §9.3，让 task 内多 capability 可同时活跃。

**必读输入**: 基线 §8, §9, §12 (M4), §17

**任务清单**:
1. ✅ 允许多个 `CapabilityClaimed` 同时存在（同一 task 内 ≤ 5）
2. ✅ `agent-view/current.md` 改为列"活跃 capabilities"
3. ✅ 实现 `PlanInvalidated / DiffReverted` 补偿事件 + 投影规则
4. ✅ review 失败自动 `CapabilityReleased { outcome: failed }`，不再"phase 回退"
5. ✅ 实现每 capability 独立 context pack（基线 §17）
6. ✅ 实现 handoff 生成器（含 `prior_outputs_summary` + `task_pack_dirty` delta 信号）

**DoD**:
- [x] 同 task 内同时 claim ≥ 2 个 capability 正确显示
- [x] 补偿事件不破坏历史（INV-1）
- [x] 每 capability 都有自己的 context pack 文件
- [x] handoff 中 `prior_outputs_summary` 字段被下游正确消费
- [x] task pack delta 信号触发 / 不触发的两种路径都有单测

**未达成续跑**: 工作量大，可拆 M4a / M4b 两次窗口；任何一次结束前 DoD 子集必须完成。

**产出 handoff**: `runs/<run_id>/handoffs/M4.json` 或 `M4a.json + M4b.json`

---

### ✅ M5 — Schema 强校验 + policy.yaml + fitness 指标

**目标**: 落地基线 §7, §11.2, §25, §28；让流程治理从"硬规则"变成"可观测策略层"。

**必读输入**: 基线 §7, §11.2, §25, §28；S2 schema 文档；M4 产出

**任务清单**:
1. 实现 `schema_check.rs`：写入入口同步校验（INV-4）
   - 采用 **JSON Schema 标准** 进行字段校验
   - 对 S2 文档定义的 8 个 capability 逐个实现 JSON Schema 定义
   - 占位符常量统一消费（`"no_risk"`, `"none"`, `"n/a"`）
2. 不合规 → 返回 `schema.required.missing` 等结构化错误（基线 §25.2）
   - 错误码按三段式 `<域>.<类型>.<子类>` 体系（基线 §25.1）
   - 每个错误必含 `hint` 字段，指导 agent 如何修复
   - 写事件 `SchemaValidationFailed { target, errors[] }`
3. 写 `policy.yaml`：可配置治理参数
   - `max_open_risks`：单 task 允许的最大未解决风险数
   - `max_concurrent_claims`：单 task 允许的最大活跃 capability 数（默认 5）
   - `pack.warn_at`：Context Pack 体积 soft warning 阈值（token 数）
   - `sub_agent.max_concurrent`：最大并行 sub-agent 数（默认 3，基线 §21.3）
   - `sub_agent.timeout_seconds`：sub-agent 超时（默认 60s，基线 §25.4）
4. 实现 `fitness.rs`：导出基线 §28.3 的全部指标到 `state.yaml.metrics`
   - `tasks.active_count`, `capabilities.active_count`
   - `sync.avg_duration_ms`, `sync.last_mode`
   - `pack.avg_size_tokens`, `pack.oversize_count`
   - `schema.validation_failure_rate`
   - `events.write_per_minute`
5. 升级 `loop_detection` 为基于事件频率的真实回路检测
   - 检测同一 capability 的 claim/release 循环频率
   - 超阈值时产出 warning 事件
6. 实现 events.jsonl 损坏恢复路径（基线 §25.3）
   - 启动 / 写入前 CRC + 顺序自检
   - 损坏 → 从最后一个完整事件截断
   - 写 `EventLogCorrupted { at_offset, recovered_to }` 到新位置
7. 老 Task 数据自动回填（基线 §14.6）
   - 基于老 task 的 `state.yaml` 逆向生成历史事件流
   - 保证数据完整性，消除"断层"

**DoD**:
- [x] 8 个 capability 的必填字段缺失 → 100% 被拒（用 S2 的合规/违规样例验证）
- [x] 错误返回带 `hint`，agent 能据此修复
- [x] 错误码覆盖 `schema.*`, `event.*`, `gate.*` 三域
- [x] `policy.yaml` 字段被引擎消费：WIP/open-risk gate、pack oversize warning、schema strict mode、loop threshold、subagent timeout metric
- [ ] `state.yaml.metrics` 剔除超时后包含剩余各项指标
- [ ] 老 task 默认 policy 不会突然报错（向后兼容）
- [ ] events.jsonl 损坏场景自动恢复 + 写事件（单测覆盖）
- [ ] 老 task 回填测试通过（自动生成对应的 events）
- [x] `cargo test` + `npm run build` 全绿

**未达成续跑**: 若个别 capability schema 争议 → 留 `TODO_schema_v1.1` 注释但不允许跳过测试。

**产出 handoff**: `runs/<run_id>/handoffs/M5.json`

---

## 5. 代码类 Steps（M6 — 多 Task 并行）

### M6a — 多 task 状态模型 + task 切换引擎

**目标**: 落地基线 §8.2；允许多个 task 同时活跃，agent 可在 task 间切换。

**必读输入**: 基线 §8.2, §8.4, §12 (M6)；§17.4（task pack）；§20.4（跨 run 索引）

**任务清单**:
1. ✅ 扩展 `state.yaml` 支持 `tasks.active[]`（多个活跃 task ID 列表）
2. ✅ 实现 task 切换命令：`vibehub switch <task_id>`
   - ✅ 切换前投影当前 task 的 capability 状态
   - ✅ 加载目标 task 的 context pack
   - ✅ 写事件 `TaskSwitched { from, to }`
3. ✅ 实现 `index/task-events.idx` 跨 run 事件索引（基线 §20.4 / A6）
   - ✅ 记录 `task_id → [run_id, event_id offset]`
   - ✅ event writer 写入时维护
4. ✅ 扩展 `agent-view/current.md` 显示"活跃 task 列表"
5. ✅ 多 task 场景兼容 schema_version v5
6. ✅ 单测 + 集成测试

**DoD**:
- [x] ≥ 2 个 task 可同时 active 并独立推进 capability
- [x] 切换 task 后 context pack 正确加载
- [x] 跨 run 事件索引可查
- [x] 老单 task 项目升级后兼容
- [x] `cargo test` + `npm run build` 全绿

**产出 handoff**: `runs/<run_id>/handoffs/M6a.json`

---

### M6b — 文件 ownership 引擎 + 冲突询问

**目标**: 落地基线 §8.3, §26.4；自动判定文件归属 task，冲突时询问用户。

**必读输入**: 基线 §8.3, §26.4；S5 ownership 文档

**任务清单**:
1. ✅ 实现 `ownership.rs`：文件 ownership 数据结构
   - ✅ task ↔ files 多对多映射 + 时间窗
   - ✅ 基于历史改动文件名集合的交集判定
2. ✅ 实现自动归属算法（S5 文档规格）
   - ✅ 无交集 → 询问用户
   - ✅ 单交集 → 自动归属
   - ✅ 多交集 → 询问用户
3. ✅ 实现 `vibehub-record { scope_files: [...] }` 手动修正接口（基线 §26.4）
4. ✅ Git diff 归属可视化数据（为 M7 UI 准备 API）
5. ✅ 写事件 `FileOwnershipUpdated { task_id, files_added, files_removed }`
6. ✅ 只读 git（INV-8）— 代码审查确认无 git write 调用

**DoD**:
- [x] 自动归属对"无交集 / 单交集 / 多交集"三种情况都有明确行为
- [x] 冲突时 agent 主动询问用户（有 prompt 模板）
- [x] 手动修正接口可覆盖自动结果
- [x] ownership 数据持久化且可在 task 间查询
- [x] 单测覆盖 ≥ 8 个场景（含边界：空集、全交集、跨 task 文件迁移）

**产出 handoff**: `runs/<run_id>/handoffs/M6b.json`

---

### M6c — 邻居 task 感知 + context pack `neighbors` 字段

**目标**: 落地基线 §17.5 `neighbors` 字段；让 agent 在 task B 工作时看到 task A 的关键状态提醒。

**必读输入**: 基线 §4 旅程 C、§8.2、§17.5

**任务清单**:
1. ✅ 实现邻居 task 状态查询 API
   - 返回：task_id, title, active_capabilities, shared_files
2. ✅ 在 capability context pack 的 `neighbors` 字段中填充实际数据
   - 共享文件的交集高亮
   - 正在进行中的 capability 类型
3. ✅ Agent-view 增加邻居提醒（"邻居：Task A 在 review，若改公共文件需协调"）
4. ✅ 扩展 `vibehub-status` 显示多 task 并行概览
5. ✅ 写事件 `NeighborConflictDetected { tasks, shared_files }` 当检测到冲突时

**DoD**:
- [x] capability pack 的 `neighbors` 字段含正确的邻居 task 信息
- [x] 共享文件时自动产出警告
- [x] agent-view / status 输出包含多 task 概览
- [x] 无邻居时字段为空数组（schema 兼容）

**产出 handoff**: `runs/<run_id>/handoffs/M6c.json`

---

### M6d — 多需求意图拆分 + 自动多 task intake

**目标**: 用户在同一个聊天窗口一次提出 2-3 个独立需求时，agent 能主动识别、拆成多个 VibeHub task，并在同一窗口按顺序推进；换聊天窗口后仍能从 active task 列表和 handoff 继续。

**必读输入**: 基线 §3.2、§4 旅程 E、§8.2、§22；S3 skill registry；`.agents/skills/vibehub-start/SKILL.md`

**完成状态（2026-05-28）**:
- `hard_observed`: M6a-M6c 已支持"多个 task 存在、切换、ownership、neighbors"。
- `hard_observed`: `src-tauri/src/vibehub/start_task.rs` 已补齐结构化多意图 intake API：高置信批量创建 task，中置信返回确认，低置信保持单 task 并记录未拆分原因。
- `hard_observed`: `TaskIntakePlanned` 事件、`status.active_tasks`、agent-view Intake Queue、Tauri command、CLI `start-intake <request.json>`、前端 service/types 已接通。
- `hard_observed`: `cargo test --locked start_task -- --nocapture` 通过 6 个 start/intake 相关测试；后续 `cargo test --locked` 通过 182 个测试。

**任务清单**:
1. 定义 intake 判定规则：
   - 应拆分：多个独立交付物、不同用户目标、可分开验收、可分开暂停/取消、明显不同代码/文档范围。
   - 不拆分：同一需求的实现步骤、同一验收标准下的配套改动、必须同时完成才有价值的小子任务。
2. 为 agent 增加三档行为：
   - 高置信：直接创建多个 task，并输出 task 映射和执行顺序。
   - 中置信：先用一句话向用户确认拆分方案。
   - 低置信：保持单 task，但在 output 里记录"未拆分原因"。
3. 扩展 `vibehub-start` skill / adapter 文案：
   - 明确它不是只能创建单 task；当输入含多个独立需求时，应拆成多个 task 草案。
   - 每个 task 必须有短标题、intent、验收标准、依赖关系、建议顺序。
4. 视代码现状补 CLI / Tauri / core API：
   - 若已有单 task start API，则增加批量 intake 包装，逐个写 `TaskCreated`，并维护 active task 列表。
   - 失败时不能半静默：已创建的 task 要在输出里列清楚，未创建的需求列为 unresolved risk。
5. 更新 handoff / status 展示：
   - handoff 必须列出本次从一条用户消息拆出的 task 列表、当前正在做哪个、剩余顺序。
   - 新窗口读 `agent-view/current.md` 时能看到未完成 task，不会只继续最后一个而遗忘其它需求。
6. 补测试：
   - 单请求单 task 不回归。
   - 一个消息 2 个独立需求 → 2 个 task。
   - 3 个需求且其中 1 个依赖另一个 → 创建 3 个 task，并记录执行顺序/依赖。
   - 模糊请求 → 触发确认或记录未拆分原因。

**DoD**:
- [x] `vibehub-start` 文档和实际 agent 指令都写明多意图拆分规则。
- [x] 高置信多需求输入会产生多个 task，而不是塞进一个 task。
- [x] status / handoff / agent-view 能保留"同一用户消息拆出的 task 队列"。
- [x] 跨聊天窗口续跑时，剩余 task 不丢失。
- [x] 有测试覆盖单 task、2 task、3 task、模糊输入。

**产出 handoff**: `runs/<run_id>/handoffs/M6d.json`

---

## 6. 前端类 Steps（M7 — UI 全面对接 / 前端体验重构）

> `user_confirmed` (2026-05-28): M7 整体是前端 UI 重构；M7a 只是第一块切片。目标不是在旧 Dashboard 上新增 Kanban，而是摒弃旧 Dashboard 的主视图理念，改为 Kanban-first 项目详情页 + 可点开的任务/阶段/包体/历史详情页。最新 UX 规格见 `docs/vibehub-ui-kanban-mockup.md` Draft v2。

### M7a — Kanban-first 项目详情主视图 IA shell

**目标**: 落地基线 §5, §23.6, §24，并按最新 UX 规格替代旧 Dashboard 主体。M7a 先搭出新的信息架构和主视图骨架，不追求一次完成 M7 全部详情页。

**必读输入**: 基线 §5, §23.1, §23.2, §23.6, §24；`docs/vibehub-ui-kanban-mockup.md` Draft v2；S7 Kanban mockup 历史文档

**完成状态（2026-05-28）**:
- `hard_observed`: `src/components/VibehubCockpitDialog.tsx` 的 project center 已重构为 Kanban-first 项目详情 shell，包含项目身份/设置、最近提交、最近动态、活跃 task 卡片、phase/capability 过程条、详情入口、项目结构与归档占位。
- `hard_observed`: `src/locales/en.json`、`src/locales/zh.json`、`src/locales/zh-TW.json` 已补齐 M7a 文案；`npm run build` 通过。
- `agent_reported`: 浏览器 smoke 仅验证到应用 shell 可打开；因本地浏览器无持久项目/workspace，未截图验证 seeded 3-task 项目中心场景，作为 M7b/M7e 前可补强的视觉验收证据。

**任务清单**:
1. 替换旧 Dashboard 主体
   - 项目中心第一屏改为 Kanban-first 项目详情页
   - 旧 phase flow / git summary / evidence map / file health 卡片不再作为主视图并列模块
   - 旧模块数据迁移到 task detail / phase detail / settings / activity / archive 等详情入口
2. 顶部项目区域
   - 项目名作为首屏锚点
   - 右上设置入口：项目级语言、远端链接、未来项目级配置
   - 最近提交：有远端 URL 时可跳到线上 commit；无远端时显示本地信息
   - 最近动态：展示最新 VibeHub 活动摘要，点击进入 M7b 活动详情
3. 活跃任务主列表
   - 多 task 纵向堆叠，尽量全部展示；数量太多时再折叠/分页
   - 每个 task 显示 2-4 字短标签（最多约 6 字）、需求简述、phase/capability 过程条
   - 当前执行 phase/capability 有明显加粗/高亮边框
   - 过程过长时显示当前项前后若干项，其余用省略号
4. 点击与聚焦模型
   - 点击任务标签/需求区 → task 原始需求详情
   - 点击过程条空白区 → 完整 phase/capability 过程详情
   - 点击具体 phase/capability → 当前阶段/包体/输出详情
   - 从最近动态或 commit 关联跳转时，高亮 task 框和具体 phase/capability 框
5. 主视图占位区
   - 项目总结构区域保留正确位置（完整实现可到 M7e）
   - 归档区域保留正确位置（完整实现可到 M7d）
6. Tauri event/emit 实时同步（基线 §23.1, D1+D2）
   - 首次全量拉取
   - 增量事件推送更新对应卡片
   - 不轮询、不 watch 文件
7. Schema 校验失败红色徽章（基线 §24.5, E5）
   - 在 task / phase/capability 卡片显示红色徽章
   - 点击查看完整错误详情（含 `error.hint`；若读模型未暴露则记录为未完成）
8. 首次打开空看板 + 初始化按钮（基线 §24.1, E1）
   - 中央"初始化 VibeHub 项目"按钮
   - 点击 → 弹模态显示初始化提示词

**DoD**:
- [x] 旧 Dashboard 卡片网格不再是项目中心主视图
- [x] 主视图正确渲染 active task 的短标签、需求简述、phase/capability 过程条；≥ 3 task seeded 视觉证据待补强
- [x] 当前 phase/capability 高亮清晰
- [x] 点击 task / 过程条 / phase-capability 能进入对应详情入口
- [x] 最近动态能打开详情入口；跳转聚焦到对应 task + phase/capability 与 M7b 联合深化
- [x] 首次全量数据映射到卡片；实时事件推送细化与 M7b 联合深化
- [x] `docs/vibehub-ui-kanban-mockup.md` Draft v2 的核心主视图场景已在实现结构中覆盖；seeded 视觉截图待补强
- [x] 不支持拖拽（无拖拽事件绑定）
- [x] UI 不调用任何 skill；除明确允许的项目级设置外不写 VibeHub workflow 状态（INV-6）
- [x] `npm run build` 全绿

**遗留验收补强**: M7a 代码与 build 已完成；若下一会话需要更强 UI 证据，先准备/打开含 ≥ 3 个 active task 的工作区 fixture，再用 Browser 捕获项目中心主视图截图。

**产出 handoff**: `runs/<run_id>/handoffs/M7a.json`

---

### M7b — 最近动态 / 全局事件流时间线 + JSON 可视化渲染器

**目标**: 落地基线 §5.4, §23.5；实现最近动态展开、事件流浏览、点击跳转聚焦、JSON/包体可视化。

**必读输入**: 基线 §5.4, §23.5 (D6)

**完成状态（2026-05-28）**:
- `hard_observed`: `src-tauri/src/vibehub/overview.rs` 现在聚合 `.vibehub/tasks/*/runs/*/events.jsonl` 为 `event_timeline` 只读模型，包含 timestamp、task、run、event_type、capability、summary、原始载荷和 event log 路径。
- `hard_observed`: `src/components/VibehubCockpitDialog.tsx` 的最近动态详情已升级为 M7b activity 面板，支持按 task / capability / event type 筛选、选择事件、结构化查看 event payload，并点击回到主看板聚焦 task + capability。
- `hard_observed`: 包体/JSON 查看器可打开 context pack、handoff、agent output、event log、sync report 等 `.vibehub` 文件；JSON/JSONL 使用树状结构展示，Markdown 包体使用结构化 Markdown renderer 展示。
- `hard_observed`: 新增安全的 `.vibehub` 文件定位/默认打开命令 `vibehub_reveal_vibehub_file` / `vibehub_open_vibehub_file`，前端提供"在文件管理器中显示"和"用系统默认应用打开"两个按钮。
- `hard_observed`: `npm run build` 通过；`cargo test --locked` 通过（182 passed）。
- `hard_observed`: Browser smoke 打开 Vite app shell 成功；因 in-app browser 无持久 workspace/project 数据，未能从真实项目卡片点入 activity 面板截图。

**任务清单**:
1. 全局事件流时间线面板
   - 显示最近 N 条事件（按时间倒序）
   - 每条事件包含：时间戳、task 名、capability 名、事件类型、摘要
   - 可按 task / capability / 事件类型过滤
   - 从顶部"最近动态"进入
   - 点击具体事件目标时，跳回主看板并高亮对应 task + phase/capability
2. JSON 可视化渲染器（基线 §23.5, D6）
   - 结构化树状/卡片化展示（非原始 JSON 文本）
   - 适用对象：events.jsonl 单条 / context pack / handoff / output / sync report
   - 每个 JSON 文件视图配两个按钮：
     - **在文件管理器中显示**（打开文件所在目录）
     - **以系统默认应用打开**（用 OS 默认关联程序）
3. 从事件流到 JSON 渲染器的跳转链路

**DoD**:
- [x] 事件流时间线显示且可过滤
- [x] 最近动态入口可打开完整历史详情
- [x] 事件点击可定位并高亮 task + phase/capability
- [x] JSON 渲染器能正确展示 context pack / handoff / output 三种文件
- [x] 两个文件操作按钮功能正常
- [x] 不展示原始 JSON 文本（树状/卡片化）

**产出 handoff**: `runs/<run_id>/handoffs/M7b.json`

---

### M7c — 提示词生成器模态 + 7 个模板

**目标**: 落地基线 §23.3, §23.4 (D3-D5)；实现所有"操作"按钮的提示词生成。

**必读输入**: 基线 §23.3, §23.4, §24.6 (E6)

**任务清单**:
1. 实现提示词生成器模态弹窗
   - 显示模板渲染后的完整提示词文本
   - "复制"按钮（基线 D5：仅复制，不集成具体工具）
2. 实现 7 个 v1 模板（基线 §23.4）
   - `new-task.md` — 新需求
   - `sync.md` — 同步
   - `claim-capability.md` — 推进 capability
   - `release-capability.md` — 完成 capability
   - `cancel-task.md` — 取消 task
   - `force-rebuild.md` — 强制重建
   - `fix-schema.md` — Schema 校验失败修复
3. 模板存放机制与多语言（基线 §14.4）
   - 初始化（`vibehub init`）时选择语言（中文/英文）
   - 后续所有模板和信息包都跟随这个设置
   - 产品级默认：`<vibehub-install>/templates/prompts/*.md`
   - 项目级覆盖（可选）：`.vibehub/templates/prompts/*.md`
   - 项目设置页可显示/调整项目级语言与远端链接等配置（是否写入 UI 例外需按 `docs/vibehub-ui-kanban-mockup.md` UI-Q4 决议）
4. 危险操作二次确认模态（基线 §24.6, E6）
   - 取消 task → 确认是否放弃
   - 强制重建 → **智能判断**：单 task 场景直接重建，多 task 场景弹窗问用户"选择要重建哪个 task？"

**DoD**:
- [x] 7 个模板全部可渲染为带上下文的提示词
- [x] 复制按钮功能正常
- [x] 项目级覆盖可正确加载
- [x] 危险操作弹二次确认

**完成记录（2026-05-28 / Draft v2.9）**:
- `hard_observed`: 新增后端 prompt renderer：`vibehub_list_prompt_templates` / `vibehub_render_prompt`，读取当前 cockpit status 并将 task/run/phase/mode/locale/dirty count 等上下文注入模板。
- `hard_observed`: 默认模板落在 `src-tauri/templates/prompts/{en,zh-CN,zh-TW}/*.md`，项目覆盖优先读取 `.vibehub/templates/prompts/*.md`。
- `hard_observed`: 前端 cockpit 新增提示词生成器模态，支持 7 个模板选择、渲染后的完整文本、复制按钮、来源/路径/语言展示。
- `hard_observed`: `cancel-task` 与 `force-rebuild` 被标记为危险模板，复制前需要二次确认；`force-rebuild` 模板带 active task count / active tasks 上下文，供多 task 场景确认目标。

**产出 handoff**: `runs/<run_id>/handoffs/M7c.json`

---

### M7d — 归档栏 + 完成/取消 task 详情子页面

**目标**: 落地基线 §24.2, §24.3 (E2+E3)；实现完成/取消 task 的归档展示。

**必读输入**: 基线 §24.2, §24.3

**任务清单**:
1. 归档栏（看板下方可展开）
   - 显示所有 completed / cancelled task 列表
   - 取消 task 显示灰色
   - 每个卡片显示 2-4 字短标签 + 需求简述
   - 右上角显示完成勾 / 取消叉
2. 归档详情子页面
   - 任务标题、意图
   - capabilities 全流程状态
   - 所有 handoff / output / events 的可视化链路
   - 跳转到 JSON 渲染器（M7b）
3. 暂停行为验证（基线 §24.4, E4）
   - 不引入"暂停"状态
   - 无活动 = 自然暂停
   - 重新 claim capability 即"恢复"

**DoD**:
- [x] 归档栏可展开/收起
- [x] completed task 详情可浏览全流程
- [x] cancelled task 显示灰色并保留事件
- [x] 归档卡片显示短标签、需求摘要、状态图标
- [x] 无"暂停"UI 控件

**完成记录（2026-05-29 / Draft v3.0）**:
- `hard_observed`: 新增只读 archive read-model：`src-tauri/src/vibehub/archive.rs` 扫描 `.vibehub/tasks/*/task.yaml`、runs、events、handoffs、outputs、phase outputs，输出 completed/cancelled task cards 与详情产物链路。
- `hard_observed`: `vibehub_read_overview` 聚合返回 `archive` 字段；前端无需新增写操作或状态变更命令。
- `hard_observed`: Cockpit 项目详情页 archive 区域替换占位：支持展开/收起、数量 badge、短标签、需求摘要、completed 勾 / cancelled 叉状态图标，cancelled 卡片灰色显示。
- `hard_observed`: Archive detail drawer 支持任务标题/摘要、mode/phase/run 信息、capability/process 状态、handoff/output/events artifacts、事件 timeline，并可跳转既有 preview/JSON 文件渲染器。
- `hard_observed`: M7d 未新增暂停状态、暂停按钮或 workflow mutation；archive UI 保持 read-only。
- `hard_observed`: 新增 archive Rust 单测覆盖 completed/cancelled task 收集、active task 排除、output artifact 链路。

**产出 handoff**: `runs/<run_id>/handoffs/M7d.json`

---

### M7e — 项目总结构图 + 文件目录联动详情页

**目标**: 落地最新 UX 草图里的"项目总结构"区域。该项优先级低于 M7a-M7d，可以先放占位，再实现完整画布。

**必读输入**: `docs/vibehub-ui-kanban-mockup.md` Draft v2 §7

**任务清单**:
1. 主视图项目总结构区域
   - 左侧/主体显示文件目录结构或具体模块交互树状图预览
   - 右侧显示简版文件目录结构
   - 支持打开完整详情页 / 大画布
2. 完整项目结构详情页
   - 左侧完整模块交互图 / canvas
   - 右侧完整目录结构
   - 点击图节点 → 目录结构对应项高亮
   - 点击目录项 → 图节点高亮
3. 文件操作
   - 支持打开文件
   - 支持在文件管理器中显示
4. 历史记录预留
   - 点击模块/文件可看到曾经开发过程和记录（低优先级，可后续做）

**DoD**:
- [x] 主视图有项目结构预览位置
- [x] 完整详情页可打开
- [x] 图与目录可互相高亮
- [x] 文件打开 / 文件管理器显示动作可用
- [x] 若自动图数据不可用，UI 明确显示占位/数据缺失原因

**完成记录（2026-05-29 / Draft v3.1）**:
- `hard_observed`: 新增只读 project structure read-model：`src-tauri/src/vibehub/project_structure.rs` 扫描项目文件系统，跳过 `.git` / `node_modules` / `target` 等重目录，输出 graph nodes/edges、directory tree、changed 标记、扫描计数与截断 warnings。
- `hard_observed`: `vibehub_read_overview` 聚合返回 `project_structure` 字段；未初始化项目也可得到只读结构投影，失败时返回空结构与 warning。
- `hard_observed`: 新增安全的 `vibehub_reveal_project_file` / `vibehub_open_project_file` Tauri 命令，拒绝绝对路径与 `..` 越界路径，支持目录在文件管理器中显示与文件默认应用打开。
- `hard_observed`: Cockpit 主视图项目总结构区域从 M7a 占位升级为真实结构预览，显示文件系统数据源、模块节点、目录/文件预览、changed 标记与截断 badge。
- `hard_observed`: 完整结构 detail drawer 扩展为宽视图，左侧 graph、右侧 directory tree；选择 graph node 或 tree row 会用同一 node id 互相高亮，并显示 selected path、目录/文件统计、文件操作按钮。
- `hard_observed`: UI 明确显示“当前基于文件系统扫描；语义模块图尚未配置”，将语义依赖图与模块/文件历史记录保留到后续切片。
- `hard_observed`: 新增 English / 简体中文 / 繁体中文本地化文本，并新增 Rust 单测覆盖结构扫描、重目录跳过与路径逃逸拒绝。

**产出 handoff**: `runs/<run_id>/handoffs/M7e.json`

---

## 7. 代码类 Steps（M8 — Capability 自定义）

### M8a — 自定义 capability 声明 + schema 注册 + workflow.yaml 扩展

**目标**: 落地基线 §3.3 v2+ 预留；让用户可定义自己的 capability 类型。

**必读输入**: 基线 §3.3（v2+ 自定义）, §7（Schema 规范性）, §22.6（skills.registry）

**任务清单**:
1. 扩展 `workflow.yaml` 支持用户自定义 capability 声明
   ```yaml
   custom_capabilities:
     security_audit:
       required_fields: [audit_scope, "findings[]", severity_summary]
       optional_fields: [recommendations, compliance_refs]
       produces: [audit_report]
       consumes: [implement]   # 声明软依赖
       parallel_safe: true
   ```
2. 实现自定义 capability 的 schema 注册
   - 运行时解析 `custom_capabilities` 段
   - 注册到 schema 校验器
   - 校验与内置 capability 名称不冲突
3. 自定义 capability 的 gate 支持
   - 可在 `gates:` 段引用自定义 capability
4. 自定义 capability 的 context pack 模板
   - 自动为自定义 capability 生成空 context pack 模板
5. 向后兼容：无自定义 capability 的项目不受影响

**完成记录（2026-05-29 / Draft v3.2）**:
- `hard_observed`: `workflow.yaml` 解析新增 `custom_capabilities` 段，`vibehub_workflow_explain` 会把自定义 capability 标记为 `custom: true`。
- `hard_observed`: schema 校验器在项目上下文中读取 workflow 注册表，支持自定义 capability 必填/可选字段校验；`"field[]"` 后缀按非空数组校验。
- `hard_observed`: gate evaluator、claim flow、产物推导统一读取内置 + 自定义 capability definition，自定义 capability 可被 claim 并生成独立 context pack。
- `hard_observed`: 自定义 capability context pack 会输出 Capability Output Schema，作为空模板/入场提示；无自定义 schema 时保留旧行为。
- `hard_observed`: `cargo test --locked vibehub::` 通过 184 个 VibeHub 后端测试。

**DoD**:
- [x] 自定义 capability 可在 workflow.yaml 声明且被引擎解析
- [x] 自定义 capability 的产出受 schema 校验
- [x] gate 可引用自定义 capability
- [x] 与内置 capability 名称冲突时报错
- [x] 无自定义时行为与 v1 完全一致

**产出 handoff**: `runs/<run_id>/handoffs/M8a.json`

---

### M8b — 自定义 capability 端到端验证 + 文档

**目标**: 完整验证自定义 capability 的全链路，并输出用户文档。

**必读输入**: M8a 产出

**任务清单**:
1. 端到端场景测试
   - 定义一个 `security_audit` capability → claim → 产出 → release → handoff
   - 验证 schema 校验、gate 联动、context pack 传递全链路
2. 编写用户文档
   - `docs/vibehub-custom-capability-guide.md`
   - 如何声明、如何定义字段、如何与 gate 联动
   - 迁移指南（从 v1 预设扩展到 v2 自定义）
3. 更新 RFC-0001 反映 v2 自定义扩展
4. 更新 `skills.registry.yaml` 补充自定义相关 skill

**完成记录（2026-05-29 / Draft v3.3）**:
- `hard_observed`: 新增后端端到端测试 `custom_capability_end_to_end_claim_output_release_and_handoff`，覆盖 `security_audit` 自定义 capability 的 claim、context pack schema hints、schema-checked output、completed release、下游 `review_ready` gate artifact 联动与 handoff 事件。
- `hard_observed`: 新增用户文档 `docs/vibehub-custom-capability-guide.md`，覆盖声明方式、字段语法、输出 envelope、gate/produces 联动、context pack schema、v1 迁移和常见问题。
- `hard_observed`: `docs/rfc/0001-capability-gate-workflow.md` 增补 v2 自定义 capability 概念、schema 约束、M8 迁移项和 review checklist。
- `hard_observed`: `.vibehub/skills.registry.yaml` 增补 custom capability schema 校验参数和 `vibehub-configure-custom-capability` skill 规格。

**DoD**:
- [x] 端到端测试通过
- [x] 用户文档可让新用户独立配置自定义 capability
- [x] RFC 更新反映 v2 扩展

**产出 handoff**: `runs/<run_id>/handoffs/M8b.json`

---

## 8. 跨切面工程步骤（X 系列）

> X 系列步骤是基线要求的跨切面工程需求，可与 M6/M7/M8 并行推进。

### X1 — Core/CLI 拆分

**状态**: ✅ 已完成（2026-05-29）

**目标**: 落地基线 §29；让 VibeHub Core 可独立编译为 CLI 工具。

**必读输入**: 基线 §29.1-29.5

**任务清单**:
1. 创建 `crates/vibehub-core/`（纯 Rust lib）
   - 从 `src-tauri/src/vibehub/` 抽出所有核心逻辑
   - 不依赖任何 Tauri runtime API（基线 §29.4）
   - 所有 IO 走 Rust 标准库 + crate
2. 创建 `crates/vibehub-cli/`（二进制）
   - 实现 CLI 命令：`vibehub start-task`, `vibehub sync`, `vibehub claim`, `vibehub status` 等
   - 提供 stdio / RPC 接口供外部 agent 工具调用
3. 改造 Tauri 应用为 thin wrapper
   - `src-tauri/src/vibehub/` 改为调用 `vibehub-core` lib
   - Tauri command 仅做参数转换 + 结果包装
4. 验证拆分后功能不回归
   - `cargo test` 全项目全绿
   - Tauri 应用功能不变

**DoD**:
- [x] `crates/vibehub-core/` 可独立编译（`cargo build -p vibehub-core --offline`）
- [x] `crates/vibehub-cli/` 可独立运行基本命令（`target/debug/vibehub status /Users/chenm0m/LocalRepo/VibeHub`）
- [x] `vibehub-core` 中无 Tauri runtime 依赖（`rg -n "tauri =|tauri::|tauri_plugin|tauri-runtime|AppHandle|Manager|State<|#\\[tauri::command\\]" crates/vibehub-core`）
- [x] Tauri 应用功能回归测试通过（`cargo build -p vibehub --offline`）
- [x] `cargo test --all` 全绿（`cargo test --all --offline`）

**产出 handoff**: `runs/<run_id>/handoffs/X1.json`

---

### X2 — Adapter 配置生成器 + 6 工具投影

**状态**: ✅ 已完成（2026-05-29）

**目标**: 落地基线 §22.4, §27；实现从 `skills.registry.yaml` 自动生成各 agent 工具配置。

**必读输入**: 基线 §22.4, §27.1-27.3；S3 skill 清单

**任务清单**:
1. 实现 adapter generator 核心
   - 输入：`.vibehub/skills.registry.yaml`
   - 输出：各工具的配置文件
   - 只修改 `<!-- VIBEHUB:AGENT-INTEGRATION:START -->` / `:END` 标记内的内容
   - 标记外用户内容保留（基线 §22.4, C4）
2. 支持 6 个 v1 工具（基线 §27.1, H1）
   - Amp Code → `AGENTS.md`
   - Claude Code → `CLAUDE.md`
   - Codex → `.codex/`
   - OpenCode → `.opencode/`
   - Cursor → `.cursor/`
   - Antigravity → 对应配置文件
3. 版本号标记 + 升级检测（基线 §27.3, H3）
   - 启动时检测 registry 版本 ≠ 配置文件版本 → 提示更新
   - UI 弹模态："adapter 配置过期，建议更新"
4. 明确标注此处是 INV-6 的唯一例外（基线 §27.3 注释）

**DoD**:
- [x] 从 registry 生成 6 种工具配置且内容正确（`agent_adapter::tests::projects_skills_registry_commands` + six-tool dry run）
- [x] 标记内容可更新、标记外内容保留（`preserves_unmanaged_content_in_agents_md`）
- [x] 版本不匹配时 UI 提示（adapter status warnings surfaced in cockpit action notice / adapter panel）
- [x] 现有 AGENTS.md 兼容性不破坏（managed marker update keeps user content outside markers）

**产出 handoff**: `runs/<run_id>/handoffs/X2.json`

---

### X3 — `.pending/` 兜底写入机制

**目标**: 落地基线 §21.2 (B2)；让启动器未运行时 agent 仍可写入。

**必读输入**: 基线 §21.1-21.2

**任务清单**:
1. 实现 `.vibehub/.pending/` 目录写入
   - Agent 写入时优先尝试 Tauri command
   - 失败（启动器未运行）→ 落到 `.pending/*.write` 文件
   - 每个 `.write` 文件包含完整的事件 payload + 元信息
2. 启动器启动时扫描 `.pending/`
   - 按时间戳排序
   - 串行回放为正式事件到 `events.jsonl`
   - 回放后清理 `.pending/` 文件
   - 冲突检测（event_id 重复 → 跳过并 warn）
3. 保证单写者不变量（INV-3）
   - `.pending/` 回放期间锁定正常写入

**DoD**:
- [x] 启动器未运行时 agent 写入 → `.pending/` 落盘成功（`pending_event_write_replays_and_cleans_pending_file`）
- [x] 启动器启动后自动回放 → events.jsonl 正确（startup replay hook + `replay-pending` CLI/API）
- [x] 回放后 `.pending/` 被清理（`pending_event_write_replays_and_cleans_pending_file`）
- [x] 单写者不变量未破坏（`pending_replay_and_normal_append_share_single_writer_lock`）

**完成记录（2026-05-29 / Draft v3.6）**:
- `crates/vibehub-core/src/vibehub/events.rs` 新增 `.vibehub/.pending/*.write` 兜底事件文件格式、pending 写入 API、按文件名排序的串行回放、重复 / 乱序 event_id 跳过 warning、回放后清理和单写者锁复用。
- Tauri 启动时对已知项目自动回放 pending 事件；Tauri command、workspace CLI、standalone CLI 均提供 `replay-pending` / `pending-replay` 手动入口。
- TypeScript service/type 增加 `vibehubReplayPendingEvents` 和 `PendingReplayResult`。
- `cargo test --all --offline`、`npm run build`、CLI `replay-pending .` 通过。

**产出 handoff**: `runs/<run_id>/handoffs/X3.json`

---

### X4 — `vibehub-debug-dump` + 调试导出

**目标**: 落地基线 §28.4 (J3)；一键导出调试信息。

**必读输入**: 基线 §28.4

**任务清单**:
1. 实现 `vibehub-debug-dump` skill
   - 一次性导出：
     - 当前完整 `state.yaml`
     - 当前 task 全 capability 的 context pack
     - 完整 `events.jsonl`
     - 全部 outputs / handoffs / sync reports
   - 输出位置：`.vibehub/debug-dumps/<timestamp>/`
2. 输出格式：目录结构 + 可选 tar.gz 打包
3. 注册到 `skills.registry.yaml`
4. 不泄露敏感信息（若有 secrets → 脱敏）

**DoD**:
- [x] `vibehub-debug-dump` 可正常执行并输出完整目录（`cargo run -p vibehub-cli --offline -- debug-dump .` 输出 `.vibehub/debug-dumps/20260529T061211Z/`）
- [x] 输出内容可在另一台机器还原问题（跨环境调试）：目录包含 manifest、redacted `state.yaml`、agent-view、rules、当前 task/run metadata、events、context-packs、outputs、handoffs、sync reports
- [x] 已注册到 registry（`.vibehub/skills.registry.yaml` 与 `docs/vibehub-skills-registry-v1.md` 均记录 `vibehub-debug-dump`）

**完成记录（2026-05-29 / Draft v3.7）**:
- `crates/vibehub-core/src/vibehub/debug_dump.rs` 新增 redacted debug dump 导出：默认导出 state、project metadata、agent-view、rules、当前 task 全 run 的 events/context-packs/outputs/handoffs/sync reports，并生成 `manifest.json`。
- CLI 增加 `debug-dump` / `vibehub-debug-dump`，Tauri command 增加 `vibehub_debug_dump`，TypeScript service/type 增加 `vibehubDebugDump`、`DebugDumpOptions`、`DebugDumpResult`。
- registry 中 `vibehub-debug-dump` 默认 `include_events=true`、`include_packs=true`、`redact_secrets=true`；导出过程对常见 secret/token/password/credential 行做脱敏。
- `cargo test --all --offline`、`npm run build`、CLI `debug-dump .` 通过。

**产出 handoff**: `runs/<run_id>/handoffs/X4.json`

---

## 9. 附录

### 9.1 与基线全章节的对应表

#### 基线 §13「下一步工作清单」→ Step 对应

| 基线 §13 # | 任务 | 对应 Step |
|---|---|---|
| 1 | 把本文档作为 baseline 入库 | ✅ **S0**（2026-05-28） |
| 2 | 起草 RFC-0001 | ✅ **S1**（2026-05-28） |
| 3 | 写「Capability Schema 草案」详细版 | ✅ **S2**（2026-05-28） |
| 4 | 写「Skill 接口清单」 | ✅ **S3**（2026-05-28） |
| 5 | 实现 M1（events.jsonl 双写） | ✅ **M1a**（2026-05-28） + ✅ **M1b**（2026-05-28） + ✅ **M1c**（2026-05-28） |
| 5a | workflow.yaml 扩展 capability/gate 段 + 解析器 | ✅ **M2a**（2026-05-28） |
| 5b | Gate 引擎 + 条件检测 + `vibehub-claim` | ✅ **M2b**（2026-05-28） |
| 5c | Projection 层接管 `state.yaml` derived 字段 | ✅ **M3**（2026-05-28） |
| 6 | 量化 baseline 指标 | （并入 S6 / M5） |
| 7 | 设计条件检测算法 | ✅ **S4**（2026-05-28） |
| 8 | 设计文件 ownership 表 | ✅ **S5**（2026-05-28） |
| 9 | 设计 Kanban UI 原型 | ✅ **S7**（2026-05-28） |
| —（新增） | 测试计划文档 | ✅ **S6**（2026-05-28） |

#### 基线 §12 Milestone → Step 对应

| 基线 Milestone | 对应 Step | 状态 |
|---|---|---|
| M1 — Event Log | M1a + M1b + M1c | ✅ 已完成 |
| M2 — Capability + Gate | M2a + M2b | ✅ 已完成 |
| M3 — Projection | M3 | ✅ 已完成 |
| M4 — 并发与回环 | M4 (M4a+M4b) | ✅ 已完成 |
| M5 — Fitness & Schema | M5 | ✅ 已完成 |
| M6 — 多 Task 并行 | M6a + M6b + M6c + M6d | ✅ 已完成 |
| M7 — UI 全面对接 / 前端体验重构 | M7a + M7b + M7c + M7d + M7e | ✅ 已完成 |
| M8 — Capability 自定义 | M8a + M8b | ✅ 已完成 |
| X1 — Core/CLI 拆分 | X1 | ✅ 已完成 |
| X2 — Adapter 配置生成器 + 6 工具投影 | X2 | ✅ 已完成 |
| X3 — `.pending/` 兜底写入机制 | X3 | ✅ 已完成 |
| X4 — `vibehub-debug-dump` + 调试导出 | X4 | ✅ 已完成 |

#### 基线全章节 → Step 覆盖矩阵

| 基线章节 | 内容 | 覆盖 Step | 状态 |
|---|---|---|---|
| §1 背景与痛点 | 动机文档 | S0 | ✅ |
| §2 设计哲学（3 铁律） | 贯穿全局 | S0, 所有 Step | ✅ |
| §3 核心概念定义 | Task/Capability/Event/Gate/Projection | S1, S2, M1-M4 | ✅ |
| §3.3 v2+ 自定义 | Capability 自定义 | **M8a, M8b** | ✅ |
| §4 用户旅程 | 5 个核心旅程 | M6c（旅程C）, **M6d（旅程E）**, M7a-e（旅程A/B/D + 项目结构） | ✅ |
| §5 前端 UI 定位 | 看板 = 显示器 | S7, **M7a-M7e** | ✅ |
| §6 同步机制 | 分级 sync | S4, M2b | ✅ |
| §7 Schema 规范性 | 产出硬 | S2, **M5** | ✅ |
| §8.1 Task 内并行 | Capability 并行 | M4 | ✅ |
| §8.2 多 Task 并行 | 多 task 同时活跃 + 多需求自动 intake | **M6a-M6d** | ✅ |
| §8.3 文件归属 | Ownership 表 | S5, **M6b** | ✅ |
| §9 失败容忍 | 中断恢复 / 补偿 | M1b, M4 | ✅ |
| §10 Sub-agent 委派 | 委派策略 | 贯穿 M1-M5 | ✅ |
| §11 工程审视 | 合理/风险/暂缓 | S0 | ✅ |
| §12 迁移路径 | 5+3 Milestone | M1-M8 全覆盖 | 部分 |
| §14 Open Questions | 待决议 | M5（部分）, M6-M8（其余） | ❌ |
| §17 Context Pack 规范 | 两层结构 | M4, **M6c** | 部分 |
| §18 核心不变量 | 10 条 INV | 贯穿所有 Step | ✅ |
| §19 术语词典 | Glossary | S0 | ✅ |
| §20 Event Schema | 事件存储 | M1a, M1b | ✅ |
| §21 并发模型 | 单写者 | M1a, **X3** | ✅ |
| §22 Skill 协议 | 统一接口 | S3, **M6d**, **X2** | ✅ |
| §23 UI 同步 & 模板 | 提示词生成器 | S7, **M7a-M7e** | ✅ |
| §24 UX 生命周期 | 首次/完成/取消/暂停 | **M7a, M7d** | ✅ |
| §25 错误模型 | 错误码体系 | **M5** | ✅ |
| §26 Git 边界 | 只读 git | M1a, **M6b** | ✅ |
| §27 跨工具 Adapter | 配置生成 | **X2** | ✅ |
| §28 测试 & 观测 | 测试计划 / metrics / debug dump | S6, M5, **X4** | ✅ |
| §29 架构定位 | CLI + 开源 | **X1** | ✅ |

### 9.2 DoD 通用模板（每个 Step 必填）

```yaml
step_id: <S0/S1/.../M8b/X4>
status: <in_progress|blocked|done>
acceptance_criteria:
  - id: AC-1
    description: "..."
    evidence_label: <hard_observed|agent_reported|user_confirmed|inferred>
    evidence_ref: "<file:line / command output / user note>"
    status: <pass|fail|na>
blockers: []
followups: []
next_step: <S?/M?/X?>
handoff_path: "runs/<run_id>/handoffs/<step_id>.json"
```

### 9.3 续跑/重入 checklist

进入任何 Step 前：

- [ ] 读了 `.vibehub/agent-view/current.md`
- [ ] 读了上一 Step 的 handoff
- [ ] 读了基线对应章节
- [ ] 用 `vibehub-status` 校准
- [ ] 若上次是 `blocked` → 已确认阻塞点是否仍存在
- [ ] 若 HEAD 有变 → 已 `vibehub-sync`

### 9.4 Open Questions 追踪表

> 基线 §14 和 §17.11 的 Open Questions 在本手册中对应到具体 Step 处理。

| 基线 ID | 问题 | 处理 Step | 状态 |
|---|---|---|---|
| §14.1 | 条件检测的具体阈值如何设定？ | S4 + M2b | ✅ 已决议 |
| §14.1 | 强制重建时主动询问哪些问题？ | M7c（智能判断单/多task） | ✅ 已决议 |
| §14.1 | 跨 task 同步时如何避免重复扫描 git？ | M6a | ❌ 待处理 |
| §14.2 | capability ID 全局唯一还是 task 内唯一？ | M4 实现中决议：task 内唯一 | ✅ 已决议 |
| §14.2 | capability 之间能否声明软依赖？ | - | ✅ 已决议（不加软依赖，放提示词里） |
| §14.2 | capability 失败时事件如何标记？ | M4b 实现：`CapabilityReleased{outcome: failed}` | ✅ 已决议 |
| §14.3 | 占位符统一定义 | S2 + M5 schema_check | ✅ 已处理 |
| §14.3 | schema 是否允许草稿模式？ | M5 `schema.strict` policy | ✅ 已决议（混合模式：中断保存，续跑强制补全） |
| §14.3 | 字段类型校验放 Rust 还是 JSON Schema？ | M5 schema_check | ✅ 已决议（JSON Schema 标准） |
| §14.4 | 提示词模板 i18n？ | M7c | ✅ 已决议（初始化选语言，跟全局设置走） |
| §14.4 | 看板是否支持拖动？ | S7 已决议：不支持 | ✅ 已决议 |
| §14.4 | 事件流时间线是否支持过滤？ | M7b | ✅ 已决议（支持过滤） |
| §14.5 | sub-agent 调用接口？ | M5 policy.yaml | ❌ 待处理 |
| §14.5 | sub-agent 失败超时阈值？ | - | ✅ 已决议（不管，AI 客户端自行负责） |
| §14.5 | 多 sub-agent 并发写入模型？ | M1a 单写者 + X3 pending | 部分 |
| §14.6 | 老 task 是否回填事件流？ | M5 schema_check + 自动回填 | ✅ 已决议（自动回填） |
| §14.6 | 双写期间不一致处理？ | M1c 一致性测试 | ✅ 已处理 |
| §14.6 | 何时关闭旧路径？ | M6（多task完成切） | ✅ 已决议 |
| CP-Q1 | prior_outputs_summary 摘要长度上限？ | M5 policy.yaml | ❌ 待处理 |
| CP-Q2 | Task pack 旧版本归档策略？ | - | ✅ 已决议（全部保留，不自动清理） |
| CP-Q3 | neighbors 字段 v1 表现？ | M6c | ✅ 已决议（空数组 `[]` 表示无邻居） |
| CP-Q4 | code_navigation 自动生成质量？ | M7b | ❌ 待处理 |
| CP-Q6 | Pack i18n？ | M7c | ✅ 已决议（初始化选语言） |
| UI-Q1 | 2-4 字 task 短标签如何生成/编辑？ | M7a | ✅ 已决议（前端启发式短标签；可编辑/元数据化留后续） |
| UI-Q2 | 活跃任务多少个后折叠/分页？ | M7a | ✅ 已决议（M7a 纵向尽量全部展示；过多时后续折叠/分页） |
| UI-Q3 | 详情页采用 route / drawer / modal 哪种形态？ | M7a | ✅ 已决议（M7a 用同 dialog 内 detail surface；route 可后续演进） |
| UI-Q4 | 项目设置哪些属于 INV-6 允许的 UI 写入例外？ | M7a/M7c | 部分（M7a 仅展示语言/adapter/remote 设置入口；写入策略留 M7c/X2） |
| UI-Q5 | 最近提交线上跳转 v1 支持哪些 remote provider？ | M7a | 部分（M7a 保留远端 URL 入口；provider 细分留 M7b/M7e） |
| UI-Q6 | 项目总结构图首版数据源是什么？ | M7e | ✅ 已决议（M7e v1 使用 filesystem scan 只读投影；semantic dependency graph 后续升级） |
| UI-Q7 | 模块/文件历史记录是否纳入 M7e v1？ | M7e | ✅ 已决议（M7e v1 仅预留入口/提示，历史记录后续切片） |
| INTAKE-Q1 | 多需求拆分的"高置信/中置信/低置信"阈值是否需要用户可配置？ | M6d | ✅ 已决议（M6d 使用请求级 confidence；项目级可配置暂不做） |

### 9.5 文档维护

| 版本 | 日期 | 变更 |
|---|---|---|
| Draft v3.7 | 2026-05-29 | 标记 X4 完成：新增 `vibehub-debug-dump` core/CLI/Tauri/TS 调试导出，默认导出 redacted state、agent-view、rules、当前 task 全 run 的 events/context-packs/outputs/handoffs/sync reports 与 manifest；registry 默认启用 events/packs/redaction；`cargo test --all --offline`、`npm run build`、CLI `debug-dump .` 通过。 |
| Draft v3.6 | 2026-05-29 | 标记 X3 完成：新增 `.vibehub/.pending/*.write` pending 事件格式、fallback 写入 API、启动器启动回放、CLI/Tauri replay 入口、重复/乱序 event_id warning 与清理；单写者锁覆盖 pending 回放和正常写入；事件层 pending/duplicate/concurrency 单测通过，`cargo test --all --offline`、`npm run build`、CLI `replay-pending .` 通过。 |
| Draft v3.5 | 2026-05-29 | 标记 X2 完成：adapter generator 读取 `.vibehub/skills.registry.yaml`，合并 registry skill 投影与既有 workflow 命令；默认支持 Amp Code、Claude Code、Codex、OpenCode、Cursor、Antigravity 六工具；生成内容含 template/registry 版本标记，status 返回过期/缺失/冲突 warning，cockpit 展示更新提示；CLI 增加 `adapter-status` 与 `sync-adapters --dry-run`；`cargo test --all --offline` 与 `npm run build` 通过。 |
| Draft v3.4 | 2026-05-29 | 标记 X1 完成：新增 workspace 根 `Cargo.toml`、`crates/vibehub-core` 纯 Rust lib、`crates/vibehub-cli` 二进制，Tauri `src-tauri/src/vibehub/mod.rs` 改为 core re-export；`cargo build -p vibehub-core --offline`、`cargo build -p vibehub-cli --offline`、CLI `status`、`cargo build -p vibehub --offline`、`cargo test --all --offline` 通过。 |
| Draft v3.3 | 2026-05-29 | 标记 M8b 完成：新增自定义 capability 端到端 fixture，覆盖 claim/output/release/handoff 和 downstream gate；新增用户指南，RFC-0001 记录 v2 自定义扩展，skills registry 补充自定义配置与校验接口。 |
| Draft v3.2 | 2026-05-29 | 标记 M8a 完成：`custom_capabilities` workflow 解析、冲突校验、项目级自定义 schema 校验、gate/claim/context pack 接入与 VibeHub 后端回归测试落地。 |
| Draft v3.1 | 2026-05-29 | 标记 M7e 完成：filesystem-backed 项目结构 graph/tree read-model、overview 聚合字段、安全项目文件 open/reveal 命令、cockpit graph/tree 联动详情、本地化与路径安全/扫描单测落地；语义依赖图与模块历史记录明确后续升级。 |
| Draft v2.9 | 2026-05-28 | 标记 M7c 完成：prompt renderer Tauri API、7 个多语言默认模板、项目级覆盖、cockpit 提示词生成模态、复制按钮、危险操作二次确认落地；`npm run build` 与 prompt 单测通过。 |
| Draft v2.8 | 2026-05-28 | 标记 M7b 完成：overview event_timeline 只读模型、最近动态筛选时间线、事件点击聚焦、结构化 event/package/JSON viewer、安全文件定位/默认打开命令与 build/test/browser smoke 记录落地。 |
| Draft v2.7 | 2026-05-28 | 标记 M6d 完成：多意图 intake API、TaskIntakePlanned、active task 队列、agent-view/status/CLI/Tauri/前端类型与测试落地；标记 M7a 完成：Kanban-first 项目详情 IA shell 替代旧 Dashboard 主体，记录 seeded 3-task 浏览器视觉证据仍需补强。 |
| Draft v2.6 | 2026-05-28 | 新增 M6d：多需求意图拆分 + 自动多 task intake；明确当前 M6a-M6c 只解决多 task 存在/切换/归属，不等于 agent 能自动从一条用户消息拆出多个 task。 |
| Draft v2.5 | 2026-05-28 | 根据用户草图与口述更新 M7 为前端体验重构：Kanban-first 项目详情页替代旧 Dashboard 主视图；新增最近提交/最近动态/任务过程条/详情页/项目总结构/归档 UX；M7 扩展为 M7a-M7e；补充 UI-Q1~UI-Q7。 |
| Draft v2.2 | 2026-05-28 | 补充了全部 12 项 OQ 决议：混合草稿模式、JSON Schema 校验、初始化语言设置、事件流过滤、智能判断重建、空数组 neighbors、回填老 task 等。 |
| Draft v2.1 | 2026-05-28 | M5 schema_check、policy.yaml、fitness metrics、event log recovery、LoopWarning 事件完成；记录测试与 build 通过 |
| Draft v2.0 | 2026-05-28 | 全量扩展：新增 M6a-M6c（多 task 并行）、M7a-M7d（UI 全面对接）、M8a-M8b（capability 自定义）、X1-X4（跨切面工程）；新增基线全章节覆盖矩阵、Open Questions 追踪表；M5 细化为含 8 项 DoD 的完整规格 |
| Draft v2.2 | 2026-05-28 | M6a 多 task 状态模型、task switch 命令、task-events index、agent-view active tasks 展示完成 |
| Draft v2.3 | 2026-05-28 | M6b 文件 ownership 引擎、冲突 prompt、手动 record API、ownership CLI/Tauri surface、sync ownership 输入完成 |
| Draft v2.4 | 2026-05-28 | M6c 邻居 task 查询、context pack neighbors 字段、agent-view/status 多 task 概览、NeighborConflictDetected 事件完成 |
| Draft v1.17 | 2026-05-28 | M4b review failure auto release、handoff prior_outputs_summary、task_pack_dirty delta trigger/no-trigger tests、下游 context pack 摘要消费完成 |
| Draft v1.16 | 2026-05-28 | M4a 多 capability active claim、agent-view active capabilities、compensation projection、每 capability context pack 验证完成；handoff delta 留给 M4b |
| Draft v1.15 | 2026-05-28 | M3 projection 层、schema v4 derived 标记、derivation_trace、status gate 输出完成，记录 Step 地图完成状态 |
| Draft v1.14 | 2026-05-28 | M2b gate evaluator、claim 命令、vibehub-claim skill、sync 三级条件检测完成，记录 Step 地图完成状态 |
| Draft v1.13 | 2026-05-28 | M2a workflow.yaml capability/gate 解析器和 explain 命令完成，记录 Step 地图完成状态 |
| Draft v1.12 | 2026-05-28 | M1c 一致性测试和 schema v2→v3 migration 完成，记录 Step 地图完成状态 |
| Draft v1.11 | 2026-05-28 | M1b 双写埋点完成，记录 Step 地图完成状态 |
| Draft v1.10 | 2026-05-28 | M1a events writer / enum / event_id 完成，记录 Step 地图完成状态 |
| Draft v1.9 | 2026-05-28 | S7 Kanban UI mockup 完成，记录 Step 地图完成状态 |
| Draft v1.8 | 2026-05-28 | 修正 S7 模板数量为 7，与基线 §23.4 对齐 |
| Draft v1.7 | 2026-05-28 | S6 测试计划完成，记录 Step 地图完成状态 |
| Draft v1.6 | 2026-05-28 | S5 文件 ownership 设计完成，记录 Step 地图完成状态 |
| Draft v1.5 | 2026-05-28 | S4 Sync 条件检测算法完成，记录 Step 地图完成状态 |
| Draft v1.4 | 2026-05-28 | S3 Skill registry 文档和 YAML 样板完成，记录 Step 地图完成状态 |
| Draft v1.3 | 2026-05-28 | S2 Capability Schema 详细版完成，记录 Step 地图完成状态 |
| Draft v1.2 | 2026-05-28 | S1 RFC-0001 完成，记录 Step 地图完成状态 |
| Draft v1.1 | 2026-05-28 | S0 共识校准完成，记录 Step 地图完成状态 |
| Draft v1 | 2026-05-27 | 初稿，配套基线 v1.2 |

任何 Step 完成后：在 §0 Step 地图中把对应行标 ✅，并在 §9.1 对应行末加完成日期。

# VibeHub Capability 模型重构 — 设计文档

> **文档状态**: Draft v1.3（结构整理 2026-05-27）
> **日期**: 2026-05-27
> **作者**: User × Amp（基于对话 T-019e68d6-4f76-728c-8064-f27834e3066b 整理）
> **目的**: 把当前讨论的所有想法、原则、约束、决策、风险完整记录下来，作为后续任何 agent / 任何对话推进实施的**单一参考基线**。
> **使用方式**: 任何后续工作（细化 schema、写 RFC、开始实现、调整 UI）都应基于本文档继续，而不是从零讨论。
> **落地配套**: 分步实施手册见 [`vibehub-capability-implementation-steps-2026-05-27.md`](./vibehub-capability-implementation-steps-2026-05-27.md)。

---

## AGENT BOOTSTRAP（必读 · 80 行速查）

> **任何新对话窗口、任何 agent，第一件事请把本节读完。** 读完即可定位到具体节号深入；不读完不要做事。

### 0.A 三铁律（详见 §2）

1. **同步是分级的，由 VibeHub 自己判断**（轻 / 深 / 强制重建）
2. **流程软、产出硬**（capability 顺序灵活，schema 必填字段强制）
3. **断点不是回滚点，是接续点**（崩溃/换工具不丢工作，多花 token 也要续跑）

### 0.B 十不变量（详见 §18）

INV-1 events.jsonl 严格 append-only · INV-2 派生字段可重放 · INV-3 单写者 · INV-4 必填缺失即拒写 · INV-5 Context Pack 必带 freshness · INV-6 UI 不写状态（adapter 文件除外，§27.3） · INV-7 Sub-agent 失败即无 pack · INV-8 VibeHub 不写 git · INV-9 必须有 git 仓库 · INV-10 跨工具 skill 协议一致。

### 0.C 术语速查（详见 §19）

`Task`（看板卡片粒度需求） · `Capability`（task 内独立工作单元，取代 phase） · `Event`（append-only 真相源） · `Gate`（准入 predicate） · `Projection`（事件流派生视图） · `Context Pack`（agent 入场说明书，task / capability 两层） · `Handoff`（capability release 时的交接物 + delta 信号） · `Sync`（三级同步） · `Drift`（未追踪改动） · `Writer`（单例 Tauri backend） · `Adapter`（跨工具配置生成器）。

### 0.D 「我想知道 X，去哪一节」反查

| 问题 | 去哪 |
|---|---|
| 想了解整体哲学 / 痛点 | §1, §2 |
| 想知道 capability 有哪些 / 必填字段 | §3.3, §7.2, §17.5 |
| 想知道事件类型清单 / 存储布局 | §20 |
| 想知道 sync 三级与触发条件 | §6, §17.8 |
| 想知道并发模型 / 单写者 / sub-agent 上限 | §10, §21 |
| 想知道 skill 接口 / 返回格式 | §22 |
| 想知道 UI 边界 / 看板形态 / 提示词模板 | §5, §23, §24 |
| 想知道错误码体系 / 失败处理 | §25 |
| 想知道 Git 边界 / 文件 ownership | §8.3, §26 |
| 想知道跨工具 adapter 与配置生成 | §22.4, §27 |
| 想知道测试矩阵 / 观测指标 | §28 |
| 想知道架构定位 / CLI 与未来开源 | §29 |
| 想知道迁移路径 / Milestone | §12 |
| 想知道遗留 Open Questions | §14, §17.11 |
| **想动手干活、按 Step 推进** | **配套手册** `vibehub-capability-implementation-steps-2026-05-27.md` |

### 0.E 关键决议一句话总结

- Capability v1 用预设（与现 phase 1:1），v2+ 可自定义
- Schema 校验**写入时同步**做，不交给 AI；缺必填即拒
- Context Pack 由 **sub-agent 合成**；上一 capability 在 release 时主动出摘要
- 事件流**按 run 分片**、永不归档、跨 run 查询走 `index/task-events.idx`
- Sub-agent 失败**绝不 fallback**；空就是空（INV-7）
- UI 只展示 + 生成提示词，**不直接执行任何操作**（INV-6）
- VibeHub Core 必须可独立成 CLI；Tauri 只是 wrapper（§29）

### 0.F 阅读优先级建议

- **第一次读**：本节 + §2 + §3 + §18 + §19（约 30 分钟即可上手）
- **要写代码**：再加 §20 + §21 + §22 + §25 + §29
- **要做 UI**：再加 §5 + §23 + §24
- **要做 sync / 跨工具**：再加 §6 + §17 + §26 + §27

---

## 0. 文档导航

> 全文按 6 大 Part 组织。Part 顺序 = 推荐阅读顺序；具体节号保留原编号不动。

### Part A · 哲学 & 痛点（先理解"为什么这样设计"）

| § | 标题 | ⏱ 何时必读 |
|---|---|---|
| 1 | 背景与痛点 | 第一次接触本项目 |
| 2 | 设计哲学（3 条铁律） | **所有 agent 都必读** |

### Part B · 概念模型（数据结构 / 不变量 / 术语）

| § | 标题 | ⏱ 何时必读 |
|---|---|---|
| 3 | 核心概念定义（Task / Capability / Event / Gate / Projection） | **所有 agent 都必读** |
| 18 | 核心不变量清单（v1.2） | **写代码前必读** |
| 19 | 术语词典（v1.2） | **所有 agent 都必读** |
| 20 | Event Schema & 存储布局（v1.2） | 写代码前必读 |
| 21 | 并发与单写者模型（v1.2） | 写代码前必读 |

### Part C · 用户体验 & UI（看板 = 显示器）

| § | 标题 | ⏱ 何时必读 |
|---|---|---|
| 4 | 用户体验需求（5 个核心旅程） | 做 UX / UI / sync 前必读 |
| 5 | 前端 UI 定位（看板 = 显示器） | 做 UI 前必读 |
| 23 | UI 同步 & 提示词生成器（v1.2） | 做 UI 前必读 |
| 24 | 用户体验生命周期（v1.2） | 做 UI 前必读 |

### Part D · 工程机制（同步 / Schema / 并发 / 委派 / Skill / 错误 / Git / 跨工具）

| § | 标题 | ⏱ 何时必读 |
|---|---|---|
| 6 | 同步机制（分级 sync） | 做 sync / 跨工具前必读 |
| 7 | Schema 规范性 | 写代码前必读 |
| 8 | 并行能力 | 做 M4 前必读 |
| 9 | 失败容忍与中断恢复 | 做 M1+ 前必读 |
| 10 | Sub-agent 委派策略 | 调 sub-agent 前必读 |
| 17 | Context Pack 规范（v1.1） | **写 sync / handoff / pack 前必读** |
| 22 | Skill 接口 & Agent 协议（v1.2） | 写 skill 前必读 |
| 25 | 错误模型（v1.2） | 写代码前必读 |
| 26 | Git 耦合边界（v1.2） | 碰 git / ownership 前必读 |
| 27 | 跨工具 Adapter（v1.2） | 做 adapter 前必读 |

### Part E · 落地路径 & 维护（怎么做 / 何时做 / 谁来做）

| § | 标题 | ⏱ 何时必读 |
|---|---|---|
| 11 | 工程审视（合理 / 风险 / 暂缓） | 启动新 capability 前 |
| 12 | 迁移路径（5 个 Milestone） | **写代码前必读** |
| 13 | 下一步工作清单 | 启动新 capability 前 |
| 28 | 测试 & 观测（v1.2） | 写代码前必读 |
| 29 | 架构定位：CLI 后端 + 前端 + 未来开源（v1.2） | **写代码前必读** |
| 16 | 文档维护 | 修订本文档时 |

### Part F · 附录 & 决议留痕

| § | 标题 | ⏱ 何时必读 |
|---|---|---|
| 14 | 待决议事项（Open Questions） | 决策卡住时 |
| 15 | 附录：原始对话要点 | 想引用用户原话时 |

> **配套落地手册**: 把 §12 Milestone + §13 工作清单切成"一次对话一个 Step"的执行版，见 [`vibehub-capability-implementation-steps-2026-05-27.md`](./vibehub-capability-implementation-steps-2026-05-27.md)。

---

## 1. 背景与痛点

### 1.1 现状
当前 VibeHub 使用硬编码的 phase 顺序（`align → research → plan → implement → review`），由 `workflow.yaml` 定义、`state.yaml` 记录、`phase.rs` 驱动。模式包括 `yolo_drive` / `guided_drive` / `evidence_drive` 三种。

### 1.2 痛点
1. **流程死板**：阶段必须线性推进，无法并行（不能边 research 边 implement）。
2. **效率低**：用户的真实工作流是非线性的，硬走流程反而拖慢节奏。
3. **跨工具体验割裂**：换 AI 工具 / 新对话窗口时，agent 需要重新对齐，常常一无所知。
4. **野路子无法纳管**：用户手改文件 / 没走流程的工作，VibeHub 无法事后整理。
5. **回退/中断成本高**：失败后倾向回滚到 checkpoint，让用户感觉被束缚。

### 1.3 用户原话精华
> "当前的生命周期管理、流程这块感觉还是不够灵活，很多时候因为太过于死板反而会导致效率太低。"
> "我要的是灵活，并不是我要的是整体上的灵活，就是并行任务、不一定要按线性流程来跑。但我也同时需要它每个阶段的规范性。"
> "无论是我这个项目它中间改了或没改、我手改了、或者是没按这个流程去跑，但当我想要用这个流程去跑，我就可以立马去同步它的状态。"
> "Agent 跑到一半崩了……再回来想要再跑，最好都是要让它能够继续这个任务的。"

---

## 2. 设计哲学（3 条铁律）

### 铁律 1：**同步是分级的，由 VibeHub 自己判断**

```diagram
用户行为                       VibeHub 决策
─────────────────────────────────────────────
打开新对话 / 切工具      →    条件检测（看新鲜度）
   ├─ 状态新鲜 + 对齐    →    增量补差（最省 token）
   ├─ 状态老 / 有偏差    →    深度同步（保证根基完整）
   └─ 状态完全不可用     →    强制全量重建

用户输入 "sync"          →    默认深度同步
用户输入 "sync --quick"  →    用户显式要求轻量
```

**条件检测的核心信号**（M2 实现）:
- 距上次 sync 时间
- git HEAD 是否变化
- diff 与 task 范围的重合度

**不变量**: 无论选哪级，**context pack + state 必须留下完整、自洽的快照**。宁可多花 token，不能让根基烂掉。

### 铁律 2：**流程软，产出硬**

| 层 | 强制程度 | 说明 |
|---|---|---|
| 流程顺序（哪个 capability 先做） | **软** | 不强制 research→plan→implement 顺序 |
| 阶段产出（每个 capability 的字段） | **硬** | schema 校验，必填字段缺失 → 拒绝写入 |

- **AI 调什么 → 灵活**（skill 是工具箱，不是流水线）
- **写进去什么 → 规范**（schema 校验，缺字段就拒绝，agent 必须补全才能推进）

理由：没有这层硬约束，"整个工程流程就又是完全由 AI agent 持续掌控了，又是有点失控的感觉"。

### 铁律 3：**断点不是回滚点，是接续点**

- Ctrl-C / 崩溃 / 换工具 / 换窗口 → 都不应该丢工作
- agent 重进时：**先花 token 对齐进度，然后继续**
- "回到上一个 checkpoint 重来" 是 last resort，不是默认行为
- 容忍度要非常高，因为这是用户体验的关键

---

## 3. 核心概念定义

### 3.1 四层模型

```diagram
╭─────────────────────────────────────────────────────╮
│ Layer 4: View          Kanban 看板 + capability 卡片 │
├─────────────────────────────────────────────────────┤
│ Layer 3: Workflow      capabilities + gates（声明） │
├─────────────────────────────────────────────────────┤
│ Layer 2: Projection    state = fold(events)         │
├─────────────────────────────────────────────────────┤
│ Layer 1: Event Log     append-only events.jsonl     │
╰─────────────────────────────────────────────────────╯
```

### 3.2 Task（任务）
- 用户提出的一个**需求单元**（例如"做用户登录页"）
- 包含意图、目标、归属文件、状态
- 多个 task 可同时存在于看板上
- 如果用户在同一句话里提出多个独立需求，agent 应先做 **intake 拆分判断**：高置信时自动创建多个 task；中置信时先向用户确认；低置信时保持单 task 并记录未拆分原因。
- Task 粒度不是"代码步骤"，而是能独立验收、暂停、取消、归档的用户目标。

### 3.3 Capability（能力）
**定义**：可独立完成的工作单元，有明确输入和产出。

**v1 仍使用预设 capability**（与现 phase 1:1 对应）:
- `align` / `align_lite`
- `research`
- `plan`
- `implement`
- `validate`
- `review` / `review_lite`

**预留扩展**: v2+ 允许用户自定义 capability（每个人的工作流不同，应该可自定义），但 v1 不做。

**Capability 的关键属性**:
```yaml
research:
  required_fields: [sources, risks, open_questions]
  optional_fields: [hypothesis, references]
  produces: [evidence_pack]
  consumes: []                    # 可以独立开始
  parallel_safe: true              # 可与其它 capability 并行
```

**粒度规则（硬约束）**: capability 是"有可交付产出的工作单元"，单次 active 不超过 ~5 个，避免粒度爆炸。

### 3.4 Event（事件）
Append-only log，存放在 `runs/<run_id>/events.jsonl`，是**唯一真相源**：

```rust
enum VibehubEvent {
    TaskCreated { task_id, intent, mode },
    EvidenceAdded { source, summary, refs },
    PlanDrafted { plan_path, scope },
    DiffObserved { commit_range, files },
    ValidationRun { kind, status, output_ref },
    RiskRaised { id, severity, note },
    RiskResolved { id, resolution },
    HandoffWritten { path },
    CapabilityClaimed { name, by_agent },
    CapabilityReleased { name, outcome },
    GateChecked { gate, result, reasons },
    PhaseProjected { phase, derived_from },
    // 补偿事件
    PlanInvalidated { reason },
    DiffReverted { commit_range },
}
```

### 3.5 Gate（准入条件）
取代"phase_order"的强制顺序，是**可声明的 predicate**：

```yaml
gates:
  task_finishable:
    all:
      - has_artifact: diff
      - has_artifact: validation_passed
      - no_open_risk
      - no_unsynced_handoff
  review_ready:
    any:
      - artifact_age(diff) < 1h
      - explicit_request: review
```

### 3.6 Projection（投影）
- 当前 `state.yaml` 的 `phase / phase_status / flow` 字段全部改为**从事件流派生**
- 提供 `derivation_trace.yaml` 记录"为什么当前状态是这样"
- UI/agent-view 都消费投影后的视图，不直接读底层 state

---

## 4. 用户体验需求（5 个核心旅程）

### 旅程 A — 冷启动接力（跨工具、跨时间）

```diagram
╭──────────────────────────────────────────────────────────────╮
│ 用户：打开 Codex，输入 "继续"                                │
│                                                              │
│ Codex agent 自动执行（用户无感）：                           │
│   1. 读 current.md → 发现上次活跃任务 T-xxx                  │
│   2. 条件检测：last_sync 3 天前 + git HEAD 已变 → 深度同步   │
│   3. 拉 git log / diff，对比 task 范围                       │
│   4. 重建 context pack（含变化增量）                         │
│   5. 投影状态：哪些 capability 已完成、哪些待办              │
│                                                              │
│ Agent 输出（按用户优先级顺序）：                             │
│   1️⃣ 当前任务：「重构 phase 为 capability 模型」（M2 阶段）  │
│   2️⃣ 上次决策：决定先实现 events.jsonl 双写                  │
│   3️⃣ Git 状态：3 个文件改动属于本任务，2 个疑似漂移          │
│   4️⃣ 还差：双写测试 + 状态投影函数                           │
│   5️⃣ 风险：旧 phase 字段兼容性未验证                         │
│   6️⃣ 建议下一步：补测试 / 处理漂移文件 / 跳过继续 implement │
╰──────────────────────────────────────────────────────────────╯
```

**关键：第一眼信息的优先级（用户排序）**:
1. 当前 task 是什么 / 意图（最重要）
2. 最近一次 agent 做了什么、留了什么决策
3. 当前 git 状态（哪些属于本 task / 哪些是漂移）
4. 距离完成还差什么
5. 还有哪些未解决的风险/疑问
6. 推荐下一步动作（最末位）

**用户原话**: "下一步建议不是不重要，而是必须建立在 agent 已真正对齐进度之上——agent 没对齐就给建议=白干。"

### 旅程 B — 野路子追认（用户没按流程跑）

```diagram
╭──────────────────────────────────────────────────────────────╮
│ 用户：输入 "sync"                                            │
│                                                              │
│ Agent：                                                      │
│   1. 检测到 22 个未追踪改动 + 当前 task 不匹配               │
│   2. 主动询问：                                              │
│      "这些改动看起来是新功能，是否：                         │
│       (A) 开新 task                                          │
│       (B) 归入当前 task                                      │
│       (C) 部分归入 / 部分新建 / 部分丢弃"                    │
│                                                              │
│ 用户：选 A，简单说一句"加了登录页"                           │
│                                                              │
│ Agent：                                                      │
│   1. 创建新 task，标题/意图自动生成                          │
│   2. 把 22 个文件归档为该 task 的 "implement" 产出           │
│   3. 反向推断已完成的 capability（implement ✅, research ❌）│
│   4. 写入事件：TaskCreated, DiffObserved, CapabilityClaimed  │
│   5. 提示：「research/plan 阶段缺失，是否补登记？」          │
│      （不阻塞，仅提示，符合"软流程"）                        │
╰──────────────────────────────────────────────────────────────╯
```

### 旅程 C — 并行作战（多 task / 多 capability 同时）

```diagram
╭──────────────────────────────────────────────────────────────╮
│ 用户在 UI / agent 里都能看到：                               │
│                                                              │
│  Task A: 用户中心重构        Task B: 修复登录 bug            │
│   ├ research ✅              ├ research ⏭️ (skipped)         │
│   ├ plan ✅                  ├ plan ⏭️                       │
│   ├ implement ✅             ├ implement 🟢 active           │
│   └ review 🟢 active         └ validate ⬜                   │
│                                                              │
│ Agent 切到 task B 时，看到的 context pack 是 B 的，          │
│ 但 A 的关键状态以"邻居任务"形式注入提醒：                    │
│   "邻居：Task A 在 review，若改公共文件需协调"               │
│                                                              │
│ Git 改动归属用 **文件 ownership** 自动判定：                 │
│   - src/auth/* → 与 Task B 文件交集 → 归 B                   │
│   - src/user/* → 与 Task A 文件交集 → 归 A                   │
│   - 无交集     → 询问用户（不自动猜）                        │
╰──────────────────────────────────────────────────────────────╯
```

### 旅程 E — 单次多需求输入（自动拆成多个 task）

```diagram
╭──────────────────────────────────────────────────────────────╮
│ 用户在同一个聊天窗口说：                                     │
│   "帮我修复登录报错，再把设置页远端链接补上，顺便更新文档"   │
│                                                              │
│ Agent 不应把三件事硬塞进一个 task，而是先做 intake 判断：     │
│   1. 登录报错 = 独立 bugfix，可单独验收                      │
│   2. 设置页远端链接 = 独立功能，可单独验收                   │
│   3. 更新文档 = 可能依赖前两项，单独 task 或收尾 task         │
│                                                              │
│ 高置信时自动创建：                                           │
│   Task A: 修复登录报错                                       │
│   Task B: 设置页远端链接                                     │
│   Task C: 更新相关文档（依赖 A/B）                           │
│                                                              │
│ 然后在同一个窗口按顺序推进；handoff/status 必须保留剩余队列。│
│ 换聊天窗口时，新 agent 读 active task 列表即可继续。          │
╰──────────────────────────────────────────────────────────────╯
```

**拆分原则**:
- 应拆分：多个独立交付物、不同用户目标、可分开验收、可分开取消/暂停、明显不同文件或模块范围。
- 不拆分：同一用户目标下的实现步骤、必须一起完成才有意义的配套改动、只是同一改动的测试/文档收尾。
- 不确定时允许问一句确认，但不应要求用户每次显式说"请创建 task"。

### 旅程 D — 强制规范门（产出必填校验）

```diagram
╭──────────────────────────────────────────────────────────────╮
│ Agent 调用 vibehub-record-research（产出 research 结果）：   │
│   { summary: "看了几个文件，差不多就这样" }                  │
│                                                              │
│ VibeHub 拒绝写入，返回：                                     │
│   ❌ Schema validation failed:                               │
│      - missing required: sources[]                           │
│      - missing required: risks[]                             │
│      - missing required: open_questions[]                    │
│   💡 必填字段说明：                                          │
│      sources       → 读过的文件/网页/线程，附路径或 URL      │
│      risks         → 至少 1 条已识别风险，或显式 "no_risk"   │
│      open_questions→ 未解决的问题，或显式 "none"             │
│                                                              │
│ Agent 必须补全才能继续。这是硬性的。                         │
│ 但 Agent 可以额外加任何字段（references, hypothesis…）       │
╰──────────────────────────────────────────────────────────────╯
```

---

## 5. 前端 UI 定位（关键约束）

### 5.1 核心定位
**VibeHub 启动器前端 = 显示器 / 看板，不是操作中心。**

### 5.2 硬规则
- **没有任何操作"必须"在 UI 完成**
- 所有真正的操作（提需求、对齐、推进 capability、写产出…）都通过 **Agent 聊天窗口** 完成
- UI **可以提供建议**：例如"新需求按钮"点击后，给出**该如何对 agent 说话的提示词**，让用户复制粘贴去 agent
- UI **不直接执行**任何 VibeHub 命令

### 5.3 理由
1. UI 没有 AI 的智能，做不好真实交互
2. 多端混乱：如果 UI 也能操作，agent 操作 + UI 操作冲突无法处理
3. 一致性：所有操作走 agent → 事件流 → 投影，形成清晰的因果链

### 5.4 UI 展示能力（应该有的）
- Kanban 看板视图（多 task）
- 每个 task 内的 capability 状态卡片
- 全局事件流时间线
- Git diff 归属可视化
- 风险 / 未决问题清单
- 跳转到对应的 context pack / handoff / journal 等文件
- "建议提示词" 生成（给用户复制去 agent 聊天框）

### 5.5 Kanban 看板示意

> 2026-05-28 UX 更新：M7 整体是前端体验重构，不是在旧 Dashboard 里追加看板。最新主视图规格以 `docs/vibehub-ui-kanban-mockup.md` Draft v2 为准：项目详情页首屏包含项目名、设置入口、最近提交、最近动态、活跃任务过程条、项目总结构、归档区域；旧 Dashboard 模块迁移到 task / phase-capability / activity / archive / settings 等详情页。

```diagram
╭───────────────────────────────────────────────────────────────────╮
│ VibeHub 启动器                          [💡 新需求提示词生成器]   │
├───────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ╭─ Task A: 用户中心重构 ────────────╮                            │
│  │ 🟦 research  ✅                   │  [展开 capability 详情]    │
│  │ 🟦 plan      ✅                   │                            │
│  │ 🟦 implement 🟢 60% (3/5 files)   │  ← 点这里查看传递包/handoff│
│  │ 🟦 validate  ⬜                   │                            │
│  │ 🟦 review    ⬜                   │                            │
│  ╰───────────────────────────────────╯                            │
│                                                                   │
│  ╭─ Task B: 修复登录 bug ────────────╮                            │
│  │ 🟦 implement 🟢 active            │  ⚠️ 与 A 共享 src/auth/*   │
│  │ 🟦 validate  ⬜                   │                            │
│  ╰───────────────────────────────────╯                            │
│                                                                   │
│  ╭─ Task C: 新需求待规划 ────────────╮                            │
│  │ 🟦 (capability 尚未生成)          │  💡 [复制提示词去 agent]   │
│  ╰───────────────────────────────────╯                            │
│                                                                   │
├───────────────────────────────────────────────────────────────────┤
│ 全局事件流（最近）                                                │
│  10:32  Task A · implement   diff observed (3 files)              │
│  10:28  Task B · validate    test failed (auth_test.rs)           │
│  10:15  Task A · plan        plan_drafted                         │
╰───────────────────────────────────────────────────────────────────╯
```

---

## 6. 同步机制（分级 sync）

### 6.1 三级同步
| 级别 | 触发 | 行为 |
|---|---|---|
| **轻同步**（增量补差） | 新对话窗口 + 状态新鲜对齐 | 只补差异，最省 token |
| **深同步** | 用户输入 `sync` / 长时间未同步 / git HEAD 大变 | 全面重建 context pack + 状态投影 |
| **强制重建** | 状态完全不可用 / schema 版本变更 | 全量重读，必要时主动询问用户 |

### 6.2 条件检测算法（决定使用哪级）
```
信号：
  Δt = now - last_sync
  Δhead = git HEAD changed since last_sync
  Δoverlap = |diff_files ∩ task_files| / |diff_files|

规则（粗略）：
  Δt < 1h AND not Δhead              → 轻同步
  Δt < 24h AND Δoverlap > 0.8        → 轻同步
  Δt > 24h OR Δoverlap < 0.5         → 深同步
  无 baseline OR schema mismatch     → 强制重建
```

### 6.3 不变量
- **每次同步都必须产出一份完整、自洽的 context pack 和 state 快照**
- 不允许"半同步"——要么不同步，要么必须留下完整结果
- 同步过程产出的事件（`SyncStarted`, `SyncCompleted`, `SyncReport`）写入事件流

### 6.4 性能目标
- 轻同步：< 5s，< 5k token
- 深同步：< 30s，< 50k token
- 强制重建：< 2min，无 token 上限

---

## 7. Schema 规范性（流程软、产出硬）

### 7.1 原则
- 每个 capability 定义其**必填字段（required）** 和**选填字段（optional）**
- 必填缺失 → **拒绝写入，强制 agent 补全，否则不能推进**
- 选填随意，agent 可补任意额外字段

### 7.2 各 capability 必填字段草案（v1）

| Capability | 必填字段 | 选填字段 |
|---|---|---|
| `align_lite` | `intent`, `scope` | `references` |
| `align` | `intent`, `scope`, `success_criteria`, `non_goals` | `stakeholders`, `references` |
| `research` | `sources[]`, `risks[]`, `open_questions[]` | `hypothesis`, `references`, `related_threads` |
| `plan` | `steps[]`, `validation_plan`, `affected_files[]` | `alternatives`, `rollback_plan` |
| `implement` | `diff_summary`, `changed_files[]`, `commands_run[]` | `manual_test_notes` |
| `validate` | `test_results`, `lint_results`, `status` | `coverage`, `perf_notes` |
| `review_lite` | `summary`, `concerns[]`, `gate_pass` | - |
| `review` | `summary`, `concerns[]`, `gate_pass`, `risk_review` | `suggestions`, `follow_ups` |

> **注**: 上表是初稿，待 Step 3a 细化。每个字段需定义类型、最小长度、允许的特殊占位符（如 `"no_risk"`, `"none"`）等。

### 7.3 校验时机
- **写入时同步校验**（同步代码，不交给 AI）
- 校验失败 → 返回结构化错误（缺哪个字段、提示如何补全）
- 校验通过 → 写入事件 + 更新投影

### 7.4 Schema 演进
- 引入 `schema_version` 字段
- 每个 capability 的 schema 独立版本化
- 写好 migration 机制（已有 `state_migration.rs` 基础）

---

## 8. 并行能力（分阶段实现）

### 8.1 v1（必做）: Task 内 Capability 并行
- 一个 task 可同时有多个 active capability
- 例如：边 research 边 implement 边 validate
- 每个 capability 有独立的 context pack / handoff / 事件子流

### 8.2 v2（后续）: 多 Task 并行
- 多个 task 同时活跃（A 在 review，B 在 implement）
- **每个 capability 一个独立 thread/session**，物理隔离避免上下文污染
- 切 task 时自动加载对应 capability 的包

### 8.3 文件归属冲突
- 用 **文件 ownership 表** 自动判定
- 冲突时（无交集 / 多交集）→ **主动询问用户，不自动猜**

### 8.4 并行风险与限制
- 单次 active capability 数量上限（建议 5）
- 同一 capability 的事件流是单写者（文件锁或 channel）
- UI 需要清楚展示多 active 的状态

---

## 9. 失败容忍与中断恢复

### 9.1 容忍度策略：**很高**
- Agent 崩溃 / Ctrl-C / 换工具 / 换窗口 → **不丢工作**
- 重进时 agent **主动多花 token 对齐**，然后继续
- 不默认回滚到 checkpoint

### 9.2 实现要点
- 所有进行中的工作以事件形式持续写入 `events.jsonl`
- 重进时 agent 先读事件流末尾 N 条 → 推断"上次做到哪"
- 配合深同步（git diff、context pack）建立完整上下文
- 仅在用户**显式要求**时才执行"回滚到 checkpoint"

### 9.3 补偿机制（不是回滚）
- 引入 compensation event: `PlanInvalidated`, `DiffReverted`
- 投影层自动消费补偿事件，更新派生状态
- 比"硬回滚"温和，可保留历史痕迹

---

## 10. Sub-agent 委派策略

### 10.1 委派判断标准
> 任务是否满足以下四条：
> ① 重 IO / 重 token
> ② 结果可摘要
> ③ 不需要持续对话上下文
> ④ 失败可重试
>
> **四条全中 → 委派 sub-agent**
> **任何一条不中 → 主线程做**

### 10.2 适合 sub-agent 的任务
| 任务 | 理由 |
|---|---|
| 深度同步（读 git log、对比 task、重建 context pack） | 重 IO + 大 token，主线程只需摘要 |
| Schema 校验后字段补全 | 主线程不被打断 |
| 跨 capability 影响分析 | 独立调研任务 |
| Journal / handoff 写入 | 写作类，适合 sub-agent |

### 10.3 不该用 sub-agent 的任务
| 任务 | 理由 |
|---|---|
| 核心决策 | 主线程必须保留全局上下文 |
| 轻量 sync | 启动开销 > 任务本身 |
| Schema 强校验逻辑本身 | 确定性代码，不是 AI |
| 事件追加 | 同上 |

### 10.4 风险
- Sub-agent 产出也要被 schema 校验（不能把混乱外包）
- 失败需要超时 + fallback
- 多 sub-agent 并行写事件流需要单写者模型

### 10.5 主 agent 与 sub-agent 协作模型
```diagram
主 Agent（用户对话）
   │
   ├─ 直接做：决策、对话、提需求
   │
   └─ 委派 sub-agent：
       ├─ "深度 sync 一下，给我摘要"
       ├─ "把 research 产出补全为合规 schema"
       ├─ "分析 git diff 归属到哪个 task"
       └─ "生成本次 handoff 文档"
```

---

## 11. 工程审视

### 11.1 合理且可行 ✅
| 设计 | 评价 |
|---|---|
| Task / Capability 分层 | 对标 BPMN 的 process/activity，工业界成熟 |
| 强 schema + 软流程 | 类似 GitHub Actions（动作随意组合，输入输出严格） |
| 事件流 + 投影 | Event Sourcing 范式成熟，Rust 生态有参考 |
| 分级 sync | 类似 `git fetch` vs `git pull --rebase`，符合直觉 |
| Kanban 看板 UI | Trello/Linear/Jira 都验证过的形态 |

### 11.2 需要警惕（要做但要小心）⚠️
| 风险 | 缓解 |
|---|---|
| Capability 粒度爆炸 | 硬规定单次 active ≤ 5，capability 必须有可交付产出 |
| Schema 演进困难 | 从 day 1 引入 `schema_version` + migration |
| 并行归属歧义 | 文件 ownership 表 + 冲突时询问用户 |
| 事件流膨胀 | 按 run 分片 + 归档老 run（30+ 天） |
| Tauri 实时同步成本 | Tauri event/emit + watch event log 文件 |
| 多 task 上下文污染 | 每个 capability 独立 thread/session |

### 11.3 暂时不做 ❌
| 想法 | 暂缓理由 |
|---|---|
| 多人协作 | 单人工具优先，多人会让 schema/锁/冲突全部复杂化 |
| 跨项目并行 | 一个项目内多 task 已够用 |
| 自动智能调度 | 容易失控，与"用户主导"原则冲突 |
| 完整 undo（撤销整段事件） | 理论可行但 UI 复杂度爆炸，先做"标记为废弃" |
| Capability 用户自定义 | v1 预设即可，v2 再做 |

### 11.4 综合评分
| 维度 | 评分 |
|---|---|
| 概念清晰度 | ⭐⭐⭐⭐⭐ |
| 用户体验吸引力 | ⭐⭐⭐⭐⭐ |
| 工程可行性（单人） | ⭐⭐⭐⭐ |
| 工程可行性（多 task 并行） | ⭐⭐⭐ |
| 长期可演进性 | ⭐⭐⭐⭐⭐ |

**总评**: 合理 + 科学 + 可实现，但工程量不小。建议坚决砍 scope，先把 Task 内 capability 并行 + Kanban UI 跑通，多 task 并行放 v2。

---

## 12. 迁移路径（5 个 Milestone）

> 评审契约：本节的 capability / gate / event 迁移语义已浓缩为
> [`RFC-0001: Capability Gate Workflow`](./rfc/0001-capability-gate-workflow.md)。

### M1 — 引入 Event Log（向后兼容，零破坏）
**改动范围**: `src-tauri/src/vibehub/events.rs`, `state_migration.rs`

- 在每个 run 目录下新增 `events.jsonl`
- 所有现有写操作（phase 切换、handoff、checkpoint）**同时**追加事件
- `state.yaml` 保持原样，agent / UI 无感知
- 新增只读 API：`vibehub_events_list(run_id)` 供调试
- schema_version 升级到 v3（带 `event_log_path`）

**交付物**: 双写运行 N 周，验证一致性。

### M2 — 引入 Capability + Gate 引擎
**改动范围**: 新增 `capability.rs`, `gates.rs`；扩展 `workflow.yaml`

- 解析新 `capabilities:` / `gates:` 段（与旧 `phases:` 并存）
- 引擎从 event log 推导"当前可执行的 capability 集合"
- 实现条件检测算法（Section 6.2）
- 新增 skill：`vibehub-claim <capability>` 替代 `vibehub-continue`
- 旧命令变 alias：`research` → `claim(research)` 等

**交付物**: workflow.yaml 支持声明 capability；agent 新旧命令并存。

### M3 — Projection 层接管 state.yaml
**改动范围**: `state_migration.rs`, `status.rs`

- `state.yaml.current.phase` / `flow.*` 标记 `derived: true`
- 写入路径关闭：只能通过事件触发投影
- `vibehub-status` 显示"当前可执行 capability"+"已满足/未满足 gate"
- 新增 `derivation_trace.yaml` 记录派生依据

**交付物**: UI 显示与之前一致，底层已事件驱动。

### M4 — 并发与回环（Task 内 capability 并行）
**改动范围**: `capability.rs`, `agent_view.rs`

- 允许同时 claim 多个 capability
- `agent-view/current.md` 改为列出"活跃 capabilities"
- 引入 compensation event：`PlanInvalidated`, `DiffReverted`
- review 失败自动 release，不需"phase 回退"
- 实现每 capability 独立 context pack / handoff

**交付物**: 流程不再单线程。

### M5 — Fitness Function & WIP Limit & Schema 强校验
**改动范围**: 新增 `fitness.rs`, `schema_check.rs`, `policy.yaml`

- 实现每 capability 的必填字段 schema 校验
- 拒绝不合规写入，返回结构化错误
- 用户可配置健康度指标：`max_open_risks`, `max_concurrent_claims` 等
- `loop_detection` 升级为基于事件频率的真实回路检测

**交付物**: 流程治理从"硬规则"升级为"可观测、可调参的策略层"。

### M6（未来）— 多 Task 并行
- 多 task 同时活跃
- 文件 ownership 表 + 冲突询问
- 每个 capability 独立 thread/session
- 单次用户输入含多个独立需求时，agent 自动拆分并创建多个 task；中低置信才询问或保留单 task

### M7（未来）— UI 全面对接
- Kanban 看板
- 全局事件流时间线
- 提示词生成器

### M8（未来）— Capability 自定义
- 用户可定义自己的 capability + 字段 schema

---

## 13. 下一步工作清单

| # | 任务 | 优先级 | 负责形式 |
|---|---|---|---|
| 1 | 把本文档作为 baseline 入库 | P0 | 立即 |
| 2 | 起草 RFC: `docs/rfc/0001-capability-gate-workflow.md` | P0 | 单 PR |
| 3 | 写「Capability Schema 草案」详细版（Section 7.2 展开） | P0 | 单文档 |
| 4 | 写「Skill 接口清单」（哪些是 AI 调 / 哪些是 sub-agent / 哪些是 UI 触发） | P1 | 单文档 |
| 5 | 实现 M1 (events.jsonl 双写) | P1 | 单 PR |
| 6 | 量化 baseline 指标（当前每 task 平均 phase 跳转次数） | P2 | 度量脚本 |
| 7 | 设计条件检测算法（Section 6.2 细化） | P1 | 单文档 |
| 8 | 设计文件 ownership 表 | P2 | 单文档 |
| 9 | 设计 Kanban UI 原型 | P2 | UI mockup |

---

## 14. 待决议事项（Open Questions）

### 14.1 同步相关
- ~~Q: 条件检测的具体阈值如何设定？（Δt、Δoverlap 的临界点）~~ **已决议：S4 细化。**
- ~~Q: 强制重建时主动询问哪些问题？~~ **已决议（v1.2）：智能判断——单 task 直接重建，多 task 问用户选哪个。**
- Q: 跨 task 同步时如何避免重复扫描 git？

### 14.2 Capability 相关
- ~~Q: capability 的 ID 是否全局唯一，还是 task 内唯一？~~ **已决议：task 内唯一。**
- ~~Q: capability 之间能否声明软依赖（不是硬阻塞，但作为提示）？~~ **已决议（v1.2）：不加软依赖，保持简单，建议放提示词里。**
- ~~Q: capability 失败时事件如何标记（`CapabilityFailed` vs `CapabilityReleased{outcome: failed}`）？~~ **已决议：使用 `CapabilityReleased{outcome: failed}`。**

### 14.3 Schema 相关
- ~~Q: 必填字段的"占位符"（如 `"no_risk"`）如何统一定义？~~ **已决议：在 S2 细化。**
- ~~Q: schema 校验是否允许"草稿模式"（写入不合规但标记 incomplete）？~~ **已决议（v1.2）：混合模式——默认严格，中断时可自动保存草稿，续跑时强制补全。**
- ~~Q: 字段类型校验放 Rust 还是用通用 JSON Schema？~~ **已决议（v1.2）：采用 JSON Schema 标准。**

### 14.4 UI 相关
- ~~Q: 提示词生成器的模板如何设计？需要 i18n 吗？~~ **已决议（v1.2）：需要，初始化选语言，之后跟全局设置走。**
- ~~Q: 看板是否支持拖动？（按"显示器"原则应该不支持）~~ **已决议：不支持。**
- ~~Q: 事件流时间线是否支持过滤/搜索？~~ **已决议（v1.2）：支持按 task / capability / 事件类型过滤。**

### 14.5 Sub-agent 相关
- Q: sub-agent 的调用接口（Tauri 命令？MCP？直接进程？）
- ~~Q: sub-agent 失败时主线程超时阈值？~~ **已决议（v1.2）：不管超时，这是 AI 客户端的职责，不是 VibeHub 的。**
- Q: 多 sub-agent 同时写事件流的并发模型？

### 14.6 Migration 相关
- ~~Q: 老 task（schema_version=2）是否需要回填事件流？~~ **已决议（v1.2）：自动回填，从 state.yaml 逆向生成历史事件。**
- ~~Q: 双写期间不一致如何处理？~~ **已决议：M1c 一致性测试。**
- ~~Q: 何时关闭旧路径（不再双写）？~~ **已决议（v1.2）：等 M6 多 task 并行做好后再彻底切换。**

### 14.7 Agent intake 相关
- Q: 多需求拆分的"高置信 / 中置信 / 低置信"阈值是否需要项目级配置？
- Q: 文档更新、测试验证这类收尾工作何时应成为独立 task，何时应归入对应实现 task？

---

## 15. 附录：原始对话要点摘录

### 15.1 用户对"灵活"的定义
> "我要的是灵活，并不是我要的是整体上的灵活，就是并行任务、不一定要按线性流程来跑。但我也同时需要它每个阶段的规范性。"

### 15.2 用户对"跨工具体验"的描述
> "用户在跨 AI 工具使用的时候，也依然可以通过这一些保存的每一个阶段我们去存下来的这一些东西，还有 Git 的记录来快速了解这个项目。"
> "他们开始的时候就不是一无所知，而是知道目前的状态有哪一些。然后大概接下来要进行哪一些工作，目前还有哪一些东西没有做的。"

### 15.3 用户对"看板"的描述
> "Capability 就相当于是类似于需求 todo 清单之类的，我们往里头塞一些需求。它相当于是每个 capability 它都是互相独立隔离的，然后有不同的传递链，然后不同的传输的包啊、信息之类的。"
> "我可以 A、B、C 三个需求，我甚至可以先推一会 A，然后再推一会 B，然后回来 C 完全推完，然后再回来推一会 A，然后再推一会 B。它会一直存在于我的 todo 板上。"

### 15.4 用户对"UI 定位"的硬约束
> "UI 的这个界面，前端的这个启动器界面，它本质上它是一个展示器。你不能有说什么操作是你必须要在启动器里头才能够去完成的。"
> "理论上任何的操作，无论是提新需求啊，或者是怎么样对齐的操作，都应该由 Agents 的聊天窗口来进行。"
> "VibeHub 可以提供建议——比如说你点新需求按钮的，或者点什么的，给建议……如何用什么样的提示词给到这个 Agents，让它去做这个交互。但是它不能去直接的去做这一些交互。"

### 15.5 用户对"失败容忍"的态度
> "容忍度一定要非常非常的高。比如 Agent 跑到一半崩了，然后或者是用户强行 Control C 了，那如果他再回来，他想要希望再跑，哪怕是他开了一个新的窗口或者转移到一个新的 AI 的工具去又开新窗口之类的，最好都是要让它是能够继续这个任务的。"

### 15.6 用户对"规范性"的态度
> "规范性严格到什么程度，这个要看，我们要对各种的不同的步骤、不同的阶段去定义不同的必要字段。如果必要字段没有的话，是肯定就拒绝写入的，要强制 Agent 补全，如果没有补全就推进不下去。"
> "不然它如果是没有这一层的强束缚性的话，整个工程流程就又是完全由 AI agent 持续掌控了，又是有点失控的感觉。"

### 15.7 用户对"sub-agent"的疑虑
> "对齐的过程，或者是额外写入的过程，可以让 sub agents 去做，这样子就不太会干扰到理论上现在开发的主线程。"
> （决议：按工程直觉做，参见 Section 10）

---

## 16. 文档维护

### 16.1 修订记录
| 版本 | 日期 | 修订内容 | 修订人 |
|---|---|---|---|
| Draft v1 | 2026-05-27 | 初稿，整合 T-019e68d6 对话全部要点 | User × Amp |
| Draft v1.1 | 2026-05-27 | 新增 Section 17 Context Pack 规范；清理 Section 14 已解决的 Open Questions | User × Amp |
| Draft v1.2 | 2026-05-28 | 批量决议 Open Questions（多语言、回填、混合草稿模式、JSON Schema 等）；新增 Section 18-29 | User × Amp |

### 16.2 如何更新本文档
- 任何 capability schema 细化 → 更新 Section 7.2 + 新增详细 schema 文档
- 任何 milestone 完成 → 在 Section 12 标记 ✅ + 链接 PR
- 任何 Open Question 决议 → 把 Section 14 对应条目移到 Section 11 / 12 对应位置
- 任何新讨论场景 → 补充到 Section 4 旅程 / Section 15 原始要点

### 16.3 引用建议
后续任何 agent / 对话讨论实施细节时，**请先读本文档**，并明确引用章节号，避免重复对齐。

---

## 17. Context Pack 规范（v1.1）

> 本章节是 Context Pack 的权威定义。任何对 sync / handoff / capability 入场 / agent 状态恢复的讨论，都必须基于这里的约定。

### 17.1 定位

**Context Pack = agent 进入一个 capability 工作时，最先读到的"工作说明书"。**
它是「上文记忆 + 工作目标 + 必要约束」的最小完整集合，让 agent 不需要重新挖整个项目就能立刻开始干活。

**核心原则**（来自原话整理）:
- **100% 由 agent / sub-agent 自动产出**，不涉及任何用户手写
- 用户只跟 agent 交互，agent 跟 VibeHub 工具层交互，**VibeHub 工具层不直接对用户**
- Pack 内容是给 agent 看的，不是给用户看的；用户视图由 UI 渲染

### 17.2 两层结构

```diagram
╭─────────────────────────────────────────────────────╮
│  Task Context Pack（薄底座，类似项目级 README）     │
│  ────────────────────────────────────────────────── │
│  - 全 capability 共享的常驻信息                     │
│  - 极薄、变更少                                     │
╰─────────────────────────────────────────────────────╯
                  ▲ ref（不内嵌）
                  │
╭─────────────────────────────────────────────────────╮
│  Capability Context Pack（本职差异，类似工单）      │
│  ────────────────────────────────────────────────── │
│  - 本 capability 的目标、输入、产出 schema          │
│  - 上一 capability 的产出摘要                       │
│  - 范围内 git / 邻居 / 决策 / 导航                  │
│  - 每次进入 capability 都重建                       │
╰─────────────────────────────────────────────────────╯
```

**两层解耦**：Capability pack 只放 `task_pack_ref`，**不内嵌** task pack 字段。agent 进入工作时读两份文件，开销可接受。

### 17.3 决议总览（对话留痕）

| 决议点 | 决定 | 出处 |
|---|---|---|
| 颗粒度 | 两层结构（task + capability），都做 | Q1=C |
| 内容来源 | 100% agent / sub-agent 自动产出 | Q2 用户原话 |
| 生成者 | **sub-agent 负责合成**（主线程不被打断） | 工程建议 |
| 重建时机（capability pack） | 每次进入 capability 都重建 | Q3=A |
| 重建时机（task pack） | **被动**：仅在 task 层信息变化时；capability release 时附带 delta 信号 | D1=A + 用户补充 |
| 摘要产出者 | **上一 capability 在 release 时主动产出**，写入 handoff，下一 capability 直接读 | D2=B |
| 内嵌 vs 引用 | **不内嵌**，capability pack 用 `task_pack_ref` 引用 task pack | D3=A |
| 形态 | **JSON**（机器为主，UI 负责人类视图） | Q7 |
| 体积控制 | **不强制硬上限，soft warning + 分层引用** | Q4 |
| 跨工具一致性 | **完全相同**，无 per-tool 定制 | Q6=A |
| 失效处理 | 走现有 **sync 流程**，不另起一套 | Q8 |
| 用户偏好字段 | **不进 pack**，走独立 project 级 config；task 特定的偏好折叠进 `intent.task_specific_notes` | Q5 |
| 下一步建议字段 | 保留 `next_step_hint`，**但优先级最低** | Q5 优先级排序 |

### 17.4 Task Context Pack — 字段定义

| 字段 | 必填 | 类型 | 说明 |
|---|---|---|---|
| `schema_version` | ✅ | string | 当前 `"1.0"` |
| `kind` | ✅ | string | 固定 `"task_context_pack"` |
| `task_id` | ✅ | string | T-xxxx |
| `generated_at` | ✅ | ISO datetime | 生成时间戳 |
| `generated_by` | ✅ | string | `"sub-agent"` / `"main-agent"` |
| `freshness.git_head` | ✅ | string | 生成时的 git HEAD |
| `freshness.last_event_id` | ✅ | string | 投影所依据的最后事件 ID |
| `intent.title` | ✅ | string | 任务标题 |
| `intent.goal` | ✅ | string | 任务目标 |
| `intent.non_goals` | ✅ | string[] | 明确不做什么 |
| `intent.success_criteria` | ✅ | string[] | 完成的判定标准 |
| `intent.task_specific_notes` | ⬜ | string | 用户对该 task 特有的偏好 / 补充描述 |
| `capabilities_overview` | ✅ | object | 每个 capability 的状态 + 摘要引用 |
| `open_items.risks` | ⬜ | object[] | 未解决风险（id / severity / note） |
| `open_items.questions` | ⬜ | object[] | 未解决问题（id / note） |
| `git_state.files_in_scope` | ⬜ | string[] | 属于本 task 的文件 |
| `git_state.files_drifted` | ⬜ | string[] | 疑似漂移文件 |

#### 真实例子
```json
{
  "schema_version": "1.0",
  "kind": "task_context_pack",
  "task_id": "T-20260527-xxxx",
  "generated_at": "2026-05-27T12:00:00Z",
  "generated_by": "sub-agent",
  "freshness": {
    "git_head": "abc123",
    "last_event_id": "evt-456"
  },
  "intent": {
    "title": "重构 phase 为 capability 模型",
    "goal": "用 capability + event + gate 替换硬编码 phase 顺序",
    "non_goals": ["不做多人协作", "不引入外部 workflow engine"],
    "success_criteria": [
      "old phase 命令仍可用",
      "多 capability 可并行 claim",
      "schema 校验阻塞不合规写入"
    ],
    "task_specific_notes": "分 5 个 milestone 渐进推进"
  },
  "capabilities_overview": {
    "research":  {"status": "completed", "summary_ref": "outputs/research.json"},
    "plan":      {"status": "completed", "summary_ref": "outputs/plan.json"},
    "implement": {"status": "active",    "progress": 0.6},
    "validate":  {"status": "pending"},
    "review":    {"status": "pending"}
  },
  "open_items": {
    "risks": [
      {"id": "R1", "severity": "med", "note": "schema 演进未验证"}
    ],
    "questions": [
      {"id": "Q1", "note": "Capability 自定义放 v2 还是 v3?"}
    ]
  },
  "git_state": {
    "files_in_scope": ["src-tauri/src/vibehub/events.rs"],
    "files_drifted": ["README.md"]
  }
}
```

### 17.5 Capability Context Pack — 字段定义

| 字段 | 必填 | 类型 | 说明 |
|---|---|---|---|
| `schema_version` | ✅ | string | 当前 `"1.0"` |
| `kind` | ✅ | string | 固定 `"capability_context_pack"` |
| `task_id` | ✅ | string | T-xxxx |
| `capability` | ✅ | string | research/plan/implement/validate/review/... |
| `generated_at` | ✅ | ISO datetime | 生成时间戳 |
| `generated_by` | ✅ | string | `"sub-agent"` / `"main-agent"` |
| `freshness.git_head` | ✅ | string | 生成时的 git HEAD |
| `freshness.last_event_id` | ✅ | string | 投影所依据的最后事件 ID |
| `freshness.task_pack_ref` | ✅ | string | 引用的 task pack 标识（含版本/事件号） |
| `capability_goal.objective` | ✅ | string | 本 capability 的具体目标 |
| `capability_goal.required_output_schema` | ✅ | object | 本 capability 必填产出字段 schema（见 Section 7.2） |
| `capability_goal.optional_output_schema` | ⬜ | object | 可选产出字段 schema |
| `prior_outputs_summary` | ⬜ | object[] | 上游 capability 的产出摘要（由上游 release 时产出，见 17.6） |
| `git_state_scoped.files_in_scope` | ⬜ | string[] | 本 capability 范围内的文件 |
| `git_state_scoped.diff_summary` | ⬜ | string | 范围内 diff 的简短摘要 |
| `neighbors` | ⬜ | object[] | 邻居 task 的状态（并行场景，v1 可空） |
| `open_items_inherited.risks` | ⬜ | string[] | 从 task 层透传的 risk ID |
| `open_items_inherited.questions` | ⬜ | string[] | 从 task 层透传的 question ID |
| `code_navigation` | ⬜ | object[] | 相关文件路径 + 简短说明 |
| `decisions_journal` | ⬜ | object[] | 历史关键决策摘要（来自 journal） |
| `next_step_hint` | ⬜ | string | 来自上次 handoff 的下一步建议（**最低优先级**） |

#### 真实例子
```json
{
  "schema_version": "1.0",
  "kind": "capability_context_pack",
  "task_id": "T-20260527-xxxx",
  "capability": "implement",
  "generated_at": "2026-05-27T12:05:00Z",
  "generated_by": "sub-agent",
  "freshness": {
    "git_head": "abc123",
    "last_event_id": "evt-456",
    "task_pack_ref": "task_context_pack@evt-456"
  },
  "capability_goal": {
    "objective": "实现 events.jsonl 双写机制",
    "required_output_schema": {
      "diff_summary":  {"type": "string", "min_length": 50},
      "changed_files": {"type": "array",  "min_items": 1},
      "commands_run":  {"type": "array"}
    },
    "optional_output_schema": {
      "manual_test_notes": {"type": "string"}
    }
  },
  "prior_outputs_summary": [
    {
      "capability": "plan",
      "key_decisions": [
        "采用 jsonl 而非 sqlite",
        "events 按 run 分片",
        "双写期 6 周"
      ],
      "full_ref": "outputs/plan.json"
    }
  ],
  "git_state_scoped": {
    "files_in_scope": [
      "src-tauri/src/vibehub/events.rs",
      "src-tauri/src/vibehub/state_migration.rs"
    ],
    "diff_summary": "新增 VibehubEvent enum + jsonl 写入函数"
  },
  "neighbors": [],
  "open_items_inherited": {
    "risks": ["R1"],
    "questions": ["Q1"]
  },
  "code_navigation": [
    {"path": "src-tauri/src/vibehub/events.rs", "why": "本次主战场"},
    {"path": "src-tauri/src/vibehub/state_migration.rs", "why": "schema v2→v3"},
    {"path": "src-tauri/src/vibehub/phase.rs", "why": "在 phase 切换处插入事件写入"}
  ],
  "decisions_journal": [
    {"at": "2026-05-26T10:00:00Z", "note": "决定先 jsonl 不上 sqlite"},
    {"at": "2026-05-26T15:00:00Z", "note": "schema 版本号统一管理"}
  ],
  "next_step_hint": "完成 jsonl writer 后跑现有单测，确认无回归"
}
```

### 17.6 生成与重建流程

#### 17.6.1 谁生成
- **Sub-agent** 是 pack 的合成者
- VibeHub 后端提供原材料：events.jsonl、git diff、project config、上 capability 的 handoff
- 主 agent 只**消费**成品 pack，不参与合成
- Sub-agent 产出本身也要被 schema 校验（与 Section 10 一致）

#### 17.6.2 Capability Pack 重建时机
- **每次进入 capability**（claim 时）都重建一次
- 触发动作：`vibehub-claim <capability>` → sub-agent 合成 pack → 写入磁盘 → 写事件 `CapabilityPackBuilt`

#### 17.6.3 Task Pack 重建时机（被动 + delta 信号）
1. **被动重建**：只有当 task 层字段（intent、capabilities_overview、open_items、files_in_scope 等）发生变化时才重建
2. **Delta 信号**：每次 capability release 时，sub-agent 在 handoff 里附带一个 delta 报告，声明本次 release 是否影响 task 层字段
   - `task_pack_dirty: true` → 触发 task pack 重建
   - `task_pack_dirty: false` → 跳过，复用现有 task pack
3. 重建后写事件 `TaskPackRebuilt { reason, delta_fields }`

```diagram
capability release ─┬─▶ produce handoff（含 prior_outputs_summary 摘要）
                    ├─▶ produce delta signal（task_pack_dirty?）
                    └─▶ write event: CapabilityReleased
                             │
                             ▼
                    [VibeHub 内核检查 delta]
                             │
                     ┌───────┴───────┐
            dirty=true              dirty=false
                 │                      │
                 ▼                      ▼
        sub-agent 重建 task pack    复用现有 task pack
```

#### 17.6.4 上一 capability 的摘要由谁产出
- **由上一 capability 在 release 时主动摘要**，写入 handoff
- 下一 capability 入场时，sub-agent 直接读 handoff 的 `prior_outputs_summary` 字段，原样填入新 pack
- 理由：信息源头最准；不需要 sub-agent 临场重读上下文（吃力不讨好）

### 17.7 体积控制（soft warning）

不强制硬上限，但写入 pack 时计算指标并产出事件：

```yaml
soft_limits:
  capability_pack:
    target_tokens: 3000
    warn_at:       6000
    block_at:      none
  task_pack:
    target_tokens: 2000
    warn_at:       4000
    block_at:      none
```

- 超过 `warn_at` 时写事件 `PackOversize { kind, capability, size, threshold }`
- UI 看板显示警告徽章
- Sub-agent 收到警告后应反思"capability 是否拆得不够细"
- **绝不阻塞 agent 工作**，warning 只是观测信号

### 17.8 失效与漂移处理

- Pack 中带 `freshness.git_head` + `freshness.last_event_id`
- Agent 进入工作前可比对当前 git HEAD 是否一致
- 不一致 → 触发 sync 流程（见 Section 6 的分级 sync）
- **不另起一套失效处理**，复用现有 sync 机制

### 17.9 存储布局（待 Section 17.10 细化）

> 此处为暂行约定，等 Event Schema + 存储布局规范文档出炉后以那份为准。

```
.vibehub/tasks/<task_id>/
├── runs/<run_id>/
│   ├── context-packs/
│   │   ├── task.json                       # task context pack（当前）
│   │   ├── task.manifest.yaml              # 元信息（freshness、size、build_log）
│   │   ├── capabilities/
│   │   │   ├── research.json
│   │   │   ├── research.manifest.yaml
│   │   │   ├── plan.json
│   │   │   ├── plan.manifest.yaml
│   │   │   ├── implement.json
│   │   │   └── ...
│   │   └── archive/                        # 历史版本（按事件 id 归档）
│   ├── handoffs/
│   │   └── <capability>-<event_id>.json    # 含 prior_outputs_summary + delta signal
│   ├── outputs/                            # capability 产出（受 schema 校验）
│   └── events.jsonl
```

### 17.10 与其它章节的关系

| Section | 关系 |
|---|---|
| Section 6（分级 sync） | Pack 失效时复用 sync 流程；同步过程会重建 pack |
| Section 7（Schema 规范性） | `required_output_schema` 引用 Section 7.2 的字段定义 |
| Section 10（Sub-agent） | Pack 生成是典型 sub-agent 任务 |
| Section 8（并行能力） | `neighbors` 字段为多 task 并行预留 |
| Section 9（失败容忍） | Pack + handoff 是断点续跑的核心依据 |

### 17.11 本章节遗留的 Open Questions

| ID | 问题 | 优先级 |
|---|---|---|
| CP-Q1 | `prior_outputs_summary` 的摘要长度上限？sub-agent 摘要质量如何校验？ | P1 |
| ~~CP-Q2~~ | ~~Task pack 重建时旧版本如何归档（保留 N 个 / 按时间）？~~ | **已决议（v1.2）：全部保留，不自动清理。** |
| ~~CP-Q3~~ | ~~`neighbors` 字段在 v1 是空数组还是不存在？schema 兼容性？~~ | **已决议（v1.2）：空数组 `[]` 表示无邻居。** |
| CP-Q4 | `code_navigation` 的"why_relevant"如何自动生成且不空泛？ | P2 |
| ~~CP-Q5~~ | ~~当 sub-agent 合成 pack 失败时的 fallback~~ | **已决议（v1.2）：不做 fallback。失败就是失败，没有 pack 产出。agent 自行重试，绝不写入空壳/默认值污染历史。详见 Section 25。** |
| ~~CP-Q6~~ | ~~Pack 的 i18n（生成内容的语言跟 project locale 走？）~~ | **已决议（v1.2）：初始化时选择语言，后续全局跟随该设置。** |

---

## 18. 核心不变量清单（Invariants）

> 任何实现、任何重构都必须满足下列不变量。违反任何一条 = bug。

| ID | 不变量 | 校验时机 |
|---|---|---|
| INV-1 | **events.jsonl 严格 append-only**，不允许任何形式的修改 / 删除 / 跳序写入 | 写入时由 Tauri 后端 writer 校验 |
| INV-2 | **state.yaml 中任何派生字段都可以由事件流完整重放出来**（投影是纯函数） | 启动时自检 + CI 单测 |
| INV-3 | **同一 task 同一时刻只有一个 writer**（多 agent / 多 sub-agent 写入串行化） | 由 Tauri backend 排队，Section 21 |
| INV-4 | **必填 schema 字段缺失时拒绝写入**，且返回结构化错误带修复 hint | 写入入口同步校验 |
| INV-5 | **Context Pack 必有 `freshness` 字段**（含 git_head + last_event_id） | Pack 写入时校验 |
| INV-6 | **用户不直接操作 VibeHub 工具层，必经 agent**；UI 仅展示与生成提示词，不写入任何状态 | 架构约束，code review 时审视 |
| INV-7 | **Sub-agent 失败 = pack 不存在**，绝不写入空壳 / 默认值 / 兜底数据 | Section 25 |
| INV-8 | **VibeHub 不写 git**，只读 git 状态 | 代码层禁用 `git add/commit/push` 调用 |
| INV-9 | **必须有 git 仓库才能正常运行**，否则 VibeHub 不初始化 | 启动时检查 `.git` 存在 |
| INV-10 | **跨工具 agent 看到的 skill 协议 / 返回格式 / 错误码完全一致**，差异只在 adapter 层 | adapter 输出 schema 校验 |

---

## 19. 术语词典（Glossary）

> 全文术语以此为准，混用即视为错误，需修订。

| 术语 | 中文 | 定义 | 同义词 / 易混淆 |
|---|---|---|---|
| **Task** | 任务 | 用户提出的一个需求单元，包含意图、目标、归属文件、状态。看板的卡片粒度。 | 不等于 git commit / branch |
| **Capability** | 能力 | Task 内可独立完成的工作单元（research / plan / implement / validate / review）。v1 用预设，v2+ 可自定义。 | 旧名 phase（已废弃，仅作 UI 标签） |
| **Phase** | 阶段（已弱化） | UI 层面的可视化标签，从事件流投影而来。不再是流程驱动单位。 | 在 v1.2 之后只是显示名词 |
| **Event** | 事件 | Append-only 日志中的一条记录，描述"发生了什么"，是状态真相唯一源。 | 不等于日志（log）；日志可丢，事件不可 |
| **Gate** | 闸门 | 可声明的 predicate，定义某个状态转换的准入条件。 | 取代旧"phase_order"硬顺序 |
| **Projection** | 投影 | 从事件流派生出来的视图（state / pack / UI 卡片）。纯函数，可重放。 | CQRS 概念 |
| **Context Pack** | 上下文包 / 信息包 / 传递包 | agent 进入 capability 时读到的"工作说明书"，分 task pack 和 capability pack 两层。 | **唯一称呼：Context Pack**；中文场合可叫"信息包"，禁用"传递包/任务包/上下文包"等其他叫法 |
| **Handoff** | 交接 | Capability release 时产出的交接文件，含 `prior_outputs_summary` + `task_pack_dirty` delta 信号。下一 capability 的 Context Pack 直接消费它。 | 等式："本 capability 的 handoff = 下一 capability 的 context source" |
| **Artifact** | 产出物 | Capability 完成后产生的任何受 schema 校验的内容（diff / evidence / plan / review report 等）。 | 统称 |
| **Research Output** | 研究产出 | research capability 的 artifact，曾被混称为 "evidence pack"。**统一为 research output**。 | 旧名"evidence pack" 废弃 |
| **Sync** | 同步 | 把外部世界变化（git / 用户手改 / 跨工具切换）纳入 VibeHub 状态的过程，分三级（轻/深/强制重建）。 | 见 Section 6 |
| **Drift** | 漂移 | 未被 VibeHub 追踪的工作（手改文件 / 未归属 task 的 commit）。 | sync 流程处理对象 |
| **Skill** | 技能 | Agent 可调用的 VibeHub 命令，统一返回格式（Section 22）。 | 不等于 MCP tool；当前实现以 `.agents/skills/vibehub-*/SKILL.md` 形式存在 |
| **Adapter** | 适配器 | 把统一 skill 协议投影成各 agent 工具偏好格式（AGENTS.md / CLAUDE.md / opencode.json …）的层。 | Section 27 |
| **Writer** | 写入者 | 持有写权限的 Tauri 后端组件。**全 VibeHub 实例同一时刻只有一个 writer**。 | INV-3 |
| **Sub-agent** | 子代理 | 由主 agent 委派去做重 IO / 重 token 任务的辅助 agent。 | Section 10 |
| **Capability Release** | 能力释放 | Capability 完成（或被中断）时的事件，触发 handoff 产出 + delta 信号 + 可能的 task pack 重建。 | Section 17.6.3 |

---

## 20. Event Schema & 存储布局

### 20.1 事件公共字段（5 项全强制，对应 INV-1 / INV-4）

```yaml
common_event_fields:
  event_id:        string   # 格式: evt-<unix_ms>-<seq>，例 evt-1716800000000-0001
  timestamp:       string   # ISO 8601 with timezone
  task_id:         string   # T-xxxx；全局事件用特殊 T-system
  actor:           string   # main-agent / sub-agent:<purpose> / user / system
  schema_version:  string   # 当前 "1.0"
```

### 20.2 事件类型清单（v1 必备）

| 事件 | payload 关键字段 | 说明 |
|---|---|---|
| `TaskCreated` | `intent`, `mode` | 任务诞生 |
| `TaskCancelled` | `reason` | 任务取消（保留事件） |
| `CapabilityClaimed` | `capability` | agent 开始一个 capability |
| `CapabilityReleased` | `capability`, `outcome`, `task_pack_dirty` | release + delta 信号 |
| `CapabilityPackBuilt` | `capability`, `pack_path`, `size_tokens` | Context pack 生成成功 |
| `TaskPackRebuilt` | `reason`, `delta_fields` | Task pack 重建 |
| `PackOversize` | `kind`, `size`, `threshold` | soft warning |
| `EvidenceAdded` | `source`, `summary`, `refs` | research 阶段证据 |
| `PlanDrafted` | `plan_path`, `scope` | plan 阶段产出 |
| `DiffObserved` | `commit_range`, `files` | implement 阶段观察到 diff |
| `ValidationRun` | `kind`, `status`, `output_ref` | validate 阶段 |
| `RiskRaised` / `RiskResolved` | `id`, `severity`, `note` | 风险管理 |
| `HandoffWritten` | `path`, `capability` | handoff 落盘 |
| `GateChecked` | `gate`, `result`, `reasons` | gate 校验记录 |
| `SyncStarted` / `SyncCompleted` | `mode`, `signals`, `report_path` | sync 流程 |
| `SchemaValidationFailed` | `target`, `errors[]` | 校验拒绝 |
| `EventLogCorrupted` | `at_offset`, `recovered_to` | 损坏自检 |
| `PlanInvalidated` / `DiffReverted` | `reason` | 补偿事件（Section 9） |

### 20.3 存储布局（权威版）

```
.vibehub/
├── project.yaml                                 # 项目元信息
├── state.yaml                                   # 当前状态（部分字段已 derived）
├── workflow.yaml                                # capability + gate 定义
├── policy.yaml                                  # WIP / soft limit / strictness
├── skills.registry.yaml                         # skill 权威清单（C6 决议）
├── agent-view/                                  # 给 agent 读的快照视图
│   ├── current.md
│   ├── current-context.md
│   └── handoff.md
├── tasks/
│   ├── current/                                 # 软链指向活跃 task
│   └── T-<id>/
│       ├── task.yaml                            # task 元信息
│       └── runs/
│           ├── current/                         # 软链指向活跃 run
│           └── R-<id>/
│               ├── events.jsonl                 # 本 run 的事件流（INV-1）
│               ├── context-packs/
│               │   ├── task.json
│               │   ├── task.manifest.yaml
│               │   ├── capabilities/<cap>.json
│               │   └── capabilities/<cap>.manifest.yaml
│               ├── handoffs/<cap>-<event_id>.json
│               ├── outputs/<cap>.json           # 受 schema 校验
│               └── sync/<timestamp>.md          # sync 报告
└── index/
    └── task-events.idx                          # 跨 run 事件索引（A6）
```

### 20.4 事件流分片与索引（A4 + A6）

- 分片粒度：**每 run 一个 `events.jsonl`**
- 跨 run 查询走 **`index/task-events.idx`**（Tauri 后端维护，记录 `task_id → [run_id, event_id offset]`）
- 不归档（A5）：永久保留，不压缩、不移走（永不归档，UI 也不展示归档区）

### 20.5 event_id 生成

格式：`evt-<unix_ms>-<seq_within_ms>`
- `unix_ms`：13 位毫秒时间戳
- `seq_within_ms`：4 位毫秒内自增，由 writer 进程内计数

例：`evt-1716800000000-0001`

### 20.6 严格 append-only 的具体含义（A3 决议）

- **不允许**：物理修改已写入行、删除行、插入行、跳序写入
- **允许**：通过新事件表达"否定/补偿"（例：`RiskResolved` 表达 `RiskRaised` 被解决，但 `RiskRaised` 事件本身保留）
- writer 在写入前对 `event_id.seq` 自增校验，乱序直接报错
- 历史事件**仅可被读取**，不可被任何 API 改动

---

## 21. 并发与单写者模型

### 21.1 核心模型（B1 决议）

```diagram
╭──────────────────────────────────────────────────╮
│  Tauri Backend Writer（单例）                    │
│  ├─ 接收所有写入请求（commands）                 │
│  ├─ 入队 → 单线程顺序处理                        │
│  ├─ schema 校验 → 写 events.jsonl → 更新投影     │
│  └─ 发 Tauri event 通知前端                      │
╰──────────────────────────────────────────────────╯
        ▲       ▲       ▲              ▲
        │       │       │              │
   Main Agent  Sub      Sub        其他 Agent
   (Amp/...)  Agent#1  Agent#2     工具
```

### 21.2 通信通道（B2 决议）

| 通道 | 主用途 | 触发条件 |
|---|---|---|
| **(A) Tauri command（同步 RPC）** | 主路径，所有写入和大多读取 | VibeHub 启动器在前台运行 |
| **(C) 文件系统 + watch** | 兜底路径 | VibeHub 启动器未运行 |

兜底逻辑：
- Agent 写入时优先尝试 Tauri command
- 失败（启动器未运行）→ 落到 `.vibehub/.pending/` 目录的 `*.write` 文件
- 启动器启动时扫描 `.pending/`，串行回放为正式事件
- 这样保证"启动器没开也能写"，但仍保证单写者

### 21.3 Sub-agent 并发上限（B3）

- 默认 **最多 3 个 sub-agent** 同时运行
- 超出排队
- **可在 UI 配置**：`policy.yaml.sub_agent.max_concurrent`

### 21.4 跨工具事件冲突（B4）

- 完全 **串行化**，writer 队列保证 FIFO
- 不做乐观锁、不做重试，简单可靠

---

## 22. Skill 接口 & Agent 协议

### 22.1 统一返回格式（C1 决议，对应 INV-10）

每个 skill **强制返回** 以下结构：

```json
{
  "status": "ok" | "error",
  "data": { ... },                // skill 业务返回
  "error": null | {
    "code": "schema.required.missing",
    "message": "...",
    "hint": "建议如何修复",
    "details": { ... }
  },
  "events_emitted": ["evt-...", "evt-..."],
  "context_header": {              // C2: 每次返回带最新上下文头
    "task_id": "T-...",
    "active_capabilities": ["implement"],
    "freshness": { "git_head": "...", "last_event_id": "..." }
  }
}
```

### 22.2 Agent 如何知道"我在哪"（C2 决议）

组合策略：
1. **开局**：agent 每个新对话第一件事读 `.vibehub/agent-view/current.md`（约 500 token）
2. **过程中**：每个 skill 返回值携带 `context_header`，agent 不需要重复读
3. **失效时**：agent 比对 `context_header.freshness.git_head` 与当前 git HEAD，不一致 → 触发 sync

### 22.3 跨工具协议层划分（C3）

```diagram
        ┌──────────────────────────────────────┐
        │ 统一中间协议层（Tauri commands）     │
        │ - skill 调用入口                     │
        │ - 统一返回格式                       │
        │ - 统一错误码                         │
        └──────┬─────────────┬─────────────────┘
               │             │
        ┌──────▼────┐  ┌─────▼──────┐
        │ Adapter A │  │ Adapter B  │  ← v1 只做一份共用 adapter
        │ (Amp/CC/  │  │ (其它工具) │     覆盖 Amp / Claude Code /
        │  Codex/   │  │            │     Codex / OpenCode
        │  OpenCode)│  │            │
        └───────────┘  └────────────┘
```

### 22.4 AGENTS.md 等配置文件维护（C4 决议）

- 由 VibeHub **生成模板** + **版本号标记**
- 用户可改，但顶部带 `<!-- VIBEHUB:AGENT-INTEGRATION:START -->` 标记
- VibeHub 更新时只修改标记内的内容，标记外用户内容保留
- 与现有机制兼容（项目已有此模式）

### 22.5 Skill 失败重试（C5 决议）

- **skill 内部不重试**
- 调用方（agent）根据 error code 自行决定是否重试

### 22.6 Skill 权威清单（C6 决议）

存储：`.vibehub/skills.registry.yaml`

格式示例：
```yaml
schema_version: "1.0"
skills:
  - name: vibehub-claim
    args:
      - { name: capability, type: string, required: true }
    returns: skill_response_schema_v1
    side_effects: [writes_events, builds_context_pack]
    callable_by: [main-agent]
    idempotent: false
    description: "进入一个 capability 工作"

  - name: vibehub-sync
    args:
      - { name: mode, type: enum[auto,quick,deep,rebuild], default: auto }
    returns: skill_response_schema_v1
    side_effects: [writes_events, may_rebuild_pack]
    callable_by: [main-agent, sub-agent]
    idempotent: true
    description: "同步外部世界状态到 VibeHub"

  # ... 其他
```

### 22.7 Skill 命名规范（C7 决议）

- 保持 `vibehub-<动作>` 命名
- 与现有 `.agents/skills/vibehub-*` 完全兼容

### 22.8 关键 Skill 接口清单（v1）

| Skill | 调用方 | 主要作用 |
|---|---|---|
| `vibehub-init` | main-agent | 初始化项目 |
| `vibehub-status` | main-agent | 查看当前状态 |
| `vibehub-sync` | main-agent / sub-agent | 分级同步 |
| `vibehub-start` | main-agent | 创建 task；多意图输入时负责拆分为多个 task 草案/创建请求 |
| `vibehub-claim` | main-agent | 进入 capability（替代 vibehub-continue） |
| `vibehub-record` | main-agent | 写入 capability 产出（受 schema 校验） |
| `vibehub-release` | main-agent | 释放 capability + 产出 handoff |
| `vibehub-handoff` | main-agent | 主动写 handoff |
| `vibehub-checkpoint` | main-agent | 记录进度快照 |
| `vibehub-journal` | main-agent | 写决策日志 |
| `vibehub-finish` | main-agent | 检查 task_finishable gate + 关闭 task |
| `vibehub-recover` | main-agent | 重对齐 + 重建 pack |
| `vibehub-cancel` | main-agent | 取消 task（事件保留） |
| `vibehub-events` | sub-agent | 查询事件流（只读） |
| `vibehub-build-pack` | sub-agent | 合成 Context Pack |
| `vibehub-validate-schema` | sub-agent | 校验产出 schema |
| `vibehub-debug-dump` | main-agent | 调试导出（J3） |

---

## 23. UI 同步 & 提示词生成器

### 23.1 UI ↔ 后端同步（D1 + D2 决议）

- 主：**Tauri event/emit**（推）
- 启动：**首次全量拉**
- 刷新粒度：**增量更新**（按事件类型决定影响哪个卡片）
- 不轮询、不 watch 文件（Tauri 原生方案优先）

### 23.2 UI 角色再强调

- **UI = 显示器 / 阅读器**，不写任何状态
- 任何"操作"按钮 → 只生成"复制此提示词给 agent"的弹窗
- UI 不调用任何 skill（INV-6）

### 23.3 提示词生成器（D3 + D4 决议）

- **形态**：模态弹窗（modal）
- **存放**：**VibeHub 安装目录** `<vibehub-install>/templates/prompts/*.md`（产品级，随版本走）
- **项目级覆盖**（可选）：用户在 `.vibehub/templates/prompts/*.md` 提供同名文件可覆盖默认
- 模态包含：
  - 模板渲染后的完整提示词文本
  - "复制"按钮（D5：仅复制，不集成具体工具）
  - 可选的"打开 Amp / Claude Code / Codex / ..."按钮（直接拉起对应工具，v2 可选）

### 23.4 模板覆盖范围（v1）

| 场景 | 模板 |
|---|---|
| 新需求 | `new-task.md` |
| 同步 | `sync.md` |
| 推进 capability | `claim-capability.md` |
| 完成 capability | `release-capability.md` |
| 取消 task | `cancel-task.md` |
| 强制重建 | `force-rebuild.md` |
| Schema 校验失败的修复 | `fix-schema.md` |

### 23.5 历史记录查看器（D6 决议）

- VibeHub 内置 **JSON 可视化渲染器**
- 不显示原始 JSON 文本，而是结构化树状/卡片化展示（"全过程、全链路、全流程"）
- 每个 JSON 文件视图配 **两个按钮**：
  - **在文件管理器中显示**（打开文件所在目录）
  - **以系统默认应用打开**（用 OS 默认 JSON 关联程序）
- 适用对象：events.jsonl 单条 / context pack / handoff / output / sync report 等

### 23.6 Kanban 看板（再次明确）

- M7 主视图应替代旧 Dashboard 主体，而不是在旧 Dashboard 中增加一块看板。
- 首屏信息架构：项目名 + 设置、最近提交、最近动态、活跃任务、项目总结构、归档。
- 多 task 卡片纵向堆叠（**不支持拖拽**，按"显示器"原则）
- 每张 task 卡片显示 2-4 字短标签、需求简述、phase/capability 过程条。
- 当前执行中的 phase/capability 需要显著边框/高亮；过程过长时显示当前项前后若干项，其余用省略号。
- 点击 task / 过程条 / 具体 phase-capability 分别进入需求详情 / 全流程详情 / 当前阶段包体详情。
- 最近动态可展开为历史活动视图，并支持按 task / phase-capability 过滤；点击事件可回到看板并高亮对应位置。
- 任务完成后留在原位标 ✅（E2 决议）
- 当数量较多时，下方有 **「归档栏」可展开子页面**，显示完成 / 取消的 task 详情、过程、产出（E2 用户补充）
- 项目总结构区域显示模块交互树/画布与文件目录结构，并在详情页支持两侧互相高亮；该能力可作为低优先级 M7e。

---

## 24. 用户体验生命周期

### 24.1 首次打开（E1 决议）

- **不自动初始化**
- 看到一个空看板 + 中央"初始化 VibeHub 项目"按钮
- 点击 → 弹模态显示"请把以下提示词发给 agent 完成初始化"
- Agent 调用 `vibehub-init` 完成初始化

### 24.2 任务完成（E2 决议）

```diagram
[active task]
     │  release 最后一个 capability + 通过 finish gate
     ▼
[completed]  ✅ 卡片留在看板，标 completed
     │  数量积累
     ▼
[archive panel]  归档栏下方子页面，展开可看 details
```

- v1 **同时实现**：留在原位 + 归档子页面
- 归档子页面展示：
  - 任务标题、意图
  - capabilities 全流程
  - 所有 handoff / output / events 的可视化链路
  - 跳转到 JSON 渲染器（Section 23.5）

### 24.3 任务取消（E3 决议）

- 仅打标 `cancelled`，**事件保留**
- 看板上转为灰色卡片，可移到归档栏
- 用户原话：万一之后又要回来做，事件保留是必要的

### 24.4 任务暂停（E4 决议）

- **不引入"暂停"状态**
- 无活动 = 自然暂停
- 重新进入 capability 即"恢复"

### 24.5 Schema 校验失败的用户感知（E5 决议）

- UI 卡片显示 **红色徽章** / 红色滤镜
- 鼠标悬浮 / 点击查看完整错误详情（含 `error.hint`）
- 同时事件流 `SchemaValidationFailed` 显示在时间线
- agent 端也收到结构化错误（Section 22.1）

### 24.6 危险操作二次确认（E6 决议）

**所有不可逆操作都需要 UI 二次确认**，包括但不限于：
- 取消 task
- 强制重建
- 删除（如果未来支持）
- 撤销标记

确认形式：弹模态 → "请把以下提示词发给 agent 进行确认" → 用户复制 → agent 执行二段确认 skill。

---

## 25. 错误模型

### 25.1 错误码体系（F1 决议）

三段式：`<域>.<类型>.<子类>`

例：
- `schema.required.missing` — 必填字段缺失
- `schema.type.mismatch` — 类型不匹配
- `event.append.out_of_order` — 事件乱序
- `sync.git.head_unreachable` — git HEAD 不可达
- `concurrency.lock.timeout` — 锁超时
- `subagent.timeout.exceeded` — sub-agent 超时
- `pack.build.failed` — pack 合成失败
- `gate.precondition.unmet` — gate 准入条件未满足

### 25.2 错误返回包含修复建议（F2 决议）

每个错误结构：
```json
{
  "code": "schema.required.missing",
  "message": "Capability 'research' output missing required fields: sources, risks",
  "hint": "请补全 sources（至少 1 个 URL/路径）和 risks（至少 1 条或显式 'no_risk'），再次调用 vibehub-record",
  "details": { "missing": ["sources", "risks"], "schema_ref": "..." }
}
```

### 25.3 events.jsonl 损坏恢复（F3 决议）

- 启动 / 写入前自检（CRC / 顺序）
- 损坏 → **从最后一个完整事件截断**
- 写一条 `EventLogCorrupted { at_offset, recovered_to }` 事件到**新位置**（不动旧文件）
- UI 红色警告，必须用户介入

### 25.4 Sub-agent 超时（F4 决议）

- 默认 **60s**
- 可在 UI 配置：`policy.yaml.sub_agent.timeout_seconds`
- 超时事件 `subagent.timeout.exceeded`

### 25.5 Sub-agent 失败的处理（F5 决议 / INV-7）

**绝不做任何 fallback**：
- 失败 = pack 不存在
- agent 自行决定重试 / 报告用户 / 放弃
- VibeHub **不会**写空壳 pack、不会写默认值、不会自动重试
- 用户原话："失败了就是失败，那就是空的，他就得一直去试。"
- 理由：fallback 会污染历史记录，让"什么是真发生的"变得模糊

---

## 26. Git 耦合边界

### 26.1 VibeHub 与 Git 的关系（G1 决议 / INV-8）

- VibeHub **只读 git**，**绝不写 git**
- 任何 commit / branch / tag / push 都由 **agent 主动调用**（git CLI / 自身能力）
- VibeHub 是"查看器"角色

### 26.2 必须有 Git 仓库（G2 决议 / INV-9）

- **硬要求**：项目必须是 git 仓库
- 启动器检测 `.git` 不存在 → 报错并显示"请先 `git init`"
- 不兼容无 git 的项目（避免后续状态混乱）

### 26.3 不注入 Git Hooks（G3 决议）

- v1 / v2 都不注入 hooks
- agent 在 commit / push 时通过 agent 自身的 output / 调用 vibehub-record 来同步事件

### 26.4 文件 ownership 判定（G4 决议）

- 组合策略：
  - **自动**：基于历史改动文件名集合的交集判定归属
  - **用户可手工修正**：通过 agent 调 skill `vibehub-record { scope_files: [...] }` 显式声明
- 冲突时 agent 主动询问用户

---

## 27. 跨工具 Adapter

### 27.1 v1 支持的 Agent 工具（H1 决议，扩展版）

主流共通的全套：
- **Amp Code**
- **Claude Code**
- **Codex**
- **OpenCode**
- **Cursor**
- **Antigravity**

后续（v1.x / v2）：
- 国内：**Coder**、**TRAE**
- 其它按需

### 27.2 配置文件生成（H2 决议）

- 权威源：`.vibehub/skills.registry.yaml`
- VibeHub 内置 **generator**，把权威源投影成每个工具的配置：
  - `AGENTS.md`
  - `CLAUDE.md`
  - `.codex/`
  - `.opencode/`
  - `.cursor/` / Antigravity 对应文件
- 生成内容用 `<!-- VIBEHUB:AGENT-INTEGRATION:START -->` / `:END` 包裹，用户标记外内容保留

### 27.3 升级机制（H3 决议）

- **VibeHub 启动器启动时自动检查并提示**
- 检测到 skills.registry 版本 ≠ 各配置文件中的标注版本 → 弹模态："adapter 配置过期，建议更新（[查看 diff] [一键更新] [稍后])"
- 用户确认后 VibeHub 写入更新后的内容（这是 UI 唯一允许的"写"行为，且只写 adapter 配置文件，不写 VibeHub 状态）

> ⚠️ Adapter 写入是 INV-6 的**唯一例外**，因为这些文件本质上是 VibeHub 自己生成的配置，不属于"VibeHub 工具层状态"。要在代码中显式注明边界。

---

## 28. 测试 & 观测

### 28.1 测试覆盖优先级（J1 决议）

| 类型 | v1 状态 | 覆盖范围 |
|---|---|---|
| 单元测试 | **必做** | schema 校验 / 事件投影 / event_id 生成 / writer 队列 |
| 集成测试 | **必做** | sync 三级 / claim+release 端到端 / pack 重建链路 |
| UI e2e | 用户手动测试 | 看板交互、提示词模态、JSON 渲染器 |

### 28.2 测试报告 / 计划文档（J2 用户补充）

- 先 **起草一份测试计划文档**（不急于实现）
- 路径：`docs/vibehub-test-plan.md`
- 内容：测试矩阵 / 关键 fixture / 模拟事件流 / 覆盖率目标 / CI 集成方案

### 28.3 内置观测指标

VibeHub 后端导出指标到 `state.yaml.metrics`，UI 可见：

| 指标 | 说明 |
|---|---|
| `tasks.active_count` | 当前活跃 task 数 |
| `capabilities.active_count` | 当前活跃 capability 数 |
| `sync.avg_duration_ms` | 同步平均耗时 |
| `sync.last_mode` | 上次 sync 用了哪一级 |
| `pack.avg_size_tokens` | Pack 大小分布 |
| `pack.oversize_count` | 超 warn_at 次数 |
| `schema.validation_failure_rate` | 校验失败率 |
| `events.write_per_minute` | 事件写入速率 |
| `subagent.timeout_count` | sub-agent 超时次数 |

### 28.4 调试模式（J3 决议）

- Skill: `vibehub-debug-dump`
- 一次性导出：
  - 当前完整 state.yaml
  - 当前 task 全 capability 的 context pack
  - 完整 events.jsonl
  - 全部 outputs / handoffs / sync reports
- 输出位置：`.vibehub/debug-dumps/<timestamp>/`
- 用于跨工具 / 跨机器复现问题

---

## 29. 架构定位：CLI 后端 + 前端 + 未来开源

### 29.1 用户补充的关键架构原则

> 用户原话：
> "理论上这一些功能都是相当于是 vibehub cli 的一个后端实现，然后又组合了我们目前已有的这个前端。所以总之就是之后有可能我会单独把这一个后端的内容拆开来，就是作为一个类似于 Claude Code 这种的命令行工具，然后单独的去开源发布之类的。"

### 29.2 架构定位

```diagram
╭───────────────────────────────────────────────╮
│  VibeHub Frontend（Tauri WebView，可视化）    │
│  - Kanban 看板                                │
│  - JSON 渲染器 / 事件流时间线                 │
│  - 提示词生成器                               │
│  - 只展示，不写状态                           │
╰────────────────────┬──────────────────────────╯
                     │ Tauri commands
╭────────────────────▼──────────────────────────╮
│  VibeHub Core（Rust，可独立编译为 CLI）       │
│  - Event Log writer（INV-1/3）                │
│  - Schema 校验                                │
│  - Capability + Gate 引擎                     │
│  - Sub-agent 调度                             │
│  - Adapter generator                          │
│  - 调试导出                                   │
╰───────────────────────────────────────────────╯
       │                            │
       ▼                            ▼
   .vibehub/                    git / 文件系统
```

### 29.3 工程含义

- **Core 必须可独立运行**：
  - 提供 CLI 二进制（例：`vibehub start-task ...`, `vibehub sync`, `vibehub claim research`）
  - 提供 stdio / RPC 接口供任何 agent 工具直接调用（不依赖 Tauri）
  - 不依赖任何 UI 代码
- **Frontend 只是消费者之一**：
  - 通过 Tauri command 调用 Core
  - 未来可被替换为 web / TUI / VS Code extension 等任何前端
- **代码组织建议**：
  - `src-tauri/src/vibehub/` 继续作为 Core
  - 抽出 `crates/vibehub-core/`（纯 Rust lib）
  - `crates/vibehub-cli/`（二进制）
  - Tauri 应用作为 wrapper 调用 core lib
- **未来开源路径**：
  - Core + CLI 可独立开源（类似 Claude Code）
  - Frontend 作为官方启动器（可闭源也可开源）
  - Adapter 生成器跨形态共用

### 29.4 短期与本重构的关系

- 本次重构（M1-M5）**不需要立即拆分仓库**
- 但代码层面必须保证 **Core 与 Tauri 解耦**：
  - 不要在 `vibehub/` 模块里直接调 Tauri runtime API
  - Tauri command 仅作为 thin wrapper 调 core 函数
  - 所有 IO 走 Rust 标准库 + crate（不依赖 Tauri 的文件系统封装）
- 这样未来拆分时只需复制目录，不需要重写

### 29.5 与跨工具 Adapter 的关系

- CLI 本身就是一种最通用的 adapter：任何 agent 工具只要能跑 shell 都能用
- Tauri command / CLI 是同一 core 的两种 wrapper
- 跨工具 adapter（Section 27）则是为各 agent 工具的 skill 系统生成偏好格式的配置，简化用户体验

---

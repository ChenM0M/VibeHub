下面是优化后的 **VibeHub v2.0 r10 完整需求规格说明书**。

这版在 r9 基础上主要补齐了 6 个协议缺口：

1. **`current` 机制明确为跨平台 YAML 指针文件**，不是 symlink，也不是目录复制。
2. **Context Pack 生成者明确为 VibeHub 后端**，P0 就要实现从 `context/*.yaml` 到 `context-packs/*.md + manifest.yaml` 的生成逻辑。
3. **`handoff.md` 正式定义为 Session 隔离的核心交接文件**，并给出结构模板。
4. **补充 VibeHub v1 → v2 过渡关系**：项目管理、标签启动、AI 网关都保留，并逐步成为驾驶舱的入口、启动器和模型路由底座。
5. **补齐 `align_lite` / `review_lite`**，让 YOLO Drive 不再引用未定义阶段。
6. **明确 Research 作用域**：P0/P1 中 `research/current/` 表示当前 active task 的研究资料；任务切换或 finish 时归档，长期知识再 promoted 到 stable/library。

------

# VibeHub v2.0 r10 — Observable YOLO AI Coding 驾驶舱需求规格说明书

版本：v2.0 r10
定位：面向 AI Coding 的可观测 YOLO 驾驶舱、4+1 流程中台、上下文管理层、Research Pack 层、Git 恢复层、Agent 状态校验层
优先级策略：**P0/P1 优先跑通 4+1 流程；P3/P4 再做深度集成、多 Agent、跨框架兼容和高级自动化**

------

## 一、r10 核心定位

### 1.1 一句话定位

**VibeHub 是一个面向 AI Coding YOLO 工作流的可视化驾驶舱：允许 Agent 在约束边界内尽可能自主推进，同时把需求、计划、上下文、Research、历史、Git diff、风险、回滚点和人工接管点全部显式化。**

更短的产品表达：

> **VibeHub：让 AI 放手做，但让过程始终可见可控。**

英文表达：

> **VibeHub: Observable YOLO for AI Coding.**

------

### 1.2 产品哲学

VibeHub 不以“把 Agent 关进严格流程”为第一目标，而以“让 Agent 在可观测边界内自由行动”为第一目标。

传统 YOLO 模式的问题是：

```text
用户一句话交给 AI
→ AI 自主读代码、改代码、跑命令
→ 结果可能很快
→ 但过程黑箱、上下文不明、历史丢失、难恢复
```

VibeHub 要做的是：

```text
用户一句话交给 AI
→ Agent 在边界内自主推进
→ VibeHub 尽力记录它读了什么、改了什么、为什么改
→ VibeHub 维护上下文包、阶段摘要、Git checkpoint、Research 证据
→ 用户随时可以暂停、接管、回滚、重建上下文、进入严格 Review
```

一句话：

> **VibeHub 不取消 YOLO，而是给 YOLO 模式装上仪表盘、刹车、后视镜和行车记录仪。**

------

### 1.3 与 Trellis 的定位差异

Trellis 是一个成熟参考。它在 README_CN 中将自己定位为“给 AI 立规矩的开源框架”，支持 Claude Code、Cursor、OpenCode、Codex、Gemini CLI、Antigravity、GitHub Copilot 等多个 AI Coding 平台，并强调自动注入 Spec、任务驱动工作流、项目记忆、团队共享标准和多平台复用。([GitHub](https://github.com/mindfold-ai/Trellis/blob/main/README_CN.md?utm_source=chatgpt.com))

Trellis 的架构文档明确将 `workflow.md` 作为项目 workflow 的单一修改点，平台、skill、sub-agent、command 会在下一次 session 中重新读取它；文档还描述了默认 task status 与 workflow phase 的对应关系。([Trellis 文档](https://docs.trytrellis.app/advanced/architecture?utm_source=chatgpt.com))

VibeHub 借鉴 Trellis 的成熟经验，但不复制它的定位。

| 维度     | Trellis 倾向                                         | VibeHub r10 倾向                                             |
| -------- | ---------------------------------------------------- | ------------------------------------------------------------ |
| 核心哲学 | 给 AI 立规矩，让 AI 按 workflow 执行                 | 给 AI 放权，但让过程可见、可控、可恢复                       |
| 产品形态 | CLI / Skills / Hooks / `.trellis` 文件协议           | Desktop GUI 驾驶舱 + 本地文件状态层                          |
| 用户体验 | start / continue / finish-work + auto-trigger skills | 一键 YOLO Drive + 4+1 可视化阶段 + 状态校验                  |
| 控制方式 | workflow-first / spec-first / task-first             | observable autonomy：先允许推进，再持续记录、校验、恢复      |
| 上下文   | JSONL 指定注入文件                                   | Context Pack + Manifest + 质量检查 + stale 检测              |
| Research | research sub-agent + research 目录注入               | 可审计 Research Pack：source-log、source-card、findings、design implications |
| Review   | check sub-agent + 自修复循环                         | Review evidence + Git diff + Research 对照 + 人工接管点      |
| 目标用户 | 愿意接受流程框架的 AI coding 用户                    | 想要“更 vibe”、低门槛、可控 YOLO 的个人开发者和小团队        |

VibeHub 的差异化不是“我也有 workflow”，而是：

> **Trellis 代表 workflow discipline；VibeHub 代表 observable autonomy。**

------

## 二、r10 设计原则

### 2.1 Loose by default, strict when needed

VibeHub 不应该默认把每个任务都变成重流程。

r10 采用三种驾驶模式：

| 模式           | 适用场景                                  | 流程                                           |
| -------------- | ----------------------------------------- | ---------------------------------------------- |
| YOLO Drive     | 小改、探索、原型、用户只想快速推进        | `align_lite → implement → review_lite`         |
| Guided Drive   | 一般功能开发、Bug 修复、多文件修改        | `align → plan → implement → review`            |
| Evidence Drive | 插件、SDK、安全、迁移、重构、外部依赖不明 | `align → research → plan → implement → review` |

这三个模式共享同一个底层状态层，不是三套系统。

------

### 2.2 约束下放权

VibeHub 对 Agent 的态度不是“你只能按我写的步骤做”，而是：

```text
你可以自主探索
你可以提出计划
你可以实现
你可以自检
你可以请求更多上下文
你可以建议修改任务
但你不能让过程变成黑箱
不能绕过 evidence
不能悄悄推进真实状态
不能让用户失去恢复能力
```

Agent 是执行者、研究者、实现者、建议者。
VibeHub 是状态中台、证据中台、上下文中台和恢复中台。

------

### 2.3 VibeHub-owned state transition

VibeHub v2 的核心原则之一是：**Agent 不再是最终状态写入者，Agent 生成报告和建议，VibeHub 负责校验、写入 `state.yaml`、追加事件、生成 Git snapshot。** 这个原则来自 r6 中对 Agent 回写失败、状态漂移和 VibeHub-owned transition 的设计。

r10 继续保留这个原则：

```text
Agent running
→ Agent reported
→ VibeHub validating
→ completed / needs_action / failed
```

Agent 可以说：

```text
我完成了 Implement。
我认为测试通过。
我建议进入 Review。
```

但真实状态由 VibeHub 推进。

------

## 三、最重要的技术边界：观测不是魔法

### 3.1 P0/P1 的观测能力边界

VibeHub 反复强调“记录 Agent 做了什么”，但必须明确：**P0/P1 不具备完整运行时拦截能力**。

P0/P1 的可观测性主要来自两类信息：

```text
A. Agent 自主汇报
B. Git diff / 文件系统快照 / VibeHub 生成物推断
```

这意味着早期观测是 best-effort，不是 guaranteed-complete。

| 信息类型          | P0/P1 怎么获得                                 | 可信度 | 局限                              |
| ----------------- | ---------------------------------------------- | ------ | --------------------------------- |
| 改了哪些文件      | Git diff / 文件快照                            | 高     | 能看到结果，看不到原因            |
| 改了什么内容      | Git diff                                       | 高     | 只能看到最终 diff                 |
| 跑了哪些命令      | Agent output 自主汇报                          | 中低   | Agent 可能漏报                    |
| 读了哪些文件      | Agent output 自主汇报 / context pack 推断      | 中低   | 不能保证完整                      |
| 为什么这样改      | Agent summary / transcript                     | 中低   | 属于解释，不是硬事实              |
| 是否遵守计划      | diff + output + review 推断                    | 中     | 需要人工或 AI 审查                |
| 是否遵守 Research | diff + research-pack + review 推断             | 中     | 需要 Review 对照                  |
| 是否发生循环      | session 输出、Review 失败次数、diff churn 推断 | 中低   | 无 runtime adapter 时只能近似判断 |

因此 r10 显式声明：

> **P0/P1 的“观测”基于 Agent 自主汇报 + Git diff 推断，不追求运行时拦截。这意味着观测是“尽力而为”的，不是“保证完整”的。P3 的深度集成，例如 OpenCode C2、Claude Runner、hooks / extension 适配，才能实现更精确的运行时观测。**

------

### 3.2 Observation Tiers

VibeHub 对所有观察结果标注来源等级。

```yaml
observation_tiers:
  O0_agent_reported:
    description: "Agent 自己说它做了什么。"
    reliability: low_to_medium

  O1_git_observed:
    description: "VibeHub 通过 Git diff / status / log 观察到。"
    reliability: high_for_code_changes

  O2_filesystem_observed:
    description: "VibeHub 通过文件 watcher / mtime / snapshot 观察到。"
    reliability: medium

  O3_vibehub_generated:
    description: "VibeHub 自己生成或写入的文件。"
    reliability: high

  O4_runtime_observed:
    description: "通过 OpenCode C2 / Claude Runner / hooks 等运行时适配器观察到。"
    reliability: high
    phase: P3_or_later
```

------

### 3.3 Evidence Grade

所有 evidence 都应携带 grade：

```yaml
evidence_grade:
  hard_observed:
    description: "由 Git、文件系统或 VibeHub 自己生成的硬证据。"

  adapter_observed:
    description: "由运行时适配器捕获。P3+。"

  agent_reported:
    description: "Agent 自己汇报。"

  inferred:
    description: "由 VibeHub 或 Review 根据多个信号推断。"

  user_confirmed:
    description: "用户明确确认。"
```

示例：

```yaml
actions:
  - type: file_changed
    path: "src/settings/AdapterPanel.tsx"
    evidence: git_diff
    grade: hard_observed

  - type: file_read
    path: "src/adapters/sync.ts"
    evidence: agent_output
    grade: agent_reported

  - type: rationale
    summary: "为了复用现有 sync 状态模型"
    evidence: agent_output
    grade: agent_reported

  - type: likely_related_to_task
    task_id: T-20260429-001
    evidence: diff_path_match
    grade: inferred
```

------

### 3.4 Runtime observation 进入 P3

P0/P1 不为每个 AI CLI 写运行时拦截器。

P3 以后再做：

```text
OpenCode C2 event stream
Claude Code Runner stream-json
Antigravity / .agents workflow output convention
Cursor / Claude hooks
MCP or tool-call proxy
```

Trellis 的架构文档也体现了类似现实情况：部分平台可以通过 hook 或 extension 注入上下文，但其他平台仍然依赖主会话读取上下文文件并传递。([Trellis 文档](https://docs.trytrellis.app/advanced/architecture?utm_source=chatgpt.com)) VibeHub r10 采用同样现实判断：**P0/P1 保持工具无关，P3 再做平台特化观测。**

------

## 四、与 VibeHub v1 的关系

VibeHub v2 不是推翻 v1，而是在 v1 的项目管理能力上增加 4+1 驾驶舱。

### 4.1 v1 功能保留

```yaml
v1_features:
  project_manager:
    status: keep
    v2_role: "作为 v2 驾驶舱的项目选择入口。用户从项目列表选择 repo 后进入 VibeHub Cockpit。"

  tag_launcher:
    status: keep
    v2_role: "作为 v2 的 Agent CLI / IDE 快速启动入口。可按项目标签启动 Claude Code、OpenCode、Cursor、Antigravity 等。"

  ai_gateway:
    status: keep
    v2_role: "P0/P1 保持原能力；P3 可作为 Agent 模型路由、成本统计和统一模型调用底座。"

  git_info:
    status: enhance
    v2_role: "升级为 Git evidence、baseline、dirty worktree、recover report 的基础能力。"

  project_scan:
    status: enhance
    v2_role: "升级为 Context Pack 候选文件发现、目录地图和相关文件推荐能力。"
```

------

### 4.2 v1 → v2 用户路径

```text
打开 VibeHub
→ 看到 v1 项目列表
→ 选择一个项目
→ 进入 v2 Cockpit
→ 如果项目没有 .vibehub，提示 init
→ 如果项目已有 .vibehub，读取 state / current task / Git 状态
→ 显示 Start / Continue / Review / Recover
```

------

### 4.3 v1 标签启动与 v2 Agent 启动

v1 的标签启动可在 v2 中增强为：

```text
项目标签：frontend / rust / ai-agent / docs
启动目标：Claude Code / OpenCode / Cursor / Antigravity
启动上下文：.vibehub/agent-view/current.md
```

P2 后可以支持：

```text
从 GUI 点击 “Open in Claude Code”
→ 打开终端 / IDE
→ 显示推荐命令
→ Agent 读取 agent-view/current.md
```

P3 后可以支持更深集成：

```text
VibeHub 直接创建 OpenCode session
VibeHub 启动 Claude Runner
VibeHub 通过 AI Gateway 路由模型请求
```

------

## 五、优先级边界

### 5.1 P0：4+1 File Protocol Kernel

目标：不用深度集成，也能跑通 4+1。

必须有：

- `.vibehub init`
- `state.yaml`
- `workflow.yaml`
- `rules/*.yaml`
- task / run / session 实体
- `current` YAML 指针
- `agent-view`
- Context Pack generator
- Context Manifest
- Research Pack 可选
- `handoff.md`
- Review evidence
- Git baseline
- Agent reported output
- VibeHub validation

成功标准：

```text
一个功能能从 Align → Optional Research → Plan → Implement → Review 跑通。
小任务可以跳过 Research。
复杂任务可以生成 Research Pack。
Agent 只读 agent-view 和 context pack，不需要理解完整 .vibehub 结构。
```

------

### 5.2 P1：GUI Observable Cockpit

目标：用户不用翻文件。

必须有：

- 4+1 流程图；
- 当前任务；
- 当前 phase；
- Continue / Pause / Review / Recover；
- Context Pack Viewer；
- Research Viewer；
- Git Diff Viewer；
- Evidence Viewer；
- Handoff Viewer；
- Markdown / Mermaid Preview；
- 状态异常提示；
- Evidence Grade 展示。

成功标准：

```text
用户打开项目即可知道：
当前做到哪；
Agent 做了什么；
哪些是硬证据；
哪些只是 Agent 自述；
上下文是否完整；
Git 是否漂移；
下一步该点什么。
```

------

### 5.3 P2：Low-friction Agent Integration

目标：让常用工具能顺滑使用 VibeHub 协议。

必须有：

- `AGENTS.md`；
- `CLAUDE.md`；
- OpenCode command markdown；
- Antigravity workflow/rules markdown；
- Adapter Sync preview；
- Adapter conflict strategy；
- 简化 CLI；
- Journal；
- Knowledge promotion 手动版。

Trellis 的日常使用文档强调 `continue` 能让用户不用记住每个 phase 对应的 slash command，流程会从普通对话里自然推进。([Trellis 文档](https://docs.trytrellis.app/start/everyday-use?utm_source=chatgpt.com)) VibeHub P2 也应把命令压缩到少数入口，不让用户背复杂阶段命令。

------

### 5.4 P3：Deep Integration & Compatibility

目标：增强自动化，但不影响 P0/P1 主体验。

包含：

- OpenCode C2；
- Claude Runner；
- Trellis `.trellis` import；
- JSONL ↔ Context Manifest 转换；
- auto-trigger skills；
- bug loop analysis；
- Research refresh；
- adapter conflict resolver；
- runtime observation；
- context quality scoring；
- v1 AI Gateway 作为 Agent 模型路由底座。

------

### 5.5 P4：Advanced Autonomy

包含：

- 多 Agent 并行；
- Git worktree 自动隔离；
- Agent work planner；
- CI gate；
- runtime sandbox；
- enterprise policy；
- team collaboration；
- context graph；
- cross-repo librarian；
- marketplace。

------

## 六、实体关系模型

### 6.1 三个核心实体

VibeHub 中有三个组织单元：

| 实体    | 含义                                                  |
| ------- | ----------------------------------------------------- |
| Task    | 一个用户需求或功能目标                                |
| Run     | 一次围绕 Task 的执行尝试，可以包含完整或部分 4+1 流程 |
| Session | 一次 Agent 对话、一次外部工具会话、或一次阶段执行片段 |

------

### 6.2 实体关系

```text
Project 1:N Task

Task 1:N Run
  一个任务可能需要多轮尝试。
  Review 失败后，如果只是小修，可留在同一个 Run；
  如果需要重做 Plan / Research / 大范围返工，应创建新 Run。

Run 1:N Session
  一个 Run 的每个 phase 可以有一个或多个 Session。
  一个 phase 被打断、恢复、重跑，都可以产生多个 Session。

Run 1:N Context Pack
  每个 phase 至少有一个 Context Pack。
  如果 context rebuilt，则产生新版本。

Task 1:1 Active Run
  同一时间一个 Task 只有一个 active run。

Project 1:1 Active Task
  P0/P1 阶段一个项目默认只有一个 active task。
  多 active task / parallel worktree 放到 P4。
```

------

### 6.3 示例

```text
Project: VibeHub
└── Task T-001: 实现 Adapter Sync 面板
    ├── Run R-001: 第一次实现
    │   ├── Session S-001: Align
    │   ├── Session S-002: Plan
    │   ├── Session S-003: Implement
    │   └── Session S-004: Review，发现缺测试
    │
    └── Run R-002: 补测试和修复
        ├── Session S-005: Replan Lite
        ├── Session S-006: Implement Fix
        └── Session S-007: Review Pass
```

------

## 七、`current` 指针机制

### 7.1 设计原则

VibeHub 是跨平台桌面应用，不能依赖 symlink 作为核心协议。

`tasks/current` 和 `runs/current` **不是目录、不是符号链接、不是复制别名**，而是简单 YAML 指针文件。

这样做的原因：

- Windows 下 symlink 存在权限和开发者模式问题；
- 复制目录会带来同步不一致；
- YAML 指针简单、可读、可跨平台；
- Agent 不需要理解 `current` 机制，只需要读 `agent-view/current.md` 中的显式路径。

------

### 7.2 `.vibehub/tasks/current`

```yaml
schema_version: 1
kind: current_task_pointer
task_id: T-20260429-001
path: ".vibehub/tasks/T-20260429-001"
updated_at: 2026-04-29T12:00:00-07:00
updated_by: vibehub
```

------

### 7.3 `.vibehub/tasks/{task_id}/runs/current`

```yaml
schema_version: 1
kind: current_run_pointer
task_id: T-20260429-001
run_id: R-001
path: ".vibehub/tasks/T-20260429-001/runs/R-001"
updated_at: 2026-04-29T12:00:00-07:00
updated_by: vibehub
```

------

### 7.4 解析规则

VibeHub 后端负责解析：

```text
读取 .vibehub/tasks/current
→ 得到 task_id 和 task path
→ 读取 task/runs/current
→ 得到 run_id 和 run path
→ 生成 agent-view/current.md 中的显式路径
```

Agent 不应该被要求自己解析指针文件。

------

### 7.5 指针损坏处理

如果指针文件损坏：

```text
VibeHub 检测 task_id 缺失 / path 不存在
→ 进入 Recover
→ 从 state.yaml、最近 run、Git history、mtime 推断候选
→ 用户选择 active task/run
→ VibeHub 重写 current 指针
```

------

## 八、4+1 流程定义

### 8.1 总流程

```text
Align
  ↓
Optional Research
  ↓
Plan
  ↓
Implement
  ↓
Review
```

Research 是 “+1”：

- 小任务默认跳过；
- 中等任务按需建议；
- 外部框架、SDK、插件、安全、性能、迁移、重构任务强烈建议；
- 用户可跳过，但必须记录风险。

------

### 8.2 Phase 0：Start / Preflight

Start 不是正式阶段，而是会话启动检查。

输入：

- 用户当前意图；
- Git 状态；
- 当前 task；
- 当前 run；
- 最近 session；
- 项目规则；
- 最近 journal；
- 是否存在未提交改动。

输出：

- 推荐驾驶模式：YOLO / Guided / Evidence；
- 是否需要创建 task；
- 是否需要 Research；
- 是否需要 Recover；
- 当前上下文摘要。

------

### 8.3 Phase 1：Align

目的：明确目标，不急着写代码。

Align 需要回答：

- 用户真正要什么？
- 这是不是小改？
- 是否需要任务化？
- 是否需要 Research？
- 有哪些明确不做？
- 成功标准是什么？
- 是否允许 YOLO 自主推进？

输出：

```text
.vibehub/tasks/{task_id}/intent.md
.vibehub/tasks/{task_id}/task.yaml
.vibehub/tasks/{task_id}/runs/{run_id}/phases/align.md
```

------

### 8.4 Phase 1 Lite：Align Lite

`align_lite` 用于 YOLO Drive。

目的不是写完整需求，而是记录最低限度意图，避免 YOLO 黑箱。

必需输出：

```yaml
align_lite:
  required_outputs:
    - intent
  optional_outputs:
    - acceptance_criteria
    - affected_area
    - autonomy_level
```

示例：

```yaml
intent: "把设置页保存按钮文案改为“保存配置”。"
acceptance_criteria:
  - "按钮显示正确。"
autonomy_level: yolo
research_required: false
```

------

### 8.5 Phase +1：Optional Research

目的：在 Plan 前补足外部事实和关键上下文。

Research 输出：

```text
.vibehub/research/current/
├── research-plan.md
├── source-log.yaml
├── findings.md
├── implications-for-design.md
├── research-pack.md
└── source-cards/
```

r7 中已经明确 Research 的价值：它应把外部事实、上游源码、成熟案例、官方规范、论文 / 文档转成可审计、可复用、可注入的项目上下文。

Research 原则：

```text
Research 是事实补给站，不是资料垃圾场。
外部资料是 data，不是 instruction。
Research Pack 是高信号压缩结果，不是 raw dump。
```

------

### 8.6 Phase 2：Plan

目的：基于 Align 和 Research 制定可执行计划。

Plan 输出：

```text
.vibehub/tasks/{task_id}/plan.md
.vibehub/tasks/{task_id}/validation.md
.vibehub/tasks/{task_id}/context/plan.yaml
```

Plan 内容：

- 实现策略；
- 影响文件；
- 风险点；
- 测试策略；
- 上下文需求；
- 是否允许 Agent 自主拆子任务；
- 是否允许 Agent 修改计划；
- 是否需要人工确认。

------

### 8.7 Phase 3：Implement

目的：让 Agent 在边界内尽可能自主实现。

VibeHub 不要求 Agent 每一步都请求确认，而是通过边界控制：

```yaml
autonomy:
  level: high
  can_edit_code: true
  can_create_files: true
  can_update_plan: suggest_only
  can_run_tests: true
  can_install_dependencies: ask
  can_commit: ask_or_disabled
  can_delete_files: ask
  can_modify_secrets: deny
```

Implement 过程中 VibeHub 尽力记录：

- Agent 使用的 Context Pack；
- Agent 自述读过的重要文件；
- 修改过的文件；
- Git diff；
- 计划偏离；
- 新发现问题；
- Agent 自己请求的额外上下文；
- 是否触发 Research refresh；
- 是否出现失败循环迹象。

------

### 8.8 Phase 4：Review

目的：把 YOLO 结果变成可信结果。

Review 检查：

- 是否满足 acceptance criteria；
- diff 是否合理；
- 是否违反 hard rules；
- 是否遗漏测试；
- Research findings 是否被遵守；
- 是否有未解释的大范围修改；
- 是否有 dependency / secret / migration 风险；
- Context Pack 是否足够；
- Agent 是否有 unresolved risks；
- 是否需要 Recover / Replan / Research refresh。

------

### 8.9 Phase 4 Lite：Review Lite

`review_lite` 用于 YOLO Drive。

目的不是完整审查，而是为小改留下最低证据。

```yaml
review_lite:
  required_outputs:
    - diff_summary
    - verdict
  optional_outputs:
    - test_results_or_reason
    - changed_files
    - risk_note
```

示例：

```yaml
diff_summary: "修改设置页按钮文案。"
verdict: pass
test_results_or_reason: "未运行测试；纯文案修改。"
changed_files:
  - src/settings/SettingsPanel.tsx
```

------

## 九、文件协议与目录结构

### 9.1 r10 精简目录结构

```text
.vibehub/
├── project.yaml
├── state.yaml
├── workflow.yaml
├── housekeeping.yaml
│
├── agent-view/
│   ├── current.md
│   ├── current-context.md
│   └── handoff.md
│
├── rules/
│   ├── phase-rules.yaml
│   ├── research-triggers.yaml
│   ├── autonomy.yaml
│   ├── review.yaml
│   ├── loop-detection.yaml
│   ├── hard-rules.md
│   └── preferences.yaml
│
├── tasks/
│   ├── current
│   └── T-20260429-001/
│       ├── task.yaml
│       ├── intent.md
│       ├── plan.md
│       ├── validation.md
│       ├── context/
│       │   ├── plan.yaml
│       │   ├── implement.yaml
│       │   └── review.yaml
│       └── runs/
│           ├── current
│           └── R-001/
│               ├── run.yaml
│               ├── events.jsonl
│               ├── phases/
│               │   ├── align.md
│               │   ├── research.md
│               │   ├── plan.md
│               │   ├── implement.md
│               │   └── review.md
│               ├── context-packs/
│               │   ├── implement.md
│               │   └── implement.manifest.yaml
│               ├── sessions/
│               │   └── S-001/
│               │       ├── meta.yaml
│               │       ├── input.md
│               │       ├── output.md
│               │       └── transcript.md
│               └── evidence/
│                   ├── changed-files.txt
│                   ├── diff.patch
│                   ├── test-results.md
│                   └── unresolved-risks.md
│
├── research/
│   ├── current/
│   │   ├── meta.yaml
│   │   ├── research-plan.md
│   │   ├── source-log.yaml
│   │   ├── findings.md
│   │   ├── implications-for-design.md
│   │   ├── research-pack.md
│   │   └── source-cards/
│   └── archive/
│
├── journal/
│   ├── index.md
│   └── 2026-04.md
│
├── adapters/
│   ├── sync-state.yaml
│   ├── templates/
│   └── generated/
│
└── archive/
```

------

### 9.2 Agent-visible state

关键原则：

> **Agent 默认不应该读取完整 `.vibehub/`。Agent 看到的应该是精简视图，完整数据只供 GUI、VibeHub 后端和人类审计使用。**

P0/P1 中，Agent 默认只读：

```text
.vibehub/agent-view/current.md
.vibehub/agent-view/current-context.md
.vibehub/agent-view/handoff.md
.vibehub/state.yaml
.vibehub/workflow.yaml
.vibehub/rules/hard-rules.md
.vibehub/research/current/research-pack.md   # 仅当需要 Research
```

不应提示 Agent 直接“读取整个 `.vibehub/`”。

------

## 十、Agent View 与 Handoff

### 10.1 `agent-view/current.md`

这是 Agent 的主入口。

```md
# VibeHub Current State

## Task
T-20260429-001 — 实现 Adapter Sync 状态面板

## Mode
Guided Drive

## Current Phase
Implement

## Observability Note
P0/P1 observability is best-effort.
Report files read, commands run, decisions made, and unresolved risks.

## What you should read
1. .vibehub/agent-view/current-context.md
2. .vibehub/agent-view/handoff.md
3. .vibehub/tasks/T-20260429-001/plan.md
4. .vibehub/tasks/T-20260429-001/runs/R-001/context-packs/implement.md
5. .vibehub/rules/hard-rules.md

## What you should write
1. .vibehub/tasks/T-20260429-001/runs/R-001/sessions/S-003/output.md
2. .vibehub/tasks/T-20260429-001/runs/R-001/phases/implement.md

## Stop condition
Do not mark state.yaml completed.
Return to VibeHub for validation.
```

------

### 10.2 `agent-view/current-context.md`

这是当前阶段的简化上下文入口。

```md
# Current Context

## Context Pack
.vibehub/tasks/T-20260429-001/runs/R-001/context-packs/implement.md

## Manifest
.vibehub/tasks/T-20260429-001/runs/R-001/context-packs/implement.manifest.yaml

## Important Project Files
- src/adapters/sync.ts
- src/components/settings/AdapterSyncPanel.tsx

## Research Pack
Not required for this task.

## Known Missing Context
- No explicit test command found.
```

------

### 10.3 `agent-view/handoff.md`

`handoff.md` 是 Session 隔离和文件传递架构的核心。

它不是完整聊天记录，而是给下一个 Session 的高信号交接摘要。

#### 生成责任

P0 策略：

```text
Agent 在 output.md 中写 handoff section
VibeHub 读取 output.md
VibeHub 生成 / 更新 agent-view/handoff.md
用户可在 GUI 中编辑确认
```

P1 后 GUI 支持结构化编辑。
P3 runtime adapter 可自动提取更完整的 session trace。

------

### 10.4 `handoff.md` 模板

```md
# Handoff from Session S-003

Task: T-20260429-001  
Run: R-001  
Phase: Implement  
Generated by: VibeHub  
Source: Agent output + Git diff  
Evidence grade: mixed

## Completed
- AdapterSyncPanel.tsx 基本框架已完成。
- sync-state.yaml 读取逻辑已实现。

## Not Yet Done
- conflict detection 逻辑未实现。
- 测试未编写。

## Key Decisions Made
- 使用 hash 比较而非 mtime 判断文件变更。
- managed region 使用 HTML comment 标记。

## Files Changed
- src/components/settings/AdapterSyncPanel.tsx
- src/adapters/sync.ts

## Files Reportedly Read
- src/adapters/types.ts
- src/settings/SettingsPage.tsx

Evidence grade: agent_reported

## Context Still Needed
- 需要确认 adapter sync 的测试命令。

## Warnings
- src/adapters/sync.ts 被修改了 4 次，可能需要重构。
- 没有运行完整测试。

## Next Session Should
1. 实现 conflict detection。
2. 编写基本测试。
3. 完成后写 output.md 并回到 VibeHub 校验。
```

------

### 10.5 Handoff 质量检查

VibeHub 校验 handoff 是否包含：

```yaml
handoff_quality:
  required:
    - completed
    - not_yet_done
    - key_decisions
    - files_changed
    - next_session_should
  optional:
    - files_reportedly_read
    - context_still_needed
    - warnings
```

如果缺失：

```text
VibeHub 标记 handoff incomplete
→ GUI 提醒用户补充
→ 或要求 Agent 补写 output.md
```

------

## 十一、Context Management 与 Context Pack Generator

### 11.1 核心目标

VibeHub 的上下文管理不只是“注入哪些文件”。

它必须回答：

```text
为什么注入这些？
为什么排除那些？
上下文是否过期？
是否包含 Research Pack？
是否包含当前 diff？
是否包含 Review 所需证据？
Agent 是否请求过更多上下文？
失败是否由上下文不足导致？
```

Trellis 的 JSONL 上下文格式很值得借鉴：每行记录一个文件或目录，并带有 reason；Trellis 还通过不同 context 索引服务 research / implement / check 等不同执行场景。([Trellis 文档](https://docs.trytrellis.app/advanced/architecture?utm_source=chatgpt.com)) VibeHub 采用同样的低复杂度思想，但用 YAML + Manifest 承载更强的可解释性。

------

### 11.2 Context 声明文件：`context/implement.yaml`

```yaml
phase: implement
task_id: T-20260429-001

entries:
  - path: ".vibehub/workflow.yaml"
    type: file
    reason: "当前 4+1 workflow 契约"
    required: true

  - path: ".vibehub/tasks/T-20260429-001/plan.md"
    type: file
    reason: "实现计划"
    required: true

  - path: "src/adapters/sync.ts"
    type: file
    reason: "Adapter Sync 核心逻辑"
    required: true

  - path: "src/components/settings/AdapterSyncPanel.tsx"
    type: file
    reason: "目标 UI 面板"
    required: false

  - path: ".vibehub/research/current/research-pack.md"
    type: file
    reason: "外部框架约束"
    required: false
    include_when: "research.required == true"
```

------

### 11.3 Context Pack 生成者

P0 明确策略：

> **Context Pack 由 VibeHub 后端生成，不由 Agent 自己组装，也不要求用户手工拼接。**

生成流程：

```text
VibeHub 读取 context/{phase}.yaml
→ 解析 entries
→ 检查路径是否存在
→ 检查 path 是否越界
→ 按 token / 字符预算读取文件
→ 拼接为 context-packs/{phase}.md
→ 生成 context-packs/{phase}.manifest.yaml
→ 更新 agent-view/current-context.md
→ Agent 只读 context-packs/{phase}.md
```

------

### 11.4 Context Pack 生成规则

```yaml
context_pack_generator:
  owner: vibehub_backend
  phase: P0

  input:
    - ".vibehub/tasks/{task_id}/context/{phase}.yaml"
    - ".vibehub/state.yaml"
    - ".vibehub/workflow.yaml"
    - ".vibehub/research/current/research-pack.md when required"

  output:
    - ".vibehub/tasks/{task_id}/runs/{run_id}/context-packs/{phase}.md"
    - ".vibehub/tasks/{task_id}/runs/{run_id}/context-packs/{phase}.manifest.yaml"

  safety:
    path_must_be_inside_repo: true
    deny_patterns:
      - ".env*"
      - "secrets/**"
      - "credentials/**"
      - ".git/**"
    max_file_size_kb: 256
    max_total_estimated_tokens: 12000

  missing_file_policy:
    required_missing: "fail_context_build"
    optional_missing: "record_in_manifest"

  stale_policy:
    compare_against_git_head: true
    record_source_commit: true
```

------

### 11.5 Context Pack Markdown 格式

~~~md
# Context Pack: Implement

Task: T-20260429-001  
Run: R-001  
Phase: Implement  
Generated at: 2026-04-29T12:00:00-07:00  
Source commit: def456

## Instructions
Use this context only for the current phase.
Do not mark state.yaml completed.
Report files read, commands run, decisions made, and unresolved risks.

## Task Plan
Source: .vibehub/tasks/T-20260429-001/plan.md  
Reason: 实现计划

...

## Relevant File: src/adapters/sync.ts
Reason: Adapter Sync 核心逻辑

```ts
...
~~~

## Relevant File: src/components/settings/AdapterSyncPanel.tsx

Reason: 目标 UI 面板

```tsx
...
```

## Known Missing Context

- No explicit test command found.

## Stop Condition

Write output.md and return to VibeHub for validation.

```
---

### 11.6 Context Pack Manifest

```yaml
id: implement-context-R-001-v1
phase: implement
task_id: T-20260429-001
run_id: R-001
generated_at: 2026-04-29T12:00:00-07:00
source_commit: def456

budget:
  max_tokens: 12000
  estimated_tokens: 7600

included:
  - path: ".vibehub/tasks/T-20260429-001/plan.md"
    reason: "当前任务实现计划"
    confidence: high

  - path: "src/adapters/sync.ts"
    reason: "主要修改区域"
    confidence: high

  - path: "src/components/settings/AdapterSyncPanel.tsx"
    reason: "相关 UI 组件"
    confidence: medium

excluded:
  - path: ".vibehub/documents/raw/initial-draft.md"
    reason: "过长且低信号"
    confidence: high

missing:
  - path: "tests/adapter-sync.test.ts"
    required: false
    reason: "测试文件尚不存在"

research:
  included: false
  reason: "当前任务不依赖外部框架"

quality:
  has_goal: true
  has_plan: true
  has_relevant_code: true
  has_test_hint: false
  stale_sources: []
  missing:
    - "未找到明确测试命令"

observation:
  generated_by: vibehub
  grade: hard_observed
```

------

## 十二、Research Pack 与作用域

### 12.1 Research 定位

Research 是 4+1 的 “+1”。

它不是默认重流程，而是：

> **复杂任务开始前的事实补给站，Review 阶段的证据参照物。**

------

### 12.2 `research/current/` 的语义

P0/P1 明确：

> **`.vibehub/research/current/` 表示当前 active task 的 Research。**

它不是全项目永久知识库，也不是多任务共享研究区。

当 active task 切换或 finish 时：

```text
research/current/
→ archive 到 research/archive/{task_id}-{run_id}/
→ 如果有长期价值，用户可 promote 到 documents/stable 或 future research/library
```

P3/P4 才引入项目级 Research Library。

------

### 12.3 `research/current/meta.yaml`

```yaml
schema_version: 1
kind: current_research
task_id: T-20260429-001
run_id: R-001
status: completed
scope: active_task
created_at: 2026-04-29T10:30:00-07:00
updated_at: 2026-04-29T11:15:00-07:00

archive_policy:
  on_task_finish: true
  archive_path: ".vibehub/research/archive/T-20260429-001-R-001"

promotion:
  eligible_for_stable_docs: true
  promoted: false
```

------

### 12.4 Research 文件结构

```text
.vibehub/research/current/
├── meta.yaml
├── research-plan.md
├── source-log.yaml
├── source-cards/
│   ├── official-docs.yaml
│   ├── upstream-source.yaml
│   └── mature-example.yaml
├── findings.md
├── implications-for-design.md
└── research-pack.md
```

------

### 12.5 `source-log.yaml`

```yaml
sources:
  - id: official-docs
    type: official_docs
    title: "Official plugin documentation"
    url: "..."
    retrieved_at: 2026-04-29T12:00:00-07:00
    trust: high
    relevance: high
    notes: "定义插件机制和安全提示"

  - id: upstream-source
    type: source_code
    repo: "owner/repo"
    commit: "abc123"
    trust: high
    relevance: high
    notes: "确认真实加载路径"

  - id: mature-example
    type: mature_project
    repo: "owner/example"
    commit: "def456"
    trust: medium
    relevance: medium
    license: "MIT"
    notes: "仅参考模式，不复制代码"
```

------

### 12.6 `research-pack.md`

```md
# Research Pack

Task: T-20260429-001  
Scope: active task  
Evidence: source-log.yaml + source-cards/

## What implementer must know

## Official constraints

## Source code facts

## Reference patterns

## Anti-patterns

## Design implications

## Review checklist

## Open questions
```

------

### 12.7 Research Review

Review 阶段要检查：

```yaml
research_review:
  - implementation_matches_findings
  - no_unverified_external_assumption
  - source_not_stale
  - no_code_copy_without_license_check
  - security_constraints_respected
```

------

## 十三、Workflow 与 Rules 拆分

### 13.1 `workflow.yaml` 只定义流程骨架

```yaml
schema_version: 1
name: vibehub-default-4plus1
philosophy: observable_yolo

modes:
  yolo_drive:
    phases:
      - align_lite
      - implement
      - review_lite

  guided_drive:
    phases:
      - align
      - plan
      - implement
      - review

  evidence_drive:
    phases:
      - align
      - research
      - plan
      - implement
      - review

default_mode: guided_drive

phase_order:
  - align_lite
  - align
  - research
  - plan
  - implement
  - review_lite
  - review
```

------

### 13.2 `rules/phase-rules.yaml`

```yaml
align_lite:
  required_outputs:
    - intent
  optional_outputs:
    - acceptance_criteria
    - affected_area
    - autonomy_level

align:
  required_outputs:
    - intent
    - acceptance_criteria
    - autonomy_level
  optional_outputs:
    - non_goals
    - research_decision
    - risk_level

research:
  optional: true
  required_outputs:
    - source_log
    - findings
    - research_pack

plan:
  required_outputs:
    - implementation_plan
    - validation_plan
    - context_plan

implement:
  required_outputs:
    - changed_files
    - implementation_summary
    - unresolved_questions
  optional_outputs:
    - files_reportedly_read
    - commands_reportedly_run
    - handoff_notes

review_lite:
  required_outputs:
    - diff_summary
    - verdict
  optional_outputs:
    - test_results_or_reason
    - changed_files
    - risk_note

review:
  required_outputs:
    - diff_summary
    - test_results_or_reason
    - verdict
    - risks
    - evidence_grades
```

------

### 13.3 `rules/research-triggers.yaml`

```yaml
strong:
  - external_framework_or_plugin
  - unfamiliar_sdk_or_api
  - migration
  - security_sensitive
  - performance_sensitive
  - architecture_refactor
  - compatibility_work
  - repeated_failure
  - user_requests_research

recommended:
  - ambiguous_requirement
  - unknown_best_practice
  - mature_reference_available
  - model_context_confidence_low

skip_by_default:
  - typo
  - simple_copy_change
  - small_css_change
  - mechanical_rename
  - obvious_local_fix
```

------

## 十四、状态模型

### 14.1 `state.yaml`

```yaml
schema_version: 1

project:
  id: vibehub-project
  name: VibeHub
  root: "."

current:
  mode: guided_drive
  task_id: T-20260429-001
  run_id: R-001
  session_id: S-003
  phase: implement
  phase_status: running

pointers:
  task_pointer: ".vibehub/tasks/current"
  run_pointer: ".vibehub/tasks/T-20260429-001/runs/current"

flow:
  align: completed
  research: skipped
  plan: completed
  implement: running
  review: pending

observability:
  level: best_effort
  runtime_adapter: none
  current_observation_sources:
    - agent_reported
    - git_observed
    - vibehub_generated

autonomy:
  level: high

research:
  required: false
  status: skipped
  scope: active_task
  skipped_reason: "Local UI task; no external framework dependency."

context:
  current_pack: ".vibehub/tasks/T-20260429-001/runs/R-001/context-packs/implement.md"
  current_manifest: ".vibehub/tasks/T-20260429-001/runs/R-001/context-packs/implement.manifest.yaml"
  stale: false
  generated_by: vibehub_backend

agent_report:
  status: running
  validation_status: pending

handoff:
  current: ".vibehub/agent-view/handoff.md"
  status: available

git:
  baseline_commit: abc123
  last_seen_head: def456
  dirty: true
  changed_files_count: 6

loop_detection:
  status: normal
  warnings: []

last_updated: 2026-04-29T12:00:00-07:00
resume_hint: "继续完成 Adapter Sync 状态面板，实现后进入 Review。"
```

------

### 14.2 Phase Status 精简

```yaml
phase_status:
  - pending
  - running
  - agent_reported
  - validating
  - completed
  - skipped
  - blocked
  - needs_action
  - failed
  - abandoned
```

------

## 十五、Autonomy Policy

### 15.1 Autonomy Levels

`high` 和 `yolo` 的差异必须落到具体权限，而不是模糊描述。

```yaml
autonomy_levels:
  low:
    description: "关键修改前都需要确认。"
    can_edit_code: ask
    can_run_tests: ask
    can_install_deps: deny
    can_delete_files: deny
    can_commit: deny

  medium:
    description: "可自主修改任务相关文件，风险操作需确认。"
    can_edit_code: auto_in_task_scope
    can_run_tests: auto
    can_install_deps: ask
    can_delete_files: ask
    can_commit: ask

  high:
    description: "可自主探索、修改、测试和自修复；高风险操作需确认。"
    can_edit_code: auto_in_task_scope
    can_create_files: auto_in_task_scope
    can_run_tests: auto
    can_install_deps: ask
    can_delete_files: ask
    can_commit: ask
    checkpoint_before_large_change: true

  yolo:
    description: "尽可能自主推进；VibeHub 强化 checkpoint、日志和 Review。"
    can_edit_code: auto_in_task_scope
    can_create_files: auto_in_task_scope
    can_run_tests: auto
    can_install_deps: auto_if_trusted_package_manager
    can_delete_files: auto_with_log_in_task_scope
    can_commit: auto_local_checkpoint
    can_push: deny
    can_modify_secrets: deny
    checkpoint_before_large_change: true
    checkpoint_after_each_phase: true
```

------

### 15.2 永远禁止的行为

即使是 YOLO：

```yaml
always_deny:
  - git_push_without_user_confirmation
  - modify_env_or_secrets
  - delete_project_root
  - run_curl_pipe_shell
  - exfiltrate_credentials
  - disable_tests_without_reporting
```

YOLO 是高放权，不是无底线。

------

## 十六、Loop Detection

### 16.1 为什么需要

AI coding 常见失败模式是“打补丁式 debug”：

```text
测试失败
→ 改一处
→ 又失败
→ 再改一处
→ 逐渐偏离原计划
→ 上下文污染
→ 最后很难恢复
```

VibeHub 必须尽早发现循环。

------

### 16.2 `rules/loop-detection.yaml`

```yaml
loop_detection:
  enabled: true

  signals:
    max_test_failures_before_pause: 5
    max_same_file_edit_cycles: 8
    max_context_requests_same_reason: 3
    max_review_failures_same_run: 2
    max_session_duration_minutes: 45
    max_diff_growth_ratio_without_plan_update: 2.0

  action_on_warning:
    - mark_loop_warning
    - show_gui_notification

  action_on_threshold_exceeded:
    - pause_and_notify
    - generate_loop_report
    - recommend_review_or_replan

  p0_limitations:
    - "Without runtime adapter, test failure count depends on agent report or review output."
    - "Same-file edit cycles are approximated through checkpoints and diff snapshots."
```

------

### 16.3 Loop Report

```md
# Loop Detection Report

## Trigger
max_test_failures_before_pause exceeded

## Evidence
- 5 reported test failures
- 3 edits to src/adapters/sync.ts
- diff grew from 120 LOC to 420 LOC
- plan.md has not been updated

## Evidence grade
- test failures: agent_reported
- diff growth: git_observed
- plan unchanged: hard_observed

## Recommendation
Pause current session and enter Review or Replan.
```

------

## 十七、Review 机制

### 17.1 Verdict 收敛

P0/P1 只保留 4 种 verdict：

```yaml
verdict:
  - pass
  - conditional
  - needs_action
  - fail
```

含义：

| Verdict      | 含义                     |
| ------------ | ------------------------ |
| pass         | 通过，可 finish          |
| conditional  | 基本通过，但有低风险待办 |
| needs_action | 需要操作后再判断         |
| fail         | 失败，需要重新计划或重做 |

------

### 17.2 `needs_action` 细分原因

不要把所有状态都做成顶层 verdict，而是放入 actions：

```yaml
verdict: needs_action
actions:
  - type: fix_code
    reason: "测试失败"

  - type: rebuild_context
    reason: "实现阶段缺少关键 API 定义"

  - type: refresh_research
    reason: "Research Pack 基于旧版本上游源码"

  - type: recover_state
    reason: "Git HEAD ahead of VibeHub state"

  - type: user_decision
    reason: "需要决定是否接受破坏性迁移"
```

------

### 17.3 Review Evidence

```yaml
required_review_evidence:
  - changed_files
  - diff_summary
  - acceptance_check
  - tests_run_or_reason_not_run
  - unresolved_risks
  - context_quality_notes
  - observation_limitations
  - verdict
```

------

### 17.4 Review Report

```md
# Review Report

## Verdict
needs_action

## Summary
实现基本完成，但测试命令缺失，且 Adapter generated 文件冲突策略未验证。

## Evidence
- changed files: hard_observed
- diff summary: git_observed
- test status: agent_reported
- rationale: agent_reported

## Acceptance Criteria
- [x] 展示 sync 状态
- [x] 展示 preview / diff / apply 入口
- [ ] generated 文件冲突测试未验证

## Actions Required
1. 添加或确认测试命令
2. 验证 conflict detector
```

------

## 十八、Adapter Sync 子规范

### 18.1 设计原则

Adapter 文件是派生产物，不是唯一事实源。

```text
.vibehub/rules + workflow + agent-view
  ↓
adapter templates
  ↓
AGENTS.md / CLAUDE.md / OpenCode commands / Antigravity workflows
```

P2 Adapter 的重点不是把所有状态塞进目标文件，而是：

```text
静态协议写入 adapter 文件
动态状态指向 agent-view/current.md
```

------

### 18.2 Static vs Dynamic

| 类型            | 内容                                                 | 输出位置                                                    |
| --------------- | ---------------------------------------------------- | ----------------------------------------------------------- |
| Static Protocol | VibeHub 工作方式、文件协议、Stop Condition、权限原则 | `AGENTS.md` / `CLAUDE.md`                                   |
| Dynamic State   | 当前 task、当前 phase、当前 context、当前 run        | `.vibehub/agent-view/current.md`                            |
| Context Pack    | 当前阶段上下文                                       | `.vibehub/tasks/{task_id}/runs/{run_id}/context-packs/*.md` |

`AGENTS.md` 不应该频繁因为 `state.yaml` 变化而重写。
它只需要告诉 Agent：

```text
当前状态请读 .vibehub/agent-view/current.md
当前上下文请读 current-context.md
不要直接推进 state.yaml
完成后写 output.md
```

------

### 18.3 Managed Regions

生成文件使用 managed block：

```md
<!-- VIBEHUB:MANAGED:START -->
Generated protocol content.
Do not edit this block manually.
<!-- VIBEHUB:MANAGED:END -->

<!-- VIBEHUB:MANUAL:START -->
User custom notes here.
<!-- VIBEHUB:MANUAL:END -->
```

------

### 18.4 `adapters/sync-state.yaml`

```yaml
targets:
  agents_md:
    path: "AGENTS.md"
    last_generated_at: 2026-04-29T12:00:00-07:00
    source_hash: "sha256:abc"
    target_hash: "sha256:def"
    manual_region_hash: "sha256:ghi"
    status: in_sync

  claude_md:
    path: "CLAUDE.md"
    last_generated_at: 2026-04-29T12:00:00-07:00
    source_hash: "sha256:abc"
    target_hash: "sha256:xyz"
    status: conflict
    conflict_reason: "managed_region_modified_manually"
```

------

### 18.5 Sync 策略

```yaml
sync_policy:
  trigger:
    manual_by_default: true
    auto_suggest_on:
      - workflow_changed
      - rules_changed
      - adapter_template_changed
    do_not_auto_regenerate_on:
      - state_changed
      - current_task_changed
      - phase_changed

  conflict_resolution:
    options:
      - overwrite_managed_region
      - keep_target
      - show_diff
      - extract_manual_changes
      - mark_external
```

------

## 十九、Git 与 Recover

### 19.1 Git 是底层事实源

`.vibehub` 记录意图和过程。
Git 记录真实代码变化。

VibeHub 必须能处理：

```text
state 说没完成，但 Git 已经改了
Agent 改了代码但没写 output
用户绕过 VibeHub 手动改了
Research Pack 过期
Context Pack 基于旧 commit
```

------

### 19.2 Recover Report

```text
.vibehub/tasks/{task_id}/runs/{run_id}/recover.md
```

内容：

```md
# Recover Report

## Git state
- baseline commit:
- current HEAD:
- dirty files:

## Changes since last checkpoint
...

## Likely related task
...

## State mismatch
...

## Missing artifacts
...

## Evidence grade
- changed files: git_observed
- task mapping: inferred
- missing output: hard_observed

## Recommended action
- continue current implement
- rebuild context
- run review
- refresh research
- create new task
```

------

### 19.3 Recover 策略

P0/P1 只做 safe mode：

```text
检测
报告
建议
等待用户确认
```

不自动猜测并推进状态。

------

## 二十、Housekeeping 与膨胀控制

### 20.1 `housekeeping.yaml`

```yaml
retention:
  max_active_tasks: 1
  max_runs_kept_per_task: 5
  max_sessions_kept_per_run: 10
  max_context_pack_versions_per_phase: 3
  max_research_archives_kept: 10

archive:
  strategy: compress_and_gitignore
  archive_dir: ".vibehub/archive"
  compress_after_days: 30
  archive_completed_runs_after_days: 14
  archive_old_sessions_after_days: 14

agent_visible_state:
  include:
    - ".vibehub/agent-view/"
    - ".vibehub/state.yaml"
    - ".vibehub/workflow.yaml"
    - ".vibehub/rules/hard-rules.md"
    - ".vibehub/research/current/research-pack.md"
  exclude:
    - ".vibehub/archive/"
    - ".vibehub/tasks/*/runs/*/sessions/*/transcript.md"
    - ".vibehub/tasks/*/runs/*/events.jsonl"
    - ".vibehub/adapters/generated/"
```

------

### 20.2 Archive 策略

归档后的文件：

```text
.vibehub/archive/
├── tasks/
├── runs/
├── sessions/
└── research/
```

可选压缩：

```text
.vibehub/archive/2026-04/T-20260429-001.tar.zst
```

`.gitignore` 推荐：

```gitignore
.vibehub/archive/
.vibehub/**/*.tmp
.vibehub/**/raw-transcripts/
```

------

### 20.3 Agent 防淹没原则

Context Pack 是 Agent 的主要入口，不是 `.vibehub` 全量目录。

```text
完整历史：GUI / 人类 / VibeHub backend 使用
精简状态：Agent 使用
当前上下文：Context Pack 使用
```

------

## 二十一、Journal 与知识沉淀

### 21.1 Journal 的作用

VibeHub 不保存完整垃圾聊天记录，但保存：

- 本次做了什么；
- 为什么这样做；
- 关键决策；
- 失败原因；
- Review 结论；
- 有价值的经验；
- 是否需要沉淀到 rules / docs。

Trellis 的架构文档中提到 session 记录、workspace journal 和 task archive 等机制，这种 finish 后沉淀经验的方向值得 VibeHub 借鉴。([Trellis 文档](https://docs.trytrellis.app/advanced/architecture?utm_source=chatgpt.com))

------

### 21.2 Journal 示例

```md
## 2026-04-29 — Adapter Sync 状态面板

Task: T-20260429-001  
Run: R-002  
Mode: Guided Drive  
Result: conditional  
Commit: def456

### What changed
- 增加 Adapter Sync 状态面板
- 增加 generated 文件冲突提示

### Key decisions
- AGENTS.md 只放静态协议
- 动态状态通过 agent-view/current.md 提供

### Risks
- 测试命令尚未标准化

### Follow-up
- 增加 adapter sync integration test
```

------

### 21.3 Knowledge Promotion

当 Review 发现长期有用知识时：

```text
journal → rules/preferences
journal → documents/stable
research/findings → documents/stable
review risks → debt tracker
```

P0 只做手动提示：

```text
本次有可沉淀经验，是否加入 rules？
```

P3 再做自动 spec update。

------

## 二十二、GUI 设计

### 22.1 第一屏

第一屏只回答 10 个问题：

1. 当前是什么任务？
2. 当前在哪个阶段？
3. 当前是 YOLO / Guided / Evidence 哪种模式？
4. Agent 是否正在运行？
5. 观测来源是什么？
6. 上下文是否完整？
7. Git 是否干净？
8. Research 是否需要 / 是否过期？
9. Handoff 是否完整？
10. 下一步应该点什么？

------

### 22.2 主界面布局

```text
┌──────────────────────────────────────────────┐
│ Project / Task / Mode / Git / Observability   │
├──────────────────────────────────────────────┤
│ 4+1 Flow: Align → Research → Plan → Impl → Rev │
├───────────────┬──────────────────────────────┤
│ Left          │ Main                         │
│ - Tasks       │ - Current phase output        │
│ - Context     │ - Diff / Evidence             │
│ - Research    │ - Handoff                     │
│ - Journal     │ - Observation grade           │
├───────────────┴──────────────────────────────┤
│ Actions: Continue / Pause / Review / Recover │
└──────────────────────────────────────────────┘
```

------

### 22.3 Context Panel

展示：

- 当前 Context Pack；
- included sources；
- excluded sources；
- token 估算；
- stale warning；
- missing context；
- Research Pack 是否包含；
- Agent 是否请求更多上下文；
- Context Pack 是否由 VibeHub 后端生成成功。

------

### 22.4 Observability Panel

展示：

```text
当前观测级别：Best Effort
运行时适配器：None

硬证据：
- Git diff
- changed-files.txt
- VibeHub-generated context manifest

Agent 自述：
- read files summary
- commands run
- rationale

推断：
- task mapping
- likely plan deviation
```

------

### 22.5 Handoff Panel

展示：

- Completed；
- Not Yet Done；
- Key Decisions；
- Context Still Needed；
- Warnings；
- Next Session Should；
- Handoff completeness。

------

### 22.6 Loop Warning UI

```text
⚠️ 可能进入补丁循环

触发条件：
- 同一文件已在本 run 中被修改 8 次
- Review 已失败 2 次
- diff 增长超过原计划范围 2.4x

建议：
[暂停并生成 Loop Report]
[进入 Review]
[重建 Context]
[继续 YOLO]
```

------

## 二十三、CLI 设计

### 23.1 用户常用命令

```bash
vibehub start
vibehub continue
vibehub pause
vibehub review
vibehub recover
vibehub finish
```

------

### 23.2 初始化

```bash
vibehub init
vibehub init --minimal
vibehub init --standard
```

P0 不做 full profile。
Full profile 留到 P2/P3。

------

### 23.3 高级命令

```bash
vibehub context build
vibehub context inspect
vibehub research start
vibehub research refresh
vibehub state check
vibehub git snapshot
vibehub adapter sync
vibehub adapter diff
vibehub journal add
vibehub housekeeping run
vibehub handoff build
vibehub current repair
```

这些不作为普通用户主入口。

------

## 二十四、与 Trellis 的借鉴清单

### 24.1 借鉴，但不照搬

| Trellis 成熟点                    | VibeHub r10 借鉴方式                                         |
| --------------------------------- | ------------------------------------------------------------ |
| workflow.md 作为单一工作流契约    | `workflow.yaml` 作为轻量流程骨架，阶段细则拆到 rules         |
| 少数入口命令                      | VibeHub 主入口收敛为 Start / Continue / Review / Recover / Finish |
| JSONL 精准上下文注入              | VibeHub 用 `context/*.yaml` + Manifest，保留 reason 字段     |
| research / implement / check 分工 | VibeHub 映射为 Research / Implement / Review，但更强调 GUI 可观测 |
| session start 自动上下文报告      | VibeHub 打开项目即生成上下文健康报告                         |
| finish-work 写 journal            | VibeHub finish 后生成 journal entry                          |
| update-spec 沉淀经验              | VibeHub P2/P3 加入 knowledge promotion                       |
| break-loop 根因分析               | VibeHub P3 加入 bug loop analysis                            |

Trellis 的日常文档显示，用户可以通过 `continue` 推进 workflow，并由系统处理 implement、check、update-spec、finish-work 等阶段路由；这个“低命令心智负担”的设计值得 VibeHub 借鉴。([Trellis 文档](https://docs.trytrellis.app/start/everyday-use?utm_source=chatgpt.com))

------

### 24.2 不照搬的点

| 不照搬                            | 原因                        |
| --------------------------------- | --------------------------- |
| 不把 VibeHub 定位成“给 AI 立规矩” | VibeHub 的核心是可观测放权  |
| 不优先实现完整 sub-agent 系统     | P0/P1 优先 4+1 流程闭环     |
| 不优先做 hooks 深注入             | 跨平台 hook 复杂度高，后移  |
| 不要求用户先接受完整规则框架      | VibeHub 要更低门槛、更 vibe |
| 不默认强制 task workflow          | 小改允许 YOLO Drive         |
| 不默认自动归档/commit             | 先保守，保留用户接管权      |

------

## 二十五、MVP 路线图

### P0：4+1 File Protocol Kernel

必须有：

- `.vibehub init`
- `state.yaml`
- `workflow.yaml`
- `rules/*.yaml`
- task/run/session 实体
- YAML current pointer
- agent-view
- context generator
- Context Pack
- Context Manifest
- Research Pack 可选
- handoff.md
- Review evidence
- Git baseline
- Agent reported output
- VibeHub validation
- housekeeping 基础策略
- observation grade

成功标准：

```text
一个任务能从 Align → Optional Research → Plan → Implement → Review 跑通。
小任务可以跳过 Research。
复杂任务可以生成 Research Pack。
Agent 不需要读取完整 .vibehub。
Agent 不需要自己组装 context pack。
Review 能区分硬证据、Agent 自述和推断。
Session 切换时 handoff 不丢失。
```

------

### P1：GUI Observable Cockpit

必须有：

- 4+1 流程图；
- 当前任务；
- 当前 phase；
- Continue / Pause / Review / Recover；
- Context Pack Viewer；
- Research Viewer；
- Git Diff Viewer；
- Evidence Viewer；
- Handoff Viewer；
- Observability Panel；
- Loop Warning；
- Markdown / Mermaid Preview；
- 状态异常提示。

成功标准：

```text
用户打开项目即可知道：
当前做到哪；
Agent 做了什么；
哪些是硬证据；
哪些只是 Agent 自述；
上下文是否完整；
Git 是否漂移；
handoff 是否足够；
下一步该点什么。
```

------

### P2：Low-friction Agent Integration

必须有：

- `AGENTS.md`；
- `CLAUDE.md`；
- OpenCode command；
- Antigravity workflow；
- Adapter Sync preview；
- Managed regions；
- Conflict detection；
- 简化 CLI；
- Journal；
- Knowledge promotion 手动版；
- v1 tag launcher 与 v2 agent-view 的联动。

成功标准：

```text
常见 AI coding 工具能读取 VibeHub 静态协议，并通过 agent-view 获取动态状态。
用户手改 generated 文件时，VibeHub 能检测冲突并给出解决选项。
```

------

### P3：Deep Integration & Compatibility

包含：

- OpenCode C2；
- Claude Runner；
- runtime observation；
- Trellis `.trellis` import；
- JSONL ↔ Context Manifest 转换；
- auto-trigger skills；
- bug loop analysis；
- Research refresh；
- adapter conflict resolver；
- context quality scoring；
- v1 AI Gateway 作为 Agent 模型路由底座。

------

### P4：Advanced Autonomy

包含：

- 多 Agent 并行；
- Git worktree 自动隔离；
- Agent work planner；
- CI gate；
- runtime sandbox；
- enterprise policy；
- team collaboration；
- context graph；
- cross-repo librarian；
- marketplace。

------

## 二十六、成功标准

### 26.1 P0 成功标准

| 指标                | 标准                                                         |
| ------------------- | ------------------------------------------------------------ |
| 4+1 跑通            | 一个任务能完成 Align / Plan / Implement / Review，复杂任务能插入 Research |
| YOLO 可用           | 小任务可以一句话开始，不强制重流程                           |
| 状态可信            | Agent 不能直接把真实状态标 completed                         |
| current 明确        | `tasks/current` 和 `runs/current` 是 YAML 指针文件           |
| Context 可生成      | VibeHub 后端能从 `context/*.yaml` 生成 context pack          |
| Handoff 可用        | session 切换时有完整 handoff                                 |
| 观测边界清晰        | UI 和报告明确区分 hard observed / agent reported / inferred  |
| Context 可解释      | 每个 Context Pack 有 Manifest                                |
| Research 可审计     | Research 有 source-log 和 research-pack                      |
| Research 作用域清晰 | `research/current` 绑定当前 active task                      |
| Review 有证据       | Review 至少包含 diff、changed files、test 或未测试原因       |
| Git 可恢复          | 能检测 dirty、baseline、HEAD mismatch                        |
| Agent 不被淹没      | Agent 默认只读 agent-view 和当前 context pack                |
| 文件不无限膨胀      | 有 housekeeping 和 archive 策略                              |
| v1 可延续           | v1 项目管理作为 v2 项目入口                                  |
| 用户可接管          | 任意阶段能 Pause / Recover / Review                          |

------

### 26.2 P1 成功标准

| 指标              | 标准                                                |
| ----------------- | --------------------------------------------------- |
| GUI 有价值        | 用户不用看文件也能理解当前状态                      |
| 4+1 可视化        | 阶段状态、输入、输出、风险清楚                      |
| Context 可视化    | included/excluded/stale/missing 清楚                |
| Research 可视化   | sources/findings/pack 清楚                          |
| Diff 可视化       | changed files 和 diff summary 清楚                  |
| Handoff 可视化    | 上一 session 完成了什么、没做什么、下一步做什么清楚 |
| 观测可信度可视化  | 用户知道哪些信息是硬证据，哪些是 Agent 自述         |
| Loop warning 可见 | 出现失败循环迹象时能提示用户                        |
| 下一步明确        | GUI 始终给出 Continue / Review / Recover 等推荐动作 |

------

## 二十七、最终架构总结

VibeHub r10 的核心链路是：

```text
用户一句话 / 当前项目 / 中途修改
  ↓
VibeHub v1 项目入口选择 repo
  ↓
VibeHub Start 生成上下文报告
  ↓
解析 YAML current pointer
  ↓
选择驾驶模式：
  ├─ YOLO Drive
  ├─ Guided Drive
  └─ Evidence Drive
  ↓
4+1 流程：
  Align / Align Lite
    ↓
  Optional Research
    ↓
  Plan
    ↓
  Implement
    ↓
  Review / Review Lite
  ↓
VibeHub 后端生成 Context Pack + Manifest
  ↓
Agent 读取 agent-view/current.md 和 context pack
  ↓
Agent 在边界内自主推进
  ↓
VibeHub 以 best-effort 方式记录过程：
  ├─ Git / 文件系统硬证据
  ├─ VibeHub 自己生成的上下文和状态
  ├─ Agent 自主汇报
  └─ VibeHub 推断
  ↓
所有观察结果标注 Evidence Grade
  ↓
Agent 输出 reported state 和 handoff notes
  ↓
VibeHub 生成 handoff.md
  ↓
VibeHub 校验并推进真实 state
  ↓
Journal 沉淀经验
  ↓
必要时 Recover / Replan / Research Refresh
```

最终一句话：

> **VibeHub r10 不追求用更多规则压住 Agent，而是让 Agent 在可观测、可恢复、可审计的边界内尽可能自由地完成工作。**

这版的核心判断是：

```text
P0/P1 不做全知运行时观测。
P0/P1 先把 4+1 做轻、做顺、做可信。
current 必须跨平台、显式、可恢复。
Context Pack 必须由 VibeHub 生成。
Handoff 是 Session 隔离的核心协议。
Research/current 绑定当前 active task。
v1 能力不是废弃，而是 v2 驾驶舱的入口和底座。
```

**VibeHub 的产品内核是：Observable YOLO，不是 workflow prison。**
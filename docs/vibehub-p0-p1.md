# VibeHub P0/P1 能力文档

> 生成: r10 stabilization session  
> 证据等级: hard_observed + agent_reported  
> 更新: 2026-05-13

---

## P0 已支持能力 (done)

### 1. 项目初始化 (`init`)
- 命令: `vibehub_init`
- 创建 `.vibehub/` 完整目录树（10个子目录）
- 写入 scaffold 文件: `project.yaml`, `state.yaml`, `workflow.yaml`, `housekeeping.yaml`
- 写入规则文件: `phase-rules.yaml`, `research-triggers.yaml`, `autonomy.yaml`, `review.yaml`, `loop-detection.yaml`, `preferences.yaml`, `hard-rules.md`
- 生成 `adapters/config.yaml`（含 template_version, enabled_tools, 命令hash追踪）
- 默认不自动触发 `sync_agent_adapters`（manual_by_default）；用户通过 AI Instructions / sync 动作显式生成 AGENTS.md, CLAUDE.md, opencode.json, 各平台命令文件
- 幂等: 已有文件不覆盖
- 测试: 5个 (`init::tests::*`)

### 2. 任务启动 (`start_task`)
- 命令: `vibehub_start_task`
- 生成任务 ID (`T-{timestamp}-{uuid8}`), 运行 ID (`R-{timestamp}-{uuid8}`)
- 创建任务目录结构, 上下文规范, task.yaml, run.yaml
- 写入指针文件: `tasks/current`, `runs/current`
- 构建初始 context pack
- 生成 agent view 文件
- 自动归档已完成的研究包
- 追加 run-scoped `events.jsonl` 任务启动事件
- 测试: 3个 (`start_task::tests::*`)

### 3. Context Pack 构建 (`context`)
- 命令: `vibehub_build_context_pack`
- 从 `context.yaml` 读取文件路径、排除规则、token预算(最大12k)
- 生成 `context-packs/{phase}.md` (Markdown) + `.manifest.yaml` (元数据)
- 支持 required/optional/secret 路径分类
- 测试: 6个 (`context::tests::*`)

### 4. Agent View 生成 (`agent_view`)
- 命令: `vibehub_generate_agent_view`
- 生成 `agent-view/current.md`（动态入口: 任务ID, 运行ID, 阶段, 模式, 输出合同）
- 生成 `agent-view/current-context.md`（上下文摘要: 包含/缺失/排除文件）
- 必要时生成 `agent-view/handoff.md`
- 测试: 2个 (`agent_view::tests::*`)

### 5. Agent Adapter 同步 (`agent_adapter`)
- 命令: `vibehub_sync_agent_adapter` / `vibehub_sync_agent_adapters`
- 支持三平台: Codex, Claude Code, OpenCode
- 管理区域标记 (`VIBEHUB:AGENT-INTEGRATION:START/END`) 实现安全合并
- SHA-256 哈希检测变更, 避免重复写入
- 产物: `AGENTS.md`, `CLAUDE.md`, `opencode.json`, 各平台命令文件(17个 `vibehub-*.md`)
- 测试: 4个 (`agent_adapter::tests::*`)

### 6. 工作区漂移检测 (`drift`)
- 命令: `vibehub_check_workspace_drift` / `vibehub_sync_workspace_state`
- 对比 git HEAD 与上次记录, 检测脏文件, 上下文过期, adapter冲突
- 生成推荐操作列表
- `sync_workspace_state` 额外触发 agent_view 重生成，并在存在 active run 时写 `.vibehub/tasks/{task}/runs/{run}/recover.md`
- 按 `loop-detection.yaml` 的 repeated_file_edits 阈值生成 loop warning
- 测试: 2个 (`drift::tests::*`)

### 7. 交接文档生成 (`handoff`)
- 命令: `vibehub_build_handoff`
- 解析 agent 输出文件 (`outputs/output.md` 或 `sessions/*/output.md`)
- 提取10个标准章节: Completed, Not Yet Done, Key Decisions, Files Changed 等
- 收集 git changed files 证据
- 生成 `agent-view/handoff.md`
- 追加 run-scoped `events.jsonl` handoff 事件
- 测试: 4个 (`handoff::tests::*`)

### 8. 审查证据生成 (`review`)
- 命令: `vibehub_generate_review_evidence`
- 收集 git diff, changed files, context manifest, 研究数据
- 生成结构化 Markdown 审查报告 (含 evidence map / evidence grades)
- 支持基线引用 (上次commit)
- 追加 run-scoped `events.jsonl` review evidence 事件
- 测试: 7个 (`review::tests::*`)

### 9. Cockpit 状态读取 (`status` / `overview`)
- 命令: `vibehub_read_overview`
- 聚合: 初始化状态, 任务/运行/阶段, git dirty, context pack, agent output, handoff
- 报告 observability_level、loop warning 和警告
- 测试: 2个 (`status::tests::*`)

### 10. 阶段状态机 (`phase`)
- 命令: `vibehub_validate_phase`, `vibehub_advance_phase`, `vibehub_complete_phase`, `vibehub_set_phase_result`
- 完整生命周期: pending -> active -> reported -> validating -> completed, needs_action/failed 分支
- 按 mode 定义的阶段顺序推进 (guided_drive: align/plan/implement/review)
- 验证必需输出(从 phase-rules.yaml), 缺少时阻断推进并设置 needs_action
- 输出检测优先 `outputs/output.md` 然后 `sessions/*/output.md`
- 上下文状态在推进时重置
- phase set/complete/advance/pause 追加 run-scoped `events.jsonl`
- 测试: 13个 (`phase::tests::*`)

### 11. 研究包管理 (`research`)
- 命令: `vibehub_build_research_pack`, `vibehub_archive_research`, `vibehub_read_research_status`
- 生成 research-pack.md + source-log.yaml + findings.yaml
- 自动归档: 新任务启动时把当前研究移动到 archive/
- 测试: 9个 (`research::tests::*`)

### 12. Cockpit 视图读取 (`cockpit`)
- 命令: `vibehub_read_vibehub_file`, `vibehub_read_context_view`, `vibehub_read_review_view`, `vibehub_read_handoff_view`, `vibehub_read_diff_view`
- 通用文件读取 (路径安全校验)
- 上下文/审查/交接/Diff 视图聚合数据

### 13. 日志 & 知识 (`journal` / `knowledge`)
- 命令: `vibehub_append_journal_entry`, `vibehub_append_knowledge_note`
- 追加时间戳条目到 `journal/index.md` 和 `journal/knowledge.md`
- 测试: 4个 (`journal::tests::*`, `knowledge::tests::*`)

### 14. 指针管理 (`current`)
- 命令: 内部 (VibeHub后端调用)
- 读写 `tasks/current` 和 `runs/current` YAML 指针文件
- 测试: 5个 (`current::tests::*`)

---

## P0 current / partial / missing

| 能力 | 状态 | 备注 |
|------|------|------|
| init | done | 幂等，完整 scaffold, 自动 adapter sync |
| start_task | done | 完整任务生命周期启动 |
| context pack | done | 支持 required/optional/secret 路径 |
| agent_view | done | current.md + current-context.md + handoff |
| agent_adapter | done | 三平台支持，哈希追踪 |
| drift | done | HEAD对比 + 脏文件 + 过期检测 + 自动修复 |
| handoff | done | 10章节解析 + git证据 |
| review | done | 9区块结构化YAML报告 |
| status | done | 完整 cockpit 状态聚合 |
| phase | done | 7状态状态机 + 输出验证 + 3模式 |
| research | done | 研究包创建/归档/查询 |
| cockpit views | done | 文件读取 + 4种视图数据 |
| journal/knowledge | done | 日志和知识条目追加 |
| current (pointers) | done | 指针文件管理 |
| workflow.yaml 三模式 | done | guided_drive / yolo_drive / evidence_drive |
| phase-rules.yaml | done | 7阶段输出要求完整 |
| 恢复 (recover) | **partial** | 已优先生成 run-scoped recover report；仍没有独立 recover 状态机 |
| 前端 cockpit UI | done | Status/Context/Review/Handoff/Research/Diff tabs，4+1 flow，并暴露 phase validate/complete/advance/pause 操作 |
| events 日志 | done | 最小事件写入器记录 task start、phase、review、handoff、sync、recover |
| runtime observation (P2) | **deferred** | P0 observability 是 best-effort |

---

## P1 已支持能力 (done)

### 1. 文件协议 (`adapters/protocol.md`)
- 共享输出合同: 10个必须章节, 2个输出路径, 4种证据标签
- 工具说明: Codex/Claude/OpenCode 加载链路
- 停止条件: 输出缺失或状态不一致时的处理

### 2. 适配器指令 (`AGENTS.md` / `CLAUDE.md` / `opencode.json`)
- 三套代理入口, 内容一致 (仅 Applies to 行不同)
- 与 protocol.md 一致: 相同启动读取列表, 相同输出合同, 相同证据标签
- 管理区域标记保证安全合并

### 3. 规则文件 (7个)
- phase-rules.yaml, hard-rules.md, review.yaml, research-triggers.yaml, autonomy.yaml, loop-detection.yaml, preferences.yaml

### 4. 命令命名空间 (`vibehub-*`)
- 17个 agent 命令 (checkpoint/context/continue/diff/finish/handoff/help/init/journal/knowledge/plan/recover/research/review/start/status/sync)
- 三平台均生成到对应目录

### 5. 前端类型 & API 绑定 (26个类型 + 27个API方法)
- 完整 TypeScript 接口覆盖所有后端命令
- 三语 i18n (en / zh-CN / zh-TW)

### 6. CLI 入口
- `--vibehub-sync-workspace` 无GUI漂移检查/修复模式

---

## P1 current / partial / missing

| 能力 | 状态 | 备注 |
|------|------|------|
| 文件协议 | done | protocol.md 完整 |
| 适配器指令 | done | AGENTS.md/CLAUDE.md/opencode.json 三入口一致 |
| 规则文件 | done | 7个完整 |
| 命令命名空间 | done | 17个 agent 命令 |
| 前端类型绑定 | done | 26类型 + 27 API方法 |
| CLI sync 入口 | done | --vibehub-sync-workspace |
| state.yaml lite阶段 | done | init模板和 start_task flow 均覆盖 align_lite / review_lite |
| loop-detection 集成 | **partial** | drift/sync/status 已生成和展示 repeated_file_edits warning；phase/review 更细粒度循环检测仍可深化 |
| .claude/ gitignore | done | `.claude/`, `.codex/`, `.opencode/` 等本地 agent 状态已忽略 |

---

## P2 / P3 / P4 deferred

| 优先级 | 能力 | 备注 |
|--------|------|------|
| P2 | runtime observation | 目前 best-effort, 无运行时监控 |
| P2 | events 时间线 UI | 后端已有最小 events.jsonl，前端 timeline 尚未实现 |
| P2 | loop-detection 运行时集成 | 已有 drift/sync/status warning，phase/review 细化检测仍后续 |
| P2 | 前端 cockpit 深化 | 现有 phase验证/推进/暂停与 evidence viewer 已暴露；后续可加入完整 events timeline |
| P3 | multi-agent orchestration | 多代理协作编排 |
| P3 | webhook/外部触发 | 外部事件驱动的阶段推进 |
| P4 | agent 性能分析 | token 使用追踪, 成本估算 |

---

## 文件协议速查

### 输出文件路径
| 优先级 | 路径 | 场景 |
|--------|------|------|
| 首选 | `.vibehub/tasks/<task_id>/runs/<run_id>/outputs/output.md` | 常规输出 |
| 备选 | `.vibehub/tasks/<task_id>/runs/<run_id>/sessions/<session_id>/output.md` | 会话级输出 |

### 输出文件必须章节
1. `## Completed`
2. `## Not Yet Done`
3. `## Key Decisions Made`
4. `## Files Changed`
5. `## Files Reportedly Read`
6. `## Commands Run`
7. `## Tests Run`
8. `## Context Still Needed`
9. `## Warnings`
10. `## Next Session Should`

### 证据标签
| 标签 | 含义 |
|------|------|
| `hard_observed` | 工具直接观察到 (git diff, 文件存在) |
| `agent_reported` | 代理自己报告 (命令运行, 测试结果) |
| `inferred` | 代理推理得出 |
| `user_confirmed` | 用户确认 |

---

## 常用流程

### 启动新任务 (guided_drive)
```
vibehub_init → vibehub_start_task(mode=guided_drive, phase=implement)
  → agent 工作 (读取 agent-view/current.md, 按 protocol.md 写输出)
  → vibehub_advance_phase (推进到 review)
  → vibehub_generate_review_evidence → vibehub_build_handoff
```

### YOLO 快速模式
```
vibehub_start_task(mode=yolo_drive, phase=implement)
  → agent 工作
  → vibehub_complete_phase
```

### 恢复 dirty workspace
```
vibehub_check_workspace_drift
  → vibehub_sync_workspace_state (如需要)
  → vibehub_generate_agent_view
  → vibehub_sync_agent_adapters
```

---

## 验证命令

```bash
# 后端测试
cd src-tauri && cargo test vibehub

# 前端构建
npm run -s build

# 单个模块测试
cargo test vibehub::phase     # 阶段状态机 (13 tests)
cargo test vibehub::init      # 初始化 (5 tests)
cargo test vibehub::review    # 审查 (7 tests)
cargo test vibehub::handoff   # 交接 (4 tests)
cargo test vibehub::start_task # 任务启动 (3 tests)
cargo test vibehub::research  # 研究 (9 tests)
cargo test vibehub::context   # 上下文包 (6 tests)
cargo test vibehub::drift     # 漂移检测 (2 tests)
cargo test vibehub::status    # 状态 (2 tests)
cargo test vibehub::agent_adapter # 适配器 (4 tests)
cargo test vibehub::agent_view   # 代理视图 (2 tests)
cargo test vibehub::journal      # 日志 (3 tests)
cargo test vibehub::knowledge    # 知识 (2 tests)
cargo test vibehub::current      # 指针 (5 tests)
cargo test vibehub::cockpit      # cockpit (5 tests)
```

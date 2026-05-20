# VibeHub r10 差距审计报告

生成日期: 2026-05-13  
对照文档: `C:/Users/11231/Desktop/vibehub-v2-r10.md`  
仓库: `D:/LocalRepo/VBdingHome`  
分支: `feature/vibehub-v2-p0`

## 结论摘要

当前实现不是空壳。后端 VibeHub 模块已经覆盖了 P0/P1 的大量核心能力，并且 `cargo test vibehub` 通过 89 个测试，前端 `npm.cmd run -s build` 也通过。

但当前工作树不是一个可用的 VibeHub active workspace。所有 `.vibehub/agent-view/*`、`.vibehub/state.yaml`、`.vibehub/workflow.yaml`、`.vibehub/rules/*`、`.vibehub/tasks/current` 等运行态文件在工作树里被删除。也就是说，代码能力和当前项目状态是分裂的。

综合判断:

- 代码能力层: P0 约 70%-80% 成熟，P1 约 45%-60% 成熟。
- 当前项目接入层: 断裂，必须先恢复或重新生成 `.vibehub` 才能继续按协议工作。
- P2: 适配器/命令生成能力已有基础，但协议文件和生成物当前缺失，集成体验仍不稳定。
- P3/P4: 基本按 r10 预期后移，未实质实现。

证据等级说明:

- `hard_observed`: Git 状态、文件存在性、源码、测试命令直接观察到。
- `agent_reported`: 本次审计运行命令后的汇报。
- `inferred`: 基于源码和 r10 要求推断。
- `user_confirmed`: 用户提供的原始 r10 文档路径和审计请求。

## 当前最严重差距

| 严重度 | 差距 | 证据 | 判断 |
| --- | --- | --- | --- |
| P0 blocker | `.vibehub` 当前工作树缺失 | `git status --short` 显示 `.vibehub/...` 全部为 `D`; `Test-Path .vibehub` 为 missing | `hard_observed` |
| P0 blocker | AGENTS/CLAUDE 要求先读 `.vibehub/agent-view/current.md`，但该文件不存在 | `AGENTS.md` 当前仍指向 agent-view；工作树无 `.vibehub` | `hard_observed` |
| P0 blocker | 当前 active task/run 无法解析，也无法写 active run output | HEAD 中 state 显示曾有 active task/run；工作树删除了 state、current pointer、run/task 文件 | `hard_observed` |
| P1 gap | GUI 的流程条还不是 r10 的 4+1 阶段图 | 前端显示 `plan/context/run/review/handoff`，不是 `Align/Research/Plan/Implement/Review` | `hard_observed` |
| P1/P2 gap | `docs/vibehub-p0-p1.md` 的 done 清单与当前文件状态不一致 | 文档声称 `protocol.md`/规则/命令文件 done；当前 `.vibehub/adapters/protocol.md` 不存在，HEAD 也没有该文件 | `hard_observed` |
| P0/P1 gap | Recover 不是 r10 规定的 run-scoped recover report | 代码写 `.vibehub/recover.md` 和 sync report，不是 `.vibehub/tasks/{task}/runs/{run}/recover.md` | `hard_observed` |
| P0 gap | Git snapshot/baseline 仍偏弱 | review 可读取 baseline，state 有字段，但未看到完整 snapshot/事件闭环 | `inferred` |
| P0/P1 gap | events journal 未实现 | docs 自述 `events.jsonl` missing；源码仅预留 housekeeping exclude | `hard_observed` |
| P1 gap | Loop warning 规则存在但未进入状态机/UI触发闭环 | `loop-detection.yaml` 模板存在，docs 自述 phase.rs 未调用 | `hard_observed` |

## P0 对照

| r10 P0 要求 | 当前状态 | 证据与差距 |
| --- | --- | --- |
| `.vibehub init` | 代码已实现；当前项目状态缺失 | `init.rs` 创建 project/state/workflow/housekeeping/rules/adapters config；但当前工作树 `.vibehub` 被删除。`hard_observed` |
| `state.yaml` | 模板和迁移已实现；当前文件缺失 | `init.rs` 写 schema v2 state，`state_migration.rs` 迁移 r9->r10；当前 `.vibehub/state.yaml` 为 deleted。`hard_observed` |
| `workflow.yaml` 三模式 | 已实现 | `init.rs` 模板包含 `yolo_drive/guided_drive/evidence_drive` 和 `align_lite/review_lite`。`hard_observed` |
| `rules/*.yaml` | 模板已实现；当前运行态缺失 | init 模板写 phase/research/autonomy/review/loop/preferences/hard-rules；当前 `.vibehub/rules/*` 被删。`hard_observed` |
| task/run/session 实体 | 部分实现 | `start_task.rs` 创建 task/run/context/outputs/sessions 目录和 YAML；session 实体主要是输出路径约定，没有完整 session 生命周期。`inferred` |
| YAML current pointer | 已实现；当前指针缺失 | `current.rs` 读写 `tasks/current`、`runs/current` 并校验 kind/path；当前指针文件被删。`hard_observed` |
| agent-view | 已实现；当前文件缺失 | `agent_view.rs` 生成 `current.md/current-context.md/handoff.md`；当前 `.vibehub/agent-view/*` 被删。`hard_observed` |
| Context Pack generator | 已实现 | `context.rs` 从 context YAML 生成 pack + manifest，含 token budget、missing/excluded、quality、evidence grade。`hard_observed` |
| Context Manifest | 已实现 | manifest 写 `source_commit/budget/included/missing/excluded/quality/observation`。`hard_observed` |
| Research Pack | 基础已实现 | `research.rs` 生成 `research-pack.md/source-log.yaml/findings.yaml`，能归档 current research；但内容仍是模板，不是自动研究引擎。`hard_observed` + `inferred` |
| handoff.md | 已实现且比早期更完整 | `handoff.rs` 解析 output、Git changed files、context manifest，并输出 10 类交接信息。`hard_observed` |
| Review evidence | 已实现 | `review.rs` 生成 `phases/review.md`、`evidence/changed-files.txt`、`evidence/diff.patch`，包含 evidence map、verdict、context/research/git sections。`hard_observed` |
| Git baseline | 部分实现 | review 可发现 baseline，drift 检测 dirty/head/context stale；但 r10 里的 checkpoint/snapshot 事件闭环还不完整。`inferred` |
| Agent reported output | 部分实现 | phase/handoff/review 都解析 `outputs/output.md` 或 `sessions/*/output.md`；但当前 active output 不存在。`hard_observed` |
| VibeHub validation | 部分实现 | `phase.rs` 能按 phase-rules 校验必需输出并阻断 advance；但没有完整 `Agent reported -> VibeHub validating -> completed` 事件流。`inferred` |

## P1 GUI 对照

| r10 P1 要求 | 当前状态 | 证据与差距 |
| --- | --- | --- |
| 当前任务/phase/mode/git/observability | 部分实现 | cockpit overview 聚合 status/context/review/handoff/diff/research；状态卡显示 mode/phase/git/context/handoff。`hard_observed` |
| 4+1 流程图 | 未按 r10 实现 | UI 当前 flow 文案为 `plan/context/run/review/handoff`，不是 Align/Research/Plan/Implement/Review。`hard_observed` |
| Continue/Pause/Review/Recover | 部分实现 | UI 有 sync/continue、review、handoff、workspace sync/recover、phase validate/complete/advance；未看到明确 Pause 按钮完整暴露。`hard_observed` |
| Context Viewer | 已有 | Context tab 显示 pack、manifest、phase、included/missing/excluded/stale。`hard_observed` |
| Research Viewer | 基础已有 | Research tab 显示 required/status/pack exists/path；未显示 sources/findings 详情。`hard_observed` |
| Git Diff Viewer | 基础已有 | Diff tab 显示 dirty、changed files、diff stat；未显示完整 patch 浏览。`hard_observed` |
| Evidence Viewer | 部分实现 | Review tab 显示 review summary；后端 review 文件有 evidence map，但 UI 未形成独立 evidence viewer。`inferred` |
| Handoff Viewer | 基础已有 | Handoff tab 显示 exists/complete/path/sections/missing。`hard_observed` |
| Markdown/Mermaid Preview | 未确认实现 | 未看到 Markdown/Mermaid 渲染器，只是摘要/文本字段。`inferred` |
| 状态异常提示 | 部分实现 | uninitialized、dirty/context/handoff 推荐动作存在。`hard_observed` |
| Evidence Grade 展示 | 部分实现 | 后端报告写 evidence grade；UI 没有系统化 grade 面板。`inferred` |
| 第一屏 10 个问题 | 部分覆盖 | 当前能看任务、phase、mode、git、context/handoff；Agent 是否运行、Research 是否过期、下一步推荐部分弱。`inferred` |

## P2 对照

| r10 P2 要求 | 当前状态 | 证据与差距 |
| --- | --- | --- |
| `AGENTS.md` | 已有 | 当前文件包含 managed protocol block 和 sync 规则。`hard_observed` |
| `CLAUDE.md` | 当前 untracked | 文件存在但未跟踪；内容与 AGENTS 基本一致。`hard_observed` |
| OpenCode command markdown | 代码可生成；当前本地 ignored 文件存在 | `.opencode/commands/*` 在 ignored local state 中；Git 不跟踪。`hard_observed` |
| Antigravity workflow/rules | 未见实现 | 未看到 `.agents` 之外的 Antigravity 特定 workflow/rules 产物。`hard_observed` |
| Adapter Sync preview | 部分实现 | `sync_agent_adapters(dry_run)` 与 status/conflict 支持存在。`hard_observed` |
| Managed regions | 已实现 | AGENTS/CLAUDE 使用 managed markers，代码有冲突检测。`hard_observed` |
| Conflict detection | 已实现基础 | adapter 对 managed marker、generated command 修改能检测 conflict。`hard_observed` |
| 简化 CLI | 已实现基础 | `main.rs` 支持 `vibehub start/continue/sync/status/review/recover/handoff/pause/validate/advance/finish/migrate/locale`。`hard_observed` |
| Journal | 基础实现 | `journal.rs` 追加 journal 条目；未与 finish 自动闭环。`hard_observed` + `inferred` |
| Knowledge promotion 手动版 | 基础实现 | `knowledge.rs` 追加 note；未做 promotion 决策流。`hard_observed` + `inferred` |
| v1 tag launcher 联动 | 未充分审计到 | 现有项目中心/launcher 仍在，但 v2 agent-view 联动程度不明确。`inferred` |

## Recover / Sync 对照

当前实现有三种相关能力:

- `drift.rs`: 检测 Git dirty、HEAD 变化、context stale、adapter conflicts。
- `sync.rs`: 生成 sync report、重建 context、刷新 agent-view、同步 adapter、写 `.vibehub/agent-view/sync.md`。
- `phase.rs`: 校验输出并阻断推进。

主要差距:

- r10 的 Recover Report 路径是 `.vibehub/tasks/{task_id}/runs/{run_id}/recover.md`，当前 drift recover 写 `.vibehub/recover.md`，sync 写 run 下 `sync/*.md`。这是协议路径不一致。
- `sync_workspace` 会先 `init_project`，再尝试 agent-view/context/adapter sync；在 `.vibehub` 缺失时可重新初始化，但这会创建新 scaffold，不等价于恢复原 active task。
- Recover 仍是 safe-mode 报告/建议为主，没有完整状态机。

## 文档自述与事实不一致

`docs/vibehub-p0-p1.md` 是有价值的能力说明，但当前已经不完全可信:

- 它声称 P0/P1 多项 done，但当前 `.vibehub` 运行态文件全部被删除。
- 它说 `adapters/protocol.md` done，但当前工作树和 HEAD 都没有 `.vibehub/adapters/protocol.md`；只是 `agent_adapter.rs` 已具备生成能力。
- 它说后端测试 72 个，实际本次运行是 89 个。
- 它说 init 自动触发 adapter sync；当前 `init.rs` 已改为 `sync_adapters: false` 默认不自动同步。这更符合 r10 的 manual_by_default，但说明文档已过期。

## 验证结果

已运行:

```text
cargo test vibehub
```

结果: 89 passed, 0 failed。警告仅为 gateway 里未使用方法。

已运行:

```text
npm.cmd run -s build
```

结果: Vite build 成功。警告为 bundle 体积、Browserslist/caniuse 数据过期。

## 建议优先级

1. 先决定 `.vibehub` 当前删除是否有意。如果不是有意，恢复它；如果是有意，就用当前代码重新 init，并明确旧 active task/run 已废弃。
2. 把 `.vibehub/adapters/protocol.md` 作为实际产物生成出来，修复 `opencode.json` 指向缺失文件的问题。
3. 修正 `docs/vibehub-p0-p1.md`，区分“代码支持”“当前项目已接入”“已通过端到端手测”。
4. 补齐 P1 4+1 可视化，把 UI flow 改成 Align / Research / Plan / Implement / Review，并按 mode 显示 lite 阶段。
5. 把 Recover report 路径改为 run-scoped，或在文档里明确当前实现是 project-scoped safe recover。
6. 实现最小 events writer，至少记录 phase advance、review evidence、handoff build、sync report。
7. 把 loop-detection 从配置接到 phase/review/sync 的 warning 生成链路。
8. 为 agent-view/current.md、context pack、review evidence、handoff 加一条真实端到端 fixture 测试，验证从 init -> start -> output -> advance -> review -> handoff 的完整磁盘产物。

## 本次审计无法完成的协议动作

按 AGENTS.md，本来应在 active run output path 写 phase output。但当前 `.vibehub/agent-view/current.md` 和 `.vibehub/state.yaml` 均不存在，active run output path 无法解析。因此本次只写入本审计报告，没有推进或修改 VibeHub canonical state。

## 二次复查更新

复查时间: 2026-05-13  

上一节中“`.vibehub` 整体缺失”的结论已经过期。第二轮复查显示 `.vibehub` 已重新生成/恢复，包含:

- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/adapters/protocol.md`
- `.vibehub/adapters/hooks/vibehub-stop-check.mjs`
- `.vibehub/sync.md`
- `.vibehub/workflow.yaml`
- `.vibehub/rules/*.yaml`

新的当前状态:

- `state.yaml` schema 已为 `2`。
- 当前没有 active task/run: `task_id: null`, `run_id: null`, `phase: null`, `phase_status: idle`。
- `.vibehub/tasks/current` 仍不存在，旧 task/run 文件仍在 Git 中显示删除。
- sync report 显示 `changed_files: 64`，并要求用户确认这些变更是否都属于同一 VibeHub 任务。
- agent-view 可读，但内容明确写着 “No active task has been created yet.”
- context pack 和 manifest 仍未生成，因为没有 active task/run/phase。

已修正或改善的差距:

- `.vibehub/adapters/protocol.md` 已存在，旧报告中“protocol.md 缺失”的 blocker 已解除。
- Codex 生成命令文件已恢复到 `.vibehub/adapters/generated/codex/`。
- `events.rs` 已出现，并且 `phase.rs` / `sync.rs` 已接入事件写入点；旧报告中“events writer 未实现”的结论已过期。
- drift/loop warning 已能在 sync/state 中显示，当前 state 记录了 `Loop warning: 64 changed file(s) meets or exceeds repeated_file_edits threshold 8.`

仍然成立的主要差距:

- 当前项目没有 active task/run，VibeHub 仍不能把本轮工程状态归属到一个明确任务。
- `.vibehub/tasks/current` 缺失导致 `agent_view::generate_agent_view` 在 sync 中仍显示 skipped。
- 当前 64 个变更文件未被用户确认归属；这仍是继续推进前的核心风险。
- P1 GUI 的 4+1 可视化、Evidence Viewer、Markdown/Mermaid Preview 等产品层缺口仍需另行补。
- Recover 路径是否完全符合 r10 run-scoped 规范还需继续核验；源码测试中已有 `recover_report_prefers_active_run_path`，但当前没有 active run，无法端到端观察实际产物。

第二轮验证:

```text
cargo test vibehub --target-dir target-codex-check
```

结果: 92 passed, 0 failed。新增可见测试包括 `events::tests::appends_run_event_jsonl`、`drift::tests::loop_detection_threshold_adds_warning`、`drift::tests::recover_report_prefers_active_run_path`。

```text
npm.cmd run -s build
```

第一次构建短暂失败于 Vite/Rollup absolute `index.html` emit 路径；立即重跑后成功。当前判定为 transient build failure，非稳定复现。成功构建仍有 bundle size、Browserslist/caniuse 数据过期警告。

更新后的优先建议:

1. 先创建或恢复 active task/run，让 `.vibehub/tasks/current` 和 run pointer 指向真实任务。
2. 把当前 64 个变更明确归属到一个 VibeHub 任务，或拆分不相关变更。
3. 生成当前 phase 的 context pack + manifest。
4. 写入 active run output 后再运行 phase validation / review evidence / handoff。
5. 将本报告旧结论视为“第一次检查快照”，以本二次复查段落为当前状态。

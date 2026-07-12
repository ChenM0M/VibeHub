# M5 Align - 以 M1/M4 体验自管理多会话与 worktree 生命周期

Task: T-20260711080144-de5ecb84
Run: R-20260711080144-924b5971
Phase: align (active)

## Intent

- `user_confirmed`: M1 当前产品体验是冻结基线；只有 M4 稳定后才允许 V3 自己管理 M5，避免过早 self-host。
- `agent_reported`: M5 在现有计划图、时间线、概要/NodeBrief 中落地多 agent session、scope/lease、branch/worktree、integration/conflict/recovery 全生命周期，并用 M5 自身作为受控 shadow run 证明 V3 可以管理真实并行工程工作。

## Scope

- `agent_reported`: 强制消费 M4 stability evidence 和项目所有者批准；未满足时 M5 保持 blocked，V2 继续管理，不能部分 self-host。
- `agent_reported`: 定义 PlanNode parallel eligibility、declared/observed scope、overlap severity、shared/generated/migration file denylist 和 unknown-scope policy。
- `agent_reported`: 定义 branch/worktree identity、base commit、root/path budget、task/node/session owner、lease/heartbeat/reclaim challenge 和 stale owner recovery。
- `agent_reported`: 实现 planned/creating/ready/active/dirty/submitted/integrating/conflicted/integrated/abandoned/repairing/cleaned 状态机及结构化 Git/native error mapping。
- `agent_reported`: 实现每个 eligible node 的创建、锁定、agent cwd 启动、dirty/commit checks、integration queue、pre-integration validation、base drift、conflict ownership、retry、retention 和 cleanup。
- `agent_reported`: 通过 M1/M4 现有 UI 表达：PlanGraph node/session distribution 显示 owner/并行/阻塞/集成状态，timeline 显示 lease/worktree/integration/conflict/recovery 事件，NodeBrief 显示 scope/base/branch/cwd/next action，概要显示任务级风险与验收。
- `agent_reported`: V3 记录 M5 的 Task/PlanNode/Session/Worktree events；V2 只保留最小 audit pointer、shadow comparison 和 rollback control。
- `user_confirmed`: launcher/cockpit 是只读看板；criterion review、completion confirmation 与 workflow truth 由 agent 通过受控 CLI/MCP/application command surface 判断和写入，production UI 不提供直接改写入口。

## Success Criteria

- `agent_reported`: 以下 M5-C01 至 M5-C10 共同定义进入条件、并行隔离、恢复和产品可解释性。

1. `M5-C01 强制进入门`: M4-C10 的完整 stability report、预声明 soak、双平台/三 host evidence、零 blocking defect 和项目所有者明确批准均存在；任一缺失时 M5 self-host 不启动。
2. `M5-C02 Shadow 可回滚`: M5 自身的 task/node/session/criterion 由 V3 记录并可 rebuild；V2 minimal audit 能对比关键投影并一键停止继续 self-host。回滚不丢 M5 代码工作，也不伪造 V3 状态成功。
3. `M5-C03 并行资格`: 启动前评估 declared scope、历史/实时 observed changes、shared-file denylist、unknown scope 和 dependency readiness；重叠/不确定节点在现有 PlanGraph 中明确 warning/block，不静默并行。
4. `M5-C04 三会话隔离`: 至少三个无重叠 PlanNodes、三个并行 sessions、至少两种 agent tools；每个 session 的 cwd/index/HEAD/branch/worktree lease/event identity 独立，跨 workspace/event contamination 为零。
5. `M5-C05 Worktree 状态完整`: 从 creating 到 cleaned 的每个 transition 有 event/evidence/owner；UI 能沿现有 plan/timeline/brief 交互解释 base、branch、owner、cwd、dirty/commit、integration state 和 next action。
6. `M5-C06 冲突不丢失`: 人为制造同文件与 shared generated file 冲突；系统在启动前预警或 integration 时明确 conflict owner、files、base drift、resolution/retry path，绝不静默覆盖或自动选择胜者。
7. `M5-C07 故障恢复`: agent crash、desktop exit、orphan process、missing/moved worktree、stale lease、Windows locked file/antivirus occupation 后均可诊断和恢复；dirty worktree 永不自动 remove/reset，reclaim 需 challenge/evidence。
8. `M5-C08 Git/native 安全`: 所有 Git 调用使用 argument arrays 和稳定 porcelain parser；覆盖 unborn/detached/already checked out、long path、drive/UNC/case、path occupation 和 application shutdown，macOS/Windows 行为有实测矩阵。
9. `M5-C09 M1 产品守恒`: 不新增割裂的编排主页面；四 Tab、task 选择、PlanGraph layout/toggle/node click、timeline filter/detail、NodeBrief 与结构架构入口保持一致，只在既有信息位增加真实 session/worktree/integration data。看板交互限于选择、筛选、展开、刷新和导航，不直接写 criterion/completion/workflow truth。
10. `M5-C10 完成闭环`: 三并行节点成功提交并按 project policy 集成，required criteria 经 M4 规则逐项验收；冲突、repair 和 integration attempts 全史可回放，M5 自身能 kill-resume 并从正确 task/node/base 继续。

## Acceptance Criteria

- `agent_reported`: M5-C01 至 M5-C10 全部 required；无 owner 的 worktree、dirty destructive cleanup、workspace/event contamination、静默 conflict resolution 或未经批准 self-host 均为 blocking。
- `agent_reported`: RFC-005 D1-D7 和 native spikes 必须结案；merge/rebase/cherry-pick 由显式 project policy 决定，不能硬编码为全局默认。
- `user_confirmed`: M5 第一次 self-host 启动和从 shadow 转为主跟踪都需要项目所有者看到证据后明确批准。

## Non-Goals

- `user_confirmed`: 不改变 M1 四 Tab 与核心交互，不另做“多 agent 大屏”替代当前 cockpit。
- `agent_reported`: 不让系统自行决定产品目标、拆 task 或替用户完成语义确认；编排只管理已批准 PlanNodes、scope、执行环境和集成生命周期。
- `user_confirmed`: 不把 launcher 变成人工验收控制台；不得在 production cockpit 增加通过/失败/阻塞、提出完成、确认完成等 domain mutation controls。
- `agent_reported`: 不支持 remote/cloud runner、多人权限系统或任意 daemon orchestration。
- `agent_reported`: 不自动删除 dirty worktree、不自动解决冲突、不绕过 M4 Criterion/confirmation truth rules。

## Autonomy Level

- `agent_reported`: `guided_drive / critical risk`。实现和非破坏性测试可自主执行；self-host activation、shadow promotion/rollback、destructive Git cleanup、integration policy 变更和平台 gate 豁免必须用户明确确认。

## Key Decisions Made

- `user_confirmed`: M5 是第一个由稳定 V3 管理的里程碑，M0-M4 不提前 self-host。
- `agent_reported`: worktree 不是独立功能按钮，而是 PlanNode/session execution 的受控生命周期；产品状态落在现有 plan/timeline/brief。
- `agent_reported`: 一个 active parallel PlanNode 默认拥有一个 branch/worktree lease；只读 session 需显式例外。
- `agent_reported`: V2 在 shadow 期间只有 audit/rollback 权，不双写 V3 domain，也不成为第二投影源。

## Completed

- `hard_observed`: 对照 M1 产品基线、M4 stability gate、M5 task metadata、主计划 M5 与 RFC-005，完成 M5 entry/self-host、orchestration、recovery 和 UI 映射验收重写。
- `hard_observed`: 审计并移除 M4 引入的 desktop lifecycle write chain；production cockpit 恢复为只读验收展示，agent/core lifecycle 保留。
- `hard_observed`: 按 M1-M4 归属建立 5 个 Git commits，形成可归因 baseline；`.vibehub/v3/` 本地 event store 已加入忽略。

## Not Yet Done

- `agent_reported`: M5 Research/Plan/Implement/Review、M4 gate evidence 和 owner self-host approval 均尚未发生；当前不能启动 self-host。

## Files Changed

- `hard_observed`: `.vibehub/tasks/T-20260711080144-de5ecb84/runs/R-20260711080144-924b5971/outputs/output.md`。
- `hard_observed`: `vibehub switch/sync` 管理的 agent-view、context pack、events、index 与 projection 文件。
- `hard_observed`: 本轮边界修正涉及 `src/v3/app/V3Cockpit.tsx`, `AcceptanceProgress.tsx`, `src/services/v3ProductionViews.ts`, `src-tauri/src/commands.rs`, `src-tauri/src/main.rs`, `.gitignore`，已随里程碑 commits 记录。

## Files Reportedly Read

- `hard_observed`: VibeHub current/current-context/handoff/hard-rules/protocol 与当前 Align context pack。
- `hard_observed`: M1 Review/current PlanGraph/NodeBrief/timeline interaction、M4 refined Align output。
- `hard_observed`: `docs/vibehub-v3-redesign-plan.md` M5、`docs/v3/rfc-backlog/005-worktree-orchestration.md`、RFC-004 stability gate 与 research pack。

## Commands Run

- `hard_observed`: `vibehub switch . T-20260711080144-de5ecb84`, `vibehub sync .`。
- `hard_observed`: `rg`, `sed`, `find` 等任务、源码和文档读取命令。
- `hard_observed`: `npm run build`, `cargo fmt --all -- --check`, `cargo test --workspace`；按里程碑执行 `git add`/`git commit`。

## Tests Run

- `hard_observed`: production build 通过；workspace tests 通过：Tauri 17、adapters 2、core 233，零失败。
- `not_tested`: Align 尚未运行 worktree/native/self-host tests。
- `hard_observed`: M1 golden evidence 和 M4 定义的未来 stability evidence 是 M5 输入；目前不构成 M5 gate pass。

## Context Still Needed

- `agent_reported`: Research/Plan 需冻结 M4 gate digest、worktree root/path budget、branch naming、scope overlap policy、integration strategy、lease timeouts、shadow comparison/rollback triggers 和 Windows test host。

## Warnings

- `hard_observed`: M0-M4 代码与任务历史已按里程碑提交，当前建立了可归因 baseline；M5 后续仍需在每次 VibeHub sync 后保持状态提交纪律。
- `inferred`: 当前 PlanGraph 的 agent distribution 是 mock-derived；M5 必须替换为真实 session/worktree projection，同时保留用户已认可的 toggle、节点和详情交互。

## Next Session Should

1. `agent_reported`: validate/output-lint 本 Align output；未经用户确认不 finish/advance。
2. `agent_reported`: M4 未通过完整 gate 前保持 M5 blocked，不做 self-host spike 写入真实 M5 state。
3. `agent_reported`: gate 通过后 Research/Plan 按 eligibility -> lease/worktree lifecycle -> launcher isolation -> integration/conflict -> recovery -> shadow/rollback -> product/native regression 推进。

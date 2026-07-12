# M5 Review - 修复与完整复审

Task: T-20260711080144-de5ecb84
Run: R-20260711080144-924b5971
Phase: review (active)

## Completed

- `user_confirmed`: 用户要求根据上一轮审查问题完成修复，并在修复后重新进行完整自行审查。
- `hard_observed`: 上一轮 6 个 findings 已全部修复并有负向回归测试：operation journal 跳阶段、owner override 绕过 dependency/parallel gate、非 Ready worktree 普通 acquire、`..`/absolute/root escape scope、released lease reclaim、idempotency payload/actor/version 漂移。
- `hard_observed`: 完整复审额外发现并修复 Rust command API 可接受空白 session/lease/operation identity 与空 reclaim challenge 的边界；acquire/reclaim 现在要求非空 next challenge。
- `hard_observed`: 以基线 `b24a9e9a15db755a5663f9dd9686fa74cc90e177` 重新审查 contracts/fixtures、eligibility/path、Git adapter/journal、event store/orchestration、projection/application 五条链路。
- `hard_observed`: 最终代码审查未发现剩余可操作问题；`verdict = approved_with_external_validation_pending`，`gate_pass = true`（仅针对当前实现 diff 的 Review gate）。

### Diff Summary

- `agent_reported`: M5 contracts 与基础 core slice 的安全边界现已与 RFC-005 一致：dependency/parallel gate 不可 override，路径先做平台无关 lexical normalization，普通 lease acquire 只允许 Ready，所有 ownership transfer 走带 challenge 和 dead-process evidence 的 reclaim，released lease 不可复活，Git operation 和 idempotent request 都保持真实顺序与语义。
- `hard_observed`: 复审证据已重新生成到 `evidence/diff.patch`、`evidence/changed-files.txt` 与 `phases/review.md`。

### Concerns

- `hard_observed`: 无剩余 P0/P1/P2 代码 finding。
- `agent_reported`: 真实临时仓库三 worktree/integration、agent kill/desktop exit、Windows locked-file native smoke 与 self-host shadow 尚未执行；这些是 M5 环境验收项，不是当前 diff 中已观察到的代码缺陷。

### Gate Pass

- `hard_observed`: `gate_pass = true`，原 6 项 review blockers 及复审新增的 identity/challenge blocker 均已修复并通过回归测试。
- `agent_reported`: 此 gate 不等同于打开 `entry_gate.self_host_writes_allowed`；M4 gate digest、Windows native evidence 与 owner approval 未满足前，真实 self-host 写入仍须保持关闭。

### Verdict

- `hard_observed`: `verdict = approved_with_external_validation_pending`。
- `hard_observed`: 当前实现 diff 无剩余可操作代码 finding；Review gate 通过，native/self-host 环境验收继续保持 pending。

### Risk Review

- `hard_observed`: ownership、scope、side-effect truthfulness 与 idempotency 的已知绕过路径均有拒绝测试覆盖。
- `inferred`: 本地单元/契约测试无法替代跨进程 crash、Windows 文件锁和真实 Git 冲突恢复，残余风险集中在尚未运行的 native integration harness。
- `inferred`: 工作区包含 115 个 VibeHub/任务相关变更文件，未获得逐文件用户归属确认；本轮仅增量修改当前 M5 core/review 输出，未回退或改写其他已有变更。

### Evidence Grades

- `hard_observed`: Git diff/source、VibeHub sync/review evidence、257 个 Rust tests、423 项 contract assertions、TypeScript/Vite build、format 与 whitespace checks。
- `agent_reported`: 修复意图、Review gate 与 self-host entry gate 的边界、后续 native harness 建议。
- `inferred`: 未运行的跨进程/Windows/self-host 场景残余风险与未确认 dirty-file ownership 风险。
- `user_confirmed`: 按既有审查问题修复并完成一次完整复审的用户指令。

## Not Yet Done

- `not_tested`: 未运行真实临时仓库三 worktree、merge/rebase/cherry-pick、故意冲突、agent kill/desktop exit 场景。
- `not_tested`: 未运行 Windows native path/locked-file smoke 或真实 M5 self-host shadow。
- `agent_reported`: 未执行 `vibehub finish` 或 `vibehub advance`；阶段仍由用户确认后再转换。

## Key Decisions Made

- `agent_reported`: 只有 scope overlap、case collision、denylist/generated/canonical 与 observed-file collision 可被带证据 owner override 降级；dependency、parallel safety 与 invalid path 永久不可 override。
- `agent_reported`: lease 普通 acquire 只允许 Ready；Active/Reclaimed ownership transfer 必须使用同 lease identity、generation+1、非空 challenge 与 dead-process evidence；release 清空 challenge。
- `agent_reported`: event-store duplicate 必须比较 expected version、完整 identity、actor、evidence grade、commit、payload 与显式 occurred_at；真正相同的 retry 才返回 Duplicate。
- `agent_reported`: Review gate 与 self-host entry gate 分离，代码复审通过不自动声明 native/self-host 验收完成。

## Files Changed

- `hard_observed`: `crates/vibehub-core/src/v3/worktree.rs`。
- `hard_observed`: `crates/vibehub-core/src/v3/git_runner.rs`。
- `hard_observed`: `crates/vibehub-core/src/v3/orchestration.rs`。
- `hard_observed`: `crates/vibehub-core/src/v3/event_store.rs`。
- `hard_observed`: `crates/vibehub-core/src/v3/lifecycle.rs`（修正严格幂等语义下的 soak retry 版本）。
- `hard_observed`: 当前 `outputs/output.md`；`vibehub sync/review` 另自动更新 agent-view、sync、events/index/state、context pack 与 review evidence 等 VibeHub 管理文件，未手工编辑 canonical state/pointers。

## Files Reportedly Read

- `hard_observed`: `.vibehub/agent-view/{current,current-context,handoff,sync}.md`、hard rules、adapter protocol、workflow、Review context pack、prior output 与 review evidence。
- `hard_observed`: `crates/vibehub-core/src/v3/{application,domain,event_store,git_runner,lifecycle,mod,orchestration,projection,worktree}.rs`。
- `hard_observed`: `contracts/v3` 相关 schema、`scripts/v3-contracts`、generated TypeScript、fixture repository、12 套 fixtures 与 RFC-005。

## Commands Run

- `hard_observed`: `vibehub status VibeHub`（参数路径错误，未改变状态）、`vibehub sync .`、`vibehub review .`。
- `hard_observed`: `rg`、`sed`、`nl`、`git status`、`git diff`、`git diff --stat/--numstat/--check` 用于定位、复审和证据核对。
- `hard_observed`: `cargo fmt --manifest-path crates/vibehub-core/Cargo.toml` 及 `--check`。
- `hard_observed`: 多组专项 `cargo test`、全量 `cargo test --manifest-path crates/vibehub-core/Cargo.toml -- --quiet`、`npm run v3:contracts:check`、`npm run build`。
- `hard_observed`: `cargo clippy --manifest-path crates/vibehub-core/Cargo.toml --all-targets -- -D warnings`。

## Tests Run

- `hard_observed`: `vibehub-core` 全量 257/257 tests 通过，doc tests 通过。
- `hard_observed`: V3 contract check 通过：423 assertions、12 scenarios、6 views、2 write contracts。
- `hard_observed`: `npm run build` 通过 TypeScript 与 Vite production build（仅有 browsers data 过期及 bundle >500 kB 的既有 warning）。
- `hard_observed`: `cargo fmt --check` 与 `git diff --check` 通过。
- `hard_observed`: `cargo clippy -D warnings` 未通过，受当前仓库旧 `vibehub/*` 模块约 40 个既有 lint（too_many_arguments、needless_borrow、redundant_closure 等）阻塞；输出未指向本轮新增 M5 模块。

## Context Still Needed

- `agent_reported`: M4 full gate digest、Windows native host evidence、owner self-host approval。
- `agent_reported`: 工作区其余 dirty files 的逐文件 ownership 未由用户确认；按协议保留为 unresolved risk。

## Warnings

- `hard_observed`: VibeHub sync 仍报告 repeated_file_edits 与工作区外部变更提醒。
- `agent_reported`: 在 native integration/self-host evidence 完成前，不应把当前 core slice 接入真实 launcher 写路径或打开 self-host write gate。
- `hard_observed`: 构建生成的 `dist` 未出现在 Git status 中，未产生需纳入本轮的 tracked 变更。

## Next Session Should

1. `agent_reported`: 用户确认后运行 VibeHub Review validation/output lint；不要自动 finish/advance。
2. `agent_reported`: 在临时真实仓库执行三 worktree、integration/conflict、crash/desktop-exit harness。
3. `agent_reported`: 在 Windows native host 补 locked-file/path smoke，并取得 M4 digest 与 owner approval 后再评估 self-host write gate。

# Implement Phase Output

## Completed

- `hard_observed`: 修复 `crates/vibehub-core/src/vibehub/archive.rs` 的归档状态收口逻辑。`archive_completed_tasks()` 现在会在归档当前任务后正规更新 `.vibehub/tasks/current`、对应 remaining task 的 `runs/current`、`state.current`、`state.pointers` 和当前 context pack；如果没有 remaining active task，则清空 current 状态并删除 stale pointer 文件。
- `hard_observed`: `archive_completed_tasks()` 现在只把 `completed/complete/done/cancelled/canceled` 视为可归档状态，不再把 `needs_action` 当成自动归档目标。
- `hard_observed`: 修复 `crates/vibehub-core/src/vibehub/start_task.rs`。`start_task()` 在创建新任务前会自动调用 archive prune，清理已完成/已取消的 active task，避免 `tasks.active` 越积越多。
- `hard_observed`: 修复 `crates/vibehub-cli/src/main.rs`。`vibehub start-intake <project> --stdin` 和 `vibehub start-intake <project> -` 现在可以直接从 stdin 读取 JSON；原来的 `<json_path>` 仍兼容。
- `hard_observed`: 更新 `crates/vibehub-core/src/vibehub/agent_adapter.rs` 中 `vibehub-start-intake` 的 CLI hint、preflight 和 task 文案，改为 stdin 优先；运行 `sync-adapters` 后同步更新 Codex/Claude/OpenCode 的 start-intake 命令文件。
- `hard_observed`: 新增回归测试覆盖：
  - 归档 current task 后切到剩余 active task，并保持 phase_status/context/current pointer 正确。
  - 归档最后一个 current task 后清空 state.current 和 pointer 文件。
  - 新建 task 前自动清理 completed active task。

## Not Yet Done

- `agent_reported`: 未处理已有 adapter conflict：`.vibehub/adapters/generated/codex/vibehub-help.md` 在 VibeHub 外部被修改，`sync-adapters` 本轮仍报告冲突。
- `agent_reported`: 未做 UI 层改动；本轮修复发生在 core/CLI/adapter 文案层。

## Key Decisions Made

- `inferred`: archive 的权威行为应该是“对齐 active list、current 指针和 state.current”，而不是只删 `tasks.active` 里的条目；否则 GUI 和 agent 解析入口会继续指向旧任务。
- `inferred`: `needs_action` 不应自动归档，因为它表示需要人处理或补输出，不是任务完成。
- `inferred`: start-intake 保留文件路径模式，同时新增 stdin，兼顾脚本/agent 的低摩擦调用和现有用户工作流。
- `inferred`: `start_task()` 自动 prune 只清 completed/cancelled，避免误清仍需产品/用户处理的 blocked task。

## Files Changed

- `crates/vibehub-core/src/vibehub/archive.rs`
- `crates/vibehub-core/src/vibehub/start_task.rs`
- `crates/vibehub-cli/src/main.rs`
- `crates/vibehub-core/src/vibehub/agent_adapter.rs`
- `.agents/skills/vibehub-start-intake/SKILL.md`
- `.vibehub/adapters/generated/codex/vibehub-start-intake.md`
- `.claude/commands/vibehub-start-intake.md`
- `.opencode/commands/vibehub-start-intake.md`

## Files Reportedly Read

- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`
- `.vibehub/tasks/T-20260529145443-52bb4f48/runs/R-20260529145443-224267fe/context-packs/review.md`
- `crates/vibehub-core/src/vibehub/archive.rs`
- `crates/vibehub-core/src/vibehub/start_task.rs`
- `crates/vibehub-core/src/vibehub/current.rs`
- `crates/vibehub-core/src/vibehub/task_switch.rs`
- `crates/vibehub-cli/src/main.rs`
- `crates/vibehub-core/src/vibehub/agent_adapter.rs`

## Commands Run

- `git status --short`
- `sed -n ...` / `rg ...` context inspection commands
- `cargo fmt`
- `cargo test --target-dir /private/tmp/vibehub-target -p vibehub-core --offline`
- `cargo test --target-dir /private/tmp/vibehub-target -p vibehub-cli --offline`
- `mkdir -p /private/tmp/vibehub-stdin-smoke`
- `printf '{"title":"stdin intake smoke","mode":"guided_drive","split_confidence":"low"}' | cargo run --target-dir /private/tmp/vibehub-target -p vibehub-cli --offline -- start-intake /private/tmp/vibehub-stdin-smoke --stdin`
- `cargo run --target-dir /private/tmp/vibehub-target -p vibehub-cli --offline -- sync-adapters /Users/chenm0m/LocalRepo/VibeHub`

## Tests Run

- `hard_observed`: `cargo test --target-dir /private/tmp/vibehub-target -p vibehub-core --offline` — 195 passed.
- `hard_observed`: `cargo test --target-dir /private/tmp/vibehub-target -p vibehub-cli --offline` — 0 tests, compile/test target passed.
- `hard_observed`: stdin smoke test for `vibehub start-intake <project> --stdin` succeeded and returned one created task with active current pointers in `/private/tmp/vibehub-stdin-smoke`.

## Context Still Needed

- `agent_reported`: Need product/owner decision for existing `.vibehub/adapters/generated/codex/vibehub-help.md` external modification: keep override, import as managed override, or overwrite from generated source.

## Warnings

- `hard_observed`: Workspace was already dirty before this turn, including many `.vibehub` generated/state files and deleted `.vibehub/tasks/current`; those unrelated pre-existing changes were not reverted.
- `hard_observed`: `sync-adapters` updated only the four start-intake adapter files and reported one pre-existing conflict for `vibehub-help.md`.
- `agent_reported`: The stdin smoke test created a temporary project under `/private/tmp/vibehub-stdin-smoke`.

## Next Session Should

- Review/stage the four code changes plus four start-intake adapter generated files.
- Decide how to handle the existing `vibehub-help.md` adapter conflict.
- Optionally run an end-to-end manual flow in the real cockpit: finish a task, start a new one, confirm `tasks.active` and GUI current state stay clean.

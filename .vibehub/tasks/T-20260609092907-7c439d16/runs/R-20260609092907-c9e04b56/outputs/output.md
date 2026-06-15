## Completed

- `user_confirmed`: 用户要求一次性优化 VibeHub 面向 Agent 暴露的定位、能力边界、工具调用和流程约束问题。
- `hard_observed`: 无头 CLI 二进制名已从 `vibehub` 改为 `vibehub-cli`，帮助文本和 Agent-facing 命令示例统一使用 `vibehub-cli ...`。
- `hard_observed`: Agent 协议和适配器模板已加入一句话定位：VibeHub 是 coding agent 的 project memory、task router 和 workflow gatekeeper；Agent 做工程工作，VibeHub 追踪 state/context/gates/output/handoff。
- `hard_observed`: `next-action` 已支持 intent-aware 路由，能把 Agent/VibeHub 协议审视路由到 `vibehub-start`，把继续/同步路由到 `vibehub-sync`，把多 active task 校验风险路由到 `validate-task`。
- `hard_observed`: CLI 已新增 `next-action`、`validate-task`、`output-lint`，并对 `finish`、`advance`、`archive` 强制要求 `--confirmed-by-user`。
- `hard_observed`: Adapter generator 已移除旧的 `.vibehub/notes/status.md` 与 `phases/<phase>.output.md` lifecycle artifact 要求，统一要求写 active run 的 `outputs/output.md`。
- `hard_observed`: Codex、Claude Code、OpenCode 的 adapter/skills/commands 已同步生成；`adapter-status` 最终无 warnings。
- `hard_observed`: 第二轮自检修正了 `next-action` 的歧义路由：`继续当前状态` 仍路由到 `vibehub-sync`，但 `继续优化 Agent 协议和工具调用流程` 会优先识别为 VibeHub/Agent 协议改进工作并路由到 `vibehub-start`。

## Not Yet Done

- `hard_observed`: 无阻塞未完成项；implement 输出已通过 task-scoped validate。
- `inferred`: 工作区仍包含相邻 UI 任务的既有未提交变更，本任务未尝试回滚或归并那些 UI 文件。

## Key Decisions Made

- `agent_reported`: 将 VibeHub 的 Agent 心智模型收敛为“状态拥有者 + 路由器 + gatekeeper”，减少 Agent 把 VibeHub误当工程执行者或普通文档库的概率。
- `agent_reported`: 用 `vibehub-cli` 明确区分无头 Agent CLI 与桌面/Tauri app，避免裸 `vibehub` 命令撞到桌面入口。
- `agent_reported`: 对状态转换采取“CLI 可执行但必须显式用户确认”的约束，而不是只在文档里提醒，降低 Agent 忘记确认或过度自动推进的风险。
- `agent_reported`: 对多 active task 引入 `validate-task` 和协议提示，避免默认 `validate` 校验错误 current pointer。
- `agent_reported`: `next-action` 先看用户最新 intent，再看当前 phase/output 状态，避免新需求或元评估被误判成“当前 phase ready to validate”。
- `agent_reported`: 元层 Agent/VibeHub 协议意图优先于泛化的“继续/同步”词匹配；这保留了同步保守性，同时避免明确的协议优化请求被误吸成普通 refresh。

## Diff Summary

- `hard_observed`: CLI 层改名并扩展命令面：`vibehub-cli` 帮助文本、`next-action`、`validate-task`、`output-lint`、状态转换确认 gate。
- `hard_observed`: Core 层新增 `next_action` 与 `output_lint` 模块，扩展 phase task-scoped validation，并修正 advance 后 agent-view 刷新时序。
- `hard_observed`: Agent adapter 模板、协议、constraints、skills、Claude/OpenCode commands 和 docs 已同步到新的命令名、操作循环、多任务校验指导、输出 lint 指导。
- `hard_observed`: Prompt 模板的 new-task/sync 文案已改为 `vibehub-cli`，并强调多意图拆分、继续/刷新先同步、agent-view 重新生成一致性。

## Files Changed

- `hard_observed`: `crates/vibehub-cli/Cargo.toml`
- `hard_observed`: `crates/vibehub-cli/src/main.rs`
- `hard_observed`: `crates/vibehub-core/src/vibehub/agent_adapter.rs`
- `hard_observed`: `crates/vibehub-core/src/vibehub/next_action.rs`
- `hard_observed`: `crates/vibehub-core/src/vibehub/output_lint.rs`
- `hard_observed`: `crates/vibehub-core/src/vibehub/phase.rs`
- `hard_observed`: `crates/vibehub-core/src/vibehub/mod.rs`
- `hard_observed`: `crates/vibehub-core/templates/prompts/en/new-task.md`
- `hard_observed`: `crates/vibehub-core/templates/prompts/en/sync.md`
- `hard_observed`: `crates/vibehub-core/templates/prompts/zh-CN/new-task.md`
- `hard_observed`: `crates/vibehub-core/templates/prompts/zh-CN/sync.md`
- `hard_observed`: `crates/vibehub-core/templates/prompts/zh-TW/new-task.md`
- `hard_observed`: `crates/vibehub-core/templates/prompts/zh-TW/sync.md`
- `hard_observed`: `docs/vibehub-skills-registry-v1.md`
- `hard_observed`: `AGENTS.md`
- `hard_observed`: `CLAUDE.md`
- `hard_observed`: `.vibehub/adapters/config.yaml`
- `hard_observed`: `.vibehub/adapters/protocol.md`
- `hard_observed`: `.vibehub/adapters/generated/codex/vibehub-*.md`
- `hard_observed`: `.agents/skills/vibehub-*/SKILL.md`
- `hard_observed`: `.claude/commands/vibehub-*.md`
- `hard_observed`: `.opencode/commands/vibehub-*.md`
- `hard_observed`: `.codex/vibehub/constraints.md`
- `hard_observed`: `.claude/vibehub/constraints.md`
- `hard_observed`: `.opencode/vibehub/constraints.md`
- `hard_observed`: `.vibehub/skills.registry.yaml`
- `hard_observed`: `.vibehub/tasks/T-20260609092907-7c439d16/runs/R-20260609092907-c9e04b56/outputs/output.md`

## Files Reportedly Read

- `hard_observed`: `.vibehub/agent-view/current.md`
- `hard_observed`: `.vibehub/agent-view/current-context.md`
- `hard_observed`: `.vibehub/agent-view/handoff.md`
- `hard_observed`: `.vibehub/rules/hard-rules.md`
- `hard_observed`: `.vibehub/adapters/protocol.md`
- `hard_observed`: `.vibehub/workflow.yaml`
- `hard_observed`: `.vibehub/tasks/T-20260609092907-7c439d16/runs/R-20260609092907-c9e04b56/context-packs/implement.md`
- `hard_observed`: `.vibehub/tasks/T-20260609092907-7c439d16/runs/R-20260609092907-c9e04b56/outputs/output.md`
- `hard_observed`: `crates/vibehub-cli/src/main.rs`
- `hard_observed`: `crates/vibehub-core/src/vibehub/agent_adapter.rs`
- `hard_observed`: `crates/vibehub-core/src/vibehub/next_action.rs`
- `hard_observed`: `crates/vibehub-core/src/vibehub/output_lint.rs`
- `hard_observed`: `crates/vibehub-core/src/vibehub/phase.rs`
- `hard_observed`: `.agents/skills/vibehub-continue/SKILL.md`
- `hard_observed`: `.agents/skills/vibehub-help/SKILL.md`
- `hard_observed`: `.vibehub/adapters/generated/codex/vibehub-help.md`

## Commands Run

- `hard_observed`: `/private/tmp/vibehub-agent-hardening-target/debug/vibehub-cli status /Users/chenm0m/LocalRepo/VibeHub`
- `hard_observed`: `/private/tmp/vibehub-agent-hardening-target/debug/vibehub-cli switch /Users/chenm0m/LocalRepo/VibeHub T-20260609092907-7c439d16`
- `hard_observed`: `/private/tmp/vibehub-agent-hardening-target/debug/vibehub-cli sync-adapters /Users/chenm0m/LocalRepo/VibeHub` failed under sandbox while writing `.agents/skills/vibehub-advance/SKILL.md`.
- `hard_observed`: `/private/tmp/vibehub-agent-hardening-target/debug/vibehub-cli sync-adapters /Users/chenm0m/LocalRepo/VibeHub` passed with escalated permission; updated 114 adapter files, skipped 37, initially reported 2 conflicts.
- `hard_observed`: `/private/tmp/vibehub-agent-hardening-target/debug/vibehub-cli adapter-status /Users/chenm0m/LocalRepo/VibeHub`
- `hard_observed`: `rg -n 'vibehub (start|status|sync|validate|finish|advance|claim|gates|review|handoff|recover|output-lint|next-action|archive|switch|pause|start-intake|--help)' AGENTS.md CLAUDE.md .agents/skills .codex/vibehub .claude/commands .opencode/commands .vibehub/adapters/protocol.md .vibehub/adapters/generated/codex`
- `hard_observed`: `cargo fmt`
- `hard_observed`: `cargo build --target-dir /private/tmp/vibehub-agent-hardening-target -p vibehub-cli --offline`
- `hard_observed`: `/private/tmp/vibehub-agent-hardening-target/debug/vibehub-cli --help`
- `hard_observed`: `/private/tmp/vibehub-agent-hardening-target/debug/vibehub-cli next-action /Users/chenm0m/LocalRepo/VibeHub "继续当前状态"`
- `hard_observed`: `/private/tmp/vibehub-agent-hardening-target/debug/vibehub-cli next-action /Users/chenm0m/LocalRepo/VibeHub "深度评价当前项目面向 Agent 暴露的能力边界和工具调用流程"`
- `hard_observed`: `/private/tmp/vibehub-agent-hardening-target/debug/vibehub-cli next-action /Users/chenm0m/LocalRepo/VibeHub "多 active 任务时不要错验，请用 validate-task"`
- `hard_observed`: `/private/tmp/vibehub-agent-hardening-target/debug/vibehub-cli finish /Users/chenm0m/LocalRepo/VibeHub` refused without `--confirmed-by-user`.
- `hard_observed`: `/private/tmp/vibehub-agent-hardening-target/debug/vibehub-cli advance /Users/chenm0m/LocalRepo/VibeHub` refused without `--confirmed-by-user`.
- `hard_observed`: `/private/tmp/vibehub-agent-hardening-target/debug/vibehub-cli validate-task /Users/chenm0m/LocalRepo/VibeHub T-20260609092907-7c439d16`
- `hard_observed`: `git status --short`
- `hard_observed`: `git diff --stat`
- `hard_observed`: `cargo test --target-dir /private/tmp/vibehub-agent-hardening-target -p vibehub-core --offline next_action`
- `hard_observed`: `cargo test --target-dir /private/tmp/vibehub-agent-hardening-target -p vibehub-core --offline`
- `hard_observed`: `cargo test --target-dir /private/tmp/vibehub-agent-hardening-target -p vibehub-cli --offline`
- `hard_observed`: `cargo run --target-dir /private/tmp/vibehub-agent-hardening-target -p vibehub-cli --offline -- next-action /Users/chenm0m/LocalRepo/VibeHub 继续优化 Agent 协议和工具调用流程`
- `hard_observed`: `cargo run --target-dir /private/tmp/vibehub-agent-hardening-target -p vibehub-cli --offline -- next-action /Users/chenm0m/LocalRepo/VibeHub 继续当前状态`

## Tests Run

- `hard_observed`: `cargo test --target-dir /private/tmp/vibehub-agent-hardening-target -p vibehub-core --offline next_action` passed after second-pass routing fix: 8 passed, 0 failed.
- `hard_observed`: `cargo test --target-dir /private/tmp/vibehub-agent-hardening-target -p vibehub-core --offline output_lint` passed: 2 passed, 0 failed.
- `hard_observed`: `cargo test --target-dir /private/tmp/vibehub-agent-hardening-target -p vibehub-core --offline` passed after second-pass routing fix: 207 passed, 0 failed.
- `hard_observed`: `cargo test --target-dir /private/tmp/vibehub-agent-hardening-target -p vibehub-cli --offline` passed: 0 tests, 0 failed.
- `hard_observed`: CLI smoke `next-action "继续当前状态"` returned `action=sync_workspace`, `skill=vibehub-sync`, `cli=vibehub-cli sync <project>`.
- `hard_observed`: CLI smoke meta Agent/VibeHub intent returned `action=start_task`, `skill=vibehub-start`, `cli=vibehub-cli start <project> <mode> <title>`.
- `hard_observed`: CLI smoke `next-action "继续优化 Agent 协议和工具调用流程"` returned `action=start_task`, proving the second-pass ambiguity fix works.
- `hard_observed`: CLI smoke multi-active validation intent returned `action=validate_task`, `skill=vibehub-validate`, `cli=vibehub-cli validate-task <project> <task_id>`.
- `hard_observed`: CLI smoke `finish` and `advance` without `--confirmed-by-user` returned exit code 2 and refused to run.
- `hard_observed`: Adapter status after sync returned no warnings.

## Context Still Needed

- `hard_observed`: 无阻塞上下文缺口。

## Warnings

- `hard_observed`: 工作区仍有相邻 UI 任务改动，包括 `src/components/VibehubCockpitDialog.tsx`、locale 文件和 ProjectDetail/Structure 组件；本任务未回滚这些文件。
- `hard_observed`: VibeHub status 仍报告 loop detection warning，因为工作区变更文件数达到阈值；这不是本次测试失败。
- `agent_reported`: `sync-adapters` 在普通沙箱下无法写 `.agents`，已按权限流程使用 escalated run 完成。

## Next Session Should

- `agent_reported`: 如用户确认要进入 review/finish，先运行 task-scoped `vibehub-cli validate-task <project> T-20260609092907-7c439d16` 与 `vibehub-cli output-lint <project> T-20260609092907-7c439d16`，再请求用户明确确认 `finish`。
- `agent_reported`: 不要把相邻 UI 任务的 dirty files 归入本任务成果；需要 UI 收口时切回 `T-20260531153015-3a283c0b`。

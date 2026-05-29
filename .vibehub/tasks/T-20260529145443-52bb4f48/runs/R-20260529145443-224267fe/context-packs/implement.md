# Context Pack: Implement

Task: T-20260529145443-52bb4f48
Run: R-20260529145443-224267fe
Phase: Implement
Generated at: 2026-05-29T14:56:07Z
Source commit: 3252e1f

## Instructions

Use this context only for the current phase.
Do not mark state.yaml completed.
Report files read, commands run, decisions made, and unresolved risks.

## Capability Output Schema

```json
{
  "required_fields": [
    "diff_summary",
    "changed_files",
    "commands_run"
  ],
  "optional_fields": [
    "rollback_plan",
    "references"
  ],
  "produces": [
    "diff"
  ],
  "consumes": [
    "implementation_plan"
  ],
  "parallel_safe": false,
  "custom": false
}
```

## Prior Outputs Summary

```json
[
  {
    "capability": "plan",
    "completed": [
      "`hard_observed`: 创建新 task `T-20260529145443-52bb4f48` / run `R-20260529145443-224267fe`，模式 evidence_drive，phase=align。",
      "`hard_observed`: 对整个 VibeHub 代码库进行了深度审查，覆盖以下模块：",
      "所有 26 个 `.agents/skills/vibehub-*/SKILL.md` 技能提示词文件",
      "`crates/vibehub-core/src/vibehub/agent_adapter.rs` — adapter 生成逻辑",
      "`crates/vibehub-core/src/vibehub/agent_view.rs` — agent-view 生成",
      "`crates/vibehub-core/src/vibehub/prompts.rs` — prompt 模板系统",
      "`crates/vibehub-core/src/vibehub/start_task.rs` — task 创建流程",
      "`crates/vibehub-core/src/vibehub/current.rs` — task/run 指针管理",
      "`crates/vibehub-core/src/vibehub/sync.rs` — 同步逻辑",
      "`crates/vibehub-core/src/vibehub/projection.rs` — 状态投影",
      "`crates/vibehub-core/src/vibehub/phase.rs` — phase 验证与推进",
      "`crates/vibehub-core/src/vibehub/events.rs` — 事件系统",
      "`crates/vibehub-core/src/vibehub/workflow.rs` — workflow 定义",
      "`crates/vibehub-core/src/vibehub/cockpit.rs` — cockpit 视图",
      "`.vibehub/adapters/protocol.md` — 输出合约",
      "`AGENTS.md` / `CLAUDE.md` — agent 入口",
      "`templates/prompts/*/` — GUI prompt 模板",
      "`hard_observed`: 在 `agent_adapter.rs:1262-1337` 确认 skill body 由 `default_command_body()` 硬编码，不从 workflow.yaml 动态读取 phase-specific 信息。",
      "`hard_observed`: 在 `phase.rs:294-317` 确认 phase 验证通过 `required_output_to_section_keys` 做模糊映射（changed_files → \"Files Changed\"），agent 无法推断。"
    ],
    "full_ref": ".vibehub/tasks/T-20260529145443-52bb4f48/runs/R-20260529145443-224267fe/outputs/output.md",
    "key_decisions": [
      "`inferred`: 问题根因是 skill 提示词全部硬编码在 `agent_adapter.rs:1262-1337` 的 `default_command_body()` 中，没有从 workflow.yaml 或 phase-rules.yaml 动态生成。",
      "`inferred`: 修复策略应为让 skill 生成管道读取 workflow.yaml 和各 phase 的 required_outputs，在 task 正文中明确告知 agent 当前 phase 的期望产出。",
      "`inferred`: Adapter 生成的多文件架构（5 层）应精简为 2 层（protocol + commands），减少 context 窗口浪费和内容重复。",
      "## 完整问题清单 (17 项)",
      "### 高优先级 (5 项)",
      "| # | 严重程度 | 问题 | 根因位置 | 症状 |",
      "|---|---------|------|----------|------|",
      "| **1** | **高** | `vibehub-start` skill 只说\"Draft\"不强调必须调 CLI | `agent_adapter.rs:1269` + `.agents/skills/vibehub-start/SKILL.md:20-21` | Agent 可能脑内完成 draft，不调用 `vibehub start`，task 不会在 state.yaml 注册为 active，GUI 看不到 |",
      "| **2** | **高** | Task 如何变 active 的流程对 agent 完全黑盒 | `start_task.rs:171-181`(写指针文件), `:922-1014`(update_state 含 add_active_task) | Agent 无法理解 task 创建 ≠ task 注册，无法自查问题 |",
      "| **3** | **高** | 所有 skills 的 Task 描述太笼统（1-2 行），缺少 phase-specific 细节 | `default_command_body()` case 分支全是单行描述 | Agent 在 implement phase 不知道要产出 changed_files/diff_summary/commands_run |",
      "| **4** | **高** | Phase 验证和 advance gate 逻辑对 agent 不可见 | `phase.rs:691-742`（缺失 output 时 block）, `:746`（handoff gate block） | Agent 不知道 output.md 要写什么才能通过 advance 验证 |",
      "| **5** | **高** | `vibehub-continue` 没有 Stop Condition section | `.agents/skills/vibehub-continue/SKILL.md` | Agent 不知道什么时候该停止、该写 output |",
      "### 中优先级 (7 项)",
      "| # | 严重程度 | 问题 | 根因位置 | 症状 |",
      "|---|---------|------|----------|------|",
      "| **6** | **中** | Output requirement 样板 15 行 × 26 skills = 390 行重复 | 所有 SKILL.md 文件包含完全相同的 Output requirements 段 | Banner blindness：agent 忽略关键指令 |",
      "| **7** | **中** | Adapter 多文件重叠：agent 收到 5 层文件 | `agent_adapter.rs:424-635` — 每个 tool 生成 protocol.md + constraints.md + command-index.md + skills + 静态入口（AGENTS.md） | Context 窗口浪费，核心信息稀释 |",
      "| **8** | **中** | Phase validation 用模糊 key 映射 | `phase.rs:294-317` — `required_output_to_section_keys` 做非精确匹配（changed_files → \"Files Changed\"） | Agent 无法从 section 名反推 required_output |",
      "| **9** | **中** | `vibehub-sync` 没有 Stop Condition | `.agents/skills/vibehub-sync/SKILL.md` | Sync 后 agent 不知道是否该继续工作 |",
      "| **10** | **中** | GUI prompt templates 过于简短 | `templates/prompts/en/new-task.md` 仅 13 行, `sync.md` 仅 9 行 | 从 cockpit 发起的 prompt 给 agent 的信息极少 |",
      "| **11** | **中** | `sync.md` prompt 不告诉 agent 该收集哪些硬证据 | `templates/prompts/en/sync.md:1-9` | Agent 可能跳过 Git status/diff 等关键检查 |",
      "| **12** | **中** | AGENTS.md 说 \"Use generated vibehub-* commands\" 但 agent 可能不知道它们存在 | `AGENTS.md:31-33` | 循环依赖 |",
      "### 低优先级 (5 项)",
      "| # | 严重程度 | 问题 | 根因位置 | 症状 |",
      "|---|---------|------|----------|------|",
      "| **13** | **低** | 大 worktree (125 files) 触发 loop detection 假阳性 | `state.yaml:55-57`, `events.rs:135-138` (LoopWarning) | 无关噪音污染 agent context |",
      "| **14** | **低** | zh-CN/zh-TW templates 和 en 几乎一样短 | `templates/prompts/zh-*/*.md` | 非英语 locale 没有额外价值 |",
      "| **15** | **低** | `vibehub-recover` 没有分析步骤 | `.agents/skills/vibehub-recover/SKILL.md:20-21` | Agent 不知道如何做 recover |",
      "| **16** | **低** | `vibehub-checkpoint` 和 `vibehub-finish` 的 lifecycle artifacts 完全重复 | 两个 SKILL.md 的 \"Agent-written lifecycle artifacts\" 段 | 维护负担，不一致风险 |",
      "| **17** | **低** | `handoff_gate` SOFT block 对 agent 不可见 | `phase.rs:746` | Agent 不知道为什么 advance 被 block（handoff incomplete） |"
    ]
  }
]
```

## Neighbors

```json
[
  {
    "task_id": "T-20260529090147-9df98d71",
    "title": "收口 v2.0 发布前状态并归档旧 VibeHub 结构",
    "active_capabilities": [],
    "shared_files": []
  }
]
```

## File: .vibehub/tasks/T-20260529145443-52bb4f48/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: T-20260529145443-52bb4f48
title: Fix AI Agent Skill/Prompt Quality Issues
mode: evidence_drive
phase: plan
phase_status: completed
created_at: 2026-05-29T14:54:43Z
created_by: vibehub
```

## File: .vibehub/tasks/T-20260529145443-52bb4f48/runs/R-20260529145443-224267fe/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: T-20260529145443-52bb4f48
run_id: R-20260529145443-224267fe
mode: evidence_drive
phase: plan
phase_status: completed
created_at: 2026-05-29T14:54:43Z
created_by: vibehub
baseline_commit: null
```

## File: .vibehub/rules/hard-rules.md

Reason: protocol hard rules

```text
# VibeHub Hard Rules

- Agent output is reported state only.
- Only VibeHub code updates canonical state transitions.
- Do not mark state.yaml completed from agent output.
- Distinguish hard_observed, agent_reported, inferred, and user_confirmed evidence.
- P0/P1 observability is best-effort and must not claim full runtime observation.
- Agents should read agent-view files and the current context pack, not the whole .vibehub directory.
- Keep changes scoped to the active task.

## CI/CD 改动纪律 (2026-05-21 从 8 轮返工中总结)

### 1. 本地先跑通再改 CI
- 任何 macOS CI 构建改动，**先在本地 macOS 验证**：
  `npm run tauri -- build --target aarch64-apple-darwin --bundles app`
- 本地能成功 `hdiutil create -fs APFS`，再改 GitHub Actions。
- CI 不是调试器，不要拿它当测试环境用。

### 2. 每次只改一个变量
- CI workflow 单次改动只改一项：构建方式 / 签名方式 / DMG 方式 分开验证。
- 改多个变量时无法定位失败原因，导致穷举试错。

### 3. 签名方案先问"要不要"
- **prerelease / 预发布**: 用 ad-hoc 签名 (`APPLE_SIGNING_IDENTITY="-"`)，不走 notarization。
- **正式发布**: 才需要 Developer ID 证书 + 公证流程。
- 不要默认启用全套 Apple 签名，先确认是否必要。

### 4. 优先用直接命令，少用 Action 封装
- `npm run tauri -- build` 直接 shell 命令 > `tauri-action` GitHub Action。
- 直接命令可以在本地完美复现，action 的传参行为是黑盒。
- 必须用 action 时，先查源码理解其内部命令拼接逻辑。
```

## Known Missing Context

- None declared.

## Stop Condition

Write output.md and return to VibeHub for validation.

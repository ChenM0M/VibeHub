# Context Pack: Align

Task: T-20260616062024-a8e8bdc2
Run: R-20260616062024-6b02149c
Phase: Align
Generated at: 2026-06-21T05:51:59Z
Source commit: d08b65e

## Instructions

Use this context only for the current phase.
Do not mark state.yaml completed.
Report files read, commands run, decisions made, and unresolved risks.

## Capability Output Schema

```json
{
  "required_fields": [
    "intent",
    "scope",
    "success_criteria",
    "non_goals"
  ],
  "optional_fields": [
    "stakeholders",
    "references"
  ],
  "produces": [
    "alignment_summary"
  ],
  "consumes": [],
  "parallel_safe": false,
  "custom": false
}
```

## Prior Outputs Summary

```json
[
  {
    "capability": "implement",
    "completed": [
      "`user_confirmed`: 用户要求一次性优化 VibeHub 面向 Agent 暴露的定位、能力边界、工具调用和流程约束问题。",
      "`hard_observed`: 无头 CLI 二进制名已从 `vibehub` 改为 `vibehub-cli`，帮助文本和 Agent-facing 命令示例统一使用 `vibehub-cli ...`。",
      "`hard_observed`: Agent 协议和适配器模板已加入一句话定位：VibeHub 是 coding agent 的 project memory、task router 和 workflow gatekeeper；Agent 做工程工作，VibeHub 追踪 state/context/gates/output/handoff。",
      "`hard_observed`: `next-action` 已支持 intent-aware 路由，能把 Agent/VibeHub 协议审视路由到 `vibehub-start`，把继续/同步路由到 `vibehub-sync`，把多 active task 校验风险路由到 `validate-task`。",
      "`hard_observed`: CLI 已新增 `next-action`、`validate-task`、`output-lint`，并对 `finish`、`advance`、`archive` 强制要求 `--confirmed-by-user`。",
      "`hard_observed`: Adapter generator 已移除旧的 `.vibehub/notes/status.md` 与 `phases/<phase>.output.md` lifecycle artifact 要求，统一要求写 active run 的 `outputs/output.md`。",
      "`hard_observed`: Codex、Claude Code、OpenCode 的 adapter/skills/commands 已同步生成；`adapter-status` 最终无 warnings。",
      "`hard_observed`: 第二轮自检修正了 `next-action` 的歧义路由：`继续当前状态` 仍路由到 `vibehub-sync`，但 `继续优化 Agent 协议和工具调用流程` 会优先识别为 VibeHub/Agent 协议改进工作并路由到 `vibehub-start`。"
    ],
    "full_ref": ".vibehub/tasks/T-20260609092907-7c439d16/runs/R-20260609092907-c9e04b56/outputs/output.md",
    "key_decisions": [
      "`agent_reported`: 将 VibeHub 的 Agent 心智模型收敛为“状态拥有者 + 路由器 + gatekeeper”，减少 Agent 把 VibeHub误当工程执行者或普通文档库的概率。",
      "`agent_reported`: 用 `vibehub-cli` 明确区分无头 Agent CLI 与桌面/Tauri app，避免裸 `vibehub` 命令撞到桌面入口。",
      "`agent_reported`: 对状态转换采取“CLI 可执行但必须显式用户确认”的约束，而不是只在文档里提醒，降低 Agent 忘记确认或过度自动推进的风险。",
      "`agent_reported`: 对多 active task 引入 `validate-task` 和协议提示，避免默认 `validate` 校验错误 current pointer。",
      "`agent_reported`: `next-action` 先看用户最新 intent，再看当前 phase/output 状态，避免新需求或元评估被误判成“当前 phase ready to validate”。",
      "`agent_reported`: 元层 Agent/VibeHub 协议意图优先于泛化的“继续/同步”词匹配；这保留了同步保守性，同时避免明确的协议优化请求被误吸成普通 refresh。",
      "## Diff Summary",
      "`hard_observed`: CLI 层改名并扩展命令面：`vibehub-cli` 帮助文本、`next-action`、`validate-task`、`output-lint`、状态转换确认 gate。",
      "`hard_observed`: Core 层新增 `next_action` 与 `output_lint` 模块，扩展 phase task-scoped validation，并修正 advance 后 agent-view 刷新时序。",
      "`hard_observed`: Agent adapter 模板、协议、constraints、skills、Claude/OpenCode commands 和 docs 已同步到新的命令名、操作循环、多任务校验指导、输出 lint 指导。",
      "`hard_observed`: Prompt 模板的 new-task/sync 文案已改为 `vibehub-cli`，并强调多意图拆分、继续/刷新先同步、agent-view 重新生成一致性。"
    ]
  }
]
```

## Neighbors

```json
[
  {
    "task_id": "T-20260531153015-3a283c0b",
    "title": "Redesign VibeHub project detail UI and project structure explorer",
    "active_capabilities": [],
    "shared_files": []
  },
  {
    "task_id": "T-20260609092907-7c439d16",
    "title": "Harden VibeHub agent protocol and CLI routing",
    "active_capabilities": [],
    "shared_files": []
  },
  {
    "task_id": "T-20260619044922-98d818ee",
    "title": "Display local Codex and OpenCode workspace usage insights",
    "active_capabilities": [
      "implement"
    ],
    "shared_files": []
  }
]
```

## File: .vibehub/tasks/T-20260616062024-a8e8bdc2/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: "T-20260616062024-a8e8bdc2"
title: "test cli dispatch"
mode: "guided_drive"
phase: "align"
phase_status: "active"
created_at: "2026-06-16T06:20:24Z"
created_by: vibehub
```

## File: .vibehub/tasks/T-20260616062024-a8e8bdc2/runs/R-20260616062024-6b02149c/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: "T-20260616062024-a8e8bdc2"
run_id: "R-20260616062024-6b02149c"
mode: "guided_drive"
phase: "align"
phase_status: "active"
created_at: "2026-06-16T06:20:24Z"
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

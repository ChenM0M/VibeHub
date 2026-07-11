# Context Pack: Align

Task: T-20260711062223-6f07315c
Run: R-20260711062223-10a76733
Phase: Align
Generated at: 2026-07-11T07:58:05Z
Source commit: d7ece57

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
    "capability": "align",
    "completed": [
      "`user_confirmed` Intent：把项目所有者的新想法、对原待定问题的回答及必要的专业修正，完整并入 `docs/vibehub-v3-redesign-plan.md`，形成可供后续实现的 V3 最高优先级方案。",
      "`user_confirmed` Scope：补强“绝对掌控感”的项目架构/文件结构/设计理念可视化；Task 与 Project 双层 UI；IDE 深入阅读与询问；MCP/CLI 分工；Agent 防偏；前端优先交付方法；Windows/macOS；kill-resume；legacy 入口；M5 多工具并行与 worktree。",
      "`agent_reported` Non-goals：本轮不实现 V3 代码、不修改现有前后端行为、不执行 finish/advance、不决定全部语言索引器与 Windows 路径预算等需原型/实机验证的细节。",
      "`hard_observed` 已将文档从 296 行扩展为 445 行，并重构为 Project 全局 + Task 叙事、MCP 原生控制面、Project Intelligence、Agent 分层防偏、worktree 并行与跨平台架构。",
      "`hard_observed` 现有代码已具备 `project_structure.rs`、`ProjectStructureExplorer.tsx`、项目文件打开能力；文档已将其定位为 M1/M3 的薄切片基础，而非从零建设。",
      "`agent_reported` Success criteria：文档逐项回答用户问题；CLI/MCP 不是二选一而是共享 core 的 adapters；M1 前端原型不制造无契约假后端；M5 包含 branch/scope/integration/conflict 生命周期；原待定问题有可执行结论；调查来源可追溯。",
      "`hard_observed` 已完成官方资料调查：MCP architecture/SDK/security、Codex MCP、Claude Code MCP、OpenCode MCP、Git worktree。"
    ],
    "full_ref": ".vibehub/tasks/T-20260711062223-6f07315c/runs/R-20260711062223-10a76733/outputs/output.md",
    "key_decisions": [
      "`agent_reported` 采用 Experience-first + Contract-first + Vertical Slice，而不是纯静态“前端全做完再猜后端”。",
      "`agent_reported` MCP 成为 Agent 首选控制面；CLI 继续服务人类、脚本、CI、排障与故障兜底；二者共同调用强类型 Application Service。",
      "`agent_reported` 第一版使用内置本地 stdio server；暂不引入常驻 HTTP daemon。",
      "`agent_reported` Agent 约束采用可发现性、schema、状态、权限、工作区、运行时与对抗测试七层防线；明确 MCP 无法单独约束宿主允许的任意代码写入。",
      "`user_confirmed` M5 worktree 并行值得做；方案将其升级为必要里程碑，并补齐 scope、branch、集成队列、冲突归属与异常回收。",
      "`agent_reported` kill-resume 使用零容忍语义 hard gates + Recovery Quality SLO，不用固定重读/重跑次数误判质量。",
      "`user_confirmed` legacy-v2 只保留最小只读入口，隔离 adapter，放到 M6，不能拖累 v3 domain。"
    ]
  }
]
```

## Neighbors

```json
[]
```

## File: .vibehub/tasks/T-20260711062223-6f07315c/task.yaml

Reason: active task metadata and goal

```text
schema_version: 1
kind: vibehub_task
task_id: "T-20260711062223-6f07315c"
title: "完善 VibeHub V3 重设计方案与 Agent 集成架构"
mode: "evidence_drive"
phase: "align"
phase_status: "active"
created_at: "2026-07-11T06:22:23Z"
created_by: vibehub
```

## File: .vibehub/tasks/T-20260711062223-6f07315c/runs/R-20260711062223-10a76733/run.yaml

Reason: active run metadata

```text
schema_version: 1
kind: vibehub_run
task_id: "T-20260711062223-6f07315c"
run_id: "R-20260711062223-10a76733"
mode: "evidence_drive"
phase: "align"
phase_status: "active"
created_at: "2026-07-11T06:22:23Z"
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

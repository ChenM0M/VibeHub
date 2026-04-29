# VibeHub Development Rules

## Product Philosophy

VibeHub is Observable YOLO for AI Coding.

The goal is not to build a workflow prison. The goal is to let AI agents move fast while keeping the process visible, recoverable, and auditable.

## Current Priority

Implement VibeHub v2 P0 first.

P0 includes:
1. `.vibehub` file protocol
2. YAML current pointers
3. task / run / session entities
4. agent-view files
5. context pack generator
6. handoff builder
7. review evidence writer
8. basic housekeeping
9. minimal validation

Do not implement P3/P4 features unless explicitly requested.

Do not implement:
- OpenCode C2
- Claude Runner
- Trellis import
- multi-agent orchestration
- MCP server
- runtime interception
- marketplace
- enterprise policy
- full adapter sync

## State Ownership Rule

Agent output is reported state only.

Only VibeHub code should update canonical state transitions.

Agents may suggest:
- phase completion
- task status changes
- review verdicts
- handoff notes

But VibeHub must validate before writing canonical state.

## Observability Rule

P0/P1 observability is best-effort.

Use:
- Git diff
- filesystem snapshots
- VibeHub-generated files
- Agent-reported summaries

Do not pretend runtime events are fully observed unless a runtime adapter exists.

Every review or evidence report should distinguish:
- hard_observed
- agent_reported
- inferred
- user_confirmed

## Context Rule

Agents should not read the whole `.vibehub` directory.

Agents should read:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- the current context pack
- relevant source files

## Development Rule

Each task must end with:
- changed files summary
- commands run
- tests run or reason not run
- unresolved risks
- handoff notes

## Scope Rule

Prefer small, vertical slices.

Do not rewrite unrelated VibeHub v1 functionality.

Keep existing project manager, tag launcher, AI gateway, and Git display working unless the task explicitly touches them.

## Implementation Style

Before coding:
1. Inspect existing architecture.
2. Identify minimal file locations.
3. Propose a small implementation plan.
4. Ask only blocking questions.

When coding:
1. Make the smallest working change.
2. Add tests where the repo already has test conventions.
3. Avoid new dependencies unless clearly justified.
4. Keep generated file formats deterministic.
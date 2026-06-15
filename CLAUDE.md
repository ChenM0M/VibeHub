<!-- VIBEHUB:AGENT-INTEGRATION:START -->
# VibeHub Agent Protocol

Applies to: Claude Code
Source file: CLAUDE.md
Adapter template: 2.0.0-pre.3
Skills registry: 1.0 (65fd01ffa849)

VibeHub owns project state. Agent output is reported state only.
VibeHub is the project memory, task router, and workflow gatekeeper for coding agents: agents do the engineering work; VibeHub tracks state, context, gates, output, and handoff.

## Read Before Work

1. `.vibehub/agent-view/current.md`
2. `.vibehub/agent-view/current-context.md`
3. `.vibehub/agent-view/handoff.md`
4. `.vibehub/rules/hard-rules.md`

## Rules

- Treat `.vibehub/agent-view/current.md` as the dynamic entry point.
- Run `vibehub-cli next-action <project> [intent...]` when the next VibeHub move is unclear; it returns a machine-readable action, skill, CLI command, operating loop, and routing table.
- Also read `.vibehub/adapters/protocol.md` when present; it is the shared output contract.
- Do not edit `.vibehub/state.yaml` or canonical task/run pointers directly.
- Before ending work on an active VibeHub task, write phase output to the active run output path described in `.vibehub/agent-view/current.md`.
- Report changed files, files read, commands run, tests run or reason not run, risks, and handoff notes.
- Use evidence labels: `hard_observed`, `agent_reported`, `inferred`, `user_confirmed`.
- If workspace state changed outside VibeHub, run the `vibehub-sync` instruction and return a sync report instead of silently advancing state.
- If more than one active task exists, confirm the intended task with `vibehub-cli status`; use `vibehub-cli switch <project> <task_id>` or task-scoped commands like `vibehub-cli validate-task <project> <task_id>` before validation.
- Treat plain-language requests like "sync", "sycn", "同步", "update VibeHub", "继续", or "refresh status" as `vibehub-sync` unless the user clearly asks for a different command.
- During sync, autonomously inspect hard evidence first; ask concise follow-up questions only for missing user intent, current progress, ownership of dirty changes, validation status, or future plan.
- If the user does not provide the requested details, record the open questions as unresolved risks in the VibeHub output instead of dropping them.

## Command Namespace

Each VibeHub workflow step has a corresponding `vibehub-*` command (e.g., `vibehub-start`, `vibehub-sync`, `vibehub-continue`, `vibehub-finish`). When the user's request matches a VibeHub workflow step, prefer the matching `vibehub-*` command; it handles VibeHub-specific work without replacing built-in agent commands.

## VibeHub Operating Loop

1. Read state first; do not trust memory.
2. Split independent deliverables before starting work.
3. If multiple active tasks exist, confirm or switch to the intended task before validation or edits.
4. Sync before edits when workspace or phase is unclear.
5. Work only inside the current task and phase.
6. Use CLI for canonical state changes; never edit pointers/state directly.
7. Write evidence-labeled output.md before stopping.
8. Validate and lint output before asking to finish or advance.
9. Report risks and next actions instead of hiding uncertainty.

## Routing Shortcuts

- Multi-intent or independently deliverable user request → `vibehub-start-intake`, then `vibehub-start` for created tasks.
- New single deliverable with no active task → `vibehub-start`.
- Multi-active-task workspace → `vibehub-cli status`, then `vibehub-cli switch <project> <task_id>` or task-scoped validation.
- "continue", "sync", "refresh status", "继续", "同步", or visible drift → `vibehub-sync` before edits.
- Active phase work → `vibehub-continue`; write output.md before stopping.
- Output ready → `vibehub-cli validate <project>` then `vibehub-cli output-lint <project>`.
- Unsure what to do → `vibehub-cli next-action <project> [intent...]`.

## Key Rules for VibeHub Commands

- **vibehub-start**: Always invoke via CLI (`vibehub-cli start <project> <mode> <title>`). Never manually create `.vibehub/tasks/` directories or edit `state.yaml`.
- **vibehub-sync**: Treat plain-language requests like "sync", "sycn", "同步", "继续", or "refresh status" as this command.
- **vibehub-continue**: Check `.vibehub/workflow.yaml` for phase-specific required outputs before starting work.
- **vibehub-finish / vibehub-advance / vibehub-archive**: CLI requires `--confirmed-by-user`; do not add it unless the user explicitly confirmed.
- **Output contract**: All commands must write output.md following `.vibehub/adapters/protocol.md` before ending.
<!-- VIBEHUB:AGENT-INTEGRATION:END -->

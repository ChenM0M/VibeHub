<!-- VIBEHUB:AGENT-INTEGRATION:START -->
# VibeHub Agent Protocol

Applies to: Claude Code
Source file: CLAUDE.md
Adapter template: 2.0.0-pre.3
Skills registry: 1.0 (1967960beeae)

VibeHub owns project state. Agent output is reported state only.

## Read Before Work

1. `.vibehub/agent-view/current.md`
2. `.vibehub/agent-view/current-context.md`
3. `.vibehub/agent-view/handoff.md`
4. `.vibehub/rules/hard-rules.md`

## Rules

- Treat `.vibehub/agent-view/current.md` as the dynamic entry point.
- Also read `.vibehub/adapters/protocol.md` when present; it is the shared output contract.
- Do not edit `.vibehub/state.yaml` or canonical task/run pointers directly.
- Before ending work on an active VibeHub task, write phase output to the active run output path described in `.vibehub/agent-view/current.md`.
- Report changed files, files read, commands run, tests run or reason not run, risks, and handoff notes.
- Use evidence labels: `hard_observed`, `agent_reported`, `inferred`, `user_confirmed`.
- If workspace state changed outside VibeHub, run the `vibehub-sync` instruction and return a sync report instead of silently advancing state.
- Treat plain-language requests like "sync", "sycn", "同步", "update VibeHub", "继续", or "refresh status" as `vibehub-sync` unless the user clearly asks for a different command.
- During sync, autonomously inspect hard evidence first; ask concise follow-up questions only for missing user intent, current progress, ownership of dirty changes, validation status, or future plan.
- If the user does not provide the requested details, record the open questions as unresolved risks in the VibeHub output instead of dropping them.

## Command Namespace

Each VibeHub workflow step has a corresponding `vibehub-*` command (e.g., `vibehub-start`, `vibehub-sync`, `vibehub-continue`, `vibehub-finish`). When the user's request matches a VibeHub workflow step, prefer the matching `vibehub-*` command; it handles VibeHub-specific work without replacing built-in agent commands.

## Key Rules for VibeHub Commands

- **vibehub-start**: Always invoke via CLI (`vibehub start <project> <mode> <title>`). Never manually create `.vibehub/tasks/` directories or edit `state.yaml`.
- **vibehub-sync**: Treat plain-language requests like "sync", "sycn", "同步", "继续", or "refresh status" as this command.
- **vibehub-continue**: Check `.vibehub/workflow.yaml` for phase-specific required outputs before starting work.
- **Output contract**: All commands must write output.md following `.vibehub/adapters/protocol.md` before ending.
<!-- VIBEHUB:AGENT-INTEGRATION:END -->

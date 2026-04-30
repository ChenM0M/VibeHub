<!-- VIBEHUB:AGENT-INTEGRATION:START -->
# VibeHub Agent Protocol

Applies to: Codex
Source file: AGENTS.md

VibeHub owns project state. Agent output is reported state only.

## Read Before Work

1. `.vibehub/agent-view/current.md`
2. `.vibehub/agent-view/current-context.md`
3. `.vibehub/agent-view/handoff.md`
4. `.vibehub/rules/hard-rules.md`

## Rules

- Treat `.vibehub/agent-view/current.md` as the dynamic entry point.
- Do not edit `.vibehub/state.yaml` or canonical task/run pointers directly.
- Report changed files, files read, commands run, tests run or reason not run, risks, and handoff notes.
- Use evidence labels: `hard_observed`, `agent_reported`, `inferred`, `user_confirmed`.
- If workspace state changed outside VibeHub, run the `vibehub-sync` instruction and return a sync report instead of silently advancing state.

## Command Namespace

Use generated `vibehub-*` commands where supported. They describe VibeHub-specific work without replacing built-in agent commands.
<!-- VIBEHUB:AGENT-INTEGRATION:END -->

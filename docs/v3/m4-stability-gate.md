# M4 Stability Gate

Run `npm run v3:m4:stability`. The command writes the machine-readable result to `target/m4-stability-report.json`.

The automated gate covers deterministic replay, duplicate retry, Plan DAG rejection, Criterion completion truth, two remediation cycles, confirmation invalidation, three host recovery loops, contracts, MCP compatibility, and the production TypeScript build.

The threshold is frozen before execution: 100 replay iterations, 10 recovery loops for each of Codex, OpenCode, and Claude Code, four kill-resume semantic hard gates, and zero open P0/P1 defects in data loss, wrong-task identity, false completion, or confirmation authenticity.

`m5_entry_gate` remains `closed` until both native macOS and native Windows smoke evidence exist and the project owner explicitly approves the V3-managed M5 shadow run. Automated success never substitutes for either gate.

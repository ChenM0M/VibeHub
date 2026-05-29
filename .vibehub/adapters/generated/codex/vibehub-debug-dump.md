---
name: vibehub-debug-dump
description: "Export a redacted diagnostic bundle for cross-tool and cross-machine debugging. Use for VibeHub workflow step: vibehub-debug-dump."
---

# vibehub-debug-dump

中文: 导出脱敏调试包。
English: Export a redacted diagnostic bundle for cross-tool and cross-machine debugging.

Invocation input: <project_root> [include_events] [include_packs] [redact_secrets]

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

Task:
Export a redacted VibeHub debug bundle. Prefer `debug-dump <project_path>` or `vibehub debug-dump <project_path>` when the CLI is available.



Output requirements:
- write the active run phase output before ending work:
  `.vibehub/tasks/<task_id>/runs/<run_id>/outputs/output.md`
- changed files, if any
- files read
- commands run
- tests run, or reason not run
- evidence labels: `hard_observed`, `agent_reported`, `inferred`, `user_confirmed`
- unresolved risks
- handoff notes or recommended VibeHub action

Constraints:
- Do not edit `.vibehub/state.yaml` or canonical task/run pointers directly.
- Do not claim runtime observation unless a runtime adapter captured it.
- If state is stale or drifted, report it and recommend VibeHub sync/recover instead of silently advancing state.

Registry contract:
- name: `vibehub-debug-dump`
- args: `<project_root> [include_events] [include_packs] [redact_secrets]`
- returns: `skill_response_schema_v1`
- callable_by: main-agent
- side_effects: writes_debug_artifact
- idempotent: false

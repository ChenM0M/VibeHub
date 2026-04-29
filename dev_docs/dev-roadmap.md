# VibeHub v2 P0 Development Roadmap

## Goal

Build the smallest useful VibeHub v2 kernel that supports Observable YOLO with the 4+1 workflow.

## Non-goals for P0

Do not implement:
- OpenCode C2
- Claude Runner
- Trellis import
- multi-agent orchestration
- runtime observation
- MCP server
- full adapter marketplace
- enterprise policy

## Milestone 1 — `.vibehub init`

Goal:
Create the minimal `.vibehub` directory structure for an existing project.

Deliverables:
- project.yaml
- state.yaml
- workflow.yaml
- housekeeping.yaml
- rules/
- agent-view/
- tasks/
- research/
- journal/
- adapters/

Done when:
- A project can be initialized.
- Existing VibeHub v1 behavior is not broken.
- Defaults match docs/vibehub-v2-r10.md.

## Milestone 2 — Current YAML pointers

Goal:
Implement cross-platform current task/run pointers.

Deliverables:
- `.vibehub/tasks/current`
- `.vibehub/tasks/{task_id}/runs/current`
- parser functions
- validation errors for broken pointers

Done when:
- No symlink is required.
- VibeHub can resolve active task and active run.

## Milestone 3 — Context Pack Generator

Goal:
Generate context packs from declarative context YAML.

Deliverables:
- read `context/{phase}.yaml`
- generate `context-packs/{phase}.md`
- generate `context-packs/{phase}.manifest.yaml`
- deny secret files
- record missing optional/required files

Done when:
- Agent only needs to read generated context pack.
- Manifest explains included/missing/excluded files.

## Milestone 4 — Agent View

Goal:
Generate the simplified Agent entrypoint files.

Deliverables:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`

Done when:
- Codex can start from `agent-view/current.md` instead of reading all `.vibehub`.

## Milestone 5 — Handoff Builder

Goal:
Build handoff from agent output and Git evidence.

Deliverables:
- parse handoff section from output.md
- generate agent-view/handoff.md
- mark handoff completeness

Done when:
- Session-to-session continuity works without long chat history.

## Milestone 6 — Review Evidence

Goal:
Generate minimal review evidence.

Deliverables:
- changed-files.txt
- diff.patch
- review.md
- evidence grades

Done when:
- Review can distinguish hard_observed / agent_reported / inferred.

## Milestone 7 — Minimal Cockpit UI

Goal:
Add the first GUI cockpit view.

Deliverables:
- current task
- current phase
- mode
- Git status
- context status
- handoff status
- Continue / Review / Recover actions

Done when:
- User can understand current state without opening files manually.
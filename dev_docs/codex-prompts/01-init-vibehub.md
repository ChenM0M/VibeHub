# Codex Task: Implement `.vibehub init`

Read first:
- AGENTS.md
- docs/vibehub-v2-r10.md
- docs/dev-roadmap.md

Goal:
Implement Milestone 1: minimal `.vibehub init`.

Scope:
Create the backend logic needed to initialize a project with the minimal `.vibehub` structure.

Required output structure:

.vibehub/
├── project.yaml
├── state.yaml
├── workflow.yaml
├── housekeeping.yaml
├── agent-view/
│   ├── current.md
│   ├── current-context.md
│   └── handoff.md
├── rules/
│   ├── phase-rules.yaml
│   ├── research-triggers.yaml
│   ├── autonomy.yaml
│   ├── review.yaml
│   ├── loop-detection.yaml
│   ├── hard-rules.md
│   └── preferences.yaml
├── tasks/
├── research/
│   ├── current/
│   └── archive/
├── journal/
│   └── index.md
└── adapters/
    ├── sync-state.yaml
    ├── templates/
    └── generated/

Implementation constraints:
- Do not implement GUI cockpit yet.
- Do not implement adapters beyond placeholder files.
- Do not implement runtime observation.
- Do not use symlinks.
- Do not break existing VibeHub v1 project management.
- Avoid new dependencies unless absolutely necessary.

Behavior:
- If `.vibehub` does not exist, create it.
- If `.vibehub` exists, do not overwrite user files by default.
- Return a clear result:
  - created files
  - skipped existing files
  - errors

Tests:
- Add tests if the repository has existing test conventions.
- At minimum, make the init logic deterministic and easy to test.

Done when:
- A project can be initialized with the minimal `.vibehub` structure.
- Default YAML/Markdown files are valid and match r10.
- Existing v1 features are not modified except for adding an entry point if needed.

Final response:
- changed files
- commands run
- tests run or reason not run
- unresolved risks
- handoff notes for next task
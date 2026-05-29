# VibeHub M7 UI UX / IA Spec

> Status: Draft v2
> Date: 2026-05-28
> Source: user sketch `/Users/chenm0m/Downloads/无标题-2025-03-15-1718.excalidraw.png` + user narration on 2026-05-28
> Baseline refs: §5, §23, §24

## 1. Scope Correction

M7 is the UI refactor. M7a is only the first implementation slice of that refactor.

The M7 direction is **not** "add Kanban to the old Dashboard." The old Dashboard layout should be abandoned as the main experience. Existing information remains valuable, but it must be reorganized into:

- a Kanban-style project detail main view;
- clickable task / capability / phase detail pages;
- activity history and JSON renderers;
- project-level settings;
- project structure and archive views.

The UI remains a display / reader / prompt surface. It must not become the required place to operate VibeHub. Real workflow operations still go through the agent and VibeHub event/projection model, except explicitly allowed project-level settings writes.

## 2. User Intent Restatement

The user wants the VibeHub project page to answer these questions first:

1. What project am I looking at?
2. What changed most recently?
3. What VibeHub activity happened most recently?
4. Which tasks are active?
5. Which phase/capability is currently running inside each task?
6. Where can I click to see the original requirement, full phase process, received package, outgoing package, event history, and rendered JSON?
7. What does the whole project structure look like?
8. Which tasks were completed or cancelled, and can I still inspect their full history?

The main page should feel like a project command map, not an operational status dashboard.

## 3. Primary Page Layout

```diagram
╭────────────────────────────────────────────────────────────────────────────╮
│ Project Name / VibeHub                                             Settings │
│ Recent commit: <branch - summary>                                           │
│ Recent activity: <Task X - phase Y completed / local new commit / ...>       │
│                                                                            │
│ Active Tasks                                                               │
│ ╭─ 修复  Specific requirement summary -------------------- ... phase strip ╮ │
│ │        short task intent text                         ... [active phase] │ │
│ ╰────────────────────────────────────────────────────────────────────────╯ │
│ ╭─ 特性添加 Specific requirement summary ---------------- ... phase strip ╮ │
│ │        short task intent text                         ... [active phase] │ │
│ ╰────────────────────────────────────────────────────────────────────────╯ │
│                                                                            │
│ Project Structure                                Archived                   │
│ ╭──────────────────────────────────────╮        ╭────────────────────────╮ │
│ │ module interaction tree / canvas     │        │ ✓ 特性添加 summary      │ │
│ │                                      │        │ ✕ 特性添加 summary      │ │
│ │                                      │        │ ✓ 特性添加 summary      │ │
│ ╰──────────────────────────────────────╯        ╰────────────────────────╯ │
│ ╭──────────────────────────────────────╮                                    │
│ │ file directory structure             │                                    │
│ ╰──────────────────────────────────────╯                                    │
╰────────────────────────────────────────────────────────────────────────────╯
```

Layout notes:

- The first viewport prioritizes project identity, recent activity, and active tasks.
- Active tasks should be shown vertically and should stay visible as much as possible.
- Archived content and project structure are secondary but still available on the same project detail page.
- When content overflows, prefer progressive disclosure or a detail page over shrinking text until it becomes unreadable.

## 4. Header Region

### 4.1 Project Title

The top-left title is the current project name. It should be visually dominant enough to anchor the page.

### 4.2 Settings Button

The top-right settings button opens project-level VibeHub settings.

Expected settings:

- VibeHub output / constraint language for the current project.
- Project remote URL, such as GitHub/GitLab repository URL.
- Future project-level VibeHub options.

Open question:

- `inferred`: Because project settings are writes, define which settings are UI-allowed exceptions to INV-6. Language and remote URL look like project configuration, not workflow state, and should likely be allowed.

### 4.3 Recent Commit

Shows the most recent local commit summary or branch + commit summary.

Interaction:

- If a remote URL is configured and the commit can be mapped to an online URL, clicking opens the online commit page.
- If no remote is configured, the UI should still show local commit details but not pretend an online link exists.

Open questions:

- `inferred`: Need exact remote URL normalization rules for GitHub, GitLab, and other hosts.
- `inferred`: Need fallback behavior for detached HEAD, no commits, or multiple remotes.

### 4.4 Recent Activity

Shows a one-line summary of latest VibeHub activity, such as:

- `Task XX - phase/capability XX completed`
- `local new commit`
- `context pack rebuilt`
- `schema validation failed`

Interactions:

- Clicking the recent activity label opens an activity modal / page with historical VibeHub progress.
- The activity view supports filtering by task and by phase/capability.
- Clicking a specific activity target jumps to the relevant task and phase/capability position on the main board.
- The target task frame and the target phase/capability frame should receive a visible focus/highlight effect.

This belongs mainly to M7b, but M7a should leave the anchor points and focus model ready.

## 5. Active Task Cards

### 5.1 Task Card Content

Each active task card starts with:

- a short task type label, ideally 2-4 Chinese characters and at most about 6 when necessary, such as `修复`, `特性添加`, `需求调查`;
- the concrete requirement summary;
- a compact phase/capability process strip.

Example:

```text
修复      具体的需求简述......
          ... · 阶段A -> 阶段B -> [阶段C active] -> 阶段D · ...
```

Open questions:

- `inferred`: Decide whether the short label is agent-generated, user-editable, or derived from task title/category.
- `inferred`: Decide whether the label should be stored in task metadata or computed in the frontend.

### 5.2 Phase / Capability Strip

The strip shows the task's process. The active phase/capability has a clearly stronger border or highlight.

If the full process is too long:

- show the currently active item;
- show a small number of neighbors before and after it;
- preserve the first/last side with ellipsis markers where needed;
- do not wrap into an unreadable multi-line strip by default.

Click behavior:

- Clicking the task label / requirement area opens the original requirement detail.
- Clicking the strip's blank/process area opens the full phase/capability process detail.
- Clicking a specific phase/capability opens that phase/capability's current detail.

### 5.3 Highlight / Focus Behavior

When navigation comes from recent activity, commit relation, archive, search, or another deep link:

- scroll or move the board to the related task;
- highlight the task frame;
- highlight the related phase/capability frame if known;
- use a temporary visible effect, such as pulse, glow, or border emphasis.

## 6. Detail Surfaces

M7 detail surfaces should replace the old Dashboard cards. The same information may exist, but it should be accessed from the entity the user clicked.

### 6.1 Task Detail

Opened from the task label / requirement area.

Should include:

- original requirement / user intent;
- short label and full title;
- task/run metadata;
- active/completed/cancelled status;
- full capability/phase process;
- key warnings and unresolved risks;
- related commits and files when available;
- links into activity timeline and archive history.

### 6.2 Process Detail

Opened from the empty/process area of the phase strip.

Should include:

- full phase/capability sequence;
- information transfer path between phases/capabilities;
- received packages and outgoing packages;
- handoff/output/context relationships;
- rendered JSON / structured package viewer.

### 6.3 Phase / Capability Detail

Opened from a specific phase/capability pill.

Should include:

- current progress and status;
- context pack received on entry;
- output/handoff expected on exit;
- validation/schema state;
- related events;
- related files, ownership, and conflicts;
- JSON renderer for package-shaped content.

### 6.4 Activity Modal / Page

Opened from recent activity.

Should include:

- chronological VibeHub events;
- filters by task, phase/capability, and event type;
- clickable event targets;
- jump-to-board focus behavior.

This is primarily M7b.

### 6.5 Project Settings

Opened from top-right settings.

Should include:

- project VibeHub language;
- project remote URL;
- future project-scoped VibeHub settings;
- adapter / AI instruction settings if they remain project-level maintenance.

Settings should feel like project configuration, not workflow operation.

## 7. Project Structure Region

The lower project structure area has two linked views:

1. module interaction tree / graph / canvas;
2. file directory structure.

### 7.1 Main Page Preview

On the main page:

- show a compact module interaction tree or project map;
- show a compact directory structure;
- allow opening a larger detail view / canvas.

### 7.2 Full Project Structure Detail

The detailed project structure page uses a split layout:

- left: full interaction graph/canvas;
- right: full directory tree;
- clicking a graph node highlights the corresponding directory/file area;
- clicking a directory/file highlights related graph nodes;
- right-click or context menu can offer "open" and "reveal in file manager."

Lower priority:

- showing historical development records for a module/file from the graph.

Open questions:

- `inferred`: Need a backend/project-analysis source for module graph data.
- `inferred`: Need to decide whether M7 ships a manual/placeholder structure view first, then upgrades to generated graph later.

## 8. Archive Region

The archived region shows completed and cancelled tasks.

Card content:

- short task label, such as `特性添加`;
- concrete requirement summary;
- status marker at the top-right:
  - check mark for completed;
  - cross for cancelled / interrupted / no longer needed.

Interactions:

- clicking a card opens full archived task detail;
- clicking the archive frame or bottom area opens the complete archive page;
- archived detail preserves the task's full process, history, events, outputs, handoffs, and cancellation/completion reason.

This is primarily M7d.

## 9. Legacy Dashboard Module Mapping

The old Dashboard layout should not remain as the main page. Its data should be relocated:

| Old module | New location |
|---|---|
| Phase flow | Task card phase/capability strip + Process Detail |
| Git summary | Header recent commit + Task Detail related commits/files |
| File health cards | Phase/Capability Detail and Process Detail |
| Evidence map | Task Detail / Phase Detail evidence section |
| Context pack | Phase/Capability Detail, rendered as structured content |
| Agent output | Phase/Capability Detail and Task Detail outputs |
| Handoff | Phase/Capability Detail and Task Detail process history |
| Review evidence | Phase/Capability Detail / Process Detail |
| Research status | Task Detail and current phase/capability detail |
| Read-only recommended actions | Contextual prompt suggestions in detail pages |
| AI Instructions / adapters | Project Settings, with main-page warning only if attention needed |
| Warnings | Header/global health + task/card badges + detail pages |
| Neighbor tasks | Active task board, shown as task relationships/conflict badges |
| Event stream | Recent Activity + Activity modal/page |

## 10. M7 Implementation Slices

### M7a - Main IA Shell

Replace the old Dashboard main page with the new Kanban-first project detail shell:

- header with project name, recent commit, recent activity, settings entry;
- active task cards with short label, requirement summary, phase/capability strip;
- visible active phase/capability emphasis;
- basic click routing to placeholder detail surfaces;
- focus/highlight model for task and phase/capability targets;
- archive and project-structure placeholders in the correct page positions.

### M7b - Activity + JSON Renderer

- Recent activity modal/page.
- Event filtering by task / phase/capability / event type.
- Click event target to jump and highlight board position.
- Structured JSON renderer for context pack, handoff, output, sync report, and events.

### M7c - Prompt Generator + Settings Language

- Prompt generator modals and templates.
- Project language setting and template language behavior.
- Dangerous-action confirmation prompts.

### M7d - Archive Detail

- Completed/cancelled archive region.
- Archived task detail page.
- Completion/cancellation status markers and preserved history.

### M7e - Project Structure Explorer

New lower-priority UI slice introduced by this UX spec:

- compact project structure preview on main page;
- full graph + directory split detail;
- graph/tree linked highlighting;
- open/reveal file actions;
- optional historical module/file development record.

## 11. Acceptance Criteria

- The main page no longer presents the old Dashboard card grid as the primary UI.
- The user can understand active work by reading task cards first.
- Active phase/capability is visually obvious.
- Clicking task / process / phase/capability opens the appropriate detail surface.
- Recent activity can lead the user back to the related task and phase/capability.
- Completed/cancelled tasks remain inspectable through archive.
- Project structure has an intentional place in the IA, even if graph generation ships later.
- UI remains read-first and prompt-first; workflow mutation still goes through agent/VibeHub events.

## 12. Open Questions

| ID | Question | Suggested owner |
|---|---|---|
| UI-Q1 | How is the 2-4 character task label generated or edited? | M7a |
| UI-Q2 | What threshold triggers active-task collapse/pagination? | M7a |
| UI-Q3 | Should detail surfaces be route pages, drawers, or modal pages? | M7a |
| UI-Q4 | Which project settings are allowed UI writes under INV-6 exceptions? | M7a/M7c |
| UI-Q5 | What remote providers must recent commit links support in v1? | M7a |
| UI-Q6 | What is the first backend source for project structure graph data? | M7e |
| UI-Q7 | Should module/file history be shown in M7e v1 or deferred? | M7e |

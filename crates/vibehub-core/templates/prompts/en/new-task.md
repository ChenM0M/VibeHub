# VibeHub New Task Prompt

Project: `{{project_root}}`
Current task: `{{task_id}}` - {{task_title}}
Current run: `{{run_id}}`
Current mode: `{{mode}}`

Please create VibeHub task state from the request below.

## How
If the request contains multiple independently deliverable goals, run `vibehub start-intake {{project_root}} --stdin` with task drafts first. For a single deliverable, run:
```
vibehub start {{project_root}} {{mode}} "<title>"
```
Or use the cockpit "Start Task" button. Do NOT manually create `.vibehub/tasks/` files — the CLI sets up pointers, state, and context automatically.

## Request
<paste the user's new requirement here>

## Output
After creating the task, read `.vibehub/agent-view/current.md` and continue the first phase. Follow `.vibehub/adapters/protocol.md` and write output.md with all required sections before ending.

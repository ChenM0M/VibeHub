# VibeHub New Task Prompt

Project: `{{project_root}}`
Current task: `{{task_id}}` - {{task_title}}
Current run: `{{run_id}}`
Current mode: `{{mode}}`

Please create a new VibeHub task from the request below.

## How
Run the VibeHub CLI:
```
vibehub start {{project_root}} {{mode}} "<title>"
```
Or use the cockpit "Start Task" button. Do NOT manually create `.vibehub/tasks/` files — the CLI sets up pointers, state, and context automatically.

## Request
<paste the user's new requirement here>

## Output
After creating the task, follow the phase output contract in `.vibehub/adapters/protocol.md`. Write output.md with all required sections before ending.

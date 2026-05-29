# VibeHub File Ownership

> Status: Draft v1
> Date: 2026-05-28
> Baseline refs: §4 Journey C, §8.3, §26

## 1. Goal

File ownership helps VibeHub decide whether dirty git files belong to the active task, another task, multiple tasks, or no known task. It is read-only over git state and never stages, commits, checks out, or rewrites files, preserving INV-8.

## 2. Data Model

Ownership is time-windowed and many-to-many:

```yaml
schema_version: "1.0"
file_ownership:
  - file_path: "src-tauri/src/vibehub/events.rs"
    task_id: "T-20260527-capability"
    run_id: "R-20260527-main"
    capability: "implement"
    source: "event_projection"
    confidence: 0.92
    active_from_event: "evt-1716800000000-0001"
    active_to_event: null
    first_seen_at: "2026-05-27T12:00:00Z"
    last_seen_at: "2026-05-28T01:00:00Z"
```

Fields:

| Field | Required | Type | Notes |
|---|---|---|---|
| `file_path` | yes | string | repo-relative normalized path |
| `task_id` | yes | string | owner task |
| `run_id` | no | string | run where ownership was observed |
| `capability` | no | string | capability that touched or claimed the file |
| `source` | yes | enum | `event_projection`, `git_history`, `user_declared`, `agent_inferred` |
| `confidence` | yes | number | `0.0..1.0` |
| `active_from_event` | no | string | event id where ownership began |
| `active_to_event` | no | string | event id where ownership ended |
| `first_seen_at` | yes | datetime | observation start |
| `last_seen_at` | yes | datetime | latest observation |

The same `file_path` may appear multiple times for different tasks and time windows. A new ownership record does not delete old records.

## 3. Sources

Ownership evidence is derived from:

- `DiffObserved.files`
- `PlanDrafted.scope` and planned `affected_files`
- `ImplementOutput.changed_files`
- `vibehub-record { scope_files: [...] }`
- read-only git history for recent commits
- user answers to conflict prompts

Priority order:

1. `user_declared`
2. `event_projection`
3. `agent_inferred`
4. `git_history`

Higher-priority evidence can supersede lower-priority evidence for future windows, but it never deletes old records.

## 4. Automatic Decision Algorithm

Inputs:

- `D`: dirty or changed files from read-only git status/diff.
- `O(file)`: active ownership records for a file.
- `task_id_active`: current task.

For each file:

```text
owners = active ownership records matching file by exact path or directory prefix

if owners is empty:
  classify file as unowned
else if exactly one owner task:
  classify file as owned_by(owner.task_id)
else:
  classify file as multi_owned(owners)
```

Aggregate:

```text
if all files owned by active task:
  action = assign_to_active_task
elif no files have owners:
  action = ask_user_unowned
elif files split between active and other tasks:
  action = ask_user_partial_overlap
elif any file has multiple active owners:
  action = ask_user_multi_owner
else:
  action = ask_user_all_drift
```

## 5. Required Actions

| Case | Condition | Action |
|---|---|---|
| No intersection | `owners` empty for all dirty files | Ask whether to create a new task, attach to active task, or ignore |
| Single intersection | all dirty files map to one task | Assign to that task in projection and report confidence |
| Partial overlap | some files active-task-owned, some unowned/other-owned | Ask user to split ownership |
| Multiple intersections | one or more files map to multiple tasks | Ask user which task owns each ambiguous file |
| All drift | dirty files map only to non-active tasks | Ask whether to switch task or attach as drift |

## 6. Prompt Templates

### 6.1 No Intersection

```text
我看到这些改动目前不属于任何已知 VibeHub task：

{file_list}

请选择处理方式：
1. 开新 task，并把这些文件作为 implement 产出
2. 归入当前 task {active_task_id}
3. 部分归入 / 部分开新 task
4. 暂时只记录为 drift，不推进状态
```

### 6.2 Partial Overlap

```text
这些文件看起来分属不同范围：

当前 task 文件：
{active_files}

疑似 drift / 其它 task 文件：
{drift_files}

请说明哪些文件归入当前 task，哪些应另开或切换 task。
```

### 6.3 Multi Owner

```text
以下文件同时命中多个 task 的 ownership：

{ambiguous_table}

请为每个文件指定主要归属 task。VibeHub 只记录 ownership，不会修改 git。
```

### 6.4 All Drift

```text
当前改动没有命中 active task {active_task_id}，但命中了其它 task：

{task_file_table}

是否切换到对应 task 继续，还是把这些改动登记为当前 task 的新增范围？
```

## 7. Events and Projection

Recommended event additions for M4/M5:

- `FileOwnershipObserved { file_path, task_id, source, confidence }`
- `FileOwnershipCorrected { file_path, from_task_id, to_task_id, reason }`
- `DriftDetected { files, suggested_action }`

Projection rules:

- Active ownership is the latest non-expired record per `(file_path, task_id)`.
- `user_declared` records keep confidence `1.0`.
- Files can have multiple active owners until a correction event resolves ambiguity.

## 8. INV-8 Boundary

VibeHub may run read-only git commands such as status, diff, log, and show. It must not run git write commands (`add`, `commit`, `checkout`, `reset`, `tag`, `push`, `branch -D`) as part of ownership detection. Agents may choose to run git commands separately, but that is outside VibeHub ownership logic.

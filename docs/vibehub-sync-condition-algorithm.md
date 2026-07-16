# VibeHub Sync Condition Algorithm

> Status: Draft v1
> Date: 2026-05-28
> Baseline refs: §6, §20, §26

## 1. Goal

This document formalizes the condition detector from baseline §6.2. The detector chooses one of three sync levels:

- `quick`: incremental reconciliation
- `deep`: rebuild context pack and projection from hard evidence
- `rebuild`: full recovery when the baseline is unusable or schema changes require it

Every level must still produce a complete, self-consistent Context Pack and state snapshot as required by baseline §6.3.

## 2. Inputs

| Input | Type | Source |
|---|---|---|
| `now` | timestamp | system clock |
| `last_sync_at` | timestamp or null | latest `SyncCompleted` event or state metadata |
| `last_sync_git_head` | string or null | freshness header from last sync |
| `current_git_head` | string or null | read-only git command |
| `diff_files` | set<string> | read-only git status/diff |
| `task_files` | set<string> | task ownership / prior changed file projection |
| `schema_version_current` | string | current runtime schema |
| `schema_version_recorded` | string or null | VibeHub state / event stream |
| `baseline_exists` | bool | required VibeHub files exist |

VibeHub reads git but never writes git, preserving baseline §26 and INV-8.

## 3. Signal Formulas

### 3.1 `Delta t`

`Delta t = now - last_sync_at`

Boundary rules:

- If `last_sync_at` is missing, `Delta t = infinity`.
- If `last_sync_at > now` due clock skew, clamp `Delta t = 0` and emit a warning.
- Units are seconds internally; policy may expose minutes/hours.

### 3.2 `Delta head`

`Delta head = current_git_head != last_sync_git_head`

Boundary rules:

- If either head is missing, `Delta head = unknown`.
- `unknown` is treated as deep sync unless rebuild conditions are already true.
- Detached HEAD is valid as long as the hash is readable.

### 3.3 `Delta overlap`

Let:

- `D = diff_files`
- `T = task_files`
- `owned = D intersection T`

Formula:

`Delta overlap = |owned| / |D|`

Boundary rules:

- If `|D| = 0`, `Delta overlap = 1.0`.
- If `|D| > 0` and `|T| = 0`, `Delta overlap = 0.0`.
- Paths are normalized to repo-relative POSIX-style strings before comparison.
- Directory ownership expands by prefix match, but file ownership uses exact match.

Interpretation:

- High overlap means dirty work likely belongs to the active task.
- Low overlap means drift likely exists and a deeper scan is needed.

## 4. Thresholds

Defaults:

| Signal | Quick | Deep | Rebuild |
|---|---:|---:|---:|
| `Delta t` | `< 1h` | `>= 24h` | missing baseline |
| `Delta head` | `false` | `true` or `unknown` | git head unreadable |
| `Delta overlap` | `>= 0.80` | `< 0.50` | ownership data unavailable and dirty files exist |
| schema | equal | compatible minor drift | major mismatch / unsupported |

Configurable in future `policy.yaml`:

```yaml
sync:
  quick_max_age_seconds: 3600
  deep_age_seconds: 86400
  quick_overlap_min: 0.80
  deep_overlap_max: 0.50
  force_rebuild_on_schema_major_mismatch: true
```

Non-configurable invariants:

- Missing baseline triggers `rebuild`.
- Unsupported schema major mismatch triggers `rebuild`.
- Git must be readable for normal operation; unreadable git head triggers `rebuild` or a blocking error.

## 5. Decision Pseudocode

```text
function choose_sync_level(inputs):
  if !baseline_exists:
    return rebuild("missing_baseline")

  if schema_version_recorded is null:
    return rebuild("missing_schema_version")

  if major(schema_version_recorded) != major(schema_version_current):
    return rebuild("schema_major_mismatch")

  if current_git_head is null:
    return rebuild("git_head_unreadable")

  delta_t = compute_delta_t(now, last_sync_at)
  delta_head = compute_delta_head(current_git_head, last_sync_git_head)
  delta_overlap = compute_delta_overlap(diff_files, task_files)

  if delta_t < quick_max_age_seconds
     and delta_head == false
     and delta_overlap >= quick_overlap_min:
    return quick("fresh_head_aligned")

  if delta_t >= deep_age_seconds:
    return deep("stale_sync")

  if delta_head == true or delta_head == unknown:
    return deep("head_changed_or_unknown")

  if delta_overlap < deep_overlap_max:
    return deep("low_task_overlap")

  if diff_files is not empty:
    return deep("dirty_workspace_needs_reconcile")

  return quick("clean_workspace")
```

## 6. Decision Tree

```diagram
start
  ├─ missing baseline? ─ yes ─▶ rebuild
  ├─ schema major mismatch? ─ yes ─▶ rebuild
  ├─ git head unreadable? ─ yes ─▶ rebuild/block
  ├─ Delta t < 1h AND same head AND overlap >= 0.80 ─▶ quick
  ├─ Delta t >= 24h ─▶ deep
  ├─ head changed or unknown ─▶ deep
  ├─ overlap < 0.50 ─▶ deep
  ├─ dirty files remain ─▶ deep
  └─ otherwise ─▶ quick
```

## 7. Scenarios

| # | Scenario | Signals | Expected level | Reason |
|---|---|---|---|---|
| 1 | Just synced, clean workspace | `Delta t=5m`, same head, `D=empty` | `quick` | Fresh and aligned |
| 2 | Just synced, one in-scope doc changed | `Delta t=10m`, same head, overlap `1.0` | `quick` | Incremental task-owned diff |
| 3 | Two hours old, same head, no dirty files | `Delta t=2h`, same head, overlap `1.0` | `quick` | Below deep age and clean |
| 4 | One week old, clean workspace | `Delta t=7d`, same head, overlap `1.0` | `deep` | Stale enough to rebuild context |
| 5 | Fresh but git head changed | `Delta t=20m`, head changed | `deep` | New commits may affect projection |
| 6 | Dirty files mostly outside task | `Delta t=30m`, same head, overlap `0.25` | `deep` | Possible drift / ownership ambiguity |
| 7 | No task ownership yet, dirty files exist | `T=empty`, `D>0`, overlap `0.0` | `deep` | Need ownership inference or user question |
| 8 | Missing baseline or schema major mismatch | baseline missing or major mismatch | `rebuild` | Existing snapshot cannot be trusted |

## 8. Required Outputs

Every sync writes or refreshes:

- `SyncStarted` and `SyncCompleted` events when the event writer is available.
- A sync report under `.vibehub/tasks/<task_id>/runs/<run_id>/sync/`.
- `.vibehub/agent-view/current.md`.
- A Context Pack with `freshness.git_head` and `freshness.last_event_id`.

If event writer is unavailable before M1, the sync report records the same facts as best-effort observed state.

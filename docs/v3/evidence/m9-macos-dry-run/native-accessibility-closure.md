# M9 macOS native Accessibility closure

Date: 2026-07-14
Host: macOS arm64, Codex Desktop with System Events Accessibility enabled
Artifact: `/private/tmp/vibehub-m9-native-recheck-20260714/VibeHubM9Isolated.app`
Executable SHA-256: `736ced6784c7d50521e3c91754251784eaf7e50e5a989082e145746c9c5f7f8a`

The app was rebuilt from the current dirty worktree with the isolated bundle identifier
`com.vibehub.launcher.m9isolated`, ad-hoc signed, and launched with `VIBEHUB_PORTABLE=1`.
Its config referenced only fixtures below `/private/tmp/vibehub-m9-final-protocol-20260713`.
Every UI action below was bound to the isolated process PID and used AXPress; no global
screen-coordinate click or real user project was used.

## Accessibility and project-card routing

`System Events` exposed separate project-card actions:

- `打开项目 M9 Task Create Fixture`, `启动`, `收藏`
- `打开项目 M9 Rollback Fixture`, `启动`, `收藏`

This verifies the project-card DOM/action ordering fix and prevents the card action from
being confused with favorite or launch.

## A08 — inspect, initialize, migrate, and recover

An absent fixture displayed state `absent`, `project does not contain .vibehub`,
the exact `vibehub v3 <project> init` recovery guidance, and the bounded-effects
description. AXPressing `初始化 V3` produced `V3 项目`, `尚未创建任务`, and
the native `创建任务` entry.

The rollback fixture opened the native `V3 项目准备` view with state `v2`, the explicit
instruction to run V3 migration, and the required confirmation:

> 我理解现有 V2 数据将原样保存为项目内的只读 legacy-v2 归档，不会转换为 V3 domain。

After checking the confirmation and AXPressing `迁移到 V3`, the app displayed `V3 项目`.
The isolated filesystem contained `.vibehub/project.yaml` and the preserved
`.vibehub/legacy-v2` tree.

An interrupted fixture containing only isolated `.vibehub.v2-migration` staging
displayed state `migration_interrupted`, the exact `migrate-recover` guidance, and
the statement that the backend—not the UI—chooses safe forward/rollback direction.
AXPressing `恢复迁移` produced `rolled_back_to_v2`, state `v2`, and the explicit
archive confirmation before any subsequent migration.

## B05–B07 — legacy archive

The production V3 archive panel exposed source state `degraded`, warnings for intentionally
invalid V2 entries, an empty state before a valid corpus was added, and then
`归档任务（1）`. The valid card displayed:

- title: `Native Legacy Archive Fixture`
- state: `completed`
- completed at: `2026/1/2 08:00:00`
- phase: `review`
- summary: `Native legacy archive details are visible in the isolated app.`
- bounded links: `task.yaml`, `run.yaml`, and `outputs/output.md`

Native file interaction results:

- existing `output.md`: opened without a VibeHub error;
- stale link after deleting only the isolated file: UI displayed
  `VIBEHUB_FILE_NOT_FOUND: .../outputs/output.md`;
- shell-open failure with an intentionally empty PATH: UI displayed
  `No such file or directory (os error 2)`;
- traversal remains rejected by the bounded resolver tests; the production archive reader
  does not generate a traversal link.

The native failure run found and fixed two product defects: missing targets were not rejected
before invoking `open`, and Tauri string rejections were treated as `Error` objects and lost.

## C05–C06 — usage surface

AXPressing `刷新` and the project Token summary opened the production usage drawer. Native
states included `部分数据`, `总 Token 不可用`, `会话数 0`, `暂无 Task session`,
`会话明细（0）`, supported-source `empty`, unsupported-source labels, and the explicit
`本地只读 usage · 费用非实际账单` boundary.

## E06 and F04 — IDE and copied GUI

The copied isolated app completed both launcher outcomes against `M9 Task Create Fixture`:

- success: `code .`; the launch dialog closed and no error remained;
- failure: `vibehub-m9-no-such-ide .`; the dialog stayed open and displayed
  `Launch command exited early with exit status: 127`.

The failure run found and fixed two additional defects: macOS launch only checked that the
shell/open process spawned, and `LaunchDialog` did not render rejected launch errors.

Together with the previously recorded copied CLI/MCP evidence, this closes the remaining
copied-app GUI, legacy browsing, and IDE interaction portion of F04.

## Verification

- `npm run build`: 3712 modules transformed.
- `cargo test -p vibehub --bin vibehub`: 28 passed, 3 ignored, 0 failed.
- `npm run tauri -- build --bundles app --config ...`: release app bundle succeeded.
- All temporary app processes were stopped after evidence capture.

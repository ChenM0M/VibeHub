# Batch M9 — Windows W1/W2 artifact and input package

Date: 2026-07-15
Status: release handoff prepared — **no Windows behavior has been tested or claimed**.

Execution order is intentionally release-first: GitHub Actions produces the
reviewed `3.0.0` draft-release artifacts and checksum manifest; the project
owner then downloads that exact Windows artifact and performs W1/W2 manually on
a Windows machine. CI build success is artifact provenance, not native Windows
acceptance evidence.

## Required execution environment

- A physical Windows 10 or Windows 11 host, or a trusted VM whose filesystem,
  process, installer, IDE, junction/reparse, and locking behavior is treated as
  native enough for acceptance.
- At least one supported IDE installed and available from the test user account
  (record name, version, path, and `PATH`/shell invocation behavior).
- A disposable local drive/workspace and, where required, a second volume to
  exercise cross-volume rename behavior. Do not use the real VibeHub repository.
- A V2 fixture copied to a disposable test directory. Include known old V2
  files, `project.yaml`, state/history/task data, and a recorded sorted SHA-256
  manifest before migration.
- Test directories that cover spaces, long paths near the applicable Windows
  path limit, missing files, read-only/missing targets, locked files, junctions,
  reparse points, and an interrupted process/recovery state.
- Authority to install and uninstall the test package and to inspect only the
  test user's app/helper/runtime/temp paths afterward.

## Artifact identity to hand to Windows executor

The historical M9 native run produced only a **macOS arm64** artifact. The
current release workflow now builds Windows installers and a portable binary
from the exact `v3.0.0` tag, uploads them to one GitHub Release, publishes it
only after every platform job succeeds, and uploads
`SHA256SUMS-windows-x64.txt`. The Windows executor must use those
uploaded files rather than rebuilding from an uncommitted checkout. Record the
exact values below before testing:

| Required record | Required value/evidence |
| --- | --- |
| Source revision | Exact commit resolved by release tag `v3.0.0`; record `git rev-list -n 1 v3.0.0` or the Release provenance. |
| Product version | `3.0.0`; `package.json`, both Cargo packages, lockfiles, and `tauri.conf.json` are checked for equality by `npm run release:check`. |
| Windows package paths | Exact `.msi`, `.exe`, or other installer path(s). |
| SHA-256 | Compare `Get-FileHash -Algorithm SHA256 <artifact>` with the uploaded `SHA256SUMS-windows-x64.txt`. |
| Signature | Stable workflow requires `WINDOWS_CERTIFICATE` and `WINDOWS_CERTIFICATE_PASSWORD`, then rejects non-`Valid` Authenticode results. Recheck locally with `Get-AuthenticodeSignature`. |
| Architecture | `x64`/`arm64` and Windows version under test. |
| Installed executable | Exact resolved path and `--version` output. |
| IDE | Name, version, executable path, and invocation result. |

There is intentionally no Windows hash prefilled before GitHub Actions runs.
The workflow-generated checksum file is authoritative only for the assets in
that exact draft Release. A macOS DMG hash, a local rebuild, or a hash from a
different workflow run must never be substituted.

## Common fixture creation and safety rules

Use a fresh root such as `C:\vibehub-m9-windows\` or an equivalent temporary
volume-owned location. Do not migrate, delete, rename, lock, or point junctions
at the real VibeHub checkout or user project. Keep all logs, screenshots,
installer logs, event outputs, and pre/post hash manifests in that fixture root.

Suggested PowerShell helpers:

```powershell
$Root = Join-Path $env:TEMP 'vibehub-m9-windows'
New-Item -ItemType Directory -Force -Path $Root | Out-Null
$Artifact = 'C:\path\to\downloaded\VibeHub-installer.msi' # exact draft-Release asset
Get-FileHash -Algorithm SHA256 $Artifact
Get-AuthenticodeSignature $Artifact | Format-List *
```

For each test, preserve JSON stdout, stderr, process exit code, package/app
version, relevant filesystem metadata (`Get-Item -Force`, `fsutil reparsepoint
query` when applicable), and native screenshots for GUI/IDE interactions.

## W1 inputs — A07, E07, F07, F09

### A07 — Windows filesystem/lifecycle recovery

**Inputs:** V2 fixture, a V3-absent fixture, a second volume if available,
junction/reparse fixtures, a controllable lock holder, a killable migration
process or documented interruption fixture.

**Commands/operations:**

```powershell
& $InstalledExe v3 $Absent doctor
& $InstalledExe v3 $Absent init
& $InstalledExe v3 $V2 doctor
& $InstalledExe v3 $V2 migrate
& $InstalledExe v3 $Interrupted migrate-recover
```

Create and record:

- a junction/reparse root and an attempted V3 operation against it;
- a cross-volume staging/rename scenario where supported;
- a locked file/directory scenario and its non-destructive error/recovery;
- an interrupted migration state and `migrate-recover` outcome;
- a conflict fixture that proves fail-closed behavior.

**Expected evidence:** native Windows paths, exit codes/error JSON, before/after
hashes, preserved source data, recovery/rollback result, and confirmation that
no junction/reparse escape was followed unsafely.

### E07 — Windows IDE open/reveal

**Inputs:** installed supported IDE, V3 project paths with spaces and a long
path, missing target, valid file, invalid traversal/escaping target.

**Operations:** use the packaged Windows UI to invoke structure open/reveal for
valid and invalid cases. Exercise both the expected IDE success and an
understandable local failure. Do not accept backend source inspection, a raw
`start` command, or headless CLI as UI proof.

**Expected evidence:** native screenshots/video or reliable native automation
transcript showing the path, IDE launched/revealed file, and readable error for
missing/invalid cases.

### F07 — Windows installer clean install and first launch

**Inputs:** exact signed/hashed Windows installer and clean disposable user/VM
state.

**Operations:** verify installer signature/hash; install; record installed
paths/version; launch desktop app normally; capture first-window native state;
quit cleanly.

**Expected evidence:** installer log, native screenshots, resolved executable,
package version/hash/signature, and uninstall registration/location.

### F09 — Installed Windows CLI/MCP/IDE/parallel node

**Inputs:** installed package from F07, isolated V3 project, MCP contract test,
IDE fixtures from E07, worktree command JSON for happy and conflict recovery.

**Commands:**

```powershell
& $InstalledExe --version
node scripts/v3-mcp/contract-test.mjs $InstalledExe
& $InstalledExe v3 $Project worktree-event <command.json>
& $InstalledExe v3 $Project worktree-orchestration <project_id> <task_id>
```

**Expected evidence:** packaged-executable MCP matrix (7 resources, 7 tools, 21
requests in the current reference matrix), both worktree paths reaching `cleaned`, native IDE evidence, and
all outputs tied to the exact installed package hash.

## W2 inputs — F08, F10, G05

### F08 — Windows V2→V3 upgrade, lock recovery, rollback

**Inputs:** recorded V2 fixture/hash manifest, lock holder, interrupted-migrate
fixture, backup destination, installed package from W1.

**Operations:** V2 `doctor` → `migrate` → V3 `doctor`; compare full archive
manifest; repeat migration for idempotency; execute interrupted recovery;
exercise Windows lock handling; perform documented manual rollback only on the
copied fixture and confirm final `doctor` reports V2.

**Expected evidence:** exact pre/archive/post manifests, all JSON results and
exit codes, lock/recovery behavior, preserved V2 data, rollback result, and no
real project mutation.

### F10 — Windows uninstall and residue cleanup

**Inputs:** installed W1 package, separate V3/legacy project fixture, known
application data/installer/helper locations, parallel-node fixture.

**Operations:** uninstall through the package's documented method. Verify
application package files, registered helper/background/task/scheduled-service
residue, temporary/worktree residue, and that project `.vibehub/legacy-v2`
archives remain intact. Do not treat deletion of projects as successful
uninstall.

**Expected evidence:** uninstall log, before/after filesystem and registration
checks, preserved archive hash/doctor result, and all remaining expected config
items documented separately from project data.

### G05 — final dual-platform chain

**Precondition:** all A–F acceptance IDs are verified on the required hosts.

Run this chain against the exact installed Windows artifact and pair its result
with the recorded macOS chain:

```text
clean install → legacy → MCP → IDE → parallel node → upgrade/rollback → uninstall
```

**Expected evidence:** a single index mapping platform, exact artifact/version/
SHA-256, fixtures, commands, native evidence, and cleanup results for every
link. Do not mark G05 verified from macOS evidence alone.

## Required final report from Windows executor

For every W1/W2 ID report only:

1. exact artifact/version/SHA-256/signature and host/IDE details;
2. disposable fixture paths and commands;
3. raw output and native screenshots/logs where interaction is required;
4. archive/recovery/residue hashes and checks;
5. pass/fail/blocker with a concrete external cause.

If a host prerequisite is absent, leave the affected checklist item `BLOCKED`;
do not infer Windows success from current macOS source or CLI tests.

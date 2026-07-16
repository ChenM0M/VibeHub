# Batch M3 native-window permission gate

Date: 2026-07-12
Platform: macOS 26.5.1 (Build 25F80), arm64

Target IDs: A08, B05, B06, B07, C05, C06.

## Gate check

The fixed execution plan requires Accessibility and System Events UI automation permission for the actual host process before native-window interaction evidence can be collected.

Command:

```sh
osascript -e 'tell application "System Events" to return UI elements enabled'
```

Result:

```text
false
```

A pre-existing installed application process was also present:

```text
/Applications/VibeHub.app/Contents/MacOS/vibehub
```

That process is not the Batch M2 signed artifact and was not used as evidence, terminated, overwritten, or modified.

## Outcome

Native GUI launch alone cannot prove the required lifecycle, legacy archive, file-link, usage state, refresh, drawer, and session-expansion interactions. Without Accessibility permission, the current host cannot programmatically click controls, inspect native UI state through System Events, or collect a reproducible state-by-state native evidence matrix.

Per `docs/v3/m6-execution-plan.md`, these six IDs are frozen after this demonstrated permission gate and execution continues with Batch M4 instead of spending another batch retrying the same permission. Existing implementation, automated tests, and browser evidence remain intact; only the required native-window interaction evidence is blocked.

No repository `.vibehub` file, real project, real transcript, CI configuration, or installed application was read or modified by this gate check.

## M9 user-requested recheck — 2026-07-13

After the user stated that Accessibility permission was already enabled, M9
performed one explicit recovery check. The status query still returned
`false`. To rule out a status-API false negative, the final clean-copy app was
started from:

```text
/private/tmp/vibehub-m9-final-protocol-20260713/install/VibeHub.app
```

The process was observed as PID `18000`. A single real AX window query against
that exact process failed with:

```text
“System Events”遇到一个错误：“osascript”不允许辅助访问。 (-25211)
```

This is direct evidence that the current `osascript` client used by the Codex
execution host does not have effective macOS TCC Accessibility authorization,
even though the user-facing settings were believed to be enabled. No further
permission retries were made. The affected IDs remain blocked until the
authorization change is visible to this execution client, usually after
granting the correct host/client entry and restarting that host process.

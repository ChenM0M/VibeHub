# RFC Backlog 002: MCP Control Plane Contract

- Status: M0 boundary frozen; M2 compatibility decision backlog
- Owner: V3 MCP Adapter owner
- Decision milestone: M0; compatibility evidence completed in M2
- Implementation milestone: M2
- Related task: `T-20260711080143-b1ff21ea`
- Evidence: `.vibehub/research/current/research-pack.md` F004, F005

## Problem

VibeHub needs one agent-native control plane across Codex, Claude Code, and OpenCode. MCP hosts share protocol basics but differ in configuration, approval, roots, elicitation, discovery, instructions, and extensions. The adapter must not duplicate domain logic or make optional host features mandatory for recovery.

## Invariants

1. MCP and CLI call the same typed Application Service.
2. Read-heavy state is exposed as resources; state transitions are narrow typed tools.
3. Stdio is the first transport and ships inside the native VibeHub binary.
4. The minimal recovery loop works without elicitation, dynamic discovery, or a daemon.
5. Every write carries task/session identity, expected version, and idempotency key.

## Decisions To Resolve

### D1. Resource URI namespace

Define versioned URIs for project overview/structure, task timeline/plan/criteria, node brief, session state, protocol coverage, and diagnostics. Specify pagination, freshness, content type, and not-found/stale results.

### D2. Minimal tool surface

Start with `session_open`, `event_log`, and `session_close`; decide exact additions for plan changes, criterion results, findings, remediation attempts, and completion proposals. Avoid one tool per V2 CLI command.

### D3. Error and retry semantics

Map validation error, version conflict, duplicate idempotency key, permission denial, missing scope, stale resource, and internal error into stable structured results.

### D4. Human confirmation

Define `completion_proposed` plus digest/version challenge. Host elicitation or approval may collect confirmation when available; otherwise a human CLI/UI channel completes it. An agent assertion alone cannot become `user_confirmed`.

### D5. Host compatibility profile

For each pinned Codex/Claude Code/OpenCode version, record stdio launch, cwd/root semantics, instructions, resources, tool annotations, approval, elicitation, list-changed, timeout, shutdown, and Windows behavior as supported/degraded/unsupported/not-tested.

### D6. Packaging and security

Define binary discovery, argument-array launch, environment allowlist, project scope, log redaction, stdout purity, stderr diagnostics, and server version negotiation.

## Required Spikes

1. Official Rust SDK stdio server exposing one resource and one read-only tool.
2. Run the same binary from all three hosts on macOS and at least one host on Windows.
3. Inspector contract test for schema, initialization, cancellation, and clean shutdown.

## Deliverables

- Resource URI and tool catalog with schemas.
- Application Service boundary and adapter call map.
- Host compatibility matrix template and pinned test versions.
- Confirmation, authorization, error, and retry contract.
- Packaging/config generation plan with CLI fallback.

## Exit Criteria

- M1 mock interactions map one-to-one to catalog entries without invoking them.
- The minimal recovery loop has no dependency on optional host features.
- M2 can implement the adapter without changing domain transitions.
- Cross-host failures are explicit capability gaps, not silently different behavior.

## Not Decided Here

- A local Streamable HTTP daemon. Revisit only after stdio multi-process measurements.
- Remote/cloud MCP deployment. It is outside V3 M0-M6.

## M0 Freeze Record

- `decided`: MCP and CLI remain adapters over one Application Service; M1 uses
  only `V3ViewRepository` and never starts MCP or invokes a production command.
- `decided`: structured read failures use the common warning/error vocabulary;
  optional host features cannot be required for the recovery loop.
- `deferred`: exact resource URIs, tool schemas, host matrix results, packaging,
  authorization, and retry mapping require M2 protocol/native evidence.
- Alternatives retained: versioned stdio MCP is primary; CLI fallback is
  mandatory; a local HTTP daemon and remote MCP remain explicit non-scope.
- Evidence/exit: the fixture repository proves all five view reads behind one
  transport-neutral boundary; M2 exit criteria above remain unchanged.

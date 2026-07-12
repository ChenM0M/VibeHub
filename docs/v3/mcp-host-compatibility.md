# V3 MCP Host Compatibility

## Launch Contract

All hosts must launch an argument array, not a shell command:

```text
<absolute-vibehub-binary> mcp-stdio <absolute-project-root>
```

The server uses MCP `2025-11-25`, writes JSON-RPC only to stdout, sends diagnostics
to stderr, and exits cleanly when stdin closes. Project scope comes exclusively
from the final argument; no current-working-directory inference is used.

## Host Configurations

Codex (`~/.codex/config.toml`):

```toml
[mcp_servers.vibehub-v3]
command = "/absolute/path/to/vibehub"
args = ["mcp-stdio", "/absolute/path/to/project"]
startup_timeout_sec = 10
tool_timeout_sec = 30
```

Claude Code (`.mcp.json`):

```json
{
  "mcpServers": {
    "vibehub-v3": {
      "type": "stdio",
      "command": "/absolute/path/to/vibehub",
      "args": ["mcp-stdio", "/absolute/path/to/project"]
    }
  }
}
```

OpenCode (`opencode.json`):

```json
{
  "mcp": {
    "vibehub-v3": {
      "type": "local",
      "command": ["/absolute/path/to/vibehub", "mcp-stdio", "/absolute/path/to/project"],
      "enabled": true
    }
  }
}
```

## Evidence Matrix

| Host/platform | Version | Launch/config | Initialize/resources/tools | Recovery loop | Shutdown | Status |
| --- | --- | --- | --- | --- | --- | --- |
| Raw stdio/macOS arm64 | local build | supported | passed | passed | passed | supported |
| Codex/macOS arm64 | 0.132.0 | isolated-home `mcp add/get` passed | contract harness passed outside host | not host-executed | not host-executed | degraded |
| Claude Code/macOS arm64 | 2.1.197 | isolated-home `mcp add/get` passed | health check blocked by missing login | not host-executed | not host-executed | degraded |
| OpenCode/macOS arm64 | unavailable | config template only | not tested | not tested | not tested | not tested |
| Any host/Windows | unavailable | path fixtures only | not tested | not tested | not tested | not tested |

`npm run v3:mcp:check` is the repeatable transport contract. Official MCP Inspector
execution is still required before promotion; the local security reviewer rejected
running downloaded Inspector code against the workspace without explicit user approval.

## Capability Semantics

- Resources are versioned under `vibehub://v3/1.0/` and return JSON matching the
  frozen M0 view contracts.
- `session_open`, `event_log`, and `session_close` call the same typed Application
  Service as the CLI fallback.
- Duplicate idempotency keys return a successful duplicate no-op. Scope mismatch
  and optimistic version conflict return caller-visible structured tool errors.
- Elicitation, approval APIs, dynamic list changes, subscriptions, HTTP transport,
  and remote deployment are not required by the minimal recovery loop.

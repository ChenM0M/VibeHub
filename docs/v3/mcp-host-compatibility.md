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

Codex（项目 `.codex/config.toml`）:

```toml
[mcp_servers.vibehub-v3]
command = "/absolute/path/to/vibehub"
args = ["mcp-stdio", "/absolute/path/to/project"]
startup_timeout_sec = 10
tool_timeout_sec = 30
env = { VIBEHUB_MCP_CATALOG = "agent" }
```

Claude Code (`.mcp.json`):

```json
{
  "mcpServers": {
    "vibehub-v3": {
      "type": "stdio",
      "command": "/absolute/path/to/vibehub",
      "args": ["mcp-stdio", "/absolute/path/to/project"],
      "env": { "VIBEHUB_MCP_CATALOG": "agent" }
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
      "enabled": true,
      "environment": { "VIBEHUB_MCP_CATALOG": "agent" }
    }
  }
}
```

新同步的配置默认显式启用 `agent`（12 个工具），已有配置中明确选择的
`legacy`/`advanced` 保持原样。需要计划编辑、criterion review 等高级操作时，
将此连接的变量改为 `advanced` 并重连；不依赖运行中动态目录变化。未配置变量
的既有客户端继续使用兼容目录，`task_view` 形状不变。

普通恢复优先 `workspace_context(session_id)`，已知任务可直接 `task_brief`；
详情与证据用 `task_inspect`，诊断导出通过普通工具分块读取，不依赖 resource
自动展开。新组合写入使用稳定 request_id；重连后恢复显式 Session scope。

## Current Agent-native evidence (2026-09-26)

| Host / platform | Recorded version / model alias | Repeated basic loop | Two-process reconnect + evidence fallback | Controlled Memory/evidence injection |
| --- | --- | --- | --- | --- |
| Codex / macOS arm64 | 0.156.1 / gpt-6-astra | 2/2 | passed | 2/2 resisted |
| Claude Code / macOS arm64 | 2.1.273 / step-3.5-flash-2603 | 2/2 | passed | 2/2 resisted |
| OpenCode / macOS arm64 | 1.18.32 / stepfun_official/step-3.7-flash | 2/2 | passed | 2/2 resisted |
| Raw stdio / Windows 11 build 26200 | isolated source build; hash in evidence | new + legacy passed | kill/restart passed | domain boundary tests passed |
| Windows Release installation / IDE | exact artifact unavailable | BLOCKED | BLOCKED | not inferred from source tests |

The basic loop asserts an actual missing-task rejection, successful correction,
start/recovery/progress/finish, and five independently read domain events. It never
completes the Task. Models are recorded aliases, not immutable provider snapshots;
these small samples do not establish universal model reliability. Failed earlier
provider/model combinations remain historical evidence, not supported combinations.

[Evidence and repeatable commands](evidence/agent-native/README.md) include final
binary hashes, safe tool traces, actual outgoing Claude tool-content measurements,
and explicit limits. Resource rendering is optional: ordinary tool fallback is
verified in all three hosts. Official Inspector tools/list and task_brief passed
with zero schema errors; nullable and legacy free-form details portability warnings
are recorded. No universal provider-schema compatibility claim is made.

## Evidence Matrix（历史基线）

| Host/platform | Version | Launch/config | Initialize/resources/tools | Recovery loop | Shutdown | Status |
| --- | --- | --- | --- | --- | --- | --- |
| Raw stdio/macOS arm64 | local build | supported | passed | passed | passed | supported |
| Codex/macOS arm64 | 0.132.0 | isolated-home `mcp add/get` passed | contract harness passed outside host | not host-executed | not host-executed | degraded |
| Claude Code/macOS arm64 | 2.1.197 | isolated-home `mcp add/get` passed | health check blocked by missing login | not host-executed | not host-executed | degraded |
| OpenCode/macOS arm64 | unavailable | config template only | not tested | not tested | not tested | not tested |
| Any host/Windows | unavailable | path fixtures only | not tested | not tested | not tested | not tested |

`npm run v3:mcp:check` is the repeatable transport contract. Official MCP Inspector
was subsequently run against a new disposable fixture for this redesign; see the
current evidence above. The earlier approval rejection concerned the historical
attempt against the workspace and is not a current blocker.

## Capability Semantics

- Resources are versioned under `vibehub://v3/1.0/` and return JSON matching the
  frozen M0 view contracts.
- `session_open`, `event_log`, and `session_close` call the same typed Application
  Service as the CLI fallback.
- Duplicate idempotency keys return a successful duplicate no-op. Scope mismatch
  and optimistic version conflict return caller-visible structured tool errors.
- Elicitation, approval APIs, dynamic list changes, subscriptions, HTTP transport,
  and remote deployment are not required by the minimal recovery loop.

本次 agent-native 重设计的当前宿主验证结果以
[设计文档第 14 节](agent-native-mcp-redesign.md#148-ac01ac17-当前证据判定)
及其引用的真实宿主报告为准；上表保留原始历史记录，不代表当前结论。

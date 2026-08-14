# VibeHub

[English](README_EN.md) | [简体中文](README.md) | [繁體中文](README_TC.md)

![alt text](image-1.png)

> VibeHub is an all-in-one tool platform built around the Agent workflow experience. It grew
> out of the pain points of personal Vibe development: the plan, progress, acceptance and
> blockers of an Agent's work lacked a visible, traceable record. VibeHub turns that
> "invisible process" into boards and plan graphs with a task / plan / session / event-driven
> workflow. It is not tied to any specific Agent harness — as long as your Agent can read the
> workflow orchestration spec in the repository and call standard MCP tools, it can join in.
> The desktop app lowers the barrier with a visual Cockpit, while the core is essentially a
> standalone CLI that you can extract and use on its own, or fork and modify.

![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)
![GitHub Release](https://img.shields.io/github/v/release/ChenM0M/VibeHub)

[Changelog](CHANGELOG.md) · [Releases](https://github.com/ChenM0M/VibeHub/releases)

## Main feature: the V3 workflow

- **Harness-agnostic** — not bound to OpenCode / Codex / Claude Code or any particular harness; any Agent that can read the repository's workflow orchestration spec and call the V3 MCP tools can join the same workflow
- **CLI core + visual frontend** — the core is a standalone CLI (`vibehub v3 ...`) that you can extract or fork; the desktop V3 Cockpit is only its visual entry point
- **Full traceability** — tasks, plan DAGs, sessions and events are all recorded; progress / risk are logged as you go, so process, results and past work can always be traced back
- **Per-criterion acceptance + blocker repair guidance** — acceptance criteria are reviewed one by one (passed / failed / blocked); blockers carry a reason, impact and actionable repair steps
- **Completion gate closure** — plan → session → progress → result → review, none can be skipped; a task cannot be closed on a verbal "done"
- **Project Memory** — long-term memory is only written through typed commands; disputed / stale / secret entries are not injected by default
- **Visual Cockpit** — plan graph, node drawers, acceptance progress, blocker panel and AI usage panel in one place

## Around the workflow

### Project management & launching — the entry point

- Point at a workspace directory and it scans and identifies Node.js / Rust / Python / Java / Go / .NET projects automatically
- Tag categories (IDE, CLI, environment, ...) + drag-and-drop ordering; open a project with the right tool in one click, or use "custom launch" for any command
- Opened projects stay resident as tabs, with persisted ordering
- Cards show the current branch and change status directly
- Dark mode follows the system or switches manually; UI is available in 简体中文 / 繁體中文 / English

### Agent configuration & usage — the resource side

- **Agent Profiles** — manage models, providers, credential references and reasoning levels for OpenCode / Claude Code / Codex in one place; protocol capability is diagnosed automatically (direct / adapter / unavailable); credentials only accept environment-variable or system secret-store references and auth files are never read
- **AI usage stats** — locally reads and aggregates token records from Claude Code / Codex / OpenCode in a read-only way; cost is derived from real token counts; anomalous data fails closed — it never treats "0" as "no usage" and never fabricates a bill

### Infrastructure — the foundation

- **AI gateway** — built-in proxy with multi-provider load balancing, model mapping and Claude Code protocol conversion; listens on loopback `127.0.0.1` only by default
- **Portable** — green, no installation; configuration lives in the `data` directory next to the executable
- **Multilingual** — built-in Simplified Chinese / Traditional Chinese / English

## Install and download

### macOS: Homebrew

Homebrew is the recommended way to install on macOS:

```bash
brew install --cask chenm0m/vibehub/vibehub
```

Update:

```bash
brew update
brew upgrade --cask vibehub
```

The Homebrew tap repository is `ChenM0M/homebrew-vibehub`. After every GitHub Release (including previews) is published, CI updates the cask automatically from the Apple Silicon and Intel DMGs.

### Manual download

[→ Releases page](https://github.com/ChenM0M/VibeHub/releases)

| Platform | Format |
|------|------|
| Windows | `.exe` installer / `Portable.zip` |
| macOS | `.dmg` (Apple Silicon & Intel) |
| Linux | `.deb` / `.AppImage` |

Windows / Linux Portable versions are unzip-and-run. On macOS, the installed app writes configuration and AI gateway data to the system application data directory:

```text
~/Library/Application Support/VibeHub
```

If you really need portable mode, launch with `VIBEHUB_PORTABLE=1`; configuration will then be written to `data/` next to the executable. Portable mode is not recommended for regular macOS `.app` / DMG / Homebrew installs because the app bundle is usually not writable.

## V3 workflow quick start

The V3 core is a standalone CLI (with an MCP server of the same name) that you can run directly in any project directory:

```bash
# Inspect / initialize / migrate a project's V3 layout
vibehub v3 <project> doctor
vibehub v3 <project> init
vibehub v3 <project> migrate

# Create tasks and inspect tasks and plans
vibehub v3 <project> task-create <request.json>
vibehub v3 <project> task-view <task_id>
vibehub v3 <project> task-lifecycle . <task_id>

# Sessions and acceptance
vibehub v3 <project> session-open ...   # open a session before starting work
vibehub v3 <project> event-log ...      # milestone progress / risks
vibehub v3 <project> criterion-review ...  # review acceptance one by one
vibehub v3 <project> task-completion-propose ...  # ask the user to confirm completion
```

Onboarding an Agent takes two steps: point it at the workflow orchestration spec in the repository (`AGENTS.md`) and give it the V3 MCP server:

```json
{
  "mcpServers": {
    "vibehub": {
      "command": "vibehub",
      "args": ["mcp-stdio", "/path/to/your/project"]
    }
  }
}
```

All commands output JSON. For details see:

- [`docs/v3/agent-release-process.md`](docs/v3/agent-release-process.md) — mandatory Agent work and release acceptance process
- [`docs/v3/migration-guide.md`](docs/v3/migration-guide.md) — V2 → V3 migration and layout states
- [`docs/v3/agent-profiles.md`](docs/v3/agent-profiles.md) — Agent Profiles support matrix and secret model
- [`docs/v3/usage-source-capabilities.md`](docs/v3/usage-source-capabilities.md) — AI usage data source capability boundaries
- [`contracts/v3/`](contracts/v3/README.md) — view and command JSON Schema contracts

## Build from source

Requires Node.js 20+ and Rust 1.77.2+.

```bash
git clone https://github.com/ChenM0M/VibeHub.git
cd VibeHub
npm install
npm run tauri dev
```

On macOS you can run the environment check and dev launcher scripts directly:

```bash
./start-dev.sh --check
./start-dev.sh
```

Build a release:

```bash
npm run tauri build
```

Build only the V3 CLI (distributable on its own):

```bash
cargo build --locked -p vibehub-cli --bin vibehub
```

Common quality gates:

```bash
npm run release:check        # version consistency (package / lockfile / both V3 crates / Tauri / tauri.conf.json)
npm run build                # frontend type-check and build
npm run v3:contracts:check   # V3 contract, fixture and generated-type validation
npm run v3:mcp:check         # MCP contract tests
```

Platform dependencies:

- Windows → Visual Studio Build Tools
- macOS → Xcode Command Line Tools
- Linux → `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev`

## Release flow

1. After changing versions, run `npm run release:check` to make sure `package.json`, the lockfile, both V3 crates, the Tauri Cargo manifest and `tauri.conf.json` agree; to validate a tag use `RELEASE_TAG=v3.2.0 npm run release:check`.
2. Create a version tag (e.g. `v3.2.0`; tags containing `-` are previews).
3. The `Release` workflow builds Windows, Linux, macOS Apple Silicon and macOS Intel artifacts, creates a draft Release with per-platform SHA-256 manifests, and only publishes the draft after every platform succeeds and the required artifacts pass validation.
4. After publishing, the reusable `Update Homebrew Cask` workflow is called directly to update `ChenM0M/homebrew-vibehub` with the actual published DMGs and SHA-256 values (requires `HOMEBREW_TAP_TOKEN`; a missing token fails explicitly on the Release path so the cask never silently lags behind).
5. Apple / Windows code signing is an optional enhancement: with complete credentials the pipeline uses real signing, otherwise artifacts are macOS ad-hoc / Windows unsigned and are not presented as officially signed.
6. Native Windows test items (`A07`, `E07`, `F07–F10`, `G05`) must be completed after release, on a Windows host, using the same release's Windows artifact and real SHA-256.

See [`docs/v3/agent-release-process.md`](docs/v3/agent-release-process.md) for details.

## Project structure

```
VibeHub/
├── src/                     # React + TypeScript frontend
│   ├── pages/               # Home / AgentProfiles / Gateway / Settings / About
│   ├── v3/                  # V3 Cockpit (plan graph, acceptance progress, blockers, AI usage panel)
│   └── legacy-v2/           # legacy protocol entry (read-only archive and migration)
├── src-tauri/               # Tauri desktop shell and Rust commands (scanner, launcher, gateway, agent profiles, usage reader)
├── crates/
│   ├── vibehub-core/        # V3 domain core (events, projections, validators)
│   └── vibehub-cli/         # vibehub CLI and MCP server
├── contracts/v3/            # V3 view and command JSON Schema contracts
├── docs/v3/                 # V3 delivery, process and migration docs
├── scripts/                 # contract generation, gates and release checks
└── .vibehub/                # per-project V3 workflow state (events, projections, tasks)
```

## How tags and launching work

Tags are VibeHub's core concept. Each tag can bind a launch configuration (executable + arguments + environment variables), categorized as IDE, CLI, environment, and so on.

Once a project has a tag, clicking launch performs the action for that tag type — IDE tags receive the project path as an argument, CLI tags open a new window in the project directory.

You can also skip tags and use "custom launch" to run any command.

### CLI tag examples

CLI tags can select a terminal app. Each system shows options suitable for the current platform; unknown terminal values fall back safely to the system default launch method.

| Desired effect | Category | Terminal | Executable | Args |
| --- | --- | --- | --- | --- |
| Open the project in Warp and run OpenCode | CLI | `Warp` | `opencode` | empty or your args |
| Open the project in Warp and run Claude Code | CLI | `Warp` | `claude` | empty or your args |
| Open the project in Warp and run AMP | CLI | `Warp` | `amp` | empty or your args |
| Open the project in iTerm and run OpenCode | CLI | `iTerm` | `opencode` | empty or your args |
| Run npm dev in the system Terminal | CLI | `Terminal` | `npm` | `run dev` |
| Run OpenCode in Windows Terminal | CLI | `WindowsTerminal` | `opencode` | empty or your args |
| Run Claude Code in PowerShell | CLI | `PowerShell` | `claude` | empty or your args |
| Run npm dev in cmd | CLI | `CommandPrompt` | `npm` | `run dev` |

If you only want to open Warp at the project directory without running a command, Warp also supports URIs like `warp://action/new_tab?path=<project path>`; VibeHub's CLI tags are better suited for "enter the project directory and start a CLI agent".

Agent models, providers and credential references are managed centrally by **Agent Profiles**, independent of tag launch configurations.

## Contributing

PRs and Issues are welcome.

## License

[Apache License 2.0](LICENSE)

## Credits

- [Tauri](https://tauri.app/) — cross-platform desktop framework
- [React](https://react.dev/) + [TailwindCSS](https://tailwindcss.com/) — frontend
- [b4u2cc](https://github.com/CassiopeiaCode/b4u2cc) — reference for Claude Code protocol conversion

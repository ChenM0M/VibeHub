# VibeHub Agent Output

## Completed
- `user_confirmed`: The user requested macOS usability fixes, Homebrew install/release support, CI/CD and README updates, mac-safe storage behavior, a mac start-dev helper, native mac window behavior without breaking Windows, and tag support for launching CLI agents in terminals such as Warp.
- `hard_observed`: Added platform-aware app paths so normal macOS storage uses `~/Library/Application Support/VibeHub`; portable mode remains available via `VIBEHUB_PORTABLE=1`, and legacy `data/` files are migrated when present.
- `hard_observed`: Updated macOS launcher behavior so CLI tags open in a real terminal session with project-directory context, `.app` targets use `open -a`, and non-CLI commands run through `/bin/zsh -lc`.
- `hard_observed`: Added `TagConfig.terminal` on Rust and TypeScript models and exposed Terminal.app, iTerm.app, and Warp.app selection in the CLI tag edit dialog.
- `hard_observed`: Added Warp CLI-launch support through a temporary executable `.command` file under the system temp directory with delayed cleanup, avoiding project-directory and home-directory clutter.
- `hard_observed`: Added Windows-aware CLI terminal handling so tags can use Windows Terminal, PowerShell, Command Prompt, or safely fall back to the existing `cmd /C start` behavior for unsupported terminal values.
- `hard_observed`: Updated the tag editor to show platform-appropriate terminal options while preserving terminal values configured for another platform.
- `hard_observed`: Added Settings > General import/export for tags and AI gateway configuration using a JSON file with `source_system`, export timestamp, app version, tags, and `gateway_config`.
- `hard_observed`: Import now merges tags by ID or name/category, keeps unrelated local tags, replaces the AI gateway configuration, and returns a compatibility report listing affected tags/providers and field-level adjustments.
- `hard_observed`: Import compatibility normalization adjusts CLI terminal values across macOS, Windows, and Linux; strips platform-specific executable wrappers such as `.app`, `.cmd`, `.exe`, `.bat`, and `.ps1` when needed; migrates legacy gateway `port` into `anthropic_port`; and fills missing provider `api_types`.
- `hard_observed`: Settings > General shows import success/error status, source system to current system, and every compatibility adjustment made during import.
- `hard_observed`: Switched macOS runtime window decoration to native traffic-light chrome while preserving the existing frameless/custom controls on Windows and Linux.
- `hard_observed`: Hid Windows-style header window controls on macOS and disabled custom header dragging there so native mac window behavior is used.
- `hard_observed`: Replaced the black-square neon icon with a macOS-style rounded app icon, regenerated Tauri PNG, ICNS, ICO, iOS, Android, and Windows logo assets, and verified the icon is included in the macOS `.app` bundle.
- `hard_observed`: Bound gateway listeners to `127.0.0.1` instead of `0.0.0.0` to keep local gateway behavior private to the machine by default.
- `hard_observed`: Added `start-dev.sh` and package scripts `startdev`, `startdev:mac`, and `check:mac` for mac environment checks, dependency install, dev launch, and validation.
- `hard_observed`: Updated release CI to build macOS arm64, macOS x64, Windows, and Linux artifacts and to create a draft release with mac/Homebrew guidance.
- `hard_observed`: Added a Homebrew workflow that updates a tap cask from published DMG assets and documented the required `HOMEBREW_TAP_TOKEN`.
- `hard_observed`: Hardened the Homebrew workflow to generate cask URLs from the actual published DMG filenames instead of assuming the release tag and bundled app version always match, so prerelease DMGs are handled correctly.
- `hard_observed`: Updated release draft body and README to explicitly mention Homebrew install/upgrade for preview/prerelease releases.
- `hard_observed`: Updated README with Homebrew install instructions, mac development setup, release guidance, Apple signing/notarization secrets, and CLI tag examples for Warp/OpenCode/Claude Code/AMP.
- `hard_observed`: Homebrew was available at `/opt/homebrew/bin/brew`; Rust and Node 20 were installed/configured through Homebrew for the local mac development environment.

## Not Yet Done
- `inferred`: Apple Developer signing and notarization were wired into workflow environment variables but cannot be fully validated without real repository secrets and an Apple Developer certificate.
- `inferred`: The Homebrew tap update workflow cannot be fully validated until a release is published and `ChenM0M/homebrew-vibehub` plus `HOMEBREW_TAP_TOKEN` are available.
- `inferred`: Warp must be installed as `Warp.app`, and commands such as `opencode`, `claude`, and `amp` must be resolvable from the shell PATH used by `/bin/zsh`.

## Key Decisions Made
- `agent_reported`: Kept `decorations: false` in `tauri.conf.json` so Windows keeps the existing custom chrome; macOS enables native decorations at runtime under `#[cfg(target_os = "macos")]`.
- `agent_reported`: Added terminal selection only to CLI tags because IDE/app tags already have a direct executable/app launch path; Windows-specific terminal choices are handled in the Windows launcher branch.
- `agent_reported`: Used a temp `.command` file for Warp because Warp does not expose the same stable AppleScript command execution surface as Terminal/iTerm; cleanup is delayed to let Warp open the file reliably.
- `agent_reported`: Chose macOS Application Support as the default persistent data location to avoid writing runtime files into the app bundle, repository, desktop, or home root.
- `agent_reported`: Kept the icon generator as a repo script and committed generated assets so normal Windows and macOS builds do not require Swift or image tooling.

## Files Changed
- `hard_observed`: `.github/workflows/build.yml`
- `hard_observed`: `.github/workflows/release.yml`
- `hard_observed`: `.github/workflows/homebrew.yml`
- `hard_observed`: `.gitignore`
- `hard_observed`: `README.md`
- `hard_observed`: `package.json`
- `hard_observed`: `start-dev.sh`
- `hard_observed`: `app-icon.png`
- `hard_observed`: `public/app-icon.png`
- `hard_observed`: `scripts/generate-app-icon.swift`
- `hard_observed`: `src-tauri/icons/*`
- `hard_observed`: `src-tauri/Cargo.lock`
- `hard_observed`: `src-tauri/Cargo.toml`
- `hard_observed`: `src-tauri/src/app_paths.rs`
- `hard_observed`: `src-tauri/src/commands.rs`
- `hard_observed`: `src-tauri/src/gateway/mod.rs`
- `hard_observed`: `src-tauri/src/gateway/proxy.rs`
- `hard_observed`: `src-tauri/src/launcher.rs`
- `hard_observed`: `src-tauri/src/main.rs`
- `hard_observed`: `src-tauri/src/models.rs`
- `hard_observed`: `src-tauri/src/process_util.rs`
- `hard_observed`: `src-tauri/src/storage.rs`
- `hard_observed`: `src-tauri/src/updater.rs`
- `hard_observed`: `src/components/Header.tsx`
- `hard_observed`: `src/components/TagEditDialog.tsx`
- `hard_observed`: `src/pages/Settings.tsx`
- `hard_observed`: `src/services/tauri.ts`
- `hard_observed`: `src/types/index.ts`
- `hard_observed`: `src/locales/en.json`
- `hard_observed`: `src/locales/zh.json`
- `hard_observed`: `src/locales/zh-TW.json`
- `hard_observed`: `.vibehub/tasks/T-20260513154224-58d8a685/runs/R-20260513154224-488adb79/outputs/output.md`

## Files Reportedly Read
- `agent_reported`: `.vibehub/agent-view/current.md`
- `agent_reported`: `.vibehub/agent-view/current-context.md`
- `agent_reported`: `.vibehub/agent-view/handoff.md`
- `agent_reported`: `.vibehub/rules/hard-rules.md`
- `agent_reported`: `.vibehub/adapters/protocol.md`
- `agent_reported`: `.vibehub/tasks/T-20260513154224-58d8a685/runs/R-20260513154224-488adb79/context-packs/align.md`
- `agent_reported`: `package.json`
- `agent_reported`: `src-tauri/Cargo.toml`
- `agent_reported`: `src-tauri/tauri.conf.json`
- `agent_reported`: `src-tauri/src/commands.rs`
- `agent_reported`: `src-tauri/src/gateway/mod.rs`
- `agent_reported`: `src-tauri/src/gateway/proxy.rs`
- `agent_reported`: `src-tauri/src/launcher.rs`
- `agent_reported`: `src-tauri/src/main.rs`
- `agent_reported`: `src-tauri/src/models.rs`
- `agent_reported`: `src-tauri/src/process_util.rs`
- `agent_reported`: `src-tauri/src/storage.rs`
- `agent_reported`: `src-tauri/src/updater.rs`
- `agent_reported`: `src/components/Header.tsx`
- `agent_reported`: `src/components/TagEditDialog.tsx`
- `agent_reported`: `src/pages/Settings.tsx`
- `agent_reported`: `src/services/tauri.ts`
- `agent_reported`: `src/types/index.ts`
- `agent_reported`: `src/locales/en.json`
- `agent_reported`: `src/locales/zh.json`
- `agent_reported`: `src/locales/zh-TW.json`
- `agent_reported`: `README.md`

## Commands Run
- `agent_reported`: `git clone git@github.com:ChenM0M/VibeHub.git`
- `agent_reported`: `/bin/zsh -lc "mkdir -p /Users/chenm0m/.ssh && ssh-keyscan github.com >> /Users/chenm0m/.ssh/known_hosts"`
- `agent_reported`: `git status --short`
- `agent_reported`: `git branch --show-current`
- `agent_reported`: `brew --version`
- `agent_reported`: `brew install rust`
- `agent_reported`: `brew install node@20`
- `agent_reported`: `npm ci`
- `agent_reported`: `npm run build`
- `agent_reported`: `cargo fmt --all -- --check`
- `agent_reported`: `cargo fmt --all`
- `agent_reported`: `cargo check --locked`
- `agent_reported`: `npm run tauri build`
- `agent_reported`: `swift scripts/generate-app-icon.swift`
- `agent_reported`: `npm run tauri -- icon app-icon.png`
- `agent_reported`: `npm run tauri -- build --bundles app`
- `agent_reported`: `codesign --force --deep --sign - src-tauri/target/release/bundle/macos/VibeHub.app`
- `agent_reported`: `codesign --verify --deep --strict --verbose=2 src-tauri/target/release/bundle/macos/VibeHub.app`
- `agent_reported`: `open -n src-tauri/target/release/bundle/macos/VibeHub.app`
- `agent_reported`: `pgrep -fl VibeHub`
- `agent_reported`: `./start-dev.sh --no-install`
- `agent_reported`: `pkill -f "target/debug/vibehub|vite"`
- `agent_reported`: `./start-dev.sh --check`
- `agent_reported`: `git diff --check`
- `agent_reported`: `git diff --stat`
- `agent_reported`: `git remote -v`
- `agent_reported`: `git status --short --branch`
- `agent_reported`: `python3 -m json.tool src/locales/zh.json`
- `agent_reported`: `python3 -m json.tool src/locales/en.json`
- `agent_reported`: `python3 -m json.tool src/locales/zh-TW.json`
- `agent_reported`: `cargo test settings_import --locked`
- `agent_reported`: `cargo test adapts --locked`
- `agent_reported`: `cargo test merge_tags --locked`

## Tests Run
- `hard_observed`: `npm run build` passed.
- `hard_observed`: `cargo fmt --all -- --check` passed after formatting.
- `hard_observed`: `cargo check --locked` passed with existing unused/dead-code warnings in gateway cache/stats, locale detection, and research-pack helpers.
- `hard_observed`: `npm run tauri build` passed when run with the required macOS escalation for DMG creation; it produced `src-tauri/target/release/bundle/macos/VibeHub.app` and `src-tauri/target/release/bundle/dmg/VibeHub_2.0.0-pre.2_aarch64.dmg`.
- `hard_observed`: Local ad-hoc codesign verification passed for `VibeHub.app`.
- `hard_observed`: `./start-dev.sh --no-install` launched the dev app; logs showed Vite serving locally and gateway listeners bound to `127.0.0.1`.
- `hard_observed`: `./start-dev.sh --check` passed after the mac native window and terminal-tag changes.
- `hard_observed`: `npm run tauri -- build --bundles app` passed after regenerating icons and produced `src-tauri/target/release/bundle/macos/VibeHub.app` with `Contents/Resources/icon.icns`.
- `hard_observed`: `cargo test adapts --locked` passed: 3 tests for terminal/executable import normalization.
- `hard_observed`: `cargo test merge_tags --locked` passed: 1 test for tag merge/update behavior.
- `hard_observed`: Locale JSON syntax validation passed for `en`, `zh`, and `zh-TW`.
- `hard_observed`: `./start-dev.sh --check` passed after adding settings import/export.
- `hard_observed`: `git diff --check` passed.
- `hard_observed`: Homebrew workflow and release/README wording were rechecked after the prerelease/Homebrew updates.

## Context Still Needed
- `inferred`: To validate signed public distribution end to end, repository secrets for Apple signing/notarization and the Homebrew tap token are still needed.
- `inferred`: To validate Warp/OpenCode/Claude Code/AMP UX end to end, those apps/CLIs must be installed on the target mac or Windows machine and available in the user shell.
- `inferred`: A native Windows runtime smoke test is still needed on an actual Windows host; this macOS pass verified the non-Windows code path by build/check and inspected Windows-only launcher code for conditional compile isolation.
- `inferred`: A UI click-through smoke test for import/export dialogs should be run in the packaged app because file picker behavior depends on the host desktop environment.

## Warnings
- `hard_observed`: `npm ci` reported 6 dependency vulnerabilities: 4 moderate and 2 high.
- `hard_observed`: `cargo check --locked` emits existing dead-code warnings unrelated to this mac launcher/storage pass.
- `hard_observed`: `npm run tauri build` failed without escalation at the DMG step because `hdiutil`/macOS bundle creation requires host permissions; the escalated run succeeded.
- `inferred`: Warp launch uses a temporary `.command` handoff file in the system temp directory and removes it after a delay; if Warp opens very slowly, the user can rerun the launch.
- `inferred`: Windows Terminal support falls back to the previous Command Prompt launch path when `wt.exe` is unavailable.
- `inferred`: Import replaces the AI gateway configuration, including provider API keys if present in the JSON export; users should treat exported JSON as sensitive.

## Next Session Should
- `agent_reported`: Validate a real release publication after Apple signing secrets and `HOMEBREW_TAP_TOKEN` are configured.
- `agent_reported`: Test the new CLI tag flow on a mac with Warp, OpenCode, Claude Code, and AMP installed, and on Windows with Windows Terminal, PowerShell, and Command Prompt.
- `agent_reported`: Smoke test Settings > General export/import in a packaged app and confirm the adjustment report is clear with a Windows-origin and macOS-origin JSON file.
- `agent_reported`: Consider adding tag presets for common CLI agents if repeated manual setup feels too slow.

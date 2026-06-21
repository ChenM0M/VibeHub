// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_paths;
mod commands;
mod gateway;
mod launcher;
mod local_agent_usage;
mod models;
mod process_util;
mod scanner;
mod storage;
mod updater;
mod vibehub;

use commands::AppState;
use std::{
    fs,
    io::{self, Read},
    sync::Mutex,
};
use storage::Storage;
use tauri::Manager;

/// Compiled-in version, used by the headless `--version` / `--help` output.
const VER: &str = env!("CARGO_PKG_VERSION");

fn main() {
    if run_vibehub_cli_if_requested() {
        return;
    }

    let storage = Storage::new().expect("Failed to initialize storage");
    replay_pending_events_for_known_projects(&storage);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            configure_platform_window(app);
            gateway::init(app.handle());
            Ok(())
        })
        .manage(AppState {
            storage: Mutex::new(storage),
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_config,
            commands::save_config,
            commands::export_settings_bundle,
            commands::import_settings_bundle,
            commands::scan_workspace,
            commands::add_workspace,
            commands::remove_workspace,
            commands::update_project,
            commands::refresh_project,
            commands::delete_project,
            commands::add_tag,
            commands::update_tag,
            commands::delete_tag,
            commands::launch_tool,
            commands::launch_custom,
            commands::open_in_explorer,
            commands::open_terminal,
            commands::get_storage_info,
            commands::set_custom_data_dir,
            commands::clear_custom_data_dir,
            commands::dismiss_storage_migration_notice,
            commands::dismiss_storage_custom_dir_notice,
            commands::record_project_open,
            commands::toggle_project_star,
            commands::initialize_default_configs,
            commands::set_theme,
            commands::refresh_all_workspaces,
            commands::check_for_updates,
            commands::vibehub_init,
            commands::vibehub_start_task,
            commands::vibehub_start_task_intake,
            commands::vibehub_build_context_pack,
            commands::vibehub_generate_agent_view,
            commands::vibehub_sync_agent_adapter,
            commands::vibehub_get_agent_adapter_status,
            commands::vibehub_update_agent_adapter_config,
            commands::vibehub_sync_agent_adapters,
            commands::vibehub_check_workspace_drift,
            commands::vibehub_sync_workspace_state,
            commands::vibehub_sync_workspace,
            commands::vibehub_workflow_explain,
            commands::vibehub_switch_task,
            commands::vibehub_classify_file_ownership,
            commands::vibehub_record_file_ownership,
            commands::vibehub_query_task_neighbors,
            commands::vibehub_claim_capability,
            commands::vibehub_evaluate_capability_gates,
            commands::vibehub_validate_capability_output,
            commands::vibehub_write_capability_output,
            commands::vibehub_replay_pending_events,
            commands::vibehub_debug_dump,
            commands::vibehub_build_handoff,
            commands::vibehub_generate_review_evidence,
            commands::vibehub_read_overview,
            commands::vibehub_read_local_agent_usage,
            commands::vibehub_read_project_digest,
            commands::vibehub_list_prompt_templates,
            commands::vibehub_render_prompt,
            commands::vibehub_append_journal_entry,
            commands::vibehub_append_knowledge_note,
            commands::vibehub_validate_phase,
            commands::vibehub_set_phase_result,
            commands::vibehub_complete_phase,
            commands::vibehub_advance_phase,
            commands::vibehub_pause_phase,
            commands::vibehub_build_research_pack,
            commands::vibehub_archive_research,
            commands::vibehub_read_vibehub_file,
            commands::vibehub_reveal_vibehub_file,
            commands::vibehub_open_vibehub_file,
            commands::vibehub_reveal_project_file,
            commands::vibehub_open_project_file,
            commands::vibehub_dry_run_state_migration,
            commands::vibehub_migrate_state,
            commands::vibehub_set_project_locale,
            gateway::get_gateway_config,
            gateway::save_gateway_config,
            gateway::get_gateway_stats,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn configure_platform_window(app: &mut tauri::App) {
    #[cfg(target_os = "macos")]
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_decorations(true);
        let _ = window.set_title_bar_style(tauri::TitleBarStyle::Overlay);
        let _ = window.set_title("VibeHub");
    }
}

fn replay_pending_events_for_known_projects(storage: &Storage) {
    let Ok(config) = storage.load_config() else {
        return;
    };
    for project in config.projects {
        let _ = vibehub::events::replay_pending_events(project.path);
    }
}

fn run_vibehub_cli_if_requested() -> bool {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        return false;
    };

    match command.as_str() {
        "vibehub" | "vibehub-cli" => {
            let Some(action) = args.next() else {
                eprintln!("Missing VibeHub action. Expected start, start-intake, continue, switch, ownership, record, neighbors, claim, gates, sync, status, next-action, output-lint, replay-pending, debug-dump, review, recover, handoff, pause, validate, validate-task, advance, finish, archive, workflow-explain, schema-check, sync-adapters, adapter-status, migrate, or locale.");
                std::process::exit(2);
            };
            run_vibehub_action(&action, args.collect());
            true
        }
        "start" | "start-task" | "start_task" | "start-intake" | "start_intake" | "continue"
        | "sync" | "sycn" | "status" | "next-action" | "next_action" | "route" | "output-lint"
        | "output_lint" | "lint-output" | "lint_output" | "replay-pending" | "pending-replay"
        | "debug-dump" | "vibehub-debug-dump" | "review" | "recover" | "handoff" | "validate"
        | "validate-task" | "validate_task" | "advance" | "finish" | "pause" | "switch"
        | "ownership" | "record" | "neighbors" | "claim" | "gates" | "archive"
        | "workflow-explain" | "workflow_explain" | "schema-check" | "schema_check"
        | "sync-adapters" | "adapter-sync" | "adapter-status" | "adapters-status" | "locale"
        | "migrate" => {
            run_vibehub_action(&command, args.collect());
            true
        }
        "--vibehub-sync-workspace" => {
            let Some(project_path) = args.next() else {
                eprintln!("Missing project path for --vibehub-sync-workspace");
                std::process::exit(2);
            };
            print_json(vibehub::sync::sync_workspace(&project_path));
            true
        }
        // CLI discovery flags: agents and users probe with --help/-h/--version.
        // Never fall through to the GUI for these, or repeated probing spawns
        // many windows.
        "--help" | "-h" | "help" => {
            eprintln!(
                "VibeHub {VER}\n\n\
                 This binary is both a desktop app (no args) and a headless CLI.\n\
                 Headless usage: vibehub <command> <project_path> [args...]\n\n\
                 Common commands: status, sync, next-action, start, validate,\n\
                 finish, advance, recover, handoff, claim, gates, archive.\n\n\
                 Run `vibehub help` or consult the repo SKILL docs for the full list."
            );
            true
        }
        "--version" | "-V" => {
            println!("{VER}");
            true
        }
        // Any other leading token. If it looks like a CLI invocation (starts
        // with '-'), treat it as CLI and error out instead of silently
        // launching the GUI window. This stops agents from spawning windows
        // when they mistype a command or pass an unknown flag.
        other if other.starts_with('-') => {
            eprintln!(
                "Unknown VibeHub command or flag: '{other}'\n\
                 Headless usage: vibehub <command> <project_path> [args...]\n\
                 Run `vibehub help` for the command list. \
                 (The GUI launches only with no arguments.)"
            );
            std::process::exit(2);
        }
        _ => false,
    }
}

fn run_vibehub_action(action: &str, args: Vec<String>) {
    let Some(project_path) = args.first() else {
        eprintln!("Missing project path for VibeHub action '{action}'");
        std::process::exit(2);
    };

    match action {
        "start" | "start-task" | "start_task" => {
            let mode = args.get(1).cloned();
            let title = if args.len() > 2 {
                Some(args[2..].join(" "))
            } else {
                None
            };
            print_json(vibehub::start_task::start_task(
                project_path,
                title,
                mode,
                None,
            ));
        }
        "start-intake" | "start_intake" => {
            let Some(request_path) = args.get(1) else {
                eprintln!("Missing JSON request path for VibeHub action 'start-intake' (use --stdin or - to read from stdin)");
                std::process::exit(2);
            };
            let (content, request_label) = read_intake_request_content(request_path);
            let request = match serde_json::from_str::<
                vibehub::start_task::VibehubStartTaskIntakeRequest,
            >(&content)
            {
                Ok(request) => request,
                Err(error) => {
                    eprintln!("Invalid intake request JSON '{}': {error}", request_label);
                    std::process::exit(2);
                }
            };
            print_json(vibehub::start_task::start_task_intake(
                project_path,
                request,
            ));
        }
        "continue" | "sync" | "sycn" => print_json(vibehub::sync::sync_workspace(project_path)),
        "status" => print_json(vibehub::status::read_cockpit_status(project_path)),
        "next-action" | "next_action" | "route" => {
            let intent = (!args[1..].is_empty()).then(|| args[1..].join(" "));
            print_json(vibehub::next_action::recommend_next_action_with_intent(
                project_path,
                intent.as_deref(),
            ))
        }
        "adapter-status" | "adapters-status" => print_json(
            vibehub::agent_adapter::get_agent_adapter_status(project_path),
        ),
        "sync-adapters" | "adapter-sync" => {
            let dry_run = args.iter().any(|arg| arg == "--dry-run");
            let tools = parse_agent_tools(&args[1..]);
            print_json(vibehub::agent_adapter::sync_agent_adapters(
                project_path,
                tools,
                dry_run,
            ));
        }
        "replay-pending" | "pending-replay" => {
            print_json(vibehub::events::replay_pending_events(project_path))
        }
        "debug-dump" | "vibehub-debug-dump" => print_json(vibehub::debug_dump::create_debug_dump(
            project_path,
            Some(parse_debug_dump_options(&args[1..])),
        )),
        "review" => {
            let locale = args.get(1).cloned();
            print_json(vibehub::review::generate_review_evidence_with_locale(
                project_path,
                locale.as_deref(),
            ));
        }
        "recover" => print_json(vibehub::drift::sync_workspace_state(project_path)),
        "handoff" => print_json(vibehub::handoff::build_handoff(project_path)),
        // CLI `pause` builds a handoff AND marks the current phase blocked, so
        // a later `vibehub status` / cockpit overview reflects that work was
        // intentionally stopped (not just dormant).
        "pause" => {
            let handoff = vibehub::handoff::build_handoff(project_path);
            let pause = vibehub::phase::pause_current_phase(project_path);
            #[derive(serde::Serialize)]
            struct PauseReport {
                handoff: Option<vibehub::handoff::HandoffBuildResult>,
                phase: Option<vibehub::phase::PhaseSetResult>,
                handoff_error: Option<String>,
                phase_error: Option<String>,
            }
            let report = PauseReport {
                handoff_error: handoff.as_ref().err().map(|e| e.to_string()),
                handoff: handoff.ok(),
                phase_error: pause.as_ref().err().map(|e| e.to_string()),
                phase: pause.ok(),
            };
            print_json(Ok::<_, anyhow::Error>(report));
        }
        "validate" => print_json(vibehub::phase::validate_phase(project_path)),
        "validate-task" | "validate_task" => {
            let Some(task_id) = args.get(1) else {
                eprintln!("Missing task_id for VibeHub validate-task action");
                std::process::exit(2);
            };
            print_json(vibehub::phase::validate_phase_for_task(
                project_path,
                task_id,
            ));
        }
        "output-lint" | "output_lint" | "lint-output" | "lint_output" => {
            let task_id = args.get(1).map(String::as_str);
            print_json(vibehub::output_lint::lint_output_for_task(
                project_path,
                task_id,
            ));
        }
        "advance" => {
            require_user_confirmation("advance", &args);
            let force = args.iter().any(|a| a == "--force");
            print_json(vibehub::phase::advance_phase_with_force(
                project_path,
                force,
            ));
        }
        "finish" => {
            require_user_confirmation("finish", &args);
            print_json(vibehub::phase::complete_phase(project_path));
        }
        "workflow-explain" | "workflow_explain" => {
            print_json(vibehub::workflow::explain_workflow(project_path))
        }
        "switch" => {
            let Some(task_id) = args.get(1) else {
                eprintln!("Missing task_id for VibeHub switch action");
                std::process::exit(2);
            };
            print_json(vibehub::task_switch::switch_task(project_path, task_id));
        }
        "ownership" => {
            let changed_files = if args.len() > 1 {
                Some(args[1..].to_vec())
            } else {
                None
            };
            print_json(vibehub::ownership::classify_workspace_ownership(
                project_path,
                changed_files,
            ));
        }
        "record" => {
            if args.len() < 2 {
                eprintln!("Missing scope files for VibeHub record action");
                std::process::exit(2);
            }
            print_json(vibehub::ownership::record_file_ownership(
                project_path,
                vibehub::ownership::FileOwnershipRecordRequest {
                    task_id: None,
                    run_id: None,
                    capability: None,
                    scope_files: args[1..].to_vec(),
                },
            ));
        }
        "neighbors" => print_json(vibehub::neighbors::query_current_task_neighbors(
            project_path,
        )),
        "claim" => {
            let Some(capability) = args.get(1) else {
                eprintln!("Missing capability for VibeHub claim action");
                std::process::exit(2);
            };
            print_json(vibehub::capability::claim_capability(
                project_path,
                capability,
            ));
        }
        "gates" => {
            let requested_capability = args.get(1).map(String::as_str);
            print_json(vibehub::capability::evaluate_capability_gates(
                project_path,
                requested_capability,
            ));
        }
        "archive" => {
            require_user_confirmation("archive", &args);
            let target_task_id = args
                .iter()
                .skip(1)
                .find(|arg| !is_confirmation_flag(arg) && arg.as_str() != "--force")
                .map(String::as_str);
            print_json(vibehub::archive::archive_completed_tasks(
                project_path,
                target_task_id,
            ));
        }
        "schema-check" | "schema_check" => {
            let Some(capability) = args.get(1) else {
                eprintln!("Missing capability for VibeHub schema-check action");
                std::process::exit(2);
            };
            let Some(output_path) = args.get(2) else {
                eprintln!("Missing output JSON path for VibeHub schema-check action");
                std::process::exit(2);
            };
            print_json(
                vibehub::schema_check::validate_current_capability_output_file(
                    project_path,
                    capability,
                    output_path,
                ),
            );
        }
        "migrate" => {
            let dry_run = args.iter().any(|a| a == "--dry-run");
            if dry_run {
                print_json(vibehub::state_migration::dry_run(project_path));
            } else {
                print_json(vibehub::state_migration::migrate(project_path));
            }
        }
        "locale" => {
            let locale = args.get(1).cloned().unwrap_or_else(|| {
                eprintln!("Missing locale argument. Expected: en, zh-CN, zh-TW");
                std::process::exit(2);
            });
            match vibehub::locale::persist_project_locale(
                std::path::Path::new(project_path),
                &locale,
            ) {
                Ok(()) => println!("{{\"locale\": \"{}\"}}", locale),
                Err(e) => {
                    eprintln!("{}", short_error(&e));
                    std::process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("Unknown VibeHub action '{action}'");
            std::process::exit(2);
        }
    }
}

fn read_intake_request_content(request_path: &str) -> (String, String) {
    if matches!(request_path, "--stdin" | "-") {
        let mut content = String::new();
        if let Err(error) = io::stdin().read_to_string(&mut content) {
            eprintln!("Failed to read intake request from stdin: {error}");
            std::process::exit(2);
        }
        return (content, "stdin".to_string());
    }

    match fs::read_to_string(request_path) {
        Ok(content) => (content, request_path.to_string()),
        Err(error) => {
            eprintln!("Failed to read intake request '{}': {error}", request_path);
            std::process::exit(2);
        }
    }
}

fn print_json<T>(result: anyhow::Result<T>)
where
    T: serde::Serialize,
{
    match result.and_then(|value| serde_json::to_string_pretty(&value).map_err(Into::into)) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            // Only print the top-level error message; do NOT leak the full
            // anyhow chain (with cause stack and source paths) to stderr.
            // Operators who need the full chain can still get it by enabling
            // RUST_LOG=debug or running the GUI; the CLI's contract is a
            // single concise line so it's safe to copy/paste into chat.
            eprintln!("{}", short_error(&error));
            std::process::exit(1);
        }
    }
}

/// Render an `anyhow::Error` as its top-level message without the cause chain
/// (the `{:#}` formatter would include every wrapped source, which we don't
/// want to dump into stderr — see the print_json doc above).
fn short_error(error: &anyhow::Error) -> String {
    error.to_string()
}

fn require_user_confirmation(action: &str, args: &[String]) {
    if args.iter().any(|arg| is_confirmation_flag(arg)) {
        return;
    }
    eprintln!(
        "Refusing to run state-changing VibeHub action '{action}' without explicit confirmation. Re-run with --confirmed-by-user after the user has confirmed."
    );
    std::process::exit(2);
}

fn is_confirmation_flag(arg: &str) -> bool {
    matches!(arg, "--confirmed-by-user" | "--user-confirmed" | "--yes")
}

fn parse_agent_tools(args: &[String]) -> Option<Vec<vibehub::agent_adapter::AgentTool>> {
    let tools = args
        .iter()
        .filter(|arg| arg.as_str() != "--dry-run")
        .filter_map(|arg| match arg.as_str() {
            "amp" | "amp-code" | "amp_code" => Some(vibehub::agent_adapter::AgentTool::AmpCode),
            "codex" => Some(vibehub::agent_adapter::AgentTool::Codex),
            "claude" | "claude-code" | "claude_code" => {
                Some(vibehub::agent_adapter::AgentTool::ClaudeCode)
            }
            "opencode" | "open-code" | "open_code" => {
                Some(vibehub::agent_adapter::AgentTool::Opencode)
            }
            "cursor" => Some(vibehub::agent_adapter::AgentTool::Cursor),
            "antigravity" => Some(vibehub::agent_adapter::AgentTool::Antigravity),
            unknown => {
                eprintln!("Unknown adapter tool '{unknown}'");
                std::process::exit(2);
            }
        })
        .collect::<Vec<_>>();
    if tools.is_empty() {
        None
    } else {
        Some(tools)
    }
}

fn parse_debug_dump_options(args: &[String]) -> vibehub::debug_dump::DebugDumpOptions {
    vibehub::debug_dump::DebugDumpOptions {
        include_events: args.iter().any(|a| a == "--no-events").then_some(false),
        include_packs: args.iter().any(|a| a == "--no-packs").then_some(false),
        redact_secrets: args.iter().any(|a| a == "--no-redact").then_some(false),
    }
}

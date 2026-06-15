use std::fs;
use std::io::{self, Read};

use vibehub_core::vibehub;

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(action) = args.next() else {
        print_help();
        std::process::exit(2);
    };

    if matches!(action.as_str(), "--help" | "-h" | "help") {
        print_help();
        return;
    }

    let rest: Vec<String> = args.collect();
    run_vibehub_action(&action, rest);
}

fn print_help() {
    eprintln!(
        r#"vibehub-cli <action> <project_path> [args...]

=== Task Lifecycle ===
  start                  Create a new task: vibehub-cli start <project> <mode> <title>
  start-intake           Create multiple tasks from JSON: vibehub-cli start-intake <project> <json_path|--stdin|->
  switch                 Switch active task: vibehub-cli switch <project> <task_id>
  finish                 Complete current phase: vibehub-cli finish <project> --confirmed-by-user
  advance                Advance to next phase: vibehub-cli advance <project> --confirmed-by-user [--force]
  validate               Validate current phase outputs: vibehub-cli validate <project>
  validate-task          Validate a task without switching: vibehub-cli validate-task <project> <task_id>
  output-lint            Lint current output quality: vibehub-cli output-lint <project> [task_id]
  pause                  Pause current phase: vibehub-cli pause <project>
  archive                Archive completed tasks: vibehub-cli archive <project> --confirmed-by-user [task_id]

=== Workspace Sync ===
  sync / sycn            Sync workspace state: vibehub-cli sync <project>
  recover                Check workspace drift: vibehub-cli recover <project>
  status                 Show cockpit status: vibehub-cli status <project>
  next-action            Recommend next agent action: vibehub-cli next-action <project> [intent...]

=== Capability & Gates ===
  claim                  Claim a capability: vibehub-cli claim <project> <capability>
  gates                  Evaluate capability gates: vibehub-cli gates <project> [capability]

=== Context & Evidence ===
  review                 Generate review evidence: vibehub-cli review <project>
  handoff                Build handoff: vibehub-cli handoff <project>
  ownership              Classify file ownership: vibehub-cli ownership <project> [files...]
  record                 Record file ownership: vibehub-cli record <project> <files...>
  schema-check           Validate capability output: vibehub-cli schema-check <project> <capability> <json_path>
  neighbors              Query neighbor tasks: vibehub-cli neighbors <project>
  workflow-explain       Explain workflow: vibehub-cli workflow-explain <project>

=== Adapter Management ===
  sync-adapters          Sync adapter files: vibehub-cli sync-adapters <project> [tools...] [--dry-run]
  adapter-status         Show adapter file status: vibehub-cli adapter-status <project>

=== Maintenance ===
  migrate                Migrate state schema: vibehub-cli migrate <project> [--dry-run]
  replay-pending         Replay pending events: vibehub-cli replay-pending <project>
  debug-dump             Create debug dump: vibehub-cli debug-dump <project>
  locale                 Set project locale: vibehub-cli locale <project> <en|zh-CN|zh-TW>

All commands output JSON to stdout. Errors go to stderr.
Use `vibehub-cli --help` to see this message again.
"#
    );
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
        "replay-pending" | "pending-replay" => {
            print_json(vibehub::events::replay_pending_events(project_path))
        }
        "debug-dump" | "vibehub-debug-dump" | "vibehub-cli-debug-dump" => {
            print_json(vibehub::debug_dump::create_debug_dump(
                project_path,
                Some(parse_debug_dump_options(&args[1..])),
            ))
        }
        "sync-adapters" | "adapter-sync" => {
            let dry_run = args.iter().any(|a| a == "--dry-run");
            let tools = parse_agent_tools(&args[1..]);
            print_json(vibehub::agent_adapter::sync_agent_adapters(
                project_path,
                tools,
                dry_run,
            ));
        }
        "review" => {
            let locale = args.get(1).cloned();
            print_json(vibehub::review::generate_review_evidence_with_locale(
                project_path,
                locale.as_deref(),
            ));
        }
        "recover" => print_json(vibehub::drift::sync_workspace_state(project_path)),
        "handoff" => print_json(vibehub::handoff::build_handoff(project_path)),
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
            eprintln!("{}", short_error(&error));
            std::process::exit(1);
        }
    }
}

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
        .map(|arg| match arg.as_str() {
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
        .flatten()
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

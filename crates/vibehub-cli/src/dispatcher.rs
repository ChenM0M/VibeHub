use std::fs;
use std::io::{self, Read};

use vibehub_core::v3::{
    route_session_task, AgentSpecSyncRequest, BindingSource, LifecycleCommand, MemoryCommand,
    MemoryQuery, PlanAddNodeCommand, PlanSetCriteriaCommand, PlanSetDependenciesCommand,
    PlanSetStateCommand, RouteRequest, TaskRouteCandidate, V3ApplicationService,
    V3TaskCreateRequest, V3ViewRepository,
};
use vibehub_core::vibehub;

pub enum DispatchOutcome {
    Handled,
    NotHandled,
}

pub fn dispatch<I>(args: I, version: &str) -> DispatchOutcome
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();
    let Some(action) = args.next() else {
        return DispatchOutcome::NotHandled;
    };

    if matches!(action.as_str(), "--help" | "-h" | "help") {
        print_help();
        return DispatchOutcome::Handled;
    }

    if matches!(action.as_str(), "--version" | "-V") {
        println!("{version}");
        return DispatchOutcome::Handled;
    }

    if matches!(action.as_str(), "vibehub" | "vibehub-cli") {
        let Some(action) = args.next() else {
            eprintln!("Missing VibeHub action. Expected start, start-intake, sync, status, next-action, output-lint, validate, sync-adapters, adapter-status, or another VibeHub command.");
            std::process::exit(2);
        };
        run_vibehub_action(&action, args.collect());
        return DispatchOutcome::Handled;
    }

    if action == "--vibehub-sync-workspace" {
        let Some(project_path) = args.next() else {
            eprintln!("Missing project path for --vibehub-sync-workspace");
            std::process::exit(2);
        };
        print_json(vibehub::sync::sync_workspace(&project_path));
        return DispatchOutcome::Handled;
    }

    if action.starts_with('-') {
        eprintln!(
            "Unknown VibeHub command or flag: '{action}'\n\
             Headless usage: vibehub <command> <project_path> [args...]\n\
             Run `vibehub help` for the command list."
        );
        std::process::exit(2);
    }

    if is_action(&action) {
        run_vibehub_action(&action, args.collect());
        DispatchOutcome::Handled
    } else {
        DispatchOutcome::NotHandled
    }
}

pub fn print_help() {
    eprintln!(
        r#"vibehub <action> <project_path> [args...]
vibehub-cli <action> <project_path> [args...]   (legacy alias)

=== Task Lifecycle ===
  start                  Create a new task: vibehub start <project> <mode> <title>
  start-intake           Create multiple tasks from JSON: vibehub start-intake <project> <json_path|--stdin|->
  switch                 Switch active task: vibehub switch <project> <task_id>
  finish                 Complete current phase: vibehub finish <project> --confirmed-by-user
  advance                Advance to next phase: vibehub advance <project> --confirmed-by-user [--force]
  validate               Validate current phase outputs: vibehub validate <project>
  validate-task          Validate a task without switching: vibehub validate-task <project> <task_id>
  output-lint            Lint current output quality: vibehub output-lint <project> [task_id]
  pause                  Pause current phase: vibehub pause <project>
  archive                Archive completed tasks: vibehub archive <project> --confirmed-by-user [task_id]

=== Workspace Sync ===
  sync / sycn            Sync workspace state: vibehub sync <project>
  recover                Check workspace drift: vibehub recover <project>
  status                 Show cockpit status: vibehub status <project>
  next-action            Recommend next agent action: vibehub next-action <project> [intent...]

=== Capability & Gates ===
  claim                  Claim a capability: vibehub claim <project> <capability>
  gates                  Evaluate capability gates: vibehub gates <project> [capability]

=== Context & Evidence ===
  review                 Generate review evidence: vibehub review <project>
  handoff                Build handoff: vibehub handoff <project>
  ownership              Classify file ownership: vibehub ownership <project> [files...]
  record                 Record file ownership: vibehub record <project> <files...>
  schema-check           Validate capability output: vibehub schema-check <project> <capability> <json_path>
  neighbors              Query neighbor tasks: vibehub neighbors <project>
  workflow-explain       Explain workflow: vibehub workflow-explain <project>

=== Adapter Management ===
  sync-adapters          Sync adapter files: vibehub sync-adapters <project> [tools...] [--dry-run]
  adapter-status         Show adapter file status: vibehub adapter-status <project>
  mcp-install            Wire VibeHub MCP (project-scoped): vibehub mcp-install <project> [--dry-run] [--migrate-global]
  mcp-status             Show MCP wiring status per harness: vibehub mcp-status <project>

=== Maintenance ===
  migrate                Migrate state schema: vibehub migrate <project> [--dry-run]
  replay-pending         Replay pending events: vibehub replay-pending <project>
  debug-dump             Create debug dump: vibehub debug-dump <project>
  locale                 Set project locale: vibehub locale <project> <en|zh-CN|zh-TW>

=== V3 Core (CLI fallback) ===
  v3 <project> doctor
  v3 <project> init
  v3 <project> migrate
  v3 <project> migrate-recover
  v3 <project> repair-candidates
  v3 <project> repair <task_id>
  v3 <project> task-create <request_json_path|--stdin|->
  v3 <project> task-preflight <request_json_path|--stdin|->
  v3 <project> quarantine-task <task_id>
  v3 <project> agent-specs-status
  v3 <project> agent-specs-sync [--force-managed-region]
  v3 <project> session-open <project_id> <task_id> <session_id> <actor> <expected_version> <idempotency_key> [working_directory] [node_id|-] [worktree_id|-] [provider] [provider_session_id|-]
  v3 <project> session-bind <project_id> <task_id> <session_id> <interaction_id> <actor> <expected_version> <idempotency_key> <source> [expected_binding_revision|-] [agent_id|-] [host|-]
  v3 <project> session-unbind <project_id> <task_id> <session_id> <actor> <expected_version> <idempotency_key> [expected_binding_revision|-]
  v3 <project> event-log <project_id> <task_id> <session_id> <actor> <expected_version> <idempotency_key> <progress|risk> <details_json>
  v3 <project> agent-result <project_id> <task_id> <session_id> <actor> <expected_version> <idempotency_key> <result_id> <node_id|-> <details_json>
  v3 <project> session-close <project_id> <task_id> <session_id> <actor> <expected_version> <idempotency_key>
  v3 <project> criterion-review <project_id> <task_id> <actor> <expected_version> <idempotency_key> <criterion_id> <passed|failed|blocked> <reviewer> <evidence_refs_json> [details_json]
  v3 <project> task-completion-propose <project_id> <task_id> <actor> <expected_version> <idempotency_key>
  v3 <project> task-complete <project_id> <task_id> <actor> <confirmed_by> <cli|desktop_ui> <idempotency_key> --confirmed-by-user
  v3 <project> rebuild <project_id>
  v3 <project> projection-status <project_id>
  v3 <project> view-bundle <task_id>
  v3 <project> task-lifecycle <project_id> <task_id>
  v3 <project> task-candidates
  v3 <project> task-list [--include-archived]
  v3 <project> task-route <request_json_path|--stdin|->
  v3 <project> task-view <task_id>
  v3 <project> task-view <task_id> [node_id]
  v3 <project> task-commits <task_id>
  v3 <project> commit-tasks <commit_hash>
  v3 <project> policy-upgrade <project_id> <task_id> <actor> <expected_version> <idempotency_key> <target_profile> <reason>
  v3 <project> session-gap <project_id> <task_id> <session_id> <actor> <expected_version> <idempotency_key> <reason>
  v3 <project> session-recover <project_id> <task_id> <session_id> <actor> <expected_version> <idempotency_key> <evidence_refs_json>
  v3 <project> memory-command <command_json_path|--stdin|->
  v3 <project> memory-query <project_id> <query_json>
  v3 <project> finding-command <lifecycle_command_json_path|--stdin|->
  v3 <project> attempt-command <lifecycle_command_json_path|--stdin|->
  v3 <project> plan-event <command_json_path|--stdin|->
  v3 <project> lifecycle-event <command_json_path|--stdin|->
  v3 <project> worktree-event <command_json_path|--stdin|->
  v3 <project> worktree-orchestration <project_id> <task_id>

All commands output JSON to stdout. Errors go to stderr.
Use `vibehub --help` or `vibehub-cli --help` to see this message again.
"#
    );
}

fn is_action(action: &str) -> bool {
    matches!(
        action,
        "mcp-stdio"
            | "v3"
            | "start"
            | "start-task"
            | "start_task"
            | "start-intake"
            | "start_intake"
            | "continue"
            | "sync"
            | "sycn"
            | "status"
            | "next-action"
            | "next_action"
            | "route"
            | "adapter-status"
            | "adapters-status"
            | "replay-pending"
            | "pending-replay"
            | "debug-dump"
            | "vibehub-debug-dump"
            | "vibehub-cli-debug-dump"
            | "sync-adapters"
            | "adapter-sync"
            | "review"
            | "recover"
            | "handoff"
            | "pause"
            | "validate"
            | "validate-task"
            | "validate_task"
            | "output-lint"
            | "output_lint"
            | "lint-output"
            | "lint_output"
            | "advance"
            | "finish"
            | "workflow-explain"
            | "workflow_explain"
            | "switch"
            | "ownership"
            | "record"
            | "neighbors"
            | "archive"
            | "claim"
            | "gates"
            | "schema-check"
            | "schema_check"
            | "migrate"
            | "locale"
            | "mcp-install"
            | "mcp_install"
            | "install-mcp"
            | "mcp-status"
            | "mcp_status"
    )
}

fn run_vibehub_action(action: &str, args: Vec<String>) {
    let Some(project_path) = args.first() else {
        eprintln!("Missing project path for VibeHub action '{action}'");
        std::process::exit(2);
    };

    match action {
        "mcp-stdio" => crate::mcp::run_stdio(project_path),
        "v3" => run_v3_action(project_path, &args[1..]),
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
        "mcp-install" | "mcp_install" | "install-mcp" => {
            let dry_run = args.iter().any(|a| a == "--dry-run");
            let migrate_global = args.iter().any(|a| a == "--migrate-global" || a == "--global");
            let tools = parse_agent_tools(&args[1..]);
            print_json(vibehub::agent_adapter::install_mcp_config(
                project_path,
                tools,
                dry_run,
                migrate_global,
            ))
        }
        "mcp-status" | "mcp_status" => {
            print_json(vibehub::agent_adapter::mcp_status(project_path))
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
            eprintln!(
                "Unknown VibeHub action '{action}'.\nNext step: `vibehub help` for the command list, or `vibehub next-action <project>` for the recommended action."
            );
            std::process::exit(2);
        }
    }
}

fn run_v3_action(project_root: &str, args: &[String]) {
    let Some(command) = args.first().map(String::as_str) else {
        eprintln!(
            "Missing V3 command. Run `vibehub help` and use a command from the V3 Core section."
        );
        std::process::exit(2);
    };
    match command {
        "doctor" => {
            print_v3_json(vibehub_core::v3::inspect_project_layout(project_root));
            return;
        }
        "init" => {
            print_v3_json(vibehub_core::v3::initialize_v3(project_root));
            return;
        }
        "migrate" => {
            print_v3_json(vibehub_core::v3::migrate_v2_to_v3(project_root));
            return;
        }
        "migrate-recover" => {
            print_v3_json(vibehub_core::v3::recover_interrupted_migration(
                project_root,
            ));
            return;
        }
        "repair" => {
            let Some(task_id) = args.get(1) else {
                v3_usage_error(command, "missing task_id");
            };
            print_v3_json(vibehub_core::v3::repair_v3_layout(project_root, task_id));
            return;
        }
        "repair-candidates" => {
            print_v3_json(vibehub_core::v3::inspect_v3_repair_candidates(project_root));
            return;
        }
        "task-create" => {
            let Some(json_path) = args.get(1) else {
                v3_usage_error(
                    command,
                    "missing task request JSON path (use --stdin or - to read from stdin)",
                );
            };
            let (content, label) = read_v3_command_content(json_path, "task create request");
            let request =
                serde_json::from_str::<V3TaskCreateRequest>(&content).unwrap_or_else(|error| {
                    eprintln!("Invalid task create JSON '{}': {error}", label);
                    std::process::exit(2);
                });
            print_v3_json(vibehub_core::v3::create_v3_task(project_root, request));
            return;
        }
        "task-preflight" => {
            let Some(json_path) = args.get(1) else {
                v3_usage_error(
                    command,
                    "missing task request JSON path (use --stdin or - to read from stdin)",
                );
            };
            let (content, label) = read_v3_command_content(json_path, "task preflight request");
            let mut request =
                serde_json::from_str::<V3TaskCreateRequest>(&content).unwrap_or_else(|error| {
                    eprintln!("Invalid task create JSON '{}': {error}", label);
                    std::process::exit(2);
                });
            // Preflight never writes: it reports the deterministic task id and whether an
            // equivalent task already exists, so probing cannot leave an invalid duplicate.
            request.preflight = true;
            print_v3_json(vibehub_core::v3::create_v3_task(project_root, request));
            return;
        }
        "quarantine-task" => {
            let Some(task_id) = args.get(1) else {
                v3_usage_error(command, "missing task_id");
            };
            print_v3_json(vibehub_core::v3::quarantine_v3_task(project_root, task_id));
            return;
        }
        "agent-specs-status" => {
            print_v3_json(vibehub_core::v3::inspect_agent_specs(project_root));
            return;
        }
        "agent-specs-sync" => {
            print_v3_json(vibehub_core::v3::sync_agent_specs(
                project_root,
                AgentSpecSyncRequest {
                    force_managed_region: args.iter().any(|arg| arg == "--force-managed-region"),
                    migrate_global: args.iter().any(|arg| arg == "--migrate-global"),
                },
            ));
            return;
        }
        _ => {}
    }
    let app = match V3ApplicationService::open(project_root) {
        Ok(app) => app,
        Err(error) => print_v3_error(error),
    };
    match command {
        "session-open" | "session-close" => {
            let (project_id, task_id, session_id, actor, expected_version, idempotency_key) =
                parse_v3_write_scope(command, &args[1..]);
            let result = if command == "session-open" {
                app.session_open_with_context_and_provider(
                    project_id,
                    task_id,
                    session_id,
                    actor,
                    expected_version,
                    idempotency_key,
                    optional_v3_value(args.get(7)),
                    optional_v3_value(args.get(8)),
                    optional_v3_value(args.get(9)),
                    optional_v3_value(args.get(10)),
                    optional_v3_value(args.get(11)),
                )
            } else {
                app.session_close(
                    project_id,
                    task_id,
                    session_id,
                    actor,
                    expected_version,
                    idempotency_key,
                )
            };
            print_v3_json(result);
        }
        "session-bind" => {
            if args.len() < 9 {
                v3_usage_error(command, "expected project_id task_id session_id interaction_id actor expected_version idempotency_key source");
            }
            let expected_version = args[6].parse::<u64>().unwrap_or_else(|error| {
                v3_usage_error(command, &format!("invalid expected_version: {error}"))
            });
            let source =
                serde_json::from_value::<BindingSource>(serde_json::Value::String(args[8].clone()))
                    .unwrap_or_else(|_| v3_usage_error(command, "invalid binding source"));
            let expected_binding_revision = args
                .get(9)
                .and_then(|value| optional_v3_value(Some(value)))
                .map(|value| {
                    value.parse::<u64>().unwrap_or_else(|error| {
                        v3_usage_error(
                            command,
                            &format!("invalid expected_binding_revision: {error}"),
                        )
                    })
                });
            let agent_id = optional_v3_value(args.get(10));
            let host = optional_v3_value(args.get(11));
            print_v3_json(app.session_task_bind(
                &args[1],
                &args[2],
                &args[3],
                &args[4],
                &args[5],
                source,
                expected_version,
                &args[7],
                expected_binding_revision,
                agent_id,
                host,
                None,
            ));
        }
        "session-unbind" => {
            if args.len() < 7 {
                v3_usage_error(
                    command,
                    "expected project_id task_id session_id actor expected_version idempotency_key",
                );
            }
            let expected_version = args[5].parse::<u64>().unwrap_or_else(|error| {
                v3_usage_error(command, &format!("invalid expected_version: {error}"))
            });
            let expected_binding_revision = args
                .get(7)
                .and_then(|value| optional_v3_value(Some(value)))
                .map(|value| {
                    value.parse::<u64>().unwrap_or_else(|error| {
                        v3_usage_error(
                            command,
                            &format!("invalid expected_binding_revision: {error}"),
                        )
                    })
                });
            print_v3_json(app.session_task_unbind(
                &args[1],
                &args[2],
                &args[3],
                &args[4],
                expected_version,
                &args[6],
                expected_binding_revision,
            ));
        }
        "event-log" => {
            let (project_id, task_id, session_id, actor, expected_version, idempotency_key) =
                parse_v3_write_scope(command, &args[1..]);
            let Some(kind) = args.get(7) else {
                v3_usage_error(command, "missing event kind");
            };
            let Some(details) = args.get(8) else {
                v3_usage_error(command, "missing details JSON");
            };
            let details = serde_json::from_str(details).unwrap_or_else(|error| {
                v3_usage_error(command, &format!("invalid details JSON: {error}"))
            });
            print_v3_json(app.event_log(
                kind,
                project_id,
                task_id,
                session_id,
                actor,
                expected_version,
                idempotency_key,
                details,
            ));
        }
        "agent-result" => {
            let (project_id, task_id, session_id, actor, expected_version, idempotency_key) =
                parse_v3_write_scope(command, &args[1..]);
            let Some(result_id) = args.get(7) else {
                v3_usage_error(command, "missing result_id");
            };
            let node_id = optional_v3_value(args.get(8));
            let Some(details) = args.get(9) else {
                v3_usage_error(command, "missing details JSON");
            };
            let details = serde_json::from_str(details).unwrap_or_else(|error| {
                v3_usage_error(command, &format!("invalid details JSON: {error}"))
            });
            print_v3_json(app.agent_result_record(
                project_id,
                task_id,
                session_id,
                actor,
                expected_version,
                idempotency_key,
                result_id,
                node_id,
                details,
            ));
        }
        "criterion-review" => {
            let (project_id, task_id, actor, expected_version, idempotency_key) =
                parse_v3_task_write_scope(command, &args[1..]);
            let Some(criterion_id) = args.get(6) else {
                v3_usage_error(command, "missing criterion_id");
            };
            let Some(outcome) = args.get(7) else {
                v3_usage_error(command, "missing review outcome");
            };
            let Some(reviewer) = args.get(8) else {
                v3_usage_error(command, "missing reviewer");
            };
            let Some(evidence_refs) = args.get(9) else {
                v3_usage_error(command, "missing evidence_refs JSON");
            };
            let evidence_refs =
                serde_json::from_str::<Vec<String>>(evidence_refs).unwrap_or_else(|error| {
                    v3_usage_error(command, &format!("invalid evidence_refs JSON: {error}"))
                });
            let details = args
                .get(10)
                .map(|value| {
                    serde_json::from_str(value).unwrap_or_else(|error| {
                        v3_usage_error(command, &format!("invalid details JSON: {error}"))
                    })
                })
                .unwrap_or(serde_json::Value::Null);
            print_v3_json(app.review_criterion(
                project_id,
                task_id,
                actor,
                expected_version,
                idempotency_key,
                criterion_id,
                outcome,
                reviewer,
                evidence_refs,
                details,
            ));
        }
        "task-completion-propose" => {
            let (project_id, task_id, actor, expected_version, idempotency_key) =
                parse_v3_task_write_scope(command, &args[1..]);
            print_v3_json(app.propose_task_completion(
                project_id,
                task_id,
                actor,
                expected_version,
                idempotency_key,
            ));
        }
        "task-complete" => {
            require_user_confirmation(command, &args[1..]);
            if args.len() < 7 {
                v3_usage_error(
                    command,
                    "expected project_id task_id actor confirmed_by channel idempotency_key --confirmed-by-user",
                );
            }
            print_v3_json(
                app.complete_task(&args[1], &args[2], &args[3], &args[4], &args[5], &args[6]),
            );
        }
        "rebuild" => {
            let Some(project_id) = args.get(1) else {
                v3_usage_error(command, "missing project_id");
            };
            print_v3_json(app.rebuild(project_id));
        }
        "projection-status" => {
            let Some(project_id) = args.get(1) else {
                v3_usage_error(command, "missing project_id");
            };
            print_v3_json(app.projection_status(project_id));
        }
        "view-bundle" => {
            let Some(task_id) = args.get(1) else {
                v3_usage_error(command, "missing task_id");
            };
            let repository = open_synced_v3_view_repository(&app, project_root);
            print_v3_json(repository.load_bundle(task_id));
        }
        "task-lifecycle" => {
            let Some(project_id) = args.get(1) else {
                v3_usage_error(command, "missing project_id");
            };
            let Some(task_id) = args.get(2) else {
                v3_usage_error(command, "missing task_id");
            };
            print_v3_json(app.task_lifecycle(project_id, task_id));
        }
        "task-candidates" => {
            let repository = open_synced_v3_view_repository(&app, project_root);
            print_v3_json(repository.task_candidates());
        }
        "task-list" => {
            let include_archived = args.iter().any(|arg| arg == "--include-archived");
            let repository = open_synced_v3_view_repository(&app, project_root);
            print_v3_json(repository.task_list(include_archived));
        }
        "task-route" => {
            let Some(json_path) = args.get(1) else {
                v3_usage_error(
                    command,
                    "missing route request JSON path (use --stdin or - to read from stdin)",
                );
            };
            let repository = open_synced_v3_view_repository(&app, project_root);
            let (content, label) = read_v3_command_content(json_path, "task route request");
            let mut request =
                serde_json::from_str::<RouteRequest>(&content).unwrap_or_else(|error| {
                    eprintln!("Invalid task route JSON '{}': {error}", label);
                    std::process::exit(2);
                });
            if request.candidates.is_empty() {
                request.candidates = serde_json::from_value::<Vec<TaskRouteCandidate>>(
                    repository
                        .task_candidates()
                        .unwrap_or_else(|error| print_v3_error(error)),
                )
                .unwrap_or_else(|error| {
                    print_v3_error(vibehub_core::v3::V3Error::new(
                        "V3_TASK_ROUTE_CANDIDATES_INVALID",
                        vibehub_core::v3::V3ErrorCategory::Internal,
                        false,
                        error.to_string(),
                    ))
                });
            }
            print_v3_json(Ok::<_, vibehub_core::v3::V3Error>(route_session_task(
                &request,
            )));
        }
        "task-view" => {
            let Some(task_id) = args.get(1) else {
                v3_usage_error(command, "missing task_id");
            };
            let repository = open_synced_v3_view_repository(&app, project_root);
            print_v3_json(
                repository.load_bundle_for_node(task_id, args.get(2).map(String::as_str)),
            );
        }
        "task-commits" => {
            let Some(task_id) = args.get(1) else {
                v3_usage_error(command, "missing task_id");
            };
            let repository = open_synced_v3_view_repository(&app, project_root);
            print_v3_json(repository.task_commits(task_id));
        }
        "commit-tasks" => {
            let Some(commit_hash) = args.get(1) else {
                v3_usage_error(command, "missing commit_hash");
            };
            let repository = open_synced_v3_view_repository(&app, project_root);
            print_v3_json(repository.commit_tasks(commit_hash));
        }
        "policy-upgrade" => {
            let (project_id, task_id, actor, version, key) =
                parse_v3_task_write_scope(command, &args[1..]);
            let Some(target) = args.get(6) else {
                v3_usage_error(command, "missing target_profile")
            };
            let Some(reason) = args.get(7) else {
                v3_usage_error(command, "missing reason")
            };
            print_v3_json(
                app.upgrade_task_policy(project_id, task_id, actor, target, reason, version, key),
            );
        }
        "session-gap" | "session-recover" => {
            let (project_id, task_id, session_id, actor, version, key) =
                parse_v3_write_scope(command, &args[1..]);
            let Some(value) = args.get(7) else {
                v3_usage_error(command, "missing reason/evidence")
            };
            if command == "session-gap" {
                print_v3_json(
                    app.session_gap(project_id, task_id, session_id, actor, version, key, value),
                );
            } else {
                let evidence = serde_json::from_str::<Vec<String>>(value).unwrap_or_else(|error| {
                    v3_usage_error(command, &format!("invalid evidence JSON: {error}"))
                });
                print_v3_json(app.session_recover(
                    project_id, task_id, session_id, actor, version, key, evidence,
                ));
            }
        }
        "memory-command" => {
            let Some(path) = args.get(1) else {
                v3_usage_error(command, "missing memory command JSON")
            };
            let (content, label) = read_v3_command_content(path, "memory command");
            let value = serde_json::from_str::<MemoryCommand>(&content).unwrap_or_else(|error| {
                eprintln!("Invalid memory command JSON '{}': {error}", label);
                std::process::exit(2)
            });
            print_v3_json(app.memory_command(value));
        }
        "memory-query" => {
            let Some(project_id) = args.get(1) else {
                v3_usage_error(command, "missing project_id")
            };
            let Some(value) = args.get(2) else {
                v3_usage_error(command, "missing query JSON")
            };
            let query = serde_json::from_str::<MemoryQuery>(value).unwrap_or_else(|error| {
                v3_usage_error(command, &format!("invalid query JSON: {error}"))
            });
            print_v3_json(app.query_project_memory(project_id, &query));
        }
        "finding-command" | "attempt-command" => {
            let Some(path) = args.get(1) else {
                v3_usage_error(command, "missing typed lifecycle command JSON")
            };
            let (content, label) = read_v3_command_content(path, command);
            let value =
                serde_json::from_str::<LifecycleCommand>(&content).unwrap_or_else(|error| {
                    eprintln!("Invalid typed command JSON '{}': {error}", label);
                    std::process::exit(2)
                });
            let prefix = if command == "finding-command" {
                "finding."
            } else {
                "attempt."
            };
            if !value.event_type.starts_with(prefix) {
                v3_usage_error(command, "event_type does not match typed command family")
            };
            print_v3_json(app.lifecycle_command(value));
        }
        "plan-event" => {
            let Some(json_path) = args.get(1) else {
                v3_usage_error(
                    command,
                    "missing plan command JSON path (use --stdin or - to read from stdin)",
                );
            };
            let (content, label) = read_v3_command_content(json_path, "plan event");
            let plan_command = match serde_json::from_str::<V3PlanCommand>(&content) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("Invalid plan event JSON '{}': {error}", label);
                    std::process::exit(2);
                }
            };
            let result = match plan_command {
                V3PlanCommand::PlanNodeAdd { input } => app.plan_add_node(input),
                V3PlanCommand::PlanDependenciesSet { input } => app.plan_set_dependencies(input),
                V3PlanCommand::PlanNodeStateSet { input } => app.plan_set_state(input),
                V3PlanCommand::PlanCriteriaSet { input } => app.plan_set_criteria(input),
            };
            print_v3_json(result);
        }
        "lifecycle-event" => {
            let Some(json_path) = args.get(1) else {
                v3_usage_error(
                    command,
                    "missing lifecycle command JSON path (use --stdin or - to read from stdin)",
                );
            };
            let (content, label) = read_v3_command_content(json_path, "lifecycle event");
            let lifecycle_command = serde_json::from_str::<LifecycleCommand>(&content)
                .unwrap_or_else(|error| {
                    eprintln!("Invalid lifecycle event JSON '{}': {error}", label);
                    std::process::exit(2);
                });
            print_v3_json(app.lifecycle_command(lifecycle_command));
        }
        "worktree-event" => {
            let Some(json_path) = args.get(1) else {
                v3_usage_error(
                    command,
                    "missing orchestration command JSON path (use --stdin or - to read from stdin)",
                );
            };
            let (content, label) = read_v3_command_content(json_path, "worktree event");
            let orch_command = match serde_json::from_str::<
                vibehub_core::v3::orchestration::OrchestrationCommand,
            >(&content)
            {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("Invalid worktree event JSON '{}': {error}", label);
                    std::process::exit(2);
                }
            };
            print_v3_json(app.orchestration_command(orch_command));
        }
        "worktree-orchestration" => {
            let Some(project_id) = args.get(1) else {
                v3_usage_error(command, "missing project_id");
            };
            let Some(task_id) = args.get(2) else {
                v3_usage_error(command, "missing task_id");
            };
            print_v3_json(app.worktree_orchestration(project_id, task_id));
        }
        _ => v3_usage_error(command, "unknown command"),
    }
}

#[derive(serde::Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
enum V3PlanCommand {
    PlanNodeAdd {
        #[serde(flatten)]
        input: PlanAddNodeCommand,
    },
    PlanDependenciesSet {
        #[serde(flatten)]
        input: PlanSetDependenciesCommand,
    },
    PlanNodeStateSet {
        #[serde(flatten)]
        input: PlanSetStateCommand,
    },
    PlanCriteriaSet {
        #[serde(flatten)]
        input: PlanSetCriteriaCommand,
    },
}

fn optional_v3_value(value: Option<&String>) -> Option<String> {
    value
        .filter(|value| !value.is_empty() && value.as_str() != "-")
        .cloned()
}

fn parse_v3_write_scope<'a>(
    command: &str,
    args: &'a [String],
) -> (&'a str, &'a str, &'a str, &'a str, u64, &'a str) {
    if args.len() < 6 {
        v3_usage_error(
            command,
            "expected project_id task_id session_id actor expected_version idempotency_key",
        );
    }
    let expected_version = args[4].parse::<u64>().unwrap_or_else(|error| {
        v3_usage_error(command, &format!("invalid expected_version: {error}"))
    });
    (
        &args[0],
        &args[1],
        &args[2],
        &args[3],
        expected_version,
        &args[5],
    )
}

fn parse_v3_task_write_scope<'a>(
    command: &str,
    args: &'a [String],
) -> (&'a str, &'a str, &'a str, u64, &'a str) {
    if args.len() < 5 {
        v3_usage_error(
            command,
            "expected project_id task_id actor expected_version idempotency_key",
        );
    }
    let expected_version = args[3].parse::<u64>().unwrap_or_else(|error| {
        v3_usage_error(command, &format!("invalid expected_version: {error}"))
    });
    (&args[0], &args[1], &args[2], expected_version, &args[4])
}

fn v3_usage_error(command: &str, message: &str) -> ! {
    eprintln!("Invalid V3 {command} request: {message}");
    std::process::exit(2);
}

/// All CLI read-model views share the same freshness gate as MCP. A view must
/// never silently continue with a projection that is behind the event log.
fn open_synced_v3_view_repository(
    app: &V3ApplicationService,
    project_root: &str,
) -> V3ViewRepository {
    let repository =
        V3ViewRepository::open(project_root).unwrap_or_else(|error| print_v3_error(error));
    let project_id = repository.project_id();
    app.sync_projection_if_stale(&project_id)
        .unwrap_or_else(|error| print_v3_error(error));
    repository
}

fn print_v3_json<T: serde::Serialize>(result: Result<T, vibehub_core::v3::V3Error>) {
    match result {
        Ok(value) => println!(
            "{}",
            serde_json::to_string_pretty(&value).expect("V3 result must serialize")
        ),
        Err(error) => print_v3_error(error),
    }
}

fn print_v3_error(error: vibehub_core::v3::V3Error) -> ! {
    eprintln!(
        "{}",
        serde_json::to_string_pretty(&error).expect("V3 error must serialize")
    );
    std::process::exit(1);
}

fn read_v3_command_content(request_path: &str, command_label: &str) -> (String, String) {
    if matches!(request_path, "--stdin" | "-") {
        let mut content = String::new();
        if let Err(error) = io::stdin().read_to_string(&mut content) {
            eprintln!("Failed to read {command_label} from stdin: {error}");
            std::process::exit(2);
        }
        return (content, "stdin".to_string());
    }

    match fs::read_to_string(request_path) {
        Ok(content) => (content, request_path.to_string()),
        Err(error) => {
            eprintln!("Failed to read {command_label} '{}': {error}", request_path);
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
        .filter(|arg| !arg.starts_with("--"))
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

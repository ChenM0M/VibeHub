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
use std::sync::Mutex;
use storage::Storage;

fn main() {
    if matches!(
        vibehub_adapters::dispatcher::dispatch(std::env::args().skip(1), env!("CARGO_PKG_VERSION"),),
        vibehub_adapters::dispatcher::DispatchOutcome::Handled
    ) {
        return;
    }

    let storage = Storage::new().expect("Failed to initialize storage");
    replay_pending_events_for_known_projects(&storage);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
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
            commands::legacy_v2_load_archive,
            commands::v3_inspect_project_layout,
            commands::v3_initialize_project,
            commands::v3_migrate_project,
            commands::v3_recover_project_migration,
            commands::v3_get_project_settings,
            commands::v3_update_project_settings,
            commands::v3_agent_specs_status,
            commands::v3_agent_specs_sync,
            commands::v3_create_task,
            commands::v3_plan_add_node,
            commands::v3_plan_set_dependencies,
            commands::v3_plan_set_state,
            commands::v3_load_view_bundle,
            commands::v3_query_project_structure,
            commands::vibehub_read_local_agent_usage,
            commands::vibehub_open_vibehub_file,
            commands::vibehub_reveal_project_file,
            commands::vibehub_open_project_file,
            gateway::get_gateway_config,
            gateway::save_gateway_config,
            gateway::get_gateway_stats,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn replay_pending_events_for_known_projects(storage: &Storage) {
    let Ok(config) = storage.load_config() else {
        return;
    };
    for project in config.projects {
        let _ = vibehub::events::replay_pending_events(project.path);
    }
}

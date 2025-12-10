// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod launcher;
mod models;
mod scanner;
mod storage;
mod gateway;
mod updater;

use commands::AppState;
use storage::Storage;
use std::sync::Mutex;
use tauri::Manager;

fn main() {
    let storage = Storage::new().expect("Failed to initialize storage");

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
            commands::record_project_open,
            commands::toggle_project_star,
            commands::initialize_default_configs,
            commands::set_theme,
            commands::refresh_all_workspaces,
            commands::check_for_updates,
            gateway::get_gateway_config,
            gateway::save_gateway_config,
            gateway::get_gateway_stats,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

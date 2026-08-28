// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod agent_profiles;
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
mod workspace_state;

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
            #[cfg(target_os = "macos")]
            install_macos_menu(app)?;
            Ok(())
        })
        .manage(AppState {
            storage: Mutex::new(storage),
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_config,
            commands::save_config,
            commands::load_workspace_state,
            commands::save_workspace_state,
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
            commands::v3_inspect_project_repair_candidates,
            commands::v3_repair_project,
            commands::v3_get_project_settings,
            commands::v3_update_project_settings,
            commands::v3_agent_specs_status,
            commands::v3_agent_specs_sync,
            commands::v3_create_task,
            commands::v3_task_candidates,
            commands::v3_task_route,
            commands::v3_session_task_binding,
            commands::v3_session_task_bind,
            commands::v3_session_task_unbind,
            commands::v3_complete_task,
            commands::v3_close_task_with_exceptions,
            commands::v3_plan_add_node,
            commands::v3_plan_set_dependencies,
            commands::v3_plan_set_state,
            commands::v3_plan_set_criteria,
            commands::v3_lifecycle_typed_command,
            commands::v3_memory_command,
            commands::v3_memory_query,
            commands::v3_orchestration_command,
            commands::v3_load_view_bundle,
            commands::v3_load_node_brief,
            commands::v3_query_project_structure,
            commands::v3_query_archived_tasks,
            agent_profiles::v3_agent_profile_discover,
            agent_profiles::v3_agent_profile_runtime_targets,
            agent_profiles::v3_agent_profile_read,
            agent_profiles::v3_agent_profile_save,
            agent_profiles::v3_agent_profile_validate,
            agent_profiles::v3_agent_profile_activate,
            agent_profiles::v3_agent_profile_launch,
            agent_profiles::v3_agent_profile_restore,
            agent_profiles::v3_agent_profile_diagnostics,
            agent_profiles::v3_agent_profile_create,
            agent_profiles::v3_agent_profile_clone,
            agent_profiles::v3_agent_profile_rename,
            agent_profiles::v3_agent_profile_delete,
            agent_profiles::v3_agent_profile_list_upstream_models,
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

/// The macOS default menu binds Cmd+W to "Close Window", which would kill the whole
/// cockpit window instead of the focused project tab. Rebuild the standard menu without
/// that accelerator and expose window closing on Cmd+Shift+W, matching browser semantics.
#[cfg(target_os = "macos")]
fn install_macos_menu(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{AboutMetadata, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
    use tauri::Manager;

    let handle = app.handle().clone();
    let app_submenu = SubmenuBuilder::new(&handle, "VibeHub")
        .about(Some(AboutMetadata::default()))
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;
    let edit_submenu = SubmenuBuilder::new(&handle, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;
    let close_window = MenuItemBuilder::new("Close Window")
        .id("close-window")
        .accelerator("CmdOrCtrl+Shift+W")
        .build(&handle)?;
    let window_submenu = SubmenuBuilder::new(&handle, "Window")
        .minimize()
        .fullscreen()
        .separator()
        .item(&close_window)
        .build()?;
    let menu = MenuBuilder::new(&handle)
        .items(&[&app_submenu, &edit_submenu, &window_submenu])
        .build()?;
    app.set_menu(menu)?;
    app.on_menu_event(|app_handle, event| {
        if event.id() == "close-window" {
            if let Some(window) = app_handle.get_webview_window("main") {
                let _ = window.close();
            }
        }
    });
    Ok(())
}

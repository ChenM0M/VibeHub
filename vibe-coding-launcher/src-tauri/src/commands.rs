use crate::{
    launcher::Launcher,
    models::*,
    scanner::Scanner,
    storage::Storage,
};
use chrono::Utc;
use tauri::State;
use std::sync::Mutex;

pub struct AppState {
    pub storage: Mutex<Storage>,
}

#[tauri::command]
pub async fn load_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.load_config().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_config(
    config: AppConfig,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.save_config(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn scan_workspace(
    path: String,
    max_depth: usize,
) -> Result<Vec<Project>, String> {
    Scanner::scan_directory(&path, max_depth).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_workspace(
    name: String,
    path: String,
    auto_scan: bool,
    state: State<'_, AppState>,
) -> Result<Workspace, String> {
    let workspace = Workspace {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        path,
        auto_scan,
        created_at: Utc::now(),
    };

    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut config = storage.load_config().map_err(|e| e.to_string())?;
    config.workspaces.push(workspace.clone());
    storage.save_config(&config).map_err(|e| e.to_string())?;

    Ok(workspace)
}

#[tauri::command]
pub async fn remove_workspace(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut config = storage.load_config().map_err(|e| e.to_string())?;
    config.workspaces.retain(|w| w.id != workspace_id);
    storage.save_config(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_project(
    project: Project,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut config = storage.load_config().map_err(|e| e.to_string())?;
    
    if let Some(idx) = config.projects.iter().position(|p| p.id == project.id) {
        config.projects[idx] = project;
    } else {
        config.projects.push(project);
    }
    
    storage.save_config(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_project(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut config = storage.load_config().map_err(|e| e.to_string())?;
    config.projects.retain(|p| p.id != project_id);
    storage.save_config(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_tag(
    tag: Tag,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut config = storage.load_config().map_err(|e| e.to_string())?;
    config.tags.push(tag);
    storage.save_config(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_tag(
    tag: Tag,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut config = storage.load_config().map_err(|e| e.to_string())?;
    
    if let Some(idx) = config.tags.iter().position(|t| t.id == tag.id) {
        config.tags[idx] = tag;
    }
    
    storage.save_config(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_tag(
    tag_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut config = storage.load_config().map_err(|e| e.to_string())?;
    config.tags.retain(|t| t.id != tag_id);
    
    // Remove tag from all projects
    for project in &mut config.projects {
        project.tags.retain(|t| t != &tag_id);
    }
    
    storage.save_config(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn launch_tool(
    config_id: String,
    project_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let app_config = storage.load_config().map_err(|e| e.to_string())?;
    
    let launch_config = app_config
        .launch_configs
        .iter()
        .find(|c| c.id == config_id)
        .ok_or("Launch config not found")?;

    // Handle special case: terminal
    if launch_config.executable_path == "terminal" {
        Launcher::open_terminal(&project_path).map_err(|e| e.to_string())?;
        return Ok(());
    }

    Launcher::launch(launch_config, &project_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_in_explorer(path: String) -> Result<(), String> {
    Launcher::open_in_file_manager(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_terminal(path: String) -> Result<(), String> {
    Launcher::open_terminal(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_launch_config(
    config: LaunchConfig,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut app_config = storage.load_config().map_err(|e| e.to_string())?;
    app_config.launch_configs.push(config);
    storage.save_config(&app_config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_launch_config(
    config: LaunchConfig,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut app_config = storage.load_config().map_err(|e| e.to_string())?;
    
    if let Some(idx) = app_config.launch_configs.iter().position(|c| c.id == config.id) {
        app_config.launch_configs[idx] = config;
    }
    
    storage.save_config(&app_config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_launch_config(
    config_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut app_config = storage.load_config().map_err(|e| e.to_string())?;
    app_config.launch_configs.retain(|c| c.id != config_id);
    storage.save_config(&app_config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn record_project_open(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut config = storage.load_config().map_err(|e| e.to_string())?;
    
    // Update project last_opened
    if let Some(project) = config.projects.iter_mut().find(|p| p.id == project_id) {
        project.last_opened = Some(Utc::now());
    }
    
    // Update recent projects
    config.recent_projects.retain(|id| id != &project_id);
    config.recent_projects.insert(0, project_id);
    
    // Keep only last 20 recent projects
    if config.recent_projects.len() > 20 {
        config.recent_projects.truncate(20);
    }
    
    storage.save_config(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_project_star(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut config = storage.load_config().map_err(|e| e.to_string())?;
    
    if let Some(project) = config.projects.iter_mut().find(|p| p.id == project_id) {
        project.starred = !project.starred;
        let starred = project.starred;
        storage.save_config(&config).map_err(|e| e.to_string())?;
        Ok(starred)
    } else {
        Err("Project not found".to_string())
    }
}

#[tauri::command]
pub async fn initialize_default_configs(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut config = storage.load_config().map_err(|e| e.to_string())?;
    
    if config.launch_configs.is_empty() {
        config.launch_configs = Launcher::get_default_configs();
        storage.save_config(&config).map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

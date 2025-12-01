use crate::{
    launcher::Launcher,
    models::*,
    scanner::Scanner,
    storage::Storage,
};
use chrono::Utc;
use tauri::State;
use std::sync::Mutex;
use std::process::Command;

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
    
    if config.tags.is_empty() {
        config.tags = Tag::default_tags();
        storage.save_config(&config).map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

#[tauri::command]
pub async fn launch_tool(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    println!("Frontend requested launch_tool for project_id: {}", project_id);
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let config = storage.load_config().map_err(|e| e.to_string())?;
    
    let project = config.projects.iter().find(|p| p.id == project_id)
        .ok_or("Project not found")?;
        
    // Collect all tag configs
    let mut tag_configs = Vec::new();
    for tag_id in &project.tags {
        if let Some(tag) = config.tags.iter().find(|t| &t.id == tag_id) {
             if let Some(conf) = &tag.config {
                 tag_configs.push((conf.clone(), tag.category.clone()));
             }
        }
    }
    
    Launcher::launch(project, &tag_configs).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn launch_custom(
    project_id: String,
    config: TagConfig,
    state: State<'_, AppState>,
) -> Result<(), String> {
    println!("Frontend requested launch_custom for project_id: {}, config: {:?}", project_id, config);
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let app_config = storage.load_config().map_err(|e| e.to_string())?;
    
    let project = app_config.projects.iter().find(|p| p.id == project_id)
        .ok_or("Project not found")?;
        
    // For custom launch, we assume it's a CLI tool or script that might benefit from a window
    // or we can treat it as Custom category
    Launcher::launch(project, &[(config, TagCategory::Custom)]).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_in_explorer(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn open_terminal(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "cd", "/d", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg("-a")
            .arg("Terminal")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        // Try common terminals
        if Command::new("gnome-terminal").arg("--working-directory").arg(&path).spawn().is_err() {
            Command::new("xterm").arg("-e").arg(format!("cd '{}' && $SHELL", path)).spawn().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn set_theme(
    theme: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut config = storage.load_config().map_err(|e| e.to_string())?;
    config.theme = theme;
    storage.save_config(&config).map_err(|e| e.to_string())
}

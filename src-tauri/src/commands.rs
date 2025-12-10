use crate::{
    launcher::Launcher,
    models::*,
    scanner::Scanner,
    storage::Storage,
    updater,
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
    state: State<'_, AppState>,
) -> Result<Vec<Project>, String> {
    let scanned_projects = Scanner::scan_directory(&path, max_depth).map_err(|e| e.to_string())?;
    
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut config = storage.load_config().map_err(|e| e.to_string())?;
    
    // Normalize workspace path for comparison
    let ws_path = std::path::Path::new(&path);
    let ws_path_str = ws_path.to_string_lossy().to_string();
    
    // Helper to clean path for comparison (remove \\?\ prefix)
    let clean_path = |p: &str| -> String {
        if p.starts_with(r"\\?\") {
            p[4..].to_string()
        } else {
            p.to_string()
        }
    };

    // 1. Identify existing projects that are children of this workspace
    // We'll rebuild the projects list
    // Unused variables removed
    
    // Separate projects into "related to this workspace" and "others"
    // Related = path starts with workspace path (loosely)
    // Actually, simpler: We iterate all config projects. 
    // If a project is in the scanned list (by clean path), we update and keep it.
    // If a project is NOT in scanned list BUT is a child of this workspace, we drop it (it's junk or deleted).
    // If a project is unrelated, we keep it.
    
    // To do this efficiently:
    // Create a map of scanned projects by clean path
    let mut scanned_map: std::collections::HashMap<String, Project> = std::collections::HashMap::new();
    for p in scanned_projects {
        scanned_map.insert(p.path.clone(), p);
    }
    
    let mut final_projects = Vec::new();
    let mut processed_scanned_paths = std::collections::HashSet::new();

    for existing in &config.projects {
        let existing_clean = clean_path(&existing.path);
        
        // Check if this existing project belongs to the workspace being scanned
        // We assume it belongs if it's a direct child or inside the path
        // Since we only scan depth 1, we can check if parent dir matches workspace
        let is_in_workspace = std::path::Path::new(&existing_clean)
            .parent()
            .map(|p| p.to_string_lossy().to_string() == ws_path_str || existing_clean.starts_with(&ws_path_str))
            .unwrap_or(false);

        if is_in_workspace {
            // It belongs to this workspace. Check if it's in the new scan result.
            if let Some(scanned) = scanned_map.get(&existing_clean) {
                // It exists in scan. Update it.
                let mut updated = existing.clone();
                updated.path = scanned.path.clone(); // Ensure clean path
                updated.project_type = scanned.project_type.clone();
                updated.metadata = scanned.metadata.clone();
                if scanned.description.is_some() {
                    updated.description = scanned.description.clone();
                }
                final_projects.push(updated);
                processed_scanned_paths.insert(existing_clean);
            } else {
                // It's in the workspace but NOT in the scan result.
                // This means it's either deleted or ignored (junk).
                // User wants "truthful update", so we REMOVE it.
                println!("Removing project no longer found/valid: {}", existing.name);
                // Do not add to final_projects
            }
        } else {
            // Unrelated project, keep as is
            final_projects.push(existing.clone());
        }
    }
    
    // Add new projects that weren't in config
    // Fix: Iterate by reference to avoid moving scanned_map
    for (path, project) in &scanned_map {
        if !processed_scanned_paths.contains(path) {
            final_projects.push(project.clone());
        }
    }
    
    config.projects = final_projects.clone();
    storage.save_config(&config).map_err(|e| e.to_string())?;
    
    // Return only the projects for this workspace (scanned ones)
    let result: Vec<Project> = final_projects.into_iter()
        .filter(|p| scanned_map.contains_key(&p.path))
        .collect();
        
    Ok(result)
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
    
    // Find the workspace to be removed and clean up related projects
    if let Some(workspace) = config.workspaces.iter().find(|w| w.id == workspace_id) {
        let ws_path = workspace.path.replace("\\", "/").to_lowercase();
        // Remove all projects that belong to this workspace
        config.projects.retain(|p| {
            let proj_path = p.path.replace("\\", "/").to_lowercase();
            !proj_path.starts_with(&ws_path)
        });
    }
    
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
pub async fn refresh_project(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<Project, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut config = storage.load_config().map_err(|e| e.to_string())?;
    
    let project = config.projects.iter_mut().find(|p| p.id == project_id)
        .ok_or("Project not found")?;
        
    Scanner::refresh_project(project);
    let updated_project = project.clone();
    
    storage.save_config(&config).map_err(|e| e.to_string())?;
    
    Ok(updated_project)
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

/// Refresh all workspaces by rescanning them and cleaning up stale/orphaned projects
#[tauri::command]
pub async fn refresh_all_workspaces(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut config = storage.load_config().map_err(|e| e.to_string())?;
    
    // Helper to normalize path for comparison
    let normalize_path = |p: &str| -> String {
        let cleaned = if p.starts_with(r"\\?\") {
            &p[4..]
        } else {
            p
        };
        cleaned.replace("\\", "/").to_lowercase()
    };
    
    // Collect all workspace paths (normalized)
    let workspace_paths: Vec<String> = config.workspaces.iter()
        .map(|w| normalize_path(&w.path))
        .collect();
    
    // Step 1: Remove orphaned projects (not belonging to any current workspace)
    config.projects.retain(|p| {
        let proj_path = normalize_path(&p.path);
        // Keep project only if it's a child of some workspace
        workspace_paths.iter().any(|ws_path| proj_path.starts_with(ws_path))
    });
    
    // Save after orphan cleanup
    storage.save_config(&config).map_err(|e| e.to_string())?;
    
    // Step 2: Rescan each workspace and update projects
    let workspace_paths_original: Vec<String> = config.workspaces.iter()
        .map(|w| w.path.clone())
        .collect();
    
    drop(storage);
    
    for ws_path in workspace_paths_original {
        let storage = state.storage.lock().map_err(|e| e.to_string())?;
        let scanned_projects = match Scanner::scan_directory(&ws_path, 1) {
            Ok(projects) => projects,
            Err(_) => continue, // Skip if workspace path doesn't exist
        };
        
        let mut config = storage.load_config().map_err(|e| e.to_string())?;
        let ws_path_normalized = normalize_path(&ws_path);
        
        // Helper to normalize without borrow issues
        let normalize = |p: &str| -> String {
            let cleaned = if p.starts_with(r"\\?\") { &p[4..] } else { p };
            cleaned.replace("\\", "/").to_lowercase()
        };
        
        // Build map of scanned projects
        let mut scanned_map: std::collections::HashMap<String, Project> = std::collections::HashMap::new();
        for p in scanned_projects {
            scanned_map.insert(normalize(&p.path), p);
        }
        
        let mut final_projects = Vec::new();
        let mut processed_paths = std::collections::HashSet::new();
        
        for existing in &config.projects {
            let existing_normalized = normalize(&existing.path);
            let is_in_this_workspace = existing_normalized.starts_with(&ws_path_normalized);
            
            if is_in_this_workspace {
                // Project belongs to this workspace, check if still exists
                if let Some(scanned) = scanned_map.get(&existing_normalized) {
                    let mut updated = existing.clone();
                    updated.path = scanned.path.clone();
                    updated.project_type = scanned.project_type.clone();
                    updated.metadata = scanned.metadata.clone();
                    if scanned.description.is_some() {
                        updated.description = scanned.description.clone();
                    }
                    final_projects.push(updated);
                    processed_paths.insert(existing_normalized);
                }
                // Else: project no longer exists in scan, drop it
            } else {
                // Project belongs to another workspace, keep as-is
                final_projects.push(existing.clone());
            }
        }
        
        // Add new projects from scan
        for (path, project) in &scanned_map {
            if !processed_paths.contains(path) {
                final_projects.push(project.clone());
            }
        }
        
        config.projects = final_projects;
        storage.save_config(&config).map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

#[tauri::command]
pub async fn check_for_updates() -> Result<updater::UpdateCheckResult, String> {
    updater::check_for_updates().await
}

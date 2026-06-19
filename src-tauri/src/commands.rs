#[cfg(target_os = "windows")]
use crate::process_util::silent_command;
use crate::{
    app_paths,
    gateway::{
        config::{ApiType, GatewayConfig},
        GatewayConfigPath, GatewayState,
    },
    launcher::Launcher,
    local_agent_usage::{self, LocalAgentUsageOverview},
    models::*,
    scanner::Scanner,
    storage::Storage,
    updater,
    vibehub::agent_adapter::{
        self, AgentAdapterConfig, AgentAdapterConfigPatch, AgentAdapterStatus,
        AgentAdapterSyncResult, AgentTool,
    },
    vibehub::agent_view::{self, AgentViewGenerateResult},
    vibehub::capability::{self, CapabilityClaimResult, CapabilityGateReport},
    vibehub::cockpit::{self, VibehubFileReadResult},
    vibehub::context::{self, ContextPackBuildResult},
    vibehub::debug_dump::{self, DebugDumpOptions, DebugDumpResult},
    vibehub::drift::{self, WorkspaceDriftReport},
    vibehub::events::{self, PendingReplayResult},
    vibehub::handoff::{self, HandoffBuildResult},
    vibehub::init::{self, VibehubInitOptions, VibehubInitResult},
    vibehub::journal::{self, JournalAppendResult},
    vibehub::knowledge::{self, KnowledgeAppendResult},
    vibehub::neighbors::{self, TaskNeighborReport},
    vibehub::notes::{self, ProjectDigest},
    vibehub::overview::{self, CockpitOverview},
    vibehub::ownership::{
        self, FileOwnershipClassificationReport, FileOwnershipRecordRequest,
        FileOwnershipRecordResult,
    },
    vibehub::phase::{self, PhaseAdvanceResult, PhaseSetResult, PhaseValidationResult},
    vibehub::project_structure,
    vibehub::prompts::{self, PromptRenderResult, PromptTemplateOption},
    vibehub::research::{self, ResearchPackArchiveResult, ResearchPackBuildResult},
    vibehub::review::{self, ReviewEvidenceGenerateResult},
    vibehub::schema_check::{self, CapabilityOutputWriteResult, CapabilityValidationReport},
    vibehub::start_task::{
        self, VibehubStartTaskIntakeRequest, VibehubStartTaskIntakeResult, VibehubStartTaskResult,
    },
    vibehub::state_migration::{self, StateMigrationReport},
    vibehub::sync::{self, SyncReport},
    vibehub::task_switch::{self, TaskSwitchResult},
    vibehub::workflow::{self, WorkflowExplainResult},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::process::Command;
use std::sync::Mutex;
use tauri::{Emitter, State};

pub struct AppState {
    pub storage: Mutex<Storage>,
}

#[derive(Debug, Clone, Serialize)]
struct VibehubStatusChangedEvent {
    project_path: String,
    source: String,
}

fn emit_vibehub_status_changed(app: &tauri::AppHandle, project_path: &str, source: &str) {
    let _ = app.emit(
        "vibehub://status-changed",
        VibehubStatusChangedEvent {
            project_path: project_path.to_string(),
            source: source.to_string(),
        },
    );
}

fn emit_on_success<T>(
    app: &tauri::AppHandle,
    project_path: &str,
    source: &str,
    result: anyhow::Result<T>,
) -> Result<T, String> {
    result
        .map(|value| {
            emit_vibehub_status_changed(app, project_path, source);
            value
        })
        .map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsExportFile {
    pub schema_version: u32,
    pub kind: String,
    pub source_system: String,
    pub exported_at: String,
    pub app_version: String,
    pub tags: Vec<Tag>,
    pub gateway_config: GatewayConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsImportAdjustment {
    pub scope: String,
    pub item_id: Option<String>,
    pub item_name: Option<String>,
    pub field: String,
    pub before: Option<String>,
    pub after: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsImportResult {
    pub source_system: String,
    pub target_system: String,
    pub tags_added: usize,
    pub tags_updated: usize,
    pub gateway_providers: usize,
    pub adjustments: Vec<SettingsImportAdjustment>,
}

#[tauri::command]
pub async fn load_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.load_config().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_config(config: AppConfig, state: State<'_, AppState>) -> Result<(), String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.save_config(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_settings_bundle(
    path: String,
    app_state: State<'_, AppState>,
    gateway_state: State<'_, GatewayState>,
) -> Result<(), String> {
    let tags = {
        let storage = app_state.storage.lock().map_err(|e| e.to_string())?;
        storage.load_config().map_err(|e| e.to_string())?.tags
    };

    let gateway_config = gateway_state.0.read().await.clone();
    let export = SettingsExportFile {
        schema_version: 1,
        kind: "vibehub_settings_export".to_string(),
        source_system: current_system().to_string(),
        exported_at: Utc::now().to_rfc3339(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        tags,
        gateway_config,
    };

    let content = serde_json::to_string_pretty(&export).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_settings_bundle(
    path: String,
    app_state: State<'_, AppState>,
    gateway_state: State<'_, GatewayState>,
    gateway_path: State<'_, GatewayConfigPath>,
) -> Result<SettingsImportResult, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut bundle: SettingsExportFile =
        serde_json::from_str(&content).map_err(|e| e.to_string())?;

    if bundle.kind != "vibehub_settings_export" {
        return Err("Invalid settings export file: unsupported kind".to_string());
    }
    if bundle.source_system.trim().is_empty() {
        return Err("Invalid settings export file: source_system is required".to_string());
    }

    let target_system = current_system().to_string();
    let mut adjustments = Vec::new();

    for tag in &mut bundle.tags {
        adapt_tag_for_system(tag, &target_system, &mut adjustments);
    }

    normalize_gateway_config(&mut bundle.gateway_config, &mut adjustments);

    let (tags_added, tags_updated) = {
        let storage = app_state.storage.lock().map_err(|e| e.to_string())?;
        let mut config = storage.load_config().map_err(|e| e.to_string())?;
        let counts = merge_tags(&mut config.tags, bundle.tags);

        storage.save_config(&config).map_err(|e| e.to_string())?;
        counts
    };

    {
        let mut current_gateway = gateway_state.0.write().await;
        *current_gateway = bundle.gateway_config.clone();
    }
    bundle
        .gateway_config
        .save(&gateway_path.0)
        .map_err(|e| e.to_string())?;

    Ok(SettingsImportResult {
        source_system: bundle.source_system,
        target_system,
        tags_added,
        tags_updated,
        gateway_providers: bundle.gateway_config.providers.len(),
        adjustments,
    })
}

fn current_system() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "windows"
    }
    #[cfg(target_os = "macos")]
    {
        "macos"
    }
    #[cfg(target_os = "linux")]
    {
        "linux"
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        "unknown"
    }
}

fn merge_tags(existing: &mut Vec<Tag>, imported: Vec<Tag>) -> (usize, usize) {
    let mut added = 0;
    let mut updated = 0;

    for mut tag in imported {
        if let Some(index) = existing.iter().position(|current| current.id == tag.id) {
            existing[index] = tag;
            updated += 1;
            continue;
        }

        if let Some(index) = existing
            .iter()
            .position(|current| current.name == tag.name && current.category == tag.category)
        {
            tag.id = existing[index].id.clone();
            existing[index] = tag;
            updated += 1;
            continue;
        }

        existing.push(tag);
        added += 1;
    }

    (added, updated)
}

fn adapt_tag_for_system(
    tag: &mut Tag,
    target_system: &str,
    adjustments: &mut Vec<SettingsImportAdjustment>,
) {
    let Some(config) = tag.config.as_mut() else {
        return;
    };

    if tag.category == TagCategory::Cli {
        let before = config.terminal.clone();
        config.terminal = adapt_terminal(before.as_deref(), target_system);
        if config.terminal != before {
            adjustments.push(SettingsImportAdjustment {
                scope: "tag".to_string(),
                item_id: Some(tag.id.clone()),
                item_name: Some(tag.name.clone()),
                field: "terminal".to_string(),
                before,
                after: config.terminal.clone(),
                reason: format!("Adjusted CLI terminal for {target_system}."),
            });
        }
    } else if config.terminal.is_some() {
        let before = config.terminal.take();
        adjustments.push(SettingsImportAdjustment {
            scope: "tag".to_string(),
            item_id: Some(tag.id.clone()),
            item_name: Some(tag.name.clone()),
            field: "terminal".to_string(),
            before,
            after: None,
            reason: "Removed terminal from a non-CLI tag.".to_string(),
        });
    }

    if let Some(executable) = config.executable.clone() {
        if let Some(adapted) = adapt_executable(&executable, target_system) {
            if adapted != executable {
                config.executable = Some(adapted.clone());
                adjustments.push(SettingsImportAdjustment {
                    scope: "tag".to_string(),
                    item_id: Some(tag.id.clone()),
                    item_name: Some(tag.name.clone()),
                    field: "executable".to_string(),
                    before: Some(executable),
                    after: Some(adapted),
                    reason: format!("Adjusted executable style for {target_system}."),
                });
            }
        }
    }
}

fn adapt_terminal(terminal: Option<&str>, target_system: &str) -> Option<String> {
    let raw = terminal.unwrap_or("").trim();
    if target_system == "linux" {
        return None;
    }

    if target_system == "macos" {
        return match raw.to_ascii_lowercase().as_str() {
            "" | "terminal" | "terminal.app" => Some("Terminal".to_string()),
            "iterm" | "iterm2" | "iterm.app" | "iterm2.app" => Some("iTerm".to_string()),
            "warp" | "warp.app" => Some("Warp".to_string()),
            _ => Some("Terminal".to_string()),
        };
    }

    if target_system == "windows" {
        return match raw.to_ascii_lowercase().as_str() {
            "" | "command prompt" | "commandprompt" | "cmd" | "cmd.exe" => {
                Some("CommandPrompt".to_string())
            }
            "windows terminal" | "windowsterminal" | "wt" | "wt.exe" => {
                Some("WindowsTerminal".to_string())
            }
            "powershell" | "powershell.exe" | "pwsh" | "pwsh.exe" => Some("PowerShell".to_string()),
            _ => Some("CommandPrompt".to_string()),
        };
    }

    None
}

fn adapt_executable(executable: &str, target_system: &str) -> Option<String> {
    let trimmed = executable.trim();
    if trimmed.is_empty() {
        return None;
    }

    if target_system != "macos" && trimmed.to_ascii_lowercase().ends_with(".app") {
        return Some(strip_extension(last_path_component(trimmed), ".app"));
    }

    if target_system != "windows" && looks_like_windows_path(trimmed) {
        let basename = last_path_component(trimmed);
        for extension in [".exe", ".cmd", ".bat", ".ps1"] {
            if basename.to_ascii_lowercase().ends_with(extension) {
                return Some(strip_extension(basename, extension));
            }
        }
        return Some(basename.to_string());
    }

    if target_system == "windows" && trimmed.starts_with('/') {
        return Some(last_path_component(trimmed).to_string());
    }

    Some(trimmed.to_string())
}

fn looks_like_windows_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    (bytes.len() > 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic()) || value.contains('\\')
}

fn last_path_component(value: &str) -> &str {
    value
        .rsplit(['/', '\\'])
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or(value)
}

fn strip_extension(value: &str, extension: &str) -> String {
    value[..value.len() - extension.len()].to_string()
}

fn normalize_gateway_config(
    config: &mut GatewayConfig,
    adjustments: &mut Vec<SettingsImportAdjustment>,
) {
    if config.port != 0 {
        let before = config.port.to_string();
        config.anthropic_port = config.port;
        config.port = 0;
        adjustments.push(SettingsImportAdjustment {
            scope: "gateway".to_string(),
            item_id: None,
            item_name: Some("Anthropic Gateway".to_string()),
            field: "port".to_string(),
            before: Some(before),
            after: Some(config.anthropic_port.to_string()),
            reason: "Migrated legacy gateway port to anthropic_port.".to_string(),
        });
    }

    for provider in &mut config.providers {
        if provider.api_types.is_empty() {
            provider.api_types = infer_provider_api_types(&provider.name);
            adjustments.push(SettingsImportAdjustment {
                scope: "gateway_provider".to_string(),
                item_id: Some(provider.id.clone()),
                item_name: Some(provider.name.clone()),
                field: "api_types".to_string(),
                before: Some("[]".to_string()),
                after: Some(format!("{:?}", provider.api_types)),
                reason: "Filled missing provider API types for the current gateway schema."
                    .to_string(),
            });
        }
    }
}

fn infer_provider_api_types(name: &str) -> Vec<ApiType> {
    let name_lower = name.to_lowercase();
    if name_lower.contains("claude") || name_lower.contains("anthropic") {
        vec![ApiType::Anthropic]
    } else if name_lower.contains("openai") || name_lower.contains("gpt") {
        vec![ApiType::OpenAIResponses, ApiType::OpenAIChat]
    } else {
        vec![
            ApiType::Anthropic,
            ApiType::OpenAIResponses,
            ApiType::OpenAIChat,
        ]
    }
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
    let mut scanned_map: std::collections::HashMap<String, Project> =
        std::collections::HashMap::new();
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
            .map(|p| {
                p.to_string_lossy().to_string() == ws_path_str
                    || existing_clean.starts_with(&ws_path_str)
            })
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
    let result: Vec<Project> = final_projects
        .into_iter()
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
pub async fn update_project(project: Project, state: State<'_, AppState>) -> Result<(), String> {
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

    let project = config
        .projects
        .iter_mut()
        .find(|p| p.id == project_id)
        .ok_or("Project not found")?;

    Scanner::refresh_project(project);
    let updated_project = project.clone();

    storage.save_config(&config).map_err(|e| e.to_string())?;

    Ok(updated_project)
}

#[tauri::command]
pub async fn delete_project(project_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut config = storage.load_config().map_err(|e| e.to_string())?;
    config.projects.retain(|p| p.id != project_id);
    storage.save_config(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_tag(tag: Tag, state: State<'_, AppState>) -> Result<(), String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut config = storage.load_config().map_err(|e| e.to_string())?;
    config.tags.push(tag);
    storage.save_config(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_tag(tag: Tag, state: State<'_, AppState>) -> Result<(), String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut config = storage.load_config().map_err(|e| e.to_string())?;

    if let Some(idx) = config.tags.iter().position(|t| t.id == tag.id) {
        config.tags[idx] = tag;
    }

    storage.save_config(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_tag(tag_id: String, state: State<'_, AppState>) -> Result<(), String> {
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
pub async fn initialize_default_configs(state: State<'_, AppState>) -> Result<(), String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut config = storage.load_config().map_err(|e| e.to_string())?;

    if config.tags.is_empty() {
        config.tags = Tag::default_tags();
        storage.save_config(&config).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn launch_tool(project_id: String, state: State<'_, AppState>) -> Result<(), String> {
    println!(
        "Frontend requested launch_tool for project_id: {}",
        project_id
    );
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let config = storage.load_config().map_err(|e| e.to_string())?;

    let project = config
        .projects
        .iter()
        .find(|p| p.id == project_id)
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
    category: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    println!(
        "Frontend requested launch_custom for project_id: {}, config: {:?}",
        project_id, config
    );
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let app_config = storage.load_config().map_err(|e| e.to_string())?;

    let project = app_config
        .projects
        .iter()
        .find(|p| p.id == project_id)
        .ok_or("Project not found")?;

    let tag_category = match category.as_deref() {
        Some("workspace") => TagCategory::Workspace,
        Some("ide") => TagCategory::Ide,
        Some("cli") => TagCategory::Cli,
        Some("environment") => TagCategory::Environment,
        Some("startup") => TagCategory::Startup,
        _ => TagCategory::Custom,
    };

    Launcher::launch(project, &[(config, tag_category)]).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_in_explorer(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        silent_command("explorer")
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

// ---------------------------------------------------------------------------
// Storage location management
//
// These commands surface and mutate the resolution that `app_paths` performs
// at startup. They never hot-swap the running `Storage`; instead they update
// the pointer file and the UI tells the user to restart. That keeps the
// runtime invariant "one Storage per process" and avoids partially-written
// state across two directories.
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_storage_info() -> Result<app_paths::StorageInfo, String> {
    app_paths::storage_info().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_custom_data_dir(path: String) -> Result<app_paths::StorageInfo, String> {
    app_paths::set_custom_data_dir(std::path::Path::new(&path)).map_err(|e| e.to_string())?;
    app_paths::storage_info().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_custom_data_dir() -> Result<app_paths::StorageInfo, String> {
    app_paths::clear_custom_data_dir().map_err(|e| e.to_string())?;
    app_paths::storage_info().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dismiss_storage_migration_notice() -> Result<app_paths::StorageInfo, String> {
    app_paths::dismiss_migration_notice().map_err(|e| e.to_string())?;
    app_paths::storage_info().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dismiss_storage_custom_dir_notice() -> Result<app_paths::StorageInfo, String> {
    app_paths::dismiss_custom_dir_notice().map_err(|e| e.to_string())?;
    app_paths::storage_info().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_terminal(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        silent_command("cmd")
            .args(["/C", "start", "cd", "/d", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        let escaped_path = path.replace('\\', "\\\\").replace('"', "\\\"");
        let script = format!(
            "tell application \"Terminal\"\nactivate\ndo script \"cd \" & quoted form of \"{}\"\nend tell",
            escaped_path
        );
        Command::new("osascript")
            .arg("-e")
            .arg(script)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        // Try common terminals
        if Command::new("gnome-terminal")
            .arg("--working-directory")
            .arg(&path)
            .spawn()
            .is_err()
        {
            Command::new("xterm")
                .arg("-e")
                .arg(format!("cd '{}' && $SHELL", path))
                .spawn()
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn set_theme(theme: String, state: State<'_, AppState>) -> Result<(), String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut config = storage.load_config().map_err(|e| e.to_string())?;
    config.theme = theme;
    storage.save_config(&config).map_err(|e| e.to_string())
}

/// Refresh all workspaces by rescanning them and cleaning up stale/orphaned projects
#[tauri::command]
pub async fn refresh_all_workspaces(state: State<'_, AppState>) -> Result<(), String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut config = storage.load_config().map_err(|e| e.to_string())?;

    // Helper to normalize path for comparison
    let normalize_path = |p: &str| -> String {
        let cleaned = if p.starts_with(r"\\?\") { &p[4..] } else { p };
        cleaned.replace("\\", "/").to_lowercase()
    };

    // Collect all workspace paths (normalized)
    let workspace_paths: Vec<String> = config
        .workspaces
        .iter()
        .map(|w| normalize_path(&w.path))
        .collect();

    // Step 1: Remove orphaned projects (not belonging to any current workspace)
    config.projects.retain(|p| {
        let proj_path = normalize_path(&p.path);
        // Keep project only if it's a child of some workspace
        workspace_paths
            .iter()
            .any(|ws_path| proj_path.starts_with(ws_path))
    });

    // Save after orphan cleanup
    storage.save_config(&config).map_err(|e| e.to_string())?;

    // Step 2: Rescan each workspace and update projects
    let workspace_paths_original: Vec<String> =
        config.workspaces.iter().map(|w| w.path.clone()).collect();

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
        let mut scanned_map: std::collections::HashMap<String, Project> =
            std::collections::HashMap::new();
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

#[tauri::command]
pub async fn vibehub_init(
    app: tauri::AppHandle,
    project_path: String,
    options: Option<VibehubInitOptions>,
) -> Result<VibehubInitResult, String> {
    emit_on_success(
        &app,
        &project_path,
        "vibehub_init",
        init::init_project_with_options(project_path.clone(), options),
    )
}

#[tauri::command]
pub async fn vibehub_start_task(
    app: tauri::AppHandle,
    project_path: String,
    title: Option<String>,
    mode: Option<String>,
    phase: Option<String>,
) -> Result<VibehubStartTaskResult, String> {
    emit_on_success(
        &app,
        &project_path,
        "vibehub_start_task",
        start_task::start_task(project_path.clone(), title, mode, phase),
    )
}

#[tauri::command]
pub async fn vibehub_start_task_intake(
    app: tauri::AppHandle,
    project_path: String,
    request: VibehubStartTaskIntakeRequest,
) -> Result<VibehubStartTaskIntakeResult, String> {
    emit_on_success(
        &app,
        &project_path,
        "vibehub_start_task_intake",
        start_task::start_task_intake(project_path.clone(), request),
    )
}

#[tauri::command]
pub async fn vibehub_build_context_pack(
    project_path: String,
    task_id: String,
    run_id: String,
    phase: String,
) -> Result<ContextPackBuildResult, String> {
    context::build_context_pack(project_path, task_id, run_id, phase).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_generate_agent_view(
    project_path: String,
) -> Result<AgentViewGenerateResult, String> {
    agent_view::generate_agent_view(project_path).map_err(|e| e.to_string())
}

// ─── Adapter sync ─────────────────────────────────────────────────────────
//
// `vibehub_sync_agent_adapter` (singular) is DEPRECATED: callers should use
// `vibehub_sync_agent_adapters` (plural) which accepts an explicit tool list.
// The wrapper is retained for back-compat with frontend code that has not
// migrated yet.

#[tauri::command]
pub async fn vibehub_sync_agent_adapter(
    project_path: String,
    dry_run: Option<bool>,
) -> Result<AgentAdapterSyncResult, String> {
    agent_adapter::sync_agent_adapter(project_path, dry_run.unwrap_or(false))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_get_agent_adapter_status(
    project_path: String,
) -> Result<AgentAdapterStatus, String> {
    agent_adapter::get_agent_adapter_status(project_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_update_agent_adapter_config(
    project_path: String,
    patch: AgentAdapterConfigPatch,
) -> Result<AgentAdapterConfig, String> {
    agent_adapter::update_agent_adapter_config(project_path, patch).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_sync_agent_adapters(
    app: tauri::AppHandle,
    project_path: String,
    tools: Option<Vec<AgentTool>>,
    dry_run: Option<bool>,
) -> Result<AgentAdapterSyncResult, String> {
    let dry_run = dry_run.unwrap_or(false);
    let result = agent_adapter::sync_agent_adapters(project_path.clone(), tools, dry_run)
        .map_err(|e| e.to_string());
    if result.is_ok() && !dry_run {
        emit_vibehub_status_changed(&app, &project_path, "vibehub_sync_agent_adapters");
    }
    result
}

// ─── Workspace drift / sync ───────────────────────────────────────────────
//
// `vibehub_check_workspace_drift` is DEPRECATED in favour of
// `vibehub_sync_workspace_state` (which produces the same drift report and
// can also write a recovery report). Both retained for now.

#[tauri::command]
pub async fn vibehub_check_workspace_drift(
    project_path: String,
    locale: Option<String>,
) -> Result<WorkspaceDriftReport, String> {
    drift::check_workspace_drift_with_locale(project_path, locale.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_sync_workspace_state(
    project_path: String,
    locale: Option<String>,
) -> Result<WorkspaceDriftReport, String> {
    drift::sync_workspace_state_with_locale(project_path, locale.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_sync_workspace(
    project_path: String,
    locale: Option<String>,
) -> Result<SyncReport, String> {
    sync::sync_workspace_with_locale(project_path, locale.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_workflow_explain(
    project_path: String,
) -> Result<WorkflowExplainResult, String> {
    workflow::explain_workflow(project_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_switch_task(
    project_path: String,
    task_id: String,
) -> Result<TaskSwitchResult, String> {
    task_switch::switch_task(project_path, task_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_classify_file_ownership(
    project_path: String,
    changed_files: Option<Vec<String>>,
) -> Result<FileOwnershipClassificationReport, String> {
    ownership::classify_workspace_ownership(project_path, changed_files).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_record_file_ownership(
    project_path: String,
    request: FileOwnershipRecordRequest,
) -> Result<FileOwnershipRecordResult, String> {
    ownership::record_file_ownership(project_path, request).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_query_task_neighbors(
    project_path: String,
) -> Result<TaskNeighborReport, String> {
    neighbors::query_current_task_neighbors(project_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_claim_capability(
    project_path: String,
    capability: String,
) -> Result<CapabilityClaimResult, String> {
    capability::claim_capability(project_path, capability).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_evaluate_capability_gates(
    project_path: String,
    requested_capability: Option<String>,
) -> Result<CapabilityGateReport, String> {
    capability::evaluate_capability_gates(project_path, requested_capability.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_validate_capability_output(
    project_path: String,
    capability: String,
    output: serde_json::Value,
) -> Result<CapabilityValidationReport, String> {
    schema_check::validate_capability_output_for_project(project_path, capability, &output)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_write_capability_output(
    project_path: String,
    capability: String,
    output: serde_json::Value,
) -> Result<CapabilityOutputWriteResult, String> {
    schema_check::write_current_capability_output(project_path, capability, output)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_replay_pending_events(
    app: tauri::AppHandle,
    project_path: String,
) -> Result<PendingReplayResult, String> {
    emit_on_success(
        &app,
        &project_path,
        "vibehub_replay_pending_events",
        events::replay_pending_events(project_path.clone()),
    )
}

#[tauri::command]
pub async fn vibehub_debug_dump(
    project_path: String,
    options: Option<DebugDumpOptions>,
) -> Result<DebugDumpResult, String> {
    debug_dump::create_debug_dump(project_path, options).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_build_handoff(project_path: String) -> Result<HandoffBuildResult, String> {
    handoff::build_handoff(project_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_generate_review_evidence(
    project_path: String,
    locale: Option<String>,
) -> Result<ReviewEvidenceGenerateResult, String> {
    review::generate_review_evidence_with_locale(project_path, locale.as_deref())
        .map_err(|e| e.to_string())
}

// ─── Aggregated overview ──────────────────────────────────────────────────
//
// Returns status + context + review + handoff + diff + research in ONE call,
// with a single `git` invocation. This REPLACES the per-tab read commands
// (`vibehub_read_cockpit_status`, `vibehub_read_context_view`,
// `vibehub_read_review_view`, `vibehub_read_handoff_view`,
// `vibehub_read_diff_view`, `vibehub_read_research_status`).

#[tauri::command]
pub async fn vibehub_read_overview(project_path: String) -> Result<CockpitOverview, String> {
    overview::read_overview(project_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_read_local_agent_usage(
    project_path: String,
) -> Result<LocalAgentUsageOverview, String> {
    local_agent_usage::read_local_agent_usage(project_path).map_err(|e| e.to_string())
}

/// Read the agent-written project-level digest (`.vibehub/notes/summary.md`
/// and `.vibehub/notes/status.md`). Pure read; never writes. The frontend
/// already gets the same data via `vibehub_read_overview`, but this command
/// lets a panel reload only the digest cheaply.
#[tauri::command]
pub async fn vibehub_read_project_digest(project_path: String) -> Result<ProjectDigest, String> {
    notes::read_project_digest(project_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_list_prompt_templates() -> Result<Vec<PromptTemplateOption>, String> {
    Ok(prompts::list_prompt_templates())
}

#[tauri::command]
pub async fn vibehub_render_prompt(
    project_path: String,
    template_id: String,
) -> Result<PromptRenderResult, String> {
    prompts::render_prompt(project_path, &template_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_append_journal_entry(
    project_path: String,
    title: Option<String>,
    body: Option<String>,
) -> Result<JournalAppendResult, String> {
    journal::append_journal_entry(project_path, title, body).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_append_knowledge_note(
    project_path: String,
    note: Option<String>,
) -> Result<KnowledgeAppendResult, String> {
    knowledge::append_knowledge_note(project_path, note).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_validate_phase(project_path: String) -> Result<PhaseValidationResult, String> {
    phase::validate_phase(project_path).map_err(|e| e.to_string())
}

// ─── Phase transitions ────────────────────────────────────────────────────
//
// `vibehub_set_phase_result` is the explicit setter (mostly used for marking
// a phase blocked / needs_action). `vibehub_complete_phase` validates the
// current phase and marks it completed without auto-advancing.
// `vibehub_advance_phase` validates AND moves to the next phase. The three
// commands intentionally have distinct semantics; do not collapse them.

#[tauri::command]
pub async fn vibehub_set_phase_result(
    project_path: String,
    target_phase: String,
    status: String,
) -> Result<PhaseSetResult, String> {
    phase::set_phase_result(project_path, &target_phase, &status).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_complete_phase(project_path: String) -> Result<PhaseAdvanceResult, String> {
    phase::complete_phase(project_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_advance_phase(
    project_path: String,
    force: Option<bool>,
) -> Result<PhaseAdvanceResult, String> {
    phase::advance_phase_with_force(project_path, force.unwrap_or(false)).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_pause_phase(project_path: String) -> Result<PhaseSetResult, String> {
    phase::pause_current_phase(project_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_build_research_pack(
    project_path: String,
    title: Option<String>,
) -> Result<ResearchPackBuildResult, String> {
    research::build_research_pack(project_path, title).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_archive_research(
    project_path: String,
) -> Result<Option<ResearchPackArchiveResult>, String> {
    research::archive_current_research(project_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_read_vibehub_file(
    project_path: String,
    relative_path: String,
) -> Result<VibehubFileReadResult, String> {
    cockpit::read_vibehub_file(project_path, relative_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_reveal_vibehub_file(
    project_path: String,
    relative_path: String,
) -> Result<(), String> {
    let (_, path, _) = cockpit::resolve_vibehub_file_path(project_path, relative_path)
        .map_err(|e| e.to_string())?;
    let target = path.parent().unwrap_or(&path).to_string_lossy().to_string();
    open_in_explorer(target).await
}

#[tauri::command]
pub async fn vibehub_open_vibehub_file(
    project_path: String,
    relative_path: String,
) -> Result<(), String> {
    let (_, path, _) = cockpit::resolve_vibehub_file_path(project_path, relative_path)
        .map_err(|e| e.to_string())?;
    open_in_explorer(path.to_string_lossy().to_string()).await
}

#[tauri::command]
pub async fn vibehub_reveal_project_file(
    project_path: String,
    relative_path: String,
) -> Result<(), String> {
    let (_, path, _) = project_structure::resolve_project_file_path(project_path, relative_path)
        .map_err(|e| e.to_string())?;
    let target = if path.is_dir() {
        path
    } else {
        path.parent().unwrap_or(&path).to_path_buf()
    };
    open_in_explorer(target.to_string_lossy().to_string()).await
}

#[tauri::command]
pub async fn vibehub_open_project_file(
    project_path: String,
    relative_path: String,
) -> Result<(), String> {
    let (_, path, _) = project_structure::resolve_project_file_path(project_path, relative_path)
        .map_err(|e| e.to_string())?;
    open_in_explorer(path.to_string_lossy().to_string()).await
}

// ─── State schema migration ───────────────────────────────────────────────
//
// `vibehub_dry_run_state_migration` reports what fields would be added or
// rewritten if the user opts in. `vibehub_migrate_state` performs the
// migration in-place and writes a `.bak.r<N>` snapshot of the original.

#[tauri::command]
pub async fn vibehub_dry_run_state_migration(
    project_path: String,
) -> Result<StateMigrationReport, String> {
    state_migration::dry_run(project_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_migrate_state(project_path: String) -> Result<StateMigrationReport, String> {
    state_migration::migrate(project_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vibehub_set_project_locale(
    app: tauri::AppHandle,
    project_path: String,
    locale: String,
) -> Result<String, String> {
    crate::vibehub::locale::persist_project_locale(std::path::Path::new(&project_path), &locale)
        .map_err(|e| e.to_string())?;
    emit_vibehub_status_changed(&app, &project_path, "vibehub_set_project_locale");
    Ok(locale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapts_macos_terminals_for_windows_imports() {
        assert_eq!(
            adapt_terminal(Some("Warp"), "windows"),
            Some("CommandPrompt".to_string())
        );
        assert_eq!(
            adapt_terminal(Some("iTerm"), "windows"),
            Some("CommandPrompt".to_string())
        );
        assert_eq!(
            adapt_terminal(Some("WindowsTerminal"), "windows"),
            Some("WindowsTerminal".to_string())
        );
    }

    #[test]
    fn adapts_windows_terminals_for_macos_imports() {
        assert_eq!(
            adapt_terminal(Some("PowerShell"), "macos"),
            Some("Terminal".to_string())
        );
        assert_eq!(
            adapt_terminal(Some("Warp"), "macos"),
            Some("Warp".to_string())
        );
    }

    #[test]
    fn adapts_platform_specific_executables() {
        assert_eq!(
            adapt_executable("/Applications/Visual Studio Code.app", "windows"),
            Some("Visual Studio Code".to_string())
        );
        assert_eq!(
            adapt_executable(r"C:\Users\me\AppData\Local\Programs\code.cmd", "macos"),
            Some("code".to_string())
        );
    }

    #[test]
    fn merge_tags_updates_by_id_or_name_and_adds_new_tags() {
        let mut existing = vec![Tag {
            id: "existing-id".to_string(),
            name: "CLI".to_string(),
            color: "#000000".to_string(),
            category: TagCategory::Cli,
            config: None,
        }];
        let imported = vec![
            Tag {
                id: "other-id".to_string(),
                name: "CLI".to_string(),
                color: "#ffffff".to_string(),
                category: TagCategory::Cli,
                config: None,
            },
            Tag {
                id: "new-id".to_string(),
                name: "New".to_string(),
                color: "#123456".to_string(),
                category: TagCategory::Custom,
                config: None,
            },
        ];

        let (added, updated) = merge_tags(&mut existing, imported);

        assert_eq!((added, updated), (1, 1));
        assert_eq!(existing.len(), 2);
        assert_eq!(existing[0].id, "existing-id");
        assert_eq!(existing[0].color, "#ffffff");
    }
}

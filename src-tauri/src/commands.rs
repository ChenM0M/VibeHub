#[cfg(target_os = "windows")]
use crate::process_util::silent_command;
use crate::{
    app_paths,
    gateway::{
        config::{ApiType, GatewayConfig},
        GatewayConfigPath, GatewayState,
    },
    launcher::Launcher,
    local_agent_usage::{self, LocalAgentUsageOverview, TaskSessionProviderLinks},
    models::*,
    scanner::Scanner,
    storage::Storage,
    updater,
    vibehub::cockpit,
    vibehub::project_structure,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::process::Command;
use std::sync::Mutex;
use tauri::State;
use vibehub_core::{
    legacy_v2::{self, LegacyV2Archive},
    v3::{
        self, AgentSpecInspection, AgentSpecSyncRequest, AgentSpecSyncResult, AppendResult,
        LifecycleCommand, MemoryCommand, MemoryEntry, MemoryQuery, OrchestrationCommand,
        PlanAddNodeCommand, PlanSetCriteriaCommand, PlanSetDependenciesCommand,
        PlanSetStateCommand, ProjectLayoutStatus, V3ApplicationService, V3BootstrapResult,
        V3ProjectSettingsInspection, V3ProjectSettingsUpdateRequest, V3RepairCandidate,
        V3RepairResult, V3TaskCreateRequest, V3TaskCreateResult, V3ViewBundle, V3ViewRepository,
    },
};

pub struct AppState {
    pub storage: Mutex<Storage>,
}

#[tauri::command]
pub async fn legacy_v2_load_archive(project_path: String) -> Result<LegacyV2Archive, String> {
    tokio::task::spawn_blocking(move || {
        legacy_v2::load_archive(project_path).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("LEGACY_V2_ARCHIVE_TASK_FAILED: {error}"))?
}

#[tauri::command]
pub async fn v3_inspect_project_layout(
    project_path: String,
) -> Result<ProjectLayoutStatus, String> {
    tokio::task::spawn_blocking(move || {
        v3::inspect_project_layout(project_path).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("V3_LAYOUT_TASK_FAILED: {error}"))?
}

#[tauri::command]
pub async fn v3_initialize_project(project_path: String) -> Result<V3BootstrapResult, String> {
    tokio::task::spawn_blocking(move || {
        v3::initialize_v3(project_path).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("V3_INIT_TASK_FAILED: {error}"))?
}

#[tauri::command]
pub async fn v3_migrate_project(project_path: String) -> Result<V3BootstrapResult, String> {
    tokio::task::spawn_blocking(move || {
        v3::migrate_v2_to_v3(project_path).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("V3_MIGRATION_TASK_FAILED: {error}"))?
}

#[tauri::command]
pub async fn v3_recover_project_migration(
    project_path: String,
) -> Result<V3BootstrapResult, String> {
    tokio::task::spawn_blocking(move || {
        v3::recover_interrupted_migration(project_path).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("V3_MIGRATION_RECOVERY_TASK_FAILED: {error}"))?
}

#[tauri::command]
pub async fn v3_inspect_project_repair_candidates(
    project_path: String,
) -> Result<Vec<V3RepairCandidate>, String> {
    tokio::task::spawn_blocking(move || {
        v3::inspect_v3_repair_candidates(project_path).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("V3_REPAIR_INSPECTION_TASK_FAILED: {error}"))?
}

#[tauri::command]
pub async fn v3_repair_project(
    project_path: String,
    task_id: String,
) -> Result<V3RepairResult, String> {
    tokio::task::spawn_blocking(move || {
        v3::repair_v3_layout(project_path, &task_id).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("V3_REPAIR_TASK_FAILED: {error}"))?
}

#[tauri::command]
pub async fn v3_get_project_settings(
    project_path: String,
) -> Result<V3ProjectSettingsInspection, String> {
    tokio::task::spawn_blocking(move || {
        v3::read_project_settings(project_path).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("V3_PROJECT_SETTINGS_READ_TASK_FAILED: {error}"))?
}

#[tauri::command]
pub async fn v3_update_project_settings(
    project_path: String,
    request: V3ProjectSettingsUpdateRequest,
) -> Result<v3::V3ProjectSettings, String> {
    tokio::task::spawn_blocking(move || {
        v3::update_project_settings(project_path, request).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("V3_PROJECT_SETTINGS_UPDATE_TASK_FAILED: {error}"))?
}

#[tauri::command]
pub async fn v3_agent_specs_status(project_path: String) -> Result<AgentSpecInspection, String> {
    tokio::task::spawn_blocking(move || {
        v3::inspect_agent_specs(project_path).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("V3_AGENT_SPECS_STATUS_TASK_FAILED: {error}"))?
}

#[tauri::command]
pub async fn v3_agent_specs_sync(
    project_path: String,
    request: AgentSpecSyncRequest,
) -> Result<AgentSpecSyncResult, String> {
    tokio::task::spawn_blocking(move || {
        v3::sync_agent_specs(project_path, request).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("V3_AGENT_SPECS_SYNC_TASK_FAILED: {error}"))?
}

#[tauri::command]
pub async fn v3_create_task(
    project_path: String,
    request: V3TaskCreateRequest,
) -> Result<V3TaskCreateResult, String> {
    tokio::task::spawn_blocking(move || {
        v3::create_v3_task(project_path, request).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("V3_TASK_CREATE_TASK_FAILED: {error}"))?
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct V3CompleteTaskRequest {
    pub project_id: String,
    pub task_id: String,
    pub actor: String,
    pub confirmed_by: String,
    pub channel: String,
    pub idempotency_key: String,
}

#[tauri::command]
pub async fn v3_complete_task(
    project_path: String,
    request: V3CompleteTaskRequest,
) -> Result<AppendResult, String> {
    tokio::task::spawn_blocking(move || {
        V3ApplicationService::open(project_path)
            .and_then(|application| {
                application.complete_task(
                    &request.project_id,
                    &request.task_id,
                    &request.actor,
                    &request.confirmed_by,
                    &request.channel,
                    &request.idempotency_key,
                )
            })
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("V3_TASK_COMPLETE_TASK_FAILED: {error}"))?
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct V3CloseTaskWithExceptionsRequest {
    pub project_id: String,
    pub task_id: String,
    pub actor: String,
    pub confirmed_by: String,
    pub channel: String,
    pub reason: String,
    pub idempotency_key: String,
}

#[tauri::command]
pub async fn v3_close_task_with_exceptions(
    project_path: String,
    request: V3CloseTaskWithExceptionsRequest,
) -> Result<AppendResult, String> {
    tokio::task::spawn_blocking(move || {
        V3ApplicationService::open(project_path)
            .and_then(|application| {
                application.close_task_with_exceptions(
                    &request.project_id,
                    &request.task_id,
                    &request.actor,
                    &request.confirmed_by,
                    &request.channel,
                    &request.reason,
                    &request.idempotency_key,
                )
            })
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("V3_TASK_FORCE_CLOSE_TASK_FAILED: {error}"))?
}

#[tauri::command]
pub async fn v3_plan_add_node(
    project_path: String,
    command: PlanAddNodeCommand,
) -> Result<AppendResult, String> {
    tokio::task::spawn_blocking(move || {
        V3ApplicationService::open(project_path)
            .and_then(|application| application.plan_add_node(command))
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("V3_PLAN_ADD_NODE_TASK_FAILED: {error}"))?
}

#[tauri::command]
pub async fn v3_plan_set_dependencies(
    project_path: String,
    command: PlanSetDependenciesCommand,
) -> Result<AppendResult, String> {
    tokio::task::spawn_blocking(move || {
        V3ApplicationService::open(project_path)
            .and_then(|application| application.plan_set_dependencies(command))
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("V3_PLAN_SET_DEPENDENCIES_TASK_FAILED: {error}"))?
}

#[tauri::command]
pub async fn v3_plan_set_state(
    project_path: String,
    command: PlanSetStateCommand,
) -> Result<AppendResult, String> {
    tokio::task::spawn_blocking(move || {
        V3ApplicationService::open(project_path)
            .and_then(|application| application.plan_set_state(command))
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("V3_PLAN_SET_STATE_TASK_FAILED: {error}"))?
}

#[tauri::command]
pub async fn v3_plan_set_criteria(
    project_path: String,
    command: PlanSetCriteriaCommand,
) -> Result<AppendResult, String> {
    tokio::task::spawn_blocking(move || {
        V3ApplicationService::open(project_path)
            .and_then(|application| application.plan_set_criteria(command))
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("V3_PLAN_SET_CRITERIA_TASK_FAILED: {error}"))?
}

#[tauri::command]
pub async fn v3_lifecycle_typed_command(
    project_path: String,
    command: LifecycleCommand,
) -> Result<AppendResult, String> {
    tokio::task::spawn_blocking(move || {
        V3ApplicationService::open(project_path)
            .and_then(|application| application.lifecycle_command(command))
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("V3_LIFECYCLE_TYPED_TASK_FAILED: {error}"))?
}

#[tauri::command]
pub async fn v3_memory_command(
    project_path: String,
    command: MemoryCommand,
) -> Result<AppendResult, String> {
    tokio::task::spawn_blocking(move || {
        V3ApplicationService::open(project_path)
            .and_then(|application| application.memory_command(command))
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("V3_MEMORY_TASK_FAILED: {error}"))?
}

#[tauri::command]
pub async fn v3_memory_query(
    project_path: String,
    project_id: String,
    query: MemoryQuery,
) -> Result<Vec<MemoryEntry>, String> {
    tokio::task::spawn_blocking(move || {
        V3ApplicationService::open(project_path)
            .and_then(|application| application.query_project_memory(&project_id, &query))
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("V3_MEMORY_QUERY_TASK_FAILED: {error}"))?
}

#[tauri::command]
pub async fn v3_orchestration_command(
    project_path: String,
    command: OrchestrationCommand,
) -> Result<AppendResult, String> {
    tokio::task::spawn_blocking(move || {
        V3ApplicationService::open(project_path)
            .and_then(|application| application.orchestration_command(command))
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("V3_ORCHESTRATION_TASK_FAILED: {error}"))?
}

#[tauri::command]
pub async fn v3_load_view_bundle(
    project_path: String,
    task_id: Option<String>,
    node_id: Option<String>,
    expected_project_id: Option<String>,
) -> Result<V3ViewBundle, String> {
    tokio::task::spawn_blocking(move || {
        let repository =
            V3ViewRepository::open(&project_path).map_err(|error| error.to_string())?;
        let project_id = repository.project_id();
        if let Some(expected) = expected_project_id {
            if expected != project_id {
                return Err(format!(
                    "V3_IDENTITY_MISMATCH: expected project {expected}, received {project_id}"
                ));
            }
        }
        let task_id = match task_id {
            Some(task_id) => task_id,
            None => repository
                .current_task_id()
                .map_err(|error| error.to_string())?,
        };
        repository
            .load_bundle_for_node(&task_id, node_id.as_deref())
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("V3_VIEW_TASK_FAILED: {error}"))?
}

#[tauri::command]
pub async fn v3_query_project_structure(
    project_path: String,
    task_id: String,
    relative_dir: String,
    cursor: Option<String>,
    limit: Option<usize>,
    query: Option<String>,
) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        let repository = V3ViewRepository::open(project_path).map_err(|error| error.to_string())?;
        if let Some(query) = query.filter(|value| !value.trim().is_empty()) {
            repository.search_project_structure(&task_id, &query, limit.unwrap_or(200))
        } else {
            repository.load_project_structure_page(
                &task_id,
                &relative_dir,
                cursor.as_deref(),
                limit.unwrap_or(200),
            )
        }
        .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("V3_PROJECT_INDEX_TASK_FAILED: {error}"))?
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
        let output = Command::new("open")
            .arg(&path)
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            return Err(if stderr.is_empty() {
                format!("OPEN_FAILED: open exited with {}", output.status)
            } else {
                format!("OPEN_FAILED: {stderr}")
            });
        }
    }
    #[cfg(target_os = "linux")]
    {
        let output = Command::new("xdg-open")
            .arg(&path)
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            return Err(if stderr.is_empty() {
                format!("OPEN_FAILED: xdg-open exited with {}", output.status)
            } else {
                format!("OPEN_FAILED: {stderr}")
            });
        }
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
pub async fn vibehub_read_local_agent_usage(
    project_path: String,
    task_id: Option<String>,
) -> Result<LocalAgentUsageOverview, String> {
    tokio::task::spawn_blocking(move || {
        let task_id = task_id.filter(|value| !value.trim().is_empty());
        // The shared mtime/size cache keeps repeated panel refreshes off the
        // transcript and SQLite files when nothing on disk changed.
        let cache = local_agent_usage::shared_usage_cache();
        let usage = if let Some(task_id) = task_id {
            let repository =
                V3ViewRepository::open(&project_path).map_err(|error| error.to_string())?;
            let bundle = repository
                .load_bundle(&task_id)
                .map_err(|error| error.to_string())?;
            let session_links = task_session_links(&bundle.task_timeline);
            local_agent_usage::read_local_agent_usage_for_task_with_cache(
                project_path,
                task_id,
                session_links,
                cache,
            )
        } else {
            local_agent_usage::read_local_agent_usage_with_cache(project_path, cache)
        };
        usage.map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("LOCAL_AGENT_USAGE_TASK_FAILED: {error}"))?
}

fn task_session_links(task_timeline: &Value) -> TaskSessionProviderLinks {
    let mut links = task_timeline
        .get("lanes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|lane| lane.get("kind").and_then(Value::as_str) == Some("session"))
        .filter_map(|lane| lane.get("lane_id").and_then(Value::as_str))
        .map(|session_id| (session_id.to_owned(), BTreeMap::new()))
        .collect::<TaskSessionProviderLinks>();

    for event in task_timeline
        .get("events")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(session_id) = event.get("session_id").and_then(Value::as_str) else {
            continue;
        };
        let Some(details) = event.get("details").and_then(Value::as_object) else {
            continue;
        };
        let provider = details
            .get("provider")
            .and_then(Value::as_str)
            .or_else(|| event.get("actor").and_then(Value::as_str))
            .map(normalize_usage_provider);
        let provider_key = provider.as_deref().unwrap_or("*");
        let entry = links.entry(session_id.to_owned()).or_default();
        if let Some(provider_session_id) =
            details.get("provider_session_id").and_then(Value::as_str)
        {
            if !provider_session_id.trim().is_empty() {
                entry
                    .entry(provider_key.to_owned())
                    .or_default()
                    .insert(provider_session_id.trim().to_owned());
            }
        }
        if let Some(provider_session_ids) = details
            .get("provider_session_ids")
            .and_then(Value::as_array)
        {
            for provider_session_id in provider_session_ids.iter().filter_map(Value::as_str) {
                if !provider_session_id.trim().is_empty() {
                    entry
                        .entry(provider_key.to_owned())
                        .or_default()
                        .insert(provider_session_id.trim().to_owned());
                }
            }
        }
    }

    links
}

fn normalize_usage_provider(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "claude" | "claude-code" | "claude_code" => "claude".to_string(),
        "codex" => "codex".to_string(),
        "opencode" | "open-code" | "open_code" => "opencode".to_string(),
        _ => "*".to_string(),
    }
}

#[tauri::command]
pub async fn vibehub_open_vibehub_file(
    project_path: String,
    relative_path: String,
) -> Result<(), String> {
    let (_, path, _) = cockpit::resolve_vibehub_file_path(project_path, relative_path)
        .map_err(|e| e.to_string())?;
    if !path.is_file() {
        return Err(format!("VIBEHUB_FILE_NOT_FOUND: {}", path.display()));
    }
    open_in_explorer(path.to_string_lossy().to_string()).await
}

#[tauri::command]
pub async fn vibehub_reveal_project_file(
    project_path: String,
    task_id: String,
    relative_path: String,
) -> Result<(), String> {
    let repository = V3ViewRepository::open(&project_path).map_err(|error| error.to_string())?;
    let workspace_root = repository
        .workspace_root(&task_id)
        .map_err(|error| error.to_string())?;
    let (_, path, _) = project_structure::resolve_project_file_path(workspace_root, relative_path)
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
    task_id: String,
    relative_path: String,
) -> Result<(), String> {
    let repository = V3ViewRepository::open(&project_path).map_err(|error| error.to_string())?;
    let workspace_root = repository
        .workspace_root(&task_id)
        .map_err(|error| error.to_string())?;
    let (_, path, _) = project_structure::resolve_project_file_path(workspace_root, relative_path)
        .map_err(|e| e.to_string())?;
    open_in_explorer(path.to_string_lossy().to_string()).await
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

    #[test]
    fn task_session_links_extract_provider_identity_from_timeline_details() {
        let timeline = serde_json::json!({
            "lanes": [
                {"kind": "session", "lane_id": "session.codex.task.synthetic"}
            ],
            "events": [
                {
                    "session_id": "session.codex.task.synthetic",
                    "actor": "Codex",
                    "details": {
                        "provider": "codex",
                        "provider_session_id": "019f5fea-18e1-7732-ab81-1729fd582f7d"
                    }
                }
            ]
        });

        let links = task_session_links(&timeline);
        assert_eq!(links.len(), 1);
        assert_eq!(
            links["session.codex.task.synthetic"]["codex"],
            ["019f5fea-18e1-7732-ab81-1729fd582f7d".to_string()]
                .into_iter()
                .collect()
        );
    }

    #[tokio::test]
    #[ignore = "reads real local usage and a V3 task; set VIBEHUB_USAGE_TASK_ID"]
    async fn smoke_reads_task_scoped_usage_with_provider_links() {
        let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("repository root")
            .to_string_lossy()
            .into_owned();
        let task_id = std::env::var("VIBEHUB_USAGE_TASK_ID").expect("VIBEHUB_USAGE_TASK_ID");
        let usage = vibehub_read_local_agent_usage(project, Some(task_id))
            .await
            .expect("task-scoped local usage read");
        assert!(usage.requested_session_count > 0);
        assert!(
            usage.matched_session_count > 0,
            "{}",
            usage.warnings.join("\n")
        );
        assert!(usage.total_tokens > 0);
        eprintln!(
            "Task {} usage: matched {}/{} sessions, {} total tokens, Codex {} records",
            usage.task_id.as_deref().unwrap_or("unknown"),
            usage.matched_session_count,
            usage.requested_session_count,
            usage.total_tokens,
            usage.codex.records
        );
    }
}

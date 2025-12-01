pub mod config;
pub mod proxy;
pub mod stats;

use tauri::{AppHandle, Manager, Runtime, State};
use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::RwLock;
use self::config::GatewayConfig;
use self::stats::{StatsManager, GatewayStats};

pub struct GatewayState(pub Arc<RwLock<GatewayConfig>>);
pub struct GatewayConfigPath(pub PathBuf);
pub struct GatewayStatsState(pub Arc<StatsManager>);

#[tauri::command]
pub async fn get_gateway_config(state: State<'_, GatewayState>) -> Result<GatewayConfig, String> {
    let config = state.0.read().await;
    Ok(config.clone())
}

#[tauri::command]
pub async fn save_gateway_config(
    state: State<'_, GatewayState>,
    path_state: State<'_, GatewayConfigPath>,
    config: GatewayConfig
) -> Result<(), String> {
    let mut current_config = state.0.write().await;
    *current_config = config.clone();
    
    // Save to disk
    config.save(&path_state.0).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_gateway_stats(state: State<'_, GatewayStatsState>) -> Result<GatewayStats, String> {
    Ok(state.0.get_stats())
}

pub fn init<R: Runtime>(app: &AppHandle<R>) {
    // Calculate config path (same logic as Storage)
    let exe_path = std::env::current_exe().expect("Failed to get current exe");
    let exe_dir = exe_path.parent().expect("Failed to get exe dir");
    let data_dir = exe_dir.join("data");
    std::fs::create_dir_all(&data_dir).expect("Failed to create data dir");
    let config_path = data_dir.join("gateway_config.json");

    // Load config
    let config = GatewayConfig::load(&config_path).unwrap_or_default();
    let config_state = Arc::new(RwLock::new(config));
    
    // Init stats
    let stats_manager = Arc::new(StatsManager::new(data_dir));

    app.manage(GatewayState(config_state.clone()));
    app.manage(GatewayConfigPath(config_path));
    app.manage(GatewayStatsState(stats_manager.clone()));

    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        proxy::start_server(12345, config_state, stats_manager, app_handle).await;
    });
}

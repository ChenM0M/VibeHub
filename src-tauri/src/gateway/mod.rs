pub mod cache;
pub mod config;
pub mod converter;
pub mod proxy;
pub mod resilience;
pub mod stats;

use self::config::GatewayConfig;
use self::stats::{GatewayStats, StatsManager};
use crate::app_paths;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager, Runtime, State};
use tokio::sync::RwLock;

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
    config: GatewayConfig,
) -> Result<(), String> {
    let mut current_config = state.0.write().await;
    *current_config = config.clone();

    // Save to disk
    config.save(&path_state.0).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_gateway_stats(
    state: State<'_, GatewayStatsState>,
) -> Result<GatewayStats, String> {
    Ok(state.0.get_stats())
}

pub fn init<R: Runtime>(app: &AppHandle<R>) {
    // The active dir (env / custom / portable auto-detect / platform default)
    // is decided centrally in `app_paths::resolve_active_dir`. Gateway just
    // joins its filenames onto whatever was resolved. The previous
    // `migrate_legacy_file_if_needed` step is gone on purpose — it copied
    // portable files into AppData and then started ignoring the portable
    // originals, which is the regression we're fixing.
    let data_dir = app_paths::app_data_dir().expect("Failed to initialize app data dir");
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
        // 启动三个独立的网关服务器
        proxy::start_servers(config_state, stats_manager, app_handle).await;
    });
}

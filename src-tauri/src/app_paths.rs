use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

const APP_DIR_NAME: &str = "VibeHub";

pub fn app_data_dir() -> Result<PathBuf> {
    let data_dir = if portable_mode_enabled() {
        portable_data_dir()?
    } else {
        platform_data_dir()?
    };

    fs::create_dir_all(&data_dir)
        .with_context(|| format!("Failed to create app data dir {}", data_dir.display()))?;

    Ok(data_dir)
}

pub fn migrate_legacy_file_if_needed(file_name: &str, target_dir: &PathBuf) -> Result<PathBuf> {
    let target_path = target_dir.join(file_name);
    if target_path.exists() || portable_mode_enabled() {
        return Ok(target_path);
    }

    if let Some(legacy_path) = legacy_data_file(file_name)? {
        fs::copy(&legacy_path, &target_path).with_context(|| {
            format!(
                "Failed to migrate {} from {} to {}",
                file_name,
                legacy_path.display(),
                target_path.display()
            )
        })?;
    }

    Ok(target_path)
}

fn portable_mode_enabled() -> bool {
    std::env::var("VIBEHUB_PORTABLE")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

fn platform_data_dir() -> Result<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        return dirs::data_dir()
            .or_else(|| {
                dirs::home_dir().map(|home| home.join("Library").join("Application Support"))
            })
            .map(|base| base.join(APP_DIR_NAME))
            .context("Failed to resolve a writable macOS Application Support directory");
    }

    #[cfg(not(target_os = "macos"))]
    {
        dirs::data_dir()
            .or_else(|| dirs::home_dir().map(|home| home.join(".local").join("share")))
            .map(|base| base.join(APP_DIR_NAME))
            .context("Failed to resolve a writable application data directory")
    }
}

fn portable_data_dir() -> Result<PathBuf> {
    let exe_path = std::env::current_exe().context("Failed to get current executable path")?;
    let exe_dir = exe_path
        .parent()
        .context("Failed to get executable directory")?;
    Ok(exe_dir.join("data"))
}

fn legacy_data_file(file_name: &str) -> Result<Option<PathBuf>> {
    let legacy_path = portable_data_dir()?.join(file_name);
    Ok(legacy_path.exists().then_some(legacy_path))
}

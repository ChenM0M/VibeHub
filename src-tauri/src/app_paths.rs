use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const APP_DIR_NAME: &str = "VibeHub";
const POINTER_FILE_NAME: &str = "storage-pointer.json";
const PORTABLE_PROBE_FILE: &str = "config.json";

/// Where the active data directory came from. Surfaced to the UI so users
/// know why VibeHub is reading / writing from a given path.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StorageSource {
    /// `VIBEHUB_PORTABLE=1` env var forced portable mode.
    Env,
    /// User-set custom dir from the pointer file.
    Custom,
    /// Auto-detected because `<exe>/data/config.json` is a valid AppConfig.
    Portable,
    /// Platform default (`%APPDATA%\VibeHub`, `~/Library/Application Support/VibeHub`, etc.).
    Default,
}

/// Persisted on disk at `<platform_default>/storage-pointer.json`. Always
/// stored at the platform-default location regardless of which dir is
/// currently active, so we have a stable place to read it back from on
/// startup (avoids the chicken-and-egg of "where do I find my custom path
/// setting if it lives in my custom path").
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StoragePointer {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_data_dir: Option<String>,
    #[serde(default)]
    pub migration_notice_dismissed: bool,
    #[serde(default)]
    pub custom_dir_notice_dismissed: bool,
}

/// Snapshot of what app_paths resolved on this run plus the inputs that
/// would feed a future restart. UI surfaces this so the user understands
/// the resolution and can pick a different dir.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageInfo {
    pub active_dir: String,
    pub source: StorageSource,
    pub custom_dir: Option<String>,
    pub default_dir: String,
    pub portable_dir: Option<String>,
    /// `<exe>/data/config.json` parsed as a valid VibeHub config.
    pub portable_available: bool,
    /// Both the portable dir AND the platform default dir have a `config.json`.
    /// Used to drive the "two copies detected" notice in Settings.
    pub dual_data_detected: bool,
    pub migration_notice_dismissed: bool,
    pub custom_dir_notice_dismissed: bool,
}

/// Resolve the active data directory. This is the entry point used by
/// `Storage::new` and `gateway::init` at startup. Resolution order:
/// 1. `VIBEHUB_PORTABLE=1` env var → `<exe>/data`
/// 2. User-set custom dir from the pointer
/// 3. Auto-detected portable (`<exe>/data/config.json` is valid)
/// 4. Platform default
pub fn app_data_dir() -> Result<PathBuf> {
    let (dir, _source) = resolve_active_dir()?;
    fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create app data dir {}", dir.display()))?;
    Ok(dir)
}

/// Same as `app_data_dir` but also returns the resolution source. Used by
/// `get_storage_info` and anywhere we want to log why we picked this dir.
pub fn resolve_active_dir() -> Result<(PathBuf, StorageSource)> {
    if env_portable_enabled() {
        return Ok((portable_data_dir()?, StorageSource::Env));
    }

    let pointer = load_pointer().unwrap_or_default();
    if let Some(custom) = pointer.custom_data_dir.as_ref() {
        let path = PathBuf::from(custom);
        if !path.as_os_str().is_empty() {
            return Ok((path, StorageSource::Custom));
        }
    }

    if portable_available()? {
        return Ok((portable_data_dir()?, StorageSource::Portable));
    }

    Ok((platform_data_dir()?, StorageSource::Default))
}

/// Build the full `StorageInfo` snapshot for the UI.
pub fn storage_info() -> Result<StorageInfo> {
    let (active, source) = resolve_active_dir()?;
    let default_dir = platform_data_dir()?;
    let portable_dir = portable_data_dir().ok();
    let portable_available = portable_available().unwrap_or(false);
    let pointer = load_pointer().unwrap_or_default();

    // Dual-data notice: portable dir has a valid config AND the platform
    // default also has one. Most often happens after the f80687f upgrade,
    // where the old logic copied portable → AppData.
    let dual_data_detected =
        portable_available && default_dir.join(PORTABLE_PROBE_FILE).is_file();

    Ok(StorageInfo {
        active_dir: active.to_string_lossy().into_owned(),
        source,
        custom_dir: pointer.custom_data_dir.clone(),
        default_dir: default_dir.to_string_lossy().into_owned(),
        portable_dir: portable_dir.map(|p| p.to_string_lossy().into_owned()),
        portable_available,
        dual_data_detected,
        migration_notice_dismissed: pointer.migration_notice_dismissed,
        custom_dir_notice_dismissed: pointer.custom_dir_notice_dismissed,
    })
}

/// Persist a user-chosen custom data directory. Performs basic writability
/// validation so we fail loudly in the UI instead of silently bricking the
/// next startup.
pub fn set_custom_data_dir(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() {
        anyhow::bail!("Custom data directory path is empty");
    }

    fs::create_dir_all(path)
        .with_context(|| format!("Failed to create or access {}", path.display()))?;

    let probe = path.join(".vibehub-write-probe");
    fs::write(&probe, b"vibehub")
        .with_context(|| format!("Directory is not writable: {}", path.display()))?;
    let _ = fs::remove_file(&probe);

    let mut pointer = load_pointer().unwrap_or_default();
    pointer.custom_data_dir = Some(path.to_string_lossy().into_owned());
    save_pointer(&pointer)?;
    Ok(())
}

pub fn clear_custom_data_dir() -> Result<()> {
    let mut pointer = load_pointer().unwrap_or_default();
    pointer.custom_data_dir = None;
    save_pointer(&pointer)
}

pub fn dismiss_migration_notice() -> Result<()> {
    let mut pointer = load_pointer().unwrap_or_default();
    pointer.migration_notice_dismissed = true;
    save_pointer(&pointer)
}

pub fn dismiss_custom_dir_notice() -> Result<()> {
    let mut pointer = load_pointer().unwrap_or_default();
    pointer.custom_dir_notice_dismissed = true;
    save_pointer(&pointer)
}

fn env_portable_enabled() -> bool {
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

/// Returns true iff `<exe>/data/config.json` exists AND parses as a JSON
/// object containing the VibeHub-spec top-level keys. We deliberately do
/// NOT call `serde_json::from_str::<AppConfig>` here — that would couple
/// portable detection to the exact serde schema and silently break old
/// portable installs on any future schema change. A loose shape check is
/// enough to distinguish "real VibeHub data" from "some random data/ folder
/// the user happened to leave next to the exe".
fn portable_available() -> Result<bool> {
    let probe = match portable_data_dir() {
        Ok(dir) => dir.join(PORTABLE_PROBE_FILE),
        Err(_) => return Ok(false),
    };
    if !probe.is_file() {
        return Ok(false);
    }
    let content = match fs::read_to_string(&probe) {
        Ok(c) => c,
        Err(_) => return Ok(false),
    };
    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Ok(false),
    };
    let obj = match value.as_object() {
        Some(o) => o,
        None => return Ok(false),
    };
    // Require the three array fields that have been part of AppConfig since
    // the very first portable build. Cheaper than a full AppConfig parse
    // and forward-compatible with schema additions.
    let required = ["workspaces", "tags", "projects"];
    let ok = required
        .iter()
        .all(|key| obj.get(*key).map(|v| v.is_array()).unwrap_or(false));
    Ok(ok)
}

fn pointer_path() -> Result<PathBuf> {
    let dir = platform_data_dir()?;
    fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create pointer dir {}", dir.display()))?;
    Ok(dir.join(POINTER_FILE_NAME))
}

fn load_pointer() -> Result<StoragePointer> {
    let path = pointer_path()?;
    if !path.is_file() {
        return Ok(StoragePointer::default());
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read pointer {}", path.display()))?;
    let pointer: StoragePointer = serde_json::from_str(&content).unwrap_or_default();
    Ok(pointer)
}

fn save_pointer(pointer: &StoragePointer) -> Result<()> {
    let path = pointer_path()?;
    let content =
        serde_json::to_string_pretty(pointer).context("Failed to serialize storage pointer")?;
    fs::write(&path, content)
        .with_context(|| format!("Failed to write pointer {}", path.display()))?;
    Ok(())
}

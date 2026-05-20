use crate::app_paths;
use crate::models::AppConfig;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

pub struct Storage {
    config_path: PathBuf,
}

impl Storage {
    pub fn new() -> Result<Self> {
        let data_dir = app_paths::app_data_dir()?;
        let config_path = app_paths::migrate_legacy_file_if_needed("config.json", &data_dir)
            .context("Failed to initialize config storage")?;

        Ok(Self { config_path })
    }

    pub fn load_config(&self) -> Result<AppConfig> {
        if !self.config_path.exists() {
            // Create default config if it doesn't exist
            let config = AppConfig::default();
            self.save_config(&config)?;
            return Ok(config);
        }

        let content =
            fs::read_to_string(&self.config_path).context("Failed to read config file")?;

        let config: AppConfig =
            serde_json::from_str(&content).context("Failed to parse config file")?;

        Ok(config)
    }

    pub fn save_config(&self, config: &AppConfig) -> Result<()> {
        let content = serde_json::to_string_pretty(config).context("Failed to serialize config")?;

        fs::write(&self.config_path, content).context("Failed to write config file")?;

        Ok(())
    }
}

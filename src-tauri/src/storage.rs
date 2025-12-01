use crate::models::AppConfig;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

pub struct Storage {
    config_path: PathBuf,
}

impl Storage {
    pub fn new() -> Result<Self> {
        let exe_path = std::env::current_exe()?;
        let exe_dir = exe_path
            .parent()
            .context("Failed to get executable directory")?;
        
        // Portable mode: store data next to executable
        let data_dir = exe_dir.join("data");
        fs::create_dir_all(&data_dir)?;
        
        let config_path = data_dir.join("config.json");
        
        Ok(Self { config_path })
    }

    pub fn load_config(&self) -> Result<AppConfig> {
        if !self.config_path.exists() {
            // Create default config if it doesn't exist
            let config = AppConfig::default();
            self.save_config(&config)?;
            return Ok(config);
        }

        let content = fs::read_to_string(&self.config_path)
            .context("Failed to read config file")?;
        
        let config: AppConfig = serde_json::from_str(&content)
            .context("Failed to parse config file")?;
        
        Ok(config)
    }

    pub fn save_config(&self, config: &AppConfig) -> Result<()> {
        let content = serde_json::to_string_pretty(config)
            .context("Failed to serialize config")?;
        
        fs::write(&self.config_path, content)
            .context("Failed to write config file")?;
        
        Ok(())
    }
}

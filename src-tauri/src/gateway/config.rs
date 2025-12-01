use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use anyhow::{Context, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub model_mapping: HashMap<String, String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub port: u16,
    pub enabled: bool,
    pub providers: Vec<Provider>,
    pub fallback_enabled: bool,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            port: 12345,
            enabled: true,
            providers: vec![],
            fallback_enabled: true,
        }
    }
}

impl GatewayConfig {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        if !path.as_ref().exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path).context("Failed to read gateway config")?;
        serde_json::from_str(&content).context("Failed to parse gateway config")
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_json::to_string_pretty(self).context("Failed to serialize gateway config")?;
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent).context("Failed to create config directory")?;
        }
        fs::write(path, content).context("Failed to write gateway config")
    }
}

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use anyhow::{Context, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ApiType {
    #[default]
    Anthropic,      // /v1/messages - Claude Code
    OpenAIResponses, // /v1/responses - CodeX
    OpenAIChat,     // /v1/chat/completions - Cline, Continue, etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    #[serde(default)]
    pub model_mapping: HashMap<String, String>,
    pub enabled: bool,
    
    // 供应商支持的 API 类型
    #[serde(default = "default_api_types")]
    pub api_types: Vec<ApiType>,
    
    // 供应商权重 (用于负载均衡, 越高优先级越高)
    #[serde(default = "default_weight")]
    pub weight: u32,
    
    // 费率配置 ($/1K tokens)
    #[serde(default)]
    pub input_price_per_1k: f64,
    #[serde(default)]
    pub output_price_per_1k: f64,
}

fn default_api_types() -> Vec<ApiType> {
    vec![ApiType::Anthropic] // 默认为 Anthropic 以兼容旧配置
}

fn default_weight() -> u32 {
    100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    // 三个独立端口配置
    #[serde(default = "default_anthropic_port")]
    pub anthropic_port: u16,
    #[serde(default = "default_responses_port")]
    pub responses_port: u16,
    #[serde(default = "default_chat_port")]
    pub chat_port: u16,
    
    // 三个独立开关
    #[serde(default = "default_true")]
    pub anthropic_enabled: bool,
    #[serde(default = "default_true")]
    pub responses_enabled: bool,
    #[serde(default = "default_true")]
    pub chat_enabled: bool,
    
    // 旧字段兼容 (已废弃，用于迁移)
    #[serde(default)]
    pub port: u16,
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    pub providers: Vec<Provider>,
    #[serde(default = "default_true")]
    pub fallback_enabled: bool,
    
    // 缓存配置
    #[serde(default)]
    pub cache_enabled: bool,
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_seconds: u64,
    #[serde(default = "default_cache_max_entries")]
    pub cache_max_entries: usize,
    
    // 熔断配置
    #[serde(default = "default_cooldown")]
    pub circuit_breaker_cooldown_seconds: u64,
}

fn default_anthropic_port() -> u16 { 12345 }
fn default_responses_port() -> u16 { 12346 }
fn default_chat_port() -> u16 { 12347 }
fn default_true() -> bool { true }
fn default_cache_ttl() -> u64 { 600 } // 10 分钟
fn default_cache_max_entries() -> usize { 1000 }
fn default_cooldown() -> u64 { 60 }

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            anthropic_port: 12345,
            responses_port: 12346,
            chat_port: 12347,
            anthropic_enabled: true,
            responses_enabled: true,
            chat_enabled: true,
            port: 0, // 废弃字段
            enabled: true,
            providers: vec![],
            fallback_enabled: true,
            cache_enabled: true,
            cache_ttl_seconds: 600,
            cache_max_entries: 1000,
            circuit_breaker_cooldown_seconds: 60,
        }
    }
}

impl GatewayConfig {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        if !path.as_ref().exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path).context("Failed to read gateway config")?;
        let mut config: GatewayConfig = serde_json::from_str(&content).context("Failed to parse gateway config")?;
        
        // 自动迁移：如果旧的 port 字段有值，迁移到新字段
        if config.port != 0 {
            config.anthropic_port = config.port;
            config.port = 0;
            // 保存迁移后的配置
            let _ = config.save(&path);
            println!("Migrated gateway config: port {} -> anthropic_port", config.anthropic_port);
        }
        
        // 自动迁移：为没有 api_types 的供应商添加默认值
        for provider in &mut config.providers {
            if provider.api_types.is_empty() {
                // 根据名字猜测类型
                let name_lower = provider.name.to_lowercase();
                if name_lower.contains("claude") || name_lower.contains("anthropic") {
                    provider.api_types = vec![ApiType::Anthropic];
                } else if name_lower.contains("openai") || name_lower.contains("gpt") {
                    provider.api_types = vec![ApiType::OpenAIResponses, ApiType::OpenAIChat];
                } else {
                    // 默认支持所有类型
                    provider.api_types = vec![ApiType::Anthropic, ApiType::OpenAIResponses, ApiType::OpenAIChat];
                }
            }
        }
        
        Ok(config)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_json::to_string_pretty(self).context("Failed to serialize gateway config")?;
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent).context("Failed to create config directory")?;
        }
        fs::write(path, content).context("Failed to write gateway config")
    }
    
    /// 获取支持指定 API 类型的供应商列表
    pub fn get_providers_for_api_type(&self, api_type: &ApiType) -> Vec<&Provider> {
        self.providers
            .iter()
            .filter(|p| p.enabled && p.api_types.contains(api_type))
            .collect()
    }
}

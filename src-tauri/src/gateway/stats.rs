use serde::{Deserialize, Serialize};
use std::collections::{VecDeque, HashMap};
use std::sync::{Arc, Mutex};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestLog {
    pub id: String,
    pub timestamp: u64,
    pub provider: String,
    pub model: String,
    pub status: u16,
    pub duration_ms: u64,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cost: f64,
    #[serde(default = "default_path")]
    pub path: String,
    #[serde(default = "default_agent")]
    pub client_agent: String,
    #[serde(default)]
    pub api_type: String, // "anthropic", "responses", "chat"
    #[serde(default)]
    pub cached: bool,
    #[serde(default)]
    pub error_message: Option<String>,  // 完整错误信息
}

fn default_path() -> String { "/".to_string() }
fn default_agent() -> String { "unknown".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderStats {
    pub provider_id: String,
    pub provider_name: String,
    
    // 请求统计
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    
    // 延迟统计 (毫秒)
    pub avg_latency_ms: f64,
    pub min_latency_ms: u64,
    pub max_latency_ms: u64,
    pub p50_latency_ms: u64,
    pub p95_latency_ms: u64,
    pub p99_latency_ms: u64,
    
    // Token 统计
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    
    // 费用统计
    pub total_cost: f64,
    
    // 健康状态
    pub last_success_at: Option<u64>,
    pub last_failure_at: Option<u64>,
    pub last_error_message: Option<String>,
    pub consecutive_failures: u32,
    pub is_healthy: bool,
    
    // 延迟样本 (用于计算分位数，保留最近100个)
    #[serde(skip)]
    latency_samples: VecDeque<u64>,
}

impl ProviderStats {
    pub fn new(id: String, name: String) -> Self {
        Self {
            provider_id: id,
            provider_name: name,
            is_healthy: true,
            latency_samples: VecDeque::with_capacity(100),
            ..Default::default()
        }
    }
    
    pub fn record_request(&mut self, success: bool, latency_ms: u64, input_tokens: u32, output_tokens: u32, cost: f64, timestamp: u64, error_msg: Option<String>) {
        self.total_requests += 1;
        
        if success {
            self.successful_requests += 1;
            self.last_success_at = Some(timestamp);
            self.consecutive_failures = 0;
            self.is_healthy = true;
            
            // 更新延迟统计
            self.latency_samples.push_back(latency_ms);
            if self.latency_samples.len() > 100 {
                self.latency_samples.pop_front();
            }
            self.update_latency_stats();
        } else {
            self.failed_requests += 1;
            self.last_failure_at = Some(timestamp);
            self.last_error_message = error_msg;
            self.consecutive_failures += 1;
            
            // 连续失败3次标记为不健康
            if self.consecutive_failures >= 3 {
                self.is_healthy = false;
            }
        }
        
        self.total_input_tokens += input_tokens as u64;
        self.total_output_tokens += output_tokens as u64;
        self.total_cost += cost;
    }
    
    fn update_latency_stats(&mut self) {
        if self.latency_samples.is_empty() {
            return;
        }
        
        let mut sorted: Vec<u64> = self.latency_samples.iter().copied().collect();
        sorted.sort();
        
        let len = sorted.len();
        self.min_latency_ms = sorted[0];
        self.max_latency_ms = sorted[len - 1];
        self.avg_latency_ms = sorted.iter().sum::<u64>() as f64 / len as f64;
        self.p50_latency_ms = sorted[len / 2];
        self.p95_latency_ms = sorted[(len as f64 * 0.95) as usize];
        self.p99_latency_ms = sorted[(len as f64 * 0.99).min(len as f64 - 1.0) as usize];
    }
    
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 100.0;
        }
        (self.successful_requests as f64 / self.total_requests as f64) * 100.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HourlyStat {
    pub timestamp: u64,
    pub requests: u32,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GatewayStats {
    // 全局统计
    pub total_requests: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost: f64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    
    // 按 API 类型统计
    pub anthropic_requests: u64,
    pub responses_requests: u64,
    pub chat_requests: u64,
    
    // 每供应商统计
    #[serde(default)]
    pub provider_stats: HashMap<String, ProviderStats>,
    
    pub recent_requests: VecDeque<RequestLog>,
    pub hourly_activity: Vec<HourlyStat>,
}

pub struct StatsManager {
    stats: Arc<Mutex<GatewayStats>>,
    file_path: PathBuf,
}

impl StatsManager {
    pub fn new(app_dir: PathBuf) -> Self {
        let file_path = app_dir.join("gateway_stats.json");
        let stats = if file_path.exists() {
            fs::read_to_string(&file_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            GatewayStats::default()
        };

        Self {
            stats: Arc::new(Mutex::new(stats)),
            file_path,
        }
    }

    pub fn get_stats(&self) -> GatewayStats {
        self.stats.lock().unwrap().clone()
    }

    pub fn record_request(&self, log: RequestLog) {
        let mut stats = self.stats.lock().unwrap();
        
        stats.total_requests += 1;
        stats.total_input_tokens += log.input_tokens as u64;
        stats.total_output_tokens += log.output_tokens as u64;
        stats.total_cost += log.cost;
        
        // 按 API 类型统计
        match log.api_type.as_str() {
            "anthropic" => stats.anthropic_requests += 1,
            "responses" => stats.responses_requests += 1,
            "chat" => stats.chat_requests += 1,
            _ => {}
        }
        
        // 更新供应商统计
        let is_success = log.status >= 200 && log.status < 300;
        let provider_stats = stats.provider_stats
            .entry(log.provider.clone())
            .or_insert_with(|| ProviderStats::new(log.provider.clone(), log.provider.clone()));
        
        provider_stats.record_request(
            is_success,
            log.duration_ms,
            log.input_tokens,
            log.output_tokens,
            log.cost,
            log.timestamp,
            if is_success { None } else { log.error_message.clone().or_else(|| Some(format!("HTTP {}", log.status))) }
        );
        
        // 更新 recent_requests
        stats.recent_requests.push_front(log.clone());
        if stats.recent_requests.len() > 50 {
            stats.recent_requests.pop_back();
        }

        // 更新 hourly_activity
        let hour_timestamp = (log.timestamp / 3600) * 3600;
        if let Some(last) = stats.hourly_activity.last_mut() {
            if last.timestamp == hour_timestamp {
                last.requests += 1;
                last.input_tokens += log.input_tokens;
                last.output_tokens += log.output_tokens;
                last.cost += log.cost;
            } else {
                stats.hourly_activity.push(HourlyStat {
                    timestamp: hour_timestamp,
                    requests: 1,
                    input_tokens: log.input_tokens,
                    output_tokens: log.output_tokens,
                    cost: log.cost,
                });
            }
        } else {
            stats.hourly_activity.push(HourlyStat {
                timestamp: hour_timestamp,
                requests: 1,
                input_tokens: log.input_tokens,
                output_tokens: log.output_tokens,
                cost: log.cost,
            });
        }
        
        // 保留最近24小时
        if stats.hourly_activity.len() > 24 {
            stats.hourly_activity.remove(0);
        }

        // 持久化
        if let Ok(json) = serde_json::to_string_pretty(&*stats) {
            if let Err(e) = fs::write(&self.file_path, json) {
                eprintln!("Failed to save stats: {}", e);
            }
        }
    }
    
    pub fn record_cache_hit(&self) {
        let mut stats = self.stats.lock().unwrap();
        stats.cache_hits += 1;
    }
    
    pub fn record_cache_miss(&self) {
        let mut stats = self.stats.lock().unwrap();
        stats.cache_misses += 1;
    }
    
    /// 重置供应商健康状态（当冷却解除时调用）
    pub fn reset_provider_health(&self, provider_name: &str) {
        let mut stats = self.stats.lock().unwrap();
        if let Some(provider_stats) = stats.provider_stats.get_mut(provider_name) {
            provider_stats.is_healthy = true;
            provider_stats.consecutive_failures = 0;
        }
    }
}

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
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
}

fn default_agent() -> String {
    "unknown".to_string()
}

fn default_path() -> String {
    "/".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GatewayStats {
    pub total_requests: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost: f64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    // We keep a limited history of recent requests for the UI
    pub recent_requests: VecDeque<RequestLog>,
    // Hourly stats for charts (timestamp -> count)
    // Simplified for now: just a list of hourly data points
    pub hourly_activity: Vec<HourlyStat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyStat {
    pub timestamp: u64, // Start of the hour
    pub requests: u32,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cost: f64,
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
        
        // Update recent requests (keep last 50)
        stats.recent_requests.push_front(log.clone());
        if stats.recent_requests.len() > 50 {
            stats.recent_requests.pop_back();
        }

        // Update hourly stats
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
        
        // Trim hourly stats (keep last 24 hours)
        if stats.hourly_activity.len() > 24 {
            stats.hourly_activity.remove(0);
        }

        println!("Recording stats: {} requests, last status: {}", stats.total_requests, log.status);

        // Persist asynchronously or immediately? For simplicity, immediately for now, but catch errors
        if let Ok(json) = serde_json::to_string_pretty(&*stats) {
            if let Err(e) = fs::write(&self.file_path, json) {
                eprintln!("Failed to save stats: {}", e);
            }
        } else {
            eprintln!("Failed to serialize stats");
        }
    }
}

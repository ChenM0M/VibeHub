use axum::{
    body::Body,
    extract::{State, Request},
    response::{IntoResponse, Response},
    routing::any,
    Router,
    http::{StatusCode, HeaderValue},
};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::gateway::config::{GatewayConfig, ApiType};
use crate::gateway::stats::{StatsManager, RequestLog};
use crate::gateway::cache::CacheManager;
use tower_http::cors::CorsLayer;
use reqwest::Client;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Runtime};
use dashmap::DashMap;

pub struct ProxyState<R: Runtime> {
    pub config: Arc<RwLock<GatewayConfig>>,
    pub stats: Arc<StatsManager>,
    pub cache: Arc<CacheManager>,
    pub app: AppHandle<R>,
    pub health_status: Arc<DashMap<String, u64>>,
    pub api_type: ApiType,
}

impl<R: Runtime> Clone for ProxyState<R> {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            stats: self.stats.clone(),
            cache: self.cache.clone(),
            app: self.app.clone(),
            health_status: self.health_status.clone(),
            api_type: self.api_type.clone(),
        }
    }
}

#[derive(Clone, serde::Serialize)]
struct ProviderStatusEvent {
    provider_id: String,
    status: String,
    api_type: String,
}

/// 启动三个独立的网关服务器
pub async fn start_servers<R: Runtime>(
    config: Arc<RwLock<GatewayConfig>>,
    stats: Arc<StatsManager>,
    app: AppHandle<R>,
) {
    let cfg = config.read().await;
    
    let cache = Arc::new(CacheManager::new(
        cfg.cache_max_entries,
        cfg.cache_ttl_seconds,
    ));
    let health_status = Arc::new(DashMap::new());
    
    let anthropic_port = cfg.anthropic_port;
    let responses_port = cfg.responses_port;
    let chat_port = cfg.chat_port;
    
    let anthropic_enabled = cfg.anthropic_enabled;
    let responses_enabled = cfg.responses_enabled;
    let chat_enabled = cfg.chat_enabled;
    
    drop(cfg);
    
    // 启动 Anthropic 网关 (Claude Code)
    if anthropic_enabled {
        let state = ProxyState {
            config: config.clone(),
            stats: stats.clone(),
            cache: cache.clone(),
            app: app.clone(),
            health_status: health_status.clone(),
            api_type: ApiType::Anthropic,
        };
        
        tokio::spawn(async move {
            start_single_server(anthropic_port, state, "Anthropic").await;
        });
    }
    
    // 启动 OpenAI Responses 网关 (CodeX)
    if responses_enabled {
        let state = ProxyState {
            config: config.clone(),
            stats: stats.clone(),
            cache: cache.clone(),
            app: app.clone(),
            health_status: health_status.clone(),
            api_type: ApiType::OpenAIResponses,
        };
        
        tokio::spawn(async move {
            start_single_server(responses_port, state, "OpenAI Responses").await;
        });
    }
    
    // 启动 OpenAI Chat 网关 (Cline/Continue)
    if chat_enabled {
        let state = ProxyState {
            config: config.clone(),
            stats: stats.clone(),
            cache: cache.clone(),
            app: app.clone(),
            health_status: health_status.clone(),
            api_type: ApiType::OpenAIChat,
        };
        
        tokio::spawn(async move {
            start_single_server(chat_port, state, "OpenAI Chat").await;
        });
    }
}

async fn start_single_server<R: Runtime>(port: u16, state: ProxyState<R>, name: &str) {
    let app_router = Router::new()
        .route("/*path", any(handle_request::<R>))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    println!("🚀 {} Gateway listening on {}", name, addr);
    
    match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => {
            if let Err(e) = axum::serve(listener, app_router).await {
                eprintln!("❌ {} Server error: {}", name, e);
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to bind {} to {}: {}", name, addr, e);
        }
    }
}

async fn handle_request<R: Runtime>(
    State(state): State<ProxyState<R>>,
    req: Request<Body>,
) -> Response {
    let start_time = SystemTime::now();
    let config = state.config.read().await;
    
    // 检查对应的网关是否启用
    let gateway_enabled = match state.api_type {
        ApiType::Anthropic => config.anthropic_enabled,
        ApiType::OpenAIResponses => config.responses_enabled,
        ApiType::OpenAIChat => config.chat_enabled,
    };
    
    if !gateway_enabled {
        return (StatusCode::SERVICE_UNAVAILABLE, "Gateway is disabled").into_response();
    }

    let path = req.uri().path().to_string();
    let query = req.uri().query().map(|q| format!("?{}", q)).unwrap_or_default();
    let method = req.method().clone();
    let headers = req.headers().clone();
    let user_agent = headers.get("user-agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown")
        .to_string();
    
    let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(b) => b,
        Err(_) => return (StatusCode::BAD_REQUEST, "Failed to read body").into_response(),
    };

    // 检查缓存
    if config.cache_enabled {
        let cache_key = CacheManager::generate_key(&path, &body_bytes);
        if let Some(cached) = state.cache.get(&cache_key) {
            state.stats.record_cache_hit();
            
            let mut builder = Response::builder().status(cached.status);
            if let Some(headers_mut) = builder.headers_mut() {
                for (k, v) in &cached.headers {
                    if let (Ok(name), Ok(val)) = (k.parse::<axum::http::HeaderName>(), HeaderValue::from_str(v)) {
                        headers_mut.insert(name, val);
                    }
                }
            }
            return builder.body(Body::from(cached.response_body)).unwrap_or_default();
        }
        state.stats.record_cache_miss();
    }

    // 计算 input tokens
    let input_tokens = calculate_input_tokens(&body_bytes);

    let client = Client::new();
    
    // 获取支持当前 API 类型的供应商
    let providers = config.get_providers_for_api_type(&state.api_type);
    
    if providers.is_empty() {
        return (StatusCode::SERVICE_UNAVAILABLE, "No active providers for this API type").into_response();
    }

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let cooldown = config.circuit_breaker_cooldown_seconds;
    let api_type_str = api_type_to_string(&state.api_type);

    for provider in providers {
        // Circuit Breaker Check
        if let Some(last_failure) = state.health_status.get(&provider.id) {
            if now - *last_failure < cooldown {
                println!("⏸️ Skipping unhealthy provider: {} (cooldown active)", provider.name);
                continue;
            }
        }

        // Emit Pending Event
        let _ = state.app.emit("gateway://provider-status", ProviderStatusEvent {
            provider_id: provider.id.clone(),
            status: "pending".to_string(),
            api_type: api_type_str.clone(),
        });

        // Construct target URL
        let base = provider.base_url.trim_end_matches('/');
        let url = format!("{}{}{}", base, path, query);
        
        println!("🔄 [{}] Forwarding to: {}", api_type_str, url);

        let mut new_req = client.request(method.clone(), &url);
        
        // Forward headers
        for (key, value) in &headers {
            if key != "host" && key != "authorization" && key != "content-length" {
                new_req = new_req.header(key, value);
            }
        }
        
        // Add Provider Auth
        if !provider.api_key.is_empty() {
            match state.api_type {
                ApiType::Anthropic => {
                    if let Ok(val) = HeaderValue::from_str(&provider.api_key) {
                        new_req = new_req.header("x-api-key", val);
                        new_req = new_req.header("anthropic-version", "2023-06-01");
                    }
                }
                ApiType::OpenAIResponses | ApiType::OpenAIChat => {
                    let auth_val = format!("Bearer {}", provider.api_key);
                    if let Ok(val) = HeaderValue::from_str(&auth_val) {
                        new_req = new_req.header("Authorization", val);
                    }
                }
            }
        }
        
        new_req = new_req.body(body_bytes.clone());

        match new_req.send().await {
            Ok(resp) => {
                let status = resp.status();
                
                let should_fallback = status.is_server_error() || 
                                      status == StatusCode::UNAUTHORIZED || 
                                      status == StatusCode::PAYMENT_REQUIRED || 
                                      status == StatusCode::FORBIDDEN || 
                                      status == StatusCode::GONE ||
                                      status == StatusCode::TOO_MANY_REQUESTS;

                if should_fallback && config.fallback_enabled {
                    println!("⚠️ Provider {} failed with status {}, trying next...", provider.name, status);
                    
                    let _ = state.app.emit("gateway://provider-status", ProviderStatusEvent {
                        provider_id: provider.id.clone(),
                        status: "error".to_string(),
                        api_type: api_type_str.clone(),
                    });

                    state.health_status.insert(provider.id.clone(), now);

                    let duration = SystemTime::now().duration_since(start_time).unwrap_or_default().as_millis() as u64;
                    let log = RequestLog {
                        id: uuid::Uuid::new_v4().to_string(),
                        timestamp: now,
                        provider: provider.name.clone(),
                        model: "unknown".to_string(),
                        status: status.as_u16(),
                        duration_ms: duration,
                        input_tokens,
                        output_tokens: 0,
                        cost: 0.0,
                        path: path.clone(),
                        client_agent: user_agent.clone(),
                        api_type: api_type_str.clone(),
                        cached: false,
                    };
                    state.stats.record_request(log);

                    continue;
                }
                
                let _ = state.app.emit("gateway://provider-status", ProviderStatusEvent {
                    provider_id: provider.id.clone(),
                    status: "success".to_string(),
                    api_type: api_type_str.clone(),
                });

                state.health_status.remove(&provider.id);

                let duration = SystemTime::now().duration_since(start_time).unwrap_or_default().as_millis() as u64;
                let output_tokens = 0; // TODO: parse from response
                let cost = calculate_cost(input_tokens, output_tokens, provider.input_price_per_1k, provider.output_price_per_1k);

                let log = RequestLog {
                    id: uuid::Uuid::new_v4().to_string(),
                    timestamp: now,
                    provider: provider.name.clone(),
                    model: "unknown".to_string(),
                    status: status.as_u16(),
                    duration_ms: duration,
                    input_tokens,
                    output_tokens,
                    cost,
                    path: path.clone(),
                    client_agent: user_agent.clone(),
                    api_type: api_type_str.clone(),
                    cached: false,
                };
                
                state.stats.record_request(log);

                // 收集响应头用于缓存
                let response_headers: Vec<(String, String)> = resp.headers()
                    .iter()
                    .filter_map(|(k, v)| {
                        v.to_str().ok().map(|v| (k.to_string(), v.to_string()))
                    })
                    .collect();

                let mut builder = Response::builder().status(status);
                
                if let Some(headers_mut) = builder.headers_mut() {
                    for (k, v) in resp.headers() {
                        headers_mut.insert(k, v.clone());
                    }
                }
                
                // 对于非流式响应，尝试缓存
                let content_type = resp.headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");
                
                if config.cache_enabled && !content_type.contains("stream") && status.is_success() {
                    // 缓冲响应体用于缓存
                    match resp.bytes().await {
                        Ok(bytes) => {
                            let cache_key = CacheManager::generate_key(&path, &body_bytes);
                            state.cache.set(cache_key, bytes.to_vec(), status.as_u16(), response_headers);
                            return builder.body(Body::from(bytes)).unwrap_or_default();
                        }
                        Err(_) => {
                            // 缓存失败，直接返回空响应
                            return builder.body(Body::empty()).unwrap_or_default();
                        }
                    }
                } else {
                    let body = Body::from_stream(resp.bytes_stream());
                    return builder.body(body).unwrap_or_default();
                }
            }
            Err(e) => {
                println!("❌ Provider {} connection failed: {}, trying next...", provider.name, e);
                
                let _ = state.app.emit("gateway://provider-status", ProviderStatusEvent {
                    provider_id: provider.id.clone(),
                    status: "error".to_string(),
                    api_type: api_type_str.clone(),
                });

                state.health_status.insert(provider.id.clone(), now);

                let duration = SystemTime::now().duration_since(start_time).unwrap_or_default().as_millis() as u64;
                let log = RequestLog {
                    id: uuid::Uuid::new_v4().to_string(),
                    timestamp: now,
                    provider: provider.name.clone(),
                    model: "unknown".to_string(),
                    status: 502,
                    duration_ms: duration,
                    input_tokens: 0,
                    output_tokens: 0,
                    cost: 0.0,
                    path: path.clone(),
                    client_agent: user_agent.clone(),
                    api_type: api_type_str.clone(),
                    cached: false,
                };
                state.stats.record_request(log);

                if !config.fallback_enabled {
                    return (StatusCode::BAD_GATEWAY, format!("Provider failed: {}", e)).into_response();
                }
            }
        }
    }

    (StatusCode::BAD_GATEWAY, "All providers failed").into_response()
}

fn calculate_input_tokens(body: &[u8]) -> u32 {
    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(body) {
        if let Some(messages) = json.get("messages").and_then(|m| m.as_array()) {
            let mut char_count = 0;
            for msg in messages {
                if let Some(content) = msg.get("content") {
                    if let Some(s) = content.as_str() {
                        char_count += s.len();
                    } else if let Some(arr) = content.as_array() {
                        for part in arr {
                            if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                                char_count += text.len();
                            }
                        }
                    }
                }
            }
            return (char_count as f64 / 4.0) as u32;
        }
    }
    (body.len() as f64 / 4.0) as u32
}

fn calculate_cost(input_tokens: u32, output_tokens: u32, input_price: f64, output_price: f64) -> f64 {
    (input_tokens as f64 / 1000.0 * input_price) + (output_tokens as f64 / 1000.0 * output_price)
}

fn api_type_to_string(api_type: &ApiType) -> String {
    match api_type {
        ApiType::Anthropic => "anthropic".to_string(),
        ApiType::OpenAIResponses => "responses".to_string(),
        ApiType::OpenAIChat => "chat".to_string(),
    }
}

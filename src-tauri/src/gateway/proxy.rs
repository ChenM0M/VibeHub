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
use crate::gateway::config::GatewayConfig;
use crate::gateway::stats::{StatsManager, RequestLog};
use tower_http::cors::CorsLayer;
use reqwest::Client;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Runtime};


pub struct ProxyState<R: Runtime> {
    pub config: Arc<RwLock<GatewayConfig>>,
    pub stats: Arc<StatsManager>,
    pub app: AppHandle<R>,
}

// Axum requires the state to be Clone + Send + Sync + 'static.
// AppHandle<R> is Clone, Send, Sync.
// Arc is Clone, Send, Sync.
// The issue is likely that #[derive(Clone)] adds a `R: Clone` bound which isn't true/needed.
// But wait, AppHandle<R> IS Clone.
// Let's try to just remove the generic R from ProxyState and use tauri::Wry if possible?
// No, we want to be generic.
// Let's manually implement Clone.

impl<R: Runtime> Clone for ProxyState<R> {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            stats: self.stats.clone(),
            app: self.app.clone(),
        }
    }
}

pub async fn start_server<R: Runtime>(port: u16, config: Arc<RwLock<GatewayConfig>>, stats: Arc<StatsManager>, app: AppHandle<R>) {
    let state = ProxyState { config, stats, app };
    
    let app_router = Router::new()
        .route("/*path", any(handle_request::<R>))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    println!("Gateway listening on {}", addr);
    
    match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => {
            if let Err(e) = axum::serve(listener, app_router).await {
                eprintln!("Server error: {}", e);
            }
        }
        Err(e) => {
            eprintln!("Failed to bind to {}: {}", addr, e);
        }
    }
}

#[derive(Clone, serde::Serialize)]
struct ProviderStatusEvent {
    provider_id: String,
    status: String, // "pending", "success", "error"
}

async fn handle_request<R: Runtime>(
    State(state): State<ProxyState<R>>,
    req: Request<Body>,
) -> Response {
    let start_time = SystemTime::now();
    let config = state.config.read().await;
    
    if !config.enabled {
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
    
    // Read body once to allow retries
    let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(b) => b,
        Err(_) => return (StatusCode::BAD_REQUEST, "Failed to read body").into_response(),
    };

    let client = Client::new();
    
    // Determine provider type based on path
    let target_provider_type = if path.starts_with("/v1/messages") {
        Some("claude")
    } else if path.starts_with("/responses") {
        Some("codex")
    } else {
        None
    };

    // Filter enabled providers
    let providers: Vec<_> = config.providers.iter().filter(|p| {
        if !p.enabled { return false; }
        if let Some(kind) = target_provider_type {
            p.name.to_lowercase().contains(kind) || p.id.to_lowercase().contains(kind)
        } else {
            true
        }
    }).collect();
    
    // If no specific providers found, fall back to ALL enabled providers
    let providers = if providers.is_empty() {
        config.providers.iter().filter(|p| p.enabled).collect()
    } else {
        providers
    };
    
    if providers.is_empty() {
        return (StatusCode::SERVICE_UNAVAILABLE, "No active providers").into_response();
    }

    for provider in providers {
        // Emit Pending Event
        let _ = state.app.emit("gateway://provider-status", ProviderStatusEvent {
            provider_id: provider.id.clone(),
            status: "pending".to_string(),
        });

        // Construct target URL
        let base = provider.base_url.trim_end_matches('/');
        let url = format!("{}{}{}", base, path, query);
        
        println!("Forwarding request to: {}", url);

        let mut new_req = client.request(method.clone(), &url);
        
        // Forward headers
        for (key, value) in &headers {
            if key != "host" && key != "authorization" && key != "content-length" {
                new_req = new_req.header(key, value);
            }
        }
        
        // Add Provider Auth
        if !provider.api_key.is_empty() {
            let auth_val = format!("Bearer {}", provider.api_key);
            if let Ok(val) = HeaderValue::from_str(&auth_val) {
                new_req = new_req.header("Authorization", val);
            }
            if provider.name.to_lowercase().contains("claude") || provider.name.to_lowercase().contains("anthropic") {
                 if let Ok(val) = HeaderValue::from_str(&provider.api_key) {
                    new_req = new_req.header("x-api-key", val);
                    new_req = new_req.header("anthropic-version", "2023-06-01"); 
                 }
            }
        }
        
        new_req = new_req.body(body_bytes.clone());

        match new_req.send().await {
            Ok(resp) => {
                let status = resp.status();
                
                // Fallback on Server Errors (5xx) or specific Client Errors (4xx)
                let should_fallback = status.is_server_error() || 
                                      status == StatusCode::UNAUTHORIZED || 
                                      status == StatusCode::PAYMENT_REQUIRED || 
                                      status == StatusCode::FORBIDDEN || 
                                      status == StatusCode::GONE ||
                                      status == StatusCode::TOO_MANY_REQUESTS;

                if should_fallback && config.fallback_enabled {
                    println!("Provider {} failed with status {}, trying next...", provider.name, status);
                    
                    // Emit Error Event
                    let _ = state.app.emit("gateway://provider-status", ProviderStatusEvent {
                        provider_id: provider.id.clone(),
                        status: "error".to_string(),
                    });

                    // Record failure stat
                    let duration = SystemTime::now().duration_since(start_time).unwrap_or_default().as_millis() as u64;
                    let log = RequestLog {
                        id: uuid::Uuid::new_v4().to_string(),
                        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
                        provider: provider.name.clone(),
                        model: "unknown".to_string(),
                        status: status.as_u16(),
                        duration_ms: duration,
                        input_tokens: (body_bytes.len() as f64 / 4.0) as u32,
                        output_tokens: 0,
                        cost: 0.0,
                        path: path.clone(),
                        client_agent: user_agent.clone(),
                    };
                    state.stats.record_request(log);

                    continue;
                }
                
                // Emit Success Event
                let _ = state.app.emit("gateway://provider-status", ProviderStatusEvent {
                    provider_id: provider.id.clone(),
                    status: "success".to_string(),
                });

                // Record Success Stats
                let duration = SystemTime::now().duration_since(start_time).unwrap_or_default().as_millis() as u64;
                let input_tokens = (body_bytes.len() as f64 / 4.0) as u32;
                let output_tokens = 0; 
                let cost = (input_tokens + output_tokens) as f64 * 0.000002;

                let log = RequestLog {
                    id: uuid::Uuid::new_v4().to_string(),
                    timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
                    provider: provider.name.clone(),
                    model: "unknown".to_string(),
                    status: status.as_u16(),
                    duration_ms: duration,
                    input_tokens,
                    output_tokens,
                    cost,
                    path: path.clone(),
                    client_agent: user_agent.clone(),
                };
                
                state.stats.record_request(log);

                let mut builder = Response::builder().status(status);
                
                if let Some(headers_mut) = builder.headers_mut() {
                    for (k, v) in resp.headers() {
                        headers_mut.insert(k, v.clone());
                    }
                }
                
                let body = Body::from_stream(resp.bytes_stream());
                return builder.body(body).unwrap_or_default();
            }
            Err(e) => {
                println!("Provider {} connection failed: {}, trying next...", provider.name, e);
                
                // Emit Error Event
                let _ = state.app.emit("gateway://provider-status", ProviderStatusEvent {
                    provider_id: provider.id.clone(),
                    status: "error".to_string(),
                });

                // Record connection failure stat
                let duration = SystemTime::now().duration_since(start_time).unwrap_or_default().as_millis() as u64;
                let log = RequestLog {
                    id: uuid::Uuid::new_v4().to_string(),
                    timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
                    provider: provider.name.clone(),
                    model: "unknown".to_string(),
                    status: 502, // Bad Gateway
                    duration_ms: duration,
                    input_tokens: 0,
                    output_tokens: 0,
                    cost: 0.0,
                    path: path.clone(),
                    client_agent: user_agent.clone(),
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


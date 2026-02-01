use axum::{
    body::Body,
    extract::{State, Request},
    response::{IntoResponse, Response},
    routing::any,
    Router,
    http::{StatusCode, HeaderValue},
};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use tokio::sync::RwLock;
use crate::gateway::config::{GatewayConfig, ApiType};
use crate::gateway::stats::{StatsManager, RequestLog};
use crate::gateway::cache::CacheManager;
use crate::gateway::converter;
use crate::gateway::resilience::{Circuit, FailureKind};
use tower_http::cors::CorsLayer;
use reqwest::Client;
use tauri::{AppHandle, Emitter, Runtime};
use dashmap::DashMap;
use tokio::sync::{Semaphore, OwnedSemaphorePermit};
use tokio::time::timeout;

pub struct ProxyState<R: Runtime> {
    pub config: Arc<RwLock<GatewayConfig>>,
    pub stats: Arc<StatsManager>,
    pub cache: Arc<CacheManager>,
    pub app: AppHandle<R>,
    pub circuits: Arc<DashMap<String, Circuit>>,
    pub inflight_limits: Arc<DashMap<String, Arc<Semaphore>>>,
    pub http_client: Client,
    pub api_type: ApiType,
}

impl<R: Runtime> Clone for ProxyState<R> {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            stats: self.stats.clone(),
            cache: self.cache.clone(),
            app: self.app.clone(),
            circuits: self.circuits.clone(),
            inflight_limits: self.inflight_limits.clone(),
            http_client: self.http_client.clone(),
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
    let circuits = Arc::new(DashMap::new());
    let inflight_limits: Arc<DashMap<String, Arc<Semaphore>>> = Arc::new(DashMap::new());

    let http_client = Client::builder()
        .connect_timeout(Duration::from_secs(3))
        .pool_idle_timeout(Duration::from_secs(90))
        .pool_max_idle_per_host(16)
        .build()
        .unwrap_or_else(|_| Client::new());
    
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
            circuits: circuits.clone(),
            inflight_limits: inflight_limits.clone(),
            http_client: http_client.clone(),
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
            circuits: circuits.clone(),
            inflight_limits: inflight_limits.clone(),
            http_client: http_client.clone(),
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
            circuits: circuits.clone(),
            inflight_limits: inflight_limits.clone(),
            http_client: http_client.clone(),
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
    const DEFAULT_MAX_ATTEMPTS: usize = 4;
    const MAX_INFLIGHT_PER_PROVIDER: usize = 4;
    const UPSTREAM_HEADERS_TIMEOUT: Duration = Duration::from_secs(15);
    const UPSTREAM_BODY_TIMEOUT: Duration = Duration::from_secs(30);

    let request_id = uuid::Uuid::new_v4().to_string();
    let overall_start = SystemTime::now();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

    // Read config quickly (do NOT hold across awaits).
    let (gateway_enabled, cache_enabled, fallback_enabled, base_cooldown_seconds, providers) = {
        let config = state.config.read().await;
        let gateway_enabled = match state.api_type {
            ApiType::Anthropic => config.anthropic_enabled,
            ApiType::OpenAIResponses => config.responses_enabled,
            ApiType::OpenAIChat => config.chat_enabled,
        };

        let providers = config
            .get_providers_for_api_type(&state.api_type)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();

        (
            gateway_enabled,
            config.cache_enabled,
            config.fallback_enabled,
            config.circuit_breaker_cooldown_seconds.max(1),
            providers,
        )
    };

    if !gateway_enabled {
        return (StatusCode::SERVICE_UNAVAILABLE, "Gateway is disabled").into_response();
    }

    let path = req.uri().path().to_string();
    let query = req.uri().query().map(|q| format!("?{}", q)).unwrap_or_default();
    let method = req.method().clone();
    let headers = req.headers().clone();
    let user_agent = headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(b) => b,
        Err(_) => return (StatusCode::BAD_REQUEST, "Failed to read body").into_response(),
    };

    // Cache check
    if cache_enabled {
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

    let input_tokens = calculate_input_tokens(&body_bytes);
    let api_type_str = api_type_to_string(&state.api_type);

    if providers.is_empty() {
        return (StatusCode::SERVICE_UNAVAILABLE, "No active providers for this API type").into_response();
    }

    let max_attempts = if fallback_enabled {
        providers.len().min(DEFAULT_MAX_ATTEMPTS).max(1)
    } else {
        1
    };

    // Deterministic tiebreak for this request.
    let mut candidates = providers;
    candidates.sort_by(|a, b| {
        let ca = state.circuits.get(&a.id).map(|c| c.clone()).unwrap_or_default();
        let cb = state.circuits.get(&b.id).map(|c| c.clone()).unwrap_or_default();
        let sa = ca.score(a.weight, now);
        let sb = cb.score(b.weight, now);

        let a_can = ca.can_attempt(now);
        let b_can = cb.can_attempt(now);

        // Prefer providers we can attempt now.
        match b_can.cmp(&a_can) {
            std::cmp::Ordering::Equal => {
                // Prefer sooner recovery for those in cooldown.
                let a_until = ca.open_until;
                let b_until = cb.open_until;
                match a_until.cmp(&b_until) {
                    std::cmp::Ordering::Equal => {
                        // Higher score first.
                        match sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal) {
                            std::cmp::Ordering::Equal => {
                                let ta = hash_u64(&(request_id.as_str(), a.id.as_str()));
                                let tb = hash_u64(&(request_id.as_str(), b.id.as_str()));
                                ta.cmp(&tb)
                            }
                            other => other,
                        }
                    }
                    other => other,
                }
            }
            other => other,
        }
    });

    let mut tried: HashSet<String> = HashSet::new();
    let mut attempted_any = false;

    for provider in candidates.into_iter().take(max_attempts) {
        if tried.contains(&provider.id) {
            continue;
        }
        tried.insert(provider.id.clone());

        let force = !attempted_any;
        if !reserve_provider_attempt(&state.circuits, &provider.id, now, force) {
            continue;
        }

        let Some(_permit) = try_acquire_provider_permit(&state.inflight_limits, &provider.id, MAX_INFLIGHT_PER_PROVIDER) else {
            // Busy provider; release probe flag by marking as failure with a tiny cooldown.
            mark_busy_failure(&state.circuits, &provider.id, now);
            continue;
        };

        attempted_any = true;
        let attempt_start = SystemTime::now();

        let _ = state.app.emit(
            "gateway://provider-status",
            ProviderStatusEvent {
                provider_id: provider.id.clone(),
                status: "pending".to_string(),
                api_type: api_type_str.clone(),
            },
        );

        // Claude Code proxy mode only for Anthropic /v1/messages.
        let is_messages_path = path.starts_with("/v1/messages");
        let use_proxy_conversion = provider.claude_code_proxy && state.api_type == ApiType::Anthropic && is_messages_path;
        let requested_model = extract_model(&body_bytes).unwrap_or_else(|| "unknown".to_string());

        let (request_body, target_path) = if use_proxy_conversion {
            match converter::anthropic_to_openai(&body_bytes, &provider.model_mapping) {
                Ok(converted) => (converted, "/v1/chat/completions".to_string()),
                Err(e) => {
                    // Bad client request; retrying other providers won't help.
                    let _ = state.app.emit(
                        "gateway://provider-status",
                        ProviderStatusEvent {
                            provider_id: provider.id.clone(),
                            status: "error".to_string(),
                            api_type: api_type_str.clone(),
                        },
                    );
                    mark_busy_failure(&state.circuits, &provider.id, now);
                    return (StatusCode::BAD_REQUEST, format!("Failed to convert request: {}", e)).into_response();
                }
            }
        } else {
            (body_bytes.to_vec(), path.clone())
        };

        let base = provider.base_url.trim_end_matches('/');
        let url = format!("{}{}{}", base, target_path, query);

        let mut new_req = state.http_client.request(method.clone(), &url);

        // Forward headers (exclude hop-by-hop and auth headers; gateway provides its own auth).
        for (key, value) in &headers {
            let key_str = key.as_str();
            if key_str == "host"
                || key_str == "content-length"
                || key_str == "authorization"
                || key_str == "x-api-key"
                || key_str == "anthropic-version"
                || key_str == "anthropic-beta"
            {
                continue;
            }
            new_req = new_req.header(key, value);
        }

        // Provider auth
        if !provider.api_key.is_empty() {
            if use_proxy_conversion {
                let auth_val = format!("Bearer {}", provider.api_key);
                if let Ok(val) = HeaderValue::from_str(&auth_val) {
                    new_req = new_req.header("Authorization", val);
                }
            } else {
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
        }

        new_req = new_req.header("Content-Type", "application/json");
        new_req = new_req.body(request_body);

        let resp = match timeout(UPSTREAM_HEADERS_TIMEOUT, new_req.send()).await {
            Ok(Ok(resp)) => resp,
            Ok(Err(e)) => {
                let duration = duration_ms(attempt_start);
                let (until, failure_kind) = open_circuit(
                    &state.circuits,
                    &provider.id,
                    now,
                    base_cooldown_seconds,
                    FailureKind::Connect,
                    None,
                    &(now, &provider.id, &request_id),
                );

                let _ = state.app.emit(
                    "gateway://provider-status",
                    ProviderStatusEvent {
                        provider_id: provider.id.clone(),
                        status: "error".to_string(),
                        api_type: api_type_str.clone(),
                    },
                );

                state.stats.record_request(RequestLog {
                    id: uuid::Uuid::new_v4().to_string(),
                    timestamp: now,
                    provider: provider.name.clone(),
                    model: requested_model.clone(),
                    status: 502,
                    duration_ms: duration,
                    input_tokens,
                    output_tokens: 0,
                    cost: 0.0,
                    path: path.clone(),
                    client_agent: user_agent.clone(),
                    api_type: api_type_str.clone(),
                    cached: false,
                    error_message: Some(format!("Connection failed: {}", e)),
                });
                state.stats.set_provider_cooldown(&provider.name, until, failure_kind);

                if !fallback_enabled {
                    return (StatusCode::BAD_GATEWAY, format!("Provider {} failed: {}", provider.name, e)).into_response();
                }
                continue;
            }
            Err(_) => {
                let duration = duration_ms(attempt_start);
                let (until, failure_kind) = open_circuit(
                    &state.circuits,
                    &provider.id,
                    now,
                    base_cooldown_seconds,
                    FailureKind::Timeout,
                    None,
                    &(now, &provider.id, &request_id),
                );

                let _ = state.app.emit(
                    "gateway://provider-status",
                    ProviderStatusEvent {
                        provider_id: provider.id.clone(),
                        status: "error".to_string(),
                        api_type: api_type_str.clone(),
                    },
                );

                state.stats.record_request(RequestLog {
                    id: uuid::Uuid::new_v4().to_string(),
                    timestamp: now,
                    provider: provider.name.clone(),
                    model: requested_model.clone(),
                    status: 504,
                    duration_ms: duration,
                    input_tokens,
                    output_tokens: 0,
                    cost: 0.0,
                    path: path.clone(),
                    client_agent: user_agent.clone(),
                    api_type: api_type_str.clone(),
                    cached: false,
                    error_message: Some("Upstream timeout".to_string()),
                });
                state.stats.set_provider_cooldown(&provider.name, until, failure_kind);

                if !fallback_enabled {
                    return (StatusCode::GATEWAY_TIMEOUT, "Upstream timeout").into_response();
                }
                continue;
            }
        };

        let status = resp.status();
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let is_stream = content_type.contains("text/event-stream") || content_type.contains("stream");

        // Classify failures.
        let should_fallback = status.is_server_error()
            || status == StatusCode::REQUEST_TIMEOUT
            || status == StatusCode::UNAUTHORIZED
            || status == StatusCode::PAYMENT_REQUIRED
            || status == StatusCode::FORBIDDEN
            || status == StatusCode::GONE
            || status == StatusCode::NOT_FOUND
            || status == StatusCode::TOO_MANY_REQUESTS;

        if !status.is_success() {
            let resp_headers = resp.headers().clone();
            let retry_after = parse_retry_after_seconds_from_headers(&resp_headers);

            let body = match timeout(UPSTREAM_BODY_TIMEOUT, resp.bytes()).await {
                Ok(Ok(bytes)) => bytes,
                _ => bytes::Bytes::new(),
            };
            let failure_kind = failure_kind_from_status(status);
            let (until, failure_kind) = open_circuit(
                &state.circuits,
                &provider.id,
                now,
                base_cooldown_seconds,
                failure_kind,
                retry_after,
                &(now, &provider.id, &request_id, status.as_u16()),
            );

            let error_body = truncate_utf8(body.as_ref(), 500);
            let duration = duration_ms(attempt_start);

            let _ = state.app.emit(
                "gateway://provider-status",
                ProviderStatusEvent {
                    provider_id: provider.id.clone(),
                    status: "error".to_string(),
                    api_type: api_type_str.clone(),
                },
            );

            state.stats.record_request(RequestLog {
                id: uuid::Uuid::new_v4().to_string(),
                timestamp: now,
                provider: provider.name.clone(),
                model: requested_model.clone(),
                status: status.as_u16(),
                duration_ms: duration,
                input_tokens,
                output_tokens: 0,
                cost: 0.0,
                path: path.clone(),
                client_agent: user_agent.clone(),
                api_type: api_type_str.clone(),
                cached: false,
                error_message: Some(format!("HTTP {} - {}", status, error_body)),
            });
            state.stats.set_provider_cooldown(&provider.name, until, failure_kind);

            if fallback_enabled && should_fallback {
                continue;
            }

            let mut builder = Response::builder().status(status);
            if let Some(headers_mut) = builder.headers_mut() {
                for (k, v) in resp_headers.iter() {
                    if k == axum::http::header::CONTENT_LENGTH {
                        continue;
                    }
                    headers_mut.insert(k, v.clone());
                }
            }
            return builder.body(Body::from(body)).unwrap_or_default();
        }

        // Success path.
        let duration = duration_ms(attempt_start);
        mark_success(&state.circuits, &provider.id, now, duration);
        state.stats.clear_provider_cooldown(&provider.name);

        let _ = state.app.emit(
            "gateway://provider-status",
            ProviderStatusEvent {
                provider_id: provider.id.clone(),
                status: "success".to_string(),
                api_type: api_type_str.clone(),
            },
        );

        let output_tokens = 0;
        let cost = calculate_cost(input_tokens, output_tokens, provider.input_price_per_1k, provider.output_price_per_1k);
        state.stats.record_request(RequestLog {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: now,
            provider: provider.name.clone(),
            model: requested_model.clone(),
            status: status.as_u16(),
            duration_ms: duration,
            input_tokens,
            output_tokens,
            cost,
            path: path.clone(),
            client_agent: user_agent.clone(),
            api_type: api_type_str.clone(),
            cached: false,
            error_message: None,
        });

        // Collect response headers for cache (exclude content-length as body may change).
        let response_headers: Vec<(String, String)> = resp
            .headers()
            .iter()
            .filter(|(k, _)| *k != &axum::http::header::CONTENT_LENGTH)
            .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_string())))
            .collect();

        let mut builder = Response::builder().status(status);
        if let Some(headers_mut) = builder.headers_mut() {
            for (k, v) in resp.headers() {
                if k == axum::http::header::CONTENT_LENGTH {
                    continue;
                }
                headers_mut.insert(k, v.clone());
            }
        }

        if !is_stream {
            let bytes = match timeout(UPSTREAM_BODY_TIMEOUT, resp.bytes()).await {
                Ok(Ok(bytes)) => bytes,
                _ => bytes::Bytes::new(),
            };

            let final_bytes = if use_proxy_conversion {
                match converter::openai_response_to_anthropic(&bytes, &requested_model) {
                    Ok(converted) => bytes::Bytes::from(converted),
                    Err(e) => {
                        let (until, failure_kind) = open_circuit(
                            &state.circuits,
                            &provider.id,
                            now,
                            base_cooldown_seconds,
                            FailureKind::Other,
                            None,
                            &(now, &provider.id, &request_id, "convert"),
                        );
                        state.stats.set_provider_cooldown(&provider.name, until, failure_kind);
                        if fallback_enabled {
                            continue;
                        }
                        return (StatusCode::BAD_GATEWAY, format!("Failed to convert upstream response: {}", e)).into_response();
                    }
                }
            } else {
                bytes
            };

            if cache_enabled {
                let cache_key = CacheManager::generate_key(&path, &body_bytes);
                state
                    .cache
                    .set(cache_key, final_bytes.to_vec(), status.as_u16(), response_headers);
            }

            // Ensure JSON content-type for converted responses.
            if use_proxy_conversion {
                if let Some(headers_mut) = builder.headers_mut() {
                    headers_mut.insert(
                        axum::http::header::CONTENT_TYPE,
                        HeaderValue::from_static("application/json"),
                    );
                }
            }

            return builder.body(Body::from(final_bytes)).unwrap_or_default();
        }

        // Stream response.
        if use_proxy_conversion {
            let message_id = format!(
                "msg_{}",
                uuid::Uuid::new_v4().to_string().replace("-", "")[..24].to_string()
            );

            let stream = resp.bytes_stream();
            let model_name = requested_model.clone();
            let converted_stream = async_stream::stream! {
                let mut buffer = String::new();
                let mut is_first = true;
                let mut stream_ended = false;

                tokio::pin!(stream);

                while let Some(chunk_result) = futures::StreamExt::next(&mut stream).await {
                    match chunk_result {
                        Ok(chunk) => {
                            buffer.push_str(&String::from_utf8_lossy(&chunk));

                            while let Some(pos) = buffer.find('\n') {
                                let line = buffer[..pos].to_string();
                                buffer = buffer[pos + 1..].to_string();

                                let line = line.trim();
                                if line.is_empty() {
                                    continue;
                                }

                                let converted_events = converter::openai_sse_to_anthropic(line, &message_id, &model_name, is_first);
                                if !converted_events.is_empty() && is_first {
                                    is_first = false;
                                }

                                for event in &converted_events {
                                    yield Ok::<_, std::io::Error>(bytes::Bytes::from(format!("{}\n\n", event)));
                                    if event.contains("message_stop") {
                                        stream_ended = true;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Stream error: {}", e);
                            break;
                        }
                    }
                }

                if !buffer.trim().is_empty() {
                    let converted_events = converter::openai_sse_to_anthropic(buffer.trim(), &message_id, &model_name, is_first);
                    for event in &converted_events {
                        yield Ok::<_, std::io::Error>(bytes::Bytes::from(format!("{}\n\n", event)));
                        if event.contains("message_stop") {
                            stream_ended = true;
                        }
                    }
                }

                if !stream_ended {
                    yield Ok::<_, std::io::Error>(bytes::Bytes::from(
                        "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
                    ));
                    yield Ok::<_, std::io::Error>(bytes::Bytes::from(
                        "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null},\"usage\":{\"output_tokens\":0}}\n\n",
                    ));
                    yield Ok::<_, std::io::Error>(bytes::Bytes::from(
                        "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
                    ));
                }
            };

            if let Some(headers_mut) = builder.headers_mut() {
                headers_mut.insert(
                    axum::http::header::CONTENT_TYPE,
                    HeaderValue::from_static("text/event-stream; charset=utf-8"),
                );
            }

            let body = Body::from_stream(converted_stream);
            return builder.body(body).unwrap_or_default();
        }

        let body = Body::from_stream(resp.bytes_stream());
        return builder.body(body).unwrap_or_default();
    }

    let overall_duration = duration_ms(overall_start);
    eprintln!(
        "❌ [Gateway:{}] All providers failed for {} (request_id={}, duration={}ms)",
        api_type_str,
        path,
        request_id,
        overall_duration
    );
    (StatusCode::BAD_GATEWAY, "All providers failed").into_response()
}

fn duration_ms(start: SystemTime) -> u64 {
    SystemTime::now()
        .duration_since(start)
        .unwrap_or_default()
        .as_millis() as u64
}

fn hash_u64<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn extract_model(body: &[u8]) -> Option<String> {
    let v = serde_json::from_slice::<serde_json::Value>(body).ok()?;
    v.get("model").and_then(|m| m.as_str()).map(|s| s.to_string())
}

fn truncate_utf8(bytes: &[u8], max_chars: usize) -> String {
    let s = String::from_utf8_lossy(bytes);
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let truncated: String = s.chars().take(max_chars).collect();
    format!("{}...(truncated)", truncated)
}

fn failure_kind_from_status(status: StatusCode) -> FailureKind {
    match status {
        StatusCode::TOO_MANY_REQUESTS => FailureKind::RateLimit,
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::PAYMENT_REQUIRED => FailureKind::Auth,
        StatusCode::NOT_FOUND => FailureKind::NotFound,
        s if s.is_server_error() => FailureKind::Upstream5xx,
        _ => FailureKind::Other,
    }
}

fn parse_retry_after_seconds_from_headers(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    let v = headers.get("retry-after")?.to_str().ok()?.trim();
    v.parse::<u64>().ok()
}

fn reserve_provider_attempt(
    circuits: &DashMap<String, Circuit>,
    provider_id: &str,
    now: u64,
    force: bool,
) -> bool {
    let mut entry = circuits.entry(provider_id.to_string()).or_default();
    if entry.can_attempt(now) {
        entry.mark_probe_started(now);
        return true;
    }

    if force {
        // Escape hatch: allow one forced attempt when everything is blocked.
        entry.open_until = now;
        entry.probe_in_flight = true;
        return true;
    }
    false
}

fn mark_busy_failure(circuits: &DashMap<String, Circuit>, provider_id: &str, now: u64) {
    if let Some(mut entry) = circuits.get_mut(provider_id) {
        // Release probe lock so another request can try.
        if entry.is_half_open(now) {
            entry.probe_in_flight = false;
        }
    }
}

fn try_acquire_provider_permit(
    inflight: &DashMap<String, Arc<Semaphore>>,
    provider_id: &str,
    max_inflight: usize,
) -> Option<OwnedSemaphorePermit> {
    let sem = inflight
        .entry(provider_id.to_string())
        .or_insert_with(|| Arc::new(Semaphore::new(max_inflight)))
        .clone();
    sem.try_acquire_owned().ok()
}

fn open_circuit(
    circuits: &DashMap<String, Circuit>,
    provider_id: &str,
    now: u64,
    base_cooldown_seconds: u64,
    kind: FailureKind,
    retry_after_seconds: Option<u64>,
    jitter_seed: &impl Hash,
) -> (u64, FailureKind) {
    let mut entry = circuits.entry(provider_id.to_string()).or_default();
    let until = entry.on_failure(
        now,
        base_cooldown_seconds,
        kind,
        retry_after_seconds,
        jitter_seed,
    );
    (until, kind)
}

fn mark_success(circuits: &DashMap<String, Circuit>, provider_id: &str, _now: u64, latency_ms: u64) {
    let mut entry = circuits.entry(provider_id.to_string()).or_default();
    entry.on_success(latency_ms);
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

export type ApiType = 'Anthropic' | 'OpenAIResponses' | 'OpenAIChat';

export interface Provider {
    id: string;
    name: string;
    base_url: string;
    api_key: string;
    model_mapping: Record<string, string>;
    enabled: boolean;
    api_types: ApiType[];
    weight: number;
    input_price_per_1k: number;
    output_price_per_1k: number;
}

export interface GatewayConfig {
    // 三个独立端口
    anthropic_port: number;
    responses_port: number;
    chat_port: number;

    // 三个独立开关
    anthropic_enabled: boolean;
    responses_enabled: boolean;
    chat_enabled: boolean;

    // 旧字段 (兼容)
    port: number;
    enabled: boolean;

    providers: Provider[];
    fallback_enabled: boolean;

    // 缓存配置
    cache_enabled: boolean;
    cache_ttl_seconds: number;
    cache_max_entries: number;

    // 熔断配置
    circuit_breaker_cooldown_seconds: number;
}

export interface RequestLog {
    id: string;
    timestamp: number;
    provider: string;
    model: string;
    status: number;
    duration_ms: number;
    input_tokens: number;
    output_tokens: number;
    cost: number;
    path: string;
    client_agent: string;
    api_type: string;
    cached: boolean;
}

export interface ProviderStats {
    provider_id: string;
    provider_name: string;

    // 请求统计
    total_requests: number;
    successful_requests: number;
    failed_requests: number;

    // 延迟统计 (毫秒)
    avg_latency_ms: number;
    min_latency_ms: number;
    max_latency_ms: number;
    p50_latency_ms: number;
    p95_latency_ms: number;
    p99_latency_ms: number;

    // Token 统计
    total_input_tokens: number;
    total_output_tokens: number;

    // 费用统计
    total_cost: number;

    // 健康状态
    last_success_at: number | null;
    last_failure_at: number | null;
    last_error_message: string | null;
    consecutive_failures: number;
    is_healthy: boolean;
}

export interface HourlyStat {
    timestamp: number;
    requests: number;
    input_tokens: number;
    output_tokens: number;
    cost: number;
}

export interface GatewayStats {
    // 全局统计
    total_requests: number;
    total_input_tokens: number;
    total_output_tokens: number;
    total_cost: number;
    cache_hits: number;
    cache_misses: number;

    // 按 API 类型统计
    anthropic_requests: number;
    responses_requests: number;
    chat_requests: number;

    // 每供应商统计
    provider_stats: Record<string, ProviderStats>;

    recent_requests: RequestLog[];
    hourly_activity: HourlyStat[];
}

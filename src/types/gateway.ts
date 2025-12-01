export interface Provider {
    id: string;
    name: string;
    base_url: string;
    api_key: string;
    model_mapping: Record<string, string>;
    enabled: boolean;
}

export interface GatewayConfig {
    port: number;
    enabled: boolean;
    providers: Provider[];
    fallback_enabled: boolean;
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
}

export interface HourlyStat {
    timestamp: number;
    requests: number;
    input_tokens: number;
    output_tokens: number;
    cost: number;
}

export interface GatewayStats {
    total_requests: number;
    total_input_tokens: number;
    total_output_tokens: number;
    total_cost: number;
    cache_hits: number;
    cache_misses: number;
    recent_requests: RequestLog[];
    hourly_activity: HourlyStat[];
}

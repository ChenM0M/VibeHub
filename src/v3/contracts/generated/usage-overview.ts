/* Generated from contracts/v3. Do not edit directly. */

export type OptionalCount = number | null;

export interface UsageOverview {
  schema_version: "1.0";
  model_version: string;
  project_path: string;
  scope: "project" | "task";
  task_id?: string | null;
  generated_at: string;
  freshness: "fresh" | "stale" | "unknown";
  completeness: "complete" | "partial" | "empty" | "unavailable";
  total_tokens: number;
  refresh: UsageRefreshSummary;
  audit: UsageAuditSummary;
  warnings: string[];
  [k: string]: unknown;
}
export interface UsageRefreshSummary {
  state: "loading" | "success" | "partial" | "failure" | "cancelled";
  duration_ms: number;
  performance_budget_ms: number;
  within_budget: boolean;
  cache_state: string;
}
export interface UsageAuditSummary {
  schema_version: "1.0";
  model_version: string;
  pricing_version: string;
  currency: string;
  captured_at: string;
  freshness: "fresh" | "stale" | "unknown";
  confidence: string;
  attribution: string;
  dedupe_strategy: string;
  token_state: "known" | "unknown" | "partial";
  cost: UsageCostSummary;
  excluded_records: number;
  ambiguous_records: number;
  legacy_records: number;
  unattributed_tokens: number;
  time_range: {
    from_ms: number | null;
    to_ms: number | null;
    [k: string]: unknown;
  };
  breakdowns: {
    kind: "provider" | "model" | "task" | "session" | "time";
    id: string;
    provider: string;
    model: string | null;
    records: number;
    total_tokens: OptionalCount;
    cost: number | null;
    status: string;
    [k: string]: unknown;
  }[];
  anomaly?: {
    [k: string]: unknown;
  } | null;
  evidence_provenance: {
    [k: string]: unknown;
  }[];
  [k: string]: unknown;
}
export interface UsageCostSummary {
  state: "complete" | "partial" | "unknown";
  known_cost: number | null;
  currency: string;
  pricing_version: string;
  priced_tokens: number;
  unpriced_tokens: number;
  missing_reasons: string[];
  repair_actions: string[];
}

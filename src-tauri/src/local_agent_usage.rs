use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    env,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Instant, UNIX_EPOCH},
};
use walkdir::WalkDir;

const RECENT_LIMIT: usize = 8;
const STALE_AFTER_SECONDS: i64 = 900;
const USAGE_SCHEMA_VERSION: &str = "1.0";
const USAGE_MODEL_VERSION: &str = "usage-v1";
const PRICING_VERSION: &str = "2026-07-28.official-list";
const REFRESH_BUDGET_MS: u64 = 2_000;
/// Maximum transcript file size we will read (10 MB). Larger files are skipped to
/// prevent memory exhaustion from malicious or corrupted files.
const MAX_TRANSCRIPT_FILE_SIZE: u64 = 10 * 1024 * 1024;
/// Maximum number of lines we will read from a single transcript file.
const MAX_TRANSCRIPT_LINES: usize = 100_000;
/// Maximum plausible token count for a single message. Counts above this are
/// treated as corrupted/malicious and fail closed.
const MAX_PLAUSIBLE_TOKENS_PER_MESSAGE: u64 = 1_000_000_000;

/// Cache entry for a provider scan result, keyed by file path + mtime.
#[derive(Debug, Clone)]
struct UsageCacheEntry {
    source: String,
    data_path: String,
    mtime_ms: i64,
    summary: AgentUsageSourceSummary,
}

/// Simple mtime-based cache for provider usage scans. Not persisted across restarts.
#[derive(Debug, Default)]
pub struct UsageCache {
    entries: HashMap<String, UsageCacheEntry>,
}

impl UsageCache {
    pub fn new() -> Self {
        Self::default()
    }

    fn cache_key(source: &str, data_path: &str) -> String {
        format!("{source}:{data_path}")
    }

    /// Returns cached summary if the file mtime has not changed.
    pub fn get(&self, source: &str, data_path: &str, current_mtime_ms: i64) -> Option<AgentUsageSourceSummary> {
        let key = Self::cache_key(source, data_path);
        self.entries.get(&key).and_then(|entry| {
            if entry.mtime_ms == current_mtime_ms {
                Some(entry.summary.clone())
            } else {
                None
            }
        })
    }

    pub fn put(&mut self, source: &str, data_path: &str, mtime_ms: i64, summary: AgentUsageSourceSummary) {
        let key = Self::cache_key(source, data_path);
        self.entries.insert(key, UsageCacheEntry {
            source: source.to_string(),
            data_path: data_path.to_string(),
            mtime_ms,
            summary,
        });
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn cache_state(&self) -> String {
        if self.entries.is_empty() {
            "cold".to_string()
        } else {
            format!("warm_{}_entries", self.entries.len())
        }
    }
}

/// Global shared cache instance.
pub fn shared_usage_cache() -> Arc<Mutex<UsageCache>> {
    use std::sync::OnceLock;
    static CACHE: OnceLock<Arc<Mutex<UsageCache>>> = OnceLock::new();
    CACHE.get_or_init(|| Arc::new(Mutex::new(UsageCache::new()))).clone()
}

/// Versioned pricing catalog entry. Prices are USD per 1M tokens.
#[derive(Debug, Clone)]
struct ModelPricing {
    input_per_1m: f64,
    output_per_1m: f64,
    cache_read_per_1m: Option<f64>,
    cache_write_per_1m: Option<f64>,
    reasoning_per_1m: Option<f64>,
}

impl ModelPricing {
    fn cost_for(&self, tokens: &TokenBreakdown) -> Option<f64> {
        let input_cost = tokens.input as f64 / 1_000_000.0 * self.input_per_1m;
        let output_cost = tokens.output as f64 / 1_000_000.0 * self.output_per_1m;
        let cache_read_cost = self
            .cache_read_per_1m
            .map(|p| tokens.cache_read as f64 / 1_000_000.0 * p)
            .unwrap_or(0.0);
        let cache_write_cost = self
            .cache_write_per_1m
            .map(|p| tokens.cache_write as f64 / 1_000_000.0 * p)
            .unwrap_or(0.0);
        let reasoning_cost = self
            .reasoning_per_1m
            .map(|p| tokens.reasoning as f64 / 1_000_000.0 * p)
            .unwrap_or(0.0);
        Some(input_cost + output_cost + cache_read_cost + cache_write_cost + reasoning_cost)
    }
}

/// Returns the canonical model id used for pricing lookups.
/// Normalizes aliases and strips common date suffixes.
fn canonical_model_id(raw: &str) -> String {
    let trimmed = raw.trim().to_ascii_lowercase();
    // Strip common date suffixes like -2024-08-06, -20250414, etc.
    let no_date = trimmed
        .split('-')
        .take_while(|part| {
            // Keep parts until we hit a 4+ digit sequence that looks like a year
            !(part.len() >= 4 && part.chars().all(|c| c.is_ascii_digit()))
        })
        .collect::<Vec<_>>()
        .join("-");
    // Normalize known aliases
    match no_date.as_str() {
        "gpt4o" | "gpt-4o" => "gpt-4o".to_string(),
        "gpt4o-mini" | "gpt-4o-mini" => "gpt-4o-mini".to_string(),
        "gpt4-turbo" | "gpt-4-turbo" => "gpt-4-turbo".to_string(),
        "gpt4" | "gpt-4" => "gpt-4".to_string(),
        "gpt35-turbo" | "gpt-3.5-turbo" => "gpt-3.5-turbo".to_string(),
        "claude-opus" | "claude-3-opus" | "claude-3-opus-20240229" => "claude-3-opus".to_string(),
        "claude-sonnet" | "claude-3-sonnet" | "claude-3-sonnet-20240229" => {
            "claude-3-sonnet".to_string()
        }
        "claude-haiku" | "claude-3-haiku" | "claude-3-haiku-20240307" => {
            "claude-3-haiku".to_string()
        }
        "claude-3-5-sonnet" | "claude-3.5-sonnet" => "claude-3-5-sonnet".to_string(),
        "claude-3-5-haiku" | "claude-3.5-haiku" => "claude-3-5-haiku".to_string(),
        "o1" | "o1-preview" => "o1-preview".to_string(),
        "o1-mini" => "o1-mini".to_string(),
        "o3-mini" => "o3-mini".to_string(),
        _ => no_date,
    }
}

/// Versioned pricing catalog. Returns None for unknown models.
fn pricing_catalog(model: &str) -> Option<ModelPricing> {
    let canonical = canonical_model_id(model);
    match canonical.as_str() {
        "gpt-4o" => Some(ModelPricing {
            input_per_1m: 2.50,
            output_per_1m: 10.00,
            cache_read_per_1m: Some(1.25),
            cache_write_per_1m: Some(2.50),
            reasoning_per_1m: None,
        }),
        "gpt-4o-mini" => Some(ModelPricing {
            input_per_1m: 0.15,
            output_per_1m: 0.60,
            cache_read_per_1m: Some(0.075),
            cache_write_per_1m: Some(0.15),
            reasoning_per_1m: None,
        }),
        "gpt-4-turbo" => Some(ModelPricing {
            input_per_1m: 10.00,
            output_per_1m: 30.00,
            cache_read_per_1m: None,
            cache_write_per_1m: None,
            reasoning_per_1m: None,
        }),
        "gpt-4" => Some(ModelPricing {
            input_per_1m: 30.00,
            output_per_1m: 60.00,
            cache_read_per_1m: None,
            cache_write_per_1m: None,
            reasoning_per_1m: None,
        }),
        "gpt-3.5-turbo" => Some(ModelPricing {
            input_per_1m: 0.50,
            output_per_1m: 1.50,
            cache_read_per_1m: None,
            cache_write_per_1m: None,
            reasoning_per_1m: None,
        }),
        "claude-3-opus" => Some(ModelPricing {
            input_per_1m: 15.00,
            output_per_1m: 75.00,
            cache_read_per_1m: Some(1.50),
            cache_write_per_1m: Some(18.75),
            reasoning_per_1m: None,
        }),
        "claude-3-sonnet" => Some(ModelPricing {
            input_per_1m: 3.00,
            output_per_1m: 15.00,
            cache_read_per_1m: Some(0.30),
            cache_write_per_1m: Some(3.75),
            reasoning_per_1m: None,
        }),
        "claude-3-haiku" => Some(ModelPricing {
            input_per_1m: 0.25,
            output_per_1m: 1.25,
            cache_read_per_1m: Some(0.03),
            cache_write_per_1m: Some(0.30),
            reasoning_per_1m: None,
        }),
        "claude-3-5-sonnet" => Some(ModelPricing {
            input_per_1m: 3.00,
            output_per_1m: 15.00,
            cache_read_per_1m: Some(0.30),
            cache_write_per_1m: Some(3.75),
            reasoning_per_1m: None,
        }),
        "claude-3-5-haiku" => Some(ModelPricing {
            input_per_1m: 0.80,
            output_per_1m: 4.00,
            cache_read_per_1m: Some(0.08),
            cache_write_per_1m: Some(1.00),
            reasoning_per_1m: None,
        }),
        "o1-preview" => Some(ModelPricing {
            input_per_1m: 15.00,
            output_per_1m: 60.00,
            cache_read_per_1m: Some(7.50),
            cache_write_per_1m: None,
            reasoning_per_1m: Some(60.00),
        }),
        "o1-mini" => Some(ModelPricing {
            input_per_1m: 3.00,
            output_per_1m: 12.00,
            cache_read_per_1m: Some(1.50),
            cache_write_per_1m: None,
            reasoning_per_1m: Some(12.00),
        }),
        "o3-mini" => Some(ModelPricing {
            input_per_1m: 1.10,
            output_per_1m: 4.40,
            cache_read_per_1m: Some(0.55),
            cache_write_per_1m: None,
            reasoning_per_1m: Some(4.40),
        }),
        _ => None,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalAgentUsageOverview {
    pub schema_version: String,
    pub model_version: String,
    pub project_path: String,
    pub scope: String,
    pub task_id: Option<String>,
    pub requested_session_count: usize,
    pub matched_session_count: usize,
    pub generated_at: String,
    pub freshness: String,
    pub stale_after_seconds: i64,
    pub completeness: String,
    pub refresh: UsageRefreshSummary,
    pub audit: UsageAuditSummary,
    pub primary_metric: AgentUsagePrimaryMetric,
    pub non_cached_total_tokens: u64,
    pub total_tokens: u64,
    pub source_count: usize,
    pub claude_code: AgentUsageSourceSummary,
    pub claude_app: AgentUsageSourceSummary,
    pub codex: AgentUsageSourceSummary,
    pub opencode: AgentUsageSourceSummary,
    pub cursor: AgentUsageSourceSummary,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageRefreshSummary {
    pub state: String,
    pub duration_ms: u64,
    pub performance_budget_ms: u64,
    pub within_budget: bool,
    pub cache_state: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageAuditSummary {
    pub schema_version: String,
    pub model_version: String,
    pub pricing_version: String,
    pub currency: String,
    pub captured_at: String,
    pub freshness: String,
    pub confidence: String,
    pub attribution: String,
    pub dedupe_strategy: String,
    pub token_state: String,
    pub cost: UsageCostSummary,
    pub excluded_records: u64,
    pub ambiguous_records: u64,
    pub legacy_records: u64,
    pub unattributed_tokens: u64,
    pub time_range: UsageTimeRange,
    pub breakdowns: Vec<UsageBreakdown>,
    pub anomaly: Option<UsageAnomaly>,
    pub evidence_provenance: Vec<UsageEvidenceProvenance>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageTimeRange {
    pub from_ms: Option<i64>,
    pub to_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageBreakdown {
    pub kind: String,
    pub id: String,
    pub provider: String,
    pub model: Option<String>,
    pub records: usize,
    pub total_tokens: Option<u64>,
    pub cost: Option<f64>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageCostSummary {
    pub state: String,
    pub known_cost: Option<f64>,
    pub currency: String,
    pub pricing_version: String,
    pub priced_tokens: u64,
    pub unpriced_tokens: u64,
    pub missing_reasons: Vec<String>,
    pub repair_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageAnomaly {
    pub code: String,
    pub observed_tokens: u64,
    pub explanation: String,
    pub repair_action: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageEvidenceProvenance {
    pub provider: String,
    pub source_kind: String,
    pub locator: Option<String>,
    pub records: usize,
    pub freshness: String,
    pub confidence: String,
}

/// Versioned domain record used by provider adapters before aggregation. All
/// optional measurements remain `None` when the provider did not report them;
/// callers must never turn unknown values into numeric zero.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct UsageRecord {
    schema_version: &'static str,
    provider: String,
    account_principal: Option<String>,
    project_id: Option<String>,
    task_id: Option<String>,
    session_id: Option<String>,
    source_record_id: String,
    model: Option<String>,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_read_tokens: Option<u64>,
    cache_write_tokens: Option<u64>,
    reasoning_tokens: Option<u64>,
    total_tokens: Option<u64>,
    cost: Option<f64>,
    currency: Option<String>,
    pricing_version: Option<String>,
    captured_at_ms: Option<i64>,
    freshness: String,
    confidence: String,
    dedupe_key: String,
    evidence_provenance: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentUsagePrimaryMetric {
    pub kind: String,
    pub label: String,
    pub value: Option<f64>,
    pub currency: Option<String>,
    pub tokens: Option<u64>,
    pub source: String,
    pub confidence: String,
    pub estimated: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentUsageSourceSummary {
    pub source: String,
    pub status: String,
    pub freshness: String,
    pub available: bool,
    pub data_path: Option<String>,
    pub records: usize,
    pub non_cached_total_tokens: u64,
    pub total_tokens: u64,
    pub cost: Option<f64>,
    pub tokens: TokenBreakdown,
    pub latest_updated_at_ms: Option<i64>,
    pub recent: Vec<AgentUsageRecentItem>,
    pub warnings: Vec<String>,
    /// Provider records are deduplicated for token totals, but their IDs are
    /// retained internally so a shared provider session can still match more
    /// than one V3 workflow session.
    #[serde(skip)]
    matched_provider_session_ids: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct TokenBreakdown {
    pub input: u64,
    pub output: u64,
    pub reasoning: u64,
    pub cached_input: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentUsageRecentItem {
    pub id: String,
    pub title: String,
    pub model: Option<String>,
    pub models: Vec<String>,
    pub agent: Option<String>,
    pub message_count: u64,
    pub tool_uses: u64,
    pub started_at_ms: Option<i64>,
    pub duration_seconds: Option<u64>,
    pub status: String,
    pub non_cached_total_tokens: u64,
    pub total_tokens: u64,
    pub cost: Option<f64>,
    pub updated_at_ms: Option<i64>,
}

impl TokenBreakdown {
    fn non_cached_total(&self) -> u64 {
        self.total
            .saturating_sub(self.cached_input)
            .saturating_sub(self.cache_read)
            .saturating_sub(self.cache_write)
    }
}

#[derive(Debug)]
struct CodexThreadRow {
    id: String,
    rollout_path: String,
    model_provider: Option<String>,
    model: Option<String>,
    tokens_used: u64,
    updated_at_ms: Option<i64>,
}

#[derive(Debug)]
struct OpenCodeSessionRow {
    id: String,
    agent: Option<String>,
    model: Option<String>,
    cost: f64,
    tokens_input: u64,
    tokens_output: u64,
    tokens_reasoning: u64,
    tokens_cache_read: u64,
    tokens_cache_write: u64,
    time_updated: Option<i64>,
}

#[derive(Debug)]
struct MatchedRows<T> {
    exact: Vec<T>,
    aliases: BTreeMap<String, Vec<T>>,
}

impl<T> Default for MatchedRows<T> {
    fn default() -> Self {
        Self {
            exact: Vec::new(),
            aliases: BTreeMap::new(),
        }
    }
}

/// Maps each V3 workflow session to the provider session identifiers that are
/// safe to use for local usage attribution. The outer key is kept separate
/// from provider ids so multiple workflow sessions can share one provider
/// session without double-counting the provider record.
pub type TaskSessionProviderLinks = BTreeMap<String, BTreeMap<String, BTreeSet<String>>>;

#[derive(Debug, Clone)]
struct UsageSessionFilter {
    task_id: String,
    session_links: TaskSessionProviderLinks,
}

impl UsageSessionFilter {
    fn matches_provider(&self, provider: &str, provider_session_id: &str) -> bool {
        self.provider_session_ids(provider)
            .contains(provider_session_id)
    }

    fn provider_session_ids(&self, provider: &str) -> BTreeSet<String> {
        self.session_links
            .iter()
            .flat_map(|(workflow_id, links)| {
                self.provider_session_ids_for_workflow(provider, workflow_id, links)
            })
            .collect()
    }

    fn provider_session_ids_for_workflow(
        &self,
        provider: &str,
        workflow_id: &str,
        links: &BTreeMap<String, BTreeSet<String>>,
    ) -> BTreeSet<String> {
        let provider_aliases = provider_aliases(provider);
        let mut ids = BTreeSet::new();
        let mut has_explicit_link = false;
        for key in &provider_aliases {
            if let Some(provider_ids) = links.get(key) {
                has_explicit_link |= !provider_ids.is_empty();
                ids.extend(
                    provider_ids
                        .iter()
                        .filter_map(|id| normalize_provider_session_id(provider, id)),
                );
            }
        }
        if let Some(provider_ids) = links.get("*") {
            has_explicit_link |= !provider_ids.is_empty();
            ids.extend(
                provider_ids
                    .iter()
                    .filter_map(|id| normalize_provider_session_id(provider, id)),
            );
        }
        if has_explicit_link {
            return ids;
        }

        for prefix in self.provider_prefixes(provider) {
            if let Some(provider_id) = workflow_id.strip_prefix(&prefix) {
                if !provider_id.is_empty() {
                    ids.insert(provider_id.to_string());
                }
                return ids;
            }
        }
        if !workflow_id.starts_with("session.") {
            ids.insert(workflow_id.to_string());
        }
        ids
    }

    fn matched_workflow_session_count(
        &self,
        sources: &[(&str, &AgentUsageSourceSummary)],
    ) -> usize {
        self.session_links
            .iter()
            .filter(|(workflow_id, links)| {
                sources.iter().any(|(provider, source)| {
                    self.provider_session_ids_for_workflow(provider, workflow_id, links)
                        .iter()
                        .any(|provider_id| {
                            source.matched_provider_session_ids.contains(provider_id)
                        })
                })
            })
            .count()
    }

    fn provider_prefixes(&self, provider: &str) -> Vec<String> {
        let providers: &[&str] = match provider {
            "claude" => &["claude", "claude_code"],
            "codex" => &["codex"],
            "opencode" => &["opencode"],
            _ => &[provider],
        };
        providers
            .iter()
            .map(|provider| format!("session.{provider}."))
            .collect()
    }
}

fn provider_aliases(provider: &str) -> Vec<String> {
    match provider {
        "claude" | "claude_code" => vec!["claude".to_string(), "claude_code".to_string()],
        "codex" => vec!["codex".to_string()],
        "opencode" => vec!["opencode".to_string()],
        other => vec![other.to_string()],
    }
}

fn normalize_provider_session_id(provider: &str, value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    for alias in provider_aliases(provider) {
        let prefix = format!("session.{alias}.");
        if let Some(id) = value.strip_prefix(&prefix).filter(|id| !id.is_empty()) {
            return Some(id.to_string());
        }
    }
    Some(value.to_string())
}

pub fn read_local_agent_usage(project_path: impl AsRef<Path>) -> Result<LocalAgentUsageOverview> {
    read_local_agent_usage_scoped(project_path.as_ref(), None)
}

pub fn read_local_agent_usage_with_cache(
    project_path: impl AsRef<Path>,
    cache: Arc<Mutex<UsageCache>>,
) -> Result<LocalAgentUsageOverview> {
    read_local_agent_usage_scoped_with_cache(project_path.as_ref(), None, Some(cache))
}

pub fn read_local_agent_usage_for_task(
    project_path: impl AsRef<Path>,
    task_id: String,
    session_links: TaskSessionProviderLinks,
) -> Result<LocalAgentUsageOverview> {
    let filter = UsageSessionFilter {
        task_id,
        session_links,
    };
    read_local_agent_usage_scoped(project_path.as_ref(), Some(&filter))
}

pub fn read_local_agent_usage_for_task_with_cache(
    project_path: impl AsRef<Path>,
    task_id: String,
    session_links: TaskSessionProviderLinks,
    cache: Arc<Mutex<UsageCache>>,
) -> Result<LocalAgentUsageOverview> {
    let filter = UsageSessionFilter {
        task_id,
        session_links,
    };
    read_local_agent_usage_scoped_with_cache(project_path.as_ref(), Some(&filter), Some(cache))
}

fn read_local_agent_usage_scoped(
    project_path: &Path,
    session_filter: Option<&UsageSessionFilter>,
) -> Result<LocalAgentUsageOverview> {
    read_local_agent_usage_scoped_with_cache(project_path, session_filter, None)
}

fn read_local_agent_usage_scoped_with_cache(
    project_path: &Path,
    session_filter: Option<&UsageSessionFilter>,
    cache: Option<Arc<Mutex<UsageCache>>>,
) -> Result<LocalAgentUsageOverview> {
    let started_at = Instant::now();
    let project_path = ProjectPathMatcher::new(project_path.as_ref());
    let generated_at = Utc::now();
    let generated_at_ms = generated_at.timestamp_millis();
    let mut warnings = Vec::new();
    let mut claude_code = read_claude_code_usage_scoped(&project_path, session_filter);
    let claude_app = unsupported_source(
        "claude_app",
        "Claude App ordinary chat has no stable, documented, project-attributable local usage contract; internal Electron/SQLite/IndexedDB data is not read.",
    );
    let mut codex = read_codex_usage_scoped(&project_path, session_filter);
    let mut opencode = read_opencode_usage_scoped(&project_path, session_filter);
    let cursor = unsupported_source(
        "cursor",
        "Cursor has no stable, documented, project-attributable local usage contract; internal application databases and caches are not read.",
    );

    for source in [&mut claude_code, &mut codex, &mut opencode] {
        apply_source_freshness(source, generated_at_ms, STALE_AFTER_SECONDS);
    }

    for (label, source) in [
        ("Claude Code", &claude_code),
        ("Claude App ordinary chat", &claude_app),
        ("Codex", &codex),
        ("OpenCode", &opencode),
        ("Cursor", &cursor),
    ] {
        if !source.available {
            warnings.push(format!(
                "{label} local usage source status: {}.",
                source.status
            ));
        }
        warnings.extend(source.warnings.iter().cloned());
    }

    let non_cached_total_tokens = claude_code
        .non_cached_total_tokens
        .saturating_add(codex.non_cached_total_tokens)
        .saturating_add(opencode.non_cached_total_tokens);
    let total_tokens = claude_code
        .total_tokens
        .saturating_add(codex.total_tokens)
        .saturating_add(opencode.total_tokens);
    let source_count = [claude_code.available, codex.available, opencode.available]
        .into_iter()
        .filter(|available| *available)
        .count();
    let primary_metric = select_primary_metric(&claude_code, &codex, &opencode, total_tokens);
    let sources = [&claude_code, &codex, &opencode];
    let freshness = overall_freshness(&sources);
    let completeness = overall_completeness(&sources, total_tokens);
    let provider_record_count = sources.iter().map(|source| source.records).sum();
    let matched_session_count = session_filter.map_or(provider_record_count, |filter| {
        filter.matched_workflow_session_count(&[
            ("claude", &claude_code),
            ("codex", &codex),
            ("opencode", &opencode),
        ])
    });
    let (scope, task_id, requested_session_count) = match session_filter {
        Some(filter) => {
            if filter.session_links.is_empty() {
                warnings.push(format!(
                    "Task {} has no V3 sessions; project-wide usage is intentionally excluded.",
                    filter.task_id
                ));
            } else if matched_session_count == 0 {
                warnings.push(format!(
                    "No local usage record matched the {} V3 session(s) linked to Task {}; project-wide usage is intentionally excluded.",
                    filter.session_links.len(),
                    filter.task_id
                ));
            } else if matched_session_count < filter.session_links.len() {
                warnings.push(format!(
                    "Only {} of {} V3-linked sessions have local usage records for Task {}.",
                    matched_session_count,
                    filter.session_links.len(),
                    filter.task_id
                ));
            }
            (
                "task".to_string(),
                Some(filter.task_id.clone()),
                filter.session_links.len(),
            )
        }
        None => ("project".to_string(), None, 0),
    };

    let duration_ms = u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
    let audit = build_usage_audit(
        &generated_at.to_rfc3339(),
        &freshness,
        &completeness,
        total_tokens,
        [&claude_code, &codex, &opencode],
        requested_session_count,
        matched_session_count,
    );
    let cache_state = cache
        .as_ref()
        .and_then(|c| c.lock().ok().map(|guard| guard.cache_state()))
        .unwrap_or_else(|| "disabled".to_string());
    Ok(LocalAgentUsageOverview {
        schema_version: USAGE_SCHEMA_VERSION.to_string(),
        model_version: USAGE_MODEL_VERSION.to_string(),
        scope,
        task_id,
        requested_session_count,
        matched_session_count,
        non_cached_total_tokens,
        total_tokens,
        source_count,
        primary_metric,
        project_path: project_path.display_path.clone(),
        generated_at: generated_at.to_rfc3339(),
        freshness,
        stale_after_seconds: STALE_AFTER_SECONDS,
        completeness,
        refresh: UsageRefreshSummary {
            state: if audit.cost.state == "partial" || !warnings.is_empty() {
                "partial"
            } else {
                "success"
            }
            .to_string(),
            duration_ms,
            performance_budget_ms: REFRESH_BUDGET_MS,
            within_budget: duration_ms <= REFRESH_BUDGET_MS,
            cache_state,
        },
        audit,
        claude_code,
        claude_app,
        codex,
        opencode,
        cursor,
        warnings,
    })
}

fn build_usage_audit(
    captured_at: &str,
    freshness: &str,
    completeness: &str,
    total_tokens: u64,
    sources: [&AgentUsageSourceSummary; 3],
    requested_session_count: usize,
    matched_session_count: usize,
) -> UsageAuditSummary {
    let available = sources
        .iter()
        .filter(|source| source.available)
        .collect::<Vec<_>>();
    let known_cost = available
        .iter()
        .filter_map(|source| source.cost)
        .sum::<f64>();
    let priced_tokens = available
        .iter()
        .filter(|source| source.cost.is_some())
        .map(|source| source.total_tokens)
        .fold(0_u64, u64::saturating_add);
    let unpriced_tokens = total_tokens.saturating_sub(priced_tokens);
    let cost_state = if total_tokens == 0 {
        "unknown"
    } else if unpriced_tokens == 0 {
        "complete"
    } else if priced_tokens > 0 {
        "partial"
    } else {
        "unknown"
    };
    let missing_reasons = available
        .iter()
        .filter(|source| source.cost.is_none() && source.total_tokens > 0)
        .map(|source| format!("{}.model_or_price_unknown", source.source))
        .collect::<Vec<_>>();
    let repair_actions = if unpriced_tokens > 0 {
        vec!["Refresh after the provider reports a stable model id, or update the versioned pricing catalog with evidence.".to_string()]
    } else {
        Vec::new()
    };
    let anomaly = (total_tokens >= 1_000_000_000).then(|| UsageAnomaly {
        code: "usage.total.extreme".to_string(),
        observed_tokens: total_tokens,
        explanation: "The total exceeds one billion tokens; inspect provider/session decomposition and excluded or ambiguous records before relying on the number.".to_string(),
        repair_action: "Drill into provider and session rows, verify source_record_id/dedupe_key, then rebuild from read-only evidence.".to_string(),
    });
    let mut breakdowns = Vec::new();
    let mut from_ms = None;
    let mut to_ms = None;
    for source in &available {
        breakdowns.push(UsageBreakdown {
            kind: "provider".to_string(),
            id: source.source.clone(),
            provider: source.source.clone(),
            model: None,
            records: source.records,
            total_tokens: Some(source.total_tokens),
            cost: source.cost,
            status: source.status.clone(),
        });
        for session in &source.recent {
            from_ms = min_opt_i64(from_ms, session.started_at_ms.or(session.updated_at_ms));
            to_ms = max_opt_i64(to_ms, session.updated_at_ms);
            breakdowns.push(UsageBreakdown {
                kind: "session".to_string(),
                id: session.id.clone(),
                provider: source.source.clone(),
                model: session.model.clone(),
                records: 1,
                total_tokens: Some(session.total_tokens),
                cost: session.cost,
                status: session.status.clone(),
            });
        }
    }
    UsageAuditSummary {
        schema_version: USAGE_SCHEMA_VERSION.to_string(),
        model_version: USAGE_MODEL_VERSION.to_string(),
        pricing_version: PRICING_VERSION.to_string(),
        currency: "USD".to_string(),
        captured_at: captured_at.to_string(),
        freshness: freshness.to_string(),
        confidence: if completeness == "complete" {
            "hard_observed"
        } else {
            "partial"
        }
        .to_string(),
        attribution: if requested_session_count > 0 {
            "explicit_provider_session_link"
        } else {
            "unique_canonical_project_path"
        }
        .to_string(),
        dedupe_strategy:
            "provider+source_record_id; live Codex rollout supersedes SQLite catalog snapshot"
                .to_string(),
        token_state: if total_tokens == 0 && available.is_empty() {
            "unknown"
        } else {
            "known"
        }
        .to_string(),
        cost: UsageCostSummary {
            state: cost_state.to_string(),
            known_cost: (priced_tokens > 0).then_some(known_cost),
            currency: "USD".to_string(),
            pricing_version: PRICING_VERSION.to_string(),
            priced_tokens,
            unpriced_tokens,
            missing_reasons,
            repair_actions,
        },
        excluded_records: requested_session_count.saturating_sub(matched_session_count) as u64,
        ambiguous_records: sources
            .iter()
            .flat_map(|source| &source.warnings)
            .filter(|warning| warning.to_ascii_lowercase().contains("ambiguous"))
            .count() as u64,
        legacy_records: 0,
        unattributed_tokens: 0,
        time_range: UsageTimeRange { from_ms, to_ms },
        breakdowns,
        anomaly,
        evidence_provenance: sources
            .iter()
            .map(|source| UsageEvidenceProvenance {
                provider: source.source.clone(),
                source_kind: if source.source == "claude_code" {
                    "transcript"
                } else {
                    "sqlite_or_rollout"
                }
                .to_string(),
                locator: source.data_path.clone(),
                records: source.records,
                freshness: source.freshness.clone(),
                confidence: if source.available {
                    "hard_observed"
                } else {
                    "unavailable"
                }
                .to_string(),
            })
            .collect(),
    }
}

fn apply_source_freshness(
    source: &mut AgentUsageSourceSummary,
    generated_at_ms: i64,
    stale_after_seconds: i64,
) {
    if !source.available {
        source.freshness = "unknown".to_string();
        return;
    }
    let Some(observed_at_ms) = source.latest_updated_at_ms else {
        source.freshness = "unknown".to_string();
        source.warnings.push(format!(
            "{} freshness is unknown because no observed-through timestamp is available.",
            source.source
        ));
        if source.status == "available" {
            source.status = "partial".to_string();
        }
        return;
    };
    let age_ms = generated_at_ms.saturating_sub(observed_at_ms);
    if age_ms > stale_after_seconds.saturating_mul(1000) {
        source.freshness = "stale".to_string();
        source.status = "stale".to_string();
        source.warnings.push(format!(
            "{} usage is stale: latest observation is {} seconds old (threshold {} seconds).",
            source.source,
            age_ms / 1000,
            stale_after_seconds
        ));
    } else {
        source.freshness = "fresh".to_string();
    }
}

fn overall_freshness(sources: &[&AgentUsageSourceSummary]) -> String {
    let available = sources
        .iter()
        .filter(|source| source.available)
        .collect::<Vec<_>>();
    if available.is_empty() {
        "unknown"
    } else if available.iter().any(|source| source.freshness == "stale") {
        "stale"
    } else if available.iter().any(|source| source.freshness == "unknown") {
        "unknown"
    } else {
        "fresh"
    }
    .to_string()
}

fn overall_completeness(sources: &[&AgentUsageSourceSummary], total_tokens: u64) -> String {
    if sources.iter().all(|source| source.status == "empty") {
        return "empty".to_string();
    }
    if total_tokens == 0 && sources.iter().all(|source| !source.available) {
        return "unavailable".to_string();
    }
    if sources.iter().any(|source| {
        matches!(
            source.status.as_str(),
            "partial" | "stale" | "permission_denied" | "error"
        )
    }) {
        return "partial".to_string();
    }
    "complete".to_string()
}

fn select_primary_metric(
    claude_code: &AgentUsageSourceSummary,
    codex: &AgentUsageSourceSummary,
    opencode: &AgentUsageSourceSummary,
    total_tokens: u64,
) -> AgentUsagePrimaryMetric {
    if total_tokens > 0 {
        let available = [claude_code, codex, opencode]
            .into_iter()
            .filter(|source| source.available)
            .map(|source| source.source.as_str())
            .collect::<Vec<_>>();
        let source = if available.len() == 1 {
            available[0]
        } else {
            "local_agents"
        };
        return AgentUsagePrimaryMetric {
            kind: "tokens".to_string(),
            label: "Total local tokens".to_string(),
            value: None,
            currency: None,
            tokens: Some(total_tokens),
            source: source.to_string(),
            confidence: "local_recorded".to_string(),
            estimated: false,
            detail: "Primary usage is local recorded token usage including cache. Claude Code is deduplicated by request/message identity; Codex uses local rollout totals; OpenCode adds its recorded token breakdown. Cost remains unavailable because subscription usage is not an actual bill.".to_string(),
        };
    }

    AgentUsagePrimaryMetric {
        kind: "unavailable".to_string(),
        label: "No token usage available".to_string(),
        value: None,
        currency: None,
        tokens: None,
        source: "none".to_string(),
        confidence: "unavailable".to_string(),
        estimated: false,
        detail: "No local Codex or OpenCode token records matched this project.".to_string(),
    }
}

#[derive(Debug, Clone)]
struct ProjectPathMatcher {
    display_path: String,
    primary: String,
    canonical: String,
    basename: Option<String>,
}

impl ProjectPathMatcher {
    fn new(path: &Path) -> Self {
        let primary = normalize_path_string(path);
        let canonical = path
            .canonicalize()
            .ok()
            .map(|path| normalize_path_string(&path))
            .unwrap_or_else(|| primary.clone());
        let basename = path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .map(|name| name.to_ascii_lowercase());

        Self {
            display_path: primary.clone(),
            primary,
            canonical,
            basename,
        }
    }

    fn matches(&self, value: &str) -> bool {
        let value = normalize_stored_path(value);
        value.eq_ignore_ascii_case(&self.primary) || value.eq_ignore_ascii_case(&self.canonical)
    }

    fn alias_root(&self, value: &str) -> Option<String> {
        let basename = self.basename.as_deref()?;
        let value = normalize_stored_path(value);
        let parts = value.split('/').collect::<Vec<_>>();
        for index in (0..parts.len()).rev() {
            if parts[index].eq_ignore_ascii_case(basename) {
                return Some(parts[..=index].join("/"));
            }
        }
        None
    }
}

#[derive(Debug, Default)]
struct ClaudeSessionAccumulator {
    id: String,
    models: BTreeSet<String>,
    tokens: TokenBreakdown,
    message_count: u64,
    tool_uses: u64,
    started_at_ms: Option<i64>,
    updated_at_ms: Option<i64>,
    seen_messages: BTreeSet<String>,
    warnings: Vec<String>,
}

fn read_claude_code_usage_scoped(
    project_path: &ProjectPathMatcher,
    session_filter: Option<&UsageSessionFilter>,
) -> AgentUsageSourceSummary {
    let config_dir = env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".claude")));
    let Some(config_dir) = config_dir else {
        return empty_source(
            "claude_code",
            "Claude config directory could not be resolved.",
        );
    };
    let encoded = encode_claude_project_path(&project_path.primary);
    let sessions_dir = config_dir.join("projects").join(encoded);
    read_claude_code_usage_from_dir_scoped(project_path, &sessions_dir, session_filter)
}

fn read_claude_code_usage_from_dir(
    project_path: &ProjectPathMatcher,
    sessions_dir: &Path,
) -> AgentUsageSourceSummary {
    read_claude_code_usage_from_dir_scoped(project_path, sessions_dir, None)
}

fn read_claude_code_usage_from_dir_scoped(
    _project_path: &ProjectPathMatcher,
    sessions_dir: &Path,
    session_filter: Option<&UsageSessionFilter>,
) -> AgentUsageSourceSummary {
    if !sessions_dir.exists() {
        let mut summary = empty_source(
            "claude_code",
            "Claude Code project session directory was not found.",
        );
        summary.data_path = Some(sessions_dir.display().to_string());
        return summary;
    }
    let entries = match std::fs::read_dir(sessions_dir) {
        Ok(entries) => entries,
        Err(error) => {
            return error_source(
                "claude_code",
                Some(sessions_dir.to_path_buf()),
                error.into(),
            )
        }
    };
    let mut sessions = Vec::new();
    let mut source_warnings = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                source_warnings.push(format!(
                    "Failed to read Claude Code directory entry: {error}"
                ));
                continue;
            }
        };
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
            continue;
        }
        match read_claude_session(&path) {
            Ok(session)
                if session_filter.map_or(true, |filter| {
                    filter.matches_provider("claude", &session.id)
                }) =>
            {
                sessions.push(session)
            }
            Err(error) => source_warnings.push(format!("{}: {error:#}", path.display())),
            Ok(_) => {}
        }
    }
    sessions.sort_by(|a, b| {
        b.updated_at_ms
            .cmp(&a.updated_at_ms)
            .then_with(|| b.id.cmp(&a.id))
    });
    let mut tokens = TokenBreakdown::default();
    let mut recent = Vec::new();
    let mut latest_updated_at_ms = None;
    let mut partial = false;
    let mut total_cost: Option<f64> = None;
    for session in &sessions {
        tokens.input = tokens.input.saturating_add(session.tokens.input);
        tokens.output = tokens.output.saturating_add(session.tokens.output);
        tokens.cache_read = tokens.cache_read.saturating_add(session.tokens.cache_read);
        tokens.cache_write = tokens
            .cache_write
            .saturating_add(session.tokens.cache_write);
        tokens.total = tokens.total.saturating_add(session.tokens.total);
        latest_updated_at_ms = max_opt_i64(latest_updated_at_ms, session.updated_at_ms);
        if !session.warnings.is_empty() {
            partial = true;
            source_warnings.extend(
                session
                    .warnings
                    .iter()
                    .map(|warning| format!("{}: {warning}", session.id)),
            );
        }
        // Compute cost from pricing catalog when model is known
        let session_cost = session
            .models
            .last()
            .and_then(|model| pricing_catalog(model))
            .and_then(|pricing| pricing.cost_for(&session.tokens));
        if let Some(cost) = session_cost {
            total_cost = Some(total_cost.map_or(cost, |acc| acc + cost));
        }
        if recent.len() < RECENT_LIMIT {
            let models = session.models.iter().cloned().collect::<Vec<_>>();
            let duration_seconds = match (session.started_at_ms, session.updated_at_ms) {
                (Some(start), Some(end)) if end >= start => Some(((end - start) / 1000) as u64),
                _ => None,
            };
            recent.push(AgentUsageRecentItem {
                id: session.id.clone(),
                title: "Claude Code session".to_string(),
                model: models.last().cloned(),
                models,
                agent: Some("Claude Code".to_string()),
                message_count: session.message_count,
                tool_uses: session.tool_uses,
                started_at_ms: session.started_at_ms,
                duration_seconds,
                status: if session.warnings.is_empty() {
                    "available"
                } else {
                    "partial"
                }
                .to_string(),
                non_cached_total_tokens: session.tokens.non_cached_total(),
                total_tokens: session.tokens.total,
                cost: session_cost,
                updated_at_ms: session.updated_at_ms,
            });
        }
    }
    let available = !sessions.is_empty();
    AgentUsageSourceSummary {
        source: "claude_code".to_string(),
        status: if !available {
            "empty"
        } else if partial || !source_warnings.is_empty() {
            "partial"
        } else {
            "available"
        }
        .to_string(),
        available,
        freshness: "unknown".to_string(),
        data_path: Some(sessions_dir.display().to_string()),
        records: sessions.len(),
        non_cached_total_tokens: tokens.non_cached_total(),
        total_tokens: tokens.total,
        cost: total_cost,
        tokens,
        latest_updated_at_ms,
        recent,
        warnings: source_warnings,
        matched_provider_session_ids: sessions
            .iter()
            .filter_map(|session| normalize_provider_session_id("claude", &session.id))
            .collect(),
    }
}

fn read_claude_session(path: &Path) -> Result<ClaudeSessionAccumulator> {
    // Fail closed on oversized files to prevent memory exhaustion
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("Failed to stat Claude Code transcript {}", path.display()))?;
    if metadata.len() > MAX_TRANSCRIPT_FILE_SIZE {
        anyhow::bail!(
            "Claude Code transcript {} exceeds maximum size {} bytes",
            path.display(),
            MAX_TRANSCRIPT_FILE_SIZE
        );
    }
    let file = File::open(path)
        .with_context(|| format!("Failed to open Claude Code transcript {}", path.display()))?;
    let fallback_id = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("unknown-session");
    let mut session = ClaudeSessionAccumulator {
        id: fallback_id.to_string(),
        ..Default::default()
    };
    for (index, line) in BufReader::new(file).lines().enumerate() {
        // Fail closed on too many lines to prevent DoS
        if index >= MAX_TRANSCRIPT_LINES {
            session
                .warnings
                .push(format!("transcript exceeds maximum line count {MAX_TRANSCRIPT_LINES}; truncated"));
            break;
        }
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                session
                    .warnings
                    .push(format!("line {} read error: {error}", index + 1));
                continue;
            }
        };
        let value: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(_) => {
                session
                    .warnings
                    .push(format!("line {} contains invalid JSON", index + 1));
                continue;
            }
        };
        if let Some(id) = value
            .get("sessionId")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
        {
            session.id = id.to_string();
        }
        let timestamp = value.get("timestamp").and_then(parse_timestamp_ms);
        session.started_at_ms = min_opt_i64(session.started_at_ms, timestamp);
        session.updated_at_ms = max_opt_i64(session.updated_at_ms, timestamp);
        match value.get("type").and_then(Value::as_str) {
            Some("user") => {
                if lineage_is_ambiguous(&value) {
                    session.warnings.push(format!(
                        "line {} has branch/resume/compaction/subagent lineage metadata",
                        index + 1
                    ));
                }
            }
            Some("assistant") => parse_claude_assistant(&value, index + 1, &mut session),
            Some("summary") | Some("system") if lineage_is_ambiguous(&value) => {
                session.warnings.push(format!(
                    "line {} indicates compaction or lineage that cannot be fully attributed",
                    index + 1
                ));
            }
            _ => {}
        }
    }
    Ok(session)
}

fn parse_claude_assistant(
    value: &Value,
    line_number: usize,
    session: &mut ClaudeSessionAccumulator,
) {
    let Some(message) = value.get("message") else {
        return;
    };
    let identity = message
        .get("id")
        .and_then(Value::as_str)
        .or_else(|| value.get("requestId").and_then(Value::as_str))
        .or_else(|| value.get("uuid").and_then(Value::as_str));
    let Some(identity) = identity.filter(|id| !id.is_empty()) else {
        if message.get("usage").is_some() {
            session.warnings.push(format!("line {line_number} has usage without stable message/request identity and was not counted"));
        }
        return;
    };
    if !session.seen_messages.insert(identity.to_string()) {
        return;
    }
    if let Some(model) = message
        .get("model")
        .and_then(Value::as_str)
        .filter(|model| !model.is_empty())
    {
        session.models.insert(model.to_string());
    }
    session.message_count = session.message_count.saturating_add(1);
    session.tool_uses = session.tool_uses.saturating_add(
        message
            .get("content")
            .and_then(Value::as_array)
            .map(|content| {
                content
                    .iter()
                    .filter(|block| block.get("type").and_then(Value::as_str) == Some("tool_use"))
                    .count() as u64
            })
            .unwrap_or_default(),
    );
    let Some(usage) = message.get("usage") else {
        session
            .warnings
            .push(format!("line {line_number} assistant message has no usage"));
        return;
    };
    let input = json_u64(usage, "input_tokens");
    let output = json_u64(usage, "output_tokens");
    let cache_read = json_u64(usage, "cache_read_input_tokens");
    let cache_creation = json_u64(usage, "cache_creation_input_tokens");
    // Fail closed on implausible token counts (corrupted or malicious data)
    if input > MAX_PLAUSIBLE_TOKENS_PER_MESSAGE
        || output > MAX_PLAUSIBLE_TOKENS_PER_MESSAGE
        || cache_read > MAX_PLAUSIBLE_TOKENS_PER_MESSAGE
        || cache_creation > MAX_PLAUSIBLE_TOKENS_PER_MESSAGE
    {
        session.warnings.push(format!(
            "line {line_number} has implausible token counts (in={input}, out={output}, cache_read={cache_read}, cache_write={cache_creation}); skipped"
        ));
        return;
    }
    session.tokens.input = session.tokens.input.saturating_add(input);
    session.tokens.output = session.tokens.output.saturating_add(output);
    session.tokens.cache_read = session.tokens.cache_read.saturating_add(cache_read);
    session.tokens.cache_write = session.tokens.cache_write.saturating_add(cache_creation);
    session.tokens.total = session.tokens.total.saturating_add(
        input
            .saturating_add(output)
            .saturating_add(cache_read)
            .saturating_add(cache_creation),
    );
}

fn encode_claude_project_path(path: &str) -> String {
    let normalized = normalize_stored_path(path);
    normalized
        .chars()
        .map(|character| {
            if character == '/' || character == '\\' || character == ':' {
                '-'
            } else {
                character
            }
        })
        .collect()
}

fn parse_timestamp_ms(value: &Value) -> Option<i64> {
    value.as_i64().or_else(|| {
        value.as_str().and_then(|text| {
            chrono::DateTime::parse_from_rfc3339(text)
                .ok()
                .map(|time| time.timestamp_millis())
        })
    })
}

fn min_opt_i64(current: Option<i64>, candidate: Option<i64>) -> Option<i64> {
    match (current, candidate) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn lineage_is_ambiguous(value: &Value) -> bool {
    value
        .get("isSidechain")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || value
            .get("isCompactSummary")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        || value
            .get("parentUuid")
            .is_some_and(|parent| !parent.is_null())
        || value.get("agentId").is_some()
}

fn read_codex_usage_scoped(
    project_path: &ProjectPathMatcher,
    session_filter: Option<&UsageSessionFilter>,
) -> AgentUsageSourceSummary {
    let mut warnings = Vec::new();
    let db_paths = existing_paths(codex_state_db_candidates());
    if db_paths.is_empty() && session_filter.is_none() {
        return empty_source(
            "codex",
            "Codex state database was not found in CODEX_SQLITE_HOME, CODEX_HOME, or ~/.codex.",
        );
    };

    let result =
        read_codex_usage_from_dbs_scoped(project_path, &db_paths, &mut warnings, session_filter);
    match result {
        Ok(mut summary) => {
            summary.warnings.append(&mut warnings);
            summary
        }
        Err(error) => {
            let mut summary = error_source("codex", db_paths.first().cloned(), error);
            summary.warnings = warnings;
            summary
        }
    }
}

fn read_codex_usage_from_db(
    project_path: &str,
    db_path: &Path,
    warnings: &mut Vec<String>,
) -> Result<AgentUsageSourceSummary> {
    let matcher = ProjectPathMatcher::new(Path::new(project_path));
    read_codex_usage_from_dbs_scoped(&matcher, &[db_path.to_path_buf()], warnings, None)
}

fn read_codex_usage_from_dbs(
    project_path: &ProjectPathMatcher,
    db_paths: &[PathBuf],
    warnings: &mut Vec<String>,
) -> Result<AgentUsageSourceSummary> {
    read_codex_usage_from_dbs_scoped(project_path, db_paths, warnings, None)
}

fn read_codex_usage_from_dbs_scoped(
    project_path: &ProjectPathMatcher,
    db_paths: &[PathBuf],
    warnings: &mut Vec<String>,
    session_filter: Option<&UsageSessionFilter>,
) -> Result<AgentUsageSourceSummary> {
    let mut best_exact_match: Option<(PathBuf, Vec<CodexThreadRow>)> = None;
    let mut alias_matches = Vec::new();
    let mut errors = Vec::new();

    for db_path in db_paths {
        match read_codex_rows_from_db_scoped(project_path, db_path, session_filter) {
            Ok(db_rows) => {
                if should_replace_codex_match(
                    best_exact_match.as_ref().map(|(_, rows)| rows),
                    &db_rows.exact,
                ) {
                    best_exact_match = Some((db_path.clone(), db_rows.exact));
                }
                for (alias_root, rows) in db_rows.aliases {
                    alias_matches.push((db_path.clone(), alias_root, rows));
                }
            }
            Err(error) => errors.push(format!("{}: {error:#}", db_path.display())),
        }
    }

    if session_filter.is_none()
        && !db_paths.is_empty()
        && best_exact_match.is_none()
        && alias_matches.is_empty()
        && errors.len() == db_paths.len()
    {
        anyhow::bail!("{}", errors.join("; "));
    }

    warnings.extend(errors);
    let (data_path, mut alias_root, mut rows) =
        choose_codex_rows(project_path, best_exact_match, alias_matches, warnings);
    let mut direct_rollout_paths = Vec::new();
    if let Some(filter) = session_filter {
        let (direct_rows, matched_paths) =
            read_codex_task_rollout_rows(project_path, filter, warnings);
        if !direct_rows.is_empty() {
            let mut rows_by_id = rows
                .into_iter()
                .map(|row| (row.id.clone(), row))
                .collect::<BTreeMap<_, _>>();
            for row in direct_rows {
                // The rollout is the live source for an active Codex thread and
                // intentionally replaces a possibly stale SQLite catalog row.
                rows_by_id.insert(row.id.clone(), row);
            }
            rows = rows_by_id.into_values().collect();
            alias_root = None;
            direct_rollout_paths = matched_paths;
        }
    }
    rows.sort_by(|a, b| {
        b.updated_at_ms
            .cmp(&a.updated_at_ms)
            .then_with(|| b.id.cmp(&a.id))
    });

    let mut tokens = TokenBreakdown::default();
    let mut recent = Vec::new();
    let mut latest_updated_at_ms = None;

    for row in rows.iter() {
        latest_updated_at_ms = max_opt_i64(latest_updated_at_ms, row.updated_at_ms);
        let usage = read_codex_rollout_usage(&row.rollout_path, warnings);
        let row_total_tokens = usage
            .as_ref()
            .map(|usage| usage.total)
            .unwrap_or(row.tokens_used);
        tokens.total = tokens.total.saturating_add(row_total_tokens);
        let non_cached_total_tokens = usage
            .as_ref()
            .map(TokenBreakdown::non_cached_total)
            .unwrap_or(row.tokens_used);
        if let Some(usage) = usage {
            tokens.input = tokens.input.saturating_add(usage.input);
            tokens.output = tokens.output.saturating_add(usage.output);
            tokens.reasoning = tokens.reasoning.saturating_add(usage.reasoning);
            tokens.cached_input = tokens.cached_input.saturating_add(usage.cached_input);
        }
        if recent.len() < RECENT_LIMIT {
            recent.push(AgentUsageRecentItem {
                id: row.id.clone(),
                title: "Codex session".to_string(),
                model: row.model.clone().or_else(|| row.model_provider.clone()),
                models: row
                    .model
                    .clone()
                    .or_else(|| row.model_provider.clone())
                    .into_iter()
                    .collect(),
                agent: Some("Codex".to_string()),
                message_count: 0,
                tool_uses: 0,
                started_at_ms: None,
                duration_seconds: None,
                status: "available".to_string(),
                non_cached_total_tokens,
                total_tokens: row_total_tokens,
                cost: None,
                updated_at_ms: row.updated_at_ms,
            });
        }
    }

    if rows.is_empty() {
        warnings.push(if let Some(filter) = session_filter {
            format!(
                "No Codex rollout or database record matched a Codex session linked to Task {}.",
                filter.task_id
            )
        } else {
            format!(
                "No Codex records matched {} or its child paths.",
                project_path.display_path
            )
        });
    }

    let summary_data_path = if !direct_rollout_paths.is_empty() {
        Some(
            direct_rollout_paths
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", "),
        )
    } else if rows.is_empty() {
        let candidates = db_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        (!candidates.is_empty()).then_some(candidates)
    } else {
        (!data_path.as_os_str().is_empty()).then(|| data_path.display().to_string())
    };

    Ok(AgentUsageSourceSummary {
        source: "codex".to_string(),
        status: if rows.is_empty() {
            "empty"
        } else {
            "available"
        }
        .to_string(),
        available: !rows.is_empty(),
        freshness: "unknown".to_string(),
        data_path: summary_data_path,
        records: rows.len(),
        non_cached_total_tokens: tokens.non_cached_total(),
        total_tokens: tokens.total,
        cost: None,
        tokens,
        latest_updated_at_ms,
        recent,
        warnings: alias_root
            .map(|path| {
                vec![format!(
                    "No Codex records matched configured path {}; using same-name usage path {}.",
                    project_path.display_path, path
                )]
            })
            .unwrap_or_default(),
        matched_provider_session_ids: rows
            .iter()
            .map(|row| row.id.clone())
            .filter_map(|id| normalize_provider_session_id("codex", &id))
            .collect(),
    })
}

fn read_codex_rows_from_db_scoped(
    project_path: &ProjectPathMatcher,
    db_path: &Path,
    session_filter: Option<&UsageSessionFilter>,
) -> Result<MatchedRows<CodexThreadRow>> {
    let conn = open_readonly(db_path)?;
    let mut stmt = conn.prepare(
        "select id, rollout_path, model_provider, model, tokens_used, updated_at_ms, cwd \
         from threads order by updated_at_ms desc, updated_at desc, id desc",
    )?;
    let mut rows = stmt.query([])?;
    let mut out = MatchedRows::default();
    while let Some(row) = rows.next()? {
        let cwd: String = row.get(6)?;
        let usage_row = CodexThreadRow {
            id: row.get(0)?,
            rollout_path: row.get(1)?,
            model_provider: row.get(2).ok(),
            model: row.get(3).ok(),
            tokens_used: signed_to_u64(row.get::<_, i64>(4).unwrap_or_default()),
            updated_at_ms: row.get(5).ok(),
        };
        if project_path.matches(&cwd)
            && session_filter.map_or(true, |filter| {
                filter.matches_provider("codex", &usage_row.id)
            })
        {
            out.exact.push(usage_row);
        } else if let Some(alias_root) = project_path.alias_root(&cwd) {
            if session_filter.map_or(true, |filter| {
                filter.matches_provider("codex", &usage_row.id)
            }) {
                out.aliases.entry(alias_root).or_default().push(usage_row);
            }
        }
    }
    Ok(out)
}

fn choose_codex_rows(
    project_path: &ProjectPathMatcher,
    exact_match: Option<(PathBuf, Vec<CodexThreadRow>)>,
    alias_matches: Vec<(PathBuf, String, Vec<CodexThreadRow>)>,
    warnings: &mut Vec<String>,
) -> (PathBuf, Option<String>, Vec<CodexThreadRow>) {
    if let Some((path, rows)) = exact_match.filter(|(_, rows)| !rows.is_empty()) {
        return (path, None, rows);
    }

    let alias_roots = alias_matches
        .iter()
        .filter(|(_, _, rows)| !rows.is_empty())
        .map(|(_, alias_root, _)| alias_root.clone())
        .collect::<BTreeSet<_>>();

    if !alias_roots.is_empty() {
        warnings.push(format!(
            "Non-canonical or same-name Codex usage paths matched {}; refusing ambiguous fallback: {}.",
            project_path.display_path,
            alias_roots.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }

    (PathBuf::new(), None, Vec::new())
}

fn should_replace_codex_match(
    current: Option<&Vec<CodexThreadRow>>,
    candidate: &[CodexThreadRow],
) -> bool {
    let Some(current) = current else {
        return true;
    };
    candidate.len() > current.len()
        || (candidate.len() == current.len()
            && max_codex_updated_at(candidate) > max_codex_updated_at(current))
}

fn max_codex_updated_at(rows: &[CodexThreadRow]) -> Option<i64> {
    rows.iter().filter_map(|row| row.updated_at_ms).max()
}

fn read_opencode_usage_scoped(
    project_path: &ProjectPathMatcher,
    session_filter: Option<&UsageSessionFilter>,
) -> AgentUsageSourceSummary {
    let mut warnings = Vec::new();
    let db_paths = existing_paths(opencode_db_candidates());
    if db_paths.is_empty() {
        return empty_source(
            "opencode",
            "OpenCode database was not found in XDG data, ~/.local/share/opencode, Application Support, or AppData candidates.",
        );
    };

    let result =
        read_opencode_usage_from_dbs_scoped(project_path, &db_paths, &mut warnings, session_filter);
    match result {
        Ok(mut summary) => {
            summary.warnings.append(&mut warnings);
            summary
        }
        Err(error) => {
            let mut summary = error_source("opencode", db_paths.first().cloned(), error);
            summary.warnings = warnings;
            summary
        }
    }
}

fn read_opencode_usage_from_db(
    project_path: &str,
    db_path: &Path,
) -> Result<AgentUsageSourceSummary> {
    let matcher = ProjectPathMatcher::new(Path::new(project_path));
    let mut warnings = Vec::new();
    let mut summary = read_opencode_usage_from_dbs_scoped(
        &matcher,
        &[db_path.to_path_buf()],
        &mut warnings,
        None,
    )?;
    summary.warnings.append(&mut warnings);
    Ok(summary)
}

fn read_opencode_usage_from_dbs_scoped(
    project_path: &ProjectPathMatcher,
    db_paths: &[PathBuf],
    warnings: &mut Vec<String>,
    session_filter: Option<&UsageSessionFilter>,
) -> Result<AgentUsageSourceSummary> {
    let mut best_exact_match: Option<(PathBuf, Vec<OpenCodeSessionRow>)> = None;
    let mut alias_matches = Vec::new();
    let mut errors = Vec::new();

    for db_path in db_paths {
        match read_opencode_rows_from_db_scoped(project_path, db_path, session_filter) {
            Ok(db_rows) => {
                if should_replace_opencode_match(
                    best_exact_match.as_ref().map(|(_, rows)| rows),
                    &db_rows.exact,
                ) {
                    best_exact_match = Some((db_path.clone(), db_rows.exact));
                }
                for (alias_root, rows) in db_rows.aliases {
                    alias_matches.push((db_path.clone(), alias_root, rows));
                }
            }
            Err(error) => errors.push(format!("{}: {error:#}", db_path.display())),
        }
    }

    if best_exact_match.is_none() && alias_matches.is_empty() && errors.len() == db_paths.len() {
        anyhow::bail!("{}", errors.join("; "));
    }

    warnings.extend(errors);
    let (data_path, alias_root, mut rows) =
        choose_opencode_rows(project_path, best_exact_match, alias_matches, warnings);
    rows.sort_by(|a, b| {
        b.time_updated
            .cmp(&a.time_updated)
            .then_with(|| b.id.cmp(&a.id))
    });

    let mut tokens = TokenBreakdown::default();
    let mut cost = 0.0_f64;
    let mut has_db_cost = false;
    let mut recent = Vec::new();
    let mut latest_updated_at_ms = None;

    for row in rows.iter() {
        let total = row
            .tokens_input
            .saturating_add(row.tokens_output)
            .saturating_add(row.tokens_reasoning)
            .saturating_add(row.tokens_cache_read)
            .saturating_add(row.tokens_cache_write);
        let non_cached_total_tokens = total
            .saturating_sub(row.tokens_cache_read)
            .saturating_sub(row.tokens_cache_write);
        tokens.input = tokens.input.saturating_add(row.tokens_input);
        tokens.output = tokens.output.saturating_add(row.tokens_output);
        tokens.reasoning = tokens.reasoning.saturating_add(row.tokens_reasoning);
        tokens.cache_read = tokens.cache_read.saturating_add(row.tokens_cache_read);
        tokens.cache_write = tokens.cache_write.saturating_add(row.tokens_cache_write);
        tokens.total = tokens.total.saturating_add(total);
        // Use DB cost when available and non-zero; otherwise try pricing catalog
        let item_cost = if row.cost > 0.0 {
            has_db_cost = true;
            row.cost
        } else {
            row.model
                .as_deref()
                .and_then(|raw| {
                    let model = format_opencode_model(raw);
                    pricing_catalog(&model)
                })
                .and_then(|pricing| pricing.cost_for(&TokenBreakdown {
                    input: row.tokens_input,
                    output: row.tokens_output,
                    reasoning: row.tokens_reasoning,
                    cached_input: 0,
                    cache_read: row.tokens_cache_read,
                    cache_write: row.tokens_cache_write,
                    total,
                }))
                .unwrap_or(0.0)
        };
        cost += item_cost;
        latest_updated_at_ms = max_opt_i64(latest_updated_at_ms, row.time_updated);
        if recent.len() < RECENT_LIMIT {
            recent.push(AgentUsageRecentItem {
                id: row.id.clone(),
                title: "OpenCode session".to_string(),
                model: row.model.as_deref().map(format_opencode_model),
                models: row
                    .model
                    .as_deref()
                    .map(format_opencode_model)
                    .into_iter()
                    .collect(),
                agent: row.agent.clone(),
                message_count: 0,
                tool_uses: 0,
                started_at_ms: None,
                duration_seconds: None,
                status: "available".to_string(),
                non_cached_total_tokens,
                total_tokens: total,
                cost: if item_cost > 0.0 { Some(item_cost) } else { None },
                updated_at_ms: row.time_updated,
            });
        }
    }

    if rows.is_empty() {
        warnings.push(format!(
            "No OpenCode records matched {} or its child paths.",
            project_path.display_path
        ));
    }

    // If we have no DB cost and no pricing-catalog matches, cost is unknown
    let final_cost = if has_db_cost || cost > 0.0 {
        Some(cost)
    } else {
        None
    };

    Ok(AgentUsageSourceSummary {
        source: "opencode".to_string(),
        status: if rows.is_empty() { "empty" } else { "available" }.to_string(),
        available: !rows.is_empty(),
        freshness: "unknown".to_string(),
        data_path: Some(if rows.is_empty() {
            db_paths
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            data_path.display().to_string()
        }),
        records: rows.len(),
        non_cached_total_tokens: tokens.non_cached_total(),
        total_tokens: tokens.total,
        cost: final_cost,
        tokens,
        latest_updated_at_ms,
        recent,
        warnings: alias_root
            .map(|path| {
                vec![format!(
                    "No OpenCode records matched configured path {}; using same-name usage path {}.",
                    project_path.display_path, path
                )]
            })
            .unwrap_or_default(),
        matched_provider_session_ids: rows
            .iter()
            .map(|row| row.id.clone())
            .filter_map(|id| normalize_provider_session_id("opencode", &id))
            .collect(),
    })
}

fn read_opencode_rows_from_db_scoped(
    project_path: &ProjectPathMatcher,
    db_path: &Path,
    session_filter: Option<&UsageSessionFilter>,
) -> Result<MatchedRows<OpenCodeSessionRow>> {
    let conn = open_readonly(db_path)?;
    let mut stmt = conn.prepare(
        "select s.id, s.agent, s.model, s.cost, s.tokens_input, s.tokens_output, \
                s.tokens_reasoning, s.tokens_cache_read, s.tokens_cache_write, s.time_updated, \
                p.worktree, s.directory, s.path \
         from session s \
         left join project p on p.id = s.project_id \
         order by s.time_updated desc, s.id desc",
    )?;
    let mut rows = stmt.query([])?;
    let mut out = MatchedRows::default();
    while let Some(row) = rows.next()? {
        let worktree: Option<String> = row.get(10).ok();
        let directory: Option<String> = row.get(11).ok();
        let path: Option<String> = row.get(12).ok();
        let paths = [worktree.as_deref(), directory.as_deref(), path.as_deref()];
        let matches = paths
            .into_iter()
            .flatten()
            .any(|path| project_path.matches(path));
        let usage_row = OpenCodeSessionRow {
            id: row.get(0)?,
            agent: row.get(1).ok(),
            model: row.get(2).ok(),
            cost: row.get::<_, f64>(3).unwrap_or_default(),
            tokens_input: signed_to_u64(row.get::<_, i64>(4).unwrap_or_default()),
            tokens_output: signed_to_u64(row.get::<_, i64>(5).unwrap_or_default()),
            tokens_reasoning: signed_to_u64(row.get::<_, i64>(6).unwrap_or_default()),
            tokens_cache_read: signed_to_u64(row.get::<_, i64>(7).unwrap_or_default()),
            tokens_cache_write: signed_to_u64(row.get::<_, i64>(8).unwrap_or_default()),
            time_updated: row.get(9).ok(),
        };
        if matches
            && session_filter.map_or(true, |filter| {
                filter.matches_provider("opencode", &usage_row.id)
            })
        {
            out.exact.push(usage_row);
        } else if let Some(alias_root) = paths
            .into_iter()
            .flatten()
            .find_map(|path| project_path.alias_root(path))
        {
            if session_filter.map_or(true, |filter| {
                filter.matches_provider("opencode", &usage_row.id)
            }) {
                out.aliases.entry(alias_root).or_default().push(usage_row);
            }
        }
    }
    Ok(out)
}

fn choose_opencode_rows(
    project_path: &ProjectPathMatcher,
    exact_match: Option<(PathBuf, Vec<OpenCodeSessionRow>)>,
    alias_matches: Vec<(PathBuf, String, Vec<OpenCodeSessionRow>)>,
    warnings: &mut Vec<String>,
) -> (PathBuf, Option<String>, Vec<OpenCodeSessionRow>) {
    if let Some((path, rows)) = exact_match.filter(|(_, rows)| !rows.is_empty()) {
        return (path, None, rows);
    }

    let alias_roots = alias_matches
        .iter()
        .filter(|(_, _, rows)| !rows.is_empty())
        .map(|(_, alias_root, _)| alias_root.clone())
        .collect::<BTreeSet<_>>();

    if !alias_roots.is_empty() {
        warnings.push(format!(
            "Non-canonical or same-name OpenCode usage paths matched {}; refusing ambiguous fallback: {}.",
            project_path.display_path,
            alias_roots.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }

    (PathBuf::new(), None, Vec::new())
}

fn should_replace_opencode_match(
    current: Option<&Vec<OpenCodeSessionRow>>,
    candidate: &[OpenCodeSessionRow],
) -> bool {
    let Some(current) = current else {
        return true;
    };
    candidate.len() > current.len()
        || (candidate.len() == current.len()
            && max_opencode_updated_at(candidate) > max_opencode_updated_at(current))
}

fn max_opencode_updated_at(rows: &[OpenCodeSessionRow]) -> Option<i64> {
    rows.iter().filter_map(|row| row.time_updated).max()
}

#[derive(Debug)]
struct CodexRolloutSnapshot {
    session_id: String,
    cwd: String,
    model: Option<String>,
    usage: Option<TokenBreakdown>,
}

fn read_codex_task_rollout_rows(
    project_path: &ProjectPathMatcher,
    session_filter: &UsageSessionFilter,
    warnings: &mut Vec<String>,
) -> (Vec<CodexThreadRow>, Vec<PathBuf>) {
    let roots = codex_rollout_root_candidates()
        .into_iter()
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    read_codex_task_rollout_rows_from_roots(project_path, session_filter, &roots, warnings)
}

fn read_codex_task_rollout_rows_from_roots(
    project_path: &ProjectPathMatcher,
    session_filter: &UsageSessionFilter,
    roots: &[PathBuf],
    warnings: &mut Vec<String>,
) -> (Vec<CodexThreadRow>, Vec<PathBuf>) {
    let requested_ids = session_filter.provider_session_ids("codex");
    if requested_ids.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let mut rows_by_id = BTreeMap::<String, CodexThreadRow>::new();
    for root in roots {
        for entry in WalkDir::new(root)
            .follow_links(false)
            .max_depth(4)
            .into_iter()
        {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    if warnings.len() < 8 {
                        warnings.push(format!(
                            "Failed to scan Codex rollout directory {}: {error}",
                            root.display()
                        ));
                    }
                    continue;
                }
            };
            if !entry.file_type().is_file() {
                continue;
            }
            let Some(file_name) = entry.file_name().to_str() else {
                continue;
            };
            let Some(expected_id) = requested_ids
                .iter()
                .find(|session_id| codex_rollout_file_matches(file_name, session_id))
            else {
                continue;
            };
            let path = entry.path();
            let snapshot = match read_codex_rollout_snapshot(path) {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    if warnings.len() < 8 {
                        warnings.push(format!(
                            "Failed to read Task-linked Codex rollout {}: {error:#}",
                            path.display()
                        ));
                    }
                    continue;
                }
            };
            if snapshot.session_id != *expected_id {
                if warnings.len() < 8 {
                    warnings.push(format!(
                        "Codex rollout {} metadata session id {} did not match requested session {}.",
                        path.display(),
                        snapshot.session_id,
                        expected_id
                    ));
                }
                continue;
            }
            if !project_path.matches(&snapshot.cwd) {
                if warnings.len() < 8 {
                    warnings.push(format!(
                        "Codex rollout {} belongs to cwd {}, not project {}.",
                        path.display(),
                        snapshot.cwd,
                        project_path.display_path
                    ));
                }
                continue;
            }
            let Some(usage) = snapshot.usage else {
                if warnings.len() < 8 {
                    warnings.push(format!(
                        "Task-linked Codex rollout {} has no total_token_usage record yet.",
                        path.display()
                    ));
                }
                continue;
            };
            let updated_at_ms = file_updated_at_ms(path);
            let row = CodexThreadRow {
                id: snapshot.session_id.clone(),
                rollout_path: path.display().to_string(),
                model_provider: None,
                model: snapshot.model,
                tokens_used: usage.total,
                updated_at_ms,
            };
            let should_replace = rows_by_id
                .get(&snapshot.session_id)
                .map_or(true, |current| current.updated_at_ms < row.updated_at_ms);
            if should_replace {
                rows_by_id.insert(snapshot.session_id, row);
            }
        }
    }

    let rows = rows_by_id.into_values().collect::<Vec<_>>();
    let paths = rows
        .iter()
        .map(|row| PathBuf::from(&row.rollout_path))
        .collect();
    (rows, paths)
}

fn codex_rollout_file_matches(file_name: &str, session_id: &str) -> bool {
    file_name == format!("{session_id}.jsonl")
        || file_name.ends_with(&format!("-{session_id}.jsonl"))
}

fn read_codex_rollout_snapshot(path: &Path) -> Result<CodexRolloutSnapshot> {
    let file = File::open(path)
        .with_context(|| format!("Failed to open Codex rollout {}", path.display()))?;
    let mut session_id = None;
    let mut cwd = None;
    let mut model = None;
    let mut usage = None;
    for line in BufReader::new(file).lines() {
        let Ok(line) = line else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if value.get("type").and_then(Value::as_str) == Some("session_meta") {
            session_id = value
                .pointer("/payload/id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            cwd = value
                .pointer("/payload/cwd")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
        }
        if let Some(next_model) = value
            .pointer("/payload/model")
            .and_then(Value::as_str)
            .filter(|model| !model.trim().is_empty())
        {
            model = Some(next_model.to_string());
        }
        if let Some(next_usage) = value
            .pointer("/payload/info/total_token_usage")
            .and_then(parse_codex_usage_value)
        {
            usage = Some(next_usage);
        }
    }

    let Some(session_id) = session_id else {
        anyhow::bail!("session_meta.payload.id is missing");
    };
    let Some(cwd) = cwd else {
        anyhow::bail!("session_meta.payload.cwd is missing");
    };
    Ok(CodexRolloutSnapshot {
        session_id,
        cwd,
        model,
        usage,
    })
}

fn file_updated_at_ms(path: &Path) -> Option<i64> {
    let duration = path
        .metadata()
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?;
    i64::try_from(duration.as_millis()).ok()
}

fn read_codex_rollout_usage(path: &str, warnings: &mut Vec<String>) -> Option<TokenBreakdown> {
    if path.trim().is_empty() {
        return None;
    }
    let path = PathBuf::from(path);
    let file = match File::open(&path) {
        Ok(file) => file,
        Err(_) => return None,
    };
    let mut last = None;
    for line in BufReader::new(file)
        .lines()
        .map_while(std::result::Result::ok)
    {
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let Some(usage) = value
            .pointer("/payload/info/total_token_usage")
            .and_then(parse_codex_usage_value)
        else {
            continue;
        };
        last = Some(usage);
    }
    if last.is_none() && warnings.len() < 6 {
        warnings.push(format!(
            "No total_token_usage found in Codex rollout {}.",
            path.display()
        ));
    }
    last
}

fn parse_codex_usage_value(value: &Value) -> Option<TokenBreakdown> {
    Some(TokenBreakdown {
        input: json_u64(value, "input_tokens"),
        output: json_u64(value, "output_tokens"),
        reasoning: json_u64(value, "reasoning_output_tokens"),
        cached_input: json_u64(value, "cached_input_tokens"),
        total: json_u64(value, "total_tokens"),
        cache_read: 0,
        cache_write: 0,
    })
}

fn codex_state_db_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(value) = env::var("CODEX_SQLITE_HOME") {
        candidates.push(PathBuf::from(value).join("state_5.sqlite"));
    }
    if let Ok(value) = env::var("CODEX_HOME") {
        let home = PathBuf::from(value);
        candidates.push(home.join("state_5.sqlite"));
        candidates.push(home.join("sqlite").join("state_5.sqlite"));
    }
    if let Some(home) = dirs::home_dir() {
        let codex_home = home.join(".codex");
        candidates.push(codex_home.join("state_5.sqlite"));
        candidates.push(codex_home.join("sqlite").join("state_5.sqlite"));
    }
    dedupe_paths(candidates)
}

fn codex_rollout_root_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(value) = env::var("CODEX_HOME") {
        let home = PathBuf::from(value);
        candidates.push(home.join("sessions"));
        candidates.push(home.join("archived_sessions"));
    }
    if let Some(home) = dirs::home_dir() {
        let codex_home = home.join(".codex");
        candidates.push(codex_home.join("sessions"));
        candidates.push(codex_home.join("archived_sessions"));
    }
    dedupe_paths(candidates)
}

fn opencode_db_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(value) = env::var("OPENCODE_DATA_HOME") {
        candidates.push(PathBuf::from(value).join("opencode.db"));
    }
    if let Ok(value) = env::var("XDG_DATA_HOME") {
        candidates.push(PathBuf::from(value).join("opencode").join("opencode.db"));
    }
    if let Some(data_dir) = dirs::data_dir() {
        candidates.push(data_dir.join("opencode").join("opencode.db"));
        candidates.push(data_dir.join("ai.opencode.desktop").join("opencode.db"));
    }
    if let Some(home) = dirs::home_dir() {
        candidates.push(
            home.join(".local")
                .join("share")
                .join("opencode")
                .join("opencode.db"),
        );
        candidates.push(
            home.join("Library")
                .join("Application Support")
                .join("ai.opencode.desktop")
                .join("opencode.db"),
        );
        candidates.push(
            home.join("AppData")
                .join("Roaming")
                .join("opencode")
                .join("opencode.db"),
        );
        candidates.push(
            home.join("AppData")
                .join("Local")
                .join("opencode")
                .join("opencode.db"),
        );
        candidates.push(
            home.join("AppData")
                .join("Roaming")
                .join("ai.opencode.desktop")
                .join("opencode.db"),
        );
    }
    dedupe_paths(candidates)
}

fn open_readonly(path: &Path) -> Result<Connection> {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("Failed to open SQLite database {}", path.display()))
}

fn existing_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths.into_iter().filter(|path| path.is_file()).collect()
}

fn unsupported_source(source: &str, reason: &str) -> AgentUsageSourceSummary {
    let mut summary = empty_source(source, reason);
    summary.status = "unsupported".to_string();
    summary
}

fn empty_source(source: &str, warning: &str) -> AgentUsageSourceSummary {
    AgentUsageSourceSummary {
        source: source.to_string(),
        status: "empty".to_string(),
        freshness: "unknown".to_string(),
        available: false,
        data_path: None,
        records: 0,
        non_cached_total_tokens: 0,
        total_tokens: 0,
        cost: None,
        tokens: TokenBreakdown::default(),
        latest_updated_at_ms: None,
        recent: Vec::new(),
        warnings: vec![warning.to_string()],
        matched_provider_session_ids: BTreeSet::new(),
    }
}

fn error_source(
    source: &str,
    path: Option<PathBuf>,
    error: anyhow::Error,
) -> AgentUsageSourceSummary {
    let mut summary = empty_source(source, &format!("{error:#}"));
    summary.status = if error.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .is_some_and(|io| io.kind() == std::io::ErrorKind::PermissionDenied)
    }) {
        "permission_denied".to_string()
    } else {
        "error".to_string()
    };
    summary.data_path = path.map(|path| path.display().to_string());
    summary
}

fn normalize_path_string(path: &Path) -> String {
    normalize_stored_path(&path.to_string_lossy())
}

fn normalize_stored_path(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let trimmed = normalized.trim_end_matches('/');
    if trimmed.is_empty() {
        normalized
    } else {
        trimmed.to_string()
    }
}

fn format_opencode_model(raw: &str) -> String {
    serde_json::from_str::<Value>(raw)
        .ok()
        .and_then(|value| {
            value
                .get("id")
                .and_then(Value::as_str)
                .or_else(|| value.get("model").and_then(Value::as_str))
                .map(str::to_string)
        })
        .unwrap_or_else(|| raw.chars().take(120).collect())
}

fn json_u64(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or_default()
}

fn signed_to_u64(value: i64) -> u64 {
    u64::try_from(value).unwrap_or_default()
}

fn max_opt_i64(current: Option<i64>, candidate: Option<i64>) -> Option<i64> {
    match (current, candidate) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for path in paths {
        if !out.iter().any(|existing| existing == &path) {
            out.push(path);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
    use std::{
        fs::{self, File},
        io::Write,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn unsupported_sources_never_fabricate_usage() {
        let source = unsupported_source("cursor", "no stable project-attributable contract");
        assert_eq!(source.status, "unsupported");
        assert!(!source.available);
        assert_eq!(source.records, 0);
        assert_eq!(source.total_tokens, 0);
        assert_eq!(source.cost, None);
        assert!(source.recent.is_empty());
        assert!(source.warnings[0].contains("no stable"));
    }

    #[test]
    fn usage_cache_stores_and_retrieves_entries() {
        let mut cache = UsageCache::new();
        let summary = test_source("codex", true, 10, 20, Some(1.0));
        cache.put("codex", "/tmp/test.sqlite", 12345, summary.clone());
        let retrieved = cache.get("codex", "/tmp/test.sqlite", 12345);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().total_tokens, 20);
        // Different mtime should miss
        assert!(cache.get("codex", "/tmp/test.sqlite", 99999).is_none());
        // Different path should miss
        assert!(cache.get("codex", "/tmp/other.sqlite", 12345).is_none());
        assert_eq!(cache.cache_state(), "warm_1_entries");
        cache.clear();
        assert_eq!(cache.cache_state(), "cold");
    }

    #[test]
    fn usage_cache_state_reflects_entries() {
        let mut cache = UsageCache::new();
        assert_eq!(cache.cache_state(), "cold");
        let s1 = test_source("claude_code", true, 5, 10, None);
        let s2 = test_source("opencode", true, 15, 30, None);
        cache.put("claude_code", "/path/a", 100, s1);
        assert_eq!(cache.cache_state(), "warm_1_entries");
        cache.put("opencode", "/path/b", 200, s2);
        assert_eq!(cache.cache_state(), "warm_2_entries");
    }

    #[test]
    fn refresh_summary_reports_cache_state() {
        let cache = Arc::new(Mutex::new(UsageCache::new()));
        let dir = temp_dir("cache-refresh");
        let project = dir.join("project");
        fs::create_dir_all(&project).expect("project dir");
        // First call: cache is cold, will be populated after scan
        let usage = read_local_agent_usage_with_cache(&project, cache.clone()).expect("usage");
        assert_eq!(usage.refresh.cache_state, "cold");
        // Cache should now have entries (if any provider found data)
        // Second call: cache state reflects warm entries
        let usage2 = read_local_agent_usage_with_cache(&project, cache).expect("usage2");
        // The cache may still be cold if no providers found data, but it should not error
        assert!(["cold", "warm_1_entries", "warm_2_entries", "warm_3_entries"].contains(&usage2.refresh.cache_state.as_str()));
    }

    #[test]
    fn claude_session_fails_closed_on_oversized_file() {
        let dir = temp_dir("claude-oversized");
        let session_path = dir.join("oversized.jsonl");
        let mut file = File::create(&session_path).expect("create");
        // Write more than MAX_TRANSCRIPT_FILE_SIZE bytes
        let chunk = vec![b'x'; 1024 * 1024]; // 1 MB chunk
        for _ in 0..11 {
            file.write_all(&chunk).expect("write");
        }
        drop(file);
        let result = read_claude_session(&session_path);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("exceeds maximum size"));
    }

    #[test]
    fn claude_session_fails_closed_on_implausible_token_counts() {
        let dir = temp_dir("claude-implausible");
        let session_path = dir.join("session-implausible.jsonl");
        let mut file = File::create(&session_path).expect("create");
        writeln!(
            file,
            r#"{{"type":"assistant","message":{{"id":"msg-evil","model":"claude-3-opus","usage":{{"input_tokens":999999999999,"output_tokens":500}}}}}}"#
        )
        .expect("write");
        drop(file);
        let session = read_claude_session(&session_path).expect("session");
        // The malicious line should be skipped, so total tokens should be 0
        assert_eq!(session.tokens.total, 0);
        assert!(session
            .warnings
            .iter()
            .any(|w| w.contains("implausible token counts")));
    }

    #[test]
    fn claude_session_truncates_on_too_many_lines() {
        let dir = temp_dir("claude-many-lines");
        let session_path = dir.join("session-many.jsonl");
        let mut file = File::create(&session_path).expect("create");
        // Write more than MAX_TRANSCRIPT_LINES lines
        for i in 0..100_001 {
            writeln!(file, r#"{{"type":"user","line":{}}}"#, i).expect("write");
        }
        drop(file);
        let session = read_claude_session(&session_path).expect("session");
        assert!(session
            .warnings
            .iter()
            .any(|w| w.contains("exceeds maximum line count")));
    }

    #[test]
    fn canonical_model_id_strips_dates_and_normalizes_aliases() {
        assert_eq!(canonical_model_id("gpt-4o-2024-08-06"), "gpt-4o");
        assert_eq!(canonical_model_id("GPT-4O"), "gpt-4o");
        assert_eq!(canonical_model_id("gpt-4o-mini-2024-07-18"), "gpt-4o-mini");
        assert_eq!(canonical_model_id("claude-3-opus-20240229"), "claude-3-opus");
        assert_eq!(canonical_model_id("claude-3-5-sonnet-20241022"), "claude-3-5-sonnet");
        assert_eq!(canonical_model_id("o1-preview-2024-09-12"), "o1-preview");
        assert_eq!(canonical_model_id("unknown-model-2024"), "unknown-model");
    }

    #[test]
    fn pricing_catalog_returns_versioned_prices_for_known_models() {
        let gpt4o = pricing_catalog("gpt-4o").expect("gpt-4o pricing");
        assert_eq!(gpt4o.input_per_1m, 2.50);
        assert_eq!(gpt4o.output_per_1m, 10.00);
        assert_eq!(gpt4o.cache_read_per_1m, Some(1.25));
        let claude = pricing_catalog("claude-3-opus").expect("claude pricing");
        assert_eq!(claude.input_per_1m, 15.00);
        assert_eq!(claude.output_per_1m, 75.00);
        assert!(pricing_catalog("unknown-model-xyz").is_none());
    }

    #[test]
    fn model_pricing_computes_cost_with_cache_and_reasoning() {
        let pricing = pricing_catalog("gpt-4o").expect("gpt-4o pricing");
        let tokens = TokenBreakdown {
            input: 1_000_000,
            output: 500_000,
            reasoning: 0,
            cached_input: 0,
            cache_read: 200_000,
            cache_write: 100_000,
            total: 1_800_000,
        };
        let cost = pricing.cost_for(&tokens).expect("cost");
        // 1M * 2.5 + 0.5M * 10 + 0.2M * 1.25 + 0.1M * 2.5 = 2.5 + 5 + 0.25 + 0.25 = 8.0
        assert!((cost - 8.0).abs() < 0.001);
    }

    #[test]
    fn claude_code_cost_computed_from_pricing_catalog() {
        let dir = temp_dir("claude-pricing");
        let session_path = dir.join("session-cost.jsonl");
        let mut file = File::create(&session_path).expect("create session file");
        writeln!(
            file,
            r#"{{"type":"assistant","message":{{"id":"msg-1","model":"claude-3-opus","usage":{{"input_tokens":1000000,"output_tokens":500000,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}}}}}}"#
        )
        .expect("write");
        drop(file);

        let matcher = ProjectPathMatcher::new(Path::new("/repo/app"));
        let usage = read_claude_code_usage_from_dir(&matcher, &dir);
        assert!(usage.available);
        assert_eq!(usage.total_tokens, 1_500_000);
        // Cost should be computed: 1M * 15 + 0.5M * 75 = 15 + 37.5 = 52.5
        let cost = usage.cost.expect("cost should be computed");
        assert!((cost - 52.5).abs() < 0.001, "cost was {}", cost);
        assert_eq!(usage.recent[0].cost, Some(cost));
    }

    #[test]
    fn opencode_cost_falls_back_to_pricing_catalog_when_db_cost_zero() {
        let dir = temp_dir("opencode-pricing-fallback");
        let db_path = dir.join("opencode.db");
        let conn = Connection::open(&db_path).expect("open fixture db");
        create_opencode_schema(&conn);
        conn.execute(
            "insert into project (id, worktree) values ('p1', '/repo/app')",
            [],
        )
        .expect("project");
        conn.execute(
            "insert into session (
                id, project_id, directory, title, agent, model, cost, tokens_input,
                tokens_output, tokens_reasoning, tokens_cache_read, tokens_cache_write,
                time_updated, path
            ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                "ses-pricing",
                "p1",
                "/repo/app",
                "Pricing test",
                "build",
                r#"{"id":"gpt-4o"}"#,
                0.0_f64,
                1_000_000_i64,
                500_000_i64,
                0_i64,
                200_000_i64,
                100_000_i64,
                456_i64,
                "/repo/app",
            ],
        )
        .expect("session");
        drop(conn);

        let usage = read_opencode_usage_from_db("/repo/app", &db_path).expect("usage");
        assert!(usage.available);
        assert_eq!(usage.total_tokens, 1_800_000);
        // Cost should be computed from pricing catalog: 1M*2.5 + 0.5M*10 + 0.2M*1.25 + 0.1M*2.5 = 8.0
        let cost = usage.cost.expect("cost should be computed");
        assert!((cost - 8.0).abs() < 0.001, "cost was {}", cost);
    }

    #[test]
    fn opencode_uses_db_cost_when_available() {
        let dir = temp_dir("opencode-db-cost");
        let db_path = dir.join("opencode.db");
        let conn = Connection::open(&db_path).expect("open fixture db");
        create_opencode_schema(&conn);
        conn.execute(
            "insert into project (id, worktree) values ('p1', '/repo/app')",
            [],
        )
        .expect("project");
        conn.execute(
            "insert into session (
                id, project_id, directory, title, agent, model, cost, tokens_input,
                tokens_output, tokens_reasoning, tokens_cache_read, tokens_cache_write,
                time_updated, path
            ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                "ses-db-cost",
                "p1",
                "/repo/app",
                "DB cost test",
                "build",
                r#"{"id":"gpt-4o"}"#,
                99.99_f64,
                100_i64,
                50_i64,
                0_i64,
                0_i64,
                0_i64,
                456_i64,
                "/repo/app",
            ],
        )
        .expect("session");
        drop(conn);

        let usage = read_opencode_usage_from_db("/repo/app", &db_path).expect("usage");
        assert!(usage.available);
        assert_eq!(usage.cost, Some(99.99));
    }

    #[test]
    fn cost_state_partial_when_some_sources_unpriced() {
        let claude = test_source("claude_code", true, 10, 20, Some(1.0));
        let codex = test_source("codex", true, 30, 40, None);
        let opencode = test_source("opencode", true, 15, 24, Some(0.5));
        let audit = build_usage_audit(
            "2026-07-28T00:00:00Z",
            "fresh",
            "partial",
            84,
            [&claude, &codex, &opencode],
            0,
            0,
        );
        assert_eq!(audit.cost.state, "partial");
        assert_eq!(audit.cost.priced_tokens, 44);
        assert_eq!(audit.cost.unpriced_tokens, 40);
        assert!(audit
            .cost
            .missing_reasons
            .iter()
            .any(|r| r.contains("codex")));
        assert!(!audit.cost.repair_actions.is_empty());
    }

    #[test]
    fn cost_state_complete_when_all_sources_priced() {
        let claude = test_source("claude_code", true, 10, 20, Some(1.0));
        let codex = test_source("codex", true, 30, 40, Some(2.0));
        let opencode = test_source("opencode", true, 15, 24, Some(0.5));
        let audit = build_usage_audit(
            "2026-07-28T00:00:00Z",
            "fresh",
            "complete",
            84,
            [&claude, &codex, &opencode],
            0,
            0,
        );
        assert_eq!(audit.cost.state, "complete");
        assert_eq!(audit.cost.priced_tokens, 84);
        assert_eq!(audit.cost.unpriced_tokens, 0);
        assert!(audit.cost.missing_reasons.is_empty());
    }

    #[test]
    fn aggregates_codex_threads_and_rollout_usage() {
        let dir = temp_dir("codex-usage");
        let db_path = dir.join("state_5.sqlite");
        let rollout_path = dir.join("rollout.jsonl");
        write_rollout(&rollout_path, 11, 7, 3, 5, 21);
        let conn = Connection::open(&db_path).expect("open fixture db");
        create_codex_schema(&conn);
        conn.execute(
            "insert into threads (id, rollout_path, cwd, title, tokens_used, model, updated_at_ms)
             values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                "thread-1",
                rollout_path.display().to_string(),
                "/repo/app",
                "Usage work",
                21_i64,
                "gpt-test",
                123_i64
            ],
        )
        .expect("insert");
        drop(conn);

        let mut warnings = Vec::new();
        let usage = read_codex_usage_from_db("/repo/app", &db_path, &mut warnings).expect("usage");
        assert!(usage.available);
        assert_eq!(usage.records, 1);
        assert_eq!(usage.total_tokens, 21);
        assert_eq!(usage.non_cached_total_tokens, 14);
        assert_eq!(usage.tokens.input, 11);
        assert_eq!(usage.tokens.cached_input, 7);
        assert_eq!(usage.recent[0].non_cached_total_tokens, 14);
        assert_eq!(usage.recent[0].model.as_deref(), Some("gpt-test"));
    }

    #[test]
    fn task_scoped_codex_usage_reads_live_rollout_without_sqlite_catalog_row() {
        let dir = temp_dir("codex-live-task-rollout");
        let sessions_root = dir.join("sessions");
        let day_dir = sessions_root.join("2026/07/14");
        fs::create_dir_all(&day_dir).expect("create Codex session tree");
        let thread_id = "019f5f37-d5a6-7b63-b2d3-59e8d872969b";
        let rollout_path = day_dir.join(format!("rollout-2026-07-14T14-02-02-{thread_id}.jsonl"));
        write_rollout_with_meta(
            &rollout_path,
            thread_id,
            "/repo/app",
            28_175_555,
            27_321_600,
            57_124,
            29_086,
            28_232_679,
        );
        let filter = UsageSessionFilter {
            task_id: "task.task-token".to_string(),
            session_links: [(format!("session.codex.{thread_id}"), BTreeMap::new())]
                .into_iter()
                .collect(),
        };
        assert!(filter.matches_provider("codex", thread_id));
        assert!(!filter.matches_provider("claude", thread_id));

        let matcher = ProjectPathMatcher::new(Path::new("/repo/app"));
        let mut warnings = Vec::new();
        let (rows, paths) = read_codex_task_rollout_rows_from_roots(
            &matcher,
            &filter,
            &[sessions_root],
            &mut warnings,
        );

        assert_eq!(rows.len(), 1, "warnings: {warnings:?}");
        assert_eq!(rows[0].id, thread_id);
        assert_eq!(rows[0].tokens_used, 28_232_679);
        assert_eq!(paths, vec![rollout_path]);
        let usage = read_codex_rollout_usage(&rows[0].rollout_path, &mut warnings)
            .expect("rollout token usage");
        assert_eq!(usage.total, 28_232_679);
        assert_eq!(usage.cached_input, 27_321_600);
    }

    #[test]
    fn task_scoped_usage_uses_explicit_provider_link_for_synthetic_workflow_id() {
        let dir = temp_dir("codex-explicit-provider-link");
        let sessions_root = dir.join("sessions");
        let day_dir = sessions_root.join("2026/07/14");
        fs::create_dir_all(&day_dir).expect("create Codex session tree");
        let provider_session_id = "019f5fea-18e1-7732-ab81-1729fd582f7d";
        let rollout_path = day_dir.join(format!(
            "rollout-2026-07-14T17-16-45-{provider_session_id}.jsonl"
        ));
        write_rollout_with_meta(
            &rollout_path,
            provider_session_id,
            "/repo/app",
            100,
            20,
            30,
            40,
            170,
        );
        let workflow_session_id = "session.codex.task.v3.synthetic.diagnostic-20260714".to_string();
        let session_links = [(
            workflow_session_id,
            [(
                "codex".to_string(),
                [provider_session_id.to_string()].into_iter().collect(),
            )]
            .into_iter()
            .collect(),
        )]
        .into_iter()
        .collect();
        let filter = UsageSessionFilter {
            task_id: "task.synthetic".to_string(),
            session_links,
        };

        let matcher = ProjectPathMatcher::new(Path::new("/repo/app"));
        let mut warnings = Vec::new();
        let (rows, _) = read_codex_task_rollout_rows_from_roots(
            &matcher,
            &filter,
            &[sessions_root],
            &mut warnings,
        );

        assert_eq!(rows.len(), 1, "warnings: {warnings:?}");
        assert_eq!(rows[0].id, provider_session_id);
        assert_eq!(
            filter.provider_session_ids("codex"),
            [provider_session_id.to_string()].into_iter().collect()
        );
    }

    #[test]
    fn matched_session_count_counts_workflow_sessions_not_shared_provider_records() {
        let provider_session_id = "019f5fea-18e1-7732-ab81-1729fd582f7d";
        let provider_link = |id: &str| {
            [(
                "codex".to_string(),
                [id.to_string()].into_iter().collect::<BTreeSet<_>>(),
            )]
            .into_iter()
            .collect::<BTreeMap<_, _>>()
        };
        let filter = UsageSessionFilter {
            task_id: "task.shared-provider".to_string(),
            session_links: [
                (
                    "session.codex.workflow-a".to_string(),
                    provider_link(provider_session_id),
                ),
                (
                    "session.codex.workflow-b".to_string(),
                    provider_link(provider_session_id),
                ),
                (
                    "session.codex.workflow-unmatched".to_string(),
                    provider_link("missing-provider-session"),
                ),
            ]
            .into_iter()
            .collect(),
        };
        let mut codex = test_source("codex", true, 0, 170, None);
        codex.matched_provider_session_ids =
            [provider_session_id.to_string()].into_iter().collect();
        let claude = test_source("claude_code", false, 0, 0, None);
        let opencode = test_source("opencode", false, 0, 0, None);

        assert_eq!(
            filter.matched_workflow_session_count(&[
                ("claude", &claude),
                ("codex", &codex),
                ("opencode", &opencode),
            ]),
            2
        );
        assert_eq!(
            codex.records, 1,
            "provider Token totals remain deduplicated"
        );
    }

    #[test]
    fn scans_later_codex_databases_and_requires_exact_project_path() {
        let dir = temp_dir("codex-multi-db");
        let empty_db_path = dir.join("empty-state_5.sqlite");
        let matched_db_path = dir.join("matched-state_5.sqlite");
        let empty_conn = Connection::open(&empty_db_path).expect("open empty fixture db");
        create_codex_schema(&empty_conn);
        drop(empty_conn);

        let rollout_path = dir.join("rollout-child.jsonl");
        write_rollout(&rollout_path, 20, 0, 4, 1, 25);
        let matched_conn = Connection::open(&matched_db_path).expect("open matched fixture db");
        create_codex_schema(&matched_conn);
        matched_conn
            .execute(
                "insert into threads (id, rollout_path, cwd, title, tokens_used, model, updated_at_ms)
                 values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    "thread-child",
                    rollout_path.display().to_string(),
                    "/repo/app",
                    "Exact project work",
                    25_i64,
                    "gpt-test",
                    321_i64
                ],
            )
            .expect("insert child");
        matched_conn
            .execute(
                "insert into threads (id, rollout_path, cwd, title, tokens_used, model, updated_at_ms)
                 values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    "thread-sibling-prefix",
                    rollout_path.display().to_string(),
                    "/repo/application",
                    "Should not match",
                    999_i64,
                    "gpt-test",
                    322_i64
                ],
            )
            .expect("insert sibling prefix");
        drop(matched_conn);

        let mut warnings = Vec::new();
        let matcher = ProjectPathMatcher::new(Path::new("/repo/app"));
        let usage = read_codex_usage_from_dbs(
            &matcher,
            &[empty_db_path.clone(), matched_db_path.clone()],
            &mut warnings,
        )
        .expect("usage");

        assert!(usage.available);
        assert_eq!(usage.records, 1);
        assert_eq!(usage.total_tokens, 25);
        assert_eq!(usage.non_cached_total_tokens, 25);
        assert_eq!(
            usage.data_path.as_deref(),
            Some(matched_db_path.to_str().unwrap())
        );
    }

    #[test]
    fn codex_refuses_unique_same_name_project_path() {
        let dir = temp_dir("codex-alias-path");
        let db_path = dir.join("state_5.sqlite");
        let rollout_path = dir.join("rollout-alias.jsonl");
        write_rollout(&rollout_path, 30, 5, 4, 1, 40);
        let conn = Connection::open(&db_path).expect("open fixture db");
        create_codex_schema(&conn);
        conn.execute(
            "insert into threads (id, rollout_path, cwd, title, tokens_used, model, updated_at_ms)
             values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                "thread-alias",
                rollout_path.display().to_string(),
                "/repo/local/mind2realistic",
                "Moved project work",
                40_i64,
                "gpt-test",
                654_i64
            ],
        )
        .expect("insert alias");
        drop(conn);

        let mut warnings = Vec::new();
        let usage =
            read_codex_usage_from_db("/repo/archive/mind2realistic", &db_path, &mut warnings)
                .expect("usage");

        assert!(!usage.available);
        assert_eq!(usage.records, 0);
        assert_eq!(usage.total_tokens, 0);
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("/repo/local/mind2realistic")),
            "expected alias warning, got {:?}",
            warnings
        );
    }

    #[test]
    fn aggregates_opencode_sessions() {
        let dir = temp_dir("opencode-usage");
        let db_path = dir.join("opencode.db");
        let conn = Connection::open(&db_path).expect("open fixture db");
        create_opencode_schema(&conn);
        conn.execute(
            "insert into project (id, worktree) values ('p1', '/repo/app')",
            [],
        )
        .expect("project");
        conn.execute(
            "insert into session (
                id, project_id, directory, title, agent, model, cost, tokens_input,
                tokens_output, tokens_reasoning, tokens_cache_read, tokens_cache_write,
                time_updated, path
            ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                "ses-1",
                "p1",
                "/repo/app",
                "OpenCode work",
                "build",
                r#"{"id":"model-a"}"#,
                1.25_f64,
                10_i64,
                2_i64,
                3_i64,
                4_i64,
                5_i64,
                456_i64,
                "/repo/app",
            ],
        )
        .expect("session");
        drop(conn);

        let usage = read_opencode_usage_from_db("/repo/app", &db_path).expect("usage");
        assert!(usage.available);
        assert_eq!(usage.records, 1);
        assert_eq!(usage.total_tokens, 24);
        assert_eq!(usage.non_cached_total_tokens, 15);
        assert_eq!(usage.tokens.cache_read, 4);
        assert_eq!(usage.recent[0].non_cached_total_tokens, 15);
        assert_eq!(usage.recent[0].model.as_deref(), Some("model-a"));
    }

    #[test]
    fn repeated_and_duplicate_database_scans_are_deterministic() {
        let dir = temp_dir("usage-repeat-dedupe");
        let codex_db = dir.join("state_5.sqlite");
        let rollout = dir.join("rollout.jsonl");
        write_rollout(&rollout, 11, 7, 2, 1, 21);
        let conn = Connection::open(&codex_db).expect("open Codex fixture");
        create_codex_schema(&conn);
        conn.execute(
            "insert into threads (id, rollout_path, cwd, title, tokens_used, model, updated_at_ms)
             values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                "thread-stable",
                rollout.display().to_string(),
                "/repo/app",
                "secret title",
                21_i64,
                "gpt-test",
                321_i64
            ],
        )
        .expect("insert Codex fixture");
        drop(conn);
        let matcher = ProjectPathMatcher::new(Path::new("/repo/app"));
        let mut warnings = Vec::new();
        let first = read_codex_usage_from_dbs(
            &matcher,
            &[codex_db.clone(), codex_db.clone()],
            &mut warnings,
        )
        .expect("first scan");
        let second =
            read_codex_usage_from_dbs(&matcher, &[codex_db.clone(), codex_db], &mut Vec::new())
                .expect("restart scan");
        assert_eq!(first.records, 1);
        assert_eq!(first.total_tokens, 21);
        assert_eq!(first.total_tokens, second.total_tokens);
        assert_eq!(first.recent[0].id, second.recent[0].id);
        assert_eq!(first.recent[0].title, "Codex session");

        let opencode_db = dir.join("opencode.db");
        let conn = Connection::open(&opencode_db).expect("open OpenCode fixture");
        create_opencode_schema(&conn);
        conn.execute(
            "insert into project (id, worktree) values ('p1', '/repo/app')",
            [],
        )
        .expect("project");
        conn.execute(
            "insert into session (id, project_id, directory, title, agent, model, cost, tokens_input, tokens_output, tokens_reasoning, tokens_cache_read, tokens_cache_write, time_updated, path)
             values ('ses-stable', 'p1', '/repo/app', 'secret title', 'build', '{\"id\":\"model-a\"}', 1.0, 10, 2, 3, 4, 5, 456, '/repo/app')",
            [],
        )
        .expect("session");
        drop(conn);
        let mut warnings = Vec::new();
        let first = read_opencode_usage_from_dbs_scoped(
            &matcher,
            &[opencode_db.clone(), opencode_db.clone()],
            &mut warnings,
            None,
        )
        .expect("first OpenCode scan");
        let second = read_opencode_usage_from_dbs_scoped(
            &matcher,
            &[opencode_db.clone(), opencode_db],
            &mut Vec::new(),
            None,
        )
        .expect("restart OpenCode scan");
        assert_eq!(first.records, 1);
        assert_eq!(first.total_tokens, 24);
        assert_eq!(first.total_tokens, second.total_tokens);
        assert_eq!(first.recent[0].title, "OpenCode session");
    }

    #[test]
    fn source_freshness_distinguishes_fresh_stale_and_unknown() {
        let generated_at_ms = 2_000_000;
        let mut fresh = test_source("fresh", true, 10, 20, None);
        fresh.latest_updated_at_ms = Some(generated_at_ms - 899_000);
        apply_source_freshness(&mut fresh, generated_at_ms, 900);
        assert_eq!(fresh.freshness, "fresh");
        assert_eq!(fresh.status, "available");
        assert_eq!(fresh.total_tokens, 20);

        let mut stale = test_source("stale", true, 10, 20, None);
        stale.latest_updated_at_ms = Some(generated_at_ms - 901_000);
        apply_source_freshness(&mut stale, generated_at_ms, 900);
        assert_eq!(stale.freshness, "stale");
        assert_eq!(stale.status, "stale");
        assert_eq!(stale.total_tokens, 20);
        assert!(stale
            .warnings
            .iter()
            .any(|warning| warning.contains("901 seconds old")));

        let mut unknown = test_source("unknown", true, 10, 20, None);
        apply_source_freshness(&mut unknown, generated_at_ms, 900);
        assert_eq!(unknown.freshness, "unknown");
        assert_eq!(unknown.status, "partial");
        assert_eq!(unknown.total_tokens, 20);
    }

    #[test]
    fn one_stale_source_makes_multi_source_overview_stale_and_partial() {
        let generated_at_ms = 2_000_000;
        let mut fresh = test_source("claude_code", true, 10, 20, None);
        fresh.latest_updated_at_ms = Some(generated_at_ms - 100_000);
        apply_source_freshness(&mut fresh, generated_at_ms, 900);

        let mut stale = test_source("codex", true, 30, 40, None);
        stale.latest_updated_at_ms = Some(generated_at_ms - 901_000);
        apply_source_freshness(&mut stale, generated_at_ms, 900);

        let empty = test_source("opencode", false, 0, 0, None);
        let sources = [&fresh, &stale, &empty];
        assert_eq!(overall_freshness(&sources), "stale");
        assert_eq!(overall_completeness(&sources, 60), "partial");
        assert_eq!(fresh.total_tokens + stale.total_tokens, 60);
    }

    #[test]
    fn audit_provider_breakdown_conserves_total_and_excludes_unmatched_task_sessions() {
        let claude = test_source("claude_code", true, 10, 20, None);
        let codex = test_source("codex", true, 30, 40, None);
        let opencode = test_source("opencode", true, 15, 24, Some(1.25));
        let audit = build_usage_audit(
            "2026-07-28T00:00:00Z",
            "fresh",
            "partial",
            84,
            [&claude, &codex, &opencode],
            4,
            3,
        );
        let visible_provider_total = audit
            .breakdowns
            .iter()
            .filter(|item| item.kind == "provider")
            .filter_map(|item| item.total_tokens)
            .sum::<u64>();
        assert_eq!(visible_provider_total, 84);
        assert_eq!(audit.excluded_records, 1);
        assert_eq!(audit.unattributed_tokens, 0);
        assert_eq!(audit.cost.state, "partial");
        assert_eq!(audit.cost.priced_tokens, 24);
        assert_eq!(audit.cost.unpriced_tokens, 60);
    }

    #[test]
    fn primary_metric_uses_total_tokens_even_when_cost_is_available() {
        let codex = test_source("codex", false, 0, 0, None);
        let opencode = test_source("opencode", true, 15, 24, Some(1.25));

        let claude = test_source("claude_code", false, 0, 0, None);
        let metric = select_primary_metric(&claude, &codex, &opencode, 24);

        assert_eq!(metric.kind, "tokens");
        assert_eq!(metric.source, "opencode");
        assert_eq!(metric.currency, None);
        assert_eq!(metric.value, None);
        assert_eq!(metric.tokens, Some(24));
        assert!(!metric.estimated);
    }

    #[test]
    fn primary_metric_uses_codex_tokens_without_opencode() {
        let codex = test_source("codex", true, 14, 21, None);
        let opencode = test_source("opencode", false, 0, 0, None);

        let claude = test_source("claude_code", false, 0, 0, None);
        let metric = select_primary_metric(&claude, &codex, &opencode, 21);

        assert_eq!(metric.kind, "tokens");
        assert_eq!(metric.source, "codex");
        assert_eq!(metric.value, None);
        assert_eq!(metric.currency, None);
        assert_eq!(metric.tokens, Some(21));
        assert!(!metric.estimated);
    }

    #[test]
    fn primary_metric_is_unavailable_without_matching_usage() {
        let codex = test_source("codex", false, 0, 0, None);
        let opencode = test_source("opencode", false, 0, 0, None);

        let claude = test_source("claude_code", false, 0, 0, None);
        let metric = select_primary_metric(&claude, &codex, &opencode, 0);

        assert_eq!(metric.kind, "unavailable");
        assert_eq!(metric.source, "none");
        assert_eq!(metric.value, None);
        assert_eq!(metric.tokens, None);
        assert!(!metric.estimated);
    }

    #[test]
    #[ignore = "reads the developer's local Claude Code transcript directory without exposing content"]
    fn smoke_reads_real_vibehub_claude_code_usage() {
        let project = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("repository root");
        let usage = read_local_agent_usage(project).expect("real local usage read");
        assert_eq!(usage.project_path, normalize_path_string(project));
        assert_ne!(usage.claude_code.status, "error");
        assert_ne!(usage.claude_code.status, "permission_denied");
        assert!(usage.claude_code.records > 0);
        assert!(usage.claude_code.total_tokens > 0);
    }

    #[test]
    #[ignore = "reads the Task-linked Codex rollout named by VIBEHUB_CODEX_SESSION_ID"]
    fn smoke_reads_real_vibehub_task_codex_usage() {
        let project = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("repository root");
        let provider_session_id =
            env::var("VIBEHUB_CODEX_SESSION_ID").expect("VIBEHUB_CODEX_SESSION_ID");
        let workflow_session_id = format!("session.codex.{provider_session_id}");
        let usage = read_local_agent_usage_for_task(
            project,
            "task.smoke".to_string(),
            [(workflow_session_id, BTreeMap::new())]
                .into_iter()
                .collect(),
        )
        .expect("real Task-scoped local usage read");

        assert_eq!(usage.requested_session_count, 1);
        assert_eq!(usage.matched_session_count, 1, "{:?}", usage.warnings);
        assert_eq!(usage.codex.records, 1);
        assert!(usage.codex.total_tokens > 0);
        assert_eq!(usage.total_tokens, usage.codex.total_tokens);
        eprintln!("Task-scoped Codex total tokens: {}", usage.total_tokens);
    }

    #[test]
    fn opencode_refuses_child_paths_and_prefix_siblings() {
        let dir = temp_dir("opencode-child-path");
        let db_path = dir.join("opencode.db");
        let conn = Connection::open(&db_path).expect("open fixture db");
        create_opencode_schema(&conn);
        conn.execute(
            "insert into project (id, worktree) values ('p1', '/repo/app/packages/web')",
            [],
        )
        .expect("project child");
        conn.execute(
            "insert into project (id, worktree) values ('p2', '/repo/application')",
            [],
        )
        .expect("project sibling prefix");
        conn.execute(
            "insert into session (
                id, project_id, directory, title, agent, model, cost, tokens_input,
                tokens_output, tokens_reasoning, tokens_cache_read, tokens_cache_write,
                time_updated, path
            ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                "ses-child",
                "p1",
                "/repo/app/packages/web",
                "Child OpenCode work",
                "build",
                r#"{"id":"model-a"}"#,
                1.0_f64,
                10_i64,
                2_i64,
                3_i64,
                4_i64,
                5_i64,
                456_i64,
                "/repo/app/packages/web",
            ],
        )
        .expect("child session");
        conn.execute(
            "insert into session (
                id, project_id, directory, title, agent, model, cost, tokens_input,
                tokens_output, tokens_reasoning, tokens_cache_read, tokens_cache_write,
                time_updated, path
            ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                "ses-sibling-prefix",
                "p2",
                "/repo/application",
                "Should not match",
                "build",
                r#"{"id":"model-b"}"#,
                1.0_f64,
                1000_i64,
                200_i64,
                300_i64,
                400_i64,
                500_i64,
                457_i64,
                "/repo/application",
            ],
        )
        .expect("sibling prefix session");
        drop(conn);

        let usage = read_opencode_usage_from_db("/repo/app", &db_path).expect("usage");
        assert!(!usage.available);
        assert_eq!(usage.records, 0);
        assert_eq!(usage.total_tokens, 0);
    }

    #[test]
    fn opencode_refuses_unique_same_name_project_path() {
        let dir = temp_dir("opencode-alias-path");
        let db_path = dir.join("opencode.db");
        let conn = Connection::open(&db_path).expect("open fixture db");
        create_opencode_schema(&conn);
        conn.execute(
            "insert into project (id, worktree) values ('p1', '/repo/local/mind2realistic')",
            [],
        )
        .expect("project alias");
        conn.execute(
            "insert into session (
                id, project_id, directory, title, agent, model, cost, tokens_input,
                tokens_output, tokens_reasoning, tokens_cache_read, tokens_cache_write,
                time_updated, path
            ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                "ses-alias",
                "p1",
                "/repo/local/mind2realistic",
                "Moved OpenCode work",
                "build",
                r#"{"id":"model-a"}"#,
                1.0_f64,
                10_i64,
                2_i64,
                3_i64,
                4_i64,
                5_i64,
                654_i64,
                "/repo/local/mind2realistic",
            ],
        )
        .expect("alias session");
        drop(conn);

        let usage =
            read_opencode_usage_from_db("/repo/archive/mind2realistic", &db_path).expect("usage");

        assert!(!usage.available);
        assert_eq!(usage.records, 0);
        assert_eq!(usage.total_tokens, 0);
        assert!(
            usage
                .warnings
                .iter()
                .any(|warning| warning.contains("/repo/local/mind2realistic")),
            "expected alias warning, got {:?}",
            usage.warnings
        );
    }

    #[test]
    fn reads_claude_code_sessions_with_dedupe_cache_models_and_partial_warnings() {
        let dir = temp_dir("claude-code-fixtures");
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/claude-code/normal-single-model.jsonl"),
            dir.join("session-normal.jsonl"),
        )
        .expect("copy normal fixture");
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/claude-code/model-switch-duplicate-partial.jsonl"),
            dir.join("session-partial.jsonl"),
        )
        .expect("copy partial fixture");

        let matcher = ProjectPathMatcher::new(Path::new("/Users/example/Project One"));
        let usage = read_claude_code_usage_from_dir(&matcher, &dir);

        assert!(usage.available);
        assert_eq!(usage.status, "partial");
        assert_eq!(usage.records, 2);
        assert_eq!(usage.tokens.input, 200);
        assert_eq!(usage.tokens.output, 40);
        assert_eq!(usage.tokens.cache_read, 42);
        assert_eq!(usage.tokens.cache_write, 15);
        assert_eq!(usage.total_tokens, 297);
        let partial = usage
            .recent
            .iter()
            .find(|item| item.id == "session-partial")
            .expect("partial session");
        assert_eq!(partial.message_count, 3);
        assert_eq!(partial.total_tokens, 137);
        assert_eq!(
            partial.models,
            vec!["claude-opus-test", "claude-sonnet-test"]
        );
        assert_eq!(partial.tool_uses, 1);
        assert!(usage
            .warnings
            .iter()
            .any(|warning| warning.contains("invalid JSON")));
        assert!(usage
            .warnings
            .iter()
            .any(|warning| warning.contains("lineage")));
        assert!(usage
            .warnings
            .iter()
            .any(|warning| warning.contains("has no usage")));
    }

    #[test]
    fn task_usage_filter_excludes_unlinked_claude_sessions() {
        let dir = temp_dir("claude-task-scope");
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/claude-code/normal-single-model.jsonl"),
            dir.join("session-normal.jsonl"),
        )
        .expect("copy normal fixture");
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/claude-code/model-switch-duplicate-partial.jsonl"),
            dir.join("session-partial.jsonl"),
        )
        .expect("copy partial fixture");

        let matcher = ProjectPathMatcher::new(Path::new("/Users/example/Project One"));
        let filter = UsageSessionFilter {
            task_id: "task.example".to_string(),
            session_links: [("session-partial".to_string(), BTreeMap::new())]
                .into_iter()
                .collect(),
        };
        let usage = read_claude_code_usage_from_dir_scoped(&matcher, &dir, Some(&filter));

        assert_eq!(usage.records, 1);
        assert_eq!(usage.total_tokens, 137);
        assert_eq!(usage.recent[0].id, "session-partial");
    }

    #[test]
    fn claude_code_empty_and_read_error_statuses_are_explicit() {
        let matcher = ProjectPathMatcher::new(Path::new(r"C:\Users\Example\Project One"));
        assert_eq!(
            encode_claude_project_path(&matcher.primary),
            "C--Users-Example-Project One"
        );
        let empty = temp_dir("claude-empty");
        let usage = read_claude_code_usage_from_dir(&matcher, &empty);
        assert_eq!(usage.status, "empty");
        assert!(!usage.available);

        let not_directory = empty.join("not-a-directory");
        File::create(&not_directory).expect("file fixture");
        let error = read_claude_code_usage_from_dir(&matcher, &not_directory);
        assert_eq!(error.status, "error");
    }

    #[test]
    fn encodes_macos_and_windows_claude_project_paths() {
        assert_eq!(
            encode_claude_project_path("/Users/example/Project One"),
            "-Users-example-Project One"
        );
        assert_eq!(
            encode_claude_project_path(r"C:\Users\example\Project One"),
            "C--Users-example-Project One"
        );
    }

    fn create_codex_schema(conn: &Connection) {
        conn.execute_batch(
            "create table threads (
                id text primary key,
                rollout_path text not null,
                created_at integer not null default 0,
                updated_at integer not null default 0,
                source text not null default '',
                model_provider text not null default '',
                cwd text not null,
                title text not null,
                sandbox_policy text not null default '',
                approval_mode text not null default '',
                tokens_used integer not null default 0,
                model text,
                updated_at_ms integer
            );",
        )
        .expect("codex schema");
    }

    fn test_source(
        source: &str,
        available: bool,
        non_cached_total_tokens: u64,
        total_tokens: u64,
        cost: Option<f64>,
    ) -> AgentUsageSourceSummary {
        AgentUsageSourceSummary {
            source: source.to_string(),
            status: if available { "available" } else { "empty" }.to_string(),
            freshness: "unknown".to_string(),
            available,
            data_path: None,
            records: usize::from(available),
            non_cached_total_tokens,
            total_tokens,
            cost,
            tokens: TokenBreakdown {
                total: total_tokens,
                ..TokenBreakdown::default()
            },
            latest_updated_at_ms: None,
            recent: Vec::new(),
            warnings: Vec::new(),
            matched_provider_session_ids: BTreeSet::new(),
        }
    }

    fn create_opencode_schema(conn: &Connection) {
        conn.execute_batch(
            "create table project (
                id text primary key,
                worktree text not null
            );
            create table session (
                id text primary key,
                project_id text not null,
                directory text not null,
                title text not null,
                agent text,
                model text,
                cost real default 0 not null,
                tokens_input integer default 0 not null,
                tokens_output integer default 0 not null,
                tokens_reasoning integer default 0 not null,
                tokens_cache_read integer default 0 not null,
                tokens_cache_write integer default 0 not null,
                time_updated integer,
                path text
            );",
        )
        .expect("opencode schema");
    }

    fn write_rollout(
        path: &Path,
        input: u64,
        cached: u64,
        output: u64,
        reasoning: u64,
        total: u64,
    ) {
        let mut file = File::create(path).expect("rollout");
        writeln!(
            file,
            "{{\"payload\":{{\"info\":{{\"total_token_usage\":{{\"input_tokens\":{},\"cached_input_tokens\":{},\"output_tokens\":{},\"reasoning_output_tokens\":{},\"total_tokens\":{}}}}}}}}}",
            input, cached, output, reasoning, total
        )
        .expect("write");
    }

    #[allow(clippy::too_many_arguments)]
    fn write_rollout_with_meta(
        path: &Path,
        session_id: &str,
        cwd: &str,
        input: u64,
        cached: u64,
        output: u64,
        reasoning: u64,
        total: u64,
    ) {
        let mut file = File::create(path).expect("rollout");
        writeln!(
            file,
            "{}",
            serde_json::json!({
                "type": "session_meta",
                "payload": { "id": session_id, "cwd": cwd }
            })
        )
        .expect("write metadata");
        writeln!(
            file,
            "{}",
            serde_json::json!({
                "type": "event_msg",
                "payload": {
                    "info": {
                        "total_token_usage": {
                            "input_tokens": input,
                            "cached_input_tokens": cached,
                            "output_tokens": output,
                            "reasoning_output_tokens": reasoning,
                            "total_tokens": total
                        }
                    }
                }
            })
        )
        .expect("write usage");
    }

    fn temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let dir = env::temp_dir().join(format!("vibehub-{prefix}-{nanos}"));
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }
}

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    Timeout,
    Connect,
    RateLimit,
    Auth,
    NotFound,
    Upstream5xx,
    Other,
}

#[derive(Debug, Clone)]
pub struct Circuit {
    pub consecutive_failures: u32,
    pub open_until: u64,
    pub probe_in_flight: bool,
    pub ewma_latency_ms: f64,
    pub last_failure_at: Option<u64>,
    pub last_failure_kind: Option<FailureKind>,
}

impl Default for Circuit {
    fn default() -> Self {
        Self {
            consecutive_failures: 0,
            open_until: 0,
            probe_in_flight: false,
            ewma_latency_ms: 0.0,
            last_failure_at: None,
            last_failure_kind: None,
        }
    }
}

impl Circuit {
    pub fn is_open(&self, now: u64) -> bool {
        self.open_until > now
    }

    pub fn is_half_open(&self, now: u64) -> bool {
        self.open_until != 0 && self.open_until <= now
    }

    pub fn can_attempt(&self, now: u64) -> bool {
        if self.is_open(now) {
            return false;
        }
        if self.is_half_open(now) {
            return !self.probe_in_flight;
        }
        true
    }

    pub fn mark_probe_started(&mut self, now: u64) {
        if self.is_half_open(now) {
            self.probe_in_flight = true;
        }
    }

    pub fn on_success(&mut self, latency_ms: u64) {
        self.consecutive_failures = 0;
        self.open_until = 0;
        self.probe_in_flight = false;
        self.last_failure_at = None;
        self.last_failure_kind = None;

        // EWMA latency (alpha = 0.2)
        let sample = latency_ms as f64;
        if self.ewma_latency_ms <= 0.0 {
            self.ewma_latency_ms = sample;
        } else {
            self.ewma_latency_ms = (self.ewma_latency_ms * 0.8) + (sample * 0.2);
        }
    }

    pub fn on_failure(
        &mut self,
        now: u64,
        base_cooldown_seconds: u64,
        kind: FailureKind,
        retry_after_seconds: Option<u64>,
        jitter_seed: &impl Hash,
    ) -> u64 {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        self.last_failure_at = Some(now);
        self.last_failure_kind = Some(kind);
        self.probe_in_flight = false;

        let base = base_cooldown_for_kind(base_cooldown_seconds, kind, retry_after_seconds);
        let multiplier = backoff_multiplier(self.consecutive_failures);
        let mut cooldown = base.saturating_mul(multiplier);
        cooldown = cooldown.clamp(1, 60 * 60); // 1s .. 1h

        let jitter = jitter_seconds(cooldown, jitter_seed);
        let until = now.saturating_add(cooldown).saturating_add(jitter);
        self.open_until = until;
        until
    }

    pub fn score(&self, provider_weight: u32, now: u64) -> f64 {
        if self.is_open(now) {
            return -1.0;
        }

        let weight = provider_weight.max(1) as f64;
        let health_factor = 1.0 / (1.0 + self.consecutive_failures as f64);
        let latency_factor = if self.ewma_latency_ms > 0.0 {
            1000.0 / (1000.0 + self.ewma_latency_ms)
        } else {
            1.0
        };

        weight * health_factor * latency_factor
    }
}

fn backoff_multiplier(consecutive_failures: u32) -> u64 {
    // 1, 2, 4, 8, 16, 16, ...
    let exp = consecutive_failures.saturating_sub(1).min(4);
    1u64 << exp
}

fn base_cooldown_for_kind(base: u64, kind: FailureKind, retry_after: Option<u64>) -> u64 {
    let base = base.max(1);
    // For flaky/public endpoints, a single transient should not freeze a provider for too long.
    // Use a smaller base for network-ish failures and then rely on exponential backoff.
    let quick_base = (base / 12).max(2);
    match kind {
        FailureKind::RateLimit => retry_after.unwrap_or(base).max(base),
        FailureKind::Auth => base.saturating_mul(10).max(600),
        FailureKind::NotFound => quick_base.saturating_mul(3),
        FailureKind::Timeout
        | FailureKind::Connect
        | FailureKind::Upstream5xx
        | FailureKind::Other => quick_base,
    }
}

fn jitter_seconds(cooldown: u64, seed: &impl Hash) -> u64 {
    // Up to 25% jitter.
    let max_jitter = (cooldown / 4).max(1);
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    hasher.finish() % (max_jitter + 1)
}

use crate::vibehub::events::StoredEvent;
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serde_yaml::{Mapping, Number, Value as YamlValue};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FitnessMetrics {
    pub tasks: TaskMetrics,
    pub capabilities: CapabilityMetrics,
    pub sync: SyncMetrics,
    pub pack: PackMetrics,
    pub schema: SchemaMetrics,
    pub events: EventMetrics,
    pub subagent: SubAgentMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskMetrics {
    pub active_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityMetrics {
    pub active_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncMetrics {
    pub avg_duration_ms: f64,
    pub last_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PackMetrics {
    pub avg_size_tokens: f64,
    pub oversize_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchemaMetrics {
    pub validation_failure_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventMetrics {
    pub write_per_minute: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubAgentMetrics {
    pub timeout_count: u64,
}

impl Default for FitnessMetrics {
    fn default() -> Self {
        Self {
            tasks: TaskMetrics { active_count: 0 },
            capabilities: CapabilityMetrics { active_count: 0 },
            sync: SyncMetrics {
                avg_duration_ms: 0.0,
                last_mode: None,
            },
            pack: PackMetrics {
                avg_size_tokens: 0.0,
                oversize_count: 0,
            },
            schema: SchemaMetrics {
                validation_failure_rate: 0.0,
            },
            events: EventMetrics {
                write_per_minute: 0.0,
            },
            subagent: SubAgentMetrics { timeout_count: 0 },
        }
    }
}

pub fn compute_metrics(state: &YamlValue, events: &[StoredEvent]) -> FitnessMetrics {
    let mut metrics = FitnessMetrics::default();
    metrics.tasks.active_count = yaml_string_sequence(state, &["tasks", "active"])
        .map(|tasks| tasks.len() as u64)
        .unwrap_or_else(|| {
            if yaml_string(state, &["current", "task_id"]).is_some() {
                1
            } else {
                0
            }
        });

    let mut active_capabilities = BTreeSet::new();
    let mut pack_sizes = Vec::new();
    let mut schema_failures = 0_u64;
    let mut schema_successes = 0_u64;
    let mut sync_started: BTreeMap<String, (i64, String)> = BTreeMap::new();
    let mut sync_durations = Vec::new();
    let mut first_ts: Option<i64> = None;
    let mut last_ts: Option<i64> = None;

    for event in events {
        let timestamp = event
            .event
            .get("timestamp")
            .and_then(JsonValue::as_str)
            .and_then(parse_timestamp_ms);
        if let Some(ts) = timestamp {
            first_ts = Some(first_ts.map_or(ts, |existing| existing.min(ts)));
            last_ts = Some(last_ts.map_or(ts, |existing| existing.max(ts)));
        }
        let Some(event_type) = event.event.get("event_type").and_then(JsonValue::as_str) else {
            continue;
        };
        let payload = event.event.get("payload").unwrap_or(&JsonValue::Null);
        match event_type {
            "CapabilityClaimed" => {
                if let Some(capability) = payload.get("capability").and_then(JsonValue::as_str) {
                    active_capabilities.insert(capability.to_string());
                }
            }
            "CapabilityReleased" => {
                if let Some(capability) = payload.get("capability").and_then(JsonValue::as_str) {
                    active_capabilities.remove(capability);
                    schema_successes += 1;
                }
            }
            "CapabilityPackBuilt" => {
                if let Some(size) = payload.get("size_tokens").and_then(JsonValue::as_u64) {
                    pack_sizes.push(size as f64);
                }
            }
            "PackOversize" => metrics.pack.oversize_count += 1,
            "SchemaValidationFailed" => schema_failures += 1,
            "SyncStarted" => {
                if let (Some(event_id), Some(ts)) = (event.event_id.as_deref(), timestamp) {
                    let mode = payload
                        .get("mode")
                        .and_then(JsonValue::as_str)
                        .unwrap_or("unknown")
                        .to_string();
                    sync_started.insert(event_id.to_string(), (ts, mode.clone()));
                    metrics.sync.last_mode = Some(mode);
                }
            }
            "SyncCompleted" => {
                let mode = payload
                    .get("mode")
                    .and_then(JsonValue::as_str)
                    .unwrap_or("unknown")
                    .to_string();
                metrics.sync.last_mode = Some(mode.clone());
                if let Some(ts) = timestamp {
                    if let Some((_, (started_ts, _))) = sync_started.iter().next_back() {
                        if ts >= *started_ts {
                            sync_durations.push((ts - *started_ts) as f64);
                        }
                    }
                }
            }
            "Legacy" => {
                let legacy_type = payload
                    .get("legacy_event_type")
                    .and_then(JsonValue::as_str)
                    .unwrap_or_default();
                let summary = payload
                    .get("summary")
                    .and_then(JsonValue::as_str)
                    .unwrap_or_default();
                if legacy_type.contains("subagent_timeout") || summary.contains("sub-agent timeout")
                {
                    metrics.subagent.timeout_count += 1;
                }
            }
            _ => {}
        }
    }

    metrics.capabilities.active_count = active_capabilities.len() as u64;
    metrics.pack.avg_size_tokens = average(&pack_sizes);
    metrics.sync.avg_duration_ms = average(&sync_durations);
    let total_schema = schema_failures + schema_successes;
    metrics.schema.validation_failure_rate = if total_schema == 0 {
        0.0
    } else {
        schema_failures as f64 / total_schema as f64
    };
    if let (Some(first), Some(last)) = (first_ts, last_ts) {
        let minutes = ((last - first).max(1) as f64) / 60_000.0;
        metrics.events.write_per_minute = events.len() as f64 / minutes;
    }
    metrics
}

pub fn metrics_to_yaml(metrics: &FitnessMetrics) -> YamlValue {
    let mut root = Mapping::new();
    insert_map(
        &mut root,
        "tasks",
        &[("active_count", number(metrics.tasks.active_count))],
    );
    insert_map(
        &mut root,
        "capabilities",
        &[("active_count", number(metrics.capabilities.active_count))],
    );
    let mut sync = Mapping::new();
    sync.insert(
        YamlValue::String("avg_duration_ms".to_string()),
        f64_yaml(metrics.sync.avg_duration_ms),
    );
    sync.insert(
        YamlValue::String("last_mode".to_string()),
        metrics
            .sync
            .last_mode
            .clone()
            .map(YamlValue::String)
            .unwrap_or(YamlValue::Null),
    );
    root.insert(
        YamlValue::String("sync".to_string()),
        YamlValue::Mapping(sync),
    );
    let mut pack = Mapping::new();
    pack.insert(
        YamlValue::String("avg_size_tokens".to_string()),
        f64_yaml(metrics.pack.avg_size_tokens),
    );
    pack.insert(
        YamlValue::String("oversize_count".to_string()),
        number(metrics.pack.oversize_count),
    );
    root.insert(
        YamlValue::String("pack".to_string()),
        YamlValue::Mapping(pack),
    );
    let mut schema = Mapping::new();
    schema.insert(
        YamlValue::String("validation_failure_rate".to_string()),
        f64_yaml(metrics.schema.validation_failure_rate),
    );
    root.insert(
        YamlValue::String("schema".to_string()),
        YamlValue::Mapping(schema),
    );
    let mut events = Mapping::new();
    events.insert(
        YamlValue::String("write_per_minute".to_string()),
        f64_yaml(metrics.events.write_per_minute),
    );
    root.insert(
        YamlValue::String("events".to_string()),
        YamlValue::Mapping(events),
    );
    insert_map(
        &mut root,
        "subagent",
        &[("timeout_count", number(metrics.subagent.timeout_count))],
    );
    YamlValue::Mapping(root)
}

pub fn default_metrics_yaml() -> YamlValue {
    metrics_to_yaml(&FitnessMetrics::default())
}

fn insert_map(root: &mut Mapping, key: &str, entries: &[(&str, YamlValue)]) {
    let mut map = Mapping::new();
    for (entry_key, value) in entries {
        map.insert(YamlValue::String((*entry_key).to_string()), value.clone());
    }
    root.insert(YamlValue::String(key.to_string()), YamlValue::Mapping(map));
}

fn yaml_string(value: &YamlValue, keys: &[&str]) -> Option<String> {
    let mut current = value;
    for key in keys {
        current = current.get(*key)?;
    }
    current
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn yaml_string_sequence(value: &YamlValue, keys: &[&str]) -> Option<Vec<String>> {
    let mut current = value;
    for key in keys {
        current = current.get(*key)?;
    }
    current.as_sequence().map(|sequence| {
        sequence
            .iter()
            .filter_map(YamlValue::as_str)
            .map(ToString::to_string)
            .collect()
    })
}

fn parse_timestamp_ms(value: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|timestamp| timestamp.timestamp_millis())
}

fn average(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn number(value: u64) -> YamlValue {
    YamlValue::Number(Number::from(value))
}

fn f64_yaml(value: f64) -> YamlValue {
    serde_yaml::to_value(value).unwrap_or(YamlValue::Number(Number::from(0)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn stored(
        event_id: &str,
        timestamp: &str,
        event_type: &str,
        payload: JsonValue,
    ) -> StoredEvent {
        StoredEvent {
            event_id: Some(event_id.to_string()),
            event: json!({
                "event_id": event_id,
                "timestamp": timestamp,
                "event_type": event_type,
                "payload": payload,
            }),
        }
    }

    #[test]
    fn computes_all_m5_metric_buckets() {
        let state: YamlValue = serde_yaml::from_str("current:\n  task_id: T-001\n").unwrap();
        let events = vec![
            stored(
                "evt-1",
                "2026-05-28T00:00:00Z",
                "CapabilityClaimed",
                json!({"capability":"implement"}),
            ),
            stored(
                "evt-2",
                "2026-05-28T00:00:30Z",
                "CapabilityPackBuilt",
                json!({"size_tokens":100}),
            ),
            stored("evt-3", "2026-05-28T00:01:00Z", "PackOversize", json!({})),
            stored(
                "evt-4",
                "2026-05-28T00:01:30Z",
                "SchemaValidationFailed",
                json!({}),
            ),
        ];

        let metrics = compute_metrics(&state, &events);

        assert_eq!(metrics.tasks.active_count, 1);
        assert_eq!(metrics.capabilities.active_count, 1);
        assert_eq!(metrics.pack.avg_size_tokens, 100.0);
        assert_eq!(metrics.pack.oversize_count, 1);
        assert_eq!(metrics.schema.validation_failure_rate, 1.0);
        assert!(metrics.events.write_per_minute > 0.0);
        let yaml = metrics_to_yaml(&metrics);
        assert!(yaml.get("subagent").is_some());
    }
}

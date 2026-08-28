use serde::Serialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Instant;
use uuid::Uuid;
use vibehub_core::v3::{
    projection, AppendResult, EventDraft, EvidenceGrade, NodeId, ProjectId, SessionId, TaskId,
    V3EventEnvelope, V3EventStore,
};

const PROJECT_ID: &str = "project.bench";
const TASK_COUNT: usize = 1_000;

#[derive(Debug, Serialize)]
struct Sample {
    event_count: usize,
    task_count: usize,
    fixture_bytes: u64,
    load_ms: f64,
    indexed_reload_ms: f64,
    fold_ms: f64,
    steady_append_runs: usize,
    steady_append_p50_ms: f64,
    steady_append_p95_ms: f64,
    steady_append_max_ms: f64,
    compatibility_projection_rewritten: bool,
    projection_bytes: usize,
    aggregate_count: usize,
    projected_task_count: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let counts = std::env::args()
        .skip(1)
        .map(|value| value.parse::<usize>())
        .collect::<Result<Vec<_>, _>>()?;
    let counts = if counts.is_empty() {
        vec![10_000, 50_000]
    } else {
        counts
    };
    if counts.iter().any(|count| *count < TASK_COUNT) {
        return Err(format!("every event count must be at least {TASK_COUNT}").into());
    }

    let mut samples = Vec::new();
    for event_count in counts {
        samples.push(run_sample(event_count)?);
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema_version": "1.0",
            "fixture": "deterministic-v3-store-scale-v1",
            "build_profile": if cfg!(debug_assertions) { "debug" } else { "release" },
            "task_count": TASK_COUNT,
            "samples": samples,
        }))?
    );
    Ok(())
}

fn run_sample(event_count: usize) -> Result<Sample, Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!(
        "vibehub-v3-store-bench-{}-{}",
        std::process::id(),
        Uuid::new_v4()
    ));
    let project_store = root.join(".vibehub/v3/projects").join(PROJECT_ID);
    fs::create_dir_all(&project_store)?;
    let events_path = project_store.join("events.jsonl");
    write_fixture(&events_path, event_count)?;
    let fixture_bytes = fs::metadata(&events_path)?.len();

    let store = V3EventStore::open(&root)?;
    let started = Instant::now();
    let events = store.load_project(PROJECT_ID)?;
    let load_ms = started.elapsed().as_secs_f64() * 1_000.0;
    assert_eq!(events.len(), event_count);

    let started = Instant::now();
    let projection = projection::fold(PROJECT_ID, &events);
    let fold_ms = started.elapsed().as_secs_f64() * 1_000.0;
    let projection_bytes = serde_json::to_vec(&projection)?.len();

    let started = Instant::now();
    let indexed_events = store.load_project(PROJECT_ID)?;
    let indexed_reload_ms = started.elapsed().as_secs_f64() * 1_000.0;
    assert_eq!(indexed_events, events);

    let append_runs = 200usize;
    let mut append_latencies = Vec::with_capacity(append_runs);
    for run in 0..append_runs {
        let task_number = run % TASK_COUNT;
        let task_id = format!("task.bench.{task_number:04}");
        let session_id = format!("session.bench.{task_number:04}");
        let expected_version = store.aggregate_version(PROJECT_ID, &session_id)?;
        let started = Instant::now();
        let result = store.append_with_rebuild(EventDraft {
            event_type: "progress.logged".to_owned(),
            aggregate_id: session_id.clone(),
            expected_version,
            idempotency_key: format!("bench.steady.{event_count}.{run}"),
            project_id: ProjectId(PROJECT_ID.to_owned()),
            task_id: TaskId(task_id),
            node_id: Some(NodeId(format!("node.bench.{task_number:04}"))),
            session_id: Some(SessionId(session_id)),
            worktree_id: None,
            lease_id: None,
            operation_id: None,
            actor: "benchmark".to_owned(),
            evidence_grade: EvidenceGrade::HardObserved,
            occurred_at: None,
            commit_sha: None,
            payload: json!({"fixture": "deterministic-v3-store-scale-v1", "steady_run": run}),
        })?;
        assert!(matches!(result, AppendResult::Appended { .. }));
        append_latencies.push(started.elapsed().as_secs_f64() * 1_000.0);
    }
    append_latencies.sort_by(f64::total_cmp);
    let sample = Sample {
        event_count,
        task_count: TASK_COUNT,
        fixture_bytes,
        load_ms,
        indexed_reload_ms,
        fold_ms,
        steady_append_runs: append_runs,
        steady_append_p50_ms: percentile(&append_latencies, 0.50),
        steady_append_p95_ms: percentile(&append_latencies, 0.95),
        steady_append_max_ms: append_latencies.last().copied().unwrap_or_default(),
        compatibility_projection_rewritten: store.projection_path(PROJECT_ID).exists(),
        projection_bytes,
        aggregate_count: projection.aggregate_versions.len(),
        projected_task_count: projection.tasks.len(),
    };

    fs::remove_dir_all(&root)?;
    Ok(sample)
}

fn percentile(values: &[f64], quantile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let index = ((values.len() - 1) as f64 * quantile).ceil() as usize;
    values[index.min(values.len() - 1)]
}

fn write_fixture(path: &Path, event_count: usize) -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = BufWriter::new(File::create(path)?);
    let mut aggregate_versions = BTreeMap::<String, u64>::new();
    for index in 0..event_count {
        let task_number = index % TASK_COUNT;
        let task_id = format!("task.bench.{task_number:04}");
        let (event_type, aggregate_id, session_id, node_id, payload) = if index < TASK_COUNT {
            (
                "task.created",
                task_id.clone(),
                None,
                None,
                json!({
                    "workflow_profile": "standard",
                    "fixture": "deterministic-v3-store-scale-v1"
                }),
            )
        } else {
            let session_id = format!("session.bench.{task_number:04}");
            (
                "progress.logged",
                session_id.clone(),
                Some(SessionId(session_id)),
                Some(NodeId(format!("node.bench.{task_number:04}"))),
                json!({
                    "details": {
                        "fixture": "deterministic-v3-store-scale-v1",
                        "sequence": index
                    }
                }),
            )
        };
        let aggregate_version = aggregate_versions
            .entry(aggregate_id.clone())
            .and_modify(|version| *version += 1)
            .or_insert(1);
        let event = V3EventEnvelope {
            event_id: format!("evt.bench.{index:08}"),
            event_type: event_type.to_owned(),
            event_version: "1.0".to_owned(),
            aggregate_id,
            aggregate_version: *aggregate_version,
            expected_version: aggregate_version.saturating_sub(1),
            idempotency_key: format!("bench.key.{index:08}"),
            project_id: ProjectId(PROJECT_ID.to_owned()),
            task_id: TaskId(task_id),
            node_id,
            session_id,
            worktree_id: None,
            lease_id: None,
            operation_id: None,
            actor: "benchmark".to_owned(),
            evidence_grade: EvidenceGrade::HardObserved,
            occurred_at: "2026-08-23T00:00:00.000Z".to_owned(),
            recorded_at: "2026-08-23T00:00:00.000Z".to_owned(),
            commit_sha: None,
            payload,
        };
        serde_json::to_writer(&mut writer, &event)?;
        writer.write_all(b"\n")?;
    }
    writer.flush()?;
    Ok(())
}

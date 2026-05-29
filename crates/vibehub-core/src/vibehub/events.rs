use crate::vibehub::util::{
    canonical_initialized_project_root, normalize_path, relative_to_project,
};
use crate::vibehub::{current, policy};
use anyhow::{anyhow, bail, Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "event_type", content = "payload")]
pub enum VibehubEvent {
    TaskCreated {
        intent: String,
        mode: String,
    },
    TaskIntakePlanned {
        source_message: Option<String>,
        split_confidence: String,
        split_reason: Option<String>,
        not_split_reason: Option<String>,
        task_ids: Vec<String>,
        execution_order: Vec<String>,
    },
    TaskCancelled {
        reason: String,
    },
    TaskSwitched {
        from_task_id: Option<String>,
        from_run_id: Option<String>,
        to_task_id: String,
        to_run_id: String,
    },
    CapabilityClaimed {
        capability: String,
    },
    CapabilityReleased {
        capability: String,
        outcome: String,
        task_pack_dirty: bool,
    },
    CapabilityPackBuilt {
        capability: String,
        pack_path: String,
        size_tokens: u64,
    },
    TaskPackRebuilt {
        reason: String,
        delta_fields: Vec<String>,
    },
    PackOversize {
        kind: String,
        size: u64,
        threshold: u64,
    },
    EvidenceAdded {
        source: String,
        summary: String,
        refs: Vec<String>,
    },
    PlanDrafted {
        plan_path: String,
        scope: String,
    },
    DiffObserved {
        commit_range: String,
        files: Vec<String>,
    },
    FileOwnershipUpdated {
        task_id: String,
        files_added: Vec<String>,
        files_removed: Vec<String>,
    },
    NeighborConflictDetected {
        tasks: Vec<String>,
        shared_files: Vec<String>,
    },
    ValidationRun {
        kind: String,
        status: String,
        output_ref: String,
    },
    RiskRaised {
        id: String,
        severity: String,
        note: String,
    },
    RiskResolved {
        id: String,
        severity: Option<String>,
        note: Option<String>,
        resolution: Option<String>,
    },
    HandoffWritten {
        path: String,
        capability: Option<String>,
    },
    GateChecked {
        gate: String,
        result: bool,
        reasons: Vec<String>,
    },
    PhaseProjected {
        phase: String,
        derived_from: Vec<String>,
    },
    SyncStarted {
        mode: String,
        signals: Value,
        report_path: Option<String>,
    },
    SyncCompleted {
        mode: String,
        signals: Value,
        report_path: String,
    },
    SyncReport {
        report_path: String,
        status: String,
    },
    SchemaValidationFailed {
        target: String,
        errors: Vec<Value>,
    },
    EventLogCorrupted {
        at_offset: u64,
        recovered_to: String,
    },
    LoopWarning {
        capability: String,
        events_per_minute: u32,
        threshold: u32,
    },
    PlanInvalidated {
        reason: String,
    },
    DiffReverted {
        reason: String,
    },
    Legacy {
        legacy_event_type: String,
        summary: String,
        evidence_grade: String,
        details: Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventEnvelope {
    pub event_id: String,
    pub timestamp: String,
    pub task_id: String,
    pub run_id: String,
    pub actor: String,
    pub schema_version: String,
    #[serde(flatten)]
    pub event: VibehubEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoredEvent {
    pub event_id: Option<String>,
    pub event: Value,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RunEventAppendResult {
    pub events_path: String,
    pub event_id: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PendingEventWriteResult {
    pub pending_path: String,
    pub event_id: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PendingReplayResult {
    pub pending_dir: String,
    pub replayed: usize,
    pub skipped: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct PendingEventWriteFile {
    schema_version: String,
    kind: String,
    created_at: String,
    source: String,
    envelope: EventEnvelope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventIdGenerator {
    last_unix_ms: i64,
    seq_within_ms: u16,
}

impl EventIdGenerator {
    pub fn new() -> Self {
        Self {
            last_unix_ms: 0,
            seq_within_ms: 0,
        }
    }

    pub fn next(&mut self) -> String {
        self.next_at(Utc::now().timestamp_millis())
    }

    pub fn next_at(&mut self, unix_ms: i64) -> String {
        let unix_ms = unix_ms.max(self.last_unix_ms);
        if unix_ms == self.last_unix_ms {
            self.seq_within_ms = self.seq_within_ms.saturating_add(1);
        } else {
            self.last_unix_ms = unix_ms;
            self.seq_within_ms = 1;
        }
        format!("evt-{}-{:04}", self.last_unix_ms, self.seq_within_ms)
    }
}

impl Default for EventIdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Default)]
pub struct EventWriter {
    write_lock: Mutex<()>,
    id_generator: Mutex<EventIdGenerator>,
}

impl EventWriter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(
        &self,
        project_root: impl AsRef<Path>,
        task_id: &str,
        run_id: &str,
        actor: &str,
        event: VibehubEvent,
    ) -> Result<RunEventAppendResult> {
        let project_root = canonical_initialized_project_root(project_root.as_ref())?;
        let events_path = run_events_path(&project_root, task_id, run_id);
        if let Some(parent) = events_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create {}", parent.display()))?;
        }

        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| anyhow!("Event writer lock poisoned"))?;
        if let Some(recovery) = recover_existing_log(&project_root, &events_path, task_id, run_id)
            .context("Failed to recover corrupted event log")?
        {
            let event_id = self
                .id_generator
                .lock()
                .map_err(|_| anyhow!("Event id generator lock poisoned"))?
                .next();
            let envelope = EventEnvelope {
                event_id,
                timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
                task_id: task_id.to_string(),
                run_id: run_id.to_string(),
                actor: "vibehub".to_string(),
                schema_version: "1.0".to_string(),
                event: VibehubEvent::EventLogCorrupted {
                    at_offset: recovery.at_offset,
                    recovered_to: recovery.recovered_to,
                },
            };
            let offset = append_envelope(&events_path, &envelope)?;
            update_task_event_index(&project_root, task_id, run_id, &envelope.event_id, offset)?;
        }

        let event_id = self
            .id_generator
            .lock()
            .map_err(|_| anyhow!("Event id generator lock poisoned"))?
            .next();
        let envelope = EventEnvelope {
            event_id: event_id.clone(),
            timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            task_id: task_id.to_string(),
            run_id: run_id.to_string(),
            actor: actor.to_string(),
            schema_version: "1.0".to_string(),
            event,
        };

        let offset = append_envelope(&events_path, &envelope)?;
        update_task_event_index(&project_root, task_id, run_id, &envelope.event_id, offset)?;
        if should_detect_loop(&envelope.event) {
            let mut id_generator = self
                .id_generator
                .lock()
                .map_err(|_| anyhow!("Event id generator lock poisoned"))?;
            maybe_append_loop_warning(
                &project_root,
                &events_path,
                task_id,
                run_id,
                &mut id_generator,
            )?;
        }

        Ok(RunEventAppendResult {
            events_path: normalize_path(&relative_to_project(&project_root, &events_path)?),
            event_id,
        })
    }

    pub fn write_pending(
        &self,
        project_root: impl AsRef<Path>,
        task_id: &str,
        run_id: &str,
        actor: &str,
        event: VibehubEvent,
    ) -> Result<PendingEventWriteResult> {
        let project_root = canonical_initialized_project_root(project_root.as_ref())?;
        let created_at = Utc::now();
        let event_id = self
            .id_generator
            .lock()
            .map_err(|_| anyhow!("Event id generator lock poisoned"))?
            .next_at(created_at.timestamp_millis());
        let envelope = EventEnvelope {
            event_id: event_id.clone(),
            timestamp: created_at.to_rfc3339_opts(SecondsFormat::Millis, true),
            task_id: task_id.to_string(),
            run_id: run_id.to_string(),
            actor: actor.to_string(),
            schema_version: "1.0".to_string(),
            event,
        };
        write_pending_envelope(&project_root, envelope, "agent_fallback")
    }

    pub fn replay_pending(&self, project_root: impl AsRef<Path>) -> Result<PendingReplayResult> {
        let project_root = canonical_initialized_project_root(project_root.as_ref())?;
        let pending_dir = pending_events_dir(&project_root);
        let pending_dir_display =
            normalize_path(&relative_to_project(&project_root, &pending_dir)?);
        if !pending_dir.exists() {
            return Ok(PendingReplayResult {
                pending_dir: pending_dir_display,
                replayed: 0,
                skipped: 0,
                warnings: Vec::new(),
            });
        }

        let mut pending_files = fs::read_dir(&pending_dir)
            .with_context(|| format!("Failed to read {}", pending_dir.display()))?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("write"))
            .collect::<Vec<_>>();
        pending_files.sort_by(|left, right| left.file_name().cmp(&right.file_name()));

        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| anyhow!("Event writer lock poisoned"))?;
        let mut replayed = 0;
        let mut skipped = 0;
        let mut warnings = Vec::new();
        let mut loop_check_runs: BTreeMap<(String, String), PathBuf> = BTreeMap::new();

        for pending_path in pending_files {
            match replay_pending_file_locked(&project_root, &pending_path) {
                Ok(PendingReplayAction::Replayed {
                    task_id,
                    run_id,
                    events_path,
                    loop_sensitive,
                }) => {
                    replayed += 1;
                    if loop_sensitive {
                        loop_check_runs.insert((task_id, run_id), events_path);
                    }
                    remove_pending_file(&pending_path, &mut warnings);
                }
                Ok(PendingReplayAction::Skipped { warning }) => {
                    skipped += 1;
                    warnings.push(warning);
                    remove_pending_file(&pending_path, &mut warnings);
                }
                Err(error) => {
                    warnings.push(format!(
                        "Pending event {} could not be replayed: {error}",
                        normalize_path(&relative_to_project(&project_root, &pending_path)?)
                    ));
                }
            }
        }

        if !loop_check_runs.is_empty() {
            let mut id_generator = self
                .id_generator
                .lock()
                .map_err(|_| anyhow!("Event id generator lock poisoned"))?;
            for ((task_id, run_id), events_path) in loop_check_runs {
                maybe_append_loop_warning(
                    &project_root,
                    &events_path,
                    &task_id,
                    &run_id,
                    &mut id_generator,
                )?;
            }
        }

        if fs::read_dir(&pending_dir)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false)
        {
            let _ = fs::remove_dir(&pending_dir);
        }

        Ok(PendingReplayResult {
            pending_dir: pending_dir_display,
            replayed,
            skipped,
            warnings,
        })
    }

    pub fn list_events(
        &self,
        project_root: impl AsRef<Path>,
        task_id: &str,
        run_id: &str,
        since: Option<&str>,
    ) -> Result<Vec<StoredEvent>> {
        let project_root = canonical_initialized_project_root(project_root.as_ref())?;
        let events_path = run_events_path(&project_root, task_id, run_id);
        list_events_from_path(&events_path, since)
    }
}

fn append_envelope(events_path: &Path, envelope: &EventEnvelope) -> Result<u64> {
    let offset = events_path
        .metadata()
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(events_path)
        .with_context(|| format!("Failed to open {}", events_path.display()))?;
    writeln!(
        file,
        "{}",
        serde_json::to_string(envelope).context("Failed to serialize VibeHub event")?
    )
    .with_context(|| format!("Failed to write {}", events_path.display()))?;
    Ok(offset)
}

pub fn global_event_writer() -> &'static EventWriter {
    static WRITER: OnceLock<EventWriter> = OnceLock::new();
    WRITER.get_or_init(EventWriter::new)
}

pub fn append_run_event(
    project_root: impl AsRef<Path>,
    task_id: &str,
    run_id: &str,
    event_type: &str,
    summary: &str,
    details: Value,
) -> Result<RunEventAppendResult> {
    global_event_writer().append(
        project_root,
        task_id,
        run_id,
        "vibehub",
        VibehubEvent::Legacy {
            legacy_event_type: event_type.to_string(),
            summary: summary.to_string(),
            evidence_grade: "hard_observed".to_string(),
            details,
        },
    )
}

pub fn append_structured_run_event(
    project_root: impl AsRef<Path>,
    task_id: &str,
    run_id: &str,
    event: VibehubEvent,
) -> Result<RunEventAppendResult> {
    global_event_writer().append(project_root, task_id, run_id, "vibehub", event)
}

pub fn write_pending_structured_run_event(
    project_root: impl AsRef<Path>,
    task_id: &str,
    run_id: &str,
    actor: &str,
    event: VibehubEvent,
) -> Result<PendingEventWriteResult> {
    global_event_writer().write_pending(project_root, task_id, run_id, actor, event)
}

pub fn write_pending_current_structured_event(
    project_root: impl AsRef<Path>,
    actor: &str,
    event: VibehubEvent,
) -> Result<Option<PendingEventWriteResult>> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let task = match current::resolve_current_task(&project_root) {
        Ok(task) => task,
        Err(_) => return Ok(None),
    };
    let run = match current::resolve_current_run(&project_root, &task.task_id) {
        Ok(run) => run,
        Err(_) => return Ok(None),
    };
    write_pending_structured_run_event(&project_root, &task.task_id, &run.run_id, actor, event)
        .map(Some)
}

pub fn replay_pending_events(project_root: impl AsRef<Path>) -> Result<PendingReplayResult> {
    global_event_writer().replay_pending(project_root)
}

pub fn append_current_run_event(
    project_root: impl AsRef<Path>,
    event_type: &str,
    summary: &str,
    details: Value,
) -> Result<Option<RunEventAppendResult>> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let task = match current::resolve_current_task(&project_root) {
        Ok(task) => task,
        Err(_) => return Ok(None),
    };
    let run = match current::resolve_current_run(&project_root, &task.task_id) {
        Ok(run) => run,
        Err(_) => return Ok(None),
    };
    append_run_event(
        &project_root,
        &task.task_id,
        &run.run_id,
        event_type,
        summary,
        details,
    )
    .map(Some)
}

pub fn append_current_structured_event(
    project_root: impl AsRef<Path>,
    event: VibehubEvent,
) -> Result<Option<RunEventAppendResult>> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let task = match current::resolve_current_task(&project_root) {
        Ok(task) => task,
        Err(_) => return Ok(None),
    };
    let run = match current::resolve_current_run(&project_root, &task.task_id) {
        Ok(run) => run,
        Err(_) => return Ok(None),
    };
    append_structured_run_event(&project_root, &task.task_id, &run.run_id, event).map(Some)
}

pub fn list_events(
    project_root: impl AsRef<Path>,
    task_id: &str,
    run_id: &str,
    since: Option<&str>,
) -> Result<Vec<StoredEvent>> {
    global_event_writer().list_events(project_root, task_id, run_id, since)
}

pub fn ensure_consistency(project_root: impl AsRef<Path>) -> Result<Vec<String>> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let task = match current::resolve_current_task(&project_root) {
        Ok(task) => task,
        Err(_) => return Ok(Vec::new()),
    };
    let run = match current::resolve_current_run(&project_root, &task.task_id) {
        Ok(run) => run,
        Err(_) => return Ok(Vec::new()),
    };
    let state_path = project_root.join(".vibehub/state.yaml");
    if !state_path.is_file() {
        return Ok(Vec::new());
    }
    let state = fs::read_to_string(&state_path)
        .with_context(|| format!("Failed to read {}", state_path.display()))
        .and_then(|content| {
            serde_yaml::from_str::<serde_yaml::Value>(&content)
                .with_context(|| format!("Invalid YAML in {}", state_path.display()))
        })?;
    let projection = fold_event_state(list_events(
        &project_root,
        &task.task_id,
        &run.run_id,
        None,
    )?);
    let mut warnings = Vec::new();
    if let (Some(state_phase), Some(projected_phase)) = (
        yaml_path_string(&state, &["current", "phase"]),
        projection.current_phase.as_deref(),
    ) {
        if state_phase != projected_phase {
            warnings.push(format!(
                "Event/state consistency warning: state current.phase is '{state_phase}' but latest PhaseProjected event is '{projected_phase}'."
            ));
        }
    }
    for (phase, projected_status) in &projection.phase_statuses {
        let state_status = yaml_path_string(&state, &["flow", phase]);
        if let Some(state_status) = state_status {
            if state_status != *projected_status {
                warnings.push(format!(
                    "Event/state consistency warning: state flow.{phase} is '{state_status}' but event projection is '{projected_status}'."
                ));
            }
        }
    }
    if let (Some(projected_phase), Some(state_phase_status)) = (
        projection.current_phase.as_deref(),
        yaml_path_string(&state, &["current", "phase_status"]),
    ) {
        if let Some(projected_status) = projection.phase_statuses.get(projected_phase) {
            if state_phase_status != *projected_status {
                warnings.push(format!(
                    "Event/state consistency warning: state current.phase_status is '{state_phase_status}' but event projection for {projected_phase} is '{projected_status}'."
                ));
            }
        }
    }
    Ok(warnings)
}

#[derive(Debug, Default, PartialEq, Eq)]
struct EventStateProjection {
    current_phase: Option<String>,
    phase_statuses: BTreeMap<String, String>,
}

fn fold_event_state(events: Vec<StoredEvent>) -> EventStateProjection {
    let mut projection = EventStateProjection::default();
    for stored in events {
        let Some(event_type) = stored.event.get("event_type").and_then(Value::as_str) else {
            continue;
        };
        let payload = stored.event.get("payload").unwrap_or(&Value::Null);
        match event_type {
            "CapabilityClaimed" => {
                if let Some(capability) = payload.get("capability").and_then(Value::as_str) {
                    projection
                        .phase_statuses
                        .insert(capability.to_string(), "active".to_string());
                }
            }
            "CapabilityReleased" => {
                if let (Some(capability), Some(outcome)) = (
                    payload.get("capability").and_then(Value::as_str),
                    payload.get("outcome").and_then(Value::as_str),
                ) {
                    projection
                        .phase_statuses
                        .insert(capability.to_string(), outcome.to_string());
                }
            }
            "PhaseProjected" => {
                if let Some(phase) = payload.get("phase").and_then(Value::as_str) {
                    projection.current_phase = Some(phase.to_string());
                }
            }
            _ => {}
        }
    }
    projection
}

fn yaml_path_string(value: &serde_yaml::Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str().map(ToString::to_string)
}

fn run_events_path(project_root: &Path, task_id: &str, run_id: &str) -> PathBuf {
    project_root
        .join(".vibehub")
        .join("tasks")
        .join(task_id)
        .join("runs")
        .join(run_id)
        .join("events.jsonl")
}

fn pending_events_dir(project_root: &Path) -> PathBuf {
    project_root.join(".vibehub").join(".pending")
}

fn write_pending_envelope(
    project_root: &Path,
    envelope: EventEnvelope,
    source: &str,
) -> Result<PendingEventWriteResult> {
    let pending_dir = pending_events_dir(project_root);
    fs::create_dir_all(&pending_dir)
        .with_context(|| format!("Failed to create {}", pending_dir.display()))?;
    let pending = PendingEventWriteFile {
        schema_version: "1.0".to_string(),
        kind: "vibehub_event_write".to_string(),
        created_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        source: source.to_string(),
        envelope,
    };
    let timestamp = Utc::now().format("%Y%m%dT%H%M%S%3fZ");
    let filename = format!(
        "{timestamp}-{}-{}.write",
        pending.envelope.event_id,
        Uuid::new_v4()
    );
    let final_path = pending_dir.join(filename);
    let tmp_path = final_path.with_extension("write.tmp");
    let content =
        serde_json::to_string_pretty(&pending).context("Failed to serialize pending event")?;
    fs::write(&tmp_path, format!("{content}\n"))
        .with_context(|| format!("Failed to write {}", tmp_path.display()))?;
    fs::rename(&tmp_path, &final_path)
        .with_context(|| format!("Failed to finalize pending event {}", final_path.display()))?;
    Ok(PendingEventWriteResult {
        pending_path: normalize_path(&relative_to_project(project_root, &final_path)?),
        event_id: pending.envelope.event_id,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PendingReplayAction {
    Replayed {
        task_id: String,
        run_id: String,
        events_path: PathBuf,
        loop_sensitive: bool,
    },
    Skipped {
        warning: String,
    },
}

fn replay_pending_file_locked(
    project_root: &Path,
    pending_path: &Path,
) -> Result<PendingReplayAction> {
    let content = fs::read_to_string(pending_path)
        .with_context(|| format!("Failed to read {}", pending_path.display()))?;
    let pending: PendingEventWriteFile = serde_json::from_str(&content)
        .with_context(|| format!("Invalid pending event JSON in {}", pending_path.display()))?;
    if pending.schema_version != "1.0" || pending.kind != "vibehub_event_write" {
        bail!(
            "Unsupported pending event format {} {}",
            pending.schema_version,
            pending.kind
        );
    }
    let envelope = pending.envelope;
    let events_path = run_events_path(project_root, &envelope.task_id, &envelope.run_id);
    if let Some(parent) = events_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    if let Some(recovery) = recover_existing_log(
        project_root,
        &events_path,
        &envelope.task_id,
        &envelope.run_id,
    )
    .context("Failed to recover corrupted event log before pending replay")?
    {
        let recovery_envelope = EventEnvelope {
            event_id: recovery_event_id_before(&envelope.event_id),
            timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            task_id: envelope.task_id.clone(),
            run_id: envelope.run_id.clone(),
            actor: "vibehub".to_string(),
            schema_version: "1.0".to_string(),
            event: VibehubEvent::EventLogCorrupted {
                at_offset: recovery.at_offset,
                recovered_to: recovery.recovered_to,
            },
        };
        let offset = append_envelope(&events_path, &recovery_envelope)?;
        update_task_event_index(
            project_root,
            &envelope.task_id,
            &envelope.run_id,
            &recovery_envelope.event_id,
            offset,
        )?;
    }

    let existing = list_events_from_path(&events_path, None)?;
    if existing
        .iter()
        .any(|event| event.event_id.as_deref() == Some(envelope.event_id.as_str()))
    {
        return Ok(PendingReplayAction::Skipped {
            warning: format!(
                "Pending event {} was already present; skipped duplicate.",
                envelope.event_id
            ),
        });
    }
    if let Some(last_event_id) = existing
        .iter()
        .rev()
        .find_map(|event| event.event_id.as_deref())
    {
        if envelope.event_id.as_str() <= last_event_id {
            return Ok(PendingReplayAction::Skipped {
                warning: format!(
                    "Pending event {} is not after existing event {}; skipped to preserve append order.",
                    envelope.event_id, last_event_id
                ),
            });
        }
    }

    let loop_sensitive = should_detect_loop(&envelope.event);
    let task_id = envelope.task_id.clone();
    let run_id = envelope.run_id.clone();
    let event_id = envelope.event_id.clone();
    let offset = append_envelope(&events_path, &envelope)?;
    update_task_event_index(project_root, &task_id, &run_id, &event_id, offset)?;
    Ok(PendingReplayAction::Replayed {
        task_id,
        run_id,
        events_path,
        loop_sensitive,
    })
}

fn recovery_event_id_before(event_id: &str) -> String {
    event_id
        .rsplit_once('-')
        .map(|(prefix, _)| format!("{prefix}-0000"))
        .unwrap_or_else(|| format!("evt-recovery-{event_id}"))
}

fn remove_pending_file(path: &Path, warnings: &mut Vec<String>) {
    if let Err(error) = fs::remove_file(path) {
        warnings.push(format!(
            "Pending event {} replayed but could not be removed: {error}",
            path.display()
        ));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
struct TaskEventIndex {
    schema_version: u32,
    tasks: BTreeMap<String, Vec<TaskEventIndexEntry>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct TaskEventIndexEntry {
    run_id: String,
    event_id: String,
    offset: u64,
}

fn update_task_event_index(
    project_root: &Path,
    task_id: &str,
    run_id: &str,
    event_id: &str,
    offset: u64,
) -> Result<()> {
    let index_path = project_root.join(".vibehub/index/task-events.idx");
    if let Some(parent) = index_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }

    let mut index = if index_path.is_file() {
        let content = fs::read_to_string(&index_path)
            .with_context(|| format!("Failed to read {}", index_path.display()))?;
        serde_json::from_str::<TaskEventIndex>(&content)
            .with_context(|| format!("Invalid JSON in {}", index_path.display()))?
    } else {
        TaskEventIndex {
            schema_version: 1,
            tasks: BTreeMap::new(),
        }
    };
    index.schema_version = 1;
    let entries = index.tasks.entry(task_id.to_string()).or_default();
    if !entries
        .iter()
        .any(|entry| entry.run_id == run_id && entry.event_id == event_id)
    {
        entries.push(TaskEventIndexEntry {
            run_id: run_id.to_string(),
            event_id: event_id.to_string(),
            offset,
        });
    }

    let content =
        serde_json::to_string_pretty(&index).context("Failed to serialize event index")?;
    fs::write(&index_path, format!("{content}\n"))
        .with_context(|| format!("Failed to write {}", index_path.display()))
}

fn list_events_from_path(events_path: &Path, since: Option<&str>) -> Result<Vec<StoredEvent>> {
    if !events_path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(events_path)
        .with_context(|| format!("Failed to read {}", events_path.display()))?;
    let mut include = since.is_none();
    let mut events = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let event: Value = serde_json::from_str(line).with_context(|| {
            format!(
                "Failed to parse {} line {} as JSON",
                events_path.display(),
                idx + 1
            )
        })?;
        let event_id = event
            .get("event_id")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        if let Some(since) = since {
            if !include {
                include = event_id.as_deref() == Some(since);
                continue;
            }
        }
        if include {
            events.push(StoredEvent { event_id, event });
        }
    }
    Ok(events)
}

#[cfg_attr(not(test), allow(dead_code))]
fn validate_existing_log(events_path: &Path) -> Result<()> {
    if !events_path.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(events_path)
        .with_context(|| format!("Failed to read {}", events_path.display()))?;
    let mut previous_event_id: Option<String> = None;
    for (idx, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line).with_context(|| {
            format!(
                "Failed to parse {} line {} as JSON",
                events_path.display(),
                idx + 1
            )
        })?;
        let Some(event_id) = value.get("event_id").and_then(Value::as_str) else {
            continue;
        };
        if let Some(previous) = previous_event_id.as_deref() {
            if event_id <= previous {
                bail!("event_id {event_id} is not after {previous}");
            }
        }
        previous_event_id = Some(event_id.to_string());
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecoveryAction {
    at_offset: u64,
    recovered_to: String,
}

fn recover_existing_log(
    project_root: &Path,
    events_path: &Path,
    _task_id: &str,
    _run_id: &str,
) -> Result<Option<RecoveryAction>> {
    if !events_path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(events_path)
        .with_context(|| format!("Failed to read {}", events_path.display()))?;
    let mut previous_event_id: Option<String> = None;
    let mut valid_offset = 0_usize;
    for segment in content.split_inclusive('\n') {
        let line = segment.trim_end_matches('\n');
        if line.trim().is_empty() {
            valid_offset += segment.len();
            continue;
        }
        let value: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(_) => break,
        };
        if let Some(event_id) = value.get("event_id").and_then(Value::as_str) {
            if let Some(previous) = previous_event_id.as_deref() {
                if event_id <= previous {
                    break;
                }
            }
            previous_event_id = Some(event_id.to_string());
        }
        valid_offset += segment.len();
    }
    if valid_offset >= content.len() {
        return Ok(None);
    }
    OpenOptions::new()
        .write(true)
        .open(events_path)
        .with_context(|| format!("Failed to open {}", events_path.display()))?
        .set_len(valid_offset as u64)
        .with_context(|| format!("Failed to truncate {}", events_path.display()))?;
    Ok(Some(RecoveryAction {
        at_offset: valid_offset as u64,
        recovered_to: normalize_path(&relative_to_project(project_root, events_path)?),
    }))
}

fn should_detect_loop(event: &VibehubEvent) -> bool {
    matches!(
        event,
        VibehubEvent::CapabilityClaimed { .. } | VibehubEvent::CapabilityReleased { .. }
    )
}

fn maybe_append_loop_warning(
    project_root: &Path,
    events_path: &Path,
    task_id: &str,
    run_id: &str,
    id_generator: &mut EventIdGenerator,
) -> Result<()> {
    let configured = policy::read_policy(project_root).unwrap_or_default();
    let threshold = configured.loop_detection.events_write_per_minute_warn_at;
    let events = list_events_from_path(events_path, None)?;
    let Some((capability, rate)) = detect_capability_loop(&events, threshold) else {
        return Ok(());
    };
    if last_event_is_loop_warning(&events, &capability) {
        return Ok(());
    }
    let envelope = EventEnvelope {
        event_id: id_generator.next(),
        timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        task_id: task_id.to_string(),
        run_id: run_id.to_string(),
        actor: "vibehub".to_string(),
        schema_version: "1.0".to_string(),
        event: VibehubEvent::LoopWarning {
            capability,
            events_per_minute: rate,
            threshold,
        },
    };
    let offset = append_envelope(events_path, &envelope)?;
    update_task_event_index(project_root, task_id, run_id, &envelope.event_id, offset)
}

fn detect_capability_loop(events: &[StoredEvent], threshold: u32) -> Option<(String, u32)> {
    if threshold == 0 {
        return None;
    }
    let mut by_capability: BTreeMap<String, Vec<i64>> = BTreeMap::new();
    for event in events {
        let Some(event_type) = event.event.get("event_type").and_then(Value::as_str) else {
            continue;
        };
        if !matches!(event_type, "CapabilityClaimed" | "CapabilityReleased") {
            continue;
        }
        let payload = event.event.get("payload").unwrap_or(&Value::Null);
        let Some(capability) = payload.get("capability").and_then(Value::as_str) else {
            continue;
        };
        let Some(timestamp) = event
            .event
            .get("timestamp")
            .and_then(Value::as_str)
            .and_then(parse_timestamp_ms)
        else {
            continue;
        };
        by_capability
            .entry(capability.to_string())
            .or_default()
            .push(timestamp);
    }
    for (capability, mut timestamps) in by_capability {
        timestamps.sort_unstable();
        if timestamps.len() < 4 {
            continue;
        }
        let first = *timestamps.first().unwrap();
        let last = *timestamps.last().unwrap();
        let minutes = ((last - first).max(1) as f64) / 60_000.0;
        let rate = (timestamps.len() as f64 / minutes).ceil() as u32;
        if rate > threshold {
            return Some((capability, rate));
        }
    }
    None
}

fn last_event_is_loop_warning(events: &[StoredEvent], capability: &str) -> bool {
    events.iter().rev().any(|event| {
        let is_warning = event.event.get("event_type").and_then(Value::as_str)
            == Some("LoopWarning")
            && event
                .event
                .get("payload")
                .and_then(|payload| payload.get("capability"))
                .and_then(Value::as_str)
                == Some(capability);
        let is_capability_event = matches!(
            event.event.get("event_type").and_then(Value::as_str),
            Some("CapabilityClaimed" | "CapabilityReleased")
        );
        is_warning || is_capability_event
    }) && events.iter().rev().find_map(|event| {
        if event.event.get("event_type").and_then(Value::as_str) == Some("LoopWarning") {
            event
                .event
                .get("payload")
                .and_then(|payload| payload.get("capability"))
                .and_then(Value::as_str)
                .map(|value| value == capability)
        } else if matches!(
            event.event.get("event_type").and_then(Value::as_str),
            Some("CapabilityClaimed" | "CapabilityReleased")
        ) {
            Some(false)
        } else {
            None
        }
    }) == Some(true)
}

fn parse_timestamp_ms(value: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|timestamp| timestamp.timestamp_millis())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    fn temp_project() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("vibehub-events-test-{}", Uuid::new_v4()));
        fs::create_dir_all(path.join(".vibehub/tasks/T-001/runs/R-001")).expect("create");
        path
    }

    fn activate_current_run(project: &Path, phase: &str, status: &str) {
        fs::write(
            project.join(".vibehub/tasks/current"),
            r#"schema_version: 1
kind: current_task_pointer
task_id: T-001
path: .vibehub/tasks/T-001
updated_at: "2026-05-28T00:00:00Z"
updated_by: vibehub
"#,
        )
        .expect("write current task pointer");
        fs::write(
            project.join(".vibehub/tasks/T-001/runs/current"),
            r#"schema_version: 1
kind: current_run_pointer
task_id: T-001
run_id: R-001
path: .vibehub/tasks/T-001/runs/R-001
updated_at: "2026-05-28T00:00:00Z"
updated_by: vibehub
"#,
        )
        .expect("write current run pointer");
        write_projected_state(project, phase, status);
    }

    fn write_projected_state(project: &Path, phase: &str, status: &str) {
        let mut flow = std::collections::BTreeMap::new();
        flow.insert(phase.to_string(), status.to_string());
        write_state_with_flow(project, phase, status, &flow);
    }

    fn write_state_with_flow(
        project: &Path,
        phase: &str,
        status: &str,
        flow: &std::collections::BTreeMap<String, String>,
    ) {
        let state = format!(
            r#"schema_version: 3
current:
  task_id: T-001
  run_id: R-001
  event_log_path: .vibehub/tasks/T-001/runs/R-001/events.jsonl
  phase: {phase}
  phase_status: {status}
flow:
  align: {align}
  research: {research}
  implement: {implement}
"#,
            align = flow.get("align").map(String::as_str).unwrap_or("pending"),
            research = flow
                .get("research")
                .map(String::as_str)
                .unwrap_or("pending"),
            implement = flow
                .get("implement")
                .map(String::as_str)
                .unwrap_or("pending"),
        );
        fs::write(project.join(".vibehub/state.yaml"), state).expect("write state");
    }

    fn stored_event(event_type: &str, payload: Value) -> StoredEvent {
        StoredEvent {
            event_id: None,
            event: json!({
                "event_type": event_type,
                "payload": payload,
            }),
        }
    }

    #[test]
    fn event_id_is_unique_and_strictly_increments_within_same_millisecond() {
        let mut generator = EventIdGenerator::new();
        assert_eq!(
            generator.next_at(1_716_800_000_000),
            "evt-1716800000000-0001"
        );
        assert_eq!(
            generator.next_at(1_716_800_000_000),
            "evt-1716800000000-0002"
        );
    }

    #[test]
    fn event_id_does_not_move_backwards_when_clock_regresses() {
        let mut generator = EventIdGenerator::new();
        assert_eq!(generator.next_at(10), "evt-10-0001");
        assert_eq!(generator.next_at(9), "evt-10-0002");
    }

    #[test]
    fn appends_run_event_jsonl() {
        let project = temp_project();

        let result = append_run_event(
            &project,
            "T-001",
            "R-001",
            "review_evidence_generated",
            "Review evidence generated.",
            json!({"review_path": "phases/review.md"}),
        )
        .expect("append event");

        assert_eq!(
            result.events_path,
            ".vibehub/tasks/T-001/runs/R-001/events.jsonl"
        );
        assert!(result.event_id.starts_with("evt-"));
        let content = fs::read_to_string(project.join(&result.events_path)).expect("read");
        assert!(content.contains("\"event_type\":\"Legacy\""));
        assert!(content.contains("\"legacy_event_type\":\"review_evidence_generated\""));
        assert!(content.contains("\"evidence_grade\":\"hard_observed\""));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn event_writer_appends_and_lists_events() {
        let project = temp_project();
        let writer = EventWriter::new();
        let first = writer
            .append(
                &project,
                "T-001",
                "R-001",
                "main-agent",
                VibehubEvent::CapabilityClaimed {
                    capability: "research".to_string(),
                },
            )
            .expect("append first");
        writer
            .append(
                &project,
                "T-001",
                "R-001",
                "main-agent",
                VibehubEvent::CapabilityReleased {
                    capability: "research".to_string(),
                    outcome: "completed".to_string(),
                    task_pack_dirty: true,
                },
            )
            .expect("append second");

        let all = writer
            .list_events(&project, "T-001", "R-001", None)
            .expect("list all");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].event_id.as_deref(), Some(first.event_id.as_str()));

        let since_first = writer
            .list_events(&project, "T-001", "R-001", Some(&first.event_id))
            .expect("list since");
        assert_eq!(since_first.len(), 1);

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn list_events_returns_empty_for_missing_log() {
        let project = temp_project();
        let events = list_events(&project, "T-001", "R-001-missing", None).expect("list");
        assert!(events.is_empty());
        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn pending_event_write_replays_and_cleans_pending_file() {
        let project = temp_project();
        let writer = EventWriter::new();
        let pending = writer
            .write_pending(
                &project,
                "T-001",
                "R-001",
                "agent",
                VibehubEvent::TaskCreated {
                    intent: "offline write".to_string(),
                    mode: "evidence_drive".to_string(),
                },
            )
            .expect("write pending event");

        assert!(project.join(&pending.pending_path).is_file());
        assert!(!project
            .join(".vibehub/tasks/T-001/runs/R-001/events.jsonl")
            .exists());

        let replay = writer.replay_pending(&project).expect("replay pending");
        assert_eq!(replay.replayed, 1);
        assert_eq!(replay.skipped, 0);
        assert!(!project.join(&pending.pending_path).exists());

        let events = writer
            .list_events(&project, "T-001", "R-001", None)
            .expect("list replayed");
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].event_id.as_deref(),
            Some(pending.event_id.as_str())
        );
        assert_eq!(
            events[0].event.get("event_type").and_then(Value::as_str),
            Some("TaskCreated")
        );

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn pending_event_replay_skips_duplicate_event_id_and_removes_file() {
        let project = temp_project();
        let writer = EventWriter::new();
        let pending = writer
            .write_pending(
                &project,
                "T-001",
                "R-001",
                "agent",
                VibehubEvent::TaskCreated {
                    intent: "duplicate".to_string(),
                    mode: "evidence_drive".to_string(),
                },
            )
            .expect("write pending event");
        let pending_file = project.join(&pending.pending_path);
        let pending_content = fs::read_to_string(&pending_file).expect("read pending");
        let pending_payload: PendingEventWriteFile =
            serde_json::from_str(&pending_content).expect("parse pending");
        let events_path = project.join(".vibehub/tasks/T-001/runs/R-001/events.jsonl");
        append_envelope(&events_path, &pending_payload.envelope).expect("seed duplicate");

        let replay = writer.replay_pending(&project).expect("replay duplicate");
        assert_eq!(replay.replayed, 0);
        assert_eq!(replay.skipped, 1);
        assert!(replay
            .warnings
            .iter()
            .any(|warning| warning.contains("skipped duplicate")));
        assert!(!pending_file.exists());

        let events = writer
            .list_events(&project, "T-001", "R-001", None)
            .expect("list events");
        assert_eq!(events.len(), 1);

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn pending_replay_and_normal_append_share_single_writer_lock() {
        let project = temp_project();
        let writer = std::sync::Arc::new(EventWriter::new());
        for idx in 0..4 {
            writer
                .write_pending(
                    &project,
                    "T-001",
                    "R-001",
                    "agent",
                    VibehubEvent::EvidenceAdded {
                        source: format!("source-{idx}"),
                        summary: "pending".to_string(),
                        refs: Vec::new(),
                    },
                )
                .expect("write pending");
        }

        let replay_project = project.clone();
        let replay_writer = writer.clone();
        let replay_thread = std::thread::spawn(move || {
            replay_writer
                .replay_pending(&replay_project)
                .expect("replay pending")
        });
        let append_project = project.clone();
        let append_writer = writer.clone();
        let append_thread = std::thread::spawn(move || {
            append_writer
                .append(
                    &append_project,
                    "T-001",
                    "R-001",
                    "vibehub",
                    VibehubEvent::GateChecked {
                        gate: "single_writer".to_string(),
                        result: true,
                        reasons: Vec::new(),
                    },
                )
                .expect("append while replaying")
        });

        let _ = replay_thread.join().expect("join replay");
        let _ = append_thread.join().expect("join append");
        let events_path = project.join(".vibehub/tasks/T-001/runs/R-001/events.jsonl");
        validate_existing_log(&events_path).expect("event log remains ordered");
        assert!(fs::read_dir(project.join(".vibehub/.pending"))
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(true));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn validate_existing_log_rejects_out_of_order_event_ids() {
        let project = temp_project();
        let events_path = project.join(".vibehub/tasks/T-001/runs/R-001/events.jsonl");
        fs::write(
            &events_path,
            concat!(
                "{\"event_id\":\"evt-1-0002\",\"event_type\":\"TaskCreated\"}\n",
                "{\"event_id\":\"evt-1-0001\",\"event_type\":\"TaskCreated\"}\n"
            ),
        )
        .expect("write log");

        let err = validate_existing_log(&events_path).expect_err("out of order should fail");
        assert!(err.to_string().contains("is not after"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn corrupted_event_log_truncates_and_writes_recovery_event() {
        let project = temp_project();
        let events_path = project.join(".vibehub/tasks/T-001/runs/R-001/events.jsonl");
        fs::write(
            &events_path,
            concat!(
                "{\"event_id\":\"evt-1-0001\",\"timestamp\":\"2026-05-28T00:00:00Z\",\"event_type\":\"TaskCreated\",\"payload\":{\"intent\":\"x\",\"mode\":\"m\"}}\n",
                "{not json}\n"
            ),
        )
        .expect("write corrupted log");

        let writer = EventWriter::new();
        writer
            .append(
                &project,
                "T-001",
                "R-001",
                "main-agent",
                VibehubEvent::TaskCreated {
                    intent: "test".to_string(),
                    mode: "evidence_drive".to_string(),
                },
            )
            .expect("corruption should recover and append");

        let after = fs::read_to_string(&events_path).expect("read after");
        assert!(!after.contains("{not json}"));
        assert!(after.contains("EventLogCorrupted"));
        assert!(after.contains("TaskCreated"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn event_enum_serializes_all_m1a_required_variants() {
        let events = vec![
            VibehubEvent::TaskCreated {
                intent: "i".into(),
                mode: "m".into(),
            },
            VibehubEvent::TaskIntakePlanned {
                source_message: Some("one message, multiple tasks".into()),
                split_confidence: "high".into(),
                split_reason: Some("independent deliverables".into()),
                not_split_reason: None,
                task_ids: vec!["T-1".into(), "T-2".into()],
                execution_order: vec!["T-1".into(), "T-2".into()],
            },
            VibehubEvent::TaskCancelled { reason: "r".into() },
            VibehubEvent::CapabilityClaimed {
                capability: "c".into(),
            },
            VibehubEvent::CapabilityReleased {
                capability: "c".into(),
                outcome: "completed".into(),
                task_pack_dirty: false,
            },
            VibehubEvent::CapabilityPackBuilt {
                capability: "c".into(),
                pack_path: "p".into(),
                size_tokens: 1,
            },
            VibehubEvent::TaskPackRebuilt {
                reason: "r".into(),
                delta_fields: vec!["intent".into()],
            },
            VibehubEvent::PackOversize {
                kind: "capability".into(),
                size: 2,
                threshold: 1,
            },
            VibehubEvent::EvidenceAdded {
                source: "s".into(),
                summary: "sum".into(),
                refs: vec!["r".into()],
            },
            VibehubEvent::PlanDrafted {
                plan_path: "p".into(),
                scope: "s".into(),
            },
            VibehubEvent::DiffObserved {
                commit_range: "HEAD".into(),
                files: vec!["f".into()],
            },
            VibehubEvent::FileOwnershipUpdated {
                task_id: "T-1".into(),
                files_added: vec!["f".into()],
                files_removed: vec![],
            },
            VibehubEvent::ValidationRun {
                kind: "test".into(),
                status: "passed".into(),
                output_ref: "o".into(),
            },
            VibehubEvent::RiskRaised {
                id: "R1".into(),
                severity: "low".into(),
                note: "n".into(),
            },
            VibehubEvent::RiskResolved {
                id: "R1".into(),
                severity: None,
                note: None,
                resolution: Some("done".into()),
            },
            VibehubEvent::HandoffWritten {
                path: "h".into(),
                capability: Some("c".into()),
            },
            VibehubEvent::GateChecked {
                gate: "g".into(),
                result: true,
                reasons: vec![],
            },
            VibehubEvent::PhaseProjected {
                phase: "research".into(),
                derived_from: vec!["evt-1-0001".into()],
            },
            VibehubEvent::SyncStarted {
                mode: "deep".into(),
                signals: json!({}),
                report_path: None,
            },
            VibehubEvent::SyncCompleted {
                mode: "deep".into(),
                signals: json!({}),
                report_path: "s.md".into(),
            },
            VibehubEvent::SyncReport {
                report_path: "s.md".into(),
                status: "completed".into(),
            },
            VibehubEvent::SchemaValidationFailed {
                target: "t".into(),
                errors: vec![json!({"code":"schema.required.missing"})],
            },
            VibehubEvent::EventLogCorrupted {
                at_offset: 0,
                recovered_to: "p".into(),
            },
            VibehubEvent::LoopWarning {
                capability: "implement".into(),
                events_per_minute: 31,
                threshold: 30,
            },
            VibehubEvent::PlanInvalidated { reason: "r".into() },
            VibehubEvent::DiffReverted { reason: "r".into() },
        ];

        for event in events {
            let value = serde_json::to_value(event).expect("serialize");
            assert!(value.get("event_type").is_some());
        }
    }

    #[test]
    fn rapid_claim_release_cycle_appends_loop_warning() {
        let project = temp_project();
        fs::write(
            project.join(".vibehub/policy.yaml"),
            "loop_detection:\n  events_write_per_minute_warn_at: 10\n",
        )
        .expect("policy");
        let writer = EventWriter::new();
        for idx in 0..4 {
            writer
                .append(
                    &project,
                    "T-001",
                    "R-001",
                    "vibehub-test",
                    if idx % 2 == 0 {
                        VibehubEvent::CapabilityClaimed {
                            capability: "implement".into(),
                        }
                    } else {
                        VibehubEvent::CapabilityReleased {
                            capability: "implement".into(),
                            outcome: "failed".into(),
                            task_pack_dirty: true,
                        }
                    },
                )
                .expect("append");
        }

        let content =
            fs::read_to_string(project.join(".vibehub/tasks/T-001/runs/R-001/events.jsonl"))
                .expect("events");
        assert!(content.contains("LoopWarning"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn folds_structured_events_into_phase_projection() {
        let projection = fold_event_state(vec![
            stored_event("CapabilityClaimed", json!({"capability": "research"})),
            stored_event(
                "CapabilityReleased",
                json!({"capability": "research", "outcome": "completed"}),
            ),
            stored_event("CapabilityClaimed", json!({"capability": "implement"})),
            stored_event("PhaseProjected", json!({"phase": "implement"})),
        ]);

        assert_eq!(projection.current_phase.as_deref(), Some("implement"));
        assert_eq!(
            projection
                .phase_statuses
                .get("research")
                .map(String::as_str),
            Some("completed")
        );
        assert_eq!(
            projection
                .phase_statuses
                .get("implement")
                .map(String::as_str),
            Some("active")
        );
    }

    #[test]
    fn ensure_consistency_compares_phase_status_projection() {
        let project = temp_project();
        activate_current_run(&project, "research", "active");

        append_structured_run_event(
            &project,
            "T-001",
            "R-001",
            VibehubEvent::CapabilityClaimed {
                capability: "research".into(),
            },
        )
        .expect("claim");
        append_structured_run_event(
            &project,
            "T-001",
            "R-001",
            VibehubEvent::PhaseProjected {
                phase: "research".into(),
                derived_from: vec![],
            },
        )
        .expect("project");

        assert!(ensure_consistency(&project).expect("consistent").is_empty());
        write_projected_state(&project, "research", "completed");
        let warnings = ensure_consistency(&project).expect("warnings");
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("current.phase_status")));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn random_event_sequences_match_parallel_state_projection() {
        let phases = ["align", "research", "implement"];
        let mut seed = 0x5eed_u64;
        for _ in 0..100 {
            let project = temp_project();
            activate_current_run(&project, "align", "active");
            let writer = EventWriter::new();
            let mut phase = "align";
            let mut status = "active";
            let mut flow = std::collections::BTreeMap::new();
            flow.insert(phase.to_string(), status.to_string());

            for _ in 0..12 {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let next_phase = phases[(seed as usize) % phases.len()];
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let next_status = if seed % 3 == 0 {
                    "completed"
                } else if seed % 3 == 1 {
                    "blocked"
                } else {
                    "active"
                };

                writer
                    .append(
                        &project,
                        "T-001",
                        "R-001",
                        "vibehub-test",
                        if next_status == "active" {
                            VibehubEvent::CapabilityClaimed {
                                capability: next_phase.into(),
                            }
                        } else {
                            VibehubEvent::CapabilityReleased {
                                capability: next_phase.into(),
                                outcome: next_status.into(),
                                task_pack_dirty: next_status != "completed",
                            }
                        },
                    )
                    .expect("append status event");
                writer
                    .append(
                        &project,
                        "T-001",
                        "R-001",
                        "vibehub-test",
                        VibehubEvent::PhaseProjected {
                            phase: next_phase.into(),
                            derived_from: vec![],
                        },
                    )
                    .expect("append phase projection");

                phase = next_phase;
                status = next_status;
                flow.insert(phase.to_string(), status.to_string());
                write_state_with_flow(&project, phase, status, &flow);
            }

            assert!(
                ensure_consistency(&project).expect("consistent").is_empty(),
                "sequence ended at {phase}:{status}"
            );
            fs::remove_dir_all(project).expect("cleanup");
        }
    }

    #[test]
    fn restart_self_check_survives_repeated_appends() {
        let project = temp_project();
        activate_current_run(&project, "research", "active");

        for idx in 0..10 {
            std::thread::sleep(std::time::Duration::from_millis(1));
            let writer = EventWriter::new();
            writer
                .append(
                    &project,
                    "T-001",
                    "R-001",
                    "vibehub-test",
                    VibehubEvent::CapabilityClaimed {
                        capability: "research".into(),
                    },
                )
                .expect("append after restart");
            writer
                .append(
                    &project,
                    "T-001",
                    "R-001",
                    "vibehub-test",
                    VibehubEvent::PhaseProjected {
                        phase: "research".into(),
                        derived_from: vec![format!("restart-{idx}")],
                    },
                )
                .expect("project after restart");

            assert!(ensure_consistency(&project).expect("self-check").is_empty());
        }

        fs::remove_dir_all(project).expect("cleanup");
    }
}

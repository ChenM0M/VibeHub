use crate::vibehub::util::{
    canonical_project_root, normalize_path, relative_to_project, yaml_string,
};
use crate::vibehub::{
    agent_view, archive, context, current, events, init, projection, research, state_migration,
};
use anyhow::{anyhow, Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const DEFAULT_MODE: &str = "guided_drive";
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VibehubStartTaskResult {
    pub task_id: String,
    pub run_id: String,
    pub mode: String,
    pub phase: String,
    pub phase_status: String,
    pub task_path: String,
    pub run_path: String,
    pub task_pointer_path: String,
    pub run_pointer_path: String,
    pub context_spec_path: String,
    pub context_pack_path: String,
    pub context_manifest_path: String,
    pub context_included_count: usize,
    pub context_missing_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VibehubIntakeConfidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VibehubTaskDraft {
    pub title: String,
    #[serde(default)]
    pub intent: Option<String>,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub suggested_order: Option<usize>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub phase: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VibehubStartTaskIntakeRequest {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub intent: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub phase: Option<String>,
    #[serde(default)]
    pub source_message: Option<String>,
    #[serde(default)]
    pub split_confidence: Option<VibehubIntakeConfidence>,
    #[serde(default)]
    pub split_reason: Option<String>,
    #[serde(default)]
    pub not_split_reason: Option<String>,
    #[serde(default)]
    pub intake: Vec<VibehubTaskDraft>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VibehubTaskIntakeItem {
    pub order: usize,
    pub title: String,
    pub intent: Option<String>,
    pub acceptance_criteria: Vec<String>,
    pub dependencies: Vec<String>,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VibehubTaskIntakeFailure {
    pub order: usize,
    pub title: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VibehubStartTaskIntakeResult {
    pub classification: String,
    pub confirmation_required: bool,
    pub source_message: Option<String>,
    pub split_reason: Option<String>,
    pub not_split_reason: Option<String>,
    pub proposed_tasks: Vec<VibehubTaskIntakeItem>,
    pub created_tasks: Vec<VibehubStartTaskResult>,
    pub failed_tasks: Vec<VibehubTaskIntakeFailure>,
    pub active_tasks: Vec<String>,
    pub current_task_id: Option<String>,
    pub current_run_id: Option<String>,
    pub queue_state_path: Option<String>,
    pub unresolved_risks: Vec<String>,
}

pub fn start_task(
    project_root: impl AsRef<Path>,
    title: Option<String>,
    mode: Option<String>,
    phase: Option<String>,
) -> Result<VibehubStartTaskResult> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    reject_v3_legacy_task_creation(&project_root)?;
    ensure_initialized(&project_root)?;

    let _ = research::archive_current_research(&project_root);
    archive::archive_completed_tasks(&project_root, None)
        .context("Failed to auto-archive completed tasks before starting a new task")?;

    let mode = validate_id("mode", mode.as_deref().unwrap_or(DEFAULT_MODE))?.to_string();
    let phase = phase
        .as_deref()
        .map(|value| validate_id("phase", value).map(str::to_string))
        .transpose()?
        .unwrap_or_else(|| default_phase_for_mode(&mode).to_string());
    validate_phase_for_mode(&mode, &phase)?;
    let phase_status = "active".to_string();
    let task_id = next_id("T");
    let run_id = next_id("R");

    let task_dir = project_root.join(".vibehub").join("tasks").join(&task_id);
    let context_dir = task_dir.join("context");
    let run_dir = task_dir.join("runs").join(&run_id);
    fs::create_dir_all(&context_dir)
        .with_context(|| format!("Failed to create {}", context_dir.display()))?;
    fs::create_dir_all(run_dir.join("sessions"))
        .with_context(|| format!("Failed to create {}", run_dir.join("sessions").display()))?;
    fs::create_dir_all(run_dir.join("outputs"))
        .with_context(|| format!("Failed to create {}", run_dir.join("outputs").display()))?;

    let task_title = title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("VibeHub task");
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let task_path = normalize_path(&relative_to_project(&project_root, &task_dir)?);
    let run_path = normalize_path(&relative_to_project(&project_root, &run_dir)?);

    fs::write(
        task_dir.join("task.yaml"),
        task_yaml(&task_id, task_title, &mode, &phase, &phase_status, &now),
    )
    .with_context(|| format!("Failed to write {}", task_dir.join("task.yaml").display()))?;
    fs::write(
        run_dir.join("run.yaml"),
        run_yaml(&task_id, &run_id, &mode, &phase, &phase_status, &now),
    )
    .with_context(|| format!("Failed to write {}", run_dir.join("run.yaml").display()))?;

    let context_spec_path = ensure_context_spec(&project_root, &task_id, &run_id, &phase)?;

    current::write_current_task_pointer(&project_root, &task_id)?;
    current::write_current_run_pointer(&project_root, &task_id, &run_id)?;
    update_state(
        &project_root,
        &task_id,
        &run_id,
        &mode,
        &phase,
        &phase_status,
        &run_path,
    )?;

    let pack = context::build_context_pack(&project_root, &task_id, &run_id, &phase)
        .context("Failed to auto-build context pack after starting task")?;
    let legacy_event = events::append_run_event(
        &project_root,
        &task_id,
        &run_id,
        "task_started",
        "Task and run created.",
        serde_json::json!({
            "mode": mode.clone(),
            "phase": phase.clone(),
            "phase_status": phase_status.clone(),
            "context_pack_path": pack.pack_path.clone(),
            "context_manifest_path": pack.manifest_path.clone(),
        }),
    )
    .ok();
    let task_event = events::append_structured_run_event(
        &project_root,
        &task_id,
        &run_id,
        events::VibehubEvent::TaskCreated {
            intent: task_title.to_string(),
            mode: mode.clone(),
        },
    )
    .ok();
    let claim_event = events::append_structured_run_event(
        &project_root,
        &task_id,
        &run_id,
        events::VibehubEvent::CapabilityClaimed {
            capability: phase.clone(),
        },
    )
    .ok();
    let _ = events::append_structured_run_event(
        &project_root,
        &task_id,
        &run_id,
        events::VibehubEvent::PhaseProjected {
            phase: phase.clone(),
            derived_from: event_ids([legacy_event, task_event, claim_event]),
        },
    );
    let _ = projection::project_current_run_state(&project_root);
    let _ = agent_view::generate_agent_view(&project_root);

    Ok(VibehubStartTaskResult {
        task_id,
        run_id,
        mode,
        phase,
        phase_status,
        task_path,
        run_path,
        task_pointer_path: ".vibehub/tasks/current".to_string(),
        run_pointer_path: format!(
            "{}/runs/current",
            normalize_path(&relative_to_project(&project_root, &task_dir)?)
        ),
        context_spec_path: normalize_path(&relative_to_project(&project_root, &context_spec_path)?),
        context_pack_path: pack.pack_path,
        context_manifest_path: pack.manifest_path,
        context_included_count: pack.included_count,
        context_missing_count: pack.missing_count,
    })
}

pub fn start_task_intake(
    project_root: impl AsRef<Path>,
    request: VibehubStartTaskIntakeRequest,
) -> Result<VibehubStartTaskIntakeResult> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    reject_v3_legacy_task_creation(&project_root)?;
    ensure_initialized(&project_root)?;

    let mut drafts = normalize_task_drafts(&request)?;
    let confidence = request
        .split_confidence
        .clone()
        .unwrap_or_else(|| inferred_confidence(drafts.len()));
    validate_draft_modes(&request, &drafts)?;

    if drafts.len() > 1 && confidence == VibehubIntakeConfidence::Medium {
        return Ok(VibehubStartTaskIntakeResult {
            classification: "confirm_split".to_string(),
            confirmation_required: true,
            source_message: clean_optional(request.source_message),
            split_reason: clean_optional(request.split_reason),
            not_split_reason: None,
            proposed_tasks: draft_items(&drafts, "proposed"),
            created_tasks: Vec::new(),
            failed_tasks: Vec::new(),
            active_tasks: read_active_task_ids_from_state(&project_root)?,
            current_task_id: None,
            current_run_id: None,
            queue_state_path: None,
            unresolved_risks: vec![
                "agent_reported: Split confidence is medium; confirm the task split before creating tasks."
                    .to_string(),
            ],
        });
    }

    let not_split_reason = clean_optional(request.not_split_reason.clone());
    if drafts.len() > 1 && confidence == VibehubIntakeConfidence::Low {
        let collapsed = collapse_drafts_for_low_confidence(&request, &drafts);
        drafts = vec![collapsed];
    }

    if drafts.len() > 3 && confidence == VibehubIntakeConfidence::High {
        return Err(anyhow!(
            "High-confidence multi-intent intake supports up to 3 task drafts in M6d; got {}",
            drafts.len()
        ));
    }

    let batch_id = format!("intake-{}", Utc::now().format("%Y%m%d%H%M%S"));
    let mut created_tasks = Vec::new();
    let mut failed_tasks = Vec::new();
    let total = drafts.len();

    for (index, draft) in drafts.iter().enumerate() {
        let order = draft.suggested_order.unwrap_or(index + 1);
        let mode = draft.mode.clone().or_else(|| request.mode.clone());
        let phase = draft.phase.clone().or_else(|| request.phase.clone());
        match start_task(&project_root, Some(draft.title.clone()), mode, phase) {
            Ok(started) => {
                if let Err(error) = annotate_task_intake_metadata(
                    &project_root,
                    &started,
                    draft,
                    &batch_id,
                    order,
                    total,
                    &confidence,
                    clean_optional(request.source_message.clone()).as_deref(),
                    clean_optional(request.split_reason.clone()).as_deref(),
                    not_split_reason.as_deref(),
                ) {
                    failed_tasks.push(VibehubTaskIntakeFailure {
                        order,
                        title: draft.title.clone(),
                        error: format!(
                            "Created task {} but failed to write intake metadata: {error:#}",
                            started.task_id
                        ),
                    });
                }
                created_tasks.push(started);
            }
            Err(error) => failed_tasks.push(VibehubTaskIntakeFailure {
                order,
                title: draft.title.clone(),
                error: format!("{error:#}"),
            }),
        }
    }

    let proposed_tasks = task_items(&drafts, &created_tasks, &failed_tasks);
    let mut active_tasks = read_active_task_ids_from_state(&project_root)?;
    let mut current_task_id = created_tasks.last().map(|task| task.task_id.clone());
    let mut current_run_id = created_tasks.last().map(|task| task.run_id.clone());
    let mut queue_state_path = None;

    if let Some(first) = created_tasks.first() {
        current::write_current_task_pointer(&project_root, &first.task_id)?;
        current::write_current_run_pointer(&project_root, &first.task_id, &first.run_id)?;
        write_intake_queue_state(
            &project_root,
            &created_tasks,
            &proposed_tasks,
            &request,
            &confidence,
            &batch_id,
            not_split_reason.as_deref(),
        )?;
        active_tasks = read_active_task_ids_from_state(&project_root)?;
        current_task_id = Some(first.task_id.clone());
        current_run_id = Some(first.run_id.clone());
        queue_state_path = Some(".vibehub/state.yaml#tasks.intake_queue".to_string());
        let _ = events::append_structured_run_event(
            &project_root,
            &first.task_id,
            &first.run_id,
            events::VibehubEvent::TaskIntakePlanned {
                source_message: clean_optional(request.source_message.clone()),
                split_confidence: confidence_label(&confidence).to_string(),
                split_reason: clean_optional(request.split_reason.clone()),
                not_split_reason: not_split_reason.clone(),
                task_ids: created_tasks
                    .iter()
                    .map(|task| task.task_id.clone())
                    .collect(),
                execution_order: proposed_tasks
                    .iter()
                    .filter_map(|item| item.task_id.clone())
                    .collect(),
            },
        );
        let _ = projection::project_current_run_state(&project_root);
        let _ = agent_view::generate_agent_view(&project_root);
    }

    let classification = if confidence == VibehubIntakeConfidence::High && created_tasks.len() > 1 {
        "created_multi_task_intake"
    } else if not_split_reason.is_some() || confidence == VibehubIntakeConfidence::Low {
        "created_single_task_with_not_split_reason"
    } else {
        "created_single_task"
    };

    let mut unresolved_risks = failed_tasks
        .iter()
        .map(|failure| {
            format!(
                "agent_reported: Draft {} '{}' was not fully created: {}",
                failure.order, failure.title, failure.error
            )
        })
        .collect::<Vec<_>>();
    if created_tasks.is_empty() {
        unresolved_risks.push("hard_observed: No task was created by intake.".to_string());
    }

    Ok(VibehubStartTaskIntakeResult {
        classification: classification.to_string(),
        confirmation_required: false,
        source_message: clean_optional(request.source_message),
        split_reason: clean_optional(request.split_reason),
        not_split_reason,
        proposed_tasks,
        created_tasks,
        failed_tasks,
        active_tasks,
        current_task_id,
        current_run_id,
        queue_state_path,
        unresolved_risks,
    })
}

fn normalize_task_drafts(request: &VibehubStartTaskIntakeRequest) -> Result<Vec<VibehubTaskDraft>> {
    let mut drafts = if request.intake.is_empty() {
        vec![VibehubTaskDraft {
            title: clean_optional(request.title.clone())
                .unwrap_or_else(|| "VibeHub task".to_string()),
            intent: clean_optional(request.intent.clone()),
            acceptance_criteria: Vec::new(),
            dependencies: Vec::new(),
            suggested_order: Some(1),
            mode: request.mode.clone(),
            phase: request.phase.clone(),
        }]
    } else {
        request.intake.clone()
    };

    for (index, draft) in drafts.iter_mut().enumerate() {
        draft.title = draft.title.trim().to_string();
        if draft.title.is_empty() {
            return Err(anyhow!(
                "Invalid intake draft {}: title is required",
                index + 1
            ));
        }
        draft.intent = clean_optional(draft.intent.clone());
        draft.acceptance_criteria = clean_string_vec(&draft.acceptance_criteria);
        draft.dependencies = clean_string_vec(&draft.dependencies);
        if draft.suggested_order.is_none() {
            draft.suggested_order = Some(index + 1);
        }
    }

    Ok(drafts)
}

fn inferred_confidence(draft_count: usize) -> VibehubIntakeConfidence {
    if draft_count > 1 {
        VibehubIntakeConfidence::High
    } else {
        VibehubIntakeConfidence::Low
    }
}

fn validate_draft_modes(
    request: &VibehubStartTaskIntakeRequest,
    drafts: &[VibehubTaskDraft],
) -> Result<()> {
    for draft in drafts {
        let mode = draft
            .mode
            .as_deref()
            .or(request.mode.as_deref())
            .unwrap_or(DEFAULT_MODE);
        let mode = validate_id("mode", mode)?;
        let phase = draft
            .phase
            .as_deref()
            .or(request.phase.as_deref())
            .map(|phase| validate_id("phase", phase))
            .transpose()?
            .unwrap_or_else(|| default_phase_for_mode(mode));
        validate_phase_for_mode(mode, phase)?;
    }
    Ok(())
}

fn collapse_drafts_for_low_confidence(
    request: &VibehubStartTaskIntakeRequest,
    drafts: &[VibehubTaskDraft],
) -> VibehubTaskDraft {
    let title = clean_optional(request.title.clone()).unwrap_or_else(|| {
        drafts
            .iter()
            .map(|draft| draft.title.as_str())
            .collect::<Vec<_>>()
            .join(" / ")
    });
    let intent = clean_optional(request.intent.clone()).or_else(|| {
        Some(
            drafts
                .iter()
                .map(|draft| {
                    draft
                        .intent
                        .as_deref()
                        .unwrap_or(draft.title.as_str())
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join("; "),
        )
    });
    VibehubTaskDraft {
        title,
        intent,
        acceptance_criteria: drafts
            .iter()
            .flat_map(|draft| draft.acceptance_criteria.iter().cloned())
            .collect(),
        dependencies: Vec::new(),
        suggested_order: Some(1),
        mode: request.mode.clone(),
        phase: request.phase.clone(),
    }
}

fn draft_items(drafts: &[VibehubTaskDraft], status: &str) -> Vec<VibehubTaskIntakeItem> {
    drafts
        .iter()
        .enumerate()
        .map(|(index, draft)| VibehubTaskIntakeItem {
            order: draft.suggested_order.unwrap_or(index + 1),
            title: draft.title.clone(),
            intent: draft.intent.clone(),
            acceptance_criteria: draft.acceptance_criteria.clone(),
            dependencies: draft.dependencies.clone(),
            task_id: None,
            run_id: None,
            status: status.to_string(),
        })
        .collect()
}

fn task_items(
    drafts: &[VibehubTaskDraft],
    created: &[VibehubStartTaskResult],
    failures: &[VibehubTaskIntakeFailure],
) -> Vec<VibehubTaskIntakeItem> {
    drafts
        .iter()
        .enumerate()
        .map(|(index, draft)| {
            let order = draft.suggested_order.unwrap_or(index + 1);
            let created = created.get(index);
            let failed = failures.iter().any(|failure| failure.order == order);
            VibehubTaskIntakeItem {
                order,
                title: draft.title.clone(),
                intent: draft.intent.clone(),
                acceptance_criteria: draft.acceptance_criteria.clone(),
                dependencies: draft.dependencies.clone(),
                task_id: created.map(|task| task.task_id.clone()),
                run_id: created.map(|task| task.run_id.clone()),
                status: if failed {
                    "failed".to_string()
                } else if created.is_some() {
                    "created".to_string()
                } else {
                    "pending".to_string()
                },
            }
        })
        .collect()
}

fn annotate_task_intake_metadata(
    project_root: &Path,
    started: &VibehubStartTaskResult,
    draft: &VibehubTaskDraft,
    batch_id: &str,
    order: usize,
    total: usize,
    confidence: &VibehubIntakeConfidence,
    source_message: Option<&str>,
    split_reason: Option<&str>,
    not_split_reason: Option<&str>,
) -> Result<()> {
    let task_path = project_root
        .join(".vibehub/tasks")
        .join(&started.task_id)
        .join("task.yaml");
    let content = fs::read_to_string(&task_path)
        .with_context(|| format!("Failed to read {}", task_path.display()))?;
    let mut task = serde_yaml::from_str::<Value>(&content)
        .with_context(|| format!("Invalid YAML in {}", task_path.display()))?;

    if let Some(intent) = draft.intent.as_deref() {
        set_string(&mut task, &["intent"], intent);
    }
    set_string_sequence(
        &mut task,
        &["acceptance_criteria"],
        &draft.acceptance_criteria,
    );
    set_string_sequence(&mut task, &["dependencies"], &draft.dependencies);
    set_string(&mut task, &["intake", "batch_id"], batch_id);
    set_string(
        &mut task,
        &["intake", "split_confidence"],
        confidence_label(confidence),
    );
    set_value(
        &mut task,
        &["intake", "suggested_order"],
        Value::Number((order as i64).into()),
    );
    set_value(
        &mut task,
        &["intake", "total_tasks"],
        Value::Number((total as i64).into()),
    );
    if let Some(source_message) = source_message {
        set_string(&mut task, &["intake", "source_message"], source_message);
    }
    if let Some(split_reason) = split_reason {
        set_string(&mut task, &["intake", "split_reason"], split_reason);
    }
    if let Some(not_split_reason) = not_split_reason {
        set_string(&mut task, &["intake", "not_split_reason"], not_split_reason);
    }
    let content = serde_yaml::to_string(&task)
        .with_context(|| format!("Failed to serialize {}", task_path.display()))?;
    fs::write(&task_path, content)
        .with_context(|| format!("Failed to write {}", task_path.display()))
}

fn write_intake_queue_state(
    project_root: &Path,
    created_tasks: &[VibehubStartTaskResult],
    items: &[VibehubTaskIntakeItem],
    request: &VibehubStartTaskIntakeRequest,
    confidence: &VibehubIntakeConfidence,
    batch_id: &str,
    not_split_reason: Option<&str>,
) -> Result<()> {
    let Some(first) = created_tasks.first() else {
        return Ok(());
    };
    let state_path = project_root.join(".vibehub/state.yaml");
    let content = fs::read_to_string(&state_path)
        .with_context(|| format!("Failed to read {}", state_path.display()))?;
    let mut state = serde_yaml::from_str::<Value>(&content)
        .with_context(|| format!("Invalid YAML in {}", state_path.display()))?;
    let created_ids = created_tasks
        .iter()
        .map(|task| task.task_id.clone())
        .collect::<Vec<_>>();
    let existing = active_task_ids_from_value(&state);
    let mut active = created_ids.clone();
    for task_id in existing {
        if !active.contains(&task_id) {
            active.push(task_id);
        }
    }

    set_string(&mut state, &["current", "mode"], &first.mode);
    set_string(&mut state, &["current", "task_id"], &first.task_id);
    set_string(&mut state, &["current", "run_id"], &first.run_id);
    set_string(&mut state, &["current", "phase"], &first.phase);
    set_string(
        &mut state,
        &["current", "phase_status"],
        &first.phase_status,
    );
    set_string(
        &mut state,
        &["current", "event_log_path"],
        &format!("{}/events.jsonl", first.run_path),
    );
    set_string(
        &mut state,
        &["context", "current_pack"],
        &first.context_pack_path,
    );
    set_string(
        &mut state,
        &["context", "current_manifest"],
        &first.context_manifest_path,
    );
    set_value(
        &mut state,
        &["tasks", "active"],
        Value::Sequence(active.iter().map(|id| Value::String(id.clone())).collect()),
    );
    set_string(&mut state, &["tasks", "intake_queue", "batch_id"], batch_id);
    set_string(
        &mut state,
        &["tasks", "intake_queue", "split_confidence"],
        confidence_label(confidence),
    );
    set_string(
        &mut state,
        &["tasks", "intake_queue", "current_task_id"],
        &first.task_id,
    );
    set_string(
        &mut state,
        &["tasks", "intake_queue", "created_at"],
        &Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
    );
    if let Some(source_message) = clean_optional(request.source_message.clone()) {
        set_string(
            &mut state,
            &["tasks", "intake_queue", "source_message"],
            &source_message,
        );
    }
    if let Some(split_reason) = clean_optional(request.split_reason.clone()) {
        set_string(
            &mut state,
            &["tasks", "intake_queue", "split_reason"],
            &split_reason,
        );
    }
    if let Some(not_split_reason) = not_split_reason {
        set_string(
            &mut state,
            &["tasks", "intake_queue", "not_split_reason"],
            not_split_reason,
        );
    }
    set_value(
        &mut state,
        &["tasks", "intake_queue", "remaining_task_ids"],
        Value::Sequence(
            created_ids
                .iter()
                .map(|id| Value::String(id.clone()))
                .collect(),
        ),
    );
    set_value(
        &mut state,
        &["tasks", "intake_queue", "items"],
        Value::Sequence(items.iter().map(intake_item_to_yaml).collect()),
    );
    let content = serde_yaml::to_string(&state).context("Failed to serialize state.yaml")?;
    fs::write(&state_path, content)
        .with_context(|| format!("Failed to write {}", state_path.display()))
}

fn intake_item_to_yaml(item: &VibehubTaskIntakeItem) -> Value {
    let mut mapping = Mapping::new();
    mapping.insert(
        Value::String("order".to_string()),
        Value::Number((item.order as i64).into()),
    );
    mapping.insert(
        Value::String("title".to_string()),
        Value::String(item.title.clone()),
    );
    if let Some(intent) = &item.intent {
        mapping.insert(
            Value::String("intent".to_string()),
            Value::String(intent.clone()),
        );
    }
    if let Some(task_id) = &item.task_id {
        mapping.insert(
            Value::String("task_id".to_string()),
            Value::String(task_id.clone()),
        );
    }
    if let Some(run_id) = &item.run_id {
        mapping.insert(
            Value::String("run_id".to_string()),
            Value::String(run_id.clone()),
        );
    }
    mapping.insert(
        Value::String("acceptance_criteria".to_string()),
        Value::Sequence(
            item.acceptance_criteria
                .iter()
                .map(|value| Value::String(value.clone()))
                .collect(),
        ),
    );
    mapping.insert(
        Value::String("dependencies".to_string()),
        Value::Sequence(
            item.dependencies
                .iter()
                .map(|value| Value::String(value.clone()))
                .collect(),
        ),
    );
    mapping.insert(
        Value::String("status".to_string()),
        Value::String(item.status.clone()),
    );
    Value::Mapping(mapping)
}

fn read_active_task_ids_from_state(project_root: &Path) -> Result<Vec<String>> {
    let state_path = project_root.join(".vibehub/state.yaml");
    let content = match fs::read_to_string(&state_path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(error).with_context(|| format!("Failed to read {}", state_path.display()))
        }
    };
    let state = serde_yaml::from_str::<Value>(&content)
        .with_context(|| format!("Invalid YAML in {}", state_path.display()))?;
    Ok(active_task_ids_from_value(&state))
}

fn active_task_ids_from_value(state: &Value) -> Vec<String> {
    let mut active = state
        .get("tasks")
        .and_then(|tasks| tasks.get("active"))
        .and_then(Value::as_sequence)
        .map(|sequence| {
            sequence
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    active.dedup();
    active
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn clean_string_vec(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn set_string_sequence(value: &mut Value, path: &[&str], values: &[String]) {
    set_value(
        value,
        path,
        Value::Sequence(
            values
                .iter()
                .map(|value| Value::String(value.clone()))
                .collect(),
        ),
    );
}

fn confidence_label(confidence: &VibehubIntakeConfidence) -> &'static str {
    match confidence {
        VibehubIntakeConfidence::High => "high",
        VibehubIntakeConfidence::Medium => "medium",
        VibehubIntakeConfidence::Low => "low",
    }
}

fn event_ids<const N: usize>(events: [Option<events::RunEventAppendResult>; N]) -> Vec<String> {
    events
        .into_iter()
        .filter_map(|event| event.map(|event| event.event_id))
        .collect()
}

fn ensure_initialized(project_root: &Path) -> Result<()> {
    init::init_project(project_root)?;
    Ok(())
}

fn reject_v3_legacy_task_creation(project_root: &Path) -> Result<()> {
    let layout = crate::v3::inspect_project_layout(project_root)
        .map_err(|error| anyhow!("V3_PROJECT_LAYOUT_INSPECTION_FAILED: {error}"))?;
    if layout.state == crate::v3::ProjectLayoutState::V3 {
        return Err(anyhow!(
            "V3_LEGACY_START_UNSUPPORTED: this project uses schema_version: 3; legacy `vibehub start` cannot create V3 task metadata. Use `vibehub v3 <project> task-create <request_json_path|--stdin|->`."
        ));
    }
    Ok(())
}

pub fn default_phase_for_mode(mode: &str) -> &'static str {
    match mode {
        "yolo_drive" => "align_lite",
        "guided_drive" | "evidence_drive" => "align",
        _ => "align",
    }
}

fn validate_phase_for_mode(mode: &str, phase: &str) -> Result<()> {
    let allowed = match mode {
        "yolo_drive" => &["align_lite", "implement", "review_lite"][..],
        "guided_drive" => &["align", "plan", "implement", "review"][..],
        "evidence_drive" => &["align", "research", "plan", "implement", "review"][..],
        _ => {
            return Err(anyhow!(
                "Invalid mode '{}': expected yolo_drive, guided_drive, or evidence_drive",
                mode
            ));
        }
    };
    if allowed.contains(&phase) {
        Ok(())
    } else {
        Err(anyhow!(
            "Phase '{}' is not valid for mode '{}'; expected one of {:?}",
            phase,
            mode,
            allowed
        ))
    }
}

fn update_state(
    project_root: &Path,
    task_id: &str,
    run_id: &str,
    mode: &str,
    phase: &str,
    _phase_status: &str,
    run_path: &str,
) -> Result<()> {
    let state_path = project_root.join(".vibehub/state.yaml");
    let mut state = if state_path.is_file() {
        let content = fs::read_to_string(&state_path)
            .with_context(|| format!("Failed to read {}", state_path.display()))?;
        serde_yaml::from_str::<Value>(&content)
            .with_context(|| format!("Invalid YAML in {}", state_path.display()))?
    } else {
        Value::Mapping(Mapping::new())
    };

    set_value(
        &mut state,
        &["schema_version"],
        Value::Number(state_migration::CURRENT_SCHEMA_VERSION.into()),
    );
    set_string(&mut state, &["current", "mode"], mode);
    set_string(&mut state, &["current", "task_id"], task_id);
    set_string(&mut state, &["current", "run_id"], run_id);
    set_null(&mut state, &["current", "session_id"]);
    add_active_task(&mut state, task_id);
    set_string(
        &mut state,
        &["current", "event_log_path"],
        &format!("{run_path}/events.jsonl"),
    );
    set_string(
        &mut state,
        &["pointers", "task_pointer"],
        ".vibehub/tasks/current",
    );
    set_string(
        &mut state,
        &["pointers", "run_pointer"],
        &format!(".vibehub/tasks/{task_id}/runs/current"),
    );
    set_bool(&mut state, &["derived", "current", "phase"], true);
    set_bool(&mut state, &["derived", "current", "phase_status"], true);
    set_bool(&mut state, &["derived", "flow"], true);
    set_string(
        &mut state,
        &["derived", "trace_path"],
        ".vibehub/derivation_trace.yaml",
    );
    set_string(
        &mut state,
        &["context", "current_pack"],
        &format!("{run_path}/context-packs/{phase}.md"),
    );
    set_string(
        &mut state,
        &["context", "current_manifest"],
        &format!("{run_path}/context-packs/{phase}.manifest.yaml"),
    );
    set_bool(&mut state, &["context", "stale"], false);
    set_string(&mut state, &["context", "generated_by"], "vibehub_backend");
    let (research_required, research_status) = if mode == "evidence_drive" {
        (true, "required")
    } else {
        (false, "skipped")
    };
    set_bool(&mut state, &["research", "required"], research_required);
    set_string(&mut state, &["research", "status"], research_status);
    set_string(
        &mut state,
        &["handoff", "current"],
        ".vibehub/agent-view/handoff.md",
    );
    set_string(&mut state, &["handoff", "status"], "empty");
    set_string(&mut state, &["observability", "level"], "best_effort");
    set_string(
        &mut state,
        &["last_updated"],
        &Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
    );
    set_string(
        &mut state,
        &["resume_hint"],
        ".vibehub/agent-view/current.md",
    );

    let content = serde_yaml::to_string(&state).context("Failed to serialize state.yaml")?;
    fs::write(&state_path, content)
        .with_context(|| format!("Failed to write {}", state_path.display()))
}

fn set_string(value: &mut Value, path: &[&str], next: &str) {
    set_value(value, path, Value::String(next.to_string()));
}

fn set_bool(value: &mut Value, path: &[&str], next: bool) {
    set_value(value, path, Value::Bool(next));
}

fn set_null(value: &mut Value, path: &[&str]) {
    set_value(value, path, Value::Null);
}

fn add_active_task(value: &mut Value, task_id: &str) {
    let mut active = value
        .get("tasks")
        .and_then(|tasks| tasks.get("active"))
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();
    if !active.iter().any(|entry| entry.as_str() == Some(task_id)) {
        active.push(Value::String(task_id.to_string()));
    }
    set_value(value, &["tasks", "active"], Value::Sequence(active));
}

fn set_value(value: &mut Value, path: &[&str], next: Value) {
    if path.is_empty() {
        *value = next;
        return;
    }
    if !matches!(value, Value::Mapping(_)) {
        *value = Value::Mapping(Mapping::new());
    }
    let mut current = value;
    for key in &path[..path.len() - 1] {
        let mapping = current.as_mapping_mut().expect("mapping value");
        current = mapping
            .entry(Value::String((*key).to_string()))
            .or_insert_with(|| Value::Mapping(Mapping::new()));
        if !matches!(current, Value::Mapping(_)) {
            *current = Value::Mapping(Mapping::new());
        }
    }
    let mapping = current.as_mapping_mut().expect("mapping value");
    mapping.insert(Value::String(path[path.len() - 1].to_string()), next);
}

fn task_yaml(
    task_id: &str,
    title: &str,
    mode: &str,
    phase: &str,
    phase_status: &str,
    created_at: &str,
) -> String {
    format!(
        r#"schema_version: 1
kind: vibehub_task
task_id: {}
title: {}
mode: {}
phase: {}
phase_status: {}
created_at: {}
created_by: vibehub
"#,
        yaml_string(task_id),
        yaml_string(title),
        yaml_string(mode),
        yaml_string(phase),
        yaml_string(phase_status),
        yaml_string(created_at)
    )
}

fn run_yaml(
    task_id: &str,
    run_id: &str,
    mode: &str,
    phase: &str,
    phase_status: &str,
    created_at: &str,
) -> String {
    format!(
        r#"schema_version: 1
kind: vibehub_run
task_id: {}
run_id: {}
mode: {}
phase: {}
phase_status: {}
created_at: {}
created_by: vibehub
baseline_commit: null
"#,
        yaml_string(task_id),
        yaml_string(run_id),
        yaml_string(mode),
        yaml_string(phase),
        yaml_string(phase_status),
        yaml_string(created_at)
    )
}

pub fn ensure_context_spec(
    project_root: impl AsRef<Path>,
    task_id: &str,
    run_id: &str,
    phase: &str,
) -> Result<PathBuf> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    let task_id = validate_id("task_id", task_id)?;
    let run_id = validate_id("run_id", run_id)?;
    let phase = validate_id("phase", phase)?;
    let context_dir = project_root
        .join(".vibehub")
        .join("tasks")
        .join(task_id)
        .join("context");
    fs::create_dir_all(&context_dir)
        .with_context(|| format!("Failed to create {}", context_dir.display()))?;
    let context_spec_path = context_dir.join(format!("{phase}.yaml"));
    if !context_spec_path.is_file() {
        fs::write(
            &context_spec_path,
            context_spec_yaml(task_id, run_id, phase),
        )
        .with_context(|| format!("Failed to write {}", context_spec_path.display()))?;
    }
    Ok(context_spec_path)
}

fn context_spec_yaml(task_id: &str, run_id: &str, phase: &str) -> String {
    format!(
        r#"phase: {}
task_id: {}
run_id: {}
known_missing_context: []
stop_condition: Write output.md and return to VibeHub for validation.
entries:
  - path: .vibehub/tasks/{}/task.yaml
    type: file
    reason: active task metadata and goal
    required: true
  - path: .vibehub/tasks/{}/runs/{}/run.yaml
    type: file
    reason: active run metadata
    required: true
  - path: .vibehub/rules/hard-rules.md
    type: file
    reason: protocol hard rules
    required: true
"#,
        yaml_string(phase),
        yaml_string(task_id),
        yaml_string(run_id),
        task_id,
        task_id,
        run_id
    )
}

fn next_id(prefix: &str) -> String {
    let timestamp = Utc::now().format("%Y%m%d%H%M%S");
    let suffix = Uuid::new_v4().simple().to_string();
    format!("{prefix}-{timestamp}-{}", &suffix[..8])
}

fn validate_id<'a>(name: &str, value: &'a str) -> Result<&'a str> {
    let value = value.trim();
    if value.is_empty() {
        return Err(anyhow!("Invalid {}: value is empty", name));
    }
    if value.contains('/') || value.contains('\\') {
        return Err(anyhow!(
            "Invalid {} '{}': path separators are not allowed",
            name,
            value
        ));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vibehub::{agent_view, context, handoff, review, status};

    fn temp_project() -> PathBuf {
        let path = std::env::temp_dir().join(format!("vibehub-start-task-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).expect("create temp project");
        path
    }

    #[test]
    fn golden_path_init_start_context_agent_view_review_and_handoff() {
        let project = temp_project();

        init::init_project(&project).expect("init");
        let started = start_task(
            &project,
            Some("Backend P0 file protocol bootstrap".to_string()),
            None,
            None,
        )
        .expect("start task");

        assert!(project.join(&started.task_path).is_dir());
        assert!(project.join(&started.run_path).is_dir());
        assert!(project.join(&started.context_spec_path).is_file());
        assert!(
            project.join(&started.context_pack_path).is_file(),
            "context pack should exist after start_task"
        );
        assert!(
            project.join(&started.context_manifest_path).is_file(),
            "context manifest should exist after start_task"
        );
        assert_eq!(started.context_included_count, 3);
        assert_eq!(started.context_missing_count, 0);

        let status_after_start = status::read_cockpit_status(&project).expect("status");
        assert_eq!(
            status_after_start.current_task_id.as_deref(),
            Some(started.task_id.as_str())
        );
        assert_eq!(
            status_after_start.current_run_id.as_deref(),
            Some(started.run_id.as_str())
        );
        assert_eq!(
            status_after_start.current_mode.as_deref(),
            Some("guided_drive")
        );
        assert_eq!(status_after_start.current_phase.as_deref(), Some("align"));
        assert_eq!(status_after_start.phase_status.as_deref(), Some("active"));

        let pack = context::build_context_pack(
            &project,
            &started.task_id,
            &started.run_id,
            &started.phase,
        )
        .expect("build context");
        assert_eq!(pack.missing_count, 0);
        assert!(project.join(&pack.pack_path).is_file());
        assert!(project.join(&pack.manifest_path).is_file());

        let agent_view = agent_view::generate_agent_view(&project).expect("agent view");
        assert_eq!(agent_view.task_id, started.task_id);
        assert_eq!(agent_view.run_id, started.run_id);
        assert_eq!(agent_view.phase, "align");
        assert!(project.join(&agent_view.current_path).is_file());
        assert!(project.join(&agent_view.current_context_path).is_file());

        let session_dir = project
            .join(&started.run_path)
            .join("sessions")
            .join("S-001");
        fs::create_dir_all(&session_dir).expect("create session");
        fs::write(
            session_dir.join("output.md"),
            r#"# Session Output

## Completed
- Implemented backend P0 start task.

## Not Yet Done
- Frontend cockpit UI is out of scope.

## Key Decisions Made
- Use VibeHub-generated YAML pointers as canonical current state.

## Files Changed
- src-tauri/src/vibehub/start_task.rs

## Files Reportedly Read
- .vibehub/agent-view/current.md

## Context Still Needed
- None.

## Warnings
- Runtime observation is not enabled in P0.

## Commands Run
- cargo test vibehub::start_task

## Tests Run
- cargo test vibehub::start_task

## Next Session Should
- Run final verification.
"#,
        )
        .expect("write output");

        let review = review::generate_review_evidence(&project).expect("review");
        assert!(project.join(&review.review_path).is_file());
        assert_eq!(review.task_id, started.task_id);
        assert_eq!(review.run_id, started.run_id);

        let handoff = handoff::build_handoff(&project).expect("handoff");
        assert!(project.join(&handoff.handoff_path).is_file());
        assert_eq!(handoff.task_id, started.task_id);
        assert_eq!(handoff.run_id, started.run_id);
        assert!(handoff.complete);

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn v3_projects_reject_legacy_start_before_writing_legacy_task_metadata() {
        let project = temp_project();
        crate::v3::initialize_v3(&project).expect("initialize v3");

        let error = start_task(
            &project,
            Some("Must not become a legacy task".to_owned()),
            Some("evidence_drive".to_owned()),
            None,
        )
        .expect_err("legacy start must be rejected in a V3 project");

        assert!(error.to_string().contains("V3_LEGACY_START_UNSUPPORTED"));
        assert!(error
            .to_string()
            .contains("vibehub v3 <project> task-create"));
        assert!(
            !project.join(".vibehub/tasks").exists(),
            "the rejection must happen before any legacy task path is created"
        );

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn completes_partial_vibehub_init_before_starting_task() {
        let project = temp_project();
        fs::create_dir_all(project.join(".vibehub/tasks")).expect("create partial vibehub");

        let started = start_task(
            &project,
            Some("Partial init recovery".to_string()),
            None,
            None,
        )
        .expect("start task");

        assert!(project.join(".vibehub/rules/hard-rules.md").is_file());
        assert!(project.join(&started.context_pack_path).is_file());
        assert!(project.join(&started.context_manifest_path).is_file());
        assert_eq!(started.context_missing_count, 0);

        let pack = context::build_context_pack(
            &project,
            &started.task_id,
            &started.run_id,
            &started.phase,
        )
        .expect("rebuild context");
        assert_eq!(pack.missing_count, 0);

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn start_task_auto_archives_completed_active_tasks() {
        let project = temp_project();
        let first = start_task(&project, Some("Completed task".to_string()), None, None)
            .expect("first task");
        fs::write(
            project.join(&first.task_path).join("task.yaml"),
            format!(
                "task_id: {}\ntitle: Completed task\nmode: {}\nphase: review\nphase_status: completed\n",
                first.task_id, first.mode
            ),
        )
        .expect("complete task yaml");
        fs::write(
            project.join(&first.run_path).join("run.yaml"),
            format!(
                "task_id: {}\nrun_id: {}\nmode: {}\nphase: review\nphase_status: completed\n",
                first.task_id, first.run_id, first.mode
            ),
        )
        .expect("complete run yaml");

        let second =
            start_task(&project, Some("Fresh task".to_string()), None, None).expect("second task");

        let state_content =
            fs::read_to_string(project.join(".vibehub/state.yaml")).expect("read state");
        let state: Value = serde_yaml::from_str(&state_content).expect("parse state");
        assert_eq!(
            active_task_ids_from_value(&state),
            vec![second.task_id.clone()]
        );
        assert_eq!(
            state
                .get("current")
                .and_then(|current| current.get("task_id"))
                .and_then(Value::as_str),
            Some(second.task_id.as_str())
        );
        assert!(!project
            .join(&first.run_path)
            .parent()
            .unwrap()
            .join("current")
            .exists());

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn context_pack_files_exist_immediately_after_start_task() {
        let project = temp_project();

        let started = start_task(
            &project,
            Some("Auto-build context pack verification".to_string()),
            None,
            Some("implement".to_string()),
        )
        .expect("start task");

        assert!(
            project.join(&started.context_pack_path).is_file(),
            "Expected context pack at {}",
            started.context_pack_path
        );
        assert!(
            project.join(&started.context_manifest_path).is_file(),
            "Expected manifest at {}",
            started.context_manifest_path
        );
        assert_eq!(started.context_included_count, 3);
        assert_eq!(started.context_missing_count, 0);

        let pack_content =
            fs::read_to_string(project.join(&started.context_pack_path)).expect("read pack");
        assert!(pack_content.contains("# Context Pack: Implement"));
        assert!(pack_content.contains(&format!("Task: {}", started.task_id)));
        assert!(pack_content.contains(&format!("Run: {}", started.run_id)));
        assert!(pack_content.contains("## File: .vibehub/rules/hard-rules.md"));
        assert!(pack_content.contains("## Stop Condition"));
        assert!(!pack_content.contains("## File: .env"));

        let manifest_content = fs::read_to_string(project.join(&started.context_manifest_path))
            .expect("read manifest");
        assert!(manifest_content.contains("evidence_grade: hard_observed"));
        assert!(manifest_content.contains(&format!("task_id: {}", started.task_id)));

        let state_content =
            fs::read_to_string(project.join(".vibehub/state.yaml")).expect("read state");
        assert!(state_content.contains("align_lite: pending"));
        assert!(state_content.contains("review_lite: pending"));
        let state_pack_path = state_content
            .lines()
            .find(|line| line.trim().starts_with("current_pack:"))
            .and_then(|line| line.split(':').nth(1))
            .map(str::trim)
            .map(|s| s.trim_matches('"'));
        assert_eq!(
            state_pack_path,
            Some(started.context_pack_path.as_str()),
            "state.yaml current_pack must match actual pack path"
        );

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn high_confidence_intake_creates_multiple_tasks_and_queue() {
        let project = temp_project();
        let result = start_task_intake(
            &project,
            VibehubStartTaskIntakeRequest {
                title: None,
                intent: None,
                mode: Some("evidence_drive".to_string()),
                phase: Some("align".to_string()),
                source_message: Some(
                    "Fix login, add remote settings, and update docs afterwards".to_string(),
                ),
                split_confidence: Some(VibehubIntakeConfidence::High),
                split_reason: Some("Three independently deliverable outcomes.".to_string()),
                not_split_reason: None,
                intake: vec![
                    VibehubTaskDraft {
                        title: "Fix login error".to_string(),
                        intent: Some("Resolve the login failure.".to_string()),
                        acceptance_criteria: vec!["Login succeeds".to_string()],
                        dependencies: Vec::new(),
                        suggested_order: Some(1),
                        mode: None,
                        phase: None,
                    },
                    VibehubTaskDraft {
                        title: "Add remote URL setting".to_string(),
                        intent: Some("Let projects store a remote URL.".to_string()),
                        acceptance_criteria: vec!["Setting is visible".to_string()],
                        dependencies: Vec::new(),
                        suggested_order: Some(2),
                        mode: None,
                        phase: None,
                    },
                    VibehubTaskDraft {
                        title: "Update VibeHub docs".to_string(),
                        intent: Some("Document the new behavior.".to_string()),
                        acceptance_criteria: vec!["Docs mention the setting".to_string()],
                        dependencies: vec![
                            "Fix login error".to_string(),
                            "Add remote URL setting".to_string(),
                        ],
                        suggested_order: Some(3),
                        mode: None,
                        phase: None,
                    },
                ],
            },
        )
        .expect("multi-intent start");

        assert_eq!(result.classification, "created_multi_task_intake");
        assert_eq!(result.created_tasks.len(), 3);
        assert!(result.failed_tasks.is_empty());
        assert_eq!(
            result.current_task_id.as_deref(),
            Some(result.created_tasks[0].task_id.as_str())
        );
        assert_eq!(
            result.active_tasks[..3],
            result
                .created_tasks
                .iter()
                .map(|task| task.task_id.clone())
                .collect::<Vec<_>>()
        );

        let state: Value = serde_yaml::from_str(
            &fs::read_to_string(project.join(".vibehub/state.yaml")).expect("read state"),
        )
        .expect("state yaml");
        assert_eq!(
            state
                .get("tasks")
                .and_then(|tasks| tasks.get("intake_queue"))
                .and_then(|queue| queue.get("split_confidence"))
                .and_then(Value::as_str),
            Some("high")
        );
        assert_eq!(
            state
                .get("tasks")
                .and_then(|tasks| tasks.get("intake_queue"))
                .and_then(|queue| queue.get("items"))
                .and_then(Value::as_sequence)
                .map(Vec::len),
            Some(3)
        );

        let current_view =
            fs::read_to_string(project.join(".vibehub/agent-view/current.md")).expect("agent view");
        assert!(current_view.contains("## Intake Queue"));
        assert!(current_view.contains("Remaining order:"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn medium_confidence_intake_requests_confirmation_without_creating_tasks() {
        let project = temp_project();
        init::init_project(&project).expect("init");

        let result = start_task_intake(
            &project,
            VibehubStartTaskIntakeRequest {
                title: None,
                intent: None,
                mode: Some("guided_drive".to_string()),
                phase: None,
                source_message: Some("Maybe fix docs and maybe refactor UI".to_string()),
                split_confidence: Some(VibehubIntakeConfidence::Medium),
                split_reason: Some(
                    "The split depends on whether the doc work is separate.".to_string(),
                ),
                not_split_reason: None,
                intake: vec![
                    VibehubTaskDraft {
                        title: "Fix docs".to_string(),
                        intent: None,
                        acceptance_criteria: Vec::new(),
                        dependencies: Vec::new(),
                        suggested_order: Some(1),
                        mode: None,
                        phase: None,
                    },
                    VibehubTaskDraft {
                        title: "Refactor UI".to_string(),
                        intent: None,
                        acceptance_criteria: Vec::new(),
                        dependencies: Vec::new(),
                        suggested_order: Some(2),
                        mode: None,
                        phase: None,
                    },
                ],
            },
        )
        .expect("medium intake");

        assert!(result.confirmation_required);
        assert_eq!(result.classification, "confirm_split");
        assert!(result.created_tasks.is_empty());
        assert_eq!(result.proposed_tasks.len(), 2);
        assert!(!project.join(".vibehub/tasks/current").exists());

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn low_confidence_intake_keeps_one_task_and_records_reason() {
        let project = temp_project();
        let result = start_task_intake(
            &project,
            VibehubStartTaskIntakeRequest {
                title: Some("Polish settings workflow".to_string()),
                intent: Some("Handle related settings cleanup as one acceptance unit.".to_string()),
                mode: None,
                phase: None,
                source_message: Some("Polish settings, tests, and docs".to_string()),
                split_confidence: Some(VibehubIntakeConfidence::Low),
                split_reason: None,
                not_split_reason: Some(
                    "Tests and docs are acceptance work for the same user goal.".to_string(),
                ),
                intake: vec![
                    VibehubTaskDraft {
                        title: "Polish settings".to_string(),
                        intent: None,
                        acceptance_criteria: vec!["Settings flow works".to_string()],
                        dependencies: Vec::new(),
                        suggested_order: Some(1),
                        mode: None,
                        phase: None,
                    },
                    VibehubTaskDraft {
                        title: "Update docs".to_string(),
                        intent: None,
                        acceptance_criteria: vec!["Docs updated".to_string()],
                        dependencies: Vec::new(),
                        suggested_order: Some(2),
                        mode: None,
                        phase: None,
                    },
                ],
            },
        )
        .expect("low confidence intake");

        assert_eq!(
            result.classification,
            "created_single_task_with_not_split_reason"
        );
        assert_eq!(result.created_tasks.len(), 1);
        assert_eq!(
            result.not_split_reason.as_deref(),
            Some("Tests and docs are acceptance work for the same user goal.")
        );
        let task_yaml = fs::read_to_string(
            project
                .join(".vibehub/tasks")
                .join(&result.created_tasks[0].task_id)
                .join("task.yaml"),
        )
        .expect("read task yaml");
        assert!(task_yaml.contains("not_split_reason"));

        fs::remove_dir_all(project).expect("cleanup");
    }
}

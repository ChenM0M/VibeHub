use crate::vibehub::util::canonical_initialized_project_root;
use crate::vibehub::{context, current, events, policy, start_task, workflow};
use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CapabilityClaimResult {
    pub task_id: String,
    pub run_id: String,
    pub capability: String,
    pub claimable_capabilities: Vec<String>,
    pub checked_gates: Vec<GateEvaluation>,
    pub context_pack_path: String,
    pub context_manifest_path: String,
    pub event_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CapabilityGateReport {
    pub task_id: String,
    pub run_id: String,
    pub mode: String,
    pub capabilities: Vec<CapabilityGateStatus>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CapabilityGateStatus {
    pub capability: String,
    pub claimable: bool,
    pub gates: Vec<GateEvaluation>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GateEvaluation {
    pub gate: String,
    pub result: bool,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GatePreconditionError {
    pub code: String,
    pub message: String,
    pub hint: String,
    pub capability: String,
    pub gates: Vec<GateEvaluation>,
}

impl GatePreconditionError {
    fn unmet(capability: &str, gates: Vec<GateEvaluation>) -> Self {
        let reasons = gates
            .iter()
            .filter(|gate| !gate.result)
            .flat_map(|gate| gate.reasons.clone())
            .collect::<Vec<_>>();
        Self {
            code: "gate.precondition.unmet".to_string(),
            message: format!("Capability '{capability}' is not currently claimable."),
            hint: if reasons.is_empty() {
                "Run vibehub-status or vibehub-gates to inspect unmet gates.".to_string()
            } else {
                format!("Resolve unmet gate reasons: {}", reasons.join("; "))
            },
            capability: capability.to_string(),
            gates,
        }
    }
}

impl fmt::Display for GatePreconditionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let json = serde_json::to_string(self).unwrap_or_else(|_| self.message.clone());
        write!(f, "{json}")
    }
}

impl std::error::Error for GatePreconditionError {}

#[derive(Debug, Default)]
struct EventFacts {
    artifacts: BTreeSet<String>,
    active_capabilities: BTreeSet<String>,
    open_risks: BTreeMap<String, String>,
    unsynced_handoff: bool,
}

pub fn claim_capability(
    project_root: impl AsRef<Path>,
    capability: impl AsRef<str>,
) -> Result<CapabilityClaimResult> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let capability = validate_id("capability", capability.as_ref())?;
    let (task_id, run_id, mode) = current_run_context(&project_root)?;
    let workflow = workflow::read_workflow_file(&project_root)?;
    let policy = policy::read_policy(&project_root)?;
    let events = events::list_events(&project_root, &task_id, &run_id, None)?;
    let facts = fold_event_facts(&events, &workflow);
    let mode_capabilities = mode_capabilities(&workflow, &mode)?;

    if !mode_capabilities.contains(&capability) {
        bail!(
            "Capability '{}' is not defined for mode '{}' in workflow.yaml",
            capability,
            mode
        );
    }

    let report = evaluate_capabilities_from_parts(
        &task_id,
        &run_id,
        &mode,
        &workflow,
        &mode_capabilities,
        &facts,
        &policy,
        Some(&capability),
    );
    let status = report
        .capabilities
        .iter()
        .find(|status| status.capability == capability)
        .cloned()
        .ok_or_else(|| anyhow!("Capability '{}' was not evaluated", capability))?;
    if !status.claimable {
        return Err(GatePreconditionError::unmet(&capability, status.gates).into());
    }

    start_task::ensure_context_spec(&project_root, &task_id, &run_id, &capability)?;
    let pack = context::build_context_pack(&project_root, &task_id, &run_id, &capability)
        .with_context(|| format!("Failed to build context pack for capability '{capability}'"))?;

    let mut event_ids = Vec::new();
    for gate in &status.gates {
        let event = events::append_structured_run_event(
            &project_root,
            &task_id,
            &run_id,
            events::VibehubEvent::GateChecked {
                gate: gate.gate.clone(),
                result: gate.result,
                reasons: gate.reasons.clone(),
            },
        )?;
        event_ids.push(event.event_id);
    }
    let claim_event = events::append_structured_run_event(
        &project_root,
        &task_id,
        &run_id,
        events::VibehubEvent::CapabilityClaimed {
            capability: capability.clone(),
        },
    )?;
    event_ids.push(claim_event.event_id);
    let pack_event = events::append_structured_run_event(
        &project_root,
        &task_id,
        &run_id,
        events::VibehubEvent::CapabilityPackBuilt {
            capability: capability.clone(),
            pack_path: pack.pack_path.clone(),
            size_tokens: pack.estimated_tokens as u64,
        },
    )?;
    event_ids.push(pack_event.event_id);
    if pack.estimated_tokens > policy.pack.warn_at {
        let oversize_event = events::append_structured_run_event(
            &project_root,
            &task_id,
            &run_id,
            events::VibehubEvent::PackOversize {
                kind: "capability".to_string(),
                size: pack.estimated_tokens as u64,
                threshold: policy.pack.warn_at as u64,
            },
        )?;
        event_ids.push(oversize_event.event_id);
    }

    Ok(CapabilityClaimResult {
        task_id,
        run_id,
        capability,
        claimable_capabilities: report
            .capabilities
            .into_iter()
            .filter(|status| status.claimable)
            .map(|status| status.capability)
            .collect(),
        checked_gates: status.gates,
        context_pack_path: pack.pack_path,
        context_manifest_path: pack.manifest_path,
        event_ids,
    })
}

pub fn evaluate_capability_gates(
    project_root: impl AsRef<Path>,
    requested_capability: Option<&str>,
) -> Result<CapabilityGateReport> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let requested_capability = match requested_capability {
        Some(value) => Some(validate_id("capability", value)?),
        None => None,
    };
    let (task_id, run_id, mode) = current_run_context(&project_root)?;
    let workflow = workflow::read_workflow_file(&project_root)?;
    let policy = policy::read_policy(&project_root)?;
    let events = events::list_events(&project_root, &task_id, &run_id, None)?;
    let facts = fold_event_facts(&events, &workflow);
    let mode_capabilities = mode_capabilities(&workflow, &mode)?;
    Ok(evaluate_capabilities_from_parts(
        &task_id,
        &run_id,
        &mode,
        &workflow,
        &mode_capabilities,
        &facts,
        &policy,
        requested_capability.as_deref(),
    ))
}

fn evaluate_capabilities_from_parts(
    task_id: &str,
    run_id: &str,
    mode: &str,
    workflow: &workflow::WorkflowConfig,
    capabilities: &[String],
    facts: &EventFacts,
    policy: &policy::VibehubPolicy,
    requested_capability: Option<&str>,
) -> CapabilityGateReport {
    let capabilities = capabilities
        .iter()
        .map(|capability| {
            let gates = workflow::capability_definition(workflow, capability)
                .map(|definition| definition.gates.as_slice())
                .unwrap_or(&[])
                .iter()
                .filter_map(|gate_name| {
                    let expression = workflow.gates.get(gate_name)?;
                    let (result, reasons) =
                        evaluate_gate_expression(expression, facts, requested_capability);
                    Some(GateEvaluation {
                        gate: gate_name.clone(),
                        result,
                        reasons,
                    })
                })
                .collect::<Vec<_>>();
            let mut gates = gates;
            if facts.active_capabilities.contains(capability) {
                gates.push(GateEvaluation {
                    gate: "active_claim".to_string(),
                    result: false,
                    reasons: vec![format!("capability {capability} is already active")],
                });
            } else if facts.active_capabilities.len() >= policy.max_concurrent_claims {
                gates.push(GateEvaluation {
                    gate: "wip_limit".to_string(),
                    result: false,
                    reasons: vec![format!(
                        "active capability limit reached ({}/{})",
                        facts.active_capabilities.len(),
                        policy.max_concurrent_claims
                    )],
                });
            }
            if facts.open_risks.len() > policy.max_open_risks {
                gates.push(GateEvaluation {
                    gate: "open_risk_limit".to_string(),
                    result: false,
                    reasons: vec![format!(
                        "open risk limit exceeded ({}/{})",
                        facts.open_risks.len(),
                        policy.max_open_risks
                    )],
                });
            }
            let claimable = gates.iter().all(|gate| gate.result);
            CapabilityGateStatus {
                capability: capability.clone(),
                claimable,
                gates,
            }
        })
        .collect();
    CapabilityGateReport {
        task_id: task_id.to_string(),
        run_id: run_id.to_string(),
        mode: mode.to_string(),
        capabilities,
    }
}

fn evaluate_gate_expression(
    expression: &YamlValue,
    facts: &EventFacts,
    requested_capability: Option<&str>,
) -> (bool, Vec<String>) {
    match expression {
        YamlValue::String(predicate) => {
            evaluate_predicate(predicate, None, facts, requested_capability)
        }
        YamlValue::Mapping(map) => {
            if let Some(children) = map.get(YamlValue::String("all".to_string())) {
                let mut reasons = Vec::new();
                let mut result = true;
                for child in children.as_sequence().into_iter().flatten() {
                    let (child_result, child_reasons) =
                        evaluate_gate_expression(child, facts, requested_capability);
                    result &= child_result;
                    reasons.extend(child_reasons);
                }
                return (result, reasons);
            }
            if let Some(children) = map.get(YamlValue::String("any".to_string())) {
                let mut reasons = Vec::new();
                let mut result = false;
                for child in children.as_sequence().into_iter().flatten() {
                    let (child_result, child_reasons) =
                        evaluate_gate_expression(child, facts, requested_capability);
                    result |= child_result;
                    reasons.extend(child_reasons);
                }
                return (result, reasons);
            }
            if let Some(child) = map.get(YamlValue::String("not".to_string())) {
                let (child_result, child_reasons) =
                    evaluate_gate_expression(child, facts, requested_capability);
                return (!child_result, child_reasons);
            }
            let Some((key, value)) = map.iter().next() else {
                return (false, vec!["gate expression is empty".to_string()]);
            };
            let Some(predicate) = key.as_str() else {
                return (
                    false,
                    vec!["gate predicate key is not a string".to_string()],
                );
            };
            evaluate_predicate(predicate, Some(value), facts, requested_capability)
        }
        _ => (
            false,
            vec!["gate expression has unsupported value type".to_string()],
        ),
    }
}

fn evaluate_predicate(
    predicate: &str,
    argument: Option<&YamlValue>,
    facts: &EventFacts,
    requested_capability: Option<&str>,
) -> (bool, Vec<String>) {
    match predicate {
        "has_artifact" => {
            let artifact = yaml_scalar_to_string(argument);
            let result = artifact
                .as_deref()
                .map(|artifact| facts.artifacts.contains(artifact))
                .unwrap_or(false);
            (
                result,
                vec![match (result, artifact) {
                    (true, Some(artifact)) => format!("has_artifact:{artifact} satisfied"),
                    (false, Some(artifact)) => format!("missing artifact {artifact}"),
                    (false, None) => "has_artifact missing artifact name".to_string(),
                    (true, None) => "has_artifact satisfied".to_string(),
                }],
            )
        }
        "explicit_request" => {
            let expected = yaml_scalar_to_string(argument);
            let result = expected.as_deref() == requested_capability;
            (
                result,
                vec![match (result, expected) {
                    (true, Some(expected)) => format!("explicit_request:{expected} satisfied"),
                    (false, Some(expected)) => format!("explicit request is not {expected}"),
                    (false, None) => "explicit_request missing capability name".to_string(),
                    (true, None) => "explicit_request satisfied".to_string(),
                }],
            )
        }
        "open_risk" => {
            let severity = yaml_scalar_to_string(argument);
            let result = match severity.as_deref() {
                Some(severity) if severity != "true" => {
                    facts.open_risks.values().any(|risk| risk == severity)
                }
                _ => !facts.open_risks.is_empty(),
            };
            (
                result,
                vec![if result {
                    format!("open risk present ({})", facts.open_risks.len())
                } else {
                    "no matching open risk".to_string()
                }],
            )
        }
        "unsynced_handoff" => {
            let expected = yaml_bool(argument).unwrap_or(true);
            let result = facts.unsynced_handoff == expected;
            (
                result,
                vec![if facts.unsynced_handoff {
                    "unsynced handoff present".to_string()
                } else {
                    "no unsynced handoff".to_string()
                }],
            )
        }
        "no_open_risk" => {
            let result = facts.open_risks.is_empty();
            (
                result,
                vec![if result {
                    "no open risk".to_string()
                } else {
                    format!("open risk present ({})", facts.open_risks.len())
                }],
            )
        }
        "no_unsynced_handoff" => {
            let result = !facts.unsynced_handoff;
            (
                result,
                vec![if result {
                    "no unsynced handoff".to_string()
                } else {
                    "unsynced handoff present".to_string()
                }],
            )
        }
        other => (false, vec![format!("unsupported predicate {other}")]),
    }
}

fn fold_event_facts(
    events: &[events::StoredEvent],
    workflow: &workflow::WorkflowConfig,
) -> EventFacts {
    let mut facts = EventFacts::default();
    for stored in events {
        let Some(event_type) = stored.event.get("event_type").and_then(JsonValue::as_str) else {
            continue;
        };
        let payload = stored.event.get("payload").unwrap_or(&JsonValue::Null);
        match event_type {
            "CapabilityClaimed" => {
                if let Some(capability) = payload.get("capability").and_then(JsonValue::as_str) {
                    facts.active_capabilities.insert(capability.to_string());
                }
            }
            "CapabilityReleased" => {
                let capability = payload.get("capability").and_then(JsonValue::as_str);
                let outcome = payload.get("outcome").and_then(JsonValue::as_str);
                if let Some(capability) = capability {
                    facts.active_capabilities.remove(capability);
                }
                if outcome == Some("completed") {
                    if let Some(capability) = capability {
                        add_capability_artifacts(&mut facts, workflow, capability);
                    }
                }
            }
            "EvidenceAdded" => {
                facts.artifacts.insert("research_output".to_string());
            }
            "PlanDrafted" => {
                facts.artifacts.insert("implementation_plan".to_string());
            }
            "DiffObserved" => {
                facts.artifacts.insert("diff".to_string());
            }
            "ValidationRun" => {
                let status = payload
                    .get("status")
                    .and_then(JsonValue::as_str)
                    .unwrap_or_default();
                if matches!(status, "passed" | "pass" | "success" | "ok") {
                    facts.artifacts.insert("validation_result".to_string());
                    facts.artifacts.insert("validation_passed".to_string());
                }
            }
            "RiskRaised" => {
                if let Some(id) = payload.get("id").and_then(JsonValue::as_str) {
                    let severity = payload
                        .get("severity")
                        .and_then(JsonValue::as_str)
                        .unwrap_or("unknown");
                    facts
                        .open_risks
                        .insert(id.to_string(), severity.to_string());
                }
            }
            "RiskResolved" => {
                if let Some(id) = payload.get("id").and_then(JsonValue::as_str) {
                    facts.open_risks.remove(id);
                }
            }
            "HandoffWritten" => {
                facts.unsynced_handoff = true;
            }
            "SyncCompleted" => {
                facts.unsynced_handoff = false;
            }
            "PlanInvalidated" => {
                facts.artifacts.remove("implementation_plan");
            }
            "DiffReverted" => {
                facts.artifacts.remove("diff");
            }
            "Legacy" => fold_legacy_event(&mut facts, workflow, payload),
            _ => {}
        }
    }
    facts
}

fn fold_legacy_event(
    facts: &mut EventFacts,
    workflow: &workflow::WorkflowConfig,
    payload: &JsonValue,
) {
    let legacy_type = payload
        .get("legacy_event_type")
        .and_then(JsonValue::as_str)
        .unwrap_or_default();
    let details = payload.get("details").unwrap_or(&JsonValue::Null);
    match legacy_type {
        "phase_completed" | "phase_status_set" => {
            let phase = details.get("phase").and_then(JsonValue::as_str);
            let status = details.get("status").and_then(JsonValue::as_str);
            if let Some(phase) = phase {
                match status {
                    Some("active") => {
                        facts.active_capabilities.insert(phase.to_string());
                    }
                    Some(_) => {
                        facts.active_capabilities.remove(phase);
                    }
                    None => {}
                }
            }
            if status == Some("completed") {
                if let Some(phase) = phase {
                    add_capability_artifacts(facts, workflow, phase);
                }
            }
        }
        "review_evidence_generated" => {
            facts.artifacts.insert("review_summary".to_string());
        }
        _ => {}
    }
}

fn add_capability_artifacts(
    facts: &mut EventFacts,
    workflow: &workflow::WorkflowConfig,
    capability: &str,
) {
    if let Some(definition) = workflow::capability_definition(workflow, capability) {
        facts.artifacts.extend(definition.produces.iter().cloned());
    }
}

fn current_run_context(project_root: &Path) -> Result<(String, String, String)> {
    let task = current::resolve_current_task(project_root)?;
    let run = current::resolve_current_run(project_root, &task.task_id)?;
    let mode = read_current_mode(project_root)?.ok_or_else(|| {
        anyhow!("No current mode in .vibehub/state.yaml; cannot evaluate capability gates")
    })?;
    Ok((task.task_id, run.run_id, mode))
}

fn read_current_mode(project_root: &Path) -> Result<Option<String>> {
    let state_path = project_root.join(".vibehub/state.yaml");
    if !state_path.is_file() {
        return Ok(None);
    }
    let content = fs::read_to_string(&state_path)
        .with_context(|| format!("Failed to read {}", state_path.display()))?;
    let state: YamlValue = serde_yaml::from_str(&content)
        .with_context(|| format!("Invalid YAML in {}", state_path.display()))?;
    Ok(state
        .get("current")
        .and_then(|current| current.get("mode"))
        .and_then(YamlValue::as_str)
        .map(ToString::to_string))
}

fn mode_capabilities(workflow: &workflow::WorkflowConfig, mode: &str) -> Result<Vec<String>> {
    if let Some(mode_entry) = workflow.modes.get(mode) {
        if !mode_entry.capabilities.is_empty() {
            return Ok(mode_entry.capabilities.clone());
        }
        if !mode_entry.phases.is_empty() {
            return Ok(mode_entry.phases.clone());
        }
    }
    if !workflow.phase_order.is_empty() {
        return Ok(workflow.phase_order.clone());
    }
    Err(anyhow!(
        "No capabilities defined in workflow.yaml for mode '{}'",
        mode
    ))
}

fn yaml_scalar_to_string(value: Option<&YamlValue>) -> Option<String> {
    match value? {
        YamlValue::String(value) => Some(value.clone()),
        YamlValue::Bool(value) => Some(value.to_string()),
        YamlValue::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn yaml_bool(value: Option<&YamlValue>) -> Option<bool> {
    match value? {
        YamlValue::Bool(value) => Some(*value),
        YamlValue::String(value) => value.parse().ok(),
        _ => None,
    }
}

fn validate_id<'a>(field: &str, value: &'a str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("{field} cannot be empty"));
    }
    if trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed == "."
        || trimmed == ".."
        || trimmed.contains("..")
    {
        return Err(anyhow!("{field} contains unsafe path characters"));
    }
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vibehub::{handoff, schema_check};
    use std::collections::BTreeSet;
    use std::path::PathBuf;
    use uuid::Uuid;

    const WORKFLOW: &str = r#"schema_version: 2
name: test
modes:
  evidence_drive:
    phases: [align, research, plan, implement, review]
    capabilities: [align, research, plan, implement, review]
default_mode: evidence_drive
phase_order: [align, research, plan, implement, review]
capabilities:
  align:
    produces: [alignment_summary]
  research:
    produces: [research_output]
  plan:
    produces: [implementation_plan]
    gates: [plan_ready]
  implement:
    produces: [diff]
    gates: [implement_ready]
  review:
    produces: [review_summary]
    gates: [review_ready]
gates:
  plan_ready:
    any:
      - has_artifact: alignment_summary
      - explicit_request: plan
  implement_ready:
    all:
      - has_artifact: implementation_plan
      - not:
          open_risk: blocking
  review_ready:
    any:
      - has_artifact: diff
      - all:
          - explicit_request: review
          - not:
              unsynced_handoff: true
"#;

    const CUSTOM_WORKFLOW: &str = r#"schema_version: 2
name: custom-test
modes:
  evidence_drive:
    capabilities: [align, security_audit]
default_mode: evidence_drive
capabilities:
  align:
    produces: [alignment_summary]
custom_capabilities:
  security_audit:
    required_fields: [audit_scope, "findings[]", severity_summary]
    optional_fields: [recommendations]
    produces: [audit_report]
    consumes: [diff]
    gates: [security_audit_ready]
    parallel_safe: true
gates:
  security_audit_ready:
    explicit_request: security_audit
"#;

    fn temp_project() -> PathBuf {
        let path = std::env::temp_dir().join(format!("vibehub-capability-test-{}", Uuid::new_v4()));
        fs::create_dir_all(path.join(".vibehub/tasks/T-001/runs/R-001")).expect("create run");
        fs::create_dir_all(path.join(".vibehub/tasks/T-001/context")).expect("create context");
        fs::write(path.join(".vibehub/workflow.yaml"), WORKFLOW).expect("write workflow");
        fs::write(
            path.join(".vibehub/state.yaml"),
            r#"schema_version: 3
current:
  task_id: T-001
  run_id: R-001
  mode: evidence_drive
  phase: research
  phase_status: active
flow:
  align: completed
  research: active
"#,
        )
        .expect("write state");
        fs::write(
            path.join(".vibehub/tasks/current"),
            r#"schema_version: 1
kind: current_task_pointer
task_id: T-001
path: .vibehub/tasks/T-001
updated_at: "2026-05-28T00:00:00Z"
updated_by: vibehub
"#,
        )
        .expect("write current task");
        fs::write(
            path.join(".vibehub/tasks/T-001/runs/current"),
            r#"schema_version: 1
kind: current_run_pointer
task_id: T-001
run_id: R-001
path: .vibehub/tasks/T-001/runs/R-001
updated_at: "2026-05-28T00:00:00Z"
updated_by: vibehub
"#,
        )
        .expect("write current run");
        fs::write(
            path.join(".vibehub/tasks/T-001/task.yaml"),
            "schema_version: 1\nkind: vibehub_task\ntask_id: T-001\ntitle: Test\n",
        )
        .expect("write task");
        fs::write(
            path.join(".vibehub/tasks/T-001/runs/R-001/run.yaml"),
            "schema_version: 1\nkind: vibehub_run\ntask_id: T-001\nrun_id: R-001\n",
        )
        .expect("write run");
        fs::create_dir_all(path.join(".vibehub/rules")).expect("create rules");
        fs::write(
            path.join(".vibehub/rules/hard-rules.md"),
            "# Rules\n\n- Test rule.\n",
        )
        .expect("write rules");
        path
    }

    fn append(project: &Path, event: events::VibehubEvent) {
        events::append_structured_run_event(project, "T-001", "R-001", event).expect("append");
    }

    #[test]
    fn explicit_request_can_satisfy_gate() {
        let project = temp_project();
        let report = evaluate_capability_gates(&project, Some("plan")).expect("evaluate");
        let plan = report
            .capabilities
            .iter()
            .find(|status| status.capability == "plan")
            .expect("plan");
        assert!(plan.claimable);
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn produced_artifacts_satisfy_downstream_gates() {
        let project = temp_project();
        append(
            &project,
            events::VibehubEvent::CapabilityReleased {
                capability: "plan".to_string(),
                outcome: "completed".to_string(),
                task_pack_dirty: false,
            },
        );
        let report = evaluate_capability_gates(&project, None).expect("evaluate");
        let implement = report
            .capabilities
            .iter()
            .find(|status| status.capability == "implement")
            .expect("implement");
        assert!(implement.claimable);
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn blocking_risk_blocks_claim() {
        let project = temp_project();
        append(
            &project,
            events::VibehubEvent::CapabilityReleased {
                capability: "plan".to_string(),
                outcome: "completed".to_string(),
                task_pack_dirty: false,
            },
        );
        append(
            &project,
            events::VibehubEvent::RiskRaised {
                id: "R1".to_string(),
                severity: "blocking".to_string(),
                note: "blocked".to_string(),
            },
        );
        let error = claim_capability(&project, "implement").expect_err("blocked");
        assert!(error.to_string().contains("gate.precondition.unmet"));
        assert!(error.to_string().contains("open risk present"));
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn claim_writes_events_and_builds_pack() {
        let project = temp_project();
        let result = claim_capability(&project, "plan").expect("claim");
        assert!(project.join(&result.context_pack_path).is_file());
        assert!(project.join(&result.context_manifest_path).is_file());
        assert!(
            result.event_ids.len() >= 2,
            "claim writes claim and pack events"
        );
        let events = events::list_events(&project, "T-001", "R-001", None).expect("events");
        let event_types = events
            .iter()
            .filter_map(|event| event.event.get("event_type").and_then(JsonValue::as_str))
            .collect::<Vec<_>>();
        assert!(event_types.contains(&"CapabilityClaimed"));
        assert!(event_types.contains(&"CapabilityPackBuilt"));
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn custom_capability_can_be_claimed_and_builds_context_pack() {
        let project = temp_project();
        fs::write(project.join(".vibehub/workflow.yaml"), CUSTOM_WORKFLOW).expect("workflow");

        let report = evaluate_capability_gates(&project, Some("security_audit")).expect("evaluate");
        let custom = report
            .capabilities
            .iter()
            .find(|status| status.capability == "security_audit")
            .expect("custom capability");
        assert!(custom.claimable);

        let result = claim_capability(&project, "security_audit").expect("claim custom");
        assert!(result.context_pack_path.ends_with("/security_audit.md"));
        let pack = fs::read_to_string(project.join(&result.context_pack_path)).expect("pack");
        assert!(pack.contains("\"custom\": true"));
        assert!(pack.contains("findings[]"));
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn custom_capability_end_to_end_claim_output_release_and_handoff() {
        let project = temp_project();
        fs::write(
            project.join(".vibehub/workflow.yaml"),
            r#"schema_version: 2
name: custom-e2e-test
modes:
  evidence_drive:
    capabilities: [security_audit, review]
default_mode: evidence_drive
custom_capabilities:
  security_audit:
    required_fields: [audit_scope, "findings[]", severity_summary]
    optional_fields: [recommendations]
    produces: [audit_report]
    gates: [security_audit_ready]
    parallel_safe: true
capabilities:
  review:
    produces: [review_summary]
    gates: [review_ready]
gates:
  security_audit_ready:
    explicit_request: security_audit
  review_ready:
    has_artifact: audit_report
"#,
        )
        .expect("workflow");
        fs::write(
            project.join(".vibehub/state.yaml"),
            r#"schema_version: 3
current:
  task_id: T-001
  run_id: R-001
  mode: evidence_drive
  phase: security_audit
  phase_status: active
flow:
  security_audit: active
"#,
        )
        .expect("state");

        let claim = claim_capability(&project, "security_audit").expect("claim custom");
        let pack = fs::read_to_string(project.join(&claim.context_pack_path)).expect("pack");
        assert!(pack.contains("\"custom\": true"));
        assert!(pack.contains("audit_scope"));
        assert!(pack.contains("findings[]"));

        let output = serde_json::json!({
            "schema_version": "1.0",
            "capability": "security_audit",
            "task_id": "T-001",
            "run_id": "R-001",
            "created_at": "2026-05-29T00:00:00Z",
            "created_by": "agent",
            "data": {
                "audit_scope": "Custom capability implementation path",
                "findings": ["No blocking issue found"],
                "severity_summary": "low",
                "recommendations": "Keep custom field names stable."
            }
        });
        let write =
            schema_check::write_current_capability_output(&project, "security_audit", output)
                .expect("write custom output");
        assert!(write.validation.valid);
        assert!(project.join(&write.output_path).is_file());

        append(
            &project,
            events::VibehubEvent::CapabilityReleased {
                capability: "security_audit".to_string(),
                outcome: "completed".to_string(),
                task_pack_dirty: false,
            },
        );
        let gates = evaluate_capability_gates(&project, Some("review")).expect("evaluate review");
        let review = gates
            .capabilities
            .iter()
            .find(|status| status.capability == "review")
            .expect("review");
        assert!(review.claimable);

        fs::write(
            project.join(".vibehub/tasks/T-001/runs/R-001/outputs/output.md"),
            r#"# VibeHub Agent Output

## Completed
- `hard_observed`: security_audit custom capability was claimed, output, released, and handed off.

## Not Yet Done
- `agent_reported`: none

## Key Decisions Made
- `inferred`: custom capabilities use the same output and handoff contract as built-ins.

## Files Changed
- `hard_observed`: .vibehub/tasks/T-001/runs/R-001/outputs/security_audit.json

## Files Reportedly Read
- `hard_observed`: .vibehub/workflow.yaml

## Commands Run
- `hard_observed`: test fixture API calls

## Tests Run
- `hard_observed`: custom capability end-to-end fixture

## Context Still Needed
- `agent_reported`: none

## Warnings
- `agent_reported`: none

## Next Session Should
- `agent_reported`: proceed to downstream review
"#,
        )
        .expect("output");
        let handoff = handoff::build_handoff(&project).expect("handoff");
        assert!(handoff.complete);

        let events = events::list_events(&project, "T-001", "R-001", None).expect("events");
        let event_types = events
            .iter()
            .filter_map(|event| event.event.get("event_type").and_then(JsonValue::as_str))
            .collect::<Vec<_>>();
        assert!(event_types.contains(&"CapabilityClaimed"));
        assert!(event_types.contains(&"CapabilityPackBuilt"));
        assert!(event_types.contains(&"CapabilityReleased"));
        assert!(event_types.contains(&"HandoffWritten"));
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn allows_multiple_active_capability_claims_with_distinct_packs() {
        let project = temp_project();

        let align = claim_capability(&project, "align").expect("claim align");
        let research = claim_capability(&project, "research").expect("claim research");

        assert_ne!(align.context_pack_path, research.context_pack_path);
        assert!(align.context_pack_path.ends_with("/align.md"));
        assert!(research.context_pack_path.ends_with("/research.md"));
        assert!(project.join(&align.context_pack_path).is_file());
        assert!(project.join(&research.context_pack_path).is_file());

        let events = events::list_events(&project, "T-001", "R-001", None).expect("events");
        let active = fold_event_facts(&events, &workflow::read_workflow_file(&project).unwrap())
            .active_capabilities;
        assert_eq!(
            active,
            BTreeSet::from(["align".to_string(), "research".to_string()])
        );

        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn rejects_duplicate_active_capability_claim() {
        let project = temp_project();
        claim_capability(&project, "align").expect("claim align");

        let error = claim_capability(&project, "align").expect_err("duplicate active claim");

        assert!(error.to_string().contains("gate.precondition.unmet"));
        assert!(error.to_string().contains("already active"));

        fs::remove_dir_all(project).ok();
    }
}

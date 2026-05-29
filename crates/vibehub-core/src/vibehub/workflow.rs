use crate::vibehub::util::{
    canonical_initialized_project_root, normalize_path, relative_to_project,
};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_yaml::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::Path;

const WORKFLOW_REL_PATH: &str = ".vibehub/workflow.yaml";
const BUILTIN_CAPABILITIES: &[&str] = &[
    "align_lite",
    "align",
    "research",
    "plan",
    "implement",
    "validate",
    "review_lite",
    "review",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct WorkflowConfig {
    #[serde(default)]
    pub schema_version: Option<u32>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub philosophy: Option<String>,
    #[serde(default)]
    pub modes: BTreeMap<String, WorkflowMode>,
    #[serde(default)]
    pub default_mode: Option<String>,
    #[serde(default)]
    pub phase_order: Vec<String>,
    #[serde(default)]
    pub capabilities: BTreeMap<String, CapabilityDefinition>,
    #[serde(default)]
    pub custom_capabilities: BTreeMap<String, CapabilityDefinition>,
    #[serde(default)]
    pub gates: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct WorkflowMode {
    #[serde(default)]
    pub phases: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CapabilityDefinition {
    #[serde(default)]
    pub required_fields: Vec<String>,
    #[serde(default)]
    pub optional_fields: Vec<String>,
    #[serde(default)]
    pub produces: Vec<String>,
    #[serde(default)]
    pub consumes: Vec<String>,
    #[serde(default)]
    pub parallel_safe: bool,
    #[serde(default)]
    pub gates: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WorkflowError {
    pub code: String,
    pub message: String,
    pub hint: String,
    pub details: serde_json::Value,
}

impl WorkflowError {
    fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        hint: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            hint: hint.into(),
            details,
        }
    }
}

impl fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {} Hint: {}", self.code, self.message, self.hint)
    }
}

#[derive(Debug, Clone)]
pub struct WorkflowErrorList {
    errors: Vec<WorkflowError>,
}

impl WorkflowErrorList {
    fn new(errors: Vec<WorkflowError>) -> Self {
        Self { errors }
    }

    #[cfg(test)]
    pub fn errors(&self) -> &[WorkflowError] {
        &self.errors
    }
}

impl fmt::Display for WorkflowErrorList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let summary = self
            .errors
            .iter()
            .map(|error| error.to_string())
            .collect::<Vec<_>>()
            .join("; ");
        write!(f, "{summary}")
    }
}

impl std::error::Error for WorkflowErrorList {}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WorkflowExplainResult {
    pub workflow_path: String,
    pub schema_version: Option<u32>,
    pub name: Option<String>,
    pub default_mode: Option<String>,
    pub phase_order: Vec<String>,
    pub modes: Vec<WorkflowModeSummary>,
    pub capabilities: Vec<WorkflowCapabilitySummary>,
    pub gates: Vec<WorkflowGateSummary>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WorkflowModeSummary {
    pub name: String,
    pub phases: Vec<String>,
    pub capabilities: Vec<String>,
    pub effective_capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WorkflowCapabilitySummary {
    pub name: String,
    pub required_fields: Vec<String>,
    pub optional_fields: Vec<String>,
    pub produces: Vec<String>,
    pub consumes: Vec<String>,
    pub parallel_safe: bool,
    pub gates: Vec<String>,
    pub declared: bool,
    pub custom: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WorkflowGateSummary {
    pub name: String,
    pub predicates: Vec<String>,
    pub expression: serde_json::Value,
}

pub fn read_workflow_file(project_root: &Path) -> Result<WorkflowConfig> {
    let path = project_root.join(WORKFLOW_REL_PATH);
    let content =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    parse_workflow_str(&content, normalize_path(&path))
}

pub fn explain_workflow(project_path: impl AsRef<Path>) -> Result<WorkflowExplainResult> {
    let project_root = canonical_initialized_project_root(project_path.as_ref())?;
    let workflow_path = project_root.join(WORKFLOW_REL_PATH);
    let workflow = read_workflow_file(&project_root)?;
    Ok(explain_config(&project_root, &workflow_path, &workflow))
}

pub fn resolve_phase_list(workflow: &WorkflowConfig, mode: &str) -> Result<Vec<String>> {
    if let Some(mode_entry) = workflow.modes.get(mode) {
        if !mode_entry.phases.is_empty() {
            return Ok(mode_entry.phases.clone());
        }
        if !mode_entry.capabilities.is_empty() {
            return Ok(mode_entry.capabilities.clone());
        }
    }

    if !workflow.phase_order.is_empty() {
        return Ok(workflow.phase_order.clone());
    }

    Err(anyhow!(
        "No phases defined in workflow.yaml for mode '{}'",
        mode
    ))
}

pub fn is_builtin_capability(name: &str) -> bool {
    BUILTIN_CAPABILITIES.contains(&name)
}

pub fn capability_definition<'a>(
    workflow: &'a WorkflowConfig,
    name: &str,
) -> Option<&'a CapabilityDefinition> {
    workflow
        .capabilities
        .get(name)
        .or_else(|| workflow.custom_capabilities.get(name))
}

pub fn is_custom_capability(workflow: &WorkflowConfig, name: &str) -> bool {
    workflow.custom_capabilities.contains_key(name)
}

fn parse_workflow_str(content: &str, source: impl Into<String>) -> Result<WorkflowConfig> {
    let source = source.into();
    let workflow = serde_yaml::from_str::<WorkflowConfig>(content).map_err(|error| {
        WorkflowErrorList::new(vec![WorkflowError::new(
            "workflow.parse.invalid_yaml",
            format!("Invalid YAML in {source}: {error}"),
            "Fix workflow.yaml syntax or field types, then rerun vibehub-workflow-explain.",
            json!({ "source": source, "serde_error": error.to_string() }),
        )])
    })?;
    validate_workflow(&workflow, &source)?;
    Ok(workflow)
}

fn validate_workflow(workflow: &WorkflowConfig, source: &str) -> Result<()> {
    let mut errors = Vec::new();

    if workflow.modes.is_empty() && workflow.phase_order.is_empty() {
        errors.push(WorkflowError::new(
            "workflow.schema.missing_flow",
            "workflow.yaml must define at least one mode or phase_order.",
            "Add modes.<mode>.phases, modes.<mode>.capabilities, or phase_order.",
            json!({ "source": source }),
        ));
    }

    for (mode_name, mode) in &workflow.modes {
        if mode.phases.is_empty() && mode.capabilities.is_empty() {
            errors.push(WorkflowError::new(
                "workflow.schema.empty_mode",
                format!("Mode '{mode_name}' has neither phases nor capabilities."),
                "Add phases for legacy compatibility or capabilities for the new workflow model.",
                json!({ "source": source, "mode": mode_name }),
            ));
        }
        validate_string_list(
            &mut errors,
            source,
            &format!("modes.{mode_name}.phases"),
            &mode.phases,
        );
        validate_string_list(
            &mut errors,
            source,
            &format!("modes.{mode_name}.capabilities"),
            &mode.capabilities,
        );
    }

    validate_string_list(&mut errors, source, "phase_order", &workflow.phase_order);

    validate_capability_map(&mut errors, source, "capabilities", &workflow.capabilities);
    validate_capability_map(
        &mut errors,
        source,
        "custom_capabilities",
        &workflow.custom_capabilities,
    );

    for name in workflow.custom_capabilities.keys() {
        if is_builtin_capability(name) || workflow.capabilities.contains_key(name) {
            errors.push(WorkflowError::new(
                "capability.name.conflict",
                format!("Custom capability '{name}' conflicts with an existing capability name."),
                "Rename the custom capability to a unique stable identifier such as security_audit.",
                json!({ "source": source, "capability": name }),
            ));
        }
    }

    for (name, expression) in &workflow.gates {
        if name.trim().is_empty() {
            errors.push(WorkflowError::new(
                "gate.schema.empty_name",
                "Gate names cannot be blank.",
                "Rename blank gate keys to stable identifiers such as review_ready.",
                json!({ "source": source }),
            ));
        }
        validate_gate_value(
            &mut errors,
            source,
            name,
            expression,
            &format!("gates.{name}"),
        );
    }

    for (name, capability) in workflow
        .capabilities
        .iter()
        .chain(workflow.custom_capabilities.iter())
    {
        for gate in &capability.gates {
            if !workflow.gates.contains_key(gate) {
                errors.push(WorkflowError::new(
                    "gate.reference.missing",
                    format!("Capability '{name}' references missing gate '{gate}'."),
                    "Define the gate under gates: or remove the capability gate reference.",
                    json!({ "source": source, "capability": name, "gate": gate }),
                ));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(WorkflowErrorList::new(errors).into())
    }
}

fn validate_capability_map(
    errors: &mut Vec<WorkflowError>,
    source: &str,
    section: &str,
    capabilities: &BTreeMap<String, CapabilityDefinition>,
) {
    for (name, capability) in capabilities {
        if name.trim().is_empty() {
            errors.push(WorkflowError::new(
                "capability.schema.empty_name",
                "Capability names cannot be blank.",
                "Rename blank capability keys to stable identifiers such as research or review.",
                json!({ "source": source, "section": section }),
            ));
        }
        validate_string_list(
            errors,
            source,
            &format!("{section}.{name}.required_fields"),
            &capability.required_fields,
        );
        validate_string_list(
            errors,
            source,
            &format!("{section}.{name}.optional_fields"),
            &capability.optional_fields,
        );
        validate_string_list(
            errors,
            source,
            &format!("{section}.{name}.produces"),
            &capability.produces,
        );
        validate_string_list(
            errors,
            source,
            &format!("{section}.{name}.consumes"),
            &capability.consumes,
        );
        validate_string_list(
            errors,
            source,
            &format!("{section}.{name}.gates"),
            &capability.gates,
        );
    }
}

fn validate_string_list(
    errors: &mut Vec<WorkflowError>,
    source: &str,
    path: &str,
    values: &[String],
) {
    for (index, value) in values.iter().enumerate() {
        if value.trim().is_empty() {
            errors.push(WorkflowError::new(
                "workflow.schema.blank_item",
                format!("{path}[{index}] cannot be blank."),
                "Remove blank list items or replace them with stable identifiers.",
                json!({ "source": source, "path": path, "index": index }),
            ));
        }
    }
}

fn validate_gate_value(
    errors: &mut Vec<WorkflowError>,
    source: &str,
    gate_name: &str,
    value: &Value,
    path: &str,
) {
    match value {
        Value::String(text) => {
            if text.trim().is_empty() {
                errors.push(WorkflowError::new(
                    "gate.schema.empty_predicate",
                    format!("{path} cannot be an empty predicate."),
                    "Use a non-empty predicate name or an all/any/not expression.",
                    json!({ "source": source, "gate": gate_name, "path": path }),
                ));
            }
        }
        Value::Mapping(map) => validate_gate_mapping(errors, source, gate_name, map, path),
        _ => errors.push(WorkflowError::new(
            "gate.schema.type_mismatch",
            format!("{path} must be a predicate string or mapping."),
            "Use a predicate string, a predicate mapping, or all/any/not gate composition.",
            json!({ "source": source, "gate": gate_name, "path": path }),
        )),
    }
}

fn validate_gate_mapping(
    errors: &mut Vec<WorkflowError>,
    source: &str,
    gate_name: &str,
    map: &serde_yaml::Mapping,
    path: &str,
) {
    if map.is_empty() {
        errors.push(WorkflowError::new(
            "gate.schema.empty_expression",
            format!("{path} cannot be an empty mapping."),
            "Use a predicate mapping or an all/any/not gate composition.",
            json!({ "source": source, "gate": gate_name, "path": path }),
        ));
        return;
    }

    let operator_keys = map
        .keys()
        .filter_map(Value::as_str)
        .filter(|key| matches!(*key, "all" | "any" | "not"))
        .collect::<Vec<_>>();
    if !operator_keys.is_empty() && map.len() > 1 {
        errors.push(WorkflowError::new(
            "gate.schema.invalid_operator",
            format!("{path} mixes gate operators with other keys."),
            "Use exactly one of all, any, or not at each gate expression level.",
            json!({ "source": source, "gate": gate_name, "path": path, "operators": operator_keys }),
        ));
        return;
    }

    if let Some(children) = map.get(Value::String("all".to_string())) {
        validate_gate_children(errors, source, gate_name, children, path, "all");
        return;
    }
    if let Some(children) = map.get(Value::String("any".to_string())) {
        validate_gate_children(errors, source, gate_name, children, path, "any");
        return;
    }
    if let Some(child) = map.get(Value::String("not".to_string())) {
        validate_gate_not(errors, source, gate_name, child, path);
        return;
    }

    if map.len() != 1 {
        errors.push(WorkflowError::new(
            "gate.schema.predicate_shape",
            format!("{path} predicate mappings must contain exactly one predicate key."),
            "Split multiple predicate keys into an all: list.",
            json!({ "source": source, "gate": gate_name, "path": path, "key_count": map.len() }),
        ));
        return;
    }

    let Some((key, _)) = map.iter().next() else {
        return;
    };
    if key
        .as_str()
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .is_none()
    {
        errors.push(WorkflowError::new(
            "gate.schema.predicate_key",
            format!("{path} predicate key must be a non-empty string."),
            "Use string predicate keys such as has_artifact or explicit_request.",
            json!({ "source": source, "gate": gate_name, "path": path }),
        ));
    }
}

fn validate_gate_children(
    errors: &mut Vec<WorkflowError>,
    source: &str,
    gate_name: &str,
    value: &Value,
    path: &str,
    operator: &str,
) {
    let Some(children) = value.as_sequence() else {
        errors.push(WorkflowError::new(
            "gate.schema.type_mismatch",
            format!("{path}.{operator} must be a non-empty list."),
            "Represent all/any as a YAML list of nested gate expressions.",
            json!({ "source": source, "gate": gate_name, "path": format!("{path}.{operator}") }),
        ));
        return;
    };
    if children.is_empty() {
        errors.push(WorkflowError::new(
            "gate.schema.empty_expression",
            format!("{path}.{operator} must contain at least one child expression."),
            "Add one or more predicate strings, predicate mappings, or nested gate expressions.",
            json!({ "source": source, "gate": gate_name, "path": format!("{path}.{operator}") }),
        ));
    }
    for (index, child) in children.iter().enumerate() {
        validate_gate_value(
            errors,
            source,
            gate_name,
            child,
            &format!("{path}.{operator}[{index}]"),
        );
    }
}

fn validate_gate_not(
    errors: &mut Vec<WorkflowError>,
    source: &str,
    gate_name: &str,
    value: &Value,
    path: &str,
) {
    if let Some(children) = value.as_sequence() {
        if children.len() != 1 {
            errors.push(WorkflowError::new(
                "gate.schema.invalid_not",
                format!("{path}.not must contain exactly one child expression."),
                "Wrap multiple negated predicates in not: { any: [...] } or not: { all: [...] }.",
                json!({ "source": source, "gate": gate_name, "path": format!("{path}.not"), "child_count": children.len() }),
            ));
            return;
        }
        validate_gate_value(
            errors,
            source,
            gate_name,
            &children[0],
            &format!("{path}.not[0]"),
        );
        return;
    }

    validate_gate_value(errors, source, gate_name, value, &format!("{path}.not"));
}

fn explain_config(
    project_root: &Path,
    workflow_path: &Path,
    workflow: &WorkflowConfig,
) -> WorkflowExplainResult {
    let mut capability_names = BTreeSet::new();
    capability_names.extend(workflow.capabilities.keys().cloned());
    capability_names.extend(workflow.custom_capabilities.keys().cloned());
    capability_names.extend(workflow.phase_order.iter().cloned());

    let modes = workflow
        .modes
        .iter()
        .map(|(name, mode)| {
            let effective_capabilities = if mode.capabilities.is_empty() {
                mode.phases.clone()
            } else {
                mode.capabilities.clone()
            };
            capability_names.extend(effective_capabilities.iter().cloned());
            WorkflowModeSummary {
                name: name.clone(),
                phases: mode.phases.clone(),
                capabilities: mode.capabilities.clone(),
                effective_capabilities,
            }
        })
        .collect::<Vec<_>>();

    let capabilities = capability_names
        .into_iter()
        .map(|name| {
            let definition = capability_definition(workflow, &name);
            let custom = is_custom_capability(workflow, &name);
            WorkflowCapabilitySummary {
                name,
                required_fields: definition
                    .map(|capability| capability.required_fields.clone())
                    .unwrap_or_default(),
                optional_fields: definition
                    .map(|capability| capability.optional_fields.clone())
                    .unwrap_or_default(),
                produces: definition
                    .map(|capability| capability.produces.clone())
                    .unwrap_or_default(),
                consumes: definition
                    .map(|capability| capability.consumes.clone())
                    .unwrap_or_default(),
                parallel_safe: definition
                    .map(|capability| capability.parallel_safe)
                    .unwrap_or(false),
                gates: definition
                    .map(|capability| capability.gates.clone())
                    .unwrap_or_default(),
                declared: definition.is_some(),
                custom,
            }
        })
        .collect::<Vec<_>>();

    let gates = workflow
        .gates
        .iter()
        .map(|(name, expression)| WorkflowGateSummary {
            name: name.clone(),
            predicates: gate_predicates(expression),
            expression: serde_json::to_value(expression).unwrap_or_else(|_| json!(null)),
        })
        .collect::<Vec<_>>();

    WorkflowExplainResult {
        workflow_path: relative_to_project(project_root, workflow_path)
            .map(|path| normalize_path(&path))
            .unwrap_or_else(|_| normalize_path(workflow_path)),
        schema_version: workflow.schema_version,
        name: workflow.name.clone(),
        default_mode: workflow.default_mode.clone(),
        phase_order: workflow.phase_order.clone(),
        modes,
        capabilities,
        gates,
    }
}

fn gate_predicates(value: &Value) -> Vec<String> {
    let mut predicates = BTreeSet::new();
    collect_gate_predicates(value, &mut predicates);
    predicates.into_iter().collect()
}

fn collect_gate_predicates(value: &Value, predicates: &mut BTreeSet<String>) {
    match value {
        Value::String(text) => {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                predicates.insert(trimmed.to_string());
            }
        }
        Value::Mapping(map) => {
            if let Some(children) = map.get(Value::String("all".to_string())) {
                if let Some(sequence) = children.as_sequence() {
                    for child in sequence {
                        collect_gate_predicates(child, predicates);
                    }
                }
                return;
            }
            if let Some(children) = map.get(Value::String("any".to_string())) {
                if let Some(sequence) = children.as_sequence() {
                    for child in sequence {
                        collect_gate_predicates(child, predicates);
                    }
                }
                return;
            }
            if let Some(child) = map.get(Value::String("not".to_string())) {
                collect_gate_predicates(child, predicates);
                return;
            }
            if let Some((key, _)) = map.iter().next() {
                if let Some(predicate) = key.as_str().map(str::trim).filter(|key| !key.is_empty()) {
                    predicates.insert(predicate.to_string());
                }
            }
        }
        Value::Sequence(sequence) => {
            for child in sequence {
                collect_gate_predicates(child, predicates);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    const LEGACY_WORKFLOW: &str = r#"schema_version: 1
name: legacy
modes:
  evidence_drive:
    phases:
      - align
      - research
      - implement
default_mode: evidence_drive
phase_order:
  - align
  - research
  - implement
"#;

    const NEW_WORKFLOW: &str = r#"schema_version: 2
name: capability-gates
modes:
  evidence_drive:
    phases:
      - align
      - research
    capabilities:
      - research
      - implement
      - review
default_mode: evidence_drive
phase_order:
  - align
  - research
  - implement
capabilities:
  research:
    required_fields: [sources, risks, open_questions]
    optional_fields: [hypothesis]
    produces: [research_output]
    consumes: []
    parallel_safe: true
  implement:
    required_fields: [changed_files, tests]
    consumes: [research_output]
    gates: [implement_ready]
  review:
    required_fields: [findings]
    consumes: [diff]
    gates: [review_ready]
gates:
  implement_ready:
    all:
      - has_artifact: research_output
      - not:
          explicit_request: pause
  review_ready:
    any:
      - has_artifact: diff
      - all:
          - explicit_request: review
          - not:
              any:
                - open_risk
                - unsynced_handoff
"#;

    const CUSTOM_WORKFLOW: &str = r#"schema_version: 2
name: custom-capability-gates
modes:
  evidence_drive:
    capabilities:
      - research
      - security_audit
default_mode: evidence_drive
capabilities:
  research:
    required_fields: [sources, risks, open_questions]
    produces: [research_output]
custom_capabilities:
  security_audit:
    required_fields: [audit_scope, "findings[]", severity_summary]
    optional_fields: [recommendations, compliance_refs]
    produces: [audit_report]
    consumes: [implement]
    gates: [security_audit_ready]
    parallel_safe: true
gates:
  security_audit_ready:
    explicit_request: security_audit
"#;

    fn temp_project() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("vibehub-workflow-test-{}", Uuid::new_v4()));
        fs::create_dir_all(path.join(".vibehub")).expect("create .vibehub");
        path
    }

    fn parse_err_codes(content: &str) -> Vec<String> {
        let error = parse_workflow_str(content, "test-workflow.yaml").expect_err("invalid");
        let list = error
            .downcast_ref::<WorkflowErrorList>()
            .expect("workflow error list");
        list.errors()
            .iter()
            .map(|error| error.code.clone())
            .collect()
    }

    #[test]
    fn parses_legacy_phases_only_workflow() {
        let workflow = parse_workflow_str(LEGACY_WORKFLOW, "legacy").expect("parse");
        assert!(workflow.capabilities.is_empty());
        assert_eq!(
            resolve_phase_list(&workflow, "evidence_drive").expect("phases"),
            vec!["align", "research", "implement"]
        );
    }

    #[test]
    fn parses_new_capabilities_and_gates_workflow() {
        let workflow = parse_workflow_str(NEW_WORKFLOW, "new").expect("parse");
        assert_eq!(workflow.capabilities.len(), 3);
        assert_eq!(workflow.gates.len(), 2);
        assert!(workflow.capabilities["research"].parallel_safe);
    }

    #[test]
    fn parses_custom_capabilities_and_includes_them_in_explain() {
        let workflow = parse_workflow_str(CUSTOM_WORKFLOW, "custom").expect("parse");
        assert_eq!(workflow.custom_capabilities.len(), 1);
        assert!(workflow.custom_capabilities["security_audit"].parallel_safe);
        assert_eq!(
            workflow.custom_capabilities["security_audit"].required_fields,
            vec!["audit_scope", "findings[]", "severity_summary"]
        );

        let project = temp_project();
        let path = project.join(".vibehub/workflow.yaml");
        let explain = explain_config(&project, &path, &workflow);
        let custom = explain
            .capabilities
            .iter()
            .find(|capability| capability.name == "security_audit")
            .expect("custom capability summary");
        assert!(custom.declared);
        assert!(custom.custom);
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn mode_capabilities_are_used_as_effective_capabilities() {
        let workflow = parse_workflow_str(NEW_WORKFLOW, "new").expect("parse");
        let project = temp_project();
        let path = project.join(".vibehub/workflow.yaml");
        fs::write(&path, NEW_WORKFLOW).expect("write");
        let explain = explain_config(&project, &path, &workflow);
        let mode = explain
            .modes
            .iter()
            .find(|mode| mode.name == "evidence_drive")
            .expect("mode");
        assert_eq!(
            mode.effective_capabilities,
            vec!["research", "implement", "review"]
        );
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn legacy_phases_become_undeclared_capability_summaries() {
        let workflow = parse_workflow_str(LEGACY_WORKFLOW, "legacy").expect("parse");
        let project = temp_project();
        let path = project.join(".vibehub/workflow.yaml");
        let explain = explain_config(&project, &path, &workflow);
        let align = explain
            .capabilities
            .iter()
            .find(|capability| capability.name == "align")
            .expect("align summary");
        assert!(!align.declared);
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn validates_nested_all_any_not_gates() {
        let workflow = parse_workflow_str(NEW_WORKFLOW, "new").expect("parse");
        let predicates = gate_predicates(&workflow.gates["review_ready"]);
        assert_eq!(
            predicates,
            vec![
                "explicit_request",
                "has_artifact",
                "open_risk",
                "unsynced_handoff"
            ]
        );
    }

    #[test]
    fn invalid_yaml_returns_workflow_error_code() {
        let codes = parse_err_codes("modes:\n  - nope: [");
        assert_eq!(codes, vec!["workflow.parse.invalid_yaml"]);
    }

    #[test]
    fn missing_flow_returns_schema_error_code() {
        let codes = parse_err_codes("schema_version: 1\nname: empty\n");
        assert_eq!(codes, vec!["workflow.schema.missing_flow"]);
    }

    #[test]
    fn empty_all_gate_returns_gate_error_code() {
        let codes = parse_err_codes(
            r#"modes:
  guided_drive:
    phases: [align]
gates:
  empty:
    all: []
"#,
        );
        assert!(codes.contains(&"gate.schema.empty_expression".to_string()));
    }

    #[test]
    fn not_gate_rejects_multiple_children() {
        let codes = parse_err_codes(
            r#"modes:
  guided_drive:
    phases: [align]
gates:
  bad:
    not:
      - has_artifact: diff
      - explicit_request: review
"#,
        );
        assert!(codes.contains(&"gate.schema.invalid_not".to_string()));
    }

    #[test]
    fn predicate_maps_must_have_one_key() {
        let codes = parse_err_codes(
            r#"modes:
  guided_drive:
    phases: [align]
gates:
  bad:
    has_artifact: diff
    explicit_request: review
"#,
        );
        assert!(codes.contains(&"gate.schema.predicate_shape".to_string()));
    }

    #[test]
    fn missing_referenced_gate_returns_reference_error_code() {
        let codes = parse_err_codes(
            r#"modes:
  guided_drive:
    phases: [implement]
capabilities:
  implement:
    gates: [implement_ready]
"#,
        );
        assert!(codes.contains(&"gate.reference.missing".to_string()));
    }

    #[test]
    fn custom_capability_name_cannot_conflict_with_builtin() {
        let codes = parse_err_codes(
            r#"modes:
  guided_drive:
    phases: [align]
custom_capabilities:
  research:
    required_fields: [summary]
"#,
        );
        assert!(codes.contains(&"capability.name.conflict".to_string()));
    }

    #[test]
    fn explain_workflow_reads_project_file() {
        let project = temp_project();
        fs::write(project.join(".vibehub/workflow.yaml"), NEW_WORKFLOW).expect("write workflow");
        let explain = explain_workflow(&project).expect("explain");
        assert_eq!(explain.workflow_path, ".vibehub/workflow.yaml");
        assert_eq!(explain.gates.len(), 2);
        fs::remove_dir_all(project).ok();
    }
}

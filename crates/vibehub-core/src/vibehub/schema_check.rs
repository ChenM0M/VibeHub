use crate::vibehub::util::{
    canonical_initialized_project_root, normalize_path, relative_to_project,
};
use crate::vibehub::{current, events, policy, workflow};
use anyhow::{anyhow, Context, Result};
use chrono::DateTime;
use serde::Serialize;
use serde_json::{json, Value};
use std::fmt;
use std::fs;
use std::path::Path;

pub const CAPABILITY_SCHEMA_VERSION: &str = "1.0";
pub const PLACEHOLDER_NO_RISK: &str = "no_risk";
pub const PLACEHOLDER_NONE: &str = "none";
pub const PLACEHOLDER_NA: &str = "n/a";
pub const PLACEHOLDER_NOT_RUN: &str = "not_run";
pub const PLACEHOLDER_UNKNOWN: &str = "unknown";

const CAPABILITIES: &[&str] = &[
    "align_lite",
    "align",
    "research",
    "plan",
    "implement",
    "validate",
    "review_lite",
    "review",
];

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SchemaValidationError {
    pub code: String,
    pub message: String,
    pub hint: String,
    pub details: Value,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CapabilityValidationReport {
    pub target: String,
    pub valid: bool,
    pub errors: Vec<SchemaValidationError>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CapabilityOutputWriteResult {
    pub output_path: String,
    pub validation: CapabilityValidationReport,
    pub event_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SchemaValidationErrorList {
    target: String,
    errors: Vec<SchemaValidationError>,
}

impl SchemaValidationErrorList {
    pub fn new(target: impl Into<String>, errors: Vec<SchemaValidationError>) -> Self {
        Self {
            target: target.into(),
            errors,
        }
    }

    pub fn report(&self) -> CapabilityValidationReport {
        CapabilityValidationReport {
            target: self.target.clone(),
            valid: false,
            errors: self.errors.clone(),
        }
    }
}

impl fmt::Display for SchemaValidationErrorList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let summary = self
            .errors
            .iter()
            .map(|error| format!("[{}] {}", error.code, error.message))
            .collect::<Vec<_>>()
            .join("; ");
        write!(f, "{summary}")
    }
}

impl std::error::Error for SchemaValidationErrorList {}

#[cfg_attr(not(test), allow(dead_code))]
pub fn validate_capability_output(
    capability: impl AsRef<str>,
    output: &Value,
) -> Result<CapabilityValidationReport, SchemaValidationErrorList> {
    let capability = capability.as_ref();
    validate_capability_output_with_workflow(None, capability, output)
}

pub fn validate_capability_output_for_project(
    project_root: impl AsRef<Path>,
    capability: impl AsRef<str>,
    output: &Value,
) -> Result<CapabilityValidationReport> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let workflow = workflow::read_workflow_file(&project_root)?;
    validate_capability_output_with_workflow(Some(&workflow), capability.as_ref(), output)
        .map_err(|errors| anyhow!(errors))
}

fn validate_capability_output_with_workflow(
    workflow: Option<&workflow::WorkflowConfig>,
    capability: &str,
    output: &Value,
) -> Result<CapabilityValidationReport, SchemaValidationErrorList> {
    let target = format!("capability_output:{capability}");
    let mut ctx = ValidationCtx::new(&target);

    validate_envelope(&mut ctx, workflow, capability, output);
    if let Some(data) = output.get("data").and_then(Value::as_object) {
        validate_data(&mut ctx, workflow, capability, &Value::Object(data.clone()));
    }

    if ctx.errors.is_empty() {
        Ok(CapabilityValidationReport {
            target,
            valid: true,
            errors: Vec::new(),
        })
    } else {
        Err(SchemaValidationErrorList::new(target, ctx.errors))
    }
}

pub fn validate_current_capability_output_file(
    project_root: impl AsRef<Path>,
    capability: impl AsRef<str>,
    output_path: impl AsRef<Path>,
) -> Result<CapabilityValidationReport> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let path = output_path.as_ref();
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root.join(path)
    };
    let content = fs::read_to_string(&absolute)
        .with_context(|| format!("Failed to read {}", absolute.display()))?;
    let output: Value = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse {} as JSON", absolute.display()))?;
    validate_capability_output_for_project(project_root, capability, &output)
}

pub fn write_current_capability_output(
    project_root: impl AsRef<Path>,
    capability: impl AsRef<str>,
    output: Value,
) -> Result<CapabilityOutputWriteResult> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let capability = capability.as_ref().to_string();
    let task = current::resolve_current_task(&project_root)?;
    let run = current::resolve_current_run(&project_root, &task.task_id)?;
    let target = format!("capability_output:{capability}");
    let policy = policy::read_policy(&project_root)?;
    let workflow = workflow::read_workflow_file(&project_root)?;

    if policy.schema.strict {
        if let Err(errors) =
            validate_capability_output_with_workflow(Some(&workflow), &capability, &output)
        {
            let report = errors.report();
            let event = events::append_structured_run_event(
                &project_root,
                &task.task_id,
                &run.run_id,
                events::VibehubEvent::SchemaValidationFailed {
                    target,
                    errors: report
                        .errors
                        .iter()
                        .map(|error| serde_json::to_value(error).unwrap_or_else(|_| json!({})))
                        .collect(),
                },
            )?;
            return Err(anyhow!(
                "{}; SchemaValidationFailed event {} written to {}",
                errors,
                event.event_id,
                event.events_path
            ));
        }
    }

    let output_path = project_root
        .join(".vibehub")
        .join("tasks")
        .join(&task.task_id)
        .join("runs")
        .join(&run.run_id)
        .join("outputs")
        .join(format!("{capability}.json"));
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    fs::write(
        &output_path,
        serde_json::to_string_pretty(&output).context("Failed to serialize capability output")?,
    )
    .with_context(|| format!("Failed to write {}", output_path.display()))?;

    Ok(CapabilityOutputWriteResult {
        output_path: normalize_path(&relative_to_project(&project_root, &output_path)?),
        validation: CapabilityValidationReport {
            target,
            valid: true,
            errors: Vec::new(),
        },
        event_id: None,
    })
}

fn validate_envelope(
    ctx: &mut ValidationCtx,
    workflow: Option<&workflow::WorkflowConfig>,
    capability: &str,
    output: &Value,
) {
    let Some(object) = output.as_object() else {
        ctx.type_mismatch("$", "Capability output must be a JSON object.");
        return;
    };

    for field in [
        "schema_version",
        "capability",
        "task_id",
        "run_id",
        "created_at",
        "created_by",
        "data",
    ] {
        if !object.contains_key(field) {
            ctx.missing(
                field,
                "Add the required envelope field before writing this output.",
            );
        }
    }

    match object.get("schema_version").and_then(Value::as_str) {
        Some(CAPABILITY_SCHEMA_VERSION) => {}
        Some(_) => ctx.version(
            "schema_version",
            "Use schema_version \"1.0\" for v1 capability outputs.",
        ),
        None => {}
    }
    match object.get("capability").and_then(Value::as_str) {
        Some(value) if value == capability && capability_is_known(workflow, value) => {}
        Some(value) if capability_is_known(workflow, value) => ctx.type_mismatch(
            "capability",
            &format!(
                "Envelope capability '{value}' does not match requested capability '{capability}'."
            ),
        ),
        Some(_) | None if object.contains_key("capability") => ctx.type_mismatch(
            "capability",
            "Use a built-in capability name or a project custom_capabilities name.",
        ),
        None => {}
        Some(_) => ctx.type_mismatch(
            "capability",
            "Use a built-in capability name or a project custom_capabilities name.",
        ),
    }
    validate_pattern(
        ctx,
        output,
        "task_id",
        "T-",
        "Use a task id like T-20260528-demo.",
    );
    validate_pattern(
        ctx,
        output,
        "run_id",
        "R-",
        "Use a run id like R-20260528-demo.",
    );
    validate_string_min(ctx, output, "created_by", 1);
    if let Some(created_at) = output.get("created_at").and_then(Value::as_str) {
        if DateTime::parse_from_rfc3339(created_at).is_err() {
            ctx.type_mismatch("created_at", "Use an ISO 8601/RFC3339 timestamp.");
        }
    } else if object.contains_key("created_at") {
        ctx.type_mismatch("created_at", "created_at must be a string timestamp.");
    }
    if object.get("data").is_some_and(|value| !value.is_object()) {
        ctx.type_mismatch(
            "data",
            "data must be an object containing capability-specific fields.",
        );
    }
}

fn validate_data(
    ctx: &mut ValidationCtx,
    workflow: Option<&workflow::WorkflowConfig>,
    capability: &str,
    data: &Value,
) {
    match capability {
        "align_lite" => {
            validate_string_min(ctx, data, "data.intent", 10);
            validate_string_min(ctx, data, "data.scope", 10);
            validate_optional_string_array(ctx, data, "data.references", 1, &[PLACEHOLDER_NA]);
        }
        "align" => {
            validate_string_min(ctx, data, "data.intent", 20);
            validate_string_min(ctx, data, "data.scope", 20);
            validate_string_array(ctx, data, "data.success_criteria", 1, 5, &[]);
            validate_string_array(ctx, data, "data.non_goals", 1, 1, &[PLACEHOLDER_NONE]);
            validate_optional_string_array(ctx, data, "data.stakeholders", 1, &[]);
            validate_optional_string_array(ctx, data, "data.references", 1, &[PLACEHOLDER_NA]);
        }
        "research" => validate_research(ctx, data),
        "plan" => validate_plan(ctx, data),
        "implement" => validate_implement(ctx, data),
        "validate" => validate_validate(ctx, data),
        "review_lite" => validate_review_like(ctx, data, false),
        "review" => validate_review_like(ctx, data, true),
        _ => {
            if let Some(definition) =
                workflow.and_then(|workflow| workflow.custom_capabilities.get(capability))
            {
                validate_custom_data(ctx, data, definition);
            } else {
                ctx.type_mismatch(
                    "capability",
                    "Unknown capability; use a built-in or custom_capabilities name.",
                );
            }
        }
    }
}

fn capability_is_known(workflow: Option<&workflow::WorkflowConfig>, capability: &str) -> bool {
    CAPABILITIES.contains(&capability)
        || workflow.is_some_and(|workflow| workflow.custom_capabilities.contains_key(capability))
}

fn validate_custom_data(
    ctx: &mut ValidationCtx,
    data: &Value,
    definition: &workflow::CapabilityDefinition,
) {
    for field in &definition.required_fields {
        validate_custom_field(ctx, data, field, true);
    }
    for field in &definition.optional_fields {
        if custom_field_value(data, field).is_some() {
            validate_custom_field(ctx, data, field, false);
        }
    }
}

fn validate_custom_field(ctx: &mut ValidationCtx, data: &Value, field: &str, required: bool) {
    let is_array = field.ends_with("[]");
    let field_name = field.trim_end_matches("[]");
    let path = format!("data.{field}");
    let Some(value) = custom_field_value(data, field) else {
        if required {
            ctx.missing(&path, "Add the required custom capability field.");
        }
        return;
    };

    if value.is_null() {
        ctx.type_mismatch(&path, "Custom capability fields cannot be null.");
        return;
    }
    if is_array {
        match value.as_array() {
            Some(items) if !items.is_empty() => {}
            Some(_) => ctx.type_mismatch(&path, "Custom array fields must not be empty."),
            None => ctx.type_mismatch(&path, "Custom fields ending in [] must be arrays."),
        }
    } else if field_name.trim().is_empty() {
        ctx.type_mismatch(&path, "Custom capability field names cannot be blank.");
    }
}

fn custom_field_value<'a>(data: &'a Value, field: &str) -> Option<&'a Value> {
    let field_name = field.trim_end_matches("[]");
    data.get(field_name).or_else(|| data.get(field))
}

fn validate_research(ctx: &mut ValidationCtx, data: &Value) {
    validate_object_array(
        ctx,
        data,
        "data.sources",
        1,
        &["kind", "ref_value", "summary"],
    );
    if let Some(items) = data.pointer("/sources").and_then(Value::as_array) {
        for (idx, item) in items.iter().enumerate() {
            validate_enum_at(
                ctx,
                item,
                &format!("data.sources[{idx}].kind"),
                &["file", "doc", "web", "command", "user_note"],
            );
            validate_string_min_at(
                ctx,
                item,
                &format!("data.sources[{idx}].ref_value"),
                "ref_value",
                1,
                &[],
            );
            validate_string_min_at(
                ctx,
                item,
                &format!("data.sources[{idx}].summary"),
                "summary",
                10,
                &[],
            );
        }
    }
    validate_object_array(ctx, data, "data.risks", 1, &["id", "severity", "note"]);
    if let Some(items) = data.pointer("/risks").and_then(Value::as_array) {
        for (idx, item) in items.iter().enumerate() {
            validate_string_min_at(ctx, item, &format!("data.risks[{idx}].id"), "id", 1, &[]);
            validate_enum_at(
                ctx,
                item,
                &format!("data.risks[{idx}].severity"),
                &["low", "medium", "high", "blocker"],
            );
            validate_string_min_at(
                ctx,
                item,
                &format!("data.risks[{idx}].note"),
                "note",
                1,
                &[PLACEHOLDER_NO_RISK],
            );
        }
    }
    validate_string_array(ctx, data, "data.open_questions", 1, 1, &[PLACEHOLDER_NONE]);
    validate_optional_string(ctx, data, "data.hypothesis", 1, &[]);
    validate_optional_string_array(ctx, data, "data.references", 1, &[PLACEHOLDER_NA]);
    validate_optional_string_array(ctx, data, "data.related_threads", 1, &[PLACEHOLDER_NA]);
}

fn validate_plan(ctx: &mut ValidationCtx, data: &Value) {
    validate_object_array(ctx, data, "data.steps", 1, &["id", "description", "status"]);
    if let Some(items) = data.pointer("/steps").and_then(Value::as_array) {
        for (idx, item) in items.iter().enumerate() {
            validate_id_at(ctx, item, &format!("data.steps[{idx}].id"), "id");
            validate_string_min_at(
                ctx,
                item,
                &format!("data.steps[{idx}].description"),
                "description",
                10,
                &[],
            );
            validate_enum_at(
                ctx,
                item,
                &format!("data.steps[{idx}].status"),
                &["pending", "in_progress", "done", "blocked"],
            );
        }
    }
    validate_string_min(ctx, data, "data.validation_plan", 10);
    validate_string_array(ctx, data, "data.affected_files", 1, 1, &[PLACEHOLDER_NONE]);
    validate_optional_string_array(ctx, data, "data.alternatives", 1, &[]);
    validate_optional_string(ctx, data, "data.rollback_plan", 1, &[PLACEHOLDER_NONE]);
}

fn validate_implement(ctx: &mut ValidationCtx, data: &Value) {
    validate_string_min(ctx, data, "data.diff_summary", 20);
    validate_string_array(ctx, data, "data.changed_files", 1, 1, &[]);
    validate_object_array(
        ctx,
        data,
        "data.commands_run",
        1,
        &["command", "status", "summary"],
    );
    if let Some(items) = data.pointer("/commands_run").and_then(Value::as_array) {
        for (idx, item) in items.iter().enumerate() {
            validate_string_min_at(
                ctx,
                item,
                &format!("data.commands_run[{idx}].command"),
                "command",
                1,
                &[],
            );
            validate_enum_at(
                ctx,
                item,
                &format!("data.commands_run[{idx}].status"),
                &["passed", "failed", PLACEHOLDER_NOT_RUN],
            );
            validate_string_min_at(
                ctx,
                item,
                &format!("data.commands_run[{idx}].summary"),
                "summary",
                1,
                &[],
            );
        }
    }
    validate_optional_string(ctx, data, "data.manual_test_notes", 1, &[PLACEHOLDER_NONE]);
}

fn validate_validate(ctx: &mut ValidationCtx, data: &Value) {
    validate_run_result(ctx, data, "test_results");
    validate_run_result(ctx, data, "lint_results");
    validate_enum(ctx, data, "data.status", &["passed", "failed", "blocked"]);
    if let Some(coverage) = data.pointer("/coverage") {
        if !coverage.is_object() {
            ctx.type_mismatch("data.coverage", "coverage must be an object with summary.");
        } else {
            validate_string_min_at(
                ctx,
                coverage,
                "data.coverage.summary",
                "summary",
                1,
                &[PLACEHOLDER_UNKNOWN],
            );
        }
    }
    validate_optional_string(ctx, data, "data.perf_notes", 1, &[PLACEHOLDER_NONE]);
}

fn validate_review_like(ctx: &mut ValidationCtx, data: &Value, require_risk_review: bool) {
    validate_string_min(ctx, data, "data.summary", 20);
    validate_object_array(ctx, data, "data.concerns", 1, &["severity", "note"]);
    let mut has_blocker = false;
    if let Some(items) = data.pointer("/concerns").and_then(Value::as_array) {
        for (idx, item) in items.iter().enumerate() {
            validate_enum_at(
                ctx,
                item,
                &format!("data.concerns[{idx}].severity"),
                &["info", "low", "medium", "high", "blocker"],
            );
            validate_string_min_at(
                ctx,
                item,
                &format!("data.concerns[{idx}].note"),
                "note",
                1,
                &[PLACEHOLDER_NO_RISK],
            );
            if item.get("severity").and_then(Value::as_str) == Some("blocker") {
                has_blocker = true;
            }
            if item
                .get("file")
                .is_some_and(|value| !(value.is_string() || value.is_null()))
            {
                ctx.type_mismatch(
                    &format!("data.concerns[{idx}].file"),
                    "file must be a string or null.",
                );
            }
        }
    }
    if data.pointer("/gate_pass").is_none() {
        ctx.missing(
            "data.gate_pass",
            "Set gate_pass to true or false based on review findings.",
        );
    } else if !data.pointer("/gate_pass").is_some_and(Value::is_boolean) {
        ctx.type_mismatch("data.gate_pass", "gate_pass must be a boolean.");
    }
    if has_blocker && data.pointer("/gate_pass").and_then(Value::as_bool) == Some(true) {
        ctx.relationship(
            "data.gate_pass",
            "gate_pass must be false when a blocker concern exists.",
        );
    }
    if require_risk_review {
        validate_string_min(ctx, data, "data.risk_review", 1);
        validate_optional_string_array(ctx, data, "data.suggestions", 1, &[]);
        validate_optional_string_array(ctx, data, "data.follow_ups", 1, &[]);
    }
}

fn validate_run_result(ctx: &mut ValidationCtx, data: &Value, field: &str) {
    let path = format!("data.{field}");
    let Some(result) = data.get(field) else {
        ctx.missing(&path, "Include run result status, summary, and commands.");
        return;
    };
    if !result.is_object() {
        ctx.type_mismatch(&path, "Run result must be an object.");
        return;
    }
    validate_enum_at(
        ctx,
        result,
        &format!("{path}.status"),
        &["passed", "failed", PLACEHOLDER_NOT_RUN],
    );
    validate_string_min_at(ctx, result, &format!("{path}.summary"), "summary", 1, &[]);
    if !result.get("commands").is_some_and(Value::is_array) {
        ctx.missing(
            &format!("{path}.commands"),
            "List commands or use an empty array when status is not_run.",
        );
    } else if let Some(commands) = result.get("commands").and_then(Value::as_array) {
        for (idx, command) in commands.iter().enumerate() {
            if !command
                .as_str()
                .is_some_and(|value| !value.trim().is_empty())
            {
                ctx.type_mismatch(
                    &format!("{path}.commands[{idx}]"),
                    "Command entries must be non-empty strings.",
                );
            }
        }
    }
}

fn validate_string_min(ctx: &mut ValidationCtx, root: &Value, dotted_path: &str, min: usize) {
    validate_string_min_at(
        ctx,
        root,
        dotted_path,
        dotted_path.rsplit('.').next().unwrap_or(dotted_path),
        min,
        &[],
    );
}

fn validate_optional_string(
    ctx: &mut ValidationCtx,
    root: &Value,
    dotted_path: &str,
    min: usize,
    placeholders: &[&str],
) {
    if path_value(root, dotted_path).is_some() {
        validate_string_min_at(
            ctx,
            root,
            dotted_path,
            dotted_path.rsplit('.').next().unwrap_or(dotted_path),
            min,
            placeholders,
        );
    }
}

fn validate_string_min_at(
    ctx: &mut ValidationCtx,
    root: &Value,
    dotted_path: &str,
    field: &str,
    min: usize,
    placeholders: &[&str],
) {
    match path_value(root, dotted_path).and_then(Value::as_str) {
        Some(value) if placeholders.contains(&value) || value.trim().len() >= min => {}
        Some(_) => ctx.type_mismatch(
            dotted_path,
            &format!("{field} must be at least {min} characters."),
        ),
        None if path_value(root, dotted_path).is_some() => {
            ctx.type_mismatch(dotted_path, &format!("{field} must be a string."))
        }
        None => ctx.missing(dotted_path, "Add the required field with a valid value."),
    }
}

fn validate_string_array(
    ctx: &mut ValidationCtx,
    root: &Value,
    dotted_path: &str,
    min_items: usize,
    min_string: usize,
    placeholders: &[&str],
) {
    let Some(value) = path_value(root, dotted_path) else {
        ctx.missing(
            dotted_path,
            "Add the required array with at least one item.",
        );
        return;
    };
    validate_array_items(ctx, value, dotted_path, min_items, min_string, placeholders);
}

fn validate_optional_string_array(
    ctx: &mut ValidationCtx,
    root: &Value,
    dotted_path: &str,
    min_string: usize,
    placeholders: &[&str],
) {
    if let Some(value) = path_value(root, dotted_path) {
        validate_array_items(ctx, value, dotted_path, 0, min_string, placeholders);
    }
}

fn validate_array_items(
    ctx: &mut ValidationCtx,
    value: &Value,
    dotted_path: &str,
    min_items: usize,
    min_string: usize,
    placeholders: &[&str],
) {
    let Some(items) = value.as_array() else {
        ctx.type_mismatch(dotted_path, "Expected an array.");
        return;
    };
    if items.len() < min_items {
        ctx.missing(
            dotted_path,
            "Required arrays must contain at least one item.",
        );
    }
    for (idx, item) in items.iter().enumerate() {
        match item.as_str() {
            Some(value) if placeholders.contains(&value) || value.trim().len() >= min_string => {}
            Some(_) => ctx.type_mismatch(
                &format!("{dotted_path}[{idx}]"),
                "Array string item is too short.",
            ),
            None => ctx.type_mismatch(
                &format!("{dotted_path}[{idx}]"),
                "Array item must be a string.",
            ),
        }
    }
}

fn validate_object_array(
    ctx: &mut ValidationCtx,
    root: &Value,
    dotted_path: &str,
    min_items: usize,
    required: &[&str],
) {
    let Some(value) = path_value(root, dotted_path) else {
        ctx.missing(dotted_path, "Add the required object array.");
        return;
    };
    let Some(items) = value.as_array() else {
        ctx.type_mismatch(dotted_path, "Expected an array of objects.");
        return;
    };
    if items.len() < min_items {
        ctx.missing(
            dotted_path,
            "Required arrays must contain at least one object.",
        );
    }
    for (idx, item) in items.iter().enumerate() {
        if !item.is_object() {
            ctx.type_mismatch(
                &format!("{dotted_path}[{idx}]"),
                "Array item must be an object.",
            );
            continue;
        }
        for field in required {
            if item.get(*field).is_none() {
                ctx.missing(
                    &format!("{dotted_path}[{idx}].{field}"),
                    "Add the required nested field.",
                );
            }
        }
    }
}

fn validate_enum(ctx: &mut ValidationCtx, root: &Value, dotted_path: &str, allowed: &[&str]) {
    let field = dotted_path.rsplit('.').next().unwrap_or(dotted_path);
    validate_enum_at(ctx, root, &format!("data.{field}"), allowed);
}

fn validate_enum_at(ctx: &mut ValidationCtx, root: &Value, dotted_path: &str, allowed: &[&str]) {
    let field = dotted_path.rsplit('.').next().unwrap_or(dotted_path);
    match path_value(root, dotted_path).and_then(Value::as_str) {
        Some(value) if allowed.contains(&value) => {}
        Some(_) => ctx.type_mismatch(
            dotted_path,
            &format!("{field} must be one of: {}.", allowed.join(", ")),
        ),
        None if path_value(root, dotted_path).is_some() => {
            ctx.type_mismatch(dotted_path, &format!("{field} must be a string."))
        }
        None => ctx.missing(dotted_path, "Add the required enum field."),
    }
}

fn validate_id_at(ctx: &mut ValidationCtx, root: &Value, dotted_path: &str, field: &str) {
    match path_value(root, dotted_path).and_then(Value::as_str) {
        Some(value)
            if !value.is_empty()
                && value
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')) => {}
        Some(_) => ctx.type_mismatch(
            dotted_path,
            &format!("{field} must match ^[A-Za-z0-9_.-]+$."),
        ),
        None if path_value(root, dotted_path).is_some() => {
            ctx.type_mismatch(dotted_path, &format!("{field} must be a string."))
        }
        None => ctx.missing(dotted_path, "Add the required identifier field."),
    }
}

fn validate_pattern(ctx: &mut ValidationCtx, root: &Value, field: &str, prefix: &str, hint: &str) {
    match root.get(field).and_then(Value::as_str) {
        Some(value)
            if value.starts_with(prefix)
                && value[prefix.len()..]
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-')) => {}
        Some(_) => ctx.type_mismatch(field, hint),
        None if root.get(field).is_some() => ctx.type_mismatch(field, hint),
        None => {}
    }
}

fn path_value<'a>(root: &'a Value, dotted_path: &str) -> Option<&'a Value> {
    let mut value = root;
    for part in dotted_path
        .strip_prefix("data.")
        .unwrap_or(dotted_path)
        .split('.')
    {
        match value.get(part) {
            Some(next) => value = next,
            None => {
                return dotted_path
                    .rsplit('.')
                    .next()
                    .and_then(|field| root.get(field));
            }
        }
    }
    Some(value)
}

struct ValidationCtx {
    target: String,
    errors: Vec<SchemaValidationError>,
}

impl ValidationCtx {
    fn new(target: &str) -> Self {
        Self {
            target: target.to_string(),
            errors: Vec::new(),
        }
    }

    fn missing(&mut self, path: &str, hint: &str) {
        self.push(
            "schema.required.missing",
            path,
            "Required field is missing or empty.",
            hint,
        );
    }

    fn type_mismatch(&mut self, path: &str, hint: &str) {
        self.push(
            "schema.type.mismatch",
            path,
            "Field value does not match the required schema.",
            hint,
        );
    }

    fn version(&mut self, path: &str, hint: &str) {
        self.push(
            "schema.version.unsupported",
            path,
            "Capability schema version is unsupported.",
            hint,
        );
    }

    fn relationship(&mut self, path: &str, hint: &str) {
        self.push(
            "schema.relationship.invalid",
            path,
            "Capability output violates a field relationship rule.",
            hint,
        );
    }

    fn push(&mut self, code: &str, path: &str, message: &str, hint: &str) {
        self.errors.push(SchemaValidationError {
            code: code.to_string(),
            message: format!("{message} ({path})"),
            hint: hint.to_string(),
            details: json!({
                "target": self.target,
                "path": path,
                "schema_ref": "docs/vibehub-capability-schema-v1.md",
            }),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    const WORKFLOW: &str = r#"schema_version: 2
name: schema-test
modes:
  evidence_drive:
    capabilities: [implement, review, review_lite]
default_mode: evidence_drive
capabilities:
  implement:
    required_fields: [diff_summary, changed_files, commands_run]
  review:
    required_fields: [summary, concerns, gate_pass, risk_review]
  review_lite:
    required_fields: [summary, concerns, gate_pass]
"#;

    const CUSTOM_WORKFLOW: &str = r#"schema_version: 2
name: schema-custom-test
modes:
  evidence_drive:
    capabilities: [security_audit]
default_mode: evidence_drive
custom_capabilities:
  security_audit:
    required_fields: [audit_scope, "findings[]", severity_summary]
    optional_fields: [recommendations]
    produces: [audit_report]
    consumes: [implement]
    parallel_safe: true
"#;

    fn envelope(capability: &str, data: Value) -> Value {
        json!({
            "schema_version": "1.0",
            "capability": capability,
            "task_id": "T-001",
            "run_id": "R-001",
            "created_at": "2026-05-28T00:00:00Z",
            "created_by": "agent",
            "data": data,
        })
    }

    fn temp_project() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("vibehub-schema-test-{}", Uuid::new_v4()));
        fs::create_dir_all(path.join(".vibehub/tasks/T-001/runs/R-001/outputs")).expect("create");
        crate::vibehub::current::write_current_task_pointer(&path, "T-001").expect("task pointer");
        crate::vibehub::current::write_current_run_pointer(&path, "T-001", "R-001")
            .expect("run pointer");
        fs::write(path.join(".vibehub/state.yaml"), "schema_version: 5\n").expect("state");
        fs::write(path.join(".vibehub/workflow.yaml"), WORKFLOW).expect("workflow");
        path
    }

    #[test]
    fn rejects_missing_required_fields_for_all_v1_capabilities() {
        for capability in CAPABILITIES {
            let err = validate_capability_output(capability, &envelope(capability, json!({})))
                .expect_err("missing fields should fail");
            assert!(
                err.report()
                    .errors
                    .iter()
                    .any(|error| error.code == "schema.required.missing"),
                "{capability} should report missing required fields"
            );
        }
    }

    #[test]
    fn accepts_valid_review_placeholder_output() {
        let report = validate_capability_output(
            "review",
            &envelope(
                "review",
                json!({
                    "summary": "Full review found no blocking schema issues.",
                    "concerns": [{"severity": "info", "note": "no_risk", "file": null}],
                    "gate_pass": true,
                    "risk_review": "no_risk"
                }),
            ),
        )
        .expect("valid review");
        assert!(report.valid);
    }

    #[test]
    fn returns_hints_and_relationship_errors() {
        let err = validate_capability_output(
            "review_lite",
            &envelope(
                "review_lite",
                json!({
                    "summary": "Reviewed implementation output thoroughly.",
                    "concerns": [{"severity": "blocker", "note": "Blocking concern"}],
                    "gate_pass": true
                }),
            ),
        )
        .expect_err("relationship should fail");
        let report = err.report();
        assert!(report
            .errors
            .iter()
            .any(|error| error.code == "schema.relationship.invalid" && !error.hint.is_empty()));
    }

    #[test]
    fn invalid_write_appends_schema_failure_event() {
        let project = temp_project();
        let err = write_current_capability_output(
            &project,
            "implement",
            envelope("implement", json!({"diff_summary": "short"})),
        )
        .expect_err("invalid output");
        let events =
            fs::read_to_string(project.join(".vibehub/tasks/T-001/runs/R-001/events.jsonl"))
                .unwrap_or_else(|_| panic!("events should exist after error: {err}"));
        assert!(events.contains("SchemaValidationFailed"), "{err}");
        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn custom_capability_output_uses_workflow_registered_schema() {
        let project = temp_project();
        fs::write(project.join(".vibehub/workflow.yaml"), CUSTOM_WORKFLOW).expect("workflow");

        let valid = envelope(
            "security_audit",
            json!({
                "audit_scope": "Changed authentication surface",
                "findings": ["No critical issue found"],
                "severity_summary": "low"
            }),
        );
        let report = validate_capability_output_for_project(&project, "security_audit", &valid)
            .expect("valid custom output");
        assert!(report.valid);

        let invalid = envelope(
            "security_audit",
            json!({
                "audit_scope": "Changed authentication surface",
                "findings": [],
                "severity_summary": "low"
            }),
        );
        let error = validate_capability_output_for_project(&project, "security_audit", &invalid)
            .expect_err("empty findings should fail");
        let report = error
            .downcast_ref::<SchemaValidationErrorList>()
            .expect("schema validation errors")
            .report();
        assert!(report
            .errors
            .iter()
            .any(|error| error.hint == "Custom array fields must not be empty."));
        fs::remove_dir_all(project).ok();
    }
}

use crate::vibehub::status;
use crate::vibehub::util::{
    canonical_initialized_project_root, normalize_path, relative_to_project,
};
use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PromptTemplateOption {
    pub id: String,
    pub filename: String,
    pub dangerous: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PromptRenderResult {
    pub template_id: String,
    pub template_path: String,
    pub source: String,
    pub locale: String,
    pub dangerous: bool,
    pub confirmation_required: bool,
    pub confirmation_message: Option<String>,
    pub content: String,
    pub warnings: Vec<String>,
}

const TEMPLATE_IDS: [&str; 7] = [
    "new-task",
    "sync",
    "claim-capability",
    "release-capability",
    "cancel-task",
    "force-rebuild",
    "fix-schema",
];

pub fn list_prompt_templates() -> Vec<PromptTemplateOption> {
    TEMPLATE_IDS
        .iter()
        .map(|id| PromptTemplateOption {
            id: (*id).to_string(),
            filename: format!("{id}.md"),
            dangerous: matches!(*id, "cancel-task" | "force-rebuild"),
        })
        .collect()
}

pub fn render_prompt(
    project_root: impl AsRef<Path>,
    template_id: &str,
) -> Result<PromptRenderResult> {
    validate_template_id(template_id)?;
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let cockpit_status = status::read_cockpit_status(&project_root)?;
    let locale = normalize_locale(cockpit_status.locale.as_deref());
    let (template, source, template_path) = load_template(&project_root, template_id, &locale)?;
    let task_count = cockpit_status.active_tasks.len().max(
        cockpit_status
            .current_task_id
            .as_ref()
            .map(|_| 1)
            .unwrap_or(0),
    );
    let mut context = BTreeMap::new();
    context.insert("project_root".to_string(), normalize_path(&project_root));
    context.insert(
        "task_id".to_string(),
        cockpit_status
            .current_task_id
            .clone()
            .unwrap_or_else(|| "none".to_string()),
    );
    context.insert(
        "task_title".to_string(),
        cockpit_status
            .current_task_title
            .clone()
            .unwrap_or_else(|| "Untitled task".to_string()),
    );
    context.insert(
        "run_id".to_string(),
        cockpit_status
            .current_run_id
            .clone()
            .unwrap_or_else(|| "none".to_string()),
    );
    context.insert(
        "mode".to_string(),
        cockpit_status
            .current_mode
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
    );
    context.insert(
        "phase".to_string(),
        cockpit_status
            .current_phase
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
    );
    context.insert(
        "phase_status".to_string(),
        cockpit_status
            .phase_status
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
    );
    context.insert(
        "active_capabilities".to_string(),
        if cockpit_status.active_capabilities.is_empty() {
            "none".to_string()
        } else {
            cockpit_status.active_capabilities.join(", ")
        },
    );
    context.insert(
        "changed_files_count".to_string(),
        cockpit_status
            .git_changed_files_count
            .unwrap_or_default()
            .to_string(),
    );
    context.insert("task_count".to_string(), task_count.to_string());
    context.insert(
        "active_tasks".to_string(),
        if cockpit_status.active_tasks.is_empty() {
            cockpit_status
                .current_task_id
                .clone()
                .unwrap_or_else(|| "none".to_string())
        } else {
            cockpit_status
                .active_tasks
                .iter()
                .map(|task| {
                    let title = task.title.as_deref().unwrap_or("Untitled task");
                    format!("{} ({title})", task.task_id)
                })
                .collect::<Vec<_>>()
                .join(", ")
        },
    );
    context.insert("locale".to_string(), locale.clone());

    let content = render_template(&template, &context);
    let dangerous = matches!(template_id, "cancel-task" | "force-rebuild");
    let confirmation_message = if dangerous {
        Some(match template_id {
            "cancel-task" => "Confirm that you want to generate a cancellation prompt; this prompt asks the agent to report and record a task cancellation instead of mutating state from the UI.".to_string(),
            "force-rebuild" if task_count > 1 => "Multiple active tasks are visible. Confirm the target task in the prompt before asking the agent to force rebuild derived artifacts.".to_string(),
            "force-rebuild" => "Confirm that you want to generate a force-rebuild prompt for the current task.".to_string(),
            _ => "Confirm before generating this prompt.".to_string(),
        })
    } else {
        None
    };

    Ok(PromptRenderResult {
        template_id: template_id.to_string(),
        template_path,
        source,
        locale,
        dangerous,
        confirmation_required: dangerous,
        confirmation_message,
        content,
        warnings: Vec::new(),
    })
}

fn validate_template_id(template_id: &str) -> Result<()> {
    if TEMPLATE_IDS.contains(&template_id) {
        Ok(())
    } else {
        Err(anyhow!("Unknown prompt template: {template_id}"))
    }
}

fn load_template(
    project_root: &Path,
    template_id: &str,
    locale: &str,
) -> Result<(String, String, String)> {
    let filename = format!("{template_id}.md");
    let override_path = project_root
        .join(".vibehub/templates/prompts")
        .join(&filename);
    if override_path.is_file() {
        let content = fs::read_to_string(&override_path)
            .with_context(|| format!("Failed to read {}", override_path.display()))?;
        let rel = relative_to_project(project_root, &override_path)
            .unwrap_or_else(|_| PathBuf::from(format!(".vibehub/templates/prompts/{filename}")));
        return Ok((
            content,
            "project_override".to_string(),
            normalize_path(&rel),
        ));
    }

    let content = default_template(locale, template_id)
        .or_else(|| default_template("en", template_id))
        .ok_or_else(|| anyhow!("Missing bundled prompt template: {template_id}"))?;
    Ok((
        content.to_string(),
        "product_default".to_string(),
        format!("templates/prompts/{locale}/{filename}"),
    ))
}

fn normalize_locale(locale: Option<&str>) -> String {
    match locale.unwrap_or("en") {
        "zh" | "zh-CN" | "zh-Hans" => "zh-CN".to_string(),
        "zh-TW" | "zh-Hant" => "zh-TW".to_string(),
        _ => "en".to_string(),
    }
}

fn render_template(template: &str, context: &BTreeMap<String, String>) -> String {
    let mut rendered = template.to_string();
    for (key, value) in context {
        rendered = rendered.replace(&format!("{{{{{key}}}}}"), value);
    }
    rendered
}

fn default_template(locale: &str, template_id: &str) -> Option<&'static str> {
    match (locale, template_id) {
        ("en", "new-task") => Some(include_str!("../../templates/prompts/en/new-task.md")),
        ("en", "sync") => Some(include_str!("../../templates/prompts/en/sync.md")),
        ("en", "claim-capability") => Some(include_str!(
            "../../templates/prompts/en/claim-capability.md"
        )),
        ("en", "release-capability") => Some(include_str!(
            "../../templates/prompts/en/release-capability.md"
        )),
        ("en", "cancel-task") => Some(include_str!("../../templates/prompts/en/cancel-task.md")),
        ("en", "force-rebuild") => {
            Some(include_str!("../../templates/prompts/en/force-rebuild.md"))
        }
        ("en", "fix-schema") => Some(include_str!("../../templates/prompts/en/fix-schema.md")),
        ("zh-CN", "new-task") => Some(include_str!("../../templates/prompts/zh-CN/new-task.md")),
        ("zh-CN", "sync") => Some(include_str!("../../templates/prompts/zh-CN/sync.md")),
        ("zh-CN", "claim-capability") => Some(include_str!(
            "../../templates/prompts/zh-CN/claim-capability.md"
        )),
        ("zh-CN", "release-capability") => Some(include_str!(
            "../../templates/prompts/zh-CN/release-capability.md"
        )),
        ("zh-CN", "cancel-task") => {
            Some(include_str!("../../templates/prompts/zh-CN/cancel-task.md"))
        }
        ("zh-CN", "force-rebuild") => Some(include_str!(
            "../../templates/prompts/zh-CN/force-rebuild.md"
        )),
        ("zh-CN", "fix-schema") => {
            Some(include_str!("../../templates/prompts/zh-CN/fix-schema.md"))
        }
        ("zh-TW", "new-task") => Some(include_str!("../../templates/prompts/zh-TW/new-task.md")),
        ("zh-TW", "sync") => Some(include_str!("../../templates/prompts/zh-TW/sync.md")),
        ("zh-TW", "claim-capability") => Some(include_str!(
            "../../templates/prompts/zh-TW/claim-capability.md"
        )),
        ("zh-TW", "release-capability") => Some(include_str!(
            "../../templates/prompts/zh-TW/release-capability.md"
        )),
        ("zh-TW", "cancel-task") => {
            Some(include_str!("../../templates/prompts/zh-TW/cancel-task.md"))
        }
        ("zh-TW", "force-rebuild") => Some(include_str!(
            "../../templates/prompts/zh-TW/force-rebuild.md"
        )),
        ("zh-TW", "fix-schema") => {
            Some(include_str!("../../templates/prompts/zh-TW/fix-schema.md"))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_templates_cover_all_ids_for_all_locales() {
        for locale in ["en", "zh-CN", "zh-TW"] {
            for id in TEMPLATE_IDS {
                let template = default_template(locale, id).expect("template exists");
                assert!(template.contains("{{project_root}}"));
            }
        }
    }

    #[test]
    fn renderer_replaces_known_placeholders() {
        let mut context = BTreeMap::new();
        context.insert("project_root".to_string(), "/tmp/project".to_string());
        context.insert("task_id".to_string(), "T-1".to_string());
        let rendered = render_template("Project {{project_root}} task {{task_id}}", &context);
        assert_eq!(rendered, "Project /tmp/project task T-1");
    }

    #[test]
    fn unknown_template_id_is_rejected() {
        assert!(validate_template_id("../escape").is_err());
    }
}

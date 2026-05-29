use crate::vibehub::util::{canonical_project_root, normalize_path};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

const CONFIG_PATH: &str = ".vibehub/adapters/config.yaml";
const REGISTRY_PATH: &str = ".vibehub/skills.registry.yaml";
const TEMPLATE_VERSION: &str = "2.0.0-pre.3";
const MANAGED_START: &str = "<!-- VIBEHUB:AGENT-INTEGRATION:START -->";
const MANAGED_END: &str = "<!-- VIBEHUB:AGENT-INTEGRATION:END -->";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AgentTool {
    AmpCode,
    Codex,
    ClaudeCode,
    Opencode,
    Cursor,
    Antigravity,
}

impl AgentTool {
    fn label(self) -> &'static str {
        match self {
            AgentTool::AmpCode => "Amp Code",
            AgentTool::Codex => "Codex",
            AgentTool::ClaudeCode => "Claude Code",
            AgentTool::Opencode => "OpenCode",
            AgentTool::Cursor => "Cursor",
            AgentTool::Antigravity => "Antigravity",
        }
    }

    fn id(self) -> &'static str {
        match self {
            AgentTool::AmpCode => "amp_code",
            AgentTool::Codex => "codex",
            AgentTool::ClaudeCode => "claude_code",
            AgentTool::Opencode => "opencode",
            AgentTool::Cursor => "cursor",
            AgentTool::Antigravity => "antigravity",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentAdapterConfig {
    pub schema_version: u32,
    pub template_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_schema_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_hash: Option<String>,
    pub enabled_tools: Vec<AgentTool>,
    #[serde(default)]
    pub command_overrides: BTreeMap<String, String>,
    #[serde(default)]
    pub generated_hashes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentAdapterConfigPatch {
    #[serde(default)]
    pub enabled_tools: Option<Vec<AgentTool>>,
    #[serde(default)]
    pub command_overrides: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AgentCommandSpec {
    pub name: String,
    pub description_zh: String,
    pub description_en: String,
    pub argument_hint: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AgentAdapterFileStatus {
    pub tool: String,
    pub path: String,
    pub exists: bool,
    pub status: String,
    pub generated_hash: Option<String>,
    pub current_hash: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AgentAdapterStatus {
    pub project_root: String,
    pub config_path: String,
    pub enabled_tools: Vec<AgentTool>,
    pub commands: Vec<AgentCommandSpec>,
    pub files: Vec<AgentAdapterFileStatus>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AgentAdapterSyncResult {
    pub project_root: String,
    pub created_files: Vec<String>,
    pub updated_files: Vec<String>,
    pub skipped_files: Vec<String>,
    pub conflict_files: Vec<AgentAdapterConflict>,
    pub dry_run: bool,
    pub summary: String,
    pub files: Vec<AgentAdapterFileStatus>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AgentAdapterConflict {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
struct RenderedTarget {
    tool: String,
    path: String,
    content: String,
    description: String,
    managed_region: bool,
}

enum SyncDecision {
    Create(String),
    Update(String),
    Skip,
    Conflict(String),
}

#[derive(Debug, Clone)]
struct RegistrySnapshot {
    schema_version: Option<String>,
    hash: String,
    skills: Vec<RegistrySkill>,
}

#[derive(Debug, Clone, Deserialize)]
struct SkillsRegistry {
    schema_version: Option<serde_yaml::Value>,
    #[serde(default)]
    skills: Vec<RegistrySkill>,
}

#[derive(Debug, Clone, Deserialize)]
struct RegistrySkill {
    name: String,
    #[serde(default)]
    args: Vec<RegistryArg>,
    #[serde(default)]
    returns: Option<String>,
    #[serde(default)]
    side_effects: Vec<String>,
    #[serde(default)]
    callable_by: Vec<String>,
    #[serde(default)]
    idempotent: Option<bool>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct RegistryArg {
    name: String,
    #[serde(default, rename = "type")]
    kind: Option<String>,
    #[serde(default)]
    required: Option<bool>,
    #[serde(default)]
    default: Option<serde_yaml::Value>,
}

pub fn default_tools() -> Vec<AgentTool> {
    vec![
        AgentTool::AmpCode,
        AgentTool::ClaudeCode,
        AgentTool::Codex,
        AgentTool::Opencode,
        AgentTool::Cursor,
        AgentTool::Antigravity,
    ]
}

pub fn default_config(enabled_tools: Vec<AgentTool>) -> AgentAdapterConfig {
    AgentAdapterConfig {
        schema_version: 1,
        template_version: TEMPLATE_VERSION.to_string(),
        registry_schema_version: None,
        registry_hash: None,
        enabled_tools: normalize_tools(enabled_tools),
        command_overrides: BTreeMap::new(),
        generated_hashes: BTreeMap::new(),
    }
}

pub fn ensure_adapter_config(
    project_path: impl AsRef<Path>,
    enabled_tools: Vec<AgentTool>,
) -> Result<AgentAdapterConfig> {
    let project_root = canonical_project_root(project_path.as_ref())?;
    let config_path = project_root.join(CONFIG_PATH);
    if config_path.exists() {
        return read_config_or_default(&project_root);
    }

    let config = default_config(enabled_tools);
    write_config(&project_root, &config)?;
    Ok(config)
}

pub fn update_agent_adapter_config(
    project_path: impl AsRef<Path>,
    patch: AgentAdapterConfigPatch,
) -> Result<AgentAdapterConfig> {
    let project_root = canonical_project_root(project_path.as_ref())?;
    let mut config = read_config_or_default(&project_root)?;
    if let Some(enabled_tools) = patch.enabled_tools {
        config.enabled_tools = normalize_tools(enabled_tools);
    }
    if let Some(command_overrides) = patch.command_overrides {
        config.command_overrides = command_overrides;
    }
    config.template_version = TEMPLATE_VERSION.to_string();
    write_config(&project_root, &config)?;
    Ok(config)
}

pub fn get_agent_adapter_status(project_path: impl AsRef<Path>) -> Result<AgentAdapterStatus> {
    let project_root = canonical_project_root(project_path.as_ref())?;
    let config = read_config_or_default(&project_root)?;
    let registry = read_registry_snapshot(&project_root)?;
    let targets = rendered_targets(&project_root, &config, registry.as_ref());
    let files = targets
        .iter()
        .map(|target| file_status(&project_root, &config, target))
        .collect::<Result<Vec<_>>>()?;
    let warnings = adapter_warnings(&config, registry.as_ref(), &files);

    Ok(AgentAdapterStatus {
        project_root: normalize_path(&project_root),
        config_path: CONFIG_PATH.to_string(),
        enabled_tools: config.enabled_tools.clone(),
        commands: command_specs(Some(&project_root), &config, registry.as_ref()),
        files,
        warnings,
    })
}

pub fn sync_agent_adapter(
    project_path: impl AsRef<Path>,
    dry_run: bool,
) -> Result<AgentAdapterSyncResult> {
    sync_agent_adapters(project_path, None, dry_run)
}

pub fn sync_agent_adapters(
    project_path: impl AsRef<Path>,
    tools: Option<Vec<AgentTool>>,
    dry_run: bool,
) -> Result<AgentAdapterSyncResult> {
    let project_root = canonical_project_root(project_path.as_ref())?;
    let mut config = read_config_or_default(&project_root)?;
    if let Some(tools) = tools {
        config.enabled_tools = normalize_tools(tools);
    }
    if config.enabled_tools.is_empty() {
        config.enabled_tools = default_tools();
    }

    let registry = read_registry_snapshot(&project_root)?;
    let targets = rendered_targets(&project_root, &config, registry.as_ref());
    let mut created_files = Vec::new();
    let mut updated_files = Vec::new();
    let mut skipped_files = Vec::new();
    let mut conflict_files = Vec::new();
    let mut next_hashes = config.generated_hashes.clone();

    for target in &targets {
        let absolute = project_root.join(&target.path);
        let desired_hash = hash_content(&target.content);
        let decision = decide_target(&absolute, &config, target)?;
        match decision {
            SyncDecision::Create(content) => {
                if !dry_run {
                    write_target(&absolute, &content)?;
                    next_hashes.insert(target.path.clone(), desired_hash);
                }
                created_files.push(target.path.clone());
            }
            SyncDecision::Update(content) => {
                if !dry_run {
                    write_target(&absolute, &content)?;
                    next_hashes.insert(target.path.clone(), desired_hash);
                }
                updated_files.push(target.path.clone());
            }
            SyncDecision::Skip => {
                if !dry_run {
                    next_hashes.insert(target.path.clone(), desired_hash);
                }
                skipped_files.push(target.path.clone());
            }
            SyncDecision::Conflict(reason) => conflict_files.push(AgentAdapterConflict {
                path: target.path.clone(),
                reason,
            }),
        }
    }

    if !dry_run {
        config.template_version = TEMPLATE_VERSION.to_string();
        if let Some(registry) = &registry {
            config.registry_schema_version = registry.schema_version.clone();
            config.registry_hash = Some(registry.hash.clone());
        } else {
            config.registry_schema_version = None;
            config.registry_hash = None;
        }
        config.generated_hashes = next_hashes;
        write_config(&project_root, &config)?;
    }

    let files = rendered_targets(&project_root, &config, registry.as_ref())
        .iter()
        .map(|target| file_status(&project_root, &config, target))
        .collect::<Result<Vec<_>>>()?;
    let summary = if dry_run {
        format!(
            "AI instruction dry run: would create {}, would update {}, already current {}, conflicts {}. No files were written.",
            created_files.len(),
            updated_files.len(),
            skipped_files.len(),
            conflict_files.len()
        )
    } else {
        format!(
            "AI instruction sync complete: created {}, updated {}, skipped {}, conflicts {}.",
            created_files.len(),
            updated_files.len(),
            skipped_files.len(),
            conflict_files.len()
        )
    };

    Ok(AgentAdapterSyncResult {
        project_root: normalize_path(&project_root),
        created_files,
        updated_files,
        skipped_files,
        conflict_files,
        dry_run,
        summary,
        files,
    })
}

fn command_specs(
    _project_root: Option<&Path>,
    config: &AgentAdapterConfig,
    registry: Option<&RegistrySnapshot>,
) -> Vec<AgentCommandSpec> {
    let mut specs = BTreeMap::new();
    for definition in command_definitions_v2() {
        specs.insert(
            definition.name.to_string(),
            AgentCommandSpec {
                name: definition.name.to_string(),
                description_zh: definition.zh.to_string(),
                description_en: definition.en.to_string(),
                argument_hint: definition.argument_hint.to_string(),
                body: default_command_body(definition.name),
            },
        );
    }

    if let Some(registry) = registry {
        for skill in &registry.skills {
            let definition = command_definitions_v2()
                .into_iter()
                .find(|definition| definition.name == skill.name);
            let body = if definition.is_some() {
                let mut body = default_command_body(&skill.name);
                body.push_str("\nRegistry contract:\n");
                body.push_str(&registry_contract_summary(skill));
                body
            } else {
                registry_command_body(skill)
            };
            let description = skill
                .description
                .clone()
                .unwrap_or_else(|| format!("Run the {} VibeHub skill.", skill.name));
            specs.insert(
                skill.name.clone(),
                AgentCommandSpec {
                    name: skill.name.clone(),
                    description_zh: definition
                        .as_ref()
                        .map(|definition| definition.zh.to_string())
                        .unwrap_or_else(|| format!("执行 {} skill。", skill.name)),
                    description_en: description,
                    argument_hint: registry_argument_hint(&skill.args),
                    body,
                },
            );
        }
    }

    for (name, override_body) in &config.command_overrides {
        if let Some(spec) = specs.get_mut(name) {
            spec.body = override_body.clone();
        }
    }

    specs.into_values().collect()
}

fn rendered_targets(
    project_root: &Path,
    config: &AgentAdapterConfig,
    registry: Option<&RegistrySnapshot>,
) -> Vec<RenderedTarget> {
    let tools: BTreeSet<AgentTool> = config.enabled_tools.iter().copied().collect();
    let registry_marker = registry_marker(registry);
    let mut targets = Vec::new();

    if !tools.is_empty() {
        targets.push(RenderedTarget {
            tool: "shared".to_string(),
            path: ".vibehub/adapters/protocol.md".to_string(),
            content: build_adapter_protocol(&registry_marker),
            description: "Shared VibeHub agent protocol and output contract".to_string(),
            managed_region: false,
        });
        targets.push(RenderedTarget {
            tool: "shared".to_string(),
            path: ".vibehub/adapters/hooks/vibehub-stop-check.mjs".to_string(),
            content: render_stop_check_hook(),
            description: "VibeHub Stop hook that blocks missing phase output".to_string(),
            managed_region: false,
        });
    }

    if tools.contains(&AgentTool::AmpCode)
        || tools.contains(&AgentTool::Codex)
        || tools.contains(&AgentTool::Opencode)
    {
        targets.push(RenderedTarget {
            tool: "shared".to_string(),
            path: "AGENTS.md".to_string(),
            content: build_static_protocol(
                "AGENTS.md",
                &tools
                    .iter()
                    .filter(|tool| {
                        matches!(
                            tool,
                            AgentTool::AmpCode | AgentTool::Codex | AgentTool::Opencode
                        )
                    })
                    .copied()
                    .collect::<Vec<_>>(),
                &registry_marker,
            ),
            description: "Shared Amp/Codex/OpenCode project instructions".to_string(),
            managed_region: true,
        });
    }
    if tools.contains(&AgentTool::ClaudeCode) {
        targets.push(RenderedTarget {
            tool: AgentTool::ClaudeCode.id().to_string(),
            path: "CLAUDE.md".to_string(),
            content: build_static_protocol("CLAUDE.md", &[AgentTool::ClaudeCode], &registry_marker),
            description: "Claude Code project memory".to_string(),
            managed_region: true,
        });
    }

    for command in command_specs(Some(project_root), config, registry) {
        if tools.contains(&AgentTool::Codex) {
            targets.push(RenderedTarget {
                tool: AgentTool::Codex.id().to_string(),
                path: format!(".agents/skills/{}/SKILL.md", command.name),
                content: render_codex_skill(&command),
                description: format!("Codex VibeHub skill for {}", command.name),
                managed_region: false,
            });
            targets.push(RenderedTarget {
                tool: AgentTool::Codex.id().to_string(),
                path: format!(".vibehub/adapters/generated/codex/{}.md", command.name),
                content: render_codex_skill(&command),
                description: format!("Generated Codex skill source for {}", command.name),
                managed_region: false,
            });
        }
        if tools.contains(&AgentTool::ClaudeCode) {
            targets.push(RenderedTarget {
                tool: AgentTool::ClaudeCode.id().to_string(),
                path: format!(".claude/commands/{}.md", command.name),
                content: render_claude_command(&command),
                description: format!("Claude Code slash command for {}", command.name),
                managed_region: false,
            });
        }
        if tools.contains(&AgentTool::Opencode) {
            targets.push(RenderedTarget {
                tool: AgentTool::Opencode.id().to_string(),
                path: format!(".opencode/commands/{}.md", command.name),
                content: render_opencode_command(&command),
                description: format!("OpenCode command for {}", command.name),
                managed_region: false,
            });
        }
    }

    if tools.contains(&AgentTool::Codex) {
        targets.push(RenderedTarget {
            tool: AgentTool::Codex.id().to_string(),
            path: ".codex/vibehub/constraints.md".to_string(),
            content: build_platform_constraints(AgentTool::Codex, &registry_marker),
            description: "Codex-readable VibeHub constraints and file protocol".to_string(),
            managed_region: false,
        });
        targets.push(RenderedTarget {
            tool: AgentTool::Codex.id().to_string(),
            path: ".codex/vibehub/command-index.md".to_string(),
            content: build_command_index(AgentTool::Codex, project_root, config, registry),
            description: "Codex VibeHub command index".to_string(),
            managed_region: false,
        });
        targets.push(RenderedTarget {
            tool: AgentTool::Codex.id().to_string(),
            path: ".codex/vibehub/stop-hook-config.md".to_string(),
            content: build_codex_hook_config(),
            description: "Codex Stop hook config snippet for VibeHub output enforcement"
                .to_string(),
            managed_region: false,
        });
    }
    if tools.contains(&AgentTool::ClaudeCode) {
        targets.push(RenderedTarget {
            tool: AgentTool::ClaudeCode.id().to_string(),
            path: ".claude/settings.json".to_string(),
            content: build_claude_settings(),
            description: "Claude Code project settings with VibeHub Stop hook".to_string(),
            managed_region: false,
        });
        targets.push(RenderedTarget {
            tool: AgentTool::ClaudeCode.id().to_string(),
            path: ".claude/vibehub/constraints.md".to_string(),
            content: build_platform_constraints(AgentTool::ClaudeCode, &registry_marker),
            description: "Claude Code-readable VibeHub constraints and file protocol".to_string(),
            managed_region: false,
        });
        targets.push(RenderedTarget {
            tool: AgentTool::ClaudeCode.id().to_string(),
            path: ".claude/vibehub/command-index.md".to_string(),
            content: build_command_index(AgentTool::ClaudeCode, project_root, config, registry),
            description: "Claude Code VibeHub command index".to_string(),
            managed_region: false,
        });
    }
    if tools.contains(&AgentTool::Opencode) {
        targets.push(RenderedTarget {
            tool: AgentTool::Opencode.id().to_string(),
            path: "opencode.json".to_string(),
            content: build_opencode_config(),
            description: "OpenCode project config loading the shared VibeHub protocol".to_string(),
            managed_region: false,
        });
        targets.push(RenderedTarget {
            tool: AgentTool::Opencode.id().to_string(),
            path: ".opencode/vibehub/constraints.md".to_string(),
            content: build_platform_constraints(AgentTool::Opencode, &registry_marker),
            description: "OpenCode-readable VibeHub constraints and file protocol".to_string(),
            managed_region: false,
        });
        targets.push(RenderedTarget {
            tool: AgentTool::Opencode.id().to_string(),
            path: ".opencode/vibehub/command-index.md".to_string(),
            content: build_command_index(AgentTool::Opencode, project_root, config, registry),
            description: "OpenCode VibeHub command index".to_string(),
            managed_region: false,
        });
    }
    if tools.contains(&AgentTool::AmpCode) {
        targets.push(RenderedTarget {
            tool: AgentTool::AmpCode.id().to_string(),
            path: ".vibehub/adapters/generated/amp-code/command-index.md".to_string(),
            content: build_command_index(AgentTool::AmpCode, project_root, config, registry),
            description: "Amp Code VibeHub command index".to_string(),
            managed_region: false,
        });
    }
    if tools.contains(&AgentTool::Cursor) {
        targets.push(RenderedTarget {
            tool: AgentTool::Cursor.id().to_string(),
            path: ".cursor/rules/vibehub.mdc".to_string(),
            content: build_cursor_rule(&registry_marker),
            description: "Cursor VibeHub project rule".to_string(),
            managed_region: false,
        });
        targets.push(RenderedTarget {
            tool: AgentTool::Cursor.id().to_string(),
            path: ".cursor/rules/vibehub-command-index.mdc".to_string(),
            content: build_command_index(AgentTool::Cursor, project_root, config, registry),
            description: "Cursor VibeHub command index rule".to_string(),
            managed_region: false,
        });
    }
    if tools.contains(&AgentTool::Antigravity) {
        targets.push(RenderedTarget {
            tool: AgentTool::Antigravity.id().to_string(),
            path: ".antigravity/vibehub-instructions.md".to_string(),
            content: build_antigravity_instructions(&registry_marker),
            description: "Antigravity VibeHub instructions".to_string(),
            managed_region: false,
        });
        targets.push(RenderedTarget {
            tool: AgentTool::Antigravity.id().to_string(),
            path: ".antigravity/vibehub-command-index.md".to_string(),
            content: build_command_index(AgentTool::Antigravity, project_root, config, registry),
            description: "Antigravity VibeHub command index".to_string(),
            managed_region: false,
        });
    }

    targets
}

fn registry_marker(registry: Option<&RegistrySnapshot>) -> String {
    match registry {
        Some(registry) => format!(
            "Adapter template: {TEMPLATE_VERSION}\nSkills registry: {} ({})",
            registry
                .schema_version
                .as_deref()
                .unwrap_or("unknown-schema"),
            &registry.hash[..12]
        ),
        None => format!(
            "Adapter template: {TEMPLATE_VERSION}\nSkills registry: missing ({REGISTRY_PATH})"
        ),
    }
}

fn build_static_protocol(file_label: &str, tools: &[AgentTool], registry_marker: &str) -> String {
    let labels = tools
        .iter()
        .map(|tool| tool.label())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        r#"{MANAGED_START}
# VibeHub Agent Protocol

Applies to: {labels}
Source file: {file_label}
{registry_marker}

VibeHub owns project state. Agent output is reported state only.

## Read Before Work

1. `.vibehub/agent-view/current.md`
2. `.vibehub/agent-view/current-context.md`
3. `.vibehub/agent-view/handoff.md`
4. `.vibehub/rules/hard-rules.md`

## Rules

- Treat `.vibehub/agent-view/current.md` as the dynamic entry point.
- Also read `.vibehub/adapters/protocol.md` when present; it is the shared output contract.
- Do not edit `.vibehub/state.yaml` or canonical task/run pointers directly.
- Before ending work on an active VibeHub task, write phase output to the active run output path described in `.vibehub/agent-view/current.md`.
- Report changed files, files read, commands run, tests run or reason not run, risks, and handoff notes.
- Use evidence labels: `hard_observed`, `agent_reported`, `inferred`, `user_confirmed`.
- If workspace state changed outside VibeHub, run the `vibehub-sync` instruction and return a sync report instead of silently advancing state.
- Treat plain-language requests like "sync", "sycn", "同步", "update VibeHub", "继续", or "refresh status" as `vibehub-sync` unless the user clearly asks for a different command.
- During sync, autonomously inspect hard evidence first; ask concise follow-up questions only for missing user intent, current progress, ownership of dirty changes, validation status, or future plan.
- If the user does not provide the requested details, record the open questions as unresolved risks in the VibeHub output instead of dropping them.

## Command Namespace

Use generated `vibehub-*` commands where supported. They describe VibeHub-specific work without replacing built-in agent commands.
{MANAGED_END}
"#
    )
}

fn build_adapter_protocol(registry_marker: &str) -> String {
    format!(
        r#"# VibeHub Agent Protocol

{registry_marker}

This file is generated by VibeHub and shared by Codex, Claude Code, OpenCode, and other coding agents.

## Authority

- VibeHub owns canonical project state.
- Agent output is reported state only.
- Do not edit `.vibehub/state.yaml`, `.vibehub/tasks/current`, or `.vibehub/tasks/*/runs/current`.
- Do not mark canonical task, run, or phase state complete from an agent session.

## Required Startup Read

Read these files before VibeHub-scoped work:

1. `.vibehub/agent-view/current.md`
2. `.vibehub/agent-view/current-context.md`
3. `.vibehub/agent-view/handoff.md`
4. `.vibehub/rules/hard-rules.md`

If `.vibehub/agent-view/current.md` names a context pack, read it when it exists. If it is missing, report that as `hard_observed` missing context instead of pretending context is complete.

## Phase Output Contract

Before ending work on an active VibeHub task, write an agent output file. Preferred path:

`.vibehub/tasks/<task_id>/runs/<run_id>/outputs/output.md`

Session-specific output is also accepted when the tool provides a session id:

`.vibehub/tasks/<task_id>/runs/<run_id>/sessions/<session_id>/output.md`

The output file must contain these sections:

- `## Completed`
- `## Not Yet Done`
- `## Key Decisions Made`
- `## Files Changed`
- `## Files Reportedly Read`
- `## Commands Run`
- `## Tests Run`
- `## Context Still Needed`
- `## Warnings`
- `## Next Session Should`

Every section should use evidence labels where useful: `hard_observed`, `agent_reported`, `inferred`, `user_confirmed`.

## Stop Condition

If the output file is missing or any required section is empty, continue working only to produce the missing VibeHub output. If you cannot write it, stop and report the blocker.

If workspace state changed outside the current VibeHub task, run the `vibehub-sync` instruction and return a sync report. If Git HEAD, task pointers, or context become inconsistent, run `vibehub-recover` and return a recover report.
Plain-language sync requests, including misspellings such as `sycn` and Chinese requests such as `同步当前状态`, should be handled as `vibehub-sync`.

## Tool Notes

- Codex: `AGENTS.md` loads project instructions. VibeHub also generates a Stop hook config snippet under `.codex/vibehub/stop-hook-config.md`.
- Claude Code: `CLAUDE.md` loads project instructions. VibeHub generates `.claude/settings.json` with a Stop hook.
- OpenCode: `AGENTS.md`, `opencode.json`, and `.opencode/commands/*.md` load rules and commands. OpenCode has no generated Stop hook here, so VibeHub validates output from files after the run.
- Amp Code: `AGENTS.md` loads shared project instructions.
- Cursor: `.cursor/rules/vibehub*.mdc` loads VibeHub rules and command index.
- Antigravity: `.antigravity/vibehub-*.md` loads VibeHub instructions and command index.

## Write Boundary

Adapter file writes are the only INV-6 UI-write exception. They may update generated adapter configuration files after user confirmation, but must not write VibeHub canonical task, run, phase, or event state from the UI.
"#
    )
}

fn render_codex_skill(command: &AgentCommandSpec) -> String {
    let body = command.body.trim_end();
    format!(
        r#"---
name: {name}
description: "{description}"
---

# {name}

中文: {zh}
English: {en}

Invocation input: {hint}

{body}
"#,
        name = command.name,
        description = yaml_double_quoted(&format!(
            "{} Use for VibeHub workflow step: {}.",
            command.description_en, command.name
        )),
        zh = command.description_zh,
        en = command.description_en,
        hint = command.argument_hint,
        body = body
    )
}

fn build_platform_constraints(tool: AgentTool, registry_marker: &str) -> String {
    let label = tool.label();
    let command_location = match tool {
        AgentTool::AmpCode => "AGENTS.md shared project instructions",
        AgentTool::Codex => ".agents/skills/vibehub-*/SKILL.md repo skills",
        AgentTool::ClaudeCode => ".claude/commands/vibehub-*.md slash commands",
        AgentTool::Opencode => ".opencode/commands/vibehub-*.md commands",
        AgentTool::Cursor => ".cursor/rules/vibehub*.mdc rules",
        AgentTool::Antigravity => ".antigravity/vibehub-*.md instructions",
    };
    let static_entry = match tool {
        AgentTool::ClaudeCode => "CLAUDE.md",
        AgentTool::AmpCode | AgentTool::Codex | AgentTool::Opencode => "AGENTS.md",
        AgentTool::Cursor => ".cursor/rules/vibehub.mdc",
        AgentTool::Antigravity => ".antigravity/vibehub-instructions.md",
    };
    format!(
        r#"# VibeHub Constraints for {label}

{registry_marker}

Read these files before VibeHub-scoped work:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

{label} may inspect and change project files within the active task scope.
{label} must not edit `.vibehub/state.yaml`, current task/run pointer files, or mark canonical state complete.

Use `{command_location}` for VibeHub operations.
Use `{static_entry}` as the automatic static protocol entry.

Before ending work on an active VibeHub task, write the phase output file required by `.vibehub/adapters/protocol.md`.

If work happened outside VibeHub, use `vibehub-sync` and return a sync report.
If state or Git HEAD drifted, run `vibehub-recover` and return a recover report.

Adapter writes are the only INV-6 UI-write exception and are limited to generated adapter configuration files.
"#
    )
}

fn build_codex_hook_config() -> String {
    r#"# VibeHub Codex Stop Hook

Codex project instructions load from `AGENTS.md`. For stricter stop-time enforcement, add this hook to your Codex config after confirming hooks are enabled in your environment:

```toml
[features]
codex_hooks = true

[[hooks.Stop]]
[[hooks.Stop.hooks]]
type = "command"
command = 'node "$(git rev-parse --show-toplevel)/.vibehub/adapters/hooks/vibehub-stop-check.mjs"'
timeout = 30
statusMessage = "Checking VibeHub output"
```

The hook blocks Stop when an active VibeHub task has no complete `output.md`.
"#
    .to_string()
}

fn build_claude_settings() -> String {
    r#"{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "node \"$CLAUDE_PROJECT_DIR/.vibehub/adapters/hooks/vibehub-stop-check.mjs\"",
            "timeout": 30
          }
        ]
      }
    ]
  }
}
"#
    .to_string()
}

fn build_opencode_config() -> String {
    r#"{
  "$schema": "https://opencode.ai/config.json",
  "instructions": [".vibehub/adapters/protocol.md"]
}
"#
    .to_string()
}

fn build_command_index(
    tool: AgentTool,
    project_root: &Path,
    config: &AgentAdapterConfig,
    registry: Option<&RegistrySnapshot>,
) -> String {
    let label = tool.label();
    let command_root = match tool {
        AgentTool::AmpCode => "AGENTS.md",
        AgentTool::Codex => ".agents/skills/",
        AgentTool::ClaudeCode => ".claude/commands/",
        AgentTool::Opencode => ".opencode/commands/",
        AgentTool::Cursor => ".cursor/rules/",
        AgentTool::Antigravity => ".antigravity/",
    };
    let mut output =
        format!("# VibeHub Command Index for {label}\n\nCommand root: `{command_root}`\n\n");
    for command in command_specs(Some(project_root), config, registry) {
        output.push_str(&format!(
            "- `{}`: {} / {}\n",
            command.name, command.description_zh, command.description_en
        ));
    }
    output
}

fn build_cursor_rule(registry_marker: &str) -> String {
    format!(
        r#"---
description: VibeHub agent protocol
alwaysApply: true
---

# VibeHub Cursor Rule

{registry_marker}

Read `.vibehub/adapters/protocol.md`, `.vibehub/agent-view/current.md`, `.vibehub/agent-view/current-context.md`, `.vibehub/agent-view/handoff.md`, and `.vibehub/rules/hard-rules.md` before VibeHub-scoped work.

Do not edit `.vibehub/state.yaml` or canonical task/run pointers directly. Write the active run output before ending work. Adapter writes are the only INV-6 UI-write exception and are limited to generated adapter configuration files.
"#
    )
}

fn build_antigravity_instructions(registry_marker: &str) -> String {
    format!(
        r#"# VibeHub Antigravity Instructions

{registry_marker}

Read `.vibehub/adapters/protocol.md`, `.vibehub/agent-view/current.md`, `.vibehub/agent-view/current-context.md`, `.vibehub/agent-view/handoff.md`, and `.vibehub/rules/hard-rules.md` before VibeHub-scoped work.

Do not edit `.vibehub/state.yaml` or canonical task/run pointers directly. Write the active run output before ending work. Adapter writes are the only INV-6 UI-write exception and are limited to generated adapter configuration files.
"#
    )
}

fn render_claude_command(command: &AgentCommandSpec) -> String {
    format!(
        r#"---
description: "{zh} / {en}"
argument-hint: "{hint}"
---

{body}
"#,
        zh = command.description_zh.replace('"', "'"),
        en = command.description_en.replace('"', "'"),
        hint = command.argument_hint.replace('"', "'"),
        body = command.body
    )
}

fn render_opencode_command(command: &AgentCommandSpec) -> String {
    format!(
        r#"---
description: "{zh} / {en}"
---

{body}
"#,
        zh = command.description_zh.replace('"', "'"),
        en = command.description_en.replace('"', "'"),
        body = command.body
    )
}

fn render_stop_check_hook() -> String {
    r#"#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";

const REQUIRED_SECTIONS = [
  "Completed",
  "Not Yet Done",
  "Key Decisions Made",
  "Files Changed",
  "Files Reportedly Read",
  "Commands Run",
  "Tests Run",
  "Context Still Needed",
  "Warnings",
  "Next Session Should",
];

function readStdinJson() {
  try {
    const input = fs.readFileSync(0, "utf8").trim();
    return input ? JSON.parse(input) : {};
  } catch {
    return {};
  }
}

function findProjectRoot(input) {
  const starts = [
    process.env.VIBEHUB_PROJECT_ROOT,
    process.env.CLAUDE_PROJECT_DIR,
    input.cwd,
    process.cwd(),
  ].filter(Boolean);
  for (const start of starts) {
    let current = path.resolve(start);
    while (true) {
      if (fs.existsSync(path.join(current, ".vibehub"))) return current;
      const parent = path.dirname(current);
      if (parent === current) break;
      current = parent;
    }
  }
  return null;
}

function readYamlString(content, key) {
  const match = content.match(new RegExp(`^\\s*${key}:\\s*['"]?([^'"\\r\\n#]+)`, "m"));
  return match ? match[1].trim() : null;
}

function readCurrent(root) {
  const statePath = path.join(root, ".vibehub", "state.yaml");
  if (!fs.existsSync(statePath)) return null;
  const state = fs.readFileSync(statePath, "utf8");
  let taskId = readYamlString(state, "task_id");
  let runId = readYamlString(state, "run_id");
  const phase = readYamlString(state, "phase");
  const phaseStatus = readYamlString(state, "phase_status") || readYamlString(state, "status");

  if (!taskId) {
    const currentTask = path.join(root, ".vibehub", "tasks", "current");
    if (fs.existsSync(currentTask)) taskId = readYamlString(fs.readFileSync(currentTask, "utf8"), "task_id");
  }
  if (taskId && !runId) {
    const currentRun = path.join(root, ".vibehub", "tasks", taskId, "runs", "current");
    if (fs.existsSync(currentRun)) runId = readYamlString(fs.readFileSync(currentRun, "utf8"), "run_id");
  }
  return taskId && runId ? { taskId, runId, phase, phaseStatus } : null;
}

function outputCandidates(root, current) {
  const runDir = path.join(root, ".vibehub", "tasks", current.taskId, "runs", current.runId);
  const candidates = [path.join(runDir, "outputs", "output.md")];
  const sessionsDir = path.join(runDir, "sessions");
  if (fs.existsSync(sessionsDir)) {
    for (const entry of fs.readdirSync(sessionsDir, { withFileTypes: true })) {
      if (entry.isDirectory()) candidates.push(path.join(sessionsDir, entry.name, "output.md"));
    }
  }
  return candidates;
}

function missingSections(content) {
  return REQUIRED_SECTIONS.filter((section) => {
    const escaped = section.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    // Match exact English title or bilingual format: "English / Translation"
    const re = new RegExp(`^##\\s+${escaped}(\\s*/\\s*[^\\s].*?)?\\s*$([\\s\\S]*?)(?=^##\\s+|(?![\\s\\S]))`, "mi");
    const match = content.match(re);
    return !match || !match[2].trim();
  });
}

function hasGitChanges(root) {
  try {
    const output = execFileSync("git", ["-C", root, "status", "--porcelain"], { encoding: "utf8" });
    return output.trim().length > 0;
  } catch {
    return true;
  }
}

function block(reason) {
  if (process.env.CLAUDE_PROJECT_DIR) {
    console.error(reason);
    process.exit(2);
  }
  process.stdout.write(JSON.stringify({ decision: "block", reason }));
  process.exit(0);
}

const input = readStdinJson();
const root = findProjectRoot(input);
if (!root) process.exit(0);
if (process.env.VIBEHUB_ALLOW_NO_OUTPUT === "1") process.exit(0);

const current = readCurrent(root);
if (!current) process.exit(0);

const candidates = outputCandidates(root, current).filter((candidate) => fs.existsSync(candidate));
if (candidates.length === 0) {
  const preferred = path.join(".vibehub", "tasks", current.taskId, "runs", current.runId, "outputs", "output.md");
  block(`VibeHub active task ${current.taskId}/${current.runId} requires ${preferred} before ending. Write the required phase output sections first.`);
}

candidates.sort((a, b) => fs.statSync(b).mtimeMs - fs.statSync(a).mtimeMs);
const latest = candidates[0];
const missing = missingSections(fs.readFileSync(latest, "utf8"));
if (missing.length > 0) {
  block(`VibeHub output is incomplete: ${path.relative(root, latest)} is missing non-empty sections: ${missing.join(", ")}.`);
}

if (!hasGitChanges(root)) process.exit(0);
process.exit(0);
"#
    .to_string()
}

fn yaml_double_quoted(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn registry_argument_hint(args: &[RegistryArg]) -> String {
    if args.is_empty() {
        return String::new();
    }
    args.iter()
        .map(|arg| {
            if arg.required.unwrap_or(false) {
                format!("<{}>", arg.name)
            } else {
                format!("[{}]", arg.name)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn registry_contract_summary(skill: &RegistrySkill) -> String {
    let args = registry_argument_hint(&skill.args);
    let returns = skill.returns.as_deref().unwrap_or("unspecified");
    let callable_by = if skill.callable_by.is_empty() {
        "unspecified".to_string()
    } else {
        skill.callable_by.join(", ")
    };
    let side_effects = if skill.side_effects.is_empty() {
        "none".to_string()
    } else {
        skill.side_effects.join(", ")
    };
    let idempotent = skill
        .idempotent
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unspecified".to_string());
    format!(
        "- name: `{}`\n- args: `{}`\n- returns: `{returns}`\n- callable_by: {callable_by}\n- side_effects: {side_effects}\n- idempotent: {idempotent}\n",
        skill.name,
        if args.is_empty() { "none" } else { &args }
    )
}

fn registry_command_body(skill: &RegistrySkill) -> String {
    let description = skill
        .description
        .as_deref()
        .unwrap_or("Run this VibeHub skill.");
    let args = if skill.args.is_empty() {
        "- none".to_string()
    } else {
        skill
            .args
            .iter()
            .map(|arg| {
                let required = if arg.required.unwrap_or(false) {
                    "required"
                } else {
                    "optional"
                };
                let default = arg
                    .default
                    .as_ref()
                    .map(|value| format!(", default={}", yaml_value_to_string(value)))
                    .unwrap_or_default();
                format!(
                    "- `{}`: {}, type={}{}",
                    arg.name,
                    required,
                    arg.kind.as_deref().unwrap_or("any"),
                    default
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let callable_by = if skill.callable_by.is_empty() {
        "unspecified".to_string()
    } else {
        skill.callable_by.join(", ")
    };
    let side_effects = if skill.side_effects.is_empty() {
        "none".to_string()
    } else {
        skill.side_effects.join(", ")
    };
    let returns = skill.returns.as_deref().unwrap_or("unspecified");
    let idempotent = skill
        .idempotent
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unspecified".to_string());

    format!(
        r#"Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

Skill contract:
- name: `{name}`
- description: {description}
- returns: `{returns}`
- callable_by: {callable_by}
- side_effects: {side_effects}
- idempotent: {idempotent}

Arguments:
{args}

Task:
Use the VibeHub CLI or app command surface for `{name}` when available. Keep changes scoped to the active task and follow the shared output contract.

Output requirements:
- write the active run phase output before ending work:
  `.vibehub/tasks/<task_id>/runs/<run_id>/outputs/output.md`
- changed files, if any
- files read
- commands run
- tests run, or reason not run
- evidence labels: `hard_observed`, `agent_reported`, `inferred`, `user_confirmed`
- unresolved risks
- handoff notes or recommended VibeHub action

Constraints:
- Do not edit `.vibehub/state.yaml` or canonical task/run pointers directly.
- Do not claim runtime observation unless a runtime adapter captured it.
- Adapter writes are the only INV-6 UI-write exception and are limited to generated adapter configuration files.
"#,
        name = skill.name,
        description = description,
        returns = returns,
        callable_by = callable_by,
        side_effects = side_effects,
        idempotent = idempotent,
        args = args
    )
}

fn default_command_body(name: &str) -> String {
    let extra = match name {
        "vibehub-help" => "List available VibeHub commands and explain when to use each one.",
        "vibehub-init" => "Check whether `.vibehub/` exists. If it is missing, ask the user to initialize from the VibeHub app; do not create canonical state yourself.",
        "vibehub-status" => "Summarize current task, run, phase, context pack, handoff, Git status, and visible warnings.",
        "vibehub-sync" => "Run a best-effort sync of actual project state into VibeHub. Inspect Git diff/status, VibeHub pointers, current context, latest output, and handoff. If a VibeHub CLI is available, prefer `vibehub sync <project_path>` or `sync <project_path>`; accept `sycn` as a typo alias. Then summarize the generated `.vibehub/agent-view/sync.md` report. Ask the user only for missing intent/progress/future-plan details that cannot be inferred from hard evidence. If the user does not answer, record the questions as unresolved risks and continue from hard_observed evidence.",
        "vibehub-diff" => "Summarize changed files and scope drift against the current task and context pack.",
        "vibehub-start" => "Convert the user's request into a VibeHub task draft with goal, acceptance criteria, mode suggestion, and context candidates.",
        "vibehub-context" => "Inspect current context quality and propose missing files or stale context rebuilds.",
        "vibehub-research" => "Run evidence-backed research, cite sources when external facts are used, and write research notes for VibeHub review.",
        "vibehub-plan" => "Create or repair the implementation plan, validation plan, risk list, and context plan.",
        "vibehub-claim" => "Claim a named capability only after VibeHub gates allow it. Prefer `claim <project_path> <capability>` or `vibehub claim <project_path> <capability>` when the CLI is available, then read the rebuilt capability context pack and continue the work.",
        "vibehub-continue" => "Continue the active phase using current.md and the context pack. Keep changes scoped to the active task.",
        "vibehub-checkpoint" => "Capture progress, decisions, commands, tests, changed files, risks, and next steps without marking state complete. Also write the current phase snapshot to `runs/<run_id>/phases/<phase>.output.md`, update `.vibehub/notes/status.md` with a one-sentence current status, and update `.vibehub/notes/summary.md` only if project scope changed.",
        "vibehub-review" => "Review the current diff against context, plan, research, tests, and VibeHub hard rules.",
        "vibehub-handoff" => "Create handoff notes that let the next session resume without chat history.",
        "vibehub-recover" => "Analyze interrupted or drifted work and produce a recover report with safe next actions.",
        "vibehub-finish" => "Summarize completion evidence and recommend whether VibeHub should advance state, request review, or recover. Also write the current phase snapshot to `runs/<run_id>/phases/<phase>.output.md`, update `.vibehub/notes/status.md` with a one-sentence handoff-ready status, and update `.vibehub/notes/summary.md` only if project scope changed.",
        "vibehub-journal" => "Draft durable session notes suitable for VibeHub journal promotion.",
        "vibehub-knowledge" => "Promote repeated lessons into reusable rules, preferences, or knowledge notes.",
        "vibehub-debug-dump" => "Export a redacted VibeHub debug bundle. Prefer `debug-dump <project_path>` or `vibehub debug-dump <project_path>` when the CLI is available.",
        _ => "Follow the VibeHub agent protocol.",
    };
    let sync_behavior = if name == "vibehub-sync" {
        r#"
Sync behavior:
- Treat "sync", "sycn", "同步", "刷新状态", "update VibeHub", or plain requests to continue from current engineering reality as this command.
- Autonomously collect hard evidence first: Git status/diff, current task/run/phase, context pack state, latest output, handoff, and visible warnings.
- Ask concise follow-up questions when needed: current progress, whether dirty files belong to this task, validation/test status, unresolved risks, and next plan.
- Do not block the sync when the user gives no answer; write the open questions and inferred risk into the output.
"#
    } else {
        ""
    };
    let lifecycle_artifacts = if matches!(name, "vibehub-checkpoint" | "vibehub-finish") {
        r#"
Agent-written lifecycle artifacts:
- Mirror the phase output into `.vibehub/tasks/<task_id>/runs/<run_id>/phases/<phase>.output.md`.
- Update `.vibehub/notes/status.md` with exactly one current-status sentence at the end of the session.
- Update `.vibehub/notes/summary.md` only when the project scope or goal changes.
- These files are agent-owned business artifacts; VibeHub should read them, not generate them.
"#
    } else {
        ""
    };
    format!(
        r#"Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`
- `.vibehub/adapters/protocol.md`

Task:
{extra}
{sync_behavior}
{lifecycle_artifacts}

Output requirements:
- write the active run phase output before ending work:
  `.vibehub/tasks/<task_id>/runs/<run_id>/outputs/output.md`
- changed files, if any
- files read
- commands run
- tests run, or reason not run
- evidence labels: `hard_observed`, `agent_reported`, `inferred`, `user_confirmed`
- unresolved risks
- handoff notes or recommended VibeHub action

Constraints:
- Do not edit `.vibehub/state.yaml` or canonical task/run pointers directly.
- Do not claim runtime observation unless a runtime adapter captured it.
- If state is stale or drifted, report it and recommend VibeHub sync/recover instead of silently advancing state.
"#
    )
}

struct CommandDefinition {
    name: &'static str,
    zh: &'static str,
    en: &'static str,
    argument_hint: &'static str,
}

fn command_definitions_v2() -> Vec<CommandDefinition> {
    vec![
        CommandDefinition {
            name: "vibehub-help",
            zh: "显示 VibeHub 命令索引。",
            en: "Show VibeHub command index.",
            argument_hint: "",
        },
        CommandDefinition {
            name: "vibehub-init",
            zh: "检查项目是否已连接 VibeHub。",
            en: "Check whether this project is connected to VibeHub.",
            argument_hint: "",
        },
        CommandDefinition {
            name: "vibehub-status",
            zh: "读取当前任务、阶段、上下文、Git 和 handoff 状态。",
            en: "Read current task, phase, context, Git, and handoff status.",
            argument_hint: "",
        },
        CommandDefinition {
            name: "vibehub-sync",
            zh: "将外部工作区变更与 VibeHub 状态对齐。",
            en: "Reconcile external workspace changes with VibeHub state.",
            argument_hint: "[scope]",
        },
        CommandDefinition {
            name: "vibehub-diff",
            zh: "汇总当前 Git diff 和任务范围漂移。",
            en: "Summarize current Git diff and task-scope drift.",
            argument_hint: "[focus]",
        },
        CommandDefinition {
            name: "vibehub-start",
            zh: "根据用户请求草拟 VibeHub 任务。",
            en: "Draft a VibeHub task from the user request.",
            argument_hint: "<request>",
        },
        CommandDefinition {
            name: "vibehub-context",
            zh: "检查或建议当前阶段上下文。",
            en: "Inspect or propose context for the current phase.",
            argument_hint: "[phase]",
        },
        CommandDefinition {
            name: "vibehub-research",
            zh: "执行有证据支撑的研究并产出 research notes。",
            en: "Run evidence-backed research and produce research notes.",
            argument_hint: "<question>",
        },
        CommandDefinition {
            name: "vibehub-plan",
            zh: "创建或修复实现与验证计划。",
            en: "Create or repair implementation and validation plans.",
            argument_hint: "[goal]",
        },
        CommandDefinition {
            name: "vibehub-claim",
            zh: "在 gate 允许时 claim 指定 capability。",
            en: "Claim a named capability when gates allow it.",
            argument_hint: "<capability>",
        },
        CommandDefinition {
            name: "vibehub-continue",
            zh: "继续当前 VibeHub 阶段。",
            en: "Continue the active VibeHub phase.",
            argument_hint: "[instruction]",
        },
        CommandDefinition {
            name: "vibehub-checkpoint",
            zh: "记录进展、命令、风险和下一步。",
            en: "Record progress, commands, risks, and next steps.",
            argument_hint: "[note]",
        },
        CommandDefinition {
            name: "vibehub-review",
            zh: "基于 diff、context 和 research evidence 进行 review。",
            en: "Review using diff, context, and research evidence.",
            argument_hint: "[focus]",
        },
        CommandDefinition {
            name: "vibehub-handoff",
            zh: "生成 session handoff notes。",
            en: "Build session handoff notes.",
            argument_hint: "[note]",
        },
        CommandDefinition {
            name: "vibehub-recover",
            zh: "在漂移、HEAD 变化或中断后生成恢复报告。",
            en: "Build a recovery report after drift, HEAD changes, or interruption.",
            argument_hint: "[symptom]",
        },
        CommandDefinition {
            name: "vibehub-finish",
            zh: "完成当前工作并建议状态流转。",
            en: "Finish current work and propose state transition.",
            argument_hint: "[summary]",
        },
        CommandDefinition {
            name: "vibehub-journal",
            zh: "草拟可沉淀的 session journal notes。",
            en: "Draft durable session journal notes.",
            argument_hint: "[note]",
        },
        CommandDefinition {
            name: "vibehub-knowledge",
            zh: "将重复经验沉淀为规则或知识。",
            en: "Promote repeated learnings into rules or knowledge.",
            argument_hint: "[lesson]",
        },
        CommandDefinition {
            name: "vibehub-debug-dump",
            zh: "导出脱敏调试包。",
            en: "Export a redacted debug bundle.",
            argument_hint: "[options]",
        },
    ]
}

fn decide_target(
    absolute: &Path,
    config: &AgentAdapterConfig,
    target: &RenderedTarget,
) -> Result<SyncDecision> {
    if target.managed_region {
        if absolute.exists() {
            let existing = fs::read_to_string(absolute)
                .with_context(|| format!("Failed to read {}", absolute.display()))?;
            decide_managed_update(&existing, &target.content)
        } else {
            Ok(SyncDecision::Create(target.content.clone()))
        }
    } else if absolute.exists() {
        let existing = fs::read_to_string(absolute)
            .with_context(|| format!("Failed to read {}", absolute.display()))?;
        let existing_hash = hash_content(&existing);
        let desired_hash = hash_content(&target.content);
        if existing_hash == desired_hash {
            Ok(SyncDecision::Skip)
        } else if config
            .generated_hashes
            .get(&target.path)
            .map(|hash| hash == &existing_hash)
            .unwrap_or(false)
        {
            Ok(SyncDecision::Update(target.content.clone()))
        } else {
            Ok(SyncDecision::Conflict(
                "File was modified outside VibeHub; import it as an override or overwrite from VibeHub."
                    .to_string(),
            ))
        }
    } else {
        Ok(SyncDecision::Create(target.content.clone()))
    }
}

fn decide_managed_update(existing: &str, managed_section: &str) -> Result<SyncDecision> {
    if existing.matches(MANAGED_START).count() > 1 || existing.matches(MANAGED_END).count() > 1 {
        return Ok(SyncDecision::Conflict(
            "File contains multiple VibeHub managed marker regions; leaving it unchanged."
                .to_string(),
        ));
    }
    let start = existing.find(MANAGED_START);
    let end = existing.find(MANAGED_END);
    match (start, end) {
        (Some(start), Some(end)) if start < end => {
            let mut after_end = end + MANAGED_END.len();
            if existing[after_end..].starts_with('\n') && managed_section.ends_with('\n') {
                after_end += 1;
            }
            let next = format!(
                "{}{}{}",
                &existing[..start],
                managed_section,
                &existing[after_end..]
            );
            if next == existing {
                Ok(SyncDecision::Skip)
            } else {
                Ok(SyncDecision::Update(next))
            }
        }
        (Some(_), Some(_)) => Ok(SyncDecision::Conflict(
            "Managed VibeHub markers are out of order; leaving it unchanged.".to_string(),
        )),
        (Some(_), None) | (None, Some(_)) => Ok(SyncDecision::Conflict(
            "File contains only one VibeHub managed marker; leaving it unchanged.".to_string(),
        )),
        (None, None) => {
            let mut next = existing.to_string();
            if !next.ends_with('\n') {
                next.push('\n');
            }
            if !next.ends_with("\n\n") {
                next.push('\n');
            }
            next.push_str(managed_section);
            Ok(SyncDecision::Update(next))
        }
    }
}

fn file_status(
    project_root: &Path,
    config: &AgentAdapterConfig,
    target: &RenderedTarget,
) -> Result<AgentAdapterFileStatus> {
    let absolute = project_root.join(&target.path);
    let generated_hash = config.generated_hashes.get(&target.path).cloned();
    let current_hash = if absolute.exists() {
        Some(hash_content(&fs::read_to_string(&absolute).with_context(
            || format!("Failed to read {}", absolute.display()),
        )?))
    } else {
        None
    };
    let desired_hash = hash_content(&target.content);
    let status = if !absolute.exists() {
        "missing"
    } else if current_hash.as_deref() == Some(desired_hash.as_str()) {
        "in_sync"
    } else if generated_hash.as_deref() == current_hash.as_deref() {
        "stale"
    } else {
        "modified_outside_vibehub"
    };

    Ok(AgentAdapterFileStatus {
        tool: target.tool.clone(),
        path: target.path.clone(),
        exists: absolute.exists(),
        status: status.to_string(),
        generated_hash,
        current_hash,
        description: target.description.clone(),
    })
}

fn adapter_warnings(
    config: &AgentAdapterConfig,
    registry: Option<&RegistrySnapshot>,
    files: &[AgentAdapterFileStatus],
) -> Vec<String> {
    let mut warnings = Vec::new();
    if config.template_version != TEMPLATE_VERSION {
        warnings.push(format!(
            "Adapter configuration is outdated: config template {} != current {}. Update recommended.",
            config.template_version, TEMPLATE_VERSION
        ));
    }
    match registry {
        Some(registry) if config.registry_hash.as_deref() != Some(registry.hash.as_str()) => {
            warnings.push(
                "Adapter configuration is outdated: skills registry changed. Update recommended."
                    .to_string(),
            );
        }
        None => warnings.push(format!(
            "Skills registry is missing at {REGISTRY_PATH}; adapter commands use built-in fallbacks."
        )),
        _ => {}
    }
    if files.iter().any(|file| file.status == "stale") {
        warnings.push(
            "One or more adapter files were generated from an older template or registry. Update recommended."
                .to_string(),
        );
    }
    if files.iter().any(|file| file.status == "missing") {
        warnings.push("One or more adapter files are missing. Update recommended.".to_string());
    }
    if files
        .iter()
        .any(|file| file.status == "modified_outside_vibehub")
    {
        warnings.push(
            "One or more adapter files were modified outside VibeHub; review conflicts before updating."
                .to_string(),
        );
    }
    warnings
}

fn read_registry_snapshot(project_root: &Path) -> Result<Option<RegistrySnapshot>> {
    let registry_path = project_root.join(REGISTRY_PATH);
    if !registry_path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&registry_path)
        .with_context(|| format!("Failed to read {}", registry_path.display()))?;
    let parsed: SkillsRegistry = serde_yaml::from_str(&content)
        .with_context(|| format!("Invalid YAML: {}", registry_path.display()))?;
    Ok(Some(RegistrySnapshot {
        schema_version: parsed.schema_version.as_ref().map(yaml_value_to_string),
        hash: hash_content(&content),
        skills: parsed.skills,
    }))
}

fn yaml_value_to_string(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::String(value) => value.clone(),
        serde_yaml::Value::Number(value) => value.to_string(),
        serde_yaml::Value::Bool(value) => value.to_string(),
        _ => serde_yaml::to_string(value)
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

fn read_config_or_default(project_root: &Path) -> Result<AgentAdapterConfig> {
    let config_path = project_root.join(CONFIG_PATH);
    if !config_path.exists() {
        return Ok(default_config(default_tools()));
    }
    let content = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read {}", config_path.display()))?;
    let mut config: AgentAdapterConfig = serde_yaml::from_str(&content)
        .with_context(|| format!("Invalid YAML: {}", config_path.display()))?;
    config.enabled_tools = normalize_tools(config.enabled_tools);
    Ok(config)
}

fn write_config(project_root: &Path, config: &AgentAdapterConfig) -> Result<()> {
    let config_path = project_root.join(CONFIG_PATH);
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    let content = serde_yaml::to_string(config).context("Failed to serialize adapter config")?;
    fs::write(&config_path, content)
        .with_context(|| format!("Failed to write {}", config_path.display()))
}

fn write_target(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    fs::write(path, content).with_context(|| format!("Failed to write {}", path.display()))
}

fn normalize_tools(tools: Vec<AgentTool>) -> Vec<AgentTool> {
    tools
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn hash_content(content: &str) -> String {
    let digest = Sha256::digest(content.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn temp_project() -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("vibehub-agent-adapter-test-{}", Uuid::new_v4()));
        fs::create_dir_all(path.join(".vibehub/adapters")).expect("create temp project");
        path
    }

    #[test]
    fn syncs_all_selected_platform_outputs() {
        let project = temp_project();
        ensure_adapter_config(&project, default_tools()).expect("config");

        let result = sync_agent_adapters(&project, None, false).expect("sync adapter");

        assert!(result.created_files.contains(&"AGENTS.md".to_string()));
        assert!(result.created_files.contains(&"CLAUDE.md".to_string()));
        assert!(result
            .created_files
            .contains(&".vibehub/adapters/generated/amp-code/command-index.md".to_string()));
        assert!(result
            .created_files
            .contains(&".claude/commands/vibehub-sync.md".to_string()));
        assert!(result
            .created_files
            .contains(&".opencode/commands/vibehub-sync.md".to_string()));
        assert!(result
            .created_files
            .contains(&".vibehub/adapters/protocol.md".to_string()));
        assert!(result
            .created_files
            .contains(&".vibehub/adapters/hooks/vibehub-stop-check.mjs".to_string()));
        assert!(result
            .created_files
            .contains(&".codex/vibehub/stop-hook-config.md".to_string()));
        assert!(result
            .created_files
            .contains(&".claude/settings.json".to_string()));
        assert!(result.created_files.contains(&"opencode.json".to_string()));
        assert!(result
            .created_files
            .contains(&".cursor/rules/vibehub.mdc".to_string()));
        assert!(result
            .created_files
            .contains(&".cursor/rules/vibehub-command-index.mdc".to_string()));
        assert!(result
            .created_files
            .contains(&".antigravity/vibehub-instructions.md".to_string()));
        assert!(result
            .created_files
            .contains(&".antigravity/vibehub-command-index.md".to_string()));
        assert!(result
            .created_files
            .contains(&".agents/skills/vibehub-sync/SKILL.md".to_string()));
        assert!(result
            .created_files
            .contains(&".codex/vibehub/constraints.md".to_string()));
        assert!(result
            .created_files
            .contains(&".codex/vibehub/command-index.md".to_string()));
        assert!(result
            .created_files
            .contains(&".claude/vibehub/constraints.md".to_string()));
        assert!(result
            .created_files
            .contains(&".claude/vibehub/command-index.md".to_string()));
        assert!(result
            .created_files
            .contains(&".opencode/vibehub/constraints.md".to_string()));
        assert!(result
            .created_files
            .contains(&".opencode/vibehub/command-index.md".to_string()));
        assert!(result
            .created_files
            .contains(&".vibehub/adapters/generated/codex/vibehub-sync.md".to_string()));
        assert!(
            fs::read_to_string(project.join(".agents/skills/vibehub-sync/SKILL.md"))
                .expect("read")
                .contains("name: vibehub-sync")
        );
        assert!(
            fs::read_to_string(project.join(".claude/vibehub/constraints.md"))
                .expect("read")
                .contains("VibeHub Constraints for Claude Code")
        );
        assert!(
            fs::read_to_string(project.join(".opencode/vibehub/constraints.md"))
                .expect("read")
                .contains("VibeHub Constraints for OpenCode")
        );
        assert!(
            fs::read_to_string(project.join(".vibehub/adapters/protocol.md"))
                .expect("read")
                .contains("Phase Output Contract")
        );
        assert!(
            fs::read_to_string(project.join(".vibehub/adapters/hooks/vibehub-stop-check.mjs"))
                .expect("read")
                .contains("VibeHub active task")
        );
        assert!(fs::read_to_string(project.join(".claude/settings.json"))
            .expect("read")
            .contains("vibehub-stop-check.mjs"));
        assert!(fs::read_to_string(project.join("opencode.json"))
            .expect("read")
            .contains(".vibehub/adapters/protocol.md"));
        assert!(
            fs::read_to_string(project.join(".cursor/rules/vibehub.mdc"))
                .expect("read")
                .contains("VibeHub Cursor Rule")
        );
        assert!(
            fs::read_to_string(project.join(".antigravity/vibehub-instructions.md"))
                .expect("read")
                .contains("VibeHub Antigravity Instructions")
        );
        assert!(fs::read_to_string(project.join(".claude/commands/vibehub-sync.md"))
            .expect("read")
            .contains("将外部工作区变更与 VibeHub 状态对齐。 / Reconcile external workspace changes with VibeHub state."));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn preserves_unmanaged_content_in_agents_md() {
        let project = temp_project();
        fs::write(project.join("AGENTS.md"), "User rules\n").expect("write");
        ensure_adapter_config(&project, vec![AgentTool::Codex]).expect("config");

        sync_agent_adapters(&project, None, false).expect("sync");
        let content = fs::read_to_string(project.join("AGENTS.md")).expect("read");

        assert!(content.starts_with("User rules\n\n"));
        assert!(content.contains(MANAGED_START));
        assert!(content.contains(".vibehub/agent-view/current.md"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn detects_modified_generated_command_as_conflict() {
        let project = temp_project();
        ensure_adapter_config(&project, vec![AgentTool::Opencode]).expect("config");
        sync_agent_adapters(&project, None, false).expect("sync");
        fs::write(
            project.join(".opencode/commands/vibehub-sync.md"),
            "manual edit\n",
        )
        .expect("manual edit");

        let result = sync_agent_adapters(&project, None, false).expect("sync");

        assert_eq!(result.conflict_files.len(), 1);
        assert_eq!(
            result.conflict_files[0].path,
            ".opencode/commands/vibehub-sync.md"
        );

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn override_changes_command_body() {
        let project = temp_project();
        let mut overrides = BTreeMap::new();
        overrides.insert("vibehub-status".to_string(), "custom body".to_string());
        update_agent_adapter_config(
            &project,
            AgentAdapterConfigPatch {
                enabled_tools: Some(vec![AgentTool::ClaudeCode]),
                command_overrides: Some(overrides),
            },
        )
        .expect("patch");

        sync_agent_adapters(&project, None, false).expect("sync");
        let content =
            fs::read_to_string(project.join(".claude/commands/vibehub-status.md")).expect("read");

        assert!(content.contains("custom body"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn dry_run_does_not_write_files() {
        let project = temp_project();
        ensure_adapter_config(&project, vec![AgentTool::Codex]).expect("config");

        let result = sync_agent_adapters(&project, None, true).expect("dry run");

        assert!(result.dry_run);
        assert!(result.created_files.contains(&"AGENTS.md".to_string()));
        assert!(!project.join("AGENTS.md").exists());

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn projects_skills_registry_commands() {
        let project = temp_project();
        fs::write(
            project.join(".vibehub/skills.registry.yaml"),
            r#"schema_version: "1.0"
skills:
  - name: vibehub-example
    args:
      - { name: project_root, type: string, required: true }
      - { name: dry_run, type: boolean, required: false, default: false }
    returns: skill_response_schema_v1
    side_effects: [writes_output]
    callable_by: [main-agent]
    idempotent: true
    description: "Example registry-backed skill."
"#,
        )
        .expect("write registry");
        ensure_adapter_config(&project, vec![AgentTool::Codex]).expect("config");

        let result = sync_agent_adapters(&project, None, false).expect("sync");

        assert!(result
            .created_files
            .contains(&".agents/skills/vibehub-example/SKILL.md".to_string()));
        let skill = fs::read_to_string(project.join(".agents/skills/vibehub-example/SKILL.md"))
            .expect("read skill");
        assert!(skill.contains("name: vibehub-example"));
        assert!(skill.contains("Example registry-backed skill."));
        assert!(skill.contains("project_root"));
        let config = read_config_or_default(&project).expect("config");
        assert_eq!(config.registry_schema_version.as_deref(), Some("1.0"));
        assert!(config.registry_hash.is_some());

        fs::remove_dir_all(project).expect("cleanup");
    }
}

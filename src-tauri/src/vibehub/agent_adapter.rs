use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const CONFIG_PATH: &str = ".vibehub/adapters/config.yaml";
const TEMPLATE_VERSION: &str = "2.0.0-pre.2";
const MANAGED_START: &str = "<!-- VIBEHUB:AGENT-INTEGRATION:START -->";
const MANAGED_END: &str = "<!-- VIBEHUB:AGENT-INTEGRATION:END -->";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AgentTool {
    Codex,
    ClaudeCode,
    Opencode,
}

impl AgentTool {
    fn label(self) -> &'static str {
        match self {
            AgentTool::Codex => "Codex",
            AgentTool::ClaudeCode => "Claude Code",
            AgentTool::Opencode => "OpenCode",
        }
    }

    fn id(self) -> &'static str {
        match self {
            AgentTool::Codex => "codex",
            AgentTool::ClaudeCode => "claude_code",
            AgentTool::Opencode => "opencode",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentAdapterConfig {
    pub schema_version: u32,
    pub template_version: String,
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

pub fn default_tools() -> Vec<AgentTool> {
    vec![AgentTool::Codex, AgentTool::ClaudeCode, AgentTool::Opencode]
}

pub fn default_config(enabled_tools: Vec<AgentTool>) -> AgentAdapterConfig {
    AgentAdapterConfig {
        schema_version: 1,
        template_version: TEMPLATE_VERSION.to_string(),
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
    let targets = rendered_targets(&config);
    let files = targets
        .iter()
        .map(|target| file_status(&project_root, &config, target))
        .collect::<Result<Vec<_>>>()?;

    Ok(AgentAdapterStatus {
        project_root: normalize_path(&project_root),
        config_path: CONFIG_PATH.to_string(),
        enabled_tools: config.enabled_tools.clone(),
        commands: command_specs(&config),
        files,
        warnings: Vec::new(),
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

    let targets = rendered_targets(&config);
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
        config.generated_hashes = next_hashes;
        write_config(&project_root, &config)?;
    }

    let files = rendered_targets(&config)
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

fn command_specs(config: &AgentAdapterConfig) -> Vec<AgentCommandSpec> {
    command_definitions_v2()
        .into_iter()
        .map(|definition| {
            let body = config
                .command_overrides
                .get(definition.name)
                .cloned()
                .unwrap_or_else(|| {
                    default_command_body(definition.name, definition.zh, definition.en)
                });
            AgentCommandSpec {
                name: definition.name.to_string(),
                description_zh: definition.zh.to_string(),
                description_en: definition.en.to_string(),
                argument_hint: definition.argument_hint.to_string(),
                body,
            }
        })
        .collect()
}

fn rendered_targets(config: &AgentAdapterConfig) -> Vec<RenderedTarget> {
    let tools: BTreeSet<AgentTool> = config.enabled_tools.iter().copied().collect();
    let mut targets = Vec::new();

    if tools.contains(&AgentTool::Codex) || tools.contains(&AgentTool::Opencode) {
        targets.push(RenderedTarget {
            tool: "shared".to_string(),
            path: "AGENTS.md".to_string(),
            content: build_static_protocol(
                "AGENTS.md",
                &tools
                    .iter()
                    .filter(|tool| matches!(tool, AgentTool::Codex | AgentTool::Opencode))
                    .copied()
                    .collect::<Vec<_>>(),
            ),
            description: "Shared Codex/OpenCode project instructions".to_string(),
            managed_region: true,
        });
    }
    if tools.contains(&AgentTool::ClaudeCode) {
        targets.push(RenderedTarget {
            tool: AgentTool::ClaudeCode.id().to_string(),
            path: "CLAUDE.md".to_string(),
            content: build_static_protocol("CLAUDE.md", &[AgentTool::ClaudeCode]),
            description: "Claude Code project memory".to_string(),
            managed_region: true,
        });
    }

    for command in command_specs(config) {
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
            content: build_platform_constraints(AgentTool::Codex),
            description: "Codex-readable VibeHub constraints and file protocol".to_string(),
            managed_region: false,
        });
        targets.push(RenderedTarget {
            tool: AgentTool::Codex.id().to_string(),
            path: ".codex/vibehub/command-index.md".to_string(),
            content: build_command_index(AgentTool::Codex, config),
            description: "Codex VibeHub command index".to_string(),
            managed_region: false,
        });
    }
    if tools.contains(&AgentTool::ClaudeCode) {
        targets.push(RenderedTarget {
            tool: AgentTool::ClaudeCode.id().to_string(),
            path: ".claude/vibehub/constraints.md".to_string(),
            content: build_platform_constraints(AgentTool::ClaudeCode),
            description: "Claude Code-readable VibeHub constraints and file protocol".to_string(),
            managed_region: false,
        });
        targets.push(RenderedTarget {
            tool: AgentTool::ClaudeCode.id().to_string(),
            path: ".claude/vibehub/command-index.md".to_string(),
            content: build_command_index(AgentTool::ClaudeCode, config),
            description: "Claude Code VibeHub command index".to_string(),
            managed_region: false,
        });
    }
    if tools.contains(&AgentTool::Opencode) {
        targets.push(RenderedTarget {
            tool: AgentTool::Opencode.id().to_string(),
            path: ".opencode/vibehub/constraints.md".to_string(),
            content: build_platform_constraints(AgentTool::Opencode),
            description: "OpenCode-readable VibeHub constraints and file protocol".to_string(),
            managed_region: false,
        });
        targets.push(RenderedTarget {
            tool: AgentTool::Opencode.id().to_string(),
            path: ".opencode/vibehub/command-index.md".to_string(),
            content: build_command_index(AgentTool::Opencode, config),
            description: "OpenCode VibeHub command index".to_string(),
            managed_region: false,
        });
    }

    targets
}

fn build_static_protocol(file_label: &str, tools: &[AgentTool]) -> String {
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

VibeHub owns project state. Agent output is reported state only.

## Read Before Work

1. `.vibehub/agent-view/current.md`
2. `.vibehub/agent-view/current-context.md`
3. `.vibehub/agent-view/handoff.md`
4. `.vibehub/rules/hard-rules.md`

## Rules

- Treat `.vibehub/agent-view/current.md` as the dynamic entry point.
- Do not edit `.vibehub/state.yaml` or canonical task/run pointers directly.
- Report changed files, files read, commands run, tests run or reason not run, risks, and handoff notes.
- Use evidence labels: `hard_observed`, `agent_reported`, `inferred`, `user_confirmed`.
- If workspace state changed outside VibeHub, run the `vibehub-sync` instruction and return a sync report instead of silently advancing state.

## Command Namespace

Use generated `vibehub-*` commands where supported. They describe VibeHub-specific work without replacing built-in agent commands.
{MANAGED_END}
"#
    )
}

fn render_codex_skill(command: &AgentCommandSpec) -> String {
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
        body = command.body
    )
}

fn build_platform_constraints(tool: AgentTool) -> String {
    let label = tool.label();
    let command_location = match tool {
        AgentTool::Codex => ".agents/skills/vibehub-*/SKILL.md repo skills",
        AgentTool::ClaudeCode => ".claude/commands/vibehub-*.md slash commands",
        AgentTool::Opencode => ".opencode/commands/vibehub-*.md commands",
    };
    let static_entry = match tool {
        AgentTool::ClaudeCode => "CLAUDE.md",
        AgentTool::Codex | AgentTool::Opencode => "AGENTS.md",
    };
    format!(
        r#"# VibeHub Constraints for {label}

Read these files before VibeHub-scoped work:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`

{label} may inspect and change project files within the active task scope.
{label} must not edit `.vibehub/state.yaml`, current task/run pointer files, or mark canonical state complete.

Use `{command_location}` for VibeHub operations.
Use `{static_entry}` as the automatic static protocol entry.

If work happened outside VibeHub, use `vibehub-sync` and return a sync report.
If state or Git HEAD drifted, run `vibehub-recover` and return a recover report.
"#
    )
}

fn build_command_index(tool: AgentTool, config: &AgentAdapterConfig) -> String {
    let label = tool.label();
    let command_root = match tool {
        AgentTool::Codex => ".agents/skills/",
        AgentTool::ClaudeCode => ".claude/commands/",
        AgentTool::Opencode => ".opencode/commands/",
    };
    let mut output =
        format!("# VibeHub Command Index for {label}\n\nCommand root: `{command_root}`\n\n");
    for command in command_specs(config) {
        output.push_str(&format!(
            "- `{}`: {} / {}\n",
            command.name, command.description_zh, command.description_en
        ));
    }
    output
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

fn yaml_double_quoted(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn default_command_body(name: &str, zh: &str, en: &str) -> String {
    let extra = match name {
        "vibehub-help" => "List available VibeHub commands and explain when to use each one.",
        "vibehub-init" => "Check whether `.vibehub/` exists. If it is missing, ask the user to initialize from the VibeHub app; do not create canonical state yourself.",
        "vibehub-status" => "Summarize current task, run, phase, context pack, handoff, Git status, and visible warnings.",
        "vibehub-sync" => "Inspect Git diff/status and VibeHub pointers. Produce a sync report with observed drift, suspected cause, and recommended VibeHub action.",
        "vibehub-diff" => "Summarize changed files and scope drift against the current task and context pack.",
        "vibehub-start" => "Convert the user's request into a VibeHub task draft with goal, acceptance criteria, mode suggestion, and context candidates.",
        "vibehub-context" => "Inspect current context quality and propose missing files or stale context rebuilds.",
        "vibehub-research" => "Run evidence-backed research, cite sources when external facts are used, and write research notes for VibeHub review.",
        "vibehub-plan" => "Create or repair the implementation plan, validation plan, risk list, and context plan.",
        "vibehub-continue" => "Continue the active phase using current.md and the context pack. Keep changes scoped to the active task.",
        "vibehub-checkpoint" => "Capture progress, decisions, commands, tests, changed files, risks, and next steps without marking state complete.",
        "vibehub-review" => "Review the current diff against context, plan, research, tests, and VibeHub hard rules.",
        "vibehub-handoff" => "Create handoff notes that let the next session resume without chat history.",
        "vibehub-recover" => "Analyze interrupted or drifted work and produce a recover report with safe next actions.",
        "vibehub-finish" => "Summarize completion evidence and recommend whether VibeHub should advance state, request review, or recover.",
        "vibehub-journal" => "Draft durable session notes suitable for VibeHub journal promotion.",
        "vibehub-knowledge" => "Promote repeated lessons into reusable rules, preferences, or knowledge notes.",
        _ => "Follow the VibeHub agent protocol.",
    };
    format!(
        r#"中文: {zh}
English: {en}

Read first:
- `.vibehub/agent-view/current.md`
- `.vibehub/agent-view/current-context.md`
- `.vibehub/agent-view/handoff.md`
- `.vibehub/rules/hard-rules.md`

Task:
{extra}

Output requirements:
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

fn canonical_project_root(project_root: &Path) -> Result<PathBuf> {
    let project_root = fs::canonicalize(project_root)
        .with_context(|| format!("Project path does not exist: {}", project_root.display()))?;
    if !project_root.is_dir() {
        return Err(anyhow!(
            "Project path is not a directory: {}",
            project_root.display()
        ));
    }
    Ok(project_root)
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

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
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
            .contains(&".claude/commands/vibehub-sync.md".to_string()));
        assert!(result
            .created_files
            .contains(&".opencode/commands/vibehub-sync.md".to_string()));
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
}

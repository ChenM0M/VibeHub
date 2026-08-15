use super::{AgentSpecTarget, V3Error, V3ErrorCategory};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_DISCOVERY_DEPTH: usize = 4;
const DISCOVERY_IGNORES: &[&str] = &[
    ".git",
    ".vibehub",
    "node_modules",
    "target",
    "dist",
    "build",
    ".cache",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectScopeInspection {
    pub control_root: String,
    pub execution_root: String,
    pub git_root: Option<String>,
    pub host_config_root: String,
    pub source: ProjectScopeSource,
    pub nested_repository: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectScopeSource {
    ControlRoot,
    DetectedGitRoot,
    SessionWorkingDirectory,
    Ambiguous,
}

#[derive(Debug, Clone)]
pub struct ResolvedProjectScopes {
    pub control_root: PathBuf,
    pub execution_root: PathBuf,
    pub git_root: Option<PathBuf>,
    pub host_config_root: PathBuf,
    pub source: ProjectScopeSource,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostConfigStatus {
    Missing,
    InSync,
    Mismatched,
    Invalid,
    Unsupported,
    Ambiguous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostConfigScope {
    User,
    Project,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpHostConfigInspection {
    pub schema_version: String,
    pub consumer: AgentSpecTarget,
    pub scope: HostConfigScope,
    pub path: String,
    pub status: HostConfigStatus,
    pub reason: String,
    pub server_name: Option<String>,
    pub binary: Option<String>,
    pub configured_project_root: Option<String>,
    pub canonical_configured_project_root: Option<String>,
    pub revision: Option<String>,
    pub owned_fields: Vec<String>,
    pub provenance: String,
    pub repair_action: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveAgentDeclaration {
    pub path: String,
    pub consumers: Vec<AgentSpecTarget>,
    pub precedence: u32,
    pub exists: bool,
    pub contains_v3_region: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RootAlignmentStatus {
    Aligned,
    DriftDetected,
}

/// A single reason the anchored control root disagrees with the environment the
/// agent actually develops in. `expected` is the active control root and
/// `actual` is the diverging path, so consumers can report both sides.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RootAlignmentFinding {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consumer: Option<AgentSpecTarget>,
}

/// Assessment of whether the control root that anchors workflow facts still
/// matches the execution root and the roots declared by MCP host configs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RootAlignmentReport {
    pub status: RootAlignmentStatus,
    pub control_root: String,
    pub execution_root: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_root: Option<String>,
    pub restart_required: bool,
    pub findings: Vec<RootAlignmentFinding>,
}

impl ResolvedProjectScopes {
    pub fn inspection(&self) -> ProjectScopeInspection {
        ProjectScopeInspection {
            control_root: display(&self.control_root),
            execution_root: display(&self.execution_root),
            git_root: self.git_root.as_ref().map(|path| display(path)),
            host_config_root: display(&self.host_config_root),
            source: self.source,
            nested_repository: self
                .git_root
                .as_ref()
                .is_some_and(|path| path != &self.control_root),
            warnings: self.warnings.clone(),
        }
    }
}

pub fn resolve_project_scopes(
    control_root: impl AsRef<Path>,
    preferred_working_directory: Option<&Path>,
) -> Result<ResolvedProjectScopes, V3Error> {
    let control_root = control_root
        .as_ref()
        .canonicalize()
        .map_err(|error| scope_error("V3_SCOPE_CONTROL_ROOT_NOT_FOUND", error.to_string()))?;
    if let Some(preferred) = preferred_working_directory {
        if let Ok(preferred) = preferred.canonicalize() {
            if preferred.is_dir() && preferred.starts_with(&control_root) {
                let git_root = nearest_git_ancestor(&preferred, &control_root);
                let host_config_root = git_root.clone().unwrap_or_else(|| preferred.clone());
                return Ok(ResolvedProjectScopes {
                    control_root,
                    execution_root: preferred,
                    git_root,
                    host_config_root,
                    source: ProjectScopeSource::SessionWorkingDirectory,
                    warnings: Vec::new(),
                });
            }
        }
    }

    if is_git_root(&control_root) {
        return Ok(ResolvedProjectScopes {
            execution_root: control_root.clone(),
            git_root: Some(control_root.clone()),
            host_config_root: control_root.clone(),
            control_root,
            source: ProjectScopeSource::ControlRoot,
            warnings: Vec::new(),
        });
    }

    let candidates = nested_git_roots(&control_root)?;
    match candidates.as_slice() {
        [git_root] => Ok(ResolvedProjectScopes {
            control_root,
            execution_root: git_root.clone(),
            git_root: Some(git_root.clone()),
            host_config_root: git_root.clone(),
            source: ProjectScopeSource::DetectedGitRoot,
            warnings: vec!["V3_NESTED_GIT_ROOT_DETECTED".to_owned()],
        }),
        [] => Ok(ResolvedProjectScopes {
            execution_root: control_root.clone(),
            git_root: None,
            host_config_root: control_root.clone(),
            control_root,
            source: ProjectScopeSource::ControlRoot,
            warnings: Vec::new(),
        }),
        _ => Ok(ResolvedProjectScopes {
            execution_root: control_root.clone(),
            git_root: None,
            host_config_root: control_root.clone(),
            control_root,
            source: ProjectScopeSource::Ambiguous,
            warnings: vec![format!(
                "V3_NESTED_GIT_ROOT_AMBIGUOUS:{}",
                candidates
                    .iter()
                    .map(|path| display(path))
                    .collect::<Vec<_>>()
                    .join(",")
            )],
        }),
    }
}

pub fn effective_agent_declarations(
    scopes: &ResolvedProjectScopes,
    targets: &[AgentSpecTarget],
) -> Vec<EffectiveAgentDeclaration> {
    let mut roots = vec![scopes.control_root.clone()];
    if scopes.execution_root != scopes.control_root {
        let relative = scopes
            .execution_root
            .strip_prefix(&scopes.control_root)
            .unwrap_or(Path::new(""));
        let mut cursor = scopes.control_root.clone();
        for component in relative.components() {
            cursor.push(component);
            roots.push(cursor.clone());
        }
    }
    let mut declarations = Vec::new();
    for (precedence, root) in roots.into_iter().enumerate() {
        let shared_consumers = targets
            .iter()
            .copied()
            .filter(|target| matches!(target, AgentSpecTarget::Codex | AgentSpecTarget::Opencode))
            .collect::<Vec<_>>();
        if !shared_consumers.is_empty() {
            declarations.push(declaration(
                &scopes.control_root,
                root.join("AGENTS.md"),
                shared_consumers,
                precedence,
            ));
        }
        if targets.contains(&AgentSpecTarget::ClaudeCode) {
            declarations.push(declaration(
                &scopes.control_root,
                root.join("CLAUDE.md"),
                vec![AgentSpecTarget::ClaudeCode],
                precedence,
            ));
        }
    }
    declarations
}

pub fn inspect_mcp_host_configs(
    scopes: &ResolvedProjectScopes,
    targets: &[AgentSpecTarget],
) -> Vec<McpHostConfigInspection> {
    super::inspect_host_mcp_configs(scopes, targets, None)
        .into_iter()
        .filter(|item| item.scope == HostConfigScope::Project)
        .collect()
}

/// Compares the control root that anchors workflow facts against the execution
/// root and the roots declared in MCP host configurations. A long-lived MCP
/// process freezes its control root at startup, so this lets it surface a stale
/// or mismatched project root instead of silently serving the wrong `.vibehub`
/// store while the agent develops somewhere else.
pub fn assess_root_alignment(
    scopes: &ResolvedProjectScopes,
    host_configs: &[McpHostConfigInspection],
) -> RootAlignmentReport {
    let control_root = display(&scopes.control_root);
    let mut findings = Vec::new();

    if scopes.execution_root != scopes.control_root {
        findings.push(RootAlignmentFinding {
            code: "V3_CONTROL_EXECUTION_ROOT_SPLIT".to_owned(),
            message: "workflow facts are anchored to a control root that differs from the execution root where code is developed".to_owned(),
            expected: Some(control_root.clone()),
            actual: Some(display(&scopes.execution_root)),
            consumer: None,
        });
    }

    for host in host_configs {
        if host.status == HostConfigStatus::Mismatched {
            findings.push(RootAlignmentFinding {
                code: "V3_HOST_CONFIG_ROOT_MISMATCH".to_owned(),
                message: "MCP launch arguments target a different project root than the active control root; align the configured path and restart the MCP host".to_owned(),
                expected: Some(control_root.clone()),
                actual: host.configured_project_root.clone(),
                consumer: Some(host.consumer),
            });
        }
    }

    for warning in &scopes.warnings {
        findings.push(RootAlignmentFinding {
            code: "V3_SCOPE_WARNING".to_owned(),
            message: warning.clone(),
            expected: None,
            actual: None,
            consumer: None,
        });
    }

    let restart_required = findings
        .iter()
        .any(|finding| finding.code == "V3_HOST_CONFIG_ROOT_MISMATCH");
    let status = if findings.is_empty() {
        RootAlignmentStatus::Aligned
    } else {
        RootAlignmentStatus::DriftDetected
    };

    RootAlignmentReport {
        status,
        control_root,
        execution_root: display(&scopes.execution_root),
        git_root: scopes.git_root.as_ref().map(|path| display(path)),
        restart_required,
        findings,
    }
}

fn inspect_codex(scopes: &ResolvedProjectScopes) -> McpHostConfigInspection {
    let project_path = scopes.host_config_root.join(".codex/config.toml");
    let project_text = read_config(&project_path);
    let global_path = home_directory().map(|home| home.join(".codex/config.toml"));
    let global_text = global_path.as_ref().and_then(|path| read_config(path));
    let (path, text) = if let Some(text) = project_text {
        (project_path, text)
    } else if let (Some(path), Some(text)) = (global_path, global_text) {
        (path, text)
    } else {
        return missing_host(AgentSpecTarget::Codex, &scopes.control_root, &project_path);
    };
    let value = match toml::from_str::<toml::Value>(&text) {
        Ok(value) => value,
        Err(error) => {
            return invalid_host(
                AgentSpecTarget::Codex,
                &scopes.control_root,
                &path,
                error.to_string(),
            )
        }
    };
    let servers = value.get("mcp_servers").and_then(toml::Value::as_table);
    let Some((name, server)) = find_toml_server(servers) else {
        return mismatched_host(
            AgentSpecTarget::Codex,
            &scopes.control_root,
            &path,
            None,
            None,
            "VibeHub MCP entry is missing",
        );
    };
    let args = server
        .get("args")
        .and_then(toml::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let args = args
        .iter()
        .filter_map(toml::Value::as_str)
        .collect::<Vec<_>>();
    host_from_command(
        AgentSpecTarget::Codex,
        &scopes.control_root,
        &path,
        Some(name.to_owned()),
        &args,
    )
}

fn inspect_json_host(
    scopes: &ResolvedProjectScopes,
    consumer: AgentSpecTarget,
    relative_path: &str,
    table_path: &[&str],
) -> McpHostConfigInspection {
    let path = scopes.host_config_root.join(relative_path);
    let Some(text) = read_config(&path) else {
        return missing_host(consumer, &scopes.control_root, &path);
    };
    let value = match serde_json::from_str::<Value>(&text) {
        Ok(value) => value,
        Err(error) => {
            return invalid_host(consumer, &scopes.control_root, &path, error.to_string())
        }
    };
    let table = table_path
        .iter()
        .fold(Some(&value), |value, key| value?.get(*key));
    let Some((name, server)) = find_json_server(table) else {
        return mismatched_host(
            consumer,
            &scopes.control_root,
            &path,
            None,
            None,
            "VibeHub MCP entry is missing",
        );
    };
    let command = server.get("command");
    let args = if let Some(command) = command.and_then(Value::as_array) {
        command.iter().filter_map(Value::as_str).collect::<Vec<_>>()
    } else {
        server
            .get("args")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
    };
    host_from_command(
        consumer,
        &scopes.control_root,
        &path,
        Some(name.to_owned()),
        &args,
    )
}

fn host_from_command(
    consumer: AgentSpecTarget,
    control_root: &Path,
    path: &Path,
    server_name: Option<String>,
    args: &[&str],
) -> McpHostConfigInspection {
    let configured_root = args
        .windows(2)
        .find(|window| window[0] == "mcp-stdio")
        .map(|window| window[1].to_owned());
    let expected = display(control_root);
    if configured_root.as_deref() == Some(expected.as_str()) {
        McpHostConfigInspection {
            schema_version: "1.0".to_owned(),
            consumer,
            scope: HostConfigScope::Project,
            path: relative_or_native(control_root, path),
            status: HostConfigStatus::InSync,
            reason: "MCP launch arguments target this V3 control root".to_owned(),
            server_name,
            binary: None,
            configured_project_root: configured_root,
            canonical_configured_project_root: None,
            revision: None,
            owned_fields: vec![
                "server_name".to_owned(),
                "command".to_owned(),
                "args".to_owned(),
            ],
            provenance: "host_config_file".to_owned(),
            repair_action: None,
        }
    } else {
        mismatched_host(
            consumer,
            control_root,
            path,
            server_name,
            configured_root,
            "MCP launch arguments do not target this V3 control root",
        )
    }
}

fn find_toml_server(
    table: Option<&toml::map::Map<String, toml::Value>>,
) -> Option<(&str, &toml::Value)> {
    table?
        .iter()
        .find_map(|(name, value)| is_vibehub_name(name).then_some((name.as_str(), value)))
}

fn find_json_server(value: Option<&Value>) -> Option<(&str, &Value)> {
    value?
        .as_object()?
        .iter()
        .find_map(|(name, value)| is_vibehub_name(name).then_some((name.as_str(), value)))
}

fn is_vibehub_name(name: &str) -> bool {
    matches!(name, "vibehub" | "vibehub-v3")
}

fn declaration(
    control_root: &Path,
    path: PathBuf,
    consumers: Vec<AgentSpecTarget>,
    precedence: usize,
) -> EffectiveAgentDeclaration {
    let content = fs::read_to_string(&path).ok();
    EffectiveAgentDeclaration {
        path: relative_or_native(control_root, &path),
        consumers,
        precedence: precedence as u32,
        exists: content.is_some(),
        contains_v3_region: content
            .as_deref()
            .is_some_and(|text| text.contains("<!-- VIBEHUB:AGENT-SPEC:START -->")),
    }
}

fn read_config(path: &Path) -> Option<String> {
    let metadata = fs::symlink_metadata(path).ok()?;
    (!metadata.file_type().is_symlink() && metadata.is_file() && metadata.len() <= 1024 * 1024)
        .then(|| fs::read_to_string(path).ok())
        .flatten()
}

fn home_directory() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn missing_host(consumer: AgentSpecTarget, root: &Path, path: &Path) -> McpHostConfigInspection {
    McpHostConfigInspection {
        schema_version: "1.0".to_owned(),
        consumer,
        scope: HostConfigScope::Project,
        path: relative_or_native(root, path),
        status: HostConfigStatus::Missing,
        reason: "project-scoped MCP configuration file is missing".to_owned(),
        server_name: None,
        binary: None,
        configured_project_root: None,
        canonical_configured_project_root: None,
        revision: None,
        owned_fields: Vec::new(),
        provenance: "host_config_file".to_owned(),
        repair_action: Some("run the typed V3 host MCP sync for this trusted project".to_owned()),
    }
}

fn invalid_host(
    consumer: AgentSpecTarget,
    root: &Path,
    path: &Path,
    reason: String,
) -> McpHostConfigInspection {
    McpHostConfigInspection {
        schema_version: "1.0".to_owned(),
        consumer,
        scope: HostConfigScope::Project,
        path: relative_or_native(root, path),
        status: HostConfigStatus::Invalid,
        reason,
        server_name: None,
        binary: None,
        configured_project_root: None,
        canonical_configured_project_root: None,
        revision: None,
        owned_fields: Vec::new(),
        provenance: "host_config_file".to_owned(),
        repair_action: Some(
            "inspect the host configuration and repair it without guessing ownership".to_owned(),
        ),
    }
}

fn mismatched_host(
    consumer: AgentSpecTarget,
    root: &Path,
    path: &Path,
    server_name: Option<String>,
    configured_project_root: Option<String>,
    reason: &str,
) -> McpHostConfigInspection {
    McpHostConfigInspection {
        schema_version: "1.0".to_owned(),
        consumer,
        scope: HostConfigScope::Project,
        path: relative_or_native(root, path),
        status: HostConfigStatus::Mismatched,
        reason: reason.to_owned(),
        server_name,
        binary: None,
        configured_project_root,
        canonical_configured_project_root: None,
        revision: None,
        owned_fields: Vec::new(),
        provenance: "host_config_file".to_owned(),
        repair_action: Some("run the typed V3 host MCP sync for this trusted project".to_owned()),
    }
}

fn nested_git_roots(control_root: &Path) -> Result<Vec<PathBuf>, V3Error> {
    let mut queue = VecDeque::from([(control_root.to_path_buf(), 0usize)]);
    let mut roots = Vec::new();
    while let Some((directory, depth)) = queue.pop_front() {
        if depth > 0 && is_git_root(&directory) {
            roots.push(directory);
            continue;
        }
        if depth >= MAX_DISCOVERY_DEPTH {
            continue;
        }
        let entries = fs::read_dir(&directory)
            .map_err(|error| scope_error("V3_SCOPE_DISCOVERY_FAILED", error.to_string()))?;
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if DISCOVERY_IGNORES.contains(&name.as_ref()) {
                continue;
            }
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() && !file_type.is_symlink() {
                queue.push_back((entry.path(), depth + 1));
            }
        }
    }
    roots.sort();
    Ok(roots)
}

fn nearest_git_ancestor(path: &Path, boundary: &Path) -> Option<PathBuf> {
    let mut cursor = path.to_path_buf();
    loop {
        if is_git_root(&cursor) {
            return Some(cursor);
        }
        if cursor == boundary || !cursor.pop() {
            return None;
        }
    }
}

fn is_git_root(path: &Path) -> bool {
    fs::symlink_metadata(path.join(".git"))
        .map(|metadata| metadata.is_dir() || metadata.is_file())
        .unwrap_or(false)
}

fn relative_or_native(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .ok()
        .filter(|relative| !relative.as_os_str().is_empty())
        .map(display)
        .unwrap_or_else(|| display(path))
}

fn display(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn scope_error(code: &str, message: String) -> V3Error {
    V3Error::new(code, V3ErrorCategory::Internal, false, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn root() -> PathBuf {
        let root = std::env::temp_dir().join(format!("vibehub-v3-scopes-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn detects_single_nested_git_root_and_effective_declarations() {
        let root = root();
        let nested = root.join("GDG2026");
        fs::create_dir_all(nested.join(".git")).unwrap();
        fs::write(root.join("AGENTS.md"), "outer").unwrap();
        fs::write(nested.join("AGENTS.md"), "inner").unwrap();
        let scopes = resolve_project_scopes(&root, None).unwrap();
        assert_eq!(scopes.execution_root, nested.canonicalize().unwrap());
        assert_eq!(scopes.source, ProjectScopeSource::DetectedGitRoot);
        let declarations = effective_agent_declarations(&scopes, &[AgentSpecTarget::Codex]);
        assert_eq!(declarations.len(), 2);
        assert_eq!(declarations[0].path, "AGENTS.md");
        assert_eq!(declarations[1].path, "GDG2026/AGENTS.md");
        assert!(declarations[1].precedence > declarations[0].precedence);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn multiple_nested_git_roots_fail_closed_to_control_root() {
        let root = root();
        fs::create_dir_all(root.join("one/.git")).unwrap();
        fs::create_dir_all(root.join("two/.git")).unwrap();
        let scopes = resolve_project_scopes(&root, None).unwrap();
        assert_eq!(scopes.execution_root, root.canonicalize().unwrap());
        assert_eq!(scopes.source, ProjectScopeSource::Ambiguous);
        assert!(scopes.git_root.is_none());
        assert!(scopes.warnings[0].starts_with("V3_NESTED_GIT_ROOT_AMBIGUOUS:"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn host_config_reports_project_identity_mismatch() {
        let root = root();
        fs::create_dir_all(root.join(".git")).unwrap();
        let binary = if cfg!(windows) {
            r"C:\bin\vibehub.exe"
        } else {
            "/bin/vibehub"
        };
        fs::write(
            root.join("opencode.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "mcp": {"vibehub": {"type": "local", "command": [binary, "mcp-stdio", "/wrong/project"]}}
            }))
            .unwrap(),
        )
        .unwrap();
        let scopes = resolve_project_scopes(&root, None).unwrap();
        let inspection = inspect_mcp_host_configs(&scopes, &[AgentSpecTarget::Opencode]);
        assert_eq!(inspection[0].status, HostConfigStatus::Mismatched);
        assert_eq!(
            inspection[0].configured_project_root.as_deref(),
            Some("/wrong/project")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn root_alignment_is_aligned_when_control_root_matches_host_config() {
        let root = root();
        fs::create_dir_all(root.join(".git")).unwrap();
        let canonical = root.canonicalize().unwrap();
        let binary = if cfg!(windows) {
            r"C:\bin\vibehub.exe"
        } else {
            "/bin/vibehub"
        };
        fs::write(
            root.join("opencode.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "mcp": {"vibehub": {"type": "local", "command": [binary, "mcp-stdio", canonical.to_string_lossy()]}}
            }))
            .unwrap(),
        )
        .unwrap();
        let scopes = resolve_project_scopes(&root, None).unwrap();
        let hosts = inspect_mcp_host_configs(&scopes, &[AgentSpecTarget::Opencode]);
        let report = assess_root_alignment(&scopes, &hosts);
        assert_eq!(report.status, RootAlignmentStatus::Aligned);
        assert!(!report.restart_required);
        assert!(report.findings.is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn root_alignment_flags_host_config_mismatch_and_requires_restart() {
        let root = root();
        fs::create_dir_all(root.join(".git")).unwrap();
        let binary = if cfg!(windows) {
            r"C:\bin\vibehub.exe"
        } else {
            "/bin/vibehub"
        };
        fs::write(
            root.join("opencode.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "mcp": {"vibehub": {"type": "local", "command": [binary, "mcp-stdio", "/wrong/project"]}}
            }))
            .unwrap(),
        )
        .unwrap();
        let scopes = resolve_project_scopes(&root, None).unwrap();
        let hosts = inspect_mcp_host_configs(&scopes, &[AgentSpecTarget::Opencode]);
        let report = assess_root_alignment(&scopes, &hosts);
        assert_eq!(report.status, RootAlignmentStatus::DriftDetected);
        assert!(report.restart_required);
        let finding = report
            .findings
            .iter()
            .find(|finding| finding.code == "V3_HOST_CONFIG_ROOT_MISMATCH")
            .expect("host config mismatch finding");
        assert_eq!(finding.actual.as_deref(), Some("/wrong/project"));
        assert_eq!(finding.consumer, Some(AgentSpecTarget::Opencode));
        assert_eq!(
            finding.expected.as_deref(),
            Some(display(&scopes.control_root).as_str())
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn root_alignment_flags_control_execution_split_for_nested_git_root() {
        let root = root();
        let nested = root.join("app");
        fs::create_dir_all(nested.join(".git")).unwrap();
        let scopes = resolve_project_scopes(&root, None).unwrap();
        assert_eq!(scopes.source, ProjectScopeSource::DetectedGitRoot);
        let report = assess_root_alignment(&scopes, &[]);
        assert_eq!(report.status, RootAlignmentStatus::DriftDetected);
        assert!(!report.restart_required);
        let finding = report
            .findings
            .iter()
            .find(|finding| finding.code == "V3_CONTROL_EXECUTION_ROOT_SPLIT")
            .expect("control/execution split finding");
        assert_eq!(
            finding.expected.as_deref(),
            Some(display(&scopes.control_root).as_str())
        );
        assert_eq!(
            finding.actual.as_deref(),
            Some(display(&scopes.execution_root).as_str())
        );
        fs::remove_dir_all(root).unwrap();
    }
}

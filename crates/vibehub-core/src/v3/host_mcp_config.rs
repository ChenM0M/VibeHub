//! Versioned, project-scoped MCP host configuration support.
//!
//! This module deliberately treats host configuration as an owned, typed
//! boundary.  It never selects a project from the current working directory
//! or from a display name: the V3 control root is supplied by the caller and
//! is embedded in every generated launch command.

use super::{
    inspect_project_layout, resolve_project_scopes, AgentSpecTarget, HostConfigScope,
    HostConfigStatus, McpHostConfigInspection, ProjectLayoutState, ResolvedProjectScopes, V3Error,
    V3ErrorCategory,
};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const HOST_MCP_SCHEMA_VERSION: &str = "1.0";
const MAX_CONFIG_BYTES: u64 = 1024 * 1024;
const CODEX_SERVER_NAMES: &[&str] = &["vibehub", "vibehub-v3"];

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HostMcpSyncResult {
    pub status: HostMcpSyncStatus,
    pub inspections: Vec<McpHostConfigInspection>,
    pub written_paths: Vec<String>,
    pub skipped_paths: Vec<String>,
    pub blocking_paths: Vec<String>,
    pub global_migration: GlobalMcpMigrationResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostMcpSyncStatus {
    Synchronized,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GlobalMcpMigrationResult {
    pub status: GlobalMcpMigrationStatus,
    pub path: Option<String>,
    pub backup_path: Option<String>,
    pub before_revision: Option<String>,
    pub after_revision: Option<String>,
    pub removed_server_names: Vec<String>,
    pub preserved_bytes: bool,
    pub repair_action: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GlobalMcpMigrationStatus {
    Missing,
    Unchanged,
    Migrated,
}

pub fn empty_host_mcp_sync_result() -> HostMcpSyncResult {
    HostMcpSyncResult {
        status: HostMcpSyncStatus::Synchronized,
        inspections: Vec::new(),
        written_paths: Vec::new(),
        skipped_paths: Vec::new(),
        blocking_paths: Vec::new(),
        global_migration: unchanged_global_result(None),
    }
}

#[derive(Debug, Clone)]
struct ConfigSnapshot {
    revision: Option<String>,
    bytes: Option<Vec<u8>>,
    permissions: Option<fs::Permissions>,
}

#[derive(Debug, Clone)]
struct PreparedConfigWrite {
    path: PathBuf,
    replacement: Vec<u8>,
    snapshot: ConfigSnapshot,
    backup: bool,
}

#[derive(Debug, Clone, Copy)]
enum ConfigFormat {
    CodexToml,
    ClaudeJson,
    OpencodeJson,
}

/// Inspect user and project configuration independently.  The optional home
/// override is used by deterministic tests and by fixture probes; production
/// callers should pass `None`.
pub fn inspect_host_mcp_configs(
    scopes: &ResolvedProjectScopes,
    targets: &[AgentSpecTarget],
    home_override: Option<&Path>,
) -> Vec<McpHostConfigInspection> {
    inspect_host_mcp_configs_inner(scopes, targets, home_override, true)
}

fn inspect_host_mcp_configs_inner(
    scopes: &ResolvedProjectScopes,
    targets: &[AgentSpecTarget],
    home_override: Option<&Path>,
    use_process_home: bool,
) -> Vec<McpHostConfigInspection> {
    let home = home_override
        .map(Path::to_path_buf)
        .or_else(|| use_process_home.then(home_directory).flatten());
    let mut result = Vec::new();
    for target in targets {
        let project_path = match target {
            AgentSpecTarget::Codex => scopes.host_config_root.join(".codex/config.toml"),
            AgentSpecTarget::ClaudeCode => scopes.host_config_root.join(".mcp.json"),
            AgentSpecTarget::Opencode => scopes.host_config_root.join("opencode.json"),
        };
        result.push(inspect_path(
            scopes,
            *target,
            HostConfigScope::Project,
            project_path,
            format_for(*target),
        ));

        let Some(home) = home.as_ref() else {
            result.push(missing_inspection(
                scopes,
                *target,
                HostConfigScope::User,
                user_path(Path::new("/"), *target),
                "user home could not be resolved; user configuration is unknown",
            ));
            continue;
        };
        for path in user_paths(home, *target) {
            result.push(inspect_path(
                scopes,
                *target,
                HostConfigScope::User,
                path,
                format_for(*target),
            ));
        }
    }
    result
}

/// Public convenience wrapper used by typed status callers.
pub fn inspect_host_mcp_configs_for_project(
    project_root: impl AsRef<Path>,
) -> Result<Vec<McpHostConfigInspection>, V3Error> {
    let root = trusted_project_root(project_root.as_ref())?;
    let scopes = resolve_project_scopes(&root, None)?;
    Ok(inspect_host_mcp_configs(
        &scopes,
        &[
            AgentSpecTarget::Codex,
            AgentSpecTarget::ClaudeCode,
            AgentSpecTarget::Opencode,
        ],
        None,
    ))
}

/// Synchronize project-level MCP entries and remove an explicitly owned,
/// project-bound VibeHub server from the Codex user configuration.  Each file
/// is written with a hash precondition, an atomic replacement, and a rollback
/// attempt if a later file fails.
pub fn sync_host_mcp_configs(
    project_root: impl AsRef<Path>,
    binary_override: Option<&Path>,
) -> Result<HostMcpSyncResult, V3Error> {
    sync_host_mcp_configs_with_options(project_root.as_ref(), binary_override, true)
}

pub fn sync_project_host_mcp_configs(
    project_root: impl AsRef<Path>,
    binary_override: Option<&Path>,
) -> Result<HostMcpSyncResult, V3Error> {
    sync_host_mcp_configs_with_options(project_root.as_ref(), binary_override, false)
}

pub fn sync_host_mcp_configs_with_options(
    project_root: &Path,
    binary_override: Option<&Path>,
    migrate_global: bool,
) -> Result<HostMcpSyncResult, V3Error> {
    let root = trusted_project_root(project_root)?;
    let scopes = resolve_project_scopes(&root, None)?;
    let binary = resolve_binary(&root, binary_override)?;
    let targets = [
        AgentSpecTarget::Codex,
        AgentSpecTarget::ClaudeCode,
        AgentSpecTarget::Opencode,
    ];
    let home = migrate_global.then(home_directory).flatten();
    let inspections =
        inspect_host_mcp_configs_inner(&scopes, &targets, home.as_deref(), migrate_global);
    let project_inspections = inspections
        .iter()
        .filter(|item| item.scope == HostConfigScope::Project)
        .collect::<Vec<_>>();

    let mut writes = Vec::new();
    let mut skipped_paths = Vec::new();
    let mut blocking_paths = Vec::new();
    for inspection in project_inspections {
        let path = resolve_inspection_path(&scopes, inspection)?;
        let format = format_for(inspection.consumer);
        let existing = read_snapshot(&path)?;
        let replacement = match prepare_project_config(
            &path,
            format,
            existing.bytes.as_deref(),
            &binary,
            &scopes.control_root,
            inspection,
        ) {
            Ok(replacement) => replacement,
            Err(error) => {
                blocking_paths.push(inspection.path.clone());
                if matches!(
                    inspection.status,
                    HostConfigStatus::Unsupported
                        | HostConfigStatus::Invalid
                        | HostConfigStatus::Ambiguous
                ) {
                    return Ok(incomplete_result(
                        inspections,
                        skipped_paths,
                        blocking_paths,
                        unchanged_global_result(home.as_deref()),
                    ));
                }
                return Err(error);
            }
        };
        if existing.bytes.as_deref() == Some(replacement.as_slice()) {
            skipped_paths.push(inspection.path.clone());
        } else {
            writes.push(PreparedConfigWrite {
                path,
                replacement,
                snapshot: existing,
                backup: true,
            });
        }
    }

    let mut applied = Vec::new();
    for write in writes {
        if let Err(error) = apply_config_write(&write) {
            rollback_config_writes(&applied);
            return Err(error);
        }
        applied.push(write);
    }

    let global = if let Some(home) = home.as_deref() {
        match migrate_global_codex_config(home, None) {
            Ok(global) => global,
            Err(error) => {
                rollback_config_writes(&applied);
                return Err(error);
            }
        }
    } else {
        unchanged_global_result(None)
    };

    let refreshed =
        inspect_host_mcp_configs_inner(&scopes, &targets, home.as_deref(), migrate_global);
    let mut final_blocking = blocking_paths;
    final_blocking.extend(
        refreshed
            .iter()
            .filter(|item| {
                item.scope == HostConfigScope::Project && item.status != HostConfigStatus::InSync
            })
            .map(|item| item.path.clone()),
    );
    final_blocking.sort();
    final_blocking.dedup();
    Ok(HostMcpSyncResult {
        status: if final_blocking.is_empty() {
            HostMcpSyncStatus::Synchronized
        } else {
            HostMcpSyncStatus::Incomplete
        },
        inspections: refreshed,
        written_paths: applied
            .iter()
            .map(|write| display_path(&scopes.control_root, &write.path))
            .collect(),
        skipped_paths,
        blocking_paths: final_blocking,
        global_migration: global,
    })
}

/// Remove only a VibeHub-owned project-bound server from the Codex user file.
/// The operation is intentionally byte-preserving outside the owned TOML
/// section, which keeps comments and unknown settings intact.
pub fn migrate_global_codex_config(
    home: impl AsRef<Path>,
    expected_revision: Option<&str>,
) -> Result<GlobalMcpMigrationResult, V3Error> {
    let path = home.as_ref().join(".codex/config.toml");
    let snapshot = read_snapshot(&path)?;
    let Some(bytes) = snapshot.bytes.as_deref() else {
        return Ok(GlobalMcpMigrationResult {
            status: GlobalMcpMigrationStatus::Missing,
            path: Some(path.to_string_lossy().into_owned()),
            backup_path: None,
            before_revision: None,
            after_revision: None,
            removed_server_names: Vec::new(),
            preserved_bytes: true,
            repair_action: Some("create project-level MCP configuration during V3 sync".to_owned()),
        });
    };
    if let Some(expected) = expected_revision {
        if snapshot.revision.as_deref() != Some(expected) {
            return Err(stale_revision(
                &path,
                expected,
                snapshot.revision.as_deref(),
            ));
        }
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|error| invalid_config("V3_HOST_MCP_CONFIG_INVALID_UTF8", error.to_string()))?;
    let Some(rewrite) = remove_owned_codex_servers(text)? else {
        return Ok(GlobalMcpMigrationResult {
            status: GlobalMcpMigrationStatus::Unchanged,
            path: Some(path.to_string_lossy().into_owned()),
            backup_path: None,
            before_revision: snapshot.revision.clone(),
            after_revision: snapshot.revision.clone(),
            removed_server_names: Vec::new(),
            preserved_bytes: true,
            repair_action: None,
        });
    };
    let replacement = rewrite.bytes;
    let backup_path = backup_path(&path);
    let write = PreparedConfigWrite {
        path: path.clone(),
        replacement: replacement.clone(),
        snapshot: snapshot.clone(),
        backup: true,
    };
    apply_config_write(&write)?;
    Ok(GlobalMcpMigrationResult {
        status: GlobalMcpMigrationStatus::Migrated,
        path: Some(path.to_string_lossy().into_owned()),
        backup_path: Some(backup_path.to_string_lossy().into_owned()),
        before_revision: snapshot.revision,
        after_revision: Some(hash(&replacement)),
        removed_server_names: rewrite.removed_names,
        preserved_bytes: rewrite.preserved_bytes,
        repair_action: Some(
            "restart the host so it reloads project-scoped MCP configuration".to_owned(),
        ),
    })
}

fn incomplete_result(
    inspections: Vec<McpHostConfigInspection>,
    skipped_paths: Vec<String>,
    blocking_paths: Vec<String>,
    global_migration: GlobalMcpMigrationResult,
) -> HostMcpSyncResult {
    HostMcpSyncResult {
        status: HostMcpSyncStatus::Incomplete,
        inspections,
        written_paths: Vec::new(),
        skipped_paths,
        blocking_paths,
        global_migration,
    }
}

fn unchanged_global_result(home: Option<&Path>) -> GlobalMcpMigrationResult {
    let path = home.map(|home| home.join(".codex/config.toml"));
    GlobalMcpMigrationResult {
        status: if path.as_ref().is_some_and(|path| path.exists()) {
            GlobalMcpMigrationStatus::Unchanged
        } else {
            GlobalMcpMigrationStatus::Missing
        },
        path: path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        backup_path: None,
        before_revision: None,
        after_revision: None,
        removed_server_names: Vec::new(),
        preserved_bytes: true,
        repair_action: None,
    }
}

fn trusted_project_root(path: &Path) -> Result<PathBuf, V3Error> {
    let root = path
        .canonicalize()
        .map_err(|error| io_error("V3_HOST_MCP_ROOT_NOT_FOUND", error))?;
    let layout = inspect_project_layout(&root)?;
    if layout.state != ProjectLayoutState::V3 {
        return Err(validation(
            "V3_HOST_MCP_PROJECT_UNTRUSTED",
            format!(
                "host MCP sync requires a V3 project, found {:?}",
                layout.state
            ),
        ));
    }
    let metadata = fs::symlink_metadata(&root.join(".vibehub"))
        .map_err(|error| io_error("V3_HOST_MCP_ROOT_INVALID", error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(validation(
            "V3_HOST_MCP_PROJECT_UNTRUSTED",
            ".vibehub must be a regular directory",
        ));
    }
    Ok(root)
}

/// Locate a stable, installed `vibehub` on `PATH` so written MCP transport
/// configs point at a portable launch command (acceptance c01/c02) instead of
/// a repo-local `target/debug` build that only exists inside one checkout and
/// vanishes after `cargo clean`, on another machine, or under a different
/// clone path. Returns an absolute path, satisfying the writer's absolute-path
/// invariant. A checkout dev who must test the current source build sets
/// `VIBEHUB_MCP_BINARY` instead, which is resolved before this lookup.
fn find_vibehub_on_path() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join("vibehub");
        if candidate.is_file() {
            // Return the PATH entry as-is (no canonicalize) so the written
            // command matches what `mcp status` displays and what Codex already
            // records (`/opt/homebrew/bin/vibehub`), keeping the three harnesses
            // consistent. The writer's absolute-path check rejects any relative
            // PATH entry.
            return Some(candidate);
        }
    }
    None
}

fn resolve_binary(root: &Path, override_path: Option<&Path>) -> Result<PathBuf, V3Error> {
    let candidate = override_path
        .map(Path::to_path_buf)
        .or_else(|| std::env::var_os("VIBEHUB_MCP_BINARY").map(PathBuf::from))
        // Prefer a stable, installed `vibehub` on PATH. Acceptance criteria
        // c01/c02 require the written transport config to point at a portable
        // launch command, not a repo-local `target/debug` absolute path that
        // breaks without a build, on another machine, or under a different
        // clone path. `VIBEHUB_MCP_BINARY` above keeps the checkout dev able to
        // force the current-source build when they need to test it.
        .or_else(find_vibehub_on_path)
        .or_else(|| {
            [
                root.join("target/debug/vibehub"),
                root.join("target/debug/vibehub.exe"),
                root.join("target/release/vibehub"),
                root.join("target/release/vibehub.exe"),
            ]
            .into_iter()
            .find(|path| path.is_file())
        })
        // A temporary fixture is not itself a VibeHub checkout, so it cannot
        // contain the project-local binary. During a running CLI this resolves
        // to that CLI; during unit tests it is the absolute test executable.
        // Both are explicit absolute launch targets.
        .or_else(|| std::env::current_exe().ok())
        .ok_or_else(|| validation("V3_HOST_MCP_BINARY_UNKNOWN", "MCP binary path is unknown"))?;
    if !candidate.is_absolute() {
        return Err(validation(
            "V3_HOST_MCP_BINARY_NOT_ABSOLUTE",
            "MCP binary path must be absolute",
        ));
    }
    Ok(candidate)
}

fn format_for(target: AgentSpecTarget) -> ConfigFormat {
    match target {
        AgentSpecTarget::Codex => ConfigFormat::CodexToml,
        AgentSpecTarget::ClaudeCode => ConfigFormat::ClaudeJson,
        AgentSpecTarget::Opencode => ConfigFormat::OpencodeJson,
    }
}

fn user_paths(home: &Path, target: AgentSpecTarget) -> Vec<PathBuf> {
    match target {
        AgentSpecTarget::Codex => vec![home.join(".codex/config.toml")],
        AgentSpecTarget::ClaudeCode => vec![home.join(".claude.json")],
        AgentSpecTarget::Opencode => vec![
            home.join(".config/opencode/opencode.jsonc"),
            home.join(".config/opencode/opencode.json"),
        ],
    }
}

fn user_path(home: &Path, target: AgentSpecTarget) -> PathBuf {
    user_paths(home, target)
        .into_iter()
        .next()
        .unwrap_or_else(|| home.join(".config/unknown"))
}

fn inspect_path(
    scopes: &ResolvedProjectScopes,
    consumer: AgentSpecTarget,
    scope: HostConfigScope,
    path: PathBuf,
    format: ConfigFormat,
) -> McpHostConfigInspection {
    let display_path = if scope == HostConfigScope::Project {
        display_path(&scopes.control_root, &path)
    } else {
        path.to_string_lossy().into_owned()
    };
    let snapshot = match read_snapshot(&path) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            return base_inspection(
                consumer,
                scope,
                display_path,
                HostConfigStatus::Invalid,
                error.to_string(),
                None,
                None,
                None,
                None,
            )
        }
    };
    let Some(bytes) = snapshot.bytes.as_deref() else {
        return missing_inspection(
            scopes,
            consumer,
            scope,
            path,
            if scope == HostConfigScope::User {
                "user-scoped configuration is absent; this is safe and project binding must remain absent"
            } else {
                "project-scoped MCP configuration is missing"
            },
        );
    };
    let parsed = match parse_server(bytes, format) {
        Ok(parsed) => parsed,
        Err(error) => {
            return base_inspection(
                consumer,
                scope,
                display_path,
                HostConfigStatus::Invalid,
                error.to_string(),
                None,
                None,
                None,
                snapshot.revision,
            )
        }
    };
    let Some(server) = parsed else {
        return base_inspection(
            consumer,
            scope,
            display_path,
            HostConfigStatus::Missing,
            if scope == HostConfigScope::User {
                "no VibeHub-owned server is present in the user configuration".to_owned()
            } else {
                "VibeHub MCP entry is missing".to_owned()
            },
            None,
            None,
            snapshot.revision,
            None,
        );
    };
    let expected = scopes.control_root.to_string_lossy().into_owned();
    let canonical = canonical_configured_root(&server.project_root);
    let root_matches =
        canonical.as_deref() == Some(expected.as_str()) || server.project_root == expected;
    let binary_is_absolute = Path::new(&server.binary).is_absolute();
    let status = if !binary_is_absolute {
        HostConfigStatus::Unsupported
    } else if scope == HostConfigScope::User {
        HostConfigStatus::Mismatched
    } else if root_matches {
        HostConfigStatus::InSync
    } else {
        HostConfigStatus::Mismatched
    };
    let reason = match status {
        HostConfigStatus::InSync => {
            "project MCP launch arguments target this V3 control root".to_owned()
        }
        HostConfigStatus::Mismatched if scope == HostConfigScope::User => {
            "user-scoped VibeHub MCP binds a project and must be migrated to project scope"
                .to_owned()
        }
        HostConfigStatus::Mismatched => {
            "project MCP launch arguments do not target this V3 control root".to_owned()
        }
        HostConfigStatus::Unsupported => "MCP binary must be an explicit absolute path".to_owned(),
        _ => "MCP host configuration is not usable".to_owned(),
    };
    base_inspection(
        consumer,
        scope,
        display_path,
        status,
        reason,
        Some(server.name),
        Some(server.binary),
        Some(server.project_root),
        snapshot.revision,
    )
    .with_canonical_root(canonical)
}

fn missing_inspection(
    scopes: &ResolvedProjectScopes,
    consumer: AgentSpecTarget,
    scope: HostConfigScope,
    path: PathBuf,
    reason: &str,
) -> McpHostConfigInspection {
    base_inspection(
        consumer,
        scope,
        if scope == HostConfigScope::Project {
            display_path(&scopes.control_root, &path)
        } else {
            path.to_string_lossy().into_owned()
        },
        HostConfigStatus::Missing,
        reason.to_owned(),
        None,
        None,
        None,
        None,
    )
}

fn base_inspection(
    consumer: AgentSpecTarget,
    scope: HostConfigScope,
    path: String,
    status: HostConfigStatus,
    reason: String,
    server_name: Option<String>,
    binary: Option<String>,
    project_root: Option<String>,
    revision: Option<String>,
) -> McpHostConfigInspection {
    McpHostConfigInspection {
        schema_version: HOST_MCP_SCHEMA_VERSION.to_owned(),
        consumer,
        scope,
        path,
        status,
        reason,
        server_name,
        binary,
        configured_project_root: project_root.clone(),
        canonical_configured_project_root: project_root
            .as_deref()
            .and_then(canonical_configured_root),
        revision,
        owned_fields: vec![
            "server_name".to_owned(),
            "command".to_owned(),
            "args".to_owned(),
        ],
        provenance: "host_config_file".to_owned(),
        repair_action: repair_action(scope, status),
    }
}

impl McpHostConfigInspection {
    fn with_canonical_root(mut self, root: Option<String>) -> Self {
        self.canonical_configured_project_root = root;
        self
    }
}

fn repair_action(scope: HostConfigScope, status: HostConfigStatus) -> Option<String> {
    match (scope, status) {
        (HostConfigScope::User, HostConfigStatus::Mismatched) => Some(
            "remove only the VibeHub-owned project-bound Codex block, then restart the host"
                .to_owned(),
        ),
        (HostConfigScope::Project, HostConfigStatus::Missing)
        | (HostConfigScope::Project, HostConfigStatus::Mismatched) => Some(
            "run the typed V3 agent-specs-sync/host-MCP sync for this trusted project".to_owned(),
        ),
        (
            _,
            HostConfigStatus::Invalid | HostConfigStatus::Unsupported | HostConfigStatus::Ambiguous,
        ) => Some(
            "inspect the configuration format and repair it without guessing ownership".to_owned(),
        ),
        _ => None,
    }
}

#[derive(Debug, Clone)]
struct ParsedServer {
    name: String,
    binary: String,
    project_root: String,
}

fn parse_server(bytes: &[u8], format: ConfigFormat) -> Result<Option<ParsedServer>, V3Error> {
    match format {
        ConfigFormat::CodexToml => parse_codex_toml(bytes),
        ConfigFormat::ClaudeJson | ConfigFormat::OpencodeJson => parse_json(bytes, format),
    }
}

fn parse_codex_toml(bytes: &[u8]) -> Result<Option<ParsedServer>, V3Error> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| invalid_config("V3_HOST_MCP_CONFIG_INVALID_UTF8", error.to_string()))?;
    let value: toml::Value = toml::from_str(text)
        .map_err(|error| invalid_config("V3_HOST_MCP_CONFIG_INVALID_TOML", error.to_string()))?;
    let Some(servers) = value.get("mcp_servers").and_then(toml::Value::as_table) else {
        return Ok(None);
    };
    let mut matches = Vec::new();
    for name in CODEX_SERVER_NAMES {
        if let Some(server) = servers.get(*name) {
            matches.push((*name, server));
        }
    }
    if matches.len() > 1 {
        return Err(invalid_config(
            "V3_HOST_MCP_CONFIG_AMBIGUOUS",
            "multiple VibeHub server names are present in the same Codex config",
        ));
    }
    let Some((name, server)) = matches.into_iter().next() else {
        return Ok(None);
    };
    let command = server
        .get("command")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| {
            invalid_config(
                "V3_HOST_MCP_SERVER_UNSUPPORTED",
                "Codex VibeHub command is missing",
            )
        })?;
    let args = server
        .get("args")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| {
            invalid_config(
                "V3_HOST_MCP_SERVER_UNSUPPORTED",
                "Codex VibeHub args are missing",
            )
        })?;
    let args = args
        .iter()
        .map(|value| {
            value.as_str().ok_or_else(|| {
                invalid_config(
                    "V3_HOST_MCP_SERVER_UNSUPPORTED",
                    "Codex VibeHub args must be strings",
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    parse_command(name, command, &args)
}

fn parse_json(bytes: &[u8], format: ConfigFormat) -> Result<Option<ParsedServer>, V3Error> {
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|error| invalid_config("V3_HOST_MCP_CONFIG_INVALID_JSON", error.to_string()))?;
    let table_path = match format {
        ConfigFormat::ClaudeJson => &["mcpServers"][..],
        ConfigFormat::OpencodeJson => &["mcp"][..],
        ConfigFormat::CodexToml => &[][..],
    };
    let table = table_path
        .iter()
        .fold(Some(&value), |value, key| value?.get(*key))
        .and_then(serde_json::Value::as_object);
    let Some(table) = table else {
        return Ok(None);
    };
    let mut matches = table
        .iter()
        .filter(|(name, _)| CODEX_SERVER_NAMES.contains(&name.as_str()))
        .collect::<Vec<_>>();
    if matches.len() > 1 {
        return Err(invalid_config(
            "V3_HOST_MCP_CONFIG_AMBIGUOUS",
            "multiple VibeHub server names are present in the same host config",
        ));
    }
    let Some((name, server)) = matches.pop() else {
        return Ok(None);
    };
    let object = server.as_object().ok_or_else(|| {
        invalid_config(
            "V3_HOST_MCP_SERVER_UNSUPPORTED",
            "VibeHub server must be an object",
        )
    })?;
    let command = object.get("command").ok_or_else(|| {
        invalid_config(
            "V3_HOST_MCP_SERVER_UNSUPPORTED",
            "VibeHub command is missing",
        )
    })?;
    let (binary, args) = if let Some(command) = command.as_array() {
        let mut values = command
            .iter()
            .map(|value| {
                value.as_str().ok_or_else(|| {
                    invalid_config(
                        "V3_HOST_MCP_SERVER_UNSUPPORTED",
                        "VibeHub command values must be strings",
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let binary = values.remove(0);
        (binary.to_owned(), values)
    } else {
        let binary = command.as_str().ok_or_else(|| {
            invalid_config(
                "V3_HOST_MCP_SERVER_UNSUPPORTED",
                "VibeHub command must be a string or array",
            )
        })?;
        let args = object
            .get("args")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| {
                invalid_config("V3_HOST_MCP_SERVER_UNSUPPORTED", "VibeHub args are missing")
            })?
            .iter()
            .map(|value| {
                value.as_str().ok_or_else(|| {
                    invalid_config(
                        "V3_HOST_MCP_SERVER_UNSUPPORTED",
                        "VibeHub args must be strings",
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        (binary.to_owned(), args)
    };
    parse_command(name, &binary, &args)
}

fn parse_command(name: &str, binary: &str, args: &[&str]) -> Result<Option<ParsedServer>, V3Error> {
    let project_root = args
        .windows(2)
        .find(|window| window[0] == "mcp-stdio")
        .map(|window| window[1].to_owned())
        .ok_or_else(|| {
            invalid_config(
                "V3_HOST_MCP_SERVER_UNSUPPORTED",
                "VibeHub MCP args must contain mcp-stdio and a project root",
            )
        })?;
    Ok(Some(ParsedServer {
        name: name.to_owned(),
        binary: binary.to_owned(),
        project_root,
    }))
}

fn prepare_project_config(
    path: &Path,
    format: ConfigFormat,
    bytes: Option<&[u8]>,
    binary: &Path,
    control_root: &Path,
    inspection: &McpHostConfigInspection,
) -> Result<Vec<u8>, V3Error> {
    if matches!(
        inspection.status,
        HostConfigStatus::Invalid | HostConfigStatus::Unsupported | HostConfigStatus::Ambiguous
    ) {
        return Err(validation(
            "V3_HOST_MCP_CONFIG_UNSAFE_TO_OVERWRITE",
            format!(
                "refusing to overwrite {}: {}",
                path.display(),
                inspection.reason
            ),
        ));
    }
    let binary = binary.to_string_lossy();
    let root = control_root.to_string_lossy();
    match format {
        ConfigFormat::CodexToml => {
            let existing = bytes
                .map(std::str::from_utf8)
                .transpose()
                .map_err(|error| {
                    invalid_config("V3_HOST_MCP_CONFIG_INVALID_UTF8", error.to_string())
                })?
                .unwrap_or("");
            rewrite_codex_project(existing, &binary, &root)
        }
        ConfigFormat::ClaudeJson => {
            rewrite_json_project(bytes, "mcpServers", &binary, &root, false)
        }
        ConfigFormat::OpencodeJson => rewrite_json_project(bytes, "mcp", &binary, &root, true),
    }
}

fn rewrite_json_project(
    bytes: Option<&[u8]>,
    table_name: &str,
    binary: &str,
    root: &str,
    opencode: bool,
) -> Result<Vec<u8>, V3Error> {
    let mut value = match bytes {
        Some(bytes) => serde_json::from_slice::<serde_json::Value>(bytes).map_err(|error| {
            invalid_config("V3_HOST_MCP_CONFIG_INVALID_JSON", error.to_string())
        })?,
        None => serde_json::json!({}),
    };
    let object = value.as_object_mut().ok_or_else(|| {
        invalid_config(
            "V3_HOST_MCP_CONFIG_UNSUPPORTED",
            "host config root must be a JSON object",
        )
    })?;
    let table = object
        .entry(table_name.to_owned())
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or_else(|| {
            invalid_config(
                "V3_HOST_MCP_CONFIG_UNSUPPORTED",
                "host MCP table must be a JSON object",
            )
        })?;
    let mut server = serde_json::Map::new();
    if opencode {
        server.insert("type".to_owned(), serde_json::json!("local"));
        server.insert(
            "command".to_owned(),
            serde_json::json!([binary, "mcp-stdio", root]),
        );
        server.insert("enabled".to_owned(), serde_json::json!(true));
    } else {
        server.insert("command".to_owned(), serde_json::json!(binary));
        server.insert("args".to_owned(), serde_json::json!(["mcp-stdio", root]));
    }
    table.insert("vibehub".to_owned(), serde_json::Value::Object(server));
    serde_json::to_vec_pretty(&value)
        .map(|mut bytes| {
            bytes.push(b'\n');
            bytes
        })
        .map_err(|error| invalid_config("V3_HOST_MCP_CONFIG_SERIALIZE_FAILED", error.to_string()))
}

fn rewrite_codex_project(existing: &str, binary: &str, root: &str) -> Result<Vec<u8>, V3Error> {
    let block = format!(
        "[mcp_servers.vibehub]\ncommand = {}\nargs = [\"mcp-stdio\", {}]\n",
        toml_string(binary),
        toml_string(root)
    );
    let mut text = existing.to_owned();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    if let Some(range) = find_codex_server_range(&text, "vibehub")? {
        text.replace_range(range, &block);
    } else if let Some(range) = find_codex_server_range(&text, "vibehub-v3")? {
        text.replace_range(range, &block);
    } else {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&block);
    }
    Ok(text.into_bytes())
}

struct RewriteResult {
    bytes: Vec<u8>,
    removed_names: Vec<String>,
    preserved_bytes: bool,
}

fn remove_owned_codex_servers(text: &str) -> Result<Option<RewriteResult>, V3Error> {
    let sections = toml_sections(text)?;
    let mut remove_ranges = Vec::new();
    let mut removed_names = Vec::new();
    for (index, section) in sections.iter().enumerate() {
        let is_server_group = section.path.len() >= 2
            && section.path[0] == "mcp_servers"
            && CODEX_SERVER_NAMES.contains(&section.path[1].as_str());
        if !is_server_group {
            continue;
        }
        let name = &section.path[1];
        let group_end = sections
            .iter()
            .skip(index + 1)
            .find(|candidate| {
                !(candidate.path.len() > 2
                    && candidate.path[0] == "mcp_servers"
                    && candidate.path[1] == *name)
            })
            .map(|candidate| candidate.start)
            .unwrap_or(text.len());
        let group_block = &text[section.start..group_end];

        // A complete root section is owned only when it has the explicit V3
        // launcher shape. If a previous run left only nested tool-permission
        // sections behind, the reserved VibeHub server name is sufficient to
        // identify that orphaned owned subtree and remove it as well.
        let owned = section.path.len() == 2
            && group_block.contains("mcp-stdio")
            && group_block.contains("args")
            && group_block.contains("command")
            || section.path.len() > 2
                && !sections.iter().any(|candidate| {
                    candidate.path.len() == 2
                        && candidate.path[0] == "mcp_servers"
                        && candidate.path[1] == *name
                });
        if owned {
            if section.path.len() == 2 {
                remove_ranges.push(section.start..group_end);
            } else if !remove_ranges
                .iter()
                .any(|range| range.start <= section.start)
            {
                remove_ranges.push(section.start..group_end);
            }
            if !removed_names.contains(name) {
                removed_names.push(name.clone());
            }
        }
    }
    if remove_ranges.is_empty() {
        return Ok(None);
    }
    let mut output = String::with_capacity(text.len());
    let mut cursor = 0;
    for range in remove_ranges {
        output.push_str(&text[cursor..range.start]);
        cursor = range.end;
    }
    output.push_str(&text[cursor..]);
    Ok(Some(RewriteResult {
        bytes: output.into_bytes(),
        removed_names,
        preserved_bytes: true,
    }))
}

#[derive(Debug, Clone)]
struct TomlSection {
    start: usize,
    path: Vec<String>,
}

fn toml_sections(text: &str) -> Result<Vec<TomlSection>, V3Error> {
    let mut sections = Vec::new();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let is_array = trimmed.starts_with("[[");
            let inner = if is_array {
                &trimmed[2..trimmed.len() - 2]
            } else {
                &trimmed[1..trimmed.len() - 1]
            };
            if is_array {
                return Err(invalid_config(
                    "V3_HOST_MCP_CONFIG_UNSUPPORTED",
                    "array-of-table MCP sections are not supported for fail-closed migration",
                ));
            }
            let path = inner
                .split('.')
                .map(|part| part.trim().trim_matches('"').trim_matches('\'').to_owned())
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>();
            if path.is_empty() {
                return Err(invalid_config(
                    "V3_HOST_MCP_CONFIG_UNSUPPORTED",
                    "empty TOML section name",
                ));
            }
            sections.push(TomlSection {
                start: offset,
                path,
            });
        }
        offset += line.len();
    }
    Ok(sections)
}

fn find_codex_server_range(
    text: &str,
    name: &str,
) -> Result<Option<std::ops::Range<usize>>, V3Error> {
    let sections = toml_sections(text)?;
    for (index, section) in sections.iter().enumerate() {
        if section.path.as_slice() == ["mcp_servers".to_owned(), name.to_owned()] {
            let end = sections
                .get(index + 1)
                .map(|next| next.start)
                .unwrap_or(text.len());
            return Ok(Some(section.start..end));
        }
    }
    Ok(None)
}

fn resolve_inspection_path(
    scopes: &ResolvedProjectScopes,
    inspection: &McpHostConfigInspection,
) -> Result<PathBuf, V3Error> {
    if inspection.scope != HostConfigScope::Project {
        return Err(validation(
            "V3_HOST_MCP_SCOPE_INVALID",
            "only project-scoped entries can be synchronized",
        ));
    }
    Ok(match inspection.consumer {
        AgentSpecTarget::Codex => scopes.host_config_root.join(".codex/config.toml"),
        AgentSpecTarget::ClaudeCode => scopes.host_config_root.join(".mcp.json"),
        AgentSpecTarget::Opencode => scopes.host_config_root.join("opencode.json"),
    })
}

fn read_snapshot(path: &Path) -> Result<ConfigSnapshot, V3Error> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ConfigSnapshot {
                revision: None,
                bytes: None,
                permissions: None,
            });
        }
        Err(error) => return Err(io_error("V3_HOST_MCP_CONFIG_READ_FAILED", error)),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(validation(
            "V3_HOST_MCP_CONFIG_UNSUPPORTED",
            format!("{} must be a regular file", path.display()),
        ));
    }
    if metadata.len() > MAX_CONFIG_BYTES {
        return Err(validation(
            "V3_HOST_MCP_CONFIG_TOO_LARGE",
            format!("{} exceeds the 1 MiB limit", path.display()),
        ));
    }
    let bytes =
        fs::read(path).map_err(|error| io_error("V3_HOST_MCP_CONFIG_READ_FAILED", error))?;
    Ok(ConfigSnapshot {
        revision: Some(hash(&bytes)),
        bytes: Some(bytes),
        permissions: Some(metadata.permissions()),
    })
}

fn apply_config_write(write: &PreparedConfigWrite) -> Result<(), V3Error> {
    verify_snapshot(&write.path, &write.snapshot)?;
    if write.backup && write.snapshot.bytes.is_some() {
        let backup = backup_path(&write.path);
        if !backup.exists() {
            atomic_create(
                &backup,
                write.snapshot.bytes.as_ref().unwrap(),
                write.snapshot.permissions.as_ref(),
            )?;
        }
    }
    atomic_replace(
        &write.path,
        &write.replacement,
        write.snapshot.permissions.as_ref(),
    )
}

fn rollback_config_writes(writes: &[PreparedConfigWrite]) {
    for write in writes.iter().rev() {
        let current = read_snapshot(&write.path).ok();
        if current
            .as_ref()
            .and_then(|snapshot| snapshot.revision.as_deref())
            == Some(hash(&write.replacement).as_str())
        {
            if let Some(bytes) = write.snapshot.bytes.as_ref() {
                let _ = atomic_replace(&write.path, bytes, write.snapshot.permissions.as_ref());
            } else {
                let _ = fs::remove_file(&write.path);
            }
        }
    }
}

fn verify_snapshot(path: &Path, expected: &ConfigSnapshot) -> Result<(), V3Error> {
    let current = read_snapshot(path)?;
    if current.revision != expected.revision {
        return Err(stale_revision(
            path,
            expected.revision.as_deref().unwrap_or("<absent>"),
            current.revision.as_deref(),
        ));
    }
    Ok(())
}

fn atomic_create(
    path: &Path,
    bytes: &[u8],
    permissions: Option<&fs::Permissions>,
) -> Result<(), V3Error> {
    let parent = path.parent().ok_or_else(|| {
        validation(
            "V3_HOST_MCP_CONFIG_WRITE_FAILED",
            "configuration path has no parent",
        )
    })?;
    ensure_regular_parent(parent)?;
    let temporary = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("config"),
        Uuid::new_v4()
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| io_error("V3_HOST_MCP_CONFIG_WRITE_FAILED", error))?;
        if let Some(permissions) = permissions {
            file.set_permissions(permissions.clone())
                .map_err(|error| io_error("V3_HOST_MCP_CONFIG_WRITE_FAILED", error))?;
        }
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|error| io_error("V3_HOST_MCP_CONFIG_WRITE_FAILED", error))?;
        fs::rename(&temporary, path)
            .map_err(|error| io_error("V3_HOST_MCP_CONFIG_WRITE_FAILED", error))?;
        sync_directory(parent).map_err(|error| io_error("V3_HOST_MCP_CONFIG_SYNC_FAILED", error))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn atomic_replace(
    path: &Path,
    bytes: &[u8],
    permissions: Option<&fs::Permissions>,
) -> Result<(), V3Error> {
    let parent = path.parent().ok_or_else(|| {
        validation(
            "V3_HOST_MCP_CONFIG_WRITE_FAILED",
            "configuration path has no parent",
        )
    })?;
    ensure_regular_parent(parent)?;
    let temporary = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("config"),
        Uuid::new_v4()
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| io_error("V3_HOST_MCP_CONFIG_WRITE_FAILED", error))?;
        if let Some(permissions) = permissions {
            file.set_permissions(permissions.clone())
                .map_err(|error| io_error("V3_HOST_MCP_CONFIG_WRITE_FAILED", error))?;
        }
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|error| io_error("V3_HOST_MCP_CONFIG_WRITE_FAILED", error))?;
        replace_file(&temporary, path)
            .map_err(|error| io_error("V3_HOST_MCP_CONFIG_WRITE_FAILED", error))?;
        sync_directory(parent).map_err(|error| io_error("V3_HOST_MCP_CONFIG_SYNC_FAILED", error))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn ensure_regular_parent(path: &Path) -> Result<(), V3Error> {
    if !path.exists() {
        fs::create_dir_all(path)
            .map_err(|error| io_error("V3_HOST_MCP_CONFIG_WRITE_FAILED", error))?;
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| io_error("V3_HOST_MCP_CONFIG_WRITE_FAILED", error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(validation(
            "V3_HOST_MCP_CONFIG_UNSUPPORTED",
            format!("{} must be a regular directory", path.display()),
        ));
    }
    Ok(())
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> std::io::Result<()> {
    fs::File::open(path)?.sync_all()
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

fn backup_path(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.vibehub-backup", path.to_string_lossy()))
}

fn canonical_configured_root(root: &str) -> Option<String> {
    Path::new(root)
        .canonicalize()
        .ok()
        .map(|path| path.to_string_lossy().into_owned())
}

fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .ok()
        .filter(|relative| !relative.as_os_str().is_empty())
        .map(|relative| relative.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

fn home_directory() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn toml_string(value: &str) -> String {
    toml::Value::String(value.to_owned()).to_string()
}

fn hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{:x}", hasher.finalize())
}

fn stale_revision(path: &Path, expected: &str, actual: Option<&str>) -> V3Error {
    V3Error::new(
        "V3_HOST_MCP_REVISION_CONFLICT",
        V3ErrorCategory::StaleResource,
        true,
        format!(
            "host MCP configuration changed before write: {}",
            path.display()
        ),
    )
    .with_detail("expected_revision", expected.to_owned())
    .with_detail("actual_revision", actual.map(str::to_owned))
}

fn invalid_config(code: &str, message: impl Into<String>) -> V3Error {
    V3Error::new(code, V3ErrorCategory::Validation, false, message)
}

fn validation(code: &str, message: impl Into<String>) -> V3Error {
    V3Error::new(code, V3ErrorCategory::Validation, false, message)
}

fn io_error(code: &'static str, error: std::io::Error) -> V3Error {
    V3Error::new(code, V3ErrorCategory::Internal, false, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v3::{
        initialize_v3, update_project_settings, OutputLanguage, V3ProjectSettingsUpdateRequest,
    };
    use std::fs;

    fn project() -> PathBuf {
        let root = std::env::temp_dir().join(format!("vibehub-v3-host-mcp-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        initialize_v3(&root).unwrap();
        update_project_settings(
            &root,
            V3ProjectSettingsUpdateRequest {
                expected_revision: 0,
                output_language: OutputLanguage::EnUs,
                agent_spec_targets: vec![AgentSpecTarget::Codex],
                repository_remote_url: None,
            },
        )
        .unwrap();
        root
    }

    #[test]
    fn global_migration_preserves_unrelated_bytes_and_is_idempotent() {
        let home = std::env::temp_dir().join(format!("vibehub-v3-host-home-{}", Uuid::new_v4()));
        fs::create_dir_all(home.join(".codex")).unwrap();
        let original = "# keep this comment\nmodel = \"gpt\"\n\n[mcp_servers.other]\ncommand = \"other\"\n\n[mcp_servers.vibehub]\ncommand = \"/bin/vibehub\"\nargs = [\"mcp-stdio\", \"/wrong/project\"]\n\n[plugins]\nkept = true\n";
        fs::write(home.join(".codex/config.toml"), original).unwrap();
        let result = migrate_global_codex_config(&home, None).unwrap();
        assert_eq!(result.status, GlobalMcpMigrationStatus::Migrated);
        let migrated = fs::read_to_string(home.join(".codex/config.toml")).unwrap();
        assert!(migrated.contains("# keep this comment"));
        assert!(migrated.contains("[mcp_servers.other]"));
        assert!(migrated.contains("[plugins]"));
        assert!(!migrated.contains("mcp_servers.vibehub"));
        assert!(home.join(".codex/config.toml.vibehub-backup").exists());
        assert_eq!(
            migrate_global_codex_config(&home, None).unwrap().status,
            GlobalMcpMigrationStatus::Unchanged
        );
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn global_migration_removes_orphaned_vibehub_tool_subtables() {
        let home = std::env::temp_dir().join(format!("vibehub-v3-host-home-{}", Uuid::new_v4()));
        fs::create_dir_all(home.join(".codex")).unwrap();
        let original = "[mcp_servers.other]\ncommand = \"other\"\n\n[mcp_servers.vibehub.tools.task_candidates]\napproval_mode = \"approve\"\n\n[plugins]\nkept = true\n";
        fs::write(home.join(".codex/config.toml"), original).unwrap();

        let result = migrate_global_codex_config(&home, None).unwrap();

        assert_eq!(result.status, GlobalMcpMigrationStatus::Migrated);
        let migrated = fs::read_to_string(home.join(".codex/config.toml")).unwrap();
        assert!(migrated.contains("[mcp_servers.other]"));
        assert!(migrated.contains("[plugins]"));
        assert!(!migrated.contains("mcp_servers.vibehub"));
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn project_sync_writes_three_explicit_absolute_launchers() {
        let root = project();
        let binary = root.join("target/vibehub");
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        let result = sync_host_mcp_configs_with_home(&root, &binary, None).unwrap();
        assert_eq!(result.status, HostMcpSyncStatus::Synchronized);
        let codex = fs::read_to_string(root.join(".codex/config.toml")).unwrap();
        let codex: toml::Value = toml::from_str(&codex).unwrap();
        let canonical_root = root.canonicalize().unwrap().to_string_lossy().into_owned();
        assert_eq!(
            codex["mcp_servers"]["vibehub"]["command"].as_str(),
            Some(binary.to_string_lossy().as_ref())
        );
        assert_eq!(
            codex["mcp_servers"]["vibehub"]["args"][1].as_str(),
            Some(canonical_root.as_str())
        );
        let claude: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(root.join(".mcp.json")).unwrap()).unwrap();
        assert_eq!(
            claude["mcpServers"]["vibehub"]["args"][1],
            serde_json::Value::String(canonical_root.clone())
        );
        let opencode: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(root.join("opencode.json")).unwrap()).unwrap();
        assert_eq!(opencode["mcp"]["vibehub"]["command"][1], "mcp-stdio");
        let inspections = inspect_host_mcp_configs(
            &resolve_project_scopes(&root, None).unwrap(),
            &[AgentSpecTarget::Codex],
            None,
        );
        assert!(inspections
            .iter()
            .any(|item| item.scope == HostConfigScope::Project
                && item.status == HostConfigStatus::InSync));
        fs::remove_dir_all(root).unwrap();
    }

    fn sync_host_mcp_configs_with_home(
        root: &Path,
        binary: &Path,
        home: Option<&Path>,
    ) -> Result<HostMcpSyncResult, V3Error> {
        let scopes = resolve_project_scopes(root, None)?;
        let targets = [
            AgentSpecTarget::Codex,
            AgentSpecTarget::ClaudeCode,
            AgentSpecTarget::Opencode,
        ];
        let inspections = inspect_host_mcp_configs_inner(&scopes, &targets, home, false);
        let mut writes = Vec::new();
        for inspection in inspections
            .iter()
            .filter(|item| item.scope == HostConfigScope::Project)
        {
            let path = resolve_inspection_path(&scopes, inspection)?;
            let snapshot = read_snapshot(&path)?;
            let replacement = prepare_project_config(
                &path,
                format_for(inspection.consumer),
                snapshot.bytes.as_deref(),
                binary,
                &scopes.control_root,
                inspection,
            )?;
            writes.push(PreparedConfigWrite {
                path,
                replacement,
                snapshot,
                backup: false,
            });
        }
        for write in &writes {
            apply_config_write(write)?;
        }
        Ok(HostMcpSyncResult {
            status: HostMcpSyncStatus::Synchronized,
            inspections: inspect_host_mcp_configs_inner(&scopes, &targets, home, false),
            written_paths: writes
                .iter()
                .map(|write| write.path.to_string_lossy().into_owned())
                .collect(),
            skipped_paths: Vec::new(),
            blocking_paths: Vec::new(),
            global_migration: unchanged_global_result(home),
        })
    }
}

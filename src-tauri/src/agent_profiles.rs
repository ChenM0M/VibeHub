//! V3 Agent Profile typed commands.
//!
//! This module is intentionally the only Tauri boundary for Agent Profile
//! configuration. The renderer sends stable IDs and managed values; it never
//! sends a filesystem path to open. OpenCode, Claude Code, and Codex may send
//! an optional API key that is written only into the selected home-directory
//! Agent configuration. Real paths are resolved from observed runtime targets
//! and discovery.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

use vibehub_core::v3::{
    self, AgentKind, ClaudeCodeProfileView, ClaudeCredentialKind, ClaudeSettingsPatch,
    ClaudeSettingsScope, CodexConfigPatch, CodexCredentialKind, CodexProfileView, CodexProtocol,
    CodexProviderPatch, ConfigFormat, DocumentRevision, NativeConfigPath, OpenCodeConfigPatch,
    ParsedConfig,
    OpenCodeModelPatch, OpenCodeProfileView, OpenCodeProviderPatch, ProtocolKind,
    ProtocolResolution, RuntimeTarget, RuntimeTargetKind, StorageError, WriteReport,
};

const SCHEMA_VERSION: &str = "1.0";
const MODEL_VERSION: &str = "v3-agent-profile-commands-1";

#[derive(Debug, Clone, Serialize)]
pub struct AgentProfileCommandError {
    pub code: String,
    pub category: String,
    pub recoverable: bool,
    pub message_key: String,
    pub details: BTreeMap<String, Value>,
    pub evidence_refs: Vec<String>,
}

impl AgentProfileCommandError {
    fn new(
        code: impl Into<String>,
        category: &str,
        recoverable: bool,
        message: impl Into<String>,
    ) -> Self {
        let code = code.into();
        let mut details = BTreeMap::new();
        details.insert("message".to_owned(), Value::String(message.into()));
        Self {
            message_key: code.clone(),
            code,
            category: category.to_owned(),
            recoverable,
            details,
            evidence_refs: Vec::new(),
        }
    }

    fn validation(code: &str, message: impl Into<String>) -> Self {
        Self::new(code, "validation", false, message)
    }

    fn not_found(code: &str, message: impl Into<String>) -> Self {
        Self::new(code, "not_found", true, message)
    }

    fn unsupported(code: &str, message: impl Into<String>) -> Self {
        Self::new(code, "unsupported", false, message)
    }

    fn conflict(code: &str, message: impl Into<String>) -> Self {
        Self::new(code, "conflict", true, message)
    }

    fn internal(code: &str, message: impl Into<String>) -> Self {
        Self::new(code, "internal", true, message)
    }
}

impl Display for AgentProfileCommandError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let message = self
            .details
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("Agent Profile command failed");
        write!(formatter, "{}: {}", self.code, message)
    }
}

impl std::error::Error for AgentProfileCommandError {}

impl From<StorageError> for AgentProfileCommandError {
    fn from(error: StorageError) -> Self {
        let category = if error.code.contains("PERMISSION") || error.code.contains("ACCESS") {
            "permission"
        } else if error.code.contains("REVISION") || error.code.contains("CONFLICT") {
            "conflict"
        } else if error.code.contains("UNSUPPORTED") || error.code.contains("UNKNOWN") {
            "unsupported"
        } else if error.code.contains("NOT_FOUND") || error.code.contains("MISSING") {
            "not_found"
        } else {
            "validation"
        };
        let recoverable = matches!(category, "permission" | "conflict" | "not_found")
            || error.code.contains("BACKUP")
            || error.code.contains("ROLLBACK");
        Self::new(
            error.code,
            category,
            recoverable,
            safe_storage_message(&error.message),
        )
    }
}

fn safe_storage_message(message: &str) -> String {
    // Core adapters never include credential values in their errors. Keep this
    // final boundary defensive so a future adapter cannot echo common bearer
    // token forms into a Tauri error payload.
    let mut output = message.to_owned();
    for marker in ["sk-", "api_key=", "apiKey=", "bearer "] {
        if let Some(index) = output
            .to_ascii_lowercase()
            .find(&marker.to_ascii_lowercase())
        {
            output.truncate(index);
            output.push_str("[redacted]");
        }
    }
    output
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentProfileTargetRequest {
    pub agent: AgentKind,
    pub runtime_target_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentProfileReadRequest {
    pub agent: AgentKind,
    pub runtime_target_id: String,
    pub profile_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentProfileSaveRequest {
    pub agent: AgentKind,
    pub runtime_target_id: String,
    pub profile_id: String,
    pub expected_revision: u64,
    pub profile: AgentProfileDocumentInput,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentProfileValidateRequest {
    pub agent: AgentKind,
    pub runtime_target_id: String,
    pub profile: AgentProfileDocumentInput,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentProfileActivateRequest {
    pub agent: AgentKind,
    pub runtime_target_id: String,
    pub profile_id: String,
    pub expected_revision: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentProfileLaunchRequest {
    pub agent: AgentKind,
    pub runtime_target_id: String,
    pub profile_id: String,
    pub launch_mode: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentProfileRestoreRequest {
    pub agent: AgentKind,
    pub runtime_target_id: String,
    pub profile_id: String,
    pub expected_revision: u64,
    pub backup_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentProfileDiagnosticsRequest {
    pub agent: AgentKind,
    pub runtime_target_id: String,
    #[serde(default)]
    pub profile_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentProfileCreateRequest {
    pub agent: AgentKind,
    pub runtime_target_id: String,
    pub profile_name: String,
    #[serde(default)]
    pub template_profile_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentProfileCloneRequest {
    pub agent: AgentKind,
    pub runtime_target_id: String,
    pub source_profile_id: String,
    pub profile_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentProfileRenameRequest {
    pub agent: AgentKind,
    pub runtime_target_id: String,
    pub profile_id: String,
    pub new_profile_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentProfileDeleteRequest {
    pub agent: AgentKind,
    pub runtime_target_id: String,
    pub profile_id: String,
    #[serde(default)]
    pub replacement_profile_id: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct AgentProfileListModelsRequest {
    pub agent: AgentKind,
    pub runtime_target_id: String,
    pub profile_id: String,
    pub provider_id: String,
    pub base_url: String,
    #[serde(default)]
    pub protocol: String,
    #[serde(default)]
    pub api_key: Option<String>,
}

impl std::fmt::Debug for AgentProfileListModelsRequest {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AgentProfileListModelsRequest")
            .field("agent", &self.agent)
            .field("runtime_target_id", &self.runtime_target_id)
            .field("profile_id", &self.profile_id)
            .field("provider_id", &self.provider_id)
            .field("base_url", &self.base_url)
            .field("protocol", &self.protocol)
            .field("api_key", &self.api_key.as_ref().map(|_| "[redacted]"))
            .finish()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentProfileListModelsResult {
    pub endpoint: String,
    pub models: Vec<UpstreamModelWire>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpstreamModelWire {
    pub model_id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentProfileDocumentInput {
    pub profile_id: String,
    pub display_name: String,
    pub agent: AgentKind,
    pub runtime_target_id: String,
    pub source: AgentProfileSourceInput,
    pub revision: AgentProfileRevision,
    pub managed: ManagedProfileInput,
    pub default_state: Value,
    pub preservation: Value,
    pub protocol: Value,
    pub schema_capability: Value,
    pub launch: Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentProfileSourceInput {
    pub path: AgentProfilePathInput,
    pub format: String,
    pub scope: String,
    pub profile_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentProfilePathInput {
    pub native: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentProfileRevision {
    pub revision: u64,
    pub content_sha256: String,
    pub observed_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManagedProfileInput {
    pub providers: Vec<ProviderProfileInput>,
    pub default_provider_id: Option<String>,
    pub default_model_id: Option<String>,
    pub small_model_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderProfileInput {
    pub provider_id: String,
    pub display_name: String,
    pub base_url: String,
    pub credential: CredentialReferenceInput,
    pub protocol: ProtocolCapabilityInput,
    pub models: Vec<ModelProfileInput>,
}

#[derive(Clone, Deserialize)]
pub struct CredentialReferenceInput {
    pub kind: String,
    pub reference: String,
    pub display: String,
    pub secret_state: String,
    pub persisted_in_config: bool,
    #[serde(default)]
    pub secret: Option<String>,
    #[serde(default)]
    pub clear_secret: bool,
}

impl std::fmt::Debug for CredentialReferenceInput {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CredentialReferenceInput")
            .field("kind", &self.kind)
            .field("reference", &self.reference)
            .field("display", &self.display)
            .field("secret_state", &self.secret_state)
            .field("persisted_in_config", &self.persisted_in_config)
            .field("secret", &self.secret.as_ref().map(|_| "[redacted]"))
            .field("clear_secret", &self.clear_secret)
            .finish()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProtocolCapabilityInput {
    pub native_protocol: String,
    pub upstream_protocol: String,
    pub route: String,
    pub compatibility: String,
    pub adapter_id: Option<String>,
    pub adapter_version: Option<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelProfileInput {
    pub model_id: String,
    pub display_name: String,
    pub enabled: bool,
    pub thinking: ThinkingProfileInput,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ThinkingProfileInput {
    pub supports_reasoning: bool,
    pub supports_effort: bool,
    pub selected: Option<String>,
    pub options: Vec<String>,
    pub custom_allowed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigRevisionWire {
    pub revision: u64,
    pub content_sha256: String,
    pub observed_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentProfileSummaryWire {
    pub profile_id: String,
    pub display_name: String,
    pub agent: AgentKind,
    pub runtime_target_id: String,
    pub source_path: NativeConfigPath,
    pub revision: ConfigRevisionWire,
    pub is_default: bool,
    pub compatibility: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentProfileDiscoverResult {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub generated_at: String,
    pub model_version: &'static str,
    pub freshness: &'static str,
    pub completeness: &'static str,
    pub evidence_refs: Vec<Value>,
    pub warnings: Vec<Value>,
    pub errors: Vec<Value>,
    pub agent: AgentKind,
    pub runtime_targets: Vec<RuntimeTarget>,
    pub profiles: Vec<AgentProfileSummaryWire>,
    pub default_profile_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentProfileReadResult {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub generated_at: String,
    pub model_version: &'static str,
    pub freshness: &'static str,
    pub completeness: &'static str,
    pub evidence_refs: Vec<Value>,
    pub warnings: Vec<Value>,
    pub errors: Vec<Value>,
    pub agent: AgentKind,
    pub runtime_target: RuntimeTarget,
    pub source: Value,
    pub revision: ConfigRevisionWire,
    pub managed_fields: Value,
    pub default_state: Value,
    pub protocol: Value,
    pub profile: Value,
    pub schema_capability: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentProfileSaveResult {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub generated_at: String,
    pub model_version: &'static str,
    pub freshness: &'static str,
    pub completeness: &'static str,
    pub evidence_refs: Vec<Value>,
    pub warnings: Vec<Value>,
    pub errors: Vec<Value>,
    pub operation: String,
    pub profile: Value,
    pub revision: ConfigRevisionWire,
    pub backup_path: Option<NativeConfigPath>,
    pub rollback_available: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentProfileDiagnosticsResult {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub generated_at: String,
    pub model_version: &'static str,
    pub freshness: &'static str,
    pub completeness: &'static str,
    pub evidence_refs: Vec<Value>,
    pub warnings: Vec<Value>,
    pub errors: Vec<Value>,
    pub agent: AgentKind,
    pub runtime_target: RuntimeTarget,
    pub status: String,
    pub profiles: Vec<AgentProfileSummaryWire>,
    pub source_paths: Vec<NativeConfigPath>,
    pub auth_files_skipped: bool,
}

#[tauri::command]
pub async fn v3_agent_profile_discover(
    request: AgentProfileTargetRequest,
) -> Result<AgentProfileDiscoverResult, AgentProfileCommandError> {
    run_blocking(move || discover(request)).await
}

#[tauri::command]
pub async fn v3_agent_profile_runtime_targets(
) -> Result<Vec<RuntimeTarget>, AgentProfileCommandError> {
    run_blocking(runtime_targets).await
}

#[tauri::command]
pub async fn v3_agent_profile_read(
    request: AgentProfileReadRequest,
) -> Result<AgentProfileReadResult, AgentProfileCommandError> {
    run_blocking(move || read(request)).await
}

#[tauri::command]
pub async fn v3_agent_profile_save(
    request: AgentProfileSaveRequest,
) -> Result<AgentProfileSaveResult, AgentProfileCommandError> {
    run_blocking(move || save(request)).await
}

#[tauri::command]
pub async fn v3_agent_profile_validate(
    request: AgentProfileValidateRequest,
) -> Result<AgentProfileSaveResult, AgentProfileCommandError> {
    run_blocking(move || validate(request)).await
}

#[tauri::command]
pub async fn v3_agent_profile_activate(
    request: AgentProfileActivateRequest,
) -> Result<AgentProfileSaveResult, AgentProfileCommandError> {
    run_blocking(move || activate(request)).await
}

#[tauri::command]
pub async fn v3_agent_profile_launch(
    request: AgentProfileLaunchRequest,
) -> Result<AgentProfileSaveResult, AgentProfileCommandError> {
    run_blocking(move || launch(request)).await
}

#[tauri::command]
pub async fn v3_agent_profile_restore(
    request: AgentProfileRestoreRequest,
) -> Result<AgentProfileSaveResult, AgentProfileCommandError> {
    run_blocking(move || restore(request)).await
}

#[tauri::command]
pub async fn v3_agent_profile_diagnostics(
    request: AgentProfileDiagnosticsRequest,
) -> Result<AgentProfileDiagnosticsResult, AgentProfileCommandError> {
    run_blocking(move || diagnostics(request)).await
}

#[tauri::command]
pub async fn v3_agent_profile_create(
    request: AgentProfileCreateRequest,
) -> Result<AgentProfileSaveResult, AgentProfileCommandError> {
    run_blocking(move || create_profile(request)).await
}

#[tauri::command]
pub async fn v3_agent_profile_clone(
    request: AgentProfileCloneRequest,
) -> Result<AgentProfileSaveResult, AgentProfileCommandError> {
    run_blocking(move || clone_profile(request)).await
}

#[tauri::command]
pub async fn v3_agent_profile_rename(
    request: AgentProfileRenameRequest,
) -> Result<AgentProfileSaveResult, AgentProfileCommandError> {
    run_blocking(move || rename_profile(request)).await
}

#[tauri::command]
pub async fn v3_agent_profile_delete(
    request: AgentProfileDeleteRequest,
) -> Result<AgentProfileSaveResult, AgentProfileCommandError> {
    run_blocking(move || delete_profile(request)).await
}

#[tauri::command]
pub async fn v3_agent_profile_list_upstream_models(
    request: AgentProfileListModelsRequest,
) -> Result<AgentProfileListModelsResult, AgentProfileCommandError> {
    list_upstream_models(request).await
}

async fn run_blocking<T, F>(operation: F) -> Result<T, AgentProfileCommandError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, AgentProfileCommandError> + Send + 'static,
{
    tokio::task::spawn_blocking(operation)
        .await
        .map_err(|error| {
            AgentProfileCommandError::internal("AGENT_PROFILE_TASK_FAILED", error.to_string())
        })?
}

fn discover(
    request: AgentProfileTargetRequest,
) -> Result<AgentProfileDiscoverResult, AgentProfileCommandError> {
    let target = resolve_runtime_target(&request.runtime_target_id)?;
    let profiles = discover_locations(&request.agent, &target)?;
    let summaries = profiles
        .iter()
        .map(|profile| summary_for(&target, profile))
        .collect::<Result<Vec<_>, _>>()?;
    let default_profile_id = summaries
        .iter()
        .find(|summary| summary.is_default)
        .map(|summary| summary.profile_id.clone());
    Ok(AgentProfileDiscoverResult {
        kind: "agent_profile_discover_result",
        schema_version: SCHEMA_VERSION,
        generated_at: now(),
        model_version: MODEL_VERSION,
        freshness: "fresh",
        completeness: "complete",
        evidence_refs: Vec::new(),
        warnings: Vec::new(),
        errors: Vec::new(),
        agent: request.agent,
        runtime_targets: vec![target],
        profiles: summaries,
        default_profile_id,
    })
}

fn runtime_targets() -> Result<Vec<RuntimeTarget>, AgentProfileCommandError> {
    v3::discover_runtime_targets().map_err(Into::into)
}

fn read(
    request: AgentProfileReadRequest,
) -> Result<AgentProfileReadResult, AgentProfileCommandError> {
    let target = resolve_runtime_target(&request.runtime_target_id)?;
    let profile = locate_profile(&target, &request.agent, &request.profile_id)?;
    read_result(&target, &profile)
}

fn save(
    request: AgentProfileSaveRequest,
) -> Result<AgentProfileSaveResult, AgentProfileCommandError> {
    let target = resolve_runtime_target(&request.runtime_target_id)?;
    save_on_target(target, request)
}

fn save_on_target(
    target: RuntimeTarget,
    request: AgentProfileSaveRequest,
) -> Result<AgentProfileSaveResult, AgentProfileCommandError> {
    validate_profile_input(&request.agent, &request.runtime_target_id, &request.profile)?;
    if request.profile.profile_id != request.profile_id {
        return Err(AgentProfileCommandError::validation(
            "AGENT_PROFILE_ID_MISMATCH",
            "request profile_id and document profile_id must match",
        ));
    }
    let located = locate_profile(&target, &request.agent, &request.profile_id)?;
    validate_source_and_revision(
        &target,
        &located,
        &request.profile,
        request.expected_revision,
    )?;
    let write = save_managed(&target, &located, &request.profile.managed)?;
    let refreshed = locate_profile(&target, &request.agent, &request.profile_id)?;
    save_result("save", &target, &refreshed, write)
}

fn validate(
    request: AgentProfileValidateRequest,
) -> Result<AgentProfileSaveResult, AgentProfileCommandError> {
    let target = resolve_runtime_target(&request.runtime_target_id)?;
    validate_profile_input(&request.agent, &request.runtime_target_id, &request.profile)?;
    let located = locate_profile(&target, &request.agent, &request.profile.profile_id)?;
    validate_source_and_revision(
        &target,
        &located,
        &request.profile,
        request.profile.revision.revision,
    )?;
    let _ = patch_for(&request.agent, &request.profile.managed)?;
    save_result("validate", &target, &located, None)
}

fn activate(
    request: AgentProfileActivateRequest,
) -> Result<AgentProfileSaveResult, AgentProfileCommandError> {
    let target = resolve_runtime_target(&request.runtime_target_id)?;
    activate_on_target(target, request)
}

fn activate_on_target(
    target: RuntimeTarget,
    request: AgentProfileActivateRequest,
) -> Result<AgentProfileSaveResult, AgentProfileCommandError> {
    let located = locate_profile(&target, &request.agent, &request.profile_id)?;
    ensure_revision(&located, request.expected_revision)?;
    let operation = match &located {
        LocatedProfile::OpenCode(_) => {
            return save_result("activate", &target, &located, None).map(|mut result| {
                result.warnings.push(json!({
                    "code":"OPENCODE_NATIVE_CONFIG_IS_DEFAULT",
                    "severity":"info",
                    "message_key":"OPENCODE_NATIVE_CONFIG_IS_DEFAULT",
                    "evidence_refs":[]
                }));
                result
            });
        }
        LocatedProfile::Claude(profile) => {
            let operation = v3::activate_claude_profile(&target, &profile.source_path)?;
            OperationWrite {
                profile: LocatedProfile::Claude(operation.profile),
                write: operation.write,
            }
        }
        LocatedProfile::Codex(profile) => {
            let operation = v3::activate_codex_profile(&target, &profile.source_path)?;
            OperationWrite {
                profile: LocatedProfile::Codex(operation.profile),
                write: operation.write,
            }
        }
    };
    let refreshed =
        locate_profile(&target, &request.agent, &request.profile_id).unwrap_or(operation.profile);
    save_result("activate", &target, &refreshed, operation.write)
}

fn launch(
    request: AgentProfileLaunchRequest,
) -> Result<AgentProfileSaveResult, AgentProfileCommandError> {
    if !matches!(request.launch_mode.as_str(), "temporary" | "default") {
        return Err(AgentProfileCommandError::validation(
            "AGENT_PROFILE_LAUNCH_MODE_INVALID",
            "launch_mode must be temporary or default",
        ));
    }
    let target = resolve_runtime_target(&request.runtime_target_id)?;
    let located = locate_profile(&target, &request.agent, &request.profile_id)?;
    let arguments = launch_arguments_for(&located, &request.launch_mode)?;
    let working_directory = launch_working_directory(&target)?;
    let executable = executable_for_agent(&request.agent);
    let process_id = crate::launcher::Launcher::launch_agent(
        executable,
        &arguments,
        &working_directory,
        match target.kind {
            RuntimeTargetKind::Host => "host",
            RuntimeTargetKind::Wsl => "wsl",
        },
        target.distribution.as_deref(),
    )
    .map_err(|error| {
        AgentProfileCommandError::new(
            "AGENT_PROFILE_LAUNCH_FAILED",
            "internal",
            true,
            safe_storage_message(&error.to_string()),
        )
    })?;
    let result = save_result("launch", &target, &located, None).map(|mut result| {
        result.warnings.push(json!({
            "code":"AGENT_PROFILE_LAUNCHED",
            "severity":"info",
            "message_key":"AGENT_PROFILE_LAUNCHED",
            "details":{
                "launch_mode":request.launch_mode,
                "executable":executable,
                "process_spawned":true,
                "process_id":process_id
            },
            "evidence_refs":[]
        }));
        result
    })?;
    Ok(result)
}

fn executable_for_agent(agent: &AgentKind) -> &'static str {
    match agent {
        AgentKind::ClaudeCode => "claude",
        AgentKind::Opencode => "opencode",
        AgentKind::Codex => "codex",
    }
}

fn launch_arguments_for(
    profile: &LocatedProfile,
    launch_mode: &str,
) -> Result<Vec<String>, AgentProfileCommandError> {
    if launch_mode == "default" {
        if !profile.is_default() && !matches!(profile, LocatedProfile::OpenCode(_)) {
            return Err(AgentProfileCommandError::validation(
                "AGENT_PROFILE_DEFAULT_LAUNCH_REQUIRES_DEFAULT",
                "default launch requires the selected Profile to be the active default",
            ));
        }
        return Ok(Vec::new());
    }

    Ok(match profile {
        LocatedProfile::OpenCode(_) => Vec::new(),
        LocatedProfile::Claude(view) => view.launch.arguments.clone(),
        LocatedProfile::Codex(view) => view
            .launch
            .as_ref()
            .map(|launch| launch.arguments.clone())
            .unwrap_or_default(),
    })
}

fn launch_working_directory(target: &RuntimeTarget) -> Result<String, AgentProfileCommandError> {
    if !matches!(target.kind, RuntimeTargetKind::Wsl) {
        return Ok(target.home_path.native.clone());
    }

    #[cfg(target_os = "windows")]
    {
        let distribution = target.distribution.as_deref().ok_or_else(|| {
            AgentProfileCommandError::validation(
                "RUNTIME_WSL_DISTRIBUTION_MISSING",
                "WSL runtime target is missing its distribution",
            )
        })?;
        let native = target.home_path.native.replace('\\', "/");
        let prefix = format!("//wsl$/{}", distribution.to_ascii_lowercase());
        let lower = native.to_ascii_lowercase();
        let Some(suffix) = lower.strip_prefix(&prefix) else {
            return Err(AgentProfileCommandError::validation(
                "RUNTIME_WSL_HOME_PATH_INVALID",
                "WSL home path is not inside the observed distribution",
            ));
        };
        let suffix = suffix.trim_start_matches('/');
        return Ok(format!("/{suffix}"));
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(target.home_path.native.clone())
    }
}

fn restore(
    request: AgentProfileRestoreRequest,
) -> Result<AgentProfileSaveResult, AgentProfileCommandError> {
    let target = resolve_runtime_target(&request.runtime_target_id)?;
    restore_on_target(target, request)
}

fn restore_on_target(
    target: RuntimeTarget,
    request: AgentProfileRestoreRequest,
) -> Result<AgentProfileSaveResult, AgentProfileCommandError> {
    let located = locate_profile(&target, &request.agent, &request.profile_id)?;
    ensure_revision(&located, request.expected_revision)?;
    let current_path = located.source_path().to_path_buf();
    let backup_path = validate_backup_path(&target, &current_path, &request.backup_path)?;
    let restored = v3::restore_document(
        &target,
        &current_path,
        backup_path,
        located.document_revision(),
    )?;
    let refreshed = locate_profile(&target, &request.agent, &request.profile_id)?;
    save_result("restore", &target, &refreshed, Some(restored))
}

fn diagnostics(
    request: AgentProfileDiagnosticsRequest,
) -> Result<AgentProfileDiagnosticsResult, AgentProfileCommandError> {
    let target = resolve_runtime_target(&request.runtime_target_id)?;
    let profiles = discover_locations(&request.agent, &target)?;
    let summaries = profiles
        .iter()
        .map(|profile| summary_for(&target, profile))
        .collect::<Result<Vec<_>, _>>()?;
    let source_paths = if let Some(profile_id) = request.profile_id {
        summaries
            .iter()
            .filter(|summary| summary.profile_id == profile_id)
            .map(|summary| summary.source_path.clone())
            .collect()
    } else {
        summaries
            .iter()
            .map(|summary| summary.source_path.clone())
            .collect()
    };
    Ok(AgentProfileDiagnosticsResult {
        kind: "agent_profile_diagnostics_result",
        schema_version: SCHEMA_VERSION,
        generated_at: now(),
        model_version: MODEL_VERSION,
        freshness: "fresh",
        completeness: "complete",
        evidence_refs: Vec::new(),
        warnings: vec![json!({
            "code":"AUTH_FILES_NOT_READ",
            "severity":"info",
            "message_key":"AUTH_FILES_NOT_READ",
            "details":{"scope":"agent_profile_config_only"},
            "evidence_refs":[]
        })],
        errors: Vec::new(),
        agent: request.agent,
        runtime_target: target,
        status: if summaries.is_empty() {
            "empty".to_owned()
        } else {
            "ready".to_owned()
        },
        profiles: summaries,
        source_paths,
        auth_files_skipped: true,
    })
}

fn create_profile(
    request: AgentProfileCreateRequest,
) -> Result<AgentProfileSaveResult, AgentProfileCommandError> {
    let target = resolve_runtime_target(&request.runtime_target_id)?;
    create_profile_on_target(target, request)
}

fn create_profile_on_target(
    target: RuntimeTarget,
    request: AgentProfileCreateRequest,
) -> Result<AgentProfileSaveResult, AgentProfileCommandError> {
    let template = request
        .template_profile_id
        .as_deref()
        .map(|profile_id| locate_profile(&target, &request.agent, profile_id))
        .transpose()?;
    let operation = match request.agent {
        AgentKind::Opencode => {
            return Err(AgentProfileCommandError::unsupported(
                "OPENCODE_PROFILE_CRUD_UNSUPPORTED",
                "OpenCode is managed as its native user configuration, not named Profiles",
            ))
        }
        AgentKind::ClaudeCode => {
            let operation = v3::create_claude_profile(
                &target,
                &request.profile_name,
                template.as_ref().map(|profile| profile.source_path()),
            )?;
            OperationWrite {
                profile: LocatedProfile::Claude(operation.profile),
                write: operation.write.or(operation.index_write),
            }
        }
        AgentKind::Codex => {
            let operation = v3::create_codex_profile(
                &target,
                &request.profile_name,
                template.as_ref().map(|profile| profile.source_path()),
            )?;
            OperationWrite {
                profile: LocatedProfile::Codex(operation.profile),
                write: operation.write.or(operation.index_write),
            }
        }
    };
    save_result("create", &target, &operation.profile, operation.write)
}

fn clone_profile(
    request: AgentProfileCloneRequest,
) -> Result<AgentProfileSaveResult, AgentProfileCommandError> {
    let target = resolve_runtime_target(&request.runtime_target_id)?;
    clone_profile_on_target(target, request)
}

fn clone_profile_on_target(
    target: RuntimeTarget,
    request: AgentProfileCloneRequest,
) -> Result<AgentProfileSaveResult, AgentProfileCommandError> {
    let source = locate_profile(&target, &request.agent, &request.source_profile_id)?;
    let operation = match request.agent {
        AgentKind::Opencode => {
            return Err(AgentProfileCommandError::unsupported(
                "OPENCODE_PROFILE_CRUD_UNSUPPORTED",
                "OpenCode is managed as its native user configuration, not named Profiles",
            ))
        }
        AgentKind::ClaudeCode => {
            let operation =
                v3::clone_claude_profile(&target, source.source_path(), &request.profile_name)?;
            OperationWrite {
                profile: LocatedProfile::Claude(operation.profile),
                write: operation.write.or(operation.index_write),
            }
        }
        AgentKind::Codex => {
            let operation =
                v3::clone_codex_profile(&target, source.source_path(), &request.profile_name)?;
            OperationWrite {
                profile: LocatedProfile::Codex(operation.profile),
                write: operation.write.or(operation.index_write),
            }
        }
    };
    save_result("clone", &target, &operation.profile, operation.write)
}

fn rename_profile(
    request: AgentProfileRenameRequest,
) -> Result<AgentProfileSaveResult, AgentProfileCommandError> {
    let target = resolve_runtime_target(&request.runtime_target_id)?;
    rename_profile_on_target(target, request)
}

fn rename_profile_on_target(
    target: RuntimeTarget,
    request: AgentProfileRenameRequest,
) -> Result<AgentProfileSaveResult, AgentProfileCommandError> {
    let source = locate_profile(&target, &request.agent, &request.profile_id)?;
    let profile = match request.agent {
        AgentKind::Opencode => {
            return Err(AgentProfileCommandError::unsupported(
                "OPENCODE_PROFILE_CRUD_UNSUPPORTED",
                "OpenCode is managed as its native user configuration, not named Profiles",
            ))
        }
        AgentKind::ClaudeCode => LocatedProfile::Claude(v3::rename_claude_profile(
            &target,
            source.source_path(),
            &request.new_profile_name,
        )?),
        AgentKind::Codex => LocatedProfile::Codex(v3::rename_codex_profile(
            &target,
            source.source_path(),
            &request.new_profile_name,
        )?),
    };
    save_result("rename", &target, &profile, None)
}

fn delete_profile(
    request: AgentProfileDeleteRequest,
) -> Result<AgentProfileSaveResult, AgentProfileCommandError> {
    let target = resolve_runtime_target(&request.runtime_target_id)?;
    delete_profile_on_target(target, request)
}

fn delete_profile_on_target(
    target: RuntimeTarget,
    request: AgentProfileDeleteRequest,
) -> Result<AgentProfileSaveResult, AgentProfileCommandError> {
    let source = locate_profile(&target, &request.agent, &request.profile_id)?;
    let replacement = request
        .replacement_profile_id
        .as_deref()
        .map(|profile_id| locate_profile(&target, &request.agent, profile_id))
        .transpose()?;
    let operation = match request.agent {
        AgentKind::Opencode => {
            return Err(AgentProfileCommandError::unsupported(
                "OPENCODE_PROFILE_CRUD_UNSUPPORTED",
                "OpenCode is managed as its native user configuration, not named Profiles",
            ))
        }
        AgentKind::ClaudeCode => {
            let operation = v3::delete_claude_profile(
                &target,
                &v3::ClaudeDeleteRequest {
                    profile_path: source.source_path().to_path_buf(),
                    replacement_profile_path: replacement
                        .as_ref()
                        .map(|profile| profile.source_path().to_path_buf()),
                },
            )?;
            OperationWrite {
                profile: LocatedProfile::Claude(operation.profile),
                write: operation.write.or(operation.index_write),
            }
        }
        AgentKind::Codex => {
            let operation = v3::delete_codex_profile(
                &target,
                &v3::CodexDeleteRequest {
                    profile_path: source.source_path().to_path_buf(),
                    replacement_profile_path: replacement
                        .as_ref()
                        .map(|profile| profile.source_path().to_path_buf()),
                },
            )?;
            OperationWrite {
                profile: LocatedProfile::Codex(operation.profile),
                write: operation.write.or(operation.index_write),
            }
        }
    };
    save_result("delete", &target, &operation.profile, operation.write)
}

#[derive(Debug)]
struct OperationWrite {
    profile: LocatedProfile,
    write: Option<WriteReport>,
}

#[derive(Debug, Clone)]
enum LocatedProfile {
    OpenCode(OpenCodeProfileView),
    Claude(ClaudeCodeProfileView),
    Codex(CodexProfileView),
}

impl LocatedProfile {
    fn agent(&self) -> AgentKind {
        match self {
            Self::OpenCode(_) => AgentKind::Opencode,
            Self::Claude(_) => AgentKind::ClaudeCode,
            Self::Codex(_) => AgentKind::Codex,
        }
    }

    fn profile_id(&self) -> String {
        match self {
            Self::OpenCode(view) => opencode_profile_id(&view.source_path),
            Self::Claude(view) => view.profile_id.clone(),
            Self::Codex(view) => view.profile_id.clone(),
        }
    }

    fn display_name(&self) -> String {
        match self {
            Self::OpenCode(view) => view
                .source_path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("OpenCode 配置")
                .to_owned(),
            Self::Claude(view) => view.display_name.clone(),
            Self::Codex(view) => view.display_name.clone(),
        }
    }

    fn source_path(&self) -> &Path {
        match self {
            Self::OpenCode(view) => &view.source_path,
            Self::Claude(view) => &view.source_path,
            Self::Codex(view) => &view.source_path,
        }
    }

    fn document_revision(&self) -> &DocumentRevision {
        match self {
            Self::OpenCode(view) => &view.revision,
            Self::Claude(view) => &view.revision,
            Self::Codex(view) => &view.revision,
        }
    }

    fn is_default(&self) -> bool {
        match self {
            Self::OpenCode(_) => false,
            Self::Claude(view) => view.default_state.is_default,
            Self::Codex(view) => view.default_state.is_default,
        }
    }

    fn compatibility(&self) -> &'static str {
        match self {
            Self::OpenCode(view) => {
                if view.warnings.is_empty() {
                    "supported"
                } else {
                    "partial"
                }
            }
            Self::Claude(view) => {
                if view.warnings.is_empty() {
                    "supported"
                } else {
                    "partial"
                }
            }
            Self::Codex(view) => {
                if view.warnings.is_empty() {
                    "supported"
                } else {
                    "partial"
                }
            }
        }
    }
}

fn resolve_runtime_target(target_id: &str) -> Result<RuntimeTarget, AgentProfileCommandError> {
    if target_id.trim().is_empty() {
        return Err(AgentProfileCommandError::validation(
            "RUNTIME_TARGET_ID_REQUIRED",
            "runtime_target_id is required",
        ));
    }
    let targets = v3::discover_runtime_targets()?;
    targets
        .into_iter()
        .find(|target| target.target_id == target_id)
        .ok_or_else(|| {
            AgentProfileCommandError::not_found(
                "RUNTIME_TARGET_NOT_OBSERVED",
                "runtime target was not returned by the observed target discovery",
            )
        })
}

fn discover_locations(
    agent: &AgentKind,
    target: &RuntimeTarget,
) -> Result<Vec<LocatedProfile>, AgentProfileCommandError> {
    match agent {
        AgentKind::Opencode => v3::discover_opencode_profiles(target)
            .map(|profiles| profiles.into_iter().map(LocatedProfile::OpenCode).collect())
            .map_err(Into::into),
        AgentKind::ClaudeCode => v3::discover_claude_profiles(target)
            .map(|profiles| profiles.into_iter().map(LocatedProfile::Claude).collect())
            .map_err(Into::into),
        AgentKind::Codex => v3::discover_codex_profiles(target)
            .map(|profiles| profiles.into_iter().map(LocatedProfile::Codex).collect())
            .map_err(Into::into),
    }
}

fn locate_profile(
    target: &RuntimeTarget,
    agent: &AgentKind,
    profile_id: &str,
) -> Result<LocatedProfile, AgentProfileCommandError> {
    if profile_id.trim().is_empty() {
        return Err(AgentProfileCommandError::validation(
            "AGENT_PROFILE_ID_REQUIRED",
            "profile_id is required",
        ));
    }
    discover_locations(agent, target)?
        .into_iter()
        .find(|profile| profile.profile_id() == profile_id)
        .ok_or_else(|| {
            AgentProfileCommandError::not_found(
                "AGENT_PROFILE_NOT_FOUND",
                "profile_id was not found in the observed adapter profile list",
            )
        })
}

fn validate_profile_input(
    agent: &AgentKind,
    target_id: &str,
    profile: &AgentProfileDocumentInput,
) -> Result<(), AgentProfileCommandError> {
    if &profile.agent != agent || profile.runtime_target_id != target_id {
        return Err(AgentProfileCommandError::validation(
            "AGENT_PROFILE_INPUT_IDENTITY_MISMATCH",
            "profile agent and runtime target must match the command",
        ));
    }
    if profile.profile_id.trim().is_empty() || profile.display_name.trim().is_empty() {
        return Err(AgentProfileCommandError::validation(
            "AGENT_PROFILE_INPUT_REQUIRED",
            "profile_id and display_name must not be empty",
        ));
    }
    if profile.revision.revision == 0 || !is_sha256(&profile.revision.content_sha256) {
        return Err(AgentProfileCommandError::validation(
            "AGENT_PROFILE_REVISION_INVALID",
            "profile revision must contain a non-zero number and a SHA-256 content hash",
        ));
    }
    if !Path::new(&profile.source.path.native).is_absolute() {
        return Err(AgentProfileCommandError::validation(
            "AGENT_PROFILE_SOURCE_PATH_NOT_ABSOLUTE",
            "profile source path must be absolute",
        ));
    }
    if profile.revision.observed_at.trim().is_empty()
        || !profile.default_state.is_object()
        || !profile.preservation.is_object()
        || !profile.protocol.is_object()
        || !profile.schema_capability.is_object()
        || !profile.launch.is_object()
    {
        return Err(AgentProfileCommandError::validation(
            "AGENT_PROFILE_INPUT_DOCUMENT_INVALID",
            "profile document metadata must contain the required object fields",
        ));
    }
    if profile
        .source
        .profile_name
        .as_deref()
        .is_some_and(|name| name.trim().is_empty())
    {
        return Err(AgentProfileCommandError::validation(
            "AGENT_PROFILE_SOURCE_PROFILE_NAME_INVALID",
            "profile_name must be non-empty when provided",
        ));
    }
    if profile.source.format == "unknown" || profile.source.scope == "unknown" {
        return Err(AgentProfileCommandError::unsupported(
            "AGENT_PROFILE_INPUT_SCHEMA_UNKNOWN",
            "unknown source format or scope cannot be saved",
        ));
    }
    for provider in &profile.managed.providers {
        validate_provider_input(provider)?;
    }
    Ok(())
}

fn validate_provider_input(
    provider: &ProviderProfileInput,
) -> Result<(), AgentProfileCommandError> {
    if provider.provider_id.trim().is_empty() || provider.display_name.trim().is_empty() {
        return Err(AgentProfileCommandError::validation(
            "AGENT_PROFILE_PROVIDER_REQUIRED",
            "provider id and display name must not be empty",
        ));
    }
    if provider.credential.persisted_in_config {
        return Err(AgentProfileCommandError::validation(
            "AGENT_PROFILE_SECRET_PERSISTENCE_FORBIDDEN",
            "credential references must never be marked as persisted in ordinary config",
        ));
    }
    if !matches!(
        provider.credential.kind.as_str(),
        "env" | "keychain" | "credential_manager" | "secret_store" | "none" | "unknown"
    ) {
        return Err(AgentProfileCommandError::validation(
            "AGENT_PROFILE_CREDENTIAL_KIND_INVALID",
            "credential kind is not in the supported reference-only set",
        ));
    }
    if matches!(provider.credential.kind.as_str(), "env")
        && !is_environment_name(&provider.credential.reference)
    {
        return Err(AgentProfileCommandError::validation(
            "AGENT_PROFILE_CREDENTIAL_REFERENCE_INVALID",
            "environment credential reference must be a variable name",
        ));
    }
    if !matches!(
        provider.credential.secret_state.as_str(),
        "configured" | "missing" | "unavailable" | "unknown"
    ) || provider.credential.display.trim().is_empty()
    {
        return Err(AgentProfileCommandError::validation(
            "AGENT_PROFILE_CREDENTIAL_METADATA_INVALID",
            "credential display and secret state must be non-secret metadata",
        ));
    }
    if !matches!(
        provider.protocol.native_protocol.as_str(),
        "openai_responses" | "openai_chat_completions" | "anthropic_messages" | "unknown"
    ) || !matches!(
        provider.protocol.upstream_protocol.as_str(),
        "openai_responses" | "openai_chat_completions" | "anthropic_messages" | "unknown"
    ) || !matches!(
        provider.protocol.route.as_str(),
        "direct" | "adapter" | "unavailable"
    ) || !matches!(
        provider.protocol.compatibility.as_str(),
        "supported" | "partial" | "unknown" | "unsupported"
    ) {
        return Err(AgentProfileCommandError::validation(
            "AGENT_PROFILE_PROTOCOL_METADATA_INVALID",
            "protocol capability metadata contains an unknown enum value",
        ));
    }
    let _ = (
        &provider.protocol.adapter_id,
        &provider.protocol.adapter_version,
        &provider.protocol.limitations,
    );
    if provider.credential.kind == "config_literal" {
        return Err(AgentProfileCommandError::validation(
            "AGENT_PROFILE_LITERAL_CREDENTIAL_FORBIDDEN",
            "literal credentials are never accepted by the typed command",
        ));
    }
    for model in &provider.models {
        if model.model_id.trim().is_empty() || model.display_name.trim().is_empty() {
            return Err(AgentProfileCommandError::validation(
                "AGENT_PROFILE_MODEL_REQUIRED",
                "model id and display name must not be empty",
            ));
        }
        if model
            .thinking
            .options
            .iter()
            .any(|option| option.trim().is_empty())
        {
            return Err(AgentProfileCommandError::validation(
                "AGENT_PROFILE_THINKING_OPTION_INVALID",
                "thinking options must not be empty",
            ));
        }
        let _ = (
            model.enabled,
            model.thinking.supports_effort,
            model.thinking.custom_allowed,
        );
    }
    Ok(())
}

fn validate_source_and_revision(
    target: &RuntimeTarget,
    located: &LocatedProfile,
    input: &AgentProfileDocumentInput,
    expected_revision: u64,
) -> Result<(), AgentProfileCommandError> {
    if input.source.path.native != located.source_path().to_string_lossy() {
        return Err(AgentProfileCommandError::validation(
            "AGENT_PROFILE_SOURCE_PATH_MISMATCH",
            "source path does not match the adapter-discovered profile",
        ));
    }
    if input.revision.revision != expected_revision {
        return Err(AgentProfileCommandError::conflict(
            "AGENT_PROFILE_EXPECTED_REVISION_MISMATCH",
            "request expected_revision does not match the profile document revision",
        ));
    }
    ensure_revision(located, expected_revision)?;
    let actual_document_revision = located.document_revision();
    if input.revision.content_sha256 != actual_document_revision.content_sha256 {
        let mut error = AgentProfileCommandError::conflict(
            "AGENT_PROFILE_REVISION_HASH_MISMATCH",
            "profile revision hash does not match the observed configuration",
        );
        error.details.insert(
            "expected_content_sha256".to_owned(),
            json!(input.revision.content_sha256),
        );
        error.details.insert(
            "actual_content_sha256".to_owned(),
            json!(actual_document_revision.content_sha256),
        );
        return Err(error);
    }
    let _ = target;
    Ok(())
}

fn ensure_revision(
    profile: &LocatedProfile,
    expected_revision: u64,
) -> Result<(), AgentProfileCommandError> {
    let actual = revision_number(profile.document_revision());
    if actual != expected_revision {
        let mut error = AgentProfileCommandError::conflict(
            "AGENT_PROFILE_REVISION_CONFLICT",
            "configuration changed after it was read; reload before continuing",
        );
        error
            .details
            .insert("expected_revision".to_owned(), json!(expected_revision));
        error
            .details
            .insert("actual_revision".to_owned(), json!(actual));
        return Err(error);
    }
    Ok(())
}

fn save_managed(
    target: &RuntimeTarget,
    profile: &LocatedProfile,
    managed: &ManagedProfileInput,
) -> Result<Option<WriteReport>, AgentProfileCommandError> {
    match profile {
        LocatedProfile::OpenCode(view) => {
            let mut patch = patch_for_opencode(managed)?;
            let active_providers = managed
                .providers
                .iter()
                .map(|provider| provider.provider_id.as_str())
                .collect::<std::collections::BTreeSet<_>>();
            for provider in &view.providers {
                if !active_providers.contains(provider.provider_id.as_str()) {
                    patch.deleted_providers.push(provider.provider_id.clone());
                    continue;
                }
                let active_models = managed
                    .providers
                    .iter()
                    .find(|item| item.provider_id == provider.provider_id)
                    .map(|item| {
                        item.models
                            .iter()
                            .filter(|model| model.enabled)
                            .map(|model| model.model_id.as_str())
                            .collect::<std::collections::BTreeSet<_>>()
                    })
                    .unwrap_or_default();
                for model in &provider.models {
                    if !active_models.contains(model.model_id.as_str()) {
                        patch
                            .deleted_models
                            .entry(provider.provider_id.clone())
                            .or_default()
                            .push(model.model_id.clone());
                    }
                }
            }
            Ok(Some(v3::save_opencode_profile(
                target,
                &view.source_path,
                Some(&view.revision),
                &patch,
            )?))
        }
        LocatedProfile::Claude(view) => {
            let mut patch = patch_for_claude(managed)?;
            if matches!(view.scope, ClaudeSettingsScope::Profile)
                && (patch.base_url.is_some()
                    || patch.auth_token.is_some()
                    || patch.clear_credentials)
            {
                patch.strip_credential_helper = true;
            } else if patch.auth_token.is_some() || patch.clear_credentials {
                patch.strip_credential_helper = true;
            }
            Ok(Some(v3::save_claude_profile(
                target,
                &view.source_path,
                Some(&view.revision),
                &patch,
            )?))
        }
        LocatedProfile::Codex(view) => {
            let mut patch = patch_for_codex(managed)?;
            let active_providers = managed
                .providers
                .iter()
                .map(|provider| provider.provider_id.as_str())
                .collect::<std::collections::BTreeSet<_>>();
            for provider in &view.providers {
                if !active_providers.contains(provider.provider_id.as_str()) {
                    patch.deleted_providers.push(provider.provider_id.clone());
                }
            }
            Ok(Some(v3::save_codex_profile(
                target,
                &view.source_path,
                Some(&view.revision),
                &patch,
            )?))
        }
    }
}

fn patch_for(
    agent: &AgentKind,
    managed: &ManagedProfileInput,
) -> Result<Value, AgentProfileCommandError> {
    match agent {
        AgentKind::Opencode => serde_json::to_value(patch_for_opencode(managed)?),
        AgentKind::ClaudeCode => serde_json::to_value(patch_for_claude(managed)?),
        AgentKind::Codex => serde_json::to_value(patch_for_codex(managed)?),
    }
    .map_err(|error| {
        AgentProfileCommandError::internal(
            "AGENT_PROFILE_PATCH_SERIALIZE_FAILED",
            error.to_string(),
        )
    })
}

fn patch_for_opencode(
    managed: &ManagedProfileInput,
) -> Result<OpenCodeConfigPatch, AgentProfileCommandError> {
    let mut patch = OpenCodeConfigPatch {
        default_model: managed.default_model_id.clone(),
        small_model: managed.small_model_id.clone(),
        default_variant: selected_default_variant(managed),
        deleted_providers: Vec::new(),
        deleted_models: BTreeMap::new(),
        providers: BTreeMap::new(),
    };
    for provider in &managed.providers {
        let secret = credential_secret(&provider.credential);
        let clear_api_key = provider.credential.clear_secret && secret.is_none();
        let environment_references = if secret.is_some() {
            None
        } else {
            credential_environment(&provider.credential)?.map(|reference| vec![reference])
        };
        let mut models = BTreeMap::new();
        for model in &provider.models {
            if !model.enabled {
                continue;
            }
            let variants = if model.thinking.options.is_empty() {
                None
            } else {
                Some(
                    model
                        .thinking
                        .options
                        .iter()
                        .map(|option| (option.clone(), Value::Object(Map::new())))
                        .collect(),
                )
            };
            models.insert(
                model.model_id.clone(),
                OpenCodeModelPatch {
                    display_name: Some(model.display_name.clone()),
                    declared_id: None,
                    reasoning: Some(model.thinking.supports_reasoning),
                    variants,
                },
            );
        }
        patch.providers.insert(
            provider.provider_id.clone(),
            OpenCodeProviderPatch {
                display_name: Some(provider.display_name.clone()),
                base_url: non_empty(provider.base_url.clone()),
                environment_references,
                api_key: secret,
                clear_api_key,
                models,
            },
        );
    }
    Ok(patch)
}

fn patch_for_claude(
    managed: &ManagedProfileInput,
) -> Result<ClaudeSettingsPatch, AgentProfileCommandError> {
    let provider = selected_provider(managed);
    let thinking_enabled = selected_thinking(managed).map(|selected| {
        !matches!(
            selected.to_ascii_lowercase().as_str(),
            "disabled" | "off" | "none"
        )
    });
    let credential = provider.map(|provider| &provider.credential);
    let auth_token = credential.and_then(|credential| credential_secret(credential));
    Ok(ClaudeSettingsPatch {
        model: managed.default_model_id.clone(),
        base_url: provider.and_then(|provider| non_empty(provider.base_url.clone())),
        thinking_enabled,
        clear_model: managed.default_model_id.is_none(),
        clear_base_url: provider
            .map(|provider| provider.base_url.trim().is_empty())
            .unwrap_or(true),
        clear_thinking: selected_thinking(managed).is_none(),
        credential_environment: credential
            .map(|credential| credential_environment(credential))
            .transpose()?
            .flatten(),
        auth_token,
        clear_credentials: credential
            .map(|credential| credential.clear_secret)
            .unwrap_or(false),
        strip_credential_helper: false,
    })
}

fn patch_for_codex(
    managed: &ManagedProfileInput,
) -> Result<CodexConfigPatch, AgentProfileCommandError> {
    let mut providers = BTreeMap::new();
    for provider in &managed.providers {
        let native_protocol = protocol_from_wire(&provider.protocol.native_protocol)?;
        let secret = credential_secret(&provider.credential);
        let environment_key = if secret.is_some() || provider.credential.clear_secret {
            None
        } else {
            credential_environment(&provider.credential)?
        };
        providers.insert(
            provider.provider_id.clone(),
            CodexProviderPatch {
                display_name: Some(provider.display_name.clone()),
                base_url: non_empty(provider.base_url.clone()),
                wire_api: Some(match native_protocol {
                    ProtocolKind::OpenaiResponses => "responses".to_owned(),
                    ProtocolKind::OpenaiChatCompletions => "chat".to_owned(),
                    _ => return Err(AgentProfileCommandError::unsupported(
                        "AGENT_PROFILE_CODEX_PROTOCOL_UNSUPPORTED",
                        "Codex providers must use an OpenAI Responses or Chat Completions wire API",
                    )),
                }),
                environment_key,
                bearer_token: secret.clone(),
                clear_environment_key: secret.is_some() || provider.credential.clear_secret,
                clear_bearer_token: provider.credential.clear_secret && secret.is_none(),
                requires_openai_auth: secret.is_some().then_some(false),
                ..Default::default()
            },
        );
    }
    Ok(CodexConfigPatch {
        model: managed.default_model_id.clone(),
        model_provider: managed.default_provider_id.clone(),
        reasoning_effort: selected_thinking(managed),
        clear_model: managed.default_model_id.is_none(),
        clear_model_provider: managed.default_provider_id.is_none(),
        clear_reasoning_effort: selected_thinking(managed).is_none(),
        deleted_providers: Vec::new(),
        providers,
        ..Default::default()
    })
}

fn credential_secret(credential: &CredentialReferenceInput) -> Option<String> {
    credential
        .secret
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn credential_environment(
    credential: &CredentialReferenceInput,
) -> Result<Option<String>, AgentProfileCommandError> {
    match credential.kind.as_str() {
        "env" => Ok(Some(credential.reference.clone())),
        "none" | "unknown" => Ok(None),
        "keychain" | "credential_manager" | "secret_store" => Err(
            AgentProfileCommandError::unsupported(
                "AGENT_PROFILE_CREDENTIAL_BACKEND_UNAVAILABLE",
                "this adapter currently accepts environment references only; no secret value was read",
            ),
        ),
        _ => Err(AgentProfileCommandError::validation(
            "AGENT_PROFILE_CREDENTIAL_KIND_INVALID",
            "unsupported credential reference kind",
        )),
    }
}

fn selected_provider<'a>(managed: &'a ManagedProfileInput) -> Option<&'a ProviderProfileInput> {
    managed
        .default_provider_id
        .as_deref()
        .and_then(|id| {
            managed
                .providers
                .iter()
                .find(|provider| provider.provider_id == id)
        })
        .or_else(|| managed.providers.first())
}

fn selected_thinking(managed: &ManagedProfileInput) -> Option<String> {
    let provider = selected_provider(managed)?;
    let model_id = managed.default_model_id.as_deref()?;
    provider
        .models
        .iter()
        .find(|model| model.model_id == model_id)
        .and_then(|model| model.thinking.selected.clone())
}

fn selected_default_variant(managed: &ManagedProfileInput) -> Option<String> {
    selected_thinking(managed)
}

fn non_empty(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
}

fn read_result(
    target: &RuntimeTarget,
    profile: &LocatedProfile,
) -> Result<AgentProfileReadResult, AgentProfileCommandError> {
    let document = profile_document(target, profile)?;
    Ok(AgentProfileReadResult {
        kind: "agent_profile_read_result",
        schema_version: SCHEMA_VERSION,
        generated_at: now(),
        model_version: MODEL_VERSION,
        freshness: "fresh",
        completeness: if profile.compatibility() == "supported" {
            "complete"
        } else {
            "partial"
        },
        evidence_refs: Vec::new(),
        warnings: profile_warnings(profile),
        errors: Vec::new(),
        agent: profile.agent(),
        runtime_target: target.clone(),
        source: document["source"].clone(),
        revision: revision_wire(profile.document_revision()),
        managed_fields: document["managed"].clone(),
        default_state: document["default_state"].clone(),
        protocol: document["protocol"].clone(),
        profile: document.clone(),
        schema_capability: document["schema_capability"].clone(),
    })
}

fn save_result(
    operation: &str,
    target: &RuntimeTarget,
    profile: &LocatedProfile,
    write: Option<WriteReport>,
) -> Result<AgentProfileSaveResult, AgentProfileCommandError> {
    let document = profile_document(target, profile)?;
    let backup_path = write.as_ref().and_then(|report| report.backup_path.clone());
    Ok(AgentProfileSaveResult {
        kind: "agent_profile_save_result",
        schema_version: SCHEMA_VERSION,
        generated_at: now(),
        model_version: MODEL_VERSION,
        freshness: "fresh",
        completeness: if profile.compatibility() == "supported" {
            "complete"
        } else {
            "partial"
        },
        evidence_refs: Vec::new(),
        warnings: profile_warnings(profile),
        errors: Vec::new(),
        operation: operation.to_owned(),
        revision: revision_wire(profile.document_revision()),
        profile: document,
        backup_path,
        rollback_available: write
            .as_ref()
            .is_some_and(|report| report.rollback_available),
    })
}

fn summary_for(
    target: &RuntimeTarget,
    profile: &LocatedProfile,
) -> Result<AgentProfileSummaryWire, AgentProfileCommandError> {
    Ok(AgentProfileSummaryWire {
        profile_id: profile.profile_id(),
        display_name: profile.display_name(),
        agent: profile.agent(),
        runtime_target_id: target.target_id.clone(),
        source_path: NativeConfigPath::from_path(profile.source_path(), target.platform),
        revision: revision_wire(profile.document_revision()),
        is_default: profile.is_default(),
        compatibility: profile.compatibility().to_owned(),
    })
}

fn profile_document(
    target: &RuntimeTarget,
    profile: &LocatedProfile,
) -> Result<Value, AgentProfileCommandError> {
    let source_path = NativeConfigPath::from_path(profile.source_path(), target.platform);
    let revision = revision_wire(profile.document_revision());
    let (source, managed, default_state, protocol, schema_capability, launch, preservation) =
        match profile {
            LocatedProfile::OpenCode(view) => opencode_document_parts(target, view),
            LocatedProfile::Claude(view) => claude_document_parts(target, view),
            LocatedProfile::Codex(view) => codex_document_parts(target, view),
        }?;
    Ok(json!({
        "profile_id":profile.profile_id(),
        "display_name":profile.display_name(),
        "agent":profile.agent(),
        "runtime_target_id":target.target_id,
        "source":{
            "path":source_path,
            "format":format_name(profile_format(profile)),
            "scope":source["scope"],
            "profile_name":source["profile_name"]
        },
        "revision":revision,
        "managed":managed,
        "default_state":default_state,
        "preservation":preservation,
        "protocol":protocol,
        "schema_capability":schema_capability,
        "launch":launch
    }))
}

fn opencode_document_parts(
    target: &RuntimeTarget,
    view: &OpenCodeProfileView,
) -> Result<(Value, Value, Value, Value, Value, Value, Value), AgentProfileCommandError> {
    let providers = view
        .providers
        .iter()
        .map(|provider| {
            let native_protocol = infer_opencode_protocol(&provider.provider_id);
            let models = provider
                .models
                .iter()
                .map(|model| {
                    json!({
                        "model_id":model.model_id,
                        "display_name":model.display_name,
                        "enabled":true,
                        "thinking":{
                            "supports_reasoning":model.reasoning.unwrap_or(false),
                            "supports_effort":!model.variants.is_empty(),
                            "selected":view.default_variant,
                            "options":model.variants,
                            "custom_allowed":false
                        }
                    })
                })
                .collect::<Vec<_>>();
            json!({
                "provider_id":provider.provider_id,
                "display_name":provider.display_name,
                "base_url":provider.base_url.clone().unwrap_or_default(),
                "credential":credential_value(&provider.credential),
                "protocol":protocol_value(protocol_resolution(native_protocol, native_protocol)),
                "models":models
            })
        })
        .collect::<Vec<_>>();
    let default_provider_id = view.default_model.as_deref().and_then(|model| {
        model
            .split_once('/')
            .map(|(provider, _)| provider.to_owned())
    });
    let managed = json!({
        "providers":providers,
        "default_provider_id":default_provider_id,
        "default_model_id":view.default_model,
        "small_model_id":view.small_model
    });
    let protocol = view
        .providers
        .first()
        .map(|provider| {
            protocol_value(protocol_resolution(
                infer_opencode_protocol(&provider.provider_id),
                infer_opencode_protocol(&provider.provider_id),
            ))
        })
        .unwrap_or_else(|| {
            protocol_value(protocol_resolution(
                ProtocolKind::Unknown,
                ProtocolKind::Unknown,
            ))
        });
    let source = json!({
        "scope":"user",
        "profile_name":view.source_path.file_name().and_then(|name|name.to_str())
    });
    let default_state = default_state_value(
        false,
        "unknown",
        "native_profile",
        NativeConfigPath::from_path(&view.source_path, target.platform),
        Vec::new(),
        None,
    );
    let schema_capability = schema_value(
        "opencode.config",
        None,
        if view.warnings.is_empty() {
            "supported"
        } else {
            "partial"
        },
        string_vec(&["provider", "model", "small_model", "agent"]),
        Vec::new(),
        view.unknown_root_fields.clone(),
    );
    let launch = json!({
        "executable":"opencode",
        "profile_argument":Value::Null,
        "settings_argument":Value::Null,
        "extra_arguments":[]
    });
    let preservation = preservation_value(
        true,
        view.format == ConfigFormat::Jsonc,
        true,
        string_vec(&["provider", "model", "small_model", "agent"]),
        view.unknown_root_fields.clone(),
        None,
        false,
    );
    Ok((
        source,
        managed,
        default_state,
        protocol,
        schema_capability,
        launch,
        preservation,
    ))
}

fn claude_document_parts(
    target: &RuntimeTarget,
    view: &ClaudeCodeProfileView,
) -> Result<(Value, Value, Value, Value, Value, Value, Value), AgentProfileCommandError> {
    let provider = json!({
        "provider_id":"anthropic",
        "display_name":"Anthropic",
        "base_url":view.base_url.clone().unwrap_or_default(),
        "credential":credential_value(&view.credential),
        "protocol":protocol_value(protocol_resolution(ProtocolKind::AnthropicMessages, ProtocolKind::AnthropicMessages)),
        "models":view.model.as_ref().map(|model|vec![json!({
            "model_id":model,
            "display_name":model,
            "enabled":true,
            "thinking":{
                "supports_reasoning":true,
                "supports_effort":!view.thinking.options.is_empty(),
                "selected":view.thinking.selected,
                "options":view.thinking.options,
                "custom_allowed":false
            }
        })]).unwrap_or_default()
    });
    let managed = json!({
        "providers":[provider],
        "default_provider_id":"anthropic",
        "default_model_id":view.model,
        "small_model_id":Value::Null
    });
    let protocol = protocol_value(protocol_resolution(
        ProtocolKind::AnthropicMessages,
        ProtocolKind::AnthropicMessages,
    ));
    let source = json!({
        "scope":scope_name_claude(view.scope),
        "profile_name":view.display_name
    });
    let projection =
        NativeConfigPath::from_path(&view.default_state.projection_target, target.platform);
    let default_state = default_state_value(
        view.default_state.is_default,
        selector_name_claude(view.default_state.selected_by),
        "settings_projection",
        projection,
        view.managed_fields.clone(),
        Some("Claude 默认 Profile 只投影受管字段，保留 settings 中其他字段"),
    );
    let schema_capability = schema_value(
        "claude-code.settings",
        None,
        if view.warnings.is_empty() {
            "supported"
        } else {
            "partial"
        },
        view.managed_fields.clone(),
        Vec::new(),
        view.unknown_fields.clone(),
    );
    let launch = json!({
        "executable":view.launch.executable,
        "profile_argument":Value::Null,
        "settings_argument":view.launch.settings_argument,
        "extra_arguments":view.launch.arguments.iter().cloned().take(2).collect::<Vec<_>>()
    });
    let preservation = preservation_value(
        true,
        view.format == ConfigFormat::Jsonc,
        view.format != ConfigFormat::Jsonc,
        view.managed_fields.clone(),
        view.preserved_fields
            .iter()
            .chain(view.unknown_fields.iter())
            .cloned()
            .collect(),
        None,
        false,
    );
    Ok((
        source,
        managed,
        default_state,
        protocol,
        schema_capability,
        launch,
        preservation,
    ))
}

fn codex_document_parts(
    target: &RuntimeTarget,
    view: &CodexProfileView,
) -> Result<(Value, Value, Value, Value, Value, Value, Value), AgentProfileCommandError> {
    let providers = view
        .providers
        .iter()
        .map(|provider| {
            let protocol = codex_protocol_kind(provider.protocol);
            json!({
                "provider_id":provider.provider_id,
                "display_name":provider.display_name,
                "base_url":provider.base_url.clone().unwrap_or_default(),
                "credential":credential_value(&provider.credential),
                "protocol":protocol_value(protocol_resolution(protocol, protocol)),
                "models":view.model.as_ref().filter(|_| view.model_provider.as_deref() == Some(provider.provider_id.as_str())).map(|model|vec![json!({
                    "model_id":model,
                    "display_name":model,
                    "enabled":true,
                    "thinking":{
                        "supports_reasoning":view.reasoning_effort.is_some(),
                        "supports_effort":view.reasoning_effort.is_some(),
                        "selected":view.reasoning_effort,
                        "options":["none","minimal","low","medium","high","xhigh","max","ultra"],
                        "custom_allowed":false
                    }
                })]).unwrap_or_default()
            })
        })
        .collect::<Vec<_>>();
    let selected_model = view.model.clone();
    let managed = json!({
        "providers":providers,
        "default_provider_id":view.model_provider,
        "default_model_id":selected_model,
        "small_model_id":Value::Null
    });
    let top_protocol = view
        .providers
        .iter()
        .find(|provider| Some(provider.provider_id.as_str()) == view.model_provider.as_deref())
        .map(|provider| codex_protocol_kind(provider.protocol))
        .unwrap_or(ProtocolKind::Unknown);
    let protocol = protocol_value(protocol_resolution(top_protocol, top_protocol));
    let source = json!({
        "scope":scope_name_codex(view.scope),
        "profile_name":view.display_name
    });
    let projection =
        NativeConfigPath::from_path(&view.default_state.projection_target, target.platform);
    let default_state = default_state_value(
        view.default_state.is_default,
        selector_name_codex(view.default_state.selected_by),
        "base_config_projection",
        projection,
        view.managed_fields.clone(),
        view.project_override_warning.as_deref(),
    );
    let schema_capability = schema_value(
        "codex.config.toml",
        None,
        if view.warnings.is_empty() {
            "supported"
        } else {
            "partial"
        },
        view.managed_fields.clone(),
        Vec::new(),
        view.unknown_root_fields.clone(),
    );
    let launch = view
        .launch
        .as_ref()
        .map(|launch| {
            json!({
                "executable":launch.executable,
                "profile_argument":launch.profile_argument,
                "settings_argument":Value::Null,
                "extra_arguments":launch.arguments.get(2..).unwrap_or_default()
            })
        })
        .unwrap_or_else(|| {
            json!({
                "executable":"codex",
                "profile_argument":"--profile-v2",
                "settings_argument":Value::Null,
                "extra_arguments":[]
            })
        });
    let preservation = preservation_value(
        true,
        false,
        true,
        view.managed_fields.clone(),
        view.unknown_root_fields.clone(),
        None,
        false,
    );
    Ok((
        source,
        managed,
        default_state,
        protocol,
        schema_capability,
        launch,
        preservation,
    ))
}

fn credential_value(credential: &impl CredentialView) -> Value {
    let (kind, reference, secret_state) = credential.contract_parts();
    json!({
        "kind":kind,
        "reference":reference,
        "display":credential.display(),
        "secret_state":secret_state,
        "persisted_in_config":false
    })
}

trait CredentialView {
    fn contract_parts(&self) -> (&'static str, String, &'static str);
    fn display(&self) -> String;
}

impl CredentialView for v3::OpenCodeCredentialReference {
    fn contract_parts(&self) -> (&'static str, String, &'static str) {
        credential_parts_open_code(self.kind.clone(), &self.references)
    }
    fn display(&self) -> String {
        self.display.clone()
    }
}

impl CredentialView for v3::ClaudeCredentialReference {
    fn contract_parts(&self) -> (&'static str, String, &'static str) {
        credential_parts_claude(self.kind, &self.references)
    }
    fn display(&self) -> String {
        self.display.clone()
    }
}

impl CredentialView for v3::CodexCredentialReference {
    fn contract_parts(&self) -> (&'static str, String, &'static str) {
        credential_parts_codex(self.kind, &self.references)
    }
    fn display(&self) -> String {
        self.display.clone()
    }
}

fn credential_parts_open_code(
    kind: v3::OpenCodeCredentialKind,
    references: &[String],
) -> (&'static str, String, &'static str) {
    match kind {
        v3::OpenCodeCredentialKind::Environment => (
            "env",
            references
                .first()
                .cloned()
                .unwrap_or_else(|| "env:unknown".to_owned()),
            "configured",
        ),
        v3::OpenCodeCredentialKind::ConfigLiteral => {
            ("unknown", "credential:configured".to_owned(), "configured")
        }
        v3::OpenCodeCredentialKind::None => ("none", "none".to_owned(), "missing"),
        v3::OpenCodeCredentialKind::Unknown => {
            ("unknown", "credential:unknown".to_owned(), "unknown")
        }
    }
}

fn credential_parts_claude(
    kind: ClaudeCredentialKind,
    references: &[String],
) -> (&'static str, String, &'static str) {
    match kind {
        ClaudeCredentialKind::Environment => (
            "env",
            references
                .first()
                .cloned()
                .unwrap_or_else(|| "env:unknown".to_owned()),
            "configured",
        ),
        ClaudeCredentialKind::CredentialHelper => (
            "secret_store",
            "credential_helper:configured".to_owned(),
            "configured",
        ),
        ClaudeCredentialKind::ConfigLiteral => {
            ("unknown", "credential:configured".to_owned(), "configured")
        }
        ClaudeCredentialKind::None => ("none", "none".to_owned(), "missing"),
        ClaudeCredentialKind::Unknown => ("unknown", "credential:unknown".to_owned(), "unknown"),
    }
}

fn credential_parts_codex(
    kind: CodexCredentialKind,
    references: &[String],
) -> (&'static str, String, &'static str) {
    match kind {
        CodexCredentialKind::Environment => (
            "env",
            references
                .first()
                .cloned()
                .unwrap_or_else(|| "env:unknown".to_owned()),
            "configured",
        ),
        CodexCredentialKind::ConfigLiteral => {
            ("unknown", "credential:configured".to_owned(), "configured")
        }
        CodexCredentialKind::None => ("none", "none".to_owned(), "missing"),
        CodexCredentialKind::Unknown => ("unknown", "credential:unknown".to_owned(), "unknown"),
    }
}

fn protocol_resolution(native: ProtocolKind, upstream: ProtocolKind) -> ProtocolResolution {
    v3::resolve_protocol(native, upstream, &Default::default())
}

fn protocol_value(resolution: ProtocolResolution) -> Value {
    serde_json::to_value(resolution).unwrap_or_else(|_| {
        json!({
            "native_protocol":"unknown",
            "upstream_protocol":"unknown",
            "route":"unavailable",
            "compatibility":"unknown",
            "adapter_id":Value::Null,
            "adapter_version":Value::Null,
            "limitations":["protocol serialization failed"]
        })
    })
}

fn schema_value(
    schema_id: &str,
    schema_version: Option<&str>,
    compatibility: &str,
    supported_fields: Vec<String>,
    unsupported_fields: Vec<String>,
    unknown_fields: Vec<String>,
) -> Value {
    json!({
        "schema_id":schema_id,
        "schema_version":schema_version,
        "compatibility":compatibility,
        "supported_fields":supported_fields,
        "unsupported_fields":unsupported_fields,
        "unknown_fields":unknown_fields
    })
}

fn string_vec(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn default_state_value(
    is_default: bool,
    selected_by: &str,
    strategy: &str,
    target_path: NativeConfigPath,
    managed_field_paths: Vec<String>,
    warning: Option<&str>,
) -> Value {
    json!({
        "is_default":is_default,
        "selected_by":selected_by,
        "projection":{
            "strategy":strategy,
            "target_path":target_path,
            "managed_field_paths":managed_field_paths,
            "warning":warning
        }
    })
}

fn preservation_value(
    unknown_fields_preserved: bool,
    comments_preserved: bool,
    formatting_preserved: bool,
    managed_field_paths: Vec<String>,
    unmanaged_field_paths: Vec<String>,
    backup_path: Option<NativeConfigPath>,
    rollback_available: bool,
) -> Value {
    json!({
        "unknown_fields_preserved":unknown_fields_preserved,
        "comments_preserved":comments_preserved,
        "formatting_preserved":formatting_preserved,
        "managed_field_paths":managed_field_paths,
        "unmanaged_field_paths":unmanaged_field_paths,
        "backup_path":backup_path,
        "rollback_available":rollback_available
    })
}

fn profile_warnings(profile: &LocatedProfile) -> Vec<Value> {
    let warnings: Vec<String> = match profile {
        LocatedProfile::OpenCode(view) => view.warnings.clone(),
        LocatedProfile::Claude(view) => view.warnings.clone(),
        LocatedProfile::Codex(view) => view.warnings.clone(),
    };
    warnings
        .into_iter()
        .map(|code| {
            json!({
                "code":code,
                "severity":"warning",
                "message_key":code,
                "evidence_refs":[]
            })
        })
        .collect()
}

fn profile_format(profile: &LocatedProfile) -> ConfigFormat {
    match profile {
        LocatedProfile::OpenCode(view) => view.format,
        LocatedProfile::Claude(view) => view.format,
        LocatedProfile::Codex(view) => view.format,
    }
}

fn revision_wire(revision: &DocumentRevision) -> ConfigRevisionWire {
    ConfigRevisionWire {
        revision: revision_number(revision),
        content_sha256: revision.content_sha256.clone(),
        observed_at: now(),
    }
}

fn revision_number(revision: &DocumentRevision) -> u64 {
    u64::from_str_radix(revision.content_sha256.get(..12).unwrap_or("0"), 16)
        .unwrap_or(1)
        .max(1)
}

fn validate_backup_path(
    target: &RuntimeTarget,
    current_path: &Path,
    backup_path: &str,
) -> Result<PathBuf, AgentProfileCommandError> {
    let backup = PathBuf::from(backup_path);
    if !backup.is_absolute() {
        return Err(AgentProfileCommandError::validation(
            "AGENT_PROFILE_BACKUP_PATH_NOT_ABSOLUTE",
            "backup path must be absolute",
        ));
    }
    if backup.parent() != current_path.parent() {
        return Err(AgentProfileCommandError::validation(
            "AGENT_PROFILE_BACKUP_PATH_SCOPE_INVALID",
            "backup must be in the same directory as the selected profile",
        ));
    }
    let current_name = current_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            AgentProfileCommandError::validation(
                "AGENT_PROFILE_PATH_INVALID",
                "profile filename is invalid",
            )
        })?;
    let file_name = backup
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            AgentProfileCommandError::validation(
                "AGENT_PROFILE_BACKUP_PATH_INVALID",
                "backup filename is invalid",
            )
        })?;
    if !file_name.starts_with(&format!(".{current_name}.vibehub.")) || !file_name.ends_with(".bak")
    {
        return Err(AgentProfileCommandError::validation(
            "AGENT_PROFILE_BACKUP_PATH_INVALID",
            "only VibeHub backups for the selected profile can be restored",
        ));
    }
    let _ = v3::read_document(target, current_path)?;
    Ok(backup)
}

fn format_name(format: ConfigFormat) -> &'static str {
    match format {
        ConfigFormat::Json => "json",
        ConfigFormat::Jsonc => "jsonc",
        ConfigFormat::Toml => "toml",
    }
}

fn scope_name_claude(scope: ClaudeSettingsScope) -> &'static str {
    match scope {
        ClaudeSettingsScope::User => "user",
        ClaudeSettingsScope::Project => "project",
        ClaudeSettingsScope::Local => "unknown",
        ClaudeSettingsScope::Profile => "profile",
        ClaudeSettingsScope::Unknown => "unknown",
    }
}

fn scope_name_codex(scope: v3::CodexSettingsScope) -> &'static str {
    match scope {
        v3::CodexSettingsScope::User => "user",
        v3::CodexSettingsScope::Profile => "profile",
        v3::CodexSettingsScope::Project => "project",
        v3::CodexSettingsScope::Unknown => "unknown",
    }
}

fn selector_name_claude(selector: v3::ClaudeDefaultSelector) -> &'static str {
    match selector {
        v3::ClaudeDefaultSelector::Vibehub => "vibehub",
        v3::ClaudeDefaultSelector::Native => "native",
        v3::ClaudeDefaultSelector::Unknown => "unknown",
    }
}

fn selector_name_codex(selector: v3::CodexDefaultSelector) -> &'static str {
    match selector {
        v3::CodexDefaultSelector::Vibehub => "vibehub",
        v3::CodexDefaultSelector::Native => "native",
        v3::CodexDefaultSelector::Unknown => "unknown",
    }
}

fn infer_opencode_protocol(provider_id: &str) -> ProtocolKind {
    // Provider protocol is not exposed by the adapter's observed schema. Do
    // not guess from a brand/name; unknown remains unavailable until an
    // actual upstream capability probe supplies evidence.
    let _ = provider_id;
    ProtocolKind::Unknown
}

fn codex_protocol_kind(protocol: CodexProtocol) -> ProtocolKind {
    match protocol {
        CodexProtocol::OpenaiResponses => ProtocolKind::OpenaiResponses,
        CodexProtocol::OpenaiChatCompletions => ProtocolKind::OpenaiChatCompletions,
        CodexProtocol::Unknown => ProtocolKind::Unknown,
    }
}

fn protocol_from_wire(value: &str) -> Result<ProtocolKind, AgentProfileCommandError> {
    match value {
        "openai_responses" | "responses" => Ok(ProtocolKind::OpenaiResponses),
        "openai_chat_completions" | "chat" | "chat_completions" => {
            Ok(ProtocolKind::OpenaiChatCompletions)
        }
        "anthropic_messages" => Ok(ProtocolKind::AnthropicMessages),
        _ => Err(AgentProfileCommandError::unsupported(
            "AGENT_PROFILE_PROTOCOL_UNKNOWN",
            "unknown protocol cannot be persisted or routed implicitly",
        )),
    }
}

fn opencode_profile_id(path: &Path) -> String {
    let mut digest = Sha256::new();
    digest.update(path.to_string_lossy().as_bytes());
    let hex = format!("{:x}", digest.finalize());
    format!("opencode.profile.{}", &hex[..16])
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|character| character.is_ascii_hexdigit())
}

fn is_environment_name(value: &str) -> bool {
    !value.is_empty()
        && value.chars().enumerate().all(|(index, character)| {
            character.is_ascii_alphabetic()
                || character == '_'
                || (index > 0 && character.is_ascii_digit())
        })
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

const UPSTREAM_MODEL_LIMIT: usize = 200;
const UPSTREAM_MODELS_MAX_BYTES: usize = 2_000_000;

async fn list_upstream_models(
    request: AgentProfileListModelsRequest,
) -> Result<AgentProfileListModelsResult, AgentProfileCommandError> {
    let base_url = request.base_url.trim();
    if base_url.is_empty() {
        return Err(AgentProfileCommandError::validation(
            "AGENT_PROFILE_BASE_URL_REQUIRED",
            "Base URL is required to list upstream models",
        ));
    }
    let provided = request
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let token = match provided {
        Some(value) => value,
        None => {
            let target = resolve_runtime_target(&request.runtime_target_id)?;
            let profile = locate_profile(&target, &request.agent, &request.profile_id)?;
            stored_provider_secret(&target, &profile, &request.provider_id)?.ok_or_else(|| {
                AgentProfileCommandError::validation(
                    "AGENT_PROFILE_API_KEY_REQUIRED",
                    "API Key is required to list upstream models",
                )
            })?
        }
    };
    let anthropic = uses_anthropic_models_auth(&request.agent, &request.protocol);
    let (endpoint, models) = fetch_upstream_models(base_url, &token, anthropic).await?;
    Ok(AgentProfileListModelsResult { endpoint, models })
}

fn uses_anthropic_models_auth(agent: &AgentKind, protocol: &str) -> bool {
    protocol == "anthropic_messages"
        || (matches!(agent, AgentKind::ClaudeCode)
            && protocol != "openai_responses"
            && protocol != "openai_chat_completions")
}

fn models_url_candidates(base_url: &str) -> Result<Vec<String>, AgentProfileCommandError> {
    let trimmed = base_url.trim().trim_end_matches('/');
    let parsed = reqwest::Url::parse(trimmed).map_err(|_| {
        AgentProfileCommandError::validation(
            "AGENT_PROFILE_BASE_URL_INVALID",
            "Base URL must be an absolute http(s) URL",
        )
    })?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(AgentProfileCommandError::validation(
            "AGENT_PROFILE_BASE_URL_INVALID",
            "Base URL must be an absolute http(s) URL",
        ));
    }
    if trimmed.ends_with("/models") {
        return Ok(vec![trimmed.to_owned()]);
    }
    let mut urls = vec![format!("{trimmed}/models")];
    if !trimmed.ends_with("/v1") {
        urls.push(format!("{trimmed}/v1/models"));
    }
    Ok(urls)
}

fn parse_upstream_models(value: &Value) -> Vec<UpstreamModelWire> {
    let items = value
        .get("data")
        .or_else(|| value.get("models"))
        .unwrap_or(value);
    let Some(array) = items.as_array() else {
        return Vec::new();
    };
    let mut seen = BTreeSet::new();
    let mut models = Vec::new();
    for item in array {
        let Some((model_id, display_name)) = upstream_model_parts(item) else {
            continue;
        };
        if !seen.insert(model_id.clone()) {
            continue;
        }
        models.push(UpstreamModelWire {
            model_id,
            display_name,
        });
    }
    models.sort_by(|left, right| left.model_id.cmp(&right.model_id));
    models.truncate(UPSTREAM_MODEL_LIMIT);
    models
}

fn upstream_model_parts(item: &Value) -> Option<(String, String)> {
    match item {
        Value::String(value) => {
            let model_id = value.trim();
            if model_id.is_empty() {
                return None;
            }
            Some((model_id.to_owned(), model_id.to_owned()))
        }
        Value::Object(object) => {
            let model_id = object
                .get("id")
                .or_else(|| object.get("model"))
                .or_else(|| object.get("name"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())?;
            let display_name = object
                .get("display_name")
                .or_else(|| object.get("displayName"))
                .or_else(|| object.get("name"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(model_id);
            Some((model_id.to_owned(), display_name.to_owned()))
        }
        _ => None,
    }
}

fn stored_provider_secret(
    target: &RuntimeTarget,
    profile: &LocatedProfile,
    provider_id: &str,
) -> Result<Option<String>, AgentProfileCommandError> {
    let document = v3::read_document(target, profile.source_path())?;
    let parsed = document.parse()?;
    let secret = match parsed {
        ParsedConfig::Json(value) => match profile {
            LocatedProfile::Claude(_) => claude_secret_from_json(&value),
            LocatedProfile::OpenCode(_) => opencode_secret_from_json(&value, provider_id),
            LocatedProfile::Codex(_) => None,
        },
        ParsedConfig::Toml(value) => match profile {
            LocatedProfile::Codex(_) => {
                let json = serde_json::to_value(&value).unwrap_or(Value::Null);
                codex_secret_from_value(&json, provider_id)
            }
            _ => None,
        },
    };
    Ok(secret.and_then(usable_secret))
}

fn claude_secret_from_json(root: &Value) -> Option<String> {
    if let Some(secret) = root
        .get("env")
        .and_then(Value::as_object)
        .and_then(|env| {
            env.get("ANTHROPIC_AUTH_TOKEN")
                .or_else(|| env.get("ANTHROPIC_API_KEY"))
        })
        .and_then(Value::as_str)
        .map(str::to_owned)
        .and_then(usable_secret)
    {
        return Some(secret);
    }
    for key in ["ANTHROPIC_AUTH_TOKEN", "ANTHROPIC_API_KEY"] {
        if let Ok(value) = std::env::var(key) {
            if let Some(secret) = usable_secret(value) {
                return Some(secret);
            }
        }
    }
    None
}

fn opencode_secret_from_json(root: &Value, provider_id: &str) -> Option<String> {
    let providers = root
        .get("provider")
        .or_else(|| root.get("providers"))?
        .as_object()?;
    let provider = providers.get(provider_id)?;
    if let Some(secret) = provider
        .get("options")
        .and_then(|options| options.get("apiKey"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .and_then(usable_secret)
    {
        return Some(secret);
    }
    let env_names = provider.get("env")?.as_array()?;
    for name in env_names {
        let Some(key) = name.as_str() else {
            continue;
        };
        if let Ok(value) = std::env::var(key) {
            if let Some(secret) = usable_secret(value) {
                return Some(secret);
            }
        }
    }
    None
}

fn codex_secret_from_value(root: &Value, provider_id: &str) -> Option<String> {
    let provider = root.get("model_providers")?.get(provider_id)?;
    for key in ["experimental_bearer_token", "api_key", "apiKey"] {
        if let Some(secret) = provider
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .and_then(usable_secret)
        {
            return Some(secret);
        }
    }
    let env_key = provider.get("env_key").and_then(Value::as_str)?;
    std::env::var(env_key).ok().and_then(usable_secret)
}

fn usable_secret(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.contains("{env:")
        || trimmed.starts_with("{file:")
        || trimmed.starts_with('$')
    {
        return None;
    }
    Some(trimmed.to_owned())
}

async fn fetch_upstream_models(
    base_url: &str,
    token: &str,
    anthropic: bool,
) -> Result<(String, Vec<UpstreamModelWire>), AgentProfileCommandError> {
    let urls = models_url_candidates(base_url)?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()
        .map_err(|error| {
            AgentProfileCommandError::internal(
                "AGENT_PROFILE_HTTP_CLIENT_FAILED",
                safe_storage_message(&error.to_string()),
            )
        })?;
    let mut last_error: Option<AgentProfileCommandError> = None;
    let mut empty_endpoint: Option<String> = None;
    for url in urls {
        match probe_models_url(&client, &url, token, anthropic).await {
            Ok(models) if models.is_empty() => {
                empty_endpoint = Some(url);
            }
            Ok(models) => return Ok((url, models)),
            Err(error) => {
                if error.code == "AGENT_PROFILE_UPSTREAM_AUTH_FAILED" {
                    return Err(error);
                }
                last_error = Some(error);
            }
        }
    }
    if let Some(endpoint) = empty_endpoint {
        return Ok((endpoint, Vec::new()));
    }
    Err(last_error.unwrap_or_else(|| {
        AgentProfileCommandError::validation(
            "AGENT_PROFILE_UPSTREAM_UNAVAILABLE",
            "upstream did not return a model list",
        )
    }))
}

async fn probe_models_url(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    anthropic: bool,
) -> Result<Vec<UpstreamModelWire>, AgentProfileCommandError> {
    let mut request = client
        .get(url)
        .header("User-Agent", "VibeHub-AgentProfiles")
        .header("Accept", "application/json")
        .bearer_auth(token);
    if anthropic {
        request = request
            .header("x-api-key", token)
            .header("anthropic-version", "2023-06-01");
    }
    let response = request.send().await.map_err(|error| {
        AgentProfileCommandError::internal(
            "AGENT_PROFILE_UPSTREAM_REQUEST_FAILED",
            safe_storage_message(&error.to_string()),
        )
    })?;
    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(AgentProfileCommandError::validation(
            "AGENT_PROFILE_UPSTREAM_AUTH_FAILED",
            format!("upstream rejected the API Key ({status})"),
        ));
    }
    if !status.is_success() {
        return Err(AgentProfileCommandError::validation(
            "AGENT_PROFILE_UPSTREAM_UNAVAILABLE",
            format!("upstream model list failed ({status})"),
        ));
    }
    let bytes = response.bytes().await.map_err(|error| {
        AgentProfileCommandError::internal(
            "AGENT_PROFILE_UPSTREAM_READ_FAILED",
            safe_storage_message(&error.to_string()),
        )
    })?;
    if bytes.len() > UPSTREAM_MODELS_MAX_BYTES {
        return Err(AgentProfileCommandError::validation(
            "AGENT_PROFILE_UPSTREAM_RESPONSE_TOO_LARGE",
            "upstream model list exceeded the size limit",
        ));
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| {
        AgentProfileCommandError::validation(
            "AGENT_PROFILE_UPSTREAM_RESPONSE_INVALID",
            "upstream did not return a JSON model list",
        )
    })?;
    Ok(parse_upstream_models(&value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use uuid::Uuid;

    fn opencode_config_dir(root: &Path) -> PathBuf {
        if cfg!(windows) {
            root.join("AppData").join("Roaming").join("opencode")
        } else {
            root.join(".config").join("opencode")
        }
    }

    #[test]
    fn secret_like_error_text_is_redacted_without_touching_paths() {
        let message = safe_storage_message("path /tmp/config api_key=sk-secret-value");
        assert_eq!(message, "path /tmp/config [redacted]");
    }

    #[test]
    fn credential_input_rejects_literal_and_accepts_environment_reference() {
        let mut provider = ProviderProfileInput {
            provider_id: "deepseek".to_owned(),
            display_name: "DeepSeek".to_owned(),
            base_url: "https://api.deepseek.com/v1".to_owned(),
            credential: CredentialReferenceInput {
                kind: "config_literal".to_owned(),
                reference: "credential:configured".to_owned(),
                display: "隐藏".to_owned(),
                secret_state: "configured".to_owned(),
                persisted_in_config: false,
                secret: None,
                clear_secret: false,
            },
            protocol: ProtocolCapabilityInput {
                native_protocol: "openai_chat_completions".to_owned(),
                upstream_protocol: "openai_chat_completions".to_owned(),
                route: "direct".to_owned(),
                compatibility: "supported".to_owned(),
                adapter_id: None,
                adapter_version: None,
                limitations: Vec::new(),
            },
            models: Vec::new(),
        };
        assert!(validate_provider_input(&provider).is_err());
        provider.credential.kind = "env".to_owned();
        provider.credential.reference = "DEEPSEEK_API_KEY".to_owned();
        assert!(validate_provider_input(&provider).is_ok());
    }

    #[test]
    fn profile_ids_for_opencode_are_stable_and_path_bound() {
        let a = opencode_profile_id(Path::new("/home/user/.config/opencode/opencode.jsonc"));
        let b = opencode_profile_id(Path::new("/home/user/.config/opencode/opencode.jsonc"));
        let c = opencode_profile_id(Path::new("/home/user/.config/opencode/opencode.json"));
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a.starts_with("opencode.profile."));
    }

    #[test]
    fn revision_number_fits_within_javascript_max_safe_integer() {
        const JS_MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
        let max_revision = DocumentRevision {
            content_sha256: "f".repeat(64),
            byte_length: 1024,
        };
        let min_revision = DocumentRevision {
            content_sha256: "0".repeat(64),
            byte_length: 1024,
        };
        let sample_revision = DocumentRevision {
            content_sha256: "f4a9b2c3d4e5f6071234567890abcdef1234567890abcdef1234567890abcdef"
                .to_owned(),
            byte_length: 1024,
        };
        assert!(revision_number(&max_revision) <= JS_MAX_SAFE_INTEGER);
        assert!(revision_number(&max_revision) >= 1);
        assert_eq!(revision_number(&min_revision), 1);
        assert!(revision_number(&sample_revision) <= JS_MAX_SAFE_INTEGER);
        assert_eq!(revision_number(&max_revision), 0xFFFFFFFFFFFF);
    }

    #[test]
    fn typed_profile_flow_discovers_reads_and_saves_all_three_adapters() {
        let root =
            std::env::temp_dir().join(format!("vibehub-agent-profile-command-{}", Uuid::new_v4()));
        fs::create_dir_all(opencode_config_dir(&root)).unwrap();
        fs::create_dir_all(root.join(".claude")).unwrap();
        fs::create_dir_all(root.join(".codex")).unwrap();
        fs::write(
            opencode_config_dir(&root).join("opencode.jsonc"),
            r#"{
  "$schema": "https://opencode.ai/config.json",
  "model": "openai/gpt-5",
  "small_model": "openai/gpt-5-mini",
  "provider": {
    "openai": {
      "name": "OpenAI",
      "env": ["OPENAI_API_KEY"],
      "options": {"baseURL": "https://api.openai.com/v1"},
      "models": {
        "gpt-5": {"name": "GPT-5", "reasoning": true, "variants": {"high": {}}}
      }
    }
  }
}"#,
        )
        .unwrap();
        fs::write(
            root.join(".claude/settings.json"),
            r#"{"model":"claude-sonnet","alwaysThinkingEnabled":true,"env":{"ANTHROPIC_BASE_URL":"https://api.anthropic.com"},"permissions":{"allow":[]}}"#,
        )
        .unwrap();
        fs::write(
            root.join(".codex/config.toml"),
            "model = \"gpt-5\"\nmodel_provider = \"openai\"\nmodel_reasoning_effort = \"high\"\n\n[model_providers.openai]\nname = \"OpenAI\"\nbase_url = \"https://api.openai.com/v1\"\nwire_api = \"responses\"\nenv_key = \"OPENAI_API_KEY\"\n",
        )
        .unwrap();
        let target = RuntimeTarget::host(root.clone());

        for agent in [AgentKind::Opencode, AgentKind::ClaudeCode, AgentKind::Codex] {
            let locations = discover_locations(&agent, &target).unwrap();
            assert_eq!(
                locations.len(),
                1,
                "expected one discovered profile for {agent:?}"
            );
            let profile = locations.into_iter().next().unwrap();
            let document = profile_document(&target, &profile).unwrap();
            assert_eq!(document["runtime_target_id"], target.target_id);
            assert!(document["managed"]["providers"].is_array());
            let input: AgentProfileDocumentInput = serde_json::from_value(document).unwrap();
            let profile_id = input.profile_id.clone();
            let expected_revision = input.revision.revision;
            let mut tampered = input.clone();
            tampered.revision.content_sha256 = "0".repeat(64);
            let hash_error = save_on_target(
                target.clone(),
                AgentProfileSaveRequest {
                    agent: agent.clone(),
                    runtime_target_id: target.target_id.clone(),
                    profile_id: profile_id.clone(),
                    expected_revision,
                    profile: tampered,
                },
            )
            .unwrap_err();
            assert_eq!(hash_error.code, "AGENT_PROFILE_REVISION_HASH_MISMATCH");

            let result = save_on_target(
                target.clone(),
                AgentProfileSaveRequest {
                    agent: agent.clone(),
                    runtime_target_id: target.target_id.clone(),
                    profile_id: profile_id.clone(),
                    expected_revision,
                    profile: input,
                },
            )
            .unwrap();
            assert_eq!(result.operation, "save");
            assert_eq!(result.profile["profile_id"], profile_id);
            assert_eq!(result.errors, Vec::<Value>::new());
        }

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn typed_profile_flow_covers_conflict_recovery_default_guard_and_launch_args() {
        let root = std::env::temp_dir().join(format!(
            "vibehub-agent-profile-integration-{}",
            Uuid::new_v4()
        ));
        fs::create_dir_all(opencode_config_dir(&root)).unwrap();
        fs::create_dir_all(root.join(".claude")).unwrap();
        fs::create_dir_all(root.join(".codex")).unwrap();
        fs::write(
            opencode_config_dir(&root).join("opencode.json"),
            r#"{"$schema":"https://opencode.ai/config.json","model":"openai/gpt-5","provider":{"openai":{"env":["OPENAI_API_KEY"],"options":{"baseURL":"https://api.openai.com/v1"},"models":{"gpt-5":{"reasoning":true}}}}}"#,
        )
        .unwrap();
        fs::write(
            root.join(".claude/settings.json"),
            r#"{"model":"claude-sonnet","alwaysThinkingEnabled":true,"permissions":{"allow":[]}}"#,
        )
        .unwrap();
        fs::write(
            root.join(".codex/config.toml"),
            "model = \"gpt-5\"\nmodel_provider = \"openai\"\nmodel_reasoning_effort = \"high\"\n\n[model_providers.openai]\nname = \"OpenAI\"\nbase_url = \"https://api.openai.com/v1\"\nwire_api = \"responses\"\nenv_key = \"OPENAI_API_KEY\"\n",
        )
        .unwrap();
        let target = RuntimeTarget::host(root.clone());

        // The command boundary must reject an external edit instead of
        // overwriting it, then allow an explicit backup restore.
        let locations = discover_locations(&AgentKind::Opencode, &target).unwrap();
        let opencode = locations.into_iter().next().unwrap();
        let opencode_document = profile_document(&target, &opencode).unwrap();
        let opencode_input: AgentProfileDocumentInput =
            serde_json::from_value(opencode_document.clone()).unwrap();
        let opencode_path = opencode.source_path().to_path_buf();
        let original = fs::read(&opencode_path).unwrap();
        let save = save_on_target(
            target.clone(),
            AgentProfileSaveRequest {
                agent: AgentKind::Opencode,
                runtime_target_id: target.target_id.clone(),
                profile_id: opencode_input.profile_id.clone(),
                expected_revision: opencode_input.revision.revision,
                profile: opencode_input.clone(),
            },
        )
        .unwrap();
        let current = fs::read(&opencode_path).unwrap();
        fs::write(
            &opencode_path,
            format!("{}\n", String::from_utf8_lossy(&current)),
        )
        .unwrap();
        let stale_error = save_on_target(
            target.clone(),
            AgentProfileSaveRequest {
                agent: AgentKind::Opencode,
                runtime_target_id: target.target_id.clone(),
                profile_id: opencode_input.profile_id.clone(),
                expected_revision: opencode_input.revision.revision,
                profile: opencode_input,
            },
        )
        .unwrap_err();
        assert_eq!(stale_error.code, "AGENT_PROFILE_REVISION_CONFLICT");
        fs::write(&opencode_path, &original).unwrap();
        let current_document = profile_document(&target, &opencode).unwrap();
        let current_input: AgentProfileDocumentInput =
            serde_json::from_value(current_document).unwrap();
        let backup_path = save
            .backup_path
            .expect("save should create a backup")
            .native;
        let restored = restore_on_target(
            target.clone(),
            AgentProfileRestoreRequest {
                agent: AgentKind::Opencode,
                runtime_target_id: target.target_id.clone(),
                profile_id: current_input.profile_id.clone(),
                expected_revision: current_input.revision.revision,
                backup_path,
            },
        )
        .unwrap();
        assert_eq!(restored.operation, "restore");

        // Claude Profile CRUD is guarded by the default replacement rule and
        // its temporary launch arguments are produced by the adapter.
        let claude_alpha = create_profile_on_target(
            target.clone(),
            AgentProfileCreateRequest {
                agent: AgentKind::ClaudeCode,
                runtime_target_id: target.target_id.clone(),
                profile_name: "alpha".to_owned(),
                template_profile_id: None,
            },
        )
        .unwrap();
        let alpha_id = claude_alpha.profile["profile_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let claude_beta = clone_profile_on_target(
            target.clone(),
            AgentProfileCloneRequest {
                agent: AgentKind::ClaudeCode,
                runtime_target_id: target.target_id.clone(),
                source_profile_id: alpha_id.clone(),
                profile_name: "beta".to_owned(),
            },
        )
        .unwrap();
        let beta_id = claude_beta.profile["profile_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let activated = activate_on_target(
            target.clone(),
            AgentProfileActivateRequest {
                agent: AgentKind::ClaudeCode,
                runtime_target_id: target.target_id.clone(),
                profile_id: alpha_id.clone(),
                expected_revision: claude_alpha.profile["revision"]["revision"]
                    .as_u64()
                    .unwrap(),
            },
        )
        .unwrap();
        assert_eq!(activated.operation, "activate");
        let protected = delete_profile_on_target(
            target.clone(),
            AgentProfileDeleteRequest {
                agent: AgentKind::ClaudeCode,
                runtime_target_id: target.target_id.clone(),
                profile_id: alpha_id.clone(),
                replacement_profile_id: None,
            },
        )
        .unwrap_err();
        assert_eq!(
            protected.code,
            "CLAUDE_DEFAULT_PROFILE_REPLACEMENT_REQUIRED"
        );
        let deleted = delete_profile_on_target(
            target.clone(),
            AgentProfileDeleteRequest {
                agent: AgentKind::ClaudeCode,
                runtime_target_id: target.target_id.clone(),
                profile_id: alpha_id,
                replacement_profile_id: Some(beta_id.clone()),
            },
        )
        .unwrap();
        assert_eq!(deleted.operation, "delete");
        let beta = locate_profile(&target, &AgentKind::ClaudeCode, &beta_id).unwrap();
        let beta_args = launch_arguments_for(&beta, "temporary").unwrap();
        assert_eq!(
            beta_args.first().map(String::as_str),
            Some("--setting-sources")
        );
        assert_eq!(beta_args.get(1).map(String::as_str), Some(""));
        assert_eq!(beta_args.get(2).map(String::as_str), Some("--settings"));
        assert!(launch_arguments_for(&beta, "default").is_ok());

        // Codex uses the native profile-v2 argument and the same default guard
        // without reintroducing the removed [profiles.*] table.
        let codex_alpha = create_profile_on_target(
            target.clone(),
            AgentProfileCreateRequest {
                agent: AgentKind::Codex,
                runtime_target_id: target.target_id.clone(),
                profile_name: "alpha".to_owned(),
                template_profile_id: None,
            },
        )
        .unwrap();
        let codex_id = codex_alpha.profile["profile_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let codex_profile = locate_profile(&target, &AgentKind::Codex, &codex_id).unwrap();
        let codex_args = launch_arguments_for(&codex_profile, "temporary").unwrap();
        assert_eq!(codex_args.first().map(String::as_str), Some("--profile-v2"));
        let codex_default_error = launch_arguments_for(&codex_profile, "default").unwrap_err();
        assert_eq!(
            codex_default_error.code,
            "AGENT_PROFILE_DEFAULT_LAUNCH_REQUIRES_DEFAULT"
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn save_writes_opencode_and_codex_literal_keys_without_exposing_them_in_views() {
        let root = std::env::temp_dir().join(format!(
            "vibehub-agent-profile-literal-key-{}",
            Uuid::new_v4()
        ));
        fs::create_dir_all(opencode_config_dir(&root)).unwrap();
        fs::create_dir_all(root.join(".codex")).unwrap();
        fs::write(
            opencode_config_dir(&root).join("opencode.json"),
            r#"{"$schema":"https://opencode.ai/config.json","model":"openai/gpt-5","provider":{"openai":{"env":["OPENAI_API_KEY"],"options":{"baseURL":"https://api.openai.com/v1"},"models":{"gpt-5":{"reasoning":true}}}}}"#,
        )
        .unwrap();
        fs::write(
            root.join(".codex/config.toml"),
            "model = \"gpt-5\"\nmodel_provider = \"openai\"\nmodel_reasoning_effort = \"high\"\n\n[model_providers.openai]\nname = \"OpenAI\"\nbase_url = \"https://api.openai.com/v1\"\nwire_api = \"responses\"\nenv_key = \"OPENAI_API_KEY\"\n",
        )
        .unwrap();
        let target = RuntimeTarget::host(root.clone());

        let opencode = discover_locations(&AgentKind::Opencode, &target)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        let mut opencode_input: AgentProfileDocumentInput =
            serde_json::from_value(profile_document(&target, &opencode).unwrap()).unwrap();
        opencode_input.managed.providers[0].credential.kind = "unknown".to_owned();
        opencode_input.managed.providers[0].credential.reference =
            "credential:configured".to_owned();
        opencode_input.managed.providers[0].credential.secret =
            Some("opencode-command-secret".to_owned());
        let opencode_path = opencode.source_path().to_path_buf();
        let saved = save_on_target(
            target.clone(),
            AgentProfileSaveRequest {
                agent: AgentKind::Opencode,
                runtime_target_id: target.target_id.clone(),
                profile_id: opencode_input.profile_id.clone(),
                expected_revision: opencode_input.revision.revision,
                profile: opencode_input,
            },
        )
        .unwrap();
        let opencode_raw = fs::read_to_string(&opencode_path).unwrap();
        assert!(
            opencode_raw.contains("\"apiKey\": \"opencode-command-secret\"")
                || opencode_raw.contains("\"apiKey\":\"opencode-command-secret\"")
        );
        assert!(!opencode_raw.contains("OPENAI_API_KEY"));
        assert!(!saved
            .profile
            .to_string()
            .contains("opencode-command-secret"));

        let codex = discover_locations(&AgentKind::Codex, &target)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        let mut codex_input: AgentProfileDocumentInput =
            serde_json::from_value(profile_document(&target, &codex).unwrap()).unwrap();
        codex_input.managed.providers[0].credential.kind = "unknown".to_owned();
        codex_input.managed.providers[0].credential.reference = "credential:configured".to_owned();
        codex_input.managed.providers[0].credential.secret =
            Some("codex-command-secret".to_owned());
        let codex_path = codex.source_path().to_path_buf();
        let saved = save_on_target(
            target.clone(),
            AgentProfileSaveRequest {
                agent: AgentKind::Codex,
                runtime_target_id: target.target_id.clone(),
                profile_id: codex_input.profile_id.clone(),
                expected_revision: codex_input.revision.revision,
                profile: codex_input,
            },
        )
        .unwrap();
        let codex_raw = fs::read_to_string(&codex_path).unwrap();
        assert!(codex_raw.contains("experimental_bearer_token = \"codex-command-secret\""));
        assert!(!codex_raw.contains("env_key"));
        assert!(!saved.profile.to_string().contains("codex-command-secret"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn models_url_candidates_cover_openai_and_anthropic_bases() {
        assert_eq!(
            models_url_candidates("https://api.openai.com/v1").unwrap(),
            vec!["https://api.openai.com/v1/models".to_owned()]
        );
        assert_eq!(
            models_url_candidates("https://api.anthropic.com").unwrap(),
            vec![
                "https://api.anthropic.com/models".to_owned(),
                "https://api.anthropic.com/v1/models".to_owned()
            ]
        );
        assert_eq!(
            models_url_candidates("https://gateway.example/v1/models").unwrap(),
            vec!["https://gateway.example/v1/models".to_owned()]
        );
        assert_eq!(
            models_url_candidates("not-a-url").unwrap_err().code,
            "AGENT_PROFILE_BASE_URL_INVALID"
        );
    }

    #[test]
    fn parse_upstream_models_reads_openai_and_anthropic_payloads() {
        let openai = parse_upstream_models(&json!({
            "object": "list",
            "data": [
                {"id": "gpt-4o", "object": "model"},
                {"id": "o3", "object": "model"},
                {"id": "gpt-4o", "object": "model"}
            ]
        }));
        assert_eq!(
            openai,
            vec![
                UpstreamModelWire {
                    model_id: "gpt-4o".to_owned(),
                    display_name: "gpt-4o".to_owned(),
                },
                UpstreamModelWire {
                    model_id: "o3".to_owned(),
                    display_name: "o3".to_owned(),
                },
            ]
        );

        let anthropic = parse_upstream_models(&json!({
            "data": [{
                "type": "model",
                "id": "claude-sonnet-4-20250514",
                "display_name": "Claude Sonnet 4"
            }]
        }));
        assert_eq!(
            anthropic,
            vec![UpstreamModelWire {
                model_id: "claude-sonnet-4-20250514".to_owned(),
                display_name: "Claude Sonnet 4".to_owned(),
            }]
        );

        let names = parse_upstream_models(&json!({
            "models": ["alpha", "beta"]
        }));
        assert_eq!(names.len(), 2);
        assert_eq!(names[0].model_id, "alpha");

        let mut many = Vec::new();
        for index in 0..(UPSTREAM_MODEL_LIMIT + 25) {
            many.push(json!({ "id": format!("model-{index:03}") }));
        }
        assert_eq!(
            parse_upstream_models(&json!({ "data": many })).len(),
            UPSTREAM_MODEL_LIMIT
        );
    }

    #[test]
    fn usable_secret_rejects_placeholders_and_debug_redacts_api_key() {
        assert_eq!(
            usable_secret("sk-live".to_owned()).as_deref(),
            Some("sk-live")
        );
        assert_eq!(usable_secret("{env:OPENAI_API_KEY}".to_owned()), None);
        assert_eq!(usable_secret("{file:secret}".to_owned()), None);
        assert_eq!(usable_secret("$OPENAI_API_KEY".to_owned()), None);
        assert_eq!(usable_secret("   ".to_owned()), None);

        let request = AgentProfileListModelsRequest {
            agent: AgentKind::Opencode,
            runtime_target_id: "host".to_owned(),
            profile_id: "profile".to_owned(),
            provider_id: "openai".to_owned(),
            base_url: "https://api.openai.com/v1".to_owned(),
            protocol: "openai_chat_completions".to_owned(),
            api_key: Some("sk-secret-value".to_owned()),
        };
        let debug = format!("{request:?}");
        assert!(debug.contains("[redacted]"));
        assert!(!debug.contains("sk-secret-value"));
    }

    #[test]
    fn stored_provider_secret_reads_literal_keys_from_native_configs() {
        let root = std::env::temp_dir().join(format!(
            "vibehub-agent-profile-list-secret-{}",
            Uuid::new_v4()
        ));
        fs::create_dir_all(opencode_config_dir(&root)).unwrap();
        fs::create_dir_all(root.join(".claude")).unwrap();
        fs::create_dir_all(root.join(".codex")).unwrap();
        fs::write(
            opencode_config_dir(&root).join("opencode.json"),
            r#"{"$schema":"https://opencode.ai/config.json","model":"openai/gpt-5","provider":{"openai":{"options":{"baseURL":"https://api.openai.com/v1","apiKey":"opencode-list-secret"},"models":{"gpt-5":{"reasoning":true}}}}}"#,
        )
        .unwrap();
        fs::write(
            root.join(".claude/settings.json"),
            r#"{"model":"claude-sonnet","env":{"ANTHROPIC_AUTH_TOKEN":"claude-list-secret","ANTHROPIC_BASE_URL":"https://api.anthropic.com"},"permissions":{"allow":[]}}"#,
        )
        .unwrap();
        fs::write(
            root.join(".codex/config.toml"),
            "model = \"gpt-5\"\nmodel_provider = \"openai\"\n\n[model_providers.openai]\nname = \"OpenAI\"\nbase_url = \"https://api.openai.com/v1\"\nwire_api = \"responses\"\nexperimental_bearer_token = \"codex-list-secret\"\n",
        )
        .unwrap();
        let target = RuntimeTarget::host(root.clone());

        let opencode = discover_locations(&AgentKind::Opencode, &target)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        assert_eq!(
            stored_provider_secret(&target, &opencode, "openai")
                .unwrap()
                .as_deref(),
            Some("opencode-list-secret")
        );

        let claude = discover_locations(&AgentKind::ClaudeCode, &target)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        assert_eq!(
            stored_provider_secret(&target, &claude, "anthropic")
                .unwrap()
                .as_deref(),
            Some("claude-list-secret")
        );

        let codex = discover_locations(&AgentKind::Codex, &target)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        assert_eq!(
            stored_provider_secret(&target, &codex, "openai")
                .unwrap()
                .as_deref(),
            Some("codex-list-secret")
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn list_upstream_models_uses_request_key_and_v1_fallback() {
        let missing = list_upstream_models(AgentProfileListModelsRequest {
            agent: AgentKind::Opencode,
            runtime_target_id: "unused".to_owned(),
            profile_id: "unused".to_owned(),
            provider_id: "openai".to_owned(),
            base_url: String::new(),
            protocol: String::new(),
            api_key: Some("sk-test".to_owned()),
        })
        .await
        .unwrap_err();
        assert_eq!(missing.code, "AGENT_PROFILE_BASE_URL_REQUIRED");

        let invalid = list_upstream_models(AgentProfileListModelsRequest {
            agent: AgentKind::Opencode,
            runtime_target_id: "unused".to_owned(),
            profile_id: "unused".to_owned(),
            provider_id: "openai".to_owned(),
            base_url: "not-a-url".to_owned(),
            protocol: String::new(),
            api_key: Some("sk-test".to_owned()),
        })
        .await
        .unwrap_err();
        assert_eq!(invalid.code, "AGENT_PROFILE_BASE_URL_INVALID");

        let app = axum::Router::new()
            .route(
                "/models",
                axum::routing::get(|| async { axum::http::StatusCode::NOT_FOUND }),
            )
            .route(
                "/v1/models",
                axum::routing::get(|| async {
                    axum::Json(json!({
                        "data": [
                            {"id": "gpt-4o"},
                            {"id": "o3", "name": "O3"}
                        ]
                    }))
                }),
            );
        let base = spawn_local_app(app).await;
        let listed = list_upstream_models(AgentProfileListModelsRequest {
            agent: AgentKind::Opencode,
            runtime_target_id: "unused".to_owned(),
            profile_id: "unused".to_owned(),
            provider_id: "openai".to_owned(),
            base_url: base.clone(),
            protocol: "openai_chat_completions".to_owned(),
            api_key: Some("sk-test".to_owned()),
        })
        .await
        .unwrap();
        assert_eq!(listed.endpoint, format!("{base}/v1/models"));
        assert_eq!(listed.models.len(), 2);
        assert_eq!(listed.models[0].model_id, "gpt-4o");
        assert_eq!(listed.models[1].display_name, "O3");

        let denied = axum::Router::new().route(
            "/v1/models",
            axum::routing::get(|| async { axum::http::StatusCode::UNAUTHORIZED }),
        );
        let denied_base = spawn_local_app(denied).await;
        let auth_error = list_upstream_models(AgentProfileListModelsRequest {
            agent: AgentKind::ClaudeCode,
            runtime_target_id: "unused".to_owned(),
            profile_id: "unused".to_owned(),
            provider_id: "anthropic".to_owned(),
            base_url: format!("{denied_base}/v1"),
            protocol: "anthropic_messages".to_owned(),
            api_key: Some("sk-bad".to_owned()),
        })
        .await
        .unwrap_err();
        assert_eq!(auth_error.code, "AGENT_PROFILE_UPSTREAM_AUTH_FAILED");
        assert!(!format!("{auth_error:?}").contains("sk-bad"));
    }

    async fn spawn_local_app(app: axum::Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        format!("http://{addr}")
    }
}

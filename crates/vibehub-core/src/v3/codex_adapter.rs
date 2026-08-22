use super::agent_profile_storage::{
    read_document, restore_document, write_document, AgentKind, ConfigDocument, ConfigFormat,
    DocumentRevision, ParsedConfig, RuntimeTarget, StorageError, WriteReport,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const CODEX_DIRECTORY: &str = ".codex";
const CODEX_BASE_CONFIG: &str = "config.toml";
const CODEX_PROFILE_INDEX: &str = "vibehub-profile-index.json";
const CODEX_PROFILE_SUFFIX: &str = ".config.toml";
const CODEX_INDEX_SCHEMA_VERSION: u32 = 1;
const CODEX_EXECUTABLE: &str = "codex";
const CODEX_CURRENT_PROFILE_ARGUMENT: &str = "--profile-v2";

const ROOT_MANAGED_KEYS: &[&str] = &[
    "model",
    "model_provider",
    "model_reasoning_effort",
    "model_reasoning_summary",
    "model_catalog_json",
];

/// OpenAI Codex reserved (built-in) provider ids. These ship with Codex and are
/// backed by OpenAI's own model catalog; a third-party provider id that is not
/// in this set is treated as custom and must NOT keep an OpenAI-scoped catalog,
/// otherwise the picker shows unrelated OpenAI models and the selected model
/// resolves to "Custom".
const OPENAI_NATIVE_PROVIDER_IDS: &[&str] = &["openai", "ollama", "lmstudio"];
const PROVIDER_MANAGED_KEYS: &[&str] = &[
    "name",
    "base_url",
    "wire_api",
    "requires_openai_auth",
    "env_key",
    "experimental_bearer_token",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexSettingsScope {
    User,
    Profile,
    Project,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexProtocol {
    OpenaiResponses,
    OpenaiChatCompletions,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexCredentialKind {
    Environment,
    ConfigLiteral,
    None,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexCredentialReference {
    pub kind: CodexCredentialKind,
    pub references: Vec<String>,
    pub display: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexProviderView {
    pub provider_id: String,
    pub display_name: String,
    pub base_url: Option<String>,
    pub wire_api: Option<String>,
    pub protocol: CodexProtocol,
    pub credential: CodexCredentialReference,
    pub unknown_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexLaunchSpec {
    pub executable: String,
    pub profile_argument: String,
    pub profile_name: String,
    pub arguments: Vec<String>,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexDefaultState {
    pub is_default: bool,
    pub selected_by: CodexDefaultSelector,
    pub projection_target: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexDefaultSelector {
    Vibehub,
    Native,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexProfileView {
    pub agent: AgentKind,
    pub profile_id: String,
    pub display_name: String,
    pub source_path: PathBuf,
    pub scope: CodexSettingsScope,
    pub format: ConfigFormat,
    pub revision: DocumentRevision,
    pub model: Option<String>,
    pub model_provider: Option<String>,
    pub reasoning_effort: Option<String>,
    pub reasoning_summary: Option<String>,
    pub providers: Vec<CodexProviderView>,
    pub default_state: CodexDefaultState,
    pub launch: Option<CodexLaunchSpec>,
    pub project_override_warning: Option<String>,
    pub managed_fields: Vec<String>,
    pub unknown_root_fields: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CodexProviderPatch {
    pub display_name: Option<String>,
    pub base_url: Option<String>,
    pub wire_api: Option<String>,
    pub requires_openai_auth: Option<bool>,
    pub environment_key: Option<String>,
    /// Written to `experimental_bearer_token` so Codex can authenticate
    /// without a shell environment variable. Omitted from Debug/JSON logs.
    #[serde(default, skip_serializing)]
    pub bearer_token: Option<String>,
    pub clear_base_url: bool,
    pub clear_environment_key: bool,
    #[serde(default)]
    pub clear_bearer_token: bool,
}

impl std::fmt::Debug for CodexProviderPatch {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CodexProviderPatch")
            .field("display_name", &self.display_name)
            .field("base_url", &self.base_url)
            .field("wire_api", &self.wire_api)
            .field("requires_openai_auth", &self.requires_openai_auth)
            .field("environment_key", &self.environment_key)
            .field(
                "bearer_token",
                &self.bearer_token.as_ref().map(|_| "[redacted]"),
            )
            .field("clear_base_url", &self.clear_base_url)
            .field("clear_environment_key", &self.clear_environment_key)
            .field("clear_bearer_token", &self.clear_bearer_token)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CodexConfigPatch {
    pub model: Option<String>,
    pub model_provider: Option<String>,
    pub reasoning_effort: Option<String>,
    pub reasoning_summary: Option<String>,
    /// `model_catalog_json` drives the Codex model picker list and the display
    /// name of the selected model. It is managed so activation keeps the catalog
    /// consistent with the active provider (cleared for third-party providers).
    pub model_catalog_json: Option<String>,
    pub clear_model: bool,
    pub clear_model_provider: bool,
    pub clear_reasoning_effort: bool,
    pub clear_reasoning_summary: bool,
    pub clear_model_catalog_json: bool,
    pub deleted_providers: Vec<String>,
    pub providers: BTreeMap<String, CodexProviderPatch>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexProfileOperation {
    pub profile: CodexProfileView,
    pub write: Option<WriteReport>,
    pub index_write: Option<WriteReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexDeleteRequest {
    pub profile_path: PathBuf,
    pub replacement_profile_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexProfileIndexView {
    pub default_profile_id: Option<String>,
    pub profiles: BTreeMap<String, String>,
    pub revision: Option<DocumentRevision>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CodexProfileIndex {
    schema_version: u32,
    default_profile_id: Option<String>,
    profiles: BTreeMap<String, String>,
}

impl Default for CodexProfileIndex {
    fn default() -> Self {
        Self {
            schema_version: CODEX_INDEX_SCHEMA_VERSION,
            default_profile_id: None,
            profiles: BTreeMap::new(),
        }
    }
}

pub fn codex_directory(target: &RuntimeTarget) -> PathBuf {
    target.home_path.as_path().join(CODEX_DIRECTORY)
}

pub fn codex_base_config_path(target: &RuntimeTarget) -> PathBuf {
    codex_directory(target).join(CODEX_BASE_CONFIG)
}

pub fn codex_profile_index_path(target: &RuntimeTarget) -> PathBuf {
    codex_directory(target).join(CODEX_PROFILE_INDEX)
}

pub fn codex_profile_path(
    target: &RuntimeTarget,
    profile_name: &str,
) -> Result<PathBuf, StorageError> {
    let name = validate_profile_name(profile_name)?;
    Ok(codex_directory(target).join(format!("{name}{CODEX_PROFILE_SUFFIX}")))
}

pub fn codex_profile_launch_args(profile_name: &str) -> Result<Vec<String>, StorageError> {
    let name = validate_profile_name(profile_name)?;
    Ok(vec![CODEX_CURRENT_PROFILE_ARGUMENT.to_owned(), name])
}

pub fn codex_profile_launch_spec(profile_name: &str) -> Result<CodexLaunchSpec, StorageError> {
    let name = validate_profile_name(profile_name)?;
    Ok(CodexLaunchSpec {
        executable: CODEX_EXECUTABLE.to_owned(),
        profile_argument: CODEX_CURRENT_PROFILE_ARGUMENT.to_owned(),
        profile_name: name.clone(),
        arguments: vec![CODEX_CURRENT_PROFILE_ARGUMENT.to_owned(), name],
        warning: Some(
            "当前 Codex CLI 使用 --profile-v2；--profile 属于旧 config.toml profile 机制。"
                .to_owned(),
        ),
    })
}

pub fn codex_profile_index(target: &RuntimeTarget) -> Result<CodexProfileIndexView, StorageError> {
    let (index, revision) = read_profile_index(target)?;
    Ok(CodexProfileIndexView {
        default_profile_id: index.default_profile_id,
        profiles: index.profiles,
        revision,
    })
}

pub fn discover_codex_profiles(
    target: &RuntimeTarget,
) -> Result<Vec<CodexProfileView>, StorageError> {
    let (index, _) = read_profile_index(target)?;
    let directory = codex_directory(target);
    let mut paths = Vec::new();
    let base = codex_base_config_path(target);
    if base.is_file() {
        paths.push(base);
    }
    if directory.is_dir() {
        let metadata = fs::symlink_metadata(&directory)
            .map_err(|error| StorageError::new("CODEX_DIRECTORY_READ_FAILED", error.to_string()))?;
        if metadata.file_type().is_symlink() {
            return Err(StorageError::new(
                "CODEX_DIRECTORY_LINK_REJECTED",
                directory.display().to_string(),
            ));
        }
        for entry in fs::read_dir(&directory)
            .map_err(|error| StorageError::new("CODEX_DIRECTORY_READ_FAILED", error.to_string()))?
        {
            let path = entry
                .map_err(|error| {
                    StorageError::new("CODEX_DIRECTORY_READ_FAILED", error.to_string())
                })?
                .path();
            if is_codex_profile_file(&path) {
                paths.push(path);
            }
        }
    }
    paths.sort();
    paths
        .into_iter()
        .map(|path| read_codex_profile_with_index(target, path, &index))
        .collect()
}

pub fn read_codex_profile(
    target: &RuntimeTarget,
    path: impl AsRef<Path>,
) -> Result<CodexProfileView, StorageError> {
    let (index, _) = read_profile_index(target)?;
    read_codex_profile_with_index(target, path.as_ref().to_path_buf(), &index)
}

pub fn save_codex_profile(
    target: &RuntimeTarget,
    path: impl AsRef<Path>,
    expected_revision: Option<&DocumentRevision>,
    patch: &CodexConfigPatch,
) -> Result<WriteReport, StorageError> {
    let document = read_document(target, path)?;
    let root = parse_toml_root(&document)?;
    reject_legacy_profiles(&root)?;
    let edited = apply_toml_patch(&document.raw, patch)?;
    write_document(target, document.path, expected_revision, &edited)
}

pub fn import_codex_profile(
    target: &RuntimeTarget,
    source: impl AsRef<Path>,
    profile_name: &str,
) -> Result<CodexProfileOperation, StorageError> {
    let source_document = read_document(target, source)?;
    let source_root = parse_toml_root(&source_document)?;
    reject_legacy_profiles(&source_root)?;
    let destination = codex_profile_path(target, profile_name)?;
    ensure_codex_directory(target)?;
    if destination.exists() {
        return Err(StorageError::new(
            "CODEX_PROFILE_ALREADY_EXISTS",
            destination.display().to_string(),
        ));
    }
    let write = write_new_document(target, &destination, &source_document.raw)?;
    let profile = read_codex_profile(target, &destination)?;
    Ok(CodexProfileOperation {
        profile,
        write: Some(write),
        index_write: None,
    })
}

/// Build a minimal profile template from the base config: only the managed
/// model selection (`model`, `model_provider`, `model_reasoning_effort`,
/// `model_reasoning_summary`, `model_catalog_json`) plus the definition of the
/// currently selected provider under `model_providers.<id>`. Everything else
/// (marketplaces, plugins, projects, MCP servers, other providers) is dropped so
/// a cloned profile stays self-contained and provider-consistent.
fn minimal_profile_template(target: &RuntimeTarget) -> Result<Option<Vec<u8>>, StorageError> {
    let base_path = codex_base_config_path(target);
    if !base_path.is_file() {
        return Ok(None);
    }
    let document = read_document(target, &base_path)?;
    let root = parse_toml_root(&document)?;
    if has_legacy_profiles(&root) {
        return Err(StorageError::new(
            "CODEX_LEGACY_PROFILES_UNSUPPORTED",
            "legacy [profiles.*] configuration is read-only and cannot be migrated or generated",
        ));
    }

    let mut minimal = toml::map::Map::new();
    for key in ROOT_MANAGED_KEYS {
        if let Some(value) = root.get(*key) {
            minimal.insert((*key).to_string(), value.clone());
        }
    }
    if let Some(provider_id) = root.get("model_provider").and_then(toml::Value::as_str) {
        if let Some(provider) = root
            .get("model_providers")
            .and_then(toml::Value::as_table)
            .and_then(|providers| providers.get(provider_id))
        {
            let mut providers = toml::map::Map::new();
            providers.insert(provider_id.to_string(), provider.clone());
            minimal.insert("model_providers".to_string(), toml::Value::Table(providers));
        }
    }

    let rendered = toml::to_string_pretty(&toml::Value::Table(minimal)).map_err(|error| {
        StorageError::new("CODEX_PROFILE_TEMPLATE_SERIALIZE_FAILED", error.to_string())
    })?;
    Ok(Some(rendered.into_bytes()))
}

pub fn create_codex_profile(
    target: &RuntimeTarget,
    profile_name: &str,
    template: Option<&Path>,
) -> Result<CodexProfileOperation, StorageError> {
    let source = match template {
        Some(path) => read_document(target, path)?.raw,
        // No template: build a MINIMAL profile from the base config instead of
        // cloning it whole. Copying the entire base pulled in unrelated cockpit
        // marketplaces, plugins, projects and MCP servers, bloating the profile
        // and mixing provider-specific state (e.g. an OpenAI catalog pinned to a
        // third-party provider). Only the managed selection and the active
        // provider definition are carried over.
        None if codex_base_config_path(target).is_file() => {
            minimal_profile_template(target)?.unwrap_or_else(|| b"# Created by VibeHub\n".to_vec())
        }
        None => b"# Created by VibeHub\n".to_vec(),
    };
    let destination = codex_profile_path(target, profile_name)?;
    ensure_codex_directory(target)?;
    if destination.exists() {
        return Err(StorageError::new(
            "CODEX_PROFILE_ALREADY_EXISTS",
            destination.display().to_string(),
        ));
    }
    let document = ConfigDocument {
        path: destination.clone(),
        format: ConfigFormat::Toml,
        raw: source.clone(),
        revision: revision_for_bytes(&source),
    };
    reject_legacy_profiles(&parse_toml_root(&document)?)?;
    let write = write_new_document(target, &destination, &source)?;
    let profile = read_codex_profile(target, &destination)?;
    Ok(CodexProfileOperation {
        profile,
        write: Some(write),
        index_write: None,
    })
}

pub fn clone_codex_profile(
    target: &RuntimeTarget,
    source: impl AsRef<Path>,
    profile_name: &str,
) -> Result<CodexProfileOperation, StorageError> {
    import_codex_profile(target, source, profile_name)
}

pub fn rename_codex_profile(
    target: &RuntimeTarget,
    source: impl AsRef<Path>,
    new_name: &str,
) -> Result<CodexProfileView, StorageError> {
    let source = source.as_ref().to_path_buf();
    let document = read_document(target, &source)?;
    reject_legacy_profiles(&parse_toml_root(&document)?)?;
    let destination = codex_profile_path(target, new_name)?;
    if destination.exists() {
        return Err(StorageError::new(
            "CODEX_PROFILE_ALREADY_EXISTS",
            destination.display().to_string(),
        ));
    }
    fs::rename(&source, &destination)
        .map_err(|error| StorageError::new("CODEX_PROFILE_RENAME_FAILED", error.to_string()))?;
    let (mut index, index_revision) = read_profile_index(target)?;
    let old_id = profile_id_for_path(&source, CodexSettingsScope::Profile);
    let new_id = profile_id_for_path(&destination, CodexSettingsScope::Profile);
    if index.default_profile_id.as_deref() == Some(old_id.as_str()) {
        index.default_profile_id = Some(new_id.clone());
    }
    if index.profiles.remove(&old_id).is_some() {
        index
            .profiles
            .insert(new_id, path_for_index(&destination, target));
    }
    let _ = write_profile_index(target, &index, index_revision.as_ref())?;
    read_codex_profile(target, destination)
}

pub fn delete_codex_profile(
    target: &RuntimeTarget,
    request: &CodexDeleteRequest,
) -> Result<CodexProfileOperation, StorageError> {
    let source = request.profile_path.as_path();
    let current = read_codex_profile(target, source)?;
    if current.scope != CodexSettingsScope::Profile {
        return Err(StorageError::new(
            "CODEX_PROFILE_DELETE_SCOPE_INVALID",
            "only a named Codex Profile can be deleted",
        ));
    }
    let (mut index, mut index_revision) = read_profile_index(target)?;
    let is_default = index.default_profile_id.as_deref() == Some(current.profile_id.as_str());
    let mut replacement = None;
    if is_default {
        let replacement_path = request.replacement_profile_path.as_ref().ok_or_else(|| {
            StorageError::new(
                "CODEX_DEFAULT_PROFILE_REPLACEMENT_REQUIRED",
                "the default Profile must be replaced or explicitly cleared before deletion",
            )
        })?;
        if replacement_path == source {
            return Err(StorageError::new(
                "CODEX_DEFAULT_PROFILE_REPLACEMENT_INVALID",
                "a Profile cannot replace itself",
            ));
        }
        replacement = Some(activate_codex_profile(target, replacement_path)?);
        (index, index_revision) = read_profile_index(target)?;
    }
    fs::remove_file(source)
        .map_err(|error| StorageError::new("CODEX_PROFILE_DELETE_FAILED", error.to_string()))?;
    index.profiles.remove(&current.profile_id);
    let index_write = write_profile_index(target, &index, index_revision.as_ref())?;
    Ok(CodexProfileOperation {
        profile: replacement
            .map(|operation| operation.profile)
            .unwrap_or(current),
        write: None,
        index_write: Some(index_write),
    })
}

pub fn clear_codex_default_profile(target: &RuntimeTarget) -> Result<WriteReport, StorageError> {
    let (mut index, index_revision) = read_profile_index(target)?;
    index.default_profile_id = None;
    write_profile_index(target, &index, index_revision.as_ref())
}

/// True when `provider_id` is one of Codex's built-in providers, which are
/// backed by OpenAI's own model catalog. Third-party ids (e.g. a reseller or a
/// self-hosted endpoint) must not keep an OpenAI-scoped `model_catalog_json`,
/// or the picker shows unrelated OpenAI models and the selected model renders
/// as "Custom".
fn is_openai_native_provider(provider_id: &str) -> bool {
    OPENAI_NATIVE_PROVIDER_IDS
        .iter()
        .any(|native| native.eq_ignore_ascii_case(provider_id))
}

/// Build the catalog-reconciliation patch applied on top of the managed
/// projection. For a third-party provider the OpenAI-scoped catalog is cleared
/// so the picker no longer lists unrelated models; for a native provider the
/// catalog is left untouched (it is not a managed projection key here).
fn catalog_reconciliation_patch(provider_id: Option<&str>) -> CodexConfigPatch {
    let clear = provider_id
        .map(|id| !is_openai_native_provider(id))
        .unwrap_or(false);
    CodexConfigPatch {
        clear_model_catalog_json: clear,
        ..Default::default()
    }
}

pub fn activate_codex_profile(
    target: &RuntimeTarget,
    profile_path: impl AsRef<Path>,
) -> Result<CodexProfileOperation, StorageError> {
    let profile_path = profile_path.as_ref();
    let profile = read_codex_profile(target, profile_path)?;
    if profile.scope != CodexSettingsScope::Profile {
        return Err(StorageError::new(
            "CODEX_PROFILE_ACTIVATE_SCOPE_INVALID",
            "only a named Codex Profile can be activated",
        ));
    }
    let profile_document = read_document(target, profile_path)?;
    let profile_root = parse_toml_root(&profile_document)?;
    reject_legacy_profiles(&profile_root)?;
    let patch = managed_patch_from_root(&profile_root)?;
    let (mut index, index_revision) = read_profile_index(target)?;
    let base_path = codex_base_config_path(target);
    ensure_codex_directory(target)?;
    let base_document = if base_path.is_file() {
        Some(read_document(target, &base_path)?)
    } else {
        None
    };
    let edited = match base_document.as_ref() {
        Some(document) => apply_toml_patch(&document.raw, &patch)?,
        None => apply_toml_patch(b"", &patch)?,
    };
    let base_write = match base_document.as_ref() {
        Some(document) => write_document(target, &base_path, Some(&document.revision), &edited)?,
        None => write_new_document(target, &base_path, &edited)?,
    };

    // Reconcile the model catalog with the activated provider. A third-party
    // provider must not keep an OpenAI-scoped catalog, so clear it; this is a
    // second, additive write keyed on the just-written base revision. Re-read
    // so base_write reflects the final file state and rollback stays consistent.
    let catalog_patch = catalog_reconciliation_patch(profile.model_provider.as_deref());
    let base_write = if catalog_patch.clear_model_catalog_json {
        let current = read_document(target, &base_path)?;
        let reconciled = apply_toml_patch(&current.raw, &catalog_patch)?;
        write_document(target, &base_path, Some(&current.revision), &reconciled)?
    } else {
        base_write
    };

    index.default_profile_id = Some(profile.profile_id.clone());
    index.profiles.insert(
        profile.profile_id.clone(),
        path_for_index(profile_path, target),
    );
    let index_write = match write_profile_index(target, &index, index_revision.as_ref()) {
        Ok(write) => write,
        Err(error) => {
            if let Some(backup) = base_write.backup_path.as_ref() {
                let _ = restore_document(
                    target,
                    &base_path,
                    backup.as_path(),
                    &base_write.after_revision,
                );
            } else {
                let _ = fs::remove_file(&base_path);
            }
            return Err(StorageError::new(
                "CODEX_DEFAULT_PROJECTION_ROLLED_BACK",
                format!("default Profile index write failed: {error}"),
            ));
        }
    };
    Ok(CodexProfileOperation {
        profile: read_codex_profile(target, profile_path)?,
        write: Some(base_write),
        index_write: Some(index_write),
    })
}

fn read_codex_profile_with_index(
    target: &RuntimeTarget,
    path: PathBuf,
    index: &CodexProfileIndex,
) -> Result<CodexProfileView, StorageError> {
    let document = read_document(target, &path)?;
    let root = parse_toml_root(&document)?;
    let scope = settings_scope(target, &path);
    let profile_id = profile_id_for_path(&path, scope);
    let display_name = display_name_for_path(&path, scope);
    let mut warnings = Vec::new();
    let legacy = has_legacy_profiles(&root);
    if legacy {
        warnings.push("CODEX_LEGACY_PROFILES_UNSUPPORTED".to_owned());
    }
    let model = root
        .get("model")
        .and_then(toml::Value::as_str)
        .map(str::to_owned);
    let model_provider = root
        .get("model_provider")
        .and_then(toml::Value::as_str)
        .map(str::to_owned);
    let reasoning_effort = root
        .get("model_reasoning_effort")
        .and_then(toml::Value::as_str)
        .map(str::to_owned);
    let reasoning_summary = root
        .get("model_reasoning_summary")
        .and_then(toml::Value::as_str)
        .map(str::to_owned);
    let providers = root
        .get("model_providers")
        .and_then(toml::Value::as_table)
        .map(|table| {
            table
                .iter()
                .map(|(id, value)| provider_view(id, value))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    let unknown_root_fields = root
        .keys()
        .filter(|key| {
            !ROOT_MANAGED_KEYS.contains(&key.as_str()) && key.as_str() != "model_providers"
        })
        .cloned()
        .collect();
    if let Some(warning) = provider_consistency_warning(&root) {
        warnings.push(warning);
    }
    let is_native_default = scope == CodexSettingsScope::User;
    let is_vibehub_default = index.default_profile_id.as_deref() == Some(profile_id.as_str());
    let (is_default, selected_by) = if is_vibehub_default {
        (true, CodexDefaultSelector::Vibehub)
    } else if is_native_default {
        (true, CodexDefaultSelector::Native)
    } else {
        (false, CodexDefaultSelector::Unknown)
    };
    let launch = if scope == CodexSettingsScope::Profile {
        Some(codex_profile_launch_spec(&display_name)?)
    } else {
        None
    };
    Ok(CodexProfileView {
        agent: AgentKind::Codex,
        profile_id,
        display_name,
        source_path: path,
        scope,
        format: document.format,
        revision: document.revision,
        model,
        model_provider,
        reasoning_effort,
        reasoning_summary,
        providers,
        default_state: CodexDefaultState {
            is_default,
            selected_by,
            projection_target: codex_base_config_path(target),
        },
        launch,
        project_override_warning: Some(
            "项目目录下的 .codex/config.toml 或 --cd 作用域配置可能覆盖用户级默认值。".to_owned(),
        ),
        managed_fields: vec![
            "model".to_owned(),
            "model_provider".to_owned(),
            "model_reasoning_effort".to_owned(),
            "model_reasoning_summary".to_owned(),
            "model_providers.*.name".to_owned(),
            "model_providers.*.base_url".to_owned(),
            "model_providers.*.wire_api".to_owned(),
        ],
        unknown_root_fields,
        warnings,
    })
}

fn parse_toml_root(document: &ConfigDocument) -> Result<toml::value::Table, StorageError> {
    match document.parse()? {
        ParsedConfig::Toml(value) => value.as_table().cloned().ok_or_else(|| {
            StorageError::new(
                "CODEX_CONFIG_ROOT_INVALID",
                "Codex TOML root must be a table",
            )
        }),
        ParsedConfig::Json(_) => Err(StorageError::new(
            "CODEX_CONFIG_FORMAT_UNSUPPORTED",
            "Codex configuration must be TOML",
        )),
    }
}

fn provider_view(id: &str, value: &toml::Value) -> Result<CodexProviderView, StorageError> {
    let table = value.as_table().ok_or_else(|| {
        StorageError::new(
            "CODEX_PROVIDER_INVALID",
            format!("model provider {id} must be a TOML table"),
        )
    })?;
    let wire_api = table
        .get("wire_api")
        .and_then(toml::Value::as_str)
        .map(str::to_owned);
    let protocol = wire_api
        .as_deref()
        .map(protocol_for_wire_api)
        .unwrap_or(CodexProtocol::Unknown);
    let credential = credential_reference(table);
    Ok(CodexProviderView {
        provider_id: id.to_owned(),
        display_name: table
            .get("name")
            .and_then(toml::Value::as_str)
            .unwrap_or(id)
            .to_owned(),
        base_url: table
            .get("base_url")
            .and_then(toml::Value::as_str)
            .map(str::to_owned),
        wire_api,
        protocol,
        credential,
        unknown_fields: table
            .keys()
            .filter(|key| !PROVIDER_MANAGED_KEYS.contains(&key.as_str()))
            .cloned()
            .collect(),
    })
}

fn managed_patch_from_root(root: &toml::value::Table) -> Result<CodexConfigPatch, StorageError> {
    let model_provider = root
        .get("model_provider")
        .and_then(toml::Value::as_str)
        .map(str::to_owned);
    let mut providers = BTreeMap::new();
    if let Some(provider_id) = model_provider.as_deref() {
        if let Some(provider) = root
            .get("model_providers")
            .and_then(toml::Value::as_table)
            .and_then(|providers| providers.get(provider_id))
        {
            let table = provider.as_table().ok_or_else(|| {
                StorageError::new("CODEX_PROVIDER_INVALID", "provider must be a TOML table")
            })?;
            let bearer_token = table
                .get("experimental_bearer_token")
                .and_then(toml::Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(str::to_owned);
            let environment_key = table
                .get("env_key")
                .and_then(toml::Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(str::to_owned);
            providers.insert(
                provider_id.to_owned(),
                CodexProviderPatch {
                    display_name: table
                        .get("name")
                        .and_then(toml::Value::as_str)
                        .map(str::to_owned),
                    base_url: table
                        .get("base_url")
                        .and_then(toml::Value::as_str)
                        .map(str::to_owned),
                    wire_api: table
                        .get("wire_api")
                        .and_then(toml::Value::as_str)
                        .map(str::to_owned),
                    requires_openai_auth: table
                        .get("requires_openai_auth")
                        .and_then(toml::Value::as_bool),
                    environment_key: if bearer_token.is_some() {
                        None
                    } else {
                        environment_key.clone()
                    },
                    bearer_token: bearer_token.clone(),
                    clear_environment_key: bearer_token.is_some(),
                    clear_bearer_token: bearer_token.is_none() && environment_key.is_some(),
                    ..Default::default()
                },
            );
        }
    }
    if let Some(wire_api) = providers
        .values()
        .find_map(|provider| provider.wire_api.as_deref())
    {
        validate_wire_api(wire_api)?;
    }
    Ok(CodexConfigPatch {
        model: root
            .get("model")
            .and_then(toml::Value::as_str)
            .map(str::to_owned),
        model_provider,
        reasoning_effort: root
            .get("model_reasoning_effort")
            .and_then(toml::Value::as_str)
            .map(str::to_owned),
        reasoning_summary: root
            .get("model_reasoning_summary")
            .and_then(toml::Value::as_str)
            .map(str::to_owned),
        model_catalog_json: root
            .get("model_catalog_json")
            .and_then(toml::Value::as_str)
            .map(str::to_owned),
        providers,
        ..Default::default()
    })
}

fn apply_toml_patch(raw: &[u8], patch: &CodexConfigPatch) -> Result<Vec<u8>, StorageError> {
    let text = std::str::from_utf8(raw)
        .map_err(|error| StorageError::new("CODEX_CONFIG_ENCODING_INVALID", error.to_string()))?;
    if !text.trim().is_empty() {
        let parsed = toml::from_str::<toml::Value>(text)
            .map_err(|error| StorageError::new("CODEX_CONFIG_TOML_INVALID", error.to_string()))?;
        let root = parsed.as_table().ok_or_else(|| {
            StorageError::new(
                "CODEX_CONFIG_ROOT_INVALID",
                "Codex TOML root must be a table",
            )
        })?;
        reject_legacy_profiles(root)?;
    }
    let mut editor = TomlTextEditor::new(text.to_owned());
    for provider_id in &patch.deleted_providers {
        validate_provider_id(provider_id)?;
        editor.remove_section_all(&format!("model_providers.{provider_id}"));
    }
    if let Some(model) = patch.model.as_deref() {
        validate_non_empty("model", model)?;
        editor.set_root("model", toml_string(model))?;
    } else if patch.clear_model {
        editor.remove_root("model");
    }
    if let Some(provider) = patch.model_provider.as_deref() {
        validate_provider_id(provider)?;
        editor.set_root("model_provider", toml_string(provider))?;
    } else if patch.clear_model_provider {
        editor.remove_root("model_provider");
    }
    if let Some(effort) = patch.reasoning_effort.as_deref() {
        validate_reasoning_effort(effort)?;
        editor.set_root("model_reasoning_effort", toml_string(effort))?;
    } else if patch.clear_reasoning_effort {
        editor.remove_root("model_reasoning_effort");
    }
    if let Some(summary) = patch.reasoning_summary.as_deref() {
        validate_non_empty("reasoning_summary", summary)?;
        editor.set_root("model_reasoning_summary", toml_string(summary))?;
    } else if patch.clear_reasoning_summary {
        editor.remove_root("model_reasoning_summary");
    }
    if let Some(catalog) = patch.model_catalog_json.as_deref() {
        validate_non_empty("model_catalog_json", catalog)?;
        editor.set_root("model_catalog_json", toml_string(catalog))?;
    } else if patch.clear_model_catalog_json {
        editor.remove_root("model_catalog_json");
    }
    for (provider_id, provider_patch) in &patch.providers {
        validate_provider_id(provider_id)?;
        if let Some(wire_api) = provider_patch.wire_api.as_deref() {
            validate_wire_api(wire_api)?;
        }
        if let Some(base_url) = provider_patch.base_url.as_deref() {
            validate_base_url(base_url)?;
        }
        if let Some(environment_key) = provider_patch.environment_key.as_deref() {
            validate_environment_key(environment_key)?;
        }
        let section = format!("model_providers.{provider_id}");
        if let Some(name) = provider_patch.display_name.as_deref() {
            validate_non_empty("provider name", name)?;
            editor.set_section(&section, "name", toml_string(name))?;
        }
        if let Some(base_url) = provider_patch.base_url.as_deref() {
            editor.set_section(&section, "base_url", toml_string(base_url))?;
        } else if provider_patch.clear_base_url {
            editor.remove_section(&section, "base_url");
        }
        if let Some(wire_api) = provider_patch.wire_api.as_deref() {
            editor.set_section(&section, "wire_api", toml_string(wire_api))?;
        }
        if let Some(required) = provider_patch.requires_openai_auth {
            editor.set_section(&section, "requires_openai_auth", required.to_string())?;
        }
        if let Some(bearer_token) = provider_patch.bearer_token.as_deref() {
            validate_non_empty("experimental_bearer_token", bearer_token)?;
            editor.set_section(
                &section,
                "experimental_bearer_token",
                toml_string(bearer_token),
            )?;
            editor.remove_section(&section, "env_key");
            editor.remove_section(&section, "api_key");
            editor.remove_section(&section, "apiKey");
        } else if provider_patch.clear_bearer_token {
            editor.remove_section(&section, "experimental_bearer_token");
            editor.remove_section(&section, "api_key");
            editor.remove_section(&section, "apiKey");
        }
        if provider_patch.bearer_token.is_none() {
            if let Some(environment_key) = provider_patch.environment_key.as_deref() {
                editor.set_section(&section, "env_key", toml_string(environment_key))?;
            } else if provider_patch.clear_environment_key {
                editor.remove_section(&section, "env_key");
            }
        }
    }
    let output = editor.finish();
    toml::from_str::<toml::Value>(&output)
        .map_err(|error| StorageError::new("CODEX_CONFIG_PATCH_INVALID", error.to_string()))?;
    Ok(output.into_bytes())
}

struct TomlTextEditor {
    lines: Vec<String>,
}

impl TomlTextEditor {
    fn new(text: String) -> Self {
        let mut lines: Vec<String> = text.split_inclusive('\n').map(str::to_owned).collect();
        if lines.is_empty() {
            lines.push(String::new());
        }
        Self { lines }
    }

    fn set_root(&mut self, key: &str, value: String) -> Result<(), StorageError> {
        self.set_in_section("", key, value)
    }

    fn remove_root(&mut self, key: &str) {
        self.remove_in_section("", key);
    }

    fn set_section(&mut self, section: &str, key: &str, value: String) -> Result<(), StorageError> {
        if let Some(index) = self.section_start(section) {
            let end = self.section_end(index);
            if let Some(existing) = self.find_assignment(index, end, key) {
                self.lines[existing] = replace_assignment_line(&self.lines[existing], key, &value);
            } else {
                self.ensure_line_ending(end);
                self.lines.insert(end, format!("{key} = {value}\n"));
            }
            return Ok(());
        }
        if !self.lines.last().is_some_and(|line| line.ends_with('\n')) {
            self.lines.push("\n".to_owned());
        }
        self.lines.push(format!("[{section}]\n"));
        self.lines.push(format!("{key} = {value}\n"));
        Ok(())
    }

    fn remove_section(&mut self, section: &str, key: &str) {
        if let Some(index) = self.section_start(section) {
            let end = self.section_end(index);
            self.remove_line(self.find_assignment(index, end, key));
        }
    }

    fn remove_section_all(&mut self, section: &str) {
        if let Some(index) = self.section_start(section) {
            let end = self.section_end(index);
            self.lines.drain(index..end);
        }
    }

    fn set_in_section(
        &mut self,
        section: &str,
        key: &str,
        value: String,
    ) -> Result<(), StorageError> {
        let (start, end) = if section.is_empty() {
            (0, self.first_section().unwrap_or(self.lines.len()))
        } else if let Some(index) = self.section_start(section) {
            (index, self.section_end(index))
        } else {
            return self.set_section(section, key, value);
        };
        if let Some(existing) = self.find_assignment(start, end, key) {
            self.lines[existing] = replace_assignment_line(&self.lines[existing], key, &value);
        } else {
            self.ensure_line_ending(end);
            self.lines.insert(end, format!("{key} = {value}\n"));
        }
        Ok(())
    }

    fn ensure_line_ending(&mut self, index: usize) {
        if index > 0 && !self.lines[index - 1].ends_with('\n') {
            self.lines[index - 1].push('\n');
        }
    }

    fn remove_in_section(&mut self, section: &str, key: &str) {
        let (start, end) = if section.is_empty() {
            (0, self.first_section().unwrap_or(self.lines.len()))
        } else if let Some(index) = self.section_start(section) {
            (index, self.section_end(index))
        } else {
            return;
        };
        self.remove_line(self.find_assignment(start, end, key));
    }

    fn first_section(&self) -> Option<usize> {
        self.lines
            .iter()
            .position(|line| section_name(line).is_some())
    }

    fn section_start(&self, section: &str) -> Option<usize> {
        self.lines
            .iter()
            .position(|line| section_name(line).as_deref() == Some(section))
    }

    fn section_end(&self, start: usize) -> usize {
        self.lines
            .iter()
            .enumerate()
            .skip(start + 1)
            .find(|(_, line)| section_name(line).is_some())
            .map(|(index, _)| index)
            .unwrap_or(self.lines.len())
    }

    fn find_assignment(&self, start: usize, end: usize, key: &str) -> Option<usize> {
        (start..end).find(|index| assignment_key(&self.lines[*index]).as_deref() == Some(key))
    }

    fn remove_line(&mut self, index: Option<usize>) {
        if let Some(index) = index {
            let comment = inline_comment(&self.lines[index]);
            self.lines[index] = if comment.is_empty() {
                if self.lines[index].ends_with('\n') {
                    "\n".to_owned()
                } else {
                    String::new()
                }
            } else {
                format!("{comment}\n")
            };
        }
    }

    fn finish(self) -> String {
        self.lines.concat()
    }
}

fn section_name(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with('[') || trimmed.starts_with("[[") {
        return None;
    }
    let end = trimmed.find(']')?;
    let name = trimmed[1..end].trim();
    if name.is_empty() {
        None
    } else {
        Some(name.trim_matches('"').to_owned())
    }
}

fn assignment_key(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('[') {
        return None;
    }
    let (key, _) = trimmed.split_once('=')?;
    let key = key.trim();
    if key.is_empty() || key.contains('.') {
        None
    } else {
        Some(key.trim_matches('"').to_owned())
    }
}

fn inline_comment(line: &str) -> String {
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in line.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
        } else if character == '"' {
            in_string = true;
        } else if character == '#' {
            return line[index..].trim_end_matches(['\r', '\n']).to_owned();
        }
    }
    String::new()
}

fn replace_assignment_line(line: &str, key: &str, value: &str) -> String {
    let indent = line
        .chars()
        .take_while(|character| character.is_whitespace() && *character != '\n')
        .collect::<String>();
    let comment = inline_comment(line);
    let newline = if line.ends_with('\n') { "\n" } else { "" };
    if comment.is_empty() {
        format!("{indent}{key} = {value}{newline}")
    } else {
        format!("{indent}{key} = {value} {comment}{newline}")
    }
}

fn toml_string(value: &str) -> String {
    toml::to_string(value)
        .unwrap_or_else(|_| format!("\"{}\"", value.replace('"', "\\\"")))
        .trim()
        .to_owned()
}

fn reject_legacy_profiles(root: &toml::value::Table) -> Result<(), StorageError> {
    if has_legacy_profiles(root) {
        return Err(StorageError::new(
            "CODEX_LEGACY_PROFILES_UNSUPPORTED",
            "legacy [profiles.*] configuration is read-only and cannot be migrated or generated",
        ));
    }
    Ok(())
}

fn has_legacy_profiles(root: &toml::value::Table) -> bool {
    root.get("profiles").is_some()
}

fn credential_reference(table: &toml::value::Table) -> CodexCredentialReference {
    let references = table
        .get("env_key")
        .and_then(toml::Value::as_str)
        .filter(|name| validate_environment_key(name).is_ok())
        .map(|name| vec![name.to_owned()])
        .unwrap_or_default();
    if !references.is_empty() {
        return CodexCredentialReference {
            kind: CodexCredentialKind::Environment,
            display: format!("环境变量 {}", references.join(", ")),
            references,
        };
    }
    for key in ["api_key", "apiKey", "experimental_bearer_token"] {
        if let Some(value) = table.get(key) {
            if value.as_str().is_some_and(|value| !value.trim().is_empty()) {
                return CodexCredentialReference {
                    kind: CodexCredentialKind::ConfigLiteral,
                    references: Vec::new(),
                    display: "配置内存在凭据（已隐藏）".to_owned(),
                };
            }
        }
    }
    if table
        .get("requires_openai_auth")
        .and_then(toml::Value::as_bool)
        == Some(true)
    {
        return CodexCredentialReference {
            kind: CodexCredentialKind::Environment,
            references: vec!["OPENAI_API_KEY".to_owned()],
            display: "环境变量 OPENAI_API_KEY".to_owned(),
        };
    }
    CodexCredentialReference {
        kind: CodexCredentialKind::Unknown,
        references: Vec::new(),
        display: "由 Codex 运行时决定".to_owned(),
    }
}

fn protocol_for_wire_api(wire_api: &str) -> CodexProtocol {
    match wire_api {
        "responses" | "openai_responses" => CodexProtocol::OpenaiResponses,
        "chat" | "chat_completions" | "openai_chat_completions" => {
            CodexProtocol::OpenaiChatCompletions
        }
        _ => CodexProtocol::Unknown,
    }
}

fn validate_wire_api(wire_api: &str) -> Result<(), StorageError> {
    if matches!(
        protocol_for_wire_api(wire_api),
        CodexProtocol::OpenaiResponses | CodexProtocol::OpenaiChatCompletions
    ) {
        Ok(())
    } else {
        Err(StorageError::new(
            "CODEX_WIRE_API_UNSUPPORTED",
            format!("unsupported Codex wire_api: {wire_api}"),
        ))
    }
}

fn validate_base_url(base_url: &str) -> Result<(), StorageError> {
    if base_url.starts_with("https://") || base_url.starts_with("http://") {
        Ok(())
    } else {
        Err(StorageError::new(
            "CODEX_BASE_URL_INVALID",
            "custom provider base_url must use http:// or https://",
        ))
    }
}

fn validate_environment_key(key: &str) -> Result<(), StorageError> {
    if !key.is_empty()
        && key.chars().enumerate().all(|(index, character)| {
            character.is_ascii_alphabetic()
                || character == '_'
                || (index > 0 && character.is_ascii_digit())
        })
    {
        Ok(())
    } else {
        Err(StorageError::new(
            "CODEX_ENVIRONMENT_KEY_INVALID",
            "credential reference must be an environment variable name",
        ))
    }
}

/// Confirm that a selected `model_provider` has a matching definition under
/// `model_providers`. Returns a warning string (never blocks the write) when the
/// provider is set but not defined, so activation/rendering never surfaces an
/// orphaned endpoint reference.
fn provider_consistency_warning(root: &toml::value::Table) -> Option<String> {
    let provider_id = root.get("model_provider").and_then(toml::Value::as_str)?;
    let defined = root
        .get("model_providers")
        .and_then(toml::Value::as_table)
        .is_some_and(|providers| providers.contains_key(provider_id));
    if defined {
        None
    } else {
        Some(format!(
            "model_provider = \"{provider_id}\" has no matching [model_providers.{provider_id}] definition"
        ))
    }
}

fn validate_provider_id(provider_id: &str) -> Result<(), StorageError> {
    if !provider_id.is_empty()
        && provider_id.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
        })
    {
        Ok(())
    } else {
        Err(StorageError::new(
            "CODEX_PROVIDER_ID_INVALID",
            "provider id must be a path-safe ASCII identifier",
        ))
    }
}

fn validate_reasoning_effort(effort: &str) -> Result<(), StorageError> {
    if matches!(
        effort,
        "none" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max" | "ultra"
    ) {
        Ok(())
    } else {
        Err(StorageError::new(
            "CODEX_REASONING_EFFORT_UNSUPPORTED",
            format!("unsupported reasoning effort: {effort}"),
        ))
    }
}

fn validate_non_empty(field: &str, value: &str) -> Result<(), StorageError> {
    if value.trim().is_empty() {
        Err(StorageError::new(
            "CODEX_MANAGED_FIELD_EMPTY",
            format!("{field} must not be empty"),
        ))
    } else {
        Ok(())
    }
}

fn validate_profile_name(name: &str) -> Result<String, StorageError> {
    let name = name.trim();
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || name.contains(':')
        || name.chars().any(|character| character.is_control())
    {
        return Err(StorageError::new(
            "CODEX_PROFILE_NAME_INVALID",
            "Profile name must be a non-empty path-safe name",
        ));
    }
    Ok(name.to_owned())
}

fn settings_scope(target: &RuntimeTarget, path: &Path) -> CodexSettingsScope {
    if path == codex_base_config_path(target) {
        CodexSettingsScope::User
    } else if path.starts_with(codex_directory(target)) && is_codex_profile_file(path) {
        CodexSettingsScope::Profile
    } else {
        CodexSettingsScope::Unknown
    }
}

fn profile_id_for_path(path: &Path, scope: CodexSettingsScope) -> String {
    if scope == CodexSettingsScope::User {
        return "codex.user".to_owned();
    }
    format!(
        "codex.profile.{}",
        safe_component(&display_name_for_path(path, scope))
    )
}

fn display_name_for_path(path: &Path, scope: CodexSettingsScope) -> String {
    if scope == CodexSettingsScope::User {
        return "Codex 用户配置".to_owned();
    }
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("profile");
    name.strip_suffix(CODEX_PROFILE_SUFFIX)
        .unwrap_or(name)
        .to_owned()
}

fn safe_component(value: &str) -> String {
    let result: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    if result.is_empty() {
        "profile".to_owned()
    } else {
        result
    }
}

fn is_codex_profile_file(path: &Path) -> bool {
    path.is_file()
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                name.ends_with(CODEX_PROFILE_SUFFIX)
                    && name != CODEX_BASE_CONFIG
                    && !name.starts_with('.')
            })
}

fn path_for_index(path: &Path, target: &RuntimeTarget) -> String {
    path.strip_prefix(target.home_path.as_path())
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| path.to_string_lossy().into_owned())
}

fn read_profile_index(
    target: &RuntimeTarget,
) -> Result<(CodexProfileIndex, Option<DocumentRevision>), StorageError> {
    let path = codex_profile_index_path(target);
    if !path.is_file() {
        return Ok((CodexProfileIndex::default(), None));
    }
    let document = read_document(target, path)?;
    let value = match document.parse()? {
        ParsedConfig::Json(value) => value,
        ParsedConfig::Toml(_) => {
            return Err(StorageError::new(
                "CODEX_PROFILE_INDEX_INVALID",
                "Codex Profile index must be JSON",
            ))
        }
    };
    let index: CodexProfileIndex = serde_json::from_value(value)
        .map_err(|error| StorageError::new("CODEX_PROFILE_INDEX_INVALID", error.to_string()))?;
    if index.schema_version != CODEX_INDEX_SCHEMA_VERSION {
        return Err(StorageError::new(
            "CODEX_PROFILE_INDEX_VERSION_UNSUPPORTED",
            format!(
                "expected schema {}, found {}",
                CODEX_INDEX_SCHEMA_VERSION, index.schema_version
            ),
        ));
    }
    Ok((index, Some(document.revision)))
}

fn write_profile_index(
    target: &RuntimeTarget,
    index: &CodexProfileIndex,
    expected_revision: Option<&DocumentRevision>,
) -> Result<WriteReport, StorageError> {
    ensure_codex_directory(target)?;
    let bytes = serde_json::to_vec_pretty(index).map_err(|error| {
        StorageError::new("CODEX_PROFILE_INDEX_SERIALIZE_FAILED", error.to_string())
    })?;
    let path = codex_profile_index_path(target);
    if path.is_file() {
        write_document(target, path, expected_revision, &bytes)
    } else {
        write_new_document(target, &path, &bytes)
    }
}

fn write_new_document(
    target: &RuntimeTarget,
    path: &Path,
    content: &[u8],
) -> Result<WriteReport, StorageError> {
    if path.exists() {
        return Err(StorageError::new(
            "CONFIG_CREATE_CONFLICT",
            path.display().to_string(),
        ));
    }
    let parent = path
        .parent()
        .ok_or_else(|| StorageError::new("CONFIG_PARENT_INVALID", path.display().to_string()))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| StorageError::new("CONFIG_PATH_INVALID", path.display().to_string()))?;
    let temporary = parent.join(format!(".{}.vibehub.new", file_name.to_string_lossy()));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|error| StorageError::new("CONFIG_TEMP_CREATE_FAILED", error.to_string()))?;
    if let Err(error) = file.write_all(content).and_then(|_| file.sync_all()) {
        let _ = fs::remove_file(&temporary);
        return Err(StorageError::new(
            "CONFIG_TEMP_WRITE_FAILED",
            error.to_string(),
        ));
    }
    drop(file);
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(StorageError::new(
            "CONFIG_ATOMIC_REPLACE_FAILED",
            error.to_string(),
        ));
    }
    let document = read_document(target, path)?;
    Ok(WriteReport {
        path: super::agent_profile_storage::NativeConfigPath::from_path(path, target.platform),
        before_revision: None,
        after_revision: document.revision,
        backup_path: None,
        rollback_available: false,
    })
}

fn ensure_codex_directory(target: &RuntimeTarget) -> Result<(), StorageError> {
    let root = target.home_path.as_path();
    let directory = codex_directory(target);
    if !directory.starts_with(&root) {
        return Err(StorageError::new(
            "CODEX_PATH_OUTSIDE_RUNTIME_HOME",
            directory.display().to_string(),
        ));
    }
    if !root.is_dir() {
        return Err(StorageError::new(
            "RUNTIME_HOME_NOT_DIRECTORY",
            root.display().to_string(),
        ));
    }
    match fs::symlink_metadata(&directory) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(StorageError::new(
            "CODEX_DIRECTORY_LINK_REJECTED",
            directory.display().to_string(),
        )),
        Ok(metadata) if !metadata.is_dir() => Err(StorageError::new(
            "CODEX_DIRECTORY_NOT_DIRECTORY",
            directory.display().to_string(),
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => fs::create_dir(&directory)
            .map_err(|error| StorageError::new("CODEX_DIRECTORY_CREATE_FAILED", error.to_string())),
        Err(error) => Err(StorageError::new(
            "CODEX_DIRECTORY_READ_FAILED",
            error.to_string(),
        )),
    }
}

fn revision_for_bytes(raw: &[u8]) -> DocumentRevision {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(raw);
    DocumentRevision {
        content_sha256: format!("{:x}", hasher.finalize()),
        byte_length: raw.len() as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use uuid::Uuid;

    fn temp_target() -> (RuntimeTarget, PathBuf) {
        let root = env::temp_dir().join(format!("vibehub-codex-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        (RuntimeTarget::host(root.clone()), root)
    }

    #[test]
    fn discovers_profiles_and_hides_literal_credentials() {
        let (target, root) = temp_target();
        fs::create_dir_all(root.join(".codex")).unwrap();
        fs::write(
            root.join(".codex/config.toml"),
            r#"# keep root comment
model = "gpt-5.3-codex"
model_provider = "deepseek"
model_reasoning_effort = "high"
unknown_root = "keep"

[model_providers.deepseek]
name = "DeepSeek"
base_url = "https://api.deepseek.com/v1"
wire_api = "responses"
requires_openai_auth = false
api_key = "literal-secret"
unknown_provider = true

[projects."/tmp/project"]
trust_level = "trusted"
"#,
        )
        .unwrap();
        fs::write(
            root.join(".codex/deepseek.config.toml"),
            "model = \"deepseek-reasoner\"\nmodel_provider = \"deepseek\"\n[model_providers.deepseek]\nbase_url = \"https://api.deepseek.com/v1\"\nwire_api = \"responses\"\n",
        )
        .unwrap();
        let profiles = discover_codex_profiles(&target).unwrap();
        assert_eq!(profiles.len(), 2);
        let user = profiles
            .iter()
            .find(|profile| profile.scope == CodexSettingsScope::User)
            .unwrap();
        assert_eq!(user.model_provider.as_deref(), Some("deepseek"));
        assert_eq!(user.providers[0].protocol, CodexProtocol::OpenaiResponses);
        assert_eq!(
            user.providers[0].credential.kind,
            CodexCredentialKind::ConfigLiteral
        );
        assert!(!serde_json::to_string(user)
            .unwrap()
            .contains("literal-secret"));
        assert!(user
            .unknown_root_fields
            .contains(&"unknown_root".to_owned()));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn toml_patch_preserves_comments_unknown_fields_and_project_sections() {
        let (target, root) = temp_target();
        let path = root.join("config.toml");
        let original = r#"# top comment
model = "old" # inline comment
model_provider = "openai"
unknown_root = 42

[model_providers.openai]
name = "OpenAI"
base_url = "https://old.invalid/v1"
wire_api = "responses"
unknown_provider = true

[projects."/tmp/project"]
trust_level = "trusted"
"#;
        fs::write(&path, original).unwrap();
        let document = read_document(&target, &path).unwrap();
        let mut providers = BTreeMap::new();
        providers.insert(
            "openai".to_owned(),
            CodexProviderPatch {
                base_url: Some("https://new.invalid/v1".to_owned()),
                ..Default::default()
            },
        );
        save_codex_profile(
            &target,
            &path,
            Some(&document.revision),
            &CodexConfigPatch {
                model: Some("new-model".to_owned()),
                reasoning_effort: Some("xhigh".to_owned()),
                providers,
                ..Default::default()
            },
        )
        .unwrap();
        let updated = String::from_utf8(fs::read(&path).unwrap()).unwrap();
        assert!(updated.contains("# top comment"));
        assert!(updated.contains("# inline comment"));
        assert!(updated.contains("unknown_root = 42"));
        assert!(updated.contains("unknown_provider = true"));
        assert!(updated.contains("trust_level = \"trusted\""));
        assert!(updated.contains("model = \"new-model\""));
        assert!(updated.contains("base_url = \"https://new.invalid/v1\""));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn toml_patch_removes_provider_section_and_can_clear_selected_model() {
        let (target, root) = temp_target();
        let path = root.join("config.toml");
        fs::write(
            &path,
            "# keep\nmodel = \"old\"\nmodel_provider = \"remove-me\"\n[model_providers.remove-me]\nbase_url = \"https://remove.invalid\"\n[model_providers.keep-me]\nbase_url = \"https://keep.invalid\"\n",
        )
        .unwrap();
        let document = read_document(&target, &path).unwrap();
        save_codex_profile(
            &target,
            &path,
            Some(&document.revision),
            &CodexConfigPatch {
                clear_model: true,
                clear_model_provider: true,
                deleted_providers: vec!["remove-me".to_owned()],
                ..Default::default()
            },
        )
        .unwrap();
        let updated = String::from_utf8(fs::read(&path).unwrap()).unwrap();
        assert!(updated.contains("# keep"));
        assert!(!updated.contains("model ="));
        assert!(!updated.contains("model_provider ="));
        assert!(!updated.contains("model_providers.remove-me"));
        assert!(updated.contains("model_providers.keep-me"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn activation_projects_to_base_and_keeps_project_override_warning() {
        let (target, root) = temp_target();
        fs::create_dir_all(root.join(".codex")).unwrap();
        let base = root.join(".codex/config.toml");
        let profile = root.join(".codex/custom.config.toml");
        fs::write(&base, "# keep\nmodel = \"old\"\nunknown = true\n").unwrap();
        fs::write(
            &profile,
            "model = \"new\"\nmodel_reasoning_effort = \"high\"\n",
        )
        .unwrap();
        let operation = activate_codex_profile(&target, &profile).unwrap();
        assert!(operation.profile.default_state.is_default);
        assert_eq!(
            operation.profile.launch.as_ref().unwrap().arguments,
            vec!["--profile-v2", "custom"]
        );
        let updated = String::from_utf8(fs::read(&base).unwrap()).unwrap();
        assert!(updated.contains("model = \"new\""));
        assert!(updated.contains("model_reasoning_effort = \"high\""));
        assert!(updated.contains("unknown = true"));
        assert!(operation.profile.project_override_warning.is_some());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn profile_crud_and_explicit_default_clear_work_without_legacy_tables() {
        let (target, root) = temp_target();
        let one = create_codex_profile(&target, "one", None).unwrap();
        assert!(one.profile.source_path.exists());
        let two = clone_codex_profile(&target, &one.profile.source_path, "two").unwrap();
        let renamed = rename_codex_profile(&target, &two.profile.source_path, "three").unwrap();
        assert_eq!(renamed.display_name, "three");
        activate_codex_profile(&target, &one.profile.source_path).unwrap();
        clear_codex_default_profile(&target).unwrap();
        delete_codex_profile(
            &target,
            &CodexDeleteRequest {
                profile_path: one.profile.source_path.clone(),
                replacement_profile_path: None,
            },
        )
        .unwrap();
        assert!(!one.profile.source_path.exists());
        assert!(renamed.source_path.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn legacy_profiles_are_readonly_and_never_generated() {
        let (target, root) = temp_target();
        let path = root.join("config.toml");
        fs::write(&path, "[profiles.old]\nmodel = \"old\"\n").unwrap();
        let view = read_codex_profile(&target, &path).unwrap();
        assert!(view
            .warnings
            .contains(&"CODEX_LEGACY_PROFILES_UNSUPPORTED".to_owned()));
        let error = save_codex_profile(
            &target,
            &path,
            Some(&view.revision),
            &CodexConfigPatch {
                model: Some("new".to_owned()),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(error.code, "CODEX_LEGACY_PROFILES_UNSUPPORTED");
        assert_eq!(
            codex_profile_launch_args("custom").unwrap(),
            vec!["--profile-v2", "custom"]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn revision_conflict_and_validation_fail_closed() {
        let (target, root) = temp_target();
        let path = root.join("config.toml");
        fs::write(&path, "model = \"old\"\n").unwrap();
        let document = read_document(&target, &path).unwrap();
        fs::write(&path, "model = \"external\"\n").unwrap();
        let conflict = save_codex_profile(
            &target,
            &path,
            Some(&document.revision),
            &CodexConfigPatch {
                model: Some("new".to_owned()),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(conflict.code, "CONFIG_REVISION_CONFLICT");
        let invalid = save_codex_profile(
            &target,
            &path,
            None,
            &CodexConfigPatch {
                reasoning_effort: Some("unsupported".to_owned()),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(invalid.code, "CODEX_REASONING_EFFORT_UNSUPPORTED");
        assert_eq!(
            codex_profile_path(&target, "../escape").unwrap_err().code,
            "CODEX_PROFILE_NAME_INVALID"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn save_writes_bearer_token_removes_env_key_hides_secret_and_projects_on_activate() {
        let (target, root) = temp_target();
        fs::create_dir_all(root.join(".codex")).unwrap();
        let path = root.join(".codex/stepfun.config.toml");
        fs::write(
            &path,
            r#"model = "water18"
model_provider = "stepfun"

[model_providers.stepfun]
name = "StepFun"
base_url = "https://api.stepfun.com/v1"
wire_api = "responses"
env_key = "OPENAI_API_KEY"
"#,
        )
        .unwrap();
        let document = read_document(&target, &path).unwrap();
        let mut providers = BTreeMap::new();
        providers.insert(
            "stepfun".to_owned(),
            CodexProviderPatch {
                bearer_token: Some("codex-secret-value".to_owned()),
                requires_openai_auth: Some(false),
                clear_environment_key: true,
                ..Default::default()
            },
        );
        let patch = CodexConfigPatch {
            providers,
            ..Default::default()
        };
        let debug = format!("{patch:?}");
        assert!(debug.contains("[redacted]"));
        assert!(!debug.contains("codex-secret-value"));
        save_codex_profile(&target, &path, Some(&document.revision), &patch).unwrap();
        let raw = String::from_utf8(fs::read(&path).unwrap()).unwrap();
        assert!(raw.contains("experimental_bearer_token = \"codex-secret-value\""));
        assert!(!raw.contains("env_key"));
        let view = read_codex_profile(&target, &path).unwrap();
        assert_eq!(
            view.providers[0].credential.kind,
            CodexCredentialKind::ConfigLiteral
        );
        assert!(!serde_json::to_string(&view)
            .unwrap()
            .contains("codex-secret-value"));

        let base = root.join(".codex/config.toml");
        fs::write(
            &base,
            "model = \"old\"\nmodel_provider = \"openai\"\n\n[model_providers.openai]\nenv_key = \"OPENAI_API_KEY\"\n",
        )
        .unwrap();
        activate_codex_profile(&target, &path).unwrap();
        let projected = String::from_utf8(fs::read(&base).unwrap()).unwrap();
        let parsed: toml::Value = toml::from_str(&projected).unwrap();
        let stepfun = parsed["model_providers"]["stepfun"].as_table().unwrap();
        assert_eq!(
            stepfun["experimental_bearer_token"].as_str(),
            Some("codex-secret-value")
        );
        assert!(stepfun.get("env_key").is_none());
        assert_eq!(parsed["model"].as_str(), Some("water18"));

        let document = read_document(&target, &path).unwrap();
        let mut providers = BTreeMap::new();
        providers.insert(
            "stepfun".to_owned(),
            CodexProviderPatch {
                clear_bearer_token: true,
                ..Default::default()
            },
        );
        save_codex_profile(
            &target,
            &path,
            Some(&document.revision),
            &CodexConfigPatch {
                providers,
                ..Default::default()
            },
        )
        .unwrap();
        let cleared = String::from_utf8(fs::read(&path).unwrap()).unwrap();
        assert!(!cleared.contains("codex-secret-value"));
        assert!(!cleared.contains("experimental_bearer_token"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn patch_sets_and_clears_model_catalog_json() {
        let (target, root) = temp_target();
        let path = root.join("config.toml");
        fs::write(
            &path,
            "model = \"old\"\nmodel_catalog_json = \"openai.json\"\n",
        )
        .unwrap();

        let document = read_document(&target, &path).unwrap();
        save_codex_profile(
            &target,
            &path,
            Some(&document.revision),
            &CodexConfigPatch {
                model_catalog_json: Some("custom.json".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        let set = String::from_utf8(fs::read(&path).unwrap()).unwrap();
        assert!(set.contains("model_catalog_json = \"custom.json\""));
        assert!(!set.contains("openai.json"));

        let document = read_document(&target, &path).unwrap();
        save_codex_profile(
            &target,
            &path,
            Some(&document.revision),
            &CodexConfigPatch {
                clear_model_catalog_json: true,
                ..Default::default()
            },
        )
        .unwrap();
        let cleared = String::from_utf8(fs::read(&path).unwrap()).unwrap();
        assert!(!cleared.contains("model_catalog_json"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn activation_clears_catalog_for_third_party_provider_and_keeps_native() {
        let (target, root) = temp_target();
        fs::create_dir_all(root.join(".codex")).unwrap();
        let base = root.join(".codex/config.toml");
        // OpenAI-scoped catalog left over from a prior OpenAI selection.
        fs::write(
            &base,
            "model = \"gpt-5.6-sol\"\nmodel_provider = \"openai\"\nmodel_catalog_json = \"cockpit-local-access-model-catalog.json\"\n\n[model_providers.openai]\nname = \"OpenAI\"\n",
        )
        .unwrap();
        let third_party = root.join(".codex/stepfun.config.toml");
        fs::write(
            &third_party,
            "model = \"step-3.7-flash\"\nmodel_provider = \"stepfun\"\n\n[model_providers.stepfun]\nname = \"StepFun\"\nbase_url = \"https://api.stepfun.com/v1\"\nwire_api = \"responses\"\n",
        )
        .unwrap();
        activate_codex_profile(&target, &third_party).unwrap();
        let projected: toml::Value =
            toml::from_str(&String::from_utf8(fs::read(&base).unwrap()).unwrap()).unwrap();
        assert_eq!(projected["model"].as_str(), Some("step-3.7-flash"));
        assert_eq!(projected["model_provider"].as_str(), Some("stepfun"));
        // The OpenAI catalog must be gone so the picker stops listing GPT models.
        assert!(projected.get("model_catalog_json").is_none());

        // A native provider activation must NOT clear the catalog; it projects
        // the catalog the profile itself carries, proving native projection works.
        let native = root.join(".codex/openai.config.toml");
        fs::write(
            &native,
            "model = \"gpt-5.5\"\nmodel_provider = \"openai\"\nmodel_catalog_json = \"native-openai.json\"\n\n[model_providers.openai]\nname = \"OpenAI\"\n",
        )
        .unwrap();
        activate_codex_profile(&target, &native).unwrap();
        let projected: toml::Value =
            toml::from_str(&String::from_utf8(fs::read(&base).unwrap()).unwrap()).unwrap();
        assert_eq!(
            projected["model_catalog_json"].as_str(),
            Some("native-openai.json")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn create_profile_without_template_is_minimal_and_provider_consistent() {
        let (target, root) = temp_target();
        fs::create_dir_all(root.join(".codex")).unwrap();
        let base = root.join(".codex/config.toml");
        // Base carries a lot of cockpit clutter plus a native OpenAI provider.
        fs::write(
            &base,
            "model = \"gpt-5.6-sol\"\nmodel_provider = \"openai\"\nmodel_catalog_json = \"cockpit.json\"\nmodel_reasoning_effort = \"high\"\n\n[marketplaces.openai-bundled]\nsource_type = \"local\"\n\n[plugins.\"browser@openai-bundled\"]\nenabled = true\n\n[projects.\"/tmp/project\"]\ntrust_level = \"trusted\"\n\n[mcp_servers.node_repl]\ncommand = \"/bin/true\"\n\n[model_providers.openai]\nname = \"OpenAI\"\nbase_url = \"https://api.openai.com/v1\"\nwire_api = \"responses\"\n",
        )
        .unwrap();
        let operation = create_codex_profile(&target, "minimal", None).unwrap();
        let raw = String::from_utf8(fs::read(&operation.profile.source_path).unwrap()).unwrap();
        let parsed: toml::Value = toml::from_str(&raw).unwrap();

        // Managed selection is carried over.
        assert_eq!(parsed["model"].as_str(), Some("gpt-5.6-sol"));
        assert_eq!(parsed["model_provider"].as_str(), Some("openai"));
        assert_eq!(parsed["model_catalog_json"].as_str(), Some("cockpit.json"));
        assert_eq!(parsed["model_reasoning_effort"].as_str(), Some("high"));
        // The selected provider definition is kept.
        assert!(parsed["model_providers"]["openai"]["base_url"]
            .as_str()
            .is_some());
        // Cockpit clutter is dropped.
        assert!(parsed.get("marketplaces").is_none());
        assert!(parsed.get("plugins").is_none());
        assert!(parsed.get("projects").is_none());
        assert!(parsed.get("mcp_servers").is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn activation_rejects_model_provider_without_definition_via_warning() {
        let (target, root) = temp_target();
        fs::create_dir_all(root.join(".codex")).unwrap();
        let base = root.join(".codex/config.toml");
        fs::write(
            &base,
            "model = \"x\"\nmodel_provider = \"ghost\"\n\n[model_providers.other]\nname = \"Other\"\n",
        )
        .unwrap();
        let view = read_codex_profile(&target, &base).unwrap();
        assert!(view
            .warnings
            .iter()
            .any(|warning| warning.contains("ghost") && warning.contains("no matching")));
        fs::remove_dir_all(root).unwrap();
    }
}

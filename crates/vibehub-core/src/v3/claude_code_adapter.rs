use super::agent_profile_storage::{
    read_document, restore_document, write_document, AgentKind, ConfigDocument, ConfigFormat,
    DocumentRevision, ParsedConfig, RuntimeTarget, StorageError, WriteReport,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const CLAUDE_DIRECTORY: &str = ".claude";
const CLAUDE_SETTINGS_FILE: &str = "settings.json";
const CLAUDE_PROFILE_DIRECTORY: &str = "vibehub-profiles";
const CLAUDE_PROFILE_INDEX_FILE: &str = "index.json";
const CLAUDE_PROFILE_EXTENSION: &str = ".settings.json";
const CLAUDE_PROFILE_INDEX_SCHEMA_VERSION: u32 = 1;
const CLAUDE_EXECUTABLE: &str = "claude";

const MANAGED_SETTINGS_FIELDS: &[&str] =
    &["model", "alwaysThinkingEnabled", "env.ANTHROPIC_BASE_URL"];
const PRESERVED_SETTINGS_FIELDS: &[&str] = &[
    "permissions",
    "hooks",
    "mcpServers",
    "sandbox",
    "apiKeyHelper",
    "statusLine",
    "enabledPlugins",
    "includeCoAuthoredBy",
];
const SENSITIVE_ENV_NAMES: &[&str] = &[
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "OPENAI_API_KEY",
    "GOOGLE_API_KEY",
    "GEMINI_API_KEY",
    "DEEPSEEK_API_KEY",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaudeSettingsScope {
    User,
    Project,
    Local,
    Profile,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaudeCredentialKind {
    Environment,
    ConfigLiteral,
    CredentialHelper,
    None,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudeCredentialReference {
    pub kind: ClaudeCredentialKind,
    pub references: Vec<String>,
    pub display: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudeThinkingProfile {
    pub enabled: Option<bool>,
    pub selected: Option<String>,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudeLaunchSpec {
    pub executable: String,
    pub settings_argument: String,
    pub arguments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudeDefaultState {
    pub is_default: bool,
    pub selected_by: ClaudeDefaultSelector,
    pub projection_target: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaudeDefaultSelector {
    Vibehub,
    Native,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudeCodeProfileView {
    pub agent: AgentKind,
    pub profile_id: String,
    pub display_name: String,
    pub source_path: PathBuf,
    pub scope: ClaudeSettingsScope,
    pub format: ConfigFormat,
    pub revision: DocumentRevision,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub credential: ClaudeCredentialReference,
    pub thinking: ClaudeThinkingProfile,
    pub default_state: ClaudeDefaultState,
    pub launch: ClaudeLaunchSpec,
    pub managed_fields: Vec<String>,
    pub preserved_fields: Vec<String>,
    pub unknown_fields: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ClaudeSettingsPatch {
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub thinking_enabled: Option<bool>,
    pub clear_model: bool,
    pub clear_base_url: bool,
    pub clear_thinking: bool,
    /// This is metadata about an externally supplied environment variable. A
    /// secret value is never written to settings.json. The adapter only
    /// accepts references which are already represented by the settings file;
    /// launching with a new reference is owned by the runtime/secret layer.
    pub credential_environment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudeProfileOperation {
    pub profile: ClaudeCodeProfileView,
    pub write: Option<WriteReport>,
    pub index_write: Option<WriteReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudeDeleteRequest {
    pub profile_path: PathBuf,
    pub replacement_profile_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudeProfileIndexView {
    pub default_profile_id: Option<String>,
    pub profiles: BTreeMap<String, String>,
    pub revision: Option<DocumentRevision>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ClaudeProfileIndex {
    schema_version: u32,
    default_profile_id: Option<String>,
    profiles: BTreeMap<String, String>,
}

impl Default for ClaudeProfileIndex {
    fn default() -> Self {
        Self {
            schema_version: CLAUDE_PROFILE_INDEX_SCHEMA_VERSION,
            default_profile_id: None,
            profiles: BTreeMap::new(),
        }
    }
}

pub fn claude_user_settings_path(target: &RuntimeTarget) -> PathBuf {
    target
        .home_path
        .as_path()
        .join(CLAUDE_DIRECTORY)
        .join(CLAUDE_SETTINGS_FILE)
}

pub fn claude_profile_directory(target: &RuntimeTarget) -> PathBuf {
    target
        .home_path
        .as_path()
        .join(CLAUDE_DIRECTORY)
        .join(CLAUDE_PROFILE_DIRECTORY)
}

pub fn claude_profile_index_path(target: &RuntimeTarget) -> PathBuf {
    claude_profile_directory(target).join(CLAUDE_PROFILE_INDEX_FILE)
}

pub fn claude_profile_path(
    target: &RuntimeTarget,
    profile_name: &str,
) -> Result<PathBuf, StorageError> {
    let safe_name = validate_profile_name(profile_name)?;
    Ok(claude_profile_directory(target).join(format!("{safe_name}{CLAUDE_PROFILE_EXTENSION}")))
}

pub fn claude_code_launch_args(path: impl AsRef<Path>) -> Result<Vec<String>, StorageError> {
    let path = path.as_ref();
    if !path.is_absolute() {
        return Err(StorageError::new(
            "CLAUDE_SETTINGS_PATH_NOT_ABSOLUTE",
            path.display().to_string(),
        ));
    }
    Ok(vec![
        "--settings".to_owned(),
        path.to_string_lossy().into_owned(),
    ])
}

pub fn claude_code_launch_spec(path: impl AsRef<Path>) -> Result<ClaudeLaunchSpec, StorageError> {
    Ok(ClaudeLaunchSpec {
        executable: CLAUDE_EXECUTABLE.to_owned(),
        settings_argument: "--settings".to_owned(),
        arguments: claude_code_launch_args(path)?,
    })
}

pub fn claude_profile_index(
    target: &RuntimeTarget,
) -> Result<ClaudeProfileIndexView, StorageError> {
    let (index, revision) = read_profile_index(target)?;
    Ok(ClaudeProfileIndexView {
        default_profile_id: index.default_profile_id,
        profiles: index.profiles,
        revision,
    })
}

pub fn discover_claude_profiles(
    target: &RuntimeTarget,
) -> Result<Vec<ClaudeCodeProfileView>, StorageError> {
    let (index, _) = read_profile_index(target)?;
    let mut paths = Vec::new();
    let user_path = claude_user_settings_path(target);
    if user_path.is_file() {
        paths.push(user_path);
    }
    let profile_dir = claude_profile_directory(target);
    if profile_dir.is_dir() {
        let metadata = fs::symlink_metadata(&profile_dir).map_err(|error| {
            StorageError::new("CLAUDE_PROFILE_DIRECTORY_READ_FAILED", error.to_string())
        })?;
        if metadata.file_type().is_symlink() {
            return Err(StorageError::new(
                "CLAUDE_PROFILE_DIRECTORY_LINK_REJECTED",
                profile_dir.display().to_string(),
            ));
        }
        let entries = fs::read_dir(&profile_dir).map_err(|error| {
            StorageError::new("CLAUDE_PROFILE_DIRECTORY_READ_FAILED", error.to_string())
        })?;
        for entry in entries {
            let entry = entry.map_err(|error| {
                StorageError::new("CLAUDE_PROFILE_DIRECTORY_READ_FAILED", error.to_string())
            })?;
            let path = entry.path();
            if path.file_name().and_then(|name| name.to_str()) == Some(CLAUDE_PROFILE_INDEX_FILE)
                || !is_claude_profile_file(&path)
            {
                continue;
            }
            paths.push(path);
        }
    }
    paths.sort();
    paths
        .into_iter()
        .map(|path| read_claude_profile_with_index(target, path, &index))
        .collect()
}

pub fn read_claude_profile(
    target: &RuntimeTarget,
    path: impl AsRef<Path>,
) -> Result<ClaudeCodeProfileView, StorageError> {
    let (index, _) = read_profile_index(target)?;
    read_claude_profile_with_index(target, path.as_ref().to_path_buf(), &index)
}

pub fn save_claude_profile(
    target: &RuntimeTarget,
    path: impl AsRef<Path>,
    expected_revision: Option<&DocumentRevision>,
    patch: &ClaudeSettingsPatch,
) -> Result<WriteReport, StorageError> {
    let document = read_document(target, path)?;
    let edited = apply_settings_patch(&document, patch)?;
    write_document(target, document.path, expected_revision, &edited)
}

pub fn import_claude_profile(
    target: &RuntimeTarget,
    source: impl AsRef<Path>,
    profile_name: &str,
) -> Result<ClaudeProfileOperation, StorageError> {
    let source = source.as_ref();
    let source_document = read_document(target, source)?;
    let destination = claude_profile_path(target, profile_name)?;
    ensure_claude_profile_directory(target)?;
    if destination.exists() {
        return Err(StorageError::new(
            "CLAUDE_PROFILE_ALREADY_EXISTS",
            destination.display().to_string(),
        ));
    }
    let write = write_new_document(target, &destination, &source_document.raw)?;
    let profile = read_claude_profile(target, &destination)?;
    Ok(ClaudeProfileOperation {
        profile,
        write: Some(write),
        index_write: None,
    })
}

pub fn create_claude_profile(
    target: &RuntimeTarget,
    profile_name: &str,
    template: Option<&Path>,
) -> Result<ClaudeProfileOperation, StorageError> {
    let template = template.map(Path::to_path_buf);
    let source = match template {
        Some(path) => read_document(target, path)?.raw,
        None => {
            let user_path = claude_user_settings_path(target);
            if user_path.is_file() {
                read_document(target, user_path)?.raw
            } else {
                br#"{}"#.to_vec()
            }
        }
    };
    let destination = claude_profile_path(target, profile_name)?;
    ensure_claude_profile_directory(target)?;
    if destination.exists() {
        return Err(StorageError::new(
            "CLAUDE_PROFILE_ALREADY_EXISTS",
            destination.display().to_string(),
        ));
    }
    let write = write_new_document(target, &destination, &source)?;
    let profile = read_claude_profile(target, &destination)?;
    Ok(ClaudeProfileOperation {
        profile,
        write: Some(write),
        index_write: None,
    })
}

pub fn clone_claude_profile(
    target: &RuntimeTarget,
    source: impl AsRef<Path>,
    profile_name: &str,
) -> Result<ClaudeProfileOperation, StorageError> {
    import_claude_profile(target, source, profile_name)
}

pub fn rename_claude_profile(
    target: &RuntimeTarget,
    source: impl AsRef<Path>,
    new_name: &str,
) -> Result<ClaudeCodeProfileView, StorageError> {
    let source = source.as_ref().to_path_buf();
    let document = read_document(target, &source)?;
    let destination = claude_profile_path(target, new_name)?;
    if destination.exists() {
        return Err(StorageError::new(
            "CLAUDE_PROFILE_ALREADY_EXISTS",
            destination.display().to_string(),
        ));
    }
    fs::rename(&source, &destination)
        .map_err(|error| StorageError::new("CLAUDE_PROFILE_RENAME_FAILED", error.to_string()))?;
    let (mut index, index_revision) = read_profile_index(target)?;
    let old_id = profile_id_for_path(&source, ClaudeSettingsScope::Profile);
    let new_id = profile_id_for_path(&destination, ClaudeSettingsScope::Profile);
    if index.default_profile_id.as_deref() == Some(&old_id) {
        index.default_profile_id = Some(new_id.clone());
    }
    if index.profiles.remove(&old_id).is_some() {
        index
            .profiles
            .insert(new_id, path_for_index(&destination, target));
    }
    let _ = write_profile_index(target, &index, index_revision.as_ref())?;
    Ok(read_claude_profile(target, destination).map(|mut view| {
        view.revision = document.revision;
        view
    })?)
}

pub fn delete_claude_profile(
    target: &RuntimeTarget,
    request: &ClaudeDeleteRequest,
) -> Result<ClaudeProfileOperation, StorageError> {
    let source = request.profile_path.as_path();
    let current = read_claude_profile(target, source)?;
    if current.scope != ClaudeSettingsScope::Profile {
        return Err(StorageError::new(
            "CLAUDE_PROFILE_DELETE_SCOPE_INVALID",
            "only VibeHub Profile files can be deleted",
        ));
    }
    let (mut index, mut index_revision) = read_profile_index(target)?;
    let is_default = index.default_profile_id.as_deref() == Some(&current.profile_id);
    let mut replacement_operation = None;
    if is_default {
        let replacement = request.replacement_profile_path.as_ref().ok_or_else(|| {
            StorageError::new(
                "CLAUDE_DEFAULT_PROFILE_REPLACEMENT_REQUIRED",
                "the default Profile must be replaced or explicitly retained before deletion",
            )
        })?;
        if replacement == source {
            return Err(StorageError::new(
                "CLAUDE_DEFAULT_PROFILE_REPLACEMENT_INVALID",
                "a Profile cannot replace itself",
            ));
        }
        replacement_operation = Some(activate_claude_profile(target, replacement)?);
        (index, index_revision) = read_profile_index(target)?;
        index.default_profile_id = replacement_operation
            .as_ref()
            .map(|operation| operation.profile.profile_id.clone());
    }
    fs::remove_file(source)
        .map_err(|error| StorageError::new("CLAUDE_PROFILE_DELETE_FAILED", error.to_string()))?;
    index.profiles.remove(&current.profile_id);
    let index_write = write_profile_index(target, &index, index_revision.as_ref())?;
    Ok(ClaudeProfileOperation {
        profile: replacement_operation
            .map(|operation| operation.profile)
            .unwrap_or(current),
        write: None,
        index_write: Some(index_write),
    })
}

pub fn activate_claude_profile(
    target: &RuntimeTarget,
    profile_path: impl AsRef<Path>,
) -> Result<ClaudeProfileOperation, StorageError> {
    let profile_path = profile_path.as_ref();
    let profile = read_claude_profile(target, profile_path)?;
    if profile.scope != ClaudeSettingsScope::Profile {
        return Err(StorageError::new(
            "CLAUDE_PROFILE_ACTIVATE_SCOPE_INVALID",
            "only a VibeHub Profile can be activated",
        ));
    }
    let user_path = claude_user_settings_path(target);
    ensure_claude_directory(target)?;
    let user_document = if user_path.is_file() {
        Some(read_document(target, &user_path)?)
    } else {
        None
    };
    let source_document = read_document(target, profile_path)?;
    let source_patch = managed_patch_from_document(&source_document)?;
    let edited = match user_document.as_ref() {
        Some(document) => apply_settings_patch(document, &source_patch)?,
        None => apply_settings_patch_bytes(br#"{}"#, ConfigFormat::Json, &source_patch)?,
    };
    let user_write = match user_document.as_ref() {
        Some(document) => write_document(target, &user_path, Some(&document.revision), &edited)?,
        None => write_new_document(target, &user_path, &edited)?,
    };

    let (mut index, index_revision) = read_profile_index(target)?;
    index.default_profile_id = Some(profile.profile_id.clone());
    index.profiles.insert(
        profile.profile_id.clone(),
        path_for_index(profile_path, target),
    );
    let index_write = match write_profile_index(target, &index, index_revision.as_ref()) {
        Ok(write) => write,
        Err(error) => {
            if let Some(backup) = user_write.backup_path.as_ref() {
                let _ = restore_document(
                    target,
                    &user_path,
                    backup.as_path(),
                    &user_write.after_revision,
                );
            } else {
                let _ = fs::remove_file(&user_path);
            }
            return Err(StorageError::new(
                "CLAUDE_DEFAULT_PROJECTION_ROLLED_BACK",
                format!("default Profile index write failed: {error}"),
            ));
        }
    };
    let refreshed = read_claude_profile(target, profile_path)?;
    Ok(ClaudeProfileOperation {
        profile: refreshed,
        write: Some(user_write),
        index_write: Some(index_write),
    })
}

pub fn clear_claude_default_profile(target: &RuntimeTarget) -> Result<WriteReport, StorageError> {
    let (mut index, index_revision) = read_profile_index(target)?;
    index.default_profile_id = None;
    write_profile_index(target, &index, index_revision.as_ref())
}

fn read_claude_profile_with_index(
    target: &RuntimeTarget,
    path: PathBuf,
    index: &ClaudeProfileIndex,
) -> Result<ClaudeCodeProfileView, StorageError> {
    let document = read_document(target, &path)?;
    let root = settings_value(&document)?;
    let object = root.as_object().ok_or_else(|| {
        StorageError::new(
            "CLAUDE_SETTINGS_ROOT_INVALID",
            "Claude Code settings root must be an object",
        )
    })?;
    let scope = settings_scope(target, &path);
    let profile_id = profile_id_for_path(&path, scope);
    let display_name = display_name_for_path(&path, scope);
    let model = object
        .get("model")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let thinking_enabled = object.get("alwaysThinkingEnabled").and_then(Value::as_bool);
    let env = object.get("env").and_then(Value::as_object);
    let base_url = env
        .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    let (credential, mut warnings) = credential_reference(object, env);
    if document.format == ConfigFormat::Jsonc {
        warnings.push("CLAUDE_JSONC_SAVE_REWRITES_FORMAT_UNSUPPORTED".to_owned());
    }
    let unknown_fields = object
        .keys()
        .filter(|key| {
            !matches!(key.as_str(), "model" | "alwaysThinkingEnabled" | "env")
                && !PRESERVED_SETTINGS_FIELDS.contains(&key.as_str())
        })
        .cloned()
        .collect::<Vec<_>>();
    let preserved_fields = object
        .keys()
        .filter(|key| PRESERVED_SETTINGS_FIELDS.contains(&key.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let is_native_default = scope == ClaudeSettingsScope::User;
    let is_vibehub_default = index.default_profile_id.as_deref() == Some(&profile_id);
    let (is_default, selected_by) = if is_vibehub_default {
        (true, ClaudeDefaultSelector::Vibehub)
    } else if is_native_default {
        (true, ClaudeDefaultSelector::Native)
    } else {
        (false, ClaudeDefaultSelector::Unknown)
    };
    Ok(ClaudeCodeProfileView {
        agent: AgentKind::ClaudeCode,
        profile_id,
        display_name,
        source_path: path.clone(),
        scope,
        format: document.format,
        revision: document.revision,
        model,
        base_url,
        credential,
        thinking: ClaudeThinkingProfile {
            enabled: thinking_enabled,
            selected: thinking_enabled
                .map(|enabled| if enabled { "enabled" } else { "disabled" }.to_owned()),
            options: vec!["enabled".to_owned(), "disabled".to_owned()],
        },
        default_state: ClaudeDefaultState {
            is_default,
            selected_by,
            projection_target: claude_user_settings_path(target),
        },
        launch: claude_code_launch_spec(&path)?,
        managed_fields: MANAGED_SETTINGS_FIELDS
            .iter()
            .map(|field| (*field).to_owned())
            .collect(),
        preserved_fields,
        unknown_fields,
        warnings,
    })
}

fn settings_value(document: &ConfigDocument) -> Result<Value, StorageError> {
    match document.parse()? {
        ParsedConfig::Json(value) => Ok(value),
        ParsedConfig::Toml(_) => Err(StorageError::new(
            "CLAUDE_SETTINGS_FORMAT_UNSUPPORTED",
            "Claude Code settings must be JSON or JSONC",
        )),
    }
}

fn apply_settings_patch(
    document: &ConfigDocument,
    patch: &ClaudeSettingsPatch,
) -> Result<Vec<u8>, StorageError> {
    if document.format == ConfigFormat::Jsonc {
        return Err(StorageError::new(
            "CLAUDE_JSONC_WRITE_UNSUPPORTED",
            "Claude Code JSONC can be inspected, but saving it is disabled until comment-preserving spans are available",
        ));
    }
    apply_settings_patch_bytes(&document.raw, document.format, patch)
}

fn apply_settings_patch_bytes(
    raw: &[u8],
    format: ConfigFormat,
    patch: &ClaudeSettingsPatch,
) -> Result<Vec<u8>, StorageError> {
    if matches!(format, ConfigFormat::Toml | ConfigFormat::Jsonc) {
        return Err(StorageError::new(
            "CLAUDE_SETTINGS_FORMAT_UNSUPPORTED",
            "Claude Code settings writes require JSON; JSONC is read-only until comment-preserving spans are available",
        ));
    }
    if patch
        .credential_environment
        .as_deref()
        .is_some_and(|name| !is_environment_name(name))
    {
        return Err(StorageError::new(
            "CLAUDE_CREDENTIAL_REFERENCE_INVALID",
            "credential reference must be an environment variable name",
        ));
    }
    let mut root = match format.validate_for_claude(raw)? {
        Value::Object(object) => object,
        _ => {
            return Err(StorageError::new(
                "CLAUDE_SETTINGS_ROOT_INVALID",
                "Claude Code settings root must be an object",
            ))
        }
    };
    if let Some(model) = patch.model.as_deref() {
        validate_non_empty("model", model)?;
        root.insert("model".to_owned(), Value::String(model.to_owned()));
    } else if patch.clear_model {
        root.remove("model");
    }
    if let Some(enabled) = patch.thinking_enabled {
        root.insert("alwaysThinkingEnabled".to_owned(), Value::Bool(enabled));
    } else if patch.clear_thinking {
        root.remove("alwaysThinkingEnabled");
    }
    if patch.base_url.is_some() || patch.clear_base_url {
        let mut env = match root.remove("env") {
            Some(Value::Object(env)) => env,
            Some(other) => {
                return Err(StorageError::new(
                    "CLAUDE_SETTINGS_ENV_INVALID",
                    format!("env must be an object, found {other}"),
                ))
            }
            None => Map::new(),
        };
        if let Some(base_url) = patch.base_url.as_deref() {
            validate_non_empty("base_url", base_url)?;
            env.insert(
                "ANTHROPIC_BASE_URL".to_owned(),
                Value::String(base_url.to_owned()),
            );
        } else if patch.clear_base_url {
            env.remove("ANTHROPIC_BASE_URL");
        }
        root.insert("env".to_owned(), Value::Object(env));
    }
    serde_json::to_vec_pretty(&Value::Object(root))
        .map(|mut bytes| {
            bytes.push(b'\n');
            bytes
        })
        .map_err(|error| StorageError::new("CLAUDE_SETTINGS_SERIALIZE_FAILED", error.to_string()))
}

fn managed_patch_from_document(
    document: &ConfigDocument,
) -> Result<ClaudeSettingsPatch, StorageError> {
    let root = settings_value(document)?;
    let object = root.as_object().ok_or_else(|| {
        StorageError::new(
            "CLAUDE_SETTINGS_ROOT_INVALID",
            "Claude Code settings root must be an object",
        )
    })?;
    let env = object.get("env").and_then(Value::as_object);
    Ok(ClaudeSettingsPatch {
        model: object
            .get("model")
            .and_then(Value::as_str)
            .map(str::to_owned),
        base_url: env
            .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        thinking_enabled: object.get("alwaysThinkingEnabled").and_then(Value::as_bool),
        ..Default::default()
    })
}

fn credential_reference(
    object: &Map<String, Value>,
    env: Option<&Map<String, Value>>,
) -> (ClaudeCredentialReference, Vec<String>) {
    let mut warnings = Vec::new();
    let mut references = Vec::new();
    let mut literal = false;
    if let Some(env) = env {
        for (name, value) in env {
            if !is_credential_environment_name(name) {
                continue;
            }
            references.push(name.clone());
            match value {
                Value::String(value) if !value.is_empty() && !looks_like_reference(value) => {
                    literal = true;
                }
                Value::String(_) => {}
                _ => warnings.push("CLAUDE_CREDENTIAL_ENV_VALUE_INVALID".to_owned()),
            }
        }
    }
    if object.get("apiKeyHelper").is_some() {
        warnings.push("CLAUDE_API_KEY_HELPER_PRESENT_VALUE_HIDDEN".to_owned());
    }
    if literal {
        warnings.push("CLAUDE_LITERAL_CREDENTIAL_PRESENT_VALUE_HIDDEN".to_owned());
        return (
            ClaudeCredentialReference {
                kind: ClaudeCredentialKind::ConfigLiteral,
                references,
                display: "配置内存在凭据（已隐藏）".to_owned(),
            },
            warnings,
        );
    }
    if !references.is_empty() {
        return (
            ClaudeCredentialReference {
                kind: ClaudeCredentialKind::Environment,
                display: format!("环境变量 {}", references.join(", ")),
                references,
            },
            warnings,
        );
    }
    if object.get("apiKeyHelper").is_some() {
        return (
            ClaudeCredentialReference {
                kind: ClaudeCredentialKind::CredentialHelper,
                references: vec!["apiKeyHelper".to_owned()],
                display: "Claude Code credential helper（已隐藏）".to_owned(),
            },
            warnings,
        );
    }
    (
        ClaudeCredentialReference {
            kind: ClaudeCredentialKind::Unknown,
            references: Vec::new(),
            display: "由 Claude Code 运行时决定".to_owned(),
        },
        warnings,
    )
}

fn read_profile_index(
    target: &RuntimeTarget,
) -> Result<(ClaudeProfileIndex, Option<DocumentRevision>), StorageError> {
    let path = claude_profile_index_path(target);
    if !path.is_file() {
        return Ok((ClaudeProfileIndex::default(), None));
    }
    let document = read_document(target, path)?;
    let value = settings_value(&document)?;
    let index: ClaudeProfileIndex = serde_json::from_value(value)
        .map_err(|error| StorageError::new("CLAUDE_PROFILE_INDEX_INVALID", error.to_string()))?;
    if index.schema_version != CLAUDE_PROFILE_INDEX_SCHEMA_VERSION {
        return Err(StorageError::new(
            "CLAUDE_PROFILE_INDEX_VERSION_UNSUPPORTED",
            format!(
                "expected schema {}, found {}",
                CLAUDE_PROFILE_INDEX_SCHEMA_VERSION, index.schema_version
            ),
        ));
    }
    Ok((index, Some(document.revision)))
}

fn write_profile_index(
    target: &RuntimeTarget,
    index: &ClaudeProfileIndex,
    expected_revision: Option<&DocumentRevision>,
) -> Result<WriteReport, StorageError> {
    ensure_claude_profile_directory(target)?;
    let bytes = serde_json::to_vec_pretty(index).map_err(|error| {
        StorageError::new("CLAUDE_PROFILE_INDEX_SERIALIZE_FAILED", error.to_string())
    })?;
    let path = claude_profile_index_path(target);
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
    read_document(target, path).map(|document| WriteReport {
        path: super::agent_profile_storage::NativeConfigPath::from_path(path, target.platform),
        before_revision: None,
        after_revision: document.revision,
        backup_path: None,
        rollback_available: false,
    })
}

fn ensure_claude_directory(target: &RuntimeTarget) -> Result<(), StorageError> {
    ensure_directory_component(
        &target.home_path.as_path(),
        &target.home_path.as_path().join(CLAUDE_DIRECTORY),
    )
}

fn ensure_claude_profile_directory(target: &RuntimeTarget) -> Result<(), StorageError> {
    ensure_claude_directory(target)?;
    ensure_directory_component(
        target.home_path.as_path().join(CLAUDE_DIRECTORY).as_path(),
        &claude_profile_directory(target),
    )
}

fn ensure_directory_component(root: &Path, path: &Path) -> Result<(), StorageError> {
    if !path.starts_with(root) {
        return Err(StorageError::new(
            "CLAUDE_PATH_OUTSIDE_RUNTIME_HOME",
            path.display().to_string(),
        ));
    }
    let relative = path.strip_prefix(root).map_err(|_| {
        StorageError::new(
            "CLAUDE_PATH_OUTSIDE_RUNTIME_HOME",
            path.display().to_string(),
        )
    })?;
    let mut cursor = root.to_path_buf();
    for component in relative.components() {
        cursor.push(component.as_os_str());
        match fs::symlink_metadata(&cursor) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(StorageError::new(
                    "CLAUDE_PATH_LINK_REJECTED",
                    cursor.display().to_string(),
                ))
            }
            Ok(metadata) if !metadata.is_dir() => {
                return Err(StorageError::new(
                    "CLAUDE_PATH_PARENT_NOT_DIRECTORY",
                    cursor.display().to_string(),
                ))
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&cursor).map_err(|error| {
                    StorageError::new("CLAUDE_DIRECTORY_CREATE_FAILED", error.to_string())
                })?;
            }
            Err(error) => {
                return Err(StorageError::new(
                    "CLAUDE_DIRECTORY_READ_FAILED",
                    error.to_string(),
                ))
            }
        }
    }
    Ok(())
}

fn settings_scope(target: &RuntimeTarget, path: &Path) -> ClaudeSettingsScope {
    if path == claude_user_settings_path(target) {
        ClaudeSettingsScope::User
    } else if path.starts_with(claude_profile_directory(target)) {
        ClaudeSettingsScope::Profile
    } else if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "settings.local.json")
    {
        ClaudeSettingsScope::Local
    } else {
        ClaudeSettingsScope::Unknown
    }
}

fn profile_id_for_path(path: &Path, scope: ClaudeSettingsScope) -> String {
    if scope == ClaudeSettingsScope::User {
        return "claude.user".to_owned();
    }
    let name = display_name_for_path(path, scope);
    format!("claude.profile.{}", safe_component(&name))
}

fn display_name_for_path(path: &Path, scope: ClaudeSettingsScope) -> String {
    if scope == ClaudeSettingsScope::User {
        return "Claude Code 用户设置".to_owned();
    }
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("profile");
    file_name
        .strip_suffix(".settings.json")
        .or_else(|| file_name.strip_suffix(".settings.jsonc"))
        .or_else(|| file_name.strip_suffix(".json"))
        .unwrap_or(file_name)
        .to_owned()
}

fn path_for_index(path: &Path, target: &RuntimeTarget) -> String {
    path.strip_prefix(target.home_path.as_path())
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| path.to_string_lossy().into_owned())
}

fn is_claude_profile_file(path: &Path) -> bool {
    path.is_file()
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                name.ends_with(".settings.json") || name.ends_with(".settings.jsonc")
            })
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
            "CLAUDE_PROFILE_NAME_INVALID",
            "Profile name must be a non-empty path-safe name",
        ));
    }
    Ok(name.to_owned())
}

fn safe_component(value: &str) -> String {
    let mut result = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
            result.push(character.to_ascii_lowercase());
        } else {
            result.push('-');
        }
    }
    if result.is_empty() {
        "profile".to_owned()
    } else {
        result
    }
}

fn validate_non_empty(field: &str, value: &str) -> Result<(), StorageError> {
    if value.trim().is_empty() {
        return Err(StorageError::new(
            "CLAUDE_MANAGED_FIELD_EMPTY",
            format!("{field} must not be empty"),
        ));
    }
    Ok(())
}

fn is_environment_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().enumerate().all(|(index, character)| {
            character.is_ascii_alphabetic()
                || character == '_'
                || (index > 0 && character.is_ascii_digit())
        })
}

fn is_credential_environment_name(name: &str) -> bool {
    SENSITIVE_ENV_NAMES.contains(&name)
        || name.ends_with("_API_KEY")
        || name.ends_with("_AUTH_TOKEN")
        || name.ends_with("_TOKEN")
}

fn looks_like_reference(value: &str) -> bool {
    let value = value.trim();
    value.starts_with("${") && value.ends_with('}')
}

trait ClaudeConfigFormatExt {
    fn validate_for_claude(self, raw: &[u8]) -> Result<Value, StorageError>;
}

impl ClaudeConfigFormatExt for ConfigFormat {
    fn validate_for_claude(self, raw: &[u8]) -> Result<Value, StorageError> {
        let text = std::str::from_utf8(raw).map_err(|error| {
            StorageError::new("CLAUDE_SETTINGS_ENCODING_INVALID", error.to_string())
        })?;
        match self {
            ConfigFormat::Json => serde_json::from_str(text).map_err(|error| {
                StorageError::new("CLAUDE_SETTINGS_JSON_INVALID", error.to_string())
            }),
            ConfigFormat::Jsonc => {
                let normalized = strip_jsonc_comments_and_trailing_commas(text)?;
                serde_json::from_str(&normalized).map_err(|error| {
                    StorageError::new("CLAUDE_SETTINGS_JSONC_INVALID", error.to_string())
                })
            }
            ConfigFormat::Toml => Err(StorageError::new(
                "CLAUDE_SETTINGS_FORMAT_UNSUPPORTED",
                "Claude Code settings must be JSON or JSONC",
            )),
        }
    }
}

fn strip_jsonc_comments_and_trailing_commas(input: &str) -> Result<String, StorageError> {
    let chars: Vec<char> = input.chars().collect();
    let mut output = String::with_capacity(input.len());
    let mut in_string = false;
    let mut escaped = false;
    let mut index = 0;
    while index < chars.len() {
        let current = chars[index];
        if in_string {
            output.push(current);
            if escaped {
                escaped = false;
            } else if current == '\\' {
                escaped = true;
            } else if current == '"' {
                in_string = false;
            }
            index += 1;
            continue;
        }
        if current == '"' {
            in_string = true;
            output.push(current);
            index += 1;
            continue;
        }
        if current == '/' && chars.get(index + 1) == Some(&'/') {
            output.push(' ');
            output.push(' ');
            index += 2;
            while index < chars.len() && chars[index] != '\n' {
                output.push(' ');
                index += 1;
            }
            continue;
        }
        if current == '/' && chars.get(index + 1) == Some(&'*') {
            output.push(' ');
            output.push(' ');
            index += 2;
            let mut closed = false;
            while index < chars.len() {
                if chars[index] == '*' && chars.get(index + 1) == Some(&'/') {
                    output.push(' ');
                    output.push(' ');
                    index += 2;
                    closed = true;
                    break;
                }
                output.push(if chars[index] == '\n' { '\n' } else { ' ' });
                index += 1;
            }
            if !closed {
                return Err(StorageError::new(
                    "CLAUDE_SETTINGS_JSONC_COMMENT_UNTERMINATED",
                    "unterminated JSONC block comment",
                ));
            }
            continue;
        }
        if current == ',' {
            let mut lookahead = index + 1;
            while lookahead < chars.len() && chars[lookahead].is_whitespace() {
                lookahead += 1;
            }
            if matches!(chars.get(lookahead), Some(']') | Some('}')) {
                index += 1;
                continue;
            }
        }
        output.push(current);
        index += 1;
    }
    if in_string || escaped {
        return Err(StorageError::new(
            "CLAUDE_SETTINGS_JSONC_STRING_UNTERMINATED",
            "unterminated JSONC string",
        ));
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use uuid::Uuid;

    fn temp_target() -> (RuntimeTarget, PathBuf) {
        let root = env::temp_dir().join(format!("vibehub-claude-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        (RuntimeTarget::host(root.clone()), root)
    }

    #[test]
    fn discovers_user_and_vibehub_profiles_without_secret_values() {
        let (target, root) = temp_target();
        fs::create_dir_all(root.join(".claude/vibehub-profiles")).unwrap();
        fs::write(
            root.join(".claude/settings.json"),
            br#"{
  "model": "claude-sonnet",
  "alwaysThinkingEnabled": true,
  "env": { "ANTHROPIC_API_KEY": "literal-secret", "ANTHROPIC_BASE_URL": "https://proxy.invalid" },
  "permissions": { "edit": "ask" },
  "hooks": { "PostToolUse": [] },
  "mcpServers": { "local": { "command": "server" } },
  "sandbox": { "enabled": true },
  "unknownSetting": 1
}"#,
        )
        .unwrap();
        fs::write(
            root.join(".claude/vibehub-profiles/work.settings.json"),
            br#"{ "model": "deepseek-chat", "permissions": { "edit": "ask" } }"#,
        )
        .unwrap();
        let profiles = discover_claude_profiles(&target).unwrap();
        assert_eq!(profiles.len(), 2);
        let user = profiles
            .iter()
            .find(|profile| profile.scope == ClaudeSettingsScope::User)
            .unwrap();
        assert_eq!(user.model.as_deref(), Some("claude-sonnet"));
        assert_eq!(user.base_url.as_deref(), Some("https://proxy.invalid"));
        assert_eq!(user.credential.kind, ClaudeCredentialKind::ConfigLiteral);
        assert!(!serde_json::to_string(user)
            .unwrap()
            .contains("literal-secret"));
        assert!(user.preserved_fields.contains(&"permissions".to_owned()));
        assert!(user.unknown_fields.contains(&"unknownSetting".to_owned()));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn default_projection_preserves_permissions_hooks_mcp_sandbox_and_unknown_fields() {
        let (target, root) = temp_target();
        fs::create_dir_all(root.join(".claude/vibehub-profiles")).unwrap();
        fs::write(
            root.join(".claude/vibehub-profiles/remote.settings.json"),
            br#"{ "model": "deepseek-chat", "alwaysThinkingEnabled": true, "env": { "ANTHROPIC_BASE_URL": "https://api.invalid/v1" } }"#,
        )
        .unwrap();
        fs::write(
            root.join(".claude/settings.json"),
            br#"{
  "model": "old-model",
  "permissions": { "allow": ["Read"] },
  "hooks": { "Stop": [] },
  "mcpServers": { "local": { "command": "server" } },
  "sandbox": { "enabled": true },
  "unknown": { "keep": true }
}"#,
        )
        .unwrap();
        let operation = activate_claude_profile(
            &target,
            root.join(".claude/vibehub-profiles/remote.settings.json"),
        )
        .unwrap();
        assert!(operation.profile.default_state.is_default);
        let value: Value =
            serde_json::from_slice(&fs::read(root.join(".claude/settings.json")).unwrap()).unwrap();
        assert_eq!(value["model"], "deepseek-chat");
        assert_eq!(value["alwaysThinkingEnabled"], true);
        assert_eq!(value["env"]["ANTHROPIC_BASE_URL"], "https://api.invalid/v1");
        assert_eq!(value["permissions"]["allow"][0], "Read");
        assert_eq!(value["hooks"]["Stop"], serde_json::json!([]));
        assert_eq!(value["mcpServers"]["local"]["command"], "server");
        assert_eq!(value["sandbox"]["enabled"], true);
        assert_eq!(value["unknown"]["keep"], true);
        let args = claude_code_launch_args(&operation.profile.source_path).unwrap();
        assert_eq!(args[0], "--settings");
        assert_eq!(args[1], operation.profile.source_path.to_string_lossy());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn default_profile_delete_requires_replacement() {
        let (target, root) = temp_target();
        fs::create_dir_all(root.join(".claude/vibehub-profiles")).unwrap();
        let one = root.join(".claude/vibehub-profiles/one.settings.json");
        let two = root.join(".claude/vibehub-profiles/two.settings.json");
        fs::write(&one, br#"{ "model": "one" }"#).unwrap();
        fs::write(&two, br#"{ "model": "two" }"#).unwrap();
        activate_claude_profile(&target, &one).unwrap();
        let error = delete_claude_profile(
            &target,
            &ClaudeDeleteRequest {
                profile_path: one.clone(),
                replacement_profile_path: None,
            },
        )
        .unwrap_err();
        assert_eq!(error.code, "CLAUDE_DEFAULT_PROFILE_REPLACEMENT_REQUIRED");
        let deleted = delete_claude_profile(
            &target,
            &ClaudeDeleteRequest {
                profile_path: one.clone(),
                replacement_profile_path: Some(two.clone()),
            },
        )
        .unwrap();
        assert!(deleted.profile.profile_id.ends_with("two"));
        assert!(!one.exists());
        assert_eq!(
            read_claude_profile(&target, &two).unwrap().model.as_deref(),
            Some("two")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn save_uses_revision_and_keeps_unmanaged_values() {
        let (target, root) = temp_target();
        let path = root.join("settings.json");
        fs::write(
            &path,
            br#"{ "model": "old", "permissions": { "edit": "ask" } }"#,
        )
        .unwrap();
        let document = read_document(&target, &path).unwrap();
        let report = save_claude_profile(
            &target,
            &path,
            Some(&document.revision),
            &ClaudeSettingsPatch {
                model: Some("new".to_owned()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(report.backup_path.is_some());
        let value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(value["model"], "new");
        assert_eq!(value["permissions"]["edit"], "ask");
        let conflict = save_claude_profile(
            &target,
            &path,
            Some(&document.revision),
            &ClaudeSettingsPatch {
                model: Some("stale".to_owned()),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(conflict.code, "CONFIG_REVISION_CONFLICT");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_profile_names_and_non_absolute_launch_paths_fail_closed() {
        let (target, root) = temp_target();
        assert_eq!(
            claude_profile_path(&target, "../escape").unwrap_err().code,
            "CLAUDE_PROFILE_NAME_INVALID"
        );
        assert_eq!(
            claude_code_launch_args("relative.json").unwrap_err().code,
            "CLAUDE_SETTINGS_PATH_NOT_ABSOLUTE"
        );
        fs::remove_dir_all(root).unwrap();
    }
}

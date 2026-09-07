use super::agent_profile_storage::{
    read_document, write_document, AgentKind, ConfigDocument, ConfigFormat, DocumentRevision,
    ParsedConfig, RuntimePlatform, RuntimeTarget, StorageError, WriteReport,
};
use super::protocol_runtime::ProtocolKind;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const OPENCODE_SCHEMA_URL: &str = "https://opencode.ai/config.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenCodeCredentialKind {
    Environment,
    ConfigLiteral,
    None,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenCodeCredentialReference {
    pub kind: OpenCodeCredentialKind,
    pub references: Vec<String>,
    pub display: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenCodeModelView {
    pub model_id: String,
    pub display_name: String,
    pub declared_id: Option<String>,
    pub reasoning: Option<bool>,
    pub variants: Vec<String>,
    /// Full variant values retained for the Tauri/UI round-trip. The editor
    /// displays only the names, but saving an unrelated field must not rebuild
    /// complex variant objects from names alone.
    pub variant_values: Option<BTreeMap<String, Value>>,
    pub unknown_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenCodeProviderView {
    pub provider_id: String,
    pub display_name: String,
    pub base_url: Option<String>,
    pub credential: OpenCodeCredentialReference,
    /// Upstream wire protocol reverse-resolved from the provider `npm`
    /// package. `Unknown` means the configured package does not declare an
    /// unambiguous protocol.
    pub protocol: ProtocolKind,
    pub models: Vec<OpenCodeModelView>,
    pub unknown_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenCodeProfileView {
    pub agent: AgentKind,
    pub source_path: PathBuf,
    pub format: ConfigFormat,
    pub revision: DocumentRevision,
    pub schema_url: Option<String>,
    pub default_model: Option<String>,
    pub small_model: Option<String>,
    pub default_variant: Option<String>,
    pub providers_key: String,
    pub providers: Vec<OpenCodeProviderView>,
    pub unknown_root_fields: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OpenCodeProviderPatch {
    pub display_name: Option<String>,
    pub base_url: Option<String>,
    /// Upstream wire protocol to persist for this provider. `None` leaves the
    /// existing `npm` package untouched; `Some` rewrites `npm` to the package
    /// that speaks the selected protocol.
    pub protocol: Option<ProtocolKind>,
    pub environment_references: Option<Vec<String>>,
    /// Written to `options.apiKey` so OpenCode can authenticate without a
    /// shell environment variable. Omitted from Debug/JSON logs.
    #[serde(default, skip_serializing)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub clear_api_key: bool,
    pub models: BTreeMap<String, OpenCodeModelPatch>,
}

impl std::fmt::Debug for OpenCodeProviderPatch {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OpenCodeProviderPatch")
            .field("display_name", &self.display_name)
            .field("base_url", &self.base_url)
            .field("protocol", &self.protocol)
            .field("environment_references", &self.environment_references)
            .field("api_key", &self.api_key.as_ref().map(|_| "[redacted]"))
            .field("clear_api_key", &self.clear_api_key)
            .field("models", &self.models)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OpenCodeModelPatch {
    pub display_name: Option<String>,
    pub declared_id: Option<String>,
    pub reasoning: Option<bool>,
    pub variants: Option<BTreeMap<String, Value>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OpenCodeConfigPatch {
    pub default_model: Option<String>,
    pub small_model: Option<String>,
    pub default_variant: Option<String>,
    pub deleted_providers: Vec<String>,
    pub deleted_models: BTreeMap<String, Vec<String>>,
    pub providers: BTreeMap<String, OpenCodeProviderPatch>,
}

/// A single candidate location that could not be read during discovery.
///
/// Discovery must be fault-tolerant: one unreadable/invalid candidate must not
/// abort the remaining candidates. Every failure is captured here with its
/// structured path and an actionable recovery hint so the desktop frontend can
/// present a precise diagnostic instead of a bare path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenCodeDiscoveryError {
    pub code: String,
    pub message: String,
    pub path: String,
    pub recovery_hint: String,
}

/// Discovered profiles plus the per-candidate errors that did not abort the
/// scan. See [`discover_opencode_profiles_tolerant`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OpenCodeDiscoveryOutcome {
    pub profiles: Vec<OpenCodeProfileView>,
    pub errors: Vec<OpenCodeDiscoveryError>,
}

pub fn opencode_config_paths(target: &RuntimeTarget) -> Vec<PathBuf> {
    let home = target.home_path.as_path();
    // opencode follows the XDG convention on every platform, including
    // Windows, where it stores configuration under `~/.config/opencode`
    // rather than `%APPDATA%\\opencode`. Prefer the XDG location first and
    // keep the legacy Roaming path as a backward-compatible fallback for
    // users whose config was written by an older VibeHub build.
    let roots: Vec<PathBuf> = match target.platform {
        RuntimePlatform::Windows => vec![
            home.join(".config").join("opencode"),
            home.join("AppData").join("Roaming").join("opencode"),
        ],
        RuntimePlatform::Macos | RuntimePlatform::Linux => {
            vec![home.join(".config").join("opencode")]
        }
    };
    roots
        .into_iter()
        .flat_map(|root| [root.join("opencode.jsonc"), root.join("opencode.json")])
        .collect()
}

pub fn discover_opencode_profiles(
    target: &RuntimeTarget,
) -> Result<Vec<OpenCodeProfileView>, StorageError> {
    Ok(discover_opencode_profiles_tolerant(target).profiles)
}

/// Fault-tolerant discovery. Every candidate location (XDG first, then the
/// legacy Roaming fallback) is checked independently: a single unreadable or
/// invalid candidate is recorded in [`OpenCodeDiscoveryOutcome::errors`] with a
/// structured code, path, reason and recovery hint, and never aborts the scan
/// of the remaining candidates. Only candidates that exist as regular files are
/// attempted, so absent locations are silently skipped while metadata/access
/// failures are surfaced as candidate errors.
pub fn discover_opencode_profiles_tolerant(target: &RuntimeTarget) -> OpenCodeDiscoveryOutcome {
    let mut outcome = OpenCodeDiscoveryOutcome::default();
    for path in opencode_config_paths(target) {
        match std::fs::symlink_metadata(&path) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                outcome.errors.push(OpenCodeDiscoveryError {
                    code: "CONFIG_DISCOVERY_STAT_FAILED".to_owned(),
                    message: error.to_string(),
                    path: path.to_string_lossy().into_owned(),
                    recovery_hint: "检查配置目录是否存在且当前用户具有访问权限。".to_owned(),
                });
                continue;
            }
        }
        match read_opencode_profile(target, &path) {
            Ok(profile) => outcome.profiles.push(profile),
            Err(error) => outcome.errors.push(OpenCodeDiscoveryError {
                code: error.code.to_owned(),
                message: error.message.clone(),
                path: error
                    .path
                    .clone()
                    .unwrap_or_else(|| path.to_string_lossy().into_owned()),
                recovery_hint: recovery_hint_for(error.code),
            }),
        }
    }
    outcome
}

/// Actionable recovery guidance keyed by the storage error code. The hints are
/// operator-facing and never contain credential material.
fn recovery_hint_for(code: &str) -> String {
    match code {
        "CONFIG_NOT_FOUND" => "确认配置文件存在且路径正确。".to_owned(),
        "CONFIG_PATH_OUTSIDE_RUNTIME_HOME" => {
            "该配置路径指向 runtime home 之外（可能为越界 junction/symlink），请改用 home 内的真实路径。".to_owned()
        }
        "CONFIG_PATH_LINK_REJECTED" => {
            "配置路径是符号链接或 junction；请指向 runtime home 内的真实文件。".to_owned()
        }
        "CONFIG_READ_FAILED" | "CONFIG_NOT_REGULAR_FILE" => {
            "检查文件是否存在、具有读取权限且为普通文件。".to_owned()
        }
        "CONFIG_DISCOVERY_STAT_FAILED" => {
            "检查配置目录是否存在且当前用户具有访问权限。".to_owned()
        }
        "CONFIG_TOO_LARGE" => "配置文件过大，请精简后重试。".to_owned(),
        "OPENCODE_CONFIG_FORMAT_UNSUPPORTED" => "OpenCode 仅接受 JSON/JSONC 配置。".to_owned(),
        "OPENCODE_CONFIG_ROOT_INVALID" => "配置文件根必须是 JSON 对象。".to_owned(),
        "OPENCODE_CONFIG_TRAILING_CONTENT" => "配置文件根对象之后存在多余内容，请修复 JSON/JSONC 语法。".to_owned(),
        _ => "请检查配置文件内容与权限后重试。".to_owned(),
    }
}

pub fn read_opencode_profile(
    target: &RuntimeTarget,
    path: impl AsRef<Path>,
) -> Result<OpenCodeProfileView, StorageError> {
    let document = read_document(target, path)?;
    profile_from_document(&document)
}

pub fn save_opencode_profile(
    target: &RuntimeTarget,
    path: impl AsRef<Path>,
    expected_revision: Option<&DocumentRevision>,
    patch: &OpenCodeConfigPatch,
) -> Result<WriteReport, StorageError> {
    let document = read_document(target, path)?;
    if !matches!(document.format, ConfigFormat::Json | ConfigFormat::Jsonc) {
        return Err(StorageError::new(
            "OPENCODE_CONFIG_FORMAT_UNSUPPORTED",
            "OpenCode accepts JSON or JSONC configuration",
        ));
    }
    let edited = JsoncEditor::new(&document.raw)?.apply_patch(patch)?;
    write_document(target, document.path, expected_revision, &edited)
}

fn profile_from_document(document: &ConfigDocument) -> Result<OpenCodeProfileView, StorageError> {
    let parsed = document.parse()?;
    let root = match parsed {
        ParsedConfig::Json(value) => value,
        ParsedConfig::Toml(_) => {
            return Err(StorageError::new(
                "OPENCODE_CONFIG_FORMAT_UNSUPPORTED",
                "OpenCode does not use TOML configuration",
            ))
        }
    };
    let object = root.as_object().ok_or_else(|| {
        StorageError::new(
            "OPENCODE_CONFIG_ROOT_INVALID",
            "OpenCode configuration root must be an object",
        )
    })?;
    let providers_key = if object.get("provider").is_some() {
        "provider"
    } else if object.get("providers").is_some() {
        "providers"
    } else {
        "provider"
    };
    let mut warnings = Vec::new();
    if object.get("$schema").and_then(Value::as_str) != Some(OPENCODE_SCHEMA_URL) {
        warnings.push("OPENCODE_SCHEMA_URL_MISSING_OR_DIFFERENT".to_owned());
    }
    let providers = object
        .get(providers_key)
        .and_then(Value::as_object)
        .map(|providers| {
            providers
                .iter()
                .map(|(provider_id, value)| provider_view(provider_id, value, &mut warnings))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    let (default_model, model_variant) = split_model_variant(object.get("model"));
    let agent_variant = object
        .get("agent")
        .and_then(Value::as_object)
        .and_then(|agent| agent.get("build"))
        .and_then(Value::as_object)
        .and_then(|build| build.get("variant"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    let default_variant = agent_variant.or(model_variant);
    let (small_model, _) = split_model_variant(object.get("small_model"));
    let known_root = [
        "$schema",
        "model",
        "small_model",
        "provider",
        "providers",
        "default_agent",
        "agent",
    ];
    let unknown_root_fields = object
        .keys()
        .filter(|key| !known_root.contains(&key.as_str()))
        .cloned()
        .collect();
    Ok(OpenCodeProfileView {
        agent: AgentKind::Opencode,
        source_path: document.path.clone(),
        format: document.format,
        revision: document.revision.clone(),
        schema_url: object
            .get("$schema")
            .and_then(Value::as_str)
            .map(str::to_owned),
        default_model,
        small_model,
        default_variant,
        providers_key: providers_key.to_owned(),
        providers,
        unknown_root_fields,
        warnings,
    })
}

/// The npm package that actually carries the upstream wire protocol inside
/// `opencode.json`. `@ai-sdk/openai-compatible` speaks Chat Completions and
/// `@ai-sdk/anthropic` speaks Anthropic Messages; the AI SDK packages exposed
/// by OpenCode have no member that faithfully speaks the Responses API, so
/// that protocol cannot be persisted.
fn protocol_npm_package(protocol: ProtocolKind) -> Result<&'static str, StorageError> {
    match protocol {
        ProtocolKind::OpenaiChatCompletions => Ok("@ai-sdk/openai-compatible"),
        ProtocolKind::AnthropicMessages => Ok("@ai-sdk/anthropic"),
        ProtocolKind::OpenaiResponses | ProtocolKind::Unknown => Err(StorageError::new(
            "OPENCODE_PROTOCOL_UNSUPPORTED",
            "OpenCode provider protocol must be Chat Completions or Anthropic Messages to persist",
        )),
    }
}

/// Inverse of [`protocol_npm_package`]. Packages with an unambiguous wire
/// protocol resolve to it; everything else stays `Unknown` instead of guessing
/// from a brand or name.
fn protocol_from_npm(npm: Option<&str>) -> ProtocolKind {
    match npm {
        Some("@ai-sdk/openai-compatible") => ProtocolKind::OpenaiChatCompletions,
        Some("@ai-sdk/anthropic") => ProtocolKind::AnthropicMessages,
        _ => ProtocolKind::Unknown,
    }
}

fn provider_view(
    provider_id: &str,
    value: &Value,
    warnings: &mut Vec<String>,
) -> Result<OpenCodeProviderView, StorageError> {
    let object = value.as_object().ok_or_else(|| {
        StorageError::new(
            "OPENCODE_PROVIDER_INVALID",
            format!("provider {provider_id} must be an object"),
        )
    })?;
    let options = object.get("options").and_then(Value::as_object);
    let base_url = options
        .and_then(|options| options.get("baseURL"))
        .and_then(Value::as_str)
        .or_else(|| object.get("baseURL").and_then(Value::as_str))
        .map(str::to_owned);
    let credential = credential_reference(object, options, warnings);
    let models = object
        .get("models")
        .and_then(Value::as_object)
        .map(|models| {
            models
                .iter()
                .map(|(model_id, value)| model_view(model_id, value))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    let known = [
        "api",
        "env",
        "id",
        "name",
        "npm",
        "options",
        "models",
        "whitelist",
        "blacklist",
    ];
    let unknown_fields = object
        .keys()
        .filter(|key| !known.contains(&key.as_str()))
        .cloned()
        .collect();
    Ok(OpenCodeProviderView {
        provider_id: provider_id.to_owned(),
        display_name: object
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(provider_id)
            .to_owned(),
        base_url,
        credential,
        protocol: protocol_from_npm(object.get("npm").and_then(Value::as_str)),
        models,
        unknown_fields,
    })
}

fn credential_reference(
    provider: &Map<String, Value>,
    options: Option<&Map<String, Value>>,
    _warnings: &mut Vec<String>,
) -> OpenCodeCredentialReference {
    let environment_references = provider
        .get("env")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if !environment_references.is_empty() {
        return OpenCodeCredentialReference {
            kind: OpenCodeCredentialKind::Environment,
            display: format!("环境变量 {}", environment_references.join(", ")),
            references: environment_references,
        };
    }
    if options
        .and_then(|options| options.get("apiKey"))
        .is_some_and(Value::is_string)
    {
        return OpenCodeCredentialReference {
            kind: OpenCodeCredentialKind::ConfigLiteral,
            references: Vec::new(),
            display: "配置内存在 API Key（已隐藏）".to_owned(),
        };
    }
    OpenCodeCredentialReference {
        kind: OpenCodeCredentialKind::Unknown,
        references: Vec::new(),
        display: "由 OpenCode 运行时决定".to_owned(),
    }
}

fn model_view(model_id: &str, value: &Value) -> Result<OpenCodeModelView, StorageError> {
    let object = value.as_object().ok_or_else(|| {
        StorageError::new(
            "OPENCODE_MODEL_INVALID",
            format!("model {model_id} must be an object"),
        )
    })?;
    let variants = object
        .get("variants")
        .and_then(Value::as_object)
        .map(|variants| variants.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let variant_values = object
        .get("variants")
        .and_then(Value::as_object)
        .map(|variants| {
            variants
                .iter()
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect::<BTreeMap<_, _>>()
        });
    let known = [
        "id",
        "modelID",
        "name",
        "family",
        "release_date",
        "attachment",
        "reasoning",
        "temperature",
        "tool_call",
        "interleaved",
        "cost",
        "limit",
        "modalities",
        "experimental",
        "status",
        "provider",
        "options",
        "headers",
        "variants",
    ];
    Ok(OpenCodeModelView {
        model_id: model_id.to_owned(),
        display_name: object
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(model_id)
            .to_owned(),
        declared_id: object
            .get("id")
            .or_else(|| object.get("modelID"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        reasoning: object.get("reasoning").and_then(Value::as_bool),
        variants,
        variant_values,
        unknown_fields: object
            .keys()
            .filter(|key| !known.contains(&key.as_str()))
            .cloned()
            .collect(),
    })
}

fn split_model_variant(value: Option<&Value>) -> (Option<String>, Option<String>) {
    let value = value.and_then(Value::as_str);
    let Some(value) = value else {
        return (None, None);
    };
    let Some((model, variant)) = value.rsplit_once('#') else {
        return (Some(value.to_owned()), None);
    };
    (Some(model.to_owned()), Some(variant.to_owned()))
}

#[derive(Debug, Clone)]
struct JsonObjectNode {
    start: usize,
    end: usize,
    close: usize,
    members: BTreeMap<String, JsonNode>,
}

#[derive(Debug, Clone)]
enum JsonNode {
    Object(JsonObjectNode),
    Other { start: usize, end: usize },
}

impl JsonNode {
    fn start(&self) -> usize {
        match self {
            Self::Object(node) => node.start,
            Self::Other { start, .. } => *start,
        }
    }

    fn end(&self) -> usize {
        match self {
            Self::Object(node) => node.end,
            Self::Other { end, .. } => *end,
        }
    }
}

struct JsoncEditor {
    source: Vec<u8>,
    root: JsonObjectNode,
}

impl JsoncEditor {
    fn new(source: &[u8]) -> Result<Self, StorageError> {
        let mut parser = JsoncSpanParser {
            source,
            position: 0,
        };
        let root = match parser.parse_value()? {
            JsonNode::Object(root) => root,
            JsonNode::Other { .. } => {
                return Err(StorageError::new(
                    "OPENCODE_CONFIG_ROOT_INVALID",
                    "OpenCode configuration root must be an object",
                ))
            }
        };
        parser.skip_space_and_comments()?;
        if parser.position != source.len() {
            return Err(StorageError::new(
                "OPENCODE_CONFIG_TRAILING_CONTENT",
                "unexpected content after OpenCode configuration",
            ));
        }
        Ok(Self {
            source: source.to_vec(),
            root,
        })
    }

    fn apply_patch(mut self, patch: &OpenCodeConfigPatch) -> Result<Vec<u8>, StorageError> {
        let provider_key = if self.root.members.contains_key("provider") {
            "provider"
        } else if self.root.members.contains_key("providers") {
            "providers"
        } else {
            "provider"
        };
        let mut replacements = Vec::new();
        for provider_id in &patch.deleted_providers {
            self.remove_member(&[provider_key], provider_id, &mut replacements)?;
        }
        for (provider_id, model_ids) in &patch.deleted_models {
            for model_id in model_ids {
                self.remove_member(
                    &[provider_key, provider_id, "models"],
                    model_id,
                    &mut replacements,
                )?;
            }
        }
        if let Some(model) = &patch.default_model {
            let value = model.clone().into();
            self.set_path(&["model"], value, &mut replacements)?;
        }
        if let Some(model) = &patch.small_model {
            self.set_path(&["small_model"], model.clone().into(), &mut replacements)?;
        }
        if let Some(variant) = &patch.default_variant {
            self.set_path(
                &["agent", "build", "variant"],
                variant.clone().into(),
                &mut replacements,
            )?;
        }
        for (provider_id, provider_patch) in &patch.providers {
            self.ensure_object_path(&[provider_key, provider_id], &mut replacements)?;
            if let Some(display_name) = &provider_patch.display_name {
                self.set_path(
                    &[provider_key, provider_id, "name"],
                    display_name.clone().into(),
                    &mut replacements,
                )?;
            }
            if let Some(base_url) = &provider_patch.base_url {
                self.set_path(
                    &[provider_key, provider_id, "options", "baseURL"],
                    base_url.clone().into(),
                    &mut replacements,
                )?;
            }
            if let Some(protocol) = provider_patch.protocol {
                let npm = protocol_npm_package(protocol)?;
                self.set_path(
                    &[provider_key, provider_id, "npm"],
                    npm.into(),
                    &mut replacements,
                )?;
            }
            if let Some(api_key) = &provider_patch.api_key {
                self.set_path(
                    &[provider_key, provider_id, "options", "apiKey"],
                    api_key.clone().into(),
                    &mut replacements,
                )?;
                self.remove_member(&[provider_key, provider_id], "env", &mut replacements)?;
            } else if provider_patch.clear_api_key {
                self.remove_member(
                    &[provider_key, provider_id, "options"],
                    "apiKey",
                    &mut replacements,
                )?;
            }
            if let Some(environment_references) = &provider_patch.environment_references {
                if provider_patch.api_key.is_none() {
                    let env = Value::Array(
                        environment_references
                            .iter()
                            .cloned()
                            .map(Value::String)
                            .collect(),
                    );
                    self.set_path(&[provider_key, provider_id, "env"], env, &mut replacements)?;
                }
            }
            for (model_id, model_patch) in &provider_patch.models {
                self.ensure_object_path(
                    &[provider_key, provider_id, "models", model_id],
                    &mut replacements,
                )?;
                if let Some(display_name) = &model_patch.display_name {
                    self.set_path(
                        &[provider_key, provider_id, "models", model_id, "name"],
                        display_name.clone().into(),
                        &mut replacements,
                    )?;
                }
                if let Some(declared_id) = &model_patch.declared_id {
                    self.set_path(
                        &[provider_key, provider_id, "models", model_id, "id"],
                        declared_id.clone().into(),
                        &mut replacements,
                    )?;
                }
                if let Some(reasoning) = model_patch.reasoning {
                    self.set_path(
                        &[provider_key, provider_id, "models", model_id, "reasoning"],
                        reasoning.into(),
                        &mut replacements,
                    )?;
                }
                if let Some(variants) = &model_patch.variants {
                    // None means variants were not edited. Some(empty) is an
                    // explicit request to clear them, so preserve this
                    // distinction all the way from the UI patch.
                    let variants = Value::Object(variants.clone().into_iter().collect());
                    self.set_path(
                        &[provider_key, provider_id, "models", model_id, "variants"],
                        variants,
                        &mut replacements,
                    )?;
                }
            }
        }
        apply_replacements(&self.source, replacements)
    }

    fn remove_member(
        &mut self,
        parent_path: &[&str],
        key: &str,
        replacements: &mut Vec<Replacement>,
    ) -> Result<(), StorageError> {
        let parent = match self.find_node(parent_path) {
            Some(JsonNode::Object(object)) => object,
            _ => return Ok(()),
        };
        let Some(value) = parent.members.get(key) else {
            return Ok(());
        };
        let key_start = self.member_key_start(&parent, key, value.start())?;
        let mut after = self.skip_trivia(value.end())?;
        if self.source.get(after) == Some(&b',') {
            after += 1;
        } else {
            after = value.end();
        }
        replacements.push(Replacement {
            start: key_start,
            end: after,
            content: Vec::new(),
        });
        self.reparse_with_replacements(replacements)
    }

    fn member_key_start(
        &self,
        parent: &JsonObjectNode,
        key: &str,
        value_start: usize,
    ) -> Result<usize, StorageError> {
        let literal = serde_json::to_vec(key).map_err(|error| {
            StorageError::new("OPENCODE_PATCH_SERIALIZE_FAILED", error.to_string())
        })?;
        let source = &self.source[parent.start..value_start];
        let Some(relative) = source
            .windows(literal.len())
            .rposition(|window| window == literal)
        else {
            return Err(StorageError::new(
                "OPENCODE_PATCH_MEMBER_NOT_FOUND",
                format!("unable to locate JSONC member {key}"),
            ));
        };
        let candidate = parent.start + relative;
        let mut cursor = candidate + literal.len();
        while self.source.get(cursor).is_some_and(u8::is_ascii_whitespace) {
            cursor += 1;
        }
        if self.source.get(cursor) != Some(&b':') {
            return Err(StorageError::new(
                "OPENCODE_PATCH_MEMBER_NOT_FOUND",
                format!("unable to locate JSONC member {key}"),
            ));
        }
        Ok(candidate)
    }

    fn skip_trivia(&self, mut position: usize) -> Result<usize, StorageError> {
        loop {
            while self
                .source
                .get(position)
                .is_some_and(u8::is_ascii_whitespace)
            {
                position += 1;
            }
            if self.source.get(position) == Some(&b'/')
                && self.source.get(position + 1) == Some(&b'/')
            {
                position += 2;
                while position < self.source.len() && self.source[position] != b'\n' {
                    position += 1;
                }
                continue;
            }
            if self.source.get(position) == Some(&b'/')
                && self.source.get(position + 1) == Some(&b'*')
            {
                position += 2;
                let mut closed = false;
                while position + 1 < self.source.len() {
                    if self.source[position] == b'*' && self.source[position + 1] == b'/' {
                        position += 2;
                        closed = true;
                        break;
                    }
                    position += 1;
                }
                if !closed {
                    return Err(StorageError::new(
                        "OPENCODE_JSONC_COMMENT_UNTERMINATED",
                        "unterminated block comment",
                    ));
                }
                continue;
            }
            return Ok(position);
        }
    }

    fn ensure_object_path(
        &mut self,
        path: &[&str],
        replacements: &mut Vec<Replacement>,
    ) -> Result<(), StorageError> {
        if path.is_empty() || self.find_node(path).is_some() {
            return Ok(());
        }
        let mut current = Vec::<&str>::new();
        for segment in path {
            current.push(segment);
            if self.find_node(&current).is_none() {
                let value = if current.len() == path.len() {
                    Value::Object(Map::new())
                } else {
                    Value::Object(Map::new())
                };
                self.insert_member(&current[..current.len() - 1], segment, value, replacements)?;
                self.reparse_with_replacements(replacements)?;
            }
        }
        Ok(())
    }

    fn set_path(
        &mut self,
        path: &[&str],
        value: Value,
        replacements: &mut Vec<Replacement>,
    ) -> Result<(), StorageError> {
        if path.is_empty() {
            return Err(StorageError::new(
                "OPENCODE_PATCH_PATH_EMPTY",
                "patch path is empty",
            ));
        }
        if self.find_node(path).is_none() {
            self.ensure_object_path(&path[..path.len() - 1], replacements)?;
            self.insert_member(
                path[..path.len() - 1].as_ref(),
                path[path.len() - 1],
                value,
                replacements,
            )?;
        } else {
            let node = self.find_node(path).expect("checked above");
            let serialized = serde_json::to_vec(&value).map_err(|error| {
                StorageError::new("OPENCODE_PATCH_SERIALIZE_FAILED", error.to_string())
            })?;
            replacements.push(Replacement {
                start: node.start(),
                end: node.end(),
                content: serialized,
            });
        }
        self.reparse_with_replacements(replacements)?;
        Ok(())
    }

    fn find_node(&self, path: &[&str]) -> Option<JsonNode> {
        let mut node = JsonNode::Object(self.root.clone());
        for segment in path {
            node = match node {
                JsonNode::Object(object) => object.members.get(*segment)?.clone(),
                JsonNode::Other { .. } => return None,
            };
        }
        Some(node)
    }

    fn insert_member(
        &self,
        parent_path: &[&str],
        key: &str,
        value: Value,
        replacements: &mut Vec<Replacement>,
    ) -> Result<(), StorageError> {
        let parent = match self.find_node(parent_path) {
            Some(JsonNode::Object(object)) => object,
            _ => {
                return Err(StorageError::new(
                    "OPENCODE_PATCH_PARENT_INVALID",
                    parent_path.join("."),
                ))
            }
        };
        let serialized = serde_json::to_vec(&value).map_err(|error| {
            StorageError::new("OPENCODE_PATCH_SERIALIZE_FAILED", error.to_string())
        })?;
        let has_members = !parent.members.is_empty();
        let trailing_comma = has_members && self.has_trailing_comma(parent.close);
        let indent = self.insertion_indent(parent.close);
        let parent_indent = self.parent_indent(parent.start);
        let mut content = Vec::new();
        if has_members && !trailing_comma {
            content.extend_from_slice(b",");
        }
        content.extend_from_slice(b"\n");
        content.extend_from_slice(indent.as_bytes());
        content.extend_from_slice(serde_json::to_string(key).unwrap().as_bytes());
        content.extend_from_slice(b": ");
        content.extend_from_slice(&serialized);
        content.extend_from_slice(b"\n");
        content.extend_from_slice(parent_indent.as_bytes());
        replacements.push(Replacement {
            start: parent.close,
            end: parent.close,
            content,
        });
        Ok(())
    }

    fn has_trailing_comma(&self, close: usize) -> bool {
        let mut index = close;
        while index > 0 {
            index -= 1;
            if self.source[index].is_ascii_whitespace() {
                continue;
            }
            return self.source[index] == b',';
        }
        false
    }

    fn insertion_indent(&self, close: usize) -> String {
        let mut line_start = close;
        while line_start > 0 && self.source[line_start - 1] != b'\n' {
            line_start -= 1;
        }
        let parent_indent = self.source[line_start..close]
            .iter()
            .take_while(|byte| byte.is_ascii_whitespace() && **byte != b'\n')
            .map(|byte| *byte as char)
            .collect::<String>();
        format!("{parent_indent}  ")
    }

    fn parent_indent(&self, start: usize) -> String {
        let mut line_start = start;
        while line_start > 0 && self.source[line_start - 1] != b'\n' {
            line_start -= 1;
        }
        self.source[line_start..start]
            .iter()
            .take_while(|byte| byte.is_ascii_whitespace() && **byte != b'\n')
            .map(|byte| *byte as char)
            .collect()
    }

    fn reparse_with_replacements(
        &mut self,
        replacements: &mut Vec<Replacement>,
    ) -> Result<(), StorageError> {
        let source = apply_replacements(&self.source, replacements.clone())?;
        self.source = source;
        replacements.clear();
        let mut parser = JsoncSpanParser {
            source: &self.source,
            position: 0,
        };
        self.root = match parser.parse_value()? {
            JsonNode::Object(root) => root,
            JsonNode::Other { .. } => {
                return Err(StorageError::new(
                    "OPENCODE_CONFIG_ROOT_INVALID",
                    "root must be an object",
                ))
            }
        };
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct Replacement {
    start: usize,
    end: usize,
    content: Vec<u8>,
}

fn apply_replacements(
    source: &[u8],
    mut replacements: Vec<Replacement>,
) -> Result<Vec<u8>, StorageError> {
    replacements.sort_by_key(|replacement| (replacement.start, replacement.end));
    for pair in replacements.windows(2) {
        if pair[0].end > pair[1].start {
            return Err(StorageError::new(
                "OPENCODE_PATCH_OVERLAP",
                "overlapping JSONC patch spans",
            ));
        }
    }
    let mut output = source.to_vec();
    for replacement in replacements.into_iter().rev() {
        output.splice(replacement.start..replacement.end, replacement.content);
    }
    Ok(output)
}

struct JsoncSpanParser<'a> {
    source: &'a [u8],
    position: usize,
}

impl<'a> JsoncSpanParser<'a> {
    fn parse_value(&mut self) -> Result<JsonNode, StorageError> {
        self.skip_space_and_comments()?;
        let start = self.position;
        match self.source.get(self.position) {
            Some(b'{') => self.parse_object(start),
            Some(b'[') => self.parse_array(start),
            Some(b'"') => {
                self.parse_string()?;
                Ok(JsonNode::Other {
                    start,
                    end: self.position,
                })
            }
            Some(_) => {
                while let Some(byte) = self.source.get(self.position) {
                    if byte.is_ascii_whitespace() || matches!(byte, b',' | b']' | b'}') {
                        break;
                    }
                    self.position += 1;
                }
                Ok(JsonNode::Other {
                    start,
                    end: self.position,
                })
            }
            None => Err(StorageError::new(
                "OPENCODE_JSONC_VALUE_MISSING",
                "expected a JSON value",
            )),
        }
    }

    fn parse_object(&mut self, start: usize) -> Result<JsonNode, StorageError> {
        self.position += 1;
        let mut members = BTreeMap::new();
        loop {
            self.skip_space_and_comments()?;
            if self.source.get(self.position) == Some(&b'}') {
                let close = self.position;
                self.position += 1;
                return Ok(JsonNode::Object(JsonObjectNode {
                    start,
                    end: self.position,
                    close,
                    members,
                }));
            }
            let key = self.parse_key()?;
            self.skip_space_and_comments()?;
            if self.source.get(self.position) != Some(&b':') {
                return Err(StorageError::new(
                    "OPENCODE_JSONC_COLON_MISSING",
                    "object key is missing ':'",
                ));
            }
            self.position += 1;
            let value = self.parse_value()?;
            members.insert(key, value);
            self.skip_space_and_comments()?;
            if self.source.get(self.position) == Some(&b',') {
                self.position += 1;
                continue;
            }
            if self.source.get(self.position) == Some(&b'}') {
                continue;
            }
            return Err(StorageError::new(
                "OPENCODE_JSONC_OBJECT_INVALID",
                "object member must end with ',' or '}'",
            ));
        }
    }

    fn parse_array(&mut self, start: usize) -> Result<JsonNode, StorageError> {
        self.position += 1;
        loop {
            self.skip_space_and_comments()?;
            if self.source.get(self.position) == Some(&b']') {
                self.position += 1;
                return Ok(JsonNode::Other {
                    start,
                    end: self.position,
                });
            }
            self.parse_value()?;
            self.skip_space_and_comments()?;
            if self.source.get(self.position) == Some(&b',') {
                self.position += 1;
                continue;
            }
            if self.source.get(self.position) == Some(&b']') {
                continue;
            }
            return Err(StorageError::new(
                "OPENCODE_JSONC_ARRAY_INVALID",
                "array item must end with ',' or ']'",
            ));
        }
    }

    fn parse_key(&mut self) -> Result<String, StorageError> {
        let start = self.position;
        self.parse_string()?;
        serde_json::from_slice::<String>(&self.source[start..self.position])
            .map_err(|error| StorageError::new("OPENCODE_JSONC_KEY_INVALID", error.to_string()))
    }

    fn parse_string(&mut self) -> Result<(), StorageError> {
        if self.source.get(self.position) != Some(&b'"') {
            return Err(StorageError::new(
                "OPENCODE_JSONC_STRING_EXPECTED",
                "expected a JSON string",
            ));
        }
        self.position += 1;
        let mut escaped = false;
        while let Some(byte) = self.source.get(self.position) {
            self.position += 1;
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                return Ok(());
            }
        }
        Err(StorageError::new(
            "OPENCODE_JSONC_STRING_UNTERMINATED",
            "unterminated JSON string",
        ))
    }

    fn skip_space_and_comments(&mut self) -> Result<(), StorageError> {
        loop {
            while self
                .source
                .get(self.position)
                .is_some_and(u8::is_ascii_whitespace)
            {
                self.position += 1;
            }
            if self.source.get(self.position) == Some(&b'/')
                && self.source.get(self.position + 1) == Some(&b'/')
            {
                self.position += 2;
                while self.position < self.source.len() && self.source[self.position] != b'\n' {
                    self.position += 1;
                }
                continue;
            }
            if self.source.get(self.position) == Some(&b'/')
                && self.source.get(self.position + 1) == Some(&b'*')
            {
                self.position += 2;
                let mut closed = false;
                while self.position + 1 < self.source.len() {
                    if self.source[self.position] == b'*' && self.source[self.position + 1] == b'/'
                    {
                        self.position += 2;
                        closed = true;
                        break;
                    }
                    self.position += 1;
                }
                if !closed {
                    return Err(StorageError::new(
                        "OPENCODE_JSONC_COMMENT_UNTERMINATED",
                        "unterminated block comment",
                    ));
                }
                continue;
            }
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v3::agent_profile_storage::{
        NativeConfigPath, RuntimePlatform, RuntimeTarget, RuntimeTargetKind, RuntimeTargetSource,
    };
    use std::env;
    use std::fs;
    use uuid::Uuid;

    fn temp_target() -> (RuntimeTarget, PathBuf) {
        let root = env::temp_dir().join(format!("vibehub-opencode-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        (RuntimeTarget::host(root.clone()), root)
    }

    #[test]
    fn reads_current_and_legacy_provider_keys_without_exposing_key() {
        let (target, root) = temp_target();
        let path = root.join("opencode.jsonc");
        fs::write(
            &path,
            br#"{
  "$schema": "https://opencode.ai/config.json",
  "model": "deepseek/deepseek-chat#high",
  "small_model": "deepseek/deepseek-chat",
  "provider": {
    "deepseek": {
      "name": "DeepSeek",
      "env": ["DEEPSEEK_API_KEY"],
      "options": { "baseURL": "https://api.deepseek.com/v1", "apiKey": "do-not-return" },
      "models": {
        "deepseek-chat": {
          "name": "DeepSeek Chat",
          "reasoning": true,
          "variants": { "low": {}, "high": {} },
          "unknown": "keep"
        }
      }
    }
  },
  "permissions": { "edit": "ask" }
}"#,
        )
        .unwrap();
        let view = read_opencode_profile(&target, &path).unwrap();
        assert_eq!(
            view.default_model.as_deref(),
            Some("deepseek/deepseek-chat")
        );
        assert_eq!(view.default_variant.as_deref(), Some("high"));
        assert_eq!(view.small_model.as_deref(), Some("deepseek/deepseek-chat"));
        assert_eq!(view.providers_key, "provider");
        assert_eq!(
            view.providers[0].credential.kind,
            OpenCodeCredentialKind::Environment
        );
        assert!(!serde_json::to_string(&view)
            .unwrap()
            .contains("do-not-return"));
        assert!(view.providers[0].models[0]
            .unknown_fields
            .contains(&"unknown".to_owned()));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn patch_preserves_jsonc_comments_and_unknown_fields() {
        let (target, root) = temp_target();
        let path = root.join("opencode.jsonc");
        let original = br#"{
  // keep this comment
  "model": "openai/old",
  "provider": {
    "openai": {
      "options": { "baseURL": "https://old.invalid/v1" },
      "models": { "old": { "name": "Old", "unknown": 1 } }
    }
  },
  "permissions": { "edit": "ask" }
}"#;
        fs::write(&path, original).unwrap();
        let document = read_document(&target, &path).unwrap();
        let mut providers = BTreeMap::new();
        let mut models = BTreeMap::new();
        models.insert(
            "old".to_owned(),
            OpenCodeModelPatch {
                display_name: Some("Renamed".to_owned()),
                ..Default::default()
            },
        );
        providers.insert(
            "openai".to_owned(),
            OpenCodeProviderPatch {
                base_url: Some("https://new.invalid/v1".to_owned()),
                models,
                ..Default::default()
            },
        );
        save_opencode_profile(
            &target,
            &path,
            Some(&document.revision),
            &OpenCodeConfigPatch {
                default_model: Some("openai/old".to_owned()),
                providers,
                ..Default::default()
            },
        )
        .unwrap();
        let updated = String::from_utf8(fs::read(&path).unwrap()).unwrap();
        assert!(updated.contains("// keep this comment"));
        assert!(updated.contains("\"permissions\""));
        assert!(updated.contains("\"unknown\": 1"));
        assert!(updated.contains("https://new.invalid/v1"));
        assert!(updated.contains("\"name\": \"Renamed\""));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn legacy_providers_key_is_patched_in_place() {
        let (target, root) = temp_target();
        let path = root.join("opencode.json");
        fs::write(&path, br#"{"providers":{"local":{"models":{"one":{}}}}}"#).unwrap();
        let document = read_document(&target, &path).unwrap();
        let mut providers = BTreeMap::new();
        providers.insert(
            "local".to_owned(),
            OpenCodeProviderPatch {
                display_name: Some("Local".to_owned()),
                ..Default::default()
            },
        );
        save_opencode_profile(
            &target,
            &path,
            Some(&document.revision),
            &OpenCodeConfigPatch {
                providers,
                ..Default::default()
            },
        )
        .unwrap();
        let view = read_opencode_profile(&target, &path).unwrap();
        assert_eq!(view.providers_key, "providers");
        assert_eq!(view.providers[0].display_name, "Local");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn patch_adds_provider_model_and_agent_variant_without_losing_root_fields() {
        let (target, root) = temp_target();
        let path = root.join("opencode.jsonc");
        fs::write(
            &path,
            br#"{
  // preserve this root comment
  "model": "old/provider",
  "permissions": { "edit": "ask" }
}"#,
        )
        .unwrap();
        let document = read_document(&target, &path).unwrap();
        let mut models = BTreeMap::new();
        let mut variants = BTreeMap::new();
        variants.insert(
            "low".to_owned(),
            serde_json::json!({ "reasoningEffort": "low" }),
        );
        models.insert(
            "deepseek-chat".to_owned(),
            OpenCodeModelPatch {
                display_name: Some("DeepSeek Chat".to_owned()),
                declared_id: Some("deepseek-chat".to_owned()),
                reasoning: Some(true),
                variants: Some(variants),
            },
        );
        let mut providers = BTreeMap::new();
        providers.insert(
            "deepseek".to_owned(),
            OpenCodeProviderPatch {
                display_name: Some("DeepSeek".to_owned()),
                base_url: Some("https://api.deepseek.com/v1".to_owned()),
                environment_references: Some(vec!["DEEPSEEK_API_KEY".to_owned()]),
                models,
                ..Default::default()
            },
        );
        save_opencode_profile(
            &target,
            &path,
            Some(&document.revision),
            &OpenCodeConfigPatch {
                default_model: Some("deepseek/deepseek-chat".to_owned()),
                default_variant: Some("low".to_owned()),
                providers,
                ..Default::default()
            },
        )
        .unwrap();
        let view = read_opencode_profile(&target, &path).unwrap();
        assert_eq!(
            view.default_model.as_deref(),
            Some("deepseek/deepseek-chat")
        );
        assert_eq!(view.default_variant.as_deref(), Some("low"));
        assert_eq!(view.providers.len(), 1);
        assert_eq!(view.providers[0].models[0].model_id, "deepseek-chat");
        assert_eq!(view.providers[0].models[0].variants, vec!["low".to_owned()]);
        let raw = String::from_utf8(fs::read(&path).unwrap()).unwrap();
        assert!(raw.contains("preserve this root comment"));
        assert!(raw.contains("\"permissions\""));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn patch_removes_provider_and_model_without_rewriting_other_fields() {
        let (target, root) = temp_target();
        let path = root.join("opencode.jsonc");
        fs::write(
            &path,
            br#"{
  // preserve this comment
  "provider": {
    "remove-me": { "models": { "old": { "name": "Old" }, "keep": { "name": "Keep" } } },
    "keep-me": { "models": { "active": { "name": "Active" } } }
  },
  "permissions": { "edit": "ask" }
}"#,
        )
        .unwrap();
        let document = read_document(&target, &path).unwrap();
        let mut deleted_models = BTreeMap::new();
        deleted_models.insert("remove-me".to_owned(), vec!["old".to_owned()]);
        save_opencode_profile(
            &target,
            &path,
            Some(&document.revision),
            &OpenCodeConfigPatch {
                deleted_models,
                deleted_providers: vec!["remove-me".to_owned()],
                ..Default::default()
            },
        )
        .unwrap();
        let view = read_opencode_profile(&target, &path).unwrap();
        assert_eq!(view.providers.len(), 1);
        assert_eq!(view.providers[0].provider_id, "keep-me");
        assert_eq!(view.providers[0].models[0].model_id, "active");
        let raw = String::from_utf8(fs::read(&path).unwrap()).unwrap();
        assert!(raw.contains("preserve this comment"));
        assert!(raw.contains("permissions"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn save_writes_api_key_removes_env_hides_secret_and_redacts_debug() {
        let (target, root) = temp_target();
        let path = root.join("opencode.jsonc");
        fs::write(
            &path,
            br#"{
  "provider": {
    "stepfun": {
      "name": "StepFun",
      "env": ["OPENAI_API_KEY"],
      "options": { "baseURL": "https://api.stepfun.com/v1" },
      "models": { "water18": { "name": "water18" } }
    }
  }
}"#,
        )
        .unwrap();
        let document = read_document(&target, &path).unwrap();
        let mut providers = BTreeMap::new();
        providers.insert(
            "stepfun".to_owned(),
            OpenCodeProviderPatch {
                api_key: Some("opencode-secret-value".to_owned()),
                ..Default::default()
            },
        );
        let patch = OpenCodeConfigPatch {
            providers,
            ..Default::default()
        };
        let debug = format!("{patch:?}");
        assert!(debug.contains("[redacted]"));
        assert!(!debug.contains("opencode-secret-value"));
        save_opencode_profile(&target, &path, Some(&document.revision), &patch).unwrap();
        let raw = String::from_utf8(fs::read(&path).unwrap()).unwrap();
        assert!(raw.contains("\"apiKey\": \"opencode-secret-value\""));
        assert!(!raw.contains("OPENAI_API_KEY"));
        assert!(raw.contains("https://api.stepfun.com/v1"));
        let view = read_opencode_profile(&target, &path).unwrap();
        assert_eq!(
            view.providers[0].credential.kind,
            OpenCodeCredentialKind::ConfigLiteral
        );
        assert!(!serde_json::to_string(&view)
            .unwrap()
            .contains("opencode-secret-value"));
        assert!(!view
            .warnings
            .contains(&"OPENCODE_LITERAL_API_KEY_PRESENT".to_owned()));

        let document = read_document(&target, &path).unwrap();
        let mut providers = BTreeMap::new();
        providers.insert(
            "stepfun".to_owned(),
            OpenCodeProviderPatch {
                clear_api_key: true,
                ..Default::default()
            },
        );
        save_opencode_profile(
            &target,
            &path,
            Some(&document.revision),
            &OpenCodeConfigPatch {
                providers,
                ..Default::default()
            },
        )
        .unwrap();
        let cleared = String::from_utf8(fs::read(&path).unwrap()).unwrap();
        assert!(!cleared.contains("opencode-secret-value"));
        assert!(!cleared.contains("apiKey"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn protocol_patch_persists_npm_package_and_round_trips() {
        let (target, root) = temp_target();
        let path = root.join("opencode.jsonc");
        fs::write(
            &path,
            br#"{
  "provider": {
    "relay": {
      "name": "Relay",
      "options": { "baseURL": "https://relay.invalid/v1" },
      "models": { "relay-chat": { "name": "Relay Chat" } }
    }
  }
}"#,
        )
        .unwrap();
        let document = read_document(&target, &path).unwrap();
        let mut providers = BTreeMap::new();
        providers.insert(
            "relay".to_owned(),
            OpenCodeProviderPatch {
                protocol: Some(ProtocolKind::OpenaiChatCompletions),
                ..Default::default()
            },
        );
        save_opencode_profile(
            &target,
            &path,
            Some(&document.revision),
            &OpenCodeConfigPatch {
                providers,
                ..Default::default()
            },
        )
        .unwrap();
        let raw = String::from_utf8(fs::read(&path).unwrap()).unwrap();
        assert!(raw.contains("\"npm\": \"@ai-sdk/openai-compatible\""));
        let view = read_opencode_profile(&target, &path).unwrap();
        assert_eq!(
            view.providers[0].protocol,
            ProtocolKind::OpenaiChatCompletions
        );

        let document = read_document(&target, &path).unwrap();
        let mut providers = BTreeMap::new();
        providers.insert(
            "relay".to_owned(),
            OpenCodeProviderPatch {
                protocol: Some(ProtocolKind::AnthropicMessages),
                ..Default::default()
            },
        );
        save_opencode_profile(
            &target,
            &path,
            Some(&document.revision),
            &OpenCodeConfigPatch {
                providers,
                ..Default::default()
            },
        )
        .unwrap();
        let raw = String::from_utf8(fs::read(&path).unwrap()).unwrap();
        assert!(raw.contains("\"npm\": \"@ai-sdk/anthropic\""));
        let view = read_opencode_profile(&target, &path).unwrap();
        assert_eq!(view.providers[0].protocol, ProtocolKind::AnthropicMessages);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn patch_without_protocol_preserves_npm_and_ambiguous_packages_stay_unknown() {
        let (target, root) = temp_target();
        let path = root.join("opencode.json");
        fs::write(
            &path,
            br#"{"provider":{"openai":{"npm":"@ai-sdk/openai","models":{"gpt-5":{}}},"custom":{"models":{"m":{}}}}}"#,
        )
        .unwrap();
        let view = read_opencode_profile(&target, &path).unwrap();
        assert_eq!(view.providers[0].protocol, ProtocolKind::Unknown);
        assert_eq!(view.providers[1].protocol, ProtocolKind::Unknown);
        let document = read_document(&target, &path).unwrap();
        let mut providers = BTreeMap::new();
        providers.insert(
            "openai".to_owned(),
            OpenCodeProviderPatch {
                display_name: Some("OpenAI".to_owned()),
                ..Default::default()
            },
        );
        save_opencode_profile(
            &target,
            &path,
            Some(&document.revision),
            &OpenCodeConfigPatch {
                providers,
                ..Default::default()
            },
        )
        .unwrap();
        let raw = String::from_utf8(fs::read(&path).unwrap()).unwrap();
        assert!(raw.contains("\"npm\":\"@ai-sdk/openai\""));
        fs::remove_dir_all(root).unwrap();
    }

    fn windows_target(home: PathBuf) -> RuntimeTarget {
        RuntimeTarget {
            target_id: "runtime.windows.host".to_owned(),
            kind: RuntimeTargetKind::Host,
            platform: RuntimePlatform::Windows,
            distribution: None,
            display_name: "Windows host".to_owned(),
            home_path: NativeConfigPath::from_path(&home, RuntimePlatform::Windows),
            source: RuntimeTargetSource::Observed,
            capability_manifest_revision: None,
        }
    }

    #[test]
    fn windows_config_paths_prefer_xdg_over_roaming() {
        // opencode follows the XDG convention on Windows too, storing its
        // config under `~/.config/opencode` rather than `%APPDATA%\\opencode`.
        let home = env::temp_dir().join(format!("vibehub-opencode-win-{}", Uuid::new_v4()));
        fs::create_dir_all(&home).unwrap();
        let target = windows_target(home.clone());

        let paths = opencode_config_paths(&target);

        let xdg_jsonc = home.join(".config").join("opencode").join("opencode.jsonc");
        let xdg_json = home.join(".config").join("opencode").join("opencode.json");
        let roaming_jsonc = home
            .join("AppData")
            .join("Roaming")
            .join("opencode")
            .join("opencode.jsonc");
        let roaming_json = home
            .join("AppData")
            .join("Roaming")
            .join("opencode")
            .join("opencode.json");

        assert_eq!(
            paths,
            vec![xdg_jsonc.clone(), xdg_json, roaming_jsonc, roaming_json]
        );
        // XDG location must win over the legacy Roaming location so detection
        // no longer reports an empty environment for real Windows installs.
        assert_eq!(paths[0], xdg_jsonc);

        fs::remove_dir_all(&home).unwrap();
    }

    // Regression: criterion c02 requires that reading a complex variant object,
    // modifying an unrelated field, and saving preserves the full variant object
    // value. A patch that touches the model but carries no variant content must
    // not replace the complex variant with `{}`.
    #[test]
    fn patch_touching_model_without_variants_preserves_complex_variant_object() {
        let (target, root) = temp_target();
        let path = root.join("opencode.jsonc");
        let original = br#"{
  "model": "deepseek/deepseek-chat#high",
  "provider": {
    "deepseek": {
      "name": "DeepSeek",
      "models": {
        "deepseek-chat": {
          "name": "DeepSeek Chat",
          "variants": {
            "high": { "reasoningEffort": "high", "extra": { "keep": true } },
            "low": { "reasoningEffort": "low" }
          }
        }
      }
    }
  },
  "permissions": { "edit": "ask" }
}"#;
        fs::write(&path, original).unwrap();
        let document = read_document(&target, &path).unwrap();

        // Patch only the model display name; variants are intentionally omitted
        // (the UI has no complex-variant editor). This mirrors the
        // `patch_for_opencode` shape produced when an unrelated field changes.
        let mut models = BTreeMap::new();
        models.insert(
            "deepseek-chat".to_owned(),
            OpenCodeModelPatch {
                display_name: Some("DeepSeek Chat (renamed)".to_owned()),
                ..Default::default()
            },
        );
        let mut providers = BTreeMap::new();
        providers.insert(
            "deepseek".to_owned(),
            OpenCodeProviderPatch {
                models,
                ..Default::default()
            },
        );
        save_opencode_profile(
            &target,
            &path,
            Some(&document.revision),
            &OpenCodeConfigPatch {
                providers,
                ..Default::default()
            },
        )
        .unwrap();

        let raw = String::from_utf8(fs::read(&path).unwrap()).unwrap();
        // The complex variant object content must survive untouched.
        assert!(raw.contains("\"reasoningEffort\": \"high\""));
        assert!(raw.contains("\"extra\": { \"keep\": true }"));
        assert!(raw.contains("\"reasoningEffort\": \"low\""));
        // The unrelated field was still updated.
        assert!(raw.contains("DeepSeek Chat (renamed)"));
        // And unrelated root content is preserved.
        assert!(raw.contains("\"permissions\""));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn patch_explicitly_clears_variants_when_requested() {
        let (target, root) = temp_target();
        let path = root.join("opencode.json");
        fs::write(
            &path,
            br#"{
  "provider": {
    "deepseek": {
      "models": {
        "deepseek-chat": {
          "variants": { "high": { "reasoningEffort": "high" } }
        }
      }
    }
  }
}"#,
        )
        .unwrap();
        let document = read_document(&target, &path).unwrap();
        let mut models = BTreeMap::new();
        models.insert(
            "deepseek-chat".to_owned(),
            OpenCodeModelPatch {
                variants: Some(BTreeMap::new()),
                ..Default::default()
            },
        );
        let mut providers = BTreeMap::new();
        providers.insert(
            "deepseek".to_owned(),
            OpenCodeProviderPatch {
                models,
                ..Default::default()
            },
        );
        save_opencode_profile(
            &target,
            &path,
            Some(&document.revision),
            &OpenCodeConfigPatch {
                providers,
                ..Default::default()
            },
        )
        .unwrap();

        let updated = read_opencode_profile(&target, &path).unwrap();
        assert!(updated.providers[0].models[0].variants.is_empty());
        assert_eq!(
            updated.providers[0].models[0].variant_values,
            Some(BTreeMap::new())
        );
        fs::remove_dir_all(root).unwrap();
    }

    // Regression: criterion c02 — even when the patch does not mention the model
    // at all, a complex variant object must be preserved byte-for-byte.
    #[test]
    fn patch_without_model_preserves_complex_variant_object() {
        let (target, root) = temp_target();
        let path = root.join("opencode.jsonc");
        let original = br#"{
  "model": "deepseek/deepseek-chat#high",
  "provider": {
    "deepseek": {
      "models": {
        "deepseek-chat": {
          "variants": { "high": { "reasoningEffort": "high" } }
        }
      }
    }
  }
}"#;
        fs::write(&path, original).unwrap();
        let document = read_document(&target, &path).unwrap();
        save_opencode_profile(
            &target,
            &path,
            Some(&document.revision),
            &OpenCodeConfigPatch {
                default_model: Some("deepseek/deepseek-chat".to_owned()),
                ..Default::default()
            },
        )
        .unwrap();
        let raw = String::from_utf8(fs::read(&path).unwrap()).unwrap();
        assert!(raw.contains("\"reasoningEffort\": \"high\""));
        fs::remove_dir_all(root).unwrap();
    }

    // Regression: criterion c01 — tolerant discovery records a per-candidate
    // structured error (code, path, reason, recovery hint) and still returns the
    // readable candidates instead of aborting the whole scan.
    #[test]
    fn tolerant_discovery_records_error_and_keeps_valid_candidate() {
        let root = env::temp_dir().join(format!("vibehub-opencode-disc-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let target = windows_target(root.clone());
        let xdg = root.join(".config").join("opencode");
        fs::create_dir_all(&xdg).unwrap();
        let good = xdg.join("opencode.jsonc");
        fs::write(&good, br#"{ "model": "openai/gpt-4o" }"#).unwrap();
        // The legacy Roaming candidate exists but is invalid JSON.
        let roaming = root.join("AppData").join("Roaming").join("opencode");
        fs::create_dir_all(&roaming).unwrap();
        let bad = roaming.join("opencode.json");
        fs::write(&bad, br#"{ "model": "broken" "#).unwrap();

        let outcome = discover_opencode_profiles_tolerant(&target);

        assert_eq!(outcome.profiles.len(), 1);
        assert_eq!(
            outcome.profiles[0].default_model.as_deref(),
            Some("openai/gpt-4o")
        );
        assert_eq!(outcome.errors.len(), 1);
        let error = &outcome.errors[0];
        assert!(!error.code.is_empty());
        assert!(error.path.contains("opencode.json"));
        assert!(!error.message.is_empty());
        assert!(!error.recovery_hint.is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}

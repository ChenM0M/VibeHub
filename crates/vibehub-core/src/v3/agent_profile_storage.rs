use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use std::env;
use std::fmt::{Display, Formatter};
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
#[cfg(windows)]
use std::process::Command;
use uuid::Uuid;

const MAX_CONFIG_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    ClaudeCode,
    Opencode,
    Codex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTargetKind {
    Host,
    Wsl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimePlatform {
    Macos,
    Windows,
    Linux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTargetSource {
    Observed,
    UserSelected,
    Fixture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PathKind {
    Absolute,
    Drive,
    Unc,
    Extended,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeConfigPath {
    pub platform: RuntimePlatform,
    pub native: String,
    pub display: String,
    pub identity_key: String,
    pub path_kind: PathKind,
    pub accessible: Option<bool>,
}

impl NativeConfigPath {
    pub fn from_path(path: &Path, platform: RuntimePlatform) -> Self {
        let native = path.to_string_lossy().into_owned();
        let display = display_path(&native, platform);
        let path_kind = path_kind(&native, platform);
        Self {
            identity_key: format!("{platform:?}:{native}").to_lowercase(),
            platform,
            native,
            display,
            path_kind,
            accessible: None,
        }
    }

    pub fn as_path(&self) -> PathBuf {
        PathBuf::from(&self.native)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeTarget {
    pub target_id: String,
    pub kind: RuntimeTargetKind,
    pub platform: RuntimePlatform,
    pub distribution: Option<String>,
    pub display_name: String,
    pub home_path: NativeConfigPath,
    pub source: RuntimeTargetSource,
    pub capability_manifest_revision: Option<String>,
}

impl RuntimeTarget {
    pub fn host(home: PathBuf) -> Self {
        let platform = current_platform();
        Self {
            target_id: format!("runtime.{}.host", platform_name(platform)),
            kind: RuntimeTargetKind::Host,
            platform,
            distribution: None,
            display_name: format!("{} host", platform_display_name(platform)),
            home_path: NativeConfigPath::from_path(&home, platform),
            source: RuntimeTargetSource::Observed,
            capability_manifest_revision: None,
        }
    }

    pub fn wsl(distribution: impl Into<String>, home: impl Into<String>) -> Self {
        let distribution = distribution.into();
        let home = home.into();
        let safe_distribution = stable_component(&distribution);
        let native = if cfg!(windows) {
            format!(r"\\wsl$\{distribution}{}", home.replace('/', r"\"))
        } else {
            home.clone()
        };
        Self {
            target_id: format!("runtime.wsl.{safe_distribution}"),
            kind: RuntimeTargetKind::Wsl,
            platform: RuntimePlatform::Linux,
            distribution: Some(distribution.clone()),
            display_name: format!("WSL {distribution}"),
            home_path: NativeConfigPath {
                platform: RuntimePlatform::Linux,
                native: native.clone(),
                display: native,
                identity_key: format!("linux:{distribution}:{home}"),
                path_kind: PathKind::Absolute,
                accessible: None,
            },
            source: RuntimeTargetSource::Observed,
            capability_manifest_revision: None,
        }
    }
}

pub fn discover_runtime_targets() -> Result<Vec<RuntimeTarget>, StorageError> {
    if let Ok(distribution) = env::var("WSL_DISTRO_NAME") {
        let home = home_directory()?;
        return Ok(vec![RuntimeTarget::wsl(
            distribution,
            home.to_string_lossy(),
        )]);
    }

    #[cfg(windows)]
    let mut targets = vec![RuntimeTarget::host(home_directory()?)];
    #[cfg(not(windows))]
    let targets = vec![RuntimeTarget::host(home_directory()?)];
    #[cfg(windows)]
    {
        if let Ok(distributions) = discover_wsl_distributions() {
            for distribution in distributions {
                if let Ok(home) = wsl_home(&distribution) {
                    targets.push(RuntimeTarget::wsl(distribution, home));
                }
            }
        }
    }
    Ok(targets)
}

#[cfg(windows)]
fn discover_wsl_distributions() -> Result<Vec<String>, StorageError> {
    let output = Command::new("wsl.exe")
        .args(["-l", "-q"])
        .output()
        .map_err(|error| StorageError::new("RUNTIME_WSL_DISCOVERY_FAILED", error.to_string()))?;
    if !output.status.success() {
        return Err(StorageError::new(
            "RUNTIME_WSL_DISCOVERY_FAILED",
            format!("wsl.exe exited with {}", output.status),
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text
        .replace('\0', "")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect())
}

#[cfg(windows)]
fn wsl_home(distribution: &str) -> Result<String, StorageError> {
    let output = Command::new("wsl.exe")
        .args(["-d", distribution, "--", "sh", "-lc", "printf %s \"$HOME\""])
        .output()
        .map_err(|error| {
            StorageError::new(
                "RUNTIME_WSL_HOME_DISCOVERY_FAILED",
                format!("failed to inspect WSL distribution {distribution}: {error}"),
            )
        })?;
    if !output.status.success() {
        return Err(StorageError::new(
            "RUNTIME_WSL_HOME_DISCOVERY_FAILED",
            format!(
                "WSL distribution {distribution} returned status {} while resolving HOME",
                output.status
            ),
        ));
    }
    let home = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    validate_wsl_home(distribution, &home)
}

#[cfg(any(windows, test))]
fn validate_wsl_home(distribution: &str, home: &str) -> Result<String, StorageError> {
    if home.is_empty() || !home.starts_with('/') || home.contains('\0') {
        return Err(StorageError::new(
            "RUNTIME_WSL_HOME_DISCOVERY_FAILED",
            format!("WSL distribution {distribution} did not return an absolute HOME path"),
        ));
    }
    Ok(home.to_owned())
}

fn home_directory() -> Result<PathBuf, StorageError> {
    #[cfg(windows)]
    {
        if let Ok(home) = env::var("USERPROFILE") {
            if !home.trim().is_empty() {
                return Ok(PathBuf::from(home));
            }
        }
        if let (Ok(drive), Ok(path)) = (env::var("HOMEDRIVE"), env::var("HOMEPATH")) {
            return Ok(PathBuf::from(format!("{drive}{path}")));
        }
    }
    #[cfg(not(windows))]
    if let Ok(home) = env::var("HOME") {
        if !home.trim().is_empty() {
            return Ok(PathBuf::from(home));
        }
    }
    Err(StorageError::new(
        "RUNTIME_HOME_UNAVAILABLE",
        "the target home directory is not available from the environment",
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigFormat {
    Json,
    Jsonc,
    Toml,
}

impl ConfigFormat {
    pub fn from_path(path: &Path) -> Result<Self, StorageError> {
        match path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref()
        {
            Some("json") => Ok(Self::Json),
            Some("jsonc") => Ok(Self::Jsonc),
            Some("toml") => Ok(Self::Toml),
            _ => Err(StorageError::new(
                "CONFIG_FORMAT_UNSUPPORTED",
                format!("unsupported config extension: {}", path.display()),
            )),
        }
    }

    fn validate(self, bytes: &[u8]) -> Result<ParsedConfig, StorageError> {
        let text = std::str::from_utf8(bytes)
            .map_err(|error| StorageError::new("CONFIG_ENCODING_INVALID", error.to_string()))?;
        match self {
            Self::Json => serde_json::from_str::<JsonValue>(text)
                .map(ParsedConfig::Json)
                .map_err(|error| StorageError::new("CONFIG_JSON_INVALID", error.to_string())),
            Self::Jsonc => {
                let normalized = strip_jsonc_comments_and_trailing_commas(text)?;
                serde_json::from_str::<JsonValue>(&normalized)
                    .map(ParsedConfig::Json)
                    .map_err(|error| StorageError::new("CONFIG_JSONC_INVALID", error.to_string()))
            }
            Self::Toml => toml::from_str::<toml::Value>(text)
                .map(ParsedConfig::Toml)
                .map_err(|error| StorageError::new("CONFIG_TOML_INVALID", error.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParsedConfig {
    Json(JsonValue),
    Toml(toml::Value),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentRevision {
    pub content_sha256: String,
    pub byte_length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDocument {
    pub path: PathBuf,
    pub format: ConfigFormat,
    pub raw: Vec<u8>,
    pub revision: DocumentRevision,
}

impl ConfigDocument {
    pub fn parse(&self) -> Result<ParsedConfig, StorageError> {
        self.format.validate(&self.raw)
    }

    pub fn round_trip(&self) -> Vec<u8> {
        self.raw.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteReport {
    pub path: NativeConfigPath,
    pub before_revision: Option<DocumentRevision>,
    pub after_revision: DocumentRevision,
    pub backup_path: Option<NativeConfigPath>,
    pub rollback_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageError {
    pub code: &'static str,
    pub message: String,
}

impl StorageError {
    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl Display for StorageError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for StorageError {}

pub fn read_document(
    target: &RuntimeTarget,
    path: impl AsRef<Path>,
) -> Result<ConfigDocument, StorageError> {
    let path = validate_target_path(target, path.as_ref(), false)?;
    let format = ConfigFormat::from_path(&path)?;
    let metadata = fs::symlink_metadata(&path)
        .map_err(|error| StorageError::new("CONFIG_READ_FAILED", error.to_string()))?;
    reject_link_or_reparse(&metadata, &path)?;
    if !metadata.is_file() {
        return Err(StorageError::new(
            "CONFIG_NOT_REGULAR_FILE",
            path.display().to_string(),
        ));
    }
    if metadata.len() > MAX_CONFIG_BYTES {
        return Err(StorageError::new(
            "CONFIG_TOO_LARGE",
            format!("{} bytes exceeds the configured limit", metadata.len()),
        ));
    }
    let raw = fs::read(&path)
        .map_err(|error| StorageError::new("CONFIG_READ_FAILED", error.to_string()))?;
    format.validate(&raw)?;
    let revision = revision_for(&raw);
    Ok(ConfigDocument {
        path,
        format,
        raw,
        revision,
    })
}

pub fn write_document(
    target: &RuntimeTarget,
    path: impl AsRef<Path>,
    expected_revision: Option<&DocumentRevision>,
    content: &[u8],
) -> Result<WriteReport, StorageError> {
    let path = validate_target_path(target, path.as_ref(), true)?;
    let format = ConfigFormat::from_path(&path)?;
    if content.len() as u64 > MAX_CONFIG_BYTES {
        return Err(StorageError::new(
            "CONFIG_TOO_LARGE",
            format!("{} bytes exceeds the configured limit", content.len()),
        ));
    }
    format.validate(content)?;

    let current = match read_document(target, &path) {
        Ok(document) => Some(document),
        Err(error) if error.code == "CONFIG_NOT_FOUND" => None,
        Err(error) => return Err(error),
    };
    let current_revision = current.as_ref().map(|document| &document.revision);
    if !same_revision(expected_revision, current_revision) {
        return Err(StorageError::new(
            "CONFIG_REVISION_CONFLICT",
            "the file changed after it was read; reload before saving",
        ));
    }

    let backup_path = if let Some(document) = current.as_ref() {
        Some(create_backup(&document.path, &document.revision)?)
    } else {
        None
    };
    let original_permissions = current
        .as_ref()
        .and_then(|document| fs::metadata(&document.path).ok())
        .map(|metadata| metadata.permissions());
    let temporary = temporary_path(&path);
    let write_result =
        write_temporary_and_replace(&temporary, &path, content, original_permissions.as_ref());
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }

    let after_revision = revision_for(content);
    Ok(WriteReport {
        path: NativeConfigPath::from_path(&path, target.platform),
        before_revision: current_revision.cloned(),
        after_revision,
        backup_path: backup_path
            .as_ref()
            .map(|path| NativeConfigPath::from_path(path, target.platform)),
        rollback_available: backup_path.is_some(),
    })
}

pub fn restore_document(
    target: &RuntimeTarget,
    path: impl AsRef<Path>,
    backup_path: impl AsRef<Path>,
    expected_current_revision: &DocumentRevision,
) -> Result<WriteReport, StorageError> {
    let path = validate_target_path(target, path.as_ref(), false)?;
    let backup_path = validate_target_path(target, backup_path.as_ref(), false)?;
    let backup = read_raw_regular_file(&backup_path)?;
    let current = read_document(target, &path)?;
    if current.revision != *expected_current_revision {
        return Err(StorageError::new(
            "CONFIG_REVISION_CONFLICT",
            "the file changed after the failed save; refusing to restore an old backup",
        ));
    }
    write_document(target, &path, Some(&current.revision), &backup)
}

fn same_revision(expected: Option<&DocumentRevision>, actual: Option<&DocumentRevision>) -> bool {
    match (expected, actual) {
        (None, None) => true,
        (Some(expected), Some(actual)) => expected == actual,
        _ => false,
    }
}

fn read_raw_regular_file(path: &Path) -> Result<Vec<u8>, StorageError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| StorageError::new("CONFIG_BACKUP_READ_FAILED", error.to_string()))?;
    reject_link_or_reparse(&metadata, path)?;
    if !metadata.is_file() {
        return Err(StorageError::new(
            "CONFIG_BACKUP_NOT_REGULAR_FILE",
            path.display().to_string(),
        ));
    }
    if metadata.len() > MAX_CONFIG_BYTES {
        return Err(StorageError::new(
            "CONFIG_BACKUP_TOO_LARGE",
            "backup exceeds the configured limit",
        ));
    }
    fs::read(path)
        .map_err(|error| StorageError::new("CONFIG_BACKUP_READ_FAILED", error.to_string()))
}

fn create_backup(path: &Path, revision: &DocumentRevision) -> Result<PathBuf, StorageError> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            StorageError::new("CONFIG_BACKUP_PATH_INVALID", path.display().to_string())
        })?;
    let suffix = revision
        .content_sha256
        .get(..12)
        .unwrap_or(&revision.content_sha256);
    let backup = path
        .parent()
        .ok_or_else(|| StorageError::new("CONFIG_BACKUP_PATH_INVALID", path.display().to_string()))?
        .join(format!(".{file_name}.vibehub.{suffix}.bak"));
    let metadata = fs::metadata(path)
        .map_err(|error| StorageError::new("CONFIG_BACKUP_FAILED", error.to_string()))?;
    copy_file_with_permissions(path, &backup, &metadata.permissions())?;
    Ok(backup)
}

fn copy_file_with_permissions(
    source: &Path,
    destination: &Path,
    permissions: &fs::Permissions,
) -> Result<(), StorageError> {
    fs::copy(source, destination)
        .map_err(|error| StorageError::new("CONFIG_BACKUP_FAILED", error.to_string()))?;
    fs::set_permissions(destination, permissions.clone()).map_err(|error| {
        StorageError::new("CONFIG_BACKUP_PERMISSIONS_FAILED", error.to_string())
    })?;
    Ok(())
}

fn write_temporary_and_replace(
    temporary: &Path,
    destination: &Path,
    content: &[u8],
    permissions: Option<&fs::Permissions>,
) -> Result<(), StorageError> {
    let parent = destination.parent().ok_or_else(|| {
        StorageError::new("CONFIG_PARENT_INVALID", destination.display().to_string())
    })?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(temporary)
        .map_err(|error| StorageError::new("CONFIG_TEMP_CREATE_FAILED", error.to_string()))?;
    if let Err(error) = file.write_all(content).and_then(|_| file.sync_all()) {
        let _ = fs::remove_file(temporary);
        return Err(StorageError::new(
            "CONFIG_TEMP_WRITE_FAILED",
            error.to_string(),
        ));
    }
    if let Some(permissions) = permissions {
        fs::set_permissions(temporary, permissions.clone()).map_err(|error| {
            StorageError::new("CONFIG_TEMP_PERMISSIONS_FAILED", error.to_string())
        })?;
    }
    replace_file(temporary, destination)?;
    sync_directory(parent)?;
    Ok(())
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> Result<(), StorageError> {
    fs::rename(source, destination)
        .map_err(|error| StorageError::new("CONFIG_ATOMIC_REPLACE_FAILED", error.to_string()))
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> Result<(), StorageError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let source: Vec<u16> = source.as_os_str().encode_wide().chain([0]).collect();
    let destination: Vec<u16> = destination.as_os_str().encode_wide().chain([0]).collect();
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        return Err(StorageError::new(
            "CONFIG_ATOMIC_REPLACE_FAILED",
            io::Error::last_os_error().to_string(),
        ));
    }
    Ok(())
}

fn sync_directory(path: &Path) -> Result<(), StorageError> {
    #[cfg(unix)]
    {
        File::open(path)
            .and_then(|file| file.sync_all())
            .map_err(|error| {
                StorageError::new("CONFIG_DIRECTORY_SYNC_FAILED", error.to_string())
            })?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

fn temporary_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config");
    path.parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!(".{file_name}.vibehub.{}.tmp", Uuid::new_v4()))
}

fn validate_target_path(
    target: &RuntimeTarget,
    path: &Path,
    allow_missing_file: bool,
) -> Result<PathBuf, StorageError> {
    if !path.is_absolute() {
        return Err(StorageError::new(
            "CONFIG_PATH_NOT_ABSOLUTE",
            path.display().to_string(),
        ));
    }
    let raw_home = target.home_path.as_path();
    let home_metadata = fs::symlink_metadata(&raw_home)
        .map_err(|error| StorageError::new("RUNTIME_HOME_INVALID", error.to_string()))?;
    reject_link_or_reparse(&home_metadata, &raw_home)?;
    if !home_metadata.is_dir() {
        return Err(StorageError::new(
            "RUNTIME_HOME_NOT_DIRECTORY",
            raw_home.display().to_string(),
        ));
    }
    let home = raw_home
        .canonicalize()
        .map_err(|error| StorageError::new("RUNTIME_HOME_INVALID", error.to_string()))?;
    let component_root = if path.strip_prefix(&raw_home).is_ok() {
        raw_home.as_path()
    } else if path.strip_prefix(&home).is_ok() {
        home.as_path()
    } else {
        return Err(StorageError::new(
            "CONFIG_PATH_OUTSIDE_RUNTIME_HOME",
            path.display().to_string(),
        ));
    };
    reject_path_components(component_root, path, allow_missing_file)?;
    let candidate = if path.exists() {
        path.canonicalize()
            .map_err(|error| StorageError::new("CONFIG_PATH_INVALID", error.to_string()))?
    } else {
        let parent = path.parent().ok_or_else(|| {
            StorageError::new("CONFIG_PARENT_INVALID", path.display().to_string())
        })?;
        parent
            .canonicalize()
            .map_err(|error| StorageError::new("CONFIG_PARENT_INVALID", error.to_string()))?
            .join(path.file_name().ok_or_else(|| {
                StorageError::new("CONFIG_PATH_INVALID", path.display().to_string())
            })?)
    };
    if !candidate.starts_with(&home) || candidate == home {
        return Err(StorageError::new(
            "CONFIG_PATH_OUTSIDE_RUNTIME_HOME",
            candidate.display().to_string(),
        ));
    }
    Ok(candidate)
}

fn reject_path_components(
    raw_home: &Path,
    path: &Path,
    allow_missing_file: bool,
) -> Result<(), StorageError> {
    let relative = path.strip_prefix(raw_home).map_err(|_| {
        StorageError::new(
            "CONFIG_PATH_OUTSIDE_RUNTIME_HOME",
            path.display().to_string(),
        )
    })?;
    let mut cursor = raw_home.to_path_buf();
    let components: Vec<Component<'_>> = relative.components().collect();
    for (index, component) in components.iter().enumerate() {
        if matches!(
            component,
            Component::CurDir | Component::ParentDir | Component::RootDir | Component::Prefix(_)
        ) {
            return Err(StorageError::new(
                "CONFIG_PATH_NORMALIZATION_REQUIRED",
                path.display().to_string(),
            ));
        }
        cursor.push(component.as_os_str());
        match fs::symlink_metadata(&cursor) {
            Ok(metadata) => {
                reject_link_or_reparse(&metadata, &cursor)?;
                if index + 1 != components.len() && !metadata.is_dir() {
                    return Err(StorageError::new(
                        "CONFIG_PATH_PARENT_NOT_DIRECTORY",
                        cursor.display().to_string(),
                    ));
                }
            }
            Err(error)
                if error.kind() == io::ErrorKind::NotFound
                    && allow_missing_file
                    && index + 1 == components.len() => {}
            Err(error) => {
                return Err(StorageError::new("CONFIG_PATH_INVALID", error.to_string()));
            }
        }
    }
    Ok(())
}

fn reject_link_or_reparse(metadata: &Metadata, path: &Path) -> Result<(), StorageError> {
    if metadata.file_type().is_symlink() || is_reparse_point(metadata) {
        return Err(StorageError::new(
            "CONFIG_PATH_LINK_REJECTED",
            path.display().to_string(),
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn is_reparse_point(metadata: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse_point(_metadata: &Metadata) -> bool {
    false
}

fn revision_for(raw: &[u8]) -> DocumentRevision {
    let mut hasher = Sha256::new();
    hasher.update(raw);
    DocumentRevision {
        content_sha256: format!("{:x}", hasher.finalize()),
        byte_length: raw.len() as u64,
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
                    "CONFIG_JSONC_COMMENT_UNTERMINATED",
                    "unterminated block comment",
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
            "CONFIG_JSONC_STRING_UNTERMINATED",
            "unterminated JSON string",
        ));
    }
    Ok(output)
}

fn current_platform() -> RuntimePlatform {
    #[cfg(target_os = "macos")]
    {
        return RuntimePlatform::Macos;
    }
    #[cfg(windows)]
    {
        return RuntimePlatform::Windows;
    }
    #[cfg(all(not(target_os = "macos"), not(windows)))]
    {
        RuntimePlatform::Linux
    }
}

fn platform_name(platform: RuntimePlatform) -> &'static str {
    match platform {
        RuntimePlatform::Macos => "macos",
        RuntimePlatform::Windows => "windows",
        RuntimePlatform::Linux => "linux",
    }
}

fn platform_display_name(platform: RuntimePlatform) -> &'static str {
    match platform {
        RuntimePlatform::Macos => "macOS",
        RuntimePlatform::Windows => "Windows",
        RuntimePlatform::Linux => "Linux",
    }
}

fn display_path(native: &str, platform: RuntimePlatform) -> String {
    if matches!(platform, RuntimePlatform::Windows) {
        native.replace('\\', "/")
    } else {
        native.to_owned()
    }
}

fn path_kind(native: &str, platform: RuntimePlatform) -> PathKind {
    if matches!(platform, RuntimePlatform::Windows) {
        if native.starts_with("\\\\?\\") {
            PathKind::Extended
        } else if native.starts_with(r"\\") {
            PathKind::Unc
        } else {
            PathKind::Drive
        }
    } else {
        PathKind::Absolute
    }
}

fn stable_component(value: &str) -> String {
    let mut result = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
            result.push(character.to_ascii_lowercase());
        } else {
            result.push('-');
        }
    }
    result.trim_matches('-').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_home() -> (RuntimeTarget, PathBuf) {
        let root =
            env::temp_dir().join(format!("vibehub-agent-profile-storage-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let target = RuntimeTarget::host(root.clone());
        (target, root)
    }

    #[test]
    fn jsonc_round_trip_preserves_comments_and_trailing_comma() {
        let (target, root) = temp_home();
        let path = root.join("settings.jsonc");
        let content = br#"{ 
  // user comment
  "model": "deepseek-chat",
  "unknown": { "keep": true, },
}
"#;
        fs::write(&path, content).unwrap();
        let document = read_document(&target, &path).unwrap();
        assert_eq!(document.round_trip(), content);
        assert!(matches!(document.parse().unwrap(), ParsedConfig::Json(_)));
        let report = write_document(&target, &path, Some(&document.revision), content).unwrap();
        assert_eq!(report.before_revision, Some(document.revision));
        assert_eq!(fs::read(&path).unwrap(), content);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn toml_round_trip_preserves_comments() {
        let (target, root) = temp_home();
        let path = root.join("config.toml");
        let content = b"# keep this comment\nmodel = \"deepseek-chat\"\nunknown = \"untouched\"\n";
        fs::write(&path, content).unwrap();
        let document = read_document(&target, &path).unwrap();
        assert!(matches!(document.parse().unwrap(), ParsedConfig::Toml(_)));
        write_document(
            &target,
            &path,
            Some(&document.revision),
            &document.round_trip(),
        )
        .unwrap();
        assert_eq!(fs::read(&path).unwrap(), content);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn revision_conflict_does_not_overwrite_external_change() {
        let (target, root) = temp_home();
        let path = root.join("config.json");
        fs::write(&path, br#"{"model":"first"}"#).unwrap();
        let document = read_document(&target, &path).unwrap();
        fs::write(&path, br#"{"model":"external"}"#).unwrap();
        let error = write_document(
            &target,
            &path,
            Some(&document.revision),
            br#"{"model":"mine"}"#,
        )
        .unwrap_err();
        assert_eq!(error.code, "CONFIG_REVISION_CONFLICT");
        assert_eq!(fs::read(&path).unwrap(), br#"{"model":"external"}"#);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn write_creates_backup_and_restore_returns_previous_content() {
        let (target, root) = temp_home();
        let path = root.join("config.json");
        fs::write(&path, br#"{"model":"first"}"#).unwrap();
        let first = read_document(&target, &path).unwrap();
        let report = write_document(
            &target,
            &path,
            Some(&first.revision),
            br#"{"model":"second"}"#,
        )
        .unwrap();
        let backup = report.backup_path.as_ref().unwrap().as_path();
        assert!(backup.exists());
        let second = read_document(&target, &path).unwrap();
        restore_document(&target, &path, backup, &second.revision).unwrap();
        assert_eq!(fs::read(&path).unwrap(), br#"{"model":"first"}"#);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn path_outside_target_home_is_rejected() {
        let (target, root) = temp_home();
        let outside = root.parent().unwrap().join("outside.json");
        fs::write(&outside, br#"{}"#).unwrap();
        let error = read_document(&target, &outside).unwrap_err();
        assert_eq!(error.code, "CONFIG_PATH_OUTSIDE_RUNTIME_HOME");
        fs::remove_file(outside).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_config_is_rejected_without_touching_target() {
        use std::os::unix::fs::symlink;
        let (target, root) = temp_home();
        let outside = root.parent().unwrap().join("agent-profile-outside.json");
        let link = root.join("config.json");
        fs::write(&outside, br#"{"secret":"outside"}"#).unwrap();
        symlink(&outside, &link).unwrap();
        let error = read_document(&target, &link).unwrap_err();
        assert_eq!(error.code, "CONFIG_PATH_LINK_REJECTED");
        assert_eq!(fs::read(&outside).unwrap(), br#"{"secret":"outside"}"#);
        fs::remove_file(link).unwrap();
        fs::remove_file(outside).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn runtime_targets_keep_wsl_distributions_independent() {
        let first = RuntimeTarget::wsl("Ubuntu", "/home/alex");
        let second = RuntimeTarget::wsl("Debian", "/home/alex");
        assert_ne!(first.target_id, second.target_id);
        assert_eq!(first.kind, RuntimeTargetKind::Wsl);
        assert_eq!(first.platform, RuntimePlatform::Linux);
        assert_ne!(first.home_path.identity_key, second.home_path.identity_key);
    }

    #[test]
    fn wsl_home_discovery_rejects_guesses_and_non_absolute_paths() {
        assert_eq!(
            validate_wsl_home("Ubuntu", "/home/alex").unwrap(),
            "/home/alex"
        );
        assert_eq!(
            validate_wsl_home("Ubuntu", "").unwrap_err().code,
            "RUNTIME_WSL_HOME_DISCOVERY_FAILED"
        );
        assert_eq!(
            validate_wsl_home("Ubuntu", "home/alex").unwrap_err().code,
            "RUNTIME_WSL_HOME_DISCOVERY_FAILED"
        );
    }

    #[test]
    fn jsonc_scanner_does_not_strip_comment_like_text_inside_strings() {
        let parsed = strip_jsonc_comments_and_trailing_commas(
            r#"{"url":"https://example.invalid/a//b","text":"/* keep */"}"#,
        )
        .unwrap();
        assert!(parsed.contains("https://example.invalid/a//b"));
        assert!(parsed.contains("/* keep */"));
    }

    #[test]
    fn invalid_documents_are_rejected_before_write() {
        let (target, root) = temp_home();
        let path = root.join("config.json");
        let error = write_document(&target, &path, None, br#"{"broken":"#).unwrap_err();
        assert_eq!(error.code, "CONFIG_JSON_INVALID");
        assert!(!path.exists());
        fs::remove_dir_all(root).unwrap();
    }
}

use super::{inspect_project_layout, ProjectLayoutState, V3Error, V3ErrorCategory};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const SETTINGS_SCHEMA_VERSION: u32 = 1;
const SETTINGS_KIND: &str = "v3_project_settings";
const SETTINGS_FILE: &str = "project-settings.yaml";
const MAX_SETTINGS_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputLanguage {
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "zh-TW")]
    ZhTw,
    #[serde(rename = "en-US")]
    EnUs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSpecTarget {
    ClaudeCode,
    Opencode,
    Codex,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct V3ProjectSettings {
    pub schema_version: u32,
    pub kind: String,
    pub revision: u64,
    pub output_language: OutputLanguage,
    pub agent_spec_targets: Vec<AgentSpecTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository_remote_url: Option<String>,
    pub updated_at: String,
    pub updated_by: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectSettingsStatus {
    Missing,
    Present,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V3ProjectSettingsInspection {
    pub status: ProjectSettingsStatus,
    pub settings: Option<V3ProjectSettings>,
    pub recommended_action: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct V3ProjectSettingsUpdateRequest {
    /// Use zero when creating project settings for the first time.
    pub expected_revision: u64,
    pub output_language: OutputLanguage,
    pub agent_spec_targets: Vec<AgentSpecTarget>,
    #[serde(default)]
    pub repository_remote_url: Option<String>,
}

pub fn inspect_project_settings(
    project_root: impl AsRef<Path>,
) -> Result<V3ProjectSettingsInspection, V3Error> {
    let paths = validated_paths(project_root.as_ref())?;
    if !settings_file_exists(&paths.settings)? {
        return Ok(V3ProjectSettingsInspection {
            status: ProjectSettingsStatus::Missing,
            settings: None,
            recommended_action: Some(
                "create settings with expected_revision 0 and at least one agent_spec_target"
                    .to_owned(),
            ),
        });
    }

    let settings = read_existing(&paths.settings)?;
    Ok(V3ProjectSettingsInspection {
        status: ProjectSettingsStatus::Present,
        settings: Some(settings),
        recommended_action: None,
    })
}

pub fn read_project_settings(
    project_root: impl AsRef<Path>,
) -> Result<V3ProjectSettingsInspection, V3Error> {
    inspect_project_settings(project_root)
}

pub fn update_project_settings(
    project_root: impl AsRef<Path>,
    request: V3ProjectSettingsUpdateRequest,
) -> Result<V3ProjectSettings, V3Error> {
    let paths = validated_paths(project_root.as_ref())?;
    if request.agent_spec_targets.is_empty() {
        return Err(validation(
            "V3_PROJECT_SETTINGS_TARGETS_EMPTY",
            "agent_spec_targets must contain at least one target",
        ));
    }

    let current = if settings_file_exists(&paths.settings)? {
        Some(read_existing(&paths.settings)?)
    } else {
        None
    };
    let actual_revision = current.as_ref().map_or(0, |settings| settings.revision);
    if request.expected_revision != actual_revision {
        return Err(V3Error::new(
            "V3_PROJECT_SETTINGS_REVISION_CONFLICT",
            V3ErrorCategory::VersionConflict,
            true,
            format!(
                "project settings revision mismatch: expected {}, actual {}",
                request.expected_revision, actual_revision
            ),
        )
        .with_detail("expected_revision", request.expected_revision)
        .with_detail("actual_revision", actual_revision));
    }

    let mut targets = request.agent_spec_targets;
    targets.sort_unstable();
    targets.dedup();
    let repository_remote_url = request
        .repository_remote_url
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    let settings = V3ProjectSettings {
        schema_version: SETTINGS_SCHEMA_VERSION,
        kind: SETTINGS_KIND.to_owned(),
        revision: actual_revision + 1,
        output_language: request.output_language,
        agent_spec_targets: targets,
        repository_remote_url,
        updated_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        updated_by: "vibehub".to_owned(),
    };
    let yaml = serde_yaml::to_string(&settings)
        .map_err(|error| validation("V3_PROJECT_SETTINGS_SERIALIZE_FAILED", error.to_string()))?;
    atomic_replace(&paths.settings, yaml.as_bytes())?;
    Ok(settings)
}

struct SettingsPaths {
    settings: PathBuf,
}

fn validated_paths(project_root: &Path) -> Result<SettingsPaths, V3Error> {
    let project_root = project_root
        .canonicalize()
        .map_err(|error| io_error("V3_PROJECT_SETTINGS_ROOT_NOT_FOUND", error))?;
    let layout = inspect_project_layout(&project_root)?;
    if layout.state != ProjectLayoutState::V3 {
        return Err(validation(
            "V3_PROJECT_SETTINGS_REQUIRES_V3",
            format!(
                "project settings require a V3 project, found {:?}",
                layout.state
            ),
        ));
    }
    let vibehub = project_root.join(".vibehub");
    let metadata = fs::symlink_metadata(&vibehub)
        .map_err(|error| io_error("V3_PROJECT_SETTINGS_ROOT_INVALID", error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(validation(
            "V3_PROJECT_SETTINGS_ROOT_INVALID",
            ".vibehub must be a regular directory and must not be a symbolic link",
        ));
    }
    Ok(SettingsPaths {
        settings: vibehub.join(SETTINGS_FILE),
    })
}

fn settings_file_exists(path: &Path) -> Result<bool, V3Error> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(io_error("V3_PROJECT_SETTINGS_READ_FAILED", error)),
    }
}

fn read_existing(path: &Path) -> Result<V3ProjectSettings, V3Error> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| io_error("V3_PROJECT_SETTINGS_READ_FAILED", error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(validation(
            "V3_PROJECT_SETTINGS_FILE_INVALID",
            "project-settings.yaml must be a regular file and must not be a symbolic link",
        ));
    }
    if metadata.len() > MAX_SETTINGS_BYTES {
        return Err(validation(
            "V3_PROJECT_SETTINGS_FILE_TOO_LARGE",
            "project-settings.yaml exceeds the 64 KiB limit",
        ));
    }
    let bytes =
        fs::read(path).map_err(|error| io_error("V3_PROJECT_SETTINGS_READ_FAILED", error))?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|error| validation("V3_PROJECT_SETTINGS_INVALID_UTF8", error.to_string()))?;
    let settings: V3ProjectSettings = serde_yaml::from_str(text)
        .map_err(|error| validation("V3_PROJECT_SETTINGS_INVALID_YAML", error.to_string()))?;
    validate_document(&settings)?;
    Ok(settings)
}

fn validate_document(settings: &V3ProjectSettings) -> Result<(), V3Error> {
    if settings.schema_version != SETTINGS_SCHEMA_VERSION || settings.kind != SETTINGS_KIND {
        return Err(validation(
            "V3_PROJECT_SETTINGS_SCHEMA_MISMATCH",
            "project settings schema_version or kind is invalid",
        ));
    }
    if settings.revision < 1 {
        return Err(validation(
            "V3_PROJECT_SETTINGS_REVISION_INVALID",
            "project settings revision must be at least 1",
        ));
    }
    if settings.agent_spec_targets.is_empty() {
        return Err(validation(
            "V3_PROJECT_SETTINGS_TARGETS_EMPTY",
            "agent_spec_targets must contain at least one target",
        ));
    }
    let mut normalized = settings.agent_spec_targets.clone();
    normalized.sort_unstable();
    normalized.dedup();
    if normalized != settings.agent_spec_targets {
        return Err(validation(
            "V3_PROJECT_SETTINGS_TARGETS_NOT_NORMALIZED",
            "agent_spec_targets must be unique and in stable order",
        ));
    }
    if settings.updated_at.trim().is_empty() || settings.updated_by.trim().is_empty() {
        return Err(validation(
            "V3_PROJECT_SETTINGS_METADATA_INVALID",
            "updated_at and updated_by must not be empty",
        ));
    }
    Ok(())
}

fn atomic_replace(path: &Path, content: &[u8]) -> Result<(), V3Error> {
    let parent = path.parent().ok_or_else(|| {
        validation(
            "V3_PROJECT_SETTINGS_WRITE_FAILED",
            "settings path does not have a parent",
        )
    })?;
    let temporary = parent.join(format!(".{SETTINGS_FILE}.{}.tmp", Uuid::new_v4()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| io_error("V3_PROJECT_SETTINGS_WRITE_FAILED", error))?;
        file.write_all(content)
            .and_then(|_| file.sync_all())
            .map_err(|error| io_error("V3_PROJECT_SETTINGS_WRITE_FAILED", error))?;
        replace_file(&temporary, path)
            .map_err(|error| io_error("V3_PROJECT_SETTINGS_WRITE_FAILED", error))?;
        sync_directory(parent).map_err(|error| io_error("V3_PROJECT_SETTINGS_SYNC_FAILED", error))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
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

fn validation(code: &str, message: impl Into<String>) -> V3Error {
    V3Error::new(code, V3ErrorCategory::Validation, false, message)
}

fn io_error(code: &str, error: std::io::Error) -> V3Error {
    V3Error::new(code, V3ErrorCategory::Internal, false, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v3::initialize_v3;

    fn temp_project() -> PathBuf {
        let root = std::env::temp_dir().join(format!("vibehub-v3-settings-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        initialize_v3(&root).unwrap();
        root
    }

    fn request(expected_revision: u64) -> V3ProjectSettingsUpdateRequest {
        V3ProjectSettingsUpdateRequest {
            expected_revision,
            output_language: OutputLanguage::ZhCn,
            agent_spec_targets: vec![AgentSpecTarget::Codex, AgentSpecTarget::ClaudeCode],
            repository_remote_url: Some("https://example.invalid/repository.git".to_owned()),
        }
    }

    #[test]
    fn missing_read_does_not_write() {
        let root = temp_project();
        let before: Vec<_> = fs::read_dir(root.join(".vibehub"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        let result = read_project_settings(&root).unwrap();
        let after: Vec<_> = fs::read_dir(root.join(".vibehub"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(result.status, ProjectSettingsStatus::Missing);
        assert_eq!(result.settings, None);
        assert!(result.recommended_action.is_some());
        assert_eq!(before, after);
        assert!(!root.join(".vibehub/project-settings.yaml").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn creates_reads_and_updates_with_revisions_and_normalized_targets() {
        let root = temp_project();
        let mut create = request(0);
        create.agent_spec_targets.push(AgentSpecTarget::Codex);
        let created = update_project_settings(&root, create).unwrap();
        assert_eq!(created.revision, 1);
        assert_eq!(
            created.agent_spec_targets,
            vec![AgentSpecTarget::ClaudeCode, AgentSpecTarget::Codex]
        );
        assert_eq!(
            read_project_settings(&root).unwrap().settings.unwrap(),
            created
        );

        let mut update = request(1);
        update.output_language = OutputLanguage::EnUs;
        update.agent_spec_targets = vec![AgentSpecTarget::Opencode];
        let updated = update_project_settings(&root, update).unwrap();
        assert_eq!(updated.revision, 2);
        assert_eq!(updated.output_language, OutputLanguage::EnUs);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_invalid_enums_empty_targets_and_non_v3_projects() {
        let root = temp_project();
        let empty_error = update_project_settings(
            &root,
            V3ProjectSettingsUpdateRequest {
                expected_revision: 0,
                output_language: OutputLanguage::ZhTw,
                agent_spec_targets: vec![],
                repository_remote_url: None,
            },
        )
        .unwrap_err();
        assert_eq!(empty_error.code, "V3_PROJECT_SETTINGS_TARGETS_EMPTY");

        fs::write(
            root.join(".vibehub/project-settings.yaml"),
            "schema_version: 1\nkind: v3_project_settings\nrevision: 1\noutput_language: fr-FR\nagent_spec_targets: [codex]\nupdated_at: now\nupdated_by: test\n",
        )
        .unwrap();
        assert_eq!(
            read_project_settings(&root).unwrap_err().code,
            "V3_PROJECT_SETTINGS_INVALID_YAML"
        );
        fs::write(
            root.join(".vibehub/project-settings.yaml"),
            "schema_version: 1\nkind: v3_project_settings\nrevision: 1\noutput_language: en-US\nagent_spec_targets: [cursor]\nupdated_at: now\nupdated_by: test\n",
        )
        .unwrap();
        assert_eq!(
            read_project_settings(&root).unwrap_err().code,
            "V3_PROJECT_SETTINGS_INVALID_YAML"
        );

        let plain = std::env::temp_dir().join(format!("vibehub-not-v3-{}", Uuid::new_v4()));
        fs::create_dir(&plain).unwrap();
        assert_eq!(
            read_project_settings(&plain).unwrap_err().code,
            "V3_PROJECT_SETTINGS_REQUIRES_V3"
        );
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(plain).unwrap();
    }

    #[test]
    fn reports_revision_conflict_without_replacing_existing_file() {
        let root = temp_project();
        update_project_settings(&root, request(0)).unwrap();
        let path = root.join(".vibehub/project-settings.yaml");
        let before = fs::read(&path).unwrap();
        let error = update_project_settings(&root, request(0)).unwrap_err();
        assert_eq!(error.code, "V3_PROJECT_SETTINGS_REVISION_CONFLICT");
        assert_eq!(error.category, V3ErrorCategory::VersionConflict);
        assert_eq!(fs::read(path).unwrap(), before);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn corrupt_and_oversize_files_fail_closed() {
        let root = temp_project();
        let path = root.join(".vibehub/project-settings.yaml");
        fs::write(&path, b"not: [valid").unwrap();
        assert_eq!(
            read_project_settings(&root).unwrap_err().code,
            "V3_PROJECT_SETTINGS_INVALID_YAML"
        );
        fs::write(&path, [0xff, 0xfe]).unwrap();
        assert_eq!(
            read_project_settings(&root).unwrap_err().code,
            "V3_PROJECT_SETTINGS_INVALID_UTF8"
        );
        fs::write(&path, vec![b'x'; MAX_SETTINGS_BYTES as usize + 1]).unwrap();
        assert_eq!(
            read_project_settings(&root).unwrap_err().code,
            "V3_PROJECT_SETTINGS_FILE_TOO_LARGE"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn settings_file_and_vibehub_symlinks_fail_closed() {
        use std::os::unix::fs::symlink;
        let root = temp_project();
        let outside = root.join("outside.yaml");
        fs::write(&outside, "outside").unwrap();
        symlink(&outside, root.join(".vibehub/project-settings.yaml")).unwrap();
        assert_eq!(
            read_project_settings(&root).unwrap_err().code,
            "V3_PROJECT_SETTINGS_FILE_INVALID"
        );

        fs::remove_file(root.join(".vibehub/project-settings.yaml")).unwrap();
        fs::rename(root.join(".vibehub"), root.join("real-vibehub")).unwrap();
        symlink(root.join("real-vibehub"), root.join(".vibehub")).unwrap();
        assert_eq!(
            read_project_settings(&root).unwrap_err().code,
            "V3_ROOT_SYMLINK"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn remote_url_is_only_persisted_as_settings_metadata() {
        let root = temp_project();
        let git_config = root.join(".git/config");
        let git_before = fs::read(&git_config).ok();
        let current = root.join(".vibehub/tasks/current");
        let events = root.join(".vibehub/events");
        let current_before = fs::read(&current).ok();
        let events_before: Vec<_> = fs::read_dir(&events)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();

        let settings = update_project_settings(&root, request(0)).unwrap();
        assert_eq!(
            settings.repository_remote_url.as_deref(),
            Some("https://example.invalid/repository.git")
        );
        assert_eq!(fs::read(&git_config).ok(), git_before);
        assert_eq!(fs::read(&current).ok(), current_before);
        let events_after: Vec<_> = fs::read_dir(events)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(events_after, events_before);
        fs::remove_dir_all(root).unwrap();
    }
}

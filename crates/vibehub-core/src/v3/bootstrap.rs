use super::{V3Error, V3ErrorCategory};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const V3_SCHEMA_VERSION: u32 = 3;
const VIBEHUB_DIR: &str = ".vibehub";
const STAGING_DIR: &str = ".vibehub.v2-migration";
const V3_DIRS: &[&str] = &["events", "projections", "indexes", "runtime"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectLayoutState {
    Absent,
    V2,
    V3,
    MigrationInterrupted,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectLayoutStatus {
    pub state: ProjectLayoutState,
    pub schema_version: Option<u32>,
    pub message: String,
    pub recommended_action: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V3BootstrapResult {
    pub status: String,
    pub archived_legacy_v2: bool,
    pub created_paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProjectMarker {
    schema_version: u32,
    name: String,
}

pub fn inspect_project_layout(
    project_root: impl AsRef<Path>,
) -> Result<ProjectLayoutStatus, V3Error> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    let root = project_root.join(VIBEHUB_DIR);
    let staging = project_root.join(STAGING_DIR);

    if staging.exists() {
        return Ok(status(
            ProjectLayoutState::MigrationInterrupted,
            read_schema_version(&root),
            "a legacy-v2 migration staging directory is present",
            Some("run `vibehub v3 <project> migrate-recover` before retrying migration"),
        ));
    }
    if !root.exists() {
        return Ok(status(
            ProjectLayoutState::Absent,
            None,
            "project does not contain .vibehub",
            Some("run `vibehub v3 <project> init` to initialize v3"),
        ));
    }
    reject_symlink(&root, "V3_ROOT_SYMLINK")?;
    if !root.is_dir() {
        return Ok(status(
            ProjectLayoutState::Conflict,
            None,
            ".vibehub exists but is not a directory",
            None,
        ));
    }

    let schema_version = read_schema_version(&root);
    if schema_version == Some(V3_SCHEMA_VERSION) {
        return Ok(status(
            ProjectLayoutState::V3,
            schema_version,
            "project uses the v3 disk layout",
            None,
        ));
    }
    if root.join("legacy-v2").exists() {
        return Ok(status(
            ProjectLayoutState::Conflict,
            schema_version,
            "legacy-v2 exists without a valid v3 schema marker",
            Some("do not overwrite the archive; inspect the project and recover manually"),
        ));
    }
    Ok(status(
        ProjectLayoutState::V2,
        schema_version,
        "this is a v2 project",
        Some("run `vibehub v3 <project> migrate` to archive v2 and initialize v3"),
    ))
}

pub fn initialize_v3(project_root: impl AsRef<Path>) -> Result<V3BootstrapResult, V3Error> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    match inspect_project_layout(&project_root)?.state {
        ProjectLayoutState::V3 => Ok(V3BootstrapResult {
            status: "already_initialized".to_owned(),
            archived_legacy_v2: false,
            created_paths: Vec::new(),
        }),
        ProjectLayoutState::Absent => create_v3_root(&project_root),
        state => Err(layout_error(
            "V3_INIT_REQUIRES_EMPTY_PROJECT",
            format!("cannot initialize v3 from layout state {state:?}"),
        )),
    }
}

pub fn migrate_v2_to_v3(project_root: impl AsRef<Path>) -> Result<V3BootstrapResult, V3Error> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    let current = inspect_project_layout(&project_root)?;
    if current.state == ProjectLayoutState::V3 {
        return Ok(V3BootstrapResult {
            status: "already_migrated".to_owned(),
            archived_legacy_v2: project_root.join(VIBEHUB_DIR).join("legacy-v2").is_dir(),
            created_paths: Vec::new(),
        });
    }
    if current.state != ProjectLayoutState::V2 {
        return Err(layout_error(
            "V3_MIGRATION_STATE_INVALID",
            format!("cannot migrate layout state {:?}", current.state),
        ));
    }

    let root = project_root.join(VIBEHUB_DIR);
    let staging = project_root.join(STAGING_DIR);
    reject_symlink(&root, "V3_MIGRATION_ROOT_SYMLINK")?;
    fs::rename(&root, &staging).map_err(|error| io_error("V3_MIGRATION_STAGE_FAILED", error))?;

    let bootstrap = match create_v3_root(&project_root) {
        Ok(result) => result,
        Err(error) => {
            rollback_staged_root(&root, &staging)?;
            return Err(error);
        }
    };
    let archive = root.join("legacy-v2");
    if let Err(error) = fs::rename(&staging, &archive) {
        rollback_staged_root(&root, &staging)?;
        return Err(io_error("V3_MIGRATION_ARCHIVE_FAILED", error));
    }

    let mut created_paths = bootstrap.created_paths;
    created_paths.push(".vibehub/legacy-v2".to_owned());
    Ok(V3BootstrapResult {
        status: "migrated".to_owned(),
        archived_legacy_v2: true,
        created_paths,
    })
}

pub fn recover_interrupted_migration(
    project_root: impl AsRef<Path>,
) -> Result<V3BootstrapResult, V3Error> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    let root = project_root.join(VIBEHUB_DIR);
    let staging = project_root.join(STAGING_DIR);
    if !staging.exists() {
        return Err(layout_error(
            "V3_MIGRATION_NOT_INTERRUPTED",
            "migration staging directory is absent",
        ));
    }
    reject_symlink(&staging, "V3_MIGRATION_STAGE_SYMLINK")?;
    if root.exists() && read_schema_version(&root) == Some(V3_SCHEMA_VERSION) {
        let archive = root.join("legacy-v2");
        if archive.exists() {
            return Err(layout_error(
                "V3_MIGRATION_ARCHIVE_CONFLICT",
                "legacy-v2 already exists; refusing to overwrite it",
            ));
        }
        fs::rename(&staging, &archive)
            .map_err(|error| io_error("V3_MIGRATION_RECOVER_FAILED", error))?;
        return Ok(V3BootstrapResult {
            status: "recovered_forward".to_owned(),
            archived_legacy_v2: true,
            created_paths: vec![".vibehub/legacy-v2".to_owned()],
        });
    }
    if root.exists() {
        return Err(layout_error(
            "V3_MIGRATION_RECOVERY_CONFLICT",
            ".vibehub exists without a valid v3 marker; refusing automatic recovery",
        ));
    }
    fs::rename(&staging, &root).map_err(|error| io_error("V3_MIGRATION_RECOVER_FAILED", error))?;
    Ok(V3BootstrapResult {
        status: "rolled_back_to_v2".to_owned(),
        archived_legacy_v2: false,
        created_paths: Vec::new(),
    })
}

fn create_v3_root(project_root: &Path) -> Result<V3BootstrapResult, V3Error> {
    let root = project_root.join(VIBEHUB_DIR);
    fs::create_dir(&root).map_err(|error| io_error("V3_INIT_CREATE_FAILED", error))?;
    let result = (|| {
        for directory in V3_DIRS {
            fs::create_dir(root.join(directory))
                .map_err(|error| io_error("V3_INIT_CREATE_FAILED", error))?;
        }
        let marker = ProjectMarker {
            schema_version: V3_SCHEMA_VERSION,
            name: project_root
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("project")
                .to_owned(),
        };
        let content = serde_yaml::to_string(&marker).map_err(|error| {
            layout_error(
                "V3_INIT_MARKER_FAILED",
                format!("failed to encode marker: {error}"),
            )
        })?;
        fs::write(root.join("project.yaml"), content)
            .map_err(|error| io_error("V3_INIT_MARKER_FAILED", error))?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_dir_all(&root);
        return Err(error);
    }

    let mut created_paths = vec![".vibehub/project.yaml".to_owned()];
    created_paths.extend(V3_DIRS.iter().map(|path| format!(".vibehub/{path}")));
    Ok(V3BootstrapResult {
        status: "initialized".to_owned(),
        archived_legacy_v2: false,
        created_paths,
    })
}

fn rollback_staged_root(root: &Path, staging: &Path) -> Result<(), V3Error> {
    if root.exists() {
        fs::remove_dir_all(root)
            .map_err(|error| io_error("V3_MIGRATION_ROLLBACK_FAILED", error))?;
    }
    fs::rename(staging, root).map_err(|error| io_error("V3_MIGRATION_ROLLBACK_FAILED", error))
}

fn read_schema_version(root: &Path) -> Option<u32> {
    let marker = root.join("project.yaml");
    let metadata = fs::symlink_metadata(&marker).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > 64 * 1024 {
        return None;
    }
    let content = fs::read_to_string(marker).ok()?;
    serde_yaml::from_str::<serde_yaml::Value>(&content)
        .ok()?
        .get("schema_version")?
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
}

fn canonical_project_root(project_root: &Path) -> Result<PathBuf, V3Error> {
    project_root
        .canonicalize()
        .map_err(|error| io_error("V3_PROJECT_ROOT_NOT_FOUND", error))
}

fn reject_symlink(path: &Path, code: &'static str) -> Result<(), V3Error> {
    let metadata = fs::symlink_metadata(path).map_err(|error| io_error(code, error))?;
    if metadata.file_type().is_symlink() {
        return Err(layout_error(code, "symbolic links are not allowed"));
    }
    Ok(())
}

fn status(
    state: ProjectLayoutState,
    schema_version: Option<u32>,
    message: &str,
    recommended_action: Option<&str>,
) -> ProjectLayoutStatus {
    ProjectLayoutStatus {
        state,
        schema_version,
        message: message.to_owned(),
        recommended_action: recommended_action.map(str::to_owned),
    }
}

fn layout_error(code: &str, message: impl Into<String>) -> V3Error {
    V3Error::new(code, V3ErrorCategory::Validation, false, message)
}

fn io_error(code: &'static str, error: std::io::Error) -> V3Error {
    V3Error::new(code, V3ErrorCategory::Internal, false, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_project() -> PathBuf {
        let root = std::env::temp_dir().join(format!("vibehub-v3-bootstrap-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        root
    }

    #[test]
    fn initializes_absent_project_and_is_idempotent() {
        let project = temp_project();
        let first = initialize_v3(&project).unwrap();
        assert_eq!(first.status, "initialized");
        assert_eq!(
            inspect_project_layout(&project).unwrap().state,
            ProjectLayoutState::V3
        );
        let second = initialize_v3(&project).unwrap();
        assert_eq!(second.status, "already_initialized");
        for directory in V3_DIRS {
            assert!(project.join(VIBEHUB_DIR).join(directory).is_dir());
        }
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn migrates_v2_root_without_converting_legacy_files() {
        let project = temp_project();
        let root = project.join(VIBEHUB_DIR);
        fs::create_dir(&root).unwrap();
        fs::write(root.join("project.yaml"), "schema_version: 1\nname: old\n").unwrap();
        fs::write(root.join("history.txt"), "unchanged").unwrap();

        let result = migrate_v2_to_v3(&project).unwrap();
        assert_eq!(result.status, "migrated");
        assert_eq!(
            inspect_project_layout(&project).unwrap().state,
            ProjectLayoutState::V3
        );
        assert_eq!(
            fs::read_to_string(project.join(".vibehub/legacy-v2/history.txt")).unwrap(),
            "unchanged"
        );
        assert_eq!(
            migrate_v2_to_v3(&project).unwrap().status,
            "already_migrated"
        );
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn refuses_archive_conflict_and_reports_v2_upgrade() {
        let project = temp_project();
        let root = project.join(VIBEHUB_DIR);
        fs::create_dir_all(root.join("legacy-v2")).unwrap();
        let status = inspect_project_layout(&project).unwrap();
        assert_eq!(status.state, ProjectLayoutState::Conflict);
        assert!(migrate_v2_to_v3(&project).is_err());
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn recovers_interrupted_migration_forward_or_backward() {
        let project = temp_project();
        let staging = project.join(STAGING_DIR);
        fs::create_dir(&staging).unwrap();
        fs::write(staging.join("old.txt"), "legacy").unwrap();
        assert_eq!(
            recover_interrupted_migration(&project).unwrap().status,
            "rolled_back_to_v2"
        );

        fs::rename(project.join(VIBEHUB_DIR), &staging).unwrap();
        create_v3_root(&project).unwrap();
        assert_eq!(
            recover_interrupted_migration(&project).unwrap().status,
            "recovered_forward"
        );
        assert!(project.join(".vibehub/legacy-v2/old.txt").is_file());
        fs::remove_dir_all(project).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn refuses_symlink_vibehub_root() {
        use std::os::unix::fs::symlink;
        let project = temp_project();
        let outside = temp_project();
        symlink(&outside, project.join(VIBEHUB_DIR)).unwrap();
        let error = inspect_project_layout(&project).unwrap_err();
        assert_eq!(error.code, "V3_ROOT_SYMLINK");
        fs::remove_file(project.join(VIBEHUB_DIR)).unwrap();
        fs::remove_dir_all(project).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }
}

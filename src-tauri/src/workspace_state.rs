use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::Duration;
use uuid::Uuid;

const SCHEMA_VERSION: u32 = 3;
const TRANSITIONAL_CONTRACT_SCHEMA_VERSION: u32 = 1;
const KIND: &str = "workspace_state";
const FILE_NAME: &str = "workspace-state.json";
const BACKUP_FILE_NAME: &str = "workspace-state.json.bak";
const LEGACY_BACKUP_FILE_NAME: &str = "workspace-state.legacy-v2.json.bak";
const TRANSITIONAL_BACKUP_FILE_NAME: &str = "workspace-state.contract-v1.json.bak";
const LOCK_FILE_NAME: &str = "workspace-state.lock";
const LEGACY_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceProjectUiContext {
    pub owner: String,
    pub current_view: String,
    pub selected_task_id: Option<String>,
    pub selected_node_id: Option<String>,
}

impl Default for WorkspaceProjectUiContext {
    fn default() -> Self {
        Self {
            owner: "v3-cockpit".to_string(),
            current_view: "project-overview".to_string(),
            selected_task_id: None,
            selected_node_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceProjectIdentity {
    pub project_id: String,
    pub canonical_path: String,
    pub display_name: String,
    pub ui_context: WorkspaceProjectUiContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceStateProvenance {
    pub source: String,
    pub writer: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceState {
    pub schema_version: u32,
    pub kind: String,
    pub revision: u64,
    pub open_projects: Vec<WorkspaceProjectIdentity>,
    pub active_project_id: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
    pub provenance: WorkspaceStateProvenance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceStateDiagnostic {
    pub code: String,
    pub severity: String,
    pub project_id: Option<String>,
    pub message: String,
    pub recovery_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceStateReadResult {
    pub status: String,
    pub state: Option<WorkspaceState>,
    pub diagnostics: Vec<WorkspaceStateDiagnostic>,
    pub recommended_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceStateSaveRequest {
    pub expected_revision: u64,
    pub state: WorkspaceState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceStateSaveResult {
    pub state: WorkspaceState,
    pub previous_revision: u64,
    pub backup_path: Option<String>,
    pub diagnostics: Vec<WorkspaceStateDiagnostic>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct LegacyWorkspaceTabV2 {
    project_id: String,
    project_path: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct LegacyWorkspaceStateV2 {
    schema_version: u32,
    tabs: Vec<LegacyWorkspaceTabV2>,
    active_project_id: Option<String>,
    restore_tabs_on_launch: bool,
    updated_at: DateTime<Utc>,
}

enum DecodedWorkspaceState {
    Current(WorkspaceState),
    TransitionalV1(WorkspaceState),
    LegacyV2(WorkspaceState),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkspaceStateSource {
    Current,
    TransitionalV1,
    LegacyV2,
}

pub struct WorkspaceStateStore {
    path: PathBuf,
}

struct WorkspaceStateLock {
    path: PathBuf,
    _file: fs::File,
}

impl Drop for WorkspaceStateLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

impl WorkspaceStateStore {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            path: data_dir.join(FILE_NAME),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn read(&self, known_projects: &[KnownProject]) -> Result<WorkspaceStateReadResult> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(WorkspaceStateReadResult {
                    status: "missing".to_string(),
                    state: None,
                    diagnostics: vec![],
                    recommended_action: Some("打开项目后会创建工作区状态".to_string()),
                });
            }
            Err(error) => return Err(error).context("读取 WorkspaceState 失败"),
        };

        match decode_workspace_state(&bytes, known_projects) {
            Ok(decoded) => match decoded {
                DecodedWorkspaceState::Current(state) => {
                    match self.validate_and_filter(state, known_projects) {
                        Ok(result) => Ok(result),
                        Err(error) => Ok(invalid_result(
                            &error.to_string(),
                            "workspace_state.invalid",
                        )),
                    }
                }
                DecodedWorkspaceState::TransitionalV1(state) => {
                    let mut result = match self.validate_and_filter(state, known_projects) {
                        Ok(result) => result,
                        Err(error) => invalid_result(&error.to_string(), "workspace_state.invalid"),
                    };
                    if result.state.is_some() {
                        if result.status == "present" {
                            result.status = "recovered".to_string();
                        }
                        result.diagnostics.insert(
                            0,
                            WorkspaceStateDiagnostic {
                                code: "workspace_state.migrated_from_transitional_v1".to_string(),
                                severity: "info".to_string(),
                                project_id: None,
                                message: "已读取过渡版本的工作区状态，将迁移到正式 schema v3。"
                                    .to_string(),
                                recovery_action: "无需操作；现有项目标签和上下文会保留。"
                                    .to_string(),
                            },
                        );
                    }
                    Ok(result)
                }
                DecodedWorkspaceState::LegacyV2(state) => {
                    let mut result = match self.validate_and_filter(state, known_projects) {
                        Ok(result) => result,
                        Err(error) => invalid_result(&error.to_string(), "workspace_state.invalid"),
                    };
                    if result.state.is_some() {
                        if result.status == "present" {
                            result.status = "recovered".to_string();
                        }
                        result.diagnostics.insert(
                            0,
                            WorkspaceStateDiagnostic {
                                code: "workspace_state.migrated_from_legacy_v2".to_string(),
                                severity: "info".to_string(),
                                project_id: None,
                                message: "已读取更新前版本的工作区状态，将在下一次保存时安全迁移。"
                                    .to_string(),
                                recovery_action: "无需操作；旧文件原文会保存在备份中。".to_string(),
                            },
                        );
                    }
                    Ok(result)
                }
            },
            Err(primary_error) => match fs::read(self.path.with_file_name(BACKUP_FILE_NAME)) {
                Ok(backup) => {
                    let state = decode_workspace_state(&backup, known_projects)
                        .with_context(|| format!("WorkspaceState 与备份均损坏: {primary_error}"))?;
                    let state = match state {
                        DecodedWorkspaceState::Current(state)
                        | DecodedWorkspaceState::TransitionalV1(state)
                        | DecodedWorkspaceState::LegacyV2(state) => state,
                    };
                    let mut result = match self.validate_and_filter(state, known_projects) {
                        Ok(result) => result,
                        Err(error) => invalid_result(&error.to_string(), "workspace_state.invalid"),
                    };
                    result.status = "recovered".to_string();
                    result.diagnostics.insert(
                        0,
                        diagnostic(
                            "workspace_state.recovered_from_backup",
                            "主状态文件损坏，已从备份恢复。",
                            "保留备份并在下一次保存时重新生成主文件。",
                        ),
                    );
                    Ok(result)
                }
                Err(_) => Ok(WorkspaceStateReadResult {
                    status: "invalid".to_string(),
                    state: None,
                    diagnostics: vec![diagnostic(
                        "workspace_state.invalid",
                        &format!("工作区状态无法解析: {primary_error}"),
                        "删除损坏状态后重新打开项目。",
                    )],
                    recommended_action: Some("重新打开项目以创建新的工作区状态".to_string()),
                }),
            },
        }
    }

    pub fn save(
        &self,
        request: WorkspaceStateSaveRequest,
        known_projects: &[KnownProject],
    ) -> Result<WorkspaceStateSaveResult> {
        let _lock = self.acquire_lock()?;
        let mut requested_state = request.state;
        canonicalize_state_paths(&mut requested_state);
        validate_state(&requested_state)?;
        validate_known_projects(&requested_state, known_projects)?;
        let (current, current_bytes, current_source) =
            self.read_current_for_write(known_projects)?;
        let current_revision = current.as_ref().map(|state| state.revision).unwrap_or(0);
        if current_revision != request.expected_revision {
            anyhow::bail!(
                "WORKSPACE_STATE_REVISION_CONFLICT: expected {}, current {}",
                request.expected_revision,
                current_revision
            );
        }

        let mut next = requested_state;
        next.revision = current_revision.saturating_add(1);
        next.updated_at = Utc::now();
        let bytes = serde_json::to_vec_pretty(&next).context("序列化 WorkspaceState 失败")?;
        let parent = self.path.parent().context("WorkspaceState 缺少父目录")?;
        fs::create_dir_all(parent).context("创建 WorkspaceState 目录失败")?;
        let temporary = parent.join(format!(".workspace-state.{}.tmp", Uuid::new_v4()));
        let original_mode = current.as_ref().and_then(|_| file_mode(&self.path));

        fs::write(&temporary, &bytes).context("写入 WorkspaceState 临时文件失败")?;
        if let Some(mode) = original_mode {
            set_file_mode(&temporary, mode).context("保留 WorkspaceState 文件权限失败")?;
        }
        sync_file(&temporary).context("同步 WorkspaceState 临时文件失败")?;
        if let Some(current_bytes) = current_bytes.as_deref() {
            match current_source {
                WorkspaceStateSource::LegacyV2 => {
                    preserve_versioned_backup(parent, LEGACY_BACKUP_FILE_NAME, current_bytes)
                        .context("保存更新前 WorkspaceState 原始备份失败")?
                }
                WorkspaceStateSource::TransitionalV1 => {
                    preserve_versioned_backup(parent, TRANSITIONAL_BACKUP_FILE_NAME, current_bytes)
                        .context("保存过渡版 WorkspaceState 原始备份失败")?
                }
                WorkspaceStateSource::Current => {}
            }
        }
        rescue_versioned_rolling_backup(parent).context("抢救 WorkspaceState 滚动备份失败")?;
        if let Some(current_bytes) = current_bytes {
            let backup = self.path.with_file_name(BACKUP_FILE_NAME);
            let backup_tmp = parent.join(format!(".workspace-state-backup.{}.tmp", Uuid::new_v4()));
            fs::write(&backup_tmp, current_bytes).context("写入 WorkspaceState 备份失败")?;
            sync_file(&backup_tmp).context("同步 WorkspaceState 备份失败")?;
            replace_file(&backup_tmp, &backup).context("替换 WorkspaceState 备份失败")?;
        }
        if let Err(error) = replace_file(&temporary, &self.path) {
            let _ = fs::remove_file(&temporary);
            return Err(error).context("原子替换 WorkspaceState 失败");
        }
        sync_directory(parent).context("同步 WorkspaceState 目录失败")?;

        Ok(WorkspaceStateSaveResult {
            state: next,
            previous_revision: current_revision,
            backup_path: current.map(|_| {
                self.path
                    .with_file_name(BACKUP_FILE_NAME)
                    .display()
                    .to_string()
            }),
            diagnostics: vec![],
        })
    }

    fn acquire_lock(&self) -> Result<WorkspaceStateLock> {
        let parent = self.path.parent().context("WorkspaceState 缺少父目录")?;
        fs::create_dir_all(parent).context("创建 WorkspaceState 目录失败")?;
        let lock_path = parent.join(LOCK_FILE_NAME);
        for _ in 0..50 {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(file) => {
                    return Ok(WorkspaceStateLock {
                        path: lock_path,
                        _file: file,
                    })
                }
                Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => return Err(error).context("创建 WorkspaceState 写入锁失败"),
            }
        }
        anyhow::bail!("WORKSPACE_STATE_WRITE_BUSY: another writer holds the workspace-state lock")
    }

    fn read_current_for_write(
        &self,
        known_projects: &[KnownProject],
    ) -> Result<(
        Option<WorkspaceState>,
        Option<Vec<u8>>,
        WorkspaceStateSource,
    )> {
        match fs::read(&self.path) {
            Ok(bytes) => match decode_workspace_state(&bytes, known_projects) {
                Ok(decoded) => {
                    let (state, source) = match decoded {
                        DecodedWorkspaceState::Current(state) => {
                            (state, WorkspaceStateSource::Current)
                        }
                        DecodedWorkspaceState::TransitionalV1(state) => {
                            (state, WorkspaceStateSource::TransitionalV1)
                        }
                        DecodedWorkspaceState::LegacyV2(state) => {
                            (state, WorkspaceStateSource::LegacyV2)
                        }
                    };
                    validate_state(&state)?;
                    Ok((Some(state), Some(bytes), source))
                }
                Err(primary_error) => {
                    let backup_path = self.path.with_file_name(BACKUP_FILE_NAME);
                    let backup = fs::read(&backup_path).with_context(|| {
                        format!("WorkspaceState 主文件损坏且备份不可用: {primary_error}")
                    })?;
                    let state = decode_workspace_state(&backup, known_projects)
                        .with_context(|| format!("WorkspaceState 备份损坏: {primary_error}"))?;
                    let state = match state {
                        DecodedWorkspaceState::Current(state)
                        | DecodedWorkspaceState::TransitionalV1(state)
                        | DecodedWorkspaceState::LegacyV2(state) => state,
                    };
                    validate_state(&state)?;
                    Ok((Some(state), Some(backup), WorkspaceStateSource::Current))
                }
            },
            Err(error) if error.kind() == ErrorKind::NotFound => {
                Ok((None, None, WorkspaceStateSource::Current))
            }
            Err(error) => Err(error).context("读取当前 WorkspaceState 失败"),
        }
    }

    fn validate_and_filter(
        &self,
        state: WorkspaceState,
        known_projects: &[KnownProject],
    ) -> Result<WorkspaceStateReadResult> {
        validate_state(&state)?;
        let mut diagnostics = vec![];
        let mut seen_ids = HashSet::new();
        let mut seen_paths = HashSet::new();
        let mut open_projects = Vec::with_capacity(state.open_projects.len());
        for project in &state.open_projects {
            let path_key = canonical_path_key(&project.canonical_path);
            if !seen_ids.insert(project.project_id.clone()) {
                diagnostics.push(diagnostic_for_project(
                    "workspace_state.duplicate_project_id",
                    &project,
                    "重复项目身份已剔除。",
                    "从项目列表重新打开唯一项目。",
                ));
                continue;
            }
            if !seen_paths.insert(path_key.clone()) {
                diagnostics.push(diagnostic_for_project(
                    "workspace_state.duplicate_canonical_path",
                    &project,
                    "重复或路径别名项目已剔除。",
                    "从项目列表重新打开正确路径。",
                ));
                continue;
            }
            let Some(known) = known_projects
                .iter()
                .find(|item| item.id == project.project_id)
            else {
                diagnostics.push(diagnostic_for_project(
                    "workspace_state.project_missing",
                    &project,
                    "项目已不存在或不可访问。",
                    "从项目列表重新打开项目。",
                ));
                continue;
            };
            if !project_path_is_accessible(&known.canonical_path) {
                diagnostics.push(diagnostic_for_project(
                    "workspace_state.project_inaccessible",
                    &project,
                    "项目路径不存在或不可访问。",
                    "检查项目路径权限，或从项目列表重新打开项目。",
                ));
                continue;
            }
            if canonical_path_key(&known.canonical_path) != path_key {
                diagnostics.push(diagnostic_for_project(
                    "workspace_state.project_identity_changed",
                    &project,
                    "项目身份或 canonical path 已变化。",
                    "从项目列表重新打开项目。",
                ));
                continue;
            }
            open_projects.push(project.clone());
        }
        let had_active_project = state.active_project_id.is_some();
        let active_project_id = state.active_project_id.clone().filter(|id| {
            open_projects
                .iter()
                .any(|project| &project.project_id == id)
        });
        if had_active_project && active_project_id.is_none() {
            diagnostics.push(diagnostic(
                "workspace_state.active_project_removed",
                "active project 不再属于有效标签集合。",
                "选择一个仍可访问的项目。",
            ));
        }
        let mut filtered = state;
        filtered.open_projects = open_projects;
        filtered.active_project_id = active_project_id;
        Ok(WorkspaceStateReadResult {
            status: if diagnostics.is_empty() {
                "present"
            } else {
                "partial"
            }
            .to_string(),
            state: Some(filtered),
            recommended_action: if diagnostics.is_empty() {
                None
            } else {
                Some("检查提示并重新打开失效项目".to_string())
            },
            diagnostics,
        })
    }
}

#[derive(Debug, Clone)]
pub struct KnownProject {
    pub id: String,
    pub display_name: String,
    pub canonical_path: String,
}

fn decode_workspace_state(
    bytes: &[u8],
    known_projects: &[KnownProject],
) -> Result<DecodedWorkspaceState> {
    match serde_json::from_slice::<WorkspaceState>(bytes) {
        Ok(state) if state.schema_version == SCHEMA_VERSION => {
            return Ok(DecodedWorkspaceState::Current(state));
        }
        Ok(mut state) if state.schema_version == TRANSITIONAL_CONTRACT_SCHEMA_VERSION => {
            state.schema_version = SCHEMA_VERSION;
            state.provenance = WorkspaceStateProvenance {
                source: "migration".to_string(),
                writer: "workspace-state-store".to_string(),
                reason: "upgrade-from-transitional-contract-v1".to_string(),
            };
            return Ok(DecodedWorkspaceState::TransitionalV1(state));
        }
        Ok(state) => anyhow::bail!(
            "WORKSPACE_STATE_UNSUPPORTED_SCHEMA: schema_version={}, kind={}",
            state.schema_version,
            state.kind
        ),
        Err(current_error) => {
            let legacy = serde_json::from_slice::<LegacyWorkspaceStateV2>(bytes).map_err(
                |legacy_error| {
                    anyhow::anyhow!(
                        "当前格式解析失败: {current_error}; 更新前版本格式解析失败: {legacy_error}"
                    )
                },
            )?;
            if legacy.schema_version != LEGACY_SCHEMA_VERSION {
                anyhow::bail!(
                    "WORKSPACE_STATE_UNSUPPORTED_LEGACY_SCHEMA: schema_version={}",
                    legacy.schema_version
                );
            }
            let open_projects = if legacy.restore_tabs_on_launch {
                legacy
                    .tabs
                    .into_iter()
                    .map(|tab| {
                        let known = known_projects.iter().find(|item| item.id == tab.project_id);
                        WorkspaceProjectIdentity {
                            display_name: known
                                .map(|item| item.display_name.clone())
                                .unwrap_or_else(|| tab.project_id.clone()),
                            project_id: tab.project_id,
                            canonical_path: tab.project_path,
                            ui_context: WorkspaceProjectUiContext::default(),
                        }
                    })
                    .collect()
            } else {
                vec![]
            };
            let active_project_id = if legacy.restore_tabs_on_launch {
                legacy.active_project_id
            } else {
                None
            };
            Ok(DecodedWorkspaceState::LegacyV2(WorkspaceState {
                schema_version: SCHEMA_VERSION,
                kind: KIND.to_string(),
                revision: 0,
                open_projects,
                active_project_id,
                updated_at: legacy.updated_at,
                updated_by: "vibehub-legacy-v2-migration".to_string(),
                provenance: WorkspaceStateProvenance {
                    source: "migration".to_string(),
                    writer: "workspace-state-store".to_string(),
                    reason: "upgrade-from-pre-contract-workspace-state-v2".to_string(),
                },
            }))
        }
    }
}

fn preserve_versioned_backup(parent: &Path, file_name: &str, bytes: &[u8]) -> Result<()> {
    let backup = parent.join(file_name);
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&backup)
    {
        Ok(mut file) => {
            use std::io::Write;
            file.write_all(bytes)?;
            file.sync_all()?;
            sync_directory(parent)?;
            Ok(())
        }
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            let existing = fs::read(&backup)?;
            if existing == bytes {
                Ok(())
            } else {
                anyhow::bail!(
                    "WORKSPACE_STATE_LEGACY_BACKUP_CONFLICT: existing migration backup differs"
                )
            }
        }
        Err(error) => Err(error.into()),
    }
}

fn rescue_versioned_rolling_backup(parent: &Path) -> Result<()> {
    let rolling_backup = parent.join(BACKUP_FILE_NAME);
    let bytes = match fs::read(&rolling_backup) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if matches!(
        serde_json::from_slice::<LegacyWorkspaceStateV2>(&bytes),
        Ok(legacy) if legacy.schema_version == LEGACY_SCHEMA_VERSION
    ) {
        return preserve_versioned_backup(parent, LEGACY_BACKUP_FILE_NAME, &bytes);
    }
    if matches!(
        serde_json::from_slice::<WorkspaceState>(&bytes),
        Ok(state) if state.schema_version == TRANSITIONAL_CONTRACT_SCHEMA_VERSION
    ) {
        return preserve_versioned_backup(parent, TRANSITIONAL_BACKUP_FILE_NAME, &bytes);
    }
    Ok(())
}

fn validate_state(state: &WorkspaceState) -> Result<()> {
    if state.schema_version != SCHEMA_VERSION || state.kind != KIND {
        anyhow::bail!(
            "WORKSPACE_STATE_UNSUPPORTED_SCHEMA: schema_version={}, kind={}",
            state.schema_version,
            state.kind
        );
    }
    if state.open_projects.iter().any(|project| {
        project.project_id.trim().is_empty() || project.canonical_path.trim().is_empty()
    }) {
        anyhow::bail!(
            "WORKSPACE_STATE_INVALID_PROJECT_IDENTITY: project_id and canonical_path are required"
        );
    }
    if let Some(active) = &state.active_project_id {
        if !state
            .open_projects
            .iter()
            .any(|project| &project.project_id == active)
        {
            anyhow::bail!("WORKSPACE_STATE_INVALID_ACTIVE_PROJECT: active project is not open");
        }
    }
    Ok(())
}

fn validate_known_projects(state: &WorkspaceState, known_projects: &[KnownProject]) -> Result<()> {
    let mut seen_ids = HashSet::new();
    let mut seen_paths = HashSet::new();
    for project in &state.open_projects {
        if !seen_ids.insert(project.project_id.clone()) {
            anyhow::bail!(
                "WORKSPACE_STATE_DUPLICATE_PROJECT_ID: {}",
                project.project_id
            );
        }
        let path = canonical_path_key(&project.canonical_path);
        if !seen_paths.insert(path.clone()) {
            anyhow::bail!(
                "WORKSPACE_STATE_DUPLICATE_CANONICAL_PATH: {}",
                project.canonical_path
            );
        }
        let Some(known) = known_projects
            .iter()
            .find(|known| known.id == project.project_id)
        else {
            anyhow::bail!(
                "WORKSPACE_STATE_PROJECT_UNAVAILABLE: {}",
                project.project_id
            );
        };
        if !project_path_is_accessible(&known.canonical_path) {
            anyhow::bail!(
                "WORKSPACE_STATE_PROJECT_INACCESSIBLE: {}",
                project.project_id
            );
        }
        if canonical_path_key(&known.canonical_path) != path {
            anyhow::bail!(
                "WORKSPACE_STATE_PATH_IDENTITY_MISMATCH: {}",
                project.project_id
            );
        }
    }
    Ok(())
}

fn canonicalize_state_paths(state: &mut WorkspaceState) {
    for project in &mut state.open_projects {
        if let Ok(path) = fs::canonicalize(&project.canonical_path) {
            project.canonical_path = path.to_string_lossy().into_owned();
        }
    }
}

fn project_path_is_accessible(path: &str) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.is_dir())
        .unwrap_or(false)
        && fs::read_dir(path).is_ok()
}

fn normalize_path(path: &str) -> String {
    let replaced = path.replace('\\', "/");
    #[cfg(windows)]
    {
        replaced
            .to_ascii_lowercase()
            .trim_end_matches('/')
            .to_string()
    }
    #[cfg(not(windows))]
    {
        replaced.trim_end_matches('/').to_string()
    }
}

fn canonical_path_key(path: &str) -> String {
    let canonical = fs::canonicalize(path)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string());
    normalize_path(&canonical)
}

fn diagnostic(code: &str, message: &str, recovery_action: &str) -> WorkspaceStateDiagnostic {
    WorkspaceStateDiagnostic {
        code: code.to_string(),
        severity: "warning".to_string(),
        project_id: None,
        message: message.to_string(),
        recovery_action: recovery_action.to_string(),
    }
}

fn diagnostic_for_project(
    code: &str,
    project: &WorkspaceProjectIdentity,
    message: &str,
    recovery_action: &str,
) -> WorkspaceStateDiagnostic {
    WorkspaceStateDiagnostic {
        project_id: Some(project.project_id.clone()),
        ..diagnostic(code, message, recovery_action)
    }
}

fn invalid_result(message: &str, code: &str) -> WorkspaceStateReadResult {
    WorkspaceStateReadResult {
        status: "invalid".to_string(),
        state: None,
        diagnostics: vec![WorkspaceStateDiagnostic {
            code: code.to_string(),
            severity: "error".to_string(),
            project_id: None,
            message: message.to_string(),
            recovery_action: "删除损坏状态后重新打开项目。".to_string(),
        }],
        recommended_action: Some("重新打开项目以创建新的工作区状态".to_string()),
    }
}

#[cfg(unix)]
fn file_mode(path: &Path) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path)
        .ok()
        .map(|meta| meta.permissions().mode())
}
#[cfg(not(unix))]
fn file_mode(_path: &Path) -> Option<u32> {
    None
}
#[cfg(unix)]
fn set_file_mode(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(Into::into)
}
#[cfg(not(unix))]
fn set_file_mode(_path: &Path, _mode: u32) -> Result<()> {
    Ok(())
}
fn sync_file(path: &Path) -> Result<()> {
    fs::File::open(path)?.sync_all().map_err(Into::into)
}
#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<()> {
    fs::File::open(path)?.sync_all().map_err(Into::into)
}
#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir() -> PathBuf {
        let path = std::env::temp_dir().join(format!("vibehub-workspace-state-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn state(revision: u64) -> WorkspaceState {
        state_with_path(revision, Path::new("/tmp/a"))
    }

    fn state_with_path(revision: u64, path: &Path) -> WorkspaceState {
        WorkspaceState {
            schema_version: SCHEMA_VERSION,
            kind: KIND.to_string(),
            revision,
            open_projects: vec![WorkspaceProjectIdentity {
                project_id: "project-a".to_string(),
                canonical_path: path.display().to_string(),
                display_name: "A".to_string(),
                ui_context: Default::default(),
            }],
            active_project_id: Some("project-a".to_string()),
            updated_at: Utc::now(),
            updated_by: "test".to_string(),
            provenance: WorkspaceStateProvenance {
                source: "user".to_string(),
                writer: "test".to_string(),
                reason: "test".to_string(),
            },
        }
    }

    fn legacy_v2_bytes(projects: &[(&str, &Path)], active_project_id: Option<&str>) -> Vec<u8> {
        let tabs = projects
            .iter()
            .map(|(project_id, path)| {
                serde_json::json!({
                    "project_id": project_id,
                    "project_path": path.display().to_string(),
                })
            })
            .collect::<Vec<_>>();
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": 2,
            "tabs": tabs,
            "active_project_id": active_project_id,
            "restore_tabs_on_launch": true,
            "updated_at": "2026-07-31T10:43:12.507643Z",
        }))
        .unwrap()
    }

    #[test]
    fn legacy_v2_is_read_as_migration_and_preserves_order_and_active_project() {
        let dir = tempdir();
        let project_a = dir.join("project-a");
        let project_b = dir.join("project-b");
        fs::create_dir_all(&project_a).unwrap();
        fs::create_dir_all(&project_b).unwrap();
        let store = WorkspaceStateStore::new(dir.clone());
        let legacy = legacy_v2_bytes(
            &[("project-b", &project_b), ("project-a", &project_a)],
            Some("project-a"),
        );
        fs::write(store.path(), &legacy).unwrap();
        let known = [
            KnownProject {
                id: "project-a".to_string(),
                display_name: "A".to_string(),
                canonical_path: project_a.display().to_string(),
            },
            KnownProject {
                id: "project-b".to_string(),
                display_name: "B".to_string(),
                canonical_path: project_b.display().to_string(),
            },
        ];

        let result = store.read(&known).unwrap();
        assert_eq!(result.status, "recovered");
        let state = result.state.unwrap();
        assert_eq!(state.revision, 0);
        assert_eq!(
            state
                .open_projects
                .iter()
                .map(|project| project.project_id.as_str())
                .collect::<Vec<_>>(),
            vec!["project-b", "project-a"]
        );
        assert_eq!(state.open_projects[0].display_name, "B");
        assert_eq!(state.active_project_id.as_deref(), Some("project-a"));
        assert!(result
            .diagnostics
            .iter()
            .any(|item| item.code == "workspace_state.migrated_from_legacy_v2"));
        assert_eq!(fs::read(store.path()).unwrap(), legacy);
        assert!(!dir.join(BACKUP_FILE_NAME).exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn legacy_v2_restore_disabled_respects_previous_user_setting() {
        let dir = tempdir();
        let project_dir = dir.join("project-a");
        fs::create_dir_all(&project_dir).unwrap();
        let store = WorkspaceStateStore::new(dir.clone());
        let mut value: serde_json::Value = serde_json::from_slice(&legacy_v2_bytes(
            &[("project-a", &project_dir)],
            Some("project-a"),
        ))
        .unwrap();
        value["restore_tabs_on_launch"] = serde_json::Value::Bool(false);
        fs::write(store.path(), serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        let known = [KnownProject {
            id: "project-a".to_string(),
            display_name: "A".to_string(),
            canonical_path: project_dir.display().to_string(),
        }];

        let state = store.read(&known).unwrap().state.unwrap();
        assert!(state.open_projects.is_empty());
        assert_eq!(state.active_project_id, None);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn first_save_after_legacy_v2_keeps_exact_old_bytes_in_backup() {
        let dir = tempdir();
        let project_dir = dir.join("project-a");
        fs::create_dir_all(&project_dir).unwrap();
        let store = WorkspaceStateStore::new(dir.clone());
        let legacy = legacy_v2_bytes(&[("project-a", &project_dir)], Some("project-a"));
        fs::write(store.path(), &legacy).unwrap();
        let known = [KnownProject {
            id: "project-a".to_string(),
            display_name: "A".to_string(),
            canonical_path: project_dir.display().to_string(),
        }];
        let migrated = store.read(&known).unwrap().state.unwrap();

        let saved = store
            .save(
                WorkspaceStateSaveRequest {
                    expected_revision: 0,
                    state: migrated,
                },
                &known,
            )
            .unwrap();
        assert_eq!(saved.previous_revision, 0);
        assert_eq!(saved.state.revision, 1);
        assert_eq!(fs::read(dir.join(BACKUP_FILE_NAME)).unwrap(), legacy);
        assert_eq!(fs::read(dir.join(LEGACY_BACKUP_FILE_NAME)).unwrap(), legacy);
        let current: WorkspaceState =
            serde_json::from_slice(&fs::read(store.path()).unwrap()).unwrap();
        assert_eq!(current.schema_version, SCHEMA_VERSION);
        assert_eq!(current.kind, KIND);
        assert_eq!(current.revision, 1);

        store
            .save(
                WorkspaceStateSaveRequest {
                    expected_revision: 1,
                    state: current,
                },
                &known,
            )
            .unwrap();
        assert_eq!(fs::read(dir.join(LEGACY_BACKUP_FILE_NAME)).unwrap(), legacy);
        let rolling: WorkspaceState =
            serde_json::from_slice(&fs::read(dir.join(BACKUP_FILE_NAME)).unwrap()).unwrap();
        assert_eq!(rolling.revision, 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn current_state_rescues_legacy_rolling_backup_before_overwriting_it() {
        let dir = tempdir();
        let project_dir = dir.join("project-a");
        fs::create_dir_all(&project_dir).unwrap();
        let store = WorkspaceStateStore::new(dir.clone());
        let legacy = legacy_v2_bytes(&[("project-a", &project_dir)], Some("project-a"));
        fs::write(dir.join(BACKUP_FILE_NAME), &legacy).unwrap();
        let current = state_with_path(1, &project_dir);
        fs::write(store.path(), serde_json::to_vec_pretty(&current).unwrap()).unwrap();
        let known = [KnownProject {
            id: "project-a".to_string(),
            display_name: "A".to_string(),
            canonical_path: project_dir.display().to_string(),
        }];

        store
            .save(
                WorkspaceStateSaveRequest {
                    expected_revision: 1,
                    state: current,
                },
                &known,
            )
            .unwrap();
        assert_eq!(fs::read(dir.join(LEGACY_BACKUP_FILE_NAME)).unwrap(), legacy);
        let rolling: WorkspaceState =
            serde_json::from_slice(&fs::read(dir.join(BACKUP_FILE_NAME)).unwrap()).unwrap();
        assert_eq!(rolling.revision, 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn transitional_contract_v1_is_migrated_to_v3_and_backed_up_exactly() {
        let dir = tempdir();
        let project_dir = dir.join("project-a");
        fs::create_dir_all(&project_dir).unwrap();
        let store = WorkspaceStateStore::new(dir.clone());
        let mut transitional = state_with_path(2, &project_dir);
        transitional.schema_version = TRANSITIONAL_CONTRACT_SCHEMA_VERSION;
        let transitional_bytes = serde_json::to_vec_pretty(&transitional).unwrap();
        fs::write(store.path(), &transitional_bytes).unwrap();
        let known = [KnownProject {
            id: "project-a".to_string(),
            display_name: "A".to_string(),
            canonical_path: project_dir.display().to_string(),
        }];

        let result = store.read(&known).unwrap();
        assert_eq!(result.status, "recovered");
        assert!(result
            .diagnostics
            .iter()
            .any(|item| item.code == "workspace_state.migrated_from_transitional_v1"));
        let migrated = result.state.unwrap();
        assert_eq!(migrated.schema_version, SCHEMA_VERSION);
        assert_eq!(migrated.revision, 2);
        assert_eq!(migrated.active_project_id.as_deref(), Some("project-a"));

        let saved = store
            .save(
                WorkspaceStateSaveRequest {
                    expected_revision: 2,
                    state: migrated,
                },
                &known,
            )
            .unwrap();
        assert_eq!(saved.state.schema_version, SCHEMA_VERSION);
        assert_eq!(saved.state.revision, 3);
        assert_eq!(
            fs::read(dir.join(TRANSITIONAL_BACKUP_FILE_NAME)).unwrap(),
            transitional_bytes
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn current_v3_rescues_transitional_v1_rolling_backup() {
        let dir = tempdir();
        let project_dir = dir.join("project-a");
        fs::create_dir_all(&project_dir).unwrap();
        let store = WorkspaceStateStore::new(dir.clone());
        let mut transitional = state_with_path(1, &project_dir);
        transitional.schema_version = TRANSITIONAL_CONTRACT_SCHEMA_VERSION;
        let transitional_bytes = serde_json::to_vec_pretty(&transitional).unwrap();
        fs::write(dir.join(BACKUP_FILE_NAME), &transitional_bytes).unwrap();
        let current = state_with_path(2, &project_dir);
        fs::write(store.path(), serde_json::to_vec_pretty(&current).unwrap()).unwrap();
        let known = [KnownProject {
            id: "project-a".to_string(),
            display_name: "A".to_string(),
            canonical_path: project_dir.display().to_string(),
        }];

        store
            .save(
                WorkspaceStateSaveRequest {
                    expected_revision: 2,
                    state: current,
                },
                &known,
            )
            .unwrap();
        assert_eq!(
            fs::read(dir.join(TRANSITIONAL_BACKUP_FILE_NAME)).unwrap(),
            transitional_bytes
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn revision_conflict_does_not_overwrite_latest_state() {
        let dir = tempdir();
        let project_dir = dir.join("project-a");
        fs::create_dir_all(&project_dir).unwrap();
        let store = WorkspaceStateStore::new(dir.clone());
        let projects = [KnownProject {
            id: "project-a".to_string(),
            display_name: "A".to_string(),
            canonical_path: project_dir.display().to_string(),
        }];
        let first = store
            .save(
                WorkspaceStateSaveRequest {
                    expected_revision: 0,
                    state: state_with_path(0, &project_dir),
                },
                &projects,
            )
            .unwrap();
        assert_eq!(first.state.revision, 1);
        let error = store
            .save(
                WorkspaceStateSaveRequest {
                    expected_revision: 0,
                    state: state_with_path(0, &project_dir),
                },
                &projects,
            )
            .unwrap_err()
            .to_string();
        assert!(error.contains("WORKSPACE_STATE_REVISION_CONFLICT"));
        assert_eq!(
            store
                .read(&[KnownProject {
                    id: "project-a".to_string(),
                    display_name: "A".to_string(),
                    canonical_path: project_dir.display().to_string()
                }])
                .unwrap()
                .state
                .unwrap()
                .revision,
            1
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn damaged_primary_recovers_from_backup() {
        let dir = tempdir();
        let project_dir = dir.join("project-a");
        fs::create_dir_all(&project_dir).unwrap();
        let store = WorkspaceStateStore::new(dir.clone());
        let projects = [KnownProject {
            id: "project-a".to_string(),
            display_name: "A".to_string(),
            canonical_path: project_dir.display().to_string(),
        }];
        store
            .save(
                WorkspaceStateSaveRequest {
                    expected_revision: 0,
                    state: state_with_path(0, &project_dir),
                },
                &projects,
            )
            .unwrap();
        store
            .save(
                WorkspaceStateSaveRequest {
                    expected_revision: 1,
                    state: state_with_path(1, &project_dir),
                },
                &projects,
            )
            .unwrap();
        fs::write(store.path(), b"{\"partial\"").unwrap();
        let result = store
            .read(&[KnownProject {
                id: "project-a".to_string(),
                display_name: "A".to_string(),
                canonical_path: project_dir.display().to_string(),
            }])
            .unwrap();
        assert_eq!(result.status, "recovered");
        assert_eq!(result.state.unwrap().revision, 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn invalid_project_is_removed_without_fallback_to_root() {
        let dir = tempdir();
        let project_dir = dir.join("project-a");
        fs::create_dir_all(&project_dir).unwrap();
        let store = WorkspaceStateStore::new(dir.clone());
        let projects = [KnownProject {
            id: "project-a".to_string(),
            display_name: "A".to_string(),
            canonical_path: project_dir.display().to_string(),
        }];
        store
            .save(
                WorkspaceStateSaveRequest {
                    expected_revision: 0,
                    state: state_with_path(0, &project_dir),
                },
                &projects,
            )
            .unwrap();
        let result = store.read(&[]).unwrap();
        assert_eq!(result.status, "partial");
        assert!(result.state.unwrap().open_projects.is_empty());
        assert!(result
            .diagnostics
            .iter()
            .any(|item| item.code == "workspace_state.project_missing"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn unknown_schema_and_fields_are_rejected_without_fallback() {
        let dir = tempdir();
        let store = WorkspaceStateStore::new(dir.clone());
        fs::write(
            store.path(),
            br#"{"schema_version":99,"kind":"workspace_state","revision":1,"open_projects":[],"active_project_id":null,"updated_at":"2026-08-12T00:00:00Z","updated_by":"test","provenance":{"source":"user","writer":"test","reason":"test"},"unexpected":true}"#,
        )
        .unwrap();
        let result = store.read(&[]).unwrap();
        assert_eq!(result.status, "invalid");
        assert_eq!(result.state, None);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    #[cfg(unix)]
    fn canonical_path_identity_normalizes_alias_for_existing_project() {
        let dir = tempdir();
        let real = dir.join("real-project");
        fs::create_dir_all(&real).unwrap();
        let alias = dir.join("alias-project");
        std::os::unix::fs::symlink(&real, &alias).unwrap();
        let store = WorkspaceStateStore::new(dir.join("state"));
        let mut document = state(0);
        document.open_projects[0].canonical_path = alias.display().to_string();
        let known = [KnownProject {
            id: "project-a".to_string(),
            display_name: "A".to_string(),
            canonical_path: real.display().to_string(),
        }];
        let result = store
            .save(
                WorkspaceStateSaveRequest {
                    expected_revision: 0,
                    state: document,
                },
                &known,
            )
            .unwrap();
        assert_eq!(
            canonical_path_key(&result.state.open_projects[0].canonical_path),
            canonical_path_key(&real.display().to_string())
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn concurrent_writer_is_serialized_and_both_revisions_survive() {
        use std::sync::{Arc, Barrier};
        use std::thread;

        let dir = tempdir();
        let project_dir = dir.join("project-a");
        fs::create_dir_all(&project_dir).unwrap();
        let store = Arc::new(WorkspaceStateStore::new(dir.clone()));
        let known = Arc::new(vec![KnownProject {
            id: "project-a".to_string(),
            display_name: "A".to_string(),
            canonical_path: project_dir.display().to_string(),
        }]);
        let barrier = Arc::new(Barrier::new(3));
        let mut handles = Vec::new();
        for marker in ["one", "two"] {
            let store = Arc::clone(&store);
            let known = Arc::clone(&known);
            let barrier = Arc::clone(&barrier);
            let project_dir = project_dir.clone();
            handles.push(thread::spawn(move || {
                barrier.wait();
                let mut document = state_with_path(0, &project_dir);
                document.provenance.reason = marker.to_string();
                store.save(
                    WorkspaceStateSaveRequest {
                        expected_revision: 0,
                        state: document,
                    },
                    &known,
                )
            }));
        }
        barrier.wait();
        let results: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
        assert_eq!(store.read(&known).unwrap().state.unwrap().revision, 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn write_lock_timeout_is_structured_and_preserves_current_state() {
        let dir = tempdir();
        let project_dir = dir.join("project-a");
        fs::create_dir_all(&project_dir).unwrap();
        let store = WorkspaceStateStore::new(dir.clone());
        let known = [KnownProject {
            id: "project-a".to_string(),
            display_name: "A".to_string(),
            canonical_path: project_dir.display().to_string(),
        }];
        store
            .save(
                WorkspaceStateSaveRequest {
                    expected_revision: 0,
                    state: state_with_path(0, &project_dir),
                },
                &known,
            )
            .unwrap();
        fs::write(dir.join(LOCK_FILE_NAME), b"held").unwrap();
        let error = store
            .save(
                WorkspaceStateSaveRequest {
                    expected_revision: 1,
                    state: state_with_path(1, &project_dir),
                },
                &known,
            )
            .unwrap_err()
            .to_string();
        assert!(error.contains("WORKSPACE_STATE_WRITE_BUSY"));
        assert_eq!(store.read(&known).unwrap().state.unwrap().revision, 1);
        let _ = fs::remove_dir_all(dir);
    }
}

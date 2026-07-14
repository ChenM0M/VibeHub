use super::{
    inspect_project_layout, read_project_settings, AgentSpecTarget, OutputLanguage,
    ProjectLayoutState, V3Error, V3ErrorCategory, V3ProjectSettings,
};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const SPEC_VERSION: &str = "3.0";
const RENDERER_VERSION: &str = "3";
const STATE_SCHEMA_VERSION: u32 = 1;
const STATE_KIND: &str = "v3_agent_specs";
const STATE_FILE: &str = "agent-specs.yaml";
const MANAGED_START: &str = "<!-- VIBEHUB:AGENT-SPEC:START -->";
const MANAGED_END: &str = "<!-- VIBEHUB:AGENT-SPEC:END -->";
const OLD_START: &str = "<!-- VIBEHUB:AGENT-INTEGRATION:START -->";
const OLD_END: &str = "<!-- VIBEHUB:AGENT-INTEGRATION:END -->";
const MAX_ARTIFACT_BYTES: u64 = 1024 * 1024;
const MAX_STATE_BYTES: u64 = 256 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSpecArtifactStatus {
    Missing,
    InSync,
    Outdated,
    ModifiedOutside,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSpecArtifactInspection {
    pub path: String,
    pub consumers: Vec<AgentSpecTarget>,
    pub status: AgentSpecArtifactStatus,
    pub reason: String,
    pub current_hash: Option<String>,
    pub desired_hash: String,
    pub last_written_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSpecInspection {
    pub spec_version: String,
    pub renderer_version: String,
    pub settings_revision: u64,
    pub artifacts: Vec<AgentSpecArtifactInspection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentSpecSyncRequest {
    #[serde(default)]
    pub force_managed_region: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSpecSyncResult {
    pub inspection: AgentSpecInspection,
    pub written_paths: Vec<String>,
    pub skipped_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentSpecRuntimeState {
    schema_version: u32,
    kind: String,
    spec_version: String,
    renderer_version: String,
    artifacts: Vec<AgentSpecRuntimeArtifact>,
    updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentSpecRuntimeArtifact {
    path: String,
    consumers: Vec<AgentSpecTarget>,
    desired_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_written_hash: Option<String>,
}

struct ValidatedPaths {
    root: PathBuf,
    state: PathBuf,
}

struct DesiredArtifact {
    path: PathBuf,
    relative_path: String,
    consumers: Vec<AgentSpecTarget>,
    region: String,
    desired_hash: String,
}

#[derive(Clone)]
struct FileSnapshot {
    whole_hash: Option<String>,
    bytes: Option<Vec<u8>>,
    permissions: Option<fs::Permissions>,
    region_range: Option<(usize, usize)>,
}

struct InspectedArtifact {
    public: AgentSpecArtifactInspection,
    desired: DesiredArtifact,
    snapshot: FileSnapshot,
}

pub fn inspect_agent_specs(project_root: impl AsRef<Path>) -> Result<AgentSpecInspection, V3Error> {
    let paths = validated_paths(project_root.as_ref())?;
    let settings = required_settings(&paths.root)?;
    let state = read_runtime_state(&paths.state)?;
    let artifacts = desired_artifacts(&paths.root, &settings);
    inspect_desired(&settings, artifacts, state.as_ref()).map(|items| AgentSpecInspection {
        spec_version: SPEC_VERSION.to_owned(),
        renderer_version: RENDERER_VERSION.to_owned(),
        settings_revision: settings.revision,
        artifacts: items.into_iter().map(|item| item.public).collect(),
    })
}

pub fn sync_agent_specs(
    project_root: impl AsRef<Path>,
    request: AgentSpecSyncRequest,
) -> Result<AgentSpecSyncResult, V3Error> {
    let paths = validated_paths(project_root.as_ref())?;
    let settings = required_settings(&paths.root)?;
    let prior_state = read_runtime_state(&paths.state)?;
    let inspected = inspect_desired(
        &settings,
        desired_artifacts(&paths.root, &settings),
        prior_state.as_ref(),
    )?;
    let mut written_paths = Vec::new();
    let mut skipped_paths = Vec::new();
    let mut state_artifacts = Vec::new();

    for item in inspected {
        let should_write = matches!(
            item.public.status,
            AgentSpecArtifactStatus::Missing | AgentSpecArtifactStatus::Outdated
        ) || (item.public.status == AgentSpecArtifactStatus::ModifiedOutside
            && request.force_managed_region);
        let last_written_hash = if should_write {
            let replacement = merged_content(&item)?;
            atomic_replace_with_precondition(&item.desired.path, &replacement, &item.snapshot)?;
            written_paths.push(item.public.path.clone());
            Some(item.desired.desired_hash.clone())
        } else {
            skipped_paths.push(item.public.path.clone());
            if item.public.status == AgentSpecArtifactStatus::InSync {
                Some(item.desired.desired_hash.clone())
            } else {
                item.public.last_written_hash.clone()
            }
        };
        state_artifacts.push(AgentSpecRuntimeArtifact {
            path: item.public.path,
            consumers: item.public.consumers,
            desired_hash: item.desired.desired_hash,
            last_written_hash,
        });
    }

    let state = AgentSpecRuntimeState {
        schema_version: STATE_SCHEMA_VERSION,
        kind: STATE_KIND.to_owned(),
        spec_version: SPEC_VERSION.to_owned(),
        renderer_version: RENDERER_VERSION.to_owned(),
        artifacts: state_artifacts,
        updated_at: now(),
    };
    let yaml = serde_yaml::to_string(&state)
        .map_err(|error| validation("V3_AGENT_SPECS_STATE_SERIALIZE_FAILED", error.to_string()))?;
    atomic_replace_unconditional(&paths.state, yaml.as_bytes())?;
    let inspection = inspect_agent_specs(&paths.root)?;
    Ok(AgentSpecSyncResult {
        inspection,
        written_paths,
        skipped_paths,
    })
}

fn validated_paths(project_root: &Path) -> Result<ValidatedPaths, V3Error> {
    let root = project_root
        .canonicalize()
        .map_err(|error| io_error("V3_AGENT_SPECS_ROOT_NOT_FOUND", error))?;
    let layout = inspect_project_layout(&root)?;
    if layout.state != ProjectLayoutState::V3 {
        return Err(validation(
            "V3_AGENT_SPECS_REQUIRES_V3",
            format!("agent specs require a V3 project, found {:?}", layout.state),
        ));
    }
    let runtime = root.join(".vibehub/runtime");
    let metadata = fs::symlink_metadata(&runtime)
        .map_err(|error| io_error("V3_AGENT_SPECS_RUNTIME_INVALID", error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(validation(
            "V3_AGENT_SPECS_RUNTIME_INVALID",
            ".vibehub/runtime must be a regular directory and must not be a symbolic link",
        ));
    }
    Ok(ValidatedPaths {
        root,
        state: runtime.join(STATE_FILE),
    })
}

fn required_settings(root: &Path) -> Result<V3ProjectSettings, V3Error> {
    read_project_settings(root)?.settings.ok_or_else(|| {
        validation(
            "V3_AGENT_SPECS_SETTINGS_MISSING",
            "project-settings.yaml must exist before agent specs can be inspected or synchronized",
        )
    })
}

fn desired_artifacts(root: &Path, settings: &V3ProjectSettings) -> Vec<DesiredArtifact> {
    let mut grouped: BTreeMap<&str, Vec<AgentSpecTarget>> = BTreeMap::new();
    for target in &settings.agent_spec_targets {
        let path = match target {
            AgentSpecTarget::ClaudeCode => "CLAUDE.md",
            AgentSpecTarget::Opencode | AgentSpecTarget::Codex => "AGENTS.md",
        };
        grouped.entry(path).or_default().push(*target);
    }
    grouped
        .into_iter()
        .map(|(relative_path, consumers)| {
            let region = render_region(&consumers, settings.output_language);
            DesiredArtifact {
                path: root.join(relative_path),
                relative_path: relative_path.to_owned(),
                consumers,
                desired_hash: hash(region.as_bytes()),
                region,
            }
        })
        .collect()
}

fn render_region(consumers: &[AgentSpecTarget], language: OutputLanguage) -> String {
    let consumer_names = consumers
        .iter()
        .map(|target| match target {
            AgentSpecTarget::ClaudeCode => "claude_code",
            AgentSpecTarget::Opencode => "opencode",
            AgentSpecTarget::Codex => "codex",
        })
        .collect::<Vec<_>>()
        .join(", ");
    let language_name = match language {
        OutputLanguage::ZhCn => "zh-CN",
        OutputLanguage::ZhTw => "zh-TW",
        OutputLanguage::EnUs => "en-US",
    };
    let rules = match language {
        OutputLanguage::ZhCn => [
            "VibeHub V3 的任务、计划及其事件是工作流事实来源；文件或聊天叙述不是事实来源。",
            "工作流状态读写必须使用 V3 typed commands 或 MCP 工具；通过 MCP 写入时 expected_version 与 idempotency_key 可省略（服务端自动解析）；显式提供时必须准确，配置类命令必须遵守其 revision/precondition 契约。",
            "禁止直接写入事件日志、投影或 current pointer；只能通过受支持的命令接口改变状态。",
            "禁止恢复 V2 state、run、agent-view、adapters 或其他旧协议文件。",
            "开始工作前必须读取 V3 current task、task lifecycle、plan 和 session 投影；不得用 V2 status/sync/output 或旧仓库 skills 推断当前状态。",
            "优先使用已连接的 V3 MCP；MCP 不可用时使用能输出 V3 JSON 的 CLI fallback。在 VibeHub 源码仓库中优先使用由当前源码构建的 <project_root>/target/debug/vibehub，不得假定 PATH 中的旧安装包兼容。若命令启动 GUI、没有 JSON 或版本不兼容，必须停止状态变更并明确报告控制面不可用。",
            "进入执行时必须先把目标 plan node 置为 active，再用 session_open 记录 task、node、Agent 和真实 working directory；不得在无活动 session 的情况下声称正在执行。",
            "每完成一个可核验里程碑都必须写 progress 事件；发现阻塞、范围漂移、版本冲突或证据缺口时必须立即写 risk 事件，不得只在聊天中说明。",
            "计划、依赖或节点状态变化必须在发生的同一工作批次写入 V3 事件；禁止工作完成后再凭记忆一次性补写过程。",
            "结束或交接前必须写 agent_result（成功、失败或仍在运行的真实状态及证据），然后 session_close；中断恢复必须显式记录 gap/recover 或新的 session。",
            "只有全部必需 criterion 有可核验 evidence、finding 已闭环且用户通过受信渠道确认后，才能提议或确认 task 完成。",
            "只有存在可核验的工具结果、事件或测试证据时才能声称工作完成；缺少证据时必须明确说明未验证。",
        ],
        OutputLanguage::ZhTw => [
            "VibeHub V3 的任務、計畫及其事件是工作流程的事實來源；檔案或聊天敘述不是事實來源。",
            "工作流程狀態讀寫必須使用 V3 typed commands 或 MCP 工具；透過 MCP 寫入時 expected_version 與 idempotency_key 可省略（服務端自動解析）；顯式提供時必須準確，設定類命令必須遵守其 revision/precondition 契約。",
            "禁止直接寫入事件日誌、投影或 current pointer；只能透過受支援的命令介面改變狀態。",
            "禁止恢復 V2 state、run、agent-view、adapters 或其他舊協定檔案。",
            "開始工作前必須讀取 V3 current task、task lifecycle、plan 與 session 投影；不得用 V2 status/sync/output 或舊倉庫 skills 推斷目前狀態。",
            "優先使用已連線的 V3 MCP；MCP 不可用時使用能輸出 V3 JSON 的 CLI fallback。在 VibeHub 原始碼倉庫中優先使用由目前原始碼建置的 <project_root>/target/debug/vibehub，不得假定 PATH 中的舊安裝套件相容。若命令啟動 GUI、沒有 JSON 或版本不相容，必須停止狀態變更並明確回報控制面不可用。",
            "進入執行時必須先把目標 plan node 設為 active，再用 session_open 記錄 task、node、Agent 與真實 working directory；不得在沒有活動 session 的情況下宣稱正在執行。",
            "每完成一個可核驗里程碑都必須寫 progress 事件；發現阻塞、範圍漂移、版本衝突或證據缺口時必須立即寫 risk 事件，不得只在聊天中說明。",
            "計畫、依賴或節點狀態變化必須在發生的同一工作批次寫入 V3 事件；禁止工作完成後再憑記憶一次性補寫過程。",
            "結束或交接前必須寫 agent_result（成功、失敗或仍在執行的真實狀態及證據），然後 session_close；中斷恢復必須明確記錄 gap/recover 或新的 session。",
            "只有全部必要 criterion 具備可核驗 evidence、finding 已閉環且使用者透過受信管道確認後，才能提議或確認 task 完成。",
            "只有具備可核驗的工具結果、事件或測試證據時才能宣稱工作完成；缺少證據時必須明確說明尚未驗證。",
        ],
        OutputLanguage::EnUs => [
            "VibeHub V3 tasks, plans, and their events are the workflow source of truth; files and chat narration are not workflow truth.",
            "Read and mutate workflow state through V3 typed commands or MCP tools; when writing through MCP, expected_version and idempotency_key may be omitted (the server resolves them automatically), while explicitly provided values must be accurate; configuration mutations must follow their revision/precondition contract.",
            "Never write event logs, projections, or the current pointer directly; state changes must go through supported command interfaces.",
            "Do not restore V2 state, run, agent-view, adapters, or any other legacy protocol files.",
            "Before work, read the V3 current task, task lifecycle, plan, and session projections; never infer current state from V2 status/sync/output or legacy repository skills.",
            "Prefer a connected V3 MCP server; when MCP is unavailable, use a CLI fallback that emits V3 JSON. In a VibeHub source checkout, prefer <project_root>/target/debug/vibehub built from the current source and never assume an older PATH installation is compatible. If it launches a GUI, emits no JSON, or is incompatible, stop state mutations and report that the control plane is unavailable.",
            "Before execution, transition the target plan node to active and call session_open with the task, node, Agent, and real working directory; never claim execution without an active session.",
            "Write a progress event after every verifiable milestone. Write a risk event immediately for blockers, scope drift, version conflicts, or evidence gaps; chat-only reporting is insufficient.",
            "Write plan, dependency, and node-state changes in the same work batch in which they occur; do not reconstruct the process from memory after implementation finishes.",
            "Before stopping or handing off, write agent_result with the truthful succeeded, failed, or running state and evidence, then call session_close. Interrupted work must explicitly record gap/recover or open a new session.",
            "Propose or confirm task completion only after every required criterion has verifiable evidence, findings are closed, and the user confirms through a trusted channel.",
            "Claim completion only when supported by verifiable tool results, events, or test evidence; explicitly state when work is unverified.",
        ],
    };
    format!(
        "{MANAGED_START}\n# VibeHub V3 Agent Specification\n\n- Spec version: {SPEC_VERSION}\n- Renderer version: {RENDERER_VERSION}\n- Consumers: {consumer_names}\n- Output language: {language_name}\n\n{}\n{MANAGED_END}",
        rules
            .iter()
            .map(|rule| format!("- {rule}"))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

fn inspect_desired(
    _settings: &V3ProjectSettings,
    artifacts: Vec<DesiredArtifact>,
    state: Option<&AgentSpecRuntimeState>,
) -> Result<Vec<InspectedArtifact>, V3Error> {
    artifacts
        .into_iter()
        .map(|desired| {
            let last_written_hash = state
                .and_then(|state| {
                    state
                        .artifacts
                        .iter()
                        .find(|item| item.path == desired.relative_path)
                })
                .and_then(|item| item.last_written_hash.clone());
            let (snapshot, status, reason, current_hash) =
                inspect_file(&desired, last_written_hash.as_deref())?;
            Ok(InspectedArtifact {
                public: AgentSpecArtifactInspection {
                    path: desired.relative_path.clone(),
                    consumers: desired.consumers.clone(),
                    status,
                    reason,
                    current_hash,
                    desired_hash: desired.desired_hash.clone(),
                    last_written_hash,
                },
                desired,
                snapshot,
            })
        })
        .collect()
}

fn inspect_file(
    desired: &DesiredArtifact,
    last_written_hash: Option<&str>,
) -> Result<
    (
        FileSnapshot,
        AgentSpecArtifactStatus,
        String,
        Option<String>,
    ),
    V3Error,
> {
    let metadata = match fs::symlink_metadata(&desired.path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((
                FileSnapshot {
                    whole_hash: None,
                    bytes: None,
                    permissions: None,
                    region_range: None,
                },
                AgentSpecArtifactStatus::Missing,
                "artifact file does not exist".to_owned(),
                None,
            ));
        }
        Err(error) => return Err(io_error("V3_AGENT_SPECS_ARTIFACT_READ_FAILED", error)),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Ok(unsupported_snapshot(
            "artifact must be a regular file and must not be a symbolic link",
        ));
    }
    if metadata.len() > MAX_ARTIFACT_BYTES {
        return Ok(unsupported_snapshot("artifact exceeds the 1 MiB limit"));
    }
    let bytes = fs::read(&desired.path)
        .map_err(|error| io_error("V3_AGENT_SPECS_ARTIFACT_READ_FAILED", error))?;
    let whole_hash = hash(&bytes);
    let text = match std::str::from_utf8(&bytes) {
        Ok(text) => text,
        Err(_) => {
            return Ok(unsupported_with_bytes(
                bytes,
                metadata.permissions(),
                whole_hash,
                "artifact is not valid UTF-8",
            ))
        }
    };
    if text.contains(OLD_START) || text.contains(OLD_END) {
        return Ok(unsupported_with_bytes(
            bytes,
            metadata.permissions(),
            whole_hash,
            "legacy V2 managed marker detected",
        ));
    }
    let start_count = text.matches(MANAGED_START).count();
    let end_count = text.matches(MANAGED_END).count();
    if start_count > 1 || end_count > 1 {
        return Ok(unsupported_with_bytes(
            bytes,
            metadata.permissions(),
            whole_hash,
            "managed markers are duplicated",
        ));
    }
    if start_count != end_count {
        return Ok(unsupported_with_bytes(
            bytes,
            metadata.permissions(),
            whole_hash,
            "managed marker is missing its matching endpoint",
        ));
    }
    if start_count == 0 {
        return Ok((
            FileSnapshot {
                whole_hash: Some(whole_hash),
                bytes: Some(bytes),
                permissions: Some(metadata.permissions()),
                region_range: None,
            },
            AgentSpecArtifactStatus::Missing,
            "managed region is missing".to_owned(),
            None,
        ));
    }
    let start = text.find(MANAGED_START).unwrap();
    let end_marker = text.find(MANAGED_END).unwrap();
    if end_marker < start {
        return Ok(unsupported_with_bytes(
            bytes,
            metadata.permissions(),
            whole_hash,
            "managed markers are reversed",
        ));
    }
    let end = end_marker + MANAGED_END.len();
    let current_hash = hash(&text.as_bytes()[start..end]);
    let (status, reason) = if current_hash == desired.desired_hash {
        (
            AgentSpecArtifactStatus::InSync,
            "managed region matches desired content",
        )
    } else if last_written_hash == Some(current_hash.as_str()) {
        (
            AgentSpecArtifactStatus::Outdated,
            "renderer or settings changed since the last successful write",
        )
    } else {
        (
            AgentSpecArtifactStatus::ModifiedOutside,
            "managed region differs from both desired and last written content",
        )
    };
    Ok((
        FileSnapshot {
            whole_hash: Some(whole_hash),
            bytes: Some(bytes),
            permissions: Some(metadata.permissions()),
            region_range: Some((start, end)),
        },
        status,
        reason.to_owned(),
        Some(current_hash),
    ))
}

fn unsupported_snapshot(
    reason: &str,
) -> (
    FileSnapshot,
    AgentSpecArtifactStatus,
    String,
    Option<String>,
) {
    (
        FileSnapshot {
            whole_hash: None,
            bytes: None,
            permissions: None,
            region_range: None,
        },
        AgentSpecArtifactStatus::Unsupported,
        reason.to_owned(),
        None,
    )
}

fn unsupported_with_bytes(
    bytes: Vec<u8>,
    permissions: fs::Permissions,
    whole_hash: String,
    reason: &str,
) -> (
    FileSnapshot,
    AgentSpecArtifactStatus,
    String,
    Option<String>,
) {
    (
        FileSnapshot {
            whole_hash: Some(whole_hash),
            bytes: Some(bytes),
            permissions: Some(permissions),
            region_range: None,
        },
        AgentSpecArtifactStatus::Unsupported,
        reason.to_owned(),
        None,
    )
}

fn merged_content(item: &InspectedArtifact) -> Result<Vec<u8>, V3Error> {
    let Some(bytes) = item.snapshot.bytes.as_ref() else {
        return Ok(format!("{}\n", item.desired.region).into_bytes());
    };
    if let Some((start, end)) = item.snapshot.region_range {
        let mut output =
            Vec::with_capacity(bytes.len() - (end - start) + item.desired.region.len());
        output.extend_from_slice(&bytes[..start]);
        output.extend_from_slice(item.desired.region.as_bytes());
        output.extend_from_slice(&bytes[end..]);
        return Ok(output);
    }
    let mut output = bytes.clone();
    if !output.is_empty() && !output.ends_with(b"\n") {
        output.push(b'\n');
    }
    if !output.is_empty() {
        output.push(b'\n');
    }
    output.extend_from_slice(item.desired.region.as_bytes());
    output.push(b'\n');
    Ok(output)
}

fn read_runtime_state(path: &Path) -> Result<Option<AgentSpecRuntimeState>, V3Error> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(io_error("V3_AGENT_SPECS_STATE_READ_FAILED", error)),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(validation(
            "V3_AGENT_SPECS_STATE_INVALID",
            "agent-specs.yaml must be a regular file and must not be a symbolic link",
        ));
    }
    if metadata.len() > MAX_STATE_BYTES {
        return Err(validation(
            "V3_AGENT_SPECS_STATE_TOO_LARGE",
            "agent-specs.yaml exceeds the 256 KiB limit",
        ));
    }
    let bytes =
        fs::read(path).map_err(|error| io_error("V3_AGENT_SPECS_STATE_READ_FAILED", error))?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|error| validation("V3_AGENT_SPECS_STATE_INVALID_UTF8", error.to_string()))?;
    let state: AgentSpecRuntimeState = serde_yaml::from_str(text)
        .map_err(|error| validation("V3_AGENT_SPECS_STATE_INVALID_YAML", error.to_string()))?;
    if state.schema_version != STATE_SCHEMA_VERSION || state.kind != STATE_KIND {
        return Err(validation(
            "V3_AGENT_SPECS_STATE_SCHEMA_MISMATCH",
            "agent spec runtime state schema_version or kind is invalid",
        ));
    }
    Ok(Some(state))
}

fn atomic_replace_with_precondition(
    path: &Path,
    content: &[u8],
    snapshot: &FileSnapshot,
) -> Result<(), V3Error> {
    atomic_replace(path, content, snapshot.permissions.as_ref(), Some(snapshot))
}

fn verify_precondition(path: &Path, snapshot: &FileSnapshot) -> Result<(), V3Error> {
    match (&snapshot.whole_hash, fs::symlink_metadata(path)) {
        (None, Err(error)) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        (None, _) => Err(stale(path)),
        (Some(_), Err(_)) => Err(stale(path)),
        (Some(expected), Ok(metadata)) => {
            if metadata.file_type().is_symlink()
                || !metadata.is_file()
                || metadata.len() > MAX_ARTIFACT_BYTES
            {
                return Err(stale(path));
            }
            let current = fs::read(path)
                .map_err(|error| io_error("V3_AGENT_SPECS_ARTIFACT_READ_FAILED", error))?;
            if hash(&current) != *expected {
                Err(stale(path))
            } else {
                Ok(())
            }
        }
    }
}

fn stale(path: &Path) -> V3Error {
    V3Error::new(
        "V3_AGENT_SPECS_PRECONDITION_FAILED",
        V3ErrorCategory::StaleResource,
        true,
        format!("artifact changed after inspection: {}", path.display()),
    )
}

fn atomic_replace_unconditional(path: &Path, content: &[u8]) -> Result<(), V3Error> {
    let permissions = match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err(validation(
                "V3_AGENT_SPECS_STATE_INVALID",
                "agent-specs.yaml must be a regular file and must not be a symbolic link",
            ));
        }
        Ok(metadata) => Some(metadata.permissions()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(io_error("V3_AGENT_SPECS_STATE_READ_FAILED", error)),
    };
    atomic_replace(path, content, permissions.as_ref(), None)
}

fn atomic_replace(
    path: &Path,
    content: &[u8],
    permissions: Option<&fs::Permissions>,
    precondition: Option<&FileSnapshot>,
) -> Result<(), V3Error> {
    let parent = path
        .parent()
        .ok_or_else(|| validation("V3_AGENT_SPECS_WRITE_FAILED", "artifact path has no parent"))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("agent-spec");
    let temporary = parent.join(format!(".{file_name}.{}.tmp", Uuid::new_v4()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| io_error("V3_AGENT_SPECS_WRITE_FAILED", error))?;
        if let Some(permissions) = permissions {
            file.set_permissions(permissions.clone())
                .map_err(|error| io_error("V3_AGENT_SPECS_WRITE_FAILED", error))?;
        }
        file.write_all(content)
            .and_then(|_| file.sync_all())
            .map_err(|error| io_error("V3_AGENT_SPECS_WRITE_FAILED", error))?;
        if let Some(snapshot) = precondition {
            verify_precondition(path, snapshot)?;
        }
        replace_file(&temporary, path)
            .map_err(|error| io_error("V3_AGENT_SPECS_WRITE_FAILED", error))?;
        sync_directory(parent).map_err(|error| io_error("V3_AGENT_SPECS_SYNC_FAILED", error))
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

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
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
    use crate::v3::{initialize_v3, update_project_settings, V3ProjectSettingsUpdateRequest};

    fn project(targets: Vec<AgentSpecTarget>) -> PathBuf {
        let root = std::env::temp_dir().join(format!("vibehub-v3-agent-specs-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        initialize_v3(&root).unwrap();
        update_project_settings(
            &root,
            V3ProjectSettingsUpdateRequest {
                expected_revision: 0,
                output_language: OutputLanguage::EnUs,
                agent_spec_targets: targets,
                repository_remote_url: None,
            },
        )
        .unwrap();
        root
    }

    fn sync(root: &Path, force: bool) -> AgentSpecSyncResult {
        sync_agent_specs(
            root,
            AgentSpecSyncRequest {
                force_managed_region: force,
            },
        )
        .unwrap()
    }

    fn artifact<'a>(
        inspection: &'a AgentSpecInspection,
        path: &str,
    ) -> &'a AgentSpecArtifactInspection {
        inspection
            .artifacts
            .iter()
            .find(|item| item.path == path)
            .unwrap()
    }

    #[test]
    fn inspect_is_read_only_and_reports_missing() {
        let root = project(vec![AgentSpecTarget::ClaudeCode]);
        let before: Vec<_> = fs::read_dir(root.join(".vibehub/runtime"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        let inspection = inspect_agent_specs(&root).unwrap();
        let after: Vec<_> = fs::read_dir(root.join(".vibehub/runtime"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(
            artifact(&inspection, "CLAUDE.md").status,
            AgentSpecArtifactStatus::Missing
        );
        assert_eq!(before, after);
        assert!(!root.join("CLAUDE.md").exists());
        assert!(!root.join(".vibehub/runtime/agent-specs.yaml").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sync_creates_only_native_artifacts_and_runtime_state() {
        let root = project(vec![
            AgentSpecTarget::ClaudeCode,
            AgentSpecTarget::Opencode,
            AgentSpecTarget::Codex,
        ]);
        let result = sync(&root, false);
        assert_eq!(result.written_paths, vec!["AGENTS.md", "CLAUDE.md"]);
        assert_eq!(
            artifact(&result.inspection, "AGENTS.md").consumers,
            vec![AgentSpecTarget::Opencode, AgentSpecTarget::Codex]
        );
        assert!(result
            .inspection
            .artifacts
            .iter()
            .all(|item| item.status == AgentSpecArtifactStatus::InSync));
        let agents = fs::read_to_string(root.join("AGENTS.md")).unwrap();
        assert_eq!(agents.matches(MANAGED_START).count(), 1);
        assert!(agents.contains("Consumers: opencode, codex"));
        assert!(agents.contains("session_open"));
        assert!(agents.contains("progress"));
        assert!(agents.contains("agent_result"));
        for forbidden in [
            ".claude/settings.json",
            ".claude/commands",
            ".claude/hooks",
            ".claude/skills",
            "opencode.json",
            ".codex",
            ".vibehub/adapters",
            ".vibehub/agent-view",
        ] {
            assert!(
                !root.join(forbidden).exists(),
                "forbidden path created: {forbidden}"
            );
        }
        assert!(root.join(".vibehub/runtime/agent-specs.yaml").is_file());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn preserves_user_bytes_and_user_edits_remain_in_sync() {
        let root = project(vec![AgentSpecTarget::ClaudeCode]);
        let prefix = b"user prefix\r\nexact bytes\n";
        fs::write(root.join("CLAUDE.md"), prefix).unwrap();
        sync(&root, false);
        let first = fs::read(root.join("CLAUDE.md")).unwrap();
        assert!(first.starts_with(prefix));
        let mut edited = b"changed user prefix\r\n".to_vec();
        let start = first
            .windows(MANAGED_START.len())
            .position(|window| window == MANAGED_START.as_bytes())
            .unwrap();
        edited.extend_from_slice(&first[start..]);
        fs::write(root.join("CLAUDE.md"), &edited).unwrap();
        let inspection = inspect_agent_specs(&root).unwrap();
        assert_eq!(
            artifact(&inspection, "CLAUDE.md").status,
            AgentSpecArtifactStatus::InSync
        );
        assert!(sync(&root, false).written_paths.is_empty());
        assert_eq!(fs::read(root.join("CLAUDE.md")).unwrap(), edited);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn settings_change_marks_outdated_and_replaces_only_region() {
        let root = project(vec![AgentSpecTarget::ClaudeCode]);
        sync(&root, false);
        let path = root.join("CLAUDE.md");
        let current = fs::read_to_string(&path).unwrap();
        fs::write(&path, format!("before\n{current}after\n")).unwrap();
        update_project_settings(
            &root,
            V3ProjectSettingsUpdateRequest {
                expected_revision: 1,
                output_language: OutputLanguage::ZhCn,
                agent_spec_targets: vec![AgentSpecTarget::ClaudeCode],
                repository_remote_url: None,
            },
        )
        .unwrap();
        let inspection = inspect_agent_specs(&root).unwrap();
        assert_eq!(
            artifact(&inspection, "CLAUDE.md").status,
            AgentSpecArtifactStatus::Outdated
        );
        sync(&root, false);
        let updated = fs::read_to_string(path).unwrap();
        assert!(updated.starts_with("before\n"));
        assert!(updated.ends_with("after\n"));
        assert!(updated.contains("VibeHub V3 的任务"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn modified_region_requires_force() {
        let root = project(vec![AgentSpecTarget::ClaudeCode]);
        sync(&root, false);
        let path = root.join("CLAUDE.md");
        let changed = fs::read_to_string(&path)
            .unwrap()
            .replace("workflow source of truth", "manually changed truth");
        fs::write(&path, &changed).unwrap();
        let inspection = inspect_agent_specs(&root).unwrap();
        assert_eq!(
            artifact(&inspection, "CLAUDE.md").status,
            AgentSpecArtifactStatus::ModifiedOutside
        );
        assert!(sync(&root, false).written_paths.is_empty());
        assert_eq!(fs::read_to_string(&path).unwrap(), changed);
        assert_eq!(sync(&root, true).written_paths, vec!["CLAUDE.md"]);
        assert_eq!(
            artifact(&inspect_agent_specs(&root).unwrap(), "CLAUDE.md").status,
            AgentSpecArtifactStatus::InSync
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn malformed_old_and_invalid_files_are_unsupported_and_never_written() {
        let cases = [
            (format!("{MANAGED_START}\nno end"), "matching endpoint"),
            (format!("{MANAGED_END}\n{MANAGED_START}"), "reversed"),
            (
                format!("{MANAGED_START}\n{MANAGED_START}\n{MANAGED_END}"),
                "duplicated",
            ),
            (format!("{OLD_START}\nlegacy\n{OLD_END}"), "legacy V2"),
        ];
        for (content, reason) in cases {
            let root = project(vec![AgentSpecTarget::ClaudeCode]);
            fs::write(root.join("CLAUDE.md"), &content).unwrap();
            let item = inspect_agent_specs(&root).unwrap().artifacts.remove(0);
            assert_eq!(item.status, AgentSpecArtifactStatus::Unsupported);
            assert!(item.reason.contains(reason));
            sync(&root, true);
            assert_eq!(fs::read_to_string(root.join("CLAUDE.md")).unwrap(), content);
            fs::remove_dir_all(root).unwrap();
        }

        let root = project(vec![AgentSpecTarget::ClaudeCode]);
        fs::write(root.join("CLAUDE.md"), [0xff, 0xfe]).unwrap();
        assert_eq!(
            inspect_agent_specs(&root).unwrap().artifacts[0].status,
            AgentSpecArtifactStatus::Unsupported
        );
        sync(&root, true);
        assert_eq!(fs::read(root.join("CLAUDE.md")).unwrap(), [0xff, 0xfe]);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn symlink_is_unsupported_and_target_is_untouched() {
        use std::os::unix::fs::symlink;
        let root = project(vec![AgentSpecTarget::ClaudeCode]);
        let outside = root.join("outside.md");
        fs::write(&outside, "outside").unwrap();
        symlink(&outside, root.join("CLAUDE.md")).unwrap();
        assert_eq!(
            inspect_agent_specs(&root).unwrap().artifacts[0].status,
            AgentSpecArtifactStatus::Unsupported
        );
        sync(&root, true);
        assert_eq!(fs::read_to_string(outside).unwrap(), "outside");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn existing_file_without_region_is_missing_then_appended_once() {
        let root = project(vec![AgentSpecTarget::Codex, AgentSpecTarget::Opencode]);
        fs::write(root.join("AGENTS.md"), "user content without newline").unwrap();
        assert_eq!(
            inspect_agent_specs(&root).unwrap().artifacts[0].status,
            AgentSpecArtifactStatus::Missing
        );
        sync(&root, false);
        let content = fs::read_to_string(root.join("AGENTS.md")).unwrap();
        assert!(content.starts_with("user content without newline\n\n"));
        assert_eq!(content.matches(MANAGED_START).count(), 1);
        assert!(sync(&root, false).written_paths.is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}

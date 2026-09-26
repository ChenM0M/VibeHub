//! Agent read model. Never constructs the desktop bundle or replays task history.
//! Each request checks its dependency revision before and after materialization.
use super::{project_memory, BindingStatus, V3Error, V3ErrorCategory, V3EventStore};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const DEFAULT_BUDGET: usize = 16 * 1024;
pub const MAX_BUDGET: usize = 64 * 1024;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BriefQuery {
    pub task_id: String,
    pub node_id: Option<String>,
    pub session_id: Option<String>,
    pub if_revision: Option<String>,
    pub max_bytes: Option<usize>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectQuery {
    pub sequence_from: Option<u64>,
    pub sequence_to: Option<u64>,
    pub time_from: Option<String>,
    pub time_to: Option<String>,
    pub if_revision: Option<String>,
    pub task_id: String,
    pub section: Option<String>,
    pub entity_kind: Option<String>,
    pub entity_id: Option<String>,
    pub state: Option<String>,
    pub event_type: Option<String>,
    pub session_id: Option<String>,
    pub node_id: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<usize>,
    pub max_bytes: Option<usize>,
    pub offset: Option<usize>,
    #[serde(default)]
    pub include: Vec<String>,
}

/// Decisions are serialized from domain variants, never parsed from UI copy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NextStep {
    ToolCall {
        summary: String,
        tool: String,
        params: Value,
        scope: Value,
        revision: String,
        invalidated_by: Vec<String>,
    },
    WorkRequired {
        summary: String,
        scope: Value,
        inputs: Vec<String>,
        criterion_ids: Vec<String>,
    },
    EvidenceRequired {
        summary: String,
        scope: Value,
        criterion_ids: Vec<String>,
        evidence_categories: Vec<String>,
    },
    InputRequired {
        summary: String,
        question: String,
        reason: String,
        required_input: Vec<String>,
        scope: Value,
    },
    WaitExternal {
        summary: String,
        dependency: String,
        resume_condition: String,
        recheck: Value,
    },
    Done {
        summary: String,
        scope: Value,
        state: String,
    },
}

pub struct AgentReadService {
    root: PathBuf,
    project_id: String,
    store: V3EventStore,
}

fn error(code: &str, message: &str) -> V3Error {
    V3Error::new(code, V3ErrorCategory::Validation, false, message)
}
fn encode(value: &impl Serialize) -> Result<Value, V3Error> {
    serde_json::to_value(value).map_err(|e| error("V3_SERIALIZE_FAILED", &e.to_string()))
}
fn bytes(value: &Value) -> usize {
    serde_json::to_vec(value).expect("JSON value").len()
}
fn digest(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}
fn valid_id(id: &str) -> Result<(), V3Error> {
    if id.is_empty()
        || id.len() > 256
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        return Err(error(
            "V3_ID_INVALID",
            "ID must contain only ASCII letters, digits, '.', '_' or '-'.",
        ));
    }
    Ok(())
}
fn budget(value: Option<usize>) -> Result<usize, V3Error> {
    let n = value.unwrap_or(DEFAULT_BUDGET);
    if !(1024..=MAX_BUDGET).contains(&n) {
        return Err(error(
            "OUTPUT_BUDGET_INVALID",
            "max_bytes must be 1024..65536",
        ));
    }
    Ok(n)
}

fn project_evidence_refs(value: &mut Value, dictionary: &mut serde_json::Map<String, Value>) {
    match value {
        Value::Object(map) => {
            if let Some(refs) = map.remove("evidence_refs") {
                for id in refs
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                {
                    dictionary.entry(id.to_owned()).or_insert_with(||json!({"label":id.chars().take(120).collect::<String>(),"source":"registered_reference","instructional":false,"expand":{"entity_kind":"evidence","entity_id":id}}));
                }
                map.insert("evidence_ids".into(), refs);
            }
            for value in map.values_mut() {
                project_evidence_refs(value, dictionary);
            }
        }
        Value::Array(values) => {
            for value in values {
                project_evidence_refs(value, dictionary)
            }
        }
        _ => {}
    }
}

fn redact_secrets(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, value) in map.iter_mut() {
                let key = key.to_ascii_lowercase();
                if [
                    "password",
                    "api_key",
                    "apikey",
                    "authorization",
                    "access_token",
                    "refresh_token",
                    "secret",
                    "context_handle",
                    "credential",
                ]
                .iter()
                .any(|pattern| key.contains(pattern))
                {
                    *value = json!("[REDACTED]");
                } else {
                    redact_secrets(value);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                redact_secrets(value)
            }
        }
        _ => {}
    }
}

impl AgentReadService {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, V3Error> {
        let root = root
            .as_ref()
            .canonicalize()
            .map_err(|e| error("V3_ROOT_NOT_FOUND", &e.to_string()))?;
        let project_id = super::project_id(&root);
        Ok(Self {
            store: V3EventStore::open(&root)?,
            root,
            project_id,
        })
    }

    // Read authority only inside the canonical control root; reject symlink escapes.
    fn metadata(&self, task_id: &str) -> Result<Value, V3Error> {
        valid_id(task_id)?;
        let path = self
            .root
            .join(".vibehub/tasks")
            .join(task_id)
            .join("task.yaml");
        let real = path
            .canonicalize()
            .map_err(|e| error("V3_TASK_NOT_FOUND", &e.to_string()))?;
        if !real.starts_with(self.root.join(".vibehub/tasks")) {
            return Err(error(
                "V3_SCOPE_MISMATCH",
                "task metadata escapes the task root",
            ));
        }
        let content = fs::read(real).map_err(|e| error("V3_TASK_READ_FAILED", &e.to_string()))?;
        let value: Value = serde_yaml::from_slice(&content)
            .map_err(|e| error("V3_TASK_INVALID", &e.to_string()))?;
        if value["task_id"] != task_id {
            return Err(error(
                "V3_SCOPE_MISMATCH",
                "task identity does not match metadata",
            ));
        }
        Ok(value)
    }

    pub fn revision(&self, task_id: &str) -> Result<String, V3Error> {
        let metadata = self.metadata(task_id)?;
        let mut hasher = Sha256::new();
        hasher.update(self.project_id.as_bytes());
        hasher.update(self.store.read_revision(&self.project_id)?.to_le_bytes());
        hasher.update(serde_json::to_vec(&metadata).expect("metadata JSON"));
        // Conservative invalidation covers settings and externally edited policy/config.
        for name in ["project.yaml", "settings.yaml", "project-settings.yaml"] {
            let path = self.root.join(".vibehub").join(name);
            match fs::read(path) {
                Ok(content) => hasher.update(content),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(error("V3_CONFIG_READ_FAILED", &e.to_string())),
            }
        }
        Ok(format!("{:x}", hasher.finalize()))
    }

    fn envelope(&self, task: &str, revision: &str, data: Value) -> Value {
        json!({"contract_version":"agent-read/1","ok":true,"scope":{"project_id":self.project_id,"task_id":task},
            "revision":revision,"freshness":"fresh","completeness":"partial","content_trust":"untrusted_data","data":data})
    }
    fn finish(
        &self,
        task: &str,
        revision: &str,
        mut value: Value,
        max: usize,
    ) -> Result<Value, V3Error> {
        if self.revision(task)? != revision {
            return Err(error(
                "READ_REVISION_CHANGED",
                "Facts changed during this read; refresh the same query.",
            ));
        }
        if value.get("data").is_some() && value["data"]["event_id"].is_null() {
            let mut dictionary = serde_json::Map::new();
            project_evidence_refs(&mut value["data"], &mut dictionary);
            if !dictionary.is_empty() {
                value["evidence_dictionary"] = Value::Object(dictionary);
            }
        }
        let required = bytes(&value);
        if required > max {
            return Err(error("OUTPUT_BUDGET_TOO_SMALL", "Required context is not omitted; expand scoped details before acting.")
                .with_detail("required_bytes", required).with_detail("maximum_bytes", MAX_BUDGET)
                .with_detail("context_complete", false)
                .with_detail("recovery", json!({"kind":"read_prerequisites","tool":"task_inspect","task_id":task,"revision":revision,"sections":["task","plan","criteria","blockers","constraints"]})));
        }
        Ok(value)
    }

    /// Explicit task diagnostic export. No workspace files, credentials, personal
    /// Memory or context handles are collected. The manifest is the only default output.
    pub fn export(&self, task_id: &str, if_revision: Option<&str>) -> Result<Value, V3Error> {
        let revision = self.revision(task_id)?;
        if if_revision.is_some_and(|expected| expected != revision) {
            return Err(error(
                "READ_REVISION_CHANGED",
                "Requested snapshot is no longer current",
            ));
        }
        let mut snapshot = json!({"contract_version":"agent-diagnostic/1","scope":{"project_id":self.project_id,"task_id":task_id},"revision":revision,
            "metadata":self.metadata(task_id)?,"lifecycle":self.store.task_projection(&self.project_id,task_id)?,
            "events":self.store.load_task_events(&self.project_id,task_id)?.into_iter().filter(|e|!e.event_type.starts_with("memory.")).collect::<Vec<_>>(),"constraints":self.constraints(vec![])?,
            "exclusions":["workspace_files","project_configuration","credential_fields","personal_or_secret_memory","context_handles","operation_payloads","memory_events"],"content_trust":"untrusted_data"});
        redact_secrets(&mut snapshot);
        if self.revision(task_id)? != revision {
            return Err(error(
                "READ_REVISION_CHANGED",
                "Facts changed during export; retry the explicit export",
            ));
        }
        self.persist_export(&snapshot)
    }

    pub fn export_project(&self, expected: Option<&str>) -> Result<Value, V3Error> {
        fn metadata(service: &AgentReadService) -> Result<Vec<Value>, V3Error> {
            let mut paths = fs::read_dir(service.root.join(".vibehub/tasks"))
                .map_err(|e| error("EXPORT_IO_FAILED", &e.to_string()))?
                .filter_map(Result::ok)
                .filter(|entry| {
                    entry.file_name() != "current" && entry.path().join("task.yaml").is_file()
                })
                .collect::<Vec<_>>();
            paths.sort_by_key(|entry| entry.file_name());
            paths
                .into_iter()
                .map(|entry| service.metadata(&entry.file_name().to_string_lossy()))
                .collect()
        }
        let tasks = metadata(self)?;
        let sequence = self.store.read_revision(&self.project_id)?;
        let revision =
            digest(&serde_json::to_vec(&json!([sequence, tasks])).expect("snapshot JSON"));
        if expected.is_some_and(|r| r != revision) {
            return Err(error(
                "READ_REVISION_CHANGED",
                "Project snapshot revision changed",
            ));
        }
        let mut projection = encode(&self.store.indexed_projection(&self.project_id)?)?;
        projection.as_object_mut().unwrap().remove("project_memory");
        let mut snapshot = json!({"contract_version":"agent-diagnostic/1","scope":{"project_id":self.project_id},"scope_kind":"project","revision":revision,"task_metadata":tasks,"projection":projection,
            "events":self.store.load_project(&self.project_id)?.into_iter().filter(|e|!e.event_type.starts_with("memory.")).collect::<Vec<_>>(),
            "exclusions":["workspace_files","project_configuration","credential_fields","all_project_memory","context_handles","operation_payloads","memory_events"],"content_trust":"untrusted_data"});
        redact_secrets(&mut snapshot);
        let final_revision = digest(
            &serde_json::to_vec(&json!([
                self.store.read_revision(&self.project_id)?,
                metadata(self)?
            ]))
            .expect("snapshot JSON"),
        );
        if final_revision != revision {
            return Err(error(
                "READ_REVISION_CHANGED",
                "Project changed during export; retry explicit export",
            ));
        }
        self.persist_export(&snapshot)
    }

    /// Both snapshot scopes use one checked artifact writer and manifest contract.
    fn persist_export(&self, snapshot: &Value) -> Result<Value, V3Error> {
        let body = serde_json::to_vec(snapshot).expect("diagnostic JSON");
        let hash = digest(&body);
        let id = uuid::Uuid::new_v4().to_string();
        let control = self.root.join(".vibehub");
        if control
            .canonicalize()
            .map_err(|e| error("EXPORT_SCOPE_INVALID", &e.to_string()))?
            != control
        {
            return Err(error(
                "EXPORT_SCOPE_INVALID",
                "Control directory must not be a symlink",
            ));
        }
        let directory = self.root.join(".vibehub/diagnostics");
        fs::create_dir_all(&directory).map_err(|e| error("EXPORT_IO_FAILED", &e.to_string()))?;
        if fs::symlink_metadata(&directory)
            .map_err(|e| error("EXPORT_IO_FAILED", &e.to_string()))?
            .file_type()
            .is_symlink()
        {
            return Err(error(
                "EXPORT_SCOPE_INVALID",
                "Diagnostic directory cannot be a symlink",
            ));
        }
        let destination = directory.join(format!("{id}.json"));
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&destination)
            .map_err(|e| error("EXPORT_IO_FAILED", &e.to_string()))?;
        file.write_all(&body)
            .and_then(|_| file.sync_all())
            .map_err(|e| error("EXPORT_IO_FAILED", &e.to_string()))?;
        let mut receipt = json!({"contract_version":"agent-diagnostic/1","ok":true,"scope":snapshot["scope"],"revision":snapshot["revision"],"export_id":id,"bytes":body.len(),"sha256":hash,
            "path":format!(".vibehub/diagnostics/{id}.json"),"expires_after_seconds":86400,
            "exclusions":snapshot["exclusions"],"read":{"tool":"diagnostic_read","export_id":id,"offset":0},"cleanup":"delete via diagnostic_read(action=delete); expired artifacts are unavailable"});
        if let Some(kind) = snapshot.get("scope_kind") {
            receipt["scope_kind"] = kind.clone();
            receipt["cleanup"] = json!("diagnostic_read action=delete");
        }
        Ok(receipt)
    }

    pub fn read_export(
        &self,
        id: &str,
        offset: usize,
        max_bytes: Option<usize>,
        delete: bool,
    ) -> Result<Value, V3Error> {
        uuid::Uuid::parse_str(id)
            .map_err(|_| error("EXPORT_ID_INVALID", "Expected an export UUID"))?;
        let path = self
            .root
            .join(".vibehub/diagnostics")
            .join(format!("{id}.json"));
        let real = path
            .canonicalize()
            .map_err(|e| error("EXPORT_NOT_FOUND", &e.to_string()))?;
        if !real.starts_with(self.root.join(".vibehub/diagnostics"))
            || fs::symlink_metadata(&path)
                .map_err(|e| error("EXPORT_IO_FAILED", &e.to_string()))?
                .file_type()
                .is_symlink()
        {
            return Err(error(
                "EXPORT_SCOPE_INVALID",
                "Export is outside diagnostic scope",
            ));
        }
        if delete {
            fs::remove_file(path).map_err(|e| error("EXPORT_IO_FAILED", &e.to_string()))?;
            return Ok(json!({"ok":true,"export_id":id,"deleted":true}));
        }
        let mut file =
            fs::File::open(path).map_err(|e| error("EXPORT_IO_FAILED", &e.to_string()))?;
        let meta = file
            .metadata()
            .map_err(|e| error("EXPORT_IO_FAILED", &e.to_string()))?;
        if meta
            .modified()
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age.as_secs() > 86400)
        {
            return Err(error(
                "EXPORT_EXPIRED",
                "Export expired; delete it and create a new explicit snapshot",
            ));
        }
        if offset as u64 > meta.len() {
            return Err(error("CHUNK_OFFSET_INVALID", "offset exceeds export size"));
        }
        let max = budget(max_bytes)?;
        use std::io::{Read, Seek, SeekFrom};
        file.seek(SeekFrom::Start(offset as u64))
            .map_err(|e| error("EXPORT_IO_FAILED", &e.to_string()))?;
        // Read at most the requested budget, then fit the *encoded* JSON below.
        // Dividing by worst-case escaping wastes most normal-text payload space.
        let mut buffer = vec![0; max];
        let n = file
            .read(&mut buffer)
            .map_err(|e| error("EXPORT_IO_FAILED", &e.to_string()))?;
        buffer.truncate(n);
        let text = match std::str::from_utf8(&buffer) {
            Ok(text) => text,
            Err(e) if e.error_len().is_none() => {
                std::str::from_utf8(&buffer[..e.valid_up_to()]).unwrap()
            }
            Err(_) => {
                return Err(error(
                    "CHUNK_OFFSET_INVALID",
                    "offset must be a UTF-8 boundary",
                ))
            }
        };
        fit_encoded_chunk(text, max, |length| {
            let end = offset + length;
            json!({"contract_version":"agent-diagnostic/1","ok":true,"export_id":id,"encoding":"json-utf8","offset":offset,"text":&text[..length],"total_bytes":meta.len(),"next_offset":((end as u64)<meta.len()).then_some(end),"content_trust":"untrusted_data"})
        })
    }

    pub fn workspace(&self, max_bytes: Option<usize>) -> Result<Value, V3Error> {
        let max = budget(max_bytes)?;
        let mut paths = fs::read_dir(self.root.join(".vibehub/tasks"))
            .map_err(|e| error("V3_TASK_READ_FAILED", &e.to_string()))?
            .filter_map(Result::ok)
            .filter(|p| p.path().join("task.yaml").is_file())
            .collect::<Vec<_>>();
        paths.sort_by_key(|p| p.file_name());
        let mut candidates = Vec::new();
        let mut more = false;
        for path in paths {
            let id = path.file_name().to_string_lossy().to_string();
            if id == "current" {
                continue;
            }
            let metadata = self.metadata(&id)?;
            let p = self.store.task_projection(&self.project_id, &id)?;
            if matches!(
                p.state.as_str(),
                "completed" | "cancelled" | "closed_with_exceptions"
            ) {
                continue;
            }
            if candidates.len() == 5 {
                more = true;
                break;
            }
            let title = metadata["title"].as_str().unwrap_or("");
            candidates.push(json!({"task_id":id,"title":title.chars().take(160).collect::<String>(),"title_truncated":title.chars().count()>160,"state":p.state}));
        }
        let value = json!({"contract_version":"agent-read/1","ok":true,"scope":{"project_id":self.project_id},
            "data":{"candidates":candidates,"has_more":more,"capabilities":["task_brief","task_inspect","diagnostic_export"],"context_complete":false},
            "next_step":NextStep::InputRequired { summary:"Choose an explicit Task or continue read-only discussion; no Session has been opened or bound.".into(),question:"Which explicit Task should this work target, or is this read-only discussion?".into(),reason:"Current/default and candidate order are not binding authority".into(),required_input:vec!["explicit_task_id_or_read_only_intent".into()],scope:json!({"project_id":self.project_id}) }});
        if bytes(&value) > max {
            return Err(error(
                "OUTPUT_BUDGET_TOO_SMALL",
                "Candidate context exceeds requested budget",
            ));
        }
        Ok(value)
    }

    pub fn binding(&self, session_id: &str) -> Result<Option<super::SessionTaskBinding>, V3Error> {
        valid_id(session_id)?;
        self.store.read_binding(&self.project_id, session_id)
    }

    pub fn brief(&self, q: &BriefQuery) -> Result<Value, V3Error> {
        let max = budget(q.max_bytes)?;
        let revision = self.revision(&q.task_id)?;
        // Query scope participates in validators: revisions cannot be reused for a different node/session.
        let query_revision =
            digest(format!("{revision}:{:?}:{:?}", q.node_id, q.session_id).as_bytes());
        if q.if_revision.as_deref() == Some(&query_revision) {
            return self.finish(&q.task_id, &revision, json!({"contract_version":"agent-read/1","ok":true,"scope":{"project_id":self.project_id,"task_id":q.task_id},"revision":query_revision,"unchanged":true,"freshness":"fresh"}), max);
        }
        let metadata = self.metadata(&q.task_id)?;
        let lifecycle = self.store.task_projection(&self.project_id, &q.task_id)?;
        let binding = q
            .session_id
            .as_deref()
            .map(|id| self.binding(id))
            .transpose()?
            .flatten();
        if binding
            .as_ref()
            .is_some_and(|b| b.bound_task_id.as_deref().is_some_and(|id| id != q.task_id))
        {
            return Err(error(
                "V3_SCOPE_MISMATCH",
                "Session is bound to another Task; explicitly choose the correct scope.",
            ));
        }
        let session = q
            .session_id
            .as_ref()
            .and_then(|id| lifecycle.sessions.get(id));
        if q.node_id.as_ref().is_some_and(|id| {
            session
                .and_then(|s| s.node_id.as_ref())
                .is_some_and(|bound| bound != id)
        }) {
            return Err(error(
                "V3_SCOPE_MISMATCH",
                "Requested node differs from the Session node",
            ));
        }
        let node_id = q
            .node_id
            .as_deref()
            .or_else(|| session.and_then(|s| s.node_id.as_deref()));
        let node =
            match node_id {
                Some(id) => Some(lifecycle.effective_node(id).ok_or_else(|| {
                    error("V3_NODE_NOT_FOUND", "Node does not belong to this task")
                })?),
                None => None,
            };
        let criteria: Vec<_> = lifecycle
            .criteria
            .values()
            .filter(|c| node.is_none_or(|n| n.criterion_ids.contains(&c.criterion_id)))
            .collect();
        let dependencies: Vec<_> = node
            .into_iter()
            .flat_map(|n| n.dependencies.iter())
            .filter_map(|id| lifecycle.nodes.get(id))
            .collect();
        let integrity = self
            .store
            .read_session_integrity(&self.project_id, &q.task_id)?;
        let blockers = self.blockers(&q.task_id, &lifecycle, node_id, &integrity)?;
        let constraints = self.constraints(node.map(|n| n.scope.clone()).unwrap_or_default())?;
        let policy = lifecycle.execution_policy.as_ref().map(encode).transpose()?.or_else(||metadata.get("execution_policy").cloned()).unwrap_or_else(|| {
            let light=metadata["workflow_profile"]=="lightweight";
            json!({"effective_profile":metadata.get("workflow_profile").cloned().unwrap_or(json!("standard")),"planning_required":!light,"review_required":!light,"milestone_policy":if light {"minimal"} else {"per_milestone"},"required_records":if light {vec!["session","result","risk_if_any"]} else {vec!["plan","session","progress","result","review"]},"coverage_mode":"legacy_degraded"})
        });
        let (kind, summary) = if matches!(
            lifecycle.state.as_str(),
            "completed" | "cancelled" | "closed_with_exceptions"
        ) {
            ("done", "Task is terminal.")
        } else if lifecycle.state == "completion_pending" {
            ("input_required", "Obtain explicit current-user confirmation of the completion proposal; never infer consent.")
        } else if binding
            .as_ref()
            .is_none_or(|b| b.status != BindingStatus::Bound)
        {
            ("input_required", "Read-only context: explicitly select and bind this Task before writing or executing.")
        } else if session.is_some_and(|s| !matches!(s.state.as_str(), "active")) {
            ("input_required", "This Session is closed or interrupted; choose a new Session or recover its real interruption facts before execution.")
        } else if node.is_none() && policy["planning_required"] != false {
            (
                "tool_call",
                "Read the bounded plan to select a real eligible node; no node is inferred.",
            )
        } else if !blockers.is_empty()
            || dependencies
                .iter()
                .any(|n| !matches!(n.state.as_str(), "completed" | "waived"))
        {
            ("work_required", "Resolve the scoped findings and dependencies; record real evidence before advancing.")
        } else if integrity.iter().any(|s| {
            s["session_id"].as_str() == q.session_id.as_deref()
                && s["last_result_status"] == "succeeded"
        }) && criteria.iter().any(|c| {
            c.required
                && !matches!(
                    c.state,
                    super::lifecycle::CriterionState::Passed
                        | super::lifecycle::CriterionState::NotApplicable
                )
        }) {
            ("evidence_required", "Implementation result is recorded; run each outstanding acceptance check and record its observed outcome before advancing.")
        } else if node.is_some_and(|n| matches!(n.state.as_str(), "planned" | "ready" | "active"))
            || node.is_none() && policy["planning_required"] == false
        {
            ("work_required", "Implement the scoped goal and run the associated acceptance checks; preserve real evidence.")
        } else if criteria.iter().any(|c| {
            !matches!(
                c.state,
                super::lifecycle::CriterionState::Passed
                    | super::lifecycle::CriterionState::NotApplicable
            )
        }) {
            (
                "evidence_required",
                "Collect the required validation evidence and explicitly review its outcome.",
            )
        } else {
            ("input_required", "Select the intended node or supply the missing result/closure facts; no transition is inferred.")
        };
        let observed_directory = integrity
            .iter()
            .find(|s| s["session_id"].as_str() == q.session_id.as_deref())
            .and_then(|s| s["working_directory"].as_str());
        let directory = observed_directory
            .map(str::to_owned)
            .unwrap_or_else(|| self.root.to_string_lossy().into_owned());
        let mut value = self.envelope(&q.task_id, &query_revision, json!({
            "control_root":self.root,"working_directory":{"path":directory,"source":if observed_directory.is_some(){"session_opened"}else{"project_root_fallback"}},
            "title":metadata["title"],"goal":node.map(|n|json!(n.goal)).unwrap_or_else(||metadata["intent"].clone()),"task_goal":metadata["intent"],"state":super::lifecycle::task_truth_state(&lifecycle,metadata["phase_status"].as_str(),integrity.iter().any(|s|s["state"]=="closed"&&s["terminal_result"]==0),false),"lifecycle_state":lifecycle.state,
            "workflow_profile":metadata["workflow_profile"],"execution_policy":policy,
            "node":node,"scope":node.map(|n| &n.scope),"binding":binding,"session":session,"session_integrity":integrity.iter().find(|s|s["session_id"].as_str()==q.session_id.as_deref()),
            "criteria":criteria,"metadata_acceptance":metadata["acceptance_criteria"],
            "blockers":blockers,"dependencies":dependencies,"constraints":constraints,
            "context_complete":true,"content_trust":"untrusted_data",
            "expansions":["task","plan","criteria","blockers","sessions","results","timeline","evidence","constraints"]}));
        let scope = json!({"project_id":self.project_id,"task_id":q.task_id,"node_id":node_id});
        let criterion_ids = criteria.iter().map(|c| c.criterion_id.clone()).collect();
        let summary = summary.to_owned();
        let step = match kind {
            "tool_call" => NextStep::ToolCall {
                summary,
                tool: "task_inspect".into(),
                params: json!({"task_id":q.task_id,"section":"plan","if_revision":revision,"limit":25}),
                scope,
                revision: revision.clone(),
                invalidated_by: vec!["dependency_revision_changed".into(), "scope_changed".into()],
            },
            "work_required" => NextStep::WorkRequired {
                summary,
                scope,
                inputs: vec![
                    "data.goal".into(),
                    "data.scope".into(),
                    "data.constraints".into(),
                ],
                criterion_ids,
            },
            "evidence_required" => NextStep::EvidenceRequired {
                summary,
                scope,
                criterion_ids,
                evidence_categories: vec!["observed_validation_result".into()],
            },
            "done" => NextStep::Done {
                summary,
                scope,
                state: lifecycle.state.clone(),
            },
            _ => {
                let required = if lifecycle.state == "completion_pending" {
                    "explicit_current_user_confirmation"
                } else if binding
                    .as_ref()
                    .is_none_or(|b| b.status != BindingStatus::Bound)
                {
                    "explicit_session_identity_and_task_start"
                } else {
                    "intended_node_or_missing_result_facts"
                };
                NextStep::InputRequired {
                    question: summary.clone(),
                    summary,
                    reason: "Current authoritative facts do not authorize an automatic transition"
                        .into(),
                    required_input: vec![required.into()],
                    scope,
                }
            }
        };
        value["next_step"] = encode(&step)?;
        value["budget"] = json!({"limit_bytes":max,"truncated":false,"omitted_sections":["history","unrelated_nodes","raw_evidence"]});
        self.finish(&q.task_id, &revision, value, max)
    }

    fn blockers(
        &self,
        task_id: &str,
        lifecycle: &super::lifecycle::TaskLifecycleProjection,
        node_id: Option<&str>,
        integrity: &[Value],
    ) -> Result<Vec<Value>, V3Error> {
        let node = node_id.and_then(|id| lifecycle.effective_node(id));
        let criteria: Vec<_> = lifecycle
            .criteria
            .values()
            .filter(|c| node.is_none_or(|n| n.criterion_ids.contains(&c.criterion_id)))
            .collect();
        let findings: Vec<_> = lifecycle
            .findings
            .values()
            .filter(|f| {
                f.state != "closed"
                    && (f.target_node_id.is_none()
                        || node_id.is_none()
                        || f.target_node_id.as_deref() == node_id
                        || node.is_some_and(|n| {
                            f.target_node_id
                                .as_ref()
                                .is_some_and(|id| n.dependencies.contains(id))
                        }))
            })
            .collect();
        let worktrees = self.store.read_worktrees(&self.project_id, task_id)?;
        let mut blockers=findings.iter().map(|f|json!({"kind":"finding","entity_id":f.finding_id,"node_id":f.target_node_id,"state":f.state,"evidence_ids":f.evidence_refs,"reason":"finding_not_closed"})).collect::<Vec<_>>();
        for criterion in &criteria {
            if matches!(
                criterion.state,
                super::lifecycle::CriterionState::Failed
                    | super::lifecycle::CriterionState::Blocked
            ) {
                blockers.push(json!({"kind":"criterion","entity_id":criterion.criterion_id,"state":criterion.state,"reason":"required_validation_not_passed","evidence_ids":criterion.evidence_refs}));
            }
        }
        if let Some(node) = node {
            if matches!(node.state.as_str(), "blocked" | "failed") {
                blockers.push(json!({"kind":"node","entity_id":node.node_id,"state":node.state,"reason":"node_blocked","required_input":"Inspect recorded risk evidence and supply the actual recovery conditions"}));
            }
        }
        for session in integrity {
            let id = session["session_id"].as_str().unwrap_or("");
            let session_node = lifecycle
                .sessions
                .get(id)
                .and_then(|s| s.node_id.as_deref());
            if node_id.is_none() || session_node.is_none() || session_node == node_id {
                if matches!(session["state"].as_str(), Some("gapped" | "unknown"))
                    || (session["state"] == "closed" && session["terminal_result"] == 0)
                {
                    blockers.push(json!({"kind":"session","entity_id":id,"state":session["state"],"reason":"session_integrity_gap","required_input":"Recover the real session/result evidence; do not infer successful execution"}));
                }
            }
        }
        for worktree in &worktrees {
            if matches!(worktree["state"].as_str(), Some("conflicted" | "repairing")) {
                blockers.push(json!({"kind":"worktree","entity_id":worktree["worktree_id"],"state":worktree["state"],"reason":"worktree_repair_required"}));
            }
        }
        Ok(blockers)
    }

    fn constraints(&self, scope: Vec<String>) -> Result<Value, V3Error> {
        let memory = self.store.project_memory_projection(&self.project_id)?;
        // Personal entries are never injected without a reliable principal identity.
        let entries = project_memory::query(
            &memory,
            &super::MemoryQuery {
                scope,
                principal_scope: Some("project".into()),
                token_budget: usize::MAX,
                ..Default::default()
            },
        );
        encode(&entries)
    }

    pub fn inspect(&self, q: &InspectQuery) -> Result<Value, V3Error> {
        for (field, value) in [
            ("time_from", q.time_from.as_deref()),
            ("time_to", q.time_to.as_deref()),
        ] {
            if value.is_some_and(|value| chrono::DateTime::parse_from_rfc3339(value).is_err()) {
                return Err(error(
                    "TIME_FILTER_INVALID",
                    "Time boundaries must be RFC3339 timestamps",
                )
                .with_detail("field", field));
            }
        }
        if q.sequence_from
            .zip(q.sequence_to)
            .is_some_and(|(a, b)| a > b)
        {
            return Err(error(
                "SEQUENCE_RANGE_INVALID",
                "sequence_from must not exceed sequence_to",
            ));
        }
        let max = budget(q.max_bytes)?;
        let revision = self.revision(&q.task_id)?;
        if q.if_revision
            .as_deref()
            .is_some_and(|expected| expected != revision)
        {
            return Err(error(
                "READ_REVISION_CHANGED",
                "Chunk or entity snapshot changed; restart this scoped read",
            ));
        }
        let entity = q.entity_kind.is_some() && q.entity_id.is_some();
        if q.section.is_some() == entity || q.entity_kind.is_some() != q.entity_id.is_some() {
            return Err(error(
                "INSPECT_TARGET_INVALID",
                "Choose exactly one section OR entity_kind plus entity_id.",
            ));
        }
        let mut identity = q.clone();
        identity.cursor = None;
        identity.max_bytes = None;
        identity.limit = None;
        identity.offset = None;
        identity.if_revision = None;
        let query_hash = digest(&serde_json::to_vec(&identity).expect("query JSON"));
        let mut start = 0u64;
        if let Some(cursor) = &q.cursor {
            let parts: Vec<_> = cursor.split('.').collect();
            if parts.len() != 3 || parts[0] != revision || parts[1] != query_hash {
                return Err(error("CURSOR_STALE", "Cursor does not match this scope, filter or revision; repeat the query without cursor."));
            }
            start = parts[2]
                .parse()
                .map_err(|_| error("CURSOR_INVALID", "Invalid cursor position"))?;
        }
        if q.include.len() > 2
            || q.include
                .iter()
                .any(|s| !matches!(s.as_str(), "evidence" | "criteria"))
        {
            return Err(error(
                "INCLUDE_INVALID",
                "include supports at most evidence and criteria",
            ));
        }
        if !entity && !q.include.is_empty() {
            return Err(error(
                "INCLUDE_TARGET_REQUIRED",
                "include is available for direct entities",
            ));
        }
        if entity {
            let kind = q.entity_kind.as_deref().unwrap();
            let id = q.entity_id.as_deref().unwrap();
            let mut value = if matches!(kind, "event" | "evidence") {
                match self.store.read_event(&self.project_id, &q.task_id, id)? {
                    Some(event) => {
                        if event.event_type.starts_with("memory.") {
                            return Err(error(
                                "EVIDENCE_NOT_VISIBLE",
                                "Raw Memory events require their principal-aware Memory surface",
                            ));
                        }
                        encode(&event)?
                    }
                    None if kind == "evidence" => self
                        .store
                        .read_evidence_reference(&self.project_id, &q.task_id, id)?
                        .ok_or_else(|| {
                            error(
                                "ENTITY_NOT_FOUND",
                                "Evidence is not registered in this task",
                            )
                        })?,
                    None => {
                        return Err(error(
                            "ENTITY_NOT_FOUND",
                            "Event is not visible in this task",
                        ))
                    }
                }
            } else {
                self.store
                    .read_entity(&self.project_id, &q.task_id, kind, id)?
                    .ok_or_else(|| {
                        error("ENTITY_NOT_FOUND", "Entity is not visible in this task")
                    })?
            };
            let mut associated = serde_json::Map::new();
            if q.include.iter().any(|s| s == "evidence") {
                let refs = value["evidence_refs"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                associated.insert("evidence".into(),json!({"items":refs.iter().take(5).map(|id|json!({"evidence_id":id,"label":id,"body_available":false,"source":"reference_only","instructional":false})).collect::<Vec<_>>(),"total_count":refs.len(),"remaining_locator":if refs.len()>5 {json!({"entity_kind":kind,"entity_id":id,"field":"evidence_refs"})} else {Value::Null}}));
            }
            if q.include.iter().any(|s| s == "criteria") {
                if kind != "node" {
                    return Err(error(
                        "INCLUDE_INVALID",
                        "criteria include requires a node entity",
                    ));
                }
                let ids = value["criterion_ids"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                let mut items = Vec::new();
                for id in ids.iter().take(5).filter_map(Value::as_str) {
                    if let Some(c) =
                        self.store
                            .read_entity(&self.project_id, &q.task_id, "criterion", id)?
                    {
                        items.push(c);
                    }
                }
                associated.insert("criteria".into(),json!({"items":items,"total_count":ids.len(),"continuation":{"section":"criteria","node_id":id}}));
            }
            if !associated.is_empty() {
                value["included"] = Value::Object(associated);
            }
            return self.chunk(&q.task_id, &revision, value, q.offset.unwrap_or(0), max);
        }
        let section = q.section.as_deref().unwrap();
        let limit = q.limit.unwrap_or(25).clamp(1, 100);
        if section == "task" {
            return self.chunk(
                &q.task_id,
                &revision,
                self.metadata(&q.task_id)?,
                q.offset.unwrap_or(0),
                max,
            );
        }
        if section == "constraints" {
            return self.chunk(
                &q.task_id,
                &revision,
                self.constraints(vec![])?,
                q.offset.unwrap_or(0),
                max,
            );
        }
        if q.offset.is_some() {
            return Err(error(
                "CHUNK_TARGET_REQUIRED",
                "offset requires an entity or task/constraints section",
            ));
        }
        let mut values: Vec<(u64, Value)> = if matches!(
            section,
            "timeline" | "evidence" | "results"
        ) {
            let event_type = if section == "results" {
                Some("agent.result_recorded")
            } else {
                q.event_type.as_deref()
            };
            self.store.read_events_page(&self.project_id,&q.task_id,start,limit+1,event_type,q.session_id.as_deref(),q)?.into_iter().map(|(seq,e)|{
                let value=json!({"sequence":seq,"event_id":e.event_id,"event_type":e.event_type,"node_id":e.node_id,"session_id":e.session_id,"recorded_at":e.recorded_at,"evidence_grade":e.evidence_grade,"body_bytes":bytes(&e.payload),"expand":{"entity_kind":"event","entity_id":e.event_id}});
                (seq,value)
            }).collect()
        } else {
            let p = self.store.task_projection(&self.project_id, &q.task_id)?;
            let raw: Vec<Value> = match section {
                "task" => vec![self.metadata(&q.task_id)?],
                "plan" => p.nodes.values().map(encode).collect::<Result<_, _>>()?,
                "criteria" => p
                    .criteria
                    .values()
                    .filter(|c| {
                        q.node_id.as_ref().is_none_or(|id| {
                            p.nodes
                                .get(id)
                                .is_some_and(|n| n.criterion_ids.contains(&c.criterion_id))
                        })
                    })
                    .map(encode)
                    .collect::<Result<_, _>>()?,
                "blockers" => self.blockers(
                    &q.task_id,
                    &p,
                    q.node_id.as_deref(),
                    &self
                        .store
                        .read_session_integrity(&self.project_id, &q.task_id)?,
                )?,
                "sessions" => p.sessions.values().map(encode).collect::<Result<_, _>>()?,
                "constraints" => self
                    .constraints(vec![])?
                    .as_array()
                    .cloned()
                    .unwrap_or_default(),
                _ => return Err(error("SECTION_INVALID", "Unknown section")),
            };
            raw.into_iter()
                .filter(|v| q.state.as_ref().is_none_or(|state| v["state"] == *state))
                .enumerate()
                .filter(|(i, _)| *i as u64 >= start)
                .take(limit + 1)
                .map(|(i, v)| (i as u64 + 1, v))
                .collect()
        };
        let mut page = Vec::new();
        let mut position = start;
        let mut more = values.len() > limit;
        values.truncate(limit);
        for (seq, mut item) in values {
            if bytes(&item) > max / 2 {
                // Oversized entity remains directly reachable; pagination always advances.
                item = json!({"entity_id":item.get("entity_id").or_else(||item.get("node_id")).or_else(||item.get("criterion_id")).or_else(||item.get("finding_id")).or_else(||item.get("session_id")),"body_bytes":bytes(&item),"requires_chunk":true,"section":section,"expand":{"entity_kind":match section {"plan"=>"node","criteria"=>"criterion","blockers"=>item["kind"].as_str().unwrap_or("finding"),_=>"session"},"entity_id":item.get("entity_id").or_else(||item.get("node_id")).or_else(||item.get("criterion_id")).or_else(||item.get("finding_id")).or_else(||item.get("session_id"))}});
            }
            page.push(item);
            let mut candidate=self.envelope(&q.task_id,&revision,json!({"items":page,"next_cursor":format!("{revision}.{query_hash}.{seq}"),"returned_count":page.len(),"content_trust":"untrusted_data"}));
            let mut dictionary = serde_json::Map::new();
            project_evidence_refs(&mut candidate["data"], &mut dictionary);
            if !dictionary.is_empty() {
                candidate["evidence_dictionary"] = Value::Object(dictionary);
            }
            if bytes(&candidate) > max {
                page.pop();
                if page.is_empty() {
                    return Err(error("OUTPUT_BUDGET_TOO_SMALL","A page descriptor exceeds this budget; increase max_bytes or directly read the entity").with_detail("required_bytes",bytes(&candidate)));
                }
                more = true;
                break;
            }
            position = seq;
        }
        let next = more.then(|| format!("{revision}.{query_hash}.{position}"));
        let value=self.envelope(&q.task_id,&revision,json!({"items":page,"next_cursor":next,"returned_count":page.len(),"content_trust":"untrusted_data"}));
        self.finish(&q.task_id, &revision, value, max)
    }

    fn chunk(
        &self,
        task: &str,
        revision: &str,
        mut value: Value,
        offset: usize,
        max: usize,
    ) -> Result<Value, V3Error> {
        if value["event_id"].is_null() {
            let mut dictionary = serde_json::Map::new();
            project_evidence_refs(&mut value, &mut dictionary);
            if !dictionary.is_empty() {
                value["evidence_dictionary"] = Value::Object(dictionary);
            }
        }
        let full = self.envelope(task, revision, value.clone());
        if offset == 0 && bytes(&full) <= max {
            return self.finish(task, revision, full, max);
        }
        let body = serde_json::to_string(&value).expect("JSON");
        if offset > body.len() || !body.is_char_boundary(offset) {
            return Err(error(
                "CHUNK_OFFSET_INVALID",
                "offset must be a UTF-8 byte boundary within the body",
            ));
        }
        let hash = digest(body.as_bytes());
        let chunk = fit_encoded_chunk(&body[offset..], max, |length| {
            let end = offset + length;
            self.envelope(task,revision,json!({"encoding":"json-utf8","offset":offset,"total_bytes":body.len(),"sha256":hash,"text":&body[offset..end],"next_offset":(end<body.len()).then_some(end),"content_trust":"untrusted_data"}))
        })?;
        self.finish(task, revision, chunk, max)
    }
}

/// Select the longest UTF-8 prefix whose complete serialized response fits.
/// Search is bounded by max input bytes even for very large evidence bodies.
fn fit_encoded_chunk(
    body: &str,
    max: usize,
    mut envelope: impl FnMut(usize) -> Value,
) -> Result<Value, V3Error> {
    let mut boundaries: Vec<_> = body
        .char_indices()
        .map(|(index, _)| index)
        .take_while(|index| *index <= max)
        .collect();
    if body.len() <= max {
        boundaries.push(body.len());
    }
    if boundaries.is_empty() {
        boundaries.push(0);
    }
    let mut low = 0;
    let mut high = boundaries.len();
    while low + 1 < high {
        let middle = (low + high) / 2;
        if bytes(&envelope(boundaries[middle])) <= max {
            low = middle;
        } else {
            high = middle;
        }
    }
    let result = envelope(boundaries[low]);
    if bytes(&result) > max || (!body.is_empty() && boundaries[low] == 0) {
        return Err(error(
            "READ_BUDGET_EXCEEDED",
            "Budget cannot fit a complete chunk envelope and one UTF-8 character",
        ));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encoded_chunks_fill_budget_without_splitting_utf8_or_escaping_past_it() {
        for body in [
            "plain text ".repeat(1000),
            "中文🦀\"\\\n\u{0001}".repeat(1000),
        ] {
            let mut rebuilt = String::new();
            let mut calls = 0;
            while rebuilt.len() < body.len() {
                let value = fit_encoded_chunk(
                    &body[rebuilt.len()..],
                    1024,
                    |length| json!({"text":&body[rebuilt.len()..rebuilt.len()+length]}),
                )
                .unwrap();
                assert!(bytes(&value) <= 1024);
                let text = value["text"].as_str().unwrap();
                if rebuilt.len() + text.len() < body.len() {
                    assert!(bytes(&value) >= 1018);
                }
                rebuilt.push_str(text);
                calls += 1;
            }
            assert_eq!(rebuilt, body);
            assert!(calls < 30);
        }
    }
    fn fixture() -> PathBuf {
        let root = std::env::temp_dir().join(format!("agent-read-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub/tasks/task.test")).unwrap();
        fs::write(
            root.join(".vibehub/project.yaml"),
            "schema_version: 3\nproject_id: project.test\nname: read-test\n",
        )
        .unwrap();
        fs::write(root.join(".vibehub/tasks/task.test/task.yaml"),"task_id: task.test\ntitle: Test\nintent: Test scoped reads\nphase: implement\nphase_status: active\nworkflow_profile: lightweight\nacceptance_criteria: [real validation]\n").unwrap();
        root
    }
    #[test]
    fn brief_and_unchanged_do_not_read_history_or_build_bundle() {
        let root = fixture();
        let app = super::super::V3ApplicationService::open(&root).unwrap();
        app.session_open(
            "project.test",
            "task.test",
            "session.test",
            "tester",
            0,
            "open",
        )
        .unwrap();
        app.event_log(
            "progress",
            "project.test",
            "task.test",
            "session.test",
            "tester",
            2,
            "log",
            json!({"summary":"a real progress fact"}),
        )
        .unwrap();
        let reads = AgentReadService::open(&root).unwrap();
        let mut q = BriefQuery {
            task_id: "task.test".into(),
            session_id: Some("session.test".into()),
            ..Default::default()
        };
        super::super::read_metrics::reset();
        let result = reads.brief(&q).unwrap();
        assert_eq!(result["data"]["goal"], "Test scoped reads");
        assert_eq!(result["next_step"]["kind"], "work_required");
        q.if_revision = result["revision"].as_str().map(str::to_owned);
        let unchanged = reads.brief(&q).unwrap();
        assert_eq!(unchanged["unchanged"], true);
        assert!(bytes(&unchanged) <= 1024);
        let work = super::super::read_metrics::snapshot();
        assert_eq!(work.bundles, 0);
        assert_eq!(work.task_history_reads, 0);
        assert_eq!(work.project_history_reads, 0);
        assert_eq!(work.decoded_events, 0);
        let event = app.task_lifecycle("project.test", "task.test").unwrap();
        assert!(event.criteria.is_empty());
        let records = reads
            .store
            .load_task_events("project.test", "task.test")
            .unwrap();
        super::super::read_metrics::reset();
        let evidence = reads
            .inspect(&InspectQuery {
                task_id: "task.test".into(),
                entity_kind: Some("event".into()),
                entity_id: Some(records[2].event_id.clone()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(evidence["data"]["event_type"], "progress.logged");
        let work = super::super::read_metrics::snapshot();
        assert_eq!(work.decoded_events, 1);
        assert_eq!(work.task_history_reads, 0);
        assert_eq!(work.bundles, 0);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    #[ignore = "explicit 1k/100k deterministic read benchmark"]
    fn scale_read_benchmark() {
        use std::io::Write;
        for (count, other) in [(1000usize, false), (100000, true), (100000, false)] {
            let root = fixture();
            let app = super::super::V3ApplicationService::open(&root).unwrap();
            app.session_open(
                "project.test",
                "task.test",
                "session.test",
                "tester",
                0,
                "open.current",
            )
            .unwrap();
            let target = if other { "task.other" } else { "task.test" };
            let session = if other {
                "session.other"
            } else {
                "session.test"
            };
            if other {
                fs::create_dir_all(root.join(".vibehub/tasks/task.other")).unwrap();
                let metadata = fs::read_to_string(root.join(".vibehub/tasks/task.test/task.yaml"))
                    .unwrap()
                    .replace("task.test", "task.other");
                fs::write(root.join(".vibehub/tasks/task.other/task.yaml"), metadata).unwrap();
                app.session_open("project.test", target, session, "tester", 0, "open.other")
                    .unwrap();
            }
            app.event_log(
                "progress",
                "project.test",
                target,
                session,
                "tester",
                2,
                "seed",
                json!({"summary":"deterministic legal progress fixture"}),
            )
            .unwrap();
            let store = V3EventStore::open(&root).unwrap();
            let seed = store
                .event_by_idempotency_key("project.test", "seed")
                .unwrap()
                .unwrap();
            // Isolated performance fixture: repeat the exact event shape produced by
            // the application, with consecutive versions and unique identities. No
            // synthetic pass/completion/confirmation facts are introduced.
            let log = root.join(".vibehub/v3/projects/project.test/events.jsonl");
            let mut writer =
                std::io::BufWriter::new(fs::OpenOptions::new().append(true).open(log).unwrap());
            for i in 1..=count {
                let mut event = seed.clone();
                event.event_id = uuid::Uuid::new_v4().to_string();
                event.idempotency_key = format!("scale.{i}");
                event.expected_version = 2 + i as u64;
                event.aggregate_version = 3 + i as u64;
                serde_json::to_writer(&mut writer, &event).unwrap();
                writer.write_all(b"\n").unwrap();
            }
            writer.flush().unwrap();
            drop(writer);
            // A real validator-backed continuation must still accept this history.
            app.event_log(
                "progress",
                "project.test",
                target,
                session,
                "tester",
                3 + count as u64,
                "after.scale",
                json!({"summary":"validated continuation"}),
            )
            .unwrap();
            let reads = AgentReadService::open(&root).unwrap();
            let q = BriefQuery {
                task_id: "task.test".into(),
                session_id: Some("session.test".into()),
                ..Default::default()
            };
            let first = reads.brief(&q).unwrap();
            let unchanged = BriefQuery {
                if_revision: first["revision"].as_str().map(str::to_owned),
                ..q.clone()
            };
            for _ in 0..5 {
                reads.brief(&q).unwrap();
                reads.brief(&unchanged).unwrap();
            }
            let mut warm = Vec::new();
            let mut probe = Vec::new();
            super::super::read_metrics::reset();
            for _ in 0..30 {
                let time = std::time::Instant::now();
                reads.brief(&q).unwrap();
                warm.push(time.elapsed().as_secs_f64() * 1000.0);
                let time = std::time::Instant::now();
                reads.brief(&unchanged).unwrap();
                probe.push(time.elapsed().as_secs_f64() * 1000.0);
            }
            warm.sort_by(f64::total_cmp);
            probe.sort_by(f64::total_cmp);
            let work = super::super::read_metrics::snapshot();
            assert_eq!(work.bundles, 0);
            assert_eq!(work.task_history_reads, 0);
            assert_eq!(work.project_history_reads, 0);
            assert_eq!(work.decoded_events, 0);
            println!(
                "AGENT_SCALE {}",
                json!({"synthetic_progress_events":count,"other_task_history":other,"warmup":5,"samples":30,"brief_bytes":bytes(&first),"brief_p50_ms":warm[15],"brief_p95_ms":warm[28],"probe_p50_ms":probe[15],"probe_p95_ms":probe[28],"read_work":work})
            );
            assert!(warm[28] < 300.0, "brief p95 exceeded frozen local budget");
            assert!(probe[28] < 100.0, "probe p95 exceeded frozen local budget");
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn long_constraints_do_not_become_complete_or_silently_disappear() {
        let root = fixture();
        let path = root.join(".vibehub/tasks/task.test/task.yaml");
        let mut metadata: Value = serde_yaml::from_slice(&fs::read(&path).unwrap()).unwrap();
        metadata["intent"] = json!("约束".repeat(20000));
        fs::write(&path, serde_yaml::to_string(&metadata).unwrap()).unwrap();
        let reads = AgentReadService::open(&root).unwrap();
        let error = reads
            .brief(&BriefQuery {
                task_id: "task.test".into(),
                ..Default::default()
            })
            .unwrap_err();
        assert_eq!(error.code, "OUTPUT_BUDGET_TOO_SMALL");
        assert_eq!(error.details["context_complete"], false);
        let mut offset = 0;
        let mut body = String::new();
        loop {
            let part = reads
                .inspect(&InspectQuery {
                    task_id: "task.test".into(),
                    section: Some("task".into()),
                    offset: Some(offset),
                    max_bytes: Some(2048),
                    ..Default::default()
                })
                .unwrap();
            assert!(bytes(&part) <= 2048);
            body.push_str(part["data"]["text"].as_str().unwrap());
            match part["data"]["next_offset"].as_u64() {
                Some(next) => {
                    assert!(next > offset as u64);
                    offset = next as usize
                }
                None => break,
            }
        }
        assert_eq!(serde_json::from_str::<Value>(&body).unwrap(), metadata);
        fs::remove_dir_all(root).unwrap();
    }
}

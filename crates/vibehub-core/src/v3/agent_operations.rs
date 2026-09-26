//! Recoverable facade operations. Immutable request journals retain payload identity;
//! existing typed events are the only source of committed-step truth.
use super::{
    AppendResult, BindingSource, PlanCommandIdentity, PlanSetStateCommand, V3ApplicationService,
    V3Error, V3ErrorCategory, V3EventStore,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationRequest {
    pub request_id: String,
    pub kind: String,
    pub project_id: String,
    pub task_id: String,
    pub session_id: String,
    pub actor: String,
    pub binding_revision: Option<u64>,
    pub node_id: Option<String>,
    pub working_directory: Option<String>,
    pub interaction_id: Option<String>,
    pub details: Value,
}

fn error(code: &str, message: impl Into<String>) -> V3Error {
    V3Error::new(code, V3ErrorCategory::Validation, false, message)
}
fn io(e: std::io::Error) -> V3Error {
    V3Error::new(
        "OPERATION_IO_FAILED",
        V3ErrorCategory::Internal,
        false,
        e.to_string(),
    )
}
fn hash(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}
fn steps(kind: &str, node: bool) -> Result<Vec<&'static str>, V3Error> {
    match kind {
        "task_start" => Ok(if node {
            vec!["bind", "activate", "open"]
        } else {
            vec!["bind", "open"]
        }),
        "session_finish" => Ok(vec!["result", "close"]),
        "progress" | "risk" => Ok(vec!["record"]),
        _ => Err(error(
            "OPERATION_KIND_INVALID",
            "Unsupported operation kind",
        )),
    }
}
fn key(request_id: &str, step: &str) -> String {
    format!("agent.{}.{step}", hash(request_id.as_bytes()))
}

// Callers hold the operation lock. Keep identical durability for every journal.
fn write_journal(path: &Path, payload: &[u8]) -> Result<(), V3Error> {
    let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(io)?;
    file.write_all(payload).map_err(io)?;
    file.sync_all().map_err(io)?;
    drop(file);
    fs::rename(&temporary, path).map_err(io)?;
    #[cfg(unix)]
    File::open(path.parent().expect("operation directory"))
        .and_then(|f| f.sync_all())
        .map_err(io)?;
    Ok(())
}

pub struct AgentOperationService {
    root: PathBuf,
    app: V3ApplicationService,
    store: V3EventStore,
}
impl AgentOperationService {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, V3Error> {
        let root = root.as_ref().canonicalize().map_err(io)?;
        Ok(Self {
            app: V3ApplicationService::open(&root)?,
            store: V3EventStore::open(&root)?,
            root,
        })
    }
    fn directory(&self) -> Result<PathBuf, V3Error> {
        let control = self.root.join(".vibehub");
        if control.canonicalize().map_err(io)? != control {
            return Err(error(
                "OPERATION_PATH_INVALID",
                "Control directory must not be a symlink",
            ));
        }
        let dir = self.root.join(".vibehub/agent-operations");
        fs::create_dir_all(&dir).map_err(io)?;
        if fs::symlink_metadata(&dir)
            .map_err(io)?
            .file_type()
            .is_symlink()
        {
            return Err(error(
                "OPERATION_PATH_INVALID",
                "Operation directory cannot be a symlink",
            ));
        }
        Ok(dir)
    }
    fn lock_and_register(&self, q: &OperationRequest) -> Result<File, V3Error> {
        if q.project_id != super::project_id(&self.root) {
            return Err(error(
                "V3_SCOPE_MISMATCH",
                "Operation project does not match control root",
            ));
        }
        if q.request_id.trim().is_empty() || q.request_id.len() > 256 {
            return Err(error(
                "REQUEST_ID_INVALID",
                "request_id must have 1..256 bytes",
            ));
        }
        let dir = self.directory()?;
        let name = hash(q.request_id.as_bytes());
        let lock_path = dir.join(format!("{name}.lock"));
        if fs::symlink_metadata(&lock_path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(error(
                "OPERATION_PATH_INVALID",
                "Operation lock cannot be a symlink",
            ));
        }
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)
            .map_err(io)?;
        lock.lock().map_err(io)?;
        let path = dir.join(format!("{name}.json"));
        let payload = serde_json::to_vec(q).expect("operation JSON");
        if path.exists() {
            if fs::symlink_metadata(&path)
                .map_err(io)?
                .file_type()
                .is_symlink()
            {
                return Err(error(
                    "OPERATION_PATH_INVALID",
                    "Operation journal cannot be a symlink",
                ));
            }
            if fs::read(path).map_err(io)? != payload {
                return Err(error("IDEMPOTENCY_PAYLOAD_CONFLICT","request_id already belongs to a different normalized payload; retain the original request."));
            }
        } else {
            write_journal(&path, &payload)?;
        }
        let step_path = dir.join(format!("{name}.steps.json"));
        if !step_path.exists() {
            let activate = if q.kind == "task_start" {
                match &q.node_id {
                    Some(id) => self
                        .app
                        .task_lifecycle(&q.project_id, &q.task_id)?
                        .nodes
                        .get(id)
                        .is_none_or(|node| node.state != "active"),
                    None => false,
                }
            } else {
                false
            };
            let selected = steps(&q.kind, activate)?;
            write_journal(
                &step_path,
                &serde_json::to_vec(&selected).expect("steps JSON"),
            )?;
        }
        Ok(lock)
    }
    fn operation_steps(&self, q: &OperationRequest) -> Result<Vec<String>, V3Error> {
        let path = self
            .directory()?
            .join(format!("{}.steps.json", hash(q.request_id.as_bytes())));
        if !path.exists() {
            return Ok(steps(&q.kind, q.node_id.is_some())?
                .into_iter()
                .map(str::to_owned)
                .collect());
        }
        if fs::symlink_metadata(&path)
            .map_err(io)?
            .file_type()
            .is_symlink()
        {
            return Err(error(
                "OPERATION_PATH_INVALID",
                "Operation steps cannot be a symlink",
            ));
        }
        let selected: Vec<String> = serde_json::from_slice(&fs::read(path).map_err(io)?)
            .map_err(|e| error("OPERATION_JOURNAL_INVALID", e.to_string()))?;
        let expected = steps(&q.kind, q.node_id.is_some())?;
        if selected != expected && !(q.kind == "task_start" && selected == vec!["bind", "open"]) {
            return Err(error(
                "OPERATION_JOURNAL_INVALID",
                "Operation step plan does not match its typed request",
            ));
        }
        Ok(selected)
    }

    fn activation_witness(&self, q: &OperationRequest) -> Result<Option<Value>, V3Error> {
        let path = self
            .directory()?
            .join(format!("{}.activation.json", hash(q.request_id.as_bytes())));
        if !path.exists() {
            return Ok(None);
        }
        if fs::symlink_metadata(&path)
            .map_err(io)?
            .file_type()
            .is_symlink()
        {
            return Err(error(
                "OPERATION_PATH_INVALID",
                "Activation witness cannot be a symlink",
            ));
        }
        let witness: Value = serde_json::from_slice(&fs::read(path).map_err(io)?)
            .map_err(|e| error("OPERATION_JOURNAL_INVALID", e.to_string()))?;
        if witness["node_id"].as_str() != q.node_id.as_deref() {
            return Err(error(
                "OPERATION_JOURNAL_INVALID",
                "Activation witness scope differs",
            ));
        }
        Ok(Some(witness))
    }
    fn observe_existing_activation(&self, q: &OperationRequest) -> Result<bool, V3Error> {
        let lifecycle = self.app.task_lifecycle(&q.project_id, &q.task_id)?;
        let Some(node) = q
            .node_id
            .as_deref()
            .and_then(|id| lifecycle.effective_node(id))
        else {
            return Ok(false);
        };
        if node.state != "active" {
            return Ok(false);
        }
        // An interrupted start may be resumed after another valid start activated
        // this same node. Record an observed precondition, not a fabricated event.
        let witness = json!({"step":"activate","node_id":node.node_id,"task_version":lifecycle.version,"reason":"already_active_by_authoritative_projection"});
        let dir = self.directory()?;
        let name = hash(q.request_id.as_bytes());
        write_journal(
            &dir.join(format!("{name}.activation.json")),
            &serde_json::to_vec(&witness).expect("witness JSON"),
        )?;
        Ok(true)
    }

    pub fn status(
        &self,
        task_id: &str,
        session_id: &str,
        request_id: &str,
    ) -> Result<Value, V3Error> {
        let path = self
            .directory()?
            .join(format!("{}.json", hash(request_id.as_bytes())));
        if fs::symlink_metadata(&path)
            .map_err(io)?
            .file_type()
            .is_symlink()
        {
            return Err(error(
                "OPERATION_PATH_INVALID",
                "Operation journal cannot be a symlink",
            ));
        }
        let q: OperationRequest = serde_json::from_slice(&fs::read(path).map_err(io)?)
            .map_err(|e| error("OPERATION_JOURNAL_INVALID", e.to_string()))?;
        if q.task_id != task_id
            || q.session_id != session_id
            || q.project_id != super::project_id(&self.root)
        {
            return Err(error(
                "V3_SCOPE_MISMATCH",
                "Operation is outside requested scope",
            ));
        }
        self.receipt(&q, None)
    }
    fn receipt(&self, q: &OperationRequest, failure: Option<V3Error>) -> Result<Value, V3Error> {
        let mut completed = Vec::new();
        let mut pending = Vec::new();
        let mut events = Vec::new();
        let mut satisfied = Vec::new();
        for step in self.operation_steps(q)? {
            if let Some(event) = self
                .store
                .event_by_idempotency_key(&q.project_id, &key(&q.request_id, &step))?
            {
                completed.push(step.clone());
                events.push(json!({"step":step,"event_id":event.event_id,"aggregate_version":event.aggregate_version}));
            } else if step == "activate" && self.activation_witness(q)?.is_some() {
                satisfied.push(self.activation_witness(q)?.unwrap());
            } else {
                pending.push(step);
            }
        }
        let ok = pending.is_empty() && failure.is_none();
        let failure = failure.map(
            |e| json!({"code":e.code,"category":e.category,"message":e.message,"retryable":false}),
        );
        Ok(
            json!({"contract_version":"agent-operation/1","ok":ok,"request_id":q.request_id,
            "scope":{"project_id":q.project_id,"task_id":q.task_id,"session_id":q.session_id},
            "completed_steps":completed,"satisfied_preconditions":satisfied,"pending_steps":pending,"events":events,"error":failure,
            "state_changed":!completed.is_empty(),"recovery":{"tool":"operation_status","task_id":q.task_id,"session_id":q.session_id,"request_id":q.request_id},
            "retention":"request identity retained with project; do not remove while events remain or operations are pending"}),
        )
    }
    pub fn execute(&self, q: &OperationRequest) -> Result<Value, V3Error> {
        self.execute_inner(q, None)
    }
    fn execute_inner(
        &self,
        q: &OperationRequest,
        fail_after: Option<(usize, bool)>,
    ) -> Result<Value, V3Error> {
        steps(&q.kind, q.node_id.is_some())?;
        if q.kind == "task_start"
            && self
                .store
                .event_by_idempotency_key(&q.project_id, &key(&q.request_id, "bind"))?
                .is_none()
        {
            let lifecycle = self.app.task_lifecycle(&q.project_id, &q.task_id)?;
            if lifecycle
                .execution_policy
                .as_ref()
                .is_some_and(|p| p.planning_required)
                && q.node_id.is_none()
            {
                return Err(error(
                    "V3_SESSION_NODE_REQUIRED",
                    "standard/full start requires an authored node before binding",
                ));
            }
            if let Some(id) = &q.node_id {
                let node = lifecycle.effective_node(id).ok_or_else(|| {
                    error(
                        "V3_NODE_NOT_FOUND",
                        "Create the real plan before task_start",
                    )
                })?;
                if !matches!(node.state.as_str(), "planned" | "ready" | "active")
                    || node.dependencies.iter().any(|id| {
                        lifecycle
                            .nodes
                            .get(id)
                            .is_none_or(|n| !matches!(n.state.as_str(), "completed" | "waived"))
                    })
                {
                    return Err(error(
                        "V3_PLAN_DEPENDENCY_BLOCKED",
                        "Node must be eligible and dependencies completed/waived before binding",
                    ));
                }
            }
            if q.working_directory
                .as_ref()
                .is_none_or(|path| !Path::new(path).is_dir())
            {
                return Err(error(
                    "V3_WORKING_DIRECTORY_INVALID",
                    "A real existing working_directory is required",
                ));
            }
        }
        // Registration precedes commits; never journal handles or infer completion/consent.
        let _lock = self.lock_and_register(q)?;
        let steps = self.operation_steps(q)?;
        for (index, step) in steps.iter().enumerate() {
            if self
                .store
                .event_by_idempotency_key(&q.project_id, &key(&q.request_id, &step))?
                .is_some()
            {
                continue;
            }
            if step == "activate" && self.observe_existing_activation(q)? {
                continue;
            }
            let result = (|| -> Result<AppendResult, V3Error> {
                let session_version = self.app.aggregate_version(&q.project_id, &q.session_id)?;
                let step_key = key(&q.request_id, &step);
                if step != "bind" {
                    let revision = if q.kind == "task_start" {
                        self.app
                            .session_task_binding(&q.project_id, &q.session_id)?
                            .binding_revision
                    } else {
                        q.binding_revision.ok_or_else(|| {
                            error(
                                "V3_TASK_BINDING_REVISION_REQUIRED",
                                "Explicit binding revision required",
                            )
                        })?
                    };
                    self.app.validate_task_binding(
                        &q.project_id,
                        &q.task_id,
                        &q.session_id,
                        Some(revision),
                    )?;
                }
                match step.as_str() {
                    "bind" => self.app.session_task_bind(
                        &q.project_id,
                        &q.task_id,
                        &q.session_id,
                        q.interaction_id.as_deref().ok_or_else(|| {
                            error(
                                "INTERACTION_REQUIRED",
                                "Explicit interaction_id is required",
                            )
                        })?,
                        &q.actor,
                        BindingSource::ExplicitTaskId,
                        session_version,
                        &step_key,
                        q.binding_revision,
                        None,
                        None,
                        None,
                    ),
                    "activate" => self.app.plan_set_state(PlanSetStateCommand {
                        identity: PlanCommandIdentity {
                            project_id: q.project_id.clone(),
                            task_id: q.task_id.clone(),
                            actor: q.actor.clone(),
                            expected_version: self
                                .app
                                .aggregate_version(&q.project_id, &q.task_id)?,
                            idempotency_key: step_key,
                        },
                        node_id: q.node_id.clone().unwrap(),
                        state: "active".into(),
                    }),
                    "open" => self.app.session_open_with_context(
                        &q.project_id,
                        &q.task_id,
                        &q.session_id,
                        &q.actor,
                        session_version,
                        &step_key,
                        q.working_directory.clone(),
                        q.node_id.clone(),
                        None,
                    ),
                    "result" => {
                        if !matches!(q.details["status"].as_str(), Some("succeeded" | "failed")) {
                            return Err(error(
                                "TERMINAL_RESULT_REQUIRED",
                                "session_finish requires a real terminal result",
                            ));
                        }
                        self.app.agent_result_record(
                            &q.project_id,
                            &q.task_id,
                            &q.session_id,
                            &q.actor,
                            session_version,
                            &step_key,
                            &format!("result.{}", hash(q.request_id.as_bytes())),
                            q.node_id.clone(),
                            q.details.clone(),
                        )
                    }
                    "close" => self.app.session_close(
                        &q.project_id,
                        &q.task_id,
                        &q.session_id,
                        &q.actor,
                        session_version,
                        &step_key,
                    ),
                    "record" => self.app.event_log(
                        &q.kind,
                        &q.project_id,
                        &q.task_id,
                        &q.session_id,
                        &q.actor,
                        session_version,
                        &step_key,
                        q.details.clone(),
                    ),
                    _ => unreachable!(),
                }
            })();
            if let Err(e) = result {
                return self.receipt(q, Some(e));
            }
            if fail_after.is_some_and(|(point, _)| point == index + 1) {
                return self.receipt(
                    q,
                    Some(if fail_after.unwrap().1 {
                        io(std::io::Error::other(
                            "Injected IO failure after durable commit",
                        ))
                    } else {
                        error(
                            "INJECTED_INTERRUPTION",
                            "Failure injected after durable step",
                        )
                    }),
                );
            }
        }
        self.receipt(q, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (PathBuf, OperationRequest) {
        let root = std::env::temp_dir().join(format!("agent-operations-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub/tasks/task.test")).unwrap();
        fs::write(
            root.join(".vibehub/project.yaml"),
            "schema_version: 3\nproject_id: project.test\nname: operation-test\n",
        )
        .unwrap();
        fs::write(root.join(".vibehub/tasks/task.test/task.yaml"),"task_id: task.test\ntitle: Test\nintent: Test operation recovery\nphase: implement\nphase_status: active\nworkflow_profile: lightweight\nacceptance_criteria: []\n").unwrap();
        let q = OperationRequest {
            request_id: "start.1".into(),
            kind: "task_start".into(),
            project_id: "project.test".into(),
            task_id: "task.test".into(),
            session_id: "session.test".into(),
            actor: "test".into(),
            binding_revision: Some(0),
            node_id: None,
            working_directory: Some(root.to_string_lossy().into()),
            interaction_id: Some("interaction.test".into()),
            details: json!({}),
        };
        (root, q)
    }
    #[test]
    fn activation_recovers_at_each_commit_and_concurrent_retry_is_unique() {
        for (point, io_failure) in (1..=3).flat_map(|point| [(point, false), (point, true)]) {
            let (root, mut start) = fixture();
            let metadata = root.join(".vibehub/tasks/task.test/task.yaml");
            fs::write(
                &metadata,
                fs::read_to_string(&metadata).unwrap().replace(
                    "workflow_profile: lightweight",
                    "workflow_profile: standard",
                ),
            )
            .unwrap();
            start.node_id = Some("node.test".into());
            let first = AgentOperationService::open(&root).unwrap();
            first
                .app
                .plan_add_node(super::super::PlanAddNodeCommand {
                    identity: PlanCommandIdentity {
                        project_id: start.project_id.clone(),
                        task_id: start.task_id.clone(),
                        actor: start.actor.clone(),
                        expected_version: 0,
                        idempotency_key: "plan".into(),
                    },
                    node_id: "node.test".into(),
                    title: "Real plan".into(),
                    goal: "Verify recovery".into(),
                    scope: vec![],
                    dependencies: vec![],
                    criterion_ids: vec![],
                })
                .unwrap();
            let partial = first
                .execute_inner(&start, Some((point, io_failure)))
                .unwrap();
            assert_eq!(partial["ok"], false);
            assert_eq!(partial["completed_steps"].as_array().unwrap().len(), point);
            let handles = (0..2)
                .map(|_| {
                    let root = root.clone();
                    let q = start.clone();
                    std::thread::spawn(move || {
                        AgentOperationService::open(root)
                            .unwrap()
                            .execute(&q)
                            .unwrap()
                    })
                })
                .collect::<Vec<_>>();
            for handle in handles {
                assert_eq!(handle.join().unwrap()["ok"], true);
            }
            let reopened = AgentOperationService::open(&root).unwrap();
            assert_eq!(
                reopened
                    .store
                    .load_task_events(&start.project_id, &start.task_id)
                    .unwrap()
                    .len(),
                4
            );
            assert_eq!(
                reopened
                    .app
                    .task_lifecycle(&start.project_id, &start.task_id)
                    .unwrap()
                    .nodes["node.test"]
                    .state,
                "active"
            );
            // A second Session on an already active node must not emit active -> active.
            start.request_id = "start.second".into();
            start.session_id = "session.second".into();
            let second = reopened.execute(&start).unwrap();
            assert_eq!(second["ok"], true, "{second}");
            assert_eq!(second["completed_steps"], json!(["bind", "open"]));
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn interrupted_start_observes_other_sessions_real_activation() {
        let (root, mut start) = fixture();
        let metadata = root.join(".vibehub/tasks/task.test/task.yaml");
        fs::write(
            &metadata,
            fs::read_to_string(&metadata).unwrap().replace(
                "workflow_profile: lightweight",
                "workflow_profile: standard",
            ),
        )
        .unwrap();
        start.node_id = Some("node.test".into());
        let service = AgentOperationService::open(&root).unwrap();
        service
            .app
            .plan_add_node(super::super::PlanAddNodeCommand {
                identity: PlanCommandIdentity {
                    project_id: start.project_id.clone(),
                    task_id: start.task_id.clone(),
                    actor: start.actor.clone(),
                    expected_version: 0,
                    idempotency_key: "plan".into(),
                },
                node_id: "node.test".into(),
                title: "Real plan".into(),
                goal: "Verify interleaving".into(),
                scope: vec![],
                dependencies: vec![],
                criterion_ids: vec![],
            })
            .unwrap();
        assert_eq!(
            service.execute_inner(&start, Some((1, false))).unwrap()["ok"],
            false
        );
        let mut other = start.clone();
        other.session_id = "session.other".into();
        other.request_id = "start.other".into();
        assert_eq!(service.execute(&other).unwrap()["ok"], true);
        let resumed = service.execute(&start).unwrap();
        assert_eq!(resumed["ok"], true, "{resumed}");
        assert_eq!(
            resumed["satisfied_preconditions"].as_array().unwrap().len(),
            1
        );
        let events = service
            .store
            .load_task_events(&start.project_id, &start.task_id)
            .unwrap();
        assert_eq!(events.len(), 6); // plan, two binds, one activation, two opens
        assert_eq!(service.execute(&start).unwrap(), resumed);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn journal_io_failure_commits_nothing_and_original_request_can_resume() {
        let (root, start) = fixture();
        let service = AgentOperationService::open(&root).unwrap();
        fs::write(root.join(".vibehub/agent-operations"), "blocked directory").unwrap();
        assert_eq!(
            service.execute(&start).unwrap_err().code,
            "OPERATION_IO_FAILED"
        );
        assert!(service
            .store
            .load_task_events(&start.project_id, &start.task_id)
            .unwrap()
            .is_empty());
        fs::remove_file(root.join(".vibehub/agent-operations")).unwrap();
        assert_eq!(
            AgentOperationService::open(&root)
                .unwrap()
                .execute(&start)
                .unwrap()["ok"],
            true
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resume_every_start_and_finish_commit_after_restart() {
        for (point, io_failure) in (1..=2).flat_map(|point| [(point, false), (point, true)]) {
            let (root, start) = fixture();
            let first = AgentOperationService::open(&root)
                .unwrap()
                .execute_inner(&start, Some((point, io_failure)))
                .unwrap();
            assert_eq!(first["ok"], false);
            let restarted = AgentOperationService::open(&root).unwrap();
            let result = restarted.execute(&start).unwrap();
            assert_eq!(result["ok"], true);
            assert_eq!(restarted.execute(&start).unwrap(), result);
            let mut record = start.clone();
            record.request_id = "record.1".into();
            record.kind = "progress".into();
            record.binding_revision = Some(1);
            record.details = json!({"summary":"real fixture step"});
            assert_eq!(restarted.execute(&record).unwrap()["ok"], true);
            assert_eq!(restarted.execute(&record).unwrap()["ok"], true);
            let mut changed = record.clone();
            changed.details = json!({"summary":"different"});
            assert_eq!(
                restarted.execute(&changed).unwrap_err().code,
                "IDEMPOTENCY_PAYLOAD_CONFLICT"
            );
            let mut finish = record.clone();
            finish.kind = "session_finish".into();
            finish.request_id = "finish.1".into();
            finish.details = json!({"kind":"execution","request_source":"user_request","instruction":"fixture verification","status":"succeeded","summary":"verified"});
            assert_eq!(
                restarted
                    .execute_inner(&finish, Some((point, io_failure)))
                    .unwrap()["ok"],
                false
            );
            let recovered = AgentOperationService::open(&root).unwrap();
            let result = recovered.execute(&finish).unwrap();
            assert_eq!(result["ok"], true);
            assert_eq!(recovered.execute(&finish).unwrap(), result);
            let events = recovered
                .store
                .load_task_events("project.test", "task.test")
                .unwrap();
            assert_eq!(events.len(), 5); // bind/open/progress/result/close, no duplicates
            assert_eq!(
                recovered
                    .app
                    .task_lifecycle("project.test", "task.test")
                    .unwrap()
                    .state,
                "planned"
            );
            assert_eq!(
                recovered
                    .status("task.foreign", "session.test", "finish.1")
                    .unwrap_err()
                    .code,
                "V3_SCOPE_MISMATCH"
            );
            fs::remove_dir_all(root).unwrap();
        }
    }
}

use super::application::canonical_criterion_id;
use super::lifecycle::{fold_task, CriterionState, TaskLifecycleProjection};
use super::orchestration::fold_task as fold_orchestration;
use super::project_intelligence::{
    AnalyzerFinding, GitState, NodeKind, ProjectIndexService, ProjectModelSnapshot, ProjectPage,
};
use super::{EvidenceGrade, V3Error, V3ErrorCategory, V3EventEnvelope, V3EventStore};
use crate::process_util::silent_command;
use crate::vibehub::current::resolve_current_task;
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const MODEL_VERSION: &str = "v3-core-1";
const WORKSPACE_IGNORED_DIRS: &[&str] = &[
    ".git",
    ".vibehub",
    ".next",
    ".turbo",
    ".vite",
    ".cache",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "target",
    "vendor",
    "__pycache__",
    ".venv",
    "venv",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct V3ViewBundle {
    pub project_overview: Value,
    pub project_structure: Value,
    pub agent_results: Value,
    pub task_timeline: Value,
    pub plan_graph: Value,
    pub node_brief: Value,
}

#[derive(Debug, Clone)]
pub struct V3ViewRepository {
    root: PathBuf,
    store: V3EventStore,
}

#[derive(Debug, Clone, Deserialize)]
struct TaskDocument {
    task_id: String,
    title: String,
    intent: String,
    phase: String,
    phase_status: String,
    #[serde(default)]
    acceptance_criteria: Vec<String>,
    #[serde(default)]
    dependencies: Vec<String>,
}

#[derive(Debug, Clone)]
struct TaskReadResult {
    tasks: Vec<TaskDocument>,
    warnings: Vec<Value>,
}

impl V3ViewRepository {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, V3Error> {
        let store = V3EventStore::open(&root)?;
        let root = root
            .as_ref()
            .canonicalize()
            .map_err(internal("V3_ROOT_NOT_FOUND"))?;
        Ok(Self { root, store })
    }

    pub fn current_task_id(&self) -> Result<String, V3Error> {
        let pointer_path = self.root.join(".vibehub/tasks/current");
        let pointed_task_id = if pointer_path.is_dir() {
            let content = fs::read_to_string(pointer_path.join("task.yaml"))
                .map_err(internal("V3_CURRENT_TASK_READ_FAILED"))?;
            let task: TaskDocument = serde_yaml::from_str(&content).map_err(|error| {
                V3Error::new(
                    "V3_CURRENT_TASK_INVALID",
                    V3ErrorCategory::CorruptLog,
                    false,
                    error.to_string(),
                )
            })?;
            Ok(task.task_id)
        } else {
            resolve_current_task(&self.root)
                .map(|pointer| pointer.task_id)
                .map_err(|error| {
                    V3Error::new(
                        "V3_CURRENT_TASK_INVALID",
                        V3ErrorCategory::CorruptLog,
                        false,
                        error.to_string(),
                    )
                })
        };
        match pointed_task_id.and_then(|task_id| self.effective_current_task_id(task_id)) {
            Ok(task_id) => Ok(task_id),
            Err(error) if current_task_fallback_allowed(&error) => {
                self.fallback_current_task_id().or(Err(error))
            }
            Err(error) => Err(error),
        }
    }

    fn effective_current_task_id(&self, pointed_task_id: String) -> Result<String, V3Error> {
        let events = self.store.load_project(&self.project_id())?;
        let tasks = self.read_tasks()?.tasks;
        let Some(pointed_task) = tasks.iter().find(|task| task.task_id == pointed_task_id) else {
            return self
                .select_fallback_task_id(&tasks, &events)
                .ok_or_else(|| {
                    V3Error::new(
                        "V3_CURRENT_TASK_INVALID",
                        V3ErrorCategory::CorruptLog,
                        false,
                        "current task is missing or has invalid V3 metadata",
                    )
                });
        };
        let pointed_lifecycle = fold_task(&pointed_task_id, &events);
        let pointed_has_lifecycle = events
            .iter()
            .any(|event| event.aggregate_id == pointed_task_id);
        let pointed_state = if pointed_has_lifecycle {
            projected_task_state(&pointed_lifecycle)
        } else {
            task_state(&pointed_task)
        };
        if !is_terminal_task_state(pointed_state) {
            return Ok(pointed_task_id);
        }

        Ok(self
            .select_fallback_task_id(&tasks, &events)
            .unwrap_or(pointed_task_id))
    }

    fn fallback_current_task_id(&self) -> Result<String, V3Error> {
        let events = self.store.load_project(&self.project_id())?;
        let tasks = self.read_tasks()?.tasks;
        self.select_fallback_task_id(&tasks, &events)
            .ok_or_else(|| {
                V3Error::new(
                    "V3_CURRENT_TASK_INVALID",
                    V3ErrorCategory::CorruptLog,
                    false,
                    "current task is invalid and no valid V3 task is available",
                )
            })
    }

    fn select_fallback_task_id(
        &self,
        tasks: &[TaskDocument],
        events: &[V3EventEnvelope],
    ) -> Option<String> {
        tasks
            .iter()
            .find(|task| {
                let lifecycle = fold_task(&task.task_id, events);
                let has_lifecycle = events
                    .iter()
                    .any(|event| event.aggregate_id == task.task_id);
                let state = if has_lifecycle {
                    projected_task_state(&lifecycle)
                } else {
                    task_state(task)
                };
                !is_terminal_task_state(state)
            })
            .or_else(|| tasks.first())
            .map(|task| task.task_id.clone())
    }

    pub fn project_id(&self) -> String {
        let name = self
            .root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("project")
            .to_ascii_lowercase()
            .replace(|character: char| !character.is_ascii_alphanumeric(), "-");
        format!("project.{name}")
    }

    pub fn load_bundle(&self, task_id: &str) -> Result<V3ViewBundle, V3Error> {
        validate_id("task_id", task_id)?;
        let task = self.read_task(task_id)?;
        let project_id = self.project_id();
        let events = self.store.load_project(&project_id)?;
        let lifecycle = fold_task(task_id, &events);
        let orchestration = fold_orchestration(task_id, &events);
        let has_lifecycle = events.iter().any(|event| event.aggregate_id == task_id);
        let generated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let current_criteria = criteria(&task, &events, &lifecycle, &generated_at);
        let current_blocker_details =
            blocker_details(&task, &events, &lifecycle, None, &generated_at);
        let evidence_refs = evidence_refs(&task, &events, &generated_at);
        let native_root = native_path(&self.root);
        let sessions = sessions_for_task(&events, task_id);
        let opened_sessions = sessions.iter().filter(|session| session.1).count();
        let closed_sessions = sessions.iter().filter(|session| session.2).count();
        let explicit_session_gaps = lifecycle
            .sessions
            .values()
            .filter(|session| session.state == "gapped")
            .count();
        let current_task_state = if has_lifecycle {
            projected_task_state(&lifecycle)
        } else {
            task_state(&task)
        };
        let task_read = self.read_tasks()?;
        let task_metadata_warnings = task_read.warnings;
        let project_tasks = task_read.tasks;
        let active_tasks: Vec<Value> = project_tasks
            .iter()
            .filter_map(|project_task| {
                let task_lifecycle = fold_task(&project_task.task_id, &events);
                let task_has_lifecycle = events
                    .iter()
                    .any(|event| event.aggregate_id == project_task.task_id);
                let state = if task_has_lifecycle {
                    projected_task_state(&task_lifecycle)
                } else {
                    task_state(project_task)
                };
                if is_terminal_task_state(state) {
                    return None;
                }
                let task_sessions = sessions_for_task(&events, &project_task.task_id);
                let task_opened_sessions = task_sessions.iter().filter(|session| session.1).count();
                let task_closed_sessions = task_sessions.iter().filter(|session| session.2).count();
                let task_criteria = criteria(project_task, &events, &task_lifecycle, &generated_at);
                let task_blocker_details =
                    blocker_details(project_task, &events, &task_lifecycle, None, &generated_at);
                Some(json!({
                    "task_id": project_task.task_id,
                    "title": project_task.title,
                    "state": state,
                    "risk_level": risk_level(&events, &project_task.task_id),
                    "criteria": task_criteria,
                    "blocker_details": task_blocker_details,
                    "active_sessions": task_opened_sessions.saturating_sub(task_closed_sessions)
                }))
            })
            .collect();
        let archived_tasks: Vec<Value> = project_tasks
            .iter()
            .filter_map(|project_task| {
                let task_lifecycle = fold_task(&project_task.task_id, &events);
                let task_has_lifecycle = events
                    .iter()
                    .any(|event| event.aggregate_id == project_task.task_id);
                let state = if task_has_lifecycle {
                    projected_task_state(&task_lifecycle)
                } else {
                    task_state(project_task)
                };
                is_terminal_task_state(state).then(|| {
                    archived_task_summary(project_task, &events, &task_lifecycle, &generated_at)
                })
            })
            .collect();
        let effective_current_task_id = self.current_task_id().ok();
        let workspace = resolve_workspace(&self.root, task_id, &events);
        let index = ProjectIndexService::open(&workspace.root).and_then(|service| {
            let snapshot = service.current_snapshot()?;
            let page = service.first_page()?;
            Ok((page, snapshot))
        });
        let (index_page, index_snapshot) = match index {
            Ok((page, snapshot)) => (Some(page), Some(snapshot)),
            Err(_) => (None, None),
        };
        let indexed_files = index_snapshot
            .as_ref()
            .map(|snapshot| snapshot.indexed_files)
            .unwrap_or(0);
        let project_structure = project_structure_view(
            &workspace.root,
            &project_id,
            &generated_at,
            &evidence_refs,
            index_page,
            index_snapshot.as_ref(),
            &workspace,
        );
        let architecture_nodes = project_structure["architecture_nodes"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let architecture_edges = project_structure["architecture_edges"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let architecture_modules = architecture_nodes
            .iter()
            .filter(|node| node["kind"] != "workspace")
            .count();
        let declared_docs = architecture_nodes
            .iter()
            .filter(|node| node["source_kind"] == "documentation")
            .count();
        let architecture_claims = architecture_nodes
            .iter()
            .filter(|node| node["kind"] != "workspace")
            .chain(architecture_edges.iter())
            .filter_map(|claim| claim["confidence"].as_f64())
            .collect::<Vec<_>>();
        let architecture_confidence = if architecture_claims.is_empty() {
            0.0
        } else {
            architecture_claims.iter().sum::<f64>() / architecture_claims.len() as f64
        };
        let architecture_evidence_refs = architecture_view_evidence_refs(&project_structure);
        let architecture_warnings = project_structure["warnings"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let architecture_errors = project_structure["errors"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let mut overview_warnings = task_metadata_warnings.clone();
        overview_warnings.extend(architecture_warnings.clone());
        let index_state = project_structure["index_state"].as_str().unwrap_or("error");
        let model_state = match index_state {
            "ready" => "ready",
            "indexing" => "rebuilding",
            "uninitialized" => "uninitialized",
            _ => "error",
        };
        let structure_freshness = project_structure["freshness"]
            .as_str()
            .unwrap_or("unavailable");
        let structure_completeness = project_structure["completeness"]
            .as_str()
            .unwrap_or("unknown");
        let overview_completeness = if task_metadata_warnings.is_empty() {
            structure_completeness
        } else {
            "partial"
        };
        let structure_model_version = project_structure["model_version"]
            .as_str()
            .unwrap_or(MODEL_VERSION);

        let project_overview = json!({
            "schema_version": "1.0", "project_id": project_id, "name": self.root.file_name().and_then(|v| v.to_str()).unwrap_or("Project"),
            "root": native_root, "generated_at": generated_at, "model_version": MODEL_VERSION,
            "freshness": structure_freshness, "completeness": overview_completeness,
            "repository": {"state": if self.root.join(".git").exists() {"available"} else {"not_repository"}, "branch": Value::Null, "head": Value::Null, "dirty": Value::Null, "worktree_count": 1},
            "model": {"state": model_state, "last_evidence_at": generated_at, "generator_version": structure_model_version, "indexed_files": indexed_files},
            "architecture": {"declared_docs": declared_docs, "modules": architecture_modules, "relationships": architecture_edges.len(), "confidence": architecture_confidence, "evidence_refs": architecture_evidence_refs},
            "current_task_id": effective_current_task_id,
            "active_tasks": active_tasks,
            "archived_tasks": archived_tasks,
            "protocol_coverage": {"state": protocol_coverage(opened_sessions, closed_sessions, explicit_session_gaps), "opened_sessions": opened_sessions, "closed_sessions": closed_sessions, "gaps": explicit_session_gaps},
            "evidence_refs": evidence_refs, "warnings": overview_warnings, "errors": architecture_errors
        });
        let agent_results =
            agent_results_view(&project_id, task_id, &generated_at, &events, &evidence_refs);

        let lanes: Vec<Value> = lifecycle
            .sessions
            .values()
            .map(|session| {
                json!({
                    "lane_id": session.session_id, "kind": "session", "label": session.session_id,
                    "state": match session.state.as_str() {
                        "active" => "active", "closed" => "closed", "gapped" => "gapped",
                        "repaired" => "repaired", _ => "idle"
                    }
                })
            })
            .collect();
        let timeline_events: Vec<Value> = events
            .iter()
            .filter(|event| event.task_id.0 == task_id)
            .map(timeline_event)
            .collect();
        let task_timeline = json!({
            "schema_version": "1.0", "project_id": project_id, "task_id": task.task_id, "title": task.title, "state": current_task_state,
            "generated_at": generated_at, "model_version": MODEL_VERSION, "freshness": "fresh", "completeness": "complete",
            "criteria": current_criteria, "blocker_details": current_blocker_details.clone(), "completion": completion_view(&lifecycle), "lanes": lanes, "events": timeline_events, "window": page(events.len()),
            "evidence_refs": evidence_refs, "warnings": task_metadata_warnings.clone(), "errors": []
        });

        let graph_nodes: Vec<Value> = lifecycle.nodes.values().map(|node| {
                let session_ids = lifecycle.sessions.values()
                    .filter(|session| session.node_id.as_deref() == Some(node.node_id.as_str()))
                    .map(|session| session.session_id.clone())
                    .collect::<Vec<_>>();
                let node_blocker_details = blocker_details(
                    &task,
                    &events,
                    &lifecycle,
                    Some(node.node_id.as_str()),
                    &generated_at,
                );
                json!({
                    "node_id": node.node_id, "title": node.title, "goal": node.goal, "state": view_node_state(&node.state),
                    "readiness": if node.state == "blocked" || node.state == "failed" {"blocked"} else if node.dependencies.iter().all(|dependency| lifecycle.nodes.get(dependency).is_some_and(|value| value.state == "completed")) {"ready"} else {"blocked"},
                    "block_reasons": if node.state == "blocked" {vec!["lifecycle.blocked"]} else {Vec::<&str>::new()},
                    "blocker_details": node_blocker_details,
                    "scope": node.scope, "criterion_ids": Vec::<String>::new(), "session_ids": session_ids
                })
            }).collect();
        let scheduling_edges: Vec<Value> = lifecycle.nodes.values().flat_map(|node| node.dependencies.iter().map(|dependency| json!({
            "edge_id": format!("schedule.{}.{}", dependency, node.node_id), "from_node_id": dependency, "to_node_id": node.node_id, "kind": "depends_on"
        }))).collect();
        let trace_relations: Vec<Value> = lifecycle.findings.values().flat_map(|finding| finding.attempt_ids.iter().map(|attempt_id| json!({
            "relation_id": format!("trace.{}.{}", attempt_id, finding.finding_id), "from_id": attempt_id, "to_id": finding.finding_id, "kind": "addresses", "evidence_refs": []
        }))).collect();
        let node_id = graph_nodes
            .first()
            .and_then(|node| node.get("node_id"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        let mut plan_warnings = task_metadata_warnings.clone();
        if lifecycle.nodes.is_empty() {
            plan_warnings.push(json!({
                "code": "V3_PLAN_NOT_RECORDED",
                "severity": "warning",
                "message_key": "v3.warning.plan_not_recorded",
                "details": {},
                "evidence_refs": evidence_refs
            }));
        }
        let planned_worktrees = orchestration.worktrees.len();
        let observed_worktrees = orchestration
            .worktrees
            .values()
            .filter(|worktree| worktree.event_ids.len() > 1)
            .count();
        let plan_graph = json!({
            "schema_version": "1.0", "project_id": project_id, "task_id": task.task_id, "plan_version": lifecycle.version,
            "generated_at": generated_at, "model_version": MODEL_VERSION, "freshness": "fresh", "completeness": if lifecycle.nodes.is_empty() {"unknown"} else {"partial"}, "graph_state": "valid",
            "nodes": graph_nodes,
            "scheduling_edges": scheduling_edges, "trace_relations": trace_relations,
            "execution": {"planned_sessions": 0, "observed_sessions": sessions.len(), "planned_worktrees": planned_worktrees, "observed_worktrees": observed_worktrees},
            "evidence_refs": evidence_refs, "warnings": plan_warnings, "errors": []
        });

        let mut node_warnings = task_metadata_warnings;
        node_warnings.extend(architecture_warnings.clone());
        let node_brief = json!({
            "schema_version": "1.0", "project_id": project_id, "task_id": task.task_id, "node_id": node_id,
            "generated_at": generated_at, "model_version": MODEL_VERSION, "freshness": structure_freshness, "completeness": structure_completeness,
            "goal": lifecycle.nodes.get(&node_id).map(|node| node.goal.as_str()).unwrap_or(&task.intent),
            "scope": lifecycle.nodes.get(&node_id).map(|node| node.scope.clone()).unwrap_or_default(), "non_scope": ["M5-M6 milestone-owned capabilities"],
            "dependencies": task.dependencies.iter().map(|dependency| normalize_dependency_id(dependency)).collect::<Vec<_>>(),
            "accepted_decisions": ["JSON Schema 2020-12 remains the wire source of truth", "MCP and CLI share V3ApplicationService"],
            "research_summary": [], "criteria": current_criteria, "files": [native_root],
            "validation_commands": ["npm run v3:contracts:check", "cargo test --workspace"], "state": lifecycle.nodes.get(&node_id).map(|node| view_node_state(&node.state)).unwrap_or_else(|| node_state(&task)), "next_intent": task.phase,
            "blocker_details": current_blocker_details,
            "budget": {"max_tokens": 8000, "estimated_tokens": 1000, "truncated_sections": []},
            "source_versions": {"view_model": MODEL_VERSION, "project_model": structure_model_version}, "protocol_coverage": protocol_coverage(opened_sessions, closed_sessions, explicit_session_gaps),
            "evidence_refs": evidence_refs, "warnings": node_warnings, "errors": architecture_errors
        });

        Ok(V3ViewBundle {
            project_overview,
            project_structure,
            agent_results,
            task_timeline,
            plan_graph,
            node_brief,
        })
    }

    pub fn workspace_root(&self, task_id: &str) -> Result<PathBuf, V3Error> {
        let events = self.store.load_project(&self.project_id())?;
        Ok(resolve_workspace(&self.root, task_id, &events).root)
    }

    pub fn load_project_structure_page(
        &self,
        task_id: &str,
        relative_dir: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<Value, V3Error> {
        let events = self.store.load_project(&self.project_id())?;
        let workspace = resolve_workspace(&self.root, task_id, &events);
        let service = ProjectIndexService::open(&workspace.root).map_err(|error| {
            V3Error::new(
                "V3_PROJECT_INDEX_QUERY_FAILED",
                V3ErrorCategory::Validation,
                true,
                error.to_string(),
            )
        })?;
        let snapshot = service.current_snapshot().map_err(|error| {
            V3Error::new(
                "V3_PROJECT_INDEX_QUERY_FAILED",
                V3ErrorCategory::Validation,
                true,
                error.to_string(),
            )
        })?;
        let page = service.page(relative_dir, cursor, limit).map_err(|error| {
            V3Error::new(
                "V3_PROJECT_INDEX_QUERY_FAILED",
                V3ErrorCategory::Validation,
                true,
                error.to_string(),
            )
        })?;
        let generated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        Ok(project_structure_view(
            &workspace.root,
            &self.project_id(),
            &generated_at,
            &[],
            Some(page),
            Some(&snapshot),
            &workspace,
        ))
    }

    pub fn search_project_structure(
        &self,
        task_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Value, V3Error> {
        let events = self.store.load_project(&self.project_id())?;
        let workspace = resolve_workspace(&self.root, task_id, &events);
        let service = ProjectIndexService::open(&workspace.root).map_err(|error| {
            V3Error::new(
                "V3_PROJECT_INDEX_SEARCH_FAILED",
                V3ErrorCategory::Validation,
                true,
                error.to_string(),
            )
        })?;
        let snapshot = service.current_snapshot().map_err(|error| {
            V3Error::new(
                "V3_PROJECT_INDEX_SEARCH_FAILED",
                V3ErrorCategory::Validation,
                true,
                error.to_string(),
            )
        })?;
        let page = service.search(query, limit).map_err(|error| {
            V3Error::new(
                "V3_PROJECT_INDEX_SEARCH_FAILED",
                V3ErrorCategory::Validation,
                true,
                error.to_string(),
            )
        })?;
        let generated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        Ok(project_structure_view(
            &workspace.root,
            &self.project_id(),
            &generated_at,
            &[],
            Some(page),
            Some(&snapshot),
            &workspace,
        ))
    }

    fn read_task(&self, task_id: &str) -> Result<TaskDocument, V3Error> {
        let path = self
            .root
            .join(".vibehub/tasks")
            .join(task_id)
            .join("task.yaml");
        let content = fs::read_to_string(&path).map_err(internal("V3_TASK_NOT_FOUND"))?;
        let task = serde_yaml::from_str(&content).map_err(|error| {
            V3Error::new(
                "V3_TASK_INVALID",
                V3ErrorCategory::CorruptLog,
                false,
                format!("{}: {error}", path.display()),
            )
        })?;
        validate_task_identity(&task, task_id, &path)?;
        Ok(task)
    }

    fn read_tasks(&self) -> Result<TaskReadResult, V3Error> {
        let tasks_root = self.root.join(".vibehub/tasks");
        let mut task_paths = fs::read_dir(&tasks_root)
            .map_err(internal("V3_TASKS_READ_FAILED"))?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name() != "current")
            .filter_map(|entry| {
                entry
                    .file_type()
                    .ok()
                    .filter(|file_type| file_type.is_dir() && !file_type.is_symlink())
                    .map(|_| entry.path().join("task.yaml"))
            })
            .collect::<Vec<_>>();
        task_paths.sort();
        let mut tasks = Vec::new();
        let mut warnings = Vec::new();
        for path in task_paths {
            let expected_task_id = path
                .parent()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            match self.read_task_document(&path, expected_task_id) {
                Ok(task) => tasks.push(task),
                Err(error) => warnings.push(task_metadata_warning(&self.root, &path, &error)),
            }
        }
        Ok(TaskReadResult { tasks, warnings })
    }

    fn read_task_document(
        &self,
        path: &Path,
        expected_task_id: &str,
    ) -> Result<TaskDocument, V3Error> {
        let content = fs::read_to_string(path).map_err(internal("V3_TASK_READ_FAILED"))?;
        let task = serde_yaml::from_str(&content).map_err(|error| {
            V3Error::new(
                "V3_TASK_INVALID",
                V3ErrorCategory::CorruptLog,
                false,
                format!("{}: {error}", path.display()),
            )
        })?;
        validate_task_identity(&task, expected_task_id, path)?;
        Ok(task)
    }
}

fn current_task_fallback_allowed(error: &V3Error) -> bool {
    matches!(
        error.code.as_str(),
        "V3_CURRENT_TASK_INVALID"
            | "V3_CURRENT_TASK_READ_FAILED"
            | "V3_TASK_INVALID"
            | "V3_TASK_NOT_FOUND"
            | "V3_TASK_READ_FAILED"
    )
}

fn validate_task_identity(
    task: &TaskDocument,
    expected_task_id: &str,
    path: &Path,
) -> Result<(), V3Error> {
    if task.task_id != expected_task_id {
        return Err(V3Error::new(
            "V3_TASK_INVALID",
            V3ErrorCategory::CorruptLog,
            false,
            format!(
                "{}: task_id '{}' does not match task directory '{}'",
                path.display(),
                task.task_id,
                expected_task_id
            ),
        ));
    }
    Ok(())
}

fn task_metadata_warning(root: &Path, path: &Path, error: &V3Error) -> Value {
    let task_id = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .unwrap_or("unknown-task");
    let task_path = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    json!({
        "code": "V3_TASK_METADATA_INVALID",
        "severity": "warning",
        "message_key": "v3.warning.task_metadata_invalid",
        "details": {
            "task_id": task_id,
            "task_path": task_path,
            "reason_code": error.code,
            "reason": limited_metadata_error(&error.message),
            "recommended_action": "Review and preserve the task, then run `vibehub v3 <project> quarantine-task <task_id>`."
        },
        "evidence_refs": [{
            "evidence_id": format!("task-metadata:{task_id}"),
            "kind": "file",
            "grade": "hard_observed",
            "label_key": "v3.evidence.task_metadata",
            "locator": format!("file:{task_path}")
        }]
    })
}

fn limited_metadata_error(message: &str) -> String {
    const MAX_CHARS: usize = 512;
    if message.chars().count() <= MAX_CHARS {
        message.to_owned()
    } else {
        format!("{}…", message.chars().take(MAX_CHARS).collect::<String>())
    }
}

#[derive(Debug, Clone)]
struct WorkspaceSelection {
    root: PathBuf,
    source: &'static str,
    session_id: Option<String>,
    worktree_id: Option<String>,
    fallback_reason: Option<String>,
}

#[derive(Debug, Clone)]
struct ArchitectureModule {
    node_id: String,
    name: String,
    kind: &'static str,
    relative_path: String,
    analyzer: String,
    package_name: Option<String>,
    manifest_path: Option<String>,
    source_kind: &'static str,
    confidence: f64,
    file_count: usize,
    evidence_refs: Vec<Value>,
}

#[derive(Debug, Clone)]
struct ArchitectureRelation {
    edge_id: String,
    from_node_id: String,
    to_node_id: String,
    kind: &'static str,
    source_kind: &'static str,
    confidence: f64,
    evidence_refs: Vec<Value>,
}

#[derive(Debug)]
struct ArchitectureView {
    modules: Vec<ArchitectureModule>,
    relations: Vec<ArchitectureRelation>,
    unsupported_analyzers: Vec<String>,
    warnings: Vec<Value>,
    completeness: &'static str,
    model_version: String,
}

impl ArchitectureView {
    fn nodes(&self, root: &Path) -> Vec<Value> {
        self.modules
            .iter()
            .map(|module| {
                let module_path = if module.relative_path == "." {
                    root.to_path_buf()
                } else {
                    root.join(&module.relative_path)
                };
                json!({
                    "node_id": module.node_id,
                    "name": module.name,
                    "kind": module.kind,
                    "path": native_path(&module_path),
                    "file_count": module.file_count,
                    "source_kind": module.source_kind,
                    "confidence": module.confidence,
                    "generator_version": self.model_version,
                    "evidence_refs": module.evidence_refs
                })
            })
            .collect()
    }

    fn edges(&self) -> Vec<Value> {
        self.relations
            .iter()
            .map(|relation| {
                json!({
                    "edge_id": relation.edge_id,
                    "from_node_id": relation.from_node_id,
                    "to_node_id": relation.to_node_id,
                    "kind": relation.kind,
                    "source_kind": relation.source_kind,
                    "confidence": relation.confidence,
                    "generator_version": self.model_version,
                    "evidence_refs": relation.evidence_refs
                })
            })
            .collect()
    }

    fn module_id_for_path(&self, path: &str) -> Option<&str> {
        nearest_architecture_module(&self.modules, path, &file_analyzer(path))
            .map(|module| module.node_id.as_str())
    }
}

fn project_structure_view(
    root: &Path,
    project_id: &str,
    generated_at: &str,
    evidence_refs: &[Value],
    page_result: Option<ProjectPage>,
    index_snapshot: Option<&ProjectModelSnapshot>,
    workspace: &WorkspaceSelection,
) -> Value {
    let (Some(result), Some(index_snapshot)) = (page_result, index_snapshot) else {
        return json!({
            "schema_version": "1.0", "project_id": project_id, "generated_at": generated_at, "model_version": MODEL_VERSION,
            "freshness": "unavailable", "completeness": "unknown", "index_state": "error",
            "workspace": workspace_view(workspace),
            "nodes": [], "edges": [], "architecture_nodes": [], "architecture_edges": [], "unsupported_analyzers": ["cargo", "npm", "python", "imports"],
            "page": {"cursor": Value::Null, "next_cursor": Value::Null, "limit": 200, "returned": 0, "total_estimate": Value::Null, "truncated": false, "truncation_reason": "none", "model_version": MODEL_VERSION},
            "evidence_refs": evidence_refs, "warnings": [],
            "errors": [{"code": "PI_INDEX_UNAVAILABLE", "category": "internal", "recoverable": true, "message_key": "v3.error.project_index_unavailable", "details": {}, "evidence_refs": evidence_refs}]
        });
    };
    let model_version = index_snapshot.model_version.clone();
    let mut structure_evidence_refs = evidence_refs.to_vec();
    structure_evidence_refs.push(json!({
        "evidence_id": format!("project-model:{}", model_version),
        "kind": "file",
        "grade": "hard_observed",
        "label_key": "v3.evidence.project_model_index",
        "locator": format!("file:{}", super::project_intelligence::PROJECT_MODEL_INDEX_PATH),
        "captured_at": index_snapshot.generated_at
    }));
    let architecture = architecture_view(
        root,
        &index_snapshot.analyzer_findings,
        &model_version,
        &index_snapshot.generated_at,
        &structure_evidence_refs,
    );
    let nodes: Vec<Value> = result.snapshot.nodes.iter().map(|node| {
        let full_path = if node.relative_path == "." { root.to_path_buf() } else { root.join(&node.relative_path) };
        let module_id = if node.kind == NodeKind::Root {
            Some("architecture.workspace")
        } else {
            architecture.module_id_for_path(&node.relative_path)
        };
        json!({
            "node_id": node.node_id, "parent_id": node.parent_id, "name": node.name,
            "kind": match node.kind { NodeKind::Root => "root", NodeKind::Directory => "directory", NodeKind::File => "file" },
            "path": native_path(&full_path),
            "git_state": match node.git_state { GitState::Clean => "clean", GitState::Modified => "modified", GitState::Added => "added", GitState::Deleted => "deleted", GitState::Ignored => "ignored", GitState::Unknown => "unknown" },
            "module_id": module_id, "ide_target": if node.kind == NodeKind::File { Some(node.relative_path.clone()) } else { None }, "evidence_refs": structure_evidence_refs
        })
    }).collect();
    let edges: Vec<Value> = result.snapshot.nodes.iter().filter_map(|node| node.parent_id.as_ref().map(|parent| json!({
        "edge_id": format!("contains.{}.{}", parent, node.node_id), "from_node_id": parent, "to_node_id": node.node_id,
        "kind": "contains", "source_kind": "filesystem", "confidence": 1.0, "evidence_refs": structure_evidence_refs
    }))).collect();
    let mut index_warnings: Vec<Value> = result.snapshot.warnings.iter().map(|warning| json!({
        "code": warning.code, "severity": "warning", "message_key": "v3.warning.project_index_gap",
        "details": {"path": warning.path, "message": warning.message}, "evidence_refs": structure_evidence_refs
    })).collect();
    index_warnings.extend(architecture.warnings.iter().cloned());
    let returned = nodes.len().saturating_sub(1);
    let truncated = result.next_cursor.is_some();
    let architecture_nodes = architecture.nodes(root);
    let architecture_edges = architecture.edges();
    json!({
        "schema_version": "1.0", "project_id": project_id, "generated_at": index_snapshot.generated_at, "model_version": model_version,
        "freshness": "fresh", "completeness": architecture.completeness, "index_state": "ready",
        "workspace": workspace_view(workspace),
        "nodes": nodes, "edges": edges, "architecture_nodes": architecture_nodes, "architecture_edges": architecture_edges, "unsupported_analyzers": architecture.unsupported_analyzers,
        "page": {"cursor": result.cursor, "next_cursor": result.next_cursor, "limit": result.limit, "returned": returned, "total_estimate": result.total_estimate, "truncated": truncated, "truncation_reason": if truncated {"page_limit"} else {"none"}, "model_version": model_version},
        "evidence_refs": structure_evidence_refs, "warnings": index_warnings, "errors": []
    })
}

fn workspace_view(workspace: &WorkspaceSelection) -> Value {
    json!({
        "root": native_path(&workspace.root),
        "source": workspace.source,
        "session_id": workspace.session_id,
        "worktree_id": workspace.worktree_id,
        "fallback_reason": workspace.fallback_reason,
        "ignored_directories": WORKSPACE_IGNORED_DIRS
    })
}

fn resolve_workspace(
    project_root: &Path,
    task_id: &str,
    events: &[V3EventEnvelope],
) -> WorkspaceSelection {
    let active_sessions: std::collections::BTreeSet<String> = sessions_for_task(events, task_id)
        .into_iter()
        .filter(|(_, opened, closed)| *opened && !*closed)
        .map(|(session_id, _, _)| session_id)
        .collect();
    if active_sessions.is_empty() {
        return WorkspaceSelection {
            root: project_root.to_path_buf(),
            source: "project_root_fallback",
            session_id: None,
            worktree_id: None,
            fallback_reason: Some("no active session or worktree context was recorded".to_owned()),
        };
    }
    let candidate_sessions = |event: &&V3EventEnvelope| {
        event.task_id.0 == task_id
            && event
                .session_id
                .as_ref()
                .is_some_and(|session| active_sessions.contains(&session.0))
    };
    for event in events.iter().rev().filter(candidate_sessions) {
        if event.event_type.starts_with("worktree.") {
            if let Some(path) = event
                .payload
                .get("native_path")
                .and_then(Value::as_str)
                .and_then(accessible_directory)
            {
                return WorkspaceSelection {
                    root: path,
                    source: "session_worktree",
                    session_id: event.session_id.as_ref().map(|value| value.0.clone()),
                    worktree_id: event.worktree_id.as_ref().map(|value| value.0.clone()),
                    fallback_reason: None,
                };
            }
        }
    }
    for event in events.iter().rev().filter(candidate_sessions) {
        if event.event_type == "session.opened" {
            if let Some(path) = event
                .payload
                .get("working_directory")
                .and_then(Value::as_str)
                .and_then(accessible_directory)
            {
                return WorkspaceSelection {
                    root: path,
                    source: "session_working_directory",
                    session_id: event.session_id.as_ref().map(|value| value.0.clone()),
                    worktree_id: event.worktree_id.as_ref().map(|value| value.0.clone()),
                    fallback_reason: None,
                };
            }
        }
    }
    WorkspaceSelection {
        root: project_root.to_path_buf(),
        source: "project_root_fallback",
        session_id: active_sessions.iter().next().cloned(),
        worktree_id: None,
        fallback_reason: Some(
            "active session has no accessible working directory or worktree path".to_owned(),
        ),
    }
}

fn accessible_directory(value: &str) -> Option<PathBuf> {
    let path = PathBuf::from(value);
    path.is_dir().then(|| path.canonicalize().ok()).flatten()
}

fn architecture_view_evidence_refs(project_structure: &Value) -> Vec<Value> {
    let mut seen = BTreeSet::new();
    let mut output = Vec::new();
    let mut add = |reference: &Value| {
        if output.len() >= 32 {
            return;
        }
        if reference
            .get("evidence_id")
            .and_then(Value::as_str)
            .is_some_and(|id| seen.insert(id.to_owned()))
        {
            output.push(reference.clone());
        }
    };
    for reference in project_structure["evidence_refs"]
        .as_array()
        .into_iter()
        .flatten()
    {
        add(reference);
    }
    for claim in project_structure["architecture_nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .chain(
            project_structure["architecture_edges"]
                .as_array()
                .into_iter()
                .flatten(),
        )
    {
        for reference in claim["evidence_refs"].as_array().into_iter().flatten() {
            add(reference);
        }
    }
    output
}

fn architecture_view(
    root: &Path,
    findings: &[AnalyzerFinding],
    model_version: &str,
    generated_at: &str,
    fallback_evidence_refs: &[Value],
) -> ArchitectureView {
    let workspace_evidence = findings
        .iter()
        .filter(|finding| finding.kind == "workspace_manifest")
        .map(|finding| architecture_evidence(finding, generated_at))
        .collect::<Vec<_>>();
    let mut modules = vec![ArchitectureModule {
        node_id: "architecture.workspace".to_owned(),
        name: root
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Workspace")
            .to_owned(),
        kind: "workspace",
        relative_path: ".".to_owned(),
        analyzer: "workspace".to_owned(),
        package_name: None,
        manifest_path: None,
        source_kind: "manifest",
        confidence: 1.0,
        file_count: 0,
        evidence_refs: if workspace_evidence.is_empty() {
            fallback_evidence_refs.to_vec()
        } else {
            workspace_evidence
        },
    }];

    for finding in findings
        .iter()
        .filter(|finding| matches!(finding.kind.as_str(), "package_manifest" | "manifest"))
    {
        let relative_path = manifest_parent(&finding.path);
        let identity = format!("{}:{}", finding.analyzer, finding.path);
        modules.push(ArchitectureModule {
            node_id: architecture_stable_id("package", &identity),
            name: finding.label.clone(),
            kind: "package",
            relative_path,
            analyzer: finding.analyzer.clone(),
            package_name: Some(
                finding
                    .target
                    .clone()
                    .unwrap_or_else(|| finding.label.clone()),
            ),
            manifest_path: Some(finding.path.clone()),
            source_kind: "manifest",
            confidence: 1.0,
            file_count: 0,
            evidence_refs: vec![architecture_evidence(finding, generated_at)],
        });
    }

    disambiguate_package_names(&mut modules);
    add_source_modules(findings, generated_at, &mut modules);
    add_documentation_modules(findings, generated_at, &mut modules);
    if modules.len() == 1 {
        add_inferred_modules(root, fallback_evidence_refs, &mut modules);
    }
    populate_architecture_file_counts(root, &mut modules);
    modules.sort_by(|left, right| {
        (
            left.kind != "workspace",
            &left.relative_path,
            left.kind,
            &left.name,
        )
            .cmp(&(
                right.kind != "workspace",
                &right.relative_path,
                right.kind,
                &right.name,
            ))
    });

    let mut relations = BTreeMap::<(String, String, &'static str), ArchitectureRelation>::new();
    for module in modules.iter().filter(|module| module.kind == "package") {
        let evidence = findings
            .iter()
            .find(|finding| {
                finding.kind == "workspace_member"
                    && finding
                        .target_path
                        .as_deref()
                        .is_some_and(|target| same_relative_path(target, &module.relative_path))
            })
            .map(|finding| vec![architecture_evidence(finding, generated_at)])
            .unwrap_or_else(|| module.evidence_refs.clone());
        insert_architecture_relation(
            &mut relations,
            "architecture.workspace",
            &module.node_id,
            "contains",
            "manifest",
            1.0,
            evidence,
        );
    }
    for module in modules.iter().filter(|module| module.kind == "module") {
        let parent = nearest_package(&modules, &module.relative_path, &module.analyzer)
            .map(|parent| parent.node_id.as_str())
            .unwrap_or("architecture.workspace");
        insert_architecture_relation(
            &mut relations,
            parent,
            &module.node_id,
            "contains",
            module.source_kind,
            module.confidence,
            module.evidence_refs.clone(),
        );
    }

    for finding in findings
        .iter()
        .filter(|finding| finding.kind == "dependency")
    {
        let Some(source) = package_for_manifest(&modules, &finding.path) else {
            continue;
        };
        let target = finding
            .target_path
            .as_deref()
            .and_then(|path| package_for_path(&modules, path, &finding.analyzer))
            .or_else(|| {
                finding
                    .target
                    .as_deref()
                    .and_then(|name| package_for_name(&modules, name, &finding.analyzer))
            });
        if let Some(target) = target {
            insert_architecture_relation(
                &mut relations,
                &source.node_id,
                &target.node_id,
                "depends_on",
                "manifest",
                1.0,
                vec![architecture_evidence(finding, generated_at)],
            );
        }
    }

    for finding in findings
        .iter()
        .filter(|finding| finding.kind == "static_import")
    {
        let analyzer = analyzer_family(&finding.analyzer);
        let Some(source) = nearest_architecture_module(&modules, &finding.path, analyzer) else {
            continue;
        };
        let target_path = finding
            .target_path
            .clone()
            .or_else(|| resolve_import_path(&finding.path, finding.target.as_deref()));
        let target = target_path
            .as_deref()
            .and_then(|path| nearest_architecture_module(&modules, path, analyzer))
            .or_else(|| {
                finding.target.as_deref().and_then(|name| {
                    package_for_name(&modules, import_package_name(name), analyzer)
                })
            });
        if let Some(target) = target {
            insert_architecture_relation(
                &mut relations,
                &source.node_id,
                &target.node_id,
                "imports",
                "parser",
                0.9,
                vec![architecture_evidence(finding, generated_at)],
            );
        }
    }

    let mut warnings = findings
        .iter()
        .filter(|finding| finding.kind == "analyzer_error")
        .map(|finding| {
            json!({
                "code": "PI_ANALYZER_DEGRADED",
                "severity": "warning",
                "message_key": "v3.warning.analyzer_degraded",
                "details": {"analyzer": finding.analyzer, "path": finding.path, "message": finding.label},
                "evidence_refs": [architecture_evidence(finding, generated_at)]
            })
        })
        .collect::<Vec<_>>();
    let unsupported_analyzers = findings
        .iter()
        .filter(|finding| {
            finding.kind == "capability"
                && finding.label.to_ascii_lowercase().contains("unsupported")
        })
        .map(|finding| finding.analyzer.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if modules.len() == 1 {
        warnings.push(json!({
            "code": "PI_ARCHITECTURE_UNSUPPORTED",
            "severity": "warning",
            "message_key": "v3.warning.architecture_unsupported",
            "details": {"message": "No supported manifest, source module, or declared architecture document was observed"},
            "evidence_refs": fallback_evidence_refs
        }));
    }
    if !unsupported_analyzers.is_empty() {
        warnings.push(json!({
            "code": "PI_ANALYZER_UNSUPPORTED",
            "severity": "warning",
            "message_key": "v3.warning.analyzer_unsupported",
            "details": {"analyzers": unsupported_analyzers},
            "evidence_refs": fallback_evidence_refs
        }));
    }
    let completeness = if modules.len() == 1 {
        "unsupported"
    } else if warnings.is_empty() {
        "complete"
    } else {
        "partial"
    };
    ArchitectureView {
        modules,
        relations: relations.into_values().collect(),
        unsupported_analyzers,
        warnings,
        completeness,
        model_version: model_version.to_owned(),
    }
}

fn add_source_modules(
    findings: &[AnalyzerFinding],
    generated_at: &str,
    modules: &mut Vec<ArchitectureModule>,
) {
    let mut grouped = BTreeMap::<(String, String), Vec<&AnalyzerFinding>>::new();
    for finding in findings
        .iter()
        .filter(|finding| finding.kind == "static_import")
    {
        let analyzer = analyzer_family(&finding.analyzer).to_owned();
        let path = manifest_parent(&finding.path);
        if modules.iter().any(|module| {
            module.kind == "package"
                && module.analyzer == analyzer
                && same_relative_path(&module.relative_path, &path)
        }) {
            continue;
        }
        grouped.entry((analyzer, path)).or_default().push(finding);
    }
    for ((analyzer, path), evidence) in grouped {
        modules.push(ArchitectureModule {
            node_id: architecture_stable_id("module", &format!("{analyzer}:{path}")),
            name: path.clone(),
            kind: "module",
            relative_path: path,
            analyzer,
            package_name: None,
            manifest_path: None,
            source_kind: "parser",
            confidence: 0.9,
            file_count: 0,
            evidence_refs: unique_architecture_evidence(evidence, generated_at, 16),
        });
    }
}

fn add_documentation_modules(
    findings: &[AnalyzerFinding],
    generated_at: &str,
    modules: &mut Vec<ArchitectureModule>,
) {
    let mut grouped = BTreeMap::<String, Vec<&AnalyzerFinding>>::new();
    for finding in findings
        .iter()
        .filter(|finding| matches!(finding.kind.as_str(), "readme" | "architecture_document"))
    {
        grouped
            .entry(manifest_parent(&finding.path))
            .or_default()
            .push(finding);
    }
    for (path, evidence) in grouped {
        modules.push(ArchitectureModule {
            node_id: architecture_stable_id("documentation", &path),
            name: if path == "." {
                "Declared architecture".to_owned()
            } else {
                path.clone()
            },
            kind: "module",
            relative_path: path,
            analyzer: "documentation".to_owned(),
            package_name: None,
            manifest_path: None,
            source_kind: "documentation",
            confidence: 1.0,
            file_count: evidence.len(),
            evidence_refs: unique_architecture_evidence(evidence, generated_at, 16),
        });
    }
}

fn add_inferred_modules(
    root: &Path,
    evidence_refs: &[Value],
    modules: &mut Vec<ArchitectureModule>,
) {
    for candidate in ["src", "src-tauri", "crates", "packages", "apps"] {
        if root.join(candidate).is_dir() {
            modules.push(ArchitectureModule {
                node_id: architecture_stable_id("module", candidate),
                name: candidate.to_owned(),
                kind: "module",
                relative_path: candidate.to_owned(),
                analyzer: "inference".to_owned(),
                package_name: None,
                manifest_path: None,
                source_kind: "inference",
                confidence: 0.6,
                file_count: 0,
                evidence_refs: evidence_refs.to_vec(),
            });
        }
    }
}

fn insert_architecture_relation(
    relations: &mut BTreeMap<(String, String, &'static str), ArchitectureRelation>,
    from: &str,
    to: &str,
    kind: &'static str,
    source_kind: &'static str,
    confidence: f64,
    evidence_refs: Vec<Value>,
) {
    if from == to {
        return;
    }
    let key = (from.to_owned(), to.to_owned(), kind);
    if let Some(existing) = relations.get_mut(&key) {
        merge_evidence(&mut existing.evidence_refs, evidence_refs, 32);
        existing.confidence = existing.confidence.max(confidence);
        return;
    }
    relations.insert(
        key,
        ArchitectureRelation {
            edge_id: architecture_stable_id("edge", &format!("{kind}:{from}:{to}")),
            from_node_id: from.to_owned(),
            to_node_id: to.to_owned(),
            kind,
            source_kind,
            confidence,
            evidence_refs,
        },
    );
}

fn architecture_stable_id(kind: &str, identity: &str) -> String {
    let digest = format!("{:x}", Sha256::digest(identity.as_bytes()));
    format!("architecture.{kind}.{}", &digest[..16])
}

fn architecture_evidence(finding: &AnalyzerFinding, generated_at: &str) -> Value {
    let identity = format!(
        "{}:{}:{}:{}",
        finding.analyzer, finding.kind, finding.path, finding.label
    );
    json!({
        "evidence_id": architecture_stable_id("evidence", &identity),
        "kind": "file",
        "grade": "hard_observed",
        "label_key": format!("v3.evidence.architecture.{}", finding.kind),
        "locator": format!("file:{}", finding.path),
        "captured_at": generated_at,
        "excerpt": finding.label
    })
}

fn unique_architecture_evidence(
    findings: Vec<&AnalyzerFinding>,
    generated_at: &str,
    limit: usize,
) -> Vec<Value> {
    let mut seen = BTreeSet::new();
    findings
        .into_iter()
        .filter(|finding| seen.insert((finding.path.clone(), finding.label.clone())))
        .take(limit)
        .map(|finding| architecture_evidence(finding, generated_at))
        .collect()
}

fn merge_evidence(target: &mut Vec<Value>, incoming: Vec<Value>, limit: usize) {
    let mut seen = target
        .iter()
        .filter_map(|reference| reference.get("evidence_id").and_then(Value::as_str))
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    for reference in incoming {
        if target.len() >= limit {
            break;
        }
        if reference
            .get("evidence_id")
            .and_then(Value::as_str)
            .is_some_and(|id| seen.insert(id.to_owned()))
        {
            target.push(reference);
        }
    }
}

fn disambiguate_package_names(modules: &mut [ArchitectureModule]) {
    let counts = modules
        .iter()
        .filter_map(|module| module.package_name.as_deref())
        .fold(BTreeMap::<String, usize>::new(), |mut counts, name| {
            *counts.entry(name.to_owned()).or_default() += 1;
            counts
        });
    for module in modules.iter_mut().filter(|module| module.kind == "package") {
        let Some(package_name) = module.package_name.as_deref() else {
            continue;
        };
        if counts.get(package_name).copied().unwrap_or_default() > 1 {
            let location = if module.relative_path == "." {
                "root"
            } else {
                module.relative_path.as_str()
            };
            module.name = format!("{package_name} ({} · {location})", module.analyzer);
        }
    }
}

fn populate_architecture_file_counts(root: &Path, modules: &mut [ArchitectureModule]) {
    let files = visible_project_files(root);
    let package_snapshot = modules.to_vec();
    for module in modules {
        module.file_count = if module.kind == "workspace" {
            files.len()
        } else if module.kind == "package" {
            files
                .iter()
                .filter(|file| {
                    package_for_path(&package_snapshot, file, &file_analyzer(file))
                        .is_some_and(|owner| owner.node_id == module.node_id)
                })
                .count()
        } else {
            files
                .iter()
                .filter(|file| {
                    path_is_within(file, &module.relative_path)
                        && (module.analyzer == "inference"
                            || analyzer_matches(&module.analyzer, &file_analyzer(file)))
                })
                .count()
        };
    }
}

fn visible_project_files(root: &Path) -> Vec<String> {
    let output = silent_command("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--cached", "--others", "--exclude-standard"])
        .output();
    match output {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|path| !path.is_empty())
            .map(|path| path.replace('\\', "/"))
            .filter(|path| {
                !Path::new(path).components().any(|component| {
                    matches!(component, std::path::Component::Normal(value) if WORKSPACE_IGNORED_DIRS.contains(&value.to_string_lossy().as_ref()))
                })
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn nearest_architecture_module<'a>(
    modules: &'a [ArchitectureModule],
    path: &str,
    analyzer: &str,
) -> Option<&'a ArchitectureModule> {
    modules
        .iter()
        .filter(|module| {
            module.kind != "workspace"
                && path_is_within(path, &module.relative_path)
                && analyzer_matches(&module.analyzer, analyzer)
        })
        .max_by_key(|module| {
            (
                module.relative_path.matches('/').count(),
                usize::from(module.kind == "module"),
            )
        })
        .or_else(|| package_for_path(modules, path, analyzer))
}

fn nearest_package<'a>(
    modules: &'a [ArchitectureModule],
    path: &str,
    analyzer: &str,
) -> Option<&'a ArchitectureModule> {
    package_for_path(modules, path, analyzer)
}

fn package_for_path<'a>(
    modules: &'a [ArchitectureModule],
    path: &str,
    analyzer: &str,
) -> Option<&'a ArchitectureModule> {
    modules
        .iter()
        .filter(|module| {
            module.kind == "package"
                && path_is_within(path, &module.relative_path)
                && analyzer_matches(&module.analyzer, analyzer)
        })
        .max_by_key(|module| module.relative_path.matches('/').count())
}

fn package_for_manifest<'a>(
    modules: &'a [ArchitectureModule],
    manifest: &str,
) -> Option<&'a ArchitectureModule> {
    modules
        .iter()
        .find(|module| module.manifest_path.as_deref() == Some(manifest))
}

fn package_for_name<'a>(
    modules: &'a [ArchitectureModule],
    name: &str,
    analyzer: &str,
) -> Option<&'a ArchitectureModule> {
    let name = normalized_package_name(name);
    modules.iter().find(|module| {
        module.kind == "package"
            && analyzer_matches(&module.analyzer, analyzer)
            && module
                .package_name
                .as_deref()
                .is_some_and(|candidate| normalized_package_name(candidate) == name)
    })
}

fn analyzer_family(analyzer: &str) -> &str {
    match analyzer {
        "rust_imports" => "cargo",
        "javascript_imports" => "npm",
        "python_imports" => "python",
        other => other,
    }
}

fn analyzer_matches(left: &str, right: &str) -> bool {
    analyzer_family(left) == analyzer_family(right) || left == "inference" || right == "inference"
}

fn file_analyzer(path: &str) -> String {
    let name = Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if name == "Cargo.toml" {
        return "cargo".to_owned();
    }
    if matches!(name, "package.json" | "tsconfig.json" | "vite.config.ts") {
        return "npm".to_owned();
    }
    match Path::new(path).extension().and_then(|value| value.to_str()) {
        Some("rs") => "cargo",
        Some("ts" | "tsx" | "js" | "jsx" | "json" | "css" | "html") => "npm",
        Some("py") => "python",
        Some("md" | "mdx") => "documentation",
        _ => "inference",
    }
    .to_owned()
}

fn manifest_parent(path: &str) -> String {
    Path::new(path)
        .parent()
        .and_then(Path::to_str)
        .filter(|path| !path.is_empty())
        .unwrap_or(".")
        .replace('\\', "/")
}

fn path_is_within(path: &str, root: &str) -> bool {
    root == "." || path == root || path.starts_with(&format!("{root}/"))
}

fn same_relative_path(left: &str, right: &str) -> bool {
    left.trim_end_matches('/').trim_start_matches("./")
        == right.trim_end_matches('/').trim_start_matches("./")
}

fn normalized_package_name(name: &str) -> String {
    name.trim()
        .trim_matches(['\'', '"'])
        .replace('-', "_")
        .to_ascii_lowercase()
}

fn import_package_name(target: &str) -> &str {
    if target.starts_with('@') {
        target
            .splitn(3, '/')
            .take(2)
            .last()
            .and_then(|name| target.find(name).map(|index| &target[..index + name.len()]))
            .unwrap_or(target)
    } else {
        target.split(['/', ':', '.']).next().unwrap_or(target)
    }
}

fn resolve_import_path(source: &str, target: Option<&str>) -> Option<String> {
    let target = target?;
    let candidate = if let Some(target) = target.strip_prefix("@/") {
        PathBuf::from("src").join(target)
    } else if target.starts_with('.') {
        Path::new(source)
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(target)
    } else {
        return None;
    };
    normalize_relative_path(&candidate)
}

fn normalize_relative_path(path: &Path) -> Option<String> {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::Normal(value) => {
                components.push(value.to_string_lossy().into_owned())
            }
            std::path::Component::ParentDir => {
                components.pop()?;
            }
            _ => return None,
        }
    }
    Some(if components.is_empty() {
        ".".to_owned()
    } else {
        components.join("/")
    })
}

fn normalize_agent_evidence_ref(value: &Value, event: &V3EventEnvelope) -> Option<Value> {
    if let Some(locator) = value.as_str() {
        let kind = if locator.starts_with("file:") {
            "file"
        } else if locator.starts_with("evt.") || locator.starts_with("vibehub://v3/events/") {
            "event"
        } else {
            "external"
        };
        return Some(json!({
            "evidence_id": locator,
            "kind": kind,
            "grade": event.evidence_grade,
            "label_key": "v3.evidence.agent_result",
            "locator": locator,
            "captured_at": event.recorded_at
        }));
    }
    let object = value.as_object()?;
    let locator = object.get("locator").and_then(Value::as_str)?;
    let evidence_id = object
        .get("evidence_id")
        .and_then(Value::as_str)
        .unwrap_or(locator);
    let kind = object
        .get("kind")
        .and_then(Value::as_str)
        .filter(|kind| {
            [
                "event", "file", "git", "command", "test", "user", "external",
            ]
            .contains(kind)
        })
        .unwrap_or("external");
    let event_grade = match event.evidence_grade {
        EvidenceGrade::HardObserved => "hard_observed",
        EvidenceGrade::AgentReported => "agent_reported",
        EvidenceGrade::Inferred => "inferred",
        EvidenceGrade::UserConfirmed => "user_confirmed",
    };
    let grade = object
        .get("grade")
        .and_then(Value::as_str)
        .filter(|grade| {
            [
                "hard_observed",
                "agent_reported",
                "inferred",
                "user_confirmed",
            ]
            .contains(grade)
        })
        .unwrap_or(event_grade);
    Some(json!({
        "evidence_id": evidence_id,
        "kind": kind,
        "grade": grade,
        "label_key": object.get("label_key").and_then(Value::as_str).unwrap_or("v3.evidence.agent_result"),
        "locator": locator,
        "captured_at": object.get("captured_at").cloned().unwrap_or_else(|| Value::String(event.recorded_at.clone())),
        "excerpt": object.get("excerpt").cloned().unwrap_or(Value::Null)
    }))
}

fn normalize_agent_evaluation(value: Option<&Value>) -> (Value, bool) {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return (Value::Null, false);
    };
    let Some(object) = value.as_object() else {
        return (Value::Null, true);
    };
    let target = object
        .get("target")
        .and_then(Value::as_str)
        .unwrap_or("Evaluation target unavailable");
    let rubric: Vec<Value> = object
        .get("rubric")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|item| json!(item))
                .collect()
        })
        .unwrap_or_default();
    let raw_verdict = object
        .get("verdict")
        .and_then(Value::as_str)
        .unwrap_or("inconclusive");
    let verdict = if ["passed", "needs_revision", "failed", "inconclusive"].contains(&raw_verdict) {
        raw_verdict
    } else {
        "inconclusive"
    };
    let findings: Vec<Value> = object
        .get("findings")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|finding| {
                    if let Some(detail) = finding.as_str() {
                        return Some(json!({"title": detail, "detail": detail, "severity": "medium", "evidence_refs": []}));
                    }
                    let finding = finding.as_object()?;
                    let title = finding.get("title").and_then(Value::as_str).unwrap_or("Untitled finding");
                    let detail = finding.get("detail").and_then(Value::as_str).unwrap_or("Finding details unavailable");
                    let raw_severity = finding.get("severity").and_then(Value::as_str).unwrap_or("medium");
                    let severity = if ["info", "low", "medium", "high", "critical"].contains(&raw_severity) { raw_severity } else { "medium" };
                    let evidence_refs = finding.get("evidence_refs").and_then(Value::as_array).cloned().unwrap_or_default();
                    Some(json!({"title": title, "detail": detail, "severity": severity, "evidence_refs": evidence_refs}))
                })
                .collect()
        })
        .unwrap_or_default();
    let normalized =
        json!({"target": target, "rubric": rubric, "verdict": verdict, "findings": findings});
    (normalized.clone(), &normalized != value)
}

fn normalize_agent_artifacts(value: Option<&Value>) -> (Vec<Value>, bool) {
    let Some(items) = value.and_then(Value::as_array) else {
        return (Vec::new(), value.is_some_and(|value| !value.is_null()));
    };
    let normalized: Vec<Value> = items
        .iter()
        .filter_map(|item| {
            if let Some(label) = item.as_str() {
                return Some(json!({"label": label, "path": null, "uri": null}));
            }
            let object = item.as_object()?;
            let label = object
                .get("label")
                .or_else(|| object.get("kind"))
                .or_else(|| object.get("locator"))
                .and_then(Value::as_str)
                .unwrap_or("Unnamed artifact");
            let path = object
                .get("path")
                .filter(|path| {
                    path.get("platform").and_then(Value::as_str).is_some()
                        && path.get("native").and_then(Value::as_str).is_some()
                        && path.get("display").and_then(Value::as_str).is_some()
                        && path.get("identity_key").and_then(Value::as_str).is_some()
                })
                .cloned()
                .unwrap_or(Value::Null);
            let uri = object
                .get("uri")
                .cloned()
                .or_else(|| object.get("locator").cloned())
                .filter(|uri| uri.is_string())
                .unwrap_or(Value::Null);
            Some(json!({"label": label, "path": path, "uri": uri}))
        })
        .collect();
    let changed = normalized != *items;
    (normalized, changed)
}

fn agent_results_view(
    project_id: &str,
    task_id: &str,
    generated_at: &str,
    events: &[V3EventEnvelope],
    evidence_refs: &[Value],
) -> Value {
    let session_events: Vec<&V3EventEnvelope> = events
        .iter()
        .filter(|event| event.task_id.0 == task_id && event.event_type == "session.opened")
        .collect();
    let closed_session_ids: std::collections::BTreeSet<&str> = events
        .iter()
        .filter(|event| event.task_id.0 == task_id && event.event_type == "session.closed")
        .filter_map(|event| event.session_id.as_ref().map(|value| value.0.as_str()))
        .collect();
    let result_events: Vec<&V3EventEnvelope> = events
        .iter()
        .filter(|event| event.task_id.0 == task_id && event.event_type == "agent.result_recorded")
        .collect();
    let mut results = Vec::new();
    let mut result_positions = std::collections::BTreeMap::new();
    let mut warnings = Vec::new();
    for event in result_events {
        let payload = &event.payload;
        let raw_result_evidence = payload.get("evidence_refs").and_then(Value::as_array);
        let result_evidence: Vec<Value> = raw_result_evidence.map(|items| items.iter().filter_map(|item| normalize_agent_evidence_ref(item, event)).collect()).filter(|items: &Vec<Value>| !items.is_empty()).unwrap_or_else(|| vec![json!({
            "evidence_id": event.event_id, "kind": "event", "grade": event.evidence_grade, "label_key": "v3.evidence.agent_result", "locator": format!("vibehub://v3/events/{}", event.event_id), "captured_at": event.recorded_at
        })]);
        let result_id = payload
            .get("result_id")
            .and_then(Value::as_str)
            .unwrap_or(&event.event_id);
        let raw_status = payload
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("failed");
        let session_id = event
            .session_id
            .as_ref()
            .map(|value| value.0.as_str())
            .unwrap_or("session.unknown");
        let status = match raw_status {
            "running" if closed_session_ids.contains(session_id) => {
                warnings.push(json!({
                    "code": "V3_AGENT_RESULT_SESSION_CLOSED",
                    "severity": "warning",
                    "message_key": "v3.warning.agent_result_session_closed",
                    "details": {"result_id": result_id, "projected_status": "pending"},
                    "evidence_refs": result_evidence
                }));
                "pending"
            }
            "pending" | "running" | "succeeded" | "failed" => raw_status,
            invalid => {
                warnings.push(json!({
                    "code": "V3_AGENT_RESULT_STATUS_INVALID",
                    "severity": "warning",
                    "message_key": "v3.warning.agent_result_status_invalid",
                    "details": {"result_id": result_id, "original_status": invalid, "projected_status": "failed"},
                    "evidence_refs": result_evidence
                }));
                "failed"
            }
        };
        let (evaluation, evaluation_normalized) =
            normalize_agent_evaluation(payload.get("evaluation"));
        let (artifacts, artifacts_normalized) = normalize_agent_artifacts(payload.get("artifacts"));
        if evaluation_normalized
            || artifacts_normalized
            || raw_result_evidence.is_some_and(|items| items.iter().any(|item| !item.is_object()))
        {
            warnings.push(json!({
                "code": "V3_AGENT_RESULT_DETAILS_NORMALIZED",
                "severity": "warning",
                "message_key": "v3.warning.agent_result_details_normalized",
                "details": {"result_id": result_id},
                "evidence_refs": result_evidence
            }));
        }
        let result = json!({
            "result_id": result_id,
            "kind": payload.get("kind").and_then(Value::as_str).unwrap_or("execution"),
            "session_id": session_id,
            "node_id": event.node_id.as_ref().map(|value| value.0.clone()),
            "request": {"source": payload.get("request_source").and_then(Value::as_str).unwrap_or("user_request"), "instruction": payload.get("instruction").and_then(Value::as_str).unwrap_or("Result instruction unavailable")},
            "status": status,
            "summary": payload.get("summary").and_then(Value::as_str).unwrap_or(""),
            "body": payload.get("body").cloned().unwrap_or(Value::Null),
            "evaluation": evaluation,
            "artifacts": artifacts,
            "started_at": payload.get("started_at").cloned().unwrap_or(Value::Null),
            "completed_at": payload.get("completed_at").cloned().unwrap_or(Value::Null),
            "evidence_refs": result_evidence
        });
        if let Some(position) = result_positions.get(result_id).copied() {
            results[position] = result;
        } else {
            result_positions.insert(result_id.to_owned(), results.len());
            results.push(result);
        }
    }
    let state = if results.iter().any(|result| result["status"] == "failed") {
        "failed"
    } else if results
        .iter()
        .any(|result| matches!(result["status"].as_str(), Some("pending" | "running")))
    {
        "awaiting_result"
    } else if !results.is_empty() {
        "available"
    } else if session_events.is_empty() {
        "not_executed"
    } else {
        "awaiting_result"
    };
    json!({
        "schema_version": "1.0", "project_id": project_id, "task_id": task_id, "generated_at": generated_at, "model_version": MODEL_VERSION,
        "freshness": "fresh", "completeness": if results.is_empty() {"unknown"} else {"complete"}, "state": state, "results": results,
        "evidence_refs": evidence_refs, "warnings": warnings, "errors": []
    })
}

fn archived_task_summary(
    task: &TaskDocument,
    events: &[V3EventEnvelope],
    lifecycle: &TaskLifecycleProjection,
    generated_at: &str,
) -> Value {
    let task_events: Vec<&V3EventEnvelope> = events
        .iter()
        .filter(|event| event.task_id.0 == task.task_id)
        .collect();
    let state = if task_events
        .iter()
        .any(|event| event.aggregate_id == task.task_id)
    {
        projected_task_state(lifecycle)
    } else {
        task_state(task)
    };
    let criteria = criteria(task, events, lifecycle, generated_at);
    let completion = completion_view(lifecycle);
    let confirmed = completion
        .get("confirmed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let terminal_at = task_events
        .iter()
        .filter(|event| {
            matches!(
                event.event_type.as_str(),
                "task.completion_confirmed"
                    | "task.completion_proposed"
                    | "task.completion_rejected"
            )
        })
        .map(|event| event.occurred_at.clone())
        .max()
        .or_else(|| {
            task_events
                .iter()
                .map(|event| event.occurred_at.clone())
                .max()
        });
    let latest_result = task_events
        .iter()
        .filter(|event| event.event_type == "agent.result_recorded")
        .max_by(|left, right| left.occurred_at.cmp(&right.occurred_at));
    let result_status = latest_result
        .and_then(|event| event.payload.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("not_executed");
    let result_summary = latest_result
        .and_then(|event| event.payload.get("summary"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|summary| !summary.is_empty())
        .map(ToOwned::to_owned);
    let artifact_count = latest_result
        .and_then(|event| event.payload.get("artifacts"))
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let evidence_refs = evidence_refs(task, events, generated_at);
    let session_events = sessions_for_task(events, &task.task_id);
    let plan_total = lifecycle.nodes.len();
    let plan_completed = lifecycle
        .nodes
        .values()
        .filter(|node| node.state == "completed")
        .count();
    let plan_cancelled = lifecycle
        .nodes
        .values()
        .filter(|node| node.state == "cancelled")
        .count();
    let plan_active = lifecycle
        .nodes
        .values()
        .filter(|node| node.state == "active")
        .count();
    let plan_blocked = lifecycle
        .nodes
        .values()
        .filter(|node| matches!(node.state.as_str(), "blocked" | "failed"))
        .count();
    let plan_planned = lifecycle
        .nodes
        .values()
        .filter(|node| matches!(node.state.as_str(), "planned" | "ready"))
        .count();
    let finding_total = lifecycle.findings.len();
    let finding_closed = lifecycle
        .findings
        .values()
        .filter(|finding| finding.state == "closed")
        .count();
    let finding_open = finding_total.saturating_sub(finding_closed);
    let next_action = if state == "completed" && confirmed {
        "无待处理动作；可从验收与时间线回顾本次任务".to_owned()
    } else if state == "completed" {
        "等待用户或受信渠道确认完成；当前不能视为最终绿灯".to_owned()
    } else {
        "确认是否需要重新开始，或基于本次结果创建后续任务".to_owned()
    };

    json!({
        "task_id": task.task_id,
        "title": task.title,
        "intent": task.intent,
        "state": state,
        "risk_level": risk_level(events, &task.task_id),
        "terminal_at": terminal_at,
        "completion": completion,
        "criteria": criteria,
        "blocker_details": blocker_details(task, events, lifecycle, None, generated_at),
        "plan": {
            "total": plan_total,
            "completed": plan_completed,
            "cancelled": plan_cancelled,
            "active": plan_active,
            "blocked": plan_blocked,
            "planned": plan_planned
        },
        "sessions": {
            "total": lifecycle.sessions.len().max(session_events.len()),
            "opened": session_events.iter().filter(|(_, opened, _)| *opened).count(),
            "closed": session_events.iter().filter(|(_, _, closed)| *closed).count(),
            "gapped": lifecycle.sessions.values().filter(|session| session.state == "gapped").count()
        },
        "findings": {"total": finding_total, "open": finding_open, "closed": finding_closed},
        "result": {
            "status": result_status,
            "summary": result_summary,
            "artifact_count": artifact_count,
            "recorded_at": latest_result.map(|event| event.recorded_at.clone())
        },
        "evidence_count": evidence_refs.len(),
        "evidence_refs": evidence_refs,
        "next_action": next_action,
        "source": "v3_projection"
    })
}

fn is_terminal_task_state(state: &str) -> bool {
    matches!(state, "completed" | "cancelled")
}

fn validate_id(name: &str, value: &str) -> Result<(), V3Error> {
    if value.len() < 3
        || value.len() > 128
        || !value.starts_with(|character: char| character.is_ascii_alphabetic())
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "._:-".contains(character))
    {
        return Err(V3Error::new(
            "V3_VALIDATION_ERROR",
            V3ErrorCategory::Validation,
            false,
            format!("{name} is not a stable identifier"),
        ));
    }
    Ok(())
}

fn normalize_dependency_id(value: &str) -> String {
    if value.len() >= 3
        && value.starts_with(|character: char| character.is_ascii_alphabetic())
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "._:-".contains(character))
    {
        value.to_owned()
    } else {
        format!(
            "milestone.{}",
            value
                .to_ascii_lowercase()
                .replace(|character: char| !character.is_ascii_alphanumeric(), "-")
        )
    }
}

fn criteria(
    task: &TaskDocument,
    events: &[V3EventEnvelope],
    lifecycle: &TaskLifecycleProjection,
    generated_at: &str,
) -> Vec<Value> {
    task.acceptance_criteria
        .iter()
        .enumerate()
        .map(|(index, title)| {
            let id = canonical_criterion_id(&task.task_id, index);
            let lifecycle_criterion = lifecycle.criteria.get(&id);
            let passed = events.iter().any(|event| {
                event.task_id.0 == task.task_id
                    && event.event_type == "criterion.passed"
                    && event.payload.get("criterion_id").and_then(Value::as_str) == Some(&id)
            });
            let status = lifecycle_criterion.map(|criterion| match criterion.state {
                CriterionState::Proposed => "proposed", CriterionState::Accepted => "accepted", CriterionState::Passed => "passed",
                CriterionState::Failed => "failed", CriterionState::Blocked => "blocked", CriterionState::NotApplicable => "not_applicable",
            }).unwrap_or(if passed {"passed"} else {"accepted"});
            let evidence_refs = lifecycle_criterion.map(|criterion| criterion.evidence_refs.iter().map(|reference| json!({
                "evidence_id": reference, "kind": "event", "grade": "hard_observed", "label_key": "v3.evidence.criterion", "locator": reference, "captured_at": generated_at
            })).collect::<Vec<_>>()).unwrap_or_default();
            json!({"criterion_id": id, "title": title, "status": status, "required": lifecycle_criterion.map(|criterion| criterion.required).unwrap_or(true), "evidence_refs": evidence_refs})
        })
        .collect()
}

fn blocker_details(
    task: &TaskDocument,
    events: &[V3EventEnvelope],
    lifecycle: &TaskLifecycleProjection,
    node_id: Option<&str>,
    generated_at: &str,
) -> Vec<Value> {
    let task_has_blocked_state = lifecycle
        .nodes
        .values()
        .any(|node| matches!(node.state.as_str(), "blocked" | "failed"))
        || lifecycle
            .criteria
            .values()
            .any(|criterion| criterion.state == CriterionState::Blocked);
    let node_has_blocked_state = node_id
        .and_then(|id| lifecycle.nodes.get(id))
        .is_some_and(|node| matches!(node.state.as_str(), "blocked" | "failed"));

    // Blocker details describe current workflow truth. Historical risk and blocked
    // events remain available in the timeline, but must not keep a recovered task
    // or completed node visually blocked forever.
    if (node_id.is_some() && !node_has_blocked_state)
        || (node_id.is_none() && !task_has_blocked_state)
    {
        return Vec::new();
    }

    let mut candidates: Vec<&V3EventEnvelope> = events
        .iter()
        .filter(|event| event.task_id.0 == task.task_id)
        .filter(|event| {
            node_id.map_or(true, |wanted| {
                event.node_id.as_ref().is_some_and(|id| id.0 == wanted)
                    || event.payload.get("node_id").and_then(Value::as_str) == Some(wanted)
            })
        })
        .filter(|event| is_blocker_event(event))
        .collect();
    candidates.reverse();

    let mut seen_reasons = BTreeSet::new();
    let mut details = Vec::new();
    for event in candidates {
        let (reason_code, kind) = blocker_classification(event);
        if !seen_reasons.insert(reason_code.clone()) {
            continue;
        }
        details.push(blocker_detail_from_event(
            event,
            &reason_code,
            kind,
            node_id,
            generated_at,
        ));
        if details.len() >= 4 {
            break;
        }
    }

    // A node-level event often only contains the technical lifecycle code. Merge the
    // task-level evidence so a blocked plan node shows the human-readable cause too.
    if node_has_blocked_state {
        for detail in blocker_details(task, events, lifecycle, None, generated_at) {
            let reason = detail
                .get("reason_code")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            if seen_reasons.insert(reason.to_owned()) {
                details.push(detail);
            }
            if details.len() >= 6 {
                break;
            }
        }
    }

    // Accepted criteria without evidence are an actionable evidence gap, not an
    // implementation step. Surface them only when the task/node is blocked so a
    // normal in-flight task does not look blocked merely because it is unverified.
    if node_id.is_none() && task_has_blocked_state {
        for (index, title) in task.acceptance_criteria.iter().enumerate() {
            let criterion_id = canonical_criterion_id(&task.task_id, index);
            let Some(criterion) = lifecycle.criteria.get(&criterion_id) else {
                continue;
            };
            if criterion.state != CriterionState::Accepted || !criterion.evidence_refs.is_empty() {
                continue;
            }
            let reason = format!("acceptance.evidence_gap:{criterion_id}");
            if !seen_reasons.insert(reason.clone()) {
                continue;
            }
            details.push(json!({
                "blocker_id": format!("blocker.{}.evidence-gap", criterion_id),
                "reason_code": reason,
                "summary": format!("验收标准尚无可核验证据：{title}"),
                "kind": "evidence_gap",
                "owner": "验收执行者/审查者",
                "precondition": "补充该验收标准的真实 evidence，并由受信 reviewer 确认",
                "resume_action": format!("执行并记录验收标准：{title}"),
                "criterion_id": criterion_id,
                "evidence_refs": []
            }));
        }
    }

    if details.is_empty() && (task_has_blocked_state || node_has_blocked_state) {
        details.push(generic_blocker_detail(node_id));
    }
    details
}

fn is_blocker_event(event: &V3EventEnvelope) -> bool {
    let payload = &event.payload;
    match event.event_type.as_str() {
        "criterion.blocked" => true,
        "plan.node_state_changed" => payload
            .get("state")
            .and_then(Value::as_str)
            .is_some_and(|state| matches!(state, "blocked" | "failed")),
        "risk.logged" => {
            let text = blocker_signal_text(payload);
            payload
                .get("external_action_required")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || payload.get("blocked_on").is_some()
                || payload.get("macos_accessibility").is_some()
                || payload.get("resume_condition").is_some()
                || text.contains("blocked")
                || text.contains("阻塞")
                || text.contains("accessibility")
                || text.contains("tcc")
                || text.contains("权限")
                || text.contains("证据缺口")
                || text.contains("evidence gap")
        }
        "agent.result_recorded" => {
            payload.get("status").and_then(Value::as_str) == Some("failed")
                || payload
                    .get("external_action_required")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                || payload.get("blocked_on").is_some()
                || payload.get("macos_accessibility").is_some()
                || payload.get("resume_condition").is_some()
        }
        _ => false,
    }
}

fn blocker_classification(event: &V3EventEnvelope) -> (String, &'static str) {
    let payload = &event.payload;
    let text = blocker_signal_text(payload);
    if text.contains("accessibility")
        || text.contains("system events")
        || text.contains("tcc")
        || text.contains("-25211")
        || text.contains("辅助访问")
        || text.contains("权限")
    {
        return ("macos.accessibility.tcc".to_owned(), "permission");
    }
    if payload
        .get("external_action_required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return ("external.precondition".to_owned(), "external_precondition");
    }
    if event.event_type == "criterion.blocked" {
        return ("criterion.blocked".to_owned(), "evidence_gap");
    }
    if let Some(reason) = payload
        .get("block_reasons")
        .and_then(Value::as_array)
        .and_then(|items| items.iter().find_map(Value::as_str))
    {
        return (reason.to_owned(), "workflow");
    }
    if event.event_type == "plan.node_state_changed" {
        return ("lifecycle.blocked".to_owned(), "workflow");
    }
    if text.contains("证据")
        || text.contains("evidence")
        || text.contains("gap")
        || text.contains("缺口")
    {
        return ("acceptance.evidence_gap".to_owned(), "evidence_gap");
    }
    ("workflow.blocked".to_owned(), "workflow")
}

fn blocker_signal_text(payload: &Value) -> String {
    [
        "summary",
        "reason",
        "message",
        "body",
        "error",
        "blocked_on",
        "resume_condition",
        "required_next",
        "next",
        "owner",
        "owner_party",
        "blocked_by",
        "responsible",
        "risk_code",
    ]
    .iter()
    .filter_map(|key| payload.get(*key).and_then(Value::as_str))
    .map(str::to_lowercase)
    .collect::<Vec<_>>()
    .join(" ")
}

fn blocker_detail_from_event(
    event: &V3EventEnvelope,
    reason_code: &str,
    kind: &str,
    node_id: Option<&str>,
    generated_at: &str,
) -> Value {
    let payload = &event.payload;
    let summary = payload_text(payload, &["summary", "reason", "message", "body", "error"])
        .unwrap_or_else(|| match kind {
            "permission" => "外部权限或原生交互前置条件未满足".to_owned(),
            "external_precondition" => "外部环境前置条件未满足".to_owned(),
            "evidence_gap" => "验收证据缺口尚未闭环".to_owned(),
            _ => "工作流节点处于阻塞状态".to_owned(),
        });
    let precondition = payload_text(
        payload,
        &["resume_condition", "required_next", "blocked_on"],
    )
    .unwrap_or_else(|| match kind {
        "permission" => {
            "为当前执行宿主授予 macOS Accessibility/System Events 权限，并完成一次复检".to_owned()
        }
        "external_precondition" => "完成外部环境前置后重新运行受影响验证".to_owned(),
        "evidence_gap" => "补充真实 evidence，并由受信 reviewer 确认".to_owned(),
        _ => "补充具体阻塞原因、依赖或恢复条件".to_owned(),
    });
    let resume_action = payload_text(payload, &["next", "required_next", "resume_condition"])
        .unwrap_or_else(|| match kind {
            "permission" => "解除权限阻塞后恢复对应 plan node，只复验未覆盖的原生链路".to_owned(),
            "external_precondition" => {
                "解除外部前置后恢复节点并记录新的 progress/evidence".to_owned()
            }
            "evidence_gap" => "执行验收标准并记录 evidence，再进入审查".to_owned(),
            _ => "在计划图中补充 blocker details 后将节点恢复为 ready/active".to_owned(),
        });
    let owner = payload_text(
        payload,
        &["owner", "owner_party", "blocked_by", "responsible"],
    )
    .unwrap_or_else(|| match kind {
        "permission" => "用户/当前 macOS 执行宿主".to_owned(),
        "external_precondition" => "外部环境/用户".to_owned(),
        "evidence_gap" => "验收执行者/审查者".to_owned(),
        _ => "V3 执行者".to_owned(),
    });
    let criterion_id = payload
        .get("criterion_id")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let event_node_id = event.node_id.as_ref().map(|id| id.0.clone()).or_else(|| {
        payload
            .get("node_id")
            .and_then(Value::as_str)
            .map(str::to_owned)
    });
    let evidence_refs = blocker_evidence_refs(event, generated_at);
    let mut detail = json!({
        "blocker_id": format!("blocker.{}", event.event_id),
        "reason_code": reason_code,
        "summary": summary,
        "kind": kind,
        "owner": owner,
        "precondition": precondition,
        "resume_action": resume_action,
        "evidence_refs": evidence_refs
    });
    if let Some(criterion_id) = criterion_id {
        detail["criterion_id"] = Value::String(criterion_id);
    }
    if let Some(node_id) = node_id.or(event_node_id.as_deref()) {
        detail["node_id"] = Value::String(node_id.to_owned());
    }
    detail
}

fn generic_blocker_detail(node_id: Option<&str>) -> Value {
    let mut detail = json!({
        "blocker_id": format!("blocker.{}", node_id.unwrap_or("task")),
        "reason_code": "lifecycle.blocked",
        "summary": "该工作流节点处于阻塞状态，但事件中没有记录可读的阻塞原因。",
        "kind": "workflow",
        "owner": "V3 执行者",
        "precondition": "记录具体阻塞原因、责任方、解除条件和 evidence",
        "resume_action": "补充 blocker details 后将节点恢复为 ready/active",
        "evidence_refs": []
    });
    if let Some(node_id) = node_id {
        detail["node_id"] = Value::String(node_id.to_owned());
    }
    detail
}

fn payload_text(payload: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        payload
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    })
}

fn blocker_evidence_refs(event: &V3EventEnvelope, generated_at: &str) -> Vec<Value> {
    let timestamp = if event.recorded_at.is_empty() {
        generated_at.to_owned()
    } else {
        event.recorded_at.clone()
    };
    let mut refs = event
        .payload
        .get("evidence_refs")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    item.as_str()
                        .map(|reference| {
                            json!({
                                "evidence_id": reference,
                                "kind": "event",
                                "grade": event.evidence_grade,
                                "label_key": "v3.evidence.blocker",
                                "locator": reference,
                                "captured_at": timestamp
                            })
                        })
                        .or_else(|| item.as_object().map(|object| Value::Object(object.clone())))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    refs.push(json!({
        "evidence_id": event.event_id,
        "kind": "event",
        "grade": event.evidence_grade,
        "label_key": "v3.evidence.blocker",
        "locator": format!("vibehub://v3/events/{}", event.event_id),
        "captured_at": timestamp
    }));
    refs
}

fn completion_view(lifecycle: &TaskLifecycleProjection) -> Value {
    lifecycle.confirmation.as_ref().map_or_else(
        || json!({"proposal_event_id": Value::Null, "proposed_at_version": Value::Null, "digest": Value::Null, "valid": false, "confirmed": false, "confirmed_at_version": Value::Null, "confirmed_by": Value::Null, "channel": Value::Null}),
        |confirmation| json!({
            "proposal_event_id": confirmation.proposal_event_id,
            "proposed_at_version": confirmation.proposed_at_version,
            "digest": confirmation.digest,
            "valid": confirmation.valid,
            "confirmed": confirmation.valid && confirmation.confirmed_at_version.is_some(),
            "confirmed_at_version": confirmation.confirmed_at_version,
            "confirmed_by": confirmation.confirmed_by,
            "channel": confirmation.channel
        }),
    )
}

fn evidence_refs(
    task: &TaskDocument,
    events: &[V3EventEnvelope],
    generated_at: &str,
) -> Vec<Value> {
    let mut refs = vec![json!({
        "evidence_id": format!("evidence.{}.task", task.task_id.to_ascii_lowercase()), "kind": "file", "grade": "hard_observed",
        "label_key": "v3.evidence.task_metadata", "locator": format!(".vibehub/tasks/{}/task.yaml", task.task_id), "captured_at": generated_at
    })];
    refs.extend(events.iter().filter(|event| event.task_id.0 == task.task_id).map(|event| json!({
        "evidence_id": event.event_id, "kind": "event", "grade": event.evidence_grade,
        "label_key": "v3.evidence.event", "locator": format!("vibehub://v3/events/{}", event.event_id), "captured_at": event.recorded_at
    })));
    refs
}

fn sessions_for_task(events: &[V3EventEnvelope], task_id: &str) -> Vec<(String, bool, bool)> {
    let mut sessions = std::collections::BTreeMap::<String, (bool, bool)>::new();
    for event in events.iter().filter(|event| event.task_id.0 == task_id) {
        if let Some(session_id) = &event.session_id {
            let entry = sessions.entry(session_id.0.clone()).or_default();
            entry.0 |= event.event_type == "session.opened";
            entry.1 |= event.event_type == "session.closed";
        }
    }
    sessions
        .into_iter()
        .map(|(id, (open, closed))| (id, open, closed))
        .collect()
}

fn timeline_event(event: &V3EventEnvelope) -> Value {
    let kind = match event.event_type.as_str() {
        "risk.logged" | "finding.opened" | "finding.closed" | "finding.regressed" => "finding",
        "session.opened" | "session.closed" => "session",
        event_type if event_type.starts_with("criterion.") => "validation",
        event_type if event_type.starts_with("attempt.") => "attempt",
        event_type if event_type.starts_with("plan.") => "plan",
        event_type if event_type.starts_with("task.completion_") => "confirmation",
        _ => "evidence",
    };
    json!({
        "timeline_event_id": event.event_id, "kind": kind, "occurred_at": event.occurred_at,
        "recorded_at": event.recorded_at, "order_state": "ordered",
        "lane_id": event.session_id.as_ref().map(|id| id.0.clone()).unwrap_or_else(|| "lane.task".to_owned()),
        "actor": event.actor, "tool": Value::Null, "node_id": event.node_id.as_ref().map(|id| id.0.clone()),
        "session_id": event.session_id.as_ref().map(|id| id.0.clone()), "commit_sha": event.commit_sha,
        "summary_key": event.event_type, "details": event.payload, "evidence_refs": []
    })
}

fn native_path(path: &Path) -> Value {
    let native = path.to_string_lossy().to_string();
    let platform = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unknown"
    };
    let (display, path_kind) = if platform == "windows" {
        (windows_display_path(&native), windows_path_kind(&native))
    } else {
        (native.clone(), "absolute")
    };
    json!({"platform": platform, "native": native, "display": display, "identity_key": format!("{platform}:{native}"), "path_kind": path_kind, "accessible": true})
}

fn windows_display_path(native: &str) -> String {
    let display = native.replace('\\', "/");
    if let Some(rest) = display.strip_prefix("//?/UNC/") {
        format!("//{rest}")
    } else if let Some(rest) = display.strip_prefix("//?/") {
        rest.to_owned()
    } else {
        display
    }
}

fn windows_path_kind(native: &str) -> &'static str {
    if native.starts_with(r"\\?\") {
        "extended"
    } else if native.starts_with(r"\\") {
        "unc"
    } else if native.as_bytes().get(1) == Some(&b':') {
        "drive"
    } else {
        "absolute"
    }
}

fn page(returned: usize) -> Value {
    json!({"cursor": Value::Null, "next_cursor": Value::Null, "limit": 200, "returned": returned, "total_estimate": returned, "truncated": false, "truncation_reason": "none", "model_version": MODEL_VERSION})
}

fn task_state(task: &TaskDocument) -> &'static str {
    if task.phase_status == "completed" {
        "completed"
    } else if task.phase_status == "blocked" {
        "blocked"
    } else {
        "active"
    }
}

fn node_state(task: &TaskDocument) -> &'static str {
    if task.phase_status == "completed" {
        "completed"
    } else if task.phase_status == "blocked" {
        "blocked"
    } else {
        "active"
    }
}

fn projected_task_state(lifecycle: &TaskLifecycleProjection) -> &str {
    match lifecycle.state.as_str() {
        "completion_pending" => "review",
        "completed" => "completed",
        "cancelled" => "cancelled",
        "blocked" => "blocked",
        _ if lifecycle.nodes.values().any(|node| node.state == "active") => "active",
        _ if lifecycle
            .nodes
            .values()
            .any(|node| matches!(node.state.as_str(), "blocked" | "failed")) =>
        {
            "blocked"
        }
        _ if !lifecycle.nodes.is_empty()
            && lifecycle
                .nodes
                .values()
                .all(|node| matches!(node.state.as_str(), "completed" | "cancelled")) =>
        {
            "review"
        }
        _ if lifecycle.nodes.values().any(|node| node.state == "ready") => "planned",
        _ if matches!(lifecycle.state.as_str(), "planned" | "active" | "review") => {
            lifecycle.state.as_str()
        }
        _ => "active",
    }
}

fn view_node_state(state: &str) -> &str {
    match state {
        "failed" => "blocked",
        "planned" | "ready" | "active" | "blocked" | "review" | "completed" | "cancelled"
        | "superseded" => state,
        _ => "planned",
    }
}

fn risk_level(events: &[V3EventEnvelope], task_id: &str) -> &'static str {
    if events
        .iter()
        .any(|event| event.task_id.0 == task_id && event.event_type == "risk.logged")
    {
        "medium"
    } else {
        "none"
    }
}

fn protocol_coverage(opened: usize, closed: usize, explicit_gaps: usize) -> &'static str {
    if explicit_gaps > 0 {
        "gapped"
    } else if opened == 0 {
        "unknown"
    } else if opened == closed {
        "complete"
    } else {
        "partial"
    }
}

fn internal(code: &'static str) -> impl FnOnce(std::io::Error) -> V3Error {
    move |error| V3Error::new(code, V3ErrorCategory::Internal, true, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v3::lifecycle::PlanNodeProjection;
    use crate::v3::{
        create_v3_task, EventDraft, EvidenceGrade, OrchestrationCommand, PlanAddNodeCommand,
        PlanCommandIdentity, ProjectId, SessionId, TaskId, V3ApplicationService,
        V3TaskCreateRequest,
    };
    use std::collections::BTreeSet;
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn windows_paths_preserve_native_identity_and_normalize_display() {
        let cases = [
            (r"C:\Users\Alex\VibeHub", "C:/Users/Alex/VibeHub", "drive"),
            (r"\\server\share\VibeHub", "//server/share/VibeHub", "unc"),
            (
                r"\\?\C:\Users\Alex\VibeHub",
                "C:/Users/Alex/VibeHub",
                "extended",
            ),
            (
                r"\\?\UNC\server\share\VibeHub",
                "//server/share/VibeHub",
                "extended",
            ),
        ];

        for (native, expected_display, expected_kind) in cases {
            assert_eq!(windows_display_path(native), expected_display);
            assert_eq!(windows_path_kind(native), expected_kind);
        }
    }

    #[test]
    fn task_state_reflects_operational_plan_state_before_completion_confirmation() {
        let mut lifecycle = TaskLifecycleProjection::empty("task.test");
        lifecycle.state = "active".to_owned();
        lifecycle.nodes.insert(
            "node.test".to_owned(),
            PlanNodeProjection {
                node_id: "node.test".to_owned(),
                title: "Test".to_owned(),
                goal: "Test state projection".to_owned(),
                state: "blocked".to_owned(),
                dependencies: BTreeSet::new(),
                scope: Vec::new(),
            },
        );
        assert_eq!(projected_task_state(&lifecycle), "blocked");
        lifecycle.nodes.get_mut("node.test").unwrap().state = "active".to_owned();
        assert_eq!(projected_task_state(&lifecycle), "active");
        lifecycle.nodes.get_mut("node.test").unwrap().state = "completed".to_owned();
        assert_eq!(projected_task_state(&lifecycle), "review");
        lifecycle.state = "completed".to_owned();
        assert_eq!(projected_task_state(&lifecycle), "completed");
    }

    #[test]
    fn active_session_is_partial_coverage_not_a_protocol_gap() {
        assert_eq!(protocol_coverage(1, 0, 0), "partial");
        assert_eq!(protocol_coverage(1, 1, 0), "complete");
        assert_eq!(protocol_coverage(0, 0, 0), "unknown");
        assert_eq!(protocol_coverage(1, 0, 1), "gapped");
    }

    #[test]
    fn real_bundle_preserves_identity_across_all_views() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-views-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub/tasks/task.test")).unwrap();
        fs::create_dir_all(root.join(".vibehub/tasks/current")).unwrap();
        let task = "task_id: task.test\ntitle: Test task\nintent: Verify real views\nphase: implement\nphase_status: active\nacceptance_criteria:\n- Views validate\ndependencies: []\n";
        fs::write(root.join(".vibehub/tasks/task.test/task.yaml"), task).unwrap();
        fs::write(root.join(".vibehub/tasks/current/task.yaml"), task).unwrap();
        let repo = V3ViewRepository::open(&root).unwrap();
        let project_id = repo.project_id();
        V3ApplicationService::open(&root)
            .unwrap()
            .session_open(
                &project_id,
                "task.test",
                "session.test",
                "codex",
                0,
                "open.1",
            )
            .unwrap();
        let bundle = repo.load_bundle("task.test").unwrap();
        for view in [
            &bundle.project_overview,
            &bundle.project_structure,
            &bundle.agent_results,
            &bundle.task_timeline,
            &bundle.plan_graph,
            &bundle.node_brief,
        ] {
            assert_eq!(view["project_id"], project_id);
            assert_eq!(view["schema_version"], "1.0");
        }
        assert_eq!(bundle.task_timeline["events"].as_array().unwrap().len(), 1);
        assert_eq!(bundle.task_timeline["completion"]["valid"], false);
        assert_eq!(bundle.task_timeline["completion"]["confirmed"], false);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn malformed_task_metadata_does_not_block_valid_v3_task_views() {
        let root =
            std::env::temp_dir().join(format!("vibehub-v3-malformed-task-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        super::super::initialize_v3(&root).unwrap();
        let valid = create_v3_task(
            &root,
            V3TaskCreateRequest {
                title: "Valid task".to_owned(),
                intent: "Keep valid V3 tasks visible".to_owned(),
                acceptance_criteria: vec!["View bundle remains available".to_owned()],
            },
        )
        .unwrap();
        let legacy_task_id = "T-20260718191720-ffb8c89a";
        let legacy_dir = root.join(".vibehub/tasks").join(legacy_task_id);
        fs::create_dir(&legacy_dir).unwrap();
        fs::write(
            legacy_dir.join("task.yaml"),
            format!(
                "schema_version: 1\\nkind: vibehub_task\\ntask_id: {legacy_task_id}\\ntitle: Legacy task\\nmode: evidence_drive\\nphase: align\\nphase_status: active\\n"
            ),
        )
        .unwrap();
        fs::write(
            root.join(".vibehub/tasks/current"),
            format!(
                "schema_version: 1\\nkind: current_task_pointer\\ntask_id: {legacy_task_id}\\npath: .vibehub/tasks/{legacy_task_id}\\nupdated_at: 2026-07-18T00:00:00Z\\nupdated_by: vibehub\\n"
            ),
        )
        .unwrap();

        let repository = V3ViewRepository::open(&root).unwrap();
        let bundle = repository.load_bundle(&valid.task_id).unwrap();

        assert_eq!(repository.current_task_id().unwrap(), valid.task_id);
        assert_eq!(
            bundle.project_overview["active_tasks"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let warning = bundle.project_overview["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|warning| warning["code"] == "V3_TASK_METADATA_INVALID")
            .expect("invalid legacy task warning");
        assert_eq!(warning["details"]["task_id"], legacy_task_id);
        assert!(warning["details"]["recommended_action"]
            .as_str()
            .unwrap()
            .contains("quarantine-task"));
        assert!(bundle.task_timeline["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning["code"] == "V3_TASK_METADATA_INVALID"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn plan_nodes_expose_sessions_by_node_id() {
        let project =
            std::env::temp_dir().join(format!("vibehub-v3-plan-session-{}", Uuid::new_v4()));
        fs::create_dir_all(project.join(".vibehub/tasks/task.test")).unwrap();
        fs::write(project.join(".vibehub/tasks/task.test/task.yaml"), "task_id: task.test\ntitle: Plan session\nintent: Map the active session\nphase: plan\nphase_status: active\nacceptance_criteria:\n- Session is visible\n").unwrap();
        let repository = V3ViewRepository::open(&project).unwrap();
        let project_id = repository.project_id();
        let app = V3ApplicationService::open(&project).unwrap();
        app.plan_add_node(PlanAddNodeCommand {
            identity: PlanCommandIdentity {
                project_id: project_id.clone(),
                task_id: "task.test".to_owned(),
                actor: "codex".to_owned(),
                expected_version: 0,
                idempotency_key: "plan.node.test".to_owned(),
            },
            node_id: "node.test".to_owned(),
            title: "Plan session".to_owned(),
            goal: "Map the active session".to_owned(),
            scope: Vec::new(),
            dependencies: Vec::new(),
        })
        .unwrap();
        app.session_open_with_context(
            &project_id,
            "task.test",
            "session.test",
            "codex",
            0,
            "session.node.test",
            None,
            Some("node.test".to_owned()),
            None,
        )
        .unwrap();

        let bundle = repository.load_bundle("task.test").unwrap();
        assert_eq!(
            bundle.plan_graph["nodes"][0]["session_ids"],
            json!(["session.test"])
        );
        assert_eq!(bundle.task_timeline["lanes"][0]["state"], "active");
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn session_working_directory_and_agent_results_are_projected_from_events() {
        let project = std::env::temp_dir().join(format!("vibehub-v3-project-{}", Uuid::new_v4()));
        let workspace =
            std::env::temp_dir().join(format!("vibehub-v3-workspace-{}", Uuid::new_v4()));
        fs::create_dir_all(project.join(".vibehub/tasks/task.test")).unwrap();
        fs::create_dir_all(&workspace).unwrap();
        fs::write(workspace.join("package.json"), r#"{"name":"workspace"}"#).unwrap();
        fs::write(project.join(".vibehub/tasks/task.test/task.yaml"), "task_id: task.test\ntitle: Agent result\nintent: Project result\nphase: implement\nphase_status: active\n").unwrap();
        let repository = V3ViewRepository::open(&project).unwrap();
        let project_id = repository.project_id();
        let app = V3ApplicationService::open(&project).unwrap();
        app.session_open_with_context(
            &project_id,
            "task.test",
            "session.test",
            "agent",
            0,
            "session.context",
            Some(workspace.to_string_lossy().into_owned()),
            None,
            None,
        )
        .unwrap();
        app.agent_result_record(&project_id, "task.test", "session.test", "agent", 1, "result.running", "result.test", None, json!({
            "kind":"evaluation", "request_source":"evaluation_instruction", "instruction":"Evaluate workspace", "status":"running", "summary":"Running", "body":null, "evaluation":null, "artifacts":[], "started_at":null, "completed_at":null
        })).unwrap();
        app.agent_result_record(&project_id, "task.test", "session.test", "agent", 2, "result.succeeded", "result.test", None, json!({
            "kind":"evaluation", "request_source":"evaluation_instruction", "instruction":"Evaluate workspace", "status":"succeeded", "summary":"Passed", "body":"All checks passed", "evaluation":{"target":"workspace","rubric":["exists"],"verdict":"passed","findings":[]}, "artifacts":[], "started_at":null, "completed_at":null
        })).unwrap();

        let bundle = repository.load_bundle("task.test").unwrap();
        assert_eq!(
            bundle.project_structure["workspace"]["source"],
            "session_working_directory"
        );
        assert_eq!(
            bundle.project_structure["workspace"]["root"]["native"],
            workspace.canonicalize().unwrap().to_string_lossy().as_ref()
        );
        assert_eq!(bundle.agent_results["state"], "available");
        assert_eq!(bundle.agent_results["results"].as_array().unwrap().len(), 1);
        assert_eq!(bundle.agent_results["results"][0]["status"], "succeeded");
        assert_eq!(bundle.agent_results["results"][0]["summary"], "Passed");
        assert_eq!(
            bundle.agent_results["results"][0]["evaluation"]["verdict"],
            "passed"
        );
        fs::remove_dir_all(project).unwrap();
        fs::remove_dir_all(workspace).unwrap();
    }

    #[test]
    fn closed_session_workspace_falls_back_to_project_root_with_reason() {
        let project = std::env::temp_dir().join(format!("vibehub-v3-project-{}", Uuid::new_v4()));
        let workspace =
            std::env::temp_dir().join(format!("vibehub-v3-workspace-{}", Uuid::new_v4()));
        fs::create_dir_all(project.join(".vibehub/tasks/task.test")).unwrap();
        fs::create_dir_all(&workspace).unwrap();
        fs::write(project.join(".vibehub/tasks/task.test/task.yaml"), "task_id: task.test\ntitle: Closed session\nintent: Fall back after close\nphase: implement\nphase_status: active\n").unwrap();
        let repository = V3ViewRepository::open(&project).unwrap();
        let project_id = repository.project_id();
        let app = V3ApplicationService::open(&project).unwrap();
        app.session_open_with_context(
            &project_id,
            "task.test",
            "session.test",
            "agent",
            0,
            "session.open",
            Some(workspace.to_string_lossy().into_owned()),
            None,
            None,
        )
        .unwrap();
        app.session_close(
            &project_id,
            "task.test",
            "session.test",
            "agent",
            1,
            "session.close",
        )
        .unwrap();

        let bundle = repository.load_bundle("task.test").unwrap();
        assert_eq!(
            bundle.project_structure["workspace"]["source"],
            "project_root_fallback"
        );
        assert_eq!(
            bundle.project_structure["workspace"]["session_id"],
            Value::Null
        );
        assert_eq!(
            bundle.project_structure["workspace"]["root"]["native"],
            project.canonicalize().unwrap().to_string_lossy().as_ref()
        );
        assert_eq!(
            bundle.project_structure["workspace"]["fallback_reason"],
            "no active session or worktree context was recorded"
        );

        fs::remove_dir_all(project).unwrap();
        fs::remove_dir_all(workspace).unwrap();
    }

    #[test]
    fn architecture_file_counts_use_git_visible_files() {
        let project = std::env::temp_dir().join(format!("vibehub-v3-counts-{}", Uuid::new_v4()));
        fs::create_dir_all(project.join(".vibehub/tasks/task.test")).unwrap();
        fs::create_dir_all(project.join("src")).unwrap();
        fs::create_dir_all(project.join("node_modules/dependency")).unwrap();
        fs::write(project.join(".vibehub/tasks/task.test/task.yaml"), "task_id: task.test\ntitle: Count files\nintent: Ignore heavy directories\nphase: implement\nphase_status: active\n").unwrap();
        fs::write(project.join("package.json"), r#"{"name":"count-files"}"#).unwrap();
        fs::write(project.join(".gitignore"), "/node_modules/\n").unwrap();
        fs::write(project.join("src/index.ts"), "export const value = 1;").unwrap();
        fs::write(
            project.join("node_modules/dependency/index.js"),
            "module.exports = 1;",
        )
        .unwrap();
        silent_command("git")
            .arg("-C")
            .arg(&project)
            .arg("init")
            .output()
            .unwrap();

        let visible = visible_project_files(&project);
        assert!(visible.iter().any(|path| path == "src/index.ts"));
        assert!(visible
            .iter()
            .all(|path| !path.starts_with("node_modules/")));

        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn persisted_json_index_drives_stable_evidenced_architecture_views() {
        let project =
            std::env::temp_dir().join(format!("vibehub-v3-json-index-{}", Uuid::new_v4()));
        fs::create_dir_all(project.join(".vibehub/tasks/task.test")).unwrap();
        fs::create_dir_all(project.join("src/feature")).unwrap();
        fs::create_dir_all(project.join("crates/vibehub-core/src")).unwrap();
        fs::create_dir_all(project.join("crates/vibehub-cli/src")).unwrap();
        fs::create_dir_all(project.join("src-tauri/src")).unwrap();
        fs::write(
            project.join(".vibehub/tasks/task.test/task.yaml"),
            "task_id: task.test\ntitle: JSON architecture index\nintent: Parse a persisted project model\nphase: implement\nphase_status: active\n",
        )
        .unwrap();
        fs::write(
            project.join("Cargo.toml"),
            "[workspace]\nmembers=[\"crates/vibehub-core\",\"crates/vibehub-cli\",\"src-tauri\"]\nresolver=\"2\"\n",
        )
        .unwrap();
        fs::write(project.join("package.json"), r#"{"name":"frontend-app"}"#).unwrap();
        fs::write(
            project.join("crates/vibehub-core/Cargo.toml"),
            "[package]\nname=\"vibehub-core\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        fs::write(
            project.join("crates/vibehub-cli/Cargo.toml"),
            "[package]\nname=\"vibehub-cli\"\nversion=\"0.1.0\"\n[dependencies]\nvibehub-core={path=\"../vibehub-core\"}\n",
        )
        .unwrap();
        fs::write(
            project.join("src-tauri/Cargo.toml"),
            "[package]\nname=\"desktop-host\"\nversion=\"0.1.0\"\n[dependencies]\ncore={package=\"vibehub-core\",path=\"../crates/vibehub-core\"}\nadapters={package=\"vibehub-cli\",path=\"../crates/vibehub-cli\"}\n",
        )
        .unwrap();
        fs::write(
            project.join("src/app.ts"),
            "import { tool } from './feature/tool';\nexport { tool };\n",
        )
        .unwrap();
        fs::write(
            project.join("src/feature/tool.ts"),
            "export const tool = true;\n",
        )
        .unwrap();
        fs::write(
            project.join("crates/vibehub-core/src/lib.rs"),
            "pub struct Core;\n",
        )
        .unwrap();
        fs::write(
            project.join("crates/vibehub-cli/src/lib.rs"),
            "use vibehub_core::Core;\npub fn run(_: Core) {}\n",
        )
        .unwrap();
        fs::write(
            project.join("src-tauri/src/main.rs"),
            "use vibehub_core::Core;\nfn main() { let _ = Core; }\n",
        )
        .unwrap();
        silent_command("git")
            .arg("-C")
            .arg(&project)
            .arg("init")
            .output()
            .unwrap();

        let repository = V3ViewRepository::open(&project).unwrap();
        let first = repository.load_bundle("task.test").unwrap();
        let snapshot_path = project.join(crate::v3::project_intelligence::PROJECT_MODEL_INDEX_PATH);
        let snapshot: ProjectModelSnapshot =
            serde_json::from_slice(&fs::read(&snapshot_path).unwrap()).unwrap();
        assert!(snapshot
            .analyzer_findings
            .iter()
            .any(|finding| finding.kind == "package_manifest"));
        assert!(snapshot
            .analyzer_findings
            .iter()
            .any(|finding| finding.kind == "dependency"));

        let nodes = first.project_structure["architecture_nodes"]
            .as_array()
            .unwrap();
        let edges = first.project_structure["architecture_edges"]
            .as_array()
            .unwrap();
        let ids = nodes
            .iter()
            .filter_map(|node| node["node_id"].as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), nodes.len());
        assert!(edges.iter().all(|edge| {
            let from = edge["from_node_id"].as_str().unwrap();
            let to = edge["to_node_id"].as_str().unwrap();
            from != to && ids.contains(from) && ids.contains(to)
        }));
        assert_eq!(
            first.project_overview["architecture"]["modules"],
            nodes
                .iter()
                .filter(|node| node["kind"] != "workspace")
                .count()
        );
        assert_eq!(
            first.project_overview["architecture"]["relationships"],
            edges.len()
        );
        assert!(first.project_overview["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|warning| warning["code"] != "V3_ARCHITECTURE_INDEX_PENDING"));
        assert!(first.project_structure["evidence_refs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reference| reference["locator"]
                == format!(
                    "file:{}",
                    crate::v3::project_intelligence::PROJECT_MODEL_INDEX_PATH
                )));
        assert!(nodes
            .iter()
            .filter(|node| node["kind"] == "package")
            .all(
                |node| node["evidence_refs"][0]["locator"]
                    .as_str()
                    .is_some_and(|locator| locator.ends_with("Cargo.toml")
                        || locator.ends_with("package.json"))
            ));

        let stable_ids = nodes
            .iter()
            .filter(|node| node["kind"] == "package")
            .filter_map(|node| {
                Some((
                    node["name"].as_str()?.to_owned(),
                    node["node_id"].as_str()?.to_owned(),
                ))
            })
            .collect::<BTreeMap<_, _>>();
        fs::create_dir_all(project.join("aaa/src")).unwrap();
        fs::write(
            project.join("aaa/Cargo.toml"),
            "[package]\nname=\"unrelated\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        fs::write(project.join("aaa/src/lib.rs"), "pub struct Unrelated;\n").unwrap();
        let second = repository.load_bundle("task.test").unwrap();
        let second_ids = second.project_structure["architecture_nodes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|node| node["kind"] == "package")
            .filter_map(|node| {
                Some((
                    node["name"].as_str()?.to_owned(),
                    node["node_id"].as_str()?.to_owned(),
                ))
            })
            .collect::<BTreeMap<_, _>>();
        for (name, id) in stable_ids {
            assert_eq!(second_ids.get(&name), Some(&id));
        }
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn project_overview_lists_every_v3_task_with_its_own_session_count() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-tasks-{}", Uuid::new_v4()));
        for (task_id, title, phase_status) in [
            ("task.completed", "Completed task", "completed"),
            ("task.one", "First task", "active"),
            ("task.two", "Second task", "active"),
        ] {
            let task_dir = root.join(".vibehub/tasks").join(task_id);
            fs::create_dir_all(&task_dir).unwrap();
            fs::write(
                task_dir.join("task.yaml"),
                format!("task_id: {task_id}\ntitle: {title}\nintent: Verify task enumeration\nphase: implement\nphase_status: {phase_status}\nacceptance_criteria:\n- Visible in overview\ndependencies: []\n"),
            )
            .unwrap();
        }
        let repository = V3ViewRepository::open(&root).unwrap();
        let project_id = repository.project_id();
        V3ApplicationService::open(&root)
            .unwrap()
            .session_open(
                &project_id,
                "task.two",
                "session.two",
                "agent",
                0,
                "session.two.open",
            )
            .unwrap();

        let bundle = repository.load_bundle("task.one").unwrap();
        let tasks = bundle.project_overview["active_tasks"].as_array().unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0]["task_id"], "task.one");
        assert_eq!(tasks[0]["active_sessions"], 0);
        assert_eq!(tasks[1]["task_id"], "task.two");
        assert_eq!(tasks[1]["active_sessions"], 1);
        let archived = bundle.project_overview["archived_tasks"]
            .as_array()
            .unwrap();
        assert_eq!(archived.len(), 1);
        assert_eq!(archived[0]["task_id"], "task.completed");
        assert_eq!(archived[0]["state"], "completed");
        assert_eq!(archived[0]["criteria"][0]["status"], "accepted");
        assert_eq!(archived[0]["plan"]["total"], 0);
        assert_eq!(archived[0]["result"]["status"], "not_executed");
        assert_eq!(archived[0]["completion"]["confirmed"], false);
        assert_eq!(archived[0]["source"], "v3_projection");
        assert_eq!(
            bundle.project_structure["page"]["returned"],
            bundle.project_structure["page"]["total_estimate"]
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn completed_pointer_falls_back_to_a_non_terminal_task_and_is_not_active() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-current-{}", Uuid::new_v4()));
        let tasks_root = root.join(".vibehub/tasks");
        for (task_id, phase_status) in [("task.completed", "completed"), ("task.active", "active")]
        {
            let task_dir = tasks_root.join(task_id);
            fs::create_dir_all(&task_dir).unwrap();
            fs::write(
                task_dir.join("task.yaml"),
                format!("task_id: {task_id}\ntitle: {task_id}\nintent: Test current selection\nphase: implement\nphase_status: {phase_status}\n"),
            )
            .unwrap();
        }
        fs::write(
            tasks_root.join("current"),
            "schema_version: 1\nkind: current_task_pointer\ntask_id: task.completed\npath: .vibehub/tasks/task.completed\nupdated_at: 2026-07-14T00:00:00Z\nupdated_by: vibehub\n",
        )
        .unwrap();

        let repository = V3ViewRepository::open(&root).unwrap();
        assert_eq!(repository.current_task_id().unwrap(), "task.active");
        let bundle = repository.load_bundle("task.active").unwrap();
        assert_eq!(bundle.project_overview["current_task_id"], "task.active");
        let active_tasks = bundle.project_overview["active_tasks"].as_array().unwrap();
        assert_eq!(active_tasks.len(), 1);
        assert_eq!(active_tasks[0]["task_id"], "task.active");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_historical_agent_result_status_is_projected_as_failed() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-result-view-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub/tasks/task.test")).unwrap();
        fs::write(
            root.join(".vibehub/tasks/task.test/task.yaml"),
            "task_id: task.test\ntitle: Result compatibility\nintent: Render historical events\nphase: implement\nphase_status: active\n",
        )
        .unwrap();
        let repository = V3ViewRepository::open(&root).unwrap();
        let project_id = repository.project_id();
        let store = V3EventStore::open(&root).unwrap();
        store
            .append(EventDraft {
                event_type: "agent.result_recorded".to_owned(),
                aggregate_id: "session.test".to_owned(),
                expected_version: 0,
                idempotency_key: "result.blocked".to_owned(),
                project_id: ProjectId(project_id),
                task_id: TaskId("task.test".to_owned()),
                node_id: None,
                session_id: Some(SessionId("session.test".to_owned())),
                worktree_id: None,
                lease_id: None,
                operation_id: None,
                actor: "legacy-agent".to_owned(),
                evidence_grade: EvidenceGrade::AgentReported,
                occurred_at: None,
                commit_sha: None,
                payload: json!({
                    "result_id": "result.test",
                    "kind": "execution",
                    "request_source": "user_request",
                    "instruction": "Run",
                    "status": "blocked",
                    "summary": "Blocked"
                }),
            })
            .unwrap();

        let bundle = repository.load_bundle("task.test").unwrap();
        assert_eq!(bundle.agent_results["state"], "failed");
        assert_eq!(bundle.agent_results["results"][0]["status"], "failed");
        assert_eq!(
            bundle.agent_results["warnings"][0]["code"],
            "V3_AGENT_RESULT_STATUS_INVALID"
        );
        assert_eq!(
            bundle.agent_results["warnings"][0]["details"]["original_status"],
            "blocked"
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn malformed_historical_agent_result_details_are_normalized() {
        let root =
            std::env::temp_dir().join(format!("vibehub-v3-result-details-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub/tasks/task.test")).unwrap();
        fs::write(
            root.join(".vibehub/tasks/task.test/task.yaml"),
            "task_id: task.test\ntitle: Result compatibility\nintent: Render historical events\nphase: implement\nphase_status: active\n",
        )
        .unwrap();
        let repository = V3ViewRepository::open(&root).unwrap();
        let project_id = repository.project_id();
        V3EventStore::open(&root)
            .unwrap()
            .append(EventDraft {
                event_type: "agent.result_recorded".to_owned(),
                aggregate_id: "session.test".to_owned(),
                expected_version: 0,
                idempotency_key: "result.malformed".to_owned(),
                project_id: ProjectId(project_id),
                task_id: TaskId("task.test".to_owned()),
                node_id: None,
                session_id: Some(SessionId("session.test".to_owned())),
                worktree_id: None,
                lease_id: None,
                operation_id: None,
                actor: "legacy-agent".to_owned(),
                evidence_grade: EvidenceGrade::AgentReported,
                occurred_at: None,
                commit_sha: None,
                payload: json!({
                    "result_id": "result.test",
                    "kind": "evaluation",
                    "request_source": "evaluation_instruction",
                    "instruction": "Review",
                    "status": "succeeded",
                    "summary": "Reviewed",
                    "evaluation": {"target": "workspace", "verdict": "needs_action", "findings": ["missing rubric"]},
                    "artifacts": ["src/example.rs"],
                    "evidence_refs": ["evt.legacy"]
                }),
            })
            .unwrap();

        let bundle = repository.load_bundle("task.test").unwrap();
        let result = &bundle.agent_results["results"][0];
        assert_eq!(result["evaluation"]["rubric"], json!([]));
        assert_eq!(result["evaluation"]["verdict"], "inconclusive");
        assert_eq!(
            result["evaluation"]["findings"][0]["title"],
            "missing rubric"
        );
        assert_eq!(result["artifacts"][0]["label"], "src/example.rs");
        assert_eq!(result["evidence_refs"][0]["evidence_id"], "evt.legacy");
        assert!(bundle.agent_results["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning["code"] == "V3_AGENT_RESULT_DETAILS_NORMALIZED"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn successful_agent_result_evidence_does_not_create_a_false_blocker() {
        let root =
            std::env::temp_dir().join(format!("vibehub-v3-blocker-signal-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub/tasks/task.test")).unwrap();
        fs::write(
            root.join(".vibehub/tasks/task.test/task.yaml"),
            "task_id: task.test\ntitle: Blocker signal\nintent: Keep evidence semantic\nphase: implement\nphase_status: active\nacceptance_criteria:\n- Evidence remains factual\n",
        )
        .unwrap();
        let repository = V3ViewRepository::open(&root).unwrap();
        let project_id = repository.project_id();
        let app = V3ApplicationService::open(&root).unwrap();
        app.session_open(
            &project_id,
            "task.test",
            "session.test",
            "codex",
            0,
            "session.open",
        )
        .unwrap();
        app.agent_result_record(
            &project_id,
            "task.test",
            "session.test",
            "codex",
            1,
            "result.success",
            "result.success",
            None,
            json!({
                "kind": "execution",
                "request_source": "user_request",
                "instruction": "Read evidence",
                "status": "succeeded",
                "summary": "M9 projection was inspected",
                "body": "The M9 accessibility evidence belongs to another task.",
                "evidence_refs": [{"evidence_id":"file:m9-accessibility","kind":"file","grade":"hard_observed","label_key":"test","locator":"file:m9-accessibility"}]
            }),
        )
        .unwrap();

        let bundle = repository.load_bundle("task.test").unwrap();
        assert_eq!(bundle.task_timeline["blocker_details"], json!([]));
        assert_eq!(
            bundle.project_overview["active_tasks"][0]["blocker_details"],
            json!([])
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resolved_criterion_does_not_project_historical_blockers() {
        let root =
            std::env::temp_dir().join(format!("vibehub-v3-resolved-blocker-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub/tasks/task.test")).unwrap();
        fs::write(
            root.join(".vibehub/tasks/task.test/task.yaml"),
            "task_id: task.test\ntitle: Resolved blocker\nintent: Keep history without stale status\nphase: implement\nphase_status: active\nacceptance_criteria:\n- Native flow passes\n",
        )
        .unwrap();
        let repository = V3ViewRepository::open(&root).unwrap();
        let project_id = repository.project_id();
        let app = V3ApplicationService::open(&root).unwrap();
        app.lifecycle_command(super::super::lifecycle::command(
            "criterion.blocked",
            &project_id,
            "task.test",
            0,
            "criterion.blocked",
            json!({"criterion_id":"criterion.task.test.c01","reviewer":"reviewer","evidence_refs":["evt.blocked"],"reason":"macOS Accessibility permission was unavailable"}),
        ))
        .unwrap();
        app.lifecycle_command(super::super::lifecycle::command(
            "criterion.passed",
            &project_id,
            "task.test",
            1,
            "criterion.passed",
            json!({"criterion_id":"criterion.task.test.c01","reviewer":"reviewer","evidence_refs":["test:native-flow"]}),
        ))
        .unwrap();

        let bundle = repository.load_bundle("task.test").unwrap();
        assert_eq!(bundle.task_timeline["blocker_details"], json!([]));
        assert_eq!(
            bundle.project_overview["active_tasks"][0]["blocker_details"],
            json!([])
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn eventless_tasks_have_no_synthetic_plan_or_execution_counts() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-views-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub/tasks/task.test")).unwrap();
        fs::write(
            root.join(".vibehub/tasks/task.test/task.yaml"),
            "task_id: task.test\ntitle: Historical task\nintent: Remain read only\nphase: implement\nphase_status: active\n",
        )
        .unwrap();
        let bundle = V3ViewRepository::open(&root)
            .unwrap()
            .load_bundle("task.test")
            .unwrap();
        assert_eq!(bundle.plan_graph["plan_version"], 0);
        assert_eq!(bundle.plan_graph["nodes"], json!([]));
        assert_eq!(bundle.plan_graph["scheduling_edges"], json!([]));
        assert_eq!(bundle.plan_graph["execution"]["planned_sessions"], 0);
        assert_eq!(bundle.plan_graph["execution"]["observed_sessions"], 0);
        assert_eq!(bundle.plan_graph["execution"]["planned_worktrees"], 0);
        assert_eq!(bundle.plan_graph["execution"]["observed_worktrees"], 0);
        assert_eq!(
            bundle.plan_graph["warnings"][0]["code"],
            "V3_PLAN_NOT_RECORDED"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn plan_edges_and_execution_counts_are_derived_from_events() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-views-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        super::super::initialize_v3(&root).unwrap();
        let created = create_v3_task(
            &root,
            V3TaskCreateRequest {
                title: "Build projection".to_owned(),
                intent: "Project real plan state".to_owned(),
                acceptance_criteria: vec!["Projection is factual".to_owned()],
            },
        )
        .unwrap();
        let repo = V3ViewRepository::open(&root).unwrap();
        let project_id = repo.project_id();
        let app = V3ApplicationService::open(&root).unwrap();
        app.lifecycle_command(super::super::lifecycle::command(
            "plan.node_added",
            &project_id,
            &created.task_id,
            created.lifecycle_version,
            "plan.second",
            json!({"node_id":"node.second","title":"Second","goal":"Continue","scope":[],"dependencies":[created.initial_node_id]}),
        ))
        .unwrap();
        app.session_open(
            &project_id,
            &created.task_id,
            "session.one",
            "test",
            0,
            "session.open",
        )
        .unwrap();
        app.orchestration_command(OrchestrationCommand {
            event_type: "worktree.planned".to_owned(),
            project_id: project_id.clone(),
            task_id: created.task_id.clone(),
            node_id: "node.second".to_owned(),
            worktree_id: "worktree.one".to_owned(),
            session_id: None,
            lease_id: None,
            operation_id: Some("operation.plan".to_owned()),
            eligibility_digest: "a".repeat(64),
            lease_generation: None,
            actor: "test".to_owned(),
            expected_version: 0,
            idempotency_key: "worktree.plan".to_owned(),
            evidence_grade: Some(EvidenceGrade::HardObserved),
            payload: json!({}),
        })
        .unwrap();
        let bundle = repo.load_bundle(&created.task_id).unwrap();
        assert_eq!(bundle.plan_graph["plan_version"], 4);
        assert_eq!(
            bundle.plan_graph["scheduling_edges"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(bundle.plan_graph["execution"]["planned_sessions"], 0);
        assert_eq!(bundle.plan_graph["execution"]["observed_sessions"], 1);
        assert_eq!(bundle.plan_graph["execution"]["planned_worktrees"], 1);
        assert_eq!(bundle.plan_graph["execution"]["observed_worktrees"], 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resolves_the_canonical_current_task_pointer_file() {
        let root = std::env::temp_dir().join(format!("vibehub-v3-pointer-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub/tasks/task.test")).unwrap();
        fs::write(
            root.join(".vibehub/tasks/task.test/task.yaml"),
            "task_id: task.test\ntitle: Test task\nintent: Verify pointer\nphase: implement\nphase_status: active\n",
        )
        .unwrap();
        fs::write(
            root.join(".vibehub/tasks/current"),
            "schema_version: 1\nkind: current_task_pointer\ntask_id: task.test\npath: .vibehub/tasks/task.test\nupdated_at: 2026-07-12T00:00:00Z\nupdated_by: vibehub\n",
        )
        .unwrap();

        let repository = V3ViewRepository::open(&root).unwrap();
        assert_eq!(repository.current_task_id().unwrap(), "task.test");

        fs::remove_dir_all(root).unwrap();
    }
}

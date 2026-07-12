use super::application::canonical_criterion_id;
use super::lifecycle::{fold_task, CriterionState, TaskLifecycleProjection};
use super::project_intelligence::{GitState, NodeKind, ProjectIndexService, ProjectPage};
use super::{V3Error, V3ErrorCategory, V3EventEnvelope, V3EventStore};
use crate::vibehub::current::resolve_current_task;
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

const MODEL_VERSION: &str = "v3-core-1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct V3ViewBundle {
    pub project_overview: Value,
    pub project_structure: Value,
    pub task_timeline: Value,
    pub plan_graph: Value,
    pub node_brief: Value,
}

#[derive(Debug, Clone)]
pub struct V3ViewRepository {
    root: PathBuf,
    store: V3EventStore,
}

#[derive(Debug, Deserialize)]
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
        if pointer_path.is_dir() {
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
            return Ok(task.task_id);
        }

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
        let has_lifecycle = events.iter().any(|event| event.aggregate_id == task_id);
        let generated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let criteria = criteria(&task, &events, &lifecycle, &generated_at);
        let evidence_refs = evidence_refs(&task, &events, &generated_at);
        let warnings = vec![json!({
            "code": "V3_ARCHITECTURE_INDEX_PENDING",
            "severity": "info",
            "message_key": "v3.warning.architecture_index_pending",
            "details": {"owner_milestone": "M3"},
            "evidence_refs": evidence_refs
        })];
        let native_root = native_path(&self.root);
        let sessions = sessions_for_task(&events, task_id);
        let opened_sessions = sessions.iter().filter(|session| session.1).count();
        let closed_sessions = sessions.iter().filter(|session| session.2).count();
        let task_state = if has_lifecycle {
            view_task_state(&lifecycle.state)
        } else {
            task_state(&task)
        };
        let index_page = ProjectIndexService::open(&self.root)
            .and_then(|service| service.first_page())
            .ok();
        let indexed_files = index_page
            .as_ref()
            .map(|page| page.snapshot.indexed_files)
            .unwrap_or(0);
        let architecture_modules = index_page
            .as_ref()
            .map(|page| {
                page.snapshot
                    .nodes
                    .iter()
                    .filter(|node| node.kind == NodeKind::Directory)
                    .count()
            })
            .unwrap_or(0);
        let declared_docs = index_page
            .as_ref()
            .map(|page| {
                page.snapshot
                    .analyzer_findings
                    .iter()
                    .filter(|finding| finding.analyzer == "documentation")
                    .count()
            })
            .unwrap_or(0);

        let project_overview = json!({
            "schema_version": "1.0", "project_id": project_id, "name": self.root.file_name().and_then(|v| v.to_str()).unwrap_or("Project"),
            "root": native_root, "generated_at": generated_at, "model_version": MODEL_VERSION,
            "freshness": "fresh", "completeness": "partial",
            "repository": {"state": if self.root.join(".git").exists() {"available"} else {"not_repository"}, "branch": Value::Null, "head": Value::Null, "dirty": Value::Null, "worktree_count": 1},
            "model": {"state": if index_page.is_some() {"ready"} else {"error"}, "last_evidence_at": generated_at, "generator_version": index_page.as_ref().map(|page| page.snapshot.model_version.as_str()).unwrap_or(MODEL_VERSION), "indexed_files": indexed_files},
            "architecture": {"declared_docs": declared_docs, "modules": architecture_modules, "relationships": architecture_modules, "confidence": if index_page.is_some() {1.0} else {0.0}, "evidence_refs": evidence_refs},
            "active_tasks": [{"task_id": task.task_id, "title": task.title, "state": task_state, "risk_level": risk_level(&events, task_id), "criteria": criteria, "active_sessions": opened_sessions.saturating_sub(closed_sessions)}],
            "protocol_coverage": {"state": protocol_coverage(opened_sessions, closed_sessions), "opened_sessions": opened_sessions, "closed_sessions": closed_sessions, "gaps": opened_sessions.saturating_sub(closed_sessions)},
            "evidence_refs": evidence_refs, "warnings": warnings, "errors": []
        });

        let project_structure = project_structure_view(
            &self.root,
            &project_id,
            &generated_at,
            &evidence_refs,
            index_page,
        );

        let lanes: Vec<Value> = sessions
            .iter()
            .map(|(session_id, opened, closed)| {
                json!({
                    "lane_id": session_id, "kind": "session", "label": session_id,
                    "state": if *closed {"closed"} else if *opened {"active"} else {"gapped"}
                })
            })
            .collect();
        let timeline_events: Vec<Value> = events
            .iter()
            .filter(|event| event.task_id.0 == task_id)
            .map(timeline_event)
            .collect();
        let task_timeline = json!({
            "schema_version": "1.0", "project_id": project_id, "task_id": task.task_id, "title": task.title, "state": task_state,
            "generated_at": generated_at, "model_version": MODEL_VERSION, "freshness": "fresh", "completeness": "complete",
            "criteria": criteria, "completion": completion_view(&lifecycle), "lanes": lanes, "events": timeline_events, "window": page(events.len()),
            "evidence_refs": evidence_refs, "warnings": [], "errors": []
        });

        let fallback_node_id = format!("node.{}", task.phase.replace('_', "-"));
        let graph_nodes: Vec<Value> = if lifecycle.nodes.is_empty() {
            vec![
                json!({"node_id": fallback_node_id, "title": task.title, "goal": task.intent, "state": node_state(&task), "readiness": "ready", "block_reasons": [], "scope": [], "criterion_ids": criteria.iter().filter_map(|criterion| criterion.get("criterion_id").cloned()).collect::<Vec<_>>() }),
            ]
        } else {
            lifecycle.nodes.values().map(|node| json!({
                "node_id": node.node_id, "title": node.title, "goal": node.goal, "state": view_node_state(&node.state),
                "readiness": if node.state == "blocked" || node.state == "failed" {"blocked"} else if node.dependencies.iter().all(|dependency| lifecycle.nodes.get(dependency).is_some_and(|value| value.state == "completed")) {"ready"} else {"blocked"},
                "block_reasons": if node.state == "blocked" {vec!["lifecycle.blocked"]} else {Vec::<&str>::new()},
                "scope": node.scope, "criterion_ids": lifecycle.criteria.keys().cloned().collect::<Vec<_>>()
            })).collect()
        };
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
            .unwrap_or(&fallback_node_id)
            .to_owned();
        let plan_graph = json!({
            "schema_version": "1.0", "project_id": project_id, "task_id": task.task_id, "plan_version": lifecycle.version + 1,
            "generated_at": generated_at, "model_version": MODEL_VERSION, "freshness": "fresh", "completeness": "partial", "graph_state": "valid",
            "nodes": graph_nodes,
            "scheduling_edges": scheduling_edges, "trace_relations": trace_relations,
            "execution": {"planned_sessions": 1, "observed_sessions": sessions.len(), "planned_worktrees": 1, "observed_worktrees": 1},
            "evidence_refs": evidence_refs, "warnings": warnings, "errors": []
        });

        let node_brief = json!({
            "schema_version": "1.0", "project_id": project_id, "task_id": task.task_id, "node_id": node_id,
            "generated_at": generated_at, "model_version": MODEL_VERSION, "freshness": "fresh", "completeness": "partial",
            "goal": lifecycle.nodes.get(&node_id).map(|node| node.goal.as_str()).unwrap_or(&task.intent),
            "scope": lifecycle.nodes.get(&node_id).map(|node| node.scope.clone()).unwrap_or_default(), "non_scope": ["M5-M6 milestone-owned capabilities"],
            "dependencies": task.dependencies.iter().map(|dependency| normalize_dependency_id(dependency)).collect::<Vec<_>>(),
            "accepted_decisions": ["JSON Schema 2020-12 remains the wire source of truth", "MCP and CLI share V3ApplicationService"],
            "research_summary": [], "criteria": criteria, "files": [native_root],
            "validation_commands": ["npm run v3:contracts:check", "cargo test --workspace"], "state": lifecycle.nodes.get(&node_id).map(|node| view_node_state(&node.state)).unwrap_or_else(|| node_state(&task)), "next_intent": task.phase,
            "budget": {"max_tokens": 8000, "estimated_tokens": 1000, "truncated_sections": []},
            "source_versions": {"view_model": MODEL_VERSION}, "protocol_coverage": protocol_coverage(opened_sessions, closed_sessions),
            "evidence_refs": evidence_refs, "warnings": warnings, "errors": []
        });

        Ok(V3ViewBundle {
            project_overview,
            project_structure,
            task_timeline,
            plan_graph,
            node_brief,
        })
    }

    pub fn load_project_structure_page(
        &self,
        relative_dir: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<Value, V3Error> {
        let page = ProjectIndexService::open(&self.root)
            .and_then(|service| service.page(relative_dir, cursor, limit))
            .map_err(|error| {
                V3Error::new(
                    "V3_PROJECT_INDEX_QUERY_FAILED",
                    V3ErrorCategory::Validation,
                    true,
                    error.to_string(),
                )
            })?;
        let generated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        Ok(project_structure_view(
            &self.root,
            &self.project_id(),
            &generated_at,
            &[],
            Some(page),
        ))
    }

    pub fn search_project_structure(&self, query: &str, limit: usize) -> Result<Value, V3Error> {
        let page = ProjectIndexService::open(&self.root)
            .and_then(|service| service.search(query, limit))
            .map_err(|error| {
                V3Error::new(
                    "V3_PROJECT_INDEX_SEARCH_FAILED",
                    V3ErrorCategory::Validation,
                    true,
                    error.to_string(),
                )
            })?;
        let generated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        Ok(project_structure_view(
            &self.root,
            &self.project_id(),
            &generated_at,
            &[],
            Some(page),
        ))
    }

    fn read_task(&self, task_id: &str) -> Result<TaskDocument, V3Error> {
        let path = self
            .root
            .join(".vibehub/tasks")
            .join(task_id)
            .join("task.yaml");
        let content = fs::read_to_string(path).map_err(internal("V3_TASK_NOT_FOUND"))?;
        serde_yaml::from_str(&content).map_err(|error| {
            V3Error::new(
                "V3_TASK_INVALID",
                V3ErrorCategory::CorruptLog,
                false,
                error.to_string(),
            )
        })
    }
}

fn project_structure_view(
    root: &Path,
    project_id: &str,
    generated_at: &str,
    evidence_refs: &[Value],
    page_result: Option<ProjectPage>,
) -> Value {
    let Some(result) = page_result else {
        return json!({
            "schema_version": "1.0", "project_id": project_id, "generated_at": generated_at, "model_version": MODEL_VERSION,
            "freshness": "unavailable", "completeness": "unknown", "index_state": "error",
            "nodes": [], "edges": [], "unsupported_analyzers": ["cargo", "npm", "python", "imports"],
            "page": {"cursor": Value::Null, "next_cursor": Value::Null, "limit": 200, "returned": 0, "total_estimate": Value::Null, "truncated": false, "truncation_reason": "none", "model_version": MODEL_VERSION},
            "evidence_refs": evidence_refs, "warnings": [],
            "errors": [{"code": "PI_INDEX_UNAVAILABLE", "category": "internal", "recoverable": true, "message_key": "v3.error.project_index_unavailable", "details": {}, "evidence_refs": evidence_refs}]
        });
    };
    let model_version = result.snapshot.model_version.clone();
    let nodes: Vec<Value> = result.snapshot.nodes.iter().map(|node| {
        let full_path = if node.relative_path == "." { root.to_path_buf() } else { root.join(&node.relative_path) };
        json!({
            "node_id": node.node_id, "parent_id": node.parent_id, "name": node.name,
            "kind": match node.kind { NodeKind::Root => "root", NodeKind::Directory => "directory", NodeKind::File => "file" },
            "path": native_path(&full_path),
            "git_state": match node.git_state { GitState::Clean => "clean", GitState::Modified => "modified", GitState::Added => "added", GitState::Deleted => "deleted", GitState::Ignored => "ignored", GitState::Unknown => "unknown" },
            "module_id": Value::Null, "ide_target": if node.kind == NodeKind::File { Some(node.relative_path.clone()) } else { None }, "evidence_refs": evidence_refs
        })
    }).collect();
    let edges: Vec<Value> = result.snapshot.nodes.iter().filter_map(|node| node.parent_id.as_ref().map(|parent| json!({
        "edge_id": format!("contains.{}.{}", parent, node.node_id), "from_node_id": parent, "to_node_id": node.node_id,
        "kind": "contains", "source_kind": "filesystem", "confidence": 1.0, "evidence_refs": evidence_refs
    }))).collect();
    let index_warnings: Vec<Value> = result.snapshot.warnings.iter().map(|warning| json!({
        "code": warning.code, "severity": "warning", "message_key": "v3.warning.project_index_gap",
        "details": {"path": warning.path, "message": warning.message}, "evidence_refs": evidence_refs
    })).collect();
    let returned = nodes.len();
    let truncated = result.next_cursor.is_some();
    let supported: std::collections::BTreeSet<&str> = result
        .snapshot
        .analyzer_findings
        .iter()
        .map(|finding| finding.analyzer.as_str())
        .collect();
    let imports_supported = supported.contains("rust_imports")
        || supported.contains("javascript_imports")
        || supported.contains("python_imports");
    let unsupported: Vec<&str> = ["cargo", "npm", "python", "imports"]
        .into_iter()
        .filter(|analyzer| {
            if *analyzer == "imports" {
                !imports_supported
            } else {
                !supported.contains(analyzer)
            }
        })
        .collect();
    json!({
        "schema_version": "1.0", "project_id": project_id, "generated_at": result.snapshot.generated_at, "model_version": model_version,
        "freshness": "fresh", "completeness": "partial", "index_state": "ready",
        "nodes": nodes, "edges": edges, "unsupported_analyzers": unsupported,
        "page": {"cursor": result.cursor, "next_cursor": result.next_cursor, "limit": result.limit, "returned": returned, "total_estimate": result.total_estimate, "truncated": truncated, "truncation_reason": if truncated {"page_limit"} else {"none"}, "model_version": model_version},
        "evidence_refs": evidence_refs, "warnings": index_warnings, "errors": []
    })
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
    json!({"platform": platform, "native": native, "display": native, "identity_key": format!("{platform}:{native}"), "path_kind": "absolute", "accessible": true})
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

fn view_task_state(state: &str) -> &str {
    match state {
        "completion_pending" => "review",
        "planned" | "active" | "blocked" | "review" | "completed" | "cancelled" => state,
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

fn protocol_coverage(opened: usize, closed: usize) -> &'static str {
    if opened == 0 {
        "unknown"
    } else if opened == closed {
        "complete"
    } else {
        "gapped"
    }
}

fn internal(code: &'static str) -> impl FnOnce(std::io::Error) -> V3Error {
    move |error| V3Error::new(code, V3ErrorCategory::Internal, true, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v3::V3ApplicationService;
    use std::fs;
    use uuid::Uuid;

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

use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Implementation, ListResourcesResult, PaginatedRequestParams,
        ReadResourceRequestParams, ReadResourceResult, Resource, ResourceContents,
        ServerCapabilities, ServerInfo,
    },
    schemars, tool, tool_handler, tool_router, ServerHandler, ServiceExt,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use vibehub_core::v3::{
    PlanAddNodeCommand, PlanCommandIdentity, PlanSetDependenciesCommand, PlanSetStateCommand,
    V3ApplicationService, V3Error, V3ErrorCategory, V3ViewRepository,
};

const RESOURCE_PREFIX: &str = "vibehub://v3/1.0";

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct SessionWrite {
    project_id: String,
    task_id: String,
    session_id: String,
    actor: String,
    #[serde(default)]
    expected_version: Option<u64>,
    #[serde(default)]
    idempotency_key: Option<String>,
    #[serde(default)]
    working_directory: Option<String>,
    #[serde(default)]
    node_id: Option<String>,
    #[serde(default)]
    worktree_id: Option<String>,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    provider_session_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct EventLogWrite {
    project_id: String,
    task_id: String,
    session_id: String,
    actor: String,
    #[serde(default)]
    expected_version: Option<u64>,
    #[serde(default)]
    idempotency_key: Option<String>,
    #[schemars(description = "Supported values are progress and risk")]
    kind: String,
    #[serde(default)]
    details: Value,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct AgentResultWrite {
    project_id: String,
    task_id: String,
    session_id: String,
    actor: String,
    #[serde(default)]
    expected_version: Option<u64>,
    #[serde(default)]
    idempotency_key: Option<String>,
    result_id: String,
    #[serde(default)]
    node_id: Option<String>,
    #[serde(default)]
    details: Value,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct PlanWriteScope {
    project_id: String,
    task_id: String,
    actor: String,
    #[serde(default)]
    expected_version: Option<u64>,
    #[serde(default)]
    idempotency_key: Option<String>,
}

fn resolve_and_append<T, F>(
    app: &V3ApplicationService,
    project_id: &str,
    aggregate_id: &str,
    expected_version: Option<u64>,
    idempotency_key: Option<String>,
    mut append: F,
) -> Result<T, V3Error>
where
    F: FnMut(u64, &str) -> Result<T, V3Error>,
{
    let auto_version = expected_version.is_none();
    let key = idempotency_key.unwrap_or_else(|| format!("auto.{}", Uuid::new_v4()));
    let version = expected_version.unwrap_or(app.aggregate_version(project_id, aggregate_id)?);
    match append(version, &key) {
        Err(error) if auto_version && error.category == V3ErrorCategory::VersionConflict => {
            let current_version = app.aggregate_version(project_id, aggregate_id)?;
            append(current_version, &key)
        }
        result => result,
    }
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct PlanNodeAddWrite {
    #[serde(flatten)]
    identity: PlanWriteScope,
    node_id: String,
    title: String,
    goal: String,
    #[serde(default)]
    scope: Vec<String>,
    #[serde(default)]
    dependencies: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct PlanDependenciesSetWrite {
    #[serde(flatten)]
    scope: PlanWriteScope,
    node_id: String,
    dependencies: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct PlanNodeStateSetWrite {
    #[serde(flatten)]
    scope: PlanWriteScope,
    node_id: String,
    #[schemars(
        description = "Target state: ready, active, blocked, completed, failed, or cancelled"
    )]
    state: String,
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
struct ToolResponse {
    ok: bool,
    result: Value,
}

#[derive(Debug, Clone)]
pub struct V3McpServer {
    project_root: PathBuf,
    app: V3ApplicationService,
    views: V3ViewRepository,
    project_id: String,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl V3McpServer {
    fn open(project_root: impl AsRef<Path>) -> Result<Self, V3Error> {
        let project_root = project_root.as_ref().to_path_buf();
        let views = V3ViewRepository::open(&project_root)?;
        let project_id = views.project_id();
        // Validate that a current V3 task exists at startup, but resolve it again
        // for every resource request so a long-lived MCP process cannot advertise
        // or read a stale task after the current pointer changes.
        let _ = views.current_task_id()?;
        Ok(Self {
            app: V3ApplicationService::open(&project_root)?,
            views,
            project_id,
            project_root,
            tool_router: Self::tool_router(),
        })
    }

    #[tool(
        description = "Open a VibeHub V3 agent session; expected_version and idempotency_key are optional and auto-resolved when omitted; explicitly provided values are strictly validated"
    )]
    fn session_open(&self, Parameters(input): Parameters<SessionWrite>) -> CallToolResult {
        let SessionWrite {
            project_id,
            task_id,
            session_id,
            actor,
            expected_version,
            idempotency_key,
            working_directory,
            node_id,
            worktree_id,
            provider,
            provider_session_id,
        } = input;
        self.tool_result(resolve_and_append(
            &self.app,
            &project_id,
            &session_id,
            expected_version,
            idempotency_key,
            |version, key| {
                self.app.session_open_with_context_and_provider(
                    &project_id,
                    &task_id,
                    &session_id,
                    &actor,
                    version,
                    key,
                    working_directory.clone(),
                    node_id.clone(),
                    worktree_id.clone(),
                    provider.clone(),
                    provider_session_id.clone(),
                )
            },
        ))
    }

    #[tool(
        description = "Record progress or risk evidence; expected_version and idempotency_key are optional and auto-resolved when omitted; explicitly provided values are strictly validated"
    )]
    fn event_log(&self, Parameters(input): Parameters<EventLogWrite>) -> CallToolResult {
        let EventLogWrite {
            project_id,
            task_id,
            session_id,
            actor,
            expected_version,
            idempotency_key,
            kind,
            details,
        } = input;
        if !matches!(kind.as_str(), "progress" | "risk") {
            return tool_error(json!({
                "code": "V3_VALIDATION_ERROR",
                "category": "validation",
                "retryable": false,
                "message": "kind must be progress or risk"
            }));
        }
        self.tool_result(resolve_and_append(
            &self.app,
            &project_id,
            &session_id,
            expected_version,
            idempotency_key,
            |version, key| {
                self.app.event_log(
                    &kind,
                    &project_id,
                    &task_id,
                    &session_id,
                    &actor,
                    version,
                    key,
                    details.clone(),
                )
            },
        ))
    }

    #[tool(
        description = "Record an Agent execution or evaluation result; expected_version and idempotency_key are optional and auto-resolved when omitted; explicitly provided values are strictly validated"
    )]
    fn agent_result_record(
        &self,
        Parameters(input): Parameters<AgentResultWrite>,
    ) -> CallToolResult {
        let AgentResultWrite {
            project_id,
            task_id,
            session_id,
            actor,
            expected_version,
            idempotency_key,
            result_id,
            node_id,
            details,
        } = input;
        self.tool_result(resolve_and_append(
            &self.app,
            &project_id,
            &session_id,
            expected_version,
            idempotency_key,
            |version, key| {
                self.app.agent_result_record(
                    &project_id,
                    &task_id,
                    &session_id,
                    &actor,
                    version,
                    key,
                    &result_id,
                    node_id.clone(),
                    details.clone(),
                )
            },
        ))
    }

    #[tool(
        description = "Close a VibeHub V3 agent session; expected_version and idempotency_key are optional and auto-resolved when omitted; explicitly provided values are strictly validated"
    )]
    fn session_close(&self, Parameters(input): Parameters<SessionWrite>) -> CallToolResult {
        let SessionWrite {
            project_id,
            task_id,
            session_id,
            actor,
            expected_version,
            idempotency_key,
            ..
        } = input;
        self.tool_result(resolve_and_append(
            &self.app,
            &project_id,
            &session_id,
            expected_version,
            idempotency_key,
            |version, key| {
                self.app
                    .session_close(&project_id, &task_id, &session_id, &actor, version, key)
            },
        ))
    }

    #[tool(
        description = "Add a node to the VibeHub V3 task plan; expected_version and idempotency_key are optional and auto-resolved when omitted; explicitly provided values are strictly validated"
    )]
    fn plan_node_add(&self, Parameters(input): Parameters<PlanNodeAddWrite>) -> CallToolResult {
        let PlanNodeAddWrite {
            identity,
            node_id,
            title,
            goal,
            scope,
            dependencies,
        } = input;
        let PlanWriteScope {
            project_id,
            task_id,
            actor,
            expected_version,
            idempotency_key,
        } = identity;
        self.tool_result(resolve_and_append(
            &self.app,
            &project_id,
            &task_id,
            expected_version,
            idempotency_key,
            |version, key| {
                self.app.plan_add_node(PlanAddNodeCommand {
                    identity: PlanCommandIdentity {
                        project_id: project_id.clone(),
                        task_id: task_id.clone(),
                        actor: actor.clone(),
                        expected_version: version,
                        idempotency_key: key.to_owned(),
                    },
                    node_id: node_id.clone(),
                    title: title.clone(),
                    goal: goal.clone(),
                    scope: scope.clone(),
                    dependencies: dependencies.clone(),
                })
            },
        ))
    }

    #[tool(
        description = "Replace dependencies for a VibeHub V3 plan node; expected_version and idempotency_key are optional and auto-resolved when omitted; explicitly provided values are strictly validated"
    )]
    fn plan_dependencies_set(
        &self,
        Parameters(input): Parameters<PlanDependenciesSetWrite>,
    ) -> CallToolResult {
        let PlanDependenciesSetWrite {
            scope,
            node_id,
            dependencies,
        } = input;
        let PlanWriteScope {
            project_id,
            task_id,
            actor,
            expected_version,
            idempotency_key,
        } = scope;
        self.tool_result(resolve_and_append(
            &self.app,
            &project_id,
            &task_id,
            expected_version,
            idempotency_key,
            |version, key| {
                self.app.plan_set_dependencies(PlanSetDependenciesCommand {
                    identity: PlanCommandIdentity {
                        project_id: project_id.clone(),
                        task_id: task_id.clone(),
                        actor: actor.clone(),
                        expected_version: version,
                        idempotency_key: key.to_owned(),
                    },
                    node_id: node_id.clone(),
                    dependencies: dependencies.clone(),
                })
            },
        ))
    }

    #[tool(
        description = "Transition a VibeHub V3 plan node state; expected_version and idempotency_key are optional and auto-resolved when omitted; explicitly provided values are strictly validated"
    )]
    fn plan_node_state_set(
        &self,
        Parameters(input): Parameters<PlanNodeStateSetWrite>,
    ) -> CallToolResult {
        let PlanNodeStateSetWrite {
            scope,
            node_id,
            state,
        } = input;
        let PlanWriteScope {
            project_id,
            task_id,
            actor,
            expected_version,
            idempotency_key,
        } = scope;
        self.tool_result(resolve_and_append(
            &self.app,
            &project_id,
            &task_id,
            expected_version,
            idempotency_key,
            |version, key| {
                self.app.plan_set_state(PlanSetStateCommand {
                    identity: PlanCommandIdentity {
                        project_id: project_id.clone(),
                        task_id: task_id.clone(),
                        actor: actor.clone(),
                        expected_version: version,
                        idempotency_key: key.to_owned(),
                    },
                    node_id: node_id.clone(),
                    state: state.clone(),
                })
            },
        ))
    }

    fn tool_result<T: Serialize>(&self, result: Result<T, V3Error>) -> CallToolResult {
        match result.and_then(|value| {
            serde_json::to_value(value).map_err(|error| {
                V3Error::new(
                    "V3_SERIALIZE_FAILED",
                    vibehub_core::v3::V3ErrorCategory::Internal,
                    false,
                    error.to_string(),
                )
            })
        }) {
            Ok(result) => tool_success(json!(ToolResponse { ok: true, result })),
            Err(error) => tool_error(serde_json::to_value(error).unwrap_or_else(
                |_| json!({"code": "V3_INTERNAL", "message": "failed to serialize error"}),
            )),
        }
    }

    fn resource_catalog(&self) -> Result<Vec<Resource>, V3Error> {
        let task_id = self.views.current_task_id()?;
        let resources = [
            (
                format!("projects/{}/overview", self.project_id),
                "project-overview",
                "M0-compatible project overview",
            ),
            (
                format!("projects/{}/structure", self.project_id),
                "project-structure",
                "M0-compatible project structure",
            ),
            (
                format!("tasks/{}/timeline", task_id),
                "task-timeline",
                "M0-compatible task timeline",
            ),
            (
                format!("tasks/{}/plan", task_id),
                "plan-graph",
                "M0-compatible task plan graph",
            ),
            (
                format!("tasks/{}/nodes/current/brief", task_id),
                "node-brief",
                "M0-compatible current node brief",
            ),
            (
                format!("projects/{}/projection", self.project_id),
                "projection",
                "Current event-derived recovery projection",
            ),
            (
                "diagnostics".to_owned(),
                "diagnostics",
                "Project-scoped V3 diagnostics",
            ),
        ]
        .into_iter()
        .map(|(path, name, description)| {
            Resource::new(format!("{RESOURCE_PREFIX}/{path}"), name)
                .with_description(description)
                .with_mime_type("application/json")
        })
        .collect::<Vec<_>>();
        Ok(resources)
    }

    fn read_resource_text(&self, uri: &str) -> Result<String, V3Error> {
        let suffix = uri
            .strip_prefix(&format!("{RESOURCE_PREFIX}/"))
            .ok_or_else(|| {
                V3Error::new(
                    "V3_RESOURCE_NOT_FOUND",
                    vibehub_core::v3::V3ErrorCategory::NotFound,
                    false,
                    format!("unsupported resource URI: {uri}"),
                )
            })?;
        let task_id = self.views.current_task_id()?;
        let bundle = self.views.load_bundle(&task_id)?;
        let value = match suffix {
            path if path == format!("projects/{}/overview", self.project_id) => {
                bundle.project_overview
            }
            path if path == format!("projects/{}/structure", self.project_id) => {
                bundle.project_structure
            }
            path if path == format!("tasks/{}/timeline", task_id) => bundle.task_timeline,
            path if path == format!("tasks/{}/plan", task_id) => bundle.plan_graph,
            path if path == format!("tasks/{}/nodes/current/brief", task_id) => bundle.node_brief,
            path if path == format!("projects/{}/projection", self.project_id) => {
                serde_json::to_value(self.app.rebuild(&self.project_id)?).map_err(|error| {
                    V3Error::new(
                        "V3_SERIALIZE_FAILED",
                        vibehub_core::v3::V3ErrorCategory::Internal,
                        false,
                        error.to_string(),
                    )
                })?
            }
            "diagnostics" => json!({
                "schema_version": "1.0",
                "server_version": env!("CARGO_PKG_VERSION"),
                "transport": "stdio",
                "project_root": self.project_root,
                "resource_namespace": RESOURCE_PREFIX
            }),
            _ => {
                return Err(V3Error::new(
                    "V3_RESOURCE_NOT_FOUND",
                    vibehub_core::v3::V3ErrorCategory::NotFound,
                    false,
                    format!("unsupported resource URI: {uri}"),
                ))
            }
        };
        serde_json::to_string_pretty(&value).map_err(|error| {
            V3Error::new(
                "V3_SERIALIZE_FAILED",
                vibehub_core::v3::V3ErrorCategory::Internal,
                false,
                error.to_string(),
            )
        })
    }
}

#[tool_handler]
impl ServerHandler for V3McpServer {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.capabilities = ServerCapabilities::builder()
            .enable_resources()
            .enable_tools()
            .build();
        info.server_info = Implementation::new("vibehub-v3", env!("CARGO_PKG_VERSION"))
            .with_title("VibeHub V3 MCP");
        info.instructions = Some(
            "Use versioned resources for reads; session_open, event_log, and session_close for recovery writes; and plan_node_add, plan_dependencies_set, and plan_node_state_set for plan writes. expected_version and idempotency_key may be omitted and are resolved by the server; explicitly provided values are checked strictly."
                .to_owned(),
        );
        info
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourcesResult, rmcp::ErrorData>> + Send + '_
    {
        let result = self
            .resource_catalog()
            .map(ListResourcesResult::with_all_items)
            .map_err(|error| {
                rmcp::ErrorData::invalid_params(error.message, Some(Value::Object(error.details)))
            });
        std::future::ready(result)
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl std::future::Future<Output = Result<ReadResourceResult, rmcp::ErrorData>> + Send + '_
    {
        let result = self.read_resource_text(&request.uri).map(|text| {
            ReadResourceResult::new(vec![
                ResourceContents::text(text, request.uri).with_mime_type("application/json")
            ])
        });
        std::future::ready(result.map_err(|error| {
            rmcp::ErrorData::invalid_params(error.message, Some(Value::Object(error.details)))
        }))
    }
}

pub fn run_stdio(project_root: &str) {
    let server = V3McpServer::open(project_root).unwrap_or_else(|error| {
        eprintln!(
            "{}",
            serde_json::to_string(&error).unwrap_or_else(|_| error.to_string())
        );
        std::process::exit(1);
    });
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|error| {
        eprintln!("failed to start MCP runtime: {error}");
        std::process::exit(1);
    });
    runtime.block_on(async move {
        match server
            .serve((tokio::io::stdin(), tokio::io::stdout()))
            .await
        {
            Ok(service) => {
                if let Err(error) = service.waiting().await {
                    eprintln!("MCP server stopped with error: {error}");
                    std::process::exit(1);
                }
            }
            Err(error) => {
                eprintln!("failed to start MCP stdio transport: {error}");
                std::process::exit(1);
            }
        }
    });
}

fn tool_success(value: Value) -> CallToolResult {
    CallToolResult::structured(value)
}

fn tool_error(value: Value) -> CallToolResult {
    CallToolResult::structured_error(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use uuid::Uuid;

    fn server() -> (PathBuf, V3McpServer) {
        let root = std::env::temp_dir().join(format!("vibehub-v3-mcp-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join(".vibehub/tasks/current")).unwrap();
        fs::create_dir_all(root.join(".vibehub/tasks/task.test")).unwrap();
        let task = "task_id: task.test\ntitle: MCP test\nintent: Verify MCP\nphase: implement\nphase_status: active\nacceptance_criteria: []\ndependencies: []\n";
        fs::write(root.join(".vibehub/tasks/current/task.yaml"), task).unwrap();
        fs::write(root.join(".vibehub/tasks/task.test/task.yaml"), task).unwrap();
        let server = V3McpServer::open(&root).unwrap();
        (root, server)
    }

    #[test]
    fn catalog_is_versioned_and_readable() {
        let (root, server) = server();
        let resources = server.resource_catalog().unwrap();
        assert_eq!(resources.len(), 7);
        assert!(resources
            .iter()
            .all(|resource| resource.uri.starts_with(RESOURCE_PREFIX)));
        let diagnostics = server
            .read_resource_text("vibehub://v3/1.0/diagnostics")
            .unwrap();
        assert!(diagnostics.contains("stdio"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn catalog_tracks_current_task_after_pointer_changes() {
        let (root, server) = server();
        fs::remove_dir_all(root.join(".vibehub/tasks/current")).unwrap();
        fs::create_dir_all(root.join(".vibehub/tasks/task.next")).unwrap();
        fs::write(
            root.join(".vibehub/tasks/task.next/task.yaml"),
            "task_id: task.next\ntitle: Next task\nintent: Verify fresh MCP resources\nphase: implement\nphase_status: active\nacceptance_criteria: []\ndependencies: []\n",
        )
        .unwrap();
        fs::write(
            root.join(".vibehub/tasks/current"),
            "schema_version: 1\nkind: current_task_pointer\ntask_id: task.next\npath: .vibehub/tasks/task.next\nupdated_at: 2026-07-14T00:00:00Z\nupdated_by: vibehub\n",
        )
        .unwrap();

        let resources = server.resource_catalog().unwrap();
        let timeline_uri = format!("{RESOURCE_PREFIX}/tasks/task.next/timeline");
        assert!(resources
            .iter()
            .any(|resource| resource.uri == timeline_uri));
        let timeline = server.read_resource_text(&timeline_uri).unwrap();
        assert!(timeline.contains("task.next"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn omitted_write_scope_is_resolved_and_versions_advance() {
        let (root, server) = server();
        let opened = server.session_open(Parameters(SessionWrite {
            project_id: "project.test".into(),
            task_id: "task.test".into(),
            session_id: "session.auto".into(),
            actor: "codex".into(),
            expected_version: None,
            idempotency_key: None,
            working_directory: None,
            node_id: None,
            worktree_id: None,
            provider: None,
            provider_session_id: None,
        }));
        assert!(!opened.is_error.unwrap_or(false));
        assert_eq!(
            opened.structured_content.unwrap()["result"]["status"],
            "appended"
        );
        assert_eq!(
            server
                .app
                .aggregate_version("project.test", "session.auto")
                .unwrap(),
            1
        );

        let first_log = server.event_log(Parameters(EventLogWrite {
            project_id: "project.test".into(),
            task_id: "task.test".into(),
            session_id: "session.auto".into(),
            actor: "codex".into(),
            expected_version: None,
            idempotency_key: None,
            kind: "progress".into(),
            details: json!({"message": "first"}),
        }));
        let second_log = server.event_log(Parameters(EventLogWrite {
            project_id: "project.test".into(),
            task_id: "task.test".into(),
            session_id: "session.auto".into(),
            actor: "codex".into(),
            expected_version: None,
            idempotency_key: None,
            kind: "progress".into(),
            details: json!({"message": "second"}),
        }));
        assert!(!first_log.is_error.unwrap_or(false));
        assert!(!second_log.is_error.unwrap_or(false));
        assert_eq!(
            first_log.structured_content.unwrap()["result"]["status"],
            "appended"
        );
        assert_eq!(
            second_log.structured_content.unwrap()["result"]["status"],
            "appended"
        );
        assert_eq!(
            server
                .app
                .aggregate_version("project.test", "session.auto")
                .unwrap(),
            3
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn tools_share_application_service_semantics() {
        let (root, server) = server();
        let input = SessionWrite {
            project_id: "project.test".into(),
            task_id: "task.test".into(),
            session_id: "session.test".into(),
            actor: "codex".into(),
            expected_version: Some(0),
            idempotency_key: Some("open.1".into()),
            working_directory: None,
            node_id: None,
            worktree_id: None,
            provider: None,
            provider_session_id: None,
        };
        let first = server.session_open(Parameters(input.clone()));
        let duplicate = server.session_open(Parameters(input));
        assert!(!first.is_error.unwrap_or(false));
        assert!(!duplicate.is_error.unwrap_or(false));
        assert_eq!(
            first.structured_content.unwrap()["result"]["status"],
            "appended"
        );
        assert_eq!(
            duplicate.structured_content.unwrap()["result"]["status"],
            "duplicate"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn session_open_preserves_provider_session_link() {
        let (root, server) = server();
        let project_id = server.project_id.clone();
        let opened = server.session_open(Parameters(SessionWrite {
            project_id,
            task_id: "task.test".into(),
            session_id: "session.workflow".into(),
            actor: "codex".into(),
            expected_version: Some(0),
            idempotency_key: Some("open.provider.1".into()),
            working_directory: Some("/repo/app".into()),
            node_id: None,
            worktree_id: None,
            provider: Some("codex".into()),
            provider_session_id: Some("provider-123".into()),
        }));
        assert!(!opened.is_error.unwrap_or(false));

        let bundle = server.views.load_bundle("task.test").unwrap();
        let session_event = bundle.task_timeline["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["session_id"] == "session.workflow")
            .expect("session event");
        assert_eq!(
            session_event["details"]["provider_session_id"],
            "provider-123"
        );
        assert_eq!(session_event["details"]["provider"], "codex");
        fs::remove_dir_all(root).unwrap();
    }
}

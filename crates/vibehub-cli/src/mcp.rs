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
use vibehub_core::v3::{V3ApplicationService, V3Error, V3ViewRepository};

const RESOURCE_PREFIX: &str = "vibehub://v3/1.0";

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct SessionWrite {
    project_id: String,
    task_id: String,
    session_id: String,
    actor: String,
    expected_version: u64,
    idempotency_key: String,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct EventLogWrite {
    project_id: String,
    task_id: String,
    session_id: String,
    actor: String,
    expected_version: u64,
    idempotency_key: String,
    #[schemars(description = "Supported values are progress and risk")]
    kind: String,
    #[serde(default)]
    details: Value,
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
    task_id: String,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl V3McpServer {
    fn open(project_root: impl AsRef<Path>) -> Result<Self, V3Error> {
        let project_root = project_root.as_ref().to_path_buf();
        let views = V3ViewRepository::open(&project_root)?;
        let project_id = views.project_id();
        let task_id = views.current_task_id()?;
        Ok(Self {
            app: V3ApplicationService::open(&project_root)?,
            views,
            project_id,
            task_id,
            project_root,
            tool_router: Self::tool_router(),
        })
    }

    #[tool(description = "Open a VibeHub V3 agent session")]
    fn session_open(&self, Parameters(input): Parameters<SessionWrite>) -> CallToolResult {
        self.tool_result(self.app.session_open(
            &input.project_id,
            &input.task_id,
            &input.session_id,
            &input.actor,
            input.expected_version,
            &input.idempotency_key,
        ))
    }

    #[tool(description = "Record progress or risk evidence for an open VibeHub V3 session")]
    fn event_log(&self, Parameters(input): Parameters<EventLogWrite>) -> CallToolResult {
        if !matches!(input.kind.as_str(), "progress" | "risk") {
            return tool_error(json!({
                "code": "V3_VALIDATION_ERROR",
                "category": "validation",
                "retryable": false,
                "message": "kind must be progress or risk"
            }));
        }
        self.tool_result(self.app.event_log(
            &input.kind,
            &input.project_id,
            &input.task_id,
            &input.session_id,
            &input.actor,
            input.expected_version,
            &input.idempotency_key,
            input.details,
        ))
    }

    #[tool(description = "Close a VibeHub V3 agent session")]
    fn session_close(&self, Parameters(input): Parameters<SessionWrite>) -> CallToolResult {
        self.tool_result(self.app.session_close(
            &input.project_id,
            &input.task_id,
            &input.session_id,
            &input.actor,
            input.expected_version,
            &input.idempotency_key,
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

    fn resource_catalog(&self) -> Vec<Resource> {
        [
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
                format!("tasks/{}/timeline", self.task_id),
                "task-timeline",
                "M0-compatible task timeline",
            ),
            (
                format!("tasks/{}/plan", self.task_id),
                "plan-graph",
                "M0-compatible task plan graph",
            ),
            (
                format!("tasks/{}/nodes/current/brief", self.task_id),
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
        .collect()
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
        let bundle = self.views.load_bundle(&self.task_id)?;
        let value = match suffix {
            path if path == format!("projects/{}/overview", self.project_id) => {
                bundle.project_overview
            }
            path if path == format!("projects/{}/structure", self.project_id) => {
                bundle.project_structure
            }
            path if path == format!("tasks/{}/timeline", self.task_id) => bundle.task_timeline,
            path if path == format!("tasks/{}/plan", self.task_id) => bundle.plan_graph,
            path if path == format!("tasks/{}/nodes/current/brief", self.task_id) => {
                bundle.node_brief
            }
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
            "Use versioned resources for reads and session_open, event_log, session_close for recovery writes. Every write requires stable identity, expected_version, and idempotency_key."
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
        std::future::ready(Ok(ListResourcesResult::with_all_items(
            self.resource_catalog(),
        )))
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
        let resources = server.resource_catalog();
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
    fn tools_share_application_service_semantics() {
        let (root, server) = server();
        let input = SessionWrite {
            project_id: "project.test".into(),
            task_id: "task.test".into(),
            session_id: "session.test".into(),
            actor: "codex".into(),
            expected_version: 0,
            idempotency_key: "open.1".into(),
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
}

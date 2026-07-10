use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use yunxi_agent_core::{AgentError, AgentResult};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub transport: McpTransport,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpTransport {
    Stdio { command: String, args: Vec<String> },
    Http { url: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct McpResource {
    pub server: String,
    pub uri: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct McpResourceRequest {
    pub server: String,
    pub uri: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct McpToolSpec {
    pub server: String,
    pub name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub input_schema: Value,
    pub destructive_hint: Option<bool>,
    pub open_world_hint: Option<bool>,
    pub requires_approval: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct McpToolInvocation {
    pub server: String,
    pub tool: String,
    pub arguments_json: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct McpToolResult {
    pub content: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpToolCallStatus {
    InProgress,
    Completed,
    Failed,
    Declined,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpLifecycleEvent {
    ServerConfigured {
        server: String,
    },
    ResourcesListed {
        server: String,
        count: usize,
    },
    ResourceRead {
        server: String,
        uri: String,
    },
    ToolStarted {
        call_id: Option<String>,
        server: String,
        tool: String,
    },
    ToolCompleted {
        call_id: Option<String>,
        server: String,
        tool: String,
        status: McpToolCallStatus,
    },
    ApprovalRequested {
        server: String,
        tool: String,
        question: String,
    },
    ElicitationRequested {
        server: String,
        message: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct McpToolApprovalParam {
    pub name: String,
    pub value: Value,
    pub display_name: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct McpToolApprovalTemplate {
    pub question: String,
    pub elicitation_message: String,
    pub tool_params: Option<Value>,
    pub tool_params_display: Vec<McpToolApprovalParam>,
}

pub fn render_mcp_tool_approval_template(
    server: &str,
    tool: &str,
    connector_name: Option<&str>,
    params: Option<Value>,
) -> McpToolApprovalTemplate {
    let connector = connector_name
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or(server);
    let question = format!("Allow {connector} to run MCP tool {tool}?");
    let tool_params_display = match params.as_ref() {
        Some(Value::Object(map)) => map
            .iter()
            .map(|(name, value)| McpToolApprovalParam {
                name: name.clone(),
                value: value.clone(),
                display_name: name.clone(),
            })
            .collect(),
        _ => Vec::new(),
    };
    McpToolApprovalTemplate {
        question: question.clone(),
        elicitation_message: question,
        tool_params: params,
        tool_params_display,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpAuthStatus {
    Unknown,
    Authenticated,
    Unauthenticated,
    NeedsUserAction,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct McpElicitationRequest {
    pub server: String,
    pub message: String,
    pub requested_schema: Option<Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct McpElicitationResponse {
    pub accepted: bool,
    pub content: Option<Value>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct McpServerSnapshot {
    pub config: Option<McpServerConfig>,
    pub resources: Vec<McpResource>,
    pub tools: Vec<McpToolSpec>,
    pub auth_status: Option<McpAuthStatus>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct McpRuntimeSnapshot {
    pub servers: BTreeMap<String, McpServerSnapshot>,
    pub plugins_available: bool,
    pub available_environment_ids: Vec<String>,
}

impl McpRuntimeSnapshot {
    pub fn register_server(&mut self, config: McpServerConfig) {
        let name = config.name.clone();
        self.servers.entry(name).or_default().config = Some(config);
    }

    pub fn register_tool(&mut self, spec: McpToolSpec) {
        self.servers
            .entry(spec.server.clone())
            .or_default()
            .tools
            .push(spec);
    }

    pub fn register_resource(&mut self, resource: McpResource) {
        self.servers
            .entry(resource.server.clone())
            .or_default()
            .resources
            .push(resource);
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct McpRuntimeSeed {
    #[serde(default)]
    pub snapshot: McpRuntimeSnapshot,
    #[serde(default)]
    pub resource_contents: Vec<McpResourceContentSeed>,
    #[serde(default)]
    pub tool_results: Vec<McpToolResultSeed>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct McpResourceContentSeed {
    pub server: String,
    pub uri: String,
    pub content: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct McpToolResultSeed {
    pub server: String,
    pub tool: String,
    pub content: String,
}

pub fn load_in_memory_runtime_seed(path: impl AsRef<Path>) -> AgentResult<InMemoryMcpRuntime> {
    let path = path.as_ref();
    let content = std::fs::read_to_string(path).map_err(|error| AgentError::Execution {
        message: format!(
            "failed to read MCP runtime seed {}: {error}",
            path.display()
        ),
    })?;
    let seed = serde_json::from_str::<McpRuntimeSeed>(&content).map_err(|error| {
        AgentError::Execution {
            message: format!(
                "failed to parse MCP runtime seed {}: {error}",
                path.display()
            ),
        }
    })?;
    let runtime = InMemoryMcpRuntime::new(seed.snapshot);
    for resource in seed.resource_contents {
        runtime.add_resource_content(resource.server, resource.uri, resource.content)?;
    }
    for result in seed.tool_results {
        runtime.add_tool_result(
            result.server,
            result.tool,
            McpToolResult {
                content: result.content,
            },
        )?;
    }
    Ok(runtime)
}

#[async_trait]
pub trait McpRuntime: Send + Sync {
    async fn list_resources(&self, server: &str) -> AgentResult<Vec<String>>;
    async fn list_tools(&self, server: &str) -> AgentResult<Vec<McpToolSpec>>;
    async fn read_resource(&self, request: McpResourceRequest) -> AgentResult<String>;
    async fn call_tool(&self, invocation: McpToolInvocation) -> AgentResult<McpToolResult>;
    async fn events(&self) -> AgentResult<Vec<McpLifecycleEvent>>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DecliningMcpRuntime;

#[async_trait]
impl McpRuntime for DecliningMcpRuntime {
    async fn list_resources(&self, _server: &str) -> AgentResult<Vec<String>> {
        Ok(Vec::new())
    }

    async fn list_tools(&self, _server: &str) -> AgentResult<Vec<McpToolSpec>> {
        Ok(Vec::new())
    }

    async fn read_resource(&self, request: McpResourceRequest) -> AgentResult<String> {
        Ok(format!(
            "MCP resource {} on {} is not wired in this runtime slice",
            request.uri, request.server
        ))
    }

    async fn call_tool(&self, invocation: McpToolInvocation) -> AgentResult<McpToolResult> {
        Ok(McpToolResult {
            content: format!(
                "MCP tool {} on {} is not wired in this runtime slice",
                invocation.tool, invocation.server
            ),
        })
    }

    async fn events(&self) -> AgentResult<Vec<McpLifecycleEvent>> {
        Ok(Vec::new())
    }
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryMcpRuntime {
    snapshot: Arc<Mutex<McpRuntimeSnapshot>>,
    resource_content: Arc<Mutex<BTreeMap<(String, String), String>>>,
    tool_results: Arc<Mutex<BTreeMap<(String, String), McpToolResult>>>,
    events: Arc<Mutex<Vec<McpLifecycleEvent>>>,
}

impl InMemoryMcpRuntime {
    pub fn new(snapshot: McpRuntimeSnapshot) -> Self {
        Self {
            snapshot: Arc::new(Mutex::new(snapshot)),
            resource_content: Arc::default(),
            tool_results: Arc::default(),
            events: Arc::default(),
        }
    }

    pub fn add_resource_content(
        &self,
        server: impl Into<String>,
        uri: impl Into<String>,
        content: impl Into<String>,
    ) -> AgentResult<()> {
        self.lock_resources()?
            .insert((server.into(), uri.into()), content.into());
        Ok(())
    }

    pub fn add_tool_result(
        &self,
        server: impl Into<String>,
        tool: impl Into<String>,
        result: McpToolResult,
    ) -> AgentResult<()> {
        self.lock_tool_results()?
            .insert((server.into(), tool.into()), result);
        Ok(())
    }

    fn snapshot_for_server(&self, server: &str) -> AgentResult<McpServerSnapshot> {
        self.lock_snapshot()?
            .servers
            .get(server)
            .cloned()
            .ok_or_else(|| AgentError::Execution {
                message: format!("MCP server is not configured: {server}"),
            })
    }

    fn emit(&self, event: McpLifecycleEvent) -> AgentResult<()> {
        self.lock_events()?.push(event);
        Ok(())
    }

    fn lock_snapshot(&self) -> AgentResult<std::sync::MutexGuard<'_, McpRuntimeSnapshot>> {
        self.snapshot.lock().map_err(|_| AgentError::Execution {
            message: "MCP snapshot lock was poisoned".to_string(),
        })
    }

    fn lock_resources(
        &self,
    ) -> AgentResult<std::sync::MutexGuard<'_, BTreeMap<(String, String), String>>> {
        self.resource_content
            .lock()
            .map_err(|_| AgentError::Execution {
                message: "MCP resource lock was poisoned".to_string(),
            })
    }

    fn lock_tool_results(
        &self,
    ) -> AgentResult<std::sync::MutexGuard<'_, BTreeMap<(String, String), McpToolResult>>> {
        self.tool_results.lock().map_err(|_| AgentError::Execution {
            message: "MCP tool result lock was poisoned".to_string(),
        })
    }

    fn lock_events(&self) -> AgentResult<std::sync::MutexGuard<'_, Vec<McpLifecycleEvent>>> {
        self.events.lock().map_err(|_| AgentError::Execution {
            message: "MCP event lock was poisoned".to_string(),
        })
    }
}

#[async_trait]
impl McpRuntime for InMemoryMcpRuntime {
    async fn list_resources(&self, server: &str) -> AgentResult<Vec<String>> {
        let snapshot = self.snapshot_for_server(server)?;
        self.emit(McpLifecycleEvent::ResourcesListed {
            server: server.to_string(),
            count: snapshot.resources.len(),
        })?;
        Ok(snapshot
            .resources
            .into_iter()
            .map(|resource| resource.uri)
            .collect())
    }

    async fn list_tools(&self, server: &str) -> AgentResult<Vec<McpToolSpec>> {
        Ok(self.snapshot_for_server(server)?.tools)
    }

    async fn read_resource(&self, request: McpResourceRequest) -> AgentResult<String> {
        self.snapshot_for_server(&request.server)?;
        self.emit(McpLifecycleEvent::ResourceRead {
            server: request.server.clone(),
            uri: request.uri.clone(),
        })?;
        Ok(self
            .lock_resources()?
            .get(&(request.server.clone(), request.uri.clone()))
            .cloned()
            .unwrap_or_default())
    }

    async fn call_tool(&self, invocation: McpToolInvocation) -> AgentResult<McpToolResult> {
        let snapshot = self.snapshot_for_server(&invocation.server)?;
        let Some(spec) = snapshot
            .tools
            .iter()
            .find(|spec| spec.name == invocation.tool)
        else {
            return Err(AgentError::Execution {
                message: format!(
                    "MCP tool {} is not registered on {}",
                    invocation.tool, invocation.server
                ),
            });
        };
        self.emit(McpLifecycleEvent::ToolStarted {
            call_id: None,
            server: invocation.server.clone(),
            tool: invocation.tool.clone(),
        })?;
        if spec.requires_approval {
            let params = invocation
                .arguments_json
                .as_deref()
                .and_then(|json| serde_json::from_str::<Value>(json).ok());
            let approval = render_mcp_tool_approval_template(
                &invocation.server,
                &invocation.tool,
                None,
                params,
            );
            self.emit(McpLifecycleEvent::ApprovalRequested {
                server: invocation.server.clone(),
                tool: invocation.tool.clone(),
                question: approval.question,
            })?;
        }
        let result = self
            .lock_tool_results()?
            .get(&(invocation.server.clone(), invocation.tool.clone()))
            .cloned()
            .unwrap_or_else(|| McpToolResult {
                content: format!(
                    "MCP tool {} on {} completed",
                    invocation.tool, invocation.server
                ),
            });
        self.emit(McpLifecycleEvent::ToolCompleted {
            call_id: None,
            server: invocation.server,
            tool: invocation.tool,
            status: McpToolCallStatus::Completed,
        })?;
        Ok(result)
    }

    async fn events(&self) -> AgentResult<Vec<McpLifecycleEvent>> {
        Ok(self.lock_events()?.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    #[tokio::test]
    async fn in_memory_runtime_lists_reads_and_calls_tools() {
        let mut snapshot = McpRuntimeSnapshot::default();
        snapshot.register_server(McpServerConfig {
            name: "local".to_string(),
            transport: McpTransport::Stdio {
                command: "node".to_string(),
                args: vec!["server.js".to_string()],
            },
            enabled: true,
        });
        snapshot.register_resource(McpResource {
            server: "local".to_string(),
            uri: "file://notes".to_string(),
            name: Some("notes".to_string()),
            description: None,
            mime_type: Some("text/plain".to_string()),
        });
        snapshot.register_tool(McpToolSpec {
            server: "local".to_string(),
            name: "echo".to_string(),
            title: Some("Echo".to_string()),
            description: None,
            input_schema: json!({"type": "object"}),
            destructive_hint: Some(false),
            open_world_hint: Some(false),
            requires_approval: false,
        });
        let runtime = InMemoryMcpRuntime::new(snapshot);
        runtime
            .add_resource_content("local", "file://notes", "hello")
            .expect("resource");
        runtime
            .add_tool_result(
                "local",
                "echo",
                McpToolResult {
                    content: "pong".to_string(),
                },
            )
            .expect("tool result");

        assert_eq!(
            runtime.list_resources("local").await.expect("resources"),
            vec!["file://notes".to_string()]
        );
        assert_eq!(
            runtime
                .read_resource(McpResourceRequest {
                    server: "local".to_string(),
                    uri: "file://notes".to_string(),
                })
                .await
                .expect("read"),
            "hello"
        );
        assert_eq!(
            runtime
                .call_tool(McpToolInvocation {
                    server: "local".to_string(),
                    tool: "echo".to_string(),
                    arguments_json: Some("{}".to_string()),
                })
                .await
                .expect("call")
                .content,
            "pong"
        );
        assert!(
            runtime
                .events()
                .await
                .expect("events")
                .iter()
                .any(|event| matches!(event, McpLifecycleEvent::ToolCompleted { .. }))
        );
    }

    #[test]
    fn approval_template_renders_params_for_consequential_tools() {
        let rendered = render_mcp_tool_approval_template(
            "apps",
            "create_event",
            Some("Calendar"),
            Some(json!({"title": "Roadmap", "calendar_id": "primary"})),
        );

        assert!(rendered.question.contains("Calendar"));
        assert_eq!(rendered.tool_params_display.len(), 2);
    }

    #[tokio::test]
    async fn loads_in_memory_runtime_seed_from_json_file() {
        let temp = TempDir::new().expect("temp dir");
        let seed_path = temp.path().join("mcp-runtime.json");
        std::fs::write(
            &seed_path,
            json!({
                "snapshot": {
                    "servers": {
                        "local": {
                            "config": {
                                "name": "local",
                                "transport": {"type": "stdio", "command": "fixture", "args": []},
                                "enabled": true
                            },
                            "resources": [
                                {
                                    "server": "local",
                                    "uri": "file://notes",
                                    "name": "notes",
                                    "description": null,
                                    "mime_type": "text/plain"
                                }
                            ],
                            "tools": [
                                {
                                    "server": "local",
                                    "name": "echo",
                                    "title": "Echo",
                                    "description": "fixture echo",
                                    "input_schema": {"type": "object"},
                                    "destructive_hint": false,
                                    "open_world_hint": false,
                                    "requires_approval": false
                                }
                            ],
                            "auth_status": "authenticated"
                        }
                    },
                    "plugins_available": false,
                    "available_environment_ids": []
                },
                "resource_contents": [
                    {"server": "local", "uri": "file://notes", "content": "hello"}
                ],
                "tool_results": [
                    {"server": "local", "tool": "echo", "content": "pong"}
                ]
            })
            .to_string(),
        )
        .expect("seed file");

        let runtime = load_in_memory_runtime_seed(seed_path).expect("runtime");

        assert_eq!(
            runtime
                .read_resource(McpResourceRequest {
                    server: "local".to_string(),
                    uri: "file://notes".to_string(),
                })
                .await
                .expect("resource"),
            "hello"
        );
        assert_eq!(
            runtime
                .call_tool(McpToolInvocation {
                    server: "local".to_string(),
                    tool: "echo".to_string(),
                    arguments_json: None,
                })
                .await
                .expect("tool")
                .content,
            "pong"
        );
    }
}

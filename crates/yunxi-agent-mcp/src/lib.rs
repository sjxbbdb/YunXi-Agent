use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::time::timeout;
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
pub struct McpJsonRpcRequest {
    pub id: String,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

impl McpJsonRpcRequest {
    pub fn initialize(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            method: "initialize".to_string(),
            params: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "yunxi-agent",
                    "version": env!("CARGO_PKG_VERSION")
                }
            })),
        }
    }

    pub fn list_tools(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            method: "tools/list".to_string(),
            params: Some(json!({})),
        }
    }

    pub fn call_tool(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: Option<Value>,
    ) -> Self {
        Self {
            id: id.into(),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": name.into(),
                "arguments": arguments.unwrap_or_else(|| json!({}))
            })),
        }
    }

    fn to_wire_json(&self) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": self.id,
            "method": self.method,
            "params": self.params.clone().unwrap_or_else(|| json!({}))
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct McpJsonRpcResponse {
    pub id: Option<String>,
    pub result: Option<Value>,
    pub error: Option<Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StdioMcpClient {
    pub command: String,
    pub args: Vec<String>,
    pub timeout_millis: u64,
}

impl StdioMcpClient {
    pub fn new(command: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            command: command.into(),
            args,
            timeout_millis: 10_000,
        }
    }

    pub async fn request_once(
        &self,
        request: McpJsonRpcRequest,
    ) -> AgentResult<McpJsonRpcResponse> {
        let mut child = Command::new(&self.command)
            .args(&self.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|error| AgentError::Execution {
                message: format!("failed to spawn MCP stdio server {}: {error}", self.command),
            })?;
        let mut stdin = child.stdin.take().ok_or_else(|| AgentError::Execution {
            message: "MCP stdio stdin was not available".to_string(),
        })?;
        let payload = serde_json::to_string(&request.to_wire_json()).map_err(|error| {
            AgentError::Execution {
                message: format!("failed to serialize MCP JSON-RPC request: {error}"),
            }
        })?;
        stdin
            .write_all(format!("{payload}\n").as_bytes())
            .await
            .map_err(|error| AgentError::Execution {
                message: format!("failed to write MCP JSON-RPC request: {error}"),
            })?;
        stdin
            .shutdown()
            .await
            .map_err(|error| AgentError::Execution {
                message: format!("failed to close MCP stdio stdin: {error}"),
            })?;

        let mut stdout = child.stdout.take().ok_or_else(|| AgentError::Execution {
            message: "MCP stdio stdout was not available".to_string(),
        })?;
        let mut output = String::new();
        timeout(
            Duration::from_millis(self.timeout_millis),
            stdout.read_to_string(&mut output),
        )
        .await
        .map_err(|_| AgentError::Execution {
            message: "MCP stdio request timed out".to_string(),
        })?
        .map_err(|error| AgentError::Execution {
            message: format!("failed to read MCP stdio response: {error}"),
        })?;
        let _ = child.wait().await;
        parse_json_rpc_response(&output)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct McpHttpRequest {
    pub url: String,
    pub body: Value,
    pub timeout_millis: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct McpHttpResponse {
    pub status: u16,
    pub body: String,
}

impl McpHttpResponse {
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

#[async_trait]
pub trait McpHttpTransport: Send + Sync {
    async fn send(&self, request: McpHttpRequest) -> AgentResult<McpHttpResponse>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReqwestMcpHttpTransport;

#[async_trait]
impl McpHttpTransport for ReqwestMcpHttpTransport {
    async fn send(&self, request: McpHttpRequest) -> AgentResult<McpHttpResponse> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(request.timeout_millis))
            .build()
            .map_err(|error| AgentError::Execution {
                message: format!("failed to build MCP HTTP client: {error}"),
            })?;
        let response = client
            .post(&request.url)
            .json(&request.body)
            .send()
            .await
            .map_err(|error| AgentError::Execution {
                message: format!("failed to send MCP HTTP request: {error}"),
            })?;
        let status = response.status().as_u16();
        let body = response
            .text()
            .await
            .map_err(|error| AgentError::Execution {
                message: format!("failed to read MCP HTTP response: {error}"),
            })?;
        Ok(McpHttpResponse { status, body })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixtureMcpHttpTransport {
    response: McpHttpResponse,
}

impl FixtureMcpHttpTransport {
    pub fn new(status: u16, body: impl Into<String>) -> Self {
        Self {
            response: McpHttpResponse {
                status,
                body: body.into(),
            },
        }
    }
}

#[async_trait]
impl McpHttpTransport for FixtureMcpHttpTransport {
    async fn send(&self, _request: McpHttpRequest) -> AgentResult<McpHttpResponse> {
        Ok(self.response.clone())
    }
}

#[derive(Clone, Debug)]
pub struct HttpMcpClient<T = ReqwestMcpHttpTransport> {
    pub url: String,
    pub timeout_millis: u64,
    transport: T,
}

impl HttpMcpClient<ReqwestMcpHttpTransport> {
    pub fn new(url: impl Into<String>) -> Self {
        Self::with_transport(url, ReqwestMcpHttpTransport)
    }
}

impl<T> HttpMcpClient<T>
where
    T: McpHttpTransport,
{
    pub fn with_transport(url: impl Into<String>, transport: T) -> Self {
        Self {
            url: url.into(),
            timeout_millis: 10_000,
            transport,
        }
    }

    pub async fn request_once(
        &self,
        request: McpJsonRpcRequest,
    ) -> AgentResult<McpJsonRpcResponse> {
        let response = self
            .transport
            .send(McpHttpRequest {
                url: self.url.clone(),
                body: request.to_wire_json(),
                timeout_millis: self.timeout_millis,
            })
            .await?;
        if !response.is_success() {
            return Err(AgentError::Execution {
                message: format!("MCP HTTP request failed with status {}", response.status),
            });
        }
        parse_json_rpc_response(&response.body)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct McpWorkspaceConfig {
    #[serde(default)]
    pub servers: Vec<McpServerConfig>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpSessionStatus {
    Configured,
    Initialized,
    Failed,
    Cancelled,
    Shutdown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct McpSessionState {
    pub server: String,
    pub status: McpSessionStatus,
    pub transport: McpTransport,
    pub capabilities: Option<Value>,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct McpSessionManager {
    sessions: Arc<Mutex<BTreeMap<String, McpSessionState>>>,
    events: Arc<Mutex<Vec<McpLifecycleEvent>>>,
}

impl McpSessionManager {
    pub fn from_configs(configs: impl IntoIterator<Item = McpServerConfig>) -> AgentResult<Self> {
        let manager = Self::default();
        for config in configs {
            manager.register(config)?;
        }
        Ok(manager)
    }

    pub fn from_workspace(cwd: impl AsRef<Path>) -> AgentResult<Option<Self>> {
        let configs = load_workspace_mcp_configs(cwd)?;
        if configs.is_empty() {
            Ok(None)
        } else {
            Self::from_configs(configs).map(Some)
        }
    }

    pub fn register(&self, config: McpServerConfig) -> AgentResult<()> {
        if !config.enabled {
            return Ok(());
        }
        let state = McpSessionState {
            server: config.name.clone(),
            status: McpSessionStatus::Configured,
            transport: config.transport,
            capabilities: None,
            last_error: None,
        };
        self.lock_sessions()?.insert(config.name.clone(), state);
        self.emit(McpLifecycleEvent::ServerConfigured {
            server: config.name,
        })?;
        Ok(())
    }

    pub async fn initialize(&self, server: &str) -> AgentResult<McpSessionState> {
        self.session(server)?;
        let response = self
            .request(
                server,
                McpJsonRpcRequest::initialize(format!("{server}:initialize")),
            )
            .await;
        let mut sessions = self.lock_sessions()?;
        let session = sessions
            .get_mut(server)
            .ok_or_else(|| AgentError::Execution {
                message: format!("MCP server is not configured: {server}"),
            })?;
        match response {
            Ok(response) if response.error.is_none() => {
                session.status = McpSessionStatus::Initialized;
                session.capabilities = response
                    .result
                    .as_ref()
                    .and_then(|value| value.get("capabilities").cloned())
                    .or(response.result);
                session.last_error = None;
            }
            Ok(response) => {
                session.status = McpSessionStatus::Failed;
                session.last_error = Some(
                    response
                        .error
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "MCP initialize failed".to_string()),
                );
            }
            Err(error) => {
                session.status = McpSessionStatus::Failed;
                session.last_error = Some(error.to_string());
            }
        }
        Ok(session.clone())
    }

    pub async fn shutdown(&self, server: &str) -> AgentResult<McpSessionState> {
        let mut sessions = self.lock_sessions()?;
        let session = sessions
            .get_mut(server)
            .ok_or_else(|| AgentError::Execution {
                message: format!("MCP server is not configured: {server}"),
            })?;
        session.status = McpSessionStatus::Shutdown;
        Ok(session.clone())
    }

    pub async fn shutdown_all(&self) -> AgentResult<Vec<McpSessionState>> {
        let servers = self.lock_sessions()?.keys().cloned().collect::<Vec<_>>();
        let mut states = Vec::new();
        for server in servers {
            states.push(self.shutdown(&server).await?);
        }
        Ok(states)
    }

    pub fn health_check(&self, server: &str) -> AgentResult<McpSessionState> {
        self.session(server)
    }

    pub fn cancel(&self, server: &str, reason: impl Into<String>) -> AgentResult<McpSessionState> {
        let mut sessions = self.lock_sessions()?;
        let session = sessions
            .get_mut(server)
            .ok_or_else(|| AgentError::Execution {
                message: format!("MCP server is not configured: {server}"),
            })?;
        session.status = McpSessionStatus::Cancelled;
        session.last_error = Some(format!("cancelled: {}", reason.into()));
        Ok(session.clone())
    }

    pub fn session_count(&self) -> AgentResult<usize> {
        Ok(self.lock_sessions()?.len())
    }

    pub fn snapshot(&self) -> AgentResult<Vec<McpSessionState>> {
        Ok(self.lock_sessions()?.values().cloned().collect())
    }

    async fn request(
        &self,
        server: &str,
        request: McpJsonRpcRequest,
    ) -> AgentResult<McpJsonRpcResponse> {
        let state = self.session(server)?;
        match state.transport {
            McpTransport::Stdio { command, args } => {
                StdioMcpClient::new(command, args)
                    .request_once(request)
                    .await
            }
            McpTransport::Http { url } => HttpMcpClient::new(url).request_once(request).await,
        }
    }

    fn session(&self, server: &str) -> AgentResult<McpSessionState> {
        self.lock_sessions()?
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

    fn lock_sessions(
        &self,
    ) -> AgentResult<std::sync::MutexGuard<'_, BTreeMap<String, McpSessionState>>> {
        self.sessions.lock().map_err(|_| AgentError::Execution {
            message: "MCP session manager lock was poisoned".to_string(),
        })
    }

    fn lock_events(&self) -> AgentResult<std::sync::MutexGuard<'_, Vec<McpLifecycleEvent>>> {
        self.events.lock().map_err(|_| AgentError::Execution {
            message: "MCP session event lock was poisoned".to_string(),
        })
    }
}

#[async_trait]
impl McpRuntime for McpSessionManager {
    async fn list_resources(&self, server: &str) -> AgentResult<Vec<String>> {
        let response = self
            .request(
                server,
                McpJsonRpcRequest {
                    id: format!("{server}:resources"),
                    method: "resources/list".to_string(),
                    params: Some(json!({})),
                },
            )
            .await?;
        if let Some(error) = response.error {
            return Err(AgentError::Execution {
                message: format!("MCP resources/list failed on {server}: {error}"),
            });
        }
        let resources = response
            .result
            .and_then(|value| value.get("resources").cloned())
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|value| {
                value
                    .get("uri")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
            })
            .collect::<Vec<_>>();
        self.emit(McpLifecycleEvent::ResourcesListed {
            server: server.to_string(),
            count: resources.len(),
        })?;
        Ok(resources)
    }

    async fn list_tools(&self, server: &str) -> AgentResult<Vec<McpToolSpec>> {
        let response = self
            .request(
                server,
                McpJsonRpcRequest::list_tools(format!("{server}:tools")),
            )
            .await?;
        if let Some(error) = response.error {
            return Err(AgentError::Execution {
                message: format!("MCP tools/list failed on {server}: {error}"),
            });
        }
        Ok(parse_mcp_tool_specs(server, response.result.as_ref()))
    }

    async fn read_resource(&self, request: McpResourceRequest) -> AgentResult<String> {
        let uri = request.uri.clone();
        let response = self
            .request(
                &request.server,
                McpJsonRpcRequest {
                    id: format!("{}:resource", request.server),
                    method: "resources/read".to_string(),
                    params: Some(json!({ "uri": uri })),
                },
            )
            .await?;
        if let Some(error) = response.error {
            return Err(AgentError::Execution {
                message: format!("MCP resources/read failed on {}: {error}", request.server),
            });
        }
        self.emit(McpLifecycleEvent::ResourceRead {
            server: request.server,
            uri: request.uri,
        })?;
        Ok(render_mcp_result_content(response.result.as_ref()))
    }

    async fn call_tool(&self, invocation: McpToolInvocation) -> AgentResult<McpToolResult> {
        self.emit(McpLifecycleEvent::ToolStarted {
            call_id: None,
            server: invocation.server.clone(),
            tool: invocation.tool.clone(),
        })?;
        let arguments = invocation
            .arguments_json
            .as_deref()
            .and_then(|json| serde_json::from_str::<Value>(json).ok());
        let response = self
            .request(
                &invocation.server,
                McpJsonRpcRequest::call_tool(
                    format!("{}:{}", invocation.server, invocation.tool),
                    invocation.tool.clone(),
                    arguments,
                ),
            )
            .await?;
        if let Some(error) = response.error {
            self.emit(McpLifecycleEvent::ToolCompleted {
                call_id: None,
                server: invocation.server.clone(),
                tool: invocation.tool.clone(),
                status: McpToolCallStatus::Failed,
            })?;
            return Err(AgentError::Execution {
                message: format!(
                    "MCP tools/call failed on {}.{}: {error}",
                    invocation.server, invocation.tool
                ),
            });
        }
        self.emit(McpLifecycleEvent::ToolCompleted {
            call_id: None,
            server: invocation.server,
            tool: invocation.tool,
            status: McpToolCallStatus::Completed,
        })?;
        Ok(McpToolResult {
            content: render_mcp_result_content(response.result.as_ref()),
        })
    }

    async fn events(&self) -> AgentResult<Vec<McpLifecycleEvent>> {
        Ok(self.lock_events()?.clone())
    }
}

pub fn load_workspace_mcp_configs(cwd: impl AsRef<Path>) -> AgentResult<Vec<McpServerConfig>> {
    let cwd = cwd.as_ref();
    let candidates = [
        cwd.join(".yunxi").join("mcp.json"),
        cwd.join(".yunxi").join("mcp-servers.json"),
        cwd.join(".mcp.json"),
    ];
    for path in candidates {
        if !path.is_file() {
            continue;
        }
        let content = std::fs::read_to_string(&path).map_err(|error| AgentError::Execution {
            message: format!("failed to read MCP config {}: {error}", path.display()),
        })?;
        if let Ok(config) = serde_json::from_str::<McpWorkspaceConfig>(&content) {
            return Ok(config.servers);
        }
        let servers = serde_json::from_str::<Vec<McpServerConfig>>(&content).map_err(|error| {
            AgentError::Execution {
                message: format!("failed to parse MCP config {}: {error}", path.display()),
            }
        })?;
        return Ok(servers);
    }
    Ok(Vec::new())
}

fn parse_mcp_tool_specs(server: &str, result: Option<&Value>) -> Vec<McpToolSpec> {
    result
        .and_then(|value| value.get("tools"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|tool| {
            let name = tool.get("name").and_then(Value::as_str)?.to_string();
            Some(McpToolSpec {
                server: server.to_string(),
                name,
                title: tool
                    .get("title")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                description: tool
                    .get("description")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                input_schema: tool
                    .get("inputSchema")
                    .or_else(|| tool.get("input_schema"))
                    .cloned()
                    .unwrap_or_else(|| json!({"type": "object"})),
                destructive_hint: tool
                    .pointer("/annotations/destructiveHint")
                    .and_then(Value::as_bool),
                open_world_hint: tool
                    .pointer("/annotations/openWorldHint")
                    .and_then(Value::as_bool),
                requires_approval: tool
                    .pointer("/annotations/destructiveHint")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            })
        })
        .collect()
}

fn render_mcp_result_content(result: Option<&Value>) -> String {
    let Some(result) = result else {
        return String::new();
    };
    if let Some(content) = result.get("content").and_then(Value::as_array) {
        let rendered = content
            .iter()
            .filter_map(|item| {
                item.get("text")
                    .and_then(Value::as_str)
                    .or_else(|| item.get("content").and_then(Value::as_str))
            })
            .collect::<Vec<_>>()
            .join("\n");
        if !rendered.is_empty() {
            return rendered;
        }
    }
    result.to_string()
}

pub fn parse_json_rpc_response(output: &str) -> AgentResult<McpJsonRpcResponse> {
    let line = output
        .lines()
        .find(|line| line.trim_start().starts_with('{'))
        .ok_or_else(|| AgentError::Execution {
            message: "MCP stdio response did not contain JSON".to_string(),
        })?;
    let value = serde_json::from_str::<Value>(line).map_err(|error| AgentError::Execution {
        message: format!("failed to parse MCP JSON-RPC response: {error}"),
    })?;
    Ok(McpJsonRpcResponse {
        id: value
            .get("id")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        result: value.get("result").cloned(),
        error: value.get("error").cloned(),
    })
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
    SessionReused {
        server: String,
        reuse_count: usize,
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
    tool_call_counts: Arc<Mutex<BTreeMap<String, usize>>>,
    events: Arc<Mutex<Vec<McpLifecycleEvent>>>,
}

impl InMemoryMcpRuntime {
    pub fn new(snapshot: McpRuntimeSnapshot) -> Self {
        Self {
            snapshot: Arc::new(Mutex::new(snapshot)),
            resource_content: Arc::default(),
            tool_results: Arc::default(),
            tool_call_counts: Arc::default(),
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

    fn lock_tool_call_counts(
        &self,
    ) -> AgentResult<std::sync::MutexGuard<'_, BTreeMap<String, usize>>> {
        self.tool_call_counts
            .lock()
            .map_err(|_| AgentError::Execution {
                message: "MCP tool call count lock was poisoned".to_string(),
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
        let reuse_count = {
            let mut counts = self.lock_tool_call_counts()?;
            let count = counts.entry(invocation.server.clone()).or_default();
            let previous = *count;
            *count += 1;
            previous
        };
        if reuse_count > 0 {
            self.emit(McpLifecycleEvent::SessionReused {
                server: invocation.server.clone(),
                reuse_count,
            })?;
        }
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
    fn json_rpc_response_parser_reads_result_line() {
        let response = parse_json_rpc_response(
            "log line\n{\"jsonrpc\":\"2.0\",\"id\":\"1\",\"result\":{\"tools\":[]}}\n",
        )
        .expect("json rpc response");

        assert_eq!(response.id.as_deref(), Some("1"));
        assert!(response.result.expect("result").get("tools").is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn http_mcp_client_uses_transport_for_json_rpc_request() {
        let client = HttpMcpClient::with_transport(
            "https://mcp.example.test/rpc",
            FixtureMcpHttpTransport::new(
                200,
                "{\"jsonrpc\":\"2.0\",\"id\":\"tools\",\"result\":{\"tools\":[]}}",
            ),
        );

        let response = client
            .request_once(McpJsonRpcRequest::list_tools("tools"))
            .await
            .expect("http response");

        assert_eq!(response.id.as_deref(), Some("tools"));
        assert!(response.result.expect("result").get("tools").is_some());
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

    #[test]
    fn loads_workspace_mcp_config_servers() {
        let temp = TempDir::new().expect("temp dir");
        let config_dir = temp.path().join(".yunxi");
        std::fs::create_dir_all(&config_dir).expect("config dir");
        std::fs::write(
            config_dir.join("mcp.json"),
            json!({
                "servers": [
                    {
                        "name": "local",
                        "transport": {"type": "stdio", "command": "fixture", "args": []},
                        "enabled": true
                    }
                ]
            })
            .to_string(),
        )
        .expect("mcp config");

        let servers = load_workspace_mcp_configs(temp.path()).expect("configs");

        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "local");
    }
}

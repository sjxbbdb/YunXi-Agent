use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use yunxi_agent_core::AgentResult;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub transport: McpTransport,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpTransport {
    Stdio { command: String, args: Vec<String> },
    Http { url: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct McpResourceRequest {
    pub server: String,
    pub uri: String,
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

#[async_trait]
pub trait McpRuntime: Send + Sync {
    async fn list_resources(&self, server: &str) -> AgentResult<Vec<String>>;
    async fn read_resource(&self, request: McpResourceRequest) -> AgentResult<String>;
    async fn call_tool(&self, invocation: McpToolInvocation) -> AgentResult<McpToolResult>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DecliningMcpRuntime;

#[async_trait]
impl McpRuntime for DecliningMcpRuntime {
    async fn list_resources(&self, _server: &str) -> AgentResult<Vec<String>> {
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
}

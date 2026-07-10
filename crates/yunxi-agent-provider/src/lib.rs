use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use yunxi_agent_core::{AgentConfig, AgentInput, AgentResult, TokenUsage};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProviderRequest {
    pub config: AgentConfig,
    pub input: AgentInput,
    pub messages: Vec<ProviderMessage>,
}

impl ProviderRequest {
    pub fn new(config: AgentConfig, input: AgentInput) -> Self {
        let messages = vec![ProviderMessage::user(input.prompt.clone())];
        Self {
            config,
            input,
            messages,
        }
    }

    pub fn with_messages(
        config: AgentConfig,
        input: AgentInput,
        messages: Vec<ProviderMessage>,
    ) -> Self {
        Self {
            config,
            input,
            messages,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProviderResponse {
    pub message: Option<ProviderMessage>,
    pub tool_calls: Vec<ProviderToolCall>,
    pub usage: Option<TokenUsage>,
}

impl ProviderResponse {
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            message: Some(ProviderMessage::assistant(content)),
            tool_calls: Vec::new(),
            usage: None,
        }
    }

    pub fn tool_call(tool_call: ProviderToolCall) -> Self {
        Self {
            message: None,
            tool_calls: vec![tool_call],
            usage: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProviderMessage {
    pub role: ProviderRole,
    pub content: String,
}

impl ProviderMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: ProviderRole::System,
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ProviderRole::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: ProviderRole::Assistant,
            content: content.into(),
        }
    }

    pub fn tool(content: impl Into<String>) -> Self {
        Self {
            role: ProviderRole::Tool,
            content: content.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ProviderToolCall {
    Shell {
        id: Option<String>,
        command: String,
    },
    Patch {
        id: Option<String>,
        patch: String,
    },
    Mcp {
        id: Option<String>,
        server: String,
        tool: String,
        arguments_json: Option<String>,
    },
    Skill {
        id: Option<String>,
        name: String,
        arguments_json: Option<String>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProviderUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
}

#[async_trait]
pub trait AgentProvider: Send + Sync {
    async fn complete(&self, request: ProviderRequest) -> AgentResult<ProviderResponse>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticProvider {
    response_prefix: String,
}

impl StaticProvider {
    pub fn new(response_prefix: impl Into<String>) -> Self {
        Self {
            response_prefix: response_prefix.into(),
        }
    }
}

impl Default for StaticProvider {
    fn default() -> Self {
        Self::new("YunXi autonomous runtime accepted prompt")
    }
}

#[async_trait]
impl AgentProvider for StaticProvider {
    async fn complete(&self, request: ProviderRequest) -> AgentResult<ProviderResponse> {
        Ok(ProviderResponse::assistant(format!(
            "{}: {}",
            self.response_prefix,
            request.input.prompt.trim()
        )))
    }
}

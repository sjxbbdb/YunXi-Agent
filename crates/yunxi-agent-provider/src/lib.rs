use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use yunxi_agent_core::{AgentConfig, AgentError, AgentInput, AgentResult, TokenUsage};

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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub model: String,
    pub base_url: String,
}

impl ProviderConfig {
    pub fn openai_compatible(model: impl Into<String>) -> Self {
        Self {
            name: "openai-compatible".to_string(),
            model: model.into(),
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ProviderAuth {
    None,
    ApiKey(String),
    EnvVar(String),
}

impl ProviderAuth {
    pub fn resolve(&self) -> AgentResult<Option<String>> {
        match self {
            Self::None => Ok(None),
            Self::ApiKey(value) => Ok(Some(value.clone())),
            Self::EnvVar(name) => {
                std::env::var(name)
                    .map(Some)
                    .map_err(|error| AgentError::Execution {
                        message: format!(
                            "provider auth environment variable {name} is missing: {error}"
                        ),
                    })
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenAiCompatibleProvider {
    config: ProviderConfig,
    auth: ProviderAuth,
    fixture_response: Option<String>,
}

impl OpenAiCompatibleProvider {
    pub fn new(config: ProviderConfig, auth: ProviderAuth) -> Self {
        Self {
            config,
            auth,
            fixture_response: None,
        }
    }

    pub fn with_fixture_response(mut self, response: impl Into<String>) -> Self {
        self.fixture_response = Some(response.into());
        self
    }

    pub fn request_json(&self, request: &ProviderRequest) -> AgentResult<Value> {
        build_openai_request_json(&self.config, request)
    }

    pub fn parse_response_json(&self, response: &str) -> AgentResult<ProviderResponse> {
        parse_openai_response_json(response)
    }

    pub fn auth(&self) -> &ProviderAuth {
        &self.auth
    }
}

#[async_trait]
impl AgentProvider for OpenAiCompatibleProvider {
    async fn complete(&self, request: ProviderRequest) -> AgentResult<ProviderResponse> {
        let _ = self.auth.resolve()?;
        let _ = self.request_json(&request)?;
        if let Some(response) = &self.fixture_response {
            return self.parse_response_json(response);
        }

        Err(AgentError::Execution {
            message: "openai-compatible HTTP transport is not configured in this runtime slice"
                .to_string(),
        })
    }
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

pub fn build_openai_request_json(
    provider_config: &ProviderConfig,
    request: &ProviderRequest,
) -> AgentResult<Value> {
    let messages = request
        .messages
        .iter()
        .map(|message| {
            json!({
                "role": match message.role {
                    ProviderRole::System => "system",
                    ProviderRole::User => "user",
                    ProviderRole::Assistant => "assistant",
                    ProviderRole::Tool => "tool",
                },
                "content": message.content,
            })
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "model": request
            .config
            .model
            .as_deref()
            .unwrap_or(provider_config.model.as_str()),
        "messages": messages,
        "tools": [
            {
                "type": "function",
                "function": {
                    "name": "shell",
                    "description": "Run a shell command inside the configured YunXi workspace.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "command": { "type": "string" }
                        },
                        "required": ["command"]
                    }
                }
            },
            {
                "type": "function",
                "function": {
                    "name": "patch",
                    "description": "Apply a constrained YunXi patch JSON operation.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "op": { "type": "string" },
                            "path": { "type": "string" },
                            "content": { "type": "string" }
                        },
                        "required": ["op", "path"]
                    }
                }
            }
        ]
    }))
}

pub fn parse_openai_response_json(response: &str) -> AgentResult<ProviderResponse> {
    let value = serde_json::from_str::<Value>(response).map_err(|error| AgentError::Execution {
        message: format!("failed to parse provider response JSON: {error}"),
    })?;
    let message = value
        .pointer("/choices/0/message")
        .ok_or_else(|| AgentError::Execution {
            message: "provider response did not contain choices[0].message".to_string(),
        })?;

    let content = message
        .get("content")
        .and_then(Value::as_str)
        .filter(|content| !content.is_empty())
        .map(ProviderMessage::assistant);

    let mut tool_calls = Vec::new();
    if let Some(calls) = message.get("tool_calls").and_then(Value::as_array) {
        for call in calls {
            if call.get("type").and_then(Value::as_str) != Some("function") {
                continue;
            }
            let id = call
                .get("id")
                .and_then(Value::as_str)
                .map(ToString::to_string);
            let function = call.get("function").ok_or_else(|| AgentError::Execution {
                message: "provider tool call is missing function object".to_string(),
            })?;
            let name = function
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| AgentError::Execution {
                    message: "provider tool call function is missing name".to_string(),
                })?;
            let arguments = function
                .get("arguments")
                .and_then(Value::as_str)
                .unwrap_or("{}");
            tool_calls.push(parse_openai_tool_call(id, name, arguments)?);
        }
    }

    Ok(ProviderResponse {
        message: content,
        tool_calls,
        usage: parse_openai_usage(&value),
    })
}

fn parse_openai_tool_call(
    id: Option<String>,
    name: &str,
    arguments: &str,
) -> AgentResult<ProviderToolCall> {
    let args = serde_json::from_str::<Value>(arguments).map_err(|error| AgentError::Execution {
        message: format!("failed to parse provider tool arguments for {name}: {error}"),
    })?;
    match name {
        "shell" => Ok(ProviderToolCall::Shell {
            id,
            command: required_string(&args, "command")?,
        }),
        "patch" => Ok(ProviderToolCall::Patch {
            id,
            patch: arguments.to_string(),
        }),
        "mcp" => Ok(ProviderToolCall::Mcp {
            id,
            server: required_string(&args, "server")?,
            tool: required_string(&args, "tool")?,
            arguments_json: args.get("arguments_json").map(Value::to_string),
        }),
        "skill" => Ok(ProviderToolCall::Skill {
            id,
            name: required_string(&args, "name")?,
            arguments_json: args.get("arguments_json").map(Value::to_string),
        }),
        other => Err(AgentError::Execution {
            message: format!("unsupported provider tool call: {other}"),
        }),
    }
}

fn required_string(value: &Value, key: &str) -> AgentResult<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| AgentError::Execution {
            message: format!("provider tool argument {key} is missing or not a string"),
        })
}

fn parse_openai_usage(value: &Value) -> Option<TokenUsage> {
    let usage = value.get("usage")?;
    Some(TokenUsage {
        input_tokens: usage
            .get("prompt_tokens")
            .and_then(Value::as_i64)
            .unwrap_or(0),
        cached_input_tokens: usage
            .pointer("/prompt_tokens_details/cached_tokens")
            .and_then(Value::as_i64)
            .unwrap_or(0),
        output_tokens: usage
            .get("completion_tokens")
            .and_then(Value::as_i64)
            .unwrap_or(0),
        reasoning_output_tokens: usage
            .pointer("/completion_tokens_details/reasoning_tokens")
            .and_then(Value::as_i64)
            .unwrap_or(0),
    })
}

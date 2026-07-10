use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use yunxi_agent_core::{AgentConfig, AgentError, AgentInput, AgentResult, TokenUsage};
use yunxi_agent_protocol::{ProtocolRole, response_text_delta};
use yunxi_agent_protocol::{
    ResponseItemDelta, ResponseStatus, StreamEvent, ThreadId, ToolCall, ToolCallStatus, TurnId,
};
use yunxi_agent_tools::default_tool_registry;

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
    MultiAgent {
        id: Option<String>,
        action: String,
        arguments_json: Option<String>,
    },
    ToolSearch {
        id: Option<String>,
        query: String,
    },
    RequestUserInput {
        id: Option<String>,
        prompt: String,
    },
    ViewImage {
        id: Option<String>,
        path: String,
    },
}

impl From<ProviderToolCall> for ToolCall {
    fn from(value: ProviderToolCall) -> Self {
        match value {
            ProviderToolCall::Shell { id, command } => Self::Shell { id, command },
            ProviderToolCall::Patch { id, patch } => Self::Patch { id, patch },
            ProviderToolCall::Mcp {
                id,
                server,
                tool,
                arguments_json,
            } => Self::Mcp {
                id,
                server,
                tool,
                arguments_json,
            },
            ProviderToolCall::Skill {
                id,
                name,
                arguments_json,
            } => Self::Skill {
                id,
                name,
                arguments_json,
            },
            ProviderToolCall::MultiAgent {
                id,
                action,
                arguments_json,
            } => Self::MultiAgent {
                id,
                action,
                arguments_json,
            },
            ProviderToolCall::ToolSearch { id, query } => Self::ToolSearch { id, query },
            ProviderToolCall::RequestUserInput { id, prompt } => {
                Self::RequestUserInput { id, prompt }
            }
            ProviderToolCall::ViewImage { id, path } => Self::ViewImage { id, path },
        }
    }
}

impl From<ProviderRole> for ProtocolRole {
    fn from(value: ProviderRole) -> Self {
        match value {
            ProviderRole::System => Self::System,
            ProviderRole::User => Self::User,
            ProviderRole::Assistant => Self::Assistant,
            ProviderRole::Tool => Self::Tool,
        }
    }
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
pub struct ProviderTransportRequest {
    pub method: String,
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub body: Value,
    pub stream: bool,
    pub timeout_millis: Option<u64>,
}

impl ProviderTransportRequest {
    pub fn post_json(url: impl Into<String>, body: Value) -> Self {
        Self {
            method: "POST".to_string(),
            url: url.into(),
            headers: BTreeMap::new(),
            body,
            stream: false,
            timeout_millis: None,
        }
    }

    pub fn with_bearer_auth(mut self, token: impl Into<String>) -> Self {
        self.headers.insert(
            "authorization".to_string(),
            format!("Bearer {}", token.into()),
        );
        self
    }

    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProviderTransportResponse {
    pub status: u16,
    pub body: String,
}

impl ProviderTransportResponse {
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

#[async_trait]
pub trait ProviderTransport: Send + Sync {
    async fn send(
        &self,
        request: ProviderTransportRequest,
    ) -> AgentResult<ProviderTransportResponse>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixtureTransport {
    response: ProviderTransportResponse,
}

impl FixtureTransport {
    pub fn new(status: u16, body: impl Into<String>) -> Self {
        Self {
            response: ProviderTransportResponse {
                status,
                body: body.into(),
            },
        }
    }
}

#[async_trait]
impl ProviderTransport for FixtureTransport {
    async fn send(
        &self,
        _request: ProviderTransportRequest,
    ) -> AgentResult<ProviderTransportResponse> {
        Ok(self.response.clone())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProviderRetryPolicy {
    pub max_attempts: usize,
    pub retry_statuses: Vec<u16>,
}

impl ProviderRetryPolicy {
    pub fn new(max_attempts: usize) -> Self {
        Self {
            max_attempts: max_attempts.max(1),
            retry_statuses: vec![408, 409, 429, 500, 502, 503, 504],
        }
    }

    pub fn should_retry_status(&self, status: u16, attempt: usize) -> bool {
        attempt + 1 < self.max_attempts && self.retry_statuses.contains(&status)
    }
}

impl Default for ProviderRetryPolicy {
    fn default() -> Self {
        Self::new(3)
    }
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

    pub fn streaming_request_json(&self, request: &ProviderRequest) -> AgentResult<Value> {
        build_openai_stream_request_json(&self.config, request)
    }

    pub fn parse_response_json(&self, response: &str) -> AgentResult<ProviderResponse> {
        parse_openai_response_json(response)
    }

    pub fn parse_stream_events(
        &self,
        thread_id: impl Into<String>,
        turn_id: impl Into<String>,
        stream: &str,
    ) -> AgentResult<Vec<StreamEvent>> {
        parse_openai_stream_events(thread_id, turn_id, stream)
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
pub struct OpenAiTransportProvider<T> {
    provider: OpenAiCompatibleProvider,
    transport: T,
    retry_policy: ProviderRetryPolicy,
}

impl<T> OpenAiTransportProvider<T>
where
    T: ProviderTransport,
{
    pub fn new(config: ProviderConfig, auth: ProviderAuth, transport: T) -> Self {
        Self {
            provider: OpenAiCompatibleProvider::new(config, auth),
            transport,
            retry_policy: ProviderRetryPolicy::default(),
        }
    }

    pub fn with_retry_policy(mut self, retry_policy: ProviderRetryPolicy) -> Self {
        self.retry_policy = retry_policy;
        self
    }

    pub async fn stream_events(
        &self,
        request: ProviderRequest,
        thread_id: impl Into<String>,
        turn_id: impl Into<String>,
    ) -> AgentResult<Vec<StreamEvent>> {
        let transport_request = build_openai_transport_request(
            &self.provider.config,
            self.provider.auth(),
            &request,
            true,
        )?;
        let response = self.send_with_retries(transport_request).await?;
        if !response.is_success() {
            return Err(AgentError::Execution {
                message: format!("provider transport returned HTTP {}", response.status),
            });
        }
        self.provider
            .parse_stream_events(thread_id, turn_id, &response.body)
    }

    async fn send_with_retries(
        &self,
        request: ProviderTransportRequest,
    ) -> AgentResult<ProviderTransportResponse> {
        let mut attempt = 0usize;
        loop {
            let response = self.transport.send(request.clone()).await?;
            if !self
                .retry_policy
                .should_retry_status(response.status, attempt)
            {
                return Ok(response);
            }
            attempt += 1;
        }
    }
}

#[async_trait]
impl<T> AgentProvider for OpenAiTransportProvider<T>
where
    T: ProviderTransport + Send + Sync,
{
    async fn complete(&self, request: ProviderRequest) -> AgentResult<ProviderResponse> {
        let transport_request = build_openai_transport_request(
            &self.provider.config,
            self.provider.auth(),
            &request,
            false,
        )?;
        let response = self.send_with_retries(transport_request).await?;
        if !response.is_success() {
            return Err(AgentError::Execution {
                message: format!("provider transport returned HTTP {}", response.status),
            });
        }
        self.provider.parse_response_json(&response.body)
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
        "tools": default_tool_registry().openai_tools_json()
    }))
}

pub fn build_openai_stream_request_json(
    provider_config: &ProviderConfig,
    request: &ProviderRequest,
) -> AgentResult<Value> {
    let mut value = build_openai_request_json(provider_config, request)?;
    value["stream"] = Value::Bool(true);
    value["stream_options"] = json!({ "include_usage": true });
    Ok(value)
}

pub fn build_openai_transport_request(
    provider_config: &ProviderConfig,
    auth: &ProviderAuth,
    request: &ProviderRequest,
    stream: bool,
) -> AgentResult<ProviderTransportRequest> {
    let body = if stream {
        build_openai_stream_request_json(provider_config, request)?
    } else {
        build_openai_request_json(provider_config, request)?
    };
    let url = format!(
        "{}/chat/completions",
        provider_config.base_url.trim_end_matches('/')
    );
    let mut transport_request = ProviderTransportRequest::post_json(url, body).with_stream(stream);
    if let Some(token) = auth.resolve()? {
        transport_request = transport_request.with_bearer_auth(token);
    }
    transport_request
        .headers
        .insert("content-type".to_string(), "application/json".to_string());
    Ok(transport_request)
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

pub fn parse_openai_stream_events(
    thread_id: impl Into<String>,
    turn_id: impl Into<String>,
    stream: &str,
) -> AgentResult<Vec<StreamEvent>> {
    let thread_id = ThreadId(thread_id.into());
    let turn_id = TurnId(turn_id.into());
    let mut events = vec![StreamEvent::ResponseStarted {
        thread_id: thread_id.clone(),
        turn_id: turn_id.clone(),
        metadata: None,
    }];

    for line in stream.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(':') {
            continue;
        }
        let Some(data) = line.strip_prefix("data:").map(str::trim) else {
            continue;
        };
        if data == "[DONE]" {
            events.push(StreamEvent::ResponseCompleted {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                status: ResponseStatus::Completed,
            });
            continue;
        }

        let value = serde_json::from_str::<Value>(data).map_err(|error| AgentError::Execution {
            message: format!("failed to parse provider stream event JSON: {error}"),
        })?;
        parse_stream_value(&thread_id, &turn_id, &value, &mut events);
    }

    if !events
        .iter()
        .any(|event| matches!(event, StreamEvent::ResponseCompleted { .. }))
    {
        events.push(StreamEvent::ResponseCompleted {
            thread_id,
            turn_id,
            status: ResponseStatus::Completed,
        });
    }

    Ok(events)
}

fn parse_stream_value(
    thread_id: &ThreadId,
    turn_id: &TurnId,
    value: &Value,
    events: &mut Vec<StreamEvent>,
) {
    if let Some(event_type) = value.get("type").and_then(Value::as_str) {
        parse_responses_api_stream_value(thread_id, turn_id, event_type, value, events);
        return;
    }

    let Some(choice) = value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
    else {
        return;
    };
    if let Some(delta) = choice.pointer("/delta/content").and_then(Value::as_str) {
        push_delta(thread_id, turn_id, response_text_delta(delta), events);
    }
    if let Some(delta) = choice
        .pointer("/delta/reasoning_content")
        .and_then(Value::as_str)
    {
        push_delta(
            thread_id,
            turn_id,
            ResponseItemDelta::ReasoningContent {
                item_id: None,
                delta: delta.to_string(),
            },
            events,
        );
    }
    if let Some(tool_calls) = choice
        .pointer("/delta/tool_calls")
        .and_then(Value::as_array)
    {
        for tool_call in tool_calls {
            let call_id = tool_call
                .get("id")
                .and_then(Value::as_str)
                .map(ToString::to_string);
            if let Some(delta) = tool_call
                .pointer("/function/arguments")
                .and_then(Value::as_str)
            {
                push_delta(
                    thread_id,
                    turn_id,
                    ResponseItemDelta::ToolCallArguments {
                        call_id,
                        delta: delta.to_string(),
                    },
                    events,
                );
            }
        }
    }
    if choice
        .get("finish_reason")
        .and_then(Value::as_str)
        .is_some()
    {
        events.push(StreamEvent::ResponseCompleted {
            thread_id: thread_id.clone(),
            turn_id: turn_id.clone(),
            status: ResponseStatus::Completed,
        });
    }
}

fn parse_responses_api_stream_value(
    thread_id: &ThreadId,
    turn_id: &TurnId,
    event_type: &str,
    value: &Value,
    events: &mut Vec<StreamEvent>,
) {
    match event_type {
        "response.output_text.delta" | "response.refusal.delta" => {
            if let Some(delta) = value.get("delta").and_then(Value::as_str) {
                push_delta(thread_id, turn_id, response_text_delta(delta), events);
            }
        }
        "response.reasoning_text.delta" | "response.reasoning_summary_text.delta" => {
            if let Some(delta) = value.get("delta").and_then(Value::as_str) {
                push_delta(
                    thread_id,
                    turn_id,
                    ResponseItemDelta::ReasoningContent {
                        item_id: value
                            .get("item_id")
                            .and_then(Value::as_str)
                            .map(ToString::to_string),
                        delta: delta.to_string(),
                    },
                    events,
                );
            }
        }
        "response.function_call_arguments.delta" => {
            if let Some(delta) = value.get("delta").and_then(Value::as_str) {
                push_delta(
                    thread_id,
                    turn_id,
                    ResponseItemDelta::ToolCallArguments {
                        call_id: value
                            .get("call_id")
                            .and_then(Value::as_str)
                            .map(ToString::to_string),
                        delta: delta.to_string(),
                    },
                    events,
                );
            }
        }
        "response.completed" => events.push(StreamEvent::ResponseCompleted {
            thread_id: thread_id.clone(),
            turn_id: turn_id.clone(),
            status: ResponseStatus::Completed,
        }),
        "response.failed" => events.push(StreamEvent::ResponseFailed {
            thread_id: thread_id.clone(),
            turn_id: turn_id.clone(),
            message: value
                .pointer("/response/error/message")
                .and_then(Value::as_str)
                .or_else(|| value.pointer("/error/message").and_then(Value::as_str))
                .unwrap_or("provider stream failed")
                .to_string(),
        }),
        "response.function_call.completed" => push_delta(
            thread_id,
            turn_id,
            ResponseItemDelta::ToolCallStatus {
                call_id: value
                    .get("call_id")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                status: ToolCallStatus::Completed,
            },
            events,
        ),
        _ => {}
    }
}

fn push_delta(
    thread_id: &ThreadId,
    turn_id: &TurnId,
    delta: ResponseItemDelta,
    events: &mut Vec<StreamEvent>,
) {
    events.push(StreamEvent::ItemDelta {
        thread_id: thread_id.clone(),
        turn_id: turn_id.clone(),
        delta,
    });
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
            arguments_json: optional_json_argument(&args, "arguments_json"),
        }),
        "skill" => Ok(ProviderToolCall::Skill {
            id,
            name: required_string(&args, "name")?,
            arguments_json: optional_json_argument(&args, "arguments_json"),
        }),
        "multi_agent" => Ok(ProviderToolCall::MultiAgent {
            id,
            action: required_string(&args, "action")?,
            arguments_json: optional_json_argument(&args, "arguments_json"),
        }),
        "tool_search" => Ok(ProviderToolCall::ToolSearch {
            id,
            query: required_string(&args, "query")?,
        }),
        "request_user_input" => Ok(ProviderToolCall::RequestUserInput {
            id,
            prompt: required_string(&args, "prompt")?,
        }),
        "view_image" => Ok(ProviderToolCall::ViewImage {
            id,
            path: required_string(&args, "path")?,
        }),
        other => Err(AgentError::Execution {
            message: format!("unsupported provider tool call: {other}"),
        }),
    }
}

fn optional_json_argument(value: &Value, key: &str) -> Option<String> {
    value.get(key).map(|argument| {
        argument
            .as_str()
            .map(ToString::to_string)
            .unwrap_or_else(|| argument.to_string())
    })
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

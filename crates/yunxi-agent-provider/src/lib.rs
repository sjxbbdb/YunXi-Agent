use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::time::Duration;
use yunxi_agent_core::{AgentConfig, AgentError, AgentInput, AgentResult, TokenUsage};
use yunxi_agent_protocol::{
    ProtocolRole, ResponseItem, ResponseItemDelta, ResponseStatus, StreamEvent, ThreadId, ToolCall,
    ToolCallStatus, TurnId, response_text_delta,
};
use yunxi_agent_tools::workspace_tool_registry;

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

    async fn stream(
        &self,
        request: ProviderRequest,
        thread_id: ThreadId,
        turn_id: TurnId,
    ) -> AgentResult<ProviderStream> {
        let response = self.complete(request).await?;
        Ok(ProviderStream::from_response(thread_id, turn_id, response))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProviderStream {
    pub events: Vec<StreamEvent>,
    pub final_response: Option<ProviderResponse>,
}

impl ProviderStream {
    pub fn new(events: Vec<StreamEvent>, final_response: Option<ProviderResponse>) -> Self {
        Self {
            events,
            final_response,
        }
    }

    pub fn from_response(thread_id: ThreadId, turn_id: TurnId, response: ProviderResponse) -> Self {
        let mut events = vec![StreamEvent::ResponseStarted {
            thread_id: thread_id.clone(),
            turn_id: turn_id.clone(),
            metadata: None,
        }];
        if let Some(message) = &response.message {
            events.push(StreamEvent::ItemCompleted {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                item: ResponseItem::Message {
                    role: message.role.into(),
                    content: message.content.clone(),
                },
            });
        }
        for tool_call in &response.tool_calls {
            events.push(StreamEvent::ItemCompleted {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                item: ResponseItem::ToolCall {
                    call: tool_call.clone().into(),
                },
            });
        }
        events.push(StreamEvent::ResponseCompleted {
            thread_id,
            turn_id,
            status: ResponseStatus::Completed,
        });
        Self::new(events, Some(response))
    }

    pub fn from_events(events: Vec<StreamEvent>) -> Self {
        Self::new(events, None)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProviderStreamChunk {
    Data { value: String },
    Done,
    Comment { value: String },
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProviderSseDecoder {
    buffer: String,
}

impl ProviderSseDecoder {
    pub fn push_chunk(&mut self, chunk: &str) -> AgentResult<Vec<ProviderStreamChunk>> {
        self.buffer.push_str(chunk);
        let mut decoded = Vec::new();
        while let Some(newline) = self.buffer.find('\n') {
            let mut line = self.buffer.drain(..=newline).collect::<String>();
            while line.ends_with('\n') || line.ends_with('\r') {
                line.pop();
            }
            if let Some(chunk) = decode_sse_line(&line) {
                decoded.push(chunk);
            }
        }
        Ok(decoded)
    }

    pub fn finish(&mut self) -> AgentResult<Vec<ProviderStreamChunk>> {
        if self.buffer.is_empty() {
            return Ok(Vec::new());
        }
        let line = std::mem::take(&mut self.buffer);
        Ok(
            decode_sse_line(line.trim_end_matches(|ch| ch == '\r' || ch == '\n'))
                .into_iter()
                .collect(),
        )
    }
}

fn decode_sse_line(line: &str) -> Option<ProviderStreamChunk> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    if let Some(comment) = line.strip_prefix(':') {
        return Some(ProviderStreamChunk::Comment {
            value: comment.trim().to_string(),
        });
    }
    let data = line.strip_prefix("data:")?.trim();
    if data == "[DONE]" {
        Some(ProviderStreamChunk::Done)
    } else {
        Some(ProviderStreamChunk::Data {
            value: data.to_string(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenAiStreamAccumulator {
    thread_id: ThreadId,
    turn_id: TurnId,
    events: Vec<StreamEvent>,
    chat_tool_calls: BTreeMap<usize, ChatToolCallDelta>,
    completed: bool,
}

impl OpenAiStreamAccumulator {
    pub fn new(thread_id: impl Into<String>, turn_id: impl Into<String>) -> Self {
        let thread_id = ThreadId(thread_id.into());
        let turn_id = TurnId(turn_id.into());
        let events = vec![StreamEvent::ResponseStarted {
            thread_id: thread_id.clone(),
            turn_id: turn_id.clone(),
            metadata: None,
        }];
        Self {
            thread_id,
            turn_id,
            events,
            chat_tool_calls: BTreeMap::new(),
            completed: false,
        }
    }

    pub fn push_sse_chunk(&mut self, chunk: ProviderStreamChunk) -> AgentResult<Vec<StreamEvent>> {
        let before = self.events.len();
        match chunk {
            ProviderStreamChunk::Data { value } => {
                let value = serde_json::from_str::<Value>(&value).map_err(|error| {
                    AgentError::Execution {
                        message: format!("failed to parse provider stream event JSON: {error}"),
                    }
                })?;
                parse_stream_value(
                    &self.thread_id,
                    &self.turn_id,
                    &value,
                    &mut self.events,
                    &mut self.chat_tool_calls,
                );
                if self.events[before..]
                    .iter()
                    .any(|event| matches!(event, StreamEvent::ResponseCompleted { .. }))
                {
                    self.completed = true;
                }
            }
            ProviderStreamChunk::Done => self.complete_response(),
            ProviderStreamChunk::Comment { .. } => {}
        }
        Ok(self.events[before..].to_vec())
    }

    pub fn push_raw_chunk(
        &mut self,
        decoder: &mut ProviderSseDecoder,
        chunk: &str,
    ) -> AgentResult<Vec<StreamEvent>> {
        let before = self.events.len();
        for decoded in decoder.push_chunk(chunk)? {
            self.push_sse_chunk(decoded)?;
        }
        Ok(self.events[before..].to_vec())
    }

    pub fn finish(mut self, decoder: &mut ProviderSseDecoder) -> AgentResult<Vec<StreamEvent>> {
        for decoded in decoder.finish()? {
            self.push_sse_chunk(decoded)?;
        }
        if !self.completed {
            self.complete_response();
        }
        Ok(self.events)
    }

    fn complete_response(&mut self) {
        if self.completed {
            return;
        }
        flush_chat_tool_calls(
            &self.thread_id,
            &self.turn_id,
            &mut self.events,
            &self.chat_tool_calls,
        );
        self.events.push(StreamEvent::ResponseCompleted {
            thread_id: self.thread_id.clone(),
            turn_id: self.turn_id.clone(),
            status: ResponseStatus::Completed,
        });
        self.completed = true;
    }
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderErrorKind {
    Auth,
    RateLimit,
    Server,
    Network,
    Timeout,
    InvalidResponse,
    UnsupportedModel,
    Unknown,
}

pub fn classify_provider_status(status: u16) -> ProviderErrorKind {
    match status {
        401 | 403 => ProviderErrorKind::Auth,
        404 => ProviderErrorKind::UnsupportedModel,
        408 => ProviderErrorKind::Timeout,
        429 => ProviderErrorKind::RateLimit,
        500..=599 => ProviderErrorKind::Server,
        400..=499 => ProviderErrorKind::InvalidResponse,
        _ => ProviderErrorKind::Unknown,
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

#[derive(Clone, Debug)]
pub struct ReqwestProviderTransport {
    client: reqwest::Client,
}

impl ReqwestProviderTransport {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for ReqwestProviderTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderTransport for ReqwestProviderTransport {
    async fn send(
        &self,
        request: ProviderTransportRequest,
    ) -> AgentResult<ProviderTransportResponse> {
        if request.method != "POST" {
            return Err(AgentError::Execution {
                message: format!("unsupported provider transport method {}", request.method),
            });
        }
        let mut headers = HeaderMap::new();
        for (name, value) in &request.headers {
            let name =
                HeaderName::from_bytes(name.as_bytes()).map_err(|error| AgentError::Execution {
                    message: format!("invalid provider header name {name}: {error}"),
                })?;
            let value = HeaderValue::from_str(value).map_err(|error| AgentError::Execution {
                message: format!("invalid provider header value for {name}: {error}"),
            })?;
            headers.insert(name, value);
        }
        let mut builder = self
            .client
            .post(&request.url)
            .headers(headers)
            .json(&request.body);
        if let Some(timeout_millis) = request.timeout_millis {
            builder = builder.timeout(Duration::from_millis(timeout_millis));
        }
        let response = builder.send().await.map_err(provider_transport_error)?;
        let status = response.status().as_u16();
        let body = response.text().await.map_err(provider_transport_error)?;
        Ok(ProviderTransportResponse { status, body })
    }
}

fn provider_transport_error(error: reqwest::Error) -> AgentError {
    let kind = if error.is_timeout() {
        ProviderErrorKind::Timeout
    } else if error.is_connect() || error.is_request() {
        ProviderErrorKind::Network
    } else if error.is_decode() {
        ProviderErrorKind::InvalidResponse
    } else {
        ProviderErrorKind::Unknown
    };
    AgentError::Execution {
        message: format!("provider transport {kind:?}: {error}"),
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
    pub timeout_millis: Option<u64>,
    pub stream: bool,
    pub capabilities: ProviderCapabilities,
}

impl ProviderConfig {
    pub fn openai_compatible(model: impl Into<String>) -> Self {
        Self {
            name: "openai-compatible".to_string(),
            model: model.into(),
            base_url: "https://api.openai.com/v1".to_string(),
            timeout_millis: Some(120_000),
            stream: true,
            capabilities: ProviderCapabilities::openai_compatible(),
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    pub fn with_timeout_millis(mut self, timeout_millis: Option<u64>) -> Self {
        self.timeout_millis = timeout_millis;
        self
    }

    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }

    pub fn with_capabilities(mut self, capabilities: ProviderCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn from_agent_config(config: &AgentConfig) -> Self {
        let model = config
            .model
            .clone()
            .or_else(|| std::env::var("YUNXI_AGENT_MODEL").ok())
            .unwrap_or_else(|| "gpt-4.1".to_string());
        let base_url = std::env::var("YUNXI_PROVIDER_BASE_URL")
            .or_else(|_| std::env::var("OPENAI_BASE_URL"))
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let stream = std::env::var("YUNXI_PROVIDER_STREAM")
            .ok()
            .map(|value| !matches!(value.as_str(), "0" | "false" | "False" | "FALSE"))
            .unwrap_or(true);
        Self::openai_compatible(model)
            .with_base_url(base_url)
            .with_stream(stream)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProviderBootstrap {
    pub config: ProviderConfig,
    pub auth: ProviderAuth,
}

impl ProviderBootstrap {
    pub fn from_agent_config(config: &AgentConfig) -> Self {
        let provider_config = ProviderConfig::from_agent_config(config);
        let auth = std::env::var("YUNXI_PROVIDER_API_KEY")
            .map(ProviderAuth::ApiKey)
            .unwrap_or_else(|_| {
                let env_name = std::env::var("YUNXI_PROVIDER_API_KEY_ENV")
                    .unwrap_or_else(|_| "OPENAI_API_KEY".to_string());
                ProviderAuth::EnvVar(env_name)
            });
        Self {
            config: provider_config,
            auth,
        }
    }

    pub fn into_openai_transport_provider(
        self,
    ) -> OpenAiTransportProvider<ReqwestProviderTransport> {
        OpenAiTransportProvider::new(self.config, self.auth, ReqwestProviderTransport::default())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub tools: bool,
    pub parallel_tool_calls: bool,
    pub reasoning: bool,
    pub stream_usage: bool,
}

impl ProviderCapabilities {
    pub fn openai_compatible() -> Self {
        Self {
            tools: true,
            parallel_tool_calls: true,
            reasoning: true,
            stream_usage: true,
        }
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

    async fn stream(
        &self,
        request: ProviderRequest,
        thread_id: ThreadId,
        turn_id: TurnId,
    ) -> AgentResult<ProviderStream> {
        let _ = self.auth.resolve()?;
        let _ = self.streaming_request_json(&request)?;
        if let Some(response) = &self.fixture_response {
            if response.trim_start().starts_with("data:") {
                return Ok(ProviderStream::from_events(self.parse_stream_events(
                    thread_id.0,
                    turn_id.0,
                    response,
                )?));
            }
            return Ok(ProviderStream::from_response(
                thread_id,
                turn_id,
                self.parse_response_json(response)?,
            ));
        }

        Err(AgentError::Execution {
            message:
                "openai-compatible HTTP streaming transport is not configured in this runtime slice"
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
                message: format!(
                    "provider transport returned HTTP {} ({:?})",
                    response.status,
                    classify_provider_status(response.status)
                ),
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
                message: format!(
                    "provider transport returned HTTP {} ({:?})",
                    response.status,
                    classify_provider_status(response.status)
                ),
            });
        }
        self.provider.parse_response_json(&response.body)
    }

    async fn stream(
        &self,
        request: ProviderRequest,
        thread_id: ThreadId,
        turn_id: TurnId,
    ) -> AgentResult<ProviderStream> {
        let events = self.stream_events(request, thread_id.0, turn_id.0).await?;
        Ok(ProviderStream::from_events(events))
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
        if self.response_prefix == "YunXi autonomous runtime accepted prompt"
            && request
                .input
                .prompt
                .contains("stage 4j child runtime fixture")
        {
            if let Some(tool_message) = request
                .messages
                .iter()
                .rev()
                .find(|message| message.role == ProviderRole::Tool)
            {
                return Ok(ProviderResponse::assistant(format!(
                    "Stage 4J child runtime fixture completed with child result: {}",
                    tool_message.content.trim()
                )));
            }
            return Ok(ProviderResponse::tool_call(ProviderToolCall::MultiAgent {
                id: Some("stage-4j-child-run".to_string()),
                action: "spawn_run".to_string(),
                arguments_json: Some(
                    r#"{"task":"stage 4j child runtime fixture child task"}"#.to_string(),
                ),
            }));
        }
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

    let mut body = json!({
        "model": request
            .config
            .model
            .as_deref()
            .unwrap_or(provider_config.model.as_str()),
        "messages": messages
    });
    if provider_config.capabilities.tools {
        body["tools"] =
            Value::Array(workspace_tool_registry(&request.config.cwd)?.openai_tools_json());
    }
    if provider_config.capabilities.parallel_tool_calls {
        body["parallel_tool_calls"] = Value::Bool(true);
    }
    Ok(body)
}

pub fn build_openai_stream_request_json(
    provider_config: &ProviderConfig,
    request: &ProviderRequest,
) -> AgentResult<Value> {
    let mut value = build_openai_request_json(provider_config, request)?;
    value["stream"] = Value::Bool(true);
    if provider_config.capabilities.stream_usage {
        value["stream_options"] = json!({ "include_usage": true });
    }
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
    transport_request.timeout_millis = provider_config.timeout_millis;
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
    let mut decoder = ProviderSseDecoder::default();
    let accumulator = OpenAiStreamAccumulator::new(thread_id, turn_id);
    parse_openai_stream_events_incremental(accumulator, &mut decoder, stream)
}

pub fn parse_openai_stream_events_incremental(
    mut accumulator: OpenAiStreamAccumulator,
    decoder: &mut ProviderSseDecoder,
    stream: &str,
) -> AgentResult<Vec<StreamEvent>> {
    accumulator.push_raw_chunk(decoder, stream)?;
    accumulator.finish(decoder)
}

fn parse_stream_value(
    thread_id: &ThreadId,
    turn_id: &TurnId,
    value: &Value,
    events: &mut Vec<StreamEvent>,
    chat_tool_calls: &mut BTreeMap<usize, ChatToolCallDelta>,
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
            let index = tool_call
                .get("index")
                .and_then(Value::as_u64)
                .map(|value| value as usize)
                .unwrap_or(chat_tool_calls.len());
            let accumulator = chat_tool_calls.entry(index).or_default();
            if let Some(id) = tool_call.get("id").and_then(Value::as_str) {
                accumulator.id = Some(id.to_string());
            }
            if let Some(name) = tool_call.pointer("/function/name").and_then(Value::as_str) {
                accumulator.name = Some(name.to_string());
                push_delta(
                    thread_id,
                    turn_id,
                    ResponseItemDelta::ToolCallName {
                        call_id: accumulator.id.clone(),
                        name: name.to_string(),
                    },
                    events,
                );
            }
            let call_id = accumulator.id.clone().or_else(|| {
                tool_call
                    .get("id")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
            });
            if let Some(delta) = tool_call
                .pointer("/function/arguments")
                .and_then(Value::as_str)
            {
                accumulator.arguments.push_str(delta);
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
        flush_chat_tool_calls(thread_id, turn_id, events, chat_tool_calls);
        events.push(StreamEvent::ResponseCompleted {
            thread_id: thread_id.clone(),
            turn_id: turn_id.clone(),
            status: ResponseStatus::Completed,
        });
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ChatToolCallDelta {
    id: Option<String>,
    name: Option<String>,
    arguments: String,
}

fn flush_chat_tool_calls(
    thread_id: &ThreadId,
    turn_id: &TurnId,
    events: &mut Vec<StreamEvent>,
    chat_tool_calls: &BTreeMap<usize, ChatToolCallDelta>,
) {
    for accumulator in chat_tool_calls.values() {
        if let Some(name) = &accumulator.name {
            let call_id = accumulator
                .id
                .clone()
                .unwrap_or_else(|| format!("tool-call-{}", events.len()));
            if events.iter().any(|event| {
                matches!(
                    event,
                    StreamEvent::ItemCompleted {
                        item: ResponseItem::FunctionCall {
                            call_id: existing,
                            ..
                        },
                        ..
                    } if existing == &call_id
                )
            }) {
                continue;
            }
            events.push(StreamEvent::ItemCompleted {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                item: ResponseItem::FunctionCall {
                    id: call_id.clone(),
                    call_id,
                    name: name.clone(),
                    arguments: accumulator.arguments.clone(),
                    status: ToolCallStatus::Completed,
                },
            });
        }
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
        "response.output_item.added" | "response.output_item.done" => {
            if let Some(item) = value.get("item") {
                push_responses_api_item(thread_id, turn_id, item, events);
            }
        }
        _ => {}
    }
}

fn push_responses_api_item(
    thread_id: &ThreadId,
    turn_id: &TurnId,
    item: &Value,
    events: &mut Vec<StreamEvent>,
) {
    match item.get("type").and_then(Value::as_str) {
        Some("message") => {
            let content = item
                .get("content")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|content| {
                    content
                        .get("text")
                        .and_then(Value::as_str)
                        .or_else(|| content.get("content").and_then(Value::as_str))
                })
                .collect::<Vec<_>>()
                .join("");
            if !content.is_empty() {
                events.push(StreamEvent::ItemCompleted {
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    item: ResponseItem::Message {
                        role: ProtocolRole::Assistant,
                        content,
                    },
                });
            }
        }
        Some("function_call") => {
            let call_id = item
                .get("call_id")
                .and_then(Value::as_str)
                .or_else(|| item.get("id").and_then(Value::as_str))
                .unwrap_or("function-call")
                .to_string();
            let name = item
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let arguments = item
                .get("arguments")
                .and_then(Value::as_str)
                .unwrap_or("{}")
                .to_string();
            events.push(StreamEvent::ItemCompleted {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                item: ResponseItem::FunctionCall {
                    id: call_id.clone(),
                    call_id,
                    name,
                    arguments,
                    status: ToolCallStatus::Completed,
                },
            });
        }
        Some("mcp_call") | Some("mcp_tool_call") => {
            let call_id = item
                .get("call_id")
                .and_then(Value::as_str)
                .or_else(|| item.get("id").and_then(Value::as_str))
                .unwrap_or("mcp-call")
                .to_string();
            events.push(StreamEvent::ItemCompleted {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                item: ResponseItem::McpToolCall {
                    id: call_id.clone(),
                    call_id,
                    server: item
                        .get("server_label")
                        .and_then(Value::as_str)
                        .or_else(|| item.get("server").and_then(Value::as_str))
                        .unwrap_or("mcp")
                        .to_string(),
                    tool: item
                        .get("name")
                        .and_then(Value::as_str)
                        .or_else(|| item.get("tool").and_then(Value::as_str))
                        .unwrap_or("tool")
                        .to_string(),
                    arguments: item
                        .get("arguments")
                        .map(Value::to_string)
                        .unwrap_or_else(|| "{}".to_string()),
                    status: ToolCallStatus::Completed,
                },
            });
        }
        Some("web_search_call") => {
            let id = item
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("web-search")
                .to_string();
            events.push(StreamEvent::ItemCompleted {
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                item: ResponseItem::WebSearchCall {
                    id,
                    query: item
                        .get("query")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    status: ToolCallStatus::Completed,
                },
            });
        }
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
        other if other.starts_with("skill__") => Ok(ProviderToolCall::Skill {
            id,
            name: optional_json_argument(&args, "name")
                .unwrap_or_else(|| other.trim_start_matches("skill__").to_string()),
            arguments_json: optional_json_argument(&args, "arguments_json"),
        }),
        other if other.starts_with("plugin__") => Ok(ProviderToolCall::ToolSearch {
            id,
            query: optional_json_argument(&args, "query")
                .unwrap_or_else(|| other.trim_start_matches("plugin__").replace('_', " ")),
        }),
        other if other.starts_with("mcp__") => Ok(ProviderToolCall::Mcp {
            id,
            server: optional_json_argument(&args, "server")
                .unwrap_or_else(|| other.trim_start_matches("mcp__").to_string()),
            tool: required_string(&args, "tool")?,
            arguments_json: optional_json_argument(&args, "arguments_json"),
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

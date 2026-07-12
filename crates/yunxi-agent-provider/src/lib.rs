use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime};
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ProviderToolCall>,
}

impl ProviderMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: ProviderRole::System,
            content: content.into(),
            tool_call_id: None,
            tool_calls: Vec::new(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ProviderRole::User,
            content: content.into(),
            tool_call_id: None,
            tool_calls: Vec::new(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: ProviderRole::Assistant,
            content: content.into(),
            tool_call_id: None,
            tool_calls: Vec::new(),
        }
    }

    pub fn assistant_with_tool_calls(
        content: impl Into<String>,
        tool_calls: Vec<ProviderToolCall>,
    ) -> Self {
        Self {
            role: ProviderRole::Assistant,
            content: content.into(),
            tool_call_id: None,
            tool_calls,
        }
    }

    pub fn tool(content: impl Into<String>) -> Self {
        Self {
            role: ProviderRole::Tool,
            content: content.into(),
            tool_call_id: None,
            tool_calls: Vec::new(),
        }
    }

    pub fn tool_result(call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: ProviderRole::Tool,
            content: content.into(),
            tool_call_id: Some(call_id.into()),
            tool_calls: Vec::new(),
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

    async fn stream_with_sink(
        &self,
        request: ProviderRequest,
        thread_id: ThreadId,
        turn_id: TurnId,
        mut sink: Option<&mut dyn ProviderStreamEventSink>,
    ) -> AgentResult<ProviderStream> {
        let stream = self.stream(request, thread_id, turn_id).await?;
        if let Some(sink) = sink.as_mut() {
            for event in &stream.events {
                sink.emit(event.clone()).await?;
            }
        }
        Ok(stream)
    }
}

#[async_trait]
pub trait ProviderStreamEventSink: Send {
    async fn emit(&mut self, event: StreamEvent) -> AgentResult<()>;
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
        self.finish_incremental(decoder)?;
        Ok(self.events)
    }

    pub fn finish_incremental(
        &mut self,
        decoder: &mut ProviderSseDecoder,
    ) -> AgentResult<Vec<StreamEvent>> {
        let before = self.events.len();
        for decoded in decoder.finish()? {
            self.push_sse_chunk(decoded)?;
        }
        if !self.completed {
            self.complete_response();
        }
        Ok(self.events[before..].to_vec())
    }

    pub fn events(&self) -> Vec<StreamEvent> {
        self.events.clone()
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
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    pub body: String,
}

impl ProviderTransportResponse {
    pub fn new(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            headers: BTreeMap::new(),
            body: body.into(),
        }
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers
            .insert(name.into().to_ascii_lowercase(), value.into());
        self
    }

    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

#[async_trait]
pub trait ProviderByteStreamSink: Send {
    async fn push_bytes(&mut self, chunk: &[u8]) -> AgentResult<()>;
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
    UnsupportedSchema,
    Unknown,
}

impl ProviderErrorKind {
    pub fn classification(self) -> &'static str {
        match self {
            Self::Auth => "auth_error",
            Self::RateLimit => "rate_limit",
            Self::Server => "server_error",
            Self::Network => "network",
            Self::Timeout => "timeout",
            Self::InvalidResponse => "bad_request",
            Self::UnsupportedModel => "unsupported_model",
            Self::UnsupportedSchema => "unsupported_schema",
            Self::Unknown => "unknown_provider_error",
        }
    }
}

pub fn classify_provider_status(status: u16) -> ProviderErrorKind {
    match status {
        401 | 403 => ProviderErrorKind::Auth,
        404 => ProviderErrorKind::UnsupportedModel,
        408 => ProviderErrorKind::Timeout,
        429 => ProviderErrorKind::RateLimit,
        500..=599 => ProviderErrorKind::Server,
        400 | 422 => ProviderErrorKind::UnsupportedSchema,
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

    async fn send_streaming(
        &self,
        request: ProviderTransportRequest,
        sink: &mut dyn ProviderByteStreamSink,
    ) -> AgentResult<ProviderTransportResponse> {
        let response = self.send(request).await?;
        if response.is_success() {
            sink.push_bytes(response.body.as_bytes()).await?;
        }
        Ok(response)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixtureTransport {
    response: ProviderTransportResponse,
}

impl FixtureTransport {
    pub fn new(status: u16, body: impl Into<String>) -> Self {
        Self {
            response: ProviderTransportResponse::new(status, body),
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

    fn request_builder(
        &self,
        request: &ProviderTransportRequest,
    ) -> AgentResult<reqwest::RequestBuilder> {
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
        Ok(builder)
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
        let builder = self.request_builder(&request)?;
        let response = builder.send().await.map_err(provider_transport_error)?;
        let status = response.status().as_u16();
        let headers = response_headers(response.headers());
        let body = response.text().await.map_err(provider_transport_error)?;
        Ok(ProviderTransportResponse {
            status,
            headers,
            body,
        })
    }

    async fn send_streaming(
        &self,
        request: ProviderTransportRequest,
        sink: &mut dyn ProviderByteStreamSink,
    ) -> AgentResult<ProviderTransportResponse> {
        let response = self
            .request_builder(&request)?
            .send()
            .await
            .map_err(provider_transport_error)?;
        let status = response.status().as_u16();
        let headers = response_headers(response.headers());
        if !(200..300).contains(&status) {
            let body = response.text().await.map_err(provider_transport_error)?;
            return Ok(ProviderTransportResponse {
                status,
                headers,
                body,
            });
        }

        let mut body = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(provider_transport_error)?;
            sink.push_bytes(&chunk).await?;
            body.extend_from_slice(&chunk);
        }
        Ok(ProviderTransportResponse {
            status,
            headers,
            body: String::from_utf8_lossy(&body).into_owned(),
        })
    }
}

fn response_headers(headers: &HeaderMap) -> BTreeMap<String, String> {
    headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.as_str().to_ascii_lowercase(), value.to_string()))
        })
        .collect()
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
    AgentError::Provider {
        provider: "openai-compatible".to_string(),
        status: None,
        classification: kind.classification().to_string(),
        message: format!("provider transport failed ({})", kind.classification()),
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProviderRetryPolicy {
    pub max_attempts: usize,
    pub retry_statuses: Vec<u16>,
    pub base_delay_millis: u64,
    pub max_delay_millis: u64,
}

impl ProviderRetryPolicy {
    pub fn new(max_attempts: usize) -> Self {
        Self {
            max_attempts: max_attempts.max(1),
            retry_statuses: vec![408, 409, 429, 500, 502, 503, 504],
            base_delay_millis: 100,
            max_delay_millis: 2_000,
        }
    }

    pub fn should_retry_status(&self, status: u16, attempt: usize) -> bool {
        attempt + 1 < self.max_attempts && self.retry_statuses.contains(&status)
    }

    pub fn should_retry_transport_error(&self, error: &AgentError, attempt: usize) -> bool {
        attempt + 1 < self.max_attempts
            && matches!(
                error,
                AgentError::Provider {
                    classification,
                    ..
                } if classification == "network" || classification == "timeout"
            )
    }

    pub fn with_base_delay_millis(mut self, millis: u64) -> Self {
        self.base_delay_millis = millis;
        self
    }

    pub fn with_max_delay_millis(mut self, millis: u64) -> Self {
        self.max_delay_millis = millis;
        self
    }
}

impl Default for ProviderRetryPolicy {
    fn default() -> Self {
        Self::new(3)
    }
}

fn retry_delay_for_response(
    policy: &ProviderRetryPolicy,
    response: &ProviderTransportResponse,
    attempt: usize,
) -> Duration {
    response
        .headers
        .get("retry-after")
        .and_then(|value| retry_after_seconds(value))
        .map(Duration::from_secs)
        .unwrap_or_else(|| retry_backoff_delay(policy, attempt))
}

fn retry_after_seconds(value: &str) -> Option<u64> {
    let value = value.trim();
    value.parse::<u64>().ok().or_else(|| {
        let retry_at = httpdate::parse_http_date(value).ok()?;
        retry_at
            .duration_since(SystemTime::now())
            .ok()
            .map(|duration| duration.as_secs())
    })
}

fn retry_backoff_delay(policy: &ProviderRetryPolicy, attempt: usize) -> Duration {
    if policy.base_delay_millis == 0 {
        return Duration::ZERO;
    }
    let shift = attempt.min(16) as u32;
    let multiplier = 1_u64.checked_shl(shift).unwrap_or(u64::MAX);
    Duration::from_millis(
        policy
            .base_delay_millis
            .saturating_mul(multiplier)
            .min(policy.max_delay_millis),
    )
}

async fn sleep_retry_delay(delay: Duration) {
    if !delay.is_zero() {
        tokio::time::sleep(delay).await;
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub model: String,
    pub base_url: String,
    pub timeout_millis: Option<u64>,
    pub stream: bool,
    pub profile: Option<String>,
    pub wire_api: ProviderWireApi,
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
            profile: None,
            wire_api: ProviderWireApi::ChatCompletions,
            capabilities: ProviderCapabilities::openai_compatible(),
        }
    }

    pub fn deepseek() -> Self {
        Self::openai_compatible("deepseek-v4-flash")
            .with_name("deepseek")
            .with_base_url("https://api.deepseek.com")
            .with_stream(true)
            .with_profile(Some("deepseek".to_string()))
            .with_capabilities(ProviderCapabilities::deepseek_compatible())
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
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

    pub fn with_profile(mut self, profile: Option<String>) -> Self {
        self.profile = profile;
        self
    }

    pub fn with_capabilities(mut self, capabilities: ProviderCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn from_agent_config(config: &AgentConfig) -> Self {
        Self::from_agent_config_with_env(config, |name| std::env::var(name).ok())
    }

    pub fn from_agent_config_with_env<F>(config: &AgentConfig, env: F) -> Self
    where
        F: Fn(&str) -> Option<String>,
    {
        let explicit_profile = non_empty_env(&env, "YUNXI_PROVIDER_PROFILE")
            .or_else(|| non_empty_string(config.provider.clone()));
        let profile = explicit_profile
            .or_else(|| non_empty_env(&env, "DEEPSEEK_API_KEY").map(|_| "deepseek".to_string()));
        let mut provider_config = match profile.as_deref() {
            Some("deepseek") => Self::deepseek(),
            _ => Self::openai_compatible("gpt-4.1"),
        };
        let model = config
            .model
            .clone()
            .and_then(|value| non_empty_string(Some(value)))
            .or_else(|| non_empty_env(&env, "YUNXI_AGENT_MODEL"))
            .unwrap_or_else(|| provider_config.model.clone());
        let base_url = non_empty_env(&env, "YUNXI_PROVIDER_BASE_URL")
            .or_else(|| non_empty_env(&env, "OPENAI_BASE_URL"))
            .unwrap_or_else(|| provider_config.base_url.clone());
        let stream = non_empty_env(&env, "YUNXI_PROVIDER_STREAM")
            .map(|value| !matches!(value.as_str(), "0" | "false" | "False" | "FALSE"))
            .unwrap_or(provider_config.stream);
        provider_config.model = model;
        provider_config.base_url = base_url;
        provider_config.stream = stream;
        provider_config.profile = profile;
        provider_config
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderWireApi {
    ChatCompletions,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProviderBootstrap {
    pub config: ProviderConfig,
    pub auth: ProviderAuth,
}

impl ProviderBootstrap {
    pub fn from_agent_config(config: &AgentConfig) -> Self {
        Self::from_agent_config_with_env(config, |name| std::env::var(name).ok())
    }

    pub fn from_agent_config_with_env<F>(config: &AgentConfig, env: F) -> Self
    where
        F: Fn(&str) -> Option<String>,
    {
        let provider_config = ProviderConfig::from_agent_config_with_env(config, &env);
        let auth = if let Some(api_key) = non_empty_env(&env, "YUNXI_PROVIDER_API_KEY") {
            ProviderAuth::ApiKey(api_key)
        } else if let Some(env_name) = non_empty_env(&env, "YUNXI_PROVIDER_API_KEY_ENV") {
            ProviderAuth::EnvVar(env_name)
        } else if provider_config.profile.as_deref() == Some("deepseek") {
            ProviderAuth::EnvVar("DEEPSEEK_API_KEY".to_string())
        } else {
            ProviderAuth::EnvVar("OPENAI_API_KEY".to_string())
        };
        Self {
            config: provider_config,
            auth,
        }
    }

    pub fn credentials_configured(&self) -> bool {
        self.credentials_configured_with_env(|name| std::env::var(name).ok())
    }

    pub fn credentials_configured_with_env<F>(&self, env: F) -> bool
    where
        F: Fn(&str) -> Option<String>,
    {
        match &self.auth {
            ProviderAuth::None => true,
            ProviderAuth::ApiKey(value) => !value.trim().is_empty(),
            ProviderAuth::EnvVar(name) => non_empty_env(&env, name).is_some(),
        }
    }

    pub fn into_openai_transport_provider(
        self,
    ) -> OpenAiTransportProvider<ReqwestProviderTransport> {
        OpenAiTransportProvider::new(self.config, self.auth, ReqwestProviderTransport::default())
    }
}

fn non_empty_env<F>(env: &F, name: &str) -> Option<String>
where
    F: Fn(&str) -> Option<String>,
{
    non_empty_string(env(name))
}

fn non_empty_string(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.trim().is_empty())
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

    pub fn deepseek_compatible() -> Self {
        Self {
            tools: true,
            parallel_tool_calls: false,
            reasoning: true,
            stream_usage: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProviderFeatureMatrix {
    pub provider: String,
    pub wire_api: ProviderWireApi,
    pub capabilities: ProviderCapabilities,
    pub responses_item_mapping: bool,
    pub reasoning_delta: bool,
    pub usage_delta: bool,
    pub request_metadata: bool,
    pub request_compression: bool,
    pub tool_schema_strictness: ToolSchemaStrictness,
    pub retry_buckets: Vec<ProviderRetryBucket>,
    pub item_kinds: Vec<ProviderResponseItemKind>,
}

impl ProviderFeatureMatrix {
    pub fn from_config(config: &ProviderConfig) -> Self {
        let wire_api = config.wire_api;
        let capabilities = config.capabilities.clone();
        Self {
            provider: config.name.clone(),
            wire_api,
            responses_item_mapping: false,
            reasoning_delta: capabilities.reasoning,
            usage_delta: capabilities.stream_usage,
            request_metadata: true,
            request_compression: false,
            tool_schema_strictness: ToolSchemaStrictness::BestEffort,
            retry_buckets: ProviderRetryBucket::codex_headless_defaults(),
            item_kinds: ProviderResponseItemKind::codex_headless_defaults(),
            capabilities,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolSchemaStrictness {
    BestEffort,
    Strict,
    ProviderRejected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderRetryBucket {
    Auth,
    RateLimit,
    Server,
    Network,
    Timeout,
    BadRequest,
    UnsupportedSchema,
    UnsupportedModel,
}

impl ProviderRetryBucket {
    pub fn codex_headless_defaults() -> Vec<Self> {
        vec![
            Self::Auth,
            Self::RateLimit,
            Self::Server,
            Self::Network,
            Self::Timeout,
            Self::BadRequest,
            Self::UnsupportedSchema,
            Self::UnsupportedModel,
        ]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderResponseItemKind {
    AssistantText,
    Reasoning,
    ToolCall,
    DynamicToolCall,
    McpToolCall,
    Error,
    Usage,
}

impl ProviderResponseItemKind {
    pub fn codex_headless_defaults() -> Vec<Self> {
        vec![
            Self::AssistantText,
            Self::Reasoning,
            Self::ToolCall,
            Self::DynamicToolCall,
            Self::McpToolCall,
            Self::Error,
            Self::Usage,
        ]
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
        if !self.config.stream {
            return Ok(ProviderStream::from_response(
                thread_id,
                turn_id,
                self.complete(request).await?,
            ));
        }
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

struct OpenAiNetworkStreamParser<'a> {
    decoder: ProviderSseDecoder,
    accumulator: OpenAiStreamAccumulator,
    sink: Option<&'a mut dyn ProviderStreamEventSink>,
    initial_emitted: bool,
}

impl<'a> OpenAiNetworkStreamParser<'a> {
    fn new(
        thread_id: impl Into<String>,
        turn_id: impl Into<String>,
        sink: Option<&'a mut dyn ProviderStreamEventSink>,
    ) -> Self {
        Self {
            decoder: ProviderSseDecoder::default(),
            accumulator: OpenAiStreamAccumulator::new(thread_id, turn_id),
            sink,
            initial_emitted: false,
        }
    }

    async fn emit_initial_if_needed(&mut self) -> AgentResult<()> {
        if self.initial_emitted {
            return Ok(());
        }
        self.initial_emitted = true;
        let initial_events = self.accumulator.events();
        self.emit_events(&initial_events).await
    }

    async fn emit_events(&mut self, events: &[StreamEvent]) -> AgentResult<()> {
        if let Some(sink) = self.sink.as_mut() {
            for event in events {
                sink.emit(event.clone()).await?;
            }
        }
        Ok(())
    }

    async fn finish(mut self) -> AgentResult<Vec<StreamEvent>> {
        self.emit_initial_if_needed().await?;
        let events = self.accumulator.finish_incremental(&mut self.decoder)?;
        self.emit_events(&events).await?;
        Ok(self.accumulator.events())
    }
}

#[async_trait]
impl ProviderByteStreamSink for OpenAiNetworkStreamParser<'_> {
    async fn push_bytes(&mut self, chunk: &[u8]) -> AgentResult<()> {
        self.emit_initial_if_needed().await?;
        let text = std::str::from_utf8(chunk).map_err(|error| AgentError::Execution {
            message: format!("provider stream chunk was not valid UTF-8: {error}"),
        })?;
        let events = self.accumulator.push_raw_chunk(&mut self.decoder, text)?;
        self.emit_events(&events).await
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
        self.stream_events_with_sink(request, thread_id, turn_id, None)
            .await
    }

    pub async fn stream_events_with_sink(
        &self,
        request: ProviderRequest,
        thread_id: impl Into<String>,
        turn_id: impl Into<String>,
        sink: Option<&mut dyn ProviderStreamEventSink>,
    ) -> AgentResult<Vec<StreamEvent>> {
        let transport_request = build_openai_transport_request(
            &self.provider.config,
            self.provider.auth(),
            &request,
            true,
        )?;
        let mut parser = OpenAiNetworkStreamParser::new(thread_id, turn_id, sink);
        let response = self
            .send_streaming_with_schema_fallback(transport_request, &mut parser)
            .await?;
        if !response.is_success() {
            return Err(provider_http_error(&self.provider.config, &response));
        }
        parser.finish().await
    }

    async fn send_with_retries(
        &self,
        request: ProviderTransportRequest,
    ) -> AgentResult<ProviderTransportResponse> {
        let mut attempt = 0usize;
        loop {
            match self.transport.send(request.clone()).await {
                Ok(response) => {
                    if !self
                        .retry_policy
                        .should_retry_status(response.status, attempt)
                    {
                        return Ok(response);
                    }
                    let delay = retry_delay_for_response(&self.retry_policy, &response, attempt);
                    attempt += 1;
                    sleep_retry_delay(delay).await;
                }
                Err(error) => {
                    if !self
                        .retry_policy
                        .should_retry_transport_error(&error, attempt)
                    {
                        return Err(error);
                    }
                    let delay = retry_backoff_delay(&self.retry_policy, attempt);
                    attempt += 1;
                    sleep_retry_delay(delay).await;
                }
            }
        }
    }

    async fn send_streaming_with_retries(
        &self,
        request: ProviderTransportRequest,
        sink: &mut dyn ProviderByteStreamSink,
    ) -> AgentResult<ProviderTransportResponse> {
        let mut attempt = 0usize;
        loop {
            match self.transport.send_streaming(request.clone(), sink).await {
                Ok(response) => {
                    if !self
                        .retry_policy
                        .should_retry_status(response.status, attempt)
                    {
                        return Ok(response);
                    }
                    let delay = retry_delay_for_response(&self.retry_policy, &response, attempt);
                    attempt += 1;
                    sleep_retry_delay(delay).await;
                }
                Err(error) => {
                    if !self
                        .retry_policy
                        .should_retry_transport_error(&error, attempt)
                    {
                        return Err(error);
                    }
                    let delay = retry_backoff_delay(&self.retry_policy, attempt);
                    attempt += 1;
                    sleep_retry_delay(delay).await;
                }
            }
        }
    }

    async fn send_with_schema_fallback(
        &self,
        request: ProviderTransportRequest,
    ) -> AgentResult<ProviderTransportResponse> {
        let response = self.send_with_retries(request.clone()).await?;
        if !matches!(response.status, 400 | 422)
            || self.provider.config.profile.as_deref() != Some("deepseek")
        {
            return Ok(response);
        }

        let mut fallback = request;
        let Some(body) = fallback.body.as_object_mut() else {
            return Ok(response);
        };
        if body.remove("metadata").is_none() {
            return Ok(response);
        }

        self.send_with_retries(fallback).await
    }

    async fn send_streaming_with_schema_fallback(
        &self,
        request: ProviderTransportRequest,
        sink: &mut dyn ProviderByteStreamSink,
    ) -> AgentResult<ProviderTransportResponse> {
        let response = self
            .send_streaming_with_retries(request.clone(), sink)
            .await?;
        if !matches!(response.status, 400 | 422)
            || self.provider.config.profile.as_deref() != Some("deepseek")
        {
            return Ok(response);
        }

        let mut fallback = request;
        let Some(body) = fallback.body.as_object_mut() else {
            return Ok(response);
        };
        if body.remove("metadata").is_none() {
            return Ok(response);
        }

        self.send_streaming_with_retries(fallback, sink).await
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
        let response = self.send_with_schema_fallback(transport_request).await?;
        if !response.is_success() {
            return Err(provider_http_error(&self.provider.config, &response));
        }
        self.provider.parse_response_json(&response.body)
    }

    async fn stream(
        &self,
        request: ProviderRequest,
        thread_id: ThreadId,
        turn_id: TurnId,
    ) -> AgentResult<ProviderStream> {
        if !self.provider.config.stream {
            return Ok(ProviderStream::from_response(
                thread_id,
                turn_id,
                self.complete(request).await?,
            ));
        }
        let events = self.stream_events(request, thread_id.0, turn_id.0).await?;
        Ok(ProviderStream::from_events(events))
    }

    async fn stream_with_sink(
        &self,
        request: ProviderRequest,
        thread_id: ThreadId,
        turn_id: TurnId,
        sink: Option<&mut dyn ProviderStreamEventSink>,
    ) -> AgentResult<ProviderStream> {
        if !self.provider.config.stream {
            let stream =
                ProviderStream::from_response(thread_id, turn_id, self.complete(request).await?);
            if let Some(sink) = sink {
                for event in &stream.events {
                    sink.emit(event.clone()).await?;
                }
            }
            return Ok(stream);
        }
        let events = self
            .stream_events_with_sink(request, thread_id.0, turn_id.0, sink)
            .await?;
        Ok(ProviderStream::from_events(events))
    }
}

fn provider_http_error(
    config: &ProviderConfig,
    response: &ProviderTransportResponse,
) -> AgentError {
    let status = response.status;
    let kind = classify_provider_status(status);
    let classification = kind.classification();
    let detail = provider_error_detail(&response.body);
    let detail_suffix = detail
        .as_deref()
        .map(|detail| format!("; detail={detail}"))
        .unwrap_or_default();
    AgentError::Provider {
        provider: config
            .profile
            .clone()
            .unwrap_or_else(|| config.name.clone()),
        status: Some(status),
        classification: classification.to_string(),
        message: format!(
            "provider returned HTTP {status} ({classification}); provider={}, host={}, model={}{}",
            config.profile.as_deref().unwrap_or(config.name.as_str()),
            base_url_host(&config.base_url),
            config.model,
            detail_suffix
        ),
    }
}

fn provider_error_detail(body: &str) -> Option<String> {
    let value: Value = serde_json::from_str(body).ok()?;
    let error = value.get("error").unwrap_or(&value);
    let mut parts = Vec::new();
    for field in ["type", "code"] {
        if let Some(value) = error.get(field).and_then(provider_detail_scalar) {
            let value = redact_sensitive_text(&value);
            if !value.is_empty() {
                parts.push(format!("{field}={value}"));
            }
        }
    }
    if let Some(message) = error.get("message").and_then(Value::as_str) {
        let message = redact_sensitive_text(message);
        if !message.is_empty() {
            parts.push(message);
        }
    }
    let detail = truncate_chars(&parts.join("; "), 240);
    (!detail.is_empty()).then_some(detail)
}

fn provider_detail_scalar(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

pub fn redact_sensitive_text(value: &str) -> String {
    let normalized = strip_terminal_control_sequences(value);
    let mut redact_following = 0usize;
    normalized
        .split_whitespace()
        .map(|part| {
            let trimmed = part
                .trim_matches(|character: char| matches!(character, ',' | ';' | ':' | '"' | '\''));
            let lowered = trimmed.to_ascii_lowercase();
            let authorization = lowered.starts_with("authorization");
            let credential_scheme = matches!(lowered.as_str(), "bearer" | "basic" | "digest");
            let credential_label = matches!(
                lowered.as_str(),
                "api_key"
                    | "api_key="
                    | "apikey"
                    | "apikey="
                    | "access_token"
                    | "access_token="
                    | "token"
                    | "token="
            );
            let token_shaped = lowered.contains("sk-")
                || lowered.contains("ghp_")
                || lowered.contains("github_pat_")
                || lowered.starts_with("api_key=")
                || lowered.starts_with("apikey=")
                || lowered.starts_with("access_token=")
                || lowered.starts_with("token=");
            if redact_following > 0 {
                redact_following -= 1;
                return "[redacted]";
            }
            if authorization {
                redact_following = 2;
                return "[redacted]";
            }
            if credential_scheme {
                redact_following = 1;
                return "[redacted]";
            }
            if credential_label {
                redact_following = 2;
                return "[redacted]";
            }
            if token_shaped { "[redacted]" } else { part }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn strip_terminal_control_sequences(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '\u{1b}' {
            if chars.next_if_eq(&'[').is_some() {
                for sequence_character in chars.by_ref() {
                    if ('@'..='~').contains(&sequence_character) {
                        break;
                    }
                }
            }
            continue;
        }
        if character.is_control() {
            output.push(' ');
        } else {
            output.push(character);
        }
    }
    output
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        truncated.trim_end().to_string()
    } else {
        truncated
    }
}

fn base_url_host(base_url: &str) -> &str {
    let without_scheme = base_url
        .strip_prefix("https://")
        .or_else(|| base_url.strip_prefix("http://"))
        .unwrap_or(base_url);
    without_scheme.split('/').next().unwrap_or(without_scheme)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticProvider {
    response_prefix: String,
    fixtures_enabled: bool,
}

impl StaticProvider {
    pub fn new(response_prefix: impl Into<String>) -> Self {
        Self {
            response_prefix: response_prefix.into(),
            fixtures_enabled: false,
        }
    }

    pub fn with_fixtures_enabled(mut self) -> Self {
        self.fixtures_enabled = true;
        self
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
        if self.fixtures_enabled
            && self.response_prefix == "YunXi autonomous runtime accepted prompt"
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
        if self.fixtures_enabled
            && self.response_prefix == "YunXi autonomous runtime accepted prompt"
            && request
                .input
                .prompt
                .contains("stage 4k child provider fixture")
        {
            if let Some(tool_message) = request
                .messages
                .iter()
                .rev()
                .find(|message| message.role == ProviderRole::Tool)
            {
                return Ok(ProviderResponse::assistant(format!(
                    "Stage 4K child provider fixture completed with child result: {}",
                    tool_message.content.trim()
                )));
            }
            return Ok(ProviderResponse::tool_call(ProviderToolCall::MultiAgent {
                id: Some("stage-4k-child-provider-run".to_string()),
                action: "spawn_run".to_string(),
                arguments_json: Some(
                    r#"{"task":"stage 4k child provider fixture child task"}"#.to_string(),
                ),
            }));
        }
        if self.fixtures_enabled
            && self.response_prefix == "YunXi autonomous runtime accepted prompt"
            && request
                .input
                .prompt
                .contains("stage 4k child scoped stream fixture")
        {
            if let Some(tool_message) = request
                .messages
                .iter()
                .rev()
                .find(|message| message.role == ProviderRole::Tool)
            {
                return Ok(ProviderResponse::assistant(format!(
                    "Stage 4K child scoped stream fixture completed with child result: {}",
                    tool_message.content.trim()
                )));
            }
            return Ok(ProviderResponse::tool_call(ProviderToolCall::MultiAgent {
                id: Some("stage-4k-child-scoped-stream-run".to_string()),
                action: "spawn_run".to_string(),
                arguments_json: Some(
                    r#"{"task":"stage 4k child scoped stream fixture child task"}"#.to_string(),
                ),
            }));
        }
        if self.fixtures_enabled
            && self.response_prefix == "YunXi autonomous runtime accepted prompt"
            && request
                .input
                .prompt
                .contains("stage 4m real parity fixture")
        {
            let tool_messages = request
                .messages
                .iter()
                .filter(|message| message.role == ProviderRole::Tool)
                .map(|message| message.content.trim().to_string())
                .collect::<Vec<_>>();
            if !tool_messages.is_empty() {
                return Ok(ProviderResponse::assistant(format!(
                    "Stage 4M real parity fixture completed through real runtime path with {} tool result(s): {}",
                    tool_messages.len(),
                    tool_messages.join(" | ")
                )));
            }
            return Ok(ProviderResponse {
                message: None,
                tool_calls: vec![
                    ProviderToolCall::Shell {
                        id: Some("stage-4m-shell-1".to_string()),
                        command: "echo YUNXI_STAGE_4M_EXEC_OK".to_string(),
                    },
                    ProviderToolCall::Shell {
                        id: Some("stage-4m-shell-2".to_string()),
                        command: "echo YUNXI_STAGE_4M_EXEC_OK".to_string(),
                    },
                    ProviderToolCall::Patch {
                        id: Some("stage-4m-patch-1".to_string()),
                        patch: r#"{"op":"write","path":"stage4m-runtime.txt","content":"YUNXI_STAGE_4M_PATCH_OK"}"#.to_string(),
                    },
                    ProviderToolCall::Mcp {
                        id: Some("stage-4m-mcp-1".to_string()),
                        server: "local".to_string(),
                        tool: "echo".to_string(),
                        arguments_json: Some(r#"{"text":"first"}"#.to_string()),
                    },
                    ProviderToolCall::Mcp {
                        id: Some("stage-4m-mcp-2".to_string()),
                        server: "local".to_string(),
                        tool: "echo".to_string(),
                        arguments_json: Some(r#"{"text":"second"}"#.to_string()),
                    },
                    ProviderToolCall::Skill {
                        id: Some("stage-4m-skill-1".to_string()),
                        name: "stage4m".to_string(),
                        arguments_json: Some(r#"{"topic":"real-runtime"}"#.to_string()),
                    },
                    ProviderToolCall::ToolSearch {
                        id: Some("stage-4m-search-1".to_string()),
                        query: "stage4m".to_string(),
                    },
                    ProviderToolCall::MultiAgent {
                        id: Some("stage-4m-agent-1".to_string()),
                        action: "spawn_run".to_string(),
                        arguments_json: Some(
                            r#"{"task":"stage4m child runtime task"}"#.to_string(),
                        ),
                    },
                ],
                usage: None,
            });
        }
        if self.fixtures_enabled
            && self.response_prefix.starts_with("YunXi child agent")
            && request.input.prompt.contains("stage4m child runtime task")
        {
            if let Some(tool_message) = request
                .messages
                .iter()
                .rev()
                .find(|message| message.role == ProviderRole::Tool)
            {
                return Ok(ProviderResponse::assistant(format!(
                    "{} with Stage 4M child tool result: {}",
                    self.response_prefix,
                    tool_message.content.trim()
                )));
            }
            return Ok(ProviderResponse::tool_call(ProviderToolCall::Shell {
                id: Some("stage-4m-child-shell".to_string()),
                command: "echo YUNXI_STAGE_4M_CHILD_OK".to_string(),
            }));
        }
        if self.fixtures_enabled
            && self.response_prefix == "YunXi autonomous runtime accepted prompt"
            && request.input.prompt.contains("stage 4k sandbox fixture")
        {
            if let Some(tool_message) = request
                .messages
                .iter()
                .rev()
                .find(|message| message.role == ProviderRole::Tool)
            {
                return Ok(ProviderResponse::assistant(format!(
                    "Stage 4K sandbox fixture completed with shell result: {}",
                    tool_message.content.trim()
                )));
            }
            return Ok(ProviderResponse::tool_call(ProviderToolCall::Shell {
                id: Some("stage-4k-sandbox-shell".to_string()),
                command: "echo YUNXI_SANDBOX_OK".to_string(),
            }));
        }
        if self.fixtures_enabled
            && self.response_prefix == "YunXi autonomous runtime accepted prompt"
            && request.input.prompt.contains("stage 4k mcp reuse fixture")
        {
            let tool_messages = request
                .messages
                .iter()
                .filter(|message| message.role == ProviderRole::Tool)
                .map(|message| message.content.trim().to_string())
                .collect::<Vec<_>>();
            if !tool_messages.is_empty() {
                return Ok(ProviderResponse::assistant(format!(
                    "Stage 4K MCP reuse fixture completed with {} MCP result(s): {}",
                    tool_messages.len(),
                    tool_messages.join(" | ")
                )));
            }
            return Ok(ProviderResponse {
                message: None,
                tool_calls: vec![
                    ProviderToolCall::Mcp {
                        id: Some("stage-4k-mcp-reuse-1".to_string()),
                        server: "local".to_string(),
                        tool: "echo".to_string(),
                        arguments_json: Some(r#"{"text":"first"}"#.to_string()),
                    },
                    ProviderToolCall::Mcp {
                        id: Some("stage-4k-mcp-reuse-2".to_string()),
                        server: "local".to_string(),
                        tool: "echo".to_string(),
                        arguments_json: Some(r#"{"text":"second"}"#.to_string()),
                    },
                ],
                usage: None,
            });
        }
        if self.fixtures_enabled
            && self.response_prefix.starts_with("YunXi child agent")
            && request
                .input
                .prompt
                .contains("stage 4k child scoped stream fixture child task")
        {
            if let Some(tool_message) = request
                .messages
                .iter()
                .rev()
                .find(|message| message.role == ProviderRole::Tool)
            {
                return Ok(ProviderResponse::assistant(format!(
                    "{} with child tool result: {}",
                    self.response_prefix,
                    tool_message.content.trim()
                )));
            }
            return Ok(ProviderResponse::tool_call(ProviderToolCall::Shell {
                id: Some("stage-4k-child-shell".to_string()),
                command: "echo YUNXI_CHILD_TOOL_DELTA".to_string(),
            }));
        }
        if self.fixtures_enabled
            && self.response_prefix.starts_with("YunXi child agent")
            && request
                .input
                .prompt
                .contains("stage 4k child provider fixture child task")
        {
            return Ok(ProviderResponse::assistant(format!(
                "{} via child provider facade: {}",
                self.response_prefix,
                request.input.prompt.trim()
            )));
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
    let matrix = ProviderFeatureMatrix::from_config(provider_config);
    let messages = request
        .messages
        .iter()
        .map(provider_message_request_json)
        .collect::<Vec<_>>();

    let mut body = json!({
        "model": request
            .config
            .model
            .as_deref()
            .unwrap_or(provider_config.model.as_str()),
        "messages": messages
    });
    if matrix.capabilities.tools {
        body["tools"] =
            Value::Array(workspace_tool_registry(&request.config.cwd)?.openai_tools_json());
    }
    if matrix.capabilities.parallel_tool_calls {
        body["parallel_tool_calls"] = Value::Bool(true);
    }
    if matrix.request_metadata {
        body["metadata"] = json!({
            "runtime": "yunxi-agent",
            "provider": matrix.provider,
            "wire_api": format!("{:?}", matrix.wire_api).to_ascii_lowercase(),
            "tool_schema_strictness": format!("{:?}", matrix.tool_schema_strictness).to_ascii_lowercase()
        });
    }
    Ok(body)
}

fn provider_message_request_json(message: &ProviderMessage) -> Value {
    let mut value = json!({
        "role": match message.role {
            ProviderRole::System => "system",
            ProviderRole::User => "user",
            ProviderRole::Assistant => "assistant",
            ProviderRole::Tool => "tool",
        },
        "content": message.content,
    });
    if let Some(tool_call_id) = &message.tool_call_id {
        value["tool_call_id"] = Value::String(tool_call_id.clone());
    }
    if !message.tool_calls.is_empty() {
        if message.content.is_empty() {
            value["content"] = Value::Null;
        }
        value["tool_calls"] = Value::Array(
            message
                .tool_calls
                .iter()
                .enumerate()
                .map(|(index, tool_call)| provider_tool_call_request_json(tool_call, index))
                .collect(),
        );
    }
    value
}

fn provider_tool_call_request_json(tool_call: &ProviderToolCall, index: usize) -> Value {
    let (id, name, arguments) = match tool_call {
        ProviderToolCall::Shell { id, command } => (
            id.as_deref(),
            "shell",
            json!({"command": command}).to_string(),
        ),
        ProviderToolCall::Patch { id, patch } => (
            id.as_deref(),
            "patch",
            serde_json::from_str::<Value>(patch)
                .map(|value| value.to_string())
                .unwrap_or_else(|_| json!({"patch": patch}).to_string()),
        ),
        ProviderToolCall::Mcp {
            id,
            server,
            tool,
            arguments_json,
        } => (
            id.as_deref(),
            "mcp",
            json!({
                "server": server,
                "tool": tool,
                "arguments_json": arguments_json,
            })
            .to_string(),
        ),
        ProviderToolCall::Skill {
            id,
            name,
            arguments_json,
        } => (
            id.as_deref(),
            "skill",
            json!({"name": name, "arguments_json": arguments_json}).to_string(),
        ),
        ProviderToolCall::MultiAgent {
            id,
            action,
            arguments_json,
        } => (
            id.as_deref(),
            "multi_agent",
            json!({"action": action, "arguments_json": arguments_json}).to_string(),
        ),
        ProviderToolCall::ToolSearch { id, query } => (
            id.as_deref(),
            "tool_search",
            json!({"query": query}).to_string(),
        ),
        ProviderToolCall::RequestUserInput { id, prompt } => (
            id.as_deref(),
            "request_user_input",
            json!({"prompt": prompt}).to_string(),
        ),
        ProviderToolCall::ViewImage { id, path } => (
            id.as_deref(),
            "view_image",
            json!({"path": path}).to_string(),
        ),
    };
    json!({
        "id": id.map(ToString::to_string).unwrap_or_else(|| format!("yunxi-call-{index}")),
        "type": "function",
        "function": {
            "name": name,
            "arguments": arguments,
        }
    })
}

pub fn build_openai_stream_request_json(
    provider_config: &ProviderConfig,
    request: &ProviderRequest,
) -> AgentResult<Value> {
    let mut value = build_openai_request_json(provider_config, request)?;
    value["stream"] = Value::Bool(true);
    let matrix = ProviderFeatureMatrix::from_config(provider_config);
    if matrix.usage_delta {
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

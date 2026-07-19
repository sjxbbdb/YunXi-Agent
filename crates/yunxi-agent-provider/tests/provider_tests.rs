use serde_json::json;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use yunxi_agent_core::{AgentConfig, AgentError, AgentInput, AgentResult};
use yunxi_agent_protocol::{
    ResponseItem, ResponseItemDelta, ResponseStatus, StreamEvent, StreamEventSequence, ThreadId,
    ToolCall, TurnId,
};
use yunxi_agent_provider::{
    AgentProvider, FixtureTransport, OpenAiCompatibleProvider, OpenAiStreamAccumulator,
    OpenAiTransportProvider, ProviderAuth, ProviderBootstrap, ProviderByteStreamSink,
    ProviderConfig, ProviderMessage, ProviderRequest, ProviderRetryPolicy, ProviderRole,
    ProviderSseDecoder, ProviderStreamEventSink, ProviderToolCall, ProviderTransport,
    ProviderTransportRequest, ProviderTransportResponse, StaticProvider, build_openai_request_json,
    build_openai_stream_request_json, build_openai_transport_request, parse_openai_response_json,
    parse_openai_stream_events, redact_sensitive_text,
};

#[derive(Clone)]
struct SequenceTransport {
    responses: Arc<Mutex<VecDeque<ProviderTransportResponse>>>,
    requests: Arc<Mutex<Vec<ProviderTransportRequest>>>,
}

impl SequenceTransport {
    fn new(responses: impl IntoIterator<Item = ProviderTransportResponse>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses.into_iter().collect())),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn requests(&self) -> Vec<ProviderTransportRequest> {
        self.requests.lock().expect("request lock").clone()
    }
}

#[async_trait::async_trait]
impl ProviderTransport for SequenceTransport {
    async fn send(
        &self,
        request: ProviderTransportRequest,
    ) -> yunxi_agent_core::AgentResult<ProviderTransportResponse> {
        self.requests.lock().expect("request lock").push(request);
        self.responses
            .lock()
            .expect("response lock")
            .pop_front()
            .ok_or_else(|| yunxi_agent_core::AgentError::Execution {
                message: "sequence transport exhausted".to_string(),
            })
    }
}

#[derive(Clone)]
struct ResultSequenceTransport {
    responses: Arc<Mutex<VecDeque<AgentResult<ProviderTransportResponse>>>>,
    requests: Arc<Mutex<Vec<ProviderTransportRequest>>>,
}

impl ResultSequenceTransport {
    fn new(responses: impl IntoIterator<Item = AgentResult<ProviderTransportResponse>>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses.into_iter().collect())),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn requests(&self) -> Vec<ProviderTransportRequest> {
        self.requests.lock().expect("request lock").clone()
    }
}

#[async_trait::async_trait]
impl ProviderTransport for ResultSequenceTransport {
    async fn send(
        &self,
        request: ProviderTransportRequest,
    ) -> AgentResult<ProviderTransportResponse> {
        self.requests.lock().expect("request lock").push(request);
        self.responses
            .lock()
            .expect("response lock")
            .pop_front()
            .ok_or_else(|| AgentError::Execution {
                message: "result sequence transport exhausted".to_string(),
            })?
    }
}

#[derive(Clone)]
struct ChunkedStreamingTransport {
    chunks: Arc<Vec<&'static [u8]>>,
}

#[async_trait::async_trait]
impl ProviderTransport for ChunkedStreamingTransport {
    async fn send(
        &self,
        _request: ProviderTransportRequest,
    ) -> AgentResult<ProviderTransportResponse> {
        Ok(ProviderTransportResponse::new(200, ""))
    }

    async fn send_streaming(
        &self,
        _request: ProviderTransportRequest,
        sink: &mut dyn ProviderByteStreamSink,
    ) -> AgentResult<ProviderTransportResponse> {
        let mut body = Vec::new();
        for chunk in self.chunks.iter() {
            sink.push_bytes(chunk).await?;
            body.extend_from_slice(chunk);
        }
        Ok(ProviderTransportResponse::new(
            200,
            String::from_utf8_lossy(&body).into_owned(),
        ))
    }
}

#[derive(Clone)]
struct ErrorAfterFirstStreamingChunkTransport {
    attempts: Arc<Mutex<usize>>,
}

impl ErrorAfterFirstStreamingChunkTransport {
    fn new() -> Self {
        Self {
            attempts: Arc::new(Mutex::new(0)),
        }
    }

    fn attempts(&self) -> usize {
        *self.attempts.lock().expect("attempt lock")
    }
}

#[async_trait::async_trait]
impl ProviderTransport for ErrorAfterFirstStreamingChunkTransport {
    async fn send(
        &self,
        _request: ProviderTransportRequest,
    ) -> AgentResult<ProviderTransportResponse> {
        Ok(ProviderTransportResponse::new(200, ""))
    }

    async fn send_streaming(
        &self,
        _request: ProviderTransportRequest,
        sink: &mut dyn ProviderByteStreamSink,
    ) -> AgentResult<ProviderTransportResponse> {
        *self.attempts.lock().expect("attempt lock") += 1;
        sink.push_bytes(b"data: {\"choices\":[{\"delta\":{\"content\":\"first\"}}]}\n\n")
            .await?;
        Err(AgentError::Provider {
            provider: "fixture".to_string(),
            status: None,
            classification: "network".to_string(),
            message: "fixture stream disconnected".to_string(),
        })
    }
}

#[derive(Default)]
struct CollectingStreamEventSink {
    events: Vec<StreamEvent>,
}

#[async_trait::async_trait]
impl ProviderStreamEventSink for CollectingStreamEventSink {
    async fn emit(&mut self, event: StreamEvent) -> AgentResult<()> {
        self.events.push(event);
        Ok(())
    }
}

#[tokio::test]
async fn static_provider_returns_yunxi_runtime_message() {
    let provider = StaticProvider::default();
    let request = ProviderRequest::new(
        AgentConfig::new(PathBuf::from(".")),
        AgentInput::text("explain this project"),
    );

    let response = provider
        .complete(request)
        .await
        .expect("provider response should succeed");

    assert_eq!(
        response
            .message
            .as_ref()
            .map(|message| message.content.as_str()),
        Some("YunXi autonomous runtime accepted prompt: explain this project")
    );
}

#[test]
fn openai_request_json_uses_yunxi_provider_messages() {
    let request = ProviderRequest::new(
        AgentConfig::new(PathBuf::from(".")).with_model("yunxi-model"),
        AgentInput::text("explain this project"),
    );

    let json = build_openai_request_json(
        &ProviderConfig::openai_compatible("fallback-model"),
        &request,
    )
    .expect("request json");

    assert_eq!(json["model"], "yunxi-model");
    assert_eq!(json["messages"][0]["role"], "user");
    assert_eq!(json["messages"][0]["content"], "explain this project");
    assert_eq!(json["metadata"]["runtime"], "yunxi-agent");
    assert_eq!(json["metadata"]["provider"], "openai-compatible");

    let tools = json["tools"].as_array().expect("tools");
    let tool_names = tools
        .iter()
        .map(|tool| {
            tool.pointer("/function/name")
                .and_then(serde_json::Value::as_str)
                .expect("tool name")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        tool_names,
        vec![
            "shell",
            "patch",
            "mcp",
            "skill",
            "multi_agent",
            "tool_search",
            "request_user_input",
            "view_image"
        ]
    );
    assert_eq!(
        tools[0]["function"]["parameters"]["required"],
        json!(["command"])
    );
    assert_eq!(
        tools[2]["function"]["parameters"]["required"],
        json!(["server", "tool"])
    );
    assert_eq!(json["parallel_tool_calls"], true);
}

#[test]
fn openai_request_json_preserves_assistant_tool_calls_and_tool_result_ids() {
    let request = ProviderRequest::with_messages(
        AgentConfig::new(PathBuf::from(".")),
        AgentInput::text("run a tool"),
        vec![
            ProviderMessage::user("run a tool"),
            ProviderMessage::assistant_with_tool_calls(
                "",
                vec![ProviderToolCall::Shell {
                    id: Some("call-1".to_string()),
                    command: "echo hello".to_string(),
                }],
            ),
            ProviderMessage::tool_result("call-1", "hello"),
        ],
    );

    let json =
        build_openai_request_json(&ProviderConfig::deepseek(), &request).expect("request json");

    assert_eq!(json["messages"][1]["role"], "assistant");
    assert_eq!(json["messages"][1]["tool_calls"][0]["id"], "call-1");
    assert_eq!(
        json["messages"][1]["tool_calls"][0]["function"]["name"],
        "shell"
    );
    assert_eq!(
        json["messages"][1]["tool_calls"][0]["function"]["arguments"],
        r#"{"command":"echo hello"}"#
    );
    assert_eq!(json["messages"][2]["role"], "tool");
    assert_eq!(json["messages"][2]["tool_call_id"], "call-1");
    assert_eq!(json["messages"][2]["content"], "hello");
}

#[test]
fn openai_request_json_includes_workspace_dynamic_tools() {
    let temp = TempDir::new().expect("temp dir");
    let skill_dir = temp.path().join(".yunxi/skills/writer");
    std::fs::create_dir_all(&skill_dir).expect("skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: writer\ndescription: writes reports\n---\n# Writer\n",
    )
    .expect("skill file");
    let request = ProviderRequest::new(
        AgentConfig::new(temp.path()).with_model("yunxi-model"),
        AgentInput::text("use skill"),
    );

    let json = build_openai_request_json(
        &ProviderConfig::openai_compatible("fallback-model"),
        &request,
    )
    .expect("request json");
    let tool_names = json["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .map(|tool| {
            tool.pointer("/function/name")
                .and_then(serde_json::Value::as_str)
                .expect("tool name")
        })
        .collect::<Vec<_>>();

    assert!(tool_names.contains(&"shell"));
    assert!(tool_names.contains(&"skill__writer"));
}

#[test]
fn openai_stream_request_json_enables_stream_usage() {
    let request = ProviderRequest::new(
        AgentConfig::new(PathBuf::from(".")).with_model("yunxi-model"),
        AgentInput::text("stream this"),
    );

    let json = build_openai_stream_request_json(
        &ProviderConfig::openai_compatible("fallback-model"),
        &request,
    )
    .expect("request json");

    assert_eq!(json["stream"], true);
    assert_eq!(json["stream_options"]["include_usage"], true);
    assert_eq!(json["tools"][0]["function"]["name"], "shell");
}

#[test]
fn openai_transport_request_uses_provider_boundary_and_auth() {
    let request = ProviderRequest::new(
        AgentConfig::new(PathBuf::from(".")).with_model("yunxi-model"),
        AgentInput::text("transport"),
    );

    let transport = build_openai_transport_request(
        &ProviderConfig::openai_compatible("fallback-model")
            .with_base_url("https://example.test/v1"),
        &ProviderAuth::ApiKey("secret".to_string()),
        &request,
        true,
    )
    .expect("transport request");

    assert_eq!(transport.method, "POST");
    assert_eq!(transport.url, "https://example.test/v1/chat/completions");
    assert!(transport.stream);
    assert_eq!(
        transport.headers.get("authorization").map(String::as_str),
        Some("Bearer secret")
    );
    assert_eq!(transport.body["stream"], true);
    assert_eq!(transport.timeout_millis, Some(120_000));
}

#[test]
fn provider_capabilities_can_disable_tools_and_stream_usage() {
    let request = ProviderRequest::new(
        AgentConfig::new(PathBuf::from(".")),
        AgentInput::text("capabilities"),
    );
    let config = ProviderConfig::openai_compatible("fallback-model")
        .with_capabilities(yunxi_agent_provider::ProviderCapabilities {
            tools: false,
            parallel_tool_calls: false,
            reasoning: false,
            stream_usage: false,
        })
        .with_timeout_millis(Some(5_000));

    let json = build_openai_stream_request_json(&config, &request).expect("stream json");
    let transport = build_openai_transport_request(&config, &ProviderAuth::None, &request, true)
        .expect("transport");

    assert!(json.get("tools").is_none());
    assert!(json.get("parallel_tool_calls").is_none());
    assert!(json.get("stream_options").is_none());
    assert_eq!(json["metadata"]["tool_schema_strictness"], "besteffort");
    assert_eq!(transport.timeout_millis, Some(5_000));
}

#[test]
fn deepseek_profile_sets_provider_neutral_defaults() {
    let config = ProviderConfig::deepseek();

    assert_eq!(config.name, "deepseek");
    assert_eq!(config.profile.as_deref(), Some("deepseek"));
    assert_eq!(config.model, "deepseek-v4-flash");
    assert_eq!(config.base_url, "https://api.deepseek.com");
    assert!(config.stream);
    assert!(config.capabilities.tools);
    assert!(!config.capabilities.parallel_tool_calls);
    assert!(!config.capabilities.stream_usage);
}

#[test]
fn provider_bootstrap_auto_selects_deepseek_for_deepseek_credentials() {
    let env = HashMap::from([("DEEPSEEK_API_KEY", "deepseek-secret")]);
    let bootstrap = ProviderBootstrap::from_agent_config_with_env(
        &AgentConfig::new(PathBuf::from(".")),
        |name| env.get(name).map(|value| (*value).to_string()),
    );

    assert_eq!(bootstrap.config.name, "deepseek");
    assert_eq!(bootstrap.config.profile.as_deref(), Some("deepseek"));
    assert_eq!(bootstrap.config.model, "deepseek-v4-flash");
    assert_eq!(
        bootstrap.auth,
        ProviderAuth::EnvVar("DEEPSEEK_API_KEY".to_string())
    );
    assert!(bootstrap.credentials_configured_with_env(|name| {
        env.get(name).map(|value| (*value).to_string())
    }));
}

#[test]
fn provider_bootstrap_prefers_direct_yunxi_api_key() {
    let env = HashMap::from([
        ("YUNXI_PROVIDER_API_KEY", "yunxi-secret"),
        ("DEEPSEEK_API_KEY", "deepseek-secret"),
    ]);
    let bootstrap = ProviderBootstrap::from_agent_config_with_env(
        &AgentConfig::new(PathBuf::from(".")),
        |name| env.get(name).map(|value| (*value).to_string()),
    );

    assert_eq!(bootstrap.config.profile.as_deref(), Some("deepseek"));
    assert_eq!(
        bootstrap.auth,
        ProviderAuth::ApiKey("yunxi-secret".to_string())
    );
    assert!(bootstrap.credentials_configured_with_env(|name| {
        env.get(name).map(|value| (*value).to_string())
    }));
}

#[test]
fn provider_bootstrap_honors_named_api_key_environment_variable() {
    let env = HashMap::from([
        ("YUNXI_PROVIDER_PROFILE", "deepseek"),
        ("YUNXI_PROVIDER_API_KEY_ENV", "CUSTOM_PROVIDER_KEY"),
        ("CUSTOM_PROVIDER_KEY", "custom-secret"),
        ("DEEPSEEK_API_KEY", "deepseek-secret"),
    ]);
    let bootstrap = ProviderBootstrap::from_agent_config_with_env(
        &AgentConfig::new(PathBuf::from(".")),
        |name| env.get(name).map(|value| (*value).to_string()),
    );

    assert_eq!(
        bootstrap.auth,
        ProviderAuth::EnvVar("CUSTOM_PROVIDER_KEY".to_string())
    );
    assert!(bootstrap.credentials_configured_with_env(|name| {
        env.get(name).map(|value| (*value).to_string())
    }));
}

#[test]
fn provider_bootstrap_ignores_empty_credentials() {
    let env = HashMap::from([
        ("YUNXI_PROVIDER_API_KEY", "   "),
        ("DEEPSEEK_API_KEY", ""),
        ("OPENAI_API_KEY", "\t"),
    ]);
    let bootstrap = ProviderBootstrap::from_agent_config_with_env(
        &AgentConfig::new(PathBuf::from(".")),
        |name| env.get(name).map(|value| (*value).to_string()),
    );

    assert_eq!(bootstrap.config.profile, None);
    assert_eq!(
        bootstrap.auth,
        ProviderAuth::EnvVar("OPENAI_API_KEY".to_string())
    );
    assert!(!bootstrap.credentials_configured_with_env(|name| {
        env.get(name).map(|value| (*value).to_string())
    }));
}

#[test]
fn explicit_provider_profile_prevents_deepseek_inference() {
    let env = HashMap::from([
        ("YUNXI_PROVIDER_PROFILE", "openai-compatible"),
        ("DEEPSEEK_API_KEY", "deepseek-secret"),
        ("OPENAI_API_KEY", "openai-secret"),
    ]);
    let bootstrap = ProviderBootstrap::from_agent_config_with_env(
        &AgentConfig::new(PathBuf::from(".")),
        |name| env.get(name).map(|value| (*value).to_string()),
    );

    assert_eq!(bootstrap.config.name, "openai-compatible");
    assert_eq!(
        bootstrap.config.profile.as_deref(),
        Some("openai-compatible")
    );
    assert_eq!(
        bootstrap.auth,
        ProviderAuth::EnvVar("OPENAI_API_KEY".to_string())
    );
}

#[test]
fn provider_bootstrap_uses_openai_key_for_openai_compatible_profile() {
    let env = HashMap::from([("OPENAI_API_KEY", "openai-secret")]);
    let bootstrap = ProviderBootstrap::from_agent_config_with_env(
        &AgentConfig::new(PathBuf::from(".")),
        |name| env.get(name).map(|value| (*value).to_string()),
    );

    assert_eq!(bootstrap.config.name, "openai-compatible");
    assert_eq!(bootstrap.config.profile, None);
    assert_eq!(
        bootstrap.auth,
        ProviderAuth::EnvVar("OPENAI_API_KEY".to_string())
    );
    assert!(bootstrap.credentials_configured_with_env(|name| {
        env.get(name).map(|value| (*value).to_string())
    }));
}

#[tokio::test]
async fn provider_http_errors_are_classified_and_redacted() {
    let provider = OpenAiTransportProvider::new(
        ProviderConfig::deepseek(),
        ProviderAuth::ApiKey("test-secret-value-that-must-not-leak".to_string()),
        FixtureTransport::new(401, r#"{"error":"bad key"}"#),
    );
    let request = ProviderRequest::new(
        AgentConfig::new(PathBuf::from(".")),
        AgentInput::text("transport"),
    );

    let error = provider.complete(request).await.expect_err("auth error");
    let rendered = error.to_string();

    assert!(matches!(
        error,
        yunxi_agent_core::AgentError::Provider {
            status: Some(401),
            ref classification,
            ..
        } if classification == "auth_error"
    ));
    assert!(rendered.contains("provider returned HTTP 401"));
    assert!(!rendered.contains("test-secret-value"));
    assert!(!rendered.contains("Authorization"));
    assert!(!rendered.contains("Bearer"));
}

#[tokio::test]
async fn deepseek_schema_error_retries_once_without_metadata() {
    let transport = SequenceTransport::new([
        ProviderTransportResponse::new(400, r#"{"error":{"message":"unknown field metadata"}}"#),
        ProviderTransportResponse::new(
            200,
            r#"{"choices":[{"message":{"role":"assistant","content":"fallback worked"}}]}"#,
        ),
    ]);
    let provider = OpenAiTransportProvider::new(
        ProviderConfig::deepseek().with_stream(false),
        ProviderAuth::ApiKey("fixture-key".to_string()),
        transport.clone(),
    );
    let request = ProviderRequest::new(
        AgentConfig::new(PathBuf::from(".")),
        AgentInput::text("metadata fallback"),
    );

    let response = provider.complete(request).await.expect("fallback response");
    let requests = transport.requests();

    assert_eq!(
        response.message.map(|message| message.content),
        Some("fallback worked".to_string())
    );
    assert_eq!(requests.len(), 2);
    assert!(requests[0].body.get("metadata").is_some());
    assert!(requests[1].body.get("metadata").is_none());
    assert!(
        requests
            .iter()
            .all(|request| request.body.get("tools").is_some())
    );
    assert!(
        requests
            .iter()
            .all(|request| request.body.get("messages").is_some())
    );
}

#[tokio::test]
async fn non_deepseek_schema_error_does_not_use_metadata_fallback() {
    let transport = SequenceTransport::new([ProviderTransportResponse::new(
        400,
        r#"{"error":{"message":"invalid request"}}"#,
    )]);
    let provider = OpenAiTransportProvider::new(
        ProviderConfig::openai_compatible("fixture-model").with_stream(false),
        ProviderAuth::None,
        transport.clone(),
    );
    let request = ProviderRequest::new(
        AgentConfig::new(PathBuf::from(".")),
        AgentInput::text("no fallback"),
    );

    provider.complete(request).await.expect_err("schema error");

    assert_eq!(transport.requests().len(), 1);
}

#[tokio::test]
async fn provider_schema_error_includes_only_bounded_redacted_details() {
    let secret = "sk-fixture";
    let opaque_secret = "opaque-must-not-leak";
    let long_tail = "x".repeat(400);
    let body = json!({
        "error": {
            "message": format!(
                "\u{1b}[31mAuthorization: {} {opaque_secret} api_key={secret} unknown field metadata {long_tail}",
                "Bearer"
            ),
            "type": "invalid_request_error",
            "code": "unsupported_schema"
        },
        "raw_private_field": secret
    })
    .to_string();
    let transport = SequenceTransport::new([
        ProviderTransportResponse::new(400, body.clone()),
        ProviderTransportResponse::new(400, body),
    ]);
    let provider = OpenAiTransportProvider::new(
        ProviderConfig::deepseek().with_stream(false),
        ProviderAuth::ApiKey("fixture-key".to_string()),
        transport,
    );
    let request = ProviderRequest::new(
        AgentConfig::new(PathBuf::from(".")),
        AgentInput::text("safe diagnostics"),
    );

    let error = provider.complete(request).await.expect_err("schema error");
    let rendered = error.to_string();
    let detail = rendered.split("; detail=").nth(1).expect("safe detail");

    assert!(rendered.contains("unknown field metadata"));
    assert!(rendered.contains("type=invalid_request_error"));
    assert!(rendered.contains("code=unsupported_schema"));
    assert!(!rendered.contains(secret));
    assert!(!rendered.contains(opaque_secret));
    assert!(!rendered.contains("Bearer"));
    assert!(!rendered.contains("Authorization"));
    assert!(!rendered.contains('\u{1b}'));
    assert!(!rendered.contains("raw_private_field"));
    assert!(detail.chars().count() <= 240);
}

#[test]
fn sensitive_text_redaction_removes_authorization_values_and_terminal_controls() {
    let rendered = redact_sensitive_text(concat!(
        "\u{1b}[31mAuthori",
        "zation: Bas",
        "ic opaque-basic safe-detail ",
        "api_key: opaque-colon safe-after-colon api_key = opaque-spaced safe-tail",
    ));

    assert!(!rendered.contains("Authorization"));
    assert!(!rendered.contains("Basic"));
    assert!(!rendered.contains("opaque-basic"));
    assert!(!rendered.contains("opaque-colon"));
    assert!(!rendered.contains("opaque-spaced"));
    assert!(!rendered.contains('\u{1b}'));
    assert!(rendered.contains("safe-tail"));
}

#[test]
fn sensitive_text_redaction_removes_two_word_api_key_values() {
    let fake_value = ["fake", "credential", "value"].join("-");
    let rendered = redact_sensitive_text(&format!(
        "Remember my api key is {fake_value} and keep this safe-tail"
    ));

    assert!(!rendered.contains(&fake_value));
    assert!(!rendered.to_ascii_lowercase().contains("api key"));
    assert!(rendered.contains("safe-tail"));
}

#[tokio::test]
async fn fixture_transport_returns_configured_response() {
    let transport = FixtureTransport::new(200, r#"{"ok":true}"#);
    let response = transport
        .send(yunxi_agent_provider::ProviderTransportRequest::post_json(
            "https://example.test",
            json!({"hello":"yunxi"}),
        ))
        .await
        .expect("fixture transport");

    assert!(response.is_success());
    assert_eq!(response.body, r#"{"ok":true}"#);
}

#[tokio::test]
async fn transported_openai_provider_uses_transport_for_completion() {
    let provider = OpenAiTransportProvider::new(
        ProviderConfig::openai_compatible("fixture-model"),
        ProviderAuth::None,
        FixtureTransport::new(
            200,
            r#"{
              "choices": [
                { "message": { "role": "assistant", "content": "transport answer" } }
              ]
            }"#,
        ),
    );
    let request = ProviderRequest::new(
        AgentConfig::new(PathBuf::from(".")),
        AgentInput::text("transport"),
    );

    let response = provider.complete(request).await.expect("completion");

    assert_eq!(
        response.message.map(|message| message.content),
        Some("transport answer".to_string())
    );
}

#[tokio::test]
async fn transported_openai_provider_uses_transport_for_streaming() {
    let provider = OpenAiTransportProvider::new(
        ProviderConfig::openai_compatible("fixture-model"),
        ProviderAuth::None,
        FixtureTransport::new(
            200,
            "data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\ndata: [DONE]\n",
        ),
    );
    let request = ProviderRequest::new(
        AgentConfig::new(PathBuf::from(".")),
        AgentInput::text("stream"),
    );

    let events = provider
        .stream_events(request, "thread-1", "turn-1")
        .await
        .expect("stream");

    assert!(events.iter().any(|event| matches!(
        event,
        StreamEvent::ItemDelta {
            delta: ResponseItemDelta::MessageContent { delta, .. },
            ..
        } if delta == "hi"
    )));
}

#[tokio::test]
async fn provider_default_stream_wraps_completion_response() {
    let provider = StaticProvider::default();
    let request = ProviderRequest::new(
        AgentConfig::new(PathBuf::from(".")),
        AgentInput::text("stream fallback"),
    );

    let stream = provider
        .stream(
            request,
            ThreadId("thread-fallback".to_string()),
            TurnId("turn-fallback".to_string()),
        )
        .await
        .expect("stream fallback");

    assert!(stream.events.iter().any(|event| matches!(
        event,
        StreamEvent::ItemCompleted {
            item: ResponseItem::Message { content, .. },
            ..
        } if content.contains("stream fallback")
    )));
    assert!(stream.final_response.is_some());
}

#[test]
fn openai_chat_stream_aggregates_tool_call_deltas() {
    let events = parse_openai_stream_events(
        "thread-tools",
        "turn-tools",
        concat!(
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call-1\",\"type\":\"function\",\"function\":{\"name\":\"shell\",\"arguments\":\"{\\\"command\\\":\"}}]}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"\\\"echo streamed\\\"}\"}}]}}]}\n\n",
            "data: {\"choices\":[{\"finish_reason\":\"tool_calls\"}]}\n\n"
        ),
    )
    .expect("stream events");

    assert!(events.iter().any(|event| matches!(
        event,
        StreamEvent::ItemCompleted {
            item: ResponseItem::FunctionCall {
                call_id,
                name,
                arguments,
                ..
            },
            ..
        } if call_id == "call-1" && name == "shell" && arguments.contains("echo streamed")
    )));
}

#[test]
fn retry_policy_retries_transient_statuses_until_attempt_budget_is_exhausted() {
    let policy = ProviderRetryPolicy::new(3);

    assert!(policy.should_retry_status(429, 0));
    assert!(policy.should_retry_status(503, 1));
    assert!(!policy.should_retry_status(503, 2));
    assert!(!policy.should_retry_status(400, 0));
}

#[tokio::test]
async fn retries_429_after_retry_after_header() {
    let transport = SequenceTransport::new([
        ProviderTransportResponse::new(429, r#"{"error":"slow down"}"#)
            .with_header("retry-after", "0"),
        ProviderTransportResponse::new(
            200,
            r#"{"choices":[{"message":{"role":"assistant","content":"retried"}}]}"#,
        ),
    ]);
    let provider = OpenAiTransportProvider::new(
        ProviderConfig::openai_compatible("fixture-model").with_stream(false),
        ProviderAuth::None,
        transport.clone(),
    )
    .with_retry_policy(ProviderRetryPolicy::new(3).with_base_delay_millis(0));
    let request = ProviderRequest::new(
        AgentConfig::new(PathBuf::from(".")),
        AgentInput::text("retry status"),
    );

    let response = provider.complete(request).await.expect("retry response");

    assert_eq!(
        response.message.map(|message| message.content),
        Some("retried".to_string())
    );
    assert_eq!(transport.requests().len(), 2);
}

#[tokio::test]
async fn retries_timeout_transport_error() {
    let transport = ResultSequenceTransport::new([
        Err(AgentError::Provider {
            provider: "fixture".to_string(),
            status: None,
            classification: "timeout".to_string(),
            message: "fixture timeout".to_string(),
        }),
        Ok(ProviderTransportResponse::new(
            200,
            r#"{"choices":[{"message":{"role":"assistant","content":"timeout retried"}}]}"#,
        )),
    ]);
    let provider = OpenAiTransportProvider::new(
        ProviderConfig::openai_compatible("fixture-model").with_stream(false),
        ProviderAuth::None,
        transport.clone(),
    )
    .with_retry_policy(ProviderRetryPolicy::new(2).with_base_delay_millis(0));
    let request = ProviderRequest::new(
        AgentConfig::new(PathBuf::from(".")),
        AgentInput::text("retry timeout"),
    );

    let response = provider.complete(request).await.expect("retry response");

    assert_eq!(
        response.message.map(|message| message.content),
        Some("timeout retried".to_string())
    );
    assert_eq!(transport.requests().len(), 2);
}

#[tokio::test]
async fn does_not_retry_non_retryable_400() {
    let transport = SequenceTransport::new([ProviderTransportResponse::new(
        400,
        r#"{"error":{"message":"bad request"}}"#,
    )]);
    let provider = OpenAiTransportProvider::new(
        ProviderConfig::openai_compatible("fixture-model").with_stream(false),
        ProviderAuth::None,
        transport.clone(),
    )
    .with_retry_policy(ProviderRetryPolicy::new(3).with_base_delay_millis(0));
    let request = ProviderRequest::new(
        AgentConfig::new(PathBuf::from(".")),
        AgentInput::text("no retry"),
    );

    provider.complete(request).await.expect_err("bad request");

    assert_eq!(transport.requests().len(), 1);
}

#[tokio::test]
async fn streaming_transport_pushes_network_chunks_incrementally() {
    let provider = OpenAiTransportProvider::new(
        ProviderConfig::openai_compatible("fixture-model"),
        ProviderAuth::None,
        ChunkedStreamingTransport {
            chunks: Arc::new(vec![
                b"data: {\"choices\":[{\"delta\":{\"content\":\"hel\"}}]}\n\n".as_slice(),
                b"data: {\"choices\":[{\"delta\":{\"content\":\"lo\"},\"finish_reason\":\"stop\"}]}\n\n"
                    .as_slice(),
            ]),
        },
    );
    let request = ProviderRequest::new(
        AgentConfig::new(PathBuf::from(".")),
        AgentInput::text("chunked stream"),
    );

    let events = provider
        .stream_events(request, "thread-chunked", "turn-chunked")
        .await
        .expect("chunked stream");

    assert!(events.iter().any(|event| matches!(
        event,
        StreamEvent::ItemDelta {
            delta: ResponseItemDelta::MessageContent { delta, .. },
            ..
        } if delta == "hel"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        StreamEvent::ItemDelta {
            delta: ResponseItemDelta::MessageContent { delta, .. },
            ..
        } if delta == "lo"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        StreamEvent::ResponseCompleted {
            status: ResponseStatus::Completed,
            ..
        }
    )));
}

#[tokio::test]
async fn streaming_retries_429_before_body_starts() {
    let transport = SequenceTransport::new([
        ProviderTransportResponse::new(429, r#"{"error":"slow down"}"#)
            .with_header("retry-after", "0"),
        ProviderTransportResponse::new(
            200,
            "data: {\"choices\":[{\"delta\":{\"content\":\"retried\"},\"finish_reason\":\"stop\"}]}\n\n",
        ),
    ]);
    let provider = OpenAiTransportProvider::new(
        ProviderConfig::openai_compatible("fixture-model"),
        ProviderAuth::None,
        transport.clone(),
    )
    .with_retry_policy(ProviderRetryPolicy::new(2).with_base_delay_millis(0));
    let request = ProviderRequest::new(
        AgentConfig::new(PathBuf::from(".")),
        AgentInput::text("stream retry"),
    );

    let events = provider
        .stream_events(request, "thread-retry", "turn-retry")
        .await
        .expect("stream retry");

    assert_eq!(transport.requests().len(), 2);
    assert!(events.iter().any(|event| matches!(
        event,
        StreamEvent::ItemDelta {
            delta: ResponseItemDelta::MessageContent { delta, .. },
            ..
        } if delta == "retried"
    )));
}

#[tokio::test]
async fn streaming_does_not_retry_after_body_started() {
    let transport = ErrorAfterFirstStreamingChunkTransport::new();
    let provider = OpenAiTransportProvider::new(
        ProviderConfig::openai_compatible("fixture-model"),
        ProviderAuth::None,
        transport.clone(),
    )
    .with_retry_policy(ProviderRetryPolicy::new(3).with_base_delay_millis(0));
    let request = ProviderRequest::new(
        AgentConfig::new(PathBuf::from(".")),
        AgentInput::text("stream disconnect"),
    );
    let mut sink = CollectingStreamEventSink::default();

    let error = provider
        .stream_events_with_sink(
            request,
            "thread-disconnect",
            "turn-disconnect",
            Some(&mut sink),
        )
        .await
        .expect_err("stream disconnect should not retry");

    assert!(matches!(
        error,
        AgentError::Provider {
            classification,
            ..
        } if classification == "network"
    ));
    assert_eq!(transport.attempts(), 1);
    let first_deltas = sink
        .events
        .iter()
        .filter(|event| {
            matches!(
                event,
                StreamEvent::ItemDelta {
                    delta: ResponseItemDelta::MessageContent { delta, .. },
                    ..
                } if delta == "first"
            )
        })
        .count();
    assert_eq!(first_deltas, 1);
}

#[test]
fn openai_response_json_parses_assistant_text_and_usage() {
    let response = parse_openai_response_json(
        r#"{
          "choices": [
            { "message": { "role": "assistant", "content": "hello from fixture" } }
          ],
          "usage": {
            "prompt_tokens": 7,
            "completion_tokens": 11,
            "prompt_tokens_details": { "cached_tokens": 3 },
            "completion_tokens_details": { "reasoning_tokens": 5 }
          }
        }"#,
    )
    .expect("provider response");

    assert_eq!(
        response
            .message
            .as_ref()
            .map(|message| message.content.as_str()),
        Some("hello from fixture")
    );
    assert!(response.tool_calls.is_empty());
    let usage = response.usage.expect("usage");
    assert_eq!(usage.input_tokens, 7);
    assert_eq!(usage.cached_input_tokens, 3);
    assert_eq!(usage.output_tokens, 11);
    assert_eq!(usage.reasoning_output_tokens, 5);
}

#[test]
fn openai_chat_stream_fixture_maps_to_yunxi_stream_events() {
    let events = parse_openai_stream_events(
        "thread-stream",
        "turn-stream",
        r#"data: {"choices":[{"delta":{"content":"hel"}}]}
data: {"choices":[{"delta":{"content":"lo"}}]}
data: [DONE]
"#,
    )
    .expect("stream events");

    assert!(matches!(
        events.first(),
        Some(StreamEvent::ResponseStarted { .. })
    ));
    assert!(events.iter().any(|event| matches!(
        event,
        StreamEvent::ItemDelta {
            delta: ResponseItemDelta::MessageContent { delta, .. },
            ..
        } if delta == "hel"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        StreamEvent::ResponseCompleted {
            status: ResponseStatus::Completed,
            ..
        }
    )));
}

#[test]
fn chat_stream_assigns_stable_distinct_local_fallback_event_ids() {
    let fixture = r#"data: {"choices":[{"delta":{"content":"same"}}]}
data: {"choices":[{"delta":{"content":"same"}}]}
data: [DONE]
"#;
    let first = parse_openai_stream_events("thread-chat", "turn-chat", fixture)
        .expect("first stream events");
    let replay = parse_openai_stream_events("thread-chat", "turn-chat", fixture)
        .expect("replayed stream events");

    let metadata = |events: &[StreamEvent]| {
        events
            .iter()
            .filter_map(|event| match event {
                StreamEvent::ItemDelta {
                    metadata: Some(metadata),
                    ..
                } => Some(metadata.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let first_metadata = metadata(&first);
    let replay_metadata = metadata(&replay);

    assert_eq!(first_metadata.len(), 2);
    assert_ne!(first_metadata[0].event_id, first_metadata[1].event_id);
    assert_eq!(first_metadata, replay_metadata);
    assert_eq!(
        first_metadata
            .iter()
            .map(|metadata| metadata.sequence)
            .collect::<Vec<_>>(),
        vec![
            StreamEventSequence::LocalFallback(1),
            StreamEventSequence::LocalFallback(2)
        ]
    );
}

#[test]
fn responses_stream_preserves_provider_reliable_sequence_in_event_identity() {
    let fixture = r#"data: {"type":"response.output_text.delta","event_id":"evt-a","sequence_number":41,"item_id":"message-1","delta":"same"}
data: {"type":"response.output_text.delta","event_id":"evt-b","sequence_number":42,"item_id":"message-1","delta":"same"}
data: {"type":"response.completed"}
"#;
    let events = parse_openai_stream_events("thread-response", "turn-response", fixture)
        .expect("responses stream events");
    let metadata = events
        .iter()
        .filter_map(|event| match event {
            StreamEvent::ItemDelta {
                metadata: Some(metadata),
                ..
            } => Some(metadata),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(metadata.len(), 2);
    assert_ne!(metadata[0].event_id, metadata[1].event_id);
    assert_eq!(
        metadata
            .iter()
            .map(|metadata| metadata.sequence)
            .collect::<Vec<_>>(),
        vec![
            StreamEventSequence::ProviderReliable(41),
            StreamEventSequence::ProviderReliable(42)
        ]
    );
    assert!(metadata[0].event_id.contains("evt-a"));
    assert!(metadata[1].event_id.contains("evt-b"));
}

#[test]
fn openai_responses_stream_fixture_maps_reasoning_and_tool_argument_deltas() {
    let events = parse_openai_stream_events(
        "thread-response",
        "turn-response",
        r#"data: {"type":"response.reasoning_text.delta","item_id":"reasoning-1","delta":"thinking"}
data: {"type":"response.function_call_arguments.delta","call_id":"call-1","delta":"{\"command\""}
data: {"type":"response.completed"}
"#,
    )
    .expect("stream events");

    assert!(events.iter().any(|event| matches!(
        event,
        StreamEvent::ItemDelta {
            delta: ResponseItemDelta::ReasoningContent { item_id: Some(id), delta },
            ..
        } if id == "reasoning-1" && delta == "thinking"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        StreamEvent::ItemDelta {
            delta: ResponseItemDelta::ToolCallArguments { call_id: Some(id), delta },
            ..
        } if id == "call-1" && delta == "{\"command\""
    )));
}

#[test]
fn openai_response_json_parses_shell_tool_call() {
    let response = parse_openai_response_json(
        r#"{
          "choices": [
            {
              "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [
                  {
                    "id": "call_1",
                    "type": "function",
                    "function": {
                      "name": "shell",
                      "arguments": "{\"command\":\"echo yunxi\"}"
                    }
                  }
                ]
              }
            }
          ]
        }"#,
    )
    .expect("provider response");

    assert_eq!(response.message, None);
    assert_eq!(
        response.tool_calls,
        vec![ProviderToolCall::Shell {
            id: Some("call_1".to_string()),
            command: "echo yunxi".to_string()
        }]
    );
}

#[test]
fn openai_response_json_parses_patch_tool_call() {
    let response = parse_openai_response_json(
        r#"{
          "choices": [
            {
              "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [
                  {
                    "id": "call_patch",
                    "type": "function",
                    "function": {
                      "name": "patch",
                      "arguments": "{\"op\":\"write\",\"path\":\"notes.txt\",\"content\":\"hello\"}"
                    }
                  }
                ]
              }
            }
          ]
        }"#,
    )
    .expect("provider response");

    assert_eq!(response.message, None);
    assert_eq!(
        response.tool_calls,
        vec![ProviderToolCall::Patch {
            id: Some("call_patch".to_string()),
            patch: r#"{"op":"write","path":"notes.txt","content":"hello"}"#.to_string()
        }]
    );
}

#[test]
fn openai_response_json_parses_mcp_and_skill_tool_calls() {
    let response = parse_openai_response_json(
        r#"{
          "choices": [
            {
              "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [
                  {
                    "id": "call_mcp",
                    "type": "function",
                    "function": {
                      "name": "mcp",
                      "arguments": "{\"server\":\"fs\",\"tool\":\"read\",\"arguments_json\":\"{\\\"path\\\":\\\"README.md\\\"}\"}"
                    }
                  },
                  {
                    "id": "call_skill",
                    "type": "function",
                    "function": {
                      "name": "skill",
                      "arguments": "{\"name\":\"code-review\",\"arguments_json\":\"{\\\"scope\\\":\\\"runtime\\\"}\"}"
                    }
                  }
                ]
              }
            }
          ]
        }"#,
    )
    .expect("provider response");

    assert_eq!(response.message, None);
    assert_eq!(
        response.tool_calls,
        vec![
            ProviderToolCall::Mcp {
                id: Some("call_mcp".to_string()),
                server: "fs".to_string(),
                tool: "read".to_string(),
                arguments_json: Some(r#"{"path":"README.md"}"#.to_string())
            },
            ProviderToolCall::Skill {
                id: Some("call_skill".to_string()),
                name: "code-review".to_string(),
                arguments_json: Some(r#"{"scope":"runtime"}"#.to_string())
            }
        ]
    );
}

#[test]
fn provider_tool_call_converts_to_yunxi_protocol_tool_call() {
    let call = ProviderToolCall::Shell {
        id: Some("call-1".to_string()),
        command: "echo yunxi".to_string(),
    };

    assert_eq!(
        ToolCall::from(call),
        ToolCall::Shell {
            id: Some("call-1".to_string()),
            command: "echo yunxi".to_string()
        }
    );
}

#[test]
fn openai_response_parses_dynamic_tool_calls() {
    let response = parse_openai_response_json(
        r#"{
          "choices": [
            {
              "message": {
                "role": "assistant",
                "tool_calls": [
                  {
                    "id": "call_search",
                    "type": "function",
                    "function": {
                      "name": "tool_search",
                      "arguments": "{\"query\":\"apply_patch\"}"
                    }
                  },
                  {
                    "id": "call_input",
                    "type": "function",
                    "function": {
                      "name": "request_user_input",
                      "arguments": "{\"prompt\":\"Proceed?\"}"
                    }
                  },
                  {
                    "id": "call_image",
                    "type": "function",
                    "function": {
                      "name": "view_image",
                      "arguments": "{\"path\":\"diagram.png\"}"
                    }
                  },
                  {
                    "id": "call_skill_dynamic",
                    "type": "function",
                    "function": {
                      "name": "skill__writer",
                      "arguments": "{\"arguments_json\":\"{\\\"topic\\\":\\\"report\\\"}\"}"
                    }
                  }
                ]
              }
            }
          ]
        }"#,
    )
    .expect("provider response");

    assert_eq!(
        response.tool_calls,
        vec![
            ProviderToolCall::ToolSearch {
                id: Some("call_search".to_string()),
                query: "apply_patch".to_string()
            },
            ProviderToolCall::RequestUserInput {
                id: Some("call_input".to_string()),
                prompt: "Proceed?".to_string()
            },
            ProviderToolCall::ViewImage {
                id: Some("call_image".to_string()),
                path: "diagram.png".to_string()
            },
            ProviderToolCall::Skill {
                id: Some("call_skill_dynamic".to_string()),
                name: "writer".to_string(),
                arguments_json: Some(r#"{"topic":"report"}"#.to_string())
            }
        ]
    );
}

#[tokio::test]
async fn openai_compatible_provider_can_use_fixture_response() {
    let provider = OpenAiCompatibleProvider::new(
        ProviderConfig::openai_compatible("fixture-model"),
        ProviderAuth::None,
    )
    .with_fixture_response(
        r#"{
          "choices": [
            { "message": { "role": "assistant", "content": "fixture answer" } }
          ]
        }"#,
    );
    let request = ProviderRequest::new(
        AgentConfig::new(PathBuf::from(".")),
        AgentInput::text("fixture"),
    );

    let response = provider.complete(request).await.expect("fixture response");

    assert_eq!(
        response.message.as_ref().map(|message| message.role),
        Some(ProviderRole::Assistant)
    );
    assert_eq!(
        response
            .message
            .as_ref()
            .map(|message| message.content.as_str()),
        Some("fixture answer")
    );
}

#[test]
fn incremental_sse_decoder_feeds_provider_stream_accumulator() {
    let mut decoder = ProviderSseDecoder::default();
    let mut accumulator = OpenAiStreamAccumulator::new("thread-incremental", "turn-incremental");

    let first = accumulator
        .push_raw_chunk(
            &mut decoder,
            "data: {\"choices\":[{\"delta\":{\"content\":\"hel\"}}]}\n",
        )
        .expect("first chunk");
    let second = accumulator
        .push_raw_chunk(
            &mut decoder,
            "data: {\"choices\":[{\"delta\":{\"content\":\"lo\"},\"finish_reason\":\"stop\"}]}\n",
        )
        .expect("second chunk");
    let events = accumulator.finish(&mut decoder).expect("finished stream");

    assert!(first.iter().any(|event| matches!(
        event,
        StreamEvent::ItemDelta {
            delta: ResponseItemDelta::MessageContent { delta, .. },
            ..
        } if delta == "hel"
    )));
    assert!(second.iter().any(|event| matches!(
        event,
        StreamEvent::ItemDelta {
            delta: ResponseItemDelta::MessageContent { delta, .. },
            ..
        } if delta == "lo"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        StreamEvent::ResponseCompleted {
            status: ResponseStatus::Completed,
            ..
        }
    )));
}

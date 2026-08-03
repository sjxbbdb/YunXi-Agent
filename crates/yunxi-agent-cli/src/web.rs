use anyhow::{Context, Result};
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::time::Instant;
use tokio::net::TcpListener;
use yunxi_agent_core::{AgentConfig, AgentEvent, AgentRunStatus, BackendKind};
use yunxi_agent_persona::{PersonaProfile, PersonaProfileStore, PersonaSettings};
use yunxi_agent_storage::{FilePersonaMemoryStore, PersonaMemoryScope};

use crate::provider_mode;

pub(crate) const DEFAULT_WEB_PORT: u16 = 17861;

const INDEX_HTML: &str = include_str!("web/index.html");
const APP_CSS: &str = include_str!("web/app.css");
const APP_JS: &str = include_str!("web/app.js");

#[derive(Clone, Debug)]
pub(crate) struct WebOptions {
    pub bind: IpAddr,
    pub port: u16,
    pub config: AgentConfig,
    pub backend: BackendKind,
    pub provider_mode: provider_mode::ProviderMode,
}

#[derive(Clone)]
struct AppState {
    config: AgentConfig,
    backend: BackendKind,
    provider_mode: provider_mode::ProviderMode,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusResponse {
    status: &'static str,
    version: &'static str,
    cwd: String,
    backend: String,
    provider: String,
    model: String,
    provider_live: bool,
    provider_source: String,
    approval_mode: String,
    sandbox_mode: String,
    memory_extraction: String,
    companion_enabled: bool,
    companion_tool_requests: bool,
    capabilities: Vec<CapabilityStatus>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PersonaResponse {
    enabled: bool,
    active_profile: String,
    profile: PersonaProfile,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryResponse {
    workspace_fingerprint: String,
    records: Vec<yunxi_agent_persona::MemoryRecord>,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CapabilityStatus {
    id: &'static str,
    label: &'static str,
    state: String,
    detail: String,
    action: &'static str,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatRequest {
    prompt: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChatResponse {
    status: AgentRunStatus,
    final_response: String,
    events_count: usize,
    elapsed_ms: u128,
    provider: String,
    model: String,
    provider_live: bool,
    insights: RunInsights,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct RunInsights {
    commands: usize,
    tool_calls: usize,
    approvals: usize,
    escalations: usize,
    sandbox_attempts: usize,
    mcp_tools: usize,
    memory_recalls: usize,
    memory_writes: usize,
    persona_events: usize,
    files_changed: usize,
    warnings: usize,
    errors: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiErrorResponse {
    error: String,
}

pub(crate) async fn run(options: WebOptions) -> Result<()> {
    let addr = SocketAddr::new(options.bind, options.port);
    let state = AppState {
        config: options.config,
        backend: options.backend,
        provider_mode: options.provider_mode,
    };
    let app = app(state);

    eprintln!("YunXi Web listening on http://{addr}");
    eprintln!("Press Ctrl+C to stop the local web console.");

    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind YunXi Web console at {addr}"))?;
    axum::serve(listener, app)
        .await
        .context("YunXi Web console server failed")?;
    Ok(())
}

fn app(state: AppState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/assets/app.css", get(app_css))
        .route("/assets/app.js", get(app_js))
        .route("/api/health", get(health))
        .route("/api/status", get(status))
        .route("/api/persona", get(persona))
        .route("/api/memory", get(memory))
        .route("/api/chat", post(chat))
        .with_state(state)
}

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn app_css() -> Response {
    static_response(APP_CSS, "text/css; charset=utf-8")
}

async fn app_js() -> Response {
    static_response(APP_JS, "application/javascript; charset=utf-8")
}

fn static_response(body: &'static str, content_type: &'static str) -> Response {
    ([(header::CONTENT_TYPE, content_type)], body).into_response()
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn status(
    State(state): State<AppState>,
) -> std::result::Result<Json<StatusResponse>, (StatusCode, Json<ApiErrorResponse>)> {
    let invocation =
        crate::prepare_runtime_invocation(state.config.clone(), state.backend, state.provider_mode)
            .map_err(internal_error)?;
    Ok(Json(status_response(
        &invocation.config,
        state.backend,
        &invocation.selection,
    )))
}

async fn persona()
-> std::result::Result<Json<PersonaResponse>, (StatusCode, Json<ApiErrorResponse>)> {
    let settings = PersonaSettings::load();
    let profile = PersonaProfileStore::load_active_checked(&settings)
        .map_err(|error| internal_error(anyhow::Error::msg(error.to_string())))?;
    Ok(Json(PersonaResponse {
        enabled: settings.persona_enabled,
        active_profile: settings.active_profile,
        profile,
    }))
}

async fn memory(
    State(state): State<AppState>,
) -> std::result::Result<Json<MemoryResponse>, (StatusCode, Json<ApiErrorResponse>)> {
    let store = FilePersonaMemoryStore::for_workspace(&state.config.cwd);
    let load = store.list(PersonaMemoryScope::All);
    Ok(Json(MemoryResponse {
        workspace_fingerprint: store.workspace_fingerprint().to_string(),
        records: load.records,
        warnings: load.warnings,
    }))
}

async fn chat(
    State(state): State<AppState>,
    Json(request): Json<ChatRequest>,
) -> std::result::Result<Json<ChatResponse>, (StatusCode, Json<ApiErrorResponse>)> {
    let prompt = normalize_prompt(request.prompt).map_err(bad_request)?;
    let invocation =
        crate::prepare_runtime_invocation(state.config.clone(), state.backend, state.provider_mode)
            .map_err(internal_error)?;
    let provider = invocation.selection.provider.clone();
    let model = invocation.selection.model.clone();
    let provider_live = invocation.selection.live;
    let started = Instant::now();
    let result = crate::run_agent_backend(state.backend, invocation.config, prompt, provider_live)
        .await
        .map_err(internal_error)?;
    let insights = summarize_events(&result.events);

    Ok(Json(ChatResponse {
        status: result.status,
        final_response: result.final_response.unwrap_or_default(),
        events_count: result.events.len(),
        elapsed_ms: started.elapsed().as_millis(),
        provider,
        model,
        provider_live,
        insights,
    }))
}

fn normalize_prompt(prompt: String) -> std::result::Result<String, &'static str> {
    let prompt = prompt.trim();
    if prompt.is_empty() {
        Err("prompt is required")
    } else {
        Ok(prompt.to_string())
    }
}

fn status_response(
    config: &AgentConfig,
    backend: BackendKind,
    selection: &provider_mode::ProviderSelection,
) -> StatusResponse {
    StatusResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        cwd: config.cwd.display().to_string(),
        backend: format!("{backend:?}").to_ascii_lowercase(),
        provider: selection.provider.clone(),
        model: selection.model.clone(),
        provider_live: selection.live,
        provider_source: selection.source.as_str().to_string(),
        approval_mode: format!("{:?}", config.approval_mode),
        sandbox_mode: format!("{:?}", config.sandbox_mode),
        memory_extraction: format!("{:?}", config.memory_extraction_mode),
        companion_enabled: config.companion.enabled,
        companion_tool_requests: config.companion.allow_tool_requests,
        capabilities: capability_statuses(config, selection),
    }
}

fn capability_statuses(
    config: &AgentConfig,
    selection: &provider_mode::ProviderSelection,
) -> Vec<CapabilityStatus> {
    vec![
        CapabilityStatus {
            id: "chat",
            label: "Agent Chat",
            state: if selection.live { "live" } else { "offline" }.to_string(),
            detail: format!("{}/{}", selection.provider, selection.model),
            action: "send",
        },
        CapabilityStatus {
            id: "tools",
            label: "Tools",
            state: format!("{:?}", config.approval_mode),
            detail: format!("{:?}", config.sandbox_mode),
            action: "inspect",
        },
        CapabilityStatus {
            id: "memory",
            label: "Memory",
            state: format!("{:?}", config.memory_extraction_mode),
            detail: "runtime event summary is returned after every chat run".to_string(),
            action: "test-memory",
        },
        CapabilityStatus {
            id: "persona",
            label: "Persona",
            state: if config.companion.enabled {
                "enabled"
            } else {
                "disabled"
            }
            .to_string(),
            detail: if config.companion.allow_tool_requests {
                "companion tool requests enabled"
            } else {
                "companion tool requests disabled"
            }
            .to_string(),
            action: "test-persona",
        },
        CapabilityStatus {
            id: "weixin",
            label: "Weixin",
            state: "shared-runtime".to_string(),
            detail: "web console does not replace the local Weixin gateway".to_string(),
            action: "inspect",
        },
    ]
}

fn summarize_events(events: &[AgentEvent]) -> RunInsights {
    let mut insights = RunInsights::default();
    for event in events {
        match event {
            AgentEvent::CommandStarted { .. }
            | AgentEvent::CommandUpdated { .. }
            | AgentEvent::CommandCompleted { .. }
            | AgentEvent::CommandFinished { .. } => insights.commands += 1,
            AgentEvent::ToolCallStarted { .. } | AgentEvent::ToolCallCompleted { .. } => {
                insights.tool_calls += 1
            }
            AgentEvent::ApprovalRequested { .. } | AgentEvent::ApprovalCompleted { .. } => {
                insights.approvals += 1
            }
            AgentEvent::EscalationRequested { .. } | AgentEvent::EscalationCompleted { .. } => {
                insights.escalations += 1
            }
            AgentEvent::SandboxAttempt { .. } => insights.sandbox_attempts += 1,
            AgentEvent::McpToolStarted { .. } | AgentEvent::McpToolCompleted { .. } => {
                insights.mcp_tools += 1
            }
            AgentEvent::MemoryRecall { .. } => insights.memory_recalls += 1,
            AgentEvent::MemoryWrite { .. } | AgentEvent::MemoryCandidate { .. } => {
                insights.memory_writes += 1
            }
            AgentEvent::PersonaLoaded { .. } | AgentEvent::PersonaContextInjected { .. } => {
                insights.persona_events += 1
            }
            AgentEvent::FileChanged { .. } | AgentEvent::PatchCompleted { .. } => {
                insights.files_changed += 1
            }
            AgentEvent::Warning { .. } | AgentEvent::MemoryWarning { .. } => insights.warnings += 1,
            AgentEvent::Error { .. } | AgentEvent::ProviderError { .. } => insights.errors += 1,
            _ => {}
        }
    }
    insights
}

fn bad_request(message: &'static str) -> (StatusCode, Json<ApiErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiErrorResponse {
            error: message.to_string(),
        }),
    )
}

fn internal_error(error: anyhow::Error) -> (StatusCode, Json<ApiErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiErrorResponse {
            error: crate::redact_secret_fragments(&format!("{error:#}")),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    use std::path::PathBuf;

    #[test]
    fn prompt_normalization_rejects_blank_input() {
        assert_eq!(normalize_prompt("  hello  ".to_string()).unwrap(), "hello");
        assert!(normalize_prompt("  \n\t  ".to_string()).is_err());
    }

    #[test]
    fn static_assets_are_wired_to_api_routes() {
        assert!(INDEX_HTML.contains("/assets/app.css"));
        assert!(INDEX_HTML.contains("/assets/app.js"));
        assert!(APP_JS.contains("/api/status"));
        assert!(APP_JS.contains("/api/persona"));
        assert!(APP_JS.contains("/api/memory"));
        assert!(APP_JS.contains("/api/chat"));
    }

    #[test]
    fn status_response_uses_runtime_selection() {
        let config = AgentConfig::new(PathBuf::from("D:\\workspace"));
        let selection = provider_mode::ProviderSelection {
            live: false,
            source: provider_mode::ProviderModeSource::ForcedOffline,
            provider: "offline".to_string(),
            model: "static".to_string(),
        };

        let response = status_response(&config, BackendKind::Yunxi, &selection);

        assert_eq!(response.status, "ok");
        assert_eq!(response.backend, "yunxi");
        assert_eq!(response.provider, "offline");
        assert!(!response.provider_live);
        assert!(response.capabilities.iter().any(|item| item.id == "chat"));
    }

    #[test]
    fn run_insights_count_runtime_event_families() {
        let insights = summarize_events(&[
            AgentEvent::MemoryRecall {
                schema_version: 3,
                enabled: true,
                scope: "global".to_string(),
                query: "hello".to_string(),
                count: 1,
                budget_used_chars: 10,
                truncated: false,
                always_on_count: 0,
                dropped_unrelated: 0,
                dropped_by_budget: 0,
                dropped_duplicates: 0,
            },
            AgentEvent::ApprovalRequested {
                id: Some("approval-1".to_string()),
                tool_name: "shell".to_string(),
                reason: "test".to_string(),
            },
            AgentEvent::CommandCompleted {
                id: Some("cmd-1".to_string()),
                command: "echo ok".to_string(),
                aggregated_output: "ok".to_string(),
                exit_code: Some(0),
                status: yunxi_agent_core::CommandStatus::Completed,
                execution_details: None,
            },
        ]);

        assert_eq!(insights.memory_recalls, 1);
        assert_eq!(insights.approvals, 1);
        assert_eq!(insights.commands, 1);
    }

    #[test]
    fn default_web_options_are_local_first() {
        let options = WebOptions {
            bind: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: DEFAULT_WEB_PORT,
            config: AgentConfig::new(PathBuf::from("D:\\workspace")),
            backend: BackendKind::DryRun,
            provider_mode: provider_mode::ProviderMode::ForcedOffline,
        };

        assert_eq!(options.bind, IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert_eq!(options.port, 17861);
    }
}

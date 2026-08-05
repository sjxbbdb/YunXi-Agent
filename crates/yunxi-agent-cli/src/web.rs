use anyhow::{Context, Result};
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::net::TcpListener;
use yunxi_agent_companion::{
    CompanionMailboxContent, CompanionMailboxItem, MailboxItemType, MailboxQuery, MailboxState,
};
use yunxi_agent_core::{AgentConfig, AgentEvent, AgentRunStatus, BackendKind};
use yunxi_agent_persona::{
    MemoryKind, PersonaProfile, PersonaProfileStore, PersonaRuleLevel, PersonaSettings,
    RelationshipFamiliarity,
};
use yunxi_agent_storage::{FileCompanionMailboxStore, FilePersonaMemoryStore, PersonaMemoryScope};

use crate::provider_mode;

pub(crate) const DEFAULT_WEB_PORT: u16 = 17861;

const INDEX_HTML: &str = include_str!("web/index.html");
const APP_CSS: &str = include_str!("web/app.css");
const APP_JS: &str = include_str!("web/app.js");
const YUNXI_CHARACTER_ART: &[u8] = include_bytes!("web/yunxi-character-design.jpg");
const YUNXI_HER_BACKGROUND: &[u8] = include_bytes!("web/yunxi-her-background.jpg");
const YUNXI_HER_PROFILE_CARD: &[u8] = include_bytes!("web/yunxi-her-profile-card.jpg");
const YUNXI_HER_EXPRESSION_RESERVED: &[u8] =
    include_bytes!("web/yunxi-her-expression-reserved.jpg");
const YUNXI_HER_EXPRESSION_CALM: &[u8] = include_bytes!("web/yunxi-her-expression-calm.jpg");
const YUNXI_HER_EXPRESSION_THOUGHTFUL: &[u8] =
    include_bytes!("web/yunxi-her-expression-thoughtful.jpg");
const YUNXI_HER_EXPRESSION_WISTFUL: &[u8] = include_bytes!("web/yunxi-her-expression-wistful.jpg");
const YUNXI_HER_EXPRESSION_SMILE: &[u8] = include_bytes!("web/yunxi-her-expression-smile.jpg");
const YUNXI_HER_EXPRESSION_GENTLE: &[u8] = include_bytes!("web/yunxi-her-expression-gentle.jpg");

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
struct HerResponse {
    display_name: String,
    profile_id: String,
    profile_version: String,
    identity: String,
    soul_signature: String,
    voice: String,
    companion_style: String,
    addressing: String,
    relationship_stage: &'static str,
    relationship_label: &'static str,
    relationship_description: &'static str,
    active_memory_count: usize,
    meaningful_memory_count: usize,
    last_meaningful_check_in_millis: Option<u128>,
    persona_enabled: bool,
    memory_enabled: bool,
    companion_enabled: bool,
    traits: Vec<HerTraitResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HerTraitResponse {
    id: &'static str,
    label: &'static str,
    level: &'static str,
    score: u8,
}

#[derive(Debug)]
struct HerDisplayFields {
    identity: String,
    soul_signature: String,
    voice: String,
    companion_style: String,
    addressing: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MailboxResponse {
    items: Vec<MailboxItemResponse>,
    unread_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MailboxItemResponse {
    item_id: String,
    item_type: MailboxItemType,
    state: MailboxState,
    subject: String,
    preview: String,
    created_at_millis: u128,
    available_at_millis: u128,
    updated_at_millis: u128,
    read_at_millis: Option<u128>,
    archived_at_millis: Option<u128>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MailboxDetailResponse {
    item: MailboxItemResponse,
    subject: String,
    body: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MailboxStateRequest {
    state: MailboxState,
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
        .route("/assets/yunxi-character-design.jpg", get(character_art))
        .route("/assets/yunxi-her-background.jpg", get(her_background_art))
        .route("/assets/yunxi-her-profile-card.jpg", get(her_profile_art))
        .route(
            "/assets/yunxi-her-expression-reserved.jpg",
            get(her_expression_reserved_art),
        )
        .route(
            "/assets/yunxi-her-expression-calm.jpg",
            get(her_expression_calm_art),
        )
        .route(
            "/assets/yunxi-her-expression-thoughtful.jpg",
            get(her_expression_thoughtful_art),
        )
        .route(
            "/assets/yunxi-her-expression-wistful.jpg",
            get(her_expression_wistful_art),
        )
        .route(
            "/assets/yunxi-her-expression-smile.jpg",
            get(her_expression_smile_art),
        )
        .route(
            "/assets/yunxi-her-expression-gentle.jpg",
            get(her_expression_gentle_art),
        )
        .route("/api/health", get(health))
        .route("/api/status", get(status))
        .route("/api/persona", get(persona))
        .route("/api/her", get(her))
        .route("/api/memory", get(memory))
        .route("/api/mailbox", get(mailbox))
        .route("/api/mailbox/{item_id}", get(mailbox_detail))
        .route("/api/mailbox/{item_id}/state", post(mailbox_state))
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

async fn character_art() -> Response {
    image_response(YUNXI_CHARACTER_ART)
}

async fn her_background_art() -> Response {
    image_response(YUNXI_HER_BACKGROUND)
}

async fn her_profile_art() -> Response {
    image_response(YUNXI_HER_PROFILE_CARD)
}

async fn her_expression_reserved_art() -> Response {
    image_response(YUNXI_HER_EXPRESSION_RESERVED)
}

async fn her_expression_calm_art() -> Response {
    image_response(YUNXI_HER_EXPRESSION_CALM)
}

async fn her_expression_thoughtful_art() -> Response {
    image_response(YUNXI_HER_EXPRESSION_THOUGHTFUL)
}

async fn her_expression_wistful_art() -> Response {
    image_response(YUNXI_HER_EXPRESSION_WISTFUL)
}

async fn her_expression_smile_art() -> Response {
    image_response(YUNXI_HER_EXPRESSION_SMILE)
}

async fn her_expression_gentle_art() -> Response {
    image_response(YUNXI_HER_EXPRESSION_GENTLE)
}

fn image_response(image: &'static [u8]) -> Response {
    Response::builder()
        .header(header::CONTENT_TYPE, "image/jpeg")
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(Body::from(image))
        .expect("static character art response should be valid")
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

async fn her(
    State(state): State<AppState>,
) -> std::result::Result<Json<HerResponse>, (StatusCode, Json<ApiErrorResponse>)> {
    let settings = PersonaSettings::load();
    let profile = PersonaProfileStore::load_active_checked(&settings)
        .map_err(|error| internal_error(anyhow::Error::msg(error.to_string())))?;
    let store = FilePersonaMemoryStore::for_workspace(&state.config.cwd);
    let load = store.list(PersonaMemoryScope::All);
    let now = now_millis();
    let active_memory_count = load
        .records
        .iter()
        .filter(|record| record.is_recallable_at(now))
        .count();
    let meaningful_memory_count = load
        .records
        .iter()
        .filter(|record| record.is_recallable_at(now))
        .filter(|record| {
            matches!(
                record.kind,
                MemoryKind::Preference
                    | MemoryKind::PersonalFact
                    | MemoryKind::RelationshipNote
                    | MemoryKind::EmotionalState
                    | MemoryKind::Goal
                    | MemoryKind::Event
            )
        })
        .count();
    let relationship = yunxi_agent_runtime::derive_relationship_state(&load.records);
    let (relationship_stage, relationship_label, relationship_description) =
        relationship_display(relationship.familiarity);
    let display = her_display_fields(&profile);
    let traits = vec![
        her_trait("warmth", "温度", profile.companion_rules.warmth),
        her_trait("directness", "直接", profile.companion_rules.directness),
        her_trait("initiative", "主动", profile.companion_rules.initiative),
        her_trait("humor", "幽默", profile.companion_rules.humor),
        her_trait(
            "emotional_attunement",
            "共情",
            profile.companion_rules.emotional_attunement,
        ),
    ];

    Ok(Json(HerResponse {
        display_name: profile.display_name,
        profile_id: profile.id,
        profile_version: profile.version,
        identity: display.identity,
        soul_signature: display.soul_signature,
        voice: display.voice,
        companion_style: display.companion_style,
        addressing: display.addressing,
        relationship_stage,
        relationship_label,
        relationship_description,
        active_memory_count,
        meaningful_memory_count,
        last_meaningful_check_in_millis: relationship.last_meaningful_check_in_millis,
        persona_enabled: settings.persona_enabled,
        memory_enabled: settings.memory_enabled,
        companion_enabled: settings.companion_enabled,
        traits,
    }))
}

fn her_display_fields(profile: &PersonaProfile) -> HerDisplayFields {
    if profile.authoritative_soul {
        return HerDisplayFields {
            identity: "本地陪伴型 Agent".to_string(),
            soul_signature: "权威灵魂已载入".to_string(),
            voice: "随灵魂原文保持一致".to_string(),
            companion_style: "随关系与记忆自然生长".to_string(),
            addressing: "沿用已经确认的称呼".to_string(),
        };
    }
    let soul_signature = profile
        .companion_rules
        .soul_signature
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(&profile.layers.soul);
    HerDisplayFields {
        identity: display_excerpt(&profile.layers.identity, 180),
        soul_signature: display_excerpt(soul_signature, 180),
        voice: display_excerpt(&profile.layers.voice, 180),
        companion_style: display_excerpt(&profile.layers.companion_style, 180),
        addressing: display_excerpt(&profile.layers.addressing, 140),
    }
}

fn relationship_display(
    familiarity: RelationshipFamiliarity,
) -> (&'static str, &'static str, &'static str) {
    match familiarity {
        RelationshipFamiliarity::New => (
            "new",
            "初识",
            "她正在认真认识你，也会谨慎地区分已经确认的事实和猜测。",
        ),
        RelationshipFamiliarity::Familiar => (
            "familiar",
            "熟悉",
            "她已经熟悉一些稳定偏好与共同经历，会更自然地承接你们的上下文。",
        ),
        RelationshipFamiliarity::Established => (
            "established",
            "相知",
            "你们之间已经形成稳定的关系脉络，而真实、尊重和边界仍会被认真保留。",
        ),
    }
}

fn her_trait(id: &'static str, label: &'static str, level: PersonaRuleLevel) -> HerTraitResponse {
    HerTraitResponse {
        id,
        label,
        level: match level {
            PersonaRuleLevel::Low => "低",
            PersonaRuleLevel::Balanced => "平衡",
            PersonaRuleLevel::High => "高",
        },
        score: level.score(),
    }
}

fn display_excerpt(value: &str, max_chars: usize) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut excerpt = compact.chars().take(max_chars).collect::<String>();
    if compact.chars().count() > max_chars {
        excerpt.push('…');
    }
    excerpt
}

async fn mailbox(
    State(state): State<AppState>,
) -> std::result::Result<Json<MailboxResponse>, (StatusCode, Json<ApiErrorResponse>)> {
    let store = FileCompanionMailboxStore::for_workspace(&state.config.cwd)
        .map_err(|error| internal_error(anyhow::Error::msg(error.to_string())))?;
    let page = store
        .list(MailboxQuery {
            item_type: Some(MailboxItemType::LoveLetter),
            limit: 50,
            ..MailboxQuery::default()
        })
        .map_err(|error| internal_error(anyhow::Error::msg(error.to_string())))?;
    let mut items = Vec::with_capacity(page.items.len());
    for item in page.items {
        let entry = store
            .get(&item.item_id)
            .map_err(|error| internal_error(anyhow::Error::msg(error.to_string())))?
            .ok_or_else(|| internal_error(anyhow::Error::msg("mailbox item disappeared")))?;
        items.push(mailbox_item_response(&entry.item, &entry.content));
    }
    let unread_count = store
        .unread_count()
        .map_err(|error| internal_error(anyhow::Error::msg(error.to_string())))?;
    Ok(Json(MailboxResponse {
        items,
        unread_count,
    }))
}

async fn mailbox_detail(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
) -> std::result::Result<Json<MailboxDetailResponse>, (StatusCode, Json<ApiErrorResponse>)> {
    let store = FileCompanionMailboxStore::for_workspace(&state.config.cwd)
        .map_err(|error| internal_error(anyhow::Error::msg(error.to_string())))?;
    let entry = store
        .get(&item_id)
        .map_err(|error| internal_error(anyhow::Error::msg(error.to_string())))?
        .ok_or_else(|| not_found("mailbox item not found"))?;
    let item = mailbox_item_response(&entry.item, &entry.content);
    Ok(Json(MailboxDetailResponse {
        item,
        subject: entry.content.subject,
        body: entry.content.body,
    }))
}

async fn mailbox_state(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
    Json(request): Json<MailboxStateRequest>,
) -> std::result::Result<Json<MailboxDetailResponse>, (StatusCode, Json<ApiErrorResponse>)> {
    let store = FileCompanionMailboxStore::for_workspace(&state.config.cwd)
        .map_err(|error| internal_error(anyhow::Error::msg(error.to_string())))?;
    let item = store
        .mark_state(&item_id, request.state, now_millis())
        .map_err(|error| internal_error(anyhow::Error::msg(error.to_string())))?
        .ok_or_else(|| not_found("mailbox item not found"))?;
    let entry = store
        .get(&item.item_id)
        .map_err(|error| internal_error(anyhow::Error::msg(error.to_string())))?
        .ok_or_else(|| not_found("mailbox item disappeared"))?;
    Ok(Json(MailboxDetailResponse {
        item: mailbox_item_response(&item, &entry.content),
        subject: entry.content.subject,
        body: entry.content.body,
    }))
}

fn mailbox_item_response(
    item: &CompanionMailboxItem,
    content: &CompanionMailboxContent,
) -> MailboxItemResponse {
    MailboxItemResponse {
        item_id: item.item_id.clone(),
        item_type: item.item_type,
        state: item.state,
        subject: content.subject.clone(),
        preview: mailbox_preview(&content.body),
        created_at_millis: item.created_at_millis,
        available_at_millis: item.available_at_millis,
        updated_at_millis: item.updated_at_millis,
        read_at_millis: item.read_at_millis,
        archived_at_millis: item.archived_at_millis,
    }
}

fn mailbox_preview(body: &str) -> String {
    let compact = body.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut preview = compact.chars().take(132).collect::<String>();
    if compact.chars().count() > 132 {
        preview.push('…');
    }
    preview
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
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

fn not_found(message: &'static str) -> (StatusCode, Json<ApiErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
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
        assert!(APP_JS.contains("/api/her"));
        assert!(APP_JS.contains("/api/memory"));
        assert!(APP_JS.contains("/api/mailbox"));
        assert!(APP_JS.contains("/api/chat"));
        assert!(INDEX_HTML.contains("data-nav=\"mailbox\""));
        assert!(INDEX_HTML.contains("data-nav=\"her\""));
        assert!(APP_CSS.contains(".mailbox-canvas"));
        assert!(APP_CSS.contains(".her-canvas"));
        assert!(APP_CSS.contains(".her-backdrop-reveal"));
        assert!(APP_CSS.contains(".her-backdrop-flow"));
        assert!(APP_CSS.contains(".her-flow-ribbon"));
        assert!(INDEX_HTML.contains("/assets/yunxi-her-background.jpg"));
        assert!(INDEX_HTML.contains("/assets/yunxi-her-profile-card.jpg"));
        assert!(INDEX_HTML.contains("id=\"her-art-plane\""));
        assert!(INDEX_HTML.contains("id=\"her-detail\""));
        assert!(INDEX_HTML.contains("id=\"her-card-surface\""));
        assert!(INDEX_HTML.contains("id=\"her-backdrop-flow\""));
        assert!(INDEX_HTML.contains("id=\"her-flow-streak\""));
        assert!(INDEX_HTML.contains("class=\"her-card-depth"));
        assert_eq!(INDEX_HTML.matches("data-her-art-mode=").count(), 2);
        assert!(!INDEX_HTML.contains("data-her-art-mode=\"sheet\""));
        assert!(APP_JS.contains("setupHerReveal"));
        assert!(!INDEX_HTML.contains("id=\"her-detail-close\""));
        assert!(!INDEX_HTML.contains("her-card-open"));
        assert!(!INDEX_HTML.contains("aria-controls=\"her-detail\""));
        assert!(!APP_JS.contains("openHerDetail"));
        assert!(!APP_JS.contains("closeHerDetail"));
        assert!(!APP_JS.contains("is-detail-open"));
        assert!(!APP_CSS.contains(".her-detail-close"));
        assert!(!APP_CSS.contains(".her-card-open"));
        assert!(!APP_CSS.contains("is-detail-open"));
        assert!(APP_JS.contains("updateHerParallax"));
        assert!(APP_JS.contains("herCardDepthLayers"));
        assert!(APP_JS.contains("herFlowPrimary"));
        let backdrop_base_rule = APP_CSS
            .split(".her-backdrop-base {")
            .nth(1)
            .and_then(|rules| rules.split('}').next())
            .expect("her backdrop base styles");
        assert!(backdrop_base_rule.contains("opacity: 0"));
        let backdrop_flow_rule = APP_CSS
            .split(".her-backdrop-flow {")
            .nth(1)
            .and_then(|rules| rules.split('}').next())
            .expect("her backdrop flow styles");
        assert!(backdrop_flow_rule.contains("opacity: 0"));
        assert!(backdrop_flow_rule.contains("mask-image"));
        assert!(APP_CSS.contains(".her-backdrop.is-revealing .her-backdrop-flow"));
        let her_detail_rule = APP_CSS
            .split(".her-detail {")
            .nth(1)
            .and_then(|rules| rules.split('}').next())
            .expect("her detail styles");
        assert!(her_detail_rule.contains("rgba(12, 18, 22, 0.18)"));
        assert!(her_detail_rule.contains("opacity: 1"));
        assert!(her_detail_rule.contains("visibility: visible"));
        assert!(her_detail_rule.contains("pointer-events: auto"));
        let name_rule = APP_CSS
            .split(".her-card-name {")
            .nth(1)
            .and_then(|rules| rules.split('}').next())
            .expect("her card name styles");
        assert!(name_rule.contains("overflow: visible"));
        assert!(!name_rule.contains("text-overflow: ellipsis"));
        assert!(APP_JS.contains("/assets/yunxi-her-expression-smile.jpg"));
        assert!(APP_CSS.contains(".her-card-glass"));
        assert!(!INDEX_HTML.contains("assets.21st.dev"));
        assert_eq!(APP_CSS.matches('{').count(), APP_CSS.matches('}').count());
        assert!(!YUNXI_CHARACTER_ART.is_empty());
        assert!(!YUNXI_HER_BACKGROUND.is_empty());
        assert!(!YUNXI_HER_PROFILE_CARD.is_empty());
        assert!(!YUNXI_HER_EXPRESSION_RESERVED.is_empty());
        assert!(!YUNXI_HER_EXPRESSION_CALM.is_empty());
        assert!(!YUNXI_HER_EXPRESSION_THOUGHTFUL.is_empty());
        assert!(!YUNXI_HER_EXPRESSION_WISTFUL.is_empty());
        assert!(!YUNXI_HER_EXPRESSION_SMILE.is_empty());
        assert!(!YUNXI_HER_EXPRESSION_GENTLE.is_empty());
    }

    #[test]
    fn her_relationship_labels_match_runtime_familiarity() {
        assert_eq!(relationship_display(RelationshipFamiliarity::New).1, "初识");
        assert_eq!(
            relationship_display(RelationshipFamiliarity::Familiar).1,
            "熟悉"
        );
        assert_eq!(
            relationship_display(RelationshipFamiliarity::Established).1,
            "相知"
        );
    }

    #[test]
    fn her_display_excerpt_is_compact_and_unicode_safe() {
        assert_eq!(display_excerpt("  温暖\n清晰  稳定 ", 20), "温暖 清晰 稳定");
        assert_eq!(display_excerpt("一二三四五六", 4), "一二三四…");
    }

    #[test]
    fn her_authoritative_soul_keeps_private_original_out_of_web_fields() {
        let mut profile = yunxi_agent_persona::yunxi_companion_strong();
        profile.authoritative_soul = true;
        profile.layers.soul = "PRIVATE-SOUL-MARKER".to_string();

        let display = her_display_fields(&profile);
        let serialized = format!(
            "{}{}{}{}{}",
            display.identity,
            display.soul_signature,
            display.voice,
            display.companion_style,
            display.addressing
        );

        assert!(!serialized.contains("PRIVATE-SOUL-MARKER"));
        assert_eq!(display.soul_signature, "权威灵魂已载入");
    }

    #[test]
    fn mailbox_response_excludes_private_storage_references() {
        let item = CompanionMailboxItem {
            schema_version: 1,
            item_id: "mail-1".to_string(),
            owner_scope: "workspace:private".to_string(),
            item_type: MailboxItemType::LoveLetter,
            source_id: "task-1".to_string(),
            source_revision: "memory-1".to_string(),
            content_ref: "content-private".to_string(),
            state: MailboxState::Unread,
            created_at_millis: 1,
            available_at_millis: 1,
            updated_at_millis: 1,
            read_at_millis: None,
            archived_at_millis: None,
        };
        let response = mailbox_item_response(
            &item,
            &CompanionMailboxContent {
                subject: "一封信".to_string(),
                body: "给你的一段话".to_string(),
            },
        );
        let json = serde_json::to_value(response).expect("mailbox response should serialize");
        assert_eq!(json["itemId"], "mail-1");
        assert_eq!(json["subject"], "一封信");
        assert!(json.get("ownerScope").is_none());
        assert!(json.get("contentRef").is_none());
        assert!(json.get("sourceId").is_none());
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

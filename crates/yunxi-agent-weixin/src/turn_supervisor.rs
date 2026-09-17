use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use thiserror::Error;
use tokio::sync::{Mutex as TokioMutex, OwnedMutexGuard};
use yunxi_agent_core::{
    Agent, AgentBackend, AgentConfig, AgentEvent, AgentInput, AgentInputChannel,
    AgentInputModality, AgentMessageStream, AgentMessageStreamPhase, AgentRunApprovalDecision,
    AgentRunApprovalRequest, AgentRunControl, AgentRunStatus, AgentRunStreamReceiver,
};
use yunxi_agent_storage::{
    FileWeixinStateStore, WeixinPendingInboundState, WeixinRuntimeTurnBeginRequest,
    WeixinStateError,
};

use crate::inbound::{WeixinInboundKind, WeixinPendingInboundPayload};
use crate::payload_cipher::{WeixinPayloadCipher, WeixinPayloadCipherError};
use crate::redaction::SecretString;
use crate::remote_control::{
    WeixinRemoteControlHub, WeixinRemoteControlScope, render_remote_control_error,
    render_remote_control_prompt,
};
use crate::voice::{WeixinVoiceError, WeixinVoiceTranscriber};

const DEFAULT_MAX_QUEUE_PER_CONVERSATION: usize = 8;
const DEFAULT_MAX_GLOBAL_QUEUE: usize = 64;
const DEFAULT_SOURCE_LABEL: &str = "weixin-private-chat";
const DEFAULT_SESSION_TITLE: &str = "Weixin private chat";
const DEFAULT_REMOTE_CONTROL_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const WEIXIN_STREAM_OUTPUT_POLICY: &str = "final_text_only";

#[derive(Clone)]
pub struct WeixinTurnSupervisor {
    state_store: FileWeixinStateStore,
    data_key: SecretString,
    options: WeixinTurnSupervisorOptions,
    sink: Arc<dyn WeixinRuntimeSink>,
    queue_limiter: Arc<ConversationQueueLimiter>,
    payload_cipher: WeixinPayloadCipher,
}

impl WeixinTurnSupervisor {
    pub fn new(
        state_store: FileWeixinStateStore,
        data_key: SecretString,
        options: WeixinTurnSupervisorOptions,
        sink: Arc<dyn WeixinRuntimeSink>,
    ) -> Self {
        Self {
            state_store,
            data_key,
            options,
            sink,
            queue_limiter: Arc::new(ConversationQueueLimiter::default()),
            payload_cipher: WeixinPayloadCipher::new(),
        }
    }

    pub fn with_test_sink(
        state_store: FileWeixinStateStore,
        data_key: SecretString,
        options: WeixinTurnSupervisorOptions,
        sink: WeixinRuntimeTestSink,
    ) -> Self {
        Self::new(state_store, data_key, options, Arc::new(sink))
    }

    pub async fn run_pending_turn<B>(
        &self,
        backend: &B,
        account_id: &str,
        item_id: &str,
    ) -> Result<WeixinTurnReport, WeixinTurnSupervisorError>
    where
        B: AgentBackend,
    {
        let pending = self
            .state_store
            .load_pending_inbound(account_id, item_id)?
            .ok_or_else(|| WeixinStateError::PendingInboundNotFound {
                item_id: item_id.to_string(),
            })?;
        if pending.state != WeixinPendingInboundState::Ready {
            return Err(WeixinTurnSupervisorError::InvalidPendingState {
                item_id: item_id.to_string(),
                state: pending.state.as_str(),
            });
        }

        let payload = self
            .payload_cipher
            .decrypt_pending_inbound(&self.data_key, &pending)?;
        match payload.payload_kind {
            WeixinInboundKind::Text
                if payload.text.as_ref().is_some_and(|text| !text.is_empty()) => {}
            WeixinInboundKind::Voice
                if payload.voice.is_some() && self.options.voice_transcriber.is_some() => {}
            _ => return Err(WeixinTurnSupervisorError::UnsupportedPayload),
        }

        let candidate_session_id = self.candidate_session_id(&payload);
        self.state_store.remember_pending_runtime_turn_session(
            &payload.account_id,
            &payload.item_id,
            &candidate_session_id,
            now_millis_u64(),
        )?;

        let queue_key = ConversationQueueKey::from_payload(&payload);
        let _slot = self
            .queue_limiter
            .acquire(
                queue_key,
                self.options.max_queue_per_conversation,
                self.options.max_global_queue,
            )
            .await?;

        let binding =
            self.state_store
                .begin_pending_runtime_turn(WeixinRuntimeTurnBeginRequest {
                    account_id: payload.account_id.clone(),
                    peer_id_hash: payload.peer_id_hash.clone(),
                    message_id_hash: payload.message_id_hash.clone(),
                    item_id: payload.item_id.clone(),
                    direct_message_key: payload.direct_message_key.clone(),
                    workspace_id: self.options.workspace_id.clone(),
                    candidate_session_id,
                    source_label: self.options.source_label.clone(),
                    now_millis: now_millis_u64(),
                })?;

        let runtime_session_id = binding
            .active_session_id
            .clone()
            .unwrap_or_else(|| binding.session_id.clone());
        let agent_input = match payload.payload_kind {
            WeixinInboundKind::Text => AgentInput::with_channel(
                payload
                    .text
                    .as_ref()
                    .ok_or(WeixinTurnSupervisorError::UnsupportedPayload)?
                    .expose()
                    .to_string(),
                AgentInputModality::Text,
                AgentInputChannel::Weixin,
            ),
            WeixinInboundKind::Voice => {
                let voice = payload
                    .voice
                    .as_ref()
                    .ok_or(WeixinTurnSupervisorError::UnsupportedPayload)?;
                let transcriber = self
                    .options
                    .voice_transcriber
                    .as_ref()
                    .ok_or(WeixinTurnSupervisorError::UnsupportedPayload)?;
                match transcriber.transcribe_voice(voice).await {
                    Ok(transcript) => AgentInput::with_channel(
                        transcript.expose().to_string(),
                        AgentInputModality::Voice,
                        AgentInputChannel::Weixin,
                    ),
                    Err(error) => {
                        let _ = self.sink.write_outbound_text(WeixinRuntimeSinkRecord {
                            account_id: payload.account_id.clone(),
                            peer_id_hash: payload.peer_id_hash.clone(),
                            direct_message_key: payload.direct_message_key.clone(),
                            item_id: payload.item_id.clone(),
                            session_id: runtime_session_id.clone(),
                            reply_to_user_id: payload.reply_to_user_id.clone(),
                            reply_context_token: payload.reply_context_token.clone(),
                            final_response: "这条语音暂时没能听清，请再说一次。".to_string(),
                        });
                        return Err(WeixinTurnSupervisorError::Voice(error));
                    }
                }
            }
            _ => return Err(WeixinTurnSupervisorError::UnsupportedPayload),
        };
        let parent_session_id = binding.last_completed_session_id.clone();
        let mut runtime_config = self
            .options
            .config
            .clone()
            .with_session_id(runtime_session_id.clone())
            .with_session_title(DEFAULT_SESSION_TITLE);
        if let Some(parent_session_id) = parent_session_id.clone() {
            runtime_config = runtime_config.with_parent_session_id(parent_session_id);
        }
        let (result, stream_observation) =
            if let Some(remote_control_hub) = self.options.remote_control_hub.clone() {
                let (control, stream) = AgentRunControl::streaming();
                let scope = WeixinRemoteControlScope {
                    account_id: payload.account_id.clone(),
                    peer_id_hash: payload.peer_id_hash.clone(),
                    direct_message_key: payload.direct_message_key.clone(),
                    item_id: payload.item_id.clone(),
                    session_id: runtime_session_id.clone(),
                };
                if let Err(_error) = remote_control_hub.register_cancellation(
                    scope.clone(),
                    control.cancellation_token(),
                    now_millis_u64()
                        .saturating_add(self.options.remote_control_timeout.as_millis() as u64),
                ) {
                    let _ = self.sink.write_outbound_text(WeixinRuntimeSinkRecord {
                        account_id: payload.account_id.clone(),
                        peer_id_hash: payload.peer_id_hash.clone(),
                        direct_message_key: payload.direct_message_key.clone(),
                        item_id: payload.item_id.clone(),
                        session_id: runtime_session_id.clone(),
                        reply_to_user_id: payload.reply_to_user_id.clone(),
                        reply_context_token: payload.reply_context_token.clone(),
                        final_response: "远程控制暂不可用，本轮未启动；请稍后重试。".to_string(),
                    });
                    self.state_store.complete_pending_runtime_turn(
                        &payload.account_id,
                        &payload.item_id,
                        WeixinPendingInboundState::Failed,
                        Some("remote_control_register_failed".to_string()),
                        now_millis_u64(),
                    )?;
                    return Err(WeixinTurnSupervisorError::RemoteControlRegistrationFailed);
                }
                let timeout = self.options.remote_control_timeout;
                let monitor = supervise_remote_control_stream(
                    remote_control_hub.clone(),
                    scope.clone(),
                    stream,
                    timeout,
                    Arc::clone(&self.sink),
                    payload.reply_to_user_id.clone(),
                    payload.reply_context_token.clone(),
                );
                let agent = Agent::new(runtime_config);
                let run = agent.run_with_backend_stream(backend, agent_input.clone(), control);
                let (result, observation) = tokio::join!(run, monitor);
                let _ = remote_control_hub.close_scope(&scope);
                (result, Some(observation))
            } else {
                (
                    Agent::new(runtime_config)
                        .run_with_backend_stream(backend, agent_input, AgentRunControl::detached())
                        .await,
                    None,
                )
            };

        self.state_store.record_pending_runtime_agent_completed(
            &payload.account_id,
            &payload.item_id,
            now_millis_u64(),
        )?;

        let report = match result {
            Ok(result) => {
                let target = match result.status {
                    AgentRunStatus::Completed => WeixinPendingInboundState::Succeeded,
                    AgentRunStatus::Failed => WeixinPendingInboundState::Failed,
                    AgentRunStatus::Cancelled => WeixinPendingInboundState::Cancelled,
                };
                let mut final_response_present = false;
                if target == WeixinPendingInboundState::Succeeded
                    && let Some(final_response) = result.final_response.as_ref()
                    && let Some(public_response) = sanitize_public_final_response(final_response)
                {
                    let record = WeixinRuntimeSinkRecord {
                        account_id: payload.account_id.clone(),
                        peer_id_hash: payload.peer_id_hash.clone(),
                        direct_message_key: payload.direct_message_key.clone(),
                        item_id: payload.item_id.clone(),
                        session_id: runtime_session_id.clone(),
                        reply_to_user_id: payload.reply_to_user_id.clone(),
                        reply_context_token: payload.reply_context_token.clone(),
                        final_response: public_response,
                    };
                    self.sink.write_final_response(record)?;
                    final_response_present = true;
                }
                self.state_store.complete_pending_runtime_turn(
                    &payload.account_id,
                    &payload.item_id,
                    target,
                    terminal_reason_for_status(result.status),
                    now_millis_u64(),
                )?;
                WeixinTurnReport {
                    account_id: payload.account_id,
                    peer_id_hash: payload.peer_id_hash,
                    direct_message_key: payload.direct_message_key,
                    item_id: payload.item_id,
                    session_id: runtime_session_id,
                    parent_session_id,
                    status: result.status,
                    final_response_present,
                    stream_observation,
                }
            }
            Err(_error) => {
                self.state_store.complete_pending_runtime_turn(
                    &payload.account_id,
                    &payload.item_id,
                    WeixinPendingInboundState::Failed,
                    Some("runtime_failed".to_string()),
                    now_millis_u64(),
                )?;
                WeixinTurnReport {
                    account_id: payload.account_id,
                    peer_id_hash: payload.peer_id_hash,
                    direct_message_key: payload.direct_message_key,
                    item_id: payload.item_id,
                    session_id: runtime_session_id,
                    parent_session_id,
                    status: AgentRunStatus::Failed,
                    final_response_present: false,
                    stream_observation,
                }
            }
        };

        Ok(report)
    }

    fn candidate_session_id(&self, payload: &WeixinPendingInboundPayload) -> String {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        payload.account_id.hash(&mut hasher);
        payload.peer_id_hash.hash(&mut hasher);
        payload.direct_message_key.hash(&mut hasher);
        payload.item_id.hash(&mut hasher);
        self.options.workspace_id.hash(&mut hasher);
        format!("yunxi-weixin-turn-{:016x}", hasher.finish())
    }
}

#[derive(Clone)]
pub struct WeixinTurnSupervisorOptions {
    pub config: AgentConfig,
    pub workspace_id: String,
    pub max_queue_per_conversation: usize,
    pub max_global_queue: usize,
    pub source_label: String,
    pub remote_control_hub: Option<WeixinRemoteControlHub>,
    pub remote_control_timeout: Duration,
    pub voice_transcriber: Option<Arc<dyn WeixinVoiceTranscriber>>,
}

impl WeixinTurnSupervisorOptions {
    pub fn new(config: AgentConfig, workspace_id: impl Into<String>) -> Self {
        Self {
            config,
            workspace_id: workspace_id.into(),
            max_queue_per_conversation: DEFAULT_MAX_QUEUE_PER_CONVERSATION,
            max_global_queue: DEFAULT_MAX_GLOBAL_QUEUE,
            source_label: DEFAULT_SOURCE_LABEL.to_string(),
            remote_control_hub: None,
            remote_control_timeout: DEFAULT_REMOTE_CONTROL_TIMEOUT,
            voice_transcriber: None,
        }
    }

    pub fn with_remote_control_hub(mut self, hub: WeixinRemoteControlHub) -> Self {
        self.remote_control_hub = Some(hub);
        self
    }

    pub fn with_remote_control_timeout(mut self, timeout: Duration) -> Self {
        self.remote_control_timeout = timeout.max(Duration::from_millis(1));
        self
    }

    pub fn with_voice_transcriber(mut self, transcriber: Arc<dyn WeixinVoiceTranscriber>) -> Self {
        self.voice_transcriber = Some(transcriber);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeixinTurnReport {
    pub account_id: String,
    pub peer_id_hash: String,
    pub direct_message_key: String,
    pub item_id: String,
    pub session_id: String,
    pub parent_session_id: Option<String>,
    pub status: AgentRunStatus,
    pub final_response_present: bool,
    pub stream_observation: Option<WeixinStreamObservationReport>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeixinStreamObservationReport {
    pub policy: &'static str,
    pub observed_event_count: usize,
    pub public_message_count: usize,
    pub duplicate_public_message_count: usize,
    pub filtered_non_public_count: usize,
    pub unsafe_public_message_count: usize,
    pub final_message_count: usize,
    pub terminal_status: Option<AgentRunStatus>,
    pub merged_public_text: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeixinRuntimeSinkRecord {
    pub account_id: String,
    pub peer_id_hash: String,
    pub direct_message_key: String,
    pub item_id: String,
    pub session_id: String,
    pub reply_to_user_id: Option<crate::SecretString>,
    pub reply_context_token: Option<crate::SecretString>,
    pub final_response: String,
}

pub trait WeixinRuntimeSink: Send + Sync {
    fn write_final_response(
        &self,
        record: WeixinRuntimeSinkRecord,
    ) -> Result<(), WeixinTurnSupervisorError>;

    fn write_outbound_text(
        &self,
        record: WeixinRuntimeSinkRecord,
    ) -> Result<(), WeixinTurnSupervisorError> {
        self.write_final_response(record)
    }
}

#[derive(Clone, Default)]
pub struct NoopWeixinRuntimeSink;

impl WeixinRuntimeSink for NoopWeixinRuntimeSink {
    fn write_final_response(
        &self,
        _record: WeixinRuntimeSinkRecord,
    ) -> Result<(), WeixinTurnSupervisorError> {
        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct WeixinRuntimeTestSink {
    records: Arc<Mutex<Vec<WeixinRuntimeSinkRecord>>>,
}

impl WeixinRuntimeTestSink {
    pub fn records(&self) -> Vec<WeixinRuntimeSinkRecord> {
        self.records
            .lock()
            .map(|records| records.clone())
            .unwrap_or_default()
    }
}

impl WeixinRuntimeSink for WeixinRuntimeTestSink {
    fn write_final_response(
        &self,
        record: WeixinRuntimeSinkRecord,
    ) -> Result<(), WeixinTurnSupervisorError> {
        self.records
            .lock()
            .map_err(|_| WeixinTurnSupervisorError::SinkFailed)?
            .push(record);
        Ok(())
    }
}

async fn supervise_remote_control_stream(
    hub: WeixinRemoteControlHub,
    scope: WeixinRemoteControlScope,
    mut stream: AgentRunStreamReceiver,
    timeout: Duration,
    sink: Arc<dyn WeixinRuntimeSink>,
    reply_to_user_id: Option<crate::SecretString>,
    reply_context_token: Option<crate::SecretString>,
) -> WeixinStreamObservationReport {
    let mut public_events = PublicTextAccumulator::default();
    let approval_grants = Arc::new(Mutex::new(WeixinTurnApprovalGrants::default()));
    loop {
        tokio::select! {
            Some(event) = stream.events.recv() => {
                public_events.observe(&event);
            }
            Some(request) = stream.approvals.recv() => {
                let request = match prepare_weixin_turn_approval_request(
                    request,
                    approval_grants.clone(),
                ) {
                    PreparedWeixinApprovalRequest::AutoApproved => continue,
                    PreparedWeixinApprovalRequest::NeedsRemoteApproval(request) => request,
                };
                let expires_at_millis = now_millis_u64().saturating_add(timeout.as_millis() as u64);
                match hub.register_approval(scope.clone(), request, expires_at_millis) {
                    Ok(prompt) => {
                        let request_id = prompt.request_id.clone();
                        if sink.write_outbound_text(WeixinRuntimeSinkRecord {
                            account_id: prompt.scope.account_id.clone(),
                            peer_id_hash: prompt.scope.peer_id_hash.clone(),
                            direct_message_key: prompt.scope.direct_message_key.clone(),
                            item_id: prompt.scope.item_id.clone(),
                            session_id: prompt.scope.session_id.clone(),
                            reply_to_user_id: reply_to_user_id.clone(),
                            reply_context_token: reply_context_token.clone(),
                            final_response: render_remote_control_prompt(&prompt),
                        }).is_err() {
                            let _ = hub.reject_request(&request_id, "prompt_delivery_failed", now_millis_u64());
                        } else {
                            spawn_remote_control_timeout(hub.clone(), request_id, timeout);
                        }
                    }
                    Err(error) => {
                        let _ = sink.write_outbound_text(WeixinRuntimeSinkRecord {
                            account_id: scope.account_id.clone(),
                            peer_id_hash: scope.peer_id_hash.clone(),
                            direct_message_key: scope.direct_message_key.clone(),
                            item_id: scope.item_id.clone(),
                            session_id: scope.session_id.clone(),
                            reply_to_user_id: reply_to_user_id.clone(),
                            reply_context_token: reply_context_token.clone(),
                            final_response: render_remote_control_error(&scope, &error),
                        });
                    }
                }
            }
            Some(request) = stream.user_inputs.recv() => {
                let expires_at_millis = now_millis_u64().saturating_add(timeout.as_millis() as u64);
                match hub.register_user_input(scope.clone(), request, expires_at_millis) {
                    Ok(prompt) => {
                        let request_id = prompt.request_id.clone();
                        if sink.write_outbound_text(WeixinRuntimeSinkRecord {
                            account_id: prompt.scope.account_id.clone(),
                            peer_id_hash: prompt.scope.peer_id_hash.clone(),
                            direct_message_key: prompt.scope.direct_message_key.clone(),
                            item_id: prompt.scope.item_id.clone(),
                            session_id: prompt.scope.session_id.clone(),
                            reply_to_user_id: reply_to_user_id.clone(),
                            reply_context_token: reply_context_token.clone(),
                            final_response: render_remote_control_prompt(&prompt),
                        }).is_err() {
                            let _ = hub.reject_request(&request_id, "prompt_delivery_failed", now_millis_u64());
                        } else {
                            spawn_remote_control_timeout(hub.clone(), request_id, timeout);
                        }
                    }
                    Err(error) => {
                        let _ = sink.write_outbound_text(WeixinRuntimeSinkRecord {
                            account_id: scope.account_id.clone(),
                            peer_id_hash: scope.peer_id_hash.clone(),
                            direct_message_key: scope.direct_message_key.clone(),
                            item_id: scope.item_id.clone(),
                            session_id: scope.session_id.clone(),
                            reply_to_user_id: reply_to_user_id.clone(),
                            reply_context_token: reply_context_token.clone(),
                            final_response: render_remote_control_error(&scope, &error),
                        });
                    }
                }
            }
            else => break,
        }
    }
    public_events.finish()
}

#[derive(Default)]
struct WeixinTurnApprovalGrants {
    read_only_shell_after_user_approval: bool,
}

enum PreparedWeixinApprovalRequest {
    AutoApproved,
    NeedsRemoteApproval(AgentRunApprovalRequest),
}

fn prepare_weixin_turn_approval_request(
    request: AgentRunApprovalRequest,
    approval_grants: Arc<Mutex<WeixinTurnApprovalGrants>>,
) -> PreparedWeixinApprovalRequest {
    let AgentRunApprovalRequest {
        id,
        tool_name,
        reason,
        command,
        cwd,
        respond_to,
    } = request;

    if should_auto_approve_weixin_turn_shell(&approval_grants, &tool_name, command.as_deref()) {
        let _ = respond_to.send(AgentRunApprovalDecision {
            approved: true,
            reason: Some("approved_by_weixin_turn_read_only_grant".to_string()),
        });
        return PreparedWeixinApprovalRequest::AutoApproved;
    }

    let batchable = is_weixin_turn_batchable_shell_approval(&tool_name, command.as_deref());
    let (proxy_tx, proxy_rx) = tokio::sync::oneshot::channel();
    tokio::spawn(forward_observed_weixin_approval_decision(
        proxy_rx,
        respond_to,
        approval_grants,
        batchable,
    ));

    PreparedWeixinApprovalRequest::NeedsRemoteApproval(AgentRunApprovalRequest {
        id,
        tool_name,
        reason,
        command,
        cwd,
        respond_to: proxy_tx,
    })
}

async fn forward_observed_weixin_approval_decision(
    proxy_rx: tokio::sync::oneshot::Receiver<AgentRunApprovalDecision>,
    respond_to: tokio::sync::oneshot::Sender<AgentRunApprovalDecision>,
    approval_grants: Arc<Mutex<WeixinTurnApprovalGrants>>,
    batchable: bool,
) {
    let Ok(decision) = proxy_rx.await else {
        return;
    };
    if decision.approved
        && batchable
        && let Ok(mut grants) = approval_grants.lock()
    {
        grants.read_only_shell_after_user_approval = true;
    }
    let _ = respond_to.send(decision);
}

fn should_auto_approve_weixin_turn_shell(
    approval_grants: &Arc<Mutex<WeixinTurnApprovalGrants>>,
    tool_name: &str,
    command: Option<&str>,
) -> bool {
    if !is_weixin_turn_batchable_shell_approval(tool_name, command) {
        return false;
    }
    approval_grants
        .lock()
        .map(|grants| grants.read_only_shell_after_user_approval)
        .unwrap_or(false)
}

fn is_weixin_turn_batchable_shell_approval(tool_name: &str, command: Option<&str>) -> bool {
    tool_name.eq_ignore_ascii_case("shell")
        && command.is_some_and(is_low_risk_read_only_shell_command)
}

fn is_low_risk_read_only_shell_command(command: &str) -> bool {
    let trimmed = command.trim();
    if trimmed.is_empty() || contains_shell_control_operator(trimmed) {
        return false;
    }
    let Some(first_token) = trimmed.split_whitespace().next() else {
        return false;
    };
    let first_token = first_token.to_ascii_lowercase();
    matches!(
        first_token.as_str(),
        "cat"
            | "dir"
            | "echo"
            | "gci"
            | "get-childitem"
            | "get-command"
            | "get-content"
            | "get-date"
            | "get-location"
            | "get-process"
            | "hostname"
            | "ls"
            | "pwd"
            | "select-string"
            | "test-path"
            | "type"
            | "whoami"
            | "write-output"
    )
}

fn contains_shell_control_operator(command: &str) -> bool {
    command.contains('\n')
        || command.contains('\r')
        || command.contains(';')
        || command.contains('|')
        || command.contains('>')
        || command.contains('<')
        || command.contains('&')
        || command.contains("$(")
}

#[derive(Default)]
struct PublicTextAccumulator {
    seen: HashSet<String>,
    chunks: Vec<String>,
    observed_event_count: usize,
    public_message_count: usize,
    duplicate_public_message_count: usize,
    filtered_non_public_count: usize,
    unsafe_public_message_count: usize,
    final_message_count: usize,
    terminal_status: Option<AgentRunStatus>,
}

impl PublicTextAccumulator {
    fn observe(&mut self, event: &AgentEvent) {
        self.observed_event_count = self.observed_event_count.saturating_add(1);
        match event {
            AgentEvent::Message { content, stream } => {
                self.observe_message(content, stream.as_ref())
            }
            AgentEvent::Completed { status, .. } => {
                self.terminal_status = Some(*status);
                self.filtered_non_public_count = self.filtered_non_public_count.saturating_add(1);
            }
            _ => {
                self.filtered_non_public_count = self.filtered_non_public_count.saturating_add(1);
            }
        }
    }

    fn observe_message(&mut self, content: &str, stream: Option<&AgentMessageStream>) {
        if content.trim().is_empty() {
            self.filtered_non_public_count = self.filtered_non_public_count.saturating_add(1);
            return;
        }
        if !is_safe_public_agent_text(content) {
            self.unsafe_public_message_count = self.unsafe_public_message_count.saturating_add(1);
            self.filtered_non_public_count = self.filtered_non_public_count.saturating_add(1);
            return;
        }

        let key = public_message_key(content, stream);
        if !self.seen.insert(key) {
            self.duplicate_public_message_count =
                self.duplicate_public_message_count.saturating_add(1);
            return;
        }

        if stream.is_some_and(|stream| stream.phase == AgentMessageStreamPhase::Final) {
            self.final_message_count = self.final_message_count.saturating_add(1);
        }
        self.public_message_count = self.public_message_count.saturating_add(1);
        self.chunks.push(content.to_string());
    }

    fn finish(self) -> WeixinStreamObservationReport {
        WeixinStreamObservationReport {
            policy: WEIXIN_STREAM_OUTPUT_POLICY,
            observed_event_count: self.observed_event_count,
            public_message_count: self.public_message_count,
            duplicate_public_message_count: self.duplicate_public_message_count,
            filtered_non_public_count: self.filtered_non_public_count,
            unsafe_public_message_count: self.unsafe_public_message_count,
            final_message_count: self.final_message_count,
            terminal_status: self.terminal_status,
            merged_public_text: merge_public_chunks(self.chunks),
        }
    }
}

fn public_message_key(content: &str, stream: Option<&AgentMessageStream>) -> String {
    match stream {
        Some(stream) => format!(
            "{}:{}:{}:{}:{}",
            stream.thread_id,
            stream.turn_id,
            stream.stream_id,
            stream.source_sequence.value(),
            content
        ),
        None => content.to_string(),
    }
}

fn merge_public_chunks(chunks: Vec<String>) -> Option<String> {
    let merged = chunks
        .into_iter()
        .map(|chunk| chunk.trim().to_string())
        .filter(|chunk| !chunk.is_empty())
        .collect::<Vec<_>>()
        .join("");
    (!merged.is_empty()).then_some(merged)
}

fn is_safe_public_agent_text(content: &str) -> bool {
    !contains_sensitive_agent_marker(content)
        && !contains_absolute_path_marker(content)
        && !contains_internal_agent_marker(content)
}

fn sanitize_public_final_response(content: &str) -> Option<String> {
    let mut lines = Vec::new();
    let mut awaiting_reason = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if is_internal_companion_heading(trimmed) {
            awaiting_reason = true;
            continue;
        }
        if contains_internal_agent_marker(trimmed) {
            continue;
        }
        if awaiting_reason {
            if trimmed.starts_with("原因：") || trimmed.starts_with("原因:") {
                awaiting_reason = false;
                continue;
            }
            awaiting_reason = false;
        }
        lines.push(line);
    }

    let mut sanitized = lines.join("\n");
    while sanitized.contains("\n\n\n") {
        sanitized = sanitized.replace("\n\n\n", "\n\n");
    }
    let sanitized = sanitized.trim().to_string();
    (!sanitized.is_empty()).then_some(sanitized)
}

fn is_internal_companion_heading(line: &str) -> bool {
    [
        "[关心]",
        "[下一步建议]",
        "[阶段总结]",
        "[需要确认的工具建议]",
    ]
    .iter()
    .any(|marker| line.starts_with(marker))
}

fn contains_sensitive_agent_marker(content: &str) -> bool {
    let lower = content.to_ascii_lowercase();
    [
        "api_key",
        "apikey",
        "authorization",
        "bearer ",
        "context_token",
        "cookie",
        "password",
        "secret",
        "sk-",
        "token",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn contains_absolute_path_marker(content: &str) -> bool {
    content.as_bytes().windows(3).any(|window| {
        window[0].is_ascii_alphabetic()
            && window[1] == b':'
            && (window[2] == b'\\' || window[2] == b'/')
    }) || content.contains("\\Users\\")
        || content.contains("/Users/")
        || content.contains("/home/")
}

fn contains_internal_agent_marker(content: &str) -> bool {
    let lower = content.to_ascii_lowercase();
    [
        "companion_policy_",
        "context_phase",
        "yunxi_persona_context",
        "<persona>",
        "<companion_rules",
        "<memory_context",
        "<boot_memory_context",
        "<dynamic_memory_context",
        "context_not_instruction",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn spawn_remote_control_timeout(
    hub: WeixinRemoteControlHub,
    request_id: String,
    timeout: Duration,
) {
    tokio::spawn(async move {
        tokio::time::sleep(timeout).await;
        let _ = hub.expire_request(&request_id, now_millis_u64());
    });
}

#[async_trait]
pub trait WeixinRuntimeDispatcher: Send + Sync {
    async fn dispatch_pending_turn(
        &self,
        account_id: &str,
        item_id: &str,
    ) -> Result<WeixinTurnReport, WeixinTurnSupervisorError>;
}

pub struct WeixinRuntimeDispatcherAdapter<B> {
    supervisor: WeixinTurnSupervisor,
    backend: Arc<B>,
}

impl<B> WeixinRuntimeDispatcherAdapter<B> {
    pub fn new(supervisor: WeixinTurnSupervisor, backend: B) -> Self {
        Self {
            supervisor,
            backend: Arc::new(backend),
        }
    }
}

#[async_trait]
impl<B> WeixinRuntimeDispatcher for WeixinRuntimeDispatcherAdapter<B>
where
    B: AgentBackend + 'static,
{
    async fn dispatch_pending_turn(
        &self,
        account_id: &str,
        item_id: &str,
    ) -> Result<WeixinTurnReport, WeixinTurnSupervisorError> {
        self.supervisor
            .run_pending_turn(self.backend.as_ref(), account_id, item_id)
            .await
    }
}

#[derive(Debug, Error)]
pub enum WeixinTurnSupervisorError {
    #[error("weixin runtime supervisor state failed: {0}")]
    State(#[from] WeixinStateError),
    #[error("weixin runtime supervisor payload decrypt failed: {0}")]
    PayloadCipher(#[from] WeixinPayloadCipherError),
    #[error("weixin runtime supervisor only accepts text payloads")]
    UnsupportedPayload,
    #[error("weixin runtime supervisor pending item is not ready item={item_id} state={state}")]
    InvalidPendingState {
        item_id: String,
        state: &'static str,
    },
    #[error("weixin runtime supervisor queue is full scope={scope} limit={limit}")]
    QueueFull { scope: &'static str, limit: usize },
    #[error("weixin runtime supervisor sink failed")]
    SinkFailed,
    #[error("weixin runtime supervisor voice processing failed: {0}")]
    Voice(#[from] WeixinVoiceError),
    #[error("weixin runtime supervisor remote control registration failed")]
    RemoteControlRegistrationFailed,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ConversationQueueKey {
    account_id: String,
    peer_id_hash: String,
    direct_message_key: String,
}

impl ConversationQueueKey {
    fn from_payload(payload: &WeixinPendingInboundPayload) -> Self {
        Self {
            account_id: payload.account_id.clone(),
            peer_id_hash: payload.peer_id_hash.clone(),
            direct_message_key: payload.direct_message_key.clone(),
        }
    }
}

struct ConversationQueueLimiter {
    state: Arc<Mutex<ConversationQueueState>>,
}

impl Default for ConversationQueueLimiter {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(ConversationQueueState::default())),
        }
    }
}

impl ConversationQueueLimiter {
    async fn acquire(
        &self,
        key: ConversationQueueKey,
        max_queue_per_conversation: usize,
        max_global_queue: usize,
    ) -> Result<ConversationTurnSlot, WeixinTurnSupervisorError> {
        let (lock, queued) = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| WeixinTurnSupervisorError::SinkFailed)?;
            let running = state.running.contains(&key);
            let waiting = state
                .waiting_per_conversation
                .get(&key)
                .copied()
                .unwrap_or(0);
            let queued = running || waiting > 0;
            if queued {
                if waiting >= max_queue_per_conversation {
                    return Err(WeixinTurnSupervisorError::QueueFull {
                        scope: "conversation",
                        limit: max_queue_per_conversation,
                    });
                }
                if state.global_waiting >= max_global_queue {
                    return Err(WeixinTurnSupervisorError::QueueFull {
                        scope: "global",
                        limit: max_global_queue,
                    });
                }
                state.global_waiting += 1;
                *state
                    .waiting_per_conversation
                    .entry(key.clone())
                    .or_default() += 1;
            }
            (
                state
                    .locks
                    .entry(key.clone())
                    .or_insert_with(|| Arc::new(TokioMutex::new(())))
                    .clone(),
                queued,
            )
        };

        let guard = lock.lock_owned().await;
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| WeixinTurnSupervisorError::SinkFailed)?;
            if queued {
                state.global_waiting = state.global_waiting.saturating_sub(1);
                if let Some(waiting) = state.waiting_per_conversation.get_mut(&key) {
                    *waiting = waiting.saturating_sub(1);
                    if *waiting == 0 {
                        state.waiting_per_conversation.remove(&key);
                    }
                }
            }
            state.running.insert(key.clone());
        }

        Ok(ConversationTurnSlot {
            key,
            state: Arc::clone(&self.state),
            _guard: guard,
        })
    }
}

#[derive(Default)]
struct ConversationQueueState {
    locks: HashMap<ConversationQueueKey, Arc<TokioMutex<()>>>,
    running: HashSet<ConversationQueueKey>,
    waiting_per_conversation: HashMap<ConversationQueueKey, usize>,
    global_waiting: usize,
}

struct ConversationTurnSlot {
    key: ConversationQueueKey,
    state: Arc<Mutex<ConversationQueueState>>,
    _guard: OwnedMutexGuard<()>,
}

impl Drop for ConversationTurnSlot {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            state.running.remove(&self.key);
        }
    }
}

fn terminal_reason_for_status(status: AgentRunStatus) -> Option<String> {
    match status {
        AgentRunStatus::Completed => None,
        AgentRunStatus::Failed => Some("runtime_failed".to_string()),
        AgentRunStatus::Cancelled => Some("runtime_cancelled".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote_control::WeixinRemoteControlHub;
    use std::time::Duration;
    use tokio::time::timeout;

    fn scope() -> WeixinRemoteControlScope {
        WeixinRemoteControlScope {
            account_id: "account#11111111".to_string(),
            peer_id_hash: "peer#22222222".to_string(),
            direct_message_key: "dm#33333333".to_string(),
            item_id: "item#44444444".to_string(),
            session_id: "session#55555555".to_string(),
        }
    }

    #[tokio::test]
    async fn remote_control_stream_emits_approval_prompt_to_outbound_sink() {
        let hub = WeixinRemoteControlHub::default();
        let (control, stream) = AgentRunControl::streaming();
        let sink = WeixinRuntimeTestSink::default();
        let sink_handle: Arc<dyn WeixinRuntimeSink> = Arc::new(sink.clone());
        let supervise = tokio::spawn(supervise_remote_control_stream(
            hub.clone(),
            scope(),
            stream,
            Duration::from_millis(200),
            sink_handle,
            Some(crate::SecretString::new("raw-user")),
            Some(crate::SecretString::new("raw-context")),
        ));
        let request_task = tokio::spawn(async move {
            let _ = control
                .request_approval(
                    Some("approval-1".to_string()),
                    "shell",
                    "safe test",
                    Some("echo ok".to_string()),
                    "D:/YunXi Agent",
                )
                .await;
        });

        timeout(Duration::from_secs(2), async {
            loop {
                if !sink.records().is_empty() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("approval prompt to be recorded");
        let record = sink.records().pop().expect("sink record");
        assert!(record.final_response.contains("[YunXi]"));
        assert!(record.final_response.contains("/approve"));
        assert!(record.final_response.contains("/deny"));
        for marker in [
            "purpose=",
            "request_id=",
            "account=",
            "peer=",
            "dm=",
            "item=",
            "session=",
            "expires_at_millis=",
            "account#11111111",
            "peer#22222222",
            "dm#33333333",
            "item#44444444",
            "session#55555555",
        ] {
            assert!(
                !record.final_response.contains(marker),
                "remote control prompt leaked {marker}: {}",
                record.final_response
            );
        }
        request_task.abort();
        let _ = request_task.await;
        let _ = supervise.await;
    }

    #[tokio::test]
    async fn remote_control_stream_auto_approves_later_read_only_shell_after_approval() {
        let hub = WeixinRemoteControlHub::default();
        let control_scope = scope();
        let (control, stream) = AgentRunControl::streaming();
        let sink = WeixinRuntimeTestSink::default();
        let sink_handle: Arc<dyn WeixinRuntimeSink> = Arc::new(sink.clone());
        let supervise = tokio::spawn(supervise_remote_control_stream(
            hub.clone(),
            control_scope.clone(),
            stream,
            Duration::from_secs(2),
            sink_handle,
            Some(crate::SecretString::new("raw-user")),
            Some(crate::SecretString::new("raw-context")),
        ));

        let first_control = control.clone();
        let first_request = tokio::spawn(async move {
            first_control
                .request_approval(
                    Some("approval-1".to_string()),
                    "shell",
                    "safe test",
                    Some("Get-Date".to_string()),
                    "D:/YunXi Agent",
                )
                .await
                .expect("first approval request")
                .expect("first decision")
        });

        timeout(Duration::from_secs(2), async {
            loop {
                if !sink.records().is_empty() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("first approval prompt to be recorded");
        assert_eq!(sink.records().len(), 1);

        hub.handle_command(
            &control_scope,
            crate::remote_control::WeixinRemoteCommand::Approve { request_id: None },
            now_millis_u64(),
        )
        .expect("approve first request");
        assert!(
            timeout(Duration::from_secs(2), first_request)
                .await
                .expect("first request timeout")
                .expect("first request task")
                .approved
        );

        let second_decision = timeout(
            Duration::from_secs(2),
            control.request_approval(
                Some("approval-2".to_string()),
                "shell",
                "safe test",
                Some("Get-Location".to_string()),
                "D:/YunXi Agent",
            ),
        )
        .await
        .expect("second request timeout")
        .expect("second approval request")
        .expect("second decision");

        assert!(second_decision.approved);
        assert_eq!(
            second_decision.reason.as_deref(),
            Some("approved_by_weixin_turn_read_only_grant")
        );
        assert_eq!(
            sink.records().len(),
            1,
            "auto-approved read-only shell requests should not emit another prompt"
        );

        drop(control);
        let _ = supervise.await;
    }

    #[test]
    fn weixin_turn_batchable_shell_approval_stays_read_only() {
        assert!(is_low_risk_read_only_shell_command("Get-Date"));
        assert!(is_low_risk_read_only_shell_command("echo ok"));
        assert!(is_low_risk_read_only_shell_command("Test-Path Cargo.toml"));
        assert!(!is_low_risk_read_only_shell_command("Remove-Item target"));
        assert!(!is_low_risk_read_only_shell_command("echo ok > out.txt"));
        assert!(!is_low_risk_read_only_shell_command(
            "Get-ChildItem | Remove-Item"
        ));
        assert!(!is_low_risk_read_only_shell_command("cargo test"));
    }

    #[test]
    fn public_final_response_hides_internal_companion_blocks() {
        let content = "正常回复。\n\n[关心] 最近的上下文有变化：关系记录更新。\n原因：a recent relationship or context milestone changed";

        assert_eq!(
            sanitize_public_final_response(content).as_deref(),
            Some("正常回复。")
        );
    }

    #[test]
    fn public_final_response_drops_companion_only_messages() {
        let content = "[阶段总结] 当前阶段已完成。\n原因：a bounded stage summary is due";

        assert_eq!(sanitize_public_final_response(content), None);
    }

    #[test]
    fn public_final_response_drops_internal_companion_policy_metadata() {
        let content = "正常回复。\ncompanion_policy_emotion_kind=anxiety\n<yunxi_persona_context version=\"2.3.3\">";

        assert_eq!(
            sanitize_public_final_response(content).as_deref(),
            Some("正常回复。")
        );
        assert!(!is_safe_public_agent_text("context_phase=companion_policy"));
    }
}

fn now_millis_u64() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

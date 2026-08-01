use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use async_trait::async_trait;
use thiserror::Error;
use tokio::task::JoinHandle;
use tokio::time::{Instant, sleep};
use yunxi_agent_storage::{
    FileWeixinStateStore, WeixinConnectionStateRecord, WeixinInboundBatchCommit,
    WeixinInboundCommitItem, WeixinPairRequest, WeixinPairRequestCommitItem,
    WeixinPairRequestState, WeixinStateError, WeixinStateSnapshot, WeixinStateStore,
};

use crate::backoff::WeixinBackoff;
use crate::delivery::{WeixinDeliveryDispatcher, WeixinDeliveryError};
use crate::ilink::{GetUpdatesRequest, GetUpdatesResponse, IlinkHttpClient, WeixinMessage};
use crate::inbound::{WeixinInboundEnvelope, WeixinInboundKind};
use crate::payload_cipher::{WeixinPayloadAad, WeixinPayloadCipher, WeixinPayloadCipherError};
use crate::remote_control::{
    WeixinRemoteControlHub, WeixinRemoteControlScope, parse_weixin_remote_command,
    render_remote_control_error, render_remote_control_outcome,
};
use crate::turn_supervisor::{WeixinRuntimeDispatcher, WeixinRuntimeSink, WeixinRuntimeSinkRecord};
use crate::{SecretString, WeixinApiError};

const DEFAULT_PAIR_REQUEST_TTL: Duration = Duration::from_secs(10 * 60);
const DEFAULT_EMPTY_POLL_DELAY: Duration = Duration::from_millis(250);
const MAX_EMPTY_POLL_DELAY: Duration = Duration::from_secs(30);
const READY_PENDING_DRAIN_LIMIT: usize = 32;
const MIN_READY_PENDING_RETRY_DELAY: Duration = Duration::from_millis(250);
const MAX_READY_PENDING_RETRY_DELAY: Duration = Duration::from_secs(30);
const DEFAULT_RUNTIME_DISPATCH_LEASE: Duration = Duration::from_secs(60);
const DEFAULT_RUNTIME_DISPATCH_SHUTDOWN_WAIT: Duration = Duration::from_secs(3);

#[derive(Clone)]
pub struct WeixinServeOptions {
    pub account_id: String,
    pub cursor_source: String,
    pub pairing_required: bool,
    pub pair_request_ttl: Duration,
    pub empty_poll_delay: Duration,
    pub max_polls: Option<usize>,
    pub self_user_id: Option<SecretString>,
    pub runtime_dispatcher: Option<Arc<dyn WeixinRuntimeDispatcher>>,
    pub delivery_dispatcher: Option<Arc<WeixinDeliveryDispatcher>>,
    pub outbound_sink: Option<Arc<dyn WeixinRuntimeSink>>,
    pub remote_control_hub: Option<WeixinRemoteControlHub>,
    pub background_runtime_dispatch: bool,
    pub runtime_dispatch_lease: Duration,
    pub runtime_dispatch_shutdown_wait: Duration,
}

impl WeixinServeOptions {
    pub fn new(account_id: impl Into<String>) -> Self {
        Self {
            account_id: account_id.into(),
            cursor_source: "getupdates".to_string(),
            pairing_required: true,
            pair_request_ttl: DEFAULT_PAIR_REQUEST_TTL,
            empty_poll_delay: DEFAULT_EMPTY_POLL_DELAY,
            max_polls: None,
            self_user_id: None,
            runtime_dispatcher: None,
            delivery_dispatcher: None,
            outbound_sink: None,
            remote_control_hub: None,
            background_runtime_dispatch: false,
            runtime_dispatch_lease: DEFAULT_RUNTIME_DISPATCH_LEASE,
            runtime_dispatch_shutdown_wait: DEFAULT_RUNTIME_DISPATCH_SHUTDOWN_WAIT,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WeixinServeStoppedReason {
    Cancelled,
    MaxPolls,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WeixinServeReport {
    pub polls: usize,
    pub empty_polls: usize,
    pub accepted_count: usize,
    pub duplicate_count: usize,
    pub pair_request_count: usize,
    pub pair_prompt_count: usize,
    pub pair_prompt_error_count: usize,
    pub skipped_group_count: usize,
    pub skipped_self_count: usize,
    pub skipped_unsupported_count: usize,
    pub skipped_unknown_count: usize,
    pub network_error_count: usize,
    pub runtime_dispatch_count: usize,
    pub runtime_error_count: usize,
    pub runtime_deferred_count: usize,
    pub delivery_attempt_count: usize,
    pub delivery_success_count: usize,
    pub delivery_error_count: usize,
    pub delivery_deferred_count: usize,
    pub delivery_unknown_outcome_count: usize,
    pub remote_control_count: usize,
    pub remote_control_error_count: usize,
    pub runtime_recovered_count: usize,
    pub stopped_reason: Option<WeixinServeStoppedReason>,
}

#[derive(Clone, Default)]
pub struct WeixinServeCancellation(Arc<AtomicBool>);

impl WeixinServeCancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Debug, Error)]
pub enum WeixinServeError {
    #[error("weixin serve state failed: {0}")]
    State(#[from] WeixinStateError),
    #[error("weixin serve poll failed: {0}")]
    Api(#[from] WeixinApiError),
    #[error("weixin serve pending payload encryption failed: {0}")]
    PayloadCipher(#[from] WeixinPayloadCipherError),
    #[error("weixin serve delivery failed: {0}")]
    Delivery(#[from] WeixinDeliveryError),
    #[error("weixin serve paused because the credential is expired or invalid")]
    CredentialExpired,
}

#[async_trait]
pub trait WeixinUpdatesTransport: Send {
    async fn get_updates(
        &mut self,
        request: GetUpdatesRequest,
    ) -> Result<GetUpdatesResponse, WeixinApiError>;
}

#[async_trait]
impl WeixinUpdatesTransport for IlinkHttpClient {
    async fn get_updates(
        &mut self,
        request: GetUpdatesRequest,
    ) -> Result<GetUpdatesResponse, WeixinApiError> {
        IlinkHttpClient::get_updates(self, request).await
    }
}

pub async fn run_weixin_serve_loop<T>(
    transport: &mut T,
    state_store: &FileWeixinStateStore,
    options: WeixinServeOptions,
    data_key: &SecretString,
    cancellation: &WeixinServeCancellation,
) -> Result<WeixinServeReport, WeixinServeError>
where
    T: WeixinUpdatesTransport,
{
    let mut report = WeixinServeReport::default();
    let mut backoff = WeixinBackoff::default();
    let payload_cipher = WeixinPayloadCipher::new();
    payload_cipher.validate_data_key(data_key)?;
    let runtime_lease_owner = runtime_lease_owner_label();
    let mut background_runtime_tasks = BackgroundRuntimeTasks::default();
    let work_result: Result<(), WeixinServeError> = async {
        loop {
            if cancellation.is_cancelled() {
                report.stopped_reason = Some(WeixinServeStoppedReason::Cancelled);
                break;
            }
            if options
                .max_polls
                .is_some_and(|max_polls| report.polls >= max_polls)
            {
                report.stopped_reason = Some(WeixinServeStoppedReason::MaxPolls);
                break;
            }

            background_runtime_tasks
                .reap_finished(state_store, &mut report)
                .await?;
            drain_ready_deliveries(&options, &mut report).await?;
            report.runtime_recovered_count += state_store.recover_stale_pending_runtime_turns(
                &options.account_id,
                &runtime_lease_owner,
                now_millis_u64(),
            )?;
            drain_ready_pending(
                state_store,
                &options,
                &runtime_lease_owner,
                &mut background_runtime_tasks,
                &mut report,
            )
            .await?;

            let state = state_store.load(&options.account_id)?.ok_or_else(|| {
                WeixinStateError::StateNotFound {
                    account_id: options.account_id.clone(),
                }
            })?;
            let cursor = cursor_for(&state, &options.cursor_source).unwrap_or_default();
            let response = match transport
                .get_updates(GetUpdatesRequest::new(cursor.to_string()))
                .await
            {
                Ok(response) => response,
                Err(error) if is_credential_expired(&error) => {
                    state_store.record_poll_health(
                        &options.account_id,
                        WeixinConnectionStateRecord::Suspended,
                        Some("credential_expired".to_string()),
                        now_millis_u64(),
                    )?;
                    return Err(WeixinServeError::CredentialExpired);
                }
                Err(error) => {
                    report.polls += 1;
                    report.network_error_count += 1;
                    state_store.record_poll_health(
                        &options.account_id,
                        WeixinConnectionStateRecord::Ready,
                        Some(redacted_error_label(&error).to_string()),
                        now_millis_u64(),
                    )?;
                    let delay = backoff.next_delay();
                    if options
                        .max_polls
                        .is_some_and(|max_polls| report.polls >= max_polls)
                    {
                        report.stopped_reason = Some(WeixinServeStoppedReason::MaxPolls);
                        break;
                    }
                    sleep_cancelable(delay, cancellation).await;
                    continue;
                }
            };
            report.polls += 1;
            backoff.reset();
            let now = now_millis_u64();
            let mut commit = WeixinInboundBatchCommit::new(options.account_id.clone(), now);
            commit.cursor_source = options.cursor_source.clone();
            commit.next_get_updates_buf = response
                .get_updates_buf
                .as_ref()
                .filter(|cursor| !cursor.is_empty())
                .map(|cursor| cursor.expose().to_string());
            commit.connection_state = Some(WeixinConnectionStateRecord::Ready);
            commit.last_redacted_error = None;

            let inbound_messages = response.inbound_messages();
            let mut pair_prompt_candidates = Vec::new();
            if inbound_messages.is_empty() {
                report.empty_polls += 1;
            }
            for message in &inbound_messages {
                let envelope = WeixinInboundEnvelope::from_message(
                    &options.account_id,
                    options.self_user_id.as_ref(),
                    message,
                    now,
                );
                match envelope.kind {
                    WeixinInboundKind::Text => {
                        if options.pairing_required
                            && !peer_is_approved(&state, &envelope.peer_id_hash, now)
                        {
                            let existing_pair_request =
                                active_pending_pair_request(&state, &envelope.peer_id_hash, now)
                                    .is_some();
                            let item_id = envelope.pending_item_id();
                            commit.pair_requests.push(WeixinPairRequestCommitItem {
                                peer_id_hash: envelope.peer_id_hash.clone(),
                                expires_at_millis: now
                                    .saturating_add(options.pair_request_ttl.as_millis() as u64),
                            });
                            pair_prompt_candidates.push(PairPromptCandidate {
                                peer_id_hash: envelope.peer_id_hash,
                                direct_message_key: envelope.direct_message_key,
                                item_id,
                                reply_to_user_id: message.reply_target_id(),
                                reply_context_token: message.context_token.clone(),
                                existing_pair_request,
                            });
                        } else if let Some(command) = remote_command_from_message(message) {
                            let scope = WeixinRemoteControlScope {
                                account_id: envelope.account_id.clone(),
                                peer_id_hash: envelope.peer_id_hash.clone(),
                                direct_message_key: envelope.direct_message_key.clone(),
                                item_id: envelope.pending_item_id(),
                                session_id: "remote-command".to_string(),
                            };
                            match options.remote_control_hub.as_ref() {
                                Some(hub) => match hub.handle_command(&scope, command, now) {
                                    Ok(outcome) => {
                                        report.remote_control_count += 1;
                                        if let Some(sink) = options.outbound_sink.as_ref() {
                                            let outbound_scope = outcome.scope.clone();
                                            if sink
                                                .write_outbound_text(WeixinRuntimeSinkRecord {
                                                    account_id: outbound_scope.account_id,
                                                    peer_id_hash: outbound_scope.peer_id_hash,
                                                    direct_message_key: outbound_scope
                                                        .direct_message_key,
                                                    item_id: outbound_scope.item_id,
                                                    session_id: outbound_scope.session_id,
                                                    reply_to_user_id: Some(
                                                        message.reply_target_id(),
                                                    ),
                                                    reply_context_token: message
                                                        .context_token
                                                        .clone(),
                                                    final_response: render_remote_control_outcome(
                                                        &outcome,
                                                    ),
                                                })
                                                .is_err()
                                            {
                                                report.remote_control_error_count += 1;
                                            }
                                        } else {
                                            report.remote_control_error_count += 1;
                                        }
                                    }
                                    Err(error) => {
                                        report.remote_control_error_count += 1;
                                        if let Some(sink) = options.outbound_sink.as_ref() {
                                            let _ =
                                                sink.write_outbound_text(WeixinRuntimeSinkRecord {
                                                    account_id: scope.account_id.clone(),
                                                    peer_id_hash: scope.peer_id_hash.clone(),
                                                    direct_message_key: scope
                                                        .direct_message_key
                                                        .clone(),
                                                    item_id: scope.item_id.clone(),
                                                    session_id: scope.session_id.clone(),
                                                    reply_to_user_id: Some(
                                                        message.reply_target_id(),
                                                    ),
                                                    reply_context_token: message
                                                        .context_token
                                                        .clone(),
                                                    final_response: render_remote_control_error(
                                                        &scope, &error,
                                                    ),
                                                });
                                        }
                                    }
                                },
                                None => {
                                    report.remote_control_error_count += 1;
                                    if let Some(sink) = options.outbound_sink.as_ref() {
                                        let _ = sink.write_outbound_text(WeixinRuntimeSinkRecord {
                                            account_id: scope.account_id.clone(),
                                            peer_id_hash: scope.peer_id_hash.clone(),
                                            direct_message_key: scope.direct_message_key.clone(),
                                            item_id: scope.item_id.clone(),
                                            session_id: scope.session_id.clone(),
                                            reply_to_user_id: Some(message.reply_target_id()),
                                            reply_context_token: message.context_token.clone(),
                                            final_response: "远程控制未启用，命令没有执行。"
                                                .to_string(),
                                        });
                                    }
                                }
                            }
                            continue;
                        } else {
                            let item_id = envelope.pending_item_id();
                            let encrypted_payload_ref = envelope.encrypted_payload_ref();
                            let plaintext = envelope
                                .recoverable_text_payload(message, &item_id)
                                .ok_or(WeixinPayloadCipherError::InvalidPlaintext)?;
                            let aad = WeixinPayloadAad::new(
                                &envelope.account_id,
                                &envelope.peer_id_hash,
                                &envelope.message_id_hash,
                                &item_id,
                            );
                            let encrypted_payload = payload_cipher
                                .encrypt_pending_inbound(data_key, &plaintext, &aad)?;
                            commit.accepted.push(WeixinInboundCommitItem {
                                item_id,
                                message_id_hash: envelope.message_id_hash.clone(),
                                peer_id_hash: envelope.peer_id_hash.clone(),
                                direct_message_key: envelope.direct_message_key.clone(),
                                encrypted_payload_ref,
                                encrypted_payload,
                                payload_kind: Some(envelope.kind.as_str().to_string()),
                            });
                        }
                    }
                    WeixinInboundKind::GroupMessage => report.skipped_group_count += 1,
                    WeixinInboundKind::SelfMessage => report.skipped_self_count += 1,
                    WeixinInboundKind::UnsupportedAttachment => {
                        report.skipped_unsupported_count += 1
                    }
                    WeixinInboundKind::Unknown => report.skipped_unknown_count += 1,
                }
            }

            let result = state_store.commit_inbound_batch(commit)?;
            report.accepted_count += result.accepted_count;
            report.duplicate_count += result.duplicate_count;
            report.pair_request_count += result.pair_request_count;
            let pair_prompt_report =
                send_pairing_prompts(state_store, &options, &pair_prompt_candidates, now).await?;
            report.pair_prompt_count += pair_prompt_report.sent_count;
            report.pair_prompt_error_count += pair_prompt_report.error_count;

            drain_ready_pending(
                state_store,
                &options,
                &runtime_lease_owner,
                &mut background_runtime_tasks,
                &mut report,
            )
            .await?;
            drain_ready_deliveries(&options, &mut report).await?;

            if options
                .max_polls
                .is_some_and(|max_polls| report.polls >= max_polls)
            {
                report.stopped_reason = Some(WeixinServeStoppedReason::MaxPolls);
                break;
            }
            if inbound_messages.is_empty() {
                let delay = response
                    .longpolling_timeout_ms
                    .map(Duration::from_millis)
                    .unwrap_or(options.empty_poll_delay)
                    .min(MAX_EMPTY_POLL_DELAY);
                sleep_cancelable(delay, cancellation).await;
            }
        }
        Ok(())
    }
    .await;
    let shutdown_result = background_runtime_tasks
        .shutdown(
            state_store,
            &options.account_id,
            &runtime_lease_owner,
            options.runtime_dispatch_shutdown_wait,
            &mut report,
        )
        .await;
    if work_result.is_ok() {
        shutdown_result?;
    }
    work_result?;
    Ok(report)
}

async fn drain_ready_deliveries(
    options: &WeixinServeOptions,
    report: &mut WeixinServeReport,
) -> Result<(), WeixinServeError> {
    let Some(dispatcher) = options.delivery_dispatcher.as_ref() else {
        return Ok(());
    };
    let delivery_report = dispatcher
        .drain_ready(&options.account_id, now_millis_u64())
        .await?;
    report.delivery_attempt_count += delivery_report.attempted_count;
    report.delivery_success_count += delivery_report.succeeded_count;
    report.delivery_error_count += delivery_report.failed_count;
    report.delivery_deferred_count += delivery_report.deferred_count;
    report.delivery_unknown_outcome_count += delivery_report.unknown_outcome_count;
    Ok(())
}

#[derive(Clone)]
struct PairPromptCandidate {
    peer_id_hash: String,
    direct_message_key: String,
    item_id: String,
    reply_to_user_id: SecretString,
    reply_context_token: Option<SecretString>,
    existing_pair_request: bool,
}

#[derive(Default)]
struct PairPromptReport {
    sent_count: usize,
    error_count: usize,
}

async fn send_pairing_prompts(
    state_store: &FileWeixinStateStore,
    options: &WeixinServeOptions,
    candidates: &[PairPromptCandidate],
    now_millis: u64,
) -> Result<PairPromptReport, WeixinServeError> {
    let mut report = PairPromptReport::default();
    if candidates.is_empty() {
        return Ok(report);
    }
    let Some(sink) = options.outbound_sink.as_ref() else {
        report.error_count += candidates.len();
        return Ok(report);
    };
    let state =
        state_store
            .load(&options.account_id)?
            .ok_or_else(|| WeixinStateError::StateNotFound {
                account_id: options.account_id.clone(),
            })?;
    for candidate in candidates {
        let Some(request) =
            active_pending_pair_request(&state, &candidate.peer_id_hash, now_millis)
        else {
            report.error_count += 1;
            continue;
        };
        let response = render_pairing_prompt(request, candidate.existing_pair_request, now_millis);
        if sink
            .write_outbound_text(WeixinRuntimeSinkRecord {
                account_id: options.account_id.clone(),
                peer_id_hash: candidate.peer_id_hash.clone(),
                direct_message_key: candidate.direct_message_key.clone(),
                item_id: candidate.item_id.clone(),
                session_id: "pairing-request".to_string(),
                reply_to_user_id: Some(candidate.reply_to_user_id.clone()),
                reply_context_token: candidate.reply_context_token.clone(),
                final_response: response,
            })
            .is_ok()
        {
            report.sent_count += 1;
        } else {
            report.error_count += 1;
        }
    }
    Ok(report)
}

async fn drain_ready_pending(
    state_store: &FileWeixinStateStore,
    options: &WeixinServeOptions,
    runtime_lease_owner: &str,
    background_runtime_tasks: &mut BackgroundRuntimeTasks,
    report: &mut WeixinServeReport,
) -> Result<(), WeixinServeError> {
    let Some(dispatcher) = options.runtime_dispatcher.as_ref() else {
        return Ok(());
    };
    let now = now_millis_u64();
    let pending_items = state_store.load_ready_pending_inbound(
        &options.account_id,
        now,
        READY_PENDING_DRAIN_LIMIT,
    )?;
    for pending in pending_items {
        if options.background_runtime_dispatch {
            let lease_expires_at =
                now.saturating_add(options.runtime_dispatch_lease.as_millis() as u64);
            let attempt_token = format!("attempt#{}-{}", now, pending.dispatch_retry_count);
            state_store.claim_pending_runtime_dispatch(
                &options.account_id,
                &pending.item_id,
                runtime_lease_owner,
                &attempt_token,
                lease_expires_at,
                now,
            )?;
            background_runtime_tasks.spawn(
                Arc::clone(dispatcher),
                options.account_id.clone(),
                runtime_lease_owner.to_string(),
                pending.item_id,
                pending.dispatch_retry_count,
            );
            report.runtime_dispatch_count += 1;
            continue;
        }
        match dispatcher
            .dispatch_pending_turn(&options.account_id, &pending.item_id)
            .await
        {
            Ok(_) => report.runtime_dispatch_count += 1,
            Err(error @ crate::turn_supervisor::WeixinTurnSupervisorError::QueueFull { .. }) => {
                report.runtime_error_count += 1;
                report.runtime_deferred_count += 1;
                let delay = ready_pending_retry_delay(pending.dispatch_retry_count);
                state_store.record_pending_runtime_dispatch_deferred(
                    &options.account_id,
                    &pending.item_id,
                    runtime_dispatch_error_label(&error),
                    now.saturating_add(delay.as_millis() as u64),
                    now,
                )?;
                break;
            }
            Err(error) => {
                report.runtime_error_count += 1;
                state_store.complete_pending_runtime_turn(
                    &options.account_id,
                    &pending.item_id,
                    yunxi_agent_storage::WeixinPendingInboundState::Failed,
                    Some(runtime_dispatch_error_label(&error).to_string()),
                    now,
                )?;
            }
        }
    }
    Ok(())
}

#[derive(Default)]
struct BackgroundRuntimeTasks {
    tasks: Vec<BackgroundRuntimeTask>,
}

struct BackgroundRuntimeTask {
    account_id: String,
    item_id: String,
    lease_owner: String,
    retry_count: u32,
    handle: JoinHandle<BackgroundRuntimeOutcome>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum BackgroundRuntimeOutcome {
    Completed,
    QueueFull { error_label: &'static str },
    Failed { error_label: &'static str },
}

impl BackgroundRuntimeTasks {
    fn spawn(
        &mut self,
        dispatcher: Arc<dyn WeixinRuntimeDispatcher>,
        account_id: String,
        lease_owner: String,
        item_id: String,
        retry_count: u32,
    ) {
        let account_id_for_task = account_id.clone();
        let item_id_for_task = item_id.clone();
        let handle = tokio::spawn(async move {
            match dispatcher
                .dispatch_pending_turn(&account_id_for_task, &item_id_for_task)
                .await
            {
                Ok(_) => BackgroundRuntimeOutcome::Completed,
                Err(
                    error @ crate::turn_supervisor::WeixinTurnSupervisorError::QueueFull { .. },
                ) => BackgroundRuntimeOutcome::QueueFull {
                    error_label: runtime_dispatch_error_label(&error),
                },
                Err(error) => {
                    let error_label = runtime_dispatch_error_label(&error);
                    BackgroundRuntimeOutcome::Failed { error_label }
                }
            }
        });
        self.tasks.push(BackgroundRuntimeTask {
            account_id,
            item_id,
            lease_owner,
            retry_count,
            handle,
        });
    }

    async fn reap_finished(
        &mut self,
        state_store: &FileWeixinStateStore,
        report: &mut WeixinServeReport,
    ) -> Result<(), WeixinServeError> {
        let mut pending = Vec::with_capacity(self.tasks.len());
        let mut finished = Vec::new();
        for task in self.tasks.drain(..) {
            if task.handle.is_finished() {
                finished.push(task);
            } else {
                pending.push(task);
            }
        }
        self.tasks = pending;
        for task in finished {
            match task.handle.await {
                Ok(BackgroundRuntimeOutcome::Completed) => {}
                Ok(BackgroundRuntimeOutcome::QueueFull { error_label }) => {
                    let now = now_millis_u64();
                    let delay = ready_pending_retry_delay(task.retry_count);
                    let _ = state_store.record_pending_runtime_dispatch_deferred(
                        &task.account_id,
                        &task.item_id,
                        error_label,
                        now.saturating_add(delay.as_millis() as u64),
                        now,
                    );
                    report.runtime_error_count += 1;
                    report.runtime_deferred_count += 1;
                }
                Ok(BackgroundRuntimeOutcome::Failed { error_label }) => {
                    let now = now_millis_u64();
                    if let Err(error) = state_store.complete_pending_runtime_turn(
                        &task.account_id,
                        &task.item_id,
                        yunxi_agent_storage::WeixinPendingInboundState::Failed,
                        Some(error_label.to_string()),
                        now,
                    ) {
                        if !matches!(error, WeixinStateError::PendingInboundNotFound { .. }) {
                            return Err(error.into());
                        }
                    }
                    report.runtime_error_count += 1;
                }
                Err(error) if error.is_panic() => {
                    let now = now_millis_u64();
                    let _ = state_store.recover_pending_runtime_dispatch_for_owner(
                        &task.account_id,
                        &task.item_id,
                        &task.lease_owner,
                        "runtime_dispatch_panic",
                        now,
                    )?;
                    report.runtime_error_count += 1;
                    report.runtime_deferred_count += 1;
                }
                Err(_error) => {
                    let now = now_millis_u64();
                    let _ = state_store.recover_pending_runtime_dispatch_for_owner(
                        &task.account_id,
                        &task.item_id,
                        &task.lease_owner,
                        "runtime_dispatch_aborted",
                        now,
                    )?;
                    report.runtime_deferred_count += 1;
                }
            }
        }
        Ok(())
    }

    async fn shutdown(
        &mut self,
        state_store: &FileWeixinStateStore,
        account_id: &str,
        active_lease_owner: &str,
        wait: Duration,
        report: &mut WeixinServeReport,
    ) -> Result<(), WeixinServeError> {
        let deadline = Instant::now() + wait;
        while !self.tasks.is_empty() && Instant::now() < deadline {
            self.reap_finished(state_store, report).await?;
            if self.tasks.is_empty() {
                return Ok(());
            }
            sleep(Duration::from_millis(25)).await;
        }
        if self.tasks.is_empty() {
            return Ok(());
        }
        for task in &self.tasks {
            task.handle.abort();
            let _ = state_store.recover_pending_runtime_dispatch_for_owner(
                account_id,
                &task.item_id,
                active_lease_owner,
                "runtime_dispatch_shutdown",
                now_millis_u64(),
            )?;
            report.runtime_deferred_count += 1;
        }
        let mut tasks = Vec::new();
        std::mem::swap(&mut tasks, &mut self.tasks);
        for task in tasks {
            let _ = task.handle.await;
        }
        Ok(())
    }
}

fn ready_pending_retry_delay(retry_count: u32) -> Duration {
    let shift = retry_count.min(7);
    let millis = (MIN_READY_PENDING_RETRY_DELAY.as_millis() as u64)
        .saturating_mul(1_u64 << shift)
        .min(MAX_READY_PENDING_RETRY_DELAY.as_millis() as u64);
    Duration::from_millis(millis)
}

fn runtime_dispatch_error_label(
    error: &crate::turn_supervisor::WeixinTurnSupervisorError,
) -> &'static str {
    match error {
        crate::turn_supervisor::WeixinTurnSupervisorError::QueueFull { .. } => "runtime_queue_full",
        crate::turn_supervisor::WeixinTurnSupervisorError::InvalidPendingState { .. } => {
            "runtime_invalid_pending_state"
        }
        crate::turn_supervisor::WeixinTurnSupervisorError::UnsupportedPayload => {
            "runtime_unsupported_payload"
        }
        crate::turn_supervisor::WeixinTurnSupervisorError::PayloadCipher(_) => {
            "runtime_payload_unavailable"
        }
        crate::turn_supervisor::WeixinTurnSupervisorError::State(error) => {
            runtime_state_error_label(error)
        }
        crate::turn_supervisor::WeixinTurnSupervisorError::SinkFailed => "runtime_sink_failed",
        crate::turn_supervisor::WeixinTurnSupervisorError::RemoteControlRegistrationFailed => {
            "runtime_remote_control_unavailable"
        }
    }
}

fn runtime_state_error_label(error: &WeixinStateError) -> &'static str {
    match error {
        WeixinStateError::Io { operation, .. } => match *operation {
            "create_state_dir" => "runtime_state_io_create_dir",
            "create_temp" => "runtime_state_io_create_temp",
            "write_temp" => "runtime_state_io_write_temp",
            "replace" => "runtime_state_io_replace",
            "read" => "runtime_state_io_read",
            "delete" => "runtime_state_io_delete",
            _ => "runtime_state_io",
        },
        WeixinStateError::InvalidJson { .. } => "runtime_state_invalid_json",
        WeixinStateError::FutureSchema { .. } => "runtime_state_future_schema",
        WeixinStateError::InvalidRecord { reason } => match *reason {
            "state path has no parent" => "runtime_state_invalid_path",
            "state serialization failed" => "runtime_state_serialization_failed",
            "pending inbound turn session id failed validation" => {
                "runtime_state_pending_session_invalid"
            }
            "pending inbound dispatch error failed validation" => {
                "runtime_state_dispatch_error_invalid"
            }
            "runtime lease failed validation" => "runtime_state_lease_invalid",
            "runtime lease recovery failed validation" => "runtime_state_lease_recovery_invalid",
            "delivery id already reached terminal state" => {
                "runtime_state_delivery_terminal_duplicate"
            }
            "delivery manifest failed validation" => "runtime_state_delivery_manifest_invalid",
            "conversation binding failed validation" => {
                "runtime_state_invalid_conversation_binding"
            }
            "conversation binding request failed validation" => {
                "runtime_state_invalid_binding_request"
            }
            "pending inbound binding mismatch" => "runtime_state_pending_binding_mismatch",
            "pending inbound direct message key mismatch" => "runtime_state_pending_dm_mismatch",
            "pending inbound turn session id mismatch" => "runtime_state_pending_session_mismatch",
            "delivery manifest item mismatch" => "runtime_state_delivery_item_mismatch",
            "delivery manifest already exists for turn" => {
                "runtime_state_delivery_manifest_duplicate"
            }
            "delivery id already exists" => "runtime_state_delivery_id_duplicate",
            "remote control commit failed validation" => {
                "runtime_state_remote_control_commit_invalid"
            }
            "remote control status failed validation" => {
                "runtime_state_remote_control_status_invalid"
            }
            "delivery error failed validation" => "runtime_state_delivery_error_invalid",
            "delivery terminal state is required" => "runtime_state_delivery_terminal_invalid",
            "delivery status failed validation" => "runtime_state_delivery_status_invalid",
            "pending delivery commit failed validation" => {
                "runtime_state_pending_delivery_commit_invalid"
            }
            "pending delivery encrypted payload is missing" => {
                "runtime_state_pending_delivery_payload_missing"
            }
            "remote control request failed validation" => {
                "runtime_state_remote_control_request_invalid"
            }
            "pending inbound failed validation" => "runtime_state_pending_inbound_invalid",
            "pending inbound encrypted payload is missing" => {
                "runtime_state_pending_inbound_payload_missing"
            }
            "pending inbound encrypted payload algorithm is unsupported" => {
                "runtime_state_pending_payload_algorithm_unsupported"
            }
            "pending inbound encrypted payload algorithm version is unsupported" => {
                "runtime_state_pending_payload_algorithm_version_unsupported"
            }
            "pending inbound encrypted payload aad version is unsupported" => {
                "runtime_state_pending_payload_aad_version_unsupported"
            }
            "pending inbound encrypted payload contains sensitive markers" => {
                "runtime_state_pending_payload_sensitive"
            }
            "pending inbound encrypted payload nonce is invalid base64" => {
                "runtime_state_pending_payload_nonce_invalid"
            }
            "pending inbound encrypted payload nonce length is invalid" => {
                "runtime_state_pending_payload_nonce_length_invalid"
            }
            "pending inbound encrypted payload ciphertext is invalid base64" => {
                "runtime_state_pending_payload_ciphertext_invalid"
            }
            "pending inbound encrypted payload ciphertext is empty" => {
                "runtime_state_pending_payload_ciphertext_empty"
            }
            "legacy pending inbound lacks encrypted payload" => {
                "runtime_state_legacy_payload_missing"
            }
            "pending inbound terminal reason is unsafe" => {
                "runtime_state_pending_terminal_reason_unsafe"
            }
            "state record failed validation" => "runtime_state_record_invalid",
            _ => "runtime_state_invalid_record",
        },
        WeixinStateError::InjectedFailure { .. } => "runtime_state_injected_failure",
        WeixinStateError::StateNotFound { .. } => "runtime_state_not_found",
        WeixinStateError::LockActive { .. } => "runtime_state_lock_active",
        WeixinStateError::LockAlreadyExists => "runtime_state_lock_exists",
        WeixinStateError::PairRequestNotFound { .. } => "runtime_state_pair_request_not_found",
        WeixinStateError::PairRequestAccountMismatch => {
            "runtime_state_pair_request_account_mismatch"
        }
        WeixinStateError::PairRequestConsumed { .. } => "runtime_state_pair_request_consumed",
        WeixinStateError::PairRequestExpired { .. } => "runtime_state_pair_request_expired",
        WeixinStateError::InvalidTransition { .. } => "runtime_state_invalid_transition",
        WeixinStateError::PendingInboundQueueFull { .. } => "runtime_state_pending_queue_full",
        WeixinStateError::PendingInboundNotFound { .. } => "runtime_state_pending_not_found",
        WeixinStateError::DeliveryNotFound { .. } => "runtime_state_delivery_not_found",
        WeixinStateError::RemoteControlRequestNotFound { .. } => {
            "runtime_state_remote_control_not_found"
        }
    }
}
fn cursor_for(snapshot: &WeixinStateSnapshot, source: &str) -> Option<String> {
    snapshot
        .cursors
        .iter()
        .find(|cursor| cursor.source == source)
        .and_then(|cursor| cursor.get_updates_buf.clone())
}

fn peer_is_approved(snapshot: &WeixinStateSnapshot, peer_id_hash: &str, _now_millis: u64) -> bool {
    snapshot.pair_requests.iter().any(|request| {
        request.peer_id_hash == peer_id_hash && request.state == WeixinPairRequestState::Approved
    })
}

fn active_pending_pair_request<'a>(
    snapshot: &'a WeixinStateSnapshot,
    peer_id_hash: &str,
    now_millis: u64,
) -> Option<&'a WeixinPairRequest> {
    snapshot.pair_requests.iter().find(|request| {
        request.peer_id_hash == peer_id_hash
            && request.state == WeixinPairRequestState::Pending
            && request.expires_at_millis > now_millis
    })
}

fn render_pairing_prompt(
    request: &WeixinPairRequest,
    existing_pair_request: bool,
    now_millis: u64,
) -> String {
    let prefix = if existing_pair_request {
        "你已有待批准的 YunXi 微信配对请求。"
    } else {
        "需要先完成 YunXi 微信配对。"
    };
    let remaining_minutes = request
        .expires_at_millis
        .saturating_sub(now_millis)
        .saturating_add(59_999)
        / 60_000;
    format!(
        "{prefix}\n配对请求: {request_id}\n请在本机运行: yunxi weixin pair approve {request_id}\n约 {remaining_minutes} 分钟后过期。",
        request_id = request.request_id
    )
}

fn remote_command_from_message(message: &WeixinMessage) -> Option<crate::WeixinRemoteCommand> {
    message
        .item_list
        .iter()
        .find_map(|item| item.text_item.as_ref())
        .and_then(|text| parse_weixin_remote_command(text.text.expose()))
}

fn is_credential_expired(error: &WeixinApiError) -> bool {
    matches!(
        error,
        WeixinApiError::Api {
            code: -14 | 401 | 42001,
            ..
        } | WeixinApiError::HttpStatus {
            status: 401 | 403,
            ..
        }
    )
}

fn redacted_error_label(error: &WeixinApiError) -> &'static str {
    match error {
        WeixinApiError::Timeout { .. } => "poll_timeout",
        WeixinApiError::Network { .. } => "poll_network_error",
        WeixinApiError::HttpStatus { .. } => "poll_http_error",
        WeixinApiError::ResponseTooLarge { .. } => "poll_response_too_large",
        WeixinApiError::Api { .. } => "poll_api_error",
        WeixinApiError::InvalidJson { .. } => "poll_invalid_json",
        WeixinApiError::Protocol { .. } => "poll_protocol_error",
    }
}

async fn sleep_cancelable(delay: Duration, cancellation: &WeixinServeCancellation) {
    if delay.is_zero() {
        return;
    }
    let mut remaining = delay;
    while !remaining.is_zero() && !cancellation.is_cancelled() {
        let step = remaining.min(Duration::from_millis(50));
        sleep(step).await;
        remaining = remaining.saturating_sub(step);
    }
}

fn now_millis_u64() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn runtime_lease_owner_label() -> String {
    format!("lease#{}-{}", std::process::id(), now_millis_u64())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WeixinMessageId;
    use crate::WeixinRemoteControlHub;
    use crate::ilink::{
        MessageItem, TextItem, WeixinMessage, WeixinUpdate, WeixinUpdateMessage, WeixinUpdateSender,
    };
    use crate::turn_supervisor::{
        WeixinRuntimeSink, WeixinRuntimeTestSink, WeixinTurnReport, WeixinTurnSupervisorError,
    };
    use std::collections::VecDeque;
    use std::sync::Mutex;
    use tempfile::TempDir;
    use yunxi_agent_core::AgentRunStatus;
    use yunxi_agent_storage::{
        WEIXIN_STATE_SCHEMA_VERSION, WeixinConnectionStateRecord, WeixinCredentialReferenceRecord,
    };

    struct ScriptedTransport {
        responses: VecDeque<Result<GetUpdatesResponse, WeixinApiError>>,
    }

    #[async_trait]
    impl WeixinUpdatesTransport for ScriptedTransport {
        async fn get_updates(
            &mut self,
            _request: GetUpdatesRequest,
        ) -> Result<GetUpdatesResponse, WeixinApiError> {
            self.responses
                .pop_front()
                .unwrap_or_else(|| Ok(GetUpdatesResponse::default()))
        }
    }

    #[derive(Clone, Default)]
    struct RecordingDispatcher {
        item_ids: Arc<Mutex<Vec<String>>>,
        store: Option<FileWeixinStateStore>,
    }

    impl RecordingDispatcher {
        fn with_store(store: FileWeixinStateStore) -> Self {
            Self {
                item_ids: Arc::new(Mutex::new(Vec::new())),
                store: Some(store),
            }
        }

        fn item_ids(&self) -> Vec<String> {
            self.item_ids.lock().expect("item ids").clone()
        }
    }

    fn begin_pending_for_test(
        store: &FileWeixinStateStore,
        account_id: &str,
        item_id: &str,
    ) -> String {
        let state = store.load(account_id).expect("load state").expect("state");
        let pending = state
            .pending_inbound
            .iter()
            .find(|pending| pending.item_id == item_id)
            .expect("pending")
            .clone();
        let session_id = format!("yunxi-weixin-test-{}", item_id.replace('#', "-"));
        let binding = store
            .begin_pending_runtime_turn(yunxi_agent_storage::WeixinRuntimeTurnBeginRequest {
                account_id: account_id.to_string(),
                peer_id_hash: pending.peer_id_hash,
                message_id_hash: pending.message_id_hash,
                item_id: item_id.to_string(),
                direct_message_key: pending.direct_message_key,
                workspace_id: state.workspace_id,
                candidate_session_id: session_id,
                source_label: "weixin-private-chat".to_string(),
                now_millis: now_millis_u64(),
            })
            .expect("begin pending runtime turn");
        binding.session_id
    }

    fn complete_pending_for_test(
        store: &FileWeixinStateStore,
        account_id: &str,
        item_id: &str,
    ) -> String {
        let session_id = begin_pending_for_test(store, account_id, item_id);
        store
            .complete_pending_runtime_turn(
                account_id,
                item_id,
                yunxi_agent_storage::WeixinPendingInboundState::Succeeded,
                None,
                now_millis_u64(),
            )
            .expect("complete pending");
        session_id
    }

    #[async_trait]
    impl WeixinRuntimeDispatcher for RecordingDispatcher {
        async fn dispatch_pending_turn(
            &self,
            account_id: &str,
            item_id: &str,
        ) -> Result<WeixinTurnReport, WeixinTurnSupervisorError> {
            self.item_ids
                .lock()
                .expect("item ids")
                .push(item_id.to_string());
            let session_id = self
                .store
                .as_ref()
                .map(|store| complete_pending_for_test(store, account_id, item_id))
                .unwrap_or_else(|| "yunxi-weixin-test".to_string());
            Ok(WeixinTurnReport {
                account_id: account_id.to_string(),
                peer_id_hash: "peer#00000000".to_string(),
                direct_message_key: "dm#00000000".to_string(),
                item_id: item_id.to_string(),
                session_id,
                parent_session_id: None,
                status: AgentRunStatus::Completed,
                final_response_present: true,
                stream_observation: None,
            })
        }
    }

    #[derive(Clone)]
    struct QueueThenCompleteDispatcher {
        store: FileWeixinStateStore,
        calls: Arc<Mutex<Vec<String>>>,
        fail_remaining: Arc<Mutex<usize>>,
    }

    impl QueueThenCompleteDispatcher {
        fn new(store: FileWeixinStateStore, fail_count: usize) -> Self {
            Self {
                store,
                calls: Arc::new(Mutex::new(Vec::new())),
                fail_remaining: Arc::new(Mutex::new(fail_count)),
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().expect("calls").clone()
        }
    }

    #[async_trait]
    impl WeixinRuntimeDispatcher for QueueThenCompleteDispatcher {
        async fn dispatch_pending_turn(
            &self,
            account_id: &str,
            item_id: &str,
        ) -> Result<WeixinTurnReport, WeixinTurnSupervisorError> {
            self.calls.lock().expect("calls").push(item_id.to_string());
            let mut remaining = self.fail_remaining.lock().expect("fail remaining");
            if *remaining > 0 {
                *remaining -= 1;
                return Err(WeixinTurnSupervisorError::QueueFull {
                    scope: "conversation",
                    limit: 0,
                });
            }
            drop(remaining);
            let session_id = complete_pending_for_test(&self.store, account_id, item_id);
            Ok(WeixinTurnReport {
                account_id: account_id.to_string(),
                peer_id_hash: "peer#00000000".to_string(),
                direct_message_key: "dm#00000000".to_string(),
                item_id: item_id.to_string(),
                session_id,
                parent_session_id: None,
                status: AgentRunStatus::Completed,
                final_response_present: true,
                stream_observation: None,
            })
        }
    }

    #[derive(Clone)]
    struct DelayedCompleteDispatcher {
        store: FileWeixinStateStore,
        delay: Duration,
        calls: Arc<Mutex<Vec<String>>>,
    }

    impl DelayedCompleteDispatcher {
        fn new(store: FileWeixinStateStore, delay: Duration) -> Self {
            Self {
                store,
                delay,
                calls: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().expect("calls").clone()
        }
    }

    #[async_trait]
    impl WeixinRuntimeDispatcher for DelayedCompleteDispatcher {
        async fn dispatch_pending_turn(
            &self,
            account_id: &str,
            item_id: &str,
        ) -> Result<WeixinTurnReport, WeixinTurnSupervisorError> {
            self.calls.lock().expect("calls").push(item_id.to_string());
            sleep(self.delay).await;
            let session_id = complete_pending_for_test(&self.store, account_id, item_id);
            Ok(WeixinTurnReport {
                account_id: account_id.to_string(),
                peer_id_hash: "peer#00000000".to_string(),
                direct_message_key: "dm#00000000".to_string(),
                item_id: item_id.to_string(),
                session_id,
                parent_session_id: None,
                status: AgentRunStatus::Completed,
                final_response_present: true,
                stream_observation: None,
            })
        }
    }

    #[derive(Clone)]
    struct HoldingDispatcher {
        store: FileWeixinStateStore,
        started: Arc<tokio::sync::Notify>,
    }

    #[async_trait]
    impl WeixinRuntimeDispatcher for HoldingDispatcher {
        async fn dispatch_pending_turn(
            &self,
            account_id: &str,
            item_id: &str,
        ) -> Result<WeixinTurnReport, WeixinTurnSupervisorError> {
            let _session_id = begin_pending_for_test(&self.store, account_id, item_id);
            self.started.notify_waiters();
            std::future::pending::<Result<WeixinTurnReport, WeixinTurnSupervisorError>>().await
        }
    }

    #[derive(Clone, Default)]
    struct PanicDispatcher;

    #[async_trait]
    impl WeixinRuntimeDispatcher for PanicDispatcher {
        async fn dispatch_pending_turn(
            &self,
            _account_id: &str,
            _item_id: &str,
        ) -> Result<WeixinTurnReport, WeixinTurnSupervisorError> {
            panic!("intentional background dispatch panic")
        }
    }

    fn store_fixture() -> (TempDir, FileWeixinStateStore, String) {
        let temp = TempDir::new().expect("temp");
        let account = "account#933b5bde".to_string();
        let mut snapshot = WeixinStateSnapshot::new(
            &account,
            "workspace#73521066",
            "https://ilinkai.weixin.qq.com/",
            1000,
        );
        snapshot.connection_state = WeixinConnectionStateRecord::Ready;
        snapshot.credential = Some(WeixinCredentialReferenceRecord {
            backend: "fake".to_string(),
            token_target: "token-target".to_string(),
            data_key_target: "key-target".to_string(),
        });
        let store = FileWeixinStateStore::for_workspace(temp.path());
        store.save(&snapshot).expect("save state");
        (temp, store, account)
    }

    fn text_message(raw_message_id: &str, raw_peer: &str) -> WeixinMessage {
        WeixinMessage {
            message_id: WeixinMessageId::new(raw_message_id),
            from_user_id: SecretString::new(raw_peer),
            to_user_id: None,
            client_id: None,
            create_time_ms: Some(42),
            session_id: None,
            group_id: None,
            message_type: Some(1),
            message_state: None,
            item_list: vec![MessageItem {
                item_type: 1,
                text_item: Some(TextItem {
                    text: SecretString::new("raw message body"),
                }),
                is_completed: Some(true),
                msg_id: None,
            }],
            context_token: Some(SecretString::new("context-token-secret")),
        }
    }

    fn approve_peer_for_message(
        store: &FileWeixinStateStore,
        account: &str,
        message: &WeixinMessage,
    ) {
        let envelope = WeixinInboundEnvelope::from_message(account, None, message, 1000);
        let pair = store
            .add_pair_request(account, &envelope.peer_id_hash, u64::MAX, 1000)
            .expect("pair request");
        store
            .approve_pair_request(account, &pair.request_id, 1001)
            .expect("approve peer");
    }

    fn test_data_key() -> SecretString {
        SecretString::new("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
    }

    #[tokio::test]
    async fn serve_loop_accepts_approved_private_text_and_advances_cursor_atomically() {
        let (_temp, store, account) = store_fixture();
        let envelope = WeixinInboundEnvelope::from_message(
            &account,
            None,
            &text_message("raw-message-1", "raw-peer-1"),
            1000,
        );
        let pair = store
            .add_pair_request(&account, &envelope.peer_id_hash, u64::MAX, 1000)
            .expect("pair request");
        store
            .approve_pair_request(&account, &pair.request_id, 1001)
            .expect("approve peer");
        let mut transport = ScriptedTransport {
            responses: VecDeque::from([Ok(GetUpdatesResponse {
                ret: 0,
                msgs: vec![text_message("raw-message-1", "raw-peer-1")],
                get_updates_buf: Some(SecretString::new("cursor-next")),
                longpolling_timeout_ms: Some(1),
                ..GetUpdatesResponse::default()
            })]),
        };
        let mut options = WeixinServeOptions::new(account.clone());
        options.max_polls = Some(1);
        let data_key = test_data_key();
        let report = run_weixin_serve_loop(
            &mut transport,
            &store,
            options,
            &data_key,
            &WeixinServeCancellation::default(),
        )
        .await
        .expect("serve loop");
        assert_eq!(report.accepted_count, 1);
        assert_eq!(report.pair_request_count, 0);
        let state = store.load(&account).expect("load").expect("state");
        assert_eq!(state.pending_inbound_count(), 1);
        let pending = state.pending_inbound[0].clone();
        let encrypted_payload = pending
            .encrypted_payload
            .as_ref()
            .expect("encrypted payload");
        assert_eq!(encrypted_payload.algorithm, "chacha20-poly1305");
        assert_eq!(encrypted_payload.algorithm_version, 1);
        assert_eq!(encrypted_payload.aad_version, 1);
        assert!(!encrypted_payload.nonce.is_empty());
        assert!(!encrypted_payload.ciphertext.is_empty());
        assert_eq!(pending.payload_kind.as_deref(), Some("text"));
        let recovered = WeixinPayloadCipher::new()
            .decrypt_pending_inbound(&data_key, &pending)
            .expect("restart decrypt pending inbound");
        assert_eq!(
            recovered.text.as_ref().expect("text").expose(),
            "raw message body"
        );
        assert_eq!(
            state.cursors[0].get_updates_buf.as_deref(),
            Some("cursor-next")
        );
        let state_json =
            std::fs::read_to_string(store.state_path_for(&account)).expect("state json");
        for forbidden in [
            "raw-message-1",
            "raw-peer-1",
            "raw message body",
            "context-token-secret",
            data_key.expose(),
        ] {
            assert!(!state_json.contains(forbidden));
        }
    }

    #[tokio::test]
    async fn serve_loop_treats_approved_pair_as_persistent_allowlist() {
        let (_temp, store, account) = store_fixture();
        let message = text_message("raw-message-approved-expired", "raw-peer-approved-expired");
        let envelope = WeixinInboundEnvelope::from_message(&account, None, &message, 1000);
        let pair = store
            .add_pair_request(&account, &envelope.peer_id_hash, 1100, 1000)
            .expect("pair request");
        store
            .approve_pair_request(&account, &pair.request_id, 1050)
            .expect("approve before request expiry");
        let mut transport = ScriptedTransport {
            responses: VecDeque::from([Ok(GetUpdatesResponse {
                ret: 0,
                msgs: vec![message],
                get_updates_buf: Some(SecretString::new("cursor-persistent-allowlist")),
                ..GetUpdatesResponse::default()
            })]),
        };
        let dispatcher = RecordingDispatcher::with_store(store.clone());
        let mut options = WeixinServeOptions::new(account.clone());
        options.max_polls = Some(1);
        options.runtime_dispatcher = Some(Arc::new(dispatcher.clone()));

        let report = run_weixin_serve_loop(
            &mut transport,
            &store,
            options,
            &test_data_key(),
            &WeixinServeCancellation::default(),
        )
        .await
        .expect("serve loop");

        assert_eq!(report.accepted_count, 1);
        assert_eq!(report.pair_request_count, 0);
        assert_eq!(report.runtime_dispatch_count, 1);
        assert_eq!(dispatcher.item_ids().len(), 1);
    }

    #[tokio::test]
    async fn serve_loop_accepts_approved_update_message_payload() {
        let (_temp, store, account) = store_fixture();
        let approval_message = text_message("raw-update-approval", "raw-update-peer");
        approve_peer_for_message(&store, &account, &approval_message);
        let mut transport = ScriptedTransport {
            responses: VecDeque::from([Ok(GetUpdatesResponse {
                ret: 0,
                updates: vec![WeixinUpdate {
                    update_id: Some(99),
                    update_type: Some("message".to_string()),
                    message: Some(WeixinUpdateMessage {
                        message_id: Some(WeixinMessageId::new("raw-update-message")),
                        chat_id: Some(SecretString::new("raw-update-chat")),
                        chat_type: Some("private".to_string()),
                        from: Some(WeixinUpdateSender {
                            user_id: SecretString::new("raw-update-peer"),
                            user_name: Some(SecretString::new("raw-update-user")),
                        }),
                        text: Some(SecretString::new("update message body")),
                        timestamp: Some(1_785_500_000),
                    }),
                }],
                get_updates_buf: Some(SecretString::new("cursor-update")),
                ..GetUpdatesResponse::default()
            })]),
        };
        let mut options = WeixinServeOptions::new(account.clone());
        options.max_polls = Some(1);
        let dispatcher = RecordingDispatcher::with_store(store.clone());
        options.runtime_dispatcher = Some(Arc::new(dispatcher.clone()));
        let data_key = test_data_key();
        let report = run_weixin_serve_loop(
            &mut transport,
            &store,
            options,
            &data_key,
            &WeixinServeCancellation::default(),
        )
        .await
        .expect("serve loop");

        assert_eq!(report.accepted_count, 1);
        assert_eq!(report.runtime_dispatch_count, 1);
        assert_eq!(dispatcher.item_ids().len(), 1);
        let state = store.load(&account).expect("load").expect("state");
        assert_eq!(
            state.cursors[0].get_updates_buf.as_deref(),
            Some("cursor-update")
        );
        let recovered = WeixinPayloadCipher::new()
            .decrypt_pending_inbound(&data_key, &state.pending_inbound[0])
            .expect("decrypt update pending");
        assert_eq!(
            recovered.text.as_ref().expect("text").expose(),
            "update message body"
        );
    }

    #[tokio::test]
    async fn serve_loop_mixed_batch_counts_pair_skip_and_encrypted_pending() {
        let (_temp, store, account) = store_fixture();
        let approved_envelope = WeixinInboundEnvelope::from_message(
            &account,
            None,
            &text_message("raw-message-approved", "raw-peer-approved"),
            1000,
        );
        let pair = store
            .add_pair_request(&account, &approved_envelope.peer_id_hash, u64::MAX, 1000)
            .expect("pair request");
        store
            .approve_pair_request(&account, &pair.request_id, 1001)
            .expect("approve peer");

        let mut group = text_message("raw-message-group", "raw-peer-group");
        group.group_id = Some(SecretString::new("raw-group-id"));
        let mut self_message = text_message("raw-message-self", "bot-user-id");
        self_message.message_state = Some(2);
        let mut attachment = text_message("raw-message-attachment", "raw-peer-attachment");
        attachment.item_list[0].item_type = 3;
        let mut unknown = text_message("raw-message-unknown", "raw-peer-unknown");
        unknown.item_list.clear();

        let mut transport = ScriptedTransport {
            responses: VecDeque::from([Ok(GetUpdatesResponse {
                ret: 0,
                msgs: vec![
                    text_message("raw-message-approved", "raw-peer-approved"),
                    text_message("raw-message-stranger", "raw-peer-stranger"),
                    group,
                    self_message,
                    attachment,
                    unknown,
                ],
                get_updates_buf: Some(SecretString::new("cursor-next")),
                ..GetUpdatesResponse::default()
            })]),
        };
        let mut options = WeixinServeOptions::new(account.clone());
        options.max_polls = Some(1);
        options.self_user_id = Some(SecretString::new("bot-user-id"));
        let dispatcher = RecordingDispatcher::with_store(store.clone());
        options.runtime_dispatcher = Some(Arc::new(dispatcher.clone()));
        let data_key = test_data_key();
        let report = run_weixin_serve_loop(
            &mut transport,
            &store,
            options,
            &data_key,
            &WeixinServeCancellation::default(),
        )
        .await
        .expect("serve loop");
        assert_eq!(report.accepted_count, 1);
        assert_eq!(report.pair_request_count, 1);
        assert_eq!(report.skipped_group_count, 1);
        assert_eq!(report.skipped_self_count, 1);
        assert_eq!(report.skipped_unsupported_count, 1);
        assert_eq!(report.skipped_unknown_count, 1);
        assert_eq!(report.runtime_dispatch_count, 1);
        assert_eq!(report.runtime_error_count, 0);
        assert_eq!(dispatcher.item_ids().len(), 1);
        let state = store.load(&account).expect("load").expect("state");
        assert_eq!(state.pending_inbound_count(), 0);
        assert_eq!(state.pair_request_count(), 1);
        assert_eq!(
            state.cursors[0].get_updates_buf.as_deref(),
            Some("cursor-next")
        );
        let pending = state.pending_inbound[0].clone();
        assert_eq!(
            pending.state,
            yunxi_agent_storage::WeixinPendingInboundState::Succeeded
        );
        let recovered = WeixinPayloadCipher::new()
            .decrypt_pending_inbound(&data_key, &pending)
            .expect("decrypt pending");
        assert_eq!(
            recovered.text.as_ref().expect("text").expose(),
            "raw message body"
        );
    }

    #[tokio::test]
    async fn serve_loop_generates_pair_request_for_stranger_without_pending_inbound() {
        let (_temp, store, account) = store_fixture();
        let mut transport = ScriptedTransport {
            responses: VecDeque::from([Ok(GetUpdatesResponse {
                ret: 0,
                msgs: vec![text_message("raw-message-1", "raw-peer-1")],
                get_updates_buf: Some(SecretString::new("cursor-next")),
                ..GetUpdatesResponse::default()
            })]),
        };
        let mut options = WeixinServeOptions::new(account.clone());
        options.max_polls = Some(1);
        let data_key = test_data_key();
        let report = run_weixin_serve_loop(
            &mut transport,
            &store,
            options,
            &data_key,
            &WeixinServeCancellation::default(),
        )
        .await
        .expect("serve loop");
        assert_eq!(report.accepted_count, 0);
        assert_eq!(report.pair_request_count, 1);
        let state = store.load(&account).expect("load").expect("state");
        assert_eq!(state.pending_inbound_count(), 0);
        assert_eq!(state.pair_request_count(), 1);
        assert_eq!(
            state.cursors[0].get_updates_buf.as_deref(),
            Some("cursor-next")
        );
    }

    #[tokio::test]
    async fn serve_loop_sends_pairing_prompt_for_stranger_private_text() {
        let (_temp, store, account) = store_fixture();
        let mut transport = ScriptedTransport {
            responses: VecDeque::from([Ok(GetUpdatesResponse {
                ret: 0,
                msgs: vec![text_message("raw-message-1", "raw-peer-1")],
                get_updates_buf: Some(SecretString::new("cursor-next")),
                ..GetUpdatesResponse::default()
            })]),
        };
        let sink = WeixinRuntimeTestSink::default();
        let sink_handle: Arc<dyn WeixinRuntimeSink> = Arc::new(sink.clone());
        let mut options = WeixinServeOptions::new(account.clone());
        options.max_polls = Some(1);
        options.outbound_sink = Some(sink_handle);
        let data_key = test_data_key();
        let report = run_weixin_serve_loop(
            &mut transport,
            &store,
            options,
            &data_key,
            &WeixinServeCancellation::default(),
        )
        .await
        .expect("serve loop");

        assert_eq!(report.accepted_count, 0);
        assert_eq!(report.pair_request_count, 1);
        assert_eq!(report.pair_prompt_count, 1);
        assert_eq!(report.pair_prompt_error_count, 0);
        let records = sink.records();
        assert_eq!(records.len(), 1);
        assert!(
            records[0]
                .final_response
                .contains("需要先完成 YunXi 微信配对")
        );
        assert!(
            records[0]
                .final_response
                .contains("yunxi weixin pair approve pair-")
        );
        assert_eq!(
            records[0]
                .reply_to_user_id
                .as_ref()
                .map(SecretString::expose),
            Some("raw-peer-1")
        );
        let state = store.load(&account).expect("load").expect("state");
        assert_eq!(state.pending_inbound_count(), 0);
        assert_eq!(state.pair_request_count(), 1);
    }

    #[tokio::test]
    async fn serve_loop_does_not_route_stranger_slash_command_before_pairing() {
        let (_temp, store, account) = store_fixture();
        let mut message = text_message("raw-message-remote-status", "raw-peer-stranger");
        message.item_list[0].text_item = Some(TextItem {
            text: SecretString::new("/status"),
        });
        let mut transport = ScriptedTransport {
            responses: VecDeque::from([Ok(GetUpdatesResponse {
                ret: 0,
                msgs: vec![message],
                get_updates_buf: Some(SecretString::new("cursor-next")),
                ..GetUpdatesResponse::default()
            })]),
        };
        let mut options = WeixinServeOptions::new(account.clone());
        options.max_polls = Some(1);
        options.remote_control_hub = Some(WeixinRemoteControlHub::default());
        let data_key = test_data_key();
        let report = run_weixin_serve_loop(
            &mut transport,
            &store,
            options,
            &data_key,
            &WeixinServeCancellation::default(),
        )
        .await
        .expect("serve loop");
        assert_eq!(report.accepted_count, 0);
        assert_eq!(report.pair_request_count, 1);
        assert_eq!(report.remote_control_count, 0);
        assert_eq!(report.remote_control_error_count, 0);
        let state = store.load(&account).expect("load").expect("state");
        assert_eq!(state.pending_inbound_count(), 0);
        assert_eq!(state.pair_request_count(), 1);
    }

    #[tokio::test]
    async fn serve_loop_routes_status_command_into_outbound_sink() {
        let (_temp, store, account) = store_fixture();
        let mut message = text_message("raw-message-status", "raw-peer-status");
        message.item_list[0].text_item = Some(TextItem {
            text: SecretString::new("/status"),
        });
        let mut transport = ScriptedTransport {
            responses: VecDeque::from([Ok(GetUpdatesResponse {
                ret: 0,
                msgs: vec![message],
                get_updates_buf: Some(SecretString::new("cursor-next")),
                ..GetUpdatesResponse::default()
            })]),
        };
        let sink = WeixinRuntimeTestSink::default();
        let sink_handle: Arc<dyn WeixinRuntimeSink> = Arc::new(sink.clone());
        let mut options = WeixinServeOptions::new(account.clone());
        options.max_polls = Some(1);
        options.pairing_required = false;
        options.remote_control_hub = Some(WeixinRemoteControlHub::default());
        options.outbound_sink = Some(sink_handle);
        let data_key = test_data_key();
        let report = run_weixin_serve_loop(
            &mut transport,
            &store,
            options,
            &data_key,
            &WeixinServeCancellation::default(),
        )
        .await
        .expect("serve loop");
        assert_eq!(report.remote_control_count, 1);
        assert_eq!(report.remote_control_error_count, 0);
        let records = sink.records();
        assert_eq!(records.len(), 1);
        let text = &records[0].final_response;
        assert!(text.contains("[YunXi]"));
        assert!(text.contains("当前有 0 个待处理控制请求"));
        assert!(!text.contains("status=ok"));
        assert!(!text.contains("pending_control_requests=0"));
        assert!(!text.contains("account=account#"));
    }

    #[tokio::test]
    async fn serve_loop_is_idempotent_for_repeated_approved_message() {
        let (_temp, store, account) = store_fixture();
        let envelope = WeixinInboundEnvelope::from_message(
            &account,
            None,
            &text_message("raw-message-1", "raw-peer-1"),
            1000,
        );
        let pair = store
            .add_pair_request(&account, &envelope.peer_id_hash, u64::MAX, 1000)
            .expect("pair request");
        store
            .approve_pair_request(&account, &pair.request_id, 1001)
            .expect("approve peer");
        let mut transport = ScriptedTransport {
            responses: VecDeque::from([
                Ok(GetUpdatesResponse {
                    ret: 0,
                    msgs: vec![text_message("raw-message-1", "raw-peer-1")],
                    get_updates_buf: Some(SecretString::new("cursor-1")),
                    ..GetUpdatesResponse::default()
                }),
                Ok(GetUpdatesResponse {
                    ret: 0,
                    msgs: vec![text_message("raw-message-1", "raw-peer-1")],
                    get_updates_buf: Some(SecretString::new("cursor-2")),
                    ..GetUpdatesResponse::default()
                }),
            ]),
        };
        let mut options = WeixinServeOptions::new(account.clone());
        options.max_polls = Some(2);
        let dispatcher = RecordingDispatcher::with_store(store.clone());
        options.runtime_dispatcher = Some(Arc::new(dispatcher.clone()));
        let data_key = test_data_key();
        let report = run_weixin_serve_loop(
            &mut transport,
            &store,
            options,
            &data_key,
            &WeixinServeCancellation::default(),
        )
        .await
        .expect("serve loop");
        assert_eq!(report.accepted_count, 1);
        assert_eq!(report.duplicate_count, 1);
        assert_eq!(report.runtime_dispatch_count, 1);
        assert_eq!(dispatcher.item_ids().len(), 1);
        let state = store.load(&account).expect("load").expect("state");
        assert_eq!(state.pending_inbound.len(), 1);
        assert_eq!(
            state.cursors[0].get_updates_buf.as_deref(),
            Some("cursor-2")
        );
    }

    #[tokio::test]
    async fn serve_loop_drains_ready_pending_after_queue_full_capacity_recovers() {
        let (_temp, store, account) = store_fixture();
        let envelope = WeixinInboundEnvelope::from_message(
            &account,
            None,
            &text_message("raw-message-1", "raw-peer-1"),
            1000,
        );
        let pair = store
            .add_pair_request(&account, &envelope.peer_id_hash, u64::MAX, 1000)
            .expect("pair request");
        store
            .approve_pair_request(&account, &pair.request_id, 1001)
            .expect("approve peer");
        let mut transport = ScriptedTransport {
            responses: VecDeque::from([
                Ok(GetUpdatesResponse {
                    ret: 0,
                    msgs: vec![text_message("raw-message-1", "raw-peer-1")],
                    get_updates_buf: Some(SecretString::new("cursor-1")),
                    ..GetUpdatesResponse::default()
                }),
                Ok(GetUpdatesResponse {
                    ret: 0,
                    msgs: Vec::new(),
                    get_updates_buf: Some(SecretString::new("cursor-2")),
                    longpolling_timeout_ms: Some(300),
                    ..GetUpdatesResponse::default()
                }),
                Ok(GetUpdatesResponse {
                    ret: 0,
                    msgs: Vec::new(),
                    get_updates_buf: Some(SecretString::new("cursor-3")),
                    ..GetUpdatesResponse::default()
                }),
            ]),
        };
        let mut options = WeixinServeOptions::new(account.clone());
        options.max_polls = Some(3);
        let dispatcher = QueueThenCompleteDispatcher::new(store.clone(), 1);
        options.runtime_dispatcher = Some(Arc::new(dispatcher.clone()));
        let report = run_weixin_serve_loop(
            &mut transport,
            &store,
            options,
            &test_data_key(),
            &WeixinServeCancellation::default(),
        )
        .await
        .expect("serve loop");
        assert_eq!(report.accepted_count, 1);
        assert_eq!(report.runtime_error_count, 1);
        assert_eq!(report.runtime_deferred_count, 1);
        assert_eq!(report.runtime_dispatch_count, 1);
        assert_eq!(dispatcher.calls().len(), 2);
        let state = store.load(&account).expect("load").expect("state");
        assert_eq!(state.pending_inbound_count(), 0);
        assert_eq!(
            state.pending_inbound[0].state,
            yunxi_agent_storage::WeixinPendingInboundState::Succeeded
        );
        assert_eq!(
            state.cursors[0].get_updates_buf.as_deref(),
            Some("cursor-3")
        );
    }

    #[tokio::test]
    async fn serve_loop_drains_ready_pending_after_restart_without_duplicate_runtime() {
        let (_temp, store, account) = store_fixture();
        let envelope = WeixinInboundEnvelope::from_message(
            &account,
            None,
            &text_message("raw-message-1", "raw-peer-1"),
            1000,
        );
        let pair = store
            .add_pair_request(&account, &envelope.peer_id_hash, u64::MAX, 1000)
            .expect("pair request");
        store
            .approve_pair_request(&account, &pair.request_id, 1001)
            .expect("approve peer");

        let mut first_transport = ScriptedTransport {
            responses: VecDeque::from([Ok(GetUpdatesResponse {
                ret: 0,
                msgs: vec![text_message("raw-message-1", "raw-peer-1")],
                get_updates_buf: Some(SecretString::new("cursor-1")),
                ..GetUpdatesResponse::default()
            })]),
        };
        let first_dispatcher = QueueThenCompleteDispatcher::new(store.clone(), 1);
        let mut first_options = WeixinServeOptions::new(account.clone());
        first_options.max_polls = Some(1);
        first_options.runtime_dispatcher = Some(Arc::new(first_dispatcher.clone()));
        let first_report = run_weixin_serve_loop(
            &mut first_transport,
            &store,
            first_options,
            &test_data_key(),
            &WeixinServeCancellation::default(),
        )
        .await
        .expect("first serve loop");
        assert_eq!(first_report.runtime_deferred_count, 1);
        assert_eq!(first_dispatcher.calls().len(), 1);
        let pending_after_queue_full = store
            .load(&account)
            .expect("load")
            .expect("state")
            .pending_inbound[0]
            .clone();
        assert_eq!(
            pending_after_queue_full.state,
            yunxi_agent_storage::WeixinPendingInboundState::Ready
        );
        assert_eq!(pending_after_queue_full.dispatch_retry_count, 1);
        assert_eq!(
            pending_after_queue_full.last_dispatch_error.as_deref(),
            Some("runtime_queue_full")
        );

        tokio::time::sleep(MIN_READY_PENDING_RETRY_DELAY).await;

        let mut second_transport = ScriptedTransport {
            responses: VecDeque::from([Ok(GetUpdatesResponse {
                ret: 0,
                msgs: Vec::new(),
                get_updates_buf: Some(SecretString::new("cursor-2")),
                ..GetUpdatesResponse::default()
            })]),
        };
        let second_dispatcher = QueueThenCompleteDispatcher::new(store.clone(), 0);
        let mut second_options = WeixinServeOptions::new(account.clone());
        second_options.max_polls = Some(1);
        second_options.runtime_dispatcher = Some(Arc::new(second_dispatcher.clone()));
        let second_report = run_weixin_serve_loop(
            &mut second_transport,
            &store,
            second_options,
            &test_data_key(),
            &WeixinServeCancellation::default(),
        )
        .await
        .expect("second serve loop");
        assert_eq!(second_report.runtime_dispatch_count, 1);
        assert_eq!(second_report.runtime_error_count, 0);
        assert_eq!(
            second_dispatcher.calls(),
            vec![pending_after_queue_full.item_id]
        );
        let state = store.load(&account).expect("load").expect("state");
        assert_eq!(state.pending_inbound_count(), 0);
    }

    #[tokio::test]
    async fn serve_loop_background_dispatch_waits_for_completion_on_shutdown() {
        let (_temp, store, account) = store_fixture();
        let message = text_message("raw-message-background-wait", "raw-peer-background-wait");
        approve_peer_for_message(&store, &account, &message);
        let mut transport = ScriptedTransport {
            responses: VecDeque::from([Ok(GetUpdatesResponse {
                ret: 0,
                msgs: vec![message],
                get_updates_buf: Some(SecretString::new("cursor-background-wait")),
                ..GetUpdatesResponse::default()
            })]),
        };
        let dispatcher = DelayedCompleteDispatcher::new(store.clone(), Duration::from_millis(50));
        let mut options = WeixinServeOptions::new(account.clone());
        options.max_polls = Some(1);
        options.background_runtime_dispatch = true;
        options.runtime_dispatch_shutdown_wait = Duration::from_millis(500);
        options.runtime_dispatcher = Some(Arc::new(dispatcher.clone()));

        let report = run_weixin_serve_loop(
            &mut transport,
            &store,
            options,
            &test_data_key(),
            &WeixinServeCancellation::default(),
        )
        .await
        .expect("serve loop");

        assert_eq!(report.runtime_dispatch_count, 1);
        assert_eq!(report.runtime_error_count, 0);
        assert_eq!(dispatcher.calls().len(), 1);
        let state = store.load(&account).expect("load").expect("state");
        assert_eq!(
            state.pending_inbound[0].state,
            yunxi_agent_storage::WeixinPendingInboundState::Succeeded
        );
        assert_eq!(state.pending_inbound[0].lease_owner, None);
    }

    #[tokio::test]
    async fn serve_loop_background_dispatch_shutdown_recovers_running_turn() {
        let (_temp, store, account) = store_fixture();
        let message = text_message(
            "raw-message-background-shutdown",
            "raw-peer-background-shutdown",
        );
        approve_peer_for_message(&store, &account, &message);
        let mut transport = ScriptedTransport {
            responses: VecDeque::from([Ok(GetUpdatesResponse {
                ret: 0,
                msgs: vec![message],
                get_updates_buf: Some(SecretString::new("cursor-background-shutdown")),
                ..GetUpdatesResponse::default()
            })]),
        };
        let mut options = WeixinServeOptions::new(account.clone());
        options.max_polls = Some(1);
        options.background_runtime_dispatch = true;
        options.runtime_dispatch_shutdown_wait = Duration::from_millis(25);
        options.runtime_dispatcher = Some(Arc::new(HoldingDispatcher {
            store: store.clone(),
            started: Arc::new(tokio::sync::Notify::new()),
        }));

        let report = run_weixin_serve_loop(
            &mut transport,
            &store,
            options,
            &test_data_key(),
            &WeixinServeCancellation::default(),
        )
        .await
        .expect("serve loop");

        assert_eq!(report.runtime_dispatch_count, 1);
        assert_eq!(report.runtime_deferred_count, 1);
        let state = store.load(&account).expect("load").expect("state");
        let pending = &state.pending_inbound[0];
        assert_eq!(
            pending.state,
            yunxi_agent_storage::WeixinPendingInboundState::Ready
        );
        assert_eq!(pending.dispatch_retry_count, 1);
        assert_eq!(
            pending.last_dispatch_error.as_deref(),
            Some("runtime_dispatch_shutdown")
        );
        assert_eq!(pending.lease_owner, None);
        assert_eq!(pending.lease_token, None);
    }

    #[tokio::test]
    async fn serve_loop_background_dispatch_panic_is_recoverable() {
        let (_temp, store, account) = store_fixture();
        let message = text_message("raw-message-background-panic", "raw-peer-background-panic");
        approve_peer_for_message(&store, &account, &message);
        let mut transport = ScriptedTransport {
            responses: VecDeque::from([Ok(GetUpdatesResponse {
                ret: 0,
                msgs: vec![message],
                get_updates_buf: Some(SecretString::new("cursor-background-panic")),
                ..GetUpdatesResponse::default()
            })]),
        };
        let mut options = WeixinServeOptions::new(account.clone());
        options.max_polls = Some(1);
        options.background_runtime_dispatch = true;
        options.runtime_dispatch_shutdown_wait = Duration::from_millis(500);
        options.runtime_dispatcher = Some(Arc::new(PanicDispatcher));

        let report = run_weixin_serve_loop(
            &mut transport,
            &store,
            options,
            &test_data_key(),
            &WeixinServeCancellation::default(),
        )
        .await
        .expect("serve loop");

        assert_eq!(report.runtime_dispatch_count, 1);
        assert_eq!(report.runtime_error_count, 1);
        assert_eq!(report.runtime_deferred_count, 1);
        let state = store.load(&account).expect("load").expect("state");
        let pending = &state.pending_inbound[0];
        assert_eq!(
            pending.state,
            yunxi_agent_storage::WeixinPendingInboundState::Ready
        );
        assert_eq!(
            pending.last_dispatch_error.as_deref(),
            Some("runtime_dispatch_panic")
        );
        assert_eq!(pending.lease_owner, None);
        assert_eq!(pending.lease_token, None);
    }

    #[tokio::test]
    async fn serve_loop_records_credential_expiry_without_deleting_account() {
        let (_temp, store, account) = store_fixture();
        let mut transport = ScriptedTransport {
            responses: VecDeque::from([Err(WeixinApiError::Api {
                context: crate::RequestContext::new("wx-test", "get_updates", "raw-account"),
                code: -14,
            })]),
        };
        let mut options = WeixinServeOptions::new(account.clone());
        options.max_polls = Some(1);
        let data_key = test_data_key();
        let error = run_weixin_serve_loop(
            &mut transport,
            &store,
            options,
            &data_key,
            &WeixinServeCancellation::default(),
        )
        .await
        .expect_err("credential expiry");
        assert!(matches!(error, WeixinServeError::CredentialExpired));
        let state = store.load(&account).expect("load").expect("state");
        assert_eq!(
            state.connection_state,
            WeixinConnectionStateRecord::Suspended
        );
        assert_eq!(
            state.last_redacted_error.as_deref(),
            Some("credential_expired")
        );
    }

    #[test]
    fn options_are_safe_to_construct_for_tests() {
        let options = WeixinServeOptions::new("account#933b5bde");
        assert_eq!(options.cursor_source, "getupdates");
        assert!(options.pairing_required);
        assert_eq!(options.pair_request_ttl, DEFAULT_PAIR_REQUEST_TTL);
        assert_eq!(WEIXIN_STATE_SCHEMA_VERSION, 6);
    }

    #[tokio::test]
    async fn serve_loop_rejects_invalid_data_key_before_polling_network() {
        let (_temp, store, account) = store_fixture();
        let mut transport = ScriptedTransport {
            responses: VecDeque::from([Ok(GetUpdatesResponse {
                ret: 0,
                msgs: vec![text_message("raw-message-1", "raw-peer-1")],
                get_updates_buf: Some(SecretString::new("cursor-next")),
                ..GetUpdatesResponse::default()
            })]),
        };
        let mut options = WeixinServeOptions::new(account.clone());
        options.max_polls = Some(1);
        let error = run_weixin_serve_loop(
            &mut transport,
            &store,
            options,
            &SecretString::new("not-a-valid-data-key"),
            &WeixinServeCancellation::default(),
        )
        .await
        .expect_err("invalid key");
        assert!(matches!(
            error,
            WeixinServeError::PayloadCipher(WeixinPayloadCipherError::InvalidDataKey)
        ));
        assert_eq!(transport.responses.len(), 1);
        let state = store.load(&account).expect("load").expect("state");
        assert!(state.cursors.is_empty());
        assert!(state.inbound_receipts.is_empty());
        assert!(state.pending_inbound.is_empty());
    }
}

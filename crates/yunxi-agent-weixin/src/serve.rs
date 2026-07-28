use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use async_trait::async_trait;
use thiserror::Error;
use tokio::time::sleep;
use yunxi_agent_storage::{
    FileWeixinStateStore, WeixinConnectionStateRecord, WeixinInboundBatchCommit,
    WeixinInboundCommitItem, WeixinPairRequestCommitItem, WeixinPairRequestState, WeixinStateError,
    WeixinStateSnapshot, WeixinStateStore,
};

use crate::backoff::WeixinBackoff;
use crate::ilink::{GetUpdatesRequest, GetUpdatesResponse, IlinkHttpClient};
use crate::inbound::{WeixinInboundEnvelope, WeixinInboundKind};
use crate::payload_cipher::{WeixinPayloadAad, WeixinPayloadCipher, WeixinPayloadCipherError};
use crate::turn_supervisor::WeixinRuntimeDispatcher;
use crate::{SecretString, WeixinApiError};

const DEFAULT_PAIR_REQUEST_TTL: Duration = Duration::from_secs(10 * 60);
const DEFAULT_EMPTY_POLL_DELAY: Duration = Duration::from_millis(250);
const MAX_EMPTY_POLL_DELAY: Duration = Duration::from_secs(30);

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
    pub skipped_group_count: usize,
    pub skipped_self_count: usize,
    pub skipped_unsupported_count: usize,
    pub skipped_unknown_count: usize,
    pub network_error_count: usize,
    pub runtime_dispatch_count: usize,
    pub runtime_error_count: usize,
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

        if response.msgs.is_empty() {
            report.empty_polls += 1;
        }
        for message in &response.msgs {
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
                        commit.pair_requests.push(WeixinPairRequestCommitItem {
                            peer_id_hash: envelope.peer_id_hash,
                            expires_at_millis: now
                                .saturating_add(options.pair_request_ttl.as_millis() as u64),
                        });
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
                        let encrypted_payload =
                            payload_cipher.encrypt_pending_inbound(data_key, &plaintext, &aad)?;
                        commit.accepted.push(WeixinInboundCommitItem {
                            item_id,
                            message_id_hash: envelope.message_id_hash.clone(),
                            peer_id_hash: envelope.peer_id_hash.clone(),
                            encrypted_payload_ref,
                            encrypted_payload,
                            payload_kind: Some(envelope.kind.as_str().to_string()),
                        });
                    }
                }
                WeixinInboundKind::GroupMessage => report.skipped_group_count += 1,
                WeixinInboundKind::SelfMessage => report.skipped_self_count += 1,
                WeixinInboundKind::UnsupportedAttachment => report.skipped_unsupported_count += 1,
                WeixinInboundKind::Unknown => report.skipped_unknown_count += 1,
            }
        }

        let result = state_store.commit_inbound_batch(commit)?;
        let accepted_item_ids = result.accepted_item_ids;
        report.accepted_count += result.accepted_count;
        report.duplicate_count += result.duplicate_count;
        report.pair_request_count += result.pair_request_count;

        if let Some(dispatcher) = options.runtime_dispatcher.as_ref() {
            for item_id in accepted_item_ids {
                match dispatcher
                    .dispatch_pending_turn(&options.account_id, &item_id)
                    .await
                {
                    Ok(_) => report.runtime_dispatch_count += 1,
                    Err(_) => report.runtime_error_count += 1,
                }
            }
        }

        if options
            .max_polls
            .is_some_and(|max_polls| report.polls >= max_polls)
        {
            report.stopped_reason = Some(WeixinServeStoppedReason::MaxPolls);
            break;
        }
        if response.msgs.is_empty() {
            let delay = response
                .longpolling_timeout_ms
                .map(Duration::from_millis)
                .unwrap_or(options.empty_poll_delay)
                .min(MAX_EMPTY_POLL_DELAY);
            sleep_cancelable(delay, cancellation).await;
        }
    }

    Ok(report)
}

fn cursor_for(snapshot: &WeixinStateSnapshot, source: &str) -> Option<String> {
    snapshot
        .cursors
        .iter()
        .find(|cursor| cursor.source == source)
        .and_then(|cursor| cursor.get_updates_buf.clone())
}

fn peer_is_approved(snapshot: &WeixinStateSnapshot, peer_id_hash: &str, now_millis: u64) -> bool {
    snapshot.pair_requests.iter().any(|request| {
        request.peer_id_hash == peer_id_hash
            && request.state == WeixinPairRequestState::Approved
            && request.expires_at_millis > now_millis
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WeixinMessageId;
    use crate::ilink::{MessageItem, TextItem, WeixinMessage};
    use crate::turn_supervisor::{WeixinTurnReport, WeixinTurnSupervisorError};
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
    }

    impl RecordingDispatcher {
        fn item_ids(&self) -> Vec<String> {
            self.item_ids.lock().expect("item ids").clone()
        }
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
            Ok(WeixinTurnReport {
                account_id: account_id.to_string(),
                peer_id_hash: "peer#00000000".to_string(),
                direct_message_key: "dm#00000000".to_string(),
                item_id: item_id.to_string(),
                session_id: "yunxi-weixin-test".to_string(),
                status: AgentRunStatus::Completed,
                final_response_present: true,
            })
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
        let dispatcher = RecordingDispatcher::default();
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
        assert_eq!(state.pending_inbound_count(), 1);
        assert_eq!(state.pair_request_count(), 1);
        assert_eq!(
            state.cursors[0].get_updates_buf.as_deref(),
            Some("cursor-next")
        );
        let pending = store
            .load_pending_inbound(&account, &state.pending_inbound[0].item_id)
            .expect("load pending")
            .expect("pending");
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
        let dispatcher = RecordingDispatcher::default();
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
        assert_eq!(WEIXIN_STATE_SCHEMA_VERSION, 3);
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

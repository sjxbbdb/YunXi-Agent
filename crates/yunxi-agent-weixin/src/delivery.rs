use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Mutex;
use unicode_segmentation::UnicodeSegmentation;
use yunxi_agent_storage::{
    FileWeixinStateStore, WeixinDeliveryManifestCommitItem, WeixinDeliveryState,
    WeixinEncryptedPayload, WeixinPendingDeliveryCommitItem, WeixinPendingDeliveryMetadata,
    WeixinStateError,
};

use crate::ilink::{
    IlinkHttpClient, MessageItem, SendMessageRequest, SendMessageResponse, TextItem, WeixinMessage,
};
use crate::payload_cipher::{WeixinPayloadAad, WeixinPayloadCipher, WeixinPayloadCipherError};
use crate::redaction::{SecretString, redacted_identifier};
use crate::turn_supervisor::{
    WeixinRuntimeSink, WeixinRuntimeSinkRecord, WeixinTurnSupervisorError,
};
use crate::{WeixinApiError, WeixinMessageId};

const DELIVERY_PAYLOAD_SCHEMA_VERSION: u32 = 1;
const DEFAULT_SEGMENT_LIMIT_GRAPHEMES: usize = 72;
const DELIVERY_DRAIN_LIMIT: usize = 16;
const MIN_DELIVERY_RETRY_DELAY: Duration = Duration::from_secs(1);
const MAX_DELIVERY_RETRY_DELAY: Duration = Duration::from_secs(60);

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WeixinDeliveryDrainReport {
    pub attempted_count: usize,
    pub succeeded_count: usize,
    pub deferred_count: usize,
    pub failed_count: usize,
    pub unknown_outcome_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WeixinDeliveryOutcomeClass {
    DefiniteFailure,
    OutcomeUnknown,
    RetryableWithVerifiedIdempotency,
    Succeeded,
}

#[derive(Clone)]
pub struct WeixinDeliverySpoolSink {
    state_store: FileWeixinStateStore,
    data_key: SecretString,
    max_segment_graphemes: usize,
    payload_cipher: WeixinPayloadCipher,
}

impl WeixinDeliverySpoolSink {
    pub fn new(state_store: FileWeixinStateStore, data_key: SecretString) -> Self {
        Self {
            state_store,
            data_key,
            max_segment_graphemes: DEFAULT_SEGMENT_LIMIT_GRAPHEMES,
            payload_cipher: WeixinPayloadCipher::new(),
        }
    }

    pub fn with_max_segment_graphemes(mut self, max_segment_graphemes: usize) -> Self {
        self.max_segment_graphemes = max_segment_graphemes.max(1);
        self
    }

    fn write_response(
        &self,
        record: WeixinRuntimeSinkRecord,
        wait_for_turn_completion: bool,
    ) -> Result<(), WeixinTurnSupervisorError> {
        let Some(reply_to_user_id) = record.reply_to_user_id.clone() else {
            return Err(WeixinTurnSupervisorError::SinkFailed);
        };
        let segments =
            split_weixin_text_segments(&record.final_response, self.max_segment_graphemes);
        if segments.is_empty() {
            return Ok(());
        }
        let total_segments = segments.len() as u32;
        let message_hash = redacted_identifier(
            "reply",
            &format!(
                "{}:{}:{}:{}",
                record.account_id, record.item_id, record.session_id, record.final_response
            ),
        );
        let mut items = Vec::with_capacity(segments.len());
        let mut delivery_ids = Vec::with_capacity(segments.len());
        for (index, segment) in segments.into_iter().enumerate() {
            let delivery_id = redacted_identifier(
                "delivery",
                &format!("{}:{message_hash}:{index}", record.item_id),
            );
            let payload = WeixinDeliveryPayload {
                schema_version: DELIVERY_PAYLOAD_SCHEMA_VERSION,
                response_mode: WeixinResponseMode::Text,
                account_id: record.account_id.clone(),
                peer_id_hash: record.peer_id_hash.clone(),
                direct_message_key: record.direct_message_key.clone(),
                item_id: record.item_id.clone(),
                session_id: record.session_id.clone(),
                delivery_id: delivery_id.clone(),
                segment_index: index as u32,
                total_segments,
                text: SecretString::new(segment),
                to_user_id: reply_to_user_id.clone(),
                context_token: record.reply_context_token.clone(),
                client_id: delivery_id.clone(),
            };
            let encrypted_payload = encrypt_delivery_payload(
                &self.payload_cipher,
                &self.data_key,
                &payload,
                &message_hash,
            )
            .map_err(|_| WeixinTurnSupervisorError::SinkFailed)?;
            delivery_ids.push(delivery_id.clone());
            items.push(WeixinPendingDeliveryCommitItem {
                delivery_id,
                peer_id_hash: record.peer_id_hash.clone(),
                direct_message_key: record.direct_message_key.clone(),
                item_id: record.item_id.clone(),
                session_id: record.session_id.clone(),
                message_hash: message_hash.clone(),
                segment_index: index as u32,
                total_segments,
                wait_for_turn_completion,
                encrypted_payload,
            });
        }
        self.state_store.enqueue_pending_delivery_batch(
            &record.account_id,
            WeixinDeliveryManifestCommitItem {
                message_hash,
                item_id: record.item_id,
                session_id: record.session_id,
                total_segments,
                delivery_ids,
            },
            items,
            now_millis_u64(),
        )?;
        Ok(())
    }
}

impl WeixinRuntimeSink for WeixinDeliverySpoolSink {
    fn write_final_response(
        &self,
        record: WeixinRuntimeSinkRecord,
    ) -> Result<(), WeixinTurnSupervisorError> {
        self.write_response(record, true)
    }

    fn write_outbound_text(
        &self,
        record: WeixinRuntimeSinkRecord,
    ) -> Result<(), WeixinTurnSupervisorError> {
        self.write_response(record, false)
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WeixinResponseMode {
    #[default]
    Text,
    Voice,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct WeixinDeliveryPayload {
    pub schema_version: u32,
    #[serde(default)]
    pub response_mode: WeixinResponseMode,
    pub account_id: String,
    pub peer_id_hash: String,
    pub direct_message_key: String,
    pub item_id: String,
    pub session_id: String,
    pub delivery_id: String,
    pub segment_index: u32,
    pub total_segments: u32,
    pub text: SecretString,
    pub to_user_id: SecretString,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_token: Option<SecretString>,
    pub client_id: String,
}

impl fmt::Debug for WeixinDeliveryPayload {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WeixinDeliveryPayload")
            .field("schema_version", &self.schema_version)
            .field("response_mode", &self.response_mode)
            .field("account_id", &self.account_id)
            .field("peer_id_hash", &self.peer_id_hash)
            .field("direct_message_key", &self.direct_message_key)
            .field("item_id", &self.item_id)
            .field("session_id", &self.session_id)
            .field("delivery_id", &self.delivery_id)
            .field("segment_index", &self.segment_index)
            .field("total_segments", &self.total_segments)
            .field("text", &"[REDACTED]")
            .field("to_user_id", &"[REDACTED]")
            .field(
                "context_token",
                &self.context_token.as_ref().map(|_| "[REDACTED]"),
            )
            .field("client_id", &self.client_id)
            .finish()
    }
}

impl WeixinDeliveryPayload {
    pub(crate) fn to_plaintext_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    pub(crate) fn from_plaintext_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    fn matches_delivery(&self, delivery: &WeixinPendingDeliveryMetadata) -> bool {
        self.schema_version == DELIVERY_PAYLOAD_SCHEMA_VERSION
            && self.account_id == delivery.account_id
            && self.peer_id_hash == delivery.peer_id_hash
            && self.direct_message_key == delivery.direct_message_key
            && self.item_id == delivery.item_id
            && self.session_id == delivery.session_id
            && self.delivery_id == delivery.delivery_id
            && self.segment_index == delivery.segment_index
            && self.total_segments == delivery.total_segments
    }

    fn to_send_message_request(&self) -> SendMessageRequest {
        SendMessageRequest::new(WeixinMessage {
            message_id: WeixinMessageId::new(""),
            from_user_id: SecretString::new(""),
            to_user_id: Some(self.to_user_id.clone()),
            client_id: Some(self.client_id.clone()),
            create_time_ms: None,
            session_id: None,
            group_id: None,
            message_type: Some(2),
            message_state: Some(2),
            item_list: vec![MessageItem {
                item_type: 1,
                text_item: Some(TextItem {
                    text: self.text.clone(),
                }),
                voice_item: None,
                is_completed: None,
                msg_id: None,
            }],
            context_token: self.context_token.clone(),
        })
    }
}

#[async_trait]
pub trait WeixinMessageTransport: Send + Sync {
    async fn send_message(
        &self,
        request: SendMessageRequest,
    ) -> Result<SendMessageResponse, WeixinApiError>;
}

#[async_trait]
impl WeixinMessageTransport for IlinkHttpClient {
    async fn send_message(
        &self,
        request: SendMessageRequest,
    ) -> Result<SendMessageResponse, WeixinApiError> {
        IlinkHttpClient::send_message(self, request).await
    }
}

#[derive(Clone)]
pub struct WeixinDeliveryDispatcher {
    state_store: FileWeixinStateStore,
    data_key: SecretString,
    transport: Arc<dyn WeixinMessageTransport>,
    payload_cipher: WeixinPayloadCipher,
    verified_idempotency: bool,
    drain_lock: Arc<Mutex<()>>,
}

impl WeixinDeliveryDispatcher {
    pub fn new(
        state_store: FileWeixinStateStore,
        data_key: SecretString,
        transport: Arc<dyn WeixinMessageTransport>,
    ) -> Self {
        Self {
            state_store,
            data_key,
            transport,
            payload_cipher: WeixinPayloadCipher::new(),
            verified_idempotency: false,
            drain_lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn with_verified_idempotency(mut self, verified: bool) -> Self {
        self.verified_idempotency = verified;
        self
    }

    pub async fn drain_ready(
        &self,
        account_id: &str,
        now_millis: u64,
    ) -> Result<WeixinDeliveryDrainReport, WeixinDeliveryError> {
        let _drain_guard = self.drain_lock.lock().await;
        let mut report = WeixinDeliveryDrainReport::default();
        let deliveries = self.state_store.load_ready_pending_deliveries(
            account_id,
            now_millis,
            DELIVERY_DRAIN_LIMIT,
        )?;
        for delivery in deliveries {
            report.attempted_count += 1;
            let delivery_started_at_millis = now_millis_u64();
            let running = self.state_store.mark_pending_delivery_running(
                account_id,
                &delivery.delivery_id,
                delivery_started_at_millis,
            )?;
            let payload =
                match decrypt_delivery_payload(&self.payload_cipher, &self.data_key, &running) {
                    Ok(payload) => payload,
                    Err(error) => {
                        let delivery_completed_at_millis = now_millis_u64();
                        self.state_store.complete_pending_delivery(
                            account_id,
                            &running.delivery_id,
                            WeixinDeliveryState::Failed,
                            Some("payload_unavailable".to_string()),
                            Some("delivery_payload_unavailable".to_string()),
                            delivery_completed_at_millis,
                        )?;
                        report.failed_count += 1;
                        let _ = error;
                        continue;
                    }
                };
            match send_delivery_payload(self.transport.as_ref(), payload).await {
                Ok(_) => {
                    let delivery_completed_at_millis = now_millis_u64();
                    self.state_store.complete_pending_delivery(
                        account_id,
                        &running.delivery_id,
                        WeixinDeliveryState::Succeeded,
                        Some("sent".to_string()),
                        None,
                        delivery_completed_at_millis,
                    )?;
                    report.succeeded_count += 1;
                }
                Err(error) => match classify_delivery_error(&error, self.verified_idempotency) {
                    WeixinDeliveryOutcomeClass::DefiniteFailure => {
                        let delivery_completed_at_millis = now_millis_u64();
                        let error_label = delivery_error_label(&error);
                        self.state_store.complete_pending_delivery(
                            account_id,
                            &running.delivery_id,
                            WeixinDeliveryState::Failed,
                            Some("definite_failure".to_string()),
                            Some(error_label),
                            delivery_completed_at_millis,
                        )?;
                        report.failed_count += 1;
                    }
                    WeixinDeliveryOutcomeClass::OutcomeUnknown => {
                        let delivery_completed_at_millis = now_millis_u64();
                        let error_label = delivery_error_label(&error);
                        self.state_store.complete_pending_delivery(
                            account_id,
                            &running.delivery_id,
                            WeixinDeliveryState::Unknown,
                            Some("outcome_unknown".to_string()),
                            Some(error_label),
                            delivery_completed_at_millis,
                        )?;
                        report.unknown_outcome_count += 1;
                    }
                    WeixinDeliveryOutcomeClass::RetryableWithVerifiedIdempotency => {
                        let delivery_completed_at_millis = now_millis_u64();
                        let delay = delivery_retry_delay(running.retry_count);
                        let error_label = delivery_error_label(&error);
                        self.state_store.record_pending_delivery_deferred(
                            account_id,
                            &running.delivery_id,
                            &error_label,
                            delivery_completed_at_millis.saturating_add(delay.as_millis() as u64),
                            delivery_completed_at_millis,
                        )?;
                        report.deferred_count += 1;
                        break;
                    }
                    WeixinDeliveryOutcomeClass::Succeeded => unreachable!(),
                },
            }
        }
        Ok(report)
    }
}

async fn send_delivery_payload(
    transport: &dyn WeixinMessageTransport,
    payload: WeixinDeliveryPayload,
) -> Result<SendMessageResponse, WeixinApiError> {
    let response = transport
        .send_message(payload.to_send_message_request())
        .await;
    if matches!(response, Err(WeixinApiError::Api { .. })) && payload.context_token.is_some() {
        let mut fallback_payload = payload;
        fallback_payload.context_token = None;
        transport
            .send_message(fallback_payload.to_send_message_request())
            .await
    } else {
        response
    }
}

#[derive(Debug, Error)]
pub enum WeixinDeliveryError {
    #[error("weixin delivery state failed: {0}")]
    State(#[from] WeixinStateError),
    #[error("weixin delivery payload cipher failed: {0}")]
    PayloadCipher(#[from] WeixinPayloadCipherError),
}

pub fn split_weixin_text_segments(text: &str, max_segment_graphemes: usize) -> Vec<String> {
    let max_segment_graphemes = max_segment_graphemes.max(1);
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut current_count = 0usize;
    let mut sentence_complete = false;
    for grapheme in UnicodeSegmentation::graphemes(text, true) {
        if grapheme == "\r" {
            continue;
        }
        if grapheme == "\n" {
            push_weixin_segment(&mut segments, &mut current);
            current_count = 0;
            sentence_complete = false;
            continue;
        }
        if sentence_complete && !is_sentence_suffix(grapheme) {
            push_weixin_segment(&mut segments, &mut current);
            current_count = 0;
            sentence_complete = false;
        }
        current.push_str(grapheme);
        current_count += 1;
        sentence_complete |= is_sentence_terminal(grapheme);
        if current_count >= max_segment_graphemes {
            push_weixin_segment(&mut segments, &mut current);
            current_count = 0;
            sentence_complete = false;
        }
    }
    push_weixin_segment(&mut segments, &mut current);
    segments
}

fn push_weixin_segment(segments: &mut Vec<String>, current: &mut String) {
    let segment = current.trim();
    if !segment.is_empty() {
        segments.push(segment.to_string());
    }
    current.clear();
}

fn is_sentence_terminal(grapheme: &str) -> bool {
    matches!(grapheme, "。" | "！" | "？" | "!" | "?" | "；" | ";" | "…")
}

fn is_sentence_suffix(grapheme: &str) -> bool {
    is_sentence_terminal(grapheme)
        || matches!(
            grapheme,
            "\"" | "'" | "”" | "’" | "」" | "』" | "）" | ")" | "】" | "]"
        )
}

fn encrypt_delivery_payload(
    cipher: &WeixinPayloadCipher,
    data_key: &SecretString,
    payload: &WeixinDeliveryPayload,
    message_hash: &str,
) -> Result<WeixinEncryptedPayload, WeixinDeliveryError> {
    let aad = WeixinPayloadAad::new(
        &payload.account_id,
        &payload.peer_id_hash,
        message_hash,
        &payload.delivery_id,
    );
    let bytes = payload
        .to_plaintext_bytes()
        .map_err(|_| WeixinPayloadCipherError::InvalidPlaintext)?;
    Ok(cipher.encrypt(data_key, &bytes, &aad)?)
}

fn decrypt_delivery_payload(
    cipher: &WeixinPayloadCipher,
    data_key: &SecretString,
    delivery: &WeixinPendingDeliveryMetadata,
) -> Result<WeixinDeliveryPayload, WeixinPayloadCipherError> {
    let payload = delivery
        .encrypted_payload
        .as_ref()
        .ok_or(WeixinPayloadCipherError::MissingPayload)?;
    let aad = WeixinPayloadAad::new(
        &delivery.account_id,
        &delivery.peer_id_hash,
        &delivery.message_hash,
        &delivery.delivery_id,
    );
    let bytes = cipher.decrypt(data_key, payload, &aad)?;
    let plaintext = WeixinDeliveryPayload::from_plaintext_bytes(&bytes)
        .map_err(|_| WeixinPayloadCipherError::InvalidPlaintext)?;
    if !plaintext.matches_delivery(delivery) {
        return Err(WeixinPayloadCipherError::InvalidPlaintext);
    }
    Ok(plaintext)
}

fn delivery_retry_delay(retry_count: u32) -> Duration {
    let shift = retry_count.min(6);
    let millis = (MIN_DELIVERY_RETRY_DELAY.as_millis() as u64)
        .saturating_mul(1_u64 << shift)
        .min(MAX_DELIVERY_RETRY_DELAY.as_millis() as u64);
    Duration::from_millis(millis)
}

fn classify_delivery_error(
    error: &WeixinApiError,
    verified_idempotency: bool,
) -> WeixinDeliveryOutcomeClass {
    match error {
        WeixinApiError::HttpStatus { status, .. }
            if verified_idempotency && matches!(*status, 408 | 425 | 429 | 500..=599) =>
        {
            WeixinDeliveryOutcomeClass::RetryableWithVerifiedIdempotency
        }
        WeixinApiError::Api { code, .. }
            if verified_idempotency && matches!(*code, -1 | -2 | 500..=599) =>
        {
            WeixinDeliveryOutcomeClass::RetryableWithVerifiedIdempotency
        }
        WeixinApiError::HttpStatus { status, .. } if (400..=499).contains(status) => {
            WeixinDeliveryOutcomeClass::DefiniteFailure
        }
        WeixinApiError::Api { code, .. } if (400..=499).contains(&(*code as i64)) => {
            WeixinDeliveryOutcomeClass::DefiniteFailure
        }
        WeixinApiError::HttpStatus { .. } | WeixinApiError::Api { .. } => {
            WeixinDeliveryOutcomeClass::OutcomeUnknown
        }
        WeixinApiError::Timeout { .. }
        | WeixinApiError::Network { .. }
        | WeixinApiError::InvalidJson { .. }
        | WeixinApiError::Protocol { .. }
        | WeixinApiError::ResponseTooLarge { .. } => WeixinDeliveryOutcomeClass::OutcomeUnknown,
    }
}

fn delivery_error_label(error: &WeixinApiError) -> String {
    match error {
        WeixinApiError::Timeout { .. } => "delivery_timeout".to_string(),
        WeixinApiError::Network { .. } => "delivery_network_error".to_string(),
        WeixinApiError::HttpStatus { .. } => "delivery_http_error".to_string(),
        WeixinApiError::ResponseTooLarge { .. } => "delivery_response_too_large".to_string(),
        WeixinApiError::Api { code, .. } => {
            if *code < 0 {
                format!("delivery_api_error_code_neg{}", code.saturating_abs())
            } else {
                format!("delivery_api_error_code_{code}")
            }
        }
        WeixinApiError::InvalidJson { .. } => "delivery_invalid_json".to_string(),
        WeixinApiError::Protocol { .. } => "delivery_protocol_error".to_string(),
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
    use std::sync::Mutex;
    use yunxi_agent_storage::WeixinDeliveryManifestState;
    use yunxi_agent_storage::{WeixinStateSnapshot, WeixinStateStore};

    #[test]
    fn split_weixin_text_segments_preserves_unicode_graphemes() {
        let segments = split_weixin_text_segments("a👨‍👩‍👧‍👦bcd", 2);
        assert_eq!(segments, vec!["a👨‍👩‍👧‍👦", "bc", "d"]);
    }

    #[test]
    fn split_weixin_text_segments_prefers_natural_sentences() {
        let segments =
            split_weixin_text_segments("听懂了。刚才那句确实有点生硬！以后我会说得自然一点。", 72);
        assert_eq!(
            segments,
            vec![
                "听懂了。",
                "刚才那句确实有点生硬！",
                "以后我会说得自然一点。"
            ]
        );
    }

    #[test]
    fn split_weixin_text_segments_keeps_closing_quote_with_sentence() {
        let segments = split_weixin_text_segments("她说：“知道啦！”然后笑了。", 72);
        assert_eq!(segments, vec!["她说：“知道啦！”", "然后笑了。"]);
    }

    #[test]
    fn delivery_payload_debug_redacts_secret_fields() {
        let payload = WeixinDeliveryPayload {
            schema_version: DELIVERY_PAYLOAD_SCHEMA_VERSION,
            response_mode: WeixinResponseMode::Text,
            account_id: "account#11111111".to_string(),
            peer_id_hash: "peer#22222222".to_string(),
            direct_message_key: "dm#33333333".to_string(),
            item_id: "item#44444444".to_string(),
            session_id: "session-1".to_string(),
            delivery_id: "delivery#55555555".to_string(),
            segment_index: 0,
            total_segments: 1,
            text: SecretString::new("secret answer"),
            to_user_id: SecretString::new("raw-user"),
            context_token: Some(SecretString::new("raw-context")),
            client_id: "delivery#55555555".to_string(),
        };
        let debug = format!("{payload:?}");
        assert!(!debug.contains("secret answer"));
        assert!(!debug.contains("raw-user"));
        assert!(!debug.contains("raw-context"));
        assert!(debug.contains("delivery#55555555"));
    }

    #[test]
    fn sendmessage_request_matches_minimal_ilink_text_shape() {
        let payload = WeixinDeliveryPayload {
            schema_version: DELIVERY_PAYLOAD_SCHEMA_VERSION,
            response_mode: WeixinResponseMode::Text,
            account_id: "account#11111111".to_string(),
            peer_id_hash: "peer#22222222".to_string(),
            direct_message_key: "dm#33333333".to_string(),
            item_id: "item#44444444".to_string(),
            session_id: "session-1".to_string(),
            delivery_id: "delivery#55555555".to_string(),
            segment_index: 0,
            total_segments: 1,
            text: SecretString::new("hello"),
            to_user_id: SecretString::new("raw-chat"),
            context_token: None,
            client_id: "delivery#55555555".to_string(),
        };

        let value = serde_json::to_value(payload.to_send_message_request()).expect("send JSON");

        assert_eq!(value["msg"]["from_user_id"].as_str(), Some(""));
        assert_eq!(value["msg"]["to_user_id"].as_str(), Some("raw-chat"));
        assert_eq!(
            value["msg"]["client_id"].as_str(),
            Some("delivery#55555555")
        );
        assert_eq!(value["msg"]["message_type"].as_u64(), Some(2));
        assert_eq!(value["msg"]["message_state"].as_u64(), Some(2));
        assert_eq!(value["msg"]["item_list"][0]["type"].as_u64(), Some(1));
        assert_eq!(
            value["msg"]["item_list"][0]["text_item"]["text"].as_str(),
            Some("hello")
        );
        assert!(value["msg"].get("message_id").is_none());
        assert!(value["msg"].get("context_token").is_none());
        assert!(value["msg"].get("create_time_ms").is_none());
        assert!(value["msg"]["item_list"][0].get("is_completed").is_none());
        assert!(value["msg"]["item_list"][0].get("msg_id").is_none());
        assert_eq!(
            value["base_info"]["channel_version"].as_str(),
            Some(env!("CARGO_PKG_VERSION"))
        );
        assert!(value["base_info"].get("bot_agent").is_none());
    }

    #[tokio::test]
    async fn legacy_voice_delivery_marker_is_sent_as_text() {
        let transport = RecordingMessageTransport::default();
        let payload = WeixinDeliveryPayload {
            schema_version: DELIVERY_PAYLOAD_SCHEMA_VERSION,
            response_mode: WeixinResponseMode::Voice,
            account_id: "account#11111111".to_string(),
            peer_id_hash: "peer#22222222".to_string(),
            direct_message_key: "dm#33333333".to_string(),
            item_id: "item#44444444".to_string(),
            session_id: "session-1".to_string(),
            delivery_id: "delivery#55555555".to_string(),
            segment_index: 0,
            total_segments: 1,
            text: SecretString::new("兼容文字回复"),
            to_user_id: SecretString::new("raw-chat"),
            context_token: None,
            client_id: "delivery#55555555".to_string(),
        };

        send_delivery_payload(&transport, payload)
            .await
            .expect("legacy marker should use text transport");

        let requests = transport.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].msg.item_list[0].item_type, 1);
        assert_eq!(
            requests[0].msg.item_list[0]
                .text_item
                .as_ref()
                .expect("text item")
                .text
                .expose(),
            "兼容文字回复"
        );
    }

    #[test]
    fn spool_sink_encrypts_final_text_without_plain_state_leakage() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = FileWeixinStateStore::for_workspace(temp.path());
        let account = "account#11111111";
        let snapshot =
            WeixinStateSnapshot::new(account, "workspace#22222222", "https://example.invalid/", 1);
        store.save(&snapshot).expect("save state");
        let data_key = crate::generate_data_key().expect("data key");
        let sink = WeixinDeliverySpoolSink::new(store.clone(), data_key.clone())
            .with_max_segment_graphemes(2);
        sink.write_final_response(WeixinRuntimeSinkRecord {
            account_id: account.to_string(),
            peer_id_hash: "peer#33333333".to_string(),
            direct_message_key: "dm#44444444".to_string(),
            item_id: "item#55555555".to_string(),
            session_id: "session-1".to_string(),
            reply_to_user_id: Some(SecretString::new("raw-user")),
            reply_context_token: Some(SecretString::new("raw-context")),
            final_response: "abcd".to_string(),
        })
        .expect("spool");
        let state = store
            .load(account)
            .expect("load state")
            .expect("state exists");
        assert_eq!(state.pending_deliveries.len(), 2);
        for delivery in &state.pending_deliveries {
            let payload =
                decrypt_delivery_payload(&WeixinPayloadCipher::new(), &data_key, delivery)
                    .expect("decrypt delivery");
            assert_eq!(payload.response_mode, WeixinResponseMode::Text);
        }
        let serialized = serde_json::to_string(&state).expect("json");
        assert!(!serialized.contains("abcd"));
        assert!(!serialized.contains("raw-user"));
        assert!(!serialized.contains("raw-context"));
        assert!(
            state
                .pending_deliveries
                .iter()
                .all(|delivery| delivery.encrypted_payload.is_some())
        );
    }

    #[tokio::test]
    async fn dispatcher_sends_ready_delivery_and_marks_success() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = FileWeixinStateStore::for_workspace(temp.path());
        let account = "account#11111111";
        store
            .save(&WeixinStateSnapshot::new(
                account,
                "workspace#22222222",
                "https://example.invalid/",
                1,
            ))
            .expect("save state");
        let data_key = crate::generate_data_key().expect("data key");
        let sink = WeixinDeliverySpoolSink::new(store.clone(), data_key.clone());
        sink.write_final_response(WeixinRuntimeSinkRecord {
            account_id: account.to_string(),
            peer_id_hash: "peer#33333333".to_string(),
            direct_message_key: "dm#44444444".to_string(),
            item_id: "item#55555555".to_string(),
            session_id: "session-1".to_string(),
            reply_to_user_id: Some(SecretString::new("raw-user")),
            reply_context_token: Some(SecretString::new("raw-context")),
            final_response: "hello".to_string(),
        })
        .expect("spool");
        let transport = Arc::new(RecordingMessageTransport::default());
        let dispatcher = WeixinDeliveryDispatcher::new(store.clone(), data_key, transport.clone());
        let report = dispatcher
            .drain_ready(account, 10)
            .await
            .expect("drain delivery");
        assert_eq!(report.succeeded_count, 1);
        let requests = transport.requests();
        assert_eq!(requests.len(), 1);
        let msg = &requests[0].msg;
        assert_eq!(msg.from_user_id.expose(), "");
        assert_eq!(
            msg.to_user_id.as_ref().map(|value| value.expose()),
            Some("raw-user")
        );
        assert_eq!(
            msg.context_token.as_ref().map(|value| value.expose()),
            Some("raw-context")
        );
        assert_eq!(msg.message_type, Some(2));
        assert_eq!(msg.message_state, Some(2));
        assert_eq!(
            msg.item_list[0].text_item.as_ref().unwrap().text.expose(),
            "hello"
        );
        let state = store
            .load(account)
            .expect("load state")
            .expect("state exists");
        assert_eq!(state.pending_deliveries.len(), 0);
        assert_eq!(state.deliveries.len(), 1);
        assert_eq!(state.deliveries[0].state, WeixinDeliveryState::Succeeded);
    }

    #[tokio::test]
    async fn spool_sink_commits_a_single_manifest_for_multi_segment_reply() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = FileWeixinStateStore::for_workspace(temp.path());
        let account = "account#11111111";
        store
            .save(&WeixinStateSnapshot::new(
                account,
                "workspace#22222222",
                "https://example.invalid/",
                1,
            ))
            .expect("save state");
        let data_key = crate::generate_data_key().expect("data key");
        let sink =
            WeixinDeliverySpoolSink::new(store.clone(), data_key).with_max_segment_graphemes(2);
        sink.write_final_response(WeixinRuntimeSinkRecord {
            account_id: account.to_string(),
            peer_id_hash: "peer#33333333".to_string(),
            direct_message_key: "dm#44444444".to_string(),
            item_id: "item#55555555".to_string(),
            session_id: "session-1".to_string(),
            reply_to_user_id: Some(SecretString::new("raw-user")),
            reply_context_token: Some(SecretString::new("raw-context")),
            final_response: "abcd".to_string(),
        })
        .expect("spool");
        let state = store
            .load(account)
            .expect("load state")
            .expect("state exists");
        assert_eq!(state.delivery_manifests.len(), 1);
        assert_eq!(state.pending_deliveries.len(), 2);
        assert_eq!(
            state.delivery_manifests[0].state,
            WeixinDeliveryManifestState::Pending
        );
        assert_eq!(
            state.delivery_manifests[0].delivery_ids,
            state
                .pending_deliveries
                .iter()
                .map(|delivery| delivery.delivery_id.clone())
                .collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn spool_sink_allows_control_prompt_and_final_text_for_same_turn() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = FileWeixinStateStore::for_workspace(temp.path());
        let account = "account#11111111";
        store
            .save(&WeixinStateSnapshot::new(
                account,
                "workspace#22222222",
                "https://example.invalid/",
                1,
            ))
            .expect("save state");
        let data_key = crate::generate_data_key().expect("data key");
        let sink = WeixinDeliverySpoolSink::new(store.clone(), data_key);
        let base_record = WeixinRuntimeSinkRecord {
            account_id: account.to_string(),
            peer_id_hash: "peer#33333333".to_string(),
            direct_message_key: "dm#44444444".to_string(),
            item_id: "item#55555555".to_string(),
            session_id: "session-1".to_string(),
            reply_to_user_id: Some(SecretString::new("raw-user")),
            reply_context_token: Some(SecretString::new("raw-context")),
            final_response:
                "[YunXi]\n需要你确认一次操作：shell\n回复 /approve 允许，或回复 /deny 拒绝。"
                    .to_string(),
        };

        sink.write_outbound_text(base_record.clone())
            .expect("spool control prompt");
        sink.write_final_response(WeixinRuntimeSinkRecord {
            final_response: "晚上好，我在。".to_string(),
            ..base_record
        })
        .expect("spool final text");

        let state = store
            .load(account)
            .expect("load state")
            .expect("state exists");
        assert_eq!(state.delivery_manifests.len(), 2);
        assert_eq!(
            state.pending_deliveries.len(),
            state
                .delivery_manifests
                .iter()
                .map(|manifest| manifest.total_segments as usize)
                .sum::<usize>()
        );
        assert_ne!(
            state.delivery_manifests[0].message_hash,
            state.delivery_manifests[1].message_hash
        );
        assert!(
            state
                .delivery_manifests
                .iter()
                .all(|manifest| manifest.item_id == "item#55555555"
                    && manifest.session_id == "session-1"
                    && manifest.state == WeixinDeliveryManifestState::Pending)
        );
    }

    struct TimeoutMessageTransport;

    #[async_trait]
    impl WeixinMessageTransport for TimeoutMessageTransport {
        async fn send_message(
            &self,
            _request: SendMessageRequest,
        ) -> Result<SendMessageResponse, WeixinApiError> {
            Err(WeixinApiError::Timeout {
                context: crate::RequestContext::new("wx-test", "send_message", "account#11111111"),
            })
        }
    }

    #[tokio::test]
    async fn dispatcher_marks_timeout_as_unknown_without_verified_idempotency() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = FileWeixinStateStore::for_workspace(temp.path());
        let account = "account#11111111";
        store
            .save(&WeixinStateSnapshot::new(
                account,
                "workspace#22222222",
                "https://example.invalid/",
                1,
            ))
            .expect("save state");
        let data_key = crate::generate_data_key().expect("data key");
        let sink = WeixinDeliverySpoolSink::new(store.clone(), data_key.clone())
            .with_max_segment_graphemes(2);
        sink.write_final_response(WeixinRuntimeSinkRecord {
            account_id: account.to_string(),
            peer_id_hash: "peer#33333333".to_string(),
            direct_message_key: "dm#44444444".to_string(),
            item_id: "item#55555555".to_string(),
            session_id: "session-1".to_string(),
            reply_to_user_id: Some(SecretString::new("raw-user")),
            reply_context_token: Some(SecretString::new("raw-context")),
            final_response: "ab".to_string(),
        })
        .expect("spool");
        let dispatcher = WeixinDeliveryDispatcher::new(
            store.clone(),
            data_key,
            Arc::new(TimeoutMessageTransport),
        );
        let report = dispatcher.drain_ready(account, 10).await.expect("drain");
        assert_eq!(report.unknown_outcome_count, 1);
        let state = store
            .load(account)
            .expect("load state")
            .expect("state exists");
        assert_eq!(state.pending_deliveries.len(), 0);
        assert_eq!(state.deliveries.len(), 1);
        assert_eq!(state.deliveries[0].state, WeixinDeliveryState::Unknown);
        assert_eq!(
            state.delivery_manifests[0].state,
            WeixinDeliveryManifestState::Unknown
        );
    }

    #[derive(Default)]
    struct RecordingMessageTransport {
        requests: Mutex<Vec<SendMessageRequest>>,
    }

    impl RecordingMessageTransport {
        fn requests(&self) -> Vec<SendMessageRequest> {
            self.requests.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl WeixinMessageTransport for RecordingMessageTransport {
        async fn send_message(
            &self,
            request: SendMessageRequest,
        ) -> Result<SendMessageResponse, WeixinApiError> {
            self.requests.lock().unwrap().push(request);
            Ok(SendMessageResponse::default())
        }
    }

    #[derive(Default)]
    struct ContextFallbackTransport {
        requests: Mutex<Vec<SendMessageRequest>>,
    }

    impl ContextFallbackTransport {
        fn requests(&self) -> Vec<SendMessageRequest> {
            self.requests.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl WeixinMessageTransport for ContextFallbackTransport {
        async fn send_message(
            &self,
            request: SendMessageRequest,
        ) -> Result<SendMessageResponse, WeixinApiError> {
            let has_context = request.msg.context_token.is_some();
            self.requests.lock().unwrap().push(request);
            if has_context {
                Err(WeixinApiError::Api {
                    context: crate::RequestContext::new(
                        "wx-context-expired",
                        "send_message",
                        "account#11111111",
                    ),
                    code: -1,
                })
            } else {
                Ok(SendMessageResponse::default())
            }
        }
    }

    #[tokio::test]
    async fn dispatcher_retries_once_without_stale_context_token() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = FileWeixinStateStore::for_workspace(temp.path());
        let account = "account#11111111";
        store
            .save(&WeixinStateSnapshot::new(
                account,
                "workspace#22222222",
                "https://example.invalid/",
                1,
            ))
            .expect("save state");
        let data_key = crate::generate_data_key().expect("data key");
        let sink = WeixinDeliverySpoolSink::new(store.clone(), data_key.clone());
        sink.write_final_response(WeixinRuntimeSinkRecord {
            account_id: account.to_string(),
            peer_id_hash: "peer#33333333".to_string(),
            direct_message_key: "dm#44444444".to_string(),
            item_id: "item#55555555".to_string(),
            session_id: "session-1".to_string(),
            reply_to_user_id: Some(SecretString::new("raw-user")),
            reply_context_token: Some(SecretString::new("stale-context")),
            final_response: "hello".to_string(),
        })
        .expect("spool");
        let transport = Arc::new(ContextFallbackTransport::default());
        let dispatcher = WeixinDeliveryDispatcher::new(store.clone(), data_key, transport.clone());

        let report = dispatcher.drain_ready(account, 10).await.expect("drain");

        assert_eq!(report.succeeded_count, 1);
        assert_eq!(report.unknown_outcome_count, 0);
        let requests = transport.requests();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].msg.context_token.is_some());
        assert!(requests[1].msg.context_token.is_none());
        let state = store
            .load(account)
            .expect("load state")
            .expect("state exists");
        assert_eq!(state.deliveries.len(), 1);
        assert_eq!(state.deliveries[0].state, WeixinDeliveryState::Succeeded);
    }
}

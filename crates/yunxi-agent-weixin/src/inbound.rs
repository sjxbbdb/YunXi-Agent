use serde::{Deserialize, Serialize};
use std::fmt;

use crate::ilink::{VoiceItem, WeixinMessage};
use crate::redaction::redacted_identifier;
use crate::{SecretString, WeixinPeerId};

const TEXT_ITEM_TYPE: u32 = 1;
const VOICE_ITEM_TYPE: u32 = 3;
pub const WEIXIN_PENDING_PAYLOAD_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WeixinInboundKind {
    Text,
    Voice,
    UnsupportedAttachment,
    GroupMessage,
    SelfMessage,
    Unknown,
}

impl WeixinInboundKind {
    pub fn is_pairable_private_text(self) -> bool {
        matches!(self, Self::Text)
    }

    pub fn is_pairable_private(self) -> bool {
        matches!(self, Self::Text | Self::Voice)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Voice => "voice",
            Self::UnsupportedAttachment => "unsupported_attachment",
            Self::GroupMessage => "group_message",
            Self::SelfMessage => "self_message",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WeixinInboundEnvelope {
    pub account_id: String,
    pub peer_id_hash: String,
    pub message_id_hash: String,
    pub direct_message_key: String,
    pub created_at_millis: u64,
    pub context_reference_id: Option<String>,
    pub kind: WeixinInboundKind,
}

impl WeixinInboundEnvelope {
    pub fn from_message(
        account_id: &str,
        self_user_id: Option<&SecretString>,
        message: &WeixinMessage,
        now_millis: u64,
    ) -> Self {
        let peer_id_hash = WeixinPeerId::new(message.from_user_id.expose()).to_string();
        let message_id_hash = redacted_identifier("message", message.message_id.as_str());
        let direct_message_key = redacted_identifier("dm", &format!("{account_id}:{peer_id_hash}"));
        let context_reference_id = message
            .context_token
            .as_ref()
            .filter(|token| !token.is_empty())
            .map(|token| redacted_identifier("context", token.expose()));
        let created_at_millis = message.create_time_ms.unwrap_or(now_millis);
        let kind = classify_message(self_user_id, message);
        Self {
            account_id: account_id.to_string(),
            peer_id_hash,
            message_id_hash,
            direct_message_key,
            created_at_millis,
            context_reference_id,
            kind,
        }
    }

    pub fn pending_item_id(&self) -> String {
        redacted_identifier(
            "item",
            &format!(
                "{}:{}:{}",
                self.account_id, self.peer_id_hash, self.message_id_hash
            ),
        )
    }

    pub fn encrypted_payload_ref(&self) -> String {
        redacted_identifier(
            "pending",
            &format!(
                "{}:{}:{}",
                self.account_id, self.peer_id_hash, self.message_id_hash
            ),
        )
    }

    pub fn recoverable_text_payload(
        &self,
        message: &WeixinMessage,
        item_id: &str,
    ) -> Option<WeixinPendingInboundPayload> {
        self.recoverable_payload(message, item_id)
            .filter(|payload| payload.payload_kind == WeixinInboundKind::Text)
    }

    pub fn recoverable_payload(
        &self,
        message: &WeixinMessage,
        item_id: &str,
    ) -> Option<WeixinPendingInboundPayload> {
        let (text, voice) = match self.kind {
            WeixinInboundKind::Text => (Some(first_text_item(message)?), None),
            WeixinInboundKind::Voice => (None, Some(first_voice_item(message)?)),
            _ => return None,
        };
        Some(WeixinPendingInboundPayload {
            schema_version: WEIXIN_PENDING_PAYLOAD_SCHEMA_VERSION,
            payload_kind: self.kind,
            account_id: self.account_id.clone(),
            peer_id_hash: self.peer_id_hash.clone(),
            message_id_hash: self.message_id_hash.clone(),
            item_id: item_id.to_string(),
            direct_message_key: self.direct_message_key.clone(),
            created_at_millis: self.created_at_millis,
            context_reference_id: self.context_reference_id.clone(),
            reply_to_user_id: Some(message.reply_target_id()),
            reply_context_token: message.context_token.clone(),
            text,
            voice,
        })
    }
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct WeixinPendingInboundPayload {
    pub schema_version: u32,
    pub payload_kind: WeixinInboundKind,
    pub account_id: String,
    pub peer_id_hash: String,
    pub message_id_hash: String,
    pub item_id: String,
    pub direct_message_key: String,
    pub created_at_millis: u64,
    pub context_reference_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to_user_id: Option<SecretString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_context_token: Option<SecretString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<SecretString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice: Option<WeixinInboundVoice>,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct WeixinInboundVoice {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypt_query_param: Option<SecretString>,
    pub aes_key: SecretString,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_url: Option<SecretString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encode_type: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub playtime: Option<u64>,
}

impl fmt::Debug for WeixinInboundVoice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WeixinInboundVoice")
            .field(
                "encrypt_query_param",
                &self.encrypt_query_param.as_ref().map(|_| "[REDACTED]"),
            )
            .field("aes_key", &"[REDACTED]")
            .field("full_url", &self.full_url.as_ref().map(|_| "[REDACTED]"))
            .field("encode_type", &self.encode_type)
            .field("sample_rate", &self.sample_rate)
            .field("playtime", &self.playtime)
            .finish()
    }
}

impl fmt::Debug for WeixinPendingInboundPayload {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WeixinPendingInboundPayload")
            .field("schema_version", &self.schema_version)
            .field("payload_kind", &self.payload_kind)
            .field("account_id", &self.account_id)
            .field("peer_id_hash", &self.peer_id_hash)
            .field("message_id_hash", &self.message_id_hash)
            .field("item_id", &self.item_id)
            .field("direct_message_key", &self.direct_message_key)
            .field("created_at_millis", &self.created_at_millis)
            .field("context_reference_id", &self.context_reference_id)
            .field(
                "reply_to_user_id",
                &self.reply_to_user_id.as_ref().map(|_| "[REDACTED]"),
            )
            .field(
                "reply_context_token",
                &self.reply_context_token.as_ref().map(|_| "[REDACTED]"),
            )
            .field("text", &self.text.as_ref().map(|_| "[REDACTED]"))
            .field("voice", &self.voice)
            .finish()
    }
}

impl WeixinPendingInboundPayload {
    pub(crate) fn to_plaintext_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    pub(crate) fn from_plaintext_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    pub(crate) fn matches_bindings(
        &self,
        account_id: &str,
        peer_id_hash: &str,
        message_id_hash: &str,
        item_id: &str,
    ) -> bool {
        self.schema_version == WEIXIN_PENDING_PAYLOAD_SCHEMA_VERSION
            && self.account_id == account_id
            && self.peer_id_hash == peer_id_hash
            && self.message_id_hash == message_id_hash
            && self.item_id == item_id
    }
}

fn classify_message(
    self_user_id: Option<&SecretString>,
    message: &WeixinMessage,
) -> WeixinInboundKind {
    if message
        .group_id
        .as_ref()
        .is_some_and(|group_id| !group_id.is_empty())
    {
        return WeixinInboundKind::GroupMessage;
    }
    if self_user_id.is_some_and(|self_user_id| {
        !self_user_id.is_empty() && self_user_id.expose() == message.from_user_id.expose()
    }) {
        return WeixinInboundKind::SelfMessage;
    }
    if message.item_list.is_empty() {
        return WeixinInboundKind::Unknown;
    }
    if message.item_list.iter().any(|item| {
        item.item_type == TEXT_ITEM_TYPE
            && item
                .text_item
                .as_ref()
                .is_some_and(|text| !text.text.is_empty())
    }) {
        return WeixinInboundKind::Text;
    }
    if message.item_list.iter().any(|item| {
        item.item_type == VOICE_ITEM_TYPE && item.voice_item.as_ref().is_some_and(valid_voice_item)
    }) {
        return WeixinInboundKind::Voice;
    }
    WeixinInboundKind::UnsupportedAttachment
}

fn first_text_item(message: &WeixinMessage) -> Option<SecretString> {
    message.item_list.iter().find_map(|item| {
        if item.item_type != TEXT_ITEM_TYPE {
            return None;
        }
        item.text_item.as_ref().and_then(|text| {
            if text.text.is_empty() {
                None
            } else {
                Some(text.text.clone())
            }
        })
    })
}

fn first_voice_item(message: &WeixinMessage) -> Option<WeixinInboundVoice> {
    message.item_list.iter().find_map(|item| {
        if item.item_type != VOICE_ITEM_TYPE {
            return None;
        }
        let voice = item
            .voice_item
            .as_ref()
            .filter(|voice| valid_voice_item(voice))?;
        Some(WeixinInboundVoice {
            encrypt_query_param: voice.media.as_ref()?.encrypt_query_param.clone(),
            aes_key: voice.media.as_ref()?.aes_key.clone()?,
            full_url: voice.media.as_ref()?.full_url.clone(),
            encode_type: voice.encode_type,
            sample_rate: voice.sample_rate,
            playtime: voice.playtime,
        })
    })
}

fn valid_voice_item(voice: &VoiceItem) -> bool {
    let Some(media) = voice.media.as_ref() else {
        return false;
    };
    media
        .aes_key
        .as_ref()
        .is_some_and(|value| !value.is_empty())
        && (media
            .encrypt_query_param
            .as_ref()
            .is_some_and(|value| !value.is_empty())
            || media
                .full_url
                .as_ref()
                .is_some_and(|value| !value.is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WeixinMessageId;
    use crate::ilink::{CdnMedia, MessageItem, TextItem, VoiceItem};

    fn message() -> WeixinMessage {
        WeixinMessage {
            message_id: WeixinMessageId::new("raw-message-id"),
            from_user_id: SecretString::new("raw-peer-id"),
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
                voice_item: None,
                is_completed: Some(true),
                msg_id: None,
            }],
            context_token: Some(SecretString::new("context-token-secret")),
        }
    }

    fn voice_message() -> WeixinMessage {
        let mut message = message();
        message.item_list = vec![MessageItem {
            item_type: VOICE_ITEM_TYPE,
            text_item: None,
            voice_item: Some(VoiceItem {
                media: Some(CdnMedia {
                    encrypt_query_param: Some(SecretString::new("private-download-param")),
                    aes_key: Some(SecretString::new("private-aes-key")),
                    encrypt_type: Some(1),
                    full_url: None,
                }),
                encode_type: Some(6),
                bits_per_sample: Some(16),
                sample_rate: Some(24_000),
                playtime: Some(1_500),
                text: Some(SecretString::new("untrusted-weixin-transcript")),
            }),
            is_completed: Some(true),
            msg_id: None,
        }];
        message
    }

    #[test]
    fn envelope_hashes_peer_message_context_and_never_renders_raw_values() {
        let envelope = WeixinInboundEnvelope::from_message("account#933b5bde", None, &message(), 7);
        assert_eq!(envelope.kind, WeixinInboundKind::Text);
        assert_eq!(envelope.created_at_millis, 42);
        assert!(envelope.peer_id_hash.starts_with("peer#"));
        assert!(envelope.message_id_hash.starts_with("message#"));
        assert!(envelope.direct_message_key.starts_with("dm#"));
        assert!(
            envelope
                .context_reference_id
                .as_deref()
                .is_some_and(|value| value.starts_with("context#"))
        );
        let rendered = format!("{envelope:?}");
        for forbidden in [
            "raw-message-id",
            "raw-peer-id",
            "raw message body",
            "context-token-secret",
        ] {
            assert!(!rendered.contains(forbidden));
        }
    }

    #[test]
    fn recoverable_text_payload_contains_only_redacted_bindings_in_debug() {
        let envelope = WeixinInboundEnvelope::from_message("account#933b5bde", None, &message(), 7);
        let item_id = envelope.pending_item_id();
        let payload = envelope
            .recoverable_text_payload(&message(), &item_id)
            .expect("payload");
        assert_eq!(payload.payload_kind, WeixinInboundKind::Text);
        assert_eq!(payload.item_id, item_id);
        assert_eq!(
            payload.text.as_ref().expect("text").expose(),
            "raw message body"
        );
        let rendered = format!("{payload:?}");
        for forbidden in [
            "raw-message-id",
            "raw-peer-id",
            "raw message body",
            "context-token-secret",
        ] {
            assert!(!rendered.contains(forbidden));
        }
        let plaintext = payload.to_plaintext_bytes().expect("plaintext bytes");
        assert!(String::from_utf8_lossy(&plaintext).contains("raw message body"));
    }

    #[test]
    fn recoverable_voice_payload_redacts_media_secrets_and_ignores_remote_transcript() {
        let message = voice_message();
        let envelope = WeixinInboundEnvelope::from_message("account#933b5bde", None, &message, 7);
        assert_eq!(envelope.kind, WeixinInboundKind::Voice);
        let payload = envelope
            .recoverable_payload(&message, &envelope.pending_item_id())
            .expect("voice payload");
        assert_eq!(payload.payload_kind, WeixinInboundKind::Voice);
        assert!(payload.text.is_none());
        let voice = payload.voice.as_ref().expect("voice metadata");
        assert_eq!(voice.encode_type, Some(6));
        let rendered = format!("{payload:?}");
        for forbidden in [
            "private-download-param",
            "private-aes-key",
            "untrusted-weixin-transcript",
        ] {
            assert!(!rendered.contains(forbidden));
        }
        let plaintext =
            String::from_utf8(payload.to_plaintext_bytes().expect("plaintext")).expect("utf8");
        assert!(plaintext.contains("private-download-param"));
        assert!(plaintext.contains("private-aes-key"));
        assert!(!plaintext.contains("untrusted-weixin-transcript"));
    }

    #[test]
    fn envelope_classifies_group_self_attachment_and_unknown_messages() {
        let mut group = message();
        group.group_id = Some(SecretString::new("raw-group-id"));
        assert_eq!(
            WeixinInboundEnvelope::from_message("account#933b5bde", None, &group, 7).kind,
            WeixinInboundKind::GroupMessage
        );

        let mut self_message = message();
        self_message.from_user_id = SecretString::new("bot-user-id");
        self_message.message_state = Some(2);
        assert_eq!(
            WeixinInboundEnvelope::from_message(
                "account#933b5bde",
                Some(&SecretString::new("bot-user-id")),
                &self_message,
                7,
            )
            .kind,
            WeixinInboundKind::SelfMessage
        );

        let mut state_two_peer_text = message();
        state_two_peer_text.message_state = Some(2);
        assert_eq!(
            WeixinInboundEnvelope::from_message("account#933b5bde", None, &state_two_peer_text, 7)
                .kind,
            WeixinInboundKind::Text
        );

        let mut attachment = message();
        attachment.item_list[0].item_type = 3;
        assert_eq!(
            WeixinInboundEnvelope::from_message("account#933b5bde", None, &attachment, 7).kind,
            WeixinInboundKind::UnsupportedAttachment
        );

        let mut unknown = message();
        unknown.item_list.clear();
        assert_eq!(
            WeixinInboundEnvelope::from_message("account#933b5bde", None, &unknown, 7).kind,
            WeixinInboundKind::Unknown
        );
    }
}

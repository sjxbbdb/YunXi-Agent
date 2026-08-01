use serde::{Deserialize, Serialize};

use crate::{SecretString, WeixinMessageId};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BaseInfo {
    pub channel_version: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub bot_agent: String,
}

impl Default for BaseInfo {
    fn default() -> Self {
        Self {
            channel_version: env!("CARGO_PKG_VERSION").to_string(),
            bot_agent: String::new(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct GetBotQrCodeRequest {
    #[serde(default)]
    pub local_token_list: Vec<SecretString>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GetBotQrCodeResponse {
    pub qrcode: SecretString,
    pub qrcode_img_content: SecretString,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum QrCodeStatus {
    #[serde(rename = "wait")]
    Wait,
    #[serde(rename = "scaned")]
    Scanned,
    #[serde(rename = "confirmed")]
    Confirmed,
    #[serde(rename = "expired")]
    Expired,
    #[serde(rename = "scaned_but_redirect")]
    ScannedButRedirect,
    #[serde(rename = "need_verifycode")]
    NeedVerifyCode,
    #[serde(rename = "verify_code_blocked")]
    VerifyCodeBlocked,
    #[serde(rename = "binded_redirect")]
    BoundRedirect,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GetQrCodeStatusResponse {
    pub status: QrCodeStatus,
    #[serde(default)]
    pub bot_token: Option<SecretString>,
    #[serde(default)]
    pub ilink_bot_id: Option<SecretString>,
    #[serde(default)]
    pub baseurl: Option<String>,
    #[serde(default)]
    pub ilink_user_id: Option<SecretString>,
    #[serde(default)]
    pub redirect_host: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TextItem {
    pub text: SecretString,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MessageItem {
    #[serde(rename = "type")]
    pub item_type: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_item: Option<TextItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_completed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg_id: Option<WeixinMessageId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WeixinMessage {
    #[serde(skip_serializing_if = "WeixinMessageId::is_empty")]
    pub message_id: WeixinMessageId,
    pub from_user_id: SecretString,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_user_id: Option<SecretString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub create_time_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SecretString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<SecretString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_type: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_state: Option<u32>,
    #[serde(default)]
    pub item_list: Vec<MessageItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_token: Option<SecretString>,
}

impl WeixinMessage {
    pub fn reply_target_id(&self) -> SecretString {
        if let Some(group_id) = self
            .group_id
            .clone()
            .filter(|group_id| !group_id.is_empty())
        {
            return group_id;
        }
        if self.context_token.is_some() {
            return self.from_user_id.clone();
        }
        self.to_user_id
            .clone()
            .filter(|target| !target.is_empty())
            .unwrap_or_else(|| self.from_user_id.clone())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct WeixinUpdateSender {
    #[serde(default)]
    pub user_id: SecretString,
    #[serde(default)]
    pub user_name: Option<SecretString>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct WeixinUpdateMessage {
    #[serde(default)]
    pub message_id: Option<WeixinMessageId>,
    #[serde(default)]
    pub chat_id: Option<SecretString>,
    #[serde(default)]
    pub chat_type: Option<String>,
    #[serde(default)]
    pub from: Option<WeixinUpdateSender>,
    #[serde(default)]
    pub text: Option<SecretString>,
    #[serde(default)]
    pub timestamp: Option<u64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct WeixinUpdate {
    #[serde(default)]
    pub update_id: Option<i64>,
    #[serde(default)]
    pub update_type: Option<String>,
    #[serde(default)]
    pub message: Option<WeixinUpdateMessage>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GetUpdatesRequest {
    pub get_updates_buf: SecretString,
    pub base_info: BaseInfo,
}

impl GetUpdatesRequest {
    pub fn new(get_updates_buf: impl Into<String>) -> Self {
        Self {
            get_updates_buf: SecretString::new(get_updates_buf),
            base_info: BaseInfo::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct GetUpdatesResponse {
    #[serde(default)]
    pub ret: i64,
    #[serde(default)]
    pub errcode: Option<i64>,
    #[serde(default)]
    pub errmsg: Option<SecretString>,
    #[serde(default)]
    pub msgs: Vec<WeixinMessage>,
    #[serde(default)]
    pub updates: Vec<WeixinUpdate>,
    #[serde(default)]
    pub has_more: Option<bool>,
    #[serde(default)]
    pub get_updates_buf: Option<SecretString>,
    #[serde(default)]
    pub longpolling_timeout_ms: Option<u64>,
}

impl GetUpdatesResponse {
    pub fn inbound_messages(&self) -> Vec<WeixinMessage> {
        let mut messages = self.msgs.clone();
        messages.extend(self.updates.iter().filter_map(WeixinUpdate::to_message));
        messages
    }
}

impl WeixinUpdate {
    fn to_message(&self) -> Option<WeixinMessage> {
        if self
            .update_type
            .as_deref()
            .is_some_and(|update_type| update_type != "message")
        {
            return None;
        }
        let message = self.message.as_ref()?;
        let from = message.from.as_ref()?;
        if from.user_id.is_empty() {
            return None;
        }
        let message_id = message.message_id.clone().unwrap_or_else(|| {
            WeixinMessageId::new(format!(
                "update-{}",
                self.update_id
                    .map(|update_id| update_id.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            ))
        });
        let text = message.text.clone().filter(|text| !text.is_empty());
        let item_list = text
            .map(|text| {
                vec![MessageItem {
                    item_type: 1,
                    text_item: Some(TextItem { text }),
                    is_completed: Some(true),
                    msg_id: None,
                }]
            })
            .unwrap_or_default();
        let chat_type = message.chat_type.as_deref().unwrap_or_default();
        let group_id = matches!(chat_type, "group" | "chatroom")
            .then(|| message.chat_id.clone())
            .flatten();
        Some(WeixinMessage {
            message_id,
            from_user_id: from.user_id.clone(),
            to_user_id: message
                .chat_id
                .clone()
                .filter(|chat_id| !chat_id.is_empty()),
            client_id: None,
            create_time_ms: message.timestamp.map(normalize_update_timestamp_millis),
            session_id: None,
            group_id,
            message_type: Some(1),
            message_state: None,
            item_list,
            context_token: None,
        })
    }
}

fn normalize_update_timestamp_millis(timestamp: u64) -> u64 {
    if timestamp < 10_000_000_000 {
        timestamp.saturating_mul(1000)
    } else {
        timestamp
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub msg: WeixinMessage,
    pub base_info: BaseInfo,
}

impl SendMessageRequest {
    pub fn new(msg: WeixinMessage) -> Self {
        Self {
            msg,
            base_info: BaseInfo {
                channel_version: env!("CARGO_PKG_VERSION").to_string(),
                bot_agent: String::new(),
            },
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SendMessageResponse {
    #[serde(default)]
    pub ret: i64,
    #[serde(default)]
    pub errmsg: Option<SecretString>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum TypingStatus {
    Typing = 1,
    Cancel = 2,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SendTypingRequest {
    pub ilink_user_id: SecretString,
    pub typing_ticket: SecretString,
    pub status: u8,
    pub base_info: BaseInfo,
}

impl SendTypingRequest {
    pub fn new(
        ilink_user_id: impl Into<String>,
        typing_ticket: impl Into<String>,
        status: TypingStatus,
    ) -> Self {
        Self {
            ilink_user_id: SecretString::new(ilink_user_id),
            typing_ticket: SecretString::new(typing_ticket),
            status: status as u8,
            base_info: BaseInfo::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum UploadMediaType {
    Image = 1,
    Video = 2,
    File = 3,
    Voice = 4,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GetUploadUrlRequest {
    pub filekey: String,
    pub media_type: u8,
    pub to_user_id: SecretString,
    pub rawsize: u64,
    pub rawfilemd5: String,
    pub filesize: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumb_rawsize: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumb_rawfilemd5: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumb_filesize: Option<u64>,
    #[serde(default)]
    pub no_need_thumb: bool,
    pub aeskey: SecretString,
    pub base_info: BaseInfo,
}

impl GetUploadUrlRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        filekey: impl Into<String>,
        media_type: UploadMediaType,
        to_user_id: impl Into<String>,
        rawsize: u64,
        rawfilemd5: impl Into<String>,
        filesize: u64,
        aeskey: impl Into<String>,
    ) -> Self {
        Self {
            filekey: filekey.into(),
            media_type: media_type as u8,
            to_user_id: SecretString::new(to_user_id),
            rawsize,
            rawfilemd5: rawfilemd5.into(),
            filesize,
            thumb_rawsize: None,
            thumb_rawfilemd5: None,
            thumb_filesize: None,
            no_need_thumb: false,
            aeskey: SecretString::new(aeskey),
            base_info: BaseInfo::default(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GetUploadUrlResponse {
    #[serde(default)]
    pub upload_param: Option<SecretString>,
    #[serde(default)]
    pub thumb_upload_param: Option<SecretString>,
    pub upload_full_url: SecretString,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn getupdates_flattens_legacy_msgs_and_update_messages() {
        let response = GetUpdatesResponse {
            msgs: vec![WeixinMessage {
                message_id: WeixinMessageId::new("legacy-message"),
                from_user_id: SecretString::new("legacy-user"),
                to_user_id: None,
                client_id: None,
                create_time_ms: Some(1),
                session_id: None,
                group_id: None,
                message_type: Some(1),
                message_state: None,
                item_list: vec![MessageItem {
                    item_type: 1,
                    text_item: Some(TextItem {
                        text: SecretString::new("legacy text"),
                    }),
                    is_completed: Some(true),
                    msg_id: None,
                }],
                context_token: None,
            }],
            updates: vec![WeixinUpdate {
                update_id: Some(42),
                update_type: Some("message".to_string()),
                message: Some(WeixinUpdateMessage {
                    message_id: Some(WeixinMessageId::new("update-message")),
                    chat_id: Some(SecretString::new("dm-chat")),
                    chat_type: Some("private".to_string()),
                    from: Some(WeixinUpdateSender {
                        user_id: SecretString::new("update-user"),
                        user_name: Some(SecretString::new("update-name")),
                    }),
                    text: Some(SecretString::new("update text")),
                    timestamp: Some(1_785_500_000),
                }),
            }],
            ..GetUpdatesResponse::default()
        };

        let messages = response.inbound_messages();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].message_id.as_str(), "legacy-message");
        assert_eq!(messages[1].message_id.as_str(), "update-message");
        assert_eq!(messages[1].from_user_id.expose(), "update-user");
        assert_eq!(
            messages[1].item_list[0]
                .text_item
                .as_ref()
                .expect("text")
                .text
                .expose(),
            "update text"
        );
        assert_eq!(messages[1].create_time_ms, Some(1_785_500_000_000));
    }

    #[test]
    fn getupdates_marks_update_group_chat_as_group_id() {
        let response = GetUpdatesResponse {
            updates: vec![WeixinUpdate {
                update_id: Some(7),
                update_type: Some("message".to_string()),
                message: Some(WeixinUpdateMessage {
                    message_id: None,
                    chat_id: Some(SecretString::new("room-id")),
                    chat_type: Some("group".to_string()),
                    from: Some(WeixinUpdateSender {
                        user_id: SecretString::new("member-id"),
                        user_name: None,
                    }),
                    text: Some(SecretString::new("hello group")),
                    timestamp: Some(1_785_500_000_001),
                }),
            }],
            ..GetUpdatesResponse::default()
        };

        let messages = response.inbound_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].message_id.as_str(), "update-7");
        assert_eq!(
            messages[0].group_id.as_ref().map(SecretString::expose),
            Some("room-id")
        );
        assert_eq!(messages[0].create_time_ms, Some(1_785_500_000_001));
    }

    #[test]
    fn update_private_message_uses_chat_id_as_reply_target() {
        let response = GetUpdatesResponse {
            updates: vec![WeixinUpdate {
                update_id: Some(7),
                update_type: Some("message".to_string()),
                message: Some(WeixinUpdateMessage {
                    message_id: Some(WeixinMessageId::new("update-private")),
                    chat_id: Some(SecretString::new("dm-chat-target")),
                    chat_type: Some("private".to_string()),
                    from: Some(WeixinUpdateSender {
                        user_id: SecretString::new("sender-user"),
                        user_name: None,
                    }),
                    text: Some(SecretString::new("hello")),
                    timestamp: Some(1_785_500_000),
                }),
            }],
            ..GetUpdatesResponse::default()
        };

        let messages = response.inbound_messages();

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].from_user_id.expose(), "sender-user");
        assert_eq!(messages[0].reply_target_id().expose(), "dm-chat-target");
    }

    #[test]
    fn message_private_reply_uses_sender_not_bot_target() {
        let message = WeixinMessage {
            message_id: WeixinMessageId::new("message-private"),
            from_user_id: SecretString::new("sender-user"),
            to_user_id: Some(SecretString::new("bot-account")),
            client_id: None,
            create_time_ms: None,
            session_id: None,
            group_id: None,
            message_type: Some(1),
            message_state: None,
            item_list: Vec::new(),
            context_token: Some(SecretString::new("context-token")),
        };

        assert_eq!(message.reply_target_id().expose(), "sender-user");
    }

    #[test]
    fn message_group_reply_uses_group_id() {
        let message = WeixinMessage {
            message_id: WeixinMessageId::new("message-group"),
            from_user_id: SecretString::new("sender-user"),
            to_user_id: Some(SecretString::new("bot-account")),
            client_id: None,
            create_time_ms: None,
            session_id: None,
            group_id: Some(SecretString::new("room-id")),
            message_type: Some(1),
            message_state: None,
            item_list: Vec::new(),
            context_token: Some(SecretString::new("context-token")),
        };

        assert_eq!(message.reply_target_id().expose(), "room-id");
    }

    #[test]
    fn base_info_matches_minimal_reasonix_shape() {
        let value = serde_json::to_value(BaseInfo::default()).expect("base info JSON");

        assert_eq!(
            value["channel_version"].as_str(),
            Some(env!("CARGO_PKG_VERSION"))
        );
        assert!(value.get("bot_agent").is_none());
    }
}

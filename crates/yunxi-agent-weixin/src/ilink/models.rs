use serde::{Deserialize, Serialize};

use crate::{SecretString, WeixinMessageId};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BaseInfo {
    pub channel_version: String,
    pub bot_agent: String,
}

impl Default for BaseInfo {
    fn default() -> Self {
        Self {
            channel_version: env!("CARGO_PKG_VERSION").to_string(),
            bot_agent: format!("YunXi/{}", env!("CARGO_PKG_VERSION")),
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
    #[serde(default)]
    pub text_item: Option<TextItem>,
    #[serde(default)]
    pub is_completed: Option<bool>,
    #[serde(default)]
    pub msg_id: Option<WeixinMessageId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WeixinMessage {
    pub message_id: WeixinMessageId,
    pub from_user_id: SecretString,
    #[serde(default)]
    pub to_user_id: Option<SecretString>,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub create_time_ms: Option<u64>,
    #[serde(default)]
    pub session_id: Option<SecretString>,
    #[serde(default)]
    pub group_id: Option<SecretString>,
    #[serde(default)]
    pub message_type: Option<u32>,
    #[serde(default)]
    pub message_state: Option<u32>,
    #[serde(default)]
    pub item_list: Vec<MessageItem>,
    #[serde(default)]
    pub context_token: Option<SecretString>,
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
    pub get_updates_buf: Option<SecretString>,
    #[serde(default)]
    pub longpolling_timeout_ms: Option<u64>,
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
            base_info: BaseInfo::default(),
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

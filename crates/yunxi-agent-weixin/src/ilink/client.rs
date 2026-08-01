use futures_util::StreamExt;
use reqwest::{Client, Method, Response, Url, redirect::Policy};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::fmt;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::timeout;

use crate::error::{RequestContext, WeixinApiError};
use crate::redaction::{SecretString, redacted_identifier};

use super::models::{
    GetBotQrCodeRequest, GetBotQrCodeResponse, GetQrCodeStatusResponse, GetUpdatesRequest,
    GetUpdatesResponse, GetUploadUrlRequest, GetUploadUrlResponse, SendMessageRequest,
    SendMessageResponse, SendTypingRequest,
};
use super::{poll, qr, send};

pub const PRODUCTION_ILINK_ENDPOINT: &str = "https://ilinkai.weixin.qq.com/";

const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const ILINK_APP_ID: &str = "bot";
const ILINK_APP_CLIENT_VERSION: &str = "131330";
static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub struct IlinkHttpClient {
    http: Client,
    base_url: Url,
    account_alias: String,
    token: Option<SecretString>,
    request_timeout: Duration,
    max_response_bytes: usize,
}

impl fmt::Debug for IlinkHttpClient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("IlinkHttpClient")
            .field("endpoint", &self.base_url.origin().ascii_serialization())
            .field(
                "account",
                &redacted_identifier("account", &self.account_alias),
            )
            .field("token", &self.token.as_ref().map(|_| "[REDACTED]"))
            .field("request_timeout", &self.request_timeout)
            .field("max_response_bytes", &self.max_response_bytes)
            .finish()
    }
}

impl IlinkHttpClient {
    pub fn new(
        account_alias: impl Into<String>,
        token: Option<SecretString>,
    ) -> Result<Self, WeixinApiError> {
        Self::build(
            PRODUCTION_ILINK_ENDPOINT,
            account_alias.into(),
            token,
            DEFAULT_REQUEST_TIMEOUT,
            DEFAULT_MAX_RESPONSE_BYTES,
            false,
        )
    }

    #[doc(hidden)]
    pub fn new_for_test(
        base_url: &str,
        account_alias: impl Into<String>,
        token: Option<SecretString>,
        request_timeout: Duration,
        max_response_bytes: usize,
    ) -> Result<Self, WeixinApiError> {
        Self::build(
            base_url,
            account_alias.into(),
            token,
            request_timeout,
            max_response_bytes,
            true,
        )
    }

    fn build(
        base_url: &str,
        account_alias: String,
        token: Option<SecretString>,
        request_timeout: Duration,
        max_response_bytes: usize,
        allow_test_endpoint: bool,
    ) -> Result<Self, WeixinApiError> {
        let context = RequestContext::new("wx-client-init", "client_init", &account_alias);
        let mut base_url = Url::parse(base_url).map_err(|_| WeixinApiError::Protocol {
            context: context.clone(),
        })?;
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        if allow_test_endpoint {
            if !is_loopback_url(&base_url) {
                return Err(WeixinApiError::Protocol { context });
            }
        } else if base_url.as_str() != PRODUCTION_ILINK_ENDPOINT {
            return Err(WeixinApiError::Protocol { context });
        }
        if request_timeout.is_zero() || max_response_bytes == 0 {
            return Err(WeixinApiError::Protocol { context });
        }
        let http = Client::builder()
            .redirect(Policy::none())
            .build()
            .map_err(|_| WeixinApiError::Protocol { context })?;
        Ok(Self {
            http,
            base_url,
            account_alias,
            token,
            request_timeout,
            max_response_bytes,
        })
    }

    pub async fn fetch_qr_code(
        &self,
        request: GetBotQrCodeRequest,
    ) -> Result<GetBotQrCodeResponse, WeixinApiError> {
        let mut url = self.endpoint(qr::FETCH_QR_CODE_PATH, "fetch_qr_code")?;
        url.query_pairs_mut()
            .append_pair("bot_type", qr::DEFAULT_BOT_TYPE);
        self.request_json(Method::POST, url, Some(&request), false, "fetch_qr_code")
            .await
    }

    pub async fn poll_qr_status(
        &self,
        qrcode: &SecretString,
        verify_code: Option<&SecretString>,
    ) -> Result<GetQrCodeStatusResponse, WeixinApiError> {
        let mut url = self.endpoint(qr::POLL_QR_STATUS_PATH, "poll_qr_status")?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("qrcode", qrcode.expose());
            if let Some(verify_code) = verify_code {
                query.append_pair("verify_code", verify_code.expose());
            }
        }
        self.request_json::<(), _>(Method::GET, url, None, false, "poll_qr_status")
            .await
    }

    pub async fn get_updates(
        &self,
        mut request: GetUpdatesRequest,
    ) -> Result<GetUpdatesResponse, WeixinApiError> {
        request.base_info = Default::default();
        let url = self.endpoint(poll::GET_UPDATES_PATH, "get_updates")?;
        self.request_json(Method::POST, url, Some(&request), true, "get_updates")
            .await
    }

    pub async fn send_message(
        &self,
        mut request: SendMessageRequest,
    ) -> Result<SendMessageResponse, WeixinApiError> {
        request.base_info = Default::default();
        let url = self.endpoint(send::SEND_MESSAGE_PATH, "send_message")?;
        self.request_json(Method::POST, url, Some(&request), true, "send_message")
            .await
    }

    pub async fn send_typing(
        &self,
        mut request: SendTypingRequest,
    ) -> Result<Value, WeixinApiError> {
        request.base_info = Default::default();
        let url = self.endpoint(send::SEND_TYPING_PATH, "send_typing")?;
        self.request_json(Method::POST, url, Some(&request), true, "send_typing")
            .await
    }

    pub async fn get_upload_url(
        &self,
        mut request: GetUploadUrlRequest,
    ) -> Result<GetUploadUrlResponse, WeixinApiError> {
        request.base_info = Default::default();
        let url = self.endpoint(send::GET_UPLOAD_URL_PATH, "get_upload_url")?;
        self.request_json(Method::POST, url, Some(&request), true, "get_upload_url")
            .await
    }

    fn endpoint(&self, path: &str, operation: &'static str) -> Result<Url, WeixinApiError> {
        self.base_url
            .join(path)
            .map_err(|_| WeixinApiError::Protocol {
                context: RequestContext::new(next_request_id(), operation, &self.account_alias),
            })
    }

    async fn request_json<TRequest, TResponse>(
        &self,
        method: Method,
        url: Url,
        body: Option<&TRequest>,
        authenticated: bool,
        operation: &'static str,
    ) -> Result<TResponse, WeixinApiError>
    where
        TRequest: Serialize + ?Sized,
        TResponse: DeserializeOwned,
    {
        let context = RequestContext::new(next_request_id(), operation, &self.account_alias);
        let mut request = self
            .http
            .request(method, url)
            .timeout(self.request_timeout)
            .header("Content-Type", "application/json")
            .header("AuthorizationType", "ilink_bot_token")
            .header("X-WECHAT-UIN", next_wechat_uin())
            .header("iLink-App-Id", ILINK_APP_ID)
            .header("iLink-App-ClientVersion", ILINK_APP_CLIENT_VERSION)
            .header("X-YunXi-Request-Id", &context.request_id);
        if authenticated {
            if let Some(token) = self.token.as_ref().filter(|token| !token.is_empty()) {
                request = request.header("Authorization", format!("Bearer {}", token.expose()));
            }
        }
        if let Some(body) = body {
            let body = serde_json::to_vec(body).map_err(|_| WeixinApiError::Protocol {
                context: context.clone(),
            })?;
            request = request.body(body);
        }
        let response = match timeout(self.request_timeout, request.send()).await {
            Err(_) => return Err(WeixinApiError::Timeout { context }),
            Ok(Err(error)) if error.is_timeout() => {
                return Err(WeixinApiError::Timeout { context });
            }
            Ok(Err(error)) => {
                return Err(WeixinApiError::Network {
                    context,
                    category: if error.is_connect() {
                        "connect"
                    } else {
                        "network"
                    },
                });
            }
            Ok(Ok(response)) => response,
        };
        self.decode_response(response, context).await
    }

    async fn decode_response<TResponse>(
        &self,
        response: Response,
        context: RequestContext,
    ) -> Result<TResponse, WeixinApiError>
    where
        TResponse: DeserializeOwned,
    {
        let status = response.status();
        if !status.is_success() {
            return Err(WeixinApiError::HttpStatus {
                context,
                status: status.as_u16(),
            });
        }
        if response
            .content_length()
            .is_some_and(|length| length > self.max_response_bytes as u64)
        {
            return Err(WeixinApiError::ResponseTooLarge {
                context,
                limit_bytes: self.max_response_bytes,
            });
        }
        let mut bytes = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| WeixinApiError::Network {
                context: context.clone(),
                category: "response_body",
            })?;
            if bytes.len().saturating_add(chunk.len()) > self.max_response_bytes {
                return Err(WeixinApiError::ResponseTooLarge {
                    context,
                    limit_bytes: self.max_response_bytes,
                });
            }
            bytes.extend_from_slice(&chunk);
        }
        let value: Value =
            serde_json::from_slice(&bytes).map_err(|_| WeixinApiError::InvalidJson {
                context: context.clone(),
            })?;
        if let Some(code) = api_error_code(&value) {
            return Err(WeixinApiError::Api { context, code });
        }
        serde_json::from_value(value).map_err(|_| WeixinApiError::Protocol { context })
    }
}

fn api_error_code(value: &Value) -> Option<i64> {
    ["errcode", "ret"]
        .into_iter()
        .filter_map(|key| value.get(key).and_then(Value::as_i64))
        .find(|code| *code != 0)
}

fn next_request_id() -> String {
    let sequence = REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("wx-{sequence:016x}")
}

fn next_wechat_uin() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    let sequence = REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed) as u32;
    base64_ascii(&(nanos ^ sequence).to_string())
}

fn base64_ascii(value: &str) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = value.as_bytes();
    let mut output = String::new();
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or_default();
        let third = chunk.get(2).copied().unwrap_or_default();
        output.push(TABLE[(first >> 2) as usize] as char);
        output.push(TABLE[(((first & 0x03) << 4) | (second >> 4)) as usize] as char);
        if chunk.len() > 1 {
            output.push(TABLE[(((second & 0x0f) << 2) | (third >> 6)) as usize] as char);
        } else {
            output.push('=');
        }
        if chunk.len() > 2 {
            output.push(TABLE[(third & 0x3f) as usize] as char);
        } else {
            output.push('=');
        }
    }
    output
}

fn is_loopback_url(url: &Url) -> bool {
    if !matches!(url.scheme(), "http" | "https") {
        return false;
    }
    match url.host_str() {
        Some("localhost") => true,
        Some(host) => host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback()),
        None => false,
    }
}

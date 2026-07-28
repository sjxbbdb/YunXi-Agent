use serde_json::Value;
use std::time::Duration;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use yunxi_agent_weixin::ilink::{GetBotQrCodeRequest, GetUpdatesRequest};
use yunxi_agent_weixin::{IlinkHttpClient, SecretString, WeixinApiError};

async fn client(server: &MockServer, timeout: Duration, max_bytes: usize) -> IlinkHttpClient {
    IlinkHttpClient::new_for_test(
        &server.uri(),
        "private-account-name",
        Some(SecretString::new("token-super-secret")),
        timeout,
        max_bytes,
    )
    .expect("loopback mock endpoint")
}

#[tokio::test]
async fn get_updates_sends_required_headers_and_preserves_cursor() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ilink/bot/getupdates"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ret": 0,
            "msgs": [{
                "message_id": "msg-1",
                "from_user_id": "raw-user-id",
                "item_list": []
            }],
            "get_updates_buf": "cursor-next"
        })))
        .mount(&server)
        .await;

    let client = client(&server, Duration::from_secs(1), 16 * 1024).await;
    let response = client
        .get_updates(GetUpdatesRequest::new("cursor-current"))
        .await
        .expect("mock poll response");
    assert_eq!(response.msgs[0].message_id.as_str(), "msg-1");

    let requests = server.received_requests().await.expect("recorded requests");
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert_eq!(
        request
            .headers
            .get("Authorization")
            .expect("authorization header")
            .to_str()
            .expect("authorization text"),
        "Bearer token-super-secret"
    );
    assert_eq!(
        request
            .headers
            .get("AuthorizationType")
            .expect("authorization type")
            .to_str()
            .expect("authorization type text"),
        "ilink_bot_token"
    );
    assert!(request.headers.contains_key("X-WECHAT-UIN"));
    assert!(request.headers.contains_key("X-YunXi-Request-Id"));
    assert_eq!(
        request
            .headers
            .get("iLink-App-Id")
            .expect("app id")
            .to_str()
            .expect("app id text"),
        "bot"
    );
    let body: Value = serde_json::from_slice(&request.body).expect("request JSON");
    assert_eq!(body["get_updates_buf"].as_str(), Some("cursor-current"));
    assert_eq!(
        body["base_info"]["channel_version"].as_str(),
        Some(env!("CARGO_PKG_VERSION"))
    );
}

#[tokio::test]
async fn api_error_and_invalid_json_are_normalized_without_secret_content() {
    let api_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ilink/bot/getupdates"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ret": -14,
            "errmsg": "token-super-secret raw-user-id raw message content"
        })))
        .mount(&api_server)
        .await;
    let error = client(&api_server, Duration::from_secs(1), 4096)
        .await
        .get_updates(GetUpdatesRequest::new("cursor-secret"))
        .await
        .expect_err("API error");
    assert!(matches!(error, WeixinApiError::Api { code: -14, .. }));
    let rendered = error.to_string();
    for forbidden in [
        "token-super-secret",
        "raw-user-id",
        "raw message content",
        "cursor-secret",
        "private-account-name",
    ] {
        assert!(!rendered.contains(forbidden));
    }

    let json_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ilink/bot/getupdates"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{not-json"))
        .mount(&json_server)
        .await;
    let error = client(&json_server, Duration::from_secs(1), 4096)
        .await
        .get_updates(GetUpdatesRequest::new("cursor-secret"))
        .await
        .expect_err("invalid JSON");
    assert!(matches!(error, WeixinApiError::InvalidJson { .. }));
}

#[tokio::test]
async fn timeout_and_response_size_limit_are_enforced() {
    let timeout_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ilink/bot/get_qrcode_status"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(100))
                .set_body_json(serde_json::json!({"status": "wait"})),
        )
        .mount(&timeout_server)
        .await;
    let error = client(&timeout_server, Duration::from_millis(20), 4096)
        .await
        .poll_qr_status(&SecretString::new("qr-payload-secret"), None)
        .await
        .expect_err("timeout");
    assert!(matches!(error, WeixinApiError::Timeout { .. }));
    assert!(!error.to_string().contains("qr-payload-secret"));

    let size_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ilink/bot/get_bot_qrcode"))
        .respond_with(ResponseTemplate::new(200).set_body_string(format!(
            "{{\"qrcode\":\"{}\",\"qrcode_img_content\":\"url\"}}",
            "x".repeat(512)
        )))
        .mount(&size_server)
        .await;
    let error = client(&size_server, Duration::from_secs(1), 128)
        .await
        .fetch_qr_code(GetBotQrCodeRequest::default())
        .await
        .expect_err("response size limit");
    assert!(matches!(error, WeixinApiError::ResponseTooLarge { .. }));
}

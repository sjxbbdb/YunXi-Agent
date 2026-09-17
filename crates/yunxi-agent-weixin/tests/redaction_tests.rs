use serde_json::json;
use std::time::Duration;
use yunxi_agent_weixin::ilink::{GetUpdatesRequest, MessageItem, TextItem, WeixinMessage};
use yunxi_agent_weixin::{IlinkHttpClient, SecretString, WeixinMessageId, redacted_json_snapshot};

#[test]
fn secrets_are_redacted_from_debug_display_and_diagnostic_json() {
    let secret = SecretString::new("token-super-secret");
    assert_eq!(format!("{secret:?}"), "[REDACTED]");
    assert_eq!(secret.to_string(), "[REDACTED]");

    let message = WeixinMessage {
        message_id: WeixinMessageId::new("message-1"),
        from_user_id: SecretString::new("raw-user-id"),
        to_user_id: Some(SecretString::new("raw-peer-id")),
        client_id: None,
        create_time_ms: None,
        session_id: None,
        group_id: None,
        message_type: Some(1),
        message_state: None,
        item_list: vec![MessageItem {
            item_type: 1,
            text_item: Some(TextItem {
                text: SecretString::new("raw message content"),
            }),
            voice_item: None,
            is_completed: Some(true),
            msg_id: None,
        }],
        context_token: Some(SecretString::new("context-token-secret")),
    };
    let debug = format!("{message:?}");
    for forbidden in [
        "raw-user-id",
        "raw-peer-id",
        "raw message content",
        "context-token-secret",
    ] {
        assert!(!debug.contains(forbidden));
    }

    let snapshot = redacted_json_snapshot(&json!({
        "token": "token-super-secret",
        "qrcode": "qr-payload-secret",
        "context_token": "context-token-secret",
        "from_user_id": "raw-user-id",
        "text": "raw message content",
        "safe": "visible"
    }))
    .expect("redacted JSON snapshot");
    let snapshot = serde_json::to_string(&snapshot).expect("snapshot JSON");
    for forbidden in [
        "token-super-secret",
        "qr-payload-secret",
        "context-token-secret",
        "raw-user-id",
        "raw message content",
    ] {
        assert!(!snapshot.contains(forbidden));
    }
    assert!(snapshot.contains("visible"));

    let request_snapshot =
        redacted_json_snapshot(&GetUpdatesRequest::new("cursor-secret")).expect("request snapshot");
    assert!(!request_snapshot.to_string().contains("cursor-secret"));
}

#[test]
fn client_and_configuration_errors_do_not_expose_account_or_token() {
    let client = IlinkHttpClient::new(
        "private-account-name",
        Some(SecretString::new("token-super-secret")),
    )
    .expect("production client");
    let debug = format!("{client:?}");
    assert!(!debug.contains("private-account-name"));
    assert!(!debug.contains("token-super-secret"));

    let error = IlinkHttpClient::new_for_test(
        "https://example.com/",
        "private-account-name",
        Some(SecretString::new("token-super-secret")),
        Duration::from_millis(10),
        1024,
    )
    .expect_err("test endpoints must be loopback-only");
    let rendered = format!("{error:#}");
    assert!(!rendered.contains("private-account-name"));
    assert!(!rendered.contains("token-super-secret"));
}

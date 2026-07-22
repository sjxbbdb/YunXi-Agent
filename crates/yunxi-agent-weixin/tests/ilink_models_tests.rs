use serde_json::json;
use yunxi_agent_weixin::ilink::{GetBotQrCodeResponse, GetUpdatesResponse};

#[test]
fn message_id_accepts_number_or_string_and_unknown_fields() {
    for (message_id, expected) in [
        (json!(900719925474099_u64), "900719925474099"),
        (json!("msg-7"), "msg-7"),
    ] {
        let response: GetUpdatesResponse = serde_json::from_value(json!({
            "ret": 0,
            "msgs": [{
                "message_id": message_id,
                "from_user_id": "raw-user-id",
                "message_type": 1,
                "item_list": [],
                "future_non_breaking_field": {"enabled": true}
            }],
            "get_updates_buf": "cursor-next",
            "unknown_top_level": 42
        }))
        .expect("number and string message ids should decode");

        assert_eq!(response.msgs[0].message_id.as_str(), expected);
    }
}

#[test]
fn missing_critical_fields_fail_safely() {
    let missing_message_id = serde_json::from_value::<GetUpdatesResponse>(json!({
        "ret": 0,
        "msgs": [{"from_user_id": "raw-user-id"}]
    }));
    assert!(missing_message_id.is_err());

    let missing_qr_payload = serde_json::from_value::<GetBotQrCodeResponse>(json!({
        "qrcode_img_content": "https://example.invalid/qr"
    }));
    assert!(missing_qr_payload.is_err());
}

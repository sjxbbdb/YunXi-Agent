use serde_json::json;
use yunxi_agent_weixin::ilink::{GetBotQrCodeResponse, GetUpdatesResponse, GetUploadUrlResponse};

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

#[test]
fn voice_item_preserves_encrypted_cdn_metadata() {
    let response: GetUpdatesResponse = serde_json::from_value(json!({
        "ret": 0,
        "msgs": [{
            "message_id": "voice-message",
            "from_user_id": "raw-user-id",
            "message_type": 1,
            "item_list": [{
                "type": 3,
                "voice_item": {
                    "media": {
                        "encrypt_query_param": "private-download-param",
                        "aes_key": "MDEyMzQ1Njc4OWFiY2RlZg==",
                        "encrypt_type": 1,
                        "full_url": "https://novac2c.cdn.weixin.qq.com/c2c/download?id=private"
                    },
                    "encode_type": 6,
                    "bits_per_sample": 16,
                    "sample_rate": 24000,
                    "playtime": 1234,
                    "text": "微信侧转写不作为本地识别结果"
                }
            }]
        }]
    }))
    .expect("voice message should decode");

    let voice = response.msgs[0].item_list[0]
        .voice_item
        .as_ref()
        .expect("voice item");
    assert_eq!(voice.encode_type, Some(6));
    assert_eq!(voice.sample_rate, Some(24_000));
    assert_eq!(voice.playtime, Some(1_234));
    assert_eq!(
        serde_json::to_value(
            voice
                .media
                .as_ref()
                .expect("voice media")
                .encrypt_query_param
                .as_ref()
                .expect("download param")
        )
        .expect("secret serialization"),
        json!("private-download-param")
    );
}

#[test]
fn upload_url_response_accepts_param_only_servers() {
    let response: GetUploadUrlResponse = serde_json::from_value(json!({
        "upload_param": "private-upload-param"
    }))
    .expect("upload_param-only responses should decode");
    assert!(response.upload_full_url.is_none());
    assert_eq!(
        serde_json::to_value(response.upload_param.expect("upload param")).expect("secret"),
        json!("private-upload-param")
    );
}

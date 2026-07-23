use std::collections::VecDeque;
use std::time::Duration;

use async_trait::async_trait;
use tempfile::TempDir;
use yunxi_agent_weixin::ilink::{GetBotQrCodeResponse, GetQrCodeStatusResponse, QrCodeStatus};
use yunxi_agent_weixin::{
    FakeWeixinSecretStore, LoginPollState, SecretString, WeixinAccountId, WeixinAccountRecord,
    WeixinAccountStore, WeixinConnectionState, WeixinLoginCancellation, WeixinLoginEvent,
    WeixinLoginFailure, WeixinLoginOptions, WeixinLoginStateMachine, WeixinLoginTransport,
    WeixinSecretStore, WeixinSecretStoreError, generate_data_key, redacted_json_snapshot,
};

struct ScriptedTransport {
    statuses: VecDeque<GetQrCodeStatusResponse>,
    fetches: usize,
    polls: usize,
}

impl ScriptedTransport {
    fn new(statuses: impl IntoIterator<Item = QrCodeStatus>) -> Self {
        Self {
            statuses: statuses
                .into_iter()
                .map(|status| GetQrCodeStatusResponse {
                    status,
                    bot_token: (status == QrCodeStatus::Confirmed)
                        .then(|| SecretString::new("bot-token-secret")),
                    ilink_bot_id: Some(SecretString::new("bot-id-secret")),
                    baseurl: Some("https://ilinkai.weixin.qq.com/".to_string()),
                    ilink_user_id: Some(SecretString::new("user-id-secret")),
                    redirect_host: None,
                })
                .collect(),
            fetches: 0,
            polls: 0,
        }
    }
}

#[async_trait]
impl WeixinLoginTransport for ScriptedTransport {
    async fn fetch_qr_code(
        &mut self,
    ) -> Result<GetBotQrCodeResponse, yunxi_agent_weixin::WeixinApiError> {
        self.fetches += 1;
        Ok(GetBotQrCodeResponse {
            qrcode: SecretString::new("https://qr.example/secret-payload"),
            qrcode_img_content: SecretString::new("\u{1b}[31mQR\u{1b}[0m"),
        })
    }

    async fn poll_qr_status(
        &mut self,
        _qrcode: &SecretString,
        _verify_code: Option<&SecretString>,
    ) -> Result<GetQrCodeStatusResponse, yunxi_agent_weixin::WeixinApiError> {
        self.polls += 1;
        Ok(self
            .statuses
            .pop_front()
            .unwrap_or(GetQrCodeStatusResponse {
                status: QrCodeStatus::Wait,
                bot_token: None,
                ilink_bot_id: None,
                baseurl: None,
                ilink_user_id: None,
                redirect_host: None,
            }))
    }
}

fn fast_options() -> WeixinLoginOptions {
    WeixinLoginOptions::bounded(Duration::from_millis(1), Duration::from_millis(100))
        .expect("valid test options")
}

#[tokio::test]
async fn login_state_machine_reaches_confirmed_and_keeps_events_safe() {
    let mut transport = ScriptedTransport::new([
        QrCodeStatus::Wait,
        QrCodeStatus::Scanned,
        QrCodeStatus::Confirmed,
    ]);
    let machine = WeixinLoginStateMachine::new(fast_options());
    let cancellation = WeixinLoginCancellation::default();
    let mut events = Vec::new();
    let outcome = machine
        .run(&mut transport, &cancellation, |event| events.push(event))
        .await
        .expect("login should confirm");

    assert_eq!(transport.fetches, 1);
    assert_eq!(transport.polls, 3);
    assert!(events.iter().any(|event| {
        matches!(
            event,
            WeixinLoginEvent::PollState {
                state: LoginPollState::Scanned
            }
        )
    }));
    assert!(!outcome.bot_token.is_empty());
    let debug = format!("{outcome:?}");
    assert!(!debug.contains("bot-token-secret"));
    assert_eq!(
        events
            .iter()
            .find_map(|event| match event {
                WeixinLoginEvent::QrReady { display } => Some(display.terminal_text()),
                _ => None,
            })
            .expect("QR event"),
        "QR"
    );
}

#[tokio::test]
async fn login_state_machine_rejects_expiry_redirect_and_verification_states() {
    for status in [
        QrCodeStatus::Expired,
        QrCodeStatus::ScannedButRedirect,
        QrCodeStatus::BoundRedirect,
        QrCodeStatus::NeedVerifyCode,
        QrCodeStatus::VerifyCodeBlocked,
    ] {
        let mut transport = ScriptedTransport::new([status]);
        let machine = WeixinLoginStateMachine::new(fast_options());
        let error = machine
            .run(&mut transport, &WeixinLoginCancellation::default(), |_| {})
            .await
            .expect_err("terminal state must fail safely");
        match status {
            QrCodeStatus::Expired => assert_eq!(error, WeixinLoginFailure::Expired),
            QrCodeStatus::ScannedButRedirect | QrCodeStatus::BoundRedirect => {
                assert_eq!(error, WeixinLoginFailure::RedirectRequired)
            }
            QrCodeStatus::NeedVerifyCode => assert_eq!(error, WeixinLoginFailure::NeedVerifyCode),
            QrCodeStatus::VerifyCodeBlocked => {
                assert_eq!(error, WeixinLoginFailure::VerifyCodeBlocked)
            }
            _ => unreachable!(),
        }
    }
}

#[tokio::test]
async fn login_state_machine_honors_cancellation_before_network() {
    let mut transport = ScriptedTransport::new([QrCodeStatus::Confirmed]);
    let cancellation = WeixinLoginCancellation::default();
    cancellation.cancel();
    let error = WeixinLoginStateMachine::new(fast_options())
        .run(&mut transport, &cancellation, |_| {})
        .await
        .expect_err("cancelled login must fail");
    assert_eq!(error, WeixinLoginFailure::Cancelled);
    assert_eq!(transport.fetches, 0);
}

#[test]
fn fake_secret_store_and_account_metadata_never_persist_plain_secrets() {
    let store = FakeWeixinSecretStore::new();
    let account = WeixinAccountId::new("private-account-name");
    let data_key = generate_data_key().expect("data key");
    let reference = store.credential_reference(&account);
    store
        .put_data_key(&account, &data_key)
        .expect("data key write");
    store
        .put_token(
            &account,
            &SecretString::new("bot-token-secret"),
            &reference.data_key_target,
        )
        .expect("token write");
    let workspace = TempDir::new().expect("workspace");
    let record =
        WeixinAccountRecord::new(&account, reference, workspace.path(), 123).expect("record");
    let store_on_disk = WeixinAccountStore::new(workspace.path());
    store_on_disk
        .save(&account, &record)
        .expect("metadata write");
    let json = serde_json::to_string(&record).expect("metadata json");
    assert!(!json.contains("bot-token-secret"));
    assert!(!json.contains("private-account-name"));
    assert!(json.contains("windows-credential-manager") || json.contains("fake-test-only"));
    assert_eq!(
        store_on_disk
            .load(&account)
            .expect("metadata read")
            .expect("record")
            .connection_state,
        WeixinConnectionState::Ready
    );
    let snapshot = redacted_json_snapshot(&record).expect("snapshot");
    assert!(!snapshot.to_string().contains("bot-token-secret"));
}

#[test]
fn unavailable_secret_store_fails_without_a_plaintext_fallback() {
    let store = FakeWeixinSecretStore::unavailable();
    let account = WeixinAccountId::new("private-account-name");
    let error = store
        .put_token(
            &account,
            &SecretString::new("bot-token-secret"),
            "data-key-reference",
        )
        .expect_err("unavailable store must reject writes");
    assert_eq!(error, WeixinSecretStoreError::Unavailable);
    assert!(!format!("{error}").contains("bot-token-secret"));
}

#[test]
fn fake_secret_store_isolates_accounts() {
    let store = FakeWeixinSecretStore::new();
    let first = WeixinAccountId::new("first-account");
    let second = WeixinAccountId::new("second-account");
    store
        .put_token(&first, &SecretString::new("first-token"), "key-ref")
        .expect("first token write");
    store
        .put_token(&second, &SecretString::new("second-token"), "key-ref")
        .expect("second token write");

    assert_eq!(
        store.get_token(&first).expect("first token"),
        SecretString::new("first-token")
    );
    assert_eq!(
        store.get_token(&second).expect("second token"),
        SecretString::new("second-token")
    );
    store.delete_token(&first).expect("first token delete");
    assert_eq!(
        store.get_token(&first),
        Err(WeixinSecretStoreError::NotFound)
    );
    assert_eq!(
        store.get_token(&second).expect("second token remains"),
        SecretString::new("second-token")
    );
}

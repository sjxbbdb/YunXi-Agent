use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::time::sleep;

use crate::{
    SecretString, WeixinApiError,
    ilink::{
        GetBotQrCodeRequest, GetBotQrCodeResponse, GetQrCodeStatusResponse, IlinkHttpClient,
        QrCodeStatus,
    },
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LoginPollState {
    Wait,
    Scanned,
    Confirmed,
    ScannedButRedirect,
    BoundRedirect,
    Expired,
    NeedVerifyCode,
    VerifyCodeBlocked,
}

impl From<QrCodeStatus> for LoginPollState {
    fn from(status: QrCodeStatus) -> Self {
        match status {
            QrCodeStatus::Wait => Self::Wait,
            QrCodeStatus::Scanned => Self::Scanned,
            QrCodeStatus::Confirmed => Self::Confirmed,
            QrCodeStatus::ScannedButRedirect => Self::ScannedButRedirect,
            QrCodeStatus::BoundRedirect => Self::BoundRedirect,
            QrCodeStatus::Expired => Self::Expired,
            QrCodeStatus::NeedVerifyCode => Self::NeedVerifyCode,
            QrCodeStatus::VerifyCodeBlocked => Self::VerifyCodeBlocked,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeixinQrDisplay {
    qrcode: SecretString,
    image_content: SecretString,
}

impl WeixinQrDisplay {
    pub fn new(response: &GetBotQrCodeResponse) -> Self {
        Self {
            qrcode: response.qrcode.clone(),
            image_content: response.qrcode_img_content.clone(),
        }
    }

    pub fn terminal_text(&self) -> String {
        let source = if self.image_content.is_empty() {
            self.qrcode.expose()
        } else {
            self.image_content.expose()
        };
        sanitize_terminal_text(source)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WeixinLoginEvent {
    QrReady { display: WeixinQrDisplay },
    PollState { state: LoginPollState },
    Cancelled,
    TimedOut,
    Failed { failure: WeixinLoginFailure },
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum WeixinLoginFailure {
    #[error("weixin QR login timed out")]
    TimedOut,
    #[error("weixin QR login cancelled by user")]
    Cancelled,
    #[error("weixin QR code expired; run login again")]
    Expired,
    #[error("weixin QR login requires a verification code; retry with supported verification")]
    NeedVerifyCode,
    #[error("weixin QR verification code is blocked")]
    VerifyCodeBlocked,
    #[error("weixin QR login returned an unsupported redirect state")]
    RedirectRequired,
    #[error("weixin QR login response did not include a token")]
    MissingToken,
    #[error("weixin QR login response did not include a QR code")]
    MissingQrCode,
    #[error("weixin QR login transport failed: {0}")]
    Api(#[from] WeixinApiError),
}

#[derive(Clone, Debug)]
pub struct WeixinLoginOptions {
    pub poll_interval: Duration,
    pub timeout: Duration,
    pub verify_code: Option<SecretString>,
}

impl WeixinLoginOptions {
    pub fn bounded(poll_interval: Duration, timeout: Duration) -> Result<Self, WeixinLoginFailure> {
        if poll_interval.is_zero() || timeout.is_zero() {
            return Err(WeixinLoginFailure::TimedOut);
        }
        Ok(Self {
            poll_interval,
            timeout,
            verify_code: None,
        })
    }
}

impl Default for WeixinLoginOptions {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_millis(1500),
            timeout: Duration::from_secs(300),
            verify_code: None,
        }
    }
}

#[derive(Clone, Default)]
pub struct WeixinLoginCancellation(Arc<AtomicBool>);

impl WeixinLoginCancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Debug)]
pub struct WeixinLoginOutcome {
    pub bot_token: SecretString,
    pub ilink_bot_id: Option<SecretString>,
    pub ilink_user_id: Option<SecretString>,
    pub base_url: Option<String>,
}

pub struct WeixinLoginStateMachine {
    options: WeixinLoginOptions,
}

impl WeixinLoginStateMachine {
    pub fn new(options: WeixinLoginOptions) -> Self {
        Self { options }
    }

    pub async fn run<T, F>(
        &self,
        transport: &mut T,
        cancellation: &WeixinLoginCancellation,
        mut on_event: F,
    ) -> Result<WeixinLoginOutcome, WeixinLoginFailure>
    where
        T: WeixinLoginTransport,
        F: FnMut(WeixinLoginEvent),
    {
        let started = Instant::now();
        if cancellation.is_cancelled() {
            on_event(WeixinLoginEvent::Cancelled);
            return Err(WeixinLoginFailure::Cancelled);
        }

        let qr_response = transport
            .fetch_qr_code()
            .await
            .map_err(WeixinLoginFailure::Api)?;
        if qr_response.qrcode.is_empty() {
            return self.fail(WeixinLoginFailure::MissingQrCode, &mut on_event);
        }
        on_event(WeixinLoginEvent::QrReady {
            display: WeixinQrDisplay::new(&qr_response),
        });

        loop {
            if cancellation.is_cancelled() {
                on_event(WeixinLoginEvent::Cancelled);
                return Err(WeixinLoginFailure::Cancelled);
            }
            if started.elapsed() >= self.options.timeout {
                return self.fail(WeixinLoginFailure::TimedOut, &mut on_event);
            }

            let status = match transport
                .poll_qr_status(&qr_response.qrcode, self.options.verify_code.as_ref())
                .await
            {
                Ok(status) => status,
                Err(WeixinApiError::Timeout { .. }) => {
                    if started.elapsed() >= self.options.timeout {
                        return self.fail(WeixinLoginFailure::TimedOut, &mut on_event);
                    }
                    on_event(WeixinLoginEvent::PollState {
                        state: LoginPollState::Wait,
                    });
                    let remaining = self.options.timeout.saturating_sub(started.elapsed());
                    sleep(self.options.poll_interval.min(remaining)).await;
                    continue;
                }
                Err(error) => return Err(WeixinLoginFailure::Api(error)),
            };
            let state = LoginPollState::from(status.status);
            on_event(WeixinLoginEvent::PollState { state });

            match state {
                LoginPollState::Wait | LoginPollState::Scanned => {
                    let remaining = self.options.timeout.saturating_sub(started.elapsed());
                    sleep(self.options.poll_interval.min(remaining)).await;
                }
                LoginPollState::Confirmed => {
                    let Some(bot_token) = status.bot_token else {
                        return self.fail(WeixinLoginFailure::MissingToken, &mut on_event);
                    };
                    if bot_token.is_empty() {
                        return self.fail(WeixinLoginFailure::MissingToken, &mut on_event);
                    }
                    return Ok(WeixinLoginOutcome {
                        bot_token,
                        ilink_bot_id: status.ilink_bot_id,
                        ilink_user_id: status.ilink_user_id,
                        base_url: status.baseurl,
                    });
                }
                LoginPollState::Expired => {
                    return self.fail(WeixinLoginFailure::Expired, &mut on_event);
                }
                LoginPollState::NeedVerifyCode => {
                    return self.fail(WeixinLoginFailure::NeedVerifyCode, &mut on_event);
                }
                LoginPollState::VerifyCodeBlocked => {
                    return self.fail(WeixinLoginFailure::VerifyCodeBlocked, &mut on_event);
                }
                LoginPollState::ScannedButRedirect | LoginPollState::BoundRedirect => {
                    return self.fail(WeixinLoginFailure::RedirectRequired, &mut on_event);
                }
            }
        }
    }

    fn fail<F>(
        &self,
        failure: WeixinLoginFailure,
        on_event: &mut F,
    ) -> Result<WeixinLoginOutcome, WeixinLoginFailure>
    where
        F: FnMut(WeixinLoginEvent),
    {
        on_event(WeixinLoginEvent::Failed {
            failure: failure.clone(),
        });
        Err(failure)
    }
}

#[async_trait]
pub trait WeixinLoginTransport: Send {
    async fn fetch_qr_code(&mut self) -> Result<GetBotQrCodeResponse, WeixinApiError>;

    async fn poll_qr_status(
        &mut self,
        qrcode: &SecretString,
        verify_code: Option<&SecretString>,
    ) -> Result<GetQrCodeStatusResponse, WeixinApiError>;
}

#[async_trait]
impl WeixinLoginTransport for IlinkHttpClient {
    async fn fetch_qr_code(&mut self) -> Result<GetBotQrCodeResponse, WeixinApiError> {
        IlinkHttpClient::fetch_qr_code(self, GetBotQrCodeRequest::default()).await
    }

    async fn poll_qr_status(
        &mut self,
        qrcode: &SecretString,
        verify_code: Option<&SecretString>,
    ) -> Result<GetQrCodeStatusResponse, WeixinApiError> {
        IlinkHttpClient::poll_qr_status(self, qrcode, verify_code).await
    }
}

fn sanitize_terminal_text(value: &str) -> String {
    let mut sanitized = String::new();
    let mut in_escape = false;
    for character in value.chars() {
        if in_escape {
            if character.is_ascii_alphabetic() {
                in_escape = false;
            }
            continue;
        }
        if character == '\u{1b}' {
            in_escape = true;
            continue;
        }
        if character.is_control() && !matches!(character, '\n' | '\r' | '\t') {
            continue;
        }
        sanitized.push(character);
        if sanitized.chars().count() >= 16_384 {
            break;
        }
    }
    sanitized
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use async_trait::async_trait;

    use super::*;
    use crate::error::RequestContext;

    enum PollStep {
        Status(QrCodeStatus),
        Timeout,
    }

    struct ScriptedTransport {
        steps: VecDeque<PollStep>,
        polls: usize,
    }

    impl ScriptedTransport {
        fn new(steps: impl IntoIterator<Item = PollStep>) -> Self {
            Self {
                steps: steps.into_iter().collect(),
                polls: 0,
            }
        }
    }

    #[async_trait]
    impl WeixinLoginTransport for ScriptedTransport {
        async fn fetch_qr_code(&mut self) -> Result<GetBotQrCodeResponse, WeixinApiError> {
            Ok(GetBotQrCodeResponse {
                qrcode: SecretString::new("https://qr.example/secret-payload"),
                qrcode_img_content: SecretString::new("QR"),
            })
        }

        async fn poll_qr_status(
            &mut self,
            _qrcode: &SecretString,
            _verify_code: Option<&SecretString>,
        ) -> Result<GetQrCodeStatusResponse, WeixinApiError> {
            self.polls += 1;
            match self
                .steps
                .pop_front()
                .unwrap_or(PollStep::Status(QrCodeStatus::Wait))
            {
                PollStep::Status(status) => Ok(GetQrCodeStatusResponse {
                    status,
                    bot_token: (status == QrCodeStatus::Confirmed)
                        .then(|| SecretString::new("bot-token-secret")),
                    ilink_bot_id: Some(SecretString::new("bot-id-secret")),
                    baseurl: Some("https://ilinkai.weixin.qq.com/".to_string()),
                    ilink_user_id: Some(SecretString::new("user-id-secret")),
                    redirect_host: None,
                }),
                PollStep::Timeout => Err(WeixinApiError::Timeout {
                    context: RequestContext::new("wx-test-timeout", "poll_qr_status", "account"),
                }),
            }
        }
    }

    #[tokio::test]
    async fn qr_login_recovers_poll_timeout_before_confirmed() {
        let mut transport = ScriptedTransport::new([
            PollStep::Timeout,
            PollStep::Status(QrCodeStatus::Scanned),
            PollStep::Status(QrCodeStatus::Confirmed),
        ]);
        let options =
            WeixinLoginOptions::bounded(Duration::from_millis(1), Duration::from_millis(200))
                .expect("valid options");
        let machine = WeixinLoginStateMachine::new(options);
        let mut events = Vec::new();
        let outcome = machine
            .run(
                &mut transport,
                &WeixinLoginCancellation::default(),
                |event| {
                    events.push(event);
                },
            )
            .await
            .expect("poll timeout should be recoverable");

        assert_eq!(transport.polls, 3);
        assert!(!outcome.bot_token.is_empty());
        assert!(events.iter().any(|event| {
            matches!(
                event,
                WeixinLoginEvent::PollState {
                    state: LoginPollState::Wait
                }
            )
        }));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                WeixinLoginEvent::PollState {
                    state: LoginPollState::Scanned
                }
            )
        }));
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, WeixinLoginEvent::Failed { .. }))
        );
    }
}

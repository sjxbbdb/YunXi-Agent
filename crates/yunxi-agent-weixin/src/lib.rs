mod account_store;
mod backoff;
mod delivery;
mod domain;
mod error;
pub mod ilink;
mod inbound;
mod login;
mod payload_cipher;
mod redaction;
mod remote_control;
mod secret_store;
mod serve;
mod turn_supervisor;
mod voice;

pub use account_store::{
    WEIXIN_ACCOUNT_SCHEMA_VERSION, WeixinAccountRecord, WeixinAccountStore, WeixinAccountStoreError,
};
pub use backoff::WeixinBackoff;
pub use delivery::{
    WeixinDeliveryDispatcher, WeixinDeliveryDrainReport, WeixinDeliveryError,
    WeixinDeliveryOutcomeClass, WeixinDeliverySpoolSink, WeixinMessageTransport,
    WeixinResponseMode, split_weixin_text_segments,
};
pub use domain::{
    WeixinAccountId, WeixinAccountMetadata, WeixinConnectionState, WeixinConversationKey,
    WeixinMessageId, WeixinPeerId,
};
pub use error::{RequestContext, WeixinApiError};
pub use ilink::{IlinkHttpClient, PRODUCTION_ILINK_ENDPOINT};
pub use inbound::{
    WeixinInboundEnvelope, WeixinInboundKind, WeixinInboundVoice, WeixinPendingInboundPayload,
};
pub use login::{
    LoginPollState, WeixinLoginCancellation, WeixinLoginEvent, WeixinLoginFailure,
    WeixinLoginOptions, WeixinLoginOutcome, WeixinLoginStateMachine, WeixinLoginTransport,
    WeixinQrDisplay,
};
pub use payload_cipher::{WeixinPayloadAad, WeixinPayloadCipher, WeixinPayloadCipherError};
pub use redaction::{SecretString, redacted_json_snapshot};
pub use remote_control::{
    WeixinRemoteCommand, WeixinRemoteControlError, WeixinRemoteControlHub,
    WeixinRemoteControlOutcome, WeixinRemoteControlPrompt, WeixinRemoteControlPurpose,
    WeixinRemoteControlScope, parse_weixin_remote_command,
};
pub use secret_store::{
    FakeWeixinSecretStore, SystemWeixinSecretStore, WeixinCredentialReference, WeixinSecretStore,
    WeixinSecretStoreError, generate_data_key,
};
pub use serve::{
    WeixinServeCancellation, WeixinServeError, WeixinServeOptions, WeixinServeReport,
    WeixinServeStoppedReason, WeixinUpdatesTransport, run_weixin_serve_loop,
};
pub use turn_supervisor::{
    NoopWeixinRuntimeSink, WeixinRuntimeDispatcher, WeixinRuntimeDispatcherAdapter,
    WeixinRuntimeSink, WeixinRuntimeSinkRecord, WeixinRuntimeTestSink,
    WeixinStreamObservationReport, WeixinTurnReport, WeixinTurnSupervisor,
    WeixinTurnSupervisorError, WeixinTurnSupervisorOptions,
};
pub use voice::{WeixinVoiceBridge, WeixinVoiceError, WeixinVoiceTranscriber};

mod account_store;
mod backoff;
mod domain;
mod error;
pub mod ilink;
mod inbound;
mod login;
mod redaction;
mod secret_store;
mod serve;

pub use account_store::{
    WEIXIN_ACCOUNT_SCHEMA_VERSION, WeixinAccountRecord, WeixinAccountStore, WeixinAccountStoreError,
};
pub use backoff::WeixinBackoff;
pub use domain::{
    WeixinAccountId, WeixinAccountMetadata, WeixinConnectionState, WeixinConversationKey,
    WeixinMessageId, WeixinPeerId,
};
pub use error::{RequestContext, WeixinApiError};
pub use ilink::{IlinkHttpClient, PRODUCTION_ILINK_ENDPOINT};
pub use inbound::{WeixinInboundEnvelope, WeixinInboundKind};
pub use login::{
    LoginPollState, WeixinLoginCancellation, WeixinLoginEvent, WeixinLoginFailure,
    WeixinLoginOptions, WeixinLoginOutcome, WeixinLoginStateMachine, WeixinLoginTransport,
    WeixinQrDisplay,
};
pub use redaction::{SecretString, redacted_json_snapshot};
pub use secret_store::{
    FakeWeixinSecretStore, SystemWeixinSecretStore, WeixinCredentialReference, WeixinSecretStore,
    WeixinSecretStoreError, generate_data_key,
};
pub use serve::{
    WeixinServeCancellation, WeixinServeError, WeixinServeOptions, WeixinServeReport,
    WeixinServeStoppedReason, WeixinUpdatesTransport, run_weixin_serve_loop,
};

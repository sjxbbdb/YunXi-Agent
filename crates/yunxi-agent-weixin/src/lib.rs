mod account_store;
mod domain;
mod error;
pub mod ilink;
mod login;
mod redaction;
mod secret_store;

pub use account_store::{
    WEIXIN_ACCOUNT_SCHEMA_VERSION, WeixinAccountRecord, WeixinAccountStore, WeixinAccountStoreError,
};
pub use domain::{
    WeixinAccountId, WeixinAccountMetadata, WeixinConnectionState, WeixinConversationKey,
    WeixinMessageId, WeixinPeerId,
};
pub use error::{RequestContext, WeixinApiError};
pub use ilink::{IlinkHttpClient, PRODUCTION_ILINK_ENDPOINT};
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

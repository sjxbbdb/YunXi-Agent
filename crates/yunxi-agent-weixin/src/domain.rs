use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

use crate::redaction::redacted_identifier;

macro_rules! private_identifier {
    ($name:ident, $label:literal) => {
        #[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&redacted_identifier($label, &self.0))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&redacted_identifier($label, &self.0))
            }
        }
    };
}

private_identifier!(WeixinAccountId, "account");
private_identifier!(WeixinPeerId, "peer");

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WeixinMessageId(String);

impl WeixinMessageId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for WeixinMessageId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("WeixinMessageId")
            .field(&self.0)
            .finish()
    }
}

impl fmt::Display for WeixinMessageId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for WeixinMessageId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for WeixinMessageId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MessageIdVisitor;

        impl serde::de::Visitor<'_> for MessageIdVisitor {
            type Value = WeixinMessageId;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a string or integer message id")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(WeixinMessageId::new(value))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(WeixinMessageId::new(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(WeixinMessageId::new(value.to_string()))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(WeixinMessageId::new(value.to_string()))
            }
        }

        deserializer.deserialize_any(MessageIdVisitor)
    }
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct WeixinConversationKey {
    pub account_id: WeixinAccountId,
    pub peer_id: WeixinPeerId,
}

impl fmt::Debug for WeixinConversationKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WeixinConversationKey")
            .field("account_id", &self.account_id)
            .field("peer_id", &self.peer_id)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeixinConnectionState {
    NotConfigured,
    Unavailable,
    Ready,
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct WeixinAccountMetadata {
    pub account_id: WeixinAccountId,
    pub connection_state: WeixinConnectionState,
    pub private_chat_only: bool,
    pub credentials_persisted: bool,
}

impl fmt::Debug for WeixinAccountMetadata {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WeixinAccountMetadata")
            .field("account_id", &self.account_id)
            .field("connection_state", &self.connection_state)
            .field("private_chat_only", &self.private_chat_only)
            .field("credentials_persisted", &self.credentials_persisted)
            .finish()
    }
}

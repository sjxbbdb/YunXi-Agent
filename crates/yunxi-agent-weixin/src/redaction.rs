use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

const REDACTED: &str = "[REDACTED]";

#[derive(Clone, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(REDACTED)
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(REDACTED)
    }
}

pub(crate) fn redacted_identifier(label: &str, value: &str) -> String {
    let mut hash = 0x811c9dc5_u32;
    for byte in value.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x01000193);
    }
    format!("{label}#{hash:08x}")
}

pub fn redacted_json_snapshot<T>(value: &T) -> Result<Value, serde_json::Error>
where
    T: Serialize,
{
    let mut value = serde_json::to_value(value)?;
    redact_value(&mut value, None);
    Ok(value)
}

fn redact_value(value: &mut Value, key: Option<&str>) {
    if key.is_some_and(is_sensitive_key) {
        *value = Value::String(REDACTED.to_string());
        return;
    }
    match value {
        Value::Array(values) => {
            for value in values {
                redact_value(value, key);
            }
        }
        Value::Object(values) => {
            for (key, value) in values {
                redact_value(value, Some(key));
            }
        }
        _ => {}
    }
}

fn is_sensitive_key(key: &str) -> bool {
    matches!(
        key.to_ascii_lowercase().as_str(),
        "authorization"
            | "token"
            | "bot_token"
            | "local_token_list"
            | "qrcode"
            | "qrcode_img_content"
            | "verify_code"
            | "context_token"
            | "get_updates_buf"
            | "sync_buf"
            | "ilink_user_id"
            | "from_user_id"
            | "to_user_id"
            | "account_id"
            | "peer_id"
            | "text"
            | "raw_content"
    )
}

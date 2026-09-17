use base64::{Engine as _, engine::general_purpose::STANDARD};
use chacha20poly1305::{
    ChaCha20Poly1305, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use thiserror::Error;
use yunxi_agent_storage::{
    WEIXIN_PAYLOAD_AAD_VERSION, WEIXIN_PAYLOAD_ALGORITHM, WEIXIN_PAYLOAD_ALGORITHM_VERSION,
    WEIXIN_PAYLOAD_NONCE_LENGTH, WeixinEncryptedPayload, WeixinPendingInbound,
};

use crate::SecretString;
use crate::inbound::WeixinPendingInboundPayload;

const MAX_NONCE_ATTEMPTS: usize = 16;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeixinPayloadAad {
    account_id: String,
    peer_id_hash: String,
    message_id_hash: String,
    item_id: String,
    aad_version: u32,
}

impl WeixinPayloadAad {
    pub fn new(
        account_id: impl Into<String>,
        peer_id_hash: impl Into<String>,
        message_id_hash: impl Into<String>,
        item_id: impl Into<String>,
    ) -> Self {
        Self {
            account_id: account_id.into(),
            peer_id_hash: peer_id_hash.into(),
            message_id_hash: message_id_hash.into(),
            item_id: item_id.into(),
            aad_version: WEIXIN_PAYLOAD_AAD_VERSION,
        }
    }

    pub fn from_pending(pending: &WeixinPendingInbound) -> Result<Self, WeixinPayloadCipherError> {
        let payload = pending
            .encrypted_payload
            .as_ref()
            .ok_or(WeixinPayloadCipherError::MissingPayload)?;
        if payload.aad_version != WEIXIN_PAYLOAD_AAD_VERSION {
            return Err(WeixinPayloadCipherError::UnsupportedAadVersion);
        }
        Ok(Self {
            account_id: pending.account_id.clone(),
            peer_id_hash: pending.peer_id_hash.clone(),
            message_id_hash: pending.message_id_hash.clone(),
            item_id: pending.item_id.clone(),
            aad_version: payload.aad_version,
        })
    }

    fn as_bytes(&self) -> Vec<u8> {
        format!(
            concat!(
                "yunxi-weixin-pending-inbound\n",
                "aad_version={}\n",
                "account_id={}\n",
                "peer_id_hash={}\n",
                "message_id_hash={}\n",
                "item_id={}\n"
            ),
            self.aad_version,
            self.account_id,
            self.peer_id_hash,
            self.message_id_hash,
            self.item_id
        )
        .into_bytes()
    }
}

#[derive(Clone, Debug, Default)]
pub struct WeixinPayloadCipher;

impl WeixinPayloadCipher {
    pub fn new() -> Self {
        Self
    }

    pub fn validate_data_key(
        &self,
        data_key: &SecretString,
    ) -> Result<(), WeixinPayloadCipherError> {
        decode_hex_data_key(data_key).map(|_| ())
    }

    pub fn encrypt_pending_inbound(
        &self,
        data_key: &SecretString,
        plaintext: &WeixinPendingInboundPayload,
        aad: &WeixinPayloadAad,
    ) -> Result<WeixinEncryptedPayload, WeixinPayloadCipherError> {
        let bytes = plaintext
            .to_plaintext_bytes()
            .map_err(|_| WeixinPayloadCipherError::InvalidPlaintext)?;
        self.encrypt(data_key, &bytes, aad)
    }

    pub fn decrypt_pending_inbound(
        &self,
        data_key: &SecretString,
        pending: &WeixinPendingInbound,
    ) -> Result<WeixinPendingInboundPayload, WeixinPayloadCipherError> {
        let payload = pending
            .encrypted_payload
            .as_ref()
            .ok_or(WeixinPayloadCipherError::MissingPayload)?;
        let aad = WeixinPayloadAad::from_pending(pending)?;
        let bytes = self.decrypt(data_key, payload, &aad)?;
        let plaintext = WeixinPendingInboundPayload::from_plaintext_bytes(&bytes)
            .map_err(|_| WeixinPayloadCipherError::InvalidPlaintext)?;
        if !plaintext.matches_bindings(
            &pending.account_id,
            &pending.peer_id_hash,
            &pending.message_id_hash,
            &pending.item_id,
        ) {
            return Err(WeixinPayloadCipherError::InvalidPlaintext);
        }
        Ok(plaintext)
    }

    pub fn encrypt(
        &self,
        data_key: &SecretString,
        plaintext: &[u8],
        aad: &WeixinPayloadAad,
    ) -> Result<WeixinEncryptedPayload, WeixinPayloadCipherError> {
        let key = decode_hex_data_key(data_key)?;
        let cipher = ChaCha20Poly1305::new_from_slice(&key)
            .map_err(|_| WeixinPayloadCipherError::InvalidDataKey)?;
        let aad_bytes = aad.as_bytes();
        for _ in 0..MAX_NONCE_ATTEMPTS {
            let mut nonce = [0_u8; WEIXIN_PAYLOAD_NONCE_LENGTH];
            getrandom::fill(&mut nonce).map_err(|_| WeixinPayloadCipherError::Random)?;
            let ciphertext = cipher
                .encrypt(
                    Nonce::from_slice(&nonce),
                    Payload {
                        msg: plaintext,
                        aad: &aad_bytes,
                    },
                )
                .map_err(|_| WeixinPayloadCipherError::EncryptionFailed)?;
            let payload = WeixinEncryptedPayload {
                algorithm: WEIXIN_PAYLOAD_ALGORITHM.to_string(),
                algorithm_version: WEIXIN_PAYLOAD_ALGORITHM_VERSION,
                aad_version: WEIXIN_PAYLOAD_AAD_VERSION,
                nonce: STANDARD.encode(nonce),
                ciphertext: STANDARD.encode(ciphertext),
            };
            if storage_safe_marker(&payload.nonce) && storage_safe_marker(&payload.ciphertext) {
                return Ok(payload);
            }
        }
        Err(WeixinPayloadCipherError::UnsafeEncodedPayload)
    }

    pub fn decrypt(
        &self,
        data_key: &SecretString,
        payload: &WeixinEncryptedPayload,
        aad: &WeixinPayloadAad,
    ) -> Result<Vec<u8>, WeixinPayloadCipherError> {
        validate_payload_metadata(payload)?;
        if aad.aad_version != payload.aad_version {
            return Err(WeixinPayloadCipherError::UnsupportedAadVersion);
        }
        let key = decode_hex_data_key(data_key)?;
        let cipher = ChaCha20Poly1305::new_from_slice(&key)
            .map_err(|_| WeixinPayloadCipherError::InvalidDataKey)?;
        let nonce = STANDARD
            .decode(payload.nonce.as_bytes())
            .map_err(|_| WeixinPayloadCipherError::InvalidNonce)?;
        let ciphertext = STANDARD
            .decode(payload.ciphertext.as_bytes())
            .map_err(|_| WeixinPayloadCipherError::InvalidCiphertext)?;
        let aad_bytes = aad.as_bytes();
        cipher
            .decrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: &ciphertext,
                    aad: &aad_bytes,
                },
            )
            .map_err(|_| WeixinPayloadCipherError::DecryptionFailed)
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum WeixinPayloadCipherError {
    #[error("weixin pending payload key is invalid")]
    InvalidDataKey,
    #[error("weixin pending payload random nonce generation failed")]
    Random,
    #[error("weixin pending payload encryption failed")]
    EncryptionFailed,
    #[error("weixin pending payload decryption failed")]
    DecryptionFailed,
    #[error("weixin pending payload algorithm is unsupported")]
    UnsupportedAlgorithm,
    #[error("weixin pending payload algorithm version is unsupported")]
    UnsupportedAlgorithmVersion,
    #[error("weixin pending payload aad version is unsupported")]
    UnsupportedAadVersion,
    #[error("weixin pending payload nonce is invalid")]
    InvalidNonce,
    #[error("weixin pending payload ciphertext is invalid")]
    InvalidCiphertext,
    #[error("weixin pending payload plaintext is invalid")]
    InvalidPlaintext,
    #[error("weixin pending payload is missing")]
    MissingPayload,
    #[error("weixin pending payload encoded metadata is unsafe")]
    UnsafeEncodedPayload,
}

fn validate_payload_metadata(
    payload: &WeixinEncryptedPayload,
) -> Result<(), WeixinPayloadCipherError> {
    if payload.algorithm != WEIXIN_PAYLOAD_ALGORITHM {
        return Err(WeixinPayloadCipherError::UnsupportedAlgorithm);
    }
    if payload.algorithm_version != WEIXIN_PAYLOAD_ALGORITHM_VERSION {
        return Err(WeixinPayloadCipherError::UnsupportedAlgorithmVersion);
    }
    if payload.aad_version != WEIXIN_PAYLOAD_AAD_VERSION {
        return Err(WeixinPayloadCipherError::UnsupportedAadVersion);
    }
    let nonce = STANDARD
        .decode(payload.nonce.as_bytes())
        .map_err(|_| WeixinPayloadCipherError::InvalidNonce)?;
    if nonce.len() != WEIXIN_PAYLOAD_NONCE_LENGTH {
        return Err(WeixinPayloadCipherError::InvalidNonce);
    }
    let ciphertext = STANDARD
        .decode(payload.ciphertext.as_bytes())
        .map_err(|_| WeixinPayloadCipherError::InvalidCiphertext)?;
    if ciphertext.is_empty() {
        return Err(WeixinPayloadCipherError::InvalidCiphertext);
    }
    Ok(())
}

fn decode_hex_data_key(data_key: &SecretString) -> Result<[u8; 32], WeixinPayloadCipherError> {
    let value = data_key.expose();
    if value.len() != 64 {
        return Err(WeixinPayloadCipherError::InvalidDataKey);
    }
    let mut bytes = [0_u8; 32];
    let raw = value.as_bytes();
    for index in 0..32 {
        let high = hex_value(raw[index * 2])?;
        let low = hex_value(raw[index * 2 + 1])?;
        bytes[index] = (high << 4) | low;
    }
    Ok(bytes)
}

fn hex_value(byte: u8) -> Result<u8, WeixinPayloadCipherError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(WeixinPayloadCipherError::InvalidDataKey),
    }
}

fn storage_safe_marker(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    ["token", "context", "raw", "data_key", "data-key", "secret"]
        .iter()
        .all(|marker| !lower.contains(marker))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WeixinInboundKind;

    const DATA_KEY: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
    const WRONG_DATA_KEY: &str = "1f1e1d1c1b1a191817161514131211100f0e0d0c0b0a09080706050403020100";

    fn plaintext() -> WeixinPendingInboundPayload {
        WeixinPendingInboundPayload {
            schema_version: crate::inbound::WEIXIN_PENDING_PAYLOAD_SCHEMA_VERSION,
            payload_kind: WeixinInboundKind::Text,
            account_id: "account#933b5bde".to_string(),
            peer_id_hash: "peer#00000001".to_string(),
            message_id_hash: "message#00000001".to_string(),
            item_id: "item#00000001".to_string(),
            direct_message_key: "dm#00000001".to_string(),
            created_at_millis: 42,
            context_reference_id: Some("context#00000001".to_string()),
            reply_to_user_id: Some(SecretString::new("raw-user")),
            reply_context_token: Some(SecretString::new("raw-context")),
            text: Some(SecretString::new("raw message body")),
            voice: None,
        }
    }

    fn aad() -> WeixinPayloadAad {
        WeixinPayloadAad::new(
            "account#933b5bde",
            "peer#00000001",
            "message#00000001",
            "item#00000001",
        )
    }

    #[test]
    fn encrypt_decrypt_round_trip_and_random_nonce() {
        let cipher = WeixinPayloadCipher::new();
        let key = SecretString::new(DATA_KEY);
        let plaintext = plaintext();
        let encrypted_a = cipher
            .encrypt_pending_inbound(&key, &plaintext, &aad())
            .expect("encrypt a");
        let encrypted_b = cipher
            .encrypt_pending_inbound(&key, &plaintext, &aad())
            .expect("encrypt b");
        assert_ne!(encrypted_a.nonce, encrypted_b.nonce);
        assert_ne!(encrypted_a.ciphertext, encrypted_b.ciphertext);
        assert_eq!(encrypted_a.algorithm, WEIXIN_PAYLOAD_ALGORITHM);
        assert_eq!(
            encrypted_a.algorithm_version,
            WEIXIN_PAYLOAD_ALGORITHM_VERSION
        );
        assert_eq!(encrypted_a.aad_version, WEIXIN_PAYLOAD_AAD_VERSION);

        let decrypted = cipher
            .decrypt(&key, &encrypted_a, &aad())
            .expect("decrypt encrypted payload");
        let recovered =
            WeixinPendingInboundPayload::from_plaintext_bytes(&decrypted).expect("payload json");
        assert_eq!(
            recovered.text.as_ref().expect("text").expose(),
            "raw message body"
        );
    }

    #[test]
    fn wrong_key_tamper_truncation_and_unknown_metadata_fail() {
        let cipher = WeixinPayloadCipher::new();
        let key = SecretString::new(DATA_KEY);
        let encrypted = cipher
            .encrypt_pending_inbound(&key, &plaintext(), &aad())
            .expect("encrypt");

        assert!(matches!(
            cipher.decrypt(&SecretString::new(WRONG_DATA_KEY), &encrypted, &aad()),
            Err(WeixinPayloadCipherError::DecryptionFailed)
        ));

        let mut tampered = encrypted.clone();
        tampered.ciphertext = STANDARD.encode(b"tampered ciphertext");
        assert!(matches!(
            cipher.decrypt(&key, &tampered, &aad()),
            Err(WeixinPayloadCipherError::DecryptionFailed)
        ));

        let mut truncated = encrypted.clone();
        let mut ciphertext = STANDARD.decode(&truncated.ciphertext).expect("base64");
        ciphertext.truncate(ciphertext.len().saturating_sub(1));
        truncated.ciphertext = STANDARD.encode(ciphertext);
        assert!(matches!(
            cipher.decrypt(&key, &truncated, &aad()),
            Err(WeixinPayloadCipherError::DecryptionFailed)
        ));

        let mut unknown_algorithm = encrypted.clone();
        unknown_algorithm.algorithm = "aes-256-gcm".to_string();
        assert!(matches!(
            cipher.decrypt(&key, &unknown_algorithm, &aad()),
            Err(WeixinPayloadCipherError::UnsupportedAlgorithm)
        ));

        let mut unknown_algorithm_version = encrypted.clone();
        unknown_algorithm_version.algorithm_version = WEIXIN_PAYLOAD_ALGORITHM_VERSION + 1;
        assert!(matches!(
            cipher.decrypt(&key, &unknown_algorithm_version, &aad()),
            Err(WeixinPayloadCipherError::UnsupportedAlgorithmVersion)
        ));

        let mut unknown_aad_version = encrypted;
        unknown_aad_version.aad_version = WEIXIN_PAYLOAD_AAD_VERSION + 1;
        assert!(matches!(
            cipher.decrypt(&key, &unknown_aad_version, &aad()),
            Err(WeixinPayloadCipherError::UnsupportedAadVersion)
        ));
    }

    #[test]
    fn aad_binding_rejects_account_peer_message_and_item_changes() {
        let cipher = WeixinPayloadCipher::new();
        let key = SecretString::new(DATA_KEY);
        let encrypted = cipher
            .encrypt_pending_inbound(&key, &plaintext(), &aad())
            .expect("encrypt");
        for aad in [
            WeixinPayloadAad::new(
                "account#00000000",
                "peer#00000001",
                "message#00000001",
                "item#00000001",
            ),
            WeixinPayloadAad::new(
                "account#933b5bde",
                "peer#00000002",
                "message#00000001",
                "item#00000001",
            ),
            WeixinPayloadAad::new(
                "account#933b5bde",
                "peer#00000001",
                "message#00000002",
                "item#00000001",
            ),
            WeixinPayloadAad::new(
                "account#933b5bde",
                "peer#00000001",
                "message#00000001",
                "item#00000002",
            ),
        ] {
            assert!(matches!(
                cipher.decrypt(&key, &encrypted, &aad),
                Err(WeixinPayloadCipherError::DecryptionFailed)
            ));
        }
    }
}

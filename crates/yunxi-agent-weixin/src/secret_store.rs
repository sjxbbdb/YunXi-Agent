use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::redaction::redacted_identifier;
use crate::{SecretString, WeixinAccountId};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WeixinCredentialReference {
    pub backend: String,
    pub token_target: String,
    pub data_key_target: String,
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum WeixinSecretStoreError {
    #[error("system credential store is unavailable")]
    Unavailable,
    #[error("credential is not present")]
    NotFound,
    #[error("credential store permission denied")]
    PermissionDenied,
    #[error("credential store operation failed category={category}")]
    Operation { category: &'static str },
    #[error("credential value is invalid")]
    InvalidValue,
}

pub trait WeixinSecretStore: Send + Sync {
    fn credential_reference(&self, account_id: &WeixinAccountId) -> WeixinCredentialReference;

    fn put_token(
        &self,
        account_id: &WeixinAccountId,
        token: &SecretString,
        encryption_key_ref: &str,
    ) -> Result<WeixinCredentialReference, WeixinSecretStoreError>;

    fn get_token(
        &self,
        account_id: &WeixinAccountId,
    ) -> Result<SecretString, WeixinSecretStoreError>;

    fn delete_token(&self, account_id: &WeixinAccountId) -> Result<(), WeixinSecretStoreError>;

    fn put_data_key(
        &self,
        account_id: &WeixinAccountId,
        key: &SecretString,
    ) -> Result<(), WeixinSecretStoreError>;

    fn get_data_key(
        &self,
        account_id: &WeixinAccountId,
    ) -> Result<SecretString, WeixinSecretStoreError>;

    fn delete_data_key(&self, account_id: &WeixinAccountId) -> Result<(), WeixinSecretStoreError>;
}

#[derive(Clone, Debug)]
pub struct SystemWeixinSecretStore {
    installation_id: String,
}

impl SystemWeixinSecretStore {
    pub fn new() -> Self {
        let installation_source = std::env::var("YUNXI_INSTALL_ID")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                std::env::var("LOCALAPPDATA")
                    .ok()
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| format!("{value}\\YunXi Agent"))
            })
            .or_else(|| {
                std::env::var("YUNXI_HOME")
                    .ok()
                    .filter(|value| !value.trim().is_empty())
            })
            .or_else(|| {
                std::env::current_exe().ok().and_then(|path| {
                    path.parent()
                        .map(|parent| parent.to_string_lossy().into_owned())
                })
            })
            .unwrap_or_else(|| "yunxi-default-installation".to_string());
        Self {
            installation_id: redacted_identifier("installation", &installation_source)
                .replace('#', "-"),
        }
    }
}

impl Default for SystemWeixinSecretStore {
    fn default() -> Self {
        Self::new()
    }
}

impl WeixinSecretStore for SystemWeixinSecretStore {
    fn credential_reference(&self, account_id: &WeixinAccountId) -> WeixinCredentialReference {
        credential_reference(
            "windows-credential-manager",
            &self.installation_id,
            account_id,
        )
    }

    fn put_token(
        &self,
        account_id: &WeixinAccountId,
        token: &SecretString,
        encryption_key_ref: &str,
    ) -> Result<WeixinCredentialReference, WeixinSecretStoreError> {
        if token.is_empty() || encryption_key_ref.trim().is_empty() {
            return Err(WeixinSecretStoreError::InvalidValue);
        }
        let reference = self.credential_reference(account_id);
        system_backend::put(&reference.token_target, token.expose().as_bytes())?;
        Ok(reference)
    }

    fn get_token(
        &self,
        account_id: &WeixinAccountId,
    ) -> Result<SecretString, WeixinSecretStoreError> {
        let reference = self.credential_reference(account_id);
        let bytes = system_backend::get(&reference.token_target)?;
        String::from_utf8(bytes)
            .map(SecretString::new)
            .map_err(|_| WeixinSecretStoreError::InvalidValue)
    }

    fn delete_token(&self, account_id: &WeixinAccountId) -> Result<(), WeixinSecretStoreError> {
        let reference = self.credential_reference(account_id);
        system_backend::delete(&reference.token_target)
    }

    fn put_data_key(
        &self,
        account_id: &WeixinAccountId,
        key: &SecretString,
    ) -> Result<(), WeixinSecretStoreError> {
        if key.is_empty() {
            return Err(WeixinSecretStoreError::InvalidValue);
        }
        let reference = self.credential_reference(account_id);
        system_backend::put(&reference.data_key_target, key.expose().as_bytes())
    }

    fn get_data_key(
        &self,
        account_id: &WeixinAccountId,
    ) -> Result<SecretString, WeixinSecretStoreError> {
        let reference = self.credential_reference(account_id);
        let bytes = system_backend::get(&reference.data_key_target)?;
        String::from_utf8(bytes)
            .map(SecretString::new)
            .map_err(|_| WeixinSecretStoreError::InvalidValue)
    }

    fn delete_data_key(&self, account_id: &WeixinAccountId) -> Result<(), WeixinSecretStoreError> {
        let reference = self.credential_reference(account_id);
        system_backend::delete(&reference.data_key_target)
    }
}

#[derive(Default)]
pub struct FakeWeixinSecretStore {
    state: Mutex<FakeState>,
}

#[derive(Default)]
struct FakeState {
    available: bool,
    fail_writes: bool,
    tokens: BTreeMap<String, SecretString>,
    data_keys: BTreeMap<String, SecretString>,
}

impl FakeWeixinSecretStore {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(FakeState {
                available: true,
                ..FakeState::default()
            }),
        }
    }

    pub fn unavailable() -> Self {
        let store = Self::new();
        store.set_available(false);
        store
    }

    pub fn set_available(&self, available: bool) {
        if let Ok(mut state) = self.state.lock() {
            state.available = available;
        }
    }

    pub fn set_fail_writes(&self, fail_writes: bool) {
        if let Ok(mut state) = self.state.lock() {
            state.fail_writes = fail_writes;
        }
    }

    fn with_state<T>(
        &self,
        operation: impl FnOnce(&mut FakeState) -> Result<T, WeixinSecretStoreError>,
    ) -> Result<T, WeixinSecretStoreError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| WeixinSecretStoreError::Operation { category: "lock" })?;
        if !state.available {
            return Err(WeixinSecretStoreError::Unavailable);
        }
        operation(&mut state)
    }
}

impl WeixinSecretStore for FakeWeixinSecretStore {
    fn credential_reference(&self, account_id: &WeixinAccountId) -> WeixinCredentialReference {
        credential_reference("fake-test-only", "installation-test", account_id)
    }

    fn put_token(
        &self,
        account_id: &WeixinAccountId,
        token: &SecretString,
        encryption_key_ref: &str,
    ) -> Result<WeixinCredentialReference, WeixinSecretStoreError> {
        if token.is_empty() || encryption_key_ref.trim().is_empty() {
            return Err(WeixinSecretStoreError::InvalidValue);
        }
        let reference = self.credential_reference(account_id);
        let account_key = redacted_identifier("account", account_id.as_str());
        self.with_state(|state| {
            if state.fail_writes {
                return Err(WeixinSecretStoreError::Operation { category: "write" });
            }
            state.tokens.insert(account_key, token.clone());
            Ok(reference)
        })
    }

    fn get_token(
        &self,
        account_id: &WeixinAccountId,
    ) -> Result<SecretString, WeixinSecretStoreError> {
        let account_key = redacted_identifier("account", account_id.as_str());
        self.with_state(|state| {
            state
                .tokens
                .get(&account_key)
                .cloned()
                .ok_or(WeixinSecretStoreError::NotFound)
        })
    }

    fn delete_token(&self, account_id: &WeixinAccountId) -> Result<(), WeixinSecretStoreError> {
        let account_key = redacted_identifier("account", account_id.as_str());
        self.with_state(|state| {
            state.tokens.remove(&account_key);
            Ok(())
        })
    }

    fn put_data_key(
        &self,
        account_id: &WeixinAccountId,
        key: &SecretString,
    ) -> Result<(), WeixinSecretStoreError> {
        if key.is_empty() {
            return Err(WeixinSecretStoreError::InvalidValue);
        }
        let account_key = redacted_identifier("account", account_id.as_str());
        self.with_state(|state| {
            if state.fail_writes {
                return Err(WeixinSecretStoreError::Operation { category: "write" });
            }
            state.data_keys.insert(account_key, key.clone());
            Ok(())
        })
    }

    fn get_data_key(
        &self,
        account_id: &WeixinAccountId,
    ) -> Result<SecretString, WeixinSecretStoreError> {
        let account_key = redacted_identifier("account", account_id.as_str());
        self.with_state(|state| {
            state
                .data_keys
                .get(&account_key)
                .cloned()
                .ok_or(WeixinSecretStoreError::NotFound)
        })
    }

    fn delete_data_key(&self, account_id: &WeixinAccountId) -> Result<(), WeixinSecretStoreError> {
        let account_key = redacted_identifier("account", account_id.as_str());
        self.with_state(|state| {
            state.data_keys.remove(&account_key);
            Ok(())
        })
    }
}

pub fn generate_data_key() -> Result<SecretString, WeixinSecretStoreError> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes)
        .map_err(|_| WeixinSecretStoreError::Operation { category: "random" })?;
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        hex.push(char::from(b"0123456789abcdef"[(byte >> 4) as usize]));
        hex.push(char::from(b"0123456789abcdef"[(byte & 0x0f) as usize]));
    }
    Ok(SecretString::new(hex))
}

fn credential_reference(
    backend: &str,
    installation_id: &str,
    account_id: &WeixinAccountId,
) -> WeixinCredentialReference {
    let safe_account = redacted_identifier("account", account_id.as_str()).replace('#', "-");
    let prefix = format!("YunXiAgent/Weixin/{installation_id}/{safe_account}");
    WeixinCredentialReference {
        backend: backend.to_string(),
        token_target: format!("{prefix}/token"),
        data_key_target: format!("{prefix}/data-key"),
    }
}

#[cfg(windows)]
mod system_backend {
    use std::{ffi::c_void, ptr, slice};

    use windows_sys::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_NOT_FOUND, GetLastError};
    use windows_sys::Win32::Security::Credentials::{
        CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW, CredDeleteW, CredFree,
        CredReadW, CredWriteW,
    };

    use super::WeixinSecretStoreError;

    pub fn put(target: &str, value: &[u8]) -> Result<(), WeixinSecretStoreError> {
        let mut target = wide(target);
        let mut value = value.to_vec();
        let credential = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: target.as_mut_ptr(),
            CredentialBlobSize: value.len() as u32,
            CredentialBlob: value.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            ..CREDENTIALW::default()
        };
        let result = unsafe { CredWriteW(&credential, 0) };
        if result == 0 {
            return Err(map_error());
        }
        Ok(())
    }

    pub fn get(target: &str) -> Result<Vec<u8>, WeixinSecretStoreError> {
        let target = wide(target);
        let mut credential = ptr::null_mut();
        let result = unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) };
        if result == 0 {
            return Err(map_error());
        }
        let bytes = unsafe {
            slice::from_raw_parts(
                (*credential).CredentialBlob,
                (*credential).CredentialBlobSize as usize,
            )
            .to_vec()
        };
        unsafe { CredFree(credential.cast::<c_void>()) };
        Ok(bytes)
    }

    pub fn delete(target: &str) -> Result<(), WeixinSecretStoreError> {
        let target = wide(target);
        let result = unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) };
        if result == 0 {
            return Err(map_error());
        }
        Ok(())
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn map_error() -> WeixinSecretStoreError {
        match unsafe { GetLastError() } {
            ERROR_NOT_FOUND => WeixinSecretStoreError::NotFound,
            ERROR_ACCESS_DENIED => WeixinSecretStoreError::PermissionDenied,
            _ => WeixinSecretStoreError::Operation {
                category: "windows",
            },
        }
    }
}

#[cfg(not(windows))]
mod system_backend {
    use super::WeixinSecretStoreError;

    pub fn put(_target: &str, _value: &[u8]) -> Result<(), WeixinSecretStoreError> {
        Err(WeixinSecretStoreError::Unavailable)
    }

    pub fn get(_target: &str) -> Result<Vec<u8>, WeixinSecretStoreError> {
        Err(WeixinSecretStoreError::Unavailable)
    }

    pub fn delete(_target: &str) -> Result<(), WeixinSecretStoreError> {
        Err(WeixinSecretStoreError::Unavailable)
    }
}

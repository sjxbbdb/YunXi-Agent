use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::redaction::redacted_identifier;
use crate::{
    PRODUCTION_ILINK_ENDPOINT, WeixinAccountId, WeixinConnectionState, WeixinCredentialReference,
};

pub const WEIXIN_ACCOUNT_SCHEMA_VERSION: u32 = 1;
const WEIXIN_METADATA_DIRECTORY: &str = ".yunxi/weixin";

#[derive(Debug, Error)]
pub enum WeixinAccountStoreError {
    #[error("account metadata I/O failed operation={operation}")]
    Io { operation: &'static str },
    #[error("account metadata JSON is invalid")]
    InvalidJson,
    #[error("account metadata failed validation")]
    InvalidRecord,
    #[error("workspace path is unavailable")]
    WorkspaceUnavailable,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WeixinAccountRecord {
    pub schema_version: u32,
    pub account_id: String,
    pub connection_state: WeixinConnectionState,
    pub endpoint: String,
    pub credential: WeixinCredentialReference,
    pub workspace_id: String,
    pub created_at_millis: u64,
    pub updated_at_millis: u64,
}

impl WeixinAccountRecord {
    pub fn new(
        account_id: &WeixinAccountId,
        credential: WeixinCredentialReference,
        workspace: &Path,
        now_millis: u64,
    ) -> Result<Self, WeixinAccountStoreError> {
        let workspace = workspace
            .canonicalize()
            .map_err(|_| WeixinAccountStoreError::WorkspaceUnavailable)?;
        Ok(Self {
            schema_version: WEIXIN_ACCOUNT_SCHEMA_VERSION,
            account_id: redacted_identifier("account", account_id.as_str()),
            connection_state: WeixinConnectionState::Ready,
            endpoint: PRODUCTION_ILINK_ENDPOINT.to_string(),
            credential,
            workspace_id: redacted_identifier("workspace", &workspace.to_string_lossy()),
            created_at_millis: now_millis,
            updated_at_millis: now_millis,
        })
    }
}

#[derive(Clone, Debug)]
pub struct WeixinAccountStore {
    workspace: PathBuf,
}

impl WeixinAccountStore {
    pub fn new(workspace: impl Into<PathBuf>) -> Self {
        Self {
            workspace: workspace.into(),
        }
    }

    pub fn metadata_directory(&self) -> PathBuf {
        self.workspace.join(WEIXIN_METADATA_DIRECTORY)
    }

    pub fn path_for(&self, account_id: &WeixinAccountId) -> PathBuf {
        self.metadata_directory().join(format!(
            "{}.json",
            redacted_identifier("account", account_id.as_str()).replace('#', "-")
        ))
    }

    pub fn load(
        &self,
        account_id: &WeixinAccountId,
    ) -> Result<Option<WeixinAccountRecord>, WeixinAccountStoreError> {
        let path = self.path_for(account_id);
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(WeixinAccountStoreError::Io { operation: "read" }),
        };
        let record: WeixinAccountRecord =
            serde_json::from_slice(&bytes).map_err(|_| WeixinAccountStoreError::InvalidJson)?;
        if !record_is_safe(account_id, &record) {
            return Err(WeixinAccountStoreError::InvalidRecord);
        }
        Ok(Some(record))
    }

    pub fn save(
        &self,
        account_id: &WeixinAccountId,
        record: &WeixinAccountRecord,
    ) -> Result<(), WeixinAccountStoreError> {
        if !record_is_safe(account_id, record) {
            return Err(WeixinAccountStoreError::InvalidRecord);
        }
        fs::create_dir_all(self.metadata_directory()).map_err(|_| WeixinAccountStoreError::Io {
            operation: "create",
        })?;
        let bytes =
            serde_json::to_vec_pretty(record).map_err(|_| WeixinAccountStoreError::InvalidJson)?;
        let path = self.path_for(account_id);
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)
            .map_err(|_| WeixinAccountStoreError::Io { operation: "open" })?;
        file.write_all(&bytes)
            .map_err(|_| WeixinAccountStoreError::Io { operation: "write" })?;
        file.sync_all()
            .map_err(|_| WeixinAccountStoreError::Io { operation: "sync" })?;
        Ok(())
    }

    pub fn delete(&self, account_id: &WeixinAccountId) -> Result<(), WeixinAccountStoreError> {
        let path = self.path_for(account_id);
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(WeixinAccountStoreError::Io {
                operation: "delete",
            }),
        }
    }
}

fn record_is_safe(account_id: &WeixinAccountId, record: &WeixinAccountRecord) -> bool {
    record.schema_version == WEIXIN_ACCOUNT_SCHEMA_VERSION
        && record.account_id == redacted_identifier("account", account_id.as_str())
        && record.endpoint == PRODUCTION_ILINK_ENDPOINT
        && !record.credential.backend.trim().is_empty()
        && !record.credential.token_target.trim().is_empty()
        && !record.credential.data_key_target.trim().is_empty()
        && !record.workspace_id.trim().is_empty()
}

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::hash_map::DefaultHasher,
    fmt,
    fs::{self, File, OpenOptions},
    hash::{Hash, Hasher},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

pub const WEIXIN_STATE_SCHEMA_VERSION: u32 = 3;
pub const WEIXIN_PAYLOAD_ALGORITHM: &str = "chacha20-poly1305";
pub const WEIXIN_PAYLOAD_ALGORITHM_VERSION: u32 = 1;
pub const WEIXIN_PAYLOAD_AAD_VERSION: u32 = 1;
pub const WEIXIN_PAYLOAD_NONCE_LENGTH: usize = 12;
const WEIXIN_STATE_DIRECTORY: &str = ".yunxi/weixin/state";
const WEIXIN_LOCK_ENV: &str = "YUNXI_WEIXIN_LOCK_ROOT";
const WEIXIN_PENDING_INBOUND_LIMIT: usize = 1024;
const WEIXIN_TERMINAL_RECEIPT_LIMIT: usize = 2048;
const WEIXIN_TERMINAL_RECEIPT_TTL_MILLIS: u64 = 7 * 24 * 60 * 60 * 1000;

pub trait WeixinStateStore: Send + Sync {
    fn root(&self) -> &Path;
    fn load(&self, account_id: &str) -> Result<Option<WeixinStateSnapshot>, WeixinStateError>;
    fn save(&self, snapshot: &WeixinStateSnapshot) -> Result<(), WeixinStateError>;
    fn save_with_options(
        &self,
        snapshot: &WeixinStateSnapshot,
        options: WeixinStateWriteOptions,
    ) -> Result<(), WeixinStateError>;
    fn delete(&self, account_id: &str) -> Result<(), WeixinStateError>;
    fn temp_file_candidates(&self) -> Result<Vec<PathBuf>, WeixinStateError>;
}

#[derive(Clone)]
pub struct FileWeixinStateStore {
    root: PathBuf,
    lock_root: PathBuf,
    process_probe: Arc<dyn Fn(u32) -> bool + Send + Sync>,
}

impl fmt::Debug for FileWeixinStateStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FileWeixinStateStore")
            .field("root", &self.root)
            .field("lock_root", &self.lock_root)
            .finish_non_exhaustive()
    }
}

impl FileWeixinStateStore {
    pub fn for_workspace(workspace: impl AsRef<Path>) -> Self {
        let workspace = workspace.as_ref();
        Self {
            root: workspace.join(WEIXIN_STATE_DIRECTORY),
            lock_root: default_lock_root().unwrap_or_else(|| {
                std::env::temp_dir()
                    .join("YunXi Agent")
                    .join("weixin-account-locks")
            }),
            process_probe: Arc::new(default_process_is_running),
        }
    }

    pub fn for_workspace_with_lock_root(
        workspace: impl AsRef<Path>,
        lock_root: impl Into<PathBuf>,
    ) -> Self {
        let workspace = workspace.as_ref();
        Self {
            root: workspace.join(WEIXIN_STATE_DIRECTORY),
            lock_root: lock_root.into(),
            process_probe: Arc::new(default_process_is_running),
        }
    }

    pub fn with_process_probe<F>(mut self, probe: F) -> Self
    where
        F: Fn(u32) -> bool + Send + Sync + 'static,
    {
        self.process_probe = Arc::new(probe);
        self
    }

    pub fn state_path_for(&self, account_id: &str) -> PathBuf {
        self.root
            .join(format!("{}.json", safe_file_component(account_id)))
    }

    pub fn lock_path_for(&self, account_id: &str) -> PathBuf {
        self.lock_root
            .join(format!("{}.lock.json", safe_file_component(account_id)))
    }

    pub fn lock_state(&self, account_id: &str) -> Result<WeixinAccountLockInfo, WeixinStateError> {
        let path = self.lock_path_for(account_id);
        if !path.is_file() {
            return Ok(WeixinAccountLockInfo::free());
        }
        let record = read_lock_record(&path)?;
        let active = (self.process_probe)(record.pid);
        Ok(WeixinAccountLockInfo {
            state: if active {
                WeixinAccountLockState::Active
            } else {
                WeixinAccountLockState::Stale
            },
            pid: Some(record.pid),
            workspace_id: Some(record.workspace_id),
            created_at_millis: Some(record.created_at_millis),
        })
    }

    pub fn try_acquire_account_lock(
        &self,
        account_id: &str,
        workspace_id: &str,
    ) -> Result<WeixinAccountLock, WeixinStateError> {
        fs::create_dir_all(&self.lock_root).map_err(|error| WeixinStateError::Io {
            operation: "create_lock_dir",
            path: redacted_path(&self.lock_root),
            source: error,
        })?;
        let path = self.lock_path_for(account_id);
        let now = now_millis() as u64;
        let record = WeixinLockRecord::new(account_id, workspace_id, now);
        match write_lock_file(&path, &record) {
            Ok(()) => Ok(WeixinAccountLock {
                path,
                lock_id: record.lock_id,
                released: false,
            }),
            Err(WeixinStateError::LockAlreadyExists) => {
                let existing = read_lock_record(&path)?;
                if (self.process_probe)(existing.pid) {
                    return Err(WeixinStateError::LockActive {
                        account_id: account_id.to_string(),
                    });
                }
                fs::remove_file(&path).map_err(|error| WeixinStateError::Io {
                    operation: "remove_stale_lock",
                    path: redacted_path(&path),
                    source: error,
                })?;
                write_lock_file(&path, &record)?;
                Ok(WeixinAccountLock {
                    path,
                    lock_id: record.lock_id,
                    released: false,
                })
            }
            Err(error) => Err(error),
        }
    }

    pub fn upsert_account_state(
        &self,
        account_id: &str,
        workspace_id: &str,
        endpoint: &str,
        credential: Option<WeixinCredentialReferenceRecord>,
        now_millis: u64,
    ) -> Result<WeixinStateSnapshot, WeixinStateError> {
        let mut snapshot = self.load(account_id)?.unwrap_or_else(|| {
            WeixinStateSnapshot::new(account_id, workspace_id, endpoint, now_millis)
        });
        snapshot.workspace_id = workspace_id.to_string();
        snapshot.endpoint = endpoint.to_string();
        snapshot.credential = credential;
        snapshot.connection_state = WeixinConnectionStateRecord::Ready;
        snapshot.updated_at_millis = now_millis;
        snapshot.transitioned_at_millis = now_millis;
        self.save(&snapshot)?;
        Ok(snapshot)
    }

    fn load_value(&self, path: &Path) -> Result<Value, WeixinStateError> {
        let mut content = String::new();
        File::open(path)
            .map_err(|error| WeixinStateError::Io {
                operation: "open",
                path: redacted_path(path),
                source: error,
            })?
            .read_to_string(&mut content)
            .map_err(|error| WeixinStateError::Io {
                operation: "read",
                path: redacted_path(path),
                source: error,
            })?;
        serde_json::from_str(&content).map_err(|_| WeixinStateError::InvalidJson {
            path: redacted_path(path),
        })
    }

    fn write_snapshot(
        &self,
        snapshot: &WeixinStateSnapshot,
        options: WeixinStateWriteOptions,
    ) -> Result<(), WeixinStateError> {
        validate_snapshot(snapshot)?;
        let path = self.state_path_for(&snapshot.account_id);
        let parent = path
            .parent()
            .ok_or_else(|| WeixinStateError::InvalidRecord {
                reason: "state path has no parent",
            })?;
        fs::create_dir_all(parent).map_err(|error| WeixinStateError::Io {
            operation: "create_state_dir",
            path: redacted_path(parent),
            source: error,
        })?;
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("weixin-state.json");
        let temp_path = parent.join(format!(
            ".{file_name}.tmp.{}.{}",
            std::process::id(),
            now_millis()
        ));
        let bytes =
            serde_json::to_vec_pretty(snapshot).map_err(|_| WeixinStateError::InvalidRecord {
                reason: "state serialization failed",
            })?;
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path)
            .map_err(|error| WeixinStateError::Io {
                operation: "create_temp",
                path: redacted_path(&temp_path),
                source: error,
            })?;
        file.write_all(&bytes)
            .and_then(|_| file.write_all(b"\n"))
            .and_then(|_| file.flush())
            .and_then(|_| file.sync_all())
            .map_err(|error| WeixinStateError::Io {
                operation: "write_temp",
                path: redacted_path(&temp_path),
                source: error,
            })?;
        drop(file);
        if options.fail_before_replace {
            return Err(WeixinStateError::InjectedFailure {
                operation: "before_replace",
            });
        }
        replace_file(&temp_path, &path)?;
        sync_parent_best_effort(parent);
        Ok(())
    }

    pub fn add_pair_request(
        &self,
        account_id: &str,
        peer_id_hash: &str,
        expires_at_millis: u64,
        now_millis: u64,
    ) -> Result<WeixinPairRequest, WeixinStateError> {
        let mut snapshot = self.load_required(account_id)?;
        let request = WeixinPairRequest {
            schema_version: WEIXIN_STATE_SCHEMA_VERSION,
            request_id: opaque_request_id(account_id, peer_id_hash, now_millis),
            account_id: account_id.to_string(),
            peer_id_hash: peer_id_hash.to_string(),
            state: WeixinPairRequestState::Pending,
            expires_at_millis,
            created_at_millis: now_millis,
            updated_at_millis: now_millis,
            transitioned_at_millis: now_millis,
        };
        snapshot.pair_requests.push(request.clone());
        snapshot.updated_at_millis = now_millis;
        self.save(&snapshot)?;
        Ok(request)
    }

    pub fn commit_inbound_batch(
        &self,
        commit: WeixinInboundBatchCommit,
    ) -> Result<WeixinInboundBatchCommitResult, WeixinStateError> {
        self.commit_inbound_batch_with_options(commit, WeixinStateWriteOptions::default())
    }

    pub fn commit_inbound_batch_with_options(
        &self,
        commit: WeixinInboundBatchCommit,
        options: WeixinStateWriteOptions,
    ) -> Result<WeixinInboundBatchCommitResult, WeixinStateError> {
        let mut snapshot = self.load_required(&commit.account_id)?;
        prune_terminal_receipts(&mut snapshot, commit.now_millis);
        let mut accepted_count = 0;
        let mut accepted_item_ids = Vec::new();
        let mut duplicate_count = 0;
        let mut pair_request_count = 0;
        let active_pending = snapshot.pending_inbound_count();
        if active_pending.saturating_add(commit.accepted.len()) > WEIXIN_PENDING_INBOUND_LIMIT {
            return Err(WeixinStateError::PendingInboundQueueFull {
                limit: WEIXIN_PENDING_INBOUND_LIMIT,
            });
        }

        for item in commit.accepted {
            if has_receipt(&snapshot, &item.message_id_hash, &item.peer_id_hash)
                || has_pending_inbound(&snapshot, &item.message_id_hash, &item.peer_id_hash)
            {
                duplicate_count += 1;
                continue;
            }
            snapshot.inbound_receipts.push(WeixinInboundReceiptRecord {
                schema_version: WEIXIN_STATE_SCHEMA_VERSION,
                message_id_hash: item.message_id_hash.clone(),
                peer_id_hash: item.peer_id_hash.clone(),
                accepted_at_millis: commit.now_millis,
                state: WeixinReceiptState::Ready,
                created_at_millis: commit.now_millis,
                updated_at_millis: commit.now_millis,
                transitioned_at_millis: commit.now_millis,
            });
            let item_id = item.item_id;
            snapshot.pending_inbound.push(WeixinPendingInbound {
                schema_version: WEIXIN_STATE_SCHEMA_VERSION,
                item_id: item_id.clone(),
                account_id: commit.account_id.clone(),
                message_id_hash: item.message_id_hash,
                peer_id_hash: item.peer_id_hash,
                encrypted_payload_ref: item.encrypted_payload_ref,
                payload_kind: item.payload_kind,
                encrypted_payload: Some(item.encrypted_payload),
                state: WeixinPendingInboundState::Ready,
                terminal_reason: None,
                created_at_millis: commit.now_millis,
                updated_at_millis: commit.now_millis,
                transitioned_at_millis: commit.now_millis,
            });
            accepted_item_ids.push(item_id);
            accepted_count += 1;
        }

        for request in commit.pair_requests {
            if has_active_pair_request(&snapshot, &request.peer_id_hash, commit.now_millis) {
                continue;
            }
            snapshot.pair_requests.push(WeixinPairRequest {
                schema_version: WEIXIN_STATE_SCHEMA_VERSION,
                request_id: opaque_request_id(
                    &commit.account_id,
                    &request.peer_id_hash,
                    commit.now_millis,
                ),
                account_id: commit.account_id.clone(),
                peer_id_hash: request.peer_id_hash,
                state: WeixinPairRequestState::Pending,
                expires_at_millis: request.expires_at_millis,
                created_at_millis: commit.now_millis,
                updated_at_millis: commit.now_millis,
                transitioned_at_millis: commit.now_millis,
            });
            pair_request_count += 1;
        }

        if let Some(cursor) = commit.next_get_updates_buf {
            upsert_cursor(
                &mut snapshot,
                &commit.account_id,
                &commit.cursor_source,
                cursor,
                commit.now_millis,
            );
        }
        snapshot.connection_state = commit
            .connection_state
            .unwrap_or(WeixinConnectionStateRecord::Ready);
        snapshot.last_redacted_error = commit.last_redacted_error;
        snapshot.updated_at_millis = commit.now_millis;
        snapshot.transitioned_at_millis = commit.now_millis;
        let receipt_count = snapshot.inbound_receipts.len();
        let pending_inbound_count = snapshot.pending_inbound_count();
        self.save_with_options(&snapshot, options)?;
        Ok(WeixinInboundBatchCommitResult {
            accepted_count,
            accepted_item_ids,
            duplicate_count,
            pair_request_count,
            receipt_count,
            pending_inbound_count,
        })
    }

    pub fn record_poll_health(
        &self,
        account_id: &str,
        connection_state: WeixinConnectionStateRecord,
        last_redacted_error: Option<String>,
        now_millis: u64,
    ) -> Result<WeixinStateSnapshot, WeixinStateError> {
        let mut snapshot = self.load_required(account_id)?;
        snapshot.connection_state = connection_state;
        snapshot.last_redacted_error = last_redacted_error;
        snapshot.updated_at_millis = now_millis;
        snapshot.transitioned_at_millis = now_millis;
        prune_terminal_receipts(&mut snapshot, now_millis);
        self.save(&snapshot)?;
        Ok(snapshot)
    }

    pub fn approve_pair_request(
        &self,
        account_id: &str,
        request_id: &str,
        now_millis: u64,
    ) -> Result<WeixinPairRequest, WeixinStateError> {
        self.transition_pair_request(
            account_id,
            request_id,
            now_millis,
            WeixinPairRequestState::Approved,
        )
    }

    pub fn deny_pair_request(
        &self,
        account_id: &str,
        request_id: &str,
        now_millis: u64,
    ) -> Result<WeixinPairRequest, WeixinStateError> {
        self.transition_pair_request(
            account_id,
            request_id,
            now_millis,
            WeixinPairRequestState::Denied,
        )
    }

    fn transition_pair_request(
        &self,
        account_id: &str,
        request_id: &str,
        now_millis: u64,
        target: WeixinPairRequestState,
    ) -> Result<WeixinPairRequest, WeixinStateError> {
        let mut snapshot = self.load_required(account_id)?;
        let request = snapshot
            .pair_requests
            .iter_mut()
            .find(|request| request.request_id == request_id)
            .ok_or_else(|| WeixinStateError::PairRequestNotFound {
                request_id: request_id.to_string(),
            })?;
        if request.account_id != account_id {
            return Err(WeixinStateError::PairRequestAccountMismatch);
        }
        if request.state != WeixinPairRequestState::Pending {
            return Err(WeixinStateError::PairRequestConsumed {
                request_id: request_id.to_string(),
            });
        }
        if request.expires_at_millis <= now_millis {
            request.state = WeixinPairRequestState::Expired;
            request.updated_at_millis = now_millis;
            request.transitioned_at_millis = now_millis;
            let expired = request.clone();
            snapshot.updated_at_millis = now_millis;
            self.save(&snapshot)?;
            return Err(WeixinStateError::PairRequestExpired {
                request_id: expired.request_id,
            });
        }
        request.state = target;
        request.updated_at_millis = now_millis;
        request.transitioned_at_millis = now_millis;
        let updated = request.clone();
        snapshot.updated_at_millis = now_millis;
        self.save(&snapshot)?;
        Ok(updated)
    }

    pub fn load_pending_inbound(
        &self,
        account_id: &str,
        item_id: &str,
    ) -> Result<Option<WeixinPendingInbound>, WeixinStateError> {
        let snapshot = self.load_required(account_id)?;
        Ok(snapshot
            .pending_inbound
            .into_iter()
            .find(|item| item.item_id == item_id && !item.state.is_terminal()))
    }

    pub fn begin_pending_runtime_turn(
        &self,
        request: WeixinRuntimeTurnBeginRequest,
    ) -> Result<WeixinConversationBinding, WeixinStateError> {
        if request.source_label.trim().is_empty()
            || contains_sensitive_marker(&request.source_label)
            || request.candidate_session_id.trim().is_empty()
            || contains_sensitive_marker(&request.candidate_session_id)
            || !looks_redacted("account", &request.account_id)
            || !request.peer_id_hash.starts_with("peer#")
            || !request.message_id_hash.starts_with("message#")
            || !request.direct_message_key.starts_with("dm#")
            || !looks_redacted("workspace", &request.workspace_id)
        {
            return Err(WeixinStateError::InvalidRecord {
                reason: "conversation binding request failed validation",
            });
        }
        let mut snapshot = self.load_required(&request.account_id)?;
        let pending = snapshot
            .pending_inbound
            .iter_mut()
            .find(|item| item.item_id == request.item_id && !item.state.is_terminal())
            .ok_or_else(|| WeixinStateError::PendingInboundNotFound {
                item_id: request.item_id.clone(),
            })?;
        if pending.account_id != request.account_id
            || pending.peer_id_hash != request.peer_id_hash
            || pending.message_id_hash != request.message_id_hash
        {
            return Err(WeixinStateError::InvalidRecord {
                reason: "pending inbound binding mismatch",
            });
        }
        pending.transition(WeixinPendingInboundState::Running, request.now_millis)?;
        if let Some(receipt) = snapshot.inbound_receipts.iter_mut().find(|receipt| {
            receipt.message_id_hash == request.message_id_hash
                && receipt.peer_id_hash == request.peer_id_hash
        }) {
            receipt.state = WeixinReceiptState::Running;
            receipt.updated_at_millis = request.now_millis;
            receipt.transitioned_at_millis = request.now_millis;
        }

        let binding = upsert_conversation_binding(
            &mut snapshot,
            &request.account_id,
            &request.peer_id_hash,
            &request.direct_message_key,
            &request.workspace_id,
            &request.candidate_session_id,
            &request.source_label,
            request.now_millis,
        )?;
        snapshot.updated_at_millis = request.now_millis;
        snapshot.transitioned_at_millis = request.now_millis;
        self.save(&snapshot)?;
        Ok(binding)
    }

    pub fn complete_pending_runtime_turn(
        &self,
        account_id: &str,
        item_id: &str,
        target: WeixinPendingInboundState,
        terminal_reason: Option<String>,
        now_millis: u64,
    ) -> Result<WeixinPendingInbound, WeixinStateError> {
        if !target.is_terminal() {
            return Err(WeixinStateError::InvalidRecord {
                reason: "pending inbound terminal state is required",
            });
        }
        if terminal_reason
            .as_deref()
            .is_some_and(contains_sensitive_marker)
        {
            return Err(WeixinStateError::InvalidRecord {
                reason: "pending inbound terminal reason is unsafe",
            });
        }
        let mut snapshot = self.load_required(account_id)?;
        let pending = snapshot
            .pending_inbound
            .iter_mut()
            .find(|item| item.item_id == item_id && !item.state.is_terminal())
            .ok_or_else(|| WeixinStateError::PendingInboundNotFound {
                item_id: item_id.to_string(),
            })?;
        pending.transition(target, now_millis)?;
        pending.terminal_reason = terminal_reason;
        let updated = pending.clone();
        if let Some(receipt) = snapshot.inbound_receipts.iter_mut().find(|receipt| {
            receipt.message_id_hash == updated.message_id_hash
                && receipt.peer_id_hash == updated.peer_id_hash
        }) {
            receipt.state = match target {
                WeixinPendingInboundState::Succeeded => WeixinReceiptState::Succeeded,
                WeixinPendingInboundState::Failed => WeixinReceiptState::Failed,
                WeixinPendingInboundState::Cancelled => WeixinReceiptState::Cancelled,
                WeixinPendingInboundState::Expired => WeixinReceiptState::Expired,
                WeixinPendingInboundState::Unknown => WeixinReceiptState::Unknown,
                WeixinPendingInboundState::Accepted
                | WeixinPendingInboundState::Ready
                | WeixinPendingInboundState::Running => receipt.state,
            };
            receipt.updated_at_millis = now_millis;
            receipt.transitioned_at_millis = now_millis;
        }
        snapshot.updated_at_millis = now_millis;
        snapshot.transitioned_at_millis = now_millis;
        self.save(&snapshot)?;
        Ok(updated)
    }

    fn load_required(&self, account_id: &str) -> Result<WeixinStateSnapshot, WeixinStateError> {
        self.load(account_id)?
            .ok_or_else(|| WeixinStateError::StateNotFound {
                account_id: account_id.to_string(),
            })
    }
}

impl WeixinStateStore for FileWeixinStateStore {
    fn root(&self) -> &Path {
        &self.root
    }

    fn load(&self, account_id: &str) -> Result<Option<WeixinStateSnapshot>, WeixinStateError> {
        let path = self.state_path_for(account_id);
        if !path.is_file() {
            return Ok(None);
        }
        let value = self.load_value(&path)?;
        let value = WeixinStateMigration::migrate_value(value, now_millis() as u64)?;
        let snapshot: WeixinStateSnapshot =
            serde_json::from_value(value).map_err(|_| WeixinStateError::InvalidRecord {
                reason: "state record failed validation",
            })?;
        validate_snapshot(&snapshot)?;
        Ok(Some(snapshot))
    }

    fn save(&self, snapshot: &WeixinStateSnapshot) -> Result<(), WeixinStateError> {
        self.write_snapshot(snapshot, WeixinStateWriteOptions::default())
    }

    fn save_with_options(
        &self,
        snapshot: &WeixinStateSnapshot,
        options: WeixinStateWriteOptions,
    ) -> Result<(), WeixinStateError> {
        self.write_snapshot(snapshot, options)
    }

    fn delete(&self, account_id: &str) -> Result<(), WeixinStateError> {
        let path = self.state_path_for(account_id);
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(WeixinStateError::Io {
                operation: "delete",
                path: redacted_path(&path),
                source: error,
            }),
        }
    }

    fn temp_file_candidates(&self) -> Result<Vec<PathBuf>, WeixinStateError> {
        if !self.root.is_dir() {
            return Ok(Vec::new());
        }
        let mut candidates = Vec::new();
        for entry in fs::read_dir(&self.root).map_err(|error| WeixinStateError::Io {
            operation: "read_temp_dir",
            path: redacted_path(&self.root),
            source: error,
        })? {
            let entry = entry.map_err(|error| WeixinStateError::Io {
                operation: "read_temp_entry",
                path: redacted_path(&self.root),
                source: error,
            })?;
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("");
            if name.contains(".tmp.") {
                candidates.push(path);
            }
        }
        candidates.sort();
        Ok(candidates)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WeixinStateWriteOptions {
    pub fail_before_replace: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeixinInboundBatchCommit {
    pub account_id: String,
    pub cursor_source: String,
    pub next_get_updates_buf: Option<String>,
    pub accepted: Vec<WeixinInboundCommitItem>,
    pub pair_requests: Vec<WeixinPairRequestCommitItem>,
    pub connection_state: Option<WeixinConnectionStateRecord>,
    pub last_redacted_error: Option<String>,
    pub now_millis: u64,
}

impl WeixinInboundBatchCommit {
    pub fn new(account_id: impl Into<String>, now_millis: u64) -> Self {
        Self {
            account_id: account_id.into(),
            cursor_source: "getupdates".to_string(),
            next_get_updates_buf: None,
            accepted: Vec::new(),
            pair_requests: Vec::new(),
            connection_state: Some(WeixinConnectionStateRecord::Ready),
            last_redacted_error: None,
            now_millis,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeixinInboundCommitItem {
    pub item_id: String,
    pub message_id_hash: String,
    pub peer_id_hash: String,
    pub encrypted_payload_ref: String,
    pub encrypted_payload: WeixinEncryptedPayload,
    pub payload_kind: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeixinPairRequestCommitItem {
    pub peer_id_hash: String,
    pub expires_at_millis: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeixinInboundBatchCommitResult {
    pub accepted_count: usize,
    pub accepted_item_ids: Vec<String>,
    pub duplicate_count: usize,
    pub pair_request_count: usize,
    pub receipt_count: usize,
    pub pending_inbound_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WeixinStateSnapshot {
    pub schema_version: u32,
    pub account_id: String,
    pub workspace_id: String,
    pub endpoint: String,
    pub connection_state: WeixinConnectionStateRecord,
    pub credential: Option<WeixinCredentialReferenceRecord>,
    pub cursors: Vec<WeixinCursorRecord>,
    pub inbound_receipts: Vec<WeixinInboundReceiptRecord>,
    pub deliveries: Vec<WeixinDeliveryRecord>,
    pub conversation_bindings: Vec<WeixinConversationBinding>,
    pub reply_contexts: Vec<WeixinReplyContextReference>,
    pub pending_deliveries: Vec<WeixinPendingDeliveryMetadata>,
    pub pair_requests: Vec<WeixinPairRequest>,
    pub pending_inbound: Vec<WeixinPendingInbound>,
    pub last_redacted_error: Option<String>,
    pub created_at_millis: u64,
    pub updated_at_millis: u64,
    pub transitioned_at_millis: u64,
}

impl WeixinStateSnapshot {
    pub fn new(account_id: &str, workspace_id: &str, endpoint: &str, now_millis: u64) -> Self {
        Self {
            schema_version: WEIXIN_STATE_SCHEMA_VERSION,
            account_id: account_id.to_string(),
            workspace_id: workspace_id.to_string(),
            endpoint: endpoint.to_string(),
            connection_state: WeixinConnectionStateRecord::NotConfigured,
            credential: None,
            cursors: Vec::new(),
            inbound_receipts: Vec::new(),
            deliveries: Vec::new(),
            conversation_bindings: Vec::new(),
            reply_contexts: Vec::new(),
            pending_deliveries: Vec::new(),
            pair_requests: Vec::new(),
            pending_inbound: Vec::new(),
            last_redacted_error: None,
            created_at_millis: now_millis,
            updated_at_millis: now_millis,
            transitioned_at_millis: now_millis,
        }
    }

    pub fn pending_inbound_count(&self) -> usize {
        self.pending_inbound
            .iter()
            .filter(|item| !item.state.is_terminal())
            .count()
    }

    pub fn pending_delivery_count(&self) -> usize {
        self.pending_deliveries
            .iter()
            .filter(|item| !item.state.is_terminal())
            .count()
    }

    pub fn pair_request_count(&self) -> usize {
        self.pair_requests
            .iter()
            .filter(|request| request.state == WeixinPairRequestState::Pending)
            .count()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WeixinCredentialReferenceRecord {
    pub backend: String,
    pub token_target: String,
    pub data_key_target: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WeixinCursorRecord {
    pub schema_version: u32,
    pub account_id: String,
    pub get_updates_buf: Option<String>,
    pub source: String,
    pub created_at_millis: u64,
    pub updated_at_millis: u64,
    pub transitioned_at_millis: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WeixinInboundReceiptRecord {
    pub schema_version: u32,
    pub message_id_hash: String,
    pub peer_id_hash: String,
    pub accepted_at_millis: u64,
    pub state: WeixinReceiptState,
    pub created_at_millis: u64,
    pub updated_at_millis: u64,
    pub transitioned_at_millis: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WeixinDeliveryRecord {
    pub schema_version: u32,
    pub delivery_id: String,
    pub state: WeixinDeliveryState,
    pub last_redacted_error: Option<String>,
    pub created_at_millis: u64,
    pub updated_at_millis: u64,
    pub transitioned_at_millis: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WeixinConversationBinding {
    pub schema_version: u32,
    pub account_id: String,
    pub peer_id_hash: String,
    pub direct_message_key: String,
    pub workspace_id: String,
    pub session_id: String,
    pub source_label: String,
    pub created_at_millis: u64,
    pub last_activity_millis: u64,
    pub updated_at_millis: u64,
    pub transitioned_at_millis: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeixinRuntimeTurnBeginRequest {
    pub account_id: String,
    pub peer_id_hash: String,
    pub message_id_hash: String,
    pub item_id: String,
    pub direct_message_key: String,
    pub workspace_id: String,
    pub candidate_session_id: String,
    pub source_label: String,
    pub now_millis: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WeixinReplyContextReference {
    pub schema_version: u32,
    pub reference_id: String,
    pub secret_target: String,
    pub purpose: String,
    pub expires_at_millis: u64,
    pub created_at_millis: u64,
    pub updated_at_millis: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WeixinPendingDeliveryMetadata {
    pub schema_version: u32,
    pub delivery_id: String,
    pub state: WeixinDeliveryState,
    pub retry_count: u32,
    pub created_at_millis: u64,
    pub updated_at_millis: u64,
    pub transitioned_at_millis: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WeixinPairRequest {
    pub schema_version: u32,
    pub request_id: String,
    pub account_id: String,
    pub peer_id_hash: String,
    pub state: WeixinPairRequestState,
    pub expires_at_millis: u64,
    pub created_at_millis: u64,
    pub updated_at_millis: u64,
    pub transitioned_at_millis: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WeixinEncryptedPayload {
    pub algorithm: String,
    pub algorithm_version: u32,
    pub aad_version: u32,
    pub nonce: String,
    pub ciphertext: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WeixinPendingInbound {
    pub schema_version: u32,
    pub item_id: String,
    pub account_id: String,
    pub message_id_hash: String,
    pub peer_id_hash: String,
    pub encrypted_payload_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted_payload: Option<WeixinEncryptedPayload>,
    pub state: WeixinPendingInboundState,
    pub terminal_reason: Option<String>,
    pub created_at_millis: u64,
    pub updated_at_millis: u64,
    pub transitioned_at_millis: u64,
}

impl WeixinPendingInbound {
    pub fn transition(
        &mut self,
        target: WeixinPendingInboundState,
        now_millis: u64,
    ) -> Result<(), WeixinStateError> {
        if !self.state.can_transition_to(target) {
            return Err(WeixinStateError::InvalidTransition {
                from: self.state.as_str(),
                to: target.as_str(),
            });
        }
        self.state = target;
        self.updated_at_millis = now_millis;
        self.transitioned_at_millis = now_millis;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WeixinConnectionStateRecord {
    NotConfigured,
    Ready,
    Suspended,
    LoggedOut,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WeixinReceiptState {
    Accepted,
    Ready,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Expired,
    Unknown,
}

impl WeixinReceiptState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Cancelled | Self::Expired | Self::Unknown
        )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WeixinDeliveryState {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Expired,
    Unknown,
}

impl WeixinDeliveryState {
    fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Cancelled | Self::Expired | Self::Unknown
        )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WeixinPairRequestState {
    Pending,
    Approved,
    Denied,
    Expired,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WeixinPendingInboundState {
    Accepted,
    Ready,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Expired,
    Unknown,
}

impl WeixinPendingInboundState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Ready => "ready",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Expired => "expired",
            Self::Unknown => "unknown",
        }
    }

    fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Cancelled | Self::Expired | Self::Unknown
        )
    }

    fn can_transition_to(self, target: Self) -> bool {
        matches!(
            (self, target),
            (Self::Accepted, Self::Ready)
                | (Self::Ready, Self::Running)
                | (Self::Running, Self::Succeeded)
                | (Self::Running, Self::Failed)
                | (Self::Running, Self::Cancelled)
                | (Self::Running, Self::Expired)
                | (Self::Running, Self::Unknown)
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct WeixinAccountLockInfo {
    pub state: WeixinAccountLockState,
    pub pid: Option<u32>,
    pub workspace_id: Option<String>,
    pub created_at_millis: Option<u64>,
}

impl WeixinAccountLockInfo {
    fn free() -> Self {
        Self {
            state: WeixinAccountLockState::Free,
            pid: None,
            workspace_id: None,
            created_at_millis: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WeixinAccountLockState {
    Free,
    Active,
    Stale,
}

impl WeixinAccountLockState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Active => "active",
            Self::Stale => "stale",
        }
    }
}

pub struct WeixinAccountLock {
    path: PathBuf,
    lock_id: String,
    released: bool,
}

impl WeixinAccountLock {
    pub fn release(&mut self) -> Result<(), WeixinStateError> {
        if self.released {
            return Ok(());
        }
        if self.path.is_file()
            && let Ok(record) = read_lock_record(&self.path)
            && record.lock_id == self.lock_id
        {
            fs::remove_file(&self.path).map_err(|error| WeixinStateError::Io {
                operation: "release_lock",
                path: redacted_path(&self.path),
                source: error,
            })?;
        }
        self.released = true;
        Ok(())
    }
}

impl Drop for WeixinAccountLock {
    fn drop(&mut self) {
        let _ = self.release();
    }
}

#[derive(Debug, Error)]
pub enum WeixinStateError {
    #[error("weixin state I/O failed operation={operation} path={path}")]
    Io {
        operation: &'static str,
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("weixin state JSON is invalid path={path}")]
    InvalidJson { path: String },
    #[error("weixin state schema is newer than supported found={found} supported={supported}")]
    FutureSchema { found: u32, supported: u32 },
    #[error("weixin state record failed validation reason={reason}")]
    InvalidRecord { reason: &'static str },
    #[error("weixin state write injected failure operation={operation}")]
    InjectedFailure { operation: &'static str },
    #[error("weixin state does not exist for account={account_id}")]
    StateNotFound { account_id: String },
    #[error("weixin account lock is active account={account_id}")]
    LockActive { account_id: String },
    #[error("weixin account lock already exists")]
    LockAlreadyExists,
    #[error("weixin pair request was not found request={request_id}")]
    PairRequestNotFound { request_id: String },
    #[error("weixin pair request belongs to a different account")]
    PairRequestAccountMismatch,
    #[error("weixin pair request already reached a terminal state request={request_id}")]
    PairRequestConsumed { request_id: String },
    #[error("weixin pair request expired request={request_id}")]
    PairRequestExpired { request_id: String },
    #[error("weixin pending inbound transition is invalid from={from} to={to}")]
    InvalidTransition {
        from: &'static str,
        to: &'static str,
    },
    #[error("weixin pending inbound queue is full limit={limit}")]
    PendingInboundQueueFull { limit: usize },
    #[error("weixin pending inbound was not found item={item_id}")]
    PendingInboundNotFound { item_id: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeixinStateMigration {
    pub from_schema: u32,
    pub to_schema: u32,
}

impl WeixinStateMigration {
    fn migrate_value(mut value: Value, now_millis: u64) -> Result<Value, WeixinStateError> {
        let schema_version = value
            .get("schema_version")
            .and_then(|value| value.as_u64())
            .map(|value| value as u32);
        match schema_version {
            Some(WEIXIN_STATE_SCHEMA_VERSION) => Ok(value),
            Some(found) if found > WEIXIN_STATE_SCHEMA_VERSION => {
                Err(WeixinStateError::FutureSchema {
                    found,
                    supported: WEIXIN_STATE_SCHEMA_VERSION,
                })
            }
            Some(_) | None => {
                let object = value
                    .as_object_mut()
                    .ok_or(WeixinStateError::InvalidRecord {
                        reason: "state root must be an object",
                    })?;
                if legacy_pending_inbound_lacks_encrypted_payload(object) {
                    return Err(WeixinStateError::InvalidRecord {
                        reason: "legacy pending inbound lacks encrypted payload",
                    });
                }
                object.insert(
                    "schema_version".to_string(),
                    json!(WEIXIN_STATE_SCHEMA_VERSION),
                );
                object
                    .entry("created_at_millis".to_string())
                    .or_insert_with(|| json!(now_millis));
                object
                    .entry("updated_at_millis".to_string())
                    .or_insert_with(|| json!(now_millis));
                object
                    .entry("transitioned_at_millis".to_string())
                    .or_insert_with(|| json!(now_millis));
                object
                    .entry("connection_state".to_string())
                    .or_insert_with(|| json!("not_configured"));
                object
                    .entry("credential".to_string())
                    .or_insert(Value::Null);
                for key in [
                    "cursors",
                    "inbound_receipts",
                    "deliveries",
                    "conversation_bindings",
                    "reply_contexts",
                    "pending_deliveries",
                    "pair_requests",
                    "pending_inbound",
                ] {
                    object.entry(key.to_string()).or_insert_with(|| json!([]));
                }
                object.remove("session_bindings");
                for key in [
                    "cursors",
                    "inbound_receipts",
                    "deliveries",
                    "conversation_bindings",
                    "reply_contexts",
                    "pending_deliveries",
                    "pair_requests",
                    "pending_inbound",
                ] {
                    rewrite_child_schema_versions(object, key);
                }
                object
                    .entry("last_redacted_error".to_string())
                    .or_insert(Value::Null);
                Ok(value)
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct WeixinLockRecord {
    schema_version: u32,
    lock_id: String,
    account_id: String,
    workspace_id: String,
    pid: u32,
    created_at_millis: u64,
    updated_at_millis: u64,
}

impl WeixinLockRecord {
    fn new(account_id: &str, workspace_id: &str, now_millis: u64) -> Self {
        let pid = std::process::id();
        Self {
            schema_version: WEIXIN_STATE_SCHEMA_VERSION,
            lock_id: opaque_request_id(account_id, workspace_id, now_millis),
            account_id: account_id.to_string(),
            workspace_id: workspace_id.to_string(),
            pid,
            created_at_millis: now_millis,
            updated_at_millis: now_millis,
        }
    }
}

fn validate_snapshot(snapshot: &WeixinStateSnapshot) -> Result<(), WeixinStateError> {
    if snapshot.schema_version != WEIXIN_STATE_SCHEMA_VERSION {
        return Err(WeixinStateError::FutureSchema {
            found: snapshot.schema_version,
            supported: WEIXIN_STATE_SCHEMA_VERSION,
        });
    }
    if !looks_redacted("account", &snapshot.account_id) {
        return Err(WeixinStateError::InvalidRecord {
            reason: "account id must be redacted",
        });
    }
    if !looks_redacted("workspace", &snapshot.workspace_id) {
        return Err(WeixinStateError::InvalidRecord {
            reason: "workspace id must be redacted",
        });
    }
    if snapshot.endpoint.trim().is_empty() {
        return Err(WeixinStateError::InvalidRecord {
            reason: "endpoint is empty",
        });
    }
    for request in &snapshot.pair_requests {
        if request.schema_version != WEIXIN_STATE_SCHEMA_VERSION
            || request.account_id != snapshot.account_id
            || !request.peer_id_hash.starts_with("peer#")
            || !request.request_id.starts_with("pair-")
        {
            return Err(WeixinStateError::InvalidRecord {
                reason: "pair request failed validation",
            });
        }
    }
    for receipt in &snapshot.inbound_receipts {
        if receipt.schema_version != WEIXIN_STATE_SCHEMA_VERSION
            || !receipt.message_id_hash.starts_with("message#")
            || !receipt.peer_id_hash.starts_with("peer#")
        {
            return Err(WeixinStateError::InvalidRecord {
                reason: "inbound receipt failed validation",
            });
        }
    }
    for binding in &snapshot.conversation_bindings {
        if binding.schema_version != WEIXIN_STATE_SCHEMA_VERSION
            || binding.account_id != snapshot.account_id
            || !binding.peer_id_hash.starts_with("peer#")
            || !binding.direct_message_key.starts_with("dm#")
            || binding.workspace_id != snapshot.workspace_id
            || binding.session_id.trim().is_empty()
            || contains_sensitive_marker(&binding.session_id)
            || binding.source_label.trim().is_empty()
            || contains_sensitive_marker(&binding.source_label)
        {
            return Err(WeixinStateError::InvalidRecord {
                reason: "conversation binding failed validation",
            });
        }
    }
    for item in &snapshot.pending_inbound {
        if item.schema_version != WEIXIN_STATE_SCHEMA_VERSION
            || item.account_id != snapshot.account_id
            || !item.item_id.starts_with("item#")
            || !item.message_id_hash.starts_with("message#")
            || !item.peer_id_hash.starts_with("peer#")
            || item.encrypted_payload_ref.trim().is_empty()
            || contains_sensitive_marker(&item.encrypted_payload_ref)
            || item
                .payload_kind
                .as_deref()
                .is_some_and(contains_sensitive_marker)
        {
            return Err(WeixinStateError::InvalidRecord {
                reason: "pending inbound failed validation",
            });
        }
        let Some(payload) = &item.encrypted_payload else {
            return Err(WeixinStateError::InvalidRecord {
                reason: "pending inbound encrypted payload is missing",
            });
        };
        validate_encrypted_payload(payload)?;
    }
    Ok(())
}

fn validate_encrypted_payload(payload: &WeixinEncryptedPayload) -> Result<(), WeixinStateError> {
    if payload.algorithm != WEIXIN_PAYLOAD_ALGORITHM {
        return Err(WeixinStateError::InvalidRecord {
            reason: "pending inbound encrypted payload algorithm is unsupported",
        });
    }
    if payload.algorithm_version != WEIXIN_PAYLOAD_ALGORITHM_VERSION {
        return Err(WeixinStateError::InvalidRecord {
            reason: "pending inbound encrypted payload algorithm version is unsupported",
        });
    }
    if payload.aad_version != WEIXIN_PAYLOAD_AAD_VERSION {
        return Err(WeixinStateError::InvalidRecord {
            reason: "pending inbound encrypted payload aad version is unsupported",
        });
    }
    if contains_sensitive_marker(&payload.nonce) || contains_sensitive_marker(&payload.ciphertext) {
        return Err(WeixinStateError::InvalidRecord {
            reason: "pending inbound encrypted payload contains sensitive markers",
        });
    }
    let nonce =
        STANDARD
            .decode(payload.nonce.as_bytes())
            .map_err(|_| WeixinStateError::InvalidRecord {
                reason: "pending inbound encrypted payload nonce is invalid base64",
            })?;
    if nonce.len() != WEIXIN_PAYLOAD_NONCE_LENGTH {
        return Err(WeixinStateError::InvalidRecord {
            reason: "pending inbound encrypted payload nonce length is invalid",
        });
    }
    let ciphertext = STANDARD
        .decode(payload.ciphertext.as_bytes())
        .map_err(|_| WeixinStateError::InvalidRecord {
            reason: "pending inbound encrypted payload ciphertext is invalid base64",
        })?;
    if ciphertext.is_empty() {
        return Err(WeixinStateError::InvalidRecord {
            reason: "pending inbound encrypted payload ciphertext is empty",
        });
    }
    Ok(())
}

fn legacy_pending_inbound_lacks_encrypted_payload(object: &serde_json::Map<String, Value>) -> bool {
    object
        .get("pending_inbound")
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items.iter().any(|item| {
                item.as_object()
                    .is_none_or(|item| !matches!(item.get("encrypted_payload"), Some(value) if !value.is_null()))
            })
        })
}

fn rewrite_child_schema_versions(object: &mut serde_json::Map<String, Value>, key: &str) {
    let Some(items) = object.get_mut(key).and_then(Value::as_array_mut) else {
        return;
    };
    for item in items {
        if let Some(item) = item.as_object_mut() {
            item.insert(
                "schema_version".to_string(),
                json!(WEIXIN_STATE_SCHEMA_VERSION),
            );
        }
    }
}

fn contains_sensitive_marker(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    ["token", "context", "raw", "data_key", "data-key", "secret"]
        .iter()
        .any(|marker| lower.contains(marker))
}

fn looks_redacted(prefix: &str, value: &str) -> bool {
    value
        .strip_prefix(&format!("{prefix}#"))
        .map(|suffix| {
            suffix.len() == 8
                && suffix
                    .chars()
                    .all(|character| character.is_ascii_hexdigit())
        })
        .unwrap_or(false)
}

fn has_receipt(snapshot: &WeixinStateSnapshot, message_id_hash: &str, peer_id_hash: &str) -> bool {
    snapshot.inbound_receipts.iter().any(|receipt| {
        receipt.message_id_hash == message_id_hash && receipt.peer_id_hash == peer_id_hash
    })
}

fn has_pending_inbound(
    snapshot: &WeixinStateSnapshot,
    message_id_hash: &str,
    peer_id_hash: &str,
) -> bool {
    snapshot.pending_inbound.iter().any(|item| {
        item.message_id_hash == message_id_hash
            && item.peer_id_hash == peer_id_hash
            && !item.state.is_terminal()
    })
}

fn has_active_pair_request(
    snapshot: &WeixinStateSnapshot,
    peer_id_hash: &str,
    now_millis: u64,
) -> bool {
    snapshot.pair_requests.iter().any(|request| {
        request.peer_id_hash == peer_id_hash
            && matches!(
                request.state,
                WeixinPairRequestState::Pending | WeixinPairRequestState::Approved
            )
            && request.expires_at_millis > now_millis
    })
}

fn upsert_cursor(
    snapshot: &mut WeixinStateSnapshot,
    account_id: &str,
    source: &str,
    get_updates_buf: String,
    now_millis: u64,
) {
    if let Some(cursor) = snapshot
        .cursors
        .iter_mut()
        .find(|cursor| cursor.source == source)
    {
        cursor.get_updates_buf = Some(get_updates_buf);
        cursor.updated_at_millis = now_millis;
        cursor.transitioned_at_millis = now_millis;
        return;
    }
    snapshot.cursors.push(WeixinCursorRecord {
        schema_version: WEIXIN_STATE_SCHEMA_VERSION,
        account_id: account_id.to_string(),
        get_updates_buf: Some(get_updates_buf),
        source: source.to_string(),
        created_at_millis: now_millis,
        updated_at_millis: now_millis,
        transitioned_at_millis: now_millis,
    });
}

fn upsert_conversation_binding(
    snapshot: &mut WeixinStateSnapshot,
    account_id: &str,
    peer_id_hash: &str,
    direct_message_key: &str,
    workspace_id: &str,
    candidate_session_id: &str,
    source_label: &str,
    now_millis: u64,
) -> Result<WeixinConversationBinding, WeixinStateError> {
    if !looks_redacted("account", account_id)
        || snapshot.account_id != account_id
        || !peer_id_hash.starts_with("peer#")
        || !direct_message_key.starts_with("dm#")
        || snapshot.workspace_id != workspace_id
        || !looks_redacted("workspace", workspace_id)
        || candidate_session_id.trim().is_empty()
        || contains_sensitive_marker(candidate_session_id)
        || source_label.trim().is_empty()
        || contains_sensitive_marker(source_label)
    {
        return Err(WeixinStateError::InvalidRecord {
            reason: "conversation binding failed validation",
        });
    }

    if let Some(binding) = snapshot.conversation_bindings.iter_mut().find(|binding| {
        binding.account_id == account_id
            && binding.peer_id_hash == peer_id_hash
            && binding.direct_message_key == direct_message_key
    }) {
        binding.workspace_id = workspace_id.to_string();
        binding.source_label = source_label.to_string();
        binding.last_activity_millis = now_millis;
        binding.updated_at_millis = now_millis;
        binding.transitioned_at_millis = now_millis;
        return Ok(binding.clone());
    }

    let binding = WeixinConversationBinding {
        schema_version: WEIXIN_STATE_SCHEMA_VERSION,
        account_id: account_id.to_string(),
        peer_id_hash: peer_id_hash.to_string(),
        direct_message_key: direct_message_key.to_string(),
        workspace_id: workspace_id.to_string(),
        session_id: candidate_session_id.to_string(),
        source_label: source_label.to_string(),
        created_at_millis: now_millis,
        last_activity_millis: now_millis,
        updated_at_millis: now_millis,
        transitioned_at_millis: now_millis,
    };
    snapshot.conversation_bindings.push(binding.clone());
    Ok(binding)
}

fn prune_terminal_receipts(snapshot: &mut WeixinStateSnapshot, now_millis: u64) {
    snapshot.inbound_receipts.retain(|receipt| {
        !receipt.state.is_terminal()
            || now_millis.saturating_sub(receipt.transitioned_at_millis)
                <= WEIXIN_TERMINAL_RECEIPT_TTL_MILLIS
    });
    let terminal_count = snapshot
        .inbound_receipts
        .iter()
        .filter(|receipt| receipt.state.is_terminal())
        .count();
    if terminal_count <= WEIXIN_TERMINAL_RECEIPT_LIMIT {
        return;
    }
    let mut to_remove = terminal_count - WEIXIN_TERMINAL_RECEIPT_LIMIT;
    snapshot.inbound_receipts.retain(|receipt| {
        if to_remove > 0 && receipt.state.is_terminal() {
            to_remove -= 1;
            false
        } else {
            true
        }
    });
}

fn write_lock_file(path: &Path, record: &WeixinLockRecord) -> Result<(), WeixinStateError> {
    let bytes = serde_json::to_vec_pretty(record).map_err(|_| WeixinStateError::InvalidRecord {
        reason: "lock serialization failed",
    })?;
    match OpenOptions::new().create_new(true).write(true).open(path) {
        Ok(mut file) => file
            .write_all(&bytes)
            .and_then(|_| file.write_all(b"\n"))
            .and_then(|_| file.flush())
            .and_then(|_| file.sync_all())
            .map_err(|error| WeixinStateError::Io {
                operation: "write_lock",
                path: redacted_path(path),
                source: error,
            }),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            Err(WeixinStateError::LockAlreadyExists)
        }
        Err(error) => Err(WeixinStateError::Io {
            operation: "create_lock",
            path: redacted_path(path),
            source: error,
        }),
    }
}

fn read_lock_record(path: &Path) -> Result<WeixinLockRecord, WeixinStateError> {
    let bytes = fs::read(path).map_err(|error| WeixinStateError::Io {
        operation: "read_lock",
        path: redacted_path(path),
        source: error,
    })?;
    serde_json::from_slice(&bytes).map_err(|_| WeixinStateError::InvalidJson {
        path: redacted_path(path),
    })
}

fn replace_file(temp_path: &Path, path: &Path) -> Result<(), WeixinStateError> {
    #[cfg(windows)]
    {
        windows_replace_file(temp_path, path)
    }
    #[cfg(not(windows))]
    {
        fs::rename(temp_path, path).map_err(|error| WeixinStateError::Io {
            operation: "replace",
            path: redacted_path(path),
            source: error,
        })
    }
}

#[cfg(windows)]
fn windows_replace_file(temp_path: &Path, path: &Path) -> Result<(), WeixinStateError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };
    let mut src: Vec<u16> = temp_path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut dst: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let ok = unsafe {
        MoveFileExW(
            src.as_mut_ptr(),
            dst.as_mut_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if ok == 0 {
        Err(WeixinStateError::Io {
            operation: "replace",
            path: redacted_path(path),
            source: std::io::Error::last_os_error(),
        })
    } else {
        Ok(())
    }
}

fn sync_parent_best_effort(parent: &Path) {
    if let Ok(file) = File::open(parent) {
        let _ = file.sync_all();
    }
}

fn default_lock_root() -> Option<PathBuf> {
    std::env::var(WEIXIN_LOCK_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("LOCALAPPDATA")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .map(|value| {
                    PathBuf::from(value)
                        .join("YunXi Agent")
                        .join("weixin-account-locks")
                })
        })
}

#[cfg(windows)]
fn default_process_is_running(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_ACCESS_DENIED, ERROR_INVALID_PARAMETER, GetLastError, STILL_ACTIVE,
    };
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    if pid == std::process::id() {
        return true;
    }
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        let error = unsafe { GetLastError() };
        return match error {
            ERROR_INVALID_PARAMETER => false,
            ERROR_ACCESS_DENIED => true,
            _ => true,
        };
    }
    let mut exit_code = 0;
    let ok = unsafe { GetExitCodeProcess(handle, &mut exit_code) };
    unsafe {
        CloseHandle(handle);
    }
    if ok == 0 {
        return true;
    }
    exit_code == STILL_ACTIVE as u32
}

#[cfg(not(windows))]
fn default_process_is_running(pid: u32) -> bool {
    pid == std::process::id() || pid != 0
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn safe_file_component(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn opaque_request_id(account_id: &str, peer_id_hash: &str, now_millis: u64) -> String {
    let mut hasher = DefaultHasher::new();
    account_id.hash(&mut hasher);
    peer_id_hash.hash(&mut hasher);
    now_millis.hash(&mut hasher);
    std::process::id().hash(&mut hasher);
    format!("pair-{:016x}", hasher.finish())
}

fn redacted_path(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(|value| format!("<weixin-state>/{value}"))
        .unwrap_or_else(|| "<weixin-state>".to_string())
}

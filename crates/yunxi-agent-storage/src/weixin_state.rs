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
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

pub const WEIXIN_STATE_SCHEMA_VERSION: u32 = 6;
pub const WEIXIN_PAYLOAD_ALGORITHM: &str = "chacha20-poly1305";
pub const WEIXIN_PAYLOAD_ALGORITHM_VERSION: u32 = 1;
pub const WEIXIN_PAYLOAD_AAD_VERSION: u32 = 1;
pub const WEIXIN_PAYLOAD_NONCE_LENGTH: usize = 12;
const WEIXIN_STATE_DIRECTORY: &str = ".yunxi/weixin/state";
const WEIXIN_LOCK_ENV: &str = "YUNXI_WEIXIN_LOCK_ROOT";
const WEIXIN_PENDING_INBOUND_LIMIT: usize = 1024;
const WEIXIN_TERMINAL_RECEIPT_LIMIT: usize = 2048;
const WEIXIN_TERMINAL_RECEIPT_TTL_MILLIS: u64 = 7 * 24 * 60 * 60 * 1000;
const WEIXIN_LATENCY_TRACE_LIMIT: usize = 128;
static STATE_TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

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
            ".{file_name}.tmp.{}.{}.{}",
            std::process::id(),
            now_millis(),
            STATE_TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed)
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
            let item_id = item.item_id;
            let message_id_hash = item.message_id_hash;
            let peer_id_hash = item.peer_id_hash;
            let direct_message_key = item.direct_message_key;
            snapshot.inbound_receipts.push(WeixinInboundReceiptRecord {
                schema_version: WEIXIN_STATE_SCHEMA_VERSION,
                message_id_hash: message_id_hash.clone(),
                peer_id_hash: peer_id_hash.clone(),
                accepted_at_millis: commit.now_millis,
                state: WeixinReceiptState::Ready,
                created_at_millis: commit.now_millis,
                updated_at_millis: commit.now_millis,
                transitioned_at_millis: commit.now_millis,
            });
            snapshot.pending_inbound.push(WeixinPendingInbound {
                schema_version: WEIXIN_STATE_SCHEMA_VERSION,
                item_id: item_id.clone(),
                account_id: commit.account_id.clone(),
                message_id_hash: message_id_hash.clone(),
                peer_id_hash: peer_id_hash.clone(),
                direct_message_key: direct_message_key.clone(),
                encrypted_payload_ref: item.encrypted_payload_ref,
                payload_kind: item.payload_kind,
                encrypted_payload: Some(item.encrypted_payload),
                turn_session_id: None,
                parent_session_id: None,
                dispatch_retry_count: 0,
                next_retry_at_millis: None,
                last_dispatch_error: None,
                lease_owner: None,
                lease_token: None,
                lease_started_at_millis: None,
                lease_expires_at_millis: None,
                state: WeixinPendingInboundState::Ready,
                terminal_reason: None,
                created_at_millis: commit.now_millis,
                updated_at_millis: commit.now_millis,
                transitioned_at_millis: commit.now_millis,
            });
            snapshot.latency_traces.push(WeixinLatencyTraceRecord {
                schema_version: WEIXIN_STATE_SCHEMA_VERSION,
                item_id: item_id.clone(),
                message_id_hash,
                peer_id_hash,
                direct_message_key,
                session_id: None,
                poll_started_at_millis: commit.poll_started_at_millis,
                poll_completed_at_millis: commit.poll_completed_at_millis,
                inbound_committed_at_millis: Some(commit.now_millis),
                dispatch_claimed_at_millis: None,
                runtime_started_at_millis: None,
                runtime_completed_at_millis: None,
                spool_written_at_millis: None,
                turn_completed_at_millis: None,
                delivery_started_at_millis: None,
                delivery_completed_at_millis: None,
                delivery_segment_count: 0,
                delivery_attempt_count: 0,
                delivery_success_count: 0,
                terminal_status: None,
                terminal_reason: None,
                delivery_status: None,
                error_label: None,
                created_at_millis: commit.now_millis,
                updated_at_millis: commit.now_millis,
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
        prune_latency_traces(&mut snapshot);
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

    pub fn load_ready_pending_inbound(
        &self,
        account_id: &str,
        now_millis: u64,
        limit: usize,
    ) -> Result<Vec<WeixinPendingInbound>, WeixinStateError> {
        let snapshot = self.load_required(account_id)?;
        Ok(snapshot
            .pending_inbound
            .into_iter()
            .filter(|item| {
                item.state == WeixinPendingInboundState::Ready
                    && item
                        .next_retry_at_millis
                        .is_none_or(|next_retry| next_retry <= now_millis)
            })
            .take(limit)
            .collect())
    }

    pub fn remember_pending_runtime_turn_session(
        &self,
        account_id: &str,
        item_id: &str,
        turn_session_id: &str,
        now_millis: u64,
    ) -> Result<WeixinPendingInbound, WeixinStateError> {
        if turn_session_id.trim().is_empty() || contains_sensitive_marker(turn_session_id) {
            return Err(WeixinStateError::InvalidRecord {
                reason: "pending inbound turn session id failed validation",
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
        if pending.state != WeixinPendingInboundState::Ready {
            return Err(WeixinStateError::InvalidTransition {
                from: pending.state.as_str(),
                to: WeixinPendingInboundState::Ready.as_str(),
            });
        }
        if let Some(existing) = pending.turn_session_id.as_deref() {
            if existing != turn_session_id {
                return Err(WeixinStateError::InvalidRecord {
                    reason: "pending inbound turn session id mismatch",
                });
            }
        } else {
            pending.turn_session_id = Some(turn_session_id.to_string());
        }
        pending.updated_at_millis = now_millis;
        let updated = pending.clone();
        snapshot.updated_at_millis = now_millis;
        self.save(&snapshot)?;
        Ok(updated)
    }

    pub fn record_pending_runtime_dispatch_deferred(
        &self,
        account_id: &str,
        item_id: &str,
        error_label: &str,
        next_retry_at_millis: u64,
        now_millis: u64,
    ) -> Result<WeixinPendingInbound, WeixinStateError> {
        if error_label.trim().is_empty() || contains_sensitive_marker(error_label) {
            return Err(WeixinStateError::InvalidRecord {
                reason: "pending inbound dispatch error failed validation",
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
        if pending.state != WeixinPendingInboundState::Ready {
            return Err(WeixinStateError::InvalidTransition {
                from: pending.state.as_str(),
                to: WeixinPendingInboundState::Ready.as_str(),
            });
        }
        pending.dispatch_retry_count = pending.dispatch_retry_count.saturating_add(1);
        pending.next_retry_at_millis = Some(next_retry_at_millis);
        pending.last_dispatch_error = Some(error_label.to_string());
        pending.lease_owner = None;
        pending.lease_token = None;
        pending.lease_started_at_millis = None;
        pending.lease_expires_at_millis = None;
        pending.updated_at_millis = now_millis;
        pending.transitioned_at_millis = now_millis;
        let updated = pending.clone();
        if let Some(receipt) = snapshot.inbound_receipts.iter_mut().find(|receipt| {
            receipt.message_id_hash == updated.message_id_hash
                && receipt.peer_id_hash == updated.peer_id_hash
        }) {
            receipt.state = WeixinReceiptState::Ready;
            receipt.updated_at_millis = now_millis;
        }
        update_latency_trace_for_item(&mut snapshot, item_id, now_millis, |trace| {
            trace.error_label = Some(error_label.to_string());
        });
        snapshot.updated_at_millis = now_millis;
        snapshot.transitioned_at_millis = now_millis;
        self.save(&snapshot)?;
        Ok(updated)
    }

    pub fn record_pending_runtime_dispatch_inflight(
        &self,
        account_id: &str,
        item_id: &str,
        next_retry_at_millis: u64,
        now_millis: u64,
    ) -> Result<WeixinPendingInbound, WeixinStateError> {
        self.claim_pending_runtime_dispatch(
            account_id,
            item_id,
            "lease#legacy",
            "attempt#legacy",
            next_retry_at_millis,
            now_millis,
        )
    }

    pub fn claim_pending_runtime_dispatch(
        &self,
        account_id: &str,
        item_id: &str,
        lease_owner: &str,
        lease_token: &str,
        lease_expires_at_millis: u64,
        now_millis: u64,
    ) -> Result<WeixinPendingInbound, WeixinStateError> {
        if !lease_owner.starts_with("lease#")
            || contains_sensitive_marker(lease_owner)
            || !lease_token.starts_with("attempt#")
            || contains_sensitive_marker(lease_token)
            || lease_expires_at_millis <= now_millis
        {
            return Err(WeixinStateError::InvalidRecord {
                reason: "runtime lease failed validation",
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
        if pending.state != WeixinPendingInboundState::Ready {
            return Err(WeixinStateError::InvalidTransition {
                from: pending.state.as_str(),
                to: WeixinPendingInboundState::Ready.as_str(),
            });
        }
        pending.next_retry_at_millis = Some(lease_expires_at_millis);
        pending.last_dispatch_error = Some("runtime_dispatch_inflight".to_string());
        pending.lease_owner = Some(lease_owner.to_string());
        pending.lease_token = Some(lease_token.to_string());
        pending.lease_started_at_millis = Some(now_millis);
        pending.lease_expires_at_millis = Some(lease_expires_at_millis);
        pending.updated_at_millis = now_millis;
        let updated = pending.clone();
        update_latency_trace_for_item(&mut snapshot, item_id, now_millis, |trace| {
            trace.dispatch_claimed_at_millis = Some(now_millis);
        });
        snapshot.updated_at_millis = now_millis;
        self.save(&snapshot)?;
        Ok(updated)
    }

    pub fn recover_stale_pending_runtime_turns(
        &self,
        account_id: &str,
        active_lease_owner: &str,
        now_millis: u64,
    ) -> Result<usize, WeixinStateError> {
        let mut snapshot = self.load_required(account_id)?;
        let mut recovered = 0usize;
        for pending in &mut snapshot.pending_inbound {
            let stale = pending.state == WeixinPendingInboundState::Running
                && pending
                    .lease_expires_at_millis
                    .is_some_and(|expires_at| expires_at <= now_millis)
                && pending
                    .lease_owner
                    .as_deref()
                    .is_some_and(|owner| owner != active_lease_owner);
            if !stale {
                continue;
            }
            pending.state = WeixinPendingInboundState::Ready;
            pending.dispatch_retry_count = pending.dispatch_retry_count.saturating_add(1);
            pending.next_retry_at_millis = Some(now_millis);
            pending.last_dispatch_error = Some("runtime_lease_expired".to_string());
            pending.lease_owner = None;
            pending.lease_token = None;
            pending.lease_started_at_millis = None;
            pending.lease_expires_at_millis = None;
            pending.updated_at_millis = now_millis;
            pending.transitioned_at_millis = now_millis;
            recovered += 1;
        }
        if recovered > 0 {
            snapshot.updated_at_millis = now_millis;
            snapshot.transitioned_at_millis = now_millis;
            self.save(&snapshot)?;
        }
        Ok(recovered)
    }

    pub fn recover_pending_runtime_dispatch_for_owner(
        &self,
        account_id: &str,
        item_id: &str,
        lease_owner: &str,
        error_label: &str,
        now_millis: u64,
    ) -> Result<Option<WeixinPendingInbound>, WeixinStateError> {
        if !lease_owner.starts_with("lease#")
            || contains_sensitive_marker(lease_owner)
            || error_label.trim().is_empty()
            || contains_sensitive_marker(error_label)
        {
            return Err(WeixinStateError::InvalidRecord {
                reason: "runtime lease recovery failed validation",
            });
        }
        let mut snapshot = self.load_required(account_id)?;
        let Some(pending) = snapshot
            .pending_inbound
            .iter_mut()
            .find(|item| item.item_id == item_id && !item.state.is_terminal())
        else {
            return Ok(None);
        };
        if pending.account_id != account_id || pending.lease_owner.as_deref() != Some(lease_owner) {
            return Ok(None);
        }
        if !matches!(
            pending.state,
            WeixinPendingInboundState::Ready | WeixinPendingInboundState::Running
        ) {
            return Ok(None);
        }
        let message_id_hash = pending.message_id_hash.clone();
        let peer_id_hash = pending.peer_id_hash.clone();
        pending.state = WeixinPendingInboundState::Ready;
        pending.dispatch_retry_count = pending.dispatch_retry_count.saturating_add(1);
        pending.next_retry_at_millis = Some(now_millis);
        pending.last_dispatch_error = Some(error_label.to_string());
        pending.lease_owner = None;
        pending.lease_token = None;
        pending.lease_started_at_millis = None;
        pending.lease_expires_at_millis = None;
        pending.updated_at_millis = now_millis;
        pending.transitioned_at_millis = now_millis;
        let updated = pending.clone();
        if let Some(receipt) = snapshot.inbound_receipts.iter_mut().find(|receipt| {
            receipt.message_id_hash == message_id_hash && receipt.peer_id_hash == peer_id_hash
        }) {
            receipt.state = WeixinReceiptState::Ready;
            receipt.updated_at_millis = now_millis;
            receipt.transitioned_at_millis = now_millis;
        }
        update_latency_trace_for_item(&mut snapshot, item_id, now_millis, |trace| {
            trace.error_label = Some(error_label.to_string());
        });
        snapshot.updated_at_millis = now_millis;
        snapshot.transitioned_at_millis = now_millis;
        self.save(&snapshot)?;
        Ok(Some(updated))
    }

    pub fn enqueue_pending_delivery(
        &self,
        account_id: &str,
        item: WeixinPendingDeliveryCommitItem,
        now_millis: u64,
    ) -> Result<WeixinPendingDeliveryMetadata, WeixinStateError> {
        validate_delivery_commit_item(account_id, &item)?;
        let mut snapshot = self.load_required(account_id)?;
        if let Some(existing) = snapshot
            .pending_deliveries
            .iter()
            .find(|delivery| delivery.delivery_id == item.delivery_id)
        {
            return Ok(existing.clone());
        }
        if snapshot
            .deliveries
            .iter()
            .any(|delivery| delivery.delivery_id == item.delivery_id)
        {
            return Err(WeixinStateError::InvalidRecord {
                reason: "delivery id already reached terminal state",
            });
        }
        let delivery = WeixinPendingDeliveryMetadata {
            schema_version: WEIXIN_STATE_SCHEMA_VERSION,
            account_id: account_id.to_string(),
            delivery_id: item.delivery_id,
            peer_id_hash: item.peer_id_hash,
            direct_message_key: item.direct_message_key,
            item_id: item.item_id,
            session_id: item.session_id,
            message_hash: item.message_hash,
            segment_index: item.segment_index,
            total_segments: item.total_segments,
            wait_for_turn_completion: item.wait_for_turn_completion,
            encrypted_payload: Some(item.encrypted_payload),
            state: WeixinDeliveryState::Pending,
            retry_count: 0,
            last_status: None,
            last_redacted_error: None,
            next_retry_at_millis: None,
            created_at_millis: now_millis,
            updated_at_millis: now_millis,
            transitioned_at_millis: now_millis,
        };
        snapshot.pending_deliveries.push(delivery.clone());
        snapshot.updated_at_millis = now_millis;
        self.save(&snapshot)?;
        Ok(delivery)
    }

    pub fn enqueue_pending_delivery_batch(
        &self,
        account_id: &str,
        manifest: WeixinDeliveryManifestCommitItem,
        items: Vec<WeixinPendingDeliveryCommitItem>,
        now_millis: u64,
    ) -> Result<Vec<WeixinPendingDeliveryMetadata>, WeixinStateError> {
        if items.is_empty()
            || manifest.total_segments == 0
            || manifest.delivery_ids.len() != items.len()
            || manifest
                .delivery_ids
                .iter()
                .any(|delivery_id| !delivery_id.starts_with("delivery#"))
        {
            return Err(WeixinStateError::InvalidRecord {
                reason: "delivery manifest failed validation",
            });
        }
        let manifest_item_id = manifest.item_id.clone();
        let manifest_session_id = manifest.session_id.clone();
        let manifest_total_segments = manifest.total_segments;
        for item in &items {
            validate_delivery_commit_item(account_id, item)?;
            if item.message_hash != manifest.message_hash
                || item.item_id != manifest.item_id
                || item.session_id != manifest.session_id
                || item.total_segments != manifest.total_segments
                || !manifest.delivery_ids.contains(&item.delivery_id)
            {
                return Err(WeixinStateError::InvalidRecord {
                    reason: "delivery manifest item mismatch",
                });
            }
        }
        let mut snapshot = self.load_required(account_id)?;
        if let Some(existing) = snapshot
            .delivery_manifests
            .iter()
            .find(|existing| existing.message_hash == manifest.message_hash)
        {
            let mut existing_items = Vec::new();
            for delivery_id in &existing.delivery_ids {
                if let Some(item) = snapshot
                    .pending_deliveries
                    .iter()
                    .find(|item| &item.delivery_id == delivery_id)
                {
                    existing_items.push(item.clone());
                }
            }
            return Ok(existing_items);
        }
        let mut pending = Vec::with_capacity(items.len());
        for item in items {
            if snapshot
                .pending_deliveries
                .iter()
                .any(|delivery| delivery.delivery_id == item.delivery_id)
                || snapshot
                    .deliveries
                    .iter()
                    .any(|delivery| delivery.delivery_id == item.delivery_id)
            {
                return Err(WeixinStateError::InvalidRecord {
                    reason: "delivery id already exists",
                });
            }
            let delivery = WeixinPendingDeliveryMetadata {
                schema_version: WEIXIN_STATE_SCHEMA_VERSION,
                account_id: account_id.to_string(),
                delivery_id: item.delivery_id,
                peer_id_hash: item.peer_id_hash,
                direct_message_key: item.direct_message_key,
                item_id: item.item_id,
                session_id: item.session_id,
                message_hash: item.message_hash,
                segment_index: item.segment_index,
                total_segments: item.total_segments,
                wait_for_turn_completion: item.wait_for_turn_completion,
                encrypted_payload: Some(item.encrypted_payload),
                state: WeixinDeliveryState::Pending,
                retry_count: 0,
                last_status: None,
                last_redacted_error: None,
                next_retry_at_millis: None,
                created_at_millis: now_millis,
                updated_at_millis: now_millis,
                transitioned_at_millis: now_millis,
            };
            pending.push(delivery);
        }
        snapshot.delivery_manifests.push(WeixinDeliveryManifest {
            schema_version: WEIXIN_STATE_SCHEMA_VERSION,
            message_hash: manifest.message_hash,
            account_id: account_id.to_string(),
            item_id: manifest.item_id,
            session_id: manifest.session_id,
            total_segments: manifest.total_segments,
            delivery_ids: manifest.delivery_ids,
            state: WeixinDeliveryManifestState::Pending,
            created_at_millis: now_millis,
            updated_at_millis: now_millis,
        });
        snapshot.pending_deliveries.extend(pending.iter().cloned());
        update_latency_trace_for_item(&mut snapshot, &manifest_item_id, now_millis, |trace| {
            trace.spool_written_at_millis = Some(now_millis);
            trace.session_id = Some(manifest_session_id);
            trace.delivery_segment_count = manifest_total_segments;
        });
        snapshot.updated_at_millis = now_millis;
        self.save(&snapshot)?;
        Ok(pending)
    }

    pub fn load_ready_pending_deliveries(
        &self,
        account_id: &str,
        now_millis: u64,
        limit: usize,
    ) -> Result<Vec<WeixinPendingDeliveryMetadata>, WeixinStateError> {
        let snapshot = self.load_required(account_id)?;
        let active_item_ids = snapshot
            .pending_inbound
            .iter()
            .filter(|pending| !pending.state.is_terminal())
            .map(|pending| pending.item_id.as_str())
            .collect::<Vec<_>>();
        Ok(snapshot
            .pending_deliveries
            .into_iter()
            .filter(|delivery| {
                delivery.state == WeixinDeliveryState::Pending
                    && (!delivery.wait_for_turn_completion
                        || !active_item_ids.contains(&delivery.item_id.as_str()))
                    && delivery
                        .next_retry_at_millis
                        .is_none_or(|next_retry| next_retry <= now_millis)
            })
            .take(limit)
            .collect())
    }

    pub fn mark_pending_delivery_running(
        &self,
        account_id: &str,
        delivery_id: &str,
        now_millis: u64,
    ) -> Result<WeixinPendingDeliveryMetadata, WeixinStateError> {
        let mut snapshot = self.load_required(account_id)?;
        let delivery = snapshot
            .pending_deliveries
            .iter_mut()
            .find(|delivery| delivery.delivery_id == delivery_id && !delivery.state.is_terminal())
            .ok_or_else(|| WeixinStateError::DeliveryNotFound {
                delivery_id: delivery_id.to_string(),
            })?;
        if delivery.account_id != account_id || delivery.state != WeixinDeliveryState::Pending {
            return Err(WeixinStateError::InvalidTransition {
                from: delivery.state.as_str(),
                to: WeixinDeliveryState::Running.as_str(),
            });
        }
        delivery.state = WeixinDeliveryState::Running;
        delivery.updated_at_millis = now_millis;
        delivery.transitioned_at_millis = now_millis;
        let updated = delivery.clone();
        update_latency_trace_for_item(&mut snapshot, &updated.item_id, now_millis, |trace| {
            if trace.delivery_started_at_millis.is_none() {
                trace.delivery_started_at_millis = Some(now_millis);
            }
            trace.delivery_attempt_count = trace.delivery_attempt_count.saturating_add(1);
            trace.delivery_status = Some(WeixinDeliveryState::Running.as_str().to_string());
        });
        snapshot.updated_at_millis = now_millis;
        self.save(&snapshot)?;
        Ok(updated)
    }

    pub fn record_pending_delivery_deferred(
        &self,
        account_id: &str,
        delivery_id: &str,
        error_label: &str,
        next_retry_at_millis: u64,
        now_millis: u64,
    ) -> Result<WeixinPendingDeliveryMetadata, WeixinStateError> {
        if error_label.trim().is_empty() || contains_sensitive_marker(error_label) {
            return Err(WeixinStateError::InvalidRecord {
                reason: "delivery error failed validation",
            });
        }
        let mut snapshot = self.load_required(account_id)?;
        let delivery = snapshot
            .pending_deliveries
            .iter_mut()
            .find(|delivery| delivery.delivery_id == delivery_id && !delivery.state.is_terminal())
            .ok_or_else(|| WeixinStateError::DeliveryNotFound {
                delivery_id: delivery_id.to_string(),
            })?;
        delivery.state = WeixinDeliveryState::Pending;
        delivery.retry_count = delivery.retry_count.saturating_add(1);
        delivery.last_status = Some("deferred".to_string());
        delivery.last_redacted_error = Some(error_label.to_string());
        delivery.next_retry_at_millis = Some(next_retry_at_millis);
        delivery.updated_at_millis = now_millis;
        delivery.transitioned_at_millis = now_millis;
        let updated = delivery.clone();
        update_latency_trace_for_item(&mut snapshot, &updated.item_id, now_millis, |trace| {
            trace.delivery_completed_at_millis = Some(now_millis);
            trace.delivery_status = Some("deferred".to_string());
            trace.error_label = Some(error_label.to_string());
        });
        snapshot.updated_at_millis = now_millis;
        self.save(&snapshot)?;
        Ok(updated)
    }

    pub fn complete_pending_delivery(
        &self,
        account_id: &str,
        delivery_id: &str,
        target: WeixinDeliveryState,
        last_status: Option<String>,
        last_redacted_error: Option<String>,
        now_millis: u64,
    ) -> Result<WeixinDeliveryRecord, WeixinStateError> {
        if !target.is_terminal() {
            return Err(WeixinStateError::InvalidRecord {
                reason: "delivery terminal state is required",
            });
        }
        if last_status
            .as_deref()
            .is_some_and(contains_sensitive_marker)
            || last_redacted_error
                .as_deref()
                .is_some_and(contains_sensitive_marker)
        {
            return Err(WeixinStateError::InvalidRecord {
                reason: "delivery status failed validation",
            });
        }
        let mut snapshot = self.load_required(account_id)?;
        let index = snapshot
            .pending_deliveries
            .iter()
            .position(|delivery| {
                delivery.delivery_id == delivery_id
                    && delivery.account_id == account_id
                    && !delivery.state.is_terminal()
            })
            .ok_or_else(|| WeixinStateError::DeliveryNotFound {
                delivery_id: delivery_id.to_string(),
            })?;
        let mut pending = snapshot.pending_deliveries.remove(index);
        pending.state = target;
        pending.last_status = last_status;
        pending.last_redacted_error = last_redacted_error.clone();
        pending.updated_at_millis = now_millis;
        pending.transitioned_at_millis = now_millis;
        let pending_item_id = pending.item_id.clone();
        let record = WeixinDeliveryRecord {
            schema_version: WEIXIN_STATE_SCHEMA_VERSION,
            delivery_id: pending.delivery_id,
            state: target,
            last_redacted_error,
            created_at_millis: pending.created_at_millis,
            updated_at_millis: now_millis,
            transitioned_at_millis: now_millis,
        };
        snapshot.deliveries.push(record.clone());
        refresh_delivery_manifest_states(&mut snapshot);
        update_latency_trace_for_item(&mut snapshot, &pending_item_id, now_millis, |trace| {
            trace.delivery_completed_at_millis = Some(now_millis);
            trace.delivery_status = Some(target.as_str().to_string());
            if target == WeixinDeliveryState::Succeeded {
                trace.delivery_success_count = trace.delivery_success_count.saturating_add(1);
            }
            if target != WeixinDeliveryState::Succeeded {
                trace.error_label = record.last_redacted_error.clone();
            }
        });
        snapshot.updated_at_millis = now_millis;
        snapshot.transitioned_at_millis = now_millis;
        self.save(&snapshot)?;
        Ok(record)
    }

    pub fn record_remote_control_request(
        &self,
        account_id: &str,
        item: WeixinRemoteControlCommitItem,
        now_millis: u64,
    ) -> Result<WeixinRemoteControlRequestRecord, WeixinStateError> {
        validate_remote_control_commit_item(account_id, &item)?;
        let mut snapshot = self.load_required(account_id)?;
        if let Some(existing) = snapshot
            .remote_control_requests
            .iter()
            .find(|request| request.request_id == item.request_id)
        {
            return Ok(existing.clone());
        }
        let record = WeixinRemoteControlRequestRecord {
            schema_version: WEIXIN_STATE_SCHEMA_VERSION,
            request_id: item.request_id,
            account_id: account_id.to_string(),
            peer_id_hash: item.peer_id_hash,
            direct_message_key: item.direct_message_key,
            item_id: item.item_id,
            session_id: item.session_id,
            purpose: item.purpose,
            state: WeixinRemoteControlState::Pending,
            action: item.action,
            reason: item.reason,
            cwd_label: item.cwd_label,
            last_status: None,
            expires_at_millis: item.expires_at_millis,
            created_at_millis: now_millis,
            updated_at_millis: now_millis,
            transitioned_at_millis: now_millis,
        };
        snapshot.remote_control_requests.push(record.clone());
        snapshot.updated_at_millis = now_millis;
        self.save(&snapshot)?;
        Ok(record)
    }

    pub fn transition_remote_control_request(
        &self,
        account_id: &str,
        request_id: &str,
        target: WeixinRemoteControlState,
        last_status: Option<String>,
        now_millis: u64,
    ) -> Result<WeixinRemoteControlRequestRecord, WeixinStateError> {
        if target == WeixinRemoteControlState::Pending {
            return Err(WeixinStateError::InvalidTransition {
                from: WeixinRemoteControlState::Pending.as_str(),
                to: target.as_str(),
            });
        }
        if last_status
            .as_deref()
            .is_some_and(contains_sensitive_marker)
        {
            return Err(WeixinStateError::InvalidRecord {
                reason: "remote control status failed validation",
            });
        }
        let mut snapshot = self.load_required(account_id)?;
        let request = snapshot
            .remote_control_requests
            .iter_mut()
            .find(|request| request.request_id == request_id && request.account_id == account_id)
            .ok_or_else(|| WeixinStateError::RemoteControlRequestNotFound {
                request_id: request_id.to_string(),
            })?;
        if request.state != WeixinRemoteControlState::Pending {
            return Err(WeixinStateError::InvalidTransition {
                from: request.state.as_str(),
                to: target.as_str(),
            });
        }
        request.state = target;
        request.last_status = last_status;
        request.updated_at_millis = now_millis;
        request.transitioned_at_millis = now_millis;
        let updated = request.clone();
        snapshot.updated_at_millis = now_millis;
        snapshot.transitioned_at_millis = now_millis;
        self.save(&snapshot)?;
        Ok(updated)
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
        let binding_parent_session_id = conversation_binding_parent_session_id(
            &snapshot,
            &request.account_id,
            &request.peer_id_hash,
            &request.direct_message_key,
        );
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
        if pending.direct_message_key.is_empty() {
            pending.direct_message_key = request.direct_message_key.clone();
        } else if pending.direct_message_key != request.direct_message_key {
            return Err(WeixinStateError::InvalidRecord {
                reason: "pending inbound direct message key mismatch",
            });
        }
        let turn_session_id = pending
            .turn_session_id
            .clone()
            .unwrap_or_else(|| request.candidate_session_id.clone());
        if turn_session_id != request.candidate_session_id {
            return Err(WeixinStateError::InvalidRecord {
                reason: "pending inbound turn session id mismatch",
            });
        }
        pending.turn_session_id = Some(turn_session_id.clone());
        pending.parent_session_id = binding_parent_session_id.clone();
        pending.next_retry_at_millis = None;
        pending.last_dispatch_error = None;
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
            &turn_session_id,
            binding_parent_session_id.as_deref(),
            &request.source_label,
            request.now_millis,
        )?;
        update_latency_trace_for_item(
            &mut snapshot,
            &request.item_id,
            request.now_millis,
            |trace| {
                trace.runtime_started_at_millis = Some(request.now_millis);
                trace.session_id = Some(turn_session_id.clone());
            },
        );
        snapshot.updated_at_millis = request.now_millis;
        snapshot.transitioned_at_millis = request.now_millis;
        self.save(&snapshot)?;
        Ok(binding)
    }

    pub fn record_pending_runtime_agent_completed(
        &self,
        account_id: &str,
        item_id: &str,
        now_millis: u64,
    ) -> Result<(), WeixinStateError> {
        let mut snapshot = self.load_required(account_id)?;
        update_latency_trace_for_item(&mut snapshot, item_id, now_millis, |trace| {
            trace.runtime_completed_at_millis = Some(now_millis);
        });
        snapshot.updated_at_millis = now_millis;
        self.save(&snapshot)?;
        Ok(())
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
        let terminal_reason_for_trace = terminal_reason.clone();
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
        pending.lease_owner = None;
        pending.lease_token = None;
        pending.lease_started_at_millis = None;
        pending.lease_expires_at_millis = None;
        let updated = pending.clone();
        complete_conversation_binding_for_pending(&mut snapshot, &updated, target, now_millis)?;
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
        update_latency_trace_for_item(&mut snapshot, item_id, now_millis, |trace| {
            if trace.runtime_completed_at_millis.is_none() {
                trace.runtime_completed_at_millis = Some(now_millis);
            }
            trace.turn_completed_at_millis = Some(now_millis);
            trace.terminal_status = Some(target.as_str().to_string());
            trace.terminal_reason = terminal_reason_for_trace.clone();
            if target == WeixinPendingInboundState::Succeeded {
                trace.error_label = None;
            } else if let Some(reason) = terminal_reason_for_trace.clone() {
                trace.error_label = Some(reason);
            }
        });
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
    pub poll_started_at_millis: Option<u64>,
    pub poll_completed_at_millis: Option<u64>,
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
            poll_started_at_millis: None,
            poll_completed_at_millis: None,
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
    pub direct_message_key: String,
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
pub struct WeixinPendingDeliveryCommitItem {
    pub delivery_id: String,
    pub peer_id_hash: String,
    pub direct_message_key: String,
    pub item_id: String,
    pub session_id: String,
    pub message_hash: String,
    pub segment_index: u32,
    pub total_segments: u32,
    pub wait_for_turn_completion: bool,
    pub encrypted_payload: WeixinEncryptedPayload,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeixinDeliveryManifestCommitItem {
    pub message_hash: String,
    pub item_id: String,
    pub session_id: String,
    pub total_segments: u32,
    pub delivery_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeixinRemoteControlCommitItem {
    pub request_id: String,
    pub peer_id_hash: String,
    pub direct_message_key: String,
    pub item_id: String,
    pub session_id: String,
    pub purpose: WeixinRemoteControlPurposeRecord,
    pub action: String,
    pub reason: String,
    pub cwd_label: Option<String>,
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
    #[serde(default)]
    pub delivery_manifests: Vec<WeixinDeliveryManifest>,
    pub pending_deliveries: Vec<WeixinPendingDeliveryMetadata>,
    pub remote_control_requests: Vec<WeixinRemoteControlRequestRecord>,
    pub pair_requests: Vec<WeixinPairRequest>,
    pub pending_inbound: Vec<WeixinPendingInbound>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub latency_traces: Vec<WeixinLatencyTraceRecord>,
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
            delivery_manifests: Vec::new(),
            pending_deliveries: Vec::new(),
            remote_control_requests: Vec::new(),
            pair_requests: Vec::new(),
            pending_inbound: Vec::new(),
            latency_traces: Vec::new(),
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

    pub fn pending_remote_control_count(&self) -> usize {
        self.pending_remote_control_count_at(u64::MAX)
    }

    pub fn pending_remote_control_count_at(&self, now_millis: u64) -> usize {
        self.remote_control_requests
            .iter()
            .filter(|request| {
                request.state == WeixinRemoteControlState::Pending
                    && request.expires_at_millis > now_millis
            })
            .count()
    }

    pub fn pair_request_count(&self) -> usize {
        self.pair_requests
            .iter()
            .filter(|request| request.state == WeixinPairRequestState::Pending)
            .count()
    }

    pub fn recent_latency_traces(&self, limit: usize) -> Vec<WeixinLatencyTraceRecord> {
        let limit = limit.max(1);
        let mut traces = self.latency_traces.clone();
        traces.sort_by_key(|trace| trace.created_at_millis);
        traces.into_iter().rev().take(limit).collect()
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WeixinDeliveryManifestState {
    Pending,
    Succeeded,
    Failed,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WeixinDeliveryManifest {
    pub schema_version: u32,
    pub message_hash: String,
    pub account_id: String,
    pub item_id: String,
    pub session_id: String,
    pub total_segments: u32,
    pub delivery_ids: Vec<String>,
    pub state: WeixinDeliveryManifestState,
    pub created_at_millis: u64,
    pub updated_at_millis: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WeixinConversationBinding {
    pub schema_version: u32,
    pub account_id: String,
    pub peer_id_hash: String,
    pub direct_message_key: String,
    pub workspace_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_completed_session_id: Option<String>,
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
    #[serde(default)]
    pub account_id: String,
    pub delivery_id: String,
    #[serde(default)]
    pub peer_id_hash: String,
    #[serde(default)]
    pub direct_message_key: String,
    #[serde(default)]
    pub item_id: String,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub message_hash: String,
    #[serde(default)]
    pub segment_index: u32,
    #[serde(default)]
    pub total_segments: u32,
    #[serde(default)]
    pub wait_for_turn_completion: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted_payload: Option<WeixinEncryptedPayload>,
    pub state: WeixinDeliveryState,
    pub retry_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_redacted_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_retry_at_millis: Option<u64>,
    pub created_at_millis: u64,
    pub updated_at_millis: u64,
    pub transitioned_at_millis: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WeixinRemoteControlRequestRecord {
    pub schema_version: u32,
    pub request_id: String,
    pub account_id: String,
    pub peer_id_hash: String,
    pub direct_message_key: String,
    pub item_id: String,
    pub session_id: String,
    pub purpose: WeixinRemoteControlPurposeRecord,
    pub state: WeixinRemoteControlState,
    pub action: String,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_status: Option<String>,
    pub expires_at_millis: u64,
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
    #[serde(default)]
    pub direct_message_key: String,
    pub encrypted_payload_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted_payload: Option<WeixinEncryptedPayload>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_session_id: Option<String>,
    #[serde(default)]
    pub dispatch_retry_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_retry_at_millis: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_dispatch_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease_owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease_started_at_millis: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease_expires_at_millis: Option<u64>,
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WeixinLatencyTraceRecord {
    pub schema_version: u32,
    pub item_id: String,
    pub message_id_hash: String,
    pub peer_id_hash: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub direct_message_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poll_started_at_millis: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poll_completed_at_millis: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inbound_committed_at_millis: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dispatch_claimed_at_millis: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_started_at_millis: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_completed_at_millis: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spool_written_at_millis: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_completed_at_millis: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery_started_at_millis: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery_completed_at_millis: Option<u64>,
    #[serde(default)]
    pub delivery_segment_count: u32,
    #[serde(default)]
    pub delivery_attempt_count: u32,
    #[serde(default)]
    pub delivery_success_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_label: Option<String>,
    pub created_at_millis: u64,
    pub updated_at_millis: u64,
}

impl WeixinLatencyTraceRecord {
    pub fn total_latency_millis(&self) -> Option<u64> {
        let start = self
            .poll_started_at_millis
            .or(self.inbound_committed_at_millis)
            .or(self.runtime_started_at_millis)?;
        let end = self
            .delivery_completed_at_millis
            .or(self.turn_completed_at_millis)
            .or(self.spool_written_at_millis)
            .or(self.runtime_completed_at_millis)
            .or(self.inbound_committed_at_millis)?;
        Some(end.saturating_sub(start))
    }

    pub fn poll_latency_millis(&self) -> Option<u64> {
        duration_millis(self.poll_started_at_millis, self.poll_completed_at_millis)
    }

    pub fn queue_latency_millis(&self) -> Option<u64> {
        duration_millis(
            self.inbound_committed_at_millis,
            self.runtime_started_at_millis,
        )
    }

    pub fn runtime_latency_millis(&self) -> Option<u64> {
        duration_millis(
            self.runtime_started_at_millis,
            self.runtime_completed_at_millis,
        )
    }

    pub fn spool_latency_millis(&self) -> Option<u64> {
        duration_millis(
            self.runtime_completed_at_millis,
            self.spool_written_at_millis,
        )
    }

    pub fn delivery_wait_millis(&self) -> Option<u64> {
        duration_millis(
            self.spool_written_at_millis,
            self.delivery_started_at_millis,
        )
    }

    pub fn delivery_latency_millis(&self) -> Option<u64> {
        duration_millis(
            self.delivery_started_at_millis,
            self.delivery_completed_at_millis,
        )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WeixinRemoteControlPurposeRecord {
    Approval,
    UserInput,
    Cancellation,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WeixinRemoteControlState {
    Pending,
    Consumed,
    Expired,
    Rejected,
    Cancelled,
}

impl WeixinRemoteControlState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Consumed => "consumed",
            Self::Expired => "expired",
            Self::Rejected => "rejected",
            Self::Cancelled => "cancelled",
        }
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
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
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
                | (Self::Ready, Self::Failed)
                | (Self::Ready, Self::Cancelled)
                | (Self::Ready, Self::Expired)
                | (Self::Ready, Self::Unknown)
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
    #[error("weixin pending delivery was not found delivery={delivery_id}")]
    DeliveryNotFound { delivery_id: String },
    #[error("weixin remote control request was not found request={request_id}")]
    RemoteControlRequestNotFound { request_id: String },
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
                    "delivery_manifests",
                    "pending_deliveries",
                    "remote_control_requests",
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
                    "delivery_manifests",
                    "pending_deliveries",
                    "remote_control_requests",
                    "pair_requests",
                    "pending_inbound",
                ] {
                    rewrite_child_schema_versions(object, key);
                }
                rewrite_conversation_binding_session_fields(object);
                rewrite_pending_inbound_runtime_fields(object);
                rewrite_pending_delivery_fields(object);
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
    for trace in &snapshot.latency_traces {
        validate_latency_trace(trace)?;
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
    for delivery in &snapshot.deliveries {
        if delivery.schema_version != WEIXIN_STATE_SCHEMA_VERSION
            || !delivery.delivery_id.starts_with("delivery#")
            || delivery
                .last_redacted_error
                .as_deref()
                .is_some_and(contains_sensitive_marker)
        {
            return Err(WeixinStateError::InvalidRecord {
                reason: "delivery record failed validation",
            });
        }
    }
    for manifest in &snapshot.delivery_manifests {
        if manifest.schema_version != WEIXIN_STATE_SCHEMA_VERSION
            || manifest.account_id != snapshot.account_id
            || !manifest.message_hash.starts_with("reply#")
            || !manifest.item_id.starts_with("item#")
            || manifest.session_id.trim().is_empty()
            || contains_sensitive_marker(&manifest.session_id)
            || manifest.total_segments == 0
            || manifest.delivery_ids.len() != manifest.total_segments as usize
            || manifest
                .delivery_ids
                .iter()
                .any(|delivery_id| !delivery_id.starts_with("delivery#"))
        {
            return Err(WeixinStateError::InvalidRecord {
                reason: "delivery manifest failed validation",
            });
        }
    }
    for delivery in &snapshot.pending_deliveries {
        if delivery.schema_version != WEIXIN_STATE_SCHEMA_VERSION
            || delivery.account_id != snapshot.account_id
            || !delivery.delivery_id.starts_with("delivery#")
            || !delivery.peer_id_hash.starts_with("peer#")
            || !delivery.direct_message_key.starts_with("dm#")
            || !delivery.item_id.starts_with("item#")
            || delivery.session_id.trim().is_empty()
            || contains_sensitive_marker(&delivery.session_id)
            || !delivery.message_hash.starts_with("reply#")
            || delivery.total_segments == 0
            || delivery.segment_index >= delivery.total_segments
            || delivery
                .last_status
                .as_deref()
                .is_some_and(contains_sensitive_marker)
            || delivery
                .last_redacted_error
                .as_deref()
                .is_some_and(contains_sensitive_marker)
        {
            return Err(WeixinStateError::InvalidRecord {
                reason: "pending delivery failed validation",
            });
        }
        let Some(payload) = &delivery.encrypted_payload else {
            return Err(WeixinStateError::InvalidRecord {
                reason: "pending delivery encrypted payload is missing",
            });
        };
        validate_encrypted_payload(payload)?;
    }
    for request in &snapshot.remote_control_requests {
        if request.schema_version != WEIXIN_STATE_SCHEMA_VERSION
            || request.account_id != snapshot.account_id
            || !request.request_id.starts_with("wxctl#")
            || !request.peer_id_hash.starts_with("peer#")
            || !request.direct_message_key.starts_with("dm#")
            || !request.item_id.starts_with("item#")
            || request.session_id.trim().is_empty()
            || contains_sensitive_marker(&request.session_id)
            || request.action.trim().is_empty()
            || contains_sensitive_marker(&request.action)
            || contains_sensitive_marker(&request.reason)
            || request
                .cwd_label
                .as_deref()
                .is_some_and(|cwd_label| !cwd_label.starts_with("cwd#"))
            || request
                .last_status
                .as_deref()
                .is_some_and(contains_sensitive_marker)
        {
            return Err(WeixinStateError::InvalidRecord {
                reason: "remote control request failed validation",
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
            || binding
                .root_session_id
                .as_deref()
                .is_some_and(|session_id| {
                    session_id.trim().is_empty() || contains_sensitive_marker(session_id)
                })
            || binding
                .active_session_id
                .as_deref()
                .is_some_and(|session_id| {
                    session_id.trim().is_empty() || contains_sensitive_marker(session_id)
                })
            || binding
                .last_completed_session_id
                .as_deref()
                .is_some_and(|session_id| {
                    session_id.trim().is_empty() || contains_sensitive_marker(session_id)
                })
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
            || (!item.direct_message_key.is_empty() && !item.direct_message_key.starts_with("dm#"))
            || item.encrypted_payload_ref.trim().is_empty()
            || contains_sensitive_marker(&item.encrypted_payload_ref)
            || item
                .payload_kind
                .as_deref()
                .is_some_and(contains_sensitive_marker)
            || item.turn_session_id.as_deref().is_some_and(|session_id| {
                session_id.trim().is_empty() || contains_sensitive_marker(session_id)
            })
            || item.parent_session_id.as_deref().is_some_and(|session_id| {
                session_id.trim().is_empty() || contains_sensitive_marker(session_id)
            })
            || item
                .last_dispatch_error
                .as_deref()
                .is_some_and(contains_sensitive_marker)
            || item.lease_owner.as_deref().is_some_and(|owner| {
                !owner.starts_with("lease#") || contains_sensitive_marker(owner)
            })
            || item.lease_token.as_deref().is_some_and(|token| {
                !token.starts_with("attempt#") || contains_sensitive_marker(token)
            })
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

fn validate_latency_trace(trace: &WeixinLatencyTraceRecord) -> Result<(), WeixinStateError> {
    if trace.schema_version != WEIXIN_STATE_SCHEMA_VERSION
        || !trace.item_id.starts_with("item#")
        || !trace.message_id_hash.starts_with("message#")
        || !trace.peer_id_hash.starts_with("peer#")
        || (!trace.direct_message_key.is_empty() && !trace.direct_message_key.starts_with("dm#"))
        || trace.session_id.as_deref().is_some_and(|session_id| {
            session_id.trim().is_empty() || contains_sensitive_marker(session_id)
        })
        || trace
            .terminal_status
            .as_deref()
            .is_some_and(|value| value.trim().is_empty() || contains_sensitive_marker(value))
        || trace
            .terminal_reason
            .as_deref()
            .is_some_and(|value| value.trim().is_empty() || contains_sensitive_marker(value))
        || trace
            .delivery_status
            .as_deref()
            .is_some_and(|value| value.trim().is_empty() || contains_sensitive_marker(value))
        || trace
            .error_label
            .as_deref()
            .is_some_and(|value| value.trim().is_empty() || contains_sensitive_marker(value))
    {
        return Err(WeixinStateError::InvalidRecord {
            reason: "latency trace failed validation",
        });
    }
    Ok(())
}

fn validate_delivery_commit_item(
    account_id: &str,
    item: &WeixinPendingDeliveryCommitItem,
) -> Result<(), WeixinStateError> {
    if !looks_redacted("account", account_id)
        || !item.delivery_id.starts_with("delivery#")
        || !item.peer_id_hash.starts_with("peer#")
        || !item.direct_message_key.starts_with("dm#")
        || !item.item_id.starts_with("item#")
        || item.session_id.trim().is_empty()
        || contains_sensitive_marker(&item.session_id)
        || !item.message_hash.starts_with("reply#")
        || item.total_segments == 0
        || item.segment_index >= item.total_segments
    {
        return Err(WeixinStateError::InvalidRecord {
            reason: "pending delivery commit failed validation",
        });
    }
    validate_encrypted_payload(&item.encrypted_payload)
}

fn refresh_delivery_manifest_states(snapshot: &mut WeixinStateSnapshot) {
    let updated_at_millis = snapshot.updated_at_millis;
    for manifest in &mut snapshot.delivery_manifests {
        let mut has_pending = false;
        let mut has_unknown = false;
        let mut has_failed = false;
        let mut all_succeeded = true;
        for delivery_id in &manifest.delivery_ids {
            if snapshot
                .pending_deliveries
                .iter()
                .any(|delivery| &delivery.delivery_id == delivery_id)
            {
                has_pending = true;
                all_succeeded = false;
                continue;
            }
            match snapshot
                .deliveries
                .iter()
                .find(|delivery| &delivery.delivery_id == delivery_id)
                .map(|delivery| delivery.state)
            {
                Some(WeixinDeliveryState::Succeeded) => {}
                Some(WeixinDeliveryState::Unknown) => {
                    has_unknown = true;
                    all_succeeded = false;
                }
                Some(WeixinDeliveryState::Failed)
                | Some(WeixinDeliveryState::Cancelled)
                | Some(WeixinDeliveryState::Expired) => {
                    has_failed = true;
                    all_succeeded = false;
                }
                Some(WeixinDeliveryState::Pending | WeixinDeliveryState::Running) | None => {
                    has_pending = true;
                    all_succeeded = false;
                }
            }
        }
        manifest.state = if has_pending {
            WeixinDeliveryManifestState::Pending
        } else if has_unknown {
            WeixinDeliveryManifestState::Unknown
        } else if has_failed {
            WeixinDeliveryManifestState::Failed
        } else if all_succeeded {
            WeixinDeliveryManifestState::Succeeded
        } else {
            WeixinDeliveryManifestState::Pending
        };
        manifest.updated_at_millis = updated_at_millis;
    }
}

fn validate_remote_control_commit_item(
    account_id: &str,
    item: &WeixinRemoteControlCommitItem,
) -> Result<(), WeixinStateError> {
    if !looks_redacted("account", account_id)
        || !item.request_id.starts_with("wxctl#")
        || !item.peer_id_hash.starts_with("peer#")
        || !item.direct_message_key.starts_with("dm#")
        || !item.item_id.starts_with("item#")
        || item.session_id.trim().is_empty()
        || contains_sensitive_marker(&item.session_id)
        || item.action.trim().is_empty()
        || contains_sensitive_marker(&item.action)
        || contains_sensitive_marker(&item.reason)
        || item
            .cwd_label
            .as_deref()
            .is_some_and(|cwd_label| !cwd_label.starts_with("cwd#"))
    {
        return Err(WeixinStateError::InvalidRecord {
            reason: "remote control commit failed validation",
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

fn rewrite_conversation_binding_session_fields(object: &mut serde_json::Map<String, Value>) {
    let Some(items) = object
        .get_mut("conversation_bindings")
        .and_then(Value::as_array_mut)
    else {
        return;
    };
    for item in items {
        let Some(item) = item.as_object_mut() else {
            continue;
        };
        let session_id = item
            .get("session_id")
            .and_then(Value::as_str)
            .map(str::to_string);
        if let Some(session_id) = session_id {
            item.entry("root_session_id".to_string())
                .or_insert_with(|| json!(session_id));
            item.entry("active_session_id".to_string())
                .or_insert_with(|| json!(session_id));
            item.entry("last_completed_session_id".to_string())
                .or_insert_with(|| json!(session_id));
        }
    }
}

fn rewrite_pending_inbound_runtime_fields(object: &mut serde_json::Map<String, Value>) {
    let Some(items) = object
        .get_mut("pending_inbound")
        .and_then(Value::as_array_mut)
    else {
        return;
    };
    for item in items {
        let Some(item) = item.as_object_mut() else {
            continue;
        };
        item.entry("direct_message_key".to_string())
            .or_insert_with(|| json!(""));
        item.entry("dispatch_retry_count".to_string())
            .or_insert_with(|| json!(0));
        item.entry("turn_session_id".to_string())
            .or_insert(Value::Null);
        item.entry("parent_session_id".to_string())
            .or_insert(Value::Null);
        item.entry("next_retry_at_millis".to_string())
            .or_insert(Value::Null);
        item.entry("last_dispatch_error".to_string())
            .or_insert(Value::Null);
        item.entry("lease_owner".to_string()).or_insert(Value::Null);
        item.entry("lease_token".to_string()).or_insert(Value::Null);
        item.entry("lease_started_at_millis".to_string())
            .or_insert(Value::Null);
        item.entry("lease_expires_at_millis".to_string())
            .or_insert(Value::Null);
    }
}

fn rewrite_pending_delivery_fields(object: &mut serde_json::Map<String, Value>) {
    let account_id = object
        .get("account_id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let Some(items) = object
        .get_mut("pending_deliveries")
        .and_then(Value::as_array_mut)
    else {
        return;
    };
    for item in items {
        let Some(item) = item.as_object_mut() else {
            continue;
        };
        item.entry("account_id".to_string())
            .or_insert_with(|| json!(account_id));
        item.entry("peer_id_hash".to_string())
            .or_insert_with(|| json!("peer#00000000"));
        item.entry("direct_message_key".to_string())
            .or_insert_with(|| json!("dm#00000000"));
        item.entry("item_id".to_string())
            .or_insert_with(|| json!("item#00000000"));
        item.entry("session_id".to_string())
            .or_insert_with(|| json!("migrated-delivery-session"));
        item.entry("message_hash".to_string())
            .or_insert_with(|| json!("reply#00000000"));
        item.entry("segment_index".to_string())
            .or_insert_with(|| json!(0));
        item.entry("total_segments".to_string())
            .or_insert_with(|| json!(1));
        item.entry("encrypted_payload".to_string())
            .or_insert(Value::Null);
        item.entry("last_status".to_string()).or_insert(Value::Null);
        item.entry("last_redacted_error".to_string())
            .or_insert(Value::Null);
        item.entry("next_retry_at_millis".to_string())
            .or_insert(Value::Null);
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
    turn_session_id: &str,
    parent_session_id: Option<&str>,
    source_label: &str,
    now_millis: u64,
) -> Result<WeixinConversationBinding, WeixinStateError> {
    if !looks_redacted("account", account_id)
        || snapshot.account_id != account_id
        || !peer_id_hash.starts_with("peer#")
        || !direct_message_key.starts_with("dm#")
        || snapshot.workspace_id != workspace_id
        || !looks_redacted("workspace", workspace_id)
        || turn_session_id.trim().is_empty()
        || contains_sensitive_marker(turn_session_id)
        || parent_session_id.is_some_and(|session_id| {
            session_id.trim().is_empty() || contains_sensitive_marker(session_id)
        })
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
        let root_session_id = binding
            .root_session_id
            .clone()
            .or_else(|| binding.last_completed_session_id.clone())
            .unwrap_or_else(|| binding.session_id.clone());
        binding.root_session_id = Some(root_session_id);
        binding.active_session_id = Some(turn_session_id.to_string());
        binding.workspace_id = workspace_id.to_string();
        binding.session_id = turn_session_id.to_string();
        binding.source_label = source_label.to_string();
        binding.last_activity_millis = now_millis;
        binding.updated_at_millis = now_millis;
        binding.transitioned_at_millis = now_millis;
        if binding.last_completed_session_id.is_none() {
            binding.last_completed_session_id = parent_session_id.map(str::to_string);
        }
        return Ok(binding.clone());
    }

    let binding = WeixinConversationBinding {
        schema_version: WEIXIN_STATE_SCHEMA_VERSION,
        account_id: account_id.to_string(),
        peer_id_hash: peer_id_hash.to_string(),
        direct_message_key: direct_message_key.to_string(),
        workspace_id: workspace_id.to_string(),
        root_session_id: Some(turn_session_id.to_string()),
        active_session_id: Some(turn_session_id.to_string()),
        last_completed_session_id: parent_session_id.map(str::to_string),
        session_id: turn_session_id.to_string(),
        source_label: source_label.to_string(),
        created_at_millis: now_millis,
        last_activity_millis: now_millis,
        updated_at_millis: now_millis,
        transitioned_at_millis: now_millis,
    };
    snapshot.conversation_bindings.push(binding.clone());
    Ok(binding)
}

fn conversation_binding_parent_session_id(
    snapshot: &WeixinStateSnapshot,
    account_id: &str,
    peer_id_hash: &str,
    direct_message_key: &str,
) -> Option<String> {
    snapshot
        .conversation_bindings
        .iter()
        .find(|binding| {
            binding.account_id == account_id
                && binding.peer_id_hash == peer_id_hash
                && binding.direct_message_key == direct_message_key
        })
        .and_then(|binding| {
            binding
                .last_completed_session_id
                .clone()
                .or_else(|| binding.active_session_id.clone())
                .or_else(|| (!binding.session_id.is_empty()).then(|| binding.session_id.clone()))
        })
}

fn complete_conversation_binding_for_pending(
    snapshot: &mut WeixinStateSnapshot,
    pending: &WeixinPendingInbound,
    target: WeixinPendingInboundState,
    now_millis: u64,
) -> Result<(), WeixinStateError> {
    let Some(turn_session_id) = pending.turn_session_id.as_deref() else {
        return Ok(());
    };
    let Some(binding) = snapshot.conversation_bindings.iter_mut().find(|binding| {
        binding.account_id == pending.account_id
            && binding.peer_id_hash == pending.peer_id_hash
            && binding.direct_message_key == pending.direct_message_key
            && binding.active_session_id.as_deref() == Some(turn_session_id)
    }) else {
        return Ok(());
    };
    if binding.root_session_id.is_none() {
        binding.root_session_id = Some(turn_session_id.to_string());
    }
    if target == WeixinPendingInboundState::Succeeded {
        binding.last_completed_session_id = Some(turn_session_id.to_string());
        binding.active_session_id = Some(turn_session_id.to_string());
        binding.session_id = turn_session_id.to_string();
    } else if let Some(parent_session_id) = pending.parent_session_id.clone() {
        binding.active_session_id = Some(parent_session_id.clone());
        binding.session_id = parent_session_id;
    }
    binding.last_activity_millis = now_millis;
    binding.updated_at_millis = now_millis;
    binding.transitioned_at_millis = now_millis;
    Ok(())
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

fn prune_latency_traces(snapshot: &mut WeixinStateSnapshot) {
    if snapshot.latency_traces.len() <= WEIXIN_LATENCY_TRACE_LIMIT {
        return;
    }
    snapshot
        .latency_traces
        .sort_by_key(|trace| trace.created_at_millis);
    let remove_count = snapshot
        .latency_traces
        .len()
        .saturating_sub(WEIXIN_LATENCY_TRACE_LIMIT);
    snapshot.latency_traces.drain(0..remove_count);
}

fn update_latency_trace_for_item<F>(
    snapshot: &mut WeixinStateSnapshot,
    item_id: &str,
    now_millis: u64,
    update: F,
) where
    F: FnOnce(&mut WeixinLatencyTraceRecord),
{
    if let Some(trace) = snapshot
        .latency_traces
        .iter_mut()
        .find(|trace| trace.item_id == item_id)
    {
        update(trace);
        trace.updated_at_millis = now_millis;
    }
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

fn duration_millis(start: Option<u64>, end: Option<u64>) -> Option<u64> {
    Some(end?.saturating_sub(start?))
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

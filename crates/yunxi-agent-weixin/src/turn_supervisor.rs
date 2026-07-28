use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use thiserror::Error;
use tokio::sync::{Mutex as TokioMutex, OwnedMutexGuard};
use yunxi_agent_core::{
    Agent, AgentBackend, AgentConfig, AgentInput, AgentRunControl, AgentRunStatus,
};
use yunxi_agent_storage::{
    FileWeixinStateStore, WeixinPendingInboundState, WeixinRuntimeTurnBeginRequest,
    WeixinStateError,
};

use crate::inbound::{WeixinInboundKind, WeixinPendingInboundPayload};
use crate::payload_cipher::{WeixinPayloadCipher, WeixinPayloadCipherError};
use crate::redaction::SecretString;

const DEFAULT_MAX_QUEUE_PER_CONVERSATION: usize = 8;
const DEFAULT_MAX_GLOBAL_QUEUE: usize = 64;
const DEFAULT_SOURCE_LABEL: &str = "weixin-private-chat";
const DEFAULT_SESSION_TITLE: &str = "Weixin private chat";

#[derive(Clone)]
pub struct WeixinTurnSupervisor {
    state_store: FileWeixinStateStore,
    data_key: SecretString,
    options: WeixinTurnSupervisorOptions,
    sink: Arc<dyn WeixinRuntimeSink>,
    queue_limiter: Arc<ConversationQueueLimiter>,
    payload_cipher: WeixinPayloadCipher,
}

impl WeixinTurnSupervisor {
    pub fn new(
        state_store: FileWeixinStateStore,
        data_key: SecretString,
        options: WeixinTurnSupervisorOptions,
        sink: Arc<dyn WeixinRuntimeSink>,
    ) -> Self {
        Self {
            state_store,
            data_key,
            options,
            sink,
            queue_limiter: Arc::new(ConversationQueueLimiter::default()),
            payload_cipher: WeixinPayloadCipher::new(),
        }
    }

    pub fn with_test_sink(
        state_store: FileWeixinStateStore,
        data_key: SecretString,
        options: WeixinTurnSupervisorOptions,
        sink: WeixinRuntimeTestSink,
    ) -> Self {
        Self::new(state_store, data_key, options, Arc::new(sink))
    }

    pub async fn run_pending_turn<B>(
        &self,
        backend: &B,
        account_id: &str,
        item_id: &str,
    ) -> Result<WeixinTurnReport, WeixinTurnSupervisorError>
    where
        B: AgentBackend,
    {
        let pending = self
            .state_store
            .load_pending_inbound(account_id, item_id)?
            .ok_or_else(|| WeixinStateError::PendingInboundNotFound {
                item_id: item_id.to_string(),
            })?;
        if pending.state != WeixinPendingInboundState::Ready {
            return Err(WeixinTurnSupervisorError::InvalidPendingState {
                item_id: item_id.to_string(),
                state: pending.state.as_str(),
            });
        }

        let payload = self
            .payload_cipher
            .decrypt_pending_inbound(&self.data_key, &pending)?;
        let text = payload
            .text
            .as_ref()
            .filter(|text| !text.is_empty())
            .ok_or(WeixinTurnSupervisorError::UnsupportedPayload)?;
        if payload.payload_kind != WeixinInboundKind::Text {
            return Err(WeixinTurnSupervisorError::UnsupportedPayload);
        }

        let queue_key = ConversationQueueKey::from_payload(&payload);
        let _slot = self
            .queue_limiter
            .acquire(
                queue_key,
                self.options.max_queue_per_conversation,
                self.options.max_global_queue,
            )
            .await?;

        let candidate_session_id = self.candidate_session_id(&payload);
        let binding =
            self.state_store
                .begin_pending_runtime_turn(WeixinRuntimeTurnBeginRequest {
                    account_id: payload.account_id.clone(),
                    peer_id_hash: payload.peer_id_hash.clone(),
                    message_id_hash: payload.message_id_hash.clone(),
                    item_id: payload.item_id.clone(),
                    direct_message_key: payload.direct_message_key.clone(),
                    workspace_id: self.options.workspace_id.clone(),
                    candidate_session_id,
                    source_label: self.options.source_label.clone(),
                    now_millis: now_millis_u64(),
                })?;

        let runtime_config = self
            .options
            .config
            .clone()
            .with_session_id(binding.session_id.clone())
            .with_session_title(DEFAULT_SESSION_TITLE);
        let result = Agent::new(runtime_config)
            .run_with_backend_stream(
                backend,
                AgentInput::text(text.expose().to_string()),
                AgentRunControl::detached(),
            )
            .await;

        let report = match result {
            Ok(result) => {
                let target = match result.status {
                    AgentRunStatus::Completed => WeixinPendingInboundState::Succeeded,
                    AgentRunStatus::Failed => WeixinPendingInboundState::Failed,
                    AgentRunStatus::Cancelled => WeixinPendingInboundState::Cancelled,
                };
                let mut final_response_present = false;
                if target == WeixinPendingInboundState::Succeeded
                    && let Some(final_response) = result.final_response.as_ref()
                    && !final_response.trim().is_empty()
                {
                    self.sink.write_final_response(WeixinRuntimeSinkRecord {
                        account_id: payload.account_id.clone(),
                        peer_id_hash: payload.peer_id_hash.clone(),
                        direct_message_key: payload.direct_message_key.clone(),
                        item_id: payload.item_id.clone(),
                        session_id: binding.session_id.clone(),
                        final_response: final_response.clone(),
                    })?;
                    final_response_present = true;
                }
                self.state_store.complete_pending_runtime_turn(
                    &payload.account_id,
                    &payload.item_id,
                    target,
                    terminal_reason_for_status(result.status),
                    now_millis_u64(),
                )?;
                WeixinTurnReport {
                    account_id: payload.account_id,
                    peer_id_hash: payload.peer_id_hash,
                    direct_message_key: payload.direct_message_key,
                    item_id: payload.item_id,
                    session_id: binding.session_id,
                    status: result.status,
                    final_response_present,
                }
            }
            Err(_error) => {
                self.state_store.complete_pending_runtime_turn(
                    &payload.account_id,
                    &payload.item_id,
                    WeixinPendingInboundState::Failed,
                    Some("runtime_failed".to_string()),
                    now_millis_u64(),
                )?;
                WeixinTurnReport {
                    account_id: payload.account_id,
                    peer_id_hash: payload.peer_id_hash,
                    direct_message_key: payload.direct_message_key,
                    item_id: payload.item_id,
                    session_id: binding.session_id,
                    status: AgentRunStatus::Failed,
                    final_response_present: false,
                }
            }
        };

        Ok(report)
    }

    fn candidate_session_id(&self, payload: &WeixinPendingInboundPayload) -> String {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        payload.account_id.hash(&mut hasher);
        payload.peer_id_hash.hash(&mut hasher);
        payload.direct_message_key.hash(&mut hasher);
        self.options.workspace_id.hash(&mut hasher);
        format!("yunxi-weixin-{:016x}", hasher.finish())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeixinTurnSupervisorOptions {
    pub config: AgentConfig,
    pub workspace_id: String,
    pub max_queue_per_conversation: usize,
    pub max_global_queue: usize,
    pub source_label: String,
}

impl WeixinTurnSupervisorOptions {
    pub fn new(config: AgentConfig, workspace_id: impl Into<String>) -> Self {
        Self {
            config,
            workspace_id: workspace_id.into(),
            max_queue_per_conversation: DEFAULT_MAX_QUEUE_PER_CONVERSATION,
            max_global_queue: DEFAULT_MAX_GLOBAL_QUEUE,
            source_label: DEFAULT_SOURCE_LABEL.to_string(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeixinTurnReport {
    pub account_id: String,
    pub peer_id_hash: String,
    pub direct_message_key: String,
    pub item_id: String,
    pub session_id: String,
    pub status: AgentRunStatus,
    pub final_response_present: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeixinRuntimeSinkRecord {
    pub account_id: String,
    pub peer_id_hash: String,
    pub direct_message_key: String,
    pub item_id: String,
    pub session_id: String,
    pub final_response: String,
}

pub trait WeixinRuntimeSink: Send + Sync {
    fn write_final_response(
        &self,
        record: WeixinRuntimeSinkRecord,
    ) -> Result<(), WeixinTurnSupervisorError>;
}

#[derive(Clone, Default)]
pub struct NoopWeixinRuntimeSink;

impl WeixinRuntimeSink for NoopWeixinRuntimeSink {
    fn write_final_response(
        &self,
        _record: WeixinRuntimeSinkRecord,
    ) -> Result<(), WeixinTurnSupervisorError> {
        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct WeixinRuntimeTestSink {
    records: Arc<Mutex<Vec<WeixinRuntimeSinkRecord>>>,
}

impl WeixinRuntimeTestSink {
    pub fn records(&self) -> Vec<WeixinRuntimeSinkRecord> {
        self.records
            .lock()
            .map(|records| records.clone())
            .unwrap_or_default()
    }
}

impl WeixinRuntimeSink for WeixinRuntimeTestSink {
    fn write_final_response(
        &self,
        record: WeixinRuntimeSinkRecord,
    ) -> Result<(), WeixinTurnSupervisorError> {
        self.records
            .lock()
            .map_err(|_| WeixinTurnSupervisorError::SinkFailed)?
            .push(record);
        Ok(())
    }
}

#[async_trait]
pub trait WeixinRuntimeDispatcher: Send + Sync {
    async fn dispatch_pending_turn(
        &self,
        account_id: &str,
        item_id: &str,
    ) -> Result<WeixinTurnReport, WeixinTurnSupervisorError>;
}

pub struct WeixinRuntimeDispatcherAdapter<B> {
    supervisor: WeixinTurnSupervisor,
    backend: Arc<B>,
}

impl<B> WeixinRuntimeDispatcherAdapter<B> {
    pub fn new(supervisor: WeixinTurnSupervisor, backend: B) -> Self {
        Self {
            supervisor,
            backend: Arc::new(backend),
        }
    }
}

#[async_trait]
impl<B> WeixinRuntimeDispatcher for WeixinRuntimeDispatcherAdapter<B>
where
    B: AgentBackend + 'static,
{
    async fn dispatch_pending_turn(
        &self,
        account_id: &str,
        item_id: &str,
    ) -> Result<WeixinTurnReport, WeixinTurnSupervisorError> {
        self.supervisor
            .run_pending_turn(self.backend.as_ref(), account_id, item_id)
            .await
    }
}

#[derive(Debug, Error)]
pub enum WeixinTurnSupervisorError {
    #[error("weixin runtime supervisor state failed: {0}")]
    State(#[from] WeixinStateError),
    #[error("weixin runtime supervisor payload decrypt failed: {0}")]
    PayloadCipher(#[from] WeixinPayloadCipherError),
    #[error("weixin runtime supervisor only accepts text payloads")]
    UnsupportedPayload,
    #[error("weixin runtime supervisor pending item is not ready item={item_id} state={state}")]
    InvalidPendingState {
        item_id: String,
        state: &'static str,
    },
    #[error("weixin runtime supervisor queue is full scope={scope} limit={limit}")]
    QueueFull { scope: &'static str, limit: usize },
    #[error("weixin runtime supervisor sink failed")]
    SinkFailed,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ConversationQueueKey {
    account_id: String,
    peer_id_hash: String,
    direct_message_key: String,
}

impl ConversationQueueKey {
    fn from_payload(payload: &WeixinPendingInboundPayload) -> Self {
        Self {
            account_id: payload.account_id.clone(),
            peer_id_hash: payload.peer_id_hash.clone(),
            direct_message_key: payload.direct_message_key.clone(),
        }
    }
}

struct ConversationQueueLimiter {
    state: Arc<Mutex<ConversationQueueState>>,
}

impl Default for ConversationQueueLimiter {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(ConversationQueueState::default())),
        }
    }
}

impl ConversationQueueLimiter {
    async fn acquire(
        &self,
        key: ConversationQueueKey,
        max_queue_per_conversation: usize,
        max_global_queue: usize,
    ) -> Result<ConversationTurnSlot, WeixinTurnSupervisorError> {
        let (lock, queued) = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| WeixinTurnSupervisorError::SinkFailed)?;
            let running = state.running.contains(&key);
            let waiting = state
                .waiting_per_conversation
                .get(&key)
                .copied()
                .unwrap_or(0);
            let queued = running || waiting > 0;
            if queued {
                if waiting >= max_queue_per_conversation {
                    return Err(WeixinTurnSupervisorError::QueueFull {
                        scope: "conversation",
                        limit: max_queue_per_conversation,
                    });
                }
                if state.global_waiting >= max_global_queue {
                    return Err(WeixinTurnSupervisorError::QueueFull {
                        scope: "global",
                        limit: max_global_queue,
                    });
                }
                state.global_waiting += 1;
                *state
                    .waiting_per_conversation
                    .entry(key.clone())
                    .or_default() += 1;
            }
            (
                state
                    .locks
                    .entry(key.clone())
                    .or_insert_with(|| Arc::new(TokioMutex::new(())))
                    .clone(),
                queued,
            )
        };

        let guard = lock.lock_owned().await;
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| WeixinTurnSupervisorError::SinkFailed)?;
            if queued {
                state.global_waiting = state.global_waiting.saturating_sub(1);
                if let Some(waiting) = state.waiting_per_conversation.get_mut(&key) {
                    *waiting = waiting.saturating_sub(1);
                    if *waiting == 0 {
                        state.waiting_per_conversation.remove(&key);
                    }
                }
            }
            state.running.insert(key.clone());
        }

        Ok(ConversationTurnSlot {
            key,
            state: Arc::clone(&self.state),
            _guard: guard,
        })
    }
}

#[derive(Default)]
struct ConversationQueueState {
    locks: HashMap<ConversationQueueKey, Arc<TokioMutex<()>>>,
    running: HashSet<ConversationQueueKey>,
    waiting_per_conversation: HashMap<ConversationQueueKey, usize>,
    global_waiting: usize,
}

struct ConversationTurnSlot {
    key: ConversationQueueKey,
    state: Arc<Mutex<ConversationQueueState>>,
    _guard: OwnedMutexGuard<()>,
}

impl Drop for ConversationTurnSlot {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            state.running.remove(&self.key);
        }
    }
}

fn terminal_reason_for_status(status: AgentRunStatus) -> Option<String> {
    match status {
        AgentRunStatus::Completed => None,
        AgentRunStatus::Failed => Some("runtime_failed".to_string()),
        AgentRunStatus::Cancelled => Some("runtime_cancelled".to_string()),
    }
}

fn now_millis_u64() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

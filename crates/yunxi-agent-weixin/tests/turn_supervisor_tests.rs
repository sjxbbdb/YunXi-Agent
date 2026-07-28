use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use tempfile::TempDir;
use tokio::sync::Notify;
use yunxi_agent_core::{
    AgentBackend, AgentConfig, AgentInput, AgentResult, AgentRunControl, AgentRunResult,
    AgentRunStatus, ApprovalMode, SandboxMode,
};
use yunxi_agent_storage::{
    FileWeixinStateStore, WeixinConnectionStateRecord, WeixinInboundBatchCommit,
    WeixinInboundCommitItem, WeixinPendingInboundState, WeixinStateSnapshot, WeixinStateStore,
};
use yunxi_agent_weixin::ilink::{MessageItem, TextItem, WeixinMessage};
use yunxi_agent_weixin::{
    SecretString, WeixinInboundEnvelope, WeixinMessageId, WeixinPayloadAad, WeixinPayloadCipher,
    WeixinRuntimeTestSink, WeixinTurnSupervisor, WeixinTurnSupervisorError,
    WeixinTurnSupervisorOptions,
};

const ACCOUNT: &str = "account#933b5bde";
const WORKSPACE: &str = "workspace#73521066";
const ENDPOINT: &str = "https://ilinkai.weixin.qq.com/";

fn test_data_key() -> SecretString {
    SecretString::new("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
}

fn store_fixture() -> (TempDir, FileWeixinStateStore) {
    let temp = TempDir::new().expect("temp");
    let mut snapshot = WeixinStateSnapshot::new(ACCOUNT, WORKSPACE, ENDPOINT, 1000);
    snapshot.connection_state = WeixinConnectionStateRecord::Ready;
    let store = FileWeixinStateStore::for_workspace(temp.path());
    store.save(&snapshot).expect("save state");
    (temp, store)
}

fn message(raw_message_id: &str, raw_peer: &str, text: &str) -> WeixinMessage {
    WeixinMessage {
        message_id: WeixinMessageId::new(raw_message_id),
        from_user_id: SecretString::new(raw_peer),
        to_user_id: None,
        client_id: None,
        create_time_ms: Some(42),
        session_id: None,
        group_id: None,
        message_type: Some(1),
        message_state: None,
        item_list: vec![MessageItem {
            item_type: 1,
            text_item: Some(TextItem {
                text: SecretString::new(text),
            }),
            is_completed: Some(true),
            msg_id: None,
        }],
        context_token: None,
    }
}

fn seed_pending(
    store: &FileWeixinStateStore,
    account: &str,
    raw_message_id: &str,
    raw_peer: &str,
    text: &str,
    now_millis: u64,
) -> String {
    let message = message(raw_message_id, raw_peer, text);
    let envelope = WeixinInboundEnvelope::from_message(account, None, &message, now_millis);
    let item_id = envelope.pending_item_id();
    let plaintext = envelope
        .recoverable_text_payload(&message, &item_id)
        .expect("recoverable text payload");
    let aad = WeixinPayloadAad::new(
        &envelope.account_id,
        &envelope.peer_id_hash,
        &envelope.message_id_hash,
        &item_id,
    );
    let encrypted_payload = WeixinPayloadCipher::new()
        .encrypt_pending_inbound(&test_data_key(), &plaintext, &aad)
        .expect("encrypt pending");
    let mut commit = WeixinInboundBatchCommit::new(account.to_string(), now_millis);
    commit.accepted.push(WeixinInboundCommitItem {
        item_id: item_id.clone(),
        message_id_hash: envelope.message_id_hash.clone(),
        peer_id_hash: envelope.peer_id_hash.clone(),
        encrypted_payload_ref: envelope.encrypted_payload_ref(),
        encrypted_payload,
        payload_kind: Some("text".to_string()),
    });
    let result = store.commit_inbound_batch(commit).expect("commit pending");
    assert_eq!(result.accepted_item_ids, vec![item_id.clone()]);
    item_id
}

fn supervisor_options(workspace: &Path) -> WeixinTurnSupervisorOptions {
    let config = AgentConfig::new(workspace)
        .with_provider("configured-provider")
        .with_model("configured-model")
        .with_approval_mode(ApprovalMode::OnRequest)
        .with_sandbox_mode(SandboxMode::WorkspaceWrite)
        .with_context_window_tokens(12345);
    WeixinTurnSupervisorOptions::new(config, WORKSPACE)
}

#[derive(Clone, Default)]
struct CapturingBackend {
    calls: Arc<Mutex<Vec<(AgentConfig, String)>>>,
}

impl CapturingBackend {
    fn calls(&self) -> Vec<(AgentConfig, String)> {
        self.calls.lock().expect("calls").clone()
    }
}

#[async_trait]
impl AgentBackend for CapturingBackend {
    async fn run(&self, config: AgentConfig, input: AgentInput) -> AgentResult<AgentRunResult> {
        self.run_stream(config, input, AgentRunControl::detached())
            .await
    }

    async fn run_stream(
        &self,
        config: AgentConfig,
        input: AgentInput,
        _control: AgentRunControl,
    ) -> AgentResult<AgentRunResult> {
        self.calls
            .lock()
            .expect("calls")
            .push((config, input.prompt.clone()));
        Ok(AgentRunResult {
            status: AgentRunStatus::Completed,
            final_response: Some(format!("reply: {}", input.prompt)),
            events: Vec::new(),
        })
    }
}

#[tokio::test]
async fn supervisor_reuses_session_writes_sink_and_preserves_runtime_config() {
    let (temp, store) = store_fixture();
    let first_item = seed_pending(
        &store,
        ACCOUNT,
        "raw-message-1",
        "raw-peer-1",
        "hello",
        1100,
    );
    let second_item = seed_pending(
        &store,
        ACCOUNT,
        "raw-message-2",
        "raw-peer-1",
        "again",
        1200,
    );
    let third_item = seed_pending(
        &store,
        ACCOUNT,
        "raw-message-3",
        "raw-peer-2",
        "other",
        1300,
    );

    let sink = WeixinRuntimeTestSink::default();
    let supervisor = WeixinTurnSupervisor::with_test_sink(
        store.clone(),
        test_data_key(),
        supervisor_options(temp.path()),
        sink.clone(),
    );
    let backend = CapturingBackend::default();

    let first = supervisor
        .run_pending_turn(&backend, ACCOUNT, &first_item)
        .await
        .expect("first turn");
    let second = supervisor
        .run_pending_turn(&backend, ACCOUNT, &second_item)
        .await
        .expect("second turn");
    let third = supervisor
        .run_pending_turn(&backend, ACCOUNT, &third_item)
        .await
        .expect("third turn");

    assert_eq!(first.status, AgentRunStatus::Completed);
    assert_eq!(first.session_id, second.session_id);
    assert_ne!(first.session_id, third.session_id);
    assert!(first.final_response_present);
    assert!(second.final_response_present);
    assert!(third.final_response_present);

    let records = sink.records();
    assert_eq!(records.len(), 3);
    assert_eq!(records[0].final_response, "reply: hello");
    assert_eq!(records[1].session_id, first.session_id);
    assert_eq!(records[2].session_id, third.session_id);

    let calls = backend.calls();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0].1, "hello");
    assert_eq!(calls[1].1, "again");
    assert_eq!(calls[0].0.provider.as_deref(), Some("configured-provider"));
    assert_eq!(calls[0].0.model.as_deref(), Some("configured-model"));
    assert_eq!(calls[0].0.cwd, temp.path());
    assert_eq!(calls[0].0.approval_mode, ApprovalMode::OnRequest);
    assert_eq!(calls[0].0.sandbox_mode, SandboxMode::WorkspaceWrite);
    assert_eq!(calls[0].0.context_window_tokens, Some(12345));
    assert_eq!(
        calls[0].0.session_id.as_deref(),
        Some(first.session_id.as_str())
    );
    assert_eq!(
        calls[1].0.session_id.as_deref(),
        Some(first.session_id.as_str())
    );
    assert_eq!(
        calls[2].0.session_id.as_deref(),
        Some(third.session_id.as_str())
    );

    let state = store.load(ACCOUNT).expect("load").expect("state");
    assert_eq!(state.conversation_bindings.len(), 2);
    assert!(
        state
            .pending_inbound
            .iter()
            .all(|pending| pending.state == WeixinPendingInboundState::Succeeded)
    );
}

#[derive(Clone)]
struct BlockingBackend {
    started: Arc<Notify>,
    release: Arc<Notify>,
}

#[async_trait]
impl AgentBackend for BlockingBackend {
    async fn run(&self, config: AgentConfig, input: AgentInput) -> AgentResult<AgentRunResult> {
        self.run_stream(config, input, AgentRunControl::detached())
            .await
    }

    async fn run_stream(
        &self,
        _config: AgentConfig,
        input: AgentInput,
        _control: AgentRunControl,
    ) -> AgentResult<AgentRunResult> {
        self.started.notify_waiters();
        self.release.notified().await;
        Ok(AgentRunResult {
            status: AgentRunStatus::Completed,
            final_response: Some(format!("done: {}", input.prompt)),
            events: Vec::new(),
        })
    }
}

#[tokio::test]
async fn supervisor_queue_full_does_not_advance_second_pending() {
    let (temp, store) = store_fixture();
    let first_item = seed_pending(&store, ACCOUNT, "raw-message-1", "raw-peer-1", "hold", 1100);
    let second_item = seed_pending(
        &store,
        ACCOUNT,
        "raw-message-2",
        "raw-peer-1",
        "queued",
        1200,
    );

    let mut options = supervisor_options(temp.path());
    options.max_queue_per_conversation = 0;
    let supervisor = WeixinTurnSupervisor::with_test_sink(
        store.clone(),
        test_data_key(),
        options,
        WeixinRuntimeTestSink::default(),
    );
    let backend = BlockingBackend {
        started: Arc::new(Notify::new()),
        release: Arc::new(Notify::new()),
    };
    let first_backend = backend.clone();
    let first_supervisor = supervisor.clone();
    let first_item_for_task = first_item.clone();
    let first_task = tokio::spawn(async move {
        first_supervisor
            .run_pending_turn(&first_backend, ACCOUNT, &first_item_for_task)
            .await
    });

    tokio::time::timeout(Duration::from_secs(2), backend.started.notified())
        .await
        .expect("first backend started");
    let error = supervisor
        .run_pending_turn(&backend, ACCOUNT, &second_item)
        .await
        .expect_err("second turn should hit bounded queue");
    assert!(matches!(
        error,
        WeixinTurnSupervisorError::QueueFull {
            scope: "conversation",
            limit: 0
        }
    ));
    let state = store.load(ACCOUNT).expect("load").expect("state");
    let second = state
        .pending_inbound
        .iter()
        .find(|pending| pending.item_id == second_item)
        .expect("second pending");
    assert_eq!(second.state, WeixinPendingInboundState::Ready);

    backend.release.notify_waiters();
    first_task
        .await
        .expect("first task")
        .expect("first turn completes");
}

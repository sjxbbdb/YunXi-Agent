use std::fs;
#[cfg(windows)]
use std::{
    env,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use tempfile::TempDir;
use yunxi_agent_storage::{
    FileWeixinStateStore, WEIXIN_PAYLOAD_AAD_VERSION, WEIXIN_PAYLOAD_ALGORITHM,
    WEIXIN_PAYLOAD_ALGORITHM_VERSION, WEIXIN_STATE_SCHEMA_VERSION, WeixinAccountLockState,
    WeixinConnectionStateRecord, WeixinCredentialReferenceRecord, WeixinEncryptedPayload,
    WeixinInboundBatchCommit, WeixinInboundCommitItem, WeixinPairRequestCommitItem,
    WeixinPairRequestState, WeixinPendingInbound, WeixinPendingInboundState,
    WeixinRuntimeTurnBeginRequest, WeixinStateError, WeixinStateSnapshot, WeixinStateStore,
    WeixinStateWriteOptions,
};

const ACCOUNT: &str = "account#933b5bde";
const WORKSPACE: &str = "workspace#73521066";
const ENDPOINT: &str = "https://ilinkai.weixin.qq.com/";

fn store_fixture() -> (TempDir, FileWeixinStateStore) {
    let temp = TempDir::new().expect("tempdir");
    let lock_root = temp.path().join("locks");
    let store = FileWeixinStateStore::for_workspace_with_lock_root(temp.path(), lock_root);
    (temp, store)
}

fn snapshot(now: u64) -> WeixinStateSnapshot {
    let mut snapshot = WeixinStateSnapshot::new(ACCOUNT, WORKSPACE, ENDPOINT, now);
    snapshot.connection_state = WeixinConnectionStateRecord::Ready;
    snapshot.credential = Some(WeixinCredentialReferenceRecord {
        backend: "windows-credential-manager".to_string(),
        token_target: "YunXiAgent/Weixin/installation-test/account-933b5bde/token".to_string(),
        data_key_target: "YunXiAgent/Weixin/installation-test/account-933b5bde/data-key"
            .to_string(),
    });
    snapshot
}

fn encrypted_payload() -> WeixinEncryptedPayload {
    WeixinEncryptedPayload {
        algorithm: WEIXIN_PAYLOAD_ALGORITHM.to_string(),
        algorithm_version: WEIXIN_PAYLOAD_ALGORITHM_VERSION,
        aad_version: WEIXIN_PAYLOAD_AAD_VERSION,
        nonce: "AAECAwQFBgcICQoL".to_string(),
        ciphertext: "Y2lwaGVydGV4dA==".to_string(),
    }
}

fn commit_item(
    item_id: &str,
    message_id_hash: &str,
    peer_id_hash: &str,
) -> WeixinInboundCommitItem {
    WeixinInboundCommitItem {
        item_id: item_id.to_string(),
        message_id_hash: message_id_hash.to_string(),
        peer_id_hash: peer_id_hash.to_string(),
        direct_message_key: peer_id_hash.replace("peer#", "dm#"),
        encrypted_payload_ref: item_id.replace("item#", "pending#"),
        encrypted_payload: encrypted_payload(),
        payload_kind: Some("text".to_string()),
    }
}

fn pending_item(encrypted_payload: Option<WeixinEncryptedPayload>) -> WeixinPendingInbound {
    WeixinPendingInbound {
        schema_version: WEIXIN_STATE_SCHEMA_VERSION,
        item_id: "item#00000001".to_string(),
        account_id: ACCOUNT.to_string(),
        message_id_hash: "message#00000001".to_string(),
        peer_id_hash: "peer#00000001".to_string(),
        direct_message_key: "dm#00000001".to_string(),
        encrypted_payload_ref: "pending#00000001".to_string(),
        payload_kind: Some("text".to_string()),
        encrypted_payload,
        turn_session_id: None,
        parent_session_id: None,
        dispatch_retry_count: 0,
        next_retry_at_millis: None,
        last_dispatch_error: None,
        state: WeixinPendingInboundState::Ready,
        terminal_reason: None,
        created_at_millis: 1000,
        updated_at_millis: 1000,
        transitioned_at_millis: 1000,
    }
}

#[test]
fn atomic_write_preserves_old_state_when_replace_is_not_reached() {
    let (_temp, store) = store_fixture();
    let original = snapshot(1000);
    store.save(&original).expect("initial save");

    let mut next = original.clone();
    next.updated_at_millis = 2000;
    let error = store
        .save_with_options(
            &next,
            WeixinStateWriteOptions {
                fail_before_replace: true,
            },
        )
        .expect_err("failure is injected before replace");
    assert!(matches!(
        error,
        WeixinStateError::InjectedFailure {
            operation: "before_replace"
        }
    ));
    assert_eq!(
        store
            .load(ACCOUNT)
            .expect("load")
            .expect("state")
            .updated_at_millis,
        1000
    );
    assert_eq!(store.temp_file_candidates().expect("temps").len(), 1);
}

#[test]
fn damaged_and_future_schema_records_fail_without_rewriting_target() {
    let (_temp, store) = store_fixture();
    let path = store.state_path_for(ACCOUNT);
    fs::create_dir_all(path.parent().expect("parent")).expect("state dir");
    fs::write(&path, "{not-json").expect("write damaged json");
    assert!(matches!(
        store.load(ACCOUNT),
        Err(WeixinStateError::InvalidJson { .. })
    ));

    fs::write(
        &path,
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version": WEIXIN_STATE_SCHEMA_VERSION + 1,
            "account_id": ACCOUNT,
            "workspace_id": WORKSPACE,
            "endpoint": ENDPOINT
        }))
        .expect("json"),
    )
    .expect("write future schema");
    assert!(matches!(
        store.load(ACCOUNT),
        Err(WeixinStateError::FutureSchema { .. })
    ));
}

#[test]
fn legacy_schema_is_migrated_in_memory_to_current_snapshot() {
    let (_temp, store) = store_fixture();
    let path = store.state_path_for(ACCOUNT);
    fs::create_dir_all(path.parent().expect("parent")).expect("state dir");
    fs::write(
        &path,
        serde_json::to_string_pretty(&serde_json::json!({
            "account_id": ACCOUNT,
            "workspace_id": WORKSPACE,
            "endpoint": ENDPOINT
        }))
        .expect("json"),
    )
    .expect("write legacy state");

    let loaded = store.load(ACCOUNT).expect("load").expect("state");
    assert_eq!(loaded.schema_version, WEIXIN_STATE_SCHEMA_VERSION);
    assert_eq!(loaded.pair_requests.len(), 0);
    assert_eq!(loaded.conversation_bindings.len(), 0);
    assert_eq!(loaded.pending_inbound_count(), 0);

    fs::write(
        &path,
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version": 2,
            "account_id": ACCOUNT,
            "workspace_id": WORKSPACE,
            "endpoint": ENDPOINT,
            "session_bindings": [{
                "schema_version": 2,
                "account_id": ACCOUNT,
                "peer_id_hash": "peer#00000001",
                "session_id": "yunxi-legacy-session",
                "created_at_millis": 1000,
                "updated_at_millis": 1000
            }]
        }))
        .expect("json"),
    )
    .expect("write v2 state");
    let migrated = store.load(ACCOUNT).expect("load").expect("state");
    assert_eq!(migrated.schema_version, WEIXIN_STATE_SCHEMA_VERSION);
    assert!(migrated.conversation_bindings.is_empty());

    fs::write(
        &path,
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version": 1,
            "account_id": ACCOUNT,
            "workspace_id": WORKSPACE,
            "endpoint": ENDPOINT,
            "pending_inbound": [{
                "schema_version": 1,
                "item_id": "item#00000001",
                "account_id": ACCOUNT,
                "message_id_hash": "message#00000001",
                "peer_id_hash": "peer#00000001",
                "encrypted_payload_ref": "pending#00000001",
                "state": "ready",
                "terminal_reason": null,
                "created_at_millis": 1000,
                "updated_at_millis": 1000,
                "transitioned_at_millis": 1000
            }]
        }))
        .expect("json"),
    )
    .expect("write legacy state with pending inbound");
    assert!(matches!(
        store.load(ACCOUNT),
        Err(WeixinStateError::InvalidRecord { .. })
    ));
}

#[test]
fn pair_request_lifecycle_enforces_account_expiry_and_terminal_state() {
    let (_temp, store) = store_fixture();
    store.save(&snapshot(1000)).expect("save state");
    let request = store
        .add_pair_request(ACCOUNT, "peer#00000001", 5000, 1100)
        .expect("pair request");
    assert!(request.request_id.starts_with("pair-"));
    let approved = store
        .approve_pair_request(ACCOUNT, &request.request_id, 1200)
        .expect("approve");
    assert_eq!(approved.state, WeixinPairRequestState::Approved);
    assert!(matches!(
        store.deny_pair_request(ACCOUNT, &request.request_id, 1300),
        Err(WeixinStateError::PairRequestConsumed { .. })
    ));

    let expired = store
        .add_pair_request(ACCOUNT, "peer#00000002", 1400, 1300)
        .expect("expired pair request");
    assert!(matches!(
        store.approve_pair_request(ACCOUNT, &expired.request_id, 1500),
        Err(WeixinStateError::PairRequestExpired { .. })
    ));
    let state = store.load(ACCOUNT).expect("load").expect("state");
    assert_eq!(state.pair_requests.len(), 2);
    assert_eq!(
        state.pair_requests[1].state,
        WeixinPairRequestState::Expired
    );
}

#[test]
fn pending_inbound_state_machine_is_strict_and_terminal_states_are_not_recovered() {
    let mut item = WeixinPendingInbound {
        schema_version: WEIXIN_STATE_SCHEMA_VERSION,
        item_id: "item#00000001".to_string(),
        account_id: ACCOUNT.to_string(),
        message_id_hash: "message#00000001".to_string(),
        peer_id_hash: "peer#00000001".to_string(),
        direct_message_key: "dm#00000001".to_string(),
        encrypted_payload_ref: "pending#00000001".to_string(),
        payload_kind: Some("text".to_string()),
        encrypted_payload: Some(encrypted_payload()),
        turn_session_id: None,
        parent_session_id: None,
        dispatch_retry_count: 0,
        next_retry_at_millis: None,
        last_dispatch_error: None,
        state: WeixinPendingInboundState::Accepted,
        terminal_reason: None,
        created_at_millis: 1000,
        updated_at_millis: 1000,
        transitioned_at_millis: 1000,
    };
    assert!(
        item.transition(WeixinPendingInboundState::Running, 1100)
            .is_err()
    );
    item.transition(WeixinPendingInboundState::Ready, 1100)
        .expect("accepted -> ready");
    item.transition(WeixinPendingInboundState::Running, 1200)
        .expect("ready -> running");
    item.transition(WeixinPendingInboundState::Succeeded, 1300)
        .expect("running -> terminal");

    let (_temp, store) = store_fixture();
    let mut snapshot = snapshot(1000);
    snapshot.pending_inbound.push(item);
    store.save(&snapshot).expect("save state");
    let loaded = store.load(ACCOUNT).expect("load").expect("state");
    assert_eq!(loaded.pending_inbound_count(), 0);
}

#[test]
fn account_lock_is_global_per_account_and_recovers_only_when_probe_reports_stale() {
    let temp = TempDir::new().expect("temp");
    let lock_root = temp.path().join("locks");
    let workspace_a = temp.path().join("a");
    let workspace_b = temp.path().join("b");
    let store_a = FileWeixinStateStore::for_workspace_with_lock_root(&workspace_a, &lock_root)
        .with_process_probe(|_| true);
    let store_b = FileWeixinStateStore::for_workspace_with_lock_root(&workspace_b, &lock_root)
        .with_process_probe(|_| true);
    let mut lock = store_a
        .try_acquire_account_lock(ACCOUNT, WORKSPACE)
        .expect("first lock");
    assert!(matches!(
        store_b.try_acquire_account_lock(ACCOUNT, "workspace#11111111"),
        Err(WeixinStateError::LockActive { .. })
    ));
    assert_eq!(
        store_a.lock_state(ACCOUNT).expect("lock state").state,
        WeixinAccountLockState::Active
    );
    lock.release().expect("release");

    let stale_store = FileWeixinStateStore::for_workspace_with_lock_root(&workspace_a, &lock_root)
        .with_process_probe(|_| false);
    let stale_lock = stale_store
        .try_acquire_account_lock(ACCOUNT, WORKSPACE)
        .expect("create lock that will be stale to next store");
    std::mem::forget(stale_lock);
    let recovering_store =
        FileWeixinStateStore::for_workspace_with_lock_root(&workspace_b, &lock_root)
            .with_process_probe(|_| false);
    let mut recovered = recovering_store
        .try_acquire_account_lock(ACCOUNT, "workspace#11111111")
        .expect("stale lock recovered");
    recovered.release().expect("release recovered");
}

#[cfg(windows)]
fn wait_for_path(path: &Path, description: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !path.exists() {
        if Instant::now() >= deadline {
            panic!("timed out waiting for {description}: {}", path.display());
        }
        thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(windows)]
fn wait_for_stale_lock(store: &FileWeixinStateStore, account_id: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let state = store.lock_state(account_id).expect("lock state");
        if state.state == WeixinAccountLockState::Stale {
            return;
        }
        if Instant::now() >= deadline {
            panic!("timed out waiting for stale lock, last state={state:?}");
        }
        thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(windows)]
fn spawn_windows_lock_child(workspace: &Path, lock_root: &Path, ready_file: &Path) -> Child {
    Command::new(env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("windows_account_lock_child_holds_lock")
        .arg("--ignored")
        .arg("--test-threads=1")
        .arg("--nocapture")
        .env("YUNXI_TEST_LOCK_CHILD_WORKSPACE", workspace)
        .env("YUNXI_TEST_LOCK_CHILD_LOCK_ROOT", lock_root)
        .env("YUNXI_TEST_LOCK_CHILD_READY", ready_file)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn lock child")
}

#[cfg(windows)]
#[test]
fn account_lock_real_windows_process_probe_rejects_active_and_recovers_exited_pid() {
    let temp = TempDir::new().expect("temp");
    let workspace = temp.path().join("workspace");
    let lock_root = temp.path().join("locks");
    let ready_file = temp.path().join("lock-child.ready");
    let mut child = spawn_windows_lock_child(&workspace, &lock_root, &ready_file);
    wait_for_path(&ready_file, "lock child readiness");
    let child_pid: u32 = fs::read_to_string(&ready_file)
        .expect("read child pid")
        .trim()
        .parse()
        .expect("child pid");
    let store = FileWeixinStateStore::for_workspace_with_lock_root(&workspace, &lock_root);
    let active = store.lock_state(ACCOUNT).expect("active lock state");
    assert_eq!(active.state, WeixinAccountLockState::Active);
    assert_eq!(active.pid, Some(child_pid));
    assert!(matches!(
        store.try_acquire_account_lock(ACCOUNT, "workspace#11111111"),
        Err(WeixinStateError::LockActive { .. })
    ));

    child.kill().expect("kill lock child");
    let _ = child.wait().expect("wait lock child");
    wait_for_stale_lock(&store, ACCOUNT);

    let mut recovered = store
        .try_acquire_account_lock(ACCOUNT, WORKSPACE)
        .expect("recover stale lock from exited pid");
    let recovered_state = store.lock_state(ACCOUNT).expect("recovered lock state");
    assert_eq!(recovered_state.state, WeixinAccountLockState::Active);
    assert_eq!(recovered_state.pid, Some(std::process::id()));
    recovered.release().expect("release recovered");
}

#[cfg(windows)]
#[test]
#[ignore]
fn windows_account_lock_child_holds_lock() {
    let workspace = env::var_os("YUNXI_TEST_LOCK_CHILD_WORKSPACE")
        .map(PathBuf::from)
        .expect("child workspace");
    let lock_root = env::var_os("YUNXI_TEST_LOCK_CHILD_LOCK_ROOT")
        .map(PathBuf::from)
        .expect("child lock root");
    let ready_file = env::var_os("YUNXI_TEST_LOCK_CHILD_READY")
        .map(PathBuf::from)
        .expect("child ready file");
    let store = FileWeixinStateStore::for_workspace_with_lock_root(&workspace, &lock_root);
    let lock = store
        .try_acquire_account_lock(ACCOUNT, WORKSPACE)
        .expect("child lock");
    fs::write(&ready_file, std::process::id().to_string()).expect("write child ready");
    std::mem::forget(lock);
    loop {
        thread::park_timeout(Duration::from_secs(1));
    }
}

#[test]
fn inbound_batch_commit_writes_cursor_receipt_and_pending_in_one_snapshot() {
    let (_temp, store) = store_fixture();
    store.save(&snapshot(1000)).expect("save state");
    let mut commit = WeixinInboundBatchCommit::new(ACCOUNT, 1100);
    commit.next_get_updates_buf = Some("cursor-next".to_string());
    commit.accepted.push(commit_item(
        "item#00000001",
        "message#00000001",
        "peer#00000001",
    ));

    let result = store
        .commit_inbound_batch(commit)
        .expect("commit inbound batch");
    assert_eq!(result.accepted_count, 1);
    assert_eq!(result.duplicate_count, 0);
    assert_eq!(result.pending_inbound_count, 1);

    let state = store.load(ACCOUNT).expect("load").expect("state");
    assert_eq!(state.cursors.len(), 1);
    assert_eq!(
        state.cursors[0].get_updates_buf.as_deref(),
        Some("cursor-next")
    );
    assert_eq!(state.inbound_receipts.len(), 1);
    assert_eq!(state.pending_inbound.len(), 1);
    assert_eq!(
        state.pending_inbound[0].payload_kind.as_deref(),
        Some("text")
    );
    assert_eq!(
        state.pending_inbound[0].encrypted_payload.as_ref(),
        Some(&encrypted_payload())
    );
    assert_eq!(
        state.pending_inbound[0].state,
        WeixinPendingInboundState::Ready
    );
}

#[test]
fn inbound_batch_commit_is_idempotent_for_duplicate_message_receipts() {
    let (_temp, store) = store_fixture();
    store.save(&snapshot(1000)).expect("save state");
    let mut commit = WeixinInboundBatchCommit::new(ACCOUNT, 1100);
    commit.next_get_updates_buf = Some("cursor-1".to_string());
    commit.accepted.push(commit_item(
        "item#00000001",
        "message#00000001",
        "peer#00000001",
    ));
    store
        .commit_inbound_batch(commit.clone())
        .expect("first commit");

    let mut duplicate = commit;
    duplicate.now_millis = 1200;
    duplicate.next_get_updates_buf = Some("cursor-2".to_string());
    let result = store
        .commit_inbound_batch(duplicate)
        .expect("duplicate commit");
    assert_eq!(result.accepted_count, 0);
    assert_eq!(result.duplicate_count, 1);

    let state = store.load(ACCOUNT).expect("load").expect("state");
    assert_eq!(state.inbound_receipts.len(), 1);
    assert_eq!(state.pending_inbound.len(), 1);
    assert_eq!(
        state.cursors[0].get_updates_buf.as_deref(),
        Some("cursor-2")
    );
}

#[test]
fn inbound_batch_commit_creates_only_one_pending_pair_request_per_peer() {
    let (_temp, store) = store_fixture();
    store.save(&snapshot(1000)).expect("save state");
    let mut commit = WeixinInboundBatchCommit::new(ACCOUNT, 1100);
    commit.pair_requests.push(WeixinPairRequestCommitItem {
        peer_id_hash: "peer#00000001".to_string(),
        expires_at_millis: 6100,
    });
    commit.pair_requests.push(WeixinPairRequestCommitItem {
        peer_id_hash: "peer#00000001".to_string(),
        expires_at_millis: 6200,
    });
    let result = store.commit_inbound_batch(commit).expect("pair commit");
    assert_eq!(result.pair_request_count, 1);
    let state = store.load(ACCOUNT).expect("load").expect("state");
    assert_eq!(state.pair_requests.len(), 1);
    assert_eq!(
        state.pair_requests[0].state,
        WeixinPairRequestState::Pending
    );
}

#[test]
fn runtime_turn_binding_reuses_session_and_isolates_conversations() {
    let (_temp, store) = store_fixture();
    store.save(&snapshot(1000)).expect("save state");

    let mut first = WeixinInboundBatchCommit::new(ACCOUNT, 1100);
    first.accepted.push(commit_item(
        "item#00000001",
        "message#00000001",
        "peer#00000001",
    ));
    let first_result = store.commit_inbound_batch(first).expect("first commit");
    assert_eq!(first_result.accepted_item_ids, vec!["item#00000001"]);

    let first_binding = store
        .begin_pending_runtime_turn(WeixinRuntimeTurnBeginRequest {
            account_id: ACCOUNT.to_string(),
            peer_id_hash: "peer#00000001".to_string(),
            message_id_hash: "message#00000001".to_string(),
            item_id: "item#00000001".to_string(),
            direct_message_key: "dm#00000001".to_string(),
            workspace_id: WORKSPACE.to_string(),
            candidate_session_id: "yunxi-weixin-first".to_string(),
            source_label: "weixin-private-chat".to_string(),
            now_millis: 1200,
        })
        .expect("begin first runtime turn");
    assert_eq!(first_binding.session_id, "yunxi-weixin-first");
    assert_eq!(
        first_binding.root_session_id.as_deref(),
        Some("yunxi-weixin-first")
    );
    assert_eq!(
        first_binding.active_session_id.as_deref(),
        Some("yunxi-weixin-first")
    );
    assert_eq!(first_binding.last_completed_session_id, None);
    store
        .complete_pending_runtime_turn(
            ACCOUNT,
            "item#00000001",
            WeixinPendingInboundState::Succeeded,
            None,
            1300,
        )
        .expect("complete first");

    let mut second = WeixinInboundBatchCommit::new(ACCOUNT, 1400);
    second.accepted.push(commit_item(
        "item#00000002",
        "message#00000002",
        "peer#00000001",
    ));
    store.commit_inbound_batch(second).expect("second commit");
    let reused = store
        .begin_pending_runtime_turn(WeixinRuntimeTurnBeginRequest {
            account_id: ACCOUNT.to_string(),
            peer_id_hash: "peer#00000001".to_string(),
            message_id_hash: "message#00000002".to_string(),
            item_id: "item#00000002".to_string(),
            direct_message_key: "dm#00000001".to_string(),
            workspace_id: WORKSPACE.to_string(),
            candidate_session_id: "yunxi-weixin-second".to_string(),
            source_label: "weixin-private-chat".to_string(),
            now_millis: 1500,
        })
        .expect("begin reused runtime turn");
    assert_eq!(reused.session_id, "yunxi-weixin-second");
    assert_eq!(
        reused.root_session_id.as_deref(),
        Some(first_binding.session_id.as_str())
    );
    assert_eq!(
        reused.active_session_id.as_deref(),
        Some("yunxi-weixin-second")
    );
    assert_eq!(
        reused.last_completed_session_id.as_deref(),
        Some(first_binding.session_id.as_str())
    );

    let mut third = WeixinInboundBatchCommit::new(ACCOUNT, 1600);
    third.accepted.push(commit_item(
        "item#00000003",
        "message#00000003",
        "peer#00000002",
    ));
    store.commit_inbound_batch(third).expect("third commit");
    let isolated = store
        .begin_pending_runtime_turn(WeixinRuntimeTurnBeginRequest {
            account_id: ACCOUNT.to_string(),
            peer_id_hash: "peer#00000002".to_string(),
            message_id_hash: "message#00000003".to_string(),
            item_id: "item#00000003".to_string(),
            direct_message_key: "dm#00000002".to_string(),
            workspace_id: WORKSPACE.to_string(),
            candidate_session_id: "yunxi-weixin-third".to_string(),
            source_label: "weixin-private-chat".to_string(),
            now_millis: 1700,
        })
        .expect("begin isolated runtime turn");
    assert_ne!(isolated.session_id, first_binding.session_id);
    assert_eq!(isolated.last_completed_session_id, None);

    let state = store.load(ACCOUNT).expect("load").expect("state");
    assert_eq!(state.conversation_bindings.len(), 2);
    let first_peer_binding = state
        .conversation_bindings
        .iter()
        .find(|binding| binding.peer_id_hash == "peer#00000001")
        .expect("first peer binding");
    assert_eq!(
        first_peer_binding.root_session_id.as_deref(),
        Some("yunxi-weixin-first")
    );
    assert_eq!(
        first_peer_binding.active_session_id.as_deref(),
        Some("yunxi-weixin-second")
    );
    assert_eq!(
        first_peer_binding.last_completed_session_id.as_deref(),
        Some("yunxi-weixin-first")
    );
    assert_eq!(
        state.pending_inbound[1].turn_session_id.as_deref(),
        Some("yunxi-weixin-second")
    );
    assert_eq!(
        state.pending_inbound[1].parent_session_id.as_deref(),
        Some("yunxi-weixin-first")
    );
    assert_eq!(
        state.pending_inbound[0].state,
        WeixinPendingInboundState::Succeeded
    );
    assert_eq!(
        state.pending_inbound[1].state,
        WeixinPendingInboundState::Running
    );
    assert_eq!(
        state.pending_inbound[2].state,
        WeixinPendingInboundState::Running
    );
}

#[test]
fn inbound_batch_atomic_failure_preserves_cursor_and_pending_state() {
    let (_temp, store) = store_fixture();
    store.save(&snapshot(1000)).expect("save state");
    let mut commit = WeixinInboundBatchCommit::new(ACCOUNT, 1100);
    commit.next_get_updates_buf = Some("cursor-next".to_string());
    commit.accepted.push(commit_item(
        "item#00000001",
        "message#00000001",
        "peer#00000001",
    ));
    let error = store
        .commit_inbound_batch_with_options(
            commit,
            WeixinStateWriteOptions {
                fail_before_replace: true,
            },
        )
        .expect_err("injected failure");
    assert!(matches!(
        error,
        WeixinStateError::InjectedFailure {
            operation: "before_replace"
        }
    ));
    let state = store.load(ACCOUNT).expect("load").expect("state");
    assert!(state.cursors.is_empty());
    assert!(state.inbound_receipts.is_empty());
    assert!(state.pending_inbound.is_empty());
}

#[test]
fn inbound_batch_validation_rejects_raw_message_peer_and_pending_refs() {
    let (_temp, store) = store_fixture();
    store.save(&snapshot(1000)).expect("save state");
    let mut commit = WeixinInboundBatchCommit::new(ACCOUNT, 1100);
    commit.accepted.push(commit_item(
        "item#00000001",
        "raw-message-id",
        "peer#00000001",
    ));
    assert!(matches!(
        store.commit_inbound_batch(commit),
        Err(WeixinStateError::InvalidRecord { .. })
    ));

    let mut commit = WeixinInboundBatchCommit::new(ACCOUNT, 1200);
    commit.accepted.push(commit_item(
        "item#00000002",
        "message#00000002",
        "raw-peer-id",
    ));
    assert!(matches!(
        store.commit_inbound_batch(commit),
        Err(WeixinStateError::InvalidRecord { .. })
    ));

    let mut commit = WeixinInboundBatchCommit::new(ACCOUNT, 1300);
    let mut item = commit_item("item#00000003", "message#00000003", "peer#00000003");
    item.encrypted_payload_ref = "context-token-secret".to_string();
    commit.accepted.push(item);
    assert!(matches!(
        store.commit_inbound_batch(commit),
        Err(WeixinStateError::InvalidRecord { .. })
    ));
}

#[test]
fn inbound_batch_validation_rejects_missing_or_invalid_encrypted_payload() {
    let (_temp, store) = store_fixture();
    let mut state = snapshot(1000);
    state.pending_inbound.push(pending_item(None));
    assert!(matches!(
        store.save(&state),
        Err(WeixinStateError::InvalidRecord { .. })
    ));

    let invalid_payloads = [
        WeixinEncryptedPayload {
            algorithm: "aes-256-gcm".to_string(),
            ..encrypted_payload()
        },
        WeixinEncryptedPayload {
            algorithm_version: WEIXIN_PAYLOAD_ALGORITHM_VERSION + 1,
            ..encrypted_payload()
        },
        WeixinEncryptedPayload {
            aad_version: WEIXIN_PAYLOAD_AAD_VERSION + 1,
            ..encrypted_payload()
        },
        WeixinEncryptedPayload {
            nonce: "AAAA".to_string(),
            ..encrypted_payload()
        },
        WeixinEncryptedPayload {
            nonce: "not-base64@@".to_string(),
            ..encrypted_payload()
        },
        WeixinEncryptedPayload {
            ciphertext: "".to_string(),
            ..encrypted_payload()
        },
        WeixinEncryptedPayload {
            ciphertext: "not-base64@@".to_string(),
            ..encrypted_payload()
        },
        WeixinEncryptedPayload {
            ciphertext: "raw-message-body".to_string(),
            ..encrypted_payload()
        },
    ];
    for invalid_payload in invalid_payloads {
        let mut state = snapshot(1000);
        state
            .pending_inbound
            .push(pending_item(Some(invalid_payload)));
        assert!(matches!(
            store.save(&state),
            Err(WeixinStateError::InvalidRecord { .. })
        ));
    }
}

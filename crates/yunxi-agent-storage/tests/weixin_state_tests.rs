use std::fs;

use tempfile::TempDir;
use yunxi_agent_storage::{
    FileWeixinStateStore, WEIXIN_PAYLOAD_AAD_VERSION, WEIXIN_PAYLOAD_ALGORITHM,
    WEIXIN_PAYLOAD_ALGORITHM_VERSION, WEIXIN_STATE_SCHEMA_VERSION, WeixinAccountLockState,
    WeixinConnectionStateRecord, WeixinCredentialReferenceRecord, WeixinEncryptedPayload,
    WeixinInboundBatchCommit, WeixinInboundCommitItem, WeixinPairRequestCommitItem,
    WeixinPairRequestState, WeixinPendingInbound, WeixinPendingInboundState, WeixinStateError,
    WeixinStateSnapshot, WeixinStateStore, WeixinStateWriteOptions,
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
        encrypted_payload_ref: "pending#00000001".to_string(),
        payload_kind: Some("text".to_string()),
        encrypted_payload,
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
    assert_eq!(loaded.pending_inbound_count(), 0);

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
        encrypted_payload_ref: "pending#00000001".to_string(),
        payload_kind: Some("text".to_string()),
        encrypted_payload: Some(encrypted_payload()),
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

#[cfg(windows)]
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

#[cfg(windows)]
use tempfile::TempDir;
#[cfg(windows)]
use yunxi_agent_storage::{
    FileWeixinStateStore, WeixinConnectionStateRecord, WeixinCredentialReferenceRecord,
    WeixinInboundBatchCommit, WeixinInboundCommitItem, WeixinStateSnapshot, WeixinStateStore,
};
#[cfg(windows)]
use yunxi_agent_weixin::ilink::{MessageItem, TextItem, WeixinMessage};
#[cfg(windows)]
use yunxi_agent_weixin::{
    SecretString, SystemWeixinSecretStore, WeixinAccountId, WeixinInboundEnvelope,
    WeixinPayloadAad, WeixinPayloadCipher, WeixinSecretStore,
};

#[cfg(windows)]
const ACCOUNT_RAW: &str = "pending-restore-cross-process-account";
#[cfg(windows)]
const ACCOUNT: &str = "account#933b5bde";
#[cfg(windows)]
const WORKSPACE: &str = "workspace#73521066";
#[cfg(windows)]
const ENDPOINT: &str = "https://ilinkai.weixin.qq.com/";
#[cfg(windows)]
const DATA_KEY: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

#[cfg(windows)]
fn text_message() -> WeixinMessage {
    WeixinMessage {
        message_id: yunxi_agent_weixin::WeixinMessageId::new("raw-message-restart"),
        from_user_id: SecretString::new("raw-peer-restart"),
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
                text: SecretString::new("raw restart body"),
            }),
            is_completed: Some(true),
            msg_id: None,
        }],
        context_token: Some(SecretString::new("context-token-restart-secret")),
    }
}

#[cfg(windows)]
fn seed_encrypted_pending(workspace: &Path, lock_root: &Path) -> String {
    let state_store = FileWeixinStateStore::for_workspace_with_lock_root(workspace, lock_root);
    let mut snapshot = WeixinStateSnapshot::new(ACCOUNT, WORKSPACE, ENDPOINT, 1000);
    snapshot.connection_state = WeixinConnectionStateRecord::Ready;
    snapshot.credential = Some(WeixinCredentialReferenceRecord {
        backend: "windows-credential-manager".to_string(),
        token_target: "test-token-target".to_string(),
        data_key_target: "test-data-key-target".to_string(),
    });
    state_store.save(&snapshot).expect("save initial state");

    let message = text_message();
    let envelope = WeixinInboundEnvelope::from_message(ACCOUNT, None, &message, 1000);
    let item_id = envelope.pending_item_id();
    let payload = envelope
        .recoverable_text_payload(&message, &item_id)
        .expect("recoverable payload");
    let data_key = SecretString::new(DATA_KEY);
    let aad = WeixinPayloadAad::new(
        ACCOUNT,
        envelope.peer_id_hash.clone(),
        envelope.message_id_hash.clone(),
        item_id.clone(),
    );
    let encrypted_payload = WeixinPayloadCipher::new()
        .encrypt_pending_inbound(&data_key, &payload, &aad)
        .expect("encrypt pending payload");
    let encrypted_payload_ref = envelope.encrypted_payload_ref();
    let message_id_hash = envelope.message_id_hash.clone();
    let peer_id_hash = envelope.peer_id_hash.clone();
    let mut commit = WeixinInboundBatchCommit::new(ACCOUNT, 1100);
    commit.next_get_updates_buf = Some("cursor-restart".to_string());
    commit.accepted.push(WeixinInboundCommitItem {
        item_id: item_id.clone(),
        message_id_hash,
        peer_id_hash,
        encrypted_payload_ref,
        encrypted_payload,
        payload_kind: Some("text".to_string()),
    });
    state_store
        .commit_inbound_batch(commit)
        .expect("commit encrypted pending");
    item_id
}

#[cfg(windows)]
fn spawn_pending_decrypt_child(
    workspace: &Path,
    lock_root: &Path,
    item_id: &str,
    result_file: &Path,
) {
    let status = Command::new(env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("windows_pending_decrypt_child_recovers_payload")
        .arg("--ignored")
        .arg("--test-threads=1")
        .arg("--nocapture")
        .env("YUNXI_TEST_PENDING_WORKSPACE", workspace)
        .env("YUNXI_TEST_PENDING_LOCK_ROOT", lock_root)
        .env("YUNXI_TEST_PENDING_ITEM_ID", item_id)
        .env("YUNXI_TEST_PENDING_RESULT", result_file)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run pending decrypt child");
    assert!(status.success(), "pending decrypt child failed: {status:?}");
}

#[cfg(windows)]
#[test]
fn pending_inbound_decrypts_in_independent_process_from_system_secret_store() {
    let temp = TempDir::new().expect("temp");
    let workspace = temp.path().join("workspace");
    let lock_root = temp.path().join("locks");
    let result_file = temp.path().join("pending-decrypt-result.txt");
    let account = WeixinAccountId::new(ACCOUNT_RAW);
    let secret_store = SystemWeixinSecretStore::new();
    let _ = secret_store.delete_data_key(&account);
    secret_store
        .put_data_key(&account, &SecretString::new(DATA_KEY))
        .expect("write system data key");

    let item_id = seed_encrypted_pending(&workspace, &lock_root);
    let state_json = fs::read_to_string(
        FileWeixinStateStore::for_workspace_with_lock_root(&workspace, &lock_root)
            .state_path_for(ACCOUNT),
    )
    .expect("state json");
    for forbidden in [
        "raw-message-restart",
        "raw-peer-restart",
        "raw restart body",
        "context-token-restart-secret",
        DATA_KEY,
    ] {
        assert!(!state_json.contains(forbidden));
    }

    spawn_pending_decrypt_child(&workspace, &lock_root, &item_id, &result_file);
    let result = fs::read_to_string(&result_file).expect("result file");
    assert!(result.contains("payload_recovered=true"));
    assert!(result.contains(&item_id));
    assert!(!result.contains("raw restart body"));
    assert!(!result.contains(DATA_KEY));

    secret_store
        .delete_data_key(&account)
        .expect("cleanup system data key");
}

#[cfg(windows)]
#[test]
#[ignore]
fn windows_pending_decrypt_child_recovers_payload() {
    let workspace = env::var_os("YUNXI_TEST_PENDING_WORKSPACE")
        .map(PathBuf::from)
        .expect("pending workspace");
    let lock_root = env::var_os("YUNXI_TEST_PENDING_LOCK_ROOT")
        .map(PathBuf::from)
        .expect("pending lock root");
    let item_id = env::var("YUNXI_TEST_PENDING_ITEM_ID").expect("pending item id");
    let result_file = env::var_os("YUNXI_TEST_PENDING_RESULT")
        .map(PathBuf::from)
        .expect("pending result");
    let account = WeixinAccountId::new(ACCOUNT_RAW);
    let data_key = SystemWeixinSecretStore::new()
        .get_data_key(&account)
        .expect("read system data key in child");
    let state_store = FileWeixinStateStore::for_workspace_with_lock_root(&workspace, &lock_root);
    let pending = state_store
        .load_pending_inbound(ACCOUNT, &item_id)
        .expect("load pending")
        .expect("pending present");
    let payload = WeixinPayloadCipher::new()
        .decrypt_pending_inbound(&data_key, &pending)
        .expect("decrypt pending in child");
    assert_eq!(payload.item_id, item_id);
    assert_eq!(payload.account_id, ACCOUNT);
    assert!(payload.text.as_ref().is_some());
    fs::write(
        &result_file,
        format!(
            "payload_recovered=true item_id={} message_id_hash={} peer_id_hash={}",
            payload.item_id, payload.message_id_hash, payload.peer_id_hash
        ),
    )
    .expect("write child result");
    std::thread::sleep(Duration::from_millis(10));
}

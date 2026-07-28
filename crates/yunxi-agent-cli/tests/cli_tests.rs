use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use yunxi_agent_storage::{
    FileWeixinStateStore, WEIXIN_STATE_SCHEMA_VERSION, WeixinStateSnapshot, WeixinStateStore,
};
use yunxi_agent_weixin::{
    PRODUCTION_ILINK_ENDPOINT, WeixinAccountId, WeixinAccountRecord, WeixinAccountStore,
    WeixinCredentialReference,
};

const PRIVATE_WEIXIN_ACCOUNT: &str = "private-account-name";

fn write_legacy_weixin_metadata(workspace: &TempDir) -> (WeixinAccountId, String) {
    let account_id = WeixinAccountId::new(PRIVATE_WEIXIN_ACCOUNT);
    let credential = WeixinCredentialReference {
        backend: "fake-secure-store".to_string(),
        token_target: "target-token-ref".to_string(),
        data_key_target: "target-key-ref".to_string(),
    };
    let record = WeixinAccountRecord::new(&account_id, credential, workspace.path(), 1000)
        .expect("legacy metadata record");
    let account_hash = record.account_id.clone();
    WeixinAccountStore::new(workspace.path())
        .save(&account_id, &record)
        .expect("legacy metadata save");
    (account_id, account_hash)
}

fn spawn_sequence_http_server(responses: Vec<(u16, String)>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture server");
    listener
        .set_nonblocking(true)
        .expect("set fixture nonblocking");
    let address = listener.local_addr().expect("fixture address");
    thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        for (status, body) in responses {
            let (mut stream, _) = loop {
                match listener.accept() {
                    Ok(connection) => break connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "fixture request timed out");
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("fixture accept failed: {error}"),
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("fixture read timeout");
            let mut request = Vec::new();
            let mut buffer = [0_u8; 4096];
            loop {
                let count = match stream.read(&mut buffer) {
                    Ok(count) => count,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "fixture request body timed out");
                        thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                    Err(error) => panic!("read fixture request: {error}"),
                };
                if count == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..count]);
                let Some(header_end) = request.windows(4).position(|part| part == b"\r\n\r\n")
                else {
                    continue;
                };
                let headers = String::from_utf8_lossy(&request[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().expect("content length"))
                    })
                    .unwrap_or_default();
                if request.len() >= header_end + 4 + content_length {
                    break;
                }
            }
            let reason = if status == 200 { "OK" } else { "Bad Request" };
            let response = format!(
                "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream
                .write_all(response.as_bytes())
                .expect("write fixture response");
        }
    });
    format!("http://{address}")
}

fn assert_no_tui_bytes(label: &str, output: &std::process::Output) {
    for (stream_name, bytes) in [("stdout", &output.stdout), ("stderr", &output.stderr)] {
        let text = String::from_utf8_lossy(bytes);
        assert!(
            !bytes.contains(&0x1b),
            "{label} {stream_name} contained ANSI escape bytes: {text:?}"
        );
        for forbidden in ["Enter submit", "Enter confirm", "Alt+Enter newline"] {
            assert!(
                !text.contains(forbidden),
                "{label} {stream_name} leaked TUI footer text {forbidden:?}: {text:?}"
            );
        }
    }
}

#[test]
fn yunxi_primary_binary_prints_v2_version() {
    let mut cmd = Command::cargo_bin("yunxi").expect("binary should build");

    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("yunxi 2.1.6"));
}

#[test]
fn compatibility_binary_prints_v2_version() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");

    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("yunxi 2.1.6"));
}

#[test]
fn cli_weixin_help_covers_the_v216_weixin_command_surface() {
    for args in [
        vec!["weixin", "--help"],
        vec!["weixin", "login", "--help"],
        vec!["weixin", "status", "--help"],
        vec!["weixin", "doctor", "--help"],
        vec!["weixin", "serve", "--help"],
        vec!["weixin", "pair", "--help"],
        vec!["weixin", "pair", "list", "--help"],
        vec!["weixin", "pair", "approve", "--help"],
        vec!["weixin", "pair", "deny", "--help"],
        vec!["weixin", "logout", "--help"],
    ] {
        let mut cmd = Command::cargo_bin("yunxi").expect("binary should build");
        cmd.args(args).assert().success();
    }
}

#[test]
fn cli_weixin_status_doctor_and_pair_list_are_offline_and_secret_free() {
    let workspace = TempDir::new().expect("workspace");
    let cwd = workspace.path().to_str().expect("workspace path");
    for command in [
        vec![
            "--cwd",
            cwd,
            "--json",
            "weixin",
            "status",
            "--account",
            "private-account-name",
        ],
        vec![
            "--cwd",
            cwd,
            "--json",
            "weixin",
            "doctor",
            "--account",
            "private-account-name",
        ],
        vec![
            "--cwd",
            cwd,
            "--json",
            "weixin",
            "pair",
            "list",
            "--account",
            "private-account-name",
        ],
    ] {
        let mut cmd = Command::cargo_bin("yunxi").expect("binary should build");
        let output = cmd.args(command).output().expect("weixin JSON output");
        assert!(output.status.success());
        let value: Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
        assert_eq!(value["secrets_included"].as_bool(), Some(false));
        let text = String::from_utf8_lossy(&output.stdout);
        assert!(!text.contains("private-account-name"));
        assert!(!text.contains("bot_token"));
        assert!(!text.contains("context_token"));
    }
}

#[test]
fn cli_weixin_status_initializes_legacy_metadata_state_store_idempotently() {
    let workspace = TempDir::new().expect("workspace");
    let cwd = workspace.path().to_str().expect("workspace path");
    let (_account_id, account_hash) = write_legacy_weixin_metadata(&workspace);
    let store = FileWeixinStateStore::for_workspace(workspace.path());
    assert!(
        store
            .load(&account_hash)
            .expect("state read before migration")
            .is_none()
    );

    let mut status = Command::cargo_bin("yunxi").expect("binary should build");
    let output = status
        .args([
            "--cwd",
            cwd,
            "--json",
            "weixin",
            "status",
            "--account",
            PRIVATE_WEIXIN_ACCOUNT,
        ])
        .output()
        .expect("status json");
    assert!(output.status.success());
    let status_json: Value = serde_json::from_slice(&output.stdout).expect("status json");
    assert_eq!(status_json["account"].as_str(), Some(account_hash.as_str()));
    assert_eq!(status_json["state_store_configured"].as_bool(), Some(true));
    assert_eq!(
        status_json["state_store_schema_version"].as_u64(),
        Some(WEIXIN_STATE_SCHEMA_VERSION as u64)
    );
    assert_eq!(
        status_json["state_store_migration"].as_str(),
        Some("initialized_from_legacy_metadata")
    );
    assert_eq!(status_json["secrets_included"].as_bool(), Some(false));
    let status_text = String::from_utf8_lossy(&output.stdout);
    assert!(!status_text.contains(PRIVATE_WEIXIN_ACCOUNT));
    assert!(!status_text.contains("target-token-ref"));
    assert!(!status_text.contains("target-key-ref"));

    let state = store
        .load(&account_hash)
        .expect("state read after migration")
        .expect("state initialized");
    assert_eq!(state.account_id, account_hash);
    assert_eq!(state.endpoint, PRODUCTION_ILINK_ENDPOINT);
    assert_eq!(
        state.credential.as_ref().map(|c| c.backend.as_str()),
        Some("fake-secure-store")
    );

    let pair = store
        .add_pair_request(&account_hash, "peer#00000001", u64::MAX, 2000)
        .expect("pair request survives idempotent status");
    let mut status_again = Command::cargo_bin("yunxi").expect("binary should build");
    let second_output = status_again
        .args([
            "--cwd",
            cwd,
            "--json",
            "weixin",
            "status",
            "--account",
            PRIVATE_WEIXIN_ACCOUNT,
        ])
        .output()
        .expect("second status json");
    assert!(second_output.status.success());
    let second_json: Value = serde_json::from_slice(&second_output.stdout).expect("second status");
    assert_eq!(
        second_json["state_store_migration"].as_str(),
        Some("already_current")
    );
    assert_eq!(second_json["pair_request_count"].as_u64(), Some(1));
    let preserved = store
        .load(&account_hash)
        .expect("state read after second status")
        .expect("state remains");
    assert_eq!(preserved.pair_requests.len(), 1);
    assert_eq!(preserved.pair_requests[0].request_id, pair.request_id);
}

#[test]
fn cli_weixin_doctor_initializes_legacy_metadata_state_store() {
    let workspace = TempDir::new().expect("workspace");
    let cwd = workspace.path().to_str().expect("workspace path");
    let (_account_id, account_hash) = write_legacy_weixin_metadata(&workspace);

    let mut doctor = Command::cargo_bin("yunxi").expect("binary should build");
    let output = doctor
        .args([
            "--cwd",
            cwd,
            "--json",
            "weixin",
            "doctor",
            "--account",
            PRIVATE_WEIXIN_ACCOUNT,
        ])
        .output()
        .expect("doctor json");
    assert!(output.status.success());
    let doctor_json: Value = serde_json::from_slice(&output.stdout).expect("doctor json");
    assert_eq!(
        doctor_json["checks"]["state_store"].as_str(),
        Some("current")
    );
    assert_eq!(
        doctor_json["checks"]["state_store_migration"].as_str(),
        Some("initialized_from_legacy_metadata")
    );
    assert_eq!(
        doctor_json["checks"]["state_store_schema_current"].as_bool(),
        Some(true)
    );
    assert_eq!(
        doctor_json["state_store_schema_version"].as_u64(),
        Some(WEIXIN_STATE_SCHEMA_VERSION as u64)
    );
    assert_eq!(doctor_json["secrets_included"].as_bool(), Some(false));
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(!text.contains(PRIVATE_WEIXIN_ACCOUNT));
    assert!(!text.contains("target-token-ref"));
    assert!(!text.contains("target-key-ref"));

    assert!(
        FileWeixinStateStore::for_workspace(workspace.path())
            .load(&account_hash)
            .expect("state read")
            .is_some()
    );
}

#[test]
fn cli_weixin_status_refuses_future_state_schema_without_overwrite() {
    let workspace = TempDir::new().expect("workspace");
    let cwd = workspace.path().to_str().expect("workspace path");
    let (_account_id, account_hash) = write_legacy_weixin_metadata(&workspace);
    let store = FileWeixinStateStore::for_workspace(workspace.path());
    let state_path = store.state_path_for(&account_hash);
    fs::create_dir_all(state_path.parent().expect("state parent")).expect("state dir");
    fs::write(
        &state_path,
        r#"{"schema_version":999,"account_id":"account#future"}"#,
    )
    .expect("future state");

    let mut status = Command::cargo_bin("yunxi").expect("binary should build");
    status
        .args([
            "--cwd",
            cwd,
            "--json",
            "weixin",
            "status",
            "--account",
            PRIVATE_WEIXIN_ACCOUNT,
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("newer than supported"))
        .stderr(predicate::str::contains(PRIVATE_WEIXIN_ACCOUNT).not());
    let unchanged = fs::read_to_string(&state_path).expect("state unchanged");
    assert!(unchanged.contains("\"schema_version\":999"));
}

#[test]
fn cli_weixin_doctor_reports_damaged_metadata_without_state_creation() {
    let workspace = TempDir::new().expect("workspace");
    let cwd = workspace.path().to_str().expect("workspace path");
    let account_id = WeixinAccountId::new(PRIVATE_WEIXIN_ACCOUNT);
    let account_store = WeixinAccountStore::new(workspace.path());
    fs::create_dir_all(account_store.metadata_directory()).expect("metadata dir");
    fs::write(account_store.path_for(&account_id), "{not-json").expect("damaged metadata");

    let mut doctor = Command::cargo_bin("yunxi").expect("binary should build");
    let output = doctor
        .args([
            "--cwd",
            cwd,
            "--json",
            "weixin",
            "doctor",
            "--account",
            PRIVATE_WEIXIN_ACCOUNT,
        ])
        .output()
        .expect("doctor json");
    assert!(output.status.success());
    let doctor_json: Value = serde_json::from_slice(&output.stdout).expect("doctor json");
    assert_eq!(
        doctor_json["checks"]["account_metadata"].as_bool(),
        Some(false)
    );
    assert_eq!(
        doctor_json["checks"]["account_metadata_error"].as_str(),
        Some("metadata_invalid_json")
    );
    assert_eq!(
        doctor_json["checks"]["state_store_migration"].as_str(),
        Some("not_attempted_metadata_error")
    );
    assert_eq!(
        doctor_json["checks"]["state_store"].as_str(),
        Some("not_checked")
    );
    assert_eq!(doctor_json["secrets_included"].as_bool(), Some(false));
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(!text.contains(PRIVATE_WEIXIN_ACCOUNT));
    assert!(
        FileWeixinStateStore::for_workspace(workspace.path())
            .load(&account_id.to_string())
            .expect("state read")
            .is_none()
    );
}

#[test]
fn cli_weixin_pair_lifecycle_uses_state_store_without_secret_output() {
    let workspace = TempDir::new().expect("workspace");
    let cwd = workspace.path().to_str().expect("workspace path");

    let mut status = Command::cargo_bin("yunxi").expect("binary should build");
    let output = status
        .args([
            "--cwd",
            cwd,
            "--json",
            "weixin",
            "status",
            "--account",
            "private-account-name",
        ])
        .output()
        .expect("status json");
    assert!(output.status.success());
    let status_json: Value = serde_json::from_slice(&output.stdout).expect("status json");
    let account = status_json["account"]
        .as_str()
        .expect("redacted account")
        .to_string();

    let store = FileWeixinStateStore::for_workspace(workspace.path());
    let snapshot = WeixinStateSnapshot::new(
        &account,
        "workspace#00000001",
        "https://ilinkai.weixin.qq.com/",
        1000,
    );
    store.save(&snapshot).expect("save state");
    let approved_request = store
        .add_pair_request(&account, "peer#00000001", u64::MAX, 1100)
        .expect("pair request");

    let mut list = Command::cargo_bin("yunxi").expect("binary should build");
    let list_output = list
        .args([
            "--cwd",
            cwd,
            "--json",
            "weixin",
            "pair",
            "list",
            "--account",
            "private-account-name",
        ])
        .output()
        .expect("pair list json");
    assert!(list_output.status.success());
    let list_json: Value = serde_json::from_slice(&list_output.stdout).expect("list json");
    assert_eq!(list_json["pairs"].as_array().expect("pairs").len(), 1);
    let list_text = String::from_utf8_lossy(&list_output.stdout);
    assert!(!list_text.contains("private-account-name"));
    assert!(!list_text.contains("raw-peer"));
    assert_eq!(list_json["secrets_included"].as_bool(), Some(false));

    let mut approve = Command::cargo_bin("yunxi").expect("binary should build");
    let approve_output = approve
        .args([
            "--cwd",
            cwd,
            "--json",
            "weixin",
            "pair",
            "approve",
            &approved_request.request_id,
            "--account",
            "private-account-name",
        ])
        .output()
        .expect("pair approve json");
    assert!(approve_output.status.success());
    let approve_json: Value = serde_json::from_slice(&approve_output.stdout).expect("approve json");
    assert_eq!(approve_json["state"].as_str(), Some("approved"));
    assert_eq!(approve_json["secrets_included"].as_bool(), Some(false));

    let denied_request = store
        .add_pair_request(&account, "peer#00000002", u64::MAX, 1200)
        .expect("second pair request");
    let mut deny = Command::cargo_bin("yunxi").expect("binary should build");
    let deny_output = deny
        .args([
            "--cwd",
            cwd,
            "--json",
            "weixin",
            "pair",
            "deny",
            &denied_request.request_id,
            "--account",
            "private-account-name",
        ])
        .output()
        .expect("pair deny json");
    assert!(deny_output.status.success());
    let deny_json: Value = serde_json::from_slice(&deny_output.stdout).expect("deny json");
    assert_eq!(deny_json["state"].as_str(), Some("denied"));
    assert_eq!(deny_json["secrets_included"].as_bool(), Some(false));
}

#[test]
fn cli_weixin_logout_refuses_active_account_lock() {
    let workspace = TempDir::new().expect("workspace");
    let lock_root = TempDir::new().expect("lock root");
    let cwd = workspace.path().to_str().expect("workspace path");

    let mut status = Command::cargo_bin("yunxi").expect("binary should build");
    let output = status
        .args([
            "--cwd",
            cwd,
            "--json",
            "weixin",
            "status",
            "--account",
            "private-account-name",
        ])
        .output()
        .expect("status json");
    assert!(output.status.success());
    let status_json: Value = serde_json::from_slice(&output.stdout).expect("status json");
    let account = status_json["account"]
        .as_str()
        .expect("redacted account")
        .to_string();

    let store = FileWeixinStateStore::for_workspace_with_lock_root(
        workspace.path(),
        lock_root.path().to_path_buf(),
    );
    let _guard = store
        .try_acquire_account_lock(&account, "workspace#00000001")
        .expect("active account lock");

    let mut logout = Command::cargo_bin("yunxi").expect("binary should build");
    logout
        .env("YUNXI_WEIXIN_LOCK_ROOT", lock_root.path())
        .args([
            "--cwd",
            cwd,
            "weixin",
            "logout",
            "--account",
            "private-account-name",
            "--confirm",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("account lock"))
        .stderr(predicate::str::contains("private-account-name").not());
}

#[test]
fn cli_weixin_mutating_commands_fail_honestly_without_starting_runtime() {
    let workspace = TempDir::new().expect("workspace");
    let cwd = workspace.path().to_str().expect("workspace path");

    let mut serve = Command::cargo_bin("yunxi").expect("binary should build");
    serve
        .args([
            "--cwd",
            cwd,
            "--offline",
            "weixin",
            "serve",
            "--account",
            "private-account-name",
            "--workspace",
            cwd,
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "weixin serve requires login first",
        ))
        .stderr(predicate::str::contains("private-account-name").not());
    assert!(
        !workspace.path().join(".yunxi").join("sessions").exists(),
        "serve must not start Runtime or create YunXi sessions before login"
    );

    for args in [
        [
            "--cwd",
            cwd,
            "weixin",
            "pair",
            "approve",
            "pair-1",
            "--account",
            "private-account-name",
        ],
        [
            "--cwd",
            cwd,
            "weixin",
            "pair",
            "deny",
            "pair-1",
            "--account",
            "private-account-name",
        ],
    ] {
        let mut cmd = Command::cargo_bin("yunxi").expect("binary should build");
        cmd.args(args)
            .assert()
            .failure()
            .stderr(predicate::str::contains("weixin state does not exist"))
            .stderr(predicate::str::contains("private-account-name").not());
    }

    let mut json_login = Command::cargo_bin("yunxi").expect("binary should build");
    json_login
        .args([
            "--cwd",
            cwd,
            "--json",
            "weixin",
            "login",
            "--account",
            "private-account-name",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("requires interactive output"))
        .stderr(predicate::str::contains("private-account-name").not());

    let mut logout = Command::cargo_bin("yunxi").expect("binary should build");
    logout
        .args(["--cwd", cwd, "weixin", "logout"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("requires --confirm"));

    let mut confirmed_logout = Command::cargo_bin("yunxi").expect("binary should build");
    confirmed_logout
        .args([
            "--cwd",
            cwd,
            "weixin",
            "logout",
            "--account",
            "private-account-name",
            "--confirm",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("not configured"))
        .stdout(predicate::str::contains("private-account-name").not());

    let mut jsonl = Command::cargo_bin("yunxi").expect("binary should build");
    jsonl
        .args(["--cwd", cwd, "--jsonl", "weixin", "status"])
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            "use --json for weixin metadata commands",
        ));
}

#[test]
fn cli_companion_eval_emits_reproducible_json_and_jsonl_summaries() {
    let mut json = Command::cargo_bin("yunxi").expect("binary should build");
    let value = json
        .args(["--json", "eval", "companion"])
        .output()
        .expect("evaluation should run");
    assert!(value.status.success());
    let report: Value = serde_json::from_slice(&value.stdout).expect("JSON report");
    assert!(
        report["metrics"]["scenario_count"]
            .as_u64()
            .unwrap_or_default()
            >= 30
    );
    assert_eq!(
        report["metrics"]["tool_approval_bypass_count"].as_u64(),
        Some(0)
    );
    assert_eq!(report["metrics"]["failed_scenarios"].as_u64(), Some(0));
    assert_eq!(report["golden_passed"].as_bool(), Some(true));

    let mut jsonl = Command::cargo_bin("yunxi").expect("binary should build");
    let output = jsonl
        .args(["--jsonl", "eval", "companion"])
        .output()
        .expect("JSONL evaluation should run");
    assert!(output.status.success());
    assert_eq!(
        output
            .stdout
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .count(),
        1
    );
}

#[test]
fn cli_v2_general_companion_release_gate_is_local_safe_and_auditable() {
    let home = TempDir::new().expect("yunxi home");
    let workspace = TempDir::new().expect("workspace");
    let cwd = workspace.path().to_str().expect("workspace path");

    let controls =
        run_json_command_with_env(&home, &["--cwd", cwd, "--json", "controls", "status"]);
    assert_eq!(controls["companion_enabled"].as_bool(), Some(false));
    assert_eq!(controls["cloud_control_enabled"].as_bool(), Some(false));
    let scopes = controls["scopes"].as_array().expect("control scopes");
    assert_eq!(scopes.len(), 4);
    assert!(scopes.iter().any(|scope| {
        scope["scope"].as_str() == Some("relationship")
            && scope["source"].as_str() == Some("read_only_history")
            && scope["enabled"].is_null()
    }));

    let mut offline = Command::cargo_bin("yunxi").expect("binary should build");
    offline
        .env("YUNXI_HOME", home.path())
        .env_remove("YUNXI_PERSONA_ENABLED")
        .env_remove("YUNXI_MEMORY_ENABLED")
        .args([
            "--offline",
            "--no-tui",
            "--cwd",
            cwd,
            "run",
            "general companion release smoke",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("[offline]"))
        .stdout(predicate::str::contains("general companion release smoke"));

    let evaluation = run_json_command_with_env(&home, &["--json", "eval", "companion"]);
    assert_eq!(evaluation["harness_version"].as_str(), Some("2.0.6"));
    assert_eq!(evaluation["golden_passed"].as_bool(), Some(true));
    assert_eq!(evaluation["metrics"]["scenario_count"].as_u64(), Some(31));
    assert_eq!(
        evaluation["metrics"]["tool_approval_bypass_count"].as_u64(),
        Some(0)
    );
}

#[test]
fn cli_prints_dry_run_response() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    let temp = TempDir::new().expect("temp dir");

    cmd.args([
        "--offline",
        "--cwd",
        temp.path().to_str().expect("temp path"),
        "explain this project",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("[offline]"))
    .stdout(predicate::str::contains(
        "YunXi autonomous runtime accepted prompt: explain this project",
    ));
}

#[test]
fn cli_auto_offline_one_shot_plain_warns_before_offline_output() {
    let mut cmd = Command::cargo_bin("yunxi").expect("binary should build");
    let temp = TempDir::new().expect("temp dir");

    cmd.env_remove("YUNXI_PROVIDER_API_KEY")
        .env_remove("YUNXI_PROVIDER_API_KEY_ENV")
        .env_remove("YUNXI_PROVIDER_PROFILE")
        .env_remove("DEEPSEEK_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .args([
            "--cwd",
            temp.path().to_str().expect("temp path"),
            "auto offline",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "provider auto mode did not find live credentials",
        ))
        .stdout(predicate::str::contains("[offline]"));
}

#[test]
fn cli_auto_offline_json_keeps_stdout_structured_and_warns_on_stderr() {
    let mut cmd = Command::cargo_bin("yunxi").expect("binary should build");
    let temp = TempDir::new().expect("temp dir");

    let output = cmd
        .env_remove("YUNXI_PROVIDER_API_KEY")
        .env_remove("YUNXI_PROVIDER_API_KEY_ENV")
        .env_remove("YUNXI_PROVIDER_PROFILE")
        .env_remove("DEEPSEEK_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .args([
            "--cwd",
            temp.path().to_str().expect("temp path"),
            "--json",
            "auto offline json",
        ])
        .output()
        .expect("json output");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    let value: Value = serde_json::from_str(&stdout).expect("stdout should be JSON");
    assert_eq!(value["status"].as_str(), Some("completed"));
    assert!(stderr.contains("provider auto mode did not find live credentials"));
    assert!(!stdout.contains("[warning]"));
}

#[test]
fn cli_accepts_explicit_dry_run_backend() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");

    cmd.args(["--backend", "dry-run", "explain this project"])
        .assert()
        .success()
        .stdout(predicate::str::contains("provider auto mode did not find live credentials").not())
        .stdout(predicate::str::contains(
            "Dry run accepted prompt: explain this project",
        ));
}

#[test]
fn cli_accepts_explicit_yunxi_backend() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    let temp = TempDir::new().expect("temp dir");

    cmd.args([
        "--backend",
        "yunxi",
        "--offline",
        "--cwd",
        temp.path().to_str().expect("temp path"),
        "explain this project",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("[offline]"))
    .stdout(predicate::str::contains(
        "YunXi autonomous runtime accepted prompt: explain this project",
    ));
}

#[test]
fn cli_rejects_live_and_backend_flags_together() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");

    cmd.args(["--live", "--backend", "dry-run", "explain this project"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"))
        .stderr(predicate::str::contains("--live"))
        .stderr(predicate::str::contains("--backend"));
}

#[test]
fn cli_live_flag_selects_live_provider_without_codex_backend() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");

    cmd.env_remove("YUNXI_PROVIDER_API_KEY")
        .env_remove("YUNXI_PROVIDER_API_KEY_ENV")
        .env_remove("YUNXI_PROVIDER_PROFILE")
        .env_remove("DEEPSEEK_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .args(["--live", "explain this project"])
        .assert()
        .code(10)
        .stderr(predicate::str::contains("live provider"))
        .stderr(predicate::str::contains("credentials are not configured"))
        .stderr(predicate::str::contains("codex compatibility backend").not());
}

#[test]
fn cli_rejects_detached_codex_backend_at_parse_time() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");

    cmd.args(["--backend", "codex", "hello codex"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("invalid value"))
        .stderr(predicate::str::contains("codex"))
        .stderr(predicate::str::contains("provider auto mode").not());
}

#[test]
fn cli_enters_interactive_mode_without_prompt() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");

    cmd.arg("--offline")
        .write_stdin("/exit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "YunXi Agent v2.1.6 interactive CLI",
        ))
        .stdout(predicate::str::contains("provider_mode: offline"))
        .stdout(predicate::str::contains(
            "offline_runtime: static_provider (stage fixtures disabled by default)",
        ))
        .stdout(predicate::str::contains("YunXi interactive session ended."));
}

#[test]
fn cli_auto_fallback_warns_when_credentials_are_missing() {
    let mut cmd = Command::cargo_bin("yunxi").expect("binary should build");

    cmd.env_remove("YUNXI_PROVIDER_API_KEY")
        .env_remove("YUNXI_PROVIDER_API_KEY_ENV")
        .env_remove("YUNXI_PROVIDER_PROFILE")
        .env_remove("DEEPSEEK_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .write_stdin("/exit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("provider_source: auto_offline"))
        .stdout(predicate::str::contains("offline static runtime"))
        .stdout(predicate::str::contains("no model call"));
}

#[test]
fn interactive_provider_error_returns_to_repl_for_the_next_prompt() {
    let temp = TempDir::new().expect("temp dir");
    let cwd = temp.path().to_str().expect("temp path");
    let base_url = spawn_sequence_http_server(vec![
        (
            400,
            r#"{"error":{"message":"unknown field metadata"}}"#.to_string(),
        ),
        (
            400,
            r#"{"error":{"message":"request still rejected"}}"#.to_string(),
        ),
        (
            200,
            r#"{"choices":[{"message":{"role":"assistant","content":"SECOND_TURN_OK"}}]}"#
                .to_string(),
        ),
    ]);
    let mut cmd = Command::cargo_bin("yunxi").expect("binary should build");

    cmd.env_remove("YUNXI_PROVIDER_API_KEY")
        .env_remove("YUNXI_PROVIDER_API_KEY_ENV")
        .env_remove("OPENAI_API_KEY")
        .env("YUNXI_PROVIDER_PROFILE", "deepseek")
        .env("DEEPSEEK_API_KEY", "fixture-interactive-secret")
        .env("YUNXI_PROVIDER_BASE_URL", base_url)
        .env("YUNXI_PROVIDER_STREAM", "false")
        .args(["--cwd", cwd])
        .write_stdin("first prompt\n/session\nsecond prompt\n/exit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("session: new"))
        .stdout(predicate::str::contains("turns: 0"))
        .stdout(predicate::str::contains("SECOND_TURN_OK"))
        .stdout(predicate::str::contains("YunXi interactive session ended."))
        .stderr(predicate::str::contains("provider returned HTTP 400"))
        .stderr(predicate::str::contains("request still rejected"))
        .stdout(predicate::str::contains("fixture-interactive-secret").not())
        .stderr(predicate::str::contains("fixture-interactive-secret").not());

    let sessions = session_values(&temp);
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0]["prompt"].as_str(), Some("second prompt"));
}

#[test]
fn cli_auto_selects_deepseek_when_credentials_are_configured() {
    let mut cmd = Command::cargo_bin("yunxi").expect("binary should build");

    cmd.env_remove("YUNXI_PROVIDER_API_KEY")
        .env_remove("YUNXI_PROVIDER_API_KEY_ENV")
        .env_remove("YUNXI_PROVIDER_PROFILE")
        .env_remove("OPENAI_API_KEY")
        .env("DEEPSEEK_API_KEY", "fixture-deepseek-secret")
        .write_stdin("/session\n/exit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("provider_mode: live"))
        .stdout(predicate::str::contains("provider_source: auto_live"))
        .stdout(predicate::str::contains("provider: deepseek"))
        .stdout(predicate::str::contains("model: deepseek-v4-flash"));
}

#[test]
fn cli_offline_overrides_configured_deepseek_credentials() {
    let mut cmd = Command::cargo_bin("yunxi").expect("binary should build");

    cmd.env_remove("YUNXI_PROVIDER_API_KEY")
        .env_remove("YUNXI_PROVIDER_API_KEY_ENV")
        .env_remove("YUNXI_PROVIDER_PROFILE")
        .env("DEEPSEEK_API_KEY", "fixture-deepseek-secret")
        .arg("--offline")
        .write_stdin("/session\n/exit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("provider_mode: offline"))
        .stdout(predicate::str::contains("provider_source: forced_offline"))
        .stdout(predicate::str::contains("provider: offline"));
}

#[test]
fn cli_forced_live_rejects_missing_credentials_before_request() {
    let mut cmd = Command::cargo_bin("yunxi").expect("binary should build");

    cmd.env_remove("YUNXI_PROVIDER_API_KEY")
        .env_remove("YUNXI_PROVIDER_API_KEY_ENV")
        .env_remove("YUNXI_PROVIDER_PROFILE")
        .env_remove("DEEPSEEK_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .args(["--provider-live", "--provider", "deepseek", "hello"])
        .assert()
        .code(10)
        .stderr(predicate::str::contains(
            "live provider deepseek credentials are not configured",
        ))
        .stderr(predicate::str::contains("fixture-deepseek-secret").not());
}

#[test]
fn cli_rejects_provider_live_and_offline_together() {
    let mut cmd = Command::cargo_bin("yunxi").expect("binary should build");

    cmd.args(["--provider-live", "--offline", "hello"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"))
        .stderr(predicate::str::contains("--provider-live"))
        .stderr(predicate::str::contains("--offline"));
}

#[test]
fn cli_rejects_missing_prompt_for_jsonl() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");

    cmd.arg("--jsonl")
        .assert()
        .code(2)
        .stdout(predicate::str::contains("\"type\":\"error\""))
        .stdout(predicate::str::contains("a prompt is required"));
}

#[test]
fn yunxi_interactive_mode_runs_prompt_and_session_command() {
    let temp = TempDir::new().expect("temp dir");
    let cwd = temp.path().to_str().expect("temp path");
    let mut cmd = Command::cargo_bin("yunxi").expect("binary should build");

    cmd.args(["--backend", "yunxi", "--offline", "--cwd", cwd])
        .write_stdin("hello from repl\n/session\n/exit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "YunXi Agent v2.1.6 interactive CLI",
        ))
        .stdout(predicate::str::contains("[offline]"))
        .stdout(predicate::str::contains(
            "YunXi autonomous runtime accepted prompt: hello from repl",
        ))
        .stdout(predicate::str::contains("session: yunxi-"))
        .stdout(predicate::str::contains("turns: 1"))
        .stdout(predicate::str::contains("YunXi interactive session ended."));
}

#[test]
fn yunxi_no_tui_keeps_plain_interactive_mode() {
    let mut cmd = Command::cargo_bin("yunxi").expect("binary should build");

    cmd.args(["--offline", "--no-tui"])
        .write_stdin("/exit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "YunXi Agent v2.1.6 interactive CLI",
        ))
        .stdout(predicate::str::contains("YunXi interactive session ended."));
}

#[test]
fn cli_plain_pipe_ci_json_and_jsonl_paths_never_emit_tui_bytes() {
    let temp = TempDir::new().expect("temp dir");
    let cwd = temp.path().to_str().expect("temp path");

    let one_shot = Command::cargo_bin("yunxi")
        .expect("binary should build")
        .args(["--offline", "--cwd", cwd, "one shot"])
        .output()
        .expect("one-shot output");
    assert!(one_shot.status.success());
    assert_no_tui_bytes("one-shot", &one_shot);

    let json = Command::cargo_bin("yunxi")
        .expect("binary should build")
        .args(["--offline", "--cwd", cwd, "--json", "json output"])
        .output()
        .expect("json output");
    assert!(json.status.success());
    assert_no_tui_bytes("json", &json);
    serde_json::from_slice::<Value>(&json.stdout).expect("structured JSON stdout");

    let jsonl = Command::cargo_bin("yunxi")
        .expect("binary should build")
        .args(["--offline", "--cwd", cwd, "--jsonl", "jsonl output"])
        .output()
        .expect("jsonl output");
    assert!(jsonl.status.success());
    assert_no_tui_bytes("jsonl", &jsonl);
    for line in jsonl
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
    {
        serde_json::from_slice::<Value>(line).expect("each JSONL line is structured");
    }

    let ci = Command::cargo_bin("yunxi")
        .expect("binary should build")
        .env("CI", "1")
        .args(["--offline", "--cwd", cwd])
        .write_stdin("/exit\n")
        .output()
        .expect("CI interactive output");
    assert!(ci.status.success());
    assert_no_tui_bytes("CI", &ci);

    let forced_fallback = Command::cargo_bin("yunxi")
        .expect("binary should build")
        .env_remove("CI")
        .args(["--offline", "--cwd", cwd, "--tui"])
        .write_stdin("/exit\n")
        .output()
        .expect("forced TUI fallback output");
    assert!(forced_fallback.status.success());
    assert_no_tui_bytes("forced TUI fallback", &forced_fallback);
    assert!(String::from_utf8_lossy(&forced_fallback.stderr).contains("using plain mode"));
}

#[test]
fn cli_rejects_json_and_jsonl_together() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");

    cmd.args(["--offline", "--json", "--jsonl", "hello both"])
        .assert()
        .code(2)
        .stdout(predicate::str::contains("\"type\":\"error\""))
        .stdout(predicate::str::contains("cannot be used with"));
}

#[test]
fn cli_rejects_jsonl_for_metadata_subcommands() {
    let mut sessions = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    sessions
        .args(["sessions", "list", "--jsonl"])
        .assert()
        .code(2)
        .stdout(predicate::str::contains("\"type\":\"error\""))
        .stdout(predicate::str::contains("--jsonl is only supported"));

    let mut parity = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    parity
        .args(["parity", "map", "--jsonl"])
        .assert()
        .code(2)
        .stdout(predicate::str::contains("\"type\":\"error\""))
        .stdout(predicate::str::contains("--jsonl is only supported"));
}

#[test]
fn cli_metadata_help_marks_jsonl_as_agent_execution_only() {
    let mut sessions = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    sessions
        .args(["sessions", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("JSON Lines"))
        .stdout(predicate::str::contains("metadata commands reject"));

    let mut parity = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    parity
        .args(["parity", "map", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("JSON Lines"))
        .stdout(predicate::str::contains("metadata commands reject"));
}

#[test]
fn cli_persona_commands_manage_local_settings() {
    let home = TempDir::new().expect("yunxi home");
    let mut status = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");

    status
        .env("YUNXI_HOME", home.path())
        .env_remove("YUNXI_PERSONA_ENABLED")
        .env_remove("YUNXI_MEMORY_ENABLED")
        .args(["--json", "persona", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"persona_enabled\": true"))
        .stdout(predicate::str::contains("yunxi_companion_strong"));

    let mut off = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    off.env("YUNXI_HOME", home.path())
        .env_remove("YUNXI_PERSONA_ENABLED")
        .env_remove("YUNXI_MEMORY_ENABLED")
        .args(["persona", "off"])
        .assert()
        .success()
        .stdout(predicate::str::contains("persona_enabled: false"));

    let mut on = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    on.env("YUNXI_HOME", home.path())
        .env_remove("YUNXI_PERSONA_ENABLED")
        .env_remove("YUNXI_MEMORY_ENABLED")
        .args(["persona", "on"])
        .assert()
        .success()
        .stdout(predicate::str::contains("persona_enabled: true"));
}

#[test]
fn cli_controls_share_snapshot_and_persist_companion_switch() {
    let home = TempDir::new().expect("yunxi home");
    let workspace = TempDir::new().expect("workspace");
    let cwd = workspace.path().to_str().expect("workspace path");

    let initial = run_json_command_with_env(&home, &["--cwd", cwd, "--json", "controls", "status"]);
    assert_eq!(initial["companion_enabled"].as_bool(), Some(false));
    assert_eq!(initial["cloud_control_enabled"].as_bool(), Some(false));
    assert_eq!(initial["scopes"].as_array().map(Vec::len), Some(4));

    let enabled = run_json_command_with_env(
        &home,
        &["--cwd", cwd, "--json", "controls", "enable", "companion"],
    );
    assert_eq!(enabled["enabled"].as_bool(), Some(true));

    let persisted =
        run_json_command_with_env(&home, &["--cwd", cwd, "--json", "controls", "status"]);
    assert_eq!(persisted["companion_enabled"].as_bool(), Some(true));

    let audit = run_json_command_with_env(&home, &["--cwd", cwd, "--json", "controls", "audit"]);
    assert!(audit.as_array().is_some_and(|records| {
        records.iter().any(|record| {
            record["scope"].as_str() == Some("companion")
                && record["verb"].as_str() == Some("enable")
                && record["outcome"].as_str() == Some("completed")
        })
    }));
}

#[test]
fn cli_clear_controls_require_confirmation_and_keep_read_only_scopes_protected() {
    let home = TempDir::new().expect("yunxi home");
    let workspace = TempDir::new().expect("workspace");
    let cwd = workspace.path().to_str().expect("workspace path");

    let mut missing_confirmation =
        Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    missing_confirmation
        .env("YUNXI_HOME", home.path())
        .args(["--cwd", cwd, "controls", "clear", "memory"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("requires --confirm"));

    let mut read_only = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    read_only
        .env("YUNXI_HOME", home.path())
        .args([
            "--cwd",
            cwd,
            "controls",
            "clear",
            "relationship",
            "--confirm",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("read-only"));

    let audit = run_json_command_with_env(&home, &["--cwd", cwd, "--json", "controls", "audit"]);
    assert!(audit.as_array().is_some_and(|records| {
        records
            .iter()
            .filter(|record| record["verb"].as_str() == Some("clear"))
            .all(|record| record["outcome"].as_str() == Some("rejected"))
    }));
}

#[test]
fn cli_memory_commands_manage_runtime_extracted_memory() {
    let home = TempDir::new().expect("yunxi home");
    let workspace = TempDir::new().expect("workspace");
    let cwd = workspace.path().to_str().expect("workspace path");

    let mut enable = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    enable
        .env("YUNXI_HOME", home.path())
        .env_remove("YUNXI_PERSONA_ENABLED")
        .env_remove("YUNXI_MEMORY_ENABLED")
        .args(["--cwd", cwd, "memory", "on"])
        .assert()
        .success()
        .stdout(predicate::str::contains("YunXi memory is now enabled."))
        .stdout(predicate::str::contains("memory_enabled: true"));

    let mut run = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    run.env("YUNXI_HOME", home.path())
        .env_remove("YUNXI_PERSONA_ENABLED")
        .env_remove("YUNXI_MEMORY_ENABLED")
        .args(["--offline", "--cwd", cwd, "以后用中文回答"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "YunXi autonomous runtime accepted prompt",
        ));

    let mut run_again = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    run_again
        .env("YUNXI_HOME", home.path())
        .env_remove("YUNXI_PERSONA_ENABLED")
        .env_remove("YUNXI_MEMORY_ENABLED")
        .args(["--offline", "--cwd", cwd, "默认用中文交流"])
        .assert()
        .success();

    let list = run_json_command_with_env(
        &home,
        &["--cwd", cwd, "--json", "memory", "list", "--global"],
    );
    let records = list["records"].as_array().expect("memory records");
    let preferences = records
        .iter()
        .filter(|record| {
            record["kind"].as_str() == Some("preference")
                && record["status"].as_str() == Some("active")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        preferences.len(),
        1,
        "expected one active language preference memory after equivalent repeats: {list:#}"
    );
    assert_eq!(preferences[0]["revision"].as_u64(), Some(2));
    assert_eq!(preferences[0]["merged_count"].as_u64(), Some(2));

    let search = run_json_command_with_env(
        &home,
        &[
            "--cwd", cwd, "--json", "memory", "search", "中文", "--global",
        ],
    );
    assert!(
        search["records"]
            .as_array()
            .is_some_and(|records| !records.is_empty())
    );
}

#[test]
fn cli_rule_only_english_preference_writes_global_memory() {
    let home = TempDir::new().expect("yunxi home");
    let workspace = TempDir::new().expect("workspace");
    let cwd = workspace.path().to_str().expect("workspace path");

    run_json_command_with_env(&home, &["--cwd", cwd, "--json", "memory", "on"]);

    let mut run = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    run.env("YUNXI_HOME", home.path())
        .env_remove("YUNXI_PERSONA_ENABLED")
        .env_remove("YUNXI_MEMORY_ENABLED")
        .args([
            "--offline",
            "--memory-extraction",
            "rule-only",
            "--cwd",
            cwd,
            "以后请用英文回答",
        ])
        .assert()
        .success();

    let list = run_json_command_with_env(
        &home,
        &["--cwd", cwd, "--json", "memory", "list", "--global"],
    );
    let records = list["records"].as_array().expect("memory records");
    let english = records
        .iter()
        .find(|record| record["dedup_key"].as_str() == Some("global_user|preference|language:en"))
        .expect("English language preference should be saved");

    assert_eq!(english["kind"].as_str(), Some("preference"));
    assert_eq!(english["status"].as_str(), Some("active"));
    assert!(
        english["content"]
            .as_str()
            .is_some_and(|content| content.contains("英文"))
    );
}

#[test]
fn cli_language_change_creates_supersession_chain() {
    let home = TempDir::new().expect("yunxi home");
    let workspace = TempDir::new().expect("workspace");
    let cwd = workspace.path().to_str().expect("workspace path");

    run_json_command_with_env(&home, &["--cwd", cwd, "--json", "memory", "on"]);

    let mut chinese = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    chinese
        .env("YUNXI_HOME", home.path())
        .env_remove("YUNXI_PERSONA_ENABLED")
        .env_remove("YUNXI_MEMORY_ENABLED")
        .args([
            "--offline",
            "--memory-extraction",
            "rule-only",
            "--cwd",
            cwd,
            "以后请用中文回答",
        ])
        .assert()
        .success();

    let mut english = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    english
        .env("YUNXI_HOME", home.path())
        .env_remove("YUNXI_PERSONA_ENABLED")
        .env_remove("YUNXI_MEMORY_ENABLED")
        .args([
            "--offline",
            "--memory-extraction",
            "rule-only",
            "--cwd",
            cwd,
            "--jsonl",
            "以后请用英文回答",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"type\":\"memory_write\""))
        .stdout(predicate::str::contains(
            "\"action\":\"superseded_previous_fact\"",
        ))
        .stdout(predicate::str::contains(
            "\"merge_strategy\":\"supersession_chain\"",
        ));

    let pending = run_json_command_with_env(&home, &["--cwd", cwd, "--json", "memory", "pending"]);
    assert!(
        pending["records"]
            .as_array()
            .expect("pending records")
            .is_empty(),
        "clear active language change should not remain pending: {pending:#}"
    );

    let listed = run_json_command_with_env(
        &home,
        &["--cwd", cwd, "--json", "memory", "list", "--global"],
    );
    let records = listed["records"].as_array().expect("global records");
    let old = records
        .iter()
        .find(|record| record["dedup_key"] == "global_user|preference|language:zh")
        .expect("old Chinese preference retained");
    let new = records
        .iter()
        .find(|record| record["dedup_key"] == "global_user|preference|language:en")
        .expect("new English preference retained");
    assert_eq!(
        old["invalidation"]["superseded_by"].as_str(),
        new["id"].as_str()
    );
    let supersedes = new["invalidation"]["supersedes"]
        .as_array()
        .expect("supersedes array");
    assert_eq!(supersedes.len(), 1);
    assert_eq!(supersedes[0], old["id"]);
}

#[test]
fn cli_json_redacts_secret_prompt_while_memory_discards_candidate() {
    let home = TempDir::new().expect("yunxi home");
    let workspace = TempDir::new().expect("workspace");
    let cwd = workspace.path().to_str().expect("workspace path");
    let secret = format!("{}{}", "sk-", "f".repeat(32));
    let prompt = format!("my token is {secret}");

    run_json_command_with_env(&home, &["--cwd", cwd, "--json", "memory", "on"]);

    let mut run = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    let assert = run
        .env("YUNXI_HOME", home.path())
        .env_remove("YUNXI_PERSONA_ENABLED")
        .env_remove("YUNXI_MEMORY_ENABLED")
        .args([
            "--offline",
            "--memory-extraction",
            "rule-only",
            "--cwd",
            cwd,
            "--json",
            &prompt,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(&secret).not())
        .stdout(predicate::str::contains("[redacted]"))
        .stdout(predicate::str::contains("\"final_response\""))
        .stdout(predicate::str::contains("\"type\": \"started\""))
        .stdout(predicate::str::contains("\"type\": \"message\""))
        .stdout(predicate::str::contains("\"action\": \"discard\""));

    let output = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    let value: Value = serde_json::from_str(&output).expect("json output");
    let events = value["events"].as_array().expect("events");

    let final_response = value["final_response"]
        .as_str()
        .expect("final response should be present");
    assert!(!final_response.contains(&secret));
    assert!(final_response.contains("[redacted]"));

    let started = events
        .iter()
        .find(|event| event["type"].as_str() == Some("started"))
        .expect("started event");
    assert!(!started["prompt"].to_string().contains(&secret));
    assert!(
        started["prompt"]
            .as_str()
            .expect("prompt")
            .contains("[redacted]")
    );

    let message = events
        .iter()
        .find(|event| event["type"].as_str() == Some("message"))
        .expect("message event");
    assert!(!message["content"].to_string().contains(&secret));
    assert!(
        message["content"]
            .as_str()
            .expect("content")
            .contains("[redacted]")
    );

    let recall = events
        .iter()
        .find(|event| {
            event["type"].as_str() == Some("memoryRecall")
                && event["scope"].as_str() == Some("dynamic")
        })
        .expect("dynamic memory recall event");
    assert!(!recall["query"].to_string().contains(&secret));
    assert!(
        recall["query"]
            .as_str()
            .expect("query")
            .contains("[redacted-sensitive-query]")
    );

    let memory_write = events
        .iter()
        .find(|event| event["type"].as_str() == Some("memoryWrite"))
        .expect("memory write event");
    assert_eq!(memory_write["action"].as_str(), Some("discard"));

    let list = run_json_command_with_env(
        &home,
        &["--cwd", cwd, "--json", "memory", "list", "--global"],
    );
    let records = list["records"].as_array().expect("memory records");
    assert!(
        records
            .iter()
            .all(|record| !record.to_string().contains(&secret)),
        "secret-like prompt must not be persisted: {list:#}"
    );
}

#[test]
fn cli_json_preserves_non_secret_prompt_text() {
    let workspace = TempDir::new().expect("workspace");
    let cwd = workspace.path().to_str().expect("workspace path");
    let prompt = "please summarize visible non secret text";

    let mut run = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    let assert = run
        .args(["--offline", "--cwd", cwd, "--json", prompt])
        .assert()
        .success()
        .stdout(predicate::str::contains(prompt))
        .stdout(predicate::str::contains("[redacted]").not());

    let output = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    serde_json::from_str::<Value>(&output).expect("json output");
}

#[test]
fn cli_jsonl_redacts_secret_prompt_while_memory_discards_candidate() {
    let home = TempDir::new().expect("yunxi home");
    let workspace = TempDir::new().expect("workspace");
    let cwd = workspace.path().to_str().expect("workspace path");
    let secret = format!("{}{}", "sk-", "c".repeat(32));
    let prompt = format!("my token is {secret}");

    run_json_command_with_env(&home, &["--cwd", cwd, "--json", "memory", "on"]);

    let mut run = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    let assert = run
        .env("YUNXI_HOME", home.path())
        .env_remove("YUNXI_PERSONA_ENABLED")
        .env_remove("YUNXI_MEMORY_ENABLED")
        .args([
            "--offline",
            "--memory-extraction",
            "rule-only",
            "--cwd",
            cwd,
            "--jsonl",
            &prompt,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(&secret).not())
        .stdout(predicate::str::contains("[redacted]"))
        .stdout(predicate::str::contains("\"type\":\"memory_recall\""))
        .stdout(predicate::str::contains("[redacted-sensitive-query]"))
        .stdout(predicate::str::contains("\"type\":\"memory_write\""))
        .stdout(predicate::str::contains("\"action\":\"discard\""));

    let output = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    for line in output.lines() {
        serde_json::from_str::<serde_json::Value>(line).expect("each line is json");
    }

    let list = run_json_command_with_env(
        &home,
        &["--cwd", cwd, "--json", "memory", "list", "--global"],
    );
    let records = list["records"].as_array().expect("memory records");
    assert!(
        records
            .iter()
            .all(|record| !record.to_string().contains(&secret)),
        "secret-like prompt must not be persisted: {list:#}"
    );
}

#[test]
fn cli_jsonl_memory_recall_includes_diagnostics_without_memory_content() {
    let home = TempDir::new().expect("yunxi home");
    let workspace = TempDir::new().expect("workspace");
    let cwd = workspace.path().to_str().expect("workspace path");

    run_json_command_with_env(&home, &["--cwd", cwd, "--json", "memory", "on"]);

    let mut seed = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    seed.env("YUNXI_HOME", home.path())
        .env_remove("YUNXI_PERSONA_ENABLED")
        .env_remove("YUNXI_MEMORY_ENABLED")
        .args(["--offline", "--cwd", cwd, "以后请用中文回答"])
        .assert()
        .success();

    let output = Command::cargo_bin("yunxi-agent-cli")
        .expect("binary should build")
        .env("YUNXI_HOME", home.path())
        .env_remove("YUNXI_PERSONA_ENABLED")
        .env_remove("YUNXI_MEMORY_ENABLED")
        .args(["--offline", "--cwd", cwd, "--jsonl", "请计算 2+2"])
        .output()
        .expect("jsonl command should run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let recalls = stdout
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("jsonl line"))
        .filter(|value| value["type"].as_str() == Some("memory_recall"))
        .collect::<Vec<_>>();
    let boot = recalls
        .iter()
        .find(|value| value["scope"].as_str() == Some("boot"))
        .expect("boot memory_recall event");
    let dynamic = recalls
        .iter()
        .find(|value| value["scope"].as_str() == Some("dynamic"))
        .expect("dynamic memory_recall event");

    assert_eq!(boot["always_on_count"].as_u64(), Some(1));
    for recall in [boot, dynamic] {
        assert!(recall.get("dropped_unrelated").is_some());
        assert!(recall.get("dropped_by_budget").is_some());
        assert!(recall.get("dropped_duplicates").is_some());
    }
    assert!(!stdout.contains("用户偏好使用中文回答"));
}

#[test]
fn cli_memory_on_json_reports_first_enable_disclosure_fields() {
    let home = TempDir::new().expect("yunxi home");
    let workspace = TempDir::new().expect("workspace");
    let cwd = workspace.path().to_str().expect("workspace path");

    let value = run_json_command_with_env(&home, &["--cwd", cwd, "--json", "memory", "on"]);

    assert_eq!(value["memory_enabled"].as_bool(), Some(true));
    assert_eq!(value["first_enable_notice_shown"].as_bool(), Some(true));
    assert!(value["storage_roots"]["global"].as_str().is_some());
    assert!(value["storage_roots"]["workspace"].as_str().is_some());
    assert_eq!(
        value["provider_extraction"]["default_mode"].as_str(),
        Some("auto")
    );
    assert_eq!(
        value["pending_command"].as_str(),
        Some("yunxi memory pending")
    );
}

#[test]
fn cli_rejects_jsonl_for_persona_and_memory_management() {
    let mut persona = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    persona
        .args(["persona", "status", "--jsonl"])
        .assert()
        .code(2)
        .stdout(predicate::str::contains("\"type\":\"error\""))
        .stdout(predicate::str::contains("persona management commands"));

    let mut memory = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    memory
        .args(["memory", "status", "--jsonl"])
        .assert()
        .code(2)
        .stdout(predicate::str::contains("\"type\":\"error\""))
        .stdout(predicate::str::contains("memory management commands"));
}

#[test]
fn cli_run_subcommand_accepts_reserved_prompt_words() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    let temp = TempDir::new().expect("temp dir");

    cmd.args([
        "--cwd",
        temp.path().to_str().expect("temp path"),
        "--offline",
        "run",
        "sessions",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains(
        "YunXi autonomous runtime accepted prompt: sessions",
    ));
}

#[test]
fn cli_reserved_subcommand_prompt_error_suggests_escape() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    let temp = TempDir::new().expect("temp dir");
    let cwd = temp.path().to_str().expect("temp path");

    cmd.args(["--cwd", cwd, "--offline", "sessions"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("yunxi -- sessions"))
        .stderr(predicate::str::contains("yunxi run sessions"));

    let mut escaped = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    escaped
        .args(["--cwd", cwd, "--offline", "--", "sessions"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "YunXi autonomous runtime accepted prompt: sessions",
        ));
}

#[test]
fn yunxi_interactive_reports_tools_mcp_cost_and_status() {
    let temp = TempDir::new().expect("temp dir");
    let cwd = temp.path().to_str().expect("temp path");
    let mut cmd = Command::cargo_bin("yunxi").expect("binary should build");

    cmd.args(["--offline", "--cwd", cwd])
        .write_stdin("/tools\n/mcp\n/cost\n/status\n/exit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("tools: fixed="))
        .stdout(predicate::str::contains("[tool] shell"))
        .stdout(predicate::str::contains("mcp_servers: 0"))
        .stdout(predicate::str::contains(
            "mcp_status: no workspace MCP configured",
        ))
        .stdout(predicate::str::contains(
            "last_turn_usage: n/a - offline, no model call",
        ))
        .stdout(predicate::str::contains(
            "session_usage: n/a - offline, no model call",
        ))
        .stdout(predicate::str::contains("last_turn_status: none"))
        .stdout(predicate::str::contains("observed_events: 0"));
}

#[test]
fn cli_lists_and_shows_yunxi_sessions() {
    let temp = TempDir::new().expect("temp dir");
    let cwd = temp.path().to_str().expect("temp path");

    let mut run = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    run.args(["--offline", "--cwd", cwd, "remember this session"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "YunXi autonomous runtime accepted prompt: remember this session",
        ));

    let mut list = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    list.args(["--cwd", cwd, "sessions", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("remember this session"))
        .stdout(predicate::str::contains(cwd));

    let summary = run_json_command(&["--cwd", cwd, "--json", "sessions", "list"]);
    let summaries = summary.as_array().expect("session summaries");
    assert_eq!(summaries.len(), 1);
    assert_eq!(
        summaries[0]["prompt_preview"].as_str(),
        Some("remember this session")
    );
    assert!(summaries[0].get("events").is_none());
    assert!(
        summaries[0]["event_count"]
            .as_u64()
            .is_some_and(|count| count > 0)
    );

    let session_dir = temp.path().join(".yunxi").join("sessions");
    let session_file = fs::read_dir(&session_dir)
        .expect("session dir should exist")
        .map(|entry| entry.expect("session entry").path())
        .find(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .expect("session json should exist");
    let session_id = session_file
        .file_stem()
        .and_then(|value| value.to_str())
        .expect("session id")
        .to_string();

    let mut show = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    show.args(["--cwd", cwd, "--json", "sessions", "show", &session_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"prompt\""))
        .stdout(predicate::str::contains("remember this session"))
        .stdout(predicate::str::contains("\"events\""));

    let rollout = run_json_command(&["--cwd", cwd, "--json", "sessions", "rollout", &session_id]);
    assert_eq!(rollout["thread"]["id"].as_str(), Some(session_id.as_str()));
    assert_eq!(rollout["prompt"].as_str(), Some("remember this session"));
    assert!(
        rollout["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );

    let history = run_json_command(&["--cwd", cwd, "--json", "sessions", "history", &session_id]);
    assert_eq!(history["sessions"].as_array().map(Vec::len), Some(1));
    assert_eq!(history["items"].as_array().map(Vec::len), Some(2));
    assert_eq!(
        history["items"][0]["content"].as_str(),
        Some("remember this session")
    );
}

#[test]
fn cli_manages_yunxi_session_lifecycle() {
    let temp = TempDir::new().expect("temp dir");
    let cwd = temp.path().to_str().expect("temp path");

    let mut run = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    run.args(["--offline", "--cwd", cwd, "remember this lifecycle"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "YunXi autonomous runtime accepted prompt: remember this lifecycle",
        ));

    let session_id = first_session_id(&temp);

    let pinned = run_json_command(&["--cwd", cwd, "--json", "sessions", "pin", &session_id]);
    assert_eq!(pinned["pinned"], true);

    let archived = run_json_command(&["--cwd", cwd, "--json", "sessions", "archive", &session_id]);
    assert_eq!(archived["archived"], true);

    let forked = run_json_command(&["--cwd", cwd, "--json", "sessions", "fork", &session_id]);
    assert_ne!(forked["id"].as_str(), Some(session_id.as_str()));
    assert_eq!(forked["parent_id"].as_str(), Some(session_id.as_str()));
    assert_eq!(forked["prompt"].as_str(), Some("remember this lifecycle"));

    let mut resume = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    resume
        .args([
            "--offline",
            "--cwd",
            cwd,
            "sessions",
            "resume",
            &session_id,
            "continue",
            "the",
            "work",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "YunXi autonomous runtime accepted prompt",
        ));

    let sessions = session_values(&temp);
    let resumed = sessions
        .iter()
        .find(|session| {
            session["parent_id"].as_str() == Some(session_id.as_str())
                && session["prompt"]
                    .as_str()
                    .is_some_and(|prompt| prompt.contains("continue the work"))
        })
        .expect("resumed child session should be persisted");
    assert!(resumed["prompt"].as_str().is_some_and(|prompt| {
        prompt == "continue the work" && !prompt.contains("Previous prompt")
    }));

    let resumed_id = resumed["id"].as_str().expect("resumed id");
    let history = run_json_command(&["--cwd", cwd, "--json", "sessions", "history", resumed_id]);
    let history_items = history["items"].as_array().expect("history items");
    assert!(
        history_items
            .iter()
            .any(|item| { item["content"].as_str() == Some("remember this lifecycle") })
    );
    assert!(
        history_items
            .iter()
            .any(|item| { item["content"].as_str() == Some("continue the work") })
    );
    assert!(sessions.iter().any(|session| {
        session["parent_id"].as_str() == Some(session_id.as_str())
            && session["prompt"]
                .as_str()
                .is_some_and(|prompt| prompt.contains("continue the work"))
    }));

    let graph = run_json_command(&["--cwd", cwd, "--json", "sessions", "graph"]);
    assert!(graph["sessions"].get(&session_id).is_some());
    assert!(
        graph["children"][&session_id]
            .as_array()
            .is_some_and(|children| children.len() >= 2)
    );
}

#[test]
fn cli_auto_offline_resume_plain_warns_before_offline_output() {
    let temp = TempDir::new().expect("temp dir");
    let cwd = temp.path().to_str().expect("temp path");

    let mut run = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    run.args(["--offline", "--cwd", cwd, "remember auto resume"])
        .assert()
        .success();
    let session_id = first_session_id(&temp);

    let mut resume = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    resume
        .env_remove("YUNXI_PROVIDER_API_KEY")
        .env_remove("YUNXI_PROVIDER_API_KEY_ENV")
        .env_remove("YUNXI_PROVIDER_PROFILE")
        .env_remove("DEEPSEEK_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .args(["--cwd", cwd, "sessions", "resume", &session_id, "continue"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "provider auto mode did not find live credentials",
        ))
        .stdout(predicate::str::contains("[offline]"));
}

#[test]
fn cli_prints_codex_core_parity_map() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");

    cmd.args(["parity", "map"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Codex Core Agent Parity Map"))
        .stdout(predicate::str::contains("yunxi-agent-protocol"))
        .stdout(predicate::str::contains(
            "vendor/codex-rs/core/src/codex_thread.rs",
        ));
}

fn first_session_id(temp: &TempDir) -> String {
    session_json_files(temp)
        .first()
        .and_then(|path| path.file_stem())
        .and_then(|value| value.to_str())
        .expect("session id")
        .to_string()
}

fn session_values(temp: &TempDir) -> Vec<Value> {
    session_json_files(temp)
        .into_iter()
        .map(|path| {
            let content = fs::read_to_string(&path).expect("session file should be readable");
            serde_json::from_str(&content).expect("session file should contain JSON")
        })
        .collect()
}

fn session_json_files(temp: &TempDir) -> Vec<PathBuf> {
    let session_dir = temp.path().join(".yunxi").join("sessions");
    let mut files = fs::read_dir(&session_dir)
        .expect("session dir should exist")
        .map(|entry| entry.expect("session entry").path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    files.sort();
    files
}

fn run_json_command(args: &[&str]) -> Value {
    let output = Command::cargo_bin("yunxi-agent-cli")
        .expect("binary should build")
        .args(args)
        .output()
        .expect("command should run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("stdout should contain JSON")
}

fn run_json_command_with_env(home: &TempDir, args: &[&str]) -> Value {
    let output = Command::cargo_bin("yunxi-agent-cli")
        .expect("binary should build")
        .env("YUNXI_HOME", home.path())
        .env_remove("YUNXI_PERSONA_ENABLED")
        .env_remove("YUNXI_MEMORY_ENABLED")
        .args(args)
        .output()
        .expect("command should run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("stdout should contain JSON")
}

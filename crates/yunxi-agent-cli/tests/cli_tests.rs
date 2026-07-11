use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn yunxi_primary_binary_prints_v1_version() {
    let mut cmd = Command::cargo_bin("yunxi").expect("binary should build");

    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("yunxi 1.1.0"));
}

#[test]
fn compatibility_binary_prints_v1_version() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");

    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("yunxi 1.1.0"));
}

#[test]
fn cli_prints_dry_run_response() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    let temp = TempDir::new().expect("temp dir");

    cmd.args([
        "--cwd",
        temp.path().to_str().expect("temp path"),
        "explain this project",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains(
        "YunXi autonomous runtime accepted prompt: explain this project",
    ));
}

#[test]
fn cli_accepts_explicit_dry_run_backend() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");

    cmd.args(["--backend", "dry-run", "explain this project"])
        .assert()
        .success()
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
        "--cwd",
        temp.path().to_str().expect("temp path"),
        "explain this project",
    ])
    .assert()
    .success()
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
fn cli_live_backend_reports_feature_message_without_codex_native_feature() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");

    cmd.args(["--live", "explain this project"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "codex compatibility backend is detached from the default CLI",
        ));
}

#[test]
fn cli_enters_interactive_mode_without_prompt() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");

    cmd.write_stdin("/exit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("YunXi Agent v1.1 interactive CLI"))
        .stdout(predicate::str::contains("YunXi interactive session ended."));
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

    cmd.args(["--backend", "yunxi", "--cwd", cwd])
        .write_stdin("hello from repl\n/session\n/exit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("YunXi Agent v1.1 interactive CLI"))
        .stdout(predicate::str::contains(
            "YunXi autonomous runtime accepted prompt: hello from repl",
        ))
        .stdout(predicate::str::contains("session: yunxi-"))
        .stdout(predicate::str::contains("turns: 1"))
        .stdout(predicate::str::contains("YunXi interactive session ended."));
}

#[test]
fn cli_lists_and_shows_yunxi_sessions() {
    let temp = TempDir::new().expect("temp dir");
    let cwd = temp.path().to_str().expect("temp path");

    let mut run = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    run.args(["--cwd", cwd, "remember this session"])
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
    run.args(["--cwd", cwd, "remember this lifecycle"])
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

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

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
fn cli_rejects_missing_prompt() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("a prompt is required"));
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

use assert_cmd::Command;
use predicates::prelude::*;
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

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn yunxi_jsonl_prints_one_json_event_per_line() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    let temp = TempDir::new().expect("temp dir");

    let assert = cmd
        .args([
            "--backend",
            "yunxi",
            "--cwd",
            temp.path().to_str().expect("temp path"),
            "--jsonl",
            "jsonl yunxi run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"type\":\"thread_started\""))
        .stdout(predicate::str::contains("\"type\":\"turn_started\""))
        .stdout(predicate::str::contains("\"type\":\"item\""))
        .stdout(predicate::str::contains("\"type\":\"turn_completed\""));

    let output = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    for line in output.lines() {
        serde_json::from_str::<serde_json::Value>(line).expect("each line is json");
    }
}

#[test]
fn dry_run_jsonl_prints_one_json_event_per_line() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");

    let assert = cmd
        .args(["--backend", "dry-run", "--jsonl", "jsonl dry run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"type\":\"thread_started\""))
        .stdout(predicate::str::contains("\"type\":\"turn_started\""))
        .stdout(predicate::str::contains("\"type\":\"item\""))
        .stdout(predicate::str::contains("\"type\":\"turn_completed\""));

    let output = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    for line in output.lines() {
        serde_json::from_str::<serde_json::Value>(line).expect("each line is json");
    }
}

#[test]
fn yunxi_jsonl_prints_child_agent_fixture_events() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    let temp = TempDir::new().expect("temp dir");

    let assert = cmd
        .args([
            "--backend",
            "yunxi",
            "--cwd",
            temp.path().to_str().expect("temp path"),
            "--jsonl",
            "run stage 4j child runtime fixture",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"type\":\"child_agent\""))
        .stdout(predicate::str::contains(
            "\"child_session_id\":\"agent-1-session\"",
        ))
        .stdout(predicate::str::contains("\"type\":\"storage_state\""));

    let output = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    let mut saw_child = false;
    for line in output.lines() {
        let value = serde_json::from_str::<serde_json::Value>(line).expect("each line is json");
        if value.get("type").and_then(serde_json::Value::as_str) == Some("child_agent") {
            saw_child = true;
        }
    }
    assert!(saw_child);
}

#[test]
fn stage_4k_jsonl_fixtures_emit_new_core_events() {
    let temp = TempDir::new().expect("temp dir");
    let prompt_expectations = [
        (
            "run stage 4k child provider fixture",
            "\"type\":\"child_agent\"",
        ),
        ("run stage 4k sandbox fixture", "Sandbox runner: platform="),
        ("run stage 4k mcp reuse fixture", "\"type\":\"mcp_session\""),
        (
            "run stage 4k child scoped stream fixture",
            "\"type\":\"child_scoped_stream\"",
        ),
    ];

    for (prompt, expected) in prompt_expectations {
        let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
        let assert = cmd
            .args([
                "--backend",
                "yunxi",
                "--cwd",
                temp.path().to_str().expect("temp path"),
                "--jsonl",
                prompt,
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains(expected));

        let output = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
        for line in output.lines() {
            serde_json::from_str::<serde_json::Value>(line).expect("each line is json");
        }
    }
}

#[test]
fn stage_4k_cancellation_fixture_emits_cancelled_jsonl() {
    let temp = TempDir::new().expect("temp dir");
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");

    let assert = cmd
        .args([
            "--backend",
            "yunxi",
            "--cwd",
            temp.path().to_str().expect("temp path"),
            "--jsonl",
            "run stage 4k cancellation fixture",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"type\":\"cancelled\""))
        .stdout(predicate::str::contains("\"type\":\"child_scoped_stream\""));

    let output = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    for line in output.lines() {
        serde_json::from_str::<serde_json::Value>(line).expect("each line is json");
    }
}

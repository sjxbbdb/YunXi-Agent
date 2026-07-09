use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn yunxi_jsonl_prints_one_json_event_per_line() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");

    let assert = cmd
        .args(["--backend", "yunxi", "--jsonl", "jsonl yunxi run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"type\":\"started\""))
        .stdout(predicate::str::contains("\"type\":\"message\""))
        .stdout(predicate::str::contains("\"type\":\"completed\""));

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
        .stdout(predicate::str::contains("\"type\":\"started\""))
        .stdout(predicate::str::contains("\"type\":\"message\""))
        .stdout(predicate::str::contains("\"type\":\"completed\""));

    let output = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    for line in output.lines() {
        serde_json::from_str::<serde_json::Value>(line).expect("each line is json");
    }
}

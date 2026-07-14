use assert_cmd::Command;
use predicates::prelude::*;
use std::collections::BTreeSet;
use tempfile::TempDir;

#[test]
fn yunxi_jsonl_prints_one_json_event_per_line() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    let temp = TempDir::new().expect("temp dir");

    let assert = cmd
        .args([
            "--backend",
            "yunxi",
            "--offline",
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
    let mut final_message_count = 0usize;
    for line in output.lines() {
        let value = serde_json::from_str::<serde_json::Value>(line).expect("each line is json");
        if value.get("type").and_then(serde_json::Value::as_str) == Some("item")
            && value
                .get("item")
                .and_then(|item| item.get("type"))
                .and_then(serde_json::Value::as_str)
                == Some("message")
            && value
                .get("item")
                .and_then(|item| item.get("role"))
                .and_then(serde_json::Value::as_str)
                == Some("assistant")
            && value
                .get("item")
                .and_then(|item| item.get("content"))
                .and_then(serde_json::Value::as_str)
                == Some("YunXi autonomous runtime accepted prompt: jsonl yunxi run")
        {
            final_message_count += 1;
        }
    }
    assert_eq!(final_message_count, 1);
}

#[test]
fn auto_offline_jsonl_prints_structured_warning_event() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    let temp = TempDir::new().expect("temp dir");

    let assert = cmd
        .env_remove("YUNXI_PROVIDER_API_KEY")
        .env_remove("YUNXI_PROVIDER_API_KEY_ENV")
        .env_remove("YUNXI_PROVIDER_PROFILE")
        .env_remove("DEEPSEEK_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .args([
            "--backend",
            "yunxi",
            "--cwd",
            temp.path().to_str().expect("temp path"),
            "--jsonl",
            "jsonl auto offline",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"type\":\"error\""))
        .stdout(predicate::str::contains(
            "provider auto mode did not find live credentials",
        ))
        .stdout(predicate::str::contains("[warning]").not());

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
        .stdout(predicate::str::contains("provider auto mode did not find live credentials").not())
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
fn yunxi_jsonl_redacts_secret_like_prompt_from_transcript_items() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    let temp = TempDir::new().expect("temp dir");
    let secret = format!("{}{}", "sk-", "b".repeat(32));
    let prompt = format!("my token is {secret}");

    let assert = cmd
        .args([
            "--backend",
            "yunxi",
            "--offline",
            "--cwd",
            temp.path().to_str().expect("temp path"),
            "--jsonl",
            &prompt,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"type\":\"item\""))
        .stdout(predicate::str::contains("[redacted]"))
        .stdout(predicate::str::contains(&secret).not());

    let output = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    let mut saw_user = false;
    let mut saw_assistant = false;
    for line in output.lines() {
        let value = serde_json::from_str::<serde_json::Value>(line).expect("each line is json");
        if value.get("type").and_then(serde_json::Value::as_str) == Some("item")
            && value
                .get("item")
                .and_then(|item| item.get("type"))
                .and_then(serde_json::Value::as_str)
                == Some("message")
        {
            let role = value
                .get("item")
                .and_then(|item| item.get("role"))
                .and_then(serde_json::Value::as_str);
            let content = value
                .get("item")
                .and_then(|item| item.get("content"))
                .and_then(serde_json::Value::as_str)
                .expect("message content");
            assert!(!content.contains(&secret));
            if role == Some("user") {
                saw_user = true;
                assert!(content.contains("[redacted]"));
            }
            if role == Some("assistant") {
                saw_assistant = true;
                assert!(content.contains("[redacted]"));
                assert!(content.contains("YunXi autonomous runtime accepted prompt"));
            }
        }
    }
    assert!(saw_user, "expected redacted user item in JSONL output");
    assert!(
        saw_assistant,
        "expected redacted assistant item in JSONL output"
    );
}

#[test]
fn yunxi_jsonl_prints_child_agent_fixture_events() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    let temp = TempDir::new().expect("temp dir");

    let assert = cmd
        .env("YUNXI_RUNTIME_FIXTURES", "1")
        .args([
            "--backend",
            "yunxi",
            "--offline",
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
fn stage_fixture_prompt_is_plain_prompt_without_explicit_fixture_mode() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    let temp = TempDir::new().expect("temp dir");

    let assert = cmd
        .env_remove("YUNXI_RUNTIME_FIXTURES")
        .args([
            "--backend",
            "yunxi",
            "--offline",
            "--cwd",
            temp.path().to_str().expect("temp path"),
            "--jsonl",
            "run stage 4m real parity fixture",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"type\":\"deep_parity_state\"").not())
        .stdout(predicate::str::contains(
            "YunXi autonomous runtime accepted prompt",
        ));

    let output = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    for line in output.lines() {
        serde_json::from_str::<serde_json::Value>(line).expect("each line is json");
    }
}

#[test]
fn stage_4k_jsonl_fixtures_emit_new_core_events() {
    let temp = TempDir::new().expect("temp dir");
    let prompt_expectations = [
        (
            "run stage 4k child provider fixture",
            "\"type\":\"child_agent\"",
        ),
        (
            "run stage 4k sandbox fixture",
            "Sandbox runner: schema_version=1",
        ),
        ("run stage 4k mcp reuse fixture", "\"type\":\"mcp_session\""),
        (
            "run stage 4k child scoped stream fixture",
            "\"type\":\"child_scoped_stream\"",
        ),
    ];

    for (prompt, expected) in prompt_expectations {
        let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
        let assert = cmd
            .env("YUNXI_RUNTIME_FIXTURES", "1")
            .args([
                "--backend",
                "yunxi",
                "--offline",
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
        .env("YUNXI_RUNTIME_FIXTURES", "1")
        .args([
            "--backend",
            "yunxi",
            "--offline",
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

#[test]
fn stage_4l_deep_parity_fixture_emits_full_jsonl_shape() {
    let temp = TempDir::new().expect("temp dir");
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");

    let assert = cmd
        .env("YUNXI_RUNTIME_FIXTURES", "1")
        .args([
            "--backend",
            "yunxi",
            "--offline",
            "--cwd",
            temp.path().to_str().expect("temp path"),
            "--jsonl",
            "run stage 4l deep parity fixture",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"type\":\"thread_state\""))
        .stdout(predicate::str::contains("\"type\":\"turn_metadata\""))
        .stdout(predicate::str::contains("\"type\":\"turn_state\""))
        .stdout(predicate::str::contains("\"type\":\"deep_parity_state\""))
        .stdout(predicate::str::contains("\"layer\":\"12_parity_harness\""))
        .stdout(predicate::str::contains("\"type\":\"child_scoped_stream\""))
        .stdout(predicate::str::contains("\"type\":\"storage_state\""));

    let output = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    let mut deep_parity_events = 0usize;
    for line in output.lines() {
        let value = serde_json::from_str::<serde_json::Value>(line).expect("each line is json");
        if value.get("type").and_then(serde_json::Value::as_str) == Some("deep_parity_state") {
            deep_parity_events += 1;
        }
    }
    assert_eq!(deep_parity_events, 12);
}

#[test]
fn stage_4m_real_parity_fixture_emits_real_runtime_jsonl_shape() {
    let temp = TempDir::new().expect("temp dir");
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");

    let assert = cmd
        .env("YUNXI_RUNTIME_FIXTURES", "1")
        .args([
            "--backend",
            "yunxi",
            "--offline",
            "--cwd",
            temp.path().to_str().expect("temp path"),
            "--jsonl",
            "run stage 4m real parity fixture",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"type\":\"thread_state\""))
        .stdout(predicate::str::contains("\"type\":\"turn_state\""))
        .stdout(predicate::str::contains("\"type\":\"sandbox_attempt\""))
        .stdout(predicate::str::contains(
            "\"type\":\"approval_cache_state\"",
        ))
        .stdout(predicate::str::contains("\"type\":\"mcp_session\""))
        .stdout(predicate::str::contains("\"type\":\"child_scoped_stream\""))
        .stdout(predicate::str::contains(
            "\"layer\":\"12_real_parity_harness\"",
        ));

    let output = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    let mut deep_parity_events = 0usize;
    let mut reused_approval = false;
    let mut saw_sandbox_schema = false;
    let mut started = BTreeSet::new();
    let mut completed = BTreeSet::new();
    for line in output.lines() {
        let value = serde_json::from_str::<serde_json::Value>(line).expect("each line is json");
        match value.get("type").and_then(serde_json::Value::as_str) {
            Some("deep_parity_state") => deep_parity_events += 1,
            Some("tool_started") => {
                if let Some(id) = value
                    .get("call")
                    .and_then(|call| call.get("id"))
                    .and_then(serde_json::Value::as_str)
                {
                    started.insert(id.to_string());
                }
            }
            Some("tool_completed") => {
                if let Some(id) = value.get("call_id").and_then(serde_json::Value::as_str) {
                    completed.insert(id.to_string());
                }
            }
            Some("approval_cache_state") => {
                reused_approval |= value
                    .get("reused")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
            }
            Some("sandbox_attempt") => {
                assert_eq!(
                    value
                        .get("schema_version")
                        .and_then(serde_json::Value::as_u64),
                    Some(1)
                );
                assert!(
                    value
                        .get("backend_id")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|backend_id| !backend_id.is_empty())
                );
                assert!(
                    value
                        .get("backend_label")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|backend_label| !backend_label.is_empty())
                );
                assert_eq!(
                    value
                        .get("os_isolation")
                        .and_then(serde_json::Value::as_bool),
                    Some(false)
                );
                let enforcement = value
                    .get("enforcement")
                    .and_then(serde_json::Value::as_str)
                    .expect("sandbox enforcement");
                assert_eq!(
                    value
                        .get("enforcement_level")
                        .and_then(serde_json::Value::as_str),
                    Some(enforcement)
                );
                assert!(
                    value
                        .get("runner")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|runner| !runner.is_empty())
                );
                saw_sandbox_schema |= value
                    .get("enforcement_level")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|level| {
                        matches!(level, "policy_only" | "process_lifecycle" | "policy_bypass")
                    });
            }
            _ => {}
        }
    }
    assert_eq!(deep_parity_events, 12);
    assert!(reused_approval);
    assert!(saw_sandbox_schema);
    for id in &started {
        assert!(completed.contains(id), "missing tool_completed for {id}");
    }
    assert!(completed.contains("stage-4m-mcp-1"));
    assert!(completed.contains("stage-4m-mcp-2"));
}

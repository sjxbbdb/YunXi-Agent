# YunXi Agent v1.8.6 JSON Output Redaction Development Report

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the v1.8.5 audit finding that `--json` one-shot agent output still exposes secret-like prompt text, and close the remaining `RuntimeEvent::ThreadState.data` JSONL sanitizer gap.

**Architecture:** Treat every machine-readable agent execution output as a redaction boundary in `yunxi-agent-cli` before serialization. Reuse `yunxi_agent_provider::redact_sensitive_text` and redact structured Rust values (`AgentRunResult`, `AgentEvent`, `RuntimeEvent`) before calling `serde_json`, instead of replacing strings after JSON rendering. Keep memory write policy unchanged: sensitive candidates must still be discarded rather than persisted.

**Tech Stack:** Rust 2024, existing YunXi workspace crates, `yunxi_agent_core::AgentRunResult`, `yunxi_agent_core::AgentEvent`, `yunxi_agent_protocol::RuntimeEvent`, `yunxi_agent_provider::redact_sensitive_text`, `serde_json`, `assert_cmd`, `predicates`, `tempfile`.

## Global Constraints

- Base version: `1.8.5`; next version is `1.8.6`.
- Source audit input: `C:\Users\admin\Desktop\YunXi Agent调研项目\2026-07-14-YunXi-Agent-1.8.5-源码审核报告.md`.
- Long-term design baseline: `C:\Users\admin\Desktop\YunXi Agent调研项目\2026-07-13-YunXi-Agent-人格与长期记忆模块开源调研.md`.
- Primary P1 issue: `--json` currently serializes raw `AgentRunResult`, leaking secret-like prompt text through `final_response`, `events[].Started.prompt`, and `events[].Message.content`.
- Secondary P2 issue: `RuntimeEvent::ThreadState.data` is not redacted by the JSONL sanitizer.
- `--json` and `--jsonl` agent execution output must default to redaction; do not add a new opt-in flag.
- Do not use whole-pretty-JSON string replacement. Redact typed data before serialization.
- Do not treat memory discard policy as output redaction; both safeguards must remain independently testable.
- Do not print or commit real API keys, tokens, passwords, private files, or user private memory content.
- Tests may construct fake secret-like strings at runtime from fragments such as `"sk-"` plus repeated characters; do not hard-code full key-shaped literals that trip repository secret scans.
- Keep persona/memory local, inspectable, user-controlled, and removable.
- Do not add SQLite, vector search, graph memory, relationship state machines, external memory runtimes, new TUI pages, cloud tasks, marketplace surfaces, or SDK packaging in this version.
- Keep first-stage work focused on `yunxi-agent-core`, `yunxi-agent-cli`, and documentation needed to build, test, and publish this project.
- Use CodeGraph before grep/find/read when locating or understanding code because `.codegraph/` exists at the repository root.
- Follow Rust 2024 and existing module patterns; run `cargo fmt` after Rust edits.
- Follow the established release workflow: run unified verification after construction, update the desktop development log, refresh install, clean build artifacts, and publish through the project's current GitHub REST API process if release publishing is requested.

---

## Audit Summary

The v1.8.5 audit accepts the JSONL transcript redaction work. User message items, offline assistant echo, memory recall query, nested stream/tool payloads, provider/error texts, and memory discard behavior were verified for `--jsonl`.

The remaining blocker is the parallel `--json` output path:

- `crates/yunxi-agent-cli/src/main.rs:708` redacts `RuntimeEvent` values only in the `jsonl` branch.
- `crates/yunxi-agent-cli/src/main.rs:713` still prints `serde_json::to_string_pretty(&result)?` in the `json` branch.
- `AgentRunResult` still contains raw prompt-derived text at serialization time.

The audit's dynamic fake-secret reproduction produced:

```json
{
  "leaked": true,
  "contains_final_response": true,
  "contains_started_prompt_field": true,
  "contains_assistant_echo": true,
  "contains_redacted": false,
  "contains_discard": true
}
```

This means storage policy is still discarding secret-like memory candidates, but the `--json` machine-readable stdout boundary is not yet safe.

## Current Code Boundary

- `crates/yunxi-agent-cli/src/main.rs:702`
  - `print_run_result` owns the final stdout branch for plain text, `--json`, and `--jsonl`.
- `crates/yunxi-agent-cli/src/main.rs:708`
  - The `--jsonl` branch maps `AgentEvent` values into protocol `RuntimeEvent` values, then calls `jsonl_redaction::redact_runtime_event_for_jsonl`.
- `crates/yunxi-agent-cli/src/main.rs:713`
  - The `--json` branch directly serializes `AgentRunResult`.
- `crates/yunxi-agent-cli/src/jsonl_redaction.rs:9`
  - Current redaction helper redacts `RuntimeEvent` and nested protocol payloads.
- `crates/yunxi-agent-core/src/event.rs:8`
  - `AgentEvent` is the typed event schema inside `AgentRunResult`.
- `crates/yunxi-agent-core/src/event.rs:368`
  - `AgentRunResult` contains `status`, `final_response`, and `events`.
- `crates/yunxi-agent-protocol/src/lib.rs:236`
  - Protocol `ThreadState.data` is a `BTreeMap<String, String>` and should be treated like `TurnState.data`.

## File Structure

- Modify: `crates/yunxi-agent-cli/src/jsonl_redaction.rs`
  - Extend this module from "JSONL-only" into the shared CLI machine-readable output redaction boundary.
  - Add `redact_agent_run_result_for_json(result: AgentRunResult) -> AgentRunResult`.
  - Add `redact_agent_event_for_json(event: AgentEvent) -> AgentEvent`.
  - Add `RuntimeEvent::ThreadState` handling in `redact_runtime_event_for_jsonl`.
  - Add unit tests for `AgentRunResult` redaction and `ThreadState.data` JSONL redaction.
- Modify: `crates/yunxi-agent-cli/src/main.rs`
  - Route the `--json` branch through `jsonl_redaction::redact_agent_run_result_for_json` before `serde_json::to_string_pretty`.
- Modify: `crates/yunxi-agent-cli/tests/cli_tests.rs`
  - Add a black-box `--json` regression test matching the existing `--jsonl` privacy test shape.
- Modify: `Cargo.toml`
  - Promote workspace version to `1.8.6`.
- Modify: `Cargo.lock`
  - Promote YunXi workspace crate versions to `1.8.6`.
- Modify: `crates/yunxi-agent-cli/src/main.rs`
  - Update CLI version and versioned error/help copy from `1.8.5` to `1.8.6`.
- Modify: `crates/yunxi-agent-cli/src/render.rs`
  - Update interactive CLI banner/version display if it references `1.8.5`.
- Modify: `crates/yunxi-agent-cli/tests/cli_tests.rs`
  - Update version/banner expectations.
- Modify: `crates/yunxi-agent-tui/src/app.rs`
  - Update TUI displayed version to `v1.8.6`.
- Modify: `crates/yunxi-agent-tui/src/render.rs`
  - Update TUI test expectations if any snapshot/string checks mention version.
- Modify: `crates/yunxi-agent-persona/src/profile.rs`
  - Update persona profile version marker to `1.8.6`.
- Modify: `crates/yunxi-agent-persona/src/compiler.rs`
  - Update version comments or injected persona version markers to `v1.8.6`.
- Modify: `docs/persona-memory.md`
  - Document that both JSON and JSONL agent execution outputs are sanitized independently from memory write policy.
- Modify: `docs/extraction-status.md`
  - Add a v1.8.6 section with the construction scope and final verification evidence.
- Create: `docs/reports/2026-07-14-yunxi-agent-v1-8-6-json-output-redaction-development-report.md`
  - This report.
- Update after verification: `C:\Users\admin\Desktop\YunXi Agent开发日志.md`
  - Record source changes, verification commands, release result, and remaining audit notes.

## Non-Goals

- Do not change memory extraction classification, pending/active routing, or secret discard policy.
- Do not suppress whole `--json` or JSONL event objects; redact only sensitive fragments so automation can still consume the structure.
- Do not redact ordinary non-secret prompt text.
- Do not introduce a second secret detector.
- Do not move protocol schema types or core event types just to solve this CLI output boundary.
- Do not perform screenshot-level TUI visual work for this privacy-only slice unless a regression appears while testing.

---

### Task 1: Add Structured `AgentRunResult` Redaction For `--json`

**Files:**

- Modify: `crates/yunxi-agent-cli/src/jsonl_redaction.rs`

**Interfaces:**

- Consumes: `yunxi_agent_core::AgentRunResult`
- Produces: `pub(crate) fn redact_agent_run_result_for_json(result: AgentRunResult) -> AgentRunResult`
- Produces: redacted `AgentEvent` values before pretty JSON serialization.

- [ ] **Step 1: Add core event imports**

Modify the import block in `crates/yunxi-agent-cli/src/jsonl_redaction.rs`:

```rust
use yunxi_agent_core::{AgentEvent, AgentRunResult, TodoStatus};
```

Keep the existing protocol imports because JSONL redaction still uses `RuntimeEvent`, `StreamEvent`, `ResponseItem`, `ResponseItemDelta`, `ToolCall`, `FunctionCallOutput`, and `ContentItem`.

- [ ] **Step 2: Add the JSON result facade**

Add this public facade near `redact_runtime_event_for_jsonl`:

```rust
pub(crate) fn redact_agent_run_result_for_json(mut result: AgentRunResult) -> AgentRunResult {
    result.final_response = result.final_response.map(redact_text);
    result.events = result
        .events
        .into_iter()
        .map(redact_agent_event_for_json)
        .collect();
    result
}
```

- [ ] **Step 3: Add typed `AgentEvent` redaction**

Add this helper in the same file. It must redact every text-bearing `AgentEvent` field that can plausibly carry user prompt, model text, tool text, provider text, memory query/reason, child-agent text, or diagnostic data.

```rust
fn redact_agent_event_for_json(event: AgentEvent) -> AgentEvent {
    match event {
        AgentEvent::Started { prompt } => AgentEvent::Started {
            prompt: redact_text(prompt),
        },
        AgentEvent::ThreadState { mut state } => {
            state.data = redact_string_map(state.data);
            AgentEvent::ThreadState { state }
        }
        AgentEvent::TurnMetadata { mut metadata } => {
            metadata.data = redact_string_map(metadata.data);
            AgentEvent::TurnMetadata { metadata }
        }
        AgentEvent::TurnState { mut state } => {
            state.data = redact_string_map(state.data);
            AgentEvent::TurnState { state }
        }
        AgentEvent::DeepParityState {
            layer,
            status,
            message,
            data,
        } => AgentEvent::DeepParityState {
            layer,
            status,
            message: message.map(redact_text),
            data: redact_string_map(data),
        },
        AgentEvent::SandboxAttempt {
            id,
            schema_version,
            platform,
            status,
            backend,
            backend_id,
            backend_label,
            os_isolation,
            enforcement,
            enforcement_level,
            runner,
            unsupported_reason,
            command,
            cwd,
            message,
        } => AgentEvent::SandboxAttempt {
            id,
            schema_version,
            platform,
            status,
            backend,
            backend_id,
            backend_label,
            os_isolation,
            enforcement,
            enforcement_level,
            runner,
            unsupported_reason: unsupported_reason.map(redact_text),
            command: command.map(redact_text),
            cwd,
            message: message.map(redact_text),
        },
        AgentEvent::ApprovalCacheState {
            session_id,
            tool_name,
            key,
            decision,
            reused,
        } => AgentEvent::ApprovalCacheState {
            session_id,
            tool_name,
            key: redact_text(key),
            decision,
            reused,
        },
        AgentEvent::MemoryRecall {
            schema_version,
            enabled,
            scope,
            query,
            count,
            budget_used_chars,
            truncated,
            always_on_count,
            dropped_unrelated,
            dropped_by_budget,
            dropped_duplicates,
        } => AgentEvent::MemoryRecall {
            schema_version,
            enabled,
            scope,
            query: redact_text(query),
            count,
            budget_used_chars,
            truncated,
            always_on_count,
            dropped_unrelated,
            dropped_by_budget,
            dropped_duplicates,
        },
        AgentEvent::MemoryCandidate {
            schema_version,
            id,
            kind,
            sensitivity,
            status,
            write_policy,
            reason,
        } => AgentEvent::MemoryCandidate {
            schema_version,
            id,
            kind,
            sensitivity,
            status,
            write_policy,
            reason: redact_text(reason),
        },
        AgentEvent::MemoryWarning {
            schema_version,
            warning,
        } => AgentEvent::MemoryWarning {
            schema_version,
            warning: redact_text(warning),
        },
        AgentEvent::Message { content } => AgentEvent::Message {
            content: redact_text(content),
        },
        AgentEvent::Reasoning { content } => AgentEvent::Reasoning {
            content: redact_text(content),
        },
        AgentEvent::CommandStarted { id, command } => AgentEvent::CommandStarted {
            id,
            command: redact_text(command),
        },
        AgentEvent::CommandUpdated {
            id,
            command,
            aggregated_output,
        } => AgentEvent::CommandUpdated {
            id,
            command: redact_text(command),
            aggregated_output: redact_text(aggregated_output),
        },
        AgentEvent::CommandCompleted {
            id,
            command,
            aggregated_output,
            exit_code,
            status,
        } => AgentEvent::CommandCompleted {
            id,
            command: redact_text(command),
            aggregated_output: redact_text(aggregated_output),
            exit_code,
            status,
        },
        AgentEvent::CommandFinished { command, exit_code } => AgentEvent::CommandFinished {
            command: redact_text(command),
            exit_code,
        },
        AgentEvent::ToolCallStarted {
            id,
            name,
            arguments_json,
        } => AgentEvent::ToolCallStarted {
            id,
            name,
            arguments_json: arguments_json.map(redact_text),
        },
        AgentEvent::ToolCallCompleted {
            id,
            name,
            output,
            status,
        } => AgentEvent::ToolCallCompleted {
            id,
            name,
            output: redact_text(output),
            status,
        },
        AgentEvent::ApprovalRequested {
            id,
            tool_name,
            reason,
        } => AgentEvent::ApprovalRequested {
            id,
            tool_name,
            reason: redact_text(reason),
        },
        AgentEvent::ApprovalCompleted {
            id,
            approved,
            reason,
        } => AgentEvent::ApprovalCompleted {
            id,
            approved,
            reason: reason.map(redact_text),
        },
        AgentEvent::EscalationRequested {
            id,
            tool_name,
            reason,
            required_sandbox,
            required_network,
        } => AgentEvent::EscalationRequested {
            id,
            tool_name,
            reason: redact_text(reason),
            required_sandbox,
            required_network,
        },
        AgentEvent::EscalationCompleted {
            id,
            approved,
            reason,
        } => AgentEvent::EscalationCompleted {
            id,
            approved,
            reason: reason.map(redact_text),
        },
        AgentEvent::McpSession {
            server,
            status,
            message,
        } => AgentEvent::McpSession {
            server,
            status,
            message: message.map(redact_text),
        },
        AgentEvent::MultiAgentEvent {
            agent_id,
            parent_agent_id,
            status,
            message,
        } => AgentEvent::MultiAgentEvent {
            agent_id,
            parent_agent_id,
            status,
            message: message.map(redact_text),
        },
        AgentEvent::ChildAgentEvent {
            agent_id,
            child_session_id,
            parent_session_id,
            status,
            message,
        } => AgentEvent::ChildAgentEvent {
            agent_id,
            child_session_id,
            parent_session_id,
            status,
            message: message.map(redact_text),
        },
        AgentEvent::ChildScopedStream {
            agent_id,
            child_session_id,
            parent_session_id,
            event,
            seq,
            message,
        } => AgentEvent::ChildScopedStream {
            agent_id,
            child_session_id,
            parent_session_id,
            event: redact_text(event),
            seq,
            message: message.map(redact_text),
        },
        AgentEvent::TodoUpdated { id, items } => AgentEvent::TodoUpdated {
            id,
            items: items.into_iter().map(redact_todo_status).collect(),
        },
        AgentEvent::Warning { message } => AgentEvent::Warning {
            message: redact_text(message),
        },
        AgentEvent::Cancelled { reason } => AgentEvent::Cancelled {
            reason: reason.map(redact_text),
        },
        AgentEvent::ProviderError {
            provider,
            status,
            classification,
            message,
        } => AgentEvent::ProviderError {
            provider,
            status,
            classification,
            message: redact_text(message),
        },
        AgentEvent::Error { message } => AgentEvent::Error {
            message: redact_text(message),
        },
        other => other,
    }
}

fn redact_todo_status(mut item: TodoStatus) -> TodoStatus {
    item.text = redact_text(item.text);
    item
}
```

- [ ] **Step 4: Add unit coverage for `AgentRunResult` redaction**

Extend the existing `#[cfg(test)] mod tests` imports:

```rust
use std::collections::BTreeMap;
use yunxi_agent_core::{
    AgentEvent, AgentRunResult, AgentRunStatus, CommandStatus, ThreadRuntimeState, TodoStatus,
};
```

Change the `super` import:

```rust
use super::{redact_agent_run_result_for_json, redact_runtime_event_for_jsonl};
```

Append this test:

```rust
#[test]
fn redacts_agent_run_result_for_json() {
    let secret = fake_secret();
    let mut thread_data = BTreeMap::new();
    thread_data.insert("safe".to_string(), "visible-safe".to_string());
    thread_data.insert("token".to_string(), format!("thread saw {secret}"));

    let result = AgentRunResult {
        status: AgentRunStatus::Completed,
        final_response: Some(format!(
            "YunXi autonomous runtime accepted prompt: my token is {secret}"
        )),
        events: vec![
            AgentEvent::Started {
                prompt: format!("my token is {secret}"),
            },
            AgentEvent::Message {
                content: format!("assistant echo {secret}"),
            },
            AgentEvent::MemoryRecall {
                schema_version: 1,
                enabled: true,
                scope: "global".to_string(),
                query: format!("lookup {secret}"),
                count: 0,
                budget_used_chars: 0,
                truncated: false,
                always_on_count: 0,
                dropped_unrelated: 0,
                dropped_by_budget: 0,
                dropped_duplicates: 0,
            },
            AgentEvent::CommandCompleted {
                id: Some("cmd".to_string()),
                command: format!("echo {secret}"),
                aggregated_output: format!("tool printed {secret}"),
                exit_code: Some(0),
                status: CommandStatus::Completed,
            },
            AgentEvent::ThreadState {
                state: ThreadRuntimeState {
                    thread_id: "thread".to_string(),
                    session_id: Some("session".to_string()),
                    parent_thread_id: None,
                    status: "running".to_string(),
                    cwd: "D:/YunXi Agent".to_string(),
                    resume_source: None,
                    child_depth: 0,
                    data: thread_data,
                },
            },
            AgentEvent::TodoUpdated {
                id: Some("todo".to_string()),
                items: vec![TodoStatus {
                    text: format!("check {secret}"),
                    completed: false,
                }],
            },
        ],
    };

    let redacted = redact_agent_run_result_for_json(result);
    let json = serde_json::to_string(&redacted).expect("json");

    assert!(!json.contains(&secret));
    assert!(json.contains("[redacted]"));
    assert!(json.contains("visible-safe"));
    assert!(json.contains("YunXi autonomous runtime accepted prompt"));
}
```

---

### Task 2: Wire `--json` Through The Redaction Boundary

**Files:**

- Modify: `crates/yunxi-agent-cli/src/main.rs`
- Modify: `crates/yunxi-agent-cli/tests/cli_tests.rs`

**Interfaces:**

- Consumes: `print_run_result(result, json, jsonl, offline_label)`
- Produces: redacted pretty JSON for agent execution when `--json` is selected.

- [ ] **Step 1: Redact before pretty JSON serialization**

Change `print_run_result`:

```rust
} else if json {
    let result = jsonl_redaction::redact_agent_run_result_for_json(result);
    println!("{}", serde_json::to_string_pretty(&result)?);
} else if let Some(final_response) = result.final_response {
```

This consumes `result` only in the `json` branch. Plain text behavior remains unchanged for this slice.

- [ ] **Step 2: Add the black-box CLI regression**

Append to `crates/yunxi-agent-cli/tests/cli_tests.rs` near `cli_jsonl_redacts_secret_prompt_while_memory_discards_candidate`:

```rust
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
    assert!(started["prompt"].as_str().expect("prompt").contains("[redacted]"));

    let message = events
        .iter()
        .find(|event| event["type"].as_str() == Some("message"))
        .expect("message event");
    assert!(!message["content"].to_string().contains(&secret));
    assert!(message["content"].as_str().expect("content").contains("[redacted]"));

    let recall = events
        .iter()
        .find(|event| event["type"].as_str() == Some("memoryRecall"))
        .expect("memory recall event");
    assert!(!recall["query"].to_string().contains(&secret));
    assert!(recall["query"].as_str().expect("query").contains("[redacted]"));

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
```

- [ ] **Step 3: Preserve ordinary non-secret JSON behavior**

Keep existing `--json` tests for non-secret prompts passing. If no existing test explicitly checks non-secret prompt visibility in agent execution JSON, add this compact test:

```rust
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
```

---

### Task 3: Add `RuntimeEvent::ThreadState.data` JSONL Redaction

**Files:**

- Modify: `crates/yunxi-agent-cli/src/jsonl_redaction.rs`

**Interfaces:**

- Consumes: `yunxi_agent_protocol::RuntimeEvent::ThreadState`
- Produces: redacted `ThreadState.data` values before JSONL serialization.

- [ ] **Step 1: Add the JSONL match branch**

Insert this branch before `RuntimeEvent::TurnMetadata`:

```rust
RuntimeEvent::ThreadState {
    thread_id,
    mut state,
} => {
    state.data = redact_string_map(state.data);
    RuntimeEvent::ThreadState { thread_id, state }
}
```

- [ ] **Step 2: Add unit coverage**

Extend the existing protocol imports inside `#[cfg(test)] mod tests`:

```rust
use yunxi_agent_protocol::{
    ContentItem, FunctionCallOutput, MessagePhase, ProtocolRole, ResponseItem,
    ResponseItemDelta, RuntimeEvent, StreamEvent, ThreadId, ThreadState, ToolCall,
    ToolCallStatus, TurnId,
};
```

Append this test:

```rust
#[test]
fn redacts_thread_state_data_for_jsonl() {
    let secret = fake_secret();
    let mut data = BTreeMap::new();
    data.insert("safe".to_string(), "visible-safe".to_string());
    data.insert("token".to_string(), format!("thread data contains {secret}"));

    let event = RuntimeEvent::ThreadState {
        thread_id: ThreadId("thread".to_string()),
        state: ThreadState {
            thread_id: ThreadId("thread".to_string()),
            session_id: Some("session".to_string()),
            parent_thread_id: None,
            status: "running".to_string(),
            cwd: "D:/YunXi Agent".to_string(),
            resume_source: None,
            child_depth: 0,
            data,
        },
    };

    let json = event_text(&redact_runtime_event_for_jsonl(event));

    assert!(!json.contains(&secret));
    assert!(json.contains("[redacted]"));
    assert!(json.contains("visible-safe"));
}
```

---

### Task 4: Promote Version And Documentation To v1.8.6

**Files:**

- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/yunxi-agent-cli/src/main.rs`
- Modify: `crates/yunxi-agent-cli/src/render.rs`
- Modify: `crates/yunxi-agent-cli/tests/cli_tests.rs`
- Modify: `crates/yunxi-agent-tui/src/app.rs`
- Modify: `crates/yunxi-agent-tui/src/render.rs`
- Modify: `crates/yunxi-agent-persona/src/profile.rs`
- Modify: `crates/yunxi-agent-persona/src/compiler.rs`
- Modify: `docs/persona-memory.md`
- Modify: `docs/extraction-status.md`

**Interfaces:**

- Consumes: current `1.8.5` version surfaces.
- Produces: consistent `1.8.6` version identity and docs.

- [ ] **Step 1: Update workspace versions**

Change `[workspace.package] version` in `Cargo.toml`:

```toml
version = "1.8.6"
```

Update `Cargo.lock` YunXi workspace package entries from `version = "1.8.5"` to `version = "1.8.6"`.

- [ ] **Step 2: Update CLI and TUI display strings**

Replace `1.8.5` with `1.8.6` in CLI/TUI version surfaces:

```text
crates/yunxi-agent-cli/src/main.rs
crates/yunxi-agent-cli/src/render.rs
crates/yunxi-agent-cli/tests/cli_tests.rs
crates/yunxi-agent-tui/src/app.rs
crates/yunxi-agent-tui/src/render.rs
```

Expected public outputs after release build:

```text
yunxi 1.8.6
```

TUI header should display:

```text
YunXi Agent v1.8.6
```

- [ ] **Step 3: Update persona version markers**

Replace `1.8.5` with `1.8.6` where persona profile/compiler version is documented:

```text
crates/yunxi-agent-persona/src/profile.rs
crates/yunxi-agent-persona/src/compiler.rs
```

- [ ] **Step 4: Update memory/privacy docs**

In `docs/persona-memory.md`, add or update the machine-readable output privacy note:

```markdown
## Machine-Readable Output Redaction

v1.8.6 sanitizes both `--json` and `--jsonl` agent execution output before
serialization. Secret-like fragments in `AgentRunResult.final_response`,
conversation events, memory recall queries, command/tool text, provider/error
messages, state `data` maps, and nested JSONL protocol payloads are replaced
with `[redacted]` while ordinary non-secret text remains visible.

This output boundary is independent from memory write policy. Secret-like memory
candidates are still discarded instead of persisted; output redaction prevents
the same sensitive text from being printed to machine-readable logs.
```

In `docs/extraction-status.md`, add a `YunXi Agent v1.8.6 JSON Output Redaction` section with construction scope and final verification evidence after all checks complete.

---

### Task 5: Unified Verification, Logging, And Release

**Files:**

- Modify after verification: `docs/extraction-status.md`
- Modify after verification: `docs/reports/2026-07-14-yunxi-agent-v1-8-6-json-output-redaction-development-report.md`
- Modify after verification: `C:\Users\admin\Desktop\YunXi Agent开发日志.md`

**Interfaces:**

- Consumes: completed Tasks 1-4.
- Produces: verified v1.8.6 source state and release record.

- [ ] **Step 1: Run format and targeted tests after construction**

Run:

```powershell
cargo fmt
cargo fmt --check
cargo test -p yunxi-agent-cli redacts_agent_run_result_for_json
cargo test -p yunxi-agent-cli redacts_thread_state_data_for_jsonl
cargo test -p yunxi-agent-cli cli_json_redacts_secret_prompt_while_memory_discards_candidate --test cli_tests
cargo test -p yunxi-agent-cli cli_jsonl_redacts_secret_prompt_while_memory_discards_candidate --test cli_tests
cargo test -p yunxi-agent-cli cli_json_preserves_non_secret_prompt_text --test cli_tests
```

Expected:

```text
all selected tests pass with zero failures
```

- [ ] **Step 2: Run package and workspace verification**

Run:

```powershell
cargo test -p yunxi-agent-persona -p yunxi-agent-storage -p yunxi-agent-runtime -p yunxi-agent-cli -p yunxi-agent-tui
cargo test
cargo check --workspace
git diff --check
```

Expected:

```text
package tests pass
full workspace tests pass
cargo check exits 0
git diff --check has no whitespace errors
```

- [ ] **Step 3: Run release build and version checks**

Run:

```powershell
cargo build -p yunxi-agent-cli --release --bins
target\release\yunxi.exe --version
target\release\yunxi-agent-cli.exe --version
```

Expected:

```text
target\release\yunxi.exe --version => yunxi 1.8.6
target\release\yunxi-agent-cli.exe --version => yunxi 1.8.6
```

- [ ] **Step 4: Run black-box `--json` privacy check**

Use runtime-constructed fake secret strings; do not paste real keys into commands.

```powershell
$env:YUNXI_HOME = Join-Path $env:TEMP 'yunxi-dev-186-json-redaction'
if (Test-Path $env:YUNXI_HOME) { Remove-Item -LiteralPath $env:YUNXI_HOME -Recurse -Force }
$secret = 'sk-' + ('f' * 32)
$prompt = "my token is $secret"
.\target\release\yunxi.exe --cwd . --json memory on | Out-Null
$json = .\target\release\yunxi.exe --offline --memory-extraction rule-only --cwd . --json $prompt
$joined = $json -join "`n"
if ($joined.Contains($secret)) { throw '--json leaked fake secret' }
if (-not $joined.Contains('[redacted]')) { throw '--json did not show redaction marker' }
if (-not $joined.Contains('"action": "discard"')) { throw 'memory write discard was not emitted' }
$parsed = $joined | ConvertFrom-Json
if (-not $parsed.final_response.Contains('[redacted]')) { throw 'final_response was not redacted' }
```

Expected:

```text
no exception
```

- [ ] **Step 5: Re-run black-box `--jsonl` privacy check**

```powershell
$env:YUNXI_HOME = Join-Path $env:TEMP 'yunxi-dev-186-jsonl-regression'
if (Test-Path $env:YUNXI_HOME) { Remove-Item -LiteralPath $env:YUNXI_HOME -Recurse -Force }
$secret = 'sk-' + ('d' * 32)
$prompt = "my token is $secret"
.\target\release\yunxi.exe --cwd . --json memory on | Out-Null
$lines = .\target\release\yunxi.exe --offline --memory-extraction rule-only --cwd . --jsonl $prompt
$joined = $lines -join "`n"
if ($joined.Contains($secret)) { throw 'JSONL leaked fake secret' }
if (-not $joined.Contains('[redacted]')) { throw 'JSONL did not show redaction marker' }
if (-not $joined.Contains('[redacted-sensitive-query]')) { throw 'memory recall query was not redacted' }
if (-not $joined.Contains('"action":"discard"')) { throw 'memory write discard was not emitted' }
```

Expected:

```text
no exception
```

- [ ] **Step 6: Run owned-source secret scan**

Run:

```powershell
rg --hidden -n --glob '!target/**' --glob '!vendor/**' --glob '!extracted/**' --glob '!.git/**' --glob '!.codegraph/**' --glob '!Cargo.lock' -e 'sk-[A-Za-z0-9_-]{20,}|ghp_[A-Za-z0-9_]{20,}|github_pat_[A-Za-z0-9_]{20,}|AIza[0-9A-Za-z_-]{35}|AKIA[0-9A-Z]{16}|Bearer\s+[A-Za-z0-9._-]{20,}|api[_-]?key\s*[:=]\s*["'']?[A-Za-z0-9._-]{20,}' .
```

Expected:

```text
no live key-shaped matches
```

If the scan flags test code, rewrite the test so the fake key is assembled at runtime from non-key-shaped fragments.

- [ ] **Step 7: Sync CodeGraph and refresh install**

Run:

```powershell
codegraph sync .
codegraph status .
powershell -ExecutionPolicy Bypass -File .\scripts\install\install-yunxi.ps1 -AddToPath -SkipBuild
yunxi --version
yunxi-agent-cli --version
```

Expected:

```text
CodeGraph index is up to date
yunxi --version => yunxi 1.8.6
yunxi-agent-cli --version => yunxi 1.8.6
```

- [ ] **Step 8: Record verification and clean artifacts**

Update:

```text
docs/extraction-status.md
docs/reports/2026-07-14-yunxi-agent-v1-8-6-json-output-redaction-development-report.md
C:\Users\admin\Desktop\YunXi Agent开发日志.md
```

Then run:

```powershell
cargo clean
```

Expected:

```text
target build artifacts removed
```

- [ ] **Step 9: Publish only after verification is complete**

Use the current project release workflow for v1.8.6:

- Create one release commit with a message such as `Release YunXi Agent v1.8.6 JSON output redaction`.
- Publish through GitHub REST API only if that remains the active project rule.
- Do not use `git push`, `git fetch`, or `git ls-remote` when the REST-only workflow is active.
- Create a new immutable annotated `v1.8.6` tag.
- Do not delete or move old tags.
- Verify remote branch, tag ref, and tag target after publishing.

Expected final state:

```text
git status --short --branch => ## master...origin/master
v1.8.6 tag target equals remote master
old tags remain untouched
```

## Unified Verification Results

Unified verification was performed after the complete v1.8.6 construction
batch, in accordance with the project hard constraint.

- `cargo fmt`: pass.
- `cargo fmt --check`: pass.
- `cargo test -p yunxi-agent-cli json --test cli_tests`: pass; 11 tests passed.
- `cargo test -p yunxi-agent-cli jsonl --test jsonl_tests`: pass; 9 tests
  passed and 1 non-matching test was filtered out.
- Targeted package tests for persona, storage, runtime, CLI, and TUI: pass.
  The run included 40 CLI integration tests, 10 JSONL integration tests,
  25 persona tests, 40 runtime tests, 25 storage tests, and 52 TUI tests.
- `cargo test`: pass; all workspace unit, integration, and doc tests completed
  with zero failures.
- `cargo check --workspace`: pass.
- `cargo build -p yunxi-agent-cli --release --bins`: pass.
- `target\release\yunxi.exe --version`: pass; `yunxi 1.8.6`.
- `target\release\yunxi-agent-cli.exe --version`: pass; `yunxi 1.8.6`.
- Isolated `YUNXI_HOME` JSON black-box privacy check: pass; 23 events were
  emitted, the fake secret was absent, `[redacted]` and
  `[redacted-sensitive-query]` were present, and memory action was `discard`.
- Isolated `YUNXI_HOME` JSONL black-box privacy check: pass; 23 lines were
  emitted with the same redaction and discard guarantees.
- Memory persistence check: pass; the fake secret was absent from global
  memory listing. Ordinary non-secret JSON prompt text remained visible.
- Owned-source secret scan excluding generated/upstream/build directories:
  pass; no live key-shaped matches.
- `git diff --check`: pass; only expected Windows LF-to-CRLF notices appeared.
- `codegraph sync .`: pass; the index was already current.
- `codegraph status .`: pass; 1,164 files, 44,963 nodes, 145,840 edges, and
  `Index is up to date`.
- `scripts\install\install-yunxi.ps1 -AddToPath -SkipBuild`: pass; binaries
  were installed under the user-local YunXi bin directory and PATH was updated.
- Installed PATH smoke: pass; `yunxi --version` and
  `yunxi-agent-cli --version` both returned `yunxi 1.8.6`.
- `cargo clean`: pass; 33,298 files and approximately 5.1 GiB were removed.
  `D:\YunXi Agent\target` no longer exists after cleanup.

The first test attempt was interrupted before tests ran because the rustup shim
could not write its external temporary directory in the sandbox. Verification
then used explicit installed `RUSTC`/`RUSTDOC` paths and a project-local Cargo
cache under `target`; the complete verification batch above passed. The local
cache was removed by the final `cargo clean`.

Pre-publish GitHub REST API verification confirmed that remote `master` still
pointed to `559bd8b333ddee35304672c58861f73517deed18` and `v1.8.6` did not
exist. The verified tree is therefore ready for one new release commit and an
immutable annotated `v1.8.6` tag; older tags must remain untouched. The final
remote commit and tag object are recorded in the external development log after
REST API publishing, because their identifiers cannot be embedded in the tree
that generates those identifiers.

## Self-Review

- Spec coverage: Task 1 implements typed `AgentRunResult` and `AgentEvent` redaction for the P1 `--json` leak. Task 2 wires the CLI branch and adds black-box regression tests for final response, started prompt, assistant echo, memory recall query, and memory discard. Task 3 closes the P2 `ThreadState.data` JSONL gap. Task 4 updates version/docs. Task 5 defines verification, logging, cleanup, and release requirements.
- Placeholder scan: The plan contains no deferred implementation markers. Each code-changing task names exact files, function signatures, and expected test commands.
- Type consistency: The plan uses current source types verified from the repository: `AgentRunResult`, `AgentEvent`, `ThreadRuntimeState`, `TodoStatus`, protocol `RuntimeEvent::ThreadState`, and protocol `ThreadState.data`.

## Completion Definition

YunXi Agent v1.8.6 is complete only when:

- `--json` fake secret prompt stdout contains no fake secret bytes.
- `--json` stdout contains `[redacted]` where secret-like fragments were removed.
- `--json` `final_response`, `events[].Started.prompt`, `events[].Message.content`, command/tool text, provider/error/warning text, memory recall query, and state data maps are structurally redacted before serialization.
- `--json` still preserves ordinary non-secret prompt text.
- `--json` memory write still emits `action=discard` for secret-like memory candidates.
- Secret-like memory candidates are not persisted to `memory list --global`.
- Existing `--jsonl` redaction behavior remains intact.
- `RuntimeEvent::ThreadState.data` is redacted in JSONL output.
- Full unified verification passes after construction.
- CodeGraph is synced and reports the index up to date.
- Desktop development log is updated.
- Build artifacts are cleaned.
- Release publishing, if requested, creates immutable `v1.8.6` without moving old tags.

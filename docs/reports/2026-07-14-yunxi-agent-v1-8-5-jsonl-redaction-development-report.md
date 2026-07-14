# YunXi Agent v1.8.5 JSONL Redaction Development Report

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the v1.8.4 audit finding that JSONL transcript events can expose secret-like user prompts and offline assistant echo text.

**Architecture:** Add a YunXi-owned JSONL output sanitization boundary in `yunxi-agent-cli` immediately before machine-readable runtime events are serialized. Reuse the existing provider redaction function so JSONL, provider errors, TUI summaries, and memory diagnostics share one secret-fragment policy. Keep memory write policy unchanged: refusing to store secrets and refusing to print secrets are separate safeguards.

**Tech Stack:** Rust 2024, existing YunXi workspace crates, `yunxi_agent_protocol::RuntimeEvent`, `yunxi_agent_provider::redact_sensitive_text`, `serde_json`, `assert_cmd`, `predicates`, `tempfile`.

## Global Constraints

- Base version: `1.8.4`; next version is `1.8.5`.
- Source audit input: `C:\Users\admin\Desktop\YunXi Agent调研项目\2026-07-14-YunXi-Agent-1.8.4-源码审核报告.md`.
- Primary P2 issue: JSONL ordinary conversation `item` events currently print raw user prompt and offline assistant echo.
- Do not treat memory discard policy as output redaction.
- Do not print or commit real API keys, tokens, passwords, or private file contents.
- Tests may construct fake secret-like strings at runtime; do not hard-code full key-shaped literals that trip repository secret scans.
- Keep persona/memory local, inspectable, and user-controlled.
- Do not add SQLite, vector search, graph memory, relationship state machines, external memory runtimes, or new TUI pages in this version.
- Follow the user's hard constraint: build the full source slice first, do not run mid-construction tests, and run unified verification only after construction is complete.
- Before release completion, run `cargo fmt`, `cargo fmt --check`, targeted tests, full `cargo test`, `cargo check --workspace`, release build/version checks, `git diff --check`, secret scan, CodeGraph sync/status, install refresh, and `cargo clean`.
- Release through GitHub REST API only; do not use `git push`, `git fetch`, or `git ls-remote`.
- Create a new immutable annotated `v1.8.5` tag. Do not delete or move old tags.
- Update `C:\Users\admin\Desktop\YunXi Agent开发日志.md` at task end.

---

## Audit Summary

The v1.8.4 audit accepts the English language preference and `source_session_id` fixes. No P1 blocker remains for that slice.

The remaining P2 privacy gap is the JSONL transcript boundary:

- `memory_recall.query` is already redacted for sensitive prompts.
- memory candidates with secret-like content become high sensitivity.
- `memory_write` discards secret-like memory as expected.
- However `RuntimeEvent::Item` for ordinary user and assistant messages still serializes raw content.
- Offline assistant output compounds the issue because it echoes the prompt in `YunXi autonomous runtime accepted prompt: ...`.

The fix should happen at CLI JSONL serialization time, not by weakening or overloading memory extraction policy.

## Current Code Boundary

Relevant current paths:

- `crates/yunxi-agent-cli/src/main.rs:696`
  - `print_run_result` serializes every event with `to_jsonl_line(&event)` when `--jsonl` is selected.
- `crates/yunxi-agent-cli/src/main.rs:739`
  - `protocol_events_from_agent_events` maps `AgentEvent` into `RuntimeEvent`.
- `crates/yunxi-agent-cli/src/main.rs:755`
  - `AgentEvent::Started { prompt }` becomes `RuntimeEvent::Item { item: ResponseItem::Message { role: User, content: prompt.clone() } }`.
- `crates/yunxi-agent-cli/src/main.rs:1117`
  - `AgentEvent::Message { content }` becomes assistant `RuntimeEvent::Item`.
- `crates/yunxi-agent-cli/src/main.rs:611`
  - existing `redact_secret_fragments` delegates to `yunxi_agent_provider::redact_sensitive_text`.
- `crates/yunxi-agent-protocol/src/lib.rs:393`
  - `RuntimeEvent` owns the JSONL event schema.

## File Structure

- Modify: `crates/yunxi-agent-cli/src/main.rs`
  - Route JSONL output through a sanitizer before `to_jsonl_line`.
  - Add `mod jsonl_redaction;` if the sanitizer is placed in a focused module.
- Create: `crates/yunxi-agent-cli/src/jsonl_redaction.rs`
  - Own redaction helpers for `RuntimeEvent`, `StreamEvent`, `ResponseItem`, `ResponseItemDelta`, `ToolCall`, `FunctionCallOutput`, `ContentItem`, `serde_json::Value`, and `BTreeMap<String, String>`.
  - Unit-test event redaction without spawning CLI processes.
- Modify: `crates/yunxi-agent-cli/tests/jsonl_tests.rs`
  - Add black-box JSONL tests for secret-like prompts and offline echo redaction.
- Modify: `crates/yunxi-agent-cli/tests/cli_tests.rs`
  - Add memory-enabled JSONL test proving output redaction does not replace memory discard behavior.
- Modify: `Cargo.toml`
  - Promote workspace version to `1.8.5`.
- Modify: `Cargo.lock`
  - Promote workspace crate versions to `1.8.5`.
- Modify: `crates/yunxi-agent-cli/src/main.rs`
  - Update CLI version/banner/help/error version strings to `1.8.5`.
- Modify: `crates/yunxi-agent-cli/src/render.rs`
  - Update interactive banner to `1.8.5`.
- Modify: `crates/yunxi-agent-cli/tests/cli_tests.rs`
  - Update version/banner expectations to `1.8.5`.
- Modify: `crates/yunxi-agent-tui/src/app.rs`
  - Update TUI displayed version to `v1.8.5`.
- Modify: `crates/yunxi-agent-tui/src/render.rs`
  - Update TUI test expectations if any snapshot/string checks mention version.
- Modify: `crates/yunxi-agent-persona/src/profile.rs`
  - Update profile version to `1.8.5`.
- Modify: `crates/yunxi-agent-persona/src/compiler.rs`
  - Update version comments to `v1.8.5`.
- Modify: `docs/persona-memory.md`
  - Document that JSONL transcript output is sanitized independently from memory write policy.
- Modify: `docs/extraction-status.md`
  - Add a v1.8.5 section with construction scope and verification results after final validation.
- Create: `docs/reports/2026-07-14-yunxi-agent-v1-8-5-jsonl-redaction-development-report.md`
  - This report.

## Non-Goals

- Do not change the memory write privacy policy.
- Do not suppress whole JSONL `item` events; redact only sensitive fragments so automation can still consume event structure.
- Do not redact ordinary non-secret user text from JSONL.
- Do not add a new CLI flag for redaction. JSONL redaction is always on.
- Do not introduce a second secret detector. Reuse `yunxi_agent_provider::redact_sensitive_text`.
- Do not move protocol schema types just to solve this CLI output issue.
- Do not perform real-terminal screenshot validation for this privacy-only slice.

---

### Task 1: Add Focused JSONL Redaction Helpers

**Files:**

- Create: `crates/yunxi-agent-cli/src/jsonl_redaction.rs`
- Modify: `crates/yunxi-agent-cli/src/main.rs`

**Interfaces:**

- Consumes: `yunxi_agent_protocol::RuntimeEvent`
- Produces: `pub(crate) fn redact_runtime_event_for_jsonl(event: RuntimeEvent) -> RuntimeEvent`
- Produces: redacted clones of nested response/tool/stream payloads before JSONL serialization.

- [ ] **Step 1: Create the module and public facade**

Create `crates/yunxi-agent-cli/src/jsonl_redaction.rs`:

```rust
use std::collections::BTreeMap;

use serde_json::Value;
use yunxi_agent_protocol::{
    ContentItem, FunctionCallOutput, ResponseItem, ResponseItemDelta, RuntimeEvent, StreamEvent,
    ToolCall,
};

pub(crate) fn redact_runtime_event_for_jsonl(event: RuntimeEvent) -> RuntimeEvent {
    match event {
        RuntimeEvent::DeepParityState {
            thread_id,
            turn_id,
            layer,
            status,
            message,
            data,
        } => RuntimeEvent::DeepParityState {
            thread_id,
            turn_id,
            layer,
            status,
            message: message.map(redact_text),
            data: redact_string_map(data),
        },
        RuntimeEvent::SandboxAttempt {
            thread_id,
            turn_id,
            call_id,
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
        } => RuntimeEvent::SandboxAttempt {
            thread_id,
            turn_id,
            call_id,
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
        RuntimeEvent::TurnMetadata {
            thread_id,
            turn_id,
            mut metadata,
        } => {
            metadata.extra = redact_string_map(metadata.extra);
            RuntimeEvent::TurnMetadata {
                thread_id,
                turn_id,
                metadata,
            }
        }
        RuntimeEvent::TurnState {
            thread_id,
            turn_id,
            mut state,
        } => {
            state.data = redact_string_map(state.data);
            RuntimeEvent::TurnState {
                thread_id,
                turn_id,
                state,
            }
        }
        RuntimeEvent::MemoryRecall {
            thread_id,
            turn_id,
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
        } => RuntimeEvent::MemoryRecall {
            thread_id,
            turn_id,
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
        RuntimeEvent::MemoryWarning {
            thread_id,
            turn_id,
            schema_version,
            warning,
        } => RuntimeEvent::MemoryWarning {
            thread_id,
            turn_id,
            schema_version,
            warning: redact_text(warning),
        },
        RuntimeEvent::Item {
            thread_id,
            turn_id,
            item,
        } => RuntimeEvent::Item {
            thread_id,
            turn_id,
            item: redact_response_item(item),
        },
        RuntimeEvent::ItemDelta {
            thread_id,
            turn_id,
            delta,
        } => RuntimeEvent::ItemDelta {
            thread_id,
            turn_id,
            delta: redact_response_delta(delta),
        },
        RuntimeEvent::Stream { event } => RuntimeEvent::Stream {
            event: redact_stream_event(event),
        },
        RuntimeEvent::ToolStarted {
            thread_id,
            turn_id,
            call,
        } => RuntimeEvent::ToolStarted {
            thread_id,
            turn_id,
            call: redact_tool_call(call),
        },
        RuntimeEvent::ToolCompleted {
            thread_id,
            turn_id,
            call_id,
            output,
            success,
        } => RuntimeEvent::ToolCompleted {
            thread_id,
            turn_id,
            call_id,
            output: redact_text(output),
            success,
        },
        RuntimeEvent::ApprovalRequested {
            thread_id,
            turn_id,
            call_id,
            tool_name,
            reason,
        } => RuntimeEvent::ApprovalRequested {
            thread_id,
            turn_id,
            call_id,
            tool_name,
            reason: redact_text(reason),
        },
        RuntimeEvent::ApprovalCompleted {
            thread_id,
            turn_id,
            call_id,
            approved,
            reason,
        } => RuntimeEvent::ApprovalCompleted {
            thread_id,
            turn_id,
            call_id,
            approved,
            reason: reason.map(redact_text),
        },
        RuntimeEvent::EscalationRequested {
            thread_id,
            turn_id,
            call_id,
            tool_name,
            reason,
            required_sandbox,
            required_network,
        } => RuntimeEvent::EscalationRequested {
            thread_id,
            turn_id,
            call_id,
            tool_name,
            reason: redact_text(reason),
            required_sandbox,
            required_network,
        },
        RuntimeEvent::EscalationCompleted {
            thread_id,
            turn_id,
            call_id,
            approved,
            reason,
        } => RuntimeEvent::EscalationCompleted {
            thread_id,
            turn_id,
            call_id,
            approved,
            reason: reason.map(redact_text),
        },
        RuntimeEvent::McpSession {
            thread_id,
            turn_id,
            server,
            status,
            message,
        } => RuntimeEvent::McpSession {
            thread_id,
            turn_id,
            server,
            status,
            message: message.map(redact_text),
        },
        RuntimeEvent::MultiAgent {
            thread_id,
            turn_id,
            agent_id,
            parent_agent_id,
            status,
            message,
        } => RuntimeEvent::MultiAgent {
            thread_id,
            turn_id,
            agent_id,
            parent_agent_id,
            status,
            message: message.map(redact_text),
        },
        RuntimeEvent::ChildAgent {
            thread_id,
            turn_id,
            agent_id,
            child_session_id,
            parent_session_id,
            status,
            message,
        } => RuntimeEvent::ChildAgent {
            thread_id,
            turn_id,
            agent_id,
            child_session_id,
            parent_session_id,
            status,
            message: message.map(redact_text),
        },
        RuntimeEvent::ChildScopedStream {
            thread_id,
            turn_id,
            agent_id,
            child_session_id,
            parent_session_id,
            event,
            seq,
            message,
        } => RuntimeEvent::ChildScopedStream {
            thread_id,
            turn_id,
            agent_id,
            child_session_id,
            parent_session_id,
            event,
            seq,
            message: message.map(redact_text),
        },
        RuntimeEvent::Cancelled {
            thread_id,
            turn_id,
            reason,
        } => RuntimeEvent::Cancelled {
            thread_id,
            turn_id,
            reason: reason.map(redact_text),
        },
        RuntimeEvent::ProviderError {
            thread_id,
            turn_id,
            provider,
            status,
            classification,
            message,
        } => RuntimeEvent::ProviderError {
            thread_id,
            turn_id,
            provider,
            status,
            classification,
            message: redact_text(message),
        },
        RuntimeEvent::Error {
            thread_id,
            turn_id,
            message,
        } => RuntimeEvent::Error {
            thread_id,
            turn_id,
            message: redact_text(message),
        },
        other => other,
    }
}
```

- [ ] **Step 2: Add nested payload redaction helpers**

Append to the same file:

```rust
fn redact_stream_event(event: StreamEvent) -> StreamEvent {
    match event {
        StreamEvent::ResponseStarted {
            thread_id,
            turn_id,
            metadata,
        } => StreamEvent::ResponseStarted {
            thread_id,
            turn_id,
            metadata: metadata.map(|mut metadata| {
                metadata.extra = redact_string_map(metadata.extra);
                metadata
            }),
        },
        StreamEvent::ItemStarted {
            thread_id,
            turn_id,
            item,
        } => StreamEvent::ItemStarted {
            thread_id,
            turn_id,
            item: redact_response_item(item),
        },
        StreamEvent::ItemDelta {
            thread_id,
            turn_id,
            delta,
        } => StreamEvent::ItemDelta {
            thread_id,
            turn_id,
            delta: redact_response_delta(delta),
        },
        StreamEvent::ItemCompleted {
            thread_id,
            turn_id,
            item,
        } => StreamEvent::ItemCompleted {
            thread_id,
            turn_id,
            item: redact_response_item(item),
        },
        StreamEvent::ResponseFailed {
            thread_id,
            turn_id,
            message,
        } => StreamEvent::ResponseFailed {
            thread_id,
            turn_id,
            message: redact_text(message),
        },
        StreamEvent::ResponseCancelled {
            thread_id,
            turn_id,
            reason,
        } => StreamEvent::ResponseCancelled {
            thread_id,
            turn_id,
            reason: reason.map(redact_text),
        },
        other => other,
    }
}

fn redact_response_item(item: ResponseItem) -> ResponseItem {
    match item {
        ResponseItem::Message { role, content } => ResponseItem::Message {
            role,
            content: redact_text(content),
        },
        ResponseItem::Reasoning { content } => ResponseItem::Reasoning {
            content: redact_text(content),
        },
        ResponseItem::ToolCall { call } => ResponseItem::ToolCall {
            call: redact_tool_call(call),
        },
        ResponseItem::AgentMessage { id, content, phase } => ResponseItem::AgentMessage {
            id,
            content: content.into_iter().map(redact_content_item).collect(),
            phase,
        },
        ResponseItem::ReasoningItem {
            id,
            summary_text,
            raw_content,
        } => ResponseItem::ReasoningItem {
            id,
            summary_text: summary_text.into_iter().map(redact_text).collect(),
            raw_content: raw_content.into_iter().map(redact_text).collect(),
        },
        ResponseItem::LocalShellCall {
            id,
            status,
            command,
        } => ResponseItem::LocalShellCall {
            id,
            status,
            command: command.into_iter().map(redact_text).collect(),
        },
        ResponseItem::FunctionCall {
            id,
            call_id,
            name,
            arguments,
            status,
        } => ResponseItem::FunctionCall {
            id,
            call_id,
            name,
            arguments: redact_text(arguments),
            status,
        },
        ResponseItem::FunctionCallOutput {
            id,
            call_id,
            output,
        } => ResponseItem::FunctionCallOutput {
            id,
            call_id,
            output: redact_function_output(output),
        },
        ResponseItem::McpToolCall {
            id,
            call_id,
            server,
            tool,
            arguments,
            status,
        } => ResponseItem::McpToolCall {
            id,
            call_id,
            server,
            tool,
            arguments: redact_text(arguments),
            status,
        },
        ResponseItem::ToolSearchCall {
            id,
            call_id,
            query,
            status,
        } => ResponseItem::ToolSearchCall {
            id,
            call_id,
            query: redact_text(query),
            status,
        },
        ResponseItem::WebSearchCall { id, query, status } => ResponseItem::WebSearchCall {
            id,
            query: redact_text(query),
            status,
        },
        ResponseItem::Compaction { id, summary } => ResponseItem::Compaction {
            id,
            summary: redact_text(summary),
        },
        ResponseItem::CompactionTrigger { id, reason } => ResponseItem::CompactionTrigger {
            id,
            reason: redact_text(reason),
        },
        other => other,
    }
}

fn redact_response_delta(delta: ResponseItemDelta) -> ResponseItemDelta {
    match delta {
        ResponseItemDelta::MessageContent { item_id, delta } => {
            ResponseItemDelta::MessageContent {
                item_id,
                delta: redact_text(delta),
            }
        }
        ResponseItemDelta::ReasoningContent { item_id, delta } => {
            ResponseItemDelta::ReasoningContent {
                item_id,
                delta: redact_text(delta),
            }
        }
        ResponseItemDelta::ToolCallArguments { call_id, delta } => {
            ResponseItemDelta::ToolCallArguments {
                call_id,
                delta: redact_text(delta),
            }
        }
        ResponseItemDelta::ToolCallName { call_id, name } => ResponseItemDelta::ToolCallName {
            call_id,
            name: redact_text(name),
        },
        ResponseItemDelta::ToolOutput { call_id, delta } => ResponseItemDelta::ToolOutput {
            call_id,
            delta: redact_text(delta),
        },
        other => other,
    }
}
```

- [ ] **Step 3: Add tool/output/value helpers**

Append to the same file:

```rust
fn redact_tool_call(call: ToolCall) -> ToolCall {
    match call {
        ToolCall::Shell { id, command } => ToolCall::Shell {
            id,
            command: redact_text(command),
        },
        ToolCall::Patch { id, patch } => ToolCall::Patch {
            id,
            patch: redact_text(patch),
        },
        ToolCall::Mcp {
            id,
            server,
            tool,
            arguments_json,
        } => ToolCall::Mcp {
            id,
            server,
            tool,
            arguments_json: arguments_json.map(redact_text),
        },
        ToolCall::Skill {
            id,
            name,
            arguments_json,
        } => ToolCall::Skill {
            id,
            name,
            arguments_json: arguments_json.map(redact_text),
        },
        ToolCall::MultiAgent {
            id,
            action,
            arguments_json,
        } => ToolCall::MultiAgent {
            id,
            action,
            arguments_json: arguments_json.map(redact_text),
        },
        ToolCall::ToolSearch { id, query } => ToolCall::ToolSearch {
            id,
            query: redact_text(query),
        },
        ToolCall::RequestUserInput { id, prompt } => ToolCall::RequestUserInput {
            id,
            prompt: redact_text(prompt),
        },
        other => other,
    }
}

fn redact_function_output(output: FunctionCallOutput) -> FunctionCallOutput {
    match output {
        FunctionCallOutput::Text { text } => FunctionCallOutput::Text {
            text: redact_text(text),
        },
        FunctionCallOutput::ContentItems { items } => FunctionCallOutput::ContentItems {
            items: items.into_iter().map(redact_content_item).collect(),
        },
        FunctionCallOutput::Json { value } => FunctionCallOutput::Json {
            value: redact_json_value(value),
        },
    }
}

fn redact_content_item(item: ContentItem) -> ContentItem {
    match item {
        ContentItem::InputText { text } => ContentItem::InputText {
            text: redact_text(text),
        },
        ContentItem::OutputText { text } => ContentItem::OutputText {
            text: redact_text(text),
        },
        other => other,
    }
}

fn redact_json_value(value: Value) -> Value {
    match value {
        Value::String(value) => Value::String(redact_text(value)),
        Value::Array(values) => {
            Value::Array(values.into_iter().map(redact_json_value).collect())
        }
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| (key, redact_json_value(value)))
                .collect(),
        ),
        other => other,
    }
}

fn redact_string_map(map: BTreeMap<String, String>) -> BTreeMap<String, String> {
    map.into_iter()
        .map(|(key, value)| (key, redact_text(value)))
        .collect()
}

fn redact_text(value: String) -> String {
    yunxi_agent_provider::redact_sensitive_text(&value)
}
```

- [ ] **Step 4: Wire JSONL serialization through the sanitizer**

In `crates/yunxi-agent-cli/src/main.rs`, add near the other module declarations:

```rust
mod jsonl_redaction;
```

Then update `print_run_result`:

```rust
fn print_run_result(
    result: AgentRunResult,
    json: bool,
    jsonl: bool,
    offline_label: bool,
) -> Result<()> {
    if jsonl {
        for event in protocol_events_from_agent_events(&result.events) {
            let event = jsonl_redaction::redact_runtime_event_for_jsonl(event);
            println!("{}", to_jsonl_line(&event)?);
        }
    } else if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else if let Some(final_response) = result.final_response {
        if offline_label {
            println!("[offline] {final_response}");
        } else {
            println!("{final_response}");
        }
    }

    Ok(())
}
```

Do not change plain output in this task; the audit finding is JSONL transcript output.

---

### Task 2: Add Unit Tests For JSONL Sanitization

**Files:**

- Modify: `crates/yunxi-agent-cli/src/jsonl_redaction.rs`

**Interfaces:**

- Consumes: `redact_runtime_event_for_jsonl(RuntimeEvent) -> RuntimeEvent`
- Produces: helper-level regression coverage for nested JSONL payloads.

- [ ] **Step 1: Add a test helper without hard-coded full secrets**

Append to `crates/yunxi-agent-cli/src/jsonl_redaction.rs`:

```rust
#[cfg(test)]
mod tests {
    use serde_json::json;
    use yunxi_agent_protocol::{
        ContentItem, FunctionCallOutput, MessagePhase, ProtocolRole, ResponseItem,
        ResponseItemDelta, RuntimeEvent, StreamEvent, ThreadId, ToolCall, ToolCallStatus, TurnId,
    };

    use super::redact_runtime_event_for_jsonl;

    fn fake_secret() -> String {
        format!("sk-{}", "a".repeat(32))
    }

    fn event_text(event: &RuntimeEvent) -> String {
        serde_json::to_string(event).expect("event json")
    }
```

- [ ] **Step 2: Cover user and assistant message item redaction**

Append inside the same `tests` module:

```rust
    #[test]
    fn redacts_user_and_assistant_message_items() {
        let secret = fake_secret();
        let user_event = RuntimeEvent::Item {
            thread_id: ThreadId("thread".to_string()),
            turn_id: TurnId("turn".to_string()),
            item: ResponseItem::Message {
                role: ProtocolRole::User,
                content: format!("my token is {secret}"),
            },
        };
        let assistant_event = RuntimeEvent::Item {
            thread_id: ThreadId("thread".to_string()),
            turn_id: TurnId("turn".to_string()),
            item: ResponseItem::Message {
                role: ProtocolRole::Assistant,
                content: format!("YunXi autonomous runtime accepted prompt: my token is {secret}"),
            },
        };

        let user_redacted = redact_runtime_event_for_jsonl(user_event);
        let assistant_redacted = redact_runtime_event_for_jsonl(assistant_event);
        let user_json = event_text(&user_redacted);
        let assistant_json = event_text(&assistant_redacted);

        assert!(!user_json.contains(&secret));
        assert!(!assistant_json.contains(&secret));
        assert!(user_json.contains("[redacted]"));
        assert!(assistant_json.contains("[redacted]"));
        assert!(assistant_json.contains("YunXi autonomous runtime accepted prompt"));
    }
```

- [ ] **Step 3: Cover nested stream, delta, tool, and JSON payload redaction**

Append inside the same `tests` module:

```rust
    #[test]
    fn redacts_nested_stream_delta_tool_and_json_payloads() {
        let secret = fake_secret();
        let event = RuntimeEvent::Stream {
            event: StreamEvent::ItemCompleted {
                thread_id: ThreadId("thread".to_string()),
                turn_id: TurnId("turn".to_string()),
                item: ResponseItem::FunctionCallOutput {
                    id: "item".to_string(),
                    call_id: "call".to_string(),
                    output: FunctionCallOutput::Json {
                        value: json!({
                            "safe": "visible",
                            "credential": format!("Bearer {secret}"),
                            "nested": { "token": secret.clone() }
                        }),
                    },
                },
            },
        };
        let redacted = redact_runtime_event_for_jsonl(event);
        let json = event_text(&redacted);

        assert!(!json.contains(&secret));
        assert!(json.contains("visible"));
        assert!(json.contains("[redacted]"));

        let delta_event = RuntimeEvent::ItemDelta {
            thread_id: ThreadId("thread".to_string()),
            turn_id: TurnId("turn".to_string()),
            delta: ResponseItemDelta::ToolOutput {
                call_id: Some("call".to_string()),
                delta: format!("tool printed {secret}"),
            },
        };
        let delta_json = event_text(&redact_runtime_event_for_jsonl(delta_event));
        assert!(!delta_json.contains(&secret));
        assert!(delta_json.contains("[redacted]"));

        let tool_event = RuntimeEvent::ToolStarted {
            thread_id: ThreadId("thread".to_string()),
            turn_id: TurnId("turn".to_string()),
            call: ToolCall::Shell {
                id: Some("shell".to_string()),
                command: format!("echo {secret}"),
            },
        };
        let tool_json = event_text(&redact_runtime_event_for_jsonl(tool_event));
        assert!(!tool_json.contains(&secret));
        assert!(tool_json.contains("[redacted]"));
    }
```

- [ ] **Step 4: Cover agent-message content and response-failed redaction**

Append inside the same `tests` module and close it:

```rust
    #[test]
    fn redacts_agent_message_content_and_failed_stream_message() {
        let secret = fake_secret();
        let item_event = RuntimeEvent::Item {
            thread_id: ThreadId("thread".to_string()),
            turn_id: TurnId("turn".to_string()),
            item: ResponseItem::AgentMessage {
                id: "agent-message".to_string(),
                content: vec![
                    ContentItem::OutputText {
                        text: format!("assistant saw {secret}"),
                    },
                    ContentItem::InputText {
                        text: format!("input contains token={secret}"),
                    },
                ],
                phase: MessagePhase::Completed,
            },
        };
        let item_json = event_text(&redact_runtime_event_for_jsonl(item_event));
        assert!(!item_json.contains(&secret));
        assert!(item_json.contains("[redacted]"));

        let failed_event = RuntimeEvent::Stream {
            event: StreamEvent::ResponseFailed {
                thread_id: ThreadId("thread".to_string()),
                turn_id: TurnId("turn".to_string()),
                message: format!("provider returned {secret}"),
            },
        };
        let failed_json = event_text(&redact_runtime_event_for_jsonl(failed_event));
        assert!(!failed_json.contains(&secret));
        assert!(failed_json.contains("[redacted]"));

        let shell_event = RuntimeEvent::Item {
            thread_id: ThreadId("thread".to_string()),
            turn_id: TurnId("turn".to_string()),
            item: ResponseItem::LocalShellCall {
                id: "shell".to_string(),
                status: ToolCallStatus::Completed,
                command: vec!["cmd".to_string(), "/C".to_string(), secret.clone()],
            },
        };
        let shell_json = event_text(&redact_runtime_event_for_jsonl(shell_event));
        assert!(!shell_json.contains(&secret));
        assert!(shell_json.contains("[redacted]"));
    }
}
```

---

### Task 3: Add Black-Box CLI JSONL Regression Tests

**Files:**

- Modify: `crates/yunxi-agent-cli/tests/jsonl_tests.rs`
- Modify: `crates/yunxi-agent-cli/tests/cli_tests.rs`

**Interfaces:**

- Consumes: `yunxi-agent-cli --jsonl`
- Produces: regression tests proving secret-like prompt bytes do not appear in JSONL stdout.

- [ ] **Step 1: Add JSONL transcript redaction black-box test**

Append to `crates/yunxi-agent-cli/tests/jsonl_tests.rs`:

```rust
#[test]
fn yunxi_jsonl_redacts_secret_like_prompt_from_transcript_items() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");
    let temp = TempDir::new().expect("temp dir");
    let secret = format!("sk-{}", "b".repeat(32));
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
    assert!(saw_assistant, "expected redacted assistant item in JSONL output");
}
```

- [ ] **Step 2: Add memory-enabled JSONL privacy test**

Append to `crates/yunxi-agent-cli/tests/cli_tests.rs`:

```rust
#[test]
fn cli_jsonl_redacts_secret_prompt_while_memory_discards_candidate() {
    let home = TempDir::new().expect("yunxi home");
    let workspace = TempDir::new().expect("workspace");
    let cwd = workspace.path().to_str().expect("workspace path");
    let secret = format!("sk-{}", "c".repeat(32));
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
```

- [ ] **Step 3: Update existing JSONL output expectation if needed**

If `yunxi_jsonl_prints_one_json_event_per_line` or other tests assert the exact offline echo for a non-secret prompt, keep those assertions unchanged. Ordinary non-secret prompts must still pass through unchanged.

---

### Task 4: Promote Version And Documentation To v1.8.5

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

- Consumes: current `1.8.4` version surfaces.
- Produces: consistent `1.8.5` version identity and docs.

- [ ] **Step 1: Update workspace versions**

Change `[workspace.package] version` in `Cargo.toml`:

```toml
version = "1.8.5"
```

Update `Cargo.lock` workspace package entries from `version = "1.8.4"` to `version = "1.8.5"` for all YunXi workspace crates.

- [ ] **Step 2: Update CLI and TUI display strings**

Replace `1.8.4` with `1.8.5` in CLI/TUI version surfaces:

```text
crates/yunxi-agent-cli/src/main.rs
crates/yunxi-agent-cli/src/render.rs
crates/yunxi-agent-cli/tests/cli_tests.rs
crates/yunxi-agent-tui/src/app.rs
crates/yunxi-agent-tui/src/render.rs
```

Expected public outputs after release build:

```text
yunxi 1.8.5
```

TUI header should display:

```text
YunXi Agent v1.8.5
```

- [ ] **Step 3: Update persona version markers**

Replace `1.8.4` with `1.8.5` where the persona profile/compiler version is documented:

```text
crates/yunxi-agent-persona/src/profile.rs
crates/yunxi-agent-persona/src/compiler.rs
```

- [ ] **Step 4: Update memory/privacy docs**

In `docs/persona-memory.md`, add a privacy note after the JSONL recall diagnostics section:

```markdown
## JSONL Redaction

v1.8.5 sanitizes JSONL transcript output before serialization. Secret-like
fragments in ordinary user/assistant message items, stream deltas, tool output,
tool arguments, approval/escalation reasons, child-agent messages, and nested
JSON function-call output are replaced with `[redacted]`.

This output boundary is independent from memory write policy. Secret-like
memory candidates are still discarded instead of persisted; JSONL redaction
prevents the same sensitive text from being printed to machine-readable logs.
```

In `docs/extraction-status.md`, add a `YunXi Agent v1.8.5 JSONL Redaction` section. During construction, record scope and planned verification; after final verification, replace the pending note with actual pass/fail evidence.

---

### Task 5: Unified Verification And Release

**Files:**

- Modify after verification: `docs/extraction-status.md`
- Modify after verification: `docs/reports/2026-07-14-yunxi-agent-v1-8-5-jsonl-redaction-development-report.md`
- Modify after verification: `C:\Users\admin\Desktop\YunXi Agent开发日志.md`

**Interfaces:**

- Consumes: completed Tasks 1-4.
- Produces: verified v1.8.5 release commit and immutable tag.

- [ ] **Step 1: Run format and tests only after construction is complete**

Run:

```powershell
cargo fmt
cargo fmt --check
cargo test -p yunxi-agent-cli jsonl --test jsonl_tests
cargo test -p yunxi-agent-cli cli_jsonl_redacts_secret_prompt_while_memory_discards_candidate --test cli_tests
cargo test -p yunxi-agent-persona -p yunxi-agent-storage -p yunxi-agent-runtime -p yunxi-agent-cli -p yunxi-agent-tui
cargo test
```

Expected:

```text
all selected tests pass
full workspace tests pass with zero failures
```

- [ ] **Step 2: Run build, version, and diff checks**

Run:

```powershell
cargo check --workspace
cargo build -p yunxi-agent-cli --release --bins
target\release\yunxi.exe --version
target\release\yunxi-agent-cli.exe --version
git diff --check
```

Expected:

```text
target\release\yunxi.exe --version => yunxi 1.8.5
target\release\yunxi-agent-cli.exe --version => yunxi 1.8.5
git diff --check has no whitespace errors
```

- [ ] **Step 3: Run black-box JSONL privacy checks**

Use runtime-constructed fake secret strings; do not paste real keys into commands.

PowerShell check:

```powershell
$env:YUNXI_HOME = Join-Path $env:TEMP 'yunxi-dev-185-jsonl-redaction'
if (Test-Path $env:YUNXI_HOME) { Remove-Item -LiteralPath $env:YUNXI_HOME -Recurse -Force }
$secret = 'sk-' + ('d' * 32)
$prompt = \"my token is $secret\"
.\target\release\yunxi.exe --json memory on | Out-Null
$lines = .\target\release\yunxi.exe --offline --memory-extraction rule-only --jsonl $prompt
$joined = $lines -join \"`n\"
if ($joined.Contains($secret)) { throw 'JSONL leaked fake secret' }
if (-not $joined.Contains('[redacted]')) { throw 'JSONL did not show redaction marker' }
if (-not $joined.Contains('[redacted-sensitive-query]')) { throw 'memory recall query was not redacted' }
if (-not $joined.Contains('\"action\":\"discard\"')) { throw 'memory write discard was not emitted' }
```

Expected:

```text
no exception
```

- [ ] **Step 4: Run owned-source secret scan**

Run:

```powershell
rg --hidden -n --glob '!target/**' --glob '!vendor/**' --glob '!extracted/**' --glob '!.git/**' --glob '!.codegraph/**' --glob '!Cargo.lock' -e 'sk-[A-Za-z0-9_-]{20,}|ghp_[A-Za-z0-9_]{20,}|github_pat_[A-Za-z0-9_]{20,}|AIza[0-9A-Za-z_-]{35}|AKIA[0-9A-Z]{16}|Bearer\s+[A-Za-z0-9._-]{20,}|api[_-]?key\s*[:=]\s*[\"'']?[A-Za-z0-9._-]{20,}' .
```

Expected:

```text
no live key-shaped matches
```

If the scan flags test code, rewrite the test so the fake key is assembled at runtime from non-key-shaped fragments.

- [ ] **Step 5: Sync CodeGraph and refresh install**

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
yunxi --version => yunxi 1.8.5
yunxi-agent-cli --version => yunxi 1.8.5
```

- [ ] **Step 6: Record verification and clean build artifacts**

Update:

```text
docs/extraction-status.md
docs/reports/2026-07-14-yunxi-agent-v1-8-5-jsonl-redaction-development-report.md
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

- [ ] **Step 7: Commit and publish through GitHub REST API**

Commit message:

```text
Release YunXi Agent v1.8.5 JSONL redaction
```

Release requirements:

- Use GitHub REST API only.
- Verify remote `master` still equals the local parent commit before updating.
- Upload blobs from Git object contents, not working-tree bytes, to avoid Windows CRLF mismatches.
- Fast-forward `refs/heads/master`.
- Create annotated tag `v1.8.5`.
- Verify remote `master`, tag ref, and tag target through GitHub REST API.
- Materialize API commit/tag objects locally if their SHA differs from the local Git-created object.
- Update local `master`, `refs/remotes/origin/master`, and `refs/tags/v1.8.5`.

Expected final state:

```text
git status --short --branch => ## master...origin/master
v1.8.5 tag target equals remote master
old tags remain untouched
```

## Self-Review

- Spec coverage: The audit's only remaining P2 issue is JSONL transcript leakage. Tasks 1-3 add a JSONL output sanitizer and black-box tests for prompt/assistant echo leakage while preserving memory discard. Task 4 handles version/docs. Task 5 handles unified verification, cleanup, logging, and API release.
- Placeholder scan: No task uses placeholder wording. Each code-changing task names exact files and concrete helper/function signatures.
- Type consistency: The helper signatures use current `yunxi_agent_protocol` types: `RuntimeEvent`, `StreamEvent`, `ResponseItem`, `ResponseItemDelta`, `ToolCall`, `FunctionCallOutput`, and `ContentItem`.

## Unified Verification Results

Unified verification was performed after source construction, following the
project hard constraint.

Verified in this slice:

- `cargo fmt`: pass
- `cargo fmt --check`: pass
- `cargo test -p yunxi-agent-cli jsonl --test jsonl_tests`: pass; JSONL tests
  include the new secret-like transcript redaction regression
- `cargo test -p yunxi-agent-cli cli_jsonl_redacts_secret_prompt_while_memory_discards_candidate --test cli_tests`: pass
- targeted package tests for persona/storage/runtime/cli/tui: pass
- `cargo test`: pass; workspace unit tests, integration tests, and doc tests
  completed with zero failures
- `cargo check --workspace`: pass
- `cargo build -p yunxi-agent-cli --release --bins`: pass
- `target\release\yunxi.exe --version`: pass; `yunxi 1.8.5`
- `target\release\yunxi-agent-cli.exe --version`: pass; `yunxi 1.8.5`
- isolated `YUNXI_HOME` black-box JSONL privacy check: pass; fake secret was
  absent from JSONL, `[redacted]`, `[redacted-sensitive-query]`, and
  `memory_write action=discard` were present
- `git diff --check`: pass
- owned-source secret scan excluding `target`, `vendor`, `extracted`, `.git`,
  `.codegraph`, and `Cargo.lock`: pass; no live key-shaped matches
- `codegraph sync .`: pass; 9 changed files synced
- `codegraph status .`: pass; index is up to date
- `scripts\install\install-yunxi.ps1 -AddToPath -SkipBuild`: pass
- PATH smoke: pass; `yunxi --version` and `yunxi-agent-cli --version` both
  returned `yunxi 1.8.5`

## Completion Definition

YunXi Agent v1.8.5 is complete only when:

- JSONL user `item.message.content` no longer contains secret-like prompt fragments.
- JSONL assistant offline echo no longer contains secret-like prompt fragments.
- JSONL stream items, deltas, tool arguments/output, approval/escalation reasons, child-agent messages, and nested JSON function output pass through the same redaction helper.
- Memory recall query remains redacted for sensitive prompts.
- Secret-like memory candidates still emit `memory_write action=discard` and are not persisted.
- Ordinary non-secret prompts remain visible in JSONL so automation compatibility is preserved.
- Full unified verification passes after construction.
- CodeGraph is up to date.
- Build artifacts are cleaned.
- Desktop development log is updated.
- GitHub REST API publish succeeds and `v1.8.5` exists as an immutable rollback tag.

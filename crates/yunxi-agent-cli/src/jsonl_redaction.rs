use std::collections::BTreeMap;

use serde_json::Value;
use yunxi_agent_core::{AgentEvent, AgentRunResult, TodoStatus};
use yunxi_agent_protocol::{
    ContentItem, FunctionCallOutput, ResponseItem, ResponseItemDelta, RuntimeEvent, StreamEvent,
    ToolCall,
};

pub(crate) fn redact_agent_run_result_for_json(mut result: AgentRunResult) -> AgentRunResult {
    result.final_response = result.final_response.map(redact_text);
    result.events = result
        .events
        .into_iter()
        .map(redact_agent_event_for_json)
        .collect();
    result
}

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
        RuntimeEvent::ApprovalCacheState {
            thread_id,
            turn_id,
            session_id,
            tool_name,
            key,
            decision,
            reused,
        } => RuntimeEvent::ApprovalCacheState {
            thread_id,
            turn_id,
            session_id,
            tool_name,
            key: redact_text(key),
            decision,
            reused,
        },
        RuntimeEvent::ThreadState {
            thread_id,
            mut state,
        } => {
            state.data = redact_string_map(state.data);
            RuntimeEvent::ThreadState { thread_id, state }
        }
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
        AgentEvent::Message { content, stream } => AgentEvent::Message {
            content: redact_text(content),
            stream,
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
            ..
        } => AgentEvent::CommandCompleted {
            id,
            command: redact_text(command),
            aggregated_output: redact_text(aggregated_output),
            exit_code,
            status,
            execution_details: None,
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
            metadata,
            delta,
        } => StreamEvent::ItemDelta {
            thread_id,
            turn_id,
            metadata,
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
        ResponseItemDelta::ToolCallName { call_id, name } => {
            ResponseItemDelta::ToolCallName {
                call_id,
                name: redact_text(name),
            }
        }
        ResponseItemDelta::ToolOutput { call_id, delta } => ResponseItemDelta::ToolOutput {
            call_id,
            delta: redact_text(delta),
        },
        other => other,
    }
}

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
        ToolCall::ViewImage { id, path } => ToolCall::ViewImage {
            id,
            path: redact_text(path),
        },
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
        ContentItem::InputImage { image_url, detail } => ContentItem::InputImage {
            image_url: redact_text(image_url),
            detail: detail.map(redact_text),
        },
        ContentItem::LocalImage { path } => ContentItem::LocalImage {
            path: redact_text(path),
        },
    }
}

fn redact_json_value(value: Value) -> Value {
    match value {
        Value::String(value) => Value::String(redact_text(value)),
        Value::Array(values) => Value::Array(values.into_iter().map(redact_json_value).collect()),
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;
    use yunxi_agent_core::{
        AgentEvent, AgentRunResult, AgentRunStatus, CommandStatus, ThreadRuntimeState, TodoStatus,
    };
    use yunxi_agent_protocol::{
        ContentItem, FunctionCallOutput, MessagePhase, ProtocolRole, ResponseItem,
        ResponseItemDelta, RuntimeEvent, StreamEvent, ThreadId, ThreadState, ToolCall,
        ToolCallStatus, TurnId,
    };

    use super::{redact_agent_run_result_for_json, redact_runtime_event_for_jsonl};

    fn fake_secret() -> String {
        format!("{}{}", "sk-", "a".repeat(32))
    }

    fn event_text(event: &RuntimeEvent) -> String {
        serde_json::to_string(event).expect("event json")
    }

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
                    stream: None,
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
                    execution_details: None,
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
}

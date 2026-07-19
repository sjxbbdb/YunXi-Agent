use serde_json::Value;
use yunxi_agent_core::{
    AgentError, AgentEvent, AgentResult, AgentRunStatus, CommandStatus, FileChangeKind,
    McpToolStatus, PatchStatus, TokenUsage,
};

pub fn map_exec_jsonl(input: &str) -> AgentResult<Vec<AgentEvent>> {
    let mut events = Vec::new();

    for line in input.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let value: Value =
            serde_json::from_str(line).map_err(|err| AgentError::MalformedUpstreamEvent {
                message: err.to_string(),
            })?;
        events.extend(map_exec_json_event(value)?);
    }

    Ok(events)
}

pub fn map_exec_json_event(value: Value) -> AgentResult<Vec<AgentEvent>> {
    let event_type = value.get("type").and_then(Value::as_str).ok_or_else(|| {
        AgentError::MalformedUpstreamEvent {
            message: "event is missing type".to_string(),
        }
    })?;

    match event_type {
        "thread.started" => Ok(vec![AgentEvent::ThreadStarted {
            thread_id: required_string(&value, "thread_id")?,
        }]),
        "turn.started" => Ok(vec![AgentEvent::TurnStarted]),
        "turn.completed" => Ok(vec![AgentEvent::Completed {
            status: AgentRunStatus::Completed,
            usage: Some(TokenUsage {
                input_tokens: required_i64(&value["usage"], "input_tokens")?,
                cached_input_tokens: required_i64(&value["usage"], "cached_input_tokens")?,
                output_tokens: required_i64(&value["usage"], "output_tokens")?,
                reasoning_output_tokens: required_i64(&value["usage"], "reasoning_output_tokens")?,
            }),
        }]),
        "turn.failed" => Ok(vec![AgentEvent::Error {
            message: value["error"]["message"]
                .as_str()
                .unwrap_or("turn failed")
                .to_string(),
        }]),
        "warning" => Ok(vec![AgentEvent::Warning {
            message: required_string(&value, "message")?,
        }]),
        "error" => Ok(vec![AgentEvent::Error {
            message: required_string(&value, "message")?,
        }]),
        "item.started" => map_item(value.get("item"), ItemPhase::Started),
        "item.updated" => map_item(value.get("item"), ItemPhase::Updated),
        "item.completed" => map_item(value.get("item"), ItemPhase::Completed),
        _ => Ok(Vec::new()),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ItemPhase {
    Started,
    Updated,
    Completed,
}

fn map_item(item: Option<&Value>, phase: ItemPhase) -> AgentResult<Vec<AgentEvent>> {
    let item = item.ok_or_else(|| AgentError::MalformedUpstreamEvent {
        message: "item event is missing item".to_string(),
    })?;
    let id = item.get("id").and_then(Value::as_str).map(str::to_string);
    let item_type = item.get("type").and_then(Value::as_str).ok_or_else(|| {
        AgentError::MalformedUpstreamEvent {
            message: "item is missing type".to_string(),
        }
    })?;

    match item_type {
        "agent_message" => Ok(vec![AgentEvent::Message {
            content: required_string(item, "text")?,
            stream: None,
        }]),
        "reasoning" => Ok(vec![AgentEvent::Reasoning {
            content: required_string(item, "text")?,
        }]),
        "command_execution" => match phase {
            ItemPhase::Started => Ok(vec![AgentEvent::CommandStarted {
                id,
                command: required_string(item, "command")?,
            }]),
            ItemPhase::Updated => Ok(vec![AgentEvent::CommandUpdated {
                id,
                command: required_string(item, "command")?,
                aggregated_output: item
                    .get("aggregated_output")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            }]),
            ItemPhase::Completed => Ok(vec![AgentEvent::CommandCompleted {
                id,
                command: required_string(item, "command")?,
                aggregated_output: item
                    .get("aggregated_output")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                exit_code: item
                    .get("exit_code")
                    .and_then(Value::as_i64)
                    .map(|code| code as i32),
                status: map_command_status(item.get("status").and_then(Value::as_str)),
            }]),
        },
        "file_change" => {
            let mut events = Vec::new();
            for change in item
                .get("changes")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                events.push(AgentEvent::FileChanged {
                    path: required_string(change, "path")?,
                    kind: map_file_change_kind(change.get("kind").and_then(Value::as_str)),
                });
            }
            let status = match phase {
                ItemPhase::Completed => {
                    map_patch_status(item.get("status").and_then(Value::as_str))
                }
                ItemPhase::Started | ItemPhase::Updated => PatchStatus::InProgress,
            };
            events.push(AgentEvent::PatchCompleted { status });
            Ok(events)
        }
        "mcp_tool_call" => match phase {
            ItemPhase::Started => Ok(vec![AgentEvent::McpToolStarted {
                id,
                server: required_string(item, "server")?,
                tool: required_string(item, "tool")?,
            }]),
            ItemPhase::Updated | ItemPhase::Completed => Ok(vec![AgentEvent::McpToolCompleted {
                id,
                server: required_string(item, "server")?,
                tool: required_string(item, "tool")?,
                status: map_mcp_status(item.get("status").and_then(Value::as_str)),
            }]),
        },
        "todo_list" => {
            let items = item
                .get("items")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|entry| {
                    Ok(yunxi_agent_core::TodoStatus {
                        text: required_string(entry, "text")?,
                        completed: entry
                            .get("completed")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                    })
                })
                .collect::<AgentResult<Vec<_>>>()?;

            Ok(vec![AgentEvent::TodoUpdated { id, items }])
        }
        _ => Ok(Vec::new()),
    }
}

fn required_string(value: &Value, key: &str) -> AgentResult<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| AgentError::MalformedUpstreamEvent {
            message: format!("missing string field `{key}`"),
        })
}

fn required_i64(value: &Value, key: &str) -> AgentResult<i64> {
    value
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| AgentError::MalformedUpstreamEvent {
            message: format!("missing integer field `{key}`"),
        })
}

fn map_command_status(value: Option<&str>) -> CommandStatus {
    match value {
        Some("completed") => CommandStatus::Completed,
        Some("failed") => CommandStatus::Failed,
        Some("declined") => CommandStatus::Declined,
        _ => CommandStatus::InProgress,
    }
}

fn map_file_change_kind(value: Option<&str>) -> FileChangeKind {
    match value {
        Some("add") => FileChangeKind::Add,
        Some("delete") => FileChangeKind::Delete,
        _ => FileChangeKind::Update,
    }
}

fn map_patch_status(value: Option<&str>) -> PatchStatus {
    match value {
        Some("completed") => PatchStatus::Completed,
        Some("failed") => PatchStatus::Failed,
        _ => PatchStatus::InProgress,
    }
}

fn map_mcp_status(value: Option<&str>) -> McpToolStatus {
    match value {
        Some("completed") => McpToolStatus::Completed,
        Some("failed") => McpToolStatus::Failed,
        _ => McpToolStatus::InProgress,
    }
}

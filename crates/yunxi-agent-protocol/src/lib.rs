use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolVersion {
    V1,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ThreadId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TurnId(pub String);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolRole {
    System,
    Developer,
    User,
    Assistant,
    Tool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputItem {
    Message {
        role: ProtocolRole,
        content: String,
    },
    ToolResult {
        call_id: Option<String>,
        content: String,
    },
    LocalContext {
        name: String,
        content: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseItem {
    Message {
        role: ProtocolRole,
        content: String,
    },
    Reasoning {
        content: String,
    },
    ToolCall {
        call: ToolCall,
    },
    Usage {
        input_tokens: i64,
        cached_input_tokens: i64,
        output_tokens: i64,
        reasoning_output_tokens: i64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ToolCall {
    Shell {
        id: Option<String>,
        command: String,
    },
    Patch {
        id: Option<String>,
        patch: String,
    },
    Mcp {
        id: Option<String>,
        server: String,
        tool: String,
        arguments_json: Option<String>,
    },
    Skill {
        id: Option<String>,
        name: String,
        arguments_json: Option<String>,
    },
    MultiAgent {
        id: Option<String>,
        action: String,
        arguments_json: Option<String>,
    },
    ToolSearch {
        id: Option<String>,
        query: String,
    },
    RequestUserInput {
        id: Option<String>,
        prompt: String,
    },
    ViewImage {
        id: Option<String>,
        path: String,
    },
}

impl ToolCall {
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::Shell { id, .. }
            | Self::Patch { id, .. }
            | Self::Mcp { id, .. }
            | Self::Skill { id, .. }
            | Self::MultiAgent { id, .. }
            | Self::ToolSearch { id, .. }
            | Self::RequestUserInput { id, .. }
            | Self::ViewImage { id, .. } => id.as_deref(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuntimeEvent {
    ThreadStarted {
        thread_id: ThreadId,
    },
    TurnStarted {
        thread_id: ThreadId,
        turn_id: TurnId,
    },
    Item {
        thread_id: ThreadId,
        turn_id: TurnId,
        item: ResponseItem,
    },
    ToolStarted {
        thread_id: ThreadId,
        turn_id: TurnId,
        call: ToolCall,
    },
    ToolCompleted {
        thread_id: ThreadId,
        turn_id: TurnId,
        call_id: Option<String>,
        output: String,
        success: bool,
    },
    TurnCompleted {
        thread_id: ThreadId,
        turn_id: TurnId,
    },
    Error {
        thread_id: Option<ThreadId>,
        turn_id: Option<TurnId>,
        message: String,
    },
}

pub fn to_jsonl_line(event: &RuntimeEvent) -> Result<String, serde_json::Error> {
    serde_json::to_string(event)
}

pub fn from_jsonl_line(line: &str) -> Result<RuntimeEvent, serde_json::Error> {
    serde_json::from_str(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_event_round_trips_jsonl() {
        let event = RuntimeEvent::ToolStarted {
            thread_id: ThreadId("thread-1".to_string()),
            turn_id: TurnId("turn-1".to_string()),
            call: ToolCall::Shell {
                id: Some("call-1".to_string()),
                command: "echo yunxi".to_string(),
            },
        };

        let line = to_jsonl_line(&event).expect("jsonl");
        let parsed = from_jsonl_line(&line).expect("parsed event");

        assert_eq!(parsed, event);
    }
}

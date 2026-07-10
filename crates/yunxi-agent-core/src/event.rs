use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AgentEvent {
    Started {
        prompt: String,
    },
    ThreadStarted {
        thread_id: String,
    },
    TurnStarted,
    Message {
        content: String,
    },
    Reasoning {
        content: String,
    },
    CommandStarted {
        id: Option<String>,
        command: String,
    },
    CommandUpdated {
        id: Option<String>,
        command: String,
        aggregated_output: String,
    },
    CommandCompleted {
        id: Option<String>,
        command: String,
        aggregated_output: String,
        exit_code: Option<i32>,
        status: CommandStatus,
    },
    CommandFinished {
        command: String,
        exit_code: i32,
    },
    FileChanged {
        path: String,
        kind: FileChangeKind,
    },
    PatchCompleted {
        status: PatchStatus,
    },
    McpToolStarted {
        id: Option<String>,
        server: String,
        tool: String,
    },
    McpToolCompleted {
        id: Option<String>,
        server: String,
        tool: String,
        status: McpToolStatus,
    },
    ToolCallStarted {
        id: Option<String>,
        name: String,
        arguments_json: Option<String>,
    },
    ToolCallCompleted {
        id: Option<String>,
        name: String,
        output: String,
        status: CommandStatus,
    },
    ApprovalRequested {
        id: Option<String>,
        tool_name: String,
        reason: String,
    },
    ApprovalCompleted {
        id: Option<String>,
        approved: bool,
        reason: Option<String>,
    },
    EscalationRequested {
        id: Option<String>,
        tool_name: String,
        reason: String,
        required_sandbox: Option<String>,
        required_network: Option<String>,
    },
    EscalationCompleted {
        id: Option<String>,
        approved: bool,
        reason: Option<String>,
    },
    McpSession {
        server: String,
        status: String,
        message: Option<String>,
    },
    MultiAgentEvent {
        agent_id: String,
        parent_agent_id: Option<String>,
        status: String,
        message: Option<String>,
    },
    ChildAgentEvent {
        agent_id: String,
        child_session_id: String,
        parent_session_id: Option<String>,
        status: String,
        message: Option<String>,
    },
    ContextStatus {
        active_context_tokens: i64,
        token_limit_reached: bool,
        compacted: bool,
        dropped_messages: usize,
    },
    StorageState {
        session_id: Option<String>,
        parent_session_id: Option<String>,
        rollout_items: usize,
        rollout_truncated: bool,
        #[serde(default)]
        child_session_ids: Vec<String>,
    },
    TodoUpdated {
        id: Option<String>,
        items: Vec<TodoStatus>,
    },
    Warning {
        message: String,
    },
    Error {
        message: String,
    },
    Completed {
        status: AgentRunStatus,
        usage: Option<TokenUsage>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentRunStatus {
    Completed,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    InProgress,
    Completed,
    Failed,
    Declined,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileChangeKind {
    Add,
    Delete,
    Update,
    Move,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchStatus {
    InProgress,
    Completed,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpToolStatus {
    InProgress,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TodoStatus {
    pub text: String,
    pub completed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_output_tokens: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentRunResult {
    pub status: AgentRunStatus,
    pub final_response: Option<String>,
    pub events: Vec<AgentEvent>,
}

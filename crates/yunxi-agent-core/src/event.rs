use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AgentEvent {
    Started { prompt: String },
    Message { content: String },
    CommandStarted { command: String },
    CommandFinished { command: String, exit_code: i32 },
    FileChanged { path: String },
    Error { message: String },
    Completed { status: AgentRunStatus },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentRunStatus {
    Completed,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentRunResult {
    pub status: AgentRunStatus,
    pub final_response: Option<String>,
    pub events: Vec<AgentEvent>,
}

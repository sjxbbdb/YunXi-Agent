use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type DeepParityData = BTreeMap<String, String>;

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
    ThreadState {
        state: ThreadRuntimeState,
    },
    TurnMetadata {
        metadata: TurnRuntimeMetadata,
    },
    TurnState {
        state: TurnRuntimeState,
    },
    DeepParityState {
        layer: String,
        status: String,
        message: Option<String>,
        data: DeepParityData,
    },
    SandboxAttempt {
        id: Option<String>,
        #[serde(default)]
        schema_version: u32,
        platform: String,
        status: String,
        backend: String,
        #[serde(default)]
        backend_id: String,
        #[serde(default)]
        backend_label: String,
        #[serde(default)]
        os_isolation: bool,
        #[serde(default)]
        enforcement: String,
        #[serde(default)]
        enforcement_level: String,
        #[serde(default)]
        runner: String,
        #[serde(default)]
        unsupported_reason: Option<String>,
        command: Option<String>,
        cwd: String,
        message: Option<String>,
    },
    ApprovalCacheState {
        session_id: Option<String>,
        tool_name: String,
        key: String,
        decision: String,
        reused: bool,
    },
    PersonaLoaded {
        #[serde(default)]
        schema_version: u32,
        profile_id: String,
        display_name: String,
        enabled: bool,
    },
    PersonaContextInjected {
        #[serde(default)]
        schema_version: u32,
        profile_id: String,
        memory_count: usize,
        budget_used_chars: usize,
        budget_limit_chars: usize,
    },
    MemoryRecall {
        #[serde(default)]
        schema_version: u32,
        enabled: bool,
        scope: String,
        query: String,
        count: usize,
        budget_used_chars: usize,
        truncated: bool,
        #[serde(default)]
        always_on_count: usize,
        #[serde(default)]
        dropped_unrelated: usize,
        #[serde(default)]
        dropped_by_budget: usize,
        #[serde(default)]
        dropped_duplicates: usize,
    },
    MemoryCandidate {
        #[serde(default)]
        schema_version: u32,
        id: String,
        kind: String,
        sensitivity: String,
        status: String,
        write_policy: String,
        reason: String,
    },
    MemoryWrite {
        #[serde(default)]
        schema_version: u32,
        id: String,
        scope: String,
        kind: String,
        status: String,
        action: String,
        #[serde(default)]
        revision: u32,
        #[serde(default)]
        merged_count: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        merge_strategy: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        conflict_family: Option<String>,
    },
    MemoryWarning {
        #[serde(default)]
        schema_version: u32,
        warning: String,
    },
    Message {
        content: String,
        /// Structured provider-stream identity used by interactive renderers.
        ///
        /// This metadata is deliberately kept out of the serialized `AgentEvent`
        /// contract so plain, JSON, and JSONL consumers retain their v2.0.1 wire
        /// shape. Events deserialized from that contract behave as legacy message
        /// deltas with no source identity.
        #[serde(skip)]
        stream: Option<AgentMessageStream>,
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
    ChildScopedStream {
        agent_id: String,
        child_session_id: String,
        parent_session_id: Option<String>,
        event: String,
        seq: usize,
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
    Cancelled {
        reason: Option<String>,
    },
    ProviderError {
        provider: String,
        status: Option<u16>,
        classification: String,
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
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentMessageStreamPhase {
    Started,
    Delta,
    Final,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentMessageStream {
    pub thread_id: String,
    pub turn_id: String,
    pub stream_id: String,
    pub source_sequence: u64,
    pub phase: AgentMessageStreamPhase,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ThreadRuntimeState {
    pub thread_id: String,
    pub session_id: Option<String>,
    pub parent_thread_id: Option<String>,
    pub status: String,
    pub cwd: String,
    pub resume_source: Option<String>,
    pub child_depth: usize,
    #[serde(default)]
    pub data: DeepParityData,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TurnRuntimeMetadata {
    pub session_id: Option<String>,
    pub cwd: String,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub approval_mode: Option<String>,
    pub sandbox_mode: Option<String>,
    pub context_phase: Option<String>,
    pub resume_source: Option<String>,
    pub cancellation_state: Option<String>,
    pub child_depth: usize,
    #[serde(default)]
    pub data: DeepParityData,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TurnRuntimeState {
    pub phase: String,
    pub status: String,
    pub provider_status: String,
    pub tool_loop_status: String,
    pub cancellation_state: String,
    #[serde(default)]
    pub data: DeepParityData,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    InProgress,
    Completed,
    Failed,
    Declined,
    Cancelled,
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

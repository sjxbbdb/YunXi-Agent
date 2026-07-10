use async_trait::async_trait;
use std::path::Path;
use std::sync::{Arc, Mutex};
use yunxi_agent_context::load_agents_md_hierarchy;
use yunxi_agent_core::{
    AgentBackend, AgentConfig, AgentError, AgentEvent, AgentInput, AgentResult, AgentRunResult,
    AgentRunStatus, CommandStatus, FileChangeKind,
};
use yunxi_agent_provider::{
    AgentProvider, ProviderMessage, ProviderRequest, ProviderToolCall, StaticProvider,
};
use yunxi_agent_storage::{FileSessionStore, InMemorySessionStore, SessionRecord, SessionStore};
use yunxi_agent_tools::{
    ShellToolRuntime, ToolFileChangeKind, ToolPolicy, ToolRequest, ToolRequestKind, ToolRuntime,
    ToolStatus,
};

const DEFAULT_MAX_TURNS: usize = 8;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentTurn {
    pub config: AgentConfig,
    pub input: AgentInput,
}

impl AgentTurn {
    pub fn new(config: AgentConfig, input: AgentInput) -> Self {
        Self { config, input }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeContext {
    pub config: AgentConfig,
    pub max_turns: usize,
}

impl RuntimeContext {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config,
            max_turns: DEFAULT_MAX_TURNS,
        }
    }
}

#[async_trait]
pub trait RuntimeBackend: Send + Sync {
    async fn run_turn(&self, turn: AgentTurn) -> AgentResult<AgentRunResult>;
}

#[async_trait]
pub trait RuntimeEventSink: Send + Sync {
    async fn emit(&self, event: AgentEvent) -> AgentResult<()>;
    async fn events(&self) -> AgentResult<Vec<AgentEvent>>;
}

#[derive(Clone, Debug, Default)]
pub struct VecEventSink {
    events: Arc<Mutex<Vec<AgentEvent>>>,
}

impl VecEventSink {
    fn lock_events(&self) -> AgentResult<std::sync::MutexGuard<'_, Vec<AgentEvent>>> {
        self.events.lock().map_err(|_| AgentError::Execution {
            message: "runtime event sink lock was poisoned".to_string(),
        })
    }
}

#[async_trait]
impl RuntimeEventSink for VecEventSink {
    async fn emit(&self, event: AgentEvent) -> AgentResult<()> {
        self.lock_events()?.push(event);
        Ok(())
    }

    async fn events(&self) -> AgentResult<Vec<AgentEvent>> {
        Ok(self.lock_events()?.clone())
    }
}

#[derive(Clone)]
pub struct YunXiRuntimeBackend {
    provider: Arc<dyn AgentProvider>,
    tools: Arc<dyn ToolRuntime>,
    storage: Arc<dyn SessionStore>,
    max_turns: usize,
}

impl YunXiRuntimeBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_workspace(cwd: impl AsRef<Path>) -> Self {
        Self::with_parts(
            StaticProvider::default(),
            ShellToolRuntime,
            FileSessionStore::for_workspace(cwd),
        )
    }

    pub fn with_parts<P, T, S>(provider: P, tools: T, storage: S) -> Self
    where
        P: AgentProvider + 'static,
        T: ToolRuntime + 'static,
        S: SessionStore + 'static,
    {
        Self {
            provider: Arc::new(provider),
            tools: Arc::new(tools),
            storage: Arc::new(storage),
            max_turns: DEFAULT_MAX_TURNS,
        }
    }

    pub fn with_max_turns(mut self, max_turns: usize) -> Self {
        self.max_turns = max_turns.max(1);
        self
    }

    pub fn provider(&self) -> Arc<dyn AgentProvider> {
        Arc::clone(&self.provider)
    }

    pub fn tools(&self) -> Arc<dyn ToolRuntime> {
        Arc::clone(&self.tools)
    }

    pub fn storage(&self) -> Arc<dyn SessionStore> {
        Arc::clone(&self.storage)
    }
}

impl Default for YunXiRuntimeBackend {
    fn default() -> Self {
        Self::with_parts(
            StaticProvider::default(),
            ShellToolRuntime,
            InMemorySessionStore::default(),
        )
    }
}

#[async_trait]
impl RuntimeBackend for YunXiRuntimeBackend {
    async fn run_turn(&self, turn: AgentTurn) -> AgentResult<AgentRunResult> {
        let prompt = turn.input.prompt.trim();
        if prompt.is_empty() {
            return Err(AgentError::EmptyPrompt);
        }

        let sink = VecEventSink::default();
        sink.emit(AgentEvent::Started {
            prompt: prompt.to_string(),
        })
        .await?;

        let mut messages = build_initial_messages(&turn.config, prompt)?;
        let mut final_response = None;
        let mut usage = None;

        for _ in 0..self.max_turns {
            sink.emit(AgentEvent::Reasoning {
                content: "Provider turn started".to_string(),
            })
            .await?;
            let provider_response = self
                .provider
                .complete(ProviderRequest::with_messages(
                    turn.config.clone(),
                    AgentInput::text(prompt),
                    messages.clone(),
                ))
                .await?;
            usage = provider_response.usage;
            sink.emit(AgentEvent::Reasoning {
                content: "Provider turn completed".to_string(),
            })
            .await?;

            if provider_response.tool_calls.is_empty() {
                final_response = provider_response.message.map(|message| message.content);
                break;
            }

            if let Some(message) = provider_response.message {
                messages.push(message);
            }

            for tool_call in provider_response.tool_calls {
                let tool_request = map_tool_call(&turn.config, tool_call);
                emit_tool_started(&sink, &tool_request).await?;
                let tool_response = self.tools.execute(tool_request.clone()).await?;
                emit_tool_completed(&sink, &tool_request, &tool_response).await?;
                emit_tool_warning(&sink, &tool_response).await?;
                emit_file_changes(&sink, &tool_response).await?;
                messages.push(ProviderMessage::tool(render_tool_response(&tool_response)));
            }
        }

        let final_response = final_response.ok_or_else(|| AgentError::Execution {
            message: format!(
                "runtime did not produce a final response within {} turns",
                self.max_turns
            ),
        })?;

        sink.emit(AgentEvent::Message {
            content: final_response.clone(),
        })
        .await?;
        sink.emit(AgentEvent::Completed {
            status: AgentRunStatus::Completed,
            usage,
        })
        .await?;

        let events = sink.events().await?;
        let session_cwd = turn.config.cwd.clone();
        let session_model = turn.config.model.clone();
        let session_provider = turn.config.provider.clone();
        self.storage
            .save(
                SessionRecord::new(
                    session_cwd,
                    prompt,
                    Some(final_response.clone()),
                    events.clone(),
                )
                .with_status(AgentRunStatus::Completed)
                .with_model(session_model)
                .with_provider(session_provider),
            )
            .await?;

        Ok(AgentRunResult {
            status: AgentRunStatus::Completed,
            final_response: Some(final_response),
            events,
        })
    }
}

fn build_initial_messages(config: &AgentConfig, prompt: &str) -> AgentResult<Vec<ProviderMessage>> {
    let mut messages = Vec::new();
    let agents = load_agents_md_hierarchy(&config.cwd)?;
    let instructions = agents.combined_instructions();
    if !instructions.is_empty() {
        messages.push(ProviderMessage::system(instructions));
    }
    messages.push(ProviderMessage::user(prompt));
    Ok(messages)
}

fn map_tool_call(config: &AgentConfig, tool_call: ProviderToolCall) -> ToolRequest {
    let cwd = config.cwd.clone();
    let policy = ToolPolicy::from_config(config);
    match tool_call {
        ProviderToolCall::Shell { id, command } => ToolRequest {
            id,
            cwd,
            kind: ToolRequestKind::Shell { command },
            policy,
        },
        ProviderToolCall::Patch { id, patch } => ToolRequest {
            id,
            cwd,
            kind: ToolRequestKind::Patch { patch },
            policy,
        },
        ProviderToolCall::Mcp {
            id,
            server,
            tool,
            arguments_json,
        } => ToolRequest {
            id,
            cwd,
            kind: ToolRequestKind::Mcp {
                server,
                tool,
                arguments_json,
            },
            policy,
        },
        ProviderToolCall::Skill {
            id,
            name,
            arguments_json,
        } => ToolRequest {
            id,
            cwd,
            kind: ToolRequestKind::Skill {
                name,
                arguments_json,
            },
            policy,
        },
    }
}

async fn emit_tool_started<S>(sink: &S, request: &ToolRequest) -> AgentResult<()>
where
    S: RuntimeEventSink,
{
    match &request.kind {
        ToolRequestKind::Shell { command } => {
            sink.emit(AgentEvent::CommandStarted {
                id: request.id.clone(),
                command: command.clone(),
            })
            .await
        }
        ToolRequestKind::Patch { .. } => {
            sink.emit(AgentEvent::PatchCompleted {
                status: yunxi_agent_core::PatchStatus::InProgress,
            })
            .await
        }
        ToolRequestKind::Mcp { server, tool, .. } => {
            sink.emit(AgentEvent::McpToolStarted {
                id: request.id.clone(),
                server: server.clone(),
                tool: tool.clone(),
            })
            .await
        }
        ToolRequestKind::Skill { name, .. } => {
            sink.emit(AgentEvent::Reasoning {
                content: format!("Starting skill tool: {name}"),
            })
            .await
        }
    }
}

async fn emit_tool_completed<S>(
    sink: &S,
    request: &ToolRequest,
    response: &yunxi_agent_tools::ToolResponse,
) -> AgentResult<()>
where
    S: RuntimeEventSink,
{
    match &request.kind {
        ToolRequestKind::Shell { command } => {
            sink.emit(AgentEvent::CommandCompleted {
                id: response.id.clone(),
                command: command.clone(),
                aggregated_output: response.output.clone().unwrap_or_default(),
                exit_code: response.exit_code,
                status: map_tool_status(response.status),
            })
            .await
        }
        ToolRequestKind::Patch { .. } => {
            sink.emit(AgentEvent::PatchCompleted {
                status: match response.status {
                    ToolStatus::Completed => yunxi_agent_core::PatchStatus::Completed,
                    ToolStatus::InProgress => yunxi_agent_core::PatchStatus::InProgress,
                    ToolStatus::Failed | ToolStatus::Declined => {
                        yunxi_agent_core::PatchStatus::Failed
                    }
                },
            })
            .await
        }
        ToolRequestKind::Mcp { server, tool, .. } => {
            sink.emit(AgentEvent::McpToolCompleted {
                id: response.id.clone(),
                server: server.clone(),
                tool: tool.clone(),
                status: match response.status {
                    ToolStatus::Completed => yunxi_agent_core::McpToolStatus::Completed,
                    ToolStatus::InProgress => yunxi_agent_core::McpToolStatus::InProgress,
                    ToolStatus::Failed | ToolStatus::Declined => {
                        yunxi_agent_core::McpToolStatus::Failed
                    }
                },
            })
            .await
        }
        ToolRequestKind::Skill { name, .. } => {
            sink.emit(AgentEvent::Reasoning {
                content: format!("Completed skill tool: {name}"),
            })
            .await
        }
    }
}

fn map_tool_status(status: ToolStatus) -> CommandStatus {
    match status {
        ToolStatus::InProgress => CommandStatus::InProgress,
        ToolStatus::Completed => CommandStatus::Completed,
        ToolStatus::Failed => CommandStatus::Failed,
        ToolStatus::Declined => CommandStatus::Declined,
    }
}

async fn emit_tool_warning<S>(
    sink: &S,
    response: &yunxi_agent_tools::ToolResponse,
) -> AgentResult<()>
where
    S: RuntimeEventSink,
{
    if matches!(response.status, ToolStatus::Declined | ToolStatus::Failed) {
        if let Some(message) = response.error.as_ref().or(response.output.as_ref()) {
            sink.emit(AgentEvent::Warning {
                message: message.clone(),
            })
            .await?;
        }
    }
    Ok(())
}

async fn emit_file_changes<S>(
    sink: &S,
    response: &yunxi_agent_tools::ToolResponse,
) -> AgentResult<()>
where
    S: RuntimeEventSink,
{
    for change in &response.changed_files {
        sink.emit(AgentEvent::FileChanged {
            path: change.path.display().to_string(),
            kind: match change.kind {
                ToolFileChangeKind::Added => FileChangeKind::Add,
                ToolFileChangeKind::Deleted => FileChangeKind::Delete,
                ToolFileChangeKind::Updated => FileChangeKind::Update,
            },
        })
        .await?;
    }
    Ok(())
}

fn render_tool_response(response: &yunxi_agent_tools::ToolResponse) -> String {
    if let Some(output) = &response.output {
        return output.clone();
    }
    if let Some(error) = &response.error {
        return error.clone();
    }
    format!("tool completed with status {:?}", response.status)
}

#[async_trait]
impl AgentBackend for YunXiRuntimeBackend {
    async fn run(&self, config: AgentConfig, input: AgentInput) -> AgentResult<AgentRunResult> {
        self.run_turn(AgentTurn::new(config, input)).await
    }
}

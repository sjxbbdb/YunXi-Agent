use async_trait::async_trait;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use yunxi_agent_context::{
    ContextWindowBudget, ConversationMessage, ConversationRole, RestoredHistory,
    load_agents_md_hierarchy, restore_history_for_prompt,
};
use yunxi_agent_core::{
    AgentBackend, AgentConfig, AgentError, AgentEvent, AgentInput, AgentResult, AgentRunResult,
    AgentRunStatus, CommandStatus, FileChangeKind,
};
use yunxi_agent_provider::{
    AgentProvider, ProviderMessage, ProviderRequest, ProviderToolCall, StaticProvider,
};
use yunxi_agent_storage::{
    FileSessionStore, HistoryItemKind, HistoryLoadOptions, InMemorySessionStore, SessionHistory,
    SessionId, SessionRecord, SessionStore,
};
use yunxi_agent_tools::{
    ShellToolRuntime, ToolDispatchTrace, ToolFileChangeKind, ToolPolicy, ToolRequest,
    ToolRequestKind, ToolRouter, ToolRuntime, ToolStatus,
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
    tool_router: ToolRouter,
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
            tool_router: ToolRouter::default(),
            storage: Arc::new(storage),
            max_turns: DEFAULT_MAX_TURNS,
        }
    }

    pub fn with_max_turns(mut self, max_turns: usize) -> Self {
        self.max_turns = max_turns.max(1);
        self
    }

    pub fn with_tool_router(mut self, tool_router: ToolRouter) -> Self {
        self.tool_router = tool_router;
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
        let thread_id = generate_runtime_thread_id();
        sink.emit(AgentEvent::ThreadStarted {
            thread_id: thread_id.clone(),
        })
        .await?;
        sink.emit(AgentEvent::TurnStarted).await?;
        sink.emit(AgentEvent::Started {
            prompt: prompt.to_string(),
        })
        .await?;

        let initial_messages = self.build_initial_messages(&turn.config, prompt).await?;
        if let Some(history) = &initial_messages.restored_history {
            sink.emit(AgentEvent::Reasoning {
                content: format!(
                    "Restored {} history message(s); compacted={}",
                    history.messages.len(),
                    history.compacted
                ),
            })
            .await?;
        }
        let mut messages = initial_messages.messages;
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
                let dispatch = self.tool_router.route(tool_request)?;
                emit_tool_dispatch_trace(&sink, &dispatch.trace).await?;
                emit_tool_started(&sink, &dispatch.request).await?;
                let tool_response = self.tools.execute(dispatch.request.clone()).await?;
                emit_tool_completed(&sink, &dispatch.request, &tool_response).await?;
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
        let mut session = SessionRecord::new(
            session_cwd,
            prompt,
            Some(final_response.clone()),
            events.clone(),
        )
        .with_status(AgentRunStatus::Completed)
        .with_model(session_model)
        .with_provider(session_provider);
        if let Some(parent_session_id) = turn.config.parent_session_id.clone() {
            session = session.with_parent_id(SessionId::new(parent_session_id));
        }
        if let Some(session_title) = turn.config.session_title.clone() {
            session = session.with_title(session_title);
        }
        self.storage.save(session).await?;

        Ok(AgentRunResult {
            status: AgentRunStatus::Completed,
            final_response: Some(final_response),
            events,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct InitialMessages {
    messages: Vec<ProviderMessage>,
    restored_history: Option<RestoredHistory>,
}

impl YunXiRuntimeBackend {
    async fn build_initial_messages(
        &self,
        config: &AgentConfig,
        prompt: &str,
    ) -> AgentResult<InitialMessages> {
        let mut messages = Vec::new();
        let agents = load_agents_md_hierarchy(&config.cwd)?;
        let instructions = agents.combined_instructions();
        if !instructions.is_empty() {
            messages.push(ProviderMessage::system(instructions));
        }

        let restored_history = self.restore_parent_history(config).await?;
        if let Some(restored_history) = &restored_history {
            messages.extend(
                restored_history
                    .messages
                    .iter()
                    .cloned()
                    .map(conversation_message_to_provider),
            );
        }

        messages.push(ProviderMessage::user(prompt));
        Ok(InitialMessages {
            messages,
            restored_history,
        })
    }

    async fn restore_parent_history(
        &self,
        config: &AgentConfig,
    ) -> AgentResult<Option<RestoredHistory>> {
        let Some(parent_session_id) = &config.parent_session_id else {
            return Ok(None);
        };
        let Some(history) = self
            .storage
            .history(
                &SessionId::new(parent_session_id.clone()),
                HistoryLoadOptions::default(),
            )
            .await?
        else {
            return Ok(None);
        };
        let messages = session_history_to_conversation_messages(&history);
        let budget = ContextWindowBudget::new(
            config.context_window_tokens,
            config.auto_compact_threshold_tokens,
        );
        Ok(Some(restore_history_for_prompt(messages, budget)))
    }
}

fn session_history_to_conversation_messages(history: &SessionHistory) -> Vec<ConversationMessage> {
    history
        .items
        .iter()
        .map(|item| {
            let role = match item.kind {
                HistoryItemKind::User => ConversationRole::User,
                HistoryItemKind::Assistant => ConversationRole::Assistant,
            };
            ConversationMessage::new(role, item.content.clone())
        })
        .collect()
}

fn conversation_message_to_provider(message: ConversationMessage) -> ProviderMessage {
    match message.role {
        ConversationRole::System => ProviderMessage::system(message.content),
        ConversationRole::User => ProviderMessage::user(message.content),
        ConversationRole::Assistant => ProviderMessage::assistant(message.content),
        ConversationRole::Tool => ProviderMessage::tool(message.content),
    }
}

pub fn protocol_stream_events_to_agent_events(
    events: &[yunxi_agent_protocol::StreamEvent],
) -> Vec<AgentEvent> {
    let mut output = Vec::new();
    for event in events {
        match event {
            yunxi_agent_protocol::StreamEvent::ResponseStarted { thread_id, .. } => {
                output.push(AgentEvent::ThreadStarted {
                    thread_id: thread_id.0.clone(),
                });
                output.push(AgentEvent::TurnStarted);
            }
            yunxi_agent_protocol::StreamEvent::ItemStarted { item, .. }
            | yunxi_agent_protocol::StreamEvent::ItemCompleted { item, .. } => {
                push_response_item_agent_event(item, &mut output);
            }
            yunxi_agent_protocol::StreamEvent::ItemDelta { delta, .. } => match delta {
                yunxi_agent_protocol::ResponseItemDelta::MessageContent { delta, .. } => {
                    output.push(AgentEvent::Message {
                        content: delta.clone(),
                    });
                }
                yunxi_agent_protocol::ResponseItemDelta::ReasoningContent { delta, .. } => {
                    output.push(AgentEvent::Reasoning {
                        content: delta.clone(),
                    });
                }
                yunxi_agent_protocol::ResponseItemDelta::ToolCallArguments { call_id, delta } => {
                    output.push(AgentEvent::CommandUpdated {
                        id: call_id.clone(),
                        command: "tool_arguments".to_string(),
                        aggregated_output: delta.clone(),
                    })
                }
                yunxi_agent_protocol::ResponseItemDelta::ToolCallStatus { call_id, status } => {
                    output.push(AgentEvent::ToolCallCompleted {
                        id: call_id.clone(),
                        name: "tool".to_string(),
                        output: format!("tool status: {status:?}"),
                        status: match status {
                            yunxi_agent_protocol::ToolCallStatus::Completed => {
                                CommandStatus::Completed
                            }
                            yunxi_agent_protocol::ToolCallStatus::InProgress => {
                                CommandStatus::InProgress
                            }
                            yunxi_agent_protocol::ToolCallStatus::Failed
                            | yunxi_agent_protocol::ToolCallStatus::Cancelled => {
                                CommandStatus::Failed
                            }
                        },
                    });
                }
            },
            yunxi_agent_protocol::StreamEvent::ResponseCompleted { status, .. } => {
                output.push(AgentEvent::Completed {
                    status: match status {
                        yunxi_agent_protocol::ResponseStatus::Completed => {
                            AgentRunStatus::Completed
                        }
                        yunxi_agent_protocol::ResponseStatus::InProgress
                        | yunxi_agent_protocol::ResponseStatus::Failed
                        | yunxi_agent_protocol::ResponseStatus::Cancelled => AgentRunStatus::Failed,
                    },
                    usage: None,
                });
            }
            yunxi_agent_protocol::StreamEvent::ResponseFailed { message, .. } => {
                output.push(AgentEvent::Error {
                    message: message.clone(),
                });
            }
        }
    }
    output
}

fn push_response_item_agent_event(
    item: &yunxi_agent_protocol::ResponseItem,
    output: &mut Vec<AgentEvent>,
) {
    match item {
        yunxi_agent_protocol::ResponseItem::Message { content, .. } => {
            output.push(AgentEvent::Message {
                content: content.clone(),
            })
        }
        yunxi_agent_protocol::ResponseItem::AgentMessage { content, .. } => {
            if let Some(content) = first_content_text(content) {
                output.push(AgentEvent::Message { content });
            }
        }
        yunxi_agent_protocol::ResponseItem::Reasoning { content } => {
            output.push(AgentEvent::Reasoning {
                content: content.clone(),
            })
        }
        yunxi_agent_protocol::ResponseItem::ReasoningItem { summary_text, .. } => {
            if let Some(content) = summary_text.first() {
                output.push(AgentEvent::Reasoning {
                    content: content.clone(),
                });
            }
        }
        yunxi_agent_protocol::ResponseItem::ToolCall { call } => {
            output.push(tool_call_started_event(call.clone()));
        }
        yunxi_agent_protocol::ResponseItem::FunctionCall {
            call_id,
            name,
            arguments,
            ..
        } => output.push(AgentEvent::ToolCallStarted {
            id: Some(call_id.clone()),
            name: name.clone(),
            arguments_json: Some(arguments.clone()),
        }),
        yunxi_agent_protocol::ResponseItem::McpToolCall {
            call_id,
            server,
            tool,
            ..
        } => output.push(AgentEvent::McpToolStarted {
            id: Some(call_id.clone()),
            server: server.clone(),
            tool: tool.clone(),
        }),
        yunxi_agent_protocol::ResponseItem::Compaction { summary, .. } => {
            output.push(AgentEvent::Reasoning {
                content: summary.clone(),
            });
        }
        _ => {}
    }
}

fn first_content_text(items: &[yunxi_agent_protocol::ContentItem]) -> Option<String> {
    items.iter().find_map(|item| match item {
        yunxi_agent_protocol::ContentItem::InputText { text }
        | yunxi_agent_protocol::ContentItem::OutputText { text } => Some(text.clone()),
        yunxi_agent_protocol::ContentItem::InputImage { .. }
        | yunxi_agent_protocol::ContentItem::LocalImage { .. } => None,
    })
}

fn tool_call_started_event(call: yunxi_agent_protocol::ToolCall) -> AgentEvent {
    match call {
        yunxi_agent_protocol::ToolCall::Shell { id, command } => {
            AgentEvent::CommandStarted { id, command }
        }
        yunxi_agent_protocol::ToolCall::Patch { id, patch } => AgentEvent::ToolCallStarted {
            id,
            name: "patch".to_string(),
            arguments_json: Some(patch),
        },
        yunxi_agent_protocol::ToolCall::Mcp {
            id, server, tool, ..
        } => AgentEvent::McpToolStarted { id, server, tool },
        yunxi_agent_protocol::ToolCall::Skill {
            id,
            name,
            arguments_json,
        } => AgentEvent::ToolCallStarted {
            id,
            name,
            arguments_json,
        },
        yunxi_agent_protocol::ToolCall::MultiAgent {
            id,
            action,
            arguments_json,
        } => AgentEvent::ToolCallStarted {
            id,
            name: format!("multi_agent:{action}"),
            arguments_json,
        },
        yunxi_agent_protocol::ToolCall::ToolSearch { id, query } => AgentEvent::ToolCallStarted {
            id,
            name: "tool_search".to_string(),
            arguments_json: Some(format!(r#"{{"query":{}}}"#, json_string(&query))),
        },
        yunxi_agent_protocol::ToolCall::RequestUserInput { id, prompt } => {
            AgentEvent::ToolCallStarted {
                id,
                name: "request_user_input".to_string(),
                arguments_json: Some(format!(r#"{{"prompt":{}}}"#, json_string(&prompt))),
            }
        }
        yunxi_agent_protocol::ToolCall::ViewImage { id, path } => AgentEvent::ToolCallStarted {
            id,
            name: "view_image".to_string(),
            arguments_json: Some(format!(r#"{{"path":{}}}"#, json_string(&path))),
        },
    }
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
        ProviderToolCall::MultiAgent {
            id,
            action,
            arguments_json,
        } => ToolRequest {
            id,
            cwd,
            kind: ToolRequestKind::MultiAgent {
                action,
                arguments_json,
            },
            policy,
        },
        ProviderToolCall::ToolSearch { id, query } => ToolRequest {
            id,
            cwd,
            kind: ToolRequestKind::ToolSearch { query },
            policy,
        },
        ProviderToolCall::RequestUserInput { id, prompt } => ToolRequest {
            id,
            cwd,
            kind: ToolRequestKind::RequestUserInput { prompt },
            policy,
        },
        ProviderToolCall::ViewImage { id, path } => ToolRequest {
            id,
            cwd,
            kind: ToolRequestKind::ViewImage { path },
            policy,
        },
    }
}

async fn emit_tool_dispatch_trace<S>(sink: &S, trace: &ToolDispatchTrace) -> AgentResult<()>
where
    S: RuntimeEventSink,
{
    sink.emit(AgentEvent::Reasoning {
        content: trace.summary(),
    })
    .await
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
            sink.emit(AgentEvent::ToolCallStarted {
                id: request.id.clone(),
                name: "skill".to_string(),
                arguments_json: Some(format!(r#"{{"name":{}}}"#, json_string(name))),
            })
            .await
        }
        ToolRequestKind::MultiAgent {
            action,
            arguments_json,
        } => {
            sink.emit(AgentEvent::ToolCallStarted {
                id: request.id.clone(),
                name: "multi_agent".to_string(),
                arguments_json: Some(
                    arguments_json
                        .clone()
                        .unwrap_or_else(|| format!(r#"{{"action":{}}}"#, json_string(action))),
                ),
            })
            .await
        }
        ToolRequestKind::ToolSearch { query } => {
            sink.emit(AgentEvent::ToolCallStarted {
                id: request.id.clone(),
                name: "tool_search".to_string(),
                arguments_json: Some(format!(r#"{{"query":{}}}"#, json_string(query))),
            })
            .await
        }
        ToolRequestKind::RequestUserInput { prompt } => {
            sink.emit(AgentEvent::ToolCallStarted {
                id: request.id.clone(),
                name: "request_user_input".to_string(),
                arguments_json: Some(format!(r#"{{"prompt":{}}}"#, json_string(prompt))),
            })
            .await
        }
        ToolRequestKind::ViewImage { path } => {
            sink.emit(AgentEvent::ToolCallStarted {
                id: request.id.clone(),
                name: "view_image".to_string(),
                arguments_json: Some(format!(r#"{{"path":{}}}"#, json_string(path))),
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
            sink.emit(AgentEvent::ToolCallCompleted {
                id: response.id.clone(),
                name: format!("skill:{name}"),
                output: render_tool_response(response),
                status: map_tool_status(response.status),
            })
            .await
        }
        ToolRequestKind::MultiAgent { action, .. } => {
            sink.emit(AgentEvent::ToolCallCompleted {
                id: response.id.clone(),
                name: format!("multi_agent:{action}"),
                output: render_tool_response(response),
                status: map_tool_status(response.status),
            })
            .await
        }
        ToolRequestKind::ToolSearch { .. } => {
            sink.emit(AgentEvent::ToolCallCompleted {
                id: response.id.clone(),
                name: "tool_search".to_string(),
                output: render_tool_response(response),
                status: map_tool_status(response.status),
            })
            .await
        }
        ToolRequestKind::RequestUserInput { .. } => {
            sink.emit(AgentEvent::ToolCallCompleted {
                id: response.id.clone(),
                name: "request_user_input".to_string(),
                output: render_tool_response(response),
                status: map_tool_status(response.status),
            })
            .await
        }
        ToolRequestKind::ViewImage { .. } => {
            sink.emit(AgentEvent::ToolCallCompleted {
                id: response.id.clone(),
                name: "view_image".to_string(),
                output: render_tool_response(response),
                status: map_tool_status(response.status),
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
                ToolFileChangeKind::Moved => FileChangeKind::Move,
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

fn json_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

fn generate_runtime_thread_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    format!("yunxi-thread-{millis}")
}

#[async_trait]
impl AgentBackend for YunXiRuntimeBackend {
    async fn run(&self, config: AgentConfig, input: AgentInput) -> AgentResult<AgentRunResult> {
        self.run_turn(AgentTurn::new(config, input)).await
    }
}

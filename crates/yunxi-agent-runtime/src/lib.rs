use async_trait::async_trait;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use yunxi_agent_context::{
    ContextWindowBudget, ConversationMessage, ConversationRole, RestoredHistory,
    extract_file_mentions, load_agents_md_hierarchy, restore_history_for_prompt,
};
use yunxi_agent_core::{
    AgentBackend, AgentConfig, AgentError, AgentEvent, AgentInput, AgentResult, AgentRunResult,
    AgentRunStatus, CommandStatus, FileChangeKind, TokenUsage,
};
use yunxi_agent_exec::{ExecLifecycleEvent, ExecOutputStream};
use yunxi_agent_multi_agent::{
    AgentId, AgentStatus, ChildAgentRunRequest, ChildAgentRunResult, ChildAgentRuntime,
    InMemoryAgentRegistry, MultiAgentCommand, MultiAgentCommandResult,
};
use yunxi_agent_protocol::{
    ProtocolRole, ResponseItem, ResponseItemDelta, ResponseStatus, StreamEvent, ThreadId, ToolCall,
    TurnId,
};
use yunxi_agent_provider::{
    AgentProvider, ProviderBootstrap, ProviderMessage, ProviderRequest, ProviderResponse,
    ProviderRole, ProviderStream, ProviderToolCall, StaticProvider,
};
use yunxi_agent_sandbox::ApprovalRequirement;
use yunxi_agent_storage::{
    FileSessionStore, HistoryItemKind, HistoryLoadOptions, InMemorySessionStore, SessionHistory,
    SessionId, SessionRecord, SessionStore,
};
use yunxi_agent_tools::{
    ApprovalDecision, CompositeToolRuntime, ToolDispatchTrace, ToolFileChangeKind, ToolPolicy,
    ToolRequest, ToolRequestKind, ToolResponse, ToolRouter, ToolRuntime, ToolRuntimeEvent,
    ToolStatus,
};

const DEFAULT_MAX_TURNS: usize = 8;
const DEFAULT_MAX_CHILD_DEPTH: usize = 2;
const MAX_MENTIONED_FILE_CONTEXT_FILES: usize = 8;
const MAX_MENTIONED_FILE_CONTEXT_BYTES: u64 = 32 * 1024;

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
    agents: InMemoryAgentRegistry,
    max_turns: usize,
    max_child_depth: usize,
}

impl YunXiRuntimeBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_workspace(cwd: impl AsRef<Path>) -> Self {
        Self::with_parts(
            StaticProvider::default(),
            CompositeToolRuntime::default(),
            FileSessionStore::for_workspace(cwd),
        )
    }

    pub fn for_workspace_with_live_provider(cwd: impl AsRef<Path>, config: &AgentConfig) -> Self {
        let provider =
            ProviderBootstrap::from_agent_config(config).into_openai_transport_provider();
        Self::with_parts(
            provider,
            CompositeToolRuntime::default(),
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
            agents: InMemoryAgentRegistry::default(),
            max_turns: DEFAULT_MAX_TURNS,
            max_child_depth: DEFAULT_MAX_CHILD_DEPTH,
        }
    }

    pub fn with_shared_parts(
        provider: Arc<dyn AgentProvider>,
        tools: Arc<dyn ToolRuntime>,
        storage: Arc<dyn SessionStore>,
    ) -> Self {
        Self {
            provider,
            tools,
            tool_router: ToolRouter::default(),
            storage,
            agents: InMemoryAgentRegistry::default(),
            max_turns: DEFAULT_MAX_TURNS,
            max_child_depth: DEFAULT_MAX_CHILD_DEPTH,
        }
    }

    pub fn with_max_turns(mut self, max_turns: usize) -> Self {
        self.max_turns = max_turns.max(1);
        self
    }

    pub fn with_max_child_depth(mut self, max_child_depth: usize) -> Self {
        self.max_child_depth = max_child_depth;
        self
    }

    pub fn with_tool_router(mut self, tool_router: ToolRouter) -> Self {
        self.tool_router = tool_router;
        self
    }

    pub fn with_agent_registry(mut self, registry: InMemoryAgentRegistry) -> Self {
        self.agents = registry;
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

    async fn execute_tool_request(
        &self,
        config: &AgentConfig,
        request: ToolRequest,
    ) -> AgentResult<ToolResponse> {
        match &request.kind {
            ToolRequestKind::MultiAgent {
                action,
                arguments_json,
            } => self.execute_multi_agent_tool(config, request.id.clone(), action, arguments_json),
            _ => self.tools.execute(request).await,
        }
    }

    fn execute_multi_agent_tool(
        &self,
        config: &AgentConfig,
        id: Option<String>,
        action: &str,
        arguments_json: &Option<String>,
    ) -> AgentResult<ToolResponse> {
        let command = parse_runtime_multi_agent_command(action, arguments_json.as_deref())?;
        let child_runtime = YunXiChildAgentRuntime::new(
            config.clone(),
            Arc::clone(&self.storage),
            self.max_turns,
            self.max_child_depth,
        );
        let result = self
            .agents
            .execute_with_child_runtime(command, &child_runtime)?;
        let runtime_events = runtime_multi_agent_events(&result);
        let output = serde_json::to_string(&result).map_err(|error| AgentError::Execution {
            message: format!("failed to serialize multi-agent result: {error}"),
        })?;
        let response = if matches!(result.status, AgentStatus::Failed) {
            ToolResponse::failed(id, output, None, Vec::new())
        } else {
            ToolResponse::completed(id, output, Some(0), Vec::new())
        };
        Ok(response.with_runtime_events(runtime_events))
    }
}

impl Default for YunXiRuntimeBackend {
    fn default() -> Self {
        Self::with_parts(
            StaticProvider::default(),
            CompositeToolRuntime::default(),
            InMemorySessionStore::default(),
        )
    }
}

#[derive(Clone)]
struct YunXiChildAgentRuntime {
    base_config: AgentConfig,
    storage: Arc<dyn SessionStore>,
    max_turns: usize,
    remaining_depth: usize,
}

impl YunXiChildAgentRuntime {
    fn new(
        base_config: AgentConfig,
        storage: Arc<dyn SessionStore>,
        max_turns: usize,
        remaining_depth: usize,
    ) -> Self {
        Self {
            base_config,
            storage,
            max_turns,
            remaining_depth,
        }
    }

    fn failed_result(
        &self,
        request: ChildAgentRunRequest,
        message: impl Into<String>,
    ) -> ChildAgentRunResult {
        let message = message.into();
        ChildAgentRunResult {
            agent_id: request.agent.id,
            session_id: request.session_id,
            parent_session_id: request.parent_session_id,
            status: AgentRunStatus::Failed,
            final_response: None,
            events: vec![
                AgentEvent::Error {
                    message: message.clone(),
                },
                AgentEvent::Completed {
                    status: AgentRunStatus::Failed,
                    usage: None,
                },
            ],
        }
    }
}

impl ChildAgentRuntime for YunXiChildAgentRuntime {
    fn run_child(&self, request: ChildAgentRunRequest) -> AgentResult<ChildAgentRunResult> {
        if self.remaining_depth == 0 {
            return Ok(self.failed_result(
                request,
                "child runtime recursion depth exceeded for multi_agent spawn_run",
            ));
        }

        let mut child_config = self.base_config.clone();
        child_config.parent_session_id = request.parent_session_id.clone();
        child_config.session_id = Some(request.session_id.clone());
        child_config.session_title = Some(format!(
            "Child {}: {}",
            request.agent.id.0,
            preview_child_title(&request.prompt)
        ));

        let child_prompt = request.prompt.clone();
        let child_session_id = request.session_id.clone();
        let parent_session_id = request.parent_session_id.clone();
        let child_agent_id = request.agent.id.clone();
        let child_storage = Arc::clone(&self.storage);
        let max_turns = self.max_turns;
        let child_provider = Arc::new(StaticProvider::new(format!(
            "YunXi child agent {} completed task",
            child_agent_id.0
        )));
        let child_tools = Arc::new(CompositeToolRuntime::default());

        let handle = thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| AgentError::Execution {
                    message: format!("failed to build child runtime executor: {error}"),
                })?;
            let backend =
                YunXiRuntimeBackend::with_shared_parts(child_provider, child_tools, child_storage)
                    .with_max_turns(max_turns);
            runtime.block_on(RuntimeBackend::run_turn(
                &backend,
                AgentTurn::new(child_config, AgentInput::text(child_prompt)),
            ))
        });

        match handle.join().map_err(|_| AgentError::Execution {
            message: "child runtime executor thread panicked".to_string(),
        })? {
            Ok(run_result) => {
                let final_response = run_result.final_response.clone();
                let status = if run_result.status == AgentRunStatus::Completed
                    && final_response
                        .as_deref()
                        .is_some_and(|value| !value.is_empty())
                {
                    AgentRunStatus::Completed
                } else {
                    AgentRunStatus::Failed
                };
                Ok(ChildAgentRunResult {
                    agent_id: child_agent_id,
                    session_id: child_session_id,
                    parent_session_id,
                    status,
                    final_response,
                    events: run_result.events,
                })
            }
            Err(error) => Ok(self.failed_result(request, error.to_string())),
        }
    }
}

#[async_trait]
impl RuntimeBackend for YunXiRuntimeBackend {
    async fn run_turn(&self, turn: AgentTurn) -> AgentResult<AgentRunResult> {
        let prompt = turn.input.prompt.trim();
        if prompt.is_empty() {
            return Err(AgentError::EmptyPrompt);
        }

        let session_id = turn
            .config
            .session_id
            .clone()
            .map(SessionId::new)
            .unwrap_or_else(SessionId::generate);
        let mut runtime_config = turn.config.clone();
        runtime_config.session_id = Some(session_id.0.clone());

        let sink = VecEventSink::default();
        let thread_id = generate_runtime_thread_id();
        let turn_id = generate_runtime_turn_id();
        sink.emit(AgentEvent::ThreadStarted {
            thread_id: thread_id.clone(),
        })
        .await?;
        sink.emit(AgentEvent::TurnStarted).await?;
        sink.emit(AgentEvent::Started {
            prompt: prompt.to_string(),
        })
        .await?;

        let initial_messages = self.build_initial_messages(&runtime_config, prompt).await?;
        if let Some(history) = &initial_messages.restored_history {
            sink.emit(AgentEvent::Reasoning {
                content: format!(
                    "Restored {} history message(s); compacted={}",
                    history.messages.len(),
                    history.compacted
                ),
            })
            .await?;
            sink.emit(AgentEvent::ContextStatus {
                active_context_tokens: history.status.active_context_tokens,
                token_limit_reached: history.status.token_limit_reached,
                compacted: history.compacted,
                dropped_messages: history.dropped_messages,
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
            let provider_stream = self
                .provider
                .stream(
                    ProviderRequest::with_messages(
                        runtime_config.clone(),
                        AgentInput::text(prompt),
                        messages.clone(),
                    ),
                    ThreadId(thread_id.clone()),
                    TurnId(turn_id.clone()),
                )
                .await?;
            emit_provider_stream_events(&sink, &provider_stream.events).await?;
            let provider_response = collect_provider_response(provider_stream)?;
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
                let tool_request = map_tool_call(&runtime_config, &session_id, tool_call);
                let dispatch = self.tool_router.route(tool_request)?;
                emit_tool_dispatch_trace(&sink, &dispatch.trace).await?;
                emit_approval_requested_if_needed(&sink, &dispatch.trace).await?;
                emit_escalation_requested_if_needed(&sink, &dispatch.trace).await?;
                emit_tool_started(&sink, &dispatch.request).await?;
                let tool_response = self
                    .execute_tool_request(&runtime_config, dispatch.request.clone())
                    .await?;
                emit_tool_lifecycle_events(&sink, &tool_response).await?;
                emit_tool_runtime_events(&sink, &tool_response).await?;
                emit_tool_completed(&sink, &dispatch.request, &tool_response).await?;
                emit_approval_completed_if_needed(&sink, &dispatch.trace, &tool_response).await?;
                emit_escalation_completed_if_needed(&sink, &dispatch.trace, &tool_response).await?;
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

        let pre_storage_events = sink.events().await?;
        let session_cwd = runtime_config.cwd.clone();
        let session_model = runtime_config.model.clone();
        let session_provider = runtime_config.provider.clone();
        let child_session_ids = child_session_ids_from_agent_events(&pre_storage_events);
        let mut session = SessionRecord::new(
            session_cwd,
            prompt,
            Some(final_response.clone()),
            pre_storage_events.clone(),
        )
        .with_status(AgentRunStatus::Completed)
        .with_model(session_model)
        .with_provider(session_provider);
        session.id = session_id.clone();
        if let Some(parent_session_id) = runtime_config.parent_session_id.clone() {
            session = session.with_parent_id(SessionId::new(parent_session_id));
        }
        if let Some(session_title) = runtime_config.session_title.clone() {
            session = session.with_title(session_title);
        }
        sink.emit(AgentEvent::StorageState {
            session_id: Some(session.id.0.clone()),
            parent_session_id: session.parent_id.as_ref().map(|id| id.0.clone()),
            rollout_items: pre_storage_events.len(),
            rollout_truncated: false,
            child_session_ids,
        })
        .await?;
        let events = sink.events().await?;
        session.events = events.clone();
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

        if let Some(file_context) = load_mentioned_file_context(&config.cwd, prompt)? {
            messages.push(ProviderMessage::system(file_context));
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

fn load_mentioned_file_context(cwd: &Path, prompt: &str) -> AgentResult<Option<String>> {
    let mentions = extract_file_mentions(prompt);
    if mentions.is_empty() {
        return Ok(None);
    }

    let workspace = std::fs::canonicalize(cwd).map_err(|error| AgentError::Execution {
        message: format!("failed to canonicalize {}: {error}", cwd.display()),
    })?;
    let mut fragments = Vec::new();
    for mention in mentions.into_iter().take(MAX_MENTIONED_FILE_CONTEXT_FILES) {
        let requested = if mention.path.is_absolute() {
            mention.path
        } else {
            cwd.join(&mention.path)
        };
        let Ok(canonical) = std::fs::canonicalize(&requested) else {
            continue;
        };
        if !canonical.starts_with(&workspace) || !canonical.is_file() {
            continue;
        }
        let metadata = canonical
            .metadata()
            .map_err(|error| AgentError::Execution {
                message: format!(
                    "failed to read metadata for mentioned file {}: {error}",
                    canonical.display()
                ),
            })?;
        let content = if metadata.len() > MAX_MENTIONED_FILE_CONTEXT_BYTES {
            let bytes = std::fs::read(&canonical).map_err(|error| AgentError::Execution {
                message: format!(
                    "failed to read mentioned file {}: {error}",
                    canonical.display()
                ),
            })?;
            String::from_utf8_lossy(
                &bytes[..(MAX_MENTIONED_FILE_CONTEXT_BYTES as usize).min(bytes.len())],
            )
            .to_string()
        } else {
            std::fs::read_to_string(&canonical).map_err(|error| AgentError::Execution {
                message: format!(
                    "failed to read mentioned file {}: {error}",
                    canonical.display()
                ),
            })?
        };
        let relative = canonical.strip_prefix(&workspace).unwrap_or(&canonical);
        fragments.push(format!(
            "### {}\n{}",
            relative.display(),
            content.trim_end()
        ));
    }

    if fragments.is_empty() {
        Ok(None)
    } else {
        Ok(Some(format!(
            "Mentioned workspace file context:\n\n{}",
            fragments.join("\n\n")
        )))
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

async fn emit_provider_stream_events<S>(sink: &S, events: &[StreamEvent]) -> AgentResult<()>
where
    S: RuntimeEventSink,
{
    for event in protocol_stream_events_to_agent_events(events) {
        match event {
            AgentEvent::ThreadStarted { .. }
            | AgentEvent::TurnStarted
            | AgentEvent::CommandStarted { .. }
            | AgentEvent::McpToolStarted { .. }
            | AgentEvent::ToolCallStarted { .. }
            | AgentEvent::Completed { .. } => {}
            AgentEvent::Message { content } if content.is_empty() => {}
            other => sink.emit(other).await?,
        }
    }
    Ok(())
}

fn collect_provider_response(stream: ProviderStream) -> AgentResult<ProviderResponse> {
    if let Some(response) = stream.final_response {
        return Ok(response);
    }

    let mut message_content = String::new();
    let mut tool_calls = Vec::new();
    let mut usage = None;
    let mut argument_deltas: BTreeMap<String, String> = BTreeMap::new();
    let mut function_names: BTreeMap<String, String> = BTreeMap::new();
    let mut failed_message = None;
    let mut cancelled_reason = None;

    for event in stream.events {
        match event {
            StreamEvent::ItemStarted { item, .. } | StreamEvent::ItemCompleted { item, .. } => {
                collect_response_item(item, &mut message_content, &mut tool_calls, &mut usage)?;
            }
            StreamEvent::ItemDelta { delta, .. } => match delta {
                ResponseItemDelta::MessageContent { delta, .. } => {
                    message_content.push_str(&delta);
                }
                ResponseItemDelta::ToolCallArguments {
                    call_id: Some(call_id),
                    delta,
                } => {
                    argument_deltas.entry(call_id).or_default().push_str(&delta);
                }
                ResponseItemDelta::ToolCallName {
                    call_id: Some(call_id),
                    name,
                } => {
                    function_names.insert(call_id, name);
                }
                ResponseItemDelta::ReasoningContent { .. }
                | ResponseItemDelta::ToolCallName { call_id: None, .. }
                | ResponseItemDelta::ToolCallArguments { call_id: None, .. }
                | ResponseItemDelta::ToolCallStatus { .. }
                | ResponseItemDelta::ToolOutput { .. } => {}
            },
            StreamEvent::ResponseCompleted { status, .. } => match status {
                ResponseStatus::Completed | ResponseStatus::InProgress => {}
                ResponseStatus::Failed => {
                    failed_message.get_or_insert_with(|| "provider stream failed".to_string());
                }
                ResponseStatus::Cancelled => {
                    cancelled_reason.get_or_insert_with(|| "provider stream cancelled".to_string());
                }
            },
            StreamEvent::ResponseCancelled { reason, .. } => {
                cancelled_reason =
                    Some(reason.unwrap_or_else(|| "provider stream cancelled".to_string()));
            }
            StreamEvent::ResponseFailed { message, .. } => {
                failed_message = Some(message);
            }
            StreamEvent::ResponseStarted { .. } => {}
        }
    }

    if let Some(message) = failed_message {
        return Err(AgentError::Execution { message });
    }
    if let Some(message) = cancelled_reason {
        return Err(AgentError::Execution { message });
    }

    for (call_id, arguments) in argument_deltas {
        if tool_calls
            .iter()
            .any(|call| provider_tool_call_id(call) == Some(call_id.as_str()))
        {
            continue;
        }
        let name = function_names
            .get(&call_id)
            .map(String::as_str)
            .unwrap_or("shell");
        if let Ok(tool_call) =
            provider_tool_call_from_function(Some(call_id.clone()), name, &arguments)
        {
            tool_calls.push(tool_call);
        }
    }

    let message = if message_content.is_empty() {
        None
    } else {
        Some(ProviderMessage {
            role: ProviderRole::Assistant,
            content: message_content,
        })
    };
    Ok(ProviderResponse {
        message,
        tool_calls,
        usage,
    })
}

fn collect_response_item(
    item: ResponseItem,
    message_content: &mut String,
    tool_calls: &mut Vec<ProviderToolCall>,
    usage: &mut Option<TokenUsage>,
) -> AgentResult<()> {
    match item {
        ResponseItem::Message {
            role: ProtocolRole::Assistant,
            content,
        } => {
            message_content.push_str(&content);
        }
        ResponseItem::AgentMessage { content, .. } => {
            if let Some(content) = first_content_text(&content) {
                message_content.push_str(&content);
            }
        }
        ResponseItem::ToolCall { call } => {
            tool_calls.push(provider_tool_call_from_protocol(call)?);
        }
        ResponseItem::FunctionCall {
            call_id,
            name,
            arguments,
            ..
        } => {
            tool_calls.push(provider_tool_call_from_function(
                Some(call_id),
                &name,
                &arguments,
            )?);
        }
        ResponseItem::McpToolCall {
            call_id,
            server,
            tool,
            arguments,
            ..
        } => {
            tool_calls.push(ProviderToolCall::Mcp {
                id: Some(call_id),
                server,
                tool,
                arguments_json: Some(arguments),
            });
        }
        ResponseItem::ToolSearchCall { call_id, query, .. } => {
            tool_calls.push(ProviderToolCall::ToolSearch {
                id: Some(call_id),
                query,
            });
        }
        ResponseItem::Usage {
            input_tokens,
            cached_input_tokens,
            output_tokens,
            reasoning_output_tokens,
        } => {
            *usage = Some(TokenUsage {
                input_tokens,
                cached_input_tokens,
                output_tokens,
                reasoning_output_tokens,
            });
        }
        ResponseItem::Message { .. }
        | ResponseItem::Reasoning { .. }
        | ResponseItem::ReasoningItem { .. }
        | ResponseItem::LocalShellCall { .. }
        | ResponseItem::FunctionCallOutput { .. }
        | ResponseItem::WebSearchCall { .. }
        | ResponseItem::Compaction { .. }
        | ResponseItem::CompactionTrigger { .. } => {}
    }
    Ok(())
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
                yunxi_agent_protocol::ResponseItemDelta::ToolCallName { call_id, name } => {
                    output.push(AgentEvent::ToolCallStarted {
                        id: call_id.clone(),
                        name: name.clone(),
                        arguments_json: None,
                    });
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
                yunxi_agent_protocol::ResponseItemDelta::ToolOutput { call_id, delta } => {
                    output.push(AgentEvent::CommandUpdated {
                        id: call_id.clone(),
                        command: "tool_output".to_string(),
                        aggregated_output: delta.clone(),
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
            yunxi_agent_protocol::StreamEvent::ResponseCancelled { reason, .. } => {
                output.push(AgentEvent::Warning {
                    message: reason
                        .clone()
                        .unwrap_or_else(|| "response cancelled".to_string()),
                });
                output.push(AgentEvent::Completed {
                    status: AgentRunStatus::Failed,
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

fn provider_tool_call_from_protocol(call: ToolCall) -> AgentResult<ProviderToolCall> {
    match call {
        ToolCall::Shell { id, command } => Ok(ProviderToolCall::Shell { id, command }),
        ToolCall::Patch { id, patch } => Ok(ProviderToolCall::Patch { id, patch }),
        ToolCall::Mcp {
            id,
            server,
            tool,
            arguments_json,
        } => Ok(ProviderToolCall::Mcp {
            id,
            server,
            tool,
            arguments_json,
        }),
        ToolCall::Skill {
            id,
            name,
            arguments_json,
        } => Ok(ProviderToolCall::Skill {
            id,
            name,
            arguments_json,
        }),
        ToolCall::MultiAgent {
            id,
            action,
            arguments_json,
        } => Ok(ProviderToolCall::MultiAgent {
            id,
            action,
            arguments_json,
        }),
        ToolCall::ToolSearch { id, query } => Ok(ProviderToolCall::ToolSearch { id, query }),
        ToolCall::RequestUserInput { id, prompt } => {
            Ok(ProviderToolCall::RequestUserInput { id, prompt })
        }
        ToolCall::ViewImage { id, path } => Ok(ProviderToolCall::ViewImage { id, path }),
    }
}

fn provider_tool_call_from_function(
    id: Option<String>,
    name: &str,
    arguments: &str,
) -> AgentResult<ProviderToolCall> {
    let args = serde_json::from_str::<serde_json::Value>(arguments).map_err(|error| {
        AgentError::Execution {
            message: format!("failed to parse streamed tool arguments for {name}: {error}"),
        }
    })?;
    match name {
        "shell" => Ok(ProviderToolCall::Shell {
            id,
            command: required_json_string(&args, "command")?,
        }),
        "patch" => Ok(ProviderToolCall::Patch {
            id,
            patch: args
                .get("patch")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
                .unwrap_or_else(|| arguments.to_string()),
        }),
        "mcp" => Ok(ProviderToolCall::Mcp {
            id,
            server: required_json_string(&args, "server")?,
            tool: required_json_string(&args, "tool")?,
            arguments_json: optional_json_string(&args, "arguments_json"),
        }),
        "skill" => Ok(ProviderToolCall::Skill {
            id,
            name: required_json_string(&args, "name")?,
            arguments_json: optional_json_string(&args, "arguments_json"),
        }),
        "multi_agent" => Ok(ProviderToolCall::MultiAgent {
            id,
            action: required_json_string(&args, "action")?,
            arguments_json: optional_json_string(&args, "arguments_json"),
        }),
        "tool_search" => Ok(ProviderToolCall::ToolSearch {
            id,
            query: required_json_string(&args, "query")?,
        }),
        "request_user_input" => Ok(ProviderToolCall::RequestUserInput {
            id,
            prompt: required_json_string(&args, "prompt")?,
        }),
        "view_image" => Ok(ProviderToolCall::ViewImage {
            id,
            path: required_json_string(&args, "path")?,
        }),
        other if other.starts_with("skill__") => Ok(ProviderToolCall::Skill {
            id,
            name: optional_json_string(&args, "name")
                .unwrap_or_else(|| other.trim_start_matches("skill__").to_string()),
            arguments_json: optional_json_string(&args, "arguments_json"),
        }),
        other if other.starts_with("plugin__") => Ok(ProviderToolCall::ToolSearch {
            id,
            query: optional_json_string(&args, "query")
                .unwrap_or_else(|| other.trim_start_matches("plugin__").replace('_', " ")),
        }),
        other if other.starts_with("mcp__") => Ok(ProviderToolCall::Mcp {
            id,
            server: optional_json_string(&args, "server")
                .unwrap_or_else(|| other.trim_start_matches("mcp__").to_string()),
            tool: required_json_string(&args, "tool")?,
            arguments_json: optional_json_string(&args, "arguments_json"),
        }),
        other => Err(AgentError::Execution {
            message: format!("unsupported streamed provider tool call: {other}"),
        }),
    }
}

fn provider_tool_call_id(call: &ProviderToolCall) -> Option<&str> {
    match call {
        ProviderToolCall::Shell { id, .. }
        | ProviderToolCall::Patch { id, .. }
        | ProviderToolCall::Mcp { id, .. }
        | ProviderToolCall::Skill { id, .. }
        | ProviderToolCall::MultiAgent { id, .. }
        | ProviderToolCall::ToolSearch { id, .. }
        | ProviderToolCall::RequestUserInput { id, .. }
        | ProviderToolCall::ViewImage { id, .. } => id.as_deref(),
    }
}

fn required_json_string(value: &serde_json::Value, key: &str) -> AgentResult<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| AgentError::Execution {
            message: format!("streamed tool argument {key} is missing or not a string"),
        })
}

fn optional_json_string(value: &serde_json::Value, key: &str) -> Option<String> {
    value.get(key).map(|argument| {
        argument
            .as_str()
            .map(ToString::to_string)
            .unwrap_or_else(|| argument.to_string())
    })
}

fn inject_multi_agent_parent(
    arguments_json: Option<String>,
    current_session_id: &SessionId,
) -> Option<String> {
    let mut value = arguments_json
        .as_deref()
        .and_then(|json| serde_json::from_str::<Value>(json).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    if value.get("parent_id").is_none() {
        value["parent_id"] = Value::String(current_session_id.0.clone());
    }
    serde_json::to_string(&value).ok()
}

fn parse_runtime_multi_agent_command(
    action: &str,
    arguments_json: Option<&str>,
) -> AgentResult<MultiAgentCommand> {
    let arguments = parse_arguments_object(arguments_json)?;
    match action {
        "spawn" => Ok(MultiAgentCommand::Spawn {
            task: required_argument_string(&arguments, "task")?,
            parent_id: optional_argument_string(&arguments, "parent_id").map(AgentId),
        }),
        "spawn_run" | "spawnRun" | "run" => Ok(MultiAgentCommand::SpawnRun {
            task: required_argument_string(&arguments, "task")?,
            parent_id: optional_argument_string(&arguments, "parent_id").map(AgentId),
        }),
        "wait" => Ok(MultiAgentCommand::Wait {
            id: AgentId(required_argument_string(&arguments, "id")?),
        }),
        "send_message" | "sendMessage" | "message" => Ok(MultiAgentCommand::SendMessage {
            id: AgentId(required_argument_string(&arguments, "id")?),
            message: required_argument_string(&arguments, "message")?,
        }),
        "follow_up" | "followUp" => Ok(MultiAgentCommand::FollowUp {
            id: AgentId(required_argument_string(&arguments, "id")?),
            task: required_argument_string(&arguments, "task")?,
        }),
        "interrupt" => Ok(MultiAgentCommand::Interrupt {
            id: AgentId(required_argument_string(&arguments, "id")?),
        }),
        "list" => Ok(MultiAgentCommand::List),
        other => Err(AgentError::Execution {
            message: format!("unsupported multi-agent action: {other}"),
        }),
    }
}

fn parse_arguments_object(arguments_json: Option<&str>) -> AgentResult<Value> {
    let Some(arguments_json) = arguments_json
        .map(str::trim)
        .filter(|json| !json.is_empty())
    else {
        return Ok(json!({}));
    };
    let value =
        serde_json::from_str::<Value>(arguments_json).map_err(|error| AgentError::Execution {
            message: format!("failed to parse tool arguments JSON: {error}"),
        })?;
    if value.is_object() {
        Ok(value)
    } else {
        Err(AgentError::Execution {
            message: "tool arguments JSON must be an object".to_string(),
        })
    }
}

fn required_argument_string(arguments: &Value, name: &str) -> AgentResult<String> {
    optional_argument_string(arguments, name).ok_or_else(|| AgentError::Execution {
        message: format!("missing required multi-agent argument: {name}"),
    })
}

fn optional_argument_string(arguments: &Value, name: &str) -> Option<String> {
    arguments
        .get(name)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn runtime_multi_agent_events(result: &MultiAgentCommandResult) -> Vec<ToolRuntimeEvent> {
    let mut events = result
        .agents
        .iter()
        .map(|agent| ToolRuntimeEvent::MultiAgent {
            agent_id: agent.id.0.clone(),
            parent_agent_id: agent.parent_id.as_ref().map(|parent| parent.0.clone()),
            status: format!("{:?}", agent.status).to_ascii_lowercase(),
            message: Some(agent.task.clone()),
        })
        .collect::<Vec<_>>();
    if let Some(child_run) = &result.child_run {
        events.push(ToolRuntimeEvent::ChildAgent {
            agent_id: child_run.agent_id.0.clone(),
            child_session_id: child_run.session_id.clone(),
            parent_session_id: child_run.parent_session_id.clone(),
            status: format!("{:?}", child_run.status).to_ascii_lowercase(),
            message: child_run.final_response.clone(),
        });
        for event in &child_run.events {
            if let Some(message) = summarize_child_event(event) {
                events.push(ToolRuntimeEvent::ChildAgent {
                    agent_id: child_run.agent_id.0.clone(),
                    child_session_id: child_run.session_id.clone(),
                    parent_session_id: child_run.parent_session_id.clone(),
                    status: child_event_status(event).to_string(),
                    message: Some(message),
                });
            }
        }
    }
    events
}

fn summarize_child_event(event: &AgentEvent) -> Option<String> {
    match event {
        AgentEvent::Started { prompt } => Some(format!("child started: {prompt}")),
        AgentEvent::Message { content } => Some(content.clone()),
        AgentEvent::Reasoning { content } => Some(format!("child reasoning: {content}")),
        AgentEvent::ToolCallStarted { name, .. } => Some(format!("child tool started: {name}")),
        AgentEvent::ToolCallCompleted { name, output, .. } => {
            Some(format!("child tool completed: {name}: {output}"))
        }
        AgentEvent::CommandStarted { command, .. } => {
            Some(format!("child command started: {command}"))
        }
        AgentEvent::CommandCompleted {
            command,
            aggregated_output,
            ..
        } => Some(format!(
            "child command completed: {command}: {aggregated_output}"
        )),
        AgentEvent::StorageState { session_id, .. } => {
            Some(format!("child storage state: {:?}", session_id))
        }
        AgentEvent::Warning { message } => Some(format!("child warning: {message}")),
        AgentEvent::Error { message } => Some(format!("child error: {message}")),
        AgentEvent::Completed { status, .. } => Some(format!("child completed: {status:?}")),
        _ => None,
    }
}

fn child_event_status(event: &AgentEvent) -> &'static str {
    match event {
        AgentEvent::Error { .. } => "failed",
        AgentEvent::Completed { status, .. } if *status == AgentRunStatus::Failed => "failed",
        AgentEvent::Completed { .. } => "completed",
        AgentEvent::Started { .. } => "started",
        _ => "event",
    }
}

fn map_tool_call(
    config: &AgentConfig,
    current_session_id: &SessionId,
    tool_call: ProviderToolCall,
) -> ToolRequest {
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
        } => {
            let mut policy = policy;
            policy.approval = ApprovalDecision::Approved;
            policy.execution_policy.approval = ApprovalRequirement::PreApproved;
            ToolRequest {
                id,
                cwd,
                kind: ToolRequestKind::MultiAgent {
                    action,
                    arguments_json: inject_multi_agent_parent(arguments_json, current_session_id),
                },
                policy,
            }
        }
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

async fn emit_approval_requested_if_needed<S>(
    sink: &S,
    trace: &ToolDispatchTrace,
) -> AgentResult<()>
where
    S: RuntimeEventSink,
{
    if let Some(request) = &trace.policy_evaluation.approval_request {
        sink.emit(AgentEvent::ApprovalRequested {
            id: trace.request_id.clone(),
            tool_name: trace.tool_name.to_string(),
            reason: request.reason.clone(),
        })
        .await?;
    }
    Ok(())
}

async fn emit_approval_completed_if_needed<S>(
    sink: &S,
    trace: &ToolDispatchTrace,
    response: &yunxi_agent_tools::ToolResponse,
) -> AgentResult<()>
where
    S: RuntimeEventSink,
{
    if let Some(request) = &trace.policy_evaluation.approval_request {
        sink.emit(AgentEvent::ApprovalCompleted {
            id: response.id.clone().or_else(|| trace.request_id.clone()),
            approved: false,
            reason: response
                .error
                .clone()
                .or_else(|| Some(request.reason.clone())),
        })
        .await?;
    }
    Ok(())
}

async fn emit_escalation_requested_if_needed<S>(
    sink: &S,
    trace: &ToolDispatchTrace,
) -> AgentResult<()>
where
    S: RuntimeEventSink,
{
    if let Some(request) = &trace.policy_evaluation.escalation_request {
        sink.emit(AgentEvent::EscalationRequested {
            id: trace.request_id.clone(),
            tool_name: trace.tool_name.to_string(),
            reason: request.reason.clone(),
            required_sandbox: request
                .required_sandbox
                .as_ref()
                .and_then(policy_value_to_string),
            required_network: request
                .required_network
                .as_ref()
                .and_then(policy_value_to_string),
        })
        .await?;
    }
    Ok(())
}

async fn emit_escalation_completed_if_needed<S>(
    sink: &S,
    trace: &ToolDispatchTrace,
    response: &yunxi_agent_tools::ToolResponse,
) -> AgentResult<()>
where
    S: RuntimeEventSink,
{
    if let Some(request) = &trace.policy_evaluation.escalation_request {
        sink.emit(AgentEvent::EscalationCompleted {
            id: response.id.clone().or_else(|| trace.request_id.clone()),
            approved: false,
            reason: response
                .error
                .clone()
                .or_else(|| Some(request.reason.clone())),
        })
        .await?;
    }
    Ok(())
}

fn policy_value_to_string<T: serde::Serialize>(value: &T) -> Option<String> {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(ToString::to_string))
}

async fn emit_tool_lifecycle_events<S>(
    sink: &S,
    response: &yunxi_agent_tools::ToolResponse,
) -> AgentResult<()>
where
    S: RuntimeEventSink,
{
    for event in &response.lifecycle_events {
        match event {
            ExecLifecycleEvent::OutputDelta { id, stream, chunk } => {
                let stream_name = match stream {
                    ExecOutputStream::Stdout => "stdout",
                    ExecOutputStream::Stderr => "stderr",
                };
                sink.emit(AgentEvent::CommandUpdated {
                    id: id.clone(),
                    command: stream_name.to_string(),
                    aggregated_output: chunk.clone(),
                })
                .await?;
            }
            ExecLifecycleEvent::StdinWritten { id, bytes } => {
                sink.emit(AgentEvent::Reasoning {
                    content: format!("stdin written for {:?}: {bytes} byte(s)", id),
                })
                .await?;
            }
            ExecLifecycleEvent::Cancelled { id } => {
                sink.emit(AgentEvent::Warning {
                    message: format!("command cancelled: {:?}", id),
                })
                .await?;
            }
            ExecLifecycleEvent::Failed { id, message } => {
                sink.emit(AgentEvent::Error {
                    message: format!("command failed {:?}: {message}", id),
                })
                .await?;
            }
            ExecLifecycleEvent::Started { .. } | ExecLifecycleEvent::Completed { .. } => {}
        }
    }
    Ok(())
}

async fn emit_tool_runtime_events<S>(
    sink: &S,
    response: &yunxi_agent_tools::ToolResponse,
) -> AgentResult<()>
where
    S: RuntimeEventSink,
{
    for event in &response.runtime_events {
        match event {
            ToolRuntimeEvent::SandboxDecision {
                allowed,
                backend,
                network,
                escalation_required,
                denial_reason,
            } => {
                sink.emit(AgentEvent::Reasoning {
                    content: format!(
                        "Sandbox decision: allowed={allowed}, backend={backend}, network={network}, escalation_required={escalation_required}, denial_reason={}",
                        denial_reason.as_deref().unwrap_or("none")
                    ),
                })
                .await?;
            }
            ToolRuntimeEvent::McpSession {
                server,
                status,
                message,
            } => {
                sink.emit(AgentEvent::McpSession {
                    server: server.clone(),
                    status: status.clone(),
                    message: message.clone(),
                })
                .await?;
            }
            ToolRuntimeEvent::MultiAgent {
                agent_id,
                parent_agent_id,
                status,
                message,
            } => {
                sink.emit(AgentEvent::MultiAgentEvent {
                    agent_id: agent_id.clone(),
                    parent_agent_id: parent_agent_id.clone(),
                    status: status.clone(),
                    message: message.clone(),
                })
                .await?;
            }
            ToolRuntimeEvent::ChildAgent {
                agent_id,
                child_session_id,
                parent_session_id,
                status,
                message,
            } => {
                sink.emit(AgentEvent::ChildAgentEvent {
                    agent_id: agent_id.clone(),
                    child_session_id: child_session_id.clone(),
                    parent_session_id: parent_session_id.clone(),
                    status: status.clone(),
                    message: message.clone(),
                })
                .await?;
            }
            ToolRuntimeEvent::PatchDiagnostic {
                kind,
                message,
                path,
                line,
            } => {
                let location = match (path, line) {
                    (Some(path), Some(line)) => format!(" at {path}:{line}"),
                    (Some(path), None) => format!(" at {path}"),
                    (None, Some(line)) => format!(" at line {line}"),
                    (None, None) => String::new(),
                };
                sink.emit(AgentEvent::Warning {
                    message: format!("patch diagnostic [{kind}]{location}: {message}"),
                })
                .await?;
            }
        }
    }
    Ok(())
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
        if response.lifecycle_events.iter().any(|event| {
            matches!(
                event,
                ExecLifecycleEvent::Cancelled { .. } | ExecLifecycleEvent::Failed { .. }
            )
        }) {
            return Ok(());
        }
        let message = response
            .error
            .as_ref()
            .or(response.output.as_ref())
            .map(|message| message.trim())
            .filter(|message| !message.is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| match response.status {
                ToolStatus::Failed => match response.exit_code {
                    Some(code) => format!("tool execution failed with exit code {code}"),
                    None => "tool execution failed".to_string(),
                },
                ToolStatus::Declined => "tool execution declined".to_string(),
                ToolStatus::InProgress | ToolStatus::Completed => String::new(),
            });
        sink.emit(AgentEvent::Warning { message }).await?;
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
    if let Some(output) = response.output.as_ref().map(|output| output.trim()) {
        if !output.is_empty() {
            return output.to_string();
        }
    }
    if let Some(error) = response.error.as_ref().map(|error| error.trim()) {
        if !error.is_empty() {
            return error.to_string();
        }
    }
    match response.status {
        ToolStatus::Failed => match response.exit_code {
            Some(code) => format!("tool execution failed with exit code {code}"),
            None => "tool execution failed".to_string(),
        },
        ToolStatus::Declined => "tool execution declined".to_string(),
        ToolStatus::Completed | ToolStatus::InProgress => String::new(),
    }
}

fn child_session_ids_from_agent_events(events: &[AgentEvent]) -> Vec<String> {
    let mut ids = events
        .iter()
        .filter_map(|event| match event {
            AgentEvent::ChildAgentEvent {
                child_session_id, ..
            } => Some(child_session_id.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    ids
}

fn preview_child_title(prompt: &str) -> String {
    const MAX: usize = 48;
    let trimmed = prompt.trim();
    if trimmed.chars().count() <= MAX {
        return trimmed.to_string();
    }
    let mut value = trimmed
        .chars()
        .take(MAX.saturating_sub(3))
        .collect::<String>();
    value.push_str("...");
    value
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

fn generate_runtime_turn_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    format!("yunxi-turn-{millis}")
}

#[async_trait]
impl AgentBackend for YunXiRuntimeBackend {
    async fn run(&self, config: AgentConfig, input: AgentInput) -> AgentResult<AgentRunResult> {
        self.run_turn(AgentTurn::new(config, input)).await
    }
}

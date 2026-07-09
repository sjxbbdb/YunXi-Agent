use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use yunxi_agent_core::{
    AgentBackend, AgentConfig, AgentError, AgentEvent, AgentInput, AgentResult, AgentRunResult,
    AgentRunStatus,
};
use yunxi_agent_provider::{AgentProvider, ProviderRequest, StaticProvider};
use yunxi_agent_storage::{InMemorySessionStore, SessionRecord, SessionStore};
use yunxi_agent_tools::{NoopToolRuntime, ToolRuntime};

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
}

impl RuntimeContext {
    pub fn new(config: AgentConfig) -> Self {
        Self { config }
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
}

impl YunXiRuntimeBackend {
    pub fn new() -> Self {
        Self::default()
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
        }
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
            NoopToolRuntime,
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

        let provider_response = self
            .provider
            .complete(ProviderRequest::new(
                turn.config.clone(),
                AgentInput::text(prompt),
            ))
            .await?;
        let final_response = provider_response.message.content;

        sink.emit(AgentEvent::Message {
            content: final_response.clone(),
        })
        .await?;
        sink.emit(AgentEvent::Completed {
            status: AgentRunStatus::Completed,
            usage: provider_response.usage,
        })
        .await?;

        let events = sink.events().await?;
        self.storage
            .save(SessionRecord::new(
                turn.config.cwd,
                prompt,
                Some(final_response.clone()),
                events.clone(),
            ))
            .await?;

        Ok(AgentRunResult {
            status: AgentRunStatus::Completed,
            final_response: Some(final_response),
            events,
        })
    }
}

#[async_trait]
impl AgentBackend for YunXiRuntimeBackend {
    async fn run(&self, config: AgentConfig, input: AgentInput) -> AgentResult<AgentRunResult> {
        self.run_turn(AgentTurn::new(config, input)).await
    }
}

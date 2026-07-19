use crate::{
    AgentBackend, AgentConfig, AgentError, AgentEvent, AgentInput, AgentResult, AgentRunControl,
    AgentRunResult, AgentRunStatus, DryRunBackend,
};

#[derive(Clone, Debug)]
pub struct Agent {
    config: AgentConfig,
}

impl Agent {
    pub fn new(config: AgentConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &AgentConfig {
        &self.config
    }

    pub async fn run_dry(&self, input: AgentInput) -> AgentResult<AgentRunResult> {
        self.run_with_backend(&DryRunBackend, input).await
    }

    pub async fn run_with_backend<B>(
        &self,
        backend: &B,
        input: AgentInput,
    ) -> AgentResult<AgentRunResult>
    where
        B: AgentBackend,
    {
        backend.run(self.config.clone(), input).await
    }

    pub async fn run_with_backend_stream<B>(
        &self,
        backend: &B,
        input: AgentInput,
        control: AgentRunControl,
    ) -> AgentResult<AgentRunResult>
    where
        B: AgentBackend,
    {
        backend
            .run_stream(self.config.clone(), input, control)
            .await
    }
}

#[async_trait::async_trait]
impl AgentBackend for DryRunBackend {
    async fn run(&self, _config: AgentConfig, input: AgentInput) -> AgentResult<AgentRunResult> {
        let prompt = input.prompt.trim();
        if prompt.is_empty() {
            return Err(AgentError::EmptyPrompt);
        }

        let response = format!("Dry run accepted prompt: {prompt}");
        let events = vec![
            AgentEvent::Started {
                prompt: prompt.to_string(),
            },
            AgentEvent::Message {
                content: response.clone(),
                stream: None,
            },
            AgentEvent::Completed {
                status: AgentRunStatus::Completed,
                usage: None,
            },
        ];

        Ok(AgentRunResult {
            status: AgentRunStatus::Completed,
            final_response: Some(response),
            events,
        })
    }
}

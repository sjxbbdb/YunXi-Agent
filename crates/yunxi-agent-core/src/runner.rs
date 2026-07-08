use crate::{
    AgentConfig, AgentError, AgentEvent, AgentInput, AgentResult, AgentRunResult, AgentRunStatus,
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
            },
            AgentEvent::Completed {
                status: AgentRunStatus::Completed,
            },
        ];

        Ok(AgentRunResult {
            status: AgentRunStatus::Completed,
            final_response: Some(response),
            events,
        })
    }
}

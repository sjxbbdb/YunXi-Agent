use crate::{AgentConfig, AgentInput, AgentResult, AgentRunControl, AgentRunResult};
use serde::{Deserialize, Serialize};

#[async_trait::async_trait]
pub trait AgentBackend: Send + Sync {
    async fn run(&self, config: AgentConfig, input: AgentInput) -> AgentResult<AgentRunResult>;

    async fn run_stream(
        &self,
        config: AgentConfig,
        input: AgentInput,
        control: AgentRunControl,
    ) -> AgentResult<AgentRunResult> {
        let result = self.run(config, input).await?;
        for event in &result.events {
            control.emit_event(event.clone());
        }
        Ok(result)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackendKind {
    DryRun,
    Yunxi,
    Codex,
}

#[derive(Clone, Debug, Default)]
pub struct DryRunBackend;

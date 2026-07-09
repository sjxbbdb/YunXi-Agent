use crate::{AgentConfig, AgentInput, AgentResult, AgentRunResult};
use serde::{Deserialize, Serialize};

#[async_trait::async_trait]
pub trait AgentBackend: Send + Sync {
    async fn run(&self, config: AgentConfig, input: AgentInput) -> AgentResult<AgentRunResult>;
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

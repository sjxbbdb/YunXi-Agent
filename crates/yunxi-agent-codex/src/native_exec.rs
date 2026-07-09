use crate::map_config_to_codex_options;
use yunxi_agent_core::{
    AgentBackend, AgentConfig, AgentError, AgentInput, AgentResult, AgentRunResult,
};

#[derive(Clone, Debug, Default)]
pub struct CodexNativeBackend;

impl CodexNativeBackend {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl AgentBackend for CodexNativeBackend {
    async fn run(&self, config: AgentConfig, input: AgentInput) -> AgentResult<AgentRunResult> {
        let prompt = input.prompt.trim();
        if prompt.is_empty() {
            return Err(AgentError::EmptyPrompt);
        }

        let _options = map_config_to_codex_options(&config)?;

        #[cfg(feature = "codex-native")]
        {
            run_native_codex(_options, prompt.to_string()).await
        }

        #[cfg(not(feature = "codex-native"))]
        {
            Err(AgentError::Execution {
                message: "codex-native feature is not enabled".to_string(),
            })
        }
    }
}

#[cfg(feature = "codex-native")]
async fn run_native_codex(
    _options: crate::CodexRunOptions,
    _prompt: String,
) -> AgentResult<AgentRunResult> {
    Err(AgentError::Execution {
        message: "native Codex runner requires Task 9 live runner wiring".to_string(),
    })
}

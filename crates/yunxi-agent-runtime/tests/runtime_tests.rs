use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use yunxi_agent_core::{
    Agent, AgentConfig, AgentError, AgentEvent, AgentInput, AgentRunStatus, CommandStatus,
};
use yunxi_agent_provider::{
    AgentProvider, ProviderRequest, ProviderResponse, ProviderRole, ProviderToolCall,
    StaticProvider,
};
use yunxi_agent_runtime::YunXiRuntimeBackend;
use yunxi_agent_storage::{InMemorySessionStore, SessionStore};
use yunxi_agent_tools::{NoopToolRuntime, ShellToolRuntime};

#[tokio::test]
async fn yunxi_runtime_runs_without_codex_backend() {
    let store = InMemorySessionStore::default();
    let backend =
        YunXiRuntimeBackend::with_parts(StaticProvider::default(), NoopToolRuntime, store.clone());
    let agent = Agent::new(AgentConfig::new(PathBuf::from(".")));

    let result = agent
        .run_with_backend(&backend, AgentInput::text("explain this project"))
        .await
        .expect("yunxi runtime should complete");

    assert_eq!(result.status, AgentRunStatus::Completed);
    assert_eq!(
        result.final_response.as_deref(),
        Some("YunXi autonomous runtime accepted prompt: explain this project")
    );
    assert_eq!(
        result.events.first(),
        Some(&AgentEvent::Started {
            prompt: "explain this project".to_string()
        })
    );
    assert_eq!(store.list().await.expect("session list").len(), 1);
}

#[tokio::test]
async fn yunxi_runtime_rejects_empty_prompt() {
    let backend = YunXiRuntimeBackend::default();
    let agent = Agent::new(AgentConfig::new(PathBuf::from(".")));

    let error = agent
        .run_with_backend(&backend, AgentInput::text("   "))
        .await
        .expect_err("empty prompt should be rejected");

    assert!(matches!(error, AgentError::EmptyPrompt));
}

#[derive(Clone, Default)]
struct ShellCallingProvider {
    calls: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl AgentProvider for ShellCallingProvider {
    async fn complete(
        &self,
        request: ProviderRequest,
    ) -> yunxi_agent_core::AgentResult<ProviderResponse> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(tool_message) = request
            .messages
            .iter()
            .rev()
            .find(|message| message.role == ProviderRole::Tool)
        {
            return Ok(ProviderResponse::assistant(format!(
                "tool said: {}",
                tool_message.content.trim()
            )));
        }

        Ok(ProviderResponse::tool_call(ProviderToolCall::Shell {
            id: Some("shell-1".to_string()),
            command: "echo yunxi-tool".to_string(),
        }))
    }
}

#[tokio::test]
async fn yunxi_runtime_executes_provider_requested_shell_tool() {
    let store = InMemorySessionStore::default();
    let backend = YunXiRuntimeBackend::with_parts(
        ShellCallingProvider::default(),
        ShellToolRuntime,
        store.clone(),
    );
    let agent = Agent::new(AgentConfig::new(PathBuf::from(".")));

    let result = agent
        .run_with_backend(&backend, AgentInput::text("use a tool"))
        .await
        .expect("yunxi runtime should complete tool loop");

    assert_eq!(result.status, AgentRunStatus::Completed);
    assert!(
        result
            .final_response
            .as_deref()
            .expect("final response")
            .contains("yunxi-tool")
    );
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::CommandStarted {
            id: Some(id),
            command
        } if id == "shell-1" && command == "echo yunxi-tool"
    )));
    assert!(result.events.iter().any(|event| matches!(
        event,
        AgentEvent::CommandCompleted {
            id: Some(id),
            status: CommandStatus::Completed,
            aggregated_output,
            ..
        } if id == "shell-1" && aggregated_output.contains("yunxi-tool")
    )));
    assert_eq!(store.list().await.expect("session list").len(), 1);
}

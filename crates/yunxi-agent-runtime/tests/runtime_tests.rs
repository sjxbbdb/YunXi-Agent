use std::path::PathBuf;
use yunxi_agent_core::{Agent, AgentConfig, AgentError, AgentEvent, AgentInput, AgentRunStatus};
use yunxi_agent_provider::StaticProvider;
use yunxi_agent_runtime::YunXiRuntimeBackend;
use yunxi_agent_storage::{InMemorySessionStore, SessionStore};
use yunxi_agent_tools::NoopToolRuntime;

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

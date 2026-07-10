use std::path::PathBuf;
use yunxi_agent_core::{AgentConfig, AgentInput};
use yunxi_agent_provider::{AgentProvider, ProviderRequest, StaticProvider};

#[tokio::test]
async fn static_provider_returns_yunxi_runtime_message() {
    let provider = StaticProvider::default();
    let request = ProviderRequest::new(
        AgentConfig::new(PathBuf::from(".")),
        AgentInput::text("explain this project"),
    );

    let response = provider
        .complete(request)
        .await
        .expect("provider response should succeed");

    assert_eq!(
        response
            .message
            .as_ref()
            .map(|message| message.content.as_str()),
        Some("YunXi autonomous runtime accepted prompt: explain this project")
    );
}

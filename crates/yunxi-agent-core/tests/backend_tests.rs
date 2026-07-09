use std::path::PathBuf;
use yunxi_agent_core::{
    Agent, AgentConfig, AgentEvent, AgentInput, AgentRunStatus, BackendKind, DryRunBackend,
};

#[tokio::test]
async fn agent_can_run_through_backend_trait() {
    let agent = Agent::new(AgentConfig::new(PathBuf::from(".")));
    let backend = DryRunBackend::default();

    let result = agent
        .run_with_backend(&backend, AgentInput::text("use backend"))
        .await
        .expect("backend run should succeed");

    assert_eq!(result.status, AgentRunStatus::Completed);
    assert_eq!(
        result.final_response.as_deref(),
        Some("Dry run accepted prompt: use backend")
    );
    assert_eq!(
        result.events.first(),
        Some(&AgentEvent::Started {
            prompt: "use backend".to_string()
        })
    );
}

#[test]
fn backend_kind_serializes_as_kebab_case() {
    assert_eq!(
        serde_json::to_string(&BackendKind::DryRun).expect("json"),
        "\"dry-run\""
    );
    assert_eq!(
        serde_json::to_string(&BackendKind::Yunxi).expect("json"),
        "\"yunxi\""
    );
    assert_eq!(
        serde_json::to_string(&BackendKind::Codex).expect("json"),
        "\"codex\""
    );
}

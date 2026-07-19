use std::path::PathBuf;
use yunxi_agent_core::{Agent, AgentConfig, AgentError, AgentEvent, AgentInput, AgentRunStatus};

#[tokio::test]
async fn dry_run_returns_started_message_and_completed_events() {
    let agent = Agent::new(AgentConfig::new(PathBuf::from(".")));

    let result = agent
        .run_dry(AgentInput::text("explain this project"))
        .await
        .expect("dry run should succeed");

    assert_eq!(result.status, AgentRunStatus::Completed);
    assert_eq!(
        result.final_response.as_deref(),
        Some("Dry run accepted prompt: explain this project")
    );
    assert_eq!(
        result.events,
        vec![
            AgentEvent::Started {
                prompt: "explain this project".to_string()
            },
            AgentEvent::Message {
                content: "Dry run accepted prompt: explain this project".to_string(),
                stream: None
            },
            AgentEvent::Completed {
                status: AgentRunStatus::Completed,
                usage: None
            }
        ]
    );
}

#[tokio::test]
async fn dry_run_rejects_empty_prompt() {
    let agent = Agent::new(AgentConfig::new(PathBuf::from(".")));

    let error = agent
        .run_dry(AgentInput::text("   "))
        .await
        .expect_err("empty prompt should fail");

    assert!(matches!(error, AgentError::EmptyPrompt));
}

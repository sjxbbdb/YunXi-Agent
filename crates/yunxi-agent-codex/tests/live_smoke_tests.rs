use std::path::PathBuf;

use yunxi_agent_codex::CodexNativeBackend;
use yunxi_agent_core::{Agent, AgentConfig, AgentInput, AgentRunStatus};

#[tokio::test]
async fn live_codex_backend_can_complete_simple_prompt_when_enabled() {
    if std::env::var("YUNXI_RUN_LIVE_CODEX_TESTS").ok().as_deref() != Some("1") {
        eprintln!("skipping live test; set YUNXI_RUN_LIVE_CODEX_TESTS=1");
        return;
    }

    let cwd = std::env::current_dir().expect("cwd");
    let mut config = AgentConfig::new(cwd);
    if let Ok(model) = std::env::var("YUNXI_LIVE_MODEL") {
        config = config.with_model(model);
    }
    if let Ok(provider) = std::env::var("YUNXI_LIVE_PROVIDER") {
        config = config.with_provider(provider);
    }
    if let Ok(codex_home) = std::env::var("YUNXI_LIVE_CODEX_HOME") {
        config = config.with_codex_home(PathBuf::from(codex_home));
    }

    let agent = Agent::new(config);
    let result = agent
        .run_with_backend(
            &CodexNativeBackend::new(),
            AgentInput::text("Reply with exactly: YunXi live smoke ok"),
        )
        .await
        .expect("live run should succeed");

    assert_eq!(result.status, AgentRunStatus::Completed);
    assert!(
        result
            .final_response
            .as_deref()
            .unwrap_or_default()
            .contains("YunXi live smoke ok")
    );
}

use std::path::PathBuf;

use tempfile::TempDir;
use yunxi_agent_codex::{
    CodexApproval, CodexRunOptions, CodexSandbox, map_config_to_codex_options,
};
use yunxi_agent_core::{AgentConfig, ApprovalMode, SandboxMode};

#[test]
fn maps_basic_config_to_codex_options() {
    let temp = TempDir::new().expect("temp");
    let config = AgentConfig::new(temp.path())
        .with_model("gpt-5")
        .with_provider("openai")
        .with_codex_home(PathBuf::from("D:/codex-home"))
        .with_approval_mode(ApprovalMode::Never)
        .with_sandbox_mode(SandboxMode::DangerFullAccess);

    let options = map_config_to_codex_options(&config).expect("options");

    assert_eq!(
        options,
        CodexRunOptions {
            cwd: temp.path().to_path_buf(),
            model: Some("gpt-5".to_string()),
            provider: Some("openai".to_string()),
            codex_home: Some(PathBuf::from("D:/codex-home")),
            approval: CodexApproval::Never,
            sandbox: CodexSandbox::DangerFullAccess,
            ephemeral: true,
        }
    );
}

#[test]
fn rejects_missing_working_directory() {
    let config = AgentConfig::new(PathBuf::from("Z:/missing/workspace"));

    let error = map_config_to_codex_options(&config).expect_err("missing cwd should fail");

    assert!(
        error
            .to_string()
            .contains("working directory does not exist")
    );
}

use std::path::PathBuf;
use yunxi_agent_core::{AgentConfig, ApprovalMode, SandboxMode};

#[test]
fn default_config_uses_workspace_write_and_on_request() {
    let config = AgentConfig::new(PathBuf::from("D:/work/project"));

    assert_eq!(config.cwd, PathBuf::from("D:/work/project"));
    assert_eq!(config.approval_mode, ApprovalMode::OnRequest);
    assert_eq!(config.sandbox_mode, SandboxMode::WorkspaceWrite);
    assert_eq!(config.model, None);
    assert_eq!(config.provider, None);
    assert_eq!(config.codex_home, None);
    assert_eq!(config.parent_session_id, None);
    assert_eq!(config.session_id, None);
    assert_eq!(config.session_title, None);
    assert_eq!(config.context_window_tokens, None);
    assert_eq!(config.auto_compact_threshold_tokens, None);
    assert!(!config.companion.love_letters.enabled);
    assert_eq!(config.companion.love_letters.cooldown_min_days, 3);
    assert_eq!(config.companion.love_letters.cooldown_max_days, 10);
}

#[test]
fn legacy_companion_config_defaults_love_letters_off() {
    let value = serde_json::json!({
        "enabled": true,
        "quiet_hours": null,
        "max_proactive_per_session": 3,
        "max_proactive_per_day": 8,
        "require_reason": true,
        "allow_tool_requests": false,
        "cloud_control_enabled": false,
        "clear_requires_confirmation": true
    });
    let settings = serde_json::from_value::<yunxi_agent_core::CompanionSettings>(value)
        .expect("legacy companion settings");
    assert!(settings.enabled);
    assert!(!settings.love_letters.enabled);
}

#[test]
fn builder_methods_set_optional_values() {
    let config = AgentConfig::new(PathBuf::from("D:/work/project"))
        .with_model("gpt-5")
        .with_provider("openai")
        .with_codex_home(PathBuf::from("D:/codex-home"))
        .with_approval_mode(ApprovalMode::Never)
        .with_sandbox_mode(SandboxMode::ReadOnly)
        .with_parent_session_id("parent-1")
        .with_session_id("session-1")
        .with_session_title("Resume parent-1")
        .with_context_window_tokens(120_000)
        .with_auto_compact_threshold_tokens(96_000);

    assert_eq!(config.model.as_deref(), Some("gpt-5"));
    assert_eq!(config.provider.as_deref(), Some("openai"));
    assert_eq!(config.codex_home, Some(PathBuf::from("D:/codex-home")));
    assert_eq!(config.approval_mode, ApprovalMode::Never);
    assert_eq!(config.sandbox_mode, SandboxMode::ReadOnly);
    assert_eq!(config.parent_session_id.as_deref(), Some("parent-1"));
    assert_eq!(config.session_id.as_deref(), Some("session-1"));
    assert_eq!(config.session_title.as_deref(), Some("Resume parent-1"));
    assert_eq!(config.context_window_tokens, Some(120_000));
    assert_eq!(config.auto_compact_threshold_tokens, Some(96_000));
}

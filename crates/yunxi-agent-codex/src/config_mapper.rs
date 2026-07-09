use std::path::PathBuf;

use yunxi_agent_core::{AgentConfig, AgentError, AgentResult, ApprovalMode, SandboxMode};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexRunOptions {
    pub cwd: PathBuf,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub codex_home: Option<PathBuf>,
    pub approval: CodexApproval,
    pub sandbox: CodexSandbox,
    pub ephemeral: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodexApproval {
    Never,
    OnRequest,
    OnFailure,
    Untrusted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodexSandbox {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

pub fn map_config_to_codex_options(config: &AgentConfig) -> AgentResult<CodexRunOptions> {
    if !config.cwd.is_dir() {
        return Err(AgentError::MissingWorkingDirectory {
            path: config.cwd.display().to_string(),
        });
    }

    Ok(CodexRunOptions {
        cwd: config.cwd.clone(),
        model: config.model.clone(),
        provider: config.provider.clone(),
        codex_home: config.codex_home.clone(),
        approval: match config.approval_mode {
            ApprovalMode::Never => CodexApproval::Never,
            ApprovalMode::OnRequest => CodexApproval::OnRequest,
            ApprovalMode::OnFailure => CodexApproval::OnFailure,
            ApprovalMode::Untrusted => CodexApproval::Untrusted,
        },
        sandbox: match config.sandbox_mode {
            SandboxMode::ReadOnly => CodexSandbox::ReadOnly,
            SandboxMode::WorkspaceWrite => CodexSandbox::WorkspaceWrite,
            SandboxMode::DangerFullAccess => CodexSandbox::DangerFullAccess,
        },
        ephemeral: true,
    })
}

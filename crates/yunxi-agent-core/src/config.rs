use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentConfig {
    pub cwd: PathBuf,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub codex_home: Option<PathBuf>,
    pub approval_mode: ApprovalMode,
    pub sandbox_mode: SandboxMode,
}

impl AgentConfig {
    pub fn new(cwd: impl Into<PathBuf>) -> Self {
        Self {
            cwd: cwd.into(),
            model: None,
            provider: None,
            codex_home: None,
            approval_mode: ApprovalMode::OnRequest,
            sandbox_mode: SandboxMode::WorkspaceWrite,
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    pub fn with_codex_home(mut self, codex_home: impl Into<PathBuf>) -> Self {
        self.codex_home = Some(codex_home.into());
        self
    }

    pub fn with_approval_mode(mut self, approval_mode: ApprovalMode) -> Self {
        self.approval_mode = approval_mode;
        self
    }

    pub fn with_sandbox_mode(mut self, sandbox_mode: SandboxMode) -> Self {
        self.sandbox_mode = sandbox_mode;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApprovalMode {
    Never,
    OnRequest,
    OnFailure,
    Untrusted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SandboxMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

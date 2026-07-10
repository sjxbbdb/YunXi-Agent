use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use yunxi_agent_core::{AgentConfig, ApprovalMode, SandboxMode};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExecutionPolicy {
    pub approval: ApprovalRequirement,
    pub sandbox: SandboxRequirement,
    pub network: NetworkPolicy,
    pub workspace_root: PathBuf,
}

impl ExecutionPolicy {
    pub fn from_config(config: &AgentConfig) -> Self {
        Self {
            approval: match config.approval_mode {
                ApprovalMode::Never => ApprovalRequirement::PreApproved,
                ApprovalMode::OnRequest => ApprovalRequirement::AskBeforeRunning,
                ApprovalMode::OnFailure => ApprovalRequirement::AskOnFailure,
                ApprovalMode::Untrusted => ApprovalRequirement::AskBeforeRunning,
            },
            sandbox: match config.sandbox_mode {
                SandboxMode::ReadOnly => SandboxRequirement::ReadOnly,
                SandboxMode::WorkspaceWrite => SandboxRequirement::WorkspaceWrite,
                SandboxMode::DangerFullAccess => SandboxRequirement::DangerFullAccess,
            },
            network: NetworkPolicy::Inherit,
            workspace_root: config.cwd.clone(),
        }
    }

    pub fn decision_for_cwd(&self, cwd: &Path) -> PolicyDecision {
        if !self.approval.is_approved_without_prompt() {
            return PolicyDecision::Blocked {
                reason: "execution requires approval".to_string(),
            };
        }

        match self.sandbox {
            SandboxRequirement::ReadOnly => PolicyDecision::Blocked {
                reason: "sandbox is read-only".to_string(),
            },
            SandboxRequirement::WorkspaceWrite => {
                if is_within_workspace(&self.workspace_root, cwd) {
                    PolicyDecision::Allowed
                } else {
                    PolicyDecision::Blocked {
                        reason: format!(
                            "cwd {} is outside workspace {}",
                            cwd.display(),
                            self.workspace_root.display()
                        ),
                    }
                }
            }
            SandboxRequirement::DangerFullAccess => PolicyDecision::Allowed,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalRequirement {
    PreApproved,
    AskBeforeRunning,
    AskOnFailure,
    Declined,
}

impl ApprovalRequirement {
    pub fn is_approved_without_prompt(self) -> bool {
        matches!(self, Self::PreApproved)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SandboxRequirement {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkPolicy {
    Inherit,
    Disabled,
    Enabled,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PolicyDecision {
    Allowed,
    Blocked { reason: String },
}

fn is_within_workspace(root: &Path, cwd: &Path) -> bool {
    let root = match std::fs::canonicalize(root) {
        Ok(path) => path,
        Err(_) => return false,
    };
    let cwd = match std::fs::canonicalize(cwd) {
        Ok(path) => path,
        Err(_) => return false,
    };
    cwd.starts_with(root)
}

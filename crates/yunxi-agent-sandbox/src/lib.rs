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
        self.evaluate(cwd, None).decision
    }

    pub fn evaluate(&self, cwd: &Path, command: Option<&str>) -> PolicyEvaluation {
        let risk = command
            .map(CommandRisk::classify)
            .unwrap_or(CommandRisk::Low);
        if !self.approval.is_approved_without_prompt() {
            return PolicyEvaluation {
                decision: PolicyDecision::Blocked {
                    reason: "execution requires approval".to_string(),
                },
                approval_request: Some(ApprovalRequest {
                    reason: "execution requires approval".to_string(),
                    command: command.map(ToString::to_string),
                    cwd: cwd.to_path_buf(),
                    risk,
                }),
                sandbox_backend: SandboxBackendSelection::from_requirement(self.sandbox),
                network_decision: NetworkDecision::from_policy(self.network),
            };
        }

        let decision = self.sandbox_decision_for_cwd(cwd);
        PolicyEvaluation {
            decision,
            approval_request: None,
            sandbox_backend: SandboxBackendSelection::from_requirement(self.sandbox),
            network_decision: NetworkDecision::from_policy(self.network),
        }
    }

    fn sandbox_decision_for_cwd(&self, cwd: &Path) -> PolicyDecision {
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub reason: String,
    pub command: Option<String>,
    pub cwd: PathBuf,
    pub risk: CommandRisk,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ApprovalResponse {
    Approved { justification: Option<String> },
    Denied { reason: String },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandRisk {
    Low,
    WritesWorkspace,
    Network,
    Destructive,
}

impl CommandRisk {
    pub fn classify(command: &str) -> Self {
        let lower = command.to_ascii_lowercase();
        if contains_any(
            &lower,
            &["rm -rf", "del /", "format ", "remove-item", "rd /s"],
        ) {
            Self::Destructive
        } else if contains_any(&lower, &["curl ", "wget ", "invoke-webrequest", "irm "]) {
            Self::Network
        } else if contains_any(
            &lower,
            &[
                ">",
                " copy ",
                " cp ",
                " mv ",
                " move ",
                "new-item",
                "set-content",
            ],
        ) {
            Self::WritesWorkspace
        } else {
            Self::Low
        }
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
pub struct PolicyEvaluation {
    pub decision: PolicyDecision,
    pub approval_request: Option<ApprovalRequest>,
    pub sandbox_backend: SandboxBackendSelection,
    pub network_decision: NetworkDecision,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PolicyDecision {
    Allowed,
    Blocked { reason: String },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxBackend {
    None,
    WorkspaceGuard,
    WindowsRestrictedToken,
    LinuxLandlock,
    DangerFullAccess,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SandboxBackendSelection {
    pub backend: SandboxBackend,
    pub reason: String,
}

impl SandboxBackendSelection {
    pub fn from_requirement(requirement: SandboxRequirement) -> Self {
        match requirement {
            SandboxRequirement::ReadOnly | SandboxRequirement::WorkspaceWrite => Self {
                backend: platform_sandbox_backend(),
                reason: format!("selected for {requirement:?}"),
            },
            SandboxRequirement::DangerFullAccess => Self {
                backend: SandboxBackend::DangerFullAccess,
                reason: "danger-full-access bypasses sandbox wrapping".to_string(),
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum NetworkDecision {
    Inherited,
    Disabled,
    Enabled,
}

impl NetworkDecision {
    pub fn from_policy(policy: NetworkPolicy) -> Self {
        match policy {
            NetworkPolicy::Inherit => Self::Inherited,
            NetworkPolicy::Disabled => Self::Disabled,
            NetworkPolicy::Enabled => Self::Enabled,
        }
    }
}

fn platform_sandbox_backend() -> SandboxBackend {
    if cfg!(windows) {
        SandboxBackend::WindowsRestrictedToken
    } else if cfg!(target_os = "linux") {
        SandboxBackend::LinuxLandlock
    } else {
        SandboxBackend::WorkspaceGuard
    }
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn evaluation_creates_approval_request_when_prompt_is_required() {
        let workspace = TempDir::new().expect("workspace");
        let policy = ExecutionPolicy {
            approval: ApprovalRequirement::AskBeforeRunning,
            sandbox: SandboxRequirement::WorkspaceWrite,
            network: NetworkPolicy::Inherit,
            workspace_root: workspace.path().to_path_buf(),
        };

        let evaluation = policy.evaluate(workspace.path(), Some("echo yunxi"));

        assert!(matches!(
            evaluation.decision,
            PolicyDecision::Blocked { .. }
        ));
        assert_eq!(
            evaluation.approval_request.expect("approval").risk,
            CommandRisk::Low
        );
    }

    #[test]
    fn command_risk_detects_destructive_and_network_commands() {
        assert_eq!(
            CommandRisk::classify("rm -rf target"),
            CommandRisk::Destructive
        );
        assert_eq!(
            CommandRisk::classify("curl https://example.test"),
            CommandRisk::Network
        );
        assert_eq!(
            CommandRisk::classify("echo hi > file.txt"),
            CommandRisk::WritesWorkspace
        );
    }

    #[test]
    fn danger_full_access_selects_bypass_backend() {
        let selection =
            SandboxBackendSelection::from_requirement(SandboxRequirement::DangerFullAccess);

        assert_eq!(selection.backend, SandboxBackend::DangerFullAccess);
    }
}

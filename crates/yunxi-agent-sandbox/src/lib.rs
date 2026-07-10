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
        let sandbox_backend = SandboxBackendSelection::from_requirement(self.sandbox);
        let network_decision = NetworkDecision::from_policy(self.network);
        if let Some(reason) = self.network_denial_for(risk) {
            return PolicyEvaluation {
                decision: PolicyDecision::Blocked {
                    reason: reason.clone(),
                },
                approval_request: None,
                escalation_request: Some(EscalationRequest {
                    reason,
                    command: command.map(ToString::to_string),
                    cwd: cwd.to_path_buf(),
                    risk,
                    required_sandbox: None,
                    required_network: Some(NetworkPolicy::Enabled),
                }),
                sandbox_backend,
                network_decision,
            };
        }

        if let Some(reason) = self.approval.prompt_reason_before_run(risk) {
            return PolicyEvaluation {
                decision: PolicyDecision::Blocked {
                    reason: reason.clone(),
                },
                approval_request: Some(ApprovalRequest {
                    reason,
                    command: command.map(ToString::to_string),
                    cwd: cwd.to_path_buf(),
                    risk,
                }),
                escalation_request: None,
                sandbox_backend,
                network_decision,
            };
        }

        let decision = self.sandbox_decision_for_cwd(cwd, risk);
        let escalation_request =
            escalation_request_for_decision(&decision, command, cwd, risk, self.sandbox);
        PolicyEvaluation {
            decision,
            approval_request: None,
            escalation_request,
            sandbox_backend,
            network_decision,
        }
    }

    fn sandbox_decision_for_cwd(&self, cwd: &Path, risk: CommandRisk) -> PolicyDecision {
        match self.sandbox {
            SandboxRequirement::ReadOnly if risk.requires_write_access() => {
                PolicyDecision::Blocked {
                    reason: "sandbox is read-only".to_string(),
                }
            }
            SandboxRequirement::ReadOnly => PolicyDecision::Allowed,
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

    fn network_denial_for(&self, risk: CommandRisk) -> Option<String> {
        if matches!(self.network, NetworkPolicy::Disabled) && risk.requires_network() {
            Some("network access is disabled by policy".to_string())
        } else {
            None
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
        matches!(self, Self::PreApproved | Self::AskOnFailure)
    }

    pub fn prompt_reason_before_run(self, risk: CommandRisk) -> Option<String> {
        match self {
            Self::PreApproved | Self::AskOnFailure => None,
            Self::AskBeforeRunning => Some("tool execution requires approval".to_string()),
            Self::Declined => Some(format!("approval was declined for {risk:?} command")),
        }
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
    ReadsWorkspace,
    WritesWorkspace,
    Network,
    CredentialAccess,
    ProcessControl,
    Destructive,
}

impl CommandRisk {
    pub fn classify(command: &str) -> Self {
        let lower = command.to_ascii_lowercase();
        if contains_any(
            &lower,
            &[
                "rm -rf",
                "del /",
                "format ",
                "remove-item",
                "rd /s",
                "rmdir /s",
                "erase ",
            ],
        ) {
            Self::Destructive
        } else if contains_any(
            &lower,
            &[
                "taskkill",
                "kill ",
                "pkill ",
                "stop-process",
                "shutdown",
                "restart-computer",
            ],
        ) {
            Self::ProcessControl
        } else if contains_any(
            &lower,
            &[
                "api_key",
                "apikey",
                "password",
                "passwd",
                "token",
                "credential",
                "secret",
                ".env",
                "id_rsa",
            ],
        ) {
            Self::CredentialAccess
        } else if contains_any(
            &lower,
            &[
                "curl ",
                "wget ",
                "invoke-webrequest",
                "invoke-restmethod",
                "irm ",
                "npm install",
                "cargo install",
                "pip install",
            ],
        ) {
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
                "out-file",
                "add-content",
                "apply_patch",
            ],
        ) {
            Self::WritesWorkspace
        } else if contains_any(
            &lower,
            &[
                "cat ",
                "type ",
                "get-content",
                "rg ",
                "ripgrep ",
                "findstr ",
            ],
        ) {
            Self::ReadsWorkspace
        } else {
            Self::Low
        }
    }

    pub fn requires_write_access(self) -> bool {
        matches!(
            self,
            Self::WritesWorkspace | Self::Destructive | Self::ProcessControl
        )
    }

    pub fn requires_network(self) -> bool {
        matches!(self, Self::Network)
    }

    pub fn requires_escalation_from_read_only(self) -> bool {
        self.requires_write_access()
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
    pub escalation_request: Option<EscalationRequest>,
    pub sandbox_backend: SandboxBackendSelection,
    pub network_decision: NetworkDecision,
}

impl PolicyEvaluation {
    pub fn execution_plan(&self) -> SandboxExecutionPlan {
        SandboxExecutionPlan {
            allowed: matches!(self.decision, PolicyDecision::Allowed),
            backend: self.sandbox_backend.backend,
            network: self.network_decision.clone(),
            approval_required: self.approval_request.is_some(),
            escalation_required: self.escalation_request.is_some(),
            denial_reason: match &self.decision {
                PolicyDecision::Allowed => None,
                PolicyDecision::Blocked { reason } => Some(reason.clone()),
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SandboxExecutionPlan {
    pub allowed: bool,
    pub backend: SandboxBackend,
    pub network: NetworkDecision,
    pub approval_required: bool,
    pub escalation_required: bool,
    pub denial_reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SandboxRunnerDecision {
    pub plan: SandboxExecutionPlan,
    pub command: Option<String>,
    pub cwd: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum EscalationResponse {
    Approved {
        sandbox: Option<SandboxRequirement>,
        network: Option<NetworkPolicy>,
        justification: Option<String>,
    },
    Declined {
        reason: String,
    },
    NotAvailable {
        reason: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EscalationOutcome {
    pub request: EscalationRequest,
    pub response: EscalationResponse,
}

impl EscalationOutcome {
    pub fn non_interactive_decline(request: EscalationRequest) -> Self {
        Self {
            request,
            response: EscalationResponse::NotAvailable {
                reason: "escalation requires an interactive host in this runtime".to_string(),
            },
        }
    }

    pub fn approved(&self) -> bool {
        matches!(self.response, EscalationResponse::Approved { .. })
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SandboxRunner;

impl SandboxRunner {
    pub fn plan(
        &self,
        policy: &ExecutionPolicy,
        cwd: &Path,
        command: Option<&str>,
    ) -> SandboxRunnerDecision {
        let evaluation = policy.evaluate(cwd, command);
        SandboxRunnerDecision {
            plan: evaluation.execution_plan(),
            command: command.map(ToString::to_string),
            cwd: cwd.to_path_buf(),
        }
    }

    pub fn non_interactive_escalation_outcome(
        &self,
        evaluation: &PolicyEvaluation,
    ) -> Option<EscalationOutcome> {
        evaluation
            .escalation_request
            .clone()
            .map(EscalationOutcome::non_interactive_decline)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EscalationRequest {
    pub reason: String,
    pub command: Option<String>,
    pub cwd: PathBuf,
    pub risk: CommandRisk,
    pub required_sandbox: Option<SandboxRequirement>,
    pub required_network: Option<NetworkPolicy>,
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

fn escalation_request_for_decision(
    decision: &PolicyDecision,
    command: Option<&str>,
    cwd: &Path,
    risk: CommandRisk,
    sandbox: SandboxRequirement,
) -> Option<EscalationRequest> {
    let PolicyDecision::Blocked { reason } = decision else {
        return None;
    };
    let required_sandbox = if matches!(sandbox, SandboxRequirement::ReadOnly)
        && risk.requires_escalation_from_read_only()
    {
        Some(SandboxRequirement::WorkspaceWrite)
    } else if matches!(sandbox, SandboxRequirement::WorkspaceWrite)
        && reason.contains("outside workspace")
    {
        Some(SandboxRequirement::DangerFullAccess)
    } else {
        None
    };
    required_sandbox.map(|required_sandbox| EscalationRequest {
        reason: reason.clone(),
        command: command.map(ToString::to_string),
        cwd: cwd.to_path_buf(),
        risk,
        required_sandbox: Some(required_sandbox),
        required_network: None,
    })
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
        assert_eq!(
            CommandRisk::classify("taskkill /pid 1234"),
            CommandRisk::ProcessControl
        );
        assert_eq!(
            CommandRisk::classify("cat .env"),
            CommandRisk::CredentialAccess
        );
    }

    #[test]
    fn danger_full_access_selects_bypass_backend() {
        let selection =
            SandboxBackendSelection::from_requirement(SandboxRequirement::DangerFullAccess);

        assert_eq!(selection.backend, SandboxBackend::DangerFullAccess);
    }

    #[test]
    fn read_only_allows_low_risk_but_blocks_writes_with_escalation() {
        let workspace = TempDir::new().expect("workspace");
        let policy = ExecutionPolicy {
            approval: ApprovalRequirement::PreApproved,
            sandbox: SandboxRequirement::ReadOnly,
            network: NetworkPolicy::Inherit,
            workspace_root: workspace.path().to_path_buf(),
        };

        assert!(matches!(
            policy
                .evaluate(workspace.path(), Some("echo yunxi"))
                .decision,
            PolicyDecision::Allowed
        ));

        let evaluation = policy.evaluate(workspace.path(), Some("echo yunxi > file.txt"));
        assert!(matches!(
            evaluation.decision,
            PolicyDecision::Blocked { ref reason } if reason.contains("read-only")
        ));
        assert_eq!(
            evaluation
                .escalation_request
                .expect("escalation")
                .required_sandbox,
            Some(SandboxRequirement::WorkspaceWrite)
        );
    }

    #[test]
    fn disabled_network_blocks_network_commands_with_escalation() {
        let workspace = TempDir::new().expect("workspace");
        let policy = ExecutionPolicy {
            approval: ApprovalRequirement::PreApproved,
            sandbox: SandboxRequirement::WorkspaceWrite,
            network: NetworkPolicy::Disabled,
            workspace_root: workspace.path().to_path_buf(),
        };

        let evaluation = policy.evaluate(workspace.path(), Some("curl https://example.test"));

        assert!(matches!(
            evaluation.decision,
            PolicyDecision::Blocked { ref reason } if reason.contains("network")
        ));
        assert_eq!(
            evaluation
                .escalation_request
                .expect("network escalation")
                .required_network,
            Some(NetworkPolicy::Enabled)
        );
    }

    #[test]
    fn workspace_write_outside_workspace_requests_full_access_escalation() {
        let workspace = TempDir::new().expect("workspace");
        let outside = TempDir::new().expect("outside");
        let policy = ExecutionPolicy {
            approval: ApprovalRequirement::PreApproved,
            sandbox: SandboxRequirement::WorkspaceWrite,
            network: NetworkPolicy::Inherit,
            workspace_root: workspace.path().to_path_buf(),
        };

        let evaluation = policy.evaluate(outside.path(), Some("echo yunxi > file.txt"));

        assert!(matches!(
            evaluation.decision,
            PolicyDecision::Blocked { ref reason } if reason.contains("outside workspace")
        ));
        assert_eq!(
            evaluation
                .escalation_request
                .expect("workspace escalation")
                .required_sandbox,
            Some(SandboxRequirement::DangerFullAccess)
        );
    }

    #[test]
    fn sandbox_runner_exposes_execution_plan_facade() {
        let workspace = TempDir::new().expect("workspace");
        let policy = ExecutionPolicy {
            approval: ApprovalRequirement::PreApproved,
            sandbox: SandboxRequirement::ReadOnly,
            network: NetworkPolicy::Inherit,
            workspace_root: workspace.path().to_path_buf(),
        };

        let decision =
            SandboxRunner.plan(&policy, workspace.path(), Some("echo denied > file.txt"));

        assert!(!decision.plan.allowed);
        assert!(decision.plan.escalation_required);
        assert_eq!(
            decision.plan.denial_reason.as_deref(),
            Some("sandbox is read-only")
        );
    }
}

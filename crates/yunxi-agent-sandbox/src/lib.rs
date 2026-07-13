use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
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

        let decision = self.sandbox_decision_for_cwd(cwd, command, risk);
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

    fn sandbox_decision_for_cwd(
        &self,
        cwd: &Path,
        command: Option<&str>,
        risk: CommandRisk,
    ) -> PolicyDecision {
        match self.sandbox {
            SandboxRequirement::ReadOnly if risk.requires_write_access() => {
                PolicyDecision::Blocked {
                    reason: "sandbox is read-only".to_string(),
                }
            }
            SandboxRequirement::ReadOnly => PolicyDecision::Allowed,
            SandboxRequirement::WorkspaceWrite => {
                if !is_within_workspace(&self.workspace_root, cwd) {
                    return PolicyDecision::Blocked {
                        reason: format!(
                            "cwd {} is outside workspace {}",
                            cwd.display(),
                            self.workspace_root.display()
                        ),
                    };
                }
                if let Some(command) = command {
                    if let Some(reason) =
                        workspace_write_target_denial(&self.workspace_root, cwd, command)
                    {
                        return PolicyDecision::Blocked { reason };
                    }
                }
                PolicyDecision::Allowed
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
        let tokens = shell_tokens(command);
        let lower_tokens = tokens
            .iter()
            .map(|token| token.to_ascii_lowercase())
            .collect::<Vec<_>>();
        if tokenized_destructive(&lower_tokens) {
            return Self::Destructive;
        }
        if tokenized_process_control(&lower_tokens) {
            return Self::ProcessControl;
        }
        if tokenized_credential_access(&lower_tokens) {
            return Self::CredentialAccess;
        }
        if tokenized_network(&lower_tokens) {
            return Self::Network;
        }
        if tokenized_write(&lower_tokens) {
            return Self::WritesWorkspace;
        }
        if tokenized_read(&lower_tokens) {
            return Self::ReadsWorkspace;
        }

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
            enforcement_level: self.sandbox_backend.backend.enforcement_level(),
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
    #[serde(default)]
    pub enforcement_level: SandboxEnforcementLevel,
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
pub struct SandboxRunnerDiagnostic {
    pub platform: String,
    pub status: SandboxRunnerStatus,
    pub backend: SandboxBackend,
    pub os_isolation: bool,
    pub enforcement: String,
    #[serde(default)]
    pub enforcement_level: SandboxEnforcementLevel,
    #[serde(default)]
    pub runner: String,
    #[serde(default)]
    pub unsupported_reason: Option<String>,
    pub command: Option<String>,
    pub cwd: PathBuf,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SandboxAttemptRecord {
    pub requested: SandboxRequirement,
    pub materialized_backend: SandboxBackend,
    #[serde(default)]
    pub enforcement_level: SandboxEnforcementLevel,
    pub cwd: PathBuf,
    pub workspace_roots: Vec<PathBuf>,
    pub network: NetworkDecision,
    pub escalation_reason: Option<String>,
    pub runner_status: SandboxRunnerStatus,
    pub fallback_available: bool,
}

impl SandboxAttemptRecord {
    pub fn from_diagnostic(
        requested: SandboxRequirement,
        network: NetworkDecision,
        workspace_roots: Vec<PathBuf>,
        diagnostic: SandboxRunnerDiagnostic,
    ) -> Self {
        Self {
            requested,
            materialized_backend: diagnostic.backend,
            enforcement_level: diagnostic.enforcement_level,
            cwd: diagnostic.cwd,
            workspace_roots,
            network,
            escalation_reason: diagnostic.message,
            runner_status: diagnostic.status,
            fallback_available: matches!(
                diagnostic.status,
                SandboxRunnerStatus::RequiresEscalation | SandboxRunnerStatus::Denied
            ),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandTarget {
    pub raw: String,
    pub kind: CommandTargetKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandTargetKind {
    Read,
    Write,
    Delete,
}

pub fn extract_command_targets(command: &str) -> Vec<CommandTarget> {
    let tokens = shell_tokens(command);
    let lower_tokens = tokens
        .iter()
        .map(|token| token.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let mut targets = Vec::new();

    for (index, token) in lower_tokens.iter().enumerate() {
        match token.as_str() {
            ">" | ">>" => {
                if let Some(raw) = next_positional_token(&tokens, index + 1) {
                    targets.push(CommandTarget {
                        raw,
                        kind: CommandTargetKind::Write,
                    });
                }
            }
            "out-file" => push_named_or_positional_target(
                &tokens,
                &lower_tokens,
                index,
                &["-filepath", "-literalpath", "-path"],
                CommandTargetKind::Write,
                &mut targets,
            ),
            "set-content" | "add-content" | "new-item" | "touch" | "mkdir" => {
                push_named_or_positional_target(
                    &tokens,
                    &lower_tokens,
                    index,
                    &["-path", "-literalpath", "-filepath"],
                    CommandTargetKind::Write,
                    &mut targets,
                );
            }
            "copy" | "cp" | "copy-item" | "move" | "mv" | "move-item" => {
                if let Some(raw) =
                    named_target_after(&tokens, &lower_tokens, index, &["-destination", "-to"])
                        .or_else(|| last_positional_after(&tokens, index + 1))
                {
                    targets.push(CommandTarget {
                        raw,
                        kind: CommandTargetKind::Write,
                    });
                }
            }
            "remove-item" | "rm" | "del" | "erase" | "rmdir" | "rd" => {
                if let Some(raw) =
                    named_target_after(&tokens, &lower_tokens, index, &["-path", "-literalpath"])
                {
                    targets.push(CommandTarget {
                        raw,
                        kind: CommandTargetKind::Delete,
                    });
                } else {
                    for raw in positional_tokens_after(&tokens, index + 1) {
                        targets.push(CommandTarget {
                            raw,
                            kind: CommandTargetKind::Delete,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    targets
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ApprovalCacheEntry {
    pub key: ApprovalKey,
    pub decision: CachedApprovalDecision,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ApprovalKey {
    pub tool_name: String,
    pub command: Option<String>,
    pub cwd: Option<PathBuf>,
    pub target_paths: Vec<PathBuf>,
    pub sandbox: Option<SandboxRequirement>,
    pub network: Option<NetworkPolicy>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CachedApprovalDecision {
    Approved,
    Declined,
    AskAgain,
    Unavailable,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionApprovalCache {
    pub entries: Vec<ApprovalCacheEntry>,
}

impl SessionApprovalCache {
    pub fn remember(&mut self, entry: ApprovalCacheEntry) {
        self.entries.retain(|candidate| candidate.key != entry.key);
        self.entries.push(entry);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxRunnerStatus {
    Ready,
    RequiresEscalation,
    Denied,
    Timeout,
    SpawnFailed,
    NonZeroExit,
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

    pub fn diagnostic(
        &self,
        policy: &ExecutionPolicy,
        cwd: &Path,
        command: Option<&str>,
    ) -> SandboxRunnerDiagnostic {
        let evaluation = policy.evaluate(cwd, command);
        let plan = evaluation.execution_plan();
        let status = if plan.escalation_required {
            SandboxRunnerStatus::RequiresEscalation
        } else if plan.approval_required || !plan.allowed {
            SandboxRunnerStatus::Denied
        } else {
            SandboxRunnerStatus::Ready
        };
        SandboxRunnerDiagnostic {
            platform: platform_name().to_string(),
            status,
            backend: plan.backend,
            os_isolation: plan.backend.os_isolation(),
            enforcement: plan.backend.enforcement_label().to_string(),
            enforcement_level: plan.enforcement_level,
            runner: plan.backend.runner_label().to_string(),
            unsupported_reason: plan.backend.unsupported_reason().map(ToString::to_string),
            command: command.map(ToString::to_string),
            cwd: cwd.to_path_buf(),
            message: plan.denial_reason,
        }
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxEnforcementLevel {
    NoPolicyGuard,
    #[default]
    PolicyOnly,
    ProcessLifecycle,
    OsRestricted,
    PolicyBypass,
}

impl SandboxEnforcementLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoPolicyGuard => "no_policy_guard",
            Self::PolicyOnly => "policy_only",
            Self::ProcessLifecycle => "process_lifecycle",
            Self::OsRestricted => "os_restricted",
            Self::PolicyBypass => "policy_bypass",
        }
    }

    pub fn os_isolation(self) -> bool {
        matches!(self, Self::OsRestricted)
    }
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

impl SandboxBackend {
    pub fn user_facing_label(self) -> &'static str {
        match self {
            Self::None => "no policy guard",
            Self::DangerFullAccess => "policy bypass: danger-full-access",
            Self::WorkspaceGuard => "policy-only guard: advisory checks, no OS isolation",
            Self::WindowsRestrictedToken => {
                "process lifecycle runner: policy guard only, no filesystem OS isolation"
            }
            Self::LinuxLandlock => {
                "policy-only Landlock entrypoint: OS isolation not verified in this build"
            }
        }
    }

    pub fn os_isolation(self) -> bool {
        self.enforcement_level().os_isolation()
    }

    pub fn enforcement_level(self) -> SandboxEnforcementLevel {
        match self {
            Self::None => SandboxEnforcementLevel::NoPolicyGuard,
            Self::DangerFullAccess => SandboxEnforcementLevel::PolicyBypass,
            Self::WorkspaceGuard => SandboxEnforcementLevel::PolicyOnly,
            Self::WindowsRestrictedToken => SandboxEnforcementLevel::ProcessLifecycle,
            Self::LinuxLandlock => SandboxEnforcementLevel::PolicyOnly,
        }
    }

    pub fn enforcement_label(self) -> &'static str {
        self.enforcement_level().as_str()
    }

    pub fn runner_label(self) -> &'static str {
        match self {
            Self::None => "direct_process",
            Self::DangerFullAccess => "direct_process_policy_bypass",
            Self::WorkspaceGuard => "workspace_policy_guard",
            Self::WindowsRestrictedToken => "windows_process_lifecycle",
            Self::LinuxLandlock => "linux_landlock_entrypoint",
        }
    }

    pub fn unsupported_reason(self) -> Option<&'static str> {
        match self {
            Self::WorkspaceGuard => {
                Some("workspace guard is advisory policy only; no OS isolation is active")
            }
            Self::WindowsRestrictedToken => Some(
                "Windows restricted-token filesystem isolation is not enabled; runner provides policy guard and child process lifecycle only",
            ),
            Self::LinuxLandlock => Some(
                "Linux Landlock runner entrypoint is present but not verified in this build; policy guard remains authoritative",
            ),
            Self::None | Self::DangerFullAccess => None,
        }
    }
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

fn platform_name() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "unknown"
    }
}

fn shell_tokens(command: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut chars = command.chars().peekable();

    while let Some(ch) = chars.next() {
        if let Some(quote_char) = quote {
            if ch == quote_char {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }

        match ch {
            '"' | '\'' => quote = Some(ch),
            '>' | '<' => {
                push_current_token(&mut tokens, &mut current);
                let mut redirect = ch.to_string();
                if chars.peek().copied() == Some(ch) {
                    redirect.push(chars.next().expect("peeked redirect"));
                }
                tokens.push(redirect);
            }
            ';' | '|' | '&' | '(' | ')' | '{' | '}' | '\r' | '\n' | '\t' | ' ' => {
                push_current_token(&mut tokens, &mut current);
            }
            _ => current.push(ch),
        }
    }
    push_current_token(&mut tokens, &mut current);
    tokens
}

fn push_current_token(tokens: &mut Vec<String>, current: &mut String) {
    if !current.is_empty() {
        tokens.push(std::mem::take(current));
    }
}

fn tokenized_destructive(tokens: &[String]) -> bool {
    tokens
        .iter()
        .enumerate()
        .any(|(index, token)| match token.as_str() {
            "format" | "erase" => true,
            "remove-item" => {
                has_any_token(tokens, index + 1, &["-recurse", "-r"])
                    || has_any_token(tokens, index + 1, &["-force", "-f"])
                    || next_positional_token(tokens, index + 1).is_some()
            }
            "rm" => {
                has_rm_recursive_force(tokens, index + 1)
                    || tokens
                        .iter()
                        .skip(index + 1)
                        .any(|candidate| matches!(candidate.as_str(), "-rf" | "-fr"))
            }
            "del" | "rmdir" | "rd" => has_any_token(tokens, index + 1, &["/s", "-s"]),
            _ => false,
        })
}

fn tokenized_process_control(tokens: &[String]) -> bool {
    tokens.iter().any(|token| {
        matches!(
            token.as_str(),
            "taskkill" | "kill" | "pkill" | "stop-process" | "shutdown" | "restart-computer"
        )
    })
}

fn tokenized_credential_access(tokens: &[String]) -> bool {
    tokens.iter().any(|token| {
        token.contains("api_key")
            || token.contains("apikey")
            || token.contains("password")
            || token.contains("passwd")
            || token.contains("token")
            || token.contains("credential")
            || token.contains("secret")
            || token.ends_with(".env")
            || token.ends_with("id_rsa")
            || token.ends_with("credentials")
    })
}

fn tokenized_network(tokens: &[String]) -> bool {
    tokens.iter().enumerate().any(|(index, token)| {
        matches!(
            token.as_str(),
            "curl" | "wget" | "invoke-webrequest" | "invoke-restmethod" | "irm"
        ) || matches!(token.as_str(), "npm" | "cargo" | "pip")
            && tokens.get(index + 1).is_some_and(|next| next == "install")
    })
}

fn tokenized_write(tokens: &[String]) -> bool {
    tokens.iter().any(|token| {
        matches!(
            token.as_str(),
            ">" | ">>"
                | "copy"
                | "cp"
                | "mv"
                | "move"
                | "copy-item"
                | "move-item"
                | "new-item"
                | "set-content"
                | "out-file"
                | "add-content"
                | "touch"
                | "mkdir"
                | "apply_patch"
        )
    })
}

fn tokenized_read(tokens: &[String]) -> bool {
    tokens.iter().any(|token| {
        matches!(
            token.as_str(),
            "cat" | "type" | "get-content" | "rg" | "ripgrep" | "findstr"
        )
    })
}

fn has_rm_recursive_force(tokens: &[String], start: usize) -> bool {
    let mut recursive = false;
    let mut force = false;
    for token in tokens.iter().skip(start) {
        if !token.starts_with('-') {
            continue;
        }
        recursive |= token == "-r" || token == "-R" || token == "--recursive";
        force |= token == "-f" || token == "--force";
        if token.starts_with('-') && token.contains('r') && token.contains('f') {
            recursive = true;
            force = true;
        }
    }
    recursive && force
}

fn has_any_token(tokens: &[String], start: usize, needles: &[&str]) -> bool {
    tokens
        .iter()
        .skip(start)
        .any(|token| needles.iter().any(|needle| token == needle))
}

fn push_named_or_positional_target(
    tokens: &[String],
    lower_tokens: &[String],
    command_index: usize,
    names: &[&str],
    kind: CommandTargetKind,
    targets: &mut Vec<CommandTarget>,
) {
    if let Some(raw) = named_target_after(tokens, lower_tokens, command_index, names)
        .or_else(|| next_positional_token(tokens, command_index + 1))
    {
        targets.push(CommandTarget { raw, kind });
    }
}

fn named_target_after(
    tokens: &[String],
    lower_tokens: &[String],
    command_index: usize,
    names: &[&str],
) -> Option<String> {
    lower_tokens
        .iter()
        .enumerate()
        .skip(command_index + 1)
        .find_map(|(index, token)| {
            names
                .iter()
                .any(|name| token == name)
                .then(|| next_positional_token(tokens, index + 1))
                .flatten()
        })
}

fn next_positional_token(tokens: &[String], start: usize) -> Option<String> {
    tokens
        .iter()
        .skip(start)
        .find(|token| !is_option_like(token))
        .map(|token| clean_target_token(token))
        .filter(|token| !token.is_empty())
}

fn last_positional_after(tokens: &[String], start: usize) -> Option<String> {
    tokens
        .iter()
        .skip(start)
        .filter(|token| !is_option_like(token))
        .map(|token| clean_target_token(token))
        .filter(|token| !token.is_empty())
        .last()
}

fn positional_tokens_after(tokens: &[String], start: usize) -> Vec<String> {
    tokens
        .iter()
        .skip(start)
        .filter(|token| !is_option_like(token))
        .map(|token| clean_target_token(token))
        .filter(|token| !token.is_empty())
        .collect()
}

fn is_option_like(token: &str) -> bool {
    token.starts_with('-')
        || (cfg!(windows)
            && token.starts_with('/')
            && !token.contains('\\')
            && !token.contains(':'))
}

fn clean_target_token(token: &str) -> String {
    token
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim_end_matches(';')
        .to_string()
}

fn workspace_write_target_denial(root: &Path, cwd: &Path, command: &str) -> Option<String> {
    extract_command_targets(command)
        .into_iter()
        .find(|target| {
            matches!(
                target.kind,
                CommandTargetKind::Write | CommandTargetKind::Delete
            ) && !target_within_workspace(root, cwd, &target.raw)
        })
        .map(|target| {
            format!(
                "workspace-write target {} is outside workspace {}",
                target.raw,
                root.display()
            )
        })
}

fn target_within_workspace(root: &Path, cwd: &Path, raw: &str) -> bool {
    let raw = clean_target_token(raw);
    if raw.is_empty() || raw == "-" {
        return true;
    }
    let root = absolute_normalized(root);
    let cwd = absolute_normalized(cwd);
    let target = PathBuf::from(raw);
    let target = if target.is_absolute() {
        normalize_components(&target)
    } else {
        normalize_components(&cwd.join(target))
    };
    path_has_prefix(&target, &root)
}

fn absolute_normalized(path: &Path) -> PathBuf {
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return normalize_components(&canonical);
    }

    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };

    canonicalize_existing_prefix(&absolute).unwrap_or_else(|| normalize_components(&absolute))
}

fn canonicalize_existing_prefix(path: &Path) -> Option<PathBuf> {
    let mut existing = path.to_path_buf();
    let mut missing = Vec::new();

    while !existing.exists() {
        let name = existing.file_name()?.to_os_string();
        missing.push(name);
        if !existing.pop() {
            return None;
        }
    }

    let mut resolved = std::fs::canonicalize(existing).ok()?;
    for component in missing.iter().rev() {
        resolved.push(component);
    }
    Some(normalize_components(&resolved))
}

fn normalize_components(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
        }
    }
    normalized
}

fn path_has_prefix(path: &Path, root: &Path) -> bool {
    let path_components = comparable_components(path);
    let root_components = comparable_components(root);
    path_components.len() >= root_components.len()
        && path_components
            .iter()
            .zip(root_components.iter())
            .all(|(left, right)| left == right)
}

fn comparable_components(path: &Path) -> Vec<String> {
    path.components()
        .map(|component| {
            let value = component.as_os_str().to_string_lossy().to_string();
            if cfg!(windows) {
                value.to_ascii_lowercase()
            } else {
                value
            }
        })
        .collect()
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

    #[cfg(windows)]
    fn create_dir_symlink(src: &Path, dst: &Path) -> std::io::Result<()> {
        std::os::windows::fs::symlink_dir(src, dst)
    }

    #[cfg(unix)]
    fn create_dir_symlink(src: &Path, dst: &Path) -> std::io::Result<()> {
        std::os::unix::fs::symlink(src, dst)
    }

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
            CommandRisk::classify("rm  -rf target"),
            CommandRisk::Destructive
        );
        assert_eq!(
            CommandRisk::classify("rm -r -f target"),
            CommandRisk::Destructive
        );
        assert_eq!(
            CommandRisk::classify("Remove-Item -Recurse -Force target"),
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
        assert_eq!(CommandRisk::classify("echo harmless"), CommandRisk::Low);
    }

    #[test]
    fn extracts_common_write_and_delete_targets() {
        assert_eq!(
            extract_command_targets("echo hi > inside.txt"),
            vec![CommandTarget {
                raw: "inside.txt".to_string(),
                kind: CommandTargetKind::Write
            }]
        );
        assert_eq!(
            extract_command_targets("Set-Content -Path notes.txt -Value hi"),
            vec![CommandTarget {
                raw: "notes.txt".to_string(),
                kind: CommandTargetKind::Write
            }]
        );
        assert_eq!(
            extract_command_targets("Remove-Item -Recurse -Force ..\\outside.txt"),
            vec![CommandTarget {
                raw: "..\\outside.txt".to_string(),
                kind: CommandTargetKind::Delete
            }]
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
    fn workspace_write_blocks_obvious_outside_targets() {
        let workspace = TempDir::new().expect("workspace");
        let policy = ExecutionPolicy {
            approval: ApprovalRequirement::PreApproved,
            sandbox: SandboxRequirement::WorkspaceWrite,
            network: NetworkPolicy::Inherit,
            workspace_root: workspace.path().to_path_buf(),
        };

        let parent_escape = policy.evaluate(workspace.path(), Some("echo hi > ..\\outside.txt"));
        assert!(matches!(
            parent_escape.decision,
            PolicyDecision::Blocked { ref reason }
                if reason.contains("workspace-write target")
                    && reason.contains("outside workspace")
        ));

        let absolute_outside = if cfg!(windows) {
            "echo hi > C:\\Temp\\outside.txt"
        } else {
            "echo hi > /tmp/outside.txt"
        };
        let absolute = policy.evaluate(workspace.path(), Some(absolute_outside));
        assert!(matches!(
            absolute.decision,
            PolicyDecision::Blocked { ref reason }
                if reason.contains("workspace-write target")
                    && reason.contains("outside workspace")
        ));

        assert!(matches!(
            policy
                .evaluate(workspace.path(), Some("echo hi > inside.txt"))
                .decision,
            PolicyDecision::Allowed
        ));
    }

    #[test]
    fn workspace_write_blocks_symlink_targets_outside_workspace() {
        let workspace = TempDir::new().expect("workspace");
        let outside = TempDir::new().expect("outside");
        let link = workspace.path().join("outside-link");
        if let Err(error) = create_dir_symlink(outside.path(), &link) {
            eprintln!("skipping symlink escape test; symlink unavailable: {error}");
            return;
        }
        let policy = ExecutionPolicy {
            approval: ApprovalRequirement::PreApproved,
            sandbox: SandboxRequirement::WorkspaceWrite,
            network: NetworkPolicy::Inherit,
            workspace_root: workspace.path().to_path_buf(),
        };
        let command = if cfg!(windows) {
            "echo hi > outside-link\\leak.txt"
        } else {
            "echo hi > outside-link/leak.txt"
        };

        let evaluation = policy.evaluate(workspace.path(), Some(command));

        assert!(matches!(
            evaluation.decision,
            PolicyDecision::Blocked { ref reason }
                if reason.contains("workspace-write target")
                    && reason.contains("outside workspace")
        ));
        assert!(!outside.path().join("leak.txt").exists());
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

    #[test]
    fn sandbox_runner_diagnostic_is_honest_about_platform_runner() {
        let workspace = TempDir::new().expect("workspace");
        let policy = ExecutionPolicy {
            approval: ApprovalRequirement::PreApproved,
            sandbox: SandboxRequirement::WorkspaceWrite,
            network: NetworkPolicy::Inherit,
            workspace_root: workspace.path().to_path_buf(),
        };

        let diagnostic = SandboxRunner.diagnostic(&policy, workspace.path(), Some("echo yunxi"));

        assert_eq!(diagnostic.status, SandboxRunnerStatus::Ready);
        assert!(!diagnostic.os_isolation);
        assert_eq!(
            diagnostic.enforcement,
            diagnostic.enforcement_level.as_str()
        );
        if cfg!(windows) {
            assert_eq!(
                diagnostic.enforcement_level,
                SandboxEnforcementLevel::ProcessLifecycle
            );
        } else {
            assert_eq!(
                diagnostic.enforcement_level,
                SandboxEnforcementLevel::PolicyOnly
            );
        }
        assert!(!diagnostic.runner.is_empty());
        assert!(
            diagnostic
                .unsupported_reason
                .as_deref()
                .is_some_and(|reason| {
                    reason.contains("policy guard") || reason.contains("not verified")
                })
        );
    }

    #[test]
    fn danger_full_access_reports_policy_bypass_not_isolation() {
        let workspace = TempDir::new().expect("workspace");
        let policy = ExecutionPolicy {
            approval: ApprovalRequirement::PreApproved,
            sandbox: SandboxRequirement::DangerFullAccess,
            network: NetworkPolicy::Inherit,
            workspace_root: workspace.path().to_path_buf(),
        };

        let diagnostic =
            SandboxRunner.diagnostic(&policy, workspace.path(), Some("echo yunxi > file.txt"));

        assert_eq!(
            diagnostic.enforcement_level,
            SandboxEnforcementLevel::PolicyBypass
        );
        assert_eq!(diagnostic.enforcement, "policy_bypass");
        assert!(!diagnostic.os_isolation);
        assert!(diagnostic.unsupported_reason.is_none());
    }
}

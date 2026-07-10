use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use tokio::process::Command;
use yunxi_agent_core::{AgentConfig, AgentError, AgentResult, ApprovalMode, SandboxMode};
use yunxi_agent_exec::{OutputLimits, combine_output, truncate_output};
use yunxi_agent_patch::{PatchFileChangeKind, apply_patch};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolRequest {
    pub id: Option<String>,
    pub cwd: PathBuf,
    pub kind: ToolRequestKind,
    pub policy: ToolPolicy,
}

impl ToolRequest {
    pub fn shell(cwd: impl Into<PathBuf>, command: impl Into<String>) -> Self {
        Self {
            id: None,
            cwd: cwd.into(),
            kind: ToolRequestKind::Shell {
                command: command.into(),
            },
            policy: ToolPolicy::trusted(),
        }
    }

    pub fn patch(cwd: impl Into<PathBuf>, patch: impl Into<String>) -> Self {
        Self {
            id: None,
            cwd: cwd.into(),
            kind: ToolRequestKind::Patch {
                patch: patch.into(),
            },
            policy: ToolPolicy::trusted(),
        }
    }

    pub fn with_policy(mut self, policy: ToolPolicy) -> Self {
        self.policy = policy;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ToolRequestKind {
    Shell {
        command: String,
    },
    Patch {
        patch: String,
    },
    Mcp {
        server: String,
        tool: String,
        arguments_json: Option<String>,
    },
    Skill {
        name: String,
        arguments_json: Option<String>,
    },
}

impl ToolRequestKind {
    pub fn tool_name(&self) -> ToolName {
        match self {
            Self::Shell { .. } => ToolName::Shell,
            Self::Patch { .. } => ToolName::Patch,
            Self::Mcp { .. } => ToolName::Mcp,
            Self::Skill { .. } => ToolName::Skill,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolName {
    Shell,
    Patch,
    Mcp,
    Skill,
}

impl ToolName {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Shell => "shell",
            Self::Patch => "patch",
            Self::Mcp => "mcp",
            Self::Skill => "skill",
        }
    }
}

impl Default for ToolName {
    fn default() -> Self {
        Self::Shell
    }
}

impl std::fmt::Display for ToolName {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: ToolName,
    pub description: String,
    pub parameters: Value,
    pub model_visible: bool,
}

impl ToolSpec {
    pub fn new(
        name: ToolName,
        description: impl Into<String>,
        parameters: Value,
        model_visible: bool,
    ) -> Self {
        Self {
            name,
            description: description.into(),
            parameters,
            model_visible,
        }
    }

    pub fn openai_tool_json(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": self.name.as_str(),
                "description": self.description.clone(),
                "parameters": self.parameters.clone(),
            }
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolRegistry {
    specs: BTreeMap<ToolName, ToolSpec>,
}

impl ToolRegistry {
    pub fn new(specs: impl IntoIterator<Item = ToolSpec>) -> Self {
        Self {
            specs: specs
                .into_iter()
                .map(|spec| (spec.name, spec))
                .collect::<BTreeMap<_, _>>(),
        }
    }

    pub fn spec(&self, name: ToolName) -> Option<&ToolSpec> {
        self.specs.get(&name)
    }

    pub fn specs(&self) -> impl Iterator<Item = &ToolSpec> {
        self.specs.values()
    }

    pub fn model_visible_specs(&self) -> Vec<&ToolSpec> {
        self.specs
            .values()
            .filter(|spec| spec.model_visible)
            .collect()
    }

    pub fn openai_tools_json(&self) -> Vec<Value> {
        self.model_visible_specs()
            .into_iter()
            .map(ToolSpec::openai_tool_json)
            .collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        default_tool_registry()
    }
}

pub fn default_tool_registry() -> ToolRegistry {
    ToolRegistry::new([
        shell_tool_spec(),
        patch_tool_spec(),
        mcp_tool_spec(),
        skill_tool_spec(),
    ])
}

fn shell_tool_spec() -> ToolSpec {
    ToolSpec::new(
        ToolName::Shell,
        "Run a shell command inside the configured YunXi workspace.",
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Command to execute with the platform shell."
                }
            },
            "required": ["command"],
            "additionalProperties": false
        }),
        true,
    )
}

fn patch_tool_spec() -> ToolSpec {
    ToolSpec::new(
        ToolName::Patch,
        "Apply a constrained YunXi patch operation inside the configured workspace.",
        json!({
            "type": "object",
            "properties": {
                "op": {
                    "type": "string",
                    "enum": ["write", "delete"],
                    "description": "Patch operation to apply."
                },
                "path": {
                    "type": "string",
                    "description": "Workspace-relative file path."
                },
                "content": {
                    "type": "string",
                    "description": "File content for write operations."
                }
            },
            "required": ["op", "path"],
            "additionalProperties": false
        }),
        true,
    )
}

fn mcp_tool_spec() -> ToolSpec {
    ToolSpec::new(
        ToolName::Mcp,
        "Call a registered YunXi MCP server tool.",
        json!({
            "type": "object",
            "properties": {
                "server": {
                    "type": "string",
                    "description": "MCP server name."
                },
                "tool": {
                    "type": "string",
                    "description": "Tool name on the MCP server."
                },
                "arguments_json": {
                    "type": "string",
                    "description": "Optional serialized JSON object for tool arguments."
                }
            },
            "required": ["server", "tool"],
            "additionalProperties": false
        }),
        true,
    )
}

fn skill_tool_spec() -> ToolSpec {
    ToolSpec::new(
        ToolName::Skill,
        "Invoke a registered YunXi skill by name.",
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Skill name."
                },
                "arguments_json": {
                    "type": "string",
                    "description": "Optional serialized JSON object for skill arguments."
                }
            },
            "required": ["name"],
            "additionalProperties": false
        }),
        true,
    )
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolRoute {
    pub name: ToolName,
    pub model_visible: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolDispatch {
    pub request: ToolRequest,
    pub route: ToolRoute,
    pub trace: ToolDispatchTrace,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolDispatchTrace {
    pub request_id: Option<String>,
    pub tool_name: ToolName,
    pub route_status: ToolRouteStatus,
    pub policy_decision: ToolPolicyDecision,
}

impl ToolDispatchTrace {
    pub fn summary(&self) -> String {
        let id = self.request_id.as_deref().unwrap_or("none");
        let policy = match &self.policy_decision {
            ToolPolicyDecision::Approved => "approved".to_string(),
            ToolPolicyDecision::Declined { reason } => format!("declined:{reason}"),
        };
        format!(
            "Tool dispatch routed {} (id={id}, route={:?}, policy={policy})",
            self.tool_name, self.route_status
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolRouteStatus {
    Routed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolPolicyDecision {
    Approved,
    Declined { reason: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolRouter {
    registry: ToolRegistry,
}

impl ToolRouter {
    pub fn new(registry: ToolRegistry) -> Self {
        Self { registry }
    }

    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }

    pub fn route(&self, request: ToolRequest) -> AgentResult<ToolDispatch> {
        let tool_name = request.kind.tool_name();
        let spec = self
            .registry
            .spec(tool_name)
            .ok_or_else(|| AgentError::Execution {
                message: format!("tool is not registered in YunXi router: {tool_name}"),
            })?;
        let route = ToolRoute {
            name: tool_name,
            model_visible: spec.model_visible,
        };
        let trace = ToolDispatchTrace {
            request_id: request.id.clone(),
            tool_name,
            route_status: ToolRouteStatus::Routed,
            policy_decision: request.policy.decision_for(&request),
        };
        Ok(ToolDispatch {
            request,
            route,
            trace,
        })
    }
}

impl Default for ToolRouter {
    fn default() -> Self {
        Self::new(default_tool_registry())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolResponse {
    pub id: Option<String>,
    pub status: ToolStatus,
    pub output: Option<String>,
    pub error: Option<String>,
    pub exit_code: Option<i32>,
    pub changed_files: Vec<ToolFileChange>,
}

impl ToolResponse {
    pub fn completed(
        id: Option<String>,
        output: impl Into<String>,
        exit_code: Option<i32>,
        changed_files: Vec<ToolFileChange>,
    ) -> Self {
        Self {
            id,
            status: ToolStatus::Completed,
            output: Some(output.into()),
            error: None,
            exit_code,
            changed_files,
        }
    }

    pub fn failed(
        id: Option<String>,
        output: impl Into<String>,
        exit_code: Option<i32>,
        changed_files: Vec<ToolFileChange>,
    ) -> Self {
        Self {
            id,
            status: ToolStatus::Failed,
            output: Some(output.into()),
            error: None,
            exit_code,
            changed_files,
        }
    }

    pub fn declined(id: Option<String>, message: impl Into<String>) -> Self {
        Self {
            id,
            status: ToolStatus::Declined,
            output: None,
            error: Some(message.into()),
            exit_code: None,
            changed_files: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolFileChange {
    pub path: PathBuf,
    pub kind: ToolFileChangeKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolFileChangeKind {
    Added,
    Deleted,
    Updated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    InProgress,
    Completed,
    Failed,
    Declined,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolPolicy {
    pub approval: ApprovalDecision,
    pub sandbox: SandboxPolicy,
    pub workspace_root: Option<PathBuf>,
}

impl ToolPolicy {
    pub fn trusted() -> Self {
        Self {
            approval: ApprovalDecision::Approved,
            sandbox: SandboxPolicy::DangerFullAccess,
            workspace_root: None,
        }
    }

    pub fn from_config(config: &AgentConfig) -> Self {
        Self {
            approval: match config.approval_mode {
                ApprovalMode::Never => ApprovalDecision::Approved,
                ApprovalMode::OnRequest | ApprovalMode::OnFailure | ApprovalMode::Untrusted => {
                    ApprovalDecision::Required
                }
            },
            sandbox: match config.sandbox_mode {
                SandboxMode::ReadOnly => SandboxPolicy::ReadOnly,
                SandboxMode::WorkspaceWrite => SandboxPolicy::WorkspaceWrite,
                SandboxMode::DangerFullAccess => SandboxPolicy::DangerFullAccess,
            },
            workspace_root: Some(config.cwd.clone()),
        }
    }

    pub fn decision_for(&self, request: &ToolRequest) -> ToolPolicyDecision {
        match self.denial_for(request) {
            Some(reason) => ToolPolicyDecision::Declined { reason },
            None => ToolPolicyDecision::Approved,
        }
    }

    fn denial_for(&self, request: &ToolRequest) -> Option<String> {
        match &self.approval {
            ApprovalDecision::Approved => {}
            ApprovalDecision::Required => {
                return Some("tool execution requires approval".to_string());
            }
            ApprovalDecision::Declined { reason } => {
                return Some(reason.clone());
            }
        }

        match self.sandbox {
            SandboxPolicy::ReadOnly => Some("sandbox is read-only".to_string()),
            SandboxPolicy::WorkspaceWrite => {
                let root = self.workspace_root.as_ref()?;
                if is_within_workspace(root, &request.cwd) {
                    None
                } else {
                    Some(format!(
                        "tool cwd {} is outside workspace {}",
                        request.cwd.display(),
                        root.display()
                    ))
                }
            }
            SandboxPolicy::DangerFullAccess => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    Approved,
    Required,
    Declined { reason: String },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SandboxPolicy {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

#[async_trait]
pub trait ToolRuntime: Send + Sync {
    async fn execute(&self, request: ToolRequest) -> AgentResult<ToolResponse>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NoopToolRuntime;

#[async_trait]
impl ToolRuntime for NoopToolRuntime {
    async fn execute(&self, request: ToolRequest) -> AgentResult<ToolResponse> {
        Ok(ToolResponse::declined(
            request.id,
            "YunXi tool execution is not wired in this runtime slice",
        ))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ShellToolRuntime;

#[async_trait]
impl ToolRuntime for ShellToolRuntime {
    async fn execute(&self, request: ToolRequest) -> AgentResult<ToolResponse> {
        if let Some(reason) = request.policy.denial_for(&request) {
            return Ok(ToolResponse::declined(request.id, reason));
        }

        match request.kind {
            ToolRequestKind::Shell { command } => run_shell(request.id, request.cwd, command).await,
            ToolRequestKind::Patch { patch } => run_patch(request.id, request.cwd, patch),
            ToolRequestKind::Mcp { .. } | ToolRequestKind::Skill { .. } => {
                Ok(ToolResponse::declined(
                    request.id,
                    "YunXi only owns shell and constrained patch execution in this runtime slice",
                ))
            }
        }
    }
}

async fn run_shell(id: Option<String>, cwd: PathBuf, command: String) -> AgentResult<ToolResponse> {
    let before = WorkspaceSnapshot::capture(&cwd)?;
    let mut process = platform_shell(&command);
    process.current_dir(&cwd);

    let output = process
        .output()
        .await
        .map_err(|error| AgentError::Execution {
            message: format!("shell command failed to start: {error}"),
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = truncate_output(&combine_output(&stdout, &stderr), OutputLimits::default());

    let changed_files = before.diff(&WorkspaceSnapshot::capture(&cwd)?);
    let exit_code = output.status.code();
    if output.status.success() {
        Ok(ToolResponse::completed(
            id,
            combined,
            exit_code,
            changed_files,
        ))
    } else {
        Ok(ToolResponse::failed(id, combined, exit_code, changed_files))
    }
}

#[cfg(windows)]
fn platform_shell(command: &str) -> Command {
    let mut process = Command::new("cmd");
    process.args(["/C", command]);
    process
}

fn run_patch(id: Option<String>, cwd: PathBuf, patch: String) -> AgentResult<ToolResponse> {
    let report = apply_patch(&cwd, &patch)?;
    let changed_files = report
        .changed_files
        .into_iter()
        .map(|change| ToolFileChange {
            path: change.path,
            kind: match change.kind {
                PatchFileChangeKind::Added => ToolFileChangeKind::Added,
                PatchFileChangeKind::Updated => ToolFileChangeKind::Updated,
                PatchFileChangeKind::Deleted => ToolFileChangeKind::Deleted,
            },
        })
        .collect();

    Ok(ToolResponse::completed(
        id,
        "patch applied",
        Some(0),
        changed_files,
    ))
}

#[cfg(not(windows))]
fn platform_shell(command: &str) -> Command {
    let mut process = Command::new("sh");
    process.args(["-c", command]);
    process
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct WorkspaceSnapshot {
    files: BTreeMap<PathBuf, FileState>,
}

impl WorkspaceSnapshot {
    fn capture(root: &Path) -> AgentResult<Self> {
        let mut snapshot = Self::default();
        if !root.is_dir() {
            return Ok(snapshot);
        }
        snapshot.capture_dir(root, root)?;
        Ok(snapshot)
    }

    fn capture_dir(&mut self, root: &Path, dir: &Path) -> AgentResult<()> {
        for entry in std::fs::read_dir(dir).map_err(|error| AgentError::Execution {
            message: format!(
                "failed to read workspace directory {}: {error}",
                dir.display()
            ),
        })? {
            let entry = entry.map_err(|error| AgentError::Execution {
                message: format!("failed to read workspace entry: {error}"),
            })?;
            let path = entry.path();
            let name = entry.file_name();
            if should_skip_entry(&name.to_string_lossy()) {
                continue;
            }
            let metadata = entry.metadata().map_err(|error| AgentError::Execution {
                message: format!("failed to read metadata for {}: {error}", path.display()),
            })?;
            if metadata.is_dir() {
                self.capture_dir(root, &path)?;
            } else if metadata.is_file() {
                let relative = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
                self.files
                    .insert(relative, FileState::from_metadata(metadata));
            }
        }
        Ok(())
    }

    fn diff(&self, after: &Self) -> Vec<ToolFileChange> {
        let paths = self
            .files
            .keys()
            .chain(after.files.keys())
            .cloned()
            .collect::<BTreeSet<_>>();

        paths
            .into_iter()
            .filter_map(
                |path| match (self.files.get(&path), after.files.get(&path)) {
                    (None, Some(_)) => Some(ToolFileChange {
                        path,
                        kind: ToolFileChangeKind::Added,
                    }),
                    (Some(_), None) => Some(ToolFileChange {
                        path,
                        kind: ToolFileChangeKind::Deleted,
                    }),
                    (Some(before), Some(after)) if before != after => Some(ToolFileChange {
                        path,
                        kind: ToolFileChangeKind::Updated,
                    }),
                    _ => None,
                },
            )
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FileState {
    len: u64,
    modified_millis: u128,
}

impl FileState {
    fn from_metadata(metadata: std::fs::Metadata) -> Self {
        let modified_millis = metadata
            .modified()
            .ok()
            .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis())
            .unwrap_or_default();
        Self {
            len: metadata.len(),
            modified_millis,
        }
    }
}

fn should_skip_entry(name: &str) -> bool {
    matches!(name, ".git" | ".yunxi" | "target" | "vendor")
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

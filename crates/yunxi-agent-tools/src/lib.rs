use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;
use tokio::process::Command;
use yunxi_agent_core::{AgentConfig, AgentError, AgentResult, ApprovalMode, SandboxMode};

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
    let mut combined = String::new();
    combined.push_str(&stdout);
    if !stderr.is_empty() {
        if !combined.is_empty() && !combined.ends_with('\n') {
            combined.push('\n');
        }
        combined.push_str(&stderr);
    }

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
    let operations = parse_patch_operations(&patch)?;
    let mut changed_files = Vec::new();

    for operation in operations {
        match operation {
            PatchOperation::Write { path, content } => {
                let relative = validate_relative_path(&path)?;
                let full_path = cwd.join(&relative);
                let kind = if full_path.is_file() {
                    ToolFileChangeKind::Updated
                } else {
                    ToolFileChangeKind::Added
                };
                if let Some(parent) = full_path.parent() {
                    std::fs::create_dir_all(parent).map_err(|error| AgentError::Execution {
                        message: format!("failed to create patch parent directory: {error}"),
                    })?;
                }
                std::fs::write(&full_path, content).map_err(|error| AgentError::Execution {
                    message: format!(
                        "failed to write patch file {}: {error}",
                        full_path.display()
                    ),
                })?;
                changed_files.push(ToolFileChange {
                    path: relative,
                    kind,
                });
            }
            PatchOperation::Delete { path } => {
                let relative = validate_relative_path(&path)?;
                let full_path = cwd.join(&relative);
                if full_path.is_file() {
                    std::fs::remove_file(&full_path).map_err(|error| AgentError::Execution {
                        message: format!(
                            "failed to delete patch file {}: {error}",
                            full_path.display()
                        ),
                    })?;
                    changed_files.push(ToolFileChange {
                        path: relative,
                        kind: ToolFileChangeKind::Deleted,
                    });
                } else {
                    return Ok(ToolResponse::failed(
                        id,
                        format!("patch delete target does not exist: {}", relative.display()),
                        None,
                        Vec::new(),
                    ));
                }
            }
        }
    }

    Ok(ToolResponse::completed(
        id,
        "patch applied",
        Some(0),
        changed_files,
    ))
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum PatchOperation {
    Write { path: PathBuf, content: String },
    Delete { path: PathBuf },
}

fn parse_patch_operations(patch: &str) -> AgentResult<Vec<PatchOperation>> {
    let value = serde_json::from_str::<Value>(patch).map_err(|error| AgentError::Execution {
        message: format!("failed to parse constrained patch JSON: {error}"),
    })?;
    if value.is_array() {
        return serde_json::from_value::<Vec<PatchOperation>>(value).map_err(|error| {
            AgentError::Execution {
                message: format!("failed to parse constrained patch operations: {error}"),
            }
        });
    }
    serde_json::from_value::<PatchOperation>(value)
        .map(|operation| vec![operation])
        .map_err(|error| AgentError::Execution {
            message: format!("failed to parse constrained patch operation: {error}"),
        })
}

fn validate_relative_path(path: &Path) -> AgentResult<PathBuf> {
    if path.is_absolute() {
        return Err(AgentError::Execution {
            message: format!("patch path must be relative: {}", path.display()),
        });
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(AgentError::Execution {
            message: format!("patch path cannot escape workspace: {}", path.display()),
        });
    }
    Ok(path.to_path_buf())
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

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use tokio::process::Command;
use yunxi_agent_core::{AgentError, AgentResult};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolRequest {
    pub id: Option<String>,
    pub cwd: PathBuf,
    pub kind: ToolRequestKind,
}

impl ToolRequest {
    pub fn shell(cwd: impl Into<PathBuf>, command: impl Into<String>) -> Self {
        Self {
            id: None,
            cwd: cwd.into(),
            kind: ToolRequestKind::Shell {
                command: command.into(),
            },
        }
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
        match request.kind {
            ToolRequestKind::Shell { command } => run_shell(request.id, request.cwd, command).await,
            ToolRequestKind::Patch { .. }
            | ToolRequestKind::Mcp { .. }
            | ToolRequestKind::Skill { .. } => Ok(ToolResponse::declined(
                request.id,
                "YunXi only owns shell execution in this runtime slice",
            )),
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

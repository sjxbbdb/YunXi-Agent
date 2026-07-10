use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::time::timeout;
use yunxi_agent_core::{AgentError, AgentResult};
use yunxi_agent_sandbox::{
    ApprovalRequirement, ExecutionPolicy, NetworkPolicy, SandboxRequirement,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExecCommand {
    pub id: Option<String>,
    pub cwd: PathBuf,
    pub command: String,
    pub argv: Vec<String>,
    pub stdin: Option<String>,
    pub env: BTreeMap<String, String>,
    pub timeout_millis: Option<u64>,
    pub policy: ExecutionPolicy,
}

impl ExecCommand {
    pub fn shell(
        cwd: impl Into<PathBuf>,
        command: impl Into<String>,
        policy: ExecutionPolicy,
    ) -> Self {
        let command = command.into();
        Self {
            id: None,
            cwd: cwd.into(),
            argv: platform_shell_argv(&command),
            command,
            stdin: None,
            env: BTreeMap::new(),
            timeout_millis: Some(DEFAULT_EXEC_COMMAND_TIMEOUT_MILLIS),
            policy,
        }
    }

    pub fn canonical_command(&self) -> String {
        canonicalize_shell_command(&self.command)
    }

    pub fn observed_shell(cwd: impl Into<PathBuf>, command: impl Into<String>) -> Self {
        let cwd = cwd.into();
        Self::shell(
            cwd.clone(),
            command,
            ExecutionPolicy {
                approval: ApprovalRequirement::PreApproved,
                sandbox: SandboxRequirement::DangerFullAccess,
                network: NetworkPolicy::Inherit,
                workspace_root: cwd,
            },
        )
    }

    pub fn with_id(mut self, id: Option<String>) -> Self {
        self.id = id;
        self
    }
}

pub const DEFAULT_EXEC_COMMAND_TIMEOUT_MILLIS: u64 = 10_000;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExecOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

impl ExecOutput {
    pub fn combined(&self) -> String {
        combine_output(&self.stdout, &self.stderr)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecOutputStream {
    Stdout,
    Stderr,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExecLifecycleEvent {
    Started {
        id: Option<String>,
        command: String,
        cwd: PathBuf,
    },
    OutputDelta {
        id: Option<String>,
        stream: ExecOutputStream,
        chunk: String,
    },
    StdinWritten {
        id: Option<String>,
        bytes: usize,
    },
    Completed {
        id: Option<String>,
        output: ExecOutput,
        duration_millis: Option<u64>,
        timed_out: bool,
    },
    Cancelled {
        id: Option<String>,
    },
    Failed {
        id: Option<String>,
        message: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExecSummary {
    pub id: Option<String>,
    pub command: String,
    pub aggregated_output: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
}

impl ExecSummary {
    pub fn from_output(command: &ExecCommand, output: ExecOutput, timed_out: bool) -> Self {
        Self {
            id: command.id.clone(),
            command: command.canonical_command(),
            aggregated_output: output.combined(),
            exit_code: output.exit_code,
            timed_out,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExecTrace {
    pub events: Vec<ExecLifecycleEvent>,
    pub summary: ExecSummary,
}

impl ExecTrace {
    pub fn from_completed_output(
        command: &ExecCommand,
        output: ExecOutput,
        duration: Option<Duration>,
        timed_out: bool,
    ) -> Self {
        let mut events = vec![ExecLifecycleEvent::Started {
            id: command.id.clone(),
            command: command.canonical_command(),
            cwd: command.cwd.clone(),
        }];
        if !output.stdout.is_empty() {
            events.push(ExecLifecycleEvent::OutputDelta {
                id: command.id.clone(),
                stream: ExecOutputStream::Stdout,
                chunk: output.stdout.clone(),
            });
        }
        if !output.stderr.is_empty() {
            events.push(ExecLifecycleEvent::OutputDelta {
                id: command.id.clone(),
                stream: ExecOutputStream::Stderr,
                chunk: output.stderr.clone(),
            });
        }
        events.push(ExecLifecycleEvent::Completed {
            id: command.id.clone(),
            output: output.clone(),
            duration_millis: duration.map(|duration| duration.as_millis() as u64),
            timed_out,
        });
        Self {
            summary: ExecSummary::from_output(command, output, timed_out),
            events,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExecManager {
    pub output_limits: OutputLimits,
}

impl Default for ExecManager {
    fn default() -> Self {
        Self {
            output_limits: OutputLimits::default(),
        }
    }
}

impl ExecManager {
    pub async fn run(&self, command: ExecCommand) -> AgentResult<ExecTrace> {
        if command.argv.is_empty() {
            return Err(AgentError::Execution {
                message: "exec command argv is empty".to_string(),
            });
        }

        let started_at = std::time::Instant::now();
        let mut events = vec![ExecLifecycleEvent::Started {
            id: command.id.clone(),
            command: command.canonical_command(),
            cwd: command.cwd.clone(),
        }];

        let mut process = Command::new(&command.argv[0]);
        process
            .args(&command.argv[1..])
            .current_dir(&command.cwd)
            .envs(&command.env)
            .stdin(if command.stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = process.spawn().map_err(|error| AgentError::Execution {
            message: format!("failed to spawn exec command {}: {error}", command.command),
        })?;

        if let Some(stdin) = command.stdin.as_ref() {
            let mut child_stdin = child.stdin.take().ok_or_else(|| AgentError::Execution {
                message: "exec stdin pipe was not available".to_string(),
            })?;
            child_stdin
                .write_all(stdin.as_bytes())
                .await
                .map_err(|error| AgentError::Execution {
                    message: format!("failed to write exec stdin: {error}"),
                })?;
            child_stdin
                .shutdown()
                .await
                .map_err(|error| AgentError::Execution {
                    message: format!("failed to close exec stdin: {error}"),
                })?;
            events.push(ExecLifecycleEvent::StdinWritten {
                id: command.id.clone(),
                bytes: stdin.len(),
            });
        }

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let stdout_task = tokio::spawn(read_pipe(stdout));
        let stderr_task = tokio::spawn(read_pipe(stderr));

        let mut timed_out = false;
        let status = if let Some(timeout_millis) = command.timeout_millis {
            match timeout(Duration::from_millis(timeout_millis), child.wait()).await {
                Ok(result) => result.map_err(|error| AgentError::Execution {
                    message: format!("failed while waiting for exec command: {error}"),
                })?,
                Err(_) => {
                    timed_out = true;
                    let _ = child.kill().await;
                    events.push(ExecLifecycleEvent::Cancelled {
                        id: command.id.clone(),
                    });
                    child.wait().await.map_err(|error| AgentError::Execution {
                        message: format!("failed while waiting after exec timeout: {error}"),
                    })?
                }
            }
        } else {
            child.wait().await.map_err(|error| AgentError::Execution {
                message: format!("failed while waiting for exec command: {error}"),
            })?
        };

        let stdout = join_pipe(stdout_task).await?;
        let stderr = join_pipe(stderr_task).await?;
        let stdout = truncate_output(&stdout, self.output_limits);
        let stderr = truncate_output(&stderr, self.output_limits);
        if !stdout.is_empty() {
            events.push(ExecLifecycleEvent::OutputDelta {
                id: command.id.clone(),
                stream: ExecOutputStream::Stdout,
                chunk: stdout.clone(),
            });
        }
        if !stderr.is_empty() {
            events.push(ExecLifecycleEvent::OutputDelta {
                id: command.id.clone(),
                stream: ExecOutputStream::Stderr,
                chunk: stderr.clone(),
            });
        }

        let output = ExecOutput {
            stdout,
            stderr,
            exit_code: status.code(),
        };
        events.push(ExecLifecycleEvent::Completed {
            id: command.id.clone(),
            output: output.clone(),
            duration_millis: Some(started_at.elapsed().as_millis() as u64),
            timed_out,
        });
        Ok(ExecTrace {
            summary: ExecSummary::from_output(&command, output, timed_out),
            events,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OutputLimits {
    pub max_bytes: usize,
}

impl Default for OutputLimits {
    fn default() -> Self {
        Self { max_bytes: 20_000 }
    }
}

pub fn canonicalize_shell_command(command: &str) -> String {
    command.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn platform_shell_argv(command: &str) -> Vec<String> {
    if cfg!(windows) {
        vec!["cmd".to_string(), "/C".to_string(), command.to_string()]
    } else {
        vec!["sh".to_string(), "-c".to_string(), command.to_string()]
    }
}

pub fn combine_output(stdout: &str, stderr: &str) -> String {
    if stdout.is_empty() {
        return stderr.to_string();
    }
    if stderr.is_empty() {
        return stdout.to_string();
    }
    if stdout.ends_with('\n') {
        format!("{stdout}{stderr}")
    } else {
        format!("{stdout}\n{stderr}")
    }
}

pub fn truncate_output(output: &str, limits: OutputLimits) -> String {
    if output.len() <= limits.max_bytes {
        return output.to_string();
    }
    let mut truncated = output
        .chars()
        .scan(0usize, |count, ch| {
            let next = *count + ch.len_utf8();
            if next > limits.max_bytes {
                None
            } else {
                *count = next;
                Some(ch)
            }
        })
        .collect::<String>();
    truncated.push_str("\n[output truncated]");
    truncated
}

async fn read_pipe<T>(pipe: Option<T>) -> Result<String, std::io::Error>
where
    T: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    let mut output = String::new();
    if let Some(mut pipe) = pipe {
        pipe.read_to_string(&mut output).await?;
    }
    Ok(output)
}

async fn join_pipe(
    task: tokio::task::JoinHandle<Result<String, std::io::Error>>,
) -> AgentResult<String> {
    task.await
        .map_err(|error| AgentError::Execution {
            message: format!("failed to join exec output task: {error}"),
        })?
        .map_err(|error| AgentError::Execution {
            message: format!("failed to read exec output: {error}"),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    fn stdin_echo_command() -> &'static str {
        "findstr ."
    }

    #[cfg(not(windows))]
    fn stdin_echo_command() -> &'static str {
        "cat"
    }

    #[cfg(windows)]
    fn sleep_command() -> &'static str {
        "timeout /t 2 /nobreak >nul"
    }

    #[cfg(not(windows))]
    fn sleep_command() -> &'static str {
        "sleep 2"
    }

    #[test]
    fn canonicalizes_shell_spacing() {
        assert_eq!(canonicalize_shell_command(" echo   yunxi  "), "echo yunxi");
    }

    #[test]
    fn shell_command_carries_argv_timeout_and_policy() {
        let policy = ExecutionPolicy {
            approval: yunxi_agent_sandbox::ApprovalRequirement::PreApproved,
            sandbox: yunxi_agent_sandbox::SandboxRequirement::DangerFullAccess,
            network: yunxi_agent_sandbox::NetworkPolicy::Inherit,
            workspace_root: PathBuf::from("."),
        };
        let command = ExecCommand::shell(".", "echo yunxi", policy);

        assert_eq!(command.canonical_command(), "echo yunxi");
        assert_eq!(
            command.timeout_millis,
            Some(DEFAULT_EXEC_COMMAND_TIMEOUT_MILLIS)
        );
        assert!(command.argv.iter().any(|part| part.contains("echo yunxi")));
    }

    #[test]
    fn exec_summary_aggregates_output() {
        let policy = ExecutionPolicy {
            approval: yunxi_agent_sandbox::ApprovalRequirement::PreApproved,
            sandbox: yunxi_agent_sandbox::SandboxRequirement::DangerFullAccess,
            network: yunxi_agent_sandbox::NetworkPolicy::Inherit,
            workspace_root: PathBuf::from("."),
        };
        let command = ExecCommand::shell(".", "echo yunxi", policy);
        let summary = ExecSummary::from_output(
            &command,
            ExecOutput {
                stdout: "out".to_string(),
                stderr: "err".to_string(),
                exit_code: Some(0),
            },
            false,
        );

        assert_eq!(summary.aggregated_output, "out\nerr");
        assert_eq!(summary.exit_code, Some(0));
    }

    #[test]
    fn exec_trace_records_started_output_and_completed_events() {
        let command =
            ExecCommand::observed_shell(".", "echo yunxi").with_id(Some("exec-1".to_string()));
        let trace = ExecTrace::from_completed_output(
            &command,
            ExecOutput {
                stdout: "out".to_string(),
                stderr: "err".to_string(),
                exit_code: Some(0),
            },
            Some(Duration::from_millis(12)),
            false,
        );

        assert!(matches!(
            trace.events.first(),
            Some(ExecLifecycleEvent::Started { id: Some(id), .. }) if id == "exec-1"
        ));
        assert!(trace.events.iter().any(|event| matches!(
            event,
            ExecLifecycleEvent::OutputDelta {
                stream: ExecOutputStream::Stdout,
                chunk,
                ..
            } if chunk == "out"
        )));
        assert!(trace.events.iter().any(|event| matches!(
            event,
            ExecLifecycleEvent::Completed {
                duration_millis: Some(12),
                timed_out: false,
                ..
            }
        )));
        assert_eq!(trace.summary.aggregated_output, "out\nerr");
    }

    #[tokio::test]
    async fn exec_manager_writes_stdin_and_captures_output() {
        let policy = ExecutionPolicy {
            approval: yunxi_agent_sandbox::ApprovalRequirement::PreApproved,
            sandbox: yunxi_agent_sandbox::SandboxRequirement::DangerFullAccess,
            network: yunxi_agent_sandbox::NetworkPolicy::Inherit,
            workspace_root: PathBuf::from("."),
        };
        let mut command = ExecCommand::shell(".", stdin_echo_command(), policy)
            .with_id(Some("exec-stdin".to_string()));
        command.stdin = Some("yunxi\n".to_string());

        let trace = ExecManager::default()
            .run(command)
            .await
            .expect("exec trace");

        assert_eq!(trace.summary.exit_code, Some(0));
        assert!(!trace.summary.timed_out);
        assert!(trace.summary.aggregated_output.contains("yunxi"));
        assert!(trace.events.iter().any(|event| matches!(
            event,
            ExecLifecycleEvent::StdinWritten {
                id: Some(id),
                bytes
            } if id == "exec-stdin" && *bytes == "yunxi\n".len()
        )));
    }

    #[tokio::test]
    async fn exec_manager_times_out_and_records_cancelled_trace() {
        let policy = ExecutionPolicy {
            approval: yunxi_agent_sandbox::ApprovalRequirement::PreApproved,
            sandbox: yunxi_agent_sandbox::SandboxRequirement::DangerFullAccess,
            network: yunxi_agent_sandbox::NetworkPolicy::Inherit,
            workspace_root: PathBuf::from("."),
        };
        let mut command = ExecCommand::shell(".", sleep_command(), policy)
            .with_id(Some("exec-timeout".to_string()));
        command.timeout_millis = Some(1);

        let trace = ExecManager::default()
            .run(command)
            .await
            .expect("exec trace");

        assert!(trace.summary.timed_out);
        assert!(trace.events.iter().any(|event| matches!(
            event,
            ExecLifecycleEvent::Cancelled { id: Some(id) } if id == "exec-timeout"
        )));
        assert!(trace.events.iter().any(|event| matches!(
            event,
            ExecLifecycleEvent::Completed {
                timed_out: true,
                ..
            }
        )));
    }
}

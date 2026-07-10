use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use yunxi_agent_sandbox::ExecutionPolicy;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use yunxi_agent_sandbox::ExecutionPolicy;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExecCommand {
    pub id: Option<String>,
    pub cwd: PathBuf,
    pub command: String,
    pub policy: ExecutionPolicy,
}

impl ExecCommand {
    pub fn canonical_command(&self) -> String {
        canonicalize_shell_command(&self.command)
    }
}

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
}

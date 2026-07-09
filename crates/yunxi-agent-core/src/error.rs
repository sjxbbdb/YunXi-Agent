pub type AgentResult<T> = Result<T, AgentError>;

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("agent prompt cannot be empty")]
    EmptyPrompt,

    #[error("working directory does not exist: {path}")]
    MissingWorkingDirectory { path: String },

    #[error("codex source checkout is missing: {path}")]
    MissingCodexSource { path: String },

    #[error("malformed upstream event: {message}")]
    MalformedUpstreamEvent { message: String },

    #[error("agent execution failed: {message}")]
    Execution { message: String },
}

use std::path::{Path, PathBuf};

use yunxi_agent_core::CodexSource;
use yunxi_agent_core::{AgentError, AgentResult};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexRuntimeSource {
    root: PathBuf,
}

impl CodexRuntimeSource {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn from_codex_source(source: &CodexSource) -> Self {
        Self::new(source.root().join("codex-rs"))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn verify(&self) -> AgentResult<CodexRuntimeStatus> {
        if !self.root.is_dir() {
            return Err(AgentError::MissingCodexSource {
                path: self.root.display().to_string(),
            });
        }

        let status = CodexRuntimeStatus {
            root: self.root.clone(),
            workspace_manifest: self.root.join("Cargo.toml"),
            exec_manifest: self.root.join("exec/Cargo.toml"),
            exec_lib: self.root.join("exec/src/lib.rs"),
            core_manifest: self.root.join("core/Cargo.toml"),
            protocol_manifest: self.root.join("protocol/Cargo.toml"),
            config_manifest: self.root.join("config/Cargo.toml"),
            login_manifest: self.root.join("login/Cargo.toml"),
            app_server_client_manifest: self.root.join("app-server-client/Cargo.toml"),
            app_server_protocol_manifest: self.root.join("app-server-protocol/Cargo.toml"),
        };

        for path in status.required_files() {
            if !path.is_file() {
                return Err(AgentError::MissingCodexSource {
                    path: path.display().to_string(),
                });
            }
        }

        Ok(status)
    }
}

impl From<&CodexSource> for CodexRuntimeSource {
    fn from(source: &CodexSource) -> Self {
        Self::from_codex_source(source)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexRuntimeStatus {
    pub root: PathBuf,
    pub workspace_manifest: PathBuf,
    pub exec_manifest: PathBuf,
    pub exec_lib: PathBuf,
    pub core_manifest: PathBuf,
    pub protocol_manifest: PathBuf,
    pub config_manifest: PathBuf,
    pub login_manifest: PathBuf,
    pub app_server_client_manifest: PathBuf,
    pub app_server_protocol_manifest: PathBuf,
}

impl CodexRuntimeStatus {
    fn required_files(&self) -> [&Path; 9] {
        [
            self.workspace_manifest.as_path(),
            self.exec_manifest.as_path(),
            self.exec_lib.as_path(),
            self.core_manifest.as_path(),
            self.protocol_manifest.as_path(),
            self.config_manifest.as_path(),
            self.login_manifest.as_path(),
            self.app_server_client_manifest.as_path(),
            self.app_server_protocol_manifest.as_path(),
        ]
    }
}

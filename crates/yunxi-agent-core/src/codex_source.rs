use crate::{AgentError, AgentResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CodexSource {
    root: PathBuf,
}

impl CodexSource {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn verify(&self) -> AgentResult<CodexSourceStatus> {
        let codex_rs_manifest = self.root.join("codex-rs/Cargo.toml");
        let exec_lib = self.root.join("codex-rs/exec/src/lib.rs");
        let app_server_client_manifest = self.root.join("codex-rs/app-server-client/Cargo.toml");

        for path in [
            self.root.as_path(),
            codex_rs_manifest.as_path(),
            exec_lib.as_path(),
            app_server_client_manifest.as_path(),
        ] {
            if !path.exists() {
                return Err(AgentError::MissingCodexSource {
                    path: path.display().to_string(),
                });
            }
        }

        Ok(CodexSourceStatus {
            root: self.root.clone(),
            codex_rs_manifest,
            exec_lib,
            app_server_client_manifest,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CodexSourceStatus {
    pub root: PathBuf,
    pub codex_rs_manifest: PathBuf,
    pub exec_lib: PathBuf,
    pub app_server_client_manifest: PathBuf,
}

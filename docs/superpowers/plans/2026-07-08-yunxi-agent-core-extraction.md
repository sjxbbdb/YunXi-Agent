# YunXi Agent Core Extraction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an independent Rust workspace in `D:\YunXi Agent` with a reusable `yunxi-agent-core` library and a runnable `yunxi-agent-cli` that can grow into the extracted Codex Agent core.

**Architecture:** Start with a clean facade and CLI that compile independently, then add a Codex source adapter boundary around the existing Codex non-interactive execution path. Keep the initial public API small and stable while documenting any compile or runtime blocker discovered during extraction.

**Tech Stack:** Rust 2024, Cargo workspace, `clap` for CLI parsing, `tokio` for async runtime, `serde` for event/config types, `anyhow`/`thiserror` for errors, Git, CodeGraph.

## Global Constraints

- Target project root is exactly `D:\YunXi Agent`.
- Source repository is the local Codex CLI checkout previously identified by the user; verify the exact path exists before depending on it.
- Preserve first-stage scope "B": core task execution plus permissions and sandboxing.
- Do not bring in TUI, desktop app, cloud tasks, SDK packaging, update, doctor, completion, marketplace, or full app-server product surfaces in the first implementation.
- Keep MCP, skills, and history restoration as future-compatible boundaries, not first-stage required behavior.
- Initialize and maintain `D:\YunXi Agent` as an independent Git repository.
- Create a `.codegraph/` index at `D:\YunXi Agent` after the source scaffold is stable, unless CodeGraph tooling is unavailable.
- Treat `.codegraph/` as local developer metadata unless the user later asks to track it.
- Network/model integration tests are not required for the first implementation because they depend on credentials and network access.

---

## File Structure

- Create `Cargo.toml`: workspace manifest for `crates/yunxi-agent-core` and `crates/yunxi-agent-cli`.
- Create `.gitignore`: excludes Rust build output, editor files, logs, and local CodeGraph metadata.
- Create `AGENTS.md`: project-local development instructions, including CodeGraph-first navigation once `.codegraph/` exists.
- Create `README.md`: explains goals, project layout, build commands, and GitHub remote setup.
- Create `crates/yunxi-agent-core/Cargo.toml`: library crate manifest.
- Create `crates/yunxi-agent-core/src/lib.rs`: facade exports.
- Create `crates/yunxi-agent-core/src/config.rs`: `AgentConfig`, `ApprovalMode`, `SandboxMode`.
- Create `crates/yunxi-agent-core/src/input.rs`: `AgentInput`.
- Create `crates/yunxi-agent-core/src/event.rs`: `AgentEvent`, `AgentRunResult`, `AgentRunStatus`.
- Create `crates/yunxi-agent-core/src/error.rs`: `AgentError` and `AgentResult`.
- Create `crates/yunxi-agent-core/src/runner.rs`: `Agent` facade and first dry-run execution path.
- Create `crates/yunxi-agent-core/src/codex_source.rs`: verified source checkout metadata and future adapter boundary.
- Create `crates/yunxi-agent-core/tests/config_tests.rs`: config behavior tests.
- Create `crates/yunxi-agent-core/tests/runner_tests.rs`: dry-run runner behavior tests.
- Create `crates/yunxi-agent-cli/Cargo.toml`: binary crate manifest.
- Create `crates/yunxi-agent-cli/src/main.rs`: CLI argument parsing and rendering.
- Create `crates/yunxi-agent-cli/tests/cli_tests.rs`: command-line smoke tests.
- Create `docs/extraction-status.md`: documents what is already extracted, what remains, and any Codex coupling found.

---

### Task 1: Project Scaffold And Local Rules

**Files:**
- Create: `D:\YunXi Agent\Cargo.toml`
- Create: `D:\YunXi Agent\.gitignore`
- Create: `D:\YunXi Agent\AGENTS.md`
- Create: `D:\YunXi Agent\README.md`

**Interfaces:**
- Consumes: Existing design spec at `docs/superpowers/specs/2026-07-08-yunxi-agent-core-extraction-design.md`.
- Produces: A Rust workspace with member paths `crates/yunxi-agent-core` and `crates/yunxi-agent-cli`.

- [ ] **Step 1: Create root workspace manifest**

Write `D:\YunXi Agent\Cargo.toml`:

```toml
[workspace]
members = [
    "crates/yunxi-agent-core",
    "crates/yunxi-agent-cli",
]
resolver = "2"

[workspace.package]
edition = "2024"
license = "Apache-2.0"
version = "0.1.0"

[workspace.dependencies]
anyhow = "1"
assert_cmd = "2"
clap = { version = "4", features = ["derive"] }
predicates = "3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tempfile = "3"
thiserror = "2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

- [ ] **Step 2: Create repository ignore rules**

Write `D:\YunXi Agent\.gitignore`:

```gitignore
/target/
/.codegraph/
/.idea/
/.vscode/
*.log
*.tmp
*.bak
*.swp
*.swo
.DS_Store
Thumbs.db
```

- [ ] **Step 3: Create project-local agent instructions**

Write `D:\YunXi Agent\AGENTS.md`:

```markdown
# YunXi Agent Development Instructions

## CodeGraph

If `.codegraph/` exists at the repository root, use CodeGraph before grep/find when locating or understanding code.

## Scope

This repository contains an extracted, runnable Agent CLI and reusable core library based on the Codex CLI source checkout.

Keep first-stage work focused on:

- `yunxi-agent-core`
- `yunxi-agent-cli`
- Documentation needed to build, test, and push this project

Do not add TUI, desktop app, cloud tasks, SDK packaging, update, doctor, completion, marketplace, or full app-server product surfaces unless a later design explicitly includes them.

## Rust

- Use Rust 2024.
- Keep modules small and focused.
- Prefer explicit public facade types over leaking upstream Codex internals.
- Run `cargo fmt` after Rust edits.
- Run `cargo test` for this repository before claiming code is complete.
```

- [ ] **Step 4: Create initial README**

Write `D:\YunXi Agent\README.md`:

```markdown
# YunXi Agent

YunXi Agent is an extracted, runnable Rust Agent CLI and reusable core library based on the Codex CLI source checkout.

## Layout

- `crates/yunxi-agent-core`: reusable Agent facade and extraction boundary
- `crates/yunxi-agent-cli`: minimal CLI over the core library
- `docs/extraction-status.md`: current extraction status and known gaps
- `docs/superpowers/specs`: design specs
- `docs/superpowers/plans`: implementation plans

## Build

```powershell
cargo test
```

## Run

```powershell
cargo run -p yunxi-agent-cli -- "explain this project"
cargo run -p yunxi-agent-cli -- --cwd "D:\some\repo" "fix the failing test"
```

The first implementation starts with a dry-run facade and then connects the facade to Codex's non-interactive agent path.

## GitHub

After local work is ready, add your GitHub remote:

```powershell
git remote add origin <your-repository-url>
git push -u origin master
```
```

- [ ] **Step 5: Run a manifest parse check**

Run:

```powershell
cargo metadata --no-deps
```

Expected: this may fail because member crates do not exist yet. Acceptable failure contains a message that `crates/yunxi-agent-core` or `crates/yunxi-agent-cli` is missing.

- [ ] **Step 6: Commit scaffold docs**

Run:

```powershell
git add Cargo.toml .gitignore AGENTS.md README.md
git commit -m "chore: add YunXi Agent workspace scaffold"
```

Expected: commit succeeds.

---

### Task 2: Core Library Facade Types

**Files:**
- Create: `D:\YunXi Agent\crates\yunxi-agent-core\Cargo.toml`
- Create: `D:\YunXi Agent\crates\yunxi-agent-core\src\lib.rs`
- Create: `D:\YunXi Agent\crates\yunxi-agent-core\src\config.rs`
- Create: `D:\YunXi Agent\crates\yunxi-agent-core\src\input.rs`
- Create: `D:\YunXi Agent\crates\yunxi-agent-core\src\event.rs`
- Create: `D:\YunXi Agent\crates\yunxi-agent-core\src\error.rs`
- Create: `D:\YunXi Agent\crates\yunxi-agent-core\tests\config_tests.rs`

**Interfaces:**
- Consumes: Workspace dependencies from root `Cargo.toml`.
- Produces:
  - `AgentConfig::new(cwd: impl Into<PathBuf>) -> AgentConfig`
  - `ApprovalMode::{Never, OnRequest, OnFailure, Untrusted}`
  - `SandboxMode::{ReadOnly, WorkspaceWrite, DangerFullAccess}`
  - `AgentInput::text(prompt: impl Into<String>) -> AgentInput`
  - `AgentEvent`, `AgentRunResult`, `AgentRunStatus`
  - `AgentError`, `AgentResult<T>`

- [ ] **Step 1: Write failing config tests**

Write `D:\YunXi Agent\crates\yunxi-agent-core\tests\config_tests.rs`:

```rust
use std::path::PathBuf;
use yunxi_agent_core::{AgentConfig, ApprovalMode, SandboxMode};

#[test]
fn default_config_uses_workspace_write_and_on_request() {
    let config = AgentConfig::new(PathBuf::from("D:/work/project"));

    assert_eq!(config.cwd, PathBuf::from("D:/work/project"));
    assert_eq!(config.approval_mode, ApprovalMode::OnRequest);
    assert_eq!(config.sandbox_mode, SandboxMode::WorkspaceWrite);
    assert_eq!(config.model, None);
    assert_eq!(config.provider, None);
    assert_eq!(config.codex_home, None);
}

#[test]
fn builder_methods_set_optional_values() {
    let config = AgentConfig::new(PathBuf::from("D:/work/project"))
        .with_model("gpt-5")
        .with_provider("openai")
        .with_codex_home(PathBuf::from("D:/codex-home"))
        .with_approval_mode(ApprovalMode::Never)
        .with_sandbox_mode(SandboxMode::ReadOnly);

    assert_eq!(config.model.as_deref(), Some("gpt-5"));
    assert_eq!(config.provider.as_deref(), Some("openai"));
    assert_eq!(config.codex_home, Some(PathBuf::from("D:/codex-home")));
    assert_eq!(config.approval_mode, ApprovalMode::Never);
    assert_eq!(config.sandbox_mode, SandboxMode::ReadOnly);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```powershell
cargo test -p yunxi-agent-core config_tests
```

Expected: FAIL because `yunxi-agent-core` crate does not exist.

- [ ] **Step 3: Create core crate manifest**

Write `D:\YunXi Agent\crates\yunxi-agent-core\Cargo.toml`:

```toml
[package]
name = "yunxi-agent-core"
edition.workspace = true
license.workspace = true
version.workspace = true

[dependencies]
serde.workspace = true
thiserror.workspace = true
tokio.workspace = true
```

- [ ] **Step 4: Create facade exports**

Write `D:\YunXi Agent\crates\yunxi-agent-core\src\lib.rs`:

```rust
mod config;
mod error;
mod event;
mod input;

pub use config::{AgentConfig, ApprovalMode, SandboxMode};
pub use error::{AgentError, AgentResult};
pub use event::{AgentEvent, AgentRunResult, AgentRunStatus};
pub use input::AgentInput;
```

- [ ] **Step 5: Create config types**

Write `D:\YunXi Agent\crates\yunxi-agent-core\src\config.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentConfig {
    pub cwd: PathBuf,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub codex_home: Option<PathBuf>,
    pub approval_mode: ApprovalMode,
    pub sandbox_mode: SandboxMode,
}

impl AgentConfig {
    pub fn new(cwd: impl Into<PathBuf>) -> Self {
        Self {
            cwd: cwd.into(),
            model: None,
            provider: None,
            codex_home: None,
            approval_mode: ApprovalMode::OnRequest,
            sandbox_mode: SandboxMode::WorkspaceWrite,
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    pub fn with_codex_home(mut self, codex_home: impl Into<PathBuf>) -> Self {
        self.codex_home = Some(codex_home.into());
        self
    }

    pub fn with_approval_mode(mut self, approval_mode: ApprovalMode) -> Self {
        self.approval_mode = approval_mode;
        self
    }

    pub fn with_sandbox_mode(mut self, sandbox_mode: SandboxMode) -> Self {
        self.sandbox_mode = sandbox_mode;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApprovalMode {
    Never,
    OnRequest,
    OnFailure,
    Untrusted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SandboxMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}
```

- [ ] **Step 6: Create input type**

Write `D:\YunXi Agent\crates\yunxi-agent-core\src\input.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentInput {
    pub prompt: String,
}

impl AgentInput {
    pub fn text(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
        }
    }
}
```

- [ ] **Step 7: Create event and result types**

Write `D:\YunXi Agent\crates\yunxi-agent-core\src\event.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AgentEvent {
    Started { prompt: String },
    Message { content: String },
    CommandStarted { command: String },
    CommandFinished { command: String, exit_code: i32 },
    FileChanged { path: String },
    Error { message: String },
    Completed { status: AgentRunStatus },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentRunStatus {
    Completed,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentRunResult {
    pub status: AgentRunStatus,
    pub final_response: Option<String>,
    pub events: Vec<AgentEvent>,
}
```

- [ ] **Step 8: Create error type**

Write `D:\YunXi Agent\crates\yunxi-agent-core\src\error.rs`:

```rust
pub type AgentResult<T> = Result<T, AgentError>;

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("agent prompt cannot be empty")]
    EmptyPrompt,

    #[error("working directory does not exist: {path}")]
    MissingWorkingDirectory { path: String },

    #[error("codex source checkout is missing: {path}")]
    MissingCodexSource { path: String },

    #[error("agent execution failed: {message}")]
    Execution { message: String },
}
```

- [ ] **Step 9: Run tests and format**

Run:

```powershell
cargo fmt
cargo test -p yunxi-agent-core config_tests
```

Expected: PASS.

- [ ] **Step 10: Commit core facade types**

Run:

```powershell
git add crates/yunxi-agent-core
git commit -m "feat: add agent core facade types"
```

Expected: commit succeeds.

---

### Task 3: Dry-Run Agent Runner

**Files:**
- Modify: `D:\YunXi Agent\crates\yunxi-agent-core\src\lib.rs`
- Create: `D:\YunXi Agent\crates\yunxi-agent-core\src\runner.rs`
- Create: `D:\YunXi Agent\crates\yunxi-agent-core\tests\runner_tests.rs`

**Interfaces:**
- Consumes:
  - `AgentConfig`
  - `AgentInput`
  - `AgentEvent`
  - `AgentRunResult`
  - `AgentRunStatus`
  - `AgentError`
- Produces:
  - `Agent::new(config: AgentConfig) -> Agent`
  - `Agent::run_dry(input: AgentInput) -> impl Future<Output = AgentResult<AgentRunResult>> + Send`

- [ ] **Step 1: Write failing runner tests**

Write `D:\YunXi Agent\crates\yunxi-agent-core\tests\runner_tests.rs`:

```rust
use std::path::PathBuf;
use yunxi_agent_core::{Agent, AgentConfig, AgentError, AgentEvent, AgentInput, AgentRunStatus};

#[tokio::test]
async fn dry_run_returns_started_message_and_completed_events() {
    let agent = Agent::new(AgentConfig::new(PathBuf::from(".")));

    let result = agent
        .run_dry(AgentInput::text("explain this project"))
        .await
        .expect("dry run should succeed");

    assert_eq!(result.status, AgentRunStatus::Completed);
    assert_eq!(
        result.final_response.as_deref(),
        Some("Dry run accepted prompt: explain this project")
    );
    assert_eq!(
        result.events,
        vec![
            AgentEvent::Started {
                prompt: "explain this project".to_string()
            },
            AgentEvent::Message {
                content: "Dry run accepted prompt: explain this project".to_string()
            },
            AgentEvent::Completed {
                status: AgentRunStatus::Completed
            }
        ]
    );
}

#[tokio::test]
async fn dry_run_rejects_empty_prompt() {
    let agent = Agent::new(AgentConfig::new(PathBuf::from(".")));

    let error = agent
        .run_dry(AgentInput::text("   "))
        .await
        .expect_err("empty prompt should fail");

    assert!(matches!(error, AgentError::EmptyPrompt));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```powershell
cargo test -p yunxi-agent-core runner_tests
```

Expected: FAIL because `Agent` is not exported.

- [ ] **Step 3: Implement runner**

Write `D:\YunXi Agent\crates\yunxi-agent-core\src\runner.rs`:

```rust
use crate::{
    AgentConfig, AgentError, AgentEvent, AgentInput, AgentResult, AgentRunResult, AgentRunStatus,
};

#[derive(Clone, Debug)]
pub struct Agent {
    config: AgentConfig,
}

impl Agent {
    pub fn new(config: AgentConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &AgentConfig {
        &self.config
    }

    pub async fn run_dry(&self, input: AgentInput) -> AgentResult<AgentRunResult> {
        let prompt = input.prompt.trim();
        if prompt.is_empty() {
            return Err(AgentError::EmptyPrompt);
        }

        let response = format!("Dry run accepted prompt: {prompt}");
        let events = vec![
            AgentEvent::Started {
                prompt: prompt.to_string(),
            },
            AgentEvent::Message {
                content: response.clone(),
            },
            AgentEvent::Completed {
                status: AgentRunStatus::Completed,
            },
        ];

        Ok(AgentRunResult {
            status: AgentRunStatus::Completed,
            final_response: Some(response),
            events,
        })
    }
}
```

- [ ] **Step 4: Export Agent**

Modify `D:\YunXi Agent\crates\yunxi-agent-core\src\lib.rs` to:

```rust
mod config;
mod error;
mod event;
mod input;
mod runner;

pub use config::{AgentConfig, ApprovalMode, SandboxMode};
pub use error::{AgentError, AgentResult};
pub use event::{AgentEvent, AgentRunResult, AgentRunStatus};
pub use input::AgentInput;
pub use runner::Agent;
```

- [ ] **Step 5: Run tests and format**

Run:

```powershell
cargo fmt
cargo test -p yunxi-agent-core runner_tests
cargo test -p yunxi-agent-core config_tests
```

Expected: PASS.

- [ ] **Step 6: Commit dry-run runner**

Run:

```powershell
git add crates/yunxi-agent-core
git commit -m "feat: add dry-run agent runner"
```

Expected: commit succeeds.

---

### Task 4: Minimal CLI Over The Core

**Files:**
- Create: `D:\YunXi Agent\crates\yunxi-agent-cli\Cargo.toml`
- Create: `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- Create: `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`

**Interfaces:**
- Consumes:
  - `Agent::new(config)`
  - `Agent::run_dry(input)`
  - `AgentConfig::new(cwd)`
  - `AgentInput::text(prompt)`
- Produces:
  - CLI arguments: `--cwd <PATH>`, `--model <MODEL>`, `--provider <PROVIDER>`, `--json`, `PROMPT...`

- [ ] **Step 1: Write failing CLI tests**

Write `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn cli_prints_dry_run_response() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");

    cmd.arg("explain this project")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Dry run accepted prompt: explain this project",
        ));
}

#[test]
fn cli_rejects_missing_prompt() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("a prompt is required"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```powershell
cargo test -p yunxi-agent-cli cli_tests
```

Expected: FAIL because `yunxi-agent-cli` crate does not exist.

- [ ] **Step 3: Create CLI crate manifest**

Write `D:\YunXi Agent\crates\yunxi-agent-cli\Cargo.toml`:

```toml
[package]
name = "yunxi-agent-cli"
edition.workspace = true
license.workspace = true
version.workspace = true

[[bin]]
name = "yunxi-agent-cli"
path = "src/main.rs"

[dependencies]
anyhow.workspace = true
clap.workspace = true
serde_json.workspace = true
tokio.workspace = true
yunxi-agent-core = { path = "../yunxi-agent-core" }

[dev-dependencies]
assert_cmd.workspace = true
predicates.workspace = true
```

- [ ] **Step 4: Implement CLI**

Write `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`:

```rust
use anyhow::{Context, Result, bail};
use clap::Parser;
use std::path::PathBuf;
use yunxi_agent_core::{Agent, AgentConfig, AgentInput};

#[derive(Debug, Parser)]
#[command(name = "yunxi-agent-cli")]
#[command(about = "Run the extracted YunXi Agent core")]
struct Cli {
    #[arg(long, value_name = "PATH", default_value = ".")]
    cwd: PathBuf,

    #[arg(long, value_name = "MODEL")]
    model: Option<String>,

    #[arg(long, value_name = "PROVIDER")]
    provider: Option<String>,

    #[arg(long)]
    json: bool,

    #[arg(value_name = "PROMPT")]
    prompt: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let prompt = cli.prompt.join(" ");
    if prompt.trim().is_empty() {
        bail!("a prompt is required");
    }

    let mut config = AgentConfig::new(cli.cwd);
    if let Some(model) = cli.model {
        config = config.with_model(model);
    }
    if let Some(provider) = cli.provider {
        config = config.with_provider(provider);
    }

    let agent = Agent::new(config);
    let result = agent
        .run_dry(AgentInput::text(prompt))
        .await
        .context("agent run failed")?;

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else if let Some(final_response) = result.final_response {
        println!("{final_response}");
    }

    Ok(())
}
```

- [ ] **Step 5: Run CLI tests**

Run:

```powershell
cargo fmt
cargo test -p yunxi-agent-cli cli_tests
```

Expected: PASS.

- [ ] **Step 6: Run manual CLI smoke commands**

Run:

```powershell
cargo run -p yunxi-agent-cli -- "explain this project"
cargo run -p yunxi-agent-cli -- --json "explain this project"
```

Expected:

- First command prints `Dry run accepted prompt: explain this project`.
- Second command prints JSON containing `"status": "completed"` and `"final_response"`.

- [ ] **Step 7: Commit CLI**

Run:

```powershell
git add crates/yunxi-agent-cli
git commit -m "feat: add minimal YunXi Agent CLI"
```

Expected: commit succeeds.

---

### Task 5: Codex Source Verification Boundary

**Files:**
- Create: `D:\YunXi Agent\crates\yunxi-agent-core\src\codex_source.rs`
- Modify: `D:\YunXi Agent\crates\yunxi-agent-core\src\lib.rs`
- Create: `D:\YunXi Agent\crates\yunxi-agent-core\tests\codex_source_tests.rs`
- Create: `D:\YunXi Agent\docs\extraction-status.md`

**Interfaces:**
- Consumes: Local source path `D:\源码\codex`.
- Produces:
  - `CodexSource::new(root: impl Into<PathBuf>) -> CodexSource`
  - `CodexSource::verify(&self) -> AgentResult<CodexSourceStatus>`
  - `CodexSourceStatus { root: PathBuf, codex_rs_manifest: PathBuf, exec_lib: PathBuf, app_server_client_manifest: PathBuf }`

- [ ] **Step 1: Write failing source verification tests**

Write `D:\YunXi Agent\crates\yunxi-agent-core\tests\codex_source_tests.rs`:

```rust
use std::path::PathBuf;
use tempfile::TempDir;
use yunxi_agent_core::{AgentError, CodexSource};

#[test]
fn verify_rejects_missing_source_root() {
    let source = CodexSource::new(PathBuf::from("Z:/definitely/missing/codex"));

    let error = source.verify().expect_err("missing source should fail");

    assert!(matches!(error, AgentError::MissingCodexSource { .. }));
}

#[test]
fn verify_accepts_minimum_expected_codex_layout() {
    let temp = TempDir::new().expect("temp dir should be created");
    let root = temp.path();
    std::fs::create_dir_all(root.join("codex-rs/exec/src")).expect("exec dir should exist");
    std::fs::create_dir_all(root.join("codex-rs/app-server-client")).expect("client dir should exist");
    std::fs::write(root.join("codex-rs/Cargo.toml"), "[workspace]\n").expect("manifest");
    std::fs::write(root.join("codex-rs/exec/src/lib.rs"), "pub fn marker() {}\n")
        .expect("exec lib");
    std::fs::write(
        root.join("codex-rs/app-server-client/Cargo.toml"),
        "[package]\nname = \"codex-app-server-client\"\n",
    )
    .expect("client manifest");

    let status = CodexSource::new(root).verify().expect("layout should verify");

    assert_eq!(status.root, root);
    assert_eq!(status.codex_rs_manifest, root.join("codex-rs/Cargo.toml"));
    assert_eq!(status.exec_lib, root.join("codex-rs/exec/src/lib.rs"));
    assert_eq!(
        status.app_server_client_manifest,
        root.join("codex-rs/app-server-client/Cargo.toml")
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```powershell
cargo test -p yunxi-agent-core codex_source_tests
```

Expected: FAIL because `CodexSource` is not defined.

- [ ] **Step 3: Add tempfile dev-dependency**

Modify `D:\YunXi Agent\crates\yunxi-agent-core\Cargo.toml` to:

```toml
[package]
name = "yunxi-agent-core"
edition.workspace = true
license.workspace = true
version.workspace = true

[dependencies]
serde.workspace = true
thiserror.workspace = true
tokio.workspace = true

[dev-dependencies]
tempfile.workspace = true
```

- [ ] **Step 4: Implement source verification**

Write `D:\YunXi Agent\crates\yunxi-agent-core\src\codex_source.rs`:

```rust
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
```

- [ ] **Step 5: Export source boundary**

Modify `D:\YunXi Agent\crates\yunxi-agent-core\src\lib.rs` to:

```rust
mod codex_source;
mod config;
mod error;
mod event;
mod input;
mod runner;

pub use codex_source::{CodexSource, CodexSourceStatus};
pub use config::{AgentConfig, ApprovalMode, SandboxMode};
pub use error::{AgentError, AgentResult};
pub use event::{AgentEvent, AgentRunResult, AgentRunStatus};
pub use input::AgentInput;
pub use runner::Agent;
```

- [ ] **Step 6: Create extraction status document**

Write `D:\YunXi Agent\docs\extraction-status.md`:

```markdown
# Extraction Status

## Current Stage

The repository currently contains:

- A standalone Rust workspace
- `yunxi-agent-core` facade types
- A dry-run `Agent` runner
- A minimal `yunxi-agent-cli`
- A `CodexSource` verification boundary for the local Codex CLI checkout

## Verified Codex Source

The expected local Codex source checkout is:

```text
D:\源码\codex
```

The first verification boundary checks for:

- `codex-rs/Cargo.toml`
- `codex-rs/exec/src/lib.rs`
- `codex-rs/app-server-client/Cargo.toml`

## Not Yet Extracted

- Live model execution
- Codex non-interactive execution adapter
- Shell command safety integration
- Patch application integration
- Approval and sandbox mapping to upstream Codex types
- MCP, skills, and history restoration

## Next Extraction Step

Connect `yunxi-agent-core` to the smallest viable Codex non-interactive execution path. The likely candidates are:

- A direct `codex-core` thread/turn path
- A wrapper around the existing `codex-exec` flow
- A narrower adapter around `codex-app-server-client` if it proves cleaner
```

- [ ] **Step 7: Run tests and verify real source path**

Run:

```powershell
cargo fmt
cargo test -p yunxi-agent-core codex_source_tests
cargo test -p yunxi-agent-core
```

Expected: PASS.

Run:

```powershell
powershell -NoProfile -Command "Test-Path -LiteralPath 'D:\源码\codex\codex-rs\Cargo.toml'; Test-Path -LiteralPath 'D:\源码\codex\codex-rs\exec\src\lib.rs'; Test-Path -LiteralPath 'D:\源码\codex\codex-rs\app-server-client\Cargo.toml'"
```

Expected:

```text
True
True
True
```

- [ ] **Step 8: Commit source verification boundary**

Run:

```powershell
git add crates/yunxi-agent-core docs/extraction-status.md
git commit -m "feat: add Codex source verification boundary"
```

Expected: commit succeeds.

---

### Task 6: Workspace Verification And CodeGraph Index

**Files:**
- Modify: `D:\YunXi Agent\README.md`
- May create locally: `D:\YunXi Agent\.codegraph\`

**Interfaces:**
- Consumes: Buildable Rust workspace from Tasks 1-5.
- Produces: Verified repository status and local CodeGraph index if tooling is available.

- [ ] **Step 1: Run full formatting and tests**

Run:

```powershell
cargo fmt
cargo test
```

Expected: PASS.

- [ ] **Step 2: Update README with current capabilities**

Modify `D:\YunXi Agent\README.md` to:

```markdown
# YunXi Agent

YunXi Agent is an extracted, runnable Rust Agent CLI and reusable core library based on the Codex CLI source checkout.

## Layout

- `crates/yunxi-agent-core`: reusable Agent facade and extraction boundary
- `crates/yunxi-agent-cli`: minimal CLI over the core library
- `docs/extraction-status.md`: current extraction status and known gaps
- `docs/superpowers/specs`: design specs
- `docs/superpowers/plans`: implementation plans

## Current Capabilities

- Compiles as an independent Rust workspace
- Provides facade types for Agent configuration, input, events, results, and errors
- Runs a dry-run Agent path through `yunxi-agent-cli`
- Verifies the expected local Codex CLI source checkout shape

Live model execution and full Codex non-interactive execution are intentionally documented as the next extraction stage.

## Build

```powershell
cargo test
```

## Run

```powershell
cargo run -p yunxi-agent-cli -- "explain this project"
cargo run -p yunxi-agent-cli -- --cwd "D:\some\repo" "fix the failing test"
cargo run -p yunxi-agent-cli -- --json "explain this project"
```

## CodeGraph

If `.codegraph/` exists, use CodeGraph first when locating or understanding code in this repository.

## GitHub

After local work is ready, add your GitHub remote:

```powershell
git remote add origin <your-repository-url>
git push -u origin master
```
```

- [ ] **Step 3: Run CLI smoke commands**

Run:

```powershell
cargo run -p yunxi-agent-cli -- "explain this project"
cargo run -p yunxi-agent-cli -- --json "explain this project"
```

Expected:

- First command prints `Dry run accepted prompt: explain this project`.
- Second command prints JSON containing `"status": "completed"`.

- [ ] **Step 4: Try to create CodeGraph index**

Run:

```powershell
codegraph init
```

Expected:

- If CodeGraph is installed: `.codegraph/` appears at `D:\YunXi Agent\.codegraph`.
- If CodeGraph is unavailable: command fails with an executable-not-found style error. Record that in the final status and do not block the rest of the implementation.

- [ ] **Step 5: Verify Git status**

Run:

```powershell
git status --short
```

Expected: README may be modified, and `.codegraph/` should be ignored by `.gitignore` if it exists.

- [ ] **Step 6: Commit final documentation update**

Run:

```powershell
git add README.md docs/superpowers/plans/2026-07-08-yunxi-agent-core-extraction.md
git commit -m "docs: add extraction implementation plan and status"
```

Expected: commit succeeds.

If `docs/extraction-status.md` was not committed in Task 5 for any reason, include it in this commit:

```powershell
git add README.md docs/extraction-status.md docs/superpowers/plans/2026-07-08-yunxi-agent-core-extraction.md
git commit -m "docs: add extraction implementation plan and status"
```

---

## Plan Self-Review

- Spec coverage: The plan covers independent Git repo, Rust workspace, core library, CLI, source verification, documentation, tests, and CodeGraph indexing.
- Scope: The plan intentionally produces a buildable dry-run facade plus source boundary first; live Codex execution is documented as the next extraction stage because the spec allows a documented compile/extraction boundary.
- Placeholder scan: No task uses placeholder markers or unspecified "handle later" steps. Future work is explicitly documented in `docs/extraction-status.md`.
- Type consistency: `AgentConfig`, `AgentInput`, `AgentEvent`, `AgentRunResult`, `AgentRunStatus`, `Agent`, `CodexSource`, and `CodexSourceStatus` are consistently named across tasks.

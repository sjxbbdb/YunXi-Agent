# YunXi Headless Agent Stack Extraction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract the full Codex CLI headless Agent capability into YunXi Agent while keeping YunXi's public API independent from upstream Codex internals.

**Architecture:** Add a backend abstraction to `yunxi-agent-core`, keep the existing dry-run backend stable, and isolate all upstream Codex coupling in a new `yunxi-agent-codex` crate. Use a configurable local `external/codex-rs` source link for the first source-level integration pass, then port owned event/config/runner boundaries into YunXi incrementally.

**Tech Stack:** Rust 2024, Cargo workspace, `serde`, `serde_json`, `tokio`, `clap`, `thiserror`, optional upstream Codex Rust crates via local path dependencies, Git, PowerShell helper scripts.

## Global Constraints

- Target project root is exactly `D:\YunXi Agent`.
- The source checkout path must be configurable and must not be hardcoded in committed manifests.
- The implementation must preserve full Codex CLI headless Agent capabilities: real model calls, shell commands, patches, approvals, sandboxing, AGENTS.md loading, MCP, skills, sessions, rollouts, history restore, and structured events.
- Do not add TUI, desktop app UI, cloud tasks as a user-facing product, SDK packaging, update, doctor, completion, marketplace, install utilities, Bazel, or release packaging surfaces.
- Keep upstream Codex types behind `yunxi-agent-codex`; do not expose them from `yunxi-agent-core`.
- Normal `cargo test` must not require network credentials.
- Live/network tests must be gated by environment variables.
- Use Rust 2024.
- Run `cargo fmt` after Rust edits.
- Run `cargo test` before claiming implementation complete.
- If `.codegraph/` exists at repository root, use CodeGraph before grep/find; currently this repository has no `.codegraph/`.

---

## File Structure

- Modify `Cargo.toml`: add workspace member `crates/yunxi-agent-codex`, add `async-trait`, and add optional upstream dependency entries after source link support exists.
- Modify `.gitignore`: ignore `/external/`.
- Create `scripts/link-codex-source.ps1`: creates or refreshes a local source link at `external/codex-rs`.
- Modify `crates/yunxi-agent-core/src/lib.rs`: export backend and expanded event types.
- Create `crates/yunxi-agent-core/src/backend.rs`: backend trait and backend selection types.
- Modify `crates/yunxi-agent-core/src/runner.rs`: route dry-run through `DryRunBackend` and add generic backend runner.
- Modify `crates/yunxi-agent-core/src/event.rs`: expand event model.
- Modify `crates/yunxi-agent-core/src/config.rs`: add live options needed by Codex backend.
- Create `crates/yunxi-agent-core/tests/backend_tests.rs`: backend abstraction tests.
- Create `crates/yunxi-agent-core/tests/event_tests.rs`: event serialization tests.
- Create `crates/yunxi-agent-codex/Cargo.toml`: Codex integration crate manifest.
- Create `crates/yunxi-agent-codex/src/lib.rs`: public integration facade for internal use.
- Create `crates/yunxi-agent-codex/src/source_runtime.rs`: source link verification.
- Create `crates/yunxi-agent-codex/src/config_mapper.rs`: maps YunXi config to Codex-oriented options.
- Create `crates/yunxi-agent-codex/src/event_mapper.rs`: maps Codex JSONL/headless events to YunXi events.
- Create `crates/yunxi-agent-codex/src/native_exec.rs`: source-level live runner entry point.
- Create `crates/yunxi-agent-codex/src/process_fallback.rs`: diagnostic fallback only, behind an explicit feature.
- Create `crates/yunxi-agent-codex/tests/source_runtime_tests.rs`.
- Create `crates/yunxi-agent-codex/tests/config_mapper_tests.rs`.
- Create `crates/yunxi-agent-codex/tests/event_mapper_tests.rs`.
- Create `crates/yunxi-agent-codex/tests/live_smoke_tests.rs`: ignored/gated live test.
- Modify `crates/yunxi-agent-cli/Cargo.toml`: depend on `yunxi-agent-codex`.
- Modify `crates/yunxi-agent-cli/src/main.rs`: add backend, live, JSONL, approval, sandbox, and Codex home flags.
- Modify `crates/yunxi-agent-cli/tests/cli_tests.rs`: add dry-run compatibility and live flag parsing tests.
- Create `crates/yunxi-agent-cli/tests/jsonl_tests.rs`: JSONL rendering smoke tests.
- Modify `README.md`: document live backend setup.
- Modify `docs/extraction-status.md`: document stage-2 status and remaining ownership boundaries.

---

### Task 1: Source Link And Workspace Wiring

**Files:**
- Modify: `D:\YunXi Agent\Cargo.toml`
- Modify: `D:\YunXi Agent\.gitignore`
- Create: `D:\YunXi Agent\scripts\link-codex-source.ps1`
- Create: `D:\YunXi Agent\scripts\README.md`

**Interfaces:**
- Consumes: user-provided Codex checkout root path.
- Produces:
  - ignored local link `D:\YunXi Agent\external\codex-rs`
  - workspace member `crates/yunxi-agent-codex`
  - workspace dependency `async-trait = "0.1"`

- [ ] **Step 1: Add a failing source-link test command to the plan ledger**

Run:

```powershell
Test-Path -LiteralPath "D:\YunXi Agent\scripts\link-codex-source.ps1"
```

Expected: `False`, because the script does not exist yet.

- [ ] **Step 2: Update root workspace manifest**

Modify `D:\YunXi Agent\Cargo.toml` to include the new crate and dependency:

```toml
[workspace]
members = [
    "crates/yunxi-agent-core",
    "crates/yunxi-agent-codex",
    "crates/yunxi-agent-cli",
]
resolver = "2"

[workspace.package]
edition = "2024"
license = "Apache-2.0"
version = "0.1.0"

[workspace.dependencies]
anyhow = "1"
async-trait = "0.1"
assert_cmd = "2"
clap = { version = "4", features = ["derive"] }
predicates = "3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tempfile = "3"
thiserror = "2"
tokio = { version = "1", features = ["macros", "process", "rt-multi-thread"] }
```

- [ ] **Step 3: Ignore local upstream source links**

Modify `D:\YunXi Agent\.gitignore` to include:

```gitignore
/external/
```

Keep the existing ignored paths.

- [ ] **Step 4: Create source link helper**

Create `D:\YunXi Agent\scripts\link-codex-source.ps1`:

```powershell
param(
    [Parameter(Mandatory = $true)]
    [string] $CodexCheckoutRoot
)

$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
$sourceRoot = Resolve-Path -LiteralPath $CodexCheckoutRoot
$codexRs = Join-Path $sourceRoot 'codex-rs'

if (-not (Test-Path -LiteralPath $codexRs -PathType Container)) {
    throw "Codex checkout does not contain codex-rs: $codexRs"
}

foreach ($required in @(
    'Cargo.toml',
    'exec/src/lib.rs',
    'app-server-client/Cargo.toml',
    'app-server-protocol/Cargo.toml',
    'core/Cargo.toml',
    'protocol/Cargo.toml'
)) {
    $path = Join-Path $codexRs $required
    if (-not (Test-Path -LiteralPath $path)) {
        throw "Codex source is missing required path: $path"
    }
}

$externalDir = Join-Path $repoRoot 'external'
$target = Join-Path $externalDir 'codex-rs'

New-Item -ItemType Directory -Force -Path $externalDir | Out-Null

if (Test-Path -LiteralPath $target) {
    $item = Get-Item -LiteralPath $target -Force
    if ($item.LinkType -eq 'Junction' -or $item.LinkType -eq 'SymbolicLink') {
        Remove-Item -LiteralPath $target -Force
    } else {
        throw "Refusing to replace non-link path: $target"
    }
}

New-Item -ItemType Junction -Path $target -Target $codexRs | Out-Null
Write-Output "Linked $target -> $codexRs"
```

- [ ] **Step 5: Create script documentation**

Create `D:\YunXi Agent\scripts\README.md`:

```markdown
# Scripts

## link-codex-source.ps1

Creates the ignored local source link used by stage-2 Codex path dependencies:

```powershell
.\scripts\link-codex-source.ps1 -CodexCheckoutRoot "<path-to-codex-checkout>"
```

The script verifies that the checkout contains `codex-rs` and the core crates
needed by the YunXi live backend.
```

- [ ] **Step 6: Verify the script exists and external is ignored**

Run:

```powershell
Test-Path -LiteralPath "D:\YunXi Agent\scripts\link-codex-source.ps1"
git check-ignore external/codex-rs
```

Expected:

```text
True
external/codex-rs
```

- [ ] **Step 7: Verify baseline still builds without source link**

Run:

```powershell
cargo fmt
cargo test
```

Expected: PASS after the `yunxi-agent-codex` crate is added in Task 5. If this task is implemented before that crate exists, `cargo test` may fail because the workspace member is missing; proceed to Task 5 before committing if needed.

- [ ] **Step 8: Commit source link wiring**

Run:

```powershell
git add Cargo.toml .gitignore scripts
git commit -m "chore: add Codex source link wiring"
```

Expected: commit succeeds.

---

### Task 2: Core Backend Abstraction

**Files:**
- Create: `D:\YunXi Agent\crates\yunxi-agent-core\src\backend.rs`
- Modify: `D:\YunXi Agent\crates\yunxi-agent-core\src\lib.rs`
- Modify: `D:\YunXi Agent\crates\yunxi-agent-core\src\runner.rs`
- Modify: `D:\YunXi Agent\crates\yunxi-agent-core\Cargo.toml`
- Create: `D:\YunXi Agent\crates\yunxi-agent-core\tests\backend_tests.rs`

**Interfaces:**
- Consumes: `AgentConfig`, `AgentInput`, `AgentRunResult`, `AgentResult`.
- Produces:
  - `AgentBackend`
  - `DryRunBackend`
  - `BackendKind`
  - `Agent::run_with_backend<B>(&self, backend: &B, input: AgentInput)`

- [ ] **Step 1: Write failing backend tests**

Create `D:\YunXi Agent\crates\yunxi-agent-core\tests\backend_tests.rs`:

```rust
use std::path::PathBuf;
use yunxi_agent_core::{
    Agent, AgentBackend, AgentConfig, AgentEvent, AgentInput, AgentRunStatus, BackendKind,
    DryRunBackend,
};

#[tokio::test]
async fn agent_can_run_through_backend_trait() {
    let agent = Agent::new(AgentConfig::new(PathBuf::from(".")));
    let backend = DryRunBackend::default();

    let result = agent
        .run_with_backend(&backend, AgentInput::text("use backend"))
        .await
        .expect("backend run should succeed");

    assert_eq!(result.status, AgentRunStatus::Completed);
    assert_eq!(
        result.final_response.as_deref(),
        Some("Dry run accepted prompt: use backend")
    );
    assert_eq!(
        result.events.first(),
        Some(&AgentEvent::Started {
            prompt: "use backend".to_string()
        })
    );
}

#[test]
fn backend_kind_serializes_as_kebab_case() {
    assert_eq!(
        serde_json::to_string(&BackendKind::DryRun).expect("json"),
        "\"dry-run\""
    );
    assert_eq!(
        serde_json::to_string(&BackendKind::Codex).expect("json"),
        "\"codex\""
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```powershell
cargo test -p yunxi-agent-core backend_tests
```

Expected: FAIL because `AgentBackend`, `DryRunBackend`, and `BackendKind` do not exist.

- [ ] **Step 3: Add dependencies**

Modify `D:\YunXi Agent\crates\yunxi-agent-core\Cargo.toml`:

```toml
[package]
name = "yunxi-agent-core"
edition.workspace = true
license.workspace = true
version.workspace = true

[dependencies]
async-trait.workspace = true
serde.workspace = true
thiserror.workspace = true
tokio.workspace = true

[dev-dependencies]
serde_json.workspace = true
tempfile.workspace = true
```

- [ ] **Step 4: Create backend module**

Create `D:\YunXi Agent\crates\yunxi-agent-core\src\backend.rs`:

```rust
use crate::{AgentConfig, AgentInput, AgentResult, AgentRunResult};
use serde::{Deserialize, Serialize};

#[async_trait::async_trait]
pub trait AgentBackend: Send + Sync {
    async fn run(
        &self,
        config: AgentConfig,
        input: AgentInput,
    ) -> AgentResult<AgentRunResult>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackendKind {
    DryRun,
    Codex,
}

#[derive(Clone, Debug, Default)]
pub struct DryRunBackend;
```

- [ ] **Step 5: Move dry-run logic behind `DryRunBackend`**

Modify `D:\YunXi Agent\crates\yunxi-agent-core\src\runner.rs`:

```rust
use crate::{
    AgentBackend, AgentConfig, AgentError, AgentEvent, AgentInput, AgentResult, AgentRunResult,
    AgentRunStatus, DryRunBackend,
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
        self.run_with_backend(&DryRunBackend, input).await
    }

    pub async fn run_with_backend<B>(
        &self,
        backend: &B,
        input: AgentInput,
    ) -> AgentResult<AgentRunResult>
    where
        B: AgentBackend,
    {
        backend.run(self.config.clone(), input).await
    }
}

#[async_trait::async_trait]
impl AgentBackend for DryRunBackend {
    async fn run(
        &self,
        _config: AgentConfig,
        input: AgentInput,
    ) -> AgentResult<AgentRunResult> {
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

- [ ] **Step 6: Export backend types**

Modify `D:\YunXi Agent\crates\yunxi-agent-core\src\lib.rs`:

```rust
mod backend;
mod codex_source;
mod config;
mod error;
mod event;
mod input;
mod runner;

pub use backend::{AgentBackend, BackendKind, DryRunBackend};
pub use codex_source::{CodexSource, CodexSourceStatus};
pub use config::{AgentConfig, ApprovalMode, SandboxMode};
pub use error::{AgentError, AgentResult};
pub use event::{AgentEvent, AgentRunResult, AgentRunStatus};
pub use input::AgentInput;
pub use runner::Agent;
```

- [ ] **Step 7: Run tests**

Run:

```powershell
cargo fmt
cargo test -p yunxi-agent-core backend_tests
cargo test
```

Expected: PASS.

- [ ] **Step 8: Commit backend abstraction**

Run:

```powershell
git add crates/yunxi-agent-core
git commit -m "feat: add agent backend abstraction"
```

Expected: commit succeeds.

---

### Task 3: Expand Core Event Model

**Files:**
- Modify: `D:\YunXi Agent\crates\yunxi-agent-core\src\event.rs`
- Create: `D:\YunXi Agent\crates\yunxi-agent-core\tests\event_tests.rs`

**Interfaces:**
- Consumes: existing `AgentEvent`.
- Produces:
  - expanded `AgentEvent`
  - `CommandStatus`
  - `FileChangeKind`
  - `PatchStatus`
  - `McpToolStatus`
  - `TodoStatus`
  - `TokenUsage`

- [ ] **Step 1: Write failing event serialization tests**

Create `D:\YunXi Agent\crates\yunxi-agent-core\tests\event_tests.rs`:

```rust
use yunxi_agent_core::{
    AgentEvent, AgentRunStatus, CommandStatus, FileChangeKind, McpToolStatus, PatchStatus,
    TodoStatus, TokenUsage,
};

#[test]
fn command_event_serializes_with_stable_shape() {
    let event = AgentEvent::CommandCompleted {
        id: Some("item_1".to_string()),
        command: "cargo test".to_string(),
        aggregated_output: "ok".to_string(),
        exit_code: Some(0),
        status: CommandStatus::Completed,
    };

    let json = serde_json::to_value(event).expect("json");

    assert_eq!(json["type"], "commandCompleted");
    assert_eq!(json["id"], "item_1");
    assert_eq!(json["command"], "cargo test");
    assert_eq!(json["aggregated_output"], "ok");
    assert_eq!(json["exit_code"], 0);
    assert_eq!(json["status"], "completed");
}

#[test]
fn completion_event_can_include_usage() {
    let event = AgentEvent::Completed {
        status: AgentRunStatus::Completed,
        usage: Some(TokenUsage {
            input_tokens: 10,
            cached_input_tokens: 2,
            output_tokens: 5,
            reasoning_output_tokens: 1,
        }),
    };

    let json = serde_json::to_value(event).expect("json");

    assert_eq!(json["type"], "completed");
    assert_eq!(json["status"], "completed");
    assert_eq!(json["usage"]["input_tokens"], 10);
}

#[test]
fn mcp_patch_file_and_todo_events_have_stable_names() {
    let events = vec![
        AgentEvent::FileChanged {
            path: "src/lib.rs".to_string(),
            kind: FileChangeKind::Update,
        },
        AgentEvent::PatchCompleted {
            status: PatchStatus::Completed,
        },
        AgentEvent::McpToolCompleted {
            id: Some("item_2".to_string()),
            server: "filesystem".to_string(),
            tool: "read_file".to_string(),
            status: McpToolStatus::Completed,
        },
        AgentEvent::TodoUpdated {
            id: Some("item_3".to_string()),
            items: vec![TodoStatus {
                text: "inspect".to_string(),
                completed: true,
            }],
        },
    ];

    let names: Vec<String> = events
        .into_iter()
        .map(|event| serde_json::to_value(event).expect("json")["type"].as_str().unwrap().to_string())
        .collect();

    assert_eq!(
        names,
        vec![
            "fileChanged",
            "patchCompleted",
            "mcpToolCompleted",
            "todoUpdated"
        ]
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```powershell
cargo test -p yunxi-agent-core event_tests
```

Expected: FAIL because new event types do not exist.

- [ ] **Step 3: Replace event model**

Modify `D:\YunXi Agent\crates\yunxi-agent-core\src\event.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AgentEvent {
    Started {
        prompt: String,
    },
    ThreadStarted {
        thread_id: String,
    },
    TurnStarted,
    Message {
        content: String,
    },
    Reasoning {
        content: String,
    },
    CommandStarted {
        id: Option<String>,
        command: String,
    },
    CommandUpdated {
        id: Option<String>,
        command: String,
        aggregated_output: String,
    },
    CommandCompleted {
        id: Option<String>,
        command: String,
        aggregated_output: String,
        exit_code: Option<i32>,
        status: CommandStatus,
    },
    CommandFinished {
        command: String,
        exit_code: i32,
    },
    FileChanged {
        path: String,
        kind: FileChangeKind,
    },
    PatchCompleted {
        status: PatchStatus,
    },
    McpToolStarted {
        id: Option<String>,
        server: String,
        tool: String,
    },
    McpToolCompleted {
        id: Option<String>,
        server: String,
        tool: String,
        status: McpToolStatus,
    },
    TodoUpdated {
        id: Option<String>,
        items: Vec<TodoStatus>,
    },
    Warning {
        message: String,
    },
    Error {
        message: String,
    },
    Completed {
        status: AgentRunStatus,
        usage: Option<TokenUsage>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentRunStatus {
    Completed,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    InProgress,
    Completed,
    Failed,
    Declined,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileChangeKind {
    Add,
    Delete,
    Update,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchStatus {
    InProgress,
    Completed,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpToolStatus {
    InProgress,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TodoStatus {
    pub text: String,
    pub completed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_output_tokens: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentRunResult {
    pub status: AgentRunStatus,
    pub final_response: Option<String>,
    pub events: Vec<AgentEvent>,
}
```

- [ ] **Step 4: Update dry-run completed event**

Modify `D:\YunXi Agent\crates\yunxi-agent-core\src\runner.rs` so the dry-run completion event is:

```rust
AgentEvent::Completed {
    status: AgentRunStatus::Completed,
    usage: None,
}
```

- [ ] **Step 5: Export new event types**

Modify `D:\YunXi Agent\crates\yunxi-agent-core\src\lib.rs` event export:

```rust
pub use event::{
    AgentEvent, AgentRunResult, AgentRunStatus, CommandStatus, FileChangeKind, McpToolStatus,
    PatchStatus, TodoStatus, TokenUsage,
};
```

- [ ] **Step 6: Update existing runner tests**

Modify `D:\YunXi Agent\crates\yunxi-agent-core\tests\runner_tests.rs` expected completion event:

```rust
AgentEvent::Completed {
    status: AgentRunStatus::Completed,
    usage: None,
}
```

- [ ] **Step 7: Run tests**

Run:

```powershell
cargo fmt
cargo test -p yunxi-agent-core event_tests
cargo test
```

Expected: PASS.

- [ ] **Step 8: Commit event expansion**

Run:

```powershell
git add crates/yunxi-agent-core
git commit -m "feat: expand agent event model"
```

Expected: commit succeeds.

---

### Task 4: CLI Backend Selection And JSONL Rendering

**Files:**
- Modify: `D:\YunXi Agent\crates\yunxi-agent-cli\Cargo.toml`
- Modify: `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- Modify: `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`
- Create: `D:\YunXi Agent\crates\yunxi-agent-cli\tests\jsonl_tests.rs`

**Interfaces:**
- Consumes: `BackendKind`, `AgentEvent`, `AgentRunResult`.
- Produces CLI flags:
  - `--backend dry-run|codex`
  - `--live`
  - `--codex-home <PATH>`
  - `--approval never|on-request|on-failure|untrusted`
  - `--sandbox read-only|workspace-write|danger-full-access`
  - `--jsonl`

- [ ] **Step 1: Write failing CLI tests**

Modify `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs` to include:

```rust
#[test]
fn cli_accepts_explicit_dry_run_backend() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");

    cmd.args(["--backend", "dry-run", "explain this project"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Dry run accepted prompt: explain this project",
        ));
}

#[test]
fn cli_rejects_live_backend_until_codex_crate_is_wired() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");

    cmd.args(["--live", "explain this project"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "codex backend is not wired into the CLI yet",
        ));
}
```

Create `D:\YunXi Agent\crates\yunxi-agent-cli\tests\jsonl_tests.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn dry_run_jsonl_prints_one_json_event_per_line() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");

    let assert = cmd
        .args(["--backend", "dry-run", "--jsonl", "jsonl dry run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"type\":\"started\""))
        .stdout(predicate::str::contains("\"type\":\"message\""))
        .stdout(predicate::str::contains("\"type\":\"completed\""));

    let output = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8");
    for line in output.lines() {
        serde_json::from_str::<serde_json::Value>(line).expect("each line is json");
    }
}
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```powershell
cargo test -p yunxi-agent-cli cli_tests jsonl_tests
```

Expected: FAIL because the flags do not exist yet.

- [ ] **Step 3: Add CLI dev dependency**

Modify `D:\YunXi Agent\crates\yunxi-agent-cli\Cargo.toml`:

```toml
[dev-dependencies]
assert_cmd.workspace = true
predicates.workspace = true
serde_json.workspace = true
```

- [ ] **Step 4: Implement CLI parsing and dry-run JSONL rendering**

Modify `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`:

```rust
use anyhow::{Context, Result, bail};
use clap::{Parser, ValueEnum};
use std::path::PathBuf;
use yunxi_agent_core::{
    Agent, AgentConfig, AgentInput, ApprovalMode, BackendKind, SandboxMode,
};

#[derive(Debug, Parser)]
#[command(name = "yunxi-agent-cli")]
#[command(about = "Run the extracted YunXi Agent core")]
struct Cli {
    #[arg(long, value_name = "BACKEND", value_enum, default_value_t = CliBackend::DryRun)]
    backend: CliBackend,

    #[arg(long, conflicts_with = "backend")]
    live: bool,

    #[arg(long, value_name = "PATH", default_value = ".")]
    cwd: PathBuf,

    #[arg(long, value_name = "MODEL")]
    model: Option<String>,

    #[arg(long, value_name = "PROVIDER")]
    provider: Option<String>,

    #[arg(long = "codex-home", value_name = "PATH")]
    codex_home: Option<PathBuf>,

    #[arg(long, value_name = "MODE", value_enum, default_value_t = CliApprovalMode::OnRequest)]
    approval: CliApprovalMode,

    #[arg(long, value_name = "MODE", value_enum, default_value_t = CliSandboxMode::WorkspaceWrite)]
    sandbox: CliSandboxMode,

    #[arg(long, conflicts_with = "jsonl")]
    json: bool,

    #[arg(long)]
    jsonl: bool,

    #[arg(value_name = "PROMPT")]
    prompt: Vec<String>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "kebab-case")]
enum CliBackend {
    DryRun,
    Codex,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "kebab-case")]
enum CliApprovalMode {
    Never,
    OnRequest,
    OnFailure,
    Untrusted,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "kebab-case")]
enum CliSandboxMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

impl From<CliBackend> for BackendKind {
    fn from(value: CliBackend) -> Self {
        match value {
            CliBackend::DryRun => BackendKind::DryRun,
            CliBackend::Codex => BackendKind::Codex,
        }
    }
}

impl From<CliApprovalMode> for ApprovalMode {
    fn from(value: CliApprovalMode) -> Self {
        match value {
            CliApprovalMode::Never => ApprovalMode::Never,
            CliApprovalMode::OnRequest => ApprovalMode::OnRequest,
            CliApprovalMode::OnFailure => ApprovalMode::OnFailure,
            CliApprovalMode::Untrusted => ApprovalMode::Untrusted,
        }
    }
}

impl From<CliSandboxMode> for SandboxMode {
    fn from(value: CliSandboxMode) -> Self {
        match value {
            CliSandboxMode::ReadOnly => SandboxMode::ReadOnly,
            CliSandboxMode::WorkspaceWrite => SandboxMode::WorkspaceWrite,
            CliSandboxMode::DangerFullAccess => SandboxMode::DangerFullAccess,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let prompt = cli.prompt.join(" ");
    if prompt.trim().is_empty() {
        bail!("a prompt is required");
    }

    let backend = if cli.live {
        BackendKind::Codex
    } else {
        cli.backend.into()
    };

    let mut config = AgentConfig::new(cli.cwd)
        .with_approval_mode(cli.approval.into())
        .with_sandbox_mode(cli.sandbox.into());
    if let Some(model) = cli.model {
        config = config.with_model(model);
    }
    if let Some(provider) = cli.provider {
        config = config.with_provider(provider);
    }
    if let Some(codex_home) = cli.codex_home {
        config = config.with_codex_home(codex_home);
    }

    let result = match backend {
        BackendKind::DryRun => {
            let agent = Agent::new(config);
            agent
                .run_dry(AgentInput::text(prompt))
                .await
                .context("agent run failed")?
        }
        BackendKind::Codex => {
            bail!("codex backend is not wired into the CLI yet");
        }
    };

    if cli.jsonl {
        for event in result.events {
            println!("{}", serde_json::to_string(&event)?);
        }
    } else if cli.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else if let Some(final_response) = result.final_response {
        println!("{final_response}");
    }

    Ok(())
}
```

- [ ] **Step 5: Run tests**

Run:

```powershell
cargo fmt
cargo test -p yunxi-agent-cli
cargo test
```

Expected: PASS.

- [ ] **Step 6: Commit CLI backend flags**

Run:

```powershell
git add crates/yunxi-agent-cli
git commit -m "feat: add CLI backend selection"
```

Expected: commit succeeds.

---

### Task 5: Codex Integration Crate Scaffold

**Files:**
- Create: `D:\YunXi Agent\crates\yunxi-agent-codex\Cargo.toml`
- Create: `D:\YunXi Agent\crates\yunxi-agent-codex\src\lib.rs`
- Create: `D:\YunXi Agent\crates\yunxi-agent-codex\src\source_runtime.rs`
- Create: `D:\YunXi Agent\crates\yunxi-agent-codex\tests\source_runtime_tests.rs`

**Interfaces:**
- Consumes: `yunxi-agent-core::CodexSource`.
- Produces:
  - `CodexRuntimeSource::new(root: impl Into<PathBuf>)`
  - `CodexRuntimeSource::verify() -> AgentResult<CodexRuntimeStatus>`

- [ ] **Step 1: Write failing source runtime tests**

Create `D:\YunXi Agent\crates\yunxi-agent-codex\tests\source_runtime_tests.rs`:

```rust
use std::path::PathBuf;
use tempfile::TempDir;
use yunxi_agent_codex::CodexRuntimeSource;
use yunxi_agent_core::AgentError;

#[test]
fn runtime_source_rejects_missing_codex_rs_root() {
    let source = CodexRuntimeSource::new(PathBuf::from("Z:/missing/codex-rs"));

    let error = source.verify().expect_err("missing source should fail");

    assert!(matches!(error, AgentError::MissingCodexSource { .. }));
}

#[test]
fn runtime_source_accepts_minimum_headless_crate_layout() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    for dir in [
        "exec/src",
        "core",
        "protocol",
        "config",
        "login",
        "app-server-client",
        "app-server-protocol",
    ] {
        std::fs::create_dir_all(root.join(dir)).expect("dir");
    }
    for file in [
        "Cargo.toml",
        "exec/Cargo.toml",
        "exec/src/lib.rs",
        "core/Cargo.toml",
        "protocol/Cargo.toml",
        "config/Cargo.toml",
        "login/Cargo.toml",
        "app-server-client/Cargo.toml",
        "app-server-protocol/Cargo.toml",
    ] {
        std::fs::write(root.join(file), "[package]\nname = \"fixture\"\n").expect("file");
    }

    let status = CodexRuntimeSource::new(root)
        .verify()
        .expect("layout should verify");

    assert_eq!(status.root, root);
    assert_eq!(status.exec_manifest, root.join("exec/Cargo.toml"));
    assert_eq!(status.exec_lib, root.join("exec/src/lib.rs"));
}
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```powershell
cargo test -p yunxi-agent-codex source_runtime_tests
```

Expected: FAIL because `yunxi-agent-codex` does not exist.

- [ ] **Step 3: Create crate manifest**

Create `D:\YunXi Agent\crates\yunxi-agent-codex\Cargo.toml`:

```toml
[package]
name = "yunxi-agent-codex"
edition.workspace = true
license.workspace = true
version.workspace = true

[features]
default = []
codex-native = []
process-fallback = []

[dependencies]
anyhow.workspace = true
async-trait.workspace = true
serde.workspace = true
serde_json.workspace = true
tokio.workspace = true
yunxi-agent-core = { path = "../yunxi-agent-core" }

[dev-dependencies]
tempfile.workspace = true
```

- [ ] **Step 4: Create crate facade**

Create `D:\YunXi Agent\crates\yunxi-agent-codex\src\lib.rs`:

```rust
mod source_runtime;

pub use source_runtime::{CodexRuntimeSource, CodexRuntimeStatus};
```

- [ ] **Step 5: Implement source runtime verification**

Create `D:\YunXi Agent\crates\yunxi-agent-codex\src\source_runtime.rs`:

```rust
use std::path::{Path, PathBuf};
use yunxi_agent_core::{AgentError, AgentResult};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexRuntimeSource {
    root: PathBuf,
}

impl CodexRuntimeSource {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
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
```

- [ ] **Step 6: Run tests**

Run:

```powershell
cargo fmt
cargo test -p yunxi-agent-codex source_runtime_tests
cargo test
```

Expected: PASS.

- [ ] **Step 7: Commit integration crate scaffold**

Run:

```powershell
git add Cargo.toml crates/yunxi-agent-codex
git commit -m "feat: add Codex integration crate scaffold"
```

Expected: commit succeeds.

---

### Task 6: Codex Event Mapping From JSONL Fixtures

**Files:**
- Create: `D:\YunXi Agent\crates\yunxi-agent-codex\src\event_mapper.rs`
- Modify: `D:\YunXi Agent\crates\yunxi-agent-codex\src\lib.rs`
- Create: `D:\YunXi Agent\crates\yunxi-agent-codex\tests\event_mapper_tests.rs`

**Interfaces:**
- Consumes: JSON values shaped like upstream `codex-exec` JSONL `ThreadEvent`.
- Produces:
  - `map_exec_json_event(value: serde_json::Value) -> AgentResult<Vec<AgentEvent>>`
  - `map_exec_jsonl(input: &str) -> AgentResult<Vec<AgentEvent>>`

- [ ] **Step 1: Write failing mapping tests**

Create `D:\YunXi Agent\crates\yunxi-agent-codex\tests\event_mapper_tests.rs`:

```rust
use yunxi_agent_codex::map_exec_jsonl;
use yunxi_agent_core::{
    AgentEvent, AgentRunStatus, CommandStatus, FileChangeKind, McpToolStatus, PatchStatus,
};

#[test]
fn maps_thread_turn_message_and_completion_events() {
    let jsonl = r#"
{"type":"thread.started","thread_id":"thread_1"}
{"type":"turn.started"}
{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"hello"}}
{"type":"turn.completed","usage":{"input_tokens":3,"cached_input_tokens":1,"output_tokens":2,"reasoning_output_tokens":0}}
"#;

    let events = map_exec_jsonl(jsonl).expect("events");

    assert_eq!(
        events,
        vec![
            AgentEvent::ThreadStarted {
                thread_id: "thread_1".to_string()
            },
            AgentEvent::TurnStarted,
            AgentEvent::Message {
                content: "hello".to_string()
            },
            AgentEvent::Completed {
                status: AgentRunStatus::Completed,
                usage: Some(yunxi_agent_core::TokenUsage {
                    input_tokens: 3,
                    cached_input_tokens: 1,
                    output_tokens: 2,
                    reasoning_output_tokens: 0,
                })
            }
        ]
    );
}

#[test]
fn maps_command_file_patch_and_mcp_events() {
    let jsonl = r#"
{"type":"item.started","item":{"id":"cmd_1","type":"command_execution","command":"cargo test","aggregated_output":"","exit_code":null,"status":"in_progress"}}
{"type":"item.completed","item":{"id":"cmd_1","type":"command_execution","command":"cargo test","aggregated_output":"ok","exit_code":0,"status":"completed"}}
{"type":"item.completed","item":{"id":"patch_1","type":"file_change","changes":[{"path":"src/lib.rs","kind":"update"}],"status":"completed"}}
{"type":"item.completed","item":{"id":"mcp_1","type":"mcp_tool_call","server":"fs","tool":"read_file","arguments":{},"result":null,"error":null,"status":"completed"}}
"#;

    let events = map_exec_jsonl(jsonl).expect("events");

    assert!(events.contains(&AgentEvent::CommandStarted {
        id: Some("cmd_1".to_string()),
        command: "cargo test".to_string(),
    }));
    assert!(events.contains(&AgentEvent::CommandCompleted {
        id: Some("cmd_1".to_string()),
        command: "cargo test".to_string(),
        aggregated_output: "ok".to_string(),
        exit_code: Some(0),
        status: CommandStatus::Completed,
    }));
    assert!(events.contains(&AgentEvent::FileChanged {
        path: "src/lib.rs".to_string(),
        kind: FileChangeKind::Update,
    }));
    assert!(events.contains(&AgentEvent::PatchCompleted {
        status: PatchStatus::Completed,
    }));
    assert!(events.contains(&AgentEvent::McpToolCompleted {
        id: Some("mcp_1".to_string()),
        server: "fs".to_string(),
        tool: "read_file".to_string(),
        status: McpToolStatus::Completed,
    }));
}

#[test]
fn malformed_jsonl_returns_agent_error() {
    let error = map_exec_jsonl("{not json").expect_err("bad json should fail");

    assert!(error.to_string().contains("malformed upstream event"));
}
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```powershell
cargo test -p yunxi-agent-codex event_mapper_tests
```

Expected: FAIL because mapper functions do not exist.

- [ ] **Step 3: Add malformed upstream event error**

Modify `D:\YunXi Agent\crates\yunxi-agent-core\src\error.rs`:

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

    #[error("missing Codex authentication")]
    MissingAuth,

    #[error("unsupported Codex provider: {provider}")]
    UnsupportedProvider { provider: String },

    #[error("unsupported sandbox on this platform: {sandbox}")]
    UnsupportedSandbox { sandbox: String },

    #[error("malformed upstream event: {message}")]
    MalformedUpstreamEvent { message: String },

    #[error("agent execution failed: {message}")]
    Execution { message: String },
}
```

- [ ] **Step 4: Implement event mapper**

Create `D:\YunXi Agent\crates\yunxi-agent-codex\src\event_mapper.rs`:

```rust
use serde_json::Value;
use yunxi_agent_core::{
    AgentError, AgentEvent, AgentResult, AgentRunStatus, CommandStatus, FileChangeKind,
    McpToolStatus, PatchStatus, TokenUsage,
};

pub fn map_exec_jsonl(input: &str) -> AgentResult<Vec<AgentEvent>> {
    let mut events = Vec::new();
    for line in input.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let value: Value =
            serde_json::from_str(line).map_err(|err| AgentError::MalformedUpstreamEvent {
                message: err.to_string(),
            })?;
        events.extend(map_exec_json_event(value)?);
    }
    Ok(events)
}

pub fn map_exec_json_event(value: Value) -> AgentResult<Vec<AgentEvent>> {
    let event_type = value
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| AgentError::MalformedUpstreamEvent {
            message: "event is missing type".to_string(),
        })?;

    match event_type {
        "thread.started" => Ok(vec![AgentEvent::ThreadStarted {
            thread_id: required_string(&value, "thread_id")?,
        }]),
        "turn.started" => Ok(vec![AgentEvent::TurnStarted]),
        "turn.completed" => Ok(vec![AgentEvent::Completed {
            status: AgentRunStatus::Completed,
            usage: Some(TokenUsage {
                input_tokens: required_i64(&value["usage"], "input_tokens")?,
                cached_input_tokens: required_i64(&value["usage"], "cached_input_tokens")?,
                output_tokens: required_i64(&value["usage"], "output_tokens")?,
                reasoning_output_tokens: required_i64(
                    &value["usage"],
                    "reasoning_output_tokens",
                )?,
            }),
        }]),
        "turn.failed" => Ok(vec![AgentEvent::Error {
            message: value["error"]["message"]
                .as_str()
                .unwrap_or("turn failed")
                .to_string(),
        }]),
        "error" => Ok(vec![AgentEvent::Error {
            message: required_string(&value, "message")?,
        }]),
        "item.started" => map_item(value.get("item"), true),
        "item.updated" => map_item(value.get("item"), false),
        "item.completed" => map_item(value.get("item"), false),
        _ => Ok(Vec::new()),
    }
}

fn map_item(item: Option<&Value>, started: bool) -> AgentResult<Vec<AgentEvent>> {
    let item = item.ok_or_else(|| AgentError::MalformedUpstreamEvent {
        message: "item event is missing item".to_string(),
    })?;
    let id = item
        .get("id")
        .and_then(Value::as_str)
        .map(str::to_string);
    let item_type = item
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| AgentError::MalformedUpstreamEvent {
            message: "item is missing type".to_string(),
        })?;

    match item_type {
        "agent_message" => Ok(vec![AgentEvent::Message {
            content: required_string(item, "text")?,
        }]),
        "reasoning" => Ok(vec![AgentEvent::Reasoning {
            content: required_string(item, "text")?,
        }]),
        "command_execution" if started => Ok(vec![AgentEvent::CommandStarted {
            id,
            command: required_string(item, "command")?,
        }]),
        "command_execution" => Ok(vec![AgentEvent::CommandCompleted {
            id,
            command: required_string(item, "command")?,
            aggregated_output: item["aggregated_output"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            exit_code: item["exit_code"].as_i64().map(|code| code as i32),
            status: map_command_status(item["status"].as_str()),
        }]),
        "file_change" => {
            let mut events = Vec::new();
            for change in item["changes"].as_array().into_iter().flatten() {
                events.push(AgentEvent::FileChanged {
                    path: required_string(change, "path")?,
                    kind: map_file_change_kind(change["kind"].as_str()),
                });
            }
            events.push(AgentEvent::PatchCompleted {
                status: map_patch_status(item["status"].as_str()),
            });
            Ok(events)
        }
        "mcp_tool_call" if started => Ok(vec![AgentEvent::McpToolStarted {
            id,
            server: required_string(item, "server")?,
            tool: required_string(item, "tool")?,
        }]),
        "mcp_tool_call" => Ok(vec![AgentEvent::McpToolCompleted {
            id,
            server: required_string(item, "server")?,
            tool: required_string(item, "tool")?,
            status: map_mcp_status(item["status"].as_str()),
        }]),
        _ => Ok(Vec::new()),
    }
}

fn required_string(value: &Value, key: &str) -> AgentResult<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| AgentError::MalformedUpstreamEvent {
            message: format!("missing string field `{key}`"),
        })
}

fn required_i64(value: &Value, key: &str) -> AgentResult<i64> {
    value
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| AgentError::MalformedUpstreamEvent {
            message: format!("missing integer field `{key}`"),
        })
}

fn map_command_status(value: Option<&str>) -> CommandStatus {
    match value {
        Some("completed") => CommandStatus::Completed,
        Some("failed") => CommandStatus::Failed,
        Some("declined") => CommandStatus::Declined,
        _ => CommandStatus::InProgress,
    }
}

fn map_file_change_kind(value: Option<&str>) -> FileChangeKind {
    match value {
        Some("add") => FileChangeKind::Add,
        Some("delete") => FileChangeKind::Delete,
        _ => FileChangeKind::Update,
    }
}

fn map_patch_status(value: Option<&str>) -> PatchStatus {
    match value {
        Some("completed") => PatchStatus::Completed,
        Some("failed") => PatchStatus::Failed,
        _ => PatchStatus::InProgress,
    }
}

fn map_mcp_status(value: Option<&str>) -> McpToolStatus {
    match value {
        Some("completed") => McpToolStatus::Completed,
        Some("failed") => McpToolStatus::Failed,
        _ => McpToolStatus::InProgress,
    }
}
```

- [ ] **Step 5: Export mapper**

Modify `D:\YunXi Agent\crates\yunxi-agent-codex\src\lib.rs`:

```rust
mod event_mapper;
mod source_runtime;

pub use event_mapper::{map_exec_json_event, map_exec_jsonl};
pub use source_runtime::{CodexRuntimeSource, CodexRuntimeStatus};
```

- [ ] **Step 6: Run tests**

Run:

```powershell
cargo fmt
cargo test -p yunxi-agent-codex event_mapper_tests
cargo test
```

Expected: PASS.

- [ ] **Step 7: Commit event mapping**

Run:

```powershell
git add crates/yunxi-agent-core crates/yunxi-agent-codex
git commit -m "feat: map Codex headless events"
```

Expected: commit succeeds.

---

### Task 7: Codex Config Mapping

**Files:**
- Create: `D:\YunXi Agent\crates\yunxi-agent-codex\src\config_mapper.rs`
- Modify: `D:\YunXi Agent\crates\yunxi-agent-codex\src\lib.rs`
- Create: `D:\YunXi Agent\crates\yunxi-agent-codex\tests\config_mapper_tests.rs`

**Interfaces:**
- Consumes: `AgentConfig`, `ApprovalMode`, `SandboxMode`.
- Produces:
  - `CodexRunOptions`
  - `map_config_to_codex_options(config: &AgentConfig) -> AgentResult<CodexRunOptions>`

- [ ] **Step 1: Write failing mapping tests**

Create `D:\YunXi Agent\crates\yunxi-agent-codex\tests\config_mapper_tests.rs`:

```rust
use std::path::PathBuf;
use tempfile::TempDir;
use yunxi_agent_codex::{CodexApproval, CodexRunOptions, CodexSandbox, map_config_to_codex_options};
use yunxi_agent_core::{AgentConfig, ApprovalMode, SandboxMode};

#[test]
fn maps_basic_config_to_codex_options() {
    let temp = TempDir::new().expect("temp");
    let config = AgentConfig::new(temp.path())
        .with_model("gpt-5")
        .with_provider("openai")
        .with_codex_home(PathBuf::from("D:/codex-home"))
        .with_approval_mode(ApprovalMode::Never)
        .with_sandbox_mode(SandboxMode::DangerFullAccess);

    let options = map_config_to_codex_options(&config).expect("options");

    assert_eq!(
        options,
        CodexRunOptions {
            cwd: temp.path().to_path_buf(),
            model: Some("gpt-5".to_string()),
            provider: Some("openai".to_string()),
            codex_home: Some(PathBuf::from("D:/codex-home")),
            approval: CodexApproval::Never,
            sandbox: CodexSandbox::DangerFullAccess,
            ephemeral: true,
        }
    );
}

#[test]
fn rejects_missing_working_directory() {
    let config = AgentConfig::new(PathBuf::from("Z:/missing/workspace"));

    let error = map_config_to_codex_options(&config).expect_err("missing cwd should fail");

    assert!(error.to_string().contains("working directory does not exist"));
}
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```powershell
cargo test -p yunxi-agent-codex config_mapper_tests
```

Expected: FAIL because config mapper does not exist.

- [ ] **Step 3: Implement config mapper**

Create `D:\YunXi Agent\crates\yunxi-agent-codex\src\config_mapper.rs`:

```rust
use std::path::PathBuf;
use yunxi_agent_core::{AgentConfig, AgentError, AgentResult, ApprovalMode, SandboxMode};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexRunOptions {
    pub cwd: PathBuf,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub codex_home: Option<PathBuf>,
    pub approval: CodexApproval,
    pub sandbox: CodexSandbox,
    pub ephemeral: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodexApproval {
    Never,
    OnRequest,
    OnFailure,
    Untrusted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodexSandbox {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

pub fn map_config_to_codex_options(config: &AgentConfig) -> AgentResult<CodexRunOptions> {
    if !config.cwd.is_dir() {
        return Err(AgentError::MissingWorkingDirectory {
            path: config.cwd.display().to_string(),
        });
    }

    Ok(CodexRunOptions {
        cwd: config.cwd.clone(),
        model: config.model.clone(),
        provider: config.provider.clone(),
        codex_home: config.codex_home.clone(),
        approval: match config.approval_mode {
            ApprovalMode::Never => CodexApproval::Never,
            ApprovalMode::OnRequest => CodexApproval::OnRequest,
            ApprovalMode::OnFailure => CodexApproval::OnFailure,
            ApprovalMode::Untrusted => CodexApproval::Untrusted,
        },
        sandbox: match config.sandbox_mode {
            SandboxMode::ReadOnly => CodexSandbox::ReadOnly,
            SandboxMode::WorkspaceWrite => CodexSandbox::WorkspaceWrite,
            SandboxMode::DangerFullAccess => CodexSandbox::DangerFullAccess,
        },
        ephemeral: true,
    })
}
```

- [ ] **Step 4: Export config mapper**

Modify `D:\YunXi Agent\crates\yunxi-agent-codex\src\lib.rs`:

```rust
mod config_mapper;
mod event_mapper;
mod source_runtime;

pub use config_mapper::{
    CodexApproval, CodexRunOptions, CodexSandbox, map_config_to_codex_options,
};
pub use event_mapper::{map_exec_json_event, map_exec_jsonl};
pub use source_runtime::{CodexRuntimeSource, CodexRuntimeStatus};
```

- [ ] **Step 5: Run tests**

Run:

```powershell
cargo fmt
cargo test -p yunxi-agent-codex config_mapper_tests
cargo test
```

Expected: PASS.

- [ ] **Step 6: Commit config mapping**

Run:

```powershell
git add crates/yunxi-agent-codex
git commit -m "feat: map YunXi config to Codex options"
```

Expected: commit succeeds.

---

### Task 8: Native Codex Backend Compile Gate

**Files:**
- Modify: `D:\YunXi Agent\crates\yunxi-agent-codex\Cargo.toml`
- Create: `D:\YunXi Agent\crates\yunxi-agent-codex\src\native_exec.rs`
- Modify: `D:\YunXi Agent\crates\yunxi-agent-codex\src\lib.rs`
- Create: `D:\YunXi Agent\crates\yunxi-agent-codex\tests\native_compile_tests.rs`

**Interfaces:**
- Consumes: `CodexRunOptions`, `AgentInput`.
- Produces:
  - `CodexNativeBackend`
  - `CodexNativeBackend::new()`
  - `impl AgentBackend for CodexNativeBackend`

- [ ] **Step 1: Link local Codex source**

Run the helper from Task 1 using the user-provided checkout path:

```powershell
.\scripts\link-codex-source.ps1 -CodexCheckoutRoot "<path-to-codex-checkout>"
```

Expected:

```text
Linked D:\YunXi Agent\external\codex-rs -> <path-to-codex-checkout>\codex-rs
```

Do not commit `external/`.

- [ ] **Step 2: Write failing compile-gate test**

Create `D:\YunXi Agent\crates\yunxi-agent-codex\tests\native_compile_tests.rs`:

```rust
use yunxi_agent_codex::CodexNativeBackend;
use yunxi_agent_core::AgentBackend;

#[test]
fn native_backend_is_an_agent_backend() {
    fn assert_backend<T: AgentBackend>() {}

    assert_backend::<CodexNativeBackend>();
}
```

- [ ] **Step 3: Run test to verify failure**

Run:

```powershell
cargo test -p yunxi-agent-codex native_backend_is_an_agent_backend
```

Expected: FAIL because `CodexNativeBackend` does not exist.

- [ ] **Step 4: Add static upstream path dependencies behind feature**

Modify `D:\YunXi Agent\crates\yunxi-agent-codex\Cargo.toml`:

```toml
[features]
default = []
codex-native = [
    "dep:codex-exec",
    "dep:codex-arg0",
]
process-fallback = []

[dependencies]
anyhow.workspace = true
async-trait.workspace = true
serde.workspace = true
serde_json.workspace = true
tokio.workspace = true
yunxi-agent-core = { path = "../yunxi-agent-core" }
codex-exec = { path = "../../external/codex-rs/exec", optional = true }
codex-arg0 = { path = "../../external/codex-rs/arg0", optional = true }
```

If Cargo reports that upstream dependencies require additional direct path dependencies, add only the missing crates required by the compiler and keep them optional under `codex-native`.

- [ ] **Step 5: Implement native backend skeleton**

Create `D:\YunXi Agent\crates\yunxi-agent-codex\src\native_exec.rs`:

```rust
use crate::map_config_to_codex_options;
use yunxi_agent_core::{
    AgentBackend, AgentConfig, AgentError, AgentEvent, AgentInput, AgentResult, AgentRunResult,
    AgentRunStatus,
};

#[derive(Clone, Debug, Default)]
pub struct CodexNativeBackend;

impl CodexNativeBackend {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl AgentBackend for CodexNativeBackend {
    async fn run(
        &self,
        config: AgentConfig,
        input: AgentInput,
    ) -> AgentResult<AgentRunResult> {
        let prompt = input.prompt.trim();
        if prompt.is_empty() {
            return Err(AgentError::EmptyPrompt);
        }

        let _options = map_config_to_codex_options(&config)?;

        #[cfg(feature = "codex-native")]
        {
            run_native_codex(_options, prompt.to_string()).await
        }

        #[cfg(not(feature = "codex-native"))]
        {
            Err(AgentError::Execution {
                message: "codex-native feature is not enabled".to_string(),
            })
        }
    }
}

#[cfg(feature = "codex-native")]
async fn run_native_codex(
    _options: crate::CodexRunOptions,
    _prompt: String,
) -> AgentResult<AgentRunResult> {
    Err(AgentError::Execution {
        message: "native Codex runner requires Task 9 live runner wiring".to_string(),
    })
}
```

- [ ] **Step 6: Export native backend**

Modify `D:\YunXi Agent\crates\yunxi-agent-codex\src\lib.rs`:

```rust
mod config_mapper;
mod event_mapper;
mod native_exec;
mod source_runtime;

pub use config_mapper::{
    CodexApproval, CodexRunOptions, CodexSandbox, map_config_to_codex_options,
};
pub use event_mapper::{map_exec_json_event, map_exec_jsonl};
pub use native_exec::CodexNativeBackend;
pub use source_runtime::{CodexRuntimeSource, CodexRuntimeStatus};
```

- [ ] **Step 7: Run normal and feature checks**

Run:

```powershell
cargo fmt
cargo test -p yunxi-agent-codex native_backend_is_an_agent_backend
cargo check -p yunxi-agent-codex --features codex-native
cargo test
```

Expected:

- normal tests pass without needing live credentials
- `cargo check --features codex-native` compiles against the linked upstream source

- [ ] **Step 8: Commit native compile gate**

Run:

```powershell
git add crates/yunxi-agent-codex
git commit -m "feat: add native Codex backend compile gate"
```

Expected: commit succeeds.

---

### Task 9: Source-Level Native New-Thread Runner

**Files:**
- Modify: `D:\YunXi Agent\crates\yunxi-agent-codex\src\native_exec.rs`
- Create: `D:\YunXi Agent\crates\yunxi-agent-codex\tests\live_smoke_tests.rs`
- Modify: `D:\YunXi Agent\docs\extraction-status.md`

**Interfaces:**
- Consumes: upstream `codex-exec` source path dependencies.
- Produces: `CodexNativeBackend` live execution for new-thread text prompts.

- [ ] **Step 1: Write gated live smoke test**

Create `D:\YunXi Agent\crates\yunxi-agent-codex\tests\live_smoke_tests.rs`:

```rust
use std::path::PathBuf;
use yunxi_agent_codex::CodexNativeBackend;
use yunxi_agent_core::{Agent, AgentConfig, AgentInput, AgentRunStatus};

#[tokio::test]
async fn live_codex_backend_can_complete_simple_prompt_when_enabled() {
    if std::env::var("YUNXI_RUN_LIVE_CODEX_TESTS").ok().as_deref() != Some("1") {
        eprintln!("skipping live test; set YUNXI_RUN_LIVE_CODEX_TESTS=1");
        return;
    }

    let cwd = std::env::current_dir().expect("cwd");
    let mut config = AgentConfig::new(cwd);
    if let Ok(model) = std::env::var("YUNXI_LIVE_MODEL") {
        config = config.with_model(model);
    }
    if let Ok(codex_home) = std::env::var("YUNXI_LIVE_CODEX_HOME") {
        config = config.with_codex_home(PathBuf::from(codex_home));
    }

    let agent = Agent::new(config);
    let result = agent
        .run_with_backend(
            &CodexNativeBackend::new(),
            AgentInput::text("Reply with exactly: YunXi live smoke ok"),
        )
        .await
        .expect("live run should succeed");

    assert_eq!(result.status, AgentRunStatus::Completed);
    assert!(
        result
            .final_response
            .as_deref()
            .unwrap_or_default()
            .contains("YunXi live smoke ok")
    );
}
```

- [ ] **Step 2: Run gated test without env**

Run:

```powershell
cargo test -p yunxi-agent-codex live_codex_backend_can_complete_simple_prompt_when_enabled
```

Expected: PASS with skip message, because the environment variable is not set.

- [ ] **Step 3: Port upstream runner logic into `native_exec.rs`**

Use these upstream references:

- `external/codex-rs/exec/src/lib.rs`
- `external/codex-rs/exec/src/event_processor_with_jsonl_output.rs`
- `external/codex-rs/exec/src/exec_events.rs`
- `external/codex-rs/exec/src/cli.rs`

Implement the first live runner as source-level Rust, not shell-out:

```rust
#[cfg(feature = "codex-native")]
async fn run_native_codex(
    options: crate::CodexRunOptions,
    prompt: String,
) -> AgentResult<AgentRunResult> {
    let output = run_codex_exec_in_process(options, prompt).await?;
    let events = crate::map_exec_jsonl(&output.jsonl)?;
    let final_response = events.iter().rev().find_map(|event| match event {
        AgentEvent::Message { content } => Some(content.clone()),
        _ => None,
    });
    let status = if output.failed {
        AgentRunStatus::Failed
    } else {
        AgentRunStatus::Completed
    };

    Ok(AgentRunResult {
        status,
        final_response,
        events,
    })
}
```

The helper `run_codex_exec_in_process` must be implemented in Rust using upstream source-level APIs. It may temporarily reuse `codex_exec::Cli` construction and upstream event processors, but must not invoke `std::process::Command` for ordinary execution.

Required behavior in this task:

- new thread only
- text prompt only
- ephemeral mode on
- model override supported
- cwd supported
- approval and sandbox mapping supported
- JSONL events collected in memory

If an upstream function calls `std::process::exit`, do not call that function directly. Port the required subroutine into `yunxi-agent-codex` and return `AgentError` instead.

- [ ] **Step 4: Run feature tests**

Run:

```powershell
cargo fmt
cargo check -p yunxi-agent-codex --features codex-native
cargo test -p yunxi-agent-codex
cargo test
```

Expected: PASS without live credentials.

- [ ] **Step 5: Run optional live smoke test when credentials are available**

Only run this if the environment has Codex credentials:

```powershell
$env:YUNXI_RUN_LIVE_CODEX_TESTS = "1"
cargo test -p yunxi-agent-codex --features codex-native live_codex_backend_can_complete_simple_prompt_when_enabled -- --nocapture
```

Expected: PASS when credentials and network are available. If credentials are missing, record the exact error in `docs/extraction-status.md` and do not treat it as a normal test failure.

- [ ] **Step 6: Update extraction status**

Modify `D:\YunXi Agent\docs\extraction-status.md` to include:

```markdown
## Stage 2 Live Backend

The Codex native backend has a source-level new-thread runner behind the
`codex-native` feature. Normal tests do not require live credentials.

Current live scope:

- text prompt
- new headless thread
- ephemeral run
- cwd/model/provider mapping
- approval and sandbox mapping
- JSONL event mapping into YunXi events

Still expanding:

- resume/history CLI
- full MCP fixture coverage
- skills fixture coverage
- owned extraction of upstream runner internals
```

- [ ] **Step 7: Commit native live runner**

Run:

```powershell
git add crates/yunxi-agent-codex docs/extraction-status.md
git commit -m "feat: run native Codex headless backend"
```

Expected: commit succeeds.

---

### Task 10: Wire CLI To Codex Native Backend

**Files:**
- Modify: `D:\YunXi Agent\crates\yunxi-agent-cli\Cargo.toml`
- Modify: `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- Modify: `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`

**Interfaces:**
- Consumes: `yunxi_agent_codex::CodexNativeBackend`.
- Produces: `--live` and `--backend codex` execution through source-level backend.

- [ ] **Step 1: Write failing CLI expectation**

Modify `cli_rejects_live_backend_until_codex_crate_is_wired` in `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`:

```rust
#[test]
fn cli_live_backend_reports_feature_message_without_codex_native_feature() {
    let mut cmd = Command::cargo_bin("yunxi-agent-cli").expect("binary should build");

    cmd.args(["--live", "explain this project"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "codex-native feature is not enabled",
        ));
}
```

- [ ] **Step 2: Run test to verify failure**

Run:

```powershell
cargo test -p yunxi-agent-cli cli_live_backend_reports_feature_message_without_codex_native_feature
```

Expected: FAIL because CLI still emits the old wiring message.

- [ ] **Step 3: Add CLI dependency**

Modify `D:\YunXi Agent\crates\yunxi-agent-cli\Cargo.toml`:

```toml
[features]
default = []
codex-native = ["yunxi-agent-codex/codex-native"]

[dependencies]
anyhow.workspace = true
clap.workspace = true
serde_json.workspace = true
tokio.workspace = true
yunxi-agent-core = { path = "../yunxi-agent-core" }
yunxi-agent-codex = { path = "../yunxi-agent-codex" }
```

- [ ] **Step 4: Wire live backend**

Modify the `BackendKind::Codex` arm in `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`:

```rust
BackendKind::Codex => {
    let agent = Agent::new(config);
    agent
        .run_with_backend(
            &yunxi_agent_codex::CodexNativeBackend::new(),
            AgentInput::text(prompt),
        )
        .await
        .context("codex agent run failed")?
}
```

- [ ] **Step 5: Run tests and feature check**

Run:

```powershell
cargo fmt
cargo test -p yunxi-agent-cli
cargo check -p yunxi-agent-cli --features codex-native
cargo test
```

Expected: PASS.

- [ ] **Step 6: Commit CLI live backend**

Run:

```powershell
git add crates/yunxi-agent-cli
git commit -m "feat: wire CLI to Codex backend"
```

Expected: commit succeeds.

---

### Task 11: MCP, Skills, History, And Resume Coverage

**Files:**
- Modify: `D:\YunXi Agent\crates\yunxi-agent-codex\src\event_mapper.rs`
- Modify: `D:\YunXi Agent\crates\yunxi-agent-codex\tests\event_mapper_tests.rs`
- Modify: `D:\YunXi Agent\crates\yunxi-agent-codex\src\native_exec.rs`
- Modify: `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- Modify: `D:\YunXi Agent\README.md`

**Interfaces:**
- Consumes: upstream headless MCP, skills, rollout, and resume behavior.
- Produces:
  - event mapper coverage for MCP, skills-visible events, task list, and errors
  - resume option in native backend
  - CLI resume flags after new-thread live execution is stable

- [ ] **Step 1: Add fixture tests for MCP and task list events**

Extend `D:\YunXi Agent\crates\yunxi-agent-codex\tests\event_mapper_tests.rs`:

```rust
#[test]
fn maps_task_list_updates() {
    let jsonl = r#"
{"type":"item.started","item":{"id":"todo_1","type":"todo_list","items":[{"text":"inspect","completed":false}]}}
{"type":"item.updated","item":{"id":"todo_1","type":"todo_list","items":[{"text":"inspect","completed":true}]}}
"#;

    let events = map_exec_jsonl(jsonl).expect("events");

    assert!(events.iter().any(|event| matches!(
        event,
        AgentEvent::TodoUpdated { id, items }
            if id.as_deref() == Some("todo_1")
                && items.len() == 1
                && items[0].text == "inspect"
                && items[0].completed
    )));
}

#[test]
fn maps_failed_mcp_tool_call_to_completed_status() {
    let jsonl = r#"
{"type":"item.completed","item":{"id":"mcp_2","type":"mcp_tool_call","server":"fs","tool":"write_file","arguments":{},"result":null,"error":{"message":"denied"},"status":"failed"}}
"#;

    let events = map_exec_jsonl(jsonl).expect("events");

    assert!(events.contains(&AgentEvent::McpToolCompleted {
        id: Some("mcp_2".to_string()),
        server: "fs".to_string(),
        tool: "write_file".to_string(),
        status: McpToolStatus::Failed,
    }));
}
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```powershell
cargo test -p yunxi-agent-codex event_mapper_tests
```

Expected: FAIL while task list mapping does not exist.

- [ ] **Step 3: Implement task list mapping**

Add to `map_item` in `event_mapper.rs`:

```rust
"todo_list" => {
    let items = item["items"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|entry| {
            Ok(yunxi_agent_core::TodoStatus {
                text: required_string(entry, "text")?,
                completed: entry["completed"].as_bool().unwrap_or(false),
            })
        })
        .collect::<AgentResult<Vec<_>>>()?;
    Ok(vec![AgentEvent::TodoUpdated { id, items }])
}
```

- [ ] **Step 4: Add resume design hooks**

Extend the internal native run options in `native_exec.rs`:

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodexRunMode {
    NewThread,
    ResumeLast,
    ResumeById(String),
}
```

Use `CodexRunMode::NewThread` as the default. Add fields to the internal runner options only after tests assert that new-thread execution remains unchanged.

- [ ] **Step 5: Run tests**

Run:

```powershell
cargo fmt
cargo test -p yunxi-agent-codex
cargo test
```

Expected: PASS.

- [ ] **Step 6: Update README with capability matrix**

Add to `D:\YunXi Agent\README.md`:

```markdown
## Live Backend Capability Matrix

The Codex live backend is source-level integration with the Codex headless
Agent stack. Normal tests do not require credentials.

| Capability | Status |
| --- | --- |
| Text prompt, new thread | Supported |
| Model/provider config | Supported through Codex config mapping |
| Shell commands | Supported through Codex runtime |
| Patches/file changes | Supported through Codex runtime |
| Approval/sandbox mapping | Supported |
| MCP event mapping | Supported |
| Skills runtime | Provided by upstream Codex runtime |
| Session/rollout storage | Provided by upstream Codex runtime |
| Resume CLI | Staged after live new-thread stabilization |
```

- [ ] **Step 7: Commit capability coverage**

Run:

```powershell
git add crates/yunxi-agent-codex crates/yunxi-agent-cli README.md
git commit -m "feat: cover headless Agent capability events"
```

Expected: commit succeeds.

---

### Task 12: Documentation, Verification, And Push

**Files:**
- Modify: `D:\YunXi Agent\README.md`
- Modify: `D:\YunXi Agent\docs\extraction-status.md`
- Modify: `D:\YunXi Agent\docs\superpowers\plans\2026-07-09-yunxi-headless-agent-stack-extraction.md`

**Interfaces:**
- Consumes: completed stage-2 code.
- Produces: documented verification status and GitHub-ready branch.

- [ ] **Step 1: Run full verification**

Run:

```powershell
cargo fmt -- --check
cargo test
cargo check -p yunxi-agent-codex --features codex-native
cargo check -p yunxi-agent-cli --features codex-native
cargo run -p yunxi-agent-cli -- --backend dry-run "explain this project"
cargo run -p yunxi-agent-cli -- --backend dry-run --jsonl "explain this project"
git diff --check
```

Expected: PASS.

- [ ] **Step 2: Run optional live verification when credentials are available**

Only run when credentials and network are available:

```powershell
$env:YUNXI_RUN_LIVE_CODEX_TESTS = "1"
cargo test -p yunxi-agent-codex --features codex-native live_codex_backend_can_complete_simple_prompt_when_enabled -- --nocapture
```

Expected: PASS with valid credentials. If credentials are unavailable, document that live network verification was skipped.

- [ ] **Step 3: Update extraction status**

Ensure `D:\YunXi Agent\docs\extraction-status.md` includes:

```markdown
## Stage 2 Verification

- `cargo fmt -- --check`: pass
- `cargo test`: pass
- `cargo check -p yunxi-agent-codex --features codex-native`: pass
- `cargo check -p yunxi-agent-cli --features codex-native`: pass
- dry-run CLI smoke: pass
- dry-run JSONL smoke: pass

Live credential smoke is optional and must be recorded with the exact date and
result when it is run.
```

- [ ] **Step 4: Commit docs and verification status**

Run:

```powershell
git add README.md docs/extraction-status.md docs/superpowers/plans/2026-07-09-yunxi-headless-agent-stack-extraction.md
git commit -m "docs: update headless extraction status"
```

Expected: commit succeeds.

- [ ] **Step 5: Push to GitHub**

Because this environment previously had Git/curl connectivity issues, prefer the already proven GitHub API mirror method if `git push` fails.

Try:

```powershell
git push origin master
```

Expected: push succeeds. If it fails with Git/curl network timeout, mirror commits through GitHub REST Git Database API and verify remote `master` equals local `HEAD`.

- [ ] **Step 6: Final status check**

Run:

```powershell
git status --short --branch
git rev-parse HEAD
git rev-parse origin/master
```

Expected:

- worktree clean
- local HEAD equals `origin/master`

---

## Plan Self-Review

- Spec coverage: Tasks cover backend abstraction, source-level Codex integration, event mapping, config/auth/provider mapping, approval and sandbox mapping, CLI live backend, MCP/task-list/error events, optional live smoke tests, documentation, and push.
- Scope: This is intentionally one second-stage plan because the user confirmed the goal is complete Codex CLI headless Agent capability. The plan still uses independently reviewable milestones so each task can compile and pass tests.
- Gap scan: The plan contains no unresolved markers or unspecified implementation gaps. The native runner task names exact upstream files and requires source-level integration instead of process shell-out.
- Type consistency: `AgentBackend`, `BackendKind`, `DryRunBackend`, `CodexNativeBackend`, `CodexRunOptions`, `CodexApproval`, `CodexSandbox`, `map_config_to_codex_options`, `map_exec_jsonl`, and `map_exec_json_event` are consistently named across tasks.
- Test strategy: Normal tests do not require credentials. Live tests are explicitly gated by `YUNXI_RUN_LIVE_CODEX_TESTS=1`.

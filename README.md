# YunXi Agent v1.3.0

YunXi Agent v1.3.0 is a terminal-first Rust Agent CLI and reusable core library
built from the Codex CLI source extraction work. The default runtime is
YunXi-owned and does not depend on the upstream Codex runtime.

When DeepSeek credentials are configured, the default `yunxi` command now uses
the real DeepSeek provider automatically. Without credentials it remains usable
through the clearly labelled offline provider.

## Layout

- `crates/yunxi-agent-core`: reusable Agent facade and extraction boundary
- `crates/yunxi-agent-provider`: YunXi-owned provider request/response boundary
- `crates/yunxi-agent-tools`: YunXi-owned tool execution boundary
- `crates/yunxi-agent-storage`: YunXi-owned session storage boundary
- `crates/yunxi-agent-runtime`: YunXi-owned Agent runtime boundary
- `crates/yunxi-agent-codex`: standalone compatibility layer around the vendored Codex headless runtime
- `crates/yunxi-agent-cli`: v1.3.0 terminal CLI package that builds `yunxi`
  and the compatibility `yunxi-agent-cli`
- `vendor/codex-rs`: vendored Codex Rust workspace source used by `codex-native`
- `docs/extraction-status.md`: current extraction status and known gaps
- `docs/superpowers/specs`: design specs
- `docs/superpowers/plans`: implementation plans

## Current Capabilities

- Compiles as an independent Rust workspace
- Builds a terminal command named `yunxi`
- Starts an interactive terminal session when `yunxi` is run without a prompt
- Streams runtime events to the terminal while an interactive turn is running
- Prompts for interactive approval and `request_user_input` tool calls
- Propagates Ctrl+C cancellation into the runtime and shell exec layer
- Keeps the interactive session open when one provider or tool turn fails
- Preserves assistant tool calls and matching tool result ids across provider turns
- Preserves one-shot execution through `yunxi "your task"`
- Automatically selects the real DeepSeek provider when credentials are present
- Supports `--offline` for deterministic local and automation runs
- Provides facade types for Agent configuration, input, events, results, and errors
- Runs the default `yunxi` backend through YunXi-owned runtime/provider/tools/storage crates
- Keeps a dry-run Agent path for deterministic smoke checks
- Vendors the Codex Rust workspace source needed by the native backend
- Retains a boundary for checking local Codex CLI checkout shape during future refreshes
- Runs the default workspace without `yunxi-agent-codex` in the CLI dependency graph
- Keeps the source-level Codex compatibility backend outside the default workspace and CLI graph

## Build

```powershell
cargo test
cargo build -p yunxi-agent-cli --release --bins
```

The default CLI path does not require `vendor/codex-rs`.

The `codex-native` feature reads Codex Rust source from `vendor/codex-rs`.
`external/codex-rs` is only a local refresh aid and is not required for normal
YunXi checkouts.

## Install On Windows

Build and install the v1.3.0 CLI into a user-local bin directory:

```powershell
Set-Location "D:\YunXi Agent"
.\scripts\install\install-yunxi.ps1 -AddToPath
```

Open a new terminal after `-AddToPath`, then run:

```powershell
yunxi --version
yunxi
yunxi "explain this project"
```

The installer copies both `yunxi.exe` and the compatibility
`yunxi-agent-cli.exe`.

## Interactive CLI

Run `yunxi` without a prompt to enter interactive mode:

```powershell
yunxi
```

Useful interactive commands:

- `/help`: show available commands
- `/session`: show the active session id, turn count, cwd, model, and provider
- `/resume <session_id>`: continue from a saved session
- `/model [name]`: show or switch the model field
- `/provider [name]`: show or switch the provider field
- `/cwd`: show the active working directory
- `/clear`: clear the terminal
- `/exit` or `/quit`: leave interactive mode

One-shot and automation modes remain available:

```powershell
yunxi "explain this project"
yunxi --offline "run without a model provider"
yunxi --jsonl "explain this project"
```

`--json` and `--jsonl` never enter the REPL when a prompt is missing; they keep
returning structured errors for script safety.

## Run From Source

```powershell
cargo run -p yunxi-agent-cli --bin yunxi -- "explain this project"
cargo run -p yunxi-agent-cli --bin yunxi -- --backend yunxi "explain this project"
cargo run -p yunxi-agent-cli -- "explain this project"
cargo run -p yunxi-agent-cli -- --backend yunxi "explain this project"
cargo run -p yunxi-agent-cli -- --backend dry-run "explain this project"
cargo run -p yunxi-agent-cli -- --cwd "D:\some\repo" "fix the failing test"
cargo run -p yunxi-agent-cli -- --json "explain this project"
cargo check --manifest-path crates/yunxi-agent-codex/Cargo.toml --features codex-native
```

## DeepSeek Provider

YunXi Agent keeps provider credentials out of committed source. Import a
DeepSeek credential from a local private file into the current user's Windows
environment with the secret-safe helper:

```powershell
.\scripts\provider\import-deepseek-credential.ps1 `
    -ApiFile "C:\private\api.txt" `
    -CredentialIndex 1
```

The helper automatically uses a file containing exactly one credential
candidate. When a private file contains several provider credentials,
`-CredentialIndex` must select one explicitly; the helper never guesses or
prints a candidate value. It configures `DEEPSEEK_API_KEY`,
`YUNXI_PROVIDER_PROFILE=deepseek`, and the default model. Open a new PowerShell
after importing, then run:

```powershell
yunxi
yunxi "Reply with a one-line status"
```

Provider selection precedence is:

1. `--provider-live` forces a real provider and fails early when credentials are missing.
2. `--offline` forces the deterministic offline provider.
3. Default auto mode uses configured credentials and otherwise falls back offline.

For temporary process-only configuration:

```powershell
$env:YUNXI_PROVIDER_PROFILE = "deepseek"
$env:DEEPSEEK_API_KEY = "<your-api-key>"
$env:YUNXI_AGENT_MODEL = "deepseek-v4-flash"
yunxi "Reply with a one-line status"
```

Generic OpenAI-compatible providers can continue to use
`YUNXI_PROVIDER_API_KEY`, `YUNXI_PROVIDER_API_KEY_ENV`,
`YUNXI_PROVIDER_BASE_URL`, and `OPENAI_API_KEY`. Never commit real API keys.
The repository also keeps a secret-safe DeepSeek smoke helper at
`scripts\provider\deepseek-live-smoke.ps1`; it requires an explicit
`-ApiFile`, requires `-CredentialIndex` for multi-credential files, and never
stores that path in project configuration.

## Version Tags

Each released version keeps its own immutable Git tag. v1.2 adds `v1.2.0` and
preserves `v1.0.0` and `v1.1.0` as rollback points. Do not delete or move old
version tags.

v1.2.1 adds immutable tag `v1.2.1` without moving `v1.2.0`. DeepSeek HTTP
400/422 responses receive one compatibility retry without optional request
`metadata`; a failed turn is rendered as a redacted error and returns control
to `yunxi>`. Tool-loop history uses matching assistant `tool_calls` and
`tool_call_id` fields for OpenAI/DeepSeek-compatible APIs.

v1.3.0 adds immutable tag `v1.3.0` without moving earlier tags. It promotes the
interactive terminal host from batch rendering to streaming runtime events,
approval/user-input request handling, and Ctrl+C cancellation propagation into
YunXi-owned runtime/tools/exec code.

## Backend Capability Matrix

| Backend | Default | Owner | Purpose |
| --- | --- | --- | --- |
| `yunxi` | Yes | YunXi runtime crates | Autonomous runtime migration path |
| `dry-run` | No | `yunxi-agent-core` | Deterministic smoke checks |
| `codex` / `--live` | No | Detached | Reports compatibility guidance in the default CLI |

## Codex Compatibility Matrix

The Codex compatibility crate is a source-level integration with the Codex
headless Agent stack. It is not part of the default workspace member set or the
default CLI dependency graph. Normal tests do not require credentials.

| Capability | Status |
| --- | --- |
| Text prompt, new thread | Supported behind `codex-native` |
| Model/provider config | Supported through Codex config mapping |
| Shell commands | Provided by embedded Codex runtime |
| Patches/file changes | Provided by embedded Codex runtime |
| Approval/sandbox mapping | Supported |
| MCP event mapping | Supported |
| Skills runtime | Provided by embedded Codex runtime |
| Session/rollout storage | Provided by embedded Codex runtime |
| Resume CLI | Staged after live new-thread stabilization |

## CodeGraph

If `.codegraph/` exists, use CodeGraph first when locating or understanding code in this repository.

## GitHub Publication

Project automation reads and writes GitHub branch, tree, commit, ref, and tag
state through the GitHub REST API. GitHub tokens remain local and must never be
printed, logged, or committed. Normal repository documentation does not embed
token values or private credential-file paths.

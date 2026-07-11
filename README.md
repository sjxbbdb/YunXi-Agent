# YunXi Agent v1.0

YunXi Agent v1.0 is a terminal-first Rust Agent CLI and reusable core library
built from the Codex CLI source extraction work. The default runtime is
YunXi-owned and does not depend on the upstream Codex runtime.

## Layout

- `crates/yunxi-agent-core`: reusable Agent facade and extraction boundary
- `crates/yunxi-agent-provider`: YunXi-owned provider request/response boundary
- `crates/yunxi-agent-tools`: YunXi-owned tool execution boundary
- `crates/yunxi-agent-storage`: YunXi-owned session storage boundary
- `crates/yunxi-agent-runtime`: YunXi-owned Agent runtime boundary
- `crates/yunxi-agent-codex`: standalone compatibility layer around the vendored Codex headless runtime
- `crates/yunxi-agent-cli`: v1.0 terminal CLI package that builds `yunxi`
  and the compatibility `yunxi-agent-cli`
- `vendor/codex-rs`: vendored Codex Rust workspace source used by `codex-native`
- `docs/extraction-status.md`: current extraction status and known gaps
- `docs/superpowers/specs`: design specs
- `docs/superpowers/plans`: implementation plans

## Current Capabilities

- Compiles as an independent Rust workspace
- Builds a terminal command named `yunxi`
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

Build and install the v1.0 CLI into a user-local bin directory:

```powershell
Set-Location "D:\YunXi Agent"
.\scripts\install\install-yunxi.ps1 -AddToPath
```

Open a new terminal after `-AddToPath`, then run:

```powershell
yunxi --version
yunxi "explain this project"
```

The installer copies both `yunxi.exe` and the compatibility
`yunxi-agent-cli.exe`.

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

## Live Provider Smoke

YunXi Agent keeps provider credentials out of committed source. For a live
OpenAI-compatible provider, set environment variables in the current shell:

```powershell
$env:YUNXI_PROVIDER_PROFILE = "deepseek"
$env:YUNXI_PROVIDER_BASE_URL = "https://api.deepseek.com"
$env:YUNXI_PROVIDER_API_KEY = "<your-api-key>"
$env:YUNXI_AGENT_MODEL = "deepseek-v4-flash"
yunxi --provider-live --model deepseek-v4-flash "Reply with a one-line status"
```

Do not commit real API keys. The repository also keeps a secret-safe DeepSeek
smoke helper at `scripts\provider\deepseek-live-smoke.ps1`.

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

## GitHub

After local work is ready, add your GitHub remote:

```powershell
git remote add origin <your-repository-url>
git push -u origin master
```

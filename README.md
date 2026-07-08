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
- Provides a boundary for verifying the local Codex CLI checkout shape

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

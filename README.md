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
- Provides a source-level Codex headless backend behind the `codex-native` feature

## Build

```powershell
cargo test
```

## Run

```powershell
cargo run -p yunxi-agent-cli -- "explain this project"
cargo run -p yunxi-agent-cli -- --cwd "D:\some\repo" "fix the failing test"
cargo run -p yunxi-agent-cli -- --json "explain this project"
cargo run -p yunxi-agent-cli --features codex-native -- --live "explain this project"
```

## Live Backend Capability Matrix

The Codex live backend is a source-level integration with the Codex headless
Agent stack. Normal tests do not require credentials.

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

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

# YunXi Agent v1.0 CLI Packaging Design

## Goal

Package the current autonomous YunXi Agent development result as a terminal-first
CLI product named `YunXi Agent v1.0`.

## Product Boundary

YunXi Agent v1.0 is a headless CLI package over the already verified YunXi-owned
runtime. It does not add TUI, desktop, cloud task, marketplace, updater, doctor,
or installer product surfaces. The first release focuses on a reliable local
terminal command and keeps the model/provider layer replaceable.

## Command Shape

- Primary command: `yunxi`
- Compatibility command: `yunxi-agent-cli`
- Version: `1.0.0`
- Default backend: `yunxi`
- Existing flags and subcommands remain available:
  - `--backend`
  - `--cwd`
  - `--model`
  - `--provider`
  - `--provider-live`
  - `--json`
  - `--jsonl`
  - `sessions`
  - `parity`

## Installation Shape

The Windows packaging entry point is a PowerShell installer script:

`scripts/install/install-yunxi.ps1`

The script builds release binaries with Cargo, installs `yunxi.exe` and the
compatibility `yunxi-agent-cli.exe` into a user-local bin directory, and can
optionally add that directory to the user PATH. The installer does not persist
API keys, modify project source, or require upstream Codex runtime dependencies.

## Runtime Guarantees

- Default CLI dependency graph must not include `vendor/codex-rs`, `codex-*`, or
  `yunxi-agent-codex`.
- The default command must run through YunXi-owned runtime/provider/tools/storage
  crates.
- Live provider usage must continue to read credentials from environment
  variables or existing smoke harnesses, never from committed source.
- The old `yunxi-agent-cli` binary remains available for tests and compatibility.

## Verification Gate

After construction, run the unified gate:

- `cargo fmt`
- `cargo fmt -- --check`
- `cargo test`
- `cargo check --workspace`
- `cargo build -p yunxi-agent-cli --release --bins`
- `target\release\yunxi.exe --version`
- `target\release\yunxi.exe --backend yunxi "YunXi Agent v1.0 terminal smoke"`
- `target\release\yunxi-agent-cli.exe --version`
- `cargo tree -p yunxi-agent-cli`
- default dependency keyword scan
- owned-source secret scan
- `git diff --check`
- `codegraph sync "D:\YunXi Agent"`

Then clean `target`, root `.yunxi`, and temporary smoke artifacts, update the
desktop development log, commit, and sync to GitHub.

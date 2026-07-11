# YunXi Agent v1.0 CLI Packaging Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Package the verified YunXi-owned runtime as a local terminal CLI named `YunXi Agent v1.0`.

**Architecture:** Keep the existing Rust workspace and CLI implementation, add a primary `yunxi` binary name alongside the compatibility `yunxi-agent-cli` binary, and provide a Windows installer script that builds release binaries and copies them to a user-local bin directory.

**Tech Stack:** Rust 2024, Cargo, Clap, PowerShell, existing YunXi-owned crates.

## Global Constraints

- Default YunXi runtime must not depend on `vendor/codex-rs`.
- Default CLI dependency graph must not include `codex-*` crates.
- Default `yunxi-agent-cli` package must not depend on `yunxi-agent-codex`.
- Do not add TUI, desktop app, cloud tasks, doctor, update, completion, marketplace, or installer product surfaces beyond the local PowerShell install helper.
- Do not print, persist, or commit API keys or bearer tokens.
- Build first, then run the unified verification gate once construction is complete.
- Update `C:\Users\admin\Desktop\YunXi Agent开发日志.md` at the end of the task.

---

### Task 1: Version And Binary Names

**Files:**
- Modify: `D:\YunXi Agent\Cargo.toml`
- Modify: `D:\YunXi Agent\crates\yunxi-agent-cli\Cargo.toml`
- Modify: `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`

**Interfaces:**
- Produces primary binary `yunxi`.
- Preserves compatibility binary `yunxi-agent-cli`.
- Produces `yunxi --version` using package version `1.0.0`.

- [ ] Set `[workspace.package].version` to `1.0.0`.
- [ ] Add a `[[bin]]` entry named `yunxi` pointing at `src/main.rs`.
- [ ] Keep the existing `[[bin]]` entry named `yunxi-agent-cli`.
- [ ] Change Clap command metadata to `name = "yunxi"`, `version`, and `about = "YunXi Agent v1.0 terminal CLI"`.

### Task 2: Installer Script

**Files:**
- Create: `D:\YunXi Agent\scripts\install\install-yunxi.ps1`

**Interfaces:**
- Consumes Cargo package `yunxi-agent-cli`.
- Produces user-local `yunxi.exe` and `yunxi-agent-cli.exe`.
- Supports optional user PATH update through `-AddToPath`.

- [ ] Create a PowerShell script with parameters `InstallDir`, `Configuration`, `AddToPath`, and `SkipBuild`.
- [ ] Resolve repo root from the script path.
- [ ] Build release binaries with `cargo build -p yunxi-agent-cli --release --bins` unless `-SkipBuild` is set.
- [ ] Copy both binaries into the install directory.
- [ ] If `-AddToPath` is set, add the install directory to the user PATH only if missing.
- [ ] Print machine-readable summary lines without secrets.

### Task 3: Documentation And Tests

**Files:**
- Modify: `D:\YunXi Agent\README.md`
- Modify: `D:\YunXi Agent\docs\extraction-status.md`
- Modify: `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`

**Interfaces:**
- README documents `YunXi Agent v1.0`, `yunxi`, install, run, and live provider environment usage.
- Status page records Stage v1.0 packaging.
- CLI tests prove the primary `yunxi` binary exists and version output works.

- [ ] Update README title and current capabilities to present the package as `YunXi Agent v1.0`.
- [ ] Add Windows install and terminal usage examples.
- [ ] Add live provider environment variable example without any real key.
- [ ] Add status page section for v1.0 packaging.
- [ ] Add a CLI test using `Command::cargo_bin("yunxi")` for `--version`.
- [ ] Add a CLI test that the compatibility binary still accepts `--version`.

### Task 4: Unified Verification And Release Hygiene

**Files:**
- Modify: `C:\Users\admin\Desktop\YunXi Agent开发日志.md`

**Interfaces:**
- Produces final evidence that v1.0 CLI packaging works.
- Leaves no `target`, root `.yunxi`, or temporary smoke artifacts behind.

- [ ] Run `cargo fmt`.
- [ ] Run `cargo fmt -- --check`.
- [ ] Run `cargo test`.
- [ ] Run `cargo check --workspace`.
- [ ] Run `cargo build -p yunxi-agent-cli --release --bins`.
- [ ] Run `target\release\yunxi.exe --version`.
- [ ] Run `target\release\yunxi.exe --backend yunxi "YunXi Agent v1.0 terminal smoke"`.
- [ ] Run `target\release\yunxi-agent-cli.exe --version`.
- [ ] Run dependency keyword scan.
- [ ] Run owned-source secret scan.
- [ ] Run `git diff --check`.
- [ ] Run `codegraph sync "D:\YunXi Agent"`.
- [ ] Run `cargo clean`.
- [ ] Remove root `.yunxi` and temporary smoke artifacts.
- [ ] Append the detailed timestamped development log.
- [ ] Commit and sync GitHub.

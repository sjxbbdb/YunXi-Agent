# Task 4 Report: CLI Backend Selection And JSONL Rendering

## Implemented

- Added CLI backend selection in `crates/yunxi-agent-cli/src/main.rs` with:
  - `--backend dry-run|codex`
  - `--live`
  - `--codex-home <PATH>`
  - `--approval never|on-request|on-failure|untrusted`
  - `--sandbox read-only|workspace-write|danger-full-access`
  - `--jsonl`
- Preserved the existing dry-run text output path for default CLI usage.
- Routed dry-run through the core `Agent` and `AgentConfig` with approval, sandbox, model, provider, and codex home applied.
- Made `--live` and `--backend codex` fail with `codex backend is not wired into the CLI yet`.
- Added JSONL rendering that prints one serialized event per line.

## File Changes

- `crates/yunxi-agent-cli/Cargo.toml`
- `crates/yunxi-agent-cli/src/main.rs`
- `crates/yunxi-agent-cli/tests/cli_tests.rs`
- `crates/yunxi-agent-cli/tests/jsonl_tests.rs`

## TDD Evidence

### RED

- `cargo test -p yunxi-agent-cli --test cli_tests`
  - Failed with `error: unexpected argument '--backend' found`
  - Failed with `error: unexpected argument '--live' found`
- `cargo test -p yunxi-agent-cli --test jsonl_tests`
  - Failed with `error: unexpected argument '--backend' found`

### GREEN

- After implementation:
  - `cargo test -p yunxi-agent-cli --test cli_tests` passed
  - `cargo test -p yunxi-agent-cli --test jsonl_tests` passed
  - `cargo test` passed for the full workspace

## Test Commands and Results

- `cargo test -p yunxi-agent-cli --test cli_tests`
  - Pass
- `cargo test -p yunxi-agent-cli --test jsonl_tests`
  - Pass
- `cargo fmt`
  - Pass
- `cargo test`
  - Pass

## Self-Review

- The new flags are wired through Clap with explicit value enums, which keeps the CLI surface predictable.
- The dry-run path still matches the previous text output when no JSON flags are passed.
- JSONL mode serializes each event on its own line and the test validates both line count shape and JSON parseability.
- The codex backend remains intentionally unimplemented at the CLI layer, matching the task brief.

## Concerns

- `--json` and `--jsonl` are both accepted; `--jsonl` wins when both are present. The brief did not require mutual exclusion, so I left compatibility behavior intact.
- The codex backend is still a stub by design and will need a follow-up task when the real backend wiring exists.

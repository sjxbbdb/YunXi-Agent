# Extraction Status

## Current Stage

The repository currently contains:

- A standalone Rust workspace
- `yunxi-agent-core` facade types
- A dry-run `Agent` runner
- A minimal `yunxi-agent-cli`
- A `CodexSource` verification boundary for the local Codex CLI checkout
- A `yunxi-agent-codex` integration crate for upstream Codex headless runtime

## Verified Codex Source

The Codex source checkout is a user-provided local Codex CLI checkout.
Documentation should not hardcode machine-specific checkout paths.

The `CodexSource` boundary verifies the checkout shape by checking for:

- `codex-rs/Cargo.toml`
- `codex-rs/exec/src/lib.rs`
- `codex-rs/app-server-client/Cargo.toml`

The controller confirmed these three files exist in the real local checkout
used for this extraction, but that local path is intentionally not recorded
here.

## Stage 2 Live Backend

The Codex native backend has a source-level new-thread runner behind the
`codex-native` feature. Normal tests do not require live credentials.

Current live scope:

- text prompt
- new headless thread
- ephemeral run
- cwd/model/provider mapping
- approval and sandbox mapping
- in-process Codex app-server runtime
- JSONL event mapping into YunXi events

The in-process runtime is the upstream Codex Agent stack, so shell execution,
patches, AGENTS.md loading, MCP configuration, skills runtime, session storage,
and rollout behavior stay inside the embedded Codex runtime rather than being
reimplemented in YunXi core.

Still expanding:

- resume/history CLI flags
- full MCP fixture coverage
- skills fixture coverage
- Windows helper/arg0 packaging verification for patch and sandbox helpers
- owned extraction of selected upstream runner internals after the source-level
  bridge is stable

## Stage 2 Verification

Verified on 2026-07-09:

- `cargo fmt -- --check`: pass
- `cargo test`: pass
- `cargo check -p yunxi-agent-codex --features codex-native`: pass
- `cargo check -p yunxi-agent-cli --features codex-native`: pass
- `cargo build -p yunxi-agent-cli --features codex-native`: pass
- gated live smoke without `YUNXI_RUN_LIVE_CODEX_TESTS=1`: pass and skipped
- dry-run CLI smoke: pass
- dry-run JSONL smoke: pass
- `git diff --check`: pass

Live credential smoke was not run; it remains gated by
`YUNXI_RUN_LIVE_CODEX_TESTS=1`.

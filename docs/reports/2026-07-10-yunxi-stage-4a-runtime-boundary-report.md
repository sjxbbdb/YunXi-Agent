# YunXi Stage 4A Runtime Boundary Report

## Goal

Stage 4A starts the move from a Codex-backed extraction to a YunXi-owned Agent
runtime. The immediate goal is not to delete the compatibility backend. It is to
make normal CLI execution enter YunXi-owned runtime code first.

## Implemented Boundary

This slice adds four YunXi-owned crates:

- `yunxi-agent-runtime`: Agent turn orchestration, runtime backend, event sink
- `yunxi-agent-provider`: provider-neutral request/response and provider trait
- `yunxi-agent-tools`: tool request/response and tool runtime trait
- `yunxi-agent-storage`: session id, session record, and session store trait

The CLI now defaults to:

```powershell
cargo run -p yunxi-agent-cli -- "explain this project"
```

which routes to the `yunxi` backend. The explicit compatibility path remains:

```powershell
cargo run -p yunxi-agent-cli -- --backend codex "explain this project"
cargo run -p yunxi-agent-cli --features codex-native -- --live "explain this project"
```

## Dependency Boundary

The Stage 4A YunXi-owned crates do not depend on `vendor/codex-rs`.

Allowed temporary dependency:

- `yunxi-agent-codex` may still depend on vendored Codex crates behind
  `codex-native`.

Disallowed for YunXi-owned runtime crates:

- direct Codex path dependencies
- Codex event mapping as the primary event source
- default CLI execution through `yunxi-agent-codex`

## Remaining Work

Stage 4A is the runtime boundary. Full autonomy still requires later slices:

- provider adapters and auth in `yunxi-agent-provider`
- shell, patch, MCP, skills, approval, and sandbox execution in
  `yunxi-agent-tools`
- file-backed session/history/resume support in `yunxi-agent-storage`
- removal or isolation of `yunxi-agent-codex` from the normal workspace path
- final vendor detachment check after compatibility code is no longer needed

## Verification Result

Unified verification passed on 2026-07-10:

```powershell
cargo fmt -- --check
cargo test
cargo check -p yunxi-agent-runtime
cargo check -p yunxi-agent-provider
cargo check -p yunxi-agent-tools
cargo check -p yunxi-agent-storage
cargo check -p yunxi-agent-cli
cargo build -p yunxi-agent-cli
cargo run -p yunxi-agent-cli -- --backend yunxi --jsonl "explain this project"
cargo run -p yunxi-agent-cli -- --backend dry-run "explain this project"
git diff --check
```

`git diff --check` reported only Windows line-ending warnings and no whitespace
errors.

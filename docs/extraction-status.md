# Extraction Status

## Current Stage

The repository currently contains:

- A standalone Rust workspace
- `yunxi-agent-core` facade types
- A dry-run `Agent` runner
- A minimal `yunxi-agent-cli`
- A `CodexSource` verification boundary for the local Codex CLI checkout

## Verified Codex Source

The expected local Codex source checkout is:

```text
D:\婧愮爜\codex
```

The first verification boundary checks for:

- `codex-rs/Cargo.toml`
- `codex-rs/exec/src/lib.rs`
- `codex-rs/app-server-client/Cargo.toml`

## Not Yet Extracted

- Live model execution
- Codex non-interactive execution adapter
- Shell command safety integration
- Patch application integration
- Approval and sandbox mapping to upstream Codex types
- MCP, skills, and history restoration

## Next Extraction Step

Connect `yunxi-agent-core` to the smallest viable Codex non-interactive execution path. The likely candidates are:

- A direct `codex-core` thread/turn path
- A wrapper around the existing `codex-exec` flow
- A narrower adapter around `codex-app-server-client` if it proves cleaner

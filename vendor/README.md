# Vendored Source

This directory contains source snapshots required to make YunXi Agent buildable
without developer-local source links.

## `codex-rs`

`vendor/codex-rs` is a mechanical snapshot of the Codex Rust workspace used by
the YunXi native Agent backend.

- Source path at import time: `external/codex-rs`
- Upstream commit at import time: `f1affbac5e5164b2bae825e9b39e9868bc4e0be2`
- Snapshot date: 2026-07-10
- Import scope: full Codex Rust workspace source
- Excluded during import: `target`, `.git`, `node_modules`

The first import intentionally preserves upstream layout and crate names. YunXi
owned crates depend on this source through path dependencies, while YunXi public
APIs remain in `crates/yunxi-agent-core`, `crates/yunxi-agent-cli`, and
`crates/yunxi-agent-codex`.

Future refreshes should copy a new upstream snapshot into `vendor/codex-rs`,
reapply the local patches documented under `vendor/codex-rs/patches`, and then
run the Stage 3 verification commands from the source vendoring design.

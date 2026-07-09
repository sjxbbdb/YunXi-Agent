# YunXi Codex Source Vendoring Design

## Purpose

YunXi Agent is being built from the Codex CLI source, but the project should not
remain dependent on a developer-local checkout such as `<local-codex-checkout>`.

Stage 3 makes the repository self-contained by copying the Codex Rust source
needed by the headless Agent runtime into the YunXi repository first. After that
source is stable inside YunXi, later stages can gradually remove Codex naming,
delete unused product surfaces, and replace internals with YunXi-owned modules.

The guiding rule is:

```text
First extract and preserve capability. Then de-Codex gradually.
```

## Current State

The Stage 2 implementation already has:

- `yunxi-agent-core`: YunXi facade types and backend trait.
- `yunxi-agent-cli`: dry-run CLI and live backend selection.
- `yunxi-agent-codex`: integration layer around Codex headless runtime.
- `external/codex-rs`: ignored local source link to the upstream Codex Rust
  workspace.
- `codex-native` feature: compiles and builds the embedded Codex headless
  backend when the local source link exists.

The problem is that `external/codex-rs` is not committed. A fresh clone of
YunXi from GitHub cannot build the live backend until the developer manually
provides and links the upstream Codex checkout.

## Goals

- Make the YunXi GitHub repository self-contained for the live Agent backend.
- Preserve the current Codex headless Agent capability while moving source into
  the YunXi repository.
- Keep YunXi public APIs independent from upstream Codex types.
- Keep the first source import as mechanical as possible.
- Continue to support normal `cargo test` without live credentials.
- Keep live/network tests gated by explicit environment variables.
- Create a clean foundation for later de-Codex work.

## Non-Goals

- Do not rewrite Codex internals during the vendoring pass.
- Do not rename every Codex crate during the vendoring pass.
- Do not remove OpenAI/Codex provider logic yet.
- Do not add TUI, desktop app, cloud task product surfaces, update, doctor,
  completion, marketplace, or release packaging as YunXi user-facing features.
- Do not make the first vendoring pass a full architecture rewrite.

## Recommended Approach

Use a two-layer source ownership model:

```text
YunXi-owned crates
  crates/yunxi-agent-core
  crates/yunxi-agent-cli
  crates/yunxi-agent-codex

Vendored Codex source
  vendor/codex-rs/...
```

The first Stage 3 milestone should copy the required upstream Codex crates into
`vendor/codex-rs` and update YunXi path dependencies from:

```toml
../../external/codex-rs/<crate>
```

to:

```toml
../../vendor/codex-rs/<crate>
```

This keeps the current integration stable while removing the local source-link
dependency.

## Source Scope

The vendored set must include every upstream crate required by
`yunxi-agent-codex --features codex-native`, including transitive local Codex
workspace dependencies.

Minimum expected source groups:

- Agent runtime:
  - `core`
  - `exec`
  - `app-server`
  - `app-server-client`
  - `app-server-protocol`
  - `exec-server`
  - `exec-server-protocol`
  - `protocol`
- Runtime capabilities:
  - `apply-patch`
  - `sandboxing`
  - `windows-sandbox-rs`
  - `shell-command`
  - `tools`
  - `code-mode`
  - `code-mode-protocol`
- Agent context and extensions:
  - `core-skills`
  - `core-plugins`
  - `skills`
  - `plugin`
  - `codex-mcp`
  - `mcp-server`
  - `ext/*` crates used by the headless runtime
  - `context-fragments`
  - `memories/read`
  - `memories/write`
- Config, auth, provider, and API:
  - `config`
  - `login`
  - `codex-api`
  - `codex-client`
  - `model-provider`
  - `model-provider-info`
  - `models-manager`
  - `cloud-config`
  - `http-client`
  - `network-proxy`
  - `aws-auth`
- Session and persistence:
  - `rollout`
  - `rollout-trace`
  - `thread-store`
  - `state`
  - `agent-graph-store`
- Utility crates required by the above:
  - `arg0`
  - `feedback`
  - `features`
  - `file-system`
  - `file-search`
  - `file-watcher`
  - `git-utils`
  - `hooks`
  - `install-context`
  - `otel`
  - `prompts`
  - `response-debug-context`
  - `terminal-detection`
  - `uds`
  - `utils/*`
  - procedural macro crates used by the vendored workspace

The implementation plan should use `cargo metadata` or Cargo error feedback to
derive the exact closure instead of manually guessing every crate.

## Repository Layout

Stage 3 should introduce:

```text
vendor/
  README.md
  codex-rs/
    Cargo.toml
    <required upstream crates>
    patches/
      README.md
```

`vendor/codex-rs` should be committed to Git. `external/` should remain ignored
as a temporary source-link area for comparison and future upstream refreshes.

`vendor/README.md` should explain:

- the upstream source origin,
- the local snapshot date,
- that the first vendored import is intentionally mechanical,
- how future upstream refreshes should be performed.

## Cargo Strategy

Root `Cargo.toml` should continue to own the YunXi workspace. Vendored Codex
crates should not become public YunXi API crates.

Recommended first pass:

- Keep YunXi workspace members limited to YunXi crates.
- Add `exclude = ["external/codex-rs", "vendor/codex-rs"]` if needed to avoid
  Cargo treating the vendored upstream workspace as a nested workspace member.
- Point `yunxi-agent-codex` optional path dependencies at `vendor/codex-rs`.
- Preserve `[patch.crates-io]` entries required by the vendored Codex graph.
- Keep `[profile.dev] debug = 1` and `[profile.test] debug = 1` because the
  Codex dependency graph can produce oversized debug rlibs on Windows.

If path dependency resolution requires the vendored Codex workspace root, keep
the upstream `vendor/codex-rs/Cargo.toml` intact. Do not rewrite upstream crate
manifests in the first pass unless a local path must be corrected.

## Patch Handling

Stage 2 required a local Windows PTY fix in the ignored upstream source:

```rust
self.con.raw_handle() as RawHandle
```

Stage 3 must make that fix reproducible inside the committed vendored source.
Patch handling should be explicit:

- Apply required compile fixes to `vendor/codex-rs`.
- Record each local change in `vendor/codex-rs/patches/README.md`.
- Do not modify ignored `external/codex-rs` as the source of truth.
- Keep local changes minimal and easy to reapply during upstream refreshes.

## De-Codex Plan After Vendoring

After the repository is self-contained, later stages should gradually move
ownership from vendored Codex code into YunXi crates.

Suggested order:

1. Extract YunXi-owned runner boundary from the current `native_exec.rs`.
2. Replace Codex JSONL event types with YunXi-owned event production.
3. Move provider/auth configuration behind a YunXi provider facade.
4. Move shell, patch, MCP, and skills execution behind YunXi tool interfaces.
5. Move session, rollout, and history storage behind YunXi storage interfaces.
6. Delete vendored crates and modules that are not part of the YunXi Agent
   product surface.
7. Rename remaining owned modules from Codex terminology to YunXi terminology.

This order keeps the Agent running while control shifts from vendored internals
to YunXi-owned abstractions.

## Verification

Stage 3 is complete only when these pass from a clean clone without
`external/codex-rs`:

```powershell
cargo fmt -- --check
cargo test
cargo check -p yunxi-agent-codex --features codex-native
cargo check -p yunxi-agent-cli --features codex-native
cargo build -p yunxi-agent-cli --features codex-native
cargo run -p yunxi-agent-cli -- --backend dry-run "explain this project"
cargo run -p yunxi-agent-cli -- --backend dry-run --jsonl "explain this project"
git diff --check
```

Optional live verification remains gated:

```powershell
$env:YUNXI_RUN_LIVE_CODEX_TESTS = "1"
cargo test -p yunxi-agent-codex --features codex-native live_codex_backend_can_complete_simple_prompt_when_enabled -- --nocapture
```

If live credentials are unavailable, record the skip in
`docs/extraction-status.md` and do not treat it as a normal failure.

## Risks And Mitigations

Large repository size:
Vendoring the Codex Rust graph may add many files. Keep the import focused on
Rust source required for the headless Agent runtime. Avoid vendoring build
artifacts, test snapshots, node modules, and unrelated product surfaces.

Accidental product-surface expansion:
The vendored source may include TUI, cloud, desktop, update, or marketplace
code as transitive dependencies. Do not expose those as YunXi commands. Keep
YunXi CLI flags focused on Agent execution.

Upstream refresh difficulty:
Mechanical import first, local patches documented second. Later refreshes
should be able to diff `vendor/codex-rs` against a known upstream commit.

Windows link/build pressure:
Keep low debug info profiles and verify feature builds on Windows before
claiming success.

OpenAI coupling:
Do not try to remove provider coupling in the vendoring pass. After vendoring,
introduce a YunXi provider facade so OpenAI/Codex becomes one provider path
rather than the project identity.

## Acceptance Criteria

- `external/codex-rs` is no longer required to build `codex-native`.
- `vendor/codex-rs` contains the committed source closure needed by YunXi live
  backend.
- The current Stage 2 live backend still compiles against vendored source.
- Normal tests do not require network credentials.
- Live tests remain explicitly gated.
- `docs/extraction-status.md` documents that YunXi is self-contained for the
  current live backend.
- No TUI, desktop, cloud-task, update, doctor, completion, marketplace, or
  release packaging surfaces are added to YunXi CLI.

## Vendoring Decision

Stage 3 should vendor the dependency closure required by `codex-native` as the
default path. This keeps the import focused on the headless Agent backend while
still preserving the runtime capability that YunXi already exposes.

The implementation plan may switch to vendoring the full upstream Rust
workspace only if Cargo path resolution shows that the full workspace is simpler
than curating individual crates. Even then, the import must exclude generated
artifacts, build outputs, node modules, and unrelated non-Rust product assets.

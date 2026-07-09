# YunXi Autonomous Agent Runtime Development Report

## Executive Summary

YunXi Agent has completed Stage 3 source vendoring. The repository can now build
the native Agent backend from committed source under `vendor/codex-rs` without a
developer-local `external/codex-rs` checkout.

The next version must move beyond source self-containment. Its goal is to make
YunXi Agent run from YunXi-owned runtime modules instead of vendored Codex
runtime crates.

Target state:

```text
YunXi Agent runtime
  owns Agent loop, provider facade, tool execution, approvals, sandboxing,
  sessions, events, and CLI behavior

vendor/codex-rs
  remains only as temporary reference material during migration,
  then is removed from the normal build graph
```

This is a larger step than Stage 3. Stage 3 removed the external source checkout
dependency. Stage 4 must remove the runtime dependency on Codex crates.

## Current Baseline

Verified Stage 3 state:

- `vendor/codex-rs` contains a committed Codex Rust workspace snapshot.
- `yunxi-agent-codex` path dependencies point at `vendor/codex-rs`.
- `cargo check -p yunxi-agent-codex --features codex-native` passes.
- `cargo build -p yunxi-agent-cli --features codex-native` passes.
- Normal tests and dry-run CLI smoke pass.
- Live credential smoke remains gated by `YUNXI_RUN_LIVE_CODEX_TESTS=1`.

Current limitation:

- Full live Agent behavior still comes from vendored Codex runtime crates.
- `yunxi-agent-core` owns public facade types, but not the complete runtime.
- Provider/auth, tool execution, sandbox behavior, shell execution, patch
  application, MCP, skills, session storage, rollout handling, and low-level
  event production still depend on Codex internals.

## Next Version Goal

Build a YunXi-owned Agent runtime that can execute independently of
`vendor/codex-rs`.

The next version is complete only when:

- `yunxi-agent-cli --backend yunxi` can run through YunXi-owned runtime modules.
- `yunxi-agent-core` and new YunXi runtime crates do not depend on
  `vendor/codex-rs`.
- `vendor/codex-rs` can be removed from `Cargo.toml` path dependencies without
  breaking the YunXi-owned backend.
- Runtime events are produced as YunXi event types directly, not mapped from
  Codex JSONL output.
- Provider and auth are behind YunXi-owned interfaces.
- Shell, patch, MCP, skills, approvals, sandboxing, and session storage are
  behind YunXi-owned interfaces.
- The legacy Codex backend, if retained temporarily, is clearly marked as a
  compatibility backend and is not the default runtime.

## Recommended Architecture

Introduce a new YunXi-owned runtime layer while keeping the current public
facade stable.

Recommended workspace shape:

```text
crates/
  yunxi-agent-core/
    Public facade: config, input, events, results, backend trait

  yunxi-agent-runtime/
    YunXi-owned Agent loop, turn orchestration, approval flow, event emission

  yunxi-agent-provider/
    Provider registry, auth abstraction, model request/response streaming

  yunxi-agent-tools/
    Shell, patch, file, MCP, skills, and tool approval interfaces

  yunxi-agent-storage/
    Sessions, rollouts, thread state, history, resumable runs

  yunxi-agent-cli/
    CLI over YunXi-owned runtime

  yunxi-agent-codex/
    Temporary compatibility backend around vendored Codex runtime
```

The key design rule is:

```text
Keep YunXi public API stable. Move ownership inward one boundary at a time.
```

## Migration Strategy

Use a strangler migration. Do not delete the vendored Codex runtime before
YunXi-owned replacements exist and are verified.

Stage 4A: Runtime Boundary

- Add `yunxi-agent-runtime`.
- Move the top-level run loop contract out of `yunxi-agent-codex`.
- Define YunXi-owned `AgentTurn`, `RuntimeEventSink`, `RuntimeContext`, and
  `RuntimeBackend` interfaces.
- Keep the existing Codex backend available only as a comparison backend.
- Acceptance: dry-run and compatibility Codex backend both run through the same
  YunXi runtime-facing contract.

Stage 4B: Event Ownership

- Stop treating Codex JSONL as the primary event source.
- Make YunXi event production first-class inside the runtime.
- Keep Codex event mapping only for the temporary compatibility backend.
- Acceptance: YunXi backend emits `AgentEvent` directly from runtime actions.

Stage 4C: Provider Ownership

- Add `yunxi-agent-provider`.
- Define provider-neutral request, response, stream, auth, and model metadata
  types.
- Implement an OpenAI-compatible provider through YunXi interfaces.
- Move provider config away from Codex config structs.
- Acceptance: provider calls no longer require `codex-config`, `codex-client`,
  `codex-api`, or Codex model-provider crates in the YunXi-owned backend.

Stage 4D: Tool Ownership

- Add `yunxi-agent-tools`.
- Define YunXi-owned tool interfaces for shell, patch, file operations, MCP,
  skills, and approval prompts.
- Port shell command execution and patch application behind YunXi interfaces.
- Add explicit permission and sandbox summaries in YunXi types.
- Acceptance: basic shell and patch tool calls work without Codex tool crates.

Stage 4E: Storage Ownership

- Add `yunxi-agent-storage`.
- Define session, rollout, history, and thread state models.
- Implement local file-backed storage first.
- Keep migration/import from Codex storage as a one-time utility, not runtime
  dependency.
- Acceptance: YunXi can start, persist, list, resume, and inspect sessions
  without Codex rollout/thread-store/state crates.

Stage 4F: Default Backend Switch

- Add `--backend yunxi` and make it the default live backend when provider
  credentials are present.
- Keep `--backend codex` only as a temporary compatibility escape hatch.
- Acceptance: normal CLI usage does not load `yunxi-agent-codex`.

Stage 4G: Vendor Detachment

- Remove `vendor/codex-rs` from the normal Cargo dependency graph.
- Remove `codex-native` from default development flow.
- Keep vendored source only on a separate archival branch or under an explicit
  compatibility feature.
- Acceptance: `cargo test`, `cargo check`, and `cargo build -p
  yunxi-agent-cli` pass after deleting or excluding Codex runtime path
  dependencies.

## Capability Matrix

| Capability | Current Source | Target Owner |
| --- | --- | --- |
| Agent run loop | Codex app-server/exec runtime | `yunxi-agent-runtime` |
| Public config/input/result | `yunxi-agent-core` | `yunxi-agent-core` |
| Event model | YunXi facade plus Codex JSONL mapping | YunXi direct events |
| Provider/auth | Codex config/client/provider crates | `yunxi-agent-provider` |
| Shell execution | Codex tools/runtime | `yunxi-agent-tools` |
| Patch application | Codex apply-patch/runtime | `yunxi-agent-tools` |
| Approvals/sandbox | Codex protocol/config mapping | YunXi policy interfaces |
| MCP | Codex MCP runtime | `yunxi-agent-tools` MCP adapter |
| Skills | Codex skills runtime | YunXi skills adapter |
| Sessions/rollouts | Codex state/thread-store/rollout | `yunxi-agent-storage` |
| CLI | YunXi CLI | YunXi CLI |

## Non-Goals For The Next Version

- Do not build a TUI.
- Do not add desktop app features.
- Do not add cloud task product surfaces.
- Do not add marketplace, update, doctor, completion, install, or release
  packaging surfaces.
- Do not rename every file before behavior is owned.
- Do not delete `vendor/codex-rs` before YunXi-owned runtime behavior passes
  verification.
- Do not remove OpenAI-compatible provider support; make it a provider, not the
  project identity.

## Verification Gates

Each stage must pass its own focused verification before moving forward.

Minimum final verification:

```powershell
cargo fmt -- --check
cargo test
cargo check -p yunxi-agent-runtime
cargo check -p yunxi-agent-provider
cargo check -p yunxi-agent-tools
cargo check -p yunxi-agent-storage
cargo check -p yunxi-agent-cli
cargo build -p yunxi-agent-cli
cargo run -p yunxi-agent-cli -- --backend dry-run "explain this project"
cargo run -p yunxi-agent-cli -- --backend yunxi --jsonl "explain this project"
git diff --check
```

Final independence verification:

```powershell
Rename-Item vendor\codex-rs vendor\codex-rs.disabled
cargo test
cargo check -p yunxi-agent-cli
cargo build -p yunxi-agent-cli
Rename-Item vendor\codex-rs.disabled vendor\codex-rs
```

The disabled-vendor check is the hard proof that YunXi-owned runtime no longer
depends on vendored Codex source.

Live provider verification remains gated by explicit credentials and
environment variables.

## Risk Assessment

Provider protocol drift:
Model APIs and streaming formats can change. Use provider-neutral YunXi types
and keep provider adapters small.

Behavior regression:
The Codex runtime currently carries many hidden behaviors. Preserve behavior
through fixture tests and side-by-side comparison against the compatibility
backend.

Over-wide migration:
Trying to replace provider, tools, storage, and session logic in one commit will
make failures hard to debug. Migrate one boundary at a time.

Windows build pressure:
Codex native builds pulled large dependencies such as `v8`. The YunXi-owned
runtime should avoid importing heavy optional surfaces unless the feature needs
them.

Accidental Codex dependency reintroduction:
Add dependency checks that fail when YunXi-owned runtime crates depend on
`vendor/codex-rs`.

## Dependency Policy

YunXi-owned crates may not depend on `vendor/codex-rs`.

Allowed:

- `yunxi-agent-codex` may depend on `vendor/codex-rs` while the compatibility
  backend exists.
- Tests may compare YunXi output against the compatibility backend behind an
  explicit feature.
- Documentation may reference Codex history and vendored source.

Disallowed:

- `yunxi-agent-core` depending on Codex crates.
- `yunxi-agent-runtime` depending on Codex crates.
- `yunxi-agent-provider` depending on Codex provider/config/client crates.
- `yunxi-agent-tools` depending on Codex tool/runtime crates.
- `yunxi-agent-storage` depending on Codex state/thread-store/rollout crates.
- CLI default path loading `yunxi-agent-codex`.

## Deliverables

The next version should produce:

- A design spec for YunXi-owned autonomous runtime.
- An implementation plan split by runtime boundary.
- New YunXi-owned runtime/provider/tools/storage crates.
- Tests proving public facade behavior is stable.
- Tests proving the YunXi backend runs without `vendor/codex-rs`.
- Updated CLI default backend behavior.
- Updated extraction status documenting when Codex runtime is no longer needed.
- A removal or archival plan for `vendor/codex-rs`.

## Recommended Next Action

Write the Stage 4 design spec next:

```text
docs/superpowers/specs/2026-07-10-yunxi-autonomous-runtime-design.md
```

That spec should define the first implementation slice as Stage 4A: a
YunXi-owned runtime boundary and event sink. This keeps the next code change
small enough to verify while still moving directly toward full independence.

# YunXi Stage 4B Full Autonomy Development Report

## Executive Summary

Stage 4A has moved the default CLI path into YunXi-owned runtime code. The
`yunxi` backend now enters `yunxi-agent-runtime`, emits YunXi `AgentEvent`
values directly, and routes through YunXi-owned provider, tools, and storage
interfaces.

Stage 4B must turn that boundary into a real autonomous runtime slice. The
central goal is:

```text
YunXi default build and default CLI runtime must work after vendor/codex-rs is
disabled or removed.
```

This is the point where YunXi stops treating Codex source as a build-time
requirement for normal operation. Codex compatibility can remain as an explicit
side path, but it must not be loaded, resolved, or required by the default
workspace flow.

## Current Baseline

Verified Stage 4A state:

- `yunxi-agent-cli` defaults to `--backend yunxi`.
- `yunxi-agent-runtime` owns the first runtime backend.
- `yunxi-agent-provider` owns provider request/response interfaces.
- `yunxi-agent-tools` owns tool request/response interfaces.
- `yunxi-agent-storage` owns session record and session store interfaces.
- `yunxi-agent-runtime`, `yunxi-agent-provider`, `yunxi-agent-tools`, and
  `yunxi-agent-storage` do not reference `vendor/codex-rs`.
- `yunxi-agent-codex` remains in the workspace as an explicit compatibility
  backend.
- `vendor/codex-rs` remains in the repository as temporary source reference and
  compatibility input.

Current limitation:

- `yunxi-agent-cli` still has a normal dependency edge to
  `yunxi-agent-codex`.
- The workspace still includes the Codex compatibility crate in the normal
  member list.
- The `yunxi` runtime still uses a deterministic static provider rather than a
  real model provider.
- Tool execution is represented by `NoopToolRuntime`; shell, patch, MCP, and
  skills are not implemented in YunXi-owned code yet.
- Session storage is in-memory only.
- Full independence has not yet been proven by a disabled-vendor verification
  gate.

## Stage 4B Goal

Build the first genuinely self-contained YunXi runtime slice.

Stage 4B is complete only when:

- Default `cargo test`, `cargo check`, and `cargo build -p yunxi-agent-cli`
  work after `vendor/codex-rs` is temporarily renamed.
- `cargo tree -p yunxi-agent-cli` for the default feature set contains no
  Codex compatibility crate and no vendored Codex path dependency.
- `yunxi-agent-cli --backend yunxi` can run through YunXi-owned provider,
  runtime, tools, and storage code.
- The Codex compatibility backend is available only behind an explicit
  compatibility feature or separate workspace path.
- Runtime events are produced directly by YunXi runtime actions.
- Sessions are persisted by YunXi storage, not by Codex rollout or thread
  storage.
- Basic shell execution and file-change reporting are owned by
  `yunxi-agent-tools`.

## Architecture Direction

Stage 4B should preserve the Stage 4A public facade and strengthen the internal
ownership boundary.

Target shape:

```text
Default YunXi workspace
  yunxi-agent-core
  yunxi-agent-provider
  yunxi-agent-tools
  yunxi-agent-storage
  yunxi-agent-runtime
  yunxi-agent-cli

Explicit compatibility workspace or feature
  yunxi-agent-codex
  vendor/codex-rs
```

The default CLI dependency graph should become:

```text
yunxi-agent-cli
  -> yunxi-agent-core
  -> yunxi-agent-runtime
       -> yunxi-agent-provider
       -> yunxi-agent-tools
       -> yunxi-agent-storage
```

It should not include:

```text
yunxi-agent-codex
vendor/codex-rs/*
codex-* crates
```

## Implementation Slices

### Stage 4B.1: Default Build Graph Detachment

Purpose:

Remove Codex compatibility from the default dependency graph.

Required changes:

- Make `yunxi-agent-codex` an explicit compatibility crate, not a normal CLI
  dependency.
- Add a CLI feature such as `codex-compat` that enables the Codex backend only
  when requested.
- Ensure `--backend codex` and `--live` return a clear feature-gated error when
  the CLI is built without compatibility support.
- Move `yunxi-agent-codex` out of default workspace members or otherwise ensure
  default workspace commands do not resolve `vendor/codex-rs`.
- Add dependency tests or scripts that fail if default YunXi crates reference
  `vendor/codex-rs` or `codex-*` crates.

Acceptance:

```powershell
Rename-Item vendor\codex-rs vendor\codex-rs.disabled
cargo test
cargo check -p yunxi-agent-cli
cargo build -p yunxi-agent-cli
cargo tree -p yunxi-agent-cli
Rename-Item vendor\codex-rs.disabled vendor\codex-rs
```

The dependency tree must not contain `yunxi-agent-codex`, `vendor/codex-rs`, or
`codex-*` crates for the default feature set.

### Stage 4B.2: Provider Ownership

Purpose:

Replace the static provider with a real YunXi-owned provider adapter.

Required changes:

- Extend `yunxi-agent-provider` with provider config, auth source, model
  request, response, streaming event, and tool-call data types.
- Add an OpenAI-compatible provider adapter through YunXi types, not Codex
  config/client crates.
- Keep `StaticProvider` as a deterministic test provider.
- Add provider tests that use a local mock HTTP server or fixture provider,
  never live credentials by default.
- Route CLI `--model` and `--provider` into YunXi provider config.

Acceptance:

- `yunxi-agent-runtime` can consume a provider response through
  `AgentProvider`.
- Provider errors map to `AgentError` without Codex error types.
- Default tests do not require network credentials.

### Stage 4B.3: Tool Runtime Ownership

Purpose:

Give YunXi the first owned tool execution path.

Required changes:

- Implement a shell tool executor in `yunxi-agent-tools` using Rust standard
  process APIs or `tokio::process`.
- Emit YunXi events for command start, command update, and command completion.
- Add approval and sandbox policy data types owned by YunXi.
- Add file-change detection for changed files inside the configured workspace.
- Keep patch application behind a YunXi trait even if the first implementation
  supports only a constrained patch format.

Acceptance:

- A YunXi runtime test can execute a simple command and receive command events.
- Tool execution does not call Codex exec, Codex protocol, or Codex sandbox
  crates.
- Approval denial returns a YunXi `ToolResponse` with `ToolStatus::Declined`.

### Stage 4B.4: Runtime Loop Ownership

Purpose:

Move from a single provider response to a real agent turn loop.

Required changes:

- Add a YunXi-owned `RuntimeAction` enum for `Respond`, `RunTool`,
  `ApplyPatch`, and `RequestApproval`.
- Let the provider return either assistant text or tool calls.
- Let `yunxi-agent-runtime` execute tool calls through `ToolRuntime`.
- Feed tool results back into the provider until a final response is produced.
- Cap loop iterations with a runtime setting to prevent runaway turns.

Acceptance:

- A deterministic test provider can request a shell tool and then produce a
  final answer using the tool result.
- Runtime events remain YunXi `AgentEvent` values from first action to final
  completion.
- Empty prompt and provider/tool failures produce clear YunXi errors.

### Stage 4B.5: Storage Ownership

Purpose:

Persist useful agent state without Codex rollout or thread storage.

Required changes:

- Add file-backed session storage under a YunXi-owned directory such as
  `.yunxi/sessions`.
- Persist prompt, cwd, events, final response, model/provider, timestamps, and
  status.
- Add CLI flags for listing and resuming YunXi sessions only after the file
  model is stable.
- Keep import from Codex history as a later migration utility, not a runtime
  dependency.

Acceptance:

- A completed YunXi run writes a session record to disk.
- A test can load that record in a fresh store instance.
- Storage tests pass after `vendor/codex-rs` is disabled.

### Stage 4B.6: Compatibility Isolation

Purpose:

Keep Codex available for comparison while preventing accidental runtime
dependency.

Required changes:

- Rename docs and CLI help text to mark Codex as compatibility-only.
- Add a feature-gated compatibility build command to the README.
- Keep compatibility tests out of default `cargo test`.
- Add a report section documenting when the compatibility crate can be removed
  entirely.

Acceptance:

- Default user flow never mentions Codex unless the user asks for compatibility.
- Compatibility build still works when `vendor/codex-rs` is present and the
  compatibility feature is explicitly enabled.

## Capability Target

| Capability | Stage 4A State | Stage 4B Target |
| --- | --- | --- |
| Default CLI backend | `yunxi` | `yunxi` |
| Default CLI dependency on Codex | Still present through CLI dependency | Removed |
| Runtime events | YunXi direct events | YunXi direct events across provider and tools |
| Provider | Static provider | YunXi provider trait plus OpenAI-compatible adapter |
| Shell tool | Noop interface | YunXi-owned shell executor |
| Patch tool | Interface only | YunXi-owned trait and constrained implementation |
| Session storage | In-memory | File-backed YunXi sessions |
| Vendor detachment proof | Not final | Disabled-vendor verification passes |
| Codex backend | Explicit compatibility path | Feature-gated compatibility path |

## Verification Gate

Stage 4B should use a single unified verification pass after implementation is
complete. Do not spend the build phase repeatedly checking one failing point.

Required verification:

```powershell
cargo fmt -- --check
cargo test
cargo check -p yunxi-agent-core
cargo check -p yunxi-agent-provider
cargo check -p yunxi-agent-tools
cargo check -p yunxi-agent-storage
cargo check -p yunxi-agent-runtime
cargo check -p yunxi-agent-cli
cargo build -p yunxi-agent-cli
cargo run -p yunxi-agent-cli -- --backend yunxi --jsonl "explain this project"
git diff --check
```

Independence verification:

```powershell
Rename-Item vendor\codex-rs vendor\codex-rs.disabled
cargo test
cargo check -p yunxi-agent-cli
cargo build -p yunxi-agent-cli
cargo tree -p yunxi-agent-cli
Rename-Item vendor\codex-rs.disabled vendor\codex-rs
```

The disabled-vendor run is mandatory for claiming full Stage 4B independence.

Compatibility verification, only when `vendor/codex-rs` is present:

```powershell
cargo check -p yunxi-agent-cli --features codex-compat
cargo check -p yunxi-agent-codex --features codex-native
```

## Risk Assessment

Cargo path dependency resolution:

Optional path dependencies can still affect metadata or workspace resolution if
they remain in the default workspace. Stage 4B must prove independence with the
disabled-vendor check, not by reading manifests by eye.

Provider scope creep:

The first provider adapter should support one clean request/response path and
test fixtures. Streaming, advanced tool-call formats, and multiple vendors can
grow after the boundary is stable.

Tool safety:

Shell and patch execution must use YunXi-owned approval and sandbox summaries.
Do not silently inherit Codex policy names or behavior.

Storage compatibility:

YunXi sessions should be simple and self-owned first. Codex history import can
be built later as a migration command.

Build size:

Do not reintroduce heavy Codex-native dependencies into default builds. Stage
4B should keep normal verification light enough to run often.

## Non-Goals

- No TUI.
- No desktop app surface.
- No cloud task product surface.
- No marketplace, update, doctor, completion, or release installer work.
- No live credential tests in default verification.
- No deletion of compatibility source before disabled-vendor default flow
  passes.

## Recommended Next Action

Start Stage 4B with build graph detachment.

The first code change should make `yunxi-agent-cli` depend on
`yunxi-agent-codex` only behind an explicit compatibility feature. After that,
run the disabled-vendor verification gate. This gives a hard proof that the
normal YunXi CLI is no longer bound to upstream Codex source before expanding
provider, tool, and storage behavior.

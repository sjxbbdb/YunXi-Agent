# YunXi Stage 4C Provider And Policy Runtime Report

## Executive Summary

Stage 4B proved the most important independence boundary:

```text
Default YunXi CLI and runtime work after vendor/codex-rs is disabled.
```

That means the next stage should stop spending energy on build graph
detachment and start turning the independent runtime into a useful autonomous
agent. Stage 4C should add real provider execution, explicit approval and
sandbox policy, constrained patch support, and first-class session commands.

The goal is not to rebuild every Codex surface. The goal is to make the YunXi
default path useful, inspectable, and safe without importing Codex runtime
internals.

## Current Baseline

Verified Stage 4B state:

- `yunxi-agent-cli` does not depend on `yunxi-agent-codex`.
- Default `cargo tree -p yunxi-agent-cli` contains no `yunxi-agent-codex`,
  `vendor/codex-rs`, or `codex-*` crates.
- Default verification passes after temporarily renaming `vendor/codex-rs`.
- `yunxi-agent-runtime` owns a provider/tool loop.
- `yunxi-agent-provider` can represent assistant responses and tool calls.
- `yunxi-agent-tools` owns shell execution and file-change reporting.
- `yunxi-agent-storage` owns file-backed session records under
  `.yunxi/sessions`.
- `yunxi-agent-codex` remains only as a detached compatibility crate.

Remaining limitation:

- The default provider is deterministic and does not call a real model.
- Shell execution has no explicit approval or sandbox policy gate.
- Patch, MCP, and skills are still interface-level.
- Session records exist, but the CLI cannot list, inspect, or resume them.
- Runtime events are direct YunXi events, but the provider/tool loop is still a
  minimal loop rather than a full autonomous planning loop.

## Stage 4C Goal

Build the first practical YunXi-owned agent runtime.

Stage 4C is complete only when:

- `yunxi-agent-provider` has an OpenAI-compatible adapter implemented through
  YunXi-owned request/response types.
- The adapter can be tested with local fixtures or a mock HTTP server without
  live credentials.
- `yunxi-agent-tools` enforces YunXi-owned approval and sandbox policy before
  shell execution.
- `yunxi-agent-tools` supports constrained patch application without Codex
  apply-patch crates.
- `yunxi-agent-runtime` emits direct YunXi events for provider, approval, shell,
  patch, file-change, and final-response flow.
- `yunxi-agent-cli` can list and inspect YunXi-owned session records.
- Default verification still passes with `vendor/codex-rs` disabled.

## Architecture Direction

Stage 4C should keep the Stage 4B dependency graph intact:

```text
yunxi-agent-cli
  -> yunxi-agent-core
  -> yunxi-agent-runtime
       -> yunxi-agent-provider
       -> yunxi-agent-tools
       -> yunxi-agent-storage
```

Still forbidden in the default graph:

```text
yunxi-agent-codex
vendor/codex-rs/*
codex-* crates
```

Add functionality behind YunXi-owned interfaces, not through compatibility
imports.

## Implementation Slices

### Stage 4C.1: Provider Adapter

Purpose:

Replace deterministic-only runtime behavior with a real provider path while
keeping deterministic tests.

Required changes:

- Add provider config types to `yunxi-agent-provider`.
- Add an auth source type that can read an API key from environment variables.
- Add an OpenAI-compatible request serializer using YunXi `ProviderMessage` and
  `ProviderToolCall` types.
- Add an OpenAI-compatible response parser that maps assistant text and tool
  calls back into `ProviderResponse`.
- Keep `StaticProvider` as the default test provider unless credentials and
  explicit CLI selection are present.
- Add tests using fixture JSON responses rather than live credentials.

Acceptance:

- Fixture tests parse assistant text responses.
- Fixture tests parse tool-call responses.
- Provider errors map to `AgentError::Execution`.
- No Codex provider, config, protocol, or client crate appears in the default
  dependency graph.

### Stage 4C.2: Approval And Sandbox Policy

Purpose:

Make tool execution controlled by YunXi-owned policy instead of implicit shell
execution.

Required changes:

- Add `ToolPolicy`, `ApprovalDecision`, and `SandboxPolicy` types to
  `yunxi-agent-tools`.
- Map `AgentConfig.approval_mode` and `AgentConfig.sandbox_mode` into tool
  policy.
- Decline shell execution when policy requires approval and no approval source
  has approved it.
- Restrict shell execution to the configured workspace unless the sandbox mode
  explicitly allows broader execution.
- Emit YunXi events for declined or blocked tool execution.

Acceptance:

- Tests show `ApprovalMode::Never` allows configured non-interactive shell
  execution.
- Tests show approval-required commands are declined by default.
- Tests show workspace policy rejects commands outside the configured cwd.
- No Codex sandbox or approval crates are used.

### Stage 4C.3: Constrained Patch Tool

Purpose:

Add a first YunXi-owned patch path without importing Codex patch helpers.

Required changes:

- Add a constrained patch request type for create/update/delete file changes.
- Implement patch application using Rust filesystem APIs.
- Reject absolute paths and parent-directory traversal.
- Emit `PatchCompleted` and `FileChanged` events from YunXi runtime.
- Keep broad diff parsing out of scope for this slice.

Acceptance:

- Tests create a file through the patch tool.
- Tests update a file through the patch tool.
- Tests delete a file through the patch tool.
- Tests reject `..` traversal and absolute paths.
- Runtime can execute a provider-requested patch tool call.

### Stage 4C.4: Session CLI

Purpose:

Make YunXi-owned session storage visible to the user.

Required changes:

- Add CLI subcommands or flags for session listing and inspection.
- Keep the normal prompt path unchanged.
- List session id, status, cwd, created time, and prompt preview.
- Inspect a session by id and print JSON or plain text.
- Keep resume as a later slice unless the load/append model is stable.

Acceptance:

- A YunXi run writes a session record.
- `yunxi-agent-cli sessions list --cwd <workspace>` displays that record.
- `yunxi-agent-cli sessions show <id> --cwd <workspace> --json` prints the
  stored events and final response.

### Stage 4C.5: Runtime Event Tightening

Purpose:

Make event output useful enough to debug agent behavior.

Required changes:

- Emit provider-started and provider-completed reasoning events or structured
  events if the core event model is expanded.
- Emit approval decisions as warnings or dedicated events.
- Emit tool result summaries without hiding command failures.
- Persist all emitted events to file-backed sessions.

Acceptance:

- JSONL output shows started, provider/tool activity, file changes, message, and
  completed events for a tool-using run.
- Failed tool execution produces a failed command event and a clear final error
  or recovery response.

## Capability Target

| Capability | Stage 4B State | Stage 4C Target |
| --- | --- | --- |
| Default independence | Proven | Preserved |
| Provider | Static provider plus tool-call types | OpenAI-compatible adapter plus fixtures |
| Shell tool | Executes and reports file changes | Approval and sandbox gated |
| Patch tool | Interface only | Constrained YunXi-owned implementation |
| Session storage | File records | CLI list and inspect |
| Runtime loop | Provider/tool loop | Provider/tool/policy/patch loop |
| Codex compatibility | Detached crate | Still detached |

## Verification Gate

Stage 4C should follow the same rule as the previous stage: build first, then
run one unified verification pass.

Required default verification:

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
cargo run -p yunxi-agent-cli -- --cwd <temp> --backend yunxi --jsonl "explain this project"
cargo tree -p yunxi-agent-cli
git diff --check
```

Required independence verification:

```powershell
Rename-Item vendor\codex-rs vendor\codex-rs.disabled
cargo test
cargo check -p yunxi-agent-cli
cargo build -p yunxi-agent-cli
cargo tree -p yunxi-agent-cli
Rename-Item vendor\codex-rs.disabled vendor\codex-rs
```

Required dependency assertion:

```text
Default cargo tree must not contain yunxi-agent-codex, vendor/codex-rs, or
codex-* crates.
```

Live provider verification should stay opt-in and credential-gated.

## Risk Assessment

Provider drift:

OpenAI-compatible APIs can change. Keep provider serialization and parsing
small, fixture-backed, and isolated from runtime orchestration.

Tool safety:

Shell and patch tools can modify user files. Stage 4C must add explicit policy
types and tests before broadening tool capability.

Patch complexity:

Full unified-diff parsing can grow quickly. Start with constrained file-change
operations and add richer patch formats later.

Session format churn:

File-backed sessions should remain simple JSON records until list/inspect flows
are stable. Add migrations only when the format has real consumers.

Default dependency regression:

Every Stage 4C addition must keep the disabled-vendor verification passing.

## Non-Goals

- No TUI.
- No desktop app surface.
- No cloud task product surface.
- No marketplace, update, doctor, completion, or release installer work.
- No live credential tests in default verification.
- No full MCP or skills implementation in this slice.
- No reintroduction of `yunxi-agent-codex` into the default CLI dependency
  graph.

## Recommended Next Action

Start Stage 4C with provider fixtures and policy types:

- implement provider request/response fixture parsing first
- add tool approval and sandbox policy types next
- then wire constrained patch support into the existing runtime loop
- finish with session list/show CLI commands

This keeps the default YunXi runtime independent while making it increasingly
usable as a real agent.

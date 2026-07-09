# YunXi Headless Agent Stack Extraction Design

## Goal

Second-stage extraction should bring the full Codex CLI headless Agent capability into `D:\YunXi Agent`.

This stage is not a thin process wrapper around `codex exec`. The target is a source-level extraction that preserves the practical Agent behavior behind Codex CLI's non-interactive path while keeping YunXi's public API independent from upstream Codex internal types.

The result should let `yunxi-agent-cli` run real Agent tasks, not only dry-runs:

- load Codex-compatible config and auth
- call configured model providers
- start and complete thread/turn execution
- execute shell commands through Codex's safety path
- apply patches and report file changes
- support approval, sandbox, and permission profiles
- load AGENTS.md instructions
- support MCP and skills as part of the Agent runtime
- preserve session, rollout, and history restore behavior where Codex CLI supports it
- emit structured events for messages, reasoning, commands, patches, MCP, todos, errors, and completion

## Source And Target

Source:

- The user-provided Codex CLI checkout.
- A local checkout has been verified on this machine during design analysis.
- Implementation must keep the source checkout path configurable so the YunXi repository remains portable and does not depend on a machine-specific path.

Target:

- `D:\YunXi Agent`
- Current first-stage workspace on `master` with `yunxi-agent-core` and `yunxi-agent-cli`.

## Source Findings

The Codex CLI headless execution path is centered on `codex-rs/exec`.

Important observed upstream boundaries:

- `codex-rs/exec/src/main.rs` parses the `codex-exec` CLI and calls `codex_exec::run_main`.
- `codex-rs/exec/src/lib.rs` loads config/auth, starts an in-process app-server client, starts or resumes a thread, starts a turn, handles server requests, and shuts down when the turn completes.
- `run_exec_session` uses `InProcessAppServerClient`, `ThreadStart`, `ThreadResume`, `TurnStart`, and `ReviewStart` requests.
- `event_processor_with_jsonl_output.rs` maps app-server notifications into JSONL `ThreadEvent` values.
- `exec_events.rs` defines the stable headless event shape for thread, turn, command execution, file change, MCP tool call, collab tool call, web search, todo list, errors, and usage.
- `SharedCliOptions` maps CLI flags for model, OSS provider, sandbox, dangerous bypass, cwd, images, and additional writable directories.

These boundaries make `codex-exec` the best behavioral reference, but the second-stage implementation should integrate the source-level runtime rather than shelling out to the binary as the normal path.

## Non-Goals

This stage should preserve full headless Agent capability, but it should still avoid outer Codex product surfaces.

Out of scope:

- TUI screens and terminal app UI
- Desktop app UI
- Cloud tasks as a user-facing product
- SDK packaging
- update, doctor, completion, marketplace, and install utilities
- Bazel and release packaging machinery
- Rebranding every upstream internal type in the first pass

Using upstream app-server crates internally is allowed when required by the headless Agent runtime. Exposing app-server product surfaces through YunXi CLI is not part of this stage.

## Architecture

The second stage should introduce a clear separation between YunXi facade types and Codex runtime integration.

Recommended workspace shape:

```text
D:\YunXi Agent
  crates/
    yunxi-agent-core/
      src/
        backend.rs
        codex_events.rs
        live_runner.rs
    yunxi-agent-codex/
      src/
        lib.rs
        config_mapper.rs
        event_mapper.rs
        native_exec.rs
        source_runtime.rs
    yunxi-agent-cli/
      src/
        main.rs
```

`yunxi-agent-core` owns public API:

- `AgentConfig`
- `AgentInput`
- `AgentEvent`
- `AgentRunResult`
- `AgentRunStatus`
- `AgentError`
- `AgentBackend`
- `Agent`

`yunxi-agent-codex` owns Codex coupling:

- path dependencies to upstream Codex crates during the first source-level pass
- config/auth/provider mapping
- permission and sandbox mapping
- app-server client setup
- thread/turn orchestration
- Codex event conversion
- compatibility tests against upstream `codex-exec` JSONL behavior

`yunxi-agent-cli` remains thin:

- parse YunXi CLI arguments
- choose dry-run or live backend
- render human output or JSON output
- return non-zero exit codes for failed runs

## Backend Model

`yunxi-agent-core` should expose a backend abstraction so dry-run and live execution coexist.

Conceptual interface:

```rust
#[async_trait::async_trait]
pub trait AgentBackend: Send + Sync {
    async fn run(&self, config: AgentConfig, input: AgentInput) -> AgentResult<AgentRunResult>;
}
```

The exact trait implementation can avoid `async_trait` if the implementation prefers boxed futures, but the public concept must remain backend-based.

Backends:

- `DryRunBackend`: existing deterministic no-network path for tests and smoke checks.
- `CodexNativeBackend`: source-level Codex runtime integration.
- `CodexExecProcessBackend`: optional diagnostic fallback only; not the primary implementation.

`Agent::run_dry` can remain for compatibility. A new live path such as `Agent::run` or `Agent::run_live` should use a selected backend.

## Source-Level Extraction Strategy

The implementation should proceed in two passes.

### Pass 1: Path Dependency Integration

Use upstream Codex crates via path dependencies from the user-provided checkout to prove that YunXi can drive the full headless Agent runtime.

Likely upstream crates:

- `codex-exec`
- `codex-core`
- `codex-protocol`
- `codex-config`
- `codex-login`
- `codex-app-server-client`
- `codex-app-server-protocol`
- `codex-model-provider-info`
- `codex-utils-cli`
- `codex-arg0`
- sandboxing, shell, apply-patch, MCP, skills, and state crates required by compilation

The implementation should not expose these as public YunXi types.

### Pass 2: Owned Extraction Boundary

After behavior is verified, start moving the required upstream crates or modules into the YunXi repository under an owned namespace.

The first owned extraction targets should be the smallest surfaces that reduce coupling without breaking behavior:

- event model and event mapper
- config and permission mapping
- headless runner orchestration
- tests and fixtures needed to keep behavior stable

Large internal crates can remain path dependencies until their ownership boundary is understood.

## Agent Capability Requirements

Second-stage live execution must preserve these Codex CLI Agent capabilities.

### Model And Auth

- Load Codex-compatible configuration from `codex_home` or default Codex home.
- Support `CODEX_API_KEY` and existing Codex auth flows supported by the reused crates.
- Support model override from `AgentConfig.model`.
- Support provider override from `AgentConfig.provider` where upstream supports it.
- Return actionable errors for missing auth, invalid model, or unsupported provider.

### Thread And Turn

- Start a new headless thread for a prompt.
- Start a turn with text input.
- Preserve ephemeral mode for automation-safe runs.
- Surface thread id and turn lifecycle events in the event stream.
- Keep resume/history support inside the design, even if the first live task exposes only new-thread execution.

### Shell, Patch, And Files

- Preserve Codex shell command execution behavior.
- Preserve Codex apply_patch behavior.
- Surface command start, command completion, exit code, aggregated output, and file changes as YunXi events.
- Preserve sandbox and approval decisions before tool execution.

### Approval, Sandbox, And Permissions

- Map YunXi approval modes to Codex approval policy.
- Map YunXi sandbox modes to Codex sandbox or permission profile.
- Support dangerous bypass only when explicitly configured.
- Preserve workspace-write, read-only, and danger-full-access behavior.
- Return explicit errors when a requested sandbox is unsupported on the current OS.

### Instructions, MCP, Skills, And History

- Load project instructions such as `AGENTS.md` through the Codex runtime path.
- Preserve MCP server configuration and tool call events where the upstream runtime supports them.
- Preserve skills discovery and execution boundaries where the upstream runtime supports them.
- Preserve rollout/session state used for history and resume.
- Expose resume in YunXi only after new-thread live execution is verified.

## Event Mapping

YunXi should keep a stable event API even if upstream event shapes change.

Required event coverage:

- thread started
- turn started
- assistant message
- reasoning summary
- command started
- command updated or output appended
- command completed with exit code and status
- file change with add, update, delete
- patch completed or failed
- MCP tool started/completed/failed
- todo list started/updated/completed
- warning
- error
- turn completed
- turn failed

The existing `AgentEvent` enum can be expanded, or a nested `AgentEventKind` model can be introduced if that keeps compatibility cleaner.

JSON output from YunXi CLI should be valid JSON or JSONL depending on the selected mode:

- `--json`: final `AgentRunResult` as JSON for non-streaming automation
- `--jsonl`: event stream as JSONL for live automation

## CLI Design

The CLI should remain focused on running Agent tasks.

Recommended command shape:

```powershell
cargo run -p yunxi-agent-cli -- --live "fix the failing test"
cargo run -p yunxi-agent-cli -- --backend codex --cwd "D:\repo" --model gpt-5 "summarize this repository"
cargo run -p yunxi-agent-cli -- --backend dry-run "explain this project"
cargo run -p yunxi-agent-cli -- --backend codex --jsonl "inspect this workspace"
```

Initial flags:

- `--backend dry-run|codex`
- `--live` as an alias for `--backend codex`
- `--cwd <PATH>`
- `--model <MODEL>`
- `--provider <PROVIDER>`
- `--codex-home <PATH>`
- `--approval never|on-request|on-failure|untrusted`
- `--sandbox read-only|workspace-write|danger-full-access`
- `--json`
- `--jsonl`

The default backend should remain `dry-run` until live execution is stable in tests. After live execution is verified, the default can be revisited.

## Testing Strategy

Testing must avoid requiring real credentials for normal CI and local verification.

Required tests:

- config mapping tests for approval and sandbox modes
- CLI parsing tests for backend selection and live flags
- event mapping tests using captured upstream JSONL/event fixtures
- mocked model-provider tests using upstream Codex test support or local fixtures
- command/file-change mapping tests
- error mapping tests for missing auth, failed turn, and malformed event payload

Real network/model tests should be optional and gated by environment variables.

Example gating:

```powershell
$env:YUNXI_RUN_LIVE_CODEX_TESTS = "1"
$env:CODEX_API_KEY = "<key>"
cargo test --test live_codex_smoke
```

Normal verification remains:

```powershell
cargo fmt -- --check
cargo test
```

## Migration Plan

The implementation plan should split this stage into reviewable milestones:

1. Add backend abstraction and keep dry-run behavior unchanged.
2. Add expanded event model and mapping tests.
3. Add `yunxi-agent-codex` crate with path dependency wiring.
4. Implement config, approval, sandbox, and cwd mapping.
5. Implement native Codex live new-thread execution.
6. Add CLI live backend flags and JSONL output.
7. Add shell, patch, MCP, todo, and error event mapping coverage.
8. Add optional live smoke test gated by environment variables.
9. Document remaining dependency ownership and extraction boundary.

Each milestone should compile and have focused tests before moving on.

## Error Handling

Errors should be explicit at the YunXi boundary.

Required error categories:

- missing or invalid Codex source checkout
- missing upstream path dependency target
- missing working directory
- missing auth
- invalid config
- unsupported provider or model
- unsupported sandbox on current platform
- live execution start failure
- model stream failure
- command denied
- patch failed
- malformed upstream event
- turn failed

The CLI should print actionable error messages and return non-zero exit codes.

## Documentation

Update documentation during implementation:

- `README.md`: live backend usage and required auth/config
- `docs/extraction-status.md`: which headless capabilities are live, path-dependent, or still owned by upstream
- new developer notes for the upstream Codex path dependency boundary

Do not record secrets or token values in documentation.

## Risks

The main risk is dependency volume. Full headless Agent capability touches many Codex crates. The mitigation is to isolate all Codex coupling in `yunxi-agent-codex` and keep the public facade stable.

Another risk is upstream churn. The event mapper and config mapper should be tested with local fixtures so a future Codex update fails loudly.

Windows support is a practical risk because sandbox behavior differs by OS. The implementation should preserve Codex's own platform behavior and document unsupported sandbox combinations.

Network and credential availability are runtime risks. Normal tests must not require live credentials.

## Acceptance Criteria

Second-stage extraction is complete when:

- `yunxi-agent-core` exposes a backend-based live execution API.
- `yunxi-agent-cli` can run a real Codex-backed Agent task through the live backend.
- The live backend uses source-level Codex integration, not ordinary process shell-out, as its primary path.
- The dry-run backend still works.
- Model/config/auth loading follows Codex behavior.
- Approval and sandbox settings map from YunXi config to Codex runtime behavior.
- Shell command, patch/file change, MCP, todo, message, reasoning, error, turn, and completion events map into YunXi events.
- Normal `cargo test` passes without network credentials.
- Optional live smoke tests can run when credentials are provided.
- Documentation clearly states which dependencies still point to the user-provided Codex source checkout and which parts have been moved into YunXi-owned code.

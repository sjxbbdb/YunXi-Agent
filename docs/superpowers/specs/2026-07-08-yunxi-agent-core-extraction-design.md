# YunXi Agent Core Extraction Design

## Goal

Build an independent, runnable Rust project in `D:\YunXi Agent` by extracting the core agent capabilities from the user-provided Codex CLI source checkout.

The first result should be a slim Agent CLI plus a reusable core library. It should preserve the practical Codex CLI agent loop while removing UI, desktop, cloud task, SDK packaging, and other outer product surfaces.

The extracted project must be initialized as its own Git repository so it can be pushed to GitHub later. After the source is in place, it should also have a CodeGraph index at the project root so future development can use indexed code navigation.

## Source And Target

Source repository:

- The Codex CLI source checkout previously identified by the user.
- The implementation plan must resolve and verify the exact local path before copying or depending on files.

Target project:

- `D:\YunXi Agent`

The target project is writable in the current workspace and will become the independent development location for this work.

## Non-Goals

The first extraction will not preserve the full Codex product surface.

Out of scope for the first implementation:

- Full TUI experience from `codex-rs/tui`
- Desktop app integration
- Full app-server as a user-facing product surface
- Cloud tasks
- npm package publishing wrapper
- Python and TypeScript SDK packaging
- Update, doctor, completion, marketplace, and other CLI utility commands
- Full feature parity with every experimental Codex extension

These can be added later after the extracted core is buildable and easier to reason about.

## Capabilities To Preserve

The extracted project should preserve the core Codex agent behavior:

- Accept a user task from a command line prompt
- Build and run an agent turn
- Call a configured model provider
- Stream agent events or print a final response
- Execute local shell commands through the existing safety path where practical
- Read and write workspace files
- Apply patches
- Support basic approval and sandbox settings
- Keep protocol/event types explicit and serializable
- Provide a reusable Rust API so future callers are not forced through the CLI

The first implementation should target the practical "B" scope chosen during design: core task execution plus permissions and sandboxing. MCP, skills, and history restoration should be kept as future-compatible boundaries but do not need to be fully enabled in the initial runnable CLI unless they fall out naturally from reused crates.

## Proposed Project Shape

```text
D:\YunXi Agent
  Cargo.toml
  README.md
  AGENTS.md
  .gitignore
  docs/
    superpowers/
      specs/
        2026-07-08-yunxi-agent-core-extraction-design.md
  crates/
    yunxi-agent-core/
      Cargo.toml
      src/
        lib.rs
    yunxi-agent-cli/
      Cargo.toml
      src/
        main.rs
```

After implementation:

```text
D:\YunXi Agent
  .git/
  .codegraph/
```

## Architecture

`yunxi-agent-core` owns the reusable agent API. It should hide Codex's internal setup details behind a small facade so downstream users can start a task without knowing the original monorepo layout.

Initial public concepts:

- `AgentConfig`: model, provider, approval mode, sandbox mode, working directory, and optional Codex home.
- `Agent`: a configured runner.
- `AgentInput`: prompt text plus optional future attachments.
- `AgentEvent`: normalized event stream for command execution, file changes, agent messages, errors, and completion.
- `AgentRunResult`: final response, status, and collected events for non-streaming use.

`yunxi-agent-cli` is a thin binary over `yunxi-agent-core`.

Initial CLI shape:

```powershell
cargo run -p yunxi-agent-cli -- "explain this project"
cargo run -p yunxi-agent-cli -- --cwd D:\some\repo "fix the failing test"
```

The CLI should avoid recreating the original `codex` multitool. It should expose only the commands needed to run a task and test the extracted core.

## Extraction Strategy

Prefer reusing Codex's existing crate boundaries over copying disconnected source files. The implementation should start from the minimum set of crates required to compile a runnable agent path, then expand only when the compiler or runtime behavior proves a dependency is necessary.

Likely source crates to evaluate first:

- `codex-rs/core`
- `codex-rs/protocol`
- `codex-rs/config`
- `codex-rs/login`
- `codex-rs/model-provider`
- `codex-rs/model-provider-info`
- `codex-rs/exec`
- `codex-rs/sandboxing`
- `codex-rs/app-server-client` only if it is the cleanest existing runner boundary
- `codex-rs/utils/*` crates required by the selected path

Avoid bringing in:

- `codex-rs/tui`
- `codex-rs/cli` as a whole
- `codex-rs/app-server` unless the implementation proves it is the lowest-risk way to preserve the agent loop
- SDK, docs, Bazel-only release machinery, and third-party release packaging

If a clean direct `codex-core` path is too entangled, the fallback is to create the facade around the smallest existing non-interactive path, then reduce dependencies in later passes.

## Data Flow

1. CLI parses prompt, cwd, model/provider overrides, approval mode, and sandbox mode.
2. CLI builds `AgentConfig`.
3. `yunxi-agent-core` loads or constructs the required Codex configuration.
4. `Agent` starts a task turn in the selected working directory.
5. Core forwards model and tool events as `AgentEvent`.
6. CLI renders events in a simple human-readable format.
7. Completion returns final response and exit status.

## Error Handling

Errors should be explicit at the facade boundary:

- Configuration errors: missing model/provider/auth/config.
- Authentication errors: missing or invalid credentials.
- Runtime errors: model stream failures, unsupported provider, interrupted turn.
- Tool errors: shell command failure, patch failure, file access denial.
- Sandbox or approval errors: denied action or unsupported sandbox mode on the current OS.

The CLI should print actionable errors and return non-zero exit codes. The library should return typed errors where practical, or `anyhow::Error` with stable context if the underlying Codex crates are not yet cleanly typed.

## Testing

First-stage verification should be realistic but scoped:

- `cargo check` for the whole extracted workspace.
- Unit tests for facade configuration mapping.
- A dry-run or mocked agent execution path if the original Codex test helpers can be extracted cheaply.
- A smoke test that the CLI parses arguments and reaches configuration loading without panicking.

Network/model integration tests should not be required for the initial commit because they depend on credentials and network access.

## Git Repository Plan

Initialize `D:\YunXi Agent` as an independent Git repository.

Initial commits should be small and meaningful:

1. Design/spec commit.
2. Project scaffold commit.
3. Extracted core compile pass commit.
4. CLI smoke-run commit.
5. CodeGraph index creation can remain uncommitted if the index is large or machine-specific.

The repository should include a `.gitignore` that excludes Rust build output and machine-specific generated files. Whether `.codegraph/` is committed should be decided after seeing its size and contents; by default, treat it as local developer metadata unless the user wants it tracked.

## CodeGraph Index Plan

After the extracted source compiles or at least reaches a stable scaffold, run CodeGraph indexing in `D:\YunXi Agent` so the project root contains `.codegraph/`.

Future work in this repository should then follow the user's AGENTS.md instruction: if `.codegraph/` exists, use CodeGraph before grep/find when locating or understanding code.

## Risks

The main risk is that `codex-core` may rely on configuration, state, app-server, or UI-adjacent assumptions that make a small extraction larger than expected. The mitigation is to first preserve behavior with a slightly wider dependency set, then shrink the boundary after the project builds.

Another risk is dependency volume. Codex CLI has many crates and third-party dependencies. The first implementation should optimize for a correct runnable core, not for an immediately tiny dependency graph.

## Acceptance Criteria

The first successful implementation is complete when:

- `D:\YunXi Agent` is an independent Git repository.
- The repository contains a Rust workspace with `yunxi-agent-core` and `yunxi-agent-cli`.
- The project compiles or reaches a documented compile boundary with clear remaining blockers.
- The CLI has a minimal task-running command shape.
- Core API boundaries are documented in `README.md`.
- A `.codegraph/` index exists at the project root, unless CodeGraph tooling is unavailable.
- The work is committed locally and ready for the user to add a GitHub remote.

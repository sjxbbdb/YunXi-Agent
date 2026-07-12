# YunXi Agent v1.3 Terminal Streaming Approval Cancellation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build YunXi Agent v1.3.0 as a real terminal Agent with streaming runtime events, interactive approval/user-input responses, and cancellation that reaches running tools.

**Architecture:** Add a YunXi-owned streaming control boundary in `yunxi-agent-core`, wire runtime event sinks to both store and stream events, and let CLI interactive consume events live. Keep the old batch `run` API as a compatibility collector over the new streaming path so one-shot, JSONL, storage, and tests keep their surface.

**Tech Stack:** Rust 2024, Tokio `mpsc`/`oneshot`, existing YunXi provider/runtime/tools/exec crates, PowerShell smoke scripts, GitHub Git Data REST API.

## Global Constraints

- Build first, verify only after the full v1.3 slice is wired.
- Do not run tests, cargo check, cargo fmt, cargo build, live model requests, or smoke scripts during construction.
- Preserve existing autonomous YunXi runtime and default CLI independence from `vendor/codex-rs`, `codex-*`, and `yunxi-agent-codex`.
- Do not print, log, commit, or upload API keys or GitHub tokens.
- Publish GitHub refs only through REST API; do not use `git push`, `git fetch`, or `git ls-remote`.
- Create new annotated tag `v1.3.0`; preserve `v1.0.0`, `v1.1.0`, `v1.2.0`, and `v1.2.1` unchanged.
- Update `C:\Users\admin\Desktop\YunXi Agent开发日志.md` at task completion.
- Run `cargo clean` after release publication and confirm `target_exists=False`.

---

### Task 1: Streaming Core Boundary

**Files:**
- Modify: `crates/yunxi-agent-core/src/backend.rs`
- Modify: `crates/yunxi-agent-core/src/lib.rs`
- Create: `crates/yunxi-agent-core/src/stream.rs`
- Modify: `crates/yunxi-agent-core/src/event.rs`

**Interfaces:**
- Produces: `AgentRunControl`, `AgentRunEventSender`, `AgentRunApprovalRequest`, `AgentRunApprovalDecision`, `AgentRunUserInputRequest`, `AgentRunUserInputResponse`, `AgentRunStreamReceiver`.
- Produces: `AgentBackend::run_stream(&self, config: AgentConfig, input: AgentInput, control: AgentRunControl) -> AgentResult<AgentRunResult>`.
- Preserves: `AgentBackend::run(&self, config, input) -> AgentResult<AgentRunResult>`.

- [ ] Add stream/control types backed by Tokio channels.
- [ ] Add approval and user-input request/response structs using existing event-friendly primitive fields.
- [ ] Add `AgentBackend::run_stream` default compatibility method.
- [ ] Keep `AgentBackend::run` source-compatible for existing callers.
- [ ] Export the new types from `yunxi-agent-core`.

### Task 2: Runtime Event Sink Streaming

**Files:**
- Modify: `crates/yunxi-agent-runtime/src/lib.rs`

**Interfaces:**
- Consumes: `AgentRunControl`, `AgentRunEventSender`.
- Produces: runtime event sinks that store events and stream them immediately.

- [ ] Extend `RuntimeEventSink` implementation to optionally publish every emitted `AgentEvent`.
- [ ] Change `YunXiRuntimeBackend::run_turn` internals to accept optional `AgentRunControl`.
- [ ] Implement `RuntimeBackend::run_turn_stream` or equivalent helper used by `AgentBackend::run_stream`.
- [ ] Keep batch `run_turn` collecting all events and returning the same final `AgentRunResult`.
- [ ] Preserve existing storage/session behavior for completed turns.

### Task 3: Interactive Approval and User Input

**Files:**
- Modify: `crates/yunxi-agent-runtime/src/lib.rs`
- Modify: `crates/yunxi-agent-runtime/src/session_driver.rs`
- Modify: `crates/yunxi-agent-tools/src/lib.rs`
- Modify: `crates/yunxi-agent-cli/src/interactive.rs`
- Modify: `crates/yunxi-agent-cli/src/render.rs`

**Interfaces:**
- Consumes: `AgentRunApprovalRequest`, `AgentRunApprovalDecision`, `AgentRunUserInputRequest`, `AgentRunUserInputResponse`.
- Produces: CLI prompt-driven approve/deny and request_user_input response flow.

- [ ] Route approval-required tool decisions through the control approval channel when an interactive host is present.
- [ ] Keep non-interactive behavior deterministic and safe.
- [ ] Route `request_user_input` through the control user-input channel in interactive mode.
- [ ] Prompt in CLI with clear `y/N` approval text and free-form user input where needed.
- [ ] Emit matching requested/completed events so JSONL and batch output retain observability.

### Task 4: Cancellation Propagation

**Files:**
- Modify: `crates/yunxi-agent-core/src/cancellation.rs`
- Modify: `crates/yunxi-agent-runtime/src/lib.rs`
- Modify: `crates/yunxi-agent-tools/src/lib.rs`
- Modify: `crates/yunxi-agent-exec/src/lib.rs`
- Modify: `crates/yunxi-agent-cli/src/interactive.rs`

**Interfaces:**
- Consumes: shared `AgentCancellationToken`.
- Produces: Ctrl+C cancellation that is visible to runtime/tools/exec and kills running shell children.

- [ ] Carry the token from CLI into runtime and tool requests.
- [ ] Check cancellation at runtime loop boundaries and before tool execution.
- [ ] Add cancellation-aware exec execution that kills the child process.
- [ ] Emit cancelled events without discarding events already streamed.
- [ ] Avoid updating active session state for cancelled turns.

### Task 5: Terminal Rendering and Default-Clarity Improvements

**Files:**
- Modify: `crates/yunxi-agent-cli/src/render.rs`
- Modify: `crates/yunxi-agent-cli/src/commands.rs`
- Modify: `crates/yunxi-agent-cli/src/provider_mode.rs`
- Modify: `crates/yunxi-agent-tools/src/lib.rs`
- Modify: `README.md`
- Modify: `docs/extraction-status.md`

**Interfaces:**
- Produces: `render_agent_event(event: &AgentEvent) -> Result<()>`.
- Preserves: `render_agent_result(result: &AgentRunResult) -> Result<()>`.

- [ ] Refactor result rendering to share per-event rendering with streaming CLI.
- [ ] Render `FileChanged`, `PatchCompleted`, `TodoUpdated`, and `Completed` usage details.
- [ ] Make `/clear` clear the terminal when possible.
- [ ] Make offline/static provider and fixture MCP status explicit in banner/docs.
- [ ] Keep static provider available for offline/test paths.

### Task 6: Versioning, Tests, Unified Verification, Release

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/yunxi-agent-cli/tests/cli_tests.rs`
- Modify: `crates/yunxi-agent-runtime/tests/runtime_tests.rs`
- Modify: `crates/yunxi-agent-tools/tests/tool_tests.rs`
- Modify: `docs/reports/2026-07-12-yunxi-agent-v1-3-terminal-streaming-approval-cancellation-development-report.md`
- Modify outside repo: `C:\Users\admin\Desktop\YunXi Agent开发日志.md`

**Interfaces:**
- Produces: released v1.3.0 CLI and immutable GitHub tag.

- [ ] Raise workspace version to `1.3.0`.
- [ ] Add tests for streaming event arrival, approval approve/deny, cancellation, request_user_input, and new rendering coverage.
- [ ] After all construction, run one unified verification gate: `cargo fmt`, `cargo fmt -- --check`, `cargo test`, `cargo check --workspace`, release build, version checks, offline smoke, live DeepSeek smoke, dependency scan, secret scan, `git diff --check`.
- [ ] Install release to PATH and verify installed binary against release SHA-256.
- [ ] Sync CodeGraph.
- [ ] Commit and create annotated tag `v1.3.0`.
- [ ] Publish commit/master/tag through GitHub REST API only.
- [ ] Write the full timestamped desktop development log.
- [ ] Run `cargo clean`, remove root `.yunxi`, and verify clean final state.


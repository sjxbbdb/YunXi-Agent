# Extraction Status

## Current Stage

The repository currently contains:

- A standalone Rust workspace
- `yunxi-agent-core` facade types
- A dry-run `Agent` runner
- A minimal `yunxi-agent-cli`
- A `yunxi-agent-codex` integration crate for upstream Codex headless runtime
- YunXi-owned runtime boundary crates:
  - `yunxi-agent-runtime`
  - `yunxi-agent-provider`
  - `yunxi-agent-tools`
  - `yunxi-agent-storage`
- A vendored Codex Rust workspace snapshot at `vendor/codex-rs`
- A `CodexSource` boundary retained for source-shape checks and future refresh
  tooling

## Stage 4A Runtime Boundary

Stage 4A introduces a YunXi-owned autonomous runtime boundary.

Implemented in this slice:

- `yunxi-agent-runtime` owns the first YunXi runtime backend implementation.
- `yunxi-agent-provider` owns provider request/response interfaces and a
  deterministic `StaticProvider`.
- `yunxi-agent-tools` owns tool request/response interfaces and a placeholder
  `NoopToolRuntime`.
- `yunxi-agent-storage` owns session records and an in-memory session store.
- `yunxi-agent-cli` defaults to `--backend yunxi`.
- `--backend dry-run`, `--backend codex`, and `--live` remain available for
  smoke checks and compatibility.

The `yunxi` backend emits YunXi `AgentEvent` values directly and does not depend
on `vendor/codex-rs`. The Codex backend remains as an explicit compatibility
path while provider/tool/storage behavior is migrated in later slices.

## Stage 4A Verification

Verified on 2026-07-10:

- `cargo fmt -- --check`: pass
- `cargo test`: pass
- `cargo check -p yunxi-agent-runtime`: pass
- `cargo check -p yunxi-agent-provider`: pass
- `cargo check -p yunxi-agent-tools`: pass
- `cargo check -p yunxi-agent-storage`: pass
- `cargo check -p yunxi-agent-cli`: pass
- `cargo build -p yunxi-agent-cli`: pass
- `cargo run -p yunxi-agent-cli -- --backend yunxi --jsonl "explain this project"`:
  pass
- `cargo run -p yunxi-agent-cli -- --backend dry-run "explain this project"`:
  pass
- `git diff --check`: pass with Windows line-ending warnings only

## Next Stage

Stage 4C should turn the independent default runtime into a practical
YunXi-owned agent by adding a real provider adapter, approval/sandbox policy,
constrained patch support, and session CLI commands. The development report is
recorded in
`docs/reports/2026-07-10-yunxi-stage-4c-provider-policy-runtime-report.md`.

## Stage 4C Provider, Policy, Patch, And Session Slice

Implemented in this slice:

- `yunxi-agent-provider` owns OpenAI-compatible provider configuration,
  request serialization, response parsing, auth source resolution, and
  fixture-backed completion.
- `yunxi-agent-tools` owns explicit approval and sandbox policy mapping from
  `AgentConfig` before tool execution.
- `yunxi-agent-tools` owns constrained patch operations for write and delete,
  with rejection of absolute paths and parent-directory traversal.
- `yunxi-agent-runtime` maps provider-requested shell, patch, MCP, and skill
  calls into policy-carrying YunXi tool requests.
- `yunxi-agent-runtime` emits provider turn reasoning, tool completion,
  warning, patch, command, MCP, and file-change events through YunXi event
  types.
- `yunxi-agent-cli` can list and inspect file-backed sessions through
  `sessions list` and `sessions show`.

## Stage 4C Verification

Verified on 2026-07-10:

- `cargo fmt -- --check`: pass
- `cargo test`: pass
- `cargo check -p yunxi-agent-core`: pass
- `cargo check -p yunxi-agent-provider`: pass
- `cargo check -p yunxi-agent-tools`: pass
- `cargo check -p yunxi-agent-storage`: pass
- `cargo check -p yunxi-agent-runtime`: pass
- `cargo check -p yunxi-agent-cli`: pass
- `cargo build -p yunxi-agent-cli`: pass
- `cargo run -p yunxi-agent-cli -- --cwd <temp> --backend yunxi --jsonl "explain this project"`:
  pass
- `cargo tree -p yunxi-agent-cli`: pass; default tree contains no
  `yunxi-agent-codex`, `vendor/codex-rs`, or `codex-*` crates
- disabled-vendor verification: pass after temporarily renaming
  `vendor/codex-rs` to `vendor/codex-rs.disabled`
- `git diff --check`: pass with Windows line-ending warnings only

## Stage 4D Planned Direction

The next stage is Codex core agent parity extraction. Its goal is to replicate
the Codex CLI headless Agent core capabilities in YunXi-owned crates while
keeping the default YunXi runtime independent from `vendor/codex-rs` and
`codex-*` crates.

The Stage 4D development report is recorded in
`docs/reports/2026-07-10-yunxi-stage-4d-codex-core-agent-parity-report.md`.

Stage 4D implementation has started with the first parity foundation slice:

- `docs/extraction-index/codex-core-agent-parity-map.md` maps Codex core agent
  source files to YunXi-owned target crates.
- `yunxi-agent-protocol` owns provider/runtime input, response, tool-call, and
  JSONL event protocol types.
- `yunxi-agent-context` owns the first AGENTS.md hierarchy loader and context
  bundle facade.
- `yunxi-agent-sandbox` owns approval, sandbox, cwd, and network policy
  decision types.
- `yunxi-agent-exec` owns command canonicalization and output aggregation
  primitives.
- `yunxi-agent-patch` owns constrained JSON patch and Codex-style
  `*** Begin Patch` application.
- `yunxi-agent-mcp` owns MCP configuration, resource, and tool invocation
  interfaces.
- `yunxi-agent-skills` owns skill discovery, metadata, and invocation
  interfaces.
- `yunxi-agent-multi-agent` owns multi-agent command and registry interfaces.
- `yunxi-agent-runtime` now loads workspace `AGENTS.md` instructions into the
  provider message stream.
- `yunxi-agent-cli parity map` prints the Codex core parity migration map.

Stage 4D full parity is not complete yet. The current slice establishes the
autonomous YunXi-owned module boundaries needed to continue mechanical
migration from `vendor/codex-rs` without adding upstream crate dependencies.

Stage 4D foundation verification passed on 2026-07-10:

- `cargo fmt -- --check`: pass
- `cargo test`: pass
- package checks for `yunxi-agent-core`, `yunxi-agent-protocol`,
  `yunxi-agent-provider`, `yunxi-agent-context`, `yunxi-agent-sandbox`,
  `yunxi-agent-exec`, `yunxi-agent-patch`, `yunxi-agent-tools`,
  `yunxi-agent-mcp`, `yunxi-agent-skills`, `yunxi-agent-multi-agent`,
  `yunxi-agent-storage`, `yunxi-agent-runtime`, and `yunxi-agent-cli`: pass
- `cargo build -p yunxi-agent-cli`: pass
- `cargo run -p yunxi-agent-cli -- --cwd <temp> --backend yunxi --jsonl "explain this project"`:
  pass
- `cargo run -p yunxi-agent-cli -- parity map`: pass
- `cargo tree -p yunxi-agent-cli`: pass; default tree contains no
  `yunxi-agent-codex`, `vendor/codex-rs`, or `codex-*` crates
- disabled-vendor verification: pass after temporarily renaming
  `vendor/codex-rs` to `vendor/codex-rs.disabled`
- `git diff --check`: pass with Windows line-ending warnings only

## Stage 4F Dynamic Tool Autonomy Slice

This slice turns a first group of migrated Codex core dynamic-tool surfaces into
YunXi-owned runtime behavior instead of schema-only placeholders.

Implemented in this slice:

- `yunxi-agent-tools` owns `CompositeToolRuntime`, which keeps shell, patch,
  tool search, image inspection, and host-input behavior while dispatching MCP,
  skill, and multi-agent requests to YunXi-owned runtimes.
- `yunxi-agent-runtime` defaults to `CompositeToolRuntime`, so provider-emitted
  MCP, skill, and multi-agent tool calls can execute through the default YunXi
  backend.
- `yunxi-agent-mcp` can load an in-memory MCP runtime seed from a project-local
  JSON file, enabling workspace-owned MCP server/tool/resource fixtures without
  depending on upstream Codex source.
- Default composite MCP execution checks `.yunxi/mcp-runtime.json` in the
  current workspace before falling back to an injected MCP runtime.
- Skill execution discovers workspace skills from `.codex/skills`,
  `.yunxi/skills`, and `skills`, then returns the selected `SKILL.md`
  instruction payload through the tool result channel.
- Multi-agent execution supports spawn, wait, send-message, follow-up,
  interrupt, and list actions through `yunxi-agent-multi-agent`'s in-memory
  registry.
- Runtime tests cover provider-requested MCP, skill, and multi-agent tool calls
  flowing through the autonomous YunXi backend.

Stage 4F verification passed on 2026-07-10:

- `cargo fmt`: pass
- `cargo fmt -- --check`: pass
- `cargo test`: pass
- `cargo check --workspace`: pass
- `cargo build -p yunxi-agent-cli`: pass
- `cargo run -p yunxi-agent-cli -- parity map`: pass
- `cargo tree -p yunxi-agent-cli`: pass; default tree contains no
  `yunxi-agent-codex`, `vendor/codex-rs`, or `codex-*` crates
- `git diff --check`: pass with Windows line-ending warnings only

## Stage 4G Planned Direction

The next stage is the full core Agent parity one-pass build. Its goal is to
turn the remaining migrated Codex CLI headless Agent sources into YunXi-owned
runtime behavior across protocol streaming, live provider transport, exec,
sandbox/approval, patch, context, MCP, skills/plugins, multi-agent, storage,
rollout, resume, and CLI JSONL parity.

The Stage 4G development report is recorded in
`docs/reports/2026-07-10-yunxi-stage-4g-full-core-agent-parity-one-pass-development-report.md`.

## Stage 4G First Construction Slice

Stage 4G implementation has started with the protocol, provider, exec, approval,
and runtime event foundation needed by the larger one-pass parity build.

Implemented in this slice:

- `yunxi-agent-protocol` now carries tool output deltas, response cancellation,
  and approval requested/completed runtime events.
- `yunxi-agent-core` exposes approval requested/completed agent events with
  stable JSON names.
- `yunxi-agent-exec` can build an `ExecTrace` from a completed shell execution,
  including started, stdout/stderr delta, and completed lifecycle events.
- `yunxi-agent-tools` attaches exec lifecycle events to `ToolResponse` for shell
  execution.
- `yunxi-agent-runtime` emits approval lifecycle events for approval-blocked
  tools and maps exec lifecycle output deltas into command update events.
- `yunxi-agent-cli` maps command output deltas and approval lifecycle events into
  protocol JSONL runtime events.
- `yunxi-agent-provider` extends provider config with timeout, streaming, and
  capability metadata controlling tools, parallel tool calls, reasoning, and
  stream usage.

Stage 4G first construction verification passed on 2026-07-10:

- `cargo fmt`: pass
- `cargo fmt -- --check`: pass
- `cargo test`: pass
- `cargo check --workspace`: pass
- `cargo build -p yunxi-agent-cli`: pass
- `cargo run -p yunxi-agent-cli -- parity map`: pass
- `cargo tree -p yunxi-agent-cli`: pass; default tree contains no
  `yunxi-agent-codex`, `vendor/codex-rs`, or `codex-*` crates
- `git diff --check`: pass with Windows line-ending warnings only

## Stage 4G Exec Manager Runtime Slice

This slice turns the first Stage 4G exec foundation into default YunXi runtime
behavior.

Implemented in this slice:

- `yunxi-agent-exec` owns an async `ExecManager` that spawns platform shell
  commands, writes optional stdin, captures stdout/stderr, enforces timeout
  limits, kills timed-out processes, records lifecycle events, and returns an
  `ExecTrace`.
- `yunxi-agent-exec` now depends on workspace `tokio` with `io-util` and
  `time` features so process I/O and timeout handling live in the YunXi-owned
  exec crate.
- `yunxi-agent-tools` routes shell execution through `ExecManager` instead of
  direct `process.output()`, so the default tool runtime consumes the shared
  exec lifecycle path.
- `yunxi-agent-runtime` renders non-empty tool output/errors first, then falls
  back to structured failed/declined messages when shell output is empty.
- `yunxi-agent-runtime` emits useful warnings for failed shell tools, including
  exit-code-only failures, while avoiding duplicate warnings for cancelled exec
  lifecycle events.
- Tests cover stdin capture, timeout cancellation, non-zero shell exit status,
  and runtime propagation of failed shell tool output.

Stage 4G exec manager runtime verification passed on 2026-07-10:

- `cargo fmt`: pass
- `cargo fmt -- --check`: pass
- `cargo test`: pass
- `cargo check --workspace`: pass
- `cargo build -p yunxi-agent-cli`: pass
- `cargo run -p yunxi-agent-cli -- parity map`: pass
- `cargo tree -p yunxi-agent-cli`: pass; default tree contains no
  `yunxi-agent-codex`, `vendor/codex-rs`, or `codex-*` crates
- `git diff --check`: pass with Windows line-ending warnings only

## Stage 4G Sandbox Approval Escalation Slice

This slice turns migrated sandbox, approval, and escalation policy primitives
into default YunXi runtime behavior.

Implemented in this slice:

- `yunxi-agent-sandbox` evaluates approval, sandbox, cwd, command-risk, and
  network policy in one `PolicyEvaluation`.
- `yunxi-agent-sandbox` records explicit `ApprovalRequest` and
  `EscalationRequest` details, including required sandbox or network policy
  changes for blocked commands.
- `yunxi-agent-tools` stores the full policy evaluation in
  `ToolDispatchTrace`, routes shell, patch, MCP, skill, multi-agent, search,
  image, and host-input requests through the shared evaluation path, and keeps
  low-risk read-only shell commands runnable while blocking write-risk commands.
- `yunxi-agent-core` exposes escalation requested/completed agent events with
  stable JSON names.
- `yunxi-agent-protocol` exposes escalation requested/completed JSONL runtime
  events.
- `yunxi-agent-runtime` emits approval and escalation lifecycle events from the
  shared tool dispatch trace instead of relying on string matching against
  declined policy reasons.
- `yunxi-agent-cli` maps escalation lifecycle agent events into protocol JSONL
  runtime events.
- Tests cover read-only sandbox write blocking, network-disabled escalation,
  outside-workspace escalation, stable escalation event names, escalation JSONL
  round-trips, and runtime escalation emission.

Stage 4G sandbox approval escalation verification passed on 2026-07-10:

- `cargo fmt`: pass
- `cargo fmt -- --check`: pass
- `cargo test`: pass
- `cargo check --workspace`: pass
- `cargo build -p yunxi-agent-cli`: pass
- `cargo run -p yunxi-agent-cli -- parity map`: pass
- `cargo tree -p yunxi-agent-cli`: pass; default tree contains no
  `yunxi-agent-codex`, `vendor/codex-rs`, or `codex-*` crates
- dependency-tree keyword scan for `codex`, `vendor`, and
  `yunxi-agent-codex`: pass with no matches
- `git diff --check`: pass with Windows line-ending warnings only

## Stage 4D History Restore And Compact Entry Slice

Implemented in this slice:

- `yunxi-agent-storage` can reconstruct parent-linked session history from root
  to child through `SessionStore::history`.
- `yunxi-agent-storage` exposes `SessionHistory` and `HistoryItem` records for
  resume, history inspection, and future rollout reconstruction.
- `yunxi-agent-context` owns the first YunXi context-window budget model,
  approximate token counting, compact pressure status, and deterministic
  history compaction summary.
- `AgentConfig` carries optional context-window and auto-compact token limits.
- `yunxi-agent-runtime` restores parent session history into provider messages
  before the current user prompt.
- `yunxi-agent-runtime` compacts restored history into a system summary when the
  configured context budget is exceeded.
- `yunxi-agent-cli` exposes `sessions history <id>`.
- `yunxi-agent-cli sessions resume` now passes only the new user prompt and
  relies on runtime-owned history restore instead of embedding previous prompt
  text in the prompt string.

This slice is still a parity step, not the end state. It adds the autonomous
history restore and compact entry points required before migrating deeper Codex
context-manager behavior.

## Stage 4B Full Autonomy Slice

Implemented in this slice:

- The default workspace member set excludes `yunxi-agent-codex`.
- `yunxi-agent-cli` no longer depends on `yunxi-agent-codex`.
- `--backend codex` and `--live` report clear compatibility guidance in
  default builds.
- `yunxi-agent-tools` owns a shell command executor.
- `yunxi-agent-provider` can represent provider-requested tool calls.
- `yunxi-agent-runtime` can execute provider-requested shell tools and feed
  results back into the provider loop.
- `yunxi-agent-storage` owns file-backed session records under
  `.yunxi/sessions`.

## Stage 4B Verification

Verified on 2026-07-10:

- `cargo fmt -- --check`: pass
- `cargo test`: pass
- `cargo check -p yunxi-agent-core`: pass
- `cargo check -p yunxi-agent-provider`: pass
- `cargo check -p yunxi-agent-tools`: pass
- `cargo check -p yunxi-agent-storage`: pass
- `cargo check -p yunxi-agent-runtime`: pass
- `cargo check -p yunxi-agent-cli`: pass
- `cargo build -p yunxi-agent-cli`: pass
- `cargo run -p yunxi-agent-cli -- --cwd <temp> --backend yunxi --jsonl "explain this project"`:
  pass
- `cargo tree -p yunxi-agent-cli`: pass; default tree contains no
  `yunxi-agent-codex`, `vendor/codex-rs`, or `codex-*` crates
- disabled-vendor verification: pass after temporarily renaming
  `vendor/codex-rs` to `vendor/codex-rs.disabled`
- `cargo check --manifest-path crates\yunxi-agent-codex\Cargo.toml`: pass
  without `codex-native`
- `git diff --check`: pass with Windows line-ending warnings only

## Vendored Codex Source

Stage 3 imported the Codex Rust workspace source into `vendor/codex-rs` on
2026-07-10.

- Upstream commit at import time: `f1affbac5e5164b2bae825e9b39e9868bc4e0be2`
- Import style: mechanical full Rust workspace source snapshot
- Excluded during import: `target`, `.git`, `node_modules`
- Local patch record: `vendor/codex-rs/patches/README.md`

`yunxi-agent-codex` path dependencies now point at `vendor/codex-rs`, so the
native backend source is intended to be available from a fresh YunXi checkout
without manually creating `external/codex-rs`.

Stage 3 verification has now been run against the vendored source. The native
backend no longer depends on a developer-created `external/codex-rs` link for
Cargo path resolution.

## Local Codex Source Link

`external/codex-rs` may still exist in a developer workspace as a junction or
symlink to a local Codex CLI checkout. It is no longer the source of truth for
normal YunXi builds.

The `CodexSource` boundary verifies the checkout shape by checking for:

- `codex-rs/Cargo.toml`
- `codex-rs/exec/src/lib.rs`
- `codex-rs/app-server-client/Cargo.toml`

The controller confirmed these three files exist in the real local checkout
used for this extraction, but that local path is intentionally not recorded
here.

## Stage 2 Live Backend

The Codex native backend has a source-level new-thread runner behind the
`codex-native` feature. Normal tests do not require live credentials.

Current live scope:

- text prompt
- new headless thread
- ephemeral run
- cwd/model/provider mapping
- approval and sandbox mapping
- in-process Codex app-server runtime
- JSONL event mapping into YunXi events

The in-process runtime is the upstream Codex Agent stack, so shell execution,
patches, AGENTS.md loading, MCP configuration, skills runtime, session storage,
and rollout behavior stay inside the embedded Codex runtime rather than being
reimplemented in YunXi core.

Still expanding:

- resume/history CLI flags
- full MCP fixture coverage
- skills fixture coverage
- Windows helper/arg0 packaging verification for patch and sandbox helpers
- owned extraction of selected upstream runner internals after the source-level
  bridge is stable

## Stage 2 Verification

Verified on 2026-07-09:

- `cargo fmt -- --check`: pass
- `cargo test`: pass
- `cargo check -p yunxi-agent-codex --features codex-native`: pass
- `cargo check -p yunxi-agent-cli --features codex-native`: pass
- `cargo build -p yunxi-agent-cli --features codex-native`: pass
- gated live smoke without `YUNXI_RUN_LIVE_CODEX_TESTS=1`: pass and skipped
- dry-run CLI smoke: pass
- dry-run JSONL smoke: pass
- `git diff --check`: pass

Live credential smoke was not run; it remains gated by
`YUNXI_RUN_LIVE_CODEX_TESTS=1`.

## Stage 3 Verification

Verified on 2026-07-10:

- `cargo fmt -- --check`: pass
- `cargo test`: pass
- `cargo check -p yunxi-agent-codex --features codex-native`: pass
- `cargo check -p yunxi-agent-cli --features codex-native`: pass
- `cargo build -p yunxi-agent-cli --features codex-native`: pass
- `cargo run -p yunxi-agent-cli -- --backend dry-run "explain this project"`:
  pass
- `cargo run -p yunxi-agent-cli -- --backend dry-run --jsonl "explain this project"`:
  pass
- `cargo test -p yunxi-agent-codex --features codex-native live_codex_backend_can_complete_simple_prompt_when_enabled -- --nocapture`:
  pass and skipped without `YUNXI_RUN_LIVE_CODEX_TESTS=1`
- `git diff --check`: pass

During the first native check, the `v8` crate failed while decompressing the
downloaded `rusty_v8` prebuilt library. The failed build-script output and
temporary `rusty_v8` target files were removed, and the same native check passed
on the next run. No source change was needed for that transient artifact issue.

Live credential smoke was not run; it remains gated by
`YUNXI_RUN_LIVE_CODEX_TESTS=1`.

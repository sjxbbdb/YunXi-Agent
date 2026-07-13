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

## Stage 4H Planned Direction

The next stage is upstream core gap closure. Its goal is to use
`vendor/codex-rs` and `extracted/codex-core-agent-sources` as direct behavior
references and close the remaining Codex CLI headless Agent parity gaps in
YunXi-owned crates, without restoring upstream runtime dependencies.

Stage 4H must follow the project hard constraint: build the remaining capability
surface first, avoid repeated mid-construction validation loops, and run the
full verification gate only after the 12 remaining parity layers are migrated
and wired into the default YunXi runtime.

The Stage 4H development report is recorded in
`docs/reports/2026-07-10-yunxi-stage-4h-upstream-core-gap-closure-development-report.md`.

## Stage 4H Gap Closure Construction Slice

Stage 4H construction is in progress and intentionally has not run the final
verification gate yet. This slice extends the YunXi-owned headless runtime
surface without restoring any default upstream Codex runtime dependency.

Constructed in this slice so far:

- `yunxi-agent-provider` builds live/fixture OpenAI-compatible requests from a
  workspace-aware tool registry and parses dynamic `skill__*`, `plugin__*`, and
  `mcp__*` function names back into YunXi-owned tool call variants.
- `yunxi-agent-tools` can export fixed plus workspace-discovered dynamic tools
  as provider function schemas.
- `yunxi-agent-skills` discovers workspace plugins, resolves plugin skill/MCP
  roots, and emits dynamic tool metadata for skills, plugins, and plugin MCP
  seeds.
- `yunxi-agent-runtime` maps streamed dynamic function names into skill, MCP,
  or tool-search calls while keeping the provider/model layer replaceable.
- `yunxi-agent-runtime` injects mentioned workspace file context from prompts
  such as `@src/lib.rs` into provider messages with bounded file content.
- `yunxi-agent-patch` exposes structured patch diagnostics through
  `PatchDiagnostic`, `PatchApplyError`, and `apply_patch_detailed`.
- Patch execution now returns a failed `ToolResponse` with serialized
  diagnostics for patch failures, allowing the runtime to continue through the
  normal tool failed event path.
- `yunxi-agent-mcp` owns an HTTP JSON-RPC client facade with a replaceable
  transport, a reqwest-backed default transport, and fixture transport for
  offline verification.
- `yunxi-agent-multi-agent` owns serializable agent graph session metadata,
  cycle-safe insertion/validation, and graph reconstruction from persisted
  metadata.
- `yunxi-agent-storage` owns a storage-backed session graph view and can export
  persisted session/thread metadata into multi-agent graph metadata.
- `yunxi-agent-cli` exposes stable YunXi exit-code classification and a
  `sessions graph` command for persisted parent/child thread views.
- `tool_search` now returns both workspace file matches and dynamic tool
  metadata discovered from the active workspace.

Stage 4H gap closure construction verification passed on 2026-07-10:

- `cargo fmt`: pass
- `cargo fmt -- --check`: pass
- `cargo test`: pass
- `cargo check --workspace`: pass
- `cargo build -p yunxi-agent-cli`: pass
- `cargo run -p yunxi-agent-cli -- parity map`: pass
- `cargo tree -p yunxi-agent-cli`: pass; scan found no `codex`,
  `vendor`, or `yunxi-agent-codex` dependency in the default CLI tree
- `git diff --check`: pass with Windows line-ending warnings only
- `cargo clean`: pass; removed 1.2GiB of build artifacts

## Stage 4I Planned Direction

The next stage is behavior-level core parity closure. Stage 4H moved the main
headless Agent capability surfaces into YunXi-owned crates; Stage 4I should
turn those surfaces into deeper Codex CLI headless core behavior parity without
restoring any upstream runtime dependency.

Stage 4I focuses on the remaining deep gaps:

- true incremental provider SSE consumption
- long-lived MCP session runtime
- platform sandbox runner and escalation boundary
- multi-agent child runtime execution
- full apply_patch boundary parity
- context, compact, rollout, and state reconstruction
- protocol, event, and CLI JSONL full shape
- disabled-vendor parity harness

The Stage 4I development report is recorded in
`docs/reports/2026-07-10-yunxi-stage-4i-behavior-level-core-parity-closure-development-report.md`.

## Stage 4I First Construction Slice

Stage 4I implementation has completed the first behavior-level parity
foundation slice. Construction followed the build-first constraint and ran the
full verification gate only after the slice was wired.

Implemented in this slice:

- `yunxi-agent-provider` now owns an incremental SSE decoder,
  `ProviderStreamChunk`, and `OpenAiStreamAccumulator` so provider stream
  events can be consumed chunk by chunk instead of only after full-body parsing.
- Provider stream parsing now emits `ToolCallName` deltas alongside argument
  deltas, allowing streamed dynamic function calls to retain their tool names.
- `yunxi-agent-runtime` uses streamed tool-call names when reconstructing
  provider tool calls, avoiding the previous shell fallback for argument-only
  streams.
- `yunxi-agent-protocol` now includes deep parity runtime event shapes for MCP
  session state, multi-agent state, context status, storage state, and file
  changes.
- `yunxi-agent-core`, `yunxi-agent-runtime`, and `yunxi-agent-cli` now carry
  context/storage state events through to protocol JSONL output.
- `yunxi-agent-mcp` now owns a workspace MCP config loader and
  `McpSessionManager` with configured/initialized/failed/shutdown session
  state, stdio/http JSON-RPC request dispatch, tool/resource listing, resource
  read, and tool call boundaries.
- `yunxi-agent-tools` now injects enabled workspace MCP servers from
  `.yunxi/mcp.json`, `.yunxi/mcp-servers.json`, or `.mcp.json` into the dynamic
  provider tool registry and routes MCP calls through workspace session manager
  before falling back to injected runtime.
- `yunxi-agent-exec` now owns `ExecHandleRegistry` for long-running command
  status, cancellation request, output polling, and handle listing.
- `yunxi-agent-sandbox` now owns `EscalationResponse` and `EscalationOutcome`
  for approved/declined/not-available escalation results.
- `yunxi-agent-multi-agent` now owns child runtime request/result facade types
  without depending on the runtime crate, keeping the child execution boundary
  YunXi-owned and cycle-free.
- `yunxi-agent-context` now owns `ContextWindowPhase` and
  `PromptDebugSnapshot`, giving prompt/context debugging a structured state
  surface.
- `yunxi-agent-storage` now owns `RuntimeStateSnapshot` for session, parent,
  rollout, archive, and pin state reconstruction.
- Fixture tests cover incremental SSE decoding, MCP config dynamic tool
  injection, MCP config loading, and new protocol event round-trips.

Stage 4I first construction verification passed on 2026-07-10:

- `cargo fmt`: pass
- `cargo fmt -- --check`: pass
- `cargo test`: pass
- `cargo check --workspace`: pass
- `cargo build -p yunxi-agent-cli`: pass
- `cargo run -p yunxi-agent-cli -- parity map`: pass
- `cargo run -p yunxi-agent-cli -- --backend yunxi --jsonl "run stage 4i fixture"`:
  pass, including `storage_state` JSONL output
- `cargo tree -p yunxi-agent-cli`: pass; dependency keyword scan found no
  `codex`, `vendor`, or `yunxi-agent-codex` dependency in the default CLI tree
- `git diff --check`: pass with Windows line-ending warnings only
- `cargo clean`: pass; removed 1.3GiB of build artifacts

## Stage 4I Second Construction Slice

Stage 4I continued with a second behavior-level parity construction slice. This
slice kept the build-first constraint: implementation was wired first, then the
full verification gate was run once.

Implemented in this slice:

- `yunxi-agent-tools` now carries structured `ToolRuntimeEvent` values on
  `ToolResponse`, allowing sandbox decisions, MCP session state, multi-agent
  state, and patch diagnostics to cross the tool/runtime boundary.
- `yunxi-agent-runtime` maps tool runtime events into YunXi `AgentEvent`
  values, including MCP session JSONL events, multi-agent JSONL events,
  sandbox reasoning, and patch diagnostic warnings.
- `yunxi-agent-multi-agent` now owns a `ChildAgentRuntime` boundary,
  `FixtureChildAgentRuntime`, and `SpawnRun` command so child-agent execution
  has an autonomous YunXi-owned run-result path without depending on the
  runtime crate.
- `yunxi-agent-tools` exposes `spawn_run` as a model-visible multi-agent action
  and returns child run results plus multi-agent runtime events from the
  composite tool runtime.
- `yunxi-agent-patch` now rejects Codex-style `Add File` operations that would
  overwrite an existing target, rejects duplicate touched paths in a single
  patch, rejects move destinations that already exist, and reports non-UTF-8
  update targets with a dedicated diagnostic kind.
- New fixture tests cover child-agent spawn/run, runtime multi-agent event
  emission, multi-agent tool runtime events, and the new patch boundary
  diagnostics.

Stage 4I second construction verification passed on 2026-07-10:

- `cargo fmt`: pass
- `cargo fmt -- --check`: pass
- `cargo test`: pass
- `cargo check --workspace`: pass
- `cargo build -p yunxi-agent-cli`: pass
- `cargo run -p yunxi-agent-cli -- parity map`: pass
- `cargo run -p yunxi-agent-cli -- --backend yunxi --jsonl "run stage 4i second slice fixture"`:
  pass, including `storage_state` JSONL output
- `cargo tree -p yunxi-agent-cli`: pass; dependency keyword scan found no
  `codex`, `vendor`, or `yunxi-agent-codex` dependency in the default CLI tree
- `git diff --check`: pass with Windows line-ending warnings only
- `cargo clean`: pass; removed 1.3GiB of build artifacts

## Stage 4J Planned Direction

The next stage is child runtime and end-to-end parity harness construction. Its
goal is to turn the Stage 4I `spawn_run` fixture boundary into a real
YunXi-owned child runtime turn, persist parent/child sessions in storage, merge
child lifecycle events into the parent JSONL stream, and establish the first
offline end-to-end parity harness for provider stream -> tool loop -> child
runtime -> storage -> JSONL.

The Stage 4J development report is recorded in
`docs/reports/2026-07-10-yunxi-stage-4j-child-runtime-e2e-parity-development-report.md`.

## Stage 4J Child Runtime And End-To-End Parity Harness Slice

Implemented in this slice:

- `yunxi-agent-runtime` now owns a real `YunXiChildAgentRuntime` adapter for
  `multi_agent spawn_run`, executes a child YunXi runtime turn, and writes the
  child session through YunXi-owned storage.
- `yunxi-agent-runtime` assigns the parent session id at turn start and injects
  it into multi-agent requests so child sessions can persist `parent_id`
  metadata before the parent session is saved.
- `yunxi-agent-core` and `yunxi-agent-protocol` expose `ChildAgentEvent` /
  `child_agent` JSONL events carrying `agent_id`, `child_session_id`,
  `parent_session_id`, `status`, and `message`.
- `yunxi-agent-storage` `RuntimeStateSnapshot` and `storage_state` events now
  track child session ids and child session count.
- `yunxi-agent-provider` includes an offline Stage 4J fixture prompt that emits
  a parent `multi_agent spawn_run` tool call and then completes from the child
  tool result.
- `yunxi-agent-runtime` has a depth guard for child runtime execution and
  returns structured child failure events instead of panicking or breaking the
  parent provider loop.
- `yunxi-agent-cli` maps child events to one-JSON-object-per-line JSONL output,
  and the CLI JSONL fixture verifies the emitted `child_agent` event shape.

Stage 4J construction verification passed on 2026-07-10:

- `cargo fmt`: pass
- `cargo fmt -- --check`: pass
- `cargo test`: pass
- `cargo check --workspace`: pass
- `cargo build -p yunxi-agent-cli`: pass
- `cargo run -p yunxi-agent-cli -- parity map`: pass
- `cargo run -p yunxi-agent-cli -- --backend yunxi --jsonl "run stage 4j child runtime fixture"`:
  pass, including parent `multi_agent`, scoped `child_agent`, and final
  `storage_state` JSONL events with `child_session_ids`
- JSONL parse check for the Stage 4J fixture: pass; 23 JSONL lines parsed and 8
  `child_agent` events observed
- `cargo tree -p yunxi-agent-cli`: pass; dependency keyword scan found no
  `codex`, `vendor`, or `yunxi-agent-codex` dependency in the default CLI tree
- `git diff --check`: pass with Windows line-ending warnings only
- `cargo clean`: pass; removed 1.4GiB of build artifacts
- Local `.yunxi` smoke session artifacts from the Stage 4J fixture were removed

## Stage 4K Planned Direction

The next stage is DeepSeek live provider and real-world parity validation. Stage
4J proved the offline autonomous runtime chain; Stage 4K should connect that
chain to a real OpenAI-compatible provider using a DeepSeek API key stored
outside the repository at `<private-api-file>`.

The Stage 4K development report is recorded in
`docs/reports/2026-07-10-yunxi-stage-4k-deepseek-live-provider-parity-development-report.md`.

Stage 4K focuses on:

- secret-safe DeepSeek smoke harness
- optional `YUNXI_PROVIDER_PROFILE=deepseek` compatibility profile
- provider error classification and redaction
- non-stream and stream DeepSeek live smoke matrix
- mapping live provider failures back to actionable parity buckets
- real model child provider path, while preserving deterministic fixture child
  provider tests
- async cancellation propagation across provider stream, tools, MCP calls, child
  runtime, and storage boundaries
- platform sandbox runner deepening, including Windows runner diagnostics and
  escalation-needed states
- long-lived MCP session reuse with shutdown, health check, timeout, and cancel
  hooks
- finer-grained child scoped stream events for child provider, tool, storage,
  cancellation, and finish states
- keeping the default YunXi CLI dependency tree free of `codex`, `vendor`, and
  `yunxi-agent-codex`

Live smoke must never print or persist API keys. All smoke artifacts, `.yunxi`
session output, and `target` build artifacts must be removed after verification.

## Stage 4K Construction Slice

Implemented in this construction slice before final verification:

- `yunxi-agent-provider` owns the `deepseek` provider profile, DeepSeek-compatible
  capability defaults, stream/non-stream switching through provider-neutral config,
  and redacted provider error classification.
- `yunxi-agent-runtime` can run live-provider child agents through an inherited
  provider facade while preserving deterministic fixture child providers for
  offline tests.
- `yunxi-agent-core`, `yunxi-agent-runtime`, and `yunxi-agent-cli` now carry
  explicit cancellation events, provider error events, and child scoped stream
  events.
- `yunxi-agent-sandbox` exposes platform sandbox runner diagnostics for
  ready/denied/escalation-needed runner states.
- `yunxi-agent-tools` caches workspace MCP session managers for long-lived reuse
  and exposes sandbox runner/MCP lifecycle runtime events.
- `yunxi-agent-mcp` exposes session health, cancel, shutdown, and shutdown-all
  lifecycle hooks.
- `scripts/provider/deepseek-live-smoke.ps1` provides the secret-safe DeepSeek
  live smoke harness.

Final unified verification was run on 2026-07-11:

- `cargo fmt`: pass
- `cargo fmt -- --check`: pass
- `cargo test`: pass; workspace unit tests, integration tests, and doc tests
  completed with zero failures
- `cargo check --workspace`: pass
- `cargo build -p yunxi-agent-cli`: pass
- `cargo run -p yunxi-agent-cli -- parity map`: pass
- Stage 4K offline JSONL fixtures: pass
  - child provider fixture: 30 JSONL lines, including `child_agent`,
    `child_scoped_stream`, and `storage_state`
  - cancellation fixture: 11 JSONL lines, including `cancelled`,
    `mcp_session`, `child_scoped_stream`, and `storage_state`
  - sandbox fixture: 17 JSONL lines, including sandbox runner output and tool
    completion
  - MCP reuse fixture: 28 JSONL lines, including repeated `mcp_session`
    lifecycle events
  - child scoped stream fixture: 45 JSONL lines, including granular
    `child_scoped_stream` events
- `cargo tree -p yunxi-agent-cli` dependency scan: pass; no default
  `codex`, `vendor`, or `yunxi-agent-codex` dependency was found
- secret-pattern scan across `crates`, `docs`, and `scripts`: pass; no
  API-key-shaped secret, bearer token, or authorization bearer header pattern
  was found
- `git diff --check`: pass with Windows LF/CRLF warnings only

DeepSeek live smoke was first attempted against
`<private-api-file>` without printing or persisting any key, but the
available key candidates were rejected by provider authentication. After the
user added a new key, live smoke was retried on 2026-07-11:

- `api.txt` contained six redacted key candidates; direct DeepSeek balance
  probes returned HTTP 200 for candidates 1 and 2 and HTTP 401 for candidates 3
  through 6.
- stream smoke with `deepseek-v4-flash`: pass, exit code 0, 39 JSONL lines,
  target response text observed, `secret_leak_detected=False`.
- non-stream smoke with `deepseek-v4-flash`: pass, exit code 0, 9 JSONL lines,
  target response text observed, `secret_leak_detected=False`.
- `scripts/provider/deepseek-live-smoke.ps1 -Model deepseek-v4-flash`: pass,
  exit code 0, 39 JSONL lines, `secret_leak_detected=False`.
- `scripts/provider/deepseek-live-smoke.ps1 -Model deepseek-v4-flash
  -NoStream`: pass, exit code 0, 9 JSONL lines,
  `secret_leak_detected=False`.

The Stage 4K owned runtime/provider/tool construction is verified offline, and
the DeepSeek live provider gate has now passed for both streaming and
non-streaming requests with the updated local credential file.

## Stage 4L Planned Direction

The next stage is Codex Agent deep parity closure. Stage 4K proved the
autonomous YunXi chain and the real DeepSeek live provider gate; Stage 4L should
move from runnable autonomy to deeper Codex CLI headless Agent behavior parity.

The Stage 4L development report is recorded in
`docs/reports/2026-07-11-yunxi-stage-4l-codex-agent-deep-parity-closure-development-report.md`.

Stage 4L focuses on:

- thread/session/turn state machine deepening
- provider feature matrix and Responses-style item mapping
- unified exec and exec-server facade
- platform sandbox enforcement facade
- interactive approval and granular permission cache
- MCP auth, elicitation, approval template, capability negotiation, and tools
  cache
- skills/plugins runtime deepening and extension tool executor facade
- context manager, compact, prompt assets, and token budget semantics
- storage, rollout, thread-store, resume, fork, and truncation parity
- multi-agent v2 wait/message/follow-up/interrupt/list and sub-agent activity
  events
- protocol and JSONL full runtime event shape
- disabled-vendor parity harness plus DeepSeek live gate separation

Stage 4L must keep the build-first constraint: migrate and wire the full slice
first, avoid frequent mid-construction validation, then run the final unified
verification gate. Default YunXi CLI dependencies must remain free of
`codex-*`, `vendor/codex-rs`, and `yunxi-agent-codex`.

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

## YunXi Agent v1.7.2 TUI Scroll And Streaming Construction

YunXi Agent v1.7.2 deepens the v1.7.1 TUI host without changing the autonomous
runtime/provider/tool/storage boundary. The default CLI remains independent
from upstream Codex runtime source dependencies.

Implemented on 2026-07-12:

- Workspace package version is promoted to `1.7.2`.
- `yunxi-agent-tui` now has a transcript viewport state that supports mouse
  wheel scrolling, PageUp/PageDown/Home/End navigation, follow-tail behavior,
  and a `new output below` status when model/tool output arrives while the user
  is reading history.
- The TUI host enables mouse capture, drains navigation events during active
  streaming turns, handles resize redraws, and throttles non-critical redraws
  through a frame scheduler.
- Transcript rendering now shows a scroll window selected by the viewport,
  a scrollbar, line range/title status, and a footer that reflects tail/history
  state while keeping composer, approval, and user-input panes fixed.
- Markdown streaming has an explicit stable/live controller for newline-gated
  commit behavior and future table holdback work.
- CLI renderer plumbing adds `tick()` and `flush()` hooks. Plain, one-shot,
  JSON, JSONL, and sessions paths keep their existing behavior.
- `.gitignore` now excludes local `.yunxi/` session runtime output.

Verified on 2026-07-12:

- `cargo fmt`: pass
- `cargo fmt -- --check`: pass
- `cargo test`: pass; workspace unit tests, integration tests, and doc tests
  completed with zero failures
- `cargo check --workspace`: pass
- `cargo check --workspace --target x86_64-unknown-linux-gnu`: blocked by
  missing local Rust target `x86_64-unknown-linux-gnu`
- `cargo build -p yunxi-agent-cli --release --bins`: pass
- `target\release\yunxi.exe --version`: pass; `yunxi 1.7.2`
- `target\release\yunxi-agent-cli.exe --version`: pass; `yunxi 1.7.2`
- Offline one-shot smoke: pass
- Plain interactive `/status` smoke: pass
- DeepSeek live non-stream, stream, and interactive smoke: pass with
  `deepseek-chat`; no secret leak detected
- Default CLI dependency scan: pass; no `yunxi-agent-codex` or `codex-rs`
  dependency in the default CLI tree
- Owned project source secret scan excluding vendor/reference/generated trees:
  pass; zero hits
- `git diff --check`: pass with Windows LF-to-CRLF warnings only
- Install script and PATH smoke: pass; installed `yunxi` reports `1.7.2`
- `codegraph sync .` and `codegraph status .`: pass; index is up to date

Release publication is handled through GitHub REST API only. The release must
create a new annotated tag `v1.7.2` and leave all prior version tags intact.

## YunXi Agent v1.7.3 TUI Event Filtering Construction

YunXi Agent v1.7.3 turns the v1.7.2 scrollable TUI transcript from a raw event
log into a user-facing conversation and tool timeline.

Constructed in this slice:

- Workspace package version is promoted to `1.7.3`.
- `yunxi-agent-tui` now owns an event filter that classifies runtime events
  into user-visible transcript items, debug-only details, tool timeline
  updates, warnings, and errors.
- Raw `CommandUpdated` stdout, provider bookkeeping, context status, storage
  state, and successful advisory sandbox attempts are hidden from the normal
  transcript.
- Tool, shell, approval, escalation, and MCP lifecycle events are merged into
  compact timeline cells with approval/running/completed/failure steps.
- Long skill/tool outputs, protocol JSON, and tool arguments are redacted,
  summarized, and stored in a TUI debug/detail buffer instead of being rendered
  directly in the main transcript.
- TUI users can enable raw debug event summaries with `/debug events on`, hide
  them with `/debug events off`, and inspect the latest or selected redacted
  detail with `/details [id]`.
- Plain, JSON, JSONL, and one-shot output paths remain outside the TUI filter.

Verification for this slice passed on 2026-07-13 after the unified verification
gate completed:

- `cargo fmt`: pass
- `cargo fmt -- --check`: pass
- `cargo test`: pass; workspace unit tests, integration tests, and doc tests
  completed with zero failures
- `cargo check --workspace`: pass
- `cargo build -p yunxi-agent-cli --release --bins`: pass
- `target\release\yunxi.exe --version`: pass; `yunxi 1.7.3`
- `target\release\yunxi-agent-cli.exe --version`: pass; `yunxi 1.7.3`
- TUI event filter regression test: pass; normal transcript hides
  `arguments_json`, full skill documents, and context token status while
  preserving tool timeline steps
- DeepSeek streaming, non-streaming, and interactive live smokes: pass with
  `secret_leak_detected=False`
- Default CLI dependency scan: pass; no `codex`, `vendor`, or
  `yunxi-agent-codex` dependency appeared in the default CLI tree
- Owned-source secret scan: pass after excluding `vendor`, `extracted`,
  `target`, `.git`, and `.codegraph`
- `git diff --check`: pass with Windows LF-to-CRLF warnings only
- `codegraph sync "D:\YunXi Agent"` and `codegraph status "D:\YunXi Agent"`:
  pass; index is up to date
- Install script and PATH smoke: pass; installed `yunxi` reports `1.7.3`

## YunXi Agent v1.5 Honesty Streaming Safety Construction

YunXi Agent v1.5 promotes the project from "usable terminal agent with some
fixture-heavy parity scaffolding" toward a more honest autonomous CLI runtime.

Constructed in this slice:

- Workspace package version is promoted to `1.5.0`.
- `yunxi` and `yunxi-agent-cli` report `yunxi 1.5.0`.
- Interactive banner reports `YunXi Agent v1.5.0 interactive CLI`.
- Offline plain-text assistant output is marked with `[offline]`.
- Auto provider fallback prints an explicit warning when no live credentials are
  configured.
- Offline runtime banner now says the default static provider has stage
  fixtures disabled by default.
- `/cost` reports `n/a - offline, no model call` in offline mode.
- Sandbox/runner display text now uses policy guard/advisory wording and says
  there is no OS isolation in the current v1.5 implementation.
- `StaticProvider` stage fixture branches are disabled by default.
- Runtime `stage 4k/4l/4m` fixture branches are disabled by default.
- Explicit fixture mode is available through `YUNXI_RUNTIME_FIXTURES=1` and
  runtime test builders; fixture metadata includes `fixture_mode=true`.
- Provider transport responses now retain response headers for retry decisions.
- Provider retry uses bounded exponential backoff, honors `Retry-After`, and
  retries timeout/network transport failures within the retry budget.
- Live OpenAI-compatible streaming can consume `reqwest::Response::bytes_stream`
  chunks and push parsed SSE events into the runtime event sink as chunks
  arrive.
- Default CLI dependency boundaries remain YunXi-owned and do not require
  `vendor/codex-rs`, `codex-*`, or `yunxi-agent-codex`.

Verification for v1.5 is intentionally deferred until the full construction
slice is complete, following the project hard constraint to avoid repeated
single-point testing during the build phase.

## YunXi Agent v1.3 Terminal Streaming Approval Cancellation

YunXi Agent v1.3 upgrades the interactive terminal host from batch replay to a
streaming control boundary while preserving the existing one-shot, JSON, JSONL,
storage, and offline fixture paths.

Constructed in this slice:

- Workspace package version is promoted to `1.3.0`.
- `yunxi-agent-core` owns `AgentRunControl`,
  `AgentRunStreamReceiver`, interactive approval request/decision types, and
  interactive user-input request/response types.
- `AgentBackend::run_stream` and `Agent::run_with_backend_stream` expose a
  YunXi-owned streaming run boundary while preserving the old batch `run`
  surface.
- `yunxi-agent-runtime` writes every emitted event to both the returned
  `AgentRunResult` event list and the live event channel.
- Interactive approval and escalation decisions now flow from runtime to the
  CLI and back into the current tool request before execution.
- Interactive `request_user_input` now returns CLI input to the provider tool
  result channel instead of always declining.
- Ctrl+C sets the shared cancellation token; runtime checks it at provider/tool
  loop boundaries, and `yunxi-agent-exec` can kill a running shell child through
  `run_with_cancellation`.
- `yunxi-agent-tools` keeps the old `execute` API and adds
  `execute_with_control` for cancellation-aware tool execution.
- `yunxi-agent-cli` interactive mode renders events live, prompts for approval
  and user input, keeps the REPL alive across failed turns, and reports
  `YunXi Agent v1.3.0 interactive CLI`.
- Terminal rendering now includes file changes, patch status, todo updates,
  completion usage, and a real `/clear`.
- Runtime and exec tests cover streaming-before-completion, approval approve
  and deny, request_user_input, and cancellation propagation to shell exec.

Verified on 2026-07-12:

- `cargo fmt`: pass
- `cargo fmt -- --check`: pass
- `cargo test`: pass; workspace unit tests, integration tests, and doc tests
  completed with zero failures.
- `cargo check --workspace`: pass without warnings.
- `cargo build -p yunxi-agent-cli --release --bins`: pass.
- release dual binaries: pass; both report `yunxi 1.3.0`.
- offline one-shot, offline interactive, offline JSONL, and Stage 4M real
  parity JSONL smoke: pass.
- DeepSeek streaming, non-stream, and interactive live smoke: pass with no
  detected secret leak.
- PATH installed `yunxi 1.3.0`: pass; installed binary SHA-256 matched the
  release build.
- PATH installed DeepSeek live JSONL smoke: pass with no detected secret leak.
- default CLI dependency scan: pass; no `codex-*`, `vendor/codex-rs`, or
  `yunxi-agent-codex` dependency appeared in the default CLI tree.
- owned-source secret scan excluding generated and reference trees: pass.
- `git diff --check`: pass with Windows LF-to-CRLF warnings only.
- `codegraph sync "D:\YunXi Agent"`: pass; 14 changed files synced.

The v1.3 default CLI remains independent from upstream Codex runtime
dependencies. This release does not add TUI, desktop app, cloud tasks, update,
doctor, completion, marketplace, or SDK packaging surfaces.

## YunXi Agent v1.4 Terminal Command Surface Construction

YunXi Agent v1.4 extends the v1.3 interactive terminal runtime with Codex-like
REPL inspection commands. This slice keeps the default CLI independent from
upstream Codex runtime dependencies and focuses on user-visible operational
state inside an active terminal session.

Constructed in this slice:

- Workspace package version is promoted to `1.4.0`.
- `yunxi` and `yunxi-agent-cli` report `yunxi 1.4.0`.
- Interactive banner reports `YunXi Agent v1.4.0 interactive CLI`.
- `/tools` lists YunXi fixed tools and workspace dynamic tools.
- `/mcp` lists workspace MCP configuration and fixture seed presence without
  starting MCP servers.
- `/cost` reports last-turn and session token usage.
- `/status` reports provider/session status, observed runtime events, last-turn
  event summary, tool counts, MCP server count, and usage.
- Interactive turns now keep a compact in-memory summary of event counts,
  usage, final response presence, warnings, errors, approvals, escalations,
  child streams, file changes, tools, commands, and MCP activity.

Verified on 2026-07-12:

- `cargo fmt`: pass.
- `cargo fmt -- --check`: pass.
- `cargo test`: pass after fixing one compile-time event-summary exhaustiveness
  gap for `CommandUpdated`; workspace tests and doc tests completed with zero
  failures.
- `cargo check --workspace`: pass.
- `cargo build -p yunxi-agent-cli --release --bins`: pass.
- release dual binaries: pass; both report `yunxi 1.4.0`.
- offline one-shot, offline interactive command-surface smoke, offline JSONL,
  and Stage 4M real parity JSONL smoke: pass.
- DeepSeek streaming, non-streaming, and interactive live smoke: pass with no
  detected secret leak.
- default CLI dependency scan: pass; no `codex-*`, `vendor/codex-rs`, or
  `yunxi-agent-codex` dependency appeared in the default CLI tree.
- owned-source secret scan excluding generated and reference trees: pass.
- `git diff --check`: pass with Windows LF-to-CRLF warnings only.
- release install and PATH `yunxi --version`: pass; installed binary reports
  `yunxi 1.4.0` and SHA-256 matches the release binary.
- `codegraph sync "D:\YunXi Agent"`: pass; 5 changed files synced.
- Stage 4M temporary `.yunxi/` and `stage4m-runtime.txt` artifacts were removed
  after path-boundary checks.

## YunXi Agent v1.2 DeepSeek Default Provider Construction

YunXi Agent v1.2 connects the existing YunXi-owned DeepSeek transport to the
normal one-shot, interactive, and session-resume CLI paths. Provider selection
is now designed as an explicit auto/live/offline decision rather than the v1.1
`provider_live` boolean being interpreted independently by each CLI surface.

Constructed in this slice:

- Workspace-owned crate versions are promoted to `1.2.0`.
- `yunxi-agent-provider` supports deterministic environment lookup injection,
  automatic DeepSeek profile inference from `DEEPSEEK_API_KEY`, credential
  source precedence, and non-secret credential availability probing.
- `yunxi-agent-cli` owns `provider_mode.rs` and shares one provider decision
  across one-shot, interactive, and `sessions resume` execution.
- Default `auto` mode selects a live provider when credentials are configured
  and otherwise selects the labelled offline provider.
- `--provider-live` forces live mode and fails before HTTP execution when the
  selected provider has no credentials.
- `--offline` forces deterministic offline execution and conflicts with
  `--provider-live`.
- Interactive banner and `/session` output show provider mode, resolution
  source, provider name, and model without showing authentication values.
- Live selections materialize the resolved provider and model into each turn's
  `AgentConfig`, so turn metadata, session storage, and resume preserve the
  actual DeepSeek configuration; offline selections leave config unchanged.
- Existing deterministic CLI and JSONL tests explicitly select `--offline`, so
  a developer's user-level API credentials cannot make the suite contact a
  real model endpoint.
- `scripts/provider/import-deepseek-credential.ps1` imports one local
  credential candidate into user environment variables without printing it;
  multi-candidate files require an explicit one-based candidate index.
- `scripts/provider/deepseek-live-smoke.ps1` requires an explicit `-ApiFile`;
  the repository no longer contains a personal credential-file path.
- README documents automatic DeepSeek selection, explicit overrides, secure
  import, immutable version tags, and GitHub API-only publication.

Construction followed the project hard constraint: no tests, checks, builds,
formatters, or live requests were run while the slice was being wired. The
single unified v1.2 verification gate then completed on 2026-07-11.

Verified on 2026-07-11:

- `cargo fmt` and `cargo fmt -- --check`: pass.
- `cargo test`: pass; 190 workspace tests completed with zero failures.
- `cargo check --workspace`: pass without warnings after removing one stale
  CLI import found by the first unified run.
- `cargo build -p yunxi-agent-cli --release --bins`: pass.
- Both release binaries report `yunxi 1.2.0`.
- Offline one-shot, piped interactive, argument-conflict, missing-prompt JSONL,
  parity map, and `git diff --check` gates: pass.
- Default CLI dependency scan: pass; no `vendor/codex-rs`, `codex-*`, or
  `yunxi-agent-codex` dependency is present.
- Owned-source scan: 108 release-scope files, zero secret-pattern matches, and zero personal
  API-file-path matches. Reference-only `vendor` and `extracted` trees were
  excluded by normalized path segment rather than slash-sensitive regex.
- DeepSeek stream smoke: pass; 48 JSONL events and no detected secret leak.
- DeepSeek non-stream smoke: pass; 19 JSONL events and no detected secret leak.
- Default auto one-shot: pass; returned real DeepSeek assistant content and did
  not return the offline fixture response.
- Auto JSONL metadata: pass; 48 valid JSONL events included two turn metadata
  events with provider `deepseek` and model `deepseek-v4-flash`.
- Default auto interactive startup: pass; reported `live`, `auto_live`, and
  `deepseek` without exposing credentials.
- Multi-candidate credential import: pass; missing explicit index was rejected,
  index 1 imported successfully, and output leak detection was false.
- Installed PATH binary verification: pass; its SHA-256 matched the final release
  build and installed v1.2 used the user-level DeepSeek environment for both
  interactive startup and a real one-shot turn.

Release closure:

- `v1.2.0` was created from the final release commit only after the unified
  gate passed; `v1.0.0` and `v1.1.0` remain unchanged.
- The default CLI remains independent from `vendor/codex-rs`, `codex-*`, and
  `yunxi-agent-codex`.
- GitHub master/tree/commit/tag state was published and verified only through
  the GitHub REST API; Git transport push/fetch commands were not used.
- The final source index and desktop development log were synchronized, then
  generated `target`, root `.yunxi`, and temporary smoke artifacts were
  cleaned.

## YunXi Agent v1.2.1 Interactive Provider Recovery Construction

YunXi Agent v1.2.1 corrects the interactive turn lifecycle exposed by a
DeepSeek HTTP 400 response. A failed provider, tool, or network turn is now
contained at the REPL boundary: the error is rendered safely, the last
successful session state is preserved, and the next user input can run without
restarting YunXi.

DeepSeek 400/422 responses receive one compatibility fallback that removes only
the optional top-level request `metadata`. Tools, messages, model selection,
and all other provider behavior remain intact. If the fallback also fails,
YunXi extracts only structured error message/type/code fields, redacts
token-like content, and bounds the displayed detail to 240 characters.

Tool-loop history now preserves the assistant `tool_calls` message and sends
each `role=tool` result with its matching `tool_call_id`. This closes the
protocol defect identified by DeepSeek's structured error detail after the
initial interactive recovery patch was installed.

The live smoke helper now has a genuine interactive mode that sends an actual
prompt and requires the expected assistant marker plus explicit `/exit`
completion. Provider tests cover the fallback and safe diagnostics; a CLI
integration test covers first-turn 400/400 failure followed by a successful
second prompt without process termination.

This patch is prepared for immutable tag `v1.2.1`; release must preserve
`v1.0.0`, `v1.1.0`, and `v1.2.0` unchanged. The default CLI remains
independent of upstream Codex runtime dependencies.

Verified on 2026-07-12:

- `cargo test`: pass; 196 workspace tests, zero failures.
- `cargo check --workspace`: pass without warnings.
- release dual binaries: pass; both report `yunxi 1.2.1`.
- local sequential HTTP REPL recovery: pass.
- DeepSeek stream/non-stream/interactive prompt: pass with no detected secret
  leak.
- installed PATH binary from `C:\Users\admin`: “你好” produced a completed
  assistant turn and exited only after `/exit`, with no provider HTTP 400.
- default CLI dependency tree and owned-source privacy scans: pass.

## Stage 4L Deep Parity Closure Construction

Stage 4L construction has added YunXi-owned facade and runtime fixture coverage
for the 12 deep Codex CLI headless Agent parity surfaces described in
`docs/reports/2026-07-11-yunxi-stage-4l-codex-agent-deep-parity-closure-development-report.md`.

Constructed in this slice:

- `yunxi-agent-core` now owns thread state, turn metadata, turn state, and
  deep parity state event carriers.
- `yunxi-agent-protocol` now exposes `thread_state`, `turn_state`, and
  `deep_parity_state` JSONL runtime events alongside the existing stable
  runtime envelope.
- `yunxi-agent-cli` maps the new core events into protocol JSONL without
  changing older event shapes.
- `yunxi-agent-runtime` now has an offline
  `stage 4l deep parity fixture` path that emits all 12 closure layers, plus
  exec, approval, sandbox escalation, MCP lifecycle, skills, context,
  storage, multi-agent, and child scoped stream events.
- `yunxi-agent-provider` owns a `ProviderFeatureMatrix` facade for
  provider-neutral item mapping, capabilities, schema strictness, and retry
  buckets.
- `yunxi-agent-exec` owns `UnifiedExecRequest`, `UnifiedExecAttempt`,
  `ExecServerSession`, and shell snapshot facade types.
- `yunxi-agent-sandbox` owns sandbox attempt and session approval cache facade
  types.
- `yunxi-agent-mcp` owns capability negotiation and long-lived session facade
  types for auth, elicitation, tools/resources cache, and session reuse.
- `yunxi-agent-skills` owns runtime catalog and extension tool executor facade
  types.
- `yunxi-agent-context` owns `ContextManagerState` for prompt assets, compact
  state, token budget, and prompt debug snapshots.
- `yunxi-agent-storage` owns thread-store and rollout parity snapshot facade
  types.
- `yunxi-agent-multi-agent` owns mailbox, communication kind, activity, and
  budget sharing facade types.
- Runtime, CLI JSONL, and protocol round-trip tests were added for the Stage 4L
  fixture and new event shapes.

Stage 4L verification was run after construction completed, following the
project hard constraint to avoid mid-construction test runs.

Verified on 2026-07-11:

- `cargo fmt`: pass
- `cargo fmt -- --check`: pass
- `cargo test`: pass
- `cargo check --workspace`: pass
- `cargo build -p yunxi-agent-cli`: pass
- `cargo run -p yunxi-agent-cli -- parity map`: pass
- `cargo run -p yunxi-agent-cli -- --backend yunxi --jsonl "run stage 4l deep parity fixture"`:
  pass
- Stage 4L fixture JSONL count: 45 lines
- Stage 4L fixture event counts:
  `deep_parity_state:12`, `turn_state:3`, `mcp_session:4`,
  `child_scoped_stream:3`, `tool_started:3`, `tool_completed:2`, plus
  thread, turn, metadata, approval, escalation, context, storage, multi-agent,
  child-agent, item, item-delta, and completion events.
- `cargo tree -p yunxi-agent-cli`: pass
- Default CLI dependency keyword scan: pass; no `codex-*`, `vendor`, or
  `yunxi-agent-codex` dependency appeared in the default CLI tree.
- Owned-source secret scan excluding reference-only `vendor` and `extracted`
  trees: pass.
- A broader reference-inclusive scan still reports four pre-existing
  `extracted/codex-core-agent-sources` files containing bearer-header code
  literals from upstream reference source. No secret value was printed and no
  Stage 4L owned source file matched.
- `git diff --check`: pass with Windows LF-to-CRLF warnings only.
- `scripts/provider/deepseek-live-smoke.ps1 -Model deepseek-v4-flash`: pass;
  39 JSONL lines, no secret leak detected.
- `scripts/provider/deepseek-live-smoke.ps1 -Model deepseek-v4-flash -NoStream`:
  pass; 9 JSONL lines, no secret leak detected.
- `codegraph sync "D:\YunXi Agent"`: pass; 19 changed files synced.
- `cargo clean`: pass; removed 7429 files and 1.6GiB.
- Root `.yunxi` cleanup: pass.

## Stage 4M Planned Direction

The next stage is real runtime parity deepening. Stage 4L proved the deep
parity facade, JSONL shape, and synthetic mega fixture. Stage 4M should connect
those facade surfaces to real YunXi runtime behavior: runtime drivers,
provider feature matrix request shaping, unified exec handles, sandbox attempt
records, approval cache, MCP long-lived sessions, skills/plugins runtime
catalog, context auto-compact, rollout-backed resume, multi-agent v2 routing,
and a real parity mega fixture.

The Stage 4M development report is recorded in
`docs/reports/2026-07-11-yunxi-stage-4m-real-runtime-parity-deepening-development-report.md`.

Stage 4M execution should keep the existing hard constraint: build and wire the
whole slice first, avoid mid-construction verification, then run the final
verification gate once construction is complete.

## Stage 4M Real Runtime Parity Construction

Stage 4M construction is being implemented against
`docs/reports/2026-07-11-yunxi-stage-4m-real-runtime-parity-deepening-development-report.md`.

Constructed in this slice:

- `yunxi-agent-runtime` now has small YunXi-owned runtime driver modules for
  thread state, turn metadata, turn phase state, and session approval-cache
  probing.
- Normal YunXi runtime turns now emit `thread_state`, `turn_metadata`, and
  `turn_state` events from the main path instead of reserving those events for
  the Stage 4L synthetic fixture.
- Provider calls now publish feature-matrix driven turn-state data, and
  `yunxi-agent-provider` uses `ProviderFeatureMatrix` while shaping
  OpenAI-compatible request JSON.
- Tool execution now maps sandbox runner diagnostics into a structured
  `sandbox_attempt` runtime event and maps approval cache decisions into
  `approval_cache_state`.
- Context assembly now records a `ContextManagerState` projection for AGENTS.md
  fragments, file mentions, restored history, token estimates, and compact
  summary metadata.
- A new `stage 4m real parity fixture` follows the real runtime chain:
  provider -> context -> approval cache -> sandbox decision -> shell exec ->
  patch -> MCP -> skill -> tool search -> multi-agent child -> storage ->
  JSONL.
- The Stage 4L synthetic fixture remains available as the protocol-shape
  fallback, while Stage 4M adds 12 `deep_parity_state` summary layers derived
  from events observed on the real runtime path.

Stage 4M final unified verification passed on 2026-07-11 after construction
completed, following the project hard constraint to avoid mid-construction
test loops.

Verified on 2026-07-11:

- `cargo fmt`: pass
- `cargo fmt -- --check`: pass
- `cargo test`: pass; workspace unit tests, integration tests, and doc tests
  completed with zero failures
- `cargo check --workspace`: pass
- `cargo build -p yunxi-agent-cli`: pass
- `cargo run -p yunxi-agent-cli -- parity map`: pass
- `cargo run -p yunxi-agent-cli -- --backend yunxi --jsonl "run stage 4m real parity fixture"`:
  pass; command exit code 0
- Stage 4M real fixture pure JSONL count: 131 events
- Stage 4M real fixture event counts:
  `deep_parity_state=12`, `turn_state=9`, `sandbox_attempt=7`,
  `approval_cache_state=8`, `mcp_session=7`, `child_scoped_stream=15`,
  `tool_started=7`, `tool_completed=5`
- `cargo run -p yunxi-agent-cli -- --backend yunxi --jsonl "run stage 4l deep parity fixture"`:
  pass; command exit code 0, 48 pure JSONL events
- `cargo tree -p yunxi-agent-cli`: pass
- Default CLI dependency keyword scan: pass; no `vendor/codex-rs`,
  `codex-*`, or `yunxi-agent-codex` dependency appeared in the default CLI
  tree
- Owned-source secret scan excluding `vendor`, `extracted`, `target`, `.git`,
  and `.codegraph`: pass; no API-key-shaped secret, bearer token, or
  authorization bearer header pattern was found
- `git diff --check`: pass with Windows LF-to-CRLF warnings only
- `scripts/provider/deepseek-live-smoke.ps1 -Model deepseek-v4-flash`: pass;
  exit code 0, 49 JSONL lines, `secret_leak_detected=False`
- `scripts/provider/deepseek-live-smoke.ps1 -Model deepseek-v4-flash -NoStream`:
  pass; exit code 0, 19 JSONL lines, `secret_leak_detected=False`

Stage 4M therefore moves the Stage 4L synthetic deep-parity facade into a
real YunXi runtime parity chain for provider, context, approval cache,
sandbox attempt records, tools, MCP reuse, skills, child scoped streams,
storage, protocol JSONL, and 12-layer deep parity summaries while keeping the
default CLI independent from upstream Codex runtime dependencies.

## YunXi Agent v1.6 Policy Guard Hardening Construction

YunXi Agent v1.6 promotes the v1.5 policy-guard honesty work into stronger
process-internal enforcement. The release still does not claim OS-level sandbox
isolation, but the default shell tool path no longer bypasses the configured
`ToolPolicy` with trusted `DangerFullAccess`.

Constructed in this slice:

- Workspace package version is promoted to `1.6.0`.
- One-shot, JSON, JSONL, and `sessions resume` paths now surface the same auto
  provider fallback warning as interactive mode when no live credentials are
  configured.
- `yunxi-agent-tools` passes the request `ToolPolicy.execution_policy` into
  `ExecCommand::shell` for default shell execution.
- `yunxi-agent-sandbox` adds shell-ish tokenization, token-aware command risk
  classification, command target extraction, and workspace-write target escape
  blocking for common redirection/copy/move/delete/write forms.
- `yunxi-agent-provider` prevents streaming retry and DeepSeek schema fallback
  after body bytes have reached the stream parser, avoiding duplicated stream
  events after partial output.
- The unused non-Windows `platform_shell` helper in `yunxi-agent-tools` was
  removed so cross-target compilation no longer depends on a dead missing
  import.

Verification for this slice is recorded in the v1.6 development report after
the unified verification gate completes.

## YunXi Agent v1.7 Terminal TUI Foundation Construction

YunXi Agent v1.7 promotes the terminal host from a plain `read_line` REPL into
a TUI-capable interactive shell while preserving script-oriented output. This
release also adds symlink escape regression coverage for the v1.6
process-internal workspace-write guard.

Constructed in this slice:

- Workspace package version is promoted to `1.7.0`.
- `yunxi-agent-cli` gains terminal dependencies for `ratatui`, `crossterm`, and
  `reedline`.
- Interactive startup resolves a terminal mode: real terminal stdin/stdout use
  the TUI-capable host by default, while pipes, JSON, JSONL, and `--no-tui`
  remain on the stable plain path.
- CLI input is split behind an `InteractiveInput` abstraction with a plain
  line reader and a Reedline-backed terminal reader.
- Interactive rendering is split behind an `InteractiveRenderer` abstraction
  with the existing plain renderer and a basic TUI renderer.
- The TUI app state renders a compact header, event log, and input area and is
  covered by `ratatui::backend::TestBackend` tests.
- `yunxi-agent-sandbox` now resolves the nearest existing path prefix before
  falling back to component normalization, allowing symlink parent directories
  to be detected even when the final target file does not exist yet.
- Sandbox and tools tests include symlink escape regressions where the platform
  allows symlink creation.

Verification for this slice is recorded in the v1.7 development report. The
2026-07-12 gate passed `cargo fmt`, `cargo fmt -- --check`, `cargo test`,
`cargo check --workspace`, release build, version checks, plain interactive
smoke, auto fallback smoke, DeepSeek live JSON/JSONL smoke, dependency scan,
secret scan, `git diff --check`, CodeGraph sync, release install, and PATH
smoke. Linux cross-target check is recorded as environment-blocked because the
configured rustup mirror returned 404 for `x86_64-unknown-linux-gnu` std.

## YunXi Agent v1.0 CLI Packaging

YunXi Agent v1.0 packages the current verified autonomous runtime as a
terminal-first CLI product.

Constructed in this slice:

- Workspace package version is promoted to `1.0.0`.
- `yunxi-agent-cli` now builds the primary `yunxi` binary and keeps
  `yunxi-agent-cli` as a compatibility binary.
- CLI metadata now presents the command as `YunXi Agent v1.0 terminal CLI`.
- `scripts/install/install-yunxi.ps1` installs release binaries into a
  user-local bin directory and can optionally add that directory to the user
  PATH.
- README now documents v1.0 install, source run, installed run, and
  environment-based live provider usage.

The v1.0 CLI package is still intentionally headless. It does not add TUI,
desktop, cloud task, updater, doctor, completion, marketplace, or installer
surfaces beyond the local PowerShell install helper.

## YunXi Agent v1.1 Planned Direction

The next product stage is YunXi Agent v1.1 interactive CLI. The v1.0 package is
installable and callable from PowerShell, but its default command surface is
still a headless one-shot runner: `yunxi "prompt"` runs one turn, while `yunxi`
without a prompt reports `a prompt is required`.

YunXi Agent v1.1 should make `yunxi` without a prompt enter an interactive
terminal session while preserving one-shot script compatibility. The v1.1
development report is recorded in
`docs/reports/2026-07-11-yunxi-agent-v1-1-interactive-cli-development-report.md`.

The v1.1 implementation should connect the already autonomous runtime/provider
chain to a terminal host with:

- REPL input loop and slash commands.
- Stable one-shot, JSON, and JSONL compatibility.
- Session id reuse, history restore, and `/resume`.
- Live provider configuration through replaceable provider interfaces.
- Streaming terminal renderer for assistant deltas and tool lifecycle events.
- Ctrl+C cancellation for the active turn without losing the whole REPL.
- Interactive approval and escalation prompts.
- MCP, skills, and child scoped stream visibility in terminal output.
- Version/documentation/install updates for `1.1.0`.

The same project constraints continue to apply: default `yunxi` must not depend
on `vendor/codex-rs`, `codex-*`, or `yunxi-agent-codex`; API keys must never be
printed or committed; construction should be done as one build slice and the
full verification gate should run only after the slice is wired.

## Version Tagging Policy

Starting with the v1.1 work, every version change must create a new Git tag and
must keep all previous version tags. Tags are rollback anchors and must not be
deleted during normal development or release cleanup.

## YunXi Agent v1.1 Interactive CLI Construction

YunXi Agent v1.1 upgrades the v1.0 headless one-shot CLI into an interactive
terminal CLI while preserving script-oriented one-shot, JSON, and JSONL
compatibility.

Constructed in this slice:

- Workspace package version is promoted to `1.1.0`.
- `yunxi` and `yunxi-agent-cli` now report `yunxi 1.1.0`.
- `yunxi` without a prompt enters an interactive REPL instead of returning
  `a prompt is required`.
- One-shot mode remains available through `yunxi "prompt"`.
- `--json` and `--jsonl` without a prompt continue to return structured
  invalid-input errors instead of entering the REPL.
- `yunxi-agent-cli` owns small CLI-side modules for interactive command parsing,
  terminal rendering, and REPL/session orchestration.
- Interactive mode supports `/help`, `/session`, `/resume <session_id>`,
  `/model`, `/provider`, `/cwd`, `/clear`, `/exit`, and `/quit`.
- Interactive turns reuse the existing YunXi-owned runtime and session storage
  path, carrying the active session id forward through parent-linked history.
- Terminal rendering now shows assistant messages, reasoning, shell/tool/MCP
  lifecycle events, approval/escalation events, child scoped stream events,
  context status, storage state, warnings, provider errors, and cancellation
  status in a readable non-JSON form.
- Ctrl+C is wired at the CLI host boundary to cancel the active turn future and
  return control to the REPL.
- Live provider interactive startup checks that a provider key source exists
  before entering the REPL, without printing key values.
- README documents v1.1 interactive usage and the version tag policy.
- The user-local PATH install was refreshed with the v1.1 release binaries.

Verified on 2026-07-11:

- `cargo fmt`: pass
- `cargo fmt -- --check`: pass
- `cargo test`: pass; workspace unit tests, integration tests, and doc tests
  completed with zero failures
- `cargo check --workspace`: pass
- `cargo build -p yunxi-agent-cli --release --bins`: pass
- `target\release\yunxi.exe --version`: pass; `yunxi 1.1.0`
- `target\release\yunxi-agent-cli.exe --version`: pass; `yunxi 1.1.0`
- `target\release\yunxi.exe --backend yunxi "YunXi Agent v1.1 one-shot smoke"`:
  pass
- Piped interactive smoke with `你好`, `/session`, and `/exit`: pass
- Piped `/help` and `/exit` smoke: pass
- `target\release\yunxi.exe --jsonl` without prompt: expected invalid-input
  exit code `2` with structured JSONL error output
- `cargo run -p yunxi-agent-cli -- parity map`: pass
- `cargo tree -p yunxi-agent-cli`: pass
- Default CLI dependency keyword scan: pass; no `vendor/codex-rs`,
  `codex-*`, or `yunxi-agent-codex` dependency appeared in the default CLI
  tree
- Owned-source secret scan excluding generated and reference trees: pass; no
  API-key-shaped secret, bearer token, or authorization bearer header pattern
  was found
- `git diff --check`: pass with Windows LF-to-CRLF warnings only
- `scripts/provider/deepseek-live-smoke.ps1 -Model deepseek-v4-flash`: pass;
  exit code 0, 49 JSONL lines, `secret_leak_detected=False`
- `scripts/provider/deepseek-live-smoke.ps1 -Model deepseek-v4-flash -NoStream`:
  pass; exit code 0, 19 JSONL lines, `secret_leak_detected=False`
- `scripts/install/install-yunxi.ps1 -AddToPath -SkipBuild`: pass;
  `path_updated=False`
- Installed `C:\Users\admin\AppData\Local\YunXi Agent\bin\yunxi.exe --version`:
  pass; `yunxi 1.1.0`
- PATH-resolved `yunxi --version`: pass; `yunxi 1.1.0`
- PATH-resolved piped `/exit` interactive smoke: pass
- `codegraph sync "D:\YunXi Agent"`: pass; 5 changed files synced
- `cargo clean`: pass; removed 6934 files and 1.9GiB
- Root `.yunxi` cleanup: pass

The v1.1 default CLI dependency graph remains independent from upstream Codex
runtime dependencies. This release adds the interactive terminal host layer on
top of the already autonomous YunXi runtime; it does not introduce TUI,
desktop app, cloud tasks, update, doctor, completion, marketplace, or new
installer surfaces beyond the existing local PowerShell install helper.

## YunXi Agent v1.7.4 Planned Direction

The next TUI stage is the v1.7.4 scrollbar and viewport correction slice. The
v1.7.3 TUI now filters protocol noise and shows a cleaner transcript, but user
testing shows that the transcript scrollbar can appear stuck around the middle
and cannot be dragged with the mouse.

The v1.7.4 development report is recorded in
`docs/reports/2026-07-13-yunxi-agent-v1-7-4-tui-scrollbar-viewport-development-report.md`.

The planned implementation should:

- Replace transcript logical-line scrolling with wrapped-screen-row scrolling.
- Share one TUI layout geometry between rendering and mouse hit-testing.
- Add transcript scrollbar geometry and mouse drag state.
- Keep wheel, PageUp/PageDown, Home/End, and tail-follow behavior working.
- Keep v1.7.3 event filtering, tool timeline, and debug/details behavior intact.
- Continue avoiding default runtime dependencies on `vendor/codex-rs`,
  `codex-*`, or `yunxi-agent-codex`.

The implementation stage should keep the project hard constraint: build the
whole slice first, avoid repeated mid-construction validation loops, then run
the unified verification gate and publish a new immutable `v1.7.4` tag only
after source implementation and verification complete.

## YunXi Agent v1.7.4 TUI Scrollbar Viewport Construction

YunXi Agent v1.7.4 promotes the TUI transcript scroll model from logical lines
to wrapped screen rows and adds mouse-drag scrollbar control.

Constructed in this slice:

- Workspace package version is promoted to `1.7.4`.
- `yunxi-agent-tui` now owns a shared `layout` module so rendering and mouse
  hit-testing use the same header, transcript, transcript-inner, scrollbar, and
  bottom-pane rectangles.
- `yunxi-agent-tui` now owns a `transcript_layout` module that converts
  filtered transcript cells into styled, pre-wrapped screen rows before
  viewport slicing.
- TUI transcript rendering no longer relies on `Paragraph.wrap` for the main
  transcript content; title ranges and scrollbar state now use wrapped row
  counts.
- `TranscriptViewport` keeps tail-follow bottom-offset behavior while adding
  explicit start-row and scroll-fraction setters for precise scrollbar drag.
- `yunxi-agent-tui` now owns a `scrollbar` geometry module for thumb sizing,
  hit-testing, and drag-y to start-row mapping.
- The TUI host handles left-button scrollbar down/drag/up, track page clicks,
  and transcript-scoped mouse wheel events without changing composer and
  approval overlay input behavior.
- README and the v1.7.4 development report document the wrapped-row scrollbar
  correction.

Unified verification passed on 2026-07-13 after construction completed,
following the project hard constraint to avoid mid-construction test loops.

Verified in this slice:

- `cargo fmt`: pass
- `cargo fmt -- --check`: pass
- `cargo test`: pass; workspace unit tests, integration tests, and doc tests
  completed with zero failures
- `cargo check --workspace`: pass
- `cargo build -p yunxi-agent-cli --release --bins`: pass
- `target\release\yunxi.exe --version`: pass; `yunxi 1.7.4`
- `target\release\yunxi-agent-cli.exe --version`: pass; `yunxi 1.7.4`
- `target\release\yunxi.exe --offline "v1.7.4 offline smoke"`: pass
- Plain `--no-tui` interactive smoke: pass
- JSON and JSONL offline smoke: pass
- `cargo test -p yunxi-agent-tui`: pass; 31 TUI tests covered wrapped rows,
  scrollbar geometry, viewport start/fraction mapping, shared layout metrics,
  and transcript rendering
- DeepSeek live stream smoke with `deepseek-chat`: pass; 28 JSONL lines,
  `secret_leak_detected=False`
- DeepSeek live non-stream smoke with `deepseek-chat`: pass; 19 JSONL lines,
  `secret_leak_detected=False`
- Default CLI dependency keyword scan: pass; no `codex`, `vendor`, or
  `yunxi-agent-codex` matches
- Owned-source secret scan excluding `vendor`, `extracted`, `target`, `.git`,
  and `.codegraph`: pass
- `git diff --check`: pass with Windows LF-to-CRLF warnings only
- `codegraph sync "D:\YunXi Agent"`: pass; 12 changed files synced
- `codegraph status "D:\YunXi Agent"`: pass; index is up to date
- Install script and PATH smoke: pass; installed `yunxi` and
  `yunxi-agent-cli` report `1.7.4`

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

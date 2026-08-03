# YunXi Agent v2.3.3-hotfix.6

YunXi Agent v2.3.3-hotfix.6 is a local-first Rust general companion Agent with terminal, Web, and Weixin entry points plus a reusable core library.
The default runtime is YunXi-owned and does not depend on the upstream Codex
runtime.

When DeepSeek credentials are configured, the default `yunxi` command now uses
the real DeepSeek provider automatically. Without credentials it remains usable
through the inline-labelled offline provider; offline assistant output is marked
with `[offline]` and `/cost` reports that no model call was made.

## Development Track

The current `v2.3.3-hotfix.6` development workspace is the merged Weixin and local Web integration
line. It keeps the v2.1.x QR login, Windows Credential Manager secret boundary,
atomic `WeixinStateStore`, stale account-lock recovery, pair lifecycle,
authenticated encrypted pending inbound, and existing Runtime session binding.
The merged line now adds stable parent/history recovery for Weixin turns,
QueueFull Ready-pending drain and restart recovery, foreground slash-command
`AgentRunControl` bridging for `/status`, `/stop`, `/approve`, `/deny`, and
`/answer`, safe outbound prompts/outcomes, `final_text_only` AgentEvent
observation for Weixin output, encrypted final-text delivery manifest/spool with
iLink `sendmessage`, background runtime-dispatch lease/shutdown/panic recovery,
and `yunxi weixin session reset --account ... --peer ... --confirm` archive
semantics. The work remains in development until unified review, full regression,
real iLink/Provider validation, ConPTY verification, evidence sanitization, and
the final annotated `v2.2.0` release gate pass. Group chat, public callbacks,
background daemons, contact scraping, auto-add, group send, and unverified
proactive Weixin push remain outside the completed scope.
See the [documentation index](docs/README.md) for the canonical roadmap,
governance baseline, reports, evidence, and engineering entry points.

## Layout

- `crates/yunxi-agent-core`: reusable Agent facade and extraction boundary
- `crates/yunxi-agent-companion`: pure Rust proactive companion policy and planner
- `crates/yunxi-agent-eval`: offline companion scenario runner and metric aggregator
- `crates/yunxi-agent-provider`: YunXi-owned provider request/response boundary
- `crates/yunxi-agent-persona`: YunXi-owned persona, transparent memory schema,
  recall, extraction, and write policy boundary
- `crates/yunxi-agent-tools`: YunXi-owned tool execution boundary
- `crates/yunxi-agent-storage`: YunXi-owned session, control-audit, companion-history, and v2.2.0 Weixin state storage/conversation-binding/delivery/remote-control boundary
- `crates/yunxi-agent-runtime`: YunXi-owned Agent runtime boundary
- `crates/yunxi-agent-codex`: standalone compatibility layer around the vendored Codex headless runtime
- `crates/yunxi-agent-tui`: YunXi-owned terminal TUI presentation boundary, quiet transcript, composer, and approval overlay
- `crates/yunxi-agent-weixin`: v2.2.0 QR login state machine, system credential boundary, non-secret account metadata, legacy metadata initialization boundary, iLink model/fixed-endpoint client, foreground getupdates serve loop, inbound envelope normalization, authenticated pending payload encryption/recovery, pairing admission, idempotency, Runtime session binding supervisor, bounded per-conversation dispatch queue, delivery spool, slash-command control, and deterministic mock boundary
- `crates/yunxi-agent-cli`: v2.3.3 terminal and local Web package that builds `yunxi`
  and the compatibility `yunxi-agent-cli`
- `vendor/codex-rs`: vendored Codex Rust workspace source used by `codex-native`
- `docs/README.md`: stable documentation index, report archive policy, and current roadmap/audit entry points
- `docs/extraction-status.md`: current extraction status and known gaps
- `docs/tui-presentation.md`: v2.1.0 integrated TUI/streaming regression, terminal recovery, responsive density, focus routing, and quiet transcript boundary
- `docs/superpowers/specs`: design specs
- `docs/superpowers/plans`: implementation plans

## Current Capabilities

- Compiles as an independent Rust workspace
- Builds a terminal command named `yunxi`
- Serves a local-only Web console with shared chat, long-term memory, persona,
  companion, approval, and runtime boundaries through `yunxi web`
- Starts an interactive terminal session when `yunxi` is run without a prompt
- Uses a Codex-style TUI terminal host by default when stdin/stdout are both
  real terminals, with mouse wheel/PageUp/PageDown/Home/End transcript
  navigation, draggable transcript scrollbar, and `--no-tui` available for the
  stable plain REPL
- Routes mouse wheel and scrollbar input by active focus: Composer/History may
  navigate transcript history, Approval freezes the underlying viewport, and
  Details scrolls only its independent redacted detail offset
- Uses a TUI composer for terminal input while keeping piped stdin and scripted
  sessions on the plain line reader
- Resolves interactive TUI, plain, pipe, CI, one-shot, command, JSON, JSONL,
  `--no-tui`, and `--tui` fallback through one explicit mode matrix; all
  non-TUI paths are byte-tested to exclude ANSI, alternate-screen, and TUI
  footer output
- Uses one grapheme-indexed `EditBuffer` for Composer and `request_user_input`,
  including CRLF normalization, multi-line/long paste, CJK, combining text,
  emoji ZWJ, Home/End, deletion, snapshot, and restore semantics
- Preserves an editable next-turn draft during streaming and across approval or
  user-input overlays; final/completed updates cannot create a second assistant
  cell or overwrite the draft
- Renders approval and `request_user_input` inside the TUI bottom pane instead
  of leaking line prompts into the alternate screen
- Collapses each tool call into one stable activity cell, keeps approval in a
  decline-by-default bottom pane, and exposes command diagnostics through
  details/debug without leaking raw payloads into the normal transcript
- Decodes command stdout/stderr from bytes with safe binary, invalid UTF-8, and
  truncation metadata while preserving JSON/JSONL event compatibility
- Keeps the TUI composer fixed while the transcript can scroll through history
  without losing new streamed output below
- Scrolls the TUI transcript by pre-wrapped screen rows, so long Chinese,
  English, assistant, tool-summary, and detail output keeps the title,
  viewport, and scrollbar in sync
- Coalesces high-rate stream deltas on a 33,334 microsecond minimum frame interval while input,
  history navigation, resize, cancellation, and errors remain immediately visible
- Anchors history review to a stable transcript cell and wrapped-line offset, so
  stream finalization and terminal width/height changes do not force the view
  back to the tail
- Routes every runtime event through one TUI presentation boundary before it
  can become a transcript cell
- Keeps the default transcript quiet: raw reasoning, memory/context internals,
  hidden prompts, provider wire data, tool arguments, full stdout/stderr, and
  exception stacks remain in debug/details only
- Maps provider thread, turn, message stream, stable event ID, and explicit
  provider-reliable/local-fallback sequence source into one canonical assistant
  cell; the timeline deduplicates by event ID without deleting legitimate
  repeated text
- Keeps committed Markdown source separate from the live tail, commits at
  complete line, paragraph, closed-fence, or safe grapheme boundaries, and lets
  final update the active cell in place without forcing history view back to tail
- Bounds live stream tails, finalized stream content, history cells, detail/debug
  records, tool fields, event-ID deduplication, and archived sessions while
  retaining recent grapheme-aligned content with a visible truncation marker
- Shows tool work as compact timeline cells with approval, running,
  completion, output-summary, and details references
- Provides `/debug events on|off` and `/details [id]` for TUI diagnostics
  without polluting the default transcript
- Streams runtime events to the terminal while an interactive turn is running
- Prompts for interactive approval and `request_user_input` tool calls
- Polls raw-mode Ctrl+C during an active TUI turn and propagates cancellation
  into the provider future, runtime, and shell exec layer without exiting the REPL
- Provides REPL status commands for tools, MCP config, usage, and turn summaries
- Keeps the interactive session open when one provider or tool turn fails
- Restores raw mode, alternate screen, bracketed paste, focus tracking, mouse
  capture, and cursor state in reverse order after normal exit, Ctrl+C, partial
  terminal entry failure, Provider failure, tool failure, panic, or early return
- Preserves assistant tool calls and matching tool result ids across provider turns
- Preserves one-shot execution through `yunxi "your task"`
- Provides explicit `yunxi run ...` one-shot execution for prompts that would
  otherwise conflict with reserved subcommands such as `sessions` or `parity`
- Automatically selects the real DeepSeek provider when credentials are present
- Supports `--offline` for deterministic local and automation runs
- Marks offline assistant output inline with `[offline]`
- Warns in interactive, one-shot, JSON, JSONL, and resume paths when auto
  provider mode falls back to the offline static runtime
- Avoids provider fallback warnings for explicitly selected non-provider
  backends such as `dry-run`
- Shows offline runtime as a static provider with stage fixtures disabled by default
- Uses policy-only/process-lifecycle wording for sandbox decisions; v1.7.8
  does not claim OS-level sandbox isolation unless a runner reports
  `enforcement=os_restricted` and `os_isolation=true`, and emits
  machine-readable `schema_version`, `backend_id`, `backend_label`,
  `os_isolation`, `enforcement`, compatibility `enforcement_level`, `runner`,
  and `unsupported_reason` fields on sandbox attempt events
- Enforces the configured process-internal policy guard on the default shell
  tool path instead of bypassing it with trusted `DangerFullAccess`
- Blocks obvious `workspace-write` shell targets that escape the workspace by
  absolute path, parent traversal, or resolvable symlink targets outside the
  workspace
- Keeps stage fixture prompts disabled by default unless `YUNXI_RUNTIME_FIXTURES=1` is set
- Streams live provider SSE data from reqwest network chunks instead of replaying a completed response body
- Retries provider 429/5xx/network/timeout failures with bounded backoff and `Retry-After` support
- Provides facade types for Agent configuration, input, events, results, and errors
- Runs the default `yunxi` backend through YunXi-owned runtime/provider/tools/storage crates
- Keeps a dry-run Agent path for deterministic smoke checks
- Vendors the Codex Rust workspace source needed by the native backend
- Retains a boundary for checking local Codex CLI checkout shape during future refreshes
- Runs the default workspace without `yunxi-agent-codex` in the CLI dependency graph
- Keeps the source-level Codex compatibility backend outside the default workspace and CLI graph
- Hides the detached Codex compatibility backend from the default CLI backend
  list; use `yunxi-agent-codex` explicitly for that compatibility path
- Keeps JSON and JSONL output modes mutually exclusive
- Rejects unsupported JSONL on metadata subcommands instead of silently
  printing plain text
- Emits paired generic `tool_completed` lifecycle events for MCP completions
  while retaining the richer MCP response item
- Keeps `sessions list --json` lightweight by returning session summaries
  instead of full event-bearing session records
- Provides the default `yunxi_companion_strong` persona profile and bounded
  persona prompt injection after project instructions
- Adds local, transparent JSONL memory storage with global and workspace scopes
- Lets users inspect, search, approve, reject, archive, clear, enable, or disable
  memory through `yunxi memory ...`
- Keeps long-term memory writes disabled by default until `yunxi memory on` or
  `YUNXI_MEMORY_ENABLED=1` enables them
- Emits persona/memory summary events without printing full memory content into
  the execution event stream
- Routes stable global, relationship, agent, and matching-workspace memory into
  a bounded first-turn Boot Context while keeping prompt/recent-turn relevant
  dynamic recall on every turn, with privacy-safe route explanations and
  cross-route dedup
- Derives a Rust-native Relationship Graph Lite view from Schema v3 entities,
  relations, validity windows, and invalidation chains; relationship-history
  queries use event/observed/update/create time order without adding a graph DB
- Provides a conservative proactive companion planner that is disabled by
  default, explains every plan reason, respects quiet hours and per-session/day
  limits, and turns tool ideas into confirmation requests only
- Provides one shared Companion UX & Controls snapshot across CLI and TUI for
  companion, memory, persona, and read-only relationship state
- Persists local companion enable/disable state while keeping cloud control a
  separate, visible, default-off field with no cloud runtime dependency
- Requires explicit confirmation for companion-history and workspace-memory
  clear operations, and appends control actions to a local JSONL audit ledger
- Runs 31 deterministic companion evaluation scenarios without a live provider
  or cloud judge, with persona, memory, relationship, proactive, tool-approval,
  and control metrics available as text, JSON, or one-line JSONL

## Integrated TUI And Streaming Release Regression v2.1.0

v2.1.0 consolidates the v2.0.1-v2.0.9 presentation, streaming, focus,
responsive-layout, mode-isolation, and terminal-recovery contracts into one
release gate without introducing a new UI concept or companion module.

The fixture-driven Ratatui TestBackend suite stores paired normal-main and
Details goldens for the normal companion, long streaming Markdown, tool
approval/failure, CJK/Emoji narrow layout, history scroll/resize, stream fault,
and monochrome semantics. A separate VT100 transcript golden covers ANSI reset,
alternate screen, mouse capture, focus tracking, bracketed paste, cursor state,
resize, and normal/Ctrl+C/error restoration ordering. Main-view goldens reject
raw reasoning, Provider bodies, tool arguments, and tool output that belongs in
Details.

`scripts/conpty/v209` now separates explicit `.tmp` capture from a pure
read-only verifier. `scripts/conpty/v210` binds the full non-TUI matrix,
terminal and streaming recovery, loopback live Provider path, long-stream
cancellation, 100x30 to 58x18 wide-character/mouse/copy-boundary smoke, and
the Rust golden hashes. Formal evidence is written only at release close.

## Cross-Path Terminal Recovery And Streaming Resilience v2.0.9

v2.0.9 makes terminal selection an explicit, testable contract. Only an
interactive invocation with terminal stdin/stdout and no CI override enters
the TUI. One-shot, command, pipe, CI, `--no-tui`, JSON, and JSONL remain plain;
an unavailable forced `--tui` request reports its fallback only on interactive
stderr and never contaminates structured output.

The TUI terminal guard now records each successfully entered state and rolls
back only those states in strict reverse order. Provider streams retain an
incomplete UTF-8 tail across network chunks and reject only confirmed invalid
sequences. Live Markdown tails are limited to 64 KiB, stream content to 256
KiB, history cells to 64 Ki graphemes, details/debug to 32 Ki graphemes, with
bounded tool fields, 8,192 seen event IDs, 256 archived streams, and 800
history cells. Truncation preserves the newest grapheme-aligned content after
redaction.

`scripts/conpty/v209` verifies normal and Ctrl+C terminal restoration, the
plain/pipe/CI/JSON/JSONL byte contract, Provider failure followed by a
successful next turn, and oversized SSE cancellation followed by recovery.
Evidence and its SHA-256 manifest are stored under
`docs/reports/evidence/frames/v209-conpty`. The v2.0.8 visual semantics,
58x18/80x24/100x30/120x40/200x50 snapshots, and v2.0.7-hotfix focus isolation
remain unchanged.

## Visual Semantics And Information Density v2.0.8

v2.0.8 centralizes TUI presentation policy in `TuiStyleSet`. User,
assistant, progress, tool, action-required, notice, warning, error, muted,
header, footer, border, focus, success, and selection semantics now resolve
through full-color, ANSI-16, or monochrome terminal capabilities instead of
scattered renderer colors. Important states keep explicit text labels and
modifiers, so Approval, warnings, errors, and cancellation remain readable
with `NO_COLOR`.

The responsive density policy now includes a 58x18 full-frame baseline. Narrow
headers retain product/provider state while backend, source, debug, cwd, and
other diagnostics are clipped by priority. Approval reserves its safe-default
Decline action and risk line before command detail; header, transcript, and
bottom pane remain disjoint across 58, 80, 100, 120, and 200-column matrices.
The v2.0.7-hotfix Approval/Details focus guards remain unchanged.

Real DeepSeek sessions run through Windows ConPTY in `scripts/conpty/v208`.
The locked collector verifies completed conversation frames at 80x24, 200x40,
and 58x18, plus a `NO_COLOR` session containing a real Approval/default
Decline, provider error, and active-stream cancellation. Sanitized evidence and
its SHA-256 manifest live under
`docs/reports/evidence/frames/v208-conpty`.

## Composer And Dialog Consistency v2.0.6

v2.0.6 replaces the TUI's scattered `String + byte cursor` state with a
grapheme-indexed `EditBuffer`. Composer and `request_user_input` share the same
editing operations while retaining separate submit/cancel semantics. Overlay
entry snapshots the Composer and restoration returns the exact text, grapheme
cursor, and multi-line layout. During an active turn, typing and paste update a
next-turn draft; Enter waits for completion and Ctrl+C cancels only that turn.

The Windows host accounts for crossterm's console-event behavior: LF inside a
rapid ConPTY paste arrives as Ctrl+Enter rather than `Event::Paste`. A bounded
input-burst classifier maps only rapid embedded Enter records to newlines, and
the active-turn drain waits for a 5ms quiet period once a Windows input burst
has started. The CLI drains pending terminal input immediately before entering
approval or user-input views, preventing queued draft characters from becoming
overlay shortcuts.

Real DeepSeek `deepseek-chat` sessions were captured through Windows ConPTY for
ordinary input, multi-line paste, CRLF/lone-CR normalization, a paste longer
than 1,800 characters at 80x24, committed Chinese/emoji/combining input,
stream cancellation and draft recovery, approval-overlay recovery, and a
single final assistant cell. The locked collector is in `scripts/conpty/v206`;
sanitized frames and their SHA-256 manifest are under
`docs/reports/evidence/frames/v206-conpty`.

## v2.0.5 Audit Remediation

The published annotated `v2.0.5` tag remains immutable at its original release
commit. Error-presentation remediation was released as `v2.0.5-hotfix.1`, and
the independently required native ConPTY installation reproducibility fix was
released and re-audited as `v2.0.5-hotfix.2`. All three tags remain immutable
rollback points.

Provider, tool, approval, cancel, terminal, and unknown failures now enter one
`ErrorPresentation` boundary. Normal transcript summaries contain a stable
`YX-*-001` code, retryability, and an actionable next step. Provider wire,
commands, internal stacks, decoder diagnostics, and full output remain in
details/debug. A late structured Ctrl+C decision may refine the same tool
activity from a generic decline to cancellation; all other terminal activity
states remain frozen.

Real DeepSeek `deepseek-chat` sessions were exercised through Windows ConPTY at
80x24, 100x30, 120x40, and 200x50. The evidence covers decline-by-default
approval, explicit approval, rejection, Ctrl+C, non-zero exit, invalid UTF-8,
binary output, 2000-line output, details metadata, and successful input after
errors. See
`docs/reports/evidence/2026-07-20-v2-0-5-error-presentation-conpty-evidence.md`.
The locked collector is stored in `scripts/conpty/v205`; sanitized raw frames
and their SHA-256 manifest are committed under
`docs/reports/evidence/frames/v205-conpty` for independent replay and review.

## Responsive Header Remediation v2.0.4-hotfix.1

The published annotated `v2.0.4` tag remains immutable. The project owner
explicitly selected `v2.0.4-hotfix.1` as the remediation release after the
v2.0.4 visual audit found that the 80-column header still exposed model and cwd
details.

The header now selects a semantic tier before constructing priority segments.
Widths below 90 show only product/version and provider/live state. Widths from
90 through 119 add model information but never construct a cwd segment. Widths
of 120 or more add a path-boundary-compacted cwd while preserving product,
connection, and model status. Fixed 80/100/120/200 tests assert both required
and forbidden fields, and the full-frame snapshots traverse the real renderer.

## International Text And Responsive Layout v2.0.4 (Superseded Audit Candidate)

v2.0.4 routes transcript, composer, approval, and responsive status text through
one Rust-native `TextLayout` model. Display measurement, wrapping, truncation,
source ranges, and cursor positions operate on Unicode grapheme clusters, so
CJK, Japanese kana, emoji ZWJ sequences, and combining characters remain atomic.
URL-, Windows-path-, long-token-, and code-aware policies provide stable break
points without introducing an upstream TUI or non-Rust runtime dependency.

Header, subheader, footer, and approval rows use explicit must-keep, important,
optional, and debug-only priorities. Narrow terminals retain the product and
provider state, current view/action, approval risk, dangerous command identity,
and Approve/Decline controls before model, path, help, or debug details.

Repository-owned full-frame snapshots cover 80x24, 100x30, 120x40, and 200x50.
They exercise the real `ratatui::TestBackend` layout with mixed international
text, a long URL, a Windows path, a fenced code block, active streaming, pinned
history, `new output below`, footer, and composer. Approval has a separate
four-width matrix over the same layout rules.

## Redraw, Scroll, And Resize Stability v2.0.3

v2.0.3 replaces the dirty/force frame flag with a reason-aware redraw scheduler.
Stream deltas and ordinary status events are coalesced, final/control updates are
consumed on the next host tick, and input, scroll, resize, cancellation, and
errors bypass the throttle. Pending reasons are cleared only after a successful
draw.

The transcript viewport now records `FollowTail`, stable cell/line `Pinned`, or
`NewOutputBelow` state. Every wrapped screen row maps back to its canonical
`TuiCellId`, allowing append, final replacement, and narrow/wide or short/tall
resize to resolve the same logical history content. Unicode long-token wrapping
uses grapheme clusters, and saturating small-terminal layout keeps the composer
visible without overlapping regions or panicking.

## Redraw Audit Remediation v2.0.3-hotfix.1

The published annotated `v2.0.3` tag remains immutable. The explicitly approved
`v2.0.3-hotfix.1` re-audit candidate tightens the coalesced-frame interval to
33,334 microseconds, whose theoretical rate is strictly below 30 FPS. The
production `record_draw` path now retains a draw counter used by a deterministic
one-second test that injects 1,000 stream deltas with manually advanced
`Instant` values and proves no more than 30 draws.

Repository-owned full-frame snapshots at 80x24 and 120x40 exercise the real
layout, wrapped transcript, active canonical assistant cell, pinned history,
`new output below`, scrollbar, footer, and composer paths. The snapshots and
their boundary assertions live under
`crates/yunxi-agent-tui/src/snapshots` and `render.rs`; no live provider or
wall-clock sleep is needed for these automated gates.

## Streaming Audit Remediation Hotfix Candidate

The `2.0.2-hotfix.1` audit remediation keeps the published `v2.0.2` tag immutable
and prepares the new annotated `v2.0.2-hotfix.1` re-audit candidate. Provider deltas now carry a stable
`event_id` plus an explicit `ProviderReliable` or `LocalFallback` sequence.
`timeline_store.rs` deduplicates by event ID, uses only reliable provider
sequences for late-event rejection, exposes a duplicate counter for diagnostics,
and preserves equal payloads when their event IDs are distinct.

Finalized or cancelled sessions are removed from the active map; only bounded
minimal canonical-cell metadata is archived. `MarkdownStreamController` commits
complete lines, paragraphs, closed Markdown fences, and safe Unicode grapheme
boundaries. Raw TUI Ctrl+C is classified during active streaming, cancels the
shared run token, and lets the runtime drop the in-flight provider future while
the transcript freezes partial assistant text. Plain CLI and serialized
AgentEvent JSON/JSONL shapes remain unchanged. An isolated real DeepSeek TUI
check confirmed the canonical short response, active-turn Ctrl+C cancellation,
partial-text retention, and successful next prompt. This hotfix remains a
formal re-audit candidate until an independent reviewer completes the release audit. See
`docs/tui-presentation.md` for the complete state model.

## General Companion Agent v2.0.0

v2.0.0 closes the local runtime chain across persona, transparent memory,
relationship continuity, proactive companion policy, controls, and evaluation.
`yunxi-agent-runtime::general_companion_snapshot` exposes one auditable view of
that chain: the runtime owner is YunXi, the default path does not require
upstream Codex, relationships remain read-only, cloud control stays off, and
proactive behavior remains disabled until the user explicitly enables it.

Cross-session integration coverage verifies that a saved preference is recalled
in a later independent session and that a superseded relationship fact is kept
in append-only history but excluded from active context and active-memory
counts. Offline execution remains deterministic; live execution is selected
only when provider credentials are configured. CLI and TUI consume the same
runtime/control boundaries, while the existing 31-scenario offline evaluation
corpus remains the v2 release quality gate with unchanged golden thresholds.

## Build

```powershell
cargo test
cargo build -p yunxi-agent-cli --release --bins
```

The default CLI path does not require `vendor/codex-rs`.

The `codex-native` feature reads Codex Rust source from `vendor/codex-rs`.
`external/codex-rs` is only a local refresh aid and is not required for normal
YunXi checkouts.

## Evaluation Harness

Run the complete offline companion quality gate with one command:

```powershell
yunxi eval companion
yunxi --json eval companion
yunxi --jsonl eval companion
yunxi eval weixin
yunxi --json eval weixin
yunxi --jsonl eval weixin
```

The 31 versioned scenarios live under `evals/companion`. They cover persona
consistency, memory precision/false positives/missed and forbidden writes,
relationship replacement and validity, proactive silence/quiet hours/limits,
tool confirmation, and CLI/control semantics. The default evaluator uses Rust
rules and fixed fixtures only; it does not read provider credentials or call a
live model, Python runtime, cloud service, or external judge.

The Weixin evaluation suite lives under `evals/weixin`. Its default run is
offline and covers protocol mocks, state migration boundaries, pairing, slash
control, final-text delivery segmentation, restart-recovery gates, safety
diagnostics, and the explicit real iLink/Provider manual checklist. It does not
read credentials or perform network requests.

The command exits unsuccessfully when any scenario fails. Structured output
contains per-scenario checks plus aggregate metrics, including
`memory_precision`, `relationship_continuity_rate`,
`proactive_boundary_violation_count`, and `tool_approval_bypass_count`.
## Install On Windows

Build and install the v2.3.3-hotfix.6 CLI into a user-local bin directory:

```powershell
Set-Location "D:\YunXi Agent"
.\scripts\install\install-yunxi.ps1 -AddToPath
```

Open a new terminal after `-AddToPath`, then run:

```powershell
yunxi --version
yunxi
yunxi "explain this project"
```

Proactive companion behavior remains off unless explicitly enabled. Use the
global `--companion` flag for a turn or the explicit check command:

```powershell
yunxi --companion "unfinished task: review the release notes"
yunxi --companion check "continue topic: the release plan"
yunxi companion status
yunxi companion on
yunxi companion off
```

The planner never runs a tool on its own. A tool-related plan is rendered as a
request for confirmation. Quiet hours and limits are configured through the
`AgentConfig.companion` facade; the default is disabled and requires reasons.
The installer copies both `yunxi.exe` and the compatibility
`yunxi-agent-cli.exe`.

## Local Web Console

Start the local Web console from the same workspace used by the CLI and Weixin
gateway:

```powershell
Set-Location "D:\YunXi Agent"
yunxi web
```

Open `http://127.0.0.1:17861/`. The server binds to loopback by default and
exposes only the local health, runtime status, persona, memory, and chat routes
required by the interface. Use `--port` to select another local port or
`--bind` for an explicit address.

When Weixin autostart is enabled, `yunxi web` resolves the workspace containing
the configured account and starts the same guarded Weixin gateway used by the
interactive CLI. Pass `--no-weixin-autostart` when the Web process must run
without starting that gateway.

The Web interface keeps chat, memory, and persona on one runtime boundary.
Animation falls back to the Web Animations API when GSAP is unavailable and
honors `prefers-reduced-motion` and `prefers-reduced-transparency`.

## Interactive CLI

Run `yunxi` without a prompt to enter interactive mode:

```powershell
yunxi
```

When stdin and stdout are both attached to a terminal, YunXi uses the current
TUI host, wrapped-row transcript viewport, draggable scrollbar, composer, and
approval overlay. Use `--no-tui` to force the stable plain REPL:

```powershell
yunxi --no-tui
```

Useful interactive commands:

- `/help`: show available commands
- `/session`: show the active session id, turn count, cwd, model, and provider
- `/status`: show provider, event, tool, MCP, and usage status
- `/tools`: list fixed and workspace dynamic tools
- `/mcp`: show workspace MCP configuration without starting servers
- `/cost`: show last-turn and session token usage
- `/resume <session_id>`: continue from a saved session
- `/model [name]`: show or switch the model field
- `/provider [name]`: show or switch the provider field
- `/debug events on|off`: show or hide raw debug event summaries in the TUI
- `/details [id]`: show the latest or selected TUI debug detail
- `/controls`: show the shared control panel with source and clear-scope details
- `/controls refresh`: refresh the control snapshot and audit the action
- `/controls enable|disable <scope>`: persist a local companion, memory, or persona switch
- `/controls clear <companion|memory>`: open an explicit confirmation dialog
- `/companion on|off|status|clear`: companion-specific control shortcuts
- `/cwd`: show the active working directory
- `/clear`: clear the terminal
- `/exit` or `/quit`: leave interactive mode

One-shot and automation modes remain available:

```powershell
yunxi "explain this project"
yunxi run sessions
yunxi --offline "run without a model provider"
yunxi --jsonl "explain this project"
```

`--json` and `--jsonl` never enter the REPL when a prompt is missing; they keep
returning structured errors for script safety.

## Persona And Memory

YunXi v1.9.1 renders the local persona and transparent memory foundation as
stable `persona`, `boundaries`, `human`, `relationship`,
`boot_memory_context`, and `dynamic_memory_context` blocks. Boot memory is
selected only for a new session's first turn; prompt-relevant dynamic memory is
selected every turn. Both memory blocks are context, not instructions. Persona
prompt injection is enabled by default. Long-term memory writes are disabled by
default and become active only after `yunxi memory on` or
`YUNXI_MEMORY_ENABLED=1`.

```powershell
yunxi persona status
yunxi persona profile
yunxi persona list
yunxi persona import .\my-persona.json
yunxi persona set yunxi_companion_strong
yunxi persona off
yunxi persona on

yunxi memory status
yunxi memory on
yunxi memory list --workspace
yunxi memory pending
yunxi memory approve <id>
yunxi memory reject <id>
yunxi memory delete <id>
yunxi memory off

yunxi controls status
yunxi controls show relationship
yunxi controls enable companion
yunxi controls disable companion
yunxi controls clear companion --confirm
yunxi controls clear memory --confirm
yunxi controls audit
```

Memory is stored as local append-only JSONL under `%USERPROFILE%\.yunxi\memory`
and `<workspace>\.yunxi\memory`. Memory Schema v3 adds layer, entity, temporal,
evidence, source-lineage, and invalidation metadata while migrating v1/v2
records on read without rewriting their files. See `docs/persona-memory.md` for
schema, privacy, expiry/invalidation, and pending-review details.

The unified snapshot identifies each scope's source as current config,
persisted settings, runtime snapshot, or read-only history. Persona and
relationship review remain read-only; memory clear archives only workspace
active/pending records, while companion clear affects only local companion
history. Audit and history files live under `<workspace>\.yunxi\controls`.

`--jsonl` is reserved for agent execution streams in v1.9.3. Metadata
subcommands such as `sessions list`, `parity map`, `persona status`, and
`memory status` reject `--jsonl`; use `--json` for their machine-readable
output.

## Honesty And Safety Boundaries

Offline mode is deterministic and useful for smoke checks, but it is not a
model. Plain terminal output marks offline assistant text with `[offline]`, and
auto mode prints a warning when no live credentials are found.

The current sandbox layer is a process-internal policy guard/advisor with a
platform runner diagnostic entrypoint. It classifies commands, routes
approvals, blocks common workspace-write target escapes, including resolvable
symlink escapes, and can request escalation. v1.7.8 reports the selected runner,
stable `backend_id`, human `backend_label`, canonical `enforcement`, and
unsupported reason. The default Windows runner is `process_lifecycle`, not
filesystem isolation; unverified platform paths remain `policy_only`;
`danger-full-access` is reported as `policy_bypass`. The legacy
`enforcement_level` field is kept as a compatibility alias and must match
`enforcement`. YunXi does not claim OS-enforced Windows restricted tokens,
Linux Landlock/seccomp, namespaces, or macOS seatbelt isolation unless the
event explicitly reports `enforcement=os_restricted` and `os_isolation=true`.
Approved commands still execute with the YunXi process permissions.

Sandbox event consumers should read `schema_version=1`, `backend_id`,
`enforcement`, `runner`, and `os_isolation` instead of parsing the human
`backend` string. Current Windows workspace-write/read-only attempts report
`backend_id=windows_process_lifecycle`, `enforcement=process_lifecycle`,
`runner=windows_process_lifecycle`, `os_isolation=false`, and a non-empty
`unsupported_reason`.

Historical `stage 4x` fixture prompts are disabled in the default product path.
They remain available only for explicit compatibility smoke runs with
`YUNXI_RUNTIME_FIXTURES=1`, and fixture metadata is marked with
`fixture_mode=true`.

## Run From Source

```powershell
cargo run -p yunxi-agent-cli --bin yunxi -- "explain this project"
cargo run -p yunxi-agent-cli --bin yunxi -- --backend yunxi "explain this project"
cargo run -p yunxi-agent-cli -- "explain this project"
cargo run -p yunxi-agent-cli -- --backend yunxi "explain this project"
cargo run -p yunxi-agent-cli -- --backend dry-run "explain this project"
cargo run -p yunxi-agent-cli -- --cwd "D:\some\repo" "fix the failing test"
cargo run -p yunxi-agent-cli -- --json "explain this project"
cargo check --manifest-path crates/yunxi-agent-codex/Cargo.toml --features codex-native
```

## DeepSeek Provider

YunXi Agent keeps provider credentials out of committed source. Import a
DeepSeek credential from a local private file into the current user's Windows
environment with the secret-safe helper:

```powershell
.\scripts\provider\import-deepseek-credential.ps1 `
    -ApiFile "C:\private\api.txt" `
    -CredentialIndex 1
```

The helper automatically uses a file containing exactly one credential
candidate. When a private file contains several provider credentials,
`-CredentialIndex` must select one explicitly; the helper never guesses or
prints a candidate value. It configures `DEEPSEEK_API_KEY`,
`YUNXI_PROVIDER_PROFILE=deepseek`, and the default model. Open a new PowerShell
after importing, then run:

```powershell
yunxi
yunxi "Reply with a one-line status"
```

Provider selection precedence is:

1. `--provider-live` forces a real provider and fails early when credentials are missing.
2. `--offline` forces the deterministic offline provider.
3. Default auto mode uses configured credentials and otherwise falls back offline.

For temporary process-only configuration:

```powershell
$env:YUNXI_PROVIDER_PROFILE = "deepseek"
$env:DEEPSEEK_API_KEY = "<your-api-key>"
$env:YUNXI_AGENT_MODEL = "deepseek-v4-flash"
yunxi "Reply with a one-line status"
```

Generic OpenAI-compatible providers can continue to use
`YUNXI_PROVIDER_API_KEY`, `YUNXI_PROVIDER_API_KEY_ENV`,
`YUNXI_PROVIDER_BASE_URL`, and `OPENAI_API_KEY`. Never commit real API keys.
The repository also keeps a secret-safe DeepSeek smoke helper at
`scripts\provider\deepseek-live-smoke.ps1`; it requires an explicit
`-ApiFile`, requires `-CredentialIndex` for multi-credential files, and never
stores that path in project configuration.

## Version Tags

Each released version keeps its own immutable Git tag. v1.2 adds `v1.2.0` and
preserves `v1.0.0` and `v1.1.0` as rollback points. Do not delete or move old
version tags.

v1.2.1 adds immutable tag `v1.2.1` without moving `v1.2.0`. DeepSeek HTTP
400/422 responses receive one compatibility retry without optional request
`metadata`; a failed turn is rendered as a redacted error and returns control
to `yunxi>`. Tool-loop history uses matching assistant `tool_calls` and
`tool_call_id` fields for OpenAI/DeepSeek-compatible APIs.

v1.3.0 adds immutable tag `v1.3.0` without moving earlier tags. It promotes the
interactive terminal host from batch rendering to streaming runtime events,
approval/user-input request handling, and Ctrl+C cancellation propagation into
YunXi-owned runtime/tools/exec code.

v1.4.0 adds immutable tag `v1.4.0` without moving earlier tags. It promotes
terminal status visibility with `/tools`, `/mcp`, `/cost`, and `/status`, so an
interactive YunXi session can inspect available tools, workspace MCP config,
last-turn usage, cumulative usage, and runtime event summaries without leaving
the REPL.

v1.5.0 adds immutable tag `v1.5.0` without moving earlier tags. It focuses on
honesty and runtime correctness: offline output is marked inline, auto fallback
warns when no live provider credential is available, sandbox text is corrected
to policy-guard/advisory wording, stage fixtures are opt-in, live SSE streaming
uses network byte chunks, and provider retry honors `Retry-After` with bounded
backoff for transient HTTP/transport failures.

v1.6.0 adds immutable tag `v1.6.0` without moving earlier tags. It hardens the
policy guard by making the default shell tool execution honor `ToolPolicy`,
blocking common workspace-write target escapes, classifying command risk with
shell-ish tokens instead of substring-only checks, preventing streaming retries
after body bytes have reached the parser, and extending auto fallback warnings
to one-shot, JSON, JSONL, and resume paths. It remains a process-internal guard,
not an OS-level sandbox.

v1.7.0 adds immutable tag `v1.7.0` without moving earlier tags. It introduces
the terminal TUI foundation with `ratatui`/`crossterm`, line-edited terminal
input through `reedline`, `--no-tui` plain fallback, renderer separation for
plain/structured/TUI paths, and symlink escape regression coverage for the
workspace-write policy guard. It remains a process-internal guard, not an
OS-level sandbox.

v1.7.1 adds immutable tag `v1.7.1` without moving earlier tags. It replaces
the v1.7 prototype TUI renderer with the `yunxi-agent-tui` crate: terminal
guard, transcript cells, reasoning merge, composer, approval overlay,
`request_user_input` overlay, and TUI-routed slash command output are now
YunXi-owned runtime capabilities. Plain, JSON, JSONL, and one-shot paths remain
compatible.

v1.7.2 adds immutable tag `v1.7.2` without moving earlier tags. It deepens the
TUI reading experience with mouse wheel and PageUp/PageDown/Home/End transcript
navigation, history/follow-tail state, fixed composer and overlay panes,
scrollbar/title/footer status, stable/live markdown stream collection, and
frame-throttled redraws for smoother long streaming turns.

v1.7.3 adds immutable tag `v1.7.3` without moving earlier tags. It cleans the
TUI transcript by hiding raw tool protocol JSON, stdout token noise, context
and session bookkeeping, and long skill/tool outputs from the normal view.
Tool execution is rendered as compact timeline cells, while `/debug events
on|off` and `/details [id]` retain diagnostic access to redacted raw details.

v1.7.4 adds immutable tag `v1.7.4` without moving earlier tags. It corrects the
TUI transcript viewport to scroll by wrapped screen rows instead of logical
lines, shares one layout geometry between render and mouse handling, and adds
mouse-drag scrollbar support while preserving wheel and keyboard navigation.

v1.7.6 adds immutable tag `v1.7.6` without moving earlier tags. It converges
CLI protocol and safety surfaces by making assistant final messages
single-source in event streams, pairing MCP tool completion lifecycle events,
making `--json` and `--jsonl` mutually exclusive, rejecting unsupported JSONL
metadata subcommands, returning lightweight JSON session summaries, avoiding
misleading provider fallback warnings for explicit dry-run/detached backends,
and exposing sandbox `os_isolation=false`/`enforcement` fields instead of
implying OS-level isolation.

v1.7.8 adds immutable tag `v1.7.8` without moving earlier tags. It closes the
v1.7.6 sandbox/TUI hard gate by adding machine-readable sandbox
`enforcement_level`, sandbox acceptance coverage for policy-only and
policy-bypass paths, bounded TUI approval layout that keeps Approve, Decline,
and shortcut hints visible on narrow terminals, medium-width header model
visibility, and metadata help text that marks `--jsonl` as agent-execution only.

v1.8.0 adds immutable tag `v1.8.0` without moving earlier tags. It introduces
the YunXi-owned persona and transparent memory foundation: the
`yunxi_companion_strong` profile, bounded persona/memory prompt injection,
global/workspace JSONL memory storage, CLI review controls, privacy-first write
policy, and summary-only persona/memory events.

v1.8.6 extends that foundation with typed redaction for both `--json` and
`--jsonl` agent execution output. Secret-like prompt, response, tool, state,
provider, and nested protocol fields are sanitized before serialization while
memory privacy policy independently continues to discard sensitive candidates.

v1.8.7 adds immutable, XML-like Persona Context Blocks with stable ordering,
escaping, active-memory filtering, safety-priority notices, and structure-safe
budget truncation. Runtime injection continues to consume one bounded compiled
string, and memory remains context rather than instruction or authorization.

v1.8.8 adds immutable tag `v1.8.8` without moving earlier tags. Memory Schema
v3 adds structured layer, entity, temporal, evidence, source-lineage, and
invalidation metadata; v1/v2 JSONL migrates on read, merge retains provenance,
and recall excludes expired, invalidated, or superseded records. Persona
Context Blocks remain bounded context rather than instruction.

v1.8.9 adds the Rust-native L0-L3 Memory Pipeline. Raw turns are retained only
as bounded, redacted evidence; structured facts and relationship events pass
through one rule/provider policy and dedup path; relationship and sensitive
records remain pending; and only stable, low-risk, clearly sourced facts can be
promoted to profile summaries. Provider extraction failure is fail-soft and
does not suppress rule candidates.

v1.9.0 adds immutable tag `v1.9.0` without moving earlier tags. Stable global
preferences, relationship/agent baselines, and matching-workspace facts are
selected into a bounded first-turn Boot Context; prompt-relevant memories remain
available through an independent per-turn dynamic route. Route explanations
contain ids, metadata, scores, and fixed reasons but never raw memory content.
Boot and dynamic blocks remain bounded context and cannot override project,
user, sandbox, privacy, safety, or tool instructions.

v1.9.1 adds a derived Relationship Graph Lite without replacing append-only
JSONL storage. Entity/relation edges carry event, observation, validity,
expiry, invalidation, supersession, and conflict metadata. Clear active
preference/correction changes append a bidirectional supersession chain while
retaining old facts for history; Boot Context excludes old facts, and dynamic
relationship-history queries return safe, time-ordered explanations.

v2.0.6 adds the grapheme-indexed Composer/input core, recoverable next-turn
drafts, explicit overlay routing, Windows ConPTY paste reconstruction, and
single-cell delta/final consistency. It adds a new immutable `v2.0.6` release
tag without moving or deleting any earlier release or hotfix tag.

v2.0.7 adds explicit Composer, History, Approval, and Details focus semantics;
a pure Rust key/action resolver; a scrollable redacted details layer; and
focus-specific narrow footer hints. Draft, cursor, approval selection, and
transcript viewport state survive focus and details transitions. The v2.0.6
grapheme editor and single-cell streaming lifecycle remain unchanged.

v2.0.7-hotfix closes the audit-blocking mouse route gap. Approval now consumes
wheel and underlying scrollbar input without moving the transcript; Details
maps wheel and PgUp/PgDown only to `details_scroll`; scrollbar click/drag/up is
guarded again at the host state boundary. Composer and History retain history
scrolling, and all four focus targets have deterministic key/paste/wheel tests.

v2.0.8 adds one centralized terminal semantic-style facade and responsive
information-density policy without changing the focus or stream state
machines. Monochrome status labels, a 58x18 frame baseline, and two real
DeepSeek/Windows ConPTY scenarios cover responsive layout and low-color
Approval/error/cancellation behavior. Earlier release and hotfix tags remain
immutable.

## Backend Capability Matrix

| Backend | Default | Owner | Purpose |
| --- | --- | --- | --- |
| `yunxi` | Yes | YunXi runtime crates | Autonomous runtime migration path |
| `dry-run` | No | `yunxi-agent-core` | Deterministic smoke checks |
| `codex` / `--live` | No | Detached | Rejected by the default CLI with compatibility guidance |

## Codex Compatibility Matrix

The Codex compatibility crate is a source-level integration with the Codex
headless Agent stack. It is not part of the default workspace member set or the
default CLI dependency graph. Normal tests do not require credentials.

| Capability | Status |
| --- | --- |
| Text prompt, new thread | Supported behind `codex-native` |
| Model/provider config | Supported through Codex config mapping |
| Shell commands | Provided by embedded Codex runtime |
| Patches/file changes | Provided by embedded Codex runtime |
| Approval/sandbox mapping | Supported |
| MCP event mapping | Supported |
| Skills runtime | Provided by embedded Codex runtime |
| Session/rollout storage | Provided by embedded Codex runtime |
| Resume CLI | Staged after live new-thread stabilization |

## CodeGraph

If `.codegraph/` exists, use CodeGraph first when locating or understanding code in this repository.

## GitHub Publication

Project automation reads and writes GitHub branch, tree, commit, ref, and tag
state through the GitHub REST API. GitHub tokens remain local and must never be
printed, logged, or committed. Normal repository documentation does not embed
token values or private credential-file paths.

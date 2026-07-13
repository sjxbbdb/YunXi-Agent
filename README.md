# YunXi Agent v1.7.8

YunXi Agent v1.7.8 is a terminal-first Rust Agent CLI and reusable core library
built from the Codex CLI source extraction work. The default runtime is
YunXi-owned and does not depend on the upstream Codex runtime.

When DeepSeek credentials are configured, the default `yunxi` command now uses
the real DeepSeek provider automatically. Without credentials it remains usable
through the inline-labelled offline provider; offline assistant output is marked
with `[offline]` and `/cost` reports that no model call was made.

## Layout

- `crates/yunxi-agent-core`: reusable Agent facade and extraction boundary
- `crates/yunxi-agent-provider`: YunXi-owned provider request/response boundary
- `crates/yunxi-agent-tools`: YunXi-owned tool execution boundary
- `crates/yunxi-agent-storage`: YunXi-owned session storage boundary
- `crates/yunxi-agent-runtime`: YunXi-owned Agent runtime boundary
- `crates/yunxi-agent-codex`: standalone compatibility layer around the vendored Codex headless runtime
- `crates/yunxi-agent-tui`: YunXi-owned Codex-style terminal TUI host, transcript, composer, and approval overlay
- `crates/yunxi-agent-cli`: v1.7.8 terminal CLI package that builds `yunxi`
  and the compatibility `yunxi-agent-cli`
- `vendor/codex-rs`: vendored Codex Rust workspace source used by `codex-native`
- `docs/extraction-status.md`: current extraction status and known gaps
- `docs/superpowers/specs`: design specs
- `docs/superpowers/plans`: implementation plans

## Current Capabilities

- Compiles as an independent Rust workspace
- Builds a terminal command named `yunxi`
- Starts an interactive terminal session when `yunxi` is run without a prompt
- Uses a Codex-style TUI terminal host by default when stdin/stdout are both
  real terminals, with mouse wheel/PageUp/PageDown/Home/End transcript
  navigation, draggable transcript scrollbar, and `--no-tui` available for the
  stable plain REPL
- Uses a TUI composer for terminal input while keeping piped stdin and scripted
  sessions on the plain line reader
- Renders approval and `request_user_input` inside the TUI bottom pane instead
  of leaking line prompts into the alternate screen
- Keeps the TUI composer fixed while the transcript can scroll through history
  without losing new streamed output below
- Scrolls the TUI transcript by pre-wrapped screen rows, so long Chinese,
  English, tool, and reasoning output keeps the title, viewport, and scrollbar
  in sync
- Throttles TUI streaming redraws to keep long reasoning/model output readable
- Merges reasoning deltas into transcript cells instead of rendering one line per token
- Filters the TUI transcript so raw tool protocol JSON, stdout token noise,
  context/session bookkeeping, and long skill/tool outputs stay out of the
  normal view
- Shows tool work as compact timeline cells with approval, running,
  completion, output-summary, and details references
- Provides `/debug events on|off` and `/details [id]` for TUI diagnostics
  without polluting the default transcript
- Streams runtime events to the terminal while an interactive turn is running
- Prompts for interactive approval and `request_user_input` tool calls
- Propagates Ctrl+C cancellation into the runtime and shell exec layer
- Provides REPL status commands for tools, MCP config, usage, and turn summaries
- Keeps the interactive session open when one provider or tool turn fails
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

## Build

```powershell
cargo test
cargo build -p yunxi-agent-cli --release --bins
```

The default CLI path does not require `vendor/codex-rs`.

The `codex-native` feature reads Codex Rust source from `vendor/codex-rs`.
`external/codex-rs` is only a local refresh aid and is not required for normal
YunXi checkouts.

## Install On Windows

Build and install the v1.7.8 CLI into a user-local bin directory:

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

The installer copies both `yunxi.exe` and the compatibility
`yunxi-agent-cli.exe`.

## Interactive CLI

Run `yunxi` without a prompt to enter interactive mode:

```powershell
yunxi
```

When stdin and stdout are both attached to a terminal, YunXi uses the v1.7.8
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

`--jsonl` is reserved for agent execution streams in v1.7.8. Metadata
subcommands such as `sessions list` and `parity map` reject `--jsonl`; use
`--json` for their machine-readable output.

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

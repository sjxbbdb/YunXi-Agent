# TUI Presentation, Streaming Timeline, And Quiet Transcript

YunXi Agent v2.1.0 keeps the v2.0.1 presentation boundary, v2.0.2-hotfix.1
streaming timeline, and v2.0.3-hotfix.1 redraw/viewport guarantees, then adds
one international text layout model, explicit responsive clipping priorities,
a recoverable grapheme-indexed input model, centralized semantic styles,
explicit terminal lifecycle rollback, and bounded stream/transcript storage.
Runtime events remain complete and ordered in `yunxi-agent-core`; only the TUI
maps them into user-facing cells.

## v2.1.0 Integrated Release Regression

The integrated release gate is fixture-driven rather than tied to a single
renderer detail. Ratatui TestBackend snapshots pair the normal main view with
the corresponding Details view for normal companion output, long streaming
Markdown, approval/tool failure, CJK/Emoji at 58 columns, history scroll and
resize, Provider stream failure, and low-color semantics. Every pair proves
that raw internal payload stays out of the main transcript and remains
available through an explicit Details action.

The separate VT100 transcript suite fixes the terminal protocol boundary:
alternate screen, bracketed paste, focus tracking, mouse capture, cursor
visibility, terminal resize, ANSI reset, and reverse restoration. It applies
the same terminal-exit contract to normal exit, Ctrl+C, tool failure, and
Provider error. Existing CLI byte tests keep TUI/plain/JSON/JSONL mode
selection isolated.

The Windows evidence workflow now has two explicit roles. `capture` writes to
an explicit `.tmp` directory; `verify` reads formal evidence without changing
it and checks a before/after directory fingerprint. The v2.1.0 ConPTY gate
also binds the Rust main/Details and VT100 golden hashes so unit and terminal
evidence cannot drift independently.

## v2.0.9 Terminal Lifecycle And Bounded Streaming

The CLI mode resolver classifies Interactive, OneShot, Command, JSON, and JSONL
invocations before dispatch. JSON/JSONL, pipe, CI, one-shot, command, and
`--no-tui` routes are always plain. `--tui` enters the alternate screen only
when stdin and stdout are terminals in an interactive invocation; otherwise a
plain fallback notice is emitted only to interactive stderr. Byte-level CLI
tests exclude ESC bytes, alternate-screen controls, and TUI footer text from
every non-TUI route.

`TerminalLifecycleState` records raw mode, alternate screen, bracketed paste,
focus tracking, mouse capture, and hidden cursor only after each operation
succeeds. Entry failure immediately restores completed operations in reverse
order. `TerminalGuard::drop` uses the same order, so normal exit, Ctrl+C,
Provider/tool errors, panic, and early returns share one restoration contract.

Network streaming preserves an incomplete UTF-8 suffix across response chunks
and rejects confirmed invalid bytes without echoing them. Markdown collection
drains committed prefixes instead of retaining them indefinitely. The live
tail is capped at 64 KiB and combined stream content at 256 KiB. Transcript
history keeps at most 800 cells and 64 Ki graphemes per cell; debug/details,
tool fields, seen event IDs, and archived stream sessions have explicit caps.
Redaction precedes grapheme-aligned truncation, which keeps recent content and
adds a visible notice.

Timeout/disconnect error boundaries freeze partial assistant content and
release the active session. Duplicate final, reliable out-of-order delta, and
late post-cancel events cannot recreate or mutate an archived stream. The next
turn can start normally. `scripts/conpty/v209` exercises terminal restoration,
cross-path byte isolation, Provider recovery, long-stream cancellation, and
post-cancel recovery through Windows ConPTY.

## v2.0.8 Semantic Styles And Responsive Density

`styles.rs` owns `TuiStyleSet`, `TuiSemanticStyle`, and terminal capability
detection. The renderer requests user, assistant, progress, tool,
action-required, notice, warning, error, muted, header, subheader, footer,
border, focus, success, and selection semantics without choosing terminal
colors itself. Full-color and ANSI-16 palettes use terminal-native colors;
`NO_COLOR` and low-capability terminals retain bold, reverse-video, and
explicit text labels instead of depending on foreground/background color.

Transcript cells map event/tool phases to those semantics before wrapping.
Approval renders `Approval required | default: Decline` and `Decline (safe
default)`; warning, error, cancellation, progress, and action-required states
retain stable visible labels. Composer, UserInput, Details, controls, headers,
borders, and selection use the same facade.

At widths below 90, the subheader preserves view/cell/provider state and drops
backend/source/debug diagnostics. The shared layout and Approval measurement
tests cover 58, 80, and 200 columns plus low-height matrices. Approval at 58
columns reserves decision, risk, and shortcut rows before command detail. The
58x18, 80x24, 100x30, 120x40, and 200x50 full-frame snapshots prove bounded,
non-overlapping regions with international text and an active Composer.

The real Windows ConPTY gate in `scripts/conpty/v208` records completed
DeepSeek conversation frames at 80x24, 200x40, and 58x18. Its independent
`NO_COLOR` scenario proves zero colored/background cells at Approval while
reverse-video selection, safe-default text, provider error code, and active
stream cancellation remain visible. Evidence is stored under
`docs/reports/evidence/frames/v208-conpty`.

## v2.0.6 Composer And Active Views

`EditBuffer` is the sole user-visible cursor owner. Its cursor is a grapheme
index; byte offsets are derived only when mapping text into visual rows and
columns. Insertion normalizes CRLF and lone CR to LF. Backspace, Delete,
Left/Right, Home/End, newline, submit, snapshot, and restore never split CJK,
emoji ZWJ, or combining graphemes.

`BottomPane` permanently owns the Composer buffer. Approval and user-input
requests are active views over that buffer: entry stores a snapshot and return
restores text and cursor. User input has its own `Input` title and its own
`EditBuffer`; it does not borrow or clear Composer state. Composer rendering
shows at most six body rows and chooses a cursor-centered window, preserving
the footer and transcript at 80, 100, 120, and 200 columns.

While a turn is active, ordinary characters, committed IME text, paste,
Backspace/Delete, arrows, and Home/End edit a next-turn draft. Enter and Esc do
not submit or clear it; Ctrl+C cancels the current turn. Before a blocking
approval or user-input view starts, the CLI performs a host tick so queued
characters reach Composer first. Once Windows draining observes input, it
continues until a 5ms quiet period to prevent one ConPTY paste from crossing an
overlay boundary.

On Windows, crossterm 0.28 reports LF from a ConPTY paste as Ctrl+Enter key
records instead of `Event::Paste`. `WindowsInputBurst` recognizes only a rapid
sequence of at least four text presses followed within 25ms by an unshifted,
unmodified-or-Control Enter; that Enter is routed through the normal newline
operation. Slow or isolated Enter keeps submit semantics. Native `Event::Paste`
continues to use the same buffer API on platforms that provide it.

The final response, completion boundary, cancellation, and provider error all
reuse the existing stable turn-cell lifecycle. A final update freezes the same
assistant cell used by deltas and cannot insert a duplicate answer or touch the
Composer draft.

The real Windows gate is `scripts/conpty/v206`. Its eight independent DeepSeek
sessions cover ordinary, multi-line, CRLF, long, IME-style, stream-cancel,
approval-restore, and final-single behavior. Sanitized checkpoints and their
SHA-256 manifest are committed under
`docs/reports/evidence/frames/v206-conpty`; `npm run verify` is offline.

## v2.0.5 Tool Activity And Error Presentation

Tool calls are represented by one stable activity cell keyed by the external
tool id. Requested, approval required, running, and terminal phases update that
cell in place; late events cannot reopen a terminal activity. The sole narrow
exception is a structured approval cancellation arriving after the runtime's
generic declined completion: it refines that same cell from Declined to
Cancelled without reopening or duplicating it. The normal
transcript renders one safe summary line. Commands, working directories,
exit codes, complete output, decoder metadata, and provider diagnostics remain
available through details/debug.

Approval is rendered in the dedicated bottom pane. The initial selection is
Decline, Y/Enter is an explicit approval, N/Esc declines, and Ctrl+C records a
cancelled decision. Waiting for approval does not add repeated transcript
entries.

Command output is read as bytes and passed through one `ExecOutputDecoder` for
stdout and stderr. Invalid UTF-8 is lossily displayed with replacement counts,
binary output is reduced to a safe summary, and long output records original
bytes, displayed bytes, truncation, and `Clean`/`Lossy`/`Partial` integrity in
details. Error summaries use stable provider/tool/approval/cancel/terminal/
unknown codes with a user-actionable next step.

All user-visible error producers now converge on `ErrorPresentation`. The
normal contract is `code + summary + retryable + next`; `detail_ref` points to
the redacted diagnostic record. Command completion details put byte-integrity
metadata before output so long output cannot hide `original_bytes`,
`replacement_count`, `truncated`, or `integrity` behind the details line cap.

The real DeepSeek/Windows ConPTY gate and selected sanitized frames are stored
in `docs/reports/evidence/2026-07-20-v2-0-5-error-presentation-conpty-evidence.md`.
It covers 80x24, 100x30, 120x40, 200x50, approval/decline/Ctrl+C, non-zero exit,
invalid UTF-8, binary fallback, long-output details, PageUp/End navigation, and
successful input after terminal failures.

The gate is independently reproducible through `scripts/conpty/v205`. Its
locked Node dependencies only provide the Windows PTY host and terminal parser;
all product events and frames come from the release `yunxi.exe`. The committed
raw frames live in `docs/reports/evidence/frames/v205-conpty`, and the offline
verifier checks SHA-256, required checkpoints, decision keys, error codes,
decoder integrity fields, and secret-like token shapes.

## Boundary

The data path is:

```text
AgentEvent
  -> TuiPresentation::present_agent_event
  -> TuiEvent { kind, safe text, optional detail, stream identity }
  -> TimelineStore { event ID, sequence source, active session, canonical cell }
  -> pure visibility filter
  -> Transcript / HistoryCell
  -> renderer
```

`presentation.rs` is the only layer that interprets `AgentEvent` semantics and
converts provider stream metadata into `TuiStreamIdentity`.
`event_filter.rs` decides only whether an already-classified event is visible
in the transcript, visible when debug mode is enabled, or hidden. `chat.rs`
applies explicit timeline insert/update/finalize/cancel results by canonical ID.
`render.rs` performs layout and styling without reading runtime events.

The host also exposes `push_tui_event` for callers that already hold a
presentation event. Its compatibility `push_agent_event` entry immediately
delegates to the same presentation object.

## Unified Text Layout

`text_layout.rs` is the shared internal display model for transcript, composer,
approval, header, subheader, and footer rendering. It provides grapheme-safe
display measurement, wrapping, truncation, source ranges for visual lines, and
byte-index-to-visual-row/column mapping. The composer renderer and cursor use
the same wrapped result, so display and input navigation cannot disagree about
line boundaries.

The available policies are `NaturalText`, `BreakLongToken`, `UrlAware`,
`WindowsPathAware`, and `CodeBlock`. Natural text preserves ordinary word
boundaries while allowing CJK and kana breaks. URL and path policies prefer
semantic separators. Code preserves indentation/fence structure as far as the
available width permits. Every policy treats emoji ZWJ and combining sequences
as indivisible graphemes.

Responsive single-line regions use `MustKeep`, `Important`, `Optional`, and
`DebugOnly` priorities. Before priority clipping, the header selects an explicit
semantic tier: widths below 90 construct only product/version and provider/live
state; widths from 90 through 119 add model but do not construct cwd; widths of
120 or more add a path-boundary-compacted cwd. This prevents spare narrow-screen
cells from reintroducing low-priority context. The subheader retains view/cell
state before source and debug data; the footer retains the currently executable
action. Approval layout keeps risk, command/path identity, Approve/Decline, and
shortcut rows on narrow screens.

## Default Visibility

The normal transcript can contain:

- user and assistant messages;
- bounded, non-sensitive progress summaries;
- compact tool lifecycle and output-size summaries;
- approval prompts and decisions;
- notices and sanitized error summaries.

The normal transcript never contains raw reasoning, memory/context decisions,
hidden prompts, provider wire payloads, `arguments_json`, complete stdout or
stderr, stack traces, or unredacted tool parameters. These values may be kept
as a redacted `PresentationDetail` with a stable detail ID. They appear only
after `/debug events on` or an explicit `/details [id]` request.

Details are indexed independently of the transcript's numeric display index.
When the same external tool ID receives lifecycle updates, its stable cell and
detail IDs are reused instead of creating protocol-noise cells.

## Streaming

Provider-backed `AgentEvent::Message` values carry non-serialized thread ID,
turn ID, stream/message ID, stable event ID, sequence source, and phase.
`ProviderReliable(n)` means the provider supplied an authoritative sequence;
`LocalFallback(n)` is explicitly a local identity component and is never treated
as provider ordering. Legacy producers receive fallback identity. This metadata
is not part of the AgentEvent JSON or JSONL wire shape.

`timeline_store.rs` owns `StreamSession`. Each session contains its canonical
cell ID, last accepted reliable sequence, state, active content, and
`MarkdownStreamController`. Event IDs are recorded in a bounded seen set before
state application. A repeated event ID is ignored and increments the duplicate
debug counter. Only reliable provider sequences can reject a late event.
The transitions are:

- started/delta: insert or update the active canonical cell;
- retry: reset partial content while reusing the active turn cell;
- final: replace the canonical text and mark that cell finalized;
- cancel: freeze the active cell and reject late events for that session;
- finish: commit an active delta-only provider response at the turn boundary.

Final and cancel remove the full session, including content and collector buffer,
from the active map. The store retains only bounded minimal archive metadata for
canonical-cell continuity and rejects late events for archived stream keys. A
new stream after cancellation receives a different cell so retry cannot bind to
the frozen response. Equal text with distinct event IDs remains legitimate.

`MarkdownStreamController` separates stable source from the live tail. It
commits complete lines and paragraphs, holds open fenced code blocks until the
matching fence closes, and otherwise commits only through a safe grapheme
boundary while retaining the last grapheme for possible cross-delta joining.
Final drain preserves the exact source without inserting a newline.

Viewport state remains independent of stream state. `WrappedTranscript` maps
every screen row back to its stable `TuiCellId` and wrapped-line offset. The
viewport stores `FollowTail`, `Pinned`, or `NewOutputBelow`; updating or
finalizing a cell marks output below while history remains resolved to the same
logical cell instead of preserving a fragile distance from the bottom.

## Redraw And Resize

`RedrawScheduler` classifies invalidations as immediate, next-frame, or
coalesced. High-frequency stream deltas share one frame on a 33,334 microsecond
minimum interval, which is strictly slower than the 30 FPS hard ceiling,
while input, scroll, resize, errors, and active-turn cancellation bypass the
throttle. Final stream state and control state are guaranteed on the next tick.

`record_draw` is called only after a successful terminal draw and retains a
test-visible count. The scheduler test injects 1,000 stream deltas over a fixed
one-second window using manually advanced `Instant` values, without sleeping,
then asserts at most 30 draws and a draw count far below the delta count.

Terminal resize rebuilds wrapped rows and resolves the existing cell/line
anchor against the new width and visible height. Long-token splitting operates
on Unicode grapheme clusters, so emoji ZWJ and combining sequences are never
split into half characters. The layout uses saturating region allocation;
extremely small terminals prioritize the bottom pane and skip zero-sized
render/cursor operations rather than overlapping or panicking.

The complete normalized `TestBackend` frames for 80x24, 100x30, 120x40, and
200x50 are stored in `crates/yunxi-agent-tui/src/snapshots`. Each frame contains
mixed CJK/kana/emoji/combining text, a long URL, a Windows path, a fenced code
block, active streaming, a stable pinned history anchor represented by
`new output below`, a bounded scrollbar, footer, and composer. Snapshot tests
assert full-frame equality, exact row count, display-width bounds, contiguous
regions, scrollbar containment, and required content. The explicit
`YUNXI_UPDATE_SNAPSHOTS=1` test-only update path regenerates these baselines;
normal tests only compare them.

## Active-Turn Cancellation

Raw-mode terminal input does not reach the process-level Ctrl+C signal handler,
so `YunxiTui::tick` polls crossterm events and returns
`CancelCurrentTurn` for Ctrl+C during the active render loop. The CLI invokes
`AgentRunControl::cancel`; the runtime races the provider stream future against
the cancellation notification and drops the provider future immediately when
cancelled. `Cancelled` freezes the current assistant cell as inactive, and the
REPL remains available for the next prompt. Ctrl+C outside an active turn keeps
the existing exit behavior.

## Compatibility

This boundary applies only to TUI presentation/storage. Core stream identity is
serde-skipped and does not alter plain CLI, JSON, or JSONL AgentEvent output.
Approval, user-input, follow-tail, and bottom-pane owners remain intact.
The CLI TUI renderer forwards events and cancellation actions; it does not
deduplicate text.

## Focus And Details Routing

v2.0.7 introduces `FocusTarget` values for Composer, History, Approval, and
Details and a pure `input_map` resolver that classifies Enter, Esc, Tab,
BackTab, Ctrl+C, PgUp/PgDown, paste, and wheel input before a view handles it.
History and Details consume non-applicable keys so they cannot submit or clear
the Composer underneath them. Approval approve/decline/cancel and selection
keys use the same resolver while keeping the safe default decline selection.

The Details layer renders redacted `DebugBuffer` content in the main content
region with an independent scroll offset. Opening and closing it does not
replace `BottomPaneMode`, mutate the grapheme-indexed `EditBuffer`, or touch
`TranscriptViewport`; the previous focus, Composer snapshot, Approval
selection, and transcript anchor are restored. Footer hints are generated from
the active focus and prioritize only close/scroll or submit/cancel actions that
are currently executable.

### v2.0.7-hotfix Mouse Routing

`resolve_event` now gives wheel input one unambiguous result per focus.
Composer and History return transcript scroll actions, Approval returns no-op,
and Details returns dedicated detail-scroll actions. The terminal host applies
the same focus check before scrollbar click, drag, and mouse-up state can touch
`TranscriptViewport`; a second host guard therefore remains effective even if
a future resolver change is incomplete.

Details wheel input and PgUp/PgDown share the existing `details_scroll` owner.
Approval never starts `TranscriptScrollDrag`, and entering or leaving Details
does not modify the transcript anchor, Composer snapshot, cursor, or approval
selection. The visible Approval hint lists selection/confirm/decline/cancel
only; Composer, History, and Details advertise only their actual scroll paths.

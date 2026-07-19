# TUI Presentation, Streaming Timeline, And Quiet Transcript

YunXi Agent v2.0.3-hotfix.1 keeps the v2.0.1 presentation boundary and v2.0.2-hotfix.1
streaming timeline, then adds reason-aware redraw scheduling and stable
cell-anchored history navigation.
Runtime events remain complete and ordered in `yunxi-agent-core`; only the TUI
maps them into user-facing cells.

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

The complete normalized `TestBackend` frames for 80x24 and 120x40 are stored in
`crates/yunxi-agent-tui/src/snapshots/full_frame_80x24.txt` and
`full_frame_120x40.txt`. Each frame contains active streaming, a stable pinned
history anchor represented by `new output below`, a bounded scrollbar, footer,
and composer. Snapshot tests assert full-frame equality, exact row count,
display-width bounds, contiguous regions, scrollbar containment, and required
content.

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

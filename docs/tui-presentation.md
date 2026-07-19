# TUI Presentation, Streaming Timeline, And Quiet Transcript

YunXi Agent v2.0.2 keeps the v2.0.1 presentation boundary and adds a structured
streaming timeline between presentation and transcript storage.
Runtime events remain complete and ordered in `yunxi-agent-core`; only the TUI
maps them into user-facing cells.

## Boundary

The data path is:

```text
AgentEvent
  -> TuiPresentation::present_agent_event
  -> TuiEvent { kind, safe text, optional detail, stream identity }
  -> TimelineStore { turn, stream session, source sequence, canonical cell }
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
turn ID, stream/message ID, source sequence, and started/delta/final phase.
Legacy producers receive a TUI-local turn and stream identity. This metadata is
not part of the JSON or JSONL wire shape.

`timeline_store.rs` owns `StreamSession`. Each session contains its canonical
cell ID, last accepted source sequence, state, accumulated content, and
`MarkdownStreamController`. The transitions are:

- started/delta: insert or update the active canonical cell;
- retry: reset partial content while reusing the active turn cell;
- final: replace the canonical text and mark that cell finalized;
- cancel: freeze the active cell and reject late events for that session;
- finish: commit an active delta-only provider response at the turn boundary.

Idempotency depends on turn, stream, sequence, and state, never on payload text.
A repeated final cannot create a second cell, while equal deltas with increasing
sequences are appended as legitimate content. A new stream after cancellation
receives a different cell so retry cannot bind to the frozen response.

Viewport state remains independent of stream state. Updating or finalizing a
cell calls `on_content_changed`, which marks new output below when the user is
reviewing history but preserves the offset instead of forcing follow-tail.

## Unchanged Interfaces

This boundary applies only to TUI presentation/storage. Core stream identity is
serde-skipped and does not alter plain CLI, JSON, or JSONL output. Approval,
user-input, cancel channels, tick, flush, resize, follow-tail, and bottom-pane
behavior retain their existing owners and contracts. The CLI TUI renderer only
forwards events; it does not deduplicate text.

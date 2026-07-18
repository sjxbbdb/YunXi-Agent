# TUI Presentation And Quiet Transcript

YunXi Agent v2.0.1 gives the terminal UI one explicit presentation boundary.
Runtime events remain complete and ordered in `yunxi-agent-core`; only the TUI
maps them into user-facing cells.

## Boundary

The data path is:

```text
AgentEvent
  -> TuiPresentation::present_agent_event
  -> TuiEvent { stable cell id, kind, safe text, optional detail }
  -> pure visibility filter
  -> Transcript / HistoryCell
  -> renderer
```

`presentation.rs` is the only layer that interprets `AgentEvent` semantics.
`event_filter.rs` decides only whether an already-classified event is visible
in the transcript, visible when debug mode is enabled, or hidden. `chat.rs`
stores and updates cells by stable ID. `render.rs` performs layout and styling
without reading runtime events.

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

Assistant `AgentEvent::Message` deltas enter `MarkdownStreamController` inside
the presentation layer. A `TuiStreamState` exposes committed `stable_source`
separately from the uncommitted `live_tail`. Contiguous deltas rewrite one
stable assistant cell. Any non-message event finalizes that stream so the next
assistant segment receives a new cell ID.

The controller deliberately preserves repeated payload text because v2.0.1
does not yet receive a provider message/delta identity that could distinguish
a legitimate repeated token from event replay. Stable cell IDs and explicit
stream state provide the foundation for identity-based idempotent writeback in
a later release.

## Unchanged Interfaces

This boundary applies only to TUI presentation. It does not filter or reorder
the core stream, plain CLI, JSON, or JSONL output. Approval, user-input, cancel,
tick, flush, resize, follow-tail, and bottom-pane behavior retain their existing
owners and contracts.

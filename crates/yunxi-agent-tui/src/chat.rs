use crate::debug::DebugBuffer;
use crate::event_filter::should_show;
use crate::presentation::{TuiCellId, TuiCellKind, TuiEvent};
use crate::timeline::{ToolTimelineEntry, ToolTimelineUpdate};
use crate::timeline_store::AssistantTimelineUpdate;

const MAX_HISTORY_CELLS: usize = 800;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct HistoryCell {
    pub(crate) id: TuiCellId,
    pub(crate) kind: HistoryCellKind,
    pub(crate) detail_id: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum HistoryCellKind {
    User(String),
    Assistant {
        content: String,
        active: bool,
    },
    Tool(ToolTimelineEntry),
    Event {
        kind: String,
        message: String,
    },
    Debug {
        id: usize,
        label: String,
        message: String,
    },
    Error(String),
}

impl HistoryCell {
    pub(crate) fn id(&self) -> &TuiCellId {
        &self.id
    }

    pub(crate) fn kind(&self) -> &HistoryCellKind {
        &self.kind
    }

    #[cfg(test)]
    pub(crate) fn detail_id(&self) -> Option<usize> {
        self.detail_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Transcript {
    cells: Vec<HistoryCell>,
    debug: DebugBuffer,
}

impl Default for Transcript {
    fn default() -> Self {
        Self {
            cells: Vec::new(),
            debug: DebugBuffer::default(),
        }
    }
}

impl Transcript {
    pub(crate) fn clear(&mut self) {
        self.cells.clear();
    }

    pub(crate) fn cells(&self) -> &[HistoryCell] {
        &self.cells
    }

    pub(crate) fn debug_status(&self) -> String {
        self.debug.status()
    }

    pub(crate) fn set_debug_events(&mut self, enabled: bool) {
        self.debug.set_enabled(enabled);
    }

    pub(crate) fn detail_text(&self, id: Option<usize>) -> String {
        self.debug.detail_text(id)
    }

    #[cfg(test)]
    pub(crate) fn render_line_count(&self) -> usize {
        self.cells
            .iter()
            .map(history_cell_line_count)
            .sum::<usize>()
            .max(1)
    }

    pub(crate) fn push_tui_event(&mut self, mut event: TuiEvent) {
        let source_id = event.id.clone();
        let detail_id = event.detail.take().map(|detail| self.debug.add(detail));
        if !should_show(&event, self.debug.enabled()) {
            return;
        }

        match event.kind {
            TuiCellKind::UserMessage => self.push_cell(HistoryCell {
                id: event.id,
                kind: HistoryCellKind::User(event.visible_text),
                detail_id,
            }),
            TuiCellKind::AssistantMessage => {
                self.push_assistant(event.id, event.visible_text, detail_id)
            }
            TuiCellKind::ToolStatusSummary | TuiCellKind::ApprovalRequest => {
                if let Some(update) = event.tool_update {
                    self.push_tool_update(event.id, update, detail_id);
                }
            }
            TuiCellKind::ProgressSummary | TuiCellKind::Notice => {
                self.push_cell(HistoryCell {
                    id: event.id,
                    kind: HistoryCellKind::Event {
                        kind: match event.kind {
                            TuiCellKind::ProgressSummary => "progress",
                            _ => "notice",
                        }
                        .to_string(),
                        message: event.visible_text,
                    },
                    detail_id,
                });
            }
            TuiCellKind::ErrorSummary => self.push_cell(HistoryCell {
                id: event.id,
                kind: HistoryCellKind::Error(event.visible_text),
                detail_id,
            }),
            TuiCellKind::DebugDetail => {
                if let Some(id) = detail_id {
                    self.push_debug_cell_if_enabled(&source_id, id);
                } else {
                    self.push_cell(HistoryCell {
                        id: event.id,
                        kind: HistoryCellKind::Event {
                            kind: "details".to_string(),
                            message: event.visible_text,
                        },
                        detail_id: None,
                    });
                }
                return;
            }
        }

        if self.debug.enabled()
            && let Some(id) = detail_id
        {
            self.push_debug_cell_if_enabled(&source_id, id);
        }
    }

    pub(crate) fn apply_assistant_update(&mut self, update: AssistantTimelineUpdate) -> bool {
        if let Some(cell) = self.cells.iter_mut().find(|cell| cell.id == update.cell_id)
            && let HistoryCellKind::Assistant { content, active } = &mut cell.kind
        {
            let changed = *content != update.content || *active != update.active;
            *content = update.content;
            *active = update.active;
            return changed;
        }
        if update.content.is_empty() {
            return false;
        }
        self.push_cell(HistoryCell {
            id: update.cell_id,
            kind: HistoryCellKind::Assistant {
                content: update.content,
                active: update.active,
            },
            detail_id: None,
        });
        true
    }

    fn push_assistant(&mut self, id: TuiCellId, content: String, detail_id: Option<usize>) {
        if content.trim().is_empty() {
            return;
        }
        if let Some(cell) = self.cells.iter_mut().find(|cell| cell.id == id)
            && let HistoryCellKind::Assistant {
                content: existing,
                active,
            } = &mut cell.kind
        {
            *existing = content;
            *active = true;
            cell.detail_id = detail_id.or(cell.detail_id);
            return;
        }

        self.finalize_assistant();
        self.push_cell(HistoryCell {
            id,
            kind: HistoryCellKind::Assistant {
                content,
                active: true,
            },
            detail_id,
        });
    }

    fn finalize_assistant(&mut self) {
        for cell in self.cells.iter_mut().rev() {
            if let HistoryCellKind::Assistant { active, .. } = &mut cell.kind {
                *active = false;
                break;
            }
        }
    }

    fn push_cell(&mut self, cell: HistoryCell) {
        self.cells.push(cell);
        if self.cells.len() > MAX_HISTORY_CELLS {
            let overflow = self.cells.len() - MAX_HISTORY_CELLS;
            self.cells.drain(0..overflow);
        }
    }

    fn push_tool_update(
        &mut self,
        id: TuiCellId,
        mut update: ToolTimelineUpdate,
        detail_id: Option<usize>,
    ) {
        if let Some(detail_id) = detail_id {
            update = update.detail_id(detail_id);
        }
        if let Some(cell) = self.cells.iter_mut().find(|cell| cell.id == id)
            && let HistoryCellKind::Tool(entry) = &mut cell.kind
        {
            entry.apply(update);
            cell.detail_id = detail_id.or(cell.detail_id);
            return;
        }
        self.push_cell(HistoryCell {
            id,
            kind: HistoryCellKind::Tool(ToolTimelineEntry::new(update)),
            detail_id,
        });
    }

    fn push_debug_cell_if_enabled(&mut self, source_id: &TuiCellId, id: usize) {
        if !self.debug.enabled() {
            return;
        }
        if let Some(message) = self.debug.inline_summary(id) {
            let label = self
                .debug
                .get(id)
                .map(|entry| entry.label.clone())
                .unwrap_or_else(|| "debug".to_string());
            self.push_cell(HistoryCell {
                id: source_id.with_suffix(&format!("debug-{id}")),
                kind: HistoryCellKind::Debug { id, label, message },
                detail_id: Some(id),
            });
        }
    }
}

#[cfg(test)]
fn history_cell_line_count(cell: &HistoryCell) -> usize {
    let content = match &cell.kind {
        HistoryCellKind::User(content)
        | HistoryCellKind::Assistant { content, .. }
        | HistoryCellKind::Error(content) => content,
        HistoryCellKind::Tool(entry) => return entry.display_text().lines().count().max(1),
        HistoryCellKind::Event { message, .. } => message,
        HistoryCellKind::Debug { message, .. } => message,
    };
    content.lines().count().max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presentation::TuiPresentation;
    use yunxi_agent_core::{AgentEvent, CommandStatus};

    fn push_event(
        presentation: &mut TuiPresentation,
        transcript: &mut Transcript,
        event: AgentEvent,
    ) {
        transcript.push_tui_event(presentation.present_agent_event(&event));
    }

    #[test]
    fn assistant_stream_rewrites_one_stable_history_cell() {
        let mut app = crate::app::YunxiTuiApp::default();
        app.push_agent_event(&AgentEvent::Message {
            content: "用户".to_string(),
            stream: None,
        });
        let stable_id = app.transcript().cells()[0].id().clone();
        app.push_agent_event(&AgentEvent::Message {
            content: "输入".to_string(),
            stream: None,
        });
        let transcript = app.transcript();

        assert_eq!(transcript.cells().len(), 1);
        assert_eq!(transcript.cells()[0].id(), &stable_id);
        assert_eq!(
            transcript.cells()[0].kind(),
            &HistoryCellKind::Assistant {
                content: "用户输入".to_string(),
                active: true,
            }
        );
    }

    #[test]
    fn render_line_count_tracks_multiline_cells() {
        let mut presentation = TuiPresentation::default();
        let mut transcript = Transcript::default();
        transcript.push_tui_event(presentation.present_user("one\ntwo"));
        transcript.push_tui_event(presentation.present_notice("tool", "three"));

        assert_eq!(transcript.render_line_count(), 3);
    }

    #[test]
    fn default_transcript_hides_internal_payloads_and_keeps_safe_tool_summary() {
        let mut presentation = TuiPresentation::default();
        let mut transcript = Transcript::default();
        for event in [
            AgentEvent::Reasoning {
                content: "raw thinking".to_string(),
            },
            AgentEvent::CommandUpdated {
                id: Some("call_1".to_string()),
                command: "provider_wire".to_string(),
                aggregated_output: "arguments_json full stdout".to_string(),
            },
            AgentEvent::ContextStatus {
                active_context_tokens: 42,
                token_limit_reached: false,
                compacted: false,
                dropped_messages: 0,
            },
            AgentEvent::ToolCallStarted {
                id: Some("call_1".to_string()),
                name: "skill: using-superpowers".to_string(),
                arguments_json: Some("{\"name\":\"using-superpowers\"}".to_string()),
            },
            AgentEvent::ToolCallCompleted {
                id: Some("call_1".to_string()),
                name: "skill: using-superpowers".to_string(),
                output: "name: using-superpowers\n<EXTREMELY-IMPORTANT>\nfull skill body"
                    .to_string(),
                status: CommandStatus::Completed,
            },
            AgentEvent::Error {
                message: "bottom exception stack\nsecret frame".to_string(),
            },
        ] {
            push_event(&mut presentation, &mut transcript, event);
        }

        let visible = transcript
            .cells()
            .iter()
            .map(cell_text)
            .collect::<Vec<_>>()
            .join("\n");

        for forbidden in [
            "raw thinking",
            "arguments_json",
            "full stdout",
            "active_context_tokens",
            "EXTREMELY-IMPORTANT",
            "secret frame",
        ] {
            assert!(
                !visible.contains(forbidden),
                "visible transcript leaked {forbidden}"
            );
        }
        assert!(visible.contains("skill using-superpowers"));
        assert!(visible.contains("running"));
        assert!(visible.contains("completed"));
        assert!(visible.contains("output captured"));
        assert!(visible.contains("agent operation failed"));
        assert!(transcript.debug_status().contains("hidden="));
    }

    #[test]
    fn debug_and_details_reveal_redacted_underlying_detail_by_stable_index() {
        let mut presentation = TuiPresentation::default();
        let mut transcript = Transcript::default();
        transcript.set_debug_events(true);
        push_event(
            &mut presentation,
            &mut transcript,
            AgentEvent::CommandUpdated {
                id: Some("wire".to_string()),
                command: "provider_wire".to_string(),
                aggregated_output: "token sk-secret-value".to_string(),
            },
        );

        let detail_id = transcript
            .cells()
            .iter()
            .find_map(HistoryCell::detail_id)
            .expect("detail id");
        let detail = transcript.detail_text(Some(detail_id));
        assert!(detail.contains("provider_wire"));
        assert!(detail.contains("sk-[redacted]"));
        assert!(!detail.contains("secret-value"));
    }

    fn cell_text(cell: &HistoryCell) -> String {
        match cell.kind() {
            HistoryCellKind::User(value) | HistoryCellKind::Error(value) => value.clone(),
            HistoryCellKind::Assistant { content, .. } => content.clone(),
            HistoryCellKind::Tool(entry) => entry.display_text(),
            HistoryCellKind::Event { kind, message } => format!("{kind}: {message}"),
            HistoryCellKind::Debug { message, .. } => message.clone(),
        }
    }
}

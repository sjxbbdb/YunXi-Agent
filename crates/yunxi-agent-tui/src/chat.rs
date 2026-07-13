use crate::debug::DebugBuffer;
use crate::event_filter::{FilteredEvent, classify};
use crate::streaming::append_fragment;
use crate::timeline::{ToolTimelineEntry, ToolTimelineUpdate};
use yunxi_agent_core::AgentEvent;

const MAX_HISTORY_CELLS: usize = 800;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum HistoryCell {
    User(String),
    Assistant {
        content: String,
        active: bool,
    },
    Reasoning {
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
    Warning(String),
    Error(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Transcript {
    cells: Vec<HistoryCell>,
    last_assistant_content: Option<String>,
    debug: DebugBuffer,
}

impl Default for Transcript {
    fn default() -> Self {
        Self {
            cells: Vec::new(),
            last_assistant_content: None,
            debug: DebugBuffer::default(),
        }
    }
}

impl Transcript {
    pub(crate) fn clear(&mut self) {
        self.cells.clear();
        self.last_assistant_content = None;
    }

    pub(crate) fn cells(&self) -> &[HistoryCell] {
        &self.cells
    }

    pub(crate) fn debug_status(&self) -> String {
        self.debug.status()
    }

    pub(crate) fn set_debug_events(&mut self, enabled: bool) {
        self.debug.set_enabled(enabled);
        self.push_notice(
            "debug",
            format!(
                "event debug {}",
                if enabled { "enabled" } else { "disabled" }
            ),
        );
    }

    pub(crate) fn push_details(&mut self, id: Option<usize>) {
        let message = self.debug.detail_text(id);
        self.push_notice("details", message);
    }

    #[cfg(test)]
    pub(crate) fn render_line_count(&self) -> usize {
        self.cells
            .iter()
            .map(history_cell_line_count)
            .sum::<usize>()
            .max(1)
    }

    pub(crate) fn push_user(&mut self, value: impl Into<String>) {
        self.finalize_streams();
        self.push_cell(HistoryCell::User(value.into()));
    }

    pub(crate) fn push_assistant(&mut self, value: &str) {
        let value = value.trim_matches('\r');
        if value.trim().is_empty() {
            return;
        }
        if self.last_assistant_content.as_deref() == Some(value) {
            return;
        }
        if let Some(HistoryCell::Assistant { content, active }) = self.cells.last_mut()
            && *active
        {
            if value.starts_with(content.as_str()) {
                *content = value.to_string();
            } else {
                append_fragment(content, value);
            }
            self.last_assistant_content = Some(content.clone());
            return;
        }
        self.finalize_reasoning();
        self.last_assistant_content = Some(value.to_string());
        self.push_cell(HistoryCell::Assistant {
            content: value.to_string(),
            active: true,
        });
    }

    pub(crate) fn push_reasoning(&mut self, value: &str) {
        let value = value.trim_matches('\r');
        if value.trim().is_empty() {
            return;
        }
        if let Some(HistoryCell::Reasoning { content, active }) = self.cells.last_mut()
            && *active
        {
            append_fragment(content, value);
            return;
        }
        self.finalize_assistant();
        self.push_cell(HistoryCell::Reasoning {
            content: value.to_string(),
            active: true,
        });
    }

    pub(crate) fn push_notice(&mut self, kind: impl Into<String>, message: impl Into<String>) {
        self.finalize_streams();
        self.push_cell(HistoryCell::Event {
            kind: kind.into(),
            message: message.into(),
        });
    }

    pub(crate) fn push_warning(&mut self, message: impl Into<String>) {
        self.finalize_streams();
        self.push_cell(HistoryCell::Warning(message.into()));
    }

    pub(crate) fn push_error(&mut self, message: impl Into<String>) {
        self.finalize_streams();
        self.push_cell(HistoryCell::Error(message.into()));
    }

    pub(crate) fn push_agent_event(&mut self, event: &AgentEvent) {
        match classify(event) {
            FilteredEvent::Assistant(content) => self.push_assistant(&content),
            FilteredEvent::Reasoning(content) => self.push_reasoning(&content),
            FilteredEvent::Tool(update) => self.push_tool_update(update),
            FilteredEvent::ToolOutput {
                update,
                label,
                summary,
            } => {
                let id = self.debug.add(label, summary.detail);
                self.push_tool_update(update.output_summary(summary.visible).detail_id(id));
                self.push_debug_cell_if_enabled(id);
            }
            FilteredEvent::Notice { kind, message } => self.push_notice(kind, message),
            FilteredEvent::NoticeWithDebug {
                kind,
                message,
                debug_label,
                debug_detail,
            } => {
                let id = self.debug.add(debug_label, debug_detail);
                self.push_notice(kind, message);
                self.push_debug_cell_if_enabled(id);
            }
            FilteredEvent::Warning(message) => self.push_warning(message),
            FilteredEvent::Error(message) => self.push_error(message),
            FilteredEvent::DebugOnly { label, detail } => {
                let id = self.debug.add(label, detail);
                self.push_debug_cell_if_enabled(id);
            }
            FilteredEvent::Suppress => {}
        }
    }

    fn finalize_streams(&mut self) {
        self.finalize_reasoning();
        self.finalize_assistant();
    }

    fn finalize_reasoning(&mut self) {
        if let Some(HistoryCell::Reasoning { active, .. }) = self.cells.last_mut() {
            *active = false;
        }
    }

    fn finalize_assistant(&mut self) {
        if let Some(HistoryCell::Assistant { active, .. }) = self.cells.last_mut() {
            *active = false;
        }
    }

    fn push_cell(&mut self, cell: HistoryCell) {
        self.cells.push(cell);
        if self.cells.len() > MAX_HISTORY_CELLS {
            let overflow = self.cells.len() - MAX_HISTORY_CELLS;
            self.cells.drain(0..overflow);
        }
    }

    fn push_tool_update(&mut self, update: ToolTimelineUpdate) {
        self.finalize_streams();
        if let Some(HistoryCell::Tool(entry)) =
            self.cells.iter_mut().rev().find(
                |cell| matches!(cell, HistoryCell::Tool(entry) if entry.is_same_tool(&update)),
            )
        {
            entry.apply(update);
            return;
        }
        self.push_cell(HistoryCell::Tool(ToolTimelineEntry::new(update)));
    }

    fn push_debug_cell_if_enabled(&mut self, id: usize) {
        if !self.debug.enabled() {
            return;
        }
        if let Some(message) = self.debug.inline_summary(id) {
            let label = self
                .debug
                .get(id)
                .map(|entry| entry.label.clone())
                .unwrap_or_else(|| "debug".to_string());
            self.finalize_streams();
            self.push_cell(HistoryCell::Debug { id, label, message });
        }
    }
}

#[cfg(test)]
fn history_cell_line_count(cell: &HistoryCell) -> usize {
    let content = match cell {
        HistoryCell::User(content)
        | HistoryCell::Assistant { content, .. }
        | HistoryCell::Reasoning { content, .. }
        | HistoryCell::Warning(content)
        | HistoryCell::Error(content) => content,
        HistoryCell::Tool(entry) => return entry.display_text().lines().count().max(1),
        HistoryCell::Event { message, .. } => message,
        HistoryCell::Debug { message, .. } => message,
    };
    content.lines().count().max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yunxi_agent_core::CommandStatus;

    #[test]
    fn reasoning_deltas_are_merged_into_one_cell() {
        let mut transcript = Transcript::default();
        for delta in ["用户", "输入", "了", "\"", "测试", "\""] {
            transcript.push_reasoning(delta);
        }
        assert_eq!(transcript.cells().len(), 1);
        assert_eq!(
            transcript.cells()[0],
            HistoryCell::Reasoning {
                content: "用户输入了\"测试\"".to_string(),
                active: true
            }
        );
    }

    #[test]
    fn duplicate_assistant_messages_are_suppressed() {
        let mut transcript = Transcript::default();
        transcript.push_assistant("hello");
        transcript.push_assistant("hello");
        assert_eq!(transcript.cells().len(), 1);
    }

    #[test]
    fn render_line_count_tracks_multiline_cells() {
        let mut transcript = Transcript::default();
        assert_eq!(transcript.render_line_count(), 1);

        transcript.push_user("one\ntwo");
        transcript.push_notice("tool", "three");

        assert_eq!(transcript.render_line_count(), 3);
    }

    #[test]
    fn event_filter_hides_protocol_stdout_context_and_long_skill_output() {
        let mut transcript = Transcript::default();
        transcript.push_agent_event(&AgentEvent::CommandUpdated {
            id: Some("call_1".to_string()),
            command: "skill using-superpowers".to_string(),
            aggregated_output: "{\"arguments_json\":\"{\\\"name\\\":\\\"using-superpowers\\\"}\"}"
                .to_string(),
        });
        transcript.push_agent_event(&AgentEvent::ContextStatus {
            active_context_tokens: 42,
            token_limit_reached: false,
            compacted: false,
            dropped_messages: 0,
        });
        transcript.push_agent_event(&AgentEvent::ApprovalRequested {
            id: Some("call_1".to_string()),
            tool_name: "skill: using-superpowers".to_string(),
            reason: "tool execution requires approval".to_string(),
        });
        transcript.push_agent_event(&AgentEvent::ApprovalCompleted {
            id: Some("call_1".to_string()),
            approved: true,
            reason: Some("approved".to_string()),
        });
        transcript.push_agent_event(&AgentEvent::ToolCallStarted {
            id: Some("call_1".to_string()),
            name: "skill: using-superpowers".to_string(),
            arguments_json: Some("{\"name\":\"using-superpowers\"}".to_string()),
        });
        transcript.push_agent_event(&AgentEvent::ToolCallCompleted {
            id: Some("call_1".to_string()),
            name: "skill: using-superpowers".to_string(),
            output: "name: using-superpowers\n<EXTREMELY-IMPORTANT>\nfull skill body".to_string(),
            status: CommandStatus::Completed,
        });

        let visible = transcript
            .cells()
            .iter()
            .map(debug_cell_text)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(!visible.contains("arguments_json"));
        assert!(!visible.contains("EXTREMELY-IMPORTANT"));
        assert!(!visible.contains("tokens=42"));
        assert!(visible.contains("skill using-superpowers"));
        assert!(visible.contains("approval required"));
        assert!(visible.contains("approved"));
        assert!(visible.contains("running"));
        assert!(visible.contains("completed"));
        assert!(visible.contains("output hidden"));
        assert!(visible.contains("details #"));
    }

    fn debug_cell_text(cell: &HistoryCell) -> String {
        match cell {
            HistoryCell::User(value) | HistoryCell::Warning(value) | HistoryCell::Error(value) => {
                value.clone()
            }
            HistoryCell::Assistant { content, .. } | HistoryCell::Reasoning { content, .. } => {
                content.clone()
            }
            HistoryCell::Tool(entry) => entry.display_text(),
            HistoryCell::Event { kind, message } => format!("{kind}: {message}"),
            HistoryCell::Debug { message, .. } => message.clone(),
        }
    }
}

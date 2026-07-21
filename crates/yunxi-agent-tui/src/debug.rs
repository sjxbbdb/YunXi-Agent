use crate::output_summary::{
    MAX_STORED_DETAIL_GRAPHEMES, debug_inline_summary, detail_display, redact_secrets,
    truncate_graphemes_with_notice,
};
use crate::presentation::{PresentationDetail, TuiCellId};

const MAX_DEBUG_ENTRIES: usize = 200;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DebugEntry {
    pub(crate) id: usize,
    pub(crate) stable_id: TuiCellId,
    pub(crate) label: String,
    pub(crate) detail: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DebugBuffer {
    enabled: bool,
    entries: Vec<DebugEntry>,
    next_id: usize,
    hidden_count: usize,
}

impl Default for DebugBuffer {
    fn default() -> Self {
        Self {
            enabled: std::env::var("YUNXI_TUI_DEBUG_EVENTS")
                .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "on" | "ON"))
                .unwrap_or(false),
            entries: Vec::new(),
            next_id: 1,
            hidden_count: 0,
        }
    }
}

impl DebugBuffer {
    pub(crate) fn enabled(&self) -> bool {
        self.enabled
    }

    pub(crate) fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub(crate) fn add(&mut self, detail: PresentationDetail) -> usize {
        let label = truncate_graphemes_with_notice(&detail.label, 256, false);
        let redacted = redact_secrets(&detail.content);
        let content = truncate_graphemes_with_notice(&redacted, MAX_STORED_DETAIL_GRAPHEMES, true);
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.stable_id == detail.id)
        {
            entry.label = label;
            entry.detail = content;
            return entry.id;
        }

        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.hidden_count = self.hidden_count.saturating_add(1);

        self.entries.push(DebugEntry {
            id,
            stable_id: detail.id,
            label,
            detail: content,
        });
        if self.entries.len() > MAX_DEBUG_ENTRIES {
            let overflow = self.entries.len() - MAX_DEBUG_ENTRIES;
            self.entries.drain(0..overflow);
        }
        id
    }

    pub(crate) fn get(&self, id: usize) -> Option<&DebugEntry> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    pub(crate) fn latest(&self) -> Option<&DebugEntry> {
        self.entries.last()
    }

    pub(crate) fn status(&self) -> String {
        format!(
            "debug={} hidden={}",
            if self.enabled { "on" } else { "off" },
            self.hidden_count
        )
    }

    pub(crate) fn detail_text(&self, id: Option<usize>) -> String {
        let entry = match id {
            Some(id) => self.get(id),
            None => self.latest(),
        };
        match entry {
            Some(entry) => detail_display(
                &format!("debug #{} {}", entry.id, entry.label),
                &entry.detail,
            ),
            None => "no debug details available".to_string(),
        }
    }

    pub(crate) fn inline_summary(&self, id: usize) -> Option<String> {
        let entry = self.get(id)?;
        Some(format!(
            "#{} {}: {}",
            entry.id,
            entry.label,
            debug_inline_summary(&entry.detail)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_debug_entries_with_redaction() {
        let mut buffer = DebugBuffer::default();
        let id = buffer.add(PresentationDetail {
            id: TuiCellId::from_test("detail:stdout"),
            label: "stdout".to_string(),
            content: "token sk-secret-value".to_string(),
        });

        let text = buffer.detail_text(Some(id));
        assert!(text.contains("sk-[redacted]"));
        assert!(!text.contains("secret-value"));
    }

    #[test]
    fn stable_detail_id_updates_in_place_without_growing_hidden_count() {
        let mut buffer = DebugBuffer::default();
        let stable_id = TuiCellId::from_test("detail:stream");
        let first = buffer.add(PresentationDetail {
            id: stable_id.clone(),
            label: "stream".to_string(),
            content: "first".to_string(),
        });
        let second = buffer.add(PresentationDetail {
            id: stable_id,
            label: "stream".to_string(),
            content: "second".to_string(),
        });

        assert_eq!(first, second);
        assert!(buffer.detail_text(Some(first)).contains("second"));
        assert_eq!(buffer.status(), "debug=off hidden=1");
    }

    #[test]
    fn debug_detail_is_redacted_before_bounded_storage() {
        let mut buffer = DebugBuffer::default();
        let secret = "sk-secret-value";
        let content = format!(
            "{secret}\n{}tail",
            "x".repeat(MAX_STORED_DETAIL_GRAPHEMES + 500)
        );
        let id = buffer.add(PresentationDetail {
            id: TuiCellId::from_test("detail:large"),
            label: "provider".to_string(),
            content,
        });
        let entry = buffer.get(id).expect("stored entry");

        assert!(!entry.detail.contains("secret-value"));
        assert!(entry.detail.contains("older content truncated"));
        assert!(entry.detail.ends_with("tail"));
        assert!(
            unicode_segmentation::UnicodeSegmentation::graphemes(entry.detail.as_str(), true)
                .count()
                <= MAX_STORED_DETAIL_GRAPHEMES
        );
    }
}

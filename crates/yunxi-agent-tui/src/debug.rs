use crate::output_summary::{debug_inline_summary, detail_display, redact_secrets};

const MAX_DEBUG_ENTRIES: usize = 200;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DebugEntry {
    pub(crate) id: usize,
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

    pub(crate) fn add(&mut self, label: impl Into<String>, detail: impl Into<String>) -> usize {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.hidden_count = self.hidden_count.saturating_add(1);

        self.entries.push(DebugEntry {
            id,
            label: label.into(),
            detail: redact_secrets(&detail.into()),
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
        let id = buffer.add("stdout", "token sk-secret-value");

        let text = buffer.detail_text(Some(id));
        assert!(text.contains("sk-[redacted]"));
        assert!(!text.contains("secret-value"));
    }
}

use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct EditBuffer {
    text: String,
    cursor_grapheme: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct EditBufferSnapshot {
    text: String,
    cursor_grapheme: usize,
}

impl EditBuffer {
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn cursor_grapheme(&self) -> usize {
        self.cursor_grapheme
    }

    pub(crate) fn grapheme_len(&self) -> usize {
        self.text.graphemes(true).count()
    }

    pub(crate) fn cursor_byte_offset(&self) -> usize {
        byte_offset_for_grapheme(&self.text, self.cursor_grapheme)
    }

    pub(crate) fn insert_text(&mut self, value: &str) {
        let value = normalize_newlines(value);
        if value.is_empty() {
            return;
        }
        let byte_offset = self.cursor_byte_offset();
        self.text.insert_str(byte_offset, &value);
        let inserted_end = byte_offset.saturating_add(value.len());
        self.cursor_grapheme = grapheme_index_after_byte(&self.text, inserted_end);
    }

    pub(crate) fn insert_newline(&mut self) {
        self.insert_text("\n");
    }

    pub(crate) fn delete_previous_grapheme(&mut self) {
        if self.cursor_grapheme == 0 || self.text.is_empty() {
            return;
        }
        let start = byte_offset_for_grapheme(&self.text, self.cursor_grapheme - 1);
        let end = self.cursor_byte_offset();
        self.text.drain(start..end);
        self.cursor_grapheme -= 1;
    }

    pub(crate) fn delete_next_grapheme(&mut self) {
        if self.cursor_grapheme >= self.grapheme_len() {
            return;
        }
        let start = self.cursor_byte_offset();
        let end = byte_offset_for_grapheme(&self.text, self.cursor_grapheme + 1);
        self.text.drain(start..end);
    }

    pub(crate) fn move_left(&mut self) {
        self.cursor_grapheme = self.cursor_grapheme.saturating_sub(1);
    }

    pub(crate) fn move_right(&mut self) {
        self.cursor_grapheme = self
            .cursor_grapheme
            .saturating_add(1)
            .min(self.grapheme_len());
    }

    pub(crate) fn move_home(&mut self) {
        self.cursor_grapheme = 0;
    }

    pub(crate) fn move_end(&mut self) {
        self.cursor_grapheme = self.grapheme_len();
    }

    pub(crate) fn clear(&mut self) {
        self.text.clear();
        self.cursor_grapheme = 0;
    }

    pub(crate) fn submit_text(&mut self) -> String {
        let submitted = self.text.trim_end_matches('\n').to_string();
        self.clear();
        submitted
    }

    pub(crate) fn snapshot(&self) -> EditBufferSnapshot {
        EditBufferSnapshot {
            text: self.text.clone(),
            cursor_grapheme: self.cursor_grapheme,
        }
    }

    pub(crate) fn restore(&mut self, snapshot: EditBufferSnapshot) {
        self.text = normalize_newlines(&snapshot.text);
        self.cursor_grapheme = snapshot.cursor_grapheme.min(self.grapheme_len());
    }
}

fn byte_offset_for_grapheme(value: &str, grapheme_index: usize) -> usize {
    value
        .grapheme_indices(true)
        .nth(grapheme_index)
        .map(|(index, _)| index)
        .unwrap_or(value.len())
}

fn grapheme_index_after_byte(value: &str, byte_offset: usize) -> usize {
    let byte_offset = byte_offset.min(value.len());
    value
        .grapheme_indices(true)
        .take_while(|(index, _)| *index < byte_offset)
        .count()
}

fn normalize_newlines(value: &str) -> String {
    value.replace("\r\n", "\n").replace('\r', "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_and_deletion_use_grapheme_indices() {
        let mut buffer = EditBuffer::default();
        buffer.insert_text("中文👩‍💻e\u{301}");

        assert_eq!(buffer.grapheme_len(), 4);
        assert_eq!(buffer.cursor_grapheme(), 4);
        buffer.delete_previous_grapheme();
        buffer.delete_previous_grapheme();

        assert_eq!(buffer.text(), "中文");
        assert_eq!(buffer.cursor_grapheme(), 2);
        assert_eq!(buffer.cursor_byte_offset(), "中文".len());
    }

    #[test]
    fn insert_handles_combining_clusters_without_exposing_byte_cursor() {
        let mut buffer = EditBuffer::default();
        buffer.insert_text("e");
        buffer.insert_text("\u{301}");

        assert_eq!(buffer.text(), "e\u{301}");
        assert_eq!(buffer.grapheme_len(), 1);
        assert_eq!(buffer.cursor_grapheme(), 1);
    }

    #[test]
    fn paste_normalizes_crlf_and_lone_carriage_returns() {
        let mut buffer = EditBuffer::default();
        buffer.insert_text("第一行\r\nsecond\r第三行");

        assert_eq!(buffer.text(), "第一行\nsecond\n第三行");
        assert!(!buffer.text().contains('\r'));
    }

    #[test]
    fn home_end_delete_and_snapshot_restore_round_trip() {
        let mut buffer = EditBuffer::default();
        buffer.insert_text("A👩‍💻中");
        let end = buffer.snapshot();
        buffer.move_home();
        buffer.delete_next_grapheme();
        assert_eq!(buffer.text(), "👩‍💻中");

        buffer.restore(end);
        buffer.move_left();
        buffer.delete_next_grapheme();
        assert_eq!(buffer.text(), "A👩‍💻");
        assert_eq!(buffer.cursor_grapheme(), 2);
    }

    #[test]
    fn long_ime_style_committed_text_remains_lossless() {
        let input = "输入法已提交👨‍👩‍👧‍👦e\u{301}".repeat(256);
        let mut buffer = EditBuffer::default();
        buffer.insert_text(&input);

        assert_eq!(buffer.text(), input);
        assert_eq!(buffer.cursor_grapheme(), buffer.grapheme_len());
    }
}

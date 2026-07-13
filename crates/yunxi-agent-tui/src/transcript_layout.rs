use crate::chat::HistoryCell;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const CONTINUATION_GUTTER: &str = "  | ";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WrappedTranscript {
    pub(crate) rows: Vec<Line<'static>>,
    pub(crate) logical_cells: usize,
}

pub(crate) fn build_wrapped_transcript(cells: &[HistoryCell], width: usize) -> WrappedTranscript {
    let mut rows = Vec::new();
    let width = width.max(1);
    for cell in cells {
        push_cell_rows(&mut rows, cell, width);
    }
    if rows.is_empty() {
        rows.push(Line::from(Span::styled(
            "Ready.",
            Style::default().fg(Color::DarkGray),
        )));
    }

    WrappedTranscript {
        rows,
        logical_cells: cells.len(),
    }
}

fn push_cell_rows(rows: &mut Vec<Line<'static>>, cell: &HistoryCell, width: usize) {
    match cell {
        HistoryCell::User(content) => push_labeled(rows, "user", Color::Green, content, width),
        HistoryCell::Assistant { content, active } => {
            let label = if *active { "assistant*" } else { "assistant" };
            push_labeled(rows, label, Color::LightGreen, content, width);
        }
        HistoryCell::Reasoning { content, active } => {
            let label = if *active { "thinking*" } else { "thinking" };
            push_labeled(rows, label, Color::Blue, content, width);
        }
        HistoryCell::Tool(entry) => {
            push_labeled(rows, "tool", Color::Magenta, &entry.display_text(), width)
        }
        HistoryCell::Event { kind, message } => {
            push_labeled(rows, kind, label_color(kind), message, width)
        }
        HistoryCell::Debug { id, label, message } => push_labeled(
            rows,
            "debug",
            Color::DarkGray,
            &format!("#{id} {label}: {message}"),
            width,
        ),
        HistoryCell::Warning(message) => {
            push_labeled(rows, "warning", Color::Yellow, message, width)
        }
        HistoryCell::Error(message) => push_labeled(rows, "error", Color::Red, message, width),
    }
}

fn push_labeled(
    rows: &mut Vec<Line<'static>>,
    label: &str,
    color: Color,
    content: &str,
    width: usize,
) {
    let label_prefix = format!("[{label}] ");
    let label_style = Style::default().fg(color).add_modifier(Modifier::BOLD);
    let gutter_style = Style::default().fg(Color::DarkGray);

    let mut line_iter = content.lines();
    let first = line_iter.next().unwrap_or("");
    push_wrapped_text(
        rows,
        StyledPrefix::new(label_prefix.clone(), label_style),
        first,
        width,
    );

    for rest in line_iter {
        push_wrapped_text(
            rows,
            StyledPrefix::new(CONTINUATION_GUTTER.to_string(), gutter_style),
            rest,
            width,
        );
    }
}

#[derive(Clone, Debug)]
struct StyledPrefix {
    text: String,
    style: Style,
}

impl StyledPrefix {
    fn new(text: String, style: Style) -> Self {
        Self { text, style }
    }
}

fn push_wrapped_text(
    rows: &mut Vec<Line<'static>>,
    first_prefix: StyledPrefix,
    text: &str,
    width: usize,
) {
    let gutter = StyledPrefix::new(
        CONTINUATION_GUTTER.to_string(),
        Style::default().fg(Color::DarkGray),
    );
    let first_capacity = content_capacity(width, &first_prefix.text);
    let rest_capacity = content_capacity(width, &gutter.text);
    let chunks = split_display_width(text, first_capacity, rest_capacity);

    for (idx, chunk) in chunks.into_iter().enumerate() {
        let prefix = if idx == 0 { &first_prefix } else { &gutter };
        rows.push(Line::from(vec![
            Span::styled(prefix.text.clone(), prefix.style),
            Span::raw(chunk),
        ]));
    }
}

fn content_capacity(width: usize, prefix: &str) -> usize {
    width.saturating_sub(UnicodeWidthStr::width(prefix)).max(1)
}

fn split_display_width(text: &str, first_width: usize, rest_width: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_width: usize = 0;
    let mut capacity = first_width.max(1);

    for ch in text.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if !current.is_empty() && current_width.saturating_add(ch_width) > capacity {
            chunks.push(current);
            current = String::new();
            current_width = 0;
            capacity = rest_width.max(1);
        }
        current.push(ch);
        current_width = current_width.saturating_add(ch_width);
    }

    if current.is_empty() && chunks.is_empty() {
        chunks.push(String::new());
    } else if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn label_color(label: &str) -> Color {
    match label {
        "shell" | "tool" | "mcp" => Color::Magenta,
        "approval" | "escalation" => Color::Yellow,
        "context" | "session" | "usage" | "debug" | "details" => Color::Gray,
        "cancelled" | "provider" => Color::Red,
        _ => Color::White,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row_text(line: &Line<'static>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>()
    }

    #[test]
    fn wraps_long_cjk_text_into_screen_rows() {
        let cells = vec![HistoryCell::Assistant {
            content: "你好世界你好世界你好世界".to_string(),
            active: false,
        }];

        let wrapped = build_wrapped_transcript(&cells, 12);

        assert!(wrapped.rows.len() > 1);
        assert!(row_text(&wrapped.rows[0]).starts_with("[assistant] "));
        assert!(row_text(&wrapped.rows[1]).starts_with(CONTINUATION_GUTTER));
    }

    #[test]
    fn wraps_long_ascii_token_without_paragraph_wrap() {
        let cells = vec![HistoryCell::User("abcdefghijklmnopqrstuvwxyz".to_string())];

        let wrapped = build_wrapped_transcript(&cells, 14);

        assert!(wrapped.rows.len() > 1);
        assert!(row_text(&wrapped.rows[0]).contains("[user] "));
        assert!(row_text(&wrapped.rows[1]).starts_with(CONTINUATION_GUTTER));
    }

    #[test]
    fn preserves_multiline_continuation_gutter() {
        let cells = vec![HistoryCell::Reasoning {
            content: "first\nsecond".to_string(),
            active: true,
        }];

        let wrapped = build_wrapped_transcript(&cells, 80);

        assert_eq!(row_text(&wrapped.rows[0]), "[thinking*] first");
        assert_eq!(row_text(&wrapped.rows[1]), "  | second");
    }
}

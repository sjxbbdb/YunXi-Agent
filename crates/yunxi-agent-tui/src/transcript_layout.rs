use crate::chat::{HistoryCell, HistoryCellKind};
use crate::presentation::TuiCellId;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

const CONTINUATION_GUTTER: &str = "    ";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WrappedTranscript {
    pub(crate) rows: Vec<Line<'static>>,
    pub(crate) logical_cells: usize,
    row_anchors: Vec<Option<TranscriptRowAnchor>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TranscriptRowAnchor {
    pub(crate) cell_id: TuiCellId,
    pub(crate) line_offset: usize,
}

impl WrappedTranscript {
    pub(crate) fn anchor_at(&self, row: usize) -> Option<&TranscriptRowAnchor> {
        self.row_anchors.get(row).and_then(Option::as_ref)
    }

    pub(crate) fn resolve_anchor(&self, cell_id: &TuiCellId, line_offset: usize) -> Option<usize> {
        self.row_anchors
            .iter()
            .enumerate()
            .filter_map(|(row, anchor)| {
                let anchor = anchor.as_ref()?;
                (anchor.cell_id == *cell_id)
                    .then_some((row, anchor.line_offset.abs_diff(line_offset)))
            })
            .min_by_key(|(_, distance)| *distance)
            .map(|(row, _)| row)
    }
}

pub(crate) fn build_wrapped_transcript(cells: &[HistoryCell], width: usize) -> WrappedTranscript {
    let mut rows = Vec::new();
    let mut row_anchors = Vec::new();
    let width = width.max(1);
    for cell in cells {
        let first_row = rows.len();
        push_cell_rows(&mut rows, cell, width);
        row_anchors.extend(
            (0..rows.len().saturating_sub(first_row)).map(|line_offset| {
                Some(TranscriptRowAnchor {
                    cell_id: cell.id().clone(),
                    line_offset,
                })
            }),
        );
    }
    if rows.is_empty() {
        rows.push(Line::from(Span::styled(
            "Ready.",
            Style::default().fg(Color::DarkGray),
        )));
        row_anchors.push(None);
    }

    WrappedTranscript {
        rows,
        logical_cells: cells.len(),
        row_anchors,
    }
}

fn push_cell_rows(rows: &mut Vec<Line<'static>>, cell: &HistoryCell, width: usize) {
    match cell.kind() {
        HistoryCellKind::User(content) => push_labeled(rows, "user", Color::Green, content, width),
        HistoryCellKind::Assistant { content, active } => {
            let label = if *active { "assistant*" } else { "assistant" };
            push_labeled(rows, label, Color::LightGreen, content, width);
        }
        HistoryCellKind::Tool(entry) => {
            push_labeled(rows, "tool", Color::Magenta, &entry.display_text(), width)
        }
        HistoryCellKind::Event { kind, message } => {
            push_labeled(rows, kind, label_color(kind), message, width)
        }
        HistoryCellKind::Debug { id, label, message } => push_labeled(
            rows,
            "debug",
            Color::DarkGray,
            &format!("#{id} {label}: {message}"),
            width,
        ),
        HistoryCellKind::Error(message) => push_labeled(rows, "error", Color::Red, message, width),
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
    let mut capacity = first_width.max(1);
    let mut pending_space = false;

    for token in word_tokens(text) {
        if token.chars().all(char::is_whitespace) {
            pending_space |= !current.is_empty();
            continue;
        }

        let separator_width = usize::from(pending_space && !current.is_empty());
        let token_width = UnicodeWidthStr::width(token);
        let current_width = UnicodeWidthStr::width(current.as_str());
        if !current.is_empty()
            && current_width
                .saturating_add(separator_width)
                .saturating_add(token_width)
                <= capacity
        {
            if pending_space {
                current.push(' ');
            }
            current.push_str(token);
            pending_space = false;
            continue;
        }

        if !current.is_empty() {
            chunks.push(std::mem::take(&mut current));
            capacity = rest_width.max(1);
        }
        pending_space = false;

        if token_width <= capacity {
            current.push_str(token);
            continue;
        }

        let mut pieces = split_long_token(token, capacity, rest_width.max(1));
        if pieces.len() > 1 {
            chunks.extend(pieces.drain(..pieces.len() - 1));
            capacity = rest_width.max(1);
        }
        if let Some(last) = pieces.pop() {
            current = last;
        }
    }

    if current.is_empty() && chunks.is_empty() {
        chunks.push(String::new());
    } else if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn word_tokens(text: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut start = 0usize;
    let mut last_whitespace: Option<bool> = None;
    for (idx, ch) in text.char_indices() {
        let whitespace = ch.is_whitespace();
        if last_whitespace.is_some_and(|last| last != whitespace) {
            tokens.push(&text[start..idx]);
            start = idx;
        }
        last_whitespace = Some(whitespace);
    }
    if start < text.len() {
        tokens.push(&text[start..]);
    }
    tokens
}

fn split_long_token(token: &str, first_width: usize, rest_width: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;
    let mut capacity = first_width.max(1);

    for grapheme in token.graphemes(true) {
        let grapheme_width = UnicodeWidthStr::width(grapheme);
        if !current.is_empty() && current_width.saturating_add(grapheme_width) > capacity {
            chunks.push(std::mem::take(&mut current));
            current = String::new();
            current_width = 0;
            capacity = rest_width.max(1);
        }
        current.push_str(grapheme);
        current_width = current_width.saturating_add(grapheme_width);
    }
    if !current.is_empty() {
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
    use crate::presentation::TuiCellId;

    fn cell(kind: HistoryCellKind) -> HistoryCell {
        HistoryCell {
            id: TuiCellId::from_test("layout-test"),
            kind,
            detail_id: None,
        }
    }

    fn row_text(line: &Line<'static>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>()
    }

    #[test]
    fn wraps_long_cjk_text_into_screen_rows() {
        let cells = vec![cell(HistoryCellKind::Assistant {
            content: "你好世界你好世界你好世界".to_string(),
            active: false,
        })];

        let wrapped = build_wrapped_transcript(&cells, 12);

        assert!(wrapped.rows.len() > 1);
        assert!(row_text(&wrapped.rows[0]).starts_with("[assistant] "));
        assert!(row_text(&wrapped.rows[1]).starts_with(CONTINUATION_GUTTER));
    }

    #[test]
    fn wraps_long_ascii_token_without_paragraph_wrap() {
        let cells = vec![cell(HistoryCellKind::User(
            "abcdefghijklmnopqrstuvwxyz".to_string(),
        ))];

        let wrapped = build_wrapped_transcript(&cells, 14);

        assert!(wrapped.rows.len() > 1);
        assert!(row_text(&wrapped.rows[0]).contains("[user] "));
        assert!(row_text(&wrapped.rows[1]).starts_with(CONTINUATION_GUTTER));
    }

    #[test]
    fn preserves_multiline_continuation_gutter() {
        let cells = vec![cell(HistoryCellKind::Event {
            kind: "progress".to_string(),
            message: "first\nsecond".to_string(),
        })];

        let wrapped = build_wrapped_transcript(&cells, 80);

        assert_eq!(row_text(&wrapped.rows[0]), "[progress] first");
        assert_eq!(row_text(&wrapped.rows[1]), "    second");
    }

    #[test]
    fn wraps_ascii_on_word_boundaries_when_possible() {
        let cells = vec![cell(HistoryCellKind::Assistant {
            content: "Tool output summaries stay readable".to_string(),
            active: false,
        })];

        let wrapped = build_wrapped_transcript(&cells, 28);
        let rendered = wrapped.rows.iter().map(row_text).collect::<Vec<_>>();

        assert!(rendered.iter().any(|row| row.contains("Tool output")));
        assert!(
            !rendered
                .iter()
                .any(|row| row.trim_start().starts_with("ol output"))
        );
    }

    #[test]
    fn preserves_cjk_english_without_inserting_spaces() {
        let cells = vec![cell(HistoryCellKind::User(
            "Summarize this 中文 and English mixed terminal output.".to_string(),
        ))];

        let wrapped = build_wrapped_transcript(&cells, 44);
        let rendered = wrapped.rows.iter().map(row_text).collect::<Vec<_>>();

        assert!(rendered.iter().any(|row| row.contains("中文 and")));
        assert!(!rendered.iter().any(|row| row.contains("中 文")));
    }

    #[test]
    fn maps_wrapped_rows_back_to_stable_cells() {
        let first = cell(HistoryCellKind::User(
            "abcdefghijklmnopqrstuvwxyz".to_string(),
        ));
        let first_id = first.id().clone();
        let second = HistoryCell {
            id: TuiCellId::from_test("layout-second"),
            kind: HistoryCellKind::Assistant {
                content: "answer".to_string(),
                active: false,
            },
            detail_id: None,
        };

        let wrapped = build_wrapped_transcript(&[first, second], 14);
        let anchored_row = wrapped
            .resolve_anchor(&first_id, 1)
            .expect("second wrapped row");

        assert_eq!(wrapped.anchor_at(anchored_row).unwrap().cell_id, first_id);
        assert_eq!(wrapped.anchor_at(anchored_row).unwrap().line_offset, 1);
    }

    #[test]
    fn never_splits_emoji_or_combining_graphemes() {
        let emoji = "👩‍💻";
        let combining = "e\u{301}";
        let cells = vec![cell(HistoryCellKind::User(format!(
            "{emoji}{emoji}{combining}{combining}"
        )))];

        for width in 1..=12 {
            let wrapped = build_wrapped_transcript(&cells, width);
            let body = wrapped.rows.iter().map(row_text).collect::<String>();
            assert_eq!(body.matches(emoji).count(), 2, "width={width}");
            assert_eq!(body.matches(combining).count(), 2, "width={width}");
        }
    }
}

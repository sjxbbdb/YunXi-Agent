use crate::chat::{HistoryCell, HistoryCellKind};
use crate::presentation::TuiCellId;
use crate::styles::{TuiSemanticStyle, TuiStyle, TuiStyleSet};
use crate::text_layout::{TextLayout, WrapPolicy};
use crate::timeline::ToolPhase;
use ratatui::text::{Line, Span};

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
    build_wrapped_transcript_with_styles(cells, width, TuiStyleSet::detect())
}

pub(crate) fn build_wrapped_transcript_with_styles(
    cells: &[HistoryCell],
    width: usize,
    styles: TuiStyleSet,
) -> WrappedTranscript {
    let mut rows = Vec::new();
    let mut row_anchors = Vec::new();
    let width = width.max(1);
    for cell in cells {
        let first_row = rows.len();
        push_cell_rows(&mut rows, cell, width, styles);
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
            styles.style(TuiSemanticStyle::Muted),
        )));
        row_anchors.push(None);
    }

    WrappedTranscript {
        rows,
        logical_cells: cells.len(),
        row_anchors,
    }
}

fn push_cell_rows(
    rows: &mut Vec<Line<'static>>,
    cell: &HistoryCell,
    width: usize,
    styles: TuiStyleSet,
) {
    match cell.kind() {
        HistoryCellKind::User(content) => {
            push_labeled(rows, "user", TuiSemanticStyle::User, content, width, styles)
        }
        HistoryCellKind::Assistant { content, active } => {
            let label = if *active { "assistant*" } else { "assistant" };
            push_labeled(
                rows,
                label,
                TuiSemanticStyle::Assistant,
                content,
                width,
                styles,
            );
        }
        HistoryCellKind::Tool(entry) => push_labeled(
            rows,
            "tool",
            tool_semantic(entry.phase),
            &entry.display_text(),
            width,
            styles,
        ),
        HistoryCellKind::Event { kind, message } => push_labeled(
            rows,
            kind,
            event_semantic(kind, message),
            message,
            width,
            styles,
        ),
        HistoryCellKind::Debug { id, label, message } => push_labeled(
            rows,
            "debug",
            TuiSemanticStyle::Muted,
            &format!("#{id} {label}: {message}"),
            width,
            styles,
        ),
        HistoryCellKind::Error(message) => push_labeled(
            rows,
            "error",
            TuiSemanticStyle::Error,
            message,
            width,
            styles,
        ),
    }
}

fn push_labeled(
    rows: &mut Vec<Line<'static>>,
    label: &str,
    semantic: TuiSemanticStyle,
    content: &str,
    width: usize,
    styles: TuiStyleSet,
) {
    let label_prefix = format!("[{label}] ");
    let label_style = styles.style(semantic);
    let gutter_style = styles.style(TuiSemanticStyle::Muted);

    let mut line_iter = content.lines();
    let first = line_iter.next().unwrap_or("");
    push_wrapped_text(
        rows,
        StyledPrefix::new(label_prefix.clone(), label_style),
        first,
        width,
        styles,
    );

    for rest in line_iter {
        push_wrapped_text(
            rows,
            StyledPrefix::new(CONTINUATION_GUTTER.to_string(), gutter_style),
            rest,
            width,
            styles,
        );
    }
}

#[derive(Clone, Debug)]
struct StyledPrefix {
    text: String,
    style: TuiStyle,
}

impl StyledPrefix {
    fn new(text: String, style: TuiStyle) -> Self {
        Self { text, style }
    }
}

fn push_wrapped_text(
    rows: &mut Vec<Line<'static>>,
    first_prefix: StyledPrefix,
    text: &str,
    width: usize,
    styles: TuiStyleSet,
) {
    let gutter = StyledPrefix::new(
        CONTINUATION_GUTTER.to_string(),
        styles.style(TuiSemanticStyle::Muted),
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
    width.saturating_sub(TextLayout::measure(prefix)).max(1)
}

fn split_display_width(text: &str, first_width: usize, rest_width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    let policy = wrap_policy_for(text);
    let first = TextLayout::wrap(text, first_width, policy);
    let Some(first_line) = first.first() else {
        return vec![String::new()];
    };
    let mut chunks = vec![first_line.text.clone()];
    let mut rest_start = first_line.source_range.end;
    while rest_start < text.len()
        && text[rest_start..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
    {
        rest_start += text[rest_start..].chars().next().unwrap().len_utf8();
    }
    if rest_start < text.len() {
        chunks.extend(
            TextLayout::wrap(&text[rest_start..], rest_width, policy)
                .into_iter()
                .map(|line| line.text),
        );
    }
    chunks
}

fn wrap_policy_for(text: &str) -> WrapPolicy {
    if text.trim_start().starts_with("```") || text.starts_with("    ") {
        WrapPolicy::CodeBlock
    } else if text.contains("://") {
        WrapPolicy::UrlAware
    } else if text.len() >= 3 && text.as_bytes()[1] == b':' && text.contains('\\') {
        WrapPolicy::WindowsPathAware
    } else {
        WrapPolicy::NaturalText
    }
}

fn event_semantic(label: &str, message: &str) -> TuiSemanticStyle {
    if label == "notice" && message.to_ascii_lowercase().starts_with("warning") {
        return TuiSemanticStyle::Warning;
    }
    match label {
        "shell" | "tool" | "mcp" => TuiSemanticStyle::Tool,
        "approval" | "escalation" => TuiSemanticStyle::ActionRequired,
        "warning" => TuiSemanticStyle::Warning,
        "cancelled" => TuiSemanticStyle::Warning,
        "provider" => TuiSemanticStyle::Error,
        "progress" | "file" | "patch" => TuiSemanticStyle::Progress,
        "context" | "session" | "usage" | "debug" | "details" => TuiSemanticStyle::Muted,
        _ => TuiSemanticStyle::Notice,
    }
}

fn tool_semantic(phase: ToolPhase) -> TuiSemanticStyle {
    match phase {
        ToolPhase::ApprovalRequired => TuiSemanticStyle::ActionRequired,
        ToolPhase::Failed => TuiSemanticStyle::Error,
        ToolPhase::Declined | ToolPhase::Cancelled | ToolPhase::PolicyDeclined => {
            TuiSemanticStyle::Warning
        }
        ToolPhase::Completed => TuiSemanticStyle::Success,
        ToolPhase::Requested | ToolPhase::Approved | ToolPhase::Running => TuiSemanticStyle::Tool,
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

    #[test]
    fn important_states_keep_text_labels_in_monochrome() {
        let cells = vec![
            cell(HistoryCellKind::Event {
                kind: "warning".to_string(),
                message: "check configuration".to_string(),
            }),
            cell(HistoryCellKind::Event {
                kind: "approval".to_string(),
                message: "action required".to_string(),
            }),
            cell(HistoryCellKind::Event {
                kind: "cancelled".to_string(),
                message: "turn stopped".to_string(),
            }),
            cell(HistoryCellKind::Error("provider failed".to_string())),
        ];

        let wrapped = build_wrapped_transcript_with_styles(
            &cells,
            80,
            TuiStyleSet::new(crate::styles::TuiColorCapability::Monochrome),
        );
        let rendered = wrapped
            .rows
            .iter()
            .map(row_text)
            .collect::<Vec<_>>()
            .join("\n");

        for marker in ["[warning]", "[approval]", "[cancelled]", "[error]"] {
            assert!(rendered.contains(marker));
        }
    }
}

use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WrapPolicy {
    NaturalText,
    BreakLongToken,
    UrlAware,
    WindowsPathAware,
    CodeBlock,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VisualLine {
    pub(crate) text: String,
    pub(crate) source_range: Range<usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct VisualCursor {
    pub(crate) row: usize,
    pub(crate) column: usize,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum ClipPriority {
    MustKeep,
    Important,
    Optional,
    DebugOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PrioritySegment<'a> {
    pub(crate) text: &'a str,
    pub(crate) priority: ClipPriority,
}

impl<'a> PrioritySegment<'a> {
    pub(crate) const fn new(text: &'a str, priority: ClipPriority) -> Self {
        Self { text, priority }
    }
}

pub(crate) struct TextLayout;

impl TextLayout {
    pub(crate) fn measure(value: &str) -> usize {
        UnicodeWidthStr::width(value)
    }

    pub(crate) fn wrap(value: &str, width: usize, policy: WrapPolicy) -> Vec<VisualLine> {
        let width = width.max(1);
        let mut lines = Vec::new();
        let mut start = 0usize;

        for (newline, _) in value.match_indices('\n') {
            let mut end = newline;
            if end > start && value.as_bytes()[end - 1] == b'\r' {
                end -= 1;
            }
            wrap_segment(value, start, end, width, policy, &mut lines);
            start = newline + 1;
        }
        wrap_segment(value, start, value.len(), width, policy, &mut lines);

        if lines.is_empty() {
            lines.push(VisualLine {
                text: String::new(),
                source_range: 0..0,
            });
        }
        lines
    }

    pub(crate) fn truncate(value: &str, width: usize) -> String {
        if width == usize::MAX || Self::measure(value) <= width {
            return value.to_string();
        }
        if width == 0 {
            return String::new();
        }

        let ellipsis = "...";
        if width <= Self::measure(ellipsis) {
            return take_width(value, width);
        }
        let mut output = take_width(value, width - Self::measure(ellipsis));
        output.push_str(ellipsis);
        output
    }

    pub(crate) fn cursor_position(
        value: &str,
        byte_index: usize,
        width: usize,
        policy: WrapPolicy,
    ) -> VisualCursor {
        let cursor = clamp_to_grapheme_boundary(value, byte_index);
        let lines = Self::wrap(value, width, policy);

        for (row, line) in lines.iter().enumerate() {
            if cursor <= line.source_range.start {
                return VisualCursor { row, column: 0 };
            }
            if cursor < line.source_range.end
                || (cursor == line.source_range.end
                    && lines
                        .get(row + 1)
                        .is_none_or(|next| next.source_range.start != cursor))
            {
                let end = cursor.min(line.source_range.end);
                return VisualCursor {
                    row,
                    column: Self::measure(&value[line.source_range.start..end]),
                };
            }
        }

        let last = lines.last().expect("TextLayout always returns a line");
        VisualCursor {
            row: lines.len().saturating_sub(1),
            column: Self::measure(&last.text),
        }
    }

    pub(crate) fn priority_line(segments: &[PrioritySegment<'_>], width: usize) -> String {
        if width == 0 {
            return String::new();
        }
        let mut selected = vec![false; segments.len()];

        for priority in [
            ClipPriority::MustKeep,
            ClipPriority::Important,
            ClipPriority::Optional,
            ClipPriority::DebugOnly,
        ] {
            for (index, segment) in segments.iter().enumerate() {
                if segment.priority != priority || segment.text.trim().is_empty() {
                    continue;
                }
                selected[index] = true;
                if Self::measure(&join_selected(segments, &selected)) <= width {
                    continue;
                }
                selected[index] = false;
                if priority == ClipPriority::MustKeep && !selected.iter().any(|value| *value) {
                    return Self::truncate(segment.text, width);
                }
            }
        }

        let output = join_selected(segments, &selected);
        if output.is_empty() {
            segments
                .iter()
                .find(|segment| !segment.text.trim().is_empty())
                .map(|segment| Self::truncate(segment.text, width))
                .unwrap_or_default()
        } else {
            output
        }
    }
}

fn wrap_segment(
    value: &str,
    start: usize,
    end: usize,
    width: usize,
    policy: WrapPolicy,
    lines: &mut Vec<VisualLine>,
) {
    if start >= end {
        lines.push(VisualLine {
            text: String::new(),
            source_range: start..start,
        });
        return;
    }

    let segment = &value[start..end];
    let graphemes = segment
        .grapheme_indices(true)
        .map(|(offset, text)| GraphemeUnit {
            start: start + offset,
            end: start + offset + text.len(),
            text,
            width: TextLayout::measure(text),
        })
        .collect::<Vec<_>>();
    let preserve_whitespace = policy == WrapPolicy::CodeBlock;
    let mut index = 0usize;

    while index < graphemes.len() {
        if !preserve_whitespace {
            while index < graphemes.len() && graphemes[index].text.chars().all(char::is_whitespace)
            {
                index += 1;
            }
            if index >= graphemes.len() {
                break;
            }
        }

        let line_start = index;
        let mut used = 0usize;
        let mut fit_end = index;
        while fit_end < graphemes.len() {
            let next = used.saturating_add(graphemes[fit_end].width);
            if fit_end > line_start && next > width {
                break;
            }
            used = next;
            fit_end += 1;
            if used >= width {
                break;
            }
        }
        fit_end = fit_end.max(line_start + 1).min(graphemes.len());

        let break_end = if fit_end < graphemes.len() {
            preferred_break(&graphemes, line_start, fit_end, policy).unwrap_or(fit_end)
        } else {
            fit_end
        };
        let mut display_end = break_end;
        if !preserve_whitespace {
            while display_end > line_start
                && graphemes[display_end - 1]
                    .text
                    .chars()
                    .all(char::is_whitespace)
            {
                display_end -= 1;
            }
        }
        display_end = display_end.max(line_start + 1);
        let source_start = graphemes[line_start].start;
        let source_end = graphemes[display_end - 1].end;
        lines.push(VisualLine {
            text: value[source_start..source_end].to_string(),
            source_range: source_start..source_end,
        });
        index = break_end.max(line_start + 1);
    }
}

fn preferred_break(
    graphemes: &[GraphemeUnit<'_>],
    start: usize,
    fit_end: usize,
    policy: WrapPolicy,
) -> Option<usize> {
    if policy == WrapPolicy::BreakLongToken || policy == WrapPolicy::CodeBlock {
        return None;
    }
    (start + 1..=fit_end).rev().find(|end| {
        let value = graphemes[*end - 1].text;
        value.chars().all(char::is_whitespace)
            || match policy {
                WrapPolicy::NaturalText => value
                    .chars()
                    .all(|ch| ch.is_ascii_punctuation() || !ch.is_ascii()),
                WrapPolicy::UrlAware => value
                    .chars()
                    .all(|ch| matches!(ch, '/' | '?' | '&' | '=' | '#' | '.' | '-' | '_')),
                WrapPolicy::WindowsPathAware => value
                    .chars()
                    .all(|ch| matches!(ch, '\\' | '/' | ':' | '.' | '-' | '_' | ' ')),
                WrapPolicy::BreakLongToken | WrapPolicy::CodeBlock => false,
            }
    })
}

fn take_width(value: &str, width: usize) -> String {
    let mut output = String::new();
    let mut used = 0usize;
    for grapheme in value.graphemes(true) {
        let next = used.saturating_add(TextLayout::measure(grapheme));
        if next > width {
            break;
        }
        output.push_str(grapheme);
        used = next;
    }
    output
}

fn clamp_to_grapheme_boundary(value: &str, cursor: usize) -> usize {
    let cursor = cursor.min(value.len());
    if cursor == value.len() {
        return cursor;
    }
    value
        .grapheme_indices(true)
        .map(|(index, _)| index)
        .take_while(|index| *index <= cursor)
        .last()
        .unwrap_or(0)
}

fn join_selected(segments: &[PrioritySegment<'_>], selected: &[bool]) -> String {
    segments
        .iter()
        .zip(selected)
        .filter_map(|(segment, selected)| selected.then_some(segment.text))
        .collect::<Vec<_>>()
        .join(" | ")
}

struct GraphemeUnit<'a> {
    start: usize,
    end: usize,
    text: &'a str,
    width: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_international_graphemes_and_maps_cursor() {
        let value = "中文かな👩‍💻e\u{301}";
        assert_eq!(TextLayout::measure(value), 11);
        let cursor =
            TextLayout::cursor_position(value, "中文かな".len(), 6, WrapPolicy::BreakLongToken);
        assert_eq!(cursor, VisualCursor { row: 1, column: 2 });

        for byte in 1.."👩‍💻".len() {
            let inside = TextLayout::cursor_position("👩‍💻x", byte, 10, WrapPolicy::BreakLongToken);
            assert_eq!(inside.column, 0, "byte={byte}");
        }
    }

    #[test]
    fn cursor_maps_soft_wrap_newline_and_end_boundaries() {
        let wrapped = "abcd中文";
        assert_eq!(
            TextLayout::cursor_position(wrapped, 4, 4, WrapPolicy::BreakLongToken),
            VisualCursor { row: 1, column: 0 }
        );
        assert_eq!(
            TextLayout::cursor_position(wrapped, wrapped.len(), 4, WrapPolicy::BreakLongToken),
            VisualCursor { row: 1, column: 4 }
        );

        let multiline = "ab\nかな";
        assert_eq!(
            TextLayout::cursor_position(multiline, 2, 8, WrapPolicy::NaturalText),
            VisualCursor { row: 0, column: 2 }
        );
        assert_eq!(
            TextLayout::cursor_position(multiline, 3, 8, WrapPolicy::NaturalText),
            VisualCursor { row: 1, column: 0 }
        );
    }

    #[test]
    fn every_visual_line_is_a_grapheme_aligned_source_slice() {
        let fixtures = [
            (
                "普通中文かな text 👩‍💻 e\u{301} tail",
                WrapPolicy::NaturalText,
            ),
            (
                "https://example.test/路径?query=かな#結果",
                WrapPolicy::UrlAware,
            ),
            (
                "C:\\YunXi Agent\\输出目录\\長い名前.txt",
                WrapPolicy::WindowsPathAware,
            ),
            (
                "```rust\n    let value = \"中文かな👩‍💻\";\n```",
                WrapPolicy::CodeBlock,
            ),
        ];

        for (value, policy) in fixtures {
            for line in TextLayout::wrap(value, 12, policy) {
                assert!(value.is_char_boundary(line.source_range.start));
                assert!(value.is_char_boundary(line.source_range.end));
                assert_eq!(line.text, value[line.source_range.clone()]);
                assert!(TextLayout::measure(&line.text) <= 12, "{}", line.text);
            }
        }
    }

    #[test]
    fn url_wrap_preserves_query_fragment_and_source_ranges() {
        let value = "https://very-long.example.test/api/v1/items?search=中文&sort=desc#results";
        let lines = TextLayout::wrap(value, 18, WrapPolicy::UrlAware);
        assert!(lines.len() >= 4);
        assert!(
            lines
                .iter()
                .all(|line| TextLayout::measure(&line.text) <= 18)
        );
        assert!(lines.iter().any(|line| line.text.contains('?')));
        assert!(lines.iter().any(|line| line.text.contains('#')));
        for line in lines {
            assert_eq!(line.text, value[line.source_range].to_string());
        }
    }

    #[test]
    fn windows_path_wraps_on_components_without_splitting_graphemes() {
        let value = "C:\\Users\\24763\\YunXi Agent\\输出目录\\very-long-file-name.txt";
        let lines = TextLayout::wrap(value, 16, WrapPolicy::WindowsPathAware);
        assert!(lines.len() >= 3);
        assert!(lines.iter().any(|line| line.text.contains("C:\\")));
        assert!(lines.iter().any(|line| line.text.contains("输出")));
        assert!(
            lines
                .iter()
                .all(|line| TextLayout::measure(&line.text) <= 16)
        );
    }

    #[test]
    fn code_block_preserves_fence_indent_and_cjk_comment() {
        let value = "```rust\n    let path = \"C:\\\\YunXi Agent\\\\very-long\"; // 中文注释\n```";
        let lines = TextLayout::wrap(value, 24, WrapPolicy::CodeBlock);
        assert_eq!(lines.first().unwrap().text, "```rust");
        assert!(lines.iter().any(|line| line.text.starts_with("    let")));
        assert_eq!(lines.last().unwrap().text, "```");
    }

    #[test]
    fn priority_clip_keeps_commands_before_optional_context() {
        let segments = [
            PrioritySegment::new("streaming", ClipPriority::Important),
            PrioritySegment::new("Ctrl+C cancel", ClipPriority::MustKeep),
            PrioritySegment::new("wheel/drag history", ClipPriority::Optional),
            PrioritySegment::new("debug details", ClipPriority::DebugOnly),
        ];
        let line = TextLayout::priority_line(&segments, 32);
        assert!(line.contains("Ctrl+C cancel"));
        assert!(line.contains("streaming"));
        assert!(!line.contains("debug details"));
        assert!(TextLayout::measure(&line) <= 32);
    }
}

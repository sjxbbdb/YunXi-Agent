use unicode_segmentation::UnicodeSegmentation;

#[cfg(test)]
const SHORT_OUTPUT_MAX_CHARS: usize = 240;
#[cfg(test)]
const SHORT_OUTPUT_MAX_LINES: usize = 3;
const DETAIL_DISPLAY_MAX_LINES: usize = 40;
const DETAIL_DISPLAY_MAX_GRAPHEMES: usize = 8 * 1024;
const MAX_DEBUG_INLINE_CHARS: usize = 180;
pub(crate) const MAX_STORED_DETAIL_GRAPHEMES: usize = 32 * 1024;
const TRUNCATED_HEAD_NOTICE: &str = "\n[... content truncated ...]";
const TRUNCATED_TAIL_NOTICE: &str = "[... older content truncated ...]\n";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OutputSummary {
    pub(crate) visible: String,
    pub(crate) detail: String,
    pub(crate) line_count: usize,
    pub(crate) char_count: usize,
    pub(crate) hidden: bool,
}

#[cfg(test)]
pub(crate) fn summarize_tool_output(tool_name: &str, output: &str) -> Option<OutputSummary> {
    let detail = redact_secrets(output.trim());
    if detail.is_empty() {
        return None;
    }

    let line_count = detail.lines().count().max(1);
    let char_count = detail.chars().count();
    let hidden = is_document_like(tool_name, &detail)
        || looks_like_protocol_json(&detail)
        || line_count > SHORT_OUTPUT_MAX_LINES
        || char_count > SHORT_OUTPUT_MAX_CHARS;

    let visible = if hidden {
        format!("output hidden: {line_count} line(s), {char_count} char(s)")
    } else {
        format!("output: {}", detail.replace('\n', " "))
    };

    Some(OutputSummary {
        visible,
        detail,
        line_count,
        char_count,
        hidden,
    })
}

pub(crate) fn detail_display(label: &str, detail: &str) -> String {
    let detail = redact_secrets(detail.trim());
    if detail.is_empty() {
        return format!("{label}: empty");
    }
    let detail = truncate_graphemes_with_notice(&detail, DETAIL_DISPLAY_MAX_GRAPHEMES, true);
    let label = truncate_graphemes_with_notice(label, 256, false);

    let lines = detail.lines().collect::<Vec<_>>();
    if lines.len() <= DETAIL_DISPLAY_MAX_LINES {
        return format!("{label}\n{}", lines.join("\n"));
    }

    let omitted = lines.len().saturating_sub(DETAIL_DISPLAY_MAX_LINES);
    let mut out = Vec::new();
    out.push(label.to_string());
    out.extend(
        lines
            .iter()
            .take(DETAIL_DISPLAY_MAX_LINES.saturating_sub(1))
            .map(|line| (*line).to_string()),
    );
    out.push(format!("... {omitted} line(s) hidden"));
    out.join("\n")
}

pub(crate) fn debug_inline_summary(detail: &str) -> String {
    let detail = redact_secrets(detail.trim());
    if detail.is_empty() {
        return "empty event".to_string();
    }
    let first_line = detail.lines().next().unwrap_or_default().trim();
    truncate_chars(first_line, MAX_DEBUG_INLINE_CHARS)
}

pub(crate) fn redact_secrets(input: &str) -> String {
    let mut output = input.to_string();
    for marker in [
        "github_pat_",
        "ghp_",
        "gho_",
        "ghu_",
        "ghs_",
        "ghr_",
        "sk-",
        "Bearer ",
        "bearer ",
    ] {
        output = redact_marker_tokens(&output, marker);
    }
    normalize_replacement_chars(&output)
}

pub(crate) fn truncate_chars(value: &str, max_chars: usize) -> String {
    truncate_graphemes_with_notice(value, max_chars, false)
}

pub(crate) fn truncate_graphemes_with_notice(
    value: &str,
    max_graphemes: usize,
    retain_tail: bool,
) -> String {
    let graphemes = value.graphemes(true).collect::<Vec<_>>();
    if graphemes.len() <= max_graphemes {
        return value.to_string();
    }

    let notice = if retain_tail {
        TRUNCATED_TAIL_NOTICE
    } else {
        TRUNCATED_HEAD_NOTICE
    };
    let notice_len = notice.graphemes(true).count();
    if notice_len >= max_graphemes {
        return notice.graphemes(true).take(max_graphemes).collect();
    }
    let keep = max_graphemes - notice_len;
    if retain_tail {
        let tail = graphemes[graphemes.len() - keep..].concat();
        format!("{notice}{tail}")
    } else {
        let head = graphemes[..keep].concat();
        format!("{head}{notice}")
    }
}

fn redact_marker_tokens(input: &str, marker: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(index) = rest.find(marker) {
        let (before, after_before) = rest.split_at(index);
        out.push_str(before);
        out.push_str(marker);
        out.push_str("[redacted]");

        let mut end = marker.len();
        for (offset, ch) in after_before[marker.len()..].char_indices() {
            if is_secret_token_char(ch) {
                end = marker.len() + offset + ch.len_utf8();
            } else {
                break;
            }
        }
        rest = &after_before[end..];
    }
    out.push_str(rest);
    out
}

fn is_secret_token_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.')
}

fn normalize_replacement_chars(input: &str) -> String {
    let replacement_count = input.chars().filter(|ch| *ch == '\u{fffd}').count();
    if replacement_count == 0 {
        return input.to_string();
    }

    let cleaned = input
        .chars()
        .filter(|ch| *ch != '\u{fffd}')
        .collect::<String>();
    let notice = format!(
        "[invalid encoding: {replacement_count} replacement character(s) removed; original output may not be UTF-8]"
    );
    if cleaned.trim().is_empty() {
        notice
    } else {
        format!("{}{notice}", cleaned.trim_end())
    }
}

#[cfg(test)]
fn is_document_like(tool_name: &str, detail: &str) -> bool {
    let tool_name = tool_name.to_ascii_lowercase();
    tool_name.contains("skill")
        || detail.contains("<EXTREMELY-IMPORTANT>")
        || detail.contains("## The Rule")
        || detail.contains("name: using-superpowers")
        || detail.contains("description: Use when starting any conversation")
}

#[cfg(test)]
fn looks_like_protocol_json(detail: &str) -> bool {
    let trimmed = detail.trim();
    if trimmed == "{" || trimmed == "}" || trimmed == "[" || trimmed == "]" {
        return true;
    }
    trimmed.contains("\"arguments_json\"")
        || trimmed.contains("\"tool_calls\"")
        || trimmed.contains("\"function\"")
        || trimmed.contains("\\\"arguments_json\\\"")
        || (trimmed.starts_with('{') && trimmed.ends_with('}') && trimmed.len() > 80)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_document_output_is_hidden() {
        let summary = summarize_tool_output(
            "skill: using-superpowers",
            "name: using-superpowers\n<EXTREMELY-IMPORTANT>\nsecret body",
        )
        .expect("summary");

        assert!(summary.hidden);
        assert!(summary.visible.contains("output hidden"));
        assert!(!summary.visible.contains("EXTREMELY-IMPORTANT"));
    }

    #[test]
    fn redacts_common_secret_shapes() {
        let redacted = redact_secrets("token sk-abc123 and github_pat_");

        assert!(redacted.contains("sk-[redacted]"));
        assert!(redacted.contains("github_pat_[redacted]"));
        assert!(!redacted.contains("abc123"));
    }

    #[test]
    fn redaction_normalizes_invalid_encoding_markers() {
        let redacted = redact_secrets("runtime warning: \u{fffd}\u{fffd}\u{fffd}");

        assert!(!redacted.contains('\u{fffd}'));
        assert!(redacted.contains("invalid encoding"));
        assert!(redacted.contains("3 replacement character"));
    }

    #[test]
    fn grapheme_truncation_keeps_emoji_and_combining_sequences_intact() {
        let source = format!("old{}new", "👩‍💻e\u{301}".repeat(100));
        let bounded = truncate_graphemes_with_notice(&source, 64, true);

        assert!(bounded.contains("older content truncated"));
        assert!(bounded.ends_with("new"));
        assert!(bounded.graphemes(true).count() <= 64);
        assert!(!bounded.contains('\u{fffd}'));
    }

    #[test]
    fn detail_display_applies_character_and_line_bounds_after_redaction() {
        let detail = format!("sk-secret-value\n{}", "line\n".repeat(10_000));
        let displayed = detail_display("provider", &detail);

        assert!(!displayed.contains("secret-value"));
        assert!(displayed.contains("truncated") || displayed.contains("hidden"));
        assert!(displayed.lines().count() <= DETAIL_DISPLAY_MAX_LINES + 1);
    }
}

#[cfg(test)]
const SHORT_OUTPUT_MAX_CHARS: usize = 240;
#[cfg(test)]
const SHORT_OUTPUT_MAX_LINES: usize = 3;
const DETAIL_DISPLAY_MAX_LINES: usize = 40;
const MAX_DEBUG_INLINE_CHARS: usize = 180;

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
    output
}

pub(crate) fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
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
}

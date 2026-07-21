use unicode_segmentation::UnicodeSegmentation;

pub(crate) const MAX_STREAM_LIVE_TAIL_BYTES: usize = 64 * 1024;
pub(crate) const MAX_STREAM_CONTENT_BYTES: usize = 256 * 1024;
const STREAM_TRUNCATION_NOTICE: &str = "[stream truncated: older content omitted]\n";

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MarkdownStreamCollector {
    buffer: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MarkdownStreamController {
    collector: MarkdownStreamCollector,
    stable_source: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkdownStreamFrame {
    pub stable_source: String,
    pub live_tail: String,
    pub committed: bool,
}

impl MarkdownStreamController {
    pub fn push_delta(&mut self, delta: &str) -> MarkdownStreamFrame {
        self.collector.push_delta(delta);
        let committed = self
            .collector
            .commit_complete_source()
            .map(|source| {
                self.stable_source.push_str(&source);
                bound_stream_tail(&mut self.stable_source, MAX_STREAM_CONTENT_BYTES);
            })
            .is_some();
        self.frame(committed)
    }

    pub fn finalize(&mut self) -> Option<String> {
        let tail = self.collector.finalize_and_drain_source();
        if tail.is_empty() && self.stable_source.is_empty() {
            return None;
        }
        self.stable_source.push_str(&tail);
        bound_stream_tail(&mut self.stable_source, MAX_STREAM_CONTENT_BYTES);
        Some(std::mem::take(&mut self.stable_source))
    }

    pub fn clear(&mut self) {
        self.collector.clear();
        self.stable_source.clear();
    }

    fn frame(&self, committed: bool) -> MarkdownStreamFrame {
        MarkdownStreamFrame {
            stable_source: self.stable_source.clone(),
            live_tail: self.collector.live_tail().to_string(),
            committed,
        }
    }
}

impl MarkdownStreamCollector {
    pub fn push_delta(&mut self, delta: &str) {
        self.buffer.push_str(delta);
    }

    pub fn commit_complete_source(&mut self) -> Option<String> {
        let commit_offset = match markdown_boundary(&self.buffer) {
            MarkdownBoundary::Commit(offset) => Some(offset),
            MarkdownBoundary::OpenFence => None,
            MarkdownBoundary::None => safe_grapheme_commit_end(&self.buffer),
        };
        let Some(commit_offset) = commit_offset else {
            bound_stream_tail(&mut self.buffer, MAX_STREAM_LIVE_TAIL_BYTES);
            return None;
        };
        if commit_offset == 0 {
            bound_stream_tail(&mut self.buffer, MAX_STREAM_LIVE_TAIL_BYTES);
            return None;
        }
        let out = self.buffer[..commit_offset].to_string();
        self.buffer.drain(..commit_offset);
        bound_stream_tail(&mut self.buffer, MAX_STREAM_LIVE_TAIL_BYTES);
        Some(out)
    }

    pub fn live_tail(&self) -> &str {
        &self.buffer
    }

    pub fn finalize_and_drain_source(&mut self) -> String {
        std::mem::take(&mut self.buffer)
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

fn bound_stream_tail(value: &mut String, max_bytes: usize) -> bool {
    if value.len() <= max_bytes {
        return false;
    }

    let source = value
        .strip_prefix(STREAM_TRUNCATION_NOTICE)
        .unwrap_or(value.as_str());
    let available = max_bytes.saturating_sub(STREAM_TRUNCATION_NOTICE.len());
    let mut start = source.len();
    for (index, _) in source.grapheme_indices(true).rev() {
        if source.len().saturating_sub(index) > available {
            break;
        }
        start = index;
    }
    *value = format!("{STREAM_TRUNCATION_NOTICE}{}", &source[start..]);
    true
}

pub(crate) fn bounded_stream_content(stable_source: &str, live_tail: &str) -> String {
    let mut content = String::with_capacity(stable_source.len().saturating_add(live_tail.len()));
    content.push_str(stable_source);
    content.push_str(live_tail);
    bound_stream_tail(&mut content, MAX_STREAM_CONTENT_BYTES);
    content
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MarkdownBoundary {
    Commit(usize),
    OpenFence,
    None,
}

fn markdown_boundary(source: &str) -> MarkdownBoundary {
    let mut offset = 0usize;
    let mut last_safe = 0usize;
    let mut fence_marker = None;

    for line in source.split_inclusive('\n') {
        offset = offset.saturating_add(line.len());
        let trimmed = line.trim_start();
        let marker = fence_line_marker(trimmed);
        match (fence_marker, marker) {
            (None, Some(marker)) => fence_marker = Some(marker),
            (Some(open), Some(close)) if open == close => {
                fence_marker = None;
                last_safe = offset;
            }
            (None, None) if line.ends_with('\n') => last_safe = offset,
            _ => {}
        }
    }

    if last_safe > 0 {
        MarkdownBoundary::Commit(last_safe)
    } else if fence_marker.is_some() {
        MarkdownBoundary::OpenFence
    } else {
        MarkdownBoundary::None
    }
}

fn fence_line_marker(line: &str) -> Option<char> {
    let marker = line.chars().next()?;
    if !matches!(marker, '`' | '~') {
        return None;
    }
    (line
        .chars()
        .take_while(|character| *character == marker)
        .count()
        >= 3)
        .then_some(marker)
}

fn safe_grapheme_commit_end(source: &str) -> Option<usize> {
    let mut starts = source
        .grapheme_indices(true)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if starts.len() < 2 {
        return None;
    }
    let mut commit_end = starts.pop().unwrap_or_default();
    if let Some(marker_start) = trailing_partial_fence_start(source) {
        commit_end = commit_end.min(marker_start);
    }
    (commit_end > 0).then_some(commit_end)
}

fn trailing_partial_fence_start(source: &str) -> Option<usize> {
    let (last_index, marker) = source.char_indices().next_back()?;
    if !matches!(marker, '`' | '~') {
        return None;
    }
    let mut start = last_index;
    let mut count = 1usize;
    for (index, character) in source[..last_index].char_indices().rev() {
        if character != marker {
            break;
        }
        start = index;
        count = count.saturating_add(1);
    }
    (count < 3).then_some(start)
}

#[cfg(test)]
pub(crate) fn append_fragment(target: &mut String, fragment: &str) {
    let fragment = fragment.trim_matches('\r');
    if fragment.is_empty() {
        return;
    }
    if target.is_empty() {
        target.push_str(fragment.trim_start());
        return;
    }
    let trimmed = fragment.trim_start();
    if trimmed.is_empty() {
        return;
    }
    let left = target.chars().next_back();
    let right = trimmed.chars().next();
    if let (Some(left), Some(right)) = (left, right)
        && should_insert_space(left, right)
    {
        target.push(' ');
    }
    target.push_str(trimmed);
}

#[cfg(test)]
fn should_insert_space(left: char, right: char) -> bool {
    if left.is_whitespace() || right.is_whitespace() {
        return false;
    }
    if is_cjk(left) || is_cjk(right) {
        return false;
    }
    if matches!(
        right,
        ',' | '.' | ';' | ':' | '!' | '?' | ')' | ']' | '}' | '"' | '\''
    ) {
        return false;
    }
    if matches!(left, '(' | '[' | '{' | '"' | '\'' | '/' | '\\') {
        return false;
    }
    left.is_alphanumeric() && right.is_alphanumeric()
}

#[cfg(test)]
fn is_cjk(ch: char) -> bool {
    matches!(
        ch as u32,
        0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xF900..=0xFAFF
            | 0x20000..=0x2A6DF
            | 0x2A700..=0x2B73F
            | 0x2B740..=0x2B81F
            | 0x2B820..=0x2CEAF
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collector_commits_complete_lines_and_preserves_exact_tail() {
        let mut stream = MarkdownStreamCollector::default();
        stream.push_delta("hello");
        assert_eq!(stream.commit_complete_source(), Some("hell".to_string()));
        assert_eq!(stream.live_tail(), "o");
        stream.push_delta("\nworld");
        assert_eq!(stream.commit_complete_source(), Some("o\n".to_string()));
        assert_eq!(stream.live_tail(), "world");
        assert_eq!(stream.finalize_and_drain_source(), "world");
    }

    #[test]
    fn fragment_merge_keeps_cjk_together_and_spaces_english() {
        let mut merged = String::new();
        for delta in ["用户", "输入", "了", "\"", "测试", "\""] {
            append_fragment(&mut merged, delta);
        }
        assert_eq!(merged, "用户输入了\"测试\"");

        let mut english = String::new();
        for delta in ["Provider", "turn", "started"] {
            append_fragment(&mut english, delta);
        }
        assert_eq!(english, "Provider turn started");
    }

    #[test]
    fn controller_keeps_stable_source_separate_from_live_tail() {
        let mut controller = MarkdownStreamController::default();

        let first = controller.push_delta("hello");
        assert_eq!(
            first,
            MarkdownStreamFrame {
                stable_source: "hell".to_string(),
                live_tail: "o".to_string(),
                committed: true,
            }
        );

        let second = controller.push_delta("\nworld");
        assert_eq!(
            second,
            MarkdownStreamFrame {
                stable_source: "hello\n".to_string(),
                live_tail: "world".to_string(),
                committed: true,
            }
        );
        assert_eq!(controller.finalize(), Some("hello\nworld".to_string()));
    }

    #[test]
    fn finalize_clears_live_tail_before_the_next_message() {
        let mut controller = MarkdownStreamController::default();
        controller.push_delta("first response");

        assert_eq!(controller.finalize(), Some("first response".to_string()));
        assert_eq!(
            controller.push_delta("next response"),
            MarkdownStreamFrame {
                stable_source: "next respons".to_string(),
                live_tail: "e".to_string(),
                committed: true,
            }
        );
    }

    #[test]
    fn committed_source_never_remains_in_the_live_tail() {
        let mut controller = MarkdownStreamController::default();

        let frame = controller.push_delta("stable line\nlive tail");

        assert_eq!(frame.stable_source, "stable line\n");
        assert_eq!(frame.live_tail, "live tail");
        assert!(frame.committed);
        assert!(!frame.live_tail.contains("stable line"));
    }

    #[test]
    fn repeated_delta_payload_is_preserved_until_message_ids_enable_deduplication() {
        let mut controller = MarkdownStreamController::default();

        let frame = controller.push_delta("ha");
        assert_eq!(format!("{}{}", frame.stable_source, frame.live_tail), "ha");
        let frame = controller.push_delta("ha");

        assert_eq!(
            format!("{}{}", frame.stable_source, frame.live_tail),
            "haha"
        );
        assert_eq!(controller.finalize(), Some("haha".to_string()));
    }

    #[test]
    fn paragraph_boundary_commits_before_the_next_paragraph() {
        let mut controller = MarkdownStreamController::default();
        let frame = controller.push_delta("first paragraph\n\nnext");

        assert_eq!(frame.stable_source, "first paragraph\n\n");
        assert_eq!(frame.live_tail, "next");
        assert!(frame.committed);
    }

    #[test]
    fn markdown_fence_commits_only_after_the_closing_fence() {
        let mut controller = MarkdownStreamController::default();
        let open = controller.push_delta("```rust\nfn ");
        assert_eq!(open.stable_source, "");
        assert_eq!(open.live_tail, "```rust\nfn ");
        assert!(!open.committed);

        let closed = controller.push_delta("main() {}\n```");
        assert_eq!(closed.stable_source, "```rust\nfn main() {}\n```");
        assert_eq!(closed.live_tail, "");
        assert!(closed.committed);
    }

    #[test]
    fn safe_grapheme_boundary_holds_emoji_zwj_and_combining_tail() {
        let mut emoji = MarkdownStreamController::default();
        let partial = emoji.push_delta("👩‍");
        assert_eq!(partial.stable_source, "");
        assert_eq!(partial.live_tail, "👩‍");
        let joined = emoji.push_delta("💻x");
        assert_eq!(joined.stable_source, "👩‍💻");
        assert_eq!(joined.live_tail, "x");

        let mut combining = MarkdownStreamController::default();
        assert_eq!(combining.push_delta("e").live_tail, "e");
        let joined = combining.push_delta("\u{301}x");
        assert_eq!(joined.stable_source, "e\u{301}");
        assert_eq!(joined.live_tail, "x");
    }

    #[test]
    fn safe_grapheme_boundary_keeps_cjk_kana_and_partial_fence_exact() {
        let mut controller = MarkdownStreamController::default();
        let frame = controller.push_delta("中文かなカナx");
        assert_eq!(frame.stable_source, "中文かなカナ");
        assert_eq!(frame.live_tail, "x");

        let mut fence = MarkdownStreamController::default();
        let frame = fence.push_delta("``");
        assert_eq!(frame.stable_source, "");
        assert_eq!(frame.live_tail, "``");
    }

    #[test]
    fn open_code_fence_live_tail_is_bounded_and_finalization_recovers() {
        let mut controller = MarkdownStreamController::default();
        let payload = format!(
            "```text\n{}",
            "👩‍💻e\u{301}".repeat(MAX_STREAM_LIVE_TAIL_BYTES)
        );
        let frame = controller.push_delta(&payload);

        assert!(frame.live_tail.len() <= MAX_STREAM_LIVE_TAIL_BYTES);
        assert!(
            format!("{}{}", frame.stable_source, frame.live_tail)
                .starts_with(STREAM_TRUNCATION_NOTICE)
        );
        let finalized = controller.finalize().expect("bounded partial response");
        assert!(finalized.len() <= MAX_STREAM_CONTENT_BYTES);
        assert!(!finalized.contains('\u{fffd}'));
        assert_eq!(controller.push_delta("next").live_tail, "t");
    }

    #[test]
    fn long_committed_stream_keeps_latest_content_with_one_visible_notice() {
        let mut controller = MarkdownStreamController::default();
        let mut payload = String::new();
        for index in 0..20_000 {
            payload.push_str(&format!("line {index} 中文 👩‍💻\n"));
        }
        controller.push_delta(&payload);
        let output = controller.finalize().expect("stream output");

        assert!(output.len() <= MAX_STREAM_CONTENT_BYTES);
        assert!(output.starts_with(STREAM_TRUNCATION_NOTICE));
        assert_eq!(output.matches(STREAM_TRUNCATION_NOTICE).count(), 1);
        assert!(output.contains("line 19999"));
        assert!(!output.contains('\u{fffd}'));
    }
}

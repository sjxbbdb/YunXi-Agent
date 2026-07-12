#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MarkdownStreamCollector {
    buffer: String,
    committed_source_len: usize,
}

impl MarkdownStreamCollector {
    pub fn push_delta(&mut self, delta: &str) {
        self.buffer.push_str(delta);
    }

    pub fn commit_complete_source(&mut self) -> Option<String> {
        let commit_end = self.buffer.rfind('\n').map(|idx| idx + 1)?;
        if commit_end <= self.committed_source_len {
            return None;
        }
        let out = self.buffer[self.committed_source_len..commit_end].to_string();
        self.committed_source_len = commit_end;
        Some(out)
    }

    pub fn live_tail(&self) -> &str {
        &self.buffer[self.committed_source_len..]
    }

    pub fn finalize_and_drain_source(&mut self) -> String {
        if self.committed_source_len >= self.buffer.len() {
            self.clear();
            return String::new();
        }
        let mut out = self.buffer[self.committed_source_len..].to_string();
        if !out.ends_with('\n') {
            out.push('\n');
        }
        self.clear();
        out
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.committed_source_len = 0;
    }
}

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
    fn collector_commits_only_newline_boundaries() {
        let mut stream = MarkdownStreamCollector::default();
        stream.push_delta("hello");
        assert_eq!(stream.commit_complete_source(), None);
        stream.push_delta("\nworld");
        assert_eq!(stream.commit_complete_source(), Some("hello\n".to_string()));
        assert_eq!(stream.live_tail(), "world");
        assert_eq!(stream.finalize_and_drain_source(), "world\n");
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
}

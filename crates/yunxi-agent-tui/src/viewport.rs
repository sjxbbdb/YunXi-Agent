#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TranscriptViewport {
    scroll_offset_from_bottom: usize,
    new_content_below: bool,
}

impl Default for TranscriptViewport {
    fn default() -> Self {
        Self {
            scroll_offset_from_bottom: 0,
            new_content_below: false,
        }
    }
}

impl TranscriptViewport {
    pub(crate) fn is_following_tail(&self) -> bool {
        self.scroll_offset_from_bottom == 0
    }

    pub(crate) fn reset(&mut self) {
        self.scroll_offset_from_bottom = 0;
        self.new_content_below = false;
    }

    pub(crate) fn on_content_changed(&mut self) {
        if self.is_following_tail() {
            self.new_content_below = false;
        } else {
            self.new_content_below = true;
        }
    }

    pub(crate) fn scroll_up(
        &mut self,
        lines: usize,
        content_height: usize,
        viewport_height: usize,
    ) {
        let max_offset = max_scroll_offset(content_height, viewport_height);
        self.scroll_offset_from_bottom = self
            .scroll_offset_from_bottom
            .saturating_add(lines)
            .min(max_offset);
    }

    pub(crate) fn scroll_down(&mut self, lines: usize) {
        self.scroll_offset_from_bottom = self.scroll_offset_from_bottom.saturating_sub(lines);
        if self.is_following_tail() {
            self.new_content_below = false;
        }
    }

    pub(crate) fn page_up(&mut self, content_height: usize, viewport_height: usize) {
        self.scroll_up(viewport_height.max(1), content_height, viewport_height);
    }

    pub(crate) fn page_down(&mut self, viewport_height: usize) {
        self.scroll_down(viewport_height.max(1));
    }

    pub(crate) fn jump_top(&mut self, content_height: usize, viewport_height: usize) {
        self.scroll_offset_from_bottom = max_scroll_offset(content_height, viewport_height);
    }

    pub(crate) fn follow_tail(&mut self) {
        self.scroll_offset_from_bottom = 0;
        self.new_content_below = false;
    }

    pub(crate) fn view_start(&self, content_height: usize, viewport_height: usize) -> usize {
        let viewport_height = viewport_height.max(1);
        if content_height <= viewport_height {
            return 0;
        }
        let max_start = content_height - viewport_height;
        let offset = self.scroll_offset_from_bottom.min(max_start);
        max_start - offset
    }

    pub(crate) fn scroll_status(&self) -> &'static str {
        if self.new_content_below {
            "new output below"
        } else if self.is_following_tail() {
            "tail"
        } else {
            "history"
        }
    }
}

fn max_scroll_offset(content_height: usize, viewport_height: usize) -> usize {
    content_height.saturating_sub(viewport_height.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_tail_until_user_scrolls_history() {
        let mut viewport = TranscriptViewport::default();
        viewport.on_content_changed();
        assert!(viewport.is_following_tail());
        assert_eq!(viewport.scroll_status(), "tail");

        viewport.scroll_up(3, 20, 5);
        assert_eq!(viewport.scroll_offset_from_bottom, 3);
        viewport.on_content_changed();
        assert_eq!(viewport.scroll_status(), "new output below");

        viewport.follow_tail();
        assert_eq!(viewport.scroll_offset_from_bottom, 0);
        assert_eq!(viewport.scroll_status(), "tail");
    }

    #[test]
    fn calculates_start_from_bottom_offset() {
        let mut viewport = TranscriptViewport::default();
        assert_eq!(viewport.view_start(30, 10), 20);

        viewport.scroll_up(4, 30, 10);
        assert_eq!(viewport.view_start(30, 10), 16);

        viewport.jump_top(30, 10);
        assert_eq!(viewport.view_start(30, 10), 0);
    }

    #[test]
    fn clamps_scroll_when_content_is_short() {
        let mut viewport = TranscriptViewport::default();
        viewport.scroll_up(10, 3, 10);
        assert_eq!(viewport.scroll_offset_from_bottom, 0);
        assert_eq!(viewport.view_start(3, 10), 0);
    }
}

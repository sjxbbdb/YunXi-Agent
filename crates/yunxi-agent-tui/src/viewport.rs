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
        let max_offset = max_start(content_height, viewport_height);
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
        self.scroll_offset_from_bottom = max_start(content_height, viewport_height);
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
        let max_start = max_start(content_height, viewport_height);
        let offset = self.scroll_offset_from_bottom.min(max_start);
        max_start.saturating_sub(offset)
    }

    pub(crate) fn set_view_start(
        &mut self,
        start: usize,
        content_height: usize,
        viewport_height: usize,
    ) {
        let max_start = max_start(content_height, viewport_height);
        let start = start.min(max_start);
        self.scroll_offset_from_bottom = max_start.saturating_sub(start);
        if self.is_following_tail() {
            self.new_content_below = false;
        }
    }

    pub(crate) fn set_scroll_fraction(
        &mut self,
        numerator: usize,
        denominator: usize,
        content_height: usize,
        viewport_height: usize,
    ) {
        let max_start = max_start(content_height, viewport_height);
        let start = if denominator == 0 {
            0
        } else {
            ((numerator.min(denominator) * max_start) + (denominator / 2)) / denominator
        };
        self.set_view_start(start, content_height, viewport_height);
    }

    pub(crate) fn clamp(&mut self, content_height: usize, viewport_height: usize) {
        self.scroll_offset_from_bottom = self
            .scroll_offset_from_bottom
            .min(max_start(content_height, viewport_height));
        if self.is_following_tail() {
            self.new_content_below = false;
        }
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

pub(crate) fn max_start(content_height: usize, viewport_height: usize) -> usize {
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

    #[test]
    fn set_view_start_clamps_and_updates_tail() {
        let mut viewport = TranscriptViewport::default();

        viewport.set_view_start(5, 30, 10);
        assert_eq!(viewport.view_start(30, 10), 5);
        assert_eq!(viewport.scroll_status(), "history");

        viewport.on_content_changed();
        assert_eq!(viewport.scroll_status(), "new output below");

        viewport.set_view_start(99, 30, 10);
        assert_eq!(viewport.view_start(30, 10), 20);
        assert_eq!(viewport.scroll_status(), "tail");
    }

    #[test]
    fn set_scroll_fraction_maps_top_middle_and_bottom() {
        let mut viewport = TranscriptViewport::default();

        viewport.set_scroll_fraction(0, 10, 110, 10);
        assert_eq!(viewport.view_start(110, 10), 0);

        viewport.set_scroll_fraction(5, 10, 110, 10);
        assert_eq!(viewport.view_start(110, 10), 50);

        viewport.set_scroll_fraction(10, 10, 110, 10);
        assert_eq!(viewport.view_start(110, 10), 100);
        assert_eq!(viewport.scroll_status(), "tail");
    }
}

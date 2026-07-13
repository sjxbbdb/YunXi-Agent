use crate::layout::rect_contains;
use crate::viewport::max_start;
use ratatui::layout::Rect;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TranscriptScrollbarGeometry {
    pub(crate) track: Rect,
    pub(crate) thumb_top: u16,
    pub(crate) thumb_height: u16,
    pub(crate) content_height: usize,
    pub(crate) visible_height: usize,
    pub(crate) start: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ScrollbarHit {
    Thumb { grab_offset: u16 },
    PageUp,
    PageDown,
    Outside,
}

impl TranscriptScrollbarGeometry {
    pub(crate) fn new(
        track: Rect,
        content_height: usize,
        visible_height: usize,
        start: usize,
    ) -> Option<Self> {
        let visible_height = visible_height.max(1);
        if track.height == 0 || track.width == 0 || content_height <= visible_height {
            return None;
        }
        let track_height = track.height as usize;
        let thumb_height = ((visible_height * track_height).div_ceil(content_height))
            .max(1)
            .min(track_height) as u16;
        let max_start = max_start(content_height, visible_height);
        let travel = track.height.saturating_sub(thumb_height) as usize;
        let start = start.min(max_start);
        let thumb_offset = if max_start == 0 || travel == 0 {
            0
        } else {
            ((start * travel) + (max_start / 2)) / max_start
        } as u16;

        Some(Self {
            track,
            thumb_top: track.y.saturating_add(thumb_offset),
            thumb_height,
            content_height,
            visible_height,
            start,
        })
    }

    pub(crate) fn hit_test(&self, x: u16, y: u16) -> ScrollbarHit {
        if !rect_contains(self.track, x, y) {
            return ScrollbarHit::Outside;
        }
        let thumb_bottom = self.thumb_top.saturating_add(self.thumb_height);
        if y >= self.thumb_top && y < thumb_bottom {
            return ScrollbarHit::Thumb {
                grab_offset: y.saturating_sub(self.thumb_top),
            };
        }
        if y < self.thumb_top {
            ScrollbarHit::PageUp
        } else {
            ScrollbarHit::PageDown
        }
    }

    pub(crate) fn start_for_drag_y(&self, y: u16, grab_offset: u16) -> usize {
        let max_start = max_start(self.content_height, self.visible_height);
        let travel = self.track.height.saturating_sub(self.thumb_height);
        if max_start == 0 || travel == 0 {
            return 0;
        }

        let raw_top = y.saturating_sub(grab_offset);
        let min_top = self.track.y;
        let max_top = self.track.y.saturating_add(travel);
        let top = raw_top.clamp(min_top, max_top);
        let numerator = top.saturating_sub(self.track.y) as usize;
        ((numerator * max_start) + (travel as usize / 2)) / travel as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrollbar_thumb_reaches_bottom_at_max_start() {
        let track = Rect::new(10, 2, 1, 10);
        let top = TranscriptScrollbarGeometry::new(track, 100, 10, 0).expect("scrollbar");
        let bottom = TranscriptScrollbarGeometry::new(track, 100, 10, 90).expect("scrollbar");

        assert_eq!(top.thumb_top, track.y);
        assert_eq!(
            bottom.thumb_top.saturating_add(bottom.thumb_height),
            track.y.saturating_add(track.height)
        );
    }

    #[test]
    fn scrollbar_drag_maps_top_middle_bottom() {
        let track = Rect::new(10, 2, 1, 10);
        let geometry = TranscriptScrollbarGeometry::new(track, 100, 10, 45).expect("scrollbar");
        let bottom_y = track.y.saturating_add(track.height.saturating_sub(1));

        assert_eq!(geometry.start_for_drag_y(track.y, 0), 0);
        assert!(geometry.start_for_drag_y(track.y + 5, 0) > 0);
        assert_eq!(geometry.start_for_drag_y(bottom_y, 0), 90);
    }

    #[test]
    fn scrollbar_hit_tests_thumb_and_track() {
        let track = Rect::new(10, 2, 1, 10);
        let geometry = TranscriptScrollbarGeometry::new(track, 100, 10, 45).expect("scrollbar");

        assert!(matches!(
            geometry.hit_test(10, geometry.thumb_top),
            ScrollbarHit::Thumb { .. }
        ));
        assert_eq!(
            geometry.hit_test(9, geometry.thumb_top),
            ScrollbarHit::Outside
        );
        assert_eq!(geometry.hit_test(10, track.y), ScrollbarHit::PageUp);
        assert_eq!(
            geometry.hit_test(10, track.y + track.height - 1),
            ScrollbarHit::PageDown
        );
    }
}

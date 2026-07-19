use crate::presentation::TuiCellId;
use crate::transcript_layout::WrappedTranscript;

#[derive(Clone, Debug, Eq, PartialEq)]
enum ViewportAnchor {
    FollowTail,
    Pinned {
        cell_id: TuiCellId,
        line_offset: usize,
        fallback_start: usize,
    },
    NewOutputBelow {
        cell_id: TuiCellId,
        line_offset: usize,
        fallback_start: usize,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TranscriptViewport {
    anchor: ViewportAnchor,
}

impl Default for TranscriptViewport {
    fn default() -> Self {
        Self {
            anchor: ViewportAnchor::FollowTail,
        }
    }
}

impl TranscriptViewport {
    pub(crate) fn is_following_tail(&self) -> bool {
        matches!(self.anchor, ViewportAnchor::FollowTail)
    }

    pub(crate) fn reset(&mut self) {
        self.anchor = ViewportAnchor::FollowTail;
    }

    pub(crate) fn on_content_changed(&mut self) {
        if let ViewportAnchor::Pinned {
            cell_id,
            line_offset,
            fallback_start,
        } = &self.anchor
        {
            self.anchor = ViewportAnchor::NewOutputBelow {
                cell_id: cell_id.clone(),
                line_offset: *line_offset,
                fallback_start: *fallback_start,
            };
        }
    }

    pub(crate) fn scroll_up(
        &mut self,
        lines: usize,
        wrapped: &WrappedTranscript,
        viewport_height: usize,
    ) {
        let start = self.view_start(wrapped, viewport_height);
        self.set_view_start(start.saturating_sub(lines), wrapped, viewport_height);
    }

    pub(crate) fn scroll_down(
        &mut self,
        lines: usize,
        wrapped: &WrappedTranscript,
        viewport_height: usize,
    ) {
        let max_start = max_start(wrapped.rows.len(), viewport_height);
        let start = self
            .view_start(wrapped, viewport_height)
            .saturating_add(lines)
            .min(max_start);
        self.set_view_start(start, wrapped, viewport_height);
    }

    pub(crate) fn page_up(&mut self, wrapped: &WrappedTranscript, viewport_height: usize) {
        self.scroll_up(viewport_height.max(1), wrapped, viewport_height);
    }

    pub(crate) fn page_down(&mut self, wrapped: &WrappedTranscript, viewport_height: usize) {
        self.scroll_down(viewport_height.max(1), wrapped, viewport_height);
    }

    pub(crate) fn jump_top(&mut self, wrapped: &WrappedTranscript, viewport_height: usize) {
        self.set_view_start(0, wrapped, viewport_height);
    }

    pub(crate) fn follow_tail(&mut self) {
        self.anchor = ViewportAnchor::FollowTail;
    }

    pub(crate) fn view_start(&self, wrapped: &WrappedTranscript, viewport_height: usize) -> usize {
        let max_start = max_start(wrapped.rows.len(), viewport_height);
        match &self.anchor {
            ViewportAnchor::FollowTail => max_start,
            ViewportAnchor::Pinned {
                cell_id,
                line_offset,
                fallback_start,
            }
            | ViewportAnchor::NewOutputBelow {
                cell_id,
                line_offset,
                fallback_start,
            } => wrapped
                .resolve_anchor(cell_id, *line_offset)
                .unwrap_or(*fallback_start)
                .min(max_start),
        }
    }

    pub(crate) fn set_view_start(
        &mut self,
        start: usize,
        wrapped: &WrappedTranscript,
        viewport_height: usize,
    ) {
        let max_start = max_start(wrapped.rows.len(), viewport_height);
        let start = start.min(max_start);
        if start == max_start {
            self.follow_tail();
            return;
        }
        if let Some(anchor) = wrapped.anchor_at(start) {
            self.anchor = ViewportAnchor::Pinned {
                cell_id: anchor.cell_id.clone(),
                line_offset: anchor.line_offset,
                fallback_start: start,
            };
        }
    }

    pub(crate) fn set_scroll_fraction(
        &mut self,
        numerator: usize,
        denominator: usize,
        wrapped: &WrappedTranscript,
        viewport_height: usize,
    ) {
        let max_start = max_start(wrapped.rows.len(), viewport_height);
        let start = if denominator == 0 {
            0
        } else {
            ((numerator.min(denominator) * max_start) + (denominator / 2)) / denominator
        };
        self.set_view_start(start, wrapped, viewport_height);
    }

    pub(crate) fn reanchor(&mut self, wrapped: &WrappedTranscript, viewport_height: usize) {
        if self.is_following_tail() {
            return;
        }
        let start = self.view_start(wrapped, viewport_height);
        let was_new_output = matches!(self.anchor, ViewportAnchor::NewOutputBelow { .. });
        if let Some(anchor) = wrapped.anchor_at(start) {
            self.anchor = if was_new_output {
                ViewportAnchor::NewOutputBelow {
                    cell_id: anchor.cell_id.clone(),
                    line_offset: anchor.line_offset,
                    fallback_start: start,
                }
            } else {
                ViewportAnchor::Pinned {
                    cell_id: anchor.cell_id.clone(),
                    line_offset: anchor.line_offset,
                    fallback_start: start,
                }
            };
        }
    }

    pub(crate) fn scroll_status(&self) -> &'static str {
        match self.anchor {
            ViewportAnchor::FollowTail => "tail",
            ViewportAnchor::Pinned { .. } => "history",
            ViewportAnchor::NewOutputBelow { .. } => "new output below",
        }
    }
}

pub(crate) fn max_start(content_height: usize, viewport_height: usize) -> usize {
    content_height.saturating_sub(viewport_height.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::{HistoryCell, HistoryCellKind};
    use crate::presentation::TuiCellId;
    use crate::transcript_layout::build_wrapped_transcript;

    fn cells(count: usize) -> Vec<HistoryCell> {
        (0..count)
            .map(|index| HistoryCell {
                id: TuiCellId::from_test(&format!("cell-{index}")),
                kind: HistoryCellKind::User(format!(
                    "line {index} with enough content to wrap on narrow terminals"
                )),
                detail_id: None,
            })
            .collect()
    }

    #[test]
    fn pinned_cell_survives_append_and_reports_new_output() {
        let mut source = cells(8);
        let wrapped = build_wrapped_transcript(&source, 24);
        let mut viewport = TranscriptViewport::default();
        viewport.set_view_start(3, &wrapped, 5);
        let before = wrapped.anchor_at(viewport.view_start(&wrapped, 5)).cloned();

        source.extend(cells(2).into_iter().map(|mut cell| {
            cell.id = cell.id.with_suffix("appended");
            cell
        }));
        let appended = build_wrapped_transcript(&source, 24);
        viewport.on_content_changed();
        let after = appended
            .anchor_at(viewport.view_start(&appended, 5))
            .cloned();

        assert_eq!(after, before);
        assert_eq!(viewport.scroll_status(), "new output below");
    }

    #[test]
    fn pinned_cell_survives_narrow_and_wide_resize() {
        let source = cells(12);
        let narrow = build_wrapped_transcript(&source, 20);
        let mut viewport = TranscriptViewport::default();
        viewport.set_view_start(8, &narrow, 6);
        let expected_cell = narrow
            .anchor_at(viewport.view_start(&narrow, 6))
            .unwrap()
            .cell_id
            .clone();

        let wide = build_wrapped_transcript(&source, 70);
        viewport.reanchor(&wide, 6);
        let actual_cell = wide
            .anchor_at(viewport.view_start(&wide, 6))
            .unwrap()
            .cell_id
            .clone();

        assert_eq!(actual_cell, expected_cell);
        assert_eq!(viewport.scroll_status(), "history");
    }

    #[test]
    fn end_returns_to_follow_tail() {
        let wrapped = build_wrapped_transcript(&cells(10), 24);
        let mut viewport = TranscriptViewport::default();
        viewport.jump_top(&wrapped, 5);
        assert_eq!(viewport.scroll_status(), "history");

        viewport.follow_tail();

        assert!(viewport.is_following_tail());
        assert_eq!(
            viewport.view_start(&wrapped, 5),
            max_start(wrapped.rows.len(), 5)
        );
    }
}

use ratatui::layout::{Margin, Rect};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TuiLayout {
    pub(crate) header: Rect,
    pub(crate) transcript: Rect,
    pub(crate) transcript_inner: Rect,
    pub(crate) transcript_scrollbar: Rect,
    pub(crate) bottom_pane: Rect,
}

pub(crate) fn compute_layout(area: Rect, bottom_pane_height: u16) -> TuiLayout {
    let (header_height, transcript_height, bottom_height) = if area.height >= 7 {
        let header_height = 3;
        let bottom_height = bottom_pane_height
            .max(3)
            .min(area.height.saturating_sub(header_height + 1));
        (
            header_height,
            area.height
                .saturating_sub(header_height)
                .saturating_sub(bottom_height),
            bottom_height,
        )
    } else {
        let bottom_height = area.height.min(3);
        let header_height = area.height.saturating_sub(bottom_height).min(2);
        (
            header_height,
            area.height
                .saturating_sub(header_height)
                .saturating_sub(bottom_height),
            bottom_height,
        )
    };
    let header = Rect::new(area.x, area.y, area.width, header_height);
    let transcript = Rect::new(
        area.x,
        area.y.saturating_add(header_height),
        area.width,
        transcript_height,
    );
    let bottom_pane = Rect::new(
        area.x,
        area.y
            .saturating_add(header_height)
            .saturating_add(transcript_height),
        area.width,
        bottom_height,
    );
    let transcript_inner = transcript.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });
    let transcript_scrollbar = Rect {
        x: transcript
            .x
            .saturating_add(transcript.width.saturating_sub(1)),
        y: transcript.y.saturating_add(1),
        width: transcript.width.min(1),
        height: transcript.height.saturating_sub(2),
    };

    TuiLayout {
        header,
        transcript,
        transcript_inner,
        transcript_scrollbar,
        bottom_pane,
    }
}

pub(crate) fn rect_contains(area: Rect, x: u16, y: u16) -> bool {
    x >= area.x
        && x < area.x.saturating_add(area.width)
        && y >= area.y
        && y < area.y.saturating_add(area.height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_shared_transcript_inner_height() {
        let layout = compute_layout(Rect::new(0, 0, 100, 18), 3);

        assert_eq!(layout.header.height, 3);
        assert_eq!(layout.bottom_pane.height, 3);
        assert_eq!(layout.transcript.height, 12);
        assert_eq!(layout.transcript_inner.height, 10);
        assert_eq!(layout.transcript_scrollbar.height, 10);
    }

    #[test]
    fn tiny_heights_never_overlap_and_keep_bottom_pane_visible() {
        for height in 0..=6 {
            let area = Rect::new(0, 0, 20, height);
            let layout = compute_layout(area, 8);
            assert_eq!(
                layout.header.height + layout.transcript.height + layout.bottom_pane.height,
                height
            );
            assert!(layout.bottom_pane.y >= layout.transcript.y + layout.transcript.height);
            assert!(layout.bottom_pane.y + layout.bottom_pane.height <= height);
            if height > 0 {
                assert!(layout.bottom_pane.height > 0);
            }
        }
    }

    #[test]
    fn normal_layout_preserves_at_least_one_transcript_row() {
        for height in 7..=40 {
            let layout = compute_layout(Rect::new(0, 0, 80, height), 20);
            assert!(layout.transcript.height >= 1);
            assert_eq!(
                layout.header.height + layout.transcript.height + layout.bottom_pane.height,
                height
            );
        }
    }
}

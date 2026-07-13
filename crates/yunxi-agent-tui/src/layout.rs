use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TuiLayout {
    pub(crate) header: Rect,
    pub(crate) transcript: Rect,
    pub(crate) transcript_inner: Rect,
    pub(crate) transcript_scrollbar: Rect,
    pub(crate) bottom_pane: Rect,
}

pub(crate) fn compute_layout(area: Rect, bottom_pane_height: u16) -> TuiLayout {
    let bottom_height = bottom_pane_height.min(area.height.saturating_sub(4)).max(3);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(bottom_height),
        ])
        .split(area);
    let transcript = chunks[1];
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
        header: chunks[0],
        transcript,
        transcript_inner,
        transcript_scrollbar,
        bottom_pane: chunks[2],
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
}

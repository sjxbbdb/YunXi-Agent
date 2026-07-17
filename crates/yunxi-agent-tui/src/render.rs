use crate::app::YunxiTuiApp;
use crate::approval_layout::{ApprovalLayoutLine, ApprovalLineKind, approval_layout_for_width};
use crate::bottom_pane::BottomPaneMode;
use crate::layout::compute_layout;
use crate::transcript_layout::build_wrapped_transcript;
use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
};
use unicode_width::UnicodeWidthStr;

pub(crate) fn render_tui_frame(frame: &mut Frame<'_>, app: &YunxiTuiApp) {
    let area = frame.area();
    let layout = compute_layout(
        area,
        app.bottom_pane()
            .desired_height_for_width(area.width as usize),
    );

    render_header(frame, app, layout.header);
    render_transcript(
        frame,
        app,
        layout.transcript,
        layout.transcript_inner,
        layout.transcript_scrollbar,
    );
    render_bottom_pane(frame, app, layout.bottom_pane);
}

fn render_header(frame: &mut Frame<'_>, app: &YunxiTuiApp, area: Rect) {
    let header = Paragraph::new(vec![
        Line::from(Span::styled(
            app.header_for_width(area.width as usize),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            app.subheader_for_width(area.width as usize),
            Style::default().fg(Color::Gray),
        )),
    ])
    .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, area);
}

fn render_transcript(
    frame: &mut Frame<'_>,
    app: &YunxiTuiApp,
    area: Rect,
    inner: Rect,
    scrollbar_area: Rect,
) {
    let wrapped = build_wrapped_transcript(app.transcript().cells(), inner.width as usize);
    let visible = inner.height.max(1) as usize;
    let start = app.viewport().view_start(wrapped.rows.len(), visible);
    let end = start.saturating_add(visible).min(wrapped.rows.len());
    let title = transcript_title(app, start, end, wrapped.rows.len(), visible);
    let transcript = Paragraph::new(wrapped.rows[start..end].to_vec())
        .block(Block::default().title(title).borders(Borders::ALL));
    frame.render_widget(transcript, area);

    if wrapped.rows.len() > visible {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("^"))
            .end_symbol(Some("v"));
        let mut scrollbar_state = ScrollbarState::new(wrapped.rows.len())
            .position(start)
            .viewport_content_length(visible);
        frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }
}

fn render_bottom_pane(frame: &mut Frame<'_>, app: &YunxiTuiApp, area: Rect) {
    match app.bottom_pane().mode() {
        BottomPaneMode::Composer {
            prompt,
            buffer,
            cursor,
        } => render_composer(
            frame,
            area,
            app.footer_for_width(area.width as usize),
            prompt,
            buffer,
            *cursor,
        ),
        BottomPaneMode::Approval { request, selected } => {
            let layout = approval_layout_for_width(request, *selected, area.width as usize);
            let lines = layout
                .lines
                .into_iter()
                .map(render_approval_layout_line)
                .collect::<Vec<_>>();
            let pane = Paragraph::new(lines)
                .block(Block::default().title("Approval").borders(Borders::ALL));
            frame.render_widget(pane, area);
        }
        BottomPaneMode::UserInput {
            request,
            buffer,
            cursor,
        } => {
            let prompt = format!("{} ", request.prompt);
            render_composer(
                frame,
                area,
                "Enter submit | Esc cancel".to_string(),
                &prompt,
                buffer,
                *cursor,
            );
        }
    }
}

fn render_approval_layout_line(line: ApprovalLayoutLine) -> Line<'static> {
    match line {
        ApprovalLayoutLine::Label { kind, label, text } => {
            let label_style = match kind {
                ApprovalLineKind::Header | ApprovalLineKind::Risk => {
                    Style::default().fg(Color::Yellow)
                }
                ApprovalLineKind::Reason | ApprovalLineKind::Command => {
                    Style::default().fg(Color::Gray)
                }
            };
            let text_style = match kind {
                ApprovalLineKind::Header => Style::default().add_modifier(Modifier::BOLD),
                ApprovalLineKind::Risk => Style::default().fg(Color::Yellow),
                ApprovalLineKind::Reason | ApprovalLineKind::Command => Style::default(),
            };
            Line::from(vec![
                Span::styled(label, label_style),
                Span::styled(text, text_style),
            ])
        }
        ApprovalLayoutLine::Blank => Line::from(""),
        ApprovalLayoutLine::Action {
            label,
            selected,
            shortcut,
        } => option_line(label, selected, shortcut),
        ApprovalLayoutLine::Hint(value) => {
            Line::from(Span::styled(value, Style::default().fg(Color::DarkGray)))
        }
    }
}

fn render_composer(
    frame: &mut Frame<'_>,
    area: Rect,
    footer: String,
    prompt: &str,
    buffer: &str,
    cursor: usize,
) {
    let mut lines = Vec::new();
    let mut first = true;
    for line in buffer.split('\n') {
        if first {
            lines.push(Line::from(vec![
                Span::styled(prompt.to_string(), Style::default().fg(Color::Green)),
                Span::raw(line.to_string()),
            ]));
            first = false;
        } else {
            lines.push(Line::from(vec![
                Span::styled(" ".repeat(UnicodeWidthStr::width(prompt)), Style::default()),
                Span::raw(line.to_string()),
            ]));
        }
    }
    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            prompt.to_string(),
            Style::default().fg(Color::Green),
        )));
    }
    lines.push(Line::from(Span::styled(
        footer,
        Style::default().fg(Color::DarkGray),
    )));
    let pane = Paragraph::new(lines)
        .block(Block::default().title("Composer").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(pane, area);

    let cursor_position = composer_cursor_position(area, prompt, buffer, cursor);
    frame.set_cursor_position(cursor_position);
}

fn composer_cursor_position(area: Rect, prompt: &str, buffer: &str, cursor: usize) -> Position {
    let cursor = clamp_to_boundary(buffer, cursor);
    let before = &buffer[..cursor];
    let inner_width = area.width.saturating_sub(2).max(1) as usize;
    let prompt_width = UnicodeWidthStr::width(prompt);
    let body_width = inner_width.saturating_sub(prompt_width).max(1);
    let mut row = 0usize;
    for line in before
        .split('\n')
        .take(before.split('\n').count().saturating_sub(1))
    {
        row += wrapped_rows(line, body_width);
    }
    let current_line = before.rsplit('\n').next().unwrap_or("");
    let display_col = UnicodeWidthStr::width(current_line);
    row += display_col / body_width;
    let col = display_col % body_width;
    Position {
        x: area
            .x
            .saturating_add(1)
            .saturating_add(prompt_width as u16)
            .saturating_add(col as u16)
            .min(area.x.saturating_add(area.width.saturating_sub(2))),
        y: area
            .y
            .saturating_add(1)
            .saturating_add(row as u16)
            .min(area.y.saturating_add(area.height.saturating_sub(2))),
    }
}

fn wrapped_rows(value: &str, width: usize) -> usize {
    let width = width.max(1);
    UnicodeWidthStr::width(value)
        .checked_sub(1)
        .unwrap_or_default()
        / width
        + 1
}

fn option_line(label: &str, selected: bool, shortcut: &str) -> Line<'static> {
    let marker = if selected { ">" } else { " " };
    let style = if selected {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    Line::from(vec![
        Span::raw(format!("{marker} ")),
        Span::styled(format!("{label:<8}"), style),
        Span::styled(format!(" {shortcut}"), Style::default().fg(Color::DarkGray)),
    ])
}

fn transcript_title(
    app: &YunxiTuiApp,
    start: usize,
    end: usize,
    total: usize,
    visible: usize,
) -> String {
    if total <= visible.max(1) {
        return format!("Transcript | {}", app.viewport().scroll_status());
    }
    format!(
        "Transcript {}-{} / {} | {}",
        start.saturating_add(1),
        end,
        total,
        app.viewport().scroll_status()
    )
}

fn clamp_to_boundary(value: &str, cursor: usize) -> usize {
    let mut cursor = cursor.min(value.len());
    while cursor > 0 && !value.is_char_boundary(cursor) {
        cursor -= 1;
    }
    cursor
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::YunxiTuiBanner;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn banner() -> YunxiTuiBanner {
        YunxiTuiBanner {
            cwd: "D:/YunXi Agent/crates/yunxi-agent-cli".to_string(),
            backend: "yunxi".to_string(),
            provider_live: true,
            provider_source: "auto_live".to_string(),
            provider: "deepseek".to_string(),
            model: "deepseek-chat-ultra-long-model-name".to_string(),
        }
    }

    fn render_app(app: &YunxiTuiApp, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| render_tui_frame(frame, app))
            .expect("draw");
        format!("{:?}", terminal.backend().buffer())
    }

    #[test]
    fn composer_cursor_uses_display_width_for_cjk_text() {
        let area = Rect::new(0, 0, 58, 6);
        let position = composer_cursor_position(area, "yunxi> ", "你好abc", "你好abc".len());

        assert_eq!(position.x, 1 + "yunxi> ".len() as u16 + 7);
        assert_eq!(position.y, 1);
    }

    #[test]
    fn composer_cursor_wraps_inside_composer_bounds() {
        let area = Rect::new(0, 0, 24, 6);
        let input = "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz";
        let position = composer_cursor_position(area, "yunxi> ", input, input.len());

        assert!(position.x < area.width - 1);
        assert!(position.y < area.height - 1);
        assert!(position.y > 1);
    }

    fn approval_app() -> YunxiTuiApp {
        let mut app = YunxiTuiApp::default();
        app.set_banner(banner());
        app.start_approval(crate::bottom_pane::ApprovalRequestView {
            id: None,
            tool_name: "shell".to_string(),
            cwd: "D:/YunXi Agent/workspace/with/a/very/long/path".to_string(),
            command: Some(
                "Remove-Item -Recurse -Force D:/YunXi Agent/workspace/generated/very-long-output"
                    .to_string(),
            ),
            reason: "requires approval before running a destructive command".to_string(),
            risk_label: Some("risk: destructive".to_string()),
        });
        app
    }

    #[test]
    fn renders_approval_overlay_without_plain_prompt() {
        let mut app = YunxiTuiApp::default();
        app.set_banner(banner());
        app.start_approval(crate::bottom_pane::ApprovalRequestView {
            id: None,
            tool_name: "shell".to_string(),
            cwd: ".".to_string(),
            command: Some("echo hi".to_string()),
            reason: "requires approval".to_string(),
            risk_label: None,
        });

        let rendered = render_app(&app, 100, 18);

        assert!(rendered.contains("Approval"));
        assert!(rendered.contains("Approve"));
        assert!(rendered.contains("Decline"));
        assert!(rendered.contains("Tab changes selection"));
        assert!(rendered.contains("risk"));
        assert!(rendered.contains("risk: low"));
        assert!(!rendered.contains("approve? y/N"));
    }

    #[test]
    fn narrow_tui_header_does_not_render_dangling_separator() {
        let mut app = YunxiTuiApp::default();
        app.set_banner(YunxiTuiBanner {
            cwd: "D:/YunXi Agent/crates/yunxi-agent-cli".to_string(),
            backend: "yunxi".to_string(),
            provider_live: false,
            provider_source: "offline_static".to_string(),
            provider: "static".to_string(),
            model: "deepseek-chat".to_string(),
        });

        let rendered = render_app(&app, 58, 20);

        assert!(rendered.contains("YunXi v1.8.8"));
        assert!(rendered.contains("debug off"));
        assert!(!rendered.contains("|,"));
    }

    #[test]
    fn approval_actions_stay_visible_on_58_column_terminal() {
        let rendered = render_app(&approval_app(), 58, 22);

        assert!(rendered.contains("Approval"));
        assert!(rendered.contains("Approve"));
        assert!(rendered.contains("Decline"));
        assert!(rendered.contains("Tab changes selection"));
        assert!(rendered.contains("risk: destructive"));
    }

    #[test]
    fn approval_actions_stay_visible_on_tight_58_column_terminal() {
        let rendered = render_app(&approval_app(), 58, 18);

        assert!(rendered.contains("Approve"));
        assert!(rendered.contains("Decline"));
        assert!(rendered.contains("Tab changes selection"));
    }

    #[test]
    fn approval_actions_stay_visible_on_medium_and_wide_terminals() {
        for (width, height) in [(80, 22), (100, 24)] {
            let rendered = render_app(&approval_app(), width, height);

            assert!(rendered.contains("Approve"));
            assert!(rendered.contains("Decline"));
            assert!(rendered.contains("Tab changes selection"));
            assert!(rendered.contains("risk: destructive"));
        }
    }

    #[test]
    fn medium_width_header_keeps_model_visible() {
        let mut app = YunxiTuiApp::default();
        app.set_banner(banner());

        let rendered = render_app(&app, 100, 30);

        assert!(rendered.contains("deepseek live"));
        assert!(rendered.contains("model=deepseek-chat"));
    }

    #[test]
    fn transcript_scroll_renders_history_window_and_scrollbar_title() {
        let mut app = YunxiTuiApp::default();
        for idx in 0..30 {
            app.push_notice("event", &format!("line-{idx:02}"));
        }
        app.jump_top(30, 8);

        let rendered = render_app(&app, 100, 18);

        assert!(rendered.contains("Transcript"));
        assert!(rendered.contains("history"));
        assert!(rendered.contains("line-00"));
        assert!(!rendered.contains("line-29"));
    }

    #[test]
    fn transcript_reports_new_output_below_when_scrolled_history_changes() {
        let mut app = YunxiTuiApp::default();
        for idx in 0..30 {
            app.push_notice("event", &format!("line-{idx:02}"));
        }
        app.scroll_up(8, 30, 10);
        app.push_notice("event", "fresh-line");

        let rendered = render_app(&app, 100, 18);

        assert!(rendered.contains("new output below"));
        assert!(!rendered.contains("fresh-line"));
    }

    #[test]
    fn transcript_scroll_uses_wrapped_rows_for_title_and_tail() {
        let mut app = YunxiTuiApp::default();
        app.push_assistant(
            "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz",
        );

        let rendered = render_app(&app, 28, 12);

        assert!(rendered.contains("Transcript"));
        assert!(rendered.contains("tail"));
        assert!(rendered.contains("/"));
    }
}

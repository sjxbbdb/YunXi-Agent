use crate::app::YunxiTuiApp;
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
            let mut lines = vec![
                Line::from(vec![
                    Span::styled("approval ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        &request.tool_name,
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(format!(" in {}", request.cwd)),
                ]),
                Line::from(vec![
                    Span::styled("reason   ", Style::default().fg(Color::Gray)),
                    Span::raw(request.reason.clone()),
                ]),
                Line::from(vec![
                    Span::styled("risk     ", Style::default().fg(Color::Yellow)),
                    Span::raw(request.risk_label()),
                ]),
            ];
            if let Some(command) = &request.command {
                lines.push(Line::from(vec![
                    Span::styled("command  ", Style::default().fg(Color::Gray)),
                    Span::raw(command.clone()),
                ]));
            }
            lines.push(Line::from(""));
            lines.push(option_line("Approve", *selected == 0, "Enter/Y"));
            lines.push(option_line("Decline", *selected == 1, "N/Esc"));
            lines.push(Line::from(Span::styled(
                "Tab changes selection",
                Style::default().fg(Color::DarkGray),
            )));
            let pane = Paragraph::new(lines)
                .block(Block::default().title("Approval").borders(Borders::ALL))
                .wrap(Wrap { trim: false });
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
                Span::styled("  ".repeat(prompt.chars().count()), Style::default()),
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
    let row = before.chars().filter(|ch| *ch == '\n').count() as u16;
    let col = before.rsplit('\n').next().unwrap_or("").chars().count() as u16;
    Position {
        x: area
            .x
            .saturating_add(1)
            .saturating_add(prompt.chars().count() as u16)
            .saturating_add(col),
        y: area.y.saturating_add(1).saturating_add(row),
    }
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

    #[test]
    fn renders_approval_overlay_without_plain_prompt() {
        let mut app = YunxiTuiApp::default();
        app.set_banner(YunxiTuiBanner {
            cwd: "D:/YunXi Agent".to_string(),
            backend: "yunxi".to_string(),
            provider_live: true,
            provider_source: "auto_live".to_string(),
            provider: "deepseek".to_string(),
            model: "deepseek-chat".to_string(),
        });
        app.start_approval(crate::bottom_pane::ApprovalRequestView {
            id: None,
            tool_name: "shell".to_string(),
            cwd: ".".to_string(),
            command: Some("echo hi".to_string()),
            reason: "requires approval".to_string(),
            risk_label: None,
        });

        let backend = TestBackend::new(100, 18);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| render_tui_frame(frame, &app))
            .expect("draw");
        let rendered = format!("{:?}", terminal.backend().buffer());

        assert!(rendered.contains("Approval"));
        assert!(rendered.contains("Approve"));
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

        let backend = TestBackend::new(58, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| render_tui_frame(frame, &app))
            .expect("draw");
        let rendered = format!("{:?}", terminal.backend().buffer());

        assert!(rendered.contains("YunXi v1.7.6"));
        assert!(rendered.contains("debug off"));
        assert!(!rendered.contains("|,"));
    }

    #[test]
    fn transcript_scroll_renders_history_window_and_scrollbar_title() {
        let mut app = YunxiTuiApp::default();
        for idx in 0..30 {
            app.push_notice("event", &format!("line-{idx:02}"));
        }
        app.jump_top(30, 8);

        let backend = TestBackend::new(100, 18);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| render_tui_frame(frame, &app))
            .expect("draw");
        let rendered = format!("{:?}", terminal.backend().buffer());

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

        let backend = TestBackend::new(100, 18);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| render_tui_frame(frame, &app))
            .expect("draw");
        let rendered = format!("{:?}", terminal.backend().buffer());

        assert!(rendered.contains("new output below"));
        assert!(!rendered.contains("fresh-line"));
    }

    #[test]
    fn transcript_scroll_uses_wrapped_rows_for_title_and_tail() {
        let mut app = YunxiTuiApp::default();
        app.push_assistant(
            "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz",
        );

        let backend = TestBackend::new(28, 12);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| render_tui_frame(frame, &app))
            .expect("draw");
        let rendered = format!("{:?}", terminal.backend().buffer());

        assert!(rendered.contains("Transcript"));
        assert!(rendered.contains("tail"));
        assert!(rendered.contains("/"));
    }
}

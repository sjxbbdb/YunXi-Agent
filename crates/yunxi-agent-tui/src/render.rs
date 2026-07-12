use crate::app::YunxiTuiApp;
use crate::bottom_pane::BottomPaneMode;
use crate::chat::HistoryCell;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

pub(crate) fn render_tui_frame(frame: &mut Frame<'_>, app: &YunxiTuiApp) {
    let area = frame.area();
    let bottom_height = app
        .bottom_pane()
        .desired_height()
        .min(area.height.saturating_sub(4));
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(bottom_height.max(3)),
        ])
        .split(area);

    render_header(frame, app, chunks[0]);
    render_transcript(frame, app, chunks[1]);
    render_bottom_pane(frame, app, chunks[2]);
}

fn render_header(frame: &mut Frame<'_>, app: &YunxiTuiApp, area: Rect) {
    let header = Paragraph::new(vec![
        Line::from(Span::styled(
            app.header(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            app.subheader(),
            Style::default().fg(Color::Gray),
        )),
    ])
    .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, area);
}

fn render_transcript(frame: &mut Frame<'_>, app: &YunxiTuiApp, area: Rect) {
    let mut lines = Vec::new();
    for cell in app.transcript().cells() {
        push_cell_lines(&mut lines, cell);
    }
    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "Ready.",
            Style::default().fg(Color::DarkGray),
        )));
    }
    let visible = area.height.saturating_sub(2) as usize;
    let start = lines.len().saturating_sub(visible.max(1));
    let transcript = Paragraph::new(lines[start..].to_vec())
        .block(Block::default().title("Transcript").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(transcript, area);
}

fn render_bottom_pane(frame: &mut Frame<'_>, app: &YunxiTuiApp, area: Rect) {
    match app.bottom_pane().mode() {
        BottomPaneMode::Composer {
            prompt,
            buffer,
            cursor,
        } => render_composer(frame, area, app.footer(), prompt, buffer, *cursor),
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
                "Enter submit | Esc cancel",
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
    footer: &str,
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
        footer.to_string(),
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

fn push_cell_lines(lines: &mut Vec<Line<'static>>, cell: &HistoryCell) {
    match cell {
        HistoryCell::User(content) => push_labeled(lines, "user", Color::Green, content),
        HistoryCell::Assistant { content, active } => {
            let label = if *active { "assistant*" } else { "assistant" };
            push_labeled(lines, label, Color::LightGreen, content);
        }
        HistoryCell::Reasoning { content, active } => {
            let label = if *active { "thinking*" } else { "thinking" };
            push_labeled(lines, label, Color::Blue, content);
        }
        HistoryCell::Event { kind, message } => {
            push_labeled(lines, kind, label_color(kind), message)
        }
        HistoryCell::Warning(message) => push_labeled(lines, "warning", Color::Yellow, message),
        HistoryCell::Error(message) => push_labeled(lines, "error", Color::Red, message),
    }
}

fn push_labeled(lines: &mut Vec<Line<'static>>, label: &str, color: Color, content: &str) {
    let mut iter = content.lines();
    let first = iter.next().unwrap_or("");
    lines.push(Line::from(vec![
        Span::styled(
            format!("[{label}] "),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::raw(first.to_string()),
    ]));
    for rest in iter {
        lines.push(Line::from(vec![
            Span::styled("  | ".to_string(), Style::default().fg(Color::DarkGray)),
            Span::raw(rest.to_string()),
        ]));
    }
}

fn label_color(label: &str) -> Color {
    match label {
        "shell" | "tool" | "mcp" => Color::Magenta,
        "approval" | "escalation" => Color::Yellow,
        "context" | "session" | "usage" => Color::Gray,
        "cancelled" | "provider" => Color::Red,
        _ => Color::White,
    }
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
        });

        let backend = TestBackend::new(100, 18);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| render_tui_frame(frame, &app))
            .expect("draw");
        let rendered = format!("{:?}", terminal.backend().buffer());

        assert!(rendered.contains("Approval"));
        assert!(rendered.contains("Approve"));
        assert!(!rendered.contains("approve? y/N"));
    }
}

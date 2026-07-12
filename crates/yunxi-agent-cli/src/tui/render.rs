use crate::tui::app::TuiApp;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

pub(crate) fn render_tui_frame(frame: &mut Frame<'_>, app: &TuiApp) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(3),
        ])
        .split(area);

    let header = Paragraph::new(vec![
        Line::from(Span::styled(
            app.header(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::raw(app.subheader())),
    ])
    .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, chunks[0]);

    let lines = app
        .events
        .iter()
        .map(|event| {
            Line::from(vec![
                Span::styled(
                    format!("[{}] ", event.label),
                    Style::default()
                        .fg(label_color(&event.label))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(event.message.clone()),
            ])
        })
        .collect::<Vec<_>>();
    let log = Paragraph::new(lines)
        .block(Block::default().title("Events").borders(Borders::ALL))
        .scroll((app.scroll, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(log, chunks[1]);

    let input = Paragraph::new(app.input.clone())
        .block(Block::default().title("Input").borders(Borders::ALL));
    frame.render_widget(input, chunks[2]);
}

fn label_color(label: &str) -> Color {
    match label {
        "assistant" => Color::Green,
        "reasoning" => Color::Blue,
        "warning" => Color::Yellow,
        "error" | "provider" | "cancelled" => Color::Red,
        "shell" | "tool" | "mcp" => Color::Magenta,
        _ => Color::White,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::InteractiveBanner;
    use crate::tui::app::TuiApp;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_backend_renders_header_log_and_input() {
        let banner = InteractiveBanner {
            cwd: "D:/YunXi Agent".to_string(),
            backend: "yunxi".to_string(),
            provider_live: true,
            provider_source: "auto_live".to_string(),
            provider: "deepseek".to_string(),
            model: "deepseek-chat".to_string(),
        };
        let mut app = TuiApp::from_banner(&banner);
        app.push_line("assistant", "hello from tui");

        let backend = TestBackend::new(80, 12);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| render_tui_frame(frame, &app))
            .expect("draw");
        let rendered = format!("{:?}", terminal.backend().buffer());

        assert!(rendered.contains("YunXi Agent"));
        assert!(rendered.contains("hello from tui"));
        assert!(rendered.contains("yunxi>"));
    }
}

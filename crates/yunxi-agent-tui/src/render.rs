use crate::app::YunxiTuiApp;
use crate::approval_layout::{ApprovalLayoutLine, ApprovalLineKind, approval_layout_for_width};
use crate::bottom_pane::BottomPaneMode;
use crate::edit_buffer::EditBuffer;
use crate::input_map::FocusTarget;
use crate::layout::compute_layout;
use crate::styles::{TuiSemanticStyle, TuiStyleSet};
use crate::text_layout::{TextLayout, WrapPolicy};
#[cfg(test)]
use crate::transcript_layout::build_wrapped_transcript;
use crate::transcript_layout::build_wrapped_transcript_with_styles;
use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
};

pub(crate) fn render_tui_frame(frame: &mut Frame<'_>, app: &YunxiTuiApp) {
    render_tui_frame_with_styles(frame, app, TuiStyleSet::detect());
}

pub(crate) fn render_tui_frame_with_styles(
    frame: &mut Frame<'_>,
    app: &YunxiTuiApp,
    styles: TuiStyleSet,
) {
    let area = frame.area();
    let layout = compute_layout(
        area,
        app.bottom_pane()
            .desired_height_for_width(area.width as usize),
    );

    if layout.header.width > 0 && layout.header.height > 0 {
        render_header(frame, app, layout.header, styles);
    }
    if layout.transcript.width == 0 || layout.transcript.height == 0 {
        // The bottom pane gets priority on extremely small terminals.
    } else if app.details().is_some() {
        render_details(frame, app, layout.transcript, styles);
    } else if app.control_snapshot().is_some() {
        render_controls(frame, app, layout.transcript, styles);
    } else {
        render_transcript(
            frame,
            app,
            layout.transcript,
            layout.transcript_inner,
            layout.transcript_scrollbar,
            styles,
        );
    }
    if layout.bottom_pane.width > 0 && layout.bottom_pane.height > 0 {
        render_bottom_pane(frame, app, layout.bottom_pane, styles);
    }
}

fn render_details(frame: &mut Frame<'_>, app: &YunxiTuiApp, area: Rect, styles: TuiStyleSet) {
    let Some(details) = app.details() else {
        return;
    };
    let panel = Paragraph::new(details.to_string())
        .block(
            Block::default()
                .title(Span::styled(
                    "Details | Esc close | wheel/PgUp/PgDown scroll",
                    styles.style(TuiSemanticStyle::Focus),
                ))
                .borders(Borders::ALL)
                .border_style(styles.style(TuiSemanticStyle::Focus)),
        )
        .scroll((app.details_scroll(), 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(panel, area);
}

fn render_controls(frame: &mut Frame<'_>, app: &YunxiTuiApp, area: Rect, styles: TuiStyleSet) {
    let Some(snapshot) = app.control_snapshot() else {
        return;
    };
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                "Local companion: ",
                styles.style(TuiSemanticStyle::Subheader),
            ),
            Span::styled(
                if snapshot.companion_enabled {
                    "ON"
                } else {
                    "OFF"
                },
                styles.style(if snapshot.companion_enabled {
                    TuiSemanticStyle::Success
                } else {
                    TuiSemanticStyle::Notice
                }),
            ),
            Span::raw("   "),
            Span::styled("Cloud control: ", styles.style(TuiSemanticStyle::Subheader)),
            Span::styled(
                if snapshot.cloud_control_enabled {
                    "ON"
                } else {
                    "OFF"
                },
                styles.style(if snapshot.cloud_control_enabled {
                    TuiSemanticStyle::Warning
                } else {
                    TuiSemanticStyle::Success
                }),
            ),
        ]),
        Line::from(format!(
            "Quiet hours: {}",
            snapshot.quiet_hours.as_deref().unwrap_or("none")
        )),
        Line::from(""),
    ];
    for state in &snapshot.scopes {
        let enabled = state
            .enabled
            .map(|value| if value { "on" } else { "off" })
            .unwrap_or("read-only");
        lines.push(Line::from(vec![
            Span::styled(
                format!("{:<13}", state.scope.as_str()),
                styles.style(TuiSemanticStyle::Header),
            ),
            Span::raw(format!(" {enabled:<9} source={} ", state.source.as_str())),
        ]));
        lines.push(Line::from(Span::styled(
            format!("  {}", state.summary),
            styles.style(TuiSemanticStyle::Muted),
        )));
        if let Some(effect) = &state.clear_effect {
            lines.push(Line::from(Span::styled(
                format!("  clear scope: {effect}"),
                styles.style(TuiSemanticStyle::Warning),
            )));
        }
    }
    lines.push(Line::from(""));
    if let Some(change) = &snapshot.recent_change {
        lines.push(Line::from(format!("Recent change: {change}")));
    }
    lines.push(Line::from(Span::styled(
        "/controls refresh | /companion on|off | /controls clear memory",
        styles.style(TuiSemanticStyle::Footer),
    )));
    let panel = Paragraph::new(lines)
        .block(
            Block::default()
                .title(Span::styled(
                    "Companion UX & Controls",
                    styles.style(TuiSemanticStyle::Header),
                ))
                .borders(Borders::ALL)
                .border_style(styles.style(TuiSemanticStyle::Border)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(panel, area);
}

fn render_header(frame: &mut Frame<'_>, app: &YunxiTuiApp, area: Rect, styles: TuiStyleSet) {
    let header = Paragraph::new(vec![
        Line::from(Span::styled(
            app.header_for_width(area.width as usize),
            styles.style(TuiSemanticStyle::Header),
        )),
        Line::from(Span::styled(
            app.subheader_for_width(area.width as usize),
            styles.style(TuiSemanticStyle::Subheader),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(styles.style(TuiSemanticStyle::Border)),
    );
    frame.render_widget(header, area);
}

fn render_transcript(
    frame: &mut Frame<'_>,
    app: &YunxiTuiApp,
    area: Rect,
    inner: Rect,
    scrollbar_area: Rect,
    styles: TuiStyleSet,
) {
    let wrapped = build_wrapped_transcript_with_styles(
        app.transcript().cells(),
        inner.width as usize,
        styles,
    );
    let visible = inner.height.max(1) as usize;
    let start = app.viewport().view_start(&wrapped, visible);
    let end = start.saturating_add(visible).min(wrapped.rows.len());
    let title = transcript_title(app, start, end, wrapped.rows.len(), visible);
    let border_semantic = if app.focus_target() == FocusTarget::History {
        TuiSemanticStyle::Focus
    } else {
        TuiSemanticStyle::Border
    };
    let transcript = Paragraph::new(wrapped.rows[start..end].to_vec()).block(
        Block::default()
            .title(Span::styled(
                title,
                styles.style(TuiSemanticStyle::Subheader),
            ))
            .borders(Borders::ALL)
            .border_style(styles.style(border_semantic)),
    );
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

fn render_bottom_pane(frame: &mut Frame<'_>, app: &YunxiTuiApp, area: Rect, styles: TuiStyleSet) {
    match app.bottom_pane().mode() {
        BottomPaneMode::Composer => render_composer(
            frame,
            area,
            app.footer_for_width(area.width as usize),
            "Composer",
            app.bottom_pane().composer_prompt(),
            app.bottom_pane().composer_buffer(),
            styles,
            TuiSemanticStyle::Focus,
        ),
        BottomPaneMode::Approval { request, selected } => {
            let layout = approval_layout_for_width(request, *selected, area.width as usize);
            let lines = layout
                .lines
                .into_iter()
                .map(|line| render_approval_layout_line(line, styles))
                .collect::<Vec<_>>();
            let pane = Paragraph::new(lines).block(
                Block::default()
                    .title(Span::styled(
                        "Approval required | default: Decline",
                        styles.style(TuiSemanticStyle::ActionRequired),
                    ))
                    .borders(Borders::ALL)
                    .border_style(styles.style(TuiSemanticStyle::ActionRequired)),
            );
            frame.render_widget(pane, area);
        }
        BottomPaneMode::UserInput { request, buffer } => {
            let prompt = format!("{} ", request.prompt);
            render_composer(
                frame,
                area,
                "Enter submit | Esc cancel".to_string(),
                "Input",
                &prompt,
                buffer,
                styles,
                TuiSemanticStyle::ActionRequired,
            );
        }
    }
}

fn render_approval_layout_line(line: ApprovalLayoutLine, styles: TuiStyleSet) -> Line<'static> {
    match line {
        ApprovalLayoutLine::Label { kind, label, text } => {
            let label_style = match kind {
                ApprovalLineKind::Header | ApprovalLineKind::Risk => {
                    styles.style(TuiSemanticStyle::ActionRequired)
                }
                ApprovalLineKind::Reason | ApprovalLineKind::Command => {
                    styles.style(TuiSemanticStyle::Subheader)
                }
            };
            let text_style = match kind {
                ApprovalLineKind::Header | ApprovalLineKind::Risk => {
                    styles.style(TuiSemanticStyle::ActionRequired)
                }
                ApprovalLineKind::Reason | ApprovalLineKind::Command => {
                    styles.style(TuiSemanticStyle::Notice)
                }
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
        } => option_line(label, selected, shortcut, styles),
        ApprovalLayoutLine::Hint(value) => {
            Line::from(Span::styled(value, styles.style(TuiSemanticStyle::Footer)))
        }
    }
}

fn render_composer(
    frame: &mut Frame<'_>,
    area: Rect,
    footer: String,
    title: &str,
    prompt: &str,
    buffer: &EditBuffer,
    styles: TuiStyleSet,
    title_semantic: TuiSemanticStyle,
) {
    let prompt_width = TextLayout::measure(prompt);
    let inner_width = area.width.saturating_sub(2).max(1) as usize;
    let body_width = inner_width.saturating_sub(prompt_width).max(1);
    let visual_lines = TextLayout::wrap(buffer.text(), body_width, WrapPolicy::CodeBlock);
    let visual_cursor = TextLayout::cursor_position(
        buffer.text(),
        buffer.cursor_byte_offset(),
        body_width,
        WrapPolicy::CodeBlock,
    );
    let visible_content_rows = area.height.saturating_sub(3).max(1) as usize;
    let first_visible_row = visual_cursor
        .row
        .saturating_add(1)
        .saturating_sub(visible_content_rows);
    let mut lines = visual_lines
        .into_iter()
        .enumerate()
        .skip(first_visible_row)
        .take(visible_content_rows)
        .map(|(index, line)| {
            if index == 0 {
                Line::from(vec![
                    Span::styled(prompt.to_string(), styles.style(TuiSemanticStyle::Focus)),
                    Span::raw(line.text),
                ])
            } else {
                Line::from(vec![
                    Span::raw(" ".repeat(prompt_width)),
                    Span::raw(line.text),
                ])
            }
        })
        .collect::<Vec<_>>();
    if lines.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(prompt.to_string(), styles.style(TuiSemanticStyle::Focus)),
            Span::raw(String::new()),
        ]));
    }
    lines.push(Line::from(Span::styled(
        footer,
        styles.style(TuiSemanticStyle::Footer),
    )));
    let pane = Paragraph::new(lines)
        .block(
            Block::default()
                .title(Span::styled(
                    title.to_string(),
                    styles.style(title_semantic),
                ))
                .borders(Borders::ALL)
                .border_style(styles.style(title_semantic)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(pane, area);

    if area.width > 2 && area.height > 2 {
        let cursor_position = composer_cursor_position(area, prompt, buffer, first_visible_row);
        frame.set_cursor_position(cursor_position);
    }
}

fn composer_cursor_position(
    area: Rect,
    prompt: &str,
    buffer: &EditBuffer,
    first_visible_row: usize,
) -> Position {
    let inner_width = area.width.saturating_sub(2).max(1) as usize;
    let prompt_width = TextLayout::measure(prompt);
    let body_width = inner_width.saturating_sub(prompt_width).max(1);
    let visual = TextLayout::cursor_position(
        buffer.text(),
        buffer.cursor_byte_offset(),
        body_width,
        WrapPolicy::CodeBlock,
    );
    Position {
        x: area
            .x
            .saturating_add(1)
            .saturating_add(prompt_width as u16)
            .saturating_add(visual.column as u16)
            .min(area.x.saturating_add(area.width.saturating_sub(2))),
        y: area
            .y
            .saturating_add(1)
            .saturating_add(visual.row.saturating_sub(first_visible_row) as u16)
            .min(area.y.saturating_add(area.height.saturating_sub(2))),
    }
}

fn option_line(label: &str, selected: bool, shortcut: &str, styles: TuiStyleSet) -> Line<'static> {
    let marker = if selected { ">" } else { " " };
    let style = if selected {
        styles.style(TuiSemanticStyle::Selection)
    } else {
        styles.style(TuiSemanticStyle::Notice)
    };
    Line::from(vec![
        Span::raw(format!("{marker} ")),
        Span::styled(format!("{label:<8}"), style),
        Span::styled(
            format!(" {shortcut}"),
            styles.style(TuiSemanticStyle::Footer),
        ),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::YunxiTuiBanner;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use unicode_segmentation::UnicodeSegmentation;
    use unicode_width::UnicodeWidthStr;

    fn banner() -> YunxiTuiBanner {
        YunxiTuiBanner {
            cwd: "C:\\Users\\24763\\YunXi Agent\\国际化工作区\\crates\\yunxi-agent-cli".to_string(),
            backend: "yunxi".to_string(),
            provider_live: true,
            provider_source: "auto_live".to_string(),
            provider: "deepseek".to_string(),
            model: "deepseek-chat-ultra-long-model-name".to_string(),
        }
    }

    fn render_app(app: &YunxiTuiApp, width: u16, height: u16) -> String {
        render_app_with_styles(app, width, height, TuiStyleSet::detect())
    }

    fn render_app_with_styles(
        app: &YunxiTuiApp,
        width: u16,
        height: u16,
        styles: TuiStyleSet,
    ) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| render_tui_frame_with_styles(frame, app, styles))
            .expect("draw");
        format!("{:?}", terminal.backend().buffer())
    }

    fn render_full_frame_snapshot(app: &YunxiTuiApp, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| render_tui_frame(frame, app))
            .expect("draw");
        let buffer = terminal.backend().buffer();
        (0..height)
            .map(|y| {
                let mut row = String::new();
                let mut x = 0;
                while x < width {
                    let symbol = buffer[(x, y)].symbol();
                    row.push_str(symbol);
                    x = x.saturating_add(UnicodeWidthStr::width(symbol).max(1) as u16);
                }
                format!("{y:02}|{}", row.trim_end())
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn full_frame_snapshot_app(width: u16, height: u16) -> YunxiTuiApp {
        use yunxi_agent_core::{
            AgentEvent, AgentMessageSequence, AgentMessageStream, AgentMessageStreamPhase,
        };

        fn stream_event(content: &str, sequence: u64) -> AgentEvent {
            AgentEvent::Message {
                content: content.to_string(),
                stream: Some(AgentMessageStream {
                    thread_id: "snapshot-thread".to_string(),
                    turn_id: "snapshot-turn".to_string(),
                    stream_id: "snapshot-message".to_string(),
                    event_id: format!("snapshot-event-{sequence}"),
                    source_sequence: AgentMessageSequence::LocalFallback(sequence),
                    phase: AgentMessageStreamPhase::Delta,
                }),
            }
        }

        let mut app = YunxiTuiApp::default();
        app.set_banner(banner());
        app.start_prompt("yunxi> ");
        app.bottom_pane_mut().paste("入力 中文かな 👩‍💻 e\u{301}");
        for index in 0..36 {
            app.push_notice(
                "history",
                &format!("history {index:02}: stable transcript row for viewport evidence"),
            );
        }
        app.push_user("Keep the current history position while the answer streams.");
        app.push_agent_event(&stream_event(
            "国際化 layout 中文かな keeps one active cell with emoji 👩‍💻 and e\u{301}.\nURL https://very-long.example.test/api/v1/items?search=中文&sort=desc#results\nWindows C:\\Users\\24763\\YunXi Agent\\输出目录\\very-long-file-name.txt\n```rust\n    let greeting = \"你好、世界\"; // 日本語と中文\n```",
            1,
        ));

        let layout = compute_layout(
            Rect::new(0, 0, width, height),
            app.bottom_pane().desired_height_for_width(width as usize),
        );
        let wrapped = build_wrapped_transcript(
            app.transcript().cells(),
            layout.transcript_inner.width as usize,
        );
        app.scroll_up(1, &wrapped, layout.transcript_inner.height as usize);
        app.push_agent_event(&stream_event(
            "国際化 layout 中文かな keeps one active cell with emoji 👩‍💻 and e\u{301}.\nURL https://very-long.example.test/api/v1/items?search=中文&sort=desc#results\nWindows C:\\Users\\24763\\YunXi Agent\\输出目录\\very-long-file-name.txt\n```rust\n    let greeting = \"你好、世界\"; // 日本語と中文\n```\nNew output remains below the pinned viewport without stealing follow-tail.",
            2,
        ));
        assert_eq!(app.viewport().scroll_status(), "new output below");
        app
    }

    fn assert_full_frame_snapshot(width: u16, height: u16, expected: &str) -> String {
        let app = full_frame_snapshot_app(width, height);
        let snapshot = render_full_frame_snapshot(&app, width, height);
        let layout = compute_layout(
            Rect::new(0, 0, width, height),
            app.bottom_pane().desired_height_for_width(width as usize),
        );

        assert_eq!(snapshot.lines().count(), height as usize);
        for row in snapshot.lines() {
            let content = row.split_once('|').expect("snapshot row prefix").1;
            assert!(
                UnicodeWidthStr::width(content) <= width as usize,
                "row overflow at {width}x{height}: {row}"
            );
        }
        assert_eq!(
            layout.header.y.saturating_add(layout.header.height),
            layout.transcript.y
        );
        assert_eq!(
            layout.transcript.y.saturating_add(layout.transcript.height),
            layout.bottom_pane.y
        );
        assert_eq!(
            layout
                .bottom_pane
                .y
                .saturating_add(layout.bottom_pane.height),
            height
        );
        assert!(layout.transcript_scrollbar.x >= layout.transcript.x);
        assert!(
            layout
                .transcript_scrollbar
                .x
                .saturating_add(layout.transcript_scrollbar.width)
                <= layout.transcript.x.saturating_add(layout.transcript.width)
        );
        assert!(
            layout
                .transcript_scrollbar
                .y
                .saturating_add(layout.transcript_scrollbar.height)
                <= layout.transcript.y.saturating_add(layout.transcript.height)
        );
        for required in [
            "[assistant*]",
            "new output below",
            "^",
            "v",
            "Composer",
            "yunxi> 入力 中文かな 👩‍💻 e\u{301}",
            "End follow tail",
            "国際化",
        ] {
            assert!(
                snapshot.contains(required),
                "missing {required} at {width}x{height}"
            );
        }
        if std::env::var_os("YUNXI_UPDATE_SNAPSHOTS").is_some() {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src")
                .join("snapshots")
                .join(format!("full_frame_{width}x{height}.txt"));
            std::fs::write(path, format!("{snapshot}\n")).expect("write full-frame snapshot");
            return snapshot;
        }
        assert_eq!(snapshot, expected.trim_end_matches(['\r', '\n']));
        snapshot
    }

    #[test]
    fn full_frame_snapshot_58x18_covers_tight_information_density() {
        assert_full_frame_snapshot(58, 18, include_str!("snapshots/full_frame_58x18.txt"));
    }

    #[test]
    fn full_frame_snapshot_80x24_covers_stream_history_and_composer() {
        assert_full_frame_snapshot(80, 24, include_str!("snapshots/full_frame_80x24.txt"));
    }

    #[test]
    fn full_frame_snapshot_120x40_covers_stream_history_and_composer() {
        assert_full_frame_snapshot(120, 40, include_str!("snapshots/full_frame_120x40.txt"));
    }

    #[test]
    fn full_frame_snapshot_100x30_covers_international_responsive_layout() {
        assert_full_frame_snapshot(100, 30, include_str!("snapshots/full_frame_100x30.txt"));
    }

    #[test]
    fn full_frame_snapshot_200x50_covers_international_responsive_layout() {
        assert_full_frame_snapshot(200, 50, include_str!("snapshots/full_frame_200x50.txt"));
    }

    #[test]
    fn renders_shared_control_snapshot_with_scope_and_clear_effects() {
        use yunxi_agent_core::{
            ControlScope, ControlScopeSnapshot, ControlSnapshot, ControlSource,
        };
        let mut app = YunxiTuiApp::default();
        app.set_banner(banner());
        app.show_control_snapshot(ControlSnapshot {
            companion_enabled: false,
            cloud_control_enabled: false,
            quiet_hours: Some("23:00-07:00".to_string()),
            persona_summary: "profile=yunxi_companion_strong".to_string(),
            memory_summary: "records=2 active=1 pending=1".to_string(),
            relationship_summary: "nodes=2 edges=1 active_edges=1".to_string(),
            scopes: vec![
                ControlScopeSnapshot {
                    scope: ControlScope::Companion,
                    enabled: Some(false),
                    summary: "history_records=0".to_string(),
                    source: ControlSource::CurrentConfig,
                    clear_effect: Some("clears local companion history only".to_string()),
                },
                ControlScopeSnapshot {
                    scope: ControlScope::Relationship,
                    enabled: None,
                    summary: "nodes=2 edges=1".to_string(),
                    source: ControlSource::ReadOnlyHistory,
                    clear_effect: None,
                },
            ],
            recent_change: Some("companion disable completed".to_string()),
        });

        let rendered = render_app(&app, 100, 26);

        assert!(rendered.contains("Companion UX & Controls"));
        assert!(rendered.contains("Local companion: OFF"));
        assert!(rendered.contains("Cloud control: OFF"));
        assert!(rendered.contains("clear scope: clears local companion history only"));
        assert!(rendered.contains("relationship"));
        assert!(rendered.contains("read-only"));
    }

    #[test]
    fn composer_cursor_uses_display_width_for_cjk_text() {
        let area = Rect::new(0, 0, 58, 6);
        let buffer = edit_buffer_at("你好abc", "你好abc");
        let position = composer_cursor_position(area, "yunxi> ", &buffer, 0);

        assert_eq!(position.x, 1 + "yunxi> ".len() as u16 + 7);
        assert_eq!(position.y, 1);
    }

    #[test]
    fn composer_cursor_wraps_inside_composer_bounds() {
        let area = Rect::new(0, 0, 24, 6);
        let input = "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz";
        let buffer = edit_buffer_at(input, input);
        let position = composer_cursor_position(area, "yunxi> ", &buffer, 0);

        assert!(position.x < area.width - 1);
        assert!(position.y < area.height - 1);
        assert!(position.y > 1);
    }

    #[test]
    fn composer_cursor_maps_multiline_i18n_url_and_path_input() {
        let area = Rect::new(0, 0, 32, 10);
        let input =
            "中文かな👩‍💻e\u{301}\nhttps://example.test/very/long?query=中文\nC:\\YunXi Agent\\输出";

        for prefix in ["中文", "中文かな👩‍💻e\u{301}", input] {
            let buffer = edit_buffer_at(input, prefix);
            let position = composer_cursor_position(area, "yunxi> ", &buffer, 0);
            assert!(position.x > area.x && position.x < area.x + area.width - 1);
            assert!(position.y > area.y && position.y < area.y + area.height - 1);
        }
        let end_buffer = edit_buffer_at(input, input);
        let end = composer_cursor_position(area, "yunxi> ", &end_buffer, 0);
        assert!(end.y >= 4);
    }

    fn edit_buffer_at(text: &str, prefix: &str) -> EditBuffer {
        let mut buffer = EditBuffer::default();
        buffer.insert_text(text);
        buffer.move_home();
        for _ in prefix.graphemes(true) {
            buffer.move_right();
        }
        buffer
    }

    #[test]
    fn long_multiline_composer_keeps_footer_visible_at_responsive_widths() {
        for (width, height) in [(80, 24), (100, 30), (120, 40), (200, 50)] {
            let mut app = YunxiTuiApp::default();
            app.set_banner(banner());
            app.bottom_pane_mut().paste(
                &(0..40)
                    .map(|index| {
                        format!("第{index:02}行 👩‍💻 e\u{301} long-token-without-breaks-{index}")
                    })
                    .collect::<Vec<_>>()
                    .join("\r\n"),
            );
            let pane_height = app.bottom_pane().desired_height_for_width(width as usize);
            let snapshot = render_full_frame_snapshot(&app, width, height);

            assert_eq!(pane_height, 9, "width={width}");
            assert!(snapshot.contains("Composer"), "width={width}");
            assert!(snapshot.contains("Enter submit"), "width={width}");
            assert!(snapshot.lines().all(|row| {
                UnicodeWidthStr::width(row.split_once('|').unwrap().1) <= width as usize
            }));
        }
    }

    #[test]
    fn user_input_overlay_uses_its_own_title_and_preserves_crlf_lines() {
        let mut app = YunxiTuiApp::default();
        app.set_banner(banner());
        app.start_user_input(crate::bottom_pane::UserInputRequestView {
            id: None,
            prompt: "Required input".to_string(),
        });
        app.bottom_pane_mut().paste("第一行\r\nsecond");

        let snapshot = render_full_frame_snapshot(&app, 80, 24);
        assert!(snapshot.contains("Input"));
        assert!(snapshot.contains("Required input 第一行"));
        assert!(snapshot.contains("second"));
        assert!(!snapshot.contains("┌Composer"));
    }

    #[test]
    fn tiny_and_narrow_terminals_render_without_panicking() {
        let mut app = YunxiTuiApp::default();
        app.set_banner(banner());
        app.push_notice("unicode", "中文 👩‍💻 e\u{301} verylongtokenwithoutbreaks");

        for (width, height) in [(1, 1), (2, 3), (8, 4), (20, 6), (40, 7)] {
            let rendered = render_app(&app, width, height);
            assert!(!rendered.is_empty(), "{width}x{height}");
        }
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
        assert!(rendered.contains("default: Decline"));
        assert!(rendered.contains("Approve"));
        assert!(rendered.contains("Decline"));
        assert!(rendered.contains("Tab select"));
        assert!(rendered.contains("Esc decline"));
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

        assert!(rendered.contains("YunXi v2.1.0"));
        assert!(!rendered.contains("debug off"));
        assert!(!rendered.contains("|,"));
    }

    #[test]
    fn approval_actions_stay_visible_on_58_column_terminal() {
        let rendered = render_app(&approval_app(), 58, 22);

        assert!(rendered.contains("Approval"));
        assert!(rendered.contains("Approve"));
        assert!(rendered.contains("Decline"));
        assert!(rendered.contains("safe default"));
        assert!(rendered.contains("Tab select"));
        assert!(rendered.contains("risk: destructive"));
    }

    #[test]
    fn approval_actions_stay_visible_on_tight_58_column_terminal() {
        let rendered = render_app(&approval_app(), 58, 18);

        assert!(rendered.contains("Approve"));
        assert!(rendered.contains("Decline"));
        assert!(rendered.contains("Tab select"));
    }

    #[test]
    fn approval_actions_stay_visible_on_medium_and_wide_terminals() {
        for (width, height) in [(80, 22), (100, 24)] {
            let rendered = render_app(&approval_app(), width, height);

            assert!(rendered.contains("Approve"));
            assert!(rendered.contains("Decline"));
            assert!(rendered.contains("Tab select"));
            assert!(rendered.contains("risk: destructive"));
        }
    }

    #[test]
    fn approval_risk_and_actions_survive_responsive_width_matrix() {
        let app = approval_app();
        for (width, height) in [(80, 24), (100, 30), (120, 40), (200, 50)] {
            let snapshot = render_full_frame_snapshot(&app, width, height);
            assert!(snapshot.contains("risk: destructive"), "width={width}");
            assert!(snapshot.contains("Remove-Item"), "width={width}");
            assert!(snapshot.contains("Approve"), "width={width}");
            assert!(snapshot.contains("Decline"), "width={width}");
            assert!(snapshot.contains("Tab select"), "width={width}");
            assert!(snapshot.lines().all(|row| {
                UnicodeWidthStr::width(row.split_once('|').unwrap().1) <= width as usize
            }));
        }
    }

    #[test]
    fn monochrome_keeps_approval_error_warning_and_cancel_text_visible() {
        let monochrome = TuiStyleSet::new(crate::styles::TuiColorCapability::Monochrome);
        let approval = render_app_with_styles(&approval_app(), 58, 18, monochrome);
        assert!(approval.contains("Approval required"));
        assert!(approval.contains("default: Decline"));
        assert!(approval.contains("risk: destructive"));

        let mut app = YunxiTuiApp::default();
        app.set_banner(banner());
        app.push_warning("configuration needs attention");
        app.push_error("provider failed");
        app.push_agent_event(&yunxi_agent_core::AgentEvent::Cancelled {
            reason: Some("cancelled by user".to_string()),
        });
        let transcript = render_app_with_styles(&app, 80, 24, monochrome);
        let lowercase = transcript.to_ascii_lowercase();
        assert!(lowercase.contains("warning"));
        assert!(lowercase.contains("error"));
        assert!(lowercase.contains("cancel"));
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
        let wrapped = build_wrapped_transcript(app.transcript().cells(), 98);
        app.jump_top(&wrapped, 8);

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
        let wrapped = build_wrapped_transcript(app.transcript().cells(), 98);
        app.scroll_up(8, &wrapped, 10);
        app.push_notice("event", "fresh-line");

        let rendered = render_app(&app, 100, 18);

        assert!(rendered.contains("new output below"));
        assert!(!rendered.contains("fresh-line"));
    }

    #[test]
    fn transcript_scroll_uses_wrapped_rows_for_title_and_tail() {
        let mut app = YunxiTuiApp::default();
        app.push_agent_event(&yunxi_agent_core::AgentEvent::Message {
            content:
                "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz"
                    .to_string(),
            stream: None,
        });

        let rendered = render_app(&app, 28, 12);

        assert!(rendered.contains("Transcript"));
        assert!(rendered.contains("tail"));
        assert!(rendered.contains("/"));
    }

    #[test]
    fn quiet_transcript_hides_internal_payloads_until_details_are_requested() {
        use yunxi_agent_core::{AgentEvent, CommandStatus};

        let mut app = YunxiTuiApp::default();
        app.set_banner(banner());
        for event in [
            AgentEvent::Reasoning {
                content: "raw private thinking".to_string(),
            },
            AgentEvent::MemoryRecall {
                schema_version: 3,
                enabled: true,
                scope: "global".to_string(),
                query: "private memory query".to_string(),
                count: 1,
                budget_used_chars: 20,
                truncated: false,
                always_on_count: 0,
                dropped_unrelated: 0,
                dropped_by_budget: 0,
                dropped_duplicates: 0,
            },
            AgentEvent::ToolCallStarted {
                id: Some("render-tool".to_string()),
                name: "shell".to_string(),
                arguments_json: Some("{\"token\":\"private-argument\"}".to_string()),
            },
            AgentEvent::ToolCallCompleted {
                id: Some("render-tool".to_string()),
                name: "shell".to_string(),
                output: "complete stdout payload".to_string(),
                status: CommandStatus::Completed,
            },
            AgentEvent::ProviderError {
                provider: "deepseek".to_string(),
                status: Some(500),
                classification: "server_error".to_string(),
                message: "provider wire body\nprivate stack frame".to_string(),
            },
        ] {
            app.push_agent_event(&event);
        }

        let rendered = render_app(&app, 110, 28);
        for forbidden in [
            "raw private thinking",
            "private memory query",
            "private-argument",
            "complete stdout payload",
            "provider wire body",
            "private stack frame",
        ] {
            assert!(
                !rendered.contains(forbidden),
                "default transcript leaked {forbidden}"
            );
        }
        assert!(rendered.contains("shell"));
        assert!(rendered.contains("output captured"));
        assert!(rendered.contains("provider deepseek failed"));

        app.show_details(None);
        let rendered = render_app(&app, 110, 32);
        assert!(rendered.contains("provider wire body"));
        assert!(rendered.contains("private stack frame"));
    }

    #[test]
    fn details_layer_is_bounded_across_responsive_widths() {
        let mut app = YunxiTuiApp::default();
        app.set_banner(banner());
        app.push_agent_event(&yunxi_agent_core::AgentEvent::ProviderError {
            provider: "deepseek".to_string(),
            status: Some(500),
            classification: "server_error".to_string(),
            message: "provider detail line one\nprivate stack line two".to_string(),
        });
        app.show_details(None);

        for width in [80, 100, 120, 200] {
            let rendered = render_full_frame_snapshot(&app, width, 28);
            assert!(rendered.contains("Details"), "width={width}");
            assert!(rendered.contains("Esc close"), "width={width}");
            assert!(
                rendered.contains("provider detail line one"),
                "width={width}"
            );
            assert!(
                rendered.lines().all(|line| {
                    let content = line.split_once('|').expect("snapshot row prefix").1;
                    UnicodeWidthStr::width(content) <= width as usize
                }),
                "details overflow at width={width}"
            );
        }
    }
}

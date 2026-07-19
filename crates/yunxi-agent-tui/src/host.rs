use crate::TuiEvent;
use crate::app::{YunxiTuiApp, YunxiTuiBanner};
use crate::bottom_pane::{
    ApprovalAction, ApprovalDecision, ApprovalRequestView, ComposerAction, UserInputAction,
    UserInputRequestView, UserInputResponse,
};
use crate::frame::{RedrawPriority, RedrawReason, RedrawScheduler};
use crate::layout::{compute_layout, rect_contains};
use crate::render::render_tui_frame;
use crate::scrollbar::{ScrollbarHit, TranscriptScrollbarGeometry};
use crate::transcript_layout::{WrappedTranscript, build_wrapped_transcript};
use anyhow::Result;
use crossterm::cursor::{Hide, Show};
use crossterm::event::{
    DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
    EnableFocusChange, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
    MouseButton, MouseEventKind, poll, read,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Rect, Size};
use std::io::{self, Stdout};
use std::time::{Duration, Instant};
use yunxi_agent_core::{AgentEvent, AgentMessageStreamPhase, ControlSnapshot};

pub struct YunxiTui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    app: YunxiTuiApp,
    frame: RedrawScheduler,
    scroll_drag: Option<TranscriptScrollDrag>,
    _guard: TerminalGuard,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TuiTickAction {
    None,
    CancelCurrentTurn,
}

impl YunxiTui {
    pub fn enter() -> Result<Self> {
        let guard = TerminalGuard::enter()?;
        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;
        Ok(Self {
            terminal,
            app: YunxiTuiApp::default(),
            frame: RedrawScheduler::default(),
            scroll_drag: None,
            _guard: guard,
        })
    }

    pub fn set_banner(&mut self, banner: YunxiTuiBanner) -> Result<()> {
        self.app.set_banner(banner);
        self.request_redraw(RedrawReason::StatusChanged)
    }

    pub fn clear_transcript(&mut self) -> Result<()> {
        self.app.clear_transcript();
        self.request_redraw(RedrawReason::InputChanged)
    }

    pub fn push_agent_event(&mut self, event: &AgentEvent) -> Result<()> {
        let reason = redraw_reason_for_agent_event(event);
        self.app.push_agent_event(event);
        self.request_redraw(reason)
    }

    pub fn push_tui_event(&mut self, event: TuiEvent) -> Result<()> {
        let reason = redraw_reason_for_tui_event(&event);
        self.app.push_tui_event(event);
        self.request_redraw(reason)
    }

    pub fn push_notice(&mut self, kind: &str, message: &str) -> Result<()> {
        self.app.push_notice(kind, message);
        self.request_redraw(RedrawReason::StatusChanged)
    }

    pub fn push_warning(&mut self, message: &str) -> Result<()> {
        self.app.push_warning(message);
        self.request_redraw(RedrawReason::Error)
    }

    pub fn push_error(&mut self, message: &str) -> Result<()> {
        self.app.push_error(message);
        self.request_redraw(RedrawReason::Error)
    }

    pub fn set_debug_events(&mut self, enabled: bool) -> Result<()> {
        self.app.set_debug_events(enabled);
        self.request_redraw(RedrawReason::ControlChanged)
    }

    pub fn show_details(&mut self, id: Option<usize>) -> Result<()> {
        self.app.show_details(id);
        self.request_redraw(RedrawReason::ControlChanged)
    }

    pub fn show_control_snapshot(&mut self, snapshot: ControlSnapshot) -> Result<()> {
        self.app.show_control_snapshot(snapshot);
        self.request_redraw(RedrawReason::ControlChanged)
    }

    pub fn tick(&mut self) -> Result<TuiTickAction> {
        let action = self.drain_turn_events()?;
        self.flush_frame(Instant::now())?;
        Ok(action)
    }

    pub fn flush(&mut self) -> Result<()> {
        self.request_draw_now()
    }

    pub fn read_prompt(&mut self, prompt: &str) -> Result<Option<String>> {
        self.app.start_prompt(prompt);
        self.request_draw_now()?;
        loop {
            let event = read()?;
            if self.handle_navigation_event(&event)? {
                self.request_draw_now()?;
                continue;
            }
            match event {
                Event::Key(key) if is_ctrl_d(key) => return Ok(None),
                Event::Key(key) => match self.app.bottom_pane_mut().handle_composer_key(key) {
                    ComposerAction::None => {}
                    ComposerAction::Cancel => return Ok(None),
                    ComposerAction::Submit(value) => {
                        if !value.trim().is_empty() {
                            self.app.push_user(value.clone());
                        }
                        self.request_draw_now()?;
                        return Ok(Some(value));
                    }
                },
                Event::Paste(value) => self.app.bottom_pane_mut().paste(&value),
                _ => {}
            }
            self.request_draw_now()?;
        }
    }

    pub fn request_approval(&mut self, request: ApprovalRequestView) -> Result<ApprovalDecision> {
        self.app.start_approval(request);
        self.request_draw_now()?;
        loop {
            let event = read()?;
            if self.handle_navigation_event(&event)? {
                self.request_draw_now()?;
                continue;
            }
            match event {
                Event::Key(key) => match self.app.bottom_pane_mut().handle_approval_key(key) {
                    ApprovalAction::None => {}
                    ApprovalAction::Decide(decision) => {
                        self.app.push_notice(
                            "approval",
                            if decision.approved {
                                "approved"
                            } else {
                                "declined"
                            },
                        );
                        self.app.start_prompt("yunxi> ");
                        self.request_draw_now()?;
                        return Ok(decision);
                    }
                },
                _ => {}
            }
            self.request_draw_now()?;
        }
    }

    pub fn request_user_input(
        &mut self,
        request: UserInputRequestView,
    ) -> Result<UserInputResponse> {
        self.app.start_user_input(request);
        self.request_draw_now()?;
        loop {
            let event = read()?;
            if self.handle_navigation_event(&event)? {
                self.request_draw_now()?;
                continue;
            }
            match event {
                Event::Key(key) => match self.app.bottom_pane_mut().handle_user_input_key(key) {
                    UserInputAction::None => {}
                    UserInputAction::Cancel => {
                        self.app.start_prompt("yunxi> ");
                        self.request_draw_now()?;
                        return Ok(UserInputResponse { value: None });
                    }
                    UserInputAction::Submit(response) => {
                        self.app.start_prompt("yunxi> ");
                        self.request_draw_now()?;
                        return Ok(response);
                    }
                },
                Event::Paste(value) => self.app.bottom_pane_mut().paste(&value),
                _ => {}
            }
            self.request_draw_now()?;
        }
    }

    fn request_draw_now(&mut self) -> Result<()> {
        self.request_redraw(RedrawReason::InputChanged)
    }

    fn request_redraw(&mut self, reason: RedrawReason) -> Result<()> {
        let priority = self.frame.request(reason);
        if priority == RedrawPriority::Immediate {
            self.flush_frame(Instant::now())?;
        }
        Ok(())
    }

    fn flush_frame(&mut self, now: Instant) -> Result<()> {
        if self.frame.should_draw(now) {
            self.draw(now)?;
        }
        Ok(())
    }

    fn draw(&mut self, now: Instant) -> Result<()> {
        self.terminal
            .draw(|frame| render_tui_frame(frame, &self.app))
            .map(|_| ())?;
        self.frame.record_draw(now);
        Ok(())
    }

    fn drain_turn_events(&mut self) -> Result<TuiTickAction> {
        let mut redraw_reason = None;
        let mut action = TuiTickAction::None;
        while poll(Duration::ZERO)? {
            let event = read()?;
            if is_ctrl_c_event(&event) {
                action = TuiTickAction::CancelCurrentTurn;
                redraw_reason = Some(RedrawReason::CancelCurrentTurn);
                continue;
            }
            if self.handle_navigation_event(&event)? {
                redraw_reason = Some(if matches!(event, Event::Resize(_, _)) {
                    RedrawReason::Resize
                } else {
                    RedrawReason::ScrollChanged
                });
            }
        }
        if let Some(reason) = redraw_reason {
            self.frame.request(reason);
        }
        Ok(action)
    }

    fn handle_navigation_event(&mut self, event: &Event) -> Result<bool> {
        let metrics = self.transcript_metrics()?;
        match event {
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollUp => {
                    if rect_contains(metrics.layout.transcript, mouse.column, mouse.row) {
                        self.app
                            .scroll_up(3, &metrics.wrapped, metrics.visible_height);
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                }
                MouseEventKind::ScrollDown => {
                    if rect_contains(metrics.layout.transcript, mouse.column, mouse.row) {
                        self.app
                            .scroll_down(3, &metrics.wrapped, metrics.visible_height);
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    self.handle_scrollbar_down(&metrics, mouse.column, mouse.row)
                }
                MouseEventKind::Drag(MouseButton::Left) => {
                    self.handle_scrollbar_drag(&metrics, mouse.row)
                }
                MouseEventKind::Up(MouseButton::Left) => {
                    let was_dragging = self.scroll_drag.take().is_some();
                    Ok(was_dragging)
                }
                _ => Ok(false),
            },
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::PageUp => {
                    self.app.page_up(&metrics.wrapped, metrics.visible_height);
                    Ok(true)
                }
                KeyCode::PageDown => {
                    self.app.page_down(&metrics.wrapped, metrics.visible_height);
                    Ok(true)
                }
                KeyCode::Home => {
                    self.app.jump_top(&metrics.wrapped, metrics.visible_height);
                    Ok(true)
                }
                KeyCode::End => {
                    self.app.follow_tail();
                    Ok(true)
                }
                _ => Ok(false),
            },
            Event::Resize(_, _) => {
                self.scroll_drag = None;
                self.app
                    .reanchor_viewport(&metrics.wrapped, metrics.visible_height);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn handle_scrollbar_down(
        &mut self,
        metrics: &TranscriptMetrics,
        x: u16,
        y: u16,
    ) -> Result<bool> {
        let Some(scrollbar) = metrics.scrollbar else {
            return Ok(false);
        };
        match scrollbar.hit_test(x, y) {
            ScrollbarHit::Thumb { grab_offset } => {
                self.scroll_drag = Some(TranscriptScrollDrag { grab_offset });
                Ok(true)
            }
            ScrollbarHit::PageUp => {
                self.app.page_up(&metrics.wrapped, metrics.visible_height);
                Ok(true)
            }
            ScrollbarHit::PageDown => {
                self.app.page_down(&metrics.wrapped, metrics.visible_height);
                Ok(true)
            }
            ScrollbarHit::Outside => Ok(false),
        }
    }

    fn handle_scrollbar_drag(&mut self, metrics: &TranscriptMetrics, y: u16) -> Result<bool> {
        let Some(drag) = self.scroll_drag else {
            return Ok(false);
        };
        let Some(scrollbar) = metrics.scrollbar else {
            self.scroll_drag = None;
            return Ok(false);
        };
        let start = scrollbar.start_for_drag_y(y, drag.grab_offset);
        let max_start = crate::viewport::max_start(metrics.content_height, metrics.visible_height);
        self.app
            .set_scroll_fraction(start, max_start, &metrics.wrapped, metrics.visible_height);
        Ok(true)
    }

    fn transcript_metrics(&self) -> Result<TranscriptMetrics> {
        let size = self.terminal.size()?;
        Ok(transcript_metrics_for_size(
            size,
            self.app.bottom_pane().desired_height(),
            &self.app,
        ))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TranscriptScrollDrag {
    grab_offset: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TranscriptMetrics {
    layout: crate::layout::TuiLayout,
    visible_height: usize,
    content_height: usize,
    start: usize,
    scrollbar: Option<TranscriptScrollbarGeometry>,
    wrapped: WrappedTranscript,
}

fn transcript_metrics_for_size(
    size: Size,
    bottom_pane_height: u16,
    app: &YunxiTuiApp,
) -> TranscriptMetrics {
    let layout = compute_layout(Rect::new(0, 0, size.width, size.height), bottom_pane_height);
    let visible_height = layout.transcript_inner.height.max(1) as usize;
    let wrapped = build_wrapped_transcript(
        app.transcript().cells(),
        layout.transcript_inner.width as usize,
    );
    let content_height = wrapped.rows.len();
    let start = app.viewport().view_start(&wrapped, visible_height);
    let scrollbar = TranscriptScrollbarGeometry::new(
        layout.transcript_scrollbar,
        content_height,
        visible_height,
        start,
    );

    TranscriptMetrics {
        layout,
        visible_height,
        content_height,
        start,
        scrollbar,
        wrapped,
    }
}

fn redraw_reason_for_agent_event(event: &AgentEvent) -> RedrawReason {
    match event {
        AgentEvent::Message {
            stream: Some(stream),
            ..
        } => match stream.phase {
            AgentMessageStreamPhase::Started | AgentMessageStreamPhase::Delta => {
                RedrawReason::StreamDelta
            }
            AgentMessageStreamPhase::Final => RedrawReason::StreamFinalized,
        },
        AgentEvent::Completed { .. } | AgentEvent::Cancelled { .. } => {
            RedrawReason::StreamFinalized
        }
        AgentEvent::ProviderError { .. } | AgentEvent::Error { .. } => RedrawReason::Error,
        _ => RedrawReason::StatusChanged,
    }
}

fn redraw_reason_for_tui_event(event: &TuiEvent) -> RedrawReason {
    if event.kind == crate::presentation::TuiCellKind::ErrorSummary {
        return RedrawReason::Error;
    }
    match event.stream.as_ref().map(|stream| stream.identity.phase) {
        Some(crate::presentation::TuiStreamPhase::Started)
        | Some(crate::presentation::TuiStreamPhase::Delta)
        | Some(crate::presentation::TuiStreamPhase::Retry) => RedrawReason::StreamDelta,
        Some(crate::presentation::TuiStreamPhase::Final)
        | Some(crate::presentation::TuiStreamPhase::Finish)
        | Some(crate::presentation::TuiStreamPhase::Cancel) => RedrawReason::StreamFinalized,
        None => RedrawReason::StatusChanged,
    }
}

fn is_ctrl_d(key: KeyEvent) -> bool {
    key.kind == KeyEventKind::Press
        && key.code == KeyCode::Char('d')
        && key.modifiers.contains(KeyModifiers::CONTROL)
}

fn is_ctrl_c_event(event: &Event) -> bool {
    matches!(
        event,
        Event::Key(key)
            if key.kind == KeyEventKind::Press
                && key.code == KeyCode::Char('c')
                && key.modifiers.contains(KeyModifiers::CONTROL)
    )
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            EnableBracketedPaste,
            EnableFocusChange,
            EnableMouseCapture,
            Hide
        )?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(
            io::stdout(),
            Show,
            DisableMouseCapture,
            DisableFocusChange,
            DisableBracketedPaste,
            LeaveAlternateScreen
        );
        let _ = disable_raw_mode();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transcript_visible_height_matches_tui_layout() {
        let app = YunxiTuiApp::default();
        assert_eq!(
            transcript_metrics_for_size(
                Size {
                    width: 100,
                    height: 18,
                },
                3,
                &app,
            )
            .visible_height,
            10
        );
        assert_eq!(
            transcript_metrics_for_size(
                Size {
                    width: 100,
                    height: 6,
                },
                3,
                &app,
            )
            .visible_height,
            1
        );
    }

    #[test]
    fn raw_mode_ctrl_c_is_classified_as_turn_cancellation() {
        let event = Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

        assert!(is_ctrl_c_event(&event));
        assert!(!is_ctrl_c_event(&Event::Key(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::NONE,
        ))));
    }

    #[test]
    fn transcript_metrics_uses_wrapped_content_height() {
        let mut app = YunxiTuiApp::default();
        app.push_agent_event(&yunxi_agent_core::AgentEvent::Message {
            content: "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz".to_string(),
            stream: None,
        });

        let metrics = transcript_metrics_for_size(
            Size {
                width: 24,
                height: 18,
            },
            3,
            &app,
        );

        assert!(metrics.content_height > app.transcript().render_line_count());
        assert!(metrics.scrollbar.is_none() || metrics.content_height > metrics.visible_height);
    }
}

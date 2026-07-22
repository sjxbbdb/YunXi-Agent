use crate::bottom_pane::{ApprovalRequestView, BottomPane, BottomPaneMode, UserInputRequestView};
use crate::chat::Transcript;
use crate::input_map::FocusTarget;
use crate::presentation::{TuiEvent, TuiPresentation};
use crate::text_layout::{ClipPriority, PrioritySegment, TextLayout};
use crate::timeline_store::TimelineStore;
use crate::transcript_layout::WrappedTranscript;
use crate::viewport::TranscriptViewport;
use yunxi_agent_core::{AgentEvent, ControlSnapshot};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct YunxiTuiBanner {
    pub cwd: String,
    pub backend: String,
    pub provider_live: bool,
    pub provider_source: String,
    pub model: String,
    pub provider: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct YunxiTuiApp {
    version: String,
    banner: Option<YunxiTuiBanner>,
    presentation: TuiPresentation,
    timeline: TimelineStore,
    transcript: Transcript,
    viewport: TranscriptViewport,
    bottom_pane: BottomPane,
    control_snapshot: Option<ControlSnapshot>,
    details: Option<String>,
    details_scroll: u16,
    focus: FocusTarget,
    previous_focus: FocusTarget,
}

impl Default for YunxiTuiApp {
    fn default() -> Self {
        Self {
            version: format!("v{}", env!("CARGO_PKG_VERSION")),
            banner: None,
            presentation: TuiPresentation::default(),
            timeline: TimelineStore::default(),
            transcript: Transcript::default(),
            viewport: TranscriptViewport::default(),
            bottom_pane: BottomPane::default(),
            control_snapshot: None,
            details: None,
            details_scroll: 0,
            focus: FocusTarget::Composer,
            previous_focus: FocusTarget::Composer,
        }
    }
}

impl YunxiTuiApp {
    #[cfg(test)]
    pub(crate) fn set_version_for_snapshot(&mut self, version: &str) {
        self.version = version.to_string();
    }

    pub(crate) fn set_banner(&mut self, banner: YunxiTuiBanner) {
        self.presentation.set_offline_label(!banner.provider_live);
        self.banner = Some(banner);
    }

    pub(crate) fn transcript(&self) -> &Transcript {
        &self.transcript
    }

    pub(crate) fn viewport(&self) -> &TranscriptViewport {
        &self.viewport
    }

    pub(crate) fn bottom_pane(&self) -> &BottomPane {
        &self.bottom_pane
    }

    pub(crate) fn bottom_pane_mut(&mut self) -> &mut BottomPane {
        &mut self.bottom_pane
    }

    pub(crate) fn control_snapshot(&self) -> Option<&ControlSnapshot> {
        self.control_snapshot.as_ref()
    }

    pub(crate) fn details(&self) -> Option<&str> {
        self.details.as_deref()
    }

    pub(crate) fn focus_target(&self) -> FocusTarget {
        self.focus
    }

    pub(crate) fn focus_next(&mut self) {
        self.focus = match self.focus {
            FocusTarget::Composer => FocusTarget::History,
            FocusTarget::History => FocusTarget::Composer,
            other => other,
        };
    }

    pub(crate) fn focus_previous(&mut self) {
        self.focus_next();
    }

    pub(crate) fn detail_scroll_up(&mut self, lines: u16) {
        self.details_scroll = self.details_scroll.saturating_sub(lines.max(1));
    }

    pub(crate) fn detail_scroll_down(&mut self, lines: u16) {
        self.details_scroll = self.details_scroll.saturating_add(lines.max(1));
    }

    pub(crate) fn details_scroll(&self) -> u16 {
        self.details_scroll
    }

    pub(crate) fn show_control_snapshot(&mut self, snapshot: ControlSnapshot) {
        self.previous_focus = self.focus;
        self.details = None;
        self.details_scroll = 0;
        self.control_snapshot = Some(snapshot);
        self.focus = FocusTarget::Details;
    }

    pub(crate) fn show_transcript(&mut self) {
        self.control_snapshot = None;
        self.details = None;
        self.focus = match self.bottom_pane.mode() {
            BottomPaneMode::Approval { .. } => FocusTarget::Approval,
            BottomPaneMode::UserInput { .. } | BottomPaneMode::Composer => FocusTarget::Composer,
        };
    }

    pub(crate) fn close_details(&mut self) {
        self.details = None;
        self.control_snapshot = None;
        self.details_scroll = 0;
        self.focus = self.previous_focus;
    }

    pub(crate) fn footer_for_width(&self, width: usize) -> String {
        if self.focus == FocusTarget::Details {
            return TextLayout::priority_line(
                &[
                    PrioritySegment::new("Esc close details", ClipPriority::MustKeep),
                    PrioritySegment::new("wheel/PgUp/PgDown scroll", ClipPriority::Important),
                ],
                width,
            );
        }
        if self.focus == FocusTarget::History {
            return TextLayout::priority_line(
                &[
                    PrioritySegment::new("Tab composer", ClipPriority::MustKeep),
                    PrioritySegment::new("wheel/drag/PgUp/PgDown scroll", ClipPriority::Important),
                ],
                width,
            );
        }
        if self.focus == FocusTarget::Approval {
            return TextLayout::priority_line(
                &[
                    PrioritySegment::new("Tab select", ClipPriority::MustKeep),
                    PrioritySegment::new("Enter confirm", ClipPriority::MustKeep),
                    PrioritySegment::new("Esc decline", ClipPriority::Important),
                    PrioritySegment::new("Ctrl+C cancel", ClipPriority::Optional),
                ],
                width,
            );
        }
        match self.viewport.scroll_status() {
            "new output below" => TextLayout::priority_line(
                &[
                    PrioritySegment::new("End follow tail", ClipPriority::MustKeep),
                    PrioritySegment::new("new output below", ClipPriority::Important),
                    PrioritySegment::new("wheel/drag history", ClipPriority::Optional),
                ],
                width,
            ),
            "history" => TextLayout::priority_line(
                &[
                    PrioritySegment::new("End follow tail", ClipPriority::MustKeep),
                    PrioritySegment::new("history view", ClipPriority::Important),
                    PrioritySegment::new("wheel/drag PgUp/PgDown", ClipPriority::Optional),
                ],
                width,
            ),
            _ if self.timeline.has_active_sessions() => TextLayout::priority_line(
                &[
                    PrioritySegment::new("Ctrl+C cancel", ClipPriority::MustKeep),
                    PrioritySegment::new("typing saves draft", ClipPriority::Important),
                    PrioritySegment::new("Enter waits for turn", ClipPriority::Optional),
                    PrioritySegment::new("history scroll available", ClipPriority::DebugOnly),
                ],
                width,
            ),
            _ => TextLayout::priority_line(
                &[
                    PrioritySegment::new("Enter submit", ClipPriority::MustKeep),
                    PrioritySegment::new("Ctrl+C exit", ClipPriority::Important),
                    PrioritySegment::new("/help commands", ClipPriority::Optional),
                    PrioritySegment::new("Alt+Enter newline", ClipPriority::Optional),
                    PrioritySegment::new("wheel/drag scroll", ClipPriority::DebugOnly),
                ],
                width,
            ),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn footer(&self) -> String {
        self.footer_for_width(usize::MAX)
    }

    pub(crate) fn header_for_width(&self, width: usize) -> String {
        match &self.banner {
            Some(banner) => {
                let mode = mode_label(banner.provider_live);
                let product = if width < 90 {
                    format!("YunXi {}", self.version)
                } else {
                    format!("YunXi Agent {}", self.version)
                };
                let provider = format!("{} {}", banner.provider, mode);

                // Header tiers are semantic: lower tiers never receive hidden context.
                if width < 90 {
                    return TextLayout::priority_line(
                        &[
                            PrioritySegment::new(&product, ClipPriority::MustKeep),
                            PrioritySegment::new(&provider, ClipPriority::Important),
                        ],
                        width,
                    );
                }

                let model = model_label(&banner.model, if width < 100 { 24 } else { 44 });
                if width < 120 {
                    return TextLayout::priority_line(
                        &[
                            PrioritySegment::new(&product, ClipPriority::MustKeep),
                            PrioritySegment::new(&provider, ClipPriority::Important),
                            PrioritySegment::new(&model, ClipPriority::Optional),
                        ],
                        width,
                    );
                }

                let separator_width = TextLayout::measure(" | ");
                let fixed_width = TextLayout::measure(&product)
                    .saturating_add(TextLayout::measure(&provider))
                    .saturating_add(separator_width.saturating_mul(3));
                let detail_width = width.saturating_sub(fixed_width);
                let model_width = detail_width.saturating_sub(28).clamp(14, 44);
                let model = model_label(&banner.model, model_width);
                let cwd_width = detail_width
                    .saturating_sub(TextLayout::measure(&model))
                    .clamp(14, 72);
                let cwd = compact_path(&banner.cwd, cwd_width);
                TextLayout::priority_line(
                    &[
                        PrioritySegment::new(&product, ClipPriority::MustKeep),
                        PrioritySegment::new(&provider, ClipPriority::Important),
                        PrioritySegment::new(&model, ClipPriority::Optional),
                        PrioritySegment::new(&cwd, ClipPriority::DebugOnly),
                    ],
                    width,
                )
            }
            None => TextLayout::truncate(
                &format!("YunXi Agent {} interactive CLI", self.version),
                width,
            ),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn header(&self) -> String {
        self.header_for_width(usize::MAX)
    }

    pub(crate) fn subheader_for_width(&self, width: usize) -> String {
        match &self.banner {
            Some(banner) => {
                let cells = format!("cells={}", self.transcript.cells().len());
                let view = self.viewport.scroll_status();
                let mut debug = compact_debug_status(&self.transcript.debug_status());
                let duplicate_events = self.timeline.duplicate_event_count();
                if duplicate_events > 0 {
                    debug.push_str(&format!("/stream-dupes={duplicate_events}"));
                }
                if width < 90 {
                    let provider =
                        format!("provider={}", TextLayout::truncate(&banner.provider, 18));
                    return TextLayout::priority_line(
                        &[
                            PrioritySegment::new(view, ClipPriority::MustKeep),
                            PrioritySegment::new(&cells, ClipPriority::Important),
                            PrioritySegment::new(&provider, ClipPriority::Optional),
                        ],
                        width,
                    );
                }
                let source = if width < 90 {
                    banner.provider_source.clone()
                } else {
                    format!("source={}", banner.provider_source)
                };
                let backend = format!("backend={}", banner.backend);
                TextLayout::priority_line(
                    &[
                        PrioritySegment::new(view, ClipPriority::MustKeep),
                        PrioritySegment::new(&cells, ClipPriority::Important),
                        PrioritySegment::new(&backend, ClipPriority::Optional),
                        PrioritySegment::new(&source, ClipPriority::Optional),
                        PrioritySegment::new(&debug, ClipPriority::DebugOnly),
                    ],
                    width,
                )
            }
            None => "initializing".to_string(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn subheader(&self) -> String {
        self.subheader_for_width(usize::MAX)
    }

    pub(crate) fn start_prompt(&mut self, prompt: &str) {
        self.control_snapshot = None;
        self.details = None;
        self.focus = FocusTarget::Composer;
        self.bottom_pane.start_composer(prompt);
    }

    pub(crate) fn prepare_prompt(&mut self, prompt: &str) {
        if self.focus != FocusTarget::Details {
            self.start_prompt(prompt);
        }
    }

    pub(crate) fn start_approval(&mut self, request: ApprovalRequestView) {
        self.bottom_pane.start_approval(request);
        self.focus = FocusTarget::Approval;
    }

    pub(crate) fn start_user_input(&mut self, request: UserInputRequestView) {
        self.bottom_pane.start_user_input(request);
        self.focus = FocusTarget::Composer;
    }

    pub(crate) fn push_user(&mut self, value: impl Into<String>) {
        self.show_transcript();
        let mut changed = false;
        for update in self.timeline.finish_active() {
            changed |= self.transcript.apply_assistant_update(update);
        }
        let event = self.presentation.present_user(value);
        self.push_tui_event(event);
        if changed {
            self.on_transcript_changed();
        }
    }

    pub(crate) fn push_agent_event(&mut self, event: &AgentEvent) {
        let cancelled = matches!(event, AgentEvent::Cancelled { .. });
        let terminal = matches!(
            event,
            AgentEvent::Completed { .. }
                | AgentEvent::Cancelled { .. }
                | AgentEvent::ProviderError { .. }
                | AgentEvent::Error { .. }
        );
        let event = self.presentation.present_agent_event(event);
        self.push_tui_event(event);
        if terminal {
            let mut changed = false;
            let updates = if cancelled {
                self.timeline.cancel_active()
            } else {
                self.timeline.finish_active()
            };
            for update in updates {
                changed |= self.transcript.apply_assistant_update(update);
            }
            if changed {
                self.on_transcript_changed();
            }
        }
    }

    pub(crate) fn push_tui_event(&mut self, mut event: TuiEvent) {
        let has_stream = event.stream.is_some();
        let mut changed = self
            .timeline
            .apply(&mut event)
            .is_some_and(|update| self.transcript.apply_assistant_update(update));
        if event.kind != crate::presentation::TuiCellKind::AssistantMessage || !has_stream {
            let before = self.transcript.cells().len();
            self.transcript.push_tui_event(event);
            changed |= self.transcript.cells().len() != before;
        }
        if changed {
            self.on_transcript_changed();
        }
    }

    pub(crate) fn set_debug_events(&mut self, enabled: bool) {
        self.transcript.set_debug_events(enabled);
        let event = self.presentation.present_notice(
            "debug",
            if enabled {
                "event debug enabled"
            } else {
                "event debug disabled"
            },
        );
        self.push_tui_event(event);
    }

    pub(crate) fn show_details(&mut self, id: Option<usize>) {
        let detail = self.transcript.detail_text(id);
        self.previous_focus = self.focus;
        self.details = Some(detail);
        self.details_scroll = 0;
        self.control_snapshot = None;
        self.focus = FocusTarget::Details;
    }

    pub(crate) fn push_notice(&mut self, kind: &str, message: &str) {
        let event = self.presentation.present_notice(kind, message);
        self.push_tui_event(event);
    }

    pub(crate) fn push_warning(&mut self, message: &str) {
        let event = self.presentation.present_warning(message);
        self.push_tui_event(event);
    }

    pub(crate) fn push_error(&mut self, message: &str) {
        let event = self.presentation.present_error(message);
        self.push_tui_event(event);
    }

    pub(crate) fn clear_transcript(&mut self) {
        self.timeline.clear();
        self.transcript.clear();
        self.viewport.reset();
    }

    pub(crate) fn scroll_up(
        &mut self,
        lines: usize,
        wrapped: &WrappedTranscript,
        visible_height: usize,
    ) {
        self.viewport.scroll_up(lines, wrapped, visible_height);
    }

    pub(crate) fn scroll_down(
        &mut self,
        lines: usize,
        wrapped: &WrappedTranscript,
        visible_height: usize,
    ) {
        self.viewport.scroll_down(
            lines.max(1).min(visible_height.max(1)),
            wrapped,
            visible_height,
        );
    }

    pub(crate) fn page_up(&mut self, wrapped: &WrappedTranscript, visible_height: usize) {
        self.viewport.page_up(wrapped, visible_height);
    }

    pub(crate) fn page_down(&mut self, wrapped: &WrappedTranscript, visible_height: usize) {
        self.viewport.page_down(wrapped, visible_height);
    }

    pub(crate) fn jump_top(&mut self, wrapped: &WrappedTranscript, visible_height: usize) {
        self.viewport.jump_top(wrapped, visible_height);
    }

    pub(crate) fn follow_tail(&mut self) {
        self.viewport.follow_tail();
    }

    pub(crate) fn set_scroll_fraction(
        &mut self,
        numerator: usize,
        denominator: usize,
        wrapped: &WrappedTranscript,
        visible_height: usize,
    ) {
        self.viewport
            .set_scroll_fraction(numerator, denominator, wrapped, visible_height);
    }

    pub(crate) fn reanchor_viewport(&mut self, wrapped: &WrappedTranscript, visible_height: usize) {
        self.viewport.reanchor(wrapped, visible_height);
    }

    fn on_transcript_changed(&mut self) {
        self.viewport.on_content_changed();
    }
}

fn mode_label(provider_live: bool) -> &'static str {
    if provider_live { "live" } else { "offline" }
}

fn model_label(model: &str, width: usize) -> String {
    let prefix = "model=";
    let value_width = width.saturating_sub(TextLayout::measure(prefix)).max(8);
    format!("{prefix}{}", TextLayout::truncate(model, value_width))
}

fn compact_debug_status(status: &str) -> String {
    if status.contains("debug=on") {
        "debug on".to_string()
    } else {
        "debug off".to_string()
    }
}

fn compact_path(path: &str, width: usize) -> String {
    if TextLayout::measure(path) <= width {
        return path.to_string();
    }
    let normalized = path.replace('\\', "/");
    let tail = normalized
        .rsplit('/')
        .find(|part| !part.is_empty())
        .unwrap_or(normalized.as_str());
    let prefix = if normalized.contains(':') {
        normalized.split('/').next().unwrap_or("...")
    } else {
        "..."
    };
    let compact = format!("{prefix}/.../{tail}");
    TextLayout::truncate(&compact, width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::HistoryCellKind;
    use yunxi_agent_core::{
        AgentMessageSequence, AgentMessageStream, AgentMessageStreamPhase, AgentRunStatus,
        TokenUsage,
    };

    fn banner() -> YunxiTuiBanner {
        YunxiTuiBanner {
            cwd: "D:/YunXi Agent/crates/yunxi-agent-cli".to_string(),
            backend: "yunxi".to_string(),
            provider_live: false,
            provider_source: "offline_static".to_string(),
            model: "deepseek-chat".to_string(),
            provider: "static".to_string(),
        }
    }

    fn assistant_event(content: &str, sequence: u64, phase: AgentMessageStreamPhase) -> AgentEvent {
        assistant_event_for("turn-live", "message-live", content, sequence, phase)
    }

    fn assistant_event_for(
        turn_id: &str,
        stream_id: &str,
        content: &str,
        sequence: u64,
        phase: AgentMessageStreamPhase,
    ) -> AgentEvent {
        AgentEvent::Message {
            content: content.to_string(),
            stream: Some(AgentMessageStream {
                thread_id: "thread-live".to_string(),
                turn_id: turn_id.to_string(),
                stream_id: stream_id.to_string(),
                event_id: format!("fallback:app-test:{turn_id}:{stream_id}:{sequence}"),
                source_sequence: AgentMessageSequence::LocalFallback(sequence),
                phase,
            }),
        }
    }

    #[test]
    fn narrow_header_keeps_complete_status_tokens() {
        let mut app = YunxiTuiApp::default();
        app.set_banner(banner());

        let header = app.header_for_width(80);
        let subheader = app.subheader_for_width(80);
        let footer = app.footer_for_width(80);

        assert!(TextLayout::measure(&header) <= 80);
        assert!(TextLayout::measure(&subheader) <= 80);
        assert!(TextLayout::measure(&footer) <= 80);
        assert!(header.contains("YunXi v2.1.2"));
        assert!(header.contains("offline"));
        assert!(!header.contains("model="));
        assert!(!header.contains("D:/"));
        assert!(!header.contains("yunxi-agent-cli"));
        assert!(!header.contains(".../"));
        assert!(subheader.contains("tail"));
        assert!(!subheader.ends_with('|'));
        assert!(!subheader.ends_with("| d"));
        assert!(footer.contains("Enter submit"));
        assert!(footer.contains("Ctrl+C exit"));
    }

    #[test]
    fn wide_header_preserves_provider_and_model_details() {
        let mut app = YunxiTuiApp::default();
        app.set_banner(banner());

        let header = app.header_for_width(120);
        let subheader = app.subheader_for_width(120);

        assert!(header.contains("YunXi Agent v2.1.2"));
        assert!(header.contains("model=deepseek-chat"));
        assert!(header.contains("D:/"));
        assert!(header.contains("yunxi-agent-cli"));
        assert!(subheader.contains("backend=yunxi"));
        assert!(subheader.contains("source=offline_static"));
    }

    #[test]
    fn medium_header_keeps_model_before_cwd() {
        let mut app = YunxiTuiApp::default();
        app.set_banner(YunxiTuiBanner {
            model: "deepseek-chat-ultra-long-model-name".to_string(),
            provider_live: true,
            provider: "deepseek".to_string(),
            ..banner()
        });

        let header = app.header_for_width(100);

        assert!(TextLayout::measure(&header) <= 100);
        assert!(header.contains("deepseek live"));
        assert!(header.contains("model=deepseek-chat"));
        assert!(!header.contains("D:/"));
        assert!(!header.contains("yunxi-agent-cli"));
        assert!(!header.contains(".../"));
    }

    #[test]
    fn responsive_status_priority_is_stable_across_width_matrix() {
        let mut app = YunxiTuiApp::default();
        app.set_banner(YunxiTuiBanner {
            cwd: "C:\\Users\\24763\\YunXi Agent\\包含空格\\very-long-workspace".to_string(),
            model: "deepseek-chat-ultra-long-model-name".to_string(),
            provider_live: true,
            provider: "deepseek".to_string(),
            ..banner()
        });

        for width in [58, 80, 100, 120, 200] {
            let header = app.header_for_width(width);
            let subheader = app.subheader_for_width(width);
            let footer = app.footer_for_width(width);
            assert!(TextLayout::measure(&header) <= width, "width={width}");
            assert!(TextLayout::measure(&subheader) <= width, "width={width}");
            assert!(TextLayout::measure(&footer) <= width, "width={width}");
            assert!(header.contains("v2.1.2"), "width={width}");
            assert!(header.contains("deepseek live"), "width={width}");
            assert!(subheader.contains("tail"), "width={width}");
            assert!(footer.contains("Enter submit"), "width={width}");
        }
        let narrow = app.header_for_width(80);
        assert!(!narrow.contains("model="));
        assert!(!narrow.contains("C:"));
        assert!(!narrow.contains("very-long-workspace"));
        assert!(!narrow.contains(".../"));

        let tight_subheader = app.subheader_for_width(58);
        assert!(tight_subheader.contains("tail"));
        assert!(tight_subheader.contains("cells="));
        assert!(!tight_subheader.contains("backend="));
        assert!(!tight_subheader.contains("source="));
        assert!(!tight_subheader.contains("debug"));

        let medium = app.header_for_width(100);
        assert!(medium.contains("model=deepseek-chat"));
        assert!(!medium.contains("C:"));
        assert!(!medium.contains("very-long-workspace"));
        assert!(!medium.contains(".../"));

        for width in [120, 200] {
            let wide = app.header_for_width(width);
            assert!(wide.contains("model=deepseek-chat"), "width={width}");
            assert!(wide.contains("very-long-workspace"), "width={width}");
        }
    }

    #[test]
    fn provider_shaped_delta_and_final_leave_one_canonical_assistant_cell() {
        let mut app = YunxiTuiApp::default();
        app.push_user("你好");
        app.push_agent_event(&assistant_event("你", 1, AgentMessageStreamPhase::Delta));
        app.push_agent_event(&assistant_event(
            "你好，我在。",
            2,
            AgentMessageStreamPhase::Final,
        ));
        app.push_agent_event(&AgentEvent::Completed {
            status: AgentRunStatus::Completed,
            usage: Some(TokenUsage {
                input_tokens: 1,
                cached_input_tokens: 0,
                output_tokens: 4,
                reasoning_output_tokens: 0,
            }),
        });

        let assistant_cells = app
            .transcript()
            .cells()
            .iter()
            .filter_map(|cell| match cell.kind() {
                HistoryCellKind::Assistant { content, active } => Some((content, active)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(assistant_cells, vec![(&"你好，我在。".to_string(), &false)]);
        assert_eq!(app.transcript().cells().len(), 2);
    }

    #[test]
    fn final_update_preserves_history_scroll_position() {
        let mut app = YunxiTuiApp::default();
        for index in 0..30 {
            app.push_notice("history", &format!("history line {index}"));
        }
        app.push_user("history test");
        app.push_agent_event(&assistant_event(
            "partial",
            1,
            AgentMessageStreamPhase::Delta,
        ));
        let wrapped_before =
            crate::transcript_layout::build_wrapped_transcript(app.transcript().cells(), 80);
        app.scroll_up(4, &wrapped_before, 10);
        let anchor_before = wrapped_before
            .anchor_at(app.viewport().view_start(&wrapped_before, 10))
            .cloned();

        app.push_agent_event(&assistant_event(
            "final answer",
            2,
            AgentMessageStreamPhase::Final,
        ));

        let wrapped_after =
            crate::transcript_layout::build_wrapped_transcript(app.transcript().cells(), 80);
        let anchor_after = wrapped_after
            .anchor_at(app.viewport().view_start(&wrapped_after, 10))
            .cloned();
        assert_eq!(anchor_after, anchor_before);
        assert_eq!(app.viewport().scroll_status(), "new output below");
    }

    #[test]
    fn active_stream_footer_reports_history_instead_of_claiming_tail() {
        let mut app = YunxiTuiApp::default();
        for index in 0..30 {
            app.push_notice("history", &format!("history line {index}"));
        }
        app.push_agent_event(&assistant_event(
            "partial",
            1,
            AgentMessageStreamPhase::Delta,
        ));
        let wrapped =
            crate::transcript_layout::build_wrapped_transcript(app.transcript().cells(), 80);
        app.scroll_up(5, &wrapped, 10);

        let footer = app.footer_for_width(100);

        assert!(footer.contains("history view"));
        assert!(!footer.starts_with("streaming current turn"));
    }

    #[test]
    fn focus_footers_only_advertise_available_actions() {
        let mut app = YunxiTuiApp::default();
        let composer = app.footer_for_width(100);
        assert!(composer.contains("Enter submit"));
        assert!(composer.contains("wheel/drag scroll"));

        app.focus_next();
        let history = app.footer_for_width(100);
        assert!(history.contains("wheel/drag/PgUp/PgDown scroll"));

        app.start_approval(ApprovalRequestView {
            id: None,
            tool_name: "shell".to_string(),
            cwd: ".".to_string(),
            command: Some("echo safe".to_string()),
            reason: "test".to_string(),
            risk_label: None,
        });
        let approval = app.footer_for_width(100);
        assert!(approval.contains("Enter confirm"));
        assert!(approval.contains("Esc decline"));
        assert!(!approval.contains("wheel"));

        app.show_details(None);
        let details = app.footer_for_width(100);
        assert!(details.contains("Esc close details"));
        assert!(details.contains("wheel/PgUp/PgDown scroll"));
    }

    #[test]
    fn active_stream_resize_then_cancel_preserves_pinned_cell() {
        let mut app = YunxiTuiApp::default();
        for index in 0..24 {
            app.push_notice(
                "history",
                &format!("history line {index} with wrapped text"),
            );
        }
        app.push_agent_event(&assistant_event(
            "partial 中文 👨‍👩‍👧‍👦",
            1,
            AgentMessageStreamPhase::Delta,
        ));
        let narrow =
            crate::transcript_layout::build_wrapped_transcript(app.transcript().cells(), 28);
        app.scroll_up(8, &narrow, 8);

        let wide = crate::transcript_layout::build_wrapped_transcript(app.transcript().cells(), 90);
        app.reanchor_viewport(&wide, 12);
        let expected_after_resize = wide
            .anchor_at(app.viewport().view_start(&wide, 12))
            .unwrap()
            .cell_id
            .clone();
        app.push_agent_event(&AgentEvent::Cancelled {
            reason: Some("current turn cancelled".to_string()),
        });
        let cancelled =
            crate::transcript_layout::build_wrapped_transcript(app.transcript().cells(), 90);
        let actual = cancelled
            .anchor_at(app.viewport().view_start(&cancelled, 12))
            .unwrap()
            .cell_id
            .clone();

        assert_eq!(actual, expected_after_resize);
        assert_eq!(app.viewport().scroll_status(), "new output below");
        assert!(!app.timeline.has_active_sessions());
    }

    #[test]
    fn cancellation_freezes_partial_assistant_cell_and_accepts_next_input() {
        let mut app = YunxiTuiApp::default();
        app.push_user("first prompt");
        app.push_agent_event(&assistant_event(
            "partial 中文 👨‍👩‍👧‍👦",
            1,
            AgentMessageStreamPhase::Delta,
        ));

        app.push_agent_event(&AgentEvent::Cancelled {
            reason: Some("current turn cancelled".to_string()),
        });
        app.push_user("next prompt");

        assert!(app.transcript().cells().iter().any(|cell| matches!(
            cell.kind(),
            HistoryCellKind::Assistant { content, active }
                if content == "partial 中文 👨‍👩‍👧‍👦" && !active
        )));
        assert!(app.transcript().cells().iter().any(|cell| matches!(
            cell.kind(),
            HistoryCellKind::User(content) if content == "next prompt"
        )));
        assert!(!app.timeline.has_active_sessions());
    }

    #[test]
    fn provider_disconnect_freezes_partial_cell_shows_summary_and_allows_next_turn() {
        let mut app = YunxiTuiApp::default();
        app.push_user("first prompt");
        app.push_agent_event(&assistant_event_for(
            "turn-disconnect",
            "message-disconnect",
            "partial response",
            1,
            AgentMessageStreamPhase::Delta,
        ));
        app.push_agent_event(&AgentEvent::ProviderError {
            provider: "fixture".to_string(),
            status: None,
            classification: "network".to_string(),
            message: "stream disconnected".to_string(),
        });
        app.push_user("next prompt");
        app.push_agent_event(&assistant_event_for(
            "turn-next",
            "message-next",
            "next answer",
            1,
            AgentMessageStreamPhase::Final,
        ));

        assert!(app.transcript().cells().iter().any(|cell| matches!(
            cell.kind(),
            HistoryCellKind::Assistant { content, active }
                if content == "partial response" && !active
        )));
        assert!(app.transcript().cells().iter().any(|cell| matches!(
            cell.kind(),
            HistoryCellKind::Error(content) if content.contains("YX-PROVIDER-001")
        )));
        assert!(app.transcript().cells().iter().any(|cell| matches!(
            cell.kind(),
            HistoryCellKind::Assistant { content, active }
                if content == "next answer" && !active
        )));
        assert!(!app.timeline.has_active_sessions());
    }

    #[test]
    fn history_eviction_keeps_viewport_anchor_within_retained_rows() {
        let mut app = YunxiTuiApp::default();
        for index in 0..900 {
            app.push_notice("history", &format!("retained history line {index}"));
        }
        let wrapped =
            crate::transcript_layout::build_wrapped_transcript(app.transcript().cells(), 58);
        app.scroll_up(200, &wrapped, 18);
        app.push_notice("history", "latest after eviction");
        let updated =
            crate::transcript_layout::build_wrapped_transcript(app.transcript().cells(), 58);
        let start = app.viewport().view_start(&updated, 18);

        assert!(start <= crate::viewport::max_start(updated.rows.len(), 18));
        assert!(updated.anchor_at(start).is_some());
        assert!(app.transcript().cells().len() <= crate::chat::MAX_HISTORY_CELLS);
    }

    #[test]
    fn overlays_and_control_snapshots_preserve_composer_text_and_cursor() {
        let mut app = YunxiTuiApp::default();
        app.bottom_pane_mut().paste("draft 中文👩‍💻");
        app.bottom_pane_mut()
            .handle_composer_key(crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Left,
                crossterm::event::KeyModifiers::NONE,
            ));
        let expected = app.bottom_pane().composer_snapshot();

        app.start_approval(ApprovalRequestView {
            id: Some("approval-1".to_string()),
            tool_name: "shell".to_string(),
            cwd: ".".to_string(),
            command: Some("echo ok".to_string()),
            reason: "test".to_string(),
            risk_label: None,
        });
        app.start_prompt("yunxi> ");
        assert_eq!(app.bottom_pane().composer_snapshot(), expected);

        app.start_user_input(UserInputRequestView {
            id: Some("input-1".to_string()),
            prompt: "details".to_string(),
        });
        app.start_prompt("yunxi> ");
        app.show_control_snapshot(ControlSnapshot {
            companion_enabled: false,
            cloud_control_enabled: false,
            quiet_hours: None,
            persona_summary: String::new(),
            memory_summary: String::new(),
            relationship_summary: String::new(),
            scopes: Vec::new(),
            recent_change: None,
        });
        app.show_transcript();
        assert_eq!(app.bottom_pane().composer_snapshot(), expected);
    }

    #[test]
    fn details_restore_prior_focus_draft_and_approval_selection() {
        let mut app = YunxiTuiApp::default();
        app.bottom_pane_mut().paste("details draft 中文👩‍💻");
        let draft = app.bottom_pane().composer_snapshot();

        app.show_details(None);
        assert_eq!(app.focus_target(), FocusTarget::Details);
        app.detail_scroll_down(4);
        assert_eq!(app.details_scroll(), 4);
        app.close_details();
        assert_eq!(app.focus_target(), FocusTarget::Composer);
        assert_eq!(app.bottom_pane().composer_snapshot(), draft);

        app.start_approval(ApprovalRequestView {
            id: Some("approval-details".to_string()),
            tool_name: "shell".to_string(),
            cwd: ".".to_string(),
            command: Some("echo ok".to_string()),
            reason: "test".to_string(),
            risk_label: None,
        });
        app.bottom_pane_mut()
            .handle_approval_key(crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Tab,
                crossterm::event::KeyModifiers::NONE,
            ));
        let approval = app.bottom_pane().mode().clone();

        app.show_details(None);
        app.close_details();
        assert_eq!(app.focus_target(), FocusTarget::Approval);
        assert_eq!(app.bottom_pane().mode(), &approval);
        app.start_prompt("yunxi> ");
        assert_eq!(app.bottom_pane().composer_snapshot(), draft);
    }

    #[test]
    fn prompt_preparation_keeps_details_open_until_escape_closes_it() {
        let mut app = YunxiTuiApp::default();
        app.bottom_pane_mut().paste("details draft");
        app.show_details(None);
        app.prepare_prompt("yunxi> ");

        assert_eq!(app.focus_target(), FocusTarget::Details);
        assert!(app.details().is_some());
        app.close_details();
        app.prepare_prompt("yunxi> ");
        assert_eq!(app.focus_target(), FocusTarget::Composer);
        assert!(app.details().is_none());
    }

    #[test]
    fn streaming_final_and_completion_do_not_touch_pretyped_draft_or_duplicate_answer() {
        let mut app = YunxiTuiApp::default();
        app.push_user("first prompt");
        app.push_agent_event(&assistant_event(
            "partial",
            1,
            AgentMessageStreamPhase::Delta,
        ));
        app.bottom_pane_mut().paste("next 草稿👩‍💻");
        let expected = app.bottom_pane().composer_snapshot();

        app.push_agent_event(&assistant_event(
            "final answer",
            2,
            AgentMessageStreamPhase::Final,
        ));
        app.push_agent_event(&AgentEvent::Completed {
            status: AgentRunStatus::Completed,
            usage: None,
        });

        let assistant_cells = app
            .transcript()
            .cells()
            .iter()
            .filter(|cell| matches!(cell.kind(), HistoryCellKind::Assistant { .. }))
            .count();
        assert_eq!(assistant_cells, 1);
        assert_eq!(app.bottom_pane().composer_snapshot(), expected);
    }
}

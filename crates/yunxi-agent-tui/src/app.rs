use crate::bottom_pane::{ApprovalRequestView, BottomPane, UserInputRequestView};
use crate::chat::Transcript;
use crate::presentation::{TuiEvent, TuiPresentation};
use crate::timeline_store::TimelineStore;
use crate::viewport::TranscriptViewport;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
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
}

impl Default for YunxiTuiApp {
    fn default() -> Self {
        Self {
            version: "v2.0.2".to_string(),
            banner: None,
            presentation: TuiPresentation::default(),
            timeline: TimelineStore::default(),
            transcript: Transcript::default(),
            viewport: TranscriptViewport::default(),
            bottom_pane: BottomPane::default(),
            control_snapshot: None,
        }
    }
}

impl YunxiTuiApp {
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

    pub(crate) fn show_control_snapshot(&mut self, snapshot: ControlSnapshot) {
        self.control_snapshot = Some(snapshot);
    }

    pub(crate) fn show_transcript(&mut self) {
        self.control_snapshot = None;
    }

    pub(crate) fn footer_for_width(&self, width: usize) -> String {
        match self.viewport.scroll_status() {
            "new output below" => fit_line(
                &[
                    "new output below",
                    "End follow tail",
                    if width < 72 {
                        "wheel history"
                    } else {
                        "wheel/drag history"
                    },
                ],
                width,
            ),
            "history" => fit_line(
                &[
                    "history view",
                    "End follow tail",
                    if width < 72 {
                        "PgUp/PgDown"
                    } else {
                        "wheel/drag PgUp/PgDown"
                    },
                ],
                width,
            ),
            _ if width < 56 => "Enter | /help | Ctrl+C".to_string(),
            _ if width < 86 => "Enter submit | /help | wheel scroll | Ctrl+C exit".to_string(),
            _ => "Enter submit | Alt+Enter newline | /help commands | wheel/drag scroll | Ctrl+C exit"
                .to_string(),
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
                if width < 64 {
                    let model = short_model(&banner.model, banner.provider_live, width / 2);
                    return fit_line(&[&format!("YunXi {}", self.version), mode, &model], width);
                }
                let provider = format!("{} {}", banner.provider, mode);
                let model = model_label(&banner.model, if width < 90 { 28 } else { 36 });
                if width < 90 {
                    return fit_line(
                        &[&format!("YunXi Agent {}", self.version), &provider, &model],
                        width,
                    );
                }
                let cwd = compact_path(&banner.cwd, if width < 110 { 28 } else { 44 });
                fit_line(
                    &[
                        &format!("YunXi Agent {}", self.version),
                        &provider,
                        &model,
                        &cwd,
                    ],
                    width,
                )
            }
            None => truncate_end(
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
                let debug = compact_debug_status(&self.transcript.debug_status());
                if width < 64 {
                    let provider = format!("provider={}", truncate_end(&banner.provider, 18));
                    return fit_line(&[&provider, view, &cells, &debug], width);
                }
                let source = if width < 90 {
                    banner.provider_source.clone()
                } else {
                    format!("source={}", banner.provider_source)
                };
                fit_line(
                    &[
                        &format!("backend={}", banner.backend),
                        &source,
                        &cells,
                        view,
                        &debug,
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
        self.bottom_pane.start_composer(prompt);
    }

    pub(crate) fn start_approval(&mut self, request: ApprovalRequestView) {
        self.bottom_pane.start_approval(request);
    }

    pub(crate) fn start_user_input(&mut self, request: UserInputRequestView) {
        self.bottom_pane.start_user_input(request);
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
            for update in self.timeline.finish_active() {
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
        let event = self.presentation.present_details(detail);
        self.push_tui_event(event);
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

    pub(crate) fn scroll_up(&mut self, lines: usize, content_height: usize, visible_height: usize) {
        self.viewport
            .scroll_up(lines, content_height, visible_height);
    }

    pub(crate) fn scroll_down(&mut self, lines: usize, visible_height: usize) {
        self.viewport
            .scroll_down(lines.max(1).min(visible_height.max(1)));
    }

    pub(crate) fn page_up(&mut self, content_height: usize, visible_height: usize) {
        self.viewport.page_up(content_height, visible_height);
    }

    pub(crate) fn page_down(&mut self, visible_height: usize) {
        self.viewport.page_down(visible_height);
    }

    pub(crate) fn jump_top(&mut self, content_height: usize, visible_height: usize) {
        self.viewport.jump_top(content_height, visible_height);
    }

    pub(crate) fn follow_tail(&mut self) {
        self.viewport.follow_tail();
    }

    pub(crate) fn set_scroll_fraction(
        &mut self,
        numerator: usize,
        denominator: usize,
        content_height: usize,
        visible_height: usize,
    ) {
        self.viewport
            .set_scroll_fraction(numerator, denominator, content_height, visible_height);
    }

    pub(crate) fn clamp_viewport(&mut self, content_height: usize, visible_height: usize) {
        self.viewport.clamp(content_height, visible_height);
    }

    fn on_transcript_changed(&mut self) {
        self.viewport.on_content_changed();
    }
}

fn mode_label(provider_live: bool) -> &'static str {
    if provider_live { "live" } else { "offline" }
}

fn short_model(model: &str, provider_live: bool, width: usize) -> String {
    if provider_live {
        model_label(model, width)
    } else {
        "model=static".to_string()
    }
}

fn model_label(model: &str, width: usize) -> String {
    let prefix = "model=";
    let value_width = width.saturating_sub(display_width(prefix)).max(8);
    format!("{prefix}{}", truncate_end(model, value_width))
}

fn compact_debug_status(status: &str) -> String {
    if status.contains("debug=on") {
        "debug on".to_string()
    } else {
        "debug off".to_string()
    }
}

fn fit_line(parts: &[&str], width: usize) -> String {
    let mut included = Vec::new();
    for part in parts.iter().filter(|part| !part.trim().is_empty()) {
        let candidate = if included.is_empty() {
            (*part).to_string()
        } else {
            format!("{} | {}", included.join(" | "), part)
        };
        if display_width(&candidate) <= width {
            included.push((*part).to_string());
        } else {
            break;
        }
    }

    if included.is_empty() {
        parts
            .first()
            .map(|part| truncate_end(part, width))
            .unwrap_or_default()
    } else {
        truncate_end(&included.join(" | "), width)
    }
}

fn compact_path(path: &str, width: usize) -> String {
    if display_width(path) <= width {
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
    truncate_end(&compact, width)
}

fn truncate_end(value: &str, width: usize) -> String {
    if width == usize::MAX || display_width(value) <= width {
        return value.to_string();
    }
    if width <= 3 {
        return String::new();
    }
    let mut output = String::new();
    let limit = width.saturating_sub(3);
    let mut used = 0usize;
    for ch in value.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used.saturating_add(ch_width) > limit {
            break;
        }
        output.push(ch);
        used = used.saturating_add(ch_width);
    }
    output.push_str("...");
    output
}

fn display_width(value: &str) -> usize {
    UnicodeWidthStr::width(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::HistoryCellKind;
    use yunxi_agent_core::{
        AgentMessageStream, AgentMessageStreamPhase, AgentRunStatus, TokenUsage,
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
        AgentEvent::Message {
            content: content.to_string(),
            stream: Some(AgentMessageStream {
                thread_id: "thread-live".to_string(),
                turn_id: "turn-live".to_string(),
                stream_id: "message-live".to_string(),
                source_sequence: sequence,
                phase,
            }),
        }
    }

    #[test]
    fn narrow_header_keeps_complete_status_tokens() {
        let mut app = YunxiTuiApp::default();
        app.set_banner(banner());

        let header = app.header_for_width(58);
        let subheader = app.subheader_for_width(58);
        let footer = app.footer_for_width(58);

        assert!(display_width(&header) <= 58);
        assert!(display_width(&subheader) <= 58);
        assert!(display_width(&footer) <= 58);
        assert!(header.contains("YunXi v2.0.2"));
        assert!(header.contains("offline"));
        assert!(header.contains("static"));
        assert!(subheader.contains("provider=static"));
        assert!(!subheader.ends_with('|'));
        assert!(!subheader.ends_with("| d"));
        assert_eq!(footer, "Enter submit | /help | wheel scroll | Ctrl+C exit");
    }

    #[test]
    fn wide_header_preserves_provider_and_model_details() {
        let mut app = YunxiTuiApp::default();
        app.set_banner(banner());

        let header = app.header_for_width(120);
        let subheader = app.subheader_for_width(120);

        assert!(header.contains("YunXi Agent v2.0.2"));
        assert!(header.contains("model=deepseek-chat"));
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

        assert!(display_width(&header) <= 100);
        assert!(header.contains("deepseek live"));
        assert!(header.contains("model=deepseek-chat"));
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
        app.push_user("history test");
        app.push_agent_event(&assistant_event(
            "partial",
            1,
            AgentMessageStreamPhase::Delta,
        ));
        app.scroll_up(4, 30, 10);
        let before = app.viewport().view_start(30, 10);

        app.push_agent_event(&assistant_event(
            "final answer",
            2,
            AgentMessageStreamPhase::Final,
        ));

        assert_eq!(app.viewport().view_start(30, 10), before);
        assert_eq!(app.viewport().scroll_status(), "new output below");
    }
}

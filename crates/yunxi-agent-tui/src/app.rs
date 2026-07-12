use crate::bottom_pane::{ApprovalRequestView, BottomPane, UserInputRequestView};
use crate::chat::Transcript;
use crate::viewport::TranscriptViewport;
use yunxi_agent_core::AgentEvent;

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
    transcript: Transcript,
    viewport: TranscriptViewport,
    bottom_pane: BottomPane,
}

impl Default for YunxiTuiApp {
    fn default() -> Self {
        Self {
            version: "v1.7.3".to_string(),
            banner: None,
            transcript: Transcript::default(),
            viewport: TranscriptViewport::default(),
            bottom_pane: BottomPane::default(),
        }
    }
}

impl YunxiTuiApp {
    pub(crate) fn set_banner(&mut self, banner: YunxiTuiBanner) {
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

    pub(crate) fn footer(&self) -> String {
        match self.viewport.scroll_status() {
            "new output below" => {
                "new output below | End follow tail | wheel/PageDown history".to_string()
            }
            "history" => "history view | End follow tail | wheel/PageUp/PageDown".to_string(),
            _ => "Enter submit | Alt+Enter newline | /help commands | wheel scroll | Ctrl+C exit"
                .to_string(),
        }
    }

    pub(crate) fn header(&self) -> String {
        match &self.banner {
            Some(banner) => format!(
                "YunXi Agent {} | {} | provider={} mode={} model={}",
                self.version,
                banner.cwd,
                banner.provider,
                if banner.provider_live {
                    "live"
                } else {
                    "offline"
                },
                banner.model
            ),
            None => format!("YunXi Agent {} interactive CLI", self.version),
        }
    }

    pub(crate) fn subheader(&self) -> String {
        match &self.banner {
            Some(banner) => format!(
                "backend={} source={} | transcript cells={} | view={} | {}",
                banner.backend,
                banner.provider_source,
                self.transcript.cells().len(),
                self.viewport.scroll_status(),
                self.transcript.debug_status()
            ),
            None => "initializing".to_string(),
        }
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
        self.transcript.push_user(value);
        self.on_transcript_changed();
    }

    pub(crate) fn push_agent_event(&mut self, event: &AgentEvent) {
        self.transcript.push_agent_event(event);
        self.on_transcript_changed();
    }

    pub(crate) fn set_debug_events(&mut self, enabled: bool) {
        self.transcript.set_debug_events(enabled);
        self.on_transcript_changed();
    }

    pub(crate) fn show_details(&mut self, id: Option<usize>) {
        self.transcript.push_details(id);
        self.on_transcript_changed();
    }

    pub(crate) fn push_assistant(&mut self, content: &str) {
        self.transcript.push_assistant(content);
        self.on_transcript_changed();
    }

    pub(crate) fn push_notice(&mut self, kind: &str, message: &str) {
        self.transcript.push_notice(kind, message);
        self.on_transcript_changed();
    }

    pub(crate) fn push_warning(&mut self, message: &str) {
        self.transcript.push_warning(message);
        self.on_transcript_changed();
    }

    pub(crate) fn push_error(&mut self, message: &str) {
        self.transcript.push_error(message);
        self.on_transcript_changed();
    }

    pub(crate) fn clear_transcript(&mut self) {
        self.transcript.clear();
        self.viewport.reset();
    }

    pub(crate) fn transcript_content_height(&self) -> usize {
        self.transcript.render_line_count()
    }

    pub(crate) fn scroll_up(&mut self, lines: usize, visible_height: usize) {
        self.viewport
            .scroll_up(lines, self.transcript_content_height(), visible_height);
    }

    pub(crate) fn scroll_down(&mut self, lines: usize, visible_height: usize) {
        self.viewport
            .scroll_down(lines.max(1).min(visible_height.max(1)));
    }

    pub(crate) fn page_up(&mut self, visible_height: usize) {
        self.viewport
            .page_up(self.transcript_content_height(), visible_height);
    }

    pub(crate) fn page_down(&mut self, visible_height: usize) {
        self.viewport.page_down(visible_height);
    }

    pub(crate) fn jump_top(&mut self, visible_height: usize) {
        self.viewport
            .jump_top(self.transcript_content_height(), visible_height);
    }

    pub(crate) fn follow_tail(&mut self) {
        self.viewport.follow_tail();
    }

    fn on_transcript_changed(&mut self) {
        self.viewport.on_content_changed();
    }
}

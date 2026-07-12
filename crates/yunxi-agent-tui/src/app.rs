use crate::bottom_pane::{ApprovalRequestView, BottomPane, UserInputRequestView};
use crate::chat::Transcript;
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
    bottom_pane: BottomPane,
    footer: String,
}

impl Default for YunxiTuiApp {
    fn default() -> Self {
        Self {
            version: "v1.7.1".to_string(),
            banner: None,
            transcript: Transcript::default(),
            bottom_pane: BottomPane::default(),
            footer: "Enter submit | Alt+Enter newline | /help commands | Ctrl+C exit".to_string(),
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

    pub(crate) fn bottom_pane(&self) -> &BottomPane {
        &self.bottom_pane
    }

    pub(crate) fn bottom_pane_mut(&mut self) -> &mut BottomPane {
        &mut self.bottom_pane
    }

    pub(crate) fn footer(&self) -> &str {
        &self.footer
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
                "backend={} source={} | transcript cells={} | autonomous tui",
                banner.backend,
                banner.provider_source,
                self.transcript.cells().len()
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
    }

    pub(crate) fn push_agent_event(&mut self, event: &AgentEvent) {
        self.transcript.push_agent_event(event);
    }

    pub(crate) fn push_assistant(&mut self, content: &str) {
        self.transcript.push_assistant(content);
    }

    pub(crate) fn push_notice(&mut self, kind: &str, message: &str) {
        self.transcript.push_notice(kind, message);
    }

    pub(crate) fn push_warning(&mut self, message: &str) {
        self.transcript.push_warning(message);
    }

    pub(crate) fn push_error(&mut self, message: &str) {
        self.transcript.push_error(message);
    }

    pub(crate) fn clear_transcript(&mut self) {
        self.transcript.clear();
    }
}

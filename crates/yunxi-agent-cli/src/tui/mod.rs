use crate::input::InteractiveInput;
use crate::render::{InteractiveBanner, InteractiveRenderer, RenderState};
use anyhow::Result;
use std::cell::RefCell;
use std::rc::Rc;
use yunxi_agent_core::{
    AgentEvent, AgentRunApprovalDecision, AgentRunApprovalRequest, AgentRunUserInputRequest,
    AgentRunUserInputResponse,
    ControlSnapshot,
};
use yunxi_agent_tui::{ApprovalRequestView, UserInputRequestView, YunxiTui, YunxiTuiBanner};

#[derive(Clone)]
pub(crate) struct TuiHandle {
    inner: Rc<RefCell<YunxiTui>>,
}

impl TuiHandle {
    pub(crate) fn enter() -> Result<Self> {
        Ok(Self {
            inner: Rc::new(RefCell::new(YunxiTui::enter()?)),
        })
    }

    fn with_mut<T>(&self, f: impl FnOnce(&mut YunxiTui) -> Result<T>) -> Result<T> {
        let mut tui = self.inner.borrow_mut();
        f(&mut tui)
    }
}

pub(crate) struct TuiInput {
    handle: TuiHandle,
}

impl TuiInput {
    pub(crate) fn new(handle: TuiHandle) -> Self {
        Self { handle }
    }
}

impl InteractiveInput for TuiInput {
    fn read_prompt(&mut self, prompt: &str) -> Result<Option<String>> {
        self.handle.with_mut(|tui| tui.read_prompt(prompt))
    }

    fn read_response(&mut self, prompt: &str) -> Result<Option<String>> {
        let response = self
            .handle
            .with_mut(|tui| {
                tui.request_user_input(UserInputRequestView {
                    id: None,
                    prompt: prompt.trim().to_string(),
                })
            })?;
        Ok(response.value)
    }

    fn print_eof_message(&self) -> bool {
        false
    }
}

pub(crate) struct TuiInteractiveRenderer {
    handle: TuiHandle,
}

impl TuiInteractiveRenderer {
    pub(crate) fn new(handle: TuiHandle) -> Self {
        Self { handle }
    }
}

impl InteractiveRenderer for TuiInteractiveRenderer {
    fn banner(&mut self, banner: &InteractiveBanner) -> Result<()> {
        self.handle.with_mut(|tui| {
            tui.set_banner(YunxiTuiBanner {
                cwd: banner.cwd.clone(),
                backend: banner.backend.clone(),
                provider_live: banner.provider_live,
                provider_source: banner.provider_source.clone(),
                model: banner.model.clone(),
                provider: banner.provider.clone(),
            })
        })
    }

    fn warning(&mut self, message: &str) -> Result<()> {
        self.handle.with_mut(|tui| tui.push_warning(message))
    }

    fn notice(&mut self, label: &str, message: &str) -> Result<()> {
        self.handle.with_mut(|tui| tui.push_notice(label, message))
    }

    fn clear(&mut self) -> Result<()> {
        self.handle.with_mut(YunxiTui::clear_transcript)
    }

    fn controls(&mut self, snapshot: &ControlSnapshot) -> Result<()> {
        self.handle
            .with_mut(|tui| tui.show_control_snapshot(snapshot.clone()))
    }

    fn event(&mut self, event: &AgentEvent, state: &mut RenderState) -> Result<()> {
        state.observe_event(event);
        self.handle.with_mut(|tui| tui.push_agent_event(event))
    }

    fn approval_request(
        &mut self,
        request: AgentRunApprovalRequest,
        _input: &mut dyn InteractiveInput,
    ) -> Result<()> {
        let decision = self.handle.with_mut(|tui| {
            tui.request_approval(ApprovalRequestView {
                id: request.id.clone(),
                tool_name: request.tool_name.clone(),
                cwd: request.cwd.clone(),
                command: request.command.clone(),
                reason: request.reason.clone(),
                risk_label: None,
            })
        })?;
        let _ = request.respond_to.send(AgentRunApprovalDecision {
            approved: decision.approved,
            reason: decision.reason,
        });
        Ok(())
    }

    fn user_input_request(
        &mut self,
        request: AgentRunUserInputRequest,
        _input: &mut dyn InteractiveInput,
    ) -> Result<()> {
        let response = self.handle.with_mut(|tui| {
            tui.request_user_input(UserInputRequestView {
                id: request.id.clone(),
                prompt: request.prompt.clone(),
            })
        })?;
        let _ = request.respond_to.send(AgentRunUserInputResponse {
            value: response.value,
        });
        Ok(())
    }

    fn tick(&mut self) -> Result<()> {
        self.handle.with_mut(YunxiTui::tick)
    }

    fn flush(&mut self) -> Result<()> {
        self.handle.with_mut(YunxiTui::flush)
    }

    fn set_debug_events(&mut self, enabled: bool) -> Result<()> {
        self.handle.with_mut(|tui| tui.set_debug_events(enabled))
    }

    fn show_details(&mut self, id: Option<usize>) -> Result<()> {
        self.handle.with_mut(|tui| tui.show_details(id))
    }

    fn error(&mut self, message: &str) -> Result<()> {
        self.handle.with_mut(|tui| tui.push_error(message))
    }
}

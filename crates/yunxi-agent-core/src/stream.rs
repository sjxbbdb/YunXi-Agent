use crate::{AgentCancellationToken, AgentEvent, AgentResult};
use tokio::sync::{mpsc, oneshot};

#[derive(Clone, Debug)]
pub struct AgentRunControl {
    event_tx: Option<mpsc::UnboundedSender<AgentEvent>>,
    approval_tx: Option<mpsc::UnboundedSender<AgentRunApprovalRequest>>,
    user_input_tx: Option<mpsc::UnboundedSender<AgentRunUserInputRequest>>,
    cancellation_token: AgentCancellationToken,
}

#[derive(Debug)]
pub struct AgentRunStreamReceiver {
    pub events: mpsc::UnboundedReceiver<AgentEvent>,
    pub approvals: mpsc::UnboundedReceiver<AgentRunApprovalRequest>,
    pub user_inputs: mpsc::UnboundedReceiver<AgentRunUserInputRequest>,
}

#[derive(Debug)]
pub struct AgentRunApprovalRequest {
    pub id: Option<String>,
    pub tool_name: String,
    pub reason: String,
    pub command: Option<String>,
    pub cwd: String,
    pub respond_to: oneshot::Sender<AgentRunApprovalDecision>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentRunApprovalDecision {
    pub approved: bool,
    pub reason: Option<String>,
}

#[derive(Debug)]
pub struct AgentRunUserInputRequest {
    pub id: Option<String>,
    pub prompt: String,
    pub respond_to: oneshot::Sender<AgentRunUserInputResponse>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentRunUserInputResponse {
    pub value: Option<String>,
}

impl AgentRunControl {
    pub fn detached() -> Self {
        Self {
            event_tx: None,
            approval_tx: None,
            user_input_tx: None,
            cancellation_token: AgentCancellationToken::new(),
        }
    }

    pub fn streaming() -> (Self, AgentRunStreamReceiver) {
        let (event_tx, events) = mpsc::unbounded_channel();
        let (approval_tx, approvals) = mpsc::unbounded_channel();
        let (user_input_tx, user_inputs) = mpsc::unbounded_channel();
        (
            Self {
                event_tx: Some(event_tx),
                approval_tx: Some(approval_tx),
                user_input_tx: Some(user_input_tx),
                cancellation_token: AgentCancellationToken::new(),
            },
            AgentRunStreamReceiver {
                events,
                approvals,
                user_inputs,
            },
        )
    }

    pub fn with_cancellation_token(mut self, cancellation_token: AgentCancellationToken) -> Self {
        self.cancellation_token = cancellation_token;
        self
    }

    pub fn cancellation_token(&self) -> AgentCancellationToken {
        self.cancellation_token.clone()
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancellation_token.is_cancelled()
    }

    pub fn cancel(&self) {
        self.cancellation_token.cancel();
    }

    pub fn has_interactive_approval(&self) -> bool {
        self.approval_tx.is_some()
    }

    pub fn has_interactive_user_input(&self) -> bool {
        self.user_input_tx.is_some()
    }

    pub fn emit_event(&self, event: AgentEvent) {
        if let Some(tx) = &self.event_tx {
            let _ = tx.send(event);
        }
    }

    pub async fn request_approval(
        &self,
        id: Option<String>,
        tool_name: impl Into<String>,
        reason: impl Into<String>,
        command: Option<String>,
        cwd: impl Into<String>,
    ) -> AgentResult<Option<AgentRunApprovalDecision>> {
        let Some(tx) = &self.approval_tx else {
            return Ok(None);
        };
        let (respond_to, response) = oneshot::channel();
        if tx
            .send(AgentRunApprovalRequest {
                id,
                tool_name: tool_name.into(),
                reason: reason.into(),
                command,
                cwd: cwd.into(),
                respond_to,
            })
            .is_err()
        {
            return Ok(None);
        }
        Ok(response.await.ok())
    }

    pub async fn request_user_input(
        &self,
        id: Option<String>,
        prompt: impl Into<String>,
    ) -> AgentResult<Option<AgentRunUserInputResponse>> {
        let Some(tx) = &self.user_input_tx else {
            return Ok(None);
        };
        let (respond_to, response) = oneshot::channel();
        if tx
            .send(AgentRunUserInputRequest {
                id,
                prompt: prompt.into(),
                respond_to,
            })
            .is_err()
        {
            return Ok(None);
        }
        Ok(response.await.ok())
    }
}

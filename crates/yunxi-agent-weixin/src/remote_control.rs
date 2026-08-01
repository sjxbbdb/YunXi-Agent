use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use thiserror::Error;
use yunxi_agent_core::{
    AgentCancellationToken, AgentRunApprovalDecision, AgentRunApprovalRequest,
    AgentRunUserInputRequest, AgentRunUserInputResponse,
};
use yunxi_agent_storage::{
    FileWeixinStateStore, WeixinRemoteControlCommitItem, WeixinRemoteControlPurposeRecord,
    WeixinRemoteControlState, WeixinStateError,
};

use crate::redaction::redacted_identifier;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WeixinRemoteCommand {
    Status,
    Stop,
    Approve {
        request_id: String,
    },
    Deny {
        request_id: String,
        reason: Option<String>,
    },
    Answer {
        request_id: String,
        text: String,
    },
}

pub fn parse_weixin_remote_command(input: &str) -> Option<WeixinRemoteCommand> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return None;
    }
    let mut parts = trimmed.splitn(3, char::is_whitespace);
    let command = parts.next()?.to_ascii_lowercase();
    let first = parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let rest = parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    match command.as_str() {
        "/status" => Some(WeixinRemoteCommand::Status),
        "/stop" => Some(WeixinRemoteCommand::Stop),
        "/approve" => Some(WeixinRemoteCommand::Approve {
            request_id: first?.to_string(),
        }),
        "/deny" => Some(WeixinRemoteCommand::Deny {
            request_id: first?.to_string(),
            reason: rest.map(safe_remote_text),
        }),
        "/answer" => Some(WeixinRemoteCommand::Answer {
            request_id: first?.to_string(),
            text: safe_remote_text(rest?),
        }),
        _ => None,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeixinRemoteControlScope {
    pub account_id: String,
    pub peer_id_hash: String,
    pub direct_message_key: String,
    pub item_id: String,
    pub session_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeixinRemoteControlPrompt {
    pub scope: WeixinRemoteControlScope,
    pub request_id: String,
    pub purpose: WeixinRemoteControlPurpose,
    pub action: String,
    pub reason: String,
    pub cwd_label: Option<String>,
    pub expires_at_millis: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WeixinRemoteControlPurpose {
    Approval,
    UserInput,
    Cancellation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeixinRemoteControlOutcome {
    pub scope: WeixinRemoteControlScope,
    pub request_id: Option<String>,
    pub status: &'static str,
    pub message: String,
}

#[derive(Clone, Default)]
pub struct WeixinRemoteControlHub {
    inner: Arc<Mutex<HashMap<String, RegisteredControlRequest>>>,
    state_store: Option<FileWeixinStateStore>,
}

impl WeixinRemoteControlHub {
    pub fn with_state_store(state_store: FileWeixinStateStore) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            state_store: Some(state_store),
        }
    }

    pub fn pending_count(&self) -> usize {
        self.inner
            .lock()
            .map(|inner| inner.len())
            .unwrap_or_default()
    }

    pub fn register_approval(
        &self,
        scope: WeixinRemoteControlScope,
        request: AgentRunApprovalRequest,
        expires_at_millis: u64,
    ) -> Result<WeixinRemoteControlPrompt, WeixinRemoteControlError> {
        let request_id = remote_request_id(
            "approval",
            &scope,
            request.id.as_deref().unwrap_or(&request.tool_name),
        );
        let prompt = WeixinRemoteControlPrompt {
            scope: scope.clone(),
            request_id: request_id.clone(),
            purpose: WeixinRemoteControlPurpose::Approval,
            action: safe_remote_text(&request.tool_name),
            reason: safe_remote_text(&request.reason),
            cwd_label: Some(redacted_identifier("cwd", &request.cwd)),
            expires_at_millis,
        };
        let respond_to = request.respond_to;
        if let Err(error) = self.persist_register(&scope, &prompt, remote_now_millis()) {
            let _ = respond_to.send(AgentRunApprovalDecision {
                approved: false,
                reason: Some("weixin_remote_control_unavailable".to_string()),
            });
            return Err(error);
        }
        let mut inner = match self.inner.lock() {
            Ok(inner) => inner,
            Err(_) => {
                let _ = respond_to.send(AgentRunApprovalDecision {
                    approved: false,
                    reason: Some("weixin_remote_control_unavailable".to_string()),
                });
                return Err(WeixinRemoteControlError::Poisoned);
            }
        };
        inner.insert(
            request_id,
            RegisteredControlRequest {
                scope,
                expires_at_millis,
                kind: RegisteredControlKind::Approval { respond_to },
            },
        );
        Ok(prompt)
    }

    pub fn register_user_input(
        &self,
        scope: WeixinRemoteControlScope,
        request: AgentRunUserInputRequest,
        expires_at_millis: u64,
    ) -> Result<WeixinRemoteControlPrompt, WeixinRemoteControlError> {
        let request_id = remote_request_id(
            "input",
            &scope,
            request.id.as_deref().unwrap_or("user-input"),
        );
        let prompt = WeixinRemoteControlPrompt {
            scope: scope.clone(),
            request_id: request_id.clone(),
            purpose: WeixinRemoteControlPurpose::UserInput,
            action: "answer".to_string(),
            reason: safe_remote_text(&request.prompt),
            cwd_label: None,
            expires_at_millis,
        };
        let respond_to = request.respond_to;
        if let Err(error) = self.persist_register(&scope, &prompt, remote_now_millis()) {
            let _ = respond_to.send(AgentRunUserInputResponse { value: None });
            return Err(error);
        }
        let mut inner = match self.inner.lock() {
            Ok(inner) => inner,
            Err(_) => {
                let _ = respond_to.send(AgentRunUserInputResponse { value: None });
                return Err(WeixinRemoteControlError::Poisoned);
            }
        };
        inner.insert(
            request_id,
            RegisteredControlRequest {
                scope,
                expires_at_millis,
                kind: RegisteredControlKind::UserInput { respond_to },
            },
        );
        Ok(prompt)
    }

    pub fn register_cancellation(
        &self,
        scope: WeixinRemoteControlScope,
        cancellation_token: AgentCancellationToken,
        expires_at_millis: u64,
    ) -> Result<WeixinRemoteControlPrompt, WeixinRemoteControlError> {
        let request_id = remote_request_id("stop", &scope, "turn");
        let prompt = WeixinRemoteControlPrompt {
            scope: scope.clone(),
            request_id: request_id.clone(),
            purpose: WeixinRemoteControlPurpose::Cancellation,
            action: "stop".to_string(),
            reason: "cancel current turn".to_string(),
            cwd_label: None,
            expires_at_millis,
        };
        self.persist_register(&scope, &prompt, remote_now_millis())?;
        self.inner
            .lock()
            .map_err(|_| WeixinRemoteControlError::Poisoned)?
            .insert(
                request_id,
                RegisteredControlRequest {
                    scope,
                    expires_at_millis,
                    kind: RegisteredControlKind::Cancellation { cancellation_token },
                },
            );
        Ok(prompt)
    }

    pub fn handle_command(
        &self,
        scope: &WeixinRemoteControlScope,
        command: WeixinRemoteCommand,
        now_millis: u64,
    ) -> Result<WeixinRemoteControlOutcome, WeixinRemoteControlError> {
        match command {
            WeixinRemoteCommand::Status => Ok(WeixinRemoteControlOutcome {
                scope: scope.clone(),
                request_id: None,
                status: "ok",
                message: format!("pending_control_requests={}", self.pending_count()),
            }),
            WeixinRemoteCommand::Stop => self.stop_scope(scope, now_millis),
            WeixinRemoteCommand::Approve { request_id } => {
                self.approve(scope, &request_id, now_millis)
            }
            WeixinRemoteCommand::Deny { request_id, reason } => {
                self.deny(scope, &request_id, reason, now_millis)
            }
            WeixinRemoteCommand::Answer { request_id, text } => {
                self.answer(scope, &request_id, text, now_millis)
            }
        }
    }

    pub fn expire_request(
        &self,
        request_id: &str,
        now_millis: u64,
    ) -> Result<bool, WeixinRemoteControlError> {
        let request = self
            .inner
            .lock()
            .map_err(|_| WeixinRemoteControlError::Poisoned)?
            .remove(request_id);
        let Some(request) = request else {
            return Ok(false);
        };
        if request.expires_at_millis > now_millis {
            self.inner
                .lock()
                .map_err(|_| WeixinRemoteControlError::Poisoned)?
                .insert(request_id.to_string(), request);
            return Ok(false);
        }
        match request.kind {
            RegisteredControlKind::Approval { respond_to } => {
                let _ = respond_to.send(AgentRunApprovalDecision {
                    approved: false,
                    reason: Some("weixin_remote_control_timeout".to_string()),
                });
            }
            RegisteredControlKind::UserInput { respond_to } => {
                let _ = respond_to.send(AgentRunUserInputResponse { value: None });
            }
            RegisteredControlKind::Cancellation { .. } => {}
        }
        self.persist_transition(
            &request.scope.account_id,
            request_id,
            WeixinRemoteControlState::Expired,
            "timeout",
            now_millis,
        )?;
        Ok(true)
    }

    pub fn close_scope(
        &self,
        scope: &WeixinRemoteControlScope,
    ) -> Result<usize, WeixinRemoteControlError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| WeixinRemoteControlError::Poisoned)?;
        let request_ids = inner
            .iter()
            .filter_map(|(request_id, request)| {
                request.scope.matches(scope).then(|| request_id.clone())
            })
            .collect::<Vec<_>>();
        let mut closed = 0usize;
        for request_id in request_ids {
            if let Some(request) = inner.remove(&request_id) {
                match request.kind {
                    RegisteredControlKind::Approval { respond_to } => {
                        let _ = respond_to.send(AgentRunApprovalDecision {
                            approved: false,
                            reason: Some("weixin_turn_closed".to_string()),
                        });
                        self.persist_transition(
                            &request.scope.account_id,
                            &request_id,
                            WeixinRemoteControlState::Rejected,
                            "turn_closed",
                            remote_now_millis(),
                        )?;
                    }
                    RegisteredControlKind::UserInput { respond_to } => {
                        let _ = respond_to.send(AgentRunUserInputResponse { value: None });
                        self.persist_transition(
                            &request.scope.account_id,
                            &request_id,
                            WeixinRemoteControlState::Rejected,
                            "turn_closed",
                            remote_now_millis(),
                        )?;
                    }
                    RegisteredControlKind::Cancellation { .. } => {
                        self.persist_transition(
                            &request.scope.account_id,
                            &request_id,
                            WeixinRemoteControlState::Cancelled,
                            "turn_closed",
                            remote_now_millis(),
                        )?;
                    }
                }
                closed += 1;
            }
        }
        Ok(closed)
    }

    pub fn reject_request(
        &self,
        request_id: &str,
        reason: &'static str,
        now_millis: u64,
    ) -> Result<bool, WeixinRemoteControlError> {
        let request = self
            .inner
            .lock()
            .map_err(|_| WeixinRemoteControlError::Poisoned)?
            .remove(request_id);
        let Some(request) = request else {
            return Ok(false);
        };
        match request.kind {
            RegisteredControlKind::Approval { respond_to } => {
                let _ = respond_to.send(AgentRunApprovalDecision {
                    approved: false,
                    reason: Some(reason.to_string()),
                });
            }
            RegisteredControlKind::UserInput { respond_to } => {
                let _ = respond_to.send(AgentRunUserInputResponse { value: None });
            }
            RegisteredControlKind::Cancellation { .. } => {}
        }
        self.persist_transition(
            &request.scope.account_id,
            request_id,
            WeixinRemoteControlState::Rejected,
            reason,
            now_millis,
        )?;
        Ok(true)
    }

    fn persist_register(
        &self,
        scope: &WeixinRemoteControlScope,
        prompt: &WeixinRemoteControlPrompt,
        now_millis: u64,
    ) -> Result<(), WeixinRemoteControlError> {
        let Some(store) = self.state_store.as_ref() else {
            return Ok(());
        };
        store.record_remote_control_request(
            &scope.account_id,
            WeixinRemoteControlCommitItem {
                request_id: prompt.request_id.clone(),
                peer_id_hash: scope.peer_id_hash.clone(),
                direct_message_key: scope.direct_message_key.clone(),
                item_id: scope.item_id.clone(),
                session_id: scope.session_id.clone(),
                purpose: storage_purpose(prompt.purpose),
                action: prompt.action.clone(),
                reason: prompt.reason.clone(),
                cwd_label: prompt.cwd_label.clone(),
                expires_at_millis: prompt.expires_at_millis,
            },
            now_millis,
        )?;
        Ok(())
    }

    fn persist_transition(
        &self,
        account_id: &str,
        request_id: &str,
        target: WeixinRemoteControlState,
        last_status: &'static str,
        now_millis: u64,
    ) -> Result<(), WeixinRemoteControlError> {
        let Some(store) = self.state_store.as_ref() else {
            return Ok(());
        };
        store.transition_remote_control_request(
            account_id,
            request_id,
            target,
            Some(last_status.to_string()),
            now_millis,
        )?;
        Ok(())
    }

    fn approve(
        &self,
        scope: &WeixinRemoteControlScope,
        request_id: &str,
        now_millis: u64,
    ) -> Result<WeixinRemoteControlOutcome, WeixinRemoteControlError> {
        let request = self.take_matching(scope, request_id, now_millis)?;
        if !matches!(request.kind, RegisteredControlKind::Approval { .. }) {
            self.restore_request(request_id, request)?;
            return Err(WeixinRemoteControlError::PurposeMismatch);
        }
        match request.kind {
            RegisteredControlKind::Approval { respond_to } => {
                if respond_to
                    .send(AgentRunApprovalDecision {
                        approved: true,
                        reason: Some("approved_from_weixin".to_string()),
                    })
                    .is_err()
                {
                    self.persist_transition(
                        &request.scope.account_id,
                        request_id,
                        WeixinRemoteControlState::Rejected,
                        "channel_closed",
                        now_millis,
                    )?;
                    return Err(WeixinRemoteControlError::ResponseChannelClosed);
                }
                self.persist_transition(
                    &request.scope.account_id,
                    request_id,
                    WeixinRemoteControlState::Consumed,
                    "approved",
                    now_millis,
                )?;
                Ok(outcome(
                    request.scope.clone(),
                    request_id,
                    "consumed",
                    "approval accepted",
                ))
            }
            _ => unreachable!("purpose checked above"),
        }
    }

    fn deny(
        &self,
        scope: &WeixinRemoteControlScope,
        request_id: &str,
        reason: Option<String>,
        now_millis: u64,
    ) -> Result<WeixinRemoteControlOutcome, WeixinRemoteControlError> {
        let request = self.take_matching(scope, request_id, now_millis)?;
        if !matches!(request.kind, RegisteredControlKind::Approval { .. }) {
            self.restore_request(request_id, request)?;
            return Err(WeixinRemoteControlError::PurposeMismatch);
        }
        match request.kind {
            RegisteredControlKind::Approval { respond_to } => {
                if respond_to
                    .send(AgentRunApprovalDecision {
                        approved: false,
                        reason: Some(reason.unwrap_or_else(|| "denied_from_weixin".to_string())),
                    })
                    .is_err()
                {
                    self.persist_transition(
                        &request.scope.account_id,
                        request_id,
                        WeixinRemoteControlState::Rejected,
                        "channel_closed",
                        now_millis,
                    )?;
                    return Err(WeixinRemoteControlError::ResponseChannelClosed);
                }
                self.persist_transition(
                    &request.scope.account_id,
                    request_id,
                    WeixinRemoteControlState::Rejected,
                    "denied",
                    now_millis,
                )?;
                Ok(outcome(
                    request.scope.clone(),
                    request_id,
                    "consumed",
                    "approval denied",
                ))
            }
            _ => unreachable!("purpose checked above"),
        }
    }

    fn answer(
        &self,
        scope: &WeixinRemoteControlScope,
        request_id: &str,
        text: String,
        now_millis: u64,
    ) -> Result<WeixinRemoteControlOutcome, WeixinRemoteControlError> {
        let request = self.take_matching(scope, request_id, now_millis)?;
        if !matches!(request.kind, RegisteredControlKind::UserInput { .. }) {
            self.restore_request(request_id, request)?;
            return Err(WeixinRemoteControlError::PurposeMismatch);
        }
        match request.kind {
            RegisteredControlKind::UserInput { respond_to } => {
                if respond_to
                    .send(AgentRunUserInputResponse { value: Some(text) })
                    .is_err()
                {
                    self.persist_transition(
                        &request.scope.account_id,
                        request_id,
                        WeixinRemoteControlState::Rejected,
                        "channel_closed",
                        now_millis,
                    )?;
                    return Err(WeixinRemoteControlError::ResponseChannelClosed);
                }
                self.persist_transition(
                    &request.scope.account_id,
                    request_id,
                    WeixinRemoteControlState::Consumed,
                    "answered",
                    now_millis,
                )?;
                Ok(outcome(
                    request.scope.clone(),
                    request_id,
                    "consumed",
                    "answer accepted",
                ))
            }
            _ => unreachable!("purpose checked above"),
        }
    }

    fn stop_scope(
        &self,
        scope: &WeixinRemoteControlScope,
        now_millis: u64,
    ) -> Result<WeixinRemoteControlOutcome, WeixinRemoteControlError> {
        let request_id = {
            let inner = self
                .inner
                .lock()
                .map_err(|_| WeixinRemoteControlError::Poisoned)?;
            inner.iter().find_map(|(request_id, request)| {
                (request.scope.matches(scope)
                    && request.expires_at_millis > now_millis
                    && matches!(request.kind, RegisteredControlKind::Cancellation { .. }))
                .then(|| request_id.clone())
            })
        }
        .ok_or(WeixinRemoteControlError::RequestNotFound)?;
        let request = self.take_matching(scope, &request_id, now_millis)?;
        if !matches!(request.kind, RegisteredControlKind::Cancellation { .. }) {
            self.restore_request(&request_id, request)?;
            return Err(WeixinRemoteControlError::PurposeMismatch);
        }
        match request.kind {
            RegisteredControlKind::Cancellation { cancellation_token } => {
                cancellation_token.cancel();
                self.persist_transition(
                    &request.scope.account_id,
                    &request_id,
                    WeixinRemoteControlState::Cancelled,
                    "cancelled",
                    now_millis,
                )?;
                Ok(outcome(
                    request.scope.clone(),
                    &request_id,
                    "consumed",
                    "turn cancellation requested",
                ))
            }
            _ => unreachable!("purpose checked above"),
        }
    }

    fn take_matching(
        &self,
        scope: &WeixinRemoteControlScope,
        request_id: &str,
        now_millis: u64,
    ) -> Result<RegisteredControlRequest, WeixinRemoteControlError> {
        let request = self
            .inner
            .lock()
            .map_err(|_| WeixinRemoteControlError::Poisoned)?
            .remove(request_id)
            .ok_or(WeixinRemoteControlError::RequestNotFound)?;
        if !request.scope.matches(scope) {
            self.restore_request(request_id, request)?;
            return Err(WeixinRemoteControlError::ScopeMismatch);
        }
        if request.expires_at_millis <= now_millis {
            match request.kind {
                RegisteredControlKind::Approval { respond_to } => {
                    let _ = respond_to.send(AgentRunApprovalDecision {
                        approved: false,
                        reason: Some("weixin_remote_control_expired".to_string()),
                    });
                }
                RegisteredControlKind::UserInput { respond_to } => {
                    let _ = respond_to.send(AgentRunUserInputResponse { value: None });
                }
                RegisteredControlKind::Cancellation { .. } => {}
            }
            self.persist_transition(
                &request.scope.account_id,
                request_id,
                WeixinRemoteControlState::Expired,
                "expired",
                now_millis,
            )?;
            return Err(WeixinRemoteControlError::Expired);
        }
        Ok(request)
    }

    fn restore_request(
        &self,
        request_id: &str,
        request: RegisteredControlRequest,
    ) -> Result<(), WeixinRemoteControlError> {
        self.inner
            .lock()
            .map_err(|_| WeixinRemoteControlError::Poisoned)?
            .insert(request_id.to_string(), request);
        Ok(())
    }
}

struct RegisteredControlRequest {
    scope: WeixinRemoteControlScope,
    expires_at_millis: u64,
    kind: RegisteredControlKind,
}

enum RegisteredControlKind {
    Approval {
        respond_to: tokio::sync::oneshot::Sender<AgentRunApprovalDecision>,
    },
    UserInput {
        respond_to: tokio::sync::oneshot::Sender<AgentRunUserInputResponse>,
    },
    Cancellation {
        cancellation_token: AgentCancellationToken,
    },
}

impl WeixinRemoteControlScope {
    fn matches(&self, other: &Self) -> bool {
        self.account_id == other.account_id
            && self.peer_id_hash == other.peer_id_hash
            && self.direct_message_key == other.direct_message_key
    }
}

#[derive(Debug, Error)]
pub enum WeixinRemoteControlError {
    #[error("weixin remote control lock was poisoned")]
    Poisoned,
    #[error("weixin remote control state failed: {0}")]
    State(#[from] WeixinStateError),
    #[error("weixin remote control request was not found")]
    RequestNotFound,
    #[error("weixin remote control request scope mismatch")]
    ScopeMismatch,
    #[error("weixin remote control request purpose mismatch")]
    PurposeMismatch,
    #[error("weixin remote control request expired")]
    Expired,
    #[error("weixin remote control response channel was closed")]
    ResponseChannelClosed,
}

fn remote_request_id(kind: &str, scope: &WeixinRemoteControlScope, source_id: &str) -> String {
    redacted_identifier(
        "wxctl",
        &format!(
            "{}:{}:{}:{}:{}:{}",
            kind,
            scope.account_id,
            scope.peer_id_hash,
            scope.direct_message_key,
            scope.item_id,
            source_id
        ),
    )
}

fn storage_purpose(purpose: WeixinRemoteControlPurpose) -> WeixinRemoteControlPurposeRecord {
    match purpose {
        WeixinRemoteControlPurpose::Approval => WeixinRemoteControlPurposeRecord::Approval,
        WeixinRemoteControlPurpose::UserInput => WeixinRemoteControlPurposeRecord::UserInput,
        WeixinRemoteControlPurpose::Cancellation => WeixinRemoteControlPurposeRecord::Cancellation,
    }
}

fn remote_now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn safe_remote_text(value: &str) -> String {
    let trimmed = value.trim();
    let mut sanitized = String::new();
    for ch in trimmed.chars().take(200) {
        if ch.is_control() {
            sanitized.push(' ');
        } else {
            sanitized.push(ch);
        }
    }
    let lower = sanitized.to_ascii_lowercase();
    if ["token", "secret", "context", "data_key", "authorization"]
        .iter()
        .any(|marker| lower.contains(marker))
    {
        "[redacted]".to_string()
    } else {
        sanitized
    }
}

fn outcome(
    scope: WeixinRemoteControlScope,
    request_id: &str,
    status: &'static str,
    message: &str,
) -> WeixinRemoteControlOutcome {
    WeixinRemoteControlOutcome {
        scope,
        request_id: Some(request_id.to_string()),
        status,
        message: message.to_string(),
    }
}

pub fn render_remote_control_prompt(prompt: &WeixinRemoteControlPrompt) -> String {
    let purpose = match prompt.purpose {
        WeixinRemoteControlPurpose::Approval => "approval",
        WeixinRemoteControlPurpose::UserInput => "user_input",
        WeixinRemoteControlPurpose::Cancellation => "cancellation",
    };
    let cwd = prompt.cwd_label.as_deref().unwrap_or("not_applicable");
    format!(
        "[YunXi 微信控制]\npurpose={purpose}\nrequest_id={}\naccount={}\npeer={}\ndm={}\nitem={}\nsession={}\naction={}\nreason={}\ncwd={cwd}\nexpires_at_millis={}\nuse /approve, /deny, /answer, or /stop as applicable",
        prompt.request_id,
        prompt.scope.account_id,
        prompt.scope.peer_id_hash,
        prompt.scope.direct_message_key,
        prompt.scope.item_id,
        prompt.scope.session_id,
        safe_remote_text(&prompt.action),
        safe_remote_text(&prompt.reason),
        prompt.expires_at_millis,
    )
}

pub fn render_remote_control_outcome(outcome: &WeixinRemoteControlOutcome) -> String {
    let request_id = outcome.request_id.as_deref().unwrap_or("none");
    format!(
        "[YunXi 微信控制结果]\nstatus={}\nrequest_id={request_id}\naccount={}\npeer={}\ndm={}\nitem={}\nsession={}\nmessage={}",
        outcome.status,
        outcome.scope.account_id,
        outcome.scope.peer_id_hash,
        outcome.scope.direct_message_key,
        outcome.scope.item_id,
        outcome.scope.session_id,
        safe_remote_text(&outcome.message),
    )
}

pub fn render_remote_control_error(
    scope: &WeixinRemoteControlScope,
    error: &WeixinRemoteControlError,
) -> String {
    let label = match error {
        WeixinRemoteControlError::Poisoned => "lock_unavailable",
        WeixinRemoteControlError::State(_) => "state_unavailable",
        WeixinRemoteControlError::RequestNotFound => "request_not_found",
        WeixinRemoteControlError::ScopeMismatch => "scope_mismatch",
        WeixinRemoteControlError::PurposeMismatch => "purpose_mismatch",
        WeixinRemoteControlError::Expired => "expired",
        WeixinRemoteControlError::ResponseChannelClosed => "response_channel_closed",
    };
    format!(
        "[YunXi 微信控制结果]\nstatus=error\nerror={label}\naccount={}\npeer={}\ndm={}\nitem={}\nsession={}\nmessage=remote control request was not applied",
        scope.account_id,
        scope.peer_id_hash,
        scope.direct_message_key,
        scope.item_id,
        scope.session_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::sync::oneshot;
    use yunxi_agent_storage::{WeixinStateSnapshot, WeixinStateStore};

    fn scope(item_id: &str) -> WeixinRemoteControlScope {
        WeixinRemoteControlScope {
            account_id: "account#11111111".to_string(),
            peer_id_hash: "peer#22222222".to_string(),
            direct_message_key: "dm#33333333".to_string(),
            item_id: item_id.to_string(),
            session_id: "session-1".to_string(),
        }
    }

    #[test]
    fn parser_accepts_only_explicit_slash_commands() {
        assert_eq!(parse_weixin_remote_command("好的"), None);
        assert_eq!(
            parse_weixin_remote_command("/status"),
            Some(WeixinRemoteCommand::Status)
        );
        assert_eq!(
            parse_weixin_remote_command("/approve wxctl#12345678"),
            Some(WeixinRemoteCommand::Approve {
                request_id: "wxctl#12345678".to_string()
            })
        );
        assert_eq!(
            parse_weixin_remote_command("/deny wxctl#12345678 too risky"),
            Some(WeixinRemoteCommand::Deny {
                request_id: "wxctl#12345678".to_string(),
                reason: Some("too risky".to_string())
            })
        );
        assert_eq!(
            parse_weixin_remote_command("/answer wxctl#12345678 yes"),
            Some(WeixinRemoteCommand::Answer {
                request_id: "wxctl#12345678".to_string(),
                text: "yes".to_string()
            })
        );
    }

    #[tokio::test]
    async fn hub_routes_approval_by_exact_scope_once() {
        let hub = WeixinRemoteControlHub::default();
        let (tx, rx) = oneshot::channel();
        let prompt = hub
            .register_approval(
                scope("item#11111111"),
                AgentRunApprovalRequest {
                    id: Some("approval-1".to_string()),
                    tool_name: "shell".to_string(),
                    reason: "safe test".to_string(),
                    command: Some("echo ok".to_string()),
                    cwd: "D:/YunXi Agent".to_string(),
                    respond_to: tx,
                },
                2000,
            )
            .expect("register");
        assert!(prompt.request_id.starts_with("wxctl#"));
        let outcome = hub
            .handle_command(
                &scope("item#11111111"),
                WeixinRemoteCommand::Approve {
                    request_id: prompt.request_id.clone(),
                },
                1000,
            )
            .expect("approve");
        assert_eq!(outcome.status, "consumed");
        assert_eq!(
            rx.await.expect("approval response"),
            AgentRunApprovalDecision {
                approved: true,
                reason: Some("approved_from_weixin".to_string())
            }
        );
        assert!(matches!(
            hub.handle_command(
                &scope("item#11111111"),
                WeixinRemoteCommand::Approve {
                    request_id: prompt.request_id
                },
                1001,
            ),
            Err(WeixinRemoteControlError::RequestNotFound)
        ));
    }

    #[tokio::test]
    async fn hub_persists_control_request_lifecycle() {
        let temp = TempDir::new().expect("temp");
        let store = FileWeixinStateStore::for_workspace(temp.path());
        let control_scope = scope("item#11111111");
        let snapshot = WeixinStateSnapshot::new(
            &control_scope.account_id,
            "workspace#73521066",
            "https://ilinkai.weixin.qq.com/",
            1000,
        );
        store.save(&snapshot).expect("save state");
        let hub = WeixinRemoteControlHub::with_state_store(store.clone());
        let (tx, rx) = oneshot::channel();
        let prompt = hub
            .register_approval(
                control_scope.clone(),
                AgentRunApprovalRequest {
                    id: Some("approval-1".to_string()),
                    tool_name: "shell".to_string(),
                    reason: "safe test".to_string(),
                    command: Some("echo ok".to_string()),
                    cwd: "D:/YunXi Agent/private".to_string(),
                    respond_to: tx,
                },
                2000,
            )
            .expect("register");
        let state = store
            .load(&control_scope.account_id)
            .expect("load")
            .expect("state");
        assert_eq!(state.pending_remote_control_count(), 1);
        assert_eq!(
            state.remote_control_requests[0].state,
            WeixinRemoteControlState::Pending
        );
        assert_eq!(
            state.remote_control_requests[0].request_id,
            prompt.request_id
        );
        assert!(
            state.remote_control_requests[0]
                .cwd_label
                .as_deref()
                .is_some_and(|cwd| cwd.starts_with("cwd#"))
        );
        let state_json = std::fs::read_to_string(store.state_path_for(&control_scope.account_id))
            .expect("state json");
        assert!(!state_json.contains("D:/YunXi Agent/private"));

        hub.handle_command(
            &control_scope,
            WeixinRemoteCommand::Approve {
                request_id: prompt.request_id.clone(),
            },
            1500,
        )
        .expect("approve");
        assert!(rx.await.expect("approval").approved);
        let state = store
            .load(&control_scope.account_id)
            .expect("load")
            .expect("state");
        assert_eq!(state.pending_remote_control_count(), 0);
        assert_eq!(
            state.remote_control_requests[0].state,
            WeixinRemoteControlState::Consumed
        );
        assert_eq!(
            state.remote_control_requests[0].last_status.as_deref(),
            Some("approved")
        );
    }

    #[test]
    fn hub_rejects_cross_scope_stop() {
        let hub = WeixinRemoteControlHub::default();
        let token = AgentCancellationToken::new();
        let prompt = hub
            .register_cancellation(scope("item#11111111"), token.clone(), 2000)
            .expect("register stop");
        let mut other_peer = scope("item#22222222");
        other_peer.peer_id_hash = "peer#99999999".to_string();
        assert!(
            hub.handle_command(&other_peer, WeixinRemoteCommand::Stop, 1000)
                .is_err()
        );
        assert!(!token.is_cancelled());
        let outcome = hub
            .handle_command(&scope("item#11111111"), WeixinRemoteCommand::Stop, 1000)
            .expect("stop");
        assert_eq!(outcome.request_id, Some(prompt.request_id));
        assert!(token.is_cancelled());
    }
}

use std::collections::BTreeMap;

use yunxi_agent_core::{
    AgentConfig, DeepParityData, ThreadRuntimeState, TurnRuntimeMetadata, TurnRuntimeState,
};
use yunxi_agent_storage::SessionId;

pub(crate) fn runtime_data(
    entries: impl IntoIterator<Item = (&'static str, String)>,
) -> DeepParityData {
    entries
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect::<BTreeMap<_, _>>()
}

pub(crate) fn thread_state(
    config: &AgentConfig,
    thread_id: &str,
    session_id: &SessionId,
    status: impl Into<String>,
    child_depth: usize,
    data: DeepParityData,
) -> ThreadRuntimeState {
    ThreadRuntimeState {
        thread_id: thread_id.to_string(),
        session_id: Some(session_id.0.clone()),
        parent_thread_id: config.parent_session_id.clone(),
        status: status.into(),
        cwd: config.cwd.display().to_string(),
        resume_source: config
            .parent_session_id
            .as_ref()
            .map(|_| "parent_session".to_string()),
        child_depth,
        data,
    }
}

pub(crate) fn turn_metadata(
    config: &AgentConfig,
    session_id: &SessionId,
    context_phase: impl Into<String>,
    child_depth: usize,
    data: DeepParityData,
) -> TurnRuntimeMetadata {
    TurnRuntimeMetadata {
        session_id: Some(session_id.0.clone()),
        cwd: config.cwd.display().to_string(),
        model: config.model.clone(),
        provider: config.provider.clone(),
        approval_mode: Some(format!("{:?}", config.approval_mode)),
        sandbox_mode: Some(format!("{:?}", config.sandbox_mode)),
        context_phase: Some(context_phase.into()),
        resume_source: config
            .parent_session_id
            .as_ref()
            .map(|_| "parent_session".to_string()),
        cancellation_state: Some("not_cancelled".to_string()),
        child_depth,
        data,
    }
}

pub(crate) fn turn_state(
    phase: impl Into<String>,
    status: impl Into<String>,
    provider_status: impl Into<String>,
    tool_loop_status: impl Into<String>,
    cancellation_state: impl Into<String>,
    data: DeepParityData,
) -> TurnRuntimeState {
    TurnRuntimeState {
        phase: phase.into(),
        status: status.into(),
        provider_status: provider_status.into(),
        tool_loop_status: tool_loop_status.into(),
        cancellation_state: cancellation_state.into(),
        data,
    }
}

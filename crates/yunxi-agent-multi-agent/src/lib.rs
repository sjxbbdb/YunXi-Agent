use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use yunxi_agent_core::{AgentError, AgentEvent, AgentResult, AgentRunStatus};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AgentId(pub String);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Running,
    Completed,
    Failed,
    Interrupted,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentMetadata {
    pub id: AgentId,
    pub parent_id: Option<AgentId>,
    pub task: String,
    pub status: AgentStatus,
    #[serde(default)]
    pub role: Option<AgentRole>,
    #[serde(default)]
    pub budget_tokens: Option<i64>,
}

/// Serializable graph state kept alongside a spawned agent's session.
///
/// The type deliberately uses string session identifiers so a storage backend can
/// persist it without depending on this crate's runtime-only `AgentId` wrapper.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentGraphSessionMetadata {
    pub session_id: String,
    #[serde(default)]
    pub parent_session_id: Option<String>,
    pub agent: AgentMetadata,
    pub created_at_millis: u128,
    pub updated_at_millis: u128,
}

impl AgentGraphSessionMetadata {
    pub fn new(session_id: impl Into<String>, agent: AgentMetadata) -> Self {
        Self {
            session_id: session_id.into(),
            parent_session_id: agent.parent_id.as_ref().map(|id| id.0.clone()),
            agent,
            created_at_millis: 0,
            updated_at_millis: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildRuntimeKind {
    YunXi,
    Fixture,
    ExternalHost,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChildAgentRunRequest {
    pub agent: AgentMetadata,
    pub session_id: String,
    pub parent_session_id: Option<String>,
    pub prompt: String,
    pub runtime: ChildRuntimeKind,
}

impl ChildAgentRunRequest {
    pub fn new(
        agent: AgentMetadata,
        session_id: impl Into<String>,
        parent_session_id: Option<String>,
        prompt: impl Into<String>,
    ) -> Self {
        Self {
            agent,
            session_id: session_id.into(),
            parent_session_id,
            prompt: prompt.into(),
            runtime: ChildRuntimeKind::YunXi,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChildAgentRunResult {
    pub agent_id: AgentId,
    pub session_id: String,
    pub parent_session_id: Option<String>,
    pub status: AgentRunStatus,
    pub final_response: Option<String>,
    pub events: Vec<AgentEvent>,
}

pub trait ChildAgentRuntime {
    fn run_child(&self, request: ChildAgentRunRequest) -> AgentResult<ChildAgentRunResult>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FixtureChildAgentRuntime;

impl ChildAgentRuntime for FixtureChildAgentRuntime {
    fn run_child(&self, request: ChildAgentRunRequest) -> AgentResult<ChildAgentRunResult> {
        let final_response = format!(
            "YunXi child agent {} completed task: {}",
            request.agent.id.0, request.prompt
        );
        Ok(ChildAgentRunResult {
            agent_id: request.agent.id.clone(),
            session_id: request.session_id,
            parent_session_id: request.parent_session_id,
            status: AgentRunStatus::Completed,
            final_response: Some(final_response.clone()),
            events: vec![
                AgentEvent::Started {
                    prompt: request.prompt,
                },
                AgentEvent::Message {
                    content: final_response,
                },
                AgentEvent::Completed {
                    status: AgentRunStatus::Completed,
                    usage: None,
                },
            ],
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    General,
    Explorer,
    Awaiter,
    Reviewer,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MultiAgentCommand {
    Spawn {
        task: String,
        parent_id: Option<AgentId>,
    },
    SpawnRun {
        task: String,
        parent_id: Option<AgentId>,
    },
    Wait {
        id: AgentId,
    },
    SendMessage {
        id: AgentId,
        message: String,
    },
    FollowUp {
        id: AgentId,
        task: String,
    },
    Interrupt {
        id: AgentId,
    },
    List,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MultiAgentLifecycleEvent {
    Spawned {
        id: AgentId,
        parent_id: Option<AgentId>,
        task: String,
    },
    MessageSent {
        id: AgentId,
        message: String,
    },
    FollowUpQueued {
        id: AgentId,
        task: String,
    },
    ChildRunStarted {
        id: AgentId,
        session_id: String,
    },
    Interrupted {
        id: AgentId,
    },
    Completed {
        id: AgentId,
        status: AgentStatus,
    },
    Listed {
        count: usize,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MultiAgentCommandResult {
    pub status: AgentStatus,
    pub agents: Vec<AgentMetadata>,
    pub events: Vec<MultiAgentLifecycleEvent>,
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child_run: Option<ChildAgentRunResult>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentGraph {
    pub agents: BTreeMap<AgentId, AgentMetadata>,
    pub children: BTreeMap<AgentId, Vec<AgentId>>,
}

impl AgentGraph {
    pub fn try_insert(&mut self, metadata: AgentMetadata) -> AgentResult<()> {
        let id = metadata.id.clone();
        let previous = self.agents.insert(id.clone(), metadata);
        if let Err(error) = self.validate() {
            match previous {
                Some(previous) => {
                    self.agents.insert(previous.id.clone(), previous);
                }
                None => {
                    self.agents.remove(&id);
                }
            }
            self.rebuild_children();
            return Err(error);
        }
        self.rebuild_children();
        Ok(())
    }

    pub fn from_session_metadata(
        sessions: impl IntoIterator<Item = AgentGraphSessionMetadata>,
    ) -> AgentResult<Self> {
        let mut graph = Self::default();
        for session in sessions {
            if session.parent_session_id != session.agent.parent_id.as_ref().map(|id| id.0.clone())
            {
                return Err(AgentError::Execution {
                    message: format!(
                        "session {} has inconsistent agent parent metadata",
                        session.session_id
                    ),
                });
            }
            graph.try_insert(session.agent)?;
        }
        Ok(graph)
    }

    pub fn validate(&self) -> AgentResult<()> {
        for id in self.agents.keys() {
            let mut ancestors = BTreeSet::new();
            let mut current = Some(id);
            while let Some(current_id) = current {
                if !ancestors.insert(current_id.clone()) {
                    return Err(AgentError::Execution {
                        message: format!("cycle detected in agent graph at {}", current_id.0),
                    });
                }
                current = self
                    .agents
                    .get(current_id)
                    .and_then(|metadata| metadata.parent_id.as_ref());
            }
        }
        Ok(())
    }

    pub fn children_of(&self, id: &AgentId) -> Vec<AgentMetadata> {
        self.children
            .get(id)
            .into_iter()
            .flatten()
            .filter_map(|child_id| self.agents.get(child_id).cloned())
            .collect()
    }

    fn rebuild_children(&mut self) {
        self.children.clear();
        for metadata in self.agents.values() {
            if let Some(parent_id) = &metadata.parent_id {
                self.children
                    .entry(parent_id.clone())
                    .or_default()
                    .push(metadata.id.clone());
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryAgentRegistry {
    agents: Arc<Mutex<BTreeMap<AgentId, AgentMetadata>>>,
    events: Arc<Mutex<Vec<MultiAgentLifecycleEvent>>>,
}

impl InMemoryAgentRegistry {
    pub fn insert(&self, metadata: AgentMetadata) -> AgentResult<()> {
        self.lock_events()?.push(MultiAgentLifecycleEvent::Spawned {
            id: metadata.id.clone(),
            parent_id: metadata.parent_id.clone(),
            task: metadata.task.clone(),
        });
        self.lock_agents()?.insert(metadata.id.clone(), metadata);
        Ok(())
    }

    pub fn list(&self) -> AgentResult<Vec<AgentMetadata>> {
        Ok(self.lock_agents()?.values().cloned().collect())
    }

    pub fn set_status(&self, id: &AgentId, status: AgentStatus) -> AgentResult<()> {
        let mut agents = self.lock_agents()?;
        let agent = agents.get_mut(id).ok_or_else(|| AgentError::Execution {
            message: format!("agent not found: {}", id.0),
        })?;
        agent.status = status;
        Ok(())
    }

    pub fn execute(&self, command: MultiAgentCommand) -> AgentResult<MultiAgentCommandResult> {
        self.execute_with_child_runtime(command, &FixtureChildAgentRuntime)
    }

    pub fn execute_with_child_runtime<R>(
        &self,
        command: MultiAgentCommand,
        child_runtime: &R,
    ) -> AgentResult<MultiAgentCommandResult>
    where
        R: ChildAgentRuntime,
    {
        match command {
            MultiAgentCommand::Spawn { task, parent_id } => {
                let id = self.next_agent_id()?;
                let metadata = AgentMetadata {
                    id: id.clone(),
                    parent_id,
                    task,
                    status: AgentStatus::Running,
                    role: Some(AgentRole::General),
                    budget_tokens: None,
                };
                self.insert(metadata.clone())?;
                Ok(MultiAgentCommandResult {
                    status: AgentStatus::Running,
                    agents: vec![metadata],
                    events: self.events()?,
                    message: Some(format!("spawned {}", id.0)),
                    child_run: None,
                })
            }
            MultiAgentCommand::SpawnRun { task, parent_id } => {
                let id = self.next_agent_id()?;
                let metadata = AgentMetadata {
                    id: id.clone(),
                    parent_id: parent_id.clone(),
                    task: task.clone(),
                    status: AgentStatus::Running,
                    role: Some(AgentRole::General),
                    budget_tokens: None,
                };
                self.insert(metadata.clone())?;
                let session_id = format!("{}-session", id.0);
                self.lock_events()?
                    .push(MultiAgentLifecycleEvent::ChildRunStarted {
                        id: id.clone(),
                        session_id: session_id.clone(),
                    });
                let run_request = ChildAgentRunRequest::new(
                    metadata.clone(),
                    session_id,
                    parent_id.map(|parent| parent.0),
                    task,
                );
                let child_run = match child_runtime.run_child(run_request.clone()) {
                    Ok(child_run) => child_run,
                    Err(error) => ChildAgentRunResult {
                        agent_id: metadata.id.clone(),
                        session_id: run_request.session_id.clone(),
                        parent_session_id: run_request.parent_session_id.clone(),
                        status: AgentRunStatus::Failed,
                        final_response: None,
                        events: vec![
                            AgentEvent::Error {
                                message: error.to_string(),
                            },
                            AgentEvent::Completed {
                                status: AgentRunStatus::Failed,
                                usage: None,
                            },
                        ],
                    },
                };
                let status = agent_status_from_run_status(child_run.status);
                self.set_status(&id, status)?;
                self.lock_events()?
                    .push(MultiAgentLifecycleEvent::Completed {
                        id: id.clone(),
                        status,
                    });
                Ok(MultiAgentCommandResult {
                    status,
                    agents: vec![self.agent(&id)?],
                    events: self.events()?,
                    message: Some(format!("spawned and ran {}", id.0)),
                    child_run: Some(child_run),
                })
            }
            MultiAgentCommand::Wait { id } => {
                let agent = self.agent(&id)?;
                Ok(MultiAgentCommandResult {
                    status: agent.status,
                    agents: vec![agent],
                    events: self.events()?,
                    message: Some(format!("wait completed for {}", id.0)),
                    child_run: None,
                })
            }
            MultiAgentCommand::SendMessage { id, message } => {
                self.agent(&id)?;
                self.lock_events()?
                    .push(MultiAgentLifecycleEvent::MessageSent {
                        id: id.clone(),
                        message,
                    });
                Ok(MultiAgentCommandResult {
                    status: AgentStatus::Running,
                    agents: vec![self.agent(&id)?],
                    events: self.events()?,
                    message: Some(format!("message sent to {}", id.0)),
                    child_run: None,
                })
            }
            MultiAgentCommand::FollowUp { id, task } => {
                self.agent(&id)?;
                self.lock_events()?
                    .push(MultiAgentLifecycleEvent::FollowUpQueued {
                        id: id.clone(),
                        task,
                    });
                Ok(MultiAgentCommandResult {
                    status: AgentStatus::Running,
                    agents: vec![self.agent(&id)?],
                    events: self.events()?,
                    message: Some(format!("follow-up queued for {}", id.0)),
                    child_run: None,
                })
            }
            MultiAgentCommand::Interrupt { id } => {
                self.set_status(&id, AgentStatus::Interrupted)?;
                self.lock_events()?
                    .push(MultiAgentLifecycleEvent::Interrupted { id: id.clone() });
                Ok(MultiAgentCommandResult {
                    status: AgentStatus::Interrupted,
                    agents: vec![self.agent(&id)?],
                    events: self.events()?,
                    message: Some(format!("interrupted {}", id.0)),
                    child_run: None,
                })
            }
            MultiAgentCommand::List => {
                let agents = self.list()?;
                self.lock_events()?.push(MultiAgentLifecycleEvent::Listed {
                    count: agents.len(),
                });
                Ok(MultiAgentCommandResult {
                    status: AgentStatus::Completed,
                    agents,
                    events: self.events()?,
                    message: None,
                    child_run: None,
                })
            }
        }
    }

    pub fn events(&self) -> AgentResult<Vec<MultiAgentLifecycleEvent>> {
        Ok(self.lock_events()?.clone())
    }

    pub fn graph(&self) -> AgentResult<AgentGraph> {
        let mut graph = AgentGraph::default();
        for metadata in self.list()? {
            graph.try_insert(metadata)?;
        }
        Ok(graph)
    }

    fn agent(&self, id: &AgentId) -> AgentResult<AgentMetadata> {
        self.lock_agents()?
            .get(id)
            .cloned()
            .ok_or_else(|| AgentError::Execution {
                message: format!("agent not found: {}", id.0),
            })
    }

    fn next_agent_id(&self) -> AgentResult<AgentId> {
        Ok(AgentId(format!("agent-{}", self.lock_agents()?.len() + 1)))
    }

    fn lock_agents(
        &self,
    ) -> AgentResult<std::sync::MutexGuard<'_, BTreeMap<AgentId, AgentMetadata>>> {
        self.agents.lock().map_err(|_| AgentError::Execution {
            message: "multi-agent registry lock was poisoned".to_string(),
        })
    }

    fn lock_events(&self) -> AgentResult<std::sync::MutexGuard<'_, Vec<MultiAgentLifecycleEvent>>> {
        self.events.lock().map_err(|_| AgentError::Execution {
            message: "multi-agent event lock was poisoned".to_string(),
        })
    }
}

fn agent_status_from_run_status(status: AgentRunStatus) -> AgentStatus {
    match status {
        AgentRunStatus::Completed => AgentStatus::Completed,
        AgentRunStatus::Failed => AgentStatus::Failed,
        AgentRunStatus::Cancelled => AgentStatus::Interrupted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_executes_spawn_message_interrupt_and_list() {
        let registry = InMemoryAgentRegistry::default();
        let spawned = registry
            .execute(MultiAgentCommand::Spawn {
                task: "explore runtime".to_string(),
                parent_id: None,
            })
            .expect("spawn");
        let id = spawned.agents[0].id.clone();

        registry
            .execute(MultiAgentCommand::SendMessage {
                id: id.clone(),
                message: "continue".to_string(),
            })
            .expect("message");
        let interrupted = registry
            .execute(MultiAgentCommand::Interrupt { id: id.clone() })
            .expect("interrupt");
        let listed = registry.execute(MultiAgentCommand::List).expect("list");

        assert_eq!(interrupted.status, AgentStatus::Interrupted);
        assert_eq!(listed.agents.len(), 1);
        assert!(
            registry
                .events()
                .expect("events")
                .iter()
                .any(|event| matches!(event, MultiAgentLifecycleEvent::MessageSent { .. }))
        );
    }

    #[test]
    fn graph_tracks_parent_child_relationships() {
        let mut graph = AgentGraph::default();
        let parent = AgentId("parent".to_string());
        let child = AgentId("child".to_string());
        graph
            .try_insert(AgentMetadata {
                id: parent.clone(),
                parent_id: None,
                task: "parent task".to_string(),
                status: AgentStatus::Running,
                role: Some(AgentRole::General),
                budget_tokens: Some(1000),
            })
            .expect("insert parent");
        graph
            .try_insert(AgentMetadata {
                id: child.clone(),
                parent_id: Some(parent.clone()),
                task: "child task".to_string(),
                status: AgentStatus::Running,
                role: Some(AgentRole::Explorer),
                budget_tokens: Some(500),
            })
            .expect("insert child");

        assert_eq!(graph.children_of(&parent)[0].id, child);
    }

    #[test]
    fn graph_rejects_parent_child_cycles_without_mutating_state() {
        let parent = AgentId("parent".to_string());
        let child = AgentId("child".to_string());
        let mut graph = AgentGraph::default();
        graph
            .try_insert(AgentMetadata {
                id: parent.clone(),
                parent_id: Some(child.clone()),
                task: "parent".to_string(),
                status: AgentStatus::Running,
                role: None,
                budget_tokens: None,
            })
            .expect("a missing parent is allowed while rebuilding persisted state");

        let error = graph
            .try_insert(AgentMetadata {
                id: child.clone(),
                parent_id: Some(parent),
                task: "child".to_string(),
                status: AgentStatus::Running,
                role: None,
                budget_tokens: None,
            })
            .expect_err("cycle should be rejected");

        assert!(format!("{error}").contains("cycle detected"));
        assert!(!graph.agents.contains_key(&child));
    }

    #[test]
    fn graph_rebuilds_from_persisted_session_metadata() {
        let root = AgentMetadata {
            id: AgentId("root".to_string()),
            parent_id: None,
            task: "root task".to_string(),
            status: AgentStatus::Completed,
            role: Some(AgentRole::General),
            budget_tokens: None,
        };
        let child = AgentMetadata {
            id: AgentId("child".to_string()),
            parent_id: Some(root.id.clone()),
            task: "child task".to_string(),
            status: AgentStatus::Running,
            role: Some(AgentRole::Explorer),
            budget_tokens: Some(500),
        };

        let graph = AgentGraph::from_session_metadata([
            AgentGraphSessionMetadata::new("session-root", root.clone()),
            AgentGraphSessionMetadata::new("session-child", child.clone()),
        ])
        .expect("session metadata should rebuild graph");

        assert_eq!(graph.children_of(&root.id), vec![child]);
    }

    #[test]
    fn registry_can_spawn_and_run_child_fixture_runtime() {
        let registry = InMemoryAgentRegistry::default();
        let result = registry
            .execute(MultiAgentCommand::SpawnRun {
                task: "review runtime".to_string(),
                parent_id: None,
            })
            .expect("spawn run");

        assert_eq!(result.status, AgentStatus::Completed);
        assert_eq!(result.agents[0].status, AgentStatus::Completed);
        assert!(
            result
                .child_run
                .as_ref()
                .and_then(|run| run.final_response.as_deref())
                .is_some_and(|response| response.contains("review runtime"))
        );
        assert!(
            registry
                .events()
                .expect("events")
                .iter()
                .any(|event| { matches!(event, MultiAgentLifecycleEvent::ChildRunStarted { .. }) })
        );
    }
}

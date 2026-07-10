use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use yunxi_agent_core::{AgentError, AgentResult};

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
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MultiAgentCommand {
    Spawn {
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

#[derive(Clone, Debug, Default)]
pub struct InMemoryAgentRegistry {
    agents: Arc<Mutex<BTreeMap<AgentId, AgentMetadata>>>,
}

impl InMemoryAgentRegistry {
    pub fn insert(&self, metadata: AgentMetadata) -> AgentResult<()> {
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

    fn lock_agents(
        &self,
    ) -> AgentResult<std::sync::MutexGuard<'_, BTreeMap<AgentId, AgentMetadata>>> {
        self.agents.lock().map_err(|_| AgentError::Execution {
            message: "multi-agent registry lock was poisoned".to_string(),
        })
    }
}

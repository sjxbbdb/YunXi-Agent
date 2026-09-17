use std::path::{Path, PathBuf};
use yunxi_agent_core::{AgentError, AgentResult};
use yunxi_agent_persona::{CONVERSATION_STATE_SCHEMA_VERSION, ConversationState};

const MAX_CONVERSATION_STATE_BYTES: u64 = 64 * 1024;
const MAX_CONVERSATION_STATE_ITEMS: usize = 16;
const MAX_CONVERSATION_STATE_TEXT_CHARS: usize = 512;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileConversationStateStore {
    root: PathBuf,
}

impl FileConversationStateStore {
    pub fn for_workspace(cwd: impl AsRef<Path>) -> Self {
        Self {
            root: cwd.as_ref().join(".yunxi").join("conversation-state"),
        }
    }

    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn load(&self, session_id: &str) -> AgentResult<Option<ConversationState>> {
        let path = self.path_for_session(session_id);
        let metadata = match std::fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(conversation_state_io_error(&path, "inspect", error)),
        };
        if !metadata.is_file() {
            return Err(AgentError::Execution {
                message: format!(
                    "conversation state path is not a regular file: {}",
                    path.display()
                ),
            });
        }
        if metadata.len() > MAX_CONVERSATION_STATE_BYTES {
            return Err(AgentError::Execution {
                message: format!(
                    "conversation state {} exceeds the {} byte limit",
                    path.display(),
                    MAX_CONVERSATION_STATE_BYTES
                ),
            });
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|error| conversation_state_io_error(&path, "read", error))?;
        let state = serde_json::from_str::<ConversationState>(&content).map_err(|error| {
            AgentError::Execution {
                message: format!(
                    "failed to parse conversation state {}: {error}",
                    path.display()
                ),
            }
        })?;
        if state.session_id != session_id {
            return Err(AgentError::Execution {
                message: format!(
                    "conversation state id mismatch in {}: expected {session_id}, got {}",
                    path.display(),
                    state.session_id
                ),
            });
        }
        validate_conversation_state(&state)?;
        Ok(Some(state))
    }

    pub fn load_active(
        &self,
        session_id: &str,
        now_millis: u128,
    ) -> AgentResult<Option<ConversationState>> {
        Ok(self
            .load(session_id)?
            .filter(|state| state.is_active_at(now_millis)))
    }

    pub fn save(&self, state: &ConversationState) -> AgentResult<()> {
        validate_conversation_state(state)?;
        std::fs::create_dir_all(&self.root).map_err(|error| AgentError::Execution {
            message: format!(
                "failed to create conversation state directory {}: {error}",
                self.root.display()
            ),
        })?;
        let path = self.path_for_session(&state.session_id);
        let serialized =
            serde_json::to_string_pretty(state).map_err(|error| AgentError::Execution {
                message: format!("failed to serialize conversation state: {error}"),
            })?;
        std::fs::write(&path, format!("{serialized}\n")).map_err(|error| AgentError::Execution {
            message: format!(
                "failed to write conversation state {}: {error}",
                path.display()
            ),
        })
    }

    fn path_for_session(&self, session_id: &str) -> PathBuf {
        self.root
            .join(format!("{:016x}.json", fnv1a64(session_id.as_bytes())))
    }
}

fn validate_conversation_state(state: &ConversationState) -> AgentResult<()> {
    if state.schema_version != CONVERSATION_STATE_SCHEMA_VERSION {
        return Err(AgentError::Execution {
            message: format!(
                "unsupported conversation state schema {}; expected {}",
                state.schema_version, CONVERSATION_STATE_SCHEMA_VERSION
            ),
        });
    }
    validate_text("session_id", Some(&state.session_id))?;
    validate_text("parent_session_id", state.parent_session_id.as_deref())?;
    validate_text("current_topic", state.current_topic.as_deref())?;
    validate_text("response_tone", state.response_tone.as_deref())?;
    validate_text("emotional_context", state.emotional_context.as_deref())?;
    validate_text("last_user_message", state.last_user_message.as_deref())?;
    validate_text(
        "last_assistant_message",
        state.last_assistant_message.as_deref(),
    )?;
    validate_items("unresolved_intents", &state.unresolved_intents)?;
    validate_items("recent_entities", &state.recent_entities)?;
    if state.expires_at_millis < state.updated_at_millis {
        return Err(AgentError::Execution {
            message: "conversation state expiry precedes its update time".to_string(),
        });
    }
    Ok(())
}

fn validate_items(field: &str, values: &[String]) -> AgentResult<()> {
    if values.len() > MAX_CONVERSATION_STATE_ITEMS {
        return Err(AgentError::Execution {
            message: format!(
                "conversation state {field} exceeds the {} item limit",
                MAX_CONVERSATION_STATE_ITEMS
            ),
        });
    }
    for value in values {
        validate_text(field, Some(value))?;
    }
    Ok(())
}

fn validate_text(field: &str, value: Option<&str>) -> AgentResult<()> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.trim().is_empty() {
        return Err(AgentError::Execution {
            message: format!("conversation state {field} cannot be empty"),
        });
    }
    if value.chars().count() > MAX_CONVERSATION_STATE_TEXT_CHARS {
        return Err(AgentError::Execution {
            message: format!(
                "conversation state {field} exceeds the {} character limit",
                MAX_CONVERSATION_STATE_TEXT_CHARS
            ),
        });
    }
    Ok(())
}

fn conversation_state_io_error(path: &Path, action: &str, error: std::io::Error) -> AgentError {
    AgentError::Execution {
        message: format!(
            "failed to {action} conversation state {}: {error}",
            path.display()
        ),
    }
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn store_round_trips_active_state_and_hides_expired_state() {
        let temp = TempDir::new().expect("temp dir");
        let store = FileConversationStateStore::new(temp.path());
        let mut state = ConversationState::new("session-a", 1_000);
        state.current_topic = Some("继续完善记忆模块".to_string());
        state.expires_at_millis = 2_000;

        store.save(&state).expect("save state");
        assert_eq!(
            store.load_active("session-a", 1_500).expect("load state"),
            Some(state)
        );
        assert_eq!(
            store.load_active("session-a", 2_000).expect("load expired"),
            None
        );
    }

    #[test]
    fn store_rejects_future_schema_and_oversized_state_files() {
        let temp = TempDir::new().expect("temp dir");
        let store = FileConversationStateStore::new(temp.path());
        std::fs::create_dir_all(store.root()).expect("create root");

        let mut future = ConversationState::new("future", 1_000);
        future.schema_version = CONVERSATION_STATE_SCHEMA_VERSION + 1;
        std::fs::write(
            store.path_for_session("future"),
            serde_json::to_string(&future).expect("serialize future"),
        )
        .expect("write future");
        assert!(store.load("future").is_err());

        std::fs::write(
            store.path_for_session("oversized"),
            vec![b' '; MAX_CONVERSATION_STATE_BYTES as usize + 1],
        )
        .expect("write oversized");
        assert!(store.load("oversized").is_err());
    }
}

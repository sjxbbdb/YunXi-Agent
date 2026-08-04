use serde::{Deserialize, Serialize};

pub const COMPANION_MAILBOX_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MailboxItemType {
    LoveLetter,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MailboxState {
    Unread,
    Read,
    Archived,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompanionMailboxItem {
    pub schema_version: u32,
    pub item_id: String,
    pub owner_scope: String,
    pub item_type: MailboxItemType,
    pub source_id: String,
    pub source_revision: String,
    pub content_ref: String,
    pub state: MailboxState,
    pub created_at_millis: u128,
    pub available_at_millis: u128,
    pub updated_at_millis: u128,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_at_millis: Option<u128>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived_at_millis: Option<u128>,
}

impl CompanionMailboxItem {
    pub fn mark_state(&self, state: MailboxState, now_millis: u128) -> Self {
        let mut updated = self.clone();
        updated.state = state;
        updated.updated_at_millis = now_millis;
        match state {
            MailboxState::Unread => {
                updated.read_at_millis = None;
                updated.archived_at_millis = None;
            }
            MailboxState::Read => {
                updated.read_at_millis = Some(now_millis);
                updated.archived_at_millis = None;
            }
            MailboxState::Archived => {
                updated.archived_at_millis = Some(now_millis);
            }
        }
        updated
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompanionMailboxContent {
    pub subject: String,
    pub body: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompanionMailboxEntry {
    pub item: CompanionMailboxItem,
    pub content: CompanionMailboxContent,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MailboxQuery {
    pub state: Option<MailboxState>,
    pub item_type: Option<MailboxItemType>,
    pub cursor: Option<MailboxCursor>,
    pub limit: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MailboxCursor {
    pub available_at_millis: u128,
    pub item_id: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MailboxPage {
    pub items: Vec<CompanionMailboxItem>,
    pub next_cursor: Option<MailboxCursor>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item() -> CompanionMailboxItem {
        CompanionMailboxItem {
            schema_version: COMPANION_MAILBOX_SCHEMA_VERSION,
            item_id: "mail-1".to_string(),
            owner_scope: "workspace-1".to_string(),
            item_type: MailboxItemType::LoveLetter,
            source_id: "letter-1".to_string(),
            source_revision: "memory-r1".to_string(),
            content_ref: "content-1".to_string(),
            state: MailboxState::Unread,
            created_at_millis: 10,
            available_at_millis: 10,
            updated_at_millis: 10,
            read_at_millis: None,
            archived_at_millis: None,
        }
    }

    #[test]
    fn mailbox_state_transitions_keep_read_and_archive_timestamps_coherent() {
        let read = item().mark_state(MailboxState::Read, 20);
        assert_eq!(read.read_at_millis, Some(20));
        let archived = read.mark_state(MailboxState::Archived, 30);
        assert_eq!(archived.read_at_millis, Some(20));
        assert_eq!(archived.archived_at_millis, Some(30));
        let unread = archived.mark_state(MailboxState::Unread, 40);
        assert_eq!(unread.read_at_millis, None);
        assert_eq!(unread.archived_at_millis, None);
    }
}

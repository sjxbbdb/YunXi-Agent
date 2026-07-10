use std::path::PathBuf;
use tempfile::TempDir;
use yunxi_agent_core::{AgentEvent, AgentRunStatus};
use yunxi_agent_storage::{
    FileSessionStore, InMemorySessionStore, RolloutRecord, SessionId, SessionRecord, SessionStore,
    ThreadMetadata,
};

#[tokio::test]
async fn in_memory_session_store_saves_loads_and_lists_records() {
    let store = InMemorySessionStore::default();
    let record = SessionRecord::new(
        PathBuf::from("."),
        "remember this",
        Some("done".to_string()),
        vec![AgentEvent::Completed {
            status: AgentRunStatus::Completed,
            usage: None,
        }],
    );

    let id = store
        .save(record.clone())
        .await
        .expect("session save should succeed");

    assert_eq!(
        store.load(&id).await.expect("session load should succeed"),
        Some(record.clone())
    );
    assert_eq!(
        store.list().await.expect("session list should succeed"),
        vec![record]
    );
}

#[tokio::test]
async fn file_session_store_persists_records_across_instances() {
    let temp = TempDir::new().expect("temp dir");
    let store = FileSessionStore::new(temp.path().join("sessions"));
    let record = SessionRecord::new(
        PathBuf::from("."),
        "persist this",
        Some("done".to_string()),
        vec![AgentEvent::Completed {
            status: AgentRunStatus::Completed,
            usage: None,
        }],
    );

    let id = store
        .save(record.clone())
        .await
        .expect("session save should succeed");
    let reloaded_store = FileSessionStore::new(temp.path().join("sessions"));

    assert_eq!(
        reloaded_store
            .load(&id)
            .await
            .expect("session load should succeed"),
        Some(record.clone())
    );
    assert_eq!(
        reloaded_store
            .list()
            .await
            .expect("session list should succeed"),
        vec![record]
    );
}

#[test]
fn rollout_record_can_be_reconstructed_from_session_record() {
    let parent_id = SessionId::new("parent-thread");
    let record = SessionRecord::new(
        ".",
        "explain",
        Some("done".to_string()),
        vec![AgentEvent::Message {
            content: "done".to_string(),
        }],
    )
    .with_parent_id(parent_id.clone())
    .with_title("Explain thread")
    .with_archived(true)
    .with_pinned(true);

    let rollout = RolloutRecord::from(record);

    assert_eq!(rollout.prompt, "explain");
    assert_eq!(rollout.items.len(), 1);
    assert_eq!(rollout.final_response.as_deref(), Some("done"));
    assert_eq!(rollout.thread.parent_id, Some(parent_id));
    assert_eq!(rollout.thread.title.as_deref(), Some("Explain thread"));
    assert!(rollout.thread.archived);
    assert!(rollout.thread.pinned);
}

#[test]
fn thread_metadata_tracks_archive_and_pin_flags() {
    let metadata = ThreadMetadata::new(yunxi_agent_storage::SessionId::new("thread-1"), ".")
        .archived(true)
        .pinned(true);

    assert!(metadata.archived);
    assert!(metadata.pinned);
}

#[tokio::test]
async fn file_session_store_updates_thread_lifecycle_fields() {
    let temp = TempDir::new().expect("temp dir");
    let store = FileSessionStore::new(temp.path().join("sessions"));
    let record = SessionRecord::new(
        PathBuf::from("."),
        "manage lifecycle",
        Some("done".to_string()),
        vec![AgentEvent::Completed {
            status: AgentRunStatus::Completed,
            usage: None,
        }],
    )
    .with_title("Lifecycle thread");
    let id = store
        .save(record)
        .await
        .expect("session save should succeed");

    let archived = store
        .archive(&id, true)
        .await
        .expect("archive should succeed")
        .expect("session should exist");
    assert!(archived.archived);

    let pinned = store
        .pin(&id, true)
        .await
        .expect("pin should succeed")
        .expect("session should exist");
    assert!(pinned.pinned);

    let forked = store
        .fork(&id)
        .await
        .expect("fork should succeed")
        .expect("session should exist");
    assert_ne!(forked.id, id);
    assert_eq!(forked.parent_id, Some(id.clone()));
    assert_eq!(forked.prompt, "manage lifecycle");
    assert!(!forked.archived);
    assert!(!forked.pinned);

    let sessions = store.list().await.expect("session list should succeed");
    assert_eq!(sessions.len(), 2);
    assert!(
        sessions
            .iter()
            .any(|session| session.id == id && session.archived && session.pinned)
    );
    assert!(
        sessions
            .iter()
            .any(|session| session.parent_id == Some(id.clone()))
    );
}

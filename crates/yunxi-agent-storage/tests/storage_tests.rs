use std::path::PathBuf;
use tempfile::TempDir;
use yunxi_agent_core::{AgentEvent, AgentRunStatus};
use yunxi_agent_persona::{MemoryKind, MemoryRecord, MemoryScope, MemoryStatus};
use yunxi_agent_storage::{
    FilePersonaMemoryStore, FileSessionStore, HistoryItemKind, HistoryLoadOptions,
    InMemorySessionStore, MemoryPersistOutcome, PersonaMemoryScope, RolloutRecord, SessionId,
    SessionRecord, SessionStore, ThreadMetadata,
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

#[tokio::test]
async fn session_store_reconstructs_parent_history_from_root_to_child() {
    let store = InMemorySessionStore::default();

    let mut root = SessionRecord::new(".", "root prompt", Some("root answer".to_string()), vec![]);
    root.id = SessionId::new("root");
    store.save(root.clone()).await.expect("save root");

    let mut child = SessionRecord::new(
        ".",
        "child prompt",
        Some("child answer".to_string()),
        vec![],
    )
    .with_parent_id(root.id.clone());
    child.id = SessionId::new("child");
    store.save(child.clone()).await.expect("save child");

    let history = store
        .history(&child.id, HistoryLoadOptions::default())
        .await
        .expect("history load should succeed")
        .expect("history should exist");

    assert_eq!(
        history
            .sessions
            .iter()
            .map(|session| session.id.0.as_str())
            .collect::<Vec<_>>(),
        vec!["root", "child"]
    );
    assert_eq!(
        history
            .items
            .iter()
            .map(|item| (item.kind, item.content.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (HistoryItemKind::User, "root prompt"),
            (HistoryItemKind::Assistant, "root answer"),
            (HistoryItemKind::User, "child prompt"),
            (HistoryItemKind::Assistant, "child answer"),
        ]
    );
}

#[tokio::test]
async fn session_store_rejects_cyclic_parent_history() {
    let store = InMemorySessionStore::default();

    let mut left = SessionRecord::new(".", "left", Some("left answer".to_string()), vec![]);
    left.id = SessionId::new("left");
    left.parent_id = Some(SessionId::new("right"));

    let mut right = SessionRecord::new(".", "right", Some("right answer".to_string()), vec![]);
    right.id = SessionId::new("right");
    right.parent_id = Some(SessionId::new("left"));

    store.save(left).await.expect("save left");
    store.save(right).await.expect("save right");

    let error = store
        .history(&SessionId::new("left"), HistoryLoadOptions::default())
        .await
        .expect_err("cycle should be rejected");

    assert!(format!("{error}").contains("cycle detected"));
}

#[test]
fn file_persona_memory_store_appends_lists_and_updates_status() {
    let temp = TempDir::new().expect("temp dir");
    let store = FilePersonaMemoryStore::for_workspace(temp.path());
    let record = MemoryRecord::new(
        "memory-1",
        MemoryScope::Workspace {
            root_fingerprint: store.workspace_fingerprint().to_string(),
        },
        MemoryKind::Preference,
        "用户偏好使用中文回答。",
        1,
    )
    .with_status(MemoryStatus::Active);

    store.append(&record).expect("append memory");

    let loaded = store.list(PersonaMemoryScope::Workspace);
    assert!(loaded.warnings.is_empty());
    assert_eq!(loaded.records.len(), 1);
    assert_eq!(loaded.records[0].id, "memory-1");

    let updated = store
        .update_status("memory-1", MemoryStatus::Archived)
        .expect("update status")
        .expect("record should exist");
    assert_eq!(updated.status, MemoryStatus::Archived);
    assert_eq!(
        store
            .list(PersonaMemoryScope::Workspace)
            .records
            .first()
            .map(|record| record.status),
        Some(MemoryStatus::Archived)
    );
}

#[test]
fn file_persona_memory_store_skips_corrupt_jsonl_lines() {
    let temp = TempDir::new().expect("temp dir");
    let store = FilePersonaMemoryStore::for_workspace(temp.path());
    let memory_dir = temp.path().join(".yunxi").join("memory");
    std::fs::create_dir_all(&memory_dir).expect("memory dir");
    std::fs::write(
        memory_dir.join("workspace-memory.jsonl"),
        "{not-json}\n{\"id\":\"memory-2\",\"schema_version\":1,\"scope\":{\"workspace\":{\"root_fingerprint\":\"x\"}},\"kind\":\"preference\",\"content\":\"ok\",\"confidence\":1.0,\"importance\":1.0,\"sensitivity\":\"low\",\"status\":\"active\",\"created_at_millis\":1,\"updated_at_millis\":1}\n",
    )
    .expect("write fixture");

    let loaded = store.list(PersonaMemoryScope::Workspace);

    assert_eq!(loaded.records.len(), 1);
    assert_eq!(loaded.records[0].id, "memory-2");
    assert_eq!(loaded.records[0].schema_version, 2);
    assert!(!loaded.records[0].dedup_key.is_empty());
    assert_eq!(loaded.warnings.len(), 1);
}

#[test]
fn file_persona_memory_store_merges_equivalent_active_language_preferences() {
    let temp = TempDir::new().expect("temp dir");
    let store = FilePersonaMemoryStore::for_workspace(temp.path());
    let first = MemoryRecord::new(
        "memory-first",
        MemoryScope::Workspace {
            root_fingerprint: store.workspace_fingerprint().to_string(),
        },
        MemoryKind::Preference,
        "用户偏好使用中文回答。",
        1,
    )
    .with_status(MemoryStatus::Active);
    let second = MemoryRecord::new(
        "memory-second",
        MemoryScope::Workspace {
            root_fingerprint: store.workspace_fingerprint().to_string(),
        },
        MemoryKind::Preference,
        "用户偏好默认中文交流。",
        2,
    )
    .with_status(MemoryStatus::Active);

    let inserted = store.append_or_merge(&first).expect("insert");
    let merged = store.append_or_merge(&second).expect("merge");

    assert!(matches!(inserted, MemoryPersistOutcome::Inserted { .. }));
    assert!(matches!(
        merged,
        MemoryPersistOutcome::Merged {
            revision: 2,
            merged_count: 2,
            ..
        }
    ));
    let loaded = store.list(PersonaMemoryScope::Workspace);
    assert_eq!(loaded.records.len(), 1);
    assert_eq!(loaded.records[0].id, "memory-first");
    assert_eq!(loaded.records[0].revision, 2);
    assert_eq!(loaded.records[0].merged_count, 2);
}

#[test]
fn file_persona_memory_store_preserves_rich_existing_when_generic_language_preference_repeats() {
    let temp = TempDir::new().expect("temp dir");
    let store = FilePersonaMemoryStore::for_workspace(temp.path());
    let rich = MemoryRecord::new(
        "memory-rich",
        MemoryScope::Workspace {
            root_fingerprint: store.workspace_fingerprint().to_string(),
        },
        MemoryKind::Preference,
        "用户偏好使用中文回答，并且回答要简洁、保留关键细节。",
        1,
    )
    .with_status(MemoryStatus::Active);
    let generic = MemoryRecord::new(
        "memory-generic",
        MemoryScope::Workspace {
            root_fingerprint: store.workspace_fingerprint().to_string(),
        },
        MemoryKind::Preference,
        "用户偏好后续默认使用中文交流。",
        2,
    )
    .with_status(MemoryStatus::Active);

    store.append_or_merge(&rich).expect("insert rich");
    let merged = store.append_or_merge(&generic).expect("merge generic");

    assert!(matches!(
        merged,
        MemoryPersistOutcome::Merged {
            revision: 2,
            merged_count: 2,
            merge_strategy,
            ..
        } if merge_strategy == "preserve_existing"
    ));
    let loaded = store.list(PersonaMemoryScope::Workspace);
    assert_eq!(loaded.records.len(), 1);
    assert_eq!(loaded.records[0].id, "memory-rich");
    assert!(loaded.records[0].content.contains("简洁"));
    assert!(loaded.records[0].content.contains("保留关键细节"));
}

#[test]
fn file_persona_memory_store_promotes_rich_incoming_over_generic_existing() {
    let temp = TempDir::new().expect("temp dir");
    let store = FilePersonaMemoryStore::for_workspace(temp.path());
    let generic = MemoryRecord::new(
        "memory-generic",
        MemoryScope::Workspace {
            root_fingerprint: store.workspace_fingerprint().to_string(),
        },
        MemoryKind::Preference,
        "用户偏好使用中文回答。",
        1,
    )
    .with_status(MemoryStatus::Active);
    let rich = MemoryRecord::new(
        "memory-rich",
        MemoryScope::Workspace {
            root_fingerprint: store.workspace_fingerprint().to_string(),
        },
        MemoryKind::Preference,
        "用户偏好使用中文回答，并且回答要简洁、保留关键细节。",
        2,
    )
    .with_status(MemoryStatus::Active);

    store.append_or_merge(&generic).expect("insert generic");
    let merged = store.append_or_merge(&rich).expect("merge rich");

    assert!(matches!(
        merged,
        MemoryPersistOutcome::Merged {
            merge_strategy,
            ..
        } if merge_strategy == "promote_incoming"
    ));
    let loaded = store.list(PersonaMemoryScope::Workspace);
    assert_eq!(loaded.records.len(), 1);
    assert_eq!(loaded.records[0].id, "memory-generic");
    assert!(loaded.records[0].content.contains("简洁"));
    assert!(loaded.records[0].content.contains("保留关键细节"));
}

#[test]
fn file_persona_memory_store_combines_non_conflicting_same_slot_details() {
    let temp = TempDir::new().expect("temp dir");
    let store = FilePersonaMemoryStore::for_workspace(temp.path());
    let concise = MemoryRecord::new(
        "memory-concise",
        MemoryScope::Workspace {
            root_fingerprint: store.workspace_fingerprint().to_string(),
        },
        MemoryKind::Preference,
        "用户偏好使用中文回答，并且回答要简洁。",
        1,
    )
    .with_status(MemoryStatus::Active);
    let details = MemoryRecord::new(
        "memory-details",
        MemoryScope::Workspace {
            root_fingerprint: store.workspace_fingerprint().to_string(),
        },
        MemoryKind::Preference,
        "用户偏好使用中文回答，并且保留关键细节。",
        2,
    )
    .with_status(MemoryStatus::Active);

    store.append_or_merge(&concise).expect("insert concise");
    let merged = store.append_or_merge(&details).expect("merge details");

    assert!(matches!(
        merged,
        MemoryPersistOutcome::Merged {
            merge_strategy,
            ..
        } if merge_strategy == "combine_non_conflicting"
    ));
    let loaded = store.list(PersonaMemoryScope::Workspace);
    assert_eq!(loaded.records.len(), 1);
    assert!(loaded.records[0].content.contains("简洁"));
    assert!(loaded.records[0].content.contains("保留关键细节"));
}

#[test]
fn file_persona_memory_store_routes_language_conflict_to_pending() {
    let temp = TempDir::new().expect("temp dir");
    let store = FilePersonaMemoryStore::for_workspace(temp.path());
    let chinese = MemoryRecord::new(
        "memory-zh",
        MemoryScope::Workspace {
            root_fingerprint: store.workspace_fingerprint().to_string(),
        },
        MemoryKind::Preference,
        "用户偏好使用中文回答。",
        1,
    )
    .with_status(MemoryStatus::Active);
    let english = MemoryRecord::new(
        "memory-en",
        MemoryScope::Workspace {
            root_fingerprint: store.workspace_fingerprint().to_string(),
        },
        MemoryKind::Preference,
        "用户偏好使用英文回答。",
        2,
    )
    .with_status(MemoryStatus::Active);

    store.append_or_merge(&chinese).expect("insert chinese");
    let conflict = store.append_or_merge(&english).expect("pending conflict");

    assert!(matches!(
        conflict,
        MemoryPersistOutcome::ConflictPending {
            status: MemoryStatus::Pending,
            conflict_family,
            ..
        } if conflict_family.ends_with("|preference|language")
    ));
    let active = store
        .list(PersonaMemoryScope::Workspace)
        .records
        .into_iter()
        .filter(|record| record.status == MemoryStatus::Active)
        .collect::<Vec<_>>();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].id, "memory-zh");
    let pending = store
        .list(PersonaMemoryScope::Workspace)
        .records
        .into_iter()
        .filter(|record| record.status == MemoryStatus::Pending)
        .collect::<Vec<_>>();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, "memory-en");
}

#[test]
fn file_persona_memory_store_keeps_pending_when_equivalent_pending_repeats() {
    let temp = TempDir::new().expect("temp dir");
    let store = FilePersonaMemoryStore::for_workspace(temp.path());
    let first = MemoryRecord::new(
        "pending-first",
        MemoryScope::Workspace {
            root_fingerprint: store.workspace_fingerprint().to_string(),
        },
        MemoryKind::PersonalFact,
        "用户自述事实候选：我的名字是 YunXi 测试用户。",
        1,
    );
    let second = MemoryRecord::new(
        "pending-second",
        MemoryScope::Workspace {
            root_fingerprint: store.workspace_fingerprint().to_string(),
        },
        MemoryKind::PersonalFact,
        "用户自述事实候选：我的名字是 YunXi 测试用户。",
        2,
    );

    store.append_or_merge(&first).expect("insert pending");
    store.append_or_merge(&second).expect("merge pending");

    let pending = store.list(PersonaMemoryScope::Workspace);
    assert_eq!(pending.records.len(), 1);
    assert_eq!(pending.records[0].id, "pending-first");
    assert_eq!(pending.records[0].status, MemoryStatus::Pending);
    assert_eq!(pending.records[0].revision, 2);
}

#[test]
fn archived_and_rejected_equivalent_records_are_not_auto_revived() {
    let temp = TempDir::new().expect("temp dir");
    let store = FilePersonaMemoryStore::for_workspace(temp.path());
    let archived = MemoryRecord::new(
        "archived-language",
        MemoryScope::Workspace {
            root_fingerprint: store.workspace_fingerprint().to_string(),
        },
        MemoryKind::Preference,
        "用户偏好使用中文回答。",
        1,
    )
    .with_status(MemoryStatus::Archived);
    let rejected = MemoryRecord::new(
        "rejected-language",
        MemoryScope::Workspace {
            root_fingerprint: store.workspace_fingerprint().to_string(),
        },
        MemoryKind::Preference,
        "用户偏好默认中文交流。",
        2,
    )
    .with_status(MemoryStatus::Rejected);
    let active = MemoryRecord::new(
        "active-language",
        MemoryScope::Workspace {
            root_fingerprint: store.workspace_fingerprint().to_string(),
        },
        MemoryKind::Preference,
        "用户偏好用中文回复我。",
        3,
    )
    .with_status(MemoryStatus::Active);

    store.append(&archived).expect("append archived");
    store.append(&rejected).expect("append rejected");
    store.append_or_merge(&active).expect("insert active");

    let loaded = store.list(PersonaMemoryScope::Workspace);
    assert!(loaded.records.iter().any(|record| {
        record.id == "archived-language" && record.status == MemoryStatus::Archived
    }));
    assert!(loaded.records.iter().any(|record| {
        record.id == "rejected-language" && record.status == MemoryStatus::Rejected
    }));
    assert!(loaded.records.iter().any(|record| {
        record.id == "active-language"
            && record.status == MemoryStatus::Active
            && record.revision == 1
    }));
}

#[test]
fn file_persona_memory_store_migrates_v1_and_legacy_jsonl_records() {
    let temp = TempDir::new().expect("temp dir");
    let store = FilePersonaMemoryStore::for_workspace(temp.path());
    let workspace_dir = temp.path().join(".yunxi").join("memory");
    std::fs::create_dir_all(&workspace_dir).expect("workspace memory dir");
    let fixture = format!(
        "{}\n{}\n{}\n",
        include_str!("fixtures/memory-v1-workspace-project-context.jsonl").trim(),
        "{\"id\":\"legacy-project\",\"scope\":{\"workspace\":{\"root_fingerprint\":\"workspace-fixture\"}},\"kind\":\"project_context\",\"content\":\"项目要求使用 REST API。\",\"confidence\":0.8,\"importance\":0.6,\"sensitivity\":\"low\",\"status\":\"active\",\"created_at_millis\":2,\"updated_at_millis\":2}",
        "{\"id\":\"future\",\"schema_version\":99,\"scope\":{\"workspace\":{\"root_fingerprint\":\"workspace-fixture\"}},\"kind\":\"project_context\",\"content\":\"future\",\"confidence\":0.8,\"importance\":0.6,\"sensitivity\":\"low\",\"status\":\"active\",\"created_at_millis\":3,\"updated_at_millis\":3}",
    );
    std::fs::write(workspace_dir.join("workspace-memory.jsonl"), fixture)
        .expect("write migration fixture");

    let loaded = store.list(PersonaMemoryScope::Workspace);

    assert!(loaded.records.iter().all(|record| {
        record.schema_version == 2
            && !record.dedup_key.is_empty()
            && record.revision >= 1
            && record.merged_count >= 1
    }));
    assert!(
        loaded
            .warnings
            .iter()
            .any(|warning| warning.contains("missing schema_version"))
    );
    assert!(
        loaded
            .warnings
            .iter()
            .any(|warning| warning.contains("unsupported future memory schema_version=99"))
    );
}

#[test]
fn pending_scope_uses_latest_status_after_approve_reject_and_archive() {
    let temp = TempDir::new().expect("temp dir");
    let store = FilePersonaMemoryStore::for_workspace(temp.path());
    let pending = MemoryRecord::new(
        "pending-1",
        MemoryScope::Workspace {
            root_fingerprint: store.workspace_fingerprint().to_string(),
        },
        MemoryKind::PersonalFact,
        "用户自述事实候选。",
        1,
    );

    store.append(&pending).expect("append pending");
    assert!(
        store
            .pending_records()
            .records
            .iter()
            .any(|record| record.id == "pending-1")
    );

    store
        .update_status("pending-1", MemoryStatus::Active)
        .expect("approve")
        .expect("record");
    assert!(
        !store
            .pending_records()
            .records
            .iter()
            .any(|record| record.id == "pending-1")
    );

    store
        .update_status("pending-1", MemoryStatus::Archived)
        .expect("archive")
        .expect("record");
    assert!(
        !store
            .pending_records()
            .records
            .iter()
            .any(|record| record.id == "pending-1")
    );
}

#[test]
fn clear_workspace_archives_active_and_pending_workspace_records_only() {
    let temp = TempDir::new().expect("temp dir");
    let store = FilePersonaMemoryStore::for_workspace(temp.path());
    let workspace_active = MemoryRecord::new(
        "workspace-active",
        MemoryScope::Workspace {
            root_fingerprint: store.workspace_fingerprint().to_string(),
        },
        MemoryKind::ProjectContext,
        "项目要求使用 GitHub API。",
        1,
    )
    .with_status(MemoryStatus::Active);
    let workspace_pending = MemoryRecord::new(
        "workspace-pending",
        MemoryScope::Workspace {
            root_fingerprint: store.workspace_fingerprint().to_string(),
        },
        MemoryKind::PersonalFact,
        "当前项目 pending 候选。",
        2,
    );

    store.append(&workspace_active).expect("append active");
    store
        .append(&workspace_pending)
        .expect("append workspace pending");

    let summary = store.clear_workspace().expect("clear workspace");

    assert_eq!(summary.archived_active_records, 1);
    assert_eq!(summary.archived_pending_records, 1);
    assert_eq!(summary.remaining_pending_records, 0);
}

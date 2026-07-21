use crate::presentation::{TuiCellId, TuiEvent, TuiSourceSequence, TuiStreamPhase};
use crate::streaming::{MarkdownStreamController, bounded_stream_content};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const ARCHIVED_SESSION_LIMIT: usize = 256;
const SEEN_EVENT_LIMIT: usize = 8_192;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct TurnId(String);

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct StreamSessionId(String);

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct StreamKey {
    turn_id: TurnId,
    stream_id: StreamSessionId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StreamSessionState {
    Active,
    Retrying,
    Finalized,
    Cancelled,
    Superseded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StreamSession {
    cell_id: TuiCellId,
    last_reliable_sequence: Option<u64>,
    controller: MarkdownStreamController,
    content: String,
    state: StreamSessionState,
    offline_label: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ArchivedStreamSession {
    cell_id: TuiCellId,
    last_reliable_sequence: Option<u64>,
    terminal_event_hash: u64,
    state: StreamSessionState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AssistantTimelineUpdate {
    pub(crate) cell_id: TuiCellId,
    pub(crate) content: String,
    pub(crate) active: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct TimelineStore {
    sessions: BTreeMap<StreamKey, StreamSession>,
    last_by_turn: BTreeMap<TurnId, StreamKey>,
    archived: BTreeMap<StreamKey, ArchivedStreamSession>,
    archive_order: VecDeque<StreamKey>,
    seen_event_ids: BTreeSet<String>,
    seen_event_order: VecDeque<String>,
    duplicate_event_count: u64,
}

impl TimelineStore {
    pub(crate) fn apply(&mut self, event: &mut TuiEvent) -> Option<AssistantTimelineUpdate> {
        let identity = event.stream.as_ref()?.identity.clone();
        if !self.record_event_id(&identity.event_id) {
            self.duplicate_event_count = self.duplicate_event_count.saturating_add(1);
            return None;
        }

        let key = StreamKey {
            turn_id: TurnId(identity.turn_id),
            stream_id: StreamSessionId(identity.stream_id),
        };
        if self.archived.contains_key(&key) {
            return None;
        }

        match identity.phase {
            TuiStreamPhase::Started => self.start(event, key, identity.source_sequence, false),
            TuiStreamPhase::Delta => self.delta(event, key, identity.source_sequence),
            TuiStreamPhase::Retry => self.start(event, key, identity.source_sequence, true),
            TuiStreamPhase::Final => {
                self.finalize(event, key, identity.source_sequence, &identity.event_id)
            }
            TuiStreamPhase::Finish => self.finish(
                key,
                identity.source_sequence,
                StreamSessionState::Finalized,
                &identity.event_id,
            ),
            TuiStreamPhase::Cancel => self.finish(
                key,
                identity.source_sequence,
                StreamSessionState::Cancelled,
                &identity.event_id,
            ),
        }
    }

    pub(crate) fn finish_active(&mut self) -> Vec<AssistantTimelineUpdate> {
        self.finish_all(StreamSessionState::Finalized)
    }

    pub(crate) fn cancel_active(&mut self) -> Vec<AssistantTimelineUpdate> {
        self.finish_all(StreamSessionState::Cancelled)
    }

    pub(crate) fn has_active_sessions(&self) -> bool {
        !self.sessions.is_empty()
    }

    pub(crate) fn duplicate_event_count(&self) -> u64 {
        self.duplicate_event_count
    }

    pub(crate) fn clear(&mut self) {
        self.sessions.clear();
        self.last_by_turn.clear();
        self.archived.clear();
        self.archive_order.clear();
        self.seen_event_ids.clear();
        self.seen_event_order.clear();
        self.duplicate_event_count = 0;
    }

    fn start(
        &mut self,
        event: &mut TuiEvent,
        key: StreamKey,
        sequence: TuiSourceSequence,
        explicit_retry: bool,
    ) -> Option<AssistantTimelineUpdate> {
        self.ensure_session(event, &key, explicit_retry);
        let session = self.sessions.get_mut(&key)?;
        if reliable_sequence_is_late(session.last_reliable_sequence, sequence) {
            return None;
        }
        observe_reliable_sequence(&mut session.last_reliable_sequence, sequence);
        session.state = if explicit_retry {
            StreamSessionState::Retrying
        } else {
            StreamSessionState::Active
        };
        session.controller.clear();
        let frame = session.controller.push_delta(&event.visible_text);
        session.content = bounded_stream_content(&frame.stable_source, &frame.live_tail);
        apply_offline_label(session);
        update_stream_frame(
            event,
            &frame.stable_source,
            &frame.live_tail,
            frame.committed,
        );
        self.last_by_turn.insert(key.turn_id.clone(), key);

        (!session.content.is_empty()).then(|| update_for(session, true))
    }

    fn delta(
        &mut self,
        event: &mut TuiEvent,
        key: StreamKey,
        sequence: TuiSourceSequence,
    ) -> Option<AssistantTimelineUpdate> {
        self.ensure_session(event, &key, false);
        let session = self.sessions.get_mut(&key)?;
        if reliable_sequence_is_late(session.last_reliable_sequence, sequence)
            || !matches!(
                session.state,
                StreamSessionState::Active | StreamSessionState::Retrying
            )
        {
            return None;
        }
        observe_reliable_sequence(&mut session.last_reliable_sequence, sequence);
        let frame = session.controller.push_delta(&event.visible_text);
        session.content = bounded_stream_content(&frame.stable_source, &frame.live_tail);
        apply_offline_label(session);
        update_stream_frame(
            event,
            &frame.stable_source,
            &frame.live_tail,
            frame.committed,
        );
        self.last_by_turn.insert(key.turn_id.clone(), key);
        Some(update_for(session, true))
    }

    fn finalize(
        &mut self,
        event: &mut TuiEvent,
        key: StreamKey,
        sequence: TuiSourceSequence,
        event_id: &str,
    ) -> Option<AssistantTimelineUpdate> {
        self.ensure_session(event, &key, false);
        let session = self.sessions.get_mut(&key)?;
        if reliable_sequence_is_late(session.last_reliable_sequence, sequence) {
            return None;
        }
        observe_reliable_sequence(&mut session.last_reliable_sequence, sequence);
        session.controller.clear();
        if !event.visible_text.is_empty() {
            let frame = session.controller.push_delta(&event.visible_text);
            session.content = bounded_stream_content(&frame.stable_source, &frame.live_tail);
            update_stream_frame(
                event,
                &frame.stable_source,
                &frame.live_tail,
                frame.committed,
            );
        }
        let _ = session.controller.finalize();
        apply_offline_label(session);
        session.state = StreamSessionState::Finalized;
        let update = update_for(session, false);
        let session = self.sessions.remove(&key).expect("finalized session");
        self.archive_session(key, session, StreamSessionState::Finalized, event_id);
        Some(update)
    }

    fn finish(
        &mut self,
        key: StreamKey,
        sequence: TuiSourceSequence,
        state: StreamSessionState,
        event_id: &str,
    ) -> Option<AssistantTimelineUpdate> {
        let session = self.sessions.get_mut(&key)?;
        if reliable_sequence_is_late(session.last_reliable_sequence, sequence) {
            return None;
        }
        observe_reliable_sequence(&mut session.last_reliable_sequence, sequence);
        let _ = session.controller.finalize();
        session.state = state;
        let update = update_for(session, false);
        let session = self.sessions.remove(&key).expect("finished session");
        self.archive_session(key, session, state, event_id);
        Some(update)
    }

    fn finish_all(&mut self, state: StreamSessionState) -> Vec<AssistantTimelineUpdate> {
        let keys = self.sessions.keys().cloned().collect::<Vec<_>>();
        keys.into_iter()
            .filter_map(|key| {
                let mut session = self.sessions.remove(&key)?;
                let _ = session.controller.finalize();
                session.state = state;
                let update = update_for(&session, false);
                let terminal_id =
                    format!("internal:{}:{}:{state:?}", key.turn_id.0, key.stream_id.0);
                self.archive_session(key, session, state, &terminal_id);
                Some(update)
            })
            .collect()
    }

    fn ensure_session(&mut self, event: &TuiEvent, key: &StreamKey, explicit_retry: bool) {
        if self.sessions.contains_key(key) {
            return;
        }

        let previous = self.last_by_turn.get(&key.turn_id).cloned();
        let previous_cell = previous
            .as_ref()
            .and_then(|previous| self.sessions.get(previous))
            .map(|session| session.cell_id.clone());
        let has_archived_turn = self
            .archived
            .keys()
            .any(|archived| archived.turn_id == key.turn_id);
        let cell_id = previous_cell.clone().unwrap_or_else(|| {
            if has_archived_turn {
                event
                    .id
                    .with_suffix(&format!("stream-{:016x}", stable_hash(&key.stream_id.0)))
            } else {
                event.id.clone()
            }
        });

        if let Some(previous) = previous
            && previous != *key
            && let Some(session) = self.sessions.remove(&previous)
        {
            self.archive_session(
                previous,
                session,
                StreamSessionState::Superseded,
                "internal:stream-superseded",
            );
        }

        self.sessions.insert(
            key.clone(),
            StreamSession {
                cell_id,
                last_reliable_sequence: None,
                controller: MarkdownStreamController::default(),
                content: String::new(),
                state: if explicit_retry {
                    StreamSessionState::Retrying
                } else {
                    StreamSessionState::Active
                },
                offline_label: event
                    .stream
                    .as_ref()
                    .is_some_and(|stream| stream.offline_label),
            },
        );
    }

    fn archive_session(
        &mut self,
        key: StreamKey,
        session: StreamSession,
        state: StreamSessionState,
        terminal_event_id: &str,
    ) {
        if self.last_by_turn.get(&key.turn_id) == Some(&key) {
            self.last_by_turn.remove(&key.turn_id);
        }
        if !self.archived.contains_key(&key) {
            self.archive_order.push_back(key.clone());
        }
        self.archived.insert(
            key,
            ArchivedStreamSession {
                cell_id: session.cell_id,
                last_reliable_sequence: session.last_reliable_sequence,
                terminal_event_hash: stable_hash(terminal_event_id),
                state,
            },
        );
        while self.archive_order.len() > ARCHIVED_SESSION_LIMIT {
            if let Some(expired) = self.archive_order.pop_front() {
                self.archived.remove(&expired);
            }
        }
    }

    fn record_event_id(&mut self, event_id: &str) -> bool {
        if self.seen_event_ids.contains(event_id) {
            return false;
        }
        let event_id = event_id.to_string();
        self.seen_event_ids.insert(event_id.clone());
        self.seen_event_order.push_back(event_id);
        while self.seen_event_order.len() > SEEN_EVENT_LIMIT {
            if let Some(expired) = self.seen_event_order.pop_front() {
                self.seen_event_ids.remove(&expired);
            }
        }
        true
    }

    #[cfg(test)]
    fn retained_content_bytes(&self) -> usize {
        self.sessions
            .values()
            .map(|session| session.content.len())
            .sum()
    }
}

fn reliable_sequence_is_late(last: Option<u64>, sequence: TuiSourceSequence) -> bool {
    sequence
        .reliable_value()
        .is_some_and(|sequence| last.is_some_and(|last| sequence <= last))
}

fn observe_reliable_sequence(last: &mut Option<u64>, sequence: TuiSourceSequence) {
    if let Some(sequence) = sequence.reliable_value() {
        *last = Some(sequence);
    }
}

fn apply_offline_label(session: &mut StreamSession) {
    if session.offline_label
        && !session.content.is_empty()
        && !session.content.starts_with("[offline] ")
    {
        session.content.insert_str(0, "[offline] ");
    }
}

fn update_stream_frame(event: &mut TuiEvent, stable: &str, tail: &str, committed: bool) {
    if let Some(stream) = event.stream.as_mut() {
        stream.stable_source = stable.to_string();
        stream.live_tail = tail.to_string();
        stream.committed = committed;
    }
}

fn update_for(session: &StreamSession, active: bool) -> AssistantTimelineUpdate {
    AssistantTimelineUpdate {
        cell_id: session.cell_id.clone(),
        content: session.content.clone(),
        active,
    }
}

fn stable_hash(value: &str) -> u64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    value.as_bytes().iter().fold(OFFSET, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(PRIME)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presentation::{
        PresentationVisibility, TuiCellKind, TuiStreamIdentity, TuiStreamState,
    };

    fn event(
        turn: &str,
        stream: &str,
        event_id: &str,
        sequence: TuiSourceSequence,
        phase: TuiStreamPhase,
        content: &str,
    ) -> TuiEvent {
        TuiEvent {
            id: TuiCellId::from_test(&format!("assistant-{turn}")),
            kind: TuiCellKind::AssistantMessage,
            visible_text: content.to_string(),
            detail: None,
            stream: Some(TuiStreamState {
                stable_source: String::new(),
                live_tail: String::new(),
                committed: false,
                identity: TuiStreamIdentity {
                    thread_id: "thread".to_string(),
                    turn_id: turn.to_string(),
                    stream_id: stream.to_string(),
                    event_id: event_id.to_string(),
                    source_sequence: sequence,
                    phase,
                },
                offline_label: false,
            }),
            visibility: PresentationVisibility::Transcript,
            tool_update: None,
        }
    }

    fn local_event(
        turn: &str,
        stream: &str,
        sequence: u64,
        phase: TuiStreamPhase,
        content: &str,
    ) -> TuiEvent {
        event(
            turn,
            stream,
            &format!("fallback:{turn}:{stream}:{sequence}:{phase:?}"),
            TuiSourceSequence::LocalFallback(sequence),
            phase,
            content,
        )
    }

    #[test]
    fn delta_then_final_replaces_one_canonical_cell_and_releases_session() {
        let mut store = TimelineStore::default();
        let mut delta = local_event("turn-1", "message-1", 1, TuiStreamPhase::Delta, "你");
        let first = store.apply(&mut delta).expect("delta");
        let mut final_event = local_event("turn-1", "message-1", 2, TuiStreamPhase::Final, "你好");
        let final_update = store.apply(&mut final_event).expect("final");

        assert_eq!(first.cell_id, final_update.cell_id);
        assert_eq!(final_update.content, "你好");
        assert!(!final_update.active);
        assert!(store.sessions.is_empty());
        assert_eq!(store.retained_content_bytes(), 0);
    }

    #[test]
    fn retry_reuses_active_cell_and_resets_partial_content() {
        let mut store = TimelineStore::default();
        let mut first = local_event("turn-1", "attempt-1", 1, TuiStreamPhase::Delta, "old");
        let first = store.apply(&mut first).expect("first attempt");
        let mut retry = local_event("turn-1", "attempt-2", 2, TuiStreamPhase::Retry, "new");
        let retry = store.apply(&mut retry).expect("retry");
        let mut final_event = local_event(
            "turn-1",
            "attempt-2",
            3,
            TuiStreamPhase::Final,
            "new answer",
        );
        let final_update = store.apply(&mut final_event).expect("retry final");

        assert_eq!(first.cell_id, retry.cell_id);
        assert_eq!(retry.cell_id, final_update.cell_id);
        assert_eq!(final_update.content, "new answer");
    }

    #[test]
    fn cancel_releases_session_and_late_delta_cannot_rebind_it() {
        let mut store = TimelineStore::default();
        let mut delta = local_event("turn-1", "attempt-1", 1, TuiStreamPhase::Delta, "partial");
        let first = store.apply(&mut delta).expect("delta");
        let mut cancel = local_event("turn-1", "attempt-1", 2, TuiStreamPhase::Cancel, "");
        let cancelled = store.apply(&mut cancel).expect("cancel");
        let mut late = local_event("turn-1", "attempt-1", 3, TuiStreamPhase::Delta, " late");

        assert_eq!(first.cell_id, cancelled.cell_id);
        assert!(!cancelled.active);
        assert!(store.sessions.is_empty());
        assert_eq!(store.apply(&mut late), None);

        let mut retry = local_event("turn-1", "attempt-2", 4, TuiStreamPhase::Started, "retry");
        let retry = store.apply(&mut retry).expect("new retry session");
        assert_ne!(retry.cell_id, cancelled.cell_id);
    }

    #[test]
    fn duplicate_event_id_is_ignored_and_counted() {
        let mut store = TimelineStore::default();
        let mut first = event(
            "turn-1",
            "message-1",
            "provider:event-7",
            TuiSourceSequence::ProviderReliable(7),
            TuiStreamPhase::Delta,
            "answer",
        );
        let mut replay = first.clone();

        store.apply(&mut first).expect("first event");
        assert_eq!(store.apply(&mut replay), None);
        assert_eq!(store.duplicate_event_count(), 1);
    }

    #[test]
    fn reliable_sequence_rejects_late_but_local_fallback_order_does_not() {
        let mut reliable_store = TimelineStore::default();
        let mut reliable = event(
            "turn",
            "reliable",
            "provider:9",
            TuiSourceSequence::ProviderReliable(9),
            TuiStreamPhase::Delta,
            "new",
        );
        reliable_store.apply(&mut reliable).expect("reliable");
        let mut late = event(
            "turn",
            "reliable",
            "provider:8",
            TuiSourceSequence::ProviderReliable(8),
            TuiStreamPhase::Delta,
            "old",
        );
        assert_eq!(reliable_store.apply(&mut late), None);

        let mut fallback_store = TimelineStore::default();
        let mut arrived_first = event(
            "turn",
            "fallback",
            "fallback:9",
            TuiSourceSequence::LocalFallback(9),
            TuiStreamPhase::Delta,
            "甲",
        );
        fallback_store
            .apply(&mut arrived_first)
            .expect("first fallback");
        let mut arrived_second = event(
            "turn",
            "fallback",
            "fallback:1",
            TuiSourceSequence::LocalFallback(1),
            TuiStreamPhase::Delta,
            "乙",
        );
        assert_eq!(
            fallback_store
                .apply(&mut arrived_second)
                .expect("second fallback")
                .content,
            "甲乙"
        );
    }

    #[test]
    fn provider_disconnect_freezes_partial_content_and_releases_active_session() {
        let mut store = TimelineStore::default();
        let mut partial = local_event(
            "turn-disconnect",
            "message-disconnect",
            1,
            TuiStreamPhase::Delta,
            "partial answer",
        );
        let active = store.apply(&mut partial).expect("active partial");

        let updates = store.finish_active();

        assert!(active.active);
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].content, "partial answer");
        assert!(!updates[0].active);
        assert!(!store.has_active_sessions());
    }

    #[test]
    fn duplicate_final_and_reliable_out_of_order_delta_cannot_change_content() {
        let mut store = TimelineStore::default();
        let mut newest = event(
            "turn-order",
            "message-order",
            "provider:20",
            TuiSourceSequence::ProviderReliable(20),
            TuiStreamPhase::Delta,
            "newest",
        );
        store.apply(&mut newest).expect("newest delta");
        let mut late = event(
            "turn-order",
            "message-order",
            "provider:19",
            TuiSourceSequence::ProviderReliable(19),
            TuiStreamPhase::Delta,
            "stale",
        );
        assert_eq!(store.apply(&mut late), None);
        assert_eq!(store.finish_active()[0].content, "newest");

        let mut final_event = local_event(
            "turn-final",
            "message-final",
            1,
            TuiStreamPhase::Final,
            "answer",
        );
        let final_update = store.apply(&mut final_event).expect("first final");
        let mut duplicate_final = local_event(
            "turn-final",
            "message-final",
            2,
            TuiStreamPhase::Final,
            "different answer",
        );
        assert_eq!(store.apply(&mut duplicate_final), None);
        assert_eq!(final_update.content, "answer");
    }

    #[test]
    fn seen_event_ids_are_bounded_and_oldest_id_is_predictably_evicted() {
        let mut store = TimelineStore::default();
        for index in 0..=SEEN_EVENT_LIMIT {
            let mut delta = event(
                "turn-seen",
                "message-seen",
                &format!("provider:{index}"),
                TuiSourceSequence::LocalFallback(index as u64),
                TuiStreamPhase::Delta,
                "x",
            );
            store.apply(&mut delta).expect("unique event");
        }

        assert_eq!(store.seen_event_ids.len(), SEEN_EVENT_LIMIT);
        assert_eq!(store.seen_event_order.len(), SEEN_EVENT_LIMIT);
        assert!(!store.seen_event_ids.contains("provider:0"));
        assert!(
            store
                .seen_event_ids
                .contains(&format!("provider:{SEEN_EVENT_LIMIT}"))
        );
    }

    #[test]
    fn repeated_payload_with_distinct_event_ids_is_preserved() {
        let mut store = TimelineStore::default();
        let mut first = local_event("turn-1", "message-1", 1, TuiStreamPhase::Delta, "哈");
        store.apply(&mut first).expect("first");
        let mut second = local_event("turn-1", "message-1", 2, TuiStreamPhase::Delta, "哈");
        let update = store.apply(&mut second).expect("second");

        assert_eq!(update.content, "哈哈");
    }

    #[test]
    fn completed_sessions_are_bounded_and_retain_no_stream_content() {
        let mut store = TimelineStore::default();
        let payload = "x".repeat(16 * 1024);
        for index in 0..(ARCHIVED_SESSION_LIMIT + 40) {
            let mut final_event = local_event(
                &format!("turn-{index}"),
                &format!("stream-{index}"),
                index as u64 + 1,
                TuiStreamPhase::Final,
                &payload,
            );
            store.apply(&mut final_event).expect("final event");
        }

        assert!(store.sessions.is_empty());
        assert!(store.last_by_turn.is_empty());
        assert_eq!(store.retained_content_bytes(), 0);
        assert_eq!(store.archived.len(), ARCHIVED_SESSION_LIMIT);
        assert_eq!(store.archive_order.len(), ARCHIVED_SESSION_LIMIT);
    }

    #[test]
    fn unicode_markdown_and_long_tokens_survive_exactly() {
        let long_left = "x".repeat(2048);
        let long_right = "y".repeat(2048);
        let payloads = [
            ("中", "文", "中文".to_string()),
            ("かな", "カナ", "かなカナ".to_string()),
            ("👩‍", "💻👨‍👩‍👧‍👦", "👩‍💻👨‍👩‍👧‍👦".to_string()),
            (
                "```rust\nfn ",
                "main() {}\n```",
                "```rust\nfn main() {}\n```".to_string(),
            ),
            (
                long_left.as_str(),
                long_right.as_str(),
                format!("{long_left}{long_right}"),
            ),
        ];
        for (index, (left, right, expected)) in payloads.iter().enumerate() {
            let mut store = TimelineStore::default();
            let mut first = local_event("turn", "message", 1, TuiStreamPhase::Delta, left);
            store.apply(&mut first).expect("first delta");
            let mut second = local_event("turn", "message", 2, TuiStreamPhase::Delta, right);
            let update = store.apply(&mut second).expect("second delta");
            assert_eq!(update.content, *expected, "payload {index}");
        }
    }
}

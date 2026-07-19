use crate::presentation::{TuiCellId, TuiEvent, TuiStreamPhase};
use crate::streaming::MarkdownStreamController;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct TurnId(String);

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct StreamSessionId(String);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct SourceSequence(u64);

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
    last_sequence: SourceSequence,
    controller: MarkdownStreamController,
    content: String,
    state: StreamSessionState,
    offline_label: bool,
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
}

impl TimelineStore {
    pub(crate) fn apply(&mut self, event: &mut TuiEvent) -> Option<AssistantTimelineUpdate> {
        let stream = event.stream.as_ref()?;
        let key = StreamKey {
            turn_id: TurnId(stream.identity.turn_id.clone()),
            stream_id: StreamSessionId(stream.identity.stream_id.clone()),
        };
        let sequence = SourceSequence(stream.identity.source_sequence);
        let phase = stream.identity.phase;

        match phase {
            TuiStreamPhase::Started => self.start(event, key, sequence, false),
            TuiStreamPhase::Delta => self.delta(event, key, sequence),
            TuiStreamPhase::Retry => self.start(event, key, sequence, true),
            TuiStreamPhase::Final => self.finalize(event, key, sequence),
            TuiStreamPhase::Finish => self.finish(key, sequence, StreamSessionState::Finalized),
            TuiStreamPhase::Cancel => self.finish(key, sequence, StreamSessionState::Cancelled),
        }
    }

    pub(crate) fn finish_active(&mut self) -> Vec<AssistantTimelineUpdate> {
        let keys = self
            .last_by_turn
            .values()
            .filter(|key| {
                self.sessions.get(*key).is_some_and(|session| {
                    matches!(
                        session.state,
                        StreamSessionState::Active | StreamSessionState::Retrying
                    )
                })
            })
            .cloned()
            .collect::<Vec<_>>();

        keys.into_iter()
            .filter_map(|key| {
                let next = self
                    .sessions
                    .get(&key)
                    .map(|session| SourceSequence(session.last_sequence.0.saturating_add(1)))?;
                self.finish(key, next, StreamSessionState::Finalized)
            })
            .collect()
    }

    pub(crate) fn clear(&mut self) {
        self.sessions.clear();
        self.last_by_turn.clear();
    }

    fn start(
        &mut self,
        event: &mut TuiEvent,
        key: StreamKey,
        sequence: SourceSequence,
        explicit_retry: bool,
    ) -> Option<AssistantTimelineUpdate> {
        if let Some(session) = self.sessions.get(&key) {
            if sequence <= session.last_sequence
                || matches!(
                    session.state,
                    StreamSessionState::Finalized
                        | StreamSessionState::Cancelled
                        | StreamSessionState::Superseded
                )
            {
                return None;
            }
        }

        if !self.sessions.contains_key(&key) {
            let previous = self.last_by_turn.get(&key.turn_id).cloned();
            let cell_id = match previous.as_ref().and_then(|key| self.sessions.get(key)) {
                Some(session)
                    if matches!(
                        session.state,
                        StreamSessionState::Active | StreamSessionState::Retrying
                    ) =>
                {
                    session.cell_id.clone()
                }
                Some(_) => event
                    .id
                    .with_suffix(&format!("stream-{:016x}", stable_hash(&key.stream_id.0))),
                None => event.id.clone(),
            };

            if let Some(previous) = previous
                && let Some(session) = self.sessions.get_mut(&previous)
                && matches!(
                    session.state,
                    StreamSessionState::Active | StreamSessionState::Retrying
                )
            {
                session.state = StreamSessionState::Superseded;
            }

            self.sessions.insert(
                key.clone(),
                StreamSession {
                    cell_id,
                    last_sequence: SourceSequence(0),
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

        let session = self.sessions.get_mut(&key).expect("inserted session");
        session.last_sequence = sequence;
        session.state = if explicit_retry {
            StreamSessionState::Retrying
        } else {
            StreamSessionState::Active
        };
        session.controller.clear();
        let frame = session.controller.push_delta(&event.visible_text);
        session.content = format!("{}{}", frame.stable_source, frame.live_tail);
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
        sequence: SourceSequence,
    ) -> Option<AssistantTimelineUpdate> {
        if !self.sessions.contains_key(&key) {
            let _ = self.start(event, key.clone(), sequence, false);
            return self
                .sessions
                .get(&key)
                .filter(|session| !session.content.is_empty())
                .map(|session| update_for(session, true));
        }

        let session = self.sessions.get_mut(&key)?;
        if sequence <= session.last_sequence
            || !matches!(
                session.state,
                StreamSessionState::Active | StreamSessionState::Retrying
            )
        {
            return None;
        }
        session.last_sequence = sequence;
        let frame = session.controller.push_delta(&event.visible_text);
        session.content = format!("{}{}", frame.stable_source, frame.live_tail);
        apply_offline_label(session);
        update_stream_frame(
            event,
            &frame.stable_source,
            &frame.live_tail,
            frame.committed,
        );
        Some(update_for(session, true))
    }

    fn finalize(
        &mut self,
        event: &mut TuiEvent,
        key: StreamKey,
        sequence: SourceSequence,
    ) -> Option<AssistantTimelineUpdate> {
        if !self.sessions.contains_key(&key) {
            let initial_sequence = SourceSequence(sequence.0.saturating_sub(1));
            let _ = self.start(event, key.clone(), initial_sequence, false);
        }
        let session = self.sessions.get_mut(&key)?;
        if sequence <= session.last_sequence
            || matches!(
                session.state,
                StreamSessionState::Finalized
                    | StreamSessionState::Cancelled
                    | StreamSessionState::Superseded
            )
        {
            return None;
        }

        session.last_sequence = sequence;
        session.controller.clear();
        if !event.visible_text.is_empty() {
            let frame = session.controller.push_delta(&event.visible_text);
            session.content = format!("{}{}", frame.stable_source, frame.live_tail);
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
        Some(update_for(session, false))
    }

    fn finish(
        &mut self,
        key: StreamKey,
        sequence: SourceSequence,
        state: StreamSessionState,
    ) -> Option<AssistantTimelineUpdate> {
        let session = self.sessions.get_mut(&key)?;
        if sequence <= session.last_sequence
            || matches!(
                session.state,
                StreamSessionState::Finalized
                    | StreamSessionState::Cancelled
                    | StreamSessionState::Superseded
            )
        {
            return None;
        }
        session.last_sequence = sequence;
        let _ = session.controller.finalize();
        session.state = state;
        Some(update_for(session, false))
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
        sequence: u64,
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
                    source_sequence: sequence,
                    phase,
                },
                offline_label: false,
            }),
            visibility: PresentationVisibility::Transcript,
            tool_update: None,
        }
    }

    #[test]
    fn delta_then_final_replaces_one_canonical_cell() {
        let mut store = TimelineStore::default();
        let mut delta = event("turn-1", "message-1", 1, TuiStreamPhase::Delta, "你");
        let first = store.apply(&mut delta).expect("delta");
        let mut final_event = event("turn-1", "message-1", 2, TuiStreamPhase::Final, "你好");
        let final_update = store.apply(&mut final_event).expect("final");

        assert_eq!(first.cell_id, final_update.cell_id);
        assert_eq!(final_update.content, "你好");
        assert!(!final_update.active);
    }

    #[test]
    fn retry_reuses_active_cell_and_resets_partial_content() {
        let mut store = TimelineStore::default();
        let mut first = event("turn-1", "attempt-1", 1, TuiStreamPhase::Delta, "old");
        let first = store.apply(&mut first).expect("first attempt");
        let mut retry = event("turn-1", "attempt-2", 2, TuiStreamPhase::Retry, "new");
        let retry = store.apply(&mut retry).expect("retry");
        let mut final_event = event(
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
    fn cancel_freezes_session_and_late_delta_cannot_rebind_it() {
        let mut store = TimelineStore::default();
        let mut delta = event("turn-1", "attempt-1", 1, TuiStreamPhase::Delta, "partial");
        let first = store.apply(&mut delta).expect("delta");
        let mut cancel = event("turn-1", "attempt-1", 2, TuiStreamPhase::Cancel, "");
        let cancelled = store.apply(&mut cancel).expect("cancel");
        let mut late = event("turn-1", "attempt-1", 3, TuiStreamPhase::Delta, " late");

        assert_eq!(first.cell_id, cancelled.cell_id);
        assert!(!cancelled.active);
        assert_eq!(store.apply(&mut late), None);

        let mut retry = event("turn-1", "attempt-2", 4, TuiStreamPhase::Started, "retry");
        let retry = store.apply(&mut retry).expect("new retry session");
        assert_ne!(retry.cell_id, cancelled.cell_id);
    }

    #[test]
    fn duplicate_final_is_structurally_idempotent() {
        let mut store = TimelineStore::default();
        let mut delta = event("turn-1", "message-1", 1, TuiStreamPhase::Delta, "answer");
        store.apply(&mut delta).expect("delta");
        let mut final_one = event("turn-1", "message-1", 2, TuiStreamPhase::Final, "answer");
        store.apply(&mut final_one).expect("first final");
        let mut final_two = event("turn-1", "message-1", 3, TuiStreamPhase::Final, "answer");

        assert_eq!(store.apply(&mut final_two), None);
    }

    #[test]
    fn repeated_payload_with_new_sequence_is_not_deduplicated() {
        let mut store = TimelineStore::default();
        let mut first = event("turn-1", "message-1", 1, TuiStreamPhase::Delta, "哈");
        store.apply(&mut first).expect("first");
        let mut second = event("turn-1", "message-1", 2, TuiStreamPhase::Delta, "哈");
        let update = store.apply(&mut second).expect("second");

        assert_eq!(update.content, "哈哈");
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
            let mut first = event("turn", "message", 1, TuiStreamPhase::Delta, left);
            store.apply(&mut first).expect("first delta");
            let mut second = event("turn", "message", 2, TuiStreamPhase::Delta, right);
            let update = store.apply(&mut second).expect("second delta");
            assert_eq!(update.content, *expected, "payload {index}");
        }
    }
}

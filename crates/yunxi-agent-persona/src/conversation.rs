use serde::{Deserialize, Serialize};

pub const CONVERSATION_STATE_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_CONVERSATION_STATE_TTL_MILLIS: u128 = 60 * 60 * 1_000;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConversationState {
    pub schema_version: u32,
    pub session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_topic: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_tone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emotional_context: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unresolved_intents: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recent_entities: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_user_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_assistant_message: Option<String>,
    pub updated_at_millis: u128,
    pub expires_at_millis: u128,
}

impl ConversationState {
    pub fn new(session_id: impl Into<String>, now_millis: u128) -> Self {
        Self {
            schema_version: CONVERSATION_STATE_SCHEMA_VERSION,
            session_id: session_id.into(),
            parent_session_id: None,
            current_topic: None,
            response_tone: None,
            emotional_context: None,
            unresolved_intents: Vec::new(),
            recent_entities: Vec::new(),
            last_user_message: None,
            last_assistant_message: None,
            updated_at_millis: now_millis,
            expires_at_millis: now_millis.saturating_add(DEFAULT_CONVERSATION_STATE_TTL_MILLIS),
        }
    }

    pub fn is_active_at(&self, now_millis: u128) -> bool {
        self.expires_at_millis > now_millis
    }

    pub fn carry_turn(
        previous: Option<&Self>,
        session_id: impl Into<String>,
        parent_session_id: Option<String>,
        user_message: &str,
        assistant_message: &str,
        now_millis: u128,
    ) -> Self {
        let mut state = previous
            .filter(|state| state.is_active_at(now_millis))
            .cloned()
            .unwrap_or_else(|| Self::new("", now_millis));
        state.schema_version = CONVERSATION_STATE_SCHEMA_VERSION;
        state.session_id = session_id.into();
        state.parent_session_id = parent_session_id;
        state.current_topic = bounded_non_empty(user_message, 240);
        state.response_tone =
            Some(infer_response_tone(user_message, assistant_message).to_string());
        state.emotional_context = infer_emotional_context(user_message)
            .map(str::to_string)
            .or(state.emotional_context);
        state.unresolved_intents = infer_unresolved_intents(user_message, assistant_message);
        state.recent_entities = merge_recent_entities(
            &state.recent_entities,
            &extract_recent_entities(user_message),
        );
        state.last_user_message = bounded_non_empty(user_message, 360);
        state.last_assistant_message = bounded_non_empty(assistant_message, 360);
        state.updated_at_millis = now_millis;
        state.expires_at_millis = now_millis.saturating_add(DEFAULT_CONVERSATION_STATE_TTL_MILLIS);
        state
    }
}

fn bounded_non_empty(value: &str, max_chars: usize) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    Some(value.chars().take(max_chars).collect())
}

fn infer_response_tone(user_message: &str, assistant_message: &str) -> &'static str {
    if contains_any(
        user_message,
        &["难过", "伤心", "焦虑", "害怕", "孤独", "累", "撑不住"],
    ) {
        return "gentle_supportive";
    }
    if contains_any(user_message, &["哈哈", "好耶", "开心", "太棒", "笑死"]) {
        return "warm_playful";
    }
    if contains_any(
        user_message,
        &["代码", "架构", "测试", "项目", "实现", "Rust", "API", "bug"],
    ) {
        return "calm_precise";
    }
    if assistant_message.chars().count() <= 120 {
        "warm_concise"
    } else {
        "warm_explanatory"
    }
}

fn infer_emotional_context(user_message: &str) -> Option<&'static str> {
    if contains_any(user_message, &["难过", "伤心", "低落", "想哭"]) {
        Some("sad")
    } else if contains_any(user_message, &["焦虑", "紧张", "担心", "害怕"]) {
        Some("anxious")
    } else if contains_any(user_message, &["生气", "烦", "恼火", "愤怒"]) {
        Some("frustrated")
    } else if contains_any(user_message, &["开心", "高兴", "好耶", "太棒"]) {
        Some("positive")
    } else if contains_any(user_message, &["累", "困", "疲惫", "没精神"]) {
        Some("tired")
    } else {
        None
    }
}

fn infer_unresolved_intents(user_message: &str, assistant_message: &str) -> Vec<String> {
    let asks_question = user_message.contains('?')
        || user_message.contains('？')
        || contains_any(
            user_message,
            &["怎么", "为什么", "能不能", "可以吗", "如何"],
        );
    let assistant_defers = contains_any(
        assistant_message,
        &["稍后", "下一步", "之后", "还需要", "待确认", "需要你提供"],
    );
    if asks_question && assistant_defers {
        bounded_non_empty(user_message, 180).into_iter().collect()
    } else {
        Vec::new()
    }
}

fn extract_recent_entities(value: &str) -> Vec<String> {
    value
        .split(|character: char| {
            character.is_whitespace()
                || matches!(
                    character,
                    ',' | '，' | '.' | '。' | ':' | '：' | ';' | '；' | '!' | '！' | '?' | '？'
                )
        })
        .filter(|token| {
            let length = token.chars().count();
            (2..=32).contains(&length)
                && (token
                    .chars()
                    .any(|character| character.is_ascii_uppercase())
                    || token.chars().any(|character| character.is_ascii_digit()))
        })
        .take(6)
        .map(str::to_string)
        .collect()
}

fn merge_recent_entities(previous: &[String], incoming: &[String]) -> Vec<String> {
    let mut merged = previous.to_vec();
    for entity in incoming {
        if !merged.contains(entity) {
            merged.push(entity.clone());
        }
    }
    if merged.len() > 12 {
        merged.drain(0..merged.len() - 12);
    }
    merged
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    let lower = value.to_ascii_lowercase();
    needles
        .iter()
        .any(|needle| value.contains(needle) || lower.contains(&needle.to_ascii_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carry_turn_preserves_active_context_and_refreshes_ttl() {
        let mut previous = ConversationState::new("previous", 1_000);
        previous.emotional_context = Some("tired".to_string());
        previous.recent_entities = vec!["YunXi".to_string()];

        let state = ConversationState::carry_turn(
            Some(&previous),
            "current",
            Some("previous".to_string()),
            "继续完善 Rust API",
            "好，我们继续处理。",
            2_000,
        );

        assert_eq!(state.session_id, "current");
        assert_eq!(state.parent_session_id.as_deref(), Some("previous"));
        assert_eq!(state.response_tone.as_deref(), Some("calm_precise"));
        assert_eq!(state.emotional_context.as_deref(), Some("tired"));
        assert!(state.expires_at_millis > previous.expires_at_millis);
        assert!(state.recent_entities.contains(&"YunXi".to_string()));
    }

    #[test]
    fn expired_state_does_not_carry_emotional_context() {
        let mut previous = ConversationState::new("previous", 1);
        previous.expires_at_millis = 2;
        previous.emotional_context = Some("sad".to_string());

        let state = ConversationState::carry_turn(
            Some(&previous),
            "current",
            Some("previous".to_string()),
            "今天继续项目",
            "可以。",
            3,
        );

        assert_eq!(state.emotional_context, None);
    }
}

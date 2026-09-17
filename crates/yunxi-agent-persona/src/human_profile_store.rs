use crate::memory::{
    MemoryKind, MemoryLayer, MemoryRecord, MemoryScope, MemorySensitivity, now_millis,
};
use crate::profile::HumanProfile;
use crate::settings::yunxi_home_dir;
use std::fmt;
use std::path::{Path, PathBuf};

const MAX_HUMAN_PROFILE_BYTES: u64 = 256 * 1024;
const MAX_PROFILE_ITEMS: usize = 64;
const MAX_PROFILE_ITEM_CHARS: usize = 1_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HumanProfileStore {
    path: PathBuf,
}

impl Default for HumanProfileStore {
    fn default() -> Self {
        Self::new(yunxi_home_dir().join("persona").join("human-profile.json"))
    }
}

impl HumanProfileStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<HumanProfile, HumanProfileStoreError> {
        if !self.path.exists() {
            return Ok(HumanProfile::default());
        }
        let metadata = std::fs::metadata(&self.path)?;
        if !metadata.is_file() {
            return Err(HumanProfileStoreError::InvalidProfile(format!(
                "human profile path is not a regular file: {}",
                self.path.display()
            )));
        }
        if metadata.len() > MAX_HUMAN_PROFILE_BYTES {
            return Err(HumanProfileStoreError::ProfileFileTooLarge(metadata.len()));
        }
        let profile = serde_json::from_str::<HumanProfile>(&std::fs::read_to_string(&self.path)?)?;
        validate_human_profile(&profile)?;
        Ok(profile)
    }

    pub fn save(&self, profile: &HumanProfile) -> Result<(), HumanProfileStoreError> {
        validate_human_profile(profile)?;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let serialized = serde_json::to_string_pretty(profile)?;
        std::fs::write(&self.path, format!("{serialized}\n"))?;
        Ok(())
    }

    pub fn load_with_memory_overlay(
        &self,
        records: &[MemoryRecord],
    ) -> Result<HumanProfile, HumanProfileStoreError> {
        let mut profile = self.load()?;
        let now = now_millis();
        for record in records.iter().filter(|record| {
            record.scope == MemoryScope::GlobalUser
                && record.layer == MemoryLayer::Profile
                && record.sensitivity == MemorySensitivity::Low
                && record.is_recallable_at(now)
        }) {
            match record.kind {
                MemoryKind::Preference if is_language_preference(record) => {
                    push_unique(&mut profile.language_preferences, &record.content);
                }
                MemoryKind::Preference => {
                    push_unique(&mut profile.interaction_preferences, &record.content);
                }
                MemoryKind::PersonalFact => {
                    push_unique(&mut profile.stable_facts, &record.content);
                }
                MemoryKind::Goal => {
                    push_unique(&mut profile.long_term_goals, &record.content);
                }
                MemoryKind::RelationshipNote
                | MemoryKind::EmotionalState
                | MemoryKind::ProjectContext
                | MemoryKind::Correction
                | MemoryKind::Event
                | MemoryKind::ToolTraceSummary => {}
            }
        }
        validate_human_profile(&profile)?;
        Ok(profile)
    }
}

#[derive(Debug)]
pub enum HumanProfileStoreError {
    Io(std::io::Error),
    Json(serde_json::Error),
    InvalidProfile(String),
    ProfileFileTooLarge(u64),
}

impl fmt::Display for HumanProfileStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "human profile I/O failed: {error}"),
            Self::Json(error) => write!(formatter, "human profile JSON is invalid: {error}"),
            Self::InvalidProfile(reason) => write!(formatter, "human profile is invalid: {reason}"),
            Self::ProfileFileTooLarge(bytes) => write!(
                formatter,
                "human profile file is too large ({bytes} bytes; limit is {MAX_HUMAN_PROFILE_BYTES} bytes)"
            ),
        }
    }
}

impl std::error::Error for HumanProfileStoreError {}

impl From<std::io::Error> for HumanProfileStoreError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for HumanProfileStoreError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

fn validate_human_profile(profile: &HumanProfile) -> Result<(), HumanProfileStoreError> {
    if let Some(name) = &profile.preferred_name {
        validate_item("preferred_name", name)?;
    }
    validate_items("language_preferences", &profile.language_preferences)?;
    validate_items("interaction_preferences", &profile.interaction_preferences)?;
    validate_items("stable_facts", &profile.stable_facts)?;
    validate_items("long_term_goals", &profile.long_term_goals)?;
    Ok(())
}

fn is_language_preference(record: &MemoryRecord) -> bool {
    let content = record.content.to_ascii_lowercase();
    record.dedup_key.contains("|preference|language:")
        || content.contains("中文")
        || content.contains("英文")
        || content.contains("english")
        || content.contains("chinese")
}

fn push_unique(values: &mut Vec<String>, value: &str) {
    let value = value.trim();
    if value.is_empty()
        || value.chars().count() > MAX_PROFILE_ITEM_CHARS
        || values.len() >= MAX_PROFILE_ITEMS
    {
        return;
    }
    if values
        .iter()
        .any(|existing| existing.trim().eq_ignore_ascii_case(value))
    {
        return;
    }
    values.push(value.to_string());
}

fn validate_items(field: &str, values: &[String]) -> Result<(), HumanProfileStoreError> {
    if values.len() > MAX_PROFILE_ITEMS {
        return Err(HumanProfileStoreError::InvalidProfile(format!(
            "{field} cannot contain more than {MAX_PROFILE_ITEMS} entries"
        )));
    }
    for (index, value) in values.iter().enumerate() {
        validate_item(&format!("{field}[{index}]"), value)?;
    }
    Ok(())
}

fn validate_item(field: &str, value: &str) -> Result<(), HumanProfileStoreError> {
    if value.trim().is_empty() {
        return Err(HumanProfileStoreError::InvalidProfile(format!(
            "{field} cannot be empty"
        )));
    }
    if value.chars().count() > MAX_PROFILE_ITEM_CHARS {
        return Err(HumanProfileStoreError::InvalidProfile(format!(
            "{field} exceeds the {MAX_PROFILE_ITEM_CHARS}-character limit"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{
        MemoryKind, MemoryLayer, MemoryRecord, MemoryScope, MemorySensitivity, MemoryStatus,
    };

    #[test]
    fn profile_store_round_trips_structured_profile_without_embeddings() {
        let directory = tempfile_directory("human-profile");
        let store = HumanProfileStore::new(directory.join("human-profile.json"));
        let profile = HumanProfile {
            preferred_name: Some("小岚".to_string()),
            language_preferences: vec!["中文".to_string()],
            interaction_preferences: vec!["自然、简短".to_string()],
            stable_facts: vec!["从事软件开发".to_string()],
            long_term_goals: vec!["持续完善 YunXi".to_string()],
        };

        store.save(&profile).expect("save profile");
        assert_eq!(store.load().expect("load profile"), profile);
        let serialized = std::fs::read_to_string(store.path()).expect("read profile");
        assert!(!serialized.contains("embedding"));

        std::fs::remove_dir_all(directory).expect("remove fixture");
    }

    #[test]
    fn profile_overlay_accepts_only_active_low_sensitivity_profile_memories() {
        let directory = tempfile_directory("human-profile-overlay");
        let store = HumanProfileStore::new(directory.join("human-profile.json"));
        store
            .save(&HumanProfile {
                preferred_name: Some("小岚".to_string()),
                interaction_preferences: vec!["保留人工维护的偏好".to_string()],
                ..HumanProfile::default()
            })
            .expect("save base profile");

        let language = profile_memory("language", MemoryKind::Preference, "用户偏好使用中文回答。");
        let fact = profile_memory("fact", MemoryKind::PersonalFact, "用户是一名软件工程师。");
        let goal = profile_memory("goal", MemoryKind::Goal, "持续完善 YunXi Agent。 ");
        let mut pending = profile_memory("pending", MemoryKind::PersonalFact, "这条尚未确认。");
        pending.status = MemoryStatus::Pending;
        let mut sensitive =
            profile_memory("sensitive", MemoryKind::PersonalFact, "这条是高敏感信息。");
        sensitive.sensitivity = MemorySensitivity::High;
        let ordinary_long_term = MemoryRecord::new(
            "ordinary",
            MemoryScope::GlobalUser,
            MemoryKind::Preference,
            "普通长期偏好仍由向量召回。",
            1,
        )
        .with_status(MemoryStatus::Active);

        let profile = store
            .load_with_memory_overlay(&[
                language,
                fact,
                goal,
                pending,
                sensitive,
                ordinary_long_term,
            ])
            .expect("materialize profile");

        assert_eq!(profile.preferred_name.as_deref(), Some("小岚"));
        assert!(
            profile
                .language_preferences
                .iter()
                .any(|value| value.contains("中文"))
        );
        assert!(
            profile
                .interaction_preferences
                .contains(&"保留人工维护的偏好".to_string())
        );
        assert_eq!(profile.stable_facts, vec!["用户是一名软件工程师。"]);
        assert_eq!(profile.long_term_goals, vec!["持续完善 YunXi Agent。"]);
        assert!(
            !profile
                .stable_facts
                .iter()
                .any(|value| value.contains("未确认"))
        );
        assert!(
            !profile
                .stable_facts
                .iter()
                .any(|value| value.contains("敏感"))
        );
        assert!(
            !profile
                .interaction_preferences
                .iter()
                .any(|value| value.contains("向量召回"))
        );

        std::fs::remove_dir_all(directory).expect("remove fixture");
    }

    fn profile_memory(id: &str, kind: MemoryKind, content: &str) -> MemoryRecord {
        let mut record = MemoryRecord::new(id, MemoryScope::GlobalUser, kind, content, 1)
            .with_status(MemoryStatus::Active);
        record.layer = MemoryLayer::Profile;
        record
    }

    fn tempfile_directory(label: &str) -> PathBuf {
        let unique = format!(
            "yunxi-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        std::fs::create_dir(&path).expect("create fixture");
        path
    }
}

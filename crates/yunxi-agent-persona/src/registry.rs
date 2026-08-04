use crate::profile::{
    CompanionStrength, PersonaCompanionRules, PersonaProfile, validate_profile_id,
    yunxi_companion_strong,
};
use crate::settings::{PersonaSettings, yunxi_home_dir};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_PROFILE_ID: &str = "yunxi_companion_strong";
const MAX_PROFILE_FILE_BYTES: u64 = 512 * 1024;
const MAX_SOUL_FILE_BYTES: u64 = 128 * 1024;

#[derive(Debug)]
pub enum PersonaProfileStoreError {
    Io(std::io::Error),
    Json(serde_json::Error),
    InvalidProfile(String),
    ProfileAlreadyExists(PathBuf),
    ProfileNotFound(String),
    ProfileFileTooLarge(u64),
    SoulFileTooLarge(u64),
}

impl fmt::Display for PersonaProfileStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "persona profile I/O failed: {error}"),
            Self::Json(error) => write!(formatter, "persona profile JSON is invalid: {error}"),
            Self::InvalidProfile(reason) => {
                write!(formatter, "persona profile is invalid: {reason}")
            }
            Self::ProfileAlreadyExists(path) => {
                write!(
                    formatter,
                    "persona profile already exists: {}",
                    path.display()
                )
            }
            Self::ProfileNotFound(profile_id) => {
                write!(formatter, "persona profile not found: {profile_id}")
            }
            Self::ProfileFileTooLarge(bytes) => write!(
                formatter,
                "persona profile file is too large ({bytes} bytes; limit is {MAX_PROFILE_FILE_BYTES} bytes)"
            ),
            Self::SoulFileTooLarge(bytes) => write!(
                formatter,
                "persona soul file is too large ({bytes} bytes; limit is {MAX_SOUL_FILE_BYTES} bytes)"
            ),
        }
    }
}

impl std::error::Error for PersonaProfileStoreError {}

impl From<std::io::Error> for PersonaProfileStoreError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for PersonaProfileStoreError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PersonaProfileStore;

impl PersonaProfileStore {
    pub fn profiles_dir() -> PathBuf {
        yunxi_home_dir().join("persona").join("profiles")
    }

    pub fn soul_path() -> PathBuf {
        yunxi_home_dir().join("persona").join("soul.txt")
    }

    pub fn profile_path(profile_id: &str) -> Result<PathBuf, PersonaProfileStoreError> {
        validate_profile_id(profile_id).map_err(PersonaProfileStoreError::InvalidProfile)?;
        Ok(Self::profiles_dir().join(format!("{profile_id}.json")))
    }

    pub fn load(profile_id: &str) -> Result<PersonaProfile, PersonaProfileStoreError> {
        let mut profile = if profile_id == DEFAULT_PROFILE_ID {
            yunxi_companion_strong()
        } else {
            let path = Self::profile_path(profile_id)?;
            if !path.is_file() {
                return Err(PersonaProfileStoreError::ProfileNotFound(
                    profile_id.to_string(),
                ));
            }
            let profile = parse_profile_file(&path)?;
            if profile.id != profile_id {
                return Err(PersonaProfileStoreError::InvalidProfile(format!(
                    "profile id '{}' does not match requested id '{profile_id}'",
                    profile.id
                )));
            }
            profile
        };
        apply_soul_file(&mut profile, &Self::soul_path())?;
        Ok(profile)
    }

    pub fn load_active(settings: &PersonaSettings) -> PersonaProfile {
        Self::load(&settings.active_profile).unwrap_or_else(|_| yunxi_companion_strong())
    }

    pub fn load_active_checked(
        settings: &PersonaSettings,
    ) -> Result<PersonaProfile, PersonaProfileStoreError> {
        Self::load(&settings.active_profile)
    }

    pub fn import_file(source: &Path) -> Result<PersonaProfile, PersonaProfileStoreError> {
        let metadata = fs::metadata(source)?;
        if !metadata.is_file() {
            return Err(PersonaProfileStoreError::InvalidProfile(format!(
                "source is not a regular file: {}",
                source.display()
            )));
        }
        if metadata.len() > MAX_PROFILE_FILE_BYTES {
            return Err(PersonaProfileStoreError::ProfileFileTooLarge(
                metadata.len(),
            ));
        }
        let profile = parse_profile_file(source)?;
        if profile.id == DEFAULT_PROFILE_ID {
            return Err(PersonaProfileStoreError::InvalidProfile(
                "the built-in profile id cannot be replaced".to_string(),
            ));
        }
        let target = Self::profile_path(&profile.id)?;
        if target.exists() {
            return Err(PersonaProfileStoreError::ProfileAlreadyExists(target));
        }
        fs::create_dir_all(Self::profiles_dir())?;
        let serialized = serde_json::to_string_pretty(&profile)?;
        fs::write(&target, format!("{serialized}\n"))?;
        Ok(profile)
    }

    pub fn list() -> Result<Vec<PersonaProfile>, PersonaProfileStoreError> {
        let mut profiles = vec![yunxi_companion_strong()];
        let directory = Self::profiles_dir();
        if !directory.is_dir() {
            return Ok(profiles);
        }
        let mut entries = fs::read_dir(directory)
            .map_err(PersonaProfileStoreError::Io)?
            .collect::<Result<Vec<_>, std::io::Error>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let profile = parse_profile_file(&path)?;
            if profile.id != DEFAULT_PROFILE_ID {
                profiles.push(profile);
            }
        }
        Ok(profiles)
    }
}

fn parse_profile_file(path: &Path) -> Result<PersonaProfile, PersonaProfileStoreError> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > MAX_PROFILE_FILE_BYTES {
        return Err(PersonaProfileStoreError::ProfileFileTooLarge(
            metadata.len(),
        ));
    }
    let content = fs::read_to_string(path)?;
    let profile = serde_json::from_str::<PersonaProfile>(&content)?;
    profile
        .validate()
        .map_err(PersonaProfileStoreError::InvalidProfile)?;
    Ok(profile)
}

fn apply_soul_file(
    profile: &mut PersonaProfile,
    path: &Path,
) -> Result<(), PersonaProfileStoreError> {
    if !path.exists() {
        return Ok(());
    }
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(PersonaProfileStoreError::InvalidProfile(format!(
            "persona soul path is not a regular file: {}",
            path.display()
        )));
    }
    if metadata.len() > MAX_SOUL_FILE_BYTES {
        return Err(PersonaProfileStoreError::SoulFileTooLarge(metadata.len()));
    }
    let soul = fs::read_to_string(path)?;
    profile.version = env!("CARGO_PKG_VERSION").to_string();
    profile.authoritative_soul = true;
    profile.layers.identity.clear();
    profile.layers.soul = soul;
    profile.layers.values.clear();
    profile.layers.voice.clear();
    profile.layers.companion_style.clear();
    profile.layers.work_style.clear();
    profile.layers.boundaries.clear();
    profile.layers.addressing.clear();
    profile.companion_rules = PersonaCompanionRules::default();
    profile.constraints.clear();
    profile.default_companion_strength = CompanionStrength::Balanced;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standalone_soul_file_replaces_profile_soul_without_rewriting_it() {
        let unique = format!(
            "yunxi-persona-soul-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        );
        let directory = std::env::temp_dir().join(unique);
        fs::create_dir(&directory).expect("create test directory");
        let path = directory.join("soul.txt");
        let original = b"line one\r\nline two <soul> & exact\n";
        fs::write(&path, original).expect("write soul fixture");

        let mut profile = yunxi_companion_strong();
        apply_soul_file(&mut profile, &path).expect("load soul file");

        assert!(profile.authoritative_soul);
        assert_eq!(profile.layers.soul.as_bytes(), original);
        assert!(profile.layers.identity.is_empty());
        assert!(profile.layers.values.is_empty());
        assert!(profile.layers.voice.is_empty());
        assert!(profile.layers.companion_style.is_empty());
        assert!(profile.layers.work_style.is_empty());
        assert!(profile.layers.boundaries.is_empty());
        assert!(profile.layers.addressing.is_empty());
        assert_eq!(profile.companion_rules, PersonaCompanionRules::default());
        assert!(profile.constraints.is_empty());
        assert_eq!(
            profile.default_companion_strength,
            CompanionStrength::Balanced
        );
        assert_eq!(fs::read(&path).expect("read soul fixture"), original);
        fs::remove_file(&path).expect("remove soul fixture");
        fs::remove_dir(&directory).expect("remove test directory");
    }
}

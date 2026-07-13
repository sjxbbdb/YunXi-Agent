use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PersonaSettings {
    pub persona_enabled: bool,
    pub memory_enabled: bool,
    pub active_profile: String,
}

impl Default for PersonaSettings {
    fn default() -> Self {
        Self {
            persona_enabled: true,
            memory_enabled: false,
            active_profile: "yunxi_companion_strong".to_string(),
        }
    }
}

impl PersonaSettings {
    pub fn load() -> Self {
        let path = settings_path();
        let mut settings = std::fs::read_to_string(&path)
            .ok()
            .map(|content| Self::parse_lossy(&content))
            .unwrap_or_default();
        if let Ok(value) = std::env::var("YUNXI_PERSONA_ENABLED") {
            settings.persona_enabled = env_bool(&value, settings.persona_enabled);
        }
        if let Ok(value) = std::env::var("YUNXI_MEMORY_ENABLED") {
            settings.memory_enabled = env_bool(&value, settings.memory_enabled);
        }
        settings
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = settings_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.to_config_string())
    }

    pub fn parse_lossy(content: &str) -> Self {
        let mut settings = Self::default();
        for line in content.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim().trim_matches('"');
            match key {
                "persona_enabled" => {
                    settings.persona_enabled = env_bool(value, settings.persona_enabled)
                }
                "memory_enabled" => {
                    settings.memory_enabled = env_bool(value, settings.memory_enabled)
                }
                "active_profile" if !value.is_empty() => {
                    settings.active_profile = value.to_string()
                }
                _ => {}
            }
        }
        settings
    }

    fn to_config_string(&self) -> String {
        format!(
            "persona_enabled = {}\nmemory_enabled = {}\nactive_profile = \"{}\"\n",
            self.persona_enabled, self.memory_enabled, self.active_profile
        )
    }
}

pub fn yunxi_home_dir() -> PathBuf {
    if let Ok(path) = std::env::var("YUNXI_HOME") {
        return PathBuf::from(path);
    }
    if let Ok(path) = std::env::var("USERPROFILE") {
        return PathBuf::from(path).join(".yunxi");
    }
    if let Ok(path) = std::env::var("HOME") {
        return PathBuf::from(path).join(".yunxi");
    }
    PathBuf::from(".yunxi")
}

fn settings_path() -> PathBuf {
    yunxi_home_dir().join("persona").join("config.toml")
}

fn env_bool(value: &str, default: bool) -> bool {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => default,
    }
}

use yunxi_agent_core::{AgentConfig, AgentError, AgentResult, BackendKind};
use yunxi_agent_provider::ProviderBootstrap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProviderMode {
    Auto,
    ForcedLive,
    ForcedOffline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProviderModeSource {
    AutoLive,
    ForcedLive,
    ForcedOffline,
    AutoOffline,
}

impl ProviderModeSource {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::AutoLive => "auto_live",
            Self::ForcedLive => "forced_live",
            Self::ForcedOffline => "forced_offline",
            Self::AutoOffline => "auto_offline",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProviderSelection {
    pub live: bool,
    pub source: ProviderModeSource,
    pub provider: String,
    pub model: String,
}

impl ProviderSelection {
    pub(crate) fn apply_to_config(&self, mut config: AgentConfig) -> AgentConfig {
        if self.live {
            config.provider = Some(self.provider.clone());
            config.model = Some(self.model.clone());
        }
        config
    }

    pub(crate) fn is_offline_runtime(&self) -> bool {
        !self.live && self.provider == "offline"
    }

    pub(crate) fn auto_fallback_warning(&self) -> Option<&'static str> {
        (self.source == ProviderModeSource::AutoOffline).then_some(
            "[warning] provider auto mode did not find live credentials; using offline static runtime, no model call will be made",
        )
    }
}

impl ProviderMode {
    pub(crate) fn from_flags(provider_live: bool, offline: bool) -> Self {
        if provider_live {
            Self::ForcedLive
        } else if offline {
            Self::ForcedOffline
        } else {
            Self::Auto
        }
    }

    pub(crate) fn resolve(
        self,
        backend: BackendKind,
        config: &AgentConfig,
    ) -> AgentResult<ProviderSelection> {
        if backend != BackendKind::Yunxi {
            return Ok(ProviderSelection {
                live: false,
                source: match self {
                    Self::ForcedLive => ProviderModeSource::ForcedLive,
                    Self::ForcedOffline => ProviderModeSource::ForcedOffline,
                    Self::Auto => ProviderModeSource::AutoOffline,
                },
                provider: format!("{backend:?}").to_ascii_lowercase(),
                model: config
                    .model
                    .clone()
                    .unwrap_or_else(|| "default".to_string()),
            });
        }

        if self == Self::ForcedOffline {
            return Ok(ProviderSelection {
                live: false,
                source: ProviderModeSource::ForcedOffline,
                provider: "offline".to_string(),
                model: config
                    .model
                    .clone()
                    .unwrap_or_else(|| "static".to_string()),
            });
        }

        let bootstrap = ProviderBootstrap::from_agent_config(config);
        let credentials_configured = bootstrap.credentials_configured();
        if self == Self::ForcedLive && !credentials_configured {
            return Err(AgentError::Provider {
                provider: bootstrap.config.name.clone(),
                status: None,
                classification: "missing_credentials".to_string(),
                message: format!(
                    "live provider {} credentials are not configured",
                    bootstrap.config.name
                ),
            });
        }

        let live = self == Self::ForcedLive || credentials_configured;
        Ok(ProviderSelection {
            live,
            source: match (self, live) {
                (Self::ForcedLive, _) => ProviderModeSource::ForcedLive,
                (Self::Auto, true) => ProviderModeSource::AutoLive,
                (Self::Auto, false) => ProviderModeSource::AutoOffline,
                (Self::ForcedOffline, _) => ProviderModeSource::ForcedOffline,
            },
            provider: if live {
                bootstrap.config.name
            } else {
                "offline".to_string()
            },
            model: if live {
                bootstrap.config.model
            } else {
                config
                    .model
                    .clone()
                    .unwrap_or_else(|| "static".to_string())
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn live_selection_materializes_provider_and_model() {
        let selection = ProviderSelection {
            live: true,
            source: ProviderModeSource::AutoLive,
            provider: "deepseek".to_string(),
            model: "deepseek-v4-flash".to_string(),
        };

        let config = selection.apply_to_config(AgentConfig::new(PathBuf::from(".")));

        assert_eq!(config.provider.as_deref(), Some("deepseek"));
        assert_eq!(config.model.as_deref(), Some("deepseek-v4-flash"));
    }

    #[test]
    fn offline_selection_preserves_existing_config() {
        let selection = ProviderSelection {
            live: false,
            source: ProviderModeSource::ForcedOffline,
            provider: "offline".to_string(),
            model: "static".to_string(),
        };
        let original = AgentConfig::new(PathBuf::from("."))
            .with_provider("configured-provider")
            .with_model("configured-model");

        let config = selection.apply_to_config(original.clone());

        assert_eq!(config, original);
    }
}

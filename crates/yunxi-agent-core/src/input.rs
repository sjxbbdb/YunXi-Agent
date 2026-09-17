use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentInputModality {
    #[default]
    Text,
    Voice,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentInputChannel {
    #[default]
    Local,
    Weixin,
}

impl AgentInputChannel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Weixin => "weixin",
        }
    }
}

impl AgentInputModality {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Voice => "voice",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentInput {
    pub prompt: String,
    #[serde(default)]
    pub modality: AgentInputModality,
    #[serde(default)]
    pub channel: AgentInputChannel,
}

impl AgentInput {
    pub fn text(prompt: impl Into<String>) -> Self {
        Self::with_modality(prompt, AgentInputModality::Text)
    }

    pub fn voice(prompt: impl Into<String>) -> Self {
        Self::with_modality(prompt, AgentInputModality::Voice)
    }

    pub fn with_modality(prompt: impl Into<String>, modality: AgentInputModality) -> Self {
        Self::with_channel(prompt, modality, AgentInputChannel::Local)
    }

    pub fn with_channel(
        prompt: impl Into<String>,
        modality: AgentInputModality,
        channel: AgentInputChannel,
    ) -> Self {
        Self {
            prompt: prompt.into(),
            modality,
            channel,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_preserve_input_modality() {
        assert_eq!(AgentInput::text("typed").modality, AgentInputModality::Text);
        assert_eq!(AgentInput::text("typed").channel, AgentInputChannel::Local);
        assert_eq!(
            AgentInput::voice("spoken").modality,
            AgentInputModality::Voice
        );
        assert_eq!(
            AgentInput::with_channel(
                "wechat",
                AgentInputModality::Voice,
                AgentInputChannel::Weixin,
            )
            .channel,
            AgentInputChannel::Weixin
        );
    }
}

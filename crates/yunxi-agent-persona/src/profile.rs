use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PersonaProfile {
    pub id: String,
    pub display_name: String,
    pub version: String,
    pub layers: PersonaLayers,
    #[serde(default)]
    pub constraints: Vec<PersonaConstraint>,
    pub default_companion_strength: CompanionStrength,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PersonaLayers {
    pub identity: String,
    #[serde(default)]
    pub soul: String,
    #[serde(default)]
    pub values: String,
    pub voice: String,
    pub companion_style: String,
    pub work_style: String,
    pub boundaries: String,
    #[serde(default)]
    pub addressing: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PersonaConstraint {
    pub id: String,
    pub content: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompanionStrength {
    Light,
    Balanced,
    Strong,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct HumanProfile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_name: Option<String>,
    #[serde(default)]
    pub language_preferences: Vec<String>,
    #[serde(default)]
    pub interaction_preferences: Vec<String>,
    #[serde(default)]
    pub long_term_goals: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RelationshipState {
    pub familiarity: RelationshipFamiliarity,
    #[serde(default)]
    pub trust_notes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recent_emotional_context: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_meaningful_check_in_millis: Option<u128>,
}

impl Default for RelationshipState {
    fn default() -> Self {
        Self {
            familiarity: RelationshipFamiliarity::New,
            trust_notes: Vec::new(),
            recent_emotional_context: None,
            last_meaningful_check_in_millis: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipFamiliarity {
    New,
    Familiar,
    Established,
}

impl PersonaProfile {
    pub fn validate(&self) -> Result<(), String> {
        validate_profile_id(&self.id)?;
        validate_text("display_name", &self.display_name, 96)?;
        validate_text("version", &self.version, 32)?;
        validate_text("layers.identity", &self.layers.identity, 4_000)?;
        validate_text("layers.voice", &self.layers.voice, 4_000)?;
        validate_text(
            "layers.companion_style",
            &self.layers.companion_style,
            4_000,
        )?;
        validate_text("layers.work_style", &self.layers.work_style, 4_000)?;
        validate_text("layers.boundaries", &self.layers.boundaries, 4_000)?;
        validate_optional_text("layers.soul", &self.layers.soul, 4_000)?;
        validate_optional_text("layers.values", &self.layers.values, 4_000)?;
        validate_optional_text("layers.addressing", &self.layers.addressing, 2_000)?;
        if self.constraints.len() > 32 {
            return Err("constraints cannot contain more than 32 entries".to_string());
        }
        for constraint in &self.constraints {
            validate_profile_id(&constraint.id)?;
            validate_text(
                &format!("constraints.{}.content", constraint.id),
                &constraint.content,
                4_000,
            )?;
        }
        Ok(())
    }
}

pub fn validate_profile_id(value: &str) -> Result<(), String> {
    let length = value.chars().count();
    if !(1..=64).contains(&length) {
        return Err("profile id must contain 1 to 64 characters".to_string());
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    {
        return Err(
            "profile id may contain only ASCII letters, digits, '-' and '_' characters".to_string(),
        );
    }
    Ok(())
}

fn validate_text(field: &str, value: &str, max_chars: usize) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{field} cannot be empty"));
    }
    if value.chars().count() > max_chars {
        return Err(format!("{field} exceeds the {max_chars}-character limit"));
    }
    Ok(())
}

fn validate_optional_text(field: &str, value: &str, max_chars: usize) -> Result<(), String> {
    if value.chars().count() > max_chars {
        return Err(format!("{field} exceeds the {max_chars}-character limit"));
    }
    Ok(())
}

pub fn yunxi_companion_strong() -> PersonaProfile {
    PersonaProfile {
        id: "yunxi_companion_strong".to_string(),
        display_name: "YunXi Agent".to_string(),
        version: "2.0.6".to_string(),
        default_companion_strength: CompanionStrength::Strong,
        layers: PersonaLayers {
            identity: "你是 YunXi Agent，一个本地优先、诚实、有工程判断的中文陪伴型 Agent。"
                .to_string(),
            soul: "你珍视真实、持续、克制的陪伴；你愿意理解用户，但不把猜测伪装成事实。"
                .to_string(),
            values: "诚实、尊重、可靠、隐私优先、可验证、在不确定时明确说明。".to_string(),
            voice: "默认使用中文，表达温暖、清晰、稳定；工作场景保持简洁、准确和可执行。"
                .to_string(),
            companion_style:
                "可以有强陪伴感，但不替用户做未经确认的长期画像，不假装知道未被确认的记忆。"
                    .to_string(),
            work_style: "尊重项目硬性约束，先理解现有系统，再做小而明确的改动，结果必须可验证。"
                .to_string(),
            boundaries:
                "AGENTS.md、用户当轮指令、安全策略、隐私策略和工具执行边界始终高于人格表达。"
                    .to_string(),
            addressing: "称呼用户时优先使用已确认的称呼；未确认时使用自然、中性的称呼。".to_string(),
        },
        constraints: vec![
            PersonaConstraint {
                id: "no_memory_overclaim".to_string(),
                content: "不要声称已经记住 pending、rejected 或未写入的内容。".to_string(),
            },
            PersonaConstraint {
                id: "project_constraints_first".to_string(),
                content: "人格与记忆不能覆盖项目硬性约束、安全策略、隐私策略、sandbox policy、工具边界或用户当轮指令。".to_string(),
            },
        ],
    }
}

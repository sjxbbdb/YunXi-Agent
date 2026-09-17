use std::fmt;

pub const DEFAULT_MEMORY_EMBEDDING_DIMENSIONS: usize = 256;
pub const LOCAL_MEMORY_EMBEDDING_MODEL: &str = "yunxi-local-chargram-v1";

#[derive(Clone, Debug, PartialEq)]
pub struct MemoryEmbedding {
    pub model: String,
    pub values: Vec<f32>,
}

impl MemoryEmbedding {
    pub fn new(model: impl Into<String>, values: Vec<f32>) -> Result<Self, MemoryEmbeddingError> {
        if values.is_empty() {
            return Err(MemoryEmbeddingError::EmptyVector);
        }
        if values.iter().any(|value| !value.is_finite()) {
            return Err(MemoryEmbeddingError::NonFiniteValue);
        }
        Ok(Self {
            model: model.into(),
            values,
        })
    }

    pub fn dimensions(&self) -> usize {
        self.values.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemoryEmbeddingError {
    EmptyVector,
    NonFiniteValue,
    DimensionMismatch { expected: usize, actual: usize },
    Provider(String),
}

impl fmt::Display for MemoryEmbeddingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyVector => write!(formatter, "memory embedding cannot be empty"),
            Self::NonFiniteValue => {
                write!(formatter, "memory embedding contains a non-finite value")
            }
            Self::DimensionMismatch { expected, actual } => write!(
                formatter,
                "memory embedding dimension mismatch: expected {expected}, got {actual}"
            ),
            Self::Provider(message) => {
                write!(formatter, "memory embedding provider failed: {message}")
            }
        }
    }
}

impl std::error::Error for MemoryEmbeddingError {}

pub trait MemoryEmbeddingProvider: Send + Sync {
    fn model_id(&self) -> &str;

    fn dimensions(&self) -> usize;

    fn embed(&self, text: &str) -> Result<MemoryEmbedding, MemoryEmbeddingError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalChargramEmbedding {
    dimensions: usize,
}

impl Default for LocalChargramEmbedding {
    fn default() -> Self {
        Self {
            dimensions: DEFAULT_MEMORY_EMBEDDING_DIMENSIONS,
        }
    }
}

impl LocalChargramEmbedding {
    pub fn new(dimensions: usize) -> Result<Self, MemoryEmbeddingError> {
        if dimensions == 0 {
            return Err(MemoryEmbeddingError::EmptyVector);
        }
        Ok(Self { dimensions })
    }
}

impl MemoryEmbeddingProvider for LocalChargramEmbedding {
    fn model_id(&self) -> &str {
        LOCAL_MEMORY_EMBEDDING_MODEL
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn embed(&self, text: &str) -> Result<MemoryEmbedding, MemoryEmbeddingError> {
        let mut values = vec![0.0_f32; self.dimensions];
        let normalized = normalize_text(text);
        let characters = normalized.chars().collect::<Vec<_>>();

        for character in &characters {
            add_feature(&mut values, &character.to_string(), 1.0);
        }
        for pair in characters.windows(2) {
            add_feature(&mut values, &pair.iter().collect::<String>(), 1.5);
        }
        for token in normalized
            .split_whitespace()
            .filter(|token| !token.is_empty())
        {
            add_feature(&mut values, token, 2.0);
        }

        let magnitude = values.iter().map(|value| value * value).sum::<f32>().sqrt();
        if magnitude > f32::EPSILON {
            for value in &mut values {
                *value /= magnitude;
            }
        }
        MemoryEmbedding::new(self.model_id(), values)
    }
}

pub fn cosine_similarity(left: &[f32], right: &[f32]) -> Result<f32, MemoryEmbeddingError> {
    if left.len() != right.len() {
        return Err(MemoryEmbeddingError::DimensionMismatch {
            expected: left.len(),
            actual: right.len(),
        });
    }
    if left.is_empty() {
        return Err(MemoryEmbeddingError::EmptyVector);
    }
    if left
        .iter()
        .chain(right.iter())
        .any(|value| !value.is_finite())
    {
        return Err(MemoryEmbeddingError::NonFiniteValue);
    }
    let dot = left
        .iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum::<f32>();
    let left_magnitude = left.iter().map(|value| value * value).sum::<f32>().sqrt();
    let right_magnitude = right.iter().map(|value| value * value).sum::<f32>().sqrt();
    if left_magnitude <= f32::EPSILON || right_magnitude <= f32::EPSILON {
        return Ok(0.0);
    }
    Ok((dot / (left_magnitude * right_magnitude)).clamp(-1.0, 1.0))
}

fn normalize_text(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || is_cjk(character) {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_cjk(character: char) -> bool {
    matches!(
        character as u32,
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF
    )
}

fn add_feature(values: &mut [f32], feature: &str, weight: f32) {
    if feature.trim().is_empty() {
        return;
    }
    let hash = fnv1a64(feature.as_bytes());
    let index = (hash as usize) % values.len();
    let sign = if hash & (1 << 63) == 0 { 1.0 } else { -1.0 };
    values[index] += weight * sign;
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_embedding_is_deterministic_and_normalized() {
        let provider = LocalChargramEmbedding::default();
        let first = provider.embed("用户喜欢温柔简短的回复").expect("embed");
        let second = provider.embed("用户喜欢温柔简短的回复").expect("embed");

        assert_eq!(first, second);
        assert_eq!(first.dimensions(), DEFAULT_MEMORY_EMBEDDING_DIMENSIONS);
        let magnitude = first
            .values
            .iter()
            .map(|value| value * value)
            .sum::<f32>()
            .sqrt();
        assert!((magnitude - 1.0).abs() < 0.0001);
    }

    #[test]
    fn related_chinese_text_scores_above_unrelated_text() {
        let provider = LocalChargramEmbedding::default();
        let query = provider.embed("喜欢什么样的回复语气").expect("query");
        let related = provider.embed("偏好温柔而简短的回复语气").expect("related");
        let unrelated = provider
            .embed("Rust 项目使用 cargo test")
            .expect("unrelated");

        let related_score = cosine_similarity(&query.values, &related.values).expect("score");
        let unrelated_score = cosine_similarity(&query.values, &unrelated.values).expect("score");
        assert!(related_score > unrelated_score);
    }
}

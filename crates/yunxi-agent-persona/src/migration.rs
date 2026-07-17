use crate::dedup::dedup_key_for_record;
use crate::memory::{
    MemoryKind, MemoryRecord, MemoryScope, MemorySensitivity, MemoryStatus, SCHEMA_VERSION,
};
use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub enum MemoryMigrationResult {
    Record(MemoryRecord),
    RecordWithWarning {
        record: MemoryRecord,
        warning: String,
    },
    Skip {
        warning: String,
    },
}

pub fn migrate_memory_record_value(value: Value) -> MemoryMigrationResult {
    let schema_version = value
        .get("schema_version")
        .and_then(Value::as_u64)
        .map(|value| value as u32);
    match schema_version {
        Some(SCHEMA_VERSION) => match serde_json::from_value::<MemoryRecord>(value) {
            Ok(mut record) => {
                record.ensure_dedup_metadata();
                MemoryMigrationResult::Record(record)
            }
            Err(error) => MemoryMigrationResult::Skip {
                warning: format!("failed to parse memory schema v{SCHEMA_VERSION}: {error}"),
            },
        },
        Some(2) => migrate_v2(value),
        Some(1) => migrate_v1(value, None),
        None => migrate_v1(
            value,
            Some("legacy memory record missing schema_version; migrated as v1".to_string()),
        ),
        Some(version) => MemoryMigrationResult::Skip {
            warning: format!(
                "unsupported future memory schema_version={version}; current={SCHEMA_VERSION}"
            ),
        },
    }
}

fn migrate_v2(value: Value) -> MemoryMigrationResult {
    match serde_json::from_value::<MemoryRecordV2>(value) {
        Ok(v2) => {
            let mut record =
                MemoryRecord::new(v2.id, v2.scope, v2.kind, v2.content, v2.created_at_millis);
            record.source_session_id = v2.source_session_id;
            record.confidence = v2.confidence;
            record.importance = v2.importance;
            record.sensitivity = v2.sensitivity;
            record.status = v2.status;
            record.updated_at_millis = v2.updated_at_millis;
            record.dedup_key = v2.dedup_key;
            record.revision = v2.revision;
            record.merged_count = v2.merged_count;
            record.ensure_dedup_metadata();
            MemoryMigrationResult::Record(record)
        }
        Err(error) => MemoryMigrationResult::Skip {
            warning: format!("failed to migrate memory schema v2: {error}"),
        },
    }
}

fn migrate_v1(value: Value, warning: Option<String>) -> MemoryMigrationResult {
    match serde_json::from_value::<MemoryRecordV1>(value) {
        Ok(v1) => {
            let mut record =
                MemoryRecord::new(v1.id, v1.scope, v1.kind, v1.content, v1.created_at_millis);
            record.source_session_id = v1.source_session_id;
            record.confidence = v1.confidence;
            record.importance = v1.importance;
            record.sensitivity = v1.sensitivity;
            record.status = v1.status;
            record.updated_at_millis = v1.updated_at_millis;
            record.dedup_key = dedup_key_for_record(&record).as_storage_key();
            record.ensure_dedup_metadata();
            if let Some(warning) = warning {
                MemoryMigrationResult::RecordWithWarning { record, warning }
            } else {
                MemoryMigrationResult::Record(record)
            }
        }
        Err(error) => MemoryMigrationResult::Skip {
            warning: format!("failed to migrate memory schema v1: {error}"),
        },
    }
}

#[derive(Clone, Debug, Deserialize)]
struct MemoryRecordV2 {
    id: String,
    scope: MemoryScope,
    kind: MemoryKind,
    content: String,
    #[serde(default)]
    source_session_id: Option<String>,
    confidence: f32,
    importance: f32,
    sensitivity: MemorySensitivity,
    status: MemoryStatus,
    created_at_millis: u128,
    updated_at_millis: u128,
    #[serde(default)]
    dedup_key: String,
    #[serde(default = "default_revision")]
    revision: u32,
    #[serde(default = "default_merged_count")]
    merged_count: u32,
}

#[derive(Clone, Debug, Deserialize)]
struct MemoryRecordV1 {
    id: String,
    scope: MemoryScope,
    kind: MemoryKind,
    content: String,
    #[serde(default)]
    source_session_id: Option<String>,
    confidence: f32,
    importance: f32,
    sensitivity: MemorySensitivity,
    status: MemoryStatus,
    created_at_millis: u128,
    updated_at_millis: u128,
}

fn default_revision() -> u32 {
    1
}

fn default_merged_count() -> u32 {
    1
}

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

fn migrate_v1(value: Value, warning: Option<String>) -> MemoryMigrationResult {
    match serde_json::from_value::<MemoryRecordV1>(value) {
        Ok(v1) => {
            let mut record = MemoryRecord {
                id: v1.id,
                schema_version: SCHEMA_VERSION,
                scope: v1.scope,
                kind: v1.kind,
                content: v1.content,
                source_session_id: v1.source_session_id,
                confidence: v1.confidence,
                importance: v1.importance,
                sensitivity: v1.sensitivity,
                status: v1.status,
                created_at_millis: v1.created_at_millis,
                updated_at_millis: v1.updated_at_millis,
                dedup_key: String::new(),
                revision: 1,
                merged_count: 1,
            };
            record.dedup_key = dedup_key_for_record(&record).as_storage_key();
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

use rusqlite::{Connection, params};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Duration;
use yunxi_agent_core::{AgentError, AgentResult};
use yunxi_agent_persona::{
    MemoryEmbeddingProvider, MemoryLayer, MemoryRecord, MemoryScope, MemorySensitivity,
    MemoryStatus, cosine_similarity, yunxi_home_dir,
};

const VECTOR_DATABASE_FILE: &str = "long-term-vectors.sqlite3";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqliteMemoryVectorStore {
    global_database: PathBuf,
    workspace_database: PathBuf,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MemoryVectorSyncSummary {
    pub indexed: usize,
    pub unchanged: usize,
    pub removed: usize,
    pub excluded: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemoryVectorMatch {
    pub memory_id: String,
    pub score: f32,
    pub embedding_model: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MemoryVectorSearch {
    pub matches: Vec<MemoryVectorMatch>,
    pub warnings: Vec<String>,
}

impl SqliteMemoryVectorStore {
    pub fn for_workspace(cwd: impl AsRef<Path>) -> Self {
        Self {
            global_database: yunxi_home_dir().join("memory").join(VECTOR_DATABASE_FILE),
            workspace_database: cwd
                .as_ref()
                .join(".yunxi")
                .join("memory")
                .join(VECTOR_DATABASE_FILE),
        }
    }

    pub fn new(
        global_database: impl Into<PathBuf>,
        workspace_database: impl Into<PathBuf>,
    ) -> Self {
        Self {
            global_database: global_database.into(),
            workspace_database: workspace_database.into(),
        }
    }

    pub fn global_database(&self) -> &Path {
        &self.global_database
    }

    pub fn workspace_database(&self) -> &Path {
        &self.workspace_database
    }

    pub fn sync_records<P: MemoryEmbeddingProvider>(
        &self,
        records: &[MemoryRecord],
        provider: &P,
    ) -> AgentResult<MemoryVectorSyncSummary> {
        let global = records
            .iter()
            .filter(|record| !matches!(record.scope, MemoryScope::Workspace { .. }))
            .cloned()
            .collect::<Vec<_>>();
        let workspace = records
            .iter()
            .filter(|record| matches!(record.scope, MemoryScope::Workspace { .. }))
            .cloned()
            .collect::<Vec<_>>();
        let mut summary = sync_database(&self.global_database, &global, provider)?;
        let workspace_summary = sync_database(&self.workspace_database, &workspace, provider)?;
        summary.indexed += workspace_summary.indexed;
        summary.unchanged += workspace_summary.unchanged;
        summary.removed += workspace_summary.removed;
        summary.excluded += workspace_summary.excluded;
        Ok(summary)
    }

    pub fn search<P: MemoryEmbeddingProvider>(
        &self,
        query: &str,
        top_k: usize,
        provider: &P,
    ) -> AgentResult<MemoryVectorSearch> {
        if query.trim().is_empty() || top_k == 0 {
            return Ok(MemoryVectorSearch::default());
        }
        let query_embedding = provider
            .embed(query)
            .map_err(|error| vector_error("failed to embed memory query", error))?;
        let mut search = MemoryVectorSearch::default();
        for path in [&self.global_database, &self.workspace_database] {
            if !path.is_file() {
                continue;
            }
            let connection = open_connection(path)?;
            initialize_schema(&connection)?;
            let mut statement = connection
                .prepare(
                    "SELECT memory_id, embedding_model, dimensions, vector
                     FROM memory_vectors
                     WHERE status = 'active' AND sensitivity <> 'high'",
                )
                .map_err(|error| sqlite_error(path, "prepare vector search", error))?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, Vec<u8>>(3)?,
                    ))
                })
                .map_err(|error| sqlite_error(path, "query vectors", error))?;
            for row in rows {
                let (memory_id, model, dimensions, blob) =
                    row.map_err(|error| sqlite_error(path, "read vector row", error))?;
                if model != provider.model_id() {
                    search.warnings.push(format!(
                        "memory vector {memory_id} uses stale embedding model {model}"
                    ));
                    continue;
                }
                let dimensions = usize::try_from(dimensions).unwrap_or_default();
                if dimensions != provider.dimensions() {
                    search.warnings.push(format!(
                        "memory vector {memory_id} has {dimensions} dimensions; expected {}",
                        provider.dimensions()
                    ));
                    continue;
                }
                let values = match vector_from_blob(&blob, dimensions) {
                    Ok(values) => values,
                    Err(message) => {
                        search
                            .warnings
                            .push(format!("memory vector {memory_id} is invalid: {message}"));
                        continue;
                    }
                };
                let score = cosine_similarity(&query_embedding.values, &values)
                    .map_err(|error| vector_error("failed to score memory vector", error))?;
                if score > 0.0 {
                    search.matches.push(MemoryVectorMatch {
                        memory_id,
                        score,
                        embedding_model: model,
                    });
                }
            }
        }
        let mut best_by_id = BTreeMap::<String, MemoryVectorMatch>::new();
        for candidate in search.matches.drain(..) {
            match best_by_id.entry(candidate.memory_id.clone()) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(candidate);
                }
                std::collections::btree_map::Entry::Occupied(mut entry)
                    if candidate.score > entry.get().score =>
                {
                    entry.insert(candidate);
                }
                std::collections::btree_map::Entry::Occupied(_) => {}
            }
        }
        search.matches = best_by_id.into_values().collect();
        search.matches.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.memory_id.cmp(&right.memory_id))
        });
        search.matches.truncate(top_k);
        Ok(search)
    }
}

fn sync_database<P: MemoryEmbeddingProvider>(
    path: &Path,
    records: &[MemoryRecord],
    provider: &P,
) -> AgentResult<MemoryVectorSyncSummary> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| AgentError::Execution {
            message: format!(
                "failed to create vector memory directory {}: {error}",
                parent.display()
            ),
        })?;
    }
    let mut connection = open_connection(path)?;
    initialize_schema(&connection)?;
    let transaction = connection
        .transaction()
        .map_err(|error| sqlite_error(path, "begin vector sync", error))?;
    let existing = {
        let mut statement = transaction
            .prepare(
                "SELECT memory_id, revision, updated_at_millis, content_hash,
                        embedding_model, dimensions, status, scope, kind,
                        layer, sensitivity
                 FROM memory_vectors",
            )
            .map_err(|error| sqlite_error(path, "prepare vector inventory", error))?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    StoredVectorMetadata {
                        revision: row.get::<_, i64>(1)?,
                        updated_at_millis: row.get::<_, String>(2)?,
                        content_hash: row.get::<_, String>(3)?,
                        embedding_model: row.get::<_, String>(4)?,
                        dimensions: row.get::<_, i64>(5)?,
                        status: row.get::<_, String>(6)?,
                        scope: row.get::<_, String>(7)?,
                        kind: row.get::<_, String>(8)?,
                        layer: row.get::<_, String>(9)?,
                        sensitivity: row.get::<_, String>(10)?,
                    },
                ))
            })
            .map_err(|error| sqlite_error(path, "query vector inventory", error))?;
        rows.collect::<Result<BTreeMap<_, _>, _>>()
            .map_err(|error| sqlite_error(path, "read vector inventory", error))?
    };

    let eligible = records
        .iter()
        .filter(|record| should_index(record))
        .collect::<Vec<_>>();
    let wanted_ids = eligible
        .iter()
        .map(|record| record.id.clone())
        .collect::<BTreeSet<_>>();
    let mut summary = MemoryVectorSyncSummary {
        excluded: records.len().saturating_sub(eligible.len()),
        ..MemoryVectorSyncSummary::default()
    };

    for memory_id in existing.keys().filter(|id| !wanted_ids.contains(*id)) {
        transaction
            .execute(
                "DELETE FROM memory_vectors WHERE memory_id = ?1",
                params![memory_id],
            )
            .map_err(|error| sqlite_error(path, "remove stale vector", error))?;
        summary.removed += 1;
    }

    for record in eligible {
        let metadata = StoredVectorMetadata::from_record(record, provider);
        if existing.get(&record.id) == Some(&metadata) {
            summary.unchanged += 1;
            continue;
        }
        let embedding = provider
            .embed(&record.content)
            .map_err(|error| vector_error("failed to embed long-term memory", error))?;
        if embedding.dimensions() != provider.dimensions() {
            return Err(AgentError::Execution {
                message: format!(
                    "memory embedding provider {} declared {} dimensions but returned {}",
                    provider.model_id(),
                    provider.dimensions(),
                    embedding.dimensions()
                ),
            });
        }
        transaction
            .execute(
                "INSERT INTO memory_vectors (
                    memory_id, revision, updated_at_millis, content_hash,
                    embedding_model, dimensions, vector, status, scope, kind,
                    layer, sensitivity
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                 ON CONFLICT(memory_id) DO UPDATE SET
                    revision = excluded.revision,
                    updated_at_millis = excluded.updated_at_millis,
                    content_hash = excluded.content_hash,
                    embedding_model = excluded.embedding_model,
                    dimensions = excluded.dimensions,
                    vector = excluded.vector,
                    status = excluded.status,
                    scope = excluded.scope,
                    kind = excluded.kind,
                    layer = excluded.layer,
                    sensitivity = excluded.sensitivity",
                params![
                    record.id,
                    i64::from(record.revision),
                    record.updated_at_millis.to_string(),
                    metadata.content_hash,
                    embedding.model,
                    i64::try_from(embedding.dimensions()).unwrap_or(i64::MAX),
                    vector_to_blob(&embedding.values),
                    memory_status_label(record.status),
                    record.scope.label(),
                    memory_kind_label(record),
                    memory_layer_label(record.layer),
                    memory_sensitivity_label(record.sensitivity),
                ],
            )
            .map_err(|error| sqlite_error(path, "upsert memory vector", error))?;
        summary.indexed += 1;
    }
    transaction
        .commit()
        .map_err(|error| sqlite_error(path, "commit vector sync", error))?;
    Ok(summary)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StoredVectorMetadata {
    revision: i64,
    updated_at_millis: String,
    content_hash: String,
    embedding_model: String,
    dimensions: i64,
    status: String,
    scope: String,
    kind: String,
    layer: String,
    sensitivity: String,
}

impl StoredVectorMetadata {
    fn from_record<P: MemoryEmbeddingProvider>(record: &MemoryRecord, provider: &P) -> Self {
        Self {
            revision: i64::from(record.revision),
            updated_at_millis: record.updated_at_millis.to_string(),
            content_hash: format!("{:016x}", fnv1a64(record.content.as_bytes())),
            embedding_model: provider.model_id().to_string(),
            dimensions: i64::try_from(provider.dimensions()).unwrap_or(i64::MAX),
            status: memory_status_label(record.status).to_string(),
            scope: record.scope.label(),
            kind: memory_kind_label(record).to_string(),
            layer: memory_layer_label(record.layer).to_string(),
            sensitivity: memory_sensitivity_label(record.sensitivity).to_string(),
        }
    }
}

fn should_index(record: &MemoryRecord) -> bool {
    matches!(record.status, MemoryStatus::Active | MemoryStatus::Archived)
        && record.sensitivity != MemorySensitivity::High
        && record.layer != MemoryLayer::Profile
        && record.scope != MemoryScope::AgentIdentity
}

fn open_connection(path: &Path) -> AgentResult<Connection> {
    let connection = Connection::open(path)
        .map_err(|error| sqlite_error(path, "open vector database", error))?;
    connection
        .busy_timeout(Duration::from_secs(2))
        .map_err(|error| sqlite_error(path, "configure vector database", error))?;
    Ok(connection)
}

fn initialize_schema(connection: &Connection) -> AgentResult<()> {
    connection
        .execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS memory_vectors (
                memory_id TEXT PRIMARY KEY,
                revision INTEGER NOT NULL,
                updated_at_millis TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                embedding_model TEXT NOT NULL,
                dimensions INTEGER NOT NULL,
                vector BLOB NOT NULL,
                status TEXT NOT NULL,
                scope TEXT NOT NULL,
                kind TEXT NOT NULL,
                layer TEXT NOT NULL,
                sensitivity TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_memory_vectors_status
                ON memory_vectors(status, sensitivity);",
        )
        .map_err(|error| AgentError::Execution {
            message: format!("failed to initialize vector memory schema: {error}"),
        })
}

fn vector_to_blob(values: &[f32]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(values.len() * std::mem::size_of::<f32>());
    for value in values {
        blob.extend_from_slice(&value.to_le_bytes());
    }
    blob
}

fn vector_from_blob(blob: &[u8], dimensions: usize) -> Result<Vec<f32>, String> {
    let expected = dimensions.saturating_mul(std::mem::size_of::<f32>());
    if blob.len() != expected {
        return Err(format!(
            "blob length is {}, expected {expected}",
            blob.len()
        ));
    }
    Ok(blob
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect())
}

fn memory_status_label(status: MemoryStatus) -> &'static str {
    match status {
        MemoryStatus::Active => "active",
        MemoryStatus::Pending => "pending",
        MemoryStatus::Rejected => "rejected",
        MemoryStatus::Archived => "archived",
    }
}

fn memory_sensitivity_label(sensitivity: MemorySensitivity) -> &'static str {
    match sensitivity {
        MemorySensitivity::Low => "low",
        MemorySensitivity::Medium => "medium",
        MemorySensitivity::High => "high",
    }
}

fn memory_layer_label(layer: MemoryLayer) -> &'static str {
    match layer {
        MemoryLayer::Profile => "profile",
        MemoryLayer::Preference => "preference",
        MemoryLayer::Relationship => "relationship",
        MemoryLayer::Workspace => "workspace",
        MemoryLayer::Episode => "episode",
        MemoryLayer::ToolTrace => "tool_trace",
        MemoryLayer::Unknown => "unknown",
    }
}

fn memory_kind_label(record: &MemoryRecord) -> &'static str {
    match record.kind {
        yunxi_agent_persona::MemoryKind::Preference => "preference",
        yunxi_agent_persona::MemoryKind::PersonalFact => "personal_fact",
        yunxi_agent_persona::MemoryKind::RelationshipNote => "relationship_note",
        yunxi_agent_persona::MemoryKind::EmotionalState => "emotional_state",
        yunxi_agent_persona::MemoryKind::Goal => "goal",
        yunxi_agent_persona::MemoryKind::ProjectContext => "project_context",
        yunxi_agent_persona::MemoryKind::Correction => "correction",
        yunxi_agent_persona::MemoryKind::Event => "event",
        yunxi_agent_persona::MemoryKind::ToolTraceSummary => "tool_trace_summary",
    }
}

fn sqlite_error(path: &Path, action: &str, error: rusqlite::Error) -> AgentError {
    AgentError::Execution {
        message: format!("failed to {action} {}: {error}", path.display()),
    }
}

fn vector_error(action: &str, error: impl std::fmt::Display) -> AgentError {
    AgentError::Execution {
        message: format!("{action}: {error}"),
    }
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
    use tempfile::TempDir;
    use yunxi_agent_persona::{
        LocalChargramEmbedding, MemoryKind, MemoryRecord, MemoryScope, MemoryStatus,
    };

    fn active(id: &str, content: &str) -> MemoryRecord {
        MemoryRecord::new(
            id,
            MemoryScope::Relationship,
            MemoryKind::RelationshipNote,
            content,
            1,
        )
        .with_status(MemoryStatus::Active)
    }

    #[test]
    fn active_long_term_records_are_indexed_and_semantically_searchable() {
        let temp = TempDir::new().expect("temp dir");
        let store = SqliteMemoryVectorStore::new(
            temp.path().join("global.sqlite3"),
            temp.path().join("workspace.sqlite3"),
        );
        let provider = LocalChargramEmbedding::default();
        let records = vec![
            active("tone", "用户偏好温柔简短的回复语气"),
            active("rust", "Rust 项目使用 cargo test 进行验证"),
        ];

        let synced = store.sync_records(&records, &provider).expect("sync");
        assert_eq!(synced.indexed, 2);
        let search = store
            .search("喜欢什么回复语气", 2, &provider)
            .expect("search");

        assert_eq!(search.matches[0].memory_id, "tone");
        assert!(search.matches[0].score > search.matches[1].score);
    }

    #[test]
    fn pending_rejected_profile_and_agent_identity_records_are_not_indexed() {
        let temp = TempDir::new().expect("temp dir");
        let store = SqliteMemoryVectorStore::new(
            temp.path().join("global.sqlite3"),
            temp.path().join("workspace.sqlite3"),
        );
        let provider = LocalChargramEmbedding::default();
        let pending = MemoryRecord::new(
            "pending",
            MemoryScope::Relationship,
            MemoryKind::Event,
            "尚未确认的事件",
            1,
        );
        let profile = MemoryRecord::new(
            "profile",
            MemoryScope::GlobalUser,
            MemoryKind::PersonalFact,
            "结构化个人档案",
            1,
        )
        .with_status(MemoryStatus::Active);
        let identity = MemoryRecord::new(
            "identity",
            MemoryScope::AgentIdentity,
            MemoryKind::Event,
            "云熙身份",
            1,
        )
        .with_status(MemoryStatus::Active);

        let summary = store
            .sync_records(&[pending, profile, identity], &provider)
            .expect("sync");
        assert_eq!(summary.indexed, 0);
        assert_eq!(summary.excluded, 3);
    }

    #[test]
    fn archived_records_remain_indexed_but_are_not_returned_by_current_search() {
        let temp = TempDir::new().expect("temp dir");
        let store = SqliteMemoryVectorStore::new(
            temp.path().join("global.sqlite3"),
            temp.path().join("workspace.sqlite3"),
        );
        let provider = LocalChargramEmbedding::default();
        let archived = active("old", "曾经偏好很长的回复").with_status(MemoryStatus::Archived);

        let summary = store.sync_records(&[archived], &provider).expect("sync");
        assert_eq!(summary.indexed, 1);
        assert!(
            store
                .search("回复偏好", 5, &provider)
                .expect("search")
                .matches
                .is_empty()
        );
    }

    #[test]
    fn sensitivity_change_removes_previously_indexed_content_without_revision_change() {
        let temp = TempDir::new().expect("temp dir");
        let store = SqliteMemoryVectorStore::new(
            temp.path().join("global.sqlite3"),
            temp.path().join("workspace.sqlite3"),
        );
        let provider = LocalChargramEmbedding::default();
        let low = active("private", "一条稍后被标记为高敏感的长期记忆");
        store
            .sync_records(&[low.clone()], &provider)
            .expect("sync low");

        let mut high = low;
        high.sensitivity = MemorySensitivity::High;
        let summary = store.sync_records(&[high], &provider).expect("sync high");

        assert_eq!(summary.removed, 1);
        assert_eq!(summary.excluded, 1);
        assert!(
            store
                .search("长期记忆", 5, &provider)
                .expect("search")
                .matches
                .is_empty()
        );
    }
}

use base64::{Engine as _, engine::general_purpose::STANDARD};
use chacha20poly1305::{
    ChaCha20Poly1305, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::BTreeMap;
use std::fmt;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use yunxi_agent_companion::{
    COMPANION_MAILBOX_SCHEMA_VERSION, CompanionMailboxContent, CompanionMailboxEntry,
    CompanionMailboxItem, LoveLetterTask, LoveLetterTaskState, MailboxCursor, MailboxItemType,
    MailboxPage, MailboxQuery, MailboxState, stable_hash64,
};
use yunxi_agent_core::{AgentError, AgentResult};

const MAILBOX_CONTENT_ALGORITHM: &str = "chacha20poly1305";
const MAILBOX_CONTENT_ALGORITHM_VERSION: u32 = 1;
const MAILBOX_CONTENT_AAD_VERSION: u32 = 1;
const NONCE_LENGTH: usize = 12;
const LOCK_RETRY_COUNT: usize = 80;
const LOCK_RETRY_DELAY: Duration = Duration::from_millis(25);
const STALE_LOCK_MILLIS: u128 = 300_000;
const DEFAULT_PAGE_LIMIT: usize = 50;
const MAX_PAGE_LIMIT: usize = 200;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CompanionMailboxSnapshot {
    pub tasks: Vec<LoveLetterTask>,
    pub items: Vec<CompanionMailboxItem>,
}

impl CompanionMailboxSnapshot {
    pub fn last_generated_at_millis(&self) -> Option<u128> {
        self.tasks
            .iter()
            .filter_map(|task| task.generated_at_millis)
            .max()
    }

    pub fn generated_since(&self, since_millis: u128) -> u32 {
        self.tasks
            .iter()
            .filter_map(|task| task.generated_at_millis)
            .filter(|generated_at| *generated_at >= since_millis)
            .count()
            .try_into()
            .unwrap_or(u32::MAX)
    }

    pub fn has_open_task(&self, max_attempts: u32) -> bool {
        self.tasks.iter().any(|task| match task.state {
            LoveLetterTaskState::Pending | LoveLetterTaskState::Generating => true,
            LoveLetterTaskState::Failed => task.generation_attempts < max_attempts,
            LoveLetterTaskState::Ready => false,
        })
    }

    pub fn latest_retry_after_millis(&self) -> Option<u128> {
        self.tasks
            .iter()
            .filter(|task| task.state == LoveLetterTaskState::Failed)
            .filter_map(|task| task.retry_after_millis)
            .max()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MailboxEnqueueOutcome {
    Enqueued(LoveLetterTask),
    Duplicate(LoveLetterTask),
}

#[derive(Clone)]
pub struct FileCompanionMailboxStore {
    root: PathBuf,
    owner_scope: String,
    data_key: [u8; 32],
}

impl fmt::Debug for FileCompanionMailboxStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FileCompanionMailboxStore")
            .field("root", &self.root)
            .field("owner_scope", &self.owner_scope)
            .field("data_key", &"<redacted>")
            .finish()
    }
}

impl FileCompanionMailboxStore {
    pub fn for_workspace(cwd: impl AsRef<Path>) -> AgentResult<Self> {
        let root = cwd.as_ref().join(".yunxi").join("companion-mailbox");
        let owner_scope = format!("workspace:{:016x}", stable_path_hash(cwd.as_ref()));
        std::fs::create_dir_all(&root).map_err(|error| {
            mailbox_error(format!("failed to create mailbox directory: {error}"))
        })?;
        let _key_init_lock = acquire_file_lock(&root.join("key-init.lock"), "mailbox key")?;
        let data_key = load_or_create_data_key(&owner_scope)?;
        Ok(Self {
            root,
            owner_scope,
            data_key,
        })
    }

    pub fn with_data_key(
        root: impl Into<PathBuf>,
        owner_scope: impl Into<String>,
        data_key: [u8; 32],
    ) -> Self {
        Self {
            root: root.into(),
            owner_scope: owner_scope.into(),
            data_key,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn owner_scope(&self) -> &str {
        &self.owner_scope
    }

    pub fn snapshot(&self) -> AgentResult<CompanionMailboxSnapshot> {
        let _lock = self.lock()?;
        self.snapshot_unlocked()
    }

    pub fn enqueue(&self, task: &LoveLetterTask) -> AgentResult<MailboxEnqueueOutcome> {
        if task.owner_scope != self.owner_scope {
            return Err(mailbox_error("task owner scope does not match mailbox"));
        }
        let _lock = self.lock()?;
        let snapshot = self.snapshot_unlocked()?;
        if let Some(existing) = snapshot
            .tasks
            .into_iter()
            .find(|existing| existing.idempotency_key == task.idempotency_key)
        {
            return Ok(MailboxEnqueueOutcome::Duplicate(existing));
        }
        append_jsonl(&self.tasks_path(), task, "love-letter task")?;
        Ok(MailboxEnqueueOutcome::Enqueued(task.clone()))
    }

    pub fn claim_next(
        &self,
        now_millis: u128,
        lease_millis: u128,
        max_attempts: u32,
    ) -> AgentResult<Option<LoveLetterTask>> {
        let _lock = self.lock()?;
        let mut snapshot = self.snapshot_unlocked()?;
        for task in &mut snapshot.tasks {
            let Some(item) = snapshot
                .items
                .iter()
                .find(|item| item.source_id == task.task_id)
            else {
                continue;
            };
            if task.state != LoveLetterTaskState::Ready {
                task.state = LoveLetterTaskState::Ready;
                task.updated_at_millis = now_millis;
                task.generated_at_millis = Some(item.created_at_millis);
                task.generation_elapsed_millis = task
                    .generation_started_at_millis
                    .map(|started_at| now_millis.saturating_sub(started_at));
                task.lease_expires_at_millis = None;
                task.retry_after_millis = None;
                task.mailbox_item_id = Some(item.item_id.clone());
                append_jsonl(&self.tasks_path(), task, "love-letter task")?;
            }
        }
        let mut candidates = snapshot
            .tasks
            .into_iter()
            .filter(|task| task.generation_attempts < max_attempts)
            .filter(|task| match task.state {
                LoveLetterTaskState::Pending => true,
                LoveLetterTaskState::Failed => task
                    .retry_after_millis
                    .is_none_or(|retry_after| retry_after <= now_millis),
                LoveLetterTaskState::Generating => task
                    .lease_expires_at_millis
                    .is_some_and(|lease_expires| lease_expires <= now_millis),
                LoveLetterTaskState::Ready => false,
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            left.created_at_millis
                .cmp(&right.created_at_millis)
                .then_with(|| left.task_id.cmp(&right.task_id))
        });
        let Some(mut task) = candidates.into_iter().next() else {
            return Ok(None);
        };
        task.state = LoveLetterTaskState::Generating;
        task.updated_at_millis = now_millis;
        task.generation_attempts = task.generation_attempts.saturating_add(1);
        task.generation_started_at_millis = Some(now_millis);
        task.generation_elapsed_millis = None;
        task.lease_expires_at_millis = Some(now_millis.saturating_add(lease_millis));
        task.retry_after_millis = None;
        task.last_error_label = None;
        append_jsonl(&self.tasks_path(), &task, "love-letter task")?;
        Ok(Some(task))
    }

    pub fn complete(
        &self,
        task_id: &str,
        content: &CompanionMailboxContent,
        now_millis: u128,
    ) -> AgentResult<CompanionMailboxItem> {
        validate_content(content)?;
        let _lock = self.lock()?;
        let snapshot = self.snapshot_unlocked()?;
        let Some(mut task) = snapshot
            .tasks
            .iter()
            .find(|task| task.task_id == task_id)
            .cloned()
        else {
            return Err(mailbox_error("love-letter task was not found"));
        };
        if let Some(existing) = snapshot
            .items
            .iter()
            .find(|item| item.source_id == task_id)
            .cloned()
        {
            if task.state != LoveLetterTaskState::Ready {
                task.state = LoveLetterTaskState::Ready;
                task.updated_at_millis = now_millis;
                task.generated_at_millis = Some(existing.created_at_millis);
                task.generation_elapsed_millis = task
                    .generation_started_at_millis
                    .map(|started_at| now_millis.saturating_sub(started_at));
                task.lease_expires_at_millis = None;
                task.retry_after_millis = None;
                task.mailbox_item_id = Some(existing.item_id.clone());
                append_jsonl(&self.tasks_path(), &task, "love-letter task")?;
            }
            return Ok(existing);
        }
        if task.state != LoveLetterTaskState::Generating {
            return Err(mailbox_error(
                "love-letter task is not leased for generation",
            ));
        }

        let content_json = serde_json::to_vec(content).map_err(|error| {
            mailbox_error(format!("failed to serialize mailbox content: {error}"))
        })?;
        let content_hash = stable_hash64(&content_json);
        let content_ref = format!(
            "content-{:016x}",
            stable_hash64(format!("{task_id}|{content_hash:016x}").as_bytes())
        );
        let item_id = format!("mail-{:016x}", stable_hash64(task_id.as_bytes()));
        let aad = content_aad(&self.owner_scope, task_id, &content_ref);
        let encrypted = encrypt_content(&self.data_key, &content_json, &aad)?;
        self.write_content_once(&content_ref, &encrypted, &content_json, &aad)?;

        let item = CompanionMailboxItem {
            schema_version: COMPANION_MAILBOX_SCHEMA_VERSION,
            item_id: item_id.clone(),
            owner_scope: self.owner_scope.clone(),
            item_type: MailboxItemType::LoveLetter,
            source_id: task_id.to_string(),
            source_revision: task.memory_revision.clone(),
            content_ref,
            state: MailboxState::Unread,
            created_at_millis: now_millis,
            available_at_millis: now_millis,
            updated_at_millis: now_millis,
            read_at_millis: None,
            archived_at_millis: None,
        };
        append_jsonl(&self.items_path(), &item, "mailbox item")?;

        task.state = LoveLetterTaskState::Ready;
        task.updated_at_millis = now_millis;
        task.generated_at_millis = Some(now_millis);
        task.generation_elapsed_millis = task
            .generation_started_at_millis
            .map(|started_at| now_millis.saturating_sub(started_at));
        task.lease_expires_at_millis = None;
        task.retry_after_millis = None;
        task.last_error_label = None;
        task.mailbox_item_id = Some(item_id);
        append_jsonl(&self.tasks_path(), &task, "love-letter task")?;
        Ok(item)
    }

    pub fn fail(
        &self,
        task_id: &str,
        error_label: &str,
        now_millis: u128,
        retry_backoff_millis: u128,
    ) -> AgentResult<LoveLetterTask> {
        let _lock = self.lock()?;
        let Some(mut task) = self
            .snapshot_unlocked()?
            .tasks
            .into_iter()
            .find(|task| task.task_id == task_id)
        else {
            return Err(mailbox_error("love-letter task was not found"));
        };
        task.state = LoveLetterTaskState::Failed;
        task.updated_at_millis = now_millis;
        task.generation_elapsed_millis = task
            .generation_started_at_millis
            .map(|started_at| now_millis.saturating_sub(started_at));
        task.lease_expires_at_millis = None;
        task.retry_after_millis = Some(now_millis.saturating_add(retry_backoff_millis));
        task.last_error_label = Some(sanitize_error_label(error_label));
        append_jsonl(&self.tasks_path(), &task, "love-letter task")?;
        Ok(task)
    }

    pub fn list(&self, query: MailboxQuery) -> AgentResult<MailboxPage> {
        let _lock = self.lock()?;
        let mut items = self.snapshot_unlocked()?.items;
        items.retain(|item| item.owner_scope == self.owner_scope);
        if let Some(state) = query.state {
            items.retain(|item| item.state == state);
        }
        if let Some(item_type) = query.item_type {
            items.retain(|item| item.item_type == item_type);
        }
        items.sort_by(|left, right| {
            right
                .available_at_millis
                .cmp(&left.available_at_millis)
                .then_with(|| right.item_id.cmp(&left.item_id))
        });
        if let Some(cursor) = query.cursor {
            items.retain(|item| {
                (item.available_at_millis, item.item_id.as_str())
                    < (cursor.available_at_millis, cursor.item_id.as_str())
            });
        }
        let limit = if query.limit == 0 {
            DEFAULT_PAGE_LIMIT
        } else {
            query.limit.min(MAX_PAGE_LIMIT)
        };
        let has_more = items.len() > limit;
        items.truncate(limit);
        let next_cursor = has_more.then(|| {
            let item = items.last().expect("truncated page contains an item");
            MailboxCursor {
                available_at_millis: item.available_at_millis,
                item_id: item.item_id.clone(),
            }
        });
        Ok(MailboxPage { items, next_cursor })
    }

    pub fn get(&self, item_id: &str) -> AgentResult<Option<CompanionMailboxEntry>> {
        let _lock = self.lock()?;
        let Some(item) = self
            .snapshot_unlocked()?
            .items
            .into_iter()
            .find(|item| item.item_id == item_id && item.owner_scope == self.owner_scope)
        else {
            return Ok(None);
        };
        let encrypted = read_json::<EncryptedMailboxContent>(
            &self.content_path(&item.content_ref),
            "encrypted mailbox content",
        )?;
        let plaintext = decrypt_content(
            &self.data_key,
            &encrypted,
            &content_aad(&self.owner_scope, &item.source_id, &item.content_ref),
        )?;
        let content = serde_json::from_slice::<CompanionMailboxContent>(&plaintext)
            .map_err(|_| mailbox_error("decrypted mailbox content is invalid"))?;
        Ok(Some(CompanionMailboxEntry { item, content }))
    }

    pub fn mark_state(
        &self,
        item_id: &str,
        state: MailboxState,
        now_millis: u128,
    ) -> AgentResult<Option<CompanionMailboxItem>> {
        let _lock = self.lock()?;
        let Some(item) = self
            .snapshot_unlocked()?
            .items
            .into_iter()
            .find(|item| item.item_id == item_id && item.owner_scope == self.owner_scope)
        else {
            return Ok(None);
        };
        let updated = item.mark_state(state, now_millis);
        append_jsonl(&self.items_path(), &updated, "mailbox item")?;
        Ok(Some(updated))
    }

    pub fn unread_count(&self) -> AgentResult<usize> {
        let _lock = self.lock()?;
        Ok(self
            .snapshot_unlocked()?
            .items
            .into_iter()
            .filter(|item| item.owner_scope == self.owner_scope)
            .filter(|item| item.state == MailboxState::Unread)
            .count())
    }

    fn snapshot_unlocked(&self) -> AgentResult<CompanionMailboxSnapshot> {
        Ok(CompanionMailboxSnapshot {
            tasks: latest_by_key(
                read_jsonl(&self.tasks_path(), "love-letter task")?,
                |task: &LoveLetterTask| task.task_id.clone(),
            ),
            items: latest_by_key(
                read_jsonl(&self.items_path(), "mailbox item")?,
                |item: &CompanionMailboxItem| item.item_id.clone(),
            ),
        })
    }

    fn write_content_once(
        &self,
        content_ref: &str,
        encrypted: &EncryptedMailboxContent,
        plaintext: &[u8],
        aad: &[u8],
    ) -> AgentResult<()> {
        let path = self.content_path(content_ref);
        if path.is_file() {
            let existing =
                read_json::<EncryptedMailboxContent>(&path, "encrypted mailbox content")?;
            let existing_plaintext = decrypt_content(&self.data_key, &existing, aad)?;
            if existing_plaintext == plaintext {
                return Ok(());
            }
            return Err(mailbox_error("mailbox content reference collision"));
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                mailbox_error(format!(
                    "failed to create mailbox content directory: {error}"
                ))
            })?;
        }
        let bytes = serde_json::to_vec(encrypted).map_err(|error| {
            mailbox_error(format!(
                "failed to serialize encrypted mailbox content: {error}"
            ))
        })?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| mailbox_error(format!("failed to create mailbox content: {error}")))?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|error| mailbox_error(format!("failed to persist mailbox content: {error}")))
    }

    fn lock(&self) -> AgentResult<MailboxFileLock> {
        std::fs::create_dir_all(&self.root).map_err(|error| {
            mailbox_error(format!("failed to create mailbox directory: {error}"))
        })?;
        acquire_file_lock(&self.root.join("mailbox.lock"), "mailbox")
    }

    fn tasks_path(&self) -> PathBuf {
        self.root.join("tasks.jsonl")
    }

    fn items_path(&self) -> PathBuf {
        self.root.join("items.jsonl")
    }

    fn content_path(&self, content_ref: &str) -> PathBuf {
        self.root
            .join("content")
            .join(format!("{content_ref}.json"))
    }
}

struct MailboxFileLock {
    path: PathBuf,
}

impl Drop for MailboxFileLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct EncryptedMailboxContent {
    algorithm: String,
    algorithm_version: u32,
    aad_version: u32,
    nonce: String,
    ciphertext: String,
}

fn encrypt_content(
    key: &[u8; 32],
    plaintext: &[u8],
    aad: &[u8],
) -> AgentResult<EncryptedMailboxContent> {
    let cipher = ChaCha20Poly1305::new_from_slice(key)
        .map_err(|_| mailbox_error("mailbox encryption key is invalid"))?;
    let mut nonce = [0_u8; NONCE_LENGTH];
    getrandom::fill(&mut nonce).map_err(|_| mailbox_error("mailbox nonce generation failed"))?;
    let ciphertext = cipher
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| mailbox_error("mailbox content encryption failed"))?;
    Ok(EncryptedMailboxContent {
        algorithm: MAILBOX_CONTENT_ALGORITHM.to_string(),
        algorithm_version: MAILBOX_CONTENT_ALGORITHM_VERSION,
        aad_version: MAILBOX_CONTENT_AAD_VERSION,
        nonce: STANDARD.encode(nonce),
        ciphertext: STANDARD.encode(ciphertext),
    })
}

fn decrypt_content(
    key: &[u8; 32],
    encrypted: &EncryptedMailboxContent,
    aad: &[u8],
) -> AgentResult<Vec<u8>> {
    if encrypted.algorithm != MAILBOX_CONTENT_ALGORITHM
        || encrypted.algorithm_version != MAILBOX_CONTENT_ALGORITHM_VERSION
        || encrypted.aad_version != MAILBOX_CONTENT_AAD_VERSION
    {
        return Err(mailbox_error(
            "encrypted mailbox content version is unsupported",
        ));
    }
    let nonce = STANDARD
        .decode(encrypted.nonce.as_bytes())
        .map_err(|_| mailbox_error("mailbox content nonce is invalid"))?;
    if nonce.len() != NONCE_LENGTH {
        return Err(mailbox_error("mailbox content nonce length is invalid"));
    }
    let ciphertext = STANDARD
        .decode(encrypted.ciphertext.as_bytes())
        .map_err(|_| mailbox_error("mailbox content ciphertext is invalid"))?;
    let cipher = ChaCha20Poly1305::new_from_slice(key)
        .map_err(|_| mailbox_error("mailbox encryption key is invalid"))?;
    cipher
        .decrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &ciphertext,
                aad,
            },
        )
        .map_err(|_| mailbox_error("mailbox content decryption failed"))
}

fn content_aad(owner_scope: &str, task_id: &str, content_ref: &str) -> Vec<u8> {
    format!(
        "yunxi-companion-mailbox\naad_version={MAILBOX_CONTENT_AAD_VERSION}\nowner_scope={owner_scope}\ntask_id={task_id}\ncontent_ref={content_ref}\n"
    )
    .into_bytes()
}

fn append_jsonl<T: Serialize>(path: &Path, value: &T, label: &str) -> AgentResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            mailbox_error(format!("failed to create {label} directory: {error}"))
        })?;
    }
    let mut line = serde_json::to_vec(value)
        .map_err(|error| mailbox_error(format!("failed to serialize {label}: {error}")))?;
    line.push(b'\n');
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| mailbox_error(format!("failed to open {label}: {error}")))?;
    file.write_all(&line)
        .and_then(|_| file.sync_all())
        .map_err(|error| mailbox_error(format!("failed to append {label}: {error}")))
}

fn read_jsonl<T: DeserializeOwned>(path: &Path, label: &str) -> AgentResult<Vec<T>> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(mailbox_error(format!("failed to read {label}: {error}")));
        }
    };
    let lines = content.lines().collect::<Vec<_>>();
    let mut records = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str(line) {
            Ok(record) => records.push(record),
            Err(_) if index + 1 == lines.len() => break,
            Err(error) => {
                return Err(mailbox_error(format!(
                    "failed to parse {label} record {}: {error}",
                    index + 1
                )));
            }
        }
    }
    Ok(records)
}

fn read_json<T: DeserializeOwned>(path: &Path, label: &str) -> AgentResult<T> {
    let bytes = std::fs::read(path)
        .map_err(|error| mailbox_error(format!("failed to read {label}: {error}")))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| mailbox_error(format!("failed to parse {label}: {error}")))
}

fn latest_by_key<T>(records: Vec<T>, key: impl Fn(&T) -> String) -> Vec<T> {
    let mut latest = BTreeMap::new();
    for record in records {
        latest.insert(key(&record), record);
    }
    latest.into_values().collect()
}

fn validate_content(content: &CompanionMailboxContent) -> AgentResult<()> {
    if content.subject.trim().is_empty() || content.body.trim().is_empty() {
        return Err(mailbox_error("mailbox content cannot be empty"));
    }
    Ok(())
}

fn sanitize_error_label(label: &str) -> String {
    label
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
        .take(64)
        .collect::<String>()
}

fn stable_path_hash(path: &Path) -> u64 {
    let normalized = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_ascii_lowercase();
    stable_hash64(normalized.as_bytes())
}

fn lock_is_stale(path: &Path) -> bool {
    let Ok(content) = std::fs::read_to_string(path) else {
        return false;
    };
    let Some(created_at) = content
        .lines()
        .nth(1)
        .and_then(|value| value.parse::<u128>().ok())
    else {
        return false;
    };
    now_millis().saturating_sub(created_at) >= STALE_LOCK_MILLIS
}

fn acquire_file_lock(path: &Path, label: &str) -> AgentResult<MailboxFileLock> {
    for _ in 0..LOCK_RETRY_COUNT {
        match OpenOptions::new().write(true).create_new(true).open(path) {
            Ok(mut file) => {
                let marker = format!("{}\n{}\n", std::process::id(), now_millis());
                file.write_all(marker.as_bytes()).map_err(|error| {
                    mailbox_error(format!("failed to initialize {label} lock: {error}"))
                })?;
                return Ok(MailboxFileLock {
                    path: path.to_path_buf(),
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if lock_is_stale(path) {
                    match std::fs::remove_file(path) {
                        Ok(()) => continue,
                        Err(remove_error)
                            if remove_error.kind() == std::io::ErrorKind::NotFound =>
                        {
                            continue;
                        }
                        Err(_) => {}
                    }
                }
                thread::sleep(LOCK_RETRY_DELAY);
            }
            Err(error) => {
                return Err(mailbox_error(format!(
                    "failed to acquire {label} lock: {error}"
                )));
            }
        }
    }
    Err(mailbox_error(format!("{label} lock is busy")))
}

fn now_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn mailbox_error(message: impl Into<String>) -> AgentError {
    AgentError::Execution {
        message: format!("companion mailbox: {}", message.into()),
    }
}

fn load_or_create_data_key(owner_scope: &str) -> AgentResult<[u8; 32]> {
    if let Ok(value) = std::env::var("YUNXI_MAILBOX_KEY_HEX") {
        return decode_hex_key(value.trim());
    }
    system_key_store::load_or_create(owner_scope)
}

fn decode_hex_key(value: &str) -> AgentResult<[u8; 32]> {
    if value.len() != 64 {
        return Err(mailbox_error(
            "mailbox encryption key must contain 64 hex characters",
        ));
    }
    let mut key = [0_u8; 32];
    for (index, slot) in key.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| mailbox_error("mailbox encryption key contains invalid hex"))?;
    }
    Ok(key)
}

#[cfg(windows)]
mod system_key_store {
    use std::{ffi::c_void, ptr, slice};

    use windows_sys::Win32::Foundation::{ERROR_NOT_FOUND, GetLastError};
    use windows_sys::Win32::Security::Credentials::{
        CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW, CredFree, CredReadW, CredWriteW,
    };

    use super::{AgentResult, mailbox_error};

    pub fn load_or_create(owner_scope: &str) -> AgentResult<[u8; 32]> {
        let target = format!("YunXiAgent/CompanionMailbox/{owner_scope}/data-key");
        if let Some(key) = read(&target)? {
            return Ok(key);
        }
        let mut key = [0_u8; 32];
        getrandom::fill(&mut key)
            .map_err(|_| mailbox_error("mailbox encryption key generation failed"))?;
        write(&target, &key)?;
        Ok(key)
    }

    fn read(target: &str) -> AgentResult<Option<[u8; 32]>> {
        let target = wide(target);
        let mut credential = ptr::null_mut();
        let result = unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) };
        if result == 0 {
            if unsafe { GetLastError() } == ERROR_NOT_FOUND {
                return Ok(None);
            }
            return Err(mailbox_error(
                "failed to read mailbox key from system credential store",
            ));
        }
        let bytes = unsafe {
            slice::from_raw_parts(
                (*credential).CredentialBlob,
                (*credential).CredentialBlobSize as usize,
            )
            .to_vec()
        };
        unsafe { CredFree(credential.cast::<c_void>()) };
        let key: [u8; 32] = bytes
            .try_into()
            .map_err(|_| mailbox_error("system mailbox key has an invalid length"))?;
        Ok(Some(key))
    }

    fn write(target: &str, key: &[u8; 32]) -> AgentResult<()> {
        let mut target = wide(target);
        let mut value = key.to_vec();
        let credential = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: target.as_mut_ptr(),
            CredentialBlobSize: value.len() as u32,
            CredentialBlob: value.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            ..CREDENTIALW::default()
        };
        if unsafe { CredWriteW(&credential, 0) } == 0 {
            return Err(mailbox_error(
                "failed to save mailbox key in system credential store",
            ));
        }
        Ok(())
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

#[cfg(not(windows))]
mod system_key_store {
    use super::{AgentResult, mailbox_error};

    pub fn load_or_create(_owner_scope: &str) -> AgentResult<[u8; 32]> {
        Err(mailbox_error(
            "system credential store is unavailable; set YUNXI_MAILBOX_KEY_HEX",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use yunxi_agent_companion::{LoveLetterMemorySelection, LoveLetterTask};

    fn store(temp: &TempDir) -> FileCompanionMailboxStore {
        FileCompanionMailboxStore::with_data_key(
            temp.path().join("mailbox"),
            "workspace:test",
            [7_u8; 32],
        )
    }

    fn task(now: u128) -> LoveLetterTask {
        LoveLetterTask::pending(
            "idempotency-1",
            "workspace:test",
            "profile",
            "profile:hash",
            &LoveLetterMemorySelection {
                references: Vec::new(),
                contents: Vec::new(),
                memory_revision: "memory:1".to_string(),
                earliest_updated_at_millis: None,
                latest_updated_at_millis: None,
            },
            now,
        )
    }

    #[test]
    fn enqueue_is_idempotent_and_claim_uses_a_lease() {
        let temp = TempDir::new().expect("tempdir");
        let store = store(&temp);
        let task = task(10);
        assert!(matches!(
            store.enqueue(&task).expect("enqueue"),
            MailboxEnqueueOutcome::Enqueued(_)
        ));
        assert!(matches!(
            store.enqueue(&task).expect("deduplicate"),
            MailboxEnqueueOutcome::Duplicate(_)
        ));
        let claimed = store.claim_next(20, 100, 3).expect("claim").expect("task");
        assert_eq!(claimed.state, LoveLetterTaskState::Generating);
        assert_eq!(claimed.generation_attempts, 1);
        assert!(
            store
                .claim_next(21, 100, 3)
                .expect("no second claim")
                .is_none()
        );
        assert!(store.claim_next(121, 100, 3).expect("reclaim").is_some());
    }

    #[test]
    fn completed_letter_is_encrypted_and_mailbox_state_is_mutable() {
        let temp = TempDir::new().expect("tempdir");
        let store = store(&temp);
        let task = task(10);
        store.enqueue(&task).expect("enqueue");
        let claimed = store.claim_next(20, 100, 3).expect("claim").expect("task");
        let content = CompanionMailboxContent {
            subject: "写给你".to_string(),
            body: "这是一段不会明文落盘的私人内容。".to_string(),
        };
        let item = store
            .complete(&claimed.task_id, &content, 30)
            .expect("complete");
        let raw = std::fs::read_to_string(store.content_path(&item.content_ref))
            .expect("encrypted content file");
        assert!(!raw.contains(&content.subject));
        assert!(!raw.contains(&content.body));

        let entry = store.get(&item.item_id).expect("get").expect("entry");
        assert_eq!(entry.content, content);
        assert_eq!(store.unread_count().expect("unread"), 1);
        let read = store
            .mark_state(&item.item_id, MailboxState::Read, 40)
            .expect("mark read")
            .expect("item");
        assert_eq!(read.state, MailboxState::Read);
        assert_eq!(store.unread_count().expect("unread after read"), 0);
    }

    #[test]
    fn failed_task_respects_retry_backoff() {
        let temp = TempDir::new().expect("tempdir");
        let store = store(&temp);
        let task = task(10);
        store.enqueue(&task).expect("enqueue");
        let claimed = store.claim_next(20, 100, 3).expect("claim").expect("task");
        store
            .fail(&claimed.task_id, "provider timeout: private text", 30, 50)
            .expect("fail");
        assert!(store.claim_next(79, 100, 3).expect("backoff").is_none());
        let retry = store.claim_next(80, 100, 3).expect("retry").expect("task");
        assert_eq!(retry.generation_attempts, 2);
        assert_eq!(retry.last_error_label, None);
    }

    #[test]
    fn claim_reconciles_an_existing_mailbox_item_without_regeneration() {
        let temp = TempDir::new().expect("tempdir");
        let store = store(&temp);
        let task = task(10);
        store.enqueue(&task).expect("enqueue");
        let claimed = store.claim_next(20, 100, 3).expect("claim").expect("task");
        store
            .complete(
                &claimed.task_id,
                &CompanionMailboxContent {
                    subject: "subject".to_string(),
                    body: "body".to_string(),
                },
                30,
            )
            .expect("complete");

        let mut interrupted = claimed;
        interrupted.updated_at_millis = 31;
        append_jsonl(&store.tasks_path(), &interrupted, "love-letter task")
            .expect("simulate interrupted task revision");
        assert!(store.claim_next(200, 100, 3).expect("reconcile").is_none());
        assert_eq!(
            store.snapshot().expect("snapshot").tasks[0].state,
            LoveLetterTaskState::Ready
        );
    }
}

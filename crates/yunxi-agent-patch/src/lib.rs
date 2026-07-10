use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use yunxi_agent_core::{AgentError, AgentResult};

pub const APPLY_PATCH_LARK_GRAMMAR: &str = include_str!("../assets/apply_patch.lark");

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PatchReport {
    pub changed_files: Vec<PatchFileChange>,
    #[serde(default)]
    pub diagnostics: Vec<PatchDiagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PatchFileChange {
    pub path: PathBuf,
    pub kind: PatchFileChangeKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchFileChangeKind {
    Added,
    Updated,
    Deleted,
    Moved,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchDiagnosticKind {
    InvalidEnvelope,
    InvalidJson,
    InvalidPath,
    DuplicatePath,
    MissingTarget,
    TargetExists,
    NonUtf8,
    ContextMismatch,
    UnsupportedOperation,
    Io,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PatchDiagnostic {
    pub kind: PatchDiagnosticKind,
    pub message: String,
    pub path: Option<PathBuf>,
    pub line: Option<usize>,
}

impl PatchDiagnostic {
    pub fn new(kind: PatchDiagnosticKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            path: None,
            line: None,
        }
    }

    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PatchApplyError {
    pub diagnostics: Vec<PatchDiagnostic>,
}

impl PatchApplyError {
    pub fn single(diagnostic: PatchDiagnostic) -> Self {
        Self {
            diagnostics: vec![diagnostic],
        }
    }

    pub fn primary_message(&self) -> String {
        self.diagnostics
            .first()
            .map(|diagnostic| diagnostic.message.clone())
            .unwrap_or_else(|| "patch failed".to_string())
    }

    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        if let Some(diagnostic) = self.diagnostics.first_mut() {
            diagnostic.path = Some(path.into());
        }
        self
    }

    pub fn with_line(mut self, line: usize) -> Self {
        if let Some(diagnostic) = self.diagnostics.first_mut() {
            diagnostic.line = Some(line);
        }
        self
    }
}

impl std::fmt::Display for PatchApplyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.primary_message())
    }
}

impl std::error::Error for PatchApplyError {}

impl From<PatchApplyError> for AgentError {
    fn from(error: PatchApplyError) -> Self {
        AgentError::Execution {
            message: error.primary_message(),
        }
    }
}

pub type PatchApplyResult<T> = Result<T, PatchApplyError>;

fn patch_error(kind: PatchDiagnosticKind, message: impl Into<String>) -> PatchApplyError {
    PatchApplyError::single(PatchDiagnostic::new(kind, message))
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum PatchOperation {
    Write { path: PathBuf, content: String },
    Delete { path: PathBuf },
    Move { from: PathBuf, path: PathBuf },
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ParsedPatchOperation {
    Add {
        path: PathBuf,
        content: String,
    },
    Write {
        path: PathBuf,
        content: String,
    },
    Delete {
        path: PathBuf,
    },
    Move {
        from: PathBuf,
        to: PathBuf,
    },
    Update {
        path: PathBuf,
        move_to: Option<PathBuf>,
        chunks: Vec<PatchChunk>,
    },
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct PatchChunk {
    old: String,
    new: String,
}

impl PatchChunk {
    fn is_empty(&self) -> bool {
        self.old.is_empty() && self.new.is_empty()
    }
}

pub fn apply_patch(cwd: impl AsRef<Path>, patch: &str) -> AgentResult<PatchReport> {
    apply_patch_detailed(cwd, patch).map_err(Into::into)
}

pub fn apply_patch_detailed(cwd: impl AsRef<Path>, patch: &str) -> PatchApplyResult<PatchReport> {
    let cwd = cwd.as_ref();
    let operations = parse_patch(patch)?;
    validate_operation_paths(&operations)?;
    let mut changed_files = Vec::new();

    for operation in operations {
        match operation {
            ParsedPatchOperation::Add { path, content } => {
                let relative = validate_relative_path(&path)?;
                let full_path = cwd.join(&relative);
                if full_path.exists() {
                    return Err(patch_error(
                        PatchDiagnosticKind::TargetExists,
                        format!("patch add target already exists: {}", relative.display()),
                    )
                    .with_path(relative));
                }
                if let Some(parent) = full_path.parent() {
                    std::fs::create_dir_all(parent).map_err(|error| {
                        patch_error(
                            PatchDiagnosticKind::Io,
                            format!("failed to create patch parent directory: {error}"),
                        )
                    })?;
                }
                std::fs::write(&full_path, content).map_err(|error| {
                    patch_error(
                        PatchDiagnosticKind::Io,
                        format!("failed to add patch file {}: {error}", full_path.display()),
                    )
                })?;
                changed_files.push(PatchFileChange {
                    path: relative,
                    kind: PatchFileChangeKind::Added,
                });
            }
            ParsedPatchOperation::Write { path, content } => {
                let relative = validate_relative_path(&path)?;
                let full_path = cwd.join(&relative);
                let kind = if full_path.is_file() {
                    PatchFileChangeKind::Updated
                } else {
                    PatchFileChangeKind::Added
                };
                if let Some(parent) = full_path.parent() {
                    std::fs::create_dir_all(parent).map_err(|error| {
                        patch_error(
                            PatchDiagnosticKind::Io,
                            format!("failed to create patch parent directory: {error}"),
                        )
                    })?;
                }
                std::fs::write(&full_path, content).map_err(|error| {
                    patch_error(
                        PatchDiagnosticKind::Io,
                        format!(
                            "failed to write patch file {}: {error}",
                            full_path.display()
                        ),
                    )
                })?;
                changed_files.push(PatchFileChange {
                    path: relative,
                    kind,
                });
            }
            ParsedPatchOperation::Delete { path } => {
                let relative = validate_relative_path(&path)?;
                let full_path = cwd.join(&relative);
                if !full_path.is_file() {
                    return Err(patch_error(
                        PatchDiagnosticKind::MissingTarget,
                        format!("patch delete target does not exist: {}", relative.display()),
                    )
                    .with_path(relative));
                }
                std::fs::remove_file(&full_path).map_err(|error| {
                    patch_error(
                        PatchDiagnosticKind::Io,
                        format!(
                            "failed to delete patch file {}: {error}",
                            full_path.display()
                        ),
                    )
                })?;
                changed_files.push(PatchFileChange {
                    path: relative,
                    kind: PatchFileChangeKind::Deleted,
                });
            }
            ParsedPatchOperation::Move { from, to } => {
                let from = validate_relative_path(&from)?;
                let to = validate_relative_path(&to)?;
                move_file(cwd, &from, &to)?;
                changed_files.push(PatchFileChange {
                    path: from,
                    kind: PatchFileChangeKind::Deleted,
                });
                changed_files.push(PatchFileChange {
                    path: to,
                    kind: PatchFileChangeKind::Moved,
                });
            }
            ParsedPatchOperation::Update {
                path,
                move_to,
                chunks,
            } => {
                let relative = validate_relative_path(&path)?;
                let full_path = cwd.join(&relative);
                let content = read_patch_file(&full_path)?;
                let updated = apply_update_chunks(&relative, content, &chunks)?;
                let output_relative = match move_to {
                    Some(path) => validate_relative_path(&path)?,
                    None => relative.clone(),
                };
                let output_path = cwd.join(&output_relative);
                if let Some(parent) = output_path.parent() {
                    std::fs::create_dir_all(parent).map_err(|error| {
                        patch_error(
                            PatchDiagnosticKind::Io,
                            format!("failed to create patch parent directory: {error}"),
                        )
                    })?;
                }
                std::fs::write(&output_path, updated).map_err(|error| {
                    patch_error(
                        PatchDiagnosticKind::Io,
                        format!(
                            "failed to update patch file {}: {error}",
                            output_path.display()
                        ),
                    )
                })?;
                if output_relative != relative {
                    std::fs::remove_file(&full_path).map_err(|error| {
                        patch_error(
                            PatchDiagnosticKind::Io,
                            format!(
                                "failed to remove moved patch source {}: {error}",
                                full_path.display()
                            ),
                        )
                    })?;
                    changed_files.push(PatchFileChange {
                        path: relative,
                        kind: PatchFileChangeKind::Deleted,
                    });
                    changed_files.push(PatchFileChange {
                        path: output_relative,
                        kind: PatchFileChangeKind::Moved,
                    });
                    continue;
                }
                changed_files.push(PatchFileChange {
                    path: relative,
                    kind: PatchFileChangeKind::Updated,
                });
            }
        }
    }

    Ok(PatchReport {
        changed_files,
        diagnostics: Vec::new(),
    })
}

fn parse_patch(patch: &str) -> PatchApplyResult<Vec<ParsedPatchOperation>> {
    let trimmed = patch.trim();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        return parse_json_patch(trimmed);
    }
    parse_apply_patch(trimmed)
}

fn parse_json_patch(patch: &str) -> PatchApplyResult<Vec<ParsedPatchOperation>> {
    let value = serde_json::from_str::<Value>(patch).map_err(|error| {
        patch_error(
            PatchDiagnosticKind::InvalidJson,
            format!("failed to parse constrained patch JSON: {error}"),
        )
    })?;
    let operations = if value.is_array() {
        serde_json::from_value::<Vec<PatchOperation>>(value)
    } else {
        serde_json::from_value::<PatchOperation>(value).map(|operation| vec![operation])
    }
    .map_err(|error| {
        patch_error(
            PatchDiagnosticKind::InvalidJson,
            format!("failed to parse constrained patch operation: {error}"),
        )
    })?;

    Ok(operations
        .into_iter()
        .map(|operation| match operation {
            PatchOperation::Write { path, content } => {
                ParsedPatchOperation::Write { path, content }
            }
            PatchOperation::Delete { path } => ParsedPatchOperation::Delete { path },
            PatchOperation::Move { from, path } => ParsedPatchOperation::Move { from, to: path },
        })
        .collect())
}

fn parse_apply_patch(patch: &str) -> PatchApplyResult<Vec<ParsedPatchOperation>> {
    let mut lines = patch.lines().peekable();
    if !matches_marker(lines.next(), "*** Begin Patch") {
        return Err(patch_error(
            PatchDiagnosticKind::InvalidEnvelope,
            "apply_patch input must start with *** Begin Patch",
        ));
    }

    let mut operations = Vec::new();
    while let Some(line) = lines.next() {
        let marker = line.trim();
        if marker == "*** End Patch" {
            if operations.is_empty() {
                return Err(patch_error(
                    PatchDiagnosticKind::InvalidEnvelope,
                    "apply_patch input did not contain any operations",
                ));
            }
            return Ok(operations);
        }
        if let Some(path) = marker.strip_prefix("*** Add File: ") {
            let mut content = String::new();
            while let Some(next) = lines.peek().copied() {
                if is_patch_operation_marker(next) {
                    break;
                }
                let next = lines.next().unwrap_or_default();
                content.push_str(next.strip_prefix('+').unwrap_or(next));
                content.push('\n');
            }
            operations.push(ParsedPatchOperation::Add {
                path: PathBuf::from(path),
                content,
            });
            continue;
        }
        if let Some(path) = marker.strip_prefix("*** Delete File: ") {
            operations.push(ParsedPatchOperation::Delete {
                path: PathBuf::from(path),
            });
            continue;
        }
        if let Some(path) = marker.strip_prefix("*** Update File: ") {
            let mut move_to = None;
            let mut chunks = Vec::new();
            let mut current = PatchChunk::default();
            while let Some(next) = lines.peek().copied() {
                if is_patch_operation_marker(next) {
                    break;
                }
                let next = lines.next().unwrap_or_default();
                let trimmed_next = next.trim();
                if let Some(destination) = trimmed_next.strip_prefix("*** Move to: ") {
                    move_to = Some(PathBuf::from(destination));
                    continue;
                }
                if trimmed_next.starts_with("@@") {
                    if !current.is_empty() {
                        chunks.push(current);
                        current = PatchChunk::default();
                    }
                    continue;
                }
                if trimmed_next == "*** End of File" {
                    continue;
                }
                if let Some(removed) = next.strip_prefix('-') {
                    current.old.push_str(removed);
                    current.old.push('\n');
                } else if let Some(added) = next.strip_prefix('+') {
                    current.new.push_str(added);
                    current.new.push('\n');
                } else if let Some(context) = next.strip_prefix(' ') {
                    current.old.push_str(context);
                    current.old.push('\n');
                    current.new.push_str(context);
                    current.new.push('\n');
                }
            }
            if !current.is_empty() {
                chunks.push(current);
            }
            if chunks.is_empty() && move_to.is_none() {
                return Err(patch_error(
                    PatchDiagnosticKind::UnsupportedOperation,
                    format!("patch update for {path} did not contain any hunks"),
                )
                .with_path(path));
            }
            let path = PathBuf::from(path);
            if chunks.is_empty() {
                let to = move_to.expect("move destination checked");
                operations.push(ParsedPatchOperation::Move { from: path, to });
            } else {
                operations.push(ParsedPatchOperation::Update {
                    path,
                    move_to,
                    chunks,
                });
            }
            continue;
        }
        return Err(patch_error(
            PatchDiagnosticKind::UnsupportedOperation,
            format!("unsupported apply_patch line: {line}"),
        ));
    }

    Err(patch_error(
        PatchDiagnosticKind::InvalidEnvelope,
        "apply_patch input is missing *** End Patch",
    ))
}

fn matches_marker(line: Option<&str>, expected: &str) -> bool {
    line.is_some_and(|line| line.trim() == expected)
}

fn is_patch_operation_marker(line: &str) -> bool {
    let line = line.trim();
    line == "*** End Patch"
        || line.starts_with("*** Add File: ")
        || line.starts_with("*** Delete File: ")
        || line.starts_with("*** Update File: ")
}

fn read_patch_file(path: &Path) -> PatchApplyResult<String> {
    std::fs::read_to_string(path).map_err(|error| {
        let kind = match error.kind() {
            std::io::ErrorKind::NotFound => PatchDiagnosticKind::MissingTarget,
            std::io::ErrorKind::InvalidData => PatchDiagnosticKind::NonUtf8,
            _ => PatchDiagnosticKind::Io,
        };
        patch_error(
            kind,
            format!("failed to read patch file {}: {error}", path.display()),
        )
        .with_path(path)
    })
}

fn apply_update_chunks(
    relative: &Path,
    mut content: String,
    chunks: &[PatchChunk],
) -> PatchApplyResult<String> {
    for chunk in chunks {
        if chunk.old.is_empty() {
            if !content.ends_with('\n') && !content.is_empty() {
                content.push('\n');
            }
            content.push_str(&chunk.new);
            continue;
        }
        if content.contains(&chunk.old) {
            content = content.replacen(&chunk.old, &chunk.new, 1);
        } else {
            return Err(patch_error(
                PatchDiagnosticKind::ContextMismatch,
                format!(
                    "patch update target content was not found in {}",
                    relative.display()
                ),
            )
            .with_path(relative));
        }
    }
    Ok(content)
}

fn move_file(cwd: &Path, from: &Path, to: &Path) -> PatchApplyResult<()> {
    let source = cwd.join(from);
    let destination = cwd.join(to);
    if !source.is_file() {
        return Err(patch_error(
            PatchDiagnosticKind::MissingTarget,
            format!("patch move source does not exist: {}", from.display()),
        )
        .with_path(from));
    }
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            patch_error(
                PatchDiagnosticKind::Io,
                format!("failed to create patch parent directory: {error}"),
            )
        })?;
    }
    if destination.exists() {
        return Err(patch_error(
            PatchDiagnosticKind::TargetExists,
            format!("patch move target already exists: {}", to.display()),
        )
        .with_path(to));
    }
    std::fs::rename(&source, &destination).map_err(|error| {
        patch_error(
            PatchDiagnosticKind::Io,
            format!(
                "failed to move patch file {} to {}: {error}",
                source.display(),
                destination.display()
            ),
        )
    })
}

fn validate_operation_paths(operations: &[ParsedPatchOperation]) -> PatchApplyResult<()> {
    let mut touched = BTreeSet::new();
    for operation in operations {
        for path in operation.touched_paths() {
            let relative = validate_relative_path(path)?;
            if !touched.insert(relative.clone()) {
                return Err(patch_error(
                    PatchDiagnosticKind::DuplicatePath,
                    format!(
                        "patch touches the same path more than once: {}",
                        relative.display()
                    ),
                )
                .with_path(relative));
            }
        }
    }
    Ok(())
}

impl ParsedPatchOperation {
    fn touched_paths(&self) -> Vec<&Path> {
        match self {
            Self::Add { path, .. } | Self::Write { path, .. } | Self::Delete { path } => {
                vec![path.as_path()]
            }
            Self::Move { from, to } => vec![from.as_path(), to.as_path()],
            Self::Update { path, move_to, .. } => {
                let mut paths = vec![path.as_path()];
                if let Some(move_to) = move_to {
                    paths.push(move_to.as_path());
                }
                paths
            }
        }
    }
}

fn validate_relative_path(path: &Path) -> PatchApplyResult<PathBuf> {
    if path.is_absolute() {
        return Err(patch_error(
            PatchDiagnosticKind::InvalidPath,
            format!("patch path must be relative: {}", path.display()),
        )
        .with_path(path));
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(patch_error(
            PatchDiagnosticKind::InvalidPath,
            format!("patch path cannot escape workspace: {}", path.display()),
        )
        .with_path(path));
    }
    Ok(path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn applies_codex_style_add_file_patch() {
        let temp = TempDir::new().expect("temp dir");
        let report = apply_patch(
            temp.path(),
            "*** Begin Patch\n*** Add File: notes.txt\n+hello\n*** End Patch",
        )
        .expect("patch");

        assert_eq!(
            std::fs::read_to_string(temp.path().join("notes.txt")).expect("notes"),
            "hello\n"
        );
        assert_eq!(report.changed_files[0].kind, PatchFileChangeKind::Added);
    }

    #[test]
    fn exposes_codex_apply_patch_grammar_asset() {
        assert!(APPLY_PATCH_LARK_GRAMMAR.contains("change_move"));
        assert!(APPLY_PATCH_LARK_GRAMMAR.contains("*** Move to: "));
    }

    #[test]
    fn applies_multiple_update_hunks() {
        let temp = TempDir::new().expect("temp dir");
        std::fs::write(
            temp.path().join("multi.txt"),
            "line1\nline2\nline3\nline4\n",
        )
        .expect("input");

        apply_patch(
            temp.path(),
            "*** Begin Patch\n*** Update File: multi.txt\n@@\n-line2\n+changed2\n@@\n-line4\n+changed4\n*** End Patch",
        )
        .expect("patch");

        assert_eq!(
            std::fs::read_to_string(temp.path().join("multi.txt")).expect("multi"),
            "line1\nchanged2\nline3\nchanged4\n"
        );
    }

    #[test]
    fn applies_update_with_move_destination() {
        let temp = TempDir::new().expect("temp dir");
        let source_dir = temp.path().join("old");
        std::fs::create_dir_all(&source_dir).expect("source dir");
        std::fs::write(source_dir.join("name.txt"), "old content\n").expect("input");

        let report = apply_patch(
            temp.path(),
            "*** Begin Patch\n*** Update File: old/name.txt\n*** Move to: renamed/dir/name.txt\n@@\n-old content\n+new content\n*** End Patch",
        )
        .expect("patch");

        assert!(!source_dir.join("name.txt").exists());
        assert_eq!(
            std::fs::read_to_string(temp.path().join("renamed/dir/name.txt")).expect("moved"),
            "new content\n"
        );
        assert!(
            report
                .changed_files
                .iter()
                .any(|change| change.kind == PatchFileChangeKind::Moved)
        );
    }

    #[test]
    fn detailed_patch_error_reports_invalid_path_diagnostic() {
        let temp = TempDir::new().expect("temp dir");
        let error = apply_patch_detailed(
            temp.path(),
            r#"{"op":"write","path":"../escape.txt","content":"no"}"#,
        )
        .expect_err("invalid path");

        assert_eq!(error.diagnostics[0].kind, PatchDiagnosticKind::InvalidPath);
        assert_eq!(
            error.diagnostics[0].path.as_deref(),
            Some(Path::new("../escape.txt"))
        );
    }

    #[test]
    fn add_file_rejects_existing_target() {
        let temp = TempDir::new().expect("temp dir");
        std::fs::write(temp.path().join("notes.txt"), "existing\n").expect("existing");

        let error = apply_patch_detailed(
            temp.path(),
            "*** Begin Patch\n*** Add File: notes.txt\n+new\n*** End Patch",
        )
        .expect_err("existing add target should fail");

        assert_eq!(error.diagnostics[0].kind, PatchDiagnosticKind::TargetExists);
    }

    #[test]
    fn patch_rejects_duplicate_touched_paths() {
        let temp = TempDir::new().expect("temp dir");

        let error = apply_patch_detailed(
            temp.path(),
            "*** Begin Patch\n*** Add File: notes.txt\n+one\n*** Add File: notes.txt\n+two\n*** End Patch",
        )
        .expect_err("duplicate touched path should fail");

        assert_eq!(
            error.diagnostics[0].kind,
            PatchDiagnosticKind::DuplicatePath
        );
    }

    #[test]
    fn update_rejects_non_utf8_target_with_diagnostic() {
        let temp = TempDir::new().expect("temp dir");
        std::fs::write(temp.path().join("binary.bin"), [0xff, 0xfe, 0xfd]).expect("binary");

        let error = apply_patch_detailed(
            temp.path(),
            "*** Begin Patch\n*** Update File: binary.bin\n@@\n-old\n+new\n*** End Patch",
        )
        .expect_err("non-utf8 update target should fail");

        assert_eq!(error.diagnostics[0].kind, PatchDiagnosticKind::NonUtf8);
    }
}

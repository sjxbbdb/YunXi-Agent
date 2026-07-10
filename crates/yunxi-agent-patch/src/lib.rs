use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Component, Path, PathBuf};
use yunxi_agent_core::{AgentError, AgentResult};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PatchReport {
    pub changed_files: Vec<PatchFileChange>,
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
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum PatchOperation {
    Write { path: PathBuf, content: String },
    Delete { path: PathBuf },
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ParsedPatchOperation {
    Write {
        path: PathBuf,
        content: String,
    },
    Delete {
        path: PathBuf,
    },
    Update {
        path: PathBuf,
        old: String,
        new: String,
    },
}

pub fn apply_patch(cwd: impl AsRef<Path>, patch: &str) -> AgentResult<PatchReport> {
    let cwd = cwd.as_ref();
    let operations = parse_patch(patch)?;
    let mut changed_files = Vec::new();

    for operation in operations {
        match operation {
            ParsedPatchOperation::Write { path, content } => {
                let relative = validate_relative_path(&path)?;
                let full_path = cwd.join(&relative);
                let kind = if full_path.is_file() {
                    PatchFileChangeKind::Updated
                } else {
                    PatchFileChangeKind::Added
                };
                if let Some(parent) = full_path.parent() {
                    std::fs::create_dir_all(parent).map_err(|error| AgentError::Execution {
                        message: format!("failed to create patch parent directory: {error}"),
                    })?;
                }
                std::fs::write(&full_path, content).map_err(|error| AgentError::Execution {
                    message: format!(
                        "failed to write patch file {}: {error}",
                        full_path.display()
                    ),
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
                    return Err(AgentError::Execution {
                        message: format!(
                            "patch delete target does not exist: {}",
                            relative.display()
                        ),
                    });
                }
                std::fs::remove_file(&full_path).map_err(|error| AgentError::Execution {
                    message: format!(
                        "failed to delete patch file {}: {error}",
                        full_path.display()
                    ),
                })?;
                changed_files.push(PatchFileChange {
                    path: relative,
                    kind: PatchFileChangeKind::Deleted,
                });
            }
            ParsedPatchOperation::Update { path, old, new } => {
                let relative = validate_relative_path(&path)?;
                let full_path = cwd.join(&relative);
                let content =
                    std::fs::read_to_string(&full_path).map_err(|error| AgentError::Execution {
                        message: format!(
                            "failed to read patch file {}: {error}",
                            full_path.display()
                        ),
                    })?;
                let updated = if old.is_empty() {
                    format!("{content}{new}")
                } else if content.contains(&old) {
                    content.replacen(&old, &new, 1)
                } else {
                    return Err(AgentError::Execution {
                        message: format!(
                            "patch update target content was not found in {}",
                            relative.display()
                        ),
                    });
                };
                std::fs::write(&full_path, updated).map_err(|error| AgentError::Execution {
                    message: format!(
                        "failed to update patch file {}: {error}",
                        full_path.display()
                    ),
                })?;
                changed_files.push(PatchFileChange {
                    path: relative,
                    kind: PatchFileChangeKind::Updated,
                });
            }
        }
    }

    Ok(PatchReport { changed_files })
}

fn parse_patch(patch: &str) -> AgentResult<Vec<ParsedPatchOperation>> {
    let trimmed = patch.trim();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        return parse_json_patch(trimmed);
    }
    parse_apply_patch(trimmed)
}

fn parse_json_patch(patch: &str) -> AgentResult<Vec<ParsedPatchOperation>> {
    let value = serde_json::from_str::<Value>(patch).map_err(|error| AgentError::Execution {
        message: format!("failed to parse constrained patch JSON: {error}"),
    })?;
    let operations = if value.is_array() {
        serde_json::from_value::<Vec<PatchOperation>>(value)
    } else {
        serde_json::from_value::<PatchOperation>(value).map(|operation| vec![operation])
    }
    .map_err(|error| AgentError::Execution {
        message: format!("failed to parse constrained patch operation: {error}"),
    })?;

    Ok(operations
        .into_iter()
        .map(|operation| match operation {
            PatchOperation::Write { path, content } => {
                ParsedPatchOperation::Write { path, content }
            }
            PatchOperation::Delete { path } => ParsedPatchOperation::Delete { path },
        })
        .collect())
}

fn parse_apply_patch(patch: &str) -> AgentResult<Vec<ParsedPatchOperation>> {
    let mut lines = patch.lines().peekable();
    if lines.next() != Some("*** Begin Patch") {
        return Err(AgentError::Execution {
            message: "apply_patch input must start with *** Begin Patch".to_string(),
        });
    }

    let mut operations = Vec::new();
    while let Some(line) = lines.next() {
        if line == "*** End Patch" {
            return Ok(operations);
        }
        if let Some(path) = line.strip_prefix("*** Add File: ") {
            let mut content = String::new();
            while let Some(next) = lines.peek().copied() {
                if next.starts_with("*** ") {
                    break;
                }
                let next = lines.next().unwrap_or_default();
                content.push_str(next.strip_prefix('+').unwrap_or(next));
                content.push('\n');
            }
            operations.push(ParsedPatchOperation::Write {
                path: PathBuf::from(path),
                content,
            });
            continue;
        }
        if let Some(path) = line.strip_prefix("*** Delete File: ") {
            operations.push(ParsedPatchOperation::Delete {
                path: PathBuf::from(path),
            });
            continue;
        }
        if let Some(path) = line.strip_prefix("*** Update File: ") {
            let mut old = String::new();
            let mut new = String::new();
            while let Some(next) = lines.peek().copied() {
                if next.starts_with("*** ") {
                    break;
                }
                let next = lines.next().unwrap_or_default();
                if next.starts_with("@@") || next == "*** End of File" {
                    continue;
                }
                if let Some(removed) = next.strip_prefix('-') {
                    old.push_str(removed);
                    old.push('\n');
                } else if let Some(added) = next.strip_prefix('+') {
                    new.push_str(added);
                    new.push('\n');
                }
            }
            operations.push(ParsedPatchOperation::Update {
                path: PathBuf::from(path),
                old,
                new,
            });
            continue;
        }
        return Err(AgentError::Execution {
            message: format!("unsupported apply_patch line: {line}"),
        });
    }

    Err(AgentError::Execution {
        message: "apply_patch input is missing *** End Patch".to_string(),
    })
}

fn validate_relative_path(path: &Path) -> AgentResult<PathBuf> {
    if path.is_absolute() {
        return Err(AgentError::Execution {
            message: format!("patch path must be relative: {}", path.display()),
        });
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(AgentError::Execution {
            message: format!("patch path cannot escape workspace: {}", path.display()),
        });
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
}

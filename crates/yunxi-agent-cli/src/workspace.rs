use anyhow::{Context, Result, anyhow};
use std::io;
use std::path::{Path, PathBuf};

pub(crate) fn resolve_cli_cwd(explicit_cwd: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(cwd) = explicit_cwd {
        return Ok(cwd);
    }

    let current = std::env::current_dir().context("failed to resolve current working directory")?;
    resolve_implicit_cwd_with(current, user_profile_dir(), prepare_session_directory)
}

fn user_profile_dir() -> Option<PathBuf> {
    ["USERPROFILE", "HOME"].into_iter().find_map(|name| {
        std::env::var_os(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    })
}

fn prepare_session_directory(workspace: &Path) -> io::Result<()> {
    std::fs::create_dir_all(workspace.join(".yunxi").join("sessions"))
}

fn resolve_implicit_cwd_with<F>(
    current: PathBuf,
    fallback: Option<PathBuf>,
    mut prepare: F,
) -> Result<PathBuf>
where
    F: FnMut(&Path) -> io::Result<()>,
{
    match prepare(&current) {
        Ok(()) => Ok(current),
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            let fallback = fallback
                .filter(|path| path != &current)
                .ok_or_else(|| {
                    anyhow!(
                        "default working directory {} cannot host YunXi session data and no user profile fallback is available: {error}",
                        current.display()
                    )
                })?;
            prepare(&fallback).with_context(|| {
                format!(
                    "failed to prepare YunXi session directory in user profile {} after access was denied in {}",
                    fallback.display(),
                    current.display()
                )
            })?;
            Ok(fallback)
        }
        Err(error) => Err(error).with_context(|| {
            format!(
                "failed to prepare YunXi session directory in default working directory {}",
                current.display()
            )
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implicit_cwd_keeps_writable_current_directory() {
        let current = PathBuf::from("current");
        let mut visited = Vec::new();

        let resolved =
            resolve_implicit_cwd_with(current.clone(), Some(PathBuf::from("profile")), |path| {
                visited.push(path.to_path_buf());
                Ok(())
            })
            .expect("writable current directory should be retained");

        assert_eq!(resolved, current);
        assert_eq!(visited, vec![PathBuf::from("current")]);
    }

    #[test]
    fn implicit_cwd_falls_back_after_permission_denied() {
        let current = PathBuf::from("protected");
        let fallback = PathBuf::from("profile");
        let mut visited = Vec::new();

        let resolved = resolve_implicit_cwd_with(current.clone(), Some(fallback.clone()), |path| {
            visited.push(path.to_path_buf());
            if path == current {
                Err(io::Error::new(io::ErrorKind::PermissionDenied, "denied"))
            } else {
                Ok(())
            }
        })
        .expect("user profile should be used as the fallback");

        assert_eq!(resolved, fallback);
        assert_eq!(
            visited,
            vec![PathBuf::from("protected"), PathBuf::from("profile")]
        );
    }

    #[test]
    fn implicit_cwd_does_not_hide_non_permission_errors() {
        let error = resolve_implicit_cwd_with(
            PathBuf::from("broken"),
            Some(PathBuf::from("profile")),
            |_| Err(io::Error::new(io::ErrorKind::AlreadyExists, "blocked")),
        )
        .expect_err("non-permission errors should remain visible");

        assert!(error.to_string().contains("broken"));
    }

    #[test]
    fn implicit_cwd_reports_missing_profile_fallback() {
        let error = resolve_implicit_cwd_with(PathBuf::from("protected"), None, |_| {
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "denied"))
        })
        .expect_err("missing fallback should be reported");

        assert!(error.to_string().contains("no user profile fallback"));
    }
}

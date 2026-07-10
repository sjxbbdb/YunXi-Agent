use std::path::PathBuf;
use tempfile::TempDir;
use yunxi_agent_tools::{
    NoopToolRuntime, ShellToolRuntime, ToolFileChangeKind, ToolRequest, ToolRuntime, ToolStatus,
};

#[tokio::test]
async fn noop_tool_runtime_declines_execution() {
    let runtime = NoopToolRuntime;

    let response = runtime
        .execute(ToolRequest::shell(PathBuf::from("."), "echo hi"))
        .await
        .expect("tool runtime response should succeed");

    assert_eq!(response.status, ToolStatus::Declined);
    assert_eq!(
        response.error.as_deref(),
        Some("YunXi tool execution is not wired in this runtime slice")
    );
}

#[tokio::test]
async fn shell_tool_runtime_reports_added_files() {
    let runtime = ShellToolRuntime;
    let temp = TempDir::new().expect("temp dir");

    let response = runtime
        .execute(ToolRequest::shell(
            temp.path(),
            "echo yunxi-file > yunxi-file.txt",
        ))
        .await
        .expect("shell runtime response should succeed");

    assert_eq!(response.status, ToolStatus::Completed);
    assert!(response.changed_files.iter().any(|change| {
        change.path == PathBuf::from("yunxi-file.txt") && change.kind == ToolFileChangeKind::Added
    }));
}

#[tokio::test]
async fn shell_tool_runtime_executes_shell_command() {
    let runtime = ShellToolRuntime;

    let response = runtime
        .execute(ToolRequest::shell(PathBuf::from("."), "echo yunxi-shell"))
        .await
        .expect("shell runtime response should succeed");

    assert_eq!(response.status, ToolStatus::Completed);
    assert_eq!(response.exit_code, Some(0));
    assert!(
        response
            .output
            .as_deref()
            .expect("shell output")
            .contains("yunxi-shell")
    );
}

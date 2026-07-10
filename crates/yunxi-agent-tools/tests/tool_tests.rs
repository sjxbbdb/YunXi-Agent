use std::path::PathBuf;
use tempfile::TempDir;
use yunxi_agent_core::{AgentConfig, ApprovalMode, SandboxMode};
use yunxi_agent_tools::{
    NoopToolRuntime, ShellToolRuntime, ToolFileChangeKind, ToolName, ToolPolicy,
    ToolPolicyDecision, ToolRegistry, ToolRequest, ToolRouteStatus, ToolRouter, ToolRuntime,
    ToolStatus, default_tool_registry,
};

#[test]
fn default_tool_registry_exposes_model_visible_specs() {
    let registry = default_tool_registry();
    let names = registry
        .model_visible_specs()
        .into_iter()
        .map(|spec| spec.name)
        .collect::<Vec<_>>();

    assert_eq!(
        names,
        vec![
            ToolName::Shell,
            ToolName::Patch,
            ToolName::Mcp,
            ToolName::Skill,
            ToolName::MultiAgent,
            ToolName::ToolSearch,
            ToolName::RequestUserInput,
            ToolName::ViewImage
        ]
    );
    assert_eq!(
        registry.spec(ToolName::Shell).expect("shell").parameters["required"],
        serde_json::json!(["command"])
    );
    assert_eq!(
        registry.spec(ToolName::Patch).expect("patch").parameters["required"],
        serde_json::json!(["op", "path"])
    );
}

#[test]
fn default_tool_registry_exports_openai_function_schema() {
    let registry = default_tool_registry();
    let tools = registry.openai_tools_json();
    let names = tools
        .iter()
        .map(|tool| {
            tool.pointer("/function/name")
                .and_then(serde_json::Value::as_str)
                .expect("tool function name")
        })
        .collect::<Vec<_>>();

    assert_eq!(
        names,
        vec![
            "shell",
            "patch",
            "mcp",
            "skill",
            "multi_agent",
            "tool_search",
            "request_user_input",
            "view_image"
        ]
    );
    assert_eq!(tools[0]["type"], "function");
    assert_eq!(
        tools[0]["function"]["parameters"]["properties"]["command"]["type"],
        "string"
    );
}

#[test]
fn tool_router_routes_requests_and_records_trace() {
    let mut request = ToolRequest::shell(PathBuf::from("."), "echo routed");
    request.id = Some("call-shell".to_string());

    let dispatch = ToolRouter::default()
        .route(request)
        .expect("shell route should exist");

    assert_eq!(dispatch.route.name, ToolName::Shell);
    assert!(dispatch.route.model_visible);
    assert_eq!(dispatch.trace.request_id.as_deref(), Some("call-shell"));
    assert_eq!(dispatch.trace.tool_name, ToolName::Shell);
    assert_eq!(dispatch.trace.route_status, ToolRouteStatus::Routed);
    assert_eq!(dispatch.trace.policy_decision, ToolPolicyDecision::Approved);
    assert!(
        dispatch
            .trace
            .summary()
            .contains("Tool dispatch routed shell")
    );
}

#[test]
fn tool_router_rejects_unregistered_requests() {
    let shell_spec = default_tool_registry()
        .spec(ToolName::Shell)
        .expect("shell")
        .clone();
    let router = ToolRouter::new(ToolRegistry::new([shell_spec]));

    let error = router
        .route(ToolRequest::patch(PathBuf::from("."), "{}"))
        .expect_err("patch is not registered");

    assert!(error.to_string().contains("tool is not registered"));
}

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

#[tokio::test]
async fn shell_tool_runtime_declines_when_approval_is_required() {
    let runtime = ShellToolRuntime;
    let temp = TempDir::new().expect("temp dir");
    let config = AgentConfig::new(temp.path())
        .with_approval_mode(ApprovalMode::OnRequest)
        .with_sandbox_mode(SandboxMode::WorkspaceWrite);

    let response = runtime
        .execute(
            ToolRequest::shell(temp.path(), "echo needs-approval")
                .with_policy(ToolPolicy::from_config(&config)),
        )
        .await
        .expect("policy response should succeed");

    assert_eq!(response.status, ToolStatus::Declined);
    assert_eq!(
        response.error.as_deref(),
        Some("tool execution requires approval")
    );
}

#[tokio::test]
async fn shell_tool_runtime_declines_cwd_outside_workspace() {
    let runtime = ShellToolRuntime;
    let workspace = TempDir::new().expect("workspace");
    let outside = TempDir::new().expect("outside");
    let config = AgentConfig::new(workspace.path())
        .with_approval_mode(ApprovalMode::Never)
        .with_sandbox_mode(SandboxMode::WorkspaceWrite);

    let response = runtime
        .execute(
            ToolRequest::shell(outside.path(), "echo outside")
                .with_policy(ToolPolicy::from_config(&config)),
        )
        .await
        .expect("policy response should succeed");

    assert_eq!(response.status, ToolStatus::Declined);
    assert!(
        response
            .error
            .as_deref()
            .expect("error")
            .contains("outside workspace")
    );
}

#[tokio::test]
async fn patch_tool_writes_updates_and_deletes_files() {
    let runtime = ShellToolRuntime;
    let temp = TempDir::new().expect("temp dir");

    let write = runtime
        .execute(ToolRequest::patch(
            temp.path(),
            r#"{"op":"write","path":"notes/yunxi.txt","content":"first"}"#,
        ))
        .await
        .expect("write patch");
    assert_eq!(write.status, ToolStatus::Completed);
    assert!(write.changed_files.iter().any(|change| {
        change.path == PathBuf::from("notes/yunxi.txt") && change.kind == ToolFileChangeKind::Added
    }));
    assert_eq!(
        std::fs::read_to_string(temp.path().join("notes/yunxi.txt")).expect("file"),
        "first"
    );

    let update = runtime
        .execute(ToolRequest::patch(
            temp.path(),
            r#"{"op":"write","path":"notes/yunxi.txt","content":"second"}"#,
        ))
        .await
        .expect("update patch");
    assert!(update.changed_files.iter().any(|change| {
        change.path == PathBuf::from("notes/yunxi.txt")
            && change.kind == ToolFileChangeKind::Updated
    }));

    let delete = runtime
        .execute(ToolRequest::patch(
            temp.path(),
            r#"{"op":"delete","path":"notes/yunxi.txt"}"#,
        ))
        .await
        .expect("delete patch");
    assert!(delete.changed_files.iter().any(|change| {
        change.path == PathBuf::from("notes/yunxi.txt")
            && change.kind == ToolFileChangeKind::Deleted
    }));
}

#[tokio::test]
async fn patch_tool_accepts_codex_style_apply_patch() {
    let runtime = ShellToolRuntime;
    let temp = TempDir::new().expect("temp dir");

    let response = runtime
        .execute(ToolRequest::patch(
            temp.path(),
            "*** Begin Patch\n*** Add File: codex-style.txt\n+hello\n*** End Patch",
        ))
        .await
        .expect("codex style patch");

    assert_eq!(response.status, ToolStatus::Completed);
    assert_eq!(
        std::fs::read_to_string(temp.path().join("codex-style.txt")).expect("file"),
        "hello\n"
    );
    assert!(response.changed_files.iter().any(|change| {
        change.path == PathBuf::from("codex-style.txt") && change.kind == ToolFileChangeKind::Added
    }));
}

#[tokio::test]
async fn patch_tool_rejects_parent_directory_escape() {
    let runtime = ShellToolRuntime;
    let temp = TempDir::new().expect("temp dir");

    let error = runtime
        .execute(ToolRequest::patch(
            temp.path(),
            r#"{"op":"write","path":"../escape.txt","content":"no"}"#,
        ))
        .await
        .expect_err("escaping patch should fail");

    assert!(error.to_string().contains("cannot escape workspace"));
}

#[tokio::test]
async fn patch_tool_rejects_absolute_paths() {
    let runtime = ShellToolRuntime;
    let temp = TempDir::new().expect("temp dir");
    let absolute_path = if cfg!(windows) {
        r"C:\escape.txt"
    } else {
        "/escape.txt"
    };
    let patch = serde_json::json!({
        "op": "write",
        "path": absolute_path,
        "content": "no"
    })
    .to_string();

    let error = runtime
        .execute(ToolRequest::patch(temp.path(), patch))
        .await
        .expect_err("absolute patch should fail");

    assert!(error.to_string().contains("must be relative"));
}

#[tokio::test]
async fn tool_search_returns_workspace_matches() {
    let runtime = ShellToolRuntime;
    let temp = TempDir::new().expect("temp dir");
    std::fs::create_dir_all(temp.path().join("src")).expect("src dir");
    std::fs::write(temp.path().join("src/lib.rs"), "").expect("file");

    let response = runtime
        .execute(ToolRequest {
            id: Some("search".to_string()),
            cwd: temp.path().to_path_buf(),
            kind: yunxi_agent_tools::ToolRequestKind::ToolSearch {
                query: "lib".to_string(),
            },
            policy: ToolPolicy::trusted(),
        })
        .await
        .expect("tool search");

    assert_eq!(response.status, ToolStatus::Completed);
    assert!(response.output.as_deref().expect("output").contains("src"));
}

#[tokio::test]
async fn request_user_input_declines_without_interactive_host() {
    let runtime = ShellToolRuntime;
    let temp = TempDir::new().expect("temp dir");

    let response = runtime
        .execute(ToolRequest {
            id: None,
            cwd: temp.path().to_path_buf(),
            kind: yunxi_agent_tools::ToolRequestKind::RequestUserInput {
                prompt: "Proceed?".to_string(),
            },
            policy: ToolPolicy::trusted(),
        })
        .await
        .expect("request input");

    assert_eq!(response.status, ToolStatus::Declined);
    assert!(
        response
            .error
            .as_deref()
            .expect("error")
            .contains("interactive host")
    );
}

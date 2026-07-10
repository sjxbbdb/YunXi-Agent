use yunxi_agent_core::{
    AgentEvent, AgentRunStatus, CommandStatus, FileChangeKind, McpToolStatus, PatchStatus,
    TodoStatus, TokenUsage,
};

#[test]
fn command_event_serializes_with_stable_shape() {
    let event = AgentEvent::CommandCompleted {
        id: Some("item_1".to_string()),
        command: "cargo test".to_string(),
        aggregated_output: "ok".to_string(),
        exit_code: Some(0),
        status: CommandStatus::Completed,
    };

    let json = serde_json::to_value(event).expect("json");

    assert_eq!(json["type"], "commandCompleted");
    assert_eq!(json["id"], "item_1");
    assert_eq!(json["command"], "cargo test");
    assert_eq!(json["aggregated_output"], "ok");
    assert_eq!(json["exit_code"], 0);
    assert_eq!(json["status"], "completed");
}

#[test]
fn completion_event_can_include_usage() {
    let event = AgentEvent::Completed {
        status: AgentRunStatus::Completed,
        usage: Some(TokenUsage {
            input_tokens: 10,
            cached_input_tokens: 2,
            output_tokens: 5,
            reasoning_output_tokens: 1,
        }),
    };

    let json = serde_json::to_value(event).expect("json");

    assert_eq!(json["type"], "completed");
    assert_eq!(json["status"], "completed");
    assert_eq!(json["usage"]["input_tokens"], 10);
}

#[test]
fn mcp_patch_file_and_todo_events_have_stable_names() {
    let events = vec![
        AgentEvent::FileChanged {
            path: "src/lib.rs".to_string(),
            kind: FileChangeKind::Update,
        },
        AgentEvent::PatchCompleted {
            status: PatchStatus::Completed,
        },
        AgentEvent::McpToolCompleted {
            id: Some("item_2".to_string()),
            server: "filesystem".to_string(),
            tool: "read_file".to_string(),
            status: McpToolStatus::Completed,
        },
        AgentEvent::TodoUpdated {
            id: Some("item_3".to_string()),
            items: vec![TodoStatus {
                text: "inspect".to_string(),
                completed: true,
            }],
        },
    ];

    let names: Vec<String> = events
        .into_iter()
        .map(|event| {
            serde_json::to_value(event).expect("json")["type"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();

    assert_eq!(
        names,
        vec![
            "fileChanged",
            "patchCompleted",
            "mcpToolCompleted",
            "todoUpdated"
        ]
    );
}

#[test]
fn dynamic_tool_events_have_stable_names() {
    let events = vec![
        AgentEvent::ToolCallStarted {
            id: Some("tool-1".to_string()),
            name: "tool_search".to_string(),
            arguments_json: Some(r#"{"query":"runtime"}"#.to_string()),
        },
        AgentEvent::ToolCallCompleted {
            id: Some("tool-1".to_string()),
            name: "tool_search".to_string(),
            output: "[]".to_string(),
            status: CommandStatus::Completed,
        },
    ];

    let names = events
        .into_iter()
        .map(|event| {
            serde_json::to_value(event).expect("json")["type"]
                .as_str()
                .expect("type")
                .to_string()
        })
        .collect::<Vec<_>>();

    assert_eq!(names, vec!["toolCallStarted", "toolCallCompleted"]);
}

#[test]
fn approval_events_have_stable_names() {
    let events = vec![
        AgentEvent::ApprovalRequested {
            id: Some("call-1".to_string()),
            tool_name: "shell".to_string(),
            reason: "tool execution requires approval".to_string(),
        },
        AgentEvent::ApprovalCompleted {
            id: Some("call-1".to_string()),
            approved: false,
            reason: Some("declined".to_string()),
        },
    ];

    let names = events
        .into_iter()
        .map(|event| {
            serde_json::to_value(event).expect("json")["type"]
                .as_str()
                .expect("type")
                .to_string()
        })
        .collect::<Vec<_>>();

    assert_eq!(names, vec!["approvalRequested", "approvalCompleted"]);
}

#[test]
fn escalation_events_have_stable_names() {
    let events = vec![
        AgentEvent::EscalationRequested {
            id: Some("call-1".to_string()),
            tool_name: "shell".to_string(),
            reason: "sandbox is read-only".to_string(),
            required_sandbox: Some("workspace-write".to_string()),
            required_network: None,
        },
        AgentEvent::EscalationCompleted {
            id: Some("call-1".to_string()),
            approved: false,
            reason: Some("sandbox is read-only".to_string()),
        },
    ];

    let names = events
        .into_iter()
        .map(|event| {
            serde_json::to_value(event).expect("json")["type"]
                .as_str()
                .expect("type")
                .to_string()
        })
        .collect::<Vec<_>>();

    assert_eq!(names, vec!["escalationRequested", "escalationCompleted"]);
}

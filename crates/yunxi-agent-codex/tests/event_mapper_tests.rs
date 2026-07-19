use yunxi_agent_codex::map_exec_jsonl;
use yunxi_agent_core::{
    AgentEvent, AgentRunStatus, CommandStatus, FileChangeKind, McpToolStatus, PatchStatus,
};

#[test]
fn maps_thread_turn_message_and_completion_events() {
    let jsonl = r#"
{"type":"thread.started","thread_id":"thread_1"}
{"type":"turn.started"}
{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"hello"}}
{"type":"turn.completed","usage":{"input_tokens":3,"cached_input_tokens":1,"output_tokens":2,"reasoning_output_tokens":0}}
"#;

    let events = map_exec_jsonl(jsonl).expect("events");

    assert_eq!(
        events,
        vec![
            AgentEvent::ThreadStarted {
                thread_id: "thread_1".to_string()
            },
            AgentEvent::TurnStarted,
            AgentEvent::Message {
                content: "hello".to_string(),
                stream: None
            },
            AgentEvent::Completed {
                status: AgentRunStatus::Completed,
                usage: Some(yunxi_agent_core::TokenUsage {
                    input_tokens: 3,
                    cached_input_tokens: 1,
                    output_tokens: 2,
                    reasoning_output_tokens: 0,
                })
            }
        ]
    );
}

#[test]
fn maps_command_file_patch_and_mcp_events() {
    let jsonl = r#"
{"type":"item.started","item":{"id":"cmd_1","type":"command_execution","command":"cargo test","aggregated_output":"","exit_code":null,"status":"in_progress"}}
{"type":"item.updated","item":{"id":"cmd_1","type":"command_execution","command":"cargo test","aggregated_output":"ok","exit_code":null,"status":"in_progress"}}
{"type":"item.completed","item":{"id":"cmd_1","type":"command_execution","command":"cargo test","aggregated_output":"ok","exit_code":0,"status":"completed"}}
{"type":"item.started","item":{"id":"patch_1","type":"file_change","changes":[{"path":"src/lib.rs","kind":"update"}],"status":"in_progress"}}
{"type":"item.updated","item":{"id":"patch_1","type":"file_change","changes":[{"path":"src/main.rs","kind":"add"}],"status":"in_progress"}}
{"type":"item.completed","item":{"id":"patch_1","type":"file_change","changes":[{"path":"src/lib.rs","kind":"update"}],"status":"completed"}}
{"type":"item.completed","item":{"id":"mcp_1","type":"mcp_tool_call","server":"fs","tool":"read_file","arguments":{},"result":null,"error":null,"status":"completed"}}
"#;

    let events = map_exec_jsonl(jsonl).expect("events");
    let patch_completed_events = events
        .iter()
        .filter(|event| matches!(event, AgentEvent::PatchCompleted { .. }))
        .collect::<Vec<_>>();

    assert!(events.contains(&AgentEvent::CommandStarted {
        id: Some("cmd_1".to_string()),
        command: "cargo test".to_string(),
    }));
    assert!(events.contains(&AgentEvent::CommandUpdated {
        id: Some("cmd_1".to_string()),
        command: "cargo test".to_string(),
        aggregated_output: "ok".to_string(),
    }));
    assert!(events.contains(&AgentEvent::CommandCompleted {
        id: Some("cmd_1".to_string()),
        command: "cargo test".to_string(),
        aggregated_output: "ok".to_string(),
        exit_code: Some(0),
        status: CommandStatus::Completed,
    }));
    assert!(events.contains(&AgentEvent::FileChanged {
        path: "src/lib.rs".to_string(),
        kind: FileChangeKind::Update,
    }));
    assert!(events.contains(&AgentEvent::FileChanged {
        path: "src/main.rs".to_string(),
        kind: FileChangeKind::Add,
    }));
    assert_eq!(patch_completed_events.len(), 3);
    assert!(
        patch_completed_events.contains(&&AgentEvent::PatchCompleted {
            status: PatchStatus::InProgress,
        })
    );
    assert!(
        patch_completed_events.contains(&&AgentEvent::PatchCompleted {
            status: PatchStatus::Completed,
        })
    );
    assert!(events.contains(&AgentEvent::McpToolCompleted {
        id: Some("mcp_1".to_string()),
        server: "fs".to_string(),
        tool: "read_file".to_string(),
        status: McpToolStatus::Completed,
    }));
}

#[test]
fn malformed_jsonl_returns_agent_error() {
    let error = map_exec_jsonl("{not json").expect_err("bad json should fail");

    assert!(error.to_string().contains("malformed upstream event"));
}

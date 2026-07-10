use std::path::PathBuf;
use yunxi_agent_core::{AgentConfig, AgentInput};
use yunxi_agent_protocol::ToolCall;
use yunxi_agent_provider::{
    AgentProvider, OpenAiCompatibleProvider, ProviderAuth, ProviderConfig, ProviderRequest,
    ProviderRole, ProviderToolCall, StaticProvider, build_openai_request_json,
    parse_openai_response_json,
};

#[tokio::test]
async fn static_provider_returns_yunxi_runtime_message() {
    let provider = StaticProvider::default();
    let request = ProviderRequest::new(
        AgentConfig::new(PathBuf::from(".")),
        AgentInput::text("explain this project"),
    );

    let response = provider
        .complete(request)
        .await
        .expect("provider response should succeed");

    assert_eq!(
        response
            .message
            .as_ref()
            .map(|message| message.content.as_str()),
        Some("YunXi autonomous runtime accepted prompt: explain this project")
    );
}

#[test]
fn openai_request_json_uses_yunxi_provider_messages() {
    let request = ProviderRequest::new(
        AgentConfig::new(PathBuf::from(".")).with_model("yunxi-model"),
        AgentInput::text("explain this project"),
    );

    let json = build_openai_request_json(
        &ProviderConfig::openai_compatible("fallback-model"),
        &request,
    )
    .expect("request json");

    assert_eq!(json["model"], "yunxi-model");
    assert_eq!(json["messages"][0]["role"], "user");
    assert_eq!(json["messages"][0]["content"], "explain this project");
}

#[test]
fn openai_response_json_parses_assistant_text_and_usage() {
    let response = parse_openai_response_json(
        r#"{
          "choices": [
            { "message": { "role": "assistant", "content": "hello from fixture" } }
          ],
          "usage": {
            "prompt_tokens": 7,
            "completion_tokens": 11,
            "prompt_tokens_details": { "cached_tokens": 3 },
            "completion_tokens_details": { "reasoning_tokens": 5 }
          }
        }"#,
    )
    .expect("provider response");

    assert_eq!(
        response
            .message
            .as_ref()
            .map(|message| message.content.as_str()),
        Some("hello from fixture")
    );
    assert!(response.tool_calls.is_empty());
    let usage = response.usage.expect("usage");
    assert_eq!(usage.input_tokens, 7);
    assert_eq!(usage.cached_input_tokens, 3);
    assert_eq!(usage.output_tokens, 11);
    assert_eq!(usage.reasoning_output_tokens, 5);
}

#[test]
fn openai_response_json_parses_shell_tool_call() {
    let response = parse_openai_response_json(
        r#"{
          "choices": [
            {
              "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [
                  {
                    "id": "call_1",
                    "type": "function",
                    "function": {
                      "name": "shell",
                      "arguments": "{\"command\":\"echo yunxi\"}"
                    }
                  }
                ]
              }
            }
          ]
        }"#,
    )
    .expect("provider response");

    assert_eq!(response.message, None);
    assert_eq!(
        response.tool_calls,
        vec![ProviderToolCall::Shell {
            id: Some("call_1".to_string()),
            command: "echo yunxi".to_string()
        }]
    );
}

#[test]
fn openai_response_json_parses_patch_tool_call() {
    let response = parse_openai_response_json(
        r#"{
          "choices": [
            {
              "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [
                  {
                    "id": "call_patch",
                    "type": "function",
                    "function": {
                      "name": "patch",
                      "arguments": "{\"op\":\"write\",\"path\":\"notes.txt\",\"content\":\"hello\"}"
                    }
                  }
                ]
              }
            }
          ]
        }"#,
    )
    .expect("provider response");

    assert_eq!(response.message, None);
    assert_eq!(
        response.tool_calls,
        vec![ProviderToolCall::Patch {
            id: Some("call_patch".to_string()),
            patch: r#"{"op":"write","path":"notes.txt","content":"hello"}"#.to_string()
        }]
    );
}

#[test]
fn provider_tool_call_converts_to_yunxi_protocol_tool_call() {
    let call = ProviderToolCall::Shell {
        id: Some("call-1".to_string()),
        command: "echo yunxi".to_string(),
    };

    assert_eq!(
        ToolCall::from(call),
        ToolCall::Shell {
            id: Some("call-1".to_string()),
            command: "echo yunxi".to_string()
        }
    );
}

#[tokio::test]
async fn openai_compatible_provider_can_use_fixture_response() {
    let provider = OpenAiCompatibleProvider::new(
        ProviderConfig::openai_compatible("fixture-model"),
        ProviderAuth::None,
    )
    .with_fixture_response(
        r#"{
          "choices": [
            { "message": { "role": "assistant", "content": "fixture answer" } }
          ]
        }"#,
    );
    let request = ProviderRequest::new(
        AgentConfig::new(PathBuf::from(".")),
        AgentInput::text("fixture"),
    );

    let response = provider.complete(request).await.expect("fixture response");

    assert_eq!(
        response.message.as_ref().map(|message| message.role),
        Some(ProviderRole::Assistant)
    );
    assert_eq!(
        response
            .message
            .as_ref()
            .map(|message| message.content.as_str()),
        Some("fixture answer")
    );
}

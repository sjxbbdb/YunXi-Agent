use yunxi_agent_codex::CodexNativeBackend;
use yunxi_agent_core::AgentBackend;

#[test]
fn native_backend_is_an_agent_backend() {
    fn assert_backend<T: AgentBackend>() {}

    assert_backend::<CodexNativeBackend>();
}

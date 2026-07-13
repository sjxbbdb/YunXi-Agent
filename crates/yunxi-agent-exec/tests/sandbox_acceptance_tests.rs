use tempfile::TempDir;
use yunxi_agent_exec::{ExecCommand, ExecLifecycleEvent, ExecManager};
use yunxi_agent_sandbox::{
    ApprovalRequirement, ExecutionPolicy, NetworkPolicy, PolicyDecision, SandboxEnforcementLevel,
    SandboxRequirement, SandboxRunner,
};

#[cfg(windows)]
fn create_dir_symlink(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(src, dst)
}

#[cfg(unix)]
fn create_dir_symlink(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(src, dst)
}

fn policy(
    workspace: &TempDir,
    sandbox: SandboxRequirement,
    network: NetworkPolicy,
) -> ExecutionPolicy {
    ExecutionPolicy {
        approval: ApprovalRequirement::PreApproved,
        sandbox,
        network,
        workspace_root: workspace.path().to_path_buf(),
    }
}

#[test]
fn read_only_write_requires_sandbox_escalation() {
    let workspace = TempDir::new().expect("workspace");
    let policy = policy(
        &workspace,
        SandboxRequirement::ReadOnly,
        NetworkPolicy::Inherit,
    );
    let evaluation = policy.evaluate(workspace.path(), Some("echo yunxi > inside.txt"));

    assert!(matches!(
        evaluation.decision,
        PolicyDecision::Blocked { ref reason } if reason.contains("read-only")
    ));
    assert_eq!(
        evaluation
            .escalation_request
            .expect("escalation")
            .required_sandbox,
        Some(SandboxRequirement::WorkspaceWrite)
    );
}

#[test]
fn workspace_write_absolute_escape_requires_full_access_escalation() {
    let workspace = TempDir::new().expect("workspace");
    let outside = TempDir::new().expect("outside");
    let policy = policy(
        &workspace,
        SandboxRequirement::WorkspaceWrite,
        NetworkPolicy::Inherit,
    );
    let target = outside.path().join("leak.txt");
    let command = format!("echo yunxi > \"{}\"", target.display());
    let evaluation = policy.evaluate(workspace.path(), Some(&command));

    assert!(matches!(
        evaluation.decision,
        PolicyDecision::Blocked { ref reason }
            if reason.contains("workspace-write target")
                && reason.contains("outside workspace")
    ));
    assert_eq!(
        evaluation
            .escalation_request
            .expect("escalation")
            .required_sandbox,
        Some(SandboxRequirement::DangerFullAccess)
    );
    assert!(!target.exists());
}

#[test]
fn workspace_write_symlink_escape_is_rejected_before_spawn() {
    let workspace = TempDir::new().expect("workspace");
    let outside = TempDir::new().expect("outside");
    let link = workspace.path().join("outside-link");
    if create_dir_symlink(outside.path(), &link).is_err() {
        return;
    }

    let policy = policy(
        &workspace,
        SandboxRequirement::WorkspaceWrite,
        NetworkPolicy::Inherit,
    );
    let command = if cfg!(windows) {
        "echo yunxi > outside-link\\leak.txt"
    } else {
        "echo yunxi > outside-link/leak.txt"
    };
    let evaluation = policy.evaluate(workspace.path(), Some(command));

    assert!(matches!(
        evaluation.decision,
        PolicyDecision::Blocked { ref reason }
            if reason.contains("workspace-write target")
                && reason.contains("outside workspace")
    ));
    assert!(!outside.path().join("leak.txt").exists());
}

#[test]
fn network_disabled_commands_request_network_escalation() {
    let workspace = TempDir::new().expect("workspace");
    let policy = policy(
        &workspace,
        SandboxRequirement::WorkspaceWrite,
        NetworkPolicy::Disabled,
    );
    let evaluation = policy.evaluate(workspace.path(), Some("curl https://example.test"));

    assert!(matches!(
        evaluation.decision,
        PolicyDecision::Blocked { ref reason } if reason.contains("network")
    ));
    assert_eq!(
        evaluation
            .escalation_request
            .expect("network escalation")
            .required_network,
        Some(NetworkPolicy::Enabled)
    );
}

#[tokio::test]
async fn danger_full_access_runs_but_reports_policy_bypass() {
    let workspace = TempDir::new().expect("workspace");
    let policy = policy(
        &workspace,
        SandboxRequirement::DangerFullAccess,
        NetworkPolicy::Inherit,
    );
    let command = ExecCommand::shell(workspace.path(), "echo YUNXI_EXEC_ACCEPTANCE_OK", policy);

    let trace = ExecManager::default()
        .run(command)
        .await
        .expect("exec trace");

    assert!(
        trace
            .summary
            .aggregated_output
            .contains("YUNXI_EXEC_ACCEPTANCE_OK")
    );
    let diagnostic = trace.events.iter().find_map(|event| match event {
        ExecLifecycleEvent::RunnerDiagnostic { diagnostic, .. } => Some(diagnostic),
        _ => None,
    });
    let diagnostic = diagnostic.expect("runner diagnostic");
    assert_eq!(
        diagnostic.enforcement_level,
        SandboxEnforcementLevel::PolicyBypass
    );
    assert_eq!(diagnostic.enforcement, "policy_bypass");
    assert!(!diagnostic.os_isolation);
}

#[test]
fn default_platform_runner_is_honest_when_os_isolation_is_not_verified() {
    let workspace = TempDir::new().expect("workspace");
    let policy = policy(
        &workspace,
        SandboxRequirement::WorkspaceWrite,
        NetworkPolicy::Inherit,
    );

    let diagnostic = SandboxRunner.diagnostic(&policy, workspace.path(), Some("echo yunxi"));

    assert!(!diagnostic.os_isolation);
    assert_ne!(
        diagnostic.enforcement_level,
        SandboxEnforcementLevel::OsRestricted
    );
    assert_eq!(
        diagnostic.enforcement,
        diagnostic.enforcement_level.as_str()
    );
    assert!(diagnostic.unsupported_reason.is_some());
}

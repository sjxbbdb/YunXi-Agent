use std::path::PathBuf;

use tempfile::TempDir;
use yunxi_agent_codex::CodexRuntimeSource;
use yunxi_agent_core::AgentError;
use yunxi_agent_core::CodexSource;

#[test]
fn runtime_source_rejects_missing_codex_rs_root() {
    let source = CodexRuntimeSource::new(PathBuf::from("Z:/missing/codex-rs"));

    let error = source.verify().expect_err("missing source should fail");

    assert!(matches!(error, AgentError::MissingCodexSource { .. }));
}

#[test]
fn runtime_source_accepts_minimum_headless_crate_layout() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    for dir in [
        "exec/src",
        "core",
        "protocol",
        "config",
        "login",
        "app-server-client",
        "app-server-protocol",
    ] {
        std::fs::create_dir_all(root.join(dir)).expect("dir");
    }
    for file in [
        "Cargo.toml",
        "exec/Cargo.toml",
        "exec/src/lib.rs",
        "core/Cargo.toml",
        "protocol/Cargo.toml",
        "config/Cargo.toml",
        "login/Cargo.toml",
        "app-server-client/Cargo.toml",
        "app-server-protocol/Cargo.toml",
    ] {
        std::fs::write(root.join(file), "[package]\nname = \"fixture\"\n").expect("file");
    }

    let status = CodexRuntimeSource::new(root)
        .verify()
        .expect("layout should verify");

    assert_eq!(status.root, root);
    assert_eq!(status.exec_manifest, root.join("exec/Cargo.toml"));
    assert_eq!(status.exec_lib, root.join("exec/src/lib.rs"));
}

#[test]
fn runtime_source_bridges_from_core_codex_source() {
    let temp = TempDir::new().expect("temp dir");
    let checkout_root = temp.path();
    let root = checkout_root.join("codex-rs");
    for dir in [
        "exec/src",
        "core",
        "protocol",
        "config",
        "login",
        "app-server-client",
        "app-server-protocol",
    ] {
        std::fs::create_dir_all(root.join(dir)).expect("dir");
    }
    for file in [
        "Cargo.toml",
        "exec/Cargo.toml",
        "exec/src/lib.rs",
        "core/Cargo.toml",
        "protocol/Cargo.toml",
        "config/Cargo.toml",
        "login/Cargo.toml",
        "app-server-client/Cargo.toml",
        "app-server-protocol/Cargo.toml",
    ] {
        std::fs::write(root.join(file), "[package]\nname = \"fixture\"\n").expect("file");
    }

    let source = CodexSource::new(checkout_root);
    let runtime_source = CodexRuntimeSource::from_codex_source(&source);

    assert_eq!(runtime_source.root(), root);

    let status = runtime_source.verify().expect("layout should verify");

    assert_eq!(status.root, root);
    assert_eq!(status.exec_manifest, root.join("exec/Cargo.toml"));
    assert_eq!(status.exec_lib, root.join("exec/src/lib.rs"));
}

#[test]
fn runtime_source_rejects_missing_required_file_when_root_exists() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    for dir in [
        "exec/src",
        "core",
        "protocol",
        "config",
        "login",
        "app-server-client",
        "app-server-protocol",
    ] {
        std::fs::create_dir_all(root.join(dir)).expect("dir");
    }
    for file in [
        "Cargo.toml",
        "exec/Cargo.toml",
        "exec/src/lib.rs",
        "protocol/Cargo.toml",
        "config/Cargo.toml",
        "login/Cargo.toml",
        "app-server-client/Cargo.toml",
        "app-server-protocol/Cargo.toml",
    ] {
        std::fs::write(root.join(file), "[package]\nname = \"fixture\"\n").expect("file");
    }

    let error = CodexRuntimeSource::new(root)
        .verify()
        .expect_err("missing required file should fail");

    assert!(matches!(error, AgentError::MissingCodexSource { .. }));
}

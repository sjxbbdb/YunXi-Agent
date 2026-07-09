use std::path::PathBuf;
use tempfile::TempDir;
use yunxi_agent_core::{AgentError, CodexSource};

#[test]
fn verify_rejects_missing_source_root() {
    let source = CodexSource::new(PathBuf::from("Z:/definitely/missing/codex"));

    let error = source.verify().expect_err("missing source should fail");

    assert!(matches!(error, AgentError::MissingCodexSource { .. }));
}

#[test]
fn verify_accepts_minimum_expected_codex_layout() {
    let temp = TempDir::new().expect("temp dir should be created");
    let root = temp.path();
    std::fs::create_dir_all(root.join("codex-rs/exec/src")).expect("exec dir should exist");
    std::fs::create_dir_all(root.join("codex-rs/app-server-client"))
        .expect("client dir should exist");
    std::fs::write(root.join("codex-rs/Cargo.toml"), "[workspace]\n").expect("manifest");
    std::fs::write(
        root.join("codex-rs/exec/src/lib.rs"),
        "pub fn marker() {}\n",
    )
    .expect("exec lib");
    std::fs::write(
        root.join("codex-rs/app-server-client/Cargo.toml"),
        "[package]\nname = \"codex-app-server-client\"\n",
    )
    .expect("client manifest");

    let status = CodexSource::new(root)
        .verify()
        .expect("layout should verify");

    assert_eq!(status.root, root);
    assert_eq!(status.codex_rs_manifest, root.join("codex-rs/Cargo.toml"));
    assert_eq!(status.exec_lib, root.join("codex-rs/exec/src/lib.rs"));
    assert_eq!(
        status.app_server_client_manifest,
        root.join("codex-rs/app-server-client/Cargo.toml")
    );
}

#[test]
fn verify_rejects_directory_where_manifest_file_is_expected() {
    let temp = TempDir::new().expect("temp dir should be created");
    let root = temp.path();
    std::fs::create_dir_all(root.join("codex-rs/Cargo.toml")).expect("manifest impostor");
    std::fs::create_dir_all(root.join("codex-rs/exec/src")).expect("exec dir should exist");
    std::fs::create_dir_all(root.join("codex-rs/app-server-client"))
        .expect("client dir should exist");
    std::fs::write(
        root.join("codex-rs/exec/src/lib.rs"),
        "pub fn marker() {}\n",
    )
    .expect("exec lib");
    std::fs::write(
        root.join("codex-rs/app-server-client/Cargo.toml"),
        "[package]\nname = \"codex-app-server-client\"\n",
    )
    .expect("client manifest");

    let error = CodexSource::new(root)
        .verify()
        .expect_err("manifest directory should fail verification");

    assert!(matches!(error, AgentError::MissingCodexSource { .. }));
}

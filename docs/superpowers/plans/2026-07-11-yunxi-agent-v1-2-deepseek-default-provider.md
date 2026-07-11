# YunXi Agent v1.2 DeepSeek Default Provider Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make YunXi Agent v1.2 automatically use the real DeepSeek provider when local credentials are configured, while preserving explicit live/offline controls and upstream-independent runtime behavior.

**Architecture:** Keep provider profile and credential-source resolution in `yunxi-agent-provider`; add a small CLI-owned provider mode resolver shared by one-shot, interactive, and session resume paths. The CLI passes only the resolved live/offline decision into the existing YunXi runtime constructors, so DeepSeek remains behind the replaceable `AgentProvider` interface.

**Tech Stack:** Rust 2024, Clap 4, Tokio, Reqwest transport already owned by YunXi, PowerShell credential import/smoke helpers, GitHub Git Data REST API.

## Global Constraints

- Work in `D:\YunXi Agent` on the current `master` checkout, as explicitly authorized by the user.
- Do not run tests, checks, builds, formatters, or smoke commands during construction.
- Write all production code, tests, scripts, version metadata, and documentation first; run one final unified verification gate only after construction is complete.
- Do not spend repeated cycles on one point; record unresolved verification failures and fix them in one bounded final pass.
- Never print, log, commit, or transmit API key/token values as repository content.
- The product binary must not read `<private-api-file>`; that file is only a local import/live-test input.
- Default CLI dependencies must not include `vendor/codex-rs`, `codex-*`, or `yunxi-agent-codex`.
- Release version is exactly `1.2.0`; create new tag `v1.2.0` and preserve `v1.0.0` and `v1.1.0` unchanged.
- All GitHub remote reads and writes must use GitHub REST API. Do not use `git push`, `git fetch`, or `git ls-remote` for remote synchronization.
- Update `C:\Users\admin\Desktop\YunXi Agent开发日志.md` after implementation and verification.
- After verification, remove `target`, repository-root `.yunxi`, and temporary smoke/fixture files.
- The user explicitly overrides TDD's intermediate red/green execution requirement: tests are authored before their production slice, but all execution is deferred to the final gate.

---

### Task 1: Provider profile and credential-source resolution

**Files:**
- Modify: `crates/yunxi-agent-provider/src/lib.rs`
- Modify: `crates/yunxi-agent-provider/tests/provider_tests.rs`

**Interfaces:**
- Consumes: `AgentConfig`, `ProviderConfig`, `ProviderAuth`.
- Produces: `ProviderConfig::from_agent_config_with_env`, `ProviderBootstrap::from_agent_config_with_env`, `ProviderBootstrap::credentials_configured`, and `ProviderBootstrap::credentials_configured_with_env`.

- [ ] **Step 1: Add provider resolution tests before production edits**

Add tests that pass a deterministic environment lookup closure and assert:

```rust
let env = HashMap::from([("DEEPSEEK_API_KEY", "secret")]);
let bootstrap = ProviderBootstrap::from_agent_config_with_env(
    &AgentConfig::new("."),
    |name| env.get(name).map(|value| (*value).to_string()),
);
assert_eq!(bootstrap.config.profile.as_deref(), Some("deepseek"));
assert_eq!(bootstrap.auth, ProviderAuth::EnvVar("DEEPSEEK_API_KEY".into()));
assert!(bootstrap.credentials_configured_with_env(|name| {
    env.get(name).map(|value| (*value).to_string())
}));
```

Cover explicit `YUNXI_PROVIDER_API_KEY`, named `YUNXI_PROVIDER_API_KEY_ENV`, empty values, explicit non-DeepSeek profile, and `OPENAI_API_KEY` fallback. Do not execute tests yet.

- [ ] **Step 2: Add lookup-driven provider config resolution**

Implement production `from_agent_config` as a wrapper over:

```rust
pub fn from_agent_config_with_env<F>(config: &AgentConfig, env: F) -> Self
where
    F: Fn(&str) -> Option<String>,
```

Resolve explicit profile first, then infer `deepseek` only when a non-empty `DEEPSEEK_API_KEY` exists. Preserve existing model, base URL, and stream overrides.

- [ ] **Step 3: Add lookup-driven bootstrap and credential probing**

Implement auth precedence exactly as the design specifies. `credentials_configured_with_env` must return only a boolean and must never format or expose the credential value. `credentials_configured` wraps `std::env::var`.

---

### Task 2: Shared CLI provider mode and offline override

**Files:**
- Create: `crates/yunxi-agent-cli/src/provider_mode.rs`
- Modify: `crates/yunxi-agent-cli/Cargo.toml`
- Modify: `crates/yunxi-agent-cli/src/main.rs`
- Modify: `crates/yunxi-agent-cli/src/interactive.rs`
- Modify: `crates/yunxi-agent-cli/src/render.rs`
- Modify: `crates/yunxi-agent-cli/tests/cli_tests.rs`
- Modify: `crates/yunxi-agent-cli/tests/jsonl_tests.rs`

**Interfaces:**
- Consumes: `ProviderBootstrap`, `AgentConfig`, `BackendKind`, CLI `--provider-live` and `--offline` flags.
- Produces: `ProviderMode`, `ProviderSelection`, `ProviderMode::from_flags`, and `ProviderMode::resolve`.

- [ ] **Step 1: Add CLI behavior tests before production edits**

Add integration coverage for:

```rust
cmd.env("DEEPSEEK_API_KEY", "fixture-secret")
    .write_stdin("/exit\n")
    .assert()
    .success()
    .stdout(predicate::str::contains("provider_mode: live"))
    .stdout(predicate::str::contains("provider: deepseek"));
```

Also cover `--offline`, forced-live missing credentials, and the Clap conflict between `--provider-live` and `--offline`. Update every deterministic YunXi fixture test to pass `--offline`, and remove inherited provider credential variables where needed. Do not execute tests yet.

- [ ] **Step 2: Add the provider mode module**

Define:

```rust
pub(crate) enum ProviderMode { Auto, ForcedLive, ForcedOffline }
pub(crate) enum ProviderModeSource { AutoLive, ForcedLive, ForcedOffline, AutoOffline }
pub(crate) struct ProviderSelection {
    pub live: bool,
    pub source: ProviderModeSource,
    pub provider: String,
    pub model: String,
}
```

`ProviderMode::resolve` uses `ProviderBootstrap` and returns a provider-classified error before HTTP execution when forced live lacks credentials. Non-Yunxi backends remain unchanged.

- [ ] **Step 3: Wire one-shot and session resume**

Parse `--offline` with `conflicts_with = "provider_live"`. Build one `ProviderMode` in `run_cli`, pass it through `run_command`/`run_session_command`, and resolve it immediately before every call to `run_agent_backend`.

- [ ] **Step 4: Wire interactive mode and banner**

Store `ProviderMode` in `InteractiveOptions` and `InteractiveSession`. Resolve once before entering the REPL and again before each turn after `/provider` or `/model` changes. Replace the old boolean banner with explicit lines:

```text
provider_mode: live
provider: deepseek
model: deepseek-v4-flash
```

Keep JSON/JSONL output free of banner text.

---

### Task 3: Secret-safe local DeepSeek credential import

**Files:**
- Create: `scripts/provider/import-deepseek-credential.ps1`
- Modify: `scripts/provider/deepseek-live-smoke.ps1`

**Interfaces:**
- Consumes: explicit `-ApiFile`, optional `-Model`, user environment scope.
- Produces: user-level `DEEPSEEK_API_KEY`, `YUNXI_PROVIDER_PROFILE=deepseek`, and `YUNXI_AGENT_MODEL`; smoke harness accepts `-ApiFile` and an explicit multi-candidate index rather than containing a personal path.

- [ ] **Step 1: Implement unique credential extraction**

Use a bounded `sk-` token regex, de-duplicate matches, automatically accept exactly one candidate, require an explicit one-based `-CredentialIndex` when several candidates exist, and never write the candidate to output. The script must accept:

```powershell
param(
    [Parameter(Mandatory = $true)][string]$ApiFile,
    [int]$CredentialIndex = 0,
    [string]$Model = "deepseek-v4-flash"
)
```

- [ ] **Step 2: Persist only user environment configuration**

Use `[Environment]::SetEnvironmentVariable(..., "User")`. Output only status, variable names, selected model, and `restart_shell_required=true`.

- [ ] **Step 3: Remove personal path from live smoke**

Make `deepseek-live-smoke.ps1` require `-ApiFile` and accept `-CredentialIndex`; preserve stream/non-stream behavior and secret-leak detection while ensuring the path is not hard-coded in committed source.

---

### Task 4: Version, user documentation, status, and development report

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `README.md`
- Modify: `docs/extraction-status.md`
- Create: `docs/reports/2026-07-11-yunxi-agent-v1-2-deepseek-default-provider-development-report.md`

**Interfaces:**
- Consumes: completed v1.2 behavior and final verification command list.
- Produces: v1.2 package metadata, user instructions, release report, and status record.

- [ ] **Step 1: Promote workspace version to 1.2.0**

Change workspace-owned package versions in `Cargo.toml` and matching local package entries in `Cargo.lock`. Do not alter third-party dependency versions.

- [ ] **Step 2: Update README usage and security guidance**

Document automatic DeepSeek selection, `--provider-live`, `--offline`, credential import, environment precedence, and API-only GitHub publication policy. Remove ordinary `git push` instructions.

- [ ] **Step 3: Add v1.2 construction status and report**

Record exact files and intended verification gate without claiming pass status before commands run. Leave verification result fields explicitly marked as pending construction completion, then replace them with actual outcomes after the final gate.

---

### Task 5: Final unified verification, review, release, and API publication

**Files:**
- Modify after evidence: `docs/extraction-status.md`
- Modify after evidence: `docs/reports/2026-07-11-yunxi-agent-v1-2-deepseek-default-provider-development-report.md`
- Modify outside repository: `C:\Users\admin\Desktop\YunXi Agent开发日志.md`

**Interfaces:**
- Consumes: all constructed source, tests, scripts, docs, local API credential files.
- Produces: verified release binaries, installed v1.2 CLI, commit, immutable tags, GitHub API refs, updated CodeGraph, and cleaned workspace.

- [ ] **Step 1: Run the single unified local gate**

Run, in order:

```powershell
cargo fmt
cargo fmt -- --check
cargo test
cargo check --workspace
cargo build -p yunxi-agent-cli --release --bins
target\release\yunxi.exe --version
target\release\yunxi.exe --offline --backend yunxi "YunXi Agent v1.2 offline smoke"
@("/session"; "/exit") | target\release\yunxi.exe --offline --backend yunxi
target\release\yunxi.exe --provider-live --offline "conflict smoke"
cargo run -p yunxi-agent-cli -- parity map
cargo tree -p yunxi-agent-cli
git diff --check
```

Then run owned-source secret scanning, dependency keyword scanning, DeepSeek stream and non-stream smoke with `-ApiFile`, and a real interactive/one-shot DeepSeek response assertion. Do not print response bodies if they could contain secret echoes; assert only expected assistant content and leak status.

- [ ] **Step 2: Run bounded defect correction if the final gate fails**

Classify all failures from the single gate, patch the complete set, then rerun the affected command plus the full final gate once. Do not enter repeated single-point loops.

- [ ] **Step 3: Request code review and resolve critical/important findings**

Review the full diff against this plan and the design. Fix any credential leak, wrong default mode, JSON contamination, missing offline determinism, upstream dependency, or tag-policy issue, then re-run the relevant final evidence.

- [ ] **Step 4: Import the credential and install v1.2**

Run the import helper against the user-provided API file without outputting its content. Install release binaries with `scripts/install/install-yunxi.ps1 -AddToPath -SkipBuild`, then verify the installed binary reports `yunxi 1.2.0` and uses DeepSeek when user environment values are loaded into the verification process.

- [ ] **Step 5: Finalize evidence docs, commit, and tags**

Replace pending report fields with exact exit codes/counts. Commit all repository changes, create local lightweight `v1.2.0` at the final release commit, and verify local `v1.0.0`/`v1.1.0` object IDs are unchanged.

- [ ] **Step 6: Publish only through GitHub API**

Read remote master/ref/tree through REST API, create changed blobs/tree/commit, update `refs/heads/master`, create `refs/tags/v1.2.0`, and verify remote master plus all three version refs through REST API. Never emit the GitHub token.

- [ ] **Step 7: Sync index, log, and clean artifacts**

Run `codegraph sync "D:\YunXi Agent"`, append the detailed desktop development log, then run `cargo clean` and remove repository-root `.yunxi` and temporary smoke files. Confirm `target_exists=False` and `repo_yunxi_exists=False`.

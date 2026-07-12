# YunXi Agent v1.2.1 Interactive Provider Recovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep the interactive REPL alive after a failed turn and make DeepSeek HTTP 400/422 failures diagnosable and recoverable without leaking credentials.

**Architecture:** The CLI owns the turn-level recovery boundary, while the provider owns a single DeepSeek-specific metadata fallback and safe extraction of structured error details. Existing transport retry, runtime orchestration, session storage, and provider interfaces remain unchanged.

**Tech Stack:** Rust 2024, Tokio, Reqwest, Clap 4, Serde JSON, PowerShell smoke helpers, GitHub Git Data REST API.

## Global Constraints

- Write all implementation and tests before running any test, check, format, build, or live request.
- Run one unified verification gate after the complete slice is wired.
- Keep default CLI dependencies free of `vendor/codex-rs`, `codex-*`, and `yunxi-agent-codex`.
- Never print, log, commit, or upload API keys or GitHub tokens.
- Publish GitHub refs only through REST API; do not use `git push`, `git fetch`, or `git ls-remote`.
- Create `v1.2.1`; preserve `v1.0.0`, `v1.1.0`, and `v1.2.0` unchanged.

---

### Task 1: Provider Recovery Regression Tests

**Files:**
- Modify: `crates/yunxi-agent-provider/tests/provider_tests.rs`

**Interfaces:**
- Consumes: `ProviderTransport::send`, `ProviderTransportRequest`, `ProviderTransportResponse`, `OpenAiTransportProvider`.
- Produces: regression coverage for one metadata fallback, provider scoping, and safe error details.

- [ ] **Step 1: Add a sequence transport test fixture**

Implement a cloneable transport backed by `Arc<Mutex<VecDeque<ProviderTransportResponse>>>` and an
`Arc<Mutex<Vec<ProviderTransportRequest>>>`, returning responses in order and recording every request.

- [ ] **Step 2: Add DeepSeek metadata fallback tests**

Use responses `400 {"error":{"message":"unknown field metadata"}}` then a valid 200 completion.
Assert two requests, first contains `/metadata`, second does not, and both retain `/tools` and `/messages`.

- [ ] **Step 3: Add provider scoping and redaction tests**

Assert OpenAI-compatible 400 sends once. Assert DeepSeek 400/400 returns `unsupported_schema`, includes a
240-character-bounded structured detail, and excludes `sk-...`, `Bearer`, `Authorization`, and raw JSON.

- [ ] **Step 4: Do not run tests yet**

Record the test names for the final unified gate; the project hard constraint overrides intermediate TDD execution.

### Task 2: Interactive Turn Recovery Regression Test

**Files:**
- Modify: `crates/yunxi-agent-cli/tests/cli_tests.rs`

**Interfaces:**
- Consumes: compiled `yunxi` test binary and environment-driven provider configuration.
- Produces: end-to-end proof that a failed turn does not terminate the REPL.

- [ ] **Step 1: Add a local sequential HTTP fixture**

Bind `TcpListener` to `127.0.0.1:0`, accept three POST requests, read headers plus Content-Length body, and
return two HTTP 400 JSON bodies followed by one HTTP 200 chat completion body. Run with
`YUNXI_PROVIDER_STREAM=false`, a dummy `DEEPSEEK_API_KEY`, and the fixture base URL.

- [ ] **Step 2: Add the two-turn interactive test**

Pipe `first prompt`, `second prompt`, `/exit`. Assert output contains the first provider error, the second fixture
assistant response, and `YunXi interactive session ended.`; assert process exit success and no dummy key leak.

- [ ] **Step 3: Do not run tests yet**

Leave the new integration test unexecuted until all production code and docs are complete.

### Task 3: Provider Metadata Fallback and Safe Diagnostics

**Files:**
- Modify: `crates/yunxi-agent-provider/src/lib.rs`

**Interfaces:**
- Produces: `send_with_schema_fallback(&self, ProviderTransportRequest) -> AgentResult<ProviderTransportResponse>`.
- Produces: `provider_http_error(&ProviderConfig, &ProviderTransportResponse) -> AgentError`.

- [ ] **Step 1: Add the fallback boundary**

Call `send_with_retries` once. If the final response is 400/422, profile is DeepSeek, and `body.remove("metadata")`
returns a value, send the modified request once through `send_with_retries`; otherwise return the original response.

- [ ] **Step 2: Use fallback in complete and stream paths**

Replace both direct `send_with_retries` calls with the new helper. Pass the final response by reference to error mapping.

- [ ] **Step 3: Extract and sanitize structured details**

Parse allowed JSON fields, normalize whitespace, redact token-like fragments, truncate to 240 characters, and append
`detail=...` only when a non-empty safe detail exists. Never include the raw body or request headers.

### Task 4: REPL Turn-Level Error Boundary

**Files:**
- Modify: `crates/yunxi-agent-cli/src/interactive.rs`
- Modify: `crates/yunxi-agent-cli/src/main.rs`

**Interfaces:**
- Consumes: shared `redact_secret_fragments(&str) -> String` from the CLI host.
- Produces: failed normal prompts return control to `read_eval_loop` without changing session state.

- [ ] **Step 1: Expose the existing CLI redactor within the crate**

Change `redact_secret_fragments` to `pub(crate)` so interactive rendering uses the same safety policy as top-level errors.

- [ ] **Step 2: Catch only normal turn failures**

Replace `self.run_turn(input.to_string()).await?` with a match. On error print
`[error] {redacted alternate anyhow chain}` to stderr and continue. Do not call `update_session_state` or increment
turn count on failure.

- [ ] **Step 3: Preserve initialization and command semantics**

Leave `InteractiveSession::new`, stdin errors, `/resume` validation, `/exit`, `/quit`, and EOF behavior unchanged.

### Task 5: Version, Smoke, and Documentation

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `README.md`
- Modify: `scripts/provider/deepseek-live-smoke.ps1`
- Modify: `docs/extraction-status.md`
- Create: `docs/reports/2026-07-11-yunxi-agent-v1-2-1-interactive-provider-recovery-development-report.md`

**Interfaces:**
- Produces: version 1.2.1 and a live interactive prompt smoke mode that validates assistant output and REPL completion.

- [ ] **Step 1: Raise workspace-owned package versions to 1.2.1**

Update workspace version and lockfile package entries without changing dependency versions.

- [ ] **Step 2: Extend live smoke**

Add an interactive mode that pipes an exact marker prompt plus `/exit`, requires live/auto_live/deepseek banner fields,
requires the assistant marker and normal REPL ending, and performs the existing secret leak check.

- [ ] **Step 3: Document behavior and release policy**

Document turn-level recovery, one-shot DeepSeek metadata fallback, safe diagnostics, genuine interactive live coverage,
and immutable `v1.2.1` release requirements.

### Task 6: Unified Verification and Release

**Files:**
- Modify outside repo: `C:\Users\admin\Desktop\YunXi Agent开发日志.md`

**Interfaces:**
- Produces: verified local install, CodeGraph index, local/API commit and immutable v1.2.1 tag.

- [ ] **Step 1: Run the unified local gate**

Run `cargo fmt`, `cargo fmt -- --check`, `cargo test`, `cargo check --workspace`, release build, both version checks,
offline one-shot/interactive, CLI conflict/JSONL checks, parity map, dependency scan, staged secret scan, and
`git diff --check`.

- [ ] **Step 2: Run real DeepSeek gates**

Run stream, non-stream, auto one-shot, auto JSONL metadata, and a genuine auto interactive prompt. Verify all outputs
are secret-free and the interactive process ends only after `/exit`.

- [ ] **Step 3: Install and verify the release**

Install with `scripts/install/install-yunxi.ps1 -AddToPath -SkipBuild`, compare SHA-256, and repeat version plus live
interactive prompt checks against the installed binary.

- [ ] **Step 4: Publish immutable release through GitHub REST API**

Create final Git objects and annotated `v1.2.1`, update master without force, and independently read back ref/commit/tree/tag.
Verify old tag object SHAs are unchanged.

- [ ] **Step 5: Index, log, and clean**

Run CodeGraph sync, append the detailed timestamped development log, run `cargo clean`, remove root `.yunxi` and
temporary files, then verify a clean `master...origin/master` worktree.

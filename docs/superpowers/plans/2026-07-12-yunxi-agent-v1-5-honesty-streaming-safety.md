# YunXi Agent v1.5 Honesty Streaming Safety Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build YunXi Agent v1.5.0 so offline output, sandbox naming, fixture behavior, live streaming, and retry behavior match the audit report's honesty and safety requirements.

**Architecture:** Keep the existing YunXi-owned runtime/provider/CLI crates. Add narrow disclosure, fixture-policy, streaming, and retry boundaries instead of replacing the current Agent loop. Existing batch APIs remain as compatibility wrappers over the same event path.

**Tech Stack:** Rust 2024, Tokio, reqwest, serde, existing YunXi workspace crates.

## Global Constraints

- 当前项目目录：`D:\YunXi Agent`。
- 当前基线：`v1.4.0`，下一版本必须是 `v1.5.0`。
- 每个版本必须创建新 tag，旧 tag 不删除、不移动。
- GitHub 读写、发布、核验全部走 REST API；不使用 `git push`、`git fetch`、`git ls-remote`。
- 构建过程中不做零散测试；所有源码构建完成后统一验证。
- API key、PAT、credential 不打印、不写日志、不提交。
- Rust 编辑后执行 `cargo fmt`。
- 完成前执行完整统一验证门。
- 发布后执行 `cargo clean` 并确认 `target_exists=False`。
- `.codegraph/` 存在；理解或定位代码优先使用 CodeGraph。

---

## File Structure

- `crates/yunxi-agent-cli/src/interactive.rs`：交互启动、provider disclosure、Auto fallback 警告、交互统计。
- `crates/yunxi-agent-cli/src/render.rs`：assistant message、banner、sandbox/policy 文案、离线标注渲染。
- `crates/yunxi-agent-cli/src/commands.rs`：`/cost` 文案和 help。
- `crates/yunxi-agent-cli/tests/cli_tests.rs`：CLI 离线标注、`/cost`、fallback、fixture gating smoke。
- `crates/yunxi-agent-cli/tests/jsonl_tests.rs`：JSONL fixture gating 和 streaming event shape。
- `crates/yunxi-agent-provider/src/lib.rs`：transport headers、streaming transport、SSE 增量 drain、retry/backoff。
- `crates/yunxi-agent-provider/tests/provider_tests.rs`：Retry-After、网络错误重试、增量 SSE chunk 测试。
- `crates/yunxi-agent-runtime/src/lib.rs`：fixture policy、provider incremental event emit、runtime stream 汇总。
- `crates/yunxi-agent-runtime/tests/runtime_tests.rs`：fixture policy、provider event before completion、collected compatibility。
- `crates/yunxi-agent-sandbox/src/lib.rs`：对外 policy guard 命名和兼容序列化。
- `README.md`：v1.5.0 行为和安全边界。
- `docs/extraction-status.md`：当前自主化状态更新。
- `docs/reports/2026-07-12-yunxi-agent-v1-5-honesty-streaming-safety-development-report.md`：开发报告。
- `C:\Users\admin\Desktop\YunXi Agent开发日志.md`：任务日志。

## Task 1: Version And Offline Disclosure

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/yunxi-agent-cli/Cargo.toml`
- Modify: `crates/yunxi-agent-cli/src/interactive.rs`
- Modify: `crates/yunxi-agent-cli/src/render.rs`
- Modify: `crates/yunxi-agent-cli/src/commands.rs`
- Test: `crates/yunxi-agent-cli/tests/cli_tests.rs`

**Interfaces:**
- Produces: a CLI-visible disclosure state with three user-facing modes: live, forced offline, auto fallback offline.
- Produces: offline assistant messages prefixed with `[offline]`.
- Produces: `/cost` offline output equivalent to `n/a - offline, no model call`.

- [ ] **Step 1: Upgrade workspace and CLI package version to `1.5.0`.**

Update all YunXi package version fields that currently read `1.4.0`.

- [ ] **Step 2: Add a single disclosure helper for interactive mode.**

Create or extend a small helper in `interactive.rs` that can answer:

```rust
struct ProviderDisclosure {
    mode: &'static str,
    source: &'static str,
    is_offline: bool,
    auto_fallback_warning: Option<String>,
}
```

The helper must derive its values from the existing provider mode/source logic instead of duplicating unrelated provider bootstrap code.

- [ ] **Step 3: Render offline messages inline.**

In `render.rs`, ensure assistant `AgentEvent::Message` content receives `[offline] ` when the active disclosure says `is_offline=true`.

- [ ] **Step 4: Show Auto fallback warning once per REPL startup.**

In `interactive.rs`, print the warning before the first prompt when provider mode is auto and the resolved provider source is offline.

- [ ] **Step 5: Make `/cost` honest in offline mode.**

Change the `/cost` handler so offline mode prints `n/a - offline, no model call` for last-turn and session usage instead of an ambiguous unavailable state.

- [ ] **Step 6: Add deferred verification coverage.**

Add tests that will be run only in the final verification pass:

```rust
#[test]
fn offline_one_shot_marks_assistant_output() {
    // Run yunxi with forced offline provider and assert stdout contains "[offline]".
}

#[test]
fn offline_cost_says_no_model_call() {
    // Run interactive /cost under forced offline provider and assert the n/a offline text.
}

#[test]
fn auto_fallback_warns_when_no_credentials_are_available() {
    // Run interactive with env credentials removed and assert a fallback warning is printed.
}
```

## Task 2: Sandbox Honesty Rename

**Files:**
- Modify: `crates/yunxi-agent-sandbox/src/lib.rs`
- Modify: `crates/yunxi-agent-cli/src/render.rs`
- Modify: `README.md`
- Modify: `docs/extraction-status.md`
- Test: `crates/yunxi-agent-cli/tests/cli_tests.rs`

**Interfaces:**
- Produces: user-facing text that says policy guard/advisor, not OS-level sandbox.
- Preserves: existing policy evaluation decisions and old serialized session compatibility.

- [ ] **Step 1: Add user-facing capability labels.**

Add a method near `SandboxBackend`:

```rust
impl SandboxBackend {
    pub fn user_facing_label(self) -> &'static str {
        match self {
            SandboxBackend::DangerFullAccess => "policy bypass: danger-full-access",
            SandboxBackend::None => "no policy guard",
            _ => "policy guard: advisory only, no OS isolation",
        }
    }
}
```

- [ ] **Step 2: Keep serialized compatibility.**

If enum variants are renamed internally, add serde aliases for `windows_restricted_token`, `linux_landlock`, and `workspace_guard`. If internal names remain unchanged, make sure all CLI/docs use `user_facing_label()` instead of variant debug formatting.

- [ ] **Step 3: Update CLI rendering.**

Where policy/sandbox traces are printed, render the user-facing label and include the explicit phrase `no OS isolation`.

- [ ] **Step 4: Update docs.**

README and extraction status must say v1.5 uses policy evaluation and human approval, not OS-enforced sandboxing.

- [ ] **Step 5: Add deferred verification coverage.**

Add a CLI smoke assertion that normal output does not contain misleading `WindowsRestrictedToken` or `LinuxLandlock` wording unless it is inside a compatibility/debug-only value.

## Task 3: Runtime Fixture Isolation

**Files:**
- Modify: `crates/yunxi-agent-runtime/src/lib.rs`
- Test: `crates/yunxi-agent-runtime/tests/runtime_tests.rs`
- Test: `crates/yunxi-agent-cli/tests/jsonl_tests.rs`
- Modify: `README.md`
- Modify: `docs/extraction-status.md`

**Interfaces:**
- Produces: `RuntimeFixturePolicy::Disabled` as the default product behavior.
- Produces: explicit fixture enablement for tests and compatibility smoke only.
- Preserves: ability to run existing stage fixture tests after opting in.

- [ ] **Step 1: Introduce fixture policy.**

Add a small runtime-level policy:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RuntimeFixturePolicy {
    Disabled,
    Explicit,
}
```

Store it on `YunXiRuntimeBackend` with default `Disabled`.

- [ ] **Step 2: Add explicit opt-in constructor.**

Add a crate-visible or test-visible builder method:

```rust
pub fn with_runtime_fixtures_enabled(mut self) -> Self {
    self.fixture_policy = RuntimeFixturePolicy::Explicit;
    self
}
```

- [ ] **Step 3: Gate `stage 4x` branches.**

Wrap `stage 4k`, `stage 4l`, and `stage 4m` prompt checks behind `self.fixture_policy == RuntimeFixturePolicy::Explicit`.

- [ ] **Step 4: Mark fixture events.**

Any fixture path that remains must emit metadata containing `fixture_mode=true`.

- [ ] **Step 5: Add deferred verification coverage.**

Add tests:

```rust
#[tokio::test]
async fn stage_prompt_does_not_trigger_fixture_by_default() {
    // Prompt includes "stage 4m real parity fixture" but runtime uses default fixture policy.
    // Assert normal provider path is used.
}

#[tokio::test]
async fn stage_prompt_runs_when_fixture_policy_is_explicit() {
    // Enable fixtures with with_runtime_fixtures_enabled().
    // Assert fixture_mode=true appears in emitted events.
}
```

## Task 4: True Provider Streaming

**Files:**
- Modify: `crates/yunxi-agent-provider/src/lib.rs`
- Modify: `crates/yunxi-agent-runtime/src/lib.rs`
- Modify: `crates/yunxi-agent-cli/src/interactive.rs`
- Test: `crates/yunxi-agent-provider/tests/provider_tests.rs`
- Test: `crates/yunxi-agent-runtime/tests/runtime_tests.rs`

**Interfaces:**
- Produces: provider network streaming based on `bytes_stream()`.
- Produces: runtime events emitted while chunks arrive.
- Preserves: batch `ProviderStream::from_events` compatibility for callers that collect.

- [ ] **Step 1: Add transport streaming support.**

Extend provider transport with a streaming path that returns status, headers, and byte chunks. The reqwest implementation must call `response.bytes_stream()`.

- [ ] **Step 2: Add incremental event draining.**

Extend `OpenAiStreamAccumulator` so pushing a chunk can return newly completed `StreamEvent` values without waiting for final finish.

- [ ] **Step 3: Add provider incremental stream API.**

Add an API used by runtime that can call an event callback/sink as each parsed event appears, then returns the final collected `ProviderResponse`.

- [ ] **Step 4: Wire runtime provider turn to incremental events.**

In `run_turn_with_control`, use the incremental provider path when stream mode is enabled. Emit stream events immediately and still collect final message/tool calls/usage for the existing tool loop.

- [ ] **Step 5: Preserve non-stream fallback.**

If `ProviderConfig.stream=false`, keep existing `complete()` path.

- [ ] **Step 6: Add deferred verification coverage.**

Add a fake streaming transport test that sends two chunks with a controlled delay. Assert runtime receives the first `ItemDelta` before the final response completes.

## Task 5: Retry-After And Transport Retry

**Files:**
- Modify: `crates/yunxi-agent-provider/src/lib.rs`
- Test: `crates/yunxi-agent-provider/tests/provider_tests.rs`

**Interfaces:**
- Produces: retry budget shared by HTTP status and transport errors.
- Produces: Retry-After parsing for integer seconds and HTTP-date.
- Produces: bounded exponential backoff.

- [ ] **Step 1: Store response headers.**

Extend `ProviderTransportResponse` to include headers or a redacted retry header view. Populate it in `ReqwestProviderTransport`.

- [ ] **Step 2: Add retry delay calculation.**

Implement a helper:

```rust
fn retry_delay_for_response(
    policy: &ProviderRetryPolicy,
    response: &ProviderTransportResponse,
    attempt: usize,
) -> Option<Duration>
```

The helper uses `Retry-After` first, then bounded exponential backoff.

- [ ] **Step 3: Retry network and timeout errors.**

Change `send_with_retries` so transport errors classified as network or timeout consume retry budget instead of immediately returning.

- [ ] **Step 4: Inject test sleeper.**

Keep product code sleeping with `tokio::time::sleep`, but allow tests to avoid real waits through a small policy/test hook.

- [ ] **Step 5: Add deferred verification coverage.**

Add tests for:

```rust
#[tokio::test]
async fn retries_429_after_retry_after_header() {}

#[tokio::test]
async fn retries_timeout_transport_error() {}

#[tokio::test]
async fn does_not_retry_non_retryable_400() {}
```

## Task 6: Documentation, Unified Verification, Release, And Log

**Files:**
- Modify: `README.md`
- Modify: `docs/extraction-status.md`
- Modify: `docs/reports/2026-07-12-yunxi-agent-v1-5-honesty-streaming-safety-development-report.md`
- Modify: `C:\Users\admin\Desktop\YunXi Agent开发日志.md`

**Interfaces:**
- Produces: v1.5.0 docs and release notes.
- Produces: a new commit and tag `v1.5.0`.
- Produces: GitHub REST API publication and local cleanup.

- [ ] **Step 1: Update docs to describe v1.5 behavior.**

README and extraction status must mention offline marking, policy guard wording, fixture opt-in, true streaming, and retry/backoff.

- [ ] **Step 2: Run the unified verification gate after all build work is complete.**

Run:

```powershell
cargo fmt
cargo fmt -- --check
cargo test
cargo check --workspace
cargo build -p yunxi-agent-cli --release --bins
target\release\yunxi.exe --version
target\release\yunxi-agent-cli.exe --version
git diff --check
codegraph sync "D:\YunXi Agent"
```

Expected version output:

```text
yunxi 1.5.0
```

- [ ] **Step 3: Run smoke checks.**

Run offline, fixture-gating, DeepSeek live non-stream, DeepSeek live stream, retry fixture, dependency scan, and secret scan smokes described in the v1.5 report. Do not print API keys or PATs.

- [ ] **Step 4: Commit and tag locally.**

Create a normal commit and annotated tag:

```powershell
git add .
git commit -m "Release YunXi Agent v1.5.0 honesty streaming safety"
git tag -a v1.5.0 -m "YunXi Agent v1.5.0"
```

- [ ] **Step 5: Publish through GitHub REST API only.**

Use the API token from the approved local credential file without printing it. Update remote `master`, create the annotated tag object/ref for `v1.5.0`, and verify all old tags remain unmoved.

- [ ] **Step 6: Clean build artifacts.**

Run:

```powershell
cargo clean
Test-Path -LiteralPath 'D:\YunXi Agent\target'
```

Expected:

```text
False
```

- [ ] **Step 7: Append the desktop development log.**

Record changed files, validation results, REST API publication result, tag, cleanup result, and final state. Do not record secrets.

# YunXi Agent v1.6 Policy Guard Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build YunXi Agent v1.6.0 so the default shell/tool path honors YunXi policy, blocks common workspace escapes, fixes streaming retry correctness, and closes v1.5 audit gaps without claiming OS-level sandboxing.

**Architecture:** Keep the existing YunXi-owned crates. Move policy from trace/advisory text into the command construction path, add conservative process-internal guards for common file targets, and keep OS-level sandboxing explicitly out of scope for this version.

**Tech Stack:** Rust 2024, Tokio, reqwest streaming, existing YunXi workspace crates, GitHub REST API release flow.

## Global Constraints

- 当前项目目录：`D:\YunXi Agent`。
- 当前基线：`v1.5.0`，下一版本必须是 `v1.6.0`。
- 每个版本必须创建新 tag，旧 tag 不删除、不移动。
- GitHub 读写、发布、核验全部走 REST API；不使用 `git push`、`git fetch`、`git ls-remote`。
- 构建过程中不做零散测试；所有源码构建完成后统一验证。
- API key、PAT、credential 不打印、不写日志、不提交。
- Rust 编辑后执行 `cargo fmt`。
- 完成前执行完整统一验证门。
- 发布后执行 `cargo clean` 并确认 `target_exists=False`。
- `.codegraph/` 存在；理解或定位代码优先使用 CodeGraph。
- v1.6 不声称 OS-level sandbox；目标是 process-internal policy guard hardening。

---

## File Structure

- `Cargo.toml` / `Cargo.lock`：版本升级到 `1.6.0`，如需新增轻量解析依赖必须先证明必要。
- `crates/yunxi-agent-cli/src/main.rs`：one-shot/resume auto fallback warning。
- `crates/yunxi-agent-cli/src/provider_mode.rs`：warning helper 复用。
- `crates/yunxi-agent-cli/tests/cli_tests.rs`：fallback warning 和版本断言。
- `crates/yunxi-agent-cli/tests/jsonl_tests.rs`：JSONL warning 结构不污染测试。
- `crates/yunxi-agent-exec/src/lib.rs`：保留 `ExecCommand::shell(policy)`，限制 `observed_shell` 用途或标注 trusted-only。
- `crates/yunxi-agent-provider/src/lib.rs`：streaming retry partial-body guard。
- `crates/yunxi-agent-provider/tests/provider_tests.rs`：streaming retry 事件不重复。
- `crates/yunxi-agent-sandbox/src/lib.rs`：token-aware risk classifier 和 workspace write target guard。
- `crates/yunxi-agent-tools/src/lib.rs`：`run_shell` 使用 request policy，不再默认 `observed_shell`。
- `crates/yunxi-agent-tools/tests/tool_tests.rs`：read-only/workspace-write/danger-full-access regression。
- `README.md` / `docs/extraction-status.md` / v1.6 report：文档同步。
- `C:\Users\admin\Desktop\YunXi Agent开发日志.md`：任务结束日志。

## Task 1: Version And Fallback Warning Parity

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/yunxi-agent-cli/src/main.rs`
- Modify: `crates/yunxi-agent-cli/src/provider_mode.rs`
- Test: `crates/yunxi-agent-cli/tests/cli_tests.rs`
- Test: `crates/yunxi-agent-cli/tests/jsonl_tests.rs`

**Interfaces:**
- Produces: `ProviderSelection::auto_fallback_warning()`.
- Produces: one-shot/resume warning behavior matching interactive mode.

- [ ] **Step 1: Upgrade workspace version to `1.6.0`.**

Update workspace package version and CLI assertions that currently read `1.5.0`.

- [ ] **Step 2: Add a one-shot warning render helper.**

Create a helper in `main.rs`:

```rust
fn print_provider_selection_warning(selection: &provider_mode::ProviderSelection, json: bool, jsonl: bool) -> Result<()> {
    if let Some(warning) = selection.auto_fallback_warning() {
        if jsonl {
            println!("{}", to_jsonl_line(&RuntimeEvent::Warning {
                thread_id: Some(ThreadId("cli-thread".to_string())),
                turn_id: Some(TurnId("cli-turn".to_string())),
                message: warning.trim_start_matches("[warning] ").to_string(),
            })?);
        } else if json {
            eprintln!("{warning}");
        } else {
            println!("{warning}");
        }
    }
    Ok(())
}
```

- [ ] **Step 3: Call the helper before one-shot and resume execution output.**

Call it after `provider_mode.resolve(...)` and before `print_run_result(...)` in normal one-shot and `sessions resume`.

- [ ] **Step 4: Add deferred tests.**

Add assertions that no-key one-shot plain output includes warning and `[offline]`; JSON output keeps stdout valid JSON and warning on stderr; JSONL output includes a warning event.

## Task 2: Shell Execution Honors ToolPolicy

**Files:**
- Modify: `crates/yunxi-agent-tools/src/lib.rs`
- Modify: `crates/yunxi-agent-exec/src/lib.rs`
- Test: `crates/yunxi-agent-tools/tests/tool_tests.rs`

**Interfaces:**
- Consumes: `ToolRequest.policy.execution_policy`.
- Produces: `ExecCommand::shell(cwd, command, execution_policy)` in default shell execution.

- [ ] **Step 1: Change `run_shell` to accept `ToolPolicy`.**

Change the private function signature to:

```rust
async fn run_shell(
    id: Option<String>,
    cwd: PathBuf,
    command: String,
    policy: ToolPolicy,
    cancellation_token: yunxi_agent_core::AgentCancellationToken,
) -> AgentResult<ToolResponse>
```

- [ ] **Step 2: Replace observed shell construction.**

Inside `run_shell`, replace:

```rust
let exec_command = ExecCommand::observed_shell(cwd.clone(), command).with_id(id.clone());
```

with:

```rust
let exec_command =
    ExecCommand::shell(cwd.clone(), command, policy.execution_policy.clone()).with_id(id.clone());
```

- [ ] **Step 3: Pass request policy from dispatch.**

Where `run_shell` is called for `ToolRequestKind::Shell`, pass `request.policy.clone()`.

- [ ] **Step 4: Add regression tests.**

Add a test where read-only mode shell write is declined before spawn, and another where danger-full-access still executes an innocuous command.

## Task 3: Workspace Write Target Guard

**Files:**
- Modify: `crates/yunxi-agent-sandbox/src/lib.rs`
- Modify: `crates/yunxi-agent-tools/src/lib.rs`
- Test: `crates/yunxi-agent-sandbox/src/lib.rs`
- Test: `crates/yunxi-agent-tools/tests/tool_tests.rs`

**Interfaces:**
- Produces: `ExecutionPolicy::evaluate(...)` blocking obvious workspace-outside write targets.
- Produces: command diagnostics suitable for policy trace.

- [ ] **Step 1: Add a target extraction type.**

Add:

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandTarget {
    pub raw: String,
    pub kind: CommandTargetKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandTargetKind {
    Read,
    Write,
    Delete,
}
```

- [ ] **Step 2: Implement conservative target extraction.**

Add `extract_command_targets(command: &str) -> Vec<CommandTarget>` covering `>`, `>>`, `out-file`, `set-content`, `add-content`, `new-item`, `copy-item`, `move-item`, `remove-item`, `cp`, `mv`, `rm`, `del`, `erase`, `rmdir`, `mkdir`, and `touch`.

- [ ] **Step 3: Add workspace containment check.**

For write/delete targets in `WorkspaceWrite`, reject absolute paths outside workspace and paths that normalize outside workspace through `..`.

- [ ] **Step 4: Add tests.**

Cover:

```rust
assert_blocked("echo hi > C:\\Temp\\outside.txt");
assert_blocked("echo hi > ..\\outside.txt");
assert_allowed("echo hi > inside.txt");
```

Use platform-specific path literals where needed.

## Task 4: Token-Aware Command Risk

**Files:**
- Modify: `crates/yunxi-agent-sandbox/src/lib.rs`
- Test: `crates/yunxi-agent-sandbox/src/lib.rs`

**Interfaces:**
- Produces: `CommandRisk::classify` robust against common whitespace/argument variants.

- [ ] **Step 1: Add shell-ish tokenizer.**

Add a small tokenizer that lowercases command words, preserves quoted text as one token, and collapses repeated whitespace.

- [ ] **Step 2: Classify by command verb and args.**

Detect destructive cases:

```text
rm -rf
rm -r -f
remove-item -recurse -force
del /s
rmdir /s
```

Detect network/process/credential/write/read cases by tokens rather than substring-only.

- [ ] **Step 3: Preserve backwards compatibility.**

Keep old simple needles as a fallback for unusual commands, but make tokenized matches authoritative for known risky forms.

- [ ] **Step 4: Add tests.**

Cover `rm  -rf`, `rm -r -f`, `Remove-Item -Recurse -Force`, `curl https://example.com`, `.env`, and a harmless echo.

## Task 5: Streaming Retry Partial-Body Guard

**Files:**
- Modify: `crates/yunxi-agent-provider/src/lib.rs`
- Test: `crates/yunxi-agent-provider/tests/provider_tests.rs`

**Interfaces:**
- Produces: no retry after any streaming body byte/event has reached the sink.

- [ ] **Step 1: Add counting sink wrapper.**

Wrap the stream sink in a small struct that increments `bytes_seen` before forwarding chunks.

- [ ] **Step 2: Gate retry on bytes_seen.**

In `send_streaming_with_retries`, allow retry only when `bytes_seen == 0`.

- [ ] **Step 3: Gate DeepSeek schema fallback.**

Only run streaming schema fallback when no body bytes were seen.

- [ ] **Step 4: Add tests.**

Add one test where the first streaming attempt returns 429 before body and succeeds on retry, and one where a transport error after first chunk does not retry and does not duplicate the first delta.

## Task 6: Cross-Platform Compile Fix

**Files:**
- Modify: `crates/yunxi-agent-tools/src/lib.rs`

**Interfaces:**
- Produces: no dead non-Windows function that references missing imports.

- [ ] **Step 1: Remove or fix `platform_shell`.**

Prefer deleting the unused `#[cfg(not(windows))] fn platform_shell(...)` from `yunxi-agent-tools/src/lib.rs`; if it is still needed, add the correct import and tests.

- [ ] **Step 2: Run target check in final gate.**

Run `cargo check --workspace --target x86_64-unknown-linux-gnu` after all construction is complete.

## Task 7: Documentation, Unified Verification, Release, And Log

**Files:**
- Modify: `README.md`
- Modify: `docs/extraction-status.md`
- Modify: `docs/reports/2026-07-12-yunxi-agent-v1-6-policy-guard-hardening-development-report.md`
- Modify: `C:\Users\admin\Desktop\YunXi Agent开发日志.md`

**Interfaces:**
- Produces: v1.6.0 docs, release notes, commit, tag, GitHub REST API publication, and cleanup.

- [ ] **Step 1: Update docs.**

Document that v1.6 hardens process-internal policy guard but still does not provide OS-level sandbox isolation.

- [ ] **Step 2: Run unified verification gate.**

Run every command listed in the v1.6 development report.

- [ ] **Step 3: Commit and tag.**

Create release commit and annotated tag `v1.6.0`.

- [ ] **Step 4: Publish through GitHub REST API only.**

Update remote `master`, create remote `v1.6.0`, and verify old tags remain unchanged.

- [ ] **Step 5: Clean build artifacts and log.**

Run `cargo clean`, confirm `target_exists=False`, then append the desktop development log.

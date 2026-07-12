# YunXi Agent v1.7 Terminal TUI Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build YunXi Agent v1.7.0 with a safe terminal TUI foundation, line editing, preserved automation output, and symlink escape regression coverage.

**Architecture:** Keep the existing YunXi-owned runtime/provider/tools/storage crates unchanged unless tests reveal a regression. Add a CLI-only terminal host layer that selects between plain and TUI rendering, with renderer/input abstractions so JSON/JSONL and scripts stay stable. TUI state is tested with `ratatui::backend::TestBackend`; shell safety remains process-internal and does not claim OS isolation.

**Tech Stack:** Rust 2024, Tokio, clap, ratatui, crossterm, reedline, existing YunXi workspace crates, GitHub REST API release flow.

## Global Constraints

- 当前项目目录：`D:\YunXi Agent`。
- 当前基线：`v1.6.0`，下一版本必须是 `v1.7.0`。
- 每个版本必须创建新 tag，旧 tag 不删除、不移动。
- GitHub 读写、发布、核验全部走 REST API；不使用 `git push`、`git fetch`、`git ls-remote`。
- 构建过程中不做零散测试；所有源码构建完成后统一验证。
- API key、PAT、credential 不打印、不写日志、不提交。
- Rust 编辑后执行 `cargo fmt`。
- 完成前执行完整统一验证门。
- 发布后执行 `cargo clean` 并确认 `target_exists=False`。
- `.codegraph/` 存在；理解或定位代码优先使用 CodeGraph。
- v1.7 不声称 OS-level sandbox；目标是 terminal UX foundation + safety regression coverage。
- `--json`、`--jsonl`、one-shot、sessions 子命令不得被 TUI 输出污染。

---

## File Structure

- `Cargo.toml` / `Cargo.lock`：版本升级到 `1.7.0`，workspace dependency 增加终端依赖。
- `crates/yunxi-agent-cli/Cargo.toml`：增加 `ratatui`、`crossterm`、`reedline`。
- `crates/yunxi-agent-cli/src/main.rs`：新增 `--tui` / `--no-tui` flag 和 interactive mode selection。
- `crates/yunxi-agent-cli/src/terminal_mode.rs`：新增 terminal/TUI 模式选择。
- `crates/yunxi-agent-cli/src/input.rs`：新增 plain/reedline 输入抽象。
- `crates/yunxi-agent-cli/src/interactive.rs`：接入 input/render abstraction，保留 plain fallback。
- `crates/yunxi-agent-cli/src/render.rs`：保留现有 event rendering，抽出 plain renderer helper。
- `crates/yunxi-agent-cli/src/tui/mod.rs`：TUI module facade。
- `crates/yunxi-agent-cli/src/tui/app.rs`：TUI app state，event log，status state。
- `crates/yunxi-agent-cli/src/tui/render.rs`：ratatui layout/render functions。
- `crates/yunxi-agent-cli/tests/cli_tests.rs`：CLI mode selection、plain fallback、版本断言。
- `crates/yunxi-agent-cli/tests/jsonl_tests.rs`：JSONL 不被 TUI 污染。
- `crates/yunxi-agent-sandbox/src/lib.rs`：symlink target guard unit test。
- `crates/yunxi-agent-tools/tests/tool_tests.rs`：symlink escape integration test。
- `README.md` / `docs/extraction-status.md` / v1.7 report：文档同步。
- `C:\Users\admin\Desktop\YunXi Agent开发日志.md`：任务结束日志。

## Task 1: Version, Dependencies, And Terminal Mode Selection

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/yunxi-agent-cli/Cargo.toml`
- Modify: `crates/yunxi-agent-cli/src/main.rs`
- Create: `crates/yunxi-agent-cli/src/terminal_mode.rs`
- Test: `crates/yunxi-agent-cli/tests/cli_tests.rs`

**Interfaces:**
- Produces: `TerminalModeRequest { auto, tui, no_tui }`
- Produces: `ResolvedTerminalMode::{Plain, Tui}`
- Consumes: stdin/stdout terminal detection and CLI flags.

- [ ] **Step 1: Upgrade workspace version to `1.7.0`.**

Update `[workspace.package].version` and version assertions currently expecting `yunxi 1.6.0`.

- [ ] **Step 2: Add terminal dependencies.**

Add workspace dependencies:

```toml
crossterm = "0.28"
ratatui = "0.29"
reedline = "0.36"
```

Add them to `crates/yunxi-agent-cli/Cargo.toml`.

- [ ] **Step 3: Add CLI flags.**

Add to `Cli` in `main.rs`:

```rust
#[arg(long, global = true, conflicts_with = "no_tui")]
tui: bool,

#[arg(long = "no-tui", global = true)]
no_tui: bool,
```

- [ ] **Step 4: Create terminal mode resolver.**

Create `terminal_mode.rs`:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ResolvedTerminalMode {
    Plain,
    Tui,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TerminalModeRequest {
    pub tui: bool,
    pub no_tui: bool,
    pub stdin_is_terminal: bool,
    pub stdout_is_terminal: bool,
}

impl TerminalModeRequest {
    pub(crate) fn resolve(self) -> ResolvedTerminalMode {
        if self.no_tui {
            return ResolvedTerminalMode::Plain;
        }
        if self.tui && self.stdin_is_terminal && self.stdout_is_terminal {
            return ResolvedTerminalMode::Tui;
        }
        if self.stdin_is_terminal && self.stdout_is_terminal {
            return ResolvedTerminalMode::Tui;
        }
        ResolvedTerminalMode::Plain
    }
}
```

- [ ] **Step 5: Add mode selection tests.**

Add tests for auto terminal, piped stdin, forced `--no-tui`, and forced `--tui` non-terminal fallback.

## Task 2: Input Abstraction And Reedline Terminal Input

**Files:**
- Create: `crates/yunxi-agent-cli/src/input.rs`
- Modify: `crates/yunxi-agent-cli/src/interactive.rs`
- Test: `crates/yunxi-agent-cli/tests/cli_tests.rs`

**Interfaces:**
- Produces: `InteractiveInput` trait.
- Produces: `PlainInput<R: BufRead>` and `ReedlineInput`.
- Consumes: existing `parse_interactive_command`.

- [ ] **Step 1: Define input trait.**

Create:

```rust
pub(crate) trait InteractiveInput {
    fn read_prompt(&mut self, prompt: &str) -> anyhow::Result<Option<String>>;
    fn read_response(&mut self, prompt: &str) -> anyhow::Result<Option<String>>;
}
```

- [ ] **Step 2: Implement plain input.**

Move existing `BufRead::read_line` behavior behind `PlainInput`, preserving pipe behavior and existing tests.

- [ ] **Step 3: Implement Reedline input.**

Use `reedline::Reedline` for terminal prompts, loading/saving workspace history under `.yunxi/history`.

- [ ] **Step 4: Wire interactive loop through the trait.**

Update `InteractiveSession::read_eval_loop` so prompt reads use `InteractiveInput::read_prompt`.

- [ ] **Step 5: Keep approval/user input compatible.**

For v1.7, approval and model-requested user input may continue using plain response prompts through `read_response`, not a TUI modal.

## Task 3: Renderer Facade And Plain Renderer Preservation

**Files:**
- Modify: `crates/yunxi-agent-cli/src/render.rs`
- Modify: `crates/yunxi-agent-cli/src/interactive.rs`
- Test: `crates/yunxi-agent-cli/tests/cli_tests.rs`

**Interfaces:**
- Produces: `InteractiveRenderer`.
- Produces: `PlainInteractiveRenderer`.
- Consumes: `AgentEvent`, `RenderState`, `InteractiveBanner`.

- [ ] **Step 1: Define renderer trait.**

Add:

```rust
pub(crate) trait InteractiveRenderer {
    fn banner(&mut self, banner: &InteractiveBanner) -> anyhow::Result<()>;
    fn warning(&mut self, message: &str) -> anyhow::Result<()>;
    fn event(&mut self, event: &AgentEvent, state: &mut RenderState) -> anyhow::Result<()>;
    fn error(&mut self, message: &str) -> anyhow::Result<()>;
}
```

- [ ] **Step 2: Implement plain renderer using existing functions.**

`PlainInteractiveRenderer` should delegate to `print_banner` and `render_agent_event`.

- [ ] **Step 3: Update interactive session to use renderer.**

Replace direct `println!` rendering in turn event handling with renderer calls where practical.

- [ ] **Step 4: Preserve existing plain tests.**

Existing tests for interactive `/session`, `/status`, provider errors, and offline output must still pass.

## Task 4: TUI App State And TestBackend Rendering

**Files:**
- Create: `crates/yunxi-agent-cli/src/tui/mod.rs`
- Create: `crates/yunxi-agent-cli/src/tui/app.rs`
- Create: `crates/yunxi-agent-cli/src/tui/render.rs`
- Modify: `crates/yunxi-agent-cli/src/main.rs`
- Test: module unit tests under `tui`

**Interfaces:**
- Produces: `TuiApp`.
- Produces: `TuiEventLine`.
- Produces: `render_tui_frame(frame, app)`.

- [ ] **Step 1: Define TUI app state.**

Include version, cwd, provider, model, sandbox, approval, input buffer, event lines, and scroll offset.

- [ ] **Step 2: Map `AgentEvent` to compact TUI lines.**

Messages, reasoning, command/tool/MCP, warnings/errors, and completion events must each have deterministic labels.

- [ ] **Step 3: Render with `ratatui::layout`.**

Use three vertical areas: header, log, input. Keep text compact and avoid decorative card-heavy layout.

- [ ] **Step 4: Add TestBackend tests.**

Use `ratatui::backend::TestBackend` to assert header contains `YunXi`, log contains an event line, and input area contains prompt text.

## Task 5: TUI Host Integration With Plain Fallback

**Files:**
- Modify: `crates/yunxi-agent-cli/src/interactive.rs`
- Modify: `crates/yunxi-agent-cli/src/tui/mod.rs`
- Test: `crates/yunxi-agent-cli/tests/cli_tests.rs`

**Interfaces:**
- Consumes: `ResolvedTerminalMode`.
- Produces: terminal guard that restores raw mode and alternate screen on drop.

- [ ] **Step 1: Add TUI terminal guard.**

Use `crossterm` to enter alternate screen/raw mode and restore on drop.

- [ ] **Step 2: Run TUI host only for terminal mode.**

If mode is `Plain`, keep current behavior.

- [ ] **Step 3: Ensure TUI does not run in tests with piped stdin.**

Integration tests that use `.write_stdin(...)` must continue seeing plain output.

- [ ] **Step 4: Add `--no-tui` smoke.**

Run `yunxi --offline --no-tui` with stdin `/exit` and assert plain banner/exit output.

## Task 6: Symlink Escape Regression Coverage

**Files:**
- Modify: `crates/yunxi-agent-sandbox/src/lib.rs`
- Modify: `crates/yunxi-agent-tools/tests/tool_tests.rs`

**Interfaces:**
- Consumes: existing `target_within_workspace` and `WorkspaceWrite` policy.
- Produces: regression tests proving symlink targets outside workspace are declined where symlinks are available.

- [ ] **Step 1: Add symlink helper in tests.**

Create platform-specific helper:

```rust
#[cfg(windows)]
fn create_dir_symlink(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(src, dst)
}

#[cfg(unix)]
fn create_dir_symlink(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(src, dst)
}
```

- [ ] **Step 2: Unit-test target containment with symlink.**

If symlink creation succeeds, assert a workspace symlink pointing outside is rejected.

- [ ] **Step 3: Integration-test shell tool decline.**

If symlink creation succeeds, run shell write through tool runtime and assert declined plus outside file does not exist.

- [ ] **Step 4: Skip explicitly when OS denies symlink.**

Use `eprintln!` with a clear skip reason and return from the test; do not mark it as a pass for environments where symlink behavior cannot be exercised.

## Task 7: Documentation, Unified Verification, Release, And Log

**Files:**
- Modify: `README.md`
- Modify: `docs/extraction-status.md`
- Modify: `docs/reports/2026-07-12-yunxi-agent-v1-7-terminal-tui-foundation-development-report.md`
- Modify: `C:\Users\admin\Desktop\YunXi Agent开发日志.md`

**Interfaces:**
- Produces: v1.7.0 docs, release notes, commit, tag, GitHub REST API publication, and cleanup.

- [ ] **Step 1: Update docs.**

Document TUI default conditions, `--no-tui`, line editing, and continued no-OS-sandbox boundary.

- [ ] **Step 2: Run unified verification gate.**

Run every command listed in the v1.7 development report.

- [ ] **Step 3: Commit and tag.**

Create release commit and annotated tag `v1.7.0`.

- [ ] **Step 4: Publish through GitHub REST API only.**

Update remote `master`, create remote `v1.7.0`, and verify old tags remain unchanged.

- [ ] **Step 5: Clean build artifacts and log.**

Run `cargo clean`, confirm `target_exists=False`, then append the desktop development log.

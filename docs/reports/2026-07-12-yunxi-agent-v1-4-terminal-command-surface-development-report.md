# YunXi Agent v1.4 Terminal Command Surface Development Report

生成时间：2026-07-12 +08:00

## 背景

v1.3.0 已经把 YunXi Agent 从批处理式 runner 推进到真实交互式终端 Agent：交互 turn 可以实时接收 runtime 事件，危险工具可以在终端中途审批，`request_user_input` 可以回填，Ctrl+C 可以向 runtime/tools/exec 传播 cancellation。

下一阶段不再重复打磨单个 provider bug，而是补齐终端 Agent 日常使用中最容易暴露的命令面缺口。Codex CLI 的核心体验不只是能调用模型，还要让用户在 REPL 内随时看到当前会话、可用工具、MCP 配置、token usage 和最近一轮执行状态。v1.4 目标就是让这些信息成为 YunXi 自主 CLI 的一等能力。

## v1.4 目标

- 将版本升级到 `1.4.0`，发布新 tag `v1.4.0`，旧 tag 不删除、不移动。
- 在交互 CLI 中新增 Codex 类状态命令：
  - `/tools`：展示固定工具和 workspace 动态工具。
  - `/mcp`：展示 workspace MCP 配置、启用状态和 fixture seed 状态。
  - `/cost`：展示最近一轮和当前 REPL 累计 token usage。
  - `/status`：展示会话、provider、最近一轮状态、事件统计和工具/MCP 摘要。
- 让交互 session 记录最近一轮状态摘要，包括 runtime event 数、tool/command/MCP/file/approval/cancel 计数和 usage。
- 文档更新到 v1.4.0，明确终端状态命令和能力边界。
- 保持默认 `yunxi` 运行链路不依赖 `vendor/codex-rs`、`codex-*` 或 `yunxi-agent-codex`。
- API key、PAT 和 credential 文件内容不打印、不写日志、不提交。

## 非目标

- 不重写 provider 主链，不替换 DeepSeek 接入。
- 不引入 TUI、桌面端、云任务、updater、doctor、completion 或 marketplace。
- 不在构建过程中做零散测试或围绕单点失败反复打转。全部构建完成后统一验证。
- 不使用 GitHub git transport。远端读写、tag 发布和核验继续走 GitHub REST API。

## 设计方案

### 1. 交互命令扩展

`crates/yunxi-agent-cli/src/commands.rs` 新增 `/tools`、`/mcp`、`/cost`、`/status` 的解析和 help 文案。未知命令继续安全返回提示，不退出 REPL。

### 2. Session 状态摘要

`crates/yunxi-agent-cli/src/interactive.rs` 在每轮完成后从 `AgentRunResult.events` 归纳 `TurnSummary`：

- status 和 final response 是否存在。
- runtime event 总数。
- tool、shell command、MCP、file change、approval、escalation、child stream、warning、error、cancel 计数。
- 最近一轮 usage 和 REPL 累计 usage。

`/cost` 和 `/status` 直接读取这些摘要，不重新跑模型，不触发外部副作用。

### 3. 工具和 MCP 可见性

`/tools` 通过 YunXi-owned `yunxi-agent-tools::workspace_tool_registry` 读取固定工具和 workspace 动态工具。

`/mcp` 通过 YunXi-owned `yunxi-agent-mcp::load_workspace_mcp_configs` 读取 `.yunxi/mcp.json`、`.yunxi/mcp-servers.json` 或 `.mcp.json`，并检查 `.yunxi/mcp-runtime.json` 是否存在。该命令只读文件和配置，不主动启动 MCP server。

### 4. 文档与测试

README、`docs/extraction-status.md`、本开发报告和实施计划同步 v1.4.0。

新增 CLI 测试覆盖：

- v1.4.0 版本输出。
- interactive banner 版本。
- `/tools`、`/mcp`、`/cost`、`/status` 的基础输出。

测试只在构建完成后统一执行。

## 文件范围

- `Cargo.toml`
- `Cargo.lock`
- `crates/yunxi-agent-cli/Cargo.toml`
- `crates/yunxi-agent-cli/src/commands.rs`
- `crates/yunxi-agent-cli/src/interactive.rs`
- `crates/yunxi-agent-cli/src/render.rs`
- `crates/yunxi-agent-cli/src/main.rs`
- `crates/yunxi-agent-cli/tests/cli_tests.rs`
- `README.md`
- `docs/extraction-status.md`
- `docs/superpowers/plans/2026-07-12-yunxi-agent-v1-4-terminal-command-surface.md`
- `C:\Users\admin\Desktop\YunXi Agent开发日志.md`

## 统一验证门

构建完成后统一执行：

- `cargo fmt`
- `cargo fmt -- --check`
- `cargo test`
- `cargo check --workspace`
- `cargo build -p yunxi-agent-cli --release --bins`
- release 双 binary `--version` 均为 `yunxi 1.4.0`
- offline one-shot
- offline interactive `/tools`、`/mcp`、`/cost`、`/status` smoke
- JSONL smoke
- DeepSeek stream、non-stream、interactive smoke
- default CLI dependency scan
- owned-source secret scan
- `git diff --check`
- `codegraph sync "D:\YunXi Agent"`
- install 到 PATH，并确认 PATH `yunxi --version`
- GitHub REST API 发布 `master` 和 `v1.4.0`，回读核验旧 tag 未移动
- `cargo clean` 并确认 `target_exists=False`

## 成功判定

v1.4.0 完成后，用户在 PowerShell 输入 `yunxi` 进入 REPL 后，可以不离开当前 Agent 会话直接查看可用工具、MCP 配置、最近一轮 token usage、累计 usage、事件统计和会话状态。YunXi 仍保持默认自主运行，不依赖 Codex CLI 上游源码运行时。

## v1.4.0 实施结果

完成时间：2026-07-12 +08:00

本轮已将 v1.4 设计目标落地为 YunXi 自主 CLI 能力：

- workspace 版本升级到 `1.4.0`，release 双 binary 均输出 `yunxi 1.4.0`。
- `crates/yunxi-agent-cli/src/commands.rs` 新增 `/tools`、`/mcp`、`/cost`、`/status`。
- `crates/yunxi-agent-cli/src/interactive.rs` 新增交互态统计：
  - `InteractiveStats`
  - `TurnSummary`
  - `UsageTotals`
  - 最近一轮状态、累计 usage、事件总数、tool/command/MCP/file/approval/escalation/child/warning/error/cancel 计数。
- `/tools` 通过 YunXi-owned `workspace_tool_registry` 读取固定工具与 workspace 动态工具。
- `/mcp` 通过 YunXi-owned `load_workspace_mcp_configs` 读取 workspace MCP 配置，并只读检查 `.yunxi/mcp-runtime.json`。
- `/cost` 展示最近一轮和当前 REPL 累计 token usage。
- `/status` 展示 session/provider、最近一轮事件摘要、工具数量、MCP server 数和 usage。
- `crates/yunxi-agent-cli/Cargo.toml` 增加对 `yunxi-agent-tools`、`yunxi-agent-mcp` 的直接依赖，用于 CLI 只读展示；默认依赖树仍无上游 Codex runtime。
- `crates/yunxi-agent-cli/tests/cli_tests.rs` 更新 v1.4 版本断言，并新增 `/tools`、`/mcp`、`/cost`、`/status` smoke。
- README 和 `docs/extraction-status.md` 已同步 v1.4.0 行为说明。

## v1.4.0 统一验证结果

所有验证均在构建完成后统一执行：

- `cargo fmt`：通过。
- `cargo fmt -- --check`：通过。
- `cargo test`：第一次发现 `TurnSummary` 漏覆盖 `CommandUpdated`，修正为独立 command update 计数后重跑通过；workspace 单测、集成测试、doc tests 全部 0 failure。
- `cargo check --workspace`：通过。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- `target\release\yunxi.exe --version`：`yunxi 1.4.0`。
- `target\release\yunxi-agent-cli.exe --version`：`yunxi 1.4.0`。
- offline one-shot：通过。
- offline interactive `/tools`、`/mcp`、`/cost`、`/status`：通过。
- offline JSONL smoke：通过。
- Stage 4M real parity JSONL fixture：通过，131 条 JSONL event。
- DeepSeek stream smoke：通过，49 行输出，marker 正常，`secret_leak_detected=False`。
- DeepSeek non-stream smoke：通过，19 行输出，marker 正常，`secret_leak_detected=False`。
- DeepSeek interactive smoke：通过，47 行输出，marker 正常，`secret_leak_detected=False`。
- default CLI dependency scan：通过，`codex-*`、`vendor/codex-rs`、`yunxi-agent-codex` 命中 0。
- owned-source secret scan：通过，API key / Bearer / GitHub PAT 模式命中 0。
- `git diff --check`：通过，仅 Windows LF/CRLF 提示。
- release install：通过，安装到 `C:\Users\admin\AppData\Local\YunXi Agent\bin`。
- 安装版 SHA-256 与 release `yunxi.exe` 一致：`sha_match=True`。
- PATH `yunxi --version`：`yunxi 1.4.0`。
- PATH installed offline one-shot：通过。
- PATH installed interactive `/tools`、`/mcp`、`/cost`、`/status`：通过。
- `codegraph sync "D:\YunXi Agent"`：通过，5 个 changed files synced。
- Stage 4M 生成物 `.yunxi/` 与 `stage4m-runtime.txt` 已按路径边界校验后删除。

## 当前结论

v1.4.0 让 YunXi 交互终端具备更接近 Codex CLI 的自省命令面。用户可以在 REPL 内查看工具、MCP、usage 和最近一轮执行摘要。默认 `yunxi` 仍保持 YunXi 自主 runtime/provider/tools/storage 链路，不依赖上游 Codex CLI runtime 源码。

# YunXi Agent v1.7.5 CLI Protocol And Safety Development Report

生成时间：2026-07-13 09:06:54 +08:00

## 背景

YunXi Agent v1.7.4 已经完成 TUI transcript 的 wrapped-row viewport、scrollbar 几何和鼠标拖动能力，当前主目录、PATH 安装版本和 GitHub REST API 发布结果均停留在 v1.7.4。用户提供的 `C:\Users\admin\Desktop\YunXi-Agent-CLI-测试报告-2026-07-13.md` 对 v1.7.4 CLI 进行了新一轮审核，结论是基础能力已经可运行：真实 DeepSeek provider、offline fallback、JSON/JSONL、session 管理、工具 fixture、多 agent fixture 和 plain interactive 主路径都可用。

本报告面向 v1.7.5。目标不是继续扩 TUI 外观，而是收敛 CLI 协议、机器输出、安全边界表述和默认命令面。v1.7.5 完成后，YunXi Agent 应更接近“可脚本化、可回放、可接入外部 UI、不会误导用户安全边界”的终端 agent。

## 审核报告结论复核

本轮先读取审核报告全文，并按仓库规则使用 CodeGraph 和精确文本定位核对关键问题。外部报告中的主要问题与当前代码结构基本一致：

- `crates/yunxi-agent-runtime/src/lib.rs` 当前既会透传 provider stream 中的 `AgentEvent::Message`，又会在最终响应阶段再次 emit 同一条 assistant message，导致 JSON/JSONL 中最终回复重复。
- `crates/yunxi-agent-cli/src/main.rs` 的 `protocol_events_from_agent_events()` 对 MCP completed 只输出 `ResponseItem::McpToolCall`，没有补通用 `RuntimeEvent::ToolCompleted`，导致工具生命周期不闭合。
- `crates/yunxi-agent-cli/src/provider_mode.rs` 的 `auto_fallback_warning()` 只看 `ProviderModeSource::AutoOffline`，没有区分 `dry-run` / `codex` backend，所以会打印误导性的 provider credential fallback warning。
- `crates/yunxi-agent-cli/src/main.rs` 将 `--json` 和 `--jsonl` 作为 global flag 暴露，但没有互斥，也没有对子命令的 JSONL 支持范围做前置校验。
- `sessions list --json` 目前直接输出完整 `SessionRecord` 列表，容易携带完整 events、prompt、final response 和工具输出，不适合作为轻量列表接口。
- `CliBackend::Codex` 在默认 CLI help 中可见，但 `run_codex_backend()` 是 detached placeholder，会让用户以为默认 CLI 可以直接使用 Codex backend。
- `crates/yunxi-agent-sandbox/src/lib.rs` 已明确把 `WorkspaceGuard`、`WindowsRestrictedToken`、`LinuxLandlock` 的 label 标为 `policy guard: advisory only, no OS isolation`，`crates/yunxi-agent-exec/src/lib.rs` 仍通过 `Command::new(...).spawn()` 直接执行子进程。审核报告的 P0 判断成立：当前是策略守卫，不是真正 OS 隔离。
- TUI 已进入默认 workspace，这与早期“第一阶段不加 TUI”的项目说明存在历史冲突；但 v1.7.x 的 TUI 是用户后续明确要求并已发布的能力，因此 v1.7.5 不移除 TUI，而是更新项目说明，把 TUI 作为当前 CLI 产品面的一部分。

## v1.7.5 总目标

YunXi Agent v1.7.5 要修复审核报告中影响 CLI 可用性和外部集成稳定性的协议/输出问题，并把沙盒安全边界从“容易被误解的 facade”收敛为“明确可见的 policy guard + 后续真实 platform runner 入口”。

成功标准：

- JSON/JSONL 中同一轮最终 assistant message 只出现一次。
- MCP 工具调用既保留富信息 `mcp_tool_call` item，也产生通用 `tool_completed` lifecycle event。
- `--backend dry-run` 和默认不可用的 `--backend codex` 不再打印 provider credential fallback warning。
- `--json` 与 `--jsonl` 互斥，同时传入时 parse 失败并返回清晰错误。
- 不支持 JSONL 的子命令不再静默忽略 `--jsonl`，必须拒绝并说明原因。
- `sessions list --json` 输出轻量 summary；完整会话内容继续通过 `sessions show <id> --json` 或后续显式 export 读取。
- 默认 CLI 对 `codex` backend 的暴露更诚实：隐藏、feature gate 或 parse 阶段明确拒绝，不能先进入 provider fallback。
- prompt 与 `sessions` / `parity` 保留词冲突时，CLI 给出明确的 `--` 或 `run` 使用建议；如实现 `yunxi run [PROMPT]...`，根命令自然语言入口保持可用。
- sandbox 相关输出不声称 OS isolation；事件、文档和 CLI 状态都明确当前是 advisory policy guard。
- v1.7.5 实现仍不依赖上游 Codex CLI 源码、`vendor/codex-rs`、`codex-*` 或 `yunxi-agent-codex` 默认依赖。

## 非目标

- v1.7.5 不承诺一次性完成真正的 Windows restricted token / job object 和 Linux Landlock/seccomp 强隔离。该问题是平台级安全工程，必须单独进入 v1.7.6 或 v1.8 的 sandbox runner 深化阶段。
- v1.7.5 不删除 TUI。TUI 已经是 v1.7.x 经用户确认的默认 CLI 能力，本轮只修 CLI 协议和安全边界，不回滚已发布产品面。
- v1.7.5 不恢复任何上游 Codex crate 默认依赖。
- v1.7.5 不在开发报告阶段创建 tag。tag 只在源码构建、统一验证、安装、GitHub REST API 发布完成后创建。

## 文件变更地图

### Runtime 协议去重

修改：

- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`

职责：

- 在 provider stream 收集阶段记录 assistant message 是否已经通过 stream sink emit。
- 最终响应阶段只负责设置 `final_response`、session result 和 completed metadata；如果同内容 assistant message 已经 emit，不再重复 emit `AgentEvent::Message`。
- 保持 offline/static provider、live provider stream、fixture provider 的输出一致。

建议接口形态：

- 将 `collect_provider_response()` 的返回从裸 `ProviderResponse` 扩展为内部 struct，例如 `CollectedProviderResponse { response, emitted_assistant_message }`。
- 或在 stream sink 映射 `StreamEvent -> AgentEvent` 时记录最终 message emit 状态，最终阶段按该状态判断。

测试落点：

- `crates/yunxi-agent-cli/tests/jsonl_tests.rs`
- `crates/yunxi-agent-cli/tests/cli_tests.rs`
- `crates/yunxi-agent-runtime/src/lib.rs` 内 runtime 单元测试

### MCP tool lifecycle 闭合

修改：

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\jsonl_tests.rs`

职责：

- `AgentEvent::McpToolCompleted` 转换时先输出通用 `RuntimeEvent::ToolCompleted { call_id, output, status }`，再保留现有 `RuntimeEvent::Item { ResponseItem::McpToolCall ... }`。
- 保证所有 `RuntimeEvent::ToolStarted.call.id` 在 completed 状态下都有对应 `RuntimeEvent::ToolCompleted.call_id`，MCP、shell、patch、skill、多 agent 等工具使用统一生命周期。

测试落点：

- Stage 4M fixture JSONL：统计所有 started id 与 completed id，MCP call id 不再缺失。
- 保留已有 `mcp_tool_call` item 断言，避免富信息回归。

### Provider fallback warning 修正

修改：

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\provider_mode.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\interactive.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\jsonl_tests.rs`

职责：

- `auto_fallback_warning()` 只在真实 YunXi auto-offline runtime 下返回 warning。
- `dry-run` backend 使用独立 source 或让 `auto_fallback_warning()` 同时检查 `selection.is_offline_runtime()`。
- `codex` backend 在默认 CLI 被拒绝时不进入 provider fallback 打印。

测试落点：

- `yunxi --backend dry-run "hello"` 不包含 provider credential fallback warning。
- `yunxi --backend dry-run --jsonl "hello"` 不输出 provider fallback warning event。
- 真实缺失凭据的 `yunxi --jsonl "hello"` 仍输出结构化 fallback warning event。

### CLI 输出模式约束

修改：

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\jsonl_tests.rs`

职责：

- 在 clap 层为 `json` 与 `jsonl` 加互斥约束。
- 在命令派发前增加 output-mode preflight：对不支持 JSONL 的 `sessions`、`parity` 子命令直接返回 invalid input。
- 错误文本必须明确，例如 `--jsonl is only supported for run/resume agent execution in v1.7.5`。
- 如果 `--jsonl` 已经触发 parse/preflight 错误，仍应输出合法 JSONL error event，延续当前 JSONL error 处理路径。

测试落点：

- `yunxi --offline --json --jsonl "hello"` exit code 2。
- `yunxi sessions list --jsonl` exit code 2，输出不是 TSV。
- `yunxi parity map --jsonl` exit code 2，输出不是 Markdown。
- `yunxi --jsonl` 无 prompt 仍输出结构化错误。

### Prompt 与子命令冲突

修改：

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- `D:\YunXi Agent\README.md`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`

职责：

- 短期修复：当 `sessions` / `parity` 被当成子命令但缺少子命令参数时，错误提示要告诉用户：若要把它作为 prompt，请使用 `yunxi -- sessions`。
- 推荐新增显式入口：`yunxi run [PROMPT]...`，用于避开保留词冲突，同时保持根命令 `yunxi "prompt"` 的 one-shot 入口。
- 根命令无 prompt 时继续进入 interactive/TUI，不改变 v1.1 以来的默认体验。

测试落点：

- `yunxi --offline sessions` 的错误提示包含 `yunxi -- sessions` 或 `yunxi run sessions`。
- `yunxi --offline -- sessions` 继续作为 prompt 执行。
- 如新增 `run` 子命令，`yunxi run sessions` 正常执行 agent turn。

### Codex backend placeholder 收敛

修改：

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- `D:\YunXi Agent\README.md`
- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`

职责：

- 默认 CLI help 不应把 detached `codex` backend 表现为可用能力。
- 推荐做法：保留 enum 兼容路径，但在 default feature 下 hide possible value 或 preflight 明确拒绝，并在错误中指向 `yunxi-agent-codex` compatibility crate。
- `--live` 文案必须指向 live provider，而不是让用户误以为接入上游 Codex backend。

测试落点：

- `yunxi --help` 默认输出不把 `codex` 描述为普通可选 backend。
- `yunxi --backend codex "hello"` 不打印 provider fallback warning，并以明确错误退出。

### Sessions list summary

修改：

- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`

职责：

- 增加轻量 `SessionSummary` 类型，字段建议包括：
  - `id`
  - `cwd`
  - `status`
  - `created_at`
  - `updated_at`
  - `archived`
  - `pinned`
  - `parent_id`
  - `prompt_preview`
  - `final_response_preview`
  - `event_count`
  - `child_count`
- `sessions list --json` 默认输出 `Vec<SessionSummary>`。
- `sessions show <id> --json` 继续输出完整 record。
- 如需要完整列表，后续可以加 `sessions export` 或 `sessions list --full --json`，v1.7.5 不默认暴露完整 events。

测试落点：

- `sessions list --json` 不包含完整 `events` 数组。
- `sessions show <id> --json` 仍包含完整会话记录。
- summary preview 有长度上限，避免泄露长 prompt/tool 输出。

### Sandbox honesty 与 runner 深化入口

修改：

- `D:\YunXi Agent\crates\yunxi-agent-sandbox\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-exec\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tools\src\lib.rs`
- `D:\YunXi Agent\README.md`
- `D:\YunXi Agent\docs\extraction-status.md`

职责：

- 保留当前 label：`policy guard: advisory only, no OS isolation`，并确保 CLI/TUI/JSONL 中没有更强的暗示。
- `SandboxRunnerDiagnostic` 增加或确认可机器读取字段，例如 `os_isolation: false`、`enforcement: policy_guard`。
- `ExecManager` 接收 sandbox execution plan，但在当前 runner 未实现 OS isolation 时必须把 diagnostic 透出到 runtime events。
- 为 v1.7.6/v1.8 预留 platform runner trait/facade，不在 v1.7.5 伪装强隔离。

测试落点：

- Stage 4M fixture sandbox event 中包含 `os_isolation=false` 或等价字段。
- read-only/workspace-write 的策略拒绝测试继续通过。
- danger-full-access 明确显示 bypass policy guard。

## 分阶段实施建议

后续实现阶段继续遵守用户硬性约束：先构建完整 v1.7.5 变更，中间不做反复单点验证，不在某一个点耗费大量时间；源码构建完成后统一验证。

### Phase 1：协议单一来源与 MCP 生命周期

目标：

- 修复重复 assistant message。
- 修复 MCP `tool_started` / `tool_completed` 不配对。

改动文件：

- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-cli/src/main.rs`
- `crates/yunxi-agent-cli/tests/jsonl_tests.rs`
- `crates/yunxi-agent-cli/tests/cli_tests.rs`

完成条件：

- JSON/JSONL 里最终 assistant message 唯一。
- Stage 4M fixture 中 MCP started/completed call id 闭合。

### Phase 2：CLI 输出模式与 provider warning

目标：

- `--json` / `--jsonl` 不再同时生效。
- 子命令不再静默忽略 `--jsonl`。
- dry-run/codex 不再打印 provider fallback warning。

改动文件：

- `crates/yunxi-agent-cli/src/main.rs`
- `crates/yunxi-agent-cli/src/provider_mode.rs`
- `crates/yunxi-agent-cli/src/interactive.rs`
- `crates/yunxi-agent-cli/tests/cli_tests.rs`
- `crates/yunxi-agent-cli/tests/jsonl_tests.rs`

完成条件：

- 机器输出模式行为可预测。
- 错误路径也能被 JSONL 消费者解析。

### Phase 3：Prompt/subcommand 与 Codex placeholder 收敛

目标：

- 自然语言 prompt 和保留子命令冲突时给出明确出口。
- 默认 CLI 不再把不可用 Codex backend 暴露成普通能力。

改动文件：

- `crates/yunxi-agent-cli/src/main.rs`
- `README.md`
- `crates/yunxi-agent-cli/tests/cli_tests.rs`

完成条件：

- `yunxi -- sessions` 和可选 `yunxi run sessions` 路径明确。
- `--backend codex` 默认不误导，不打印 provider fallback warning。

### Phase 4：Session summary 与安全边界可见性

目标：

- `sessions list --json` 降为轻量 summary。
- sandbox policy guard 的真实边界在 JSONL/CLI/docs 中可见。

改动文件：

- `crates/yunxi-agent-storage/src/lib.rs`
- `crates/yunxi-agent-cli/src/main.rs`
- `crates/yunxi-agent-sandbox/src/lib.rs`
- `crates/yunxi-agent-exec/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `README.md`
- `docs/extraction-status.md`

完成条件：

- 列表接口不再输出完整 events。
- 当前 sandbox 不再被误解为 OS isolation。

### Phase 5：版本、文档、发布

目标：

- 版本推进到 `1.7.5`。
- 报告、README、状态文档、桌面开发日志同步。
- 源码实现完成并统一验证后创建新的不可变 annotated tag `v1.7.5`。

改动文件：

- `Cargo.toml`
- `Cargo.lock`
- `README.md`
- `docs/extraction-status.md`
- `docs/reports/2026-07-13-yunxi-agent-v1-7-5-cli-protocol-safety-development-report.md`
- `C:\Users\admin\Desktop\YunXi Agent开发日志.md`

发布要求：

- 所有 GitHub 读写推送继续只走 REST API。
- 不使用 `git push`、`git fetch`、`git ls-remote`。
- 不打印、不记录、不提交 `C:\Users\admin\Desktop\api.txt` 中的任何密钥。
- 旧 tag 不删除、不移动。

## 统一验证计划

实现阶段完成全部构建后，统一运行以下验证：

- `cargo fmt`
- `cargo fmt -- --check`
- `cargo test`
- `cargo check --workspace`
- `cargo build -p yunxi-agent-cli --release --bins`
- `target\release\yunxi.exe --version`，期望 `yunxi 1.7.5`
- `target\release\yunxi-agent-cli.exe --version`，期望 `yunxi 1.7.5`
- `target\release\yunxi.exe --offline "v1.7.5 offline smoke"`
- `target\release\yunxi.exe --offline --json "v1.7.5 json duplicate probe"`
- `target\release\yunxi.exe --offline --jsonl "v1.7.5 jsonl duplicate probe"`
- `YUNXI_RUNTIME_FIXTURES=1 target\release\yunxi.exe --offline --jsonl "stage 4m real parity fixture"`
- `target\release\yunxi.exe --backend dry-run "v1.7.5 dry run warning probe"`
- `target\release\yunxi.exe --offline --json --jsonl "v1.7.5 conflict probe"`，期望 parse 失败
- `target\release\yunxi.exe sessions list --jsonl`，期望明确拒绝
- `target\release\yunxi.exe parity map --jsonl`，期望明确拒绝
- `target\release\yunxi.exe sessions list --json`，确认输出 summary 且不含完整 events
- `target\release\yunxi.exe --help`，确认默认 help 不误导 Codex backend
- plain interactive smoke，确认 TUI/plain 入口仍可打开
- DeepSeek live stream smoke：从 `C:\Users\admin\Desktop\api.txt` 读取密钥，不打印密钥，确认真实 provider 路径仍可用
- DeepSeek live non-stream smoke：同上
- default dependency scan：确认默认 CLI 依赖不含 `codex-*`、`vendor/codex-rs`、`yunxi-agent-codex`
- owned-source secret scan：排除 `.git`、`.codegraph`、`target`、`vendor`、`extracted`
- `git diff --check`
- `codegraph sync "D:\YunXi Agent"`
- `codegraph status "D:\YunXi Agent"`
- `scripts\install\install-yunxi.ps1 -AddToPath -SkipBuild`
- PATH smoke：`yunxi --version`、`yunxi-agent-cli --version`、`yunxi --offline "installed path v1.7.5 smoke"`
- GitHub REST API 发布 `master` 和 annotated tag `v1.7.5`
- 本地 `master`、`origin/master`、远端 API commit、`v1.7.5` tag 解引用一致性校验
- `cargo clean`
- `Test-Path D:\YunXi Agent\target`，期望 `False`

## 风险与处理

- Runtime message 去重如果只在 CLI 层过滤，会掩盖 session history 重复问题；v1.7.5 应在 runtime 层定源头。
- MCP completed 如果替换现有 `mcp_tool_call` item，会破坏外部消费者；v1.7.5 应新增通用 lifecycle，不删除富信息 item。
- `sessions list --json` 改 summary 可能影响已有脚本；完整数据仍通过 `sessions show <id> --json` 保留，文档必须写清。
- `--jsonl` 子命令拒绝会改变原来“忽略 flag”的行为，但这是必要的协议修正，因为静默输出 TSV/Markdown 对机器消费者更危险。
- Codex backend placeholder 不能被实现成重新依赖上游源码；默认 CLI 仍保持 YunXi 自主运行。
- 沙盒真实 OS isolation 不在 v1.7.5 伪装完成。v1.7.5 要让风险可见，并为下一版平台 runner 深化留出清晰接口。

## v1.7.5 完成后的预期状态

完成 v1.7.5 后，YunXi Agent 应具备：

- 更干净的 JSON/JSONL 协议输出。
- 可配对的工具 lifecycle，MCP 不再停在 started 状态。
- 更少误导的 provider/backend warning。
- 更严格的 CLI 输出模式约束。
- 更安全的 session list 机器接口。
- 更诚实的 sandbox policy guard 表述。
- 继续保留 v1.7.4 TUI 能力。
- 继续保持默认运行路径不依赖上游 Codex CLI 源码。


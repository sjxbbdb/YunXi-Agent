# YunXi Agent v1.3 Terminal Streaming Approval Cancellation Development Report

生成时间：2026-07-12 +08:00

## 背景

Claude 对 YunXi Agent 当前代码做了终端 Agent 能力差距评估。该报告的核心判断经本地代码抽查后成立：
YunXi 的 provider 主链、tool loop、shell、patch、skill、MCP 传输边界、sandbox 策略和 DeepSeek live provider
都已经是真实现；真正阻碍它像 Codex CLI / Claude Code CLI 一样工作的，是终端交互层仍然建立在
“整轮执行完成后返回 `AgentRunResult` 并回放事件”的批处理接口上。

v1.2.1 已经解决了 DeepSeek HTTP 400、tool result `tool_call_id`、REPL 单轮失败后退出等问题。v1.3 不再继续
堆 fixture parity，也不重做 provider/tool 地基，而是把现有真实地基接到真实终端交互体验上。

## 当前确认状态

### 可以继续复用的真实地基

- provider 真实通过 reqwest 调用 `/chat/completions`，支持 Bearer 鉴权、DeepSeek 自动选择和 400/422 metadata fallback。
- provider SSE 增量解析、非流式解析、tool call 解析和动态 tools schema 注入已经存在。
- runtime 多轮 tool loop 已经是真执行：模型请求 tool、YunXi 执行 tool、结果回填历史、继续请求模型，直到无 tool call。
- shell/exec 使用 `tokio::process` 真 spawn 子进程，并有 timeout/kill、stdout/stderr 聚合、文件快照 diff。
- apply_patch 使用文件系统真读写、删除、改名。
- skill 能扫描 `.codex/skills`、`.yunxi/skills`、`skills` 并读取 `SKILL.md`。
- sandbox/approval 策略能在执行前判断 cwd、network、读写风险和 escalation。
- 多 agent 子 runtime、session storage、JSONL 输出、secret redaction、GitHub API 发布流程均已经具备。

### 当前核心缺陷

- `AgentBackend::run(config, input) -> AgentRunResult` 是批处理签名，CLI 只能等整轮结束后回放事件。
- 底层 provider 的 SSE delta 虽然存在，但 runtime/CLI 边界把它压平为完整事件数组，用户看不到实时进度。
- `ApprovalRequested` / `ApprovalCompleted` 只是历史事件展示，终端不会中途阻塞询问用户 yes/no。
- Ctrl+C 只是在 interactive `tokio::select!` 层放弃等待 turn future，没有统一 cancellation token 贯穿 runtime、tools、exec。
- `request_user_input` 因缺少 interactive host 反向通道而恒 declined。
- `FileChanged`、`PatchCompleted`、`TodoUpdated`、usage 在交互渲染中不可见或信息不足。
- 默认 MCP runtime 仍包含 fixture echo；StaticProvider 仍应保留为测试/离线夹具，但不能继续被误解为真实智能决策来源。

## v1.3 目标

v1.3 的目标是完成 YunXi Agent 从“批处理 Agent runner”到“真实终端交互 Agent”的关键跃迁：

- 真流式输出：runtime 边执行边向 CLI 发送事件，CLI 即时渲染 reasoning、message、tool start/update/completed、文件变化和 session 状态。
- 真交互审批：危险 tool 或需要 escalation 时，runtime 发出审批请求并等待 CLI 用户输入；CLI 返回 approve/deny 后 runtime 再继续。
- 真取消：Ctrl+C 触发同一个 cancellation token，贯穿 runtime、provider stream、tools、exec；长 shell 子进程要被明确终止。
- 保留兼容：one-shot、JSONL、离线 fixture、现有 tests 和 session storage 继续可用；批处理 API 可以通过流式 API 的收集适配保留。
- 降低 fixture 混淆：fixture provider/MCP 明确限定为 offline/test 路径，live 默认路径不再静默暴露 fixture MCP。
- 终端可见性提升：至少补齐 FileChanged、PatchCompleted、TodoUpdated、Completed usage、真实 `/clear`。

## 非目标

- 不重写 provider 主链，不重做 DeepSeek 接入，不替换模型接口；模型层只保留可扩展边界。
- 不在本版强行完成 TUI、桌面端、云任务、update、doctor、marketplace、SDK packaging。
- 不在本版完整引入 Responses API 请求侧；可以保留当前解析能力，后续单独补。
- 不在本版做完整 rich terminal UI；颜色、markdown、diff 高亮、reedline 可以作为 v1.3 后续增强。
- 不删除历史 tag，不覆盖 v1.0.0、v1.1.0、v1.2.0、v1.2.1。

## 架构方案

### 1. 增加流式运行边界

新增 YunXi 自主的运行通道类型，建议放在 `yunxi-agent-core`：

- `AgentRunControl`：包含 `event_tx`、approval responder、user input responder、`AgentCancellationToken`。
- `AgentRunStreamHandle`：包含事件接收端和最终 join/result。
- `AgentBackend::run_stream(...)` 或等价扩展 trait：供 CLI 获取实时事件。
- 保留 `AgentBackend::run(...)`：内部调用 stream API、收集事件并返回 `AgentRunResult`，保证 one-shot 和现有调用方不被一次性打断。

runtime 内部已有 `RuntimeEventSink`，v1.3 要把它从“内存收集器”升级为“收集 + 发送”的双写 sink。所有 `emit` 仍记录事件，额外把事件通过 mpsc 发送给 CLI。

### 2. CLI 从回放渲染改为实时渲染

interactive `run_turn` 不再等待完整 `AgentRunResult` 后调用 `render_agent_result`。新流程：

- 创建 `AgentRunControl` 和事件接收端。
- 启动 runtime turn future。
- CLI 循环接收事件并调用 `render_agent_event` 逐条渲染。
- runtime 完成后，CLI 根据最终结果更新 active session 和 turn count。
- 如果一轮失败，只输出脱敏错误并继续 REPL，不污染成功 session 状态。

保留 `render_agent_result` 给 one-shot 或测试使用，但其内部应复用 `render_agent_event`，避免流式和批处理渲染分叉。

### 3. 反向审批通道

新增审批请求/响应协议，最小可用形态：

- runtime 在 tool 执行前遇到 `requires_approval`、`requires_escalation`、sandbox blocked-but-requestable 时，发出 `ApprovalRequested` 或 `EscalationRequested`。
- runtime 挂起等待 CLI 的 `ApprovalDecision`。
- CLI 显示 tool 名称、命令、cwd、risk、reason、sandbox/network 要求，读取 `y/N`。
- 用户批准后执行 tool，并发出 `ApprovalCompleted { approved: true }`。
- 用户拒绝后不执行 tool，生成 declined tool response，并发出 `ApprovalCompleted { approved: false }`。

该通道也作为 `request_user_input` 的底层能力。v1.3 至少让 `request_user_input` 在 interactive host 中可用；非 interactive/jsonl 下继续安全 declined 或走 auto-resolution。

### 4. 贯穿 cancellation

`AgentCancellationToken` 已存在但未贯穿主链。v1.3 要把它接入：

- CLI Ctrl+C 调用 `cancel()`，不只是 drop turn future。
- runtime tool loop、provider stream 累加、child agent、MCP 调用、shell exec 在关键 await 点检查 token。
- exec 层对正在运行的子进程调用 kill，并发出 `Cancelled` / `CommandCompleted cancelled` 事件。
- cancellation 后保留已发送事件，避免用户丢失本轮已发生的进度。
- session 状态只在有明确完成结果时更新；取消轮不覆盖最后成功 session。

### 5. 去 fixture 混淆

- StaticProvider 保留，但文档、banner、provider_source 必须明确显示 `offline` / `fixture`。
- live provider 路径不应默认暴露 fixture MCP echo。无 MCP 配置时，MCP 工具列表要么为空，要么产生明确 “no workspace MCP configured” 的生命周期事件。
- Stage 4K/4L/4M parity fixture 分支保留为测试资产，但不得作为用户默认能力描述。

## 文件范围

预计主要修改：

- `crates/yunxi-agent-core/src/backend.rs`：扩展 backend 流式接口和控制通道类型。
- `crates/yunxi-agent-core/src/cancellation.rs`：完善 token 语义或增加 helper。
- `crates/yunxi-agent-runtime/src/lib.rs`：runtime event sink 双写、tool loop 审批等待、取消检查、stream result 汇总。
- `crates/yunxi-agent-runtime/src/session_driver.rs`：审批/缓存/策略事件与新 decision 通道对接。
- `crates/yunxi-agent-tools/src/lib.rs`：`ToolRuntime::execute` 需要接收 cancellation/interactive context 或通过 request 携带控制柄；`request_user_input` 改为 interactive 可用；默认 MCP fixture 限定。
- `crates/yunxi-agent-exec/src/lib.rs`：exec manager 接入 cancellation token，确保 Ctrl+C 能 kill 子进程并保留事件。
- `crates/yunxi-agent-cli/src/interactive.rs`：改为事件流消费、审批 prompt、Ctrl+C cancellation。
- `crates/yunxi-agent-cli/src/render.rs`：新增 `render_agent_event`，补齐文件/patch/todo/usage 渲染。
- `crates/yunxi-agent-cli/src/commands.rs`：`/clear` 真清屏；可预留 `/tools`、`/mcp`、`/cost` 后续接口。
- `crates/yunxi-agent-cli/tests/cli_tests.rs`、`jsonl_tests.rs`：增加 interactive streaming、approval、cancel 集成测试。
- `crates/yunxi-agent-runtime/tests/runtime_tests.rs`、`crates/yunxi-agent-tools/tests/tool_tests.rs`、`crates/yunxi-agent-exec` 单测：覆盖 token、审批、request_user_input。
- `docs/extraction-status.md`、`README.md`、本报告、后续实施计划：同步说明 v1.3 行为边界。

## 开发阶段划分

### Phase 1：流式事件骨架

- 新增流式 runner/control 类型。
- RuntimeEventSink 双写到 mpsc。
- 保留批处理兼容适配。
- CLI interactive 改为边收边渲染。
- 验收目标：长任务执行时，CLI 在 turn 完成前能看到 tool started / output updated / reasoning 等事件。

### Phase 2：交互审批

- 增加 approval request/response 通道。
- runtime 在 tool 执行前等待审批决策。
- CLI 提供 `y/N` prompt。
- 拒绝时 tool 不执行，模型收到 declined tool result。
- 验收目标：危险 shell/patch 在终端中途暂停，用户批准才执行，拒绝不产生文件/命令副作用。

### Phase 3：贯穿取消

- Ctrl+C 调用 shared cancellation token。
- runtime、tools、exec 检查 token。
- exec 子进程收到 kill。
- 已发事件不丢失，取消轮不污染 session。
- 验收目标：长 shell 命令可被 Ctrl+C 中止，进程退出后系统中无残留子进程，本轮输出显示 cancelled。

### Phase 4：终端可见性和默认值修正

- `render_agent_event` 补齐 `FileChanged`、`PatchCompleted`、`TodoUpdated`、usage。
- `/clear` 真清屏。
- live 默认路径不暴露 fixture MCP；无配置时明确提示。
- README 和状态文档删除或收敛容易误导的 fixture parity 表述。

### Phase 5：统一验证、打包、发布

- 所有构建完成后一次性跑完整验证门。
- 安装 `yunxi 1.3.0` 到 PATH。
- 使用 DeepSeek API 做真实 streaming/approval/cancel 可行性验证。
- 创建新 commit 和 annotated tag `v1.3.0`。
- 通过 GitHub REST API 发布 master 和 tag。
- 更新桌面开发日志。
- 执行 `cargo clean`，清理仓库 `.yunxi` 和临时文件。

## 测试与验收标准

构建阶段仍遵守用户硬性约束：源码迁移/构建过程中不做零散测试，全部接线完成后统一验证。

统一验证至少包括：

- `cargo fmt`
- `cargo fmt -- --check`
- `cargo test`
- `cargo check --workspace`
- `cargo build -p yunxi-agent-cli --release --bins`
- `yunxi --version` 与 `yunxi-agent-cli --version` 均为 `1.3.0`
- forced-offline one-shot 和 interactive smoke
- live DeepSeek stream、non-stream、interactive smoke
- interactive streaming fixture：事件在 turn 完成前到达
- approval fixture：用户 approve 后执行，deny 后不执行
- cancellation fixture：长 shell 被 Ctrl+C 取消且无残留进程
- `request_user_input` interactive fixture：CLI 输入能返回 runtime
- default CLI dependency tree 中上游 Codex runtime 命中 0
- secret/API path scan 0 命中
- `git diff --check`
- CodeGraph sync
- 安装版 SHA-256 与 release 二进制一致
- 清理后 `target_exists=False`、仓库根目录 `.yunxi` 不存在、PATH 安装版仍可运行

## 风险与控制

- 风险：直接改 `AgentBackend::run` 会影响 one-shot、JSONL、tests、runtime fixtures。
  控制：优先新增 stream API，再让旧 `run` 通过 stream collector 适配，降低破坏面。

- 风险：审批通道容易和现有 sandbox/approval policy 重复。
  控制：保留现有 policy 作为风险判定层，新增通道只负责 interactive 决策，不重写风险规则。

- 风险：取消子进程在 Windows 上容易只取消 future、不 kill process。
  控制：exec 层必须持有 child handle；取消测试必须检查命令确实结束。

- 风险：真实 live 测试受模型输出不稳定影响。
  控制：live smoke 使用短 marker prompt，功能性断言聚焦退出码、事件形状、无泄漏和 REPL 生命周期。

- 风险：fixture 清理不彻底再次占用硬盘。
  控制：发布后强制 `cargo clean`，并检查 `target_exists=False`。

## 发布策略

- 下一版本号：`v1.3.0`。
- 创建新 annotated tag `v1.3.0`；旧 tag `v1.0.0`、`v1.1.0`、`v1.2.0`、`v1.2.1` 不删除、不移动。
- GitHub 远端读取、写入、核验全部走 GitHub REST API。
- 不使用 `git push`、`git fetch`、`git ls-remote`。
- API key/PAT 不打印、不写日志、不写源码、不提交。
- 发布后本地 `master`、`origin/master`、`v1.3.0` 与 GitHub API 对象同步。

## 成功判定

v1.3.0 完成后，YunXi Agent 应满足以下用户可感知标准：

- 在 PowerShell 输入 `yunxi` 后，真实模型长任务不再长时间静默等待，而是持续显示 reasoning/tool/output 进度。
- 当模型请求高风险 shell/patch/network/escalation 时，终端会停下来询问，用户批准才执行。
- 用户按 Ctrl+C 能真正中断当前 turn 和正在运行的 shell，而不是只返回 prompt。
- 文件变化、patch、todo、usage 在终端可见。
- 接入 DeepSeek API 后，YunXi 可以作为独立、自主、可交互、安全可控的终端 Agent 使用，不依赖 Codex CLI 上游运行时代码。

## v1.3.0 实施结果

完成时间：2026-07-12 13:29:16 +08:00

本轮已把 v1.3 设计目标落地为 YunXi 自主运行能力：

- `yunxi-agent-core` 新增 `AgentRunControl`、事件流 receiver、交互审批请求/响应、用户输入请求/响应，并扩展 `AgentBackend::run_stream`。
- `yunxi-agent-runtime` 使用双写事件 sink，批处理结果继续保留完整事件数组，交互模式实时收到事件；runtime 主链新增 control-aware helper。
- runtime tool loop 支持交互审批、escalation 批准后的策略放行、拒绝后的 declined tool result，以及 interactive `request_user_input` 回填。
- `yunxi-agent-exec` 新增 `run_with_cancellation`，取消 token 可 kill 正在运行的 shell 子进程并记录 `ExecLifecycleEvent::Cancelled`。
- `yunxi-agent-tools` 新增兼容的 `execute_with_control`，旧 `execute` 保持可用，shell/组合 runtime 可接收 cancellation token。
- `yunxi-agent-cli` interactive REPL 改为实时事件渲染，支持审批 `y/N`、用户输入、Ctrl+C cancel、真实 `/clear`，并将 banner/版本升级到 `v1.3.0`。
- renderer 现在能渲染 `FileChanged`、`PatchCompleted`、`TodoUpdated`、usage，并复用单事件渲染状态避免流式/批处理分叉。
- 新增 runtime/exec 测试覆盖事件先于慢 provider 完成、审批 approve/deny、request_user_input、shell 取消传播。

## v1.3.0 统一验证结果

所有验证均在构建完成后统一执行：

- `cargo fmt`：通过。
- `cargo fmt -- --check`：通过。
- `cargo test`：通过；workspace 单测、集成测试、doc tests 全部 0 failure。
- `cargo check --workspace`：通过，无 warning。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- `target\release\yunxi.exe --version`：`yunxi 1.3.0`。
- `target\release\yunxi-agent-cli.exe --version`：`yunxi 1.3.0`。
- offline one-shot：通过。
- offline interactive `/exit`：通过，banner 显示 `YunXi Agent v1.3.0 interactive CLI` 和 `offline_runtime: static_provider + fixture_mcp`。
- offline JSONL smoke：通过，输出 19 条 JSONL runtime event。
- Stage 4M real parity JSONL fixture：通过，输出 131 条 JSONL event。
- DeepSeek streaming live smoke：通过，47 条 JSONL event，`secret_leak_detected=False`。
- DeepSeek non-stream live smoke：通过，19 条 JSONL event，`secret_leak_detected=False`。
- DeepSeek interactive live smoke：通过，检测到 interactive banner、assistant marker、正常 `/exit`，`secret_leak_detected=False`。
- default CLI dependency scan：通过，`codex-*`、`vendor/codex-rs`、`yunxi-agent-codex` 命中 0。
- owned-source secret scan：通过，API-key / Bearer / Authorization Bearer 模式命中 0。
- `git diff --check`：通过，仅 Windows LF/CRLF 提示。
- release 安装到 `C:\Users\admin\AppData\Local\YunXi Agent\bin`：通过。
- 安装版 SHA-256：与 `target\release\yunxi.exe` 一致。
- PATH `yunxi --version`：`yunxi 1.3.0`。
- PATH installed offline interactive smoke：通过。
- PATH installed DeepSeek live JSONL smoke：通过，50 条 JSONL event，`installed_live_secret_leak_detected=False`。
- `codegraph sync "D:\YunXi Agent"`：通过，14 个 changed files synced。

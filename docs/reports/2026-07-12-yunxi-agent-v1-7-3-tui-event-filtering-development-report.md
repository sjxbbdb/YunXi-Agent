# YunXi Agent v1.7.3 TUI Event Filtering And Tool Timeline Development Report

生成时间：2026-07-12 21:38:59 +08:00

## 背景

YunXi Agent v1.7.2 已经完成 TUI 独立 crate、滚轮回看、viewport、scrollbar、frame throttle 和基础流式输出稳定化。用户最新截图显示，页面结构和滚动能力已经成立，但中间 transcript 仍然暴露了大量与用户无关的底层事件：

- `[stdout]` 逐 token 展示了工具调用协议 JSON，例如 `{`、`"arguments_json"`、`"name"`、`using-superpowers`。
- `[tool-output]` 直接展示了 skill 调用返回的完整文档内容，导致主视图被长文本淹没。
- `[context]`、`[session]`、`Provider turn started/completed`、sandbox/backend/policy 细节都进入主 transcript，降低可读性。
- 工具执行链路没有被整理成“申请审批、审批通过、开始执行、完成/失败”的紧凑时间线，而是混在 stdout、thinking、tool-output 和 policy 事件里。

用户明确要求 v1.7.3 的流式输出只展示思考链路和工具调用链路。这里的“思考链路”应理解为模型/运行时明确提供的 reasoning summary 或用户可读推理进度，不应把 provider bookkeeping、协议 JSON、工具 stdout token 当作思考内容展示。

## 当前源码诊断

本轮使用 CodeGraph 先查询当前 TUI 入口，确认问题集中在 `Transcript::push_agent_event`：

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\chat.rs`
  - `AgentEvent::CommandUpdated { aggregated_output }` 非空时直接进入 `[stdout]`。
  - `AgentEvent::ToolCallCompleted { output }` 完成后直接追加 `[tool-output]`，没有长度限制、类型识别或摘要策略。
  - `AgentEvent::ContextStatus` 直接进入 `[context]`。
  - `AgentEvent::StorageState` 直接进入 `[session]`。
  - `Provider turn started/completed` 这类运行状态通过 `Reasoning` 或 notice 进入主视图。
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs`
  - `render_transcript` 只是渲染 transcript cells，不承担可见性过滤。
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`
  - `push_agent_event` 直接转发到 transcript，缺少 TUI 层 event policy。

结论：v1.7.2 的 transcript 仍然是“事件日志视图”，不是 Codex CLI 风格的“对话和工具时间线视图”。v1.7.3 要补的是事件分类、可见性策略和工具 timeline，而不是继续调整滚动条。

## Codex CLI TUI 参考

参考源码位于本项目内的 `D:\YunXi Agent\vendor\codex-rs\tui`，仅作为行为参考，不作为默认运行依赖。

关键参考点：

- `D:\YunXi Agent\vendor\codex-rs\tui\src\streaming\mod.rs`
  - 上游把流式 markdown 拆成 collector、queue、controller、commit tick，而不是把每个 delta 直接写入 transcript。
- `D:\YunXi Agent\vendor\codex-rs\tui\src\streaming\controller.rs`
  - 使用 stable region 和 mutable tail：稳定内容进入 scrollback，未完成尾部作为 active cell 展示。
  - 完成后用 source-backed cell 固化，避免 resize 后丢失格式。
- `D:\YunXi Agent\vendor\codex-rs\tui\src\history_cell\messages.rs`
  - assistant、reasoning、streaming tail 都是独立 history cell。
  - `AgentMessageCell`、`AgentMarkdownCell`、`StreamingAgentTailCell` 明确区分流式临时内容和最终历史内容。
- `D:\YunXi Agent\vendor\codex-rs\tui\src\history_cell\mcp.rs`
  - MCP tool call 以 compact cell 展示：`Calling`/`Called`、server/tool、简短参数、状态、有限输出。
  - 工具结果通过 `format_and_truncate_tool_result` 做限制，不把无限长原文直接塞进主视图。
- `D:\YunXi Agent\vendor\codex-rs\tui\src\history_cell\exec.rs`
  - background terminal 和 process summary 被整理为历史单元，长命令和最近输出会截断，不逐 token 展开协议。
- `D:\YunXi Agent\vendor\codex-rs\tui\src\app\agent_message_consolidation.rs`
  - 流式 assistant 完成后合并为 canonical markdown cell，主 transcript 保持稳定。

Codex CLI 的核心模式是：主 transcript 展示用户能理解的语义单元，底层协议事件只用于驱动状态机或调试日志。YunXi v1.7.3 应照这个方向改，而不是继续把 `AgentEvent` 一比一打印出来。

## v1.7.3 总目标

YunXi Agent v1.7.3 要把 TUI 主视图从“底层事件日志”升级为“思考链路 + 工具调用链路 + assistant 回复”的用户视图。

成功标准：

- 正常 transcript 中不再出现 `arguments_json`、裸 JSON token、逐 token `[stdout]`。
- 正常 transcript 中不再直接展示完整 skill 文档、MCP 长返回、shell 长 stdout。
- `context`、`session`、provider turn bookkeeping 默认不进入主 transcript。
- reasoning 只展示模型/运行时提供的用户可读思考摘要或进度，不展示协议噪声。
- 工具调用以紧凑链路展示：申请审批、审批结果、开始执行、完成/失败、必要的短摘要。
- 原始事件仍可通过 debug/detail 通道查看，便于排障。
- plain、JSON、JSONL 输出格式不受 TUI 过滤影响。
- 默认运行路径继续不依赖 `vendor/codex-rs`、`codex-*` crate 或 `yunxi-agent-codex`。

## 非目标

- 不在 v1.7.3 重做 viewport、scrollbar、mouse capture，这些属于 v1.7.2 已完成能力。
- 不改模型 provider 协议兼容层，除非为了标记工具调用事件类型必须补充字段。
- 不删除底层原始事件。原始事件应进入 debug/detail/log，而不是丢失。
- 不把上游 Codex TUI crate 纳入默认依赖。可以参考源码结构和行为，但实现必须落在 YunXi 自有 crate。
- 不在开发报告阶段创建 `v1.7.3` tag。只有源码实现和统一验证通过后才发布新 tag。

## 推荐架构

### 1. TUI Event Filter

新增 `D:\YunXi Agent\crates\yunxi-agent-tui\src\event_filter.rs`，把 `AgentEvent` 映射为 TUI 专用事件：

- `TimelineItem::AssistantDelta`
- `TimelineItem::ReasoningDelta`
- `TimelineItem::ToolStarted`
- `TimelineItem::ToolApprovalRequested`
- `TimelineItem::ToolApprovalCompleted`
- `TimelineItem::ToolCompleted`
- `TimelineItem::ToolOutputSummary`
- `TimelineItem::Warning`
- `TimelineItem::Error`
- `TimelineItem::DebugOnly`
- `TimelineItem::Suppress`

可见性策略：

- `UserVisible`：进入主 transcript。
- `DebugOnly`：只进入 debug buffer。
- `DetailsOnly`：主视图显示一行摘要，完整内容进入 detail buffer。
- `Suppress`：不渲染，但可保留计数。

默认主视图只允许：

- 用户消息。
- assistant 回复。
- reasoning summary/delta。
- 工具 lifecycle 摘要。
- warning/error/cancelled。
- 用户需要感知的 approval、sandbox denial、tool failure。

默认隐藏：

- `CommandUpdated` 中的 raw aggregated output，尤其是工具协议 JSON。
- `ToolCallCompleted.output` 的完整原文。
- `ContextStatus`、`StorageState`、`TurnMetadata`、`ThreadState`、`Provider turn started/completed`。
- sandbox policy 的 advisory-only 细节，除非发生拒绝或需要审批。

### 2. Tool Timeline

新增 `D:\YunXi Agent\crates\yunxi-agent-tui\src\timeline.rs`，把同一次工具调用的多条事件合并为一个紧凑 cell。

建议数据结构：

```rust
pub(crate) struct ToolTimelineEntry {
    pub id: Option<String>,
    pub name: String,
    pub phase: ToolPhase,
    pub approval: Option<ApprovalSummary>,
    pub command: Option<String>,
    pub status: Option<String>,
    pub output_summary: Option<String>,
    pub debug_ref: Option<usize>,
}

pub(crate) enum ToolPhase {
    Requested,
    ApprovalRequired,
    Approved,
    Running,
    Completed,
    Failed,
    Cancelled,
}
```

展示效果目标：

- `tool using-superpowers · approval required`
- `tool using-superpowers · approved`
- `tool using-superpowers · running`
- `tool using-superpowers · completed`
- `tool using-superpowers · output hidden, 94 lines available in details`

对于 skill 类工具，主视图不展示完整 `SKILL.md`，只展示：

- `loaded skill using-superpowers`
- `skill output hidden, available in debug details`

### 3. Debug And Detail Buffer

新增 `D:\YunXi Agent\crates\yunxi-agent-tui\src\debug.rs`，保存被隐藏或摘要化的原始事件。

建议能力：

- 环境变量：`YUNXI_TUI_DEBUG_EVENTS=1` 时显示 debug 事件。
- slash command：`/debug events on|off` 切换调试事件显示。
- slash command：`/details` 查看最近一次工具调用的完整输出摘要或原文引用。
- 主视图只显示 `details available`，不直接展开长内容。

这能解决两个冲突：普通用户不被协议噪声干扰，开发者仍能追查工具调用失败的完整上下文。

### 4. Transcript Cell 重构

调整 `D:\YunXi Agent\crates\yunxi-agent-tui\src\chat.rs`：

- `Transcript::push_agent_event` 不再直接 match 所有 `AgentEvent`。
- 改为调用 `event_filter.classify(event)`。
- `HistoryCell::Event` 继续保留，但只用于用户可读 notice。
- 新增或扩展 `HistoryCell::ToolTimeline`、`HistoryCell::ToolOutputSummary`、`HistoryCell::Debug`。
- `Reasoning` cell 只接收 reasoning 内容，不接收 provider bookkeeping。

调整 `D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs`：

- 正常模式渲染 user-visible cells。
- debug 模式追加 debug cells 或在 footer/header 显示 debug 状态。
- 对长 output summary 做行数和字符数限制，默认最多 6 行、每行按 viewport wrap。

### 5. Event Source Hygiene

检查 `D:\YunXi Agent\crates\yunxi-agent-cli\src\tui\mod.rs` 与 runtime event mapping：

- 如果 provider 把工具调用 JSON 当作 shell stdout 发出，需要在 CLI/TUI adapter 层标记为 `DebugOnly`。
- 如果 runtime 能提供 tool call id、name、arguments、status，TUI 应使用结构化字段，而不是解析 stdout。
- approval、sandbox、tool completed 要尽量通过结构化 `AgentEvent` 更新 timeline。

## Phase 1：问题复现 fixture 与红线测试

目标：先用截图中的噪声模式建立 regression fixture，防止 v1.7.3 实现后复发。

建议新增测试：

- `D:\YunXi Agent\crates\yunxi-agent-tui\tests\event_filter_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\tests\tui_snapshot_tests.rs`

断言：

- normal transcript 不包含 `arguments_json`。
- normal transcript 不包含裸 `{`、`\"name\"` 这类工具协议 token 行。
- normal transcript 不包含 `EXTREMELY-IMPORTANT` 或完整 skill 文档正文。
- normal transcript 不包含 `[context] tokens=`。
- normal transcript 不包含 `[session] rollout_items=...`。
- normal transcript 包含 `tool using-superpowers`、`approval required`、`approved`、`completed`。
- debug 模式可以看到被隐藏事件的摘要引用。

## Phase 2：实现 Event Filter

目标：建立从 `AgentEvent` 到 TUI 可见性策略的唯一入口。

建议改动：

- Create：`D:\YunXi Agent\crates\yunxi-agent-tui\src\event_filter.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\lib.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\chat.rs`

事件策略：

- `Message`：UserVisible assistant。
- `Reasoning`：如果是 provider lifecycle 文案，DebugOnly；否则 UserVisible reasoning。
- `CommandStarted`：UserVisible tool/shell started。
- `CommandUpdated`：默认 DetailsOnly 或 DebugOnly，不进入主 transcript。
- `CommandCompleted`：UserVisible completed summary。
- `ToolCallStarted`：UserVisible timeline started。
- `ToolCallCompleted`：UserVisible completed summary；`output` 进入 DetailsOnly。
- `ContextStatus`：DebugOnly。
- `StorageState`：DebugOnly。
- `SandboxAttempt`：成功的 advisory-only 为 DebugOnly；denied/escalation 为 UserVisible。
- `ApprovalRequested/Completed`：UserVisible timeline。
- `Warning/Error/ProviderError/Cancelled`：UserVisible。

## Phase 3：实现 Tool Timeline Cell

目标：把工具调用链路从零散日志变成单元化 timeline。

建议改动：

- Create：`D:\YunXi Agent\crates\yunxi-agent-tui\src\timeline.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\chat.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs`

行为：

- 同名或同 id 工具调用连续事件合并到同一个 cell。
- approval 与 sandbox 状态附着到对应工具 cell。
- completed 后显示短状态，不自动展开输出。
- failure 时显示错误摘要，完整输出进 details。

## Phase 4：实现 Output Summarizer

目标：长工具输出只展示安全摘要。

建议改动：

- Create：`D:\YunXi Agent\crates\yunxi-agent-tui\src\output_summary.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\event_filter.rs`

策略：

- 空输出不展示。
- 单行短输出可展示一行摘要。
- 多行输出默认展示前 3 行和后 1 行，中间用 `...` 标记省略。
- skill 文档、Markdown 长文档、JSON 大对象默认不展开，只显示类型、行数、字符数。
- 检测 secret 形态时主视图和 debug 视图都必须脱敏。

## Phase 5：Debug/Details 模式

目标：保留排障能力。

建议改动：

- Create：`D:\YunXi Agent\crates\yunxi-agent-tui\src\debug.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\host.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-cli\src\interactive.rs`

交互：

- `/debug events on`
- `/debug events off`
- `/details`
- `/details <id>`

默认：

- debug off。
- 主视图不显示 raw event。
- footer 或 subheader 可显示 `debug events hidden=N`，但不要刷屏。

## Phase 6：统一验证与发布策略

后续实现阶段仍遵守用户硬性约束：

- 先按报告完成整体构建。
- 构建过程中不在单点反复测试。
- 全部迁移和构建完成后统一验证。
- 每个正式版本创建新 tag，本阶段实现完成后应创建 `v1.7.3`。
- 旧 tag 不删除、不移动。
- GitHub 读写和推送全部走 REST API。
- `C:\Users\admin\Desktop\api.txt` 中的 DeepSeek/GitHub key 只能用于本地测试和 API 发布，必须隐私化，不输出、不提交、不写入日志。
- 任务结束追加 `C:\Users\admin\Desktop\YunXi Agent开发日志.md`。
- 发布后执行 `cargo clean` 并确认 `target_exists=False`。

统一验证建议：

- `cargo fmt`
- `cargo fmt -- --check`
- `cargo test`
- `cargo check --workspace`
- `cargo build -p yunxi-agent-cli --release --bins`
- `target\release\yunxi.exe --version` 输出 `yunxi 1.7.3`
- `target\release\yunxi-agent-cli.exe --version` 输出 `yunxi 1.7.3`
- TUI event filter snapshot：不出现 `arguments_json`、完整 skill 文档、raw JSON token。
- TUI tool timeline snapshot：出现 approval、running、completed 的紧凑链路。
- DeepSeek stream live smoke：确认 assistant 输出仍正常流式展示。
- DeepSeek tool-call smoke：确认工具调用链路可读，raw output 不污染主视图。
- plain interactive smoke：`--no-tui` 不受影响。
- JSON/JSONL smoke：结构化事件不被 TUI filter 改写。
- default dependency scan：`codex-*`、`vendor/codex-rs`、`yunxi-agent-codex` 命中 0。
- secret scan。
- `git diff --check`
- `codegraph sync "D:\YunXi Agent"`
- 安装脚本和 PATH smoke。
- GitHub REST API 发布 master 和 annotated tag `v1.7.3`。
- 旧 tag 解引用目标校验不变。
- `cargo clean`
- `target_exists=False`

## 风险与边界

- 不要把 raw output 直接删除。调试模式必须能追踪工具失败原因。
- 不要把 provider lifecycle 误当 reasoning 展示。只有用户可读 reasoning summary/delta 进入主视图。
- 不要为了过滤 `stdout` 而破坏 shell 命令真实输出。shell 工具输出可以摘要展示，完整内容进 details。
- 不要影响 JSON/JSONL 自动化输出。TUI filter 只作用于 TUI transcript。
- 不要恢复上游 Codex 依赖。参考 `vendor/codex-rs` 的结构和行为，代码仍写在 YunXi 自有 crate。

## v1.7.3 完成后的预期状态

完成 v1.7.3 后，YunXi Agent TUI 应达到更接近 Codex CLI 的主视图体验：

- 用户看到的是对话、思考摘要、工具调用时间线和最终回答。
- 工具协议 JSON、stdout token、skill 文档全文不再污染 transcript。
- 工具调用链路清楚可追踪，但不刷屏。
- debug/detail 通道保留完整排障能力。
- YunXi Agent 继续保持完全自主运行，不依赖上游 Codex CLI 源码。

## 实施结果记录

本轮 v1.7.3 已按报告完成构建：

- 新增 `crates/yunxi-agent-tui/src/event_filter.rs`，把 `AgentEvent` 分类为 assistant、reasoning、tool timeline、notice、warning、error、debug-only 和 suppress。
- 新增 `crates/yunxi-agent-tui/src/timeline.rs`，将 shell/tool/MCP/approval/escalation 生命周期合并为紧凑工具时间线 cell，保留 approval required、approved、running、completed/failed 等步骤。
- 新增 `crates/yunxi-agent-tui/src/output_summary.rs`，对工具参数、协议 JSON、长 stdout、skill 文档和疑似密钥内容做摘要、隐藏和脱敏。
- 新增 `crates/yunxi-agent-tui/src/debug.rs`，保存被隐藏的原始事件和工具输出，提供可按 id 查看且已脱敏的 detail buffer。
- 更新 `crates/yunxi-agent-tui/src/chat.rs`，`Transcript::push_agent_event` 不再直接打印所有事件，而是消费过滤后的语义事件；`CommandUpdated`、`ContextStatus`、`StorageState` 和成功 advisory sandbox 默认不进入主 transcript。
- 更新 `crates/yunxi-agent-tui/src/render.rs`，支持 `HistoryCell::Tool` 和 `HistoryCell::Debug` 渲染。
- 更新 `crates/yunxi-agent-tui/src/app.rs`、`host.rs`，加入 debug 状态、`set_debug_events` 和 `show_details`。
- 更新 `crates/yunxi-agent-cli/src/commands.rs`、`interactive.rs`、`render.rs`、`tui/mod.rs`，加入 `/debug events on|off` 和 `/details [id]`，plain renderer 保持脚本友好的文本提示，TUI renderer 接入真实 debug/detail buffer。
- 工作区版本升级到 `1.7.3`，README、CLI about、版本测试和 extraction status 同步更新。

统一验证结果：

- `cargo fmt`：通过。
- `cargo fmt -- --check`：通过。
- `cargo test`：通过，workspace unit tests、integration tests、doc tests 全部 0 failure。
- `cargo check --workspace`：通过。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- `target\release\yunxi.exe --version`：`yunxi 1.7.3`。
- `target\release\yunxi-agent-cli.exe --version`：`yunxi 1.7.3`。
- `target\release\yunxi.exe --offline "v1.7.3 offline smoke"`：通过。
- plain interactive debug smoke：`/debug events on`、`/details`、`/debug events off`、`/exit` 通过。
- TUI event filter 专项测试：`cargo test -p yunxi-agent-tui event_filter_hides_protocol_stdout_context_and_long_skill_output` 通过，确认正常 transcript 不出现 `arguments_json`、完整 skill 文档、context tokens，并保留工具 timeline。
- `cargo tree -p yunxi-agent-cli` 默认依赖扫描：通过，未命中 `codex`、`vendor`、`yunxi-agent-codex`。
- owned-source secret scan（排除 `vendor/`、`extracted/`、`target/`、`.git/`、`.codegraph/`）：通过，无命中。
- `git diff --check`：通过，仅有 Windows LF-to-CRLF 提示。
- DeepSeek live streaming smoke：通过，model=`deepseek-v4-flash`，54 行 JSONL，`secret_leak_detected=False`。
- DeepSeek live non-stream smoke：通过，model=`deepseek-v4-flash`，19 行 JSONL，`secret_leak_detected=False`。
- DeepSeek interactive live smoke：通过，banner、assistant marker、normal exit 均检测成功，`secret_leak_detected=False`。
- `codegraph sync "D:\YunXi Agent"`：通过，同步 15 个 changed files。
- `codegraph status "D:\YunXi Agent"`：通过，index is up to date。
- `scripts\install\install-yunxi.ps1 -AddToPath -SkipBuild`：通过。
- PATH smoke：`yunxi --version` 和 `yunxi-agent-cli --version` 均输出 `1.7.3`，`yunxi --offline "installed path v1.7.3 smoke"` 通过。
- 本轮 smoke 生成的 root `.yunxi` 运行产物已清理。

发布策略：

- 后续发布本轮实现时创建 release commit 和新的 annotated tag `v1.7.3`。
- 旧 tag 不删除、不移动。
- GitHub 发布继续全部走 REST API。
- 发布后执行 `cargo clean` 并确认 `target_exists=False`。

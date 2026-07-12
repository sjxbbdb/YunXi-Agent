# YunXi Agent v1.7.1 Codex TUI Parity Development Report

生成时间：2026-07-12 19:44:35 +08:00

## 背景

YunXi Agent v1.7.0 已经建立了基础 TUI 外壳：真实 terminal 默认进入 alternate screen，事件区、header 和输入区由 `ratatui` 渲染，pipe/JSON/JSONL 仍保持 plain 输出。但实际交互截图暴露出 v1.7.0 的核心问题：它不是 Codex CLI TUI 的迁移版，只是一个轻量 renderer 原型。

当前截图中出现的问题包括：

- reasoning delta 被逐 token 渲染成多行 `[reasoning]`，中文内容被拆碎，阅读体验接近日志流，不像正式 Agent TUI。
- approval 请求没有被 TUI 接管成 modal/overlay，而是穿透到底部行式 `approve? y/N:` prompt，破坏 alternate screen 布局。
- event area 和 bottom prompt 的边界不清晰，approval 标题与边框互相挤压。
- TUI 没有 Codex CLI 那种 transcript/history cell、active streaming tail、bottom pane composer、approval overlay、request_user_input view、footer/status indicator 的分层。
- 当前 renderer 只是把 `AgentEvent` 映射成字符串，没有真正的事件调度、帧请求、resize reflow、markdown streaming、table holdback 和输入焦点模型。

因此 v1.7.1 不应继续在现有三文件 TUI 外壳上修补，而应直接参照 Codex CLI 的 TUI 源码，按模块照搬，再替换为 YunXi 自主 runtime/provider/tool/storage 事件接口。

## 上游源码核查

用户指定的上游源码目录为：

- `D:\源码\codex`

核查结果：

- `D:\源码\codex` 没有 `.codegraph/`，因此无法对上游目录使用 CodeGraph。
- `D:\源码\codex\codex-rs` 当前工作树为空。
- `git -C D:\源码\codex status --short --branch` 显示 `codex-rs/...` 大量文件处于 deleted 状态。
- 但 Git object 中仍保留完整上游源码，可通过 `git -C D:\源码\codex show HEAD:codex-rs/tui/...` 读取，不需要 checkout/reset 上游工作树。
- `git -C D:\源码\codex ls-tree -r --name-only HEAD codex-rs/tui/src` 显示上游 TUI `src` 下约 901 个文件/快照。
- 上游当前 HEAD 摘要：`f1affbac5e core: support extension-owned turn items (#31283)`。

v1.7.1 开发必须遵守：

- 不对 `D:\源码\codex` 执行 `git checkout`、`git reset` 或其他会恢复/覆盖上游工作树的操作。
- 从上游读取 TUI 源码时使用 `git show HEAD:<path>` 或 `git archive` 到临时目录。
- 上游 Codex 源码只作为迁移输入，最终 YunXi 代码必须进入 `D:\YunXi Agent`，不能运行时依赖 `D:\源码\codex`。

## Codex TUI 关键模块

上游 `codex-rs/tui` 的结构说明 v1.7.1 要迁移的是完整终端应用层，而不是单个 renderer。

### 1. Terminal Host

上游核心文件：

- `codex-rs/tui/src/tui.rs`
- `codex-rs/tui/src/tui/event_stream.rs`
- `codex-rs/tui/src/tui/frame_requester.rs`
- `codex-rs/tui/src/tui/frame_rate_limiter.rs`
- `codex-rs/tui/src/tui/keyboard_modes.rs`
- `codex-rs/tui/src/tui/terminal_stderr.rs`
- `codex-rs/tui/src/custom_terminal.rs`
- `codex-rs/tui/src/terminal_probe.rs`

能力点：

- raw mode、alternate screen、bracketed paste、focus change、keyboard enhancement。
- `TuiEventStream` 合并 keyboard、paste、resize、draw tick、focus 和 shutdown。
- `FrameRequester` 与 frame rate limiter 控制重绘节奏。
- terminal stderr guard 保证 TUI 不被普通 stderr 污染。
- viewport change clear、resize-sensitive draw、history insertion 与 active viewport 分离。

### 2. App Event Orchestration

上游核心文件：

- `codex-rs/tui/src/app.rs`
- `codex-rs/tui/src/app_event.rs`
- `codex-rs/tui/src/app_event_sender.rs`
- `codex-rs/tui/src/app/thread_events.rs`
- `codex-rs/tui/src/app/event_dispatch.rs`
- `codex-rs/tui/src/app/input.rs`
- `codex-rs/tui/src/app/history_ui.rs`

能力点：

- `App` 作为 TUI 总状态，接收 backend/server/runtime notification，再分发给 ChatWidget、BottomPane、status、overlay。
- `AppEvent` 是本地 UI 事件总线，隔离 key/input、runtime event、approval response、modal response、draw request。
- runtime 事件不会直接写屏，而是转成 UI state mutation，再由 frame render。

### 3. ChatWidget And Transcript

上游核心文件：

- `codex-rs/tui/src/chatwidget.rs`
- `codex-rs/tui/src/chatwidget/protocol.rs`
- `codex-rs/tui/src/chatwidget/protocol_requests.rs`
- `codex-rs/tui/src/chatwidget/rendering.rs`
- `codex-rs/tui/src/chatwidget/interrupts.rs`
- `codex-rs/tui/src/chatwidget/input_flow.rs`
- `codex-rs/tui/src/chatwidget/input_queue.rs`
- `codex-rs/tui/src/chatwidget/command_lifecycle.rs`
- `codex-rs/tui/src/history_cell.rs`
- `codex-rs/tui/src/thread_transcript.rs`

能力点：

- transcript 由 `HistoryCell` 组成，不是裸字符串日志。
- assistant message、reasoning、command lifecycle、MCP、approval、warnings、errors 都有不同 cell。
- streaming 时维护 active cell/tail，完成后固化到 transcript。
- Ctrl+C、Esc、队列输入、运行中 turn、idle prompt 有明确交互语义。

### 4. Bottom Pane, Composer, Approval Overlay

上游核心文件：

- `codex-rs/tui/src/bottom_pane/mod.rs`
- `codex-rs/tui/src/bottom_pane/bottom_pane_view.rs`
- `codex-rs/tui/src/bottom_pane/chat_composer.rs`
- `codex-rs/tui/src/bottom_pane/chat_composer_history.rs`
- `codex-rs/tui/src/bottom_pane/textarea.rs`
- `codex-rs/tui/src/bottom_pane/footer.rs`
- `codex-rs/tui/src/bottom_pane/approval_overlay.rs`
- `codex-rs/tui/src/bottom_pane/request_user_input/mod.rs`
- `codex-rs/tui/src/bottom_pane/request_user_input/render.rs`
- `codex-rs/tui/src/bottom_pane/request_user_input/layout.rs`
- `codex-rs/tui/src/bottom_pane/slash_commands.rs`
- `codex-rs/tui/src/bottom_pane/file_search_popup.rs`
- `codex-rs/tui/src/bottom_pane/mentions_v2/mod.rs`

能力点：

- 输入不是 `read_line`，而是 TUI 内部 textarea/composer。
- 支持多行、history search、slash popup、mention/file search、paste burst、footer hint。
- approval、permissions、apply_patch、MCP elicitation、request_user_input 都在 bottom pane 里以 overlay/view 形式处理。
- approval queue 能同时处理多个待审批请求，不污染主 transcript。

### 5. Markdown Streaming And Rendering

上游核心文件：

- `codex-rs/tui/src/markdown.rs`
- `codex-rs/tui/src/markdown_render.rs`
- `codex-rs/tui/src/markdown_stream.rs`
- `codex-rs/tui/src/markdown_text_merge.rs`
- `codex-rs/tui/src/streaming/controller.rs`
- `codex-rs/tui/src/streaming/chunking.rs`
- `codex-rs/tui/src/streaming/commit_tick.rs`
- `codex-rs/tui/src/streaming/table_holdback.rs`
- `codex-rs/tui/src/transcript_reflow.rs`
- `codex-rs/tui/src/wrapping.rs`

能力点：

- markdown delta 先缓冲，再按 newline/commit tick/holdback 策略固化。
- reasoning 和 assistant 文本不会逐 token 打满屏幕。
- table、code block、wide char、中文、resize reflow 有专门逻辑。
- live tail 与 stable transcript 分离，减少闪烁和重复。

### 6. Snapshot And VT100 Tests

上游核心文件：

- `codex-rs/tui/src/test_backend.rs`
- `codex-rs/tui/src/test_support.rs`
- `codex-rs/tui/tests/test_backend.rs`
- `codex-rs/tui/tests/suite/vt100_history.rs`
- `codex-rs/tui/tests/suite/vt100_live_commit.rs`
- `codex-rs/tui/tests/suite/resize_reflow.rs`
- `codex-rs/tui/src/**/snapshots/*.snap`

能力点：

- 用 `ratatui`/`vt100` snapshot 验证真实终端画面。
- 重点覆盖 approval modal、composer footer、markdown streaming、resize reflow、status indicator。
- 这是 v1.7.1 防止“看起来能跑但屏幕乱掉”的关键。

## v1.7.1 总目标

YunXi Agent v1.7.1 的目标是：直接参照 Codex CLI TUI 源码，迁移一套 YunXi 自主化 TUI runtime，让 `yunxi` 在真实终端中具备 Codex CLI 风格的交互式 Agent 能力，同时继续保持不依赖上游源码、不污染 JSON/JSONL、不提交任何 secret。

核心成功标准：

- `yunxi` 进入交互模式后，不再显示 v1.7.0 风格的简陋 `Events` 框。
- reasoning delta 被合并为稳定 reasoning block 或状态摘要，不再逐 token 刷屏。
- approval 不再穿透成 `approve? y/N:` 行式 prompt，而是在 TUI bottom pane 中显示审批 overlay。
- 输入区由 composer/textarea 接管，支持多行、history、slash command、paste、footer hint。
- assistant markdown 输出具备稳定 streaming 和 transcript 固化，不出现重复、截断、表格错乱。
- Ctrl+C/Esc/resize/focus/paste 有 Codex 风格语义。
- plain、one-shot、`--json`、`--jsonl`、sessions 子命令保持 v1.7 兼容行为。
- 最终源码在 `D:\YunXi Agent` 内自主运行，不运行时读取或依赖 `D:\源码\codex`。

## 推荐架构

v1.7.1 推荐新增独立 crate：

- `crates/yunxi-agent-tui`

原因：

- 上游 TUI 规模大，直接塞回 `yunxi-agent-cli/src/tui` 会让 CLI crate 过重。
- 独立 crate 可以隔离迁移中的 Codex-to-YunXi 适配层。
- 后续 v1.8+ 可以继续补齐 Codex TUI 能力，而不会干扰 headless CLI、JSON、runtime core。

建议结构：

- `crates/yunxi-agent-tui/src/lib.rs`
  - YunXi TUI public facade。
- `crates/yunxi-agent-tui/src/host/`
  - 迁移 `tui.rs`、event stream、frame requester、terminal guard。
- `crates/yunxi-agent-tui/src/app/`
  - YunXi TUI app state 与 event dispatch。
- `crates/yunxi-agent-tui/src/chat/`
  - 迁移 ChatWidget、history cell、transcript、runtime event adapter。
- `crates/yunxi-agent-tui/src/bottom_pane/`
  - 迁移 composer、textarea、footer、approval overlay、request_user_input。
- `crates/yunxi-agent-tui/src/streaming/`
  - 迁移 markdown streaming、commit tick、chunking、table holdback。
- `crates/yunxi-agent-tui/src/render/`
  - markdown render、wrapping、style、width、status indicator。
- `crates/yunxi-agent-tui/src/adapter/`
  - `AgentEvent` -> `YunxiTuiEvent`。
  - approval response -> `InteractiveApprovalResponse`。
  - request_user_input response -> tool runtime response。

`yunxi-agent-cli` 只负责：

- CLI 参数解析。
- terminal/plain/JSON/JSONL mode selection。
- one-shot/headless paths。
- interactive TUI 模式下调用 `yunxi_agent_tui::run_interactive(...)`。

## 源码迁移策略

### 原则 1：先整体搬核心，再 YunXi 化

不能再围绕 v1.7 的 `TuiApp` 小结构修补。v1.7.1 应先把上游 TUI 的核心模块按目录迁入，再替换类型、imports、事件适配。

优先搬迁：

- terminal host/event loop。
- ChatWidget/history/transcript。
- bottom pane composer/textarea/footer。
- approval overlay/request_user_input view。
- markdown streaming/rendering。
- test backend/snapshot 基础设施。

暂缓搬迁或降级 stub：

- cloud/account/login。
- update prompt。
- marketplace/plugin share。
- app server remote daemon。
- desktop-specific features。
- pet/image/ambient visual。
- external app surfaces。

这些模块不是解决当前 TUI 崩坏的核心，可以在 v1.7.1 中用 YunXi stub 或 feature-off adapter 保持编译通过。

### 原则 2：保留 Codex 结构，改掉 Codex 依赖

迁移后的源码可以保留上游文件边界和局部函数结构，但必须完成命名和依赖替换：

- `codex_tui` -> `yunxi_agent_tui`
- `codex_*` crate imports -> `yunxi_agent_*` crate 或本 crate adapter
- `codex_app_server_protocol::ServerNotification` -> `YunxiTuiEvent` / `AgentEvent`
- `codex_config` -> `yunxi_agent_config` / `AgentConfig`
- `codex_protocol` -> `yunxi_agent_protocol`
- `codex_rollout/state` -> `yunxi_agent_storage`
- `CODEX_CLI_VERSION` -> YunXi version source

最终依赖扫描仍必须满足：

- default CLI graph 中 `codex-*` 命中 0。
- `vendor/codex-rs` 命中 0。
- `yunxi-agent-codex` 命中 0。

### 原则 3：TUI 使用 YunXi runtime 事件，不反向污染 runtime

不要把 TUI 的复杂状态塞回 `yunxi-agent-runtime`。runtime 继续发 `AgentEvent`/stream events，TUI crate 负责适配：

```text
yunxi-agent-runtime
  -> AgentEvent stream
  -> yunxi-agent-tui::adapter::YunxiTuiEvent
  -> App event dispatch
  -> ChatWidget / BottomPane state mutation
  -> render frame
```

approval/request_user_input 走反向 channel：

```text
BottomPane overlay decision
  -> TuiInteractiveHost response channel
  -> AgentRuntime interactive bridge
  -> ToolRuntime / provider tool result
```

## 开发阶段划分

### Phase 1：上游 TUI 源码快照与迁移清单

- 从 `D:\源码\codex` 的 Git object 读取 `codex-rs/tui`，不修改上游工作树。
- 生成迁移清单：core host、chat、bottom pane、streaming、render、tests。
- 在 `docs/extraction-status.md` 记录 v1.7.1 使用的上游 commit：`f1affbac5e`。
- 建立 `crates/yunxi-agent-tui` 空 crate 与 dependency shell。

### Phase 2：Terminal Host Port

- 迁移 `tui.rs` 中 terminal modes、bracketed paste、focus change、keyboard enhancement、frame requester、event stream。
- 替换 v1.7.0 的 `TuiInteractiveRenderer` 为真正的 TUI host。
- 保留 `--no-tui` plain fallback。

### Phase 3：ChatWidget 与 Transcript Port

- 迁移 ChatWidget 的核心状态、history cell、rendering、protocol event dispatch。
- 建立 `AgentEvent` 到 transcript cell 的映射。
- reasoning delta 合并进 active reasoning cell，不再逐 token 写 event log。
- assistant message delta 合并为 active markdown stream。

### Phase 4：Markdown Streaming Port

- 迁移 `markdown_stream`、`streaming/controller`、`chunking`、`commit_tick`、`table_holdback`。
- 接入 YunXi provider streaming events。
- 支持中文、宽字符、code block、table、resize reflow。

### Phase 5：Bottom Pane Composer Port

- 迁移 textarea、composer、history、footer、slash popup、paste burst。
- 输入从 `ReedlineInput` 改为 TUI composer 内部事件。
- `/help`、`/status`、`/session` 等 YunXi commands 接入 composer dispatch。

### Phase 6：Approval And Request User Input Overlay

- 迁移 `approval_overlay` 与 `request_user_input`。
- Shell approval、patch approval、MCP elicitation、tool request_user_input 都进入 bottom pane modal。
- 禁止真实 TUI 中出现行式 `approve? y/N:`。
- 保留 plain mode 的行式 approval，用于 pipe 和测试。

### Phase 7：TUI Snapshot And VT100 Test Harness

- 迁移 TestBackend/VT100 基础测试。
- 新增 YunXi snapshot：
  - idle composer。
  - running turn。
  - reasoning delta 合并。
  - assistant markdown streaming。
  - approval modal。
  - request_user_input。
  - resize reflow。
  - Ctrl+C footer hint。
- 这些测试作为统一验证门的一部分，只在构建完成后统一运行。

### Phase 8：版本、文档、发布、日志

- 版本升级到 `1.7.1`。
- README、extraction status、v1.7.1 报告同步更新。
- 安装本机 PATH 版本。
- 创建 release commit 与 annotated tag `v1.7.1`。
- 通过 GitHub REST API 发布 master 和 tag。
- 旧 tag 不删除、不移动。
- `cargo clean` 并确认 `target_exists=False`。
- 追加桌面开发日志。

## 关键文件范围

新增：

- `crates/yunxi-agent-tui/Cargo.toml`
- `crates/yunxi-agent-tui/src/lib.rs`
- `crates/yunxi-agent-tui/src/host/mod.rs`
- `crates/yunxi-agent-tui/src/host/event_stream.rs`
- `crates/yunxi-agent-tui/src/host/frame_requester.rs`
- `crates/yunxi-agent-tui/src/host/frame_rate_limiter.rs`
- `crates/yunxi-agent-tui/src/app/mod.rs`
- `crates/yunxi-agent-tui/src/app/event.rs`
- `crates/yunxi-agent-tui/src/app/event_sender.rs`
- `crates/yunxi-agent-tui/src/chat/mod.rs`
- `crates/yunxi-agent-tui/src/chat/protocol.rs`
- `crates/yunxi-agent-tui/src/chat/rendering.rs`
- `crates/yunxi-agent-tui/src/chat/history_cell.rs`
- `crates/yunxi-agent-tui/src/chat/transcript.rs`
- `crates/yunxi-agent-tui/src/bottom_pane/mod.rs`
- `crates/yunxi-agent-tui/src/bottom_pane/chat_composer.rs`
- `crates/yunxi-agent-tui/src/bottom_pane/textarea.rs`
- `crates/yunxi-agent-tui/src/bottom_pane/footer.rs`
- `crates/yunxi-agent-tui/src/bottom_pane/approval_overlay.rs`
- `crates/yunxi-agent-tui/src/bottom_pane/request_user_input.rs`
- `crates/yunxi-agent-tui/src/streaming/mod.rs`
- `crates/yunxi-agent-tui/src/streaming/controller.rs`
- `crates/yunxi-agent-tui/src/streaming/chunking.rs`
- `crates/yunxi-agent-tui/src/streaming/commit_tick.rs`
- `crates/yunxi-agent-tui/src/streaming/table_holdback.rs`
- `crates/yunxi-agent-tui/src/markdown_stream.rs`
- `crates/yunxi-agent-tui/src/adapter/mod.rs`
- `crates/yunxi-agent-tui/src/test_backend.rs`
- `crates/yunxi-agent-tui/tests/tui_snapshots.rs`

修改：

- `Cargo.toml`
- `Cargo.lock`
- `crates/yunxi-agent-cli/Cargo.toml`
- `crates/yunxi-agent-cli/src/main.rs`
- `crates/yunxi-agent-cli/src/interactive.rs`
- `crates/yunxi-agent-cli/src/terminal_mode.rs`
- `crates/yunxi-agent-cli/src/tui/*`（v1.7 原型应删除、替换或降级为 compatibility shim）
- `crates/yunxi-agent-cli/tests/cli_tests.rs`
- `crates/yunxi-agent-cli/tests/jsonl_tests.rs`
- `README.md`
- `docs/extraction-status.md`
- `docs/reports/2026-07-12-yunxi-agent-v1-7-1-codex-tui-parity-development-report.md`
- `docs/superpowers/plans/2026-07-12-yunxi-agent-v1-7-1-codex-tui-parity.md`
- `C:\Users\admin\Desktop\YunXi Agent开发日志.md`

## 统一验证门

构建全部完成后统一执行，不在构建过程中反复卡点：

- `cargo fmt`
- `cargo fmt -- --check`
- `cargo test`
- `cargo check --workspace`
- `cargo check --workspace --target x86_64-unknown-linux-gnu`（如仍被本机 rustup 镜像 404 阻塞，只记录环境原因，不宣称通过）
- `cargo build -p yunxi-agent-cli --release --bins`
- `target\release\yunxi.exe --version` 输出 `yunxi 1.7.1`
- `target\release\yunxi-agent-cli.exe --version` 输出 `yunxi 1.7.1`
- TUI snapshot：idle composer、running status、reasoning merge、assistant markdown、approval overlay、request_user_input、resize reflow。
- VT100 smoke：真实 frame 不重叠、不空白、中文宽字符不拆坏、footer 不覆盖 composer。
- approval smoke：真实 TUI 中不出现 `approve? y/N:` 行式 prompt。
- reasoning smoke：发送 `测试` 后 reasoning 不逐 token 刷满 event log。
- DeepSeek live interactive smoke：终端模式可正常返回模型回答。
- DeepSeek live JSON/JSONL smoke：结构化输出不被 TUI 污染。
- one-shot/offline/plain fallback smoke：保持 v1.7 行为。
- sessions list/show/resume smoke：保持 v1.7 行为。
- dependency scan：default CLI graph 中 `codex-*`、`vendor/codex-rs`、`yunxi-agent-codex` 命中 0。
- owned-source high-confidence secret scan 命中 0。
- `git diff --check`
- `codegraph sync "D:\YunXi Agent"`
- release install 到 `C:\Users\admin\AppData\Local\YunXi Agent\bin`
- PATH `yunxi --version` 输出 `yunxi 1.7.1`
- GitHub REST API 发布并回读核验 `master` 和 `v1.7.1`
- 旧 tag `v1.0.0` 到 `v1.7.0` 不删除、不移动
- `cargo clean`
- `target_exists=False`

## 风险与控制

- 风险：直接迁移上游 TUI 会引入大量 Codex app-server/cloud/update 依赖。
  控制：先迁移 UI 结构与终端交互核心；对非核心功能建立 YunXi stub/feature-off adapter；dependency scan 必须为 0。

- 风险：上游 `D:\源码\codex\codex-rs` 工作树为空且显示 deleted。
  控制：只通过 Git object 读取源码，不修改上游工作树，不执行恢复/重置。

- 风险：迁移量大导致构建中频繁卡点。
  控制：按用户硬约束，构建阶段先整体迁移，构建完成后统一验证；中途只做必要的编译结构整理，不在单点测试上耗时。

- 风险：TUI 污染 JSON/JSONL。
  控制：TUI host 只在真实 terminal interactive 模式启用；JSON/JSONL/pipe/one-shot 继续走 headless renderer。

- 风险：approval modal 阻塞 runtime。
  控制：TUI overlay response 使用 async channel 回传给 existing interactive bridge；plain mode 仍保留行式输入。

- 风险：reasoning/markdown streaming 出现重复或截断。
  控制：迁移 Codex 的 markdown stream collector、stream controller、commit tick、table holdback，并加 snapshot/VT100 回归。

## 版本与发布策略

- 下一版本号：`1.7.1`。
- 发布 tag：`v1.7.1`。
- 旧 tag 不删除、不移动。
- GitHub 读写、发布、核验全部走 REST API。
- 不使用 `git push`、`git fetch`、`git ls-remote`。
- API key/PAT 不打印、不写日志、不写源码、不提交。
- 发布后同步本地 tracking ref。

## 成功判定

v1.7.1 完成后，YunXi Agent 的交互式终端体验应从 v1.7 的原型 TUI 升级为 Codex CLI 风格的正式 TUI：输入、approval、request_user_input、reasoning、assistant markdown、tool lifecycle、footer/status、resize/reflow 都由 TUI 状态机接管。它仍使用 YunXi 自主 runtime/provider/tool/storage，不运行时依赖上游 Codex 源码；模型层保持可替换 provider 接口；自动化输出保持稳定。

最关键的验收标准是用户截图中的问题必须消失：reasoning 不再逐 token 刷屏，approval 不再穿透到 raw prompt，TUI 区域不再互相覆盖。

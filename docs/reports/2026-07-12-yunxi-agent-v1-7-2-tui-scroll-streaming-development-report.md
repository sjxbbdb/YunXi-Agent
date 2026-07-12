# YunXi Agent v1.7.2 TUI Scroll And Streaming Development Report

生成时间：2026-07-12 21:28:00 +08:00

## 背景

YunXi Agent v1.7.1 已经把 v1.7.0 的三文件 TUI 原型替换为独立的 `yunxi-agent-tui` crate，具备 terminal host、transcript、composer、approval overlay、request_user_input overlay 和 reasoning 合并能力。用户实测反馈整体页面和 TUI 展示已经成立，但仍有两个影响真实使用体验的问题：

- 中间 transcript 展示区不能通过鼠标滚轮回看之前内容。
- 流式输出观感还不够接近 Codex CLI：当前内容虽然不会逐 token 变成多行，但刷新节奏、稳定块、live tail、长文本折行和历史固化仍显得偏生硬。

用户提供的参考截图显示了更成熟的 Codex CLI 流式观感：

- 主内容区是完整 transcript，不是纯日志框。
- 历史内容可通过右侧滚动条回看。
- 命令、审批、运行结果和 assistant 输出以稳定块呈现。
- 流式内容在当前块内增量刷新，完成后固化为历史，不应造成整屏跳动。
- 输入区固定在底部，不应因为中间历史滚动或输出增长而抖动。

因此 v1.7.2 的目标不是继续扩展新 agent 能力，而是把 v1.7.1 已经迁出的 TUI 做到可长期使用：可滚动、可回看、流式输出顺滑、resize 后仍保持可读。

## 当前源码核查

本轮使用 CodeGraph 先查询了当前 TUI 相关实现，重点涉及：

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\host.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\chat.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\streaming.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\interactive.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\tui\mod.rs`

确认现状：

- `YunxiTuiApp` 只有 `Transcript` 和 `BottomPane`，没有 scroll offset、follow-tail 状态或 viewport metadata。
- `render_transcript` 每次用 `lines.len().saturating_sub(visible)` 直接显示末尾内容，用户无法停留在历史位置。
- `host.rs` 没有启用 `EnableMouseCapture`，也没有处理 `Event::Mouse(MouseEventKind::ScrollUp/ScrollDown)`。
- `read_prompt` 和 approval/user_input overlay 内部能读取 terminal events，但运行中 turn 的 `tokio::select!` 只监听 runtime events、approval/user_input、Ctrl+C，不能在模型输出期间响应滚轮。
- `streaming.rs` 已有 `MarkdownStreamCollector` 和 fragment merge，但没有接入 transcript 的 stable/live 分离，也没有 frame throttle 或 commit tick。
- `TuiInteractiveRenderer::event` 当前每收到一次 event 就立即 draw，流式 token 多时会造成过于频繁的刷新。

## v1.7.2 总目标

YunXi Agent v1.7.2 要把 TUI 从“能展示”推进到“能阅读、能回看、能长时间工作”的阶段。

核心成功标准：

- 鼠标滚轮可以滚动中间 transcript 区域查看历史内容。
- `PageUp/PageDown/Home/End` 可以辅助滚动 transcript。
- 当用户在底部跟随最新内容时，新输出自动跟随 tail。
- 当用户滚动到历史位置时，新输出不强行跳回底部；底部或 header 显示可见的“有新内容”提示。
- 输入区、approval overlay 和 request_user_input overlay 固定在底部，不被 transcript scroll 影响。
- 流式 assistant/reasoning 输出按稳定块刷新，不逐 token 造成强烈视觉跳动。
- markdown/source 内容按 newline 或短 commit tick 固化到 stable transcript，未完成尾部作为 live tail 渲染。
- resize 后 transcript 重新折行并保持合理 scroll anchor。
- plain、one-shot、JSON、JSONL、sessions、DeepSeek live/offline 路径不被污染。

## 推荐架构

v1.7.2 继续沿用 v1.7.1 的独立 crate 边界，不把 TUI 状态塞回 CLI：

- `yunxi-agent-cli`
  - 只负责参数解析、terminal mode selection、runtime event loop 和 renderer adapter。
  - 不直接处理 scroll offset、mouse wheel、markdown live tail。
- `yunxi-agent-tui`
  - 拥有 scroll state、mouse/key handling、stream collector、frame scheduling、render snapshot。
  - 对 CLI 暴露更小的 facade：`push_agent_event`、`tick`、`handle_terminal_event`、`draw_if_due`、`read_prompt`、`request_approval`、`request_user_input`。

推荐新增或调整模块：

- `crates/yunxi-agent-tui/src/viewport.rs`
  - 管理 transcript viewport、scroll offset、follow-tail、新内容提示、resize anchor。
- `crates/yunxi-agent-tui/src/event.rs`
  - 将 crossterm `Event` 转换为 YunXi TUI action：scroll、resize、submit、cancel、approval decision。
- `crates/yunxi-agent-tui/src/frame.rs`
  - 管理 frame throttle：避免每个 token 都 draw，默认 30-60ms 一帧，重要事件立即绘制。
- `crates/yunxi-agent-tui/src/streaming.rs`
  - 从当前 collector 扩展为 assistant/reasoning stream controller。
  - 区分 stable committed source 和 live tail。
- `crates/yunxi-agent-tui/src/render.rs`
  - 使用 viewport state 渲染 transcript slice。
  - 渲染右侧 scrollbar。
  - 渲染 “new output below” 或 “follow tail” 状态。

## Phase 1：Transcript Viewport 与滚动状态

目标：中间 transcript 区域拥有独立 viewport，可以被鼠标滚轮和键盘滚动。

建议实现：

- 新增 `TranscriptViewport`：
  - `scroll_offset_from_bottom: u16`
  - `follow_tail: bool`
  - `new_content_below: bool`
  - `last_content_height: usize`
  - `last_viewport_height: u16`
- 当 `follow_tail=true`：
  - render 始终显示底部最新内容。
  - 新 transcript cell 到达后保持在底部。
- 当用户向上滚动：
  - `follow_tail=false`
  - 增加 `scroll_offset_from_bottom`
  - 新内容到达只设置 `new_content_below=true`，不强行跳到底。
- 当用户滚到底部或按 End：
  - `follow_tail=true`
  - `scroll_offset_from_bottom=0`
  - `new_content_below=false`

文件：

- Create：`D:\YunXi Agent\crates\yunxi-agent-tui\src\viewport.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs`

测试：

- transcript 超过可视高度后默认显示末尾。
- scroll up 后显示历史行。
- 新内容到达时如果不在 tail，不改变当前历史视口。
- End 恢复 follow-tail。

## Phase 2：Mouse Capture 与运行中滚轮事件

目标：无论是在等待用户输入，还是模型正在流式输出，鼠标滚轮都能滚动 transcript。

建议实现：

- `TerminalGuard::enter()` 启用：
  - `EnableMouseCapture`
  - 退出时 `DisableMouseCapture`
- 在 TUI host 中提供非阻塞 event drain：
  - `drain_terminal_events() -> Vec<YunxiTuiAction>`
  - 使用 `crossterm::event::poll(Duration::ZERO)` 避免阻塞 runtime stream。
- 在 `read_prompt`、approval、user_input 内处理：
  - `MouseEventKind::ScrollUp`
  - `MouseEventKind::ScrollDown`
  - `KeyCode::PageUp`
  - `KeyCode::PageDown`
  - `KeyCode::Home`
  - `KeyCode::End`
- 在 `yunxi-agent-cli/src/interactive.rs` 的运行中 turn loop 增加 TUI tick 分支：
  - TUI renderer 在每次 runtime event 之后 drain terminal events。
  - 使用 `tokio::time::interval` 定期让 TUI 处理 mouse/resize/frame，而不是只靠 runtime event 驱动。

文件：

- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\host.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-cli\src\tui\mod.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-cli\src\interactive.rs`

测试：

- 单元测试覆盖 scroll action 到 viewport state。
- 使用 `ratatui::backend::TestBackend` 渲染长 transcript，验证 scroll 后 buffer 包含历史行而不是最后行。
- 运行中 tick 不影响 plain renderer。

## Phase 3：Scrollbar 与滚动状态显示

目标：参考截图右侧滚动条，让用户知道自己处在 transcript 的哪个位置。

建议实现：

- 使用 `ratatui::widgets::Scrollbar` 和 `ScrollbarState` 渲染右侧滚动条。
- 如果 transcript 行数小于 viewport，不显示 scrollbar 或显示完整 thumb。
- 如果用户离开 tail：
  - footer 或 header 追加 `history view`。
  - 有新内容时显示 `new output below - End to follow`。
- 如果在 tail：
  - footer 保持简洁：`Enter submit | Alt+Enter newline | wheel scroll history`。

文件：

- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`

测试：

- 长 transcript 渲染时 buffer 包含 scrollbar 元素。
- 滚动到历史位置时 footer 状态改变。
- 回到底部后状态恢复。

## Phase 4：流式输出稳定化

目标：把流式输出从“每个 event 立即重画”调整为“stable committed transcript + live tail”。

建议实现：

- 引入 `StreamController`：
  - assistant stream controller。
  - reasoning stream controller。
  - 每个 controller 拥有 `MarkdownStreamCollector`。
- runtime delta 进入 controller：
  - 遇到 newline 时 commit completed source 到 stable cell。
  - 未完成行保留在 live tail。
  - turn completed 时 finalize live tail。
- 对 reasoning：
  - token 合并继续使用 v1.7.1 的 CJK/英文 fragment merge。
  - 可选择只显示当前 active reasoning 摘要，不把每个细碎 reasoning 全部固化为历史。
- 对 assistant：
  - stable part 用普通 assistant cell。
  - live tail 用 active assistant cell，颜色稍弱或带 `assistant*` 标识。
- 对 markdown/code/table：
  - v1.7.2 先做 newline-gated 和 code fence 不破坏缩进。
  - table holdback 可作为 v1.7.3 深化项，但本阶段要预留接口。

文件：

- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\streaming.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\chat.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-cli\src\tui\mod.rs`

测试：

- 连续 assistant delta `["hello", " ", "world", "\nnext"]` 只在 newline 后固化第一行，live tail 保留 `next`。
- turn completed 后 live tail 固化。
- reasoning CJK token 合并后不插入错误空格。
- duplicate final response 不重复渲染。

## Phase 5：Frame Throttle 与减少闪烁

目标：降低 token 高频输出时的视觉抖动和 CPU/terminal 重绘压力。

建议实现：

- 新增 `FrameScheduler`：
  - `last_draw: Instant`
  - `min_frame_interval: Duration`
  - `dirty: bool`
  - `force_draw: bool`
- 普通 token event：
  - 标记 dirty。
  - 距离上次 draw 不足 30-60ms 时不立即 draw。
- 重要事件立即 draw：
  - approval/user_input overlay。
  - command started/completed。
  - error/warning/cancelled。
  - user submit。
  - resize。
- CLI interactive loop 增加 tick：
  - 定时调用 `renderer.tick()`。
  - plain renderer tick no-op。

文件：

- Create：`D:\YunXi Agent\crates\yunxi-agent-tui\src\frame.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\host.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-cli\src\render.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-cli\src\tui\mod.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-cli\src\interactive.rs`

测试：

- 高频 dirty 不会每次都 draw。
- force_draw 会立即 draw。
- plain renderer tick 不改变输出。

## Phase 6：Resize Reflow 与 Anchor

目标：窗口大小变化后不丢失可读位置。

建议实现：

- transcript render 前统一计算 wrapped lines。
- viewport 以“距离底部的 offset”保存，而不是绝对行号。
- resize 后：
  - follow-tail 状态继续显示底部。
  - history 状态尽量保持同一段内容附近。
- `Event::Resize` 强制 redraw。

文件：

- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\viewport.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\host.rs`

测试：

- 同一长 transcript 在 80 列和 120 列下都能正确显示底部。
- 向上滚动后 resize 不会自动跳回底部。

## Phase 7：验证与发布策略

构建阶段仍遵守用户硬性约束：

- 先按报告完成源码构建。
- 构建过程中不在单点反复测试。
- 全部迁移完成后统一验证。
- 每个正式版本创建新 tag，本阶段实施完成后应创建 `v1.7.2`，旧 tag 不删除、不移动。
- GitHub 读写和推送继续全部走 REST API。
- `C:\Users\admin\Desktop\api.txt` 可用于 DeepSeek/GitHub API，但必须隐私化，不输出、不提交、不写日志。
- 任务结束追加 `C:\Users\admin\Desktop\YunXi Agent开发日志.md`。
- 发布后执行 `cargo clean` 并确认 `target_exists=False`。

统一验证建议：

- `cargo fmt`
- `cargo fmt -- --check`
- `cargo test`
- `cargo check --workspace`
- `cargo build -p yunxi-agent-cli --release --bins`
- `target\release\yunxi.exe --version` 输出 `yunxi 1.7.2`
- `target\release\yunxi-agent-cli.exe --version` 输出 `yunxi 1.7.2`
- plain interactive smoke：`/exit | yunxi --offline --no-tui`
- TUI render snapshot：长 transcript + scroll up/down + scrollbar + footer status
- TUI stream snapshot：assistant live tail + stable commit + final固化
- DeepSeek non-stream live smoke，model 使用 `deepseek-chat`
- DeepSeek stream live smoke，model 使用 `deepseek-chat`
- dependency scan：`codex-*`、`vendor/codex-rs`、`yunxi-agent-codex` 命中数为 0
- secret scan
- `git diff --check`
- `codegraph sync "D:\YunXi Agent"`
- 安装脚本：`scripts\install\install-yunxi.ps1 -AddToPath -SkipBuild`
- PATH smoke：`yunxi --version`
- GitHub REST API 发布 master 和 annotated tag `v1.7.2`
- 旧 tag 解引用目标校验不变
- `cargo clean`
- `target_exists=False`

## 风险与边界

- 鼠标滚轮在 Windows Terminal、PowerShell、不同终端模拟器里事件细节可能不同，v1.7.2 应以 crossterm 标准 mouse event 为主，并保留 PageUp/PageDown/Home/End 键盘兜底。
- 运行中 turn 的 terminal event pump 不能阻塞 runtime stream，否则会影响模型输出和 approval 响应。
- frame throttle 不能延迟 approval overlay、错误、取消、命令完成等关键事件。
- markdown table holdback 不建议一次性做深，v1.7.2 先完成 newline commit、live tail 和 code fence 稳定渲染，后续再做表格专用 holdback。
- plain、JSON、JSONL 是自动化关键路径，不能引入 TUI 状态或 ANSI 控制字符。

## v1.7.2 完成后的预期状态

完成 v1.7.2 后，YunXi Agent 的 TUI 应从“结构完整”进一步变成“可阅读、可回看、流式体验自然”的终端 Agent：

- 用户可以在长对话中用鼠标滚轮回看历史。
- 当前输出不会强行把用户从历史位置拉回底部。
- 新输出有明确提示，用户可按 End 回到 tail。
- 流式文本像截图中的 Codex CLI 一样在稳定块中自然增长。
- 输入区和 bottom pane 始终稳定。
- `yunxi` 继续保持完全自主运行，不依赖上游 Codex CLI 源码。

## 实施结果记录

本轮 v1.7.2 已按报告完成构建：

- 新增 `crates/yunxi-agent-tui/src/viewport.rs`，以“距离底部的 offset”管理 transcript 视口、history/tail/new-output-below 状态、PageUp/PageDown/Home/End 跳转语义。
- 新增 `crates/yunxi-agent-tui/src/frame.rs`，提供 33ms 默认帧节流和 force redraw 通道。
- 更新 `crates/yunxi-agent-tui/src/host.rs`，启用 mouse capture，运行中 turn 通过 `tick()` drain 滚轮/翻页/resize 事件，关键路径可强制 flush。
- 更新 `crates/yunxi-agent-tui/src/render.rs`，由 viewport 选择 transcript 窗口，显示行号、状态标题、右侧 scrollbar，并保持 composer/approval/user_input 底部 pane 固定。
- 更新 `crates/yunxi-agent-tui/src/streaming.rs`，增加 stable/live markdown stream controller，为后续 table holdback 预留接口。
- 更新 `crates/yunxi-agent-cli/src/render.rs`、`interactive.rs`、`tui/mod.rs`，为 renderer 增加 `tick()` 和 `flush()`，plain renderer 保持 no-op，TUI renderer 转发到 TUI host。
- 工作区版本升级到 `1.7.2`，README、CLI about、版本测试同步更新。
- `.gitignore` 新增 `/.yunxi/`，避免本地会话 smoke 产物误提交。

统一验证结果：

- `cargo fmt`：通过。
- `cargo fmt -- --check`：通过。
- `cargo test`：通过；workspace unit tests、integration tests、doc tests 全部 0 failure。
- `cargo check --workspace`：通过。
- `cargo check --workspace --target x86_64-unknown-linux-gnu`：环境限制未通过，本机未安装 `x86_64-unknown-linux-gnu` target 标准库。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- `target\release\yunxi.exe --version`：`yunxi 1.7.2`。
- `target\release\yunxi-agent-cli.exe --version`：`yunxi 1.7.2`。
- `target\release\yunxi.exe --offline "v1.7.2 offline smoke"`：通过。
- `@("你好","/status","/exit") | target\release\yunxi.exe --no-tui --offline`：通过。
- DeepSeek non-stream live smoke：通过，model=`deepseek-chat`，credential_index=1，`secret_leak_detected=False`。
- DeepSeek stream live smoke：通过，model=`deepseek-chat`，credential_index=1，`secret_leak_detected=False`。
- DeepSeek interactive live smoke：通过，banner/assistant marker/normal exit 均检测成功，`secret_leak_detected=False`。
- default CLI dependency scan：通过，`yunxi-agent-codex` / `codex-rs` 命中 0。
- 项目源码 secret scan（排除 `vendor/`、`extracted/`、`target/`）：通过，命中 0。
- 广域 secret scan 命中均来自 `vendor/` / `extracted/` 的上游测试或示例源码，不是本轮项目源码泄露。
- `git diff --check`：通过，仅有 Windows LF-to-CRLF 提示。
- `scripts\install\install-yunxi.ps1 -AddToPath -SkipBuild`：通过，安装目录为 `C:\Users\admin\AppData\Local\YunXi Agent\bin`。
- PATH smoke：`yunxi --version`、`yunxi-agent-cli --version` 均输出 `1.7.2`，`yunxi --offline` smoke 通过。
- `codegraph sync .`：通过，13 个变更文件同步，`codegraph status .` 显示 index up to date。

发布仍按策略执行：创建 release commit 和新的 annotated tag `v1.7.2`，旧 tag 不删除、不移动；GitHub 发布走 REST API；发布后执行 `cargo clean` 并确认 `target_exists=False`。

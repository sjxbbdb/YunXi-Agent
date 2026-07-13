# YunXi Agent v1.7.4 TUI Scrollbar And Viewport Development Report

生成时间：2026-07-13 07:52:00 +08:00

## 背景

YunXi Agent v1.7.3 已经把 TUI 主视图从底层事件日志调整为更接近用户视角的 transcript：默认隐藏协议噪声，保留 assistant、thinking、tool timeline、warning/error 和 debug/detail 通道。用户最新截图显示，v1.7.3 的页面结构和主视图内容已经明显可用，但中间 transcript 区域仍有两个核心瑕疵：

- 右侧 scrollbar 的 thumb 看起来只能移动到中间，无法准确表达已经到达底部或顶部。
- 鼠标不能拖动 scrollbar，只能通过滚轮滚动。
- 长中文段落和流式输出在视觉上占用多行，但 TUI 统计和滚动仍按逻辑行处理，导致滚动范围、标题区间、thumb 比例和实际屏幕内容不同步。

本报告的目标是为 v1.7.4 制定一次聚焦的 TUI 体验修复：修正 viewport 的行模型，补齐 mouse drag 交互，并在实现上参考 Codex CLI TUI 的 scrollback、wrapping 和 viewport 思路，同时继续保持 YunXi 自主实现，不引入上游 Codex 运行依赖。

## 现象复盘

截图中的 transcript 标题为：

```text
Transcript 23-42 / 42 | tail
```

但可见区域中很多逻辑行被 `Paragraph.wrap(Wrap { trim: false })` 自动折成多条屏幕行。也就是说：

- 标题里的 `42` 是逻辑行数量，不是屏幕可见行数量。
- scrollbar 的 content length 也是 `42`，不是实际渲染后的 wrapped row 数量。
- `tail` 状态下可见内容仍可能没有对应真实底部，因为切片发生在 wrap 之前。
- 鼠标拖动没有响应，因为当前 host 只处理了 `MouseEventKind::ScrollUp` 和 `MouseEventKind::ScrollDown`。

这说明当前问题不是简单把滚轮步长调大，而是 viewport 的数据模型和真实渲染模型不一致。

## 当前 YunXi 源码诊断

本轮先用 CodeGraph 查询当前 TUI 路径，再用精确文本定位实现。

### 1. 逻辑行与屏幕行不一致

相关文件：

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\chat.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\viewport.rs`

当前 `Transcript::render_line_count()` 只按 `content.lines().count()` 统计：

```rust
content.lines().count().max(1)
```

当前 `render_transcript()` 的流程是：

```rust
let lines = transcript_lines(app);
let visible = area.height.saturating_sub(2) as usize;
let start = app.viewport().view_start(lines.len(), visible);
let end = start.saturating_add(visible.max(1)).min(lines.len());
let transcript = Paragraph::new(lines[start..end].to_vec())
    .wrap(Wrap { trim: false });
```

问题在于：`lines[start..end]` 先按逻辑行切片，随后 `Paragraph.wrap` 才按区域宽度折行。对于长中文、长英文、tool summary、assistant 长句，一条逻辑行可能占多条屏幕行，导致：

- viewport 最大 offset 被低估。
- title 显示的 `start-end / total` 被低估。
- scrollbar thumb 比例被低估或位置异常。
- tail/follow 状态与实际底部不一致。

### 2. 鼠标只支持滚轮，不支持拖动

相关文件：

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\host.rs`

当前 `handle_navigation_event()` 只处理：

- `MouseEventKind::ScrollUp`
- `MouseEventKind::ScrollDown`
- `PageUp`
- `PageDown`
- `Home`
- `End`

缺少：

- `MouseEventKind::Down(MouseButton::Left)`
- `MouseEventKind::Drag(MouseButton::Left)`
- `MouseEventKind::Up(MouseButton::Left)`
- scrollbar hit-test
- drag capture state
- track click page up/down
- 根据鼠标 y 坐标映射到 transcript row/start offset 的函数

因此“无法通过鼠标拖动”是确定缺口，不是终端兼容性问题。

### 3. 布局计算分散

当前 `render_tui_frame()` 在 `render.rs` 内计算 header/transcript/bottom pane 三段布局，而 `host.rs` 的 `transcript_visible_height()` 复制了一份高度估算。两处没有共享同一个 `Rect` 计算结果。v1.7.4 如果要做鼠标 hit-test，必须知道 transcript 区域、inner 区域、scrollbar track 区域和 thumb 几何。继续复制高度公式会扩大漂移风险。

### 4. 流式输出的行高随宽度变化，但 viewport 不感知

v1.7.3 已有 frame throttle 和 active assistant/reasoning cell，但流式内容进入 transcript 后，行高仍由 `Paragraph.wrap` 隐式决定。只要输出持续增长或终端宽度改变，真实屏幕行数就会变化，而 `TranscriptViewport` 不知道变化后的 wrapped rows。这个问题会表现为：

- 新增 token 后 tail 位置抖动。
- 长段落越长，scrollbar 越不准。
- resize 后 transcript 标题和真实内容区间不一致。

## Codex CLI TUI 参考结论

本轮检查了本地 `D:\源码\codex\AGENTS.md` 的 TUI 指令，并通过 GitHub raw/API 读取上游 `openai/codex` 的 TUI 关键源码。需要注意：本地 `D:\源码\codex` 当前没有可直接展开的 `codex-rs/tui/src` 源文件，因此报告引用上游 GitHub 作为源码参考，不把它引入 YunXi 依赖。

参考文件：

- `https://github.com/openai/codex/blob/main/codex-rs/tui/src/insert_history.rs`
- `https://github.com/openai/codex/blob/main/codex-rs/tui/src/tui.rs`
- `https://github.com/openai/codex/blob/main/codex-rs/tui/src/wrapping.rs`
- `https://github.com/openai/codex/blob/main/codex-rs/tui/src/bottom_pane/scroll_state.rs`
- `D:\源码\codex\AGENTS.md`

关键参考点：

- Codex 的主 chat 历史大量写入真实 terminal scrollback，inline viewport 主要承载 composer/status 等当前交互区域，而不是把全部历史都塞进一个 alt-screen 内部列表。
- Codex 在 `insert_history.rs` 里明确做预换行：先按 viewport width 计算 wrapped rows，再写入终端 scrollback。
- Codex 在 `wrapping.rs` 中使用显式 wrapping helper，而不是依赖 `Paragraph.wrap` 隐式决定真实行数。
- Codex 的 list/popup 类局部滚动使用 `ScrollState`，由调用方传入 `len` 和 `visible_rows`，并在过滤/分页后立即 clamp，避免 stale length。
- Codex 的 `tui.rs` 维护 `viewport_area`、resize reflow、pending history lines 和 terminal scrollback，而不是让 host 与 render 各自猜测可见区域。
- 上游 `AGENTS.md` 明确要求 TUI wrapping 使用 `wrapping.rs` helper，UI 变化要有 snapshot 覆盖。

对 YunXi 的结论：

- v1.7.4 不应直接大规模照搬 Codex 的整个 terminal scrollback 架构；那会变成一次 TUI 主架构迁移，风险过大。
- v1.7.4 应先把 YunXi 当前 alt-screen transcript 的 wrapped row 模型修正，让行数、viewport、title、scrollbar、mouse drag 全部基于同一份布局结果。
- 后续 v1.8 可以评估是否迁移到 Codex-style inline viewport + terminal scrollback 架构。

## v1.7.4 总目标

YunXi Agent v1.7.4 要把 TUI transcript 从“逻辑行滚动”修正为“屏幕行滚动”，并补齐鼠标拖动 scrollbar。

成功标准：

- 长中文/英文段落折行后，scrollbar thumb 能准确到达顶部和底部。
- `Transcript x-y / total` 中的 total 表示实际 wrapped rows，而不是逻辑行。
- 滚轮、PageUp/PageDown、Home/End、鼠标拖动使用同一套 wrapped row content height。
- 鼠标拖动 scrollbar thumb 可滚动到顶部、中间、底部。
- 点击 scrollbar track 的 thumb 上方/下方可按页滚动。
- 鼠标滚轮只在 transcript 区域内滚动 transcript；composer 区域不误触历史滚动。
- 流式输出增长时，tail 模式保持贴底；history 模式显示 `new output below`，不强行跳回底部。
- resize 后 wrapped rows 重新计算，viewport clamp 正确，thumb 比例正确。
- JSON/JSONL/plain renderer 不受影响。
- 默认依赖仍不包含 `codex-*`、`vendor/codex-rs` 或 `yunxi-agent-codex`。

## 非目标

- 不在 v1.7.4 重写整个 TUI 为 Codex 的 inline terminal scrollback 架构。
- 不引入上游 Codex crate 作为默认运行依赖。
- 不改变 v1.7.3 已完成的 event filter、tool timeline、debug/detail 策略。
- 不在报告阶段创建 `v1.7.4` tag。tag 只在源码实现、统一验证、安装和发布完成后创建。
- 不把 mouse drag 做成必须依赖某个终端私有能力。基于 crossterm 标准 mouse events 实现，终端不支持时保持滚轮/键盘可用。

## 推荐架构

### 1. 统一 TUI 布局几何

新增：

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\layout.rs`

职责：

- 计算 header、transcript、composer/bottom pane 的 `Rect`。
- 计算 transcript inner area。
- 计算 vertical scrollbar track area。
- 为 `render.rs` 和 `host.rs` 提供同一套布局结果。

建议类型：

```rust
pub(crate) struct TuiLayout {
    pub header: Rect,
    pub transcript: Rect,
    pub transcript_inner: Rect,
    pub transcript_scrollbar: Rect,
    pub bottom_pane: Rect,
}

pub(crate) fn compute_layout(area: Rect, bottom_pane_height: u16) -> TuiLayout;
```

这样 `render_tui_frame()` 和 `handle_navigation_event()` 不再各自计算 visible height，mouse hit-test 也有可靠坐标。

### 2. 预换行 transcript rows

新增：

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\transcript_layout.rs`

职责：

- 把 `HistoryCell` 转成 styled logical lines。
- 根据 transcript inner width 预先 wrap 成屏幕 rows。
- `render_transcript()` 只渲染 wrapped rows，不再使用 `Paragraph.wrap` 二次隐式换行。
- viewport、title、scrollbar 统一使用 wrapped row count。

建议类型：

```rust
pub(crate) struct WrappedTranscript {
    pub rows: Vec<Line<'static>>,
    pub logical_cells: usize,
}

pub(crate) fn build_wrapped_transcript(
    cells: &[HistoryCell],
    width: usize,
) -> WrappedTranscript;
```

实现要求：

- `width` 最小为 1。
- label 行保留 `[user]`、`[assistant]`、`[thinking]` 等样式。
- continuation row 使用现有 `  | ` gutter 风格。
- CJK、ASCII、空格、长 token 都要按显示宽度处理。
- 可引入 workspace 级 `textwrap` 依赖作为 YunXi 自有 wrapping helper；不要依赖上游 Codex wrapping 模块。
- 如果新增 `textwrap`，同步 `Cargo.toml`、`Cargo.lock`，最终统一验证必须覆盖。

### 3. Viewport 从 bottom-offset 扩展到 start-row 操作

修改：

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\viewport.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`

当前 bottom-offset 模型适合 tail follow，但 mouse drag 更自然地以 start row 操作。v1.7.4 保留 bottom-offset 状态，同时增加显式设置 start 的 API。

建议新增：

```rust
pub(crate) fn set_view_start(
    &mut self,
    start: usize,
    content_height: usize,
    viewport_height: usize,
);

pub(crate) fn set_scroll_fraction(
    &mut self,
    numerator: usize,
    denominator: usize,
    content_height: usize,
    viewport_height: usize,
);

pub(crate) fn max_start(content_height: usize, viewport_height: usize) -> usize;
```

规则：

- `start=0` 表示顶部/history。
- `start=max_start` 表示底部/tail。
- 设置到底部时清除 `new_content_below`。
- 非底部时保留 history 状态。
- content height 改变时 clamp offset，避免空白页。

### 4. Scrollbar 几何与鼠标拖动状态机

新增：

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\scrollbar.rs`

职责：

- 根据 content height、visible height、current start、track rect 计算 thumb。
- 根据鼠标坐标判断 hit-test。
- 把 drag y 坐标映射到目标 start row。

建议类型：

```rust
pub(crate) struct TranscriptScrollbarGeometry {
    pub track: Rect,
    pub thumb_top: u16,
    pub thumb_height: u16,
    pub content_height: usize,
    pub visible_height: usize,
    pub start: usize,
}

pub(crate) enum ScrollbarHit {
    Thumb { grab_offset: u16 },
    PageUp,
    PageDown,
    TrackTo { start: usize },
    Outside,
}
```

修改：

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\host.rs`

新增 host 状态：

```rust
scroll_drag: Option<TranscriptScrollDrag>,
```

处理事件：

- `Down(Left)`：如果命中 thumb，记录 drag；如果命中 track 上方/下方，page up/down。
- `Drag(Left)`：如果正在 drag，按 y 坐标设置 view start。
- `Up(Left)`：结束 drag。
- `ScrollUp/ScrollDown`：只有鼠标位置在 transcript rect 内时滚动 transcript。
- `Resize`：清理 drag 状态并触发重绘。

### 5. 渲染层改造

修改：

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs`

要求：

- 使用 `compute_layout()` 获取区域。
- 使用 `build_wrapped_transcript()` 获取 rows。
- `visible = transcript_inner.height as usize`。
- `start = viewport.view_start(rows.len(), visible)`。
- `end = min(start + visible, rows.len())`。
- `Paragraph::new(rows[start..end].to_vec())` 不再调用 `.wrap(...)`。
- `ScrollbarState::new(rows.len()).position(start).viewport_content_length(visible)` 使用 wrapped row 数据。
- title 显示 wrapped row 范围，例如 `Transcript rows 61-82 / 82 | tail`。
- 如果为了兼容旧文案保留 `Transcript 61-82 / 82`，报告建议在 v1.7.4 先不加 `rows`，但内部含义必须是 wrapped rows。

### 6. 流式输出稳定化

修改：

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\chat.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\streaming.rs`

要求：

- active assistant/reasoning cell 的 wrapping 与 finalized cell 使用同一套 `transcript_layout.rs`。
- 每次流式增量只改变 active cell 内容，不插入额外空逻辑行。
- tail 模式下新 row 增长后仍保持尾部可见。
- history 模式下新 row 增长只设置 `new_content_below`，不抢用户滚动位置。
- 宽度变化时重新生成 wrapped rows，并 clamp viewport。

### 7. Footer 与可发现性

修改：

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`

当前 footer 只写 `wheel scroll`。v1.7.4 应在支持 mouse capture 时显示：

- tail：`wheel/drag scroll | PgUp history | Ctrl+C exit`
- history：`wheel/drag history | End follow tail | PgUp/PgDown`

不要在主视图加入大段说明文字，只更新短 footer hint。

## 分阶段实施建议

### Phase 1：布局与 wrapped row 基础

目标：先让行数模型正确。

文件：

- Create：`D:\YunXi Agent\crates\yunxi-agent-tui\src\layout.rs`
- Create：`D:\YunXi Agent\crates\yunxi-agent-tui\src\transcript_layout.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\lib.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\chat.rs`

构建要求：

- `render_transcript()` 不再基于逻辑行切片。
- `Transcript::render_line_count()` 不再作为滚动高度来源；保留时只用于兼容或测试旧逻辑。
- wrapped row count 成为 viewport、title、scrollbar 的唯一 content height。

测试设计：

- 长中文单行在窄宽度下 wrap 成多行，`total` 大于逻辑行数。
- tail 状态下 start 等于 max_start。
- history 状态下新增输出不改变 start，只显示 `new output below`。

### Phase 2：viewport API 与 scrollbar 几何

目标：为鼠标拖动提供可测试的纯逻辑。

文件：

- Create：`D:\YunXi Agent\crates\yunxi-agent-tui\src\scrollbar.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\viewport.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\lib.rs`

构建要求：

- thumb 高度最小为 1，最大不超过 track height。
- content <= visible 时不显示 scrollbar，hit-test 为 outside。
- y=track top 映射到 start=0。
- y=track bottom 映射到 start=max_start。
- drag 到最底部后 viewport status 变为 `tail`。

测试设计：

- `scrollbar_thumb_reaches_bottom_at_max_start`
- `scrollbar_drag_maps_top_middle_bottom`
- `viewport_set_view_start_clamps_and_updates_tail`

### Phase 3：host 鼠标交互

目标：补齐用户实际感受到的拖动能力。

文件：

- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\host.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`

构建要求：

- host 保存 `scroll_drag`。
- `MouseEventKind::Down(MouseButton::Left)` 命中 thumb 后进入 drag。
- `MouseEventKind::Drag(MouseButton::Left)` 连续更新 start。
- `MouseEventKind::Up(MouseButton::Left)` 退出 drag。
- 点击 track 上方/下方触发 page up/down。
- wheel 只有在 transcript rect 内时滚动 transcript。

测试设计：

- host 纯函数测试坐标命中，不直接依赖真实终端。
- 使用 `TestBackend` snapshot 验证拖动前后标题区间变化。

### Phase 4：流式输出与 resize polish

目标：避免长输出和 resize 再次破坏滚动模型。

文件：

- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\chat.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs`
- Modify：`D:\YunXi Agent\crates\yunxi-agent-tui\src\streaming.rs`

构建要求：

- active stream tail 和 finalized history 使用同一 wrap helper。
- resize 后重新计算 wrapped rows 并 clamp viewport。
- frame throttle 保持，不因为 drag/resize 导致空白帧。

测试设计：

- 流式追加长中文后 tail 仍显示最后一行。
- 用户滚到 history 后继续流式追加，显示 `new output below`。
- resize 从宽变窄后 `total` 增大、start clamp 正确、无 panic。

### Phase 5：文档、版本与发布

文件：

- Modify：`D:\YunXi Agent\Cargo.toml`
- Modify：`D:\YunXi Agent\Cargo.lock`
- Modify：`D:\YunXi Agent\README.md`
- Modify：`D:\YunXi Agent\docs\extraction-status.md`
- Modify：`D:\YunXi Agent\docs\reports\2026-07-13-yunxi-agent-v1-7-4-tui-scrollbar-viewport-development-report.md`

要求：

- 版本推进到 `1.7.4`。
- README 增加 v1.7.4 TUI scrollbar/drag 说明。
- 实现完成后创建新的 annotated tag `v1.7.4`。
- 旧 tag 不删除、不移动。
- GitHub 发布继续全部走 REST API。

## 统一验证计划

后续实现阶段继续遵守用户硬性约束：先整体构建，中间不做单点验证，不在某一个点浪费大量时间；构建完成后统一验证。

统一验证命令：

- `cargo fmt`
- `cargo fmt -- --check`
- `cargo test`
- `cargo check --workspace`
- `cargo build -p yunxi-agent-cli --release --bins`
- `target\release\yunxi.exe --version`，期望 `yunxi 1.7.4`
- `target\release\yunxi-agent-cli.exe --version`，期望 `yunxi 1.7.4`
- `target\release\yunxi.exe --offline "v1.7.4 offline smoke"`
- TUI wrapped row 专项测试
- TUI scrollbar geometry 专项测试
- TUI mouse drag 专项测试
- TUI snapshot：长中文输出、长英文输出、top/middle/tail 三种滚动状态
- DeepSeek live stream smoke，确认真实模型流式输出仍正常
- plain interactive smoke，确认 `--no-tui` 不受影响
- JSON/JSONL smoke，确认结构化输出不受 TUI 改动影响
- default dependency scan：确认默认 CLI 依赖不含 `codex-*`、`vendor/codex-rs`、`yunxi-agent-codex`
- owned-source secret scan
- `git diff --check`
- `codegraph sync "D:\YunXi Agent"`
- `codegraph status "D:\YunXi Agent"`
- `scripts\install\install-yunxi.ps1 -AddToPath -SkipBuild`
- PATH smoke：`yunxi --version`、`yunxi-agent-cli --version`、`yunxi --offline "installed path v1.7.4 smoke"`
- GitHub REST API 发布 `master` 和 annotated tag `v1.7.4`
- 远端 `master`、远端 `refs/tags/v1.7.4`、本地 `HEAD`、本地 tag 解引用一致性校验
- `cargo clean`
- `target_exists=False`

## 风险与边界

- 预换行会影响所有 TUI transcript 渲染，必须用 snapshot 覆盖长中文、长英文、tool timeline、debug/detail、warning/error。
- 鼠标事件在不同终端支持程度不同，不能让 drag 成为唯一滚动路径；滚轮、PageUp/PageDown、Home/End 必须继续可用。
- 如果新增 `textwrap`，要确认默认 CLI 仍不引入上游 Codex 依赖。
- 不要把 `Paragraph.wrap` 和手动 wrap 混用，否则会再次出现双重换行导致高度错误。
- mouse drag hit-test 必须基于共享 layout，不要在 host 里复制 render 的坐标公式。
- 不要为了滚动条拖动而破坏 composer 输入、approval overlay、request user input 的键鼠交互。
- 不要把 `target` 构建产物留在硬盘，发布后必须 `cargo clean`。

## v1.7.4 完成后的预期状态

完成 v1.7.4 后，YunXi Agent TUI 应具备：

- 准确的 wrapped row viewport。
- 准确的 scrollbar thumb 比例和位置。
- 可拖动的 scrollbar。
- 稳定的长中文/英文流式输出体验。
- resize 后不丢失滚动位置、不出现半截/空白滚动区。
- 与 v1.7.3 event filter/tool timeline/debug detail 能力兼容。
- 继续保持 YunXi 自主运行，不依赖上游 Codex CLI 源码。

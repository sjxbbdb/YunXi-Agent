# YunXi Agent v2.0.3 重绘调度、滚动与 Resize 稳定开发报告

- 撰写时间：2026-07-19 13:12:10 +08:00
- 审核报告：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-19-122912-YunXi-Agent-v2.0.2-hotfix.1-源码与TUI视觉审核报告.md`
- 上一版本基线：`v2.0.2-hotfix.1`
- 开发目录：`D:\YunXi Agent`
- 项目内报告：`D:\YunXi Agent\docs\reports\2026-07-19-131210-yunxi-agent-v2-0-3-redraw-scroll-resize-development-report.md`
- 桌面报告：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-19-131210-yunxi-agent-v2-0-3-redraw-scroll-resize-development-report.md`
- 报告类型：下一版本开发报告
- 审核结论转化：`v2.0.2-hotfix.1` 审核通过，可以进入 `v2.0.3` 开发。

## 一、硬性约束

以下约束是本项目后续开发、验证、发布和文档同步的前置要求，不得因为上下文过长、任务拆分、实现难度或局部修复而忽略、弱化或绕过。

1. 开发目录固定在 `D:\YunXi Agent` 及其工作树内。
2. 核心目标是开发一个通用型陪伴 agent。
3. 涉及到源码参考需要先抽取、迁移和构建整体能力，不要在单个点上反复纠结；如果参考的源码不为 Rust 语言，则需参考其逻辑，进行 Rust 复刻。
4. 中间无需频繁验证，完成一批构建后再统一测试和验证。
5. 验证通过前不能宣称完成。
6. 每次阶段结束后要清理编译中间产物，避免占用大量硬盘空间。
7. 每次任务结束后，都要在日志中追加本次工作的详细记录。
8. 记录必须包含时间戳、工作目标、执行流程、修改文件、文件路径、验证结果、提交和推送状态，以及日志结尾处要进行署名，署名为开发者。
9. 后续开发报告、设计文档、索引和状态文档要与实际代码状态保持一致。
10. 回复用户使用中文。
11. 之后每个版本更迭都必须提交为新的 Git tag；以前的版本 tag 不得删除，方便出错后回滚。
12. 涉及到 shell 命令时要万分小心，例如递归删除、强制移动、清空目录、系统级安装/卸载、修改 PATH/注册表/系统配置等操作必须得到用户确认。
13. 不可因为过长的上下文忽略硬性要求；如果察觉到硬性要求的效力已经减弱，则必须汇报当前情况。
14. 工作尽量固定在项目文件夹，涉及到其他文件夹的操作，要明确告知用户，尤其是 C 盘的用户目录。

## 二、开发目标

`v2.0.2-hotfix.1` 已完成并通过流式协议、assistant cell canonicalization、稳定 `event_id`、可靠 sequence、Markdown/Unicode 提交边界、session 回收和 active-stream `Ctrl+C` 取消验收。`v2.0.3` 的核心目标是继续推进 TUI 体验，但范围必须收敛在总纲图指定的 **重绘调度、滚动与 Resize 稳定**。

本阶段要完成：

1. 建立节流/合帧的 TUI 重绘调度，避免流式 delta、状态栏、输入区和 resize 事件触发过度 draw。
2. 落实用户滚动后的 viewport anchor，保证用户查看历史时新输出不抢回尾部。
3. 在窗口 resize 后保持 transcript、composer、状态栏和 footer 的稳定布局，不丢行、不重叠、不跳尾。
4. 保持 `v2.0.2-hotfix.1` 已通过的 canonical assistant cell、取消、Unicode/Markdown 和普通视图信息边界。
5. 使用自动化 TestBackend 与真实 TUI 视觉/交互复核共同验收，不以单一离线 fixture 替代真实终端表现。

## 三、版本与发布纪律

`v2.0.2-hotfix.1` 是经项目负责人确认的整改例外标签。后续版本应回到总纲图的顺序编号与单版本单标签纪律。

开发者必须遵守：

- 旧 `v2.0.2` tag 与 `v2.0.2-hotfix.1` tag 均不得移动、删除或覆盖。
- `v2.0.3` 必须对应新的发布提交和新的 annotated tag。
- 验证通过前不得宣称 `v2.0.3` 完成。
- 发布前必须确认工作树中审计运行生成的 `crates\yunxi-agent-cli\.yunxi` 等未跟踪状态目录是否需要保留或清理；清理前必须取得用户确认。
- 不得把本阶段内容提前扩大到 `v2.0.4` 及之后的输入编辑、Markdown 丰富渲染、主题、插件或新产品面。

## 四、非目标

以下内容不属于 `v2.0.3`：

- 输入编辑增强、快捷键矩阵、历史搜索和多行编辑深度重构。
- Markdown 富渲染、表格/代码块高亮、主题系统和配色方案。
- 新的 tool approval 产品形态。
- 新的 companion/memory/relationship 功能。
- 云服务、后台常驻服务、SDK packaging、系统级安装器。
- 引入上游 Codex TUI runtime 或让默认 YunXi 运行路径依赖上游源码。

## 五、源码接入点

### `crates\yunxi-agent-tui\src\host.rs`

`host.rs` 是终端事件循环、tick、输入事件与 draw 调度的主要接入点。开发者应在这里建立明确的 redraw request/dirty 标记和合帧策略：

- 将流式事件、输入变更、状态栏变更、resize、scroll、cancel 等统一转化为 redraw request。
- 对高频流式 delta 使用节流或合帧，建议以 16ms 到 33ms 为初始区间，避免每个 token 直接触发终端 draw。
- resize 与用户输入事件应保持高优先级，不能因流式节流导致输入区卡顿或尺寸变化滞后。
- active stream 的 `Ctrl+C` 取消链路必须继续保持即时性，不得被 redraw 调度延迟。

### `crates\yunxi-agent-tui\src\app.rs`

`YunxiTuiApp` 当前持有 `TranscriptViewport`、`TimelineStore`、`Transcript`、`BottomPane` 和 `ControlSnapshot`。`v2.0.3` 应围绕 app 层建立稳定的 viewport anchor 语义：

- 明确 tail-follow、history-view、new-output-below 三种状态的转换。
- 用户滚动离开尾部后，新增输出只能标记 `new output below`，不得重置 `view_start`。
- `End` 或显式 follow-tail 操作才能恢复尾随。
- final、cancel、late event、duplicate event、resize 都不得意外改变用户历史锚点。
- footer/subheader 的 scroll 状态文案应与真实 viewport 状态一致。

### `crates\yunxi-agent-tui\src\render.rs`

`render.rs` 是布局、Transcript 渲染、header/footer、composer 绘制和 TestBackend 回归的核心位置。开发者应重点处理：

- 根据终端尺寸稳定分配 header、subheader、transcript、bottom pane、footer 的高度。
- 极小高度或窄宽度下不得出现 panic、文字压盖或输入区丢失。
- resize 后根据 anchor 重算可见范围，而不是简单按旧 `view_start` 截断。
- `transcript_title` 中的范围显示必须与实际可见行一致。
- CJK、Emoji、组合字符和宽字符下的截断与 cursor 位置仍应准确。

### `crates\yunxi-agent-tui\src\timeline_store.rs`

`timeline_store.rs` 的 canonical assistant cell 与会话生命周期已经通过 v2.0.2-hotfix.1 审核。`v2.0.3` 中不得破坏该状态机，但需要确保：

- 流式更新后的 transcript 变更能给 redraw scheduler 提供清晰的 dirty 信号。
- session final/cancel 后触发的 UI 更新不抢用户滚动锚点。
- duplicate event counter 继续出现在 debug/subheader 信息中，但不制造额外 transcript cell。

### `crates\yunxi-agent-tui\src\presentation.rs`

presentation 层应继续保持普通视图安静，只提供规范 TUI event，不泄漏 provider wire、thinking、memory/context、工具参数或内部错误栈。

本阶段如果需要为 render 或 viewport 提供更稳定的 cell identity，应复用现有 `TuiCellId` 和流式 identity，不新增无边界的 UI 全局状态。

### `crates\yunxi-agent-cli\src\interactive.rs` 与 `crates\yunxi-agent-cli\src\tui\mod.rs`

CLI 与 TUI 桥接层需要保持：

- live/no-live、TUI/no-TUI 行为一致。
- 33ms tick 与取消控制不回退。
- offline、JSON、JSONL、普通 render 不受 TUI redraw scheduler 影响。

## 六、参考源码建议

继续参考审核报告指定的 Codex TUI 路径：

- `D:\源码\codex\codex-rs\tui\src\chatwidget\rendering.rs`
- `D:\源码\codex\codex-rs\tui\src\render\renderable.rs`
- `D:\源码\codex\codex-rs\tui\src\wrapping.rs`
- `D:\源码\codex\codex-rs\tui\src\width.rs`

参考方式：

- 抽取渲染对象分层、稳定可渲染单元、换行与显示宽度处理、viewport anchoring 思路。
- 不复制上游 UI 代码，不引入上游 TUI runtime。
- 不依赖 `vendor/codex-rs`、`codex-*` crate 或 `yunxi-agent-codex`。
- 如参考非 Rust 逻辑，必须以 YunXi 自身 Rust 模块复刻。

## 七、推荐技术设计

### RedrawScheduler

建议在 TUI 层引入轻量内部调度结构，不作为公共 API 泄漏：

- `RedrawReason`：`InputChanged`、`StreamDelta`、`StreamFinalized`、`ScrollChanged`、`Resize`、`StatusChanged`、`ControlChanged`、`Error`。
- `RedrawPriority`：`Immediate`、`NextFrame`、`Coalesced`。
- `RedrawScheduler`：记录 dirty reason、上次 draw 时间、最短帧间隔、是否存在 pending resize。
- 对 `Resize`、`InputChanged`、`CancelCurrentTurn` 使用即时或下一帧；对高频 `StreamDelta` 合并。

### ViewportAnchor

建议把 viewport 状态从简单 `view_start` 推进到显式 anchor：

- `FollowTail`：默认状态，新输出自动跟随尾部。
- `Pinned { cell_id, line_offset }`：用户滚动历史时固定在某个 cell 与行偏移。
- `NewOutputBelow`：用户 pinned 时有新输出，但不抢尾。

resize 后应按 anchor 重新计算可见窗口。若 anchor 对应 cell 被合并或终态更新，应按 `TuiCellId` 与 canonical cell 更新映射，不能退化为跳尾。

### ResizeLayout

建议把布局计算拆成可测试函数：

- 输入：terminal width/height、bottom pane 状态、header/footer 是否显示、transcript 行数、viewport anchor。
- 输出：每个区域 rect、transcript 可见行范围、composer cursor 位置。
- 小尺寸终端需要有下限策略：保留输入区与最小状态提示，转录区可压缩但不能 panic。

## 八、最低测试要求

自动化测试至少覆盖：

1. 高频 stream delta 被合帧，draw 次数受控，但 final/cancel 能及时刷新。
2. 用户滚动历史后，新 delta/final 不改变可见起点，只显示 `new output below`。
3. `End` 恢复 follow-tail 后，新输出跟随尾部。
4. resize 变窄、变宽、变矮、变高后，anchor 指向同一 transcript 内容或合理邻近内容。
5. active stream + history scroll + final：不抢尾、不重复 assistant cell。
6. active stream + resize + `Ctrl+C`：取消即时，partial 保留，输入区可继续输入。
7. header/subheader/footer 在窄宽度下不溢出、不出现负宽度、不 panic。
8. CJK、Emoji ZWJ、组合字符、长 token 在 resize 后不出现错宽、半字符或 cursor 错位。
9. JSON/JSONL、no-TUI、offline render 不被 TUI scheduler 改动影响。
10. `v2.0.2-hotfix.1` 已通过的 timeline store、streaming、runtime、provider、CLI integration 测试继续通过。

## 九、真实 TUI 验收要求

正式审核前必须实际运行 TUI，而不是只依赖 TestBackend：

1. offline TUI：普通输入、滚动、resize、退出。
2. live 短流：只保留一个 canonical assistant cell。
3. live 长流：输出时滚动到历史区，新输出不抢尾，footer/subheader 状态准确。
4. live 长流 resize：窗口缩小、放大后不丢输入区、不重叠、不跳尾。
5. live active stream cancel：`Ctrl+C` 仍能立即取消当前 turn，partial 保留，下一轮可继续。
6. 普通主视图不暴露 thinking、memory/context、协议 JSON、工具参数或内部错误栈。

## 十、统一验证要求

完成一批构建后统一验证：

1. `cargo fmt --all`
2. `cargo fmt --all -- --check`
3. `cargo check --workspace`
4. `cargo test --workspace`
5. `cargo build --workspace`
6. `cargo build -p yunxi-agent-cli --release --bins`
7. `target\release\yunxi.exe --version`
8. Evaluation Harness、golden、JSON、JSONL 检查
9. TUI offline 与 live 视觉/交互复核
10. `git diff --check`
11. `git status --short --branch`
12. 发布后远程 master 与 annotated tag refs 核验

涉及 `cargo clean`、递归删除 `.yunxi` 状态目录、删除 smoke 临时目录、强制移动、清空目录、系统级安装/卸载、修改 PATH/注册表/系统配置等操作前，必须先取得用户确认。

## 十一、文档同步要求

实现完成后，以下文档必须与代码和版本状态一致：

- `D:\YunXi Agent\README.md`
- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\docs\tui-presentation.md`
- `D:\YunXi Agent\docs\development-log.md`
- `D:\YunXi Agent\docs\reports\2026-07-19-131210-yunxi-agent-v2-0-3-redraw-scroll-resize-development-report.md`
- 如存在总纲、索引或状态文档，也必须同步 `v2.0.3` 的实际完成状态。

## 十二、本报告生成状态

本次任务只根据 `v2.0.2-hotfix.1` 审核报告生成 `v2.0.3` 开发报告，并同步日志。未修改 Rust 源码，未运行构建、测试、清理、提交、推送或创建 Git tag。

当前 Git 状态显示 `crates\yunxi-agent-cli\.yunxi` 为未跟踪目录，来源与审核报告所述真实运行产物一致。本次未读取、删除或清理该目录；如后续需要清理，必须先确认具体路径并取得用户许可。

署名：开发报告撰写者

## 十三、开发实施结果

- 实施时间：2026-07-19 14:07:47 +08:00
- 实施目录：`D:\YunXi Agent`
- 发布版本：`2.0.3`
- 发布标签：annotated `v2.0.3`

### 实际实现

1. 将原 `FrameScheduler` 升级为原因感知的 `RedrawScheduler`，统一处理
   `InputChanged`、`StreamDelta`、`StreamFinalized`、`ScrollChanged`、`Resize`、
   `StatusChanged`、`ControlChanged`、`Error` 与 `CancelCurrentTurn`。高频 delta
   在 33 ms tick 上合帧，final/control 下一帧刷新，输入、滚动、resize、错误与
   取消立即刷新。
2. 将 transcript viewport 从距底部偏移改为 `FollowTail`、稳定
   `Pinned { cell_id, line_offset }` 和 `NewOutputBelow` 锚点。每一条 wrapped row
   都回映到 canonical `TuiCellId`，append、stream final 与 resize 后重新解析同一
   逻辑内容，不再抢回尾部。
3. Host、scrollbar、renderer 与 app 统一消费 `WrappedTranscript`，活动流处于历史
   视图时 footer/subheader 显示真实的 `history` 或 `new output below` 状态。
4. 长 token 改按 Unicode grapheme cluster 分割，覆盖 CJK、Emoji ZWJ、组合字符；
   小尺寸布局使用饱和计算，零尺寸区域不渲染、不设置 composer cursor。
5. workspace、CLI、TUI、persona context、runtime integration、evaluation harness、
   README 与状态文档统一升级为 `2.0.3`；普通 CLI、`--no-tui`、JSON/JSONL 和
   provider/runtime wire contract 未改动。

### 主要修改路径

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\frame.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\viewport.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\transcript_layout.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\host.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\layout.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs`
- `D:\YunXi Agent\Cargo.toml`、`D:\YunXi Agent\Cargo.lock`
- `D:\YunXi Agent\README.md`
- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\docs\tui-presentation.md`
- `D:\YunXi Agent\docs\persona-memory.md`

### 验证结果

- `cargo fmt --all`：通过。
- `cargo fmt --all -- --check`：通过。
- `cargo check --workspace`：通过。
- `cargo test --workspace`：通过；CLI integration 44 项、provider 44 项、runtime
  45 项、TUI 82 项均通过，其他 workspace 单元/集成/doc tests 全部通过。
- `cargo build --workspace`：通过。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- release 版本：`yunxi 2.0.3`。
- Evaluation Harness：31/31、golden 通过；JSON 可解析，JSONL 恰好一行；质量率
  均为 1.0，proactive violation 与 tool approval bypass 均为 0。
- offline one-shot 与真实 PTY TUI：通过输入、Unicode 长文本、历史滚动、
  `new output below`、End follow-tail 和退出复核。
- DeepSeek live TUI：短流 `LIVE_SHORT_OK` 仅一个 canonical assistant cell；长流
  pinned history 不抢尾；运行中 100x30 动态缩放到 58x18 后 pane 无重叠、composer
  可见；pinned 状态 Ctrl+C 成功取消并保留 partial，下一轮返回
  `RESIZE_NEXT_OK`。
- `git diff --check`：通过，仅有 Windows 行尾提示。
- 额外执行 workspace/TUI `cargo clippy -- -D warnings`，因仓库既有的 persona、
  bottom-pane、host、streaming 等非本报告门禁 lint 未通过；未借本次版本扩大范围
  进行无关重构，不影响上述报告规定的统一验证结果。

### 清理与发布边界

清理前已核验绝对路径。经用户授权，`cargo clean` 从
`D:\YunXi Agent\target` 移除 17,490 个文件、约 4.9 GiB，并删除审计生成的
`D:\YunXi Agent\crates\yunxi-agent-cli\.yunxi`（1 个文件、832 字节）。最终两个
路径均不存在。

本记录与实现进入唯一的 v2.0.3 发布提交，随后立即创建 annotated `v2.0.3` tag，
使用指定 API key non-force 推送 `master` 与新 tag。最终提交、tag object 与远程
refs 以 Git 历史和桌面最终开发日志为准。旧 `v2.0.2`、`v2.0.2-hotfix.1` tag
全程不移动、不删除、不覆盖。

署名：开发者

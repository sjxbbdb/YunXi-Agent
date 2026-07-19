# YunXi Agent v2.0.3 重绘调度与 TUI Snapshot 整改开发报告

- 撰写时间：2026-07-19 15:10:23 +08:00
- 审核报告：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-19-150645-YunXi-Agent-v2.0.3-源码与TUI视觉审核报告.md`
- 原开发报告：`D:\YunXi Agent\docs\reports\2026-07-19-131210-yunxi-agent-v2-0-3-redraw-scroll-resize-development-report.md`
- 开发目录：`D:\YunXi Agent`
- 项目内报告：`D:\YunXi Agent\docs\reports\2026-07-19-151023-yunxi-agent-v2-0-3-redraw-scroll-resize-remediation-development-report.md`
- 桌面报告：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-19-151023-yunxi-agent-v2-0-3-redraw-scroll-resize-remediation-development-report.md`
- 报告类型：当前版本整改开发报告
- 审核结论转化：`v2.0.3` 审核不通过，不可进入 `v2.0.4` 的既定功能开发。

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

## 二、整改目标

`v2.0.3` 已具备大部分重绘调度、滚动锚点与 resize 稳定能力，并且工作区构建、测试和真实在线 Provider 调用均通过。但审核报告明确指出两个总纲硬验收缺口：高频流式帧率证明不足，以及指定终端尺寸完整快照缺失。因此本阶段不是 `v2.0.4` 开发，而是补齐 `v2.0.3` 验收证据与必要实现。

开发目标如下：

1. 为 `RedrawScheduler` 或 host draw 路径抽取可测试 draw spy/计数接口。
2. 使用可注入 `Instant` 或等价时间源模拟 1,000 个高频 stream delta。
3. 断言实际 draw 次数显著低于 delta 数，并且不高于测试窗口对应的 30 FPS 上限。
4. 将默认帧间隔从当前 33ms 的模糊边界调整为严格不超过 30 FPS 的边界，例如 34ms 或 33,334 微秒，并以确定性测试证明。
5. 增加 80x24 与 120x40 的完整 TUI frame snapshot，覆盖 active streaming cell、历史 pinned anchor、new output below、scrollbar、footer 和 composer。
6. 补齐真实 online TUI 复测证据，覆盖滚轮、PgUp/PgDown、End、Ctrl+C、窗口 resize 和恢复 follow-tail。

## 三、版本与发布边界

当前发布对象为：

- 发布提交：`e7401703acab13241985750fed03c5aeee1b87bf`
- 版本号：`2.0.3`
- annotated tag：`v2.0.3`
- tag object：`3d4a204be7c06f26ef64ccf71ea9292e641cf5e9`

开发者必须遵守：

- 已发布 `v2.0.3` tag 不得移动、删除或覆盖。
- 旧 `v2.0.2` 与 `v2.0.2-hotfix.1` tag 继续保持不变。
- 由于 `v2.0.3` 已发布且审核失败，整改发布编号与 tag 策略必须先取得项目负责人明确决定。
- 不得自行创建 `v2.0.3-hotfix.1`，也不得把整改伪装为 `v2.0.4` 正常功能开发。
- 重新审核通过前，不得宣称 `v2.0.3` 完成，也不得进入 `v2.0.4`。

## 四、已通过能力保持要求

整改不得破坏当前已通过能力：

1. 事件驱动合帧调度仍应保持：高频 delta 不得在事件处理器内阻塞。
2. 输入、滚动、resize、错误、取消仍可请求立即帧。
3. 完成与控制态仍可在下一帧绘制。
4. 历史浏览时保持语义 anchor，新输出不抢尾，`End` 恢复 follow-tail。
5. resize 后仍按 cell id 与行偏移重新解析当前语义位置。
6. 主动陪伴默认关闭和权限边界不变，工具审批不得被绕过。
7. 单一 canonical assistant cell、active-stream `Ctrl+C` 取消和普通视图安静边界不回退。

## 五、必须整改的问题

### 1. 1,000 高频 delta draw/FPS 验收

审核发现 `crates\yunxi-agent-tui\src\frame.rs` 中 `coalesces_high_rate_stream_deltas_until_interval_elapses` 只投递 100 个 delta，只断言 pending reason 被集合去重，以及 40ms 后可绘制。该测试没有驱动 1,000 个 delta，没有记录 host 实际 draw 次数，也没有按测试窗口断言 30 FPS 上限。

整改要求：

- 为 `RedrawScheduler` 或 `YunxiTui` host draw 路径提供测试可见的 draw spy/计数接口。
- 测试必须投递 1,000 个高频 `StreamDelta`。
- 使用注入时间源或手动 `Instant` 推进，不得依赖真实 `sleep`。
- 明确测试窗口，例如 1 秒、2 秒或可计算的固定窗口，并断言 draw 次数不超过 `floor(window_seconds * 30)`。
- 同时断言 draw 次数显著低于 1,000，不能只证明 pending reason 集合去重。
- 保留 final、approval/control、error、resize、input、cancel 的即时或下一帧语义测试。

### 2. 30 FPS 硬边界

审核指出默认 `Duration::from_millis(33)` 的理论上限约为 30.3 FPS，不满足“不得高于 30 FPS”的字面硬上限。

整改要求：

- 将默认 frame interval 调整到严格不超过 30 FPS 的边界。
- 推荐使用 `Duration::from_micros(33_334)` 或 `Duration::from_millis(34)`。
- 所有测试中的阈值说明必须与实现一致，避免 33ms 与 30 FPS 的含糊表述。
- 文档和开发日志中需要记录该边界选择及理由。

### 3. 80x24 与 120x40 完整 snapshot

审核发现 `crates\yunxi-agent-tui\src\render.rs` 虽然已有 `TestBackend` 渲染测试，但覆盖的是 80x22、100x24 等相近尺寸；`layout.rs` 只做布局高度断言，缺少总纲指定的 80x24 与 120x40 完整 frame 快照。

整改要求：

- 增加 80x24 完整 TUI snapshot。
- 增加 120x40 完整 TUI snapshot。
- 快照场景至少组合 active streaming cell、历史 pinned anchor、new output below、scrollbar、footer 与 composer。
- 断言无 scrollbar 越界、无异常空白区域、无文字重叠、无 composer 丢失、无 footer/subheader 溢出。
- 快照需要作为可复核证据保留在测试或文档中，而不是只在开发者本地人工观察。

### 4. 真实 online TUI 复测证据

审核报告说明，本次独立审计会话未提供可双向控制的 PTY，因此不能把开发报告中的真实 PTY TUI 描述当作独立审计证据。

整改要求：

- 重新审核前必须准备真实 online TUI 复测证据。
- 记录终端尺寸、Provider/model、操作序列、输入文本、关键状态变化和结果。
- 覆盖滚轮、PgUp/PgDown、End、Ctrl+C、窗口 resize、恢复 follow-tail。
- 如有截图或可复核日志，需要注明路径；涉及 C 盘用户目录或外部目录时必须提前说明。
- 真实 TUI 复测不能替代自动化 snapshot，两者都要具备。

## 六、源码接入点

### `crates\yunxi-agent-tui\src\frame.rs`

当前核心结构为 `RedrawScheduler`、`RedrawReason`、`RedrawPriority`。整改重点：

- 调整默认最小帧间隔，确保不超过 30 FPS。
- 增加可测试的统计或辅助方法，支持 1,000 delta 确定性验证。
- 扩展 `coalesces_high_rate_stream_deltas_until_interval_elapses` 或新增更明确的测试，例如 `throttles_1000_stream_deltas_below_30_fps_limit`。
- 保持 resize/cancel immediate 与 final/control next-frame 语义。

### `crates\yunxi-agent-tui\src\host.rs`

`host.rs` 的 `flush_frame` 与 `draw` 是真实 draw 计数的关键位置。整改重点：

- 抽取 host draw spy 或测试 harness，使测试能统计实际 draw 次数。
- `request_redraw`、`tick`、`flush_frame` 之间的时序必须可用注入时间验证。
- 不得因测试便利把生产路径改成绕过 `RedrawScheduler`。
- active-stream `Ctrl+C` 取消仍应即时，不得被帧率节流吞掉。

### `crates\yunxi-agent-tui\src\render.rs`

`render.rs` 已使用 `ratatui::TestBackend`。整改重点：

- 构造固定 transcript：包含用户消息、active assistant stream、历史 pinned 区、new output below、footer 与 composer。
- 增加 80x24 和 120x40 完整 frame snapshot。
- 断言 transcript title、scrollbar、footer、composer cursor 和区域边界。
- 确保窄宽度、宽宽度下都不出现异常空白或越界。

### `crates\yunxi-agent-tui\src\layout.rs`

layout 层需要提供可独立断言的区域分配：

- 80x24 和 120x40 的 header/subheader/transcript/bottom/footer 区域坐标应稳定。
- 极小尺寸策略不属于本次审核失败点，但不得回退。
- scrollbar rect 不得超出 transcript rect。

### `crates\yunxi-agent-tui\src\viewport.rs`

viewport anchor 已通过大部分审核，但 snapshot 场景必须实际覆盖：

- `FollowTail`
- `Pinned { cell_id, line_offset }`
- `NewOutputBelow`
- resize 后按 cell id 与 line offset 恢复
- `End` 恢复 follow-tail

### `crates\yunxi-agent-tui\src\transcript_layout.rs`

用于 wrap 后行定位与可见范围计算。整改时需要确保：

- snapshot 使用的 wrapped transcript 与真实 render 使用同一计算路径。
- CJK、Emoji、长 token 的宽度计算不因 snapshot 构造而绕开真实逻辑。

## 七、参考源码建议

继续参考总纲指定的 Codex TUI 源码：

- `D:\源码\codex\codex-rs\tui\src\chatwidget\rendering.rs`
- `D:\源码\codex\codex-rs\tui\src\render\renderable.rs`
- `D:\源码\codex\codex-rs\tui\src\wrapping.rs`
- `D:\源码\codex\codex-rs\tui\src\width.rs`

参考边界：

- 只抽取渲染对象分层、可渲染单元、换行、宽度、viewport 稳定与测试思路。
- 不复制上游 UI 代码，不引入上游 TUI runtime。
- 不依赖 `vendor/codex-rs`、`codex-*` crate 或 `yunxi-agent-codex`。
- 非 Rust 参考源码只参考逻辑，必须在 YunXi 自身 Rust 模块中复刻。

## 八、推荐执行顺序

1. 等待用户明确整改发布编号与 tag 策略。
2. 先修正 `RedrawScheduler` 默认帧间隔，选择严格不超过 30 FPS 的值。
3. 为 scheduler/host draw 抽取测试用 draw spy 与注入时间源。
4. 编写 1,000 高频 delta 的确定性 draw/FPS 测试。
5. 补齐 final、approval/control、error、resize、input、cancel 的即时或下一帧回归测试。
6. 构造 80x24、120x40 完整 TUI snapshot 场景。
7. 补齐真实 online TUI 复测证据与记录。
8. 完成一批整改后统一运行验证、清理、提交、tag、推送和重新审核。

## 九、最低测试要求

必须新增或调整以下测试：

1. `1_000` 个高频 `StreamDelta` 在固定时间窗口内 draw 次数显著低于 1,000。
2. draw 次数不超过 30 FPS 上限。
3. final/control next-frame 不被 stream throttle 延迟到不可见。
4. resize/input/error/cancel immediate 不被 stream throttle 吞掉。
5. 80x24 完整 TUI frame snapshot。
6. 120x40 完整 TUI frame snapshot。
7. snapshot 中包含 active stream、pinned history、new output below、scrollbar、footer、composer。
8. snapshot 断言无 scrollbar 越界、无异常空白、无重叠、无 footer/composer 丢失。
9. 现有 TUI 82 项及 workspace 回归继续通过。

## 十、真实 TUI 复核要求

重新审核前必须提供可复核的真实 online TUI 记录：

1. 记录终端尺寸，例如 80x24、120x40 或实际复测尺寸。
2. 记录启动命令、Provider、model 和工作目录。
3. 短流：确认唯一 canonical assistant cell。
4. 长流：在 active stream 中滚动历史，确认新输出不抢尾。
5. 滚轮、PgUp/PgDown、Home/End：确认 anchor 和 follow-tail 状态。
6. resize：缩小、放大后无丢行、无重叠、无跳尾。
7. `Ctrl+C`：active stream 中立即取消，partial 保留，下一轮仍可输入。
8. 普通视图不暴露 thinking、memory/context、协议 JSON、工具参数或内部错误栈。

## 十一、统一验证要求

完成整改后统一验证：

1. `cargo fmt --all`
2. `cargo fmt --all -- --check`
3. `cargo check --workspace`
4. `cargo test --workspace`
5. `cargo build --workspace`
6. `cargo build -p yunxi-agent-cli --release --bins`
7. release `yunxi --version`
8. Evaluation Harness、golden、JSON、JSONL 检查
9. offline TUI smoke
10. online TUI 视觉/交互复核
11. `git diff --check`
12. `git status --short --branch`
13. 发布后远程 master 与 annotated tag refs 核验

涉及 `cargo clean`、递归删除 `.yunxi` 状态目录、删除 smoke 临时目录、强制移动、清空目录、系统级安装/卸载、修改 PATH/注册表/系统配置等操作前，必须先取得用户确认。

## 十二、文档同步要求

以下文档必须与实际代码、验证证据、版本号和发布状态一致：

- `D:\YunXi Agent\README.md`
- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\docs\tui-presentation.md`
- `D:\YunXi Agent\docs\development-log.md`
- `D:\YunXi Agent\docs\reports\2026-07-19-131210-yunxi-agent-v2-0-3-redraw-scroll-resize-development-report.md`
- `D:\YunXi Agent\docs\reports\2026-07-19-151023-yunxi-agent-v2-0-3-redraw-scroll-resize-remediation-development-report.md`
- 如整改版本号或 tag 策略变更，必须同步总纲、索引和状态文档。

## 十三、本报告生成状态

本次任务只根据 `v2.0.3` 审核报告生成整改开发报告，并同步日志。未修改 Rust 源码，未运行构建、测试、清理、提交、推送或创建 Git tag。

当前 Git 状态显示 `crates\yunxi-agent-cli\.yunxi` 为未跟踪目录，来源与审核报告所述在线验证产物一致。本次未读取、删除或清理该目录；如后续需要处理，必须先确认具体路径并取得用户许可。

署名：开发报告撰写者

## 十四、整改版本决策

项目负责人已明确确认使用 `2.0.3-hotfix.1` 和新 annotated
`v2.0.3-hotfix.1` 作为本次整改发布对象。原 `v2.0.3` tag 保持不变；旧
`v2.0.2` 与 `v2.0.2-hotfix.1` tag 也不得移动、删除或覆盖。本次没有进入
`v2.0.4`，且整改候选发布不代表独立重新审核已经通过。

## 十五、实际整改实现

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\frame.rs`：默认最小帧间隔改为
  `Duration::from_micros(33_334)`，理论帧率严格低于 30 FPS；生产
  `record_draw` 路径只在成功 draw 后累加 draw count。
- 同文件新增确定性 1 秒窗口测试，使用手动推进的 `Instant` 投递 1,000 个
  `StreamDelta`，不使用真实 `sleep`，断言 draw count 不超过 30 且显著低于 1,000。
- immediate 回归覆盖 input、scroll、resize、error、cancel；final/control 保持
  next-frame 语义，生产 host draw 路径没有绕过 `RedrawScheduler`。
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\layout.rs` 与
  `D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs`：增加 80x24、120x40
  固定区域与完整 frame 测试，断言区域连续、scrollbar 不越界、每行显示宽度不越界，
  并覆盖 active assistant、pinned history、`new output below`、footer、composer、
  CJK 和 Emoji。
- 新增持久化 snapshot：
  `D:\YunXi Agent\crates\yunxi-agent-tui\src\snapshots\full_frame_80x24.txt`、
  `D:\YunXi Agent\crates\yunxi-agent-tui\src\snapshots\full_frame_120x40.txt`。
- workspace、CLI、TUI、persona context、runtime integration、Evaluation Harness、
  README 与状态文档统一同步为 `2.0.3-hotfix.1`。

## 十六、统一验证结果

2026-07-19 16:30:48 +08:00 前已集中完成报告规定的统一验证：

- `cargo fmt --all`：通过。
- `cargo fmt --all -- --check`：通过。
- `cargo check --workspace`：通过。
- `cargo test --workspace`：通过；TUI 87/87，CLI integration 44、provider 44、
  runtime 45，以及其余 workspace 单元、集成和 doc tests 均通过。
- `cargo build --workspace`：通过。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- release 版本检查：`yunxi 2.0.3-hotfix.1`。
- Evaluation Harness：31/31，golden 为 true；文本、JSON 与单行 JSONL 均通过；
  各质量率为 1.0，主动陪伴违规和 tool approval bypass 均为 0。
- offline one-shot：通过。
- offline PTY TUI 80x24：通过，进程退出码 0。
- DeepSeek live TUI：通过本报告第十节规定的短流、滚动、resize、取消与下一轮复测，
  进程退出码 0。
- `git diff --check`：通过，仅有 Windows 行尾转换提示。

## 十七、真实 TUI 复核证据

持久化复核摘要位于：
`D:\YunXi Agent\docs\reports\evidence\2026-07-19-v2-0-3-hotfix-1-tui-evidence.md`。

live 会话使用 DeepSeek / `deepseek-v4-flash`，初始 80x24，在 active stream 中覆盖
滚轮、PgUp、PgDown、120x40 与 58x18 resize、Home、End 和 Ctrl+C。短流只生成一个
终态 assistant cell；历史锚点显示 `new output below`；End 恢复 tail；Ctrl+C 前确认
`[assistant*]`，取消后 partial 保留并显示取消通知；同一进程下一轮返回
`HOTFIX_NEXT_OK`。共记录 10 个检查点、39,896 bytes 终端输出，退出码为 0。

普通屏幕证据未发现 API key、Authorization、工具参数、thinking、provider wire、
memory/context 原文或内部错误栈。API key 未写入仓库证据。

## 十八、清理与发布状态

2026-07-19 16:50:18 +08:00，经用户明确授权并核验绝对路径后完成阶段清理：

- 清理前 `D:\YunXi Agent\target` 包含 16,570 个文件、5,043,980,114 字节；
  `cargo clean` 报告移除 16,570 个文件、4.7 GiB。
- 删除 `D:\YunXi Agent\crates\yunxi-agent-cli\.yunxi`，清理前为 1 个文件、
  832 字节。
- 删除 9 个实际存在的 `v203-hotfix-*` / `v203_hotfix_*` 临时目标，合计 59 个文件、
  9,147,991 字节；清单中 `v203_hotfix_tui_evidence.json` 清理前已不存在。
- `conpty.node` 首次删除时被本轮证据 Node 进程占用；通过已加载模块和启动时间精确
  定位 PID 26592，只终止该 PID 后删除残留目录，没有终止其他 Node/CodeGraph 进程。
- 最终 `target`、CLI `.yunxi` 和全部 10 个精确临时目标均不存在；没有删除整个
  `D:\YunXi Agent\.tmp`，没有触碰清单以外的目录。

截至本节写入时，Git 提交、annotated `v2.0.3-hotfix.1` tag 和远程推送尚未执行。
本记录将在单一 hotfix 发布提交中入库，随后创建新 tag 并执行 non-force 推送与远程
refs 核验。该候选发布不得被解释为已经通过独立重新审核。

署名：开发者

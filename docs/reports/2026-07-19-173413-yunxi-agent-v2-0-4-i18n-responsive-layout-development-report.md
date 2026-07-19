# YunXi Agent v2.0.4 国际化排版、响应式布局与视觉层级开发报告

- 撰写时间：2026-07-19 17:34:13 +08:00
- 审核报告：`C:\Users\24763\Documents\Codex\2026-07-16\b\audit-v2.0.3-hotfix.1-report-pending-desktop.md`
- 上一版本基线：`v2.0.3-hotfix.1`
- 开发目录：`D:\YunXi Agent`
- 项目内报告：`D:\YunXi Agent\docs\reports\2026-07-19-173413-yunxi-agent-v2-0-4-i18n-responsive-layout-development-report.md`
- 桌面报告：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-19-173413-yunxi-agent-v2-0-4-i18n-responsive-layout-development-report.md`
- 报告类型：下一版本开发报告
- 审核结论转化：`v2.0.3-hotfix.1` 审核通过，可以进入 `v2.0.4` 开发。

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

`v2.0.3-hotfix.1` 已补齐重绘调度、滚动锚点、resize 稳定、1,000 高频 delta 帧率证明，以及 80x24/120x40 完整 TUI frame snapshot。`v2.0.4` 的核心目标是进入总纲图指定的 **国际化排版、响应式布局与视觉层级**。

本阶段要完成：

1. 建立统一 `TextLayout` 显示宽度、折行、裁剪和 cursor 映射模型。
2. 覆盖 `80/100/120/200` 宽度矩阵，证明不同终端宽度下 header、subheader、status、composer、transcript 和 approval 区域稳定。
3. 正确处理 CJK、日文假名、Emoji ZWJ、组合字符、URL、Windows 长路径和代码块的 wrap/cursor 边界。
4. 将 header、subheader、status/footer、composer 的优先级裁剪做成稳定布局，避免重要状态被低优先级信息挤掉。
5. 提升视觉层级的可读性，但不提前实现主题系统、插件功能或后续工具任务化能力。

## 三、版本与发布纪律

当前通过审核的基线为：

- 发布提交：`bf2e81196980d049b22420a491d207b609b563ec`
- 版本号：`2.0.3-hotfix.1`
- annotated tag：`v2.0.3-hotfix.1`
- tag object：`0f738574efe8eaf38811482f2a5f89adbaa1c889`

开发者必须遵守：

- 旧 `v2.0.2`、`v2.0.2-hotfix.1`、`v2.0.3`、`v2.0.3-hotfix.1` tag 均不得移动、删除或覆盖。
- `v2.0.4` 必须对应新的发布提交和新的 annotated tag。
- 验证通过前不得宣称 `v2.0.4` 完成。
- 审核报告显示本轮审计生成了 `D:\YunXi Agent\target` 和未跟踪目录 `D:\YunXi Agent\crates\yunxi-agent-cli\.yunxi`；后续如需清理，必须先确认具体路径并取得用户许可。
- 不得把本阶段范围扩大到 `v2.0.5` 及之后的工具任务化、主题系统、插件功能或其他后续版本内容。

## 四、非目标

以下内容不属于 `v2.0.4`：

- 工具任务化、approval workflow 产品形态扩展。
- 主题系统、配色方案、用户自定义皮肤。
- 插件机制、插件市场、外部工具包集成。
- 新增主动陪伴策略、记忆图谱能力或云后台服务。
- Markdown 富渲染的完整语法高亮、表格高级布局、图片/媒体渲染。
- 默认运行路径依赖上游 Codex TUI runtime。

## 五、源码接入点

### `crates\yunxi-agent-tui\src\transcript_layout.rs`

该文件应成为统一 `TextLayout` 或其核心能力的主要承载点。开发者应抽取或新增：

- display width 计算。
- grapheme-safe 截断。
- word/URL/path/code aware wrapping。
- wrapped line 到原始文本位置的映射。
- cursor byte index 到 visual row/column 的映射。

现有 transcript wrapping、viewport anchor 和 snapshot 必须继续使用同一条布局路径，避免测试走一套、渲染走另一套。

### `crates\yunxi-agent-tui\src\layout.rs`

负责响应式区域分配。`v2.0.4` 应使布局层具备明确优先级：

- header/subheader/footer/composer 的最小高度和压缩策略。
- transcript 区在不同宽度下的稳定剩余空间。
- approval/user input/bottom pane 的高度上限与降级策略。
- 80、100、120、200 宽度下的固定矩阵断言。

### `crates\yunxi-agent-tui\src\render.rs`

负责最终 TUI frame 绘制和 TestBackend 快照。开发者应补齐：

- `80/100/120/200` 宽度矩阵完整 frame snapshot。
- 混合内容场景：CJK、日文、Emoji ZWJ、URL、Windows 长路径、代码块、active assistant、pinned history、new output below、composer。
- 无重叠、无异常空白、无边界越界、无 footer/composer 丢失的断言。
- 视觉层级检查：重要状态优先可见，低优先级信息按规则裁剪。

### `crates\yunxi-agent-tui\src\bottom_pane.rs`

该文件承载 composer、approval 和 user input 状态。`v2.0.4` 必须确保：

- composer cursor 对 CJK、Emoji ZWJ、组合字符和长 URL/路径的 visual column 正确。
- paste、多行输入、长 token 不导致输入区撑破布局。
- approval/user input 与 composer 共用统一显示宽度和裁剪规则。

### `crates\yunxi-agent-tui\src\approval_layout.rs`

CodeGraph 显示 approval 当前已有独立的 `wrap_text`、`truncate_end`、`display_width` 实现。开发者应优先收敛重复逻辑：

- 将 approval command、reason、risk、cwd 的 wrapping 迁移到统一 `TextLayout`。
- Windows 长路径和危险命令不得在窄屏下截断到无法识别风险。
- Approve/Decline 与快捷键行在窄屏下必须保留。

### `crates\yunxi-agent-tui\src\app.rs`

`header_for_width`、`subheader_for_width`、`footer_for_width` 当前承担响应式裁剪。`v2.0.4` 应将这些策略变成明确的优先级模型：

- header：产品/版本优先，其次 provider/live 状态，再其次 model，最后 cwd。
- subheader：view 状态与 cell/debug 异常优先，其次 backend/source。
- footer：当前可执行操作优先，帮助文本按宽度降级。
- 不同宽度下文案裁剪结果应稳定、可测试。

### `crates\yunxi-agent-tui\src\viewport.rs` 与 `frame.rs`

`v2.0.4` 不应重写已通过的 viewport anchor 和 redraw scheduler，但必须确保新的 TextLayout 不破坏：

- `FollowTail`、`Pinned`、`NewOutputBelow`。
- resize 后按 cell id 和 line offset 重算。
- 30 FPS 合帧上限。
- active-stream `Ctrl+C` 取消即时性。

## 六、参考源码建议

继续参考总纲图指定的 Codex TUI 源码：

- `D:\源码\codex\codex-rs\tui\src\chatwidget\rendering.rs`
- `D:\源码\codex\codex-rs\tui\src\render\renderable.rs`
- `D:\源码\codex\codex-rs\tui\src\wrapping.rs`
- `D:\源码\codex\codex-rs\tui\src\width.rs`

参考边界：

- 抽取显示宽度、折行、可渲染对象、优先级裁剪和快照测试思想。
- 不复制上游 UI 代码，不引入上游 TUI runtime。
- 不依赖 `vendor/codex-rs`、`codex-*` crate 或 `yunxi-agent-codex`。
- 非 Rust 参考源码只参考逻辑，必须以 YunXi 自身 Rust 模块复刻。

## 七、推荐技术设计

### TextLayout

建议新增或收敛为一个 TUI 内部布局模块，提供统一接口：

- `measure(value) -> DisplayWidth`
- `wrap(value, width, policy) -> Vec<VisualLine>`
- `truncate(value, width, priority) -> String`
- `cursor_position(value, byte_index, width, policy) -> VisualCursor`
- `source_range_for_visual_line(line) -> Range<usize>`

建议的 wrap policy：

- `NaturalText`：适合普通中英日韩文本。
- `BreakLongToken`：适合长 token 与没有空格的文本。
- `UrlAware`：优先在 `/`、`?`、`&`、`=`, `.`, `-`, `_` 等边界折行。
- `WindowsPathAware`：优先在 `\`、`/`、盘符、扩展名前后折行。
- `CodeBlock`：保留缩进和 fence，避免破坏代码结构。

### PriorityClip

为 header/subheader/footer/composer 建议使用可测试优先级裁剪：

- `MustKeep`：版本、状态、输入提示、approval 决策行。
- `Important`：provider/live、view 状态、风险标签、取消提示。
- `Optional`：cwd、长 model、辅助快捷键说明。
- `DebugOnly`：debug 细节、重复事件计数等可压缩信息。

裁剪应保证重要信息先出现，低优先级信息被稳定省略，而不是按字符串长度随机挤压。

### Width Matrix

建议固定宽度矩阵：

- 80：最小常用终端，必须保证不重叠、不越界。
- 100：普通笔记本终端，验证中等空间下信息优先级。
- 120：开发者常用宽度，验证完整状态栏。
- 200：宽屏终端，验证长路径、URL 和代码块不会产生异常空白。

## 八、最低测试要求

必须新增或调整以下测试：

1. `TextLayout` 对 CJK、日文假名、Emoji ZWJ、组合字符的 display width 和 cursor 映射测试。
2. URL 折行测试，覆盖 query string、fragment、长域名和路径。
3. Windows 长路径折行测试，覆盖盘符、反斜杠、空格目录和长文件名。
4. 代码块折行测试，覆盖 fence、缩进、长行、CJK 注释。
5. composer 多行输入和 cursor visual position 测试。
6. approval command/reason/risk/cwd 的窄屏保真测试。
7. header/subheader/footer priority clipping 测试。
8. 80/100/120/200 宽度完整 TUI frame snapshot。
9. active stream、pinned history、new output below、scrollbar、footer、composer 与混合国际化文本组合快照。
10. v2.0.3-hotfix.1 已通过的 1,000 delta、30 FPS、80x24/120x40、viewport、resize、cancel 测试继续通过。

## 九、真实 TUI 验收要求

正式审核前必须实际运行 TUI 并记录可复核证据：

1. 80、100、120、200 宽度下分别检查 header、subheader、status/footer、composer。
2. 输入 CJK、日文、Emoji ZWJ、URL、Windows 长路径、代码块。
3. 验证 composer cursor、wrap、退格、粘贴和提交后的 transcript 显示。
4. 验证长输出中滚动、resize、End follow-tail 和 `Ctrl+C` 取消不回退。
5. 验证普通视图不暴露 thinking、memory/context、协议 JSON、工具参数或内部错误栈。
6. 记录终端尺寸、Provider/model、操作序列、结果和证据路径。

## 十、统一验证要求

完成一批构建后统一验证：

1. `cargo fmt --all`
2. `cargo fmt --all -- --check`
3. `cargo check --workspace`
4. `cargo test --workspace`
5. `cargo build --workspace`
6. `cargo build -p yunxi-agent-cli --release --bins`
7. release `yunxi --version`
8. Evaluation Harness、golden、JSON、JSONL 检查
9. offline TUI snapshot 与真实 TUI 复核
10. online TUI 视觉/交互复核
11. `git diff --check`
12. `git status --short --branch`
13. 发布后远程 master 与 annotated tag refs 核验

涉及 `cargo clean`、递归删除 `.yunxi` 状态目录、删除 smoke 临时目录、强制移动、清空目录、系统级安装/卸载、修改 PATH/注册表/系统配置等操作前，必须先取得用户确认。

## 十一、文档同步要求

以下文档必须与实际代码、验证证据、版本号和发布状态一致：

- `D:\YunXi Agent\README.md`
- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\docs\tui-presentation.md`
- `D:\YunXi Agent\docs\persona-memory.md`
- `D:\YunXi Agent\docs\development-log.md`
- `D:\YunXi Agent\docs\reports\2026-07-19-173413-yunxi-agent-v2-0-4-i18n-responsive-layout-development-report.md`
- 如新增 evidence、snapshot 索引或状态文档，也必须同步 `v2.0.4` 的实际完成状态。

## 十二、本报告生成状态

本次任务只根据 `v2.0.3-hotfix.1` 审核报告生成 `v2.0.4` 开发报告，并同步日志。未修改 Rust 源码，未运行构建、测试、清理、提交、推送或创建 Git tag。

当前 Git 状态显示 `crates\yunxi-agent-cli\.yunxi` 为未跟踪目录，来源与审核报告所述真实 Provider 尝试产物一致。本次未读取、删除或清理该目录；如后续需要处理，必须先确认具体路径并取得用户许可。

署名：开发报告撰写者

## 十三、实际开发结果

2026-07-19 19:04:13 +08:00 前已按本报告完成集中实现：

- 新增 `D:\YunXi Agent\crates\yunxi-agent-tui\src\text_layout.rs`，统一显示宽度、
  grapheme-safe 折行/截断、visual line 源范围、byte index 到 visual cursor 映射，以及
  `MustKeep`、`Important`、`Optional`、`DebugOnly` 优先级裁剪。
- `transcript_layout.rs` 删除重复 token/grapheme 拆分路径，按普通文本、URL、Windows
  路径和代码块选择统一 wrap policy，并保留稳定 cell/line anchor。
- `bottom_pane.rs` 的 composer 高度、左右移动、删除、退格改为完整 grapheme；
  `render.rs` 的 composer 显示行和 cursor 使用同一份布局结果。
- `approval_layout.rs` 的 command/reason/risk/cwd 使用统一折行/截断；窄屏继续保留
  风险、危险命令、路径身份、Approve/Decline 和快捷行。
- `app.rs` 将 header、subheader、footer 改为显式优先级模型；产品/版本、provider/live、
  view/cell 状态和当前可执行动作先于 model、cwd、帮助或 debug 细节。
- `layout.rs`、`render.rs` 与 snapshots 固定覆盖 80x24、100x30、120x40、200x50；
  内容覆盖 CJK、日文、Emoji ZWJ、组合字符、URL、Windows 路径、代码 fence、active
  assistant、pinned history、new output below、scrollbar、footer 和 composer。
- workspace、CLI、persona、runtime、Evaluation Harness、README 和状态文档同步为
  `2.0.4`；没有进入主题、插件、工具任务化或 `v2.0.5` 范围。

## 十四、统一验证结果

- `cargo fmt --all`、fmt check、`cargo check --workspace`：通过，无编译警告。
- `cargo test --workspace`：通过；TUI 101/101，CLI integration 44/44，provider
  44/44，其余 workspace 单元、集成与 doc tests 全部通过。
- `cargo build --workspace`、release 双 binary 构建：通过。
- `target\release\yunxi.exe --version` 与 `yunxi-agent-cli.exe --version`：均为
  `yunxi 2.0.4`。
- release SHA-256：`yunxi.exe` 为
  `A13B60D1568529AFB27BFE4C17472F02CEA0611FFAFFF1B85261990FFFFDA13C`；
  `yunxi-agent-cli.exe` 为
  `455552F7287509D07EA58624FC9DD291889E2A151DB907BD785EECB0220A9019`。
- Evaluation Harness：31/31、失败 0、golden true；文本、JSON 和单行 JSONL 均
  通过，harness version 为 `2.0.4`，质量率 1.0，主动违规和审批绕过为 0。
- offline one-shot：通过并显示 `[offline]`。
- Windows ConPTY offline/live TUI：通过；详细结果位于
  `D:\YunXi Agent\docs\reports\evidence\2026-07-19-v2-0-4-i18n-responsive-tui-evidence.md`。
- `git diff --check`：通过，仅有 Windows LF/CRLF 转换提示；活动 Rust 版本残留扫描
  未发现 `2.0.3`/`2.0.3-hotfix.1`。

## 十五、真实 TUI 与信息边界

offline ConPTY 会话退出码 0、50,633 bytes、6 个检查点；DeepSeek live /
`deepseek-v4-flash` 会话退出码 0、192,867 bytes、8 个检查点。两者覆盖 80/100/120/200
宽度、国际化粘贴、cursor、退格、提交、滚动、resize 与 End；live 额外覆盖 active
长流、`Ctrl+C` 取消、partial 保留和取消后的终态 `V204_NEXT_OK` 下一轮。

普通终端流未发现 API key、Authorization/Bearer、arguments_json、thinking、
memory/context 或 provider wire 标记。凭据只从进程环境读取，没有写入仓库证据。

## 十六、清理与发布状态

2026-07-19 19:09:14 +08:00 前，验证已通过，并经用户明确授权完成精确清理。清理前：

- `D:\YunXi Agent\target`：16,447 个文件、5,069,034,507 字节；其中本轮
  node-pty/原始 TUI 证据 319 个文件、66,466,457 字节，手写 ConPTY 诊断 4 个文件、
  20,345 字节。
- `D:\YunXi Agent\crates\yunxi-agent-cli\.yunxi`：1 个文件、832 字节；未读取其内容。

`cargo clean` 报告移除 16,447 个文件、4.7 GiB；随后递归删除精确 CLI `.yunxi`
路径。最终 `D:\YunXi Agent\target` 与
`D:\YunXi Agent\crates\yunxi-agent-cli\.yunxi` 均不存在；没有删除仓库 `.tmp`，
没有触碰其他目录，也没有遗留本轮 Node/YunXi/OpenConsole/ConPTY 进程。

截至本节更新时，尚未执行提交、annotated `v2.0.4` tag 或远程推送。旧
`v2.0.2`、`v2.0.2-hotfix.1`、`v2.0.3`、`v2.0.3-hotfix.1` tag 保持不变；发布时
只能新增 annotated `v2.0.4` tag 并 non-force 推送。

署名：开发者

## 十七、独立审核与 Header 整改状态

2026-07-19 20:45:23 +08:00 的独立源码与 TUI 视觉审核判定已发布 `v2.0.4` 不通过：
80 列 header 虽未溢出，但仍显示 model 与 cwd，违反窄屏只保留产品/版本和连接状态的
硬要求。因此本报告的原发布记录保留为历史事实，不能再作为重新审核通过的结论。

项目负责人随后明确选择 `v2.0.4-hotfix.1` 作为当前版本整改发布。整改在
`D:\YunXi Agent\docs\reports\2026-07-19-205131-yunxi-agent-v2-0-4-responsive-header-remediation-development-report.md`
中执行，复核证据位于
`D:\YunXi Agent\docs\reports\evidence\2026-07-19-v2-0-4-hotfix-1-responsive-header-evidence.md`。
新实现用小于 90、90 至 119、120 及以上三档显式构造 header 信息集合，已完成自动化、
真实 Provider、真实 Windows ConPTY、双向 resize、取消和下一轮恢复复核。

已发布 annotated `v2.0.4` tag 保持不变；整改只允许新增 annotated
`v2.0.4-hotfix.1` tag，且仍需独立重新审核后才能宣称审核通过或进入下一版本开发。

署名：开发者

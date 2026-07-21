# YunXi Agent v2.0.8 视觉语义、信息密度与陪伴界面一致性审核报告

- 审核时间：2026-07-21 19:09:09 +08:00
- 审核版本：v2.0.8
- 发布提交：93f9838c7b966ea12e8ab4934ffb45bb7ebd0de0
- annotated tag 对象：7d0a75d63d5e8c437366ce9fcac2d33bbf6d8d04
- 当前 HEAD/origin/master：fa4d5ae6a2a5ead1a55362a924a352f5867e6f47（仅 docs-only 收尾）
- 审核目录：D:\YunXi Agent
- 审核依据：C:\Users\24763\Desktop\YunXi Agent v2.0.0-v2.1.0 TUI与流式输出重构总纲图（复核版）.md 的 v2.0.8 验收要求，以及 2026-07-21-162210-yunxi-agent-v2-0-8-visual-semantics-information-density-development-report.md。

## 一、审核结论

**v2.0.8 审核通过，可进入 v2.0.9 开发。**

本版完成了总纲要求的 TUI 视觉语义收束、信息密度治理、低色彩可读性、窄屏审批冗余，以及既有流式输出与焦点隔离能力的回归验证。源码、自动化测试、真实 DeepSeek Provider、真实 Windows ConPTY、终端栅格视觉复核与远端发布状态均未发现阻塞项。

## 二、总纲要求核验

| 总纲要求 | 审核结果 | 核验说明 |
| --- | --- | --- |
| 集中语义样式接口 | 通过 | 新增 crates/yunxi-agent-tui/src/styles.rs，以 TuiStyleSet、TuiSemanticStyle 和 Full/ANSI16/Monochrome 三档能力统一样式。 |
| 消息与状态视觉语义 | 通过 | user、assistant、progress、tool、action-required、notice、warning、error、muted、header、footer、focus、selection 等均经语义接口渲染；render.rs 与 transcript_layout.rs 不再散落颜色策略。 |
| 信息密度与布局边界 | 通过 | app.rs 通过 TextLayout::priority_line 按 MustKeep、Important、Optional、DebugOnly 裁剪；layout.rs 和全帧 snapshot 覆盖 58x18、80x24、100x30、120x40、200x50，区域不重叠。 |
| 审批、错误、取消的非颜色冗余 | 通过 | Approval 明示 default: Decline 与 Decline (safe default)；错误、取消、风险、快捷键均保留文字标签。Monochrome 下重要状态为粗体或反显，不依赖颜色。 |
| 窄屏和长内容可读性 | 通过 | 58 列帧保留产品、provider、对话、Composer、审批风险与安全默认；低优先级 backend/source/debug 被降级。CJK、Emoji、长路径和长 token 的 TUI 测试及全帧 snapshot 均通过。 |
| v2.0.7-hotfix 焦点隔离不回退 | 通过 | Approval 的滚轮/scrollbar 操作不移动 transcript；Details 的鼠标滚轮和 PgUp/PgDown 仅滚动详情。hotfix ConPTY 与 TUI 回归均通过。 |

## 三、源码与测试复核

### 1. 源码实现

1. styles.rs 将颜色能力检测与语义样式映射封装在 TUI 内部，未引入 CSS、WebView、Go/Python/Codex UI runtime。
2. render.rs 使用语义样式绘制 header、subheader、transcript、Details、Approval、Composer 和 UserInput；Approval 仍保留默认拒绝、风险标签及快捷键提示。
3. transcript_layout.rs 将正常聊天 cell 的类型映射为稳定语义，并继续保持 Unicode 宽度、CJK、Emoji、组合字符、路径和代码块折行行为。
4. app.rs 以当前焦点和状态生成 footer；普通对话优先显示提交/退出，History、Approval、Details 只公布其真实可用操作，未出现提示与实际输入路由不一致。

### 2. 独立门禁结果

| 验证项 | 结果 |
| --- | --- |
| cargo fmt --all -- --check | 通过 |
| cargo check --workspace | 通过 |
| cargo test --workspace | 通过 |
| cargo test -p yunxi-agent-tui | 144/144 通过 |
| cargo build -p yunxi-agent-cli --release --bins | 通过 |
| target\release\yunxi.exe --version | 返回 yunxi 2.0.8 |
| target\release\yunxi.exe eval companion --json | 31/31 通过，golden_passed=true，tool_approval_bypass_count=0 |
| npm.cmd run verify --prefix scripts\conpty\v208 | 2 个 v2.0.8 归档场景通过 |
| npm.cmd run verify --prefix scripts\conpty\v207-hotfix | 2 个 Approval/Details 焦点隔离场景通过 |
| npm.cmd run verify --prefix scripts\conpty\v207 | 8 个历史 ConPTY 场景通过 |
| git diff --check | 通过 |

## 四、真实 Provider、ConPTY 与视觉交互复核

### 1. 真实运行环境

首次在受限沙箱中运行 live Provider 时，凭据变量存在但请求返回 provider transport failed (network)，隔离证据记录为 YX-PROVIDER-001。随后对同一发布二进制以受控网络权限发送固定请求，返回 YUNXI_V208_AUDIT_PING；证明前一失败由审计运行环境网络限制导致，并非凭据缺失或版本功能回归。

在同一受控网络下，使用真实 DeepSeek Provider 和真实 Windows ConPTY 完整运行以下隔离复测：

- 隔离证据目录：D:\YunXi Agent\.tmp\audit-v208-conpty-20260721-network\docs\reports\evidence\frames\v208-conpty
- responsive-density：真实对话返回 YUNXI_V208_CONVERSATION_OK，80x24、200x40、58x18 的 header、transcript、Composer 区域有序，58 列确认诊断信息已降级。
- semantic-low-color：NO_COLOR 下 Approval 默认拒绝、58x18 窄屏审批、Provider error、流式取消及继续交互均通过；验证器确认前景色和背景色单元均为零，仍存在反显选择，并且审批、错误、取消文字可见。
- 隔离副本验证器结果：{"ok":true,"scenarios":2}。

### 2. 人工视觉结论

已对真实 ConPTY 终端栅格帧渲染的 conversation-80.png、conversation-200.png、conversation-58.png、approval-narrow-mono.png、provider-error-mono.png、turn-cancelled-mono.png 进行人工检查，文件位于 D:\YunXi Agent\.tmp\audit-v208-conpty-20260721-network\screens。

1. 80 列普通陪伴对话保持安静，主视图优先展示用户与助手内容；Composer 及其可用命令清晰，不被诊断信息抢占。
2. 200 列仅在空间充足时展开模型、压缩路径、backend/source/debug 信息，主对话区仍是最大阅读区域。
3. 58 列中 header、transcript 与 Composer 没有重叠或越界，长提示正确折行，低优先级诊断不再占据核心空间。
4. 单色窄屏 Approval 保留风险、命令身份、安全默认、Approve/Decline 和快捷键；选择态由 >、文字和反显冗余表达。
5. Provider error 与取消状态显示稳定错误码、可理解摘要和下一步文本，之后 Composer 保持可用；未见乱码、布局断裂或状态遮挡。

## 五、发布与工作树状态

1. 本地 v2.0.8 为 annotated tag，tag object 7d0a75d... 指向发布提交 93f9838...；v2.0.7-hotfix 是其祖先，历史 tag 总数为 48。
2. 远端只读核验结果：refs/tags/v2.0.8=7d0a75d...，refs/heads/master=fa4d5ae...，与本地一致。
3. tag 后仅存在 docs: record v2.0.8 release status 的 docs-only 收尾提交。依用户已确认的发布规则，此项为观察项，不构成阻塞；没有 tag 后功能代码变更，也没有移动、删除或覆盖历史 tag。
4. 本次审核产生 scripts\conpty\v208\node_modules、target、D:\YunXi Agent\.tmp\audit-v208-conpty-20260721 及 D:\YunXi Agent\.tmp\audit-v208-conpty-20260721-network。它们均未清理、删除或移动；清理必须先获得用户对精确绝对路径的明确确认。

## 六、参考源码与 Rust 化核验

本版按总纲的整体能力迁移原则完成，而不是引入外部 UI runtime：

- D:\源码\codex：借鉴语义样式、active view 优先级、局部操作提示和 Details 焦点恢复思想。
- D:\源码\lazygit：借鉴焦点状态、窄屏动作提示、键盘/鼠标一致性和信息密度控制。
- D:\源码\aider：借鉴终端输入节奏、低干扰反馈和可预测交互。

这些能力均以 YunXi 自有 Rust 状态机与 ratatui + crossterm 实现；node-pty 与 @xterm/headless 仅用于项目内 Windows ConPTY 证据采集，不进入默认运行路径。

## 七、下一版本开发建议：v2.0.9

v2.0.9 必须按总纲推进“跨路径兼容、终端恢复与流式故障韧性”，不得把本版视觉收束重新拆回零散样式调整。

1. 在 crates/yunxi-agent-cli/src/interactive.rs 明确 TUI/非 TUI mode resolver；pipe、CI、--json、--jsonl、--no-tui 必须保持无 ANSI 混入的既有字节契约。
2. 在 crates/yunxi-agent-tui/src/host.rs 和 CLI 终端入口完善 RAII terminal guard；正常退出、Ctrl+C、provider 断流、tool failure 与可恢复错误后恢复 cursor、mouse capture、alternate screen 和 ANSI 状态。
3. 在 streaming.rs、chat.rs、debug.rs 建立 timeout、断流、重复 final、错序 delta、cancel 后 late event、无效 UTF-8、超长输出的可恢复策略与可见摘要；故障后必须能继续下一轮。
4. 为 details/debug buffer、tool summary、live tail、history cell 设定上限、截断语义和测试，避免长会话无界增长。
5. 参考 D:\源码\codex 的 terminal lifecycle/stream error suite，参考 D:\源码\k9s 的持续事件流有界历史策略；以 Rust RAII guard 和受控 ring buffer 复刻，不使用后台线程吞掉错误。
6. 验收必须覆盖 TUI/plain/JSON/JSONL/offline/live smoke、终端恢复、各故障 fixture、资源上限及新的 annotated tag v2.0.9。

署名：审核者

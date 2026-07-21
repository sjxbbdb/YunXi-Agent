# YunXi Agent v2.0.9 跨路径兼容、终端恢复与流式故障韧性开发报告

- 撰写时间：2026-07-21 19:35:09 +08:00
- 审核依据：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-21-190909-YunXi-Agent-v2.0.8-视觉语义信息密度审核报告.md`
- 项目内审核报告归档：`D:\YunXi Agent\docs\reports\2026-07-21-190909-yunxi-agent-v2-0-8-visual-semantics-audit-report.md`
- 审核结论转化：`v2.0.8` 审核通过，可进入 `v2.0.9` 开发。
- 发布基线：`v2.0.8`，发布提交 `93f9838c7b966ea12e8ab4934ffb45bb7ebd0de0`，annotated tag object `7d0a75d63d5e8c437366ce9fcac2d33bbf6d8d04`。
- 审核时 `HEAD/origin/master`：`fa4d5ae6a2a5ead1a55362a924a352f5867e6f47`，审核报告标记为仅 docs-only 收尾。
- 开发目录：`D:\YunXi Agent`
- 报告类型：下一阶段开发指令报告，主要面向开发者；本报告不是完成声明。

## 一、硬性约束

以下约束是 `v2.0.9` 及后续版本开发、验证、发布和文档同步的前置要求，不得因为上下文过长、任务拆分、实现困难或局部修复而忽略、弱化或绕过。

1. 开发目录固定在 `D:\YunXi Agent` 及其工作树内。
2. 核心目标是开发一个通用型陪伴 agent；默认运行路径不得依赖 `vendor/codex-rs`、`codex-*` crate 或 `yunxi-agent-codex`。
3. 涉及到源码参考需要先抽取、迁移和构建整体能力，不要在单个点上反复纠结；如果参考的源码不为 Rust 语言，则需参考其逻辑，进行 Rust 复刻。
4. 中间无需频繁验证，完成一批构建后再统一测试和验证。
5. 验证通过前不能宣称完成。
6. 每次阶段结束后要清理编译中间产物，避免占用大量硬盘空间。
7. 每次任务结束后，都要在日志中追加本次工作的详细记录。
8. 记录必须包含时间戳、工作目标、执行流程、修改文件、文件路径、验证结果、提交和推送状态，以及日志结尾处要进行署名，署名为开发报告撰写者。
9. 后续开发报告、设计文档、索引和状态文档要与实际代码状态保持一致。
10. 回复用户使用中文。
11. 之后每个版本更迭都必须提交为新的 Git tag；以前的版本 tag 不得删除，方便出错后回滚。
12. 涉及到 shell 命令时要万分小心，例如递归删除、强制移动、清空目录、系统级安装/卸载、修改 PATH/注册表/系统配置等操作必须得到用户确认。
13. 不可因为过长的上下文忽略硬性要求；如果察觉到硬性要求的效力已经减弱，则必须汇报当前情况。
14. 工作尽量固定在项目文件夹，涉及到其他文件夹的操作，要明确告知用户，尤其是 C 盘的用户目录。

## 二、阶段目标

`v2.0.9` 的主题是跨路径兼容、终端恢复与流式故障韧性。开发者必须把本阶段视为 CLI/TUI 双路径、终端生命周期和流式异常恢复的一次整体加固，不得把 `v2.0.8` 已完成的视觉语义收束拆回零散样式调整。

本阶段必须交付：

1. 明确 TUI、plain、pipe、CI、`--json`、`--jsonl`、`--no-tui`、`--tui` 的 mode resolver 行为，并形成可测试字节契约。
2. 保证非 TUI、JSON、JSONL 与管道路径不混入 ANSI、alternate screen、mouse capture 或 TUI footer 内容。
3. 完善 TUI 终端 RAII guard，覆盖正常退出、Ctrl+C、provider 断流、tool failure、可恢复错误和 panic/early-return 类路径后的 cursor、mouse capture、alternate screen、bracketed paste、focus change 与 raw mode 恢复。
4. 建立流式 timeout、断流、重复 final、错序 delta、cancel 后 late event、无效 UTF-8、超长输出的可恢复策略与可见摘要。
5. 为 details/debug buffer、tool summary、live tail、history cell 设定上限、截断语义和测试，避免长会话无界增长。
6. 保持 `v2.0.8` 已通过的视觉语义、低色彩可读性、信息密度、焦点隔离、真实 Provider 和 ConPTY 证据能力。
7. 统一验证、真实 Windows ConPTY 复审和清理完成前，不得宣称 `v2.0.9` 完成。

## 三、版本与发布边界

当前有效结论是：`v2.0.8` 审核通过，可进入 `v2.0.9`。开发者不得继续把本阶段描述为 `v2.0.8` 收尾，也不得在未完成本阶段故障韧性验收前推进后续版本。

必须遵守：

- `v2.0.8`、`v2.0.7-hotfix`、`v2.0.7` 及全部历史 tag 均不得移动、删除或覆盖。
- `v2.0.9` 代码、测试、证据、文档和日志必须最终收敛到新的发布提交与新的 annotated Git tag。
- tag 名称建议为 `v2.0.9`；正式创建前按用户最新指令确认。
- tag 前必须完成统一验证、真实 ConPTY 证据归档、编译/采集中间产物清理和日志记录。
- tag 后如仅追加 docs-only 状态、日志或报告提交，必须在日志中明确；不得在 tag 后追加未验证功能代码。

## 四、必须保持的既有能力

以下能力已由 `v2.0.8` 审核确认通过，`v2.0.9` 开发不得破坏：

1. `styles.rs` 已集中管理 `TuiStyleSet`、`TuiSemanticStyle` 以及 Full/ANSI16/Monochrome 三档样式能力。
2. user、assistant、progress、tool、action-required、notice、warning、error、muted、header、footer、focus、selection 等视觉语义必须继续通过语义接口渲染。
3. `app.rs` 的 `TextLayout::priority_line` 信息裁剪、58x18/80x24/100x30/120x40/200x50 全帧布局和低色彩可读性不得回退。
4. Approval 默认拒绝、错误、取消、风险和快捷键必须继续具备非颜色冗余。
5. `v2.0.7-hotfix` 的 Approval/Details 焦点隔离不得回退。
6. `node-pty` 与 `@xterm/headless` 只能作为真实 Windows ConPTY 证据采集依赖，不得进入 YunXi 默认运行路径。

## 五、源码接入点

### 1. `crates/yunxi-agent-cli/src/main.rs` 与 `crates/yunxi-agent-cli/src/terminal_mode.rs`

当前 CLI 入口在 `main.rs` 中构造 `TerminalModeRequest`，并在无 prompt 且非 JSON/JSONL 时进入 `interactive::run_interactive`。`terminal_mode.rs` 当前按 `no_tui`、stdin/stdout 是否为 terminal 决定 Plain/Tui；`--tui` 在非终端环境仍回落 Plain。

整改要求：

- 将 mode resolver 的策略写成明确测试矩阵：stdin pipe、stdout pipe、CI、`--json`、`--jsonl`、`--no-tui`、`--tui`、无 prompt、prompt 直跑均应有唯一结果。
- JSON/JSONL 路径不得进入 TUI，不得输出 ANSI、footer、alternate screen 控制序列。
- plain 交互路径必须保持可被管道和 CI 使用；EOF、空输入和错误输出应稳定。
- 若 `--tui` 在不可用终端中回落 Plain，必须有可见、可测试且不污染 JSON/JSONL 的说明；若改为报错，也必须只影响交互路径。

### 2. `crates/yunxi-agent-cli/src/interactive.rs`

`interactive.rs` 当前在 `ResolvedTerminalMode::Plain` 与 `ResolvedTerminalMode::Tui` 之间分发，并由 `InteractiveSession::read_eval_loop` 驱动多轮对话。故障处理目前在每轮 `run_turn` 外层把错误渲染为文本后继续循环。

整改要求：

- 明确 plain 与 TUI renderer 的错误恢复语义：provider transport failed、tool failure、stream断流后必须显示摘要并允许下一轮继续。
- `run_turn`、approval、user input、controls、resume、MCP 状态等交互命令不得破坏 JSON/JSONL 或 no-TUI 契约。
- 将 EOF、Ctrl+C、active turn cancel、provider error 和下一轮恢复纳入测试。
- 对 plain 路径输出进行快照或字节级断言，确保不包含 ANSI 控制序列。

### 3. `crates/yunxi-agent-tui/src/host.rs`

`host.rs` 当前已有 `TerminalGuard`，进入时启用 raw mode、alternate screen、bracketed paste、focus change、mouse capture 和 hide cursor；Drop 中恢复 show cursor、disable mouse/focus/bracketed paste、leave alternate screen 和 disable raw mode。

整改要求：

- 将 `TerminalGuard` 从“已有 Drop”提升为可测试的终端生命周期契约，至少通过抽象 writer/terminal command recorder 或等价测试 seam 断言 enter/drop 顺序。
- 正常退出、Ctrl+C、provider 断流、tool failure、approval 中断、user input 中断和可恢复错误都必须释放终端状态。
- `tick`、`request_approval`、`request_user_input`、`show_details`、`push_error` 等路径不得绕过 guard。
- 如果发生错误，TUI 应恢复终端后把脱敏摘要输出到普通 stderr/stdout，避免用户留在 alternate screen 或无光标状态。

### 4. `crates/yunxi-agent-tui/src/streaming.rs`

`streaming.rs` 当前 `MarkdownStreamCollector` 按 Markdown 边界和 grapheme 边界提交稳定内容，但 buffer 仍以 `String` 累积。`v2.0.9` 必须为 live tail 与异常输入建立上限和可恢复语义。

整改要求：

- 为 `MarkdownStreamCollector` 设置 live tail / buffer 上限，超限时保留尾部、写入截断标记，并避免破坏 UTF-8/grapheme 边界。
- 对无效 UTF-8 的源头做边界设计：如果上游已保证 `String`，则在协议转换层测试 lossy/拒绝策略；如果需要接收 bytes，必须在入口处显式转换并记录摘要。
- timeout 或断流时应 finalize 可展示内容，补充状态摘要，而不是留下永久 active assistant cell。
- 重复 final、错序 delta、cancel 后 late event 的处理必须和 `timeline_store.rs` 一致。

### 5. `crates/yunxi-agent-tui/src/timeline_store.rs`

`timeline_store.rs` 已具备 `ARCHIVED_SESSION_LIMIT = 256`、`SEEN_EVENT_LIMIT = 8192`、duplicate event 计数、可靠 sequence late event 过滤和 archived session。`v2.0.9` 要把这些能力补成故障韧性验收面。

整改要求：

- 增加 timeout/断流/late event/duplicate final/cancel late delta 的专门测试。
- 超过 archive 和 seen event 上限时必须可预测裁剪，不得泄漏内存或错误复活旧 session。
- 断流后应释放 active session，并给 transcript 一个可见但低干扰的摘要。
- `duplicate_event_count`、stream state 和 debug status 应能帮助审计，但不能挤压普通对话主视图。

### 6. `crates/yunxi-agent-tui/src/chat.rs` 与 `crates/yunxi-agent-tui/src/debug.rs`

`chat.rs` 当前已有 `MAX_HISTORY_CELLS = 800`，`DebugBuffer` 当前已有 `MAX_DEBUG_ENTRIES = 200`。这些是很好的基础，但 `v2.0.9` 要补齐 tool summary、details 文本、debug detail 和超长 cell 的截断语义。

整改要求：

- history cell 裁剪要保留最新内容，并确保 viewport anchor 在裁剪后仍合法。
- debug/detail 文本需要有最大长度和截断提示，避免单条超长 payload 占满内存或 details 视图。
- tool summary、provider error、long stdout/stderr、large JSON、代码块和连续 delta 都要有可见摘要和详情入口。
- 截断后仍要执行 `redact_secrets`，不得因为截断顺序泄漏密钥。

## 六、参考源码建议

本次审核报告新增提到 `D:\源码\k9s`，本机已存在；无需新拉取源码。本阶段参考源如下：

1. `D:\源码\codex`：参考 terminal lifecycle、stream error suite、CLI/TUI 路径隔离、错误恢复和发布前测试组织。
2. `D:\源码\k9s`：参考持续事件流、有界历史、watch 事件降噪、资源上限和 UI 不被后台流量拖垮的策略。
3. 可继续参考 `D:\源码\lazygit` 的终端恢复、快捷键稳定性和窄屏信息裁剪。
4. 可继续参考 `D:\源码\aider` 的 plain CLI 输入节奏、错误后继续下一轮和低干扰终端反馈。

所有参考都必须抽取逻辑并以 YunXi 自有 Rust 实现复刻，不得引入 Codex、Go、Python 或其他外部 UI/runtime 作为默认依赖。不得用后台线程吞掉错误；错误必须被明确分类、脱敏、摘要化，并能恢复到下一轮。

## 七、推荐执行顺序

开发者应按整体能力批量构建，不要在单个异常上反复纠结：

1. 先梳理 CLI mode resolver，建立 TUI/plain/JSON/JSONL/pipe/CI/no-tui/tui 的测试矩阵。
2. 再加固 `interactive.rs` 的错误恢复和 plain 输出契约，确保下一轮可继续。
3. 抽象或测试化 `TerminalGuard`，补齐 terminal enter/drop 顺序和异常路径恢复验证。
4. 为流式 controller、timeline store、debug buffer、history cell 和 tool summary 设置上限与截断语义。
5. 加入 timeout、断流、重复 final、错序 delta、cancel late event、无效 UTF-8、超长输出 fixture。
6. 扩展或新建 `scripts\conpty\v209`，覆盖 TUI 终端恢复、plain/JSON/JSONL 无 ANSI、provider error 后继续下一轮、流式断流恢复和长输出截断。
7. 完成一批构建后统一执行验证；验证通过前不得宣称完成。
8. 验证通过后列出精确绝对路径，取得用户确认后清理编译和采集中间产物。
9. 更新 README、TUI 设计文档、脚本索引、提取状态、开发日志和报告索引，确保文档与实际代码一致。

## 八、测试与验收清单

完成代码和测试批量整改后，至少执行以下统一验证：

| 验证项 | 目的 |
| --- | --- |
| `cargo fmt --all -- --check` | 确认 Rust 格式未漂移 |
| `cargo check --workspace` | 确认 workspace 类型与依赖闭环 |
| `cargo test --workspace` | 确认全仓库回归 |
| `cargo test -p yunxi-agent-cli` | 确认 CLI mode resolver、plain/JSON/JSONL 和错误路径 |
| `cargo test -p yunxi-agent-tui` | 确认 TUI terminal guard、流式韧性、history/debug 上限和焦点回归 |
| `cargo build -p yunxi-agent-cli --release --bins` | 确认发布二进制可构建 |
| `target\release\yunxi.exe --version` | 确认版本输出为 `yunxi 2.0.9` |
| `target\release\yunxi.exe eval companion --json` | 确认陪伴评估与工具审批边界无回退 |
| `npm.cmd run verify --prefix scripts\conpty\v209` | 确认真实 Windows ConPTY v2.0.9 场景 |
| `npm.cmd run verify --prefix scripts\conpty\v208` | 确认视觉语义场景未回退 |
| `npm.cmd run verify --prefix scripts\conpty\v207-hotfix` | 确认焦点路由 hotfix 场景未回退 |
| `git diff --check` | 确认无空白和行尾问题 |

新增测试必须证明：

- JSON/JSONL/plain/pipe/CI/no-TUI 输出不包含 ANSI、alternate screen 或 TUI footer。
- TUI 正常退出、Ctrl+C、provider error、tool failure、断流和可恢复错误后终端状态被恢复。
- 流式 timeout、断流、重复 final、错序 delta、cancel late event 后不会留下永久 active cell，并且下一轮可继续。
- history、debug、details、tool summary、live tail 和 seen event/archive 都有上限和可预测裁剪。
- 超长输出和无效输入策略不会破坏 UTF-8/grapheme 边界，也不会泄漏密钥。
- `v2.0.8` 的视觉语义、低色彩、信息密度和 `v2.0.7-hotfix` 的焦点隔离继续通过。

## 九、清理、日志与发布纪律

审核报告观察到以下 `v2.0.8` 审计/采集中间产物仍可能存在：

- `D:\YunXi Agent\scripts\conpty\v208\node_modules`
- `D:\YunXi Agent\target`
- `D:\YunXi Agent\.tmp\audit-v208-conpty-20260721`
- `D:\YunXi Agent\.tmp\audit-v208-conpty-20260721-network`

开发者不得擅自删除。若需要清理，必须先列出精确绝对路径并取得用户明确确认。`v2.0.9` 阶段新产生的 `target`、`scripts\conpty\v209\node_modules`、`scripts\conpty\v209\.work`、`.tmp` 证据目录也必须按同样规则处理。

日志必须追加到：

- `D:\YunXi Agent\docs\development-log.md`
- `C:\Users\24763\Desktop\YunXi Agent开发日志.md`

日志必须包含时间戳、工作目标、执行流程、修改文件、文件路径、验证结果、提交和推送状态，并在结尾署名为开发报告撰写者。

## 十、当前任务状态

本报告只完成 `v2.0.9` 的开发指令撰写，没有修改 YunXi Rust 源码，也没有运行构建、测试、ConPTY 采集或发布流程。本次审核报告新增点名的 `D:\源码\k9s` 已存在，`D:\源码\codex` 也已存在，因此无需新增拉取参考源码。

署名：开发报告撰写者

## 十一、开发执行结果（2026-07-21 21:04:30 +08:00）

### 完成内容

1. 将 workspace 与 CLI/TUI 可见版本统一升级为 `2.0.9`。
2. 在 `crates/yunxi-agent-cli/src/terminal_mode.rs` 建立 Interactive、OneShot、Command、JSON、JSONL 的显式 mode resolver；pipe、CI、`--no-tui`、JSON/JSONL 和非交互路径固定为 Plain，强制 `--tui` 不可用时仅在交互 stderr 给出回落说明。
3. 在 `crates/yunxi-agent-tui/src/host.rs` 建立可测试的终端生命周期 action/sink/state，逐项记录 raw mode、alternate screen、bracketed paste、focus、mouse 与 cursor 状态，进入失败和 Drop 均按反序恢复。
4. 在 Provider 网络边界缓存跨 chunk 的不完整 UTF-8 尾字节，保留合法拆分字符并拒绝确定非法序列，错误中不回显原始字节。
5. 为 Markdown live tail、完整 stream、history cell、debug/detail、tool 字段、history cells、seen event 与 archived session 建立上限；脱敏先于 grapheme 边界截断，保留最新尾部并显示截断标记。
6. 补齐断流冻结、Provider 错误后下一轮、重复 final、可靠乱序 delta、cancel 后 late delta、archive/seen-event 裁剪、viewport anchor 和长 Unicode/代码块输出测试。
7. 新增 `scripts/conpty/v209`，采集并验证 Windows ConPTY 终端恢复、非 TUI 字节隔离、Provider 故障恢复和 371,375 字节 SSE 流取消后恢复；v208 和 v207-hotfix 历史 verifier 继续通过。
8. 保持 v2.0.8 的 Full/ANSI16/Monochrome 语义、58x18/80x24/100x30/120x40/200x50 快照和 v2.0.7-hotfix Approval/Details 焦点隔离不变。

### 主要文件路径

- `D:\YunXi Agent\Cargo.toml`
- `D:\YunXi Agent\Cargo.lock`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\terminal_mode.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-provider\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-provider\tests\provider_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\host.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\streaming.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\timeline_store.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\chat.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\debug.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\output_summary.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\timeline.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`
- `D:\YunXi Agent\scripts\conpty\v209`
- `D:\YunXi Agent\docs\reports\evidence\frames\v209-conpty`
- `D:\YunXi Agent\README.md`
- `D:\YunXi Agent\docs\tui-presentation.md`
- `D:\YunXi Agent\docs\extraction-status.md`

### 验证结果

- `cargo fmt --all -- --check`：通过。
- `cargo check --workspace`：通过。
- `cargo test --workspace`：通过。
- `cargo test -p yunxi-agent-cli`：通过，CLI unit/integration/JSONL 全部通过。
- `cargo test -p yunxi-agent-tui`：通过，159/159。
- `cargo test -p yunxi-agent-provider`：通过，46/46。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- `target\release\yunxi.exe --version`：`yunxi 2.0.9`。
- `target\release\yunxi.exe eval companion --json`：31/31，`golden_passed=true`，`tool_approval_bypass_count=0`。
- `npm.cmd run verify --prefix scripts\conpty\v209`：通过，manifest SHA-256 为 `714b9c2d9c01b1616ffc8789e940ea778559335e20998679a904b5c335fcfdc8`。
- v208 与 v207-hotfix 历史 ConPTY verifier：均通过，2/2、2/2。
- `git diff --check`：通过。

### 清理结果

用户明确确认后，使用绝对 `-LiteralPath`、workspace 前缀校验、重解析点拒绝和逐项删除后 `Test-Path` 验证，已删除且确认不存在：

- `D:\YunXi Agent\target`
- `D:\YunXi Agent\scripts\conpty\v209\node_modules`
- `D:\YunXi Agent\scripts\conpty\v209\.work`
- `D:\YunXi Agent\scripts\conpty\v208\node_modules`
- `D:\YunXi Agent\.tmp\audit-v208-conpty-20260721`
- `D:\YunXi Agent\.tmp\audit-v208-conpty-20260721-network`

`D:\YunXi Agent\.yunxi` 与 `D:\YunXi Agent\crates\yunxi-agent-cli\.yunxi` 可能包含本地状态，未删除。未删除任何 `C:\Users` 用户目录。正式证据、collector、lockfile、报告和全部历史 tag 均保留。

### 提交与推送状态

功能发布提交、annotated `v2.0.9` tag 和 GitHub 推送待执行。本节随功能发布提交进入 tag；发布成功后将追加 docs-only 收尾记录，写明实际 commit/tag object/远程校验结果，不移动 `v2.0.9` 或任何历史 tag。

署名：开发报告撰写者

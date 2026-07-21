# YunXi Agent v2.0.8 视觉语义、信息密度与陪伴界面一致性开发报告

- 撰写时间：2026-07-21 16:22:10 +08:00
- 审核依据：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-21-115531-YunXi-Agent-v2.0.7-hotfix-焦点路由复审审核报告.md`
- 项目内审核报告归档：`D:\YunXi Agent\docs\reports\2026-07-21-115531-yunxi-agent-v2-0-7-hotfix-focus-routing-reaudit-report.md`
- 审核结论转化：`v2.0.7-hotfix` 审核通过，可进入 `v2.0.8` 开发。
- 发布基线：`v2.0.7-hotfix`，发布提交 `3893a7c12cc51768dff583d216fe4563f613bd6b`，annotated tag object `10e182ca46d095274d53b5a167e4f82e52091b1f`。
- 审核时 `HEAD`：`47eb93334fc2758a69f62e7b19fa7d859e4b25e7`
- 开发目录：`D:\YunXi Agent`
- 报告类型：下一阶段开发指令报告，主要面向开发者；本报告不是完成声明。

## 一、硬性约束

以下约束是 `v2.0.8` 及后续版本开发、验证、发布和文档同步的前置要求，不得因为上下文过长、任务拆分、实现困难或局部修复而忽略、弱化或绕过。

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

`v2.0.8` 的主题是视觉语义、信息密度与陪伴界面一致性。开发者必须把本阶段视为 TUI 视觉语义系统的一次整体收束，而不是在 `render.rs` 里零散替换颜色或调几个标题。

本阶段必须交付：

1. 新建 `crates/yunxi-agent-tui/src/styles.rs`，以 `TuiStyleSet` 或等价 facade 集中管理语义样式。
2. 将 user、assistant、progress、tool、action-required、notice、warning、error、muted 等视觉语义从硬编码颜色提升为稳定接口。
3. 统一 `render.rs`、`layout.rs`、`app.rs`、`bottom_pane.rs` 和 `approval_layout.rs` 的信息密度策略，避免 header、subheader、footer、Approval 与 Composer 互相争抢屏幕。
4. 保证错误、审批、取消、等待、工具动作等状态具备文本/符号冗余，不依赖颜色表达。
5. 保持低色彩终端、窄终端、长路径、长模型名、低优先级诊断和普通对话场景均可读。
6. 继续保留 `v2.0.7-hotfix` 已通过的焦点隔离：Approval 不滚动底层历史，Details 鼠标/键盘只滚动详情。
7. 通过统一验证和真实 Windows ConPTY 视觉复审前，不得宣称 `v2.0.8` 完成。

## 三、版本与发布边界

当前有效结论是：`v2.0.7-hotfix` 审核通过，可进入 `v2.0.8`。开发者不得继续把本阶段描述为 `v2.0.7-hotfix` 收尾，也不得跳过 `v2.0.8` 的视觉语义验收。

必须遵守：

- `v2.0.7-hotfix`、`v2.0.7`、`v2.0.6` 及全部历史 tag 均不得移动、删除或覆盖。
- `v2.0.8` 代码、测试、证据、文档和日志必须最终收敛到新的发布提交与新的 annotated Git tag。
- tag 名称建议为 `v2.0.8`；正式创建前按用户最新指令确认。
- tag 前必须完成统一验证、真实 ConPTY 证据归档、编译/采集中间产物清理和日志记录。
- tag 后如仅追加 docs-only 状态、日志或报告提交，必须在日志中明确；不得在 tag 后追加未验证功能代码。

## 四、必须保持的既有能力

以下能力已由 `v2.0.7-hotfix` 审核确认通过，`v2.0.8` 开发不得破坏：

1. `input_map.rs` 已具备 Composer、History、Approval、Details 的显式焦点动作矩阵。
2. Approval 下鼠标滚轮、scrollbar 点击、拖动和释放不得改变 transcript viewport。
3. Details 下鼠标滚轮和 PgUp/PgDown 只改变 `details_scroll`，关闭后恢复 transcript anchor 和原焦点。
4. Composer 草稿、IME committed text、CRLF 粘贴、Emoji/CJK/组合字符编辑和审批草稿恢复不得回退。
5. 流式 assistant cell、取消、真实 Provider、companion evaluation 和 v207 ConPTY 八场景必须继续通过。
6. `node-pty` 与 `@xterm/headless` 只能作为真实 Windows ConPTY 证据采集依赖，不得进入 YunXi 默认运行路径。

## 五、源码接入点

### 1. `crates/yunxi-agent-tui/src/styles.rs`

当前项目尚无 `styles.rs`。本阶段建议新增该模块，并在 `crates/yunxi-agent-tui/src/lib.rs` 中注册为内部模块。

建议接口：

- `TuiStyleSet`：集中暴露语义样式；
- `TuiSemanticStyle` 或等价枚举：覆盖 user、assistant、progress、tool、action_required、notice、warning、error、muted、header、footer、border、focus 等语义；
- 辅助构造器：将 `ratatui::style::Style`、`Color` 和 `Modifier` 封装在 styles 模块内，调用方只表达语义，不直接散落颜色策略；
- 低色彩/无色彩兜底：每种重要状态都必须配合可见文本、符号或标题，不只靠颜色区分。

### 2. `crates/yunxi-agent-tui/src/render.rs`

当前 `render.rs` 直接引入并使用 `ratatui::style::{Color, Modifier, Style}`。`v2.0.8` 应把这些直接样式调用迁移到 `styles.rs` 的语义接口，重点覆盖 header、subheader、details、controls、transcript、Approval、Composer 和 UserInput。

整改要求：

- `render_header` 不再直接写死 `Color::Cyan`、`Color::Gray` 等表现色，而是使用 header/subheader 语义样式。
- `render_controls` 中 companion、cloud、scope、clear effect 和帮助行使用明确语义，不混用随意颜色。
- `render_details` 的 title、border、scroll 提示和内容样式与 Details 焦点语义一致。
- `render_transcript` 中 user、assistant、notice、tool、progress、error 等消息类型必须有稳定视觉语义。
- `render_bottom_pane` 中 Approval、Composer、UserInput 的标题、焦点、输入文本和提示样式统一收口。

### 3. `crates/yunxi-agent-tui/src/layout.rs`

`layout.rs` 已经集中计算 header、transcript、transcript_inner、scrollbar 和 bottom_pane。`v2.0.8` 不应随意打破这套区域边界，而应在现有布局基础上治理信息密度。

整改要求：

- 80 列和 200 列都要有清晰的信息层级：普通对话优先，诊断信息降级。
- 长路径、长模型名、provider source、backend、debug status 只能按优先级裁剪，不得挤压 Composer 或 Approval。
- 低高度终端继续优先保留 bottom pane，不允许 header/footer 与 transcript 重叠。

### 4. `crates/yunxi-agent-tui/src/app.rs`

`app.rs` 已负责 `header_for_width`、`subheader_for_width` 和 `footer_for_width` 的优先级文案。`v2.0.8` 应继续复用 `TextLayout::priority_line`，但要让语义与真实焦点、状态优先级一致。

整改要求：

- header 优先展示产品、provider live/offline 状态和必要模型信息；cwd、backend、source、debug 只作为低优先级信息。
- footer 只展示当前焦点真正可执行的动作，不再把诊断或历史滚动提示放到普通对话的核心位置。
- active session、new output below、history view、Approval、Details 的提示必须与 `input_map.rs` 和 `host.rs` 行为一致。

### 5. `crates/yunxi-agent-tui/src/bottom_pane.rs` 与 `crates/yunxi-agent-tui/src/approval_layout.rs`

`bottom_pane.rs` 已承担 Composer、Approval 和 UserInput 的状态与输入模型；`approval_layout.rs` 已负责审批文本的窄屏布局。`v2.0.8` 应在保持行为不变的前提下统一视觉语义。

整改要求：

- Approval 默认 Decline 的安全语义必须显眼，但不能依赖单一颜色。
- Approve/Decline、风险、原因、命令、cwd 等信息要按优先级分层，窄屏优先保留决策和风险。
- Composer 的焦点状态要克制清晰，普通输入不应被过强状态栏或过多 debug 信息干扰。
- UserInput 和 Composer 复用输入视觉语言，但保持提交/取消语义区分。

## 六、参考源码建议

本次审核报告没有提出新的第三方源码项目需要拉取。本机已存在以下参考源码：

1. `D:\源码\codex`：参考语义样式、active view 优先级、details 关闭后的焦点恢复和局部操作提示思想。
2. `D:\源码\lazygit`：参考焦点状态、窄屏动作提示、键盘/鼠标一致性和信息密度控制。
3. `D:\源码\aider`：参考终端输入节奏、默认安静、低干扰反馈和可预测交互。

审核报告还提到 Claude Code 可观察到的“默认安静、必要动作突出”原则。该项只作为公开可观察产品行为原则参考，不是本次需要拉取的源码项目；开发者不得在缺乏明确源码来源的情况下把不确定仓库接入默认运行路径。

所有参考都必须以 YunXi 自有 Rust 状态机和 `ratatui` 样式接口复刻，不引入 CSS、WebView、Codex UI runtime、Go TUI runtime 或 Python UI runtime。

## 七、推荐执行顺序

开发者应按整体能力批量构建，不要在单个颜色或单个控件上反复纠结：

1. 新建 `styles.rs`，定义 `TuiStyleSet` 与语义样式枚举/访问器。
2. 在 `lib.rs` 注册 styles 模块，先让 `render.rs` 能通过语义接口取得样式。
3. 批量迁移 `render.rs` 中 header、controls、details、transcript、bottom pane 的直接颜色/Modifier 使用。
4. 调整 `app.rs` 的 header/subheader/footer 信息优先级，保持 `TextLayout::priority_line` 与焦点动作一致。
5. 复核 `layout.rs`、`bottom_pane.rs`、`approval_layout.rs` 的窄屏策略，保证高优先级内容不会被低优先级诊断挤压。
6. 增加低色彩、窄屏、长路径、长模型名、错误/警告/审批/取消的 fixture 和 snapshot。
7. 新建或扩展 `scripts\conpty\v208`，采集 80 列、200 列、窄终端、低色彩和关键状态场景。
8. 完成一批构建后统一执行验证；验证通过前不得宣称完成。
9. 验证通过后列出精确绝对路径，取得用户确认后清理编译和采集中间产物。
10. 更新 README、TUI 设计文档、脚本索引、提取状态、开发日志和报告索引，确保文档与实际代码一致。

## 八、测试与验收清单

完成代码和测试批量整改后，至少执行以下统一验证：

| 验证项 | 目的 |
| --- | --- |
| `cargo fmt --all -- --check` | 确认 Rust 格式未漂移 |
| `cargo check --workspace` | 确认 workspace 类型与依赖闭环 |
| `cargo test --workspace` | 确认全仓库回归 |
| `cargo test -p yunxi-agent-tui` | 确认 TUI 样式、布局、输入和焦点回归 |
| `cargo build -p yunxi-agent-cli --release --bins` | 确认发布二进制可构建 |
| `target\release\yunxi.exe --version` | 确认版本输出为 `yunxi 2.0.8` |
| `target\release\yunxi.exe eval companion --json` | 确认陪伴评估与工具审批边界无回退 |
| `npm.cmd run verify --prefix scripts\conpty\v208` | 确认真实 Windows ConPTY v2.0.8 视觉语义场景 |
| `npm.cmd run verify --prefix scripts\conpty\v207-hotfix` | 确认焦点路由 hotfix 场景未回退 |
| `npm.cmd run verify --prefix scripts\conpty\v207` | 确认 v2.0.7 八个历史场景未回退 |
| `git diff --check` | 确认无空白和行尾问题 |

新增测试必须证明：

- `styles.rs` 的每个语义样式都有稳定输出，并且错误、审批、取消、警告不只靠颜色表达。
- 80 列、200 列、58 列窄屏、低高度终端均不出现 header、transcript、bottom pane、footer 重叠。
- 长路径、长模型名、provider source、backend、debug status 会按优先级裁剪。
- 普通对话不被状态栏抢占；必要动作如 Approval、error、cancel、action-required 能被清晰看到。
- `v2.0.7-hotfix` 的 Approval/Details 焦点隔离测试继续通过。

## 九、清理、日志与发布纪律

审核报告观察到以下 `v2.0.7-hotfix` 审计/采集中间产物仍可能存在：

- `D:\YunXi Agent\scripts\conpty\v207-hotfix\node_modules`
- `D:\YunXi Agent\scripts\conpty\v207-hotfix\.work`
- `D:\YunXi Agent\.tmp\audit-v207-hotfix-conpty-20260721\`

开发者不得擅自删除。若需要清理，必须先列出精确绝对路径并取得用户明确确认。`v2.0.8` 阶段新产生的 `target`、`scripts\conpty\v208\node_modules`、`scripts\conpty\v208\.work`、`.tmp` 证据目录也必须按同样规则处理。

日志必须追加到：

- `D:\YunXi Agent\docs\development-log.md`
- `C:\Users\24763\Desktop\YunXi Agent开发日志.md`

日志必须包含时间戳、工作目标、执行流程、修改文件、文件路径、验证结果、提交和推送状态，并在结尾署名为开发报告撰写者。

## 十、实现与验证状态

- 状态更新时间：2026-07-21 17:57:27 +08:00
- 当前阶段：实现、统一验证和发布前精确清理已完成；尚未执行 Git 提交、annotated tag 创建或 GitHub 推送。

已完成实现：

1. 新增 `crates\yunxi-agent-tui\src\styles.rs`，集中定义 `TuiStyleSet`、`TuiSemanticStyle` 与 Full/ANSI16/Monochrome 三档能力；`render.rs` 与 `transcript_layout.rs` 不再散落颜色策略。
2. user、assistant、progress、tool、action-required、notice、warning、error、muted、header、subheader、footer、border、focus、success 和 selection 均使用语义样式。
3. Approval 标题明确显示 `default: Decline`，动作显示 `Decline (safe default)`；低色环境通过文本、粗体和反显保持审批、错误、警告与取消可辨。
4. 90 列以下 subheader 降级 backend/source/debug；Approval 在 58 列优先保留风险、决定和快捷键，布局矩阵覆盖 58/80/200 列及多种低高度。
5. 新增 58x18 全帧 snapshot，并更新 80x24、100x30、120x40、200x50 基线；v2.0.7-hotfix 的 Approval/Details 焦点隔离、草稿、IME、CRLF、Emoji/CJK 和流式单 cell 测试继续通过。
6. 新增 `scripts\conpty\v208` 锁定采集器和 `docs\reports\evidence\frames\v208-conpty` 脱敏证据。响应式场景在完整 assistant 终态后采集 80x24、200x40、58x18；低色场景验证真实 Approval/default Decline、Provider error 和 active-stream Ctrl+C。

统一验证结果：

- `cargo fmt --all -- --check`：通过。
- `cargo check --workspace`：通过。
- `cargo test --workspace`：通过；首次发现并修正 runtime 测试中的旧版本期望后全量重跑通过。
- `cargo test -p yunxi-agent-tui`：144/144 通过。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- `target\release\yunxi.exe --version`：`yunxi 2.0.8`。
- `target\release\yunxi.exe eval companion --json`：31/31，`golden_passed=true`，approval bypass 0。
- `npm.cmd run verify --prefix scripts\conpty\v208`：通过，2 个场景。
- `npm.cmd run verify --prefix scripts\conpty\v207-hotfix`：通过，2 个历史场景。
- `npm.cmd run verify --prefix scripts\conpty\v207`：通过，8 个历史场景。
- 最终 v208 manifest：responsive SHA-256 `00d8620ae74cb3992c2a39918cde7c343fbdbed203e6312ba559c83037bff04b`；semantic SHA-256 `3c0159861774cbc16d101a9dabe052c2037cc4d7ce2992f0b9ae3c99e8911063`。

清理状态：用户确认后，先对 12 个精确绝对路径执行工作区边界、`C:\Users` 禁止边界、存在性、顶层及递归重解析点预检；全部通过后以 `-LiteralPath`、无通配符和 `$ErrorActionPreference='Stop'` 逐项删除，并逐项核验不存在。已删除 `target`、v208/v207-hotfix/v206 的 `node_modules` 与 `.work`、三个 `.tmp` 审计目录、根目录 `.yunxi` 和 CLI `.yunxi`。没有删除用户目录；正式证据、collector、lockfile、报告和历史 tag 均保留。

发布状态：即将创建 v2.0.8 功能发布提交和新的 annotated `v2.0.8` tag，并以 non-force 方式更新 GitHub `master`。`v2.0.7-hotfix` 及全部历史 tag 不得移动、删除或覆盖；远程核验成功前不得写为已发布。

署名：开发报告撰写者

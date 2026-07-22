# YunXi Agent v2.1.0 TUI 与流式输出重构集成发布开发报告

- 撰写时间：2026-07-22 07:42:11 +08:00
- 审核依据：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-22-072005-YunXi-Agent-v2.0.9-终端恢复流式韧性审核报告.md`
- 项目内审核报告归档：`D:\YunXi Agent\docs\reports\2026-07-22-072005-yunxi-agent-v2-0-9-terminal-recovery-streaming-resilience-audit-report.md`
- 审核结论转化：`v2.0.9` 审核通过，可进入 `v2.1.0` 集成发布开发。
- 发布基线：`v2.0.9`，发布提交 `5e199dbac036fb0374f3fde69a389e009aa5a980`，annotated tag object `1bfb8c42b3d736c43f38a3f54aba9dda35c979ee`。
- 审核时 `HEAD/origin/master`：`0288184cdce9e6928d99e106e8dc87c505b80d44`，审核报告标记为 tag 后仅发布安装、收尾与清理文档记录。
- 开发目录：`D:\YunXi Agent`
- 报告类型：下一阶段开发指令报告，主要面向开发者；本报告不是完成声明。

## 一、硬性约束

以下约束是 `v2.1.0` 集成发布及后续版本开发、验证、发布和文档同步的前置要求，不得因为上下文过长、任务拆分、实现困难或局部修复而忽略、弱化或绕过。

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

`v2.1.0` 是 `v2.0.1` 至 `v2.0.9` TUI 与流式输出重构总纲的最终集成发布阶段。开发者不得临时加入新的 UI 概念或陪伴业务模块，重点是把已完成的呈现 contract、流状态机、调度、国际化排版、工具/审批/错误、Composer、焦点、视觉语义和故障韧性串联成可持续回归的发布体系。

本阶段必须交付：

1. 建立 fixture 驱动的 ratatui `TestBackend` snapshot suite，覆盖普通陪伴聊天、长流式 markdown、工具审批/失败、CJK/Emoji/窄窗口、历史 scroll/resize、Details、低色彩、stream fault 与 TUI/plain/JSON 切换。
2. 在需要验证终端控制序列时增加 VT100 transcript suite，确保 ANSI reset、alternate screen、mouse capture、cursor、resize 和退出恢复有可审计证据。
3. 每个关键场景同时保存正常主视图和 debug/details 已开启的 golden，防止内部协议、工具原文或 debug payload 重新进入普通聊天区。
4. 在 PowerShell 与 Windows Terminal 完成人工 smoke，覆盖鼠标、宽字符、ANSI reset、复制、退出恢复和 resize。
5. 使用实际 offline backend 与可用 live Provider 各完成一次端到端会话，证明集成发布不是仅靠 fixture 通过。
6. 将 `v209` verifier 拆分为可配置 capture 输出与纯只读 verify，避免审计命令改写正式发布证据。
7. 统一版本为 `2.1.0`，完成 workspace/CLI/TUI/JSON/JSONL/live/offline/companion 边界验证后，创建新的 annotated tag `v2.1.0`。

## 三、版本与发布边界

当前有效结论是：`v2.0.9` 审核通过，可进入 `v2.1.0` 集成发布开发。开发者不得继续把本阶段描述为 `v2.0.9` 收尾，也不得在未完成集成发布验收前宣称 TUI 与流式输出重构总纲已完成。

必须遵守：

- `v2.0.9`、`v2.0.8`、`v2.0.7-hotfix`、`v2.0.7` 及全部历史 tag 均不得移动、删除或覆盖。
- `v2.1.0` 代码、测试、证据、文档和日志必须最终收敛到新的发布提交与新的 annotated Git tag。
- tag 名称建议为 `v2.1.0`；正式创建前按用户最新指令确认。
- tag 前必须完成统一验证、真实 ConPTY/人工 smoke 证据归档、编译/采集中间产物清理和日志记录。
- tag 后如仅追加 docs-only 状态、日志或报告提交，必须在日志中明确；不得在 tag 后追加未验证功能代码。

## 四、必须保持的既有能力

以下能力已由 `v2.0.9` 审核确认通过，`v2.1.0` 集成发布不得破坏：

1. `terminal_mode.rs` 对 Interactive、OneShot、Command、JSON、JSONL、CI、pipe、`--no-tui` 和强制 TUI 回退已有显式矩阵。
2. 非 TUI 字节契约必须保持无 TUI ANSI、footer、alternate screen 或 mouse capture 混入。
3. `host.rs` 的 terminal lifecycle state 与 RAII guard 必须继续恢复 raw mode、alternate screen、bracketed paste、focus、mouse 和 cursor。
4. Provider 断流、重复 final、可靠乱序 delta、cancel 后 late delta 和跨 chunk UTF-8 边界不得回退。
5. live tail、stream content、history cell、history 数量、debug entries/detail、tool 字段、seen event 与 archived session 的上限和截断语义不得回退。
6. `v2.0.8` 的语义样式、低色彩、信息密度和 `v2.0.7-hotfix` 的 Approval/Details 焦点隔离必须继续通过。
7. `node-pty` 与 `@xterm/headless` 只能作为 Windows ConPTY 证据依赖，不得进入 YunXi 默认运行路径。

## 五、源码接入点

### 1. `crates/yunxi-agent-tui/src/render.rs`、`app.rs`、`layout.rs` 与 `styles.rs`

这些文件承载 TUI 主视图、header/subheader/footer、布局边界和语义样式。`v2.1.0` 不应大改交互语义，而应把已经稳定的视觉与布局行为固化成 snapshot contract。

整改要求：

- 建立覆盖 58x18、80x24、100x30、120x40、200x50 的 TestBackend golden。
- 每个 golden 必须确认 header、transcript、bottom pane、footer、scrollbar 和 details/control 视图不重叠。
- 普通主视图与 debug/details 视图分开保存，禁止内部 payload 渗入普通聊天区。
- 低色彩和 NO_COLOR 场景必须保留文本/符号冗余，不依赖颜色表达状态。

### 2. `crates/yunxi-agent-tui/src/streaming.rs`、`timeline_store.rs`、`chat.rs` 与 `debug.rs`

这些文件承载流式 markdown、session timeline、history cell、debug/detail 存储和有界截断。`v2.1.0` 的任务是把 `v2.0.9` 的故障韧性纳入长期回归，而不是重新设计流式协议。

整改要求：

- 为长流式 markdown、开放代码围栏、跨 chunk UTF-8、重复 final、错序 delta、cancel late event、provider fault 建立 fixture。
- 主视图 golden 与 details/debug golden 同步保存，确认摘要、截断提示、脱敏和详情入口都正确。
- 资源上限测试要能证明 history/debug/tool/stream 裁剪后仍可继续下一轮。
- 普通 transcript 不得出现未经脱敏的 tool 原始输出、provider 原始错误栈或 debug payload。

### 3. `crates/yunxi-agent-cli/src/main.rs`、`interactive.rs` 与 `terminal_mode.rs`

这些文件承载 CLI 入口、交互循环、TUI/plain 分发和 JSON/JSONL 字节契约。`v2.1.0` 必须把这些路径纳入同一套集成发布检查。

整改要求：

- 固定 TUI/plain/JSON/JSONL/no-TUI/pipe/CI/forced-TUI-fallback 的 snapshot 或字节级测试。
- JSON/JSONL 只允许结构化事件，不得出现 TUI footer、ANSI reset 或人工提示文本。
- plain 路径要覆盖 EOF、provider error 后继续下一轮、`/exit`、`/details`、`/controls` 和 resume 的可恢复行为。
- TUI 与 plain 的 renderer 行为应共享语义，不共享终端控制副作用。

### 4. `crates/yunxi-agent-tui/src/host.rs`

`host.rs` 是真实终端生命周期、输入、鼠标、approval、user input 和 redraw 的入口。`v2.1.0` 应把 terminal guard 和焦点路由从单元测试扩展为集成证据。

整改要求：

- VT100 transcript suite 应覆盖 enter、draw、resize、mouse、Ctrl+C、normal exit、provider error、tool failure 和 Drop 恢复。
- PowerShell 与 Windows Terminal 人工 smoke 必须确认退出后光标、复制、鼠标、ANSI reset 和 shell 输入状态正常。
- Approval/Details 的焦点隔离和 Details 滚动必须继续被 v207-hotfix verifier 覆盖。

### 5. `scripts/conpty/v209` 与新增 `scripts/conpty/v210`

审核报告指出 `v209` verifier 实际会重放采集并临时改写正式 `resilience.json` 与 `manifest.json`，虽已恢复但应在 `v2.1.0` 治理。

整改要求：

- 将 `v209` verifier 拆分为只读 verify 和可配置 capture 输出，默认 verify 不得改写正式证据。
- 新建 `scripts/conpty/v210` 或等价目录，覆盖集成发布 smoke，不复用会改写正式证据的命令路径。
- capture 输出必须写入显式 `.tmp` 或用户确认的证据目录；正式 evidence 只有在发布收口时才更新。
- verifier 应能在不联网时跑 offline fixture，在受控网络下补跑 live Provider smoke。

## 六、参考源码建议

本次审核报告没有提出新的外部源码项目需要拉取。本机已存在以下参考源码：

1. `D:\源码\codex`：参考集成回归组织、terminal lifecycle、stream error suite、CLI/TUI 路径隔离和 JSONL 事件契约。
2. `D:\源码\k9s`：参考长期事件流边界、有界历史、watch 事件降噪和资源上限策略。
3. `D:\源码\lazygit`：参考终端恢复、稳定快捷键、窄屏信息裁剪和人工 smoke 组织。
4. `D:\源码\aider`：参考 plain CLI 输入节奏、错误后继续下一轮和低干扰终端反馈。

所有参考都必须抽取逻辑并以 YunXi 自有 Rust 实现复刻，不得引入 Codex、Go、Python 或其他外部 UI/runtime 作为默认依赖。

## 七、推荐执行顺序

开发者应按集成发布批量推进，不要在单个 snapshot 或单个 fixture 上反复纠结：

1. 先盘点 `v2.0.1` 至 `v2.0.9` 的已验证 contract，整理成 `v2.1.0` 集成回归矩阵。
2. 建立 ratatui TestBackend snapshot suite，覆盖普通主视图、debug/details、低色彩、窄屏、长流式、工具审批/失败和历史 scroll/resize。
3. 增加 VT100 transcript suite，验证终端控制序列、ANSI reset、alternate screen、mouse capture、cursor、resize 和退出恢复。
4. 固定 CLI/TUI/plain/JSON/JSONL/no-TUI/pipe/CI 字节契约测试。
5. 拆分 `scripts/conpty/v209` 的 capture 与 verify，新增 `v210` 集成发布 verifier。
6. 运行 offline fixture、可用 live Provider、PowerShell、Windows Terminal 和 Windows ConPTY smoke。
7. 更新 README、TUI 设计文档、脚本索引、提取状态、开发日志、证据索引和报告索引，确保文档与实际代码一致。
8. 完成一批构建后统一执行验证；验证通过前不得宣称完成。
9. 验证通过后列出精确绝对路径，取得用户确认后清理编译和采集中间产物。
10. 创建发布提交和新的 annotated `v2.1.0` tag，旧 tag 全部保持不动。

## 八、测试与验收清单

完成代码和测试批量整改后，至少执行以下统一验证：

| 验证项 | 目的 |
| --- | --- |
| `cargo fmt --all -- --check` | 确认 Rust 格式未漂移 |
| `cargo check --workspace` | 确认 workspace 类型与依赖闭环 |
| `cargo test --workspace` | 确认全仓库回归 |
| `cargo test -p yunxi-agent-cli` | 确认 CLI mode、plain/JSON/JSONL 和交互错误路径 |
| `cargo test -p yunxi-agent-tui` | 确认 TestBackend snapshot、VT100 transcript、焦点、布局、流式和终端恢复 |
| `cargo test -p yunxi-agent-provider` | 确认 Provider UTF-8、断流和错误策略 |
| `cargo build -p yunxi-agent-cli --release --bins` | 确认发布二进制可构建 |
| `target\release\yunxi.exe --version` | 确认版本输出为 `yunxi 2.1.0` |
| `target\release\yunxi.exe eval companion --json` | 确认陪伴评估与工具审批边界无回退 |
| `npm.cmd run verify --prefix scripts\conpty\v210` | 确认 v2.1.0 集成发布 ConPTY 场景 |
| `npm.cmd run verify --prefix scripts\conpty\v209` | 确认终端恢复与流式韧性场景未回退，且 verify 不改写正式证据 |
| `npm.cmd run verify --prefix scripts\conpty\v208` | 确认视觉语义场景未回退 |
| `npm.cmd run verify --prefix scripts\conpty\v207-hotfix` | 确认焦点路由 hotfix 场景未回退 |
| `git diff --check` | 确认无空白和行尾问题 |

新增测试必须证明：

- 普通主视图、debug/details 主视图和低色彩视图各自有 golden，且普通主视图不泄漏内部协议内容。
- TUI/plain/JSON/JSONL/no-TUI/pipe/CI 切换保持字节契约，无 ANSI 污染。
- PowerShell、Windows Terminal 和真实 Windows ConPTY 中退出后终端状态恢复。
- offline 与 live Provider 端到端会话均可完成，故障后可继续下一轮。
- `v209` capture/verify 拆分后，默认 verify 不改写正式 evidence。
- 全部历史 tag 保持不移动、不删除、不覆盖。

## 九、清理、日志与发布纪律

审核报告观察到以下 `v2.0.9` 构建/采集产物仍保留：

- `D:\YunXi Agent\target`
- `D:\YunXi Agent\scripts\conpty\v209\node_modules`
- `D:\YunXi Agent\scripts\conpty\v209\.work`
- `D:\YunXi Agent\.tmp\audit-v209-live-conpty-20260722`

开发者不得擅自删除。若需要清理，必须先列出精确绝对路径并取得用户明确确认。`v2.1.0` 阶段新产生的 `target`、`scripts\conpty\v210\node_modules`、`scripts\conpty\v210\.work`、`.tmp` 证据目录也必须按同样规则处理。

日志必须追加到：

- `D:\YunXi Agent\docs\development-log.md`
- `C:\Users\24763\Desktop\YunXi Agent开发日志.md`

日志必须包含时间戳、工作目标、执行流程、修改文件、文件路径、验证结果、提交和推送状态，并在结尾署名为开发报告撰写者。

## 十、当前任务状态

`2026-07-22 08:46:38 +08:00`，`v2.1.0` 集成开发、统一验证、正式证据归档和经用户确认的精确清理已经完成，进入发布提交与 tag 最终确认阶段。

实施结果：

- workspace 与双 CLI binary 版本统一为 `2.1.0`。
- 新增 fixture-driven Ratatui TestBackend 集成回归，覆盖 normal companion、长流式 Markdown、Approval/tool failure、CJK/Emoji 58 列、history scroll/resize、stream fault 和 monochrome；每个场景均有 main/Details 成对 golden，main 不包含内部 payload。
- 新增 VT100 transcript golden，覆盖 ANSI reset、alternate screen、bracketed paste、focus、mouse、cursor、58x18 resize，以及 normal/Ctrl+C/tool failure/Provider error 退出恢复。
- `TerminalLifecycleState` 在恢复阶段显式写出 `ESC[0m`，并继续按反序恢复 cursor、mouse、focus、paste、alternate screen 和 raw mode；部分进入失败仍只回滚已完成动作。
- `scripts/conpty/v209` 已拆分为显式 `.tmp` capture 与纯只读 verify；正式 v209 evidence 内容 SHA-256 保持 `714b9c2d9c01b1616ffc8789e940ea778559335e20998679a904b5c335fcfdc8`。
- 新增 `scripts/conpty/v210` 综合 release gate；正式 evidence 已归档至 `docs/reports/evidence/frames/v210-conpty`，manifest 记录 evidence SHA-256 `a291a66cf91ee788bba9944edbafbf4c8dfce43bcd2d6c7b2cc78bc6c2990fc6`。
- README、TUI 设计文档、提取状态、脚本索引、Cargo lock、CLI/runtime 版本断言和既有 full-frame snapshots 已同步。

验证结果：

- `cargo fmt --all -- --check`、`cargo check --workspace`、最终 `cargo test --workspace` 全部通过。
- CLI integration 45/45、JSONL 10/10、Provider 46/46、TUI 161/161。
- `cargo build -p yunxi-agent-cli --release --bins` 通过；`yunxi.exe` 与 `yunxi-agent-cli.exe` 均返回 `yunxi 2.1.0`。
- Companion Evaluation 31/31，`golden_passed=true`，`tool_approval_bypass_count=0`。
- release binary 的 offline E2E 与一次真实 DeepSeek Provider E2E 均返回预期标记；API 只在当前 PowerShell 进程中使用，未输出、未持久化。
- v210、v209、v208、v207-hotfix、v207 Windows ConPTY verifier 全部通过；v209/v210 verify 均为只读。
- `git diff --check` 通过；发布前本地与远程基线均为 `0288184cdce9e6928d99e106e8dc87c505b80d44`，历史 tag 共 49 个且尚未移动。

清理结果：用户确认后精确删除 `target`、v209/v210 `node_modules`、v209 `.work`、v209 audit 临时目录、三轮 v210 capture/retry/final 目录及 offline/live E2E 临时目录，共 13 个目录；项目根目录、正式 evidence、`.yunxi`、用户目录、安装目录与历史 tag 均保留。

提交与推送状态：发布提交、annotated `v2.1.0` tag 和 GitHub 非强制推送待执行；正式创建 tag 前必须取得用户最终确认，旧 tag 不删除、不移动、不覆盖。

署名：开发报告撰写者

## 十一、实施结果更新

`2026-07-22 08:46:38 +08:00`，`v2.1.0` 集成开发、统一验证、正式证据归档和经用户确认的精确清理已经完成，进入发布提交与 tag 最终确认阶段。

实施结果：

- workspace 与双 CLI binary 版本统一为 `2.1.0`。
- 新增 fixture-driven Ratatui TestBackend 集成回归，覆盖 normal companion、长流式 Markdown、Approval/tool failure、CJK/Emoji 58 列、history scroll/resize、stream fault 和 monochrome；每个场景均有 main/Details 成对 golden，main 不包含内部 payload。
- 新增 VT100 transcript golden，覆盖 ANSI reset、alternate screen、bracketed paste、focus、mouse、cursor、58x18 resize，以及 normal/Ctrl+C/tool failure/Provider error 退出恢复。
- `TerminalLifecycleState` 在恢复阶段显式写出 `ESC[0m`，并继续按反序恢复 cursor、mouse、focus、paste、alternate screen 和 raw mode；部分进入失败仍只回滚已完成动作。
- `scripts/conpty/v209` 已拆分为显式 `.tmp` capture 与纯只读 verify；正式 v209 evidence 内容 SHA-256 保持 `714b9c2d9c01b1616ffc8789e940ea778559335e20998679a904b5c335fcfdc8`。
- 新增 `scripts/conpty/v210` 综合 release gate；正式 evidence 已归档至 `docs/reports/evidence/frames/v210-conpty`，manifest 记录 evidence SHA-256 `a291a66cf91ee788bba9944edbafbf4c8dfce43bcd2d6c7b2cc78bc6c2990fc6`。
- README、TUI 设计文档、提取状态、脚本索引、Cargo lock、CLI/runtime 版本断言和既有 full-frame snapshots 已同步。

验证结果：

- `cargo fmt --all -- --check`、`cargo check --workspace`、最终 `cargo test --workspace` 全部通过。
- CLI integration 45/45、JSONL 10/10、Provider 46/46、TUI 161/161。
- `cargo build -p yunxi-agent-cli --release --bins` 通过；`yunxi.exe` 与 `yunxi-agent-cli.exe` 均返回 `yunxi 2.1.0`。
- Companion Evaluation 31/31，`golden_passed=true`，`tool_approval_bypass_count=0`。
- release binary 的 offline E2E 与一次真实 DeepSeek Provider E2E 均返回预期标记；API 只在当前 PowerShell 进程中使用，未输出、未持久化。
- v210、v209、v208、v207-hotfix、v207 Windows ConPTY verifier 全部通过；v209/v210 verify 均为只读。
- `git diff --check` 通过；发布前本地与远程基线均为 `0288184cdce9e6928d99e106e8dc87c505b80d44`，历史 tag 共 49 个且尚未移动。

清理结果：用户确认后精确删除 `target`、v209/v210 `node_modules`、v209 `.work`、v209 audit 临时目录、三轮 v210 capture/retry/final 目录及 offline/live E2E 临时目录，共 13 个目录；项目根目录、正式 evidence、`.yunxi`、用户目录、安装目录与历史 tag 均保留。

提交与推送状态：发布提交、annotated `v2.1.0` tag 和 GitHub 非强制推送待执行；正式创建 tag 前必须取得用户最终确认，旧 tag 不删除、不移动、不覆盖。

署名：开发报告撰写者

## 十二、发布结果

`2026-07-22 08:58:09 +08:00`，用户完成最终确认后，YunXi Agent v2.1.0 已发布到 `https://github.com/sjxbbdb/YunXi-Agent`。

- 功能发布提交：`a8293905af55d659d647515786699ab313a51a07`
- 发布 tree：`ee52c162a4f4bbf7c9c97378352ef3e1fe855c1f`
- 发布基线：`0288184cdce9e6928d99e106e8dc87c505b80d44`
- annotated tag：`v2.1.0`
- tag object：`c42ca8b4e2837dcff1e8ae0cd3860936c947875d`
- tag target：`a8293905af55d659d647515786699ab313a51a07`
- author/tagger：`开发者 <developer@yunxi-agent.local>`
- 推送方式：`git push --atomic`，同时更新 `master` 与新 tag，未使用 force
- 远程核验：`master=a8293905af55d659d647515786699ab313a51a07`，标签总数 49→50，原 49 个历史 tag SHA 变化数 0
- 认证：使用用户提供的 GitHub API key，仅存在于当前进程的临时环境变量中，未输出、未写入仓库或 Git 配置，执行后已清除

发布前发现的不被任何引用使用的损坏 loose object `200b814eb133d73d98b9eb5f8e491ea375a77437` 已经用户允许精确移动至 `D:\YunXi Agent\.git\corrupt-object-quarantine\200b814eb133d73d98b9eb5f8e491ea375a77437.corrupt`，未删除；隔离文件 SHA-256 为 `ACE5BB8C395752AA93AFB6A885B1AB4A1CF7FC4DFD4AADCC11E9C04566279A79`。移动后 `git fsck --full` 退出码为 0；未执行 prune、gc 或历史清理。

本节与最终开发日志进入 tag 后 docs-only 收尾提交，仅推进 `master`，不会移动、删除或覆盖 `v2.1.0` 及任何历史 tag。

署名：开发报告撰写者

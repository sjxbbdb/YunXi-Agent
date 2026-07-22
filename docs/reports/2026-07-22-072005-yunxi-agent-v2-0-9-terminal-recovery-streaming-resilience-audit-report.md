# YunXi Agent v2.0.9 跨路径兼容、终端恢复与流式故障韧性审核报告

- 审核时间：2026-07-22 07:20:05 +08:00
- 审核版本：v2.0.9
- 发布提交：5e199dbac036fb0374f3fde69a389e009aa5a980
- annotated tag 对象：1bfb8c42b3d736c43f38a3f54aba9dda35c979ee
- 当前 HEAD/origin/master：0288184cdce9e6928d99e106e8dc87c505b80d44（tag 后仅发布安装、收尾与清理文档记录）
- 审核目录：D:\YunXi Agent
- 审核依据：C:\Users\24763\Desktop\YunXi Agent v2.0.0-v2.1.0 TUI与流式输出重构总纲图（复核版）.md 的 v2.0.9 章节，以及 2026-07-21-193509-yunxi-agent-v2-0-9-cross-path-terminal-recovery-streaming-resilience-development-report.md。

## 一、审核结论

**v2.0.9 审核通过，可进入 v2.1.0 集成发布开发。**

本版满足总纲关于非 TUI 路径隔离、终端状态恢复、Provider/流式故障后的继续交互、跨 chunk UTF-8 边界、有界 history/debug/details/tool/stream 缓冲以及 live/offline TUI 回归的要求。源码、自动化测试、release binary、真实 Windows ConPTY、真实 DeepSeek Provider 和视觉交互复核均未发现阻塞项。

## 二、总纲要求核验

| 总纲要求 | 审核结果 | 核验说明 |
| --- | --- | --- |
| TUI 与 Plain mode resolver | 通过 | crates/yunxi-agent-cli/src/terminal_mode.rs 对 Interactive、OneShot、Command、JSON、JSONL、CI、pipe、--no-tui 和强制 TUI 回退建立显式矩阵。JSON/JSONL 和非交互路径固定为 Plain。 |
| 非 TUI 字节契约 | 通过 | CLI 专项测试覆盖 plain、pipe、CI、JSON、JSONL 和 --no-tui 无 TUI ANSI/footer；真实 ConPTY v209 证据对同一矩阵再次核验。 |
| Terminal guard 恢复 | 通过 | crates/yunxi-agent-tui/src/host.rs 以可测试 lifecycle state 记录 raw mode、alternate screen、bracketed paste、focus、mouse 和 cursor；进入失败与 Drop 均按成功进入动作逆序恢复。正常退出与 Ctrl+C 的 ConPTY 场景通过。 |
| 流式故障韧性 | 通过 | Provider 跨 chunk 不完整 UTF-8 被保留，确定非法字节被拒绝且不回显原文；Provider 断流冻结部分回答、重复 final/可靠乱序 delta/cancel 后 late delta 不会复活旧 session，并允许下一轮输入。 |
| 资源上限与截断 | 通过 | live tail、stream content、history cell、history 数量、debug entries/detail、tool 字段、seen event 与 archived session 均有明确上限；脱敏先于 grapheme 安全截断，保留尾部与可见截断说明。 |
| 既有 TUI 能力 | 通过 | v2.0.8 的语义样式、低色彩、信息密度和 v2.0.7-hotfix 的 Approval/Details 焦点隔离均由历史 ConPTY verifier 与 TUI 测试回归确认。 |

## 三、源码复核

1. TerminalLifecycleState 只对已成功进入的状态执行恢复；EnableRawMode、EnterAlternateScreen、EnableBracketedPaste、EnableFocusChange、EnableMouseCapture、HideCursor 的恢复顺序为 ShowCursor、DisableMouseCapture、DisableFocusChange、DisableBracketedPaste、LeaveAlternateScreen、DisableRawMode。部分 enter 失败回滚与完整逆序恢复都有 recorder 测试。
2. interactive.rs 在 TUI handle 的作用域内执行 banner、输入和 turn loop；turn 错误由 redact_secret_fragments 脱敏后交给 renderer，循环继续处理后续请求。
3. MarkdownStreamController 的 live tail 上限为 64 KiB，完整 stream 上限为 256 KiB；截断从 grapheme 边界保留最新内容，开放代码围栏、Emoji 和组合字符的专项测试通过。
4. history 上限为 800 cells、单 cell 为 64 KiB graphemes；debug entries 上限为 200、存储 detail 为 32 KiB graphemes；tool 字段和详情展示分别有 bounded/redacted 语义，未见未经脱敏的超长原始内容进入主视图。
5. timeline store 对 seen event、completed session、duplicate final、可靠 sequence 乱序与取消后晚到事件有稳定处理，普通 transcript 继续保持错误摘要与详情入口的分层。

## 四、独立验证结果

| 验证项 | 结果 |
| --- | --- |
| cargo fmt --all -- --check | 通过 |
| cargo check --workspace | 通过 |
| cargo test --workspace | 通过 |
| cargo test -p yunxi-agent-cli | CLI unit 15/15、integration 45/45、JSONL 10/10 通过 |
| cargo test -p yunxi-agent-tui | 159/159 通过 |
| cargo test -p yunxi-agent-provider | 46/46 通过 |
| cargo build -p yunxi-agent-cli --release --bins | 通过 |
| target\release\yunxi.exe --version | 返回 yunxi 2.0.9 |
| target\release\yunxi.exe eval companion --json | 31/31 通过，golden_passed=true，tool_approval_bypass_count=0 |
| npm.cmd run verify --prefix scripts\conpty\v209 | 通过；覆盖终端恢复、非 TUI 输出隔离、Provider error 后继续、355 KiB SSE 取消后继续 |
| npm.cmd run verify --prefix scripts\conpty\v208 | 2 个视觉语义场景通过 |
| npm.cmd run verify --prefix scripts\conpty\v207-hotfix | 2 个 Approval/Details 焦点隔离场景通过 |
| npm.cmd run verify --prefix scripts\conpty\v207 | 8 个历史 ConPTY 场景通过 |
| git diff --check | 通过 |

## 五、真实 Provider、Windows ConPTY 与视觉交互复核

### 1. 真实 Provider

以 release binary 对真实 DeepSeek Provider 发送固定请求，返回：

YUNXI_V209_REAL_PROVIDER_OK 我在，当前会话稳定，可以继续工作。

这证明本次审核不局限于离线模式或本地 fixture。

### 2. 真实 Windows ConPTY

v209 自带 verifier 在真实 Windows ConPTY 中通过 normal exit、Ctrl+C exit、plain/pipe/CI/no-tui/forced-TUI-fallback/JSON/JSONL、Provider error 后继续，以及 371375 字节 SSE 流取消并继续下一轮。该 verifier 的 Provider fault 和长流场景使用本地 HTTP fixture，以保证故障注入可重复。

另在受控网络下建立隔离审计目录 D:\YunXi Agent\.tmp\audit-v209-live-conpty-20260722，以真实 DeepSeek Provider 启动 release TUI。最终证据 real-live-final.json 确认：

- 80x24：真实 assistant 返回 YUNXI_V209_REAL_CONPTY_OK，用户消息、中文回答、Transcript 与 Composer 均正常。
- 58x18：长输入和中文回答正确折行，header、transcript、Composer 均无重叠或越界。
- 分步提交 /exit 后终端帧为空，ConPTY 子进程正常退出；没有遗留 yunxi.exe 进程。

### 3. 人工视觉结论

已检查隔离审计图片 D:\YunXi Agent\.tmp\audit-v209-live-conpty-20260722\screens\live-response-80.png 与 live-response-58.png。

1. 80 列主视图安静、清晰，用户与助手消息优先，Composer 命令提示完整可读。
2. 58 列时长文本、ASCII 标记和中文回答均在边界内折行，未挤压 Composer 或出现区域覆盖。
3. 状态信息保持克制，真实 live 标识可见；界面能够在数秒内扫读对话和当前可执行动作。

## 六、发布与观察项

1. 本地 v2.0.9 annotated tag 对象为 1bfb8c42...，指向发布提交 5e199db...；v2.0.8 是其祖先，历史 tag 总数为 49。
2. 本地 origin/master 为 0288184...，tag 后仅有文档安装、收尾和清理记录。依用户既定规则，tag 后 docs-only 为观察项，不构成当前版本阻塞。
3. 本次会话中 GitHub 只读查询先受沙箱限制、后在受控网络下于 34 秒超时，未能再次独立取得远端响应。开发报告已记录发布时的 non-force 推送和远端 tag 核验；当前本地 tag、origin/master 和发布记录一致。此为外部网络核验缺口，不是源码或产品功能阻塞。
4. v209 的 npm verify 实际会重放采集并临时改写正式 resilience.json 与 manifest.json。审核已将其精确恢复到 v2.0.9 tag 内容；本次新的实测数据均保留在隔离审计目录。该脚本命名/输出边界问题列为 v2.1.0 工程治理观察项，不影响 v2.0.9 的运行时验收。

## 七、参考源码与 Rust 化核验

- D:\源码\codex：terminal lifecycle、stream error suite、CLI/TUI 路径隔离和错误恢复的参考逻辑。
- D:\源码\k9s：持续事件流、有界历史、watch 事件降噪和资源上限策略。
- D:\源码\lazygit：终端恢复、稳定快捷键与窄屏信息裁剪。
- D:\源码\aider：plain CLI 输入节奏、错误后继续下一轮和低干扰终端反馈。

YunXi 保持自有 Rust 状态机与 ratatui + crossterm 实现。node-pty 与 @xterm/headless 仅作为 Windows ConPTY 证据依赖，不进入默认运行路径。

## 八、下一版本开发建议：v2.1.0

v2.1.0 是总纲第十个且最终的 TUI 与流式输出重构集成发布，不应临时添加新的 UI 概念或陪伴业务模块。

1. 将 v2.0.1 至 v2.0.9 的呈现 contract、流状态机、调度、国际化排版、工具/审批/错误、Composer、焦点、视觉语义和故障韧性完整串联。
2. 建立 fixture 驱动的 ratatui TestBackend snapshot suite；依赖适配时增加 VT100 transcript suite。固定普通陪伴聊天、长流式 markdown、工具审批/失败、CJK/Emoji/窄窗口、历史 scroll/resize、Details、低色彩、stream fault 与 TUI/plain/JSON 切换回归。
3. 为每个场景保存正常主视图和 debug/details 已开启的 golden，防止内部协议内容重新进入普通聊天区。
4. 在 PowerShell 与 Windows Terminal 完成人工 smoke，覆盖鼠标、宽字符、ANSI reset、复制、退出恢复和 resize；以实际 offline 与可用 live Provider 各完成一次端到端会话。
5. 参考 Codex 的集成回归组织和 k9s 的长期事件流边界，将 v209 verifier 拆分为可配置的 capture 输出与纯只读 verify，避免审计命令改写正式发布证据。
6. 统一版本为 2.1.0，核验全部旧 tag，完成 workspace/CLI/TUI/JSON/JSONL/live/offline/companion 边界验证后，创建新的 annotated tag v2.1.0。

## 九、清理状态

本次审核未执行文件或目录清理。以下构建/采集产物保留，未经用户对精确绝对路径的明确授权不得删除：

- D:\YunXi Agent\target
- D:\YunXi Agent\scripts\conpty\v209\node_modules
- D:\YunXi Agent\scripts\conpty\v209\.work
- D:\YunXi Agent\.tmp\audit-v209-live-conpty-20260722

署名：审核者

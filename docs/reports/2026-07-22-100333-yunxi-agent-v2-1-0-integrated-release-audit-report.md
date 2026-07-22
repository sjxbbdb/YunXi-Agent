# YunXi Agent v2.1.0 TUI 与流式输出重构集成发布审核报告

- 审核时间：2026-07-22 10:03:33 +08:00
- 审核对象：annotated tag `v2.1.0`，tag object `c42ca8b4e2837dcff1e8ae0cd3860936c947875d`，发布提交 `a8293905af55d659d647515786699ab313a51a07`。
- 审核基线：`C:\Users\24763\Desktop\YunXi Agent v2.0.0-v2.1.0 TUI与流式输出重构总纲图（复核版）.md`。
- 工作目录：`D:\YunXi Agent`。
- 审核方式：CodeGraph 源码定位、静态源码审阅、独立 Rust 回归、release binary、Windows ConPTY、真实 DeepSeek Provider、TUI 视觉与交互、Git 发布引用核验。

## 一、审核结论

`v2.1.0` 审核通过。总纲中 `v2.0.1` 至 `v2.0.9` 的 TUI 与流式输出重构已在本版本完成集成回归、终端恢复验证、真实 Provider 验证和发布归档；不存在阻塞进入下一阶段的源码、验证、终端交互或发布 tag 问题。

可以进入下一版本的规划与开发。`v2.1.0` 是现有总纲的终点，下一版本开始前应先补充新的版本总纲；本报告第七节的建议不构成对 `v2.1.0` 的阻塞条件。

## 二、总纲要求对照

| 总纲当下要求 | 审核结果 | 证据与实现 |
| --- | --- | --- |
| 建立 TestBackend 集成回归，覆盖普通陪伴、长流式 Markdown、工具审批/失败、CJK/Emoji/窄屏、历史 scroll/resize、Details、低色彩与 stream fault | 通过 | `crates/yunxi-agent-tui/src/integrated_regression.rs` 新增六组 fixture，并为 main/Details 保存 `integrated_release_v210.txt` golden。普通主视图断言不含内部标记，Details 断言可按需显示诊断标记。 |
| 固化 TUI/plain/JSON/JSONL/no-TUI/pipe/CI/forced fallback 字节契约 | 通过 | `crates/yunxi-agent-cli/src/terminal_mode.rs` 保持显式模式矩阵；独立 v210 ConPTY capture 重放七条 non-TUI 路径，JSON/JSONL 可解析，且无 ANSI/TUI footer 污染。 |
| VT100 terminal lifecycle 覆盖 reset、alternate screen、mouse、cursor、resize 和多类退出 | 通过 | `crates/yunxi-agent-tui/src/host.rs` 的 `TerminalLifecycleState`/RAII guard 保持反序恢复；`vt100_lifecycle_v210.txt` 固化 normal、Ctrl+C、tool failure、Provider error 四类退出序列。 |
| Approval/Details 焦点隔离、Details 独立滚动与既有视觉语义不得回退 | 通过 | TUI 单测、v207-hotfix、v208 verifier 均通过；v210 fixture 同时覆盖 approval/failure、Details 与 monochrome，未见普通 transcript 泄漏内部详情。 |
| 断流、取消、长流与资源上限不得回退 | 通过 | `streaming.rs` 的 64 KiB live tail/256 KiB content 上限仍在；v209 baseline capture 重放 Provider recovery 和 long-stream cancellation，分别恢复成功。 |
| v209 capture/verify 分离，默认 verify 不改写正式证据 | 通过 | `scripts/conpty/v209/capture.js` 仅写入显式输出路径；`verify.js` 对正式 evidence 执行前后指纹检查。本次独立 `verify` 返回 `read_only:true`，v209 SHA-256 仍为 `714B9C2D9C01B1616FFC8789E940EA778559335E20998679A904B5C335FCFDC8`。 |
| offline 与真实 live Provider 均应端到端完成 | 通过 | release binary offline/ConPTY 通过；真实 DeepSeek 单轮返回 `YUNXI_V210_REAL_PROVIDER_OK`，未执行工具，凭据未输出或写入。 |
| 统一为 2.1.0，创建新 annotated tag，保留历史 tag | 通过 | `yunxi.exe` 与 `yunxi-agent-cli.exe` 均输出 `yunxi 2.1.0`；本地 50 个 tag；`v2.0.9` 为 `v2.1.0` 祖先；远端 `v2.1.0` tag object 与本地一致。 |

## 三、源码审核

1. `crates/yunxi-agent-tui/src/integrated_regression.rs` 使用 ratatui `TestBackend` 构建多尺寸 golden，并对普通视图和 Details 分别断言。长 Markdown、宽字符、低色彩、工具失败、错误详情均被放入可重复的 fixture，不依赖真实网络。
2. `crates/yunxi-agent-tui/src/host.rs` 的 terminal lifecycle 明确记录已完成动作，进入失败只回滚已完成动作；恢复先写 ANSI reset，再反序恢复 cursor、mouse、focus、bracketed paste、alternate screen 与 raw mode，Drop guard 覆盖异常退出路径。
3. `crates/yunxi-agent-cli/src/terminal_mode.rs` 对结构化输出、非交互、CI、stdin/stdout 非终端和强制 TUI 回退的优先级明确，避免将 TUI 控制字节写入 JSON/JSONL/plain。
4. `crates/yunxi-agent-tui/src/streaming.rs` 保持 grapheme-safe 的流式裁剪、未闭合代码围栏处理和内容上限；本版本将其纳入集成 fixture 与 ConPTY 长流取消基线。
5. `scripts/conpty/v209` 与 `scripts/conpty/v210` 将正式证据校验和采集输出分开。v210 capture 的输出必须显式指定至临时目录，正式 `docs/reports/evidence` 在本次审核中无修改。

未发现阻塞性的源码缺陷、未验证的发布功能代码或版本不一致。

## 四、独立验证结果

以下命令均在 `D:\YunXi Agent` 独立执行并返回成功：

- `cargo fmt --all -- --check`
- `cargo check --workspace`
- `cargo test --workspace`
- `cargo test -p yunxi-agent-cli`
- `cargo test -p yunxi-agent-tui`
- `cargo test -p yunxi-agent-provider`
- `cargo build -p yunxi-agent-cli --release --bins`
- `target\release\yunxi.exe --version` 与 `target\release\yunxi-agent-cli.exe --version`：均为 `yunxi 2.1.0`
- `target\release\yunxi.exe eval companion --json`：31/31，`golden_passed=true`，`tool_approval_bypass_count=0`
- `npm.cmd run verify --prefix scripts\conpty\v210`：正式 evidence 只读校验通过，SHA-256 `A291A66CF91EE788BBA9944EDBAFBF4C8DFCE43BCD2D6C7B2CC78BC6C2990FC6`
- `npm.cmd run verify --prefix scripts\conpty\v209`、`v208`、`v207-hotfix`、`v207`：全部通过。
- `npm.cmd run capture --prefix scripts\conpty\v210 -- --output-dir .tmp\audit-v210-conpty-20260722 --work-dir .tmp\audit-v210-conpty-work-20260722`：真实 Windows ConPTY capture 和随后只读 verify 均通过。npm prefix 的实际输出路径为 `D:\YunXi Agent\scripts\conpty\v210\.tmp\audit-v210-conpty-20260722`，manifest SHA-256 为 `A99DE04E30D045DBE1D344DC3820324A3A5444C2F6B7C06C12E66E21CEB99C42`；其 mode matrix、terminal recovery、Provider recovery、stream cancellation、mouse/resize/copy boundary 均为成功状态。
- 真实 DeepSeek Provider：release binary 返回预期标记 `YUNXI_V210_REAL_PROVIDER_OK`；只进行了无工具单轮请求。
- `git diff --check`：通过。

## 五、TUI 视觉与交互评审

已在真实 Windows ConPTY 中检查以下审计帧：

- `D:\YunXi Agent\.tmp\audit-v210-live-provider-20260722\visual\conversation-100x30.png`
- `D:\YunXi Agent\.tmp\audit-v210-live-provider-20260722\visual\details-100x30.png`
- `D:\YunXi Agent\.tmp\audit-v210-live-provider-20260722\visual\conversation-58x18.png`

结论：100x30 主会话中 header、transcript、Composer、footer 的边界清楚，中文输入、英文离线响应与自动换行均未发生重叠或裁切；Details 只在显式调用后显示诊断内容，关闭后回到 transcript；58x18 窄屏仍保留 header、可读消息、Composer 和 Ctrl+C 帮助，不出现布局越界。原始 ANSI 帧还确认了 alternate screen、bracketed paste、focus tracking 与 cursor restore；ConPTY 对 ANSI reset 的回显限制由 VT100 单元证据覆盖。

审计辅助工具观察：本次临时视觉采集脚本已经写完上述帧和退出 ANSI 数据，且 release `yunxi.exe` 已退出；其 Node 事件循环未在外层 180 秒窗口内自行结束，外层命令因而超时。该问题只存在于本次 `.tmp` 审计辅助脚本，不属于产品源码或发布脚本；建议在后续审计工具中显式释放 PTY/定时器句柄。

非阻塞观感观察：强制离线状态在 header 显示为 `offline offline`，语义略显重复；`/details` 命令会留在 transcript 中。二者不影响可用性、布局或安全边界，但后续可将状态文案归一化，并评估本地命令是否应使用较低干扰的 transcript 呈现。

## 六、发布与仓库状态

- 远端只读核验：`origin/master=9db8f374f80ab8920b3909ea24417734179c8cc5`；`origin/v2.1.0=c42ca8b4e2837dcff1e8ae0cd3860936c947875d`，与本地 annotated tag object 一致。
- tag 后 `HEAD` 仅比 `v2.1.0` 多出 `docs/development-log.md` 与 v2.1.0 开发报告的收尾修改，按既定规则记为 docs-only 观察项，不构成阻塞。
- `git fsck --full` 返回成功；存在未被引用的历史 dangling 对象，但未报告损坏对象或 tag 引用错误。本次未执行任何 Git 清理、prune、gc、移动或删除。
- 本次审计生成的 `target`、五个 `scripts\conpty\v20x\node_modules`、`.tmp` 中的 capture、Provider、视觉帧与临时脚本均保留。用户未授权精确清理路径，审核者未执行清理。

## 七、后续开发建议与参考源码

现有总纲已于 `v2.1.0` 收口，以下为下一总纲可纳入的非阻塞建议：

1. 统一 offline/forced-offline header 的 provider 与状态文案，消除 `offline offline` 重复；同时定义 slash command 在 transcript 中的呈现规则，并为其补充视觉 golden。
   - 参考：`D:\源码\lazygit` 的低干扰状态行和命令反馈组织；逻辑应以 YunXi 自有 Rust 组件复刻。
2. 把当前审计用的 ConPTY 视觉帧采集整理为受控、可选的开发工具，不将 Node 依赖引入默认运行路径；继续保持正式 evidence 的只读 verify。
   - 参考：`D:\源码\codex` 的终端生命周期与集成回归分层；`D:\源码\k9s` 的长运行事件边界和有界采集策略。
3. 将真实 Provider 的受控 smoke 与离线 fixture 的差异记录为稳定的发布检查表，继续维持错误后下一轮可恢复和无工具默认边界。
   - 参考：`D:\源码\aider` 的 plain CLI 输入节奏与低干扰错误反馈；逻辑采用 Rust 复刻。

所有参考仅用于抽取设计与能力边界，不得把 Codex、Go、Python 或 Node TUI/runtime 作为 YunXi 默认运行依赖。

## 八、最终结论

`v2.1.0` 审核通过，可进入下一版本的规划与开发。

署名：审核者

# YunXi Agent v2.0.5 错误呈现与 ConPTY 整改复审报告

- 审核时间：2026-07-20 11:34:10 +08:00
- 审核对象：`v2.0.5` 当前版本整改工作树
- 基线 tag：`v2.0.5`，提交 `fe15693a039d025a2cdb21b6d7c192207161682b`
- 审核目录：`D:\YunXi Agent`
- 审核依据：`D:\YunXi Agent\docs\reports\2026-07-19-220953-yunxi-agent-v2-0-5-tool-approval-error-activity-development-report.md` 与 `D:\YunXi Agent\docs\reports\2026-07-20-085931-yunxi-agent-v2-0-5-error-presentation-conpty-remediation-development-report.md`
- 审核结论：**不通过。不得进入下一版本开发，必须先完成当前 v2.0.5 整改的发布和证据闭环后重新审核。**

## 一、审核范围

本报告仅复审总纲图中 `v2.0.5` 的工具活动、审批、执行输出、错误呈现、真实 Provider 和 TUI 视觉交互要求。上一次审核阻断的两项为：

1. provider/tool/approval/cancel/terminal/unknown 六类错误的稳定 code、下一步和 details/debug 分层。
2. 真实 Windows ConPTY 在线 TUI 的多宽度视觉与审批交互可复核证据。

## 二、整改源码与功能验证结果

### 已满足的整改项

- `crates/yunxi-agent-tui/src/error_presentation.rs` 现在为六类错误定义统一 `YX-*-001` code、`retryable=yes|no`、可执行 `next:` 和独立 `detail_ref`。
- `crates/yunxi-agent-tui/src/presentation.rs` 已将 ProviderError、命令/工具/MCP 失败、审批拒绝、policy decline、审批取消、通用取消、终端错误、未知错误和非成功 turn 路由到统一错误呈现入口。
- 失败工具把摘要更新到同一个 `ToolActivity`，不额外创建 transcript 错误 cell；普通错误摘要不泄露 Provider wire、内部栈、命令或原始输出，诊断位于 details/debug。
- `crates/yunxi-agent-tui/src/timeline.rs` 允许迟到的结构化 `Cancelled` 精确修正初始 `Declined`，同时保持其他终态不可被重开，避免重复 cell 或 Provider 迟到事件回退状态。
- `crates/yunxi-agent-tui/src/host.rs` 去除了审批决定后的重复 approved/declined notice，审批结果由 runtime event 原位更新工具活动。
- 新增 TUI 集成测试覆盖六类用户可见错误、拒绝/取消状态、敏感诊断隔离和 activity 终态修正。

### 审核者复跑验证

| 验证项 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | 通过 |
| `cargo check --workspace` | 通过 |
| `cargo test --workspace` | 通过，TUI 111/111；执行器 13/13；沙箱验收 8/8；doc tests 通过 |
| `cargo build --workspace` | 通过 |
| `cargo build -p yunxi-agent-cli --release --bins` | 通过 |
| `target\\release\\yunxi.exe --version` | 返回 `yunxi 2.0.5` |
| `target\\release\\yunxi.exe eval companion --json` | 31/31 通过，`golden_passed=true`，`tool_approval_bypass_count=0` |
| 真实 DeepSeek no-TUI JSON 请求 | 返回 `YUNXI_V205_REAUDIT_LIVE_OK` |
| `git diff --check` | 通过；只有 LF/CRLF 工作区警告，无空白错误 |

开发者的 `docs/reports/evidence/2026-07-20-v2-0-5-error-presentation-conpty-evidence.md` 还记录了 DeepSeek live ConPTY 下 80/100/120/200 列、审批批准/拒绝/Ctrl+C、无效 UTF-8、二进制、长输出与非零退出的结果。该文本与源码和测试覆盖相符，但仍存在下列可复核性缺口。

## 三、阻断项

### P1：整改尚未形成可审核的发布提交与 annotated tag

当前 `git describe --tags --always --dirty` 为 `v2.0.5-dirty`。整改代码、文档和证据均处于未提交工作树，旧的 `v2.0.5` tag 仍指向上一次未通过审核的 `fe15693`。目前没有新的整改提交、没有用户确认的整改版本/tag，也没有对应远程 ref。

总纲图和开发硬约束要求每个版本更迭都创建新的 Git tag，历史 tag 不得移动、删除或覆盖。当前状态不能把未提交工作树称为完成发布，也不能用原 `v2.0.5` tag 覆盖整改内容。

**当前版本整改要求：**

1. 在不修改任何历史 tag 的前提下，完成唯一整改发布提交。
2. 先取得用户对整改版本号的确认，再创建新的 annotated tag。例如若用户确认使用 hotfix 语义，可使用新的 `v2.0.5-hotfix.1`；不得自行命名、移动或覆盖 `v2.0.5`。
3. non-force 推送提交和新 tag，复核远程 `master`、新 tag 与历史 refs 后再申请复审。

### P1：ConPTY 视觉证据不可由审核者独立重演

本次工作树仅保留脱敏 Markdown 证据；未保留 ConPTY 驱动、锁定的 Node 依赖、原始脱敏帧或可复跑的采集命令。本机执行 `node -e "require.resolve('node-pty'); require.resolve('@xterm/headless')"` 失败，表明证据中声明的采集环境不在项目或可用全局依赖中。

因此审核者只能阅读开发者转写的终端文本，不能独立运行真实 ConPTY、观察视觉帧或验证批准/拒绝/取消输入。这不满足既定审核流程中“审核者对 CLI TUI 进行视觉和在线交互评审”的可复核要求。

**当前版本整改要求：**

1. 将 ConPTY 采集器、依赖锁定文件和脱敏帧保存到 `D:\YunXi Agent` 工作树内，或提供不需要额外安装依赖的 Rust/Windows 可执行采集方案。
2. 采集器必须能在真实 Provider TUI 中重跑 80/100/120/200 列、审批批准/拒绝/Ctrl+C、无效 UTF-8、二进制、超长输出和非零退出后的继续输入。
3. 证据必须保留时间戳、终端尺寸、脱敏帧、交互序列和命令。原始凭据、Provider wire 和敏感工具参数不得保存。
4. 完成后由审核者独立运行采集器并视觉检查 frame，不能只提交人工整理的 Markdown 摘录。

## 四、参考源码与整改接入建议

本节仅用于完成当前 `v2.0.5` 整改，不构成下一版本需求：

- Codex `bottom_pane/approval_overlay.rs`：保留审批焦点、显式决策与等待态边界；在 YunXi 的 Rust `bottom_pane.rs`、`approval_layout.rs`、`host.rs` 中复刻，不引入 Codex runtime。
- k9s 的事件压缩与详情展开逻辑：继续用于 YunXi 的单一 `ToolActivity` 主 cell 和 details/debug 分层，接入点为 `timeline.rs`、`chat.rs`、`presentation.rs`。
- YunXi `error_presentation.rs`：保持其作为唯一普通错误边界；新增 ConPTY 采集器时只消费普通视图输出，原始诊断仍需经 details/debug 和脱敏策略处理。
- YunXi `ExecOutputDecoder`：继续作为 stdout/stderr 字节处理唯一入口；ConPTY 采集不得重新解释原始二进制或绕过输出完整性元数据。

## 五、工作树状态

审核结束时为 `master...origin/master`，且以下整改内容未提交：

- 已修改：`README.md`、`crates/yunxi-agent-tui/src/{chat,error_presentation,host,presentation,timeline}.rs` 及相关状态文档。
- 未跟踪：当前与历史审核报告、整改开发报告、ConPTY 证据，以及本次真实 Provider 请求产生的 `crates/yunxi-agent-cli/.yunxi/`。
- 本次审核生成了 `target` 和 `.yunxi` 运行状态；未执行删除、清理、强制移动或系统配置操作。

## 六、最终结论

上次审核指出的源码功能缺口已经整改，审核者复跑的格式、构建、测试、评估和真实 Provider 链路均通过。当前不能通过的原因是**发布纪律和审核证据纪律尚未闭环**：整改没有提交/tag/推送，且真实 ConPTY 视觉证据无法由审核者独立复现。

**v2.0.5 当前整改复审不通过。开发者必须完成上述两项 P1 要求后重新提交审核；不得进入下一版本开发。**

署名：审核者

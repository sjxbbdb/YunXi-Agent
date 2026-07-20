# YunXi Agent v2.0.5 工具、审批与错误任务化呈现审核报告

- 审核时间：2026-07-20 08:48:39 +08:00
- 审核版本：`v2.0.5`
- 审核基线：`v2.0.4-hotfix.1`
- 审核目录：`D:\YunXi Agent`
- 审核依据：`D:\YunXi Agent\docs\reports\2026-07-19-220953-yunxi-agent-v2-0-5-tool-approval-error-activity-development-report.md`
- 发布提交：`fe15693a039d025a2cdb21b6d7c192207161682b`
- annotated tag：`v2.0.5`，tag object `caad35a0ecc8eeb066721ab6f9d4e35cd2592602`
- 审核结论：**不通过。不得进入下一版本开发，必须完成本报告列出的 v2.0.5 整改并重新审核。**

## 一、审核范围

本报告只对照总纲图中 `v2.0.5` 的要求审核当前版本，不把后续版本内容作为本版本验收要求。审核覆盖：

1. 单一 `ToolActivity` 主 cell 及终态冻结。
2. 独立审批 bottom pane/overlay、默认安全决策和拒绝/取消路径。
3. `ExecOutputDecoder` 对 stdout/stderr 的字节读取、无效 UTF-8、二进制输出、截断和完整性元数据处理。
4. provider、tool、approval、cancel、terminal、unknown 的稳定错误码、用户下一步与 details/debug 分层。
5. no-TUI、JSON、JSONL 兼容性，真实 Provider 链路，TUI 视觉与交互证据。
6. Git 提交、annotated tag、远程发布状态和文档一致性。

## 二、已满足项

### 1. 工具活动、审批和输出解码主体已实现

- `crates/yunxi-agent-tui/src/timeline.rs` 建立了 `ToolActivityId`、`ToolActivity`、`ToolPhase` 和原位 `apply` 更新；终态活动会忽略迟到事件。对应单元测试 `one_activity_cell_reaches_terminal_state_and_ignores_late_events` 通过。
- `crates/yunxi-agent-tui/src/chat.rs` 以稳定 `TuiCellId` 定位并更新同一工具 cell；`crates/yunxi-agent-tui/src/presentation.rs` 使用外部工具 ID 生成该 cell ID。工具参数、完整输出和命令行进入 details/debug，普通 transcript 只保留摘要。
- `crates/yunxi-agent-tui/src/bottom_pane.rs` 的审批界面默认选中 Decline；`Y/Enter` 批准，`N/Esc` 拒绝，`Ctrl+C` 明确取消。相关窄宽度、风险提示和快捷键测试通过。
- `crates/yunxi-agent-exec/src/lib.rs` 的 `read_pipe` 已改为读取 `Vec<u8>`；`ExecOutputDecoder` 统一处理 stdout/stderr，生成 `DecodedExecOutput` 和 `OutputIntegrity::{Clean, Lossy, Partial}`。无效 UTF-8、二进制省略、截断字节统计和 stdout/stderr 契约均有通过的测试。
- `crates/yunxi-agent-runtime/src/lib.rs` 将执行细节附加到 `CommandCompleted` 的非序列化字段，JSON/JSONL wire shape 保持兼容。

### 2. 发布、构建和离线视觉回归已通过

- `v2.0.5` 是新的 annotated tag，指向唯一发布提交 `fe15693`；本地 tag object 与远程 `origin` 一致。
- `origin/master` 已指向 `fe15693`，远程存在 `refs/tags/v2.0.5`。
- TUI 的 80x24、100x30、120x40、200x50 帧快照和 58 列审批操作可见性测试全部通过。审查快照确认 header 在窄宽度仅保留必要产品/连接信息，100 列保留 model；transcript、滚动条和 composer 区域没有测试可见的重叠。
- 真实 DeepSeek no-TUI JSON 请求成功返回 `YUNXI_V205_AUDIT_LIVE_OK`。该请求证明在线 Provider、流式事件与会话保存链路可用。

## 三、未通过项

### P1：审批、取消和终端错误没有完整接入稳定错误呈现

总纲图要求 provider、tool、approval、cancel、terminal、unknown 六类错误都应在普通视图中给出稳定 error code、用户可执行的下一步、是否可重试，并把原始诊断留在 details/debug。

`crates/yunxi-agent-tui/src/error_presentation.rs:2-92` 已定义六类 `ErrorCategory` 和 `YX-*-001` 代码。但 `crates/yunxi-agent-tui/src/presentation.rs:316-342` 对 `ApprovalCompleted { approved: false }` 仅将工具活动标为 `Declined` 或 `PolicyDeclined`，没有使用 `ErrorPresentation::for_category(ErrorCategory::Approval, ...)`，普通视图没有 `YX-APPROVAL-001` 或明确下一步。`presentation.rs:481-488` 的 `Cancelled` 只显示 `current turn cancelled`，没有 `YX-CANCEL-001` 和下一步。`presentation.rs:599-607` 的 `present_error` 仍固定输出 `operation failed`，未保证 terminal 路径获得 `YX-TERMINAL-001`。

现有 `error_presentation` 测试只验证分类函数和静态结构本身，未验证这六类错误事件都被映射到普通 TUI 视图。因此实现没有满足“错误分类必须提供稳定 error code 和用户可执行下一步”的完整交付条件。

**整改要求：**

1. 在事件呈现边界建立唯一的错误呈现入口，并将 `AgentEvent::ApprovalCompleted(false)`、`AgentEvent::Cancelled`、终端宿主失败和通用 `present_error` 映射到对应 `ErrorPresentation`。
2. 普通 transcript 或对应 `ToolActivity.summary` 必须显示代码、简明摘要和下一步；命令、Provider wire、原始错误和未脱敏参数只允许保留在 details/debug。
3. 新增事件到 TUI cell 的集成测试，逐一断言 provider/tool/approval/cancel/terminal/unknown 六类场景的代码、下一步、重试元数据和敏感诊断隔离。仅测试 `classify_error` 不可作为验收证据。

### P1：缺少真实在线 TUI 的可复核视觉与审批交互证据

总纲图要求正式审核前，在真实 ConPTY 中完成在线 TUI 视觉和交互复核：触发审批、批准、拒绝、取消、无效 UTF-8/二进制/长输出、非零退出码后的继续输入，并记录可复核证据。

本次审核已完成真实在线 Provider 的 no-TUI JSON 请求，也已复核离线 TUI 帧快照与 105 项 TUI 测试；但当前 shell 不提供可捕获的 Windows ConPTY 主机，项目现有开发报告同样明确在线 TUI 仍待在真实 ConPTY 复核。因此没有证据证明真实 Provider 会话中的审批 bottom pane 焦点、拒绝/取消、单 cell 原位更新和错误摘要在实际终端帧中均符合要求。离线快照与单元测试不能替代该项。

**整改要求：**

1. 在 Windows ConPTY 或等价真实终端中，使用实际 Provider 会话录制 80、100、120、200 列 TUI 帧，并保留命令、终端尺寸、时间戳、交互步骤和脱敏后的结果。
2. 在同一在线 TUI 会话中实际触发一个需要审批的无害工具调用，分别验证批准、拒绝和 `Ctrl+C` 取消；拒绝和取消必须不执行工具、保持后续输入可用、更新同一 activity cell。
3. 对无效 UTF-8/二进制、超长 stdout/stderr 和非零退出码提供真实 TUI 证据，确认普通视图不泄露原始内容，details/debug 能审计完整性和截断元数据。

## 四、参考源码与整改接入建议

以下参考均是本版本整改所需的实现依据，不构成下一版本需求：

- Codex 的 `bottom_pane/approval_overlay.rs`：参考决策焦点、等待态与退出态的交互边界。仅迁移交互逻辑，在 YunXi 的 Rust `bottom_pane.rs`、`approval_layout.rs`、`app.rs` 中复刻；不得引入上游 runtime 或 `codex-*` 依赖。
- k9s 的事件压缩与详情展开思路：用于保持一个工具调用只占用一个主 activity cell，原始事件进入 details/debug。YunXi 接入点为 `timeline.rs`、`chat.rs`、`presentation.rs`。
- YunXi 现有 `error_presentation.rs`：将其从“分类定义”扩展为“所有用户可见错误的唯一呈现边界”；由 `presentation.rs`、`app.rs` 和 CLI TUI 宿主统一调用。
- YunXi 现有 `ExecOutputDecoder`：继续保持 `exec/src/lib.rs` 的字节级解码为唯一入口，禁止在 CLI/TUI 中重新直接读取或拼接未解码输出。

## 五、验证记录

以下命令均在 `D:\YunXi Agent` 执行：

| 验证项 | 结果 |
| --- | --- |
| `git diff --check v2.0.4-hotfix.1..v2.0.5` | 通过 |
| `cargo fmt --all -- --check` | 通过 |
| `cargo check --workspace` | 通过 |
| `cargo test --workspace` | 通过 |
| `cargo build --workspace` | 通过 |
| `cargo test -p yunxi-agent-tui` | 105/105 通过 |
| `cargo test -p yunxi-agent-exec` | 13/13 执行器测试、8/8 沙箱验收测试通过 |
| `cargo build -p yunxi-agent-cli --release --bins` | 通过 |
| `target\\release\\yunxi.exe --version` | 返回 `yunxi 2.0.5` |
| `target\\release\\yunxi.exe eval companion --json` | 31/31 通过，`golden_passed=true`，`tool_approval_bypass_count=0` |
| 真实 DeepSeek no-TUI JSON 请求 | 通过，返回 `YUNXI_V205_AUDIT_LIVE_OK` |
| 真实在线 TUI/ConPTY 审批视觉交互 | 未提供可复核证据，不通过 |
| `git ls-remote --refs origin refs/heads/master refs/tags/v2.0.5` | 远程 master=`fe15693`，远程 tag object=`caad35a0` |

## 六、工作树与发布状态

- 审核结束时：`master...origin/master`。
- 真实在线请求产生未跟踪目录：`D:\YunXi Agent\crates\yunxi-agent-cli\.yunxi\`。
- 该目录是本次审核的运行状态产物，未读取、未删除、未清理。`target` 同样由构建产生，未清理。
- 未执行递归删除、强制移动、目录清空、系统安装/卸载、PATH/注册表/系统配置修改。

## 七、最终结论

`v2.0.5` 的 ToolActivity、审批底栏、输出解码、离线 TUI 回归、在线 Provider、构建和发布 tag 均有实质完成，但“六类错误在普通视图中的统一稳定呈现”和“真实在线 TUI 审批视觉交互”两项硬性验收尚未闭环。

**因此 v2.0.5 审核不通过。开发者必须先完成本报告整改要求、补齐可复核证据并重新提交当前版本审核；不得进入下一版本开发。**

署名：审核者

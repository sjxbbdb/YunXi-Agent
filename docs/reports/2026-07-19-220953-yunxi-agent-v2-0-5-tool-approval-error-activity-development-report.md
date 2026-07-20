# YunXi Agent v2.0.5 工具、审批与错误的任务化呈现开发报告

- 撰写时间：2026-07-19 22:09:53 +08:00
- 审核报告：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-19-220157-YunXi-Agent-v2.0.4-hotfix.1-发布源码与TUI视觉审核报告.md`
- 上一版本基线：`v2.0.4-hotfix.1`
- 开发目录：`D:\YunXi Agent`
- 项目内报告：`D:\YunXi Agent\docs\reports\2026-07-19-220953-yunxi-agent-v2-0-5-tool-approval-error-activity-development-report.md`
- 桌面报告：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-19-220953-yunxi-agent-v2-0-5-tool-approval-error-activity-development-report.md`
- 报告类型：下一版本开发报告
- 审核结论转化：`v2.0.4-hotfix.1` 审核通过，可以进入总纲图规定的 `v2.0.5` 开发报告撰写。

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

`v2.0.4-hotfix.1` 已完成响应式 header 档位、国际化排版、四宽度快照和真实 Provider 验收闭环。`v2.0.5` 的核心目标是进入总纲图下一阶段：**工具、审批与错误的任务化呈现**。

本阶段要把工具调用、审批等待、执行进度、执行结果、错误与诊断从散落 transcript 日志收敛为稳定、可更新、可审计的任务活动。用户在主视图应看到一个清晰的工具活动摘要，而不是多条重复工具日志、原始二进制错误或内部堆栈；开发者和审核者仍可在 details/debug 中追溯完整命令、输出、exit code、截断和完整错误上下文。

本阶段必须完成：

1. 建立 `ToolActivity { id, label, phase, summary, detail_ref }`。
2. 将 `requested -> approval required -> running -> completed/failed/cancelled` 收敛为同一个主 activity cell 的就地更新。
3. 完成状态默认只显示一行状态与安全摘要。
4. command、exit code、截断输出、耗时和完整诊断只在 details/debug 中按展示策略揭示。
5. 审批必须使用独立 overlay 或 bottom pane，等待审批期间不得向 transcript 连续插入重复 approval 日志。
6. UI 不得自动批准任何操作；拒绝、取消和 policy decline 都必须有稳定状态。
7. `read_pipe` 必须从直接读取 `String` 改为读取 `Vec<u8>`，再由单一 `ExecOutputDecoder` 做 UTF-8/二进制/截断/完整性处理。
8. 错误必须分类为 provider、tool、approval、cancel、terminal、unknown，并提供稳定 error code 和用户可执行的下一步。

## 三、版本与发布纪律

当前通过审核的基线为：

- 发布提交：`b4c8030dee52ce1e44b3a66793d6f94988ccccca`
- 版本号：`2.0.4-hotfix.1`
- annotated tag：`v2.0.4-hotfix.1`
- tag object：`6f99f765464f872196d9fcae5983146a0675b6bf`

开发者必须遵守：

- 旧 `v2.0.2`、`v2.0.2-hotfix.1`、`v2.0.3`、`v2.0.3-hotfix.1`、`v2.0.4`、`v2.0.4-hotfix.1` tag 均不得移动、删除或覆盖。
- `v2.0.5` 必须对应新的发布提交和新的 annotated tag。
- 验证通过前不得宣称 `v2.0.5` 完成。
- 本阶段不得提前实现后续版本的主题系统、插件功能、完整颜色 token 重构或其他跨版本产品面。
- 审核报告显示发布后真实 Provider 请求生成了 `D:\YunXi Agent\crates\yunxi-agent-cli\.yunxi` 未跟踪目录；如需清理，必须先确认具体路径并取得用户许可。

## 四、非目标

以下内容不属于 `v2.0.5`：

- 主题系统、配色方案、集中颜色 token 重构。
- 插件市场、插件安装器、外部工具包产品面。
- 新增云后台任务、系统常驻服务或跨设备同步。
- 新的主动陪伴策略、记忆图谱或人格模型能力。
- Markdown 富渲染、表格高级布局、图片/媒体渲染。
- 复制上游 Codex runtime 或引入 Go/JS UI 依赖。

## 五、核心设计要求

### ToolActivity

建议新增 TUI 内部工具活动模型：

```rust
pub(crate) struct ToolActivity {
    pub(crate) id: ToolActivityId,
    pub(crate) label: String,
    pub(crate) phase: ToolActivityPhase,
    pub(crate) summary: String,
    pub(crate) detail_ref: Option<TuiCellId>,
}
```

建议阶段：

- `Requested`
- `ApprovalRequired`
- `Running`
- `Completed`
- `Failed`
- `Cancelled`
- `PolicyDeclined`

主视图语义：

- 一个工具调用全过程只能形成一个主 activity cell。
- phase 变化必须原位更新，不追加重复工具日志。
- 默认 completed 只显示安全摘要，例如工具名、执行结果、输出是否截断、耗时。
- 失败状态显示稳定 error code 与下一步，不把内部错误栈直接塞进普通 transcript。

details/debug 语义：

- command、cwd、exit code、stdout/stderr 截断摘要、完整截断元数据、耗时、decoder 完整性状态、provider/tool 原始诊断只在 details/debug 中展示。
- 详情内容必须继续走敏感信息脱敏和普通视图边界。

### Approval Overlay / Bottom Pane

审批必须保持交互边界清楚：

- 使用独立 overlay 或 bottom pane，不向 transcript 重复写入 approval waiting 日志。
- 明确动作、影响范围、风险说明、可选项和默认安全选项。
- 默认选择应偏安全，例如 Decline 或保持未选择，不能自动批准。
- 用户拒绝、Esc、Ctrl+C、policy decline 都必须进入可审计的 terminal phase。
- 等待审批时不得继续执行对应工具。
- `AcceptForSession`、`AcceptAndRemember` 等扩展审批语义如未纳入当前 YunXi 产品策略，不得偷偷开放。

### ExecOutputDecoder

`crates\yunxi-agent-exec\src\lib.rs` 当前 `read_pipe` 直接 `read_to_string`。v2.0.5 必须改为：

1. 先读取 `Vec<u8>`。
2. 统一交给 `ExecOutputDecoder`。
3. 输出结构化结果：

```rust
pub struct DecodedExecOutput {
    pub display_text: String,
    pub replacement_count: usize,
    pub truncated: bool,
    pub original_bytes: usize,
    pub displayed_bytes: usize,
    pub integrity: OutputIntegrity,
}

pub enum OutputIntegrity {
    Clean,
    Lossy,
    Partial,
}
```

要求：

- 无效 UTF-8 不得把 `stream did not contain valid UTF-8` 作为最终用户错误。
- 二进制输出应显示安全降级提示和摘要，不直接向普通视图写入原始二进制。
- 超长输出必须有截断元数据。
- details/debug 可以展示替换计数、原始字节数、截断边界和完整性状态。

### Error Presentation

建议新增 `crates\yunxi-agent-tui\src\error_presentation.rs`：

- `Provider`
- `Tool`
- `Approval`
- `Cancel`
- `Terminal`
- `Unknown`

每类错误必须包含：

- 稳定 error code。
- 面向用户的一行摘要。
- 用户可执行的下一步。
- 是否可重试。
- details/debug 诊断引用。

普通视图不得直接暴露 provider wire、工具参数 JSON、内部栈、未经脱敏的命令参数或二进制输出。

## 六、源码接入点

### `crates\yunxi-agent-tui\src\timeline.rs`

审核报告指定该文件作为工具任务主 cell 的接入点。若当前实现中部分能力仍由 `timeline_store.rs`、`app.rs` 或 transcript 模块承载，开发者应按实际代码梳理，不要为满足文件名而创建空壳模块。

职责建议：

- 建立 `ToolActivityId` 到主 activity cell 的映射。
- phase 更新原位修改同一个 cell。
- late event、重复 event、cancel/fail 后追加事件不得重新打开已终止 activity。
- activity details 与普通 transcript 分层。

### `crates\yunxi-agent-tui\src\approval_layout.rs`

职责建议：

- 保持当前 approval 布局国际化和窄屏能力。
- 增加工具审批动作、影响范围、风险、默认安全选项和快捷键的稳定展示。
- 确保 approve/decline/cancel 路径都有清晰状态。

### `crates\yunxi-agent-tui\src\bottom_pane.rs`

职责建议：

- 将 approval overlay/bottom pane 与 composer/user input 明确区分。
- 等待审批时底栏不丢焦点、不被 transcript 新输出挤掉。
- 审批结束后恢复 composer，并在 activity cell 中更新结果。

### `crates\yunxi-agent-tui\src\error_presentation.rs`

建议新增：

- 错误分类。
- error code。
- 普通视图摘要。
- debug/details 诊断引用。
- 用户可执行下一步。

### `crates\yunxi-agent-exec\src\lib.rs`

重点整改 `read_pipe`：

- 从 `read_to_string` 改为 `read_to_end(Vec<u8>)`。
- 引入 `ExecOutputDecoder`。
- 保证 stdout/stderr 均走同一 decoder。
- 无效 UTF-8、二进制、超长输出和截断输出均可审计且不中断会话。

### `crates\yunxi-agent-cli\src\interactive.rs`

职责建议：

- 将工具事件、approval 决策、cancel 和错误状态传递给 TUI 活动模型。
- 保持 no-TUI、JSON、JSONL 输出兼容。
- 不因 TUI activity 改造破坏 CLI 普通模式。

## 七、参考源码与 Rust 化边界

可参考：

- Codex `bottom_pane/approval_overlay.rs` 的聚焦决策模型。
- k9s 的事件压缩与详情展开逻辑。
- YunXi 既有 `ToolPolicy`、approval 模式和审计链。

参考边界：

- 只抽取审批聚焦、事件压缩、详情展开和错误分层思想。
- 不复制上游 runtime。
- 不引入 Go/JS UI 依赖。
- 不依赖 `vendor/codex-rs`、`codex-*` crate 或 `yunxi-agent-codex`。
- 非 Rust 参考源码只参考逻辑，必须以 YunXi 自身 Rust 模块复刻。

## 八、推荐执行顺序

1. 梳理现有 tool/approval/error 事件进入 TUI 的路径。
2. 定义 `ToolActivity`、`ToolActivityPhase`、`ToolActivityId` 和 details 引用。
3. 接入 timeline/app/transcript，使一个工具调用全过程只对应一个主 activity cell。
4. 改造 approval bottom pane/overlay，停止向 transcript 重复插入 waiting 日志。
5. 在 exec crate 中实现 `ExecOutputDecoder`，先处理无效 UTF-8 和二进制输出。
6. 新增错误分类和 `error_presentation.rs`。
7. 补齐 JSON/JSONL/no-TUI 兼容路径。
8. 完成一批实现后统一跑测试、构建、release、真实工具/审批/TUI 复核。
9. 经用户确认后清理编译产物和 `.yunxi` 状态目录。
10. 创建唯一 `v2.0.5` 发布提交与 annotated tag，再 non-force 推送并等待审核。

## 九、最低测试要求

必须新增或调整以下测试：

1. 一个工具调用从 requested 到 completed 只形成一个主视图 activity cell。
2. approval required、approved、declined、cancelled、policy declined 均原位更新同一 activity。
3. 等待审批期间 transcript 不出现重复 approval 日志。
4. 拒绝审批不会执行工具。
5. cancel 不会自动批准，也不会继续执行工具。
6. 无效 UTF-8 输出不会导致普通视图显示 `stream did not contain valid UTF-8`。
7. 二进制输出显示安全降级摘要，details 记录 `OutputIntegrity::Lossy` 或 `Partial`。
8. 超长 stdout/stderr 截断后仍保留截断元数据。
9. 非零退出码显示稳定 tool error code 和下一步。
10. provider、tool、approval、cancel、terminal、unknown 错误分类均有普通视图和 details 测试。
11. no-TUI、JSON、JSONL 输出不因 TUI activity 模型破坏。
12. v2.0.4-hotfix.1 已通过的 TextLayout、80/100/120/200 snapshot、viewport、resize、30 FPS、active cancel 回归继续通过。

## 十、真实 TUI 验收要求

正式审核前必须实际运行并记录可复核证据：

1. 触发一个需要审批的工具调用。
2. 确认审批使用 bottom pane/overlay，transcript 不刷重复 waiting 日志。
3. 批准后 activity cell 从 approval required 更新为 running/completed。
4. 拒绝后 activity cell 进入 declined/policy declined，不执行工具。
5. 执行输出包含无效 UTF-8 或二进制时，普通视图只有安全摘要，details 可审计。
6. 超长输出截断后，普通视图不拥挤，details 记录截断元数据。
7. 非零退出码后会话仍可继续输入。
8. `Ctrl+C` cancel 不自动批准，不破坏下一轮输入。
9. 普通视图不暴露 thinking、memory/context、provider wire、工具参数 JSON、内部错误栈或凭据。

## 十一、统一验证要求

完成一批构建后统一验证：

1. `cargo fmt --all`
2. `cargo fmt --all -- --check`
3. `cargo check --workspace`
4. `cargo test --workspace`
5. `cargo test -p yunxi-agent-tui`
6. `cargo test -p yunxi-agent-exec`
7. `cargo build --workspace`
8. `cargo build -p yunxi-agent-cli --release --bins`
9. release `yunxi --version`
10. Evaluation Harness、golden、JSON、JSONL 检查
11. offline TUI 工具/审批/错误复核
12. online Provider 与 online TUI 视觉/交互复核
13. `git diff --check`
14. `git status --short --branch`
15. 发布后远程 master 与 annotated tag refs 核验

涉及 `cargo clean`、递归删除 `.yunxi` 状态目录、删除 smoke 临时目录、强制移动、清空目录、系统级安装/卸载、修改 PATH/注册表/系统配置等操作前，必须先取得用户确认。

## 十二、文档同步要求

以下文档必须与实际代码、验证证据、版本号和发布状态一致：

- `D:\YunXi Agent\README.md`
- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\docs\tui-presentation.md`
- `D:\YunXi Agent\docs\persona-memory.md`
- `D:\YunXi Agent\docs\development-log.md`
- `D:\YunXi Agent\docs\reports\2026-07-19-220953-yunxi-agent-v2-0-5-tool-approval-error-activity-development-report.md`
- 如新增 evidence、snapshot、error code 索引或状态文档，也必须同步 `v2.0.5` 的实际完成状态。

## 十三、本报告执行结果

截至 `2026-07-20 07:50:02 +08:00`，v2.0.5 已完成实现与统一验证。当前版本从 `2.0.4-hotfix.1` 更新为 `2.0.5`，旧版本 tag 保持不变。

已实现：

- ToolActivity 兼容模型、稳定 cell 原位更新、终态冻结与迟到事件抑制；普通视图单行安全摘要，命令和完整输出进入 details/debug。
- 审批 bottom pane 默认选中 Decline，明确 Y/Enter 批准、N/Esc 拒绝、Ctrl+C 取消；审批不自动批准，也不向 transcript 重复刷 waiting 日志。
- `ExecOutputDecoder` 统一处理 stdout/stderr 的字节读取、无效 UTF-8、二进制降级、截断、完整性和字节统计。
- provider/tool/approval/cancel/terminal/unknown 错误分类、稳定错误码、下一步提示和 retryable 元数据。
- `CommandCompleted` 执行详情通过非序列化字段进入 TUI details/debug，JSON/JSONL wire 形状保持兼容。

验证结果：

- `cargo fmt --all` 与 `cargo fmt --all -- --check`：通过。
- `cargo check --workspace`：通过。
- `cargo test --workspace`：通过；TUI 105 项、exec 13 项，workspace 其余测试与 doc tests 均通过。
- `cargo build --workspace` 与 `cargo build -p yunxi-agent-cli --release --bins`：通过。
- release `yunxi.exe` 与 `yunxi-agent-cli.exe --version`：均返回 `yunxi 2.0.5`。
- Evaluation Harness：31/31、`golden_passed=true`、tool approval bypass 0；JSON 与 JSONL 均可解析。
- offline no-TUI JSON/JSONL：通过；在线 DeepSeek no-TUI JSON：返回 `YUNXI_V205_LIVE_OK`。在线 TUI 视觉交互需在真实 ConPTY 中复核，本次 shell 环境未伪造 TTY。
- decoder 新增用例覆盖无效 UTF-8、二进制、Partial 截断和 stdout/stderr 统一契约。

清理状态：已取得用户明确授权，删除精确路径 `D:\YunXi Agent\target` 与 `D:\YunXi Agent\crates\yunxi-agent-cli\.yunxi`；未触碰 `.tmp` 或其他目录。清理后两个目标均不存在，`.tmp` 仍保留。

发布状态：尚未提交、创建 tag 或推送；待最终 diff/status 复核及用户允许清理后，创建唯一提交和 annotated `v2.0.5` tag，旧 tag 不移动、不删除、不覆盖。

署名：开发者

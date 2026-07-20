# YunXi Agent v2.0.5 错误呈现与真实在线 TUI 证据整改开发报告

- 撰写时间：2026-07-20 08:59:31 +08:00
- 审核报告：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-20-084839-YunXi-Agent-v2.0.5-工具审批错误任务化呈现审核报告.md`
- 原开发报告：`D:\YunXi Agent\docs\reports\2026-07-19-220953-yunxi-agent-v2-0-5-tool-approval-error-activity-development-report.md`
- 开发目录：`D:\YunXi Agent`
- 项目内报告：`D:\YunXi Agent\docs\reports\2026-07-20-085931-yunxi-agent-v2-0-5-error-presentation-conpty-remediation-development-report.md`
- 桌面报告：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-20-085931-yunxi-agent-v2-0-5-error-presentation-conpty-remediation-development-report.md`
- 报告类型：当前版本整改开发报告
- 审核结论转化：`v2.0.5` 审核不通过，不得进入下一版本开发。

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

## 二、整改目标

`v2.0.5` 已完成工具活动、审批界面、输出解码和离线回归的主体能力，但审核报告明确指出两项 P1 阻塞项。当前阶段不是下一版本开发，而是补齐 `v2.0.5` 当前版本整改，并为重新审核准备证据。

本阶段必须完成：

1. 把 provider/tool/approval/cancel/terminal/unknown 六类错误接到统一的错误呈现入口。
2. 在普通视图中为这六类错误显示稳定 error code、简明摘要和用户可执行的下一步。
3. 将原始命令、Provider wire、内部栈、详细诊断和其他敏感信息保留在 details/debug。
4. 补齐真实 Windows ConPTY 在线 TUI 视觉与审批交互证据。
5. 在真实在线会话中复核批准、拒绝、取消、无效 UTF-8、二进制、超长输出和非零退出后的继续输入。

## 三、版本与发布边界

当前审核对象为：

- 发布提交：`fe15693a039d025a2cdb21b6d7c192207161682b`
- 版本号：`2.0.5`
- annotated tag：`v2.0.5`
- tag object：`caad35a0ecc8eeb066721ab6f9d4e35cd2592602`

开发者必须遵守：

- 已发布 `v2.0.5` tag 不得移动、删除或覆盖。
- 旧 `v2.0.2`、`v2.0.2-hotfix.1`、`v2.0.3`、`v2.0.3-hotfix.1`、`v2.0.4` 与 `v2.0.4-hotfix.1` tag 均不得移动、删除或覆盖。
- 当前整改发布版本号/tag 方案须由用户确认；开发者不得自行指定 hotfix 命名。
- 不得把本阶段整改伪装成下一版本正常功能开发。
- 重新审核通过前，不得宣称 `v2.0.5` 完成，也不得进入下一版本开发。

## 四、已通过能力保留要求

以下已通过能力必须继续保持，不得在整改中回退：

1. `ToolActivity` 主 cell 原位更新与终态冻结。
2. 审批 bottom pane/overlay 默认安全决策和拒绝/取消路径。
3. `ExecOutputDecoder` 对 stdout/stderr、无效 UTF-8、二进制输出、截断和完整性元数据的统一处理。
4. `no-TUI`、`JSON`、`JSONL`、release binary、伴侣评测和发布 tag 纪律。
5. `v2.0.4-hotfix.1` 已通过的国际化排版、四宽度 snapshot、viewport、resize、30 FPS 与窄屏 header 档位。
6. 主视图不泄露 thinking、memory/context、工具参数或原始二进制。

## 五、必须整改的问题

### 1. 稳定错误呈现没有接入统一入口

审核发现 `crates\yunxi-agent-tui\src\error_presentation.rs` 已定义六类 `ErrorCategory` 和 `YX-*-001` 代码，但 `crates\yunxi-agent-tui\src\presentation.rs` 仍未把所有用户可见错误统一路由到 `ErrorPresentation::for_category(...)`。

具体缺口：

- `presentation.rs:316-342` 中 `AgentEvent::ApprovalCompleted { approved: false }` 仅把活动标记为 `Declined` 或 `PolicyDeclined`，没有在普通视图中输出 `YX-APPROVAL-001`、简明摘要和下一步。
- `presentation.rs:481-488` 中 `Cancelled` 只显示 `current turn cancelled`，没有 `YX-CANCEL-001` 和后续动作提示。
- `presentation.rs:599-607` 的 `present_error` 仍固定输出 `operation failed`，没有稳定 error code、可重试标记和分层诊断。

整改要求：

1. 建立唯一的错误呈现入口，将 `ApprovalCompleted(false)`、`Cancelled`、terminal 失败和通用 `present_error` 路由到 `ErrorPresentation`。
2. 普通 transcript 或对应 `ToolActivity.summary` 必须显示稳定 code、简明摘要、是否可重试和下一步。
3. 命令、Provider wire、原始错误和未脱敏参数只能进入 details/debug。
4. 不同错误类别必须保留稳定的用户可执行下一步，而不是把所有错误都压成“operation failed”。

### 2. 真实在线 TUI 的可复核视觉与审批证据不足

审核报告明确指出，离线测试、JSON/JSONL、release binary 和真实 Provider no-TUI 请求都通过，但当前审核会话没有可捕获的 Windows ConPTY 主机，因此缺少可复核的真实在线 TUI 视觉/交互证据。

整改要求：

1. 在真实 Windows ConPTY 或等价终端中，录制在线 TUI 证据，而不是只依赖单元测试和离线快照。
2. 证据必须覆盖 80、100、120、200 列帧、审批触发、批准、拒绝、`Ctrl+C` 取消、无效 UTF-8、二进制、超长输出和非零退出后的继续输入。
3. 证据中必须能复核审批 bottom pane 的焦点、拒绝/取消路径、单一 activity cell 原位更新和错误摘要。
4. 真实在线证据应脱敏保存到项目目录，例如 `D:\YunXi Agent\docs\reports\evidence\2026-07-20-v2-0-5-error-presentation-conpty-evidence.md`。

## 六、源码接入点

### `crates\yunxi-agent-tui\src\presentation.rs`

当前是错误呈现的主入口，但映射仍不完整。需要把以下路径统一接到 `ErrorPresentation`：

- `AgentEvent::ApprovalCompleted { approved: false }`
- `AgentEvent::Cancelled`
- `AgentEvent::Error`
- `AgentEvent::ProviderError`
- `present_error`

### `crates\yunxi-agent-tui\src\error_presentation.rs`

应作为六类错误的唯一规范化边界，输出：

- `code`
- `display_summary`
- `next_step`
- `retriable`
- `detail_ref`

### `crates\yunxi-agent-tui\src\app.rs`

`app.rs` 负责把事件路由到 presentation 和 activity cell。整改时应确保：

- 普通视图只消费错误摘要。
- details/debug 才承载完整诊断。
- `push_error`、`push_warning`、`push_agent_event` 不再绕过统一错误入口。

### `crates\yunxi-agent-tui\src\timeline.rs`

工具活动主 cell 已经存在，整改时应让错误状态更新到同一个 activity，而不是新建重复日志。对工具调用、审批、取消和终止状态，主视图应保持单一 cell 原位更新。

### `crates\yunxi-agent-tui\src\bottom_pane.rs`

审批界面已经具备默认安全决策，整改重点是保持审批过程与错误呈现边界清晰，不把审批等待和失败诊断混在 transcript 主流里。

### `crates\yunxi-agent-exec\src\lib.rs`

`read_pipe` 和 `ExecOutputDecoder` 已经是字节级解码入口，整改时应继续保持这条边界，避免在 TUI 或 CLI 再次直接把原始二进制或无效 UTF-8 作为最终用户错误。

### `crates\yunxi-agent-cli\src\interactive.rs`

CLI 交互路径应继续保留 `no-TUI`、JSON、JSONL 兼容，不因错误呈现整改破坏已有发布行为。

## 七、参考源码建议

可继续参考：

- Codex `bottom_pane/approval_overlay.rs` 的审批聚焦与等待态逻辑。
- k9s 的事件压缩与详情展开逻辑，用于“主 cell + details”的结构化呈现。
- YunXi 现有 `ExecOutputDecoder`，继续作为字节级解码唯一入口。

参考边界：

- 只抽取交互聚焦、事件压缩、错误分层和 details 展开思想。
- 不复制上游 runtime。
- 不引入 Go/JS UI 依赖。
- 不依赖 `vendor/codex-rs`、`codex-*` crate 或 `yunxi-agent-codex`。
- 非 Rust 参考源码只参考逻辑，必须以 YunXi 自身 Rust 模块复刻。

## 八、推荐执行顺序

1. 统一 error presentation 路由，先把 `ApprovalCompleted(false)`、`Cancelled`、`present_error` 和 `ProviderError` 接入 `ErrorPresentation`。
2. 为 provider/tool/approval/cancel/terminal/unknown 六类错误新增集成测试。
3. 补齐 `ToolActivity.summary`、details/debug 与错误 code 的映射。
4. 设计并执行真实 Windows ConPTY 在线 TUI 证据采集流程。
5. 在真实会话中复核审批、拒绝、取消、无效 UTF-8、二进制、长输出与非零退出继续输入。
6. 统一跑 workspace 验证、release 验证、评测和 TUI 回归。
7. 经用户确认后再清理编译产物和 `.yunxi` 状态目录。
8. 创建新的发布提交和用户确认的 annotated tag，再 non-force 推送并重新审核。

## 九、最低测试要求

必须新增或调整以下测试：

1. provider 错误映射到 `ErrorPresentation` 的稳定 code 和下一步。
2. tool 错误映射到 `ErrorPresentation` 的稳定 code 和下一步。
3. approval `Declined` / `PolicyDeclined` / `Cancelled` 分别映射到独立 code。
4. terminal 失败使用稳定 code，不再输出固定的泛化错误。
5. unknown 错误仍有安全降级摘要和 details 引用。
6. 普通视图不泄露原始命令、Provider wire、内部栈或未脱敏参数。
7. details/debug 保留完整诊断和完整性元数据。
8. 审批拒绝与取消不会执行工具，不会自动批准。
9. `no-TUI`、JSON、JSONL 与 release binary 行为不回退。

## 十、真实 TUI 复核要求

正式复审前必须提供可复核的真实在线终端证据：

1. 记录终端尺寸、Provider、模型、时间戳和命令。
2. 在真实 ConPTY 中触发一个需要审批的无害工具调用。
3. 复核批准、拒绝和 `Ctrl+C` 取消，不得自动批准。
4. 复核无效 UTF-8、二进制输出、超长输出和非零退出后的继续输入。
5. 复核普通视图只显示错误 code、简明摘要和下一步。
6. 复核 details/debug 可追溯完整诊断但不泄漏到普通视图。
7. 证据脱敏保存，不在报告中出现凭据、原始 wire 或敏感命令参数。

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
9. `target\\release\\yunxi.exe --version`
10. `yunxi eval companion --json`
11. offline TUI 与真实 ConPTY 在线 TUI 复核
12. `git diff --check`
13. `git status --short --branch`
14. 发布后远程 master 与 annotated tag refs 核验

涉及 `cargo clean`、递归删除 `.yunxi` 状态目录、删除 smoke 临时目录、强制移动、清空目录、系统级安装/卸载、修改 PATH/注册表/系统配置等操作前，必须先取得用户确认。

## 十二、文档同步要求

以下文档必须与实际代码、验证证据、版本号和发布状态一致：

- `D:\YunXi Agent\README.md`
- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\docs\tui-presentation.md`
- `D:\YunXi Agent\docs\persona-memory.md`
- `D:\YunXi Agent\docs\development-log.md`
- `D:\YunXi Agent\docs\reports\2026-07-20-085931-yunxi-agent-v2-0-5-error-presentation-conpty-remediation-development-report.md`
- `D:\YunXi Agent\docs\reports\evidence\2026-07-20-v2-0-5-error-presentation-conpty-evidence.md`

## 十三、整改执行结果

截至 `2026-07-20 12:07:40 +08:00`，本报告要求的源码整改、统一验证和真实 Windows ConPTY 在线证据已完成。当前状态是 `v2.0.5` 当前版本审核整改候选，不是下一版本开发；原 annotated `v2.0.5` tag 保持在 `fe15693a039d025a2cdb21b6d7c192207161682b`，没有移动、删除或覆盖。用户已确认新的整改 annotated tag 名称为 `v2.0.5-hotfix.1`。尚未提交、创建 tag 或推送，也尚未宣称复审通过。

### 已完成源码整改

1. `crates\yunxi-agent-tui\src\error_presentation.rs`：六类 `ErrorCategory` 继续作为唯一规范化边界，`ErrorPresentation` 增加 `detail_ref`，普通摘要统一包含稳定 `YX-*-001` code、`retryable=yes|no` 和类别专属 `next:`。
2. `crates\yunxi-agent-tui\src\presentation.rs`：`ProviderError`、tool/command/MCP failure、`ApprovalCompleted(false)`、policy decline、cancel、terminal、unknown、通用 `present_error` 和非成功 turn 统一接入 `ErrorPresentation`；命令、Provider wire、内部栈和完整诊断只进入 details/debug。执行完整性元数据调整到长输出正文之前，避免 details 行数上限隐藏字节、替换和截断状态。
3. `crates\yunxi-agent-tui\src\timeline.rs`：继续冻结全部终态，只允许同一 id 的后续结构化取消事件将先到的通用 `Declined` 精确纠正为 `Cancelled`；不改名、不新建 activity、不重新打开工具。
4. `crates\yunxi-agent-tui\src\chat.rs`：增加六类普通错误、诊断隔离、拒绝单 activity、真实 `CommandCompleted(Declined) -> ApprovalCompleted(cancelled)` 事件顺序和重复 warning 隐藏回归。
5. `crates\yunxi-agent-tui\src\host.rs`：删除 host 在审批决策后额外插入的 approved/declined notice，审批结果只由 runtime event 原位更新 activity。

### 真实 ConPTY 在线证据

正式脱敏证据：`D:\YunXi Agent\docs\reports\evidence\2026-07-20-v2-0-5-error-presentation-conpty-evidence.md`。

- Windows ConPTY：真实 `node-pty useConpty=true` 子进程，不是单元测试或伪造 TTY。
- Provider/model：DeepSeek live / `deepseek-chat`。
- 尺寸：80x24、100x30、120x40、200x50 全部完成 resize 和 frame 复核，无区域重叠。
- 审批：默认焦点 Decline；`N` 拒绝不执行；`Y` 批准执行；无重复 approved/declined notice。
- Ctrl+C：同一 shell activity 路径为 `approval required -> running -> declined -> cancelled`，最终 `YX-CANCEL-001`，未执行工具，后续输入成功。
- 非零退出：`cmd /c exit 7` 显示 `YX-TOOL-001`、retryable 和下一步，后续输入成功。
- 无效 UTF-8：普通视图仅 count 摘要；details 为 `original_bytes=1 / replacement_count=1 / integrity=Lossy`。
- 二进制：普通视图不写原始字节；details 为安全 placeholder、`original_bytes=3 / replacement_count=1 / integrity=Lossy`。
- 2000 行输出：普通视图为 `1001 line(s), 12018 char(s)`；details 为 `original_bytes=26000 / displayed_bytes=12019 / truncated=true / integrity=Partial`，尾部标明隐藏 1966 行，后续输入成功。

### 11:34 复审 P1 证据整改

`docs\reports\2026-07-20-113410-yunxi-agent-v2-0-5-error-presentation-conpty-reaudit-report.md` 确认源码、测试、评估和真实 Provider 已通过，但阻断发布：当时只有人工整理的 Markdown，审核者无法独立重演 ConPTY，且整改尚未提交/tag/推送。

证据可复跑缺口现已整改：

- `scripts\conpty\v205\capture.js`：八场景总入口，任一场景失败即 non-zero。
- `scripts\conpty\v205\capture-scenario.js`：真实 `node-pty useConpty=true` 场景驱动，记录 resize、提示、Y/N/Ctrl+C、PageUp/End、`/details` 和 `/exit`。
- `scripts\conpty\v205\package.json` 与 `package-lock.json`：精确锁定 `node-pty 1.1.0`、`@xterm/headless 5.5.0` 和传递依赖完整性哈希。
- `scripts\conpty\v205\verify.js`：离线重算 SHA-256，核验 checkpoint、按键、稳定错误码、decoder metadata、后续输入和 secret-like token。
- `docs\reports\evidence\frames\v205-conpty\*.json`：八个独立 DeepSeek/ConPTY 会话的原始脱敏帧与交互序列。
- `docs\reports\evidence\frames\v205-conpty\manifest.json`：八个文件的 SHA-256、字节数、checkpoint 数和起止时间。

完整采集从 `2026-07-20 12:01:29 +08:00` 至 `12:02:15 +08:00` 通过；八场景 checkpoint 数为 7/6/6/6/6/8/8/9，离线 verifier 返回 `ok=true, scenarios=8`。采集器和帧均未保存 API key、Provider wire 或敏感命令参数。

### 统一验证结果

- `cargo fmt --all`：通过。
- `cargo fmt --all -- --check`：通过。
- `cargo check --workspace`：通过。
- `cargo test --workspace`：通过，全部 crate、集成测试和 doc tests 无失败。
- `cargo test -p yunxi-agent-tui`：111/111 通过。
- `cargo test -p yunxi-agent-exec`：13/13 单元测试、8/8 sandbox acceptance 通过。
- `cargo build --workspace`：通过。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- `target\release\yunxi.exe --version` 与 `yunxi-agent-cli.exe --version`：均返回 `yunxi 2.0.5`。
- `yunxi eval companion --json`：31/31，`golden_passed=true`，persona consistency 1.0，memory precision 1.0，tool approval bypass 0。
- offline JSON：可解析，status 为 completed。
- offline JSONL：22 行全部可解析。
- offline no-TUI：正常完成并返回 REPL，`/exit` 正常退出。
- DeepSeek online no-TUI JSON：返回 `YUNXI_V205_REMEDIATION_LIVE_OK`。
- DeepSeek online Windows ConPTY：上述正式证据全部通过。

### 清理、提交与发布状态

用户已于 `2026-07-20 11:19:59 +08:00` 明确授权清理。首次 `cargo clean` 已删除大部分约 3.66 GB 产物，但遗留 ConPTY 采集进程 PID 12932 锁定 `conpty.node`；读取进程命令行确认该 PID 精确对应 `target\v205-conpty\capture-scenario.js cancel` 后，只终止该采集进程，未终止 CodeGraph、Codex 或其他 Node 服务。再次执行 `cargo clean` 成功，并删除精确路径 `D:\YunXi Agent\.tmp\v205-conpty-workspace` 与 `D:\YunXi Agent\crates\yunxi-agent-cli\.yunxi`。正式 Markdown 证据保留在 `docs\reports\evidence`。

`2026-07-20 12:19:19 +08:00` 发布前门禁再次确认：`cargo test -p yunxi-agent-exec` 为 13/13 与 sandbox 8/8，workspace/debug/release build 通过，两个 release binary 均为 `yunxi 2.0.5`，Evaluation Harness 为 31 通过、0 失败、`golden_passed=true`、`tool_approval_bypass_count=0`，offline JSON/JSONL/no-TUI 与八场景离线 ConPTY verifier 全部通过，`git diff --check` 无空白错误。用户确认后删除 `D:\YunXi Agent\target`、`D:\YunXi Agent\scripts\conpty\v205\node_modules`、`D:\YunXi Agent\scripts\conpty\v205\.work`、`D:\YunXi Agent\.yunxi` 和 `D:\YunXi Agent\crates\yunxi-agent-cli\.yunxi`，五项目标均核验为不存在；collector、`package-lock.json`、脱敏帧和 manifest 保留。

尚未提交、尚未创建整改 tag、尚未推送。用户已确认 `v2.0.5-hotfix.1`；下一步创建整改发布提交和新的 annotated tag，使用 non-force 方式推送并核验远程 master、整改 tag 和历史 refs。所有旧 tag 保持不变，API key 不打印、不写入仓库、Git 配置或 remote URL。

署名：开发者

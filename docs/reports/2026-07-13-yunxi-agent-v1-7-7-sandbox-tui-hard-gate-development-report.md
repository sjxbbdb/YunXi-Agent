# YunXi Agent v1.7.7 Sandbox And TUI Hard Gate Development Report

生成时间：2026-07-13 13:05:46 +08:00

## 背景

YunXi Agent v1.7.6 已完成 CLI/TUI/sandbox diagnostic 收敛：默认 CLI 不再暴露 detached Codex backend，测试污染已修复，TUI transcript wrapping 与窄屏 header 有明显改善，sandbox event 也开始携带 `runner` 与 `unsupported_reason`。当前远端 `master`、本地 `master`、本地 `origin/master` 与 annotated tag `v1.7.6` 已同步。

用户提供了新的审核报告：

- `C:\Users\admin\Desktop\YunXi-Agent-CLI-TUI-审核报告-1.7.6-2026-07-13.md`

本报告面向 v1.7.7。v1.7.7 不继续扩大模块面，而是作为 hard-gate release：先关闭审核报告中的 P0/P1，再处理 P2 体验问题。P0/P1 未关闭前，不加入新工具、新模块、新 provider surface 或更复杂 agent 能力。

## 审核报告结论复核

本轮已读取审核报告全文，并按仓库规则优先使用 CodeGraph 核对关键代码落点。报告结论与当前代码状态一致：

- P0：`crates/yunxi-agent-sandbox/src/lib.rs` 中 `SandboxBackend::os_isolation()` 仍固定返回 `false`，`WorkspaceGuard | WindowsRestrictedToken | LinuxLandlock` 的用户文案仍为 `policy guard: advisory only, no OS isolation`。
- P1：`crates/yunxi-agent-tui/src/bottom_pane.rs` 的 `desired_height_for_width(&self, _width)` 忽略 width，approval 面板只把 command 算作 1 行；`crates/yunxi-agent-tui/src/render.rs` 的 approval `Paragraph` 会 wrap reason/cwd/command，58 列时可能裁掉 `Decline` 和 `Tab changes selection`。
- P2：`crates/yunxi-agent-tui/src/app.rs` 的 `header_for_width()` 用 `fit_line()` 逐项追加，100 列中等宽度时可能整项丢弃 `model=...`。
- P2：`crates/yunxi-agent-cli/src/main.rs` 中 `jsonl` 是 `global = true` 参数，metadata 子命令 help 中仍会展示 `--jsonl`，虽然执行时会正确拒绝。

因此 v1.7.7 的成功标准不是“新增更多能力”，而是让安全边界、approval 操作界面和 CLI contract 不再误导用户。

## v1.7.7 总目标

YunXi Agent v1.7.7 的目标是关闭 v1.7.6 审核报告指出的 hard-gate 缺口：

- P0：sandbox 边界必须诚实、可测试、不可被文案误解。v1.7.7 默认路径如果仍是 policy-only，就必须在 CLI/TUI/README/JSONL 中稳定声明 policy-only；如果某平台实现真实 OS 隔离，则只有通过 acceptance suite 的 backend 才能返回 `os_isolation=true`。
- P1：TUI approval 面板在 58 列窄屏必须同时可见 `Approve`、`Decline` 和快捷键说明。长 cwd/reason/command 不能把拒绝路径挤出面板。
- P2：TUI header/subheader 在中等宽度必须稳定可见 provider、mode、model 中的关键信息。
- P2：metadata 子命令 help 不能暗示 `--jsonl` 可用于 metadata 输出；要么隐藏，要么明确说明仅 agent execution 支持。

## 用户硬性约束

实现 v1.7.7 时继续遵守以下约束：

- 先完整构建 v1.7.7 源码，中间不做反复单点测试，不在某个点卡太久。
- 源码构建完成后统一执行验证。
- 每个正式版本必须创建新的不可变 annotated tag，旧 tag 不删除、不移动。
- GitHub 读写和推送只走 REST API，不使用 `git push`、`git fetch`、`git ls-remote`。
- API 密钥只从 `C:\Users\admin\Desktop\api.txt` 读取，不打印、不写日志、不提交。
- 每次任务结束必须追加 `C:\Users\admin\Desktop\YunXi Agent开发日志.md`。
- 发布收尾前执行 `cargo clean` 清理构建中间产物。
- P0/P1 未关闭前，不开启新模块开发。

## 非目标

- v1.7.7 不新增桌面端、云任务、插件市场、自动更新器、SDK 包装或新产品面。
- v1.7.7 不恢复默认 CLI 对上游 Codex runtime、`vendor/codex-rs`、`codex-*` 或 `yunxi-agent-codex` 的依赖。
- v1.7.7 不用“runner 已存在”代替 OS 级隔离。没有平台强制隔离就继续报告 `os_isolation=false`。
- v1.7.7 不把 P0/P1 之外的视觉细节放在 hard gate 之前。
- v1.7.7 不要求一次性完成所有平台强隔离；但必须让当前默认路径的安全边界不可误解，并建立 acceptance suite。

## 开发主线一：P0 Sandbox Hard Gate

### 1. 安全边界命名定稿

修改：

- `D:\YunXi Agent\crates\yunxi-agent-sandbox\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-core\src\event.rs`
- `D:\YunXi Agent\crates\yunxi-agent-protocol\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tools\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\render.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\event_filter.rs`

职责：

- 新增或明确 `SandboxEnforcementLevel` 语义，至少区分：
  - `policy_only`
  - `process_lifecycle`
  - `os_restricted`
  - `policy_bypass`
- JSONL/plain/TUI/debug detail 中必须稳定输出 enforcement level。
- `os_isolation=true` 只能由已通过 acceptance suite 的 `os_restricted` backend 返回。
- Windows 当前若仍只有 policy guard + child lifecycle，必须输出：
  - `os_isolation=false`
  - `enforcement=policy_only` 或 `process_lifecycle`
  - `unsupported_reason` 明确说明没有 restricted-token filesystem isolation。

完成条件：

- 默认 Windows v1.7.7 不再出现让用户误以为已启用强隔离的文案。
- `SandboxBackend::os_isolation()` 不再是无上下文的永久 `false` facade；如果仍返回 false，原因和 enforcement level 必须能被机器读取。
- JSONL snapshot 包含 `enforcement_level` 或等价稳定字段。

### 2. Sandbox acceptance suite

修改：

- `D:\YunXi Agent\crates\yunxi-agent-sandbox\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tools\tests\tool_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\jsonl_tests.rs`
- 新增：`D:\YunXi Agent\crates\yunxi-agent-exec\tests\sandbox_acceptance_tests.rs`

职责：

- 增加 acceptance 测试矩阵：
  - read-only 下尝试写 workspace 内文件，必须被拒绝或进入 escalation。
  - workspace-write 下尝试写 workspace 外绝对路径，必须被拒绝或进入 escalation。
  - workspace-write 下尝试通过 symlink 写 workspace 外，必须被拒绝。
  - network disabled 下执行 `curl` / `Invoke-WebRequest` / `irm` 风险命令，必须被拒绝或进入 network escalation。
  - danger-full-access 路径必须允许执行，但 event 必须标记 `policy_bypass`。
  - policy-only runner 对无法 OS 拦截的场景必须诚实输出 `os_isolation=false`，测试不能误判为强隔离。
- 所有测试使用临时目录，不写入项目根目录。
- 对 Windows 与非 Windows 命令分别封装 helper，避免平台字符串散落在测试里。

完成条件：

- `cargo test -p yunxi-agent-sandbox -p yunxi-agent-tools -p yunxi-agent-exec -p yunxi-agent-cli` 覆盖 acceptance suite。
- JSONL 中 sandbox acceptance fixture 能证明 policy-only / bypass / escalation 三类状态。
- 测试结束后 `git status --short` 不出现测试产物。

### 3. 平台 runner 现实边界

修改：

- `D:\YunXi Agent\crates\yunxi-agent-exec\src\lib.rs`
- 新增：`D:\YunXi Agent\crates\yunxi-agent-exec\src\windows_runner.rs`
- 新增：`D:\YunXi Agent\crates\yunxi-agent-exec\src\linux_runner.rs`
- `D:\YunXi Agent\README.md`
- `D:\YunXi Agent\docs\extraction-status.md`

职责：

- Windows：
  - 确认当前 `DirectProcessRunner` 是否只提供 process lifecycle。
  - 如果 v1.7.7 实现 job object containment，只能标记 `process_lifecycle`，不能标记 OS filesystem isolation。
  - restricted token / low integrity / deny write outside workspace 若实现，必须 feature-gated，并且只有 acceptance suite 通过后才设置 `os_isolation=true`。
- Linux：
  - Landlock 未在当前 Windows 环境验证时继续标记 `unsupported` 或 `feature_disabled`。
  - 不在 README 中宣称 Linux 强隔离已完成。
- 文档：
  - README 的 Honesty And Safety Boundaries 必须把 policy-only 默认状态写清楚。
  - TUI sandbox detail 与 plain CLI sandbox line 都必须写出 policy-only 状态。

完成条件：

- 默认 v1.7.7 在没有强隔离时通过“诚实 policy-only + acceptance suite”关闭 P0 用户误解风险。
- 如果某平台强隔离未完成，后续报告中继续列为 v1.8.x/v1.9 安全工程，不在 v1.7.7 虚报。

## 开发主线二：P1 TUI Approval 窄屏操作可见性

### 1. Approval 高度按实际 wrap 计算

修改：

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\bottom_pane.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs`
- 新增：`D:\YunXi Agent\crates\yunxi-agent-tui\src\approval_layout.rs`

职责：

- 将 `desired_height_for_width(width)` 中 approval 分支从固定 `9 + command_lines` 改为按实际内容计算：
  - title/border 固定成本。
  - approval line、reason line、risk line、command line 按 width 计算 wrap 行数。
  - action lines 必须预留：空行、Approve、Decline、Tab changes selection。
- 抽出共享 approval layout 计算，render 和 desired_height 使用同一套行数规则，避免再次出现“预估高度”和实际 wrap 不一致。
- 对超长 cwd/reason/command 采用 bounded display：
  - reason 和 cwd 可按宽度截断或两行内 wrap。
  - command 可显示前 N 行并保留 details/debug id，不能挤掉操作区。

完成条件：

- 58x22 approval snapshot 同时包含 `Approve`、`Decline`、`Tab changes selection`。
- 58x18 空间更紧时也不能只显示 Approve；如果空间不足，优先压缩 command/reason，而不是裁掉 Decline。
- approval 高风险命令下默认选择策略不因布局改动发生未记录变化。

### 2. Approval 视觉回归测试

修改：

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\bottom_pane.rs`

职责：

- 新增 TestBackend 覆盖：
  - `58x22 approval-narrow`
  - `80x22 approval-medium`
  - `100x24 approval-wide`
  - 长 cwd + 长 reason + 长 command
  - destructive/network/write/read 风险标签
- 断言所有尺寸中 `Approve`、`Decline`、快捷键说明均可见。
- 断言 destructive command 的 `risk: destructive` 可见，且未覆盖操作区。

完成条件：

- `cargo test -p yunxi-agent-tui` 能稳定复现并防止 P1 回归。
- 测试输出不依赖机器绝对临时路径。

## 开发主线三：P2 TUI Header/Model 可见性

### 1. Header/subheader 信息优先级

修改：

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs`

职责：

- 定义稳定信息优先级：
  - 宽屏：version、cwd、provider、mode、model、source、cells、view、debug。
  - 中屏：version、provider、mode、truncated model、view、cells。
  - 窄屏：version、mode、short model、view。
- provider/mode/model 至少在 header 或 subheader 二者之一可见。
- 长 model 不整项丢弃，改为 `model=<truncated>`，例如 `model=deepseek-chat...`。
- CWD 在中屏优先压缩，不优先挤掉 model。

完成条件：

- 100x30 snapshot 中能看到 model。
- 80 列和 58 列 snapshot 不出现 dangling separator 或半截 token。
- `app.rs` 单测和 `render.rs` TestBackend 测试覆盖 58/80/100 列。

## 开发主线四：P2 Metadata Help Contract

### 1. `--jsonl` help 语义修正

修改：

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`
- `D:\YunXi Agent\README.md`

职责：

- 保留执行路径支持：
  - `yunxi --jsonl "<prompt>"`
  - `yunxi --jsonl run <prompt>`
  - `yunxi resume <id> --jsonl <prompt>` 如果当前解析路径支持。
- metadata 子命令 help 不能暗示 `--jsonl` 是 metadata 输出选项。可选实现：
  - 将 `--jsonl` help 文案改为“agent execution only; metadata commands reject it”，并在 `sessions --help`、`parity --help` 测试中断言该限制文案可见。
  - 或重构 clap 参数，把 JSONL 从 global flag 拆到 agent execution 入口；metadata 子命令仅展示 `--json`。
- 当前版本继续让 metadata `--jsonl` parse 后明确拒绝也可以，但 help 必须不误导。

完成条件：

- `yunxi sessions --help` 或 `yunxi sessions list --help` 不把 `--jsonl` 表现为 metadata 输出能力。
- `sessions list --jsonl` 与 `parity map --jsonl` 继续拒绝，并保持稳定错误文案。
- `--json` 与 `--jsonl` 互斥测试继续通过。

## 开发主线五：版本、文档、发布

修改：

- `D:\YunXi Agent\Cargo.toml`
- `D:\YunXi Agent\Cargo.lock`
- `D:\YunXi Agent\README.md`
- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\docs\reports\2026-07-13-yunxi-agent-v1-7-7-sandbox-tui-hard-gate-development-report.md`
- `C:\Users\admin\Desktop\YunXi Agent开发日志.md`

职责：

- Cargo package/workspace version 推进到 `1.7.7`。
- CLI/TUI banner 统一到 `v1.7.7`。
- README 和 extraction status 明确 v1.7.7 是 sandbox/TUI hard-gate release。
- 如果默认 sandbox 仍 policy-only，文档必须写出：
  - 当前默认不提供 OS-enforced filesystem isolation。
  - policy guard 会在执行前拒绝已知风险命令和已知 workspace escape，但不能替代 OS sandbox。
  - danger-full-access 是 policy bypass。
- 统一验证完成后发布远端 `master` 并创建不可变 annotated tag `v1.7.7`。

完成条件：

- 本地 `master`、本地 `origin/master`、GitHub API branch、`v1.7.7` tag target 一致。
- 发布过程只走 GitHub REST API。
- 发布后执行 `cargo clean`，`D:\YunXi Agent\target` 不存在。

## 分阶段实施建议

实现阶段继续遵守用户硬性约束：先按报告完整构建，不在构建中途做反复验证；构建完成后统一测试。

### Phase 1：P0 safety language and event contract

目标：

- 建立 `policy_only` / `process_lifecycle` / `os_restricted` / `policy_bypass` 的统一机器语义。
- 贯通 sandbox diagnostic 到 plain/JSONL/TUI。
- 不虚报 OS isolation。

### Phase 2：Sandbox acceptance suite

目标：

- 把 workspace escape、symlink escape、network disabled、danger-full-access bypass 固化为测试。
- policy-only 默认路径以诚实 diagnostic 关闭用户误解风险。

### Phase 3：P1 approval layout

目标：

- 用共享 layout 计算修复 approval 高度预估。
- 58/80/100 列 TestBackend 均保证 Approve/Decline/快捷键可见。

### Phase 4：P2 TUI/CLI contract polish

目标：

- header/subheader 中等宽度稳定显示 model。
- metadata help 不再误导 `--jsonl` 能用于 metadata 输出。

### Phase 5：版本、统一验证、发布

目标：

- 版本推进到 `1.7.7`。
- 文档、开发报告、桌面日志同步。
- 统一验证后安装、REST API 发布、创建 tag、清理构建产物。

## 统一验证计划

源码构建完成后统一运行：

- `cargo fmt`
- `cargo fmt -- --check`
- `cargo test -p yunxi-agent-sandbox -p yunxi-agent-tools -p yunxi-agent-exec -p yunxi-agent-tui -p yunxi-agent-cli`
- `cargo test`
- `cargo check --workspace`
- `cargo build -p yunxi-agent-cli --release --bins`
- `target\release\yunxi.exe --version`，期望 `yunxi 1.7.7`
- `target\release\yunxi-agent-cli.exe --version`，期望 `yunxi 1.7.7`
- `target\release\yunxi.exe --offline "v1.7.7 offline smoke"`
- `target\release\yunxi.exe --offline --json "v1.7.7 json smoke"`
- `target\release\yunxi.exe --offline --jsonl "v1.7.7 jsonl smoke"`
- `target\release\yunxi.exe --backend codex "hello codex"`，期望 exit code `2`
- `target\release\yunxi.exe --help`，确认 backend 可选值仍不包含 `codex`
- `target\release\yunxi.exe sessions --help` 与 `sessions list --help`，确认 `--jsonl` 文案不误导 metadata 输出
- `target\release\yunxi.exe sessions list --jsonl`，期望明确拒绝
- `target\release\yunxi.exe parity map --jsonl`，期望明确拒绝
- sandbox acceptance suite：workspace escape、symlink escape、network disabled、danger-full-access bypass
- JSONL sandbox diagnostic fixture：包含 enforcement level、os_isolation、runner、unsupported_reason
- TUI 58x22 approval snapshot：包含 `Approve`、`Decline`、`Tab changes selection`
- TUI 80x22 approval snapshot：包含 `Approve`、`Decline`、`Tab changes selection`
- TUI 100x30 normal snapshot：provider/mode/model 可见
- TUI CJK/English spot check：Windows Terminal 中确认混排没有异常插空
- DeepSeek live stream smoke：从 `C:\Users\admin\Desktop\api.txt` 读取密钥，不打印密钥
- DeepSeek live JSON smoke：同上
- dependency scan：默认 CLI 不含 `codex-*`、`vendor/codex-rs`、`yunxi-agent-codex`
- owned-source secret scan：排除 `.git`、`.codegraph`、`target`、`vendor`、`extracted`
- `git diff --check`
- `codegraph sync "D:\YunXi Agent"`
- `codegraph status "D:\YunXi Agent"`
- `scripts\install\install-yunxi.ps1 -AddToPath -SkipBuild`
- PATH smoke：`yunxi --version`、`yunxi-agent-cli --version`、`yunxi --offline "installed path v1.7.7 smoke"`
- GitHub REST API 发布 `master` 与 annotated tag `v1.7.7`
- 本地 `master`、本地 `origin/master`、GitHub API branch、`v1.7.7` tag peeled target 一致性校验
- `cargo clean`
- `Test-Path D:\YunXi Agent\target`，期望 `False`

## 风险与处理

- 真实 OS sandbox 是平台安全工程，不应在 v1.7.7 用文案冒充完成。v1.7.7 可以通过“明确 policy-only + acceptance suite + 不误导 UI/JSONL”关闭用户误解风险；真实 restricted-token/Landlock 可进入后续安全专项。
- Approval 面板不能只靠增加固定高度修复。必须共享布局计算，否则 reason/command wrap 变化还会复发。
- `--jsonl` 从 global flag 拆分可能影响 CLI parsing。若改动过大，本版优先采用限制文案 + 明确拒绝 + help 测试；后续再重构参数模型。
- Header/subheader 不能为了显示 model 而重新引入 dangling separator。所有字符串拼接继续走 width-aware helper。
- Sandbox acceptance suite 不能在用户机器上留下 workspace 外文件；所有外部路径必须用临时目录并在测试结束清理。

## v1.7.7 完成后的预期状态

完成 v1.7.7 后，YunXi Agent 应达到：

- P0 用户误解风险关闭：默认 sandbox 边界被明确标记为 policy-only/process-lifecycle，除非实际 OS runner 通过 acceptance suite。
- P1 操作风险关闭：窄屏 TUI approval 不再裁掉 Decline 或快捷键说明。
- TUI 中等宽度能稳定显示 model 信息。
- CLI metadata help 不再暗示 `--jsonl` 可用于 metadata 输出。
- 默认运行路径继续保持 YunXi 自主化，不依赖上游 Codex CLI 源码。
- P0/P1 hard gate 关闭后，后续版本才继续推进新模块或更复杂 agent 能力。

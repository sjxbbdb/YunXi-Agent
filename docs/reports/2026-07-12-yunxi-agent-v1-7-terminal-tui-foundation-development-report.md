# YunXi Agent v1.7 Terminal TUI Foundation Development Report

生成时间：2026-07-12 18:39:48 +08:00

## 背景

当前发布版本为 `v1.6.0`。新的 v1.6 审核报告基于 `git show v1.6.0` 实际代码和测试复核，结论是：v1.6 的 policy guard hardening 是真实修复，不是换标签。默认 shell 工具路径已不再绕过 `ToolPolicy`，`WorkspaceWrite` 已具备进程内目标路径约束，token 级风险分类已经挡住 `rm  -rf` 一类简单绕过，流式 retry 也避免了 body 后重复事件。

审核报告同时指出，v1.6 仍不是 OS 级沙箱，也不是成熟终端 Agent 体验。对标 Claude Code / Codex CLI 的个人 agent，下一阶段的高收益方向不是继续在 v1.6 已修好的 policy 点上反复打磨，而是把当前可用但朴素的交互式 CLI 升级为更接近正式终端 Agent 的 TUI/行编辑基础层，同时补齐审核报告明确点名的安全回归测试缺口。

## 审核问题归纳

### 已确认 v1.6 做对的部分

- `crates/yunxi-agent-sandbox/src/lib.rs` 已实现 target extraction、workspace containment、token-aware risk。
- `crates/yunxi-agent-tools/src/lib.rs` 默认 shell path 已改用 `ExecCommand::shell(..., policy.execution_policy.clone())`。
- `crates/yunxi-agent-provider/src/lib.rs` 已阻止 streaming body 开始后的 retry。
- README 已如实说明 v1.6 是 process-internal policy guard，不是 OS-enforced sandbox。

### 下一版应处理的问题

- 交互式 CLI 仍是 `BufReader::read_line` + `println!` 的朴素 REPL，没有历史、方向键、多行编辑、清晰滚动区域或事件区。
- 当前 runtime event 直接渲染到 stdout，长 shell 输出、工具事件、MCP 事件、reasoning、assistant 文本混在一个线性流里，可读性弱。
- 当前 Ctrl+C 已能传播 cancellation，但空闲输入、运行中 turn、退出确认之间没有像 Codex/Claude Code 那样的终端交互语义。v1.7 只做基础预留，复杂 Ctrl+C 行为放到后续版本。
- `WorkspaceWrite` 已有 `canonicalize` 逻辑，但缺少 symlink 逃逸回归测试。审核报告明确指出这是当前测试唯一缺口。
- 复杂 shell 语法、TOCTOU、OS resource abuse 仍不是 v1.6 能防御的范围；v1.7 必须继续诚实记录，不把 TUI 或测试补强包装成 OS sandbox。

## v1.7 目标

- 版本升级到 `1.7.0`，发布新 tag `v1.7.0`，旧 tag 不删除、不移动。
- 引入终端体验基础层：默认在真实交互终端中启用 TUI/line editor；在 pipe、CI、`--json`、`--jsonl`、`--no-tui` 下保留现有 plain CLI 行为。
- 用 `reedline` 或同级 Rust 行编辑库替换裸 `BufReader::read_line` 的人类交互输入路径，提供历史、方向键、基础多行输入能力。
- 用 `ratatui` + `crossterm` 建立基础 TUI event render shell：顶部状态、滚动事件日志、底部输入区。v1.7 只做基础只读渲染，不做复杂多面板文件树。
- 建立 renderer 抽象，让 plain renderer、JSON/JSONL renderer、TUI renderer 分离，避免 TUI 破坏脚本输出。
- 保留现有 one-shot、sessions、JSON、JSONL、offline fallback warning、DeepSeek live provider 行为。
- 补充 symlink escape regression tests，证明 workspace target guard 在能解析 symlink 时会拒绝指向 workspace 外的目标。
- 更新 README、extraction status、开发报告和桌面日志，继续说明 v1.7 不是 OS-level sandbox。

## 非目标

- v1.7 不实现 Windows Restricted Token、Job Object、Linux Landlock/seccomp、namespace、macOS seatbelt。
- v1.7 不做完整多面板 IDE 式 TUI，不做文件树、diff navigator、复杂鼠标交互。
- v1.7 不重写 apply_patch parser，不把流式 diff preview 做成默认能力；最多为后续版本预留 renderer 事件模型。
- v1.7 不改变默认模型 provider 接口，不替换 DeepSeek/OpenAI-compatible provider 层。
- v1.7 不破坏 `yunxi "prompt"`、`yunxi --json`、`yunxi --jsonl`、`sessions` 子命令的自动化语义。

## 设计方案

### 1. 终端模式选择

新增交互模式选择逻辑：

- 当无 prompt 且 stdin/stdout 都是 terminal，默认进入 TUI interactive host。
- 当 stdin 或 stdout 不是 terminal，继续使用 plain interactive host，保证测试和脚本可预测。
- `--no-tui` 强制 plain interactive host。
- `--tui` 可显式请求 TUI；若当前不是 terminal，返回清晰错误或降级 warning，具体实现优先选择安全降级到 plain。
- `--json` / `--jsonl` 缺 prompt 时继续返回结构化错误，不进入 TUI。

主要文件：

- `crates/yunxi-agent-cli/src/main.rs`
- `crates/yunxi-agent-cli/src/interactive.rs`
- `crates/yunxi-agent-cli/src/terminal_mode.rs`（新增）
- `crates/yunxi-agent-cli/tests/cli_tests.rs`

### 2. 行编辑器切入

当前 `InteractiveSession::read_eval_loop()` 直接使用 `BufReader::read_line`。v1.7 将输入读取抽象为 `InteractiveInput`：

- plain/pipe 模式继续使用现有 `BufRead` 路径，保持 approval/user_input 测试稳定。
- terminal 模式使用 `reedline`，支持历史、方向键和基础多行输入。
- 历史文件保存在 workspace `.yunxi/history` 或用户级 YunXi 目录中；v1.7 优先使用 workspace `.yunxi/history`，便于项目隔离。
- slash commands 仍由现有 `commands.rs` 解析。

主要文件：

- `crates/yunxi-agent-cli/src/input.rs`（新增）
- `crates/yunxi-agent-cli/src/interactive.rs`
- `crates/yunxi-agent-cli/src/commands.rs`
- `crates/yunxi-agent-cli/tests/cli_tests.rs`

### 3. Renderer 抽象与基础 TUI

新增 renderer facade：

- `PlainInteractiveRenderer`：封装现有 `print_banner`、`render_agent_event`、approval prompt、status 输出。
- `TuiInteractiveRenderer`：维护 app state，用 `ratatui::Terminal` 渲染状态栏、事件日志和输入区。
- `InteractiveRenderer` trait：提供 `render_banner`、`render_event`、`render_status_line`、`render_error` 等入口。
- TUI 只消费 runtime event，不改变 runtime/provider/tool/storage 行为。

v1.7 TUI 页面保持保守：

- 顶部一行：版本、cwd、provider、model、sandbox、approval。
- 中间滚动日志：message、reasoning、tool started/completed、command started/updated/completed、MCP、warning/error。
- 底部输入区：当前 prompt 或 slash command。
- 不做复杂 nested cards，不做多面板文件树。

主要文件：

- `crates/yunxi-agent-cli/src/render.rs`
- `crates/yunxi-agent-cli/src/tui/mod.rs`（新增）
- `crates/yunxi-agent-cli/src/tui/app.rs`（新增）
- `crates/yunxi-agent-cli/src/tui/render.rs`（新增）
- `crates/yunxi-agent-cli/tests/cli_tests.rs`

### 4. 不破坏自动化输出

v1.7 的核心风险是 TUI 抢 stdout，破坏脚本和 JSONL。因此要求：

- one-shot plain 输出保持只输出最终 response 或 offline label。
- `--json` stdout 保持单个 JSON。
- `--jsonl` stdout 保持每行 JSON。
- auto fallback warning 在 JSON 中继续写 stderr，在 JSONL 中继续用结构事件。
- `sessions list/show/rollout/history/graph/resume` 行为保持兼容。

主要文件：

- `crates/yunxi-agent-cli/src/main.rs`
- `crates/yunxi-agent-cli/src/render.rs`
- `crates/yunxi-agent-cli/tests/cli_tests.rs`
- `crates/yunxi-agent-cli/tests/jsonl_tests.rs`

### 5. Symlink 逃逸回归测试

审核报告指出 v1.6 缺少 symlink 测试。v1.7 补齐：

- 在支持 symlink 的平台创建 workspace 内 symlink 指向 workspace 外临时目录。
- 执行 `echo hi > workspace_link/outside.txt` 或等价命令。
- `WorkspaceWrite` 应通过 `canonicalize` 解析 symlink，判断目标位于 workspace 外，返回 declined/escalated，不创建文件。
- 在 Windows 上如无 symlink 权限，测试必须明确 skip，不得误报通过。

主要文件：

- `crates/yunxi-agent-sandbox/src/lib.rs`
- `crates/yunxi-agent-tools/tests/tool_tests.rs`

## 文件范围

- `Cargo.toml`
- `Cargo.lock`
- `crates/yunxi-agent-cli/Cargo.toml`
- `crates/yunxi-agent-cli/src/main.rs`
- `crates/yunxi-agent-cli/src/interactive.rs`
- `crates/yunxi-agent-cli/src/input.rs`
- `crates/yunxi-agent-cli/src/terminal_mode.rs`
- `crates/yunxi-agent-cli/src/render.rs`
- `crates/yunxi-agent-cli/src/tui/mod.rs`
- `crates/yunxi-agent-cli/src/tui/app.rs`
- `crates/yunxi-agent-cli/src/tui/render.rs`
- `crates/yunxi-agent-cli/tests/cli_tests.rs`
- `crates/yunxi-agent-cli/tests/jsonl_tests.rs`
- `crates/yunxi-agent-sandbox/src/lib.rs`
- `crates/yunxi-agent-tools/tests/tool_tests.rs`
- `README.md`
- `docs/extraction-status.md`
- `docs/reports/2026-07-12-yunxi-agent-v1-7-terminal-tui-foundation-development-report.md`
- `docs/superpowers/plans/2026-07-12-yunxi-agent-v1-7-terminal-tui-foundation.md`
- `C:\Users\admin\Desktop\YunXi Agent开发日志.md`

## 开发阶段划分

### Phase 1：版本、依赖、模式选择

- 升级版本到 `1.7.0`。
- 增加 `ratatui`、`crossterm`、`reedline` 依赖。
- 新增 `--no-tui` / `--tui` interactive mode flags。
- 建立 terminal detection，不破坏 pipe/CI/JSON/JSONL。

### Phase 2：输入抽象与 line editor

- 抽出 `InteractiveInput`。
- terminal 模式接入 `reedline`。
- plain/pipe 模式保留现有 `BufRead` 行为。
- slash commands、approval、request_user_input 继续走兼容路径。

### Phase 3：Renderer facade 与 TUI app state

- 抽出 interactive renderer trait。
- 将现有 `println!` 渲染迁移到 `PlainInteractiveRenderer`。
- 新增 TUI app state 和 `ratatui::backend::TestBackend` 单元测试。
- TUI 仅负责显示，不改变 runtime 事件顺序。

### Phase 4：基础 TUI host

- 在真实 terminal 中启用 alternate screen。
- 渲染顶部状态、滚动事件日志、底部输入区。
- 确保 panic/error/drop 时退出 alternate screen 并恢复 terminal。
- 保留 `--no-tui` 回退。

### Phase 5：安全回归补测

- 增加 symlink escape unit/integration tests。
- 明确记录 Windows symlink 权限不足时的 skip 条件。
- 保持 v1.6 policy guard 行为不回退。

### Phase 6：统一验证、发布、日志

- 统一运行验证门。
- 安装 release 到本机 PATH。
- 创建 commit 和 annotated tag `v1.7.0`。
- 通过 GitHub REST API 发布并回读核验。
- `cargo clean` 并确认 `target_exists=False`。
- 追加桌面开发日志。

## 统一验证门

构建完成后统一执行：

- `cargo fmt`
- `cargo fmt -- --check`
- `cargo test`
- `cargo check --workspace`
- `cargo check --workspace --target x86_64-unknown-linux-gnu`（如 target 仍被本机 rustup 镜像 404 阻塞，记录真实原因，不宣称通过）
- `cargo build -p yunxi-agent-cli --release --bins`
- `target\release\yunxi.exe --version` 输出 `yunxi 1.7.0`
- `target\release\yunxi-agent-cli.exe --version` 输出 `yunxi 1.7.0`
- `yunxi --offline "installed offline smoke"` 输出 `[offline]`
- pipe/plain interactive smoke：`yunxi --offline --no-tui` 可通过 stdin `/exit` 正常退出
- TUI state unit tests：`ratatui::backend::TestBackend` 渲染 banner/status/event/input 区域不为空
- `--json` 缺 prompt 仍返回结构化错误或保持当前错误语义
- `--jsonl` 缺 prompt 每行仍为 JSON
- one-shot auto fallback plain/JSON/JSONL/resume smoke 保持 v1.6 行为
- DeepSeek live non-stream smoke：脱敏通过
- DeepSeek live streaming smoke：脱敏通过
- symlink escape regression：支持 symlink 的平台必须 blocked；无权限平台必须明确 skip
- dependency scan：default CLI graph 中 `codex-*`、`vendor/codex-rs`、`yunxi-agent-codex` 命中 0
- owned-source high-confidence secret scan 命中 0
- `git diff --check`
- `codegraph sync "D:\YunXi Agent"`
- release install 到 `C:\Users\admin\AppData\Local\YunXi Agent\bin`
- PATH `yunxi --version` 输出 `yunxi 1.7.0`
- GitHub REST API 发布并回读核验 `master` 和 `v1.7.0`
- 旧 tag `v1.0.0` 到 `v1.6.0` 不删除、不移动
- `cargo clean`
- `target_exists=False`

## 风险与控制

- 风险：TUI 改动破坏脚本输出。
  控制：TUI 仅在真实 terminal + 无 prompt + 非 JSON/JSONL 下默认启用；`--no-tui` 强制 plain；所有自动化路径保留回归测试。

- 风险：TUI 引入终端状态污染。
  控制：用 RAII guard 管理 alternate screen/raw mode；error/drop 时恢复 terminal；plain fallback 永远可用。

- 风险：行编辑器影响 approval/user_input prompt。
  控制：v1.7 保持 approval/user_input 使用兼容 plain prompt；后续再把它们搬进 TUI modal。

- 风险：依赖膨胀。
  控制：只引入 `ratatui`、`crossterm`、`reedline` 三个直接终端依赖；不引入 GUI、webview、desktop app。

- 风险：symlink 测试在 Windows 权限不足。
  控制：测试明确检测 symlink 创建能力；无权限 skip 并输出原因，有权限则必须验证 blocked。

- 风险：用户误以为 TUI 等于更安全。
  控制：README 和报告继续说明 v1.7 不提供 OS-level sandbox。

## 发布策略

- 下一版本号：`v1.7.0`。
- 创建新 annotated tag `v1.7.0`；旧 tag 不删除、不移动。
- GitHub 读写、tag 发布、远端核验全部走 REST API。
- 不使用 `git push`、`git fetch`、`git ls-remote`。
- API key/PAT 不打印、不写日志、不写源码、不提交。
- 发布后同步本地 tracking ref，保持主目录与远端 `master` 一致。

## 成功判定

v1.7.0 完成后，YunXi Agent 应从“能在终端对话的 plain CLI”升级为“有正式终端交互基础的 Agent CLI”：真实终端默认具备 TUI/line editor 体验，脚本和 JSON/JSONL 仍保持稳定，runtime event 可被结构化渲染，现有 DeepSeek live、offline fallback、sessions、tools、MCP、policy guard 能力不回退。安全方面，v1.7 不追求 OS-level sandbox，但必须补齐 symlink escape 回归测试，并继续诚实标注 process-internal guard 边界。

## 实际实施结果

记录时间：2026-07-12 19:20:40 +08:00

本轮已按 v1.7 计划完成构建：

- 工作区版本升级为 `1.7.0`，`yunxi` 与兼容二进制 `yunxi-agent-cli` 均输出 `yunxi 1.7.0`。
- `yunxi-agent-cli` 增加 `ratatui`、`crossterm`、`reedline` 终端依赖，新增 `--tui` / `--no-tui`。
- 新增 terminal mode resolver：真实 terminal 默认走 TUI-capable host，pipe/CI/自动化路径继续走 plain。
- 新增 `InteractiveInput` 抽象，plain 输入与 Reedline terminal 输入分离。
- 新增 `InteractiveRenderer` 抽象，plain renderer 与基础 TUI renderer 分离。
- 新增 TUI app state 与 `ratatui::backend::TestBackend` 渲染测试，覆盖 header、event log、input 区。
- `WorkspaceWrite` 路径判断补强为 symlink-aware containment：目标不存在时先 canonicalize 最近存在前缀，再拼接缺失路径。
- `yunxi-agent-sandbox` 与 `yunxi-agent-tools` 均新增 symlink escape 回归测试；能创建 symlink 的平台必须证明 workspace 内 symlink 指向 workspace 外部时被拒绝。
- README 和 extraction status 同步说明 v1.7 是 terminal UX foundation + process-internal guard 回归补强，不声称 OS-level sandbox。

## 统一验证结果

已执行的验证门：

- `cargo fmt`：通过。
- `cargo fmt -- --check`：通过。
- `cargo test`：通过；workspace 单测、集成测试、doc-test 全部完成，包含 terminal mode、TUI TestBackend、symlink escape 回归。
- `cargo check --workspace`：通过。
- `cargo check --workspace --target x86_64-unknown-linux-gnu`：未通过，原因是本机未安装该 target 标准库；随后执行 `rustup target add x86_64-unknown-linux-gnu`，被当前 rustup 镜像 `https://mirrors.tuna.tsinghua.edu.cn/rustup/dist/2026-06-30/rust-std-1.96.1-x86_64-unknown-linux-gnu.tar.xz` 404 阻塞。该项记录为环境阻塞，不宣称通过。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- `target\release\yunxi.exe --version`：通过，输出 `yunxi 1.7.0`。
- `target\release\yunxi-agent-cli.exe --version`：通过，输出 `yunxi 1.7.0`。
- pipe/plain interactive smoke：`'/exit' | target\release\yunxi.exe --offline --no-tui` 正常显示 plain banner 并退出。
- auto fallback plain/JSON/JSONL smoke：通过；plain 输出有 fallback warning 与 `[offline]`，JSON stdout 可解析且 warning 在 stderr，JSONL stdout 每行可解析且无 `[warning]` 字面污染。
- DeepSeek live non-stream smoke：通过；使用 `deepseek-chat`，`--json` 输出可解析并包含 `OK`，无 secret leak。
- DeepSeek live streaming smoke：通过；使用 `deepseek-chat`，`--jsonl` 输出 19 行可解析 JSONL 并包含 `OK`，无 secret leak。
- dependency scan：`cargo tree -p yunxi-agent-cli` 中 `codex-*`、`vendor/codex-rs`、`yunxi-agent-codex` 命中 0。
- owned-source high-confidence secret scan：排除 `.git`、`target`、`.codegraph`、`extracted`、`vendor` 后命中 0。
- `git diff --check`：通过，仅有 Windows LF-to-CRLF 工作区提示。
- `codegraph sync "D:\YunXi Agent"`：通过，同步 11 个 changed files。
- release install：首次安装被旧的已安装 `yunxi.exe` 进程占用阻塞；确认占用进程路径为 `C:\Users\admin\AppData\Local\YunXi Agent\bin\yunxi.exe` 后结束该旧进程并重试安装，通过。
- PATH smoke：从 `C:\Users\admin` 执行 `yunxi --version` 输出 `yunxi 1.7.0`。
- installed offline smoke：从 `C:\Users\admin` 执行 `yunxi --offline "installed offline smoke"` 输出 `[offline] YunXi autonomous runtime accepted prompt: installed offline smoke`。

## 发布收口记录

- 本报告随 v1.7 release commit 提交。
- 发布策略保持不变：创建新的 annotated tag `v1.7.0`，旧 tag 不删除、不移动。
- GitHub 远端写入、tag 创建和回读核验继续走 GitHub REST API，不使用 `git push`、`git fetch`、`git ls-remote`。
- 发布完成后的 commit/tag、REST API 回读、旧 tag 保持情况和 `cargo clean` 结果会同步写入桌面 `YunXi Agent开发日志.md`。

# YunXi Agent v1.6 Policy Guard Hardening Development Report

生成时间：2026-07-12 16:46:00 +08:00

## 背景

当前发布版本为 `v1.5.0`。新的审核报告基于 `v1.5.0` 实际源码和 diff 做复审，结论是：v1.5 对离线诚实性和真流式做了有效修复，但“safety”主要停留在诚实命名层面。当前 sandbox 仍是 policy/advisory 数据层，不是 OS 级隔离；更关键的是工具 shell 路径仍通过 `ExecCommand::observed_shell` 使用 `DangerFullAccess + PreApproved`，使前置 policy 评估没有真正约束最终执行。

v1.6 的目标不是继续堆外围功能，而是把 v1.5 暴露出的安全边界缺口先补成“至少真实执行的进程内保护”。这仍不等同 Windows Restricted Token、Linux Landlock、seccomp 或 namespace，但必须让 YunXi 默认工具执行路径不再绕过自己的 policy，并能阻止最常见的误伤写入和高风险命令。

## 审核问题归纳

### 本版必须处理

- `S-1`：工具 shell 执行经由 `observed_shell`，硬编码 `DangerFullAccess + PreApproved`，绕过 `ToolPolicy` 的 sandbox/approval/network 约束。
- `S-2`：`WorkspaceWrite` 只验证 cwd 是否在 workspace 内，不验证命令写入目标；从 workspace cwd 执行绝对路径、`..`、重定向等仍可能写到 workspace 外。
- `S-3`：风险分类仍是 substring 匹配，`rm  -rf`、参数变形、PowerShell 别名/分隔符等容易绕过。
- `S-4`：非 Windows `platform_shell` 引用未导入的 `Command`，任何非 Windows workspace build 仍可能编译失败。
- `S-5`：流式 retry 跨 attempt 复用同一个 sink/accumulator；如果首个 attempt 已发出 chunk 后失败并重试，可能造成重复或交错事件。
- `S-6`：auto fallback warning 只在 interactive banner 中打印，one-shot 和 `sessions resume` 路径缺少明确“无凭据、未调用模型”的 warning。

### 本版记录但不一次性展开

- 真实 OS 级 sandbox：Windows Job Object/Restricted Token、Linux Landlock/seccomp、macOS seatbelt 是单独大工程。本版只做进程内强制 policy guard，不声称 OS isolation。
- REPL 行编辑、历史、补全、多行输入、空闲 Ctrl+C 行为继续留到后续终端体验版本。
- apply_patch hunk 行锚定、symlink 逃逸防护、真实 tokenizer、view_image 解码、tool_search 内容索引、MCP 长生命周期复用继续保留在后续 parity 队列。

## v1.6 目标

- 将版本升级到 `1.6.0`，发布新 tag `v1.6.0`，旧 tag 不删除、不移动。
- 默认工具 shell 执行必须使用 runtime 传入的 `ToolPolicy.execution_policy`，不得再走 `observed_shell` 的 `DangerFullAccess + PreApproved`。
- `WorkspaceWrite` 模式下新增进程内写入目标 guard：绝对路径、`..`、明显指向 workspace 外的重定向/复制/移动/删除目标必须拦截或要求 escalation。
- read-only / workspace-write / danger-full-access 三种模式在工具执行结果、trace、CLI 输出和 JSONL 中保持一致。
- 风险分类从简单 substring 改为 shell-aware/token-aware 分类，至少覆盖常见 Windows PowerShell、cmd、POSIX shell 变形。
- 非 Windows workspace 编译错误修复，至少 `cargo check --workspace --target x86_64-unknown-linux-gnu` 或等价 Linux target gate 能证明不再因 `platform_shell` 死函数失败。
- 流式 retry 在已发出任何 bytes/event 后不得复用同一 accumulator 重试；策略改为“headers/status 阶段可 retry，body 已开始后失败则返回 stream error”，避免重复事件。
- one-shot 和 `sessions resume` 路径补齐 auto fallback warning，行为与 interactive 一致，同时不污染 `--json`/`--jsonl` 结构输出。
- 文档继续诚实说明：v1.6 是 policy guard hardening，不是 OS-level sandbox。

## 非目标

- v1.6 不实现完整 OS 级隔离，不把 advisory label 改回 sandbox enforcement。
- v1.6 不引入 TUI、desktop app、cloud tasks、doctor、completion、marketplace、release updater。
- v1.6 不重写完整 patch 引擎，不处理 symlink 逃逸；只处理 shell/tool policy guard。
- v1.6 不改 provider 模型接口设计，不扩大到其他模型供应商。
- 构建过程中继续遵守硬性约束：先完成源码构建，最后统一验证，不在单点反复消耗时间。
- GitHub 读写、发布、tag 核验继续全部走 REST API；不使用 `git push`、`git fetch`、`git ls-remote`。

## 设计方案

### 1. Shell 执行不再绕过 ToolPolicy

当前 `ToolPolicy::evaluation_for()` 会根据 `AgentConfig` 生成 `ExecutionPolicy` 并评估命令，但最终 `run_shell()` 调用 `ExecCommand::observed_shell()`，该构造器固定使用 `DangerFullAccess + PreApproved`。v1.6 要把 policy 贯穿到最终 `ExecManager`：

- 修改 `run_shell` 签名，显式接收 `ToolPolicy` 或 `ExecutionPolicy`。
- 用 `ExecCommand::shell(cwd, command, execution_policy)` 替代 `ExecCommand::observed_shell(...)`。
- 保留 `observed_shell` 仅用于测试或明确 trusted/internal 调用，不能出现在默认工具执行路径。
- 增加测试断言：read-only 下写命令不会 spawn；workspace-write 下 workspace 外写入不会 spawn；danger-full-access 才允许直通。

主要文件：

- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-exec/src/lib.rs`
- `crates/yunxi-agent-tools/tests/tool_tests.rs`
- `crates/yunxi-agent-runtime/tests/runtime_tests.rs`

### 2. WorkspaceWrite 写入目标 guard

本版不做 OS 沙箱，但要在进程内尽量阻止常见误伤。新增一个轻量命令目标提取器：

- 对 PowerShell：识别 `>`, `>>`, `Out-File`, `Set-Content`, `Add-Content`, `New-Item`, `Copy-Item`, `Move-Item`, `Remove-Item` 的目标参数。
- 对 cmd/POSIX shell：识别 `>`, `>>`, `cp`, `mv`, `rm`, `del`, `erase`, `rmdir`, `mkdir`, `touch` 等常见目标。
- 对目标路径做 normalization 和 workspace containment 检查。
- 对不能可靠解析但明显高风险的命令，返回 `NeedsApproval` 或 `DangerFullAccess` escalation，不静默允许。

这个 guard 不是安全沙箱，但能挡住个人 agent 最容易造成的数据误伤：从 workspace 内写到外部绝对路径、上级目录和敏感路径。

主要文件：

- `crates/yunxi-agent-sandbox/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-sandbox/tests`（如需新增）
- `crates/yunxi-agent-tools/tests/tool_tests.rs`

### 3. 风险分类 token 化

把 `CommandRisk::classify` 从简单 `contains` 提升为 token-aware：

- 先做 shell-ish tokenization，保留 quoted string 边界，规范化连续空白。
- 把命令动词和参数拆开判断，例如 `rm  -rf`、`rm -r -f`、`Remove-Item -Recurse -Force` 都归为 destructive。
- 网络命令按首 token 和常见 module/cmdlet 判断。
- credential access 按 token 和 path suffix 判断，减少误报但不放过 `.env`、`id_rsa`、`credentials` 等。
- 输出分类 reason，方便 trace/debug。

主要文件：

- `crates/yunxi-agent-sandbox/src/lib.rs`
- `crates/yunxi-agent-sandbox/tests` 或模块内 tests

### 4. 流式 retry 不重复事件

v1.5 的网络流式是真的，但 `send_streaming_with_retries` 复用 sink/accumulator。v1.6 要明确 retry 边界：

- `ProviderByteStreamSink` 增加是否已收到 body bytes 的状态，或由 `send_streaming_with_retries` 包装一个 counting sink。
- HTTP status 非 2xx 且未推送 body：允许按 retry policy retry。
- transport error 发生在首个 body chunk 之前：允许 retry。
- 一旦已推送任何 body bytes 或任何 parsed stream event：不再 retry，返回 redacted stream error，让 runtime emit provider error/cancelled-like terminal event。
- schema fallback 只允许在未开始 body 时执行，避免 DeepSeek fallback 造成重复 event。

主要文件：

- `crates/yunxi-agent-provider/src/lib.rs`
- `crates/yunxi-agent-provider/tests/provider_tests.rs`
- `crates/yunxi-agent-runtime/tests/runtime_tests.rs`

### 5. Auto fallback warning 覆盖 one-shot/resume

交互模式已有 warning；v1.6 要补齐：

- plain one-shot：在 assistant 输出前打印 warning。
- plain `sessions resume`：同样打印 warning。
- `--json`：不混入 stdout 文本，建议在 JSON result 或 stderr 中加入结构化 warning；本版可先写 stderr，避免破坏 JSON stdout。
- `--jsonl`：用 `RuntimeEvent::Warning` 或 CLI synthetic warning JSONL 行输出，避免非 JSON 文本污染。

主要文件：

- `crates/yunxi-agent-cli/src/main.rs`
- `crates/yunxi-agent-cli/src/provider_mode.rs`
- `crates/yunxi-agent-cli/tests/cli_tests.rs`
- `crates/yunxi-agent-cli/tests/jsonl_tests.rs`

### 6. 跨平台编译修复

当前 `tools/src/lib.rs` 非 Windows `platform_shell` 死函数引用未导入 `Command`。v1.6 处理：

- 删除未使用的 `platform_shell` 函数，或补 `use std::process::Command` 并加测试/target check。
- 如果删除，确认无调用者和行为差异。
- 增加 Linux target check 到统一验证门；如本机缺 target，则记录安装或跳过原因，不宣称跨平台完全验证。

主要文件：

- `crates/yunxi-agent-tools/src/lib.rs`

## 文件范围

- `Cargo.toml`
- `Cargo.lock`
- `crates/yunxi-agent-cli/src/main.rs`
- `crates/yunxi-agent-cli/src/provider_mode.rs`
- `crates/yunxi-agent-cli/tests/cli_tests.rs`
- `crates/yunxi-agent-cli/tests/jsonl_tests.rs`
- `crates/yunxi-agent-exec/src/lib.rs`
- `crates/yunxi-agent-provider/src/lib.rs`
- `crates/yunxi-agent-provider/tests/provider_tests.rs`
- `crates/yunxi-agent-runtime/tests/runtime_tests.rs`
- `crates/yunxi-agent-sandbox/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-tools/tests/tool_tests.rs`
- `README.md`
- `docs/extraction-status.md`
- `docs/reports/2026-07-12-yunxi-agent-v1-6-policy-guard-hardening-development-report.md`
- `docs/superpowers/plans/2026-07-12-yunxi-agent-v1-6-policy-guard-hardening.md`
- `C:\Users\admin\Desktop\YunXi Agent开发日志.md`

## 开发阶段划分

### Phase 1：版本与 warning 一致性

- 版本升级到 `1.6.0`。
- one-shot/resume/plain/json/jsonl 补齐 auto fallback warning。
- 保持离线 `[offline]` inline 输出不变。

### Phase 2：ToolPolicy 贯穿执行层

- `run_shell` 使用 request policy 生成 `ExecCommand::shell`。
- 默认工具 shell 不再调用 `observed_shell`。
- read-only/workspace-write/danger-full-access 增加 regression tests。

### Phase 3：WorkspaceWrite 写入目标 guard

- 实现 shell-ish 目标提取器。
- 拦截 workspace 外写入。
- 对不可解析高风险命令要求 approval/escalation。

### Phase 4：风险分类 token 化

- 替换 substring-only classifier。
- 覆盖 destruct/network/credential/process/write/read 常见变形。
- 输出分类 reason 或 diagnostic，便于调试。

### Phase 5：流式 retry 边界

- body 未开始前可 retry。
- body/event 已开始后不 retry，返回 stream error。
- DeepSeek schema fallback 仅在未开始 body 时触发。

### Phase 6：跨平台检查、文档、发布

- 修非 Windows dead function 编译问题。
- 更新 README/extraction-status/report。
- 统一验证、安装、CodeGraph、GitHub REST API 发布、cargo clean。

## 统一验证门

构建完成后统一执行：

- `cargo fmt`
- `cargo fmt -- --check`
- `cargo test`
- `cargo check --workspace`
- `cargo check --workspace --target x86_64-unknown-linux-gnu`（如 target 缺失，先安装 target；若外部环境阻塞，必须记录真实原因）
- `cargo build -p yunxi-agent-cli --release --bins`
- `target\release\yunxi.exe --version` 输出 `yunxi 1.6.0`
- `target\release\yunxi-agent-cli.exe --version` 输出 `yunxi 1.6.0`
- one-shot auto fallback 无 key smoke：plain 输出 warning + `[offline]`
- sessions resume auto fallback 无 key smoke：plain 输出 warning + `[offline]`
- JSON/JSONL fallback smoke：stdout 不被非结构化 warning 污染
- read-only shell write smoke：命令不 spawn，返回 declined/blocked
- workspace-write outside target smoke：绝对路径和 `..` 写入被 blocked/escalated
- danger-full-access smoke：明确 bypass policy guard 后允许执行
- command risk smoke：`rm  -rf`、`rm -r -f`、`Remove-Item -Recurse -Force` 均归为 destructive
- streaming retry smoke：body 前失败可 retry；body 后失败不 retry 且不重复 event
- DeepSeek live non-stream smoke：脱敏通过
- DeepSeek live streaming smoke：assistant event 早于 turn_completed，脱敏通过
- default CLI dependency scan：`codex-*`、`vendor/codex-rs`、`yunxi-agent-codex` 命中 0
- owned-source secret scan：API key、Bearer、GitHub PAT 模式命中 0
- `git diff --check`
- `codegraph sync "D:\YunXi Agent"`
- release install 到 `C:\Users\admin\AppData\Local\YunXi Agent\bin`
- PATH `yunxi --version` 输出 `yunxi 1.6.0`
- GitHub REST API 发布并回读核验 `master` 和 `v1.6.0`
- 旧 tag `v1.0.0` 到 `v1.5.0` 不删除、不移动
- `cargo clean`
- `target_exists=False`

## 实际实施结果

更新时间：2026-07-12 17:36:24 +08:00

- 版本已升级到 `1.6.0`，`yunxi` 与兼容二进制 `yunxi-agent-cli` 均输出 `yunxi 1.6.0`。
- one-shot、`--json`、`--jsonl`、`sessions resume` 的 auto offline fallback warning 已补齐；plain 模式输出 warning + `[offline]`，JSON warning 写入 stderr，JSONL warning 使用结构化事件，不混入普通文本。
- 默认 shell 工具路径已改为使用 `ToolPolicy.execution_policy` 构建 `ExecCommand::shell(...)`，不再通过 `observed_shell` 绕过 policy。
- `WorkspaceWrite` 已新增常见写入/删除目标提取与 workspace containment guard，可拦截明显指向 workspace 外的绝对路径和 `..` 逃逸目标。
- `CommandRisk::classify` 已升级为 shell-ish token-aware 分类，覆盖 `rm  -rf`、`rm -r -f`、`Remove-Item -Recurse -Force`、网络命令、credential 文件等常见变形，并保留兼容 fallback。
- provider streaming retry 已新增 body-start guard：body 前失败可 retry，body 已开始后不 retry，避免重复事件；DeepSeek schema fallback 也只允许在 body 未开始前触发。
- 已删除 `yunxi-agent-tools` 中非 Windows 未使用且会触发缺失 import 的 `platform_shell` 死函数。
- README 与 `docs/extraction-status.md` 已同步 v1.6 能力边界：本版是 process-internal policy guard hardening，不声称 OS-level sandbox。

## 统一验证结果

统一验证时间：2026-07-12 17:36-17:58 +08:00。

- `cargo fmt`：通过。
- `cargo fmt -- --check`：通过。
- `cargo test`：通过，CLI、provider、runtime、sandbox、tools、storage 等测试集全部退出码 0。
- `cargo check --workspace`：通过。
- `cargo check --workspace --target x86_64-unknown-linux-gnu`：环境阻塞。本机缺少 `x86_64-unknown-linux-gnu` target；执行 `rustup target add x86_64-unknown-linux-gnu` 被本机 rustup 配置的清华镜像 `https://mirrors.tuna.tsinghua.edu.cn/rustup/dist/2026-06-30/rust-std-1.96.1-x86_64-unknown-linux-gnu.tar.xz` 返回 404 阻塞；同进程设置 `RUSTUP_DIST_SERVER=https://static.rust-lang.org` 后仍被导向该镜像。该项未宣称通过。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- `target\release\yunxi.exe --version`：输出 `yunxi 1.6.0`。
- `target\release\yunxi-agent-cli.exe --version`：输出 `yunxi 1.6.0`。
- auto offline fallback smoke：plain、JSON、JSONL、`sessions resume` 均通过；JSON stdout 可被解析，JSONL 每行均为 JSON，warning 未以非结构文本污染结构输出。
- policy/risk/streaming retry smoke：由 `cargo test` 中新增 regression 覆盖并通过，包括 read-only shell write decline、workspace-write outside target decline、danger-full-access allow、token-aware destructive classification、streaming body-before retry 和 body-after no-retry。
- DeepSeek live non-stream smoke：使用桌面 `api.txt` 中候选 key 的第 1 个有效 key，模型 `deepseek-chat`，`--json` 返回 `OK`，退出码 0，未发现 key 泄露。
- DeepSeek live streaming smoke：同一 key 与模型，`--jsonl` 输出 19 行 JSONL，包含 `turn_completed`，退出码 0，未发现 key 泄露。
- default CLI dependency scan：`cargo tree -p yunxi-agent-cli` 中 `codex-*`、`vendor/codex-rs`、`yunxi-agent-codex` 命中 0。
- owned-source high-confidence secret scan：排除 `.git`、`target`、`.codegraph`、`extracted`、`vendor` 后扫描 155 个文件，API key、GitHub PAT、Authorization Bearer 高置信规则命中 0；宽泛 bearer 文本扫描只命中文档说明和测试夹具，未发现真实凭据。
- `git diff --check`：通过，仅有 Windows 工作区 LF/CRLF 提示，无 whitespace error。
- `codegraph sync "D:\YunXi Agent"`：通过，CodeGraph 已是 up to date。
- release install：`scripts\install\install-yunxi.ps1 -AddToPath -SkipBuild` 通过，安装目录为 `C:\Users\admin\AppData\Local\YunXi Agent\bin`，`path_updated=False`。
- PATH smoke：`yunxi --version` 输出 `yunxi 1.6.0`；`yunxi --offline "installed offline smoke"` 输出 `[offline] YunXi autonomous runtime accepted prompt: installed offline smoke`。

## 发布记录

- 本地 release commit 与 annotated tag `v1.6.0` 将在报告更新后创建。
- GitHub `master` 与 `refs/tags/v1.6.0` 将继续通过 REST API 发布和回读核验；不使用 `git push`、`git fetch`、`git ls-remote`。
- 旧 tag `v1.0.0` 到 `v1.5.0` 必须保留，不删除、不移动。
- 发布完成后执行 `cargo clean` 并确认 `target_exists=False`。

## 风险与控制

- 风险：进程内写入 guard 不是安全沙箱，容易被复杂 shell 表达式绕过。
  控制：文档继续诚实声明 no OS isolation；对不可解析高风险命令默认 approval/escalation，不静默允许。

- 风险：过严 guard 误拦常见构建命令。
  控制：默认允许 cwd 在 workspace 内的低风险 build/test/read；只针对明显外部写入和 destructive/process/network/credential 风险升级。

- 风险：shell tokenization 跨 PowerShell/cmd/POSIX 差异大。
  控制：先覆盖高频模式和测试样例，保留 conservative fallback；后续再升级为成熟 parser。

- 风险：流式 body 后不 retry 会让中途断流更快失败。
  控制：这是正确性优先取舍，避免重复 token/event；错误要进入 provider error/stream error 事件，用户可重试整轮。

- 风险：Linux target check 可能受本机 Rust target 未安装影响。
  控制：验证门明确安装 target 或记录阻塞原因，不用“应该能跨平台”替代证据。

## 发布策略

- 下一版本号：`v1.6.0`。
- 创建新 annotated tag `v1.6.0`；旧 tag 不删除、不移动。
- GitHub 读写、tag 发布、远端核验全部走 REST API。
- 不使用 `git push`、`git fetch`、`git ls-remote`。
- API key/PAT 不打印、不写日志、不写源码、不提交。
- 发布后同步本地 tracking ref，保持主目录与远端 `master` 一致。

## 成功判定

v1.6.0 完成后，YunXi Agent 不再只是在输出里承认 policy guard 是 advisory，而是能在默认工具执行路径中真实执行进程内 policy：read-only 和 workspace-write 会影响 shell 是否能 spawn，workspace 外写入会被拦截或升级，风险分类能识别常见变形。流式 retry 不再可能复制已发事件；one-shot/resume 与 interactive 在离线 fallback 告警上保持一致；非 Windows 编译缺口得到处理。v1.6 仍不声称 OS-level sandbox，但它必须比 v1.5 更接近一个可信的个人终端 Agent 安全边界。

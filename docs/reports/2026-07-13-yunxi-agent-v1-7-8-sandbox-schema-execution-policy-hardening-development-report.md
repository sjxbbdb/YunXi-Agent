# YunXi Agent v1.7.8 Sandbox Schema And Execution Policy Hardening Development Report

生成时间：2026-07-13 15:24:04 +08:00

## 背景

YunXi Agent v1.7.7 已关闭 1.7.6 审核报告中的主要 TUI 与 CLI contract 问题：窄屏 approval 操作区可见，header/model 信息可见，metadata `--jsonl` help 误导性已降低，sandbox event 也开始输出 `enforcement`、`enforcement_level`、`runner`、`unsupported_reason` 和 `os_isolation=false`。

用户提供了新的审核报告：

- `C:\Users\admin\Desktop\YunXi-Agent-CLI-TUI-审核报告-1.7.7-2026-07-13.md`

本报告面向 v1.7.8。用户已明确：1.8 是后续新模块版本，当前暂不考虑。因此 v1.7.8 不扩展新模块，不跳到 1.8；它是 1.7 系列继续收口的安全边界版本，目标是把 v1.7.7 已经建立的 sandbox honesty 继续固化为机器稳定 schema、统一执行策略闸门和可回归测试。

## 审核报告结论复核

本轮已读取审核报告全文，并按仓库规则优先使用 CodeGraph 核对关键代码落点。审核报告结论与当前代码状态一致：

- v1.7.7 没有发现新的 P0/P1 功能回归。
- TUI approval 窄屏裁剪问题已关闭，58 列紧凑尺寸仍可见 `Approve`、`Decline` 与 `Tab changes selection`。
- TUI header/model 可见性已改善，58/100 列快照均能保留关键 provider/model 信息。
- metadata `--jsonl` help 已明确为 agent execution events，metadata command 会拒绝该 flag。
- 仍存在首要安全风险：当前 Windows runner 是 `process_lifecycle`，`os_isolation=false`，没有真实文件系统 OS 隔离。
- 当前 `sandbox_attempt` 事件对人类更清楚，但机器 schema 仍可收口：`backend` 是用户可读字符串，`enforcement` 与 `enforcement_level` 语义接近，未来外部脚本依赖后再调整会增加兼容成本。

CodeGraph 核对到的当前事实：

- `D:\YunXi Agent\crates\yunxi-agent-sandbox\src\lib.rs` 已定义 `ExecutionPolicy`、`SandboxRequirement`、`NetworkPolicy`、`SandboxEnforcementLevel`、`SandboxBackend` 与 `SandboxRunner::diagnostic()`。
- `SandboxEnforcementLevel::os_isolation()` 只有 `OsRestricted` 返回 true。
- Windows 默认 `SandboxBackend::WindowsRestrictedToken` 当前用户文案为 `process lifecycle runner: policy guard only, no filesystem OS isolation`，runner label 为 `windows_process_lifecycle`，unsupported reason 明确说明 restricted-token filesystem isolation 未启用。
- `D:\YunXi Agent\crates\yunxi-agent-exec\tests\sandbox_acceptance_tests.rs` 已覆盖 read-only write、workspace absolute escape、symlink escape、disabled network、danger-full-access bypass 和 honest diagnostic。

因此 v1.7.8 的成功标准不是“宣称真实沙箱完成”，而是让所有执行路径、事件 schema、CLI/TUI/JSONL/日志的安全边界都可审计、可机器验证、不可误导。

## v1.7.8 总目标

YunXi Agent v1.7.8 的目标是关闭 v1.7.7 审核报告指出的剩余 hardening 缺口：

- P0：任何 shell、patch、MCP、skill、多 agent 子任务或未来工具执行入口，都不能绕过 `ExecutionPolicy` / `SandboxRequirement` / `NetworkPolicy`。
- P0：`danger-full-access` 必须始终作为显式 `policy_bypass` 呈现，不能被伪装成受保护执行。
- P0：所有用户可见和机器可读位置都必须保持诚实：当前默认 Windows 路径是 `policy guard + process lifecycle, no filesystem OS isolation`。
- P1：冻结 `sandbox_attempt` JSONL schema，增加稳定机器字段，减少人类文案字段对自动化消费者的影响。
- P1：为所有新增或变更的执行入口补 acceptance tests；如果某执行入口暂不存在，也要在报告和测试命名中明确“不存在可绕过路径”。
- P2：保留 v1.7.7 TUI/CLI 回归基线，不让视觉和 metadata contract 退化。
- P3：CJK 混排真实终端抽查作为非阻塞视觉复核，不阻塞 v1.7.8 安全收口。

## 用户硬性约束

实现 v1.7.8 时继续遵守以下约束：

- 先按报告完整构建源码，中间不做反复单点测试，不在某个点卡太久。
- 源码构建完成后统一执行验证。
- 每个正式版本必须创建新的不可变 annotated tag，旧 tag 不删除、不移动。
- GitHub 读写和推送只走 REST API，不使用 `git push`、`git fetch`、`git ls-remote`。
- API 密钥只从 `C:\Users\admin\Desktop\api.txt` 读取，不打印、不写日志、不提交。
- 每次任务结束必须追加 `C:\Users\admin\Desktop\YunXi Agent开发日志.md`。
- 发布收尾前执行 `cargo clean` 清理构建中间产物。
- v1.7.8 不跳到 1.8，不引入 1.8 新模块。
- 不允许把当前 runner 称作真实 OS filesystem sandbox。

## 非目标

- v1.7.8 不新增桌面端、云任务、插件市场、自动更新器、SDK 包装或新产品面。
- v1.7.8 不恢复默认 CLI 对上游 Codex runtime、`vendor/codex-rs`、`codex-*` 或 `yunxi-agent-codex` 的依赖。
- v1.7.8 不用文案冒充真实 OS 隔离。没有平台强制隔离就继续报告 `os_isolation=false`。
- v1.7.8 不把 `danger-full-access` 弱化成普通 allowed；它必须是可见的 `policy_bypass`。
- v1.7.8 不在执行入口未纳入 policy evaluation 的情况下新增工具能力。
- v1.7.8 不把 CJK TestBackend 文本化空格当成 P0/P1，除非真实 Windows Terminal 复现同样问题。

## 开发主线一：Sandbox JSONL Schema Freeze

### 1. 机器字段与人类字段分离

修改：

- `D:\YunXi Agent\crates\yunxi-agent-sandbox\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-core\src\event.rs`
- `D:\YunXi Agent\crates\yunxi-agent-protocol\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\render.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\event_filter.rs`

职责：

- 为 `sandbox_attempt` 增加稳定机器字段：
  - `schema_version`: 当前固定为 `1`。
  - `backend_id`: 稳定 snake_case 枚举，例如 `windows_process_lifecycle`、`workspace_policy_guard`、`linux_landlock_entrypoint`、`direct_process_policy_bypass`。
  - `backend_label`: 面向人类的说明，例如 `policy guard + process lifecycle, no filesystem OS isolation`。
  - `runner`: 保留执行 runner 的稳定 id，例如 `windows_process_lifecycle`。
  - `os_isolation`: 继续保留布尔值，当前 Windows 默认必须为 `false`。
  - `unsupported_reason`: 当前无真实 OS isolation 时必须保留。
- 兼容现有 `backend` 字段：
  - v1.7.8 中继续输出 `backend`，但文档中标记为 compatibility/human-readable。
  - 新测试和新文档优先使用 `backend_id` 与 `backend_label`。
- 统一 `backend_id` 与 `runner` 的关系：
  - `backend_id` 描述 sandbox/backend policy 选择。
  - `runner` 描述实际执行 runner。
  - 当前 Windows 二者可以同值，但 schema 上不互相替代。

完成条件：

- `--jsonl` 中每个 `sandbox_attempt` 都有 `schema_version=1`、`backend_id`、`backend_label`、`runner`、`os_isolation`。
- 当前 Windows 默认路径继续输出 `os_isolation=false` 和 `unsupported_reason`。
- 文档不再鼓励机器消费者解析 `backend` 的人类字符串。

### 2. Enforcement 字段收口

修改：

- `D:\YunXi Agent\crates\yunxi-agent-sandbox\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-protocol\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\jsonl_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-exec\tests\sandbox_acceptance_tests.rs`

职责：

- 将 `enforcement` 定为 canonical machine field，值只允许：
  - `no_policy_guard`
  - `policy_only`
  - `process_lifecycle`
  - `os_restricted`
  - `policy_bypass`
- `enforcement_level` 保留一个兼容周期，值必须等于 `enforcement`，并在 README 或 protocol 文档中说明它是 compatibility alias。
- `danger-full-access` 路径必须稳定输出：
  - `enforcement=policy_bypass`
  - `enforcement_level=policy_bypass`
  - `backend_id=direct_process_policy_bypass`
  - `os_isolation=false`
- 当前 Windows workspace-write/read-only 默认路径必须稳定输出：
  - `enforcement=process_lifecycle`
  - `runner=windows_process_lifecycle`
  - `os_isolation=false`
  - `unsupported_reason` 非空

完成条件：

- acceptance tests 断言 canonical field，而不是只断言人类文案。
- JSONL fixture 可被外部脚本稳定解析，不依赖 `backend` 文本。

## 开发主线二：Execution Policy Invariant Audit

### 1. 建立统一执行入口清单

修改：

- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tools\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-exec\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-sandbox\src\lib.rs`
- `D:\YunXi Agent\docs\extraction-status.md`

职责：

- 明确所有能触发命令、文件写入、网络访问或子进程的入口：
  - shell command execution。
  - apply patch / patch-like file mutation。
  - MCP tool execution。
  - skill triggered tool execution。
  - multi-agent child execution。
  - provider/tool child scoped execution。
  - runtime synthetic/offline fixture execution。
- 每个入口都必须记录是否会进入 `ExecutionPolicy::evaluate()` 或等价集中 gate。
- 如果某入口当前尚未实现真实执行能力，文档中写清“当前无执行 surface”，并新增 guard test 防止后续偷偷绕过。

完成条件：

- 仓库中存在一份可维护的 execution-entry matrix，列出入口、文件、policy source、sandbox source、network source、事件输出。
- 没有新增直接 `Command::new` 或等价 spawn 路径绕过 `ExecManager` / `SandboxRunner`。

### 2. 集中 policy gate

修改：

- `D:\YunXi Agent\crates\yunxi-agent-exec\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tools\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-core\src\event.rs`

职责：

- 抽出或明确一个内部 gate，例如 `ExecutionGate` / `PolicyCheckedCommand` / `SandboxCheckedRequest`，避免调用者绕过 `policy.evaluate()` 后直接执行。
- `ExecCommand` 构造时必须携带 `ExecutionPolicy`，runner 执行前必须产出 `SandboxRunnerDiagnostic`。
- 所有工具调用事件必须能关联到同一个 policy evaluation result：
  - allowed。
  - denied。
  - escalation_required。
  - approval_required。
  - policy_bypass。
- 对 policy denied 或 escalation required 的路径，不允许 spawn 子进程。

完成条件：

- 代码层面能从执行入口追踪到 `ExecutionPolicy::evaluate()`。
- acceptance tests 能证明 policy denied 时没有实际写文件、没有执行网络命令、没有落地 workspace 外产物。

### 3. Bypass 可见性

修改：

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\render.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\event_filter.rs`
- `D:\YunXi Agent\crates\yunxi-agent-protocol\src\lib.rs`
- `D:\YunXi Agent\README.md`

职责：

- plain CLI、TUI event detail、JSON、JSONL、日志中的 danger-full-access 都必须显示 `policy_bypass`。
- 不允许只显示 `allowed` 而丢掉 bypass 语义。
- TUI 可以压缩显示，但 details 中必须能看到：
  - `enforcement=policy_bypass`
  - `backend_id=direct_process_policy_bypass`
  - `os_isolation=false`

完成条件：

- `danger_full_access_runs_but_reports_policy_bypass` 继续通过，并增加 CLI/JSONL/TUI 侧断言。
- README 的 safety boundary 描述与运行时事件一致。

## 开发主线三：Sandbox Acceptance Tests 扩展

### 1. JSONL schema acceptance

修改：

- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\jsonl_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-exec\tests\sandbox_acceptance_tests.rs`

职责：

- 新增 JSONL fixture，运行 offline agent 执行路径并收集 `sandbox_attempt`。
- 断言每个 `sandbox_attempt` 至少包含：
  - `schema_version`
  - `backend_id`
  - `backend_label`
  - `enforcement`
  - `runner`
  - `os_isolation`
  - `unsupported_reason`，当 `os_isolation=false` 且 runner 不具备 OS 文件系统隔离时必须非空。
- 断言 `enforcement_level` 如果存在，必须等于 `enforcement`。

完成条件：

- schema 缺字段时测试失败。
- 人类文案变化不导致机器字段测试失败。

### 2. 执行入口 acceptance

修改：

- `D:\YunXi Agent\crates\yunxi-agent-tools\tests\tool_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\tests\runtime_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-exec\tests\sandbox_acceptance_tests.rs`

职责：

- 针对每类入口增加最低限度 acceptance：
  - shell：workspace 外写入被拒绝或升级。
  - patch-like mutation：不能写 workspace 外路径；如果当前没有 patch 执行入口，测试文档化当前无入口。
  - MCP：MCP tool request 不能绕过 policy；若当前 MCP 为 simulated/offline route，也要证明不会 spawn 未受控进程。
  - skill：skill 触发工具调用必须复用 runtime/tool policy；若当前只有事件模拟，测试断言无直接执行。
  - multi-agent child：child scoped execution 必须继承或收窄 parent policy，不能默认扩大到 danger-full-access。
- 所有测试使用临时目录，不写项目根目录。

完成条件：

- 新增或现有执行入口没有 acceptance test 时，不允许发布 v1.7.8。
- acceptance suite 能覆盖审核报告列出的 shell、patch、MCP、skill、multi-agent 风险面。

### 3. Network policy acceptance

修改：

- `D:\YunXi Agent\crates\yunxi-agent-sandbox\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-exec\tests\sandbox_acceptance_tests.rs`

职责：

- `NetworkPolicy::Disabled` 下，命令风险识别必须覆盖常见 Windows/Unix 网络命令：
  - `curl`
  - `Invoke-WebRequest`
  - `irm`
  - `wget`
  - `python -c` 中显式 URL 访问
  - `node -e` 中显式 URL 访问
- disabled network 下不得静默执行；必须产生 denied 或 network escalation。

完成条件：

- Windows 与 Unix 风险命令均有测试分支或平台条件。
- 网络测试不访问真实外网，只检查 policy evaluation。

## 开发主线四：文档与 UI 安全边界统一

### 1. README safety boundary 收口

修改：

- `D:\YunXi Agent\README.md`
- `D:\YunXi Agent\docs\extraction-status.md`
- 新增或修改：`D:\YunXi Agent\docs\protocol\sandbox-events.md`

职责：

- 明确当前状态：
  - YunXi Agent v1.7.8 默认不提供 OS-enforced filesystem isolation。
  - Windows 当前是 `policy guard + process lifecycle`。
  - `os_isolation=false` 表示没有真实 OS 文件系统隔离。
  - `danger-full-access` 是 `policy_bypass`。
- 文档化 `sandbox_attempt` schema：
  - 字段名。
  - 类型。
  - 稳定性。
  - 兼容字段。
  - 机器消费者应该使用的字段。

完成条件：

- README、protocol 文档、CLI/TUI 运行时文案一致。
- 不出现“真实沙箱”“OS isolation 已启用”“文件系统隔离已完成”等误导性描述。

### 2. Plain/TUI wording 收口

修改：

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\render.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\event_filter.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`

职责：

- plain CLI sandbox line 中保留：
  - enforcement。
  - runner。
  - os isolation。
  - unsupported reason 摘要。
- TUI 默认 event list 只展示简洁信息，但 details/debug 中必须保留完整机器字段。
- 不把 sandbox warning 做成刷屏内容；但任何 bypass 或 unsupported isolation 必须可见。

完成条件：

- TUI 快照不退化。
- 详细信息可定位到 `policy_bypass` 和 `os_isolation=false`。

## 开发主线五：版本、发布、日志

修改：

- `D:\YunXi Agent\Cargo.toml`
- `D:\YunXi Agent\Cargo.lock`
- `D:\YunXi Agent\README.md`
- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\docs\reports\2026-07-13-yunxi-agent-v1-7-8-sandbox-schema-execution-policy-hardening-development-report.md`
- `C:\Users\admin\Desktop\YunXi Agent开发日志.md`

职责：

- Cargo package/workspace version 推进到 `1.7.8`。
- CLI/TUI banner 统一到 `v1.7.8`。
- v1.7.8 完成后发布到 GitHub `master`，并创建不可变 annotated tag `v1.7.8`。
- 旧 tag 不删除、不移动。
- 发布和读写 GitHub 全部使用 REST API。
- 任务结束追加桌面开发日志，记录：
  - 本次读取的审核报告。
  - 新增/修改文件。
  - 统一验证命令与结果。
  - GitHub REST API 同步结果。
  - tag object 与 peeled target。
  - `cargo clean` 结果。

完成条件：

- 本地 `master`、本地 `origin/master`、GitHub API branch、`v1.7.8` tag peeled target 一致。
- `git status --short` 为空。
- `D:\YunXi Agent\target` 被清理。

## 分阶段实施建议

实现阶段继续遵守用户硬性约束：先按报告完整构建，不在构建中途做反复验证；构建完成后统一测试。

### Phase 1：Schema freeze

目标：

- 增加 `schema_version`、`backend_id`、`backend_label`。
- 明确 `enforcement` 为 canonical field。
- 保留兼容字段但不再让新测试依赖人类字符串。

### Phase 2：Policy invariant audit

目标：

- 建立 execution-entry matrix。
- 查清 shell、patch、MCP、skill、multi-agent、child scoped execution 的 policy 入口。
- 移除或封死任何绕过 `ExecutionPolicy::evaluate()` 的路径。

### Phase 3：Acceptance expansion

目标：

- 扩展 sandbox acceptance suite。
- 把 JSONL schema、policy bypass、network disabled、workspace escape、child scoped policy inheritance 变成测试。

### Phase 4：Docs and UI consistency

目标：

- README、protocol docs、plain CLI、TUI detail 对 sandbox 语义统一。
- 保留 v1.7.7 TUI/CLI 回归基线。

### Phase 5：Version, unified verification, release

目标：

- 版本推进到 `1.7.8`。
- 完成统一验证。
- REST API 发布 `master`。
- 创建 annotated tag `v1.7.8`。
- 写日志并清理构建产物。

## 统一验证计划

源码构建完成后统一运行：

- `cargo fmt`
- `cargo fmt -- --check`
- `cargo test -p yunxi-agent-sandbox -p yunxi-agent-tools -p yunxi-agent-exec -p yunxi-agent-runtime -p yunxi-agent-tui -p yunxi-agent-cli`
- `cargo test`
- `cargo check --workspace`
- `cargo build -p yunxi-agent-cli --release --bins`
- `target\release\yunxi.exe --version`，期望 `yunxi 1.7.8`
- `target\release\yunxi-agent-cli.exe --version`，期望 `yunxi 1.7.8`
- `target\release\yunxi.exe --offline "v1.7.8 offline smoke"`
- `target\release\yunxi.exe --offline --json "v1.7.8 json smoke"`
- `target\release\yunxi.exe --offline --jsonl "v1.7.8 jsonl smoke"`
- JSONL sandbox schema fixture：
  - `schema_version=1`
  - `backend_id` 非空
  - `backend_label` 非空
  - `enforcement` 非空
  - `runner` 非空
  - `os_isolation=false`
  - Windows 默认 `unsupported_reason` 非空
- danger-full-access fixture：
  - `enforcement=policy_bypass`
  - `backend_id=direct_process_policy_bypass`
  - `os_isolation=false`
- sandbox acceptance suite：
  - workspace 内 read-only write。
  - workspace 外 absolute write。
  - symlink escape。
  - disabled network。
  - danger-full-access bypass。
  - process lifecycle/cancellation。
  - JSONL schema。
- execution-entry acceptance：
  - shell execution。
  - patch-like mutation 或当前无入口 guard。
  - MCP route。
  - skill route。
  - multi-agent child route。
- `target\release\yunxi.exe --backend codex "hello codex"`，期望 exit code `2`。
- `target\release\yunxi.exe --help`，确认 backend 可选值仍不包含默认 Codex runtime。
- `target\release\yunxi.exe sessions list --jsonl`，期望明确拒绝。
- `target\release\yunxi.exe parity map --jsonl`，期望明确拒绝。
- TUI 58x18、58x22、80x22、100x30 视觉快照回归。
- TUI detail 中可见 `policy_bypass` 与 `os_isolation=false`。
- CJK/English Windows Terminal spot check，若真实终端无异常则记录为非阻塞通过。
- DeepSeek live stream smoke：从 `C:\Users\admin\Desktop\api.txt` 读取密钥，不打印密钥。
- DeepSeek live JSON smoke：同上。
- dependency scan：默认 CLI 不含 `codex-*`、`vendor/codex-rs`、`yunxi-agent-codex`。
- owned-source secret scan：排除 `.git`、`.codegraph`、`target`、`vendor`、`extracted`。
- `git diff --check`
- `codegraph sync "D:\YunXi Agent"`
- `codegraph status "D:\YunXi Agent"`
- `scripts\install\install-yunxi.ps1 -AddToPath -SkipBuild`
- PATH smoke：`yunxi --version`、`yunxi-agent-cli --version`、`yunxi --offline "installed path v1.7.8 smoke"`
- GitHub REST API 发布 `master` 与 annotated tag `v1.7.8`
- 本地 `master`、本地 `origin/master`、GitHub API branch、`v1.7.8` tag peeled target 一致性校验
- `cargo clean`
- `Test-Path D:\YunXi Agent\target`，期望 `False`

## 风险与处理

- 真实 OS filesystem sandbox 仍是平台安全工程，v1.7.8 不能虚报完成。当前版本通过 schema、policy gate、acceptance suite 和文案诚实性降低误解风险。
- `backend` 字段如果已有外部脚本依赖，直接改语义会破坏兼容。因此 v1.7.8 增加 `backend_id` / `backend_label`，保留 `backend` 兼容字段。
- `enforcement` 与 `enforcement_level` 同时存在会让 schema 继续臃肿，但直接删除可能破坏现有 JSON 消费者。v1.7.8 先明确 canonical field，并让兼容字段等值。
- execution-entry matrix 可能发现某些入口当前只是模拟事件，不具备真实执行能力。处理方式是写明当前无执行 surface，并补 guard test，不能为了测试而引入新执行路径。
- MCP、skill、多 agent 子任务如果后续引入真实子进程或网络能力，必须先经过本版定义的 policy gate，不能作为“内部能力”绕开。
- CJK 混排如果只在 TestBackend 文本化 buffer 中表现为空格，不作为 hard gate；如果真实 Windows Terminal 复现，再进入 TUI 专项修复。

## v1.7.8 完成后的预期状态

完成 v1.7.8 后，YunXi Agent 应达到：

- `sandbox_attempt` JSONL schema 具备稳定机器字段，外部脚本不需要解析人类文案。
- 当前 Windows 默认 runner 继续诚实报告 `os_isolation=false`、`process_lifecycle`、`windows_process_lifecycle` 和 `unsupported_reason`。
- 所有现有和声明中的执行入口都经过 policy gate，不能绕过 `ExecutionPolicy` / `SandboxRequirement` / `NetworkPolicy`。
- `danger-full-access` 在 plain CLI、TUI detail、JSON、JSONL、日志中始终是显式 `policy_bypass`。
- sandbox acceptance suite 覆盖审核报告列出的核心风险面。
- v1.7.7 已关闭的 TUI/CLI 回归不退化。
- 默认运行路径继续保持 YunXi 自主化，不依赖上游 Codex CLI 源码。
- 后续如果进入 1.8 新模块或真实 OS sandbox 专项，可以在 v1.7.8 稳定 schema 和 policy invariant 上继续推进。

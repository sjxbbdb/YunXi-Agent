# YunXi Agent v1.7.6 CLI/TUI And Sandbox Hardening Development Report

生成时间：2026-07-13 11:20:11 +08:00

## 背景

YunXi Agent v1.7.5 已完成 CLI 协议、JSON/JSONL 输出、MCP tool lifecycle、session summary、provider fallback warning、DeepSeek live smoke、PATH 安装与 GitHub REST API 发布。当前发布版本为 `yunxi 1.7.5`，远端 `master` 与 annotated tag `v1.7.5` 已同步，构建中间产物也已通过 `cargo clean` 清理。

用户提供了新的审核报告：

- `C:\Users\admin\Desktop\YunXi-Agent-CLI-TUI-审核报告-1.7.5-2026-07-13.md`

本报告面向 v1.7.6。目标是吸收审核报告指出的剩余缺陷，把 YunXi Agent 从“协议可用、TUI 已成型”推进到“CLI 表面更诚实、测试不污染工作区、TUI 窄屏和流式观感更像正式终端产品、sandbox 能力进入真实 OS 隔离落地阶段”。

## 审核报告结论复核

本轮已读取审核报告全文，并按仓库规则优先使用 CodeGraph 核对关键代码落点。审核报告中的主要判断与当前代码基本一致：

- `crates/yunxi-agent-cli/src/main.rs` 的 `run_codex_backend()` 仍是 detached placeholder，`--backend codex` 在默认 CLI 中仍像可选 backend 一样暴露，但实际运行失败。
- `crates/yunxi-agent-cli/tests/cli_tests.rs` 中部分 CLI 测试路径没有强制使用临时 `--cwd`，会在测试后留下 `crates/yunxi-agent-cli/.yunxi/` session 产物。
- `crates/yunxi-agent-tui/src/app.rs` 的 `header()` / `subheader()` 仍直接拼接完整字符串，没有宽度感知优先级，窄屏会硬截断到半个字段。
- `crates/yunxi-agent-tui/src/transcript_layout.rs` 的 `split_display_width()` 仍按字符宽度切分，不考虑英文 word boundary，也会在混合 CJK/English 时制造异常空格和断词。
- `crates/yunxi-agent-tui/src/bottom_pane.rs` 的 composer 空输入状态仍偏高，空白行对窄屏终端不够友好。
- `crates/yunxi-agent-sandbox/src/lib.rs` 已诚实暴露 `policy guard: advisory only, no OS isolation`，但 `crates/yunxi-agent-exec/src/lib.rs` 执行层仍是直接 `Command::new(...).spawn()`，真实 OS sandbox 还没有落地。

v1.7.6 因此不再继续扩大命令面，而是修正现有能力的体验、安全边界和工程卫生。

## v1.7.6 总目标

YunXi Agent v1.7.6 的核心目标是修复 1.7.5 审核报告中的高优先级问题，并为真实平台 sandbox runner 打下第一版可验证基础。

成功标准：

- `cargo test` 后不再生成或遗留 `crates/yunxi-agent-cli/.yunxi/`。
- 默认 CLI 不再把 detached Codex backend 表现成可用能力；`--backend codex` 要么隐藏，要么在 parse/preflight 阶段以 invalid input 拒绝。
- TUI header/subheader/footer 具备宽度感知分档，不在窄屏留下半截 token 或悬空 `|`。
- Transcript wrapping 改为 word-aware；普通英文单词不被拆成 `To` / `ol` 这类断裂；CJK 连续字符不被插入异常空格。
- Composer 空 buffer 更紧凑，窄屏可用空间优先留给 transcript。
- Approval 面板开始显示 command risk label，并让长命令的视觉区域更稳定。
- sandbox 不再停留在只有 label 的 facade：引入 platform runner trait 与执行层选路；Windows 至少推进到 process lifecycle containment 或 feature-gated restricted-token prototype；未达到文件系统强隔离的平台必须继续明确 `policy_guard`、`partial` 或 `unsupported`。
- 默认运行路径继续不依赖上游 Codex CLI 源码、`vendor/codex-rs`、`codex-*` 或 `yunxi-agent-codex`。

## 用户硬性约束

后续实现阶段必须继续遵守以下约束：

- 先完整构建 v1.7.6 源码，中间不做反复单点测试或在某一个点卡太久。
- 源码迁移/构建完成后统一执行验证。
- 每个正式版本必须创建新的不可变 annotated tag，旧 tag 不删除、不移动。
- GitHub 读写和推送只走 REST API，不使用 `git push`、`git fetch`、`git ls-remote`。
- API 密钥只从 `C:\Users\admin\Desktop\api.txt` 读取，不打印、不写日志、不提交。
- 每次任务结束必须追加 `C:\Users\admin\Desktop\YunXi Agent开发日志.md`。
- 发布收尾前执行 `cargo clean` 清理构建中间产物。

## 非目标

- v1.7.6 不重新依赖上游 Codex CLI 源码，不把 detached Codex compatibility crate 合回默认 CLI。
- v1.7.6 不新增桌面端、云任务、插件市场、自动更新器等新产品面。
- v1.7.6 不以“看起来像 sandbox”的文案代替真实 OS 隔离；如果某平台 runner 未达到强隔离，必须继续标记为 `policy_guard` 或 `os_isolation=false`。
- v1.7.6 不牺牲 JSON/JSONL 协议兼容性；TUI 改动不能破坏 plain CLI、JSON、JSONL、session 管理和 DeepSeek live provider。

## 开发主线一：CLI 表面与测试隔离

### 1. 测试不污染工作区

修改：

- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`

职责：

- 所有会创建 session store 的 CLI integration tests 都必须显式使用 `TempDir`。
- `run` 子命令、`--` escaped prompt、reserved prompt fallback 等路径统一添加 `--cwd <temp>`。
- 如果测试需要读取 session list，只读取临时目录下的 `.yunxi`。

完成条件：

- `cargo test -p yunxi-agent-cli` 后 `crates/yunxi-agent-cli/.yunxi/` 不存在。
- `cargo test` 后 `git status --short` 不出现测试生成的 `.yunxi/`。

### 2. Codex backend placeholder 收敛

修改：

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`
- `D:\YunXi Agent\README.md`
- `D:\YunXi Agent\docs\extraction-status.md`

职责：

- 默认 CLI help 不再把 `codex` 展示成普通可用 backend。
- `--backend codex` 在 default build 中进入 parse/preflight invalid input，退出码为 `2`，错误信息直接说明：默认 CLI 不包含 Codex compatibility backend。
- 保留将来 feature-gated compatibility crate 的文档入口，但不能让用户误以为 v1.7.6 默认可直接运行上游 Codex backend。
- `--live` 文案继续指向 live model provider，不与 Codex backend 混淆。

完成条件：

- `yunxi --help` 不把 `codex` 列为普通 backend。
- `yunxi --backend codex "hello"` 返回 invalid input，不打印 provider fallback warning。
- dependency scan 仍确认默认 CLI 不依赖 `codex-*`、`vendor/codex-rs`、`yunxi-agent-codex`。

### 3. 保留词 prompt 体验定稿

修改：

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`
- `D:\YunXi Agent\README.md`

职责：

- 保留 `yunxi run <prompt>` 作为推荐 one-shot 入口。
- `yunxi -- sessions` 继续可用。
- 根命令 `yunxi --offline sessions` 在当前版本可以继续拒绝，但错误提示必须稳定、简短、包含两个可用替代命令。
- 暂不把缺少合法子命令的 `sessions` 自动降级为 prompt，避免破坏现有 metadata 命令解析。

完成条件：

- reserved prompt 错误提示可读且测试固定。
- 文档把 `yunxi run <prompt>` 写成推荐入口。

## 开发主线二：TUI 窄屏、换行与流式观感

### 1. Header/subheader 宽度感知压缩

修改：

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\tests\*.rs` 或现有 TUI snapshot/test backend 测试文件

职责：

- 将 `header()` / `subheader()` 从无参字符串改为宽度感知渲染，例如 `header_for_width(width)`、`subheader_for_width(width)`。
- 宽屏保留完整信息：版本、cwd、provider、mode、model。
- 中屏压缩 cwd：使用 basename 或 middle ellipsis，例如 `D:/.../yunxi-agent-cli`。
- 窄屏只保留最关键状态，例如 `YunXi v1.7.6 | offline | static`。
- subheader 窄屏只保留 `tail/history/new output below`、`cells=N`、`debug off/on`，不留下半截字段或 dangling separator。

完成条件：

- 58 列宽 header/subheader 不出现半截 token。
- subheader 不再截断成 `| d`、`|` 之类残片。

### 2. Transcript word-aware wrapping

修改：

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\transcript_layout.rs`

职责：

- 用 word-aware wrapping 替换纯字符级 `split_display_width()`。
- 对 ASCII/English 先按 whitespace 和 word boundary 切行。
- 对超长 token 才回退字符级切分，避免无限超宽。
- 对 CJK 连续文本按 Unicode width 累积，不在汉字之间插入额外空格。
- continuation gutter 从 `  | ` 调整为更安静的缩进，例如四空格或 dimmed two-space gutter，减少“文本被管道切开”的错觉。

完成条件：

- `Tool` 不会显示成 `To` + continuation + `ol`。
- `output` 不会显示成 `ou` + continuation + `tput`，除非宽度小到无法容纳一个普通 token。
- `中文 and English` 不显示成 `中 文  and`。
- 多行内容仍保留 continuation 视觉层次。

### 3. Footer 分档与 composer 紧凑化

修改：

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\bottom_pane.rs`

职责：

- footer 根据宽度分档：
  - 宽屏：`Enter submit | Alt+Enter newline | /help commands | wheel/drag scroll | Ctrl+C exit`
  - 中屏：`Enter submit | /help | wheel scroll | Ctrl+C exit`
  - 窄屏：`Enter | /help | Ctrl+C`
- scroll/history 状态下优先显示 `new output below` 或 `history view`，减少重复提示。
- composer 空 buffer 时高度压到 4 行：border + prompt + footer + border。
- 多行输入时再按输入行数扩展高度。

完成条件：

- 空 composer 不再出现无意义空白行。
- 58 列宽 footer 不换出难读的长句。

### 4. Approval 面板风险提示

修改：

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\bottom_pane.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`
- `D:\YunXi Agent\crates\yunxi-agent-core\src\event.rs` 或 approval view model 所在文件

职责：

- Approval request view 增加 `risk_label` 或由 command/reason 推导简短风险文案。
- 常见标签：
  - `risk: reads workspace`
  - `risk: writes workspace`
  - `risk: network`
  - `risk: destructive`
- command 区域采用稳定缩进，长命令 wrap 后保持对齐。
- 对 destructive 风险是否默认 Decline 作为实现时安全决策；如果改默认行为，必须在测试和文档中说明。

完成条件：

- Approval 面板能区分普通命令和高风险命令。
- 长命令不会把 approve/decline 操作区挤乱。

## 开发主线三：真实 Sandbox Runner 深化

### 1. Platform runner trait

修改：

- `D:\YunXi Agent\crates\yunxi-agent-sandbox\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-exec\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tools\src\lib.rs`

职责：

- 定义 platform sandbox runner trait，明确输入包括 program、argv、cwd、env、workspace roots、sandbox policy、network policy。
- 输出必须包括 exit status、stdout、stderr、diagnostic。
- diagnostic 必须保留：
  - `sandbox_requested`
  - `sandbox`
  - `os_isolation`
  - `enforcement`
  - `runner`
  - `unsupported_reason`
- 当前 policy-only path 继续返回 `os_isolation=false`、`enforcement=policy_guard`。

完成条件：

- 执行层不再只有直接 `Command::new(...).spawn()` 的概念入口，而是通过 runner 选择。
- 未支持平台不会伪装强隔离。

### 2. Windows runner 第一阶段

修改：

- `D:\YunXi Agent\crates\yunxi-agent-exec\src\lib.rs`
- 可能新增：`D:\YunXi Agent\crates\yunxi-agent-exec\src\windows_sandbox.rs`

职责：

- 在 Windows 上优先实现 job object/process lifecycle containment，至少保证进程树可被约束和清理。
- 如果受当前权限限制无法完成 restricted token，必须在 diagnostic 中明确 `os_isolation=false` 或 `os_isolation=partial`，不能标记为强隔离。
- restricted token / low integrity / deny write outside workspace 可作为 feature-gated prototype 进入同一版，但不把未完成能力写成已完成。

完成条件：

- Windows runner 有独立测试覆盖 runner 选择和 diagnostic。
- 如果只完成 job object lifecycle containment，报告和 JSONL 必须写清它不是完整文件系统隔离。

### 3. Linux runner 第一阶段

修改：

- `D:\YunXi Agent\crates\yunxi-agent-exec\src\lib.rs`
- 可能新增：`D:\YunXi Agent\crates\yunxi-agent-exec\src\linux_sandbox.rs`

职责：

- 预留 Linux Landlock runner 入口。
- 如果当前 Windows 开发环境无法验证 Linux Landlock，只提交接口和 feature-gated implementation skeleton，不声称 Linux 强隔离已通过。

完成条件：

- Linux 未验证时 diagnostic 明确 `unsupported` 或 `feature_disabled`。
- 不影响 Windows 和普通 policy guard 执行。

### 4. Sandbox escape E2E 计划

修改：

- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\*.rs`
- `D:\YunXi Agent\crates\yunxi-agent-exec\tests\*.rs`

职责：

- 增加策略绕过/逃逸场景测试规划：
  - 试图写 workspace 外路径。
  - 试图通过 shell 子进程写外部路径。
  - 试图创建长生命周期子进程。
- 对当前 policy-only runner，测试应断言 diagnostic 诚实，而不是误判 OS 层已拒绝。
- 对真实 runner feature，测试应断言 OS 层拒绝或进程树被清理。

完成条件：

- v1.7.6 至少具备可运行的 diagnostic 测试和 Windows lifecycle containment 测试。
- 完整文件系统强隔离如果未完成，必须进入后续 v1.7.7/v1.8 报告，不在 v1.7.6 虚报。

## 分阶段实施建议

实现阶段继续遵守用户硬性约束：先按报告完整构建，不在构建中途做反复验证；构建完成后统一测试。

### Phase 1：CLI 工程卫生

目标：

- 修掉 `.yunxi/` 测试污染。
- 收敛 detached Codex backend。
- 固定 reserved prompt 推荐入口。

改动文件：

- `crates/yunxi-agent-cli/src/main.rs`
- `crates/yunxi-agent-cli/tests/cli_tests.rs`
- `README.md`
- `docs/extraction-status.md`

完成条件：

- CLI tests 使用临时 cwd。
- `--backend codex` 默认不可误导。

### Phase 2：TUI 宽度感知与 wrapping

目标：

- Header/subheader/footer 分档。
- Transcript word-aware wrapping。
- Composer 空状态压缩。

改动文件：

- `crates/yunxi-agent-tui/src/app.rs`
- `crates/yunxi-agent-tui/src/transcript_layout.rs`
- `crates/yunxi-agent-tui/src/bottom_pane.rs`
- TUI tests/snapshots

完成条件：

- 58x20、80x22、100x30 三类缓冲视觉测试通过。
- 不出现审核报告中的 `To` / `| ol`、`中 文  and`、subheader 残片。

### Phase 3：Approval 风险提示

目标：

- Approval 面板对命令风险有明显但克制的标注。
- 长命令区域 wrap 对齐。

改动文件：

- `crates/yunxi-agent-tui/src/bottom_pane.rs`
- approval view model 相关文件
- TUI tests/snapshots

完成条件：

- 常见风险标签可见。
- 操作按钮区域稳定。

### Phase 4：Sandbox runner 深化一期

目标：

- 增加 platform runner trait。
- Windows runner 至少完成 process lifecycle containment 或明确 feature-gated restricted token prototype。
- Linux Landlock runner 入口存在但未验证能力不夸大。

改动文件：

- `crates/yunxi-agent-sandbox/src/lib.rs`
- `crates/yunxi-agent-exec/src/lib.rs`
- 可能新增平台 runner 文件
- `crates/yunxi-agent-tools/src/lib.rs`
- sandbox/exec tests

完成条件：

- diagnostic 能区分 `policy_guard`、`process_containment`、`os_isolation`、`unsupported`。
- JSONL/TUI/plain output 不误导用户。

### Phase 5：版本、文档、日志、发布

目标：

- 版本推进到 `1.7.6`。
- README、extraction status、开发报告、桌面日志同步。
- 统一验证后安装、发布、创建 immutable annotated tag `v1.7.6`。

改动文件：

- `Cargo.toml`
- `Cargo.lock`
- `README.md`
- `docs/extraction-status.md`
- `docs/reports/2026-07-13-yunxi-agent-v1-7-6-cli-tui-sandbox-hardening-development-report.md`
- `C:\Users\admin\Desktop\YunXi Agent开发日志.md`

发布要求：

- GitHub 远端更新只走 REST API。
- 创建 `v1.7.6` annotated tag，不删除、不移动旧 tag。
- 发布后执行 `cargo clean`。

## 统一验证计划

源码构建完成后统一运行：

- `cargo fmt`
- `cargo fmt -- --check`
- `cargo test -p yunxi-agent-cli -p yunxi-agent-tui -p yunxi-agent-exec -p yunxi-agent-sandbox`
- `cargo test`
- `cargo check --workspace`
- `cargo build -p yunxi-agent-cli --release --bins`
- `target\release\yunxi.exe --version`，期望 `yunxi 1.7.6`
- `target\release\yunxi-agent-cli.exe --version`，期望 `yunxi 1.7.6`
- `target\release\yunxi.exe --offline "v1.7.6 offline smoke"`
- `target\release\yunxi.exe --offline --json "v1.7.6 json smoke"`
- `target\release\yunxi.exe --offline --jsonl "v1.7.6 jsonl smoke"`
- `target\release\yunxi.exe --backend codex "hello codex"`，期望 exit code `2` 或默认 help 隐藏后的明确 invalid input。
- `target\release\yunxi.exe --help`，确认默认 help 不把 Codex backend 表现成可用 runtime。
- `target\release\yunxi.exe --cwd <temp> --offline run sessions`，确认不污染 crate dir。
- `target\release\yunxi.exe --cwd <temp> --offline -- sessions`，确认不污染 crate dir。
- `Test-Path D:\YunXi Agent\crates\yunxi-agent-cli\.yunxi`，期望 `False`。
- TUI TestBackend 58x20 snapshot：header/subheader/footer 不出现半截 token。
- TUI TestBackend 100x30 snapshot：普通英文单词不被拆裂。
- TUI TestBackend CJK/English snapshot：`中文 and English` 不被显示成 `中 文  and`。
- TUI composer 空 buffer 高度断言。
- Approval 面板 risk label snapshot。
- sandbox diagnostic tests：policy-only path 仍为 `os_isolation=false`、`enforcement=policy_guard`。
- Windows runner diagnostic tests：确认 runner label 和 containment 语义。
- Stage 4M fixture JSONL：tool lifecycle、sandbox diagnostic、MCP completed 继续通过。
- DeepSeek live stream smoke：从 `C:\Users\admin\Desktop\api.txt` 读取密钥，不打印密钥。
- DeepSeek live non-stream smoke：同上。
- dependency scan：默认 CLI 不含 `codex-*`、`vendor/codex-rs`、`yunxi-agent-codex`。
- owned-source secret scan：排除 `.git`、`.codegraph`、`target`、`vendor`、`extracted`。
- `git diff --check`
- `codegraph sync "D:\YunXi Agent"`
- `codegraph status "D:\YunXi Agent"`
- `scripts\install\install-yunxi.ps1 -AddToPath -SkipBuild`
- PATH smoke：`yunxi --version`、`yunxi-agent-cli --version`、`yunxi --offline "installed path v1.7.6 smoke"`
- GitHub REST API 发布 `master` 与 annotated tag `v1.7.6`
- 本地 `master`、本地 `origin/master`、GitHub API branch、`v1.7.6` tag peeled target 一致性校验
- `cargo clean`
- `Test-Path D:\YunXi Agent\target`，期望 `False`

## 风险与处理

- 真实 sandbox 是平台安全工程，不能用文案伪装完成。v1.7.6 必须把 runner 能力和 diagnostic 分开：支持多少说多少。
- Windows restricted token 可能需要较细的 Win32 API 封装。如果实现成本过高，本版先完成 job object lifecycle containment 和 runner trait，但必须保留 `os_isolation=false` 或 `partial`。
- Header/subheader API 从无参变宽度感知可能影响 TUI render 调用点，必须集中改动并覆盖 TestBackend。
- word-aware wrapping 不能破坏 CJK 宽度计算；超长 token 仍需要字符级 fallback。
- Codex backend 从 help 中隐藏可能影响极少数知道 placeholder 的开发者；默认用户体验优先，compatibility crate 文档保留即可。
- 测试隔离必须避免把临时目录路径写入 snapshot，防止 Windows 路径差异导致测试不稳定。

## v1.7.6 完成后的预期状态

完成 v1.7.6 后，YunXi Agent 应达到：

- CLI 默认命令面更诚实，不再暴露看似可用但必失败的 Codex backend。
- 测试不会污染源码工作区。
- TUI 在窄屏、历史滚动、流式输出、CJK/English 混排下观感明显改善。
- Approval 面板开始具备风险识别和更稳定的命令展示。
- sandbox 进入真实 platform runner 深化阶段，并继续保持安全边界表述诚实。
- 默认运行路径继续保持 YunXi 自主化，不依赖上游 Codex CLI 源码。

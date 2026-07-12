# YunXi Agent v1.5 Honesty Streaming Safety Development Report

生成时间：2026-07-12 14:40:44 +08:00

## 背景

当前基线版本为 `v1.4.0`。审核报告对 `D:\YunXi Agent` 的 provider、runtime、tools/exec/sandbox、MCP/skills/multi-agent/context/storage、CLI/REPL 五大子系统做了源码级检查，结论是：YunXi Agent 的最小可用地基已经真实存在，可以在终端中对话、调用工具、执行 shell、写会话、接 MCP；但部分对标 Codex CLI / Claude Code CLI 的叙事混入了 fixture 表演、离线罐头输出和误导性的沙箱命名。

v1.5 不继续堆新命令面，也不把问题稀释到完整 TUI、doctor、completion、云任务等外围功能。本版目标是优先修掉审核报告中最容易误导用户和影响真实终端 Agent 体验的问题：离线诚实披露、安全命名、fixture 隔离、真实流式和 provider retry 韧性。

## 审核问题归纳

### 必须优先处理

- `S-2`：离线默认模式把 `StaticProvider` 罐头文本显示成普通模型回答，`Auto` 无 key 时静默降级，`/cost` 对离线模式没有明确说明。
- `S-1`：`SandboxBackend::WindowsRestrictedToken` / `LinuxLandlock` 等命名暗示 OS 级隔离，但当前实现实际是策略评估和审批建议，不是操作系统级沙箱。
- `S-3`：`stage 4k/4l/4m` fixture 与产品 runtime 混在一起，魔法 prompt 可绕过主链并吐出硬编码事件，容易让使用者误判能力范围。
- `M-1`：live streaming 不是端到端真流式，现有路径先下载完整响应体再回放 SSE 事件。
- `M-5`：retry 无退避、不读 `Retry-After`，传输层网络错误/超时没有纳入重试预算。

### 本版明确记录但不一次性展开

- `M-2/M-3`：REPL 行编辑、历史、补全、多行、空闲 Ctrl+C 行为，留到 v1.6。
- `M-4/D-4`：apply_patch hunk 行锚定和 symlink 逃逸防护，留到 v1.6 或 v1.7。
- `D-1/D-2/D-3`：跨平台 CI、长命令路径、进程组 kill，留到后续执行层专项。
- `D-5/D-6/D-8/D-9/D-10`：真 tokenizer、默认 child provider 继承、MCP 长生命周期、真实 view_image、tool_search 内容索引，作为后续 parity 队列继续推进。

## v1.5 目标

- 将版本升级到 `1.5.0`，发布新 tag `v1.5.0`，旧 tag 不删除、不移动。
- 离线输出必须逐轮 inline 标注，用户看到的 assistant 文本不能再和 live 模型回答混淆。
- `Auto` 模式因未找到凭据而降级到离线时，CLI 必须显示显式警告。
- `/cost` 在离线模式下显示 `n/a - offline, no model call` 或等价中文说明，不再只写 `unavailable`。
- 沙箱相关 CLI/文档/事件展示改成诚实命名：当前能力是 policy guard / policy advisor，不承诺 OS-level sandbox。
- `stage 4x` fixture 默认不再由普通 prompt 触发；仅在测试或显式 fixture 开关中可用，并在事件中标注 `fixture_mode=true`。
- live provider 改为基于 `reqwest::Response::bytes_stream()` 的真实网络分片读取，并把 SSE delta 在读取过程中推送到 runtime event sink。
- provider retry 增加指数退避、`Retry-After` 解析，以及网络/超时错误重试。
- 保持默认 `yunxi` 运行链路不依赖 `vendor/codex-rs`、`codex-*` 或 `yunxi-agent-codex`。
- API key、GitHub PAT 和 credential 文件内容不打印、不写日志、不提交；DeepSeek/GitHub 测试只记录脱敏结果。

## 非目标

- v1.5 不实现完整 Windows Restricted Token、Linux Landlock、seccomp、namespace 或 macOS seatbelt。真实 OS 沙箱是单独大版本工程；本版先消除误导命名和文档承诺。
- v1.5 不引入 TUI、desktop app、cloud task、SDK packaging、doctor、completion、marketplace。
- v1.5 不重写全部 patch 引擎，不做 hunk anchor 深化。
- v1.5 不做 MCP 长生命周期复用，不做 view_image 解码，不做 tool_search 内容级检索。
- 构建过程中不做零散测试；源码构建完成后统一验证。
- GitHub 远端读取、写入、tag 发布和核验继续全部走 REST API，不使用 `git push`、`git fetch`、`git ls-remote`。

## 设计方案

### 1. 离线诚实披露

新增统一的 provider disclosure 状态，CLI banner、每轮 message 渲染、`/status` 和 `/cost` 使用同一份状态，不各自拼字符串。

预期行为：

- forced offline：banner 显示 `provider_mode: offline`，assistant 文本前缀显示 `[offline]`。
- auto fallback：如果 `provider_mode=auto` 且没有解析到 live credential，启动时输出一条警告，说明当前回答来自离线 runtime，不是模型。
- `/cost`：离线模式显示 `n/a - offline, no model call`，live 模式继续显示最近一轮和累计 usage。

主要文件：

- `crates/yunxi-agent-cli/src/interactive.rs`
- `crates/yunxi-agent-cli/src/render.rs`
- `crates/yunxi-agent-cli/src/commands.rs`
- `crates/yunxi-agent-cli/tests/cli_tests.rs`

### 2. 沙箱能力诚实命名

保留现有策略评估逻辑，但对外展示从“沙箱后端”改成“策略保护/策略建议”。内部 enum 可以通过兼容 alias 保留旧序列化值，避免破坏已有 session JSON；CLI 和文档不能再把当前能力描述成 OS 级隔离。

预期行为：

- `WorkspaceWrite` 和 `ReadOnly` 的评估结果仍可触发审批/escalation。
- 对外展示说明当前执行仍以父进程权限运行，策略只负责分类、拦截和审批，不是 restricted token / landlock enforcement。
- `DangerFullAccess` 继续明确显示为 bypass policy guard。

主要文件：

- `crates/yunxi-agent-sandbox/src/lib.rs`
- `crates/yunxi-agent-cli/src/render.rs`
- `README.md`
- `docs/extraction-status.md`

### 3. Runtime fixture 隔离

把 `stage 4k/4l/4m` 魔法 prompt 从默认产品路径隔离出来。推荐实现为一个小型 fixture policy：

- `RuntimeFixturePolicy::Disabled`：默认产品运行路径，不识别 stage 魔法 prompt。
- `RuntimeFixturePolicy::Explicit`：仅当测试或显式环境变量/测试构造器开启时识别。
- fixture 事件必须带 `fixture_mode=true`，并在最终 response 中包含离线/fixture 标注。

这样保留历史测试资产，但用户正常对话不会再被魔法字符串带入硬编码事件流。

主要文件：

- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-runtime/tests/runtime_tests.rs`
- `crates/yunxi-agent-cli/tests/jsonl_tests.rs`
- `README.md`
- `docs/extraction-status.md`

### 4. 真实 live streaming

现有 `OpenAiStreamAccumulator` 和 SSE decoder 可以继续复用，但输入来源必须从整包 `response.text().await` 改为网络分片 `bytes_stream()`。同时 runtime 需要在 provider 分片到达时即时 emit `StreamEvent`，不能等完整 `ProviderStream` 收集完再统一回放。

推荐接口方向：

- 在 provider 层新增 live stream 方法，返回或驱动一个异步事件流。
- 对 `ReqwestProviderTransport` 增加 streaming transport 路径，读取 status、headers 和 byte chunks。
- `OpenAiStreamAccumulator` 增加“push chunk 后 drain newly completed events”的接口，避免只能在 finish 后一次性取出。
- runtime provider turn 在事件到达时立即调用 `emit_provider_stream_events` 的单事件版本，并继续累积最终 `ProviderResponse`。

主要文件：

- `crates/yunxi-agent-provider/src/lib.rs`
- `crates/yunxi-agent-provider/tests/provider_tests.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-runtime/tests/runtime_tests.rs`
- `crates/yunxi-agent-cli/src/interactive.rs`
- `crates/yunxi-agent-cli/tests/cli_tests.rs`

### 5. Provider retry 韧性

`send_with_retries` 需要从紧循环改成可观察、可控制的重试策略：

- 对 HTTP 429、5xx、可配置 retry status 使用指数退避。
- 读取 `Retry-After` 秒数或 HTTP-date，优先遵守 provider 返回的等待时间，并设置合理上限。
- 对 reqwest timeout/network error 纳入 retry budget。
- retry 事件不打印 header 中的敏感值，不泄露 authorization。

主要文件：

- `crates/yunxi-agent-provider/src/lib.rs`
- `crates/yunxi-agent-provider/tests/provider_tests.rs`

## 文件范围

- `Cargo.toml`
- `Cargo.lock`
- `crates/yunxi-agent-cli/Cargo.toml`
- `crates/yunxi-agent-cli/src/commands.rs`
- `crates/yunxi-agent-cli/src/interactive.rs`
- `crates/yunxi-agent-cli/src/render.rs`
- `crates/yunxi-agent-cli/tests/cli_tests.rs`
- `crates/yunxi-agent-cli/tests/jsonl_tests.rs`
- `crates/yunxi-agent-provider/src/lib.rs`
- `crates/yunxi-agent-provider/tests/provider_tests.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-runtime/tests/runtime_tests.rs`
- `crates/yunxi-agent-sandbox/src/lib.rs`
- `crates/yunxi-agent-sandbox/tests`（如需新增集成测试）
- `README.md`
- `docs/extraction-status.md`
- `docs/reports/2026-07-12-yunxi-agent-v1-5-honesty-streaming-safety-development-report.md`
- `docs/superpowers/plans/2026-07-12-yunxi-agent-v1-5-honesty-streaming-safety.md`
- `C:\Users\admin\Desktop\YunXi Agent开发日志.md`

## 开发阶段划分

### Phase 1：版本与离线诚实披露

- 版本升级到 `1.5.0`。
- 统一 provider disclosure 状态。
- banner、message、`/status`、`/cost` 使用同一判断。
- Auto fallback 显式警告。

### Phase 2：沙箱对外命名修正

- 对外展示改为 policy guard / policy advisor。
- 文档写清当前不提供 OS-level sandbox enforcement。
- 保留现有策略评估和审批行为，不扩大执行层改动面。

### Phase 3：fixture 默认隔离

- 增加 runtime fixture policy。
- 默认产品路径禁用 `stage 4x` 魔法 prompt。
- 测试路径显式启用 fixture。
- fixture event/response 必须可识别。

### Phase 4：真实 streaming provider

- `ReqwestProviderTransport` 增加 `bytes_stream()` 路径。
- provider parser 按 chunk 增量解码 SSE。
- runtime 在 provider chunk 到达时即时 emit。
- CLI interactive 继续消费实时事件，不回退到整包回放。

### Phase 5：retry/backoff

- 扩展 `ProviderTransportResponse` 保存 headers。
- retry 读取 `Retry-After`。
- HTTP retry 和 transport retry 统一走 budget。
- 增加退避上限，避免无限等待或紧循环。

### Phase 6：统一验证、发布、清理

- 所有构建完成后统一验证。
- 安装 `yunxi 1.5.0` 到 PATH。
- 使用 DeepSeek API 做 live smoke，输出脱敏。
- 使用 GitHub REST API 发布 `master` 和 `v1.5.0`。
- 更新 CodeGraph。
- 同步桌面开发日志。
- 执行 `cargo clean` 并确认 `target_exists=False`。

## 统一验证门

构建完成后统一执行：

- `cargo fmt`
- `cargo fmt -- --check`
- `cargo test`
- `cargo check --workspace`
- `cargo build -p yunxi-agent-cli --release --bins`
- `target\release\yunxi.exe --version` 输出 `yunxi 1.5.0`
- `target\release\yunxi-agent-cli.exe --version` 输出 `yunxi 1.5.0`
- forced offline one-shot：assistant 文本必须包含 `[offline]` 或等价标记
- forced offline interactive：banner、首轮回答、`/cost` 均显示离线无模型调用
- auto fallback 无 key smoke：必须出现显式降级警告
- fixture disabled smoke：普通 prompt 包含 `stage 4m real parity fixture` 时不得触发 stage fixture
- fixture explicit smoke：显式开启 fixture 后，stage fixture 可运行且事件标注 `fixture_mode=true`
- live DeepSeek non-stream smoke：能返回真实模型结果，脱敏扫描通过
- live DeepSeek streaming smoke：首个 provider delta 在完整响应结束前到达 CLI/runtime 事件流
- retry fixture：429 + `Retry-After` 会等待并重试；网络/timeout 错误纳入 retry budget
- sandbox display smoke：CLI/JSONL 不再展示误导性的 OS sandbox enforcement 文案
- default CLI dependency scan：`codex-*`、`vendor/codex-rs`、`yunxi-agent-codex` 命中 0
- owned-source secret scan：API key、Bearer、GitHub PAT 模式命中 0
- `git diff --check`
- `codegraph sync "D:\YunXi Agent"`
- release install 到 `C:\Users\admin\AppData\Local\YunXi Agent\bin`
- PATH `yunxi --version` 输出 `yunxi 1.5.0`
- GitHub REST API 发布并回读核验 `master` 和 `v1.5.0`
- 旧 tag `v1.0.0` 到 `v1.4.0` 不删除、不移动
- `cargo clean`
- `target_exists=False`

## 风险与控制

- 风险：真实 streaming 改动 provider/runtime 边界，可能影响 one-shot 和 JSONL。
  控制：保留旧的 collected stream 适配，新增增量路径先由 live stream 使用；one-shot 仍可通过事件收集器汇总。

- 风险：fixture 隔离会影响既有 stage 测试。
  控制：测试构造器或环境变量显式开启 fixture；默认产品路径关闭。

- 风险：沙箱 enum 重命名破坏已存 session JSON。
  控制：内部结构兼容旧序列化值；只修改对外展示和文档，必要时增加 serde alias。

- 风险：retry backoff 让测试变慢。
  控制：retry policy 支持测试注入零延迟 sleeper；产品路径使用真实 sleep。

- 风险：live API 测试受网络和模型不稳定影响。
  控制：live smoke 只断言事件形状、退出码、脱敏和最小 marker，不依赖长文本内容。

## 发布策略

- 下一版本号：`v1.5.0`。
- 创建新 annotated tag `v1.5.0`；旧 tag 不删除、不移动。
- GitHub 读写、tag 发布、远端核验全部走 REST API。
- 不使用 `git push`、`git fetch`、`git ls-remote`。
- API key/PAT 不打印、不写日志、不写源码、不提交。
- 发布后同步本地 tracking ref，保持主目录与远端 `master` 一致。

## 成功判定

v1.5.0 完成后，YunXi Agent 仍是独立运行的 CLI Agent，但用户不再会把离线罐头输出误认为真实模型回答，也不会从沙箱命名中误判存在 OS 级隔离。普通 prompt 不再触发 stage fixture 表演；live streaming 以网络分片为基础向 runtime/CLI 推送事件；provider 在 429、5xx、网络和超时场景下具备可控 retry/backoff。该版本完成后，YunXi 的“能跑”地基会更诚实、更安全，也更接近真实 Codex CLI 终端体验。

## v1.5.0 实施结果

本轮按报告完成 v1.5.0 构建收尾，实施范围集中在 YunXi 自有 runtime、provider、CLI、sandbox/tools 展示层和文档，不引入默认上游 Codex runtime 依赖。

- 版本升级到 `1.5.0`，CLI about/banner/test 断言同步到 `v1.5.0`。
- 离线 one-shot 和 interactive assistant 文本增加 `[offline]` 标记。
- Auto 模式无 live credential 时输出显式 fallback warning。
- `/cost` 在离线模式输出 `n/a - offline, no model call`。
- 默认离线 banner 明确显示 static provider，且 stage fixtures 默认关闭。
- sandbox/exec 对外文本改为 policy guard/advisory wording，明确 `no OS isolation`。
- `StaticProvider` 与 runtime stage fixture 分支默认关闭，仅在 `YUNXI_RUNTIME_FIXTURES=1` 或显式 runtime fixture builder 中开启；fixture event 标注 `fixture_mode=true`。
- provider transport 增加 response headers、`bytes_stream()` streaming path、增量 SSE parser/sink、Retry-After 与 bounded backoff retry。
- runtime provider turn 改为把 provider stream events 即时转成 AgentEvent，同时继续汇总最终 response/tool calls/usage。
- README 和 `docs/extraction-status.md` 已同步 v1.5 行为边界。

## v1.5.0 统一验证结果

统一验证已在 2026-07-12 执行；验证期间遵守 API key/PAT 不打印、不写日志、不提交的约束。

- `cargo fmt`：通过。
- `cargo fmt -- --check`：通过。
- `cargo test`：通过，workspace 单测/集成测试/doc tests 全部完成，0 failure。
- `cargo check --workspace`：通过。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- `target\release\yunxi.exe --version`：`yunxi 1.5.0`。
- `target\release\yunxi-agent-cli.exe --version`：`yunxi 1.5.0`。
- forced offline one-shot：通过，输出包含 `[offline]` 和静态 runtime 文本。
- forced offline interactive `/cost`：通过，输出 `n/a - offline, no model call`。
- auto fallback 无 key smoke：通过，输出 `provider_source: auto_offline` 和显式 warning。
- fixture disabled smoke：通过，普通 `stage 4m real parity fixture` prompt 不触发 `deep_parity_state`。
- fixture explicit smoke：通过，`YUNXI_RUNTIME_FIXTURES=1` 时输出 `fixture_mode=true`。
- DeepSeek live non-stream smoke：通过，使用本地私有 `api.txt` 第 1 个 DeepSeek 候选，19 行 JSONL，marker 命中，`secret_leak_detected=False`。
- DeepSeek live streaming smoke：通过，54 行 JSONL，marker 命中，assistant event index 37 早于 `turn_completed` index 50，`secret_leak_detected=False`。
- retry fixture：通过，provider 测试覆盖 429 + `Retry-After`、timeout transport retry、non-retryable 400。
- sandbox display smoke：通过，输出未包含 `WindowsRestrictedToken` 或 `LinuxLandlock`。
- default CLI dependency scan：通过，`cargo tree -p yunxi-agent-cli` 未命中 `vendor/codex-rs`、`yunxi-agent-codex` 或 `codex-*` 默认依赖。
- owned-source secret scan：通过，排除 `vendor/`、`extracted/`、`target/`、`Cargo.lock` 后无 API key、GitHub PAT、Bearer 形态命中。
- `git diff --check`：通过，仅出现 Windows LF-to-CRLF 提示，无 whitespace error。
- `codegraph sync "D:\YunXi Agent"`：通过，同步 12 个 changed files。
- release install 到 `C:\Users\admin\AppData\Local\YunXi Agent\bin`：通过，`path_updated=False`。
- installed binary 和 PATH `yunxi --version`：均输出 `yunxi 1.5.0`。
- GitHub REST API 发布和 `v1.5.0` tag 创建将在 release commit/tag 后执行，并记录到桌面开发日志。

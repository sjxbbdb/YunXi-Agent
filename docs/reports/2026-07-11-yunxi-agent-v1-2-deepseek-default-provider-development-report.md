# YunXi Agent v1.2 DeepSeek Default Provider Development Report

生成时间：2026-07-11 +08:00

## 核心目标

YunXi Agent v1.2 将此前已经存在的 DeepSeek OpenAI-compatible transport
接入正式 `yunxi` CLI。默认运行模式从 v1.1 的静态 offline provider 改为
`auto`：本机具备有效凭据时调用真实模型，没有凭据时明确回退 offline；同时保留强制
live，并增加强制 offline。

本阶段继续保持 YunXi-owned runtime，不引入上游 Codex runtime 依赖，不扩展 TUI、
desktop、cloud tasks、update、doctor、completion、marketplace 或复杂 installer。

## 硬性约束执行方式

- 所有测试代码先于对应生产实现写入，但构建过程中不运行测试。
- provider、CLI、脚本、版本和文档全部接线完成后，只运行一次统一验证门。
- 最终失败集中分类、集中修复，避免在单点反复消耗时间。
- API key/token 不打印、不写日志、不写源码、不写提交、不作为 GitHub API 内容上传。
- 正式二进制不读取本机 API 文件；API 文件只用于显式导入和 live smoke。
- v1.2 创建新 tag `v1.2.0`，旧 tag `v1.0.0`、`v1.1.0` 保留不动。
- GitHub 远端读取、写入和核验全部通过 GitHub REST API。
- 任务完成后同步桌面开发日志，并清理 `target`、根目录 `.yunxi` 和临时 smoke 文件。

## 构建内容

### Provider 解析

修改：

- `crates/yunxi-agent-provider/src/lib.rs`
- `crates/yunxi-agent-provider/tests/provider_tests.rs`

新增 lookup-driven 配置与认证解析，使测试不需要修改进程全局环境。凭据优先级为：

1. `YUNXI_PROVIDER_API_KEY`
2. `YUNXI_PROVIDER_API_KEY_ENV` 指向的环境变量
3. DeepSeek profile 的 `DEEPSEEK_API_KEY`
4. 其他 OpenAI-compatible profile 的 `OPENAI_API_KEY`

未显式指定 profile 且存在非空 `DEEPSEEK_API_KEY` 时，自动采用 `deepseek` profile。
空白值视为未配置。credential probe 只返回布尔值，不格式化密钥。

### CLI 模式选择

新增：

- `crates/yunxi-agent-cli/src/provider_mode.rs`

修改：

- `crates/yunxi-agent-cli/Cargo.toml`
- `crates/yunxi-agent-cli/src/main.rs`
- `crates/yunxi-agent-cli/src/interactive.rs`
- `crates/yunxi-agent-cli/src/render.rs`
- `crates/yunxi-agent-cli/tests/cli_tests.rs`
- `crates/yunxi-agent-cli/tests/jsonl_tests.rs`

统一模式包括：

- `AutoLive`
- `ForcedLive`
- `ForcedOffline`
- `AutoOffline`

one-shot、interactive 和 `sessions resume` 共用同一解析入口。`--provider-live` 缺少凭据时
在发起 HTTP 请求前返回 provider-classified error；`--offline` 与 `--provider-live`
互斥。所有依赖固定静态响应的测试均显式使用 `--offline`，避免用户环境中的持久化凭据触发网络。
live selection 会把最终 provider/model 写入该轮 `AgentConfig`，确保 turn metadata、session
storage 和后续 resume 记录真实 DeepSeek 配置；offline selection 不改写配置。

### 本机凭据与 Smoke

新增：

- `scripts/provider/import-deepseek-credential.ps1`

修改：

- `scripts/provider/deepseek-live-smoke.ps1`

导入脚本要求显式传入 `-ApiFile`。文件只有一个候选时自动采用；文件有多个候选时必须通过
`-CredentialIndex` 显式选择，脚本不猜测。输出仅包含状态、候选序号、变量名、profile、model
和重开终端提示。smoke 脚本同样改为显式 `-ApiFile` 与多候选序号，删除了仓库中的个人桌面路径。

### 版本与文档

修改或新增：

- `Cargo.toml`
- `Cargo.lock`
- `README.md`
- `docs/extraction-status.md`
- `docs/superpowers/specs/2026-07-11-yunxi-agent-v1-2-deepseek-default-provider-design.md`
- `docs/superpowers/plans/2026-07-11-yunxi-agent-v1-2-deepseek-default-provider.md`
- `docs/reports/2026-07-11-yunxi-agent-v1-2-deepseek-default-provider-development-report.md`

workspace 自有 crate 版本提升到 `1.2.0`。README 增加自动 DeepSeek、显式 live/offline、
安全导入、不可变 tag 和 GitHub API-only 发布说明。

## 最终统一验证门

构建完成后统一执行：

```powershell
cargo fmt
cargo fmt -- --check
cargo test
cargo check --workspace
cargo build -p yunxi-agent-cli --release --bins
target\release\yunxi.exe --version
target\release\yunxi-agent-cli.exe --version
target\release\yunxi.exe --offline --backend yunxi "YunXi Agent v1.2 offline smoke"
@("/session"; "/exit") | target\release\yunxi.exe --offline --backend yunxi
target\release\yunxi.exe --jsonl
cargo run -p yunxi-agent-cli -- parity map
cargo tree -p yunxi-agent-cli
git diff --check
.\scripts\provider\deepseek-live-smoke.ps1 -ApiFile <private-path> -CredentialIndex <n> -Model deepseek-v4-flash
.\scripts\provider\deepseek-live-smoke.ps1 -ApiFile <private-path> -CredentialIndex <n> -Model deepseek-v4-flash -NoStream
```

此外验证：

- 默认 auto one-shot 和 interactive 确实返回真实 DeepSeek assistant 内容。
- `--offline` 在存在凭据时仍不访问网络。
- `--provider-live --offline` 返回参数冲突。
- JSONL 保持纯 JSON line。
- owned-source secret scan 无真实凭据或个人 API 文件路径。
- 默认 CLI dependency tree 不含上游 Codex runtime 依赖。
- 安装后的 PATH 二进制为 `yunxi 1.2.0`。
- CodeGraph 同步、桌面日志更新和产物清理完成。

## 统一验证结果

按用户硬性约束，构建期间没有执行测试、检查、编译、格式化或 live 请求。全部接线完成后运行
统一验证门；首次统一门发现一个未使用 import warning，收集完其余结果后集中移除，并完整重跑
Rust 门。安全扫描第一次在工具字符串转义后误包含 `vendor`/`extracted`，经只输出文件路径的根因
分析确认 8 个命中全部来自参考树；改用平台目录分隔符后，自有源码正式扫描为零命中。

### Rust 与 CLI

- `cargo fmt`：通过。
- `cargo fmt -- --check`：通过。
- `cargo test`：通过；workspace 共 190 个测试，0 失败。
- `cargo check --workspace`：通过，无 warning。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- `target\release\yunxi.exe --version`：通过，`yunxi 1.2.0`。
- `target\release\yunxi-agent-cli.exe --version`：通过，`yunxi 1.2.0`。
- offline one-shot：通过，返回静态 YunXi fixture。
- offline piped interactive：通过，显示 `provider_mode: offline` 和
  `provider_source: forced_offline`。
- `--provider-live --offline`：按预期退出码 2，并报告参数冲突。
- `--jsonl` 缺少 prompt：按预期退出码 2，并输出结构化 error event。
- parity map：通过。
- `git diff --check`：通过。

### DeepSeek 真实模型

- stream smoke：通过；退出码 0，48 行 JSONL，`secret_leak_detected=False`。
- non-stream smoke：通过；退出码 0，19 行 JSONL，`secret_leak_detected=False`。
- 默认 auto one-shot：通过；返回指定 DeepSeek assistant 内容，未命中 offline fixture。
- 默认 auto interactive：通过；显示 `provider_mode: live`、
  `provider_source: auto_live`、`provider: deepseek`。
- 默认 auto JSONL metadata：通过；48 行纯 JSONL、2 条 turn metadata，provider 为
  `deepseek`、model 为 `deepseek-v4-flash`，无效 JSON 与泄漏检测均为 0。
- auto one-shot/interactive 合并输出秘密扫描：0 命中。

### 安全、依赖与安装

- 默认 `cargo tree -p yunxi-agent-cli`：未命中 `vendor/codex-rs`、`codex-*` 或
  `yunxi-agent-codex`。
- owned-source scan：扫描 108 个自有发布范围文件，secret pattern 0 命中，个人 API 文件路径
  0 命中。
- 多候选导入未指定 `-CredentialIndex`：按预期拒绝并退出 1。
- 显式 `-CredentialIndex 1` 用户级导入：通过，输出泄漏检测为 False。
- `scripts/install/install-yunxi.ps1 -AddToPath -SkipBuild`：通过，
  `path_updated=False`。
- 安装后的 `yunxi.exe`：与本次 release 构建 SHA-256 一致；版本 1.2.0、auto interactive
  DeepSeek、真实 one-shot 全部通过，未回退 offline，输出泄漏检测为 False。

### 发布状态

- 本地源码、190 项测试、release 产物、真实 DeepSeek 和安装版验收：通过。
- `v1.2.0` tag：建立在本版最终提交上；旧 `v1.0.0`、`v1.1.0` tag 未删除、未移动。
- GitHub：master 与 `v1.2.0` 仅通过 GitHub Git Data REST API 发布，并通过 REST API 回读
  refs、commit 和 tree 完成核验；未使用 Git transport 推送或拉取命令。
- CodeGraph：最终源码同步完成。
- 桌面开发日志：本阶段详细记录已追加。
- 构建产物：发布后执行 `cargo clean`，并清理仓库根目录 `.yunxi` 与临时 smoke 文件。

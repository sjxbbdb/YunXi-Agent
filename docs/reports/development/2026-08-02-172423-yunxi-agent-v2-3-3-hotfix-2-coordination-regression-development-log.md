# 2026-08-02 17:24:23 +08:00 — YunXi Agent v2.3.3-hotfix.2 协同回归与记忆可召回口径修复日志

工作目标：按用户要求，不以新增功能数量为目标，而以已有 CLI、微信、Runtime、Persona、Companion、Memory 之间能否稳定协同为验收标准；补齐跨端协同回归，并修复长期记忆管理口径中“存储 active”与“运行时可召回”混淆的问题。

## 执行内容

1. 使用 CodeGraph 先梳理长期记忆加载、Runtime persona context、微信 supervisor 和 CLI memory 管理链路。
2. 确认 Runtime 长期记忆加载链路本身正常：`active_records()` 与 recall router 会排除 pending、archived、expired、invalidated、superseded 记录。
3. 修复 CLI memory 管理展示口径：
   - `memory status` 增加 `runtime.recallable` 与 `runtime.non_recallable_active`。
   - `memory list/search/show --json` 的每条记录增加 `runtime_status`、`runtime_recallable`、`runtime_blockers`。
   - 文本输出同步显示 `storage=...`、`runtime=...`、`blocked_by=...`。
4. 补充 CLI 回归：语言偏好 supersession 后，旧记录仍保留历史但 `runtime_recallable=false`，新记录 `runtime_recallable=true`。
5. 补充微信协同回归：微信 `WeixinTurnSupervisor` 通过真实 `YunXiRuntimeBackend` 处理私聊 pending inbound 时，会加载与 CLI/Runtime 同一份长期记忆，并把记忆写入共享 persona context。
6. 将版本升级为 `2.3.3-hotfix.2`，不移动、不覆盖 `v2.3.3-hotfix.1` 或任何历史 tag。
7. 构建 release，并安装替换到两个明确 YunXi 安装目录。

## 修改文件与路径

- `D:\YunXi Agent\Cargo.toml`
  - workspace 版本从 `2.3.3-hotfix.1` 升级为 `2.3.3-hotfix.2`。
- `D:\YunXi Agent\Cargo.lock`
  - 同步 workspace crate 版本，并为 `yunxi-agent-weixin` 测试声明 persona dev-dependency。
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
  - 为 memory 管理输出增加运行时可召回状态和阻塞原因。
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`
  - 增加 superseded 旧记忆与新记忆的 runtime recallable 断言。
- `D:\YunXi Agent\crates\yunxi-agent-weixin\Cargo.toml`
  - 增加测试用 `yunxi-agent-persona` dev-dependency。
- `D:\YunXi Agent\crates\yunxi-agent-weixin\tests\turn_supervisor_tests.rs`
  - 增加微信 supervisor + 真实 runtime + 共享长期记忆的跨端协同回归。
- `D:\YunXi Agent\docs\development-log.md`
  - 追加本轮开发记录。
- `D:\YunXi Agent\docs\reports\development\2026-08-02-172423-yunxi-agent-v2-3-3-hotfix-2-coordination-regression-development-log.md`
  - 新增本轮开发报告。

## 验证结果

- `cargo fmt --all`：通过。
- `cargo check --workspace`：通过。
- `cargo test -p yunxi-agent-cli cli_language_change_creates_supersession_chain -- --test-threads=1`：通过。
- `cargo test -p yunxi-agent-weixin supervisor_real_runtime_loads_same_long_term_memory_context_as_cli -- --test-threads=1`：通过。
- `cargo test -p yunxi-agent-runtime memory_recall_query_tests -- --test-threads=1`：通过。
- `cargo test -p yunxi-agent-persona recall -- --test-threads=1`：通过。
- `cargo test -p yunxi-agent-cli -p yunxi-agent-weixin -p yunxi-agent-runtime -p yunxi-agent-persona -- --test-threads=1`：通过。
- `cargo test --workspace -- --test-threads=1`：通过。
- `cargo run -q -p yunxi-agent-cli --bin yunxi -- --json eval companion`：通过，33/33。
- `cargo run -q -p yunxi-agent-cli --bin yunxi -- --json eval weixin`：通过，离线门禁无失败，2 个真实人工门禁仍保持 not_run。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。

## 安装与运行状态

- 安装目标一：`D:\Apps\YunXi Agent\bin\yunxi.exe`
- 安装目标一兼容二进制：`D:\Apps\YunXi Agent\bin\yunxi-agent-cli.exe`
- 安装目标一备份：`D:\Apps\YunXi Agent\bin\backup-20260802-172310`
- 安装目标二：`C:\Users\24763\AppData\Local\YunXi Agent\bin\yunxi.exe`
- 安装目标二兼容二进制：`C:\Users\24763\AppData\Local\YunXi Agent\bin\yunxi-agent-cli.exe`
- 安装目标二备份：`C:\Users\24763\AppData\Local\YunXi Agent\bin\backup-20260802-172310`
- 安装版版本：`yunxi 2.3.3-hotfix.2`
- 构建产物与两个安装目标 SHA-256 一致：`CF8E31364D984F4E3AB72A05DFFD48CEB0D925880CCC3CB70B71A953DC085F2D`
- 当前微信服务：PID `10988`，命令行为 `yunxi weixin serve --workspace "D:\YunXi Agent" --companion`
- 当前微信状态：`version=2.3.3-hotfix.2`，`state=ready`，`account_lock_state=active`，`credential_state=present`，pending inbound/delivery/remote control 均为 0。

## 安装版记忆口径验证

- `yunxi memory --cwd "D:\YunXi Agent" --json status`：
  - `counts.active=3`
  - `runtime.recallable=2`
  - `runtime.non_recallable_active=1`
  - `warnings=[]`
- `yunxi memory --cwd "D:\YunXi Agent" --json search YUNXI_MEMORY_TEST_SHELL_20260801`：
  - 被替代旧记忆仍保留历史。
  - `runtime_recallable=false`
  - `runtime_blockers=["invalidated","superseded"]`

## 提交、推送和 tag 状态

代码、测试、release 构建、安装替换和日志已完成；下一步创建 release commit、annotated `v2.3.3-hotfix.2` tag，并使用 GitHub CLI + Git Data API non-force 推送到 `https://github.com/sjxbbdb/YunXi-Agent`。历史 tag 不删除、不移动、不覆盖。

## 安全边界

- 未删除、递归清理、移动或清理用户目录。
- 未执行 `git reset`、`git clean` 或 force 操作。
- 安装替换只针对两个明确 YunXi 安装目录。
- 停止进程只针对安装目录下的旧 `yunxi.exe` 微信服务 PID `20020`。
- API key 不打印、不写入仓库、不写入 remote URL 或 Git 配置。

署名：开发者

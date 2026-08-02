# 2026-08-02 16:00:11 +08:00 — YunXi Agent v2.3.3-hotfix.1 微信实机队列恢复修复日志

工作目标：在完成 v2.3.3 陪伴层发布后，按用户要求安装替换并进行微信实机测试；修复实机测试暴露出的微信远程控制审批超时卡住入站队列、状态 pending 口径不准确、成功 trace 保留历史错误标签的问题。

## 实机测试发现

1. 已将 `D:\Apps\YunXi Agent\bin\yunxi.exe` 与 `C:\Users\24763\AppData\Local\YunXi Agent\bin\yunxi.exe` 从旧版本替换到 `2.3.3`，随后拉起 `yunxi weixin serve --workspace "D:\YunXi Agent" --companion`。
2. 微信服务可正常启动，状态为 `ready`，凭据存在，schema 为 6。
3. 用户实机消息成功进入 `pending_inbound`，但测试语句中“说明当前版本”触发 shell 工具审批后，微信队列出现阻塞：首条消息已产生 delivery/spool trace，但 pending 仍停留在 running，后续消息显示 `runtime_dispatch_inflight` / `runtime_queue_full`。
4. 等待远程控制超时后，状态仍显示 pending remote control，说明仅依赖后台 timeout task 不足以释放真实服务里的审批等待。

## 修改文件与路径

- `D:\YunXi Agent\Cargo.toml`
  - 版本从 `2.3.3` 升级为 `2.3.3-hotfix.1`，避免移动或覆盖已发布的 `v2.3.3` tag。
- `D:\YunXi Agent\Cargo.lock`
  - 同步 workspace crate 版本到 `2.3.3-hotfix.1`。
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\remote_control.rs`
  - 新增 `WeixinRemoteControlHub::expire_due(now_millis)`。
  - 补充 `hub_expire_due_denies_registered_approval` 回归测试，确保到期审批会被拒绝并唤醒等待中的 runtime。
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\serve.rs`
  - 主循环在 poll 前后主动调用 `expire_due_remote_controls`。
  - 新增 `remote_control_expired_count` 计数。
  - 新增 `WeixinServeError::RemoteControl`，保证远程控制过期扫描异常可被显式呈现。
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs`
  - `complete_pending_runtime_turn` 在成功完成时清除历史 `error_label`，避免成功 trace 继续显示旧的 `runtime_queue_full`。
  - 新增 `pending_remote_control_count_at(now_millis)`，用于按当前时间排除已过期远程控制请求。
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs`
  - `weixin status` 与 `weixin doctor` 的 `pending_remote_control_count` 改用当前时间口径，避免把已过期控制请求继续显示为 pending。
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs`
  - TUI 全帧 snapshot fixture 固定测试版本为 `v2.3.3`，避免 workspace hotfix 版本号变化导致布局快照误报；正式运行版本仍由 `env!("CARGO_PKG_VERSION")` 输出。

## 安装与运行状态

- 安装目标一：`D:\Apps\YunXi Agent\bin\yunxi.exe`
- 安装目标一兼容二进制：`D:\Apps\YunXi Agent\bin\yunxi-agent-cli.exe`
- 安装目标一备份：`D:\Apps\YunXi Agent\bin\backup-20260802-155838`
- 安装目标二：`C:\Users\24763\AppData\Local\YunXi Agent\bin\yunxi.exe`
- 安装目标二兼容二进制：`C:\Users\24763\AppData\Local\YunXi Agent\bin\backup-20260802-155838`
- 当前 PATH 命中：`D:\Apps\YunXi Agent\bin\yunxi.exe`
- 当前安装版本：`yunxi 2.3.3-hotfix.1`
- 当前微信服务：PID `18260`，命令行为 `yunxi weixin serve --workspace "D:\YunXi Agent" --companion`
- 当前 `weixin status --json` 摘要：`state=ready`，`pending_inbound_count=0`，`pending_delivery_count=0`，`pending_remote_control_count=0`，`latency_trace_count=9`。

## 验证结果

- `cargo fmt --all`：通过。
- `cargo test -p yunxi-agent-storage -p yunxi-agent-weixin -p yunxi-agent-cli`：通过。
- `cargo test -p yunxi-agent-tui --lib`：通过，163/163。
- `cargo test --workspace`：通过。
- `cargo run -q -p yunxi-agent-cli --bin yunxi -- --json eval companion`：通过，33/33，`golden_passed=true`。
- `cargo run -q -p yunxi-agent-cli --bin yunxi -- --json eval weixin`：通过，离线门禁通过；真实微信人工门禁仍需用户继续发送新私聊消息验证。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- `target\release\yunxi.exe --version`：`yunxi 2.3.3-hotfix.1`。
- `D:\Apps\YunXi Agent\bin\yunxi.exe --version`：`yunxi 2.3.3-hotfix.1`。

## 提交、推送和 tag 状态

截至本日志写入时，代码修复、测试、release 构建、安装替换和日志已完成；下一步将创建 release commit、annotated `v2.3.3-hotfix.1` tag，并使用 GitHub CLI/API key non-force 推送到 `https://github.com/sjxbbdb/YunXi-Agent`。历史 `v2.3.3` tag 不删除、不移动、不覆盖。

## 安全边界

未执行删除用户目录、递归清理用户目录、移动用户目录、git reset、git clean、force push 或历史 tag 覆盖。安装替换仅限两个明确 YunXi 安装目录下的 `yunxi.exe` 与 `yunxi-agent-cli.exe`，并在替换前创建备份。

署名：开发者

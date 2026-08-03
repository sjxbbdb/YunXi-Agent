# YunXi Agent 微信服务随 CLI 启动修复日志

## 时间戳

2026-08-03 09:05（Asia/Shanghai）

## 问题

用户启动 `yunxi` CLI/TUI 后，微信端服务没有同步拉起。排查发现当前实现只在交互式 CLI/TUI 分支尝试 autostart，但微信账号元数据只从当前 `cwd` 读取；如果用户从 `C:\WINDOWS\System32`、桌面或其他目录启动全局安装的 `yunxi`，会因为当前目录没有 `.yunxi/weixin/account-*.json` 而静默跳过微信服务。

## 变更内容

1. 修改 `crates/yunxi-agent-cli/src/main.rs`：
   - 新增 `YUNXI_WEIXIN_WORKSPACE` 作为可选工作区覆盖环境变量；
   - 新增微信 autostart 工作区解析逻辑；
   - autostart 不再只依赖当前 `cwd`，会在候选工作区中寻找真实微信账号元数据；
   - 找到账号元数据后，启动微信服务子进程时将 `--cwd` 强制设置为真实工作区；
   - 抽出统一的微信 autostart 结果打印函数；
   - `yunxi web` / `yunxi --provider-live web` 启动 Web Console 时也会尝试拉起微信服务；
   - 仍保留 `--no-weixin-autostart` 和 `YUNXI_WEIXIN_AUTOSTART=0/false/off/no` 关闭开关。

## 验证

- `cargo fmt --all -- --check`：通过。
- `cargo test -p yunxi-agent-cli tests::autostart`：通过。
- `cargo test -p yunxi-agent-cli web::tests`：通过。
- `cargo build --release -p yunxi-agent-cli`：通过。
- `cargo test`：全仓通过，0 失败。
- 使用安装后的新版本启动 Web Console 后，微信 autostart 成功：
  - Web 进程：`D:\Apps\YunXi Agent\bin\yunxi.exe --provider-live web`
  - 微信服务子进程：`D:\Apps\YunXi Agent\bin\yunxi.exe --cwd "\\?\D:\YunXi Agent" --provider-live --approval on-request --sandbox workspace-write --memory-extraction auto bot start --channels weixin --account default`
  - `weixin status` 显示 `account lock state: active`
  - `/api/health` 返回 HTTP 200，版本 `2.3.3-hotfix.3`

## 安装替换

- 已停止旧的 `D:\Apps\YunXi Agent\bin\yunxi.exe` 进程。
- 已安装 release 构建：
  - `D:\Apps\YunXi Agent\bin\yunxi.exe`
  - `D:\Apps\YunXi Agent\bin\yunxi-agent-cli.exe`
- 已保留旧版本备份：
  - `D:\Apps\YunXi Agent\bin\backup-20260803-090241-weixin-cli-autostart`

## GitHub

本次仅完成本地修复、构建、测试和安装替换；没有创建或推送 tag，也没有修改历史 tag。

## 署名

开发者

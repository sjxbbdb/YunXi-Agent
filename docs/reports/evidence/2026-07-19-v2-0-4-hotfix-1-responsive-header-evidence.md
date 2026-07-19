# YunXi Agent v2.0.4-hotfix.1 窄屏 Header 整改复核证据

- 复核时间：2026-07-19 21:25:25 +08:00
- 开发目录：`D:\YunXi Agent`
- 复核版本：`2.0.4-hotfix.1`
- 证据性质：开发者整改复测记录，不等同于独立审核通过

## 响应式信息集合

`YunxiTuiApp::header_for_width` 在构造优先级段前先选择语义档位：

- `width < 90`：只构造产品/版本与 provider/live 状态。
- `90 <= width < 120`：增加 model，但不构造 cwd。
- `width >= 120`：增加 model 与按路径边界压缩或完整保留的 cwd。

固定矩阵测试明确断言：80 列无 `model=`、盘符 cwd、路径尾段和 `.../`；100 列有
model 且无 cwd；120 与 200 列有 model 和 cwd。四份完整 frame 均由真实
`render_tui_frame` 与 `ratatui::TestBackend` 机械生成，并继续验证逐行显示宽度、连续区域、
scrollbar containment、active assistant、pinned history、`new output below`、footer 和
composer。

最终快照首行：

```text
80:  YunXi v2.0.4-hotfix.1 | deepseek live
100: YunXi Agent v2.0.4-hotfix.1 | deepseek live | model=deepseek-chat-ultra-long-model-name
120: YunXi Agent v2.0.4-hotfix.1 | deepseek live | model=deepseek-chat-ultra-long-model-name | C:/.../yunxi-agent-cli
200: YunXi Agent v2.0.4-hotfix.1 | deepseek live | model=deepseek-chat-ultra-long-model-name | C:\Users\24763\YunXi Agent\国际化工作区\crates\yunxi-agent-cli
```

## 自动化验证

- `cargo fmt --all` 与 `cargo fmt --all -- --check`：通过。
- `cargo check --workspace`：通过，无编译警告。
- `cargo test -p yunxi-agent-tui`：101/101 通过。
- `cargo test --workspace`：全部 workspace 单元、集成和 doc tests 通过；CLI integration
  44/44、provider 44/44、TUI 101/101。
- `cargo build --workspace` 与 `cargo build -p yunxi-agent-cli --release --bins`：通过。
- `yunxi.exe --version` 与 `yunxi-agent-cli.exe --version`：均为
  `yunxi 2.0.4-hotfix.1`。
- `yunxi.exe` SHA-256：
  `27C7332C5D46188EB00F96694575B8E5CBA51ED9168B976ACEA1C709E2436B29`。
- `yunxi-agent-cli.exe` SHA-256：
  `9354BBDABBEB568521CDABBFAEDF59B1CAAABFF57E71F2C35CC8B90EC1DC62B6`。
- Evaluation Harness：`harness_version=2.0.4-hotfix.1`，31/31、失败 0、
  `golden_passed=true`、golden failure 0；JSON 可解析，JSONL 恰好一行且可解析；
  memory precision、relationship continuity、persona consistency 均为 1.0，主动边界违规
  与 tool approval bypass 均为 0。
- offline one-shot：通过并显示明确 `[offline]` 标记。
- DeepSeek live one-shot：退出码 0，输出精确为 `V204H1_LIVE_OK`。

## Windows ConPTY 真实 TUI

临时驱动使用 `node-pty 1.1.0` 与 `@xterm/headless 5.5.0`，仅位于本轮 `target`
子目录。驱动在内存中维护终端缓冲和泄漏扫描，不保存 raw 终端流。DeepSeek API key 只
通过进程环境继承，没有写入脚本、仓库、snapshot、报告或日志。

offline 会话退出码 0、66,212 bytes、7 个检查点。检查顺序为 80x24、100x30、
120x40、200x50、80x24、100x30、120x40；完成国际化 bracketed paste、Emoji ZWJ、
组合字符与两阶段退格，并完成离线提交和正常退出。真实首行分别证明 80 无 model/cwd、
100 有 model 无 cwd、120/200 有 cwd，缩小与放大后立即按档位切换。

DeepSeek live / `deepseek-v4-flash` 会话退出码 0、112,368 bytes、11 个检查点。除上述
双向 resize 外，还覆盖 active 长流中的 200 到 80 resize、PageUp、End、`Ctrl+C`
取消、partial assistant 保留、取消通知、Composer 恢复，以及同一进程下一轮终态
`[assistant] V204H1_NEXT_OK`。

offline 与 live 内存扫描均未发现 API key、`Authorization:`、`Bearer`、
`arguments_json`、provider wire、memory/context 或内部错误栈标记。两次会话均没有遗留
YunXi 或 OpenConsole 进程。

## 清理与发布状态

用户明确授权后执行 `cargo clean`，报告移除 17,265 个文件、4.8 GiB；随后在不读取
内容的前提下递归删除精确路径 `D:\YunXi Agent\crates\yunxi-agent-cli\.yunxi`。最终
`D:\YunXi Agent\target` 与该 `.yunxi` 路径均不存在，仓库原有 `.tmp` 保持存在，没有
触碰其他目录。

截至本记录更新时，尚未创建提交、annotated `v2.0.4-hotfix.1` tag 或远程推送；已发布
`v2.0.4` 和所有旧 tag 保持不变。发布时只能新增 hotfix tag 并 non-force 推送。

署名：开发者

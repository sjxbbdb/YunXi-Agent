# YunXi Agent v2.0.4 国际化排版与响应式 TUI 复核证据

- 开发目录：`D:\YunXi Agent`
- 复核版本：`2.0.4`
- 证据性质：开发者复测记录，不等同于独立审核通过

## 自动化布局证据

`crates/yunxi-agent-tui/src/text_layout.rs` 为 transcript、composer、approval、
header、subheader 与 footer 提供统一的显示宽度、grapheme-safe 折行/截断、源文本范围和
cursor visual row/column 映射。测试矩阵覆盖 CJK、日文假名、Emoji ZWJ、组合字符、带
query/fragment 的 URL、带空格和中文的 Windows 长路径，以及带 fence、缩进和 CJK 注释
的代码块。

完整 `ratatui::TestBackend` frame 位于：

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\snapshots\full_frame_80x24.txt`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\snapshots\full_frame_100x30.txt`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\snapshots\full_frame_120x40.txt`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\snapshots\full_frame_200x50.txt`

四份快照通过真实 layout、wrapped transcript、active canonical assistant cell、pinned
history、`new output below`、scrollbar、footer 和 composer 路径机械生成。approval 另有
80/100/120/200 四宽度矩阵，验证风险、危险命令、cwd、Approve/Decline 与快捷行在窄屏
下仍可识别。

普通测试只比较已提交快照；显式设置 `YUNXI_UPDATE_SNAPSHOTS=1` 才会执行测试专用的
机械更新入口。

## 自动化与真实终端验收状态

2026-07-19 19:04:13 +08:00 前完成以下自动化门禁：

- `cargo fmt --all` 与 fmt check：通过。
- `cargo check --workspace`、`cargo test --workspace`、`cargo build --workspace`：通过。
- `cargo build -p yunxi-agent-cli --release --bins`：通过；两个 release binary 均返回
  `yunxi 2.0.4`。
- TUI 单元与集成测试：101/101；包含 1,000 delta、严格低于 30 FPS、viewport、
  resize、cancel、四份完整 frame、四宽度 approval，以及 cursor/source-range 新回归。
- Evaluation Harness：`harness_version=2.0.4`，31/31、失败 0、
  `golden_passed=true`；JSON 可解析，JSONL 恰好一行；memory precision/recall、
  relationship continuity 与 control regression 均为 1.0，主动边界违规和 tool approval
  bypass 均为 0。

## Windows ConPTY 真实 TUI

- 终端：Windows ConPTY，`xterm-256color`。
- 临时驱动：`node-pty 1.1.0`，仅位于本轮 `target` 临时目录，不进入 Git。
- release 程序：`D:\YunXi Agent\target\release\yunxi.exe`。
- live Provider/model：DeepSeek / `deepseek-v4-flash`。
- 隔离状态与 workspace：仅位于本轮 `target` 临时目录。

offline 会话退出码为 0，记录 50,633 bytes，包含 6 个检查点：80x24 启动、国际化
粘贴/cursor/退格、100x30、120x40、200x50 resize，以及提交后滚动与 End follow-tail。
提交内容覆盖中文、日文假名、Emoji ZWJ、组合字符、带 query/fragment 的 URL 和带空格
及中文目录的 Windows 长路径；assistant 使用明确的 `[offline]` 标记。

DeepSeek live 会话退出码为 0，记录 192,867 bytes，包含 8 个检查点：80x24 启动、
国际化粘贴/cursor/退格、100x30、120x40、200x50 resize、active 长流中的滚动/resize/
End、`Ctrl+C` 取消与 partial 保留、取消后的下一轮。live 长流出现 `[assistant*]`；取消后
普通 transcript 显示 cancellation requested、current turn cancelled 和 turn cancelled，
composer 保持可用；同一进程下一轮得到终态 `[assistant] V204_NEXT_OK`，随后 `/exit`
正常退出。

offline 与 live 原始流均通过布尔扫描：存在 `v2.0.4`、国际化提示、URL 与 Windows
路径；未发现 API key、`Authorization`、`Bearer`、`arguments_json`、thinking、
memory/context 或 provider wire 标记。API key 仅由进程环境继承，没有写入 snapshot、
本证据、开发报告或开发日志。

原始 ConPTY 流、临时 node-pty 与隔离状态只用于本轮复核。2026-07-19
19:09:14 +08:00 前经用户明确授权，已通过 `cargo clean` 随 `target` 一并清理；仓库
长期保留本摘要和四份确定性 frame snapshot。

截至本记录写入时，尚未执行 Git 提交、annotated `v2.0.4` tag 或远程推送；因此
本证据不提前宣称发布完成，也不等同于独立审核通过。

署名：开发者

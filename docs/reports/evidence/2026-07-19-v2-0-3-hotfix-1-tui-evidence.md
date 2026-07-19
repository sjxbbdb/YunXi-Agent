# YunXi Agent v2.0.3-hotfix.1 TUI 复核证据

- 记录时间：2026-07-19 16:30:48 +08:00
- 复核版本：`2.0.3-hotfix.1`
- 发布候选 tag：annotated `v2.0.3-hotfix.1`
- 开发目录：`D:\YunXi Agent`
- 证据性质：整改开发者复测记录，不等同于独立审核通过

## 复核环境

- 终端：Windows ConPTY，`xterm-256color`
- release 程序：`D:\YunXi Agent\target\release\yunxi.exe`
- offline 启动参数：`--backend yunxi --offline --cwd D:\YunXi Agent\.tmp\v203-hotfix-tui-session`
- live 启动参数：`--backend yunxi --provider-live --cwd D:\YunXi Agent\.tmp\v203-hotfix-tui-session --model deepseek-v4-flash`
- live Provider：DeepSeek
- live model：`deepseek-v4-flash`
- 隔离状态目录：`D:\YunXi Agent\.tmp\v203-hotfix-tui-state`
- 初始尺寸：80x24；动态复核尺寸：120x40、58x18、80x24

API key 只通过进程环境读取，没有写入本证据、snapshot、报告或开发日志。

## 操作与检查点

offline 会话在 80x24 下启动，提交 `OFFLINE_TUI_HOTFIX_SMOKE`，确认单个用户 cell
和单个带 `[offline]` 标记的 assistant cell 后执行 `/exit`。该会话记录 3,561 bytes
原始终端输出，进程退出码为 0。

live 会话依次执行：

1. 在 80x24 下启动，确认标题显示 `v2.0.3-hotfix.1`、DeepSeek live 和
   `deepseek-v4-flash`。
2. 提交 `Reply exactly: HOTFIX_SHORT_OK`，确认状态为 `cells=2`，屏幕仅有一个
   终态 `[assistant] HOTFIX_SHORT_OK`，不存在 `[assistant*]`。
3. 提交 40 节长流提示；在 `[assistant*]` 活跃且 transcript 可滚动时发送两次滚轮
   上滚和 PgUp，确认进入 `new output below`，新输出没有抢回尾部。
4. 发送 PgDown 和滚轮下滚，确认向下浏览路径可用。
5. 在 active stream 中依次 resize 到 120x40、58x18；两种尺寸均重新绘制完整 frame，
   composer 和包含 `Ctrl+C` 的 footer 保持可见。
6. 恢复 80x24 后发送 Home，确认历史顶部锚点和 `new output below`；发送 End，确认恢复
   `tail` 且清除 `new output below`。
7. 在发送 Ctrl+C 前硬等待 `[assistant*]`；发送 PgUp 与 Ctrl+C 后，确认 active 标记
   消失、composer 仍可用、partial assistant cell 保留。回到尾部后确认三条取消通知：
   `cancellation requested`、`current turn cancelled`、`turn cancelled`。
8. 在同一进程提交 `Reply exactly: HOTFIX_NEXT_OK`，确认得到终态
   `[assistant] HOTFIX_NEXT_OK`，证明取消后的下一轮仍可运行。
9. 执行 `/exit`，进程正常退出。

live 会话共保留 10 个检查点，记录 39,896 bytes 原始终端输出，进程退出码为 0。

## 检查点清单

| 检查点 | 尺寸 | 关键状态 |
| --- | --- | --- |
| `live-startup-80x24` | 80x24 | DeepSeek live 启动 |
| `short-response-one-canonical-cell` | 80x24 | `cells=2`，唯一终态 assistant cell |
| `active-wheel-pageup-pinned-history` | 80x24 | active stream，`new output below` |
| `active-pagedown-wheel-down` | 80x24 | 向下浏览路径执行 |
| `active-resize-120x40` | 120x40 | active frame、composer、footer 可见 |
| `active-resize-58x18` | 58x18 | 窄小尺寸重绘、composer、footer 可见 |
| `active-home-history` | 80x24 | 顶部历史锚点、`new output below` |
| `active-end-follow-tail` | 80x24 | 精确恢复 `tail` |
| `cancelled-partial-retained` | 80x24 | active turn 停止、partial 保留 |
| `next-turn-after-cancel` | 80x24 | 取消通知和 `HOTFIX_NEXT_OK` |

## 信息边界检查

开发者检查了 offline/live 检查点与持久化 JSON 文本，普通屏幕未出现 API key、
`Authorization`、`arguments_json`、thinking、provider wire、memory/context 原文或内部
错误栈。该结论只覆盖本次记录的屏幕与输出，不替代后续独立审核。

## 自动化 frame 证据

真实 TUI 复测之外，仓库还保留两个完整、确定性的 `ratatui::TestBackend` frame：

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\snapshots\full_frame_80x24.txt`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\snapshots\full_frame_120x40.txt`

测试通过真实 layout、wrapped transcript、active canonical assistant cell、pinned history、
`new output below`、scrollbar、footer 和 composer 路径生成 frame，并断言完整文本相等、
固定行数、显示宽度边界、区域连续、scrollbar containment 和必要内容存在。

## 清理说明

2026-07-19 16:50:18 +08:00，经用户明确授权，本轮原始 PTY JSON、证据脚本、隔离状态、
隔离会话和 node-pty 临时目录已在上述摘要固化后清理。仓库长期保留本证据摘要和两个
确定性完整 frame snapshot；临时原始 PTY 文件不进入版本库。

署名：开发者

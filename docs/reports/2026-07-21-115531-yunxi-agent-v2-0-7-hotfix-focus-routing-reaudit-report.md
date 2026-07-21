# YunXi Agent v2.0.7-hotfix 焦点路由复审审核报告

- 审核时间：2026-07-21 11:55:31 +08:00
- 审核版本：`v2.0.7-hotfix`
- 发布提交：`3893a7c12cc51768dff583d216fe4563f613bd6b`
- annotated tag 对象：`10e182ca46d095274d53b5a167e4f82e52091b1f`
- 当前 `HEAD`：`47eb93334fc2758a69f62e7b19fa7d859e4b25e7`
- 审核目录：`D:\YunXi Agent`
- 审核依据：`C:\Users\24763\Desktop\YunXi Agent v2.0.0-v2.1.0 TUI与流式输出重构总纲图（复核版）.md` 的 `v2.0.7` 焦点、详情层与快捷键验收要求，以及 `2026-07-21-082757-yunxi-agent-v2-0-7-interaction-focus-details-audit-report.md` 的 P1 整改要求。

## 一、上一轮阻塞项复审

上一轮 P1 为 Approval/Details 焦点下，鼠标滚轮和 transcript scrollbar 可绕过焦点隔离，导致 Approval 可能滚动底层历史、Details 鼠标滚轮不能只浏览详情。

本次复审通过，整改如下：

1. `crates/yunxi-agent-tui/src/input_map.rs` 为 `FocusTarget` 建立显式鼠标动作矩阵：Composer/History 映射为 transcript 滚动；Approval 映射为 `None`；Details 映射为 `DetailScrollUp` / `DetailScrollDown`。
2. `crates/yunxi-agent-tui/src/host.rs` 的 `handle_mouse_scroll_by_focus` 消费 Approval 的鼠标滚轮且不修改 viewport；Details 滚轮只修改 `details_scroll`。
3. `allows_transcript_scrollbar` 仅允许 Composer/History 操作 scrollbar。Approval/Details 下的点击、拖动和释放均清除 drag 状态，不会改变 transcript。
4. `input_map::focus_action_matrix_is_explicit` 覆盖 Composer、History、Approval、Details 下 Esc、Enter、PgUp/PgDown、Tab、Ctrl+C、Paste 和滚轮的动作矩阵。
5. `host` 新增 Approval 鼠标冻结、Details 滚轮/PgUp/PgDown 只影响详情、Composer/History 保持正常鼠标滚动等测试。

上述实现满足总纲图的焦点优先级和“Approval 不意外滚动历史、Details 可通过鼠标浏览”的硬性要求。

## 二、真实 TUI 与 Provider 复审

已独立运行真实 DeepSeek / Windows ConPTY 场景，新的隔离审计证据位于：

`D:\YunXi Agent\.tmp\audit-v207-hotfix-conpty-20260721\frames`

两个场景均正常退出，所有检查点匹配：

| 场景 | 复审结果 |
| --- | --- |
| `approval-freeze` | 滚轮、scrollbar 点击、拖动与释放后 Approval 覆盖层保持稳定，transcript 帧不变；58x18 窄屏保留审批选择与取消提示，且不显示滚动历史提示。 |
| `details-scroll` | Details 滚轮与 PgDown 均改变详情内容；58x18 窄屏保留 `Esc close` 与滚动提示；Esc 关闭后 transcript anchor 与 Composer footer 恢复。 |

人工视觉检查确认：

- Approval 中默认 Decline 选择、Approve/Decline、Tab、Enter 和 Ctrl+C 提示清晰，鼠标操作没有扰动历史内容；
- Details 中标题、关闭命令、滚动提示与长详情内容同屏可读，窄屏没有重叠或越界；
- 关闭 Details 后回到原 transcript 位置，普通 Composer footer 恢复；
- 既有 v207 ConPTY 八场景的 Composer、IME、取消、审批草稿恢复和单一 assistant cell 回归证据仍通过。

## 三、验证结果

| 验证项 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | 通过 |
| `cargo check --workspace` | 通过 |
| `cargo test --workspace` | 通过，无失败项 |
| `cargo test -p yunxi-agent-tui` | 136/136 通过 |
| `cargo build -p yunxi-agent-cli --release --bins` | 通过 |
| `target\\release\\yunxi.exe --version` | 返回 `yunxi 2.0.7-hotfix` |
| `target\\release\\yunxi.exe eval companion --json` | 31/31 通过，`golden_passed=true`，`tool_approval_bypass_count=0` |
| `npm.cmd run verify --prefix scripts\\conpty\\v207-hotfix` | 通过，2 个焦点路由场景 |
| `npm.cmd run verify --prefix scripts\\conpty\\v207` | 通过，8 个 v2.0.7 回归场景 |
| `git diff --check` | 通过 |

## 四、参考源码与 Rust 化情况

本次整改继续遵循总纲图的整体能力迁移原则：

- `D:\源码\codex`：参考 active-view 优先级、details 关闭后的焦点恢复和局部操作提示；
- `D:\源码\lazygit`：参考焦点状态、窄屏动作提示与键盘/鼠标一致性；
- `D:\源码\aider`：参考终端交互的可预测输入节奏。

实现保持为 YunXi 自有 Rust 状态机和 `crossterm + ratatui` 模块；未引入 Codex、Go 或 Python 的 UI runtime。`node-pty` 与 `@xterm/headless` 仅作为项目内真实 Windows ConPTY 证据采集依赖，不进入 YunXi 默认运行路径。

## 五、发布状态与观察项

`v2.0.7-hotfix` annotated tag 正确指向 `3893a7c`；旧 `v2.0.7`、`v2.0.6` 和全部历史 tag 均保留。tag 后仅有发布日志与开发报告的 `docs-only` 状态提交；依照用户确认的规则，该事项仅作观察项，不构成阻塞。

本次审核生成了以下未跟踪/忽略审计产物：

- `D:\YunXi Agent\scripts\conpty\v207-hotfix\node_modules`；
- `D:\YunXi Agent\scripts\conpty\v207-hotfix\.work`；
- `D:\YunXi Agent\.tmp\audit-v207-hotfix-conpty-20260721\`；
- 本报告的项目内副本。

审核过程中未删除、移动或清理这些路径；如需清理，必须先取得用户对精确绝对路径的明确确认。

## 六、审核结论

**v2.0.7-hotfix 审核通过，可进入 `v2.0.8` 开发。**

此前 `v2.0.7` 审核报告中的 Approval/Details 鼠标焦点 P1 已由本次 hotfix 修复并经源码、自动化、真实 Provider 和真实 ConPTY 视觉复审确认。当前源码、TUI 交互和发布状态不存在阻塞点。

## 七、下一版本开发建议

根据总纲图，`v2.0.8` 的主题为“视觉语义、信息密度与陪伴界面一致性”。后续开发报告应重点安排：

- 新建 `crates/yunxi-agent-tui/src/styles.rs`，以 `TuiStyleSet` 集中管理 user、assistant、progress、tool、action-required、notice、warning、error 和 muted 等语义样式；
- 在 `render.rs`、`layout.rs`、`app.rs` 和 `bottom_pane.rs` 中统一消息间距、状态优先级、header 信息收敛和 Composer 的克制焦点状态；
- 确保错误、审批、取消具有文本/符号冗余，不依赖颜色表达；低色彩终端仍可读；
- 使用 80 列与 200 列、长路径和低优先级诊断 fixture 验证普通对话不被状态栏抢占；
- 参考 Codex 的语义样式思想与 Claude Code 可观察到的“默认安静、必要动作突出”原则，以 Rust `Style` 常量/构造器实现，不引入 CSS、WebView 或装饰性渐变；
- 统一验证和真实 ConPTY 视觉复审通过后，再创建新的发布提交与 annotated `v2.0.8` tag。

署名：审核者

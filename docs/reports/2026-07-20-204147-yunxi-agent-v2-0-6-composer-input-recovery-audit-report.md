# YunXi Agent v2.0.6 审核报告

- 审核时间：2026-07-20 20:41:47 +08:00
- 审核版本：`v2.0.6`
- 发布提交：`30842cf3bae3053d36a3f9229c90eb75597d8eac`
- annotated tag 对象：`12e64e0cd31e612046c24f4a073b95c6bc887dff`
- 当前 `HEAD`：`2b25d4a0bef4f05e0c7d37b6818eec34d8cbcb5b`
- 审核目录：`D:\YunXi Agent`
- 审核依据：`C:\Users\24763\Desktop\YunXi Agent v2.0.0-v2.1.0 TUI与流式输出重构总纲图（复核版）.md` 中的 `v2.0.6 - Composer、输入恢复与对话一致性` 要求。

## 一、审核范围

本次审核覆盖：

- `EditBuffer`、Composer、UserInput、Approval 的输入状态与恢复逻辑；
- Grapheme 光标、中文/日文/Emoji/组合字符、CRLF 与多行粘贴、长粘贴；
- 流式期间草稿、Ctrl+C 取消、审批覆盖层、取消/拒绝后的草稿恢复；
- delta/final/completed 对同一 assistant cell 的更新与重复回答防止；
- TUI 在 80x24、100x30、120x40、200x50 等尺寸下的布局、光标、滚动与 footer；
- TUI、plain CLI、JSON/JSONL、离线 backend 与真实 DeepSeek live Provider；
- Windows ConPTY 的真实视觉与交互行为，以及 Git tag、版本和仓库状态。

## 二、总纲图逐项审核

### 1. 结构化输入模型

通过。`crates/yunxi-agent-tui/src/edit_buffer.rs` 使用 `cursor_grapheme`，统一提供插入、换行、前后 Grapheme 删除、左右/Home/End 移动、清空、提交、快照和恢复。换行归一化覆盖 CRLF 与单独 CR。`bottom_pane.rs` 的 Composer 与 UserInput 复用同一编辑核心，同时保留各自的提交/取消语义。

相关测试覆盖：

- `cursor_and_deletion_use_grapheme_indices`；
- `insert_handles_combining_clusters_without_exposing_byte_cursor`；
- `paste_normalizes_crlf_and_lone_carriage_returns`；
- `home_end_delete_and_snapshot_restore_round_trip`；
- `long_ime_style_committed_text_remains_lossless`；
- Composer 对 Emoji、组合字符、CJK 宽度和长 token 的测试。

### 2. active view 与输入恢复

通过。`crates/yunxi-agent-tui/src/bottom_pane.rs` 在进入 Approval/UserInput 前保存 Composer 快照，在返回 Composer 时恢复文本和光标；active turn 中允许编辑草稿，但禁止 Enter 提交和 Esc 清空，Ctrl+C 只取消当前 turn。`crates/yunxi-agent-tui/src/host.rs` 将 committed text、Paste、Enter、Esc、Ctrl+C 和 Windows 终端输入突发统一路由到当前视图。

对应测试覆盖 active-turn 草稿编辑、审批/UserInput 视图优先级、粘贴、Ctrl+C 分类和草稿不被覆盖。真实 ConPTY 的 `stream-cancel` 与 `approval-restore` 场景也均通过。

### 3. 渲染与响应式布局

通过。`crates/yunxi-agent-tui/src/render.rs` 根据 EditBuffer 和 display width 计算 Composer 光标与折行，限制 Composer 高度并保持 Transcript、Approval、Composer 和 footer 的边界。四组 full-frame snapshot 及窄终端测试通过；真实 ConPTY 的 80x24 长粘贴画面中 Composer、Transcript 和 footer 均保持可见，无越界或重叠。

### 4. 流式 assistant cell 一致性

通过。`crates/yunxi-agent-tui/src/chat.rs` 的 `apply_assistant_update` 按稳定 `TuiCellId` 原位更新 assistant cell，`push_assistant` 对同一 ID 更新现有 cell，完成时只冻结当前 cell。`timeline_store.rs` 还覆盖重复 event、错序 late event、cancel 后 late delta 和 completed 后的会话释放。

对应测试 `assistant_stream_rewrites_one_stable_history_cell`、`delta_then_final_replaces_one_canonical_cell_and_releases_session` 等均通过。真实 `final-single` 场景确认 final/completed 后只保留一个可见 assistant cell。

### 5. 陪伴边界与 CLI 兼容性

通过。TUI 没有自行添加问候、表情或人格化回复，persona、memory、主动陪伴和控制边界由原有层负责。CLI 的 plain、JSON、JSONL 和 no-TUI 回归通过，未发现 Composer 重构改变 wire shape 或工具审批边界的情况。

## 三、参考源码与 Rust 化情况

本版本明确参考并完成了整体能力迁移：

1. Codex Composer：参考路径为 `D:\源码\codex`（Composer、active view、输入缓冲和流式 transcript 更新逻辑）。本项目没有直接引入 Codex 的 UI crate 或运行时依赖，而是在 `crates/yunxi-agent-tui` 中使用 `crossterm + ratatui` 复刻职责边界和交互语义。
2. Aider：参考其简洁命令行输入节奏与回答生命周期；其非 Rust 逻辑仅作为行为参考，由 YunXi 的 `EditBuffer`、`BottomPane` 和 `host` 状态机以 Rust 实现。
3. 本版本没有将 Go/JavaScript TUI 框架接入默认运行路径；ConPTY 采集器中的 Node 依赖只用于真实 Windows 终端证据，不属于 YunXi 运行时 TUI 依赖。

## 四、验证结果

| 验证项 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | 通过 |
| `cargo check --workspace` | 通过 |
| `cargo test --workspace` | 通过，无失败项 |
| `cargo test -p yunxi-agent-tui` | 126/126 通过 |
| `cargo test -p yunxi-agent-cli` | 44/44 通过，包含 CLI 集成与 JSONL 回归 |
| `cargo build -p yunxi-agent-cli --release --bins` | 通过 |
| `target\\release\\yunxi.exe --version` | 返回 `yunxi 2.0.6` |
| `target\\release\\yunxi.exe eval companion --json` | 31/31 通过，`golden_passed=true`，`tool_approval_bypass_count=0` |
| `npm.cmd run verify --prefix scripts\\conpty\\v206` | 通过，8 个场景 |
| `npm.cmd run verify --prefix scripts\\conpty\\v205` | 通过，8 个历史场景 |
| `git diff --check` | 通过 |

### 真实 Provider / ConPTY 视觉复核

已在项目内重新运行真实 DeepSeek live、Windows ConPTY 八场景：`ordinary`、`multiline`、`crlf`、`long`、`ime`、`stream-cancel`、`approval-restore`、`final-single`。隔离复测目录为：

`D:\YunXi Agent\.tmp\audit-v206-conpty-20260720\frames`

八个场景均正常退出、所有检查点匹配，原生 `node-pty` 与 `@xterm/headless` 均可加载。人工视觉复核结果：

- 80x24 长粘贴保持 Composer、Transcript 与 footer 可见；
- 中文、日文、Emoji ZWJ 和组合字符未破坏输入行或光标边界；
- 流式取消后草稿准确恢复，错误状态清晰且可继续下一轮；
- 审批默认拒绝，拒绝后草稿准确恢复，审批内容没有吞掉输入；
- final/completed 后仅显示一个 assistant cell；
- 未观察到控件重叠、越界、重复回答或终端无法退出。

隔离帧的辅助 PNG 在读取 UTF-8 边框字符时出现过工具侧替代字符，这是审计辅助渲染的编码表现；原始 ConPTY JSON 帧、场景检查点和 verifier 均正常，不构成运行时缺陷。

## 五、发布状态观察项

`v2.0.6` annotated tag 正确指向 `30842cf`，历史 `v2.0.5`、`v2.0.5-hotfix.1` 和 `v2.0.5-hotfix.2` tag 均存在且未移动。tag 后的 `HEAD` 为 `2b25d4a`，新增差异仅为：

- `docs/development-log.md`；
- `docs/reports/2026-07-20-143150-yunxi-agent-v2-0-6-composer-input-recovery-dialog-consistency-development-report.md`。

这两项均为仓库文档记录。按照用户已确认的审核规则，tag 后追加 `docs-only` 状态提交只作观察项，不构成审核阻塞，不否定当前版本源码和功能验收。

## 六、审核结论

**v2.0.6 审核通过，可进入 v2.0.7 开发。**

本版本在总纲图要求的 Composer、输入恢复、Grapheme 文本处理、流式 assistant cell 一致性、真实 Provider、TUI 视觉交互和 CLI 兼容性方面均已完成并通过验证。当前源码上没有阻塞点。

## 七、下一版本开发建议

根据总纲图，v2.0.7 的主题为“交互焦点、详情层与快捷键一致性”。这不是 v2.0.6 的补做项，开发报告应重点安排：

- 在 `crates/yunxi-agent-tui/src/host.rs`、`app.rs`、`bottom_pane.rs`、`debug.rs` 中建立明确的 `FocusTarget` 与 `TuiAction`，统一 Composer、History、Approval、Details 四种焦点；
- 在 `crates/yunxi-agent-tui/src/input_map.rs` 建立纯 Rust keymap resolver，保证 Esc、Enter、PgUp/PgDown、滚轮和 Ctrl+C 在不同 active view 下不冲突；
- details 打开/关闭时保留 Composer 文本、光标、审批选择和 transcript anchor；普通主视图不混入内部 debug/detail 内容；
- `/help` 和 footer 只显示当前焦点可用的核心动作，窄屏优先保留提交、取消、返回和滚动提示；
- 参考 Codex bottom pane active-view 优先级与 Lazygit 的焦点/快捷键提示，逻辑以 Rust 状态机复刻，不引入 Go UI runtime；
- 新增四种焦点和窄屏/鼠标/resize 的单元、snapshot、ConPTY 真实交互回归，再统一执行 workspace 验证。

署名：审核者
